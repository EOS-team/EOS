// SPDX-License-Identifier: GPL-2.0

//! File operations.
//!
//! C header: [`include/linux/fs.h`](../../../../include/linux/fs.h)

use core::convert::{TryFrom, TryInto};
use core::{marker, mem, ops::Deref, ptr};

use alloc::boxed::Box;

use crate::{
    bindings, c_types,
    error::{Error, Result},
    file::{File, FileRef},
    from_kernel_result,
    io_buffer::{IoBufferReader, IoBufferWriter},
    iov_iter::IovIter,
    sync::CondVar,
    types::PointerWrapper,
    user_ptr::{UserSlicePtr, UserSlicePtrReader, UserSlicePtrWriter},
};

/// Wraps the kernel's `struct poll_table_struct`.
///
/// # Invariants
///
/// The pointer `PollTable::ptr` is null or valid.
pub struct PollTable {
    ptr: *mut bindings::poll_table_struct,
}

impl PollTable {
    /// Constructors a new `struct poll_table_struct` wrapper.
    ///
    /// # Safety
    ///
    /// The pointer `ptr` must be either null or a valid pointer for the lifetime of the object.
    unsafe fn from_ptr(ptr: *mut bindings::poll_table_struct) -> Self {
        Self { ptr }
    }

    /// Associates the given file and condition variable to this poll table. It means notifying the
    /// condition variable will notify the poll table as well; additionally, the association
    /// between the condition variable and the file will automatically be undone by the kernel when
    /// the file is destructed. To unilaterally remove the association before then, one can call
    /// [`CondVar::free_waiters`].
    ///
    /// # Safety
    ///
    /// If the condition variable is destroyed before the file, then [`CondVar::free_waiters`] must
    /// be called to ensure that all waiters are flushed out.
    pub unsafe fn register_wait<'a>(&self, file: &'a File, cv: &'a CondVar) {
        if self.ptr.is_null() {
            return;
        }

        // SAFETY: `PollTable::ptr` is guaranteed to be valid by the type invariants and the null
        // check above.
        let table = unsafe { &*self.ptr };
        if let Some(proc) = table._qproc {
            // SAFETY: All pointers are known to be valid.
            unsafe { proc(file.ptr as _, cv.wait_list.get(), self.ptr) }
        }
    }
}

/// Wraps the kernel's `struct oob_poll_wait`.
///
/// # Invariants
///
/// The pointer `OobPollWait::ptr` is null or valid.
pub struct OobPollWait {
    ptr: *mut bindings::oob_poll_wait,
}

impl OobPollWait {
    /// Constructors a new `struct oob_poll_wait` wrapper.
    ///
    /// # Safety
    ///
    /// The pointer `ptr` must be either null or a valid pointer for the lifetime of the object.
    unsafe fn from_ptr(ptr: *mut bindings::oob_poll_wait) -> Self {
        Self { ptr }
    }

    /// get ptr in OobPollWait Wrapper
    pub fn get_ptr(&self) -> *mut bindings::oob_poll_wait {
        self.ptr
    }
}
/// Equivalent to [`std::io::SeekFrom`].
///
/// [`std::io::SeekFrom`]: https://doc.rust-lang.org/std/io/enum.SeekFrom.html
pub enum SeekFrom {
    /// Equivalent to C's `SEEK_SET`.
    Start(u64),

    /// Equivalent to C's `SEEK_END`.
    End(i64),

    /// Equivalent to C's `SEEK_CUR`.
    Current(i64),
}

unsafe extern "C" fn open_callback<A: FileOpenAdapter, T: FileOpener<A::Arg>>(
    inode: *mut bindings::inode,
    file: *mut bindings::file,
) -> c_types::c_int {
    from_kernel_result! {
        // SAFETY: `A::convert` must return a valid non-null pointer that
        // should point to data in the inode or file that lives longer
        // than the following use of `T::open`.
        let arg = unsafe { A::convert(inode, file) };
        // SAFETY: The C contract guarantees that `file` is valid. Additionally,
        // `fileref` never outlives this function, so it is guaranteed to be
        // valid.
        let fileref = unsafe {
            FileRef::from_ptr(file)
        };
        // SAFETY: `arg` was previously returned by `A::convert` and must
        // be a valid non-null pointer
        let ptr = T::open(unsafe { &*arg }, &fileref)?.into_pointer();
        // SAFETY: The C contract guarantees that `private_data` is available
        // for implementers of the file operations (no other C code accesses
        // it), so we know that there are no concurrent threads/CPUs accessing
        // it (it's not visible to any other Rust code).
        unsafe { (*file).private_data = ptr as *mut c_types::c_void };
        Ok(0)
    }
}

unsafe extern "C" fn read_callback<T: FileOperations>(
    file: *mut bindings::file,
    buf: *mut c_types::c_char,
    len: c_types::c_size_t,
    offset: *mut bindings::loff_t,
) -> c_types::c_ssize_t {
    from_kernel_result! {
        let mut data = unsafe { UserSlicePtr::new(buf as *mut c_types::c_void, len).writer() };
        // SAFETY: `private_data` was initialised by `open_callback` with a value returned by
        // `T::Wrapper::into_pointer`. `T::Wrapper::from_pointer` is only called by the `release`
        // callback, which the C API guarantees that will be called only when all references to
        // `file` have been released, so we know it can't be called while this function is running.
        let f = unsafe { T::Wrapper::borrow((*file).private_data) };
        // No `FMODE_UNSIGNED_OFFSET` support, so `offset` must be in [0, 2^63).
        // See discussion in https://github.com/fishinabarrel/linux-kernel-module-rust/pull/113
        let read = T::read(&f, unsafe { &FileRef::from_ptr(file) }, &mut data, unsafe { *offset }.try_into()?)?;
        unsafe { (*offset) += bindings::loff_t::try_from(read).unwrap() };
        Ok(read as _)
    }
}

unsafe extern "C" fn oob_read_callback<T: FileOperations>(
    file: *mut bindings::file,
    buf: *mut c_types::c_char,
    len: c_types::c_size_t,
) -> c_types::c_ssize_t {
    from_kernel_result! {
        let mut data = unsafe { UserSlicePtr::new(buf as *mut c_types::c_void, len).writer() };
        // SAFETY: `private_data` was initialised by `open_callback` with a value returned by
        // `T::Wrapper::into_pointer`. `T::Wrapper::from_pointer` is only called by the `release`
        // callback, which the C API guarantees that will be called only when all references to
        // `file` have been released, so we know it can't be called while this function is running.
        let f = unsafe { T::Wrapper::borrow((*file).private_data) };
        // No `FMODE_UNSIGNED_OFFSET` support, so `offset` must be in [0, 2^63).
        // See discussion in https://github.com/fishinabarrel/linux-kernel-module-rust/pull/113
        let read = T::oob_read(&f, unsafe { &FileRef::from_ptr(file) }, &mut data)?;
        Ok(read as _)
    }
}

unsafe extern "C" fn read_iter_callback<T: FileOperations>(
    iocb: *mut bindings::kiocb,
    raw_iter: *mut bindings::iov_iter,
) -> isize {
    from_kernel_result! {
        let mut iter = unsafe { IovIter::from_ptr(raw_iter) };
        let file = unsafe { (*iocb).ki_filp };
        let offset = unsafe { (*iocb).ki_pos };
        // SAFETY: `private_data` was initialised by `open_callback` with a value returned by
        // `T::Wrapper::into_pointer`. `T::Wrapper::from_pointer` is only called by the `release`
        // callback, which the C API guarantees that will be called only when all references to
        // `file` have been released, so we know it can't be called while this function is running.
        let f = unsafe { T::Wrapper::borrow((*file).private_data) };
        let read = T::read(&f, unsafe { &FileRef::from_ptr(file) }, &mut iter, offset.try_into()?)?;
        unsafe { (*iocb).ki_pos += bindings::loff_t::try_from(read).unwrap() };
        Ok(read as _)
    }
}

unsafe extern "C" fn write_callback<T: FileOperations>(
    file: *mut bindings::file,
    buf: *const c_types::c_char,
    len: c_types::c_size_t,
    offset: *mut bindings::loff_t,
) -> c_types::c_ssize_t {
    from_kernel_result! {
        let mut data = unsafe { UserSlicePtr::new(buf as *mut c_types::c_void, len).reader() };
        // SAFETY: `private_data` was initialised by `open_callback` with a value returned by
        // `T::Wrapper::into_pointer`. `T::Wrapper::from_pointer` is only called by the `release`
        // callback, which the C API guarantees that will be called only when all references to
        // `file` have been released, so we know it can't be called while this function is running.
        let f = unsafe { T::Wrapper::borrow((*file).private_data) };
        // No `FMODE_UNSIGNED_OFFSET` support, so `offset` must be in [0, 2^63).
        // See discussion in https://github.com/fishinabarrel/linux-kernel-module-rust/pull/113
        let written = T::write(&f, unsafe { &FileRef::from_ptr(file) }, &mut data, unsafe { *offset }.try_into()?)?;
        unsafe { (*offset) += bindings::loff_t::try_from(written).unwrap() };
        Ok(written as _)
    }
}

unsafe extern "C" fn oob_write_callback<T: FileOperations>(
    file: *mut bindings::file,
    buf: *const c_types::c_char,
    len: c_types::c_size_t,
) -> c_types::c_ssize_t {
    from_kernel_result! {
        let mut data = unsafe { UserSlicePtr::new(buf as *mut c_types::c_void, len).reader() };
        // SAFETY: `private_data` was initialised by `open_callback` with a value returned by
        // `T::Wrapper::into_pointer`. `T::Wrapper::from_pointer` is only called by the `release`
        // callback, which the C API guarantees that will be called only when all references to
        // `file` have been released, so we know it can't be called while this function is running.
        let f = unsafe { T::Wrapper::borrow((*file).private_data) };
        // No `FMODE_UNSIGNED_OFFSET` support, so `offset` must be in [0, 2^63).
        // See discussion in https://github.com/fishinabarrel/linux-kernel-module-rust/pull/113
        let written = T::oob_write(&f, unsafe { &FileRef::from_ptr(file) }, &mut data)?;
        Ok(written as _)
    }
}

unsafe extern "C" fn write_iter_callback<T: FileOperations>(
    iocb: *mut bindings::kiocb,
    raw_iter: *mut bindings::iov_iter,
) -> isize {
    from_kernel_result! {
        let mut iter = unsafe { IovIter::from_ptr(raw_iter) };
        let file = unsafe { (*iocb).ki_filp };
        let offset = unsafe { (*iocb).ki_pos };
        // SAFETY: `private_data` was initialised by `open_callback` with a value returned by
        // `T::Wrapper::into_pointer`. `T::Wrapper::from_pointer` is only called by the `release`
        // callback, which the C API guarantees that will be called only when all references to
        // `file` have been released, so we know it can't be called while this function is running.
        let f = unsafe { T::Wrapper::borrow((*file).private_data) };
        let written = T::write(&f, unsafe { &FileRef::from_ptr(file) }, &mut iter, offset.try_into()?)?;
        unsafe { (*iocb).ki_pos += bindings::loff_t::try_from(written).unwrap() };
        Ok(written as _)
    }
}

unsafe extern "C" fn release_callback<T: FileOperations>(
    _inode: *mut bindings::inode,
    file: *mut bindings::file,
) -> c_types::c_int {
    let ptr = mem::replace(unsafe { &mut (*file).private_data }, ptr::null_mut());
    T::release(unsafe { T::Wrapper::from_pointer(ptr as _) }, unsafe {
        &FileRef::from_ptr(file)
    });
    0
}

unsafe extern "C" fn llseek_callback<T: FileOperations>(
    file: *mut bindings::file,
    offset: bindings::loff_t,
    whence: c_types::c_int,
) -> bindings::loff_t {
    from_kernel_result! {
        let off = match whence as u32 {
            bindings::SEEK_SET => SeekFrom::Start(offset.try_into()?),
            bindings::SEEK_CUR => SeekFrom::Current(offset),
            bindings::SEEK_END => SeekFrom::End(offset),
            _ => return Err(Error::EINVAL),
        };
        // SAFETY: `private_data` was initialised by `open_callback` with a value returned by
        // `T::Wrapper::into_pointer`. `T::Wrapper::from_pointer` is only called by the `release`
        // callback, which the C API guarantees that will be called only when all references to
        // `file` have been released, so we know it can't be called while this function is running.
        let f = unsafe { T::Wrapper::borrow((*file).private_data) };
        let off = T::seek(&f, unsafe { &FileRef::from_ptr(file) }, off)?;
        Ok(off as bindings::loff_t)
    }
}

unsafe extern "C" fn unlocked_ioctl_callback<T: FileOperations>(
    file: *mut bindings::file,
    cmd: c_types::c_uint,
    arg: c_types::c_ulong,
) -> c_types::c_long {
    from_kernel_result! {
        // SAFETY: `private_data` was initialised by `open_callback` with a value returned by
        // `T::Wrapper::into_pointer`. `T::Wrapper::from_pointer` is only called by the `release`
        // callback, which the C API guarantees that will be called only when all references to
        // `file` have been released, so we know it can't be called while this function is running.
        let f = unsafe { T::Wrapper::borrow((*file).private_data) };
        let mut cmd = IoctlCommand::new(cmd as _, arg as _);
        let ret = T::ioctl(&f, unsafe { &FileRef::from_ptr(file) }, &mut cmd)?;
        Ok(ret as _)
    }
}

unsafe extern "C" fn oob_ioctl_callback<T: FileOperations>(
    file: *mut bindings::file,
    cmd: c_types::c_uint,
    arg: c_types::c_ulong,
) -> c_types::c_long {
    from_kernel_result! {
        // SAFETY: `private_data` was initialised by `open_callback` with a value returned by
        // `T::Wrapper::into_pointer`. `T::Wrapper::from_pointer` is only called by the `release`
        // callback, which the C API guarantees that will be called only when all references to
        // `file` have been released, so we know it can't be called while this function is running.
        let f = unsafe { T::Wrapper::borrow((*file).private_data) };
        let mut cmd = IoctlCommand::new(cmd as _, arg as _);
        let ret = T::oob_ioctl(&f, unsafe { &FileRef::from_ptr(file) }, &mut cmd)?;
        Ok(ret as _)
    }
}

unsafe extern "C" fn compat_ioctl_callback<T: FileOperations>(
    file: *mut bindings::file,
    cmd: c_types::c_uint,
    arg: c_types::c_ulong,
) -> c_types::c_long {
    from_kernel_result! {
        // SAFETY: `private_data` was initialised by `open_callback` with a value returned by
        // `T::Wrapper::into_pointer`. `T::Wrapper::from_pointer` is only called by the `release`
        // callback, which the C API guarantees that will be called only when all references to
        // `file` have been released, so we know it can't be called while this function is running.
        let f = unsafe { T::Wrapper::borrow((*file).private_data) };
        let mut cmd = IoctlCommand::new(cmd as _, arg as _);
        let ret = T::compat_ioctl(&f, unsafe { &FileRef::from_ptr(file) }, &mut cmd)?;
        Ok(ret as _)
    }
}

unsafe extern "C" fn mmap_callback<T: FileOperations>(
    file: *mut bindings::file,
    vma: *mut bindings::vm_area_struct,
) -> c_types::c_int {
    from_kernel_result! {
        // SAFETY: `private_data` was initialised by `open_callback` with a value returned by
        // `T::Wrapper::into_pointer`. `T::Wrapper::from_pointer` is only called by the `release`
        // callback, which the C API guarantees that will be called only when all references to
        // `file` have been released, so we know it can't be called while this function is running.
        let f = unsafe { T::Wrapper::borrow((*file).private_data) };
        T::mmap(&f, unsafe { &FileRef::from_ptr(file) }, unsafe { &mut *vma })?;
        Ok(0)
    }
}

unsafe extern "C" fn fsync_callback<T: FileOperations>(
    file: *mut bindings::file,
    start: bindings::loff_t,
    end: bindings::loff_t,
    datasync: c_types::c_int,
) -> c_types::c_int {
    from_kernel_result! {
        let start = start.try_into()?;
        let end = end.try_into()?;
        let datasync = datasync != 0;
        // SAFETY: `private_data` was initialised by `open_callback` with a value returned by
        // `T::Wrapper::into_pointer`. `T::Wrapper::from_pointer` is only called by the `release`
        // callback, which the C API guarantees that will be called only when all references to
        // `file` have been released, so we know it can't be called while this function is running.
        let f = unsafe { T::Wrapper::borrow((*file).private_data) };
        let res = T::fsync(&f, unsafe { &FileRef::from_ptr(file) }, start, end, datasync)?;
        Ok(res.try_into().unwrap())
    }
}

unsafe extern "C" fn poll_callback<T: FileOperations>(
    file: *mut bindings::file,
    wait: *mut bindings::poll_table_struct,
) -> bindings::__poll_t {
    // SAFETY: `private_data` was initialised by `open_callback` with a value returned by
    // `T::Wrapper::into_pointer`. `T::Wrapper::from_pointer` is only called by the `release`
    // callback, which the C API guarantees that will be called only when all references to `file`
    // have been released, so we know it can't be called while this function is running.
    let f = unsafe { T::Wrapper::borrow((*file).private_data) };
    match T::poll(&f, unsafe { &FileRef::from_ptr(file) }, unsafe {
        &PollTable::from_ptr(wait)
    }) {
        Ok(v) => v,
        Err(_) => bindings::POLLERR,
    }
}

unsafe extern "C" fn oob_poll_callback<T: FileOperations>(
    file: *mut bindings::file,
    wait: *mut bindings::oob_poll_wait,
) -> bindings::__poll_t {
    // SAFETY: `private_data` was initialised by `open_callback` with a value returned by
    // `T::Wrapper::into_pointer`. `T::Wrapper::from_pointer` is only called by the `release`
    // callback, which the C API guarantees that will be called only when all references to `file`
    // have been released, so we know it can't be called while this function is running.
    let f = unsafe { T::Wrapper::borrow((*file).private_data) };
    match T::oob_poll(&f, unsafe { &FileRef::from_ptr(file) }, unsafe {
        &OobPollWait::from_ptr(wait)
    }) {
        Ok(v) => v,
        Err(_) => bindings::POLLERR,
    }
}

pub(crate) struct FileOperationsVtable<A, T>(marker::PhantomData<A>, marker::PhantomData<T>);

impl<A: FileOpenAdapter, T: FileOpener<A::Arg>> FileOperationsVtable<A, T> {
    const VTABLE: bindings::file_operations = bindings::file_operations {
        open: Some(open_callback::<A, T>),
        release: Some(release_callback::<T>),
        read: if T::TO_USE.read {
            Some(read_callback::<T>)
        } else {
            None
        },
        write: if T::TO_USE.write {
            Some(write_callback::<T>)
        } else {
            None
        },
        llseek: if T::TO_USE.seek {
            Some(llseek_callback::<T>)
        } else {
            None
        },

        check_flags: None,
        compat_ioctl: if T::TO_USE.compat_ioctl {
            Some(compat_ioctl_callback::<T>)
        } else {
            None
        },
        copy_file_range: None,
        fallocate: None,
        fadvise: None,
        fasync: None,
        flock: None,
        flush: None,
        fsync: if T::TO_USE.fsync {
            Some(fsync_callback::<T>)
        } else {
            None
        },
        get_unmapped_area: None,
        iterate: None,
        iterate_shared: None,
        iopoll: None,
        lock: None,
        mmap: if T::TO_USE.mmap {
            Some(mmap_callback::<T>)
        } else {
            None
        },
        mmap_supported_flags: 0,
        owner: ptr::null_mut(),
        poll: if T::TO_USE.poll {
            Some(poll_callback::<T>)
        } else {
            None
        },
        read_iter: if T::TO_USE.read_iter {
            Some(read_iter_callback::<T>)
        } else {
            None
        },
        remap_file_range: None,
        sendpage: None,
        setlease: None,
        show_fdinfo: None,
        splice_read: None,
        splice_write: None,
        unlocked_ioctl: if T::TO_USE.ioctl {
            Some(unlocked_ioctl_callback::<T>)
        } else {
            None
        },
        write_iter: if T::TO_USE.write_iter {
            Some(write_iter_callback::<T>)
        } else {
            None
        },
        oob_read: if T::TO_USE.oob_read {
            Some(oob_read_callback::<T>)
        } else {
            None
        },
        oob_write: if T::TO_USE.oob_write {
            Some(oob_write_callback::<T>)
        } else {
            None
        },
        oob_ioctl: if T::TO_USE.oob_ioctl {
            Some(oob_ioctl_callback::<T>)
        } else {
            None
        },
        compat_oob_ioctl: None,
        oob_poll: if T::TO_USE.oob_poll {
            Some(oob_poll_callback::<T>)
        } else {
            None
        },
    };

    /// Builds an instance of [`struct file_operations`].
    ///
    /// # Safety
    ///
    /// The caller must ensure that the adapter is compatible with the way the device is registered.
    pub(crate) const unsafe fn build() -> &'static bindings::file_operations {
        &Self::VTABLE
    }
}

/// Represents which fields of [`struct file_operations`] should be populated with pointers.
pub struct ToUse {
    /// The `read` field of [`struct file_operations`].
    pub read: bool,

    /// The `read_iter` field of [`struct file_operations`].
    pub read_iter: bool,

    /// The `write` field of [`struct file_operations`].
    pub write: bool,

    /// The `write_iter` field of [`struct file_operations`].
    pub write_iter: bool,

    /// The `llseek` field of [`struct file_operations`].
    pub seek: bool,

    /// The `unlocked_ioctl` field of [`struct file_operations`].
    pub ioctl: bool,

    /// The `compat_ioctl` field of [`struct file_operations`].
    pub compat_ioctl: bool,

    /// The `fsync` field of [`struct file_operations`].
    pub fsync: bool,

    /// The `mmap` field of [`struct file_operations`].
    pub mmap: bool,

    /// The `poll` field of [`struct file_operations`].
    pub poll: bool,

    /// The `oob_read` field of [`struct file_operations`].
    pub oob_read: bool,

    /// The `oob_write` field of [`struct file_operations`].
    pub oob_write: bool,

    /// The `oob_ioctl` field of [`struct file_operations`].
    pub oob_ioctl: bool,

    /// The `compat_oob_ioctl` field of [`struct file_operations`].
    pub compat_oob_ioctl: bool,

    /// The `oob_poll` field of [`struct file_operations`].
    pub oob_poll: bool,
}

/// A constant version where all values are to set to `false`, that is, all supported fields will
/// be set to null pointers.
pub const USE_NONE: ToUse = ToUse {
    read: false,
    read_iter: false,
    write: false,
    write_iter: false,
    seek: false,
    ioctl: false,
    compat_ioctl: false,
    fsync: false,
    mmap: false,
    poll: false,
    oob_read: false,
    oob_write: false,
    oob_ioctl: false,
    compat_oob_ioctl: false,
    oob_poll: false,
};

/// Defines the [`FileOperations::TO_USE`] field based on a list of fields to be populated.
#[macro_export]
macro_rules! declare_file_operations {
    () => {
        const TO_USE: $crate::file_operations::ToUse = $crate::file_operations::USE_NONE;
    };
    ($($i:ident),+) => {
        const TO_USE: kernel::file_operations::ToUse =
            $crate::file_operations::ToUse {
                $($i: true),+ ,
                ..$crate::file_operations::USE_NONE
            };
    };
}

/// Allows the handling of ioctls defined with the `_IO`, `_IOR`, `_IOW`, and `_IOWR` macros.
///
/// For each macro, there is a handler function that takes the appropriate types as arguments.
pub trait IoctlHandler: Sync {
    /// The type of the first argument to each associated function.
    type Target;

    /// Handles ioctls defined with the `_IO` macro, that is, with no buffer as argument.
    fn pure(_this: &Self::Target, _file: &File, _cmd: u32, _arg: usize) -> Result<i32> {
        Err(Error::EINVAL)
    }

    /// Handles ioctls defined with the `_IOR` macro, that is, with an output buffer provided as
    /// argument.
    fn read(
        _this: &Self::Target,
        _file: &File,
        _cmd: u32,
        _writer: &mut UserSlicePtrWriter,
    ) -> Result<i32> {
        Err(Error::EINVAL)
    }

    /// Handles ioctls defined with the `_IOW` macro, that is, with an input buffer provided as
    /// argument.
    fn write(
        _this: &Self::Target,
        _file: &File,
        _cmd: u32,
        _reader: &mut UserSlicePtrReader,
    ) -> Result<i32> {
        Err(Error::EINVAL)
    }

    /// Handles ioctls defined with the `_IOWR` macro, that is, with a buffer for both input and
    /// output provided as argument.
    fn read_write(
        _this: &Self::Target,
        _file: &File,
        _cmd: u32,
        _data: UserSlicePtr,
    ) -> Result<i32> {
        Err(Error::EINVAL)
    }
}

/// Represents an ioctl command.
///
/// It can use the components of an ioctl command to dispatch ioctls using
/// [`IoctlCommand::dispatch`].
pub struct IoctlCommand {
    /// `cmd`: a u32 representing the command code.
    pub cmd: u32,
    /// `arg`: a usize representing the argument of the command.
    pub arg: usize,
    /// `user_slice`: an optional `UserSlicePtr` representing a user space memory slice.
    pub user_slice: Option<UserSlicePtr>,
}

impl IoctlCommand {
    /// Constructs a new [`IoctlCommand`].
    fn new(cmd: u32, arg: usize) -> Self {
        let size = (cmd >> bindings::_IOC_SIZESHIFT) & bindings::_IOC_SIZEMASK;

        // SAFETY: We only create one instance of the user slice per ioctl call, so TOCTOU issues
        // are not possible.
        let user_slice = Some(unsafe { UserSlicePtr::new(arg as _, size as _) });
        Self {
            cmd,
            arg,
            user_slice,
        }
    }

    /// Dispatches the given ioctl to the appropriate handler based on the value of the command. It
    /// also creates a [`UserSlicePtr`], [`UserSlicePtrReader`], or [`UserSlicePtrWriter`]
    /// depending on the direction of the buffer of the command.
    ///
    /// It is meant to be used in implementations of [`FileOperations::ioctl`] and
    /// [`FileOperations::compat_ioctl`].
    pub fn dispatch<T: IoctlHandler>(&mut self, handler: &T::Target, file: &File) -> Result<i32> {
        let dir = (self.cmd >> bindings::_IOC_DIRSHIFT) & bindings::_IOC_DIRMASK;
        if dir == bindings::_IOC_NONE {
            return T::pure(handler, file, self.cmd, self.arg);
        }

        let data = self.user_slice.take().ok_or(Error::EINVAL)?;
        const READ_WRITE: u32 = bindings::_IOC_READ | bindings::_IOC_WRITE;
        match dir {
            bindings::_IOC_WRITE => T::write(handler, file, self.cmd, &mut data.reader()),
            bindings::_IOC_READ => T::read(handler, file, self.cmd, &mut data.writer()),
            READ_WRITE => T::read_write(handler, file, self.cmd, data),
            _ => Err(Error::EINVAL),
        }
    }

    /// Returns the raw 32-bit value of the command and the ptr-sized argument.
    pub fn raw(&self) -> (u32, usize) {
        (self.cmd, self.arg)
    }
}

/// Trait for extracting file open arguments from kernel data structures.
///
/// This is meant to be implemented by registration managers.
pub trait FileOpenAdapter {
    /// The type of argument this adapter extracts.
    type Arg;

    /// Converts untyped data stored in [`struct inode`] and [`struct file`] (when [`struct
    /// file_operations::open`] is called) into the given type. For example, for `miscdev`
    /// devices, a pointer to the registered [`struct miscdev`] is stored in [`struct
    /// file::private_data`].
    ///
    /// # Safety
    ///
    /// This function must be called only when [`struct file_operations::open`] is being called for
    /// a file that was registered by the implementer.
    unsafe fn convert(_inode: *mut bindings::inode, _file: *mut bindings::file)
        -> *const Self::Arg;
}

/// Trait for implementers of kernel files.
///
/// In addition to the methods in [`FileOperations`], implementers must also provide
/// [`FileOpener::open`] with a customised argument. This allows a single implementation of
/// [`FileOperations`] to be used for different types of registrations, for example, `miscdev` and
/// `chrdev`.
pub trait FileOpener<T: ?Sized>: FileOperations {
    /// Creates a new instance of this file.
    ///
    /// Corresponds to the `open` function pointer in `struct file_operations`.
    fn open(context: &T, fileref: &File) -> Result<Self::Wrapper>;
}

impl<T: FileOperations<Wrapper = Box<T>> + Default> FileOpener<()> for T {
    fn open(_: &(), _fileref: &File) -> Result<Self::Wrapper> {
        Ok(Box::try_new(T::default())?)
    }
}

impl<T: FileOperations<Wrapper = Box<T>> + Default> FileOpener<u8> for T {
    fn open(_: &u8, _fileref: &File) -> Result<Self::Wrapper> {
        Ok(Box::try_new(T::default())?)
    }
}

/// Corresponds to the kernel's `struct file_operations`.
///
/// You implement this trait whenever you would create a `struct file_operations`.
///
/// File descriptors may be used from multiple threads/processes concurrently, so your type must be
/// [`Sync`]. It must also be [`Send`] because [`FileOperations::release`] will be called from the
/// thread that decrements that associated file's refcount to zero.
pub trait FileOperations: Send + Sync + Sized {
    /// The methods to use to populate [`struct file_operations`].
    const TO_USE: ToUse;

    /// The pointer type that will be used to hold ourselves.
    type Wrapper: PointerWrapper = Box<Self>;

    /// Cleans up after the last reference to the file goes away.
    ///
    /// Note that the object is moved, so it will be freed automatically unless the implementation
    /// moves it elsewhere.
    ///
    /// Corresponds to the `release` function pointer in `struct file_operations`.
    fn release(_obj: Self::Wrapper, _file: &File) {}

    /// Reads data from this file to the caller's buffer.
    ///
    /// Corresponds to the `read` and `read_iter` function pointers in `struct file_operations`.
    fn read<T: IoBufferWriter>(
        _this: &<<Self::Wrapper as PointerWrapper>::Borrowed as Deref>::Target,
        _file: &File,
        _data: &mut T,
        _offset: u64,
    ) -> Result<usize> {
        Err(Error::EINVAL)
    }

    /// Reads data from this file to the caller's buffer.
    ///
    /// Corresponds to the `oob_read` function pointers in `struct file_operations`.
    fn oob_read<T: IoBufferWriter>(
        _this: &<<Self::Wrapper as PointerWrapper>::Borrowed as Deref>::Target,
        _file: &File,
        _data: &mut T,
    ) -> Result<usize> {
        Err(Error::EINVAL)
    }

    /// Writes data from the caller's buffer to this file.
    ///
    /// Corresponds to the `write` and `write_iter` function pointers in `struct file_operations`.
    fn write<T: IoBufferReader>(
        _this: &<<Self::Wrapper as PointerWrapper>::Borrowed as Deref>::Target,
        _file: &File,
        _data: &mut T,
        _offset: u64,
    ) -> Result<usize> {
        Err(Error::EINVAL)
    }

    /// Writes data from the caller's buffer to this file.
    ///
    /// Corresponds to the `oob_write` function pointers in `struct file_operations`.
    fn oob_write<T: IoBufferReader>(
        _this: &<<Self::Wrapper as PointerWrapper>::Borrowed as Deref>::Target,
        _file: &File,
        _data: &mut T,
    ) -> Result<usize> {
        Err(Error::EINVAL)
    }

    /// Changes the position of the file.
    ///
    /// Corresponds to the `llseek` function pointer in `struct file_operations`.
    fn seek(
        _this: &<<Self::Wrapper as PointerWrapper>::Borrowed as Deref>::Target,
        _file: &File,
        _offset: SeekFrom,
    ) -> Result<u64> {
        Err(Error::EINVAL)
    }

    /// Performs IO control operations that are specific to the file.
    ///
    /// Corresponds to the `unlocked_ioctl` function pointer in `struct file_operations`.
    fn ioctl(
        _this: &<<Self::Wrapper as PointerWrapper>::Borrowed as Deref>::Target,
        _file: &File,
        _cmd: &mut IoctlCommand,
    ) -> Result<i32> {
        Err(Error::EINVAL)
    }

    /// Performs IO control operations that are specific to the file.
    ///
    /// Corresponds to the `oob_ioctl` function pointer in `struct file_operations`.
    fn oob_ioctl(
        _this: &<<Self::Wrapper as PointerWrapper>::Borrowed as Deref>::Target,
        _file: &File,
        _cmd: &mut IoctlCommand,
    ) -> Result<i32> {
        Err(Error::EINVAL)
    }

    /// Performs 32-bit IO control operations on that are specific to the file on 64-bit kernels.
    ///
    /// Corresponds to the `compat_ioctl` function pointer in `struct file_operations`.
    fn compat_ioctl(
        _this: &<<Self::Wrapper as PointerWrapper>::Borrowed as Deref>::Target,
        _file: &File,
        _cmd: &mut IoctlCommand,
    ) -> Result<i32> {
        Err(Error::EINVAL)
    }

    /// Performs 32-bit IO control operations on that are specific to the file on 64-bit kernels.
    ///
    /// Corresponds to the `compat_oob_ioctl` function pointer in `struct file_operations`.
    fn compat_oob_ioctl(
        _this: &<<Self::Wrapper as PointerWrapper>::Borrowed as Deref>::Target,
        _file: &File,
        _cmd: &mut IoctlCommand,
    ) -> Result<i32> {
        Err(Error::EINVAL)
    }

    /// Syncs pending changes to this file.
    ///
    /// Corresponds to the `fsync` function pointer in `struct file_operations`.
    fn fsync(
        _this: &<<Self::Wrapper as PointerWrapper>::Borrowed as Deref>::Target,
        _file: &File,
        _start: u64,
        _end: u64,
        _datasync: bool,
    ) -> Result<u32> {
        Err(Error::EINVAL)
    }

    /// Maps areas of the caller's virtual memory with device/file memory.
    ///
    /// Corresponds to the `mmap` function pointer in `struct file_operations`.
    /// TODO: wrap `vm_area_struct` so that we don't have to expose it.
    fn mmap(
        _this: &<<Self::Wrapper as PointerWrapper>::Borrowed as Deref>::Target,
        _file: &File,
        _vma: &mut bindings::vm_area_struct,
    ) -> Result {
        Err(Error::EINVAL)
    }

    /// Checks the state of the file and optionally registers for notification when the state
    /// changes.
    ///
    /// Corresponds to the `poll` function pointer in `struct file_operations`.
    fn poll(
        _this: &<<Self::Wrapper as PointerWrapper>::Borrowed as Deref>::Target,
        _file: &File,
        _table: &PollTable,
    ) -> Result<u32> {
        Ok(bindings::POLLIN | bindings::POLLOUT | bindings::POLLRDNORM | bindings::POLLWRNORM)
    }

    /// Checks the state of the file and optionally registers for notification when the state
    /// changes.
    ///
    /// Corresponds to the `poll` function pointer in `struct file_operations`.
    fn oob_poll(
        _this: &<<Self::Wrapper as PointerWrapper>::Borrowed as Deref>::Target,
        _file: &File,
        _wait: &OobPollWait,
    ) -> Result<u32> {
        Ok(bindings::POLLIN | bindings::POLLOUT | bindings::POLLRDNORM | bindings::POLLWRNORM)
    }
}
