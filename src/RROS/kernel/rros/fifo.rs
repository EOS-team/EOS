use crate::{sched, sched::RrosThread, thread::*};
use kernel::{
    c_types,
    double_linked_list::Node,
    prelude::*,
    sync::{Lock, SpinLock},
};

pub static mut RROS_SCHED_FIFO: sched::RrosSchedClass = sched::RrosSchedClass {
    sched_init: Some(rros_fifo_init),
    sched_tick: Some(rros_fifo_tick),
    sched_setparam: Some(rros_fifo_setparam),
    sched_getparam: Some(rros_fifo_getparam),
    sched_chkparam: Some(rros_fifo_chkparam),
    sched_trackprio: Some(rros_fifo_trackprio),
    sched_ceilprio: Some(rros_fifo_ceilprio),
    sched_show: Some(rros_fifo_show),
    weight: 4 * sched::RROS_CLASS_WEIGHT_FACTOR,
    policy: sched::SCHED_FIFO,
    name: "fifo",
    sched_enqueue: None,
    sched_dequeue: None,
    sched_requeue: None,
    sched_pick: None,
    sched_migrate: None,
    sched_declare: None,
    sched_forget: None,
    sched_kick: None,
    sched_control: None,
    nthreads: 0,
    next: 0 as *mut sched::RrosSchedClass,
    flag: 3,
};

pub const RROS_FIFO_MIN_PRIO: i32 = 1;
pub const RROS_FIFO_MAX_PRIO: i32 = 99;
pub const RROS_CORE_MIN_PRIO: i32 = 0;
pub const RROS_CORE_MAX_PRIO: i32 = 101;

// pub fn init_rros_sched_fifo() -> Rc<RefCell<sched::RrosSchedClass>> {
//     let RrosSchedFifo: Rc<RefCell<sched::RrosSchedClass>> =
//         Rc::try_new(RefCell::new(sched::RrosSchedClass {
//             sched_init: Some(rros_fifo_init),
//             // sched_tick : rros_fifo_tick,
//             sched_tick: None,
//             sched_setparam: Some(rros_fifo_setparam),
//             sched_getparam: Some(rros_fifo_getparam),
//             sched_chkparam: Some(rros_fifo_chkparam),
//             sched_trackprio: Some(rros_fifo_trackprio),
//             sched_ceilprio: Some(rros_fifo_ceilprio),
//             sched_show: Some(rros_fifo_show),
//             weight: 4 * sched::RROS_CLASS_WEIGHT_FACTOR,
//             policy: sched::SCHED_FIFO,
//             name: "fifo",
//             sched_enqueue: None,
//             sched_dequeue: None,
//             sched_requeue: None,
//             sched_pick: None,
//             sched_migrate: None,
//             sched_declare: None,
//             sched_forget: None,
//             sched_kick: None,
//             sched_control: None,
//             nthreads: 0,
//             next: None,
//         }))
//         .unwrap();
//     return RrosSchedFifo.clone();
// }

fn rros_fifo_init(_rq: *mut sched::rros_rq) -> Result<usize> {
    Ok(0)
}

fn rros_fifo_tick(rq: Option<*mut sched::rros_rq>) -> Result<usize> {
    match rq {
        Some(_) => (),
        None => return Err(kernel::Error::EINVAL),
    }
    let rq_ptr = rq.unwrap();
    let curr;
    unsafe {
        match (*rq_ptr).curr.clone() {
            Some(c) => curr = Some(c.clone()),
            None => {
                pr_warn!("err");
                return Err(kernel::Error::EINVAL);
            }
        }
    }
    sched::rros_putback_thread(curr.unwrap())?;
    Ok(0)
}

fn rros_fifo_setparam(
    thread: Option<Arc<SpinLock<sched::RrosThread>>>,
    p: Option<Arc<SpinLock<sched::RrosSchedParam>>>,
) -> Result<usize> {
    return __rros_set_fifo_schedparam(thread.clone(), p.clone());
}

fn rros_fifo_getparam(
    thread: Option<Arc<SpinLock<sched::RrosThread>>>,
    p: Option<Arc<SpinLock<sched::RrosSchedParam>>>,
) {
    __rros_get_fifo_schedparam(thread.clone(), p.clone());
}

fn rros_fifo_chkparam(
    thread: Option<Arc<SpinLock<RrosThread>>>,
    p: Option<Arc<SpinLock<sched::RrosSchedParam>>>,
) -> Result<i32> {
    return __rros_chk_fifo_schedparam(thread.clone(), p.clone());
}

fn rros_fifo_trackprio(
    thread: Option<Arc<SpinLock<sched::RrosThread>>>,
    p: Option<Arc<SpinLock<sched::RrosSchedParam>>>,
) {
    __rros_track_fifo_priority(thread.clone(), p.clone());
}

fn rros_fifo_ceilprio(thread: Arc<SpinLock<sched::RrosThread>>, prio: i32) {
    __rros_ceil_fifo_priority(thread.clone(), prio);
}

fn rros_fifo_show(
    thread: *mut sched::RrosThread,
    _buf: *mut c_types::c_char,
    _count: sched::SsizeT,
) -> Result<usize> {
    unsafe {
        if (*thread).state & T_RRB != 0 {
            // return snprintf(buf, count, "%Ld\n",ktime_to_ns(thread->rrperiod));
            pr_warn!("rros_fifo_show error!!");
            return Err(kernel::Error::EPERM);
        }
    }
    Ok(0)
}

fn __rros_set_fifo_schedparam(
    thread: Option<Arc<SpinLock<sched::RrosThread>>>,
    p: Option<Arc<SpinLock<sched::RrosSchedParam>>>,
) -> Result<usize> {
    let thread_clone = thread.clone();
    let p_unwrap = p.unwrap();
    let thread_unwrap = thread.unwrap();

    let prio = unsafe { (*p_unwrap.locked_data().get()).fifo.prio };
    let ret = sched::rros_set_effective_thread_priority(thread_clone, prio);
    let state = thread_unwrap.lock().state;
    if state & T_BOOST == 0 {
        thread_unwrap.lock().state &= !T_WEAK;
    }
    pr_debug!("thread before calling {}", state);
    ret
}

fn __rros_get_fifo_schedparam(
    thread: Option<Arc<SpinLock<sched::RrosThread>>>,
    p: Option<Arc<SpinLock<sched::RrosSchedParam>>>,
) {
    p.unwrap().lock().fifo.prio = thread.unwrap().lock().cprio;
}

// The logic is complete, but haven't been tested.
fn __rros_chk_fifo_schedparam(
    thread: Option<Arc<SpinLock<RrosThread>>>,
    p: Option<Arc<SpinLock<sched::RrosSchedParam>>>,
) -> Result<i32> {
    let thread_unwrap = thread.unwrap();
    let mut min = RROS_FIFO_MIN_PRIO;
    let mut max = RROS_FIFO_MAX_PRIO;
    let p_unwrap = p.unwrap();
    unsafe {
        let state = (*thread_unwrap.locked_data().get()).state;
        if state & T_USER == 0x0 {
            min = RROS_CORE_MIN_PRIO;
            max = RROS_CORE_MAX_PRIO;
        }
        let prio = (*p_unwrap.locked_data().get()).fifo.prio;
        if prio < min || prio > max {
            return Err(kernel::Error::EINVAL);
        }
    }
    Ok(0)
}

fn __rros_track_fifo_priority(
    thread: Option<Arc<SpinLock<sched::RrosThread>>>,
    p: Option<Arc<SpinLock<sched::RrosSchedParam>>>,
) {
    let thread_unwrap = thread.unwrap();
    if p.is_some() {
        thread_unwrap.lock().cprio = p.unwrap().lock().fifo.prio;
    } else {
        thread_unwrap.lock().cprio = thread_unwrap.lock().bprio;
        thread_unwrap.lock().state &= !T_WEAK;
    }
}

fn __rros_ceil_fifo_priority(thread: Arc<SpinLock<sched::RrosThread>>, prio: i32) {
    unsafe { (*thread.locked_data().get()).cprio = prio };
}

pub fn __rros_dequeue_fifo_thread(thread: Arc<SpinLock<sched::RrosThread>>) -> Result<usize> {
    let rq_next = thread.lock().rq_next.clone();
    if rq_next.is_none() {
        return Err(kernel::Error::EINVAL);
    } else {
        unsafe {
            // thread.lock().rq_next.clone().as_mut().unwrap().remove();
            thread.lock().rq_next.as_mut().unwrap().as_mut().remove();
            // need a release here?
        }
    }
    Ok(0)
}

// Enter the queue according to the priority. Note that rq_next must be assigned here---this variable is used when dequeuing.
pub fn __rros_enqueue_fifo_thread(thread: Arc<SpinLock<sched::RrosThread>>) -> Result<usize> {
    let rq_ptr;
    match thread.lock().rq.clone() {
        Some(rq) => rq_ptr = rq,
        None => return Err(kernel::Error::EINVAL),
    }

    let q = unsafe { (*rq_ptr).fifo.runnable.head.as_mut().unwrap() };
    let new_cprio = thread.lock().cprio;
    if q.is_empty() {
        q.add_head(thread.clone());
        thread.lock().rq_next = q.head.prev.clone();
        // pr_debug!("addr: {:p}", thread.lock().rq_next.clone().as_mut().unwrap());
    } else {
        let mut p = q.head.prev;
        // Traverse in reverse order.
        loop {
            unsafe {
                let pos_cprio = p.unwrap().as_ref().value.lock().cprio;
                if p.unwrap().as_ptr() == &mut q.head as *mut Node<Arc<SpinLock<sched::RrosThread>>>
                    || new_cprio <= pos_cprio
                {
                    p.unwrap()
                        .as_mut()
                        .add(p.unwrap().as_ref().next.unwrap().as_ptr(), thread.clone());
                    thread.lock().rq_next = p.unwrap().as_mut().next.clone();
                    // thread.lock().rq_next = Some(Node::new(p.unwrap().as_ref().next.as_ref().unwrap().value.clone()));
                    break;
                } else {
                    p = p.unwrap().as_ref().prev;
                }
            }
            if p.unwrap().as_ptr() == q.head.prev.unwrap().as_ptr() {
                break;
            }
        }
    }
    Ok(0)
}

pub fn __rros_requeue_fifo_thread(thread: Arc<SpinLock<sched::RrosThread>>) -> Result<usize> {
    unsafe {
        let rq_ptr;
        match (*thread.locked_data().get()).rq.clone() {
            Some(rq) => rq_ptr = rq,
            None => return Err(kernel::Error::EINVAL),
        }
        let q = (*rq_ptr).fifo.runnable.head.as_mut().unwrap();
        let new_cprio = (*thread.locked_data().get()).cprio;
        if q.is_empty() {
            q.add_head(thread.clone());
            // (*thread.locked_data().get()).rq_next = Some(Node::new(q.head.prev.clone().unwrap().as_ref().value.clone()));
            (*thread.locked_data().get()).rq_next = q.head.prev;
            // pr_debug!("addr: {:p}", (*thread.locked_data().get()).rq_next.clone().as_mut().unwrap());
        } else {
            let mut p = q.head.prev;
            // Traverse in reverse order.
            loop {
                let pos_cprio = (*(p.unwrap().as_ref().value).locked_data().get()).cprio;
                if p.unwrap().as_ptr() == &mut q.head as *mut Node<Arc<SpinLock<sched::RrosThread>>>
                    || new_cprio < pos_cprio
                {
                    p.unwrap()
                        .as_mut()
                        .add(p.unwrap().as_ref().next.unwrap().as_ptr(), thread.clone());
                    // (*thread.locked_data().get()).rq_next =
                    //     Some(Node::new(p.unwrap().as_ref().next.clone().unwrap().as_ref().value.clone()));
                    (*thread.locked_data().get()).rq_next = p.unwrap().as_mut().next.clone();
                    // Some(Node::new(p.unwrap().as_ref().next.clone().unwrap().as_ref().value.clone()));
                    break;
                } else {
                    p = p.unwrap().as_ref().prev;
                }
                if p.unwrap().as_ptr() == q.head.prev.unwrap().as_ptr() {
                    break;
                }
            }
        }
        Ok(0)
    }
}
