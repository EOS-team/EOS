use crate::{
    clock::{self, rros_read_clock},
    factory::{self, rros_init_element, RrosElement, RrosFactory},
    fifo::{self, RROS_SCHED_FIFO},
    file::RrosFileBinding,
    idle, lock,
    sched::*,
    timeout,
    timer::{self, program_timer, rros_dequeue_timer, rros_stop_timer, rros_update_timer_date},
    wait::RrosWaitChannel,
    RROS_OOB_CPUS,
};

use alloc::rc::Rc;

use core::{
    cell::RefCell,
    clone::Clone,
    ops::DerefMut,
    ptr,
    result::Result::{Err, Ok},
};

#[warn(unused_mut)]
use kernel::{
    bindings, c_str, c_types, capability,
    completion::Completion,
    cpumask::CpumaskT,
    cred,
    device::DeviceType,
    dovetail,
    error::{from_kernel_err_ptr, Error},
    file::File,
    file_operations::FileOperations,
    fs,
    io_buffer::IoBufferWriter,
    irq_work::IrqWork,
    kernelh, ktime,
    ktime::ktime_sub,
    prelude::*,
    premmpt,
    sched::sched_setscheduler,
    spinlock_init,
    str::CStr,
    sync::{Guard, Lock, SpinLock},
    task::{self, Task},
    types,
};

extern "C" {
    fn rust_helper_kthread_run(
        threadfn: Option<unsafe extern "C" fn(*mut c_types::c_void) -> c_types::c_int>,
        data: *mut c_types::c_void,
        namefmt: *const c_types::c_char,
        ...
    ) -> *mut c_types::c_void;
    fn rust_helper_unstall_oob();
    fn rust_helper_hard_local_irq_enable();
    #[allow(dead_code)]
    fn rust_helper_preempt_enable();
    #[allow(dead_code)]
    fn rust_helper_preempt_disable();
    fn rust_helper_hard_local_irq_save() -> c_types::c_ulong;
    fn rust_helper_hard_local_irq_restore(flags: c_types::c_ulong);
    // fn rust_helper_doveail_mm_state() -> *mut bindings::oob_mm_state;
}

pub const SIGDEBUG: i32 = 24;
pub const SIGDEBUG_MARKER: u32 = 0xfccf0000;
#[allow(dead_code)]
pub const SI_QUEUE: i32 = -1;
pub const T_SUSP: u32 = 0x00000001; /* Suspended */
pub const T_PEND: u32 = 0x00000002; /* Blocked on a wait_queue/mutex */
pub const T_DELAY: u32 = 0x00000004; /* Delayed/timed */
pub const T_WAIT: u32 = 0x00000008; /* Periodic wait */
pub const T_READY: u32 = 0x00000010; /* Ready to run (in rq) */
pub const T_DORMANT: u32 = 0x00000020; /* Not started yet */
pub const T_ZOMBIE: u32 = 0x00000040; /* Dead, waiting for disposal */
pub const T_INBAND: u32 = 0x00000080; /* Running in-band */
pub const T_HALT: u32 = 0x00000100; /* Halted */
pub const T_BOOST: u32 = 0x00000200; /* PI/PP boost undergoing */
pub const T_PTSYNC: u32 = 0x00000400; /* Synchronizing on ptrace event */
pub const T_RRB: u32 = 0x00000800; /* Undergoes round-robin scheduling */
pub const T_ROOT: u32 = 0x00001000; /* Root thread (in-band kernel placeholder) */
pub const T_WEAK: u32 = 0x00002000; /* Weak scheduling (in-band) */
pub const T_USER: u32 = 0x00004000; /* Userland thread */
#[allow(dead_code)]
pub const T_WOSS: u32 = 0x00008000; /* Warn on stage switch (HM) */
pub const T_WOLI: u32 = 0x00010000; /* Warn on locking inconsistency (HM) */
#[allow(dead_code)]
pub const T_WOSX: u32 = 0x00020000; /* Warn on stage exclusion (HM) */
#[allow(dead_code)]
pub const T_PTRACE: u32 = 0x00040000; /* Stopped on ptrace event */
pub const T_OBSERV: u32 = 0x00080000; /* Observable (only for export to userland) */
pub const T_HMSIG: u32 = 0x00100000; /* Notify HM events via SIGDEBUG */
#[allow(dead_code)]
pub const T_HMOBS: u32 = 0x00200000; /* Notify HM events via observable */

pub const T_TIMEO: u32 = 0x00000001; /* Woken up due to a timeout condition */
pub const T_RMID: u32 = 0x00000002; /* Pending on a removed resource */
pub const T_BREAK: u32 = 0x00000004; /* Forcibly awaken from a wait state */
pub const T_KICKED: u32 = 0x00000008; /* Forced out of OOB context */
pub const T_WAKEN: u32 = 0x00000010; /* Thread waken up upon resource availability */
pub const T_ROBBED: u32 = 0x00000020; /* Robbed from resource ownership */
pub const T_CANCELD: u32 = 0x00000040; /* Cancellation request is pending */
#[allow(dead_code)]
pub const T_PIALERT: u32 = 0x00000080; /* Priority inversion alert (HM notified) */
pub const T_SCHEDP: u32 = 0x00000100; /* Schedparam propagation is pending */
pub const T_BCAST: u32 = 0x00000200; /* Woken up upon resource broadcast */
#[allow(dead_code)]
pub const T_SIGNAL: u32 = 0x00000400; /* Event monitor signaled */
#[allow(dead_code)]
pub const T_SXALERT: u32 = 0x00000800; /* Stage exclusion alert (HM notified) */
pub const T_PTSIG: u32 = 0x00001000; /* Ptrace signal is pending */
pub const T_PTSTOP: u32 = 0x00002000; /* Ptrace stop is ongoing */
#[allow(dead_code)]
pub const T_PTJOIN: u32 = 0x00004000; /* Ptracee should join ptsync barrier */
pub const T_NOMEM: u32 = 0x00008000; /* No memory to complete the operation */

pub const T_SYSRST: u32 = 0x00000001; /* Thread awaiting syscall restart after signal */
pub const T_IGNOVR: u32 = 0x00000002; /* Overrun detection temporarily disabled */
#[allow(dead_code)]
pub const T_INFAULT: u32 = 0x00000004; /* In fault handling */

const RROS_INFINITE: i64 = 0;

pub const RROS_THREAD_BLOCK_BITS: u32 =
    T_SUSP | T_PEND | T_DELAY | T_WAIT | T_DORMANT | T_INBAND | T_HALT | T_PTSYNC;
pub const RROS_THREAD_INFO_MASK: u32 =
    T_RMID | T_TIMEO | T_BREAK | T_WAKEN | T_ROBBED | T_KICKED | T_BCAST | T_NOMEM;
#[allow(dead_code)]
pub const RROS_THREAD_MODE_BITS: u32 = T_WOSS | T_WOLI | T_WOSX | T_HMSIG | T_HMOBS;

pub const RROS_HMDIAG_NONE: i32 = 0;
pub const RROS_HMDIAG_TRAP: i32 = -1;
#[allow(dead_code)]
pub const RROS_HMDIAG_LKDEPEND: i32 = 5;
#[allow(dead_code)]
pub const RROS_HMDIAG_LKIMBALANCE: i32 = 6;
#[allow(dead_code)]
pub const RROS_HMDIAG_LKSLEEP: i32 = 7;
pub const RROS_HMDIAG_STAGEX: u32 = 8;

#[allow(dead_code)]
pub const RROS_THRIOC_SET_SCHEDPARAM: u32 = 1;
#[allow(dead_code)]
pub const RROS_THRIOC_GET_SCHEDPARAM: u32 = 2;
#[allow(dead_code)]
pub const RROS_THRIOC_GET_STATE: u32 = 4;

// TODO: move this to the config file
pub const CONFIG_RROS_NR_THREADS: usize = 16;

pub static mut RROS_THREAD_FACTORY: SpinLock<factory::RrosFactory> = unsafe {
    SpinLock::new(factory::RrosFactory {
        // TODO: move this and clock factory name to a variable
        name: CStr::from_bytes_with_nul_unchecked("thread\0".as_bytes()),
        // fops: Some(&ThreadOps),
        nrdev: CONFIG_RROS_NR_THREADS,
        // TODO: add the corresponding ops
        build: Some(thread_factory_build),
        // TODO: add the corresponding ops
        dispose: None,
        // TODO: add the corresponding attr
        attrs: None, //sysfs::attribute_group::new(),
        // TODO: rename this flags to the bit level variable RROS_FACTORY_CLONE and RROS_FACTORY_SINGLE
        flags: factory::RrosFactoryType::CLONE,
        inside: Some(factory::RrosFactoryInside {
            type_: DeviceType::new(),
            class: None,
            cdev: None,
            device: None,
            sub_rdev: None,
            kuid: None,
            kgid: None,
            minor_map: None,
            index: None,
            name_hash: None,
            hash_lock: None,
            register: None,
        }),
    })
};

#[derive(Default)]
pub struct ThreadOps;

impl FileOperations for ThreadOps {
    kernel::declare_file_operations!(read);

    fn read<T: IoBufferWriter>(
        _this: &Self,
        _file: &File,
        _data: &mut T,
        _offset: u64,
    ) -> Result<usize> {
        pr_debug!("I'm the read ops of the rros thread factory.");
        Ok(1)
    }
}

use core::fmt;
use core::fmt::{Debug, Formatter};

pub struct RrosKthread {
    pub thread: Option<Arc<SpinLock<RrosThread>>>,
    pub done: Completion,
    pub kthread_fn: Option<Box<dyn FnOnce() -> ()>>,
    status: i32,
    pub irq_work: IrqWork,
}

impl Debug for RrosKthread {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("good ew")
    }
}

impl RrosKthread {
    pub fn new(kfn: Option<Box<dyn FnOnce() -> ()>>) -> Self {
        RrosKthread {
            thread: None,
            done: Completion::new(),
            kthread_fn: kfn,
            status: 0,
            irq_work: IrqWork::new(),
        }
    }
}

pub struct SigIrqworkData {
    #[allow(dead_code)]
    thread: *mut RrosThread,
    #[allow(dead_code)]
    signo: i32,
    #[allow(dead_code)]
    sigval: u32,
    work: IrqWork,
}

impl SigIrqworkData {
    pub fn new() -> Self {
        SigIrqworkData {
            thread: 0 as *mut RrosThread,
            signo: 0,
            sigval: 0,
            work: IrqWork::new(),
        }
    }
}

#[allow(dead_code)]
pub struct RrosThreadState {
    pub eattrs: RrosSchedAttrs,
    pub cpu: u32,
    pub state: u32,
    pub isw: u32,
    pub csw: u32,
    pub sc: u32,
    pub rwa: u32,
    pub xtime: u32,
}
impl RrosThreadState {
    #[allow(dead_code)]
    pub fn new() -> Self {
        RrosThreadState {
            eattrs: RrosSchedAttrs::new(),
            cpu: 0,
            state: 0,
            isw: 0,
            csw: 0,
            sc: 0,
            rwa: 0,
            xtime: 0,
        }
    }
}

pub fn rros_init_thread(
    thread: &Option<Arc<SpinLock<RrosThread>>>,
    // rq_s: Rc<RefCell<rros_rq>>,
    // iattr: Rc<RefCell<RrosInitThreadAttr>>,
    iattr: RrosInitThreadAttr,
    mut rq: *mut rros_rq,
    _fmt: &'static CStr,
    // args:fmt::Arguments<'_>
) -> Result<usize> {
    let iattr_ptr = iattr;
    let mut flags = iattr_ptr.flags & (!T_SUSP as i32);

    if flags & (T_ROOT as i32) == 0x0 {
        pr_info!("called timesssss");
        flags |= (T_DORMANT | T_INBAND) as i32;
    }

    //if ((flags & T_USER) && IS_ENABLED(CONFIG_RROS_DEBUG_WOLI))
    //       flags |= T_WOLI;

    if iattr_ptr.observable.is_some() {
        flags |= T_OBSERV as i32;
    }

    if rq.is_null() {
        let affinity = unsafe { (*iattr_ptr.affinity).clone() & RROS_CPU_AFFINITY.clone() };
        if affinity.cpumask_empty().is_err() {
            rq = rros_cpu_rq(affinity.cpumask_first());
        }
        if rq.is_null() {
            return Err(Error::EINVAL);
        }
    }

    // pr_debug!("hello world flags {}", flags);
    // va_start(args,fmt);
    // pr_debug!("yinyongcishu is {}", Arc::strong_count(&thread.clone().unwrap()));
    let thread_clone = thread.clone();
    let thread_unwrap = thread_clone.unwrap();
    // let mut thread_lock = thread_unwrap.lock();
    // thread_lock.name = kvasprintf(gfp, fmt, args);
    // va_end(args);
    // pr_debug!("hello world");
    // if thread_lock.name == " "{
    //     return Err(kernel::Error::ENOMEM);
    // }

    unsafe {
        (*thread_unwrap.locked_data().get()).affinity =
            (*iattr_ptr.affinity).clone() & RROS_CPU_AFFINITY.clone();
    }

    thread_unwrap.lock().rq = Some(rq);
    thread_unwrap.lock().state = flags as u32;
    thread_unwrap.lock().info = 0;
    thread_unwrap.lock().local_info = 0;
    thread_unwrap.lock().wprio = idle::RROS_IDLE_PRIO;
    thread_unwrap.lock().cprio = idle::RROS_IDLE_PRIO;
    thread_unwrap.lock().bprio = idle::RROS_IDLE_PRIO;
    thread_unwrap.lock().rrperiod = RROS_INFINITE;
    thread_unwrap.lock().wchan = core::ptr::null_mut();
    thread_unwrap.lock().wwake = core::ptr::null_mut();
    thread_unwrap.lock().wait_data = 0 as *mut c_types::c_void;
    thread_unwrap.lock().u_window = None;
    thread_unwrap.lock().observable = iattr_ptr.observable.clone();

    // thread_lock.rq = Some(rq);
    // thread_lock.state = flags as __u32;
    // pr_debug!("{}", thread_lock.state);
    // thread_lock.info = 0;
    // thread_lock.local_info = 0;
    // thread_lock.wprio = idle::RROS_IDLE_PRIO;
    // thread_lock.cprio = idle::RROS_IDLE_PRIO;
    // thread_lock.bprio = idle::RROS_IDLE_PRIO;
    // thread_lock.rrperiod = RROS_INFINITE;
    // thread_lock.wchan = None;
    // thread_lock.wwake = None;
    // thread_lock.wait_data = 0 as *mut c_types::c_void;
    // thread_lock.u_window = None;
    // thread_lock.observable = iattr_ptr.observable.clone();

    thread
        .clone()
        .unwrap()
        .lock()
        .deref_mut()
        .inband_disable_count
        .atomic_set(0);
    // memset(&thread->PollContext, 0, sizeof(thread->PollContext));
    // memset(&thread->stat, 0, sizeof(thread->stat));
    // memset(&thread->altsched, 0, sizeof(thread->altsched));
    thread
        .clone()
        .unwrap()
        .lock()
        .deref_mut()
        .inband_work
        .init_irq_work(inband_task_wakeup)?;
    // pr_debug!("yinyongcishu is {}", Arc::strong_count(&thread.clone().unwrap()));

    //tp set gps
    // let mut gps = unsafe{RROS_SYSTEM_HEAP.rros_alloc_chunk((size_of::<RrosTpSchedule>() + 4 * size_of::<RrosTpWindow>()) as usize)};
    // if gps == None{
    //     return Err(kernel::Error::ENOMEM);
    // }
    // unsafe{(*thread_unwrap.lock().rq.unwrap()).tp.gps = gps.unwrap() as *mut RrosTpSchedule};

    rros_set_thread_policy(
        thread.clone(),
        iattr_ptr.sched_class.clone(),
        iattr_ptr.sched_param.clone(),
    )?;
    // thread_ptr.base_class = Some(iattr_ptr.unwrap());

    let rtimer = thread_unwrap.lock().rtimer.as_mut().unwrap().clone();
    unsafe {
        timer::rros_init_timer_on_rq(
            rtimer.clone(),
            &mut clock::RROS_MONO_CLOCK,
            Some(timeout_handler),
            rq,
            c_str!("rtimer"),
            timer::RROS_TIMER_IGRAVITY,
        )
    };
    // let thread_addr = &mut thread_unwrap as *mut ;
    rtimer.lock().thread = Some(thread_unwrap.clone());
    let ptimer = thread_unwrap.lock().ptimer.as_mut().unwrap().clone();
    unsafe {
        timer::rros_init_timer_on_rq(
            ptimer.clone(),
            &mut clock::RROS_MONO_CLOCK,
            Some(periodic_handler),
            rq,
            c_str!("ptimer"),
            timer::RROS_TIMER_IGRAVITY,
        )
    };
    ptimer.lock().thread = Some(thread_unwrap.clone());
    pr_info!("rros_init_thread success!");
    Ok(0)
}

pub fn timeout_handler(timer: *mut timer::RrosTimer) {
    let mut t_thread = unsafe { (*timer).thread() };
    let thread = t_thread.as_mut().unwrap().clone();
    //  timer.thread().as_mut().unwrap().clone();
    // let rq = this_rros_rq();
    rros_wakeup_thread(thread, T_DELAY | T_PEND, T_TIMEO as i32);
    // rros_sched_tick(rq);
}

pub fn periodic_handler(timer: *mut timer::RrosTimer) {
    let mut t_thread = unsafe { (*timer).thread() };
    let thread = t_thread.as_mut().unwrap().clone();
    // let rq = this_rros_rq();
    // TODO: adjust all the i32/u32 flags
    rros_wakeup_thread(thread, T_WAIT, T_TIMEO as i32);
    // rros_sched_tick(rq);
}

pub fn rros_wakeup_thread(thread: Arc<SpinLock<RrosThread>>, mask: u32, info: i32) {
    let mut flags: c_types::c_ulong = 0;
    let rq = rros_get_thread_rq(Some(thread.clone()), &mut flags);
    rros_wakeup_thread_locked(thread.clone(), mask, info);
    let _ret = rros_put_thread_rq(Some(thread.clone()), rq, flags);
}

fn rros_wakeup_thread_locked(thread: Arc<SpinLock<RrosThread>>, mut mask: u32, info: i32) {
    let rq = thread.lock().rq;
    // struct rros_rq *rq = thread->rq;
    // unsigned long oldstate;

    // assert_hard_lock(&thread->lock);
    // assert_hard_lock(&thread->rq->lock);

    // if (RROS_WARN_ON(CORE, mask & ~(T_DELAY|T_PEND|T_WAIT)))
    // 	return;

    // trace_rros_wakeup_thread(thread, mask, info);

    let oldstate = thread.lock().state;
    if (oldstate & mask) != 0x0 {
        // 	/* Clear T_DELAY along w/ T_PEND in state. */
        if (mask & T_PEND) != 0x0 {
            mask |= T_DELAY;
        }
        //
        // let mut oldstate = thread.lock();
        thread.lock().state &= !mask;

        if (mask & (T_DELAY | T_PEND)) != 0x0 {
            let rtimer = thread.lock().rtimer.clone();
            timer::rros_stop_timer(rtimer.unwrap());
        }

        // if (mask & T_PEND & oldstate)
        // 	thread.wchan = None;

        thread.lock().info |= info as u32;

        let oldstate = thread.lock().state;
        if (!(oldstate & RROS_THREAD_BLOCK_BITS)) != 0x0 {
            let _ret = rros_enqueue_thread(thread.clone());
            // oldstate |= T_READY;
            thread.lock().state |= T_READY;
            rros_set_resched(rq);
            // if (rq != this_rros_rq()){
            //     rros_inc_counter(&thread->stat.rwa);
            // }
        }
    }
}

fn __rros_run_kthread(kthread: &mut RrosKthread, _clone_flags: i32) -> Result<usize> {
    let thread = kthread.thread.clone().unwrap();
    // struct RrosThread *thread = &kthread->thread;
    // struct task_struct *p;
    // int ret;

    // ret = rros_init_element(&thread->element,
    // 		&rros_thread_factory, clone_flags);
    // if (ret)
    // 	goto fail_element;

    // ret = rros_create_core_element_device(&thread->element,
    // 				&rros_thread_factory,
    // 				thread->name);
    // if (ret)
    // 	goto fail_device;

    let data: *mut c_types::c_void;
    data = kthread as *mut RrosKthread as *mut c_types::c_void;
    // TODO: tmp hack without migrate_threaed
    let p = unsafe {
        kthread_run(
            Some(kthread_trampoline),
            data,
            c_str!("%s").as_char_ptr(),
            format_args!("{}", (*thread.locked_data().get()).name),
        )
    };
    let res = from_kernel_err_ptr(p);
    match res {
        Ok(_o) => (),
        Err(_e) => {
            pr_debug!("your thread creation failed!!!!!!");
            // uninit_thread(
            //     &mut (*(Arc::into_raw(kthread.thread.clone().as_mut().unwrap().clone())
            //         as *mut SpinLock<RrosThread>)),
            // );
            uninit_thread(kthread.thread.clone().unwrap());
            // unsafe{uninit_thread(kthread.thread.clone().unwrap().get_mut());}
            return Err(_e);
        }
    }
    // if  {
    //     pr_debug!("your thread creation failed!!!!!!");
    //     return Err();
    // }
    // p = kthread_run(kthread_trampoline, kthread, "%s", thread->name);
    // if (IS_ERR(p)) {
    // 	ret = PTR_ERR(p);
    // 	goto fail_spawn;
    // }

    // rros_index_factory_element(&thread->element);

    pr_debug!("thread before wait_for_completion");
    kthread.done.wait_for_completion();
    pr_debug!("thread after wait_for_completion ");

    if kthread.status != 0x0 {
        pr_debug!("__rros_run_kthread: kthread.status != 0x0");
        return Err(kernel::Error::EINVAL);
    }

    pr_debug!("thread before release {}", unsafe {
        (*thread.locked_data().get()).state
    });
    rros_release_thread(thread.clone(), T_DORMANT as u32, 0 as u32);
    pr_debug!("thread after release {}", unsafe {
        (*thread.locked_data().get()).state
    });

    unsafe {
        rros_schedule();
    }
    pr_debug!("thread after sched {}", unsafe {
        (*thread.locked_data().get()).state
    });

    Ok(0)
}

const SCHED_NORMAL: i32 = 0;
#[allow(dead_code)]
const SCHED_FIFO: i32 = 0;
const MAX_RT_PRIO: i32 = 100;

unsafe extern "C" fn kthread_trampoline(arg: *mut c_types::c_void) -> c_types::c_int {
    let kthread: &mut RrosKthread;
    // pr_debug!("the thread add is {:p}", arg);
    unsafe {
        let tmp = arg as *mut RrosKthread;
        kthread = &mut *tmp;
    }
    let curr = kthread.thread.as_mut().unwrap().clone();
    let policy;
    let mut prio;
    let sched_class;
    unsafe {
        match (*curr.locked_data().get()).sched_class.clone() {
            Some(c) => sched_class = c,
            None => {
                pr_err!("kthread_trampoline: curr.lock().sched_class.clone err");
                return -1 as c_types::c_int;
            }
        }
    }

    if sched_class.flag != 3 {
        // The scheduling class of curr is not rros_sched_fifo.
        policy = SCHED_NORMAL;
        prio = 0;
    } else {
        policy = SCHED_NORMAL;
        prio = unsafe { (*curr.locked_data().get()).cprio };
        if prio >= MAX_RT_PRIO {
            prio = MAX_RT_PRIO - 1;
        }
    }

    unsafe {
        pr_debug!(
            "kthread_trampoline: state1 in the thread{}",
            (*curr.locked_data().get()).state
        );
    }
    let param = types::SchedParam::new(prio);
    sched_setscheduler(
        Task::current_ptr(),
        policy as c_types::c_int,
        &param as *const types::SchedParam,
    );

    unsafe {
        pr_debug!(
            "kthread_trampoline: state2 in the thread{}",
            (*curr.locked_data().get()).state
        );
    }
    let ret = map_kthread_self(kthread);

    unsafe {
        pr_debug!(
            "kthread_trampoline: state3 in the thread{}",
            (*curr.locked_data().get()).state
        );
    }

    match ret {
        Ok(_o) => {
            // pr_debug!("bug n");
            if let Some(kfn) = kthread.kthread_fn.take() {
                kfn();
            }
        }
        Err(_e) => {}
    }
    unsafe {
        pr_debug!(
            "kthread_trampoline: state4 in the thread{}",
            (*curr.locked_data().get()).state
        );
    }
    rros_cancel_thread(kthread.thread.as_mut().unwrap().clone());
    0 as c_types::c_int
}

fn rros_cancel_thread(thread: Arc<SpinLock<RrosThread>>) {
    pr_debug!(" in rros_cancel_thread");

    let mut flags: c_types::c_ulong = 0;
    // Turn off interrupts.
    let rq = rros_get_thread_rq(Some(thread.clone()), &mut flags);

    let mut state = thread.lock().state;
    let info = thread.lock().info;
    pr_debug!("state is {:?}", thread.lock().state);

    if state & T_ZOMBIE != 0x0 {
        pr_debug!("thread->state: T_ZOMBIE");
        let _ret = rros_put_thread_rq(Some(thread.clone()), rq, flags);
        return;
    }

    if info & T_CANCELD != 0x0 {
        //goto check_self_cancel;
    }

    thread.lock().info |= T_CANCELD;

    state = thread.lock().state;
    if (state & (T_DORMANT | T_INBAND) as u32) == (T_DORMANT | T_INBAND) as u32 {
        rros_release_thread_locked(thread.clone(), T_DORMANT, T_KICKED);
        let _ret = rros_put_thread_rq(Some(thread.clone()), rq, flags);
        unsafe {
            rros_schedule();
        }
        return;
    }

    let _ret = rros_put_thread_rq(Some(thread.clone()), rq, flags);
    let rq = this_rros_rq();
    let curr = unsafe { (*rq).get_curr() };
    let curr_addr = curr.locked_data().get();
    let thread_addr = thread.locked_data().get();
    pr_debug!("curr_addr is {:?}", curr_addr);
    pr_debug!("thread_addr is {:?}", thread_addr);

    if curr_addr == thread_addr {
        pr_debug!("rros_current() == thread");
        rros_test_cancel();
        return;
    }

    state = unsafe { (*thread.locked_data().get()).state };
    if state & T_USER != 0x0 {
        //rros_demote_thread(thread);
        // rros_signal_thread(thread, SIGTERM, 0);
    } else {
        pr_debug!("rros_kick_thread: no");
        //rros_kick_thread(thread, 0);
    }
    unsafe {
        rros_schedule();
    }
}

pub fn rros_test_cancel() {
    let curr_ptr = rros_current();
    if curr_ptr != 0 as *mut SpinLock<RrosThread> {
        let curr = unsafe { &mut *curr_ptr };
        let info = unsafe { (*curr.locked_data().get()).info };
        if info & T_CANCELD != 0x0 {
            pr_debug!("rros_test_cancel: yes");
            __rros_test_cancel(curr);
        }
    }
}

fn __rros_test_cancel(curr_ptr: *mut SpinLock<RrosThread>) {
    let curr = unsafe { &mut *curr_ptr };
    let rq_local_flags =
        unsafe { (&*((*curr.locked_data().get()).rq.clone().unwrap())).local_flags };
    if rq_local_flags & RQ_IRQ != 0x0 {
        return;
    }
    let state = unsafe { (*curr.locked_data().get()).state };
    if state & T_INBAND == 0x0 {
        pr_debug!("__rros_test_cancel:!(curr->state & T_INBAND)");
        rros_switch_inband(RROS_HMDIAG_NONE as i32);
    }
    kernelh::do_exit(kernelh::ThreadExitCode::Successfully);
}

unsafe extern "C" fn wakeup_kthread_parent(irq_work: *mut IrqWork) {
    let kthread = kernel::container_of!(irq_work, RrosKthread, irq_work);
    unsafe {
        (*(kthread as *mut RrosKthread)).done.complete();
    }
}

pub fn pin_to_initial_cpu(thread: Arc<SpinLock<RrosThread>>) {
    let current_ptr = task::Task::current_ptr();

    let mut cpu: u32 = task::Task::task_cpu(current_ptr as *const _);
    if unsafe { (*thread.locked_data().get()).affinity.cpumask_test_cpu(cpu) } == false {
        cpu = unsafe { (*thread.locked_data().get()).affinity.cpumask_first() as u32 };
    }

    let name = unsafe { (*thread.locked_data().get()).name };
    pr_debug!("[smp_test]: thread name is {:?}, cpu is {:?}", name, cpu);

    unsafe {
        bindings::set_cpus_allowed_ptr(current_ptr, CpumaskT::cpumask_of(cpu) as *const _);
    }

    let rq = rros_cpu_rq(cpu as i32);
    let flags: u64 = unsafe { (*thread.locked_data().get()).lock.raw_spin_lock_irqsave() };
    rros_migrate_thread(thread.clone(), rq);
    unsafe {
        (*thread.locked_data().get())
            .lock
            .raw_spin_unlock_irqrestore(flags);
    }
}

fn map_kthread_self(kthread: &mut RrosKthread) -> Result<usize> {
    let thread = kthread.thread.clone().unwrap();

    pr_debug!("map_kthread_self:in");

    pin_to_initial_cpu(thread.clone());

    let ret;
    unsafe {
        pr_debug!(
            "map_kthread_self: the altched add is {:p}",
            &mut (*thread.locked_data().get()).altsched
        );
        (*thread.locked_data().get())
            .altsched
            .dovetail_init_altsched();
        pr_debug!("map_kthread_self: after dovetail_init_altsched");

        set_oob_threadinfo(
            Arc::into_raw(thread.clone()) as *mut SpinLock<RrosThread> as *mut c_types::c_void
        );
        pr_debug!(
            "map_kthread_self rros_current address is {:p}",
            rros_current()
        );
        dovetail::dovetail_start_altsched();

        rros_release_thread(thread.clone(), T_DORMANT as u32, 0 as u32);

        // ret = rros_switch_oob(kthread);
        ret = rros_switch_oob();
        if let Err(_e) = ret {
            kthread.status = -1;
        }
        pr_debug!("map_kthread_self: after rros_switch_oob");
        //b kernel/rros/thread.rs:531
        kthread.irq_work.init_irq_work(wakeup_kthread_parent)?;
        kthread.irq_work.irq_work_queue()?;
    }
    // enqueue_new_thread(curr);// This function has little impact.
    rros_hold_thread(kthread, T_DORMANT);
    return ret;
}

pub fn rros_current() -> *mut SpinLock<RrosThread> {
    dovetail::dovetail_current_state().thread() as *mut SpinLock<RrosThread>
}

pub fn rros_switch_oob() -> Result<usize> {
    // fn rros_switch_oob(kthread: &mut RrosKthread) -> Result<usize> {
    let _res = premmpt::running_inband()?;
    // pr_debug!("res premmpt {:?}" , res);
    // struct RrosThread *curr = rros_current();

    // let prio = kthread.thread.as_mut().unwrap().lock().cprio;

    pr_debug!("rros_switch_oob: 1");
    let _curr = unsafe { &mut *rros_current() };
    // pr_debug!("curr state {}" , curr.lock().state);
    // pr_debug!("kthread state {}" , kthread.thread.as_mut().unwrap().lock().state);

    // let prio = kthread.thread.as_mut().unwrap().lock().cprio;
    pr_debug!("rros_switch_oob: 2");

    // struct task_struct *p = current;
    // unsigned long flags;
    // int ret;

    // inband_context_only();

    // if (curr == NULL)
    // 	return -EPERM;

    if Task::current().signal_pending() {
        // pr_debug!("wrong!!!!!!!!!!!!!!!!!!!");
        return Err(kernel::Error::ERESTARTSYS);
    }

    // let prio = kthread.thread.as_mut().unwrap().lock().cprio;
    // pr_debug!("mutex 3.25");

    // if (signal_pending(p))
    // 	return -ERESTARTSYS;

    // trace_rros_switch_oob(curr);b kernel/rros/thread.rs:504
    //b kernel/rros/thread.rs:604

    // rros_clear_sync_uwindow(curr, T_INBAND);

    let ret = dovetail::dovetail_leave_inband();
    // pr_debug!("2dddddddddddddddddddddddddddddddddddddddddddeee ");
    if ret != 0x0 {
        rros_test_cancel();
        // 	rros_set_sync_uwindow(curr, T_INBAND);
        return Err(kernel::Error::EINVAL);
    }

    // /*
    //  * On success, dovetail_leave_inband() stalls the oob stage
    //  * before returning to us: clear this stall bit since we don't
    //  * use it for protection but keep hard irqs off.
    //  */
    unsafe {
        rust_helper_unstall_oob();
    }
    // let prio = kthread.thread.as_mut().unwrap().lock().cprio;
    // pr_debug!("mutex 3.4");

    // /*
    //  * The current task is now running on the out-of-band
    //  * execution stage, scheduled in by the latest call to
    //  * __rros_schedule() on this CPU: we must be holding the
    //  * runqueue lock and hard irqs must be off.
    //  */
    // oob_context_only();

    // finish_rq_switch_from_inband();
    unsafe {
        rust_helper_hard_local_irq_enable();
        // rust_helper_preempt_enable();
    }
    // unsafe{bindings::oob_irq_enable();}
    // unsafe{
    //    let rq = this_rros_rq();
    //     tick::rros_notify_proxy_tick(rq);
    // }

    // unsafe{
    //     let mut tmb = timer::rros_this_cpu_timers(&clock::RROS_MONO_CLOCK);
    //     if (*tmb).q.is_empty() == true {
    //         // tick
    //         tick::proxy_set_next_ktime(1000000, 0 as *mut bindings::clock_event_device);
    //     }
    // }

    // trace_rros_switched_oob(curr);

    // /*
    //  * In case check_cpu_affinity() caught us resuming oob from a
    //  * wrong CPU (i.e. outside of the oob set), we have T_CANCELD
    //  * set. Check and bail out if so.
    //  */
    // if (curr->info & T_CANCELD)
    // 	rros_test_cancel();

    // /*
    //  * Since handle_sigwake_event()->rros_kick_thread() won't set
    //  * T_KICKED unless T_INBAND is cleared, a signal received
    //  * during the stage transition process might have gone
    //  * unnoticed. Recheck for signals here and raise T_KICKED if
    //  * some are pending, so that we switch back in-band asap for
    //  * handling them.
    //  */
    // if (signal_pending(p)) {
    // 	raw_spin_lock_irqsave(&curr->rq->lock, flags);
    // 	curr->info |= T_KICKED;
    // 	raw_spin_unlock_irqrestore(&curr->rq->lock, flags);
    // }

    if Task::current().signal_pending() {
        // pr_debug!("wrong!!!!!!!!!!!!!!!!!!!");
        return Err(kernel::Error::ERESTARTSYS);
    }

    // return 0;
    Ok(0)
}

// pub fn finish_rq_switch_from_inband() {
//     bindings::_raw_spin_unlock_irq()
// }

pub fn rros_release_thread(thread: Arc<SpinLock<RrosThread>>, mask: u32, info: u32) {
    let mut flags: c_types::c_ulong = 0;
    let rq = rros_get_thread_rq(Some(thread.clone()), &mut flags);
    rros_release_thread_locked(thread.clone(), mask, info); // For smp, this needs to be changed, but now there is no problem.
    let _ret = rros_put_thread_rq(Some(thread.clone()), rq, flags);
}

pub fn rros_release_thread_locked(thread: Arc<SpinLock<RrosThread>>, mask: u32, info: u32) {
    let rq = thread.lock().rq.unwrap();
    let oldstate = thread.lock().state;
    // if (RROS_WARN_ON(CORE, mask & ~(T_SUSP|T_HALT|T_INBAND|T_DORMANT|T_PTSYNC)))
    // return;
    if (oldstate & mask) != 0x0 {
        thread.lock().state &= !mask;
        thread.lock().info |= info;
        if (thread.lock().state & RROS_THREAD_BLOCK_BITS) != 0x0 {
            return;
        }

        if ((oldstate & mask) & (T_HALT | T_PTSYNC)) != 0x0 {
            let _ret = rros_requeue_thread(thread.clone());
            thread.lock().state |= T_READY;
            rros_set_resched(Some(rq));
            if rq != this_rros_rq() {
                thread.lock().stat.rwa.inc_counter();
            }
            return;
        }
    } else if (oldstate & T_READY) != 0x0 {
        let _ret = rros_dequeue_thread(thread.clone());
    }
    let _ret = rros_enqueue_thread(thread.clone());
    thread.lock().state |= T_READY;
    rros_set_resched(Some(rq));
    if rq != this_rros_rq() {
        thread.lock().stat.rwa.inc_counter();
    }
}

pub fn rros_hold_thread(kthread: &mut RrosKthread, mask: u32) {
    // rros_hold_thread(kthread.thread.clone().unwrap(), T_DORMANT);
    // as_mut().unwrap()
    let thread = kthread.thread.clone().unwrap();
    let mut flags: c_types::c_ulong = 0;

    // Turn off interrupts.
    let rq_op = rros_get_thread_rq(Some(thread.clone()), &mut flags);

    let rq = rq_op.unwrap();
    let oldstate = thread.lock().state;
    let curr = unsafe { (*rq).get_curr() };
    let curr_add = curr.clone().lock().deref_mut() as *mut RrosThread;
    let thread_add = thread.clone().lock().deref_mut() as *mut RrosThread;
    if oldstate & RROS_THREAD_BLOCK_BITS == 0x0 {
        let info = thread.lock().info;
        if info & T_KICKED != 0x0 {
            thread.lock().info &= !(T_RMID | T_TIMEO);
            thread.lock().info |= T_BREAK;
            let _ret = rros_put_thread_rq(Some(thread.clone()), Some(rq), flags);
            return;
        }
        if thread_add == curr_add {
            thread.lock().info &= !RROS_THREAD_INFO_MASK; //fix!!
        }
    }

    if oldstate & T_READY != 0x0 {
        let _ret = rros_dequeue_thread(thread.clone());
        thread.lock().state &= !T_READY;
    }

    thread.lock().state |= mask;

    if thread_add == curr_add {
        rros_set_resched(Some(rq));
    } else if oldstate & (RROS_THREAD_BLOCK_BITS | T_USER) == (T_INBAND as u32 | T_USER) {
        // dovetail_request_ucall(thread->altsched.task);
        todo!();
    }
    let _ret = rros_put_thread_rq(Some(thread.clone()), Some(rq), flags);
}

pub fn set_oob_threadinfo(curr: *mut c_types::c_void) {
    // pr_debug!("oob thread info {:p}", curr);
    // unsafe{(*Task::current_ptr()).thread_info.oob_state.thread = curr }
    pr_debug!("set_oob_threadinfo: in");
    dovetail::dovetail_current_state().set_thread(curr);
    // unsafe{Arc::decrement_strong_count(curr);}
}

pub fn set_oob_mminfo(thread: Arc<SpinLock<RrosThread>>) {
    // pr_debug!("set_oob_mminfo: in");
    unsafe {
        (*thread.locked_data().get()).oob_mm = dovetail::dovetail_mm_state();
    }
}

pub fn kthread_run(
    threadfn: Option<unsafe extern "C" fn(*mut c_types::c_void) -> c_types::c_int>,
    data: *mut c_types::c_void,
    namefmt: *const c_types::c_char,
    msg: fmt::Arguments<'_>,
) -> *mut c_types::c_void {
    unsafe {
        rust_helper_kthread_run(
            threadfn,
            data,
            namefmt,
            &msg as *const _ as *const c_types::c_void,
        )
    }
}

unsafe extern "C" fn inband_task_wakeup(work: *mut IrqWork) {
    unsafe {
        let p = kernel::container_of!(work, RrosThread, inband_work);
        task::Task::wake_up_process((*p).altsched.0.task);
    }
}

pub fn rros_run_kthread(kthread: &mut RrosKthread, fmt: &'static CStr) -> Result<usize> {
    let mut iattr = RrosInitThreadAttr::new();
    // iattr.flags = T_USER as i32;
    unsafe {
        iattr.affinity = &RROS_OOB_CPUS as *const CpumaskT;
        iattr.sched_class = Some(&RROS_SCHED_FIFO);
        // iattr.sched_class = Some(&RROS_SCHED_TP);
        let prio = 98;
        let sched_param = Arc::try_new(SpinLock::new(RrosSchedParam::new()))?;
        (*sched_param.locked_data().get()).fifo.prio = prio;
        (*sched_param.locked_data().get()).idle.prio = prio;
        (*sched_param.locked_data().get()).weak.prio = prio;
        (*sched_param.locked_data().get()).tp.prio = 1;
        iattr.sched_param = Some(sched_param);
        kthread.done.init_completion();

        // kthread.borrow_mut().thread.clone().unwrap().lock().state |= sched::T_READY;
        // pr_debug!("fkkkkk {}", kthread.thread.clone().unwrap().lock().state);

        let thread_unwrap = kthread.thread.clone().unwrap();
        pr_debug!("here 2");
        rros_init_thread(&Some(thread_unwrap), iattr, this_rros_rq(), fmt)?;
        let _next_add = kthread.thread.clone().unwrap().lock().deref_mut() as *mut RrosThread;
        // pr_debug!("the run thread add is  next_add {:p}", next_add);
        // pr_debug!("fkkkkk 176 128 32 16 {}", kthread.thread.clone().unwrap().lock().state);
        // kthread.thread.clone().unwrap().lock().rrperiod = 400000;
        // kthread.thread.clone().unwrap().lock().state |= T_RRB;
        // let rq = unsafe{(*this_rros_rq()).get_rrbtimer()};
        // let value = unsafe{rros_abs_timeout((*rq).get_rrbtimer(), 4000)};
        // let interval = RROS_INFINITE;
        // rros_start_timer(rq, value, interval);

        // pr_debug!("kkkkkk {}", kthread.thread.clone().unwrap().lock().state);

        __rros_run_kthread(kthread, factory::RROS_CLONE_PUBLIC)?;

        // pr_debug!("kkkkkk2 2224 2048 128 32 16 {}", kthread.borrow_mut().thread.clone().unwrap().lock().state);
    }
    Ok(0)
}

#[allow(dead_code)]
pub fn rros_set_thread_schedparam(
    thread: Arc<SpinLock<RrosThread>>,
    sched_class: Option<&'static RrosSchedClass>,

    sched_param: Option<Arc<SpinLock<RrosSchedParam>>>,
) -> Result<usize> {
    let mut flags: c_types::c_ulong = 0;
    let rq = rros_get_thread_rq(Some(thread.clone()), &mut flags);
    rros_set_thread_schedparam_locked(thread.clone(), sched_class, sched_param.clone())?;
    rros_put_thread_rq(Some(thread.clone()), rq, flags)?;
    Ok(0)
}

pub fn rros_set_thread_schedparam_locked(
    thread: Arc<SpinLock<RrosThread>>,
    _sched_class: Option<&'static RrosSchedClass>,
    _sched_param: Option<Arc<SpinLock<RrosSchedParam>>>,
) -> Result<usize> {
    let old_wprio: i32;
    let new_wprio: i32;

    old_wprio = thread.lock().wprio;
    new_wprio = thread.lock().wprio;

    let state = thread.lock().state;
    if old_wprio != new_wprio && (state & T_PEND) != 0 {
        let func;
        unsafe {
            // let wchan = (*thread.locked_data().get()).wchan.clone().unwrap();
            match (*(*thread.locked_data().get()).wchan).reorder_wait {
                Some(f) => func = f,
                None => {
                    pr_warn!("reorder_wait function error");
                    return Err(kernel::Error::EINVAL);
                }
            }
        }
        func(thread.clone(), thread.clone())?;
    }
    thread.lock().info |= T_SCHEDP;

    let state = thread.lock().state;
    if (state & (T_INBAND as u32 | T_USER)) == (T_INBAND as u32 | T_USER) {
        dovetail::dovetail_request_ucall(thread.lock().altsched.0.task);
    }
    Ok(0)
}

#[allow(dead_code)]
pub fn rros_sleep(delay: ktime::KtimeT) -> Result<usize> {
    let end: ktime::KtimeT =
        unsafe { ktime::ktime_add(clock::rros_read_clock(&clock::RROS_MONO_CLOCK), delay) };
    rros_sleep_until(end)?;
    Ok(0)
}

#[allow(dead_code)]
fn rros_sleep_until(timeout: ktime::KtimeT) -> Result<usize> {
    let res = unsafe {
        rros_delay(
            timeout,
            timeout::RrosTmode::RrosAbs,
            &clock::RROS_MONO_CLOCK,
        )
    };
    match res {
        Ok(_o) => Ok(0),
        Err(_e) => Err(kernel::Error::EINVAL),
    }
}

pub fn rros_delay(
    timeout: ktime::KtimeT,
    timeout_mode: timeout::RrosTmode,
    clock: &clock::RrosClock,
) -> Result<usize> {
    // let curr = rros_current();
    // struct RrosThread *curr = rros_current();

    rros_sleep_on(timeout, timeout_mode, clock, 0 as *mut RrosWaitChannel);
    unsafe {
        rros_schedule();
    }

    // FIXME: add this function if the net schedule does not work
    // if (curr->info & T_BREAK)
    // rem = __rros_get_stopped_timer_delta(&curr->rtimer);

    Ok(0)
}

pub fn rros_sleep_on(
    timeout: ktime::KtimeT,
    timeout_mode: timeout::RrosTmode,
    clock: &clock::RrosClock,
    wchan: *mut RrosWaitChannel,
) {
    let mut flags: c_types::c_ulong = 0;
    let curr = unsafe { &mut *rros_current() };
    let thread = unsafe { Arc::from_raw(curr as *const SpinLock<RrosThread>) };
    // pr_debug!("rros_sleep_on: x ref is {}", Arc::strong_count(&thread.clone()));
    unsafe {
        Arc::increment_strong_count(curr);
    }
    let rq = rros_get_thread_rq(Some(thread.clone()), &mut flags);
    let _ret = rros_sleep_on_locked(timeout, timeout_mode, clock, wchan);
    let _ret = rros_put_thread_rq(Some(thread.clone()), rq, flags);
}

pub fn rros_sleep_on_locked(
    timeout: ktime::KtimeT,
    timeout_mode: timeout::RrosTmode,
    clock: &clock::RrosClock,
    wchan: *mut RrosWaitChannel,
) -> Result<u64> {
    let rq = this_rros_rq();
    let curr = unsafe { (*rq).get_curr() };

    let _next_add = curr.clone().lock().deref_mut() as *mut RrosThread;
    // pr_debug!("the tram thread add {:p}", next_add);

    // /* Sleeping while preemption is disabled is a bug. */
    // RROS_WARN_ON(CORE, rros_preempt_count() != 0);

    // assert_hard_lock(&curr->lock);
    // assert_hard_lock(&rq->lock);

    // trace_rros_sleep_on(timeout, timeout_mode, clock, wchan);

    let oldstate = curr.lock().state;

    // pr_debug!("thread before0 sleep {}", oldstate);
    // /*
    //  * If a request to switch to in-band context is pending
    //  * (T_KICKED), raise T_BREAK then return immediately.
    //  */
    if oldstate & RROS_THREAD_BLOCK_BITS != 0x0 {
        if (curr.lock().info & T_KICKED) != 0x0 {
            curr.lock().info &= !(T_RMID | T_TIMEO) as u32;
            curr.lock().info |= T_BREAK as u32;
            return Ok(0);
        }
        curr.lock().info &= !RROS_THREAD_BLOCK_BITS;
    }

    // /*
    //  *  wchan + timeout: timed wait for a resource (T_PEND|T_DELAY)
    //  *  wchan + !timeout: unbounded sleep on resource (T_PEND)
    //  * !wchan + timeout: timed sleep (T_DELAY)
    //  * !wchan + !timeout: periodic wait (T_WAIT)
    //  */
    // curr.lock().state |= sched::T_DELAY as u32;
    // pr_debug!("thread before sleep {}", curr.lock().state);

    if timeout_mode != timeout::RrosTmode::RrosRel || !timeout::timeout_infinite(timeout) {
        // timer::rros_prepare_timed_wait(curr.lock().rtimer, clock,
        // rros_thread_rq(curr));
        if timeout_mode == timeout::RrosTmode::RrosRel {
            // timeout = rros_abs_timeout(&curr->rtimer, timeout);
        } else if timeout <= clock::rros_read_clock(clock) {
            //
            curr.lock().info |= T_TIMEO as u32;
            return Ok(0);
        }
        let timer = curr.lock().rtimer.as_ref().unwrap().clone();
        timer::rros_start_timer(timer.clone(), timeout, timeout::RROS_INFINITE);
        curr.lock().state |= T_DELAY as u32;
        // pr_debug!("thread 2before sleep {}", curr.lock().state);
    } else if wchan == 0 as *mut RrosWaitChannel {
        // rros_prepare_timed_wait(&curr->ptimer, clock,rros_thread_rq(curr));
        curr.lock().state |= T_WAIT;
    }

    if (oldstate & T_READY) != 0x0 {
        rros_dequeue_thread(curr.clone())?;
        curr.lock().state &= !T_READY;
    }

    if !wchan.is_null() {
        unsafe {
            (*curr.locked_data().get()).wchan = wchan as *mut RrosWaitChannel;
            (*curr.locked_data().get()).state |= T_PEND;
        }
    }

    rros_set_resched(Some(rq));
    Ok(0)
}

#[allow(dead_code)]
pub fn rros_propagate_schedparam_change(curr: &mut SpinLock<RrosThread>) {
    // Can't write here: curr_lock = curr.lock()
    // cannot borrow `*curr` as mutable because it is also borrowed as immutable
    if curr.lock().info & T_SCHEDP != 0x0 {
        __rros_propagate_schedparam_change(curr);
    }
}

pub fn __rros_propagate_schedparam_change(_curr: &mut SpinLock<RrosThread>) {
    //todo
}

#[no_mangle]
unsafe extern "C" fn rust_handle_inband_event(event: u32, _data: *mut c_types::c_void) {
    match event {
        // case INBAND_TASK_SIGNAL:
        // 	handle_sigwake_event(data);
        // 	break;
        bindings::inband_event_type_INBAND_TASK_EXIT => {
            // pr_debug!("{}",rust_helper_dovetail_current_state().subscriber);
            // rros_drop_subscriptions(rros_get_subscriber()); // sbr in rros is NULL, comment here first.
            if rros_current() != 0 as *mut SpinLock<RrosThread> {
                let _ret = put_current_thread();
            }
        } // case INBAND_TASK_MIGRATION:
        // 	handle_migration_event(data);
        // 	break;
        // case INBAND_TASK_RETUSER:
        // 	handle_retuser_event();
        // 	break;
        // case INBAND_TASK_PTSTOP:
        // 	handle_ptstop_event();
        // 	break;
        // case INBAND_TASK_PTCONT:
        // 	handle_ptcont_event();
        // 	break;
        // case INBAND_TASK_PTSTEP:
        // 	handle_ptstep_event(data);
        // 	break;
        // case INBAND_PROCESS_CLEANUP:
        // 	handle_cleanup_event(data);
        // 	break;
        _ => {
            pr_warn!("unknown inband event");
        }
    }
}

fn put_current_thread() -> Result<usize> {
    let curr = unsafe { &mut *rros_current() };

    let state = curr.lock().state;
    if state & T_USER != 0 {
        pr_debug!("000000000000000000000000000000000000000000000000000000");
        // 	skip_ptsync(curr);
    }
    cleanup_current_thread()?;
    // rros_put_element(&curr->element);
    // unsafe{pr_debug!("600 uninit_thread: x ref is {}", Arc::strong_count(UTHREAD.as_ref().unwrap()));}
    unsafe {
        Arc::decrement_strong_count(curr);
    }
    unsafe {
        UTHREAD = None;
    }
    Ok(0)
}

fn cleanup_current_thread() -> Result<usize> {
    // unsafe{pr_debug!("00 uninit_thread: x ref is {}", Arc::strong_count(&uthread.clone().unwrap()));}
    let curr = rros_current();
    let thread = unsafe { Arc::from_raw(curr as *const SpinLock<RrosThread>) };
    unsafe {
        Arc::increment_strong_count(curr);
    }
    // unsafe{pr_debug!("01 uninit_thread: x ref is {}", Arc::strong_count(&uthread.clone().unwrap()));}
    // trace_rros_thread_unmap(curr);
    dovetail::dovetail_stop_altsched();
    do_cleanup_current(thread)?;
    // unsafe{pr_debug!("02 uninit_thread: x ref is {}", Arc::strong_count(&uthread.clone().unwrap()));}

    dovetail::dovetail_current_state().set_thread(0 as *mut c_types::c_void);
    // unsafe{Arc::decrement_strong_count(curr);}
    // unsafe{pr_debug!("090 uninit_thread: x ref is {}", Arc::strong_count(&UTHREAD.clone().unwrap()));}
    // unsafe{pr_debug!("60 uninit_thread: x ref is {}", Arc::strong_count(&UTHREAD.clone().unwrap()));}
    Ok(0)
}

fn do_cleanup_current(curr: Arc<SpinLock<RrosThread>>) -> Result<usize> {
    // fn do_cleanup_current(curr: &mut SpinLock<RrosThread>) -> Result<usize> {
    // struct cred *newcap;
    // unsafe{pr_debug!("03 uninit_thread: x ref is {}", Arc::strong_count(&UTHREAD.clone().unwrap()));}
    let mut flags: c_types::c_ulong = 0;
    let rq: Option<*mut rros_rq>;

    // The if here cannot be entered temporarily, and the observable is empty.
    // let observable = curr.lock().observable.clone();
    // if observable.is_some(){
    // rros_flush_observable(curr->observable);
    // }

    // rros_drop_tracking_mutexes(curr);

    // rros_unindex_factory_element(&curr->element);
    let state = curr.lock().state;
    if state & T_USER != 0 {
        pr_debug!("000000000000000000000000000000000000000000000000000000");
        // 	rros_free_chunk(&RROS_SHARED_HEAP, curr->u_window);
        // 	curr->u_window = NULL;
        // 	rros_drop_poll_table(curr);
        // 	newcap = prepare_creds();
        // 	if (newcap) {
        // 		drop_u_cap(curr, newcap, CAP_SYS_NICE);
        // 		drop_u_cap(curr, newcap, CAP_IPC_LOCK);
        // 		drop_u_cap(curr, newcap, CAP_SYS_RAWIO);
        // 		commit_creds(newcap);
        // 	}
    }

    pr_debug!("before dequeue_old_thread");
    // dequeue_old_thread(curr);
    pr_debug!("after dequeue_old_thread,before rros_get_thread_rq");
    // let x = unsafe { Arc::from_raw(curr as *const SpinLock<RrosThread>) };
    // unsafe{pr_debug!("b 6 uninit_thread: x ref is {}", Arc::strong_count(&UTHREAD.clone().unwrap()));}
    let x = curr.clone();
    // unsafe{pr_debug!("c 6 uninit_thread: x ref is {}", Arc::strong_count(&UTHREAD.clone().unwrap()));}
    rq = rros_get_thread_rq(Some(x.clone()), &mut flags);
    pr_debug!("after rros_get_thread_rq1111111111");
    let state = curr.lock().state;
    if state & T_READY != 0 {
        pr_debug!("before rros_dequeue_thread in thread.rs");
        // RROS_WARN_ON(CORE, (curr->state & RROS_THREAD_BLOCK_BITS));
        // unsafe { rros_dequeue_thread(Arc::from_raw(curr as *const SpinLock<RrosThread>)) };
        rros_dequeue_thread(x.clone())?;
        curr.lock().state &= !T_READY;
    }

    (*curr).lock().state |= T_ZOMBIE;
    pr_debug!("before uninit_thread");
    rros_put_thread_rq(Some(x.clone()), rq, flags)?;
    // unsafe{pr_debug!("a 6 uninit_thread: x ref is {}", Arc::strong_count(&UTHREAD.clone().unwrap()));}
    uninit_thread(curr.clone());
    // unsafe{pr_debug!("9090 uninit_thread: x ref is {}", Arc::strong_count(&UTHREAD.clone().unwrap()));}
    pr_debug!("after uninit_thread");
    Ok(0)
}

#[allow(dead_code)]
fn dequeue_old_thread(_thread: Arc<SpinLock<RrosThread>>) -> Result<usize> {
    // fn dequeue_old_thread(thread: &mut SpinLock<RrosThread>) -> Result<usize> {
    let flags = lock::hard_local_irq_save();
    // kernel corrupted bug is here: next is 0 at initialization, but uses * to get its value
    // let next = unsafe{&mut *thread.lock().next};
    // next.remove();

    // 	rros_nrthreads--;
    lock::hard_local_irq_restore(flags);
    Ok(0)
}

fn uninit_thread(thread: Arc<SpinLock<RrosThread>>) {
    pr_debug!("the thread address is {:p}", thread);
    // pr_debug!("the UTHREAD address is {:p}", UTHREAD.clone().unwrap());
    // unsafe{pr_debug!("d 6 uninit_thread: x ref is {}", Arc::strong_count(&UTHREAD.clone().unwrap()));}
    let mut flags: c_types::c_ulong = 0;
    let rq: Option<*mut rros_rq>;
    let rtimer = thread.lock().rtimer.clone();
    let ptimer = thread.lock().ptimer.clone();
    // unsafe{pr_debug!("7 uninit_thread: x ref is {}", Arc::strong_count(&UTHREAD.clone().unwrap()));}
    timer::rros_destroy_timer(rtimer.clone().unwrap());
    // unsafe{pr_debug!("8 uninit_thread: x ref is {}", Arc::strong_count(&UTHREAD.clone().unwrap()));}
    // thread.lock().rtimer = None;
    rtimer.as_ref().unwrap().lock().thread = None;
    timer::rros_destroy_timer(ptimer.unwrap());
    // unsafe{pr_debug!("9 uninit_thread: x ref is {}", Arc::strong_count(&UTHREAD.clone().unwrap()));}
    // let x = unsafe { Arc::from_raw(thread as *const SpinLock<RrosThread>) };
    let x = thread.clone();
    // unsafe{pr_debug!("10 uninit_thread: x ref is {}", Arc::strong_count(&UTHREAD.clone().unwrap()));}
    rq = rros_get_thread_rq(Some(x.clone()), &mut flags);
    // unsafe{pr_debug!("11 uninit_thread: x ref is {}", Arc::strong_count(&UTHREAD.clone().unwrap()));}
    pr_debug!("uninit_thread: x ref is {}", Arc::strong_count(&x.clone()));
    // unsafe{pr_debug!("12 uninit_thread: x ref is {}", Arc::strong_count(&UTHREAD.clone().unwrap()));}
    // unsafe { rros_forget_thread(Arc::from_raw(thread as *const SpinLock<RrosThread>)) };
    let _ret = rros_forget_thread(x.clone());
    // unsafe{pr_debug!("13 uninit_thread: x ref is {}", Arc::strong_count(&UTHREAD.clone().unwrap()));}
    let _ret = rros_put_thread_rq(Some(x.clone()), rq, flags);
    // unsafe{pr_debug!("14 uninit_thread: x ref is {}", Arc::strong_count(&UTHREAD.clone().unwrap()));}

    // let name = thread.lock().name as *const c_types::c_void;
    // bindings::kfree(name);
}

pub static mut UTHREAD: Option<Arc<SpinLock<RrosThread>>> = None;

// TODO: update the __user attribute and modify the `*mut` type
fn thread_factory_build(
    fac: &'static mut SpinLock<RrosFactory>,
    uname: &'static CStr,
    _u_attrs: Option<*mut u8>,
    clone_flags: i32,
    _state_offp: &u32,
) -> Rc<RefCell<RrosElement>> {
    // static struct rros_element *
    // thread_factory_build(struct rros_factory *fac, const char __user *u_name,
    // 		void __user *u_attrs, int clone_flags, u32 *state_offp)
    // {
    // let observalbe;
    // 	struct rros_observable *observable = NULL;
    let tsk = Task::current_ptr();
    // 	struct task_struct *tsk = current;
    let mut iattr = RrosInitThreadAttr::new();
    // 	struct rros_init_thread_attr iattr;
    // 	struct RrosThread *curr;
    // 	int ret;

    if rros_current() != 0 as *mut SpinLock<RrosThread> {
        pr_warn!("this condition should not be met!!!!!!!!!");
        // return Err(Error::EBUSY);
    }

    // 	if (rros_current())
    // 		return ERR_PTR(-EBUSY);

    if clone_flags & !factory::RROS_THREAD_CLONE_FLAGS != 0 {
        pr_warn!("I'm in this function this condition should not be met!!!!!!!!!");
        // return Err(Error::EINVAL);
    }
    // 	if (clone_flags & ~RROS_THREAD_CLONE_FLAGS)
    // 		return ERR_PTR(-EINVAL);

    // 	/* @current must open the control device first. */
    // TODO: check the flags
    // let flags = unsafe{rust_helper_doveail_mm_state().flags};
    // if !test_bit(RROS_MM_ACTIVE_BIT, &flags) {
    //     return Err(Error::EPERM);
    // }
    // 	if (!test_bit(RROS_MM_ACTIVE_BIT, &dovetail_mm_state()->flags))
    // 		return ERR_PTR(-EPERM);

    // TODO: update all the unwrap() to return error

    let curr;

    unsafe {
        // KTHREAD_RUNNER_1 = Some(KthreadRunner::new(kfn_1));
        let mut thread = SpinLock::new(RrosThread::new().unwrap());
        pr_debug!("at the thread build thread address is {:p} ", &thread);
        let pinned = Pin::new_unchecked(&mut thread);
        // pr_debug!("at the thread build thread address is {:p} ", &thread);
        spinlock_init!(pinned, "test_threads1");
        curr = Arc::try_new(thread).unwrap();
        // pr_debug!("at the thread build thread address is {:p} ", &thread);

        let mut r = SpinLock::new(timer::RrosTimer::new(1));
        let pinned_r = Pin::new_unchecked(&mut r);
        spinlock_init!(pinned_r, "rtimer_1");

        let mut p = SpinLock::new(timer::RrosTimer::new(1));
        let pinned_p = Pin::new_unchecked(&mut p);
        spinlock_init!(pinned_p, "ptimer_1");

        curr.lock().rtimer = Some(Arc::try_new(r).unwrap());
        curr.lock().ptimer = Some(Arc::try_new(p).unwrap());
    }
    pr_debug!("at the thread build curr is {:p} ", curr);
    unsafe {
        UTHREAD = Some(curr.clone());
    }
    pr_debug!("at the thread build curr is {:p} ", curr);
    unsafe {
        pr_debug!(
            "at the thread build curr is {:p} ",
            UTHREAD.as_ref().unwrap()
        )
    };
    unsafe {
        pr_debug!(
            "at the thread build curr is {:p} ",
            UTHREAD.clone().unwrap()
        )
    };
    // 	curr = kzalloc(sizeof(*curr), GFP_KERNEL);
    // 	if (curr == NULL)
    // 		return ERR_PTR(-ENOMEM);

    // TODO: add the rros_init_user_element
    let _ret = rros_init_user_element(curr.lock().element.clone(), fac, uname, clone_flags);
    // let mut ret = rros_init_element(e, fac, clone_flags);
    // 	ret = rros_init_user_element(&curr->element, &rros_thread_factory,
    // 				u_name, clone_flags);
    // 	if (ret)
    // 		goto fail_element;

    iattr.flags = T_USER as i32;
    // 	iattr.flags = T_USER;

    if clone_flags & factory::RROS_CLONE_OBSERVABLE != 0 {
        pr_warn!("This should not happen for now!!!!!!!")
        // observalbe = RrosObservable::new().unwrap();
    } else if clone_flags & factory::RROS_CLONE_MASTER != 0 {
        pr_warn!("This should not happen for now!!!!!!!")
        // observalbe = RrosObservable::new().unwrap();
    }
    // 	if (clone_flags & RROS_CLONE_OBSERVABLE) {
    // 		/*
    // 		 * Accessing the observable is done via the thread
    // 		 * element (if public), so clear the public flag for
    // 		 * the observable itself.
    // 		 */
    // 		observable = rros_alloc_observable(
    // 			u_name, clone_flags & ~RROS_CLONE_PUBLIC);
    // 		if (IS_ERR(observable)) {
    // 			ret = PTR_ERR(observable);
    // 			goto fail_observable;
    // 		}
    // 		/*
    // 		 * Element name was already set from user input by
    // 		 * rros_alloc_observable(). rros_create_core_element_device()
    // 		 * is told to skip name assignment (NULL name).
    // 		 */
    // 		ret = rros_create_core_element_device(
    // 			&observable->element,
    // 			&rros_observable_factory, NULL);
    // 		if (ret)
    // 			goto fail_observable_dev;
    // 		observable = observable;
    // 	} else if (clone_flags & RROS_CLONE_MASTER) {
    // 		ret = -EINVAL;
    // 		goto fail_observable;
    // 	}

    // TODO: update the affinity to cpu_possible_mask
    iattr.affinity = unsafe { &RROS_OOB_CPUS as *const CpumaskT };
    // 	iattr.affinity = cpu_possible_mask;
    // iattr.observable = observalbe;
    // 	iattr.observable = observable;
    // FIXME: alter sched_fifo with sched_weak, but for now we just use fifo
    iattr.sched_class = unsafe { Some(&fifo::RROS_SCHED_FIFO) };
    // 	iattr.sched_class = &rros_sched_weak;
    // FIXME: alter fifo with weak, but for now we just use fifo
    unsafe {
        let sched_param = Arc::try_new(SpinLock::new(RrosSchedParam::new())).unwrap();
        (*sched_param.locked_data().get()).fifo.prio = 10;
        iattr.sched_param = Some(sched_param);
    }
    // iattr.sched_param.weak.prio = 0;
    // 	iattr.sched_param.weak.prio = 0;
    // TODO: update the rq parameter
    unsafe {
        pr_debug!(
            "2 uninit_thread: x ref is {}",
            Arc::strong_count(&UTHREAD.clone().unwrap())
        );
    }
    let _ret = rros_init_thread(&Some(curr.clone()), iattr, this_rros_rq(), uname);
    unsafe {
        pr_debug!(
            "uninit_thread: x ref is {}",
            Arc::strong_count(&UTHREAD.clone().unwrap())
        );
    }
    // 	ret = rros_init_thread(curr, &iattr, NULL, "%s",
    // 			rros_element_name(&curr->element));
    // 	if (ret)
    // 		goto fail_thread;

    map_uthread_self(curr.clone()).unwrap();
    unsafe {
        pr_debug!(
            "3 uninit_thread: x ref is {}",
            Arc::strong_count(&UTHREAD.clone().unwrap())
        );
    }
    // 	ret = map_uthread_self(curr);
    // 	if (ret)
    // 		goto fail_map;

    // TODO: set u_window
    // let state_offp = rros_shared_offset(curr.u_window);
    // 	*state_offp = rros_shared_offset(curr->u_window);
    // TODO: add the index `rros_index_factory_element` function
    // 	rros_index_factory_element(&curr->element);

    // 	/*
    // 	 * Unlike most elements, a thread may exist in absence of any
    // 	 * file reference, so we get a reference on the emerging
    // 	 * thread here to block automatic disposal on last file
    // 	 * release. put_current_thread() drops this reference when the
    // 	 * thread exits, or voluntarily detaches by sending the
    // 	 * RROS_THRIOC_DETACH_SELF control request.
    // 	 */
    // TODO: add the index `rros_get_element` function
    // 	rros_get_element(&curr->element);

    // TODO: modify the tsk name
    let comm_name_i: [u8; 16] = *b"new_thread_lhy0\0";
    let comm_name: [i8; 16] = unsafe { core::mem::transmute(comm_name_i) };
    // let comm_name_slice = comm_name.as_bytes();
    unsafe {
        (*tsk).comm.clone_from(&comm_name);
    }
    // tsk.comm = factory::rros_element_name(curr.element);
    // 	strncpy(tsk->comm, rros_element_name(&curr->element),
    // 		sizeof(tsk->comm));
    // 	tsk->comm[sizeof(tsk->comm) - 1] = '\0';
    // `RrosElement.pointer` point to `rros_thread` struct.
    let rros_thread_ptr: *mut RrosThread = curr.locked_data().get();
    (curr.lock().element.borrow_mut()).pointer = rros_thread_ptr as *mut u8;

    unsafe { (*curr.locked_data().get()).element.clone() }
    // 	return &curr->element;

    // fail_map:
    // 	discard_unmapped_uthread(curr);
    // fail_thread:
    // 	if (observable)
    // fail_observable_dev:
    // 		rros_put_element(&observable->element); /* ->dispose() */
    // fail_observable:
    // 	rros_destroy_element(&curr->element);
    // fail_element:
    // 	kfree(curr);

    // 	return ERR_PTR(ret);
    // }
}

pub fn rros_init_user_element(
    e: Rc<RefCell<RrosElement>>,
    fac: &'static mut SpinLock<RrosFactory>,
    uname: &'static CStr,
    clone_flags: i32,
) -> Result<i32> {
    // int rros_init_user_element(struct rros_element *e,
    //     struct rros_factory *fac,
    //     const char __user *u_name,
    //     int clone_flags)
    // {
    // struct filename *devname;
    // char tmpbuf[32];
    // int ret;
    rros_init_element(e.clone(), fac, clone_flags)?;
    // ret = rros_init_element(e, fac, clone_flags);
    // if (ret)
    // return ret;

    match fs::Filename::getname(uname) {
        Ok(res) => e.borrow_mut().devname = Some(res),
        Err(error) => return Err(error),
    }
    // if (u_name) {
    // devname = getname(u_name);
    // } else {
    // snprintf(tmpbuf, sizeof(tmpbuf), "%s%%%d",
    //     fac->name, e->minor);
    // devname = getname_kernel(tmpbuf);
    // }

    // if (IS_ERR(devname)) {
    // rros_destroy_element(e);
    // return PTR_ERR(devname);
    // }

    // return 0;
    Ok(0)
    // }
}

fn map_uthread_self(thread: Arc<SpinLock<RrosThread>>) -> Result<usize> {
    // static int map_uthread_self(struct RrosThread *thread)
    // {
    //    mkdir /dev/rros
    //    mkdir /dev/rros/thread
    //    mknod /dev/rros/thread/clone c 245 1
    // 	struct mm_struct *mm = current->mm;
    // 	struct RrosUserWindow *u_window;
    // 	struct cred *newcap;

    // TODO: add the mm check
    // 	/* mlockall(MCL_FUTURE) required. */
    // 	if (!(mm->def_flags & VM_LOCKED))
    // 		return -EINVAL;

    // TODO: add the u_window to support statistics
    // let u_window = rros_alloc_zeroed(layout::size_of::<RrosUserWindow>()) as *mut RrosUserWindow;
    // 	u_window = rros_zalloc_chunk(&RROS_SHARED_HEAP, sizeof(*u_window));
    // 	if (u_window == NULL)
    // 		return -ENOMEM;

    // 	/*
    // 	 * Raise capababilities of user threads when attached to the
    // 	 * core. Filtering access to /dev/rros/control can be used to
    // 	 * restrict attachment.
    // 	 */
    unsafe {
        (*thread.locked_data().get()).raised_cap = capability::KernelCapStruct::new();
    }
    // 	thread->raised_cap = CAP_EMPTY_SET;
    // TODO: add the cred wrappers/ maybe first check the lastest RFL wrap
    let new_cap = cred::Credential::prepare_creds();
    // let new_cap = unsafe{bindings::prepare_creds()};
    if new_cap == 0 as *mut cred::Credential {
        return Err(Error::ENOMEM);
    }
    // 	newcap = prepare_creds();
    // 	if (newcap == NULL)
    // 		return -ENOMEM;

    add_u_cap(thread.clone(), new_cap, bindings::CAP_SYS_NICE);
    add_u_cap(thread.clone(), new_cap, bindings::CAP_IPC_LOCK);
    add_u_cap(thread.clone(), new_cap, bindings::CAP_SYS_RAWIO);
    // TODO: add the commit_creds wrappers
    cred::Credential::commit_creds(new_cap);
    // 	add_u_cap(thread, newcap, CAP_SYS_NICE);
    // 	add_u_cap(thread, newcap, CAP_IPC_LOCK);
    // 	add_u_cap(thread, newcap, CAP_SYS_RAWIO);
    // 	commit_creds(newcap);

    // 	/*
    // 	 * CAUTION: From that point, we assume the mapping won't fail,
    // 	 * therefore there is no added capability to drop in
    // 	 * discard_unmapped_uthread().
    // 	 */
    // TODO: add the support of u_window
    // 	thread->u_window = u_window;

    pin_to_initial_cpu(thread.clone());
    // TODO: add the trace function
    // 	trace_rros_thread_map(thread);

    unsafe {
        (*thread.locked_data().get())
            .altsched
            .dovetail_init_altsched();
    }
    // 	dovetail_init_altsched(&thread->altsched);
    unsafe {
        pr_debug!(
            "new_1 uninit_thread: x ref is {}",
            Arc::strong_count(&UTHREAD.clone().unwrap())
        );
    }
    set_oob_threadinfo(
        Arc::into_raw(thread.clone()) as *mut SpinLock<RrosThread> as *mut c_types::c_void
    );

    unsafe {
        pr_debug!(
            "new_2 uninit_thread: x ref is {}",
            Arc::strong_count(&UTHREAD.clone().unwrap())
        );
    }
    // Arc::into_raw(thread.clone()) as *mut SpinLock<RrosThread> as *mut c_types::c_void;
    unsafe {
        pr_debug!(
            "new_3 uninit_thread: x ref is {}",
            Arc::strong_count(&UTHREAD.clone().unwrap())
        );
    }
    // 	set_oob_threadinfo(thread);
    set_oob_mminfo(thread.clone());
    // 	set_oob_mminfo(thread);

    // 	/*
    // 	 * CAUTION: we enable dovetailing only when *thread is
    // 	 * consistent, so that we won't trigger false positive in
    // 	 * debug code from handle_schedule_event() and friends.
    // 	 */
    dovetail::dovetail_start_altsched();
    // 	dovetail_start_altsched();

    // 	/*
    // 	 * A user-space thread is already started RROS-wise since we
    // 	 * have an underlying in-band context for it, so we can
    // 	 * enqueue it now.
    // 	 */
    // TODO: add the enqueue_new_thread wrappers. This can be omitted as shown in the map_kthread_self.
    // 	enqueue_new_thread(thread);
    // FIXME: Maybe we could use this function to add the thread to the thread list avoiding hanging the thread. If the null pointer problem occurs, it can be solved by adding the thread to the thread list in the `enqueue_new_thread` function.
    rros_release_thread(thread.clone(), T_DORMANT, 0);
    // 	rros_release_thread(thread, T_DORMANT, 0);

    // TODO: update the thread u_window
    // 	rros_sync_uwindow(thread);

    Ok(0)
    // 	return 0;
    // }
}

// TODO: update the cred wrappers
fn add_u_cap(thread: Arc<SpinLock<RrosThread>>, new_cap: *mut cred::Credential, cap: u32) {
    // TODO: add the cap_raise&&capable wrappers
    if !capability::capable(cap as i32) {
        unsafe {
            (*new_cap).cap_raise(cap as i32);
            (*thread.locked_data().get())
                .raised_cap
                .cap_raise(cap as i32);
        }
    }
}

pub fn rros_notify_thread(thread: *mut RrosThread, tag: u32, _details: RrosValue) -> Result<usize> {
    unsafe {
        if (*thread).state & T_HMSIG != 0 {
            rros_signal_thread(thread, SIGDEBUG, tag)?;
        }
        // if (*thread).state & T_HMOBS !=0 {
        //     if (!rros_send_observable(thread->observable, tag, details) == 0){
        //         printk_ratelimited(RROS_WARNING
        //             "%s[%d] could not receive HM event #%d",
        //             rros_element_name(&thread->element),
        //                 rros_get_inband_pid(thread),
        //                 tag);
        //     }
        // }
    }
    Ok(0)
}

fn rros_signal_thread(thread: *mut RrosThread, sig: i32, arg: u32) -> Result<usize> {
    let mut sigd = SigIrqworkData::new();

    // if (RROS_WARN_ON(CORE, !(thread->state & T_USER)))
    // return;

    if premmpt::running_inband() == Ok(0) {
        // do_inband_signal(thread, sig, arg);
        return Ok(0);
    }

    sigd.work.init_irq_work(sig_irqwork)?;
    sigd.thread = thread;
    sigd.signo = sig;
    if sig == SIGDEBUG {
        sigd.sigval = arg | SIGDEBUG_MARKER;
    } else {
        sigd.sigval = arg;
    }
    // rros_get_element(&thread->element);
    IrqWork::irq_work_queue(&mut sigd.work)?;
    Ok(0)
}

unsafe extern "C" fn sig_irqwork(_work: *mut IrqWork) {
    let _sigd = SigIrqworkData::new();
    // unsafe{do_inband_signal(sigd.thread, sigd.signo, sigd.sigval)};
    // rros_put_element(&sigd->thread->element);
}

// fn do_inband_signal(thread:*mut RrosThread, signo:i32, sigval:i32){
// 		si.si_int = sigval;
// 		bindings::send_sig_info(signo, &mut si as *mut bindings::kernel_siginfo, p);
// 	} else{
// 		bindings::send_sig(signo, p, 1);
// }

#[allow(dead_code)]
fn rros_get_inband_pid(thread: *mut RrosThread) -> i32 {
    unsafe {
        if (*thread).state & (T_ROOT | T_DORMANT | T_ZOMBIE) != 0 {
            return 0;
        }

        if (*thread).altsched.0.task == ptr::null_mut() {
            return -1;
        }

        return (*(*thread).altsched.0.task).pid;
    }
}

#[allow(dead_code)]
pub fn rros_track_thread_policy(
    thread: Arc<SpinLock<RrosThread>>,
    target: Arc<SpinLock<RrosThread>>,
) -> Result<usize> {
    unsafe {
        let param = RrosSchedParam::new();
        // rros_double_rq_lock((*thread).rq, (*target).rq);

        let mut state = (*thread.locked_data().get()).state;
        if state & T_READY != 0 {
            rros_dequeue_thread(thread.clone())?;
        }

        let thread_ptr =
            Arc::into_raw(thread.clone()) as *mut SpinLock<RrosThread> as *mut RrosThread;
        let target_ptr =
            Arc::into_raw(target.clone()) as *mut SpinLock<RrosThread> as *mut RrosThread;
        if target_ptr == thread_ptr {
            (*thread.locked_data().get()).sched_class = (*thread.locked_data().get()).base_class;
            rros_track_priority(
                thread.clone(),
                Arc::try_new(SpinLock::new(RrosSchedParam::new()))?,
            )?;
            state = (*thread.locked_data().get()).state;
            if state & T_READY != 0 {
                rros_requeue_thread(thread.clone())?;
            }
        } else {
            rros_get_schedparam(target.clone(), Arc::try_new(SpinLock::new(param))?)?;
            (*thread.locked_data().get()).sched_class = (*target.locked_data().get()).sched_class;
            rros_track_priority(thread.clone(), Arc::try_new(SpinLock::new(param))?)?;
            state = (*thread.locked_data().get()).state;
            if state & T_READY != 0 {
                rros_enqueue_thread(thread.clone())?;
            }
        }

        let rq = (*thread.locked_data().get()).rq;
        rros_set_resched(rq.clone());
        // rros_double_rq_unlock(thread->rq, target->rq);
        Ok(0)
    }
}

#[allow(dead_code)]
pub fn thread_oob_ioctl(filp: &File, cmd: u32, arg: u32) -> Result<usize> {
    let __fbind = unsafe { (*filp.get_ptr()).private_data as *mut RrosFileBinding };
    let thread = unsafe {
        kernel::container_of!((*__fbind).element, RrosThread, element) as *mut RrosThread
    };

    // let curr = rros_current();

    unsafe {
        if (*thread).state & T_ZOMBIE != 0 {
            return Err(kernel::Error::ESTALE);
        }
    }

    // switch (cmd) {
    // case RROS_THRIOC_SWITCH_OOB:
    // 	if (thread == curr)
    // 		ret = 0;	/* Already there. */
    // 	break;
    // case RROS_THRIOC_SWITCH_INBAND:
    // 	if (thread == curr) {
    // 		rros_switch_inband(RROS_HMDIAG_NONE);
    // 		ret = 0;
    // 	}
    // 	break;
    // case RROS_THRIOC_SIGNAL:
    // 	ret = raw_get_user(monfd, (__u32 *)arg);
    // 	if (ret)
    // 		return -EFAULT;
    // 	ret = rros_signal_monitor_targeted(thread, monfd);
    // 	break;
    // case RROS_THRIOC_YIELD:
    // 	rros_release_thread(curr, 0, 0);
    // 	rros_schedule();
    // 	ret = 0;
    // 	break;
    // default:
    return thread_common_ioctl(thread, cmd, arg);
    // }
}

#[allow(dead_code)]
pub fn thread_common_ioctl(thread: *mut RrosThread, cmd: u32, _arg: u32) -> Result<usize> {
    let mut statebuf = RrosThreadState::new();
    let mut attrs = RrosSchedAttrs::new();
    // __u32 mask, oldmask;
    let mut ret: Result<usize> = Ok(0);

    match cmd {
        RROS_THRIOC_SET_SCHEDPARAM => {
            // ret = raw_copy_from_user(&attrs,
            //         (struct rros_sched_attrs *)arg, sizeof(attrs));
            // if (ret)
            //     return -EFAULT;
            ret = set_sched_attrs(thread, attrs);
        }
        RROS_THRIOC_GET_SCHEDPARAM => {
            get_sched_attrs(thread, &mut attrs)?;
            // ret = raw_copy_to_user((struct rros_sched_attrs *)arg,
            //         &attrs, sizeof(attrs));
            // if (ret)
            //     return -EFAULT;
        }
        RROS_THRIOC_GET_STATE => {
            rros_get_thread_state(thread, &mut statebuf)?;
            // ret = raw_copy_to_user((struct rros_thread_state *)arg,
            //         &statebuf, sizeof(statebuf));
            // if (ret)
            //     return -EFAULT;
        }
        // case RROS_THRIOC_SET_MODE:
        // case RROS_THRIOC_CLEAR_MODE:
        // 	ret = raw_get_user(mask, (__u32 *)arg);
        // 	if (ret)
        // 		return -EFAULT;
        // 	ret = update_mode(thread, mask, &oldmask,
        // 			cmd == RROS_THRIOC_SET_MODE);
        // 	if (ret)
        // 		return ret;
        // 	ret = raw_put_user(oldmask, (__u32 *)arg);
        // 	if (ret)
        // 		return -EFAULT;
        // 	break;
        // case RROS_THRIOC_UNBLOCK:
        // 	rros_unblock_thread(thread, 0);
        // 	break;
        // case RROS_THRIOC_DEMOTE:
        // 	rros_demote_thread(thread);
        // 	break;
        _ => {
            ret = Err(kernel::Error::ENOTTY);
        }
    }

    unsafe { rros_schedule() };

    return ret;
}

#[allow(dead_code)]
pub fn set_sched_attrs(thread: *mut RrosThread, attrs: RrosSchedAttrs) -> Result<usize> {
    let mut param = RrosSchedParam::new();
    let mut flags: c_types::c_ulong = 0;
    let mut ret: Result<usize>;
    let tslice = unsafe { (*thread).rrperiod };
    let thread: Option<Arc<SpinLock<RrosThread>>> =
        unsafe { Some(Arc::from_raw(thread as *mut SpinLock<RrosThread>)) };
    let rq = rros_get_thread_rq(thread.clone(), &mut flags);
    let sched_class = rros_find_sched_class(&mut param, attrs, tslice);

    // if (IS_ERR(sched_class)) {
    // 	ret = PTR_ERR(sched_class);
    // 	rros_put_thread_rq(thread, rq, flags);
    //     return ret;
    // }

    ret = set_time_slice(thread.clone(), tslice);
    if ret != Ok(0) {
        rros_put_thread_rq(thread, rq, flags)?;
        return ret;
    }

    unsafe {
        ret = rros_set_thread_schedparam_locked(
            thread.clone().unwrap(),
            Some(sched_class),
            Some(Arc::try_new(SpinLock::new(param))?),
        )
    };

    rros_put_thread_rq(thread, rq, flags)?;

    return ret;
}

#[allow(dead_code)]
pub fn rros_find_sched_class(
    param: &mut RrosSchedParam,
    _attrs: RrosSchedAttrs,
    mut _tslice_r: ktime::KtimeT,
) -> &'static RrosSchedClass {
    let sched_class: &RrosSchedClass = unsafe { &RROS_SCHED_FIFO };
    // int prio, policy;
    // KtimeT tslice;

    // policy = attrs->sched_policy;
    // prio = attrs->sched_priority;
    // tslice = RROS_INFINITE;
    // sched_class = &rros_sched_fifo;
    param.fifo.prio = 47;

    // TODO: Currently, only tp is written here in the policy.
    // switch (policy) {
    //     case SCHED_NORMAL:
    //         if (prio)
    //             return ERR_PTR(-EINVAL);
    //         fallthrough;
    //     case SCHED_WEAK:
    //         if (prio < RROS_WEAK_MIN_PRIO ||	prio > RROS_WEAK_MAX_PRIO)
    //             return ERR_PTR(-EINVAL);
    //         param->weak.prio = prio;
    //         sched_class = &rros_sched_weak;
    //         break;
    //     case SCHED_RR:
    //         /* if unspecified, use current one. */
    //         tslice = u_timespec_to_ktime(attrs->sched_rr_quantum);
    //         if (timeout_infinite(tslice) && tslice_r)
    //             tslice = *tslice_r;
    //         fallthrough;
    //     case SCHED_FIFO:
    //         /*
    //         * This routine handles requests submitted from
    //         * user-space exclusively, so a SCHED_FIFO priority
    //         * must be in the [FIFO_MIN..FIFO_MAX] range.
    //         */
    //         if (prio < RROS_FIFO_MIN_PRIO ||	prio > RROS_FIFO_MAX_PRIO)
    //             return ERR_PTR(-EINVAL);
    //         break;
    //     case SCHED_QUOTA:
    // #ifdef CONFIG_RROS_SCHED_QUOTA
    //         param->quota.prio = attrs->sched_priority;
    //         param->quota.tgid = attrs->sched_quota_group;
    //         sched_class = &rros_sched_quota;
    //         break;
    // #else
    //         return ERR_PTR(-EOPNOTSUPP);
    // #endif
    //     case SCHED_TP:
    // #ifdef CONFIG_RROS_SCHED_TP
    //         param->tp.prio = attrs->sched_priority;
    //         param->tp.ptid = attrs->sched_tp_partition;
    //         sched_class = &RROS_SCHED_TP;
    //         break;
    // #else
    //         return ERR_PTR(-EOPNOTSUPP);
    // #endif
    //     default:
    //         return ERR_PTR(-EINVAL);
    // }

    // According to what librros wrote, prio + ptid should be 50, and part should be 3.
    param.tp.prio = 47;
    param.tp.ptid = 3;
    //FIXME: tp haven't been implemented.
    // sched_class = unsafe{&RROS_SCHED_TP};
    // panic!();
    sched_class
}

#[allow(dead_code)]
pub fn set_time_slice(
    thread: Option<Arc<SpinLock<RrosThread>>>,
    quantum: ktime::KtimeT,
) -> Result<usize> {
    let thread = thread.clone().unwrap();
    let rq = unsafe { (*thread.locked_data().get()).rq.unwrap() };

    // assert_hard_lock(&thread->lock);
    // assert_hard_lock(&rq->lock);

    unsafe { (*thread.locked_data().get()).rrperiod = quantum };

    let curr_ptr = unsafe { (*rq).curr.clone().unwrap().locked_data().get() };
    let thread_ptr = thread.clone().locked_data().get();
    if quantum != 0 {
        unsafe {
            if quantum <= (&clock::RROS_MONO_CLOCK).get_gravity_user() {
                return Err(kernel::Error::EINVAL);
            }
        }

        let sched_tick = unsafe { (*thread.locked_data().get()).base_class.unwrap().sched_tick };
        if sched_tick.is_none() {
            return Err(kernel::Error::EINVAL);
        }

        unsafe { (*thread.locked_data().get()).state |= T_RRB };
        unsafe {
            if curr_ptr == thread_ptr {
                timer::rros_start_timer(
                    (*rq).rrbtimer.clone().unwrap(),
                    timer::rros_abs_timeout((*rq).rrbtimer.clone().unwrap(), quantum),
                    RROS_INFINITE,
                );
            }
        }
    } else {
        unsafe {
            (*thread.locked_data().get()).state &= !T_RRB;
            if curr_ptr == thread_ptr {
                timer::rros_stop_timer((*rq).rrbtimer.clone().unwrap());
            }
        }
    }

    Ok(0)
}

#[allow(dead_code)]
pub fn get_sched_attrs(thread: *mut RrosThread, attrs: &mut RrosSchedAttrs) -> Result<usize> {
    let mut flags: c_types::c_ulong = 0;
    let thread_ptr = thread;
    let thread: Option<Arc<SpinLock<RrosThread>>> =
        unsafe { Some(Arc::from_raw(thread as *mut SpinLock<RrosThread>)) };
    let rq = rros_get_thread_rq(thread.clone(), &mut flags);
    /* Get the base scheduling attributes. */
    attrs.sched_priority = unsafe { (*thread.clone().unwrap().locked_data().get()).bprio };
    let base_class = unsafe { (*thread.clone().unwrap().locked_data().get()).base_class };
    __get_sched_attrs(base_class.clone(), thread_ptr, attrs)?;
    rros_put_thread_rq(thread.clone(), rq.clone(), flags)?;
    Ok(0)
}

pub fn __get_sched_attrs(
    sched_class: Option<&'static RrosSchedClass>,
    thread: *mut RrosThread,
    attrs: &mut RrosSchedAttrs,
) -> Result<usize> {
    let param = unsafe { Some(Arc::try_new(SpinLock::new(RrosSchedParam::new()))?) };
    attrs.sched_policy = sched_class.unwrap().policy;

    match sched_class.unwrap().sched_getparam {
        Some(f) => {
            let thread_option = unsafe { Some(Arc::from_raw(thread as *mut SpinLock<RrosThread>)) };
            f(thread_option.clone(), param.clone());
        }
        None => {
            pr_debug!("__get_sched_attrs sched_getparam is none");
        }
    }
    if sched_class.unwrap().flag == 3 {
        unsafe {
            if (*thread).state & T_RRB != 0 {
                // attrs->sched_rr_quantum =
                // 	ktime_to_u_timespec(thread->rrperiod);
                attrs.sched_policy = SCHED_RR;
            }
        }
    }

    // #ifdef CONFIG_RROS_SCHED_QUOTA
    // 	if (sched_class == &rros_sched_quota) {
    // 		attrs->sched_quota_group = param.quota.tgid;
    // 		goto out;
    // 	}
    // #endif

    // #ifdef CONFIG_RROS_SCHED_TP
    if sched_class.unwrap().flag == 4 {
        attrs.tp_partition = unsafe { (*param.unwrap().locked_data().get()).tp.ptid };
    }
    // #endif

    // out:
    // 	trace_rros_thread_getsched(thread, attrs);
    Ok(0)
}

#[allow(dead_code)]
pub fn rros_get_thread_state(
    thread: *mut RrosThread,
    statebuf: &mut RrosThreadState,
) -> Result<usize> {
    let mut flags: c_types::c_ulong = 0;
    let thread_option = unsafe { Some(Arc::from_raw(thread as *mut SpinLock<RrosThread>)) };
    let rq = rros_get_thread_rq(thread_option.clone(), &mut flags);
    statebuf.eattrs.sched_priority = unsafe { (*thread).cprio };
    unsafe { __get_sched_attrs((*thread).sched_class.clone(), thread, &mut statebuf.eattrs)? };
    unsafe {
        statebuf.cpu = rros_rq_cpu((*thread).rq.unwrap()) as u32;
        statebuf.state = rros_rq_cpu((*thread).rq.unwrap()) as u32;
        statebuf.isw = (*thread).stat.isw.get_counter();
        statebuf.csw = (*thread).stat.csw.get_counter();
        statebuf.sc = (*thread).stat.sc.get_counter();
        statebuf.rwa = (*thread).stat.rwa.get_counter();
        statebuf.xtime = (*thread).stat.account.get_account_total() as u32;
    }
    rros_put_thread_rq(thread_option.clone(), rq.clone(), flags)?;
    Ok(0)
}

pub struct LatmusRunner {
    pub period: u64,
    pub state: RunnerState,
}

pub struct RunnerState {
    pub min_latency: u64,
    pub max_latency: u64,
    pub avg_latency: u64,
    pub ideal: u64,
    pub offset: u64,
}

// TODO: fix the kthreadrunner with more flags to adjust the latmus and move this struct to latmus
pub struct KthreadRunner(pub Option<RrosKthread>, pub u64, pub LatmusRunner);
impl KthreadRunner {
    pub const fn new_empty() -> Self {
        KthreadRunner(
            None,
            0,
            LatmusRunner {
                period: 0,
                state: RunnerState {
                    min_latency: 0,
                    max_latency: 0,
                    avg_latency: 0,
                    ideal: 0,
                    offset: 0,
                },
            },
        )
    }
    // pub fn new(kfn:Box<dyn FnOnce()>) -> Self{
    //     let mut r = Self::new_empty();
    //     r.init(kfn);
    //     return r;
    // }
    pub fn init(&mut self, kfn: Box<dyn FnOnce()>) {
        let mut kthread = RrosKthread::new(Some(kfn));
        // let mut thread = unsafe{SpinLock::new(RrosThread::new().unwrap())};
        // let pinned: Pin<&mut SpinLock<RrosThread>> = unsafe{Pin::new_unchecked(&mut thread)};
        // spinlock_init!(pinned, "test_threads2");
        // kthread.thread = Some(Arc::try_new(thread).unwrap());

        let mut tmp = Arc::<SpinLock<RrosThread>>::try_new_uninit().unwrap();
        let mut tmp = unsafe {
            ptr::write_bytes(Arc::get_mut_unchecked(&mut tmp), 0, 1);
            tmp.assume_init()
        };
        let pinned = unsafe { Pin::new_unchecked(Arc::get_mut_unchecked(&mut tmp)) };
        spinlock_init!(pinned, "rros_kthreads");
        unsafe {
            let _ret = (*Arc::get_mut_unchecked(&mut tmp).locked_data().get()).init();
        }
        kthread.thread = Some(tmp); //Arc::try_new(thread)?
                                    // unsafe{(*(*kthread.thread.as_mut().unwrap().locked_dataed_data().get()).get()).init().unwrap()};
        let pinned = unsafe {
            Pin::new_unchecked(
                &mut *(Arc::into_raw(kthread.thread.clone().unwrap()) as *mut SpinLock<RrosThread>),
            )
        };
        // &mut *Arc::into_raw( *(*rq_ptr).root_thread.clone().as_mut().unwrap()) as &mut SpinLock<RrosThread>
        spinlock_init!(pinned, "rros_threads");

        let mut r = unsafe { SpinLock::new(timer::RrosTimer::new(1)) };
        let pinned_r = unsafe { Pin::new_unchecked(&mut r) };
        spinlock_init!(pinned_r, "rtimer_3");
        let mut p = unsafe { SpinLock::new(timer::RrosTimer::new(1)) };
        let pinned_p = unsafe { Pin::new_unchecked(&mut p) };
        spinlock_init!(pinned_p, "ptimer_3");

        kthread.thread.as_mut().map(|thread| unsafe {
            let mut t = &mut (*(*thread).locked_data().get());
            t.rtimer = Some(Arc::try_new(r).unwrap());
            t.ptimer = Some(Arc::try_new(p).unwrap());
        });
        self.0 = Some(kthread);
    }

    pub fn run(&mut self, name: &'static CStr) {
        unsafe {
            let x = self.0.as_mut().unwrap().thread.as_mut().unwrap();
            (*(*x).locked_data().get()).name = core::str::from_utf8(name.as_bytes()).unwrap()
        }
        if let Some(t) = self.0.as_mut() {
            let _ret = rros_run_kthread(t, name);
        }
    }

    #[allow(dead_code)]
    fn cancel(&mut self) {
        if let Some(_t) = self.0.as_mut() {
            // rros_stop_kthread( t.thread.clone().unwrap());
            self.0 = None;
        }
    }
}

pub fn rros_set_period(
    clock: &mut clock::RrosClock,
    idate: ktime::KtimeT,
    period: ktime::KtimeT,
    flag: i32,
) {
    // int rros_set_period(struct rros_clock *clock,
    //     ktime_t idate, ktime_t period)
    // {
    let curr = rros_current();
    let thread;
    unsafe {
        thread = Arc::from_raw(curr as *mut SpinLock<RrosThread>);
        Arc::increment_strong_count(curr);
    }
    let timer = unsafe {
        (*thread.locked_data().get())
            .ptimer
            .as_ref()
            .unwrap()
            .clone()
    };

    // struct RrosThread *curr = rros_current();
    // unsigned long flags;
    // int ret = 0;

    // TODO: add the error handling
    // if (curr == NULL)
    //     return -EPERM;

    // TODO: add the error handling
    if flag == 0 {
        rros_stop_timer(timer);
        return;
    }
    // if (clock == NULL || period == RROS_INFINITE) {
    //     rros_stop_timer(&curr->ptimer);
    //     return 0;
    // }

    // TODO: add the error handling
    // /*
    //  * LART: detect periods which are shorter than the target
    //  * clock gravity for kernel thread timers. This can't work,
    //  * caller must have messed up arguments.
    //  */
    // if (period < rros_get_clock_gravity(clock, kernel))
    //     return -EINVAL;

    // TODO: we should use a guard to avoid manully locking and releasing of locks.
    let flags = thread.irq_lock_noguard();

    timer::rros_prepare_timed_wait(timer.clone(), clock, unsafe {
        (*thread.locked_data().get()).rq.unwrap()
    });

    // TODO: add this function
    // if (timeout_infinite(idate))
    //     idate = rros_abs_timeout(&curr->ptimer, period);

    timer::rros_start_timer(timer.clone(), idate, period);
    // rros_start_timer(&curr->ptimer, idate, period);

    thread.irq_unlock_noguard(flags);

    // return ret;
    // }
    // EXPORT_SYMBOL_GPL(rros_set_period);
}

pub fn rros_wait_period() -> Result<usize> {
    let curr = rros_current();
    let thread = unsafe { Arc::from_raw(curr as *mut SpinLock<RrosThread>) };
    unsafe {
        Arc::increment_strong_count(curr);
    }
    // int rros_wait_period(unsigned long *overruns_r)
    // {
    // 	unsigned long overruns, flags;
    // 	struct RrosThread *curr;
    // 	struct rros_clock *clock;
    // 	KtimeT now;

    // 	curr = rros_current();
    // TODO: add the rros error handling function
    // 	if (unlikely(!rros_timer_is_running(&curr->ptimer)))
    // 		return -EAGAIN;

    // 	trace_rros_thread_wait_period(curr);

    let flags = unsafe { rust_helper_hard_local_irq_save() };
    // 	flags = hard_local_irq_save();

    let clock = unsafe {
        (*(*thread.locked_data().get())
            .ptimer
            .as_ref()
            .unwrap()
            .locked_data()
            .get())
        .get_clock()
    };
    // 	clock = curr->ptimer.clock;
    let now = unsafe { rros_read_clock(&mut *clock as &mut clock::RrosClock) };
    // 	now = rros_read_clock(clock);
    let timer = unsafe { (*thread.locked_data().get()).ptimer.as_ref().unwrap() };
    if now < timer::rros_get_timer_next_date(timer.clone()) {
        // 	if (likely(now < rros_get_timer_next_date(&curr->ptimer))) {
        unsafe {
            rros_sleep_on(
                RROS_INFINITE,
                timeout::RrosTmode::RrosRel,
                &mut *clock as &clock::RrosClock,
                0 as *mut RrosWaitChannel,
            );
        }
        // 		rros_sleep_on(RROS_INFINITE, RROS_REL, clock, NULL); /* T_WAIT */
        unsafe { rust_helper_hard_local_irq_restore(flags) };
        // 		hard_local_irq_restore(flags);
        unsafe {
            rros_schedule();
        }
    // 		rros_schedule();

    // TODO: add the rros error handling function
    // 		if (unlikely(curr->info & T_BREAK))
    // 			return -EINTR;
    } else {
        unsafe { rust_helper_hard_local_irq_restore(flags) };
    }
    // 	} else
    // 		hard_local_irq_restore(flags);

    // TODO: overruns is zero for this situation.
    let overruns = rros_get_timer_overruns(timer.clone())?;
    // overruns = rros_get_timer_overruns(&curr->ptimer);
    if overruns != 0 {
        pr_debug!("the overruns is not zero\n")
    }
    Ok(0)
    // 	if (overruns) {
    // 		if (likely(overruns_r != NULL))
    // 			*overruns_r = overruns;
    // 		trace_rros_thread_missed_period(curr);
    // 		return -ETIMEDOUT;
    // 	}

    // 	return 0;
    // }
    // EXPORT_SYMBOL_GPL(rros_wait_period);
}

fn rros_get_timer_overruns(timer: Arc<SpinLock<timer::RrosTimer>>) -> Result<u32> {
    // unsigned long rros_get_timer_overruns(struct rros_timer *timer)
    // {
    let mut flags = 0;
    let mut overruns = 0;
    // 	unsigned long overruns = 0, flags;
    // 	struct rros_timerbase *base;
    // 	struct RrosThread *thread;
    // 	struct rros_tqueue *tq;
    // 	KtimeT now, delta;

    let clock = unsafe { (*timer.locked_data().get()).get_clock() };
    let now = rros_read_clock(unsafe { &*(clock as *const clock::RrosClock) });
    // 	now = rros_read_clock(timer->clock);
    let tmb = timer::lock_timer_base(timer.clone(), &mut flags);

    let next_date = timer::rros_get_timer_next_date(timer.clone());
    let delta = ktime_sub(now, next_date);
    // 	delta = ktime_sub(now, rros_get_timer_next_date(timer));

    let interval = unsafe { (*timer.locked_data().get()).interval };
    if delta < interval {
        pr_debug!("rros_get_timer_overruns: delta < interval this should not happen\n");
        let pexpect_ticks = unsafe { (*timer.locked_data().get()).get_pexpect_ticks() + 1 };
        unsafe {
            (*timer.locked_data().get()).set_pexpect_ticks(pexpect_ticks);
        }
        // 	timer->pexpect_ticks++;

        timer::unlock_timer_base(tmb, flags);

        // 	/*
        // 	 * Hide overruns due to the most recent ptracing session from
        // 	 * the caller.
        // 	 */
        let curr = rros_current();
        // 	thread = rros_current();
        let local_info = unsafe { (*(*curr).locked_data().get()).local_info };
        if (local_info & T_IGNOVR) != 0 {
            return Ok(0);
        }
        // 	if (thread->local_info & T_IGNOVR)
        // 		return 0;

        return Ok(overruns as u32);
    }
    // 	if (likely(delta < timer->interval))
    // 		goto done;

    overruns = ktime::ktime_divns(delta, ktime::ktime_to_ns(interval));
    // 	overruns = ktime_divns(delta, ktime_to_ns(timer->interval));

    let periodic_ticks =
        unsafe { (*timer.locked_data().get()).get_periodic_ticks() + overruns as u64 };
    unsafe { (*timer.locked_data().get()).set_periodic_ticks(periodic_ticks) };
    // 	timer->pexpect_ticks += overruns;

    if unsafe { (*timer.locked_data().get()).get_status() } & timer::RROS_TIMER_RUNNING != 0 {
        pr_warn!("rros_get_timer_overruns: timer is running. this should not happend \n");
        let pexpect_ticks = unsafe { (*timer.locked_data().get()).get_pexpect_ticks() + 1 };
        unsafe {
            (*timer.locked_data().get()).set_pexpect_ticks(pexpect_ticks);
        }
        // 	timer->pexpect_ticks++;

        timer::unlock_timer_base(tmb, flags);

        // 	/*
        // 	 * Hide overruns due to the most recent ptracing session from
        // 	 * the caller.
        // 	 */
        let curr = rros_current();
        // 	thread = rros_current();
        let local_info = unsafe { (*(*curr).locked_data().get()).local_info };
        if (local_info & T_IGNOVR) != 0 {
            return Ok(0);
        }
        // 	if (thread->local_info & T_IGNOVR)
        // 		return 0;

        return Ok(overruns as u32);
    } else {
        pr_debug!("rros_get_timer_overruns: timer is running \n");
    }
    // if rros_timer_is_running(timer.clone()) {
    //     pr_debug!("rros_get_timer_overruns: timer is running. this should not happend \n");
    // }
    // 	if (!rros_timer_is_running(timer))
    // 		goto done;

    // 	RROS_WARN_ON_ONCE(CORE, (timer->status &
    // 						(RROS_TIMER_DEQUEUED|RROS_TIMER_PERIODIC))
    // 			!= RROS_TIMER_PERIODIC);
    let tq = unsafe { &mut (*tmb).q };
    // 	tq = &base->q;
    rros_dequeue_timer(timer.clone(), tq);
    // 	rros_dequeue_timer(timer, tq);
    let rros_node_date = unsafe { (*timer.locked_data().get()).get_date() };
    while rros_node_date < now {
        let periodic_ticks = unsafe { (*timer.locked_data().get()).get_periodic_ticks() } + 1;
        unsafe { (*timer.locked_data().get()).set_periodic_ticks(periodic_ticks) };
        rros_update_timer_date(timer.clone());
    }
    // 	while (rros_tdate(timer) < now) {
    // 		timer->periodic_ticks++;
    // 		rros_update_timer_date(timer);
    // 	}

    program_timer(timer.clone(), tq);
    // 	program_timer(timer, tq);
    // done:

    let pexpect_ticks = unsafe { (*timer.locked_data().get()).get_pexpect_ticks() + 1 };
    unsafe {
        (*timer.locked_data().get()).set_pexpect_ticks(pexpect_ticks);
    }
    // 	timer->pexpect_ticks++;

    timer::unlock_timer_base(tmb, flags);

    // 	/*
    // 	 * Hide overruns due to the most recent ptracing session from
    // 	 * the caller.
    // 	 */
    let curr = rros_current();
    // 	thread = rros_current();
    let local_info = unsafe { (*(*curr).locked_data().get()).local_info };
    if (local_info & T_IGNOVR) != 0 {
        return Ok(0);
    }
    // 	if (thread->local_info & T_IGNOVR)
    // 		return 0;

    Ok(overruns as u32)
    // 	return overruns;
    // }
    // EXPORT_SYMBOL_GPL(rros_get_timer_overruns);
}

pub fn rros_kick_thread(thread: Arc<SpinLock<RrosThread>>, mut info: u32) {
    let mut flags = 0u64;
    let rq = rros_get_thread_rq(Some(thread.clone()), &mut flags);
    loop {
        let mut guard: Guard<'_, SpinLock<RrosThread>> = thread.lock();
        if guard.state & T_INBAND != 0 {
            break;
        }

        if (info & T_PTSIG == 0) && (guard.info & T_KICKED != 0) {
            break;
        }

        rros_wakeup_thread_locked(
            thread.clone(),
            T_DELAY | T_PEND | T_WAIT,
            (T_KICKED | T_BREAK) as i32,
        );

        if guard.info & T_PTSTOP != 0 {
            if guard.info & T_PTJOIN != 0 {
                guard.info &= !T_PTJOIN;
            } else {
                info &= !T_PTJOIN;
            }
        }

        rros_release_thread_locked(thread.clone(), T_SUSP | T_HALT | T_PTSYNC, T_KICKED);

        if guard.state & T_USER != 0 {
            match this_rros_rq_thread() {
                Some(_) => {
                    dovetail::dovetail_send_mayday(guard.altsched.0.task);
                }
                None => {}
            }
        }

        guard.info |= T_KICKED;

        if guard.state & T_READY != 0 {
            rros_force_thread(thread.clone());
            rros_set_resched(guard.rq);
        }

        if info != 0 {
            guard.info |= info;
        }
        break;
    }

    rros_put_thread_rq(Some(thread.clone()), rq, flags).unwrap();
}
