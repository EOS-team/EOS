// SPDX-License-Identifier: GPL-2.0

//! interrupt
//!
//! C header: [`include/linux/interrupt.h`](../../../../include/linux/interrupt.h)

use crate::{bindings, c_types};

/// The `__request_percpu_irq` function is a wrapper around the `bindings::__request_percpu_irq` function from the kernel bindings.
/// It calls the `bindings::__request_percpu_irq` function and returns a `c_int` value.
pub fn __request_percpu_irq(
    irq: c_types::c_uint,
    handler: bindings::irq_handler_t,
    flags: c_types::c_ulong,
    devname: *const c_types::c_char,
    percpu_dev_id: *mut c_types::c_void,
) -> c_types::c_int {
    unsafe { bindings::__request_percpu_irq(irq, handler, flags, devname, percpu_dev_id) }
}

/// The `__raise_softirq_irqoff` function is a wrapper around the `bindings::__raise_softirq_irqoff` function from the kernel bindings.
/// It calls the `bindings::__raise_softirq_irqoff` function.
pub fn __raise_softirq_irqoff(nr: c_types::c_uint) {
    unsafe { bindings::__raise_softirq_irqoff(nr) }
}
