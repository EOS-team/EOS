// SPDX-License-Identifier: GPL-2.0

//! Per-CPU variables and functions.
use core::i32;

use crate::c_types;

extern "C" {
    fn rust_helper_per_cpu_ptr(
        var: *mut c_types::c_void,
        cpu: c_types::c_int,
    ) -> *mut c_types::c_void;

    fn rust_helper_raw_cpu_ptr(var: *mut c_types::c_void) -> *mut c_types::c_void;

    fn rust_helper_smp_processor_id() -> c_types::c_int;
}

// per_cpu prototype:
//#define per_cpu(var, cpu)	(*per_cpu_ptr(&(var), cpu))
// It is not possible to return a specific value per_cpu, so per_cpu_ptr can only be used to return a pointer.

/// Function `per_cpu_ptr` gets a per-CPU pointer.
/// It takes a variable and a CPU ID as parameters and returns a per-CPU pointer.
/// It calls `rust_helper_per_cpu_ptr` to get the per-CPU pointer.
pub fn per_cpu_ptr(var: *mut u8, cpu: i32) -> *mut u8 {
    unsafe {
        return rust_helper_per_cpu_ptr(var as *mut c_types::c_void, cpu as c_types::c_int)
            as *mut u8;
    }
}

// We can use generic to implement part of the ability of function per_cpu. But due to the absence of the
// macro define_percpu, this function has little chance to be used.

/// Function `per_cpu` gets a per-CPU pointer.
/// It takes a variable and a CPU ID as parameters and returns a per-CPU pointer.
/// It calls `per_cpu_ptr` to get the per-CPU pointer.
pub fn per_cpu<T>(var: *mut T, cpu: i32) -> *mut T {
    return per_cpu_ptr(var as *mut u8, cpu) as *mut T;
}

/// Function `raw_cpu_ptr` gets a raw CPU pointer.
/// It takes a variable as a parameter and returns a raw CPU pointer.
/// It calls `rust_helper_raw_cpu_ptr` to get the raw CPU pointer.
pub fn raw_cpu_ptr(var: *mut u8) -> *mut u8 {
    unsafe {
        return rust_helper_raw_cpu_ptr(var as *mut c_types::c_void) as *mut u8;
    }
}

/// Function `smp_processor_id` gets the current processor ID.
/// It calls `rust_helper_smp_processor_id` to get the current processor ID.
pub fn smp_processor_id() -> c_types::c_int {
    unsafe { rust_helper_smp_processor_id() }
}
