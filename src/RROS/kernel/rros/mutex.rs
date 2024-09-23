use crate::{clock::*, factory::*, fifo::*, lock, sched::*, thread::*, timeout};
use kernel::{
    bindings, c_str, c_types,
    double_linked_list::*,
    prelude::*,
    premmpt,
    str::CStr,
    sync::{Guard, Lock, SpinLock},
};

pub const RROS_NO_HANDLE: u32 = 0x00000000;
pub const RROS_MUTEX_PI: u32 = 1;
pub const RROS_MUTEX_PP: u32 = 2;
pub const RROS_MUTEX_CLAIMED: u32 = 4;
pub const RROS_MUTEX_CEILING: u32 = 8;
pub const RROS_MUTEX_FLCLAIM: u32 = 0x80000000;
pub const RROS_MUTEX_FLCEIL: u32 = 0x40000000;
pub const EDEADLK: i32 = 35;
pub const EBUSY: i32 = 16;
pub const EIDRM: i32 = 43;
pub const ETIMEDOUT: i32 = 110;
pub const EINTR: i32 = 4;

type KtimeT = i64;
type FundleT = u32;

static mut rros_nil: RrosValue = RrosValue {
    val: 0,
    lval: 0,
    ptr: 0 as *mut c_types::c_void,
};

pub struct RrosMutex {
    pub wprio: i32,
    pub flags: i32,
    pub owner: Option<Arc<SpinLock<RrosThread>>>,
    pub clock: *mut RrosClock,
    pub fastlock: *mut bindings::atomic_t,
    pub ceiling_ref: u32,
    pub lock: bindings::hard_spinlock_t,
    pub wchan: rros_wait_channel,
    pub next_booster: *mut Node<Arc<SpinLock<RrosMutex>>>,
    pub next_tracker: *mut Node<Arc<SpinLock<RrosMutex>>>,
}
impl RrosMutex {
    pub fn new() -> Self {
        RrosMutex {
            wprio: 0,
            flags: 0,
            owner: None,
            clock: 0 as *mut RrosClock,
            fastlock: &mut bindings::atomic_t { counter: 0 } as *mut bindings::atomic_t,
            ceiling_ref: 0,
            lock: bindings::hard_spinlock_t {
                rlock: bindings::raw_spinlock {
                    raw_lock: bindings::arch_spinlock_t {
                        __bindgen_anon_1: bindings::qspinlock__bindgen_ty_1 {
                            val: bindings::atomic_t { counter: 0 },
                        },
                    },
                },
                dep_map: bindings::phony_lockdep_map {
                    wait_type_outer: 0,
                    wait_type_inner: 0,
                },
            },
            wchan: rros_wait_channel::new(),
            next_booster: 0 as *mut Node<Arc<SpinLock<RrosMutex>>>,
            next_tracker: 0 as *mut Node<Arc<SpinLock<RrosMutex>>>,
        }
    }
}

pub struct RrosKMutex {
    pub mutex: *mut RrosMutex,
    pub fastlock: *mut bindings::atomic_t,
}
impl RrosKMutex {
    pub fn new() -> Self {
        RrosKMutex {
            mutex: 0 as *mut RrosMutex,
            fastlock: &mut bindings::atomic_t { counter: 0 } as *mut bindings::atomic_t,
        }
    }
}

extern "C" {
    fn rust_helper_raw_spin_lock_init(lock: *mut bindings::hard_spinlock_t);
    fn rust_helper_raw_spin_lock(lock: *mut bindings::hard_spinlock_t);
    fn rust_helper_raw_spin_unlock(lock: *mut bindings::hard_spinlock_t);
}

pub fn raw_spin_lock_init(lock: *mut bindings::hard_spinlock_t) {
    unsafe { rust_helper_raw_spin_lock_init(lock) };
}

pub fn raw_spin_lock(lock: *mut bindings::hard_spinlock_t) {
    unsafe { rust_helper_raw_spin_lock(lock) };
}

pub fn raw_spin_unlock(lock: *mut bindings::hard_spinlock_t) {
    unsafe { rust_helper_raw_spin_unlock };
}

pub fn get_ceiling_value(mutex: *mut RrosMutex) -> u32 {
    let ceiling_ref = unsafe { (*mutex).ceiling_ref };
    if ceiling_ref < 1 {
        return 1 as u32;
    } else if ceiling_ref <= RROS_FIFO_MAX_PRIO as u32 {
        return ceiling_ref;
    } else {
        return RROS_FIFO_MAX_PRIO as u32;
    }
}

pub fn disable_inband_switch(curr: *mut RrosThread) {
    unsafe {
        if ((*curr).state & (T_WEAK | T_WOLI)) != 0 {
            atomic_inc(&mut (*curr).inband_disable_count as *mut bindings::atomic_t);
        }
    }
}

pub fn enable_inband_switch(curr: *mut RrosThread) -> bool {
    unsafe {
        if ((*curr).state & (T_WEAK | T_WOLI)) == 0 {
            return true;
        }

        if (atomic_dec_return(&mut (*curr).inband_disable_count as *mut bindings::atomic_t) >= 0) {
            return true;
        }

        atomic_set(
            &mut (*curr).inband_disable_count as *mut bindings::atomic_t,
            0,
        );
        if (*curr).state & T_WOLI != 0 {
            rros_notify_thread(curr, RROS_HMDIAG_LKIMBALANCE as u32, &rros_nil);
        }
        return false;
    }
}

pub fn raise_boost_flag(owner: Arc<SpinLock<RrosThread>>) {
    unsafe {
        // assert_hard_lock(&owner->lock);
        let lock =
            &mut (*(*owner.locked_data().get()).rq.unwrap()).lock as *mut bindings::hard_spinlock_t;
        raw_spin_lock(lock);

        let state = (*owner.locked_data().get()).state;

        if state & T_BOOST == 0 {
            (*owner.locked_data().get()).bprio = (*owner.locked_data().get()).cprio;
            (*owner.locked_data().get()).state |= T_BOOST;
        }
        raw_spin_unlock(lock);
    }
}

pub fn inherit_thread_priority(
    owner: Arc<SpinLock<RrosThread>>,
    contender: Arc<SpinLock<RrosThread>>,
    originator: Arc<SpinLock<RrosThread>>,
) -> Result<i32> {
    let ret: Result<i32>;

    // assert_hard_lock(&owner->lock);
    // assert_hard_lock(&contender->lock);

    rros_track_thread_policy(owner.clone(), contender.clone());

    let func;
    unsafe {
        let wchan = (*owner.locked_data().get()).wchan.clone().unwrap();
        match (*wchan.locked_data().get()).reorder_wait {
            Some(f) => func = f,
            None => {
                pr_warn!("inherit_thread_priority:reorder_wait function error");
                return Err(kernel::Error::EINVAL);
            }
        }
    }
    unsafe { return func(owner.clone(), originator.clone()) };
}

pub fn adjust_boost(
    owner: Arc<SpinLock<RrosThread>>,
    contender: Arc<SpinLock<RrosThread>>,
    origin: *mut RrosMutex,
    originator: Arc<SpinLock<RrosThread>>,
) -> Result<i32> {
    unsafe {
        let mut mutex = 0 as *mut RrosMutex;
        let mut pprio: u32 = 0;
        let mut ret: Result<i32> = Ok(0);
        // assert_hard_lock(&owner->lock);
        // assert_hard_lock(&origin->lock);
        let boosters = (*owner.locked_data().get()).boosters;
        mutex = Arc::into_raw((*boosters).get_head().unwrap().value.clone())
            as *mut SpinLock<RrosMutex> as *mut RrosMutex;
        if mutex != origin {
            raw_spin_lock(&mut (*mutex).lock as *mut bindings::hard_spinlock_t);
        }
        let wprio = (*owner.locked_data().get()).wprio;
        if (*mutex).wprio == wprio {
            if mutex != origin {
                raw_spin_unlock(&mut (*mutex).lock as *mut bindings::hard_spinlock_t);
            }
            return Ok(0);
        }

        if (*mutex).flags & RROS_MUTEX_PP as i32 != 0 {
            pprio = get_ceiling_value(mutex);

            rros_protect_thread_priority(owner.clone(), pprio as i32);
            let wchan = (*owner.locked_data().get()).wchan.clone().unwrap();
            let func;
            match (*wchan.locked_data().get()).reorder_wait {
                Some(f) => func = f,
                None => {
                    pr_warn!("adjust_boost:reorder_wait function error");
                    return Err(kernel::Error::EINVAL);
                }
            }
            ret = func(owner.clone(), originator.clone());
            if mutex != origin {
                raw_spin_unlock(&mut (*mutex).lock as *mut bindings::hard_spinlock_t);
            }
        } else {
            if (*(*mutex).wchan.wait_list).is_empty() {
                if mutex != origin {
                    raw_spin_unlock(&mut (*mutex).lock as *mut bindings::hard_spinlock_t);
                }
                return Ok(0);
            }
            let contender_ptr =
                Arc::into_raw(contender.clone()) as *mut SpinLock<RrosThread> as *mut RrosThread;
            if contender_ptr == 0 as *mut RrosThread {
                let contender = (*(*mutex).wchan.wait_list)
                    .get_head()
                    .unwrap()
                    .value
                    .clone();
                let lock =
                    &mut (*contender.locked_data().get()).lock as *mut bindings::hard_spinlock_t;
                raw_spin_lock(lock);
                ret = inherit_thread_priority(owner.clone(), contender.clone(), originator.clone());
                raw_spin_unlock(lock);
            } else {
                ret = inherit_thread_priority(owner.clone(), contender.clone(), originator.clone());
            }
            if mutex != origin {
                raw_spin_unlock(&mut (*mutex).lock as *mut bindings::hard_spinlock_t);
            }
        }
        return ret;
    }
}

pub fn ceil_owner_priority(
    mutex: *mut RrosMutex,
    originator: Arc<SpinLock<RrosThread>>,
) -> Result<usize> {
    unsafe {
        let owner = (*mutex).owner.clone().unwrap().clone();
        let wprio: i32;
        // assert_hard_lock(&mutex->lock);
        wprio = rros_calc_weighted_prio(&RrosSchedFifo, get_ceiling_value(mutex) as i32);
        (*mutex).wprio = wprio;
        let lock = &mut (*owner.locked_data().get()).lock as *mut bindings::hard_spinlock_t;
        raw_spin_lock(lock);

        let boosters = (*owner.locked_data().get()).boosters;
        if (*boosters).is_empty() {
            (*boosters).add_head((*(*mutex).next_booster).value.clone());
        } else {
            let mut flag = 1;
            for i in (*boosters).len()..=1 {
                let wprio_in_list = (*(*(*boosters).get_by_index(i).unwrap())
                    .value
                    .clone()
                    .locked_data()
                    .get())
                .wprio;
                if (*mutex).wprio <= wprio_in_list {
                    flag = 0;
                    (*boosters).enqueue_by_index(i, (*(*mutex).next_booster).value.clone());
                    break;
                }
            }
            if flag == 1 {
                (*boosters).add_head((*(*mutex).next_booster).value.clone());
            }
        }

        raise_boost_flag(owner.clone());
        (*mutex).flags |= RROS_MUTEX_CEILING as i32;

        let owner_wprio = (*owner.locked_data().get()).wprio;
        if wprio > owner_wprio {
            adjust_boost(
                owner.clone(),
                Arc::try_new(SpinLock::new(RrosThread::new()?))?,
                mutex,
                originator.clone(),
            );
        }
        raw_spin_unlock(lock);
        Ok(0)
    }
}

pub fn untrack_owner(mutex: *mut RrosMutex) {
    unsafe {
        let prev = (*mutex).owner.clone();

        // assert_hard_lock(&mutex->lock);
        if prev.is_some() {
            let flags = lock::raw_spin_lock_irqsave();
            (*(*mutex).next_tracker).remove();
            lock::raw_spin_unlock_irqrestore(flags);
            // rros_put_element(&prev->element);
            (*mutex).owner = None;
        }
    }
}

pub fn track_owner(mutex: *mut RrosMutex, owner: Arc<SpinLock<RrosThread>>) {
    unsafe {
        let prev = (*mutex).owner.clone();
        // assert_hard_lock(&mutex->lock);
        // if (RROS_WARN_ON_ONCE(CORE, prev == owner))
        // 	return;

        let flags = lock::raw_spin_lock_irqsave();
        if prev.is_some() {
            (*(*mutex).next_tracker).remove();
            // smp_wmb();
            // rros_put_element(&prev->element);
        }
        (*(*owner.locked_data().get()).trackers).add_head((*((*mutex).next_tracker)).value.clone());
        lock::raw_spin_unlock_irqrestore(flags);
        (*mutex).owner = Some(owner.clone());
    }
}

pub fn ref_and_track_owner(mutex: *mut RrosMutex, owner: Arc<SpinLock<RrosThread>>) {
    unsafe {
        let ptr1 = Arc::into_raw((*mutex).owner.clone().unwrap()) as *mut SpinLock<RrosThread>;
        let ptr2 = Arc::into_raw(owner.clone()) as *mut SpinLock<RrosThread>;
        if ptr1 != ptr2 {
            // rros_get_element(&owner->element);
            track_owner(mutex, owner.clone());
        }
    }
}

pub fn fast_mutex_is_claimed(handle: u32) -> bool {
    return handle & RROS_MUTEX_FLCLAIM != 0;
}

pub fn mutex_fast_claim(handle: u32) -> u32 {
    return handle | RROS_MUTEX_FLCLAIM;
}

pub fn mutex_fast_ceil(handle: u32) -> u32 {
    return handle | RROS_MUTEX_FLCEIL;
}

pub fn set_current_owner_locked(mutex: *mut RrosMutex, owner: Arc<SpinLock<RrosThread>>) {
    // assert_hard_lock(&mutex->lock);
    ref_and_track_owner(mutex, owner.clone());
    pr_debug!("1111111111111111111111111111111111111111111");
    unsafe {
        if (*mutex).flags & RROS_MUTEX_PP as i32 != 0 {
            pr_debug!("2222222222222222222222222222222222222");
            ceil_owner_priority(mutex, owner.clone());
            pr_debug!("333333333333333333333333333333333333333");
        }
    }
}

pub fn set_current_owner(mutex: *mut RrosMutex, owner: Arc<SpinLock<RrosThread>>) -> Result<usize> {
    pr_debug!("00000000000000000000000000000000000000");
    let flags = lock::raw_spin_lock_irqsave();
    pr_debug!("000000000000000.............555000000000000000000000.50.50.5");
    set_current_owner_locked(mutex, owner.clone());
    lock::raw_spin_unlock_irqrestore(flags);
    pr_debug!("99999999999999999999999999999999999999");
    Ok(0)
}

pub fn get_owner_handle(mut ownerh: u32, mutex: *mut RrosMutex) -> u32 {
    unsafe {
        if (*mutex).flags & RROS_MUTEX_PP as i32 != 0 {
            ownerh = mutex_fast_ceil(ownerh);
        }
        return ownerh;
    }
}

pub fn clear_boost_locked(
    mutex: *mut RrosMutex,
    owner: Arc<SpinLock<RrosThread>>,
    flag: i32,
) -> Result<i32> {
    unsafe {
        // assert_hard_lock(&mutex->lock);
        // assert_hard_lock(&owner->lock);
        (*mutex).flags &= !flag;

        (*(*mutex).next_booster).remove();
        let boosters = (*owner.locked_data().get()).boosters;
        if (*boosters).is_empty() {
            let lock = &mut (*(*owner.locked_data().get()).rq.unwrap()).lock
                as *mut bindings::hard_spinlock_t;
            raw_spin_lock(lock);
            (*owner.locked_data().get()).state &= !T_BOOST;
            raw_spin_unlock(lock);
            inherit_thread_priority(owner.clone(), owner.clone(), owner.clone());
        } else {
            adjust_boost(
                owner.clone(),
                Arc::try_new(SpinLock::new(RrosThread::new()?))?,
                mutex,
                owner.clone(),
            );
        }
        Ok(0)
    }
}

pub fn clear_boost(
    mutex: *mut RrosMutex,
    owner: Arc<SpinLock<RrosThread>>,
    flag: i32,
) -> Result<usize> {
    let lock = unsafe { &mut (*owner.locked_data().get()).lock as *mut bindings::hard_spinlock_t };
    raw_spin_lock(lock);
    clear_boost_locked(mutex, owner.clone(), flag);
    raw_spin_unlock(lock);
    Ok(0)
}

pub fn detect_inband_owner(mutex: *mut RrosMutex, curr: *mut RrosThread) {
    unsafe {
        let owner = (*mutex).owner.clone().unwrap();
        let lock = &mut (*(*curr).rq.unwrap()).lock as *mut bindings::hard_spinlock_t;
        raw_spin_lock(lock);
        let state = (*owner.locked_data().get()).state;
        if (*curr).info & T_PIALERT != 0 {
            (*curr).info &= !T_PIALERT;
        } else if state & T_INBAND != 0 {
            (*curr).info |= T_PIALERT;
            raw_spin_unlock(lock);
            rros_notify_thread(curr, RROS_HMDIAG_LKDEPEND as u32, &rros_nil);
            return;
        }

        raw_spin_unlock(lock);
    }
}

pub fn rros_detect_boost_drop() {
    unsafe {
        let curr = rros_current() as *mut RrosThread;
        let mut waiter = 0 as *mut RrosThread;
        let mutex = 0 as *mut RrosMutex;

        let flags = lock::raw_spin_lock_irqsave();

        let boosters = (*curr).boosters;
        for i in 1..=(*boosters).len() {
            let wait_list = (*(*boosters)
                .get_by_index(i)
                .unwrap()
                .value
                .clone()
                .locked_data()
                .get())
            .wchan
            .wait_list;
            // raw_spin_lock(&mut (*mutex).lock as *mut bindings::hard_spinlock_t);
            for j in 1..=(*wait_list).len() {
                let waiter_node = (*wait_list).get_by_index(j).unwrap().value.clone();
                waiter = Arc::into_raw(waiter_node) as *mut SpinLock<RrosThread> as *mut RrosThread;
                if (*waiter).state & T_WOLI == 0 {
                    continue;
                }
                raw_spin_lock(&mut (*(*waiter).rq.unwrap()).lock as *mut bindings::hard_spinlock_t);
                (*waiter).info |= T_PIALERT;
                raw_spin_unlock(
                    &mut (*(*waiter).rq.unwrap()).lock as *mut bindings::hard_spinlock_t,
                );
                rros_notify_thread(waiter, RROS_HMDIAG_LKDEPEND as u32, &rros_nil);
            }
            // raw_spin_unlock(&mut (*mutex).lock as *mut bindings::hard_spinlock_t);
        }

        lock::raw_spin_unlock_irqrestore(flags);
    }
}

pub fn __rros_init_mutex(
    mutex: *mut RrosMutex,
    clock: *mut RrosClock,
    fastlock: *mut bindings::atomic_t,
    ceiling_ref: u32,
) {
    unsafe {
        let mut Type: u32 = 0;
        if ceiling_ref == 0 {
            Type = RROS_MUTEX_PI;
        } else {
            Type = RROS_MUTEX_PP;
        }
        if mutex == 0 as *mut RrosMutex {
            pr_err!("__rros_init_mutex error!");
            return;
        }
        (*mutex).fastlock = fastlock;
        atomic_set(fastlock, RROS_NO_HANDLE as i32);
        (*mutex).flags = (Type & !RROS_MUTEX_CLAIMED) as i32;
        (*mutex).owner = None;
        (*mutex).wprio = -1;
        (*mutex).ceiling_ref = ceiling_ref;
        (*mutex).clock = clock;
        (*mutex).wchan.reorder_wait = Some(rros_reorder_mutex_wait);
        (*mutex).wchan.follow_depend = Some(rros_follow_mutex_depend);
        raw_spin_lock_init(&mut (*mutex).lock as *mut bindings::hard_spinlock_t);
    }
}

pub fn flush_mutex_locked(mutex: *mut RrosMutex, reason: u32) -> Result<usize> {
    let tmp = 0 as *mut RrosThread;
    // assert_hard_lock(&mutex->lock);
    unsafe {
        let mut thread_node = Arc::try_new(SpinLock::new(RrosThread::new()?))?;
        if (*(*mutex).wchan.wait_list).is_empty() {
            // RROS_WARN_ON(CORE, mutex->flags & RROS_MUTEX_CLAIMED);
        } else {
            for i in 1..=(*(*mutex).wchan.wait_list).len() {
                thread_node = (*(*(*mutex).wchan.wait_list).get_by_index(i).unwrap())
                    .value
                    .clone();
                (*(*thread_node.locked_data().get()).wait_next).remove();
                rros_wakeup_thread(thread_node.clone(), T_PEND, reason);
            }
            if (*mutex).flags & RROS_MUTEX_CLAIMED as i32 != 0 {
                clear_boost(
                    mutex,
                    (*mutex).owner.clone().unwrap(),
                    RROS_MUTEX_CLAIMED as i32,
                );
            }
        }
    }
    Ok(0)
}

pub fn rros_flush_mutex(mutex: *mut RrosMutex, reason: u32) {
    // trace_rros_mutex_flush(mutex);
    let flags = lock::raw_spin_lock_irqsave();
    flush_mutex_locked(mutex, reason);
    lock::raw_spin_unlock_irqrestore(flags);
}

pub fn rros_destroy_mutex(mutex: *mut RrosMutex) {
    // trace_rros_mutex_destroy(mutex);
    let flags = lock::raw_spin_lock_irqsave();
    untrack_owner(mutex);
    flush_mutex_locked(mutex, T_RMID);
    lock::raw_spin_unlock_irqrestore(flags);
}

pub fn rros_trylock_mutex(mutex: *mut RrosMutex) -> Result<i32> {
    let curr = unsafe { &mut *rros_current() };
    let lockp = unsafe { (*mutex).fastlock };
    let h: i32 = 0;

    premmpt::running_inband()?;
    // trace_rros_mutex_trylock(mutex);

    // h = atomic_cmpxchg(lockp, RROS_NO_HANDLE as i32,
    // 		get_owner_handle(fundle_of(curr), mutex) as i32);
    // if h as i32 != RROS_NO_HANDLE{
    // 	if rros_get_index(h) == fundle_of(curr){
    // 		return -EDEADLK;
    // 	}
    // 	else{
    // 		return -EBUSY;
    // 	}
    // }

    unsafe { set_current_owner(mutex, Arc::from_raw(curr as *const SpinLock<RrosThread>)) };
    disable_inband_switch(curr as *mut SpinLock<RrosThread> as *mut RrosThread);

    return Ok(0);
}

pub fn wait_mutex_schedule(mutex: *mut RrosMutex) -> Result<i32> {
    let curr = rros_current();
    let flags: u32 = 0;
    let mut ret: Result<i32> = Ok(0);
    let mut info: u32 = 0;

    unsafe { rros_schedule() };

    info = unsafe { (*(*rros_current()).locked_data().get()).info };
    if info & T_RMID != 0 {
        return Err(kernel::Error::EIDRM);
    }

    if info & (T_TIMEO | T_BREAK) != 0 {
        let flags = lock::raw_spin_lock_irqsave();
        let wait_next = unsafe { (*(*curr).locked_data().get()).wait_next };
        unsafe { (*wait_next).remove() };
        if info & T_TIMEO != 0 {
            ret = Err(kernel::Error::ETIMEDOUT);
        } else if info & T_BREAK != 0 {
            ret = Err(kernel::Error::EINTR);
        }

        lock::raw_spin_unlock_irqrestore(flags);
    }
    // } else if (IS_ENABLED(CONFIG_RROS_DEBUG_CORE)) {
    // 	bool empty;
    // 	// raw_spin_lock_irqsave(&mutex->lock, flags);
    // 	empty = list_empty(&curr->wait_next);
    // 	// raw_spin_unlock_irqrestore(&mutex->lock, flags);
    // 	// RROS_WARN_ON_ONCE(CORE, !empty);
    // }

    return ret;
}

pub fn finish_mutex_wait(mutex: *mut RrosMutex) {
    unsafe {
        let owner = (*mutex).owner.clone().unwrap();

        // assert_hard_lock(&mutex->lock);

        if (*mutex).flags & RROS_MUTEX_CLAIMED as i32 == 0 {
            return;
        }

        if (*(*mutex).wchan.wait_list).is_empty() {
            clear_boost(mutex, owner.clone(), RROS_MUTEX_CLAIMED as i32);
            return;
        }

        let contender = (*(*mutex).wchan.wait_list)
            .get_head()
            .unwrap()
            .value
            .clone();
        let owner_lock = &mut (*owner.locked_data().get()).lock as *mut bindings::hard_spinlock_t;
        let contender_lock =
            &mut (*contender.locked_data().get()).lock as *mut bindings::hard_spinlock_t;
        raw_spin_lock(owner_lock);
        raw_spin_lock(contender_lock);
        (*mutex).wprio = (*contender.locked_data().get()).wprio;
        (*(*mutex).next_booster).remove();

        let boosters = (*owner.locked_data().get()).boosters;
        if (*boosters).is_empty() {
            (*boosters).add_head((*(*mutex).next_booster).value.clone());
        } else {
            let mut flag = 1;
            for i in (*boosters).len()..=1 {
                let wprio_in_list = (*(*boosters)
                    .get_by_index(i)
                    .unwrap()
                    .value
                    .clone()
                    .locked_data()
                    .get())
                .wprio;
                if (*mutex).wprio <= wprio_in_list {
                    flag = 0;
                    (*boosters).enqueue_by_index(i, (*(*mutex).next_booster).value.clone());
                    break;
                }
            }
            if flag == 1 {
                (*boosters).add_head((*(*mutex).next_booster).value.clone());
            }
        }

        adjust_boost(owner.clone(), contender.clone(), mutex, owner.clone());
        raw_spin_unlock(contender_lock);
        raw_spin_unlock(owner_lock);
    }
}

pub fn check_lock_chain(
    owner: Arc<SpinLock<RrosThread>>,
    originator: Arc<SpinLock<RrosThread>>,
) -> Result<i32> {
    unsafe {
        let mut wchan = (*owner.locked_data().get()).wchan.clone();
        // assert_hard_lock(&owner->lock);
        // assert_hard_lock(&originator->lock);

        if wchan.is_some() {
            let func;
            match (*wchan.clone().unwrap().locked_data().get()).follow_depend {
                Some(f) => func = f,
                None => {
                    pr_warn!("check_lock_chain:follow_depend function error");
                    return Err(kernel::Error::EINVAL);
                }
            }
            return func(wchan.as_mut().unwrap().clone(), originator.clone());
        }
        Ok(0)
    }
}

pub fn rros_lock_mutex_timeout(
    mutex: *mut RrosMutex,
    timeout: KtimeT,
    timeout_mode: timeout::RrosTmode,
) -> Result<i32> {
    unsafe {
        let curr = &mut *rros_current();
        let owner = Arc::try_new(SpinLock::new(RrosThread::new()?))?;
        let lockp = (*mutex).fastlock;
        let mut currh: FundleT = 0;
        let mut h: FundleT = 0;
        let mut oldh: FundleT = 0;
        let flags: u32 = 0;
        let mut ret: Result<i32>;
        premmpt::running_inband()?;
        // currh = fundle_of(curr);
        // trace_rros_mutex_lock(mutex);
        pr_debug!(
            "rros_lock_mutex_timeout rros_current address is {:p}",
            rros_current()
        );
        loop {
            h = atomic_cmpxchg(
                lockp,
                RROS_NO_HANDLE as i32,
                get_owner_handle(currh, mutex) as i32,
            ) as FundleT;
            if h == RROS_NO_HANDLE {
                let temp = Arc::from_raw(rros_current() as *const SpinLock<RrosThread>);
                let test = temp.clone();
                pr_debug!("{:p}", test);
                pr_debug!("-1-1-1-1-1-1-1-1-1-1-1-1--1-1-11-1-1-1-1-1-1");
                set_current_owner(mutex, temp.clone());

                disable_inband_switch(curr as *mut SpinLock<RrosThread> as *mut RrosThread);

                return Ok(0);
            }

            if rros_get_index(h) == currh {
                return Err(kernel::Error::EDEADLK);
            }

            ret = Ok(0);
            let mut test_no_owner = 0; // goto test_no_owner
            let mut flags = lock::raw_spin_lock_irqsave();
            let curr_lock = &mut (*curr.locked_data().get()).lock as *mut bindings::hard_spinlock_t;
            raw_spin_lock(curr_lock);
            if fast_mutex_is_claimed(h) == true {
                oldh = atomic_read(lockp) as u32;
                test_no_owner = 1;
            }

            let mut redo = 0;
            loop {
                if test_no_owner == 0 {
                    oldh = atomic_cmpxchg(lockp, h as i32, mutex_fast_claim(h) as i32) as u32;
                    if oldh == h {
                        break;
                    }
                }
                if oldh == RROS_NO_HANDLE {
                    raw_spin_unlock(curr_lock);
                    lock::raw_spin_unlock_irqrestore(flags);
                    redo = 1;
                    break;
                }
                h = oldh;
                if fast_mutex_is_claimed(h) == true {
                    break;
                }
                test_no_owner = 0;
            }
            if redo == 1 {
                continue;
            }
            pr_debug!("33333333333333333333333333333333");
            // owner = rros_get_factory_element_by_fundle(&rros_thread_factory,rros_get_index(h),struct RrosThread);
            let owner_ptr =
                Arc::into_raw(owner.clone()) as *mut SpinLock<RrosThread> as *mut RrosThread;
            if owner_ptr == 0 as *mut RrosThread {
                untrack_owner(mutex);
                raw_spin_unlock(curr_lock);
                lock::raw_spin_unlock_irqrestore(flags);
                return Err(kernel::Error::EOWNERDEAD);
            }
            let ptr1 = Arc::into_raw((*mutex).owner.clone().unwrap()) as *mut SpinLock<RrosThread>
                as *mut RrosThread;
            let ptr2 = Arc::into_raw(owner.clone()) as *mut SpinLock<RrosThread> as *mut RrosThread;
            if ptr1 != ptr2 {
                track_owner(mutex, owner.clone());
            } else {
                // rros_put_element(&owner->element);
            }
            let owner_lock =
                &mut (*owner.locked_data().get()).lock as *mut bindings::hard_spinlock_t;
            raw_spin_lock(owner_lock);
            let state = (*curr.locked_data().get()).state;
            if state & T_WOLI != 0 {
                detect_inband_owner(mutex, curr as *mut SpinLock<RrosThread> as *mut RrosThread);
            }
            let wprio = (*curr.locked_data().get()).wprio;
            let owner_wprio = (*owner.locked_data().get()).wprio;
            if wprio > owner_wprio {
                let info = (*owner.locked_data().get()).info;
                let wwake = (*owner.locked_data().get()).wwake;
                if info & T_WAKEN != 0 && wwake == &mut (*mutex).wchan as *mut rros_wait_channel {
                    let temp = Arc::from_raw(curr as *const SpinLock<RrosThread>);
                    set_current_owner_locked(mutex, temp.clone());
                    let owner_rq_lock = &mut (*(*owner.locked_data().get()).rq.unwrap()).lock
                        as *mut bindings::hard_spinlock_t;
                    raw_spin_lock(owner_rq_lock);
                    (*owner.locked_data().get()).info |= T_ROBBED;
                    raw_spin_unlock(owner_rq_lock);
                    raw_spin_unlock(owner_lock);
                    disable_inband_switch(curr as *mut SpinLock<RrosThread> as *mut RrosThread);
                    if (*(*mutex).wchan.wait_list).is_empty() == false {
                        currh = mutex_fast_claim(currh);
                    }
                    atomic_set(lockp, get_owner_handle(currh, mutex) as i32);
                    raw_spin_unlock(curr_lock);
                    lock::raw_spin_unlock_irqrestore(flags);
                    return ret;
                }

                if (*(*mutex).wchan.wait_list).is_empty() {
                    let wait_next = (*curr.locked_data().get()).wait_next;
                    (*(*mutex).wchan.wait_list).add_head((*wait_next).value.clone());
                } else {
                    let mut flag = 1;
                    for i in (*(*mutex).wchan.wait_list).len()..=1 {
                        let curr_wprio = (*(*curr).locked_data().get()).wprio;
                        let wprio_in_list = (*(*(*mutex).wchan.wait_list)
                            .get_by_index(i)
                            .unwrap()
                            .value
                            .clone()
                            .locked_data()
                            .get())
                        .wprio;
                        if curr_wprio <= wprio_in_list {
                            flag = 0;
                            let wait_next = (*curr.locked_data().get()).wait_next;
                            (*(*mutex).wchan.wait_list)
                                .enqueue_by_index(i, (*wait_next).value.clone());
                            break;
                        }
                    }
                    if flag == 1 {
                        let wait_next = (*curr.locked_data().get()).wait_next;
                        (*(*mutex).wchan.wait_list).add_head((*wait_next).value.clone());
                    }
                }

                if (*mutex).flags & RROS_MUTEX_PI as i32 != 0 {
                    raise_boost_flag(owner.clone());
                    if (*mutex).flags & RROS_MUTEX_CLAIMED as i32 != 0 {
                        (*(*mutex).next_booster).remove();
                    } else {
                        (*mutex).flags |= RROS_MUTEX_CLAIMED as i32;
                    }
                    (*mutex).wprio = (*curr.locked_data().get()).wprio;

                    let boosters = (*owner.locked_data().get()).boosters;
                    if (*boosters).is_empty() {
                        (*boosters).add_head((*((*mutex).next_booster)).value.clone());
                    } else {
                        let mut flag = 1;
                        for i in (*boosters).len()..=1 {
                            let wprio_in_list = (*(*boosters)
                                .get_by_index(i)
                                .unwrap()
                                .value
                                .clone()
                                .locked_data()
                                .get())
                            .wprio;
                            if (*mutex).wprio <= wprio_in_list {
                                flag = 0;
                                (*boosters)
                                    .enqueue_by_index(i, (*((*mutex).next_booster)).value.clone());
                                break;
                            }
                        }
                        if flag == 1 {
                            (*boosters).add_head((*((*mutex).next_booster)).value.clone());
                        }
                    }
                    let temp = Arc::from_raw(curr as *const SpinLock<RrosThread>);
                    ret = inherit_thread_priority(owner.clone(), temp.clone(), temp.clone());
                } else {
                    let temp = Arc::from_raw(curr as *const SpinLock<RrosThread>);
                    ret = check_lock_chain(owner.clone(), temp.clone());
                }
            } else {
                if (*(*mutex).wchan.wait_list).is_empty() {
                    let wait_next = (*curr.locked_data().get()).wait_next;
                    (*(*mutex).wchan.wait_list).add_head((*wait_next).value.clone());
                } else {
                    let mut flag = 1;
                    for i in (*(*mutex).wchan.wait_list).len()..=1 {
                        let curr_wprio = (*curr.locked_data().get()).wprio;
                        let wprio_in_list = (*(*(*mutex).wchan.wait_list)
                            .get_by_index(i)
                            .unwrap()
                            .value
                            .clone()
                            .locked_data()
                            .get())
                        .wprio;
                        if curr_wprio <= wprio_in_list {
                            flag = 0;
                            let wait_next = (*curr.locked_data().get()).wait_next;
                            (*(*mutex).wchan.wait_list)
                                .enqueue_by_index(i, (*wait_next).value.clone());
                            break;
                        }
                    }
                    if flag == 1 {
                        let wait_next = (*curr.locked_data().get()).wait_next;
                        (*(*mutex).wchan.wait_list).add_head((*wait_next).value.clone());
                    }
                }
                let temp = Arc::from_raw(curr as *const SpinLock<RrosThread>);
                ret = check_lock_chain(owner.clone(), temp.clone());
            }
            raw_spin_unlock(owner_lock);
            if ret != Ok(0) {
                let curr_rq_lock = &mut (*(*curr.locked_data().get()).rq.unwrap()).lock
                    as *mut bindings::hard_spinlock_t;
                raw_spin_lock(curr_rq_lock);
                rros_sleep_on_locked(
                    timeout,
                    timeout_mode,
                    &(*((*mutex).clock)),
                    &mut (*mutex).wchan as *mut rros_wait_channel,
                );
                raw_spin_unlock(curr_rq_lock);
                raw_spin_unlock(curr_lock);
                lock::raw_spin_unlock_irqrestore(flags);
                ret = wait_mutex_schedule(mutex);
                flags = lock::raw_spin_lock_irqsave();
            } else {
                raw_spin_unlock(curr_lock);
            }

            finish_mutex_wait(mutex);
            raw_spin_lock(curr_lock);
            (*curr.locked_data().get()).wwake = 0 as *mut rros_wait_channel;
            let curr_rq_lock = &mut (*(*curr.locked_data().get()).rq.unwrap()).lock
                as *mut bindings::hard_spinlock_t;
            raw_spin_lock(curr_rq_lock);
            (*curr.locked_data().get()).info &= !T_WAKEN;
            if ret != Ok(0) {
                raw_spin_unlock(curr_rq_lock);
                raw_spin_unlock(curr_lock);
                lock::raw_spin_unlock_irqrestore(flags);
                return ret;
            }
            let info = (*curr.locked_data().get()).info;
            if info & T_ROBBED != 0 {
                raw_spin_unlock(curr_rq_lock);
                // if timeout_mode != timeout::RrosTmode::RrosRel ||
                // 	timeout == 0 ||
                // 	rros_get_stopped_timer_delta((*curr).locked_data().get().rtimer) != 0 {
                // 	// raw_spin_unlock(&curr->lock);
                // 	// raw_spin_unlock_irqrestore(&mutex->lock, flags);
                // 	continue;
                // } // todo rros_get_stopped_timer_delta
                raw_spin_unlock(curr_lock);
                lock::raw_spin_unlock_irqrestore(flags);
                return Err(kernel::Error::ETIMEDOUT);
            }
            raw_spin_unlock(curr_rq_lock);

            disable_inband_switch(curr as *mut SpinLock<RrosThread> as *mut RrosThread);
            if (*(*mutex).wchan.wait_list).is_empty() == false {
                currh = mutex_fast_claim(currh);
            }
            atomic_set(lockp, get_owner_handle(currh, mutex) as i32);

            raw_spin_unlock(curr_lock);
            lock::raw_spin_unlock_irqrestore(flags);
            return ret;
        } // goto redo
    } // unsafe
}

pub fn transfer_ownership(mutex: *mut RrosMutex, lastowner: Arc<SpinLock<RrosThread>>) {
    unsafe {
        let lockp = (*mutex).fastlock;
        let mut n_ownerh: FundleT = 0;

        // assert_hard_lock(&mutex->lock);

        if (*(*mutex).wchan.wait_list).is_empty() {
            untrack_owner(mutex);
            atomic_set(lockp, RROS_NO_HANDLE as i32);
            return;
        }

        let n_owner = (*(*mutex).wchan.wait_list)
            .get_head()
            .unwrap()
            .value
            .clone();
        let lock = &mut (*n_owner.locked_data().get()).lock as *mut bindings::hard_spinlock_t;
        raw_spin_lock(lock);
        (*n_owner.locked_data().get()).wwake = &mut (*mutex).wchan as *mut rros_wait_channel;
        (*n_owner.locked_data().get()).wchan = None;
        raw_spin_unlock(lock);
        (*(*n_owner.locked_data().get()).wait_next).remove();
        set_current_owner_locked(mutex, n_owner.clone());
        rros_wakeup_thread(n_owner.clone(), T_PEND, T_WAKEN);

        if (*mutex).flags & RROS_MUTEX_CLAIMED as i32 != 0 {
            clear_boost_locked(mutex, lastowner.clone(), RROS_MUTEX_CLAIMED as i32);
        }
        // n_ownerh = get_owner_handle(fundle_of(n_owner), mutex);
        if (*(*mutex).wchan.wait_list).is_empty() == false {
            n_ownerh = mutex_fast_claim(n_ownerh);
        }

        atomic_set(lockp, n_ownerh as i32);
    }
}

pub fn __rros_unlock_mutex(mutex: *mut RrosMutex) -> Result<i32> {
    let mut curr = unsafe { &mut *rros_current() };
    let owner = unsafe { Arc::from_raw(curr as *const SpinLock<RrosThread>) };
    let flags: u32 = 0;
    let currh: FundleT = 0;
    let mut h: FundleT = 0;
    let mut lockp = 0 as *mut bindings::atomic_t;

    // trace_rros_mutex_unlock(mutex);

    if enable_inband_switch(curr as *mut SpinLock<RrosThread> as *mut RrosThread) == false {
        return Ok(0);
    }

    lockp = unsafe { (*mutex).fastlock };
    // currh = fundle_of(curr);

    let flags = lock::raw_spin_lock_irqsave();
    let lock = unsafe { &mut (*curr.locked_data().get()).lock as *mut bindings::hard_spinlock_t };
    raw_spin_lock(lock);

    unsafe {
        if (*mutex).flags & RROS_MUTEX_CEILING as i32 != 0 {
            clear_boost_locked(mutex, owner.clone(), RROS_MUTEX_CEILING as i32);
        }
    }
    h = atomic_read(lockp) as u32;
    h = atomic_cmpxchg(lockp, h as i32, RROS_NO_HANDLE as i32) as u32;
    if (h & !RROS_MUTEX_FLCEIL) != currh {
        transfer_ownership(mutex, owner.clone());
    } else {
        if h != currh {
            atomic_set(lockp, RROS_NO_HANDLE as i32);
        }
        untrack_owner(mutex);
    }
    raw_spin_unlock(lock);
    lock::raw_spin_unlock_irqrestore(flags);
    Ok(0)
}

pub fn rros_unlock_mutex(mutex: *mut RrosMutex) -> Result<usize> {
    unsafe {
        let curr = &mut *rros_current();
        // FundleT currh = fundle_of(curr), h;

        premmpt::running_inband()?;

        // h = rros_get_index(atomic_read(mutex->fastlock));
        // if (RROS_WARN_ON_ONCE(CORE, h != currh))
        // 	return;

        __rros_unlock_mutex(mutex);
        rros_schedule();
        Ok(0)
    }
}

pub fn rros_drop_tracking_mutexes(curr: *mut RrosThread) {
    unsafe {
        let mut mutex = 0 as *mut RrosMutex;
        let flags: u32 = 0;
        let mut h: FundleT = 0;

        let mut flags = lock::raw_spin_lock_irqsave();

        while (*(*curr).trackers).is_empty() == false {
            mutex = Arc::into_raw((*(*(*curr).trackers).get_head().unwrap()).value.clone())
                as *mut SpinLock<RrosMutex> as *mut RrosMutex;
            lock::raw_spin_unlock_irqrestore(flags);
            h = rros_get_index(atomic_read((*mutex).fastlock) as FundleT);
            // if (h == fundle_of(curr)) {
            // 	__rros_unlock_mutex(mutex);
            // } else {
            // 	// raw_spin_lock_irqsave(&mutex->lock, flags);
            // 	if (*mutex).owner == curr{
            // 		untrack_owner(mutex);
            // 	}
            // 	// raw_spin_unlock_irqrestore(&mutex->lock, flags);
            // }
            flags = lock::raw_spin_lock_irqsave();
        }
        lock::raw_spin_unlock_irqrestore(flags);
    }
}

pub fn wchan_to_mutex(wchan: *mut rros_wait_channel) -> *mut RrosMutex {
    return kernel::container_of!(wchan, RrosMutex, wchan) as *mut RrosMutex;
}

pub fn rros_reorder_mutex_wait(
    waiter: Arc<SpinLock<RrosThread>>,
    originator: Arc<SpinLock<RrosThread>>,
) -> Result<i32> {
    unsafe {
        let waiter_ptr =
            Arc::into_raw(waiter.clone()) as *mut SpinLock<RrosThread> as *mut RrosThread;
        let originator_ptr =
            Arc::into_raw(originator.clone()) as *mut SpinLock<RrosThread> as *mut RrosThread;
        let mutex = wchan_to_mutex(Arc::into_raw((*waiter_ptr).wchan.clone().unwrap())
            as *mut SpinLock<rros_wait_channel>
            as *mut rros_wait_channel);
        let owner = (*mutex).owner.clone().unwrap();
        let owner_ptr =
            Arc::into_raw(owner.clone()) as *mut SpinLock<RrosThread> as *mut RrosThread;
        // assert_hard_lock(&waiter->lock);
        // assert_hard_lock(&originator->lock);

        let mutex_lock = &mut (*mutex).lock as *mut bindings::hard_spinlock_t;
        raw_spin_lock(mutex_lock);
        if owner_ptr == originator_ptr {
            raw_spin_unlock(mutex_lock);
            return Err(kernel::Error::EDEADLK);
        }

        (*(*waiter_ptr).wait_next).remove();
        if (*(*mutex).wchan.wait_list).is_empty() {
            (*(*mutex).wchan.wait_list).add_head((*((*waiter_ptr).wait_next)).value.clone());
        } else {
            let mut flag = 1;
            for i in (*(*mutex).wchan.wait_list).len()..=1 {
                let wprio_in_list = (*(*(*mutex).wchan.wait_list)
                    .get_by_index(i)
                    .unwrap()
                    .value
                    .clone()
                    .locked_data()
                    .get())
                .wprio;
                if (*waiter_ptr).wprio <= wprio_in_list {
                    flag = 0;
                    (*(*mutex).wchan.wait_list)
                        .enqueue_by_index(i, (*((*waiter_ptr).wait_next)).value.clone());
                    break;
                }
            }
            if flag == 1 {
                (*(*mutex).wchan.wait_list).add_head((*((*waiter_ptr).wait_next)).value.clone());
            }
        }

        if (*mutex).flags & RROS_MUTEX_PI as i32 == 0 {
            raw_spin_unlock(mutex_lock);
            return Ok(0);
        }

        (*mutex).wprio = (*waiter_ptr).wprio;
        let owner_lock = &mut (*owner.locked_data().get()).lock as *mut bindings::hard_spinlock_t;
        raw_spin_lock(owner_lock);

        if (*mutex).flags & RROS_MUTEX_CLAIMED as i32 != 0 {
            (*(*mutex).next_booster).remove();
        } else {
            (*mutex).flags |= RROS_MUTEX_CLAIMED as i32;
            raise_boost_flag(owner.clone());
        }

        let boosters = (*owner.locked_data().get()).boosters;
        if (*boosters).is_empty() {
            (*boosters).add_head((*((*mutex).next_booster)).value.clone());
        } else {
            let mut flag = 1;
            for i in (*boosters).len()..=1 {
                let wprio_in_list = (*(*boosters)
                    .get_by_index(i)
                    .unwrap()
                    .value
                    .clone()
                    .locked_data()
                    .get())
                .wprio;
                if (*mutex).wprio <= wprio_in_list {
                    flag = 0;
                    (*boosters).enqueue_by_index(i, (*((*mutex).next_booster)).value.clone());
                    break;
                }
            }
            if flag == 1 {
                (*boosters).add_head((*((*mutex).next_booster)).value.clone());
            }
        }

        raw_spin_unlock(owner_lock);
        raw_spin_unlock(mutex_lock);
        return adjust_boost(owner.clone(), waiter.clone(), mutex, originator.clone());
    }
}

pub fn rros_follow_mutex_depend(
    wchan: Arc<SpinLock<rros_wait_channel>>,
    originator: Arc<SpinLock<RrosThread>>,
) -> Result<i32> {
    let wchan = Arc::into_raw(wchan) as *mut SpinLock<rros_wait_channel> as *mut rros_wait_channel;
    let originator_ref =
        Arc::into_raw(originator.clone()) as *mut SpinLock<RrosThread> as *mut RrosThread;
    let mutex = wchan_to_mutex(wchan);
    let mut waiter = 0 as *mut RrosThread;
    let mut ret: Result<i32> = Ok(0);

    // assert_hard_lock(&originator->lock);

    let mutex_lock = unsafe { &mut (*mutex).lock as *mut bindings::hard_spinlock_t };
    raw_spin_lock(mutex_lock);
    unsafe {
        let owner_ref = Arc::into_raw((*mutex).owner.clone().unwrap()) as *mut SpinLock<RrosThread>
            as *mut RrosThread;
        if owner_ref == originator_ref {
            raw_spin_unlock(mutex_lock);
            return Err(kernel::Error::EDEADLK);
        }

        for j in 1..=(*(*mutex).wchan.wait_list).len() {
            let waiter_node = (*(*mutex).wchan.wait_list)
                .get_by_index(j)
                .unwrap()
                .value
                .clone();
            waiter = Arc::into_raw(waiter_node) as *mut SpinLock<RrosThread> as *mut RrosThread;

            let waiter_lock = &mut (*waiter).lock as *mut bindings::hard_spinlock_t;
            raw_spin_lock(waiter_lock);
            let mut depend = (*waiter).wchan.clone();
            if depend.is_some() {
                let func;
                match (*depend.clone().unwrap().locked_data().get()).follow_depend {
                    Some(f) => func = f,
                    None => {
                        pr_warn!("rros_follow_mutex_depend:follow_depend function error");
                        return Err(kernel::Error::EINVAL);
                    }
                }
                ret = func(depend.as_mut().unwrap().clone(), originator.clone());
            }
            raw_spin_unlock(waiter_lock);
            if ret != Ok(0) {
                break;
            }
        }
        raw_spin_unlock(mutex_lock);

        return ret;
    }
}

pub fn rros_commit_mutex_ceiling(mutex: *mut RrosMutex) -> Result<i32> {
    unsafe {
        let curr = &mut *rros_current();
        let thread = unsafe { Arc::from_raw(curr as *const SpinLock<RrosThread>) };
        let lockp = (*mutex).fastlock;
        let flags: u32 = 0;
        let mut oldh: i32 = 0;
        let mut h: i32 = 0;

        let flags = lock::raw_spin_lock_irqsave();

        // if (!rros_is_mutex_owner(lockp, fundle_of(curr)) ||(mutex->flags & RROS_MUTEX_CEILING))
        // 	goto out;

        ref_and_track_owner(mutex, thread.clone());
        ceil_owner_priority(mutex, thread.clone());

        loop {
            h = atomic_read(lockp);
            oldh = atomic_cmpxchg(lockp, h, mutex_fast_ceil(h as u32) as i32);
            if oldh == h {
                break;
            }
        }
        // out:
        lock::raw_spin_unlock_irqrestore(flags);
        Ok(0)
    }
}

pub fn test_mutex() -> Result<i32> {
    pr_debug!("mutex test in~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
    pr_debug!("test_mutex rros_current address is {:p}", rros_current());
    let mut kmutex = RrosKMutex::new();
    let mut kmutex = &mut kmutex as *mut RrosKMutex;
    let mut mutex = RrosMutex::new();
    unsafe { (*kmutex).mutex = &mut mutex as *mut RrosMutex };
    rros_init_kmutex(kmutex);
    pr_debug!("init ok~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
    rros_lock_kmutex(kmutex);
    pr_debug!("lock ok~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
    rros_unlock_kmutex(kmutex);
    pr_debug!("unlock ok~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
    Ok(0)
}

pub fn rros_init_kmutex(kmutex: *mut RrosKMutex) {
    unsafe {
        atomic_set((*kmutex).fastlock, 0);
        rros_init_mutex_pi(
            (*kmutex).mutex,
            &mut RROS_MONO_CLOCK as *mut RrosClock,
            (*kmutex).fastlock,
        );
    }
}

pub fn rros_init_mutex_pi(
    mutex: *mut RrosMutex,
    clock: *mut RrosClock,
    fastlock: *mut bindings::atomic_t,
) {
    __rros_init_mutex(mutex, clock, fastlock, 0);
}

pub fn rros_init_mutex_pp(
    mutex: *mut RrosMutex,
    clock: *mut RrosClock,
    fastlock: *mut bindings::atomic_t,
    ceiling: u32,
) {
    __rros_init_mutex(mutex, clock, fastlock, ceiling);
}

pub fn rros_lock_kmutex(kmutex: *mut RrosKMutex) -> Result<i32> {
    unsafe { return rros_lock_mutex((*kmutex).mutex) };
}

pub fn rros_lock_mutex(mutex: *mut RrosMutex) -> Result<i32> {
    return rros_lock_mutex_timeout(mutex, timeout::RROS_INFINITE, timeout::RrosTmode::RrosRel);
}

pub fn rros_unlock_kmutex(kmutex: *mut RrosKMutex) -> Result<usize> {
    unsafe { return rros_unlock_mutex((*kmutex).mutex) };
}
