// SPDX-License-Identifier: GPL-2.0

//! Struct for writing to a pre-allocated buffer with the [`write!`] macro.

use core::fmt;

/// A pre-allocated buffer that implements [`core::fmt::Write`].
///
/// Consecutive writes will append to what has already been written.
/// Writes that don't fit in the buffer will fail.
pub struct Buffer<'a> {
    slice: &'a mut [u8],
    pos: usize,
}

impl<'a> Buffer<'a> {
    /// Create a new buffer from an existing array.
    pub fn new(slice: &'a mut [u8]) -> Self {
        Buffer { slice, pos: 0 }
    }

    /// Number of bytes that have already been written to the buffer.
    /// This will always be less than the length of the original array.
    pub fn bytes_written(&self) -> usize {
        self.pos
    }
}

impl<'a> fmt::Write for Buffer<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if s.len() > self.slice.len() - self.pos {
            Err(fmt::Error)
        } else {
            self.slice[self.pos..self.pos + s.len()].copy_from_slice(s.as_bytes());
            self.pos += s.len();
            Ok(())
        }
    }
}
