use kernel::{c_types, prelude::*, spinlock_init, sync::SpinLock};

pub fn raw_spin_lock_init(lock: &mut SpinLock<i32>) {
    *lock = unsafe { SpinLock::new(1) };
    let pinned = unsafe { Pin::new_unchecked(lock) };
    spinlock_init!(pinned, "timerbase");
}

extern "C" {
    fn rust_helper_hard_local_irq_save() -> c_types::c_ulong;
    fn rust_helper_hard_local_irq_restore(flags: c_types::c_ulong);
    #[allow(dead_code)]
    fn rust_helper_preempt_enable();
    #[allow(dead_code)]
    fn rust_helper_preempt_disable();
    // fn rust_helper_raw_spin_lock_irqsave();
    // fn rust_helper_raw_spin_unlock_irqrestore();
}

pub fn hard_local_irq_save() -> u64 {
    unsafe { rust_helper_hard_local_irq_save() }
}

pub fn hard_local_irq_restore(flags: u64) {
    unsafe {
        rust_helper_hard_local_irq_restore(flags);
    }
}

// pub fn right_raw_spin_lock_irqsave(lock: *mut spinlock_t, flags: *mut u32) {
//     // let flags = unsafe { rust_helper_raw_local_irq_save() };
//     unsafe { rust_helper_raw_local_irq_save() };
//     // unsafe{rust_helper_preempt_disable();}
//     // return flags;
// }

// pub fn right_raw_spin_unlock_irqrestore(lock: *mut spinlock_t, flags: *mut u32) {
//     // let flags = unsafe { rust_helper_raw_local_irq_save() };
//     unsafe { rust_helper_raw_spin_unlock_irqrestore() };
//     // unsafe{rust_helper_preempt_disable();}
//     // return flags;
// }
