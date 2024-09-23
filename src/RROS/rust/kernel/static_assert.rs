// SPDX-License-Identifier: GPL-2.0

//! Static assert.

/// Static assert (i.e. compile-time assert).
///
/// Similar to C11 [`_Static_assert`] and C++11 [`static_assert`].
///
/// The feature may be added to Rust in the future: see [RFC 2790].
///
/// [`_Static_assert`]: https://en.cppreference.com/w/c/language/_Static_assert
/// [`static_assert`]: https://en.cppreference.com/w/cpp/language/static_assert
/// [RFC 2790]: https://github.com/rust-lang/rfcs/issues/2790
///
/// # Examples
///
/// ```
/// # use kernel::prelude::*;
/// static_assert!(42 > 24);
/// static_assert!(core::mem::size_of::<u8>() == 1);
///
/// const X: &[u8] = b"bar";
/// static_assert!(X[1] == 'a' as u8);
///
/// const fn f(x: i32) -> i32 {
///     x + 2
/// }
/// static_assert!(f(40) == 42);
/// ```
#[macro_export]
macro_rules! static_assert {
    ($condition:expr) => {
        // Based on the latest one in `rustc`'s one before it was [removed].
        //
        // [removed]: https://github.com/rust-lang/rust/commit/c2dad1c6b9f9636198d7c561b47a2974f5103f6d
        #[allow(dead_code)]
        const _: () = [()][!($condition) as usize];
    };
}
