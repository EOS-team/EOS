// SPDX-License-Identifier: GPL-2.0

//! mm
//!
//! C header: [`include/linux/mm.h`](../../../../include/linux/mm.h)

use crate::{
    bindings, c_types,
    error::{Error, Result},
};

extern "C" {
    // #[allow(improper_ctypes)]
    fn rust_helper_page_align(size: c_types::c_size_t) -> c_types::c_ulong;
    fn rust_helper_page_aligned(size: c_types::c_size_t) -> c_types::c_int;
}

/// Function `page_align` aligns a size to the page size.
/// It calls `rust_helper_page_align` to perform the alignment.
/// If the alignment is successful, it returns the aligned size.
/// If the alignment fails, it returns an error.
pub fn page_align(size: usize) -> Result<usize> {
    let res = unsafe { rust_helper_page_align(size) };
    if res != 0 {
        return Ok(res as usize);
    }
    Err(Error::EINVAL)
}

/// Function `page_aligned` checks if a size is page aligned.
/// It calls `rust_helper_page_aligned` to perform the check.
/// If the size is page aligned, it returns 0.
/// If the size is not page aligned, it returns an error.
pub fn page_aligned(size: usize) -> Result<usize> {
    let res = unsafe { rust_helper_page_aligned(size) };
    if res == 1 {
        return Ok(0);
    }
    Err(Error::EINVAL)
}

/// Maps a range of physical pages into the virtual address space of a process.
///
/// # Parameters
///
/// * `vma`: A pointer to a vm_area_struct structure that describes a virtual memory area.
/// * `vaddr`: The starting address in the user space where the memory should be mapped.
/// * `pfn`: The page frame number of the physical memory page to be mapped.
/// * `size`: The size of the memory to be mapped.
/// * `prot`: The protection attributes for the mapped memory.
///
/// # Returns
///
/// Returns 0 on success, and a negative error number on failure.
///
/// # Safety
///
/// This function is unsafe because it performs raw pointer operations and can lead to undefined behavior if not used correctly.
pub fn remap_pfn_range(
    vma: *mut bindings::vm_area_struct,
    vaddr: c_types::c_ulong,
    pfn: c_types::c_ulong,
    size: c_types::c_ulong,
    prot: bindings::pgprot_t,
) -> c_types::c_int {
    unsafe { bindings::remap_pfn_range(vma, vaddr, pfn, size, prot) }
}

/// Constant representing shared page protection attributes.
type PgprotT = bindings::pgprot_t;

/// This constant is used to set the protection attributes of a page to shared.
pub const PAGE_SHARED: PgprotT = PgprotT {
    // HACK: temporary hack for pgprot_t
    pgprot: 0x68000000000fc3 as u64,
};
