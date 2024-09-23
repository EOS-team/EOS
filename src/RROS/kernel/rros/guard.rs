// To be removed once updated to the latest version of the Liunx.
use kernel::error::Result;

/// Allows mutual exclusion primitives that implement the [`RrosLock`] trait to automatically unlock
/// when a guard goes out of scope. It also provides a safe and convenient way to access the data
/// protected by the lock.
pub struct Guard<'a, L: RrosLock + ?Sized> {
    pub(crate) lock: &'a L,
}

// SAFETY: `Guard` is sync when the data protected by the lock is also sync. This is more
// conservative than the default compiler implementation; more details can be found on
// https://github.com/rust-lang/rust/issues/41622 -- it refers to `MutexGuard` from the standard
// library.
unsafe impl<L> Sync for Guard<'_, L>
where
    L: RrosLock + ?Sized,
    L::Inner: Sync,
{
}

impl<L: RrosLock + ?Sized> core::ops::Deref for Guard<'_, L> {
    type Target = L::Inner;

    fn deref(&self) -> &Self::Target {
        // SAFETY: The caller owns the lock, so it is safe to deref the protected data.
        unsafe { &*self.lock.locked_data().get() }
    }
}

impl<L: RrosLock + ?Sized> core::ops::DerefMut for Guard<'_, L> {
    fn deref_mut(&mut self) -> &mut L::Inner {
        // SAFETY: The caller owns the lock, so it is safe to deref the protected data.
        unsafe { &mut *self.lock.locked_data().get() }
    }
}

impl<L: RrosLock + ?Sized> Drop for Guard<'_, L> {
    fn drop(&mut self) {
        // SAFETY: The caller owns the lock, so it is safe to unlock it.
        unsafe { self.lock.unlock() };
    }
}

impl<'a, L: RrosLock + ?Sized> Guard<'a, L> {
    /// Constructs a new lock guard.
    ///
    /// # Safety
    ///
    /// The caller must ensure that it owns the lock.
    pub unsafe fn new(lock: &'a L) -> Self {
        Self { lock }
    }
}

/// A generic mutual exclusion primitive.
///
/// [`Guard`] is written such that any mutual exclusion primitive that can implement this trait can
/// also benefit from having an automatic way to unlock itself.
pub trait RrosLock {
    /// The type of the data protected by the lock.
    type Inner: ?Sized;

    /// Acquires the lock, making the caller its owner.
    fn lock_noguard(&self) -> Result<()>;

    /// Try to cquires the lock, making the caller its owner.
    fn try_lock_noguard(&self) -> Result<()>;
    /// Releases the lock, giving up ownership of the lock.
    ///
    /// # Safety
    ///
    /// It must only be called by the current owner of the lock.
    unsafe fn unlock(&self);

    /// Returns the data protected by the lock.
    fn locked_data(&self) -> &core::cell::UnsafeCell<Self::Inner>;
}
