use alloc::rc::Rc;

use core::{cell::RefCell, cmp::min, convert::TryInto, ops::Deref};

use crate::{
    c_types::*,
    factory::{
        rros_element_name, CloneData, RrosElement, RrosFactory, RrosFactoryInside,
        RROS_CLONE_INPUT, RROS_CLONE_OUTPUT, RROS_CLONE_PUBLIC,
    },
    file::RrosFileBinding,
    flags::RrosFlag,
    lock::raw_spin_lock_init,
    poll::RrosPollHead,
    sched::rros_schedule,
    thread::rros_init_user_element,
    work::*,
};

use kernel::{
    bindings,
    device::DeviceType,
    file::File,
    file_operations::{FileOpener, FileOperations},
    fs,
    io_buffer::{IoBufferReader, IoBufferWriter},
    prelude::*,
    premmpt::running_inband,
    str::CStr,
    sync::{mutex_lock, mutex_unlock, SpinLock},
    types::Atomic,
    user_ptr::{UserSlicePtrReader, UserSlicePtrWriter},
    vmalloc::c_kzalloc,
    waitqueue,
    workqueue::*,
};

const FMODE_ATOMIC_POS: u32 = 0x8000;
type LoffT = i64;
// this should be 64
// pub const CONFIG_RROS_NR_PROXIES: usize = 64;
pub const CONFIG_RROS_NR_PROXIES: usize = 16;
const RROS_PROXY_CLONE_FLAGS: i32 = RROS_CLONE_PUBLIC | RROS_CLONE_OUTPUT | RROS_CLONE_INPUT;
pub struct ProxyRing {
    pub bufmem: *mut u8,
    pub fillsz: Atomic,
    pub nesting: i32,
    pub bufsz: u32,
    pub rdoff: u32,
    pub wroff: u32,
    pub reserved: u32,
    pub granularity: u32,
    pub oob_wait: RrosFlag,
    pub inband_wait: waitqueue::WaitQueueHead,
    pub relay_work: RrosWork,
    pub lock: SpinLock<i32>,
    pub wq: Option<BoxedQueue>,
    pub worker_lock: Arc<SpinLock<i32>>,
}

impl ProxyRing {
    pub fn new() -> Result<Self> {
        Ok(Self {
            bufmem: 0 as *mut u8,
            fillsz: Atomic::new(),
            nesting: 0,
            bufsz: 0,
            rdoff: 0,
            wroff: 0,
            reserved: 0,
            granularity: 0,
            oob_wait: RrosFlag::new(),
            inband_wait: waitqueue::WaitQueueHead::new(),
            relay_work: RrosWork::new(),
            lock: unsafe { SpinLock::new(0) },
            wq: None,
            worker_lock: unsafe { Arc::try_new(SpinLock::new(0))? },
        })
    }
}

pub struct ProxyOut {
    pub ring: ProxyRing,
}

impl ProxyOut {
    pub fn new() -> Result<Self> {
        Ok(Self {
            ring: ProxyRing::new()?,
        })
    }
}

pub struct ProxyIn {
    pub ring: ProxyRing,
    pub reqsz: Atomic,
    pub on_eof: Atomic,
    pub on_error: i32,
}

impl ProxyIn {
    pub fn new() -> Result<Self> {
        Ok(Self {
            ring: ProxyRing::new()?,
            reqsz: Atomic::new(),
            on_eof: Atomic::new(),
            on_error: 0,
        })
    }
}

pub struct RrosProxy {
    filp: File,
    output: ProxyOut,
    input: ProxyIn,
    element: Rc<RefCell<RrosElement>>,
    #[allow(dead_code)]
    poll_head: RrosPollHead,
}

impl RrosProxy {
    pub fn new(fd: u32) -> Result<Self> {
        Ok(Self {
            filp: File::from_fd(fd)?,
            output: ProxyOut::new()?,
            input: ProxyIn::new()?,
            element: Rc::try_new(RefCell::new(RrosElement::new()?))?,
            poll_head: RrosPollHead::new(),
        })
    }
}

#[repr(C)]
pub struct RrosProxyAttrs {
    fd: u32,
    bufsz: u32,
    granularity: u32,
}

impl RrosProxyAttrs {
    #[allow(dead_code)]
    fn new() -> Self {
        RrosProxyAttrs {
            fd: 0,
            bufsz: 0,
            granularity: 0,
        }
    }

    fn from_ptr(attrs: *mut RrosProxyAttrs) -> Self {
        unsafe {
            Self {
                fd: (*attrs).fd,
                bufsz: (*attrs).bufsz,
                granularity: (*attrs).granularity,
            }
        }
    }
}

pub fn proxy_is_readable(proxy: &RrosProxy) -> bool {
    (proxy.element.borrow().deref().clone_flags & RROS_CLONE_INPUT) != 0
}

pub fn proxy_is_writeable(proxy: &RrosProxy) -> bool {
    pr_debug!(
        "the proxy clone flags is {}",
        proxy.element.borrow().deref().clone_flags
    );
    (proxy.element.borrow().deref().clone_flags & RROS_CLONE_OUTPUT) != 0
}

fn rounddown(x: usize, y: usize) -> usize {
    x - x % y
}

pub fn relay_output(proxy: &mut RrosProxy) -> Result<usize> {
    let ring: &mut ProxyRing = &mut proxy.output.ring;
    let mut rdoff: u32;
    let mut count: u32;
    let mut len: u32;
    let mut n: u32;
    let mut pos: LoffT = 0;
    let mut ppos: *mut LoffT;
    let mut ret: isize = 0;
    let filp = &proxy.filp;

    let wklock = ring.worker_lock.lock();
    count = ring.fillsz.atomic_read() as u32;
    rdoff = ring.rdoff;
    ppos = 0 as *mut LoffT;
    if (unsafe { (*filp.get_ptr()).f_mode } & FMODE_ATOMIC_POS) != 0 {
        unsafe {
            mutex_lock(&mut (*filp.get_ptr()).f_pos_lock);
        }
        ppos = &mut pos as *mut LoffT;
        pos = unsafe { (*filp.get_ptr()).f_pos };
    }
    while count > 0 && ret >= 0 {
        len = count;
        loop {
            if rdoff + len > ring.bufsz {
                n = ring.bufsz - rdoff;
            } else {
                n = len;
            }

            if ring.granularity > 0 {
                n = min(n, ring.granularity);
            }

            ret = fs::kernel_write(
                filp.get_ptr(),
                unsafe { ring.bufmem.add(rdoff as usize) as *const c_void },
                n.try_into().unwrap(),
                ppos,
            );
            pr_debug!("pos: {}", pos);
            if ret >= 0 && !ppos.is_null() {
                unsafe { (*filp.get_ptr()).f_pos = *ppos };
            }
            len -= n;
            rdoff = (rdoff + n) % ring.bufsz;
            if len > 0 && ret > 0 {
                continue;
            } else {
                break;
            }
        }

        count = ring.fillsz.atomic_sub_return(count.try_into().unwrap()) as u32;
    }

    if !ppos.is_null() {
        unsafe {
            mutex_unlock(&mut (*filp.get_ptr()).f_pos_lock);
        }
    }
    ring.rdoff = rdoff;
    drop(wklock);

    // if count == 0 {
    //     rros_singal_poll_events(&proxy.poll_head, POLLOUT|POLLWRNORM);
    // }

    if count < ring.bufsz {
        ring.oob_wait.raise();
        ring.inband_wait
            .wake_up(bindings::TASK_NORMAL, 1, 0 as *mut c_void);
    } else {
        unsafe {
            rros_schedule();
        }
    }

    Ok(0)
}

pub fn relay_output_work(work: &mut RrosWork) -> i32 {
    let proxy: *mut RrosProxy =
        kernel::container_of!(work, RrosProxy, output.ring.relay_work) as *mut RrosProxy;

    let _ret = relay_output(unsafe { &mut (*proxy) });
    0
}

pub fn can_write_buffer(ring: &mut ProxyRing, size: usize) -> bool {
    ring.fillsz.atomic_read() as u32 + ring.reserved + size as u32 <= ring.bufsz
}

pub fn do_proxy_write(filp: &File, mut u_buf: *const c_char, count: usize) -> isize {
    let fbind: *const RrosFileBinding =
        unsafe { (*filp.get_ptr()).private_data as *const RrosFileBinding };
    let proxy = unsafe { (*((*fbind).element)).pointer as *mut RrosProxy };
    let ring: &mut ProxyRing = unsafe { &mut (*proxy).output.ring };

    let mut wroff: u32;
    let mut wbytes: u32;
    let mut n: u32;
    let rsvd: u32;
    let mut flags: u64;
    let ret: isize;

    if count == 0 {
        return 0;
    }

    if count > ring.bufsz as usize {
        return -(bindings::EFBIG as isize);
    }

    if ring.granularity > 1 && count % (ring.granularity as usize) > 0 {
        return -(bindings::EINVAL as isize);
    }

    flags = ring.lock.irq_lock_noguard();
    if !can_write_buffer(ring, count) {
        ret = -(bindings::EAGAIN as isize);
        ring.lock.irq_unlock_noguard(flags);
        return ret;
    } else {
        wroff = ring.wroff;
        ring.wroff = (wroff + count as u32) % ring.bufsz;
        ring.nesting += 1;
        ring.reserved += count as u32;
        ret = count as isize;
        wbytes = count as u32;

        loop {
            if wroff + wbytes > ring.bufsz {
                n = ring.bufsz - wroff;
            } else {
                n = wbytes;
            }

            ring.lock.irq_unlock_noguard(flags);

            let uptrrd = u_buf as *mut UserSlicePtrReader;
            let res = unsafe {
                (*uptrrd).read_raw(
                    (ring.bufmem as usize + wroff as usize) as *mut u8,
                    n as usize,
                )
            };

            flags = ring.lock.irq_lock_noguard();

            let _ret = match res {
                Ok(()) => 0,
                Err(_e) => -(bindings::EFAULT as i32),
            };

            u_buf = unsafe { u_buf.add(n.try_into().unwrap()) };
            wbytes -= n;
            wroff = (wroff + n) % ring.bufsz;

            if wbytes > 0 {
                continue;
            } else {
                break;
            }
        }

        ring.nesting -= 1;
        if ring.nesting == 0 {
            n = ring.fillsz.atomic_add_return(ring.reserved as i32) as u32;
            rsvd = ring.reserved;
            ring.reserved = 0;

            if n == rsvd {
                if running_inband().is_ok() {
                    ring.lock.irq_unlock_noguard(flags);
                    let _ret = relay_output(unsafe { &mut (*proxy) });
                    return ret;
                }
                pr_debug!("there has been called");
                ring.relay_work
                    .call_inband_from(ring.wq.as_ref().unwrap().deref().get_ptr());
            }
        }
        ring.lock.irq_unlock_noguard(flags);
        ret
    }
}

pub fn relay_input(proxy: &mut RrosProxy) -> Result<usize> {
    let proxyin: &mut ProxyIn = &mut proxy.input;
    let ring: &mut ProxyRing = &mut proxyin.ring;
    let mut wroff: u32;
    let mut count: u32;
    let mut len: u32;
    let mut n: u32;
    let mut pos: LoffT = 0;
    let mut ppos: *mut LoffT;
    let mut ret: isize;
    let mut exception: bool = false;
    let filp = &proxy.filp;

    let wklock = ring.worker_lock.lock();
    count = proxyin.reqsz.atomic_read() as u32;
    wroff = ring.wroff;
    ppos = 0 as *mut LoffT;
    if (unsafe { (*filp.get_ptr()).f_mode } & FMODE_ATOMIC_POS) != 0 {
        unsafe {
            mutex_lock(&mut (*filp.get_ptr()).f_pos_lock);
        }
        ppos = &mut pos as *mut LoffT;
        pos = unsafe { (*filp.get_ptr()).f_pos };
    }

    'outer1: while count > 0 {
        len = count;
        'inner1: loop {
            if wroff + len > ring.bufsz {
                n = ring.bufsz - wroff;
            } else {
                n = len;
            }

            if ring.granularity > 0 {
                n = min(n, ring.granularity);
            }

            ret = fs::kernel_read(
                filp.get_ptr(),
                unsafe { ring.bufmem.add(wroff as usize) as *mut c_void },
                n.try_into().unwrap(),
                ppos,
            );
            let _ = pos;
            if ret <= 0 {
                proxyin.reqsz.atomic_sub((count - len) as i32);
                if ret != 0 {
                    proxyin.on_error = ret as i32;
                } else {
                    proxyin.on_eof.atomic_set(1);
                }
                exception = true;
                break 'outer1;
            }
            if !ppos.is_null() {
                unsafe { (*filp.get_ptr()).f_pos = *ppos };
            }
            ring.fillsz.atomic_add(ret as i32);
            len -= ret as u32;
            wroff = (wroff + n) % ring.bufsz;
            if len > 0 {
                continue 'inner1;
            } else {
                break 'inner1;
            }
        }

        count = proxyin.reqsz.atomic_sub_return(count.try_into().unwrap()) as u32;
    }

    if !ppos.is_null() {
        unsafe {
            mutex_unlock(&mut (*filp.get_ptr()).f_pos_lock);
        }
    }

    ring.wroff = wroff;

    drop(wklock);
    if ring.fillsz.atomic_read() > 0 || exception {
        //     rros_singal_poll_events(proxy.poll_head, POLLIN|POLLRDNORM);
        ring.oob_wait.raise();
        ring.inband_wait
            .wake_up(bindings::TASK_NORMAL, 1, 0 as *mut c_void);
    }
    Ok(0)
}

pub fn relay_input_work(work: &mut RrosWork) -> i32 {
    let proxy: *mut RrosProxy =
        kernel::container_of!(work, RrosProxy, input.ring.relay_work) as *mut RrosProxy;

    let _ret = relay_input(unsafe { &mut (*proxy) });
    0
}

pub fn do_proxy_read(filp: &File, mut u_buf: *const c_char, count: usize) -> isize {
    let fbind: *const RrosFileBinding =
        unsafe { (*filp.get_ptr()).private_data as *const RrosFileBinding };
    let proxy = unsafe { (*((*fbind).element)).pointer as *mut RrosProxy };
    let proxyin: &mut ProxyIn = unsafe { &mut (*proxy).input };
    let ring: &mut ProxyRing = &mut proxyin.ring;
    let mut len: isize;
    let ret: isize;
    let mut rbytes: isize;
    let mut n: isize;
    let mut rdoff: u32;
    let mut avail: u32;
    let mut flags: u64;
    let mut _u_ptr: *const c_char;

    if count > ring.bufsz as usize {
        return -(bindings::EFBIG as isize);
    }

    if ring.granularity > 1 && (count % ring.granularity as usize) > 0 {
        return -(bindings::EINVAL as isize);
    }

    len = count as isize;
    'outer: loop {
        _u_ptr = u_buf;
        loop {
            flags = ring.lock.irq_lock_noguard();
            avail = ring.fillsz.atomic_read() as u32 - ring.reserved;
            if avail < len as u32 {
                ring.lock.irq_unlock_noguard(flags);
                if avail > 0 && (unsafe { (*filp.get_ptr()).f_flags } & bindings::O_NONBLOCK) != 0 {
                    if ring.granularity != 0 {
                        len = rounddown(avail as usize, ring.granularity as usize) as isize;
                    } else {
                        len = rounddown(avail as usize, 1) as isize;
                    }

                    if len != 0 {
                        continue 'outer;
                    }
                }

                if proxyin.on_error != 0 {
                    return proxyin.on_error as isize;
                }

                return 0;
            }
            rdoff = ring.rdoff;
            ring.rdoff = (rdoff + len as u32) % ring.bufsz;
            ring.nesting += 1;
            ring.reserved += len as u32;
            ret = len;
            rbytes = ret;

            'rbytes: loop {
                if rdoff + rbytes as u32 > ring.bufsz {
                    n = (ring.bufsz - rdoff) as isize;
                } else {
                    n = rbytes;
                }

                ring.lock.irq_unlock_noguard(flags);

                let uptrwt = u_buf as *mut UserSlicePtrWriter;
                let res = unsafe {
                    (*uptrwt).write_raw(
                        (ring.bufmem as usize + rdoff as usize) as *const u8,
                        n as usize,
                    )
                };

                flags = ring.lock.irq_lock_noguard();

                let _ret = match res {
                    Ok(()) => 0,
                    Err(_e) => -(bindings::EFAULT as i32),
                };

                u_buf = unsafe { u_buf.add(n.try_into().unwrap()) };
                rbytes -= n;
                rdoff = (rdoff + n as u32) % ring.bufsz;

                if rbytes > 0 {
                    continue 'rbytes;
                } else {
                    break 'rbytes;
                }
            }

            ring.nesting -= 1;
            if ring.nesting == 0 {
                ring.fillsz.atomic_sub(ring.reserved as i32);
                ring.reserved = 0;
            }

            break 'outer;
        }
    }

    ring.lock.irq_unlock_noguard(flags);

    unsafe {
        rros_schedule();
    }

    ret
}

pub fn proxy_oob_write<T: IoBufferReader>(filp: &File, data: &mut T) -> isize {
    let fbind: *const RrosFileBinding =
        unsafe { (*filp.get_ptr()).private_data as *const RrosFileBinding };
    let proxy = unsafe { (*((*fbind).element)).pointer as *mut RrosProxy };
    let ring: &mut ProxyRing = unsafe { &mut (*proxy).output.ring };

    let mut ret: isize;

    if !proxy_is_writeable(unsafe { &(*proxy) }) {
        return -(bindings::ENXIO as isize);
    }

    loop {
        ret = do_proxy_write(filp, data as *const _ as *const c_char, data.len());
        if ret != -(bindings::EAGAIN as isize)
            || unsafe { (*filp.get_ptr()).f_flags } & bindings::O_NONBLOCK != 0
        {
            break;
        }

        ret = ring.oob_wait.wait() as isize;
        if ret == 0 {
            continue;
        } else {
            break;
        }
    }

    if ret == -(bindings::EIDRM as isize) {
        -(bindings::EBADF as isize)
    } else {
        ret
    }
}

pub fn proxy_oob_read<T: IoBufferWriter>(filp: &File, data: &mut T) -> isize {
    let fbind: *const RrosFileBinding =
        unsafe { (*filp.get_ptr()).private_data as *const RrosFileBinding };
    let proxy = unsafe { (*((*fbind).element)).pointer as *mut RrosProxy };
    let proxyin: &mut ProxyIn = unsafe { &mut (*proxy).input };
    let ring: &mut ProxyRing = &mut proxyin.ring;

    let mut request_done: bool = false;
    let mut ret: isize;

    if !proxy_is_readable(unsafe { &(*proxy) }) {
        return -(bindings::ENXIO as isize);
    }

    let count = data.len();
    if count == 0 {
        return 0;
    }

    loop {
        ret = do_proxy_read(filp, data as *const _ as *const c_char, data.len());
        if ret != 0 || unsafe { (*filp.get_ptr()).f_flags } & bindings::O_NONBLOCK != 0 {
            break;
        }

        if !request_done {
            proxyin.reqsz.atomic_add(count as i32);
            request_done = true;
        }

        ring.relay_work
            .call_inband_from(ring.wq.as_ref().unwrap().deref().get_ptr());
        ret = ring.oob_wait.wait() as isize;
        if ret != 0 {
            break;
        }
        if proxyin.on_eof.atomic_cmpxchg(1, 0) == 1 {
            ret = 0;
            break;
        }
    }

    if ret == -(bindings::EIDRM as isize) {
        -(bindings::EBADF as isize)
    } else {
        ret
    }
}

pub fn proxy_write<T: IoBufferReader>(filp: &File, data: &mut T) -> isize {
    let fbind: *const RrosFileBinding =
        unsafe { (*filp.get_ptr()).private_data as *const RrosFileBinding };
    let proxy = unsafe { (*((*fbind).element)).pointer as *mut RrosProxy };
    let ring: &mut ProxyRing = unsafe { &mut (*proxy).output.ring };
    let mut ret: isize;

    if !proxy_is_writeable(unsafe { &(*proxy) }) {
        return -(bindings::ENXIO as isize);
    }

    loop {
        ret = do_proxy_write(filp, data as *const _ as *const c_char, data.len());
        if ret != 0 || unsafe { (*filp.get_ptr()).f_flags } & bindings::O_NONBLOCK != 0 {
            break;
        }

        let condition = can_write_buffer(ring, data.len());
        ring.inband_wait.wait_event_interruptible(condition);

        if ret == 0 {
            continue;
        } else {
            break;
        }
    }

    ret
}

pub fn proxy_read<T: IoBufferWriter>(filp: &File, data: &mut T) -> isize {
    let fbind: *const RrosFileBinding =
        unsafe { (*filp.get_ptr()).private_data as *const RrosFileBinding };
    let proxy = unsafe { (*((*fbind).element)).pointer as *mut RrosProxy };
    let proxyin: &mut ProxyIn = unsafe { &mut (*proxy).input };

    let mut request_done: bool = false;
    let mut ret: isize;

    if !proxy_is_readable(unsafe { &(*proxy) }) {
        return -(bindings::ENXIO as isize);
    }

    let count = data.len();
    if count == 0 {
        return 0;
    }

    loop {
        ret = do_proxy_read(filp, data as *mut _ as *const c_char, data.len());
        if ret != 0 || unsafe { (*filp.get_ptr()).f_flags } & bindings::O_NONBLOCK != 0 {
            break;
        }

        if !request_done {
            proxyin.reqsz.atomic_add(count as i32);
            request_done = true;
        }

        let _ret = relay_input(unsafe { &mut (*proxy) });
        if proxyin.on_eof.atomic_cmpxchg(1, 0) == 1 {
            ret = 0;
            break;
        }
    }

    ret
}

pub fn init_output_ring(proxy: &mut RrosProxy, bufsz: u32, granularity: u32) -> Result<usize> {
    let ring = &mut proxy.output.ring;
    let bufmem = c_kzalloc(bufsz as u64);
    let wq: BoxedQueue = Queue::try_new(format_args!("{}", unsafe {
        *(rros_element_name(proxy.element.borrow_mut().deref()))
    }))?;

    ring.wq = Some(wq);
    ring.bufmem = bufmem.unwrap() as *mut u8;
    ring.bufsz = bufsz;
    ring.granularity = granularity;
    raw_spin_lock_init(&mut ring.lock);
    ring.relay_work
        .init_safe(relay_output_work, proxy.element.clone());
    ring.oob_wait.init();
    let mut key = waitqueue::LockClassKey::default();
    let name = unsafe {
        CStr::from_bytes_with_nul_unchecked("PROXY RING INBAND WAITQUEUE HEAD\0".as_bytes())
    };
    ring.inband_wait
        .init_waitqueue_head(name.as_ptr() as *const i8, &mut key);
    unsafe { raw_spin_lock_init(Arc::get_mut_unchecked(&mut ring.worker_lock.clone())) };

    Ok(0)
}

pub fn init_input_ring(proxy: &mut RrosProxy, bufsz: u32, granularity: u32) -> Result<usize> {
    let ring = &mut proxy.input.ring;
    let bufmem = c_kzalloc(bufsz as u64);
    let wq: BoxedQueue = Queue::try_new(format_args!("{}", unsafe {
        *(rros_element_name(proxy.element.borrow_mut().deref()))
    }))?;

    ring.wq = Some(wq);
    ring.bufmem = bufmem.unwrap() as *mut u8;
    ring.bufsz = bufsz;
    ring.granularity = granularity;
    raw_spin_lock_init(&mut ring.lock);
    ring.relay_work
        .init_safe(relay_input_work, proxy.element.clone());
    ring.oob_wait.init();
    let mut key = waitqueue::LockClassKey::default();
    let name = unsafe {
        CStr::from_bytes_with_nul_unchecked("PROXY RING INBAND WAITQUEUE HEAD\0".as_bytes())
    };
    ring.inband_wait
        .init_waitqueue_head(name.as_ptr() as *const i8, &mut key);
    unsafe { raw_spin_lock_init(Arc::get_mut_unchecked(&mut ring.worker_lock.clone())) };

    Ok(0)
}

fn proxy_factory_build(
    fac: &'static mut SpinLock<RrosFactory>,
    uname: &'static CStr,
    u_attrs: Option<*mut u8>,
    mut clone_flags: i32,
    _state_offp: &u32,
) -> Rc<RefCell<RrosElement>> {
    pr_debug!("clone_flags = {}", clone_flags);
    if clone_flags & !RROS_PROXY_CLONE_FLAGS != 0 {
        pr_warn!("invalid proxy clone flags");
    }
    pr_debug!("the u_attrs: {:p}", u_attrs.unwrap());

    let attrs = RrosProxyAttrs::from_ptr(u_attrs.unwrap() as *mut RrosProxyAttrs);
    pr_debug!("the attrs.fd is {}", attrs.fd);
    let bufsz = attrs.bufsz;

    if bufsz == 0 && (clone_flags & (RROS_CLONE_INPUT | RROS_CLONE_OUTPUT) != 0) {
        pr_warn!("invalid proxy bufsz value");
    }

    //If a granularity is set, the buffer size must be a multiple of the granule size.
    if attrs.granularity > 1 && bufsz % attrs.granularity > 0 {
        pr_warn!("invalid granularity value");
    }

    pr_debug!("clone_flags = {}", clone_flags);
    if bufsz > 0 && (clone_flags & (RROS_CLONE_INPUT | RROS_CLONE_OUTPUT) == 0) {
        clone_flags |= RROS_CLONE_OUTPUT;
    }
    pr_debug!("clone_flags = {}", clone_flags);

    let boxed_proxy = Box::try_new(RrosProxy::new(attrs.fd).unwrap()).unwrap();
    let proxy = Box::into_raw(boxed_proxy);
    unsafe {
        let ret = rros_init_user_element((*proxy).element.clone(), fac, uname, clone_flags);
        if let Err(_e) = ret {
            pr_err!("init user element failed");
        }

        // TODO: rros_init_poll_head()
        pr_debug!("clone_flags = {}", clone_flags);
        if (clone_flags & RROS_CLONE_OUTPUT) != 0 {
            let res = init_output_ring(&mut (*proxy), bufsz, attrs.granularity);

            if let Err(_e) = res {
                pr_err!("init proxy output ring failed");
            }
        }
        if (clone_flags & RROS_CLONE_INPUT) != 0 {
            let res = init_input_ring(&mut (*proxy), bufsz, attrs.granularity);

            if let Err(_e) = res {
                pr_err!("init proxy input ring failed");
            }
        }
    }
    // TODO: rros_index_factory_element();
    unsafe {
        (*(*proxy).element.borrow_mut()).pointer = proxy as *mut u8;
    }
    unsafe { (*proxy).element.clone() }
}

pub static mut RROS_PROXY_FACTORY: SpinLock<RrosFactory> = unsafe {
    SpinLock::new(RrosFactory {
        name: CStr::from_bytes_with_nul_unchecked("proxy\0".as_bytes()),
        // fops: Some(&RustFileProxy),
        nrdev: CONFIG_RROS_NR_PROXIES,
        build: Some(proxy_factory_build),
        dispose: Some(proxy_factory_dispose),
        attrs: None, //sysfs::attribute_group::new(),
        flags: crate::factory::RrosFactoryType::CLONE,
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
pub struct ProxyOps;

impl FileOpener<u8> for ProxyOps {
    fn open(shared: &u8, _fileref: &File) -> Result<Self::Wrapper> {
        let mut data = CloneData::default();
        data.ptr = shared as *const u8 as *mut u8;
        pr_debug!("open proxy device success");
        Ok(Box::try_new(data)?)
    }
}

impl FileOperations for ProxyOps {
    kernel::declare_file_operations!(read, write, oob_read, oob_write);

    type Wrapper = Box<CloneData>;

    fn read<T: IoBufferWriter>(
        _this: &CloneData,
        file: &File,
        data: &mut T,
        _offset: u64,
    ) -> Result<usize> {
        pr_debug!("I'm the read ops of the proxy factory.");
        let ret = proxy_read(file, data);
        pr_debug!("the result of proxy read is {}", ret);
        if ret < 0 {
            Err(Error::from_kernel_errno(ret.try_into().unwrap()))
        } else {
            Ok(ret as usize)
        }
    }

    fn oob_read<T: IoBufferWriter>(_this: &CloneData, file: &File, data: &mut T) -> Result<usize> {
        pr_debug!("I'm the oob_read ops of the proxy factory.");
        let ret = proxy_oob_read(file, data);
        pr_debug!("the result of proxy oob_read is {}", ret);
        if ret < 0 {
            Err(Error::from_kernel_errno(ret.try_into().unwrap()))
        } else {
            Ok(ret as usize)
        }
    }

    fn write<T: IoBufferReader>(
        _this: &CloneData,
        file: &File,
        data: &mut T,
        _offset: u64,
    ) -> Result<usize> {
        pr_debug!("I'm the write ops of the proxy factory.");
        let ret = proxy_write(file, data);
        pr_debug!("the result of proxy write is {}", ret);
        if ret < 0 {
            Err(Error::from_kernel_errno(ret.try_into().unwrap()))
        } else {
            Ok(ret as usize)
        }
    }

    fn oob_write<T: IoBufferReader>(_this: &CloneData, file: &File, data: &mut T) -> Result<usize> {
        pr_debug!("I'm the oob_write ops of the proxy factory.");
        let ret = proxy_oob_write(file, data);
        pr_debug!("the result of proxy oob_write is {}", ret);
        if ret < 0 {
            Err(Error::from_kernel_errno(ret.try_into().unwrap()))
        } else {
            Ok(ret as usize)
        }
    }

    fn release(_this: Box<CloneData>, _file: &File) {
        pr_debug!("I'm the release ops from the proxy ops.");
        // FIXME: put the rros element
    }
}

pub fn proxy_factory_dispose(_ele: RrosElement) {}
