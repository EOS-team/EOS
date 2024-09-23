use core::{
    cell::RefCell, clone::Clone, convert::TryInto, default::Default, mem::size_of, ptr,
    result::Result::Ok,
};

use crate::{clock, control, file::RrosFileBinding, observable, poll, proxy, thread, xbuf};

use alloc::rc::Rc;

use kernel::{
    bindings,
    bitmap::{self, bitmap_zalloc},
    c_str, c_types, chrdev, class,
    device::{self, DeviceType},
    file::{self, fd_install, File},
    file_operations::{FileOpener, FileOperations, IoctlCommand},
    fs::{self, Filename},
    io_buffer::IoBufferWriter,
    irq_work, kernelh,
    prelude::*,
    rbtree, spinlock_init,
    str::CStr,
    sync::{Lock, SpinLock},
    sysfs, types,
    uidgid::{self, KgidT, KuidT},
    workqueue, ThisModule,
};

extern "C" {
    #[allow(improper_ctypes)]
    fn rust_helper_put_user(val: u32, ptr: *mut u32) -> c_types::c_int;
}

type FundleT = u32;

#[derive(Clone, Copy)]
pub enum RrosFactoryType {
    Invalid = 0,
    CLONE = 1,
    SINGLE = 2,
}
pub const RROS_OBSERVABLE_CLONE_FLAGS: i32 =
    RROS_CLONE_PUBLIC | RROS_CLONE_OBSERVABLE | RROS_CLONE_MASTER;
pub const RROS_THREAD_CLONE_FLAGS: i32 =
    RROS_CLONE_PUBLIC | RROS_CLONE_OBSERVABLE | RROS_CLONE_MASTER;
pub const RROS_CLONE_PUBLIC: i32 = 1 << 16;
#[allow(dead_code)]
pub const RROS_CLONE_PRIVATE: i32 = 0 << 16;
pub const RROS_CLONE_OBSERVABLE: i32 = 1 << 17;
#[allow(dead_code)]
const RROS_CLONE_NONBLOCK: i32 = 1 << 18;
pub const RROS_CLONE_MASTER: i32 = 1 << 19;
pub const RROS_CLONE_INPUT: i32 = 1 << 20;
pub const RROS_CLONE_OUTPUT: i32 = 1 << 21;
#[allow(dead_code)]
const RROS_CLONE_COREDEV: i32 = 1 << 31;
#[allow(dead_code)]
const RROS_CLONE_MASK: i32 = (-1 << 16) & !RROS_CLONE_COREDEV;

#[allow(dead_code)]
const RROS_DEVHASH_BITS: i32 = 8;
pub const NR_FACTORIES: usize = 8;
#[allow(dead_code)]
const NR_CLOCKNR: usize = 8;
const RROS_NO_HANDLE: FundleT = 0x00000000;
const NAME_HASH_TABLE_SIZE: u32 = 1 << 8;

const CONFIG_RROS: usize = 0; // unknown
#[allow(dead_code)]
const RROS_MUTEX_FLCLAIM: FundleT = 0x80000000;
#[allow(dead_code)]
const RROS_MUTEX_FLCEIL: FundleT = 0x40000000;
#[allow(dead_code)]
const RROS_HANDLE_INDEX_MASK: FundleT = RROS_MUTEX_FLCEIL | RROS_MUTEX_FLCLAIM;

pub struct RrosIndex {
    // #[allow(dead_code)]
    rbtree: SpinLock<rbtree::RBTree<FundleT, Rc<RefCell<RrosElement>>>>, // TODO: modify the u32.
    // lock: SpinLock<i32>,
    // #[allow(dead_code)]
    generator: FundleT,
}

pub struct RrosFactoryInside {
    pub type_: device::DeviceType,
    pub class: Option<Arc<class::Class>>,
    pub cdev: Option<chrdev::Cdev>,
    pub device: Option<device::Device>,
    pub sub_rdev: Option<class::DevT>,
    pub kuid: Option<uidgid::KuidT>,
    pub kgid: Option<uidgid::KgidT>,
    pub minor_map: Option<u64>,
    pub index: Option<RrosIndex>,
    pub name_hash: Option<[types::HlistHead; NAME_HASH_TABLE_SIZE as usize]>,
    pub hash_lock: Option<SpinLock<i32>>,
    // FIXME: This const should not be limited to 256. But the rust compiler does not support it.
    pub register: Option<Pin<Box<chrdev::Registration<16>>>>,
}

trait RrosFops: FileOperations {
    fn hello(&self, _file: &File) -> i32 {
        0
    }
    // fn release(&self, file: &File) -> Result {
    //     Ok(0)
    // }
    // fn read(&self, file: &File, buf: &mut IoBufferWriter) -> Result {
    //     Ok(0)
    // }
    // fn write(&self, file: &File, buf: &mut IoBufferWriter) -> Result {
    //     Ok(0)
    // }
    // fn poll(&self, file: &File, wait: &mut bindings::poll_table_struct) -> Result {
    //     Ok(0)
    // }
    // fn ioctl(&self, file: &File, cmd: u32, arg: u64) -> Result {
    //     Ok(0)
    // }
    // fn mmap(&self, file: &File, vma: &mut bindings::vm_area_struct) -> Result {
    //     Ok(0)
    // }
    // fn fasync(&self, file: &File, fd: i32, flag: i32) -> Result {
    //     Ok(0)
    // }
    // fn lock(&self, file: &File, cmd: i32, lock: &mut bindings::flock) -> Result {
    //     Ok(0)
    // }
    // fn compat_ioctl(&self, file: &File, cmd: u32, arg: u64) -> Result {
    //     Ok(0)
    // }
    // fn flush(&self, file: &File) -> Result {
    //     Ok(0)
    // }
    // fn fsync(&self, file: &File, datasync: i32) -> Result {
    //     Ok(0)
    // }
    // fn fallocate(&self, file: &File, mode: i32, offset: i64, len: i64) -> Result {
    //     Ok(0)
    // }
    // fn fadvise(&self, file: &File, offset: i64, len: i64, advice: i32) -> Result {
    //     Ok(0)
    // }
    // fn sendpage(&self, file: &File, page: &mut bindings::page, offset: i32, size: i32, more: &mut i32) -> Result {
    //     Ok(0)
    // }
    // fn splice_write(&self, file: &File, pipe: &mut bindings::pipe_inode_info, splice_desc: &mut bindings::splice_desc) -> Result {
    //     Ok(0)
    // }
}

pub struct RrosFactory {
    pub name: &'static CStr,
    // pub fops: Option<&'static dyn FileOperations>,
    // pub fops: RustFile, // This entry is attached to the cdev in the rfl. It can be omitted in the factory struct.
    pub nrdev: usize,
    pub build: Option<
        fn(
            fac: &'static mut SpinLock<RrosFactory>,
            uname: &'static CStr,
            u_attrs: Option<*mut u8>,
            clone_flags: i32,
            state_offp: &u32,
        ) -> Rc<RefCell<RrosElement>>,
    >,
    pub dispose: Option<fn(RrosElement)>,
    pub attrs: Option<sysfs::AttributeGroup>, // Use an `Option` for the time being.
    pub flags: RrosFactoryType,
    pub inside: Option<RrosFactoryInside>,
    // pub fops: PhantomData<T>,
}

pub static mut RROS_FACTORY: SpinLock<RrosFactory> = unsafe {
    SpinLock::new(RrosFactory {
        name: CStr::from_bytes_with_nul_unchecked("RROS_DEV\0".as_bytes()),
        // fops: Some(&Tmpops),
        nrdev: CONFIG_RROS,
        build: None,
        dispose: None,
        attrs: None, //sysfs::attribute_group::new(),
        flags: RrosFactoryType::Invalid,
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

struct Tmpops;

impl FileOperations for Tmpops {
    kernel::declare_file_operations!();

    fn read<T: IoBufferWriter>(
        _this: &Self,
        _file: &File,
        _data: &mut T,
        _offset: u64,
    ) -> Result<usize> {
        pr_debug!("I'm the read ops of the rros tmp factory.");
        Ok(1)
    }
}

pub struct RrosElement {
    pub rcu_head: types::RcuHead,
    pub factory: &'static mut SpinLock<RrosFactory>,
    pub cdev: Option<chrdev::Cdev>,
    pub dev: Option<device::Device>,
    pub devname: Option<fs::Filename>,
    pub minor: u64,
    pub refs: i32,
    pub zombie: bool,
    pub ref_lock: SpinLock<i32>,
    pub fundle: FundleT,
    pub clone_flags: i32,
    // pub struct rb_node index_node;// TODO: in rfl rb_node is not embedded in the struct.
    pub irq_work: irq_work::IrqWork,
    pub work: workqueue::Work,
    pub hash: types::HlistNode,
    pub fpriv: RrosElementfpriv,
    pub pointer: *mut u8,
}

impl RrosElement {
    pub fn new() -> Result<Self> {
        Ok(Self {
            rcu_head: types::RcuHead::new(),
            factory: unsafe { &mut RROS_FACTORY },
            cdev: None,
            dev: None,
            devname: None,
            minor: 0,
            refs: 0,
            zombie: false,
            ref_lock: unsafe { kernel::sync::SpinLock::<i32>::new(0) },
            fundle: 0,
            clone_flags: 0,
            irq_work: irq_work::IrqWork::new(),
            work: unsafe { workqueue::Work::new() },
            hash: types::HlistNode::new(),
            fpriv: RrosElementfpriv::new(),
            pointer: 0 as *mut u8,
        })
    }
}
pub struct RrosElementfpriv {
    pub filp: Option<File>,
    pub efd: file::FileDescriptorReservation,
}

impl RrosElementfpriv {
    fn new() -> Self {
        Self {
            filp: None,
            efd: file::FileDescriptorReservation { fd: 620 },
        }
    }
}

pub enum RustFile {
    #[allow(dead_code)]
    Clock(clock::RustFileClock),
}

pub struct RrosDevnode;

impl device::ClassDevnode for RrosDevnode {
    fn devnode(dev: &mut device::Device, _mode: &mut u16) -> *mut c_types::c_char {
        kernelh::_kasprintf_1(
            bindings::GFP_KERNEL,
            c_str!("rros/%s").as_char_ptr(),
            dev.dev_name(),
        )
    }
}

pub struct FactoryTypeDevnode;

impl device::Devnode for FactoryTypeDevnode {
    fn devnode(
        dev: &mut device::Device,
        _mode: &mut u16,
        uid: Option<&mut KuidT>,
        gid: Option<&mut KgidT>,
    ) -> *mut c_types::c_char {
        // TODO: currently we use raw pointer
        let element: Option<&RrosElement> = dev.get_drvdata();
        if let Some(e) = element {
            let inside = unsafe { (*(e.factory.locked_data().get())).inside.as_ref().unwrap() };
            if let Some(uid) = uid {
                if let Some(e_uid) = inside.kuid.as_ref() {
                    *uid = *e_uid;
                }
            }
            if let Some(gid) = gid {
                if let Some(e_gid) = inside.kgid.as_ref() {
                    *gid = *e_gid;
                }
            }
        }
        kernelh::_kasprintf_2(
            bindings::GFP_KERNEL,
            c_str!("rros/%s/%s").as_char_ptr(),
            dev.dev_type().unwrap().get_name(),
            dev.dev_name(),
        )
    }
}

fn create_element_device(
    e: Rc<RefCell<RrosElement>>,
    fac: &'static mut SpinLock<RrosFactory>,
) -> Result<usize> {
    let mut fac_lock = unsafe { (*fac.locked_data().get()).inside.as_mut() };
    let mut rdev: class::DevT = class::DevT::new(0);

    let _hlen: u64 = fs::hashlen_string(
        c_str!("RROS").as_char_ptr(),
        e.clone().borrow_mut().devname.as_mut().unwrap() as *mut Filename,
    );

    let _res = match fac_lock {
        Some(ref mut inside) => {
            let _ret = inside.hash_lock.as_ref().unwrap().lock();

            // hash_for_each_possible(fac->name_hash, n, hash, hlen)
            // if (!strcmp(n->devname->name, e->devname->name)) {
            //     mutex_unlock(&fac->hash_lock);
            //     goto fail_hash;
            // }

            // hash_add(fac->name_hash, &e->hash, hlen);

            unsafe {
                inside.hash_lock.as_ref().unwrap().unlock();
            }

            0
        }
        None => 1,
    };

    let _res = do_element_visibility(e.clone(), fac, &mut rdev);
    if !rros_element_is_public(e.clone()) && !rros_element_has_coredev(e.clone()) {
        let e_clone = e.clone();
        let mut e_mut = e_clone.borrow_mut();
        e_mut.refs += 1;
        let filp = e_mut.fpriv.filp.as_mut().unwrap().get_ptr();
        pr_debug!("the address of filp location 7 is {:p}, {:p}", filp, &filp);

        // if (!rros_element_is_public(e) && !rros_element_has_coredev(e)) {
        // if rros_element_is_public(e.clone()) == false && rros_element_has_coredev(e.clone()) == false {
        // unsafe { bindings::fd_install(e_mut.fpriv.efd.reserved_fd(), filp) };
        // e.fpriv.efd.commit(File{ptr: filp});
        // }
        fd_install(e_mut.fpriv.efd.reserved_fd(), filp);
        pr_debug!("the address of filp location 8 is {:p}, {:p}", filp, &filp);
    }

    Ok(0)
}

#[allow(dead_code)]
fn rros_element_is_public(e: Rc<RefCell<RrosElement>>) -> bool {
    let e_clone = e.clone();
    let e_borrow = e_clone.borrow();

    (e_borrow.clone_flags & RROS_CLONE_PUBLIC) == RROS_CLONE_PUBLIC
}

pub fn rros_element_is_observable(e: Rc<RefCell<RrosElement>>) -> bool {
    let e_clone = e.clone();
    let e_borrow = e_clone.borrow();

    (e_borrow.clone_flags & RROS_CLONE_OBSERVABLE) == RROS_CLONE_OBSERVABLE
}

#[allow(dead_code)]
fn rros_element_has_coredev(e: Rc<RefCell<RrosElement>>) -> bool {
    let e_clone = e.clone();
    let e_borrow = e_clone.borrow();

    (e_borrow.clone_flags & RROS_CLONE_COREDEV) == RROS_CLONE_COREDEV
}

fn do_element_visibility(
    e: Rc<RefCell<RrosElement>>,
    fac: &'static mut SpinLock<RrosFactory>,
    _rdev: &mut class::DevT,
) -> Result<usize> {
    // static int do_element_visibility(struct rros_element *e,
    //     struct rros_factory *fac,
    //     dev_t *rdev)
    // {

    let e_clone = e.clone();

    //     let core_dev_res = rros_element_has_coredev(e.clone());
    //     let mm_res = Task::current().kernel();
    //     if !core_dev_res && !mm_res {
    //         e_mut.clone_flags |= RROS_CLONE_COREDEV;
    //     }

    //     let mut fac_lock = fac.lock();
    //     let res = rros_element_is_public(e.clone());
    //     if res == true {
    //         let fac_res = match fac_lock.inside {
    //             Some(ref mut inside) => {
    //                 fac_lock
    //                     .inside
    //                     .as_mut()
    //                     .unwrap()
    //                     .register
    //                     .as_mut()
    //                     .unwrap()
    //                     .as_mut()
    //                     .register::<clock::RustFileClock>()?; //TODO: change this to fac->fops
    //                 0
    //             }
    //             None => 1,
    //         };
    //         match fac_res {
    //             1 => return Err(kernel::Error::EINVAL),
    //             _ => return Ok(0),
    //         }
    //         // *rdev = MKDEV(MAJOR(fac->sub_rdev), e->minor);
    //         // cdev_init(&e->cdev, fac->fops);
    //         // return cdev_add(&e->cdev, *rdev, 1);
    //     }

    let res = rros_element_has_coredev(e.clone());
    if res == true {
        return Ok(0);
    }
    let mut e_mut = e_clone.borrow_mut();

    // struct file *filp;
    // int ret, efd;

    // if (RROS_WARN_ON(CORE, !rros_element_has_coredev(e) && !current->mm))
    // e->clone_flags |= RROS_CLONE_COREDEV;

    // /*
    // * Unlike a private one, a publically visible element exports
    // * a cdev in the /dev/rros hierarchy so that any process can
    // * see it.  Both types are backed by a kernel device object so
    // * that we can export their state to userland via /sysfs.
    // */
    // if (rros_element_is_public(e)) {
    // *rdev = MKDEV(MAJOR(fac->sub_rdev), e->minor);
    // cdev_init(&e->cdev, fac->fops);
    // return cdev_add(&e->cdev, *rdev, 1);
    // }

    // *rdev = MKDEV(0, e->minor);

    // if (rros_element_has_coredev(e))
    // return 0;

    //  /*
    // 	 * Create a private user element, passing the real fops so
    // 	 * that FMODE_CAN_READ/WRITE are set accordingly by the vfs.
    // 	 */
    // let reg = unsafe{(*fac.locked_data().get()).inside.as_mut().unwrap().register.as_mut()};
    let reg = unsafe {
        (*fac.locked_data().get())
            .inside
            .as_mut()
            .unwrap()
            .register
            .as_mut()
    };
    // let reg = unsafe{&mut crate::Rros::factory};
    let inner = reg.unwrap().inner.as_ref();
    let cdev = (inner.unwrap().cdevs[0]).as_ref();
    let ops = unsafe { (*(cdev.unwrap().0)).ops };
    let filp = File::anon_inode_getfile(
        e_mut.devname.as_mut().unwrap().get_name(),
        ops,
        ptr::null_mut(),
        bindings::O_RDWR as i32,
    );

    pr_debug!("the address of filp location 1 is {:p}, {:p}", filp, &filp);
    // /*
    // * Create a private user element, passing the real fops so
    // * that FMODE_CAN_READ/WRITE are set accordingly by the vfs.
    // */
    // filp = anon_inode_getfile(rros_element_name(e), fac->fops,
    //     NULL, O_RDWR);
    // if (IS_ERR(filp)) {
    // ret = PTR_ERR(filp);
    // return ret;
    // }

    // /*
    // * Now switch to dummy fops temporarily, until calling
    // * rros_release_element() is safe for filp, meaning once
    // */
    // replace_fops(filp, &dummy_fops);

    // /*
    // * There will be no open() call for this new private element
    // * since we have no associated cdev, bind it to the anon file
    // * immediately.
    // */
    // let ret = bind_file_to_element(&mut File{ptr: filp}, e.clone());
    let _ret = bind_file_to_element(filp, e.clone());
    pr_debug!("the address of filp location 4 is {:p}, {:p}", filp, &filp);
    // if (ret) {
    // filp_close(filp, current->files);
    // /*
    // * rros_release_element() was not called: do a manual
    // * disposal.
    // */
    // fac->dispose(e);
    // return ret;
    // }

    // /* Back to the real fops for this element class. */
    // replace_fops(filp, fac->fops);

    // filp_close(filp, current->files);
    // /*
    // * rros_release_element() was not called: do a manual
    // * disposal.
    // */
    // fac->dispose(e);
    // return ret;
    // }

    // /* Back to the real fops for this element class. */
    // replace_fops(filp, fac->fops);

    let efd = file::FileDescriptorReservation::new(bindings::O_RDWR | bindings::O_CLOEXEC)?;
    // let efd = unsafe {
    //     bindings::get_unused_fd_flags(bindings::O_RDWR|bindings::O_CLOEXEC)
    // };
    // efd = get_unused_fd_flags(O_RDWR|O_CLOEXEC);

    pr_debug!("the address of filp location 5 is {:p}, {:p}", filp, &filp);
    e_mut.fpriv.filp = Some(File { ptr: filp });
    pr_debug!(
        "the address of filp location 6 is {:p}, {:p}",
        e_mut.fpriv.filp.as_ref().unwrap().ptr,
        &(&(e_mut).fpriv.filp.as_ref().unwrap().ptr)
    );
    pr_debug!("efd: {}", efd.reserved_fd());
    e_mut.fpriv.efd = efd;
    // (e_borrow.clone_flags & RROS_CLONE_COREDEV) == RROS_CLONE_COREDEV;

    // efd = get_unused_fd_flags(O_RDWR|O_CLOEXEC);
    // if (efd < 0) {
    // filp_close(filp, current->files);
    // ret = efd;
    // return ret;
    // }

    // e->fpriv.filp = filp;
    // e->fpriv.efd = efd;

    Ok(0)
    // return 0;
    // }
}

pub fn bind_file_to_element(
    filp: *mut bindings::file,
    e: Rc<RefCell<RrosElement>>,
) -> Result<usize> {
    // static int bind_file_to_element(struct file *filp, struct rros_element *e)
    // {
    // 	struct rros_file_binding *fbind;
    // 	int ret;
    let mut fbind: RrosFileBinding = RrosFileBinding::new();
    // 	fbind = kmalloc(sizeof(*fbind), GFP_KERNEL);
    // 	if (fbind == NULL)
    // 		return -ENOMEM;

    pr_debug!("the address of filp location 2 is {:p}, {:p}", filp, &filp);
    let _ret = fbind.rfile.borrow_mut().rros_open_file(filp)?;
    pr_debug!("the address of filp location 3 is {:p}, {:p}", filp, &filp);
    pr_debug!(
        "the address of fbind.rfile.filp.oob_data is {:p}",
        fbind.rfile.as_ptr()
    );
    // let ret = rros_open_file(&fbind.efile, filp.get_ptr());
    // 	ret = rros_open_file(&fbind->efile, filp);
    // 	if (ret) {
    // 		kfree(fbind);
    // 		return ret;
    // 	}

    // FIXME: this is a memory leak and fix the rc stuff
    fbind.element = unsafe { (*Rc::as_ptr(&e)).as_ptr() };
    // 	fbind->element = e;
    let fbind: Box<RrosFileBinding> = Box::try_new(fbind)?;
    let fbind_ptr = Box::into_raw(fbind);
    // unsafe{ (*((*fbind_ptr).rfile.borrow_mut().filp)).oob_data = (*fbind_ptr).rfile.as_ptr() as *const RrosFile as _;}
    unsafe {
        pr_debug!(
            "the address of fbind_ptr.rfile.filp.oob_data is {:p}",
            (*((*fbind_ptr).rfile.borrow_mut().filp)).oob_data
        )
    };
    unsafe {
        pr_debug!(
            "the address of filp.oob_data is {:p}",
            (*fbind_ptr).rfile.as_ptr()
        )
    };

    //    filp.set_private_data(fbind_ptr as *mut c_types::c_void);
    unsafe {
        (*filp).private_data = fbind_ptr as *mut c_types::c_void;
    }
    // 	filp->private_data = fbind;

    // 	return 0;
    Ok(0)
    // }
}

pub fn rros_create_core_element_device(
    e: Rc<RefCell<RrosElement>>,
    fac: &'static mut SpinLock<RrosFactory>,
    name: &'static CStr,
) -> Result<usize> {
    let e_clone = e.clone();
    let mut e_mut = e_clone.borrow_mut();

    if !name.is_empty() {
        let devname = fs::Filename::getname_kernel(name).unwrap();
        e_mut.devname = Some(devname);
    }

    e_mut.clone_flags |= RROS_CLONE_COREDEV;
    drop(e_mut);
    let res = create_element_device(e, fac);

    res
}

// This function is implemented in the rros_init_element.
// fn create_element_class(fac: Arc<&mut SpinLock<RrosFactory>>, this_module: &'static ThisModule) -> Result<usize> {
//     let rros_class: class::Class = class::Class::new(this_module, fac.name.as_char_ptr())?;
//     let minor_map = Some(bitmap_zalloc(fac.nrdev, bindings::GFP_KERNEL));
//     let rrtype = Some(device::DeviceType::new().name(fac.name.as_char_ptr()));
//     match fac.inside {
//         Some(ref mut inside) => {
//             inside.minor_map = minor_map;
//             if inside.minor_map == Some(0) {
//                 return Err(kernel::Error::EINVAL);
//             }

//             inside.class = Some(Arc::try_new(rros_class)?);
//             inside.rrtype = rrtype;
//             // fac.rrtype.devnode(Option::Some(factory_type_devnode));
//             // fac.kuid = GLOBAL_ROOT_UID;
//             // fac.kgid = GLOBAL_ROOT_GID;

//             let mut chrdev_reg: Pin<Box<chrdev::Registration<NR_CLOCKNR>>> =
//                 chrdev::Registration::new_pinned(fac.name, 0, this_module)?;
//             chrdev_reg.as_mut().register::<clock::RustFileClock>()?;
//             Ok(0)
//         }
//         None => {
//             Err(kernel::Error::EINVAL)},
//     }
// }

// TODO: The global variable should not use *mut to pass the value.
pub fn rros_init_element(
    e: Rc<RefCell<RrosElement>>,
    fac: &'static mut SpinLock<RrosFactory>,
    clone_flags: i32,
) -> Result<usize> {
    let mut minor = 0;
    let mut fac_lock = fac.lock();
    let nrdev = fac_lock.nrdev;

    let _res = match fac_lock.inside {
        Some(ref mut inside) => {
            loop {
                let minor_map;
                if inside.minor_map.is_none() {
                    return Err(kernel::Error::EINVAL);
                }
                minor_map = inside.minor_map.unwrap();

                minor = bitmap::find_first_zero_bit(
                    minor_map as *mut u8 as *const c_types::c_ulong,
                    nrdev as u64,
                );
                if minor >= nrdev as u64 {
                    pr_err!("out of factory number");
                    return Err(kernel::Error::EINVAL);
                }
                if !bitmap::test_and_set_bit(minor, minor_map as *mut c_types::c_ulong) {
                    break;
                }
            }
            0
        }
        None => 1,
    };
    unsafe {
        fac.unlock();
    }
    drop(fac_lock);
    let e_clone = e.clone();
    let mut e_mut = e_clone.borrow_mut();
    e_mut.factory = fac;
    e_mut.minor = minor;
    e_mut.refs = 1;
    e_mut.dev = None;
    e_mut.fpriv.filp = None;
    //FIXME: fd should correct.
    e_mut.fpriv.efd = file::FileDescriptorReservation { fd: 900 };
    e_mut.zombie = false;
    e_mut.fundle = RROS_NO_HANDLE;
    e_mut.devname = None;
    e_mut.clone_flags = clone_flags;
    let pinned = unsafe { Pin::new_unchecked(&mut e_mut.ref_lock) };
    spinlock_init!(pinned, "value");
    Ok(0)
}

fn create_sys_device<T>(
    rdev: u32,
    inside: &mut RrosFactoryInside,
    drvdata: *mut T,
    name: &CStr,
) -> device::Device {
    // TODO: wrap
    let mut dev = unsafe {
        device::Device::raw_new(
            |dev| {
                dev.devt = rdev;
                dev.class = inside.class.as_ref().unwrap().get_ptr();
                dev.type_ = inside.type_.get_ptr();
                // dev.groups = fac.locked_data().get().attrs.as_mut().unwrap();
            },
            name,
        )
    };
    if !drvdata.is_null() {
        dev.set_drvdata(drvdata);
    }
    dev
}

fn rros_create_factory(
    fac: &mut SpinLock<RrosFactory>,
    rros_class: Arc<class::Class>,
    chrdev_reg: &mut Pin<Box<chrdev::Registration<NR_FACTORIES>>>,
    this_module: &'static ThisModule,
) -> Result<usize> {
    let mut fac_lock = fac.lock();
    let flag = fac_lock.flags.clone();
    let name = fac_lock.name;
    let nrdev = fac_lock.nrdev;

    let res = match fac_lock.inside {
        Some(ref mut inside) => {
            let mut idevname = CStr::from_bytes_with_nul("clone\0".as_bytes())?;
            if let RrosFactoryType::SINGLE = flag {
                // RROS_FACTORY_SINGLE
                idevname = name;
                inside.class = Some(rros_class.clone());
                inside.minor_map = Some(0);
                inside.sub_rdev = Some(class::DevT::new(0));
                match idevname.to_str() {
                    Ok("control") => {
                        chrdev_reg.as_mut().register::<control::ControlOps>()?;
                    }
                    Ok("poll") => {
                        chrdev_reg.as_mut().register::<poll::PollOps>()?;
                    }
                    Ok(_) => {
                        pr_alert!("not yet implemented");
                    }
                    Err(_e) => {
                        pr_err!("should not meet here");
                    }
                }
            } else {
                // create_element_class
                inside.minor_map =
                    Some(bitmap_zalloc(nrdev.try_into().unwrap(), bindings::GFP_KERNEL) as u64);
                if inside.minor_map == Some(0) {
                    return Err(kernel::Error::EINVAL);
                }

                inside.class = Some(Arc::try_new(class::Class::new(
                    this_module,
                    name.as_char_ptr(),
                )?)?);
                let mut type_ = device::DeviceType::new().name(name.as_char_ptr());
                // TODO: ugly
                type_.set_devnode::<FactoryTypeDevnode>();
                inside.type_ = type_;
                inside.kuid = Some(KuidT::global_root_uid());
                inside.kgid = Some(KgidT::global_root_gid());
                // here we cannot get the number from the nrdev, because this requires const
                match name.to_str() {
                    Ok("clock") => {
                        let ele_chrdev_reg: Pin<
                            Box<chrdev::Registration<{ thread::CONFIG_RROS_NR_THREADS }>>,
                        > = chrdev::Registration::new_pinned(name, 0, this_module)?;
                        inside.register = Some(ele_chrdev_reg);
                        // register monotonic clock
                        inside
                            .register
                            .as_mut()
                            .unwrap()
                            .as_mut()
                            .register::<clock::ClockOps>()?;
                        // register realtime clock
                        inside
                            .register
                            .as_mut()
                            .unwrap()
                            .as_mut()
                            .register::<clock::ClockOps>()?;
                    }
                    Ok("thread") => {
                        let ele_chrdev_reg: Pin<
                            Box<chrdev::Registration<{ thread::CONFIG_RROS_NR_THREADS }>>,
                        > = chrdev::Registration::new_pinned(name, 0, this_module)?;
                        inside.register = Some(ele_chrdev_reg);
                        inside
                            .register
                            .as_mut()
                            .unwrap()
                            .as_mut()
                            .register::<thread::ThreadOps>()?;
                    }
                    Ok("xbuf") => {
                        let ele_chrdev_reg: Pin<
                            Box<chrdev::Registration<{ xbuf::CONFIG_RROS_NR_XBUFS }>>,
                        > = chrdev::Registration::new_pinned(name, 0, this_module)?;
                        inside.register = Some(ele_chrdev_reg);
                        inside
                            .register
                            .as_mut()
                            .unwrap()
                            .as_mut()
                            .register::<xbuf::XbufOps>()?;
                    }
                    Ok("proxy") => {
                        let ele_chrdev_reg: Pin<
                            Box<chrdev::Registration<{ proxy::CONFIG_RROS_NR_PROXIES }>>,
                        > = chrdev::Registration::new_pinned(name, 0, this_module)?;
                        inside.register = Some(ele_chrdev_reg);
                        inside
                            .register
                            .as_mut()
                            .unwrap()
                            .as_mut()
                            .register::<proxy::ProxyOps>()?;
                    }
                    Ok("observable") => {
                        let ele_chrdev_reg: Pin<
                            Box<chrdev::Registration<{ observable::CONFIG_RROS_NR_OBSERVABLE }>>,
                        > = chrdev::Registration::new_pinned(name, 0, this_module)?;
                        inside.register = Some(ele_chrdev_reg);
                        inside
                            .register
                            .as_mut()
                            .unwrap()
                            .as_mut()
                            .register::<observable::ObservableOps>()?;
                    }
                    Ok(_) => {
                        pr_info!("not yet implemented");
                    }
                    Err(_e) => {
                        pr_info!("should not meet here");
                    }
                }
                // no need to call register here
                // ele_chrdev_reg.as_mut().register::<fac.inside_data()>()?; //alloc_chrdev + cdev_alloc + cdev_add
                // inside.register = Some(ele_chrdev_reg);
                // inside.register.as_mut().unwrap().as_mut().register::<thread::ThreadOps>()?;
                // create_element_class end

                // FIXME: this should be variable. But the `register` needs a const value. We just hack for now. If we need more
                // factory, we need to change the code here. One way here is to use index to find the struct.

                // let factory_ops = fac.locked_data().into_inner();
                if let RrosFactoryType::CLONE = flag {
                    chrdev_reg.as_mut().register::<CloneOps>()?;
                }
            }
            if let RrosFactoryType::SINGLE | RrosFactoryType::CLONE = flag {
                let rdev = chrdev_reg.as_mut().last_registered_devt().unwrap();
                let dev = create_sys_device(rdev, inside, ptr::null_mut() as *mut u8, idevname);
                inside.device = Some(dev);
            }

            let mut index = RrosIndex {
                rbtree: unsafe { SpinLock::new(rbtree::RBTree::new()) },
                generator: RROS_NO_HANDLE,
            };
            let pinned = unsafe { Pin::new_unchecked(&mut index.rbtree) };
            spinlock_init!(pinned, "value");
            inside.index = Some(index);

            let mut hashname: [types::HlistHead; NAME_HASH_TABLE_SIZE as usize] =
                [types::HlistHead::new(); NAME_HASH_TABLE_SIZE as usize];
            types::hash_init(hashname[0].as_list_head(), NAME_HASH_TABLE_SIZE);
            inside.name_hash = Some(hashname);
            let mut hash_lock = unsafe { SpinLock::new(0) };
            let pinned = unsafe { Pin::new_unchecked(&mut hash_lock) };
            spinlock_init!(pinned, "device_name_hash_lock");
            inside.hash_lock = Some(hash_lock);
            0
        }
        None => 1,
    };

    unsafe { fac.unlock() };
    match res {
        1 => Err(kernel::Error::EINVAL),
        _ => Ok(0),
    }
}

// TODO: adjust the order of use and funciton in the whole project

// #[derive(Default)]
pub struct CloneData {
    pub ptr: *mut u8,
}

impl Default for CloneData {
    fn default() -> Self {
        CloneData { ptr: 0 as *mut u8 }
    }
}

#[derive(Default)]
pub struct CloneOps;

impl FileOpener<u8> for CloneOps {
    fn open(shared: &u8, _fileref: &File) -> Result<Self::Wrapper> {
        let mut data = CloneData::default();
        unsafe {
            data.ptr = shared as *const u8 as *mut u8;
            let a = KuidT::from_inode_ptr(shared as *const u8);
            let b = KgidT::from_inode_ptr(shared as *const u8);
            (*thread::RROS_THREAD_FACTORY.locked_data().get())
                .inside
                .as_mut()
                .unwrap()
                .kuid = Some(a);
            (*thread::RROS_THREAD_FACTORY.locked_data().get())
                .inside
                .as_mut()
                .unwrap()
                .kgid = Some(b);
        }
        // bindings::stream_open();
        pr_debug!("open clone success");
        Ok(Box::try_new(data)?)
    }
    // fn open<T: IoBufferWriter>(
    //     _this: &Self,
    //     _file: &File,
    //     _data: &mut T,
    //     _offset: u64,
    // ) -> Result<usize> {
    //     pr_debug!("I'm the open ops from the clone ops.");

    //     unsafe {
    //         (*thread::RROS_THREAD_FACTORY.get_locked_data().get()).inside.as_ref().unwrap().kuid = i

    //     };
    //     Ok(1)
    // }
}

// FIXME: all the ops is made for the thread factory. We need to change this later.
impl FileOperations for CloneOps {
    kernel::declare_file_operations!(read, ioctl);

    type Wrapper = Box<CloneData>;

    fn read<T: IoBufferWriter>(
        _this: &CloneData,
        _file: &File,
        _data: &mut T,
        _offset: u64,
    ) -> Result<usize> {
        pr_debug!("I'm the read ops from the clone ops.");
        Ok(1)
    }

    fn release(_this: Box<CloneData>, _file: &File) {
        pr_debug!("I'm the release ops from the clone ops.");
        // FIXME: put the rros element
    }

    fn ioctl(_this: &CloneData, file: &File, cmd: &mut IoctlCommand) -> Result<i32> {
        pr_debug!("I'm the unlock_ioctl ops from the clone ops.");
        // FIXME: use the IoctlCommand in the right way
        ioctl_clone_device(file, cmd.cmd, cmd.arg)?;
        Ok(0)
    }
}

fn create_core_factories(
    factories: &mut [&mut SpinLock<RrosFactory>],
    nr: usize,
    rros_class: Arc<class::Class>,
    chrdev_reg: &mut Pin<Box<chrdev::Registration<NR_FACTORIES>>>,
    this_module: &'static ThisModule,
) -> Result<usize> {
    for i in 0..nr {
        let mut _ret = rros_create_factory(
            &mut factories[i],
            rros_class.clone(),
            chrdev_reg,
            this_module,
        );
    }
    Ok(0)
}

pub fn rros_early_init_factories(
    this_module: &'static ThisModule,
) -> Result<Pin<Box<chrdev::Registration<NR_FACTORIES>>>> {
    // TODO: move the number of factories to a variable
    let mut early_factories: [&mut SpinLock<RrosFactory>; 7] = unsafe {
        [
            &mut clock::RROS_CLOCK_FACTORY,
            &mut thread::RROS_THREAD_FACTORY,
            &mut xbuf::RROS_XBUF_FACTORY,
            &mut proxy::RROS_PROXY_FACTORY,
            &mut control::RROS_CONTROL_FACTORY,
            &mut poll::RROS_POLL_FACTORY,
            &mut observable::RROS_OBSERVABLE_FACTORY,
        ]
    };
    // static struct rros_factory *early_factories[] = {
    // 	&rros_clock_factory,
    // };

    // let factories :[rros_factory; 7];

    // static struct rros_factory *factories[] = {
    // 	&rros_control_factory,
    // 	&rros_thread_factory,
    // 	&rros_monitor_factory,
    // 	&rros_poll_factory,
    // 	&rros_xbuf_factory,
    // 	&rros_proxy_factory,
    // 	&rros_observable_factory,
    // #ifdef CONFIG_FTRACE
    // 	&rros_trace_factory,
    // #endif
    // };

    let mut rros_class: Arc<class::Class> = Arc::try_new(class::Class::new(
        this_module,
        CStr::from_bytes_with_nul("rros\0".as_bytes())?.as_char_ptr(),
    )?)?;
    // TODO: create a structure to implement rros_devnode.
    Arc::get_mut(&mut rros_class)
        .unwrap()
        .set_devnode::<RrosDevnode>();
    // unsafe {
    //     (*rros_class.ptr).devnode = Option::Some(rros_devnode);
    // }
    let mut chrdev_reg: Pin<Box<chrdev::Registration<NR_FACTORIES>>> =
        chrdev::Registration::new_pinned(c_str!("rros_factory"), 0, this_module)?;
    chrdev_reg.as_mut().register::<RRosRustFile>()?;
    let len = early_factories.len();
    let _ret = create_core_factories(
        &mut early_factories,
        len,
        rros_class,
        &mut chrdev_reg,
        this_module,
    )?;

    Ok(chrdev_reg)
}

// struct inode;

// pub fn rros_open_element(inode: *const bindings::inode, filp: &mut File)-> Result<usize> {
//     let e = kernel::container_of!(&(*inode).__bindgen_anon_4.i_cdev, RrosElement, cdev);
//     let rce = unsafe { Rc::try_new(RefCell::new(*e))? };
//   	// rcu_read_lock();

// 	// raw_spin_lock_irqsave(&e->ref_lock, flags);

// 	// if (e->zombie) {
// 	// 	ret = -ESTALE;
// 	// } else {
// 	// 	RROS_WARN_ON(CORE, e->refs == 0);
// 	// 	e->refs++;
// 	// }

// 	// raw_spin_unlock_irqrestore(&e->ref_lock, flags);

// 	// rcu_read_unlock();

// 	// if (ret)
// 	// 	return ret;

//     bind_file_to_element(filp, rce.clone());
//     // if (ret) {
// 	// 	rros_put_element(e);
// 	// 	return ret;
// 	// }

// 	// stream_open(inode, filp);

//     Ok(0)
// }

// impl<T: Sync> FileOpenAdapter for Registration<T> {
//     type Arg = T;

//     unsafe fn convert(_inode: *mut bindings::inode, file: *mut bindings::file) -> *const Self::Arg {
//         // TODO: `SAFETY` comment required here even if `unsafe` is not present,
//         // because `container_of!` hides it. Ideally we would not allow
//         // `unsafe` code as parameters to macros.
//         let reg = crate::container_of!((*file).private_data, Self, mdev);
//         unsafe { &(*reg).context }
//     }
// }

#[derive(Default)]
struct RRosRustFile;

#[allow(dead_code)]
struct Ct {
    pub count: i32,
}

impl FileOperations for RRosRustFile {
    kernel::declare_file_operations!(read);

    fn read<T: IoBufferWriter>(
        _this: &Self,
        _file: &File,
        _data: &mut T,
        _offset: u64,
    ) -> Result<usize> {
        pr_debug!("I'm the read ops of the rros factory.");
        Ok(1)
    }
}

// impl FileOpener<Pin<Ref<Ct>>> for RRosRustFile {
//     fn open(shared: &Ref<Ct>) -> Result<Box<Self>> {
//         Ok(Box::try_new(Self.clone())?)
//     }
// }

#[allow(dead_code)]
pub fn rros_get_index(handle: FundleT) -> FundleT {
    handle & !RROS_HANDLE_INDEX_MASK
}

#[repr(C)]
struct RrosCloneReq {
    name: *const c_types::c_char,
    attrs: *mut c_types::c_void,
    clone_flags: c_types::c_uint,
    eids: RrosElementIds,
    efd: c_types::c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RrosElementIds {
    minor: c_types::c_uint,
    fundle: FundleT,
    state_offset: c_types::c_uint,
}

impl Default for RrosElementIds {
    fn default() -> Self {
        Self {
            minor: 0,
            fundle: 0,
            state_offset: 0,
        }
    }
}

impl RrosCloneReq {
    fn new(name: *const c_types::c_char, attrs: *mut c_types::c_void) -> Self {
        Self {
            name,
            attrs,
            clone_flags: 0,
            eids: RrosElementIds::default(),
            efd: 0,
        }
    }

    #[allow(dead_code)]
    fn from_ptr(ptr: *mut RrosCloneReq) -> Self {
        Self {
            name: unsafe { (*ptr).name },
            attrs: unsafe { (*ptr).attrs },
            clone_flags: unsafe { (*ptr).clone_flags },
            eids: unsafe { (*ptr).eids },
            efd: unsafe { (*ptr).efd },
        }
    }
}

extern "C" {
    pub fn rust_helper_copy_from_user(
        to: *mut c_types::c_void,
        from: *const c_types::c_void,
        n: c_types::c_ulong,
    ) -> c_types::c_ulong;
}

pub fn ioctl_clone_device(file: &File, _cmd: u32, arg: usize) -> Result<usize> {
    // static long ioctl_clone_device(struct file *filp, unsigned int cmd,
    //     unsigned long arg)
    // {
    // struct rros_element *e = filp->private_data;
    // struct rros_clone_req req, __user *u_req;
    let state_offset: u32 = u32::MAX;
    // __u32 val, state_offset = -1U;
    // const char __user *u_name;
    // struct rros_factory *fac;
    // void __user *u_attrs;
    // int ret;

    // TODO: add the support of clone device cmd
    // if (cmd != RROS_IOC_CLONE):
    // return -ENOTTY;

    // TODO: add the rros running check
    // if (!rros_is_running())
    // return -ENXIO;

    // TODO: add the support of private data check
    // let e = filp->private_data;
    // if (e)
    //     return -EBUSY;

    // TODO: user parameters
    pr_debug!("size is {}", size_of::<RrosCloneReq>());
    let mut real_req = RrosCloneReq::new(0 as *const c_types::c_char, 0 as *mut c_types::c_void);
    let res = unsafe {
        rust_helper_copy_from_user(
            &mut real_req as *mut RrosCloneReq as *mut c_types::c_void,
            arg as *mut c_types::c_void,
            size_of::<RrosCloneReq>() as u64,
        )
    };
    if res != 0 {
        pr_err!("copy from user failed");
        return Err(Error::EFAULT);
    }
    // let u_req = unsafe{UserSlicePtr::new(arg as *mut c_types::c_void, size_of::<RrosCloneReq>())};
    // let req = u_req.read_all()?;

    // let mut real_req: RrosCloneReq = unsafe{*ptr::slice_from_raw_parts_mut(core::mem::transmute(req.as_ptr()), req.len())};
    // TODO: fix the unsafe code
    // let mut real_req: RrosCloneReq = unsafe{core::mem::transmute_copy(&req.as_ptr())};
    pr_debug!("real_req {}", real_req.clone_flags);
    pr_debug!("real_req {}", real_req.efd);
    pr_debug!("real_req {}", real_req.eids.fundle);
    pr_debug!("real_req {}", real_req.eids.minor);
    pr_debug!("real_req {}", real_req.eids.state_offset);
    pr_debug!("real_req {:p}", real_req.name);
    pr_debug!("real_req {:p}", real_req.attrs);

    // u_req = (typeof(u_req))arg;
    // ret = copy_from_user(&req, u_req, sizeof(req));
    // if (ret)
    // return -EFAULT;

    let u_name = real_req.name as *const u8;
    // u_name = rros_valptr64(req.name_ptr, const char);
    // if (u_name == NULL && req.clone_flags & RROS_CLONE_PUBLIC)
    // return -EINVAL;

    let u_attrs = real_req.attrs as *mut u8;
    pr_debug!("the u_attrs is {:p}", u_attrs);
    // u_attrs = rros_valptr64(req.attrs_ptr, void);
    let cstr_u_name = unsafe { CStr::from_char_ptr(u_name as *const c_types::c_char) };
    // FIXME: update the cdev logic to use container_of && update the uname
    // fac = container_of(filp->f_inode->i_cdev, struct rros_factory, cdev);
    let fdname = file.get_parent_name().unwrap();
    pr_debug!("the value is {:?} ", cstr_u_name[0]);
    pr_debug!("ioctl_clone_device: clone type is {}", fdname);
    let e: Rc<RefCell<RrosElement>> = if fdname == "xbuf" {
        pr_debug!("ioctl_clone_device: xbuf clone");
        unsafe {
            (*xbuf::RROS_XBUF_FACTORY.locked_data().get())
                .build
                .unwrap()(
                &mut xbuf::RROS_XBUF_FACTORY,
                cstr_u_name,
                Some(u_attrs),
                real_req.clone_flags.try_into().unwrap(),
                &state_offset,
            )
        }
    } else if fdname == "proxy" {
        pr_debug!("ioctl_clone_device: proxy clone");
        unsafe {
            (*proxy::RROS_PROXY_FACTORY.locked_data().get())
                .build
                .unwrap()(
                &mut proxy::RROS_PROXY_FACTORY,
                cstr_u_name,
                Some(u_attrs),
                real_req.clone_flags.try_into().unwrap(),
                &state_offset,
            )
        }
    } else if fdname == "observable" {
        pr_debug!("ioctl_clone_device: observable clone");
        unsafe {
            (*observable::RROS_OBSERVABLE_FACTORY.locked_data().get())
                .build
                .unwrap()(
                &mut observable::RROS_OBSERVABLE_FACTORY,
                cstr_u_name,
                Some(u_attrs),
                real_req.clone_flags.try_into().unwrap(),
                &state_offset,
            )
        }
    } else {
        pr_debug!("maybe a thread");
        unsafe {
            (*thread::RROS_THREAD_FACTORY.locked_data().get())
                .build
                .unwrap()(
                &mut thread::RROS_THREAD_FACTORY,
                cstr_u_name,
                Some(u_attrs),
                0,
                &state_offset,
            )
        }
    };
    //real_req.clone_flags as i32

    // unsafe{pr_debug!("4 uninit_thread: x ref is {}", Arc::strong_count(&thread::UTHREAD.clone().unwrap()));}
    // unsafe{pr_debug!("4.5 uninit_thread: x ref is {}", Arc::strong_count(&thread::UTHREAD.clone().unwrap()));}
    // e = fac->build(fac, u_name, u_attrs, req.clone_flags, &state_offset);
    // if (IS_ERR(e))
    // return PTR_ERR(e);

    // /* This must be set before the device appears. */
    // file.set_private_data(e as *mut c_types::c_void);
    // filp->private_data = e;
    // barrier();

    // TODO: create the element device
    let _ret = if fdname == "xbuf" {
        pr_debug!("ioctl_clone_device: xbuf element create");
        create_element_device(e.clone(), unsafe { &mut xbuf::RROS_XBUF_FACTORY })
    } else if fdname == "proxy" {
        pr_debug!("ioctl_clone_device: proxy element create");
        create_element_device(e.clone(), unsafe { &mut proxy::RROS_PROXY_FACTORY })
    } else if fdname == "observable" {
        pr_debug!("ioctl_clone_device: observable element create");
        create_element_device(e.clone(), unsafe {
            &mut observable::RROS_OBSERVABLE_FACTORY
        })
    } else {
        pr_debug!("maybe a thread");
        create_element_device(e.clone(), unsafe { &mut thread::RROS_THREAD_FACTORY })
    };
    let e_clone = e.clone();
    let mut e_mut = e_clone.borrow_mut();
    // if (ret) {
    // /* release_clone_device() must skip cleanup. */
    // filp->private_data = NULL;
    // /*
    //  * If we failed to create a private element,
    //  * rros_release_element() did run via filp_close(), so
    //  * the disposal has taken place already.
    //  *
    //  * NOTE: this code should never directly handle core
    //  * devices, since we are running the user interface to
    //  * cloning a new element. Although a thread may be
    //  * associated with a coredev observable, the latter
    //  * does not export any direct interface to user.
    //  */
    // RROS_WARN_ON(CORE, rros_element_has_coredev(e));
    // /*
    //  * @e might be stale if it was private, test the
    //  * visibility flag from the request block instead.
    //  */
    // if (req.clone_flags & RROS_CLONE_PUBLIC)
    //     fac->dispose(e);
    // return ret;
    // }

    let mut ret: i32 = 0;
    unsafe {
        // let val = (*e).minor;
        // val = e->minor;
        // ret |= rust_helper_put_user(val as u32, &mut real_req.eids.minor as *mut u32);
        pr_debug!("the ret is {}", ret);
        // ret |= put_user(val, &u_req->eids.minor);
        // let val = (*e).fundle;
        // val = e->fundle;
        // ret |= rust_helper_put_user(val, &mut real_req.eids.fundle as *mut u32);
        // pr_debug!("the ret is {}", ret);
        // ret |= put_user(val, &u_req->eids.fundle);
        // ret |= rust_helper_put_user(state_offset, &mut real_req.eids.state_offset as *mut u32);
        // pr_debug!("the ret is {}", ret);
        // ret |= put_user(state_offset, &u_req->eids.state_offset);
        let val = &mut e_mut.fpriv.efd;
        // let val = &mut e_mut.fpriv.efd.reserved_fd();
        pr_debug!("the val is {}", val.reserved_fd());
        ret |= rust_helper_put_user(
            val.reserved_fd() as u32,
            &mut (*(arg as *mut RrosCloneReq)).efd as *mut i32 as *mut u32,
        );
        pr_debug!("the ret is {}", ret);
        // ret |= put_user(val, &u_req->efd);
    }
    // pr_debug!("the ret is {}", ret);
    // if ret!=0{
    // return Err(kernel::Error::EFAULT);
    // }
    // unsafe{pr_debug!("5 uninit_thread: x ref is {}", Arc::strong_count(&thread::UTHREAD.clone().unwrap()));}
    Ok(0)
    // return ret ? -EFAULT : 0;
    // }
}

pub fn rros_element_name(e: &RrosElement) -> *const c_types::c_char {
    if e.devname.is_some() {
        return e.devname.as_ref().unwrap().get_name();
    }
    0 as *const c_types::c_char
}

#[allow(dead_code)]
fn rros_index_element(map: &mut RrosIndex, e: Rc<RefCell<RrosElement>>) {
    let mut fundle: FundleT;
    let mut guard: FundleT = 0;
    let mut flags: u64;

    loop {
        if rros_get_index({
            guard = guard + 1;
            guard
        }) == 0
        {
            e.borrow_mut().fundle = RROS_NO_HANDLE;
            return;
        }

        flags = map.rbtree.irq_lock_noguard();
        fundle = rros_get_index({
            map.generator += 1;
            map.generator
        });
        if !fundle != 0 {
            map.generator = 1;
            fundle = map.generator;
        }

        let ret = unsafe { (*map.rbtree.locked_data().get()).get(&fundle) };
        let mut ok = Err(Error::EEXIST);
        if ret.is_none() {
            // `try_insert`` always return Ok
            ok = unsafe { (*map.rbtree.locked_data().get()).try_insert(fundle, e.clone()) };
        }

        map.rbtree.irq_unlock_noguard(flags);

        if ok.is_ok() {
            break;
        }
    }
}

#[allow(dead_code)]
#[inline]
pub fn rros_index_factory_element(e: Rc<RefCell<RrosElement>>) {
    unsafe {
        rros_index_element(
            (*e.borrow_mut().factory.locked_data().get())
                .inside
                .as_mut()
                .unwrap()
                .index
                .as_mut()
                .unwrap(),
            e.clone(),
        )
    }
}

#[allow(dead_code)]
pub fn rros_unindex_factory_element(e: Rc<RefCell<RrosElement>>) {
    unsafe {
        let map = (*e.borrow_mut().factory.locked_data().get())
            .inside
            .as_ref()
            .unwrap()
            .index
            .as_ref()
            .unwrap();
        let flag = map.rbtree.irq_lock_noguard();
        (*map.rbtree.locked_data().get()).remove(&e.borrow().fundle);
        map.rbtree.irq_unlock_noguard(flag);
    }
}

// Example of using the `rros_get_element_by_fundle` function
// let e = rros_get_element_by_fundle((*thread::RROS_THREAD_FACTORY.locked_data().get()).inside.as_mut().unwrap().index.as_mut().unwrap(), fundle);
// let element: *mut T; // T means the type of the element.
// if e.is_some() {
//      let e = e.unwrap();
//      let e = e.borrow_mut();
//      element = e.pointer as *mut T;
// }
//
#[allow(dead_code)]
pub fn rros_get_element_by_fundle(
    map: &mut RrosIndex,
    fundle: FundleT,
) -> Option<Rc<RefCell<RrosElement>>> {
    let flags = map.rbtree.irq_lock_noguard();

    let e = unsafe {
        (*map.rbtree.locked_data().get())
            .get(&fundle)
            .map(|e| e.clone())
    };

    map.rbtree.irq_unlock_noguard(flags);

    e
}

#[allow(dead_code)]
fn rros_destroy_element(_e: Rc<RefCell<RrosElement>>) {
    // `clear_bit` is unnecessary because minor_map is a u64 type in rros.
    // `putname` is implemented automatically in `drop` function.
}
