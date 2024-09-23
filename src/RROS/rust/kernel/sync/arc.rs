// SPDX-License-Identifier: GPL-2.0

//! A reference-counted pointer.
//!
//! This module implements a way for users to create reference-counted objects and pointers to
//! them. Such a pointer automatically increments and decrements the count, and drops the
//! underlying object when it reaches zero. It is also safe to use concurrently from multiple
//! threads.
//!
//! It is different from the standard library's [`Arc`] in a few ways:
//! 1. It is backed by the kernel's `refcount_t` type.
//! 2. It does not support weak references, which allows it to be half the size.
//! 3. It saturates the reference count instead of aborting when it goes over a threshold.
//! 4. It does not provide a `get_mut` method, so the ref counted object is pinned.
//!
//! [`Arc`]: https://doc.rust-lang.org/std/sync/struct.Arc.html

use crate::{bindings, c_types, Opaque, Result};
use alloc::boxed::Box;
use core::{
    convert::AsRef, marker::PhantomData, mem::ManuallyDrop, ops::Deref, pin::Pin, ptr::NonNull,
};

extern "C" {
    #[allow(dead_code)]
    fn rust_helper_refcount_new() -> bindings::refcount_t;
    fn rust_helper_refcount_inc(r: *mut bindings::refcount_t);
    fn rust_helper_refcount_dec_and_test(r: *mut bindings::refcount_t) -> bool;
    fn rust_helper_REFCOUNT_INIT(n: c_types::c_int) -> bindings::refcount_t;
}

/// A reference-counted pointer to an instance of `T`.
///
/// The reference count is incremented when new instances of [`Ref`] are created, and decremented
/// when they are dropped. When the count reaches zero, the underlying `T` is also dropped.
///
/// # Invariants
///
/// The reference count on an instance of [`Ref`] is always non-zero.
/// The object pointed to by [`Ref`] is always pinned.
pub struct Ref<T: ?Sized> {
    ptr: NonNull<RefInner<T>>,
    _p: PhantomData<RefInner<T>>,
}

struct RefInner<T: ?Sized> {
    refcount: Opaque<bindings::refcount_t>,
    data: T,
}

// This is to allow [`Ref`] (and variants) to be used as the type of `self`.
impl<T: ?Sized> core::ops::Receiver for Ref<T> {}

// SAFETY: It is safe to send `Ref<T>` to another thread when the underlying `T` is `Sync` because
// it effectively means sharing `&T` (which is safe because `T` is `Sync`); additionally, it needs
// `T` to be `Send` because any thread that has a `Ref<T>` may ultimately access `T` directly, for
// example, when the reference count reaches zero and `T` is dropped.
unsafe impl<T: ?Sized + Sync + Send> Send for Ref<T> {}

// SAFETY: It is safe to send `&Ref<T>` to another thread when the underlying `T` is `Sync` for
// the same reason as above. `T` needs to be `Send` as well because a thread can clone a `&Ref<T>`
// into a `Ref<T>`, which may lead to `T` being accessed by the same reasoning as above.
unsafe impl<T: ?Sized + Sync + Send> Sync for Ref<T> {}

impl<T> Ref<T> {
    /// Constructs a new reference counted instance of `T`.
    pub fn try_new(contents: T) -> Result<Self> {
        Self::try_new_and_init(contents, |_| {})
    }

    /// Constructs a new reference counted instance of `T` and calls the initialisation function.
    ///
    /// This is useful because it provides a mutable reference to `T` at its final location.
    pub fn try_new_and_init<U: FnOnce(Pin<&mut T>)>(contents: T, init: U) -> Result<Self> {
        // INVARIANT: The refcount is initialised to a non-zero value.
        let mut inner = Box::try_new(RefInner {
            // SAFETY: Just an FFI call that returns a `refcount_t` initialised to 1.
            refcount: Opaque::new(unsafe { rust_helper_REFCOUNT_INIT(1) }),
            data: contents,
        })?;

        // SAFETY: By the invariant, `RefInner` is pinned and `T` is also pinned.
        let pinned = unsafe { Pin::new_unchecked(&mut inner.data) };

        // INVARIANT: The only places where `&mut T` is available are here, which is explicitly
        // pinned, and in `drop`. Both are compatible with the pin requirements.
        init(pinned);

        Ok(Ref {
            ptr: NonNull::from(Box::leak(inner)),
            _p: PhantomData,
        })
    }

    /// Deconstructs a [`Ref`] object into a `usize`.
    ///
    /// It can be reconstructed once via [`Ref::from_usize`].
    pub fn into_usize(obj: Self) -> usize {
        ManuallyDrop::new(obj).ptr.as_ptr() as _
    }

    /// Borrows a [`Ref`] instance previously deconstructed via [`Ref::into_usize`].
    ///
    /// # Safety
    ///
    /// `encoded` must have been returned by a previous call to [`Ref::into_usize`]. Additionally,
    /// [`Ref::from_usize`] can only be called after *all* instances of [`RefBorrow`] have been
    /// dropped.
    pub unsafe fn borrow_usize(encoded: usize) -> RefBorrow<T> {
        // SAFETY: By the safety requirement of this function, we know that `encoded` came from
        // a previous call to `Ref::into_usize`.
        let obj = ManuallyDrop::new(unsafe { Ref::from_usize(encoded) });

        // SAFEY: The safety requirements ensure that the object remains alive for the lifetime of
        // the returned value. There is no way to create mutable references to the object.
        unsafe { RefBorrow::new(obj) }
    }

    /// Recreates a [`Ref`] instance previously deconstructed via [`Ref::into_usize`].
    ///
    /// # Safety
    ///
    /// `encoded` must have been returned by a previous call to [`Ref::into_usize`]. Additionally,
    /// it can only be called once for each previous call to [``Ref::into_usize`].
    pub unsafe fn from_usize(encoded: usize) -> Self {
        Ref {
            ptr: NonNull::new(encoded as _).unwrap(),
            _p: PhantomData,
        }
    }
}

impl<T: ?Sized> Ref<T> {
    /// Determines if two reference-counted pointers point to the same underlying instance of `T`.
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        core::ptr::eq(a.ptr.as_ptr(), b.ptr.as_ptr())
    }

    /// Returns a pinned version of a given `Ref` instance.
    pub fn pinned(obj: Self) -> Pin<Self> {
        // SAFETY: The type invariants guarantee that the value is pinned.
        unsafe { Pin::new_unchecked(obj) }
    }
}

impl<T: ?Sized> Deref for Ref<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: By the type invariant, there is necessarily a reference to the object, so it is
        // safe to dereference it.
        unsafe { &self.ptr.as_ref().data }
    }
}

impl<T: ?Sized> Clone for Ref<T> {
    fn clone(&self) -> Self {
        // INVARIANT: C `refcount_inc` saturates the refcount, so it cannot overflow to zero.
        // SAFETY: By the type invariant, there is necessarily a reference to the object, so it is
        // safe to increment the refcount.
        unsafe { rust_helper_refcount_inc(self.ptr.as_ref().refcount.get()) };
        Self {
            ptr: self.ptr,
            _p: PhantomData,
        }
    }
}

impl<T: ?Sized> AsRef<T> for Ref<T> {
    fn as_ref(&self) -> &T {
        // SAFETY: By the type invariant, there is necessarily a reference to the object, so it is
        // safe to dereference it.
        unsafe { &self.ptr.as_ref().data }
    }
}

impl<T: ?Sized> Drop for Ref<T> {
    fn drop(&mut self) {
        // SAFETY: By the type invariant, there is necessarily a reference to the object. We cannot
        // touch `refcount` after it's decremented to a non-zero value because another thread/CPU
        // may concurrently decrement it to zero and free it. It is ok to have a raw pointer to
        // freed/invalid memory as long as it is never dereferenced.
        let refcount = unsafe { self.ptr.as_ref() }.refcount.get();

        // INVARIANT: If the refcount reaches zero, there are no other instances of `Ref`, and
        // this instance is being dropped, so the broken invariant is not observable.
        // SAFETY: Also by the type invariant, we are allowed to decrement the refcount.
        let is_zero = unsafe { rust_helper_refcount_dec_and_test(refcount) };
        if is_zero {
            // The count reached zero, we must free the memory.
            //
            // SAFETY: The pointer was initialised from the result of `Box::leak`.
            unsafe { Box::from_raw(self.ptr.as_ptr()) };
        }
    }
}

/// A borrowed [`Ref`] with manually-managed lifetime.
///
/// # Invariants
///
/// There are no mutable references to the underlying [`Ref`], and it remains valid for the lifetime
/// of the [`RefBorrow`] instance.
pub struct RefBorrow<T: ?Sized> {
    inner_ref: ManuallyDrop<Ref<T>>,
}

impl<T: ?Sized> RefBorrow<T> {
    /// Creates a new [`RefBorrow`] instance.
    ///
    /// # Safety
    ///
    /// Callers must ensure the following for the lifetime of the returned [`RefBorrow`] instance:
    /// 1. That `obj` remains valid;
    /// 2. That no mutable references to `obj` are created.
    unsafe fn new(obj: ManuallyDrop<Ref<T>>) -> Self {
        // INVARIANT: The safety requirements guarantee the invariants.
        Self { inner_ref: obj }
    }
}

impl<T: ?Sized> Deref for RefBorrow<T> {
    type Target = Ref<T>;

    fn deref(&self) -> &Self::Target {
        self.inner_ref.deref()
    }
}

// impl<T> TryFrom<Vec<T>> for Ref<[T]> {
//     type Error = Error;
//     fn try_from(mut v: Vec<T>) -> Result<Self> {
//         let value_layout = Layout::array::<T>(v.len())?;
//         let layout = Layout::new::<RefInner<()>>()
//             .extend(value_layout)?
//             .0
//             .pad_to_align();
//         // SAFETY: The layout size is guaranteed to be non-zero because `RefInner` contains the
//         // reference count.
//         let ptr = NonNull::new(unsafe { alloc(layout) }).ok_or(Error::ENOMEM)?;
//         let inner =
//             core::ptr::slice_from_raw_parts_mut(ptr.as_ptr() as _, v.len()) as *mut RefInner<[T]>;
//         // SAFETY: Just an FFI call that returns a `refcount_t` initialised to 1.
//         let count = Opaque::new(unsafe { bindings::REFCOUNT_INIT(1) });
//         // SAFETY: `inner.refcount` is writable and properly aligned.
//         unsafe { core::ptr::addr_of_mut!((*inner).refcount).write(count) };
//         // SAFETY: The contents of `v` as readable and properly aligned; `inner.data` is writable
//         // and properly aligned. There is no overlap between the two because `inner` is a new
//         // allocation.
//         unsafe {
//             core::ptr::copy_nonoverlapping(
//                 v.as_ptr(),
//                 core::ptr::addr_of_mut!((*inner).data) as *mut [T] as *mut T,
//                 v.len(),
//             )
//         };
//         // SAFETY: We're setting the new length to zero, so it is <= to capacity, and old_len..0 is
//         // an empty range (so satisfies vacuously the requirement of being initialised).
//         unsafe { v.set_len(0) };
//         // SAFETY: We just created `inner` with a reference count of 1, which is owned by the new
//         // `Ref` object.
//         Ok(unsafe { Self::from_inner(NonNull::new(inner).unwrap()) })
//     }
// }
