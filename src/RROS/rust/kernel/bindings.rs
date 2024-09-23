// SPDX-License-Identifier: GPL-2.0

//! Bindings
//!
//! Imports the generated bindings by `bindgen`.

// See https://github.com/rust-lang/rust-bindgen/issues/1651.
#![cfg_attr(test, allow(deref_nullptr))]
#![cfg_attr(test, allow(unaligned_references))]
#![cfg_attr(test, allow(unsafe_op_in_unsafe_fn))]

#[allow(
    clippy::all,
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    improper_ctypes,
    unsafe_op_in_unsafe_fn
)]
mod bindings_raw {
    use crate::c_types;
    include!(env!("RUST_BINDINGS_FILE"));
}
pub use bindings_raw::*;

pub const GFP_KERNEL: gfp_t = BINDINGS_GFP_KERNEL;
pub const __GFP_ZERO: gfp_t = BINDINGS___GFP_ZERO;
pub const __GFP_HIGHMEM: gfp_t = ___GFP_HIGHMEM;
