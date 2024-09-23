// SPDX-License-Identifier: GPL-2.0

//! vmalloc
//!
//! C header: [`include/linux/vmalloc.h`](../../../../include/linux/vmalloc.h)

use crate::{
    bindings::{self, gfp_t, GFP_KERNEL},
    c_types,
};

extern "C" {
    fn rust_helper_kzalloc(size: usize, flags: gfp_t) -> *mut c_types::c_void;
}

/// Function `c_vmalloc` allocates a block of memory.
/// It takes a size as a parameter and returns an optional pointer to the allocated memory.
/// If the allocation fails, it returns `None`.
pub fn c_vmalloc(size: c_types::c_ulong) -> Option<*mut c_types::c_void> {
    let ptr = unsafe { bindings::vmalloc(size) };

    if ptr.is_null() {
        return None;
    }

    Some(ptr)
}

/// Function `c_vfree` deallocates a block of memory.
/// It takes a pointer to the memory as a parameter.
pub fn c_vfree(ptr: *const c_types::c_void) {
    unsafe {
        bindings::vfree(ptr);
    }
}

/// Function `c_kzalloc` allocates a block of zero-initialized memory.
/// It takes a size as a parameter and returns an optional pointer to the allocated memory.
/// If the allocation fails, it returns `None`.
pub fn c_kzalloc(size: c_types::c_ulong) -> Option<*mut c_types::c_void> {
    let ptr = unsafe { rust_helper_kzalloc(size as usize, GFP_KERNEL) };

    if ptr.is_null() {
        return None;
    }

    Some(ptr)
}

/// Function `c_kzfree` deallocates a block of memory and sets it to zero.
/// It takes a pointer to the memory as a parameter.
pub fn c_kzfree(ptr: *const c_types::c_void) {
    unsafe {
        bindings::kfree(ptr);
    }
}
