// SPDX-License-Identifier: GPL-2.0

//! fs
//!
//! C header: [`include/linux/fs.h`](../../../../include/linux/fs.h)

use crate::{
    bindings, c_types,
    error::{Error, Result},
    str::CStr,
};

/// The `Filename` struct wraps a pointer to a `bindings::filename` from the kernel bindings.
pub struct Filename(*mut bindings::filename);

impl Filename {
    /// `getname_kernel`: A method that takes a reference to a `CStr` and returns a `Result` containing a new `Filename`.
    /// It calls the `bindings::getname_kernel` function with the `CStr` as argument.
    /// If the function returns a null pointer, it returns `Err(Error::EINVAL)`. Otherwise, it returns `Ok(Filename(res))`.
    pub fn getname_kernel(arg1: &'static CStr) -> Result<Self> {
        let res;
        unsafe {
            res = bindings::getname_kernel(arg1.as_char_ptr());
        }
        if res == core::ptr::null_mut() {
            return Err(Error::EINVAL);
        }
        Ok(Self(res))
    }

    /// `get_name`: A method that takes a reference to a `CStr` and returns a `Result` containing a new `Filename`.
    /// It calls the `bindings::getname` function with the `CStr` as argument.
    /// If the function returns a null pointer, it returns `Err(Error::EINVAL)`. Otherwise, It returns `Ok(Filename(res))`.
    pub fn getname(arg1: &'static CStr) -> Result<Self> {
        let res;
        unsafe {
            res = bindings::getname(arg1.as_char_ptr());
        }
        if res == core::ptr::null_mut() {
            return Err(Error::EINVAL);
        }
        Ok(Self(res))
    }

    /// `get_name`: A method that returns a pointer to a `c_char`. It dereferences the `Filename`'s pointer and returns the `name` field.
    pub fn get_name(&self) -> *const c_types::c_char {
        unsafe { (*self.0).name }
    }

    /// `from_ptr`: A method that takes a pointer to a `bindings::filename` and returns a new `Filename` containing the pointer.
    pub fn from_ptr(ptr: *mut bindings::filename) -> Self {
        Self(ptr)
    }
}

impl Drop for Filename {
    fn drop(&mut self) {
        unsafe { bindings::putname(self.0) };
    }
}

/// The `hashlen_string` function is a wrapper around the `bindings::hashlen_string` function from the kernel bindings. It takes a pointer to a `c_char` and a pointer to a `Filename` and returns a `u64`. It gets the name of the `Filename` and calls the `bindings::hashlen_string` function with the `c_char` and the name as arguments.
pub fn hashlen_string(salt: *const c_types::c_char, filename: *mut Filename) -> u64 {
    unsafe {
        let name = (*filename).get_name();
        bindings::hashlen_string(salt as *const c_types::c_void, name)
    }
}

/// The `kernel_write` function is a wrapper around the `bindings::kernel_write` function from the kernel bindings.
/// It takes `arg1` and `arg2` as a destination and a source pointer respectively while `arg3` is the data length.
/// It takes `arg4` as the offset length of the destination pointer.
pub fn kernel_write(
    arg1: *mut bindings::file,
    arg2: *const c_types::c_void,
    arg3: usize,
    arg4: *mut bindings::loff_t,
) -> isize {
    unsafe { bindings::kernel_write(arg1, arg2, arg3, arg4) }
}

/// The `kernel_read` function is a wrapper around the `bindings::kernel_read` function from the kernel bindings.
/// It takes `arg1` as a source file and takes `arg2` as a buffer to save the read result.
/// It takes `arg3` as a length to read. It takes `arg4` as a offset of the source pointer.
pub fn kernel_read(
    arg1: *mut bindings::file,
    arg2: *mut c_types::c_void,
    arg3: usize,
    arg4: *mut i64,
) -> isize {
    unsafe { bindings::kernel_read(arg1, arg2, arg3, arg4) }
}

/// The `memcpy` function is a wrapper around the `bindings::memcpy` function from the kernel bindings.
/// It takes `arg1` as a destination pointer while `arg2` is the source pointer.
/// The `arg3` is the length of memory.
pub fn memcpy(
    arg1: *mut c_types::c_void,
    arg2: *const c_types::c_void,
    arg3: c_types::c_ulong,
) -> *mut c_types::c_void {
    unsafe { bindings::memcpy(arg1, arg2, arg3) }
}
