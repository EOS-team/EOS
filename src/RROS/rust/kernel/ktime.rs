// SPDX-License-Identifier: GPL-2.0

//! ktime
//!
//! C header: [`include/linux/ktime.h`](../../../../include/linux/ktime.h)
use crate::{bindings, c_types::c_longlong};

/// The `KtimeT` type is a type alias for `i64`. It represents a ktime value in the kernel.
pub type KtimeT = i64;

extern "C" {
    fn rust_helper_ktime_sub(lhs: KtimeT, rhs: KtimeT) -> KtimeT;
    fn rust_helper_ktime_add_ns(kt: KtimeT, nsval: u64) -> KtimeT;
    fn rust_helper_ktime_add(kt: KtimeT, nsval: KtimeT) -> KtimeT;
    fn rust_helper_ktime_set(secs: i64, nsecs: usize) -> KtimeT;
    fn rust_helper_ktime_divns(kt: KtimeT, div: i64) -> i64;
    fn rust_helper_ktime_compare(cmp1: KtimeT, cmp2: KtimeT) -> KtimeT;
    fn rust_helper_timespec64_to_ktime(ts: bindings::timespec64) -> KtimeT;
    fn rust_helper_ktime_to_timespec64(kt: bindings::ktime_t) -> bindings::timespec64;
}

/// A wrapper for [`bindings::timespec64`].
#[derive(Default, Copy, Clone)]
#[repr(transparent)]
pub struct Timespec64(pub bindings::timespec64);

impl Timespec64 {
    /// Construct Timespec64 with `tv_sec` and `tv_nsec`
    pub fn new(tv_sec: KtimeT, tv_nsec: c_longlong) -> Self {
        Self(bindings::timespec64 { tv_sec, tv_nsec })
    }
}

/// A wrapper for [`bindings::itimerspec64`]
#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct Itimerspec64 {
    /// timer period
    pub it_interval: Timespec64,

    /// timer expiration
    pub it_value: Timespec64,
}

/// The function `timespec64_to_ktime` will call kernel's `ktime_set` to get a `KtimeT` from `Timespec64`.
pub fn timespec64_to_ktime(u_ts: Timespec64) -> KtimeT {
    unsafe { rust_helper_timespec64_to_ktime(u_ts.0) }
}

/// The function `KtimeT` will call kernel's `ktime_to_timespec64` macro definition to get a `Timespec64` from `KtimeT`.
pub fn ktime_to_timespec64(kt: KtimeT) -> Timespec64 {
    unsafe { Timespec64(rust_helper_ktime_to_timespec64(kt)) }
}

/// The `ktime_get` function returns the current ktime value. It calls the `bindings::ktime_get` function and casts the result to a `KtimeT`.
pub fn ktime_get() -> KtimeT {
    unsafe { bindings::ktime_get() as KtimeT }
}

/// The `ktime_sub` function subtracts one ktime value from another. It calls the `rust_helper_ktime_sub` function with the provided arguments.
pub fn ktime_sub(lhs: KtimeT, rhs: KtimeT) -> KtimeT {
    unsafe { rust_helper_ktime_sub(lhs, rhs) }
}

/// The `ktime_add_ns` function adds a nanosecond value to a ktime value. It calls the `rust_helper_ktime_add_ns` function with the provided arguments.
pub fn ktime_add_ns(kt: KtimeT, nsval: u64) -> KtimeT {
    unsafe { rust_helper_ktime_add_ns(kt, nsval) }
}

/// The `ktime_add` function adds two ktime values. It calls the `rust_helper_ktime_add` function with the provided arguments.
pub fn ktime_add(kt: KtimeT, nsval: KtimeT) -> KtimeT {
    unsafe { rust_helper_ktime_add(kt, nsval) }
}

/// The `ktime_to_ns` function converts a ktime value to a nanosecond value. It casts the `KtimeT` to an `i64`.
pub fn ktime_to_ns(kt: KtimeT) -> i64 {
    kt as i64
}

/// The `ktime_set` function sets a ktime value to a specific number of seconds and nanoseconds. It calls the `rust_helper_ktime_set` function with the provided arguments.
pub fn ktime_set(secs: i64, nsecs: usize) -> KtimeT {
    unsafe { rust_helper_ktime_set(secs, nsecs) }
}

/// The `ktime_divns` function divides a ktime value by a number of nanoseconds. It calls the `rust_helper_ktime_divns` function with the provided arguments.
pub fn ktime_divns(kt: KtimeT, div: i64) -> i64 {
    unsafe { rust_helper_ktime_divns(kt, div) }
}

/// The `ktime_compare` function compares two ktime values. It takes two `KtimeT` values and returns a `KtimeT` that indicates the result of the comparison. If the first value is greater than the second, it returns a positive number. If the first value is less than the second, it returns a negative number. If the two values are equal, it returns zero.
pub fn ktime_compare(cmp1: KtimeT, cmp2: KtimeT) -> KtimeT {
    unsafe { rust_helper_ktime_compare(cmp1, cmp2) }
}
