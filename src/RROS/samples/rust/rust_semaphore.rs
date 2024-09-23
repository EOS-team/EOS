// SPDX-License-Identifier: GPL-2.0

//! Rust semaphore sample
//!
//! A counting semaphore that can be used by userspace.
//!
//! The count is incremented by writes to the device. A write of `n` bytes results in an increment
//! of `n`. It is decremented by reads; each read results in the count being decremented by 1. If
//! the count is already zero, a read will block until another write increments it.
//!
//! This can be used in user space from the shell for example  as follows (assuming a node called
//! `semaphore`): `cat semaphore` decrements the count by 1 (waiting for it to become non-zero
//! before decrementing); `echo -n 123 > semaphore` increments the semaphore by 3, potentially
//! unblocking up to 3 blocked readers.

#![no_std]
#![feature(allocator_api, global_asm)]

use core::sync::atomic::{AtomicU64, Ordering};
use kernel::{
    c_str, condvar_init, declare_file_operations,
    file::File,
    file_operations::{FileOpener, FileOperations, IoctlCommand, IoctlHandler},
    io_buffer::{IoBufferReader, IoBufferWriter},
    miscdev::Registration,
    mutex_init,
    prelude::*,
    sync::{CondVar, Mutex, Ref},
    user_ptr::{UserSlicePtrReader, UserSlicePtrWriter},
};

module! {
    type: RustSemaphore,
    name: b"rust_semaphore",
    author: b"Rust for Linux Contributors",
    description: b"Rust semaphore sample",
    license: b"GPL v2",
}

struct SemaphoreInner {
    count: usize,
    max_seen: usize,
}

struct Semaphore {
    changed: CondVar,
    inner: Mutex<SemaphoreInner>,
}

struct FileState {
    read_count: AtomicU64,
    shared: Ref<Semaphore>,
}

impl FileState {
    fn consume(&self) -> Result {
        let mut inner = self.shared.inner.lock();
        while inner.count == 0 {
            if self.shared.changed.wait(&mut inner) {
                return Err(Error::EINTR);
            }
        }
        inner.count -= 1;
        Ok(())
    }
}

impl FileOpener<Ref<Semaphore>> for FileState {
    fn open(shared: &Ref<Semaphore>, _fileref: &File) -> Result<Box<Self>> {
        Ok(Box::try_new(Self {
            read_count: AtomicU64::new(0),
            shared: shared.clone(),
        })?)
    }
}

impl FileOperations for FileState {
    declare_file_operations!(read, write, ioctl);

    fn read<T: IoBufferWriter>(this: &Self, _: &File, data: &mut T, offset: u64) -> Result<usize> {
        if data.is_empty() || offset > 0 {
            return Ok(0);
        }
        this.consume()?;
        data.write_slice(&[0u8; 1])?;
        this.read_count.fetch_add(1, Ordering::Relaxed);
        Ok(1)
    }

    fn write<T: IoBufferReader>(this: &Self, _: &File, data: &mut T, _offs: u64) -> Result<usize> {
        {
            let mut inner = this.shared.inner.lock();
            inner.count = inner.count.saturating_add(data.len());
            if inner.count > inner.max_seen {
                inner.max_seen = inner.count;
            }
        }

        this.shared.changed.notify_all();
        Ok(data.len())
    }

    fn ioctl(this: &Self, file: &File, cmd: &mut IoctlCommand) -> Result<i32> {
        cmd.dispatch::<Self>(this, file)
    }
}

struct RustSemaphore {
    _dev: Pin<Box<Registration<Ref<Semaphore>>>>,
}

impl KernelModule for RustSemaphore {
    fn init() -> Result<Self> {
        pr_info!("Rust semaphore sample (init)\n");

        let sema = Ref::try_new_and_init(
            Semaphore {
                // SAFETY: `condvar_init!` is called below.
                changed: unsafe { CondVar::new() },

                // SAFETY: `mutex_init!` is called below.
                inner: unsafe {
                    Mutex::new(SemaphoreInner {
                        count: 0,
                        max_seen: 0,
                    })
                },
            },
            |mut sema| {
                // SAFETY: `changed` is pinned when `sema` is.
                let pinned = unsafe { sema.as_mut().map_unchecked_mut(|s| &mut s.changed) };
                condvar_init!(pinned, "Semaphore::changed");

                // SAFETY: `inner` is pinned when `sema` is.
                let pinned = unsafe { sema.as_mut().map_unchecked_mut(|s| &mut s.inner) };
                mutex_init!(pinned, "Semaphore::inner");
            },
        )?;

        Ok(Self {
            _dev: Registration::new_pinned::<FileState>(c_str!("rust_semaphore"), None, sema)?,
        })
    }
}

impl Drop for RustSemaphore {
    fn drop(&mut self) {
        pr_info!("Rust semaphore sample (exit)\n");
    }
}

const IOCTL_GET_READ_COUNT: u32 = 0x80086301;
const IOCTL_SET_READ_COUNT: u32 = 0x40086301;

impl IoctlHandler for FileState {
    type Target = Self;

    fn read(this: &Self, _: &File, cmd: u32, writer: &mut UserSlicePtrWriter) -> Result<i32> {
        match cmd {
            IOCTL_GET_READ_COUNT => {
                writer.write(&this.read_count.load(Ordering::Relaxed))?;
                Ok(0)
            }
            _ => Err(Error::EINVAL),
        }
    }

    fn write(this: &Self, _: &File, cmd: u32, reader: &mut UserSlicePtrReader) -> Result<i32> {
        match cmd {
            IOCTL_SET_READ_COUNT => {
                this.read_count.store(reader.read()?, Ordering::Relaxed);
                Ok(0)
            }
            _ => Err(Error::EINVAL),
        }
    }
}
