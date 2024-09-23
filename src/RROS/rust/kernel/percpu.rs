// SPDX-License-Identifier: GPL-2.0

//! ktime_t - nanosecond-resolution time format.
//!
//! C header: [`include/linux/percpu.h`](../../../../include/linux/percpu.h)

use crate::{bindings, c_types};

/// Function `alloc_per_cpu` allocates per-CPU data.
/// It takes a size and an alignment as parameters and returns a pointer to the allocated data.
/// It calls the `__alloc_percpu` function from the bindings module to perform the allocation.
pub fn alloc_per_cpu(size: usize, align: usize) -> *mut u8 {
    unsafe {
        return bindings::__alloc_percpu(size, align) as *mut u8;
    }
}

/// Function `free_per_cpu` frees per-CPU data.
/// It takes a pointer to the data as a parameter.
/// It calls the `free_percpu` function from the bindings module to perform the freeing.
pub fn free_per_cpu(pdata: *mut u8) {
    unsafe {
        bindings::free_percpu(pdata as *mut c_types::c_void);
    }
}
