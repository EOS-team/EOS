// SPDX-License-Identifier: GPL-2.0

//! delay
//!
//! C header: [`include/linux/delay.h`](../../../../include/linux/delay.h)

use crate::{bindings, c_types::*};

/// Drop in replacement for udelay where wakeup is flexible.
pub fn usleep_range(min: c_ulong, max: c_ulong) {
    unsafe {
        bindings::usleep_range(min, max);
    }
}
