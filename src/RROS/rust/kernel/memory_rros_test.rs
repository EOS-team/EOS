// SPDX-License-Identifier: GPL-2.0

//! Rros Memory Allocator Tests. Test the correctness of memory allocation.

use crate::memory_rros::*;
use crate::random;
use crate::{double_linked_list::*, prelude::*};

/// Function `main_memory_rros_test` is the main function for testing memory operations.
/// It first prints a message indicating the start of the test.
/// It then calls `small_chunk_alloc_test` to test small chunk allocation.
pub fn main_memory_rros_test() {
    pr_debug!("main_memory_rros_test: begin");
    // get_random_test();
    small_chunk_alloc_test();
}

/// Function `test_addr_valid` checks if an address is valid.
/// It takes an address and a size as parameters and returns a boolean.
/// It currently always returns false.
#[allow(dead_code)]
fn test_addr_valid(_addr: *mut u8, _size: SizeT) -> bool {
    return false;
}

/// Function `get_random` generates a random number in the range [start, end).
/// It takes a start and an end as parameters and returns a u32.
/// It first generates a random number using the `getrandom` function.
/// If the generation is successful, it calculates the final number as the start plus the random number modulo the difference between the end and the start.
/// If the generation fails, it prints an error message and returns 0.
//[1,16)
fn get_random(start: u32, end: u32) -> u32 {
    let mut t: [u8; 4] = [1, 2, 3, 4];
    let res = random::getrandom(&mut t);
    let num;
    match res {
        Ok(_) => {
            let ptr: *const u8 = t.as_ptr();
            let ptr: *const u32 = ptr as *const u32;
            num = unsafe { *ptr };
        }
        Err(_) => {
            pr_err!("get_random err");
            return 0;
        }
    }
    return start + num % (end - start);
}

/// Function `get_random_test` tests the `get_random` function.
/// It generates and prints 20 random numbers in the range [1, 100).
#[allow(dead_code)]
fn get_random_test() {
    for _i in 1..20 {
        let num = get_random(1, 100);
        pr_debug!("num is {}", num);
    }
}

struct Pair {
    addr: *mut u8,
    size: u32,
}
impl Pair {
    pub fn new(addr: *mut u8, size: u32) -> Self {
        Pair { addr, size }
    }
}

#[allow(dead_code)]
// Calculate the number of pg linked lists in the current buckets.
fn calcuate_buckets() {
    unsafe {
        for i in 0..5 {
            let mut sum = 0;
            let pg = RROS_SYSTEM_HEAP.buckets[i];
            if pg != u32::MAX {
                pr_debug!("calcuate_buckets: in");
                let mut page = RROS_SYSTEM_HEAP.get_pagemap(pg as i32);
                loop {
                    let x = 32 >> i;
                    let pgmap = (*page).pginfo.map;
                    for j in 0..x {
                        if pgmap & 1 << j != 0x0 {
                            sum += 1;
                        }
                    }
                    pr_debug!("calcuate_buckets: i is {}, sum is {}", i, sum);
                    if (*page).next == pg {
                        break;
                    }
                    page = RROS_SYSTEM_HEAP.get_pagemap((*page).next as i32);
                }
            }
            // pr_debug!("calcuate_buckets: i is {}, sum is {}",i,sum);
        }
    }
}

// Enter the allocation range for continuous allocation testing.
fn mem_alloc_range(start: u32, end: u32, repeat: u32) {
    pr_debug!("mem_alloc_range: begin");
    let base = Pair::new(1 as *mut u8, 0);
    let mut link_head = List::new(base);
    let mut sum = 0;
    unsafe {
        pr_debug!(
            "mem_alloc_range: heap size:{}, heap used:{}",
            RROS_SYSTEM_HEAP.usable_size,
            RROS_SYSTEM_HEAP.used_size
        );
    }
    // alloc
    for i in 0..repeat {
        let num = get_random(start, end);
        sum += num;
        let x = unsafe { RROS_SYSTEM_HEAP.rros_alloc_chunk(num as usize) };
        match x {
            Some(a) => {
                let p = Pair::new(a, num);
                link_head.add_tail(p);
            }
            None => {
                pr_warn!("mem_alloc_range: rros_alloc_chunk err");
                // return ;
            }
        }
        pr_debug!("i is {}, num is {}", i, num);
    }
    pr_debug!("mem_alloc_range: has alloced: {}", sum);
    unsafe {
        pr_debug!(
            "mem_alloc_range: heap size:{}, heap used:{}",
            RROS_SYSTEM_HEAP.usable_size,
            RROS_SYSTEM_HEAP.used_size
        );
    }
    // calcuate_buckets();
    // recycle
    let length = link_head.len() + 1;
    for i in 1..length {
        let x = link_head.get_by_index(i).unwrap().value.addr;
        let y = link_head.get_by_index(i).unwrap().value.size;
        pr_debug!("i is {}, mem_alloc_range: size is {:?}", i, y);
        unsafe {
            RROS_SYSTEM_HEAP.rros_free_chunk(x);
        }
    }
    unsafe {
        pr_debug!(
            "mem_alloc_range: heap size:{}, heap used:{}",
            RROS_SYSTEM_HEAP.usable_size,
            RROS_SYSTEM_HEAP.used_size
        );
    }
    unsafe {
        for i in 0..5 {
            let x = RROS_SYSTEM_HEAP.buckets[i];
            if x != u32::MAX {
                pr_debug!("mem_alloc_range: *i != u32::MAX, i is {}, x = {}", i, x);
                let page = RROS_SYSTEM_HEAP.get_pagemap(x as i32);
                pr_debug!("page map is {}", (*page).pginfo.map);
            }
        }
    }
    pr_debug!("mem_alloc_range: end");
}

// Random allocation and recycling.
fn random_mem_alloc_range(start: u32, end: u32, repeat: u32) {
    pr_debug!("random_mem_alloc_range: begin");
    let base = Pair::new(1 as *mut u8, 0);
    let mut link_head = List::new(base);
    unsafe {
        pr_debug!(
            "random_mem_alloc_range: heap size:{}, heap used:{}",
            RROS_SYSTEM_HEAP.usable_size,
            RROS_SYSTEM_HEAP.used_size
        );
    }
    // allocate
    for i in 1..repeat {
        let r = get_random(0, 2);
        if r == 0 {
            // 0 means allocation
            let num = get_random(start, end);
            let x = unsafe { RROS_SYSTEM_HEAP.rros_alloc_chunk(num as usize) };
            match x {
                Some(a) => {
                    let p = Pair::new(a, num);
                    link_head.add_tail(p);
                }
                None => {
                    pr_warn!("random_mem_alloc_range: rros_alloc_chunk err");
                    // return ;
                }
            }
            pr_debug!(
                "random_mem_alloc_range: alloc chunk i is {}, num is {}",
                i,
                num
            );
        } else {
            // 1 means recycling
            let length = link_head.len();
            if length > 0 {
                let x = link_head.get_by_index(1).unwrap().value.addr;
                pr_debug!("random_mem_alloc_range: free chunk addr is {:?}", x);
                unsafe {
                    RROS_SYSTEM_HEAP.rros_free_chunk(x);
                }
                link_head.de_head();
            }
        }
    }
    unsafe {
        pr_debug!(
            "random_mem_alloc_range: heap size:{}, heap used:{}",
            RROS_SYSTEM_HEAP.usable_size,
            RROS_SYSTEM_HEAP.used_size
        );
    }

    // recycle
    let length = link_head.len() + 1;
    for i in 1..length {
        let x = link_head.get_by_index(i).unwrap().value.addr;
        pr_debug!(
            "random_mem_alloc_range: free chunk i is {}, addr is {:?}",
            i,
            x
        );
        unsafe {
            RROS_SYSTEM_HEAP.rros_free_chunk(x);
        }
    }
    unsafe {
        pr_debug!(
            "random_mem_alloc_range: heap size:{}, heap used:{}",
            RROS_SYSTEM_HEAP.usable_size,
            RROS_SYSTEM_HEAP.used_size
        );
    }
    unsafe {
        for i in RROS_SYSTEM_HEAP.buckets.iter_mut() {
            if *i != u32::MAX {
                pr_debug!("random_mem_alloc_range: *i != u32::MAX, *i = {}", *i);
            }
        }
    }
    pr_debug!("random_mem_alloc_range: end");
}

fn small_chunk_alloc_test() {
    mem_alloc_range(1, 257, 100); // allocate small memory continuously
    mem_alloc_range(257, 2048, 100); // allocate large memory continuously
    random_mem_alloc_range(1, 2049, 100); // randomly allocate memory
}
