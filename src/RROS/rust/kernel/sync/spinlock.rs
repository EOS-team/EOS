// SPDX-License-Identifier: GPL-2.0

//! A kernel spinlock.
//!
//! This module allows Rust code to use the kernel's [`struct spinlock`].
//!
//! See <https://www.kernel.org/doc/Documentation/locking/spinlocks.txt>.

use super::{Guard, Lock, NeedsLockClass};
use crate::str::CStr;
use crate::{bindings, c_types, Opaque};
use core::{cell::UnsafeCell, marker::PhantomPinned, pin::Pin};

extern "C" {
    #[allow(improper_ctypes)]
    fn rust_helper_spin_lock_init(
        lock: *mut bindings::spinlock_t,
        name: *const c_types::c_char,
        key: *mut bindings::lock_class_key,
    );
    #[allow(dead_code)]
    fn rust_helper_spin_lock(lock: *mut bindings::spinlock);
    #[allow(dead_code)]
    fn rust_helper_spin_unlock(lock: *mut bindings::spinlock);
    fn rust_helper_hard_spin_lock(lock: *mut bindings::raw_spinlock);
    fn rust_helper_hard_spin_unlock(lock: *mut bindings::raw_spinlock);
    fn rust_helper_raw_spin_lock_irqsave(lock: *mut bindings::hard_spinlock_t) -> u64;
    fn rust_helper_raw_spin_unlock_irqrestore(lock: *mut bindings::hard_spinlock_t, flags: u64);
    fn rust_helper_raw_spin_lock_init(lock: *mut bindings::raw_spinlock_t);
    fn rust_helper_raw_spin_lock(lock: *mut bindings::hard_spinlock_t);
    fn rust_helper_raw_spin_unlock(lock: *mut bindings::hard_spinlock_t);
    fn rust_helper_raw_spin_lock_nested(lock: *mut bindings::hard_spinlock_t, depth: u32);
}

/// Safely initialises a [`SpinLock`] with the given name, generating a new lock class.
#[macro_export]
macro_rules! spinlock_init {
    ($spinlock:expr, $name:literal) => {
        $crate::init_with_lockdep!($spinlock, $name)
    };
}

/// Exposes the kernel's [`spinlock_t`]. When multiple CPUs attempt to lock the same spinlock, only
/// one at a time is allowed to progress, the others will block (spinning) until the spinlock is
/// unlocked, at which point another CPU will be allowed to make progress.
///
/// A [`SpinLock`] must first be initialised with a call to [`SpinLock::init`] before it can be
/// used. The [`spinlock_init`] macro is provided to automatically assign a new lock class to a
/// spinlock instance.
///
/// [`SpinLock`] does not manage the interrupt state, so it can be used in only two cases: (a) when
/// the caller knows that interrupts are disabled, or (b) when callers never use it in interrupt
/// handlers (in which case it is ok for interrupts to be enabled).
///
/// [`spinlock_t`]: ../../../include/linux/spinlock.h
pub struct SpinLock<T: ?Sized> {
    spin_lock: Opaque<bindings::spinlock>,

    /// Spinlocks are architecture-defined. So we conservatively require them to be pinned in case
    /// some architecture uses self-references now or in the future.
    _pin: PhantomPinned,

    data: UnsafeCell<T>,
}

// SAFETY: `SpinLock` can be transferred across thread boundaries iff the data it protects can.
unsafe impl<T: ?Sized + Send> Send for SpinLock<T> {}

// SAFETY: `SpinLock` serialises the interior mutability it provides, so it is `Sync` as long as the
// data it protects is `Send`.
unsafe impl<T: ?Sized + Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    /// Constructs a new spinlock.
    ///
    /// # Safety
    ///
    /// The caller must call [`SpinLock::init`] before using the spinlock.
    pub const unsafe fn new(t: T) -> Self {
        Self {
            spin_lock: Opaque::uninit(),
            data: UnsafeCell::new(t),
            _pin: PhantomPinned,
        }
    }
}

impl<T: ?Sized> SpinLock<T> {
    /// Locks the spinlock and gives the caller access to the data protected by it. Only one thread
    /// at a time is allowed to access the protected data.
    pub fn lock(&self) -> Guard<'_, Self> {
        self.lock_noguard();
        // SAFETY: The spinlock was just acquired.
        unsafe { Guard::new(self) }
    }

    /// The `irq_lock` method is similar to `lock`, but it also disables interrupts before acquiring the lock. This can be used to prevent race conditions between interrupt handlers and normal code.
    pub fn irq_lock(&self) -> Guard<'_, Self> {
        self.lock_noguard();

        // SAFETY: The spinlock was just acquired.
        unsafe { Guard::new(self) }
    }

    /// The `irq_lock_noguard` method acquires the lock and disables interrupts, but does not return a `Guard`. Instead, it returns a `u64` that represents the previous interrupt state. This method is unsafe because it does not provide any guarantees about the lifetime of the lock.
    // FIXME: use this to enable the smp function
    pub fn irq_lock_noguard(&self) -> u64 {
        // SAFETY: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe {
            rust_helper_raw_spin_lock_irqsave(self.spin_lock.get() as *mut bindings::hard_spinlock_t)
        }
    }

    /// The `irq_unlock_noguard` method releases the lock and restores the interrupt state to the value given by `flags`. This method is unsafe because it does not check whether the lock is currently held by the caller.
    // FIXME: use this to enable the smp function
    pub fn irq_unlock_noguard(&self, flags: u64) {
        // SAFETY: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe {
            rust_helper_raw_spin_unlock_irqrestore(
                self.spin_lock.get() as *mut bindings::hard_spinlock_t,
                flags,
            );
        }
    }

    /// The `raw_spin_lock` method acquires the lock.
    pub fn raw_spin_lock(&self) {
        // SAFETY: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe { rust_helper_raw_spin_lock(self.spin_lock.get() as *mut bindings::hard_spinlock_t) }
    }

    /// The `raw_spin_lock_nested` method acquires the lock nestly.
    pub fn raw_spin_lock_nested(&self, depth: u32) {
        // SAFETY: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe {
            rust_helper_raw_spin_lock_nested(
                self.spin_lock.get() as *mut bindings::hard_spinlock_t,
                depth,
            )
        }
    }

    /// The `raw_spin_unlock` method release the lock.
    pub fn raw_spin_unlock(&self) {
        // SAFETY: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe {
            rust_helper_raw_spin_unlock(self.spin_lock.get() as *mut bindings::hard_spinlock_t)
        }
    }
}

impl<T: ?Sized> NeedsLockClass for SpinLock<T> {
    unsafe fn init(self: Pin<&mut Self>, name: &'static CStr, key: *mut bindings::lock_class_key) {
        // SAFETY: The caller guarantees that `name` and `key` are initialised. So the pointers are valid.
        unsafe { rust_helper_spin_lock_init(self.spin_lock.get(), name.as_char_ptr(), key) };
    }
}

impl<T: ?Sized> Lock for SpinLock<T> {
    type Inner = T;

    fn lock_noguard(&self) {
        // SAFETY: `spin_lock` points to valid memory.
        // unsafe { rust_helper_spin_lock(self.spin_lock.get()) };
        unsafe { rust_helper_hard_spin_lock(self.spin_lock.get() as *mut bindings::raw_spinlock) };
        // unsafe { rust_helper_hard_spin_lock((*self.spin_lock.get()).rlock()
        // as *mut bindings::raw_spinlock) };
    }

    unsafe fn unlock(&self) {
        // SAFETY: `spin_lock` points to valid memory.
        // unsafe { rust_helper_spin_unlock(self.spin_lock.get()) };
        unsafe {
            rust_helper_hard_spin_unlock(self.spin_lock.get() as *mut bindings::raw_spinlock)
        };
        // unsafe { rust_helper_hard_spin_unlock((*self.spin_lock.get()).rlock()
        // as *mut bindings::raw_spinlock) };
    }

    fn locked_data(&self) -> &UnsafeCell<T> {
        // SAFETY: The caller guarantees that self is initialised.
        &self.data
    }
}

/// A wrapper for [`hard_spinlock_t`].
#[repr(transparent)]
pub struct HardSpinlock {
    lock: bindings::hard_spinlock_t,
}

impl HardSpinlock {
    /// Constructs a new struct.
    pub fn new() -> Self {
        HardSpinlock {
            lock: bindings::hard_spinlock_t {
                rlock: bindings::raw_spinlock {
                    raw_lock: bindings::arch_spinlock_t {
                        __bindgen_anon_1: bindings::qspinlock__bindgen_ty_1 {
                            val: bindings::atomic_t { counter: 0 },
                        },
                    },
                },
                dep_map: bindings::phony_lockdep_map {
                    wait_type_outer: 0,
                    wait_type_inner: 0,
                },
            },
        }
    }

    /// Initialize Self.
    pub fn init(&mut self) {
        self.lock = bindings::hard_spinlock_t::default();
        // SAFETY: `self.lock` points to valid memory.
        unsafe {
            rust_helper_raw_spin_lock_init(
                &mut self.lock as *mut bindings::hard_spinlock_t as *mut bindings::raw_spinlock_t,
            );
        }
    }

    /// Call `Linux` `raw_spin_lock_irqsave` to lock.
    pub fn raw_spin_lock_irqsave(&mut self) -> u64 {
        // SAFETY: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe {
            rust_helper_raw_spin_lock_irqsave(&mut self.lock as *mut bindings::hard_spinlock_t)
        }
    }

    /// Call `Linux` `raw_spin_unlock_irqrestore` to unlock.
    pub fn raw_spin_unlock_irqrestore(&mut self, flags: u64) {
        // SAFETY: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe {
            rust_helper_raw_spin_unlock_irqrestore(
                &mut self.lock as *mut bindings::hard_spinlock_t,
                flags,
            );
        }
    }

    /// Call `Linux` `raw_spin_lock` to lock.
    pub fn raw_spin_lock(&mut self) {
        // SAFETY: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe {
            rust_helper_raw_spin_lock(&mut self.lock as *mut bindings::hard_spinlock_t);
        }
    }

    /// Call `Linux` `raw_spin_unlock` to unlock.
    pub fn raw_spin_unlock(&mut self) {
        // SAFETY: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe {
            rust_helper_raw_spin_unlock(&mut self.lock as *mut bindings::hard_spinlock_t);
        }
    }

    /// Call `Linux` `raw_spin_lock_nested` to lock nestly.
    pub fn raw_spin_lock_nested(&mut self, depth: u32) {
        // SAFETY: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe {
            rust_helper_raw_spin_lock_nested(
                &mut self.lock as *mut bindings::hard_spinlock_t,
                depth,
            )
        }
    }
}

/// A wrapper for [`raw_spinlock_t`].
#[repr(transparent)]
pub struct RawSpinLock {
    #[allow(dead_code)]
    lock: bindings::raw_spinlock_t,
}
