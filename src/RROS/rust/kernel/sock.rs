// SPDX-License-Identifier: GPL-2.0

//! sock
//!
//! C header: [`include/linux/sock.h`](../../../../include/linux/sock.h)

use crate::bindings;
use core::cell::UnsafeCell;

/// The `Sock` struct wraps a `bindings::sock` struct from the kernel bindings.
#[repr(transparent)]
pub struct Sock(pub(crate) UnsafeCell<bindings::sock>);

impl Sock {
    /// Returns a mutable reference to inner struct.
    pub fn get_mut(&mut self) -> &mut bindings::sock {
        self.0.get_mut()
    }
}
