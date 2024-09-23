// SPDX-License-Identifier: GPL-2.0

//! skbuff
//!
//! C header: [`include/linux/skbuff.h`](../../../../include/linux/skbuff.h)

use crate::{bindings, c_types};

/// The `skb_alloc_oob_head` function is a wrapper around the `bindings::skb_alloc_oob_head` function from the kernel bindings.
pub fn skb_alloc_oob_head(gfp_mask: bindings::gfp_t) -> *mut bindings::sk_buff {
    unsafe { bindings::skb_alloc_oob_head(gfp_mask) }
}

/// The `skb_put` function is a wrapper around the `bindings::skb_put` function from the kernel bindings.
pub fn skb_put(skb: *mut bindings::sk_buff, len: c_types::c_uint) -> *mut c_types::c_void {
    unsafe { bindings::skb_put(skb, len) }
}

/// The `skb_put` function is a wrapper around the `bindings::skb_push` function from the kernel bindings.
pub fn skb_push(skb: *mut bindings::sk_buff, len: c_types::c_uint) -> *mut c_types::c_void {
    unsafe { bindings::skb_push(skb, len) }
}

/// The `skb_pull` function is a wrapper around the `bindings::skb_pull` function from the kernel bindings.
pub fn skb_pull(skb: *mut bindings::sk_buff, len: c_types::c_uint) -> *mut c_types::c_void {
    unsafe { bindings::skb_pull(skb, len) }
}

/// The `__netdev_alloc_oob_skb` function is a wrapper around the `bindings::__netdev_alloc_oob_skb` function from the kernel bindings.
pub fn __netdev_alloc_oob_skb(
    dev: *mut bindings::net_device,
    len: usize,
    gfp_mask: u32,
) -> *mut bindings::sk_buff {
    unsafe { bindings::__netdev_alloc_oob_skb(dev, len, gfp_mask) }
}
