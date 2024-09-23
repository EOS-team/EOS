// SPDX-License-Identifier: GPL-2.0

//! irqstage
//!
//! C header: [`include/linux/irqstage.h`](../../../../include/linux/irqstage.h)

use crate::bindings;

/// The `PtRegs` struct wraps a pointer to a `bindings::pt_regs` from the kernel bindings.
#[derive(Copy, Clone)]
pub struct PtRegs {
    /// A pointer to `bindings::pt_regs`.
    pub ptr: *mut bindings::pt_regs,
}

impl PtRegs {
    /// Constructs a new struct with a pointer to `bindings::pt_regs`.
    pub fn from_ptr(ptr: *mut bindings::pt_regs) -> Self {
        PtRegs { ptr }
    }
}

/// The `IrqStage` struct wraps a pointer to a `bindings::irq_stage` from the kernel bindings.
#[derive(Copy, Clone)]
pub struct IrqStage {
    /// A pointer to `bindings::irq_stage`.
    pub ptr: *mut bindings::irq_stage,
}

impl IrqStage {
    /// Constructs a new struct with the `bindings::oob_stage` variable from the kernel bindings.
    pub fn get_oob_state() -> Self {
        unsafe {
            IrqStage {
                ptr: &mut bindings::oob_stage as *mut bindings::irq_stage,
            }
        }
    }

    /// Constructs a new struct with a pointer to `bindings::irq_stage`.
    pub fn from_ptr(ptr: *mut bindings::irq_stage) -> Self {
        IrqStage { ptr }
    }
}
