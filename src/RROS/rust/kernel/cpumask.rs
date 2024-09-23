// SPDX-License-Identifier: GPL-2.0

//! cpumask
//!
//! C header: [`include/linux/cpumask.h`](../../../../include/linux/cpumask.h)

use crate::{
    bindings, c_types,
    error::{Error, Result},
};

use core::{iter::Iterator, ops::BitAnd};

extern "C" {
    fn rust_helper_num_possible_cpus() -> u32;
    fn rust_helper_cpulist_parse(
        buf: *const c_types::c_char,
        dstp: *mut bindings::cpumask,
    ) -> c_types::c_int;
    fn rust_helper_cpumask_copy(dstp: *mut bindings::cpumask, srcp: *const bindings::cpumask);
    fn rust_helper_cpumask_and(
        dstp: *mut bindings::cpumask,
        srcp1: *const bindings::cpumask,
        srcp2: *const bindings::cpumask,
    );
    fn rust_helper_cpumask_empty(srcp: *const bindings::cpumask) -> c_types::c_int;
    fn rust_helper_cpumask_first(srcp: *const bindings::cpumask) -> c_types::c_int;
    fn rust_helper_cpumask_set_cpu(cpu: u32, dstp: *mut bindings::cpumask);
    fn rust_helper_cpumask_clear(srcp: *mut bindings::cpumask);
    fn rust_helper_cpumask_test_cpu(cpu: u32, srcp: *const bindings::cpumask) -> c_types::c_int;
    fn rust_helper_cpumask_of(cpu: u32) -> *const bindings::cpumask;
    fn rust_helper_cpus_read_lock();
    fn rust_helper_cpus_read_unlock();
}

/// An possible CPU index iterator.
///
/// This iterator has a similar abilitiy to the kernel's macro `for_each_possible_cpu`.
pub struct PossibleCpusIndexIter {
    index: i32,
}

/// An online CPU index iterator.
///
/// This iterator has a similar abilitiy to the kernel's macro `for_each_online_cpu`.
pub struct OnlineCpusIndexIter {
    index: i32,
}

/// An present CPU index iterator.
///
/// This iterator has a similar abilitiy to the kernel's macro `for_each_present_cpu`.
pub struct PresentCpusIndexIter {
    index: i32,
}

impl Iterator for PossibleCpusIndexIter {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        let next_cpu_id =
            unsafe { bindings::cpumask_next(self.index, &bindings::__cpu_possible_mask) };
        // When [`bindings::cpumask_next`] can not find further CPUs set in the
        // [`bindings::__cpu_possible_mask`], it returns a value >= [`bindings::nr_cpu_ids`].
        if next_cpu_id >= unsafe { bindings::nr_cpu_ids } {
            return None;
        }
        self.index = next_cpu_id as i32;
        Some(next_cpu_id)
    }
}

impl Iterator for OnlineCpusIndexIter {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        let next_cpu_id =
            unsafe { bindings::cpumask_next(self.index, &bindings::__cpu_online_mask) };
        // When [`bindings::cpumask_next`] can not find further CPUs set in the
        // [`bindings::__cpu_online_mask`], it returns a value >= [`bindings::nr_cpu_ids`].
        if next_cpu_id >= unsafe { bindings::nr_cpu_ids } {
            return None;
        }
        self.index = next_cpu_id as i32;
        Some(next_cpu_id)
    }
}

impl Iterator for PresentCpusIndexIter {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        let next_cpu_id =
            unsafe { bindings::cpumask_next(self.index, &bindings::__cpu_present_mask) };
        // When [`bindings::cpumask_next`] can not find further CPUs set in the
        // [`bindings::__cpu_present_mask`], it returns a value >= [`bindings::nr_cpu_ids`].
        if next_cpu_id >= unsafe { bindings::nr_cpu_ids } {
            return None;
        }
        self.index = next_cpu_id as i32;
        Some(next_cpu_id)
    }
}

/// Returns a [`PossibleCpusIndexIter`] that gives the possible CPU indexes.
///
/// # Examples
///
/// ```
/// # use kernel::prelude::*;
/// # use kernel::cpumask::possible_cpus;
///
/// fn example() {
///     // This prints all the possible cpu indexes.
///     for cpu in possible_cpus(){
///         pr_info!("{}\n", cpu);
///     }
/// }
/// ```
pub fn possible_cpus() -> PossibleCpusIndexIter {
    // Initial index is set to -1. Since [`bindings::cpumask_next`] return the next set bit in a
    // [`bindings::__cpu_possible_mask`], the CPU index should begins from 0.
    PossibleCpusIndexIter { index: -1 }
}

/// Returns a [`OnlineCpusIndexIter`] that gives the online CPU indexes.
///
/// # Examples
///
/// ```
/// # use kernel::prelude::*;
/// # use kernel::cpumask::online_cpus;
///
/// fn example() {
///     // This prints all the online cpu indexes.
///     for cpu in online_cpus(){
///         pr_info!("{}\n", cpu);
///     }
/// }
/// ```
pub fn online_cpus() -> OnlineCpusIndexIter {
    // Initial index is set to -1. Since [`bindings::cpumask_next`] return the next set bit in a
    // [`bindings::__cpu_online_mask`], the CPU index should begins from 0.
    OnlineCpusIndexIter { index: -1 }
}

/// Returns a [`PresentCpusIndexIter`] that gives the present CPU indexes.
///
/// # Examples
///
/// ```
/// # use kernel::prelude::*;
/// # use kernel::cpumask::present_cpus;
///
/// fn example() {
///     // This prints all the present cpu indexes.
///     for cpu in present_cpus(){
///         pr_info!("{}\n", cpu);
///     }
/// }
/// ```
pub fn present_cpus() -> PresentCpusIndexIter {
    // Initial index is set to -1. Since [`bindings::cpumask_next`] return the next set bit in a
    // [`bindings::__cpu_present_mask`], the CPU index should begins from 0.
    PresentCpusIndexIter { index: -1 }
}

// static mut CPU_ONLINE_MASK: bindings::cpumask = unsafe{ bindings::__cpu_online_mask };
/// The `CpumaskT` struct is a wrapper around the `bindings::cpumask_t` struct from the kernel bindings. It represents a CPU mask in the kernel.
#[repr(transparent)]
#[derive(Clone)]
pub struct CpumaskT(bindings::cpumask_t);

impl CpumaskT {
    /// The `from_int` method is a constructor for `CpumaskT`. It takes a 64-bit integer and creates a new `CpumaskT` with that value.
    pub const fn from_int(c: u64) -> Self {
        Self(bindings::cpumask_t { bits: [c, 0, 0, 0] })
        // Self(bindings::cpumask_t { bits: [c] })
    }

    /// The `as_cpumas_ptr` method returns a mutable pointer to the underlying `bindings::cpumask_t`. This can be used to pass the `CpumaskT` to kernel functions that expect a `bindings::cpumask_t`.
    pub fn as_cpumas_ptr(&mut self) -> *mut bindings::cpumask_t {
        &mut self.0 as *mut bindings::cpumask_t
    }

    /// The `as_cpumas_const_ptr` method returns a const pointer to the underlying `bindings::cpumask_t`. This can be used to pass the `CpumaskT` to kernel functions that expect a `bindings::cpumask_t`.
    pub fn as_cpumas_const_ptr(&self) -> *const bindings::cpumask_t {
        &self.0 as *const bindings::cpumask_t
    }

    /// The `cpu_mask_all` method returns a `CpumaskT` that represents all CPUs. It does this by creating a new `CpumaskT` with all bits set to 1.
    pub const fn cpu_mask_all() -> Self {
        let c: u64 = u64::MAX;
        Self(bindings::cpumask_t { bits: [c, c, c, c] })
        // Self(bindings::cpumask_t { bits: [c] })
    }

    /// The `read_cpu_online_mask` function is an unsafe function that reads the CPU online mask from the kernel. It returns a `bindings::cpumask` representing the online CPUs.
    pub unsafe fn read_cpu_online_mask() -> Self {
        unsafe {
            rust_helper_cpus_read_lock();
            let cpumask = Self {
                0: bindings::__cpu_online_mask,
            };
            rust_helper_cpus_read_unlock();
            cpumask
        }
    }

    /// The `cpulist_parse` function parses a CPU list from a string and stores the result in a `bindings::cpumask`. It takes a pointer to a C-style string and a mutable pointer to a `bindings::cpumask`. If the parsing is successful, it returns `Ok(0)`. Otherwise, it returns an `EINVAL` error.
    pub fn cpulist_parse(&mut self, buf: *const c_types::c_char) -> Result<usize> {
        let res = unsafe { rust_helper_cpulist_parse(buf, self.as_cpumas_ptr()) };
        if res == 0 {
            return Ok(0);
        }
        Err(Error::EINVAL)
    }

    /// The `cpumask_copy` function copies a `bindings::cpumask` from one location to another. It takes a mutable pointer to the destination `bindings::cpumask` and a constant pointer to the source `bindings::cpumask`.
    pub fn cpumask_copy(&mut self, src: &CpumaskT) {
        unsafe { rust_helper_cpumask_copy(self.as_cpumas_ptr(), src.as_cpumas_const_ptr()) }
    }

    /// The `cpumask_empty` function checks if a `bindings::cpumask` is empty (i.e., all bits are zero). It takes a constant pointer to the `bindings::cpumask`. If the `bindings::cpumask` is empty, it returns `Ok(0)`. Otherwise, it returns an `EINVAL` error.
    pub fn cpumask_empty(&self) -> Result<usize> {
        let res = unsafe { rust_helper_cpumask_empty(self.as_cpumas_const_ptr()) };
        if res == 1 {
            return Ok(0);
        }
        Err(Error::EINVAL)
    }

    /// The `cpumask_first` function returns the first CPU in a `bindings::cpumask`. It takes a constant pointer to the `bindings::cpumask`.
    pub fn cpumask_first(&self) -> c_types::c_int {
        unsafe { rust_helper_cpumask_first(self.as_cpumas_const_ptr()) }
    }

    /// The `cpumask_set_cpu` function sets a specific CPU in a `bindings::cpumask`. It takes a CPU number and a mutable pointer to a `bindings::cpumask`. It does this by calling the unsafe `rust_helper_cpumask_set_cpu` function with the provided arguments.
    pub fn cpumask_set_cpu(&mut self, cpu: u32) {
        unsafe { rust_helper_cpumask_set_cpu(cpu as c_types::c_uint, self.as_cpumas_ptr()) }
    }

    /// The `cpumask_clear` function clear all cpumasks in a `bindings::cpumask`. It takes a mutable pointer to the `bindings::cpumask`.
    pub fn cpumask_clear(&mut self) {
        unsafe { rust_helper_cpumask_clear(self.as_cpumas_ptr()) }
    }

    /// The `cpumask_test_cpu` function test whether a specific CPU in a `bindings::cpumask`. It takes a CPU number and a const pointer to a `bindings::cpumask`. It does this by calling the unsafe `rust_helper_cpumask_test_cpu` function with the provided arguments.
    pub fn cpumask_test_cpu(&self, cpu: u32) -> bool {
        unsafe { rust_helper_cpumask_test_cpu(cpu, self.as_cpumas_const_ptr()) != 0 }
    }

    /// The `cpumask_of` function returns a specific `bindings::cpumask`.
    pub fn cpumask_of(cpu: u32) -> *const CpumaskT {
        unsafe { rust_helper_cpumask_of(cpu) as *const _ }
    }
}

impl BitAnd for CpumaskT {
    type Output = CpumaskT;

    /// Override the `&` operator for `CpumaskT`. This function returns a new `CpumaskT`
    /// which holds the calculation result (`self` & `rhs`).
    fn bitand(self, rhs: Self) -> Self::Output {
        let mut result = Self::from_int(0);
        unsafe {
            rust_helper_cpumask_and(
                result.as_cpumas_ptr(),
                self.as_cpumas_const_ptr(),
                rhs.as_cpumas_const_ptr(),
            );
        }
        result
    }
}

#[cfg(not(CONFIG_CPUMASK_OFFSTACK))]
/// The `CpumaskVarT` struct is a wrapper around the `bindings::cpumask_var_t` struct from the kernel bindings. It represents a variable CPU mask in the kernel.
pub struct CpumaskVarT(bindings::cpumask_var_t);

impl CpumaskVarT {
    /// The `from_int` method is a constructor for `CpumaskVarT`. It takes a 64-bit integer and creates a new `CpumaskVarT` with that value.
    pub const fn from_int(c: u64) -> Self {
        Self([bindings::cpumask_t { bits: [c, 0, 0, 0] }])
        // Self([bindings::cpumask_t { bits: [c] }])
    }

    /// The `alloc_cpumask_var` method allocates a new `CpumaskVarT`. It takes a mutable reference to a `CpumaskVarT` and returns a `Result<usize>`. If the allocation is successful, it returns `Ok(0)`.
    pub fn alloc_cpumask_var(_mask: &mut CpumaskVarT) -> Result<usize> {
        Ok(0)
    }

    /// The `free_cpumask_var` method frees a `CpumaskVarT`. It takes a mutable reference to a `CpumaskVarT` and returns a `Result<usize>`. If the freeing is successful, it returns `Ok(0)`.
    pub fn free_cpumask_var(_mask: &mut CpumaskVarT) -> Result<usize> {
        Ok(0)
    }
}

#[cfg(CONFIG_CPUMASK_OFFSTACK)]
impl CpumaskVarT {
    // todo: implement for x86_64/x86
}

/// The `num_possible_cpus` function returns the number of possible CPUs. It does this by calling the unsafe `rust_helper_num_possible_cpus` function.
pub fn num_possible_cpus() -> u32 {
    unsafe { rust_helper_num_possible_cpus() }
}
