// SPDX-License-Identifier: GPL-2.0

//! completion
//!
//! C header: [`include/linux/completion.h`](../../../../include/linux/completion.h)

use crate::bindings;

extern "C" {
    fn rust_helper_init_completion(x: *mut bindings::completion);
}
/// The `Completion` struct is a wrapper around the `bindings::completion` struct from the kernel bindings. It represents a completion object in the kernel.
pub struct Completion(bindings::completion);

impl Completion {
    /// The `new` method is a constructor for `Completion`. It creates a new `Completion` with a default `bindings::completion`.
    pub fn new() -> Self {
        let completion = bindings::completion::default();
        Self(completion)
    }

    /// The `init_completion` method initializes the completion object. It does this by calling the unsafe `rust_helper_init_completion` function with a pointer to the underlying `bindings::completion`.
    pub fn init_completion(&mut self) {
        unsafe { rust_helper_init_completion(&mut self.0 as *mut bindings::completion) }
    }

    /// The `complete` method signals that the operation represented by the completion object has completed. It does this by calling the unsafe `bindings::complete` function with a pointer to the underlying `bindings::completion`.
    pub fn complete(&mut self) {
        unsafe { bindings::complete(&mut self.0 as *mut bindings::completion) }
    }

    /// The `wait_for_completion` method waits for the operation represented by the completion object to complete. It does this by calling the unsafe `bindings::wait_for_completion` function with a pointer to the underlying `bindings::completion`.
    pub fn wait_for_completion(&mut self) {
        unsafe { bindings::wait_for_completion(&mut self.0 as *mut bindings::completion) }
    }
}
