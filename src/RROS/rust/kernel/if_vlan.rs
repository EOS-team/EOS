// SPDX-License-Identifier: GPL-2.0

//! if_vlan
//!
//! C header: [`include/linux/if_vlan.h`](../../../../include/linux/if_vlan.h)

use crate::bindings;
use core::cell::UnsafeCell;

/// The `VlanEthhdr` struct wraps a `bindings::vlan_ethhdr` struct from the kernel bindings.
#[repr(transparent)]
pub struct VlanEthhdr(pub(crate) UnsafeCell<bindings::vlan_ethhdr>);

impl VlanEthhdr {
    /// Returns a mutable reference to inner struct.
    pub fn get_mut(&mut self) -> &mut bindings::vlan_ethhdr {
        self.0.get_mut()
    }

    /// Returns a unmutable reference to inner struct.
    pub fn get(&mut self) -> Option<&bindings::vlan_ethhdr> {
        unsafe { self.0.get().as_ref() }
    }
}
