// SPDX-License-Identifier: GPL-2.0

//! A wrapper for data protected by a lock that does not wrap it.

use super::{Guard, Lock};
use core::{cell::UnsafeCell, ops::Deref, ptr};

/// Allows access to some data to be serialised by a lock that does not wrap it.
///
/// In most cases, data protected by a lock is wrapped by the appropriate lock type, e.g.,
/// [`super::Mutex`] or [`super::SpinLock`]. [`LockedBy`] is meant for cases when this is not
/// possible. For example, if a container has a lock and some data in the contained elements needs
/// to be protected by the same lock.
///
/// [`LockedBy`] wraps the data in lieu of another locking primitive, and only allows access to it
/// when the caller shows evidence that 'external' lock is locked.
///
/// # Example
///
/// The following is an example for illustrative purposes: `InnerDirectory::bytes_used` is an
/// aggregate of all `InnerFile::bytes_used` and must be kept consistent; so we wrap `InnerFile` in
/// a `LockedBy` so that it shares a lock with `InnerDirectory`. This allows us to enforce at
/// compile-time that access to `InnerFile` is only granted when an `InnerDirectory` is also
/// locked; we enforce at run time that the right `InnerDirectory` is locked.
///
/// ```
/// # use kernel::prelude::*;
/// use kernel::sync::{LockedBy, Mutex};
///
/// struct InnerFile {
///     bytes_used: u64,
/// }
///
/// struct File {
///     name: String,
///     inner: LockedBy<InnerFile, Mutex<InnerDirectory>>,
/// }
///
/// struct InnerDirectory {
///     /// The sum of the bytes used by all files.
///     bytes_used: u64,
///     files: Vec<File>,
/// }
///
/// struct Directory {
///     name: String,
///     inner: Mutex<InnerDirectory>,
/// }
/// ```
pub struct LockedBy<T: ?Sized, L: Lock + ?Sized> {
    owner: *const L::Inner,
    data: UnsafeCell<T>,
}

// SAFETY: `LockedBy` can be transferred across thread boundaries iff the data it protects can.
unsafe impl<T: ?Sized + Send, L: Lock + ?Sized> Send for LockedBy<T, L> {}

// SAFETY: `LockedBy` serialises the interior mutability it provides, so it is `Sync` as long as the
// data it protects is `Send`.
unsafe impl<T: ?Sized + Send, L: Lock + ?Sized> Sync for LockedBy<T, L> {}

impl<T, L: Lock + ?Sized> LockedBy<T, L> {
    /// Constructs a new instance of [`LockedBy`].
    ///
    /// It stores a raw pointer to the owner that is never dereferenced. It is only used to ensure
    /// that the right owner is being used to access the protected data. If the owner is freed, the
    /// data becomes inaccessible; if another instance of the owner is allocated *on the same
    /// memory location*, the data becomes accessible again: none of this affects memory safety
    /// because in any case at most one thread (or CPU) can access the protected data at a time.
    pub fn new(owner: &L, data: T) -> Self {
        Self {
            owner: owner.locked_data().get(),
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized, L: Lock + ?Sized> LockedBy<T, L> {
    /// Returns a reference to the protected data when the caller provides evidence (via a
    /// [`Guard`]) that the owner is locked.
    pub fn access<'a>(&'a self, guard: &'a Guard<'_, L>) -> &'a T {
        if !ptr::eq(guard.deref(), self.owner) {
            panic!("guard does not match owner");
        }

        // SAFETY: `guard` is evidence that the owner is locked.
        unsafe { &mut *self.data.get() }
    }

    /// Returns a mutable reference to the protected data when the caller provides evidence (via a
    /// mutable [`Guard`]) that the owner is locked mutably.
    pub fn access_mut<'a>(&'a self, guard: &'a mut Guard<'_, L>) -> &'a mut T {
        if !ptr::eq(guard.deref().deref(), self.owner) {
            panic!("guard does not match owner");
        }

        // SAFETY: `guard` is evidence that the owner is locked.
        unsafe { &mut *self.data.get() }
    }

    /// Returns a mutable reference to the protected data when the caller provides evidence (via a
    /// mutable owner) that the owner is locked mutably. Showing a mutable reference to the owner
    /// is sufficient because we know no other references can exist to it.
    pub fn access_from_mut<'a>(&'a self, owner: &'a mut L::Inner) -> &'a mut T {
        if !ptr::eq(owner, self.owner) {
            panic!("mismatched owners");
        }

        // SAFETY: `owner` is evidence that there is only one reference to the owner.
        unsafe { &mut *self.data.get() }
    }
}
