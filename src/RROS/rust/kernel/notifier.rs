// SPDX-License-Identifier: GPL-2.0

//! notifier
//!
//! C header: [`include/linux/notifier.h`](../../../../include/linux/notifier.h)

use crate::bindings;

use core::cell::UnsafeCell;

/// The `NotifierBlock` struct wraps a `bindings::notifier_block` struct.
#[repr(transparent)]
pub struct NotifierBlock(UnsafeCell<bindings::notifier_block>);
