// SPDX-License-Identifier: GPL-2.0

//! Credentials management.
//!
//! C header: [`include/linux/cred.h`](../../../../include/linux/cred.h)
//!
//! Reference: <https://www.kernel.org/doc/html/latest/security/credentials.html>

use crate::{bindings, c_types::*};
use core::cell::UnsafeCell;

extern "C" {
    fn rust_helper_cap_raise(c: *mut bindings::kernel_cap_t, flag: i32);
}

/// Wraps the kernel's `struct cred`.
///
/// # Invariants
///
/// Instances of this type are always ref-counted, that is, a call to `get_cred` ensures that the
/// allocation remains valid at least until the matching call to `put_cred`.
#[repr(transparent)]
pub struct Credential(pub UnsafeCell<bindings::cred>);

impl Credential {
    /// Prepare a new set of credentials for modification.
    pub fn prepare_creds() -> *mut Self {
        unsafe { bindings::prepare_creds() as *mut Credential }
    }

    /// Install new credentials upon the current task.
    pub fn commit_creds(new_cap: *mut Credential) -> c_int {
        unsafe { bindings::commit_creds(new_cap as *mut bindings::cred) }
    }

    /// Check the permissions of current task.
    pub fn cap_raise(&mut self, flag: i32) {
        unsafe {
            rust_helper_cap_raise(
                &mut (*self.0.get()).cap_effective as *mut bindings::kernel_cap_t,
                flag,
            );
        }
    }
}
