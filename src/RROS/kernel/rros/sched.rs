use alloc::rc::Rc;
use kernel::{interrupt, irq_pipeline::irq_get_reschedule_oob_ipi};

use core::{
    cell::RefCell,
    cmp::Ordering::{Equal, Greater, Less},
    mem::{align_of, size_of, transmute},
    ops::DerefMut,
    ptr::{null, null_mut, NonNull},
};

#[warn(unused_mut)]
use kernel::{
    bindings, c_str, c_types, capability, completion,
    cpumask::{num_possible_cpus, online_cpus, possible_cpus, CpumaskT},
    double_linked_list::*,
    dovetail::{self, OobMmState},
    irq_pipeline,
    irq_work::IrqWork,
    ktime::Timespec64,
    linked_list::{GetLinks, Links},
    percpu::alloc_per_cpu,
    percpu_defs,
    prelude::*,
    premmpt, spinlock_init,
    str::{kstrdup, CStr},
    sync::{HardSpinlock, Lock, SpinLock},
    types::Atomic,
};

use crate::{
    clock::{self},
    factory::RrosElement,
    fifo, idle,
    list::ListHead,
    lock,
    observable::RrosObservable,
    poll::RrosPollWatchpoint,
    sched, stat,
    thread::*,
    tick,
    timeout::RROS_INFINITE,
    timer::*,
    tp,
    wait::RrosWaitChannel,
    RROS_MACHINE_CPUDATA, RROS_OOB_CPUS,
};

extern "C" {
    #[allow(dead_code)]
    fn rust_helper_list_add_tail(new: *mut ListHead, head: *mut ListHead);
    fn rust_helper_test_bit(nr: i32, addr: *const usize) -> bool;
}

pub const RQ_SCHED: u64 = 0x10000000;
pub const RQ_TIMER: u64 = 0x00010000;
pub const RQ_TPROXY: u64 = 0x00008000;
pub const RQ_IRQ: u64 = 0x00004000;
pub const RQ_TDEFER: u64 = 0x00002000;
pub const RQ_IDLE: u64 = 0x00001000;
pub const RQ_TSTOPPED: u64 = 0x00000800;

// pub const SCHED_WEAK: i32 = 43;
pub const SCHED_IDLE: i32 = 5;
pub const SCHED_FIFO: i32 = 1;
#[allow(dead_code)]
pub const SCHED_RR: i32 = 2;
pub const SCHED_TP: i32 = 45;
pub const RROS_CLASS_WEIGHT_FACTOR: i32 = 1024;
pub const RROS_MM_PTSYNC_BIT: i32 = 0;

static mut RROS_SCHED_TOPMOS: *mut RrosSchedClass = 0 as *mut RrosSchedClass;
static mut RROS_SCHED_LOWER: *mut RrosSchedClass = 0 as *mut RrosSchedClass;

// static mut rros_thread_list: List<Arc<SpinLock<RrosThread>>> = ;

// pub static mut RROS_SCHED_TOPMOS:*mut RrosSchedClass = 0 as *mut RrosSchedClass;
// pub static mut RROS_SCHED_LOWER:*mut RrosSchedClass = 0 as *mut RrosSchedClass;

//#[derive(Copy,Clone)]
#[repr(C)]
pub struct rros_rq {
    pub flags: u64,
    pub curr: Option<Arc<SpinLock<RrosThread>>>,
    pub fifo: RrosSchedFifo,
    pub weak: RrosSchedWeak,
    pub tp: tp::RrosSchedTp,
    pub root_thread: Option<Arc<SpinLock<RrosThread>>>,
    pub local_flags: u64,
    pub inband_timer: Option<Arc<SpinLock<rros_timer>>>,
    pub rrbtimer: Option<Arc<SpinLock<rros_timer>>>,
    pub proxy_timer_name: *mut c_types::c_char,
    pub rrb_timer_name: *mut c_types::c_char,
    #[cfg(CONFIG_SMP)]
    pub cpu: i32,
    #[cfg(CONFIG_SMP)]
    pub resched_cpus: CpumaskT,
    #[cfg(CONFIG_RROS_RUNSTATS)]
    pub last_account_switch: KtimeT,
    #[cfg(CONFIG_RROS_RUNSTATS)]
    pub current_account: *mut stat::RrosAccount,
    pub lock: HardSpinlock,
}

impl rros_rq {
    pub fn new() -> Result<Self> {
        Ok(rros_rq {
            flags: 0,
            curr: None,
            fifo: RrosSchedFifo::new()?,
            weak: RrosSchedWeak::new(),
            tp: tp::RrosSchedTp::new()?,
            root_thread: None,
            // root_thread: unsafe{Some(Arc::try_new(SpinLock::new(RrosThread::new()?))?)},
            local_flags: 0,
            inband_timer: None,
            rrbtimer: None,
            proxy_timer_name: null_mut(),
            rrb_timer_name: null_mut(),
            #[cfg(CONFIG_SMP)]
            cpu: 0,
            #[cfg(CONFIG_SMP)]
            resched_cpus: CpumaskT::from_int(0 as u64),
            #[cfg(CONFIG_RROS_RUNSTATS)]
            last_account_switch: 0,
            #[cfg(CONFIG_RROS_RUNSTATS)]
            current_account: stat::RrosAccount::new() as *mut stat::RrosAccount,
            lock: HardSpinlock::new(),
        })
    }

    pub fn get_inband_timer(&self) -> Arc<SpinLock<rros_timer>> {
        self.inband_timer.as_ref().unwrap().clone()
    }

    pub fn get_rrbtimer(&self) -> Arc<SpinLock<rros_timer>> {
        self.rrbtimer.as_ref().unwrap().clone()
    }

    pub fn get_curr(&self) -> Arc<SpinLock<RrosThread>> {
        self.curr.as_ref().unwrap().clone()
    }

    pub fn add_local_flags(&mut self, local_flag: u64) {
        self.local_flags |= local_flag;
    }

    pub fn change_local_flags(&mut self, local_flag: u64) {
        self.local_flags &= local_flag;
    }

    #[allow(dead_code)]
    pub fn get_local_flags(&self) -> u64 {
        self.local_flags
    }

    pub fn add_flags(&mut self, flags: u64) {
        self.flags |= flags;
    }

    pub fn get_cpu(&self) -> i32 {
        self.cpu
    }
}

#[no_mangle]
pub static helloworldint: i32 = 5433;

static mut RROS_RUNQUEUES: *mut rros_rq = 0 as *mut rros_rq;

pub static mut RROS_CPU_AFFINITY: CpumaskT = CpumaskT::cpu_mask_all();

pub fn rros_cpu_rq(cpu: i32) -> *mut rros_rq {
    unsafe { percpu_defs::per_cpu(RROS_RUNQUEUES, cpu) }
}

pub fn this_rros_rq() -> *mut rros_rq {
    unsafe {
        percpu_defs::per_cpu_ptr(RROS_RUNQUEUES as *mut u8, percpu_defs::smp_processor_id())
            as *mut rros_rq
    }
}

pub fn this_rros_rq_thread() -> Option<Arc<SpinLock<RrosThread>>> {
    let rq = this_rros_rq();
    unsafe { (*rq).curr.clone() }
}

pub fn rros_need_resched(rq: *mut rros_rq) -> bool {
    unsafe { (*rq).flags & RQ_SCHED != 0x0 }
}

pub fn rros_set_self_resched(rq: Option<*mut rros_rq>) -> Result<usize> {
    match rq {
        Some(r) => unsafe {
            (*r).flags |= RQ_SCHED;
            // (*r).local_flags |= RQ_SCHED;
        },
        None => return Err(kernel::Error::EINVAL),
    }
    Ok(0)
}

#[cfg(CONFIG_SMP)]
pub fn rros_rq_cpu(rq: *mut rros_rq) -> i32 {
    unsafe { (*rq).get_cpu() }
}

#[cfg(not(CONFIG_SMP))]
pub fn rros_rq_cpu(rq: *mut rros_rq) -> i32 {
    return 0;
}

#[allow(dead_code)]
pub fn rros_protect_thread_priority(thread: Arc<SpinLock<RrosThread>>, prio: i32) -> Result<usize> {
    unsafe {
        // raw_spin_lock(&thread->rq->lock);
        let mut state = (*thread.locked_data().get()).state;
        if state & T_READY != 0 {
            rros_dequeue_thread(thread.clone())?;
        }

        (*thread.locked_data().get()).sched_class = Some(&fifo::RROS_SCHED_FIFO);
        rros_ceil_priority(thread.clone(), prio)?;

        state = (*thread.locked_data().get()).state;
        if state & T_READY != 0 {
            rros_enqueue_thread(thread.clone())?;
        }

        let rq = (*thread.locked_data().get()).rq;
        rros_set_resched(rq.clone());

        // raw_spin_unlock(&thread->rq->lock);
        Ok(0)
    }
}

#[cfg(CONFIG_SMP)]
pub fn rros_set_resched(rq_op: Option<*mut rros_rq>) {
    let rq;
    match rq_op {
        None => return,
        Some(x) => rq = x,
    };
    let this_rq = this_rros_rq();
    if this_rq == rq {
        unsafe {
            (*this_rq).add_flags(RQ_SCHED);
        }
    } else if rros_need_resched(rq) == false {
        unsafe {
            (*rq).add_flags(RQ_SCHED);
            (*this_rq).add_local_flags(RQ_SCHED);
            (*this_rq)
                .resched_cpus
                .cpumask_set_cpu(rros_rq_cpu(rq) as u32);
        }
    }
}

#[cfg(not(CONFIG_SMP))]
pub fn rros_set_resched(rq: Option<*mut rros_rq>) {
    rros_set_self_resched(rq_clone)
}

#[cfg(CONFIG_SMP)]
pub fn is_threading_cpu(cpu: i32) -> bool {
    unsafe { RROS_CPU_AFFINITY.cpumask_test_cpu(cpu as u32) }
}

#[cfg(not(CONFIG_SMP))]
pub fn is_threading_cpu(cpu: i32) -> bool {
    return true;
}

#[cfg(CONFIG_SMP)]
pub fn is_rros_cpu(cpu: i32) -> bool {
    unsafe { RROS_OOB_CPUS.cpumask_test_cpu(cpu as u32) }
}

#[cfg(not(CONFIG_SMP))]
pub fn is_rros_cpu(cpu: i32) -> bool {
    return true;
}

#[cfg(CONFIG_SMP)]
pub fn rros_double_rq_lock(rq1: *mut rros_rq, rq2: *mut rros_rq) {
    match rq1.cmp(&rq2) {
        Equal => unsafe { (*rq1).lock.raw_spin_lock() },
        Less => unsafe {
            (*rq1).lock.raw_spin_lock();
            (*rq2)
                .lock
                .raw_spin_lock_nested(bindings::SINGLE_DEPTH_NESTING);
        },
        Greater => unsafe {
            (*rq2).lock.raw_spin_lock();
            (*rq1)
                .lock
                .raw_spin_lock_nested(bindings::SINGLE_DEPTH_NESTING);
        },
    }
}

#[cfg(CONFIG_SMP)]
pub fn rros_double_rq_unlock(rq1: *mut rros_rq, rq2: *mut rros_rq) {
    unsafe {
        (*rq1).lock.raw_spin_unlock();
        if rq1 != rq2 {
            (*rq2).lock.raw_spin_unlock();
        }
    }
}

#[cfg(not(CONFIG_SMP))]
pub fn rros_double_rq_lock(_rq1: *mut rros_rq, _rq2: *mut rros_rq) {}

#[cfg(not(CONFIG_SMP))]
pub fn rros_double_rq_unlock(_rq1: *mut rros_rq, _rq2: *mut rros_rq) {}

#[cfg(CONFIG_SMP)]
pub fn migrate_thread(thread: Arc<SpinLock<RrosThread>>, dst_rq: *mut rros_rq) {
    let src_rq = unsafe { (*thread.locked_data().get()).rq.unwrap() };
    rros_double_rq_lock(src_rq, dst_rq);

    let thread_state = unsafe { (*thread.locked_data().get()).state };
    if thread_state & T_READY != 0 {
        let _ = rros_dequeue_thread(thread.clone());
        unsafe { (*thread.locked_data().get()).state &= !T_READY };
    }

    if let Some(ref sched_class) = unsafe { (*thread.locked_data().get()).sched_class } {
        if let Some(ref sched_migrate) = sched_class.sched_migrate {
            let _ = sched_migrate(thread.clone(), dst_rq);
        }
    }

    unsafe {
        (*thread.locked_data().get()).rq = Some(dst_rq);
    }
    if unsafe { (*thread.locked_data().get()).state & RROS_THREAD_BLOCK_BITS } == 0 {
        let _ = rros_requeue_thread(thread.clone());
        unsafe {
            (*thread.locked_data().get()).state |= T_READY;
        }
        rros_set_resched(Some(dst_rq));
        rros_set_resched(Some(src_rq));
    }

    rros_double_rq_unlock(src_rq, dst_rq);
}

#[cfg(CONFIG_SMP)]
#[allow(dead_code)]
pub fn rros_migrate_thread(thread: Arc<SpinLock<RrosThread>>, dst_rq: *mut rros_rq) {
    // TODO: assert_hard_lock(&thread.lock);
    let src_rq = unsafe { (*thread.locked_data().get()).rq.unwrap() };
    if src_rq == dst_rq {
        return;
    }

    // TODO: trace_rros_thread_migrate(thread, rros_rq_cpu(dst_rq));
    migrate_thread(thread.clone(), dst_rq);
    unsafe {
        (*thread.locked_data().get())
            .stat
            .lastperiod
            .reset_account()
    };
}

#[cfg(not(CONFIG_SMP))]
pub fn rros_migrate_thread(thread: Arc<SpinLock<RrosThread>>, dst_rq: *mut rros_rq) {}

#[allow(dead_code)]
pub fn rros_in_irq() -> bool {
    let rq = this_rros_rq();
    unsafe { (*rq).get_local_flags() & RQ_IRQ != 0 }
}

pub fn rros_is_inband() -> bool {
    let thread_op = this_rros_rq_thread();
    let state;
    match thread_op {
        None => return false,
        Some(x) => state = x.lock().state,
    }
    state & T_ROOT != 0x0
}

#[allow(dead_code)]
pub fn rros_cannot_block() -> bool {
    rros_in_irq() || rros_is_inband()
}

#[no_mangle]
unsafe extern "C" fn this_rros_rq_enter_irq_local_flags() {
    unsafe {
        if RROS_RUNQUEUES == 0 as *mut rros_rq {
            return;
        }
    }

    let rq = this_rros_rq();

    unsafe {
        (*rq).local_flags |= RQ_IRQ;
    }
}

#[no_mangle]
unsafe extern "C" fn this_rros_rq_exit_irq_local_flags() -> c_types::c_int {
    unsafe {
        if RROS_RUNQUEUES == 0 as *mut rros_rq {
            return 0;
        }
    }

    let rq = this_rros_rq();
    // struct rros_rq *rq = this_rros_rq();

    unsafe {
        (*rq).local_flags &= !RQ_IRQ;
    }

    let flags;
    let local_flags;

    unsafe {
        flags = (*rq).flags;
        local_flags = (*rq).local_flags;
    }

    // pr_debug!("{} cc {} \n", flags, local_flags);

    if ((flags | local_flags) & RQ_SCHED) != 0x0 {
        return 1 as c_types::c_int;
    }

    0 as c_types::c_int
}

pub struct RrosSchedFifo {
    pub runnable: RrosSchedQueue,
}
impl RrosSchedFifo {
    fn new() -> Result<Self> {
        Ok(RrosSchedFifo {
            runnable: RrosSchedQueue::new()?,
        })
    }
}

pub struct RrosSchedWeak {
    pub runnable: Option<Rc<RefCell<RrosSchedQueue>>>,
}
impl RrosSchedWeak {
    fn new() -> Self {
        RrosSchedWeak { runnable: None }
    }
}

pub struct RrosSchedQueue {
    pub head: Option<List<Arc<SpinLock<RrosThread>>>>,
}
impl RrosSchedQueue {
    pub fn new() -> Result<Self> {
        Ok(RrosSchedQueue {
            head: None,
            // head: unsafe{List::new(Arc::try_new(SpinLock::new(rros_rq::new()?))?)},
        })
    }
}

pub type SsizeT = bindings::__kernel_ssize_t;

pub struct RrosSchedClass {
    pub sched_init: Option<fn(rq: *mut rros_rq) -> Result<usize>>,
    pub sched_enqueue: Option<fn(thread: Arc<SpinLock<RrosThread>>) -> Result<i32>>,
    pub sched_dequeue: Option<fn(thread: Arc<SpinLock<RrosThread>>)>,
    pub sched_requeue: Option<fn(thread: Arc<SpinLock<RrosThread>>)>,
    pub sched_pick: Option<fn(rq: Option<*mut rros_rq>) -> Result<Arc<SpinLock<RrosThread>>>>,
    pub sched_tick: Option<fn(rq: Option<*mut rros_rq>) -> Result<usize>>,
    pub sched_migrate:
        Option<fn(thread: Arc<SpinLock<RrosThread>>, rq: *mut rros_rq) -> Result<usize>>,
    pub sched_setparam: Option<
        fn(
            thread: Option<Arc<SpinLock<RrosThread>>>,
            p: Option<Arc<SpinLock<RrosSchedParam>>>,
        ) -> Result<usize>,
    >,
    pub sched_getparam: Option<
        fn(thread: Option<Arc<SpinLock<RrosThread>>>, p: Option<Arc<SpinLock<RrosSchedParam>>>),
    >,
    pub sched_chkparam: Option<
        fn(
            thread: Option<Arc<SpinLock<RrosThread>>>,
            p: Option<Arc<SpinLock<RrosSchedParam>>>,
        ) -> Result<i32>,
    >,
    pub sched_trackprio: Option<
        fn(thread: Option<Arc<SpinLock<RrosThread>>>, p: Option<Arc<SpinLock<RrosSchedParam>>>),
    >,
    pub sched_ceilprio: Option<fn(thread: Arc<SpinLock<RrosThread>>, prio: i32)>,

    pub sched_declare: Option<
        fn(
            thread: Option<Arc<SpinLock<RrosThread>>>,
            p: Option<Arc<SpinLock<RrosSchedParam>>>,
        ) -> Result<i32>,
    >,
    pub sched_forget: Option<fn(thread: Arc<SpinLock<RrosThread>>) -> Result<usize>>,
    pub sched_kick: Option<fn(thread: Arc<SpinLock<RrosThread>>)>,
    pub sched_show: Option<
        fn(thread: *mut RrosThread, buf: *mut c_types::c_char, count: SsizeT) -> Result<usize>,
    >,
    pub sched_control: Option<
        fn(cpu: i32, ctlp: *mut RrosSchedCtlparam, infp: *mut RrosSchedCtlinfo) -> Result<SsizeT>,
    >,
    pub nthreads: i32,
    pub next: *mut RrosSchedClass,
    pub weight: i32,
    pub policy: i32,
    pub name: &'static str,
    pub flag: i32, // Identify the scheduling class: 1:RROS_SCHED_IDLE 3:RrosSchedFifo 4:RROS_SCHED_TP
}
impl RrosSchedClass {
    #[allow(dead_code)]
    pub fn new() -> Self {
        RrosSchedClass {
            sched_init: None,
            sched_enqueue: None,
            sched_dequeue: None,
            sched_requeue: None,
            sched_pick: None,
            sched_tick: None,
            sched_migrate: None,
            sched_setparam: None,
            sched_getparam: None,
            sched_chkparam: None,
            sched_trackprio: None,
            sched_ceilprio: None,
            sched_declare: None,
            sched_forget: None,
            sched_kick: None,
            sched_show: None,
            sched_control: None,
            nthreads: 0,
            next: 0 as *mut RrosSchedClass,
            weight: 0,
            policy: 0,
            name: "sched_class",
            flag: 0,
        }
    }
}

#[derive(Copy, Clone)]
pub struct RrosSchedParam {
    pub idle: RrosIdleParam,
    pub fifo: RrosFifoParam,
    pub weak: RrosWeakParam,
    pub tp: RrosTpParam,
}
impl RrosSchedParam {
    pub fn new() -> Self {
        RrosSchedParam {
            idle: RrosIdleParam::new(),
            fifo: RrosFifoParam::new(),
            weak: RrosWeakParam::new(),
            tp: RrosTpParam::new(),
        }
    }
}

#[derive(Copy, Clone)]
pub struct RrosIdleParam {
    pub prio: i32,
}
impl RrosIdleParam {
    fn new() -> Self {
        RrosIdleParam { prio: 0 }
    }
}

#[derive(Copy, Clone)]
pub struct RrosFifoParam {
    pub prio: i32,
}
impl RrosFifoParam {
    fn new() -> Self {
        RrosFifoParam { prio: 0 }
    }
}

#[derive(Copy, Clone)]
pub struct RrosTpParam {
    pub prio: i32,
    pub ptid: i32, /* partition id. */
}
impl RrosTpParam {
    fn new() -> Self {
        RrosTpParam { prio: 0, ptid: 0 }
    }
}

#[derive(Copy, Clone)]
pub struct RrosWeakParam {
    pub prio: i32,
}
impl RrosWeakParam {
    fn new() -> Self {
        RrosWeakParam { prio: 0 }
    }
}

pub struct RrosSchedCtlparam {
    pub quota: RrosQuotaCtlparam,
    pub tp: RrosTpCtlparam,
}
impl RrosSchedCtlparam {
    #[allow(dead_code)]
    fn new() -> Self {
        RrosSchedCtlparam {
            quota: RrosQuotaCtlparam::new(),
            tp: RrosTpCtlparam::new(),
        }
    }
}

pub struct RrosSchedCtlinfo {
    pub quota: RrosQuotaCtlinfo,
    pub tp: RrosTpCtlinfo,
}
impl RrosSchedCtlinfo {
    #[allow(dead_code)]
    fn new() -> Self {
        RrosSchedCtlinfo {
            quota: RrosQuotaCtlinfo::new(),
            tp: RrosTpCtlinfo::new(),
        }
    }
}

pub struct RrosQuotaCtlparam {
    pub op: RrosQuotaCtlop,
    pub u: U,
}
impl RrosQuotaCtlparam {
    #[allow(dead_code)]
    fn new() -> Self {
        RrosQuotaCtlparam { op: 0, u: U::new() }
    }
}

pub struct RrosTpCtlparam {
    pub op: RrosTpCtlop,
    pub nr_windows: i32,
    pub windows: *mut RrosSchedTpWindow,
}
impl RrosTpCtlparam {
    #[allow(dead_code)]
    fn new() -> Self {
        RrosTpCtlparam {
            op: 0,
            nr_windows: 0,
            windows: 0 as *mut RrosSchedTpWindow,
        }
    }
}

pub struct RrosQuotaCtlinfo {
    pub tgid: i32,
    pub quota: i32,
    pub quota_peak: i32,
    pub quota_sum: i32,
}
impl RrosQuotaCtlinfo {
    #[allow(dead_code)]
    fn new() -> Self {
        RrosQuotaCtlinfo {
            tgid: 0,
            quota: 0,
            quota_peak: 0,
            quota_sum: 0,
        }
    }
}

pub struct RrosTpCtlinfo {
    pub nr_windows: i32,
    pub windows: *mut RrosSchedTpWindow,
}
impl RrosTpCtlinfo {
    #[allow(dead_code)]
    fn new() -> Self {
        RrosTpCtlinfo {
            nr_windows: 0,
            windows: 0 as *mut RrosSchedTpWindow,
        }
    }
}

// pub const RROS_QUOTA_CTLOP_RROS_QUOTA_ADD: RrosQuotaCtlop = 0;
// pub const RROS_QUOTA_CTLOP_RROS_QUOTA_REMOVE: RrosQuotaCtlop = 1;
// pub const RROS_QUOTA_CTLOP_RROS_QUOTA_FORCE_REMOVE: RrosQuotaCtlop = 2;
// pub const RROS_QUOTA_CTLOP_RROS_QUOTA_SET: RrosQuotaCtlop = 3;
// pub const RROS_QUOTA_CTLOP_RROS_QUOTA_GET: RrosQuotaCtlop = 4;
pub type RrosQuotaCtlop = c_types::c_uint;
// pub const RROS_TP_CTLOP_RROS_TP_INSTALL: RrosTpCtlop = 0;
// pub const RROS_TP_CTLOP_RROS_TP_UNINSTALL: RrosTpCtlop = 1;
// pub const RROS_TP_CTLOP_RROS_TP_START: RrosTpCtlop = 2;
// pub const RROS_TP_CTLOP_RROS_TP_STOP: RrosTpCtlop = 3;
// pub const RROS_TP_CTLOP_RROS_TP_GET: RrosTpCtlop = 4;
pub type RrosTpCtlop = c_types::c_uint;

pub struct U {
    pub remove: Remove,
    pub set: Set,
    pub get: Get,
}

impl U {
    fn new() -> Self {
        U {
            remove: Remove::new(),
            set: Set::new(),
            get: Get::new(),
        }
    }
}
pub struct Remove {
    #[allow(dead_code)]
    tgid: i32,
}
impl Remove {
    fn new() -> Self {
        Remove { tgid: 0 }
    }
}
pub struct Set {
    #[allow(dead_code)]
    tgid: i32,
    #[allow(dead_code)]
    quota: i32,
    #[allow(dead_code)]
    quota_peak: i32,
}
impl Set {
    fn new() -> Self {
        Set {
            tgid: 0,
            quota: 0,
            quota_peak: 0,
        }
    }
}
pub struct Get {
    #[allow(dead_code)]
    tgid: i32,
}
impl Get {
    fn new() -> Self {
        Get { tgid: 0 }
    }
}

pub struct RrosSchedTpWindow {
    pub offset: *mut Timespec64,
    pub duration: *mut Timespec64,
    pub ptid: i32,
}
impl RrosSchedTpWindow {
    #[allow(dead_code)]
    fn new() -> Self {
        RrosSchedTpWindow {
            offset: 0 as *mut Timespec64,
            duration: 0 as *mut Timespec64,
            ptid: 0,
        }
    }
}
use crate::timer::RrosTimer as rros_timer;
type KtimeT = i64;
use crate::clock::RrosClock as rros_clock;

#[allow(dead_code)]
pub struct RrosTqueue {
    pub q: ListHead,
}
impl RrosTqueue {
    #[allow(dead_code)]
    fn new() -> Self {
        RrosTqueue {
            q: ListHead {
                next: 0 as *mut ListHead,
                prev: 0 as *mut ListHead,
            },
        }
    }
}

#[allow(dead_code)]
pub struct Ops {
    pub read: Option<fn(clock: Rc<RefCell<rros_clock>>) -> KtimeT>,
    pub read_cycles: Option<fn(clock: Rc<RefCell<rros_clock>>) -> u64>,
    pub set: Option<fn(clock: Rc<RefCell<rros_clock>>, date: KtimeT) -> i32>,
    pub program_local_shot: Option<fn(clock: Rc<RefCell<rros_clock>>)>,
    pub program_remote_shot: Option<fn(clock: Rc<RefCell<rros_clock>>, rq: Rc<RefCell<rros_rq>>)>,
    pub set_gravity: Option<fn(clock: Rc<RefCell<rros_clock>>, p: *const RrosClockGravity) -> i32>,
    pub reset_gravity: Option<fn(clock: Rc<RefCell<rros_clock>>)>,
    pub adjust: Option<fn(clock: Rc<RefCell<rros_clock>>)>,
}
impl Ops {
    #[allow(dead_code)]
    fn new() -> Self {
        Ops {
            read: None,
            read_cycles: None,
            set: None,
            program_local_shot: None,
            program_remote_shot: None,
            set_gravity: None,
            reset_gravity: None,
            adjust: None,
        }
    }
}

#[allow(dead_code)]
pub struct RrosClockGravity {
    pub irq: KtimeT,
    pub kernel: KtimeT,
    pub user: KtimeT,
}
impl RrosClockGravity {
    #[allow(dead_code)]
    fn new() -> Self {
        RrosClockGravity {
            irq: 0,
            kernel: 0,
            user: 0,
        }
    }
}

pub struct RrosStat {
    pub isw: stat::RrosCounter,
    pub csw: stat::RrosCounter,
    pub sc: stat::RrosCounter,
    pub rwa: stat::RrosCounter,
    pub account: stat::RrosAccount,
    pub lastperiod: stat::RrosAccount,
}

impl RrosStat {
    pub fn new() -> Self {
        RrosStat {
            isw: stat::RrosCounter::new(),
            csw: stat::RrosCounter::new(),
            sc: stat::RrosCounter::new(),
            rwa: stat::RrosCounter::new(),
            account: stat::RrosAccount::new(),
            lastperiod: stat::RrosAccount::new(),
        }
    }
}

pub struct RrosThreadWithLock(SpinLock<RrosThread>);
impl RrosThreadWithLock {
    /// transmute back
    pub unsafe fn transmute_to_original(ptr: Arc<Self>) -> Arc<SpinLock<RrosThread>> {
        unsafe {
            let ptr = Arc::into_raw(ptr) as *mut SpinLock<RrosThread>;
            Arc::from_raw(transmute(NonNull::new_unchecked(ptr).as_ptr()))
        }
    }

    pub unsafe fn new_from_curr_thread() -> Arc<Self> {
        unsafe {
            let ptr = transmute(NonNull::new_unchecked(rros_current()).as_ptr());
            let ret = Arc::from_raw(ptr);
            Arc::increment_strong_count(ptr);
            ret
        }
    }
    pub fn get_wprio(&self) -> i32 {
        unsafe { (*(*self.0.locked_data()).get()).wprio }
    }
}

impl GetLinks for RrosThreadWithLock {
    type EntryType = RrosThreadWithLock;
    fn get_links(data: &Self::EntryType) -> &Links<Self::EntryType> {
        unsafe { &(*data.0.locked_data().get()).wait_next }
    }
}

pub struct RrosThread {
    pub lock: HardSpinlock,

    pub rq: Option<*mut rros_rq>,
    pub base_class: Option<&'static RrosSchedClass>,
    pub sched_class: Option<&'static RrosSchedClass>,

    pub bprio: i32,
    pub cprio: i32,
    pub wprio: i32,

    // pub boosters: *mut List<Arc<SpinLock<RrosMutex>>>,
    pub wchan: *mut RrosWaitChannel,
    pub wait_next: Links<RrosThreadWithLock>,
    pub wwake: *mut RrosWaitChannel,
    pub rtimer: Option<Arc<SpinLock<rros_timer>>>,
    pub ptimer: Option<Arc<SpinLock<rros_timer>>>,
    pub rrperiod: KtimeT,
    pub state: u32,
    pub info: u32,

    // pub rq_next: Option<List<Arc<SpinLock<RrosThread>>>>,
    pub next: *mut Node<Arc<SpinLock<RrosThread>>>,

    pub rq_next: Option<NonNull<Node<Arc<SpinLock<RrosThread>>>>>,

    pub altsched: dovetail::DovetailAltschedContext,
    pub local_info: u32,
    pub wait_data: *mut c_types::c_void,
    pub poll_context: PollContext,

    pub inband_disable_count: Atomic,
    pub inband_work: IrqWork,
    pub stat: RrosStat,
    pub u_window: Option<Rc<RefCell<RrosUserWindow>>>,

    // pub trackers: *mut List<Arc<SpinLock<RrosMutex>>>,
    pub tracking_lock: HardSpinlock,
    pub element: Rc<RefCell<RrosElement>>,
    pub affinity: CpumaskT,
    pub exited: completion::Completion,
    pub raised_cap: capability::KernelCapStruct,
    pub kill_next: ListHead,
    pub oob_mm: OobMmState,
    pub ptsync_next: ListHead,
    pub observable: Option<Rc<RefCell<RrosObservable>>>,
    pub name: &'static str,
    pub tps: *mut tp::RrosTpRq,
    pub tp_link: Option<Node<Arc<SpinLock<RrosThread>>>>,
}

impl RrosThread {
    pub fn new() -> Result<Self> {
        Ok(RrosThread {
            lock: HardSpinlock::new(),
            rq: None,
            base_class: None,
            sched_class: None,
            bprio: 0,
            cprio: 0,
            wprio: 0,
            // boosters: 0 as *mut List<Arc<SpinLock<RrosMutex>>>,
            wchan: core::ptr::null_mut(),
            wait_next: Links::new(),
            wwake: core::ptr::null_mut(),
            rtimer: None,
            ptimer: None,
            rrperiod: 0,
            state: 0,
            info: 0,
            // rq_next: unsafe{List::new(Arc::try_new(SpinLock::new(RrosThread::new()?))?)},
            rq_next: None,
            // next: list_head {
            //     next: 0 as *mut ListHead,
            //     prev: 0 as *mut ListHead,
            // },
            next: 0 as *mut Node<Arc<SpinLock<RrosThread>>>, // kernel corrupted bug
            altsched: dovetail::DovetailAltschedContext::new(),
            local_info: 0,
            wait_data: null_mut(),
            poll_context: PollContext::new(),
            inband_disable_count: Atomic::new(),
            inband_work: IrqWork::new(),
            stat: RrosStat::new(),
            u_window: None,
            // trackers: 0 as *mut List<Arc<SpinLock<RrosMutex>>>,
            tracking_lock: HardSpinlock::new(),
            element: Rc::try_new(RefCell::new(RrosElement::new()?))?,
            affinity: CpumaskT::from_int(0 as u64),
            exited: completion::Completion::new(),
            raised_cap: capability::KernelCapStruct::new(),
            kill_next: ListHead {
                next: 0 as *mut ListHead,
                prev: 0 as *mut ListHead,
            },
            ptsync_next: ListHead {
                next: 0 as *mut ListHead,
                prev: 0 as *mut ListHead,
            },
            observable: None,
            name: "thread\0",
            oob_mm: OobMmState::new(),
            tps: 0 as *mut tp::RrosTpRq,
            tp_link: None,
        })
    }

    pub fn init(&mut self) -> Result<usize> {
        self.lock.init();
        self.rq = None;
        self.base_class = None;
        self.sched_class = None;
        self.bprio = 0;
        self.cprio = 0;
        self.wprio = 0;
        self.wchan = core::ptr::null_mut();
        self.wait_next = Links::new();
        self.wwake = core::ptr::null_mut();
        self.rtimer = None;
        self.ptimer = None;
        self.rrperiod = 0;
        self.state = 0;
        self.info = 0;
        self.rq_next = None;
        self.next = 0 as *mut Node<Arc<SpinLock<RrosThread>>>; // kernel;
        self.altsched = dovetail::DovetailAltschedContext::new();
        self.local_info = 0;
        self.wait_data = null_mut();
        self.poll_context = PollContext::new();
        self.inband_disable_count = Atomic::new();
        self.inband_work = IrqWork::new();
        self.stat = RrosStat::new();
        self.u_window = None;
        self.tracking_lock.init();
        // self.element = Rc::try_new(RefCell::new(RrosElement::new()?))?;
        self.affinity = CpumaskT::from_int(0 as u64);
        self.exited = completion::Completion::new();
        self.raised_cap = capability::KernelCapStruct::new();
        self.kill_next = ListHead {
            next: 0 as *mut ListHead,
            prev: 0 as *mut ListHead,
        };
        self.ptsync_next = ListHead {
            next: 0 as *mut ListHead,
            prev: 0 as *mut ListHead,
        };
        self.observable = None;
        self.name = "thread\0";
        self.oob_mm = OobMmState::new();
        // self.tps = 0 as *mut tp::RrosTpRq;
        self.tp_link = None;

        Ok(0)
    }
}

// TODO: move oob_mm_state to c in the mm_info.h
// pub struct oob_mm_state {
//     flags: u32,
//     //todo
//     // struct list_head ptrace_sync;
//     // struct rros_wait_queue ptsync_barrier;
// }
// impl oob_mm_state {
//     fn new() -> Self {
//         oob_mm_state { flags: 0 }
//     }
// }

pub struct PollContext {
    pub table: Option<Vec<RrosPollWatchpoint, alloc::alloc_rros::RrosMem>>,
    pub generation: u32,
    pub nr: i32,
    pub active: i32,
}
impl PollContext {
    fn new() -> Self {
        PollContext {
            table: None,
            generation: 0,
            nr: 0,
            active: 0,
        }
    }
}

//#[derive(Copy,Clone)]
#[allow(dead_code)]
pub enum RrosValue {
    Val(i32),
    Lval(i64),
    Ptr(*mut c_types::c_void),
}

impl RrosValue {
    pub fn new() -> Self {
        RrosValue::Lval(0)
    }
    #[allow(dead_code)]
    pub fn new_nil() -> Self {
        RrosValue::Ptr(null_mut())
    }
}

pub struct RrosPollNode {
    pub next: ListHead,
}
impl RrosPollNode {
    #[allow(dead_code)]
    fn new() -> Self {
        RrosPollNode {
            next: ListHead {
                next: 0 as *mut ListHead,
                prev: 0 as *mut ListHead,
            },
        }
    }
}
pub struct RrosUserWindow {
    pub state: u32,
    pub info: u32,
    pub pp_pending: u32,
}
impl RrosUserWindow {
    #[allow(dead_code)]
    fn new() -> Self {
        RrosUserWindow {
            state: 0,
            info: 0,
            pp_pending: 0,
        }
    }
}

pub struct RrosPollHead {
    pub watchpoints: ListHead,
    // FIXME: use ptr here not directly object
    pub lock: HardSpinlock,
}
impl RrosPollHead {
    pub fn new() -> Self {
        RrosPollHead {
            watchpoints: ListHead {
                next: 0 as *mut ListHead,
                prev: 0 as *mut ListHead,
            },
            lock: HardSpinlock::new(),
        }
    }
}

pub struct RrosInitThreadAttr {
    pub affinity: *const CpumaskT,
    pub observable: Option<Rc<RefCell<RrosObservable>>>,
    pub flags: i32,
    pub sched_class: Option<&'static RrosSchedClass>,
    pub sched_param: Option<Arc<SpinLock<RrosSchedParam>>>,
}
impl RrosInitThreadAttr {
    pub fn new() -> Self {
        RrosInitThreadAttr {
            affinity: null(),
            observable: None,
            flags: 0,
            sched_class: None,
            sched_param: None,
        }
    }
}
fn init_inband_timer(rq_ptr: *mut rros_rq) -> Result<usize> {
    unsafe {
        (*rq_ptr) = rros_rq::new()?;
        let mut x = SpinLock::new(RrosTimer::new(1));
        let pinned = Pin::new_unchecked(&mut x);
        spinlock_init!(pinned, "inband_timer");
        (*rq_ptr).inband_timer = Some(Arc::try_new(x)?);
    }
    Ok(0)
}

fn init_rrbtimer(rq_ptr: *mut rros_rq) -> Result<usize> {
    unsafe {
        let mut y = SpinLock::new(RrosTimer::new(1));
        let pinned = Pin::new_unchecked(&mut y);
        spinlock_init!(pinned, "rrb_timer");
        (*rq_ptr).rrbtimer = Some(Arc::try_new(y)?);
    }
    Ok(0)
}

fn init_root_thread(rq_ptr: *mut rros_rq) -> Result<usize> {
    unsafe {
        let mut tmp = Arc::<SpinLock<RrosThread>>::try_new_uninit()?;
        let mut tmp = {
            core::ptr::write_bytes(Arc::get_mut_unchecked(&mut tmp), 0, 1);
            tmp.assume_init()
        };
        let pinned = { Pin::new_unchecked(Arc::get_mut_unchecked(&mut tmp)) };
        spinlock_init!(pinned, "rros_kthreads");

        // let mut thread = SpinLock::new(RrosThread::new()?);
        // let pinned = Pin::new_unchecked(&mut thread);
        // spinlock_init!(pinned, "rros_threads");
        // Arc::get_mut(&mut tmp).unwrap().write(thread);

        (*rq_ptr).root_thread = Some(tmp); //Arc::try_new(thread)?
        (*(*rq_ptr).root_thread.as_mut().unwrap().locked_data().get()).init()?;
        let pinned = Pin::new_unchecked(
            &mut *(Arc::into_raw((*rq_ptr).root_thread.clone().unwrap())
                as *mut SpinLock<RrosThread>),
        );
        // &mut *Arc::into_raw( *(*rq_ptr).root_thread.clone().as_mut().unwrap()) as &mut SpinLock<RrosThread>
        spinlock_init!(pinned, "rros_threads");
        // (*rq_ptr).root_thread.as_mut().unwrap().assume_init();
    }
    Ok(0)
}

fn init_rtimer(rq_ptr: *mut rros_rq) -> Result<usize> {
    unsafe {
        let mut r = SpinLock::new(rros_timer::new(1));
        let pinned_r = Pin::new_unchecked(&mut r);
        spinlock_init!(pinned_r, "rtimer");
        (*rq_ptr).root_thread.as_ref().unwrap().lock().rtimer = Some(Arc::try_new(r)?);
    }
    Ok(0)
}

fn init_ptimer(rq_ptr: *mut rros_rq) -> Result<usize> {
    unsafe {
        let mut p = SpinLock::new(rros_timer::new(1));
        let pinned_p = Pin::new_unchecked(&mut p);
        spinlock_init!(pinned_p, "ptimer");
        (*rq_ptr).root_thread.as_ref().unwrap().lock().ptimer = Some(Arc::try_new(p)?);
    }
    Ok(0)
}

fn init_rq_ptr(rq_ptr: *mut rros_rq) -> Result<usize> {
    init_inband_timer(rq_ptr)?;
    init_rrbtimer(rq_ptr)?;
    init_root_thread(rq_ptr)?;
    init_rtimer(rq_ptr)?;
    init_ptimer(rq_ptr)?;
    // pr_debug!("{:p}\n", &(*rq_ptr).local_flags);
    // (*rq_ptr) = rros_rq::new()?;
    // let mut x = SpinLock::new(RrosTimer::new(1));
    // let pinned =  Pin::new_unchecked(&mut x);
    // spinlock_init!(pinned, "inband_timer");
    // (*rq_ptr).inband_timer =  Some(Arc::try_new(x)?);

    // let mut y = SpinLock::new(RrosTimer::new(1));
    // let pinned = Pin::new_unchecked(&mut y);
    // spinlock_init!(pinned, "rrb_timer");
    // (*rq_ptr).rrbtimer =  Some(Arc::try_new(y)?);

    // let mut y = SpinLock::new(RrosTimer::new(1));
    // let pinned = Pin::new_unchecked(&mut y);
    // spinlock_init!(pinned, "rrb_timer");
    // (*rq_ptr).rrbtimer =  Some(Arc::try_new(y)?);

    // let pinned = Pin::new_unchecked(&mut (*rq_ptr).root_thread.unwrap());
    // spinlock_init!(pinned, "root_thread");

    // let mut thread = SpinLock::new(RrosThread::new()?);
    // let pinned = Pin::new_unchecked(&mut thread);
    // spinlock_init!(pinned, "rros_threads");
    // (*rq_ptr).root_thread =  Some(Arc::try_new(thread)?);

    // let mut r = SpinLock::new(rros_timer::new(1));
    // let pinned_r =  Pin::new_unchecked(&mut r);
    // spinlock_init!(pinned_r, "rtimer");
    // (*rq_ptr).root_thread.as_ref().unwrap().lock().rtimer = Some(Arc::try_new(r)?);

    // let mut p = SpinLock::new(rros_timer::new(1));
    // let pinned_p =  Pin::new_unchecked(&mut p);
    // spinlock_init!(pinned_p, "ptimer");
    // (*rq_ptr).root_thread.as_ref().unwrap().lock().ptimer = Some(Arc::try_new(p)?);
    Ok(0)
}

fn init_rq_ptr_inband_timer(rq_ptr: *mut rros_rq) -> Result<usize> {
    unsafe {
        let mut tmp = Arc::<SpinLock<RrosThread>>::try_new_uninit()?;
        let mut tmp = {
            core::ptr::write_bytes(Arc::get_mut_unchecked(&mut tmp), 0, 1);
            tmp.assume_init()
        };
        let pinned = { Pin::new_unchecked(Arc::get_mut_unchecked(&mut tmp)) };
        spinlock_init!(pinned, "rros_kthreads");
        // let mut thread = SpinLock::new(RrosThread::new()?);
        // let pinned = Pin::new_unchecked(&mut thread);
        // spinlock_init!(pinned, "rros_threads");
        // Arc::get_mut(&mut tmp).unwrap().write(thread);

        (*rq_ptr).fifo.runnable.head = Some(List::new(tmp)); //Arc::try_new(thread)?
        (*(*rq_ptr)
            .fifo
            .runnable
            .head
            .as_mut()
            .unwrap()
            .head
            .value
            .locked_data()
            .get())
        .init()?;
        let pinned = Pin::new_unchecked(
            &mut *(Arc::into_raw(
                (*rq_ptr)
                    .fifo
                    .runnable
                    .head
                    .as_mut()
                    .unwrap()
                    .head
                    .value
                    .clone(),
            ) as *mut SpinLock<RrosThread>),
        );
        // &mut *Arc::into_raw( *(*rq_ptr).root_thread.clone().as_mut().unwrap()) as &mut SpinLock<RrosThread>
        spinlock_init!(pinned, "rros_threads");

        // let mut x = SpinLock::new(RrosThread::new()?);

        // let pinned = Pin::new_unchecked(&mut x);
        // spinlock_init!(pinned, "rros_runnable_thread");
        // (*rq_ptr).fifo.runnable.head = Some(List::new(Arc::try_new(x)?));
        // unsafe{(*rq_ptr).fifo.runnable.head = Some(List::new(Arc::try_new(SpinLock::new(RrosThread::new()?))?));}
    }
    Ok(0)
}

#[allow(dead_code)]
pub struct RrosSchedAttrs {
    pub sched_policy: i32,
    pub sched_priority: i32,
    // union {
    // 	struct __rros_rr_param rr;
    // 	struct __rros_quota_param quota;
    // 	struct __rros_tp_param tp;
    // } sched_u;
    pub tp_partition: i32,
}
impl RrosSchedAttrs {
    #[allow(dead_code)]
    pub fn new() -> Self {
        RrosSchedAttrs {
            sched_policy: 0,
            sched_priority: 0,
            tp_partition: 0,
        }
    }
}

pub fn rros_init_sched() -> Result<usize> {
    unsafe {
        RROS_RUNQUEUES = alloc_per_cpu(
            size_of::<rros_rq>() as usize,
            align_of::<rros_rq>() as usize,
        ) as *mut rros_rq;
        if RROS_RUNQUEUES == 0 as *mut rros_rq {
            return Err(kernel::Error::ENOMEM);
        }
    }

    for cpu in possible_cpus() {
        pr_debug!("{}\n", cpu);

        // let mut rq_ptr = this_rros_rq();
        let rq_ptr = unsafe { kernel::percpu_defs::per_cpu(RROS_RUNQUEUES, cpu as i32) };
        init_rq_ptr(rq_ptr)?;
        // // pr_debug!("{:p}\n", &(*rq_ptr).local_flags);
        // (*rq_ptr) = rros_rq::new()?;
        // let mut x = SpinLock::new(RrosTimer::new(1));
        // let pinned = Pin::new_unchecked(&mut x);
        // spinlock_init!(pinned, "inband_timer");
        // (*rq_ptr).inband_timer = Some(Arc::try_new(x)?);

        // let mut y = SpinLock::new(RrosTimer::new(1));
        // let pinned = Pin::new_unchecked(&mut y);
        // spinlock_init!(pinned, "rrb_timer");
        // (*rq_ptr).rrbtimer = Some(Arc::try_new(y)?);

        // // let mut y = SpinLock::new(RrosTimer::new(1));
        // // let pinned = Pin::new_unchecked(&mut y);
        // // spinlock_init!(pinned, "rrb_timer");
        // // (*rq_ptr).rrbtimer =  Some(Arc::try_new(y)?);

        // // let pinned = Pin::new_unchecked(&mut (*rq_ptr).root_thread.unwrap());
        // // spinlock_init!(pinned, "root_thread");

        // let mut thread = SpinLock::new(RrosThread::new()?);
        // let pinned = Pin::new_unchecked(&mut thread);
        // spinlock_init!(pinned, "rros_threads");
        // (*rq_ptr).root_thread = Some(Arc::try_new(thread)?);

        // let mut r = SpinLock::new(rros_timer::new(1));
        // let pinned_r = Pin::new_unchecked(&mut r);
        // spinlock_init!(pinned_r, "rtimer");

        // let mut p = SpinLock::new(rros_timer::new(1));
        // let pinned_p = Pin::new_unchecked(&mut p);
        // spinlock_init!(pinned_p, "ptimer");

        // (*rq_ptr).root_thread.as_ref().unwrap().lock().rtimer = Some(Arc::try_new(r)?);
        // (*rq_ptr).root_thread.as_ref().unwrap().lock().ptimer = Some(Arc::try_new(p)?);

        // pr_debug!("yinyongcishu is {}", Arc::strong_count(&(*rq_ptr).root_thread.clone().unwrap()));
        // pr_debug!("yinyongcishu is {}", Arc::strong_count(&(*rq_ptr).root_thread.clone().unwrap()));

        init_rq_ptr_inband_timer(rq_ptr)?;
        // unsafe {
        //     let mut x = SpinLock::new(RrosThread::new()?);

        //     let pinned = Pin::new_unchecked(&mut x);
        //     spinlock_init!(pinned, "rros_runnable_thread");
        //     (*rq_ptr).fifo.runnable.head = Some(List::new(Arc::try_new(x)?));
        //     // unsafe{(*rq_ptr).fifo.runnable.head = Some(List::new(Arc::try_new(SpinLock::new(RrosThread::new()?))?));}
        // }
    }

    // let cpu = 0;
    let ret = register_classes();
    match ret {
        Ok(_) => pr_debug!("register_classes success!"),
        Err(e) => {
            pr_err!("register_classes error!");
            return Err(e);
        }
    }

    #[cfg(CONFIG_SMP)]
    for cpu in online_cpus() {
        unsafe {
            if RROS_SCHED_TOPMOS == 0 as *mut RrosSchedClass {
                pr_debug!("RROS_SCHED_TOPMOS is 0 ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
            }
        }
        let rq_ptr = unsafe { kernel::percpu_defs::per_cpu(RROS_RUNQUEUES, cpu as i32) };
        // TODO: fix the i32 problem
        let ret = { init_rq(rq_ptr, cpu as i32) };
        match ret {
            Ok(_) => pr_debug!("init_rq success!"),
            Err(e) => {
                pr_warn!("init_rq error!");
                return Err(e);
            }
        }
    }

    #[cfg(CONFIG_SMP)]
    if num_possible_cpus() > 1 {
        let ret = interrupt::__request_percpu_irq(
            irq_get_reschedule_oob_ipi() as u32,
            Some(oob_reschedule_interrupt),
            bindings::IRQF_OOB as u64,
            unsafe {
                CStr::from_bytes_with_nul_unchecked("RROS reschedule\0".as_bytes()).as_char_ptr()
            },
            unsafe { RROS_MACHINE_CPUDATA as *mut _ },
        );
        if ret != 0 {
            pr_warn!("request_percpu_irq error!");
            return Err(Error::EINTR);
        }
    }

    pr_info!("sched init success!");
    Ok(0)
}

#[cfg(CONFIG_SMP)]
unsafe extern "C" fn oob_reschedule_interrupt(
    _irq: i32,
    _dev_id: *mut c_types::c_void,
) -> bindings::irqreturn_t {
    // trace_rros_reschedule_ipi(this_rros_rq());

    bindings::irqreturn_IRQ_HANDLED
}

#[cfg(not(CONFIG_SMP))]
unsafe extern "C" fn oob_reschedule_interrupt(
    _irq: i32,
    _dev_id: *mut c_types::c_void,
) -> bindings::irqreturn_t {
    bindings::irqreturn_IRQ_NONE
}

fn register_classes() -> Result<usize> {
    // let RROS_SCHED_IDLE = unsafe{idle::RROS_SCHED_IDLE};
    // let RrosSchedFifo = unsafe{fifo::RrosSchedFifo};
    let res = unsafe { register_one_class(&mut idle::RROS_SCHED_IDLE, 1) };
    unsafe {
        pr_debug!(
            "after one register_one_class,topmost = {:p}",
            RROS_SCHED_TOPMOS
        )
    };
    match res {
        Ok(_) => pr_info!("register_one_class(idle) success!"),
        Err(e) => {
            pr_warn!("register_one_class(idle) error!");
            return Err(e);
        }
    }
    // register_one_class(&mut RrosSchedWeak);

    let res = unsafe { register_one_class(&mut tp::RROS_SCHED_TP, 2) };
    unsafe {
        pr_debug!(
            "after two register_one_class,topmost = {:p}",
            RROS_SCHED_TOPMOS
        )
    };
    match res {
        Ok(_) => pr_info!("register_one_class(tp) success!"),
        Err(e) => {
            pr_warn!("register_one_class(tp) error!");
            return Err(e);
        }
    }
    let res = unsafe { register_one_class(&mut fifo::RROS_SCHED_FIFO, 3) };
    unsafe {
        pr_debug!(
            "after three register_one_class,topmost = {:p}",
            RROS_SCHED_TOPMOS
        )
    };
    match res {
        Ok(_) => pr_info!("register_one_class(fifo) success!"),
        Err(e) => {
            pr_warn!("register_one_class(fifo) error!");
            return Err(e);
        }
    }
    Ok(0)
}

// TODO: After the global variables are implemented, remove `index` and `topmost`.
fn register_one_class(sched_class: &mut RrosSchedClass, index: i32) -> Result<usize> {
    // let mut sched_class_lock = sched_class.lock();
    // let index = sched_class_lock.flag;
    // sched_class_lock.next = Some(RROS_SCHED_TOPMOS);
    // unsafe{sched_class.unlock()};
    unsafe { sched_class.next = RROS_SCHED_TOPMOS };
    if index == 1 {
        unsafe { RROS_SCHED_TOPMOS = &mut idle::RROS_SCHED_IDLE as *mut RrosSchedClass };
    } else if index == 2 {
        unsafe { RROS_SCHED_TOPMOS = &mut tp::RROS_SCHED_TP as *mut RrosSchedClass };
    // FIXME: Uncomment after implement tp.
    // unsafe{RROS_SCHED_TOPMOS  = 0 as *mut RrosSchedClass};
    } else if index == 3 {
        unsafe { RROS_SCHED_TOPMOS = &mut fifo::RROS_SCHED_FIFO as *mut RrosSchedClass };
    }
    unsafe {
        pr_debug!(
            "in register_one_class,RROS_SCHED_TOPMOS = {:p}",
            RROS_SCHED_TOPMOS
        )
    };
    if index != 3 {
        if index == 1 {
            unsafe { RROS_SCHED_LOWER = &mut idle::RROS_SCHED_IDLE as *mut RrosSchedClass };
        }
        if index == 2 {
            unsafe { RROS_SCHED_LOWER = &mut tp::RROS_SCHED_TP as *mut RrosSchedClass };
            // FIXME: Uncomment after implement tp.
        }
    }
    Ok(0)
}

// TODO: After the global variables are implemented, remove `topmost`.
fn init_rq(rq: *mut rros_rq, cpu: i32) -> Result<usize> {
    let mut iattr = RrosInitThreadAttr::new();
    let name_fmt: &'static CStr = c_str!("ROOT");
    // let mut rq_ptr = rq.borrow_mut();

    #[cfg(CONFIG_SMP)]
    unsafe {
        (*rq).cpu = cpu;
        (*rq).resched_cpus.cpumask_clear();
    }

    unsafe {
        (*rq).proxy_timer_name = kstrdup(
            CStr::from_bytes_with_nul("[proxy-timer]\0".as_bytes())?.as_char_ptr(),
            bindings::GFP_KERNEL,
        )
    };
    unsafe {
        (*rq).rrb_timer_name = kstrdup(
            CStr::from_bytes_with_nul("[rrb-timer]\0".as_bytes())?.as_char_ptr(),
            bindings::GFP_KERNEL,
        )
    };

    let mut p = unsafe { RROS_SCHED_TOPMOS };

    pr_debug!("before11111111111111111111111111111111111111");
    while p != 0 as *mut RrosSchedClass {
        pr_debug!("p!=0!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
        if unsafe { (*p).sched_init != None } {
            let func;
            unsafe {
                match (*p).sched_init {
                    Some(f) => func = f,
                    None => {
                        pr_warn!("sched_init function error");
                        return Err(kernel::Error::EINVAL);
                    }
                }
            }
            func(rq)?;
        }
        unsafe { p = (*p).next };
    }
    pr_debug!("after11111111111111111111111111111111111111");

    unsafe { (*rq).flags = 0 };
    unsafe { (*rq).local_flags = RQ_IDLE };
    // pr_debug!("yinyongcishu is {}", Arc::strong_count(&(*rq).root_thread.clone().unwrap()));
    let a = unsafe { (*rq).root_thread.clone() };
    if a.is_some() {
        unsafe { (*rq).curr = a.clone() };
    }
    // pr_debug!("The state is {:}\n", (*rq).get_curr().lock().state);
    unsafe {
        rros_init_timer_on_rq(
            (*rq).get_inband_timer(),
            &mut clock::RROS_MONO_CLOCK,
            None,
            rq,
            c_str!("tick"),
            RROS_TIMER_IGRAVITY,
        )
    };
    unsafe {
        rros_init_timer_on_rq(
            (*rq).get_rrbtimer(),
            &mut clock::RROS_MONO_CLOCK,
            Some(roundrobin_handler),
            rq,
            c_str!("rrb"),
            RROS_TIMER_IGRAVITY,
        )
    };
    // rros_set_timer_name(&rq->inband_timer, rq->proxy_timer_name);
    // rros_init_timer_on_rq(&rq->rrbtimer, &rros_mono_clock, roundrobin_handler,
    // 		rq, RROS_TIMER_IGRAVITY);
    // rros_set_timer_name(&rq->rrbtimer, rq->rrb_timer_name);

    // rros_set_current_account(rq, &rq->root_thread.stat.account);
    iattr.flags = T_ROOT as i32;
    iattr.affinity = CpumaskT::cpumask_of(cpu as u32) as *const _;
    // TODO: Wait for global variables.
    unsafe {
        iattr.sched_class = Some(&idle::RROS_SCHED_IDLE);
    }
    // Most of the comments below are caused by rros_init_thread not being completed.
    // let sched_param_clone;
    // let mut sched_param_ptr;
    // match iattr.sched_param {
    //     Some(p) => sched_param_clone = p.clone(),
    //     None => return Err(kernel::Error::EINVAL),
    // }
    // sched_param_ptr = sched_param_clone.borrow_mut();
    // sched_param_ptr.idle.prio = idle::RROS_IDLE_PRIO;

    let sched_param = unsafe { Arc::try_new(SpinLock::new(RrosSchedParam::new()))? };
    unsafe { (*sched_param.locked_data().get()).fifo.prio = idle::RROS_IDLE_PRIO };
    iattr.sched_param = Some(sched_param);

    // pr_debug!("yinyongcishu is {}", Arc::strong_count(&(*rq).root_thread.clone().unwrap()));
    // pr_debug!("yinyongcishu is {}", Arc::strong_count(&(*rq).root_thread.clone().unwrap()));
    unsafe { rros_init_thread(&(*rq).root_thread.clone(), iattr, rq, name_fmt)? }; //c_str!("0").as_char_ptr()

    unsafe {
        let next_add = (*rq).root_thread.clone().unwrap().lock().deref_mut() as *mut RrosThread;
        pr_debug!("the root thread add is  next_add {:p}", next_add);
    }

    // pr_debug!("The state is {:}\n", (*rq).get_curr().lock().state);
    let rq_root_thread_2;
    unsafe {
        match (*rq).root_thread.clone() {
            Some(rt) => rq_root_thread_2 = rt.clone(),
            None => {
                pr_warn!("use rq.root_thread error");
                return Err(kernel::Error::EINVAL);
            }
        }
    }
    // let mut rq_root_thread_lock = rq_root_thread_2.lock();
    // let add = &mut rq_root_thread_2.lock().deref_mut().altsched
    //     as *mut bindings::dovetail_altsched_context;
    // unsafe { bindings::dovetail_init_altsched(add) };
    rq_root_thread_2
        .lock()
        .deref_mut()
        .altsched
        .dovetail_init_altsched();

    // let mut rros_thread_list = list_head::new();
    // list_add_tail(
    //     &mut rq_root_thread_lock.next as *mut ListHead,
    //     &mut rros_thread_list as *mut ListHead,
    // );
    // rros_nrthreads += 1;
    Ok(0)
}

fn rros_sched_tick(rq: *mut rros_rq) -> Result {
    let curr;
    unsafe {
        curr = (*rq).get_curr();
    }
    let sched_class = curr.lock().sched_class.clone().unwrap();
    let flags = curr.lock().base_class.clone().unwrap().flag;
    let state = curr.lock().state;
    let a = sched_class.flag == flags;
    let b = !sched_class.sched_tick.is_none();
    let c = state & (RROS_THREAD_BLOCK_BITS | T_RRB) == T_RRB;
    let d = rros_preempt_count() == 0;
    // pr_debug!("The current root state {}", (*rq).root_thread.as_ref().unwrap().lock().state);
    pr_debug!("abcd {} {} {} {} state{} 2208\n", a, b, c, d, state);
    if a && b && c && d {
        sched_class.sched_tick.unwrap()(Some(rq))?;
        // sched_class->sched_tick(rq);
    }
    Ok(())
}

pub fn roundrobin_handler(_timer: *mut RrosTimer) {
    let rq = this_rros_rq();
    let res = rros_sched_tick(rq);
    match res {
        Ok(_) => (),
        Err(_e) => {
            pr_warn!("rros_sched_tick error!");
        }
    }
}

#[allow(dead_code)]
fn list_add_tail(new: *mut ListHead, head: *mut ListHead) {
    unsafe { rust_helper_list_add_tail(new, head) };
}

pub fn rros_set_effective_thread_priority(
    thread: Option<Arc<SpinLock<RrosThread>>>,
    prio: i32,
) -> Result<usize> {
    let thread_clone = thread.clone();
    let thread_unwrap = thread_clone.unwrap();
    let base_class;
    match thread_unwrap.lock().base_class.clone() {
        Some(t) => base_class = t,
        None => return Err(kernel::Error::EINVAL),
    };
    let wprio: i32 = rros_calc_weighted_prio(base_class, prio);
    thread_unwrap.lock().bprio = prio;

    let thread_wprio = thread_unwrap.lock().wprio;
    let state = thread_unwrap.lock().state;
    if wprio == thread_wprio {
        return Ok(0);
    }

    if wprio < thread_wprio && (state & T_BOOST) != 0 {
        return Err(kernel::Error::EINVAL);
    }

    thread_unwrap.lock().cprio = prio;

    Ok(0)
}

#[allow(dead_code)]
pub fn rros_track_priority(
    thread: Arc<SpinLock<RrosThread>>,
    p: Arc<SpinLock<RrosSchedParam>>,
) -> Result<usize> {
    unsafe {
        let func;
        match (*thread.locked_data().get())
            .sched_class
            .unwrap()
            .sched_trackprio
        {
            Some(f) => func = f,
            None => {
                pr_warn!("rros_get_schedparam: sched_trackprio function error");
                return Err(kernel::Error::EINVAL);
            }
        };
        func(Some(thread.clone()), Some(p.clone()));

        let sched_class = (*thread.locked_data().get()).sched_class.unwrap();
        let prio = (*thread.locked_data().get()).cprio;
        (*thread.locked_data().get()).wprio = rros_calc_weighted_prio(sched_class, prio);
    }
    Ok(0)
}

fn rros_ceil_priority(thread: Arc<SpinLock<RrosThread>>, prio: i32) -> Result<usize> {
    unsafe {
        let func;
        match (*thread.locked_data().get())
            .sched_class
            .unwrap()
            .sched_ceilprio
        {
            Some(f) => func = f,
            None => {
                pr_warn!("rros_ceil_priority:sched_ceilprio function error");
                return Err(kernel::Error::EINVAL);
            }
        }
        func(thread.clone(), prio);
        let sched_class = (*thread.locked_data().get()).sched_class.unwrap();
        let prio = (*thread.locked_data().get()).cprio;
        (*thread.locked_data().get()).wprio = rros_calc_weighted_prio(sched_class, prio);
    }
    Ok(0)
}

pub fn rros_calc_weighted_prio(sched_class: &'static RrosSchedClass, prio: i32) -> i32 {
    return prio + sched_class.weight;
}

pub fn rros_putback_thread(thread: Arc<SpinLock<RrosThread>>) -> Result<usize> {
    let state = thread.lock().state;
    if state & T_READY != 0 {
        rros_dequeue_thread(thread.clone())?;
    } else {
        thread.lock().state |= T_READY;
    }
    rros_enqueue_thread(thread.clone())?;
    let rq = thread.lock().rq;
    rros_set_resched(rq);
    Ok(0)
}

pub fn rros_dequeue_thread(thread: Arc<SpinLock<RrosThread>>) -> Result<usize> {
    let sched_class;
    match thread.lock().sched_class.clone() {
        Some(c) => sched_class = c,
        None => return Err(kernel::Error::EINVAL),
    }
    if sched_class.flag == 3 {
        fifo::__rros_dequeue_fifo_thread(thread.clone())?;
    } else if sched_class.flag != 1 {
        let func;
        match sched_class.sched_dequeue {
            Some(f) => func = f,
            None => return Err(kernel::Error::EINVAL),
        }
        func(thread.clone());
    }
    Ok(0)
}

pub fn rros_enqueue_thread(thread: Arc<SpinLock<RrosThread>>) -> Result<usize> {
    let sched_class;
    match thread.lock().sched_class.clone() {
        Some(c) => sched_class = c,
        None => return Err(kernel::Error::EINVAL),
    }
    if sched_class.flag == 3 {
        fifo::__rros_enqueue_fifo_thread(thread.clone())?;
    } else if sched_class.flag != 1 {
        let func;
        match sched_class.sched_enqueue {
            Some(f) => func = f,
            None => return Err(kernel::Error::EINVAL),
        }
        func(thread.clone())?;
    }
    Ok(0)
}

pub fn rros_requeue_thread(thread: Arc<SpinLock<RrosThread>>) -> Result<usize> {
    let sched_class;
    unsafe {
        match (*thread.locked_data().get()).sched_class.clone() {
            Some(c) => sched_class = c,
            None => return Err(kernel::Error::EINVAL),
        }
    }
    if sched_class.flag == 3 {
        fifo::__rros_requeue_fifo_thread(thread.clone())?;
    } else if sched_class.flag != 1 {
        let func;
        match sched_class.sched_requeue {
            Some(f) => func = f,
            None => return Err(kernel::Error::EINVAL),
        }
        func(thread.clone());
    }
    Ok(0)
}

/* hard irqs off. */
fn test_resched(rq: *mut rros_rq) -> bool {
    let need_resched = rros_need_resched(rq);

    #[cfg(CONFIG_SMP)]
    if unsafe { (*rq).resched_cpus.cpumask_empty().is_err() } {
        unsafe {
            irq_pipeline::irq_send_oob_ipi(
                irq_pipeline::irq_get_reschedule_oob_ipi(),
                &(*rq).resched_cpus,
            );
            (*rq).resched_cpus.cpumask_clear();
            (*rq).local_flags &= !RQ_SCHED;
        }
    }

    if need_resched {
        unsafe { (*rq).flags &= !RQ_SCHED }
        // unsafe{(*rq).local_flags &= !RQ_SCHED}
    }

    need_resched
}

#[no_mangle]
pub unsafe extern "C" fn rros_schedule() {
    unsafe {
        if RROS_RUNQUEUES == 0 as *mut rros_rq {
            return;
        }
    }

    let this_rq = this_rros_rq();
    let flags;
    let local_flags;

    unsafe {
        flags = (*this_rq).flags;
        local_flags = (*this_rq).local_flags;
    }

    // pr_debug!(
    //     "rros_schedule: flags is {} local_flags is {}\n",
    //     flags,
    //     local_flags
    // );

    //b kernel/rros/sched.rs:1670
    if ((flags | local_flags) & (RQ_IRQ | RQ_SCHED)) != RQ_SCHED {
        return;
    }

    let res = premmpt::running_inband();
    let r = match res {
        Ok(_o) => true,
        Err(_e) => false,
    };
    if !r {
        unsafe {
            __rros_schedule(0 as *mut c_types::c_void);
        }
        return;
    }

    irq_pipeline::run_oob_call(Some(__rros_schedule), 0 as *mut c_types::c_void);
}

extern "C" {
    #[allow(dead_code)]
    fn rust_helper_preempt_enable();
    #[allow(dead_code)]
    fn rust_helper_preempt_disable();
}

#[no_mangle]
unsafe extern "C" fn __rros_schedule(_arg: *mut c_types::c_void) -> i32 {
    unsafe {
        // fn __rros_schedule() {
        // pr_debug!("sched thread!!!!");
        // let prev = curr;
        let prev;
        let curr;
        let next;
        let this_rq = this_rros_rq();
        let mut leaving_inband;

        let flags = lock::hard_local_irq_save();

        curr = (*this_rq).get_curr();

        let curr_state = { (*curr.locked_data().get()).state };
        if curr_state & T_USER != 0x0 {
            //rros_commit_monitor_ceiling();
        }

        // There is no need for a spin lock here because there is only one CPU, so in theory there is no problem.
        // raw_spin_lock(&curr->lock);
        // raw_spin_lock(&this_rq->lock);

        if !(test_resched(this_rq)) {
            // raw_spin_unlock(&this_rq->lock);
            // raw_spin_unlock_irqrestore(&curr->lock, flags);
            // rust_helper_hard_local_irq_restore(flags);
            lock::hard_local_irq_restore(flags);
            return 0;
        }

        let curr_add = curr.locked_data().get();
        next = pick_next_thread(Some(this_rq)).unwrap();
        // unsafe{pr_debug!("begin of the rros_schedule uninit_thread: x ref is {}", Arc::strong_count(&next.clone()));}

        let next_add = next.locked_data().get();

        if next_add == curr_add {
            // if the curr and next are both root, we should call the inband thread
            pr_debug!("__rros_schedule: next_add == curr_add ");
            let next_state = (*next.locked_data().get()).state;
            if (next_state & T_ROOT as u32) != 0x0 {
                if (*this_rq).local_flags & RQ_TPROXY != 0x0 {
                    pr_debug!("__rros_schedule: (*this_rq).local_flags & RQ_TPROXY != 0x0 ");
                    tick::rros_notify_proxy_tick(this_rq);
                }
                if (*this_rq).local_flags & RQ_TDEFER != 0x0 {
                    pr_debug!("__rros_schedule: (*this_rq).local_flags & RQ_TDEFER !=0x0 ");
                    tick::rros_program_local_tick(
                        &mut clock::RROS_MONO_CLOCK as *mut clock::RrosClock,
                    );
                }
            }
            // rust_helper_hard_local_irq_restore(flags);
            lock::hard_local_irq_restore(flags);
            return 0;
        }

        prev = curr.clone();
        (*this_rq).curr = Some(next.clone());
        // unsafe{pr_debug!("mid of the rros_schedule uninit_thread: x ref is {}", Arc::strong_count(&next.clone()));}
        leaving_inband = false;

        let prev_state = (*prev.locked_data().get()).state;
        let next_state = (*next.locked_data().get()).state;
        if prev_state & T_ROOT as u32 != 0x0 {
            // leave_inband(prev);
            leaving_inband = true;
        } else if next_state & T_ROOT as u32 != 0x0 {
            if (*this_rq).local_flags & RQ_TPROXY != 0x0 {
                tick::rros_notify_proxy_tick(this_rq);
            }
            if (*this_rq).local_flags & RQ_TDEFER != 0x0 {
                tick::rros_program_local_tick(&mut clock::RROS_MONO_CLOCK as *mut clock::RrosClock);
            }
            // enter_inband(next);
        }

        // prepare_rq_switch(this_rq, prev, next);

        let prev_add = prev.locked_data().get();
        pr_debug!("the run thread add is  spinlock prev {:p}", prev_add);

        let next_add = next.locked_data().get();
        pr_debug!("the run thread add is  spinlock  next {:p}", next_add);
        // pr_debug!("the run thread add is  arc prev {:p}", prev);
        // pr_debug!("the run thread add is  arc next {:p}", next);

        // fix!!!!!
        let inband_tail;
        // pr_debug!("before the inband_tail next state is {}", next.lock().state);
        inband_tail = dovetail::dovetail_context_switch(
            &mut (*prev.locked_data().get()).altsched,
            &mut (*next.locked_data().get()).altsched,
            leaving_inband,
        );
        // next.unlock();
        // finish_rq_switch(inband_tail, flags); //b kernel/rros/sched.rs:1751

        // if prev ==
        // bindings::dovetail_context_switch();
        // inband_tail = dovetail_context_switch(&prev->altsched,
        //     &next->altsched, leaving_inband);

        // rust_helper_hard_local_irq_restore(flags);
        // pr_debug!("before the inband_tail curr state is {}", curr.lock().state);

        pr_debug!("the inband_tail is {}", inband_tail);
        if inband_tail == false {
            lock::hard_local_irq_restore(flags);
        }
        pr_debug!(
            "end of the rros_schedule uninit_thread: x ref is {}",
            Arc::strong_count(&next.clone())
        );
        0
    }
}

// TODO: add this function
#[allow(dead_code)]
fn finish_rq_switch() {}

pub fn pick_next_thread(rq: Option<*mut rros_rq>) -> Option<Arc<SpinLock<RrosThread>>> {
    let mut next: Option<Arc<SpinLock<RrosThread>>>;
    loop {
        next = __pick_next_thread(rq);
        let next_clone = next.clone().unwrap();
        let oob_mm = unsafe { (*next_clone.locked_data().get()).oob_mm };
        if oob_mm.is_null() {
            break;
        }
        unsafe {
            if test_bit(
                RROS_MM_PTSYNC_BIT,
                &(*(oob_mm.ptr)).flags as *const _ as *const usize,
            ) == false
            {
                break;
            }
        }
        let info = unsafe { (*next_clone.locked_data().get()).info };
        if info & (T_PTSTOP | T_PTSIG | T_KICKED) != 0 {
            break;
        }
        unsafe { (*next_clone.locked_data().get()).state |= T_PTSYNC };
        unsafe { (*next_clone.locked_data().get()).state &= !T_READY };
    }
    set_next_running(rq.clone(), next.clone());

    return next;
}

pub fn __pick_next_thread(rq: Option<*mut rros_rq>) -> Option<Arc<SpinLock<RrosThread>>> {
    let curr = unsafe { (*rq.clone().unwrap()).curr.clone().unwrap() };

    let next: Option<Arc<SpinLock<RrosThread>>>;

    let curr_state = unsafe { (*curr.locked_data().get()).state };
    if curr_state & (RROS_THREAD_BLOCK_BITS | T_ZOMBIE) == 0 {
        if rros_preempt_count() > 0 {
            let _ret = rros_set_self_resched(rq);
            return Some(curr.clone());
        }
        if curr_state & T_READY == 0 {
            let _ret = rros_requeue_thread(curr.clone());
            unsafe { (*curr.locked_data().get()).state |= T_READY };
        }
    }

    next = lookup_fifo_class(rq.clone());
    // pr_debug!("next2");
    if next.is_some() {
        pr_debug!("__pick_next_thread: next.is_some");
        return next;
    }

    // Although there is no loop here, there should be no problem.
    //TODO: Here is a for loop.
    let mut next;
    let mut p = unsafe { RROS_SCHED_LOWER };
    while p != 0 as *mut RrosSchedClass {
        pr_debug!("p!=0!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!! in sched_pick");
        if unsafe { (*p).sched_pick != None } {
            let func;
            unsafe {
                match (*p).sched_pick {
                    Some(f) => func = f,
                    None => {
                        pr_warn!("sched_pick function error, this should not happen");
                        return None;
                        // return Err(kernel::Error::EINVAL);
                    }
                }
            }
            next = func(rq.clone());
            match next {
                Ok(n) => return Some(n),
                Err(_e) => {
                    pr_warn!("nothing found");
                }
            }
        }
        unsafe { p = (*p).next };
    }
    // let func = unsafe { idle::RROS_SCHED_IDLE.sched_pick.unwrap() };
    // let next = func(rq.clone());
    // match next {
    //     Ok(n) => return Some(n),
    //     Err(e) => return None,
    // }
    return None;
}

pub fn lookup_fifo_class(rq: Option<*mut rros_rq>) -> Option<Arc<SpinLock<RrosThread>>> {
    let q = &mut unsafe { (*rq.clone().unwrap()).fifo.runnable.head.as_mut().unwrap() };
    if q.is_empty() {
        return None;
    }
    // pr_debug!("next0");
    let thread = q.get_head().unwrap().value.clone();
    let sched_class = unsafe { (*thread.locked_data().get()).sched_class.clone().unwrap() };

    if sched_class.flag != 3 {
        let func = sched_class.sched_pick.unwrap();
        return Some(func(rq).unwrap());
    }

    pr_debug!("lookup_fifo_class :2");
    q.de_head();
    return Some(thread.clone());
}

pub fn set_next_running(rq: Option<*mut rros_rq>, next: Option<Arc<SpinLock<RrosThread>>>) {
    let next = next.unwrap();
    unsafe { (*next.locked_data().get()).state &= !T_READY };
    let state = unsafe { (*next.locked_data().get()).state };
    pr_debug!("set_next_running: next.lock().state is {}", unsafe {
        (*next.locked_data().get()).state
    });
    if state & T_RRB != 0 {
        unsafe {
            let delta = (*next.locked_data().get()).rrperiod;
            rros_start_timer(
                (*rq.clone().unwrap()).rrbtimer.clone().unwrap(),
                rros_abs_timeout((*rq.clone().unwrap()).rrbtimer.clone().unwrap(), delta),
                RROS_INFINITE,
            )
        };
    } else {
        unsafe { rros_stop_timer((*rq.clone().unwrap()).rrbtimer.clone().unwrap()) };
    }
}

fn rros_preempt_count() -> i32 {
    dovetail::dovetail_current_state().preempt_count()
}

fn test_bit(nr: i32, addr: *const usize) -> bool {
    unsafe { return rust_helper_test_bit(nr, addr) };
}

pub fn rros_set_thread_policy(
    thread: Option<Arc<SpinLock<RrosThread>>>,
    sched_class: Option<&'static RrosSchedClass>,
    p: Option<Arc<SpinLock<RrosSchedParam>>>,
) -> Result<usize> {
    let mut flags: c_types::c_ulong = 0;
    // let test = p.clone().unwrap();
    let rq: Option<*mut rros_rq>;
    rq = rros_get_thread_rq(thread.clone(), &mut flags);
    pr_debug!("rros_get_thread_rq success");
    rros_set_thread_policy_locked(thread.clone(), sched_class.clone(), p.clone())?;
    pr_debug!("rros_set_thread_policy_locked success");
    rros_put_thread_rq(thread.clone(), rq.clone(), flags)?;
    pr_debug!("rros_put_thread_rq success");
    Ok(0)
}

pub fn rros_get_thread_rq(
    thread: Option<Arc<SpinLock<RrosThread>>>,
    flags: &mut c_types::c_ulong,
) -> Option<*mut rros_rq> {
    // pr_debug!("yinyongcishu is {}", Arc::strong_count(&thread.clone().unwrap()));
    //todo raw_spin_lock_irqsave and raw_spin_lock
    *flags = lock::hard_local_irq_save();
    // unsafe{rust_helper_preempt_disable();}
    unsafe { (*thread.unwrap().locked_data().get()).rq.clone() }
}

pub fn rros_put_thread_rq(
    _thread: Option<Arc<SpinLock<RrosThread>>>,
    _rq: Option<*mut rros_rq>,
    flags: c_types::c_ulong,
) -> Result<usize> {
    // unsafe {
    //     rust_helper_hard_local_irq_restore(flags);
    //     // rust_helper_preempt_enable();
    // }
    lock::hard_local_irq_restore(flags);
    // TODO: raw_spin_unlock and raw_spin_unlock_irqrestore
    Ok(0)
}

pub fn rros_set_thread_policy_locked(
    thread: Option<Arc<SpinLock<RrosThread>>>,
    sched_class: Option<&'static RrosSchedClass>,
    p: Option<Arc<SpinLock<RrosSchedParam>>>,
) -> Result<usize> {
    let _test = p.clone().unwrap();
    let thread_unwrap = thread.clone().unwrap();
    let _orig_effective_class: Option<Rc<RefCell<RrosSchedClass>>> = None;
    let effective: Result<usize>;
    rros_check_schedparams(thread.clone(), sched_class.clone(), p.clone())?;
    let mut flag_base_class = 0;
    let base_class = thread_unwrap.lock().base_class;
    if base_class.is_none() {
        // pr_debug!("baseclass is none!");
        flag_base_class = 1;
    }

    if flag_base_class == 1
        || (sched_class.unwrap() as *const RrosSchedClass)
            != (base_class.unwrap() as *const RrosSchedClass)
    {
        rros_declare_thread(thread.clone(), sched_class.clone(), p.clone())?;
    }
    // pr_debug!("yinyongcishu is {}", Arc::strong_count(&thread.clone().unwrap()));
    if base_class.is_some() {
        let state = thread_unwrap.lock().state;
        if state & T_READY != 0x0 {
            rros_dequeue_thread(thread.clone().unwrap())?;
        }

        if (sched_class.unwrap() as *const RrosSchedClass)
            != (base_class.unwrap() as *const RrosSchedClass)
        {
            rros_forget_thread(thread.clone().unwrap())?;
        }
    }
    thread_unwrap.lock().base_class = sched_class.clone();
    // todo RROS_DEBUG
    // if (RROS_DEBUG(CORE)) {
    //     orig_effective_class = thread->sched_class;
    //     thread->sched_class = NULL;
    // }
    // let test = p.clone().unwrap();
    // pr_debug!("! yinyongcishu is {}", Arc::strong_count(&thread.clone().unwrap()));
    effective = rros_set_schedparam(thread.clone(), p.clone());
    // pr_debug!("thread after setting {}", thread_unwrap.lock().state);
    // pr_debug!("! yinyongcishu is {}", Arc::strong_count(&thread.clone().unwrap()));
    if effective == Ok(0) {
        thread_unwrap.lock().sched_class = sched_class.clone();
        let cprio = thread_unwrap.lock().cprio;
        let wprio = rros_calc_weighted_prio(sched_class.clone().unwrap(), cprio);
        thread_unwrap.lock().wprio = wprio;
    }
    // todo RROS_DEBUG
    // else if (RROS_DEBUG(CORE))
    //     thread->sched_class = orig_effective_class;
    let state = thread_unwrap.lock().state;
    if state & T_READY != 0x0 {
        // pr_debug!("wwwwwwwwhat the fuck!");
        rros_enqueue_thread(thread.clone().unwrap())?;
    }

    let state = thread_unwrap.lock().state;
    if state & (T_DORMANT | T_ROOT as u32) == 0x0 {
        let rq = thread_unwrap.lock().rq;
        rros_set_resched(rq);
    }
    // pr_debug!("hy kkkkk3 {}", thread_unwrap.lock().state);
    // pr_debug!("yinyongcishu is {}", Arc::strong_count(&thread.clone().unwrap()));
    Ok(0)
}

fn rros_check_schedparams(
    thread: Option<Arc<SpinLock<RrosThread>>>,
    sched_class: Option<&'static RrosSchedClass>,
    p: Option<Arc<SpinLock<RrosSchedParam>>>,
) -> Result<usize> {
    let sched_class_ptr = sched_class.unwrap();
    if sched_class_ptr.sched_chkparam.is_some() {
        let func = sched_class_ptr.sched_chkparam.unwrap();
        func(thread.clone(), p.clone())?;
    } else {
        pr_debug!("rros_check_schedparams no sched_chkparam functions");
    }
    Ok(0)
}

#[allow(dead_code)]
pub fn rros_get_schedparam(
    thread: Arc<SpinLock<RrosThread>>,
    p: Arc<SpinLock<RrosSchedParam>>,
) -> Result<usize> {
    let func;
    unsafe {
        match (*thread.locked_data().get())
            .sched_class
            .unwrap()
            .sched_getparam
        {
            Some(f) => func = f,
            None => {
                pr_warn!("rros_get_schedparam: sched_getparam function error");
                return Err(kernel::Error::EINVAL);
            }
        };
        func(Some(thread.clone()), Some(p.clone()));
    }
    Ok(0)
}

fn rros_set_schedparam(
    thread: Option<Arc<SpinLock<RrosThread>>>,
    p: Option<Arc<SpinLock<RrosSchedParam>>>,
) -> Result<usize> {
    let thread_clone = thread.clone();
    let thread_unwrap = thread_clone.unwrap();
    // let thread_lock = thread_unwrap.lock();
    let base_class_clone = thread_unwrap.lock().base_class.clone();
    if base_class_clone.is_none() {
        pr_info!("rros_set_schedparam: finded");
    }
    let base_class_unwrap = base_class_clone.unwrap();
    let func = base_class_unwrap.sched_setparam.unwrap();
    // pr_debug!("thread before setting {}", thread_unwrap.lock().state);
    let res = func(thread.clone(), p.clone());
    // pr_debug!("thread before calling {}", thread_unwrap.lock().state);
    res
    // return ;
}

// TODO: Remain to be refactored.
fn rros_declare_thread(
    thread: Option<Arc<SpinLock<RrosThread>>>,
    sched_class: Option<&'static RrosSchedClass>,
    p: Option<Arc<SpinLock<RrosSchedParam>>>,
) -> Result<usize> {
    let thread_clone = thread.clone();
    let thread_unwrap = thread_clone.unwrap();
    let sched_class_ptr = sched_class.unwrap();
    if sched_class_ptr.sched_declare.is_some() {
        let func = sched_class_ptr.sched_declare.unwrap();
        func(thread.clone(), p.clone())?;
    }
    let base_class = thread_unwrap.lock().base_class;
    unsafe {
        if base_class.is_none()
            || (sched_class_ptr as *const RrosSchedClass)
                != (base_class.unwrap() as *const RrosSchedClass)
        {
            let sched_class_mutptr = sched_class_ptr as *const RrosSchedClass;
            let mut sched_class_mutptr_mut = sched_class_mutptr as *mut RrosSchedClass;
            (*sched_class_mutptr_mut).nthreads += 1;
        }
    }

    pr_info!("rros_declare_thread success!");
    Ok(0)
}

pub fn rros_forget_thread(thread: Arc<SpinLock<RrosThread>>) -> Result<usize> {
    let thread_clone = thread.clone();
    // let thread_lock = thread_clone.lock();
    let sched_class = thread_clone.lock().base_class.clone();
    let sched_class_ptr = sched_class.unwrap() as *const RrosSchedClass;
    let mut sched_class_ptr = sched_class_ptr as *mut RrosSchedClass;
    unsafe {
        (*sched_class_ptr).nthreads -= 1;
    }

    unsafe {
        if (*sched_class_ptr).sched_forget.is_some() {
            let func = (*sched_class_ptr).sched_forget.unwrap();
            func(thread.clone())?;
        }
    }

    Ok(0)
}

extern "C" {
    fn rust_helper_unstall_oob();
    fn rust_helper_stall_oob();
}

#[cfg(CONFIG_SMP)]
pub fn check_cpu_affinity(thread: Arc<SpinLock<RrosThread>>, cpu: i32) {
    let rq = rros_cpu_rq(cpu);
    unsafe { (*thread.locked_data().get()).lock.raw_spin_lock() };

    let thread_rq = unsafe { (*thread.locked_data().get()).rq.unwrap() };

    if thread_rq != rq {
        if !is_threading_cpu(cpu) {
            let name = unsafe { (*thread.locked_data().get()).name };
            pr_warn!(
                "thread {:?} switched to non-rt CPU{:?}, aborted.",
                name,
                cpu
            );

            unsafe {
                (*rq).lock.raw_spin_lock();
                (*thread.locked_data().get()).info |= T_CANCELD;
                (*rq).lock.raw_spin_unlock();
            }
        } else if unsafe {
            (*thread.locked_data().get())
                .affinity
                .cpumask_test_cpu(cpu as u32)
                == false
        } {
            unsafe {
                (*thread.locked_data().get())
                    .affinity
                    .cpumask_set_cpu(cpu as u32);
            }
        }
        rros_migrate_thread(thread.clone(), rq);
    }

    unsafe {
        (*thread.locked_data().get()).lock.raw_spin_unlock();
    }
}

#[cfg(not(CONFIG_SMP))]
pub fn check_cpu_affinity(_thread: Arc<SpinLock<RrosThread>>, _cpu: i32) {}

#[no_mangle]
unsafe extern "C" fn rust_resume_oob_task(ptr: *mut c_types::c_void, cpu: i32) {
    // struct RrosThread *thread = rros_thread_from_task(p);

    // pr_debug!("rros rros mutex ptr{:p}", ptr);
    let thread: Arc<SpinLock<RrosThread>>;

    unsafe {
        thread = Arc::from_raw(ptr as *mut SpinLock<RrosThread>);
        pr_debug!(
            "0600 uninit_thread: x ref is {}",
            Arc::strong_count(&thread)
        );
        Arc::increment_strong_count(ptr);
        pr_debug!(
            "b600 uninit_thread: x ref is {}",
            Arc::strong_count(&thread)
        );
        pr_debug!("the ptr in resume address is {:p}", ptr);
        pr_debug!(
            "a600 uninit_thread: x ref is {}",
            Arc::strong_count(&thread)
        );
        // unsafe{pr_debug!("600 uninit_thread: x ref is {}", Arc::strong_count(ptr));}
    }
    pr_debug!(
        "2a600 uninit_thread: x ref is {}",
        Arc::strong_count(&thread)
    );

    /*
     * Dovetail calls us with hard irqs off, oob stage
     * stalled. Clear the stall bit which we don't use for
     * protection but keep hard irqs off.
     */
    unsafe {
        rust_helper_unstall_oob();
    }
    check_cpu_affinity(thread.clone(), cpu);
    pr_debug!(
        "3a600 uninit_thread: x ref is {}",
        Arc::strong_count(&thread)
    );
    rros_release_thread(thread.clone(), T_INBAND, 0);
    /*
     * If T_PTSTOP is set, pick_next_thread() is not allowed to
     * freeze @thread while in flight to the out-of-band stage.
     */
    pr_debug!(
        "4a600 uninit_thread: x ref is {}",
        Arc::strong_count(&thread)
    );
    unsafe {
        sched::rros_schedule();
        pr_debug!(
            "5a600 uninit_thread: x ref is {}",
            Arc::strong_count(&thread)
        );
        rust_helper_stall_oob();
    }
    pr_debug!(
        "6a600 uninit_thread: x ref is {}",
        Arc::strong_count(&thread)
    );
}

extern "C" {
    fn rust_helper_hard_local_irq_disable();
    fn rust_helper_hard_local_irq_enable();
}

pub fn rros_switch_inband(cause: i32) {
    pr_debug!("rros_switch_inband: in");
    let curr = unsafe { &mut *rros_current() };
    let this_rq: Option<*mut rros_rq>;
    let notify: bool;
    unsafe {
        rust_helper_hard_local_irq_disable();
    }
    let _ret = curr.lock().inband_work.irq_work_queue();
    this_rq = curr.lock().rq.clone();

    curr.lock().state |= T_INBAND;
    curr.lock().local_info &= !T_SYSRST;
    notify = curr.lock().state & T_USER != 0x0 && cause > (RROS_HMDIAG_NONE as i32);

    let info = curr.lock().info;
    if cause == (RROS_HMDIAG_TRAP as i32) {
        pr_debug!("rros_switch_inband: cause == RROS_HMDIAG_TRAP");
        // TODO:
    } else if info & T_PTSIG != 0x0 {
        pr_debug!("rros_switch_inband: curr->info & T_PTSIG");
        // TODO:
    }

    curr.lock().info &= !RROS_THREAD_INFO_MASK;
    rros_set_resched(this_rq);
    unsafe {
        dovetail::dovetail_leave_oob();
        __rros_schedule(0 as *mut c_types::c_void);
        rust_helper_hard_local_irq_enable();
        // bindings::dovetail_resume_inband();
        dovetail::dovetail_resume_inband();
    }

    // curr.lock().stat.isw.inc_counter();
    // rros_propagate_schedparam_change(curr);

    if notify {
        // TODO:
    }

    //rros_sync_uwindow(curr); todo
}

#[inline]
pub fn rros_enable_preempt() {
    extern "C" {
        fn rust_helper_rros_enable_preempt_top_part() -> bool;
    }
    unsafe {
        if rust_helper_rros_enable_preempt_top_part() {
            rros_schedule()
        }
    }
}

#[inline]
pub fn rros_disable_preempt() {
    extern "C" {
        fn rust_helper_rros_disable_preempt();
    }
    unsafe {
        rust_helper_rros_disable_preempt();
    }
}

#[inline]
pub fn rros_force_thread(thread: Arc<SpinLock<RrosThread>>) {
    // assert_thread_pinned(thread);
    {
        let guard = thread.lock();
        if guard.base_class.is_some() {
            guard.base_class.unwrap().sched_kick.unwrap()(thread.clone());
        }
    }
}
