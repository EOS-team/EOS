// SPDX-License-Identifier: GPL-2.0

//! tick
//!
//! C header: [`include/linux/tick.h`](../../../../include/linux/tick.h)

use crate::{bindings, c_types::*};

use core::option::Option;

/// The `tick_install_proxy` function is a wrapper around the `bindings::tick_install_proxy` function from the kernel bindings.
pub fn tick_install_proxy(
    setup_proxy: Option<unsafe extern "C" fn(dev: *mut bindings::clock_proxy_device)>,
    cpumask: *mut bindings::cpumask,
) -> c_int {
    unsafe { bindings::tick_install_proxy(setup_proxy, cpumask as *const bindings::cpumask) }
}
