// SPDX-License-Identifier: GPL-2.0

//! Tasks (threads and processes).
//!
//! C header: [`include/linux/sched.h`](../../../../include/linux/sched.h).

use crate::{bindings, c_types};
use core::{marker::PhantomData, mem::ManuallyDrop, ops::Deref};

extern "C" {
    #[allow(improper_ctypes)]
    fn rust_helper_signal_pending(t: *const bindings::task_struct) -> c_types::c_int;
    #[allow(improper_ctypes)]
    fn rust_helper_get_current() -> *mut bindings::task_struct;
    #[allow(improper_ctypes)]
    fn rust_helper_get_task_struct(t: *mut bindings::task_struct);
    #[allow(improper_ctypes)]
    fn rust_helper_put_task_struct(t: *mut bindings::task_struct);
    #[allow(improper_ctypes)]
    fn rust_helper_task_cpu(t: *const bindings::task_struct) -> c_types::c_uint;
}

/// Wraps the kernel's `struct task_struct`.
///
/// # Invariants
///
/// The pointer `Task::ptr` is non-null and valid. Its reference count is also non-zero.
///
/// # Examples
///
/// The following is an example of getting the PID of the current thread with zero additional cost
/// when compared to the C version:
///
/// ```
/// # use kernel::prelude::*;
/// use kernel::task::Task;
///
/// # fn test() {
/// Task::current().pid();
/// # }
/// ```
///
/// Getting the PID of the current process, also zero additional cost:
///
/// ```
/// # use kernel::prelude::*;
/// use kernel::task::Task;
///
/// # fn test() {
/// Task::current().group_leader().pid();
/// # }
/// ```
///
/// Getting the current task and storing it in some struct. The reference count is automatically
/// incremented when creating `State` and decremented when it is dropped:
///
/// ```
/// # use kernel::prelude::*;
/// use kernel::task::Task;
///
/// struct State {
///     creator: Task,
///     index: u32,
/// }
///
/// impl State {
///     fn new() -> Self {
///         Self {
///             creator: Task::current().clone(),
///             index: 0,
///         }
///     }
/// }
/// ```
pub struct Task {
    pub(crate) ptr: *mut bindings::task_struct,
}

// SAFETY: Given that the task is referenced, it is OK to send it to another thread.
unsafe impl Send for Task {}

// SAFETY: It's OK to access `Task` through references from other threads because we're either
// accessing properties that don't change (e.g., `pid`, `group_leader`) or that are properly
// synchronised by C code (e.g., `signal_pending`).
unsafe impl Sync for Task {}

/// The type of process identifiers (PIDs).
type Pid = bindings::pid_t;

impl Task {
    /// Returns a task reference for the currently executing task/thread.
    pub fn current<'a>() -> TaskRef<'a> {
        // SAFETY: Just an FFI call.
        let ptr = unsafe { rust_helper_get_current() };

        // SAFETY: If the current thread is still running, the current task is valid. Given
        // that `TaskRef` is not `Send`, we know it cannot be transferred to another thread (where
        // it could potentially outlive the caller).
        unsafe { TaskRef::from_ptr(ptr) }
    }

    /// The `current_ptr` function returns a raw pointer to the current task. It is unsafe and should be used with caution.
    pub fn current_ptr() -> *mut bindings::task_struct {
        unsafe { rust_helper_get_current() }
    }

    /// The `task_cpu` function returns a CPU number of the provided `task_struct`.
    pub fn task_cpu(ptr: *const bindings::task_struct) -> c_types::c_uint {
        unsafe { rust_helper_task_cpu(ptr) }
    }

    /// Returns the group leader of the given task.
    pub fn group_leader(&self) -> TaskRef<'_> {
        // SAFETY: By the type invariant, we know that `self.ptr` is non-null and valid.
        let ptr = unsafe { (*self.ptr).group_leader };

        // SAFETY: The lifetime of the returned task reference is tied to the lifetime of `self`,
        // and given that a task has a reference to its group leader, we know it must be valid for
        // the lifetime of the returned task reference.
        unsafe { TaskRef::from_ptr(ptr) }
    }

    /// Returns the PID of the given task.
    pub fn pid(&self) -> Pid {
        // SAFETY: By the type invariant, we know that `self.ptr` is non-null and valid.
        unsafe { (*self.ptr).pid }
    }

    /// The `kernel` function checks if the given task is a kernel task. It returns true if the memory descriptor of the task is null, indicating that it's a kernel task.
    pub fn kernel(&self) -> bool {
        unsafe { (*self.ptr).mm == core::ptr::null_mut() }
    }

    /// The `state` function returns the state of the given task as a u32. It is unsafe because it directly accesses the `state` field of the task struct.
    pub fn state(&self) -> u32 {
        unsafe { (*self.ptr).state as u32 }
    }

    /// Returns the CPU of the given task as a u32.
    /// This function is unsafe because it directly accesses the `cpu` field of the task struct.
    pub fn cpu(&self) -> u32 {
        unsafe { (*self.ptr).cpu as u32 }
    }

    /// Determines whether the given task has pending signals.
    pub fn signal_pending(&self) -> bool {
        // SAFETY: By the type invariant, we know that `self.ptr` is non-null and valid.
        unsafe { rust_helper_signal_pending(self.ptr) != 0 }
    }

    /// Call `Linux` wake_up_process.
    pub fn wake_up_process(ptr: *mut bindings::task_struct) -> i32 {
        unsafe { bindings::wake_up_process(ptr) }
    }
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl Eq for Task {}

impl Clone for Task {
    fn clone(&self) -> Self {
        // SAFETY: The type invariants guarantee that `self.ptr` has a non-zero reference count.
        unsafe { rust_helper_get_task_struct(self.ptr) };

        // INVARIANT: We incremented the reference count to account for the new `Task` being
        // created.
        Self { ptr: self.ptr }
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        // INVARIANT: We may decrement the refcount to zero, but the `Task` is being dropped, so
        // this is not observable.
        // SAFETY: The type invariants guarantee that `Task::ptr` has a non-zero reference count.
        unsafe { rust_helper_put_task_struct(self.ptr) };
    }
}

/// A wrapper for [`Task`] that doesn't automatically decrement the refcount when dropped.
///
/// We need the wrapper because [`ManuallyDrop`] alone would allow callers to call
/// [`ManuallyDrop::into_inner`]. This would allow an unsafe sequence to be triggered without
/// `unsafe` blocks because it would trigger an unbalanced call to `put_task_struct`.
///
/// We make this explicitly not [`Send`] so that we can use it to represent the current thread
/// without having to increment/decrement its reference count.
///
/// # Invariants
///
/// The wrapped [`Task`] remains valid for the lifetime of the object.
pub struct TaskRef<'a> {
    task: ManuallyDrop<Task>,
    _not_send: PhantomData<(&'a (), *mut ())>,
}

impl TaskRef<'_> {
    /// Constructs a new `struct task_struct` wrapper that doesn't change its reference count.
    ///
    /// # Safety
    ///
    /// The pointer `ptr` must be non-null and valid for the lifetime of the object.
    pub(crate) unsafe fn from_ptr(ptr: *mut bindings::task_struct) -> Self {
        Self {
            task: ManuallyDrop::new(Task { ptr }),
            _not_send: PhantomData,
        }
    }
}

// SAFETY: It is OK to share a reference to the current thread with another thread because we know
// the owner cannot go away while the shared reference exists (and `Task` itself is `Sync`).
unsafe impl Sync for TaskRef<'_> {}

impl Deref for TaskRef<'_> {
    type Target = Task;

    fn deref(&self) -> &Self::Target {
        self.task.deref()
    }
}
