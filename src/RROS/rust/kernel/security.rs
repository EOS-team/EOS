// SPDX-License-Identifier: GPL-2.0

//! Linux Security Modules (LSM).
//!
//! C header: [`include/linux/security.h`](../../../../include/linux/security.h).

use crate::{bindings, c_types, error::Error, file::File, task::Task, Result};

extern "C" {
    #[allow(improper_ctypes)]
    fn rust_helper_security_binder_set_context_mgr(
        mgr: *mut bindings::task_struct,
    ) -> c_types::c_int;
    #[allow(improper_ctypes)]
    fn rust_helper_security_binder_transaction(
        from: *mut bindings::task_struct,
        to: *mut bindings::task_struct,
    ) -> c_types::c_int;
    #[allow(improper_ctypes)]
    fn rust_helper_security_binder_transfer_binder(
        from: *mut bindings::task_struct,
        to: *mut bindings::task_struct,
    ) -> c_types::c_int;
    #[allow(improper_ctypes)]
    fn rust_helper_security_binder_transfer_file(
        from: *mut bindings::task_struct,
        to: *mut bindings::task_struct,
        file: *mut bindings::file,
    ) -> c_types::c_int;
}

/// Calls the security modules to determine if the given task can become the manager of a binder
/// context.
pub fn binder_set_context_mgr(mgr: &Task) -> Result {
    // SAFETY: By the `Task` invariants, `mgr.ptr` is valid.
    let ret = unsafe { rust_helper_security_binder_set_context_mgr(mgr.ptr) };
    if ret != 0 {
        Err(Error::from_kernel_errno(ret))
    } else {
        Ok(())
    }
}

/// Calls the security modules to determine if binder transactions are allowed from task `from` to
/// task `to`.
pub fn binder_transaction(from: &Task, to: &Task) -> Result {
    // SAFETY: By the `Task` invariants, `from.ptr` and `to.ptr` are valid.
    let ret = unsafe { rust_helper_security_binder_transaction(from.ptr, to.ptr) };
    if ret != 0 {
        Err(Error::from_kernel_errno(ret))
    } else {
        Ok(())
    }
}

/// Calls the security modules to determine if task `from` is allowed to send binder objects
/// (owned by itself or other processes) to task `to` through a binder transaction.
pub fn binder_transfer_binder(from: &Task, to: &Task) -> Result {
    // SAFETY: By the `Task` invariants, `from.ptr` and `to.ptr` are valid.
    let ret = unsafe { rust_helper_security_binder_transfer_binder(from.ptr, to.ptr) };
    if ret != 0 {
        Err(Error::from_kernel_errno(ret))
    } else {
        Ok(())
    }
}

/// Calls the security modules to determine if task `from` is allowed to send the given file to
/// task `to` (which would get its own file descriptor) through a binder transaction.
pub fn binder_transfer_file(from: &Task, to: &Task, file: &File) -> Result {
    // SAFETY: By the `Task` invariants, `from.ptr` and `to.ptr` are valid. Similarly, by the
    // `File` invariants, `file.ptr` is also valid.
    let ret = unsafe { rust_helper_security_binder_transfer_file(from.ptr, to.ptr, file.ptr) };
    if ret != 0 {
        Err(Error::from_kernel_errno(ret))
    } else {
        Ok(())
    }
}
