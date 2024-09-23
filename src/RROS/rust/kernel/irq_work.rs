// SPDX-License-Identifier: GPL-2.0

//! irq_work
//!
//! C header: [`include/linux/irq_work.h`](../../../../include/linux/irq_work.h)

use crate::{
    bindings,
    error::{Error, Result},
};

extern "C" {
    fn rust_helper_init_irq_work(
        work: *mut bindings::irq_work,
        func: unsafe extern "C" fn(work: *mut IrqWork),
    );
}

/// The `IrqWork` struct wraps a `bindings::irq_work` from the kernel bindings.
#[repr(transparent)]
pub struct IrqWork(pub bindings::irq_work);

impl IrqWork {
    /// `new`: A constructor function that returns a new `IrqWork`. It creates a default `bindings::irq_work` and wraps it in an `IrqWork`.
    pub fn new() -> Self {
        let irq_work = bindings::irq_work::default();
        Self(irq_work)
    }

    /// Constructs a new struct from a pointer to `bindings::irq_work`.
    pub fn from_ptr<'a>(work: *mut bindings::irq_work) -> &'a mut IrqWork {
        unsafe { &mut *(work as *mut IrqWork) }
    }

    /// `init_irq_work`: A method that initializes the `IrqWork`. It takes a function pointer to a C function and passes it to the `rust_helper_init_irq_work` function along with a pointer to the `bindings::irq_work`. It returns `Ok(0)` if the initialization is successful.
    pub fn init_irq_work(
        &mut self,
        func: unsafe extern "C" fn(work: *mut IrqWork),
    ) -> Result<usize> {
        unsafe {
            rust_helper_init_irq_work(&mut self.0 as *mut bindings::irq_work, func);
        }
        Ok(0)
    }

    /// `irq_work_queue`: A method that queues the `IrqWork`. It calls the `bindings::irq_work_queue` function with a pointer to the `bindings::irq_work`. If the function returns `true`, it returns `Ok(0)`. Otherwise, it returns `Err(Error::EINVAL)`.
    pub fn irq_work_queue(&mut self) -> Result<usize> {
        let res = unsafe { bindings::irq_work_queue(&mut self.0 as *mut bindings::irq_work) };
        if res == true {
            Ok(0)
        } else {
            Err(Error::EINVAL)
        }
    }

    /// `get_ptr`: A method that returns a mutable pointer to the `bindings::irq_work`. It returns a pointer to the `bindings::irq_work` field of the `IrqWork`.
    pub fn get_ptr(&mut self) -> *mut bindings::irq_work {
        // unsafe { &mut self.0 as *mut bindings::irq_work }
        &mut self.0 as *mut bindings::irq_work
    }
}
