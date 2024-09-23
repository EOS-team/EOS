// SPDX-License-Identifier: GPL-2.0

//! class
//!
//! C header: [`include/linux/device/class.h`](../../../../include/linux/device/class.h)

use core::u32;

use crate::{bindings, c_types, device, error::Error, Result};

extern "C" {
    #[allow(improper_ctypes)]
    fn rust_helper_class_create(
        this_module: &'static crate::ThisModule,
        buf: *const c_types::c_char,
    ) -> *mut bindings::class;
    #[allow(dead_code)]
    #[allow(improper_ctypes)]
    fn rust_helper_dev_name(dev: *const bindings::device) -> *const c_types::c_char;
}

/// The `DevT` struct is a wrapper around the `bindings::dev_t` struct from the kernel bindings. It represents a device type.
pub struct DevT(bindings::dev_t);

impl DevT {
    /// The `new` method is a constructor for `DevT`. It takes a number and creates a new `DevT` with that number.
    pub fn new(number: u32) -> Self {
        let a: bindings::dev_t = number;
        Self(a)
    }
}

/// The `Class` struct represents a device class in the kernel. It contains a raw pointer to the underlying `bindings::class` struct.
///
/// # Invariants
///
/// - [`self.0`] is valid and non-null.
pub struct Class(*mut bindings::class);

impl Class {
    /// The `new` method is a constructor for `Class`. It takes a reference to the current module and a name, and creates a new device class. If the creation fails, it returns an `EBADF` error.
    pub fn new(
        this_module: &'static crate::ThisModule,
        name: *const c_types::c_char,
    ) -> Result<Self> {
        let ptr = class_create(this_module, name);
        if ptr.is_null() {
            return Err(Error::EBADF);
        }
        Ok(Self(ptr))
    }

    /// Add the devnode call back function to the class.
    pub fn set_devnode<T: device::ClassDevnode>(&mut self) {
        // SAFETY: The `self.ptr` is a valid and created by `class_create` function.
        unsafe {
            (*(self.0)).devnode = device::ClassDevnodeVtable::<T>::get_class_devnode_callback();
        }
    }

    /// The `get_ptr` method returns the raw pointer to the underlying `bindings::class` struct.
    pub fn get_ptr(&self) -> *mut bindings::class {
        self.0
    }
}

/// The `class_create` function is a helper function that creates a new device class. It takes a reference to the current module and a name, and returns a raw pointer to the created class./// The `DevT` struct is a wrapper around the `bindings::dev_t` struct from the kernel bindings. It represents a device type.
fn class_create(
    this_module: &'static crate::ThisModule,
    name: *const c_types::c_char,
) -> *mut bindings::class {
    unsafe { rust_helper_class_create(this_module, name) }
}
