// SPDX-License-Identifier: GPL-2.0

//! System control.
//!
//! C header: [`include/linux/sysfs.h`](../../../../include/linux/sysfs.h)

use crate::bindings;
use core;

/// Struct `Attribute` represents a system attribute.
/// It wraps the `attribute` struct from the bindings module.
/// It includes a method `new` for creating a new `Attribute`.
/// The `new` method initializes the `Attribute` with zeroed memory.
pub struct Attribute(bindings::attribute);

impl Attribute {
    /// Method `new` creates a new `Attribute`.
    /// It initializes the `Attribute` with zeroed memory.
    #[allow(dead_code)]
    fn new() -> Self {
        unsafe { core::mem::zeroed() }
    }
}

/// Struct `AttributeGroup` represents a group of system attributes.
/// It wraps the `attribute_group` struct from the bindings module.
/// It includes a method `new` for creating a new `AttributeGroup`.
/// The `new` method initializes the `AttributeGroup` with zeroed memory.
pub struct AttributeGroup(bindings::attribute_group);

impl AttributeGroup {
    /// Method `new` creates a new `AttributeGroup`.
    /// It initializes the `AttributeGroup` with zeroed memory.
    pub fn new() -> Self {
        unsafe { core::mem::zeroed() }
    }
}
