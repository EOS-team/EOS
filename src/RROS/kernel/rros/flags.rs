use crate::{
    clock::{RrosClock, RROS_MONO_CLOCK},
    sched::rros_schedule,
    timeout::{RrosTmode, RROS_INFINITE},
    wait::{RrosWaitQueue, RROS_WAIT_PRIO},
};
use core::cell::Cell;
use kernel::ktime::KtimeT;

extern "C" {
    fn rust_helper_smp_wmb();
}

pub struct RrosFlag {
    pub wait: RrosWaitQueue,
    pub raised: Cell<bool>,
}

impl RrosFlag {
    pub fn new() -> Self {
        RrosFlag {
            wait: unsafe {
                RrosWaitQueue::new(
                    &mut RROS_MONO_CLOCK as *mut RrosClock,
                    RROS_WAIT_PRIO as i32,
                )
            },
            raised: Cell::new(false),
        }
    }

    #[inline]
    pub fn init(&mut self) {
        self.wait.init(
            unsafe { &mut RROS_MONO_CLOCK as *mut RrosClock },
            RROS_WAIT_PRIO as i32,
        );
        self.raised = Cell::new(false);
    }

    #[allow(dead_code)]
    #[inline]
    pub fn destory(&mut self) {
        self.wait.destory();
    }

    // #[inline]
    // pub fn wait_timeout(&mut self, timeout : bindings:KtimeT,) -> bool{
    //     if self.raised == false{
    //         self.wait.wait_timeout(timeout);
    //     }
    //     self.raised
    // }

    #[inline]
    pub fn peek(cell: &Cell<bool>) -> bool {
        unsafe {
            rust_helper_smp_wmb();
        }
        cell.get()
    }

    #[inline]
    pub fn read(cell: &Cell<bool>) -> bool {
        if cell.get() {
            cell.set(false);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn wait(&mut self) -> i32 {
        let cell = &self.raised;

        self.wait
            .wait_timeout(RROS_INFINITE, RrosTmode::RrosRel, || Self::read(cell))
    }

    #[inline]
    pub fn wait_same(&mut self) -> i32 {
        let cell = &self.raised;

        self.wait
            .wait_timeout(RROS_INFINITE, RrosTmode::RrosRel, || Self::peek(cell))
    }

    #[inline]
    pub fn wait_timeout(&mut self, timeout: KtimeT, tmode: RrosTmode) -> i32 {
        let cell = &self.raised;

        self.wait.wait_timeout(timeout, tmode, || Self::read(cell))
    }

    #[inline]
    pub fn raise_nosched(&mut self) {
        let flags = self.wait.lock.raw_spin_lock_irqsave();
        self.raised.set(true);
        self.wait.flush_locked(0);
        self.wait.lock.raw_spin_unlock_irqrestore(flags);
    }

    #[inline]
    pub fn raise(&mut self) {
        // let flags = unsafe{bindings::_raw_spin_lock_irqsave(&mut self.wait.lock as *const _ as *mut bindings::raw_spinlock)};
        let flags = self.wait.lock.raw_spin_lock_irqsave();
        self.raised.set(true);
        self.wait.flush_locked(0);
        self.wait.lock.raw_spin_unlock_irqrestore(flags);
        // unsafe{bindings::_raw_spin_unlock_irqrestore(&mut self.wait.lock as *const _ as *mut bindings::raw_spinlock, flags)};

        unsafe { rros_schedule() };
    }

    #[allow(dead_code)]
    #[inline]
    pub fn flush_locked(&mut self, reason: i32) {
        self.wait.flush_locked(reason);
    }

    #[inline]
    pub fn flush_nosched(&mut self, reason: i32) {
        let flags: u64 = self.wait.lock.raw_spin_lock_irqsave();
        self.wait.flush_locked(reason);
        self.wait.lock.raw_spin_unlock_irqrestore(flags);
    }
}

// pub fn test_flag(){
//     use crate::thread::KthreadRunner;
//     let mut runner = KthreadRunner::new_empty();
//     let mut global_flag : Arc<RrosFlag>    = unsafe{Arc::try_new(core::mem::zeroed()).unwrap()};
//     let x = Arc::get_mut(&mut flag).unwrap();
//     x.init();
//     drop(x);
//     let mut runner = KthreadRunner::new_empty();
//     runner.init(Box::try_new(move||{
//         let mut flag = global_flag.clone();
//         for i in 0..10{
//             flag.as_ref().wait()
//         }
//     }).unwrap());

// }
