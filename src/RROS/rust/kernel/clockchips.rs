// SPDX-License-Identifier: GPL-2.0

//! clockchips
//!
//! C header: [`include/linux/clockchips.h`](../../../../include/linux/clockchips.h)
use crate::{bindings, c_types, container_of, prelude::*};

extern "C" {
    #[allow(dead_code)]
    #[allow(improper_ctypes)]
    fn rust_helper__this_cpu_write(
        pcp: *mut bindings::clock_proxy_device,
        val: *mut bindings::clock_proxy_device,
    );
    #[allow(dead_code)]
    #[allow(improper_ctypes)]
    fn rust_helper__this_cpu_read(
        pcp: *mut bindings::clock_proxy_device,
    ) -> *mut bindings::clock_proxy_device;
    #[allow(dead_code)]
    #[allow(improper_ctypes)]
    fn rust_helper_proxy_set_next_ktime(
        expires: KtimeT,
        dev: *mut bindings::clock_event_device,
    ) -> c_types::c_int;
    #[allow(dead_code)]
    #[allow(improper_ctypes)]
    fn rust_helper_proxy_set(
        expires: KtimeT,
        dev: *mut bindings::clock_event_device,
    ) -> c_types::c_int;
}

type KtimeT = i64;

// TODO Correct the format of the following doc comment.
/// The `ClockProxyDevice` struct is a wrapper around the `bindings::clock_proxy_device` struct from the kernel bindings. It represents a clock proxy device in the kernel.
#[repr(transparent)]
pub struct ClockProxyDevice {
    /// The `ptr` field in the `ClockProxyDevice` struct is a raw pointer to the underlying `bindings::clock_proxy_device` struct from the kernel bindings. It represents the actual clock proxy device in the kernel that this `ClockProxyDevice` struct is wrapping.
    pub ptr: *mut bindings::clock_proxy_device,
}

impl ClockProxyDevice {
    /// The `new` method is a constructor for `ClockProxyDevice`. It takes a raw pointer to a `bindings::clock_proxy_device` and returns a `ClockProxyDevice`. If the pointer is null, it prints a warning and returns an `EINVAL` error.
    pub fn new(cpd: *mut bindings::clock_proxy_device) -> Result<Self> {
        let ptr = cpd;
        if ptr.is_null() {
            pr_warn!("init clock_proxy_device error!");
            return Err(Error::EINVAL);
        }
        Ok(Self { ptr })
    }

    /// The `get_ptr` method returns a mutable pointer to the underlying `bindings::clock_proxy_device`. This can be used to pass the `ClockProxyDevice` to kernel functions that expect a `bindings::clock_proxy_device`.
    pub fn get_ptr(&self) -> *mut bindings::clock_proxy_device {
        return unsafe { &mut *self.ptr as *mut bindings::clock_proxy_device };
    }

    /// The `get_proxy_device` method returns a mutable pointer to the `bindings::clock_event_device` that this `ClockProxyDevice` is proxying.
    pub fn get_proxy_device(&self) -> *mut bindings::clock_event_device {
        unsafe { (&mut (*self.ptr).proxy_device) as *mut bindings::clock_event_device }
    }

    /// The `get_real_device` method returns a mutable pointer to the real `bindings::clock_event_device` that this `ClockProxyDevice` is proxying.
    pub fn get_real_device(&self) -> *mut bindings::clock_event_device {
        unsafe { ((*self.ptr).real_device) as *mut bindings::clock_event_device }
    }

    /// The `set_handle_oob_event` method sets the out-of-band event handler for the clock proxy device. It takes a function pointer that is called when an out-of-band event occurs.
    pub fn set_handle_oob_event(
        &self,
        func: unsafe extern "C" fn(dev: *mut bindings::clock_event_device),
    ) {
        unsafe { (*self.ptr).handle_oob_event = Some(func) };
    }
}

/// The `ClockEventDevice` struct is a wrapper around the `bindings::clock_event_device` struct from the kernel bindings. It represents a clock event device in the kernel.
pub struct ClockEventDevice {
    /// The `ptr` field in the `ClockEventDevice` struct is a raw pointer to the underlying `bindings::clock_event_device`. It represents the actual clock event device in the kernel that this `ClockEventDevice` struct is wrapping.
    pub ptr: *mut bindings::clock_event_device,
}
impl ClockEventDevice {
    /// The `from_proxy_device` method is a constructor for `ClockEventDevice`. It takes a raw pointer to a `bindings::clock_event_device` and returns a `ClockEventDevice`. If the pointer is null, it prints a warning and returns an `EINVAL` error.
    pub fn from_proxy_device(ced: *mut bindings::clock_event_device) -> Result<Self> {
        let ptr = ced;
        if ptr.is_null() {
            pr_warn!("get proxy_device error!");
            return Err(Error::EINVAL);
        }
        Ok(Self { ptr })
    }

    /// The `get_ptr` method returns a mutable pointer to the underlying `bindings::clock_event_device`. This can be used to pass the `ClockEventDevice` to kernel functions that expect a `bindings::clock_event_device`.
    pub fn get_ptr(&self) -> *mut bindings::clock_event_device {
        return unsafe { &mut *self.ptr as *mut bindings::clock_event_device };
    }

    /// The `get_features` method returns the features of the clock event device. It does this by dereferencing the `ptr` field and accessing the `features` field of the underlying `bindings::clock_event_device`.
    pub fn get_features(&self) -> c_types::c_uint {
        unsafe { (*self.ptr).features }
    }

    /// The `set_features` method sets the features of the clock event device. It takes a number and sets the `features` field of the underlying `bindings::clock_event_device` to that number.
    pub fn set_features(&self, num: c_types::c_uint) {
        unsafe { (*self.ptr).features = num };
    }

    /// The `set_set_next_ktime` method sets the `set_next_ktime` function pointer of the clock event device. It takes a function pointer and sets the `set_next_ktime` field of the underlying `bindings::clock_event_device` to that function.
    pub fn set_set_next_ktime(
        &self,
        func: unsafe extern "C" fn(
            expires: KtimeT,
            arg1: *mut bindings::clock_event_device,
        ) -> c_types::c_int,
    ) {
        unsafe { (*self.ptr).set_next_ktime = Some(func) };
    }

    /// The `set_next_ktime` method sets the next ktime for the clock event device. It takes a `KtimeT` and a pointer to a `bindings::clock_event_device`. If the `set_next_ktime` function pointer of the underlying `bindings::clock_event_device` is set, it calls that function with the provided arguments and returns the result. Otherwise, it returns 1.
    pub fn set_next_ktime(
        &self,
        evt: KtimeT,
        arg1: *mut bindings::clock_event_device,
    ) -> c_types::c_int {
        unsafe {
            if (*self.ptr).set_next_ktime.is_some() {
                return (*self.ptr).set_next_ktime.unwrap()(evt, arg1);
            }
            return 1;
        }
    }

    /// The `get_set_state_oneshot_stopped` method returns the `set_state_oneshot_stopped` function pointer of the clock event device. This function is used to set the state of the clock event device to "oneshot stopped".
    pub fn get_set_state_oneshot_stopped(
        &self,
    ) -> Option<unsafe extern "C" fn(arg1: *mut bindings::clock_event_device) -> c_types::c_int>
    {
        unsafe { (*self.ptr).set_state_oneshot_stopped }
    }

    /// The `set_set_state_oneshot_stopped` method sets the `set_state_oneshot_stopped` function pointer of the clock event device. It takes a function pointer and sets the `set_state_oneshot_stopped` field of the underlying `bindings::clock_event_device` to that function.
    pub fn set_set_state_oneshot_stopped(
        &self,
        func: unsafe extern "C" fn(arg1: *mut bindings::clock_event_device) -> c_types::c_int,
    ) {
        unsafe { (*self.ptr).set_state_oneshot_stopped = Some(func) };
    }

    /// The `get_max_delta_ns` method returns the maximum delta in nanoseconds for the clock event device. It does this by dereferencing the `ptr` field and accessing the `max_delta_ns` field of the underlying `bindings::clock_event_device`.
    pub fn get_max_delta_ns(&self) -> u64 {
        unsafe { (*self.ptr).max_delta_ns as u64 }
    }

    /// The `get_min_delta_ns` method returns the minimum delta in nanoseconds for the clock event device. It does this by dereferencing the `ptr` field and accessing the `min_delta_ns` field of the underlying `bindings::clock_event_device`.
    pub fn get_min_delta_ns(&self) -> u64 {
        unsafe { (*self.ptr).min_delta_ns as u64 }
    }

    /// The `get_mult` method returns the multiplier for the clock event device. It does this by dereferencing the `ptr` field and accessing the `mult` field of the underlying `bindings::clock_event_device`.
    pub fn get_mult(&self) -> u32 {
        unsafe { (*self.ptr).mult as u32 }
    }

    /// The `get_shift` method returns the shift value for the clock event device. It does this by dereferencing the `ptr` field and accessing the `shift` field of the underlying `bindings::clock_event_device`.
    pub fn get_shift(&self) -> u32 {
        unsafe { (*self.ptr).shift as u32 }
    }

    /// The `get_min_delta_ticks` method returns the minimum delta in ticks for the clock event device. It does this by dereferencing the `ptr` field and accessing the `min_delta_ticks` field of the underlying `bindings::clock_event_device`.
    pub fn get_min_delta_ticks(&self) -> u64 {
        unsafe { (*self.ptr).min_delta_ticks as u64 }
    }

    /// The `set_next_event` method sets the next event for the clock event device. It takes a `c_types::c_ulong` representing the event and a pointer to a `bindings::clock_event_device`. If the `set_next_event` function pointer of the underlying `bindings::clock_event_device` is set, it calls that function with the provided arguments and returns the result. Otherwise, it returns 1.
    pub fn set_next_event(
        &self,
        evt: c_types::c_ulong,
        arg1: *mut bindings::clock_event_device,
    ) -> c_types::c_int {
        unsafe {
            if (*self.ptr).set_next_event.is_some() {
                return (*self.ptr).set_next_event.unwrap()(evt, arg1);
            }
            return 1;
        }
    }
}

/// A trait to implement functions for ClockProxyDevice.
pub trait ProxySetOneshotStopped {
    /// A function need to implement in trait.
    fn proxy_set_oneshot_stopped(dev: ClockProxyDevice) -> c_types::c_int;
}

/// Callback function for trait `ProxySetOneshotStopped`.
pub unsafe extern "C" fn proxy_set_oneshot_stopped<T: ProxySetOneshotStopped>(
    proxy_dev: *mut bindings::clock_event_device,
) -> c_types::c_int {
    let dev = ClockProxyDevice::new(container_of!(
        proxy_dev,
        bindings::clock_proxy_device,
        proxy_device
    ) as *mut bindings::clock_proxy_device)
    .unwrap();
    T::proxy_set_oneshot_stopped(dev)
}

/// A trait to implement functions for `ClockEventDevice`.
pub trait ProxySetNextKtime {
    /// A function need to implement in trait.
    fn proxy_set_next_ktime(expires: KtimeT, arg1: ClockEventDevice) -> i32;
}

/// Callback function for trait `ProxySetNextKtime`.
pub unsafe extern "C" fn proxy_set_next_ktime<T: ProxySetNextKtime>(
    expires: KtimeT,
    arg1: *mut bindings::clock_event_device,
) -> i32 {
    let arg1 = ClockEventDevice::from_proxy_device(arg1).unwrap();
    T::proxy_set_next_ktime(expires, arg1)
}

/// A trait to implement functions for `ClockProxyDevice`.
pub trait SetupProxy {
    /// A function need to implement in trait.
    fn setup_proxy(dev: ClockProxyDevice);
}

/// Callback function for trait `SetupProxy`.
pub unsafe extern "C" fn setup_proxy<T: SetupProxy>(_dev: *mut bindings::clock_proxy_device) {
    match ClockProxyDevice::new(_dev) {
        Ok(dev) => T::setup_proxy(dev),
        Err(_e) => pr_warn!("dev new error!"),
    }
}

/// A trait to implement functions for `CoreTick`.
pub trait CoreTick {
    /// A function need to implement in trait.
    fn core_tick(dummy: ClockEventDevice);
}

/// Callback function for trait `CoreTick`.
pub unsafe extern "C" fn core_tick<T: CoreTick>(dummy: *mut bindings::clock_event_device) {
    T::core_tick(
        ClockEventDevice::from_proxy_device(dummy).unwrap_or(ClockEventDevice { ptr: 0 as *mut _ }),
    );
}

/// A trait to implement functions for `ClockIpiHandle`.
#[cfg(CONFIG_SMP)]
pub trait ClockIpiHandler {
    /// A function need to implement in trait.
    fn clock_ipi_handler(irq: c_types::c_int, dev_id: *mut c_types::c_void) -> c_types::c_uint;
}

/// Callback function for trait `ClockIpiHandler`.
#[cfg(CONFIG_SMP)]
pub unsafe extern "C" fn clock_ipi_handler<T: ClockIpiHandler>(
    irq: c_types::c_int,
    dev_id: *mut c_types::c_void,
) -> bindings::irqreturn_t {
    T::clock_ipi_handler(irq, dev_id)
}
