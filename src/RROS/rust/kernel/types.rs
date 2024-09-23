// SPDX-License-Identifier: GPL-2.0

//! Kernel types.
//!
//! C header: [`include/linux/types.h`](../../../../include/linux/types.h)

use crate::{
    bindings, c_types,
    sync::{Ref, RefBorrow},
};
use alloc::{boxed::Box, sync::Arc};
use core::{
    cell::UnsafeCell, marker::PhantomData, mem::MaybeUninit, ops::Deref, pin::Pin, ptr::NonNull,
};

extern "C" {
    fn rust_helper_atomic_add(i: i32, v: *mut bindings::atomic_t);
    fn rust_helper_atomic_sub(i: i32, v: *mut bindings::atomic_t);
    fn rust_helper_atomic_sub_return(i: i32, v: *mut bindings::atomic_t) -> i32;
    fn rust_helper_atomic_add_return(i: i32, v: *mut bindings::atomic_t) -> i32;
    fn rust_helper_atomic_cmpxchg(v: *mut bindings::atomic_t, old: i32, new: i32) -> i32;
    fn rust_helper_atomic_set(v: *mut bindings::atomic_t, i: i32);
    fn rust_helper_atomic_inc(v: *mut bindings::atomic_t);
    fn rust_helper_atomic_dec_and_test(v: *mut bindings::atomic_t) -> bool;
    fn rust_helper_atomic_dec_return(v: *mut bindings::atomic_t) -> i32;
    fn rust_helper_atomic_read(v: *mut bindings::atomic_t) -> i32;
}

/// Permissions.
///
/// C header: [`include/uapi/linux/stat.h`](../../../../include/uapi/linux/stat.h)
///
/// C header: [`include/linux/stat.h`](../../../../include/linux/stat.h)
pub struct Mode(bindings::umode_t);

impl Mode {
    /// Creates a [`Mode`] from an integer.
    pub fn from_int(m: u16) -> Mode {
        Mode(m)
    }

    /// Returns the mode as an integer.
    pub fn as_int(&self) -> u16 {
        self.0
    }
}

/// Used to convert an object into a raw pointer that represents it.
///
/// It can eventually be converted back into the object. This is used to store objects as pointers
/// in kernel data structures, for example, an implementation of [`FileOperations`] in `struct
/// file::private_data`.
pub trait PointerWrapper {
    /// Type of values borrowed between calls to [`PointerWrapper::into_pointer`] and
    /// [`PointerWrapper::from_pointer`].
    type Borrowed: Deref;

    /// Returns the raw pointer.
    fn into_pointer(self) -> *const c_types::c_void;

    /// Returns a borrowed value.
    ///
    /// # Safety
    ///
    /// `ptr` must have been returned by a previous call to [`PointerWrapper::into_pointer`].
    /// Additionally, [`PointerWrapper::from_pointer`] can only be called after *all* values
    /// returned by [`PointerWrapper::borrow`] have been dropped.
    unsafe fn borrow(ptr: *const c_types::c_void) -> Self::Borrowed;

    /// Returns the instance back from the raw pointer.
    ///
    /// # Safety
    ///
    /// The passed pointer must come from a previous call to [`PointerWrapper::into_pointer()`].
    unsafe fn from_pointer(ptr: *const c_types::c_void) -> Self;
}

impl<T> PointerWrapper for Box<T> {
    type Borrowed = UnsafeReference<T>;

    fn into_pointer(self) -> *const c_types::c_void {
        Box::into_raw(self) as _
    }

    unsafe fn borrow(ptr: *const c_types::c_void) -> Self::Borrowed {
        // SAFETY: The safety requirements for this function ensure that the object is still alive,
        // so it is safe to dereference the raw pointer.
        // The safety requirements also ensure that the object remains alive for the lifetime of
        // the returned value.
        unsafe { UnsafeReference::new(&*ptr.cast()) }
    }

    unsafe fn from_pointer(ptr: *const c_types::c_void) -> Self {
        // SAFETY: The passed pointer comes from a previous call to [`Self::into_pointer()`].
        unsafe { Box::from_raw(ptr as _) }
    }
}

impl<T> PointerWrapper for Ref<T> {
    type Borrowed = RefBorrow<T>;

    fn into_pointer(self) -> *const c_types::c_void {
        Ref::into_usize(self) as _
    }

    unsafe fn borrow(ptr: *const c_types::c_void) -> Self::Borrowed {
        // SAFETY: The safety requirements for this function ensure that the underlying object
        // remains valid for the lifetime of the returned value.
        unsafe { Ref::borrow_usize(ptr as _) }
    }

    unsafe fn from_pointer(ptr: *const c_types::c_void) -> Self {
        // SAFETY: The passed pointer comes from a previous call to [`Self::into_pointer()`].
        unsafe { Ref::from_usize(ptr as _) }
    }
}

impl<T> PointerWrapper for Arc<T> {
    type Borrowed = UnsafeReference<T>;

    fn into_pointer(self) -> *const c_types::c_void {
        Arc::into_raw(self) as _
    }

    unsafe fn borrow(ptr: *const c_types::c_void) -> Self::Borrowed {
        // SAFETY: The safety requirements for this function ensure that the object is still alive,
        // so it is safe to dereference the raw pointer.
        // The safety requirements also ensure that the object remains alive for the lifetime of
        // the returned value.
        unsafe { UnsafeReference::new(&*ptr.cast()) }
    }

    unsafe fn from_pointer(ptr: *const c_types::c_void) -> Self {
        // SAFETY: The passed pointer comes from a previous call to [`Self::into_pointer()`].
        unsafe { Arc::from_raw(ptr as _) }
    }
}

/// A reference with manually-managed lifetime.
///
/// # Invariants
///
/// There are no mutable references to the underlying object, and it remains valid for the lifetime
/// of the [`UnsafeReference`] instance.
pub struct UnsafeReference<T: ?Sized> {
    ptr: NonNull<T>,
}

impl<T: ?Sized> UnsafeReference<T> {
    /// Creates a new [`UnsafeReference`] instance.
    ///
    /// # Safety
    ///
    /// Callers must ensure the following for the lifetime of the returned [`UnsafeReference`]
    /// instance:
    /// 1. That `obj` remains valid;
    /// 2. That no mutable references to `obj` are created.
    unsafe fn new(obj: &T) -> Self {
        // INVARIANT: The safety requirements of this function ensure that the invariants hold.
        Self {
            ptr: NonNull::from(obj),
        }
    }
}

impl<T: ?Sized> Deref for UnsafeReference<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: By the type invariant, the object is still valid and alive, and there are no
        // mutable references to it.
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: PointerWrapper + Deref> PointerWrapper for Pin<T> {
    type Borrowed = T::Borrowed;

    fn into_pointer(self) -> *const c_types::c_void {
        // SAFETY: We continue to treat the pointer as pinned by returning just a pointer to it to
        // the caller.
        let inner = unsafe { Pin::into_inner_unchecked(self) };
        inner.into_pointer()
    }

    unsafe fn borrow(ptr: *const c_types::c_void) -> Self::Borrowed {
        // SAFETY: The safety requirements for this function are the same as the ones for
        // `T::borrow`.
        unsafe { T::borrow(ptr) }
    }

    unsafe fn from_pointer(p: *const c_types::c_void) -> Self {
        // SAFETY: The object was originally pinned.
        // The passed pointer comes from a previous call to `inner::into_pointer()`.
        unsafe { Pin::new_unchecked(T::from_pointer(p)) }
    }
}

/// Runs a cleanup function/closure when dropped.
///
/// The [`ScopeGuard::dismiss`] function prevents the cleanup function from running.
///
/// # Examples
///
/// In the example below, we have multiple exit paths and we want to log regardless of which one is
/// taken:
/// ```
/// # use kernel::prelude::*;
/// # use kernel::ScopeGuard;
/// fn example1(arg: bool) {
///     let _log = ScopeGuard::new(|| pr_info!("example1 completed\n"));
///
///     if arg {
///         return;
///     }
///
///     // Do something...
/// }
/// ```
///
/// In the example below, we want to log the same message on all early exits but a different one on
/// the main exit path:
/// ```
/// # use kernel::prelude::*;
/// # use kernel::ScopeGuard;
/// fn example2(arg: bool) {
///     let log = ScopeGuard::new(|| pr_info!("example2 returned early\n"));
///
///     if arg {
///         return;
///     }
///
///     // (Other early returns...)
///
///     log.dismiss();
///     pr_info!("example2 no early return\n");
/// }
/// ```
pub struct ScopeGuard<T: FnOnce()> {
    cleanup_func: Option<T>,
}

impl<T: FnOnce()> ScopeGuard<T> {
    /// Creates a new cleanup object with the given cleanup function.
    pub fn new(cleanup_func: T) -> Self {
        Self {
            cleanup_func: Some(cleanup_func),
        }
    }

    /// Prevents the cleanup function from running.
    pub fn dismiss(mut self) {
        self.cleanup_func.take();
    }
}

impl<T: FnOnce()> Drop for ScopeGuard<T> {
    fn drop(&mut self) {
        // Run the cleanup function if one is still present.
        if let Some(cleanup) = self.cleanup_func.take() {
            cleanup();
        }
    }
}

/// Stores an opaque value.
///
/// This is meant to be used with FFI objects that are never interpreted by Rust code.
#[repr(transparent)]
pub struct Opaque<T>(MaybeUninit<UnsafeCell<T>>);

impl<T> Opaque<T> {
    /// Creates a new opaque value.
    pub fn new(value: T) -> Self {
        Self(MaybeUninit::new(UnsafeCell::new(value)))
    }

    /// Creates an uninitialised value.
    pub const fn uninit() -> Self {
        Self(MaybeUninit::uninit())
    }

    /// Returns a raw pointer to the opaque data.
    pub fn get(&self) -> *mut T {
        UnsafeCell::raw_get(self.0.as_ptr())
    }
}

extern "C" {
    fn rust_helper_hash_init(ht: *mut bindings::hlist_head, size: u32);
    #[allow(dead_code)]
    fn rust_helper_rcu_read_lock();
    #[allow(dead_code)]
    fn rust_helper_rcu_read_unlock();
}

/// The `RcuHead` struct is a wrapper around the `bindings::callback_head` struct from the kernel bindings. It represents a node in a Read-Copy-Update (RCU) list.
pub struct RcuHead(bindings::callback_head);

impl RcuHead {
    /// The `new` method is a constructor for `RcuHead`. It creates a new `RcuHead` with a default `bindings::callback_head`.
    pub fn new() -> Self {
        Self(bindings::callback_head::default())
    }
}

/// The `HlistNode` struct is a wrapper around the `bindings::hlist_node` struct from the kernel bindings. It represents a node in a hash list.
#[repr(transparent)]
pub struct HlistNode(pub bindings::hlist_node);

impl HlistNode {
    /// The `new` method is a constructor for `HlistNode`. It creates a new `HlistNode` with a default `bindings::hlist_node`.
    pub fn new() -> Self {
        Self(bindings::hlist_node::default())
    }
    // pub fn hash_del(&mut self){
    //     extern "C"{
    //         fn rust_helper_hash_del(node: *mut bindings::hlist_node);
    //     }
    //     unsafe{
    //         rust_helper_hash_del(&mut self.0 as *mut bindings::hlist_node);
    //     }
    // }
}

#[derive(Clone, Copy)]
/// The `HlistHead` struct is a wrapper around the `bindings::hlist_head` struct from the kernel bindings. It represents the head of a hash list.
pub struct HlistHead(bindings::hlist_head);

impl HlistHead {
    /// The `new` method is a constructor for `HlistHead`. It creates a new `HlistHead` with a default `bindings::hlist_head`.
    pub fn new() -> Self {
        Self(bindings::hlist_head::default())
    }

    /// The `as_list_head` method returns a mutable pointer to the underlying `bindings::hlist_head`. This can be used to pass the `HlistHead` to kernel functions that expect a `bindings::hlist_head`.
    pub fn as_list_head(&mut self) -> *mut bindings::hlist_head {
        &mut self.0 as *mut bindings::hlist_head
    }
}

/// The `hash_init` function is a wrapper around the `rust_helper_hash_init` function from the kernel bindings. It initializes a hash table with the given size. The `ht` parameter is a pointer to the hash table to initialize.
pub fn hash_init(ht: *mut bindings::hlist_head, size: u32) {
    unsafe { rust_helper_hash_init(ht, size) };
}

/// A list to store structs needed to hash.
pub struct Hashtable<const N: usize> {
    table: [bindings::hlist_head; N],
}

unsafe impl<const N: usize> Sync for Hashtable<N> {}

unsafe impl<const N: usize> Send for Hashtable<N> {}

impl<const N: usize> Hashtable<N> {
    /// Constructs a new struct.
    pub const fn new() -> Self {
        let table = [bindings::hlist_head {
            first: core::ptr::null_mut(),
        }; N];
        Self { table: table }
    }

    /// Add a new struct to Hashtable.
    pub fn add(&mut self, node: &mut bindings::hlist_node, key: u32) {
        extern "C" {
            fn rust_helper_hash_add(
                ht: *mut bindings::hlist_head,
                length: usize,
                node: *mut bindings::hlist_node,
                key: u32,
            );
        }
        unsafe {
            rust_helper_hash_add(
                &self.table as *const _ as *mut bindings::hlist_head,
                N,
                node as *mut bindings::hlist_node,
                key,
            );
        }
    }

    /// Delete a struct from Hashtable.
    pub fn del(&self, node: &mut bindings::hlist_node) {
        extern "C" {
            fn rust_helper_hash_del(node: *mut bindings::hlist_node);
        }
        unsafe {
            rust_helper_hash_del(node as *mut bindings::hlist_node);
        }
    }

    /// Get the bucket's head which is indexed by key.
    pub fn head(&mut self, key: u32) -> *const bindings::hlist_head {
        extern "C" {
            fn rust_helper_get_hlist_head(
                ht: *const bindings::hlist_head,
                length: usize,
                key: u32,
            ) -> *const bindings::hlist_head;
        }
        unsafe { rust_helper_get_hlist_head(&self.table as *const bindings::hlist_head, N, key) }
    }
}

/// Initialize a `Hashtable` struct.
#[macro_export]
macro_rules! initialize_lock_hashtable {
    ($name:ident,$bits_to_shift:expr) => {
        kernel::init_static_sync! {
            static $name: kernel::sync::Mutex<Hashtable::<$bits_to_shift>> = Hashtable::<$bits_to_shift>::new();
        }
    }
}

/// Get the struct for this entry from a ptr.
#[macro_export]
macro_rules! hlist_entry {
    ($ptr:expr, $type:ty, $($f:tt)*) => {
        kernel::container_of!($ptr, $type, $($f)*)
    }
}

/// Get the struct for this entry from a non-null ptr.
#[macro_export]
macro_rules! hlist_entry_safe {
    ($ptr:expr, $type:ty, $($f:tt)*) => {
        if ($ptr).is_null() {
            core::ptr::null()
        } else {
            kernel::container_of!($ptr, $type, $($f)*)
        }
    }
}

/// Iterate all non-null struct begin with a entry.
#[macro_export]
macro_rules! hash_for_each_possible {
    ($pos:ident, $head:expr, $type:ty, $member:ident, { $($block:tt)* } ) => {
        let mut $pos = $crate::hlist_entry_safe!(unsafe { (*$head).first }, $type, $member);
        while (!$pos.is_null()) {
            // $code
            $($block)*
            $pos = $crate::hlist_entry_safe!(unsafe { (*$pos).$member.0.next }, $type, $member);
        }
    };
}

/// Types that are _always_ reference counted.
///
/// It allows such types to define their own custom ref increment and decrement functions.
/// Additionally, it allows users to convert from a shared reference `&T` to an owned reference
/// [`ARef<T>`].
///
/// This is usually implemented by wrappers to existing structures on the C side of the code. For
/// Rust code, the recommendation is to use [`Arc`] to create reference-counted instances of a
/// type.
///
/// # Safety
///
/// Implementers must ensure that increments to the reference count keeps the object alive in
/// memory at least until a matching decrement performed.
///
/// Implementers must also ensure that all instances are reference-counted. (Otherwise they
/// won't be able to honour the requirement that [`AlwaysRefCounted::inc_ref`] keep the object
/// alive.)
pub unsafe trait AlwaysRefCounted {
    /// Increments the reference count on the object.
    fn inc_ref(&self);

    /// Decrements the reference count on the object.
    ///
    /// Frees the object when the count reaches zero.
    ///
    /// # Safety
    ///
    /// Callers must ensure that there was a previous matching increment to the reference count,
    /// and that the object is no longer used after its reference count is decremented (as it may
    /// result in the object being freed), unless the caller owns another increment on the refcount
    /// (e.g., it calls [`AlwaysRefCounted::inc_ref`] twice, then calls
    /// [`AlwaysRefCounted::dec_ref`] once).
    unsafe fn dec_ref(obj: NonNull<Self>);
}

/// An owned reference to an always-reference-counted object.
///
/// The object's reference count is automatically decremented when an instance of [`ARef`] is
/// dropped. It is also automatically incremented when a new instance is created via
/// [`ARef::clone`].
///
/// # Invariants
///
/// The pointer stored in `ptr` is non-null and valid for the lifetime of the [`ARef`] instance. In
/// particular, the [`ARef`] instance owns an increment on underlying object's reference count.
pub struct ARef<T: AlwaysRefCounted> {
    ptr: NonNull<T>,
    _p: PhantomData<T>,
}

impl<T: AlwaysRefCounted> ARef<T> {
    /// Creates a new instance of [`ARef`].
    ///
    /// It takes over an increment of the reference count on the underlying object.
    ///
    /// # Safety
    ///
    /// Callers must ensure that the reference count was incremented at least once, and that they
    /// are properly relinquishing one increment. That is, if there is only one increment, callers
    /// must not use the underlying object anymore -- it is only safe to do so via the newly
    /// created [`ARef`].
    pub unsafe fn from_raw(ptr: NonNull<T>) -> Self {
        // INVARIANT: The safety requirements guarantee that the new instance now owns the
        // increment on the refcount.
        Self {
            ptr,
            _p: PhantomData,
        }
    }
}
impl<T: AlwaysRefCounted> Clone for ARef<T> {
    fn clone(&self) -> Self {
        self.inc_ref();
        // SAFETY: We just incremented the refcount above.
        unsafe { Self::from_raw(self.ptr) }
    }
}

impl<T: AlwaysRefCounted> Deref for ARef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: The type invariants guarantee that the object is valid.
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: AlwaysRefCounted> From<&T> for ARef<T> {
    fn from(b: &T) -> Self {
        b.inc_ref();
        // SAFETY: We just incremented the refcount above.
        unsafe { Self::from_raw(NonNull::from(b)) }
    }
}

impl<T: AlwaysRefCounted> Drop for ARef<T> {
    fn drop(&mut self) {
        // SAFETY: The type invariants guarantee that the `ARef` owns the reference we're about to
        // decrement.
        unsafe { T::dec_ref(self.ptr) };
    }
}

/// A wrapper for [`sched_param`].
#[derive(Default)]
#[repr(transparent)]
pub struct SchedParam {
    #[allow(dead_code)]
    sched_param: bindings::sched_param,
}

impl SchedParam {
    /// Constructs a new struct with a `c_int` parameter represent `sched_priority`.
    pub fn new(n: c_types::c_int) -> Self {
        Self {
            sched_param: bindings::sched_param { sched_priority: n },
        }
    }
}

/// A wrapper for [`atomic_t`].
#[repr(transparent)]
pub struct Atomic(bindings::atomic_t);

impl Atomic {
    /// Constructs a new struct.
    pub fn new() -> Self {
        Atomic(bindings::atomic_t::default())
    }

    /// Add a num to self.
    pub fn atomic_add(&mut self, i: i32) {
        unsafe {
            rust_helper_atomic_add(i, &mut self.0 as *mut bindings::atomic_t);
        }
    }

    /// Subtract a num to self.
    pub fn atomic_sub(&mut self, i: i32) {
        unsafe {
            rust_helper_atomic_sub(i, &mut self.0 as *mut bindings::atomic_t);
        }
    }

    /// Subtract and return the old value.
    pub fn atomic_sub_return(&mut self, i: i32) -> i32 {
        unsafe { rust_helper_atomic_sub_return(i, &mut self.0 as *mut bindings::atomic_t) }
    }

    /// Add to self and return the old value.
    pub fn atomic_add_return(&mut self, i: i32) -> i32 {
        unsafe { rust_helper_atomic_add_return(i, &mut self.0 as *mut bindings::atomic_t) }
    }

    /// Compare, if same exchange to new, else nothing to do.
    pub fn atomic_cmpxchg(&mut self, old: i32, new: i32) -> i32 {
        unsafe { rust_helper_atomic_cmpxchg(&mut self.0 as *mut bindings::atomic_t, old, new) }
    }

    /// Set to a num.
    pub fn atomic_set(&mut self, i: i32) {
        unsafe {
            rust_helper_atomic_set(&mut self.0 as *mut bindings::atomic_t, i);
        }
    }

    /// Plus one.
    pub fn atomic_inc(&mut self) {
        unsafe {
            rust_helper_atomic_inc(&mut self.0 as *mut bindings::atomic_t);
        }
    }

    /// Sub one and test whether is zero.
    pub fn atomic_dec_and_test(&mut self) -> bool {
        unsafe { rust_helper_atomic_dec_and_test(&mut self.0 as *mut bindings::atomic_t) }
    }

    /// Sub one and return the old value.
    pub fn atomic_dec_return(&mut self) -> i32 {
        unsafe { rust_helper_atomic_dec_return(&mut self.0 as *mut bindings::atomic_t) }
    }

    /// Read self's value.
    pub fn atomic_read(&mut self) -> i32 {
        unsafe { rust_helper_atomic_read(&mut self.0 as *mut bindings::atomic_t) }
    }
}
