#![allow(warnings, unused)]
use crate::{clock::*, lock::*, sched::*, stat::*, tick::*, timeout::*};

use core::ops::DerefMut;
use kernel::{
    bindings, c_str, double_linked_list::*, ktime::*, percpu_defs, prelude::*, spinlock_init,
    str::CStr, sync::Lock, sync::SpinLock,
};
pub type RrosRq = rros_rq;

/* Timer status */
pub const RROS_TIMER_DEQUEUED: i32 = 0x00000001;
pub const RROS_TIMER_KILLED: i32 = 0x00000002;
pub const RROS_TIMER_PERIODIC: i32 = 0x00000004;
pub const RROS_TIMER_FIRED: i32 = 0x00000010;
pub const RROS_TIMER_RUNNING: i32 = 0x00000020;
pub const RROS_TIMER_KGRAVITY: i32 = 0x00000040;
pub const RROS_TIMER_UGRAVITY: i32 = 0x00000080;
pub const RROS_TIMER_IGRAVITY: i32 = 0; /* most conservative */
pub const RROS_TIMER_GRAVITY_MASK: i32 = (RROS_TIMER_KGRAVITY | RROS_TIMER_UGRAVITY);
pub const RROS_TIMER_INIT_MASK: i32 = RROS_TIMER_GRAVITY_MASK;
pub struct RrosTimerbase {
    pub lock: SpinLock<i32>,
    pub q: List<Arc<SpinLock<RrosTimer>>>,
}

pub fn rros_this_cpu_timers(clock: &RrosClock) -> *mut RrosTimerbase {
    unsafe { percpu_defs::raw_cpu_ptr(clock.get_timerdata_addr() as *mut u8) as *mut RrosTimerbase }
}

pub fn rros_timer_null_handler(timer: *mut RrosTimer) {
    pr_debug!("i am in rros_timer_null_handler");
}

pub struct RrosTimer {
    clock: *mut RrosClock,
    date: KtimeT,
    //adjlink: list_head,// Used when adjusting
    status: i32,
    pub interval: KtimeT, /* 0 == oneshot */
    pub start_date: KtimeT,
    pexpect_ticks: u64, /* periodic release date */
    periodic_ticks: u64,
    base: *mut RrosTimerbase,
    handler: fn(*mut RrosTimer),
    name: &'static CStr,
    #[cfg(CONFIG_RROS_RUNSTATS)]
    scheduled: RrosCounter,
    #[cfg(CONFIG_RROS_RUNSTATS)]
    fired: RrosCounter,
    #[cfg(CONFIG_SMP)]
    rq: *mut RrosRq,
    pub thread: Option<Arc<SpinLock<RrosThread>>>,
}

impl RrosTimer {
    pub fn new(date: KtimeT) -> Self {
        RrosTimer {
            clock: 0 as *mut RrosClock, //?
            date: date,
            status: 0,
            interval: 0,
            start_date: 123,
            pexpect_ticks: 0,
            periodic_ticks: 0,
            base: 0 as *mut RrosTimerbase,
            handler: rros_timer_null_handler, //?
            name: c_str!(""),                 //?
            #[cfg(CONFIG_SMP)]
            rq: 0 as *mut RrosRq,
            thread: None,
        }
    }

    pub fn set_clock(&mut self, clock: *mut RrosClock) {
        self.clock = clock;
    }

    pub fn set_date(&mut self, date: KtimeT) {
        self.date = date;
    }

    pub fn set_status(&mut self, status: i32) {
        self.status = status;
    }

    pub fn add_status(&mut self, status: i32) {
        self.status |= status;
    }

    pub fn change_status(&mut self, status: i32) {
        self.status &= status;
    }

    pub fn set_interval(&mut self, interval: KtimeT) {
        self.interval = interval;
    }

    pub fn set_start_date(&mut self, start_date: KtimeT) {
        self.start_date = start_date;
    }

    pub fn set_pexpect_ticks(&mut self, value: u64) {
        self.pexpect_ticks = value;
    }

    pub fn set_periodic_ticks(&mut self, value: u64) {
        self.periodic_ticks = value;
    }

    pub fn set_base(&mut self, base: *mut RrosTimerbase) {
        self.base = base;
    }

    pub fn set_handler(&mut self, handler: fn(*mut RrosTimer)) {
        self.handler = handler;
    }

    pub fn set_name(&mut self, name: &'static CStr) {
        self.name = name;
    }

    pub fn set_rq(&mut self, rq: *mut RrosRq) {
        self.rq = rq;
    }

    pub fn get_date(&self) -> KtimeT {
        self.date
    }

    pub fn get_status(&self) -> i32 {
        self.status
    }

    pub fn get_periodic_ticks(&self) -> u64 {
        self.periodic_ticks
    }

    pub fn get_pexpect_ticks(&self) -> u64 {
        self.pexpect_ticks
    }

    pub fn get_start_date(&self) -> KtimeT {
        self.start_date
    }

    pub fn get_interval(&self) -> KtimeT {
        self.interval
    }

    // pub fn get_handler(&self) -> Option<fn(&RrosTimer)> {
    //     if self.handler.is_none() {
    //         return None;
    //     }
    //     Some(self.handler.unwrap())
    // }

    pub fn get_handler(&self) -> fn(*mut RrosTimer) {
        self.handler
    }

    pub fn get_clock(&self) -> *mut RrosClock {
        self.clock
    }

    pub fn get_base(&self) -> *mut RrosTimerbase {
        self.base
    }

    #[cfg(CONFIG_RROS_RUNSTATS)]
    pub fn get_scheduled<'a>(&mut self) -> &'a mut RrosAccount {
        self.scheduled
    }

    #[cfg(CONFIG_RROS_RUNSTATS)]
    pub fn get_fired<'a>(&mut self) -> &'a mut RrosAccount {
        self.fired
    }

    #[cfg(CONFIG_SMP)]
    pub fn get_rq(&self) -> *mut RrosRq {
        self.rq
    }

    #[cfg(not(CONFIG_SMP))]
    pub fn get_rq(&self) -> *mut RrosRq {
        this_rros_rq();
    }

    pub fn is_running(&self) -> bool {
        (self.get_status() & RROS_TIMER_RUNNING) != 0
    }

    pub fn is_periodic(&self) -> bool {
        (self.get_status() & RROS_TIMER_PERIODIC) != 0
    }

    pub fn thread(&self) -> Option<Arc<SpinLock<RrosThread>>> {
        self.thread.clone()
    }
}
#[cfg(CONFIG_RROS_TIMER_SCALABLE)]
pub fn rros_insert_tnode(tq: &mut List<Arc<SpinLock<RrosTimer>>>, timer: Arc<SpinLock<RrosTimer>>) {
    todo!();
}

#[cfg(not(CONFIG_RROS_TIMER_SCALABLE))]
pub fn rros_insert_tnode(tq: &mut List<Arc<SpinLock<RrosTimer>>>, timer: Arc<SpinLock<RrosTimer>>) {
    let mut l = tq.len();
    while l >= 1 {
        let x = tq.get_by_index(l).unwrap().value.clone();
        let x_date = unsafe { (*x.locked_data().get()).get_date() };
        let timer_date = unsafe { (*timer.locked_data().get()).get_date() };
        if x_date <= timer_date {
            tq.enqueue_by_index(l, timer.clone());
            return;
        }
        l = l - 1;
    }
    tq.add_head(timer.clone());
}

pub fn rros_get_timer_gravity(timer: Arc<SpinLock<RrosTimer>>) -> KtimeT {
    let status = unsafe { (*timer.locked_data().get()).get_status() };
    if status & RROS_TIMER_KGRAVITY != 0 {
        return unsafe { (*(*timer.locked_data().get()).get_clock()).get_gravity_kernel() };
    }

    if status & RROS_TIMER_UGRAVITY != 0 {
        return unsafe { (*(*timer.locked_data().get()).get_clock()).get_gravity_user() };
    }

    return unsafe { (*(*timer.locked_data().get()).get_clock()).get_gravity_irq() };
}

pub fn rros_update_timer_date(timer: Arc<SpinLock<RrosTimer>>) {
    unsafe {
        let start_date = (*timer.locked_data().get()).get_start_date();
        let periodic_ticks = (*timer.locked_data().get()).get_periodic_ticks();
        let interval = ktime_to_ns((*timer.locked_data().get()).get_interval());
        let gravity = ktime_to_ns(rros_get_timer_gravity(timer.clone()));
        (*timer.locked_data().get()).set_date(ktime_add_ns(
            start_date,
            ((periodic_ticks as i64 * interval) - gravity) as u64,
        ));
    }
}

pub fn rros_get_timer_next_date(timer: Arc<SpinLock<RrosTimer>>) -> KtimeT {
    let start_date = unsafe { (*timer.locked_data().get()).get_start_date() };
    let periodic_ticks = unsafe { (*timer.locked_data().get()).get_periodic_ticks() };
    let interval = ktime_to_ns(unsafe { (*timer.locked_data().get()).get_interval() });
    return ktime_add_ns(start_date, (periodic_ticks as i64 * interval) as u64);
}

#[cfg(CONFIG_RROS_RUNSTATS)]
pub fn rros_reset_timer_stats(timer: Arc<SpinLock<RrosTimer>>) {
    // Insufficient conditions, so no modification
    timer.lock().get_scheduled().set_counter(0);
    timer.lock().get_fired().set_counter(0);
}

#[cfg(CONFIG_RROS_RUNSTATS)]
pub fn rros_account_timer_scheduled(timer: Arc<SpinLock<RrosTimer>>) {
    timer.lock().get_scheduled().inc_counter();
}

#[cfg(CONFIG_RROS_RUNSTATS)]
pub fn rros_account_timer_fired(timer: Arc<SpinLock<RrosTimer>>) {
    timer.lock().get_fired().inc_counter();
}

#[cfg(not(CONFIG_RROS_RUNSTATS))]
pub fn rros_reset_timer_stats(timer: Arc<SpinLock<RrosTimer>>) {} // Insufficient conditions, so no modification

#[cfg(not(CONFIG_RROS_RUNSTATS))]
pub fn rros_account_timer_scheduled(timer: Arc<SpinLock<RrosTimer>>) {}

#[cfg(not(CONFIG_RROS_RUNSTATS))]
pub fn rros_account_timer_fired(timer: Arc<SpinLock<RrosTimer>>) {}

pub fn rros_timer_deactivate(timer: Arc<SpinLock<RrosTimer>>) -> bool {
    let mut heading = true;
    let tmb = unsafe { (*timer.locked_data().get()).get_base() };
    let status = unsafe { (*timer.locked_data().get()).get_status() };
    if status & RROS_TIMER_DEQUEUED != 0 {
        heading = timer_at_front(timer.clone());
        unsafe { rros_dequeue_timer(timer.clone(), &mut (*tmb).q) };
    }

    unsafe { (*timer.locked_data().get()).change_status(!(RROS_TIMER_FIRED | RROS_TIMER_RUNNING)) };

    return heading;
}

#[cfg(CONFIG_SMP)]
pub fn rros_timer_on_rq(timer: Arc<SpinLock<RrosTimer>>, rq: *mut RrosRq) -> bool {
    unsafe { (*timer.locked_data().get()).get_rq() == rq }
}

#[cfg(not(CONFIG_SMP))]
pub fn rros_timer_on_rq(timer: Arc<SpinLock<RrosTimer>>, rq: *mut RrosRq) -> bool {
    return true;
}

pub fn stop_timer_locked(timer: Arc<SpinLock<RrosTimer>>) {
    // let timer_lock = timer.lock();
    let is_running = unsafe { (*timer.locked_data().get()).is_running() };
    if is_running {
        let heading = rros_timer_deactivate(timer.clone());
        if heading && rros_timer_on_rq(timer.clone(), this_rros_rq()) {
            let clock = unsafe { (*timer.locked_data().get()).get_clock() };
            rros_program_local_tick(clock);
        }
    }
}

pub fn __rros_stop_timer(timer: Arc<SpinLock<RrosTimer>>) {
    let mut flags: u64 = 0;
    let base: *mut RrosTimerbase = lock_timer_base(timer.clone(), &mut flags);
    stop_timer_locked(timer);
    unlock_timer_base(base, flags);
}

pub fn rros_stop_timer(timer: Arc<SpinLock<RrosTimer>>) {
    unsafe {
        let is_running = (*timer.locked_data().get()).is_running();
        if is_running {
            __rros_stop_timer(timer.clone());
        }
    }
}

#[cfg(CONFIG_SMP)]
pub fn lock_timer_base(timer: Arc<SpinLock<RrosTimer>>, flags: &mut u64) -> *mut RrosTimerbase {
    unsafe {
        let mut base = (*timer.locked_data().get()).get_base();
        while true {
            base = (*timer.locked_data().get()).get_base();
            *flags = unsafe { (*base).lock.irq_lock_noguard() };
            let base2 = (*timer.locked_data().get()).get_base();
            if (base == base2) {
                break;
            }
            unsafe {
                (*base).lock.irq_unlock_noguard(*flags);
            }
        }
        base
    }
}

#[cfg(not(CONFIG_SMP))]
pub fn lock_timer_base(timer: Arc<SpinLock<RrosTimer>>, flags: &mut u64) -> *mut RrosTimerbase {
    pr_err!("!!!!!!!!!!!! this is wrong. lock_timer_base");
    unsafe { (*timer.locked_data().get()).get_base() }
}

#[cfg(CONFIG_SMP)]
pub fn unlock_timer_base(base: *mut RrosTimerbase, flags: u64) {
    unsafe {
        (*base).lock.irq_unlock_noguard(flags);
    }
}

#[cfg(not(CONFIG_SMP))]
pub fn unlock_timer_base(_base: *mut RrosTimerbase, _flags: u64) {
    pr_err!("!!!!!!!!!!!! this is wrong. lock_timer_base");
}

pub fn rros_dequeue_timer(
    timer: Arc<SpinLock<RrosTimer>>,
    tq: &mut List<Arc<SpinLock<RrosTimer>>>,
) {
    // pr_debug!("len tq is {}", tq.len());
    let timer_addr = unsafe { timer.clone().locked_data().get() };
    // pr_debug!("the run timer add is {:p}", timer_addr);
    unsafe {
        for i in 1..=tq.len() {
            let mut _x = tq.get_by_index(i).unwrap().value.clone();
            let x = _x.locked_data().get();
            if x == timer_addr {
                tq.dequeue(i);
                break;
            }
        }
    }
    unsafe {
        (*timer.locked_data().get()).add_status(RROS_TIMER_DEQUEUED);
    }
}

pub fn rros_get_timer_expiry(timer: Arc<SpinLock<RrosTimer>>) -> KtimeT {
    let date = unsafe { (*timer.locked_data().get()).get_date() };
    let gravity = rros_get_timer_gravity(timer.clone());
    return ktime_add(date, gravity);
}

pub fn __rros_get_timer_delta(timer: Arc<SpinLock<RrosTimer>>) -> KtimeT {
    let expiry = rros_get_timer_expiry(timer.clone());
    let now = unsafe { (*timer.lock().get_clock()).read() };
    if expiry <= now {
        return ktime_set(0, 1);
    }

    return ktime_sub(expiry, now);
}

pub fn rros_get_timer_delta(timer: Arc<SpinLock<RrosTimer>>) -> KtimeT {
    let timer_clone = timer.clone();
    let is_running = unsafe { (*timer_clone.locked_data().get()).is_running() };
    if is_running == false {
        return RROS_INFINITE;
    }
    return __rros_get_timer_delta(timer.clone());
}

pub fn rros_percpu_timers(clock: &RrosClock, cpu: i32) -> *mut RrosTimerbase {
    unsafe {
        percpu_defs::per_cpu_ptr(clock.get_timerdata_addr() as *mut u8, cpu) as *mut RrosTimerbase
    }
}

#[cfg(CONFIG_SMP)]
pub fn get_clock_cpu(clock: &RrosClock, cpu: i32) -> i32 {
    if clock
        .affinity
        .as_ref()
        .unwrap()
        .cpumask_test_cpu(cpu as u32)
    {
        return cpu;
    }
    clock.affinity.as_ref().unwrap().cpumask_first()
}

#[cfg(not(CONFIG_SMP))]
pub fn get_clock_cpu(clock: &RrosClock, cpu: i32) -> i32 {
    return 0;
}

use alloc::rc::Rc;
use core::borrow::BorrowMut;
use core::cell::RefCell;

pub fn rros_init_timer_on_rq(
    timer: Arc<SpinLock<RrosTimer>>,
    clock: &mut RrosClock,
    handler: Option<fn(*mut RrosTimer)>,
    rq: *mut RrosRq,
    name: &'static CStr,
    flags: i32,
) {
    timer.lock().set_date(RROS_INFINITE);
    timer
        .lock()
        .set_status(RROS_TIMER_DEQUEUED | (flags & RROS_TIMER_INIT_MASK));
    //timer.set_handler(handler);
    if handler.is_some() {
        timer.lock().set_handler(handler.unwrap());
    }
    timer.lock().set_interval(RROS_INFINITE);

    let cpu = if rq.is_null() {
        unsafe { RROS_CPU_AFFINITY.cpumask_first() }
    } else {
        unsafe { get_clock_cpu(&(*(clock.get_master())), rros_rq_cpu(rq)) }
    };

    #[cfg(CONFIG_SMP)]
    timer.lock().set_rq(rros_cpu_rq(cpu));

    timer.lock().set_base(rros_percpu_timers(clock, cpu));
    timer.lock().set_clock(clock as *mut RrosClock);
    //timer.set_name();
    rros_reset_timer_stats(timer.clone());
}

pub fn program_timer(timer: Arc<SpinLock<RrosTimer>>, tq: &mut List<Arc<SpinLock<RrosTimer>>>) {
    rros_enqueue_timer(timer.clone(), tq);
    let rq = unsafe { (*timer.locked_data().get()).get_rq() };
    let local_flags = unsafe { (*rq).local_flags };
    if (local_flags & RQ_TSTOPPED) == 0 && timer_at_front(timer.clone()) == false {
        return;
    }
    let clock = unsafe { (*timer.locked_data().get()).get_clock() };
    if rq != this_rros_rq() {
        rros_program_remote_tick(clock, rq);
    } else {
        rros_program_local_tick(clock);
    }
}

pub fn rros_start_timer(timer: Arc<SpinLock<RrosTimer>>, value: KtimeT, interval: KtimeT) {
    // pr_debug!("yinyongcishu is {}",Arc::strong_count(&timer));
    // pr_debug!("rros_start_timer: 1");
    // pr_debug!("the start timer{:?} {:?}", value, interval);
    unsafe {
        let mut flags = 0;
        let mut tmb = lock_timer_base(timer.clone(), &mut flags);
        // let mut tmb = (*timer.locked_data().get()).get_base();
        let status = (*timer.locked_data().get()).get_status();
        if status & RROS_TIMER_DEQUEUED == 0 {
            unsafe { rros_dequeue_timer(timer.clone(), &mut (*tmb).q) };
        }
        (*timer.locked_data().get()).change_status(!(RROS_TIMER_FIRED | RROS_TIMER_PERIODIC));
        let date = ktime_sub(value, unsafe {
            (*(*timer.locked_data().get()).get_clock()).get_offset()
        });
        let gravity = rros_get_timer_gravity(timer.clone());
        (*timer.locked_data().get()).set_date(ktime_sub(date, gravity));
        (*timer.locked_data().get()).set_interval(RROS_INFINITE);
        if timeout_infinite(interval) == false {
            (*timer.locked_data().get()).set_interval(interval);
            (*timer.locked_data().get()).set_start_date(value);
            (*timer.locked_data().get()).set_pexpect_ticks(0);
            (*timer.locked_data().get()).set_periodic_ticks(0);
            (*timer.locked_data().get()).add_status(RROS_TIMER_PERIODIC);
        }

        (*timer.locked_data().get()).add_status(RROS_TIMER_RUNNING);
        // pr_debug!("rros_start_timer: 2");
        unsafe { program_timer(timer.clone(), &mut (*tmb).q) };
        unlock_timer_base(tmb, flags);
    }
}

pub fn timer_at_front(timer: Arc<SpinLock<RrosTimer>>) -> bool {
    unsafe {
        let tmb = (*timer.locked_data().get()).get_base();
        unsafe {
            if (*tmb).q.is_empty() {
                return false;
            }
        }
        let mut _head = unsafe { (*tmb).q.get_head().unwrap().value.clone() };
        let head = _head.locked_data().get();
        let timer_addr = timer.clone().locked_data().get();
        if head == timer_addr {
            return true;
        }
        unsafe {
            if (*tmb).q.len() < 2 {
                return false;
            }
        }
        let local_flags = unsafe { (*(*timer.locked_data().get()).get_rq()).local_flags };
        if (local_flags & RQ_TDEFER) != 0x0 {
            let _next = unsafe { (*tmb).q.get_by_index(2).unwrap().value.clone() };
            let next = _next.locked_data().get();
            if next == timer_addr {
                return true;
            }
        }
        return false;
    }
}

pub fn rros_get_timer_date(timer: Arc<SpinLock<RrosTimer>>) -> KtimeT {
    let mut expiry = 0;
    let is_running = unsafe { (*timer.locked_data().get()).is_running() };
    if is_running == false {
        expiry = RROS_INFINITE;
    } else {
        expiry = rros_get_timer_expiry(timer.clone());
    }
    return expiry;
}

pub fn __rros_get_stopped_timer_delta(timer: Arc<SpinLock<RrosTimer>>) -> KtimeT {
    return __rros_get_timer_delta(timer.clone());
}

pub fn rros_get_stopped_timer_delta(timer: Arc<SpinLock<RrosTimer>>) -> KtimeT {
    let t = __rros_get_stopped_timer_delta(timer.clone());

    if ktime_to_ns(t) <= 1 {
        return RROS_INFINITE;
    }

    return t;
}

pub fn rros_enqueue_timer(
    timer: Arc<SpinLock<RrosTimer>>,
    tq: &mut List<Arc<SpinLock<RrosTimer>>>,
) {
    rros_insert_tnode(tq, timer.clone());
    unsafe {
        (*timer.locked_data().get()).change_status(!RROS_TIMER_DEQUEUED);
    }
    rros_account_timer_scheduled(timer.clone());
}

pub fn rros_destroy_timer(timer: Arc<SpinLock<RrosTimer>>) {
    rros_stop_timer(timer.clone());
    timer.lock().add_status(RROS_TIMER_KILLED);
    #[cfg(CONFIG_SMP)]
    timer.lock().set_rq(0 as *mut RrosRq);

    timer.lock().set_base(0 as *mut RrosTimerbase);
}

pub fn rros_abs_timeout(timer: Arc<SpinLock<RrosTimer>>, delta: KtimeT) -> KtimeT {
    unsafe { ktime_add((*(*timer.locked_data().get()).get_clock()).read(), delta) }
}

#[cfg(CONFIG_SMP)]
pub fn rros_prepare_timed_wait(
    timer: Arc<SpinLock<RrosTimer>>,
    clock: &mut RrosClock,
    rq: *mut rros_rq,
) {
    let f: bool = unsafe { (*timer.locked_data().get()).get_clock() != clock as *mut RrosClock };
    let s: bool = unsafe { (*timer.locked_data().get()).get_rq() != rq };
    if f || s {
        rros_move_timer(timer, clock, rq);
    }
}

#[cfg(not(CONFIG_SMP))]
pub fn rros_prepare_timed_wait(
    timer: Arc<SpinLock<RrosTimer>>,
    clock: &mut RrosClock,
    rq: *mut rros_rq,
) {
    if unsafe { (*timer.locked_data().get()).get_clock() != clock as *mut RrosClock } {
        rros_move_timer(timer, clock, rq);
    }
}
