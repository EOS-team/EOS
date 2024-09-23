// SPDX-License-Identifier: GPL-2.0

//! Files and file descriptors.
//!
//! C headers: [`include/linux/fs.h`](../../../../include/linux/fs.h) and
//! [`include/linux/file.h`](../../../../include/linux/file.h)

use crate::{bindings, c_types, error::Error, str::CStr, Result};
use core::{mem::ManuallyDrop, ops::Deref};

/// Wraps the kernel's `struct file`.
///
/// # Invariants
///
/// The pointer `File::ptr` is non-null and valid. Its reference count is also non-zero.
pub struct File {
    /// The pointer `File::ptr` is non-null and valid. Its reference count is also non-zero.
    pub ptr: *mut bindings::file,
}

impl File {
    /// Constructs a new [`struct file`] wrapper from a file pointer.
    pub fn from_ptr(ptr: *mut bindings::file) -> Self {
        Self { ptr }
    }

    /// Constructs a new [`struct file`] wrapper from a file descriptor.
    ///
    /// The file descriptor belongs to the current process.
    pub fn from_fd(fd: u32) -> Result<Self> {
        // SAFETY: FFI call, there are no requirements on `fd`.
        let ptr = unsafe { bindings::fget(fd) };
        if ptr.is_null() {
            return Err(Error::EBADF);
        }

        // INVARIANTS: We checked that `ptr` is non-null, so it is valid. `fget` increments the ref
        // count before returning.
        Ok(Self { ptr })
    }

    /// Creates a new file instance by hooking it up to an anonymous inode,
    /// and a dentry that describe the "class" of the file.
    pub fn anon_inode_getfile(
        name: *const c_types::c_char,
        fops: *const bindings::file_operations,
        priv_: *mut c_types::c_void,
        flags: c_types::c_int,
    ) -> *mut bindings::file {
        unsafe { bindings::anon_inode_getfile(name, fops, priv_, flags) }
    }

    /// Returns the current seek/cursor/pointer position (`struct file::f_pos`).
    pub fn pos(&self) -> u64 {
        // SAFETY: `File::ptr` is guaranteed to be valid by the type invariants.
        unsafe { (*self.ptr).f_pos as u64 }
    }

    /// Returns whether the file is in blocking mode.
    pub fn is_blocking(&self) -> bool {
        // SAFETY: `File::ptr` is guaranteed to be valid by the type invariants.
        unsafe { (*self.ptr).f_flags & bindings::O_NONBLOCK == 0 }
    }

    /// Sets the private data of the file.
    ///
    /// SAFETY: The caller must ensure that `data` is a valid pointer.
    pub fn set_private_data(&self, data: *mut c_types::c_void) {
        unsafe { (*self.ptr).private_data = data as _ };
    }

    /// Returns the raw pointer to the underlying `file` struct.
    pub fn get_ptr(&self) -> *mut bindings::file {
        self.ptr
    }

    /// Returns the parent directory name of the file
    pub fn get_parent_name(&self) -> Result<&str> {
        let d = unsafe { (*(*self.ptr).f_path.dentry).d_parent };
        if d.is_null() {
            return Err(Error::EINVAL);
        }
        unsafe {
            match CStr::from_char_ptr((*d).d_name.name as *const c_types::c_char).to_str() {
                Ok(s) => Ok(s),
                Err(_) => Err(Error::EINVAL),
            }
        }
    }

    /// Returns the file's name
    pub fn get_name(&self) -> Result<&str> {
        unsafe {
            match CStr::from_char_ptr(
                (*(*self.ptr).f_path.dentry).d_name.name as *const c_types::c_char,
            )
            .to_str()
            {
                Ok(s) => Ok(s),
                Err(_) => Err(Error::EINVAL),
            }
        }
    }
}

impl Drop for File {
    fn drop(&mut self) {
        // SAFETY: The type invariants guarantee that `File::ptr` has a non-zero reference count.
        unsafe { bindings::fput(self.ptr) };
    }
}

/// A wrapper for [`File`] that doesn't automatically decrement the refcount when dropped.
///
/// We need the wrapper because [`ManuallyDrop`] alone would allow callers to call
/// [`ManuallyDrop::into_inner`]. This would allow an unsafe sequence to be triggered without
/// `unsafe` blocks because it would trigger an unbalanced call to `fput`.
///
/// # Invariants
///
/// The wrapped [`File`] remains valid for the lifetime of the object.
pub(crate) struct FileRef(ManuallyDrop<File>);

impl FileRef {
    /// Constructs a new [`struct file`] wrapper that doesn't change its reference count.
    ///
    /// # Safety
    ///
    /// The pointer `ptr` must be non-null and valid for the lifetime of the object.
    pub(crate) unsafe fn from_ptr(ptr: *mut bindings::file) -> Self {
        Self(ManuallyDrop::new(File { ptr }))
    }
}

impl Deref for FileRef {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

/// A file descriptor reservation.
///
/// This allows the creation of a file descriptor in two steps: first, we reserve a slot for it,
/// then we commit or drop the reservation. The first step may fail (e.g., the current process ran
/// out of available slots), but commit and drop never fail (and are mutually exclusive).
pub struct FileDescriptorReservation {
    /// The file descriptor (fd) is a non-negative integer that is used to access the open files or I/O devices.
    pub fd: u32,
}

impl FileDescriptorReservation {
    /// Creates a new file descriptor reservation.
    pub fn new(flags: u32) -> Result<Self> {
        let fd = unsafe { bindings::get_unused_fd_flags(flags) };
        if fd < 0 {
            return Err(Error::from_kernel_errno(fd));
        }
        Ok(Self { fd: fd as _ })
    }

    /// Returns the file descriptor number that was reserved.
    pub fn reserved_fd(&self) -> u32 {
        self.fd
    }

    /// Commits the reservation.
    ///
    /// The previously reserved file descriptor is bound to `file`.
    pub fn commit(self, file: File) {
        // SAFETY: `self.fd` was previously returned by `get_unused_fd_flags`, and `file.ptr` is
        // guaranteed to have an owned ref count by its type invariants.
        unsafe { bindings::fd_install(self.fd, file.ptr) };

        // `fd_install` consumes both the file descriptor and the file reference, so we cannot run
        // the destructors.
        core::mem::forget(self);
        core::mem::forget(file);
    }
}

impl Drop for FileDescriptorReservation {
    fn drop(&mut self) {
        // SAFETY: `self.fd` was returned by a previous call to `get_unused_fd_flags`.
        unsafe { bindings::put_unused_fd(self.fd) };
    }
}

/// call linux fd_install
pub fn fd_install(fd: u32, filp: *mut bindings::file) {
    // SAFETY: The caller must ensure that `filp` is a valid pointer.
    unsafe {
        bindings::fd_install(fd, filp);
    }
}

/// Wraps the kernel's `struct files_struct`.
#[repr(transparent)]
pub struct FilesStruct {
    ptr: *mut bindings::files_struct,
}

impl FilesStruct {
    /// Returns a `FilesStruct` struct from a non-null and valid pointer.
    pub fn from_ptr(ptr: *mut bindings::files_struct) -> Self {
        Self { ptr }
    }

    /// Get self's `ptr`.
    pub fn get_ptr(&self) -> *mut bindings::files_struct {
        self.ptr
    }
}
