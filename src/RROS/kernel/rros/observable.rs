use alloc::{rc::Rc, sync::Arc};

use crate::{
    clock::{rros_ktime_monotonic, RrosClock, RROS_MONO_CLOCK},
    factory::{
        self, rros_element_is_observable, CloneData, RrosElement, RrosFactory, RrosFactoryInside,
        RROS_CLONE_MASTER, RROS_CLONE_OBSERVABLE, RROS_OBSERVABLE_CLONE_FLAGS,
    },
    file::RrosFileBinding,
    sched::{rros_schedule, RrosPollHead},
    thread::{rros_init_user_element, CONFIG_RROS_NR_THREADS},
    wait::{RrosWaitQueue, RROS_WAIT_PRIO},
};
use core::{cell::RefCell, default::Default, mem::size_of};
use kernel::{
    c_types, device,
    dovetail::{self, DovetailSubscriber},
    file::File,
    file_operations::{FileOpener, FileOperations, IoctlCommand},
    io_buffer::{IoBufferReader, IoBufferWriter, ReadableFromBytes, WritableToBytes},
    irq_work::IrqWork,
    ktime::{self, KtimeT},
    linked_list::{GetLinks, Links, List},
    prelude::*,
    premmpt::running_inband,
    rbtree::RBTree,
    spinlock_init,
    str::CStr,
    sync::{HardSpinlock, Lock, SpinLock},
    task,
    uidgid::{KgidT, KuidT},
    user_ptr::UserSlicePtr,
    waitqueue,
};

pub const CONFIG_RROS_NR_OBSERVABLE: usize = 16;
pub const RROS_OBSERVABLE_IOCBASE: char = 'o';
pub const RROS_OBSIOC_SUBSCRIBE: u32 =
    kernel::ioctl::_IOW::<RrosSubscription>(RROS_OBSERVABLE_IOCBASE as u32, 0);
// pub const RROS_OBSIOC_UNSUBSCRIBE: u32 = kernel::ioctl::_IO(RROS_OBSERVABLE_IOCBASE as u32, 1);
pub const RROS_NOTIFY_ONCHANGE: i32 = 1 << 0;
pub const RROS_NOTIFY_MASK: i32 = RROS_NOTIFY_ONCHANGE;
// pub const RROS_NOTIFY_ALWAYS: i32 = 0 << 0;
pub const RROS_NOTIFY_INITIAL: i32 = 1 << 31;

// Notice tags below this value are reserved to the core.
pub const RROS_NOTICE_USER: u32 = 64;

#[repr(C)]
#[derive(Copy, Clone)]
pub union ObservableValue {
    val: i32,
    lval: i64,
    ptr: *mut c_types::c_void,
}

impl ObservableValue {
    fn new() -> Self {
        Self { lval: 0 }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct RrosNotice {
    pub tag: u32,
    pub event: ObservableValue,
}

impl RrosNotice {
    fn new() -> Self {
        Self {
            tag: 0,
            event: ObservableValue::new(),
        }
    }
}

unsafe impl ReadableFromBytes for RrosNotice {}

pub struct RrosSubscriber {
    pub subscriptions: SpinLock<RBTree<u32, Arc<RrosObserver>>>,
}

impl RrosSubscriber {
    pub fn new_and_init() -> Self {
        let mut s = Self {
            subscriptions: unsafe { SpinLock::new(RBTree::new()) },
        };
        spinlock_init!(
            unsafe { Pin::new_unchecked(&mut s.subscriptions) },
            "RrosSubscriber"
        );
        s
    }
}

// impl `DovetailSubscriber` trait for `RrosSubscriber` so that it can be used in dovetail_current_state().subscriber() and dovetail_current_state().set_subscriber()
impl DovetailSubscriber for RrosSubscriber {
    type Node = RBTree<u32, Arc<RrosObserver>>;
    fn get(&self) -> &mut Self::Node {
        unsafe { &mut *(self.subscriptions.locked_data().get()) }
    }
}

#[repr(C)]
pub struct RrosSubscription {
    pub backlog_count: u32,
    pub flags: u32,
}

impl RrosSubscription {
    fn new() -> Self {
        Self {
            backlog_count: 0,
            flags: 0,
        }
    }
}

pub struct RrosObservable {
    pub element: Rc<RefCell<RrosElement>>,
    pub observers: List<Arc<RrosObserver>>,
    pub flush_list: List<Arc<RrosObserver>>,
    pub oob_wait: RrosWaitQueue,
    pub inband_wait: waitqueue::WaitQueueHead,
    pub poll_head: RrosPollHead,
    pub wake_irqwork: IrqWork,
    pub flush_irqwork: IrqWork,
    pub lock: HardSpinlock,
    pub serial_counter: u32,
    pub writable_observers: i32,
}
impl RrosObservable {
    #[allow(dead_code)]
    pub fn new() -> Self {
        RrosObservable {
            element: Rc::try_new(RefCell::new(RrosElement::new().unwrap())).unwrap(), // TODO: no processing for initializing RrosElement
            observers: List::new(),
            flush_list: List::new(),
            oob_wait: unsafe {
                RrosWaitQueue::new(
                    &mut RROS_MONO_CLOCK as *mut RrosClock,
                    RROS_WAIT_PRIO as i32,
                )
            },
            inband_wait: waitqueue::WaitQueueHead::new(),
            poll_head: RrosPollHead::new(),
            wake_irqwork: IrqWork::new(),
            flush_irqwork: IrqWork::new(),
            lock: HardSpinlock::new(),
            serial_counter: 0,
            writable_observers: 0,
        }
    }
}

#[repr(C)]
pub struct __RrosNotification {
    pub tag: u32,
    pub serial: u32,
    pub issuer: i32,
    pub event: ObservableValue,
    pub date: ktime::Timespec64,
}

unsafe impl WritableToBytes for __RrosNotification {}

pub struct RrosNotificationRecord {
    pub tag: u32,
    pub serial: u32,
    pub issuer: i32,
    pub event: ObservableValue,
    pub date: i64,
    pub next: Links<RrosNotificationRecord>,
}

impl RrosNotificationRecord {
    pub fn new() -> Self {
        Self {
            tag: 0,
            serial: 0,
            issuer: 0,
            event: ObservableValue::new(),
            date: 0,
            next: Links::default(),
        }
    }
}

impl GetLinks for RrosNotificationRecord {
    type EntryType = RrosNotificationRecord;
    fn get_links(data: &Self::EntryType) -> &Links<Self::EntryType> {
        &data.next
    }
}

pub struct RrosObserver {
    pub backlog_size: usize,
    pub free_list: List<Box<RrosNotificationRecord>>,
    pub pending_list: List<Box<RrosNotificationRecord>>,
    pub next: Links<RrosObserver>,
    pub fundle: u32,
    pub flags: i32,
    pub refs: i32,
    pub last_notice: RrosNotice,
    // TODO: how to implement a zero-sized array, and is it essential?
    // If considering the purpose of caching, I think either don't use backlog, or make backlog also become List<Box<RrosNotificationRecord>>
    // backlog: List<Box<RrosNotificationRecord>>,
}

impl RrosObserver {
    pub fn new() -> Self {
        Self {
            backlog_size: 0,
            free_list: List::default(),
            pending_list: List::default(),
            next: Links::default(),
            fundle: 0,
            flags: 0,
            refs: 0,
            last_notice: RrosNotice::new(),
            // backlog: List::new(),
        }
    }
}

impl GetLinks for RrosObserver {
    type EntryType = RrosObserver;
    fn get_links(data: &Self::EntryType) -> &Links<Self::EntryType> {
        &data.next
    }
}

fn add_subscription(
    observable: &mut RrosObservable,
    backlog_count: u32,
    op_flags: i32,
) -> Result<i32> {
    let mut sbr = dovetail::dovetail_current_state().subscriber() as *mut RrosSubscriber;
    if backlog_count == 0 {
        return Err(kernel::Error::EINVAL);
    }
    if op_flags & (!RROS_NOTIFY_MASK) != 0 {
        return Err(kernel::Error::EINVAL);
    }
    if sbr == 0 as *mut RrosSubscriber {
        match Box::try_new(RrosSubscriber::new_and_init()) {
            Ok(s) => sbr = Box::into_raw(s),
            Err(_) => return Err(kernel::Error::ENOMEM),
        }

        dovetail::dovetail_current_state().set_subscriber(sbr);
    }

    // rros_get_element();

    let backlog_count = backlog_count as usize;
    let backlog_size = backlog_count * size_of::<RrosNotificationRecord>();

    let mut observer = RrosObserver::new();
    let fundle = observable.element.borrow().fundle;
    observer.fundle = fundle;
    observer.backlog_size = backlog_size;
    observer.flags = op_flags;
    observer.refs = 1;
    if op_flags & RROS_NOTIFY_ONCHANGE != 0 {
        observer.flags |= RROS_NOTIFY_INITIAL;
    }
    // Build the free list of notification records
    // I don't use the `backlog` field, but alloc RrosNotificationRecord in heap and push it in free_list straightly
    for _ in 0..backlog_count {
        let nf = match Box::try_new(RrosNotificationRecord::new()) {
            Ok(n) => n,
            Err(_) => {
                return Err(kernel::Error::ENOMEM);
            }
        };
        observer.free_list.push_back(nf);
    }

    let observer = match Arc::try_new(observer) {
        Ok(o) => o,
        Err(_) => {
            // TODO:
            // evl_put_element(&observable->element);
            return Err(kernel::Error::ENOMEM);
        }
    };

    let subscriber = unsafe { &mut (*sbr) };
    let flags = subscriber.subscriptions.irq_lock_noguard();
    let ret = subscriber.get().try_insert(fundle, observer.clone());
    subscriber.subscriptions.irq_unlock_noguard(flags);

    match ret {
        Ok(None) => {
            // everything is right
        }
        Ok(Some(_)) => {
            // allocate a Node, but RBTree has a node with the same key
            // node will be free automatically
            return Err(kernel::Error::EEXIST);
        }
        Err(info) => {
            // can't allocate a Node
            // observer will be free automatically
            return Err(info);
        }
    }

    /*
    Make the new observer visible to the observable. We need
    the observer and writability tracking to be atomically
    updated, so nested locking is required. Observers are
    linked to the observable's list by order of subscription
    (FIFO) in case users make such assumption for master mode
    (which undergoes round-robin).
     */
    let flags = observable.lock.raw_spin_lock_irqsave();
    observable.observers.push_back(observer);
    // TODO: We can try using `Arc` to replace the `writable_observers` implementation
    observable.oob_wait.lock.raw_spin_lock();
    observable.writable_observers += 1;
    observable.oob_wait.lock.raw_spin_unlock();
    observable.lock.raw_spin_unlock_irqrestore(flags);
    // TODO:
    // evl_put_element(&observable->element);

    Ok(0)
}

fn decrease_writability(writable_observers: &mut i32) {
    *writable_observers -= 1;
    if *writable_observers < 0 {
        pr_err!("observable: negative quantity of writable_observers\n");
    }
}

fn get_observer(observer: &mut RrosObserver) {
    observer.refs += 1;
}

fn put_observer(observer: &mut RrosObserver) -> bool {
    observer.refs -= 1;
    let new_refs = observer.refs;
    // pr_warn!
    new_refs == 0
}

fn wake_oob_threads(observable: &mut RrosObservable) {
    let flags = observable.oob_wait.lock.raw_spin_lock_irqsave();

    observable.oob_wait.lock.raw_spin_unlock_irqrestore(flags);

    unsafe {
        rros_schedule();
    }
}

fn notify_one_observer(
    oob_wait: &mut RrosWaitQueue,
    writable_observers: &mut i32,
    observer: &mut RrosObserver,
    tmpl_nfr: &mut RrosNotificationRecord,
) -> bool {
    let last_notice;
    let mut nfr;

    let flags = oob_wait.lock.raw_spin_lock_irqsave();
    if observer.flags & RROS_NOTIFY_ONCHANGE != 0 {
        last_notice = observer.last_notice.clone();
        observer.last_notice.tag = tmpl_nfr.tag;
        observer.last_notice.event = tmpl_nfr.event.clone();

        if observer.flags & RROS_NOTIFY_INITIAL != 0 {
            observer.flags &= !RROS_NOTIFY_INITIAL;
        } else if last_notice.tag == tmpl_nfr.tag
            && unsafe { last_notice.event.lval == tmpl_nfr.event.lval }
        {
            oob_wait.lock.raw_spin_unlock_irqrestore(flags);
            return true;
        }
    }

    if observer.free_list.is_empty() {
        oob_wait.lock.raw_spin_unlock_irqrestore(flags);
        return false;
    }

    nfr = observer.free_list.pop_front().unwrap();
    nfr.tag = tmpl_nfr.tag;
    nfr.serial = tmpl_nfr.serial;
    nfr.issuer = tmpl_nfr.issuer;
    nfr.event = tmpl_nfr.event.clone();
    nfr.date = tmpl_nfr.date;

    observer.pending_list.push_back(nfr);

    if observer.free_list.is_empty() {
        decrease_writability(writable_observers);
    }

    oob_wait.lock.raw_spin_unlock_irqrestore(flags);
    true
}

fn notify_single(
    _observable: &mut RrosObservable,
    _nfr: &mut RrosNotificationRecord,
    _len_r: &mut usize,
) -> bool {
    // TODO:
    unimplemented!();
}

fn notify_all(
    observable: &mut RrosObservable,
    nfr: &mut RrosNotificationRecord,
    len_r: &mut usize,
) -> bool {
    let do_flush: bool;
    let mut cursor;
    let mut observer;
    let mut arc_observer;
    let mut flags = observable.lock.raw_spin_lock_irqsave();

    if observable.observers.is_empty() {
        do_flush = false;
        observable.lock.raw_spin_unlock_irqrestore(flags);
        return do_flush == false;
    }

    nfr.serial = observable.serial_counter + 1;
    cursor = observable.observers.cursor_front_mut();
    observer = cursor.current().unwrap();
    get_observer(observer);

    // FIXME: list_for_each_safe!
    while cursor.current().is_some() {
        if cursor.peek_next().is_some() {
            observer = cursor.peek_next().unwrap();
            get_observer(observer);
        }

        observer = cursor.current().unwrap();

        observable.lock.raw_spin_unlock_irqrestore(flags);

        if notify_one_observer(
            &mut observable.oob_wait,
            &mut observable.writable_observers,
            observer,
            nfr,
        ) {
            *len_r += size_of::<RrosNotice>();
        }

        flags = observable.lock.raw_spin_lock_irqsave();

        if put_observer(observer) {
            arc_observer = cursor.remove_current().unwrap();

            observable.oob_wait.lock.raw_spin_lock();
            if Arc::get_mut(&mut arc_observer)
                .unwrap()
                .free_list
                .is_empty()
            {
                decrease_writability(&mut observable.writable_observers)
            }
            observable.oob_wait.lock.raw_spin_unlock();

            observable.flush_list.push_front(arc_observer);
        }

        cursor.move_next();
    }

    do_flush = observable.flush_list.is_empty();
    observable.lock.raw_spin_unlock_irqrestore(flags);
    do_flush
}

fn is_pool_master(observable: &RrosObservable) -> bool {
    let element: &RrosElement = &observable.element.borrow();
    element.clone_flags & RROS_CLONE_MASTER != 0
}

fn push_notification(observable: &mut RrosObservable, ntc: &RrosNotice, date: KtimeT) -> bool {
    let mut nfr = RrosNotificationRecord {
        tag: ntc.tag,
        serial: 0,
        issuer: {
            if ntc.tag >= RROS_NOTICE_USER {
                unsafe { (*task::Task::current_ptr()).pid }
            } else {
                0
            }
        },
        event: ntc.event.clone(),
        date: date as i64,
        next: Links::new(),
    };

    let mut _do_flush;
    let mut len = 0_usize;

    if is_pool_master(observable) {
        // TODO:
        _do_flush = notify_single(observable, &mut nfr, &mut len);
    } else {
        _do_flush = notify_all(observable, &mut nfr, &mut len);
    }

    // TODO:
    // if do_flush {
    //     if running_inband().is_ok() {
    //         do_flush_work(observable);
    //     } else {
    //         // evl_get_element(&observable->element);
    //         // if (!irq_work_queue(&observable->flush_irqwork)) {
    //         //     evl_put_element(&observable->element);
    //         // }
    //     }
    // }

    len > 0
}

fn wake_up_observers(observable: &mut RrosObservable) {
    wake_oob_threads(observable);
}

fn rros_write_observable<T: IoBufferReader>(
    observable: &mut RrosObservable,
    data: &mut T,
    count: usize,
) -> Result<usize> {
    if !rros_element_is_observable(observable.element.clone()) {
        return Err(kernel::Error::EBADF);
    }
    if count == 0 {
        return Ok(0);
    }
    if count % size_of::<RrosNotice>() != 0 {
        return Err(kernel::Error::EINVAL);
    }

    let mut len = 0_usize;
    let mut xlen = 0_usize;
    let mut ntc;
    let mut ret = Ok(0_usize);
    let now: KtimeT = rros_ktime_monotonic();

    while xlen < count {
        match data.read::<RrosNotice>() {
            Ok(read) => ntc = read,
            Err(_) => {
                ret = Err(kernel::Error::EFAULT);
                break;
            }
        }

        if ntc.tag < RROS_NOTICE_USER {
            ret = Err(kernel::Error::EINVAL);
            break;
        }

        if push_notification(observable, &ntc, now) {
            len += size_of::<RrosNotice>();
        }

        xlen += size_of::<RrosNotice>();
    }

    if len > 0 {
        wake_up_observers(observable);
    }

    if ret == Ok(0_usize) {
        return Ok(len);
    }

    ret
}

fn pull_from_inband(
    _observable: &mut RrosObservable,
    _observer: &mut Arc<RrosObserver>,
    _wait: bool,
) -> Result<Box<RrosNotificationRecord>> {
    // TODO:
    unimplemented!();
}

fn pull_from_oob(
    observable: &mut RrosObservable,
    observer: &mut Arc<RrosObserver>,
    wait: bool,
) -> Result<Box<RrosNotificationRecord>> {
    let nfr: Box<RrosNotificationRecord>;
    let flags: u64;
    // let mut ret: Result<usize>;

    flags = observable.oob_wait.lock.raw_spin_lock_irqsave();

    loop {
        // TODO: how to judge a observer is unsubscribed?
        // if (list_empty(&observer->next)) {
        // 	/* Unsubscribed by observable_release(). */
        // 	nfr = ERR_PTR(-EBADF);
        // 	goto out;
        // }

        if !observer.as_ref().pending_list.is_empty() {
            break;
        }

        if !wait {
            observable.oob_wait.lock.raw_spin_unlock_irqrestore(flags);
            return Err(kernel::Error::EAGAIN);
        }
    }

    pr_debug!("observer.strong_count: {}", Arc::strong_count(observer));
    nfr = unsafe {
        Arc::get_mut_unchecked(observer)
            .pending_list
            .pop_front()
            .unwrap()
    };
    pr_debug!("observer.strong_count: {}", Arc::strong_count(observer));

    observable.oob_wait.lock.raw_spin_unlock_irqrestore(flags);
    Ok(nfr)
}

fn pull_notification<T: IoBufferWriter>(
    observable: &mut RrosObservable,
    observer: &mut Arc<RrosObserver>,
    data: &mut T,
    wait: bool,
) -> Result<usize> {
    let nfr_result: Result<Box<RrosNotificationRecord>>;
    let nfr: Box<RrosNotificationRecord>;
    let nf: __RrosNotification;
    // let mut sigpoll: bool = false;
    let flags: u64;

    if running_inband().is_ok() {
        // TODO:
        nfr_result = pull_from_inband(observable, observer, wait);
    } else {
        nfr_result = pull_from_oob(observable, observer, wait);
    }

    if let Err(error) = nfr_result {
        return Err(error);
    }

    nfr = nfr_result.unwrap();

    nf = __RrosNotification {
        tag: nfr.tag,
        serial: nfr.serial,
        issuer: nfr.issuer,
        event: nfr.event.clone(),
        date: ktime::ktime_to_timespec64(nfr.date),
    };

    let ret = data.write::<__RrosNotification>(&nf);

    flags = observable.oob_wait.lock.raw_spin_lock_irqsave();

    let ret = match ret {
        Ok(_) => {
            if observer.as_ref().pending_list.is_empty() {
                observable.writable_observers += 1;
            }

            pr_debug!("observer.strong_count: {}", Arc::strong_count(observer));
            unsafe {
                Arc::get_mut_unchecked(observer).free_list.push_back(nfr);
            }
            pr_debug!("observer.strong_count: {}", Arc::strong_count(observer));

            Ok(0_usize)
        }
        Err(_) => {
            unsafe {
                Arc::get_mut_unchecked(observer)
                    .pending_list
                    .push_front(nfr);
            }
            Err(kernel::Error::EFAULT)
        }
    };

    observable.oob_wait.lock.raw_spin_unlock_irqrestore(flags);

    // TODO:
    // if (unlikely(sigpoll))
    // 	evl_signal_poll_events(&observable->poll_head,
    // 			POLLOUT|POLLWRNORM);

    ret
}

pub fn rros_read_observable<T: IoBufferWriter>(
    observable: &mut RrosObservable,
    data: &mut T,
    count: usize,
    wait: bool,
) -> Result<usize> {
    let sbr = dovetail::dovetail_current_state().subscriber() as *mut RrosSubscriber;
    if sbr == 0 as *mut _ {
        return Err(kernel::Error::ENXIO);
    }
    if !rros_element_is_observable(observable.element.clone()) {
        return Err(kernel::Error::ENXIO);
    }

    let target = observable.element.clone().borrow().fundle;

    let observer = unsafe { (*sbr).get().get_mut(&target) };

    if observer.is_none() {
        return Err(kernel::Error::ENXIO);
    }
    if count == 0 {
        return Ok(0);
    }
    if count % size_of::<__RrosNotification>() != 0 {
        return Err(kernel::Error::EINVAL);
    }

    let mut len = 0_usize;
    let mut ret = Ok(0_usize);
    let mut wait = wait;
    let observer = observer.unwrap();

    while len < count {
        ret = pull_notification(observable, observer, data, wait);
        if let Err(_) = ret {
            break;
        }
        wait = false;
        len += size_of::<__RrosNotification>();
    }

    if len == 0_usize {
        return ret;
    }
    Ok(len)
}

pub fn rros_ioctl_observable(observable: &mut RrosObservable, cmd: u32, arg: usize) -> Result<i32> {
    let mut sub = RrosSubscription::new();
    let ret: Result<i32>;

    match cmd {
        RROS_OBSIOC_SUBSCRIBE => {
            let uptrrd = &mut unsafe {
                UserSlicePtr::new(
                    arg as *mut u8 as *mut c_types::c_void,
                    size_of::<RrosSubscription>(),
                )
                .reader()
            };
            unsafe {
                if uptrrd
                    .read_raw(&mut sub as *mut _ as *mut u8, size_of::<RrosSubscription>())
                    .is_err()
                {
                    return Err(kernel::Error::EFAULT);
                }
            };
            ret = add_subscription(observable, sub.backlog_count, sub.flags as i32);
        }
        // RROS_OBSIOC_UNSUBSCRIBE => {

        // },
        _ => {
            ret = Err(kernel::Error::ENOTTY);
        }
    }

    unsafe { rros_schedule() };
    ret
}

#[derive(Default)]
pub struct ObservableOps;

impl FileOpener<u8> for ObservableOps {
    fn open(shared: &u8, _fileref: &File) -> Result<Self::Wrapper> {
        let mut data = CloneData::default();
        unsafe {
            data.ptr = shared as *const u8 as *mut u8;
            let a = KuidT::from_inode_ptr(shared as *const u8);
            let b = KgidT::from_inode_ptr(shared as *const u8);
            // let a = KuidT((*(shared as *const u8 as *const bindings::inode)).i_uid);
            // let b = KgidT((*(shared as *const u8 as *const bindings::inode)).i_gid);
            (*RROS_OBSERVABLE_FACTORY.locked_data().get())
                .inside
                .as_mut()
                .unwrap()
                .kuid = Some(a);
            (*RROS_OBSERVABLE_FACTORY.locked_data().get())
                .inside
                .as_mut()
                .unwrap()
                .kgid = Some(b);
        }
        // bindings::stream_open();
        Ok(Box::try_new(data)?)
    }
}

impl FileOperations for ObservableOps {
    // impl const TO_USE
    kernel::declare_file_operations!(ioctl, read, oob_read, write, oob_write);

    type Wrapper = Box<CloneData>;

    fn ioctl(_this: &CloneData, file: &File, cmd: &mut IoctlCommand) -> Result<i32> {
        // log for debug
        pr_debug!("I'm the ioctl ops of the observable factory.");
        let fbind: *const RrosFileBinding =
            unsafe { (*(file.get_ptr())).private_data as *const RrosFileBinding };
        let ptr = unsafe { (*(*fbind).element).pointer as *mut RrosObservable };
        let mut observable = unsafe { Box::from_raw(ptr) };
        let ret = rros_ioctl_observable(&mut observable, cmd.cmd, cmd.arg);
        Box::into_raw(observable);
        ret
    }

    fn read<T: IoBufferWriter>(
        _this: &CloneData,
        file: &File,
        data: &mut T,
        _offset: u64,
    ) -> Result<usize> {
        pr_debug!("I'm the read ops of the observable factory.");
        let fbind: *const RrosFileBinding =
            unsafe { (*(file.get_ptr())).private_data as *const RrosFileBinding };
        let ptr = unsafe { (*(*fbind).element).pointer as *mut RrosObservable };
        let mut observable = unsafe { Box::from_raw(ptr) };
        let ret = rros_read_observable(&mut observable, data, data.len(), file.is_blocking());
        Box::into_raw(observable);
        ret
    }

    fn oob_read<T: IoBufferWriter>(_this: &CloneData, file: &File, data: &mut T) -> Result<usize> {
        pr_debug!("I'm the oob_read ops of the observable factory.");
        let fbind: *const RrosFileBinding =
            unsafe { (*(file.get_ptr())).private_data as *const RrosFileBinding };
        let ptr = unsafe { (*(*fbind).element).pointer as *mut RrosObservable };
        let mut observable = unsafe { Box::from_raw(ptr) };
        let ret = rros_read_observable(&mut observable, data, data.len(), file.is_blocking());
        Box::into_raw(observable);
        ret
    }

    fn write<T: kernel::io_buffer::IoBufferReader>(
        _this: &CloneData,
        file: &File,
        data: &mut T,
        _offset: u64,
    ) -> Result<usize> {
        pr_debug!("I'm the write ops of the observable factory.");
        let fbind: *const RrosFileBinding =
            unsafe { (*(file.get_ptr())).private_data as *const RrosFileBinding };
        let ptr = unsafe { (*(*fbind).element).pointer as *mut RrosObservable };
        let mut observable = unsafe { Box::from_raw(ptr) };
        let ret = rros_write_observable(&mut observable, data, data.len());
        Box::into_raw(observable);
        ret
    }

    fn oob_write<T: kernel::io_buffer::IoBufferReader>(
        _this: &CloneData,
        file: &File,
        data: &mut T,
    ) -> Result<usize> {
        pr_debug!("I'm the oob_write ops of the observable factory.");
        let fbind: *const RrosFileBinding =
            unsafe { (*(file.get_ptr())).private_data as *const RrosFileBinding };
        let ptr = unsafe { (*(*fbind).element).pointer as *mut RrosObservable };
        let mut observable = unsafe { Box::from_raw(ptr) };
        let ret = rros_write_observable(&mut observable, data, data.len());
        Box::into_raw(observable);
        ret
    }

    fn release(_this: Box<CloneData>, _file: &File) {
        pr_debug!("I'm the release ops of the observable factory.");
    }
}

pub fn observable_factory_build(
    fac: &'static mut SpinLock<RrosFactory>,
    uname: &'static CStr,
    _u_attrs: Option<*mut u8>,
    clone_flags: i32,
    _state_offp: &u32,
) -> Rc<RefCell<RrosElement>> {
    pr_debug!("[observable] observable_factory_build: start");
    // in case failed
    if (clone_flags & !RROS_OBSERVABLE_CLONE_FLAGS) != 0 {
        pr_warn!("[observable] this is a wrong value");
        // return Err(Error::EINVAL)
    }

    let mut observable = RrosObservable::new();
    let ret = rros_init_user_element(
        observable.element.clone(),
        fac,
        uname,
        clone_flags | RROS_CLONE_OBSERVABLE,
    );
    if let Err(_) = ret {
        pr_warn!("[observable] observable_factory_build: init user element failed");
    }
    unsafe {
        observable.oob_wait.init(
            &mut RROS_MONO_CLOCK as *mut RrosClock,
            RROS_WAIT_PRIO as i32,
        );
    }
    let mut key = waitqueue::LockClassKey::default();
    observable
        .inband_wait
        .init_waitqueue_head(uname.as_ptr() as *const i8, &mut key);
    // TODO: init_irq_work(&observable->wake_irqwork, inband_wake_irqwork);
    // TODO: init_irq_work(&observable->flush_irqwork, inband_flush_irqwork);
    // TODO: evl_init_poll_head(&observable->poll_head);
    // TODO: evl_index_factory_element(&observable->element);
    observable.lock.init();
    let box_observable = Box::try_new(observable).unwrap();
    let observable_ptr = Box::into_raw(box_observable);

    unsafe {
        (*observable_ptr).element.borrow_mut().pointer = observable_ptr as *mut u8;
    }

    pr_debug!("[observable] observable_factory_build: success");
    unsafe { (*observable_ptr).element.clone() }
}

pub fn observable_factory_dispose(_ele: RrosElement) {
    pr_debug!("[observable] observable_factory_dispose");
}

pub static mut RROS_OBSERVABLE_FACTORY: SpinLock<factory::RrosFactory> = unsafe {
    SpinLock::new(RrosFactory {
        name: CStr::from_bytes_with_nul_unchecked("observable\0".as_bytes()),
        nrdev: CONFIG_RROS_NR_OBSERVABLE + CONFIG_RROS_NR_THREADS,
        build: Some(observable_factory_build),
        dispose: Some(observable_factory_dispose),
        attrs: None,
        flags: factory::RrosFactoryType::CLONE,
        inside: Some(RrosFactoryInside {
            type_: device::DeviceType::new(),
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
