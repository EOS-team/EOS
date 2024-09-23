use core::{
    borrow::BorrowMut,
    cell::{RefCell, RefMut},
    mem::size_of,
    ops::DerefMut,
    ptr::NonNull,
    usize,
};

use crate::{
    factory,
    file::{
        rros_get_file, rros_ignore_fd, rros_open_file, rros_put_file, rros_release_file,
        rros_watch_fd, RrosFile,
    },
    flags::RrosFlag,
    init_list_head,
    list_add,
    list_del,
    list_empty,
    list_entry,
    list_entry_is_head,
    list_first_entry,
    list_for_each_entry,
    list_next_entry,
    // mutex::{self, rros_init_kmutex, rros_lock_kmutex, rros_unlock_kmutex, RrosKMutex},
    pr_debug,
    pr_err,
    sched::{rros_schedule, RrosThread, RrosValue},
    thread::{rros_current, T_RMID},
    timeout::RrosTmode,
};
use alloc::{rc::Rc, sync::Arc};
use bindings::{list_head, POLLERR, POLLHUP, POLLIN, POLLNVAL, POLLOUT, POLLRDNORM, POLLWRNORM};
use kernel::{
    bindings, c_types, container_of,
    device::DeviceType,
    error::Result,
    file::File,
    file_operations::{FileOpener, FileOperations, IoctlCommand},
    io_buffer::{IoBufferReader, IoBufferWriter},
    ktime::{self, timespec64_to_ktime, Timespec64},
    linked_list::{GetLinks, Links, List},
    prelude::*,
    rbtree, spinlock_init,
    str::CStr,
    sync::Lock,
    sync::SpinLock,
    user_ptr::UserSlicePtr,
    Error,
};

const POLLER_NEST_MAX: i32 = 4;
const RROS_POLL_NR_CONNECTORS: usize = 4;

pub const RROS_POLL_IOCBASE: u32 = 'p' as u32;

pub const RROS_POLL_CTLADD: u32 = 0;
pub const RROS_POLL_CTLDEL: u32 = 1;
pub const RROS_POLL_CTLMOD: u32 = 2;

// use sizeof RrosPollCtlreq, size of RrosPollCtlreq in c is 24 bytes, so there is (i64,i64,i64)
pub const RROS_POLIOC_CTL: u32 = kernel::ioctl::_IOW::<(i64, i64, i64)>(RROS_POLL_IOCBASE, 0);
pub const RROS_POLIOC_WAIT: u32 = kernel::ioctl::_IOWR::<(i64, i64, i64)>(RROS_POLL_IOCBASE, 1);

pub struct RrosPollGroup {
    pub item_index: rbtree::RBTree<u32, Arc<RrosPollItem>>,
    pub item_list: List<Arc<RrosPollItem>>,
    pub waiter_list: SpinLock<List<Arc<RrosPollWaiter>>>,
    pub rfile: RrosFile,
    // pub item_lock: mutex::RrosKMutex,
    pub nr_items: i32,
    pub generation: u32,
}

impl RrosPollGroup {
    pub fn new() -> Self {
        Self {
            item_index: rbtree::RBTree::new(),
            item_list: List::new(),
            waiter_list: unsafe { SpinLock::new(List::new()) },
            rfile: RrosFile::new(),
            // item_lock: mutex::RrosKMutex::new(),
            nr_items: 0,
            generation: 0,
        }
    }

    pub fn init(&mut self) {
        let pinned = unsafe { Pin::new_unchecked(&mut self.waiter_list) };
        spinlock_init!(pinned, "RrosPollGroup::waiter_list");
        //FIXME: init kmutex fail
        // rros_init_kmutex(&mut item_lock as *mut RrosKMutex);
    }

    #[inline]
    pub fn new_generation(&mut self) -> u32 {
        self.generation += 1;
        if self.generation == 0 {
            self.generation = 1;
        }
        self.generation
    }

    #[inline]
    pub fn flush_item(&mut self) {
        drop(&mut self.item_list);
    }
}

pub struct RrosPollItem {
    pub fd: u32,
    pub events_polled: i32,
    pub next: Links<RrosPollItem>,
    pub pollval: Option<Rc<RefCell<RrosValue>>>,
}

impl GetLinks for RrosPollItem {
    type EntryType = RrosPollItem;
    fn get_links(data: &Self::EntryType) -> &Links<Self::EntryType> {
        &data.next
    }
}

impl RrosPollItem {
    pub fn new() -> Self {
        Self {
            fd: 0,
            events_polled: 0,
            next: Links::new(),
            pollval: None,
        }
    }
}
pub struct RrosPollWaiter {
    pub flag: Option<Rc<RefCell<RrosFlag>>>,
    pub next: Links<RrosPollWaiter>,
}

impl GetLinks for RrosPollWaiter {
    type EntryType = RrosPollWaiter;
    fn get_links(data: &Self::EntryType) -> &Links<Self::EntryType> {
        &data.next
    }
}

impl RrosPollWaiter {
    pub fn new() -> Self {
        Self {
            flag: Some(Rc::try_new(RefCell::new(RrosFlag::new())).unwrap()),
            next: Links::new(),
        }
    }
}

pub struct RrosPollHead {
    pub watchpoints: SpinLock<list_head>,
}

impl RrosPollHead {
    pub fn new() -> Self {
        Self {
            watchpoints: unsafe { SpinLock::new(list_head::default()) },
        }
    }

    pub fn init(&mut self) {
        init_list_head!(self.watchpoints.locked_data().get());
        spinlock_init!(
            unsafe { Pin::new_unchecked(&mut self.watchpoints) },
            "RrosPollHead"
        );
    }
}
pub struct RrosPollConnector {
    pub head: Option<NonNull<RrosPollHead>>,
    pub next: list_head,
    pub unwatch: Option<fn(head: &RrosPollHead) -> ()>,
    pub events_received: i32,
    pub index: i32,
}

impl RrosPollConnector {
    pub fn new() -> Self {
        Self {
            head: None,
            next: list_head::default(),
            unwatch: None,
            events_received: 0,
            index: 0,
        }
    }
}

pub struct OobPollWait {
    pub connectors: Vec<RrosPollConnector>,
}

impl OobPollWait {
    pub fn new() -> Self {
        let mut connectors = Vec::try_with_capacity(RROS_POLL_NR_CONNECTORS).unwrap();
        for _ in 0..RROS_POLL_NR_CONNECTORS {
            connectors.try_push(RrosPollConnector::new()).unwrap();
        }
        Self { connectors }
    }
}

pub struct RrosPollNode {
    pub next: list_head,
}

impl RrosPollNode {
    pub fn new() -> Self {
        Self {
            next: list_head::default(),
        }
    }
}

pub struct RrosPollWatchpoint {
    pub fd: u32,
    pub events_polled: i32,
    pub pollval: Option<Rc<RefCell<RrosValue>>>,
    pub wait: OobPollWait,
    pub flag: Option<Rc<RefCell<RrosFlag>>>,
    pub filp: Option<File>,
    pub node: RrosPollNode,
}

impl RrosPollWatchpoint {
    pub fn new() -> Self {
        Self {
            fd: 0,
            events_polled: 0,
            pollval: None,
            wait: OobPollWait::new(),
            flag: None,
            filp: None,
            node: RrosPollNode::new(),
        }
    }
}
pub struct RrosPollCtlreq {
    pub action: u32,
    pub fd: u32,
    pub events: u32,
    pub pollval: RrosValue,
}

impl RrosPollCtlreq {
    pub fn new() -> Self {
        Self {
            action: 0,
            fd: 0,
            events: 0,
            pollval: RrosValue::new(),
        }
    }
}

#[repr(C)]
struct CPollEvent {
    fd: u32,
    events: u32,
    pollval: Pollvalue,
}

impl CPollEvent {
    fn new() -> Self {
        Self {
            fd: 0,
            events: 0,
            pollval: Pollvalue {
                ptr: core::ptr::null_mut(),
            },
        }
    }
}
#[repr(C)]
union Pollvalue {
    val: i32,
    lval: i64,
    ptr: *mut c_types::c_void,
}

pub struct RrosPollEvent {
    pub fd: u32,
    pub events: u32,
    pub pollval: Option<Rc<RefCell<RrosValue>>>,
}

impl RrosPollEvent {
    pub fn new() -> Self {
        Self {
            fd: 0,
            events: 0,
            pollval: None,
        }
    }
}

#[repr(C)]
pub struct RrosPollWaitreq {
    pub timeout_ptr: u64,
    pub pollset_ptr: u64,
    nrset: i32,
}

impl RrosPollWaitreq {
    pub fn new() -> Self {
        Self {
            timeout_ptr: 0,
            pollset_ptr: 0,
            nrset: 0,
        }
    }
}

impl RrosThread {
    pub fn drop_poll_table(&mut self) {
        match self.poll_context.table.as_ref() {
            Some(table) => {
                drop(table);
            }
            None => (),
        }
    }
}

fn connect_watchpoint(
    wait: &mut OobPollWait,
    head: NonNull<RrosPollHead>,
    unwatch: Option<fn(head: &RrosPollHead) -> ()>,
) -> () {
    let mut i = 0;
    for poco in wait.connectors.as_mut_slice() {
        if poco.head.is_none() {
            poco.head = Some(head);
            poco.unwatch = unwatch;
            poco.events_received = 0;
            list_add!(
                &mut poco.next,
                head.as_ref().watchpoints.locked_data().get()
            );
            poco.index = i as i32;
            return;
        }
        if poco.head.unwrap().as_ptr() == head.as_ptr() {
            pr_err!("poll: connect_watchpoint: duplicated head\n");
            return;
        }
        i += 1;
    }
    pr_err!("poll: connect_watchpoint: no free connectors\n");
}

pub fn rros_poll_watch(
    head: NonNull<RrosPollHead>,
    wait: &mut OobPollWait,
    unwatch: Option<fn(head: &RrosPollHead) -> ()>,
) -> () {
    let flags: u64;

    unsafe {
        flags = head.as_ref().watchpoints.irq_lock_noguard();
    }
    connect_watchpoint(wait, head, unwatch);
    unsafe {
        head.as_ref().watchpoints.irq_unlock_noguard(flags);
    }
}

pub fn __rros_signal_poll_events(head: &mut RrosPollHead, events: i32) -> () {
    let mut wpt: &mut RrosPollWatchpoint;
    let flags: u64;
    let mut ready: i32;

    flags = head.watchpoints.irq_lock_noguard();
    list_for_each_entry!(
        poco,
        &*head.watchpoints.locked_data().get(),
        RrosPollConnector,
        {
            unsafe {
                wpt = &mut *(container_of!(
                    poco,
                    RrosPollWatchpoint,
                    wait.connectors[(*poco).index as usize]
                ) as *mut RrosPollWatchpoint);
                ready = events & wpt.events_polled;
                if ready != 0 {
                    (*poco).events_received |= ready;
                    (Rc::get_mut(wpt.flag.as_mut().unwrap()).unwrap().get_mut()).raise_nosched();
                }
            }
        },
        next
    );
    head.watchpoints.irq_unlock_noguard(flags);
}

pub fn rros_signal_poll_events(head: &mut RrosPollHead, events: i32) {
    if !list_empty!(head.watchpoints.locked_data().get() as *const list_head) {
        __rros_signal_poll_events(head, events);
    }
}

fn check_no_loop_deeper(origin: &File, item: &RrosPollItem, depth: i32) -> Result<i32> {
    let group: &mut RrosPollGroup;
    let mut ret: Result<i32> = Ok(0);
    let rfilp: &mut RrosFile;

    pr_debug!("poll: check_no_loop_deeper: depth is {}", depth);
    pr_debug!(
        "poll: check_no_loop_deeper: address of item is {:p}",
        item as *const RrosPollItem
    );
    pr_debug!("poll: check_no_loop_deeper: item.fd is {}", item.fd);
    if depth >= POLLER_NEST_MAX {
        return Err(Error::ELOOP);
    }

    'outer: loop {
        match rros_get_file(item.fd) {
            Some(mut r) => {
                rfilp = unsafe { r.as_mut() };
                pr_debug!(
                    "poll: check_no_loop_deeper: rfilp is {:p}, rfilp.filp.private_data is {:p}",
                    rfilp as *mut RrosFile,
                    unsafe { (*rfilp.filp).private_data }
                );
                unsafe {
                    if rfilp.get_name().unwrap() != "poll" {
                        break 'outer;
                    }
                    if (*rfilp.filp).private_data == (*origin.ptr).private_data {
                        ret = Err(Error::ELOOP);
                        break 'outer;
                    }
                    //TODO: not sure if here need offset(8). figure it out
                    group = &mut *(((*rfilp.filp).private_data as *mut u8).offset(8)
                        as *mut RrosPollGroup);
                    // group = &mut *((*rfilp.filp).private_data as *mut RrosPollGroup);
                }
            }
            None => {
                pr_err!("poll: check_no_loop_deeper: item.fd is \"invalid\" ");
                return Ok(0);
            }
        }

        // rros_lock_kmutex(&mut group.item_lock as *mut RrosKMutex);
        let mut cursor = group.item_list.cursor_front();
        while cursor.current().is_some() {
            let item = cursor.current().unwrap();
            ret = check_no_loop_deeper(origin, item, depth + 1);
            if ret.is_err() {
                break;
            }
            cursor.move_next();
        }
        // rros_unlock_kmutex(&mut group.item_lock as *mut RrosKMutex);
        break;
    }
    if let Err(e) = rros_put_file(rfilp) {
        return Err(e);
    }

    return ret;
}

fn check_no_loop(group: &File, item: &RrosPollItem) -> Result<i32> {
    return check_no_loop_deeper(group, item, 0);
}

fn add_item(filp: &File, group: &mut RrosPollGroup, creq: RrosPollCtlreq) -> Result<i32> {
    pr_debug!(
        "poll: add_item: creq.fd = {}, creq.events = {}",
        creq.fd,
        creq.events
    );
    pr_debug!("poll: add_item: filp.ptr.private_data is {:p}", unsafe {
        (*filp.ptr).private_data
    });
    let mut item: RrosPollItem = RrosPollItem::new();
    let rfilp: &mut RrosFile;
    let mut ret: Result<i32> = Ok(0);
    let events: i32;

    item.fd = creq.fd;
    events = (creq.events & (!POLLNVAL)) as i32;
    item.events_polled = events | POLLERR as i32 | POLLHUP as i32;
    item.pollval = Some(Rc::try_new(RefCell::new(creq.pollval))?);

    'fail_get: loop {
        'fail_add: loop {
            match rros_get_file(creq.fd) {
                Some(mut f) => {
                    rfilp = unsafe { f.as_mut() };
                }
                None => {
                    ret = Err(Error::EBADF);
                    break 'fail_get;
                }
            }

            // rros_lock_kmutex(&mut group.item_lock as *mut RrosKMutex);
            if let Err(e) = check_no_loop(filp, &item) {
                ret = Err(e);
                pr_err!("poll: add_item: check_no_loop is false");
                break 'fail_add;
            }

            let item_fd = item.fd;
            let arc_item = Arc::try_new(item).unwrap();
            if group
                .item_index
                .try_insert(item_fd, arc_item.clone())
                .is_err()
            {
                pr_err!("poll: add_item: try_insert is error");
                break 'fail_add;
            }

            group.item_list.push_front(arc_item);
            group.nr_items += 1;
            group.new_generation();
            // rros_unlock_kmutex(&mut group.item_lock as *mut RrosKMutex);
            if let Err(e) = rros_put_file(rfilp) {
                return Err(e);
            }

            return Ok(0);
        }
        // rros_unlock_kmutex(&mut group.item_lock as *mut RrosKMutex);
        if let Err(e) = rros_put_file(rfilp) {
            return Err(e);
        }
        break;
    }

    return ret;
}

fn del_item(group: &mut RrosPollGroup, creq: RrosPollCtlreq) -> Result<i32> {
    let item: Arc<RrosPollItem>;

    // rros_lock_kmutex(&mut group.item_lock as *mut RrosKMutex);
    match group.item_index.remove(&creq.fd) {
        Some(a) => item = a,
        None => {
            // rros_unlock_kmutex(&mut group.item_lock as *mut RrosKMutex);
            return Err(Error::ENOENT);
        }
    }

    unsafe { group.item_list.remove(&item) };
    group.nr_items -= 1;
    group.new_generation();
    // rros_unlock_kmutex(&mut group.item_lock as *mut RrosKMutex);
    return Ok(0);
}

pub fn rros_drop_watchpoints(drop_list: *mut list_head) {
    let mut wpt: &mut RrosPollWatchpoint;

    list_for_each_entry!(
        node,
        drop_list,
        RrosPollNode,
        {
            unsafe {
                if (*drop_list).next == drop_list {
                    break;
                }
            }
            wpt = unsafe {
                &mut *(container_of!(node, RrosPollWatchpoint, node) as *mut RrosPollWatchpoint)
            };

            // FIXME: there is a bug!!!
            for poco in wpt.wait.connectors.as_mut_slice() {
                if poco.head.is_some() {
                    let _guard = unsafe { poco.head.as_ref().unwrap().as_ref().watchpoints.lock() };
                    poco.events_received |= POLLNVAL as i32;
                    if poco.unwatch.is_some() {
                        unsafe {
                            (poco.unwatch.as_ref().unwrap())(poco.head.as_ref().unwrap().as_ref())
                        };
                    }
                }
            }
            unsafe {
                Rc::get_mut_unchecked(wpt.flag.as_mut().unwrap())
                    .get_mut()
                    .raise_nosched();
            }
            wpt.filp = None;
        },
        next
    );
}

fn mod_item(group: &mut RrosPollGroup, creq: RrosPollCtlreq) -> Result<i32> {
    let events: i32;

    events = (creq.events & (!POLLNVAL)) as i32;
    // rros_lock_kmutex(&mut group.item_lock as *mut RrosKMutex);

    match group.item_index.get_mut(&creq.fd) {
        Some(i) => {
            let mut item: &mut RrosPollItem = unsafe { Arc::get_mut_unchecked(i) };
            item.events_polled = events | POLLERR as i32 | POLLHUP as i32;
            if let Some(r) = Rc::get_mut(item.pollval.as_mut().unwrap()) {
                let pvref = r.try_borrow_mut();
                match pvref {
                    Ok(mut pv) => {
                        *pv = creq.pollval;
                    }
                    Err(e) => {
                        pr_err!("poll: mod_item: try_borrow_mut is error");
                        return Err(Error::from(e));
                    }
                }
            }
        }
        None => {
            // rros_unlock_kmutex(&mut group.item_lock as *mut RrosKMutex);
            return Err(Error::ENOENT);
        }
    }
    group.new_generation();
    // rros_unlock_kmutex(&mut group.item_lock as *mut RrosKMutex);
    return Ok(0);
}

fn setup_item(filp: &File, group: &mut RrosPollGroup, creq: RrosPollCtlreq) -> Result<i32> {
    let ret: Result<i32>;

    match creq.action {
        RROS_POLL_CTLADD => {
            ret = add_item(filp, group, creq);
        }
        RROS_POLL_CTLDEL => {
            ret = del_item(group, creq);
        }
        RROS_POLL_CTLMOD => {
            ret = mod_item(group, creq);
        }
        _ => {
            ret = Err(Error::EINVAL);
        }
    }
    ret
}

fn collect_events(
    group: &mut RrosPollGroup,
    u_set: *mut CPollEvent,
    maxevents: i32,
    flag: &Option<Rc<RefCell<RrosFlag>>>,
) -> Result<i32> {
    let mut curr = unsafe { &mut *(*rros_current()).locked_data().get() };
    let mut n: usize;
    let nr: usize;
    let mut count: i32 = 0;
    let mut ready: i32;
    let mut ev: RrosPollEvent = RrosPollEvent::new();
    let mut cev: CPollEvent = CPollEvent::new();
    let mut generation: u32;
    let mut rfilp: &mut RrosFile;
    let mut filp: File;
    let mut table: Vec<RrosPollWatchpoint, alloc::alloc_rros::RrosMem>;

    // rros_lock_kmutex(&mut group.item_lock as *mut RrosKMutex);
    nr = group.nr_items as usize;
    if nr == 0 {
        // rros_lock_kmutex(&mut group.item_lock as *mut RrosKMutex);
        return Err(Error::EINVAL);
    }
    'stale: loop {
        'collect: loop {
            if flag.is_none() {
                break 'collect;
            }
            if group.generation == curr.poll_context.generation {
                break 'collect;
            }
            loop {
                generation = group.generation;
                // rros_unlock_kmutex(&mut group.item_lock as *mut RrosKMutex);
                curr.drop_poll_table();
                match Vec::try_with_capacity_in(nr, alloc::alloc_rros::RrosMem) {
                    Ok(t) => {
                        table = t;
                        for _ in 0..nr {
                            table.try_push(RrosPollWatchpoint::new()).unwrap();
                        }
                    }
                    Err(e) => {
                        curr.poll_context.nr = 0;
                        curr.poll_context.active = 0;
                        curr.poll_context.table = None;
                        curr.poll_context.generation = 0;
                        return Err(Error::from(e));
                    }
                }
                // rros_lock_kmutex(&mut group.item_lock as *mut RrosKMutex);
                if generation == group.generation {
                    break;
                }
            }

            curr.poll_context.table = Some(table);
            curr.poll_context.nr = nr as i32;
            curr.poll_context.generation = generation;

            // build the poll table
            if let Some(wpt) = curr.poll_context.table.as_mut() {
                let mut i: usize = 0;
                let w = wpt.as_mut_slice();
                let mut cursor = group.item_list.cursor_front();
                while cursor.current().is_some() {
                    let item = cursor.current().unwrap();
                    w[i].fd = item.fd;
                    w[i].events_polled = item.events_polled;
                    w[i].pollval = Some(item.pollval.as_ref().unwrap().clone());
                    i += 1;
                    cursor.move_next();
                }
            }
            break 'collect;
        }
        // rros_unlock_kmutex(&mut group.item_lock as *mut RrosKMutex);

        if flag.is_some() {
            curr.poll_context.active = 0;
        }
        n = 0;
        if let Some(wpt) = curr.poll_context.table.borrow_mut() {
            let w = wpt.as_mut_slice();
            while n < nr {
                if flag.is_some() {
                    w[n].flag = Some(flag.as_ref().unwrap().clone());
                    for poco in w[n].wait.connectors.as_mut_slice() {
                        if poco.head.is_some() {
                            poco.head = None;
                            init_list_head!(&mut poco.next);
                        }
                    }
                    ready = POLLIN as i32 | POLLOUT as i32 | POLLRDNORM as i32 | POLLWRNORM as i32;
                    match rros_watch_fd(w[n].fd, &mut w[n].node) {
                        Some(mut r) => rfilp = unsafe { r.as_mut() },
                        None => break 'stale,
                    };
                    filp = File { ptr: rfilp.filp };
                    curr.poll_context.active += 1;
                    unsafe {
                        if (*(*filp.ptr).f_op).oob_poll.is_some() {
                            ready = (*(*filp.ptr).f_op).oob_poll.as_ref().unwrap()(
                                filp.ptr,
                                &mut w[n].wait as *mut OobPollWait as *mut _
                                    as *mut bindings::oob_poll_wait,
                            ) as i32;
                        }
                    }
                    w[n].filp = Some(filp);
                    if let Err(e) = rros_put_file(rfilp) {
                        return Err(e);
                    }
                } else {
                    ready = 0;
                    for poco in w[n].wait.connectors.as_slice() {
                        if poco.head.is_some() {
                            ready |= poco.events_received;
                        }
                    }
                }

                ready &= w[n].events_polled | POLLNVAL as i32;
                if ready != 0 {
                    ev.fd = w[n].fd;
                    ev.pollval = Some(w[n].pollval.as_ref().unwrap().clone());
                    ev.events = ready as u32;
                    cev.events = ev.events;
                    cev.fd = ev.fd;
                    match unsafe { &*ev.pollval.as_ref().unwrap().as_ptr() } {
                        RrosValue::Lval(l) => {
                            cev.pollval = Pollvalue { lval: *l };
                        }
                        RrosValue::Val(v) => {
                            cev.pollval = Pollvalue { val: *v };
                        }
                        RrosValue::Ptr(p) => {
                            cev.pollval = Pollvalue { ptr: *p };
                        }
                    }
                    unsafe {
                        if UserSlicePtr::new(u_set as *mut c_types::c_void, size_of::<CPollEvent>())
                            .writer()
                            .write_raw(
                                &mut cev as *mut CPollEvent as *const u8,
                                size_of::<CPollEvent>(),
                            )
                            .is_err()
                        {
                            return Err(Error::EFAULT);
                        }
                    }

                    unsafe { u_set.offset(1) };
                    count += 1;
                    if count >= maxevents {
                        break;
                    }
                }
                n += 1;
            }
        }
        return Ok(count);
    }
    // rros_lock_kmutex(&mut group.item_lock as *mut RrosKMutex);
    group.new_generation();
    // rros_unlock_kmutex(&mut group.item_lock as *mut RrosKMutex);
    return Err(Error::EBADF);
}

fn clear_wait() {
    let curr: &mut RrosThread = unsafe { &mut *(*rros_current()).locked_data().get() };
    let mut wpt: &mut RrosPollWatchpoint;
    let mut flags: u64;
    let mut n: i32;

    n = 0;
    while n < curr.poll_context.active {
        wpt = &mut curr.poll_context.table.as_mut().unwrap().as_mut_slice()[n as usize];
        if let Err(_) = rros_ignore_fd(&mut wpt.node) {
            pr_err!("clear_wait: rros_ignore_fd is failed");
        }
        for poco in wpt.wait.connectors.as_mut_slice() {
            if poco.head.is_some() {
                unsafe {
                    flags = poco.head.unwrap().as_ref().watchpoints.irq_lock_noguard();
                }
                list_del!(&mut poco.next);
                unsafe {
                    if (poco.events_received & POLLNVAL as i32 != 0) && !poco.unwatch.is_none() {
                        poco.unwatch.as_ref().unwrap()(poco.head.as_ref().unwrap().as_ref());
                    }
                    poco.head
                        .unwrap()
                        .as_ref()
                        .watchpoints
                        .irq_unlock_noguard(flags);
                }
            }
        }
        n += 1;
    }
}

fn wait_events(
    filp: &File,
    group: &mut RrosPollGroup,
    wreq: &mut RrosPollWaitreq,
    ts64: Timespec64,
) -> Result<i32> {
    let u_set: *mut CPollEvent;
    let waiter: RrosPollWaiter = RrosPollWaiter::new();
    let tmode: RrosTmode;
    let mut flags: u64;
    let timeout: ktime::KtimeT;
    let mut count: Result<i32>;

    if wreq.nrset < 0 {
        return Err(Error::EINVAL);
    }
    if wreq.nrset == 0 {
        return Ok(0);
    }

    u_set = wreq.pollset_ptr as *mut CPollEvent;
    let waiter_flag_mut: &mut RrosFlag = unsafe { &mut *waiter.flag.as_ref().unwrap().as_ptr() };
    waiter_flag_mut.init();
    'out: loop {
        'unwait: loop {
            count = collect_events(group, u_set, wreq.nrset, &waiter.flag);
            if (count.is_ok() && count.unwrap() > 0)
                || count == Err(Error::EFAULT)
                || count == Err(Error::EBADF)
            {
                break 'unwait;
            }
            if count.is_err() {
                break 'out;
            }

            unsafe {
                if (*filp.ptr).f_flags & bindings::O_NONBLOCK != 0 {
                    count = Err(Error::EAGAIN);
                    break 'unwait;
                }
            }

            timeout = timespec64_to_ktime(ts64);
            if timeout != 0 {
                tmode = RrosTmode::RrosAbs;
            } else {
                tmode = RrosTmode::RrosRel;
            }

            let waiter_list = unsafe { &mut *group.waiter_list.locked_data().get() };
            let arc_waiter = Arc::try_new(waiter).unwrap();
            flags = group.waiter_list.irq_lock_noguard();
            waiter_list.push_front(arc_waiter.clone());
            group.waiter_list.irq_unlock_noguard(flags);
            let num = waiter_flag_mut.wait_timeout(timeout, tmode);
            flags = group.waiter_list.irq_lock_noguard();
            unsafe { waiter_list.remove(&arc_waiter) };
            group.waiter_list.irq_unlock_noguard(flags);

            if num == 0 {
                count = collect_events(group, u_set, wreq.nrset, &mut None);
            } else {
                // error code is `num`
                count = Err(Error::from_kernel_errno(num));
            }
            break;
        }
        clear_wait();
        break;
    }
    waiter_flag_mut.destory();

    return count;
}

fn poll_open(filp: &File) -> Result<Box<RefCell<RrosPollGroup>>> {
    let mut group: Box<RefCell<RrosPollGroup>>;

    if let Ok(g) = Box::try_new(RefCell::new(RrosPollGroup::new())) {
        group = g;
    } else {
        return Err(Error::ENOMEM);
    }
    if rros_open_file(&mut group.get_mut().rfile, filp.ptr).is_err() {
        return Err(Error::EBADF);
    }

    group.get_mut().init();

    Ok(group)
}

fn poll_release(obj: Box<RefCell<RrosPollGroup>>, _filp: &File) {
    let mut group: RefMut<'_, RrosPollGroup> = obj.try_borrow_mut().unwrap();
    let flags: u64;
    let refmut_g = group.deref_mut();
    flags = refmut_g.waiter_list.irq_lock_noguard();
    let mut cursor = unsafe { (*refmut_g.waiter_list.locked_data().get()).cursor_front_mut() };
    while cursor.current().is_some() {
        let waiter = cursor.current().unwrap();
        waiter
            .flag
            .as_deref()
            .unwrap()
            .try_borrow_mut()
            .unwrap()
            .flush_nosched(T_RMID as i32);
        cursor.move_next();
    }
    refmut_g.waiter_list.irq_unlock_noguard(flags);
    unsafe {
        rros_schedule();
    }

    refmut_g.flush_item();
    if let Err(_) = rros_release_file(&mut refmut_g.rfile) {
        pr_err!("poll_release: rros_release_file failed");
    }
}

fn poll_oob_ioctl(
    this: &RefCell<RrosPollGroup>,
    filp: &File,
    cmd: &mut IoctlCommand,
) -> Result<i32> {
    let mut group: RefMut<'_, RrosPollGroup> = this.try_borrow_mut().unwrap();
    let mut wreq: RrosPollWaitreq = RrosPollWaitreq::new();
    let u_wreq: *mut RrosPollWaitreq;
    let mut creq: RrosPollCtlreq = RrosPollCtlreq::new();
    let ret: Result<i32>;
    let mut ts64: Timespec64 = Timespec64(bindings::timespec64::default());
    let u_uts: *mut Timespec64;
    let mut uts = Timespec64(bindings::timespec64 {
        tv_sec: 0,
        tv_nsec: 0,
    });

    match cmd.cmd {
        RROS_POLIOC_CTL => {
            //  creq layout:
            //
            // default in c:
            // 0x7fd32624b0:   0x00000000(action)      0x00000008(fd)      0x00000001(events)      0x00000000(none for alignment)
            // 0x7fd32624c0:   [0x00000000      0x00000000](pollval)
            //
            // default in rust:
            // 0xffffffc011e0ba38:  { [0x00000000      0x00000008(pollval.val)      0x00000001      0x00000000](pollval)
            // 0xffffffc011e0ba48:     0x00000000(action)      0x00000000(fd) }(useful_data)      [ 0x0812ed00(events)      0x0a5de829(alignment) ](dirty data)
            //
            //
            let uptrrd = &mut unsafe {
                UserSlicePtr::new(
                    cmd.arg as *mut u8 as *mut c_types::c_void,
                    size_of::<RrosPollCtlreq>(),
                )
                .reader()
            };
            unsafe {
                if uptrrd
                    .read_raw(
                        &mut creq.action as *mut u32 as *mut u8,
                        size_of::<(i32, i32, i32, i32)>(),
                    )
                    .is_err()
                {
                    return Err(Error::EFAULT);
                }
                if uptrrd
                    .read_raw(
                        &mut creq.pollval as *mut RrosValue as *mut u8,
                        size_of::<(i32, i32)>(),
                    )
                    .is_err()
                {
                    return Err(Error::EFAULT);
                }
            }
            ret = setup_item(filp, &mut group, creq);
        }
        RROS_POLIOC_WAIT => {
            u_wreq = cmd.arg as *mut u64 as *mut RrosPollWaitreq;
            let mut uptrrd = unsafe {
                UserSlicePtr::new(u_wreq as *mut c_types::c_void, size_of::<RrosPollWaitreq>())
                    .reader()
            };
            unsafe {
                if uptrrd
                    .read_raw(
                        &mut wreq as *mut RrosPollWaitreq as *mut u8,
                        size_of::<RrosPollWaitreq>(),
                    )
                    .is_err()
                {
                    return Err(Error::EFAULT);
                }
            }

            u_uts = wreq.timeout_ptr as *mut Timespec64;
            uptrrd = unsafe {
                UserSlicePtr::new(u_uts as *mut c_types::c_void, size_of::<Timespec64>()).reader()
            };
            unsafe {
                if uptrrd
                    .read_raw(
                        &mut uts as *mut Timespec64 as *mut u8,
                        size_of::<Timespec64>(),
                    )
                    .is_err()
                {
                    return Err(Error::EFAULT);
                }
            }
            if uts.0.tv_nsec >= 1000_000_000 {
                return Err(Error::EINVAL);
            }
            ts64.0.tv_sec = uts.0.tv_sec;
            ts64.0.tv_nsec = uts.0.tv_nsec;
            ret = wait_events(filp, &mut group, &mut wreq, ts64);
            if ret.is_err() {
                return ret;
            }
            unsafe {
                (*u_wreq).nrset = ret.unwrap();
                pr_debug!("(*u_wreq).nrset is {:?}", (*u_wreq).nrset);
            }
        }
        _ => {
            return Err(Error::ENOTTY);
        }
    }

    return ret;
}

pub struct PollOps();

impl FileOpener<u8> for PollOps {
    fn open(_shared: &u8, file: &File) -> Result<Self::Wrapper> {
        poll_open(file)
    }
}

impl FileOperations for PollOps {
    kernel::declare_file_operations!(oob_ioctl);

    type Wrapper = Box<RefCell<RrosPollGroup>>;

    fn oob_ioctl(
        this: &RefCell<RrosPollGroup>,
        file: &File,
        cmd: &mut IoctlCommand,
    ) -> Result<i32> {
        let ret = poll_oob_ioctl(this, file, cmd);
        ret
    }

    fn release(_obj: Self::Wrapper, _file: &File) {
        poll_release(_obj, _file);
    }
}

pub static mut RROS_POLL_FACTORY: SpinLock<factory::RrosFactory> = unsafe {
    SpinLock::new(factory::RrosFactory {
        name: CStr::from_bytes_with_nul_unchecked("poll\0".as_bytes()),
        // fops: Some(&Pollops),
        nrdev: 0,
        build: None,
        dispose: None,
        attrs: None,
        flags: factory::RrosFactoryType::SINGLE,
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
