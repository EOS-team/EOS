use crate::{
    clock::{RrosClock, RROS_MONO_CLOCK},
    sched::{rros_schedule, RrosThread, RrosThreadWithLock, RrosValue},
    thread::{
        rros_current, rros_notify_thread, rros_sleep_on, rros_wakeup_thread, KthreadRunner,
        T_BCAST, T_BREAK, T_NOMEM, T_PEND, T_RMID, T_TIMEO, T_WOLI,
    },
    timeout::{self, timeout_nonblock, RROS_INFINITE},
    uapi::rros::thread::RROS_HMDIAG_SYSDEMOTE,
};

use alloc::sync::Arc;

use core::{clone::Clone, ops::FnMut, ptr::NonNull, sync::atomic::AtomicBool};

use kernel::{
    bindings, delay,
    ktime::KtimeT,
    linked_list::List,
    prelude::*,
    sync::{HardSpinlock, Lock, SpinLock},
    Result,
};

pub const RROS_WAIT_PRIO: usize = 1 << 0;

pub struct RrosWaitChannel {
    // pub name: &'static CStr,
    pub wait_list: List<Arc<RrosThreadWithLock>>,
    pub reorder_wait: Option<
        fn(waiter: Arc<SpinLock<RrosThread>>, originator: Arc<SpinLock<RrosThread>>) -> Result<i32>,
    >,
    pub follow_depend: Option<
        fn(
            wchan: Arc<SpinLock<RrosWaitChannel>>,
            originator: Arc<SpinLock<RrosThread>>,
        ) -> Result<i32>,
    >,
}

pub struct RrosWaitQueue {
    pub flags: i32,
    pub clock: *mut RrosClock,
    pub wchan: RrosWaitChannel,
    pub lock: HardSpinlock,
}

impl RrosWaitQueue {
    pub fn new(clock: *mut RrosClock, flags: i32) -> Self {
        let mut wait = RrosWaitQueue {
            flags,
            clock,
            wchan: RrosWaitChannel {
                wait_list: List::new(),
                reorder_wait: None,
                follow_depend: None,
            },
            lock: HardSpinlock::new(),
        };
        wait.init(clock, flags);
        wait
    }
    // delete `key:*mut bindings::lock_class_key`, and `name:&'static CStr`.
    pub fn init(&mut self, clock: *mut RrosClock, flags: i32) {
        self.flags = flags;
        self.clock = clock;
        self.lock.init();
        // self.wchan.name = name;
        // self.wchan.wait_list.
        // init_list_head!(&self.wchan.wait_list);
        // lockdep_set_class_and_name(&wq->lock, key, name);
        // This is used during debugging.
    }
    pub fn flush(&mut self, reason: i32) {
        // rros_flush_wait
        let flags = self.lock.raw_spin_lock_irqsave();

        self.flush_locked(reason);

        self.lock.raw_spin_unlock_irqrestore(flags);
    }
    pub fn flush_locked(&mut self, reason: i32) {
        // rros_flush_wait_locked
        // trace_rros_flush_wait(wq);
        let list = &mut self.wchan.wait_list;
        while let Some(waiter) = list.pop_front() {
            // locked_thread
            // rewrap
            let waiter = unsafe { RrosThreadWithLock::transmute_to_original(waiter) };
            rros_wakeup_thread(waiter, T_PEND, reason);
        }
    }

    pub fn destory(&mut self) {
        self.flush(T_RMID as i32);
        unsafe { rros_schedule() };
    }

    pub fn wake_up(
        &mut self,
        waiter: *mut RrosThread,
        reason: i32,
    ) -> Option<Arc<SpinLock<RrosThread>>> {
        // trace_rros_wake_up(wq);
        // assert!(self.lock) //TODO:
        if self.wchan.wait_list.is_empty() {
            return None;
        } else {
            if waiter.is_null() {
                let list = &mut self.wchan.wait_list;
                let waiter = list.pop_front().unwrap();
                let waiter = unsafe { RrosThreadWithLock::transmute_to_original(waiter) };
                rros_wakeup_thread(waiter.clone(), T_PEND, reason);
                Some(waiter)
            } else {
                unimplemented!()
                // let mut list = self.wchan.wait_list;
                // let thread = RrosThreadWithLock();
                // unsafe{list.remove(&thread)};
                // rros_wakeup_thread(thread, T_PEND, reason);
            }
        }
    }

    pub fn locked_add(&mut self, timeout: KtimeT, timeout_mode: timeout::RrosTmode) {
        // rros_add_wait_queue
        let curr = unsafe { &mut *(*rros_current()).locked_data().get() };

        // assert!(self.lock) //TODO:
        pr_debug!(
            "before adding the wait list length is {}",
            self.wchan.wait_list.len()
        );
        if curr.state & T_WOLI != 0 && curr.inband_disable_count.atomic_read() > 0 {
            let _ret = rros_notify_thread(
                curr as *const _ as *mut RrosThread,
                RROS_HMDIAG_SYSDEMOTE as u32,
                RrosValue::new(),
            );
        }
        if self.flags as usize & RROS_WAIT_PRIO == 0 {
            let current = unsafe { RrosThreadWithLock::new_from_curr_thread() };

            self.wchan.wait_list.push_back(current);
        } else {
            if self.wchan.wait_list.is_empty() {
                let current = unsafe { RrosThreadWithLock::new_from_curr_thread() };
                self.wchan.wait_list.push_back(current)
            } else {
                // To join according to priority, you can look at `add_by_prio` in types_test.rs.
                let prio = curr.wprio;
                let mut last = self.wchan.wait_list.cursor_back_mut();
                let mut stop_flag = false;
                while let Some(cur) = last.current() {
                    if prio <= (*cur).get_wprio() {
                        let cur = NonNull::new(cur as *const _ as *mut RrosThreadWithLock).unwrap();
                        let item = unsafe { RrosThreadWithLock::new_from_curr_thread() };
                        unsafe { self.wchan.wait_list.insert_after(cur, item) };
                        stop_flag = true;
                        break;
                    }
                    last.move_prev();
                }
                if !stop_flag {
                    let item = unsafe { RrosThreadWithLock::new_from_curr_thread() };
                    self.wchan.wait_list.push_front(item);
                }
            }
        }
        pr_debug!(
            "after adding the wait list length is {}",
            self.wchan.wait_list.len()
        );
        rros_sleep_on(
            timeout,
            timeout_mode,
            unsafe { &*self.clock },
            &mut self.wchan as *mut RrosWaitChannel,
        ); // It must be ensured that wchan will not be released.
    }

    pub fn wait_schedule(&mut self) -> i32 {
        // rros_wait_schedule
        let _curr: *mut SpinLock<RrosThread> = rros_current();

        unsafe { rros_schedule() };

        // trace_rros_finish_wait(wq);

        let info = unsafe { (*(*rros_current()).locked_data().get()).info };
        if info & T_RMID != 0 {
            return -(bindings::EIDRM as i32);
        }
        if info & T_NOMEM != 0 {
            return -(bindings::ENOMEM as i32);
        }

        let mut ret = 0;
        if info & (T_TIMEO | T_BREAK) != 0 {
            let flags = self.lock.raw_spin_lock_irqsave();
            if !self.wchan.wait_list.is_empty() {
                let r = unsafe { RrosThreadWithLock::new_from_curr_thread() };
                unsafe { self.wchan.wait_list.remove(&r) };
                if info & T_TIMEO != 0 {
                    ret = -(bindings::ETIMEDOUT as i32);
                } else if info & T_BREAK != 0 {
                    ret = -(bindings::EINTR as i32);
                }
            }
            // unsafe{bindings::_raw_spin_unlock_irqrestore(&mut self.lock as *const _ as *mut bindings::raw_spinlock, flags)}
            self.lock.raw_spin_unlock_irqrestore(flags);
        }
        return ret;
        // else if (IS_ENABLED(CONFIG_RROS_DEBUG_CORE)) {
        //     bool empty;
        //     raw_spin_lock_irqsave(&wq->lock, flags);
        //     empty = list_empty(&curr->wait_next);
        //     raw_spin_unlock_irqrestore(&wq->lock, flags);
        //     RROS_WARN_ON_ONCE(CORE, !empty);
        // }
    }

    #[inline]
    pub fn wait_timeout(
        &mut self,
        timeout: KtimeT,
        time_mode: timeout::RrosTmode,
        mut get_cond: impl FnMut() -> bool,
    ) -> i32 {
        // implementation of rros_wait_event_timeout

        // let mut flags = unsafe{rust_helper_raw_spin_lock_irqsave(&mut self.lock as *const _ as *mut bindings::hard_spinlock_t)};
        let mut flags = self.lock.raw_spin_lock_irqsave();
        let mut ret: i32 = 0;
        if !get_cond() {
            if timeout_nonblock(timeout) {
                ret = -(bindings::EAGAIN as i32);
            } else {
                let mut bcast: u32;
                loop {
                    pr_debug!("I am in the wait timeout loop");
                    self.locked_add(timeout, time_mode);
                    pr_debug!("I am in the wait timeout loop aftering adding the timeout");
                    // unsafe{rust_helper_raw_spin_unlock_irqrestore(&mut self.lock as *const _ as *mut bindings::hard_spinlock_t, flags)};
                    self.lock.raw_spin_unlock_irqrestore(flags);
                    ret = self.wait_schedule();
                    bcast = unsafe { (*(*rros_current()).locked_data().get()).info } & T_BCAST;
                    // flags = unsafe{rust_helper_raw_spin_lock_irqsave(&mut self.lock as *const _ as *mut bindings::hard_spinlock_t)};
                    flags = self.lock.raw_spin_lock_irqsave();
                    if ret != 0 || get_cond() || bcast != 0 {
                        break;
                    }
                }
            }
        }
        self.lock.raw_spin_unlock_irqrestore(flags);
        ret
    }
    pub fn is_active(&self) -> bool {
        // rros_wait_active
        !self.wchan.wait_list.is_empty()
    }

    pub fn wake_up_head(&mut self) -> Option<Arc<SpinLock<RrosThread>>> {
        self.wake_up(core::ptr::null_mut(), 0)
    }

    // pub fn add_wait_queue(&mut self,timeout:KtimeT,timeout_mode:timeout::RrosTmode){
    //     unsafe{
    //         fn rust_helper_atomic_read(v: *mut bindings::atomic_t) -> i32;
    //     }
    //     //rros_add_wait_queue, wq->lock held, hard irqs off
    //     let curr = rros_current();

    // 	// assert_hard_lock(&wq->lock);
    //     let ref_curr = &mut unsafe{(*(*curr).locked_data().get())};
    //     if (ref_curr.state & T_WOLI != 0  && rust_helper_atomic_read(&ref_curr.inband_disable_count) > 0){
    //         rros_notify_thread(ref_curr, RROS_HMDIAG_LKSLEEP as u32, RrosValue::new_nil());
    //     }

    //     if self.flags & RROS_WAIT_PRIO ==0{
    //         list_add
    //     }
    // }
}

#[allow(dead_code)]
pub fn wait_test() {
    use core::sync::atomic::Ordering::Relaxed;
    use kernel::c_str;
    use kernel::prelude::*;
    let mut queue = unsafe {
        RrosWaitQueue::new(
            &mut RROS_MONO_CLOCK as *mut RrosClock,
            RROS_WAIT_PRIO as i32,
        )
    };
    let ptr_queue = &mut queue as *mut RrosWaitQueue;
    let mut flag = AtomicBool::new(false);
    let flag_ptr = &mut flag as *mut AtomicBool;
    let mut runner = KthreadRunner::new_empty();
    runner.init(
        Box::try_new(move || {
            for _i in 0..10 {
                unsafe {
                    (*ptr_queue).wait_timeout(RROS_INFINITE, timeout::RrosTmode::RrosRel, || {
                        if flag.load(Relaxed) == true {
                            flag.store(false, Relaxed);
                            true
                        } else {
                            false
                        }
                    })
                };
            }
        })
        .unwrap(),
    );
    runner.run(c_str!("test wait"));
    for _i in 0..10 {
        delay::usleep_range(10000, 20000);
        let flags = unsafe { (*ptr_queue).lock.raw_spin_lock_irqsave() };
        unsafe { (*flag_ptr).store(true, Relaxed) };

        unsafe { (*ptr_queue).flush_locked(0) }
        unsafe {
            (*ptr_queue).lock.raw_spin_unlock_irqrestore(flags);
        }
        unsafe { rros_schedule() };
    }
    pr_debug!("wait test done")
}
