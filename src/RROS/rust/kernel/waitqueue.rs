// SPDX-License-Identifier: GPL-2.0

//! waitqueue
//!
//! C header: [`include/linux/wait.h`](../../../../include/linux/wait.h)

use crate::{bindings, c_types::*, types::Opaque};
use core::marker::PhantomPinned;

extern "C" {
    fn rust_helper_add_wait_queue(
        wq_head: *mut bindings::wait_queue_head,
        wq_entry: *mut bindings::wait_queue_entry,
    );
    fn rust_helper_wait_event_interruptible(
        wq_head: *mut bindings::wait_queue_head,
        condition: bool,
    ) -> i32;
    fn rust_helper_init_waitqueue_head(wq: *mut bindings::wait_queue_head);
    #[allow(improper_ctypes)]
    fn __init_waitqueue_head(
        wq_head: *mut bindings::wait_queue_head,
        name: *const c_char,
        arg1: *mut bindings::lock_class_key,
    );
    fn rust_helper_spin_lock_irqsave(lock: *mut bindings::spinlock_t) -> u64;
    fn rust_helper_spin_unlock_irqrestore(lock: *mut bindings::spinlock_t, flags: u64);
    fn rust_helper_wq_has_sleeper(wq_head: *mut bindings::wait_queue_head) -> bool;
    fn rust_helper_raw_spin_lock_irqsave(lock: *mut bindings::hard_spinlock_t) -> u64;
    fn rust_helper_raw_spin_unlock_irqrestore(lock: *mut bindings::hard_spinlock_t, flags: u64);
    fn rust_helper_waitqueue_active(wq: *mut bindings::wait_queue_head) -> bool;
    fn rust_helper_list_empty(list: *const bindings::list_head) -> bool;
    fn rust_helper_list_del(list: *mut bindings::list_head);
}

/// The `LockClassKey` struct wraps a `bindings::lock_class_key` struct from the kernel bindings.
#[derive(Default)]
pub struct LockClassKey {
    lock_class_key: bindings::lock_class_key,
}

/// The `WaitQueueEntry` struct wraps a `bindings::wait_queue_entry_t` struct from the kernel bindings.
#[repr(transparent)]
pub struct WaitQueueEntry {
    wait_queue_entry: Opaque<bindings::wait_queue_entry_t>,

    /// A WaitQueueEntry needs to be pinned because it contains a [`struct list_head`] that is
    /// self-referential, so it cannot be safely moved once it is initialised.
    _pin: PhantomPinned,
}

impl WaitQueueEntry {
    /// Construct a new default struct.
    /// Safety: The caller must ensure that the returned struct is initialised before use.
    pub unsafe fn new() -> Self {
        WaitQueueEntry {
            wait_queue_entry: Opaque::uninit(),
            _pin: PhantomPinned,
        }
    }

    /// A wrapper around the `bindings::init_wait_entry` function from the kernel bindings.
    pub fn init_wait_entry(&mut self, flags: c_int) {
        // Safety: `wait_queue_entry` points to valid memory.
        unsafe {
            bindings::init_wait_entry(self.wait_queue_entry.get(), flags);
        }
    }

    /// Call `list_empty` from the rust_helper.
    pub fn list_empty(&self) -> bool {
        // Safety: `wait_queue_entry` points to valid memory.
        unsafe {
            rust_helper_list_empty(
                &(*self.wait_queue_entry.get()).entry as *const bindings::list_head,
            )
        }
    }

    /// Call `list_del` from the rust_helper to delete the entry from the list.
    pub fn list_del(&mut self) {
        // Safety: `wait_queue_entry.entry` points to valid memory.
        unsafe {
            rust_helper_list_del(
                &mut (*self.wait_queue_entry.get()).entry as *mut bindings::list_head,
            );
        }
    }
}

/// The `WaitQueueHead` struct wraps a `bindings::wait_queue_head_t` function from the kernel bindings.
#[repr(transparent)]
pub struct WaitQueueHead {
    wait_queue_head: Opaque<bindings::wait_queue_head>,

    /// A WaitQueueHead needs to be pinned because it contains a [`struct list_head`] that is
    /// self-referential, so it cannot be safely moved once it is initialised.
    _pin: PhantomPinned,
}

impl WaitQueueHead {
    /// Construct a new default struct.
    //TODO: combine new and init or refactor it like [`sync::CondVar`]
    pub fn new() -> Self {
        WaitQueueHead {
            wait_queue_head: Opaque::uninit(),
            _pin: PhantomPinned,
        }
    }

    /// Call `init_waitqueue_head` macro from the rust_helper.
    pub fn init(&mut self) {
        // Safety: `wait_queue_head` points to valid memory.
        unsafe {
            rust_helper_init_waitqueue_head(self.wait_queue_head.get());
        }
    }

    /// Call `spin_lock_irqsave` from the rust_helper.
    pub fn spin_lock_irqsave(&mut self) -> u64 {
        // Safety: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe {
            rust_helper_spin_lock_irqsave(
                &mut (*self.wait_queue_head.get()).lock as *mut bindings::spinlock_t,
            )
        }
    }

    /// Call `spin_unlock_irqrestore` from the rust_helper.
    pub fn spin_unlock_irqrestore(&mut self, flags: u64) {
        // Safety: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe {
            rust_helper_spin_unlock_irqrestore(
                &mut (*self.wait_queue_head.get()).lock as *mut bindings::spinlock_t,
                flags,
            );
        }
    }

    /// Call `raw_spin_lock_irqsave` from the rust_helper.
    pub fn raw_spin_lock_irqsave(&mut self) -> u64 {
        // Safety: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe {
            rust_helper_raw_spin_lock_irqsave(
                &mut (*self.wait_queue_head.get()).lock as *mut _ as *mut bindings::hard_spinlock_t,
            )
        }
    }

    /// Call `raw_spin_unlock_irqstore` from the rust_helper.
    pub fn raw_spin_unlock_irqrestore(&mut self, flags: u64) {
        // Safety: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe {
            rust_helper_raw_spin_unlock_irqrestore(
                &mut (*self.wait_queue_head.get()).lock as *mut _ as *mut bindings::hard_spinlock_t,
                flags,
            );
        }
    }

    /// Call `add_wait_queue` from the rust_helper.
    pub fn add_wait_queue(&mut self, wq_entry: &mut WaitQueueEntry) {
        // Safety: The caller guarantees that self and `wq_entry` are initialised. So the pointers are valid.
        unsafe {
            rust_helper_add_wait_queue(self.wait_queue_head.get(), wq_entry.wait_queue_entry.get());
        }
    }

    /// Call `wait_event_interruptible` from the rust_helper.
    pub fn wait_event_interruptible(&mut self, condition: bool) -> i32 {
        // Safety: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe { rust_helper_wait_event_interruptible(self.wait_queue_head.get(), condition) }
    }

    /// Call `__init_waitqueue_head` function from the kernel.
    pub fn init_waitqueue_head(&mut self, name: *const c_char, arg1: &mut LockClassKey) {
        let ptr_arg1 = &mut arg1.lock_class_key as *mut bindings::lock_class_key;
        // Safety: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe {
            __init_waitqueue_head(self.wait_queue_head.get(), name, ptr_arg1);
        }
    }

    /// A wrapper around the `bindings::__wake_up` from the kernel bindings.
    pub fn wake_up(&mut self, mode: c_uint, nr: c_int, key: *mut c_void) {
        // Safety: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe {
            bindings::__wake_up(self.wait_queue_head.get(), mode, nr, key);
        }
    }

    /// Call `wq_has_sleeper` from the rust_helper.
    pub fn wq_has_sleeper(&mut self) -> bool {
        // Safety: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe { rust_helper_wq_has_sleeper(self.wait_queue_head.get()) }
    }

    /// Call `waitqueue_active` from the rust_helper.
    pub fn waitqueue_active(&mut self) -> bool {
        // Safety: The caller guarantees that self is initialised. So the pointer is valid.
        unsafe { rust_helper_waitqueue_active(self.wait_queue_head.get()) }
    }
}
