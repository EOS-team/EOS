// SPDX-License-Identifier: GPL-2.0

//! irqstage
//!
//! C header: [`include/linux/irqstage.h`](../../../../include/linux/irqstage.h)

use crate::{
    bindings, c_types,
    error::{Error, Result},
};
/// The `enable_oob_stage` function is a wrapper around the `bindings::enable_oob_stage` function from the kernel bindings. It enables the out-of-band (OOB) stage with the given name.
///
/// `name`: A pointer to a `c_char` that represents the name of the OOB stage to enable.
///
/// This function calls the `bindings::enable_oob_stage` function with the provided name. If the function returns 0, it returns `Ok(0)`. Otherwise, it returns `Err(Error::EINVAL)`.
///
/// This function is unsafe because it calls an unsafe function from the kernel bindings.
pub fn enable_oob_stage(name: *const c_types::c_char) -> Result<usize> {
    let res = unsafe { bindings::enable_oob_stage(name) };
    if res == 0 {
        return Ok(0);
    }
    Err(Error::EINVAL)
}
