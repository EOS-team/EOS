use core::{clone::Clone, ffi::c_void, mem::size_of, ptr::NonNull};

use kernel::{
    bindings, c_str, init_static_sync,
    linked_list::{GetLinks, Links, List},
    net::Namespace,
    notifier::NotifierBlock,
    prelude::*,
    spinlock_init,
    sync::{Lock, SpinLock},
    vmalloc, Result,
};

use super::{skb::RrosSkbQueue, socket::RrosNetdevActivation};
use crate::{
    crossing::RrosCrossing,
    flags::RrosFlag,
    net::{input::rros_net_do_rx, skb::rros_net_dev_build_pool},
    thread::KthreadRunner,
    wait::RrosWaitQueue,
};

const IFF_OOB_PORT: usize = 1 << 1;
const IFF_OOB_CAPABLE: usize = 1 << 0;
const RROS_DEFAULT_NETDEV_POOLSZ: usize = 128;
const RROS_DEFAULT_NETDEV_BUFSZ: usize = 2048;
#[allow(dead_code)]
const RROS_MAX_NETDEV_POOLSZ: usize = 16384;
#[allow(dead_code)]
const RROS_MAX_NETDEV_BUFSZ: usize = 8192;
const KTHREAD_RX_PRIO: usize = 1;
#[allow(dead_code)]
const KTHREAD_TX_PRIO: usize = 1;

/// orphan type for implementing `Sync` and `Send` for `List`
struct ListThreadSafeWrapper(pub List<Box<NetDevice>>);
unsafe impl Sync for ListThreadSafeWrapper {}
unsafe impl Send for ListThreadSafeWrapper {}

init_static_sync! {
    static ACTIVE_PORT_LIST : SpinLock<ListThreadSafeWrapper> = ListThreadSafeWrapper(List::new());
}

pub fn start_handler_thread(
    func: Box<dyn FnOnce()>,
    name: &'static kernel::str::CStr,
) -> Result<*mut KthreadRunner> {
    // vmalloc::c_kzalloc()
    // let runner = Box::try_new(KthreadRunner::new_empty());

    let runner = vmalloc::c_kzalloc(core::mem::size_of::<KthreadRunner>() as u64);
    // if runner.is_err(){
    //     return Err(bindings::ENOMEM);
    // }
    let runner = runner.unwrap() as *mut KthreadRunner;
    unsafe {
        (*runner).init(func);
        (*runner).run(name);
    }

    // if ret.is_err(){
    //     unsafe{bindings::kfree(kt as *mut c_void)};
    //     return unsafe{(-bindings::ENOMEM) as *const _ as *mut RrosKthread};
    // }
    return Ok(runner);
}

pub struct RrosNetdevState {
    pub free_skb_pool: bindings::list_head,
    pub pool_free: usize,
    pub pool_max: usize,
    pub buf_size: usize,
    pub pool_wait: RrosWaitQueue,
    // poll_head : RROSPollHead, // TODO:
    pub rx_handler: *mut KthreadRunner,
    pub rx_flag: RrosFlag,
    pub rx_queue: RrosSkbQueue,
    // pub qdisc : RROSNetQdisc,
    // pub tx_handler : Rc<RefCell<RrosThread>>, // TODO:
    // tx_flag : RROSFlag // TODO:
    pub refs: i32,
}

pub struct OOBNetdevState {
    pub rstate: RrosNetdevState,
    pub crossing: RrosCrossing,
    next: Links<NetDevice>,
}

impl GetLinks for NetDevice {
    type EntryType = NetDevice;
    fn get_links(data: &Self::EntryType) -> &Links<Self::EntryType> {
        unsafe { &(*(data.0.as_ref().oob_context.dev_state.wrapper as *const OOBNetdevState)).next }
    }
}

/// Wraps the pointer of kernel's `struct net_device` for socket buffer.
/// We use the pointer instead of struct itself.
pub struct NetDevice(pub(crate) NonNull<bindings::net_device>);

impl NetDevice {
    pub fn from_ptr(ptr: *mut bindings::net_device) -> Option<Self> {
        if ptr.is_null() {
            return None;
        }
        Some(Self(NonNull::new(ptr).unwrap()))
    }

    pub fn is_vlan_dev(&self) -> bool {
        extern "C" {
            #[allow(improper_ctypes)]
            fn rust_helper_is_vlan_dev(dev: *const bindings::net_device) -> bool;
        }
        unsafe { rust_helper_is_vlan_dev(self.0.as_ptr()) }
    }

    #[inline]
    pub fn dev_state_mut(&mut self) -> NonNull<OOBNetdevState> {
        let wrapper =
            unsafe { (self.0.as_mut().oob_context.dev_state.wrapper) as *mut OOBNetdevState };
        if wrapper.is_null() {
            let state = vmalloc::c_kzalloc(size_of::<OOBNetdevState>() as u64).unwrap();
            unsafe { self.0.as_mut() }.oob_context.dev_state.wrapper = state;
            return NonNull::new(state as *mut OOBNetdevState).unwrap();
        } else {
            NonNull::new(wrapper).unwrap()
        }
    }

    #[inline]
    pub fn is_oob_port(&self) -> bool {
        unsafe { self.0.as_ref() }.oob_context.flags & IFF_OOB_PORT as i32 != 0
    }
    #[inline]
    fn set_oob_port(&mut self) {
        unsafe {
            self.0.as_mut().oob_context.flags |= IFF_OOB_PORT as i32;
        }
    }

    #[inline]
    fn unset_oob_port(&mut self) {
        unsafe {
            self.0.as_mut().oob_context.flags &= !IFF_OOB_PORT as i32;
        }
    }
    #[inline]
    pub fn is_oob_capable(&self) -> bool {
        unsafe { self.0.as_ref() }.oob_context.flags & IFF_OOB_CAPABLE as i32 != 0
    }

    #[inline]
    pub fn netif_oob_diversion(&self) -> bool {
        unsafe { (self.0.as_ref().state & (1 << bindings::netdev_state_t___LINK_STATE_OOB)) != 0 }
    }

    #[inline]
    fn enable_oob_diversion(&mut self) {
        extern "C" {
            fn rust_helper_set_bit(state: u32, p: *mut u64);
        }
        unsafe {
            rust_helper_set_bit(
                bindings::netdev_state_t___LINK_STATE_OOB as u32,
                &mut self.0.as_mut().state as *mut u64,
            );
        }
    }

    #[inline]
    fn disable_oob_diversion(&mut self) {
        extern "C" {
            fn rust_helper_clear_bit(state: u32, p: *mut u64);
        }
        unsafe {
            rust_helper_clear_bit(
                bindings::netdev_state_t___LINK_STATE_OOB as u32,
                &mut self.0.as_mut().state as *mut u64,
            );
        }
    }

    #[allow(dead_code)]
    pub fn get_dev(&mut self) {
        let mut state = self.dev_state_mut();
        unsafe { state.as_mut() }.crossing.down() // atomic increase
    }

    pub fn put_dev(&mut self) {
        let mut state = self.dev_state_mut();
        unsafe { state.as_mut() }.crossing.up() // atomic decrease
    }

    pub fn vlan_dev_real_dev(&self) -> Self {
        extern "C" {
            #[allow(improper_ctypes)]
            fn rust_helper_vlan_dev_real_dev(
                dev: *const bindings::net_device,
            ) -> *const bindings::net_device;
        }
        let dev = unsafe { rust_helper_vlan_dev_real_dev(self.0.as_ptr() as *const _) };
        Self(NonNull::new(dev as *mut _).unwrap())
    }

    pub fn get_net(&self) -> *const Namespace {
        extern "C" {
            #[allow(improper_ctypes)]
            fn rust_helper_dev_net(dev: *const bindings::net_device) -> *const Namespace;
        }
        unsafe { rust_helper_dev_net(self.0.as_ptr() as *const bindings::net_device) }
    }

    pub fn ifindex(&self) -> i32 {
        unsafe { self.0.as_ref() }.ifindex
    }
    fn enable_oob_port(&mut self, mut act: RrosNetdevActivation) -> i32 {
        if !self.is_vlan_dev() {
            return -(bindings::EINVAL as i32);
        }
        if self.is_oob_port() {
            return 0;
        }

        let mut real_dev = self.vlan_dev_real_dev();
        let mut nds = real_dev.dev_state_mut(); // create state
        let est = &mut unsafe { nds.as_mut() }.rstate;

        let mut vnds = self.dev_state_mut();
        let _ret = unsafe { vnds.as_mut() }.crossing.init();

        if est.refs == 0 {
            if act.poolsz == 0 {
                act.poolsz = RROS_DEFAULT_NETDEV_POOLSZ as u64;
            }

            if act.bufsz == 0 {
                act.bufsz = RROS_DEFAULT_NETDEV_BUFSZ as u64;
            }

            // let mtu = READ_ONCE(real_dev->mtu);// TODO: READ_ONCE
            let mtu = unsafe { real_dev.0.as_ref() }.mtu as u64;
            if act.bufsz < mtu {
                act.bufsz = mtu;
            }

            est.pool_free = 0;
            est.pool_max = act.poolsz as usize;
            est.buf_size = act.bufsz as usize;
            let ret = rros_net_dev_build_pool(&mut real_dev);
            if ret != 0 {
                // panic!();
                return -1;
            }
            // est.qdisc = //TODO:

            let pinned = unsafe { Pin::new_unchecked(&mut est.rx_queue) };
            spinlock_init!(pinned, "RrosSkbQueue");
            unsafe { (*est.rx_queue.locked_data().get()).init() };
            est.rx_flag.init();

            let arg1 = NetDevice(real_dev.0.clone());
            let _arg2 = KTHREAD_RX_PRIO;
            let func = Box::try_new(move || {
                let dev = arg1;
                rros_net_do_rx(dev);
            })
            .unwrap();
            est.rx_handler = start_handler_thread(func, c_str!("rros oob net rx handler")).unwrap();

            // Only those with out-of-band capabilities are required.
            if real_dev.is_oob_capable() {

                // rros_init_flag(&est->tx_flag);
                // kt = start_handler_thread(real_dev, rros_net_do_tx,
                //             KTHREAD_TX_PRIO, "tx");
                // if (IS_ERR(kt))
                //     goto fail_start_tx;

                // est->tx_handler = kt;
            }
            let _ret = unsafe { nds.as_mut() }.crossing.init();
            real_dev.enable_oob_diversion();
        }
        est.refs += 1;
        self.set_oob_port();

        let flags = ACTIVE_PORT_LIST.irq_lock_noguard();
        unsafe {
            (*ACTIVE_PORT_LIST.locked_data().get())
                .0
                .push_back(Box::try_new(NetDevice(self.0.clone())).unwrap());
        }
        ACTIVE_PORT_LIST.irq_unlock_noguard(flags);
        pr_crit!("enable oob port success");

        return 0;
        // TODO: Handle exceptions gracefully.
        //     rros_stop_kthread(est->rx_handler);
        //     rros_destroy_flag(&est->tx_flag);
        // fail_start_rx:
        //     rros_net_dev_purge_pool(real_dev);
        //     rros_destroy_flag(&est->rx_flag);
        // fail_build_pool:
        //     rros_net_free_qdisc(est->qdisc);
        //     kfree(est);
        //     nds->estate = NULL;
    }

    fn disable_oob_port(&mut self) {
        if !self.is_vlan_dev() {
            return;
        }
        if !self.is_oob_port() {
            return;
        }
        let vnds = unsafe { self.dev_state_mut().as_mut() };
        // let flags = ACTIVE_PORT_LIST.irqsave_lock();
        // unsafe{
        //     (*ACTIVE_PORT_LIST.locked_data().get()).remove(index)
        // }
        // unsafe{
        //     ()
        // }
        // unsafe{(*ACTIVE_PORT_LIST.locked_data().get()).remove(vnds)};
        // ACTIVE_PORT_LIST.irq_unlock_noguard(flags);
        vnds.crossing.pass();

        self.unset_oob_port();

        let mut real_dev = self.vlan_dev_real_dev();
        let est = &mut unsafe { real_dev.dev_state_mut().as_mut() }.rstate;
        if est.refs <= 0 {
            return;
        }
        est.refs -= 1;
        if est.refs > 0 {
            return;
        }

        // rros_signal_poll_events(&est->poll_head, POLLERR);
        // rros_flush_wait(&est->pool_wait, T_RMID); //TODO:
        // rros_schedule();

        self.disable_oob_diversion();
        // rros_stop_kthread(est->rx_handler);
        // if (est->tx_handler)
        //     rros_stop_kthread(est->tx_handler); // TODO:

        // rros_net_free_qdisc(est->qdisc);
    }
    fn switch_oob_port(&mut self, act: Option<RrosNetdevActivation>) -> i32 {
        if let Some(act) = act {
            self.enable_oob_port(act)
        } else {
            self.disable_oob_port();
            return 0;
        }
    }

    pub fn net_get_dev_by_index(net: *mut Namespace, ifindex: i32) -> Option<Self> {
        assert!(ifindex != 0);
        let flags = ACTIVE_PORT_LIST.irq_lock_noguard();

        let list = unsafe { &mut (*ACTIVE_PORT_LIST.locked_data().get()).0 };
        let cursor = list.cursor_front();
        while let Some(item) = cursor.current_mut() {
            if core::ptr::eq(item.get_net(), net) && item.ifindex() == ifindex {
                unsafe { item.dev_state_mut().as_mut().crossing.down() };
                let ret = NetDevice(item.0.clone());
                ACTIVE_PORT_LIST.irq_unlock_noguard(flags);
                return Some(ret);
            }
        }

        ACTIVE_PORT_LIST.irq_unlock_noguard(flags);
        return None;
    }
}

#[no_mangle]
pub fn netif_oob_switch_port(dev: *mut bindings::net_device, enabled: bool) -> i32 {
    let mut dev = NetDevice(NonNull::new(dev).unwrap());
    if !dev.is_vlan_dev() {
        return -(bindings::ENXIO as i32);
    }
    if enabled {
        let act = RrosNetdevActivation {
            poolsz: 0,
            bufsz: 0,
        };
        return dev.switch_oob_port(Some(act));
    } else {
        dev.switch_oob_port(None);
        return 0;
    }
}

/// netdevice notifier
#[allow(dead_code)]
fn rros_netdev_event(_ev_block: *mut NotifierBlock, event: u64, ptr: *mut c_void) -> i32 {
    extern "C" {
        #[allow(improper_ctypes)]
        fn rust_helper_netdev_notifier_info_to_dev(ptr: *mut c_void) -> *mut bindings::net_device;
    }

    let _dev = unsafe { rust_helper_netdev_notifier_info_to_dev(ptr) };
    if event == bindings::netdev_cmd_NETDEV_GOING_DOWN as u64 {
        unimplemented!();
    }
    bindings::NOTIFY_DONE as i32
}
