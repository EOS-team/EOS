// SPDX-License-Identifier: GPL-2.0

//! irq_pipeline
//!
//! C header: [`include/linux/irq_pipeline.h`](../../../../include/linux/irq_pipeline.h)

use core::option::Option;

use crate::{bindings, c_types::*, cpumask};

extern "C" {
    fn rust_helper_irq_send_oob_ipi(ipi: usize, cpumask: *const cpumask::CpumaskT);
    fn rust_helper_irq_get_TIMER_OOB_IPI() -> usize;
    fn rust_helper_irq_get_RESCHEDULE_OOB_IPI() -> usize;
}

/// `irq_send_oob_ipi`: A wrapper around `rust_helper_irq_send_oob_ipi` that sends an out-of-band IPI to the CPUs specified by the `cpumask`. It takes an IPI number and a pointer to a `cpumask::CpumaskT`.
pub fn irq_send_oob_ipi(ipi: usize, cpumask: *const cpumask::CpumaskT) {
    unsafe { rust_helper_irq_send_oob_ipi(ipi, cpumask) };
}

/// `irq_get_timer_oob_ipi`: A wrapper around `rust_helper_irq_get_TIMER_OOB_IPI` that returns the IPI number for the timer out-of-band interrupt.
pub fn irq_get_timer_oob_ipi() -> usize {
    unsafe { rust_helper_irq_get_TIMER_OOB_IPI() }
}

/// `irq_get_reschedule_oob_ipi`: A wrapper around `rust_helper_irq_get_RESCHEDULE_OOB_IPI` that returns the IPI number for the reschedule out-of-band interrupt.
pub fn irq_get_reschedule_oob_ipi() -> usize {
    unsafe { rust_helper_irq_get_RESCHEDULE_OOB_IPI() }
}

/// The `run_oob_call` function is a wrapper around the `bindings::run_oob_cal` function from the kernel bindings.
/// It calls the `bindings::run_oob_call` function.
pub fn run_oob_call(
    fn_: Option<unsafe extern "C" fn(arg: *mut c_void) -> c_int>,
    arg: *mut c_void,
) -> c_int {
    unsafe { bindings::run_oob_call(fn_, arg) }
}
