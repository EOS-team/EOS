// SPDX-License-Identifier: GPL-2.0

//! kernel
//!
//! C header: [`include/linux/kernel.h`](../../../../include/linux/kernel.h)

use crate::{bindings, c_types::*, prelude::*};

// FIXME: how to wrapper `...` in parameters
/// The `printk` function is a wrapper around the `printk` function from the kernel.
#[inline]
pub fn _kasprintf_1(gfp: bindings::gfp_t, fmt: *const c_char, arg1: *const c_char) -> *mut c_char {
    unsafe { bindings::kasprintf(gfp, fmt, arg1) }
}

/// The `printk` function is a wrapper around the `printk` function from the kernel.
#[inline]
pub fn _kasprintf_2(
    gfp: bindings::gfp_t,
    fmt: *const c_char,
    arg1: *const c_char,
    arg2: *const c_char,
) -> *mut c_char {
    unsafe { bindings::kasprintf(gfp, fmt, arg1, arg2) }
}

/// A wrapper to store thread's exit code.
pub enum ThreadExitCode {
    /// Represent a thread exit successfully.
    Successfully,
    /// Represent a thread exit with a error.
    WithError(Error),
}

impl ThreadExitCode {
    /// Get thread's real exit code from self.
    fn get_exit_code(self) -> c_long {
        match self {
            Self::Successfully => 0,
            Self::WithError(error) => error.to_kernel_errno() as c_long,
        }
    }
}

/// Call Linux do_exit
pub fn do_exit(exit_code: ThreadExitCode) {
    unsafe {
        bindings::do_exit(exit_code.get_exit_code());
    }
}
