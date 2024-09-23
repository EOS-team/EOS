// SPDX-License-Identifier: GPL-2.0

//! bitmap
//!
//! C header: [`include/linux/bitmap.h`](../../../../include/linux/bitmap.h)

use crate::{bindings, c_types};

extern "C" {
    /// The `rust_helper_test_and_set_bit` function is an unsafe function that sets a bit in a bitmap. It takes a bit index and a pointer to the bitmap. It returns a non-zero value if the bit was already set, and zero if it was not.
    fn rust_helper_test_and_set_bit(
        bit: c_types::c_uint,
        p: *mut c_types::c_ulong,
    ) -> c_types::c_int;
}

/// The `bitmap_zalloc` function is a wrapper around the `bindings::bitmap_zalloc` function from the kernel bindings. It allocates a bitmap of a given size and initializes it to zero. It takes the number of bits in the bitmap and a set of flags that control the allocation. It returns a pointer to the allocated bitmap.
pub fn bitmap_zalloc(nbits: c_types::c_uint, flags: bindings::gfp_t) -> *mut c_types::c_ulong {
    unsafe { bindings::bitmap_zalloc(nbits, flags) }
}

/// Convert list format ASCII string to bitmap.
pub fn bitmap_parselist(
    buf: *const c_types::c_char,
    maskp: *mut c_types::c_ulong,
    nmaskbits: c_types::c_int,
) -> c_types::c_int {
    unsafe { bindings::bitmap_parselist(buf, maskp, nmaskbits) }
}

/// Free bitmap.
pub fn bitmap_free(bitmap: *const c_types::c_ulong) {
    unsafe {
        bindings::bitmap_free(bitmap);
    }
}

/// The `find_first_zero_bit` function is a wrapper around the `bindings::_find_first_zero_bit` function from the kernel bindings. It finds the first zero bit in a bitmap. It takes a pointer to the bitmap and the size of the bitmap in bits. It returns the index of the first zero bit.
pub fn find_first_zero_bit(
    addr: *const c_types::c_ulong,
    size: c_types::c_ulong,
) -> c_types::c_ulong {
    unsafe { bindings::_find_first_zero_bit(addr, size) }
}

/// The `test_and_set_bit` function is a wrapper around the `rust_helper_test_and_set_bit` function. It sets a bit in a bitmap and returns whether the bit was already set. It takes a bit index and a pointer to the bitmap. It returns true if the bit was already set, and false if it was not.
pub fn test_and_set_bit(bit: c_types::c_ulong, p: *mut c_types::c_ulong) -> bool {
    let res;
    unsafe {
        res = rust_helper_test_and_set_bit(bit as c_types::c_uint, p);
    }
    res == 1
}
