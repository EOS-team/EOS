// SPDX-License-Identifier: GPL-2.0

//! device
//!
//! C header: [`include/linux/device.h`](../../../../include/linux/device.h)

use crate::prelude::Box;

use crate::{
    bindings, c_types,
    str::CStr,
    uidgid::{KgidT, KuidT},
};

use core::marker::PhantomData;

extern "C" {
    #[allow(improper_ctypes)]
    fn rust_helper_dev_name(dev: *const bindings::device) -> *const c_types::c_char;
}

/// The `DeviceType` struct is a wrapper around the `bindings::device_type` struct from the kernel bindings. It represents a device type in the kernel.
#[repr(transparent)]
pub struct DeviceType(bindings::device_type);

impl DeviceType {
    /// create a raw new device_type
    pub const fn new() -> Self {
        Self(bindings::device_type {
            name: core::ptr::null(),
            groups: core::ptr::null_mut(),
            uevent: None,
            devnode: None,
            release: None,
            pm: core::ptr::null(),
        })
    }

    /// get the name of the device type
    pub fn get_name(&self) -> *const c_types::c_char {
        self.0.name
    }

    /// sets the name of the device type. It takes a pointer to a C-style string and sets the `name` field of the underlying `bindings::device_type` to that string.
    pub fn name(mut self, name: *const c_types::c_char) -> Self {
        (self.0).name = name;
        self
    }

    /// set the devnode call back function to the device type
    pub fn set_devnode<T: Devnode>(&mut self) {
        // SAFETY: T that implement Devnode will certainly return a valid static function pointer
        unsafe {
            self.0.devnode = DevnodeVtable::<T>::get_devnode_callback();
        }
    }

    /// returns the raw pointer to the underlying `bindings::device_type` struct.
    pub fn get_ptr(&self) -> *const bindings::device_type {
        &self.0
    }
}

/// The `Device` struct is a wrapper around the `bindings::device` struct from the kernel bindings. It represents a device in the kernel.
pub struct Device(*mut bindings::device);

impl Device {
    /// create a raw new device, only used in factory currently
    // FIXME: temporarily used
    pub unsafe fn raw_new<FUNC>(mut init: FUNC, name: &CStr) -> Self
    where
        FUNC: FnMut(&mut bindings::device),
    {
        let dev = Box::try_new(bindings::device::default()).unwrap();
        let dev = Box::leak(dev);
        init(dev);
        // SAFETY: the dev is valid, and the name will be copyed, so it don't have to be 'static
        unsafe { bindings::dev_set_name(dev, name.as_char_ptr()) };
        // SAFETY: the dev is valid
        unsafe { bindings::device_register(dev) };
        Self(dev as *mut bindings::device)
    }

    /// set the name of the device
    #[inline]
    pub fn dev_name(&self) -> *const c_types::c_char {
        unsafe { rust_helper_dev_name(self.0 as *mut bindings::device as *const bindings::device) }
    }

    /// get the driver data of the device
    #[inline]
    pub fn get_drvdata<T>(&mut self) -> Option<&T> {
        // SAFETY: must ensure self.0 is valid
        let ptr = unsafe { *self.0 }.driver_data as *const T;
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { &*ptr })
        }
    }

    /// set the driver data of the device
    #[inline]
    pub fn set_drvdata<T>(&mut self, data: *mut T) {
        // TODO: make sure data is pinned or belonged to dev
        // SAFETY: must ensure self.0 is valid
        unsafe {
            (*self.0).driver_data = data as *const T as *mut c_types::c_void;
        }
    }

    /// get the device type of the device
    #[inline]
    pub fn dev_type(&self) -> Option<&DeviceType> {
        // SAFETY: must ensure self.0 is valid
        unsafe {
            if (*self.0).type_.is_null() {
                None
            } else {
                Some(&*((*self.0).type_ as *const bindings::device_type as *const DeviceType))
            }
        }
    }
}

/// Class dev_node call back wrapper
pub trait ClassDevnode {
    /// dev_node call back function
    fn devnode(dev: &mut Device, mode: &mut u16) -> *mut c_types::c_char;
}

/// dev_node call back wrapper
pub trait Devnode {
    /// dev_node call back function
    fn devnode(
        dev: &mut Device,
        mode: &mut u16,
        uid: Option<&mut KuidT>,
        gid: Option<&mut KgidT>,
    ) -> *mut c_types::c_char;
}

pub(crate) struct ClassDevnodeVtable<T: ClassDevnode>(PhantomData<T>);

impl<T: ClassDevnode> ClassDevnodeVtable<T> {
    pub(crate) unsafe fn get_class_devnode_callback() -> Option<
        unsafe extern "C" fn(dev: *mut bindings::device, mode: *mut u16) -> *mut c_types::c_char,
    > {
        Some(class_devnode_callback::<T>)
    }
}
pub(crate) struct DevnodeVtable<T: Devnode>(PhantomData<T>);
impl<T: Devnode> DevnodeVtable<T> {
    pub(crate) unsafe fn get_devnode_callback() -> Option<
        unsafe extern "C" fn(
            dev: *mut bindings::device,
            mode: *mut u16,
            uid: *mut bindings::kuid_t,
            gid: *mut bindings::kgid_t,
        ) -> *mut c_types::c_char,
    > {
        Some(devnode_callback::<T>)
    }
}

unsafe extern "C" fn class_devnode_callback<T: ClassDevnode>(
    dev: *mut bindings::device,
    mode: *mut u16,
) -> *mut c_types::c_char {
    let dev = &mut Device(dev);
    let mode = unsafe { &mut *mode };
    T::devnode(dev, mode)
}

unsafe extern "C" fn devnode_callback<T: Devnode>(
    dev: *mut bindings::device,
    mode: *mut u16,
    uid: *mut bindings::kuid_t,
    gid: *mut bindings::kgid_t,
) -> *mut c_types::c_char {
    let dev = &mut Device(dev);
    let mode = unsafe { &mut *mode };
    let uid = if uid.is_null() {
        None
    } else {
        unsafe { Some(&mut *(uid as *mut KuidT)) }
    };
    let gid = if gid.is_null() {
        None
    } else {
        unsafe { Some(&mut *(gid as *mut KgidT)) }
    };
    T::devnode(dev, mode, uid, gid)
}
