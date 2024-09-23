// SPDX-License-Identifier: GPL-2.0

//! The `kernel` crate.
//!
//! This crate contains the kernel APIs that have been ported or wrapped for
//! usage by Rust code in the kernel and is shared by all of them.
//!
//! In other words, all the rest of the Rust code in the kernel (e.g. kernel
//! modules written in Rust) depends on [`core`], [`alloc`] and this crate.
//!
//! If you need a kernel C API that is not ported or wrapped yet here, then
//! do so first instead of bypassing this crate.

#![no_std]
#![feature(
    allocator_api,
    alloc_error_handler,
    associated_type_defaults,
    const_fn_trait_bound,
    const_mut_refs,
    const_panic,
    const_raw_ptr_deref,
    const_unreachable_unchecked,
    const_fn_fn_ptr_basics,
    receiver_trait,
    try_reserve,
    unsafe_cell_raw_get
)]

// Ensure conditional compilation based on the kernel configuration works;
// otherwise we may silently break things like initcall handling.
#[cfg(not(CONFIG_RUST))]
compile_error!("Missing kernel configuration for conditional compilation");

#[cfg(not(test))]
#[cfg(not(testlib))]
mod allocator;

#[doc(hidden)]
pub mod bindings;

pub mod buffer;
pub mod c_types;
pub mod chrdev;
pub mod error;
pub mod file;
pub mod file_operations;
pub mod ioctl;
pub mod miscdev;
pub mod pages;
pub mod security;
pub mod str;
pub mod task;
pub mod traits;

pub mod linked_list;
mod raw_list;
pub mod rbtree;

#[doc(hidden)]
pub mod module_param;

mod build_assert;
pub mod prelude;
pub mod print;
pub mod random;
mod static_assert;
pub mod sync;

#[cfg(CONFIG_SYSCTL)]
pub mod sysctl;

pub mod io_buffer;
pub mod iov_iter;
pub mod of;
pub mod platdev;
pub mod types;
pub mod user_ptr;

pub mod bitmap;
pub mod capability;
pub mod class;
pub mod clockchips;
pub mod completion;
pub mod cpumask;
pub mod cred;
pub mod delay;
pub mod device;
pub mod double_linked_list;
pub mod double_linked_list2;
pub mod dovetail;
pub mod endian;
pub mod fs;
pub mod if_packet;
pub mod if_vlan;
pub mod interrupt;
pub mod irq_pipeline;
pub mod irq_work;
pub mod irqstage;
pub mod kernelh;
pub mod ktime;
pub mod memory_rros;
pub mod memory_rros_test;
pub mod mm;
#[cfg(CONFIG_NET)]
pub mod net;
pub mod netdevice;
pub mod notifier;
pub mod percpu;
pub mod percpu_defs;
pub mod premmpt;
pub mod ptrace;
pub mod sched;
pub mod skbuff;
pub mod sock;
pub mod socket;
pub mod sysfs;
pub mod tick;
pub mod timekeeping;
pub mod uapi;
pub mod uidgid;
pub mod vmalloc;
pub mod waitqueue;
pub mod workqueue;

#[doc(hidden)]
pub use build_error::build_error;

pub use crate::error::{Error, Result};
pub use crate::types::{ARef, AlwaysRefCounted, Mode, Opaque, ScopeGuard};

/// Page size defined in terms of the `PAGE_SHIFT` macro from C.
///
/// [`PAGE_SHIFT`]: ../../../include/asm-generic/page.h
pub const PAGE_SIZE: usize = 1 << bindings::PAGE_SHIFT;

/// Prefix to appear before log messages printed from within the kernel crate.
const __LOG_PREFIX: &[u8] = b"rust_kernel\0";

/// The top level entrypoint to implementing a kernel module.
///
/// For any teardown or cleanup operations, your type may implement [`Drop`].
pub trait KernelModule: Sized + Sync {
    /// Called at module initialization time.
    ///
    /// Use this method to perform whatever setup or registration your module
    /// should do.
    ///
    /// Equivalent to the `module_init` macro in the C API.
    fn init() -> Result<Self>;
}

/// Equivalent to `THIS_MODULE` in the C API.
///
/// C header: `include/linux/export.h`
#[repr(transparent)]
pub struct ThisModule(*mut bindings::module);

// SAFETY: `THIS_MODULE` may be used from all threads within a module.
unsafe impl Sync for ThisModule {}

impl ThisModule {
    /// Creates a [`ThisModule`] given the `THIS_MODULE` pointer.
    ///
    /// # Safety
    ///
    /// The pointer must be equal to the right `THIS_MODULE`.
    pub const unsafe fn from_ptr(ptr: *mut bindings::module) -> ThisModule {
        ThisModule(ptr)
    }

    /// Method `get_ptr` gets a pointer to the `module`.
    pub const fn get_ptr(&self) -> *mut bindings::module {
        self.0
    }
    /// Locks the module parameters to access them.
    ///
    /// Returns a [`KParamGuard`] that will release the lock when dropped.
    pub fn kernel_param_lock(&self) -> KParamGuard<'_> {
        // SAFETY: `kernel_param_lock` will check if the pointer is null and
        // use the built-in mutex in that case.
        #[cfg(CONFIG_SYSFS)]
        unsafe {
            bindings::kernel_param_lock(self.0)
        }

        KParamGuard { this_module: self }
    }
}

/// Scoped lock on the kernel parameters of [`ThisModule`].
///
/// Lock will be released when this struct is dropped.
pub struct KParamGuard<'a> {
    this_module: &'a ThisModule,
}

#[cfg(CONFIG_SYSFS)]
impl<'a> Drop for KParamGuard<'a> {
    fn drop(&mut self) {
        // SAFETY: `kernel_param_lock` will check if the pointer is null and
        // use the built-in mutex in that case. The existance of `self`
        // guarantees that the lock is held.
        unsafe { bindings::kernel_param_unlock(self.this_module.0) }
    }
}

/// Calculates the offset of a field from the beginning of the struct it belongs to.
///
/// # Examples
///
/// ```
/// # use kernel::prelude::*;
/// # use kernel::offset_of;
/// struct Test {
///     a: u64,
///     b: u32,
/// }
///
/// assert_eq!(offset_of!(Test, b), 8);
/// ```
#[macro_export]
macro_rules! offset_of {
    ($type:ty, $($f:tt)*) => {{
        let tmp = core::mem::MaybeUninit::<$type>::uninit();
        let outer = tmp.as_ptr();
        // To avoid warnings when nesting `unsafe` blocks.
        #[allow(unused_unsafe)]
        // SAFETY: The pointer is valid and aligned, just not initialised; `addr_of` ensures that
        // we don't actually read from `outer` (which would be UB) nor create an intermediate
        // reference.
        let inner = unsafe { core::ptr::addr_of!((*outer).$($f)*) } as *const u8;
        // To avoid warnings when nesting `unsafe` blocks.
        #[allow(unused_unsafe)]
        // SAFETY: The two pointers are within the same allocation block.
        unsafe { inner.offset_from(outer as *const u8) }
    }}
}

/// Produces a pointer to an object from a pointer to one of its fields.
///
/// # Safety
///
/// Callers must ensure that the pointer to the field is in fact a pointer to the specified field,
/// as opposed to a pointer to another object of the same type. If this condition is not met,
/// any dereference of the resulting pointer is UB.
///
/// # Examples
///
/// ```
/// # use kernel::container_of;
/// struct Test {
///     a: u64,
///     b: u32,
/// }
///
/// let test = Test { a: 10, b: 20 };
/// let b_ptr = &test.b;
/// let test_alias = container_of!(b_ptr, Test, b);
/// assert!(core::ptr::eq(&test, test_alias));
/// ```
#[macro_export]
macro_rules! container_of {
    ($ptr:expr, $type:ty, $($f:tt)*) => {{
        let ptr = $ptr as *const _ as *const u8;
        let offset = $crate::offset_of!($type, $($f)*);
        ptr.wrapping_offset(-offset) as *const $type
    }}
}
