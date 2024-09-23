// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Memory allocation APIs

#![stable(feature = "alloc_module", since = "1.28.0")]

#[cfg(not(test))]
use core::intrinsics;

#[cfg(not(test))]
use core::ptr::{self, NonNull};

#[stable(feature = "alloc_module", since = "1.28.0")]
#[doc(inline)]
pub use core::alloc::*;

#[cfg(test)]
mod tests;

extern "Rust" {
    #[rustc_allocator]
    #[rustc_allocator_nounwind]
    fn __rros_sys_heap_alloc(size: usize, align: usize) -> *mut u8;
    #[rustc_allocator_nounwind]
    fn __rros_sys_heap_dealloc(ptr: *mut u8, size: usize, align: usize);
    #[rustc_allocator_nounwind]
    fn __rros_sys_heap_realloc(ptr: *mut u8, old_size: usize, align: usize, new_size: usize) -> *mut u8;
    #[rustc_allocator_nounwind]
    fn __rros_sys_heap_alloc_zerod(size: usize, align: usize) -> *mut u8;
}
#[unstable(feature = "allocator_api", issue = "32838")]
#[derive(Copy, Clone, Default, Debug)]
#[cfg(not(test))]
/// The `RrosMem` struct is a memory allocator that interfaces with the RROS system's heap memory management.
pub struct RrosMem;

#[stable(feature = "rros_alloc", since = "1.28.0")]
#[inline]
/// The `rros_alloc` function is an unsafe function that allocates a block of memory on the heap of the given layout. It returns a raw pointer to the start of the allocated block.
pub unsafe fn rros_alloc(layout: Layout) -> *mut u8 {
    unsafe { __rros_sys_heap_alloc(layout.size(), layout.align()) }
}

#[stable(feature = "rros_alloc", since = "1.28.0")]
#[inline]
/// The `rros_dealloc` function is an unsafe function that deallocates a block of memory on the heap. The block must have been previously allocated by `rros_alloc` and not yet deallocated.
pub unsafe fn rros_dealloc(ptr: *mut u8, layout: Layout) {
    unsafe { __rros_sys_heap_dealloc(ptr, layout.size(), layout.align()) }
}

#[stable(feature = "rros_alloc", since = "1.28.0")]
#[inline]
/// The `rros_realloc` function is an unsafe function that changes the size of the memory block pointed to by `ptr` to `new_size`. It may move the memory block to a new location, in which case the new location is returned.
pub unsafe fn rros_realloc(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
    unsafe { __rros_sys_heap_realloc(ptr, layout.size(), layout.align(), new_size) }
}

#[stable(feature = "rros_alloc", since = "1.28.0")]
#[inline]
/// The `rros_alloc_zeroed` function is an unsafe function that allocates a block of memory on the heap of the given layout and then fills it with zeroes. It returns a raw pointer to the start of the allocated block.
pub unsafe fn rros_alloc_zeroed(layout: Layout) -> *mut u8 {
    unsafe { __rros_sys_heap_alloc_zerod(layout.size(), layout.align()) }
}

#[cfg(not(test))]
impl RrosMem {
    #[inline]
    fn alloc_impl(&self, layout: Layout, zeroed: bool) -> Result<NonNull<[u8]>, AllocError> {
        match layout.size() {
            0 => Ok(NonNull::slice_from_raw_parts(layout.dangling(), 0)),
            size => unsafe {
                let raw_ptr = if zeroed { rros_alloc_zeroed(layout) } else { rros_alloc(layout) };
                let ptr = NonNull::new(raw_ptr).ok_or(AllocError)?;
                Ok(NonNull::slice_from_raw_parts(ptr, size))
            },
        }
    }

    // SAFETY: Same as `Allocator::grow`
    #[inline]
    unsafe fn grow_impl(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
        zeroed: bool,
    ) -> Result<NonNull<[u8]>, AllocError> {
        debug_assert!(
            new_layout.size() >= old_layout.size(),
            "`new_layout.size()` must be greater than or equal to `old_layout.size()`"
        );

        match old_layout.size() {
            0 => self.alloc_impl(new_layout, zeroed),

            // SAFETY: `new_size` is non-zero as `old_size` is greater than or equal to `new_size`
            // as required by safety conditions. Other conditions must be upheld by the caller
            old_size if old_layout.align() == new_layout.align() => unsafe {
                let new_size = new_layout.size();

                // `realloc` probably checks for `new_size >= old_layout.size()` or something similar.
                intrinsics::assume(new_size >= old_layout.size());

                let raw_ptr = rros_realloc(ptr.as_ptr(), old_layout, new_size);
                let ptr = NonNull::new(raw_ptr).ok_or(AllocError)?;
                if zeroed {
                    raw_ptr.add(old_size).write_bytes(0, new_size - old_size);
                }
                Ok(NonNull::slice_from_raw_parts(ptr, new_size))
            },

            // SAFETY: because `new_layout.size()` must be greater than or equal to `old_size`,
            // both the old and new memory allocation are valid for reads and writes for `old_size`
            // bytes. Also, because the old allocation wasn't yet deallocated, it cannot overlap
            // `new_ptr`. Thus, the call to `copy_nonoverlapping` is safe. The safety contract
            // for `dealloc` must be upheld by the caller.
            old_size => unsafe {
                let new_ptr = self.alloc_impl(new_layout, zeroed)?;
                ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_mut_ptr(), old_size);
                self.deallocate(ptr, old_layout);
                Ok(new_ptr)
            },
        }
    }
}

#[unstable(feature = "allocator_api", issue = "32838")]
#[cfg(not(test))]
unsafe impl Allocator for RrosMem {
    #[inline]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.alloc_impl(layout, false)
    }

    #[inline]
    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.alloc_impl(layout, true)
    }

    #[inline]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        if layout.size() != 0 {
            // SAFETY: `layout` is non-zero in size,
            // other conditions must be upheld by the caller
            unsafe { rros_dealloc(ptr.as_ptr(), layout) }
        }
    }

    #[inline]
    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        // SAFETY: all conditions must be upheld by the caller
        unsafe { self.grow_impl(ptr, old_layout, new_layout, false) }
    }

    #[inline]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        // SAFETY: all conditions must be upheld by the caller
        unsafe { self.grow_impl(ptr, old_layout, new_layout, true) }
    }

    #[inline]
    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        debug_assert!(
            new_layout.size() <= old_layout.size(),
            "`new_layout.size()` must be smaller than or equal to `old_layout.size()`"
        );

        match new_layout.size() {
            // SAFETY: conditions must be upheld by the caller
            0 => unsafe {
                self.deallocate(ptr, old_layout);
                Ok(NonNull::slice_from_raw_parts(new_layout.dangling(), 0))
            },

            // SAFETY: `new_size` is non-zero. Other conditions must be upheld by the caller
            new_size if old_layout.align() == new_layout.align() => unsafe {
                // `realloc` probably checks for `new_size <= old_layout.size()` or something similar.
                intrinsics::assume(new_size <= old_layout.size());

                let raw_ptr = rros_realloc(ptr.as_ptr(), old_layout, new_size);
                let ptr = NonNull::new(raw_ptr).ok_or(AllocError)?;
                Ok(NonNull::slice_from_raw_parts(ptr, new_size))
            },

            // SAFETY: because `new_size` must be smaller than or equal to `old_layout.size()`,
            // both the old and new memory allocation are valid for reads and writes for `new_size`
            // bytes. Also, because the old allocation wasn't yet deallocated, it cannot overlap
            // `new_ptr`. Thus, the call to `copy_nonoverlapping` is safe. The safety contract
            // for `dealloc` must be upheld by the caller.
            new_size => unsafe {
                let new_ptr = self.allocate(new_layout)?;
                ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_mut_ptr(), new_size);
                self.deallocate(ptr, old_layout);
                Ok(new_ptr)
            },
        }
    }
}