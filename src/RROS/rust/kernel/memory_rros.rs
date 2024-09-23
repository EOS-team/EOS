// SPDX-License-Identifier: GPL-2.0

//! Rros Memory.
use crate::timekeeping::*;
use crate::{bindings, Result};
use crate::{c_types, mm, prelude::*, premmpt, spinlock_init, sync::SpinLock, vmalloc};
use core::{mem::size_of, ptr::addr_of_mut};

const PAGE_SIZE: u32 = 4096 as u32;
const RROS_HEAP_PAGE_SHIFT: u32 = 9; /* 2^9 => 512 bytes */
const RROS_HEAP_PAGE_SIZE: u32 = 1 << RROS_HEAP_PAGE_SHIFT;
const RROS_HEAP_PAGE_MASK: u32 = !(RROS_HEAP_PAGE_SIZE - 1);
const RROS_HEAP_MIN_LOG2: u32 = 4; /* 16 bytes */
/*
 * Use bucketed memory for sizes between 2^RROS_HEAP_MIN_LOG2 and
 * 2^(RROS_HEAP_PAGE_SHIFT-1).
 */
const RROS_HEAP_MAX_BUCKETS: u32 = RROS_HEAP_PAGE_SHIFT - RROS_HEAP_MIN_LOG2;
const RROS_HEAP_MIN_ALIGN: u32 = 1 << RROS_HEAP_MIN_LOG2; //16
                                                          /* Maximum size of a heap (4Gb - PAGE_SIZE). */
const RROS_HEAP_MAX_HEAPSZ: u32 = 4294967295 - PAGE_SIZE + 1;
/* Bits we need for encoding a page # */
#[allow(dead_code)]
const RROS_HEAP_PGENT_BITS: u32 = 32 - RROS_HEAP_PAGE_SHIFT;
/* Each page is represented by a page map entry. */
// const RROS_HEAP_PGMAP_BYTES	sizeof(struct RrosHeapPgentry)
const CONFIG_RROS_NR_THREADS: usize = 256;
const CONFIG_RROS_NR_MONITORS: usize = 512;

/// `SizeT`: This type represents a size in bytes.
pub type SizeT = usize;

extern "C" {
    fn rust_helper_rb_link_node(
        node: *mut bindings::rb_node,
        parent: *const bindings::rb_node,
        rb_link: *mut *mut bindings::rb_node,
    );
    fn rust_helper_ilog2(size: SizeT) -> c_types::c_int;
    fn rust_helper_align(x: SizeT, a: u32) -> c_types::c_ulong;
    fn rust_helper_ffs(x: u32) -> c_types::c_int;
}

#[no_mangle]
/// `__rros_sys_heap_alloc`: This function allocates a chunk of memory from the system heap.
pub fn __rros_sys_heap_alloc(size: usize, _align: usize) -> *mut u8 {
    pr_debug!("__rros_sys_heap_alloc: begin");
    unsafe { RROS_SYSTEM_HEAP.rros_alloc_chunk(size).unwrap() }
}

/// `__rros_sys_heap_dealloc`: This function deallocates a chunk of memory from the system heap.
#[no_mangle]
pub fn __rros_sys_heap_dealloc(ptr: *mut u8, _size: usize, _align: usize) {
    unsafe {
        RROS_SYSTEM_HEAP.rros_free_chunk(ptr);
    }
}

/// `__rros_sys_heap_realloc`: This function reallocates a chunk of memory from the system heap.
#[no_mangle]
pub fn __rros_sys_heap_realloc(
    ptr: *mut u8,
    old_size: usize,
    _align: usize,
    new_size: usize,
) -> *mut u8 {
    unsafe {
        RROS_SYSTEM_HEAP
            .rros_realloc_chunk(ptr, old_size, new_size)
            .unwrap()
    }
}

/// `__rros_sys_heap_alloc_zerod`: This function allocates a zero-initialized chunk of memory from the system heap.
#[no_mangle]
pub fn __rros_sys_heap_alloc_zerod(size: usize, _align: usize) -> *mut u8 {
    unsafe { RROS_SYSTEM_HEAP.rros_alloc_chunk_zeroed(size).unwrap() }
}

/// `RrosUserWindow`: This struct represents a user window in the RROS system.
#[allow(dead_code)]
struct RrosUserWindow {
    state: u32,
    info: u32,
    pp_pending: u32,
}

/// Union `pginfo` is a representation of either a map or a block size.
/// Both are represented as 32-bit unsigned integers.
#[repr(C)]
pub union pginfo {
    /// `map` is a 32-bit unsigned integer representing a map.
    pub map: u32,
    /// `bsize` is a 32-bit unsigned integer representing a block size.
    pub bsize: u32,
}

/// Struct `RrosHeapPgentry` represents an entry in the RrosHeap.
/// It contains a previous and next pointer, a page type, and a `pginfo` union.
pub struct RrosHeapPgentry {
    /// `prev` is a 32-bit unsigned integer representing the previous pointer.
    pub prev: u32,
    /// `next` is a 32-bit unsigned integer representing the next pointer.
    pub next: u32,
    /// `page_type` is a 32-bit unsigned integer representing the page type.
    pub page_type: u32,
    /// `pginfo` is a union representing either a map or a block size.
    pub pginfo: pginfo,
}

/// Struct `RrosHeapRange` represents a range in the RrosHeap.
/// It contains an address node, a size node, and a size.
pub struct RrosHeapRange {
    /// `addr_node` is a node representing the address.
    pub addr_node: bindings::rb_node,
    /// `size_node` is a node representing the size.
    pub size_node: bindings::rb_node,
    /// `size` is a SizeT representing the size.
    pub size: SizeT,
}

/// Function `new_rros_heap_range` creates a new `RrosHeapRange`.
/// It takes an address and a size as parameters, and returns a pointer to the new `RrosHeapRange`.
pub fn new_rros_heap_range(addr: *mut u8, size: SizeT) -> *mut RrosHeapRange {
    let addr = addr as *mut RrosHeapRange;
    unsafe {
        (*addr).addr_node = bindings::rb_node::default();
        (*addr).size_node = bindings::rb_node::default();
        (*addr).size = size;
    }
    return addr;
}

/// Function `addr_add_size` adds a size to an address.
/// It takes an address and a size as parameters, and returns a new address.
#[inline]
pub fn addr_add_size(addr: *mut u8, size: SizeT) -> *mut u8 {
    (addr as u64 + size as u64) as *mut u8
}

// impl RrosHeapRange {
//     pub fn new(size: SizeT) -> Self {
//         Self {
//             addr_node: bindings::rb_node::default(),
//             size_node: bindings::rb_node::default(),
//             size: size,
//         }
//     }

//     pub fn into_node(&mut self) {
//         unsafe { addr_of_mut!(self.addr_node).write(bindings::rb_node::default()) };
//         // SAFETY: `node_ptr` is valid, and so are its fields.
//         unsafe { addr_of_mut!(self.size_node).write(bindings::rb_node::default()) };
//         // SAFETY: `node_ptr` is valid, and so are its fields.
//         unsafe { addr_of_mut!(self.size).write(self.size) };
//     }
// }

/// Struct `RrosHeap` represents the RrosHeap.
/// It contains a memory base, an address tree, a size tree, a page map, usable size, used size, and buckets.
pub struct RrosHeap {
    /// `membase` is a pointer to the base of the memory.
    pub membase: *mut u8,
    /// `addr_tree` is an optional root of a red-black tree used for finding sizes based on addresses.
    pub addr_tree: Option<bindings::rb_root>,
    /// `size_tree` is an optional root of a red-black tree used for finding addresses based on sizes.
    pub size_tree: Option<bindings::rb_root>,
    /// `pagemap` is an optional pointer to a page entry in the RrosHeap.
    pub pagemap: Option<*mut RrosHeapPgentry>,
    /// `usable_size` is a SizeT representing the usable size of the RrosHeap.
    pub usable_size: SizeT,
    /// `used_size` is a SizeT representing the used size of the RrosHeap.
    pub used_size: SizeT,
    /// `buckets` is an array of 32-bit unsigned integers representing the buckets in the RrosHeap.
    pub buckets: [u32; RROS_HEAP_MAX_BUCKETS as usize],
    /// `lock` is an optional SpinLock used for ensuring thread-safety in the `RrosHeap`.
    /// It is initialized in the `init` method of the `RrosHeap` struct.
    pub lock: Option<SpinLock<i32>>,
}

/// Implementation of the `RrosHeap` struct.
impl RrosHeap {
    /// Method `init` initializes the `RrosHeap`.
    /// It takes a memory base and a size as parameters, and returns a `Result<usize>`.
    /// It checks if the current context is in-band, if the size is page-aligned, and if the size is not greater than `RROS_HEAP_MAX_HEAPSZ`.
    /// It initializes a spinlock, sets all buckets to `u32::MAX`, allocates memory for the page map, and initializes the size and address trees.
    /// It also sets the memory base, usable size, and used size.
    pub fn init(&mut self, membase: *mut u8, size: SizeT) -> Result<usize> {
        premmpt::running_inband()?;
        mm::page_aligned(size)?;
        if (size as u32) > RROS_HEAP_MAX_HEAPSZ {
            return Err(crate::Error::EINVAL);
        }

        let mut spinlock = unsafe { SpinLock::new(1) };
        let pinned = unsafe { Pin::new_unchecked(&mut spinlock) };
        spinlock_init!(pinned, "spinlock");
        self.lock = Some(spinlock);

        for i in self.buckets.iter_mut() {
            *i = u32::MAX;
        }

        let nrpages = size >> RROS_HEAP_PAGE_SHIFT;
        let a: u64 = size_of::<RrosHeapPgentry>() as u64;
        let kzalloc_res = vmalloc::c_kzalloc(a * nrpages as u64);
        match kzalloc_res {
            Some(x) => self.pagemap = Some(x as *mut RrosHeapPgentry),
            None => {
                return Err(crate::Error::ENOMEM);
            }
        }

        self.membase = membase;
        self.usable_size = size;
        self.used_size = 0;

        self.size_tree = Some(bindings::rb_root::default());
        self.addr_tree = Some(bindings::rb_root::default());
        self.release_page_range(membase, size);

        Ok(0)
    }

    /// Method `release_page_range` releases a range of pages in the `RrosHeap`.
    /// It takes a page and a size as parameters.
    /// It first creates a `RrosHeapRange` from the page and sets its size.
    pub fn release_page_range(&mut self, page: *mut u8, size: SizeT) {
        // pr_debug!("release_page_range: 1");
        let mut freed = page as *mut RrosHeapRange;
        let mut addr_linked = false;

        unsafe {
            (*freed).size = size;
        }
        // pr_debug!("release_page_range: 2");
        let left_op = self.search_left_mergeable(freed);
        match left_op {
            Some(left) => {
                let node_links = unsafe { addr_of_mut!((*left).size_node) };
                let root = self.size_tree.as_mut().unwrap();
                unsafe {
                    bindings::rb_erase(node_links, root);
                }
                unsafe {
                    (*left).size += (*freed).size;
                }
                freed = left;
                addr_linked = true;
            }
            None => (),
        }
        // pr_debug!("release_page_range: 3");
        let right_op = self.search_right_mergeable(freed);
        match right_op {
            Some(right) => {
                let mut node_links = unsafe { addr_of_mut!((*right).size_node) };
                let mut root = self.size_tree.as_mut().unwrap();
                unsafe {
                    bindings::rb_erase(node_links, root);
                }
                unsafe {
                    (*freed).size += (*right).size;
                }
                node_links = unsafe { addr_of_mut!((*right).addr_node) };
                root = self.addr_tree.as_mut().unwrap();
                if addr_linked {
                    unsafe { bindings::rb_erase(node_links, root) };
                } else {
                    let freed_node_links = unsafe { addr_of_mut!((*freed).addr_node) };
                    unsafe { bindings::rb_replace_node(node_links, freed_node_links, root) };
                }
            }
            None => {
                if !addr_linked {
                    self.insert_range_byaddr(freed);
                }
            }
        }
        // pr_debug!("release_page_range: 4");
        self.insert_range_bysize(freed);
        // pr_debug!("release_page_range: 5");
    }

    /// Method `search_left_mergeable` searches for a range in the `RrosHeap` that can be merged with the given range on the left.
    /// It takes a pointer to a `RrosHeapRange` as a parameter and returns an `Option<*mut RrosHeapRange>`.
    /// It iterates over the address tree and checks if the end address of the current node equals the start address of the given range.
    /// If it does, it returns a pointer to the current node. If it doesn't, it moves to the left or right child node based on the comparison of their addresses.
    pub fn search_left_mergeable(&self, r: *mut RrosHeapRange) -> Option<*mut RrosHeapRange> {
        let mut node: *mut bindings::rb_node = self.addr_tree.clone().unwrap().rb_node;
        while !node.is_null() {
            let p = crate::container_of!(node, RrosHeapRange, addr_node);
            unsafe {
                if addr_add_size(p as *mut u8, (*p).size) as u64 == r as u64 {
                    return Some(p as *mut RrosHeapRange);
                }
                let addr_node_addr = addr_of_mut!((*r).addr_node);
                if (addr_node_addr as u64) < (node as u64) {
                    node = (*node).rb_left;
                } else {
                    node = (*node).rb_right;
                }
            }
        }
        None
    }

    /// Method `search_right_mergeable` searches for a range in the `RrosHeap` that can be merged with the given range on the right.
    pub fn search_right_mergeable(&self, r: *mut RrosHeapRange) -> Option<*mut RrosHeapRange> {
        let mut node: *mut bindings::rb_node = self.addr_tree.clone().unwrap().rb_node;
        while !node.is_null() {
            let p = crate::container_of!(node, RrosHeapRange, addr_node);
            unsafe {
                if addr_add_size(r as *mut u8, (*r).size) as u64 == p as u64 {
                    return Some(p as *mut RrosHeapRange);
                }
                let addr_node_addr = addr_of_mut!((*r).addr_node);
                if (addr_node_addr as u64) < (node as u64) {
                    node = (*node).rb_left;
                } else {
                    node = (*node).rb_right;
                }
            }
        }
        None
    }

    /// Method `insert_range_byaddr` inserts a range into the `RrosHeap` by address.
    /// It takes a pointer to a `RrosHeapRange` as a parameter.
    /// It first gets the address of the address node of the given range and the root of the address tree.
    /// Then it iterates over the address tree and moves to the left or right child node based on the comparison of their addresses.
    /// When it finds a null node, it inserts the given range at that position and re-balances the tree.
    pub fn insert_range_byaddr(&mut self, r: *mut RrosHeapRange) {
        unsafe {
            let node_links = addr_of_mut!((*r).addr_node);
            let root = self.addr_tree.as_mut().unwrap();
            let mut new_link: &mut *mut bindings::rb_node = &mut root.rb_node;
            let mut parent = core::ptr::null_mut();
            while !new_link.is_null() {
                let p = crate::container_of!(*new_link, RrosHeapRange, addr_node);
                parent = *new_link;
                if (r as u64) < (p as u64) {
                    new_link = &mut (*parent).rb_left;
                } else {
                    new_link = &mut (*parent).rb_right;
                }
            }
            rust_helper_rb_link_node(node_links, parent, new_link);
            bindings::rb_insert_color(node_links, root);
        }
    }

    /// Method `insert_range_bysize` inserts a range into the `RrosHeap` by size.
    /// It takes a pointer to a `RrosHeapRange` as a parameter.
    /// The implementation is not shown in the provided code.
    pub fn insert_range_bysize(&mut self, r: *mut RrosHeapRange) {
        unsafe {
            let node_links = addr_of_mut!((*r).size_node);
            let root = self.size_tree.as_mut().unwrap();
            let mut new_link: &mut *mut bindings::rb_node = &mut root.rb_node;
            let mut parent = core::ptr::null_mut();
            while !new_link.is_null() {
                let p = crate::container_of!(*new_link, RrosHeapRange, size_node);
                parent = *new_link;
                if (r as u64) < (p as u64) {
                    new_link = &mut (*parent).rb_left;
                } else {
                    new_link = &mut (*parent).rb_right;
                }
            }
            rust_helper_rb_link_node(node_links, parent, new_link);
            bindings::rb_insert_color(node_links, root);
        }
    }
    /// Method `rros_alloc_chunk` allocates a chunk of memory from the `RrosHeap`.
    /// It takes a size as a parameter and returns an `Option<*mut u8>`.
    /// It first checks if the size is zero. If it is, it returns `None`.
    /// If the size is less than `RROS_HEAP_MIN_ALIGN`, it sets the block size to `RROS_HEAP_MIN_ALIGN` and `log2size` to `RROS_HEAP_MIN_LOG2`.
    /// Otherwise, it calculates `log2size` as the base-2 logarithm of the size.
    /// If the size is not a power of 2, it increments `log2size` by 1.
    /// The implementation for allocating the memory block is not shown in the provided code.
    #[no_mangle]
    pub fn rros_alloc_chunk(&mut self, size: SizeT) -> Option<*mut u8> {
        pr_debug!(
            "rros_alloc_chunk: time1 is {} size is {}",
            ktime_get_real_fast_ns(),
            size
        );
        pr_debug!("rros_alloc_chunk: begin");
        pr_debug!("rros_alloc_chunk: alloc size is {}", size);
        let mut log2size: i32;
        let ilog: i32;
        let pg: i32;
        let b: i32;
        let mut _flags: u32;
        let bsize: SizeT;

        let block: Option<*mut u8>;
        if size == 0 {
            return None;
        }

        if size < (RROS_HEAP_MIN_ALIGN as SizeT) {
            bsize = RROS_HEAP_MIN_ALIGN as SizeT;
            log2size = RROS_HEAP_MIN_LOG2 as i32;
        } else {
            log2size = unsafe { rust_helper_ilog2(size) }; //down int size
            pr_debug!(
                "rros_alloc_chunk: time1.1.0 is {}",
                ktime_get_real_fast_ns()
            );
            if log2size < (RROS_HEAP_PAGE_SHIFT as i32) {
                //9 2^4-2^8
                pr_debug!(
                    "rros_alloc_chunk: time1.1.1 is {}",
                    ktime_get_real_fast_ns()
                );
                if size & (size - 1) != 0 {
                    pr_debug!(
                        "rros_alloc_chunk: time1.1.2 is {}",
                        ktime_get_real_fast_ns()
                    );
                    log2size += 1;
                }
                pr_debug!(
                    "rros_alloc_chunk: time1.1.3 is {}",
                    ktime_get_real_fast_ns()
                );
                bsize = 1 << log2size;
                pr_debug!(
                    "rros_alloc_chunk: time1.2.0 is {}",
                    ktime_get_real_fast_ns()
                );
            } else {
                bsize = unsafe { rust_helper_align(size, RROS_HEAP_PAGE_SIZE) as SizeT }; //512 up to a int times to 512
                pr_debug!("rros_alloc_chunk: time1.3 is {}", ktime_get_real_fast_ns());
            }
        }
        pr_debug!("rros_alloc_chunk: time2 is {}", ktime_get_real_fast_ns());
        // Lock.
        if bsize >= (RROS_HEAP_PAGE_SIZE as usize) {
            block = self.add_free_range(bsize, 0);
            pr_debug!("rros_alloc_chunk: time2.1 is {}", ktime_get_real_fast_ns());
        } else {
            ilog = log2size - RROS_HEAP_MIN_LOG2 as i32;
            pr_debug!("rros_alloc_chunk: ilog is {}", ilog);
            pg = self.buckets[ilog as usize] as i32;
            pr_debug!("rros_alloc_chunk: pg is {}", pg);
            unsafe {
                if pg < 0 {
                    block = self.add_free_range(bsize, log2size);
                    pr_debug!("rros_alloc_chunk: block is {:p}", block.clone().unwrap());
                    pr_debug!("rros_alloc_chunk: time2.2 is {}", ktime_get_real_fast_ns());
                } else {
                    let pagemap = self.get_pagemap(pg);
                    if (*pagemap).pginfo.map == u32::MAX {
                        block = self.add_free_range(bsize, log2size);
                        pr_debug!("rros_alloc_chunk: time2.3 is {}", ktime_get_real_fast_ns());
                    } else {
                        let x = (*pagemap).pginfo.map;
                        pr_debug!("rros_alloc_chunk: x is {}", x);
                        b = rust_helper_ffs(!x) - 1;
                        pr_debug!("rros_alloc_chunk: b is {}", b);
                        pr_debug!("rros_alloc_chunk: time3 is {}", ktime_get_real_fast_ns());
                        (*pagemap).pginfo.map |= 1 << b;
                        let t1 = ktime_get_real_fast_ns();
                        pr_debug!("rros_alloc_chunk: time3.1 is {}", ktime_get_real_fast_ns());
                        self.used_size += bsize;
                        let t2 = ktime_get_real_fast_ns();
                        pr_debug!("rros_alloc_chunk: time3.1 is {}", t2 - t1);
                        block = Some(addr_add_size(
                            self.membase,
                            ((pg << RROS_HEAP_PAGE_SHIFT) + (b << log2size)) as SizeT,
                        ));
                        pr_debug!("rros_alloc_chunk: time3.3 is {}", ktime_get_real_fast_ns());
                        if (*pagemap).pginfo.map == u32::MAX {
                            pr_debug!("rros_alloc_chunk: time3.12 is {}", ktime_get_real_fast_ns());
                            self.move_page_back(pg, log2size);
                        }
                    }
                }
            }
            pr_debug!("rros_alloc_chunk: time4 is {}", ktime_get_real_fast_ns());
        }
        // Unlock.
        pr_debug!("rros_alloc_chunk: time5 is {}", ktime_get_real_fast_ns());
        return block;
    }

    /// Method `rros_alloc_chunk_zeroed` allocates a chunk of memory from the `RrosHeap` and initializes it to zero.
    /// It takes a size as a parameter and returns an `Option<*mut u8>`.
    /// It first calls `rros_alloc_chunk` to allocate a memory block.
    /// If the allocation is successful, it uses `memset` to set the memory block to zero and returns a pointer to the memory block.
    /// If the allocation is not successful, it returns `None`.
    pub fn rros_alloc_chunk_zeroed(&mut self, size: SizeT) -> Option<*mut u8> {
        let block = self.rros_alloc_chunk(size);
        match block {
            Some(x) => {
                unsafe { bindings::memset(x as *mut c_types::c_void, 0, size as c_types::c_ulong) };
                return Some(x);
            }
            None => return None,
        }
    }

    /// Method `rros_realloc_chunk` reallocates a chunk of memory in the `RrosHeap`.
    /// It takes a pointer to the old memory block, the old size, and the new size as parameters, and returns an `Option<*mut u8>`.
    /// It first calls `rros_alloc_chunk` to allocate a new memory block.
    /// If the allocation is successful, it uses `memcpy` to copy the old memory block to the new memory block, frees the old memory block, and returns a pointer to the new memory block.
    pub fn rros_realloc_chunk(
        &mut self,
        raw: *mut u8,
        old_size: SizeT,
        new_size: SizeT,
    ) -> Option<*mut u8> {
        let ptr_op = self.rros_alloc_chunk(new_size);
        match ptr_op {
            Some(ptr) => {
                unsafe {
                    bindings::memcpy(
                        ptr as *mut c_types::c_void,
                        raw as *mut c_types::c_void,
                        old_size as c_types::c_ulong,
                    )
                };
                self.rros_free_chunk(raw);
                return Some(ptr);
            }
            None => return None,
        }
    }

    /// Method `addr_to_pagenr` converts a memory address to a page number in the `RrosHeap`.
    /// It takes a pointer to a memory address as a parameter and returns an `i32`.
    /// It subtracts the base memory address from the given memory address, shifts the result right by `RROS_HEAP_PAGE_SHIFT`, and casts the result to an `i32`.
    #[inline]
    fn addr_to_pagenr(&mut self, p: *mut u8) -> i32 {
        ((p as u32 - self.membase as u32) >> RROS_HEAP_PAGE_SHIFT) as i32
    }

    /// Method `add_free_range` adds a free range to the `RrosHeap`.
    /// It takes a block size and a `log2size` as parameters and returns an `Option<*mut u8>`.
    /// It first reserves a page range with a size aligned to `RROS_HEAP_PAGE_SIZE`.
    /// If the reservation is successful, it gets the page map of the page and sets the page type and map or block size based on `log2size`.
    /// If `log2size` is not zero, it sets the page type to `log2size`, the map to the bitwise OR of the bitwise NOT of the block mask and 1, and adds the page to the front of the page list.
    fn add_free_range(&mut self, bsize: SizeT, log2size: i32) -> Option<*mut u8> {
        let pg_op = self
            .reserve_page_range(unsafe { rust_helper_align(bsize, RROS_HEAP_PAGE_SIZE) } as SizeT);
        let pg: i32;
        match pg_op {
            Some(x) => {
                if x < 0 {
                    return None;
                }
                pg = x;
            }
            None => return None,
        }

        let pagemap = self.get_pagemap(pg);
        if log2size != 0 {
            unsafe {
                (*pagemap).page_type = log2size as u32;
                (*pagemap).pginfo.map = !gen_block_mask(log2size) | 1;
                self.add_page_front(pg, log2size);
            }
        } else {
            unsafe {
                (*pagemap).page_type = 0x02;
                (*pagemap).pginfo.bsize = bsize as u32;
            }
        }

        self.used_size += bsize;
        return Some(self.pagenr_to_addr(pg));
    }

    /// Method `pagenr_to_addr` converts a page number to a memory address in the `RrosHeap`.
    /// It takes a page number as a parameter and returns a pointer to a memory address.
    /// It shifts the page number left by `RROS_HEAP_PAGE_SHIFT`, adds the result to the base memory address, and casts the result to a pointer to a `u8`.
    #[inline]
    fn pagenr_to_addr(&mut self, pg: i32) -> *mut u8 {
        addr_add_size(self.membase, (pg as SizeT) << RROS_HEAP_PAGE_SHIFT) as *mut u8
    }

    /// Method `search_size_ge` searches for a range in the `RrosHeap` with a size greater than or equal to the given size.
    /// It takes a size as a parameter and returns an `Option<*mut RrosHeapRange>`.
    /// It first gets the root of the size tree and initializes `deepest` to `null`.
    /// Then it iterates over the size tree and moves to the left or right child node based on the comparison of their sizes.
    /// If it finds a node with a size equal to the given size, it returns a pointer to the range of that node.
    /// If it doesn't find a node with a size equal to the given size, it sets `rb` to `deepest` and iterates over the size tree again.
    /// This time, if it finds a node with a size greater than or equal to the given size, it returns a pointer to the range of that node.
    fn search_size_ge(&mut self, size: SizeT) -> Option<*mut RrosHeapRange> {
        let mut rb = self.size_tree.as_mut().unwrap().rb_node;
        let mut deepest = core::ptr::null_mut();
        while !rb.is_null() {
            deepest = rb;
            unsafe {
                let r = crate::container_of!(rb, RrosHeapRange, size_node);
                if size < (*r).size {
                    rb = (*rb).rb_left;
                    continue;
                }
                if size > (*r).size {
                    rb = (*rb).rb_right;
                    continue;
                }
                return Some(r as *mut RrosHeapRange);
            }
        }
        rb = deepest;
        while !rb.is_null() {
            unsafe {
                let r = crate::container_of!(rb, RrosHeapRange, size_node);
                if size <= (*r).size {
                    return Some(r as *mut RrosHeapRange);
                }
                rb = bindings::rb_next(rb as *const bindings::rb_node);
            }
        }
        None
    }

    /// Method `reserve_page_range` reserves a page range in the `RrosHeap`.
    /// It takes a size as a parameter and returns an `Option<i32>`.
    /// It first calls `search_size_ge` to find a range with a size greater than or equal to the given size.
    /// If it finds such a range, it removes the range from the size tree.
    /// If the size of the range is equal to the given size, it also removes the range from the address tree and returns the page number of the range.
    /// If the size of the range is greater than the given size, it splits the range into two ranges, inserts the smaller range back into the size tree, and returns the page number of the larger range.
    fn reserve_page_range(&mut self, size: SizeT) -> Option<i32> {
        let new_op = self.search_size_ge(size);
        let mut new;
        match new_op {
            Some(x) => new = x,
            None => return None,
        }
        let mut node_links = unsafe { addr_of_mut!((*new).size_node) };
        let mut root = self.size_tree.as_mut().unwrap();
        unsafe { bindings::rb_erase(node_links, root) };

        if unsafe { (*new).size == size } {
            node_links = unsafe { addr_of_mut!((*new).addr_node) };
            root = self.addr_tree.as_mut().unwrap();
            unsafe { bindings::rb_erase(node_links, root) };
            return Some(self.addr_to_pagenr(new as *mut u8));
        }

        let mut splitr = new;
        unsafe { (*splitr).size -= size };
        new = unsafe { addr_add_size(new as *mut u8, (*splitr).size) as *mut RrosHeapRange };
        self.insert_range_bysize(splitr);
        return Some(self.addr_to_pagenr(new as *mut u8));
    }

    /// Method `move_page_back` moves a page back in the `RrosHeap`.
    /// It takes a page number and a `log2size` as parameters.
    /// It first gets the page map of the page.
    /// If the page is the next page in the page map, it returns.
    /// Otherwise, it removes the page from the page list.
    fn move_page_back(&mut self, pg: i32, log2size: i32) {
        pr_debug!("move_page_back: in");
        let old = self.get_pagemap(pg);
        if pg == unsafe { (*old).next as i32 } {
            pr_debug!("move_page_back: return");
            return;
        }

        self.remove_page(pg, log2size);

        let ilog = (log2size as u32) - RROS_HEAP_MIN_LOG2;
        let head = self.get_pagemap(self.buckets[ilog as usize] as i32);
        let last = self.get_pagemap(unsafe { (*head).prev as i32 });
        unsafe {
            (*old).prev = (*head).prev;
            (*old).next = (*last).next;
            let next = self.get_pagemap((*old).next as i32);
            (*next).prev = pg as u32;
            (*last).next = pg as u32;
        }
        pr_debug!("move_page_back: out");
    }

    /// Method `move_page_front` moves a page to the front of the `RrosHeap`.
    /// It takes a page number and a `log2size` as parameters.
    /// It first calculates `ilog` as the difference between `log2size` and `RROS_HEAP_MIN_LOG2`.
    /// If the page is already at the front of the bucket, it returns.
    /// Otherwise, it removes the page from the page list and adds it to the front of the page list.
    fn move_page_front(&mut self, pg: i32, log2size: i32) {
        let ilog = (log2size as u32) - RROS_HEAP_MIN_LOG2;

        if self.buckets[ilog as usize] == (pg as u32) {
            return;
        }

        self.remove_page(pg, log2size);
        self.add_page_front(pg, log2size);
    }

    /// Method `remove_page` removes a page from the `RrosHeap`.
    /// It takes a page number and a `log2size` as parameters.
    /// It first calculates `ilog` as the difference between `log2size` and `RROS_HEAP_MIN_LOG2`.
    /// It then gets the page map of the page.
    /// If the page is the next page in the page map, it sets the bucket to `u32::MAX`.
    /// Otherwise, if the page is the first page in the bucket, it sets the bucket to the next page.
    /// It then updates the previous and next pointers of the page map to remove the page from the page list.
    fn remove_page(&mut self, pg: i32, log2size: i32) {
        pr_debug!("remove_page: in");
        let ilog = ((log2size as u32) - RROS_HEAP_MIN_LOG2) as usize;
        let old = self.get_pagemap(pg);
        if pg == unsafe { (*old).next as i32 } {
            pr_debug!("remove_page: u32::MAX");
            self.buckets[ilog] = u32::MAX;
        } else {
            if pg == (self.buckets[ilog] as i32) {
                self.buckets[ilog] = unsafe { (*old).next };
            }
            unsafe {
                let prev = self.get_pagemap((*old).prev as i32);
                (*prev).next = (*old).next;
                let next = self.get_pagemap((*old).next as i32);
                (*next).prev = (*old).prev;
            }
        }
        pr_debug!("remove_page: out");
    }

    /// Method `add_page_front` adds a page to the front of the `RrosHeap`.
    /// It takes a page number and a `log2size` as parameters.
    /// It first calculates `ilog` as the difference between `log2size` and `RROS_HEAP_MIN_LOG2`.
    /// If the bucket is empty, it sets the bucket to the page number and sets the previous and next pointers of the page map to the page number.
    /// Otherwise, it gets the page map of the first page in the bucket, sets the previous pointer of the new page map to the first page, sets the next pointer of the new page map to the second page, sets the previous pointer of the second page to the new page, sets the next pointer of the first page to the new page, and sets the bucket to the new page.
    fn add_page_front(&mut self, pg: i32, log2size: i32) {
        let ilog = ((log2size as u32) - RROS_HEAP_MIN_LOG2) as usize;
        pr_debug!("add_page_front: ilog is {}", ilog);
        let new = self.get_pagemap(pg);
        if self.buckets[ilog] == u32::MAX {
            pr_debug!("add_page_front: if");
            self.buckets[ilog] = pg as u32;
            unsafe {
                (*new).prev = pg as u32;
                (*new).next = pg as u32;
            }
            pr_debug!("add_page_front: pg is {}", pg);
        } else {
            pr_debug!("add_page_front: else");
            let head = self.get_pagemap(self.buckets[ilog] as i32);
            unsafe {
                (*new).prev = self.buckets[ilog];
                (*new).next = (*head).next;
                let next = self.get_pagemap((*new).next as i32);
                (*next).prev = pg as u32;
                (*head).next = pg as u32;
                self.buckets[ilog] = pg as u32;
            }
        }
    }

    /// Method `get_pagemap` gets the page map of a page in the `RrosHeap`.
    /// It takes a page number as a parameter and returns a pointer to an `RrosHeapPgentry`.
    /// It calculates the address of the page map by adding the product of the page number and the size of an `RrosHeapPgentry` to the base address of the page map.
    #[inline]
    pub fn get_pagemap(&self, pg: i32) -> *mut RrosHeapPgentry {
        addr_add_size(
            self.pagemap.clone().unwrap() as *mut u8,
            ((pg as u32) * (size_of::<RrosHeapPgentry>() as u32)) as SizeT,
        ) as *mut RrosHeapPgentry
    }

    /// Method `rros_free_chunk` frees a chunk of memory in the `RrosHeap`.
    /// It takes a memory block as a parameter.
    /// It first calculates the page offset and the page number of the block.
    /// It then gets the page map of the page and the type of the page.
    /// If the type of the page is 0x02, it gets the block size from the page map, calculates the address of the block, and releases the page range.
    /// Otherwise, it calculates the block size as 2 to the power of the type of the page.
    /// It then calculates the block offset within the page.
    /// If the block offset is not aligned with the block size, it returns.
    /// It then calculates the block number within the page and gets the old map from the page map.
    /// It updates the map in the page map to mark the block as free.
    /// If the page is now empty, it removes the page from the page list.
    pub fn rros_free_chunk(&mut self, block: *mut u8) {
        let pgoff = (block as u32) - (self.membase as u32);
        let pg = (pgoff >> RROS_HEAP_PAGE_SHIFT) as i32;
        let pagemap = self.get_pagemap(pg);
        let page_type = unsafe { (*pagemap).page_type };
        let bsize: SizeT;
        if page_type == 0x02 {
            bsize = unsafe { (*pagemap).pginfo.bsize as usize };
            let addr = self.pagenr_to_addr(pg);
            self.release_page_range(addr, bsize);
        } else {
            let log2size = page_type as i32;
            bsize = 1 << log2size;
            let boff = pgoff & !RROS_HEAP_PAGE_MASK; // In-page offset
            if (boff & ((bsize - 1) as u32)) != 0 {
                return;
                //raw_spin_unlock_irqrestore(&heap->lock, flags);
            }
            let n = boff >> log2size;
            let oldmap = unsafe { (*pagemap).pginfo.map };
            unsafe { (*pagemap).pginfo.map &= !((1 as u32) << n) }; // Set the corresponding position of the original map to 0, indicating release.
            unsafe {
                pr_debug!(
                    "rros_free_chunk: pg is {}, log2size is {}, oldmap is {}, newmap is {}",
                    pg,
                    log2size,
                    oldmap,
                    (*pagemap).pginfo.map
                );
            }
            if unsafe { (*pagemap).pginfo.map == !gen_block_mask(log2size) } {
                // The page is empty after release.
                pr_debug!("rros_free_chunk: 1");
                self.remove_page(pg, log2size);
                pr_debug!("rros_free_chunk: 1.2");
                let addr = self.pagenr_to_addr(pg);
                pr_debug!("rros_free_chunk: 1.3");
                self.release_page_range(addr, RROS_HEAP_PAGE_SIZE as SizeT);
                pr_debug!("rros_free_chunk: 2");
            } else if oldmap == u32::MAX {
                pr_debug!("rros_free_chunk: 3");
                self.move_page_front(pg, log2size);
                pr_debug!("rros_free_chunk: 4");
            }
        }
        self.used_size -= bsize;
        //raw_spin_unlock_irqrestore(&heap->lock, flags);
    }

    /// Method `rros_destroy_heap` destroys the `RrosHeap`.
    /// It first checks if the current context is in-band.
    /// If it is not in-band, it prints a debug message.
    /// It then frees the memory allocated for the page map.
    pub fn rros_destroy_heap(&mut self) {
        let res = premmpt::running_inband();
        match res {
            Err(_) => {
                pr_err!("warning: rros_destroy_heap not inband");
            }
            Ok(_) => (),
        }
        vmalloc::c_kzfree(self.pagemap.clone().unwrap() as *const c_types::c_void);
    }
}

/// Function `cleanup_system_heap` cleans up the system heap.
/// It first destroys the system heap and then frees the memory allocated for the base of the system heap.
pub fn cleanup_system_heap() {
    unsafe {
        RROS_SYSTEM_HEAP.rros_destroy_heap();
        vmalloc::c_vfree(RROS_SYSTEM_HEAP.membase as *const c_types::c_void);
    }
}

/// Function `cleanup_shared_heap` cleans up the shared heap.
/// It first destroys the shared heap and then frees the memory allocated for the base of the shared heap.
pub fn cleanup_shared_heap() {
    unsafe {
        RROS_SYSTEM_HEAP.rros_destroy_heap();
        vmalloc::c_vfree(RROS_SYSTEM_HEAP.membase as *const c_types::c_void);
    }
}

/// Function `gen_block_mask` generates a block mask.
/// It takes a `log2size` as a parameter and returns a `u32`.
/// It calculates the block mask as the maximum `u32` value shifted right by the difference between 32 and the page size divided by 2 to the power of `log2size`.
pub fn gen_block_mask(log2size: i32) -> u32 {
    return u32::MAX >> (32 - (RROS_HEAP_PAGE_SIZE >> log2size));
}

/// Static variable `RROS_SYSTEM_HEAP` is an instance of `RrosHeap`.
pub static mut RROS_SYSTEM_HEAP: RrosHeap = RrosHeap {
    membase: 0 as *mut u8,
    addr_tree: None,
    size_tree: None,
    pagemap: None,
    usable_size: 0,
    used_size: 0,
    buckets: [0; RROS_HEAP_MAX_BUCKETS as usize],
    lock: None,
};

/// Static variable `RROS_SHARED_HEAP` is an instance of `RrosHeap`.
/// It is initialized with a null memory base, no address tree, no size tree, no page map, zero usable size, zero used size, an array of zeros for the buckets, and no lock.
pub static mut RROS_SHARED_HEAP: RrosHeap = RrosHeap {
    membase: 0 as *mut u8,
    addr_tree: None,
    size_tree: None,
    pagemap: None,
    usable_size: 0,
    used_size: 0,
    buckets: [0; RROS_HEAP_MAX_BUCKETS as usize],
    lock: None,
};

/// Static variable `RROS_SHM_SIZE` is the size of the shared memory.
/// It is initialized to zero.
pub static mut RROS_SHM_SIZE: usize = 0;

/// Function `init_system_heap` initializes the system heap.
/// It first allocates a block of memory of size 2048 * 1024 bytes.
/// If the allocation is successful, it initializes the system heap with the allocated memory.
/// If the initialization is successful, it prints a success message.
/// If the initialization fails, it frees the allocated memory and returns an error.
/// If the allocation fails, it returns an error.
pub fn init_system_heap() -> Result<usize> {
    let size = 2048 * 1024;
    let system = vmalloc::c_vmalloc(size as c_types::c_ulong);
    match system {
        Some(x) => {
            let ret = unsafe { RROS_SYSTEM_HEAP.init(x as *mut u8, size as usize) };
            match ret {
                Err(_) => {
                    vmalloc::c_vfree(x);
                    return Err(crate::Error::ENOMEM);
                }
                Ok(_) => (),
            }
        }
        None => return Err(crate::Error::ENOMEM),
    }
    pr_info!("rros_mem: init_system_heap success");
    Ok(0)
}
/// Function `init_shared_heap` initializes the shared heap.
/// It first calculates the size of the shared heap as the number of threads times the size of an `RrosUserWindow` plus the number of monitors times 40.
/// It then aligns the size to the page size.
/// It allocates a block of memory of the calculated size.
/// If the allocation is successful, it initializes the shared heap with the allocated memory.
/// If the initialization is successful, it sets the size of the shared memory to the calculated size.
/// If the initialization fails, it frees the allocated memory and returns an error.
/// If the allocation fails, it returns an error.
pub fn init_shared_heap() -> Result<usize> {
    let mut size: usize =
        CONFIG_RROS_NR_THREADS * size_of::<RrosUserWindow>() + CONFIG_RROS_NR_MONITORS * 40;
    size = mm::page_align(size)?;
    mm::page_aligned(size)?;
    let shared = vmalloc::c_kzalloc(size as u64);
    match shared {
        Some(x) => {
            let ret = unsafe { RROS_SHARED_HEAP.init(x as *mut u8, size as usize) };
            match ret {
                Err(_e) => {
                    vmalloc::c_kzfree(x);
                    return Err(_e);
                }
                Ok(_) => (),
            }
        }
        None => return Err(crate::Error::ENOMEM),
    }
    unsafe { RROS_SHM_SIZE = size };
    Ok(0)
}
/// Function `rros_init_memory` initializes the memory.
/// It first initializes the system heap.
/// If the initialization is successful, it initializes the shared heap.
/// If the initialization of the shared heap fails, it cleans up the system heap.
/// If the initialization of the system heap fails, it returns an error.
pub fn rros_init_memory() -> Result<usize> {
    let mut ret = init_system_heap();
    match ret {
        Err(_) => return ret,
        Ok(_) => (),
    }
    ret = init_shared_heap();
    match ret {
        Err(_) => {
            cleanup_system_heap();
            return ret;
        }
        Ok(_) => (),
    }
    Ok(0)
}

/// Function `rros_cleanup_memory` cleans up the memory.
/// It first cleans up the shared heap and then cleans up the system heap.
pub fn rros_cleanup_memory() {
    cleanup_shared_heap();
    cleanup_system_heap();
}
