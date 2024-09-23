// SPDX-License-Identifier: GPL-2.0

//! Allocator support.

use crate::bindings;
use crate::c_types;
use crate::pr_debug;
use crate::timekeeping::*;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr;
pub struct KernelAllocator;

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // `krealloc()` is used instead of `kmalloc()` because the latter is
        // an inline function and cannot be bound to as a result.
        unsafe { bindings::krealloc(ptr::null(), layout.size(), bindings::GFP_KERNEL) as *mut u8 }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe {
            bindings::kfree(ptr as *const c_types::c_void);
        }
    }
}

#[global_allocator]
static ALLOCATOR: KernelAllocator = KernelAllocator;

// `rustc` only generates these for some crate types. Even then, we would need
// to extract the object file that has them from the archive. For the moment,
// let's generate them ourselves instead.
#[no_mangle]
pub fn __rust_alloc(size: usize, _align: usize) -> *mut u8 {
    pr_debug!(
        "__rust_alloc: time1 is {} size is {}",
        ktime_get_real_fast_ns(),
        size
    );
    let x = unsafe { bindings::krealloc(core::ptr::null(), size, bindings::GFP_KERNEL) as *mut u8 };
    pr_debug!("__rust_alloc: time2 is {}", ktime_get_real_fast_ns());
    return x;
}

#[no_mangle]
pub fn __rust_dealloc(ptr: *mut u8, _size: usize, _align: usize) {
    unsafe { bindings::kfree(ptr as *const c_types::c_void) };
}

#[no_mangle]
pub fn __rust_realloc(ptr: *mut u8, _old_size: usize, _align: usize, new_size: usize) -> *mut u8 {
    unsafe {
        bindings::krealloc(
            ptr as *const c_types::c_void,
            new_size,
            bindings::GFP_KERNEL,
        ) as *mut u8
    }
}

#[no_mangle]
pub fn __rust_alloc_zeroed(size: usize, _align: usize) -> *mut u8 {
    unsafe {
        bindings::krealloc(
            core::ptr::null(),
            size,
            bindings::GFP_KERNEL | bindings::__GFP_ZERO,
        ) as *mut u8
    }
}
