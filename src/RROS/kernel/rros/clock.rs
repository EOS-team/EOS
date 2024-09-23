#![allow(warnings, unused)]
#![feature(stmt_expr_attributes)]
use crate::{
    factory::{self, CloneData, RrosElement, RrosFactory, RustFile, RROS_CLONE_PUBLIC},
    file::{rros_release_file, RrosFile, RrosFileBinding},
    list::*,
    lock::*,
    monitor::{CLOCK_MONOTONIC, CLOCK_REALTIME},
    poll::RrosPollHead,
    sched::{
        self, is_rros_cpu, rros_cpu_rq, rros_get_thread_rq, rros_rq, rros_rq_cpu, this_rros_rq,
        RQ_TDEFER, RQ_TIMER, RQ_TPROXY,
    },
    thread::{rros_current, rros_delay, T_ROOT, T_SYSRST},
    tick::{self, *},
    timeout::{RrosTmode, RROS_INFINITE},
    timer::*,
    tp::rros_timer_is_running,
    wait::RrosWaitQueue,
    RROS_OOB_CPUS,
};

use alloc::rc::Rc;

use core::{
    borrow::{Borrow, BorrowMut},
    cell::{RefCell, UnsafeCell},
    clone::Clone,
    cmp::Ordering::{Equal, Greater, Less},
    mem::{align_of, size_of},
    ops::{Deref, DerefMut},
};

use kernel::{
    bindings,
    c_types::{self, c_void},
    chrdev::Cdev,
    clockchips,
    cpumask::{self, online_cpus, CpumaskT},
    device::DeviceType,
    double_linked_list::*,
    file::File,
    file_operations::{FileOpener, FileOperations, IoctlCommand},
    io_buffer::{IoBufferReader, IoBufferWriter},
    ioctl,
    ktime::*,
    percpu,
    prelude::*,
    premmpt, spinlock_init,
    str::CStr,
    sync::{Lock, SpinLock},
    sysfs,
    task::Task,
    timekeeping,
    uidgid::{KgidT, KuidT},
    user_ptr::{UserSlicePtr, UserSlicePtrReader, UserSlicePtrWriter},
};

static mut CLOCKLIST_LOCK: SpinLock<i32> = unsafe { SpinLock::new(1) };

// Define it as a constant here first, and then read it from /dev/rros.
const CONFIG_RROS_LATENCY_USER: KtimeT = 0;
const CONFIG_RROS_LATENCY_KERNEL: KtimeT = 0;
const CONFIG_RROS_LATENCY_IRQ: KtimeT = 0;

const CONFIG_RROS_NR_CLOCKS: usize = 8;

const RROS_CLOCK_IOCBASE: u32 = 'c' as u32;

const RROS_CLOCK_MONOTONIC: i32 = -(CLOCK_MONOTONIC as i32);
const RROS_CLOCK_REALTIME: i32 = -(CLOCK_REALTIME as i32);
const RROS_CLKIOC_SLEEP: u32 = ioctl::_IOW::<Timespec64>(RROS_CLOCK_IOCBASE, 0);
const RROS_CLKIOC_GET_RES: u32 = ioctl::_IOR::<Timespec64>(RROS_CLOCK_IOCBASE, 1);
const RROS_CLKIOC_GET_TIME: u32 = ioctl::_IOR::<Timespec64>(RROS_CLOCK_IOCBASE, 2);
const RROS_CLKIOC_SET_TIME: u32 = ioctl::_IOR::<Timespec64>(RROS_CLOCK_IOCBASE, 3);
const RROS_CLKIOC_NEW_TIMER: u32 = ioctl::_IO(RROS_CLOCK_IOCBASE, 5);

extern "C" {
    fn rust_helper_minor(dev: bindings::dev_t) -> u32;
    fn rust_helper_rcu_read_lock();
    fn rust_helper_rcu_read_unlock();
}

#[derive(Default)]
pub struct RustFileClock;

impl FileOperations for RustFileClock {
    kernel::declare_file_operations!();
}

pub struct RrosClockGravity {
    irq: KtimeT,
    kernel: KtimeT,
    user: KtimeT,
}

impl RrosClockGravity {
    pub fn new(irq: KtimeT, kernel: KtimeT, user: KtimeT) -> Self {
        RrosClockGravity { irq, kernel, user }
    }
    pub fn get_irq(&self) -> KtimeT {
        self.irq
    }

    pub fn get_kernel(&self) -> KtimeT {
        self.kernel
    }

    pub fn get_user(&self) -> KtimeT {
        self.user
    }

    pub fn set_irq(&mut self, irq: KtimeT) {
        self.irq = irq;
    }

    pub fn set_kernel(&mut self, kernel: KtimeT) {
        self.kernel = kernel;
    }

    pub fn set_user(&mut self, user: KtimeT) {
        self.user = user;
    }
}

pub struct RrosClockOps {
    read: Option<fn(&RrosClock) -> KtimeT>,
    readcycles: Option<fn(&RrosClock) -> u64>,
    set: Option<fn(&mut RrosClock, KtimeT) -> i32>,
    programlocalshot: Option<fn(&RrosClock)>,
    programremoteshot: Option<fn(&RrosClock, *mut RrosRq)>,
    setgravity: Option<fn(&mut RrosClock, RrosClockGravity)>,
    resetgravity: Option<fn(&mut RrosClock)>,
    adjust: Option<fn(&mut RrosClock)>,
}

impl RrosClockOps {
    pub fn new(
        read: Option<fn(&RrosClock) -> KtimeT>,
        readcycles: Option<fn(&RrosClock) -> u64>,
        set: Option<fn(&mut RrosClock, KtimeT) -> i32>,
        programlocalshot: Option<fn(&RrosClock)>,
        programremoteshot: Option<fn(&RrosClock, *mut RrosRq)>,
        setgravity: Option<fn(&mut RrosClock, RrosClockGravity)>,
        resetgravity: Option<fn(&mut RrosClock)>,
        adjust: Option<fn(&mut RrosClock)>,
    ) -> Self {
        RrosClockOps {
            read,
            readcycles,
            set,
            programlocalshot,
            programremoteshot,
            setgravity,
            resetgravity,
            adjust,
        }
    }
}

pub struct RrosClock {
    resolution: KtimeT,
    gravity: RrosClockGravity,
    name: &'static CStr,
    flags: i32,
    ops: RrosClockOps,
    timerdata: *mut RrosTimerbase,
    master: *mut RrosClock,
    offset: KtimeT,
    next: *mut ListHead,
    element: Option<Rc<RefCell<RrosElement>>>,
    dispose: Option<fn(&mut RrosClock)>,
    #[cfg(CONFIG_SMP)]
    pub affinity: Option<CpumaskT>,
}

impl RrosClock {
    pub fn new(
        resolution: KtimeT,
        gravity: RrosClockGravity,
        name: &'static CStr,
        flags: i32,
        ops: RrosClockOps,
        timerdata: *mut RrosTimerbase,
        master: *mut RrosClock,
        offset: KtimeT,
        next: *mut ListHead,
        element: Option<Rc<RefCell<RrosElement>>>,
        dispose: Option<fn(&mut RrosClock)>,
        #[cfg(CONFIG_SMP)] affinity: Option<CpumaskT>,
    ) -> Self {
        RrosClock {
            resolution,
            gravity,
            name,
            flags,
            ops,
            timerdata,
            master,
            offset,
            next,
            element,
            dispose,
            #[cfg(CONFIG_SMP)]
            affinity,
        }
    }
    pub fn read(&self) -> KtimeT {
        // Error handling.
        if self.ops.read.is_some() {
            return self.ops.read.unwrap()(&self);
        }
        return 0;
    }
    pub fn read_cycles(&self) -> u64 {
        // Error handling.
        if self.ops.readcycles.is_some() {
            return self.ops.readcycles.unwrap()(&self);
        }
        return 0;
    }
    pub fn set(&mut self, time: KtimeT) -> Result<usize> {
        if self.ops.set.is_some() {
            self.ops.set.unwrap()(self, time);
        } else {
            // Prevent the execution of the function if it is null.
            return Err(kernel::Error::EFAULT);
        }
        Ok(0)
    }
    pub fn program_local_shot(&self) {
        if self.ops.programlocalshot.is_some() {
            self.ops.programlocalshot.unwrap()(self);
        }
    }
    pub fn program_remote_shot(&self, rq: *mut RrosRq) {
        if self.ops.programremoteshot.is_some() {
            self.ops.programremoteshot.unwrap()(self, rq);
        }
    }
    pub fn set_gravity(&mut self, gravity: RrosClockGravity) {
        if self.ops.setgravity.is_some() {
            self.ops.setgravity.unwrap()(self, gravity);
        }
    }
    pub fn reset_gravity(&mut self) {
        if self.ops.resetgravity.is_some() {
            self.ops.resetgravity.unwrap()(self);
        }
    }
    pub fn adjust(&mut self) {
        if self.ops.adjust.is_some() {
            self.ops.adjust.unwrap()(self);
        }
    }
    pub fn get_timerdata_addr(&self) -> *mut RrosTimerbase {
        // Error handling.
        return self.timerdata as *mut RrosTimerbase;
    }

    pub fn get_gravity_irq(&self) -> KtimeT {
        self.gravity.get_irq()
    }

    pub fn get_gravity_kernel(&self) -> KtimeT {
        self.gravity.get_kernel()
    }

    pub fn get_gravity_user(&self) -> KtimeT {
        self.gravity.get_user()
    }

    pub fn set_gravity_user(&mut self, user: KtimeT) {
        self.gravity.set_user(user);
    }

    pub fn get_offset(&self) -> KtimeT {
        self.offset
    }

    // TODO: We'd better change `get_master` to `master`, so as other functions.
    // But it is not a big problem here.
    // FYI: https://github.com/BUPT-OS/RROS/pull/41#discussion_r1680743392
    // FYI: https://rust-lang.github.io/api-guidelines/naming.html#getter-names-follow-rust-convention-c-getter
    pub fn get_master(&self) -> *mut RrosClock {
        self.master
    }
}

pub fn adjust_timer(
    clock: &RrosClock,
    timer: Arc<SpinLock<RrosTimer>>,
    tq: &mut List<Arc<SpinLock<RrosTimer>>>,
    delta: KtimeT,
) {
    let date = timer.lock().get_date();
    timer.lock().set_date(ktime_sub(date, delta));
    let is_periodic = timer.lock().is_periodic();
    if is_periodic == false {
        rros_enqueue_timer(timer.clone(), tq);
        return;
    }

    let start_date = timer.lock().get_start_date();
    timer.lock().set_start_date(ktime_sub(start_date, delta));

    let period = timer.lock().get_interval();
    let diff = ktime_sub(clock.read(), rros_get_timer_expiry(timer.clone()));

    if (diff >= period) {
        let div = ktime_divns(diff, ktime_to_ns(period));
        let periodic_ticks = timer.lock().get_periodic_ticks();
        timer
            .lock()
            .set_periodic_ticks((periodic_ticks as i64 + div) as u64);
    } else if (ktime_to_ns(delta) < 0
        && (timer.lock().get_status() & RROS_TIMER_FIRED != 0)
        && ktime_to_ns(ktime_add(diff, period)) <= 0)
    {
        /*
         * Timer is periodic and NOT waiting for its first
         * shot, so we make it tick sooner than its original
         * date in order to avoid the case where by adjusting
         * time to a sooner date, real-time periodic timers do
         * not tick until the original date has passed.
         */
        let div = ktime_divns(-diff, ktime_to_ns(period));
        let periodic_ticks = timer.lock().get_periodic_ticks();
        let pexpect_ticks = timer.lock().get_pexpect_ticks();
        timer
            .lock()
            .set_periodic_ticks((periodic_ticks as i64 - div) as u64);
        timer
            .lock()
            .set_pexpect_ticks((pexpect_ticks as i64 - div) as u64);
    }
    rros_update_timer_date(timer.clone());
    rros_enqueue_timer(timer.clone(), tq);
}

pub fn rros_adjust_timers(clock: &mut RrosClock, delta: KtimeT) -> Result {
    for cpu in online_cpus() {
        let rq: *mut rros_rq = rros_cpu_rq(cpu as i32);
        let tmb = rros_percpu_timers(clock, cpu as i32);
        let tq = unsafe { &mut (*tmb).q };

        let flags: u64 = unsafe { (*tmb).lock.irq_lock_noguard() };

        let mut timers_adjust: Vec<Arc<SpinLock<RrosTimer>>> =
            Vec::try_with_capacity(tq.len() as usize)?;

        while !tq.is_empty() {
            let timer = tq.get_by_index(0).unwrap().value.clone();
            rros_dequeue_timer(timer.clone(), tq);
            timers_adjust.try_push(timer)?;
        }

        while let Some(timer) = timers_adjust.pop() {
            let get_clock = timer.lock().get_clock();
            if get_clock == clock as *mut RrosClock {
                adjust_timer(clock, timer, tq, delta)
            } else {
                rros_enqueue_timer(timer, tq)
            }
        }

        if rq != this_rros_rq() {
            rros_program_remote_tick(clock, rq);
        } else {
            rros_program_local_tick(clock);
        }

        unsafe {
            (*tmb).lock.irq_unlock_noguard(flags);
        }
    }
    Ok(())
}

pub fn rros_stop_timers(clock: &RrosClock) {
    for cpu in online_cpus() {
        if !is_rros_cpu(cpu as i32) {
            continue;
        }

        let mut tmb = rros_percpu_timers(clock, cpu as i32);
        let flags = unsafe { (*tmb).lock.irq_lock_noguard() };
        let tq = unsafe { &mut (*tmb).q };
        while !tq.is_empty() {
            pr_debug!("rros_stop_timers: 213");
            let timer = tq.get_head().unwrap().value.clone();
            rros_timer_deactivate(timer);
        }
        unsafe {
            (*tmb).lock.irq_unlock_noguard(flags);
        }
    }
}

// Print the initialization log of the clock.
fn rros_clock_log() {}

fn read_mono_clock(clock: &RrosClock) -> KtimeT {
    timekeeping::ktime_get_mono_fast_ns()
}

fn read_mono_clock_cycles(clock: &RrosClock) -> u64 {
    read_mono_clock(clock) as u64
}

fn set_mono_clock(clock: &mut RrosClock, time: KtimeT) -> i32 {
    // mono cannot be set, the following should be an error type.
    0
}

fn adjust_mono_clock(clock: &mut RrosClock) {}

/**
 * The following functions are the realtime clock operations.
 */

fn read_realtime_clock(clock: &RrosClock) -> KtimeT {
    timekeeping::ktime_get_real_fast_ns()
}

fn read_realtime_clock_cycles(clock: &RrosClock) -> u64 {
    read_realtime_clock(clock) as u64
}

fn set_realtime_clock(clock: &mut RrosClock, time: KtimeT) -> i32 {
    0
}

fn adjust_realtime_clock(clock: &mut RrosClock) {
    let old_offset: KtimeT = clock.offset;
    clock.offset = rros_read_clock(clock) - unsafe { RROS_MONO_CLOCK.read() };
    rros_adjust_timers(clock, clock.offset - old_offset);
}

/**
 * The following functions are universal clock operations.
 */

fn get_default_gravity() -> RrosClockGravity {
    RrosClockGravity {
        irq: CONFIG_RROS_LATENCY_IRQ,
        kernel: CONFIG_RROS_LATENCY_KERNEL,
        user: CONFIG_RROS_LATENCY_USER,
    }
}

fn set_coreclk_gravity(clock: &mut RrosClock, gravity: RrosClockGravity) {
    clock.gravity.irq = gravity.irq;
    clock.gravity.kernel = gravity.kernel;
    clock.gravity.user = gravity.user;
}

fn reset_coreclk_gravity(clock: &mut RrosClock) {
    set_coreclk_gravity(clock, get_default_gravity());
}

static RROS_MONO_CLOCK_NAME: &CStr =
    unsafe { CStr::from_bytes_with_nul_unchecked("RROS_CLOCK_MONOTONIC_DEV\0".as_bytes()) };

pub static mut RROS_MONO_CLOCK: RrosClock = RrosClock {
    name: RROS_MONO_CLOCK_NAME,
    resolution: 1,
    gravity: RrosClockGravity {
        irq: CONFIG_RROS_LATENCY_IRQ,
        kernel: CONFIG_RROS_LATENCY_KERNEL,
        user: CONFIG_RROS_LATENCY_USER,
    },
    flags: RROS_CLONE_PUBLIC,
    ops: RrosClockOps {
        read: Some(read_mono_clock),
        readcycles: Some(read_mono_clock_cycles),
        set: None,
        programlocalshot: Some(rros_program_proxy_tick),
        #[cfg(CONFIG_SMP)]
        programremoteshot: Some(rros_send_timer_ipi),
        #[cfg(not(CONFIG_SMP))]
        programremoteshot: None,
        setgravity: Some(set_coreclk_gravity),
        resetgravity: Some(reset_coreclk_gravity),
        adjust: None,
    },
    timerdata: 0 as *mut RrosTimerbase,
    master: 0 as *mut RrosClock,
    next: 0 as *mut ListHead,
    offset: 0,
    element: None,
    dispose: None,
    #[cfg(CONFIG_SMP)]
    affinity: None,
};

static RROS_REALTIME_CLOCK_NAME: &CStr =
    unsafe { CStr::from_bytes_with_nul_unchecked("RROS_CLOCK_REALTIME_DEV\0".as_bytes()) };

pub static mut RROS_REALTIME_CLOCK: RrosClock = RrosClock {
    name: RROS_REALTIME_CLOCK_NAME,
    resolution: 1,
    gravity: RrosClockGravity {
        irq: CONFIG_RROS_LATENCY_IRQ,
        kernel: CONFIG_RROS_LATENCY_KERNEL,
        user: CONFIG_RROS_LATENCY_USER,
    },
    flags: RROS_CLONE_PUBLIC,
    ops: RrosClockOps {
        read: Some(read_realtime_clock),
        readcycles: Some(read_realtime_clock_cycles),
        set: None,
        programlocalshot: None,
        programremoteshot: None,
        setgravity: Some(set_coreclk_gravity),
        resetgravity: Some(reset_coreclk_gravity),
        adjust: Some(adjust_realtime_clock),
    },
    timerdata: 0 as *mut RrosTimerbase,
    master: 0 as *mut RrosClock,
    next: 0 as *mut ListHead,
    offset: 0,
    dispose: None,
    element: None,
    #[cfg(CONFIG_SMP)]
    affinity: None,
};

pub static mut CLOCK_LIST: List<*mut RrosClock> = List::<*mut RrosClock> {
    head: Node::<*mut RrosClock> {
        next: None,
        prev: None,
        value: 0 as *mut RrosClock,
    },
};

pub static mut RROS_CLOCK_FACTORY: SpinLock<factory::RrosFactory> = unsafe {
    SpinLock::new(factory::RrosFactory {
        name: unsafe { CStr::from_bytes_with_nul_unchecked("clock\0".as_bytes()) },
        nrdev: CONFIG_RROS_NR_CLOCKS,
        build: None,
        dispose: Some(clock_factory_dispose),
        attrs: None, //sysfs::attribute_group::new(),
        flags: factory::RrosFactoryType::Invalid,
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

pub struct RrosTimerFd {
    timer: Arc<SpinLock<RrosTimer>>,
    readers: RrosWaitQueue,
    poll_head: RrosPollHead,
    efile: RrosFile,
    ticked: bool,
}

impl RrosTimerFd {
    fn new() -> Self {
        Self {
            timer: Arc::try_new(unsafe { SpinLock::new(RrosTimer::new(0)) }).unwrap(),
            //FIXME: readers initiation is not sure
            readers: RrosWaitQueue::new(core::ptr::null_mut(), 0),
            poll_head: RrosPollHead::new(),
            efile: RrosFile::new(),
            ticked: false,
        }
    }
}

fn get_timer_value(timer: Arc<SpinLock<RrosTimer>>, value: &mut Itimerspec64) {
    let mut inner_timer_lock = timer.lock();
    let inner_timer: &mut RrosTimer = inner_timer_lock.deref_mut();
    value.it_interval = ktime_to_timespec64(inner_timer.interval);
    if rros_timer_is_running(inner_timer as *mut RrosTimer) {
        value.it_value.0.tv_sec = 0;
        value.it_value.0.tv_nsec = 0;
    } else {
        value.it_value = ktime_to_timespec64(rros_get_timer_delta(timer.clone()));
    }
}

fn set_timer_value(timer: Arc<SpinLock<RrosTimer>>, value: &Itimerspec64) -> Result<i32> {
    let start: KtimeT;
    let period: KtimeT;

    if value.it_value.0.tv_nsec == 0 && value.it_value.0.tv_sec == 0 {
        rros_stop_timer(timer.clone());
        return Ok(0);
    }

    period = timespec64_to_ktime(value.it_interval);
    start = timespec64_to_ktime(value.it_value);
    rros_start_timer(timer.clone(), start, period);
    return Ok(0);
}

// TODO: Can we use a static reference or something safe else?
pub fn rros_current_rq() -> *mut rros_rq {
    let current = rros_current();
    unsafe { (*(*current).locked_data().get()).rq.unwrap() }
}

pub fn double_timer_base_lock(tb1: *mut RrosTimerbase, tb2: *mut RrosTimerbase) {
    match tb1.cmp(&tb2) {
        Equal => unsafe { (*tb1).lock.raw_spin_lock() },
        Less => unsafe {
            (*tb1).lock.raw_spin_lock();
            (*tb2)
                .lock
                .raw_spin_lock_nested(bindings::SINGLE_DEPTH_NESTING);
        },
        Greater => unsafe {
            (*tb2).lock.raw_spin_lock();
            (*tb1)
                .lock
                .raw_spin_lock_nested(bindings::SINGLE_DEPTH_NESTING);
        },
    }
}

pub fn double_timer_base_unlock(tb1: *mut RrosTimerbase, tb2: *mut RrosTimerbase) {
    unsafe {
        (*tb1).lock.raw_spin_unlock();
        if tb1 != tb2 {
            (*tb2).lock.raw_spin_unlock();
        }
    }
}

// TODO: There are many global static references in the code that are used as raw pointers, such as
// `RrosClock`, `RrosTimerbase`, `RrosRq`. Maybe we can use references to avoid so many raw pointers.
// FYI: https://github.com/BUPT-OS/RROS/pull/41#discussion_r1680738528
pub fn rros_move_timer(
    timer: Arc<SpinLock<RrosTimer>>,
    clock: *mut RrosClock,
    mut rq: *mut rros_rq,
) {
    let cpu = get_clock_cpu(unsafe { &(*(*clock).get_master()) }, rros_rq_cpu(rq));
    rq = rros_cpu_rq(cpu);

    let mut flags: u64 = 0;
    let old_base = lock_timer_base(timer.clone(), &mut flags);

    if rros_timer_on_rq(timer.clone(), rq)
        && clock == unsafe { (*timer.locked_data().get()).get_clock() }
    {
        unlock_timer_base(old_base, flags);
        return;
    }
    let new_base = rros_percpu_timers(unsafe { &(*(*clock).get_master()) }, cpu);
    if unsafe { (*timer.locked_data().get()).get_status() & RROS_TIMER_RUNNING } != 0 {
        stop_timer_locked(timer.clone());
        unlock_timer_base(old_base, flags);
        let flags: u64 = hard_local_irq_save();
        double_timer_base_lock(old_base, new_base);

        unsafe {
            #[cfg(CONFIG_SMP)]
            (*timer.locked_data().get()).set_rq(rq);

            (*timer.locked_data().get()).set_base(new_base);
            (*timer.locked_data().get()).set_clock(clock);
        }
        rros_enqueue_timer(timer.clone(), unsafe { &mut (*new_base).q });
        if timer_at_front(timer.clone()) {
            rros_program_remote_tick(clock, rq);
        }
        double_timer_base_unlock(old_base, new_base);
        hard_local_irq_restore(flags);
    } else {
        unsafe {
            #[cfg(CONFIG_SMP)]
            (*timer.locked_data().get()).set_rq(rq);

            (*timer.locked_data().get()).set_base(new_base);
            (*timer.locked_data().get()).set_clock(clock);
        }
        unlock_timer_base(old_base, flags);
    }
}

#[cfg(CONFIG_SMP)]
fn pin_timer(timer: Arc<SpinLock<RrosTimer>>) {
    let flags = hard_local_irq_save();

    let this_rq = rros_current_rq();
    if unsafe { (*timer.locked_data().get()).get_rq() != this_rq } {
        rros_move_timer(
            timer.clone(),
            unsafe { (*timer.locked_data().get()).get_clock() },
            this_rq,
        );
    }

    hard_local_irq_restore(flags);
}

#[cfg(not(CONFIG_SMP))]
fn pin_timer(_timer: Arc<SpinLock<RrosTimer>>) {}

fn set_timerfd(
    timerfd: &RrosTimerFd,
    value: &mut Itimerspec64,
    ovalue: Itimerspec64,
) -> Result<i32> {
    get_timer_value(timerfd.timer.clone(), value);

    if rros_current() != core::ptr::null_mut() {
        pin_timer(timerfd.timer.clone());
    }

    set_timer_value(timerfd.timer.clone(), value)
}

fn new_timerfd(clock: RrosClock) -> Result<i32> {
    let timerfd: &mut RrosTimerFd = &mut RrosTimerFd::new();
    let mut filp: File = File {
        ptr: core::ptr::null_mut(),
    };
    let ret: i32;
    let fd: i32;

    let element: &RrosElement = unsafe { &(*clock.element.unwrap().as_ptr()) };
    let cdev: &Cdev = element.cdev.as_ref().unwrap();
    let fops: *const bindings::file_operations = unsafe { (*(cdev.0)).ops };
    let name: *const c_types::c_char = CStr::from_bytes_with_nul(b"[rros-timefd]\0")
        .unwrap()
        .as_ptr() as *const c_types::c_char;
    filp.ptr = unsafe {
        bindings::anon_inode_getfile(
            name,
            fops,
            timerfd as *mut RrosTimerFd as *mut c_void,
            bindings::O_RDWR as i32 | bindings::O_CLOEXEC as i32,
        )
    };

    //TODO:
    // rros_get_element(clock.element);
    unimplemented!()
}

fn clock_common_ioctl(clock: &mut RrosClock, cmd: u32, arg: usize) -> Result<i32> {
    let mut uts: Timespec64 = Timespec64::default();
    let u_uts: *mut Timespec64;
    let mut ts64: Timespec64 = Timespec64::new(0, 0);
    let mut ret: Result<i32> = Ok(0);

    match cmd {
        RROS_CLKIOC_GET_RES => {
            ts64 = ktime_to_timespec64(clock.resolution);
            uts.0.tv_nsec = ts64.0.tv_nsec;
            uts.0.tv_sec = ts64.0.tv_sec;
            u_uts = arg as *mut Timespec64;
            let mut usptr = unsafe {
                UserSlicePtr::new(
                    u_uts as *mut c_types::c_void,
                    core::mem::size_of::<Timespec64>(),
                )
                .writer()
            };
            if let Ok(_) = unsafe {
                usptr.write_raw(
                    &uts as *const Timespec64 as *const u8,
                    core::mem::size_of::<Timespec64>(),
                )
            } {
                ret = Ok(0)
            } else {
                ret = Err(Error::EFAULT);
            }
        }
        RROS_CLKIOC_GET_TIME => {
            ts64 = ktime_to_timespec64(clock.read());
            uts.0.tv_nsec = ts64.0.tv_nsec;
            uts.0.tv_sec = ts64.0.tv_sec;
            u_uts = arg as *mut Timespec64;
            let mut usptr = unsafe {
                UserSlicePtr::new(
                    u_uts as *mut c_types::c_void,
                    core::mem::size_of::<Timespec64>(),
                )
                .writer()
            };
            if let Ok(_) = unsafe {
                usptr.write_raw(
                    &uts as *const Timespec64 as *const u8,
                    core::mem::size_of::<Timespec64>(),
                )
            } {
                ret = Ok(0)
            } else {
                ret = Err(Error::EFAULT);
            }
        }
        RROS_CLKIOC_SET_TIME => {
            u_uts = arg as *mut Timespec64;
            let mut usptr = unsafe {
                UserSlicePtr::new(
                    u_uts as *mut c_types::c_void,
                    core::mem::size_of::<Timespec64>(),
                )
                .reader()
            };
            if let Err(_) = unsafe {
                usptr.read_raw(
                    &mut uts as *mut Timespec64 as *mut u8,
                    core::mem::size_of::<Timespec64>(),
                )
            } {
                return Err(Error::EFAULT);
            }
            if uts.0.tv_nsec as usize >= 1000_000_000 as usize {
                return Err(Error::EINVAL);
            }
            ts64.0.tv_nsec = uts.0.tv_nsec;
            ts64.0.tv_sec = uts.0.tv_sec;
            clock.set(timespec64_to_ktime(ts64));
        }
        _ => {
            ret = Err(Error::ENOTTY);
        }
    }

    ret
}

fn clock_ioctl(fbind: &RrosFileBinding, cmd: u32, arg: usize) -> Result<i32> {
    unimplemented!();
    let clock = unsafe { &mut *((*fbind.element).pointer as *mut RrosClock) };
    let u_fd: i32;
    let mut ret: Result<i32>;

    //TODO:
    match cmd {
        RROS_CLKIOC_NEW_TIMER => {}
        _ => {
            ret = clock_common_ioctl(clock, cmd, arg);
        }
    }

    ret
}

extern "C" fn restart_clock_sleep(param: *mut bindings::restart_block) -> i64 {
    -(bindings::EINVAL as i64)
}

fn clock_sleep(clock: &RrosClock, ts64: Timespec64) -> Result<i32> {
    let mut restart: bindings::restart_block;
    let timeout: KtimeT;
    let rem: KtimeT;
    {
        let mut curr = unsafe { (*rros_current()).lock() };

        if curr.local_info & T_SYSRST != 0 {
            curr.local_info &= !T_SYSRST;
            restart = unsafe { (*Task::current_ptr()).restart_block };
            if restart.fn_.unwrap() != restart_clock_sleep {
                return Err(Error::EINTR);
            }
            unsafe {
                timeout = restart.__bindgen_anon_1.nanosleep.expires as i64;
            }
        } else {
            timeout = timespec64_to_ktime(ts64);
        }
    }
    if let Ok(_) = rros_delay(timeout, RrosTmode::RrosAbs, clock) {
        return Ok(0);
    } else {
        if Task::current().signal_pending() {
            restart = unsafe { (*Task::current_ptr()).restart_block };
            unsafe {
                restart.__bindgen_anon_1.nanosleep.expires = timeout as u64;
            }
            restart.fn_ = Some(restart_clock_sleep);
            unsafe { (*rros_current()).lock().local_info |= T_SYSRST };
            return Err(Error::ERESTARTSYS);
        }
    }

    return Err(Error::EINTR);
}

fn clock_oob_ioctl(fbind: &RrosFileBinding, cmd: u32, arg: usize) -> Result<i32> {
    //TODO:
    let clock = unsafe { &mut *((*fbind.element).pointer as *mut RrosClock) };
    let u_uts: *mut Timespec64;
    let mut uts: Timespec64 = Timespec64::default();
    let mut ret: Result<i32> = Ok(0);

    match cmd {
        RROS_CLKIOC_SLEEP => {
            u_uts = arg as *mut Timespec64;
            let mut usrptr = unsafe {
                UserSlicePtr::new(
                    u_uts as *mut c_types::c_void,
                    core::mem::size_of::<Timespec64>(),
                )
                .reader()
            };
            if let Err(_) = unsafe {
                usrptr.read_raw(
                    &mut uts as *mut Timespec64 as *mut u8,
                    core::mem::size_of::<Timespec64>(),
                )
            } {
                return Err(Error::EFAULT);
            }
            if uts.0.tv_sec < 0 {
                return Err(Error::EINVAL);
            }
            if uts.0.tv_nsec as usize >= 1000_000_000 as usize {
                return Err(Error::EINVAL);
            }
            let ts: Timespec64 = Timespec64::new(uts.0.tv_sec, uts.0.tv_nsec);
            //TODO: clock_sleep is not implement
            ret = clock_sleep(clock, ts);
        }
        _ => {
            clock_common_ioctl(clock, cmd, arg);
        }
    }

    ret
}
pub struct ClockOps;

impl FileOpener<u8> for ClockOps {
    fn open(inode: &u8, file: &kernel::file::File) -> kernel::Result<Self::Wrapper> {
        let mut flag: u64 = 0;
        let mut mark = false;
        //TODO: restruct RrosElement to use rros_open_element instead of different
        //implement in different fileopener.
        //The key is using inode and file to get element.
        let minor =
            unsafe { rust_helper_minor((*(inode as *const _ as *const bindings::inode)).i_rdev) };
        let e = match minor {
            0 => unsafe { RROS_MONO_CLOCK.element.as_ref().unwrap().clone() },
            _ => unsafe { RROS_REALTIME_CLOCK.element.as_ref().unwrap().clone() },
        };

        let element = unsafe { &mut *e.as_ref().as_ptr() };

        unsafe {
            rust_helper_rcu_read_lock();
        }
        flag = element.ref_lock.irq_lock_noguard();

        if element.zombie {
            mark = true;
        } else {
            element.refs += 1;
        }

        element.ref_lock.irq_unlock_noguard(flag);
        unsafe {
            rust_helper_rcu_read_unlock();
        }

        if mark {
            return Err(Error::ESTALE);
        }

        factory::bind_file_to_element(file.ptr, e.clone())?;

        Ok(unsafe { Box::from_raw((*(file.ptr)).private_data as *mut RrosFileBinding) })
    }
}

impl FileOperations for ClockOps {
    kernel::declare_file_operations!(ioctl, oob_ioctl);

    type Wrapper = Box<RrosFileBinding>;

    fn ioctl(
        _this: &<<Self::Wrapper as kernel::types::PointerWrapper>::Borrowed as core::ops::Deref>::Target,
        _file: &File,
        _cmd: &mut IoctlCommand,
    ) -> Result<i32> {
        clock_ioctl(_this, _cmd.cmd, _cmd.arg)
    }

    fn oob_ioctl(
        _this: &<<Self::Wrapper as kernel::types::PointerWrapper>::Borrowed as core::ops::Deref>::Target,
        _file: &File,
        _cmd: &mut IoctlCommand,
    ) -> Result<i32> {
        clock_oob_ioctl(_this, _cmd.cmd, _cmd.arg)
    }

    fn release(_obj: Self::Wrapper, _file: &File) {
        let mut element = unsafe { &mut *_obj.element };
        rros_release_file(&mut _obj.rfile.as_ref().borrow_mut());
        {
            //TODO: restruct RrosFileBinding to use method instead of this block
            let flag = element.ref_lock.irq_lock_noguard();

            //TODO: RROS_WARN_ON
            if element.refs == 0 {
                element.ref_lock.irq_unlock_noguard(flag);
                return;
            }

            //only two clock whose refs will not be 0
            element.refs -= 1;

            element.ref_lock.irq_unlock_noguard(flag);
        }
    }
}

pub fn clock_factory_dispose(ele: factory::RrosElement) {}

fn timer_needs_enqueuing(timer: *mut RrosTimer) -> bool {
    unsafe {
        return ((*timer).get_status()
            & (RROS_TIMER_PERIODIC
                | RROS_TIMER_DEQUEUED
                | RROS_TIMER_RUNNING
                | RROS_TIMER_KILLED))
            == (RROS_TIMER_PERIODIC | RROS_TIMER_DEQUEUED | RROS_TIMER_RUNNING);
    }
}

// `rq` related tests haven't been tested, other tests passed.
pub fn do_clock_tick(clock: &mut RrosClock, tmb: *mut RrosTimerbase) {
    let rq = this_rros_rq();
    // #[cfg(CONFIG_RROS_DEBUG_CORE)]
    // if hard_irqs_disabled() == false {
    //     hard_local_irq_disable();
    // }
    let mut tq = unsafe { &mut (*tmb).q };
    //unsafe{(*tmb).lock.lock();}

    unsafe {
        (*rq).add_local_flags(RQ_TIMER);
    }

    let mut now = clock.read();

    // unsafe{
    //     if (*tmb).q.is_empty() == true {
    //         // tick
    //         tick::proxy_set_next_ktime(1000000, 0 as *mut bindings::clock_event_device);
    //     }
    // }

    unsafe {
        while tq.is_empty() == false {
            let mut timer = tq.get_head().unwrap().value.clone();
            let date = (*timer.locked_data().get()).get_date();
            if now < date {
                break;
            }

            rros_dequeue_timer(timer.clone(), tq);

            rros_account_timer_fired(timer.clone());
            (*timer.locked_data().get()).add_status(RROS_TIMER_FIRED);
            let timer_addr = timer.locked_data().get();

            let inband_timer_addr = (*rq).get_inband_timer().locked_data().get();
            if (timer_addr == inband_timer_addr) {
                (*rq).add_local_flags(RQ_TPROXY);
                (*rq).change_local_flags(!RQ_TDEFER);
                continue;
            }
            let handler = (*timer.locked_data().get()).get_handler();
            let c_ref = timer.locked_data().get();
            handler(c_ref);
            now = clock.read();
            let var_timer_needs_enqueuing = timer_needs_enqueuing(timer.locked_data().get());
            if var_timer_needs_enqueuing == true {
                loop {
                    let periodic_ticks = (*timer.locked_data().get()).get_periodic_ticks() + 1;
                    (*timer.locked_data().get()).set_periodic_ticks(periodic_ticks);
                    rros_update_timer_date(timer.clone());

                    let date = (*timer.locked_data().get()).get_date();
                    if date >= now {
                        break;
                    }
                }

                if (rros_timer_on_rq(timer.clone(), rq)) {
                    rros_enqueue_timer(timer.clone(), tq);
                }

                pr_debug!("now is {}", now);
                // pr_debug!("date is {}",timer.lock().get_date());
            }
        }
    }
    unsafe { (*rq).change_local_flags(!RQ_TIMER) };

    rros_program_local_tick(clock as *mut RrosClock);

    //raw_spin_unlock(&tmb->lock);
}

pub struct RrosCoreTick;

impl clockchips::CoreTick for RrosCoreTick {
    fn core_tick(dummy: clockchips::ClockEventDevice) {
        // pr_debug!("in rros_core_tick");
        let this_rq = this_rros_rq();
        //	if (RROS_WARN_ON_ONCE(CORE, !is_rros_cpu(rros_rq_cpu(this_rq))))
        // pr_info!("in rros_core_tick");
        unsafe {
            do_clock_tick(&mut RROS_MONO_CLOCK, rros_this_cpu_timers(&RROS_MONO_CLOCK));

            let rq_has_tproxy = ((*this_rq).local_flags & RQ_TPROXY != 0x0);
            let assd = (*(*this_rq).get_curr().locked_data().get()).state;
            let curr_state_is_t_root = (assd & (T_ROOT as u32) != 0x0);
            // This `if` won't enter, so there is a problem.
            // let a = ((*this_rq).local_flags & RQ_TPROXY != 0x0);
            // if rq_has_tproxy  {
            //     pr_debug!("in rros_core_tick");
            //     pr_debug!("in rros_core_tick");
            //     pr_debug!("in rros_core_tick");
            //     pr_debug!("in rros_core_tick");
            //     pr_debug!("in rros_core_tick");
            //     pr_debug!("in rros_core_tick");
            //     pr_debug!("in rros_core_tick");
            //     pr_debug!("in rros_core_tick");
            // }
            // let b = ((*this_rq).get_curr().lock().deref_mut().state & (T_ROOT as u32) != 0x0);

            // if curr_state_is_t_root  {
            //     pr_debug!("in rros_core_tick");
            //     pr_debug!("in rros_core_tick");
            //     pr_debug!("in rros_core_tick");
            //     pr_debug!("in rros_core_tick");
            //     pr_debug!("in rros_core_tick");
            //     pr_debug!("in rros_core_tick");
            //     pr_debug!("in rros_core_tick");
            //     pr_debug!("in rros_core_tick");
            // }
            if rq_has_tproxy && curr_state_is_t_root {
                rros_notify_proxy_tick(this_rq);
            }
        }
    }
}

fn init_clock(clock: *mut RrosClock, master: *mut RrosClock) -> Result<usize> {
    let mut ret = Ok(0);
    // unsafe{
    //     if (*clock).element.is_none(){
    //         return Err(kernel::Error::EINVAL);
    //     }
    // }
    unsafe {
        ret = factory::rros_init_element(
            (*clock).element.as_ref().unwrap().clone(),
            &mut RROS_CLOCK_FACTORY,
            (*clock).flags & RROS_CLONE_PUBLIC,
        );
    }

    if let Err(_) = ret {
        return ret;
    }

    unsafe {
        (*clock).master = master;
    }

    unsafe {
        ret = factory::rros_create_core_element_device(
            (*clock).element.as_ref().unwrap().clone(),
            &mut RROS_CLOCK_FACTORY,
            (*clock).name,
        );
    }

    if let Err(_) = ret {
        //TODO: destroy element
        return ret;
    }

    unsafe {
        CLOCKLIST_LOCK.lock();
        CLOCK_LIST.add_head(clock);
        CLOCKLIST_LOCK.unlock();
    }

    Ok(0)
}

fn rros_init_slave_clock(clock: &mut RrosClock, master: &mut RrosClock) -> Result<usize> {
    premmpt::running_inband()?;

    // TODO: Check if there is a problem here, even if the timer can run.
    #[cfg(CONFIG_SMP)]
    {
        clock.affinity = master.affinity.clone();
    }

    clock.timerdata = master.get_timerdata_addr();
    clock.offset = clock.read() - master.read();
    init_clock(clock as *mut RrosClock, master as *mut RrosClock)?;
    Ok(0)
}

fn rros_init_clock(clock: &mut RrosClock, affinity: &CpumaskT) -> Result<usize> {
    premmpt::running_inband()?;

    #[cfg(CONFIG_SMP)]
    {
        if clock.affinity.is_none() {
            clock.affinity = Some(CpumaskT::from_int(0));
        }
        if affinity.cpumask_empty().is_ok() {
            let clock_affinity = clock.affinity.as_mut().unwrap();
            clock_affinity.cpumask_clear();
            clock_affinity.cpumask_set_cpu(unsafe { RROS_OOB_CPUS.cpumask_first() as u32 });
        } else {
            clock.affinity = Some(affinity.clone() & unsafe { RROS_OOB_CPUS.clone() });
            if clock.affinity.as_ref().unwrap().cpumask_empty().is_ok() {
                return Err(Error::EINVAL);
            }
        }
    }

    // 8 byte alignment
    let tmb = percpu::alloc_per_cpu(
        size_of::<RrosTimerbase>() as usize,
        align_of::<RrosTimerbase>() as usize,
    ) as *mut RrosTimerbase;
    if tmb == 0 as *mut RrosTimerbase {
        return Err(kernel::Error::ENOMEM);
    }
    clock.timerdata = tmb;

    for cpu in online_cpus() {
        let mut tmb = rros_percpu_timers(clock, cpu as i32);
        unsafe {
            raw_spin_lock_init(&mut (*tmb).lock);
        }
    }

    clock.offset = 0;
    let ret = init_clock(clock as *mut RrosClock, clock as *mut RrosClock);
    if let Err(_) = ret {
        percpu::free_per_cpu(clock.get_timerdata_addr() as *mut u8);
        return ret;
    }
    Ok(0)
}

pub fn rros_clock_init() -> Result<usize> {
    let pinned = unsafe { Pin::new_unchecked(&mut CLOCKLIST_LOCK) };
    spinlock_init!(pinned, "CLOCKLIST_LOCK");
    unsafe {
        RROS_MONO_CLOCK.reset_gravity();
        RROS_REALTIME_CLOCK.reset_gravity();

        let mut element: RrosElement = RrosElement::new()?;
        element.pointer = &mut RROS_MONO_CLOCK as *mut _ as *mut u8;
        RROS_MONO_CLOCK.element = Some(Rc::try_new(RefCell::new(element)).unwrap());
        let mut element: RrosElement = RrosElement::new()?;
        element.pointer = &mut RROS_REALTIME_CLOCK as *mut _ as *mut u8;
        RROS_REALTIME_CLOCK.element = Some(Rc::try_new(RefCell::new(element)).unwrap());

        rros_init_clock(&mut RROS_MONO_CLOCK, &RROS_OOB_CPUS)?;
    }
    let ret = unsafe { rros_init_slave_clock(&mut RROS_REALTIME_CLOCK, &mut RROS_MONO_CLOCK) };
    if let Err(_) = ret {
        //rros_put_element(&rros_mono_clock.element);
    }
    pr_debug!("clock init success!");
    Ok(0)
}

pub fn rros_read_clock(clock: &RrosClock) -> KtimeT {
    let clock_add = clock as *const RrosClock;
    let mono_add = unsafe { &RROS_MONO_CLOCK as *const RrosClock };

    if (clock_add == mono_add) {
        return rros_ktime_monotonic();
    }

    clock.ops.read.unwrap()(&clock)
}

pub fn rros_ktime_monotonic() -> KtimeT {
    timekeeping::ktime_get_mono_fast_ns()
}
