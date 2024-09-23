// SPDX-License-Identifier: GPL-2.0

//! Rust miscellaneous device sample

#![no_std]
#![feature(allocator_api, global_asm)]

use kernel::prelude::*;
use kernel::{
    c_str,
    file::File,
    file_operations::{FileOpener, FileOperations},
    io_buffer::{IoBufferReader, IoBufferWriter},
    miscdev,
    sync::{CondVar, Mutex, Ref},
};

module! {
    type: RustMiscdev,
    name: b"rust_miscdev",
    author: b"Rust for Linux Contributors",
    description: b"Rust miscellaneous device sample",
    license: b"GPL v2",
}

const MAX_TOKENS: usize = 3;

struct SharedStateInner {
    token_count: usize,
}

struct SharedState {
    state_changed: CondVar,
    inner: Mutex<SharedStateInner>,
}

impl SharedState {
    fn try_new() -> Result<Pin<Ref<Self>>> {
        Ok(Ref::pinned(Ref::try_new_and_init(
            Self {
                // SAFETY: `condvar_init!` is called below.
                state_changed: unsafe { CondVar::new() },
                // SAFETY: `mutex_init!` is called below.
                inner: unsafe { Mutex::new(SharedStateInner { token_count: 0 }) },
            },
            |mut state| {
                // SAFETY: `state_changed` is pinned when `state` is.
                let pinned = unsafe { state.as_mut().map_unchecked_mut(|s| &mut s.state_changed) };
                kernel::condvar_init!(pinned, "SharedState::state_changed");
                // SAFETY: `inner` is pinned when `state` is.
                let pinned = unsafe { state.as_mut().map_unchecked_mut(|s| &mut s.inner) };
                kernel::mutex_init!(pinned, "SharedState::inner");
            },
        )?))
    }
}

struct Token;

impl FileOpener<Pin<Ref<SharedState>>> for Token {
    fn open(shared: &Pin<Ref<SharedState>>, _fileref: &File) -> Result<Self::Wrapper> {
        Ok(shared.clone())
    }
}

impl FileOperations for Token {
    type Wrapper = Pin<Ref<SharedState>>;

    kernel::declare_file_operations!(read, write);

    fn read<T: IoBufferWriter>(
        shared: &Ref<SharedState>,
        _: &File,
        data: &mut T,
        offset: u64,
    ) -> Result<usize> {
        // Succeed if the caller doesn't provide a buffer or if not at the start.
        if data.is_empty() || offset != 0 {
            return Ok(0);
        }

        {
            let mut inner = shared.inner.lock();

            // Wait until we are allowed to decrement the token count or a signal arrives.
            while inner.token_count == 0 {
                if shared.state_changed.wait(&mut inner) {
                    return Err(Error::EINTR);
                }
            }

            // Consume a token.
            inner.token_count -= 1;
        }

        // Notify a possible writer waiting.
        shared.state_changed.notify_all();

        // Write a one-byte 1 to the reader.
        data.write_slice(&[1u8; 1])?;
        Ok(1)
    }

    fn write<T: IoBufferReader>(
        shared: &Ref<SharedState>,
        _: &File,
        data: &mut T,
        _offset: u64,
    ) -> Result<usize> {
        {
            let mut inner = shared.inner.lock();

            // Wait until we are allowed to increment the token count or a signal arrives.
            while inner.token_count == MAX_TOKENS {
                if shared.state_changed.wait(&mut inner) {
                    return Err(Error::EINTR);
                }
            }

            // Increment the number of token so that a reader can be released.
            inner.token_count += 1;
        }

        // Notify a possible reader waiting.
        shared.state_changed.notify_all();
        Ok(data.len())
    }
}

struct RustMiscdev {
    _dev: Pin<Box<miscdev::Registration<Pin<Ref<SharedState>>>>>,
}

impl KernelModule for RustMiscdev {
    fn init() -> Result<Self> {
        pr_info!("Rust miscellaneous device sample (init)\n");

        let state = SharedState::try_new()?;

        Ok(RustMiscdev {
            _dev: miscdev::Registration::new_pinned::<Token>(c_str!("rust_miscdev"), None, state)?,
        })
    }
}

impl Drop for RustMiscdev {
    fn drop(&mut self) {
        pr_info!("Rust miscellaneous device sample (exit)\n");
    }
}
