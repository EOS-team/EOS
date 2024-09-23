use core::{convert::TryInto, result::Result::Ok};

use crate::factory::{CloneData, RrosFactory, RrosFactoryInside};

use kernel::{
    bindings,
    device::DeviceType,
    file::File,
    file_operations::{FileOpener, FileOperations, IoctlCommand},
    io_buffer::{IoBufferWriter, WritableToBytes},
    memory_rros::{RROS_SHARED_HEAP, RROS_SHM_SIZE},
    mm::{remap_pfn_range, PAGE_SHARED},
    prelude::*,
    str::CStr,
    sync::SpinLock,
};

pub const CONFIG_RROS_NR_CONTROL: usize = 0;

pub static mut RROS_CONTROL_FACTORY: SpinLock<RrosFactory> = unsafe {
    SpinLock::new(RrosFactory {
        name: CStr::from_bytes_with_nul_unchecked("control\0".as_bytes()),
        nrdev: CONFIG_RROS_NR_CONTROL,
        build: None,
        dispose: None,
        attrs: None,
        flags: crate::factory::RrosFactoryType::SINGLE,
        inside: Some(RrosFactoryInside {
            type_: DeviceType::new(),
            class: None,
            cdev: None,
            device: None,
            sub_rdev: None,
            kuid: None,
            kgid: None,
            minor_map: None,
            index: None,
            name_hash: None,
            hash_lock: None,
            register: None,
        }),
    })
};

pub struct ControlOps;

impl FileOpener<u8> for ControlOps {
    fn open(shared: &u8, _fileref: &File) -> Result<Self::Wrapper> {
        // there should be some checks
        let mut data = CloneData::default();
        data.ptr = shared as *const u8 as *mut u8;
        pr_debug!("open control device success");
        Ok(Box::try_new(data)?)
    }
}

impl FileOperations for ControlOps {
    kernel::declare_file_operations!(oob_ioctl, ioctl, mmap);

    type Wrapper = Box<CloneData>;

    // fn ioctl
    fn ioctl(_this: &CloneData, file: &File, cmd: &mut IoctlCommand) -> Result<i32> {
        pr_debug!("I'm the ioctl ops of the control factory");
        // cmd.dispatch::<Self>(this, file)
        let ret = control_ioctl(file, cmd);
        pr_debug!("the value of ret is {}", ret.unwrap());
        ret
    }

    // fn oob_ioctl
    fn oob_ioctl(_this: &CloneData, file: &File, cmd: &mut IoctlCommand) -> Result<i32> {
        pr_debug!("I'm the ioctl ops of the control factory");
        let ret = control_common_ioctl(file, cmd);
        ret
    }

    fn mmap(_this: &CloneData, file: &File, vma: &mut bindings::vm_area_struct) -> Result {
        pr_debug!("I'm the mmap ops of the control factory");
        let ret = control_mmap(file, vma);
        ret
    }
}

#[repr(C)]
pub struct RrosCoreInfo {
    abi_base: u32,
    abi_current: u32,
    fpu_features: u32,
    shm_size: u64,
}

impl RrosCoreInfo {
    pub fn new() -> Self {
        RrosCoreInfo {
            abi_base: 0,
            abi_current: 0,
            fpu_features: 0,
            shm_size: 0,
        }
    }
}

unsafe impl WritableToBytes for RrosCoreInfo {}

#[repr(C)]
#[allow(dead_code)]
pub struct RrosCpuState {
    cpu: u32,
    state_ptr: u64,
}

// pub const RROS_CONTROL_IOCBASE: u32 = 'C';

pub const RROS_ABI_BASE: u32 = 23;
pub const RROS_ABI_LEVEL: u32 = 26;

pub const RROS_CTLIOC_GET_COREINFO: u32 = 2149073664;
// pub const RROS_CTLIOC_SCHEDCTL: u32 = 3222815489;
// pub const RROS_CTLIOC_GET_CPUSTATE: u32 = 2148549378;

extern "C" {
    fn rust_helper_pa(x: usize) -> usize;
}

fn control_ioctl(file: &File, cmd: &mut IoctlCommand) -> Result<i32> {
    let mut info = RrosCoreInfo::new();
    match cmd.cmd {
        RROS_CTLIOC_GET_COREINFO => {
            info.abi_base = RROS_ABI_BASE;
            info.abi_current = RROS_ABI_LEVEL;
            // in arch/arm64/include/asm/rros/fptest.h
            // TODO: There should be a function rros_detect_fpu() related to the arm64 architecture, the result of the function is 0.
            info.fpu_features = 0;
            unsafe {
                pr_debug!(
                    "the value of info.shm_size and RROS_SHM_SIZE is {}, {}",
                    info.shm_size,
                    RROS_SHM_SIZE
                )
            };
            unsafe {
                info.shm_size = RROS_SHM_SIZE as u64;
            }
            unsafe {
                pr_debug!(
                    "the value of info.shm_size and RROS_SHM_SIZE is {}, {}",
                    info.shm_size,
                    RROS_SHM_SIZE
                )
            };
            // ret = cmd.user_slice.take().ok_or(Error::EINVAL).writer();
            let data = cmd.user_slice.take().ok_or(Error::EINVAL);
            data.unwrap().writer().write(&info)?;
            Ok(0)
        }
        _ => control_common_ioctl(file, cmd),
    }
}

fn control_common_ioctl(_file: &File, _cmd: &mut IoctlCommand) -> Result<i32> {
    Ok(0)
}

fn control_mmap(_file: &File, vma: &mut bindings::vm_area_struct) -> Result {
    let p = unsafe { RROS_SHARED_HEAP.membase };
    let pfn: usize = unsafe { rust_helper_pa(p as usize) } >> bindings::PAGE_SHIFT;
    let len: usize = (vma.vm_end - vma.vm_start) as usize;

    if len != unsafe { RROS_SHM_SIZE } {
        return Err(Error::EINVAL);
    }

    remap_pfn_range(
        vma as *mut bindings::vm_area_struct,
        vma.vm_start,
        pfn.try_into().unwrap(),
        len.try_into().unwrap(),
        PAGE_SHARED,
    );
    Ok(())
}
