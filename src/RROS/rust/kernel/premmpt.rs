// SPDX-License-Identifier: GPL-2.0

//! premmpt
//!
//! C header: [`include/linux/premmpt.h`](../../../../include/linux/premmpt.h)

use crate::{
    c_types,
    error::{Error, Result},
};

extern "C" {
    // #[allow(improper_ctypes)]
    fn rust_helper_running_inband() -> c_types::c_int;
}

/// Function `running_inband` checks if the current task is running in-band.
/// It calls `rust_helper_running_inband` to perform the check.
/// If the current task is running in-band, it returns 0.
/// If the current task is not running in-band, it returns an error.
pub fn running_inband() -> Result<usize> {
    let res = unsafe { rust_helper_running_inband() };
    if res == 1 {
        return Ok(0);
    }
    Err(Error::EINVAL)
}
