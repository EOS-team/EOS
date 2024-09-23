// SPDX-License-Identifier: GPL-2.0

//! socket
//!
//! C header: [`include/linux/socket.h`](../../../../include/linux/socket.h)

use crate::bindings;
use core::cell::UnsafeCell;

/// The `Sockaddr` struct wraps a `bindings::sockaddr` struct from the kernel bindings.
#[repr(transparent)]
pub struct Sockaddr(pub(crate) UnsafeCell<bindings::sockaddr>);
