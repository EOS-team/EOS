// SPDX-License-Identifier: GPL-2.0

//! dovetail
//!
//! C header: [`include/linux/dovetail.h`](../../../../include/linux/dovetail.h)
use crate::{bindings, c_types::*, prelude::*};

use core::ptr;

extern "C" {
    #[allow(improper_ctypes)]
    fn rust_helper_dovetail_current_state() -> *mut bindings::oob_thread_state;
    fn rust_helper_dovetail_leave_oob();
    #[allow(improper_ctypes)]
    fn rust_helper_dovetail_request_ucall(task: *mut bindings::task_struct);
    fn rust_helper_dovetail_mm_state() -> *mut bindings::oob_mm_state;
    #[allow(improper_ctypes)]
    fn rust_helper_dovetail_send_mayday(task: *mut bindings::task_struct);
}

/// The `dovetail_start` function is a wrapper around the `bindings::dovetail_start` function from the kernel bindings. It starts the Dovetail interface in the kernel.
///
/// This function does not take any arguments. It calls the `bindings::dovetail_start` function and checks the return value. If the return value is 0, it returns `Ok(0)`. Otherwise, it returns `Err(Error::EINVAL)`.
///
/// This function is unsafe because it calls an unsafe function from the kernel bindings.
pub fn dovetail_start() -> Result<usize> {
    let res = unsafe { bindings::dovetail_start() };
    if res == 0 {
        return Ok(0);
    }
    Err(Error::EINVAL)
}

/// A trait to implement function for `RrosSubscriber`.
pub trait DovetailSubscriber {
    /// The type of the return value to `get` funtion.
    type Node;
    /// Get a mutable reference to the root node of the subscriber red-black tree.
    fn get(&self) -> &mut Self::Node;
}

/// A wrapper for [`oob_thread_state`].
pub struct OobThreadState {
    pub(crate) ptr: *mut bindings::oob_thread_state,
}

impl OobThreadState {
    pub(crate) unsafe fn from_ptr(ptr: *mut bindings::oob_thread_state) -> Self {
        Self { ptr }
    }

    /// Returns the preemp_count value.
    pub fn preempt_count(&self) -> i32 {
        unsafe { (*(self.ptr)).preempt_count }
    }

    /// `thread`: A method that returns a pointer to a `c_void`. It dereferences the `OobThreadState`'s pointer and returns the `thread` field.
    pub fn thread(&self) -> *mut c_void {
        unsafe { (*(self.ptr)).thread }
    }

    /// `set_thread`: A method that set the `thread` field to the `curr` parameter.
    pub fn set_thread(&self, curr: *mut c_void) {
        unsafe {
            (*(self.ptr)).thread = curr;
        }
    }

    /// `subscriber`: A method that returns a pointer to a `T` which impl `DovetailSubscriber` trait. It dereferences the `OobThreadState`'s pointer and returns the `subscriber` field.
    pub fn subscriber<T: DovetailSubscriber>(&self) -> *mut T {
        // FIXME: need to refactor in the future.
        unsafe { (*(self.ptr)).subscriber as *mut T }
    }

    /// `set_subscriber`: A method that set the `subscriber` field to the `sbr` parameter.
    pub fn set_subscriber<T: DovetailSubscriber>(&self, sbr: *mut T) {
        // FIXME: need a SpinLock?
        unsafe {
            (*(self.ptr)).subscriber = sbr as *mut c_void;
        }
    }
}

/// Constructs a new struct from current's state.
pub fn dovetail_current_state() -> OobThreadState {
    let ptr = unsafe { rust_helper_dovetail_current_state() };
    unsafe { OobThreadState::from_ptr(ptr) }
}

/// A wrapper for [`dovetail_altsched_context`].
#[repr(transparent)]
pub struct DovetailAltschedContext(pub bindings::dovetail_altsched_context);

impl DovetailAltschedContext {
    /// Constructs a new default struct.
    pub fn new() -> Self {
        Self(bindings::dovetail_altsched_context::default())
    }

    /// Initialize struct.
    pub fn dovetail_init_altsched(&mut self) {
        unsafe {
            bindings::dovetail_init_altsched(
                &mut self.0 as *mut bindings::dovetail_altsched_context,
            );
        }
    }
}

/// Switch context.
pub fn dovetail_context_switch(
    out: &mut DovetailAltschedContext,
    in_: &mut DovetailAltschedContext,
    leave_inband: bool,
) -> bool {
    let ptr_out = &mut out.0 as *mut bindings::dovetail_altsched_context;
    let ptr_in_ = &mut in_.0 as *mut bindings::dovetail_altsched_context;
    unsafe { bindings::dovetail_context_switch(ptr_out, ptr_in_, leave_inband) }
}

/// Start a inband thread.
pub fn dovetail_resume_inband() {
    unsafe {
        bindings::dovetail_resume_inband();
    }
}

/// This call tells the kernel that the current task may request alternate
/// scheduling operations any time from now on, such as switching out-of-band
/// or back in-band. It also activates the event notifier for the task, which
/// allows it to emit out-of-band system calls to the core.
pub fn dovetail_start_altsched() {
    unsafe {
        bindings::dovetail_start_altsched();
    }
}

/// Perform the out-of-band switch for the current task.
pub fn dovetail_leave_inband() -> c_int {
    unsafe { bindings::dovetail_leave_inband() }
}

/// This call disables the event notifier for the current task,
/// which must be done before dismantling the alternate scheduling
/// support for that task in the autonomous core.
pub fn dovetail_stop_altsched() {
    unsafe {
        bindings::dovetail_stop_altsched();
    }
}

/// Leave out-of-bound context.
pub fn dovetail_leave_oob() {
    unsafe {
        rust_helper_dovetail_leave_oob();
    }
}

/// Pend a request for target to fire the INBAND_TASK_RETUSER event
/// at the first opportunity, which happens when the task is about
/// to resume execution in user mode from the in-band stage.
pub fn dovetail_request_ucall(ptr: *mut bindings::task_struct) {
    unsafe {
        rust_helper_dovetail_request_ucall(ptr);
    }
}

/// send mayday signals to userland thread.
pub fn dovetail_send_mayday(ptr: *mut bindings::task_struct) {
    unsafe {
        rust_helper_dovetail_send_mayday(ptr);
    }
}

/// A wrapper for [`oob_mm_state`].
#[derive(Copy, Clone)]
pub struct OobMmState {
    /// A pointer to `bindings::oob_mm_state`.
    pub ptr: *mut bindings::oob_mm_state,
}

impl OobMmState {
    /// Constructs a new struct.
    pub fn new() -> Self {
        Self {
            ptr: ptr::null_mut(),
        }
    }

    /// Judge whether a null-pointer.
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }
}

/// This call retrieves the address of the out-of-band data structure
/// within the mm descriptor of the current user-space task. The content
/// of this structure is zeroed by the in-band kernel when it creates
/// the memory context, and stays so until your autonomous core initializes
/// it. If called from a kernel thread, NULL is returned instead.
pub fn dovetail_mm_state() -> OobMmState {
    unsafe {
        OobMmState {
            ptr: rust_helper_dovetail_mm_state(),
        }
    }
}
