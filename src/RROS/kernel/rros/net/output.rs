use super::skb::RrosSkBuff;
use crate::{
    list_entry_is_head, list_next_entry,
    net::{skb::RrosSkbQueueInner, socket::uncharge_socke_wmem},
};
use core::ffi::c_void;
use kernel::{
    bindings, init_static_sync, interrupt,
    irq_work::IrqWork,
    netdevice,
    sync::{Lock, SpinLock},
    Error, Result,
};

// NOTE:initialize in rros_net_init_tx
// TODO: The implementation here does not use DEFINE_PER_CPU because Rust does not yet support statically defined percpu variables.
init_static_sync! {
    static OOB_TX_RELAY : SpinLock<RrosSkbQueueInner> = RrosSkbQueueInner::default();
}
static mut OOB_XMIT_WORK: IrqWork = unsafe {
    core::mem::transmute::<[u8; core::mem::size_of::<IrqWork>()], IrqWork>(
        [0; core::mem::size_of::<IrqWork>()],
    )
};

// fn oob_start_xmit(dev: *mut bindings::net_device, skb: *mut bindings::sk_buff) -> i32 {
//     unsafe{
//         (*(*dev).netdev_ops).ndo_start_xmit.unwrap()(skb, dev)
//     }
// }

// fn do_tx(qdisc: *mut RROSNetQdisc,dev:*mut bindings::net_device,skb:*mut bindings::sk_buff) {
//     rros_unchange_socket_wmem(skb);  //TODO:
//     let result = oob_start_xmit(dev, skb);
//     match result{
//         bindings::netdev_tx_NETDEV_TX_OK => {},
//         _ => {// busy, or whatever
//             unsafe{
//                 (*qdisc).pack_dropped += 1;
//             }
//             rros_net_free_skb(skb); //TODO:
//         }
//     }
// }

// fn rros_net_do_tx(arg: *mut c_void){
//     extern "C"{
//         fn rust_helper_list_del_init(list: *mut bindings::list_head);
//     }
//     let list = bindings::list_head::default();
//     init_as_list_head!(list);
//     let dev = unsafe{
//         arg as *mut bindings::net_device
//     };
//     let est = unsafe{
//         (*dev). // TODO: estate
//     }
//     while !rros_kthread_should_stop(){

//         let ret = rros_wait_flag(unsafe{&(*est).flag});
//         if ret{
//             break;
//         }
//         let qdisc : *mut RROSNetQdisc = unsafe{
//             (*est).qdisc
//         };
//         loop{
//             let skb = unsafe{
//                 (*(*qdisc).oob_ops).dequeue(qdisc)
//             };
//             if skb.is_null(){
//                 break;
//             }
//             do_tx(qdisc,dev,skb);
//         }
//         let inband_q = unsafe{
//             &(*qdisc).inband_q
//         };
//         if inband_q.move_queue(&list){
//             // TODO: use macro instead
//             let mut skb = list_first_entry!(&list,bindings::sk_buff,__bindgen_anon_1.list);
//             let mut n = list_next_entry!(pos,bindings::sk_buff,__bindgen_anon_1.list);
//             while !list_entry_is_head!(pos,&list,__bindgen_anon_1.list){
//                 unsafe{
//                     rust_helper_list_del_init(&(*skb).__bindgen_anon_1.list);
//                 }
//                 do_tx(qdisc, dev, skb);
//                 // process next skb
//                 pos = n;
//                 n = list_next_entry!(n,bindings::sk_buff,__bindgen_anon_1.list);
//             }
//         }
//     }
// }

// inband
#[no_mangle]
fn skb_inband_xmit_backlog() {
    extern "C" {
        #[allow(dead_code)]
        fn rust_helper_this_cpu_ptr(ptr: *mut c_void) -> *mut c_void;
    }
    let mut list = bindings::list_head::default();
    init_list_head!(&mut list);
    let flags = OOB_TX_RELAY.irq_lock_noguard(); // TODO: Whether lock is required.

    if unsafe { (*OOB_TX_RELAY.locked_data().get()).move_queue(&mut list) } {
        list_for_each_entry_safe!(
            skb,
            n,
            &mut list,
            bindings::sk_buff,
            {
                let mut ref_skb = RrosSkBuff::from_raw_ptr(skb);
                uncharge_socke_wmem(&mut ref_skb);
                netdevice::dev_queue_xmit(skb);
            },
            __bindgen_anon_1.list
        );
    }
    OOB_TX_RELAY.irq_unlock_noguard(flags);
}

// fn xmit_oob(dev : *mut bindings::net_device, skb : *mut bindings::sk_buff) -> i32{
//     // TODO: est,skb_inband_xmit_backlog,rros_raise_flag
//     let est = unsafe{
//         let ptr  = (*dev).oob_context.dev_state.wrapper as *mut OOBNetdevState;
//         (*ptr).estate
//     };
//     let ret = rros_net_sched_packet(dev,skb);
//     if ret{
//         ret
//     }
//     rros_raise_flag(&est.tx_flag);
//     return 0;
// }

pub fn rros_net_transmit(mut skb: &mut RrosSkBuff) -> Result<()> {
    let dev = skb.dev();
    if dev.is_none() {
        return Err(Error::EINVAL);
    }
    let dev = dev.unwrap();
    if dev.is_vlan_dev() {
        return Err(Error::EINVAL);
    }

    if unsafe { !skb.__bindgen_anon_2.sk.is_null() } {
        // sk_buff->sk
        return Err(Error::EINVAL);
    }

    if !skb.is_oob() {
        return Err(Error::EINVAL);
    }
    // if dev.is_oob_capable(){
    //     return xmit_oob()
    // }

    if kernel::premmpt::running_inband().is_ok() {
        uncharge_socke_wmem(&mut skb);
        netdevice::dev_queue_xmit(skb.0.as_ptr());
    }

    let flags = OOB_TX_RELAY.irq_lock_noguard();
    unsafe { (*OOB_TX_RELAY.locked_data().get()).add(skb) };
    OOB_TX_RELAY.irq_unlock_noguard(flags);
    unsafe {
        OOB_XMIT_WORK.irq_work_queue()?;
    }
    Ok(())
}

// fn netif_xmit_oob(skb : *mut bindings::sk_buff) -> i32{
//     if xmit_oob(unsafe{(*skb).__bindgen_anon_1.__bindgen_anon_1.__bindgen_anon_1.dev}, skb){
//         bindings::NET_XMIT_DROP
//     }else{
//         bindings::NET_XMIT_SUCCESS
//     }
// }

unsafe extern "C" fn xmit_inband(_work: *mut IrqWork) {
    interrupt::__raise_softirq_irqoff(bindings::NET_TX_SOFTIRQ);
}

pub fn rros_init_tx_irqwork() {
    unsafe {
        // OOB_TX_RELAY = alloc_per_cpu(size_of::<RROSNetSkbQueue<*mut bindings::sk_buff>>(),align_of::<RROSNetSkbQueue<*mut bindings::sk_buff>>());
        OOB_XMIT_WORK.init_irq_work(xmit_inband).unwrap();
    }
}
