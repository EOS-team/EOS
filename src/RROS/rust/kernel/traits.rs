// SPDX-License-Identifier: GPL-2.0

//! Traits useful to drivers, and their implementations for common types.

use core::{ops::Deref, pin::Pin};

use alloc::{alloc::AllocError, sync::Arc};

/// Trait which provides a fallible version of `pin()` for pointer types.
///
/// Common pointer types which implement a `pin()` method include [`Box`](alloc::boxed::Box) and [`Arc`].
pub trait TryPin<P: Deref> {
    /// Constructs a new `Pin<pointer<T>>`. If `T` does not implement [`Unpin`], then data
    /// will be pinned in memory and unable to be moved. An error will be returned
    /// if allocation fails.
    fn try_pin(data: P::Target) -> core::result::Result<Pin<P>, AllocError>;
}

impl<T> TryPin<Arc<T>> for Arc<T> {
    fn try_pin(data: T) -> core::result::Result<Pin<Arc<T>>, AllocError> {
        // SAFETY: the data `T` is exposed only through a `Pin<Arc<T>>`, which
        // does not allow data to move out of the `Arc`. Therefore it can
        // never be moved.
        Ok(unsafe { Pin::new_unchecked(Arc::try_new(data)?) })
    }
}
