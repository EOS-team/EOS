use super::device::NetDevice;
use crate::{
    net::{ethernet::input::rros_net_ether_accept, skb::RrosSkBuff},
    sched::rros_schedule,
};
use core::ptr::NonNull;
use kernel::{
    bindings,
    c_types::c_void,
    endian::be16,
    prelude::*,
    spinlock_init,
    sync::{Lock, SpinLock},
    types::HlistNode,
    vmalloc,
};

// pub struct RROSNetHandler{
//     ingress : fn(skb : *mut bindings::sk_buff),
// }

pub struct RrosNetRxqueue {
    pub hkey: u32,
    pub hash: HlistNode,
    pub subscribers: bindings::list_head,
    pub lock: SpinLock<()>,
    pub next: bindings::list_head,
}

impl RrosNetRxqueue {
    // TODO: replace this with Rc<Refcell<>>
    pub fn new(hkey: u32) -> Option<NonNull<Self>> {
        extern "C" {
            fn rust_helper_INIT_LIST_HEAD(list: *mut bindings::list_head);
        }
        let ptr = vmalloc::c_kzalloc(core::mem::size_of::<RrosNetRxqueue>() as u64);
        if ptr.is_none() {
            return None;
        }
        let ptr = unsafe { &mut *(ptr.unwrap() as *const _ as *mut RrosNetRxqueue) };
        ptr.hkey = hkey;
        unsafe { rust_helper_INIT_LIST_HEAD(&mut ptr.subscribers) };
        let pinned = unsafe { core::pin::Pin::new_unchecked(&mut ptr.lock) };
        spinlock_init!(pinned, "RrosNetRxqueue");
        NonNull::new(ptr)
    }

    pub fn free(&mut self) {
        // RROS_WARN_ON(NET, !list_empty(&rxq->subscribers));
        vmalloc::c_kzfree(self as *const _ as *mut c_void);
    }
}

#[allow(dead_code)]
pub fn rros_net_do_rx(mut dev: NetDevice) {
    extern "C" {
        #[allow(dead_code)]
        fn rust_helper_list_del(list: *mut bindings::list_head);
    }
    let est = unsafe { dev.dev_state_mut().as_mut() };
    let mut list = bindings::list_head {
        next: core::ptr::null_mut(),
        prev: core::ptr::null_mut(),
    };
    init_list_head!(&mut list);
    // while !rros_kthread_should_stop(){
    loop {
        // TODO:
        let ret = est.rstate.rx_flag.wait();
        pr_debug!("rros_net_do_rx");
        if ret != 0 {
            break;
        }
        let flags = est.rstate.rx_queue.irq_lock_noguard();
        if !unsafe { (*est.rstate.rx_queue.locked_data().get()).move_queue(&mut list) } {
            est.rstate.rx_queue.irq_unlock_noguard(flags);
            continue;
        }
        est.rstate.rx_queue.irq_unlock_noguard(flags);
        list_for_each_entry_safe!(
            skb,
            next,
            &mut list,
            bindings::sk_buff,
            {
                let mut rskb = RrosSkBuff::from_raw_ptr(skb);
                list_del!(rskb.list_mut());
                (rskb.net_cb_mut().handler)(rskb);
            },
            __bindgen_anon_1.list
        );
        unsafe { rros_schedule() };
        pr_debug!("end of rros_net_do_rx");
    }
    pr_debug!("rros_net_do_rx exit\n");
}

// NETIF

#[no_mangle]
pub fn netif_oob_run(dev: *mut bindings::net_device) {
    let mut dev = unsafe { NetDevice(NonNull::new_unchecked(dev)) };
    unsafe { &mut dev.dev_state_mut().as_mut() }
        .rstate
        .rx_flag
        .raise();
    pr_debug!("netif_oob_run");
}

#[no_mangle]
fn netif_oob_deliver(skb: *mut bindings::sk_buff) -> bool {
    pr_debug!("netif_oob_deliver");
    extern "C" {
        fn rust_helper_eth_type_vlan(eth_type: be16) -> bool;
    }
    let skb = RrosSkBuff::from_raw_ptr(skb);
    let protocol: u32 = u16::from(be16::new(skb.protocol)).into();
    pr_debug!("protocol is {}", protocol);
    match protocol {
        bindings::ETH_P_IP => {
            pr_debug!("accept");
            rros_net_ether_accept(skb)
            // rros_net_ether_accept(skb) // TODO:
        }
        _ => {
            /*
             * For those adapters without hw-accelerated VLAN
             * capabilities, check the ethertype directly.
             */
            if unsafe { rust_helper_eth_type_vlan(be16::new(skb.protocol)) } {
                pr_debug!("true");
                rros_net_ether_accept(skb)
            } else {
                pr_debug!("false");
                false
            }
        }
    }
}

pub fn rros_net_receive(mut skb: RrosSkBuff, handler: fn(skb: RrosSkBuff)) {
    extern "C" {
        #[allow(improper_ctypes)]
        fn rust_helper_skb_list_del_init(skb: *mut bindings::sk_buff) -> bool;
        fn rust_helper_running_inband() -> bool;
    }
    if !unsafe { skb.__bindgen_anon_1.__bindgen_anon_1.next.is_null() } {
        unsafe { rust_helper_skb_list_del_init(skb.0.as_ptr()) };
    }
    unsafe { skb.rros_control_cb().as_mut().handler = handler };

    let rst = unsafe { skb.dev().unwrap().dev_state_mut().as_mut() };
    let flags = rst.rstate.rx_queue.irq_lock_noguard();
    unsafe { (*rst.rstate.rx_queue.locked_data().get()).add(&mut skb) };
    rst.rstate.rx_queue.irq_unlock_noguard(flags);

    if unsafe { !rust_helper_running_inband() } {
        rst.rstate.rx_flag.raise();
    }
}
