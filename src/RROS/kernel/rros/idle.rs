use crate::{sched, thread::*};
use kernel::{
    prelude::*,
    sync::{Lock, SpinLock},
};

pub static mut RROS_SCHED_IDLE: sched::RrosSchedClass = sched::RrosSchedClass {
    sched_pick: Some(rros_idle_pick),
    sched_setparam: Some(rros_idle_setparam),
    sched_getparam: Some(rros_idle_getparam),
    sched_trackprio: Some(rros_idle_trackprio),
    sched_ceilprio: Some(rros_idle_ceilprio),
    weight: 0 * sched::RROS_CLASS_WEIGHT_FACTOR,
    policy: sched::SCHED_IDLE,
    name: "idle",
    sched_init: None,
    sched_enqueue: None,
    sched_dequeue: None,
    sched_requeue: None,
    sched_tick: None,
    sched_migrate: None,
    sched_chkparam: None,
    sched_declare: None,
    sched_forget: None,
    sched_kick: None,
    sched_show: None,
    sched_control: None,
    nthreads: 0,
    next: 0 as *mut sched::RrosSchedClass,
    flag: 1,
};

pub const RROS_IDLE_PRIO: i32 = -1;

// pub fn init_rros_sched_idle() -> Rc<RefCell<sched::RrosSchedClass>> {
//     let RROS_SCHED_IDLE: Rc<RefCell<sched::RrosSchedClass>> =
//         Rc::try_new(RefCell::new(sched::RrosSchedClass {
//             sched_pick: Some(rros_idle_pick),
//             sched_setparam: Some(rros_idle_setparam),
//             sched_getparam: Some(rros_idle_getparam),
//             sched_trackprio: Some(rros_idle_trackprio),
//             sched_ceilprio: Some(rros_idle_ceilprio),
//             weight: 0 * sched::RROS_CLASS_WEIGHT_FACTOR,
//             policy: sched::SCHED_IDLE,
//             name: "idle",
//             sched_init: None,
//             sched_enqueue: None,
//             sched_dequeue: None,
//             sched_requeue: None,
//             sched_tick: None,
//             sched_migrate: None,
//             sched_chkparam: None,
//             sched_declare: None,
//             sched_forget: None,
//             sched_kick: None,
//             sched_show: None,
//             sched_control: None,
//             nthreads: 0,
//             next: None,
//             flag:1,
//         }))
//         .unwrap();
//     return RROS_SCHED_IDLE.clone();
// }

fn rros_idle_pick(rq: Option<*mut sched::rros_rq>) -> Result<Arc<SpinLock<sched::RrosThread>>> {
    match rq {
        Some(_) => (),
        None => return Err(kernel::Error::EINVAL),
    }
    let root_thread;
    unsafe {
        match (*rq.unwrap()).root_thread.clone() {
            Some(t) => root_thread = t.clone(),
            None => return Err(kernel::Error::EINVAL),
        }
    }
    return Ok(root_thread);
}

fn rros_idle_setparam(
    thread: Option<Arc<SpinLock<sched::RrosThread>>>,
    p: Option<Arc<SpinLock<sched::RrosSchedParam>>>,
) -> Result<usize> {
    return __rros_set_idle_schedparam(thread.clone(), p.clone());
}

fn __rros_set_idle_schedparam(
    thread: Option<Arc<SpinLock<sched::RrosThread>>>,
    p: Option<Arc<SpinLock<sched::RrosSchedParam>>>,
) -> Result<usize> {
    let thread_clone = thread.clone();
    let thread_unwrap = thread_clone.unwrap();
    // let mut thread_lock = thread_unwrap.lock();
    let p_unwrap = p.unwrap();
    thread_unwrap.lock().state &= !T_WEAK;
    let prio = unsafe { (*p_unwrap.locked_data().get()).idle.prio };
    return sched::rros_set_effective_thread_priority(thread.clone(), prio);
}

fn rros_idle_getparam(
    thread: Option<Arc<SpinLock<sched::RrosThread>>>,
    p: Option<Arc<SpinLock<sched::RrosSchedParam>>>,
) {
    __rros_get_idle_schedparam(thread.clone(), p.clone());
}

fn __rros_get_idle_schedparam(
    thread: Option<Arc<SpinLock<sched::RrosThread>>>,
    p: Option<Arc<SpinLock<sched::RrosSchedParam>>>,
) {
    p.unwrap().lock().idle.prio = thread.unwrap().lock().cprio;
}

fn rros_idle_trackprio(
    thread: Option<Arc<SpinLock<sched::RrosThread>>>,
    p: Option<Arc<SpinLock<sched::RrosSchedParam>>>,
) {
    __rros_track_idle_priority(thread.clone(), p.clone());
}

fn __rros_track_idle_priority(
    thread: Option<Arc<SpinLock<sched::RrosThread>>>,
    p: Option<Arc<SpinLock<sched::RrosSchedParam>>>,
) {
    if p.is_some() {
        pr_warn!("Inheriting a priority-less class makes no sense.");
    } else {
        thread.unwrap().lock().cprio = RROS_IDLE_PRIO;
    }
}

fn rros_idle_ceilprio(thread: Arc<SpinLock<sched::RrosThread>>, prio: i32) {
    __rros_ceil_idle_priority(thread.clone(), prio);
}

fn __rros_ceil_idle_priority(_thread: Arc<SpinLock<sched::RrosThread>>, _prio: i32) {
    pr_warn!("RROS_WARN_ON_ONCE(CORE, 1)");
}
