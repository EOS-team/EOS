use alloc::rc::Rc;

use core::{cell::RefCell, convert::TryFrom, mem::size_of, sync::atomic::AtomicUsize};

use crate::{
    clock, factory,
    factory::{RrosElement, RrosFactory},
    fifo::RROS_FIFO_MAX_PRIO,
    list, sched,
    wait::RrosWaitQueue,
};

use kernel::{
    c_types, device::DeviceType, file::File, file_operations::FileOperations,
    io_buffer::IoBufferWriter, prelude::*, spinlock_init, str::CStr, sync::SpinLock, user_ptr,
    Error,
};

#[allow(dead_code)]
pub struct RrosMonitorItem1 {
    pub mutex: SpinLock<i32>,
    pub events: list::ListHead,
    pub lock: SpinLock<i32>,
}

impl RrosMonitorItem1 {
    #[allow(dead_code)]
    fn new() -> Result<Self> {
        Ok(Self {
            mutex: unsafe { SpinLock::new(0) },
            events: list::ListHead::default(),
            lock: unsafe { SpinLock::<i32>::new(0) },
        })
    }
}

#[allow(dead_code)]
pub struct RrosMonitorItem2 {
    pub wait_queue: RrosWaitQueue,
    pub gate: Option<*mut u8>,
    pub poll_head: sched::RrosPollHead,
    pub next: list::ListHead,
    pub next_poll: list::ListHead,
}

impl RrosMonitorItem2 {
    #[allow(dead_code)]
    fn new() -> Result<Self> {
        Ok(Self {
            wait_queue: unsafe { core::mem::zeroed() },
            gate: None,
            poll_head: sched::RrosPollHead::new(),
            next: list::ListHead::default(),
            next_poll: list::ListHead::default(),
        })
    }
}

#[allow(dead_code)]
pub enum RrosMonitorItem {
    Item1(RrosMonitorItem1),
    Item2(RrosMonitorItem2),
}

#[allow(dead_code)]
pub struct RrosMonitor {
    pub element: Rc<RefCell<RrosElement>>,
    pub state: Option<RrosMonitorState>,
    pub type_foo: i32,
    pub protocol: i32,
    pub item: RrosMonitorItem,
}

impl RrosMonitor {
    #[allow(dead_code)]
    pub fn new(
        element: Rc<RefCell<RrosElement>>,
        state: Option<RrosMonitorState>,
        type_foo: i32,
        protocol: i32,
        item: RrosMonitorItem,
    ) -> Result<Self> {
        match item {
            RrosMonitorItem::Item1(subitem) => Ok(Self {
                element,
                state,
                type_foo,
                protocol,
                item: RrosMonitorItem::Item1(subitem),
            }),
            RrosMonitorItem::Item2(subitem) => Ok(Self {
                element,
                state,
                type_foo,
                protocol,
                item: RrosMonitorItem::Item2(subitem),
            }),
        }
    }
}

pub struct RrosMonitorStateItemGate {
    #[allow(dead_code)]
    owner: AtomicUsize,
    #[allow(dead_code)]
    ceiling: u32,
    #[allow(dead_code)]
    recursive: u32,
    #[allow(dead_code)]
    nesting: u32,
}

pub struct RrosMonitorStateItemEvent {
    #[allow(dead_code)]
    value: AtomicUsize,
    #[allow(dead_code)]
    pollrefs: AtomicUsize,
    #[allow(dead_code)]
    gate_offset: u32,
}

// union RrosMonitorState_item {
//     gate: RrosMonitorState_item_gate,
//     event: RrosMonitorState_item_event,
// }

#[allow(dead_code)]
pub enum RrosMonitorStateItem {
    Gate(RrosMonitorStateItemGate),
    Event(RrosMonitorStateItemEvent),
}

pub struct RrosMonitorState {
    pub flags: u32,
    pub u: Option<RrosMonitorStateItem>,
}

impl RrosMonitorState {
    #[allow(dead_code)]
    pub fn new() -> Result<Self> {
        Ok(Self { flags: 0, u: None })
    }
}

#[allow(dead_code)]
pub struct RrosMonitorAttrs {
    clockfd: u32,
    type_foo: u32,
    protocol: u32,
    initval: u32,
}

impl RrosMonitorAttrs {
    #[allow(dead_code)]
    fn new() -> Result<Self> {
        Ok(Self {
            clockfd: 0,
            type_foo: 0,
            protocol: 0,
            initval: 0,
        })
    }
}

#[allow(dead_code)]
pub const RROS_MONITOR_EVENT: u32 = 0; /* Event monitor. */
#[allow(dead_code)]
pub const RROS_EVENT_GATED: u32 = 0; /* Gate protected. */
#[allow(dead_code)]
pub const RROS_EVENT_COUNT: u32 = 1; /* Semaphore. */
#[allow(dead_code)]
pub const RROS_EVENT_MASK: u32 = 2; /* Event (bit)mask. */
#[allow(dead_code)]
pub const RROS_MONITOR_GATE: u32 = 1; /* Gate monitor. */
#[allow(dead_code)]
pub const RROS_GATE_PI: u32 = 0; /* Gate with priority inheritance. */
#[allow(dead_code)]
pub const RROS_GATE_PP: u32 = 1; /* Gate with priority protection (ceiling). */

#[allow(dead_code)]
pub const RROS_MONITOR_NOGATE: u32 = 1;
#[allow(dead_code)]
pub const CLOCK_MONOTONIC: u32 = 1;
#[allow(dead_code)]
pub const CLOCK_REALTIME: u32 = 0;

#[allow(dead_code)]
const CONFIG_RROS_MONITOR: usize = 0; // Unknown.

#[allow(dead_code)]
pub fn monitor_factory_build(
    _fac: *mut RrosFactory,
    _uname: &'static CStr,
    _u_attrs: Option<*mut u8>,
    clone_flags: i32,
    _state_offp: &u32,
) -> Result<Rc<RefCell<RrosElement>>> {
    if (clone_flags & !factory::RROS_CLONE_PUBLIC) != 0 {
        return Err(Error::EINVAL);
    }

    let mut attrs = RrosMonitorAttrs::new()?;
    let len = size_of::<RrosMonitorAttrs>();
    let ptr: *mut c_types::c_void = &mut attrs as *mut RrosMonitorAttrs as *mut c_types::c_void;
    let u_attrs: *const c_types::c_void =
        &attrs as *const RrosMonitorAttrs as *const c_types::c_void;
    let ret = unsafe { user_ptr::rust_helper_copy_from_user(ptr, u_attrs as _, len as _) };
    if ret != 0 {
        return Err(Error::EFAULT);
    }

    match attrs.type_foo {
        RROS_MONITOR_GATE => match attrs.protocol {
            RROS_GATE_PP => {
                if attrs.initval == 0 || attrs.initval > RROS_FIFO_MAX_PRIO as u32 {
                    return Err(Error::EINVAL);
                }
            }
            RROS_GATE_PI => {
                if attrs.initval != 0 {
                    return Err(Error::EINVAL);
                }
            }
            _ => return Err(Error::EINVAL),
        },
        RROS_MONITOR_EVENT => match attrs.protocol {
            RROS_EVENT_GATED | RROS_EVENT_COUNT | RROS_EVENT_MASK => (),
            _ => return Err(Error::EINVAL),
        },
        _ => return Err(Error::EINVAL),
    }

    let _clock: Result<&mut clock::RrosClock> = {
        match attrs.clockfd {
            CLOCK_MONOTONIC => unsafe { Ok(&mut clock::RROS_MONO_CLOCK) },
            _ => unsafe { Ok(&mut clock::RROS_REALTIME_CLOCK) },
        }
    };

    let element = Rc::try_new(RefCell::new(RrosElement::new()?))?;
    let factory: &mut SpinLock<RrosFactory> = unsafe { &mut RROS_MONITOR_FACTORY };
    let _ret = factory::rros_init_element(element.clone(), factory, clone_flags);

    let mut state = RrosMonitorState::new()?;

    match attrs.type_foo {
        RROS_MONITOR_GATE => match attrs.protocol {
            RROS_GATE_PP => {
                state.u = Some(RrosMonitorStateItem::Gate(RrosMonitorStateItemGate {
                    owner: AtomicUsize::new(0),
                    ceiling: attrs.initval,
                    recursive: 0,
                    nesting: 0,
                }));
            }
            RROS_GATE_PI => {
                ();
            }
            _ => (),
        },
        RROS_MONITOR_EVENT => {
            state.u = Some(RrosMonitorStateItem::Event(RrosMonitorStateItemEvent {
                value: AtomicUsize::new(usize::try_from(attrs.initval)?),
                pollrefs: AtomicUsize::new(0),
                gate_offset: RROS_MONITOR_NOGATE,
            }));
        }
        _ => (),
    }

    // init monitor
    let mon = match state.u {
        Some(RrosMonitorStateItem::Gate(ref _rros_monitor_state_item_gate)) => {
            let mut item = RrosMonitorItem1::new()?;
            let pinned = unsafe { Pin::new_unchecked(&mut item.mutex) };
            spinlock_init!(pinned, "RrosMonitorItem1_lock");

            let pinned = unsafe { Pin::new_unchecked(&mut item.lock) };
            spinlock_init!(pinned, "value");
            RrosMonitor::new(
                element,
                Some(state),
                attrs.type_foo as i32,
                attrs.protocol as i32,
                RrosMonitorItem::Item1(item),
            )?
        }
        _ => {
            let item = RrosMonitorItem2::new()?;
            RrosMonitor::new(
                element,
                Some(state),
                attrs.type_foo as i32,
                attrs.protocol as i32,
                RrosMonitorItem::Item2(item),
            )?
        }
    };

    // *state_offp = rros_shared_offset(state) // todo
    // rros_index_factory_element(&mon->element)

    return Ok(mon.element);
}

#[allow(dead_code)]
pub static mut RROS_MONITOR_FACTORY: SpinLock<factory::RrosFactory> = unsafe {
    SpinLock::new(factory::RrosFactory {
        name: CStr::from_bytes_with_nul_unchecked("monitor\0".as_bytes()),
        // fops: Some(&MonitorOps),
        nrdev: CONFIG_RROS_MONITOR,
        build: None,
        dispose: Some(monitor_factory_dispose),
        attrs: None, //sysfs::attribute_group::new(),
        flags: factory::RrosFactoryType::CLONE,
        inside: Some(factory::RrosFactoryInside {
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

#[allow(dead_code)]
pub fn monitor_factory_dispose(_ele: factory::RrosElement) {}

struct MonitorOps;

impl FileOperations for MonitorOps {
    kernel::declare_file_operations!(read);

    fn read<T: IoBufferWriter>(
        _this: &Self,
        _file: &File,
        _data: &mut T,
        _offset: u64,
    ) -> Result<usize> {
        pr_debug!("I'm the read ops of the rros monitor factory.");
        Ok(1)
    }
}
