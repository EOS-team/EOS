use crate::bindings;

/// Struct `KernelOldTimespec` represents an old kernel timespec.
/// It wraps the `__kernel_old_timespec` struct from the bindings module.
/// It includes a method `new` for creating a new `KernelOldTimespec`.
/// The `new` method initializes the `KernelOldTimespec` with the default `__kernel_old_timespec`.
pub struct KernelOldTimespec {
    /// Field `spec` is an instance of `__kernel_old_timespec`.
    pub spec: bindings::__kernel_old_timespec,
}

impl KernelOldTimespec {
    /// Method `new` creates a new `KernelOldTimespec`.
    /// It initializes the `KernelOldTimespec` with the default `__kernel_old_timespec`.
    pub fn new() -> Self {
        Self {
            spec: bindings::__kernel_old_timespec::default(),
        }
    }
}

/// Struct `KernelTimespec` represents a kernel timespec.
/// It wraps the `__kernel_timespec` struct from the bindings module.
/// It includes a method `new` for creating a new `KernelTimespec`.
/// The `new` method initializes the `KernelTimespec` with the default `__kernel_timespec`.
pub struct KernelTimespec {
    /// Field `spec` is an instance of `__kernel_timespec`.
    pub spec: bindings::__kernel_timespec,
}

impl KernelTimespec {
    /// Method `new` creates a new `KernelTimespec`.
    /// It initializes the `KernelTimespec` with the default `__kernel_timespec`.
    pub fn new() -> Self {
        Self {
            spec: bindings::__kernel_timespec::default(),
        }
    }

    // fn set_tv_sec(tv_sec: bindings::__kernel_old_time_t) {
    //     self.spec.tv_sec = tv_sec;
    // }
}
