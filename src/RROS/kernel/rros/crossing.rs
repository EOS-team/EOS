use kernel::{bindings, completion::Completion, irq_work::IrqWork, Result};

use core::sync::atomic::{AtomicUsize, Ordering};

pub struct RrosCrossing {
    oob_refs: AtomicUsize,
    oob_done: Completion,
    oob_work: IrqWork,
}

impl RrosCrossing {
    pub fn new() -> Self {
        RrosCrossing {
            oob_refs: AtomicUsize::new(0),
            oob_done: Completion::new(),
            oob_work: IrqWork::new(),
        }
    }

    pub fn init(&mut self) -> Result<usize> {
        extern "C" {
            fn rust_helper_atomic_set(v: *mut bindings::atomic_t, i: i32);
        }
        unsafe {
            rust_helper_atomic_set(
                &mut self.oob_refs as *mut AtomicUsize as *mut bindings::atomic_t,
                1,
            )
        }
        self.oob_done.init_completion();
        self.oob_work.init_irq_work(rros_open_crossing)?;
        Ok(0)
    }

    #[inline]
    pub fn down(&self) {
        extern "C" {
            fn rust_helper_atomic_inc(v: *mut bindings::atomic_t);
        }
        unsafe { rust_helper_atomic_inc(&self.oob_refs as *const _ as *mut bindings::atomic_t) }
    }

    #[inline]
    pub fn up(&mut self) {
        extern "C" {
            fn rust_helper_atomic_dec_return(v: *mut bindings::atomic_t) -> i32;
        }
        if unsafe {
            rust_helper_atomic_dec_return(
                &mut self.oob_refs as *mut AtomicUsize as *mut bindings::atomic_t,
            )
        } == 0
        {
            let _ret = self.oob_work.irq_work_queue();
        }
    }

    #[inline]
    pub fn pass(&mut self) {
        extern "C" {
            fn rust_helper_atomic_dec_return(v: *mut bindings::atomic_t) -> i32;
        }
        if unsafe {
            rust_helper_atomic_dec_return(
                &mut self.oob_refs as *mut AtomicUsize as *mut bindings::atomic_t,
            )
        } > 0
        {
            self.oob_done.wait_for_completion();
        }
    }
}

unsafe extern "C" fn rros_open_crossing(work: *mut IrqWork) {
    let c = kernel::container_of!(work, RrosCrossing, oob_work) as *mut RrosCrossing;
    unsafe {
        (*c).oob_done.complete();
    }
}

#[allow(dead_code)]
pub fn rros_init_crossing(crossing: &mut RrosCrossing) -> Result<usize> {
    crossing.oob_refs.store(1, Ordering::Relaxed);
    crossing.oob_done.init_completion();
    crossing.oob_work.init_irq_work(rros_open_crossing)?;

    Ok(0)
}

// pub fn rros_reinit_crossing(crossing: &mut RrosCrossing) -> Result<usize> {
//     crossing.oob_refs.store(1, Ordering::Relaxed);
//     crossing.oob_done.reinit_completion();

//     Ok(0)
// }

pub fn rros_down_crossing(crossing: &mut RrosCrossing) -> Result<usize> {
    crossing.oob_refs.fetch_add(1, Ordering::SeqCst);

    Ok(0)
}

#[allow(dead_code)]
pub fn rros_up_crossing(crossing: &mut RrosCrossing) -> Result<usize> {
    // CAUTION: the caller must guarantee that rros_down_crossing() cannot
    // be invoked _after_ rros_pass_crossing() is entered for a given crossing.
    crossing.oob_refs.fetch_sub(1, Ordering::SeqCst);
    if crossing.oob_refs.load(Ordering::SeqCst) == 0 {
        crossing.oob_work.irq_work_queue()?;
    }

    Ok(0)
}

#[allow(dead_code)]
pub fn rros_pass_crossing(crossing: &mut RrosCrossing) -> Result<usize> {
    crossing.oob_refs.fetch_sub(1, Ordering::SeqCst);
    if crossing.oob_refs.load(Ordering::SeqCst) > 0 {
        crossing.oob_done.wait_for_completion();
    }

    Ok(0)
}
