use crate::{queue, sched, thread::*};
use alloc::rc::Rc;
use core::{
    cell::RefCell,
    ptr::{null, null_mut},
};
use kernel::{prelude::*, Error};

pub static mut RrosSchedWeak: Rc<RefCell<sched::RrosSchedClass>> = sched::RrosSchedClass {
    sched_init: Some(weak_init),
    sched_enqueue: Some(weak_enqueue),
    sched_dequeue: Some(weak_dequeue),
    sched_requeue: Some(weak_requeue),
    sched_pick: Some(weak_pick),
    sched_setparam: Some(weak_setparam),
    sched_getparam: Some(weak_getparam),
    sched_chkparam: Some(weak_chkparam),
    sched_trackprio: Some(weak_trackprio),
    sched_ceilprio: Some(weak_ceilprio),
    sched_declare: Some(weak_declare),
    weight: 1 * sched::RROS_CLASS_WEIGHT_FACTOR,
    policy: sched::SCHED_WEAK,
    name: "weak",
    sched_tick: None,
    sched_migrate: None,
    sched_forget: None,
    sched_kick: None,
    sched_show: None,
    sched_control: None,
    nthreads: 0,
    next: None,
    // FIXME: make sure is this correct?
    flag: 0,
};

const RROS_WEAK_MIN_PRIO: i32 = 0;
const RROS_WEAK_MAX_PRIO: i32 = 99;

fn weak_init(rq: Rc<RefCell<sched::rros_rq>>) {
    let rq_ptr = rq.borrow_mut();
    queue::rros_init_schedq(Rc::try_new(RefCell::new(rq_ptr.weak.runnable)));
}

fn weak_enqueue(thread: Rc<RefCell<sched::RrosThread>>) {
    let thread_clone = thread.clone();
    let thread_ptr = thread_clone.borrow_mut();
    let rq_clone;
    match thread_ptr.rq.clone() {
        Some(r) => rq_clone = r.clone(),
        None => Err(kernel::Error::EINVAL),
    }
    let rq_ptr = rq_clone.borrow_mut();
    queue::rros_add_schedq_tail(rq_ptr.weak.runnable, thread_clone);
}

fn weak_dequeue(thread: Rc<RefCell<sched::RrosThread>>) {
    let thread_clone = thread.clone();
    let thread_ptr = thread_clone.borrow_mut();
    let rq_clone;
    match thread_ptr.rq.clone() {
        Some(r) => rq_clone = r.clone(),
        None => Err(kernel::Error::EINVAL),
    }
    let rq_ptr = rq_clone.borrow_mut();
    queue::rros_del_schedq(rq_ptr.weak.runnable, thread_clone);
}

fn weak_requeue(thread: Rc<RefCell<sched::RrosThread>>) {
    let thread_clone = thread.clone();
    let thread_ptr = thread_clone.borrow_mut();
    let rq_clone;
    match thread_ptr.rq.clone() {
        Some(r) => rq_clone = r.clone(),
        None => Err(kernel::Error::EINVAL),
    }
    let rq_ptr = rq_clone.borrow_mut();
    queue::rros_add_schedq(rq_ptr.weak.runnable, thread_clone);
}

fn weak_pick(rq: Rc<ReCell<sched::rros_rq>>) {
    let rq_ptr = rq.borrow_mut();
    return queue::rros_get_schedq(rq_ptr.weak.runnable);
}

fn weak_setparam(
    thread: Rc<RefCell<sched::RrosThread>>,
    p: *const sched::RrosSchedParam,
) -> Result<usize> {
    let thread_ptr = thread.borrow_mut();
    if thread_ptr.state & sched::T_BOOST == 0 {
        thread_ptr.state |= T_WEAK;
    }

    return sched::rros_set_effective_thread_priority(thread.clone(), unsafe { (*p) }.weak.prio);
}

fn weak_getparam(thread: Rc<RefCell<sched::RrosThread>>, p: *mut sched::RrosSchedParam) {
    let thread_ptr = thread.borrow_mut();
    unsafe { (*p) }.weak.prio = thread_ptr.cprio;
}

fn weak_chkparam(
    thread: Rc<RefCell<sched::RrosThread>>,
    p: *const sched::RrosSchedParam,
) -> Result<i32> {
    if (unsafe { (*p) }.weak.prio < RROS_WEAK_MIN_PRIO
        || unsafe { (*p) }.weak.prio > RROS_WEAK_MAX_PRIO)
    {
        return Err(kernel::Error::EINVAL);
    }
    Ok(0)
}

fn weak_trackprio(thread: Rc<RefCell<sched::RrosThread>>, p: *const sched::RrosSchedParam) {
    let thread_ptr = thread.borrow_mut();
    if p != null() {
        thread_ptr.cprio = unsafe { (*p) }.weak.prio;
    } else {
        thread_ptr.cprio = thread_ptr.bprio;
    }
}

fn weak_ceilprio(thread: Rc<RefCell<sched::RrosThread>>, prio: i32) {
    let thread_ptr = thread.borrow_mut();
    if prio > RROS_WEAK_MAX_PRIO {
        prio = RROS_WEAK_MAX_PRIO;
    }
    thread_ptr.cprio = prio;
}

fn weak_declare(
    thread: Rc<RefCell<sched::RrosThread>>,
    p: *const sched::RrosSchedParam,
) -> Result<i32> {
    if (unsafe { (*p) }.weak.prio < RROS_WEAK_MIN_PRIO
        || unsafe { (*p) }.weak.prio > RROS_WEAK_MAX_PRIO)
    {
        return Err(kernel::Error::EINVAL);
    }

    Ok(0)
}
