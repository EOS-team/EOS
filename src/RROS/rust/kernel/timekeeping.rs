// SPDX-License-Identifier: GPL-2.0

//! Kernel timekeeping code and accessor functions.
//!
//! C header: [`include/linux/ktime.h`](../../../../include/linux/timekeeping.h)

use crate::bindings;

/// Function `ktime_get_mono_fast_ns` gets the monotonic time in nanoseconds.
/// It calls the `ktime_get_mono_fast_ns` function from the bindings module to get the time.
/// It returns the time as an `i64`.
pub fn ktime_get_mono_fast_ns() -> i64 {
    unsafe { bindings::ktime_get_mono_fast_ns() as i64 }
}

/// Function `ktime_get_real_fast_ns` gets the real time in nanoseconds.
/// It calls the `ktime_get_real_fast_ns` function from the bindings module to get the time.
/// It returns the time as an `i64`.
pub fn ktime_get_real_fast_ns() -> i64 {
    unsafe { bindings::ktime_get_real_fast_ns() as i64 }
}
