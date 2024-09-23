use super::{
    input::RrosNetRxqueue,
    skb::RrosSkBuff,
    socket::{RrosNetProto, RrosSocket, UserOobMsghdr},
};
use crate::{
    net::{
        device::NetDevice,
        ethernet::output::rros_net_ether_transmit,
        socket::{rros_export_iov, rros_import_iov, uncharge_socke_wmem},
    },
    sched::{rros_disable_preempt, rros_enable_preempt},
    timeout::{RrosTmode, RROS_INFINITE, RROS_NONBLOCK},
};
use core::{convert::TryInto, default::Default, mem::transmute, ops::DerefMut, ptr::NonNull, u16};
use kernel::{
    bindings, c_types,
    endian::be16,
    hash_for_each_possible, if_packet, init_static_sync,
    iov_iter::Iovec,
    ktime::{timespec64_to_ktime, KtimeT, Timespec64},
    prelude::*,
    skbuff,
    socket::Sockaddr,
    sync::Lock,
    types::*,
    Error, Result,
};

// protocol hash table
init_static_sync! {
    static PROTOCOL_HASHTABLE: kernel::sync::SpinLock<Hashtable::<8>> = Hashtable::<8>::new();
}

fn get_protol_hash(protocol: be16) -> u32 {
    extern "C" {
        fn rust_helper_jhash(key: *const c_types::c_void, length: u32, initval: u32) -> u32;
    }
    let hsrc: u32 = protocol.raw() as u32;
    unsafe { rust_helper_jhash(&hsrc as *const _ as *const c_types::c_void, 1, 0) }
}

fn find_rxqueue(hkey: u32) -> Option<NonNull<RrosNetRxqueue>> {
    let head = unsafe { (*PROTOCOL_HASHTABLE.locked_data().get()).head(hkey) };
    hash_for_each_possible!(rxq, head, RrosNetRxqueue, hash, {
        if unsafe { (*rxq).hkey } == hkey {
            return NonNull::new(rxq as *mut RrosNetRxqueue);
        }
    });
    None
}
fn find_packet_proto(prtocol: be16) -> Option<&'static dyn RrosNetProto> {
    if prtocol == be16::from(bindings::ETH_P_ALL as u16)
        || prtocol == be16::from(bindings::ETH_P_IP as u16)
    {
        return Some(&ETHERNET_NET_PROTO);
    } else {
        return None;
    }
}

pub struct EthernetRrosNetProto;
pub static ETHERNET_NET_PROTO: EthernetRrosNetProto = EthernetRrosNetProto;
impl RrosNetProto for EthernetRrosNetProto {
    fn attach(&self, sock: &mut RrosSocket, protocol: be16) -> i32 {
        extern "C" {
            fn rust_helper_list_add(
                new: *mut kernel::bindings::list_head,
                head: *mut kernel::bindings::list_head,
            );
        }
        let hkey = get_protol_hash(protocol);
        let rxq = RrosNetRxqueue::new(hkey);
        if rxq.is_none() {
            return -(bindings::ENOMEM as i32);
        }
        let mut rxq = rxq.unwrap();
        let mut redundant_rxq = false;
        let flags = PROTOCOL_HASHTABLE.irq_lock_noguard();
        sock.proto = Some(&ETHERNET_NET_PROTO);
        sock.binding.proto_hash = hkey;
        sock.protocol = protocol;

        let _rxq = find_rxqueue(hkey);
        if let Some(queue) = _rxq {
            redundant_rxq = true;
            let queue = unsafe { &mut *queue.as_ptr() };
            //rros_spin_lock
            rros_disable_preempt();
            queue.lock.lock_noguard();
            unsafe { rust_helper_list_add(&mut sock.next_sub, &mut queue.subscribers) }
            // rros_spin_unlock
            unsafe { queue.lock.unlock() };
            rros_enable_preempt();
            // drop q_guard here
        } else {
            let queue = unsafe { &mut *rxq.as_ptr() };
            unsafe { (*PROTOCOL_HASHTABLE.locked_data().get()).add(&mut queue.hash.0, hkey) };
            unsafe { rust_helper_list_add(&mut sock.next_sub, &mut queue.subscribers) }
        }
        PROTOCOL_HASHTABLE.irq_unlock_noguard(flags);

        if redundant_rxq {
            unsafe { rxq.as_mut().free() };
        }

        0
    }
    fn detach(&self, sock: &mut RrosSocket) {
        extern "C" {
            fn rust_helper_list_empty(head: *const kernel::bindings::list_head) -> bool;
        }
        if unsafe { rust_helper_list_empty(&sock.next_sub) } {
            return;
        }
        let mut tmp = bindings::list_head::default();
        init_list_head!(&mut tmp);

        let flags = PROTOCOL_HASHTABLE.irq_lock_noguard();

        let rxq = unsafe { find_rxqueue(sock.binding.proto_hash).unwrap().as_mut() };

        list_del_init!(&mut sock.next_sub);
        if unsafe { rust_helper_list_empty(&rxq.subscribers) } {
            unsafe { (*PROTOCOL_HASHTABLE.locked_data().get()).del(&mut rxq.hash.0) };
            list_add!(&mut rxq.next, &mut tmp);
        }

        PROTOCOL_HASHTABLE.irq_unlock_noguard(flags);

        list_for_each_entry_safe!(
            rxq,
            next,
            &mut tmp,
            RrosNetRxqueue,
            {
                unsafe { (*rxq).free() };
            },
            next
        );
    }
    fn bind(&self, sock: &mut RrosSocket, addr: &Sockaddr, len: i32) -> i32 {
        extern "C" {
            #[allow(improper_ctypes)]
            fn rust_helper_vlan_dev_vlan_id(dev: *const bindings::net_device) -> u16;
            #[allow(improper_ctypes)]
            fn rust_helper_vlan_dev_real_dev(
                dev: *const bindings::net_device,
            ) -> *const bindings::net_device;
        }
        if len != core::mem::size_of::<if_packet::SockaddrLL>() as i32 {
            return -(bindings::EINVAL as i32);
        }
        let sll = unsafe { &mut *(addr as *const _ as *mut if_packet::SockaddrLL) };
        if sll.get_mut().sll_family != bindings::AF_PACKET as u16 {
            return -(bindings::EINVAL as i32);
        }
        let proto = find_packet_proto(be16::new(sll.get_mut().sll_protocol));
        if proto.is_none() {
            return -(bindings::EINVAL as i32);
        }
        let _proto = proto.unwrap();
        let new_ifindex = sll.get_mut().sll_ifindex;

        sock.lock.lock_noguard();
        let old_ifindex = sock.binding.vlan_ifindex;
        let mut vlan_id = 0;
        let mut real_ifindex = 0;
        let dev: Option<NetDevice> = None;
        if new_ifindex != old_ifindex {
            // switch to new device
            if new_ifindex != 0 {
                if let Some(dev) = NetDevice::net_get_dev_by_index(sock.net, new_ifindex) {
                    vlan_id = unsafe { rust_helper_vlan_dev_vlan_id(dev.0.as_ptr() as *const _) };
                    real_ifindex = unsafe {
                        (*rust_helper_vlan_dev_real_dev(dev.0.as_ptr() as *const _)).ifindex
                    };
                } else {
                    return -(bindings::EINVAL as i32);
                }
            } else {
                vlan_id = 0;
                real_ifindex = 0;
            }
        }
        sll.get_mut().sll_ifindex = real_ifindex;

        if sock.protocol != be16::new(sll.get_mut().sll_protocol) {
            self.detach(sock);
            let ret = self.attach(sock, be16::new(sll.get_mut().sll_protocol));
            if ret != 0 {
                unsafe { sock.oob_lock.unlock() };
                if dev.is_some() {
                    let mut dev = dev.unwrap();
                    dev.put_dev();
                }
                return ret;
            }
        }
        let flags = sock.oob_lock.irq_lock_noguard();
        if new_ifindex != old_ifindex {
            sock.binding.real_ifindex = real_ifindex;
            sock.binding.vlan_id = vlan_id;
            sock.binding.vlan_ifindex = new_ifindex;
        }
        sock.oob_lock.irq_unlock_noguard(flags);
        unsafe { sock.oob_lock.unlock() };
        if dev.is_some() {
            let mut dev = dev.unwrap();
            dev.put_dev();
        }
        return 0;
    }
    fn oob_send(
        &self,
        sock: &mut RrosSocket,
        msghdr: *mut UserOobMsghdr,
        iov_vec: &mut [Iovec],
    ) -> isize {
        extern "C" {
            fn rust_helper_raw_get_user(result: *mut u32, addr: *const u32) -> isize;
            fn rust_helper_raw_copy_from_user(dst: *mut u8, src: *const u8, size: usize) -> usize;
            #[allow(improper_ctypes)]
            fn rust_helper_skb_tailroom(skb: *const bindings::sk_buff) -> i32;
            #[allow(improper_ctypes)]
            fn rust_helper_dev_validate_header(
                dev: *const bindings::net_device,
                header: *const u8,
                len: i32,
            ) -> bool;
            #[allow(improper_ctypes)]
            fn rust_helper_dev_parse_header_protocol(skb: *const bindings::sk_buff) -> be16;
            #[allow(improper_ctypes)]
            fn rust_helper_skb_set_network_header(skb: *mut bindings::sk_buff, offset: i32);
        }
        let mut msg_flags = 0;
        let msghdr = unsafe { &mut *msghdr };
        let ret = unsafe { rust_helper_raw_get_user(&mut msg_flags, &msghdr.flags) };
        if ret != 0 {
            return -(bindings::EFAULT as isize);
        }
        if msg_flags & !bindings::MSG_DONTWAIT != 0 {
            return -(bindings::EINVAL as isize);
        }
        if sock.efile.flags() & bindings::O_NONBLOCK != 0 {
            msg_flags |= bindings::MSG_DONTWAIT;
        }
        let mut uts = Timespec64::default();
        let ret = unsafe {
            rust_helper_raw_copy_from_user(
                &mut uts as *const _ as *mut u8,
                &msghdr.timeout as *const _ as *const u8,
                core::mem::size_of::<Timespec64>(),
            )
        };
        if ret != 0 {
            return -(bindings::EFAULT as isize);
        }
        let timeout = if msg_flags & bindings::MSG_DONTWAIT != 0 {
            RROS_NONBLOCK
        } else {
            timespec64_to_ktime(uts)
        };
        let tmode = if timeout != 0 {
            RrosTmode::RrosAbs
        } else {
            RrosTmode::RrosRel
        };

        let dev = find_xmit_device(sock, msghdr);
        if dev.is_err() {
            return -(dev.err().unwrap().to_kernel_errno() as isize);
        }
        let mut dev = dev.unwrap();
        let mut real_dev = dev.vlan_dev_real_dev();
        let skb = RrosSkBuff::dev_alloc_skb(&mut real_dev, timeout, tmode);
        if skb.is_none() {
            // put_dev
            // panic!();
            return -(bindings::ENOMEM as isize);
        }
        let mut skb = skb.unwrap();
        skb.reset_mac_header();
        skb.protocol = sock.protocol.raw();
        skb.set_dev(real_dev.0.as_ptr());
        skb.priority = unsafe { (*sock.sk).get_mut().sk_priority };
        let skb_tailroom = unsafe { rust_helper_skb_tailroom(skb.0.as_ptr()) } as usize;
        let mut rem: usize = 0;
        let count = rros_import_iov(
            iov_vec,
            skb.deref_mut().data,
            skb_tailroom as u64,
            Some(&mut rem),
        );
        if rem != 0
            || count as u32
                > (unsafe { dev.0.as_ref().mtu + dev.0.as_ref().hard_header_len as u32 }
                    + bindings::VLAN_HLEN) as u32
        {
            skb.free();
            dev.put_dev();
            return -(bindings::EMSGSIZE as isize);
        } else if unsafe { !rust_helper_dev_validate_header(dev.0.as_ptr(), skb.data, count) } {
            skb.free();
            dev.put_dev();
            return -(bindings::EINVAL as isize);
        }

        skb.put(count as u32);

        let skb_protocol = unsafe { be16::new(skb.0.as_ref().protocol) };
        if skb_protocol == be16::new(0) || skb_protocol == be16::from(bindings::ETH_P_ALL as u16) {
            unsafe {
                skb.0.as_mut().protocol =
                    rust_helper_dev_parse_header_protocol(skb.0.as_ptr()).raw();
            }
        }
        unsafe {
            rust_helper_skb_set_network_header(
                skb.0.as_ptr(),
                real_dev.0.as_ref().hard_header_len as i32,
            )
        };

        // sock.charge_socket_wmem_timeout(&mut skb, timeout, tmode);
        sock.charge_socket_wmem(NonNull::new(&mut skb).unwrap());

        let ret = rros_net_ether_transmit(&mut dev, &mut skb);
        if ret != 0 {
            uncharge_socke_wmem(&mut skb);
            skb.free();
            dev.put_dev();
            return ret as isize;
        }
        let ret = count as isize;
        dev.put_dev();
        ret
    }
    fn oob_receive(
        &self,
        sock: &mut RrosSocket,
        msg: *mut UserOobMsghdr,
        iov_vec: &mut [Iovec],
    ) -> isize {
        // User mode.
        extern "C" {
            fn rust_helper_raw_get_user(result: *mut u32, addr: *const u32) -> isize;
            fn rust_helper_raw_copy_from_user(dst: *mut u8, src: *const u8, size: usize) -> usize;
            fn rust_helper_list_empty(list: *const bindings::list_head) -> bool;
            #[allow(improper_ctypes)]
            fn rust_helper_skb_mac_header(skb: *const bindings::sk_buff) -> *const u8;
        }
        let mut timeout: KtimeT = RROS_INFINITE;
        let mut tmode = RrosTmode::RrosRel;
        let mut msg_flags = 0;
        let mut uts = Timespec64::default();
        let mut ret: i32;
        if !msg.is_null() {
            let ret = unsafe { rust_helper_raw_get_user(&mut msg_flags, &(*msg).flags) };
            if ret != 0 {
                return -(bindings::EFAULT as isize);
            }
            if msg_flags & !bindings::MSG_DONTWAIT != 0 {
                return -(bindings::EINVAL as isize);
            }

            if unsafe {
                rust_helper_raw_copy_from_user(
                    &mut uts as *const _ as *mut u8,
                    &(*msg).timeout as *const _ as *const u8,
                    core::mem::size_of::<Timespec64>(),
                )
            } != 0
            {
                return -(bindings::EFAULT as isize);
            }
            timeout = timespec64_to_ktime(uts);
            if timeout != 0 {
                tmode = RrosTmode::RrosAbs;
            } else {
                tmode = RrosTmode::RrosRel;
            }
        } else {
            // do nothing
        }
        if (sock.efile.flags() as u32) & bindings::O_NONBLOCK != 0 {
            msg_flags |= bindings::MSG_DONTWAIT;
        }

        loop {
            let flags = sock.input_wait.lock.raw_spin_lock_irqsave();

            if !unsafe { rust_helper_list_empty(&mut sock.input) } {
                let skb =
                    list_get_entry!(&mut sock.input, bindings::sk_buff, __bindgen_anon_1.list);
                let mut skb = RrosSkBuff::from_raw_ptr(skb);
                sock.input_wait.lock.raw_spin_unlock_irqrestore(flags);

                unsafe {
                    let len = skb
                        .data
                        .offset_from(rust_helper_skb_mac_header(skb.0.as_ptr())); //skb->data - skb_mac_header(skb))
                    skbuff::skb_push(skb.0.as_ptr(), len as u32);
                }
                let ret = copy_packet_to_user(msg, iov_vec, &mut skb);
                sock.uncharge_socke_rmem(&skb);
                skb.free();
                return ret;
            }

            if msg_flags & bindings::MSG_DONTWAIT != 0 {
                sock.input_wait.lock.raw_spin_unlock_irqrestore(flags);
                return -(bindings::EWOULDBLOCK as isize);
            }

            pr_debug!("~~~~~~~~~~ wait for input;");
            sock.input_wait.locked_add(timeout, tmode);
            pr_debug!("~~~~~~~~~~ wait for input;2");
            sock.input_wait.lock.raw_spin_unlock_irqrestore(flags);
            ret = sock.input_wait.wait_schedule();
            if ret != 0 {
                break;
            }
        }
        return ret as isize;
    }

    fn get_netif(&self, sock: &mut RrosSocket) -> Option<NetDevice> {
        NetDevice::net_get_dev_by_index(sock.net, sock.binding.vlan_ifindex)
    }
}

fn copy_packet_to_user(
    msg: *mut UserOobMsghdr,
    iov_vec: &mut [Iovec],
    skb: &mut RrosSkBuff,
) -> isize {
    extern "C" {
        fn rust_helper_raw_get_user(result: *mut u32, addr: *const u32) -> isize;
        fn rust_helper_raw_get_user_64(result: *mut u64, addr: *const u64) -> isize;

        #[allow(improper_ctypes)]
        fn rust_helper_dev_parse_header(skb: *mut bindings::sk_buff, haddr: *mut u8) -> i32;
        fn rust_helper_raw_copy_to_user(dst: *mut u8, src: *const u8, size: usize) -> usize;
        fn rust_helper_raw_put_user(x: u32, ptr: *mut u32) -> i32;
    }
    let mut name_ptr: u64 = 0;
    let mut namelen: u32 = 0;
    let u_addr: *mut if_packet::SockaddrLL;
    let mut msg_flags = 0;
    if unsafe { rust_helper_raw_get_user_64(&mut name_ptr, &(*msg).name_ptr) } != 0 {
        return -(bindings::EFAULT as isize);
    }

    if unsafe { rust_helper_raw_get_user(&mut namelen, &(*msg).name_len) } != 0 {
        return -(bindings::EFAULT as isize);
    }
    let mut addr = if_packet::SockaddrLL::default();
    if name_ptr == 0 {
        if namelen != 0 {
            return -(bindings::EINVAL as isize);
        }
    } else {
        if namelen < core::mem::size_of::<if_packet::SockaddrLL>() as u32 {
            return -(bindings::EINVAL as isize);
        }
        addr.get_mut().sll_family = bindings::AF_PACKET as u16;
        addr.get_mut().sll_protocol = skb.protocol;
        let dev = skb.dev().unwrap();
        addr.get_mut().sll_ifindex = dev.ifindex();
        addr.get_mut().sll_hatype = unsafe { dev.0.as_ref().type_ };
        addr.get_mut().sll_pkttype = unsafe { skb.0.as_ref().pkt_type() };
        addr.get_mut().sll_halen = unsafe {
            rust_helper_dev_parse_header(
                skb.0.as_ptr(),
                &mut addr.get_mut().sll_addr as *const _ as *mut u8,
            )
        }
        .try_into()
        .unwrap();
        unsafe { u_addr = transmute(name_ptr) }
        if unsafe {
            rust_helper_raw_copy_to_user(
                u_addr as *const _ as *mut u8,
                &addr as *const _ as *const u8,
                core::mem::size_of::<if_packet::SockaddrLL>(),
            )
        } != 0
        {
            return -(bindings::EFAULT as isize);
        }
    }
    let count = rros_export_iov(iov_vec, skb.data as *const _ as *mut u8, skb.len as usize);

    if (count as u32) < skb.len {
        msg_flags |= bindings::MSG_TRUNC;
    }
    if unsafe { rust_helper_raw_put_user(msg_flags, &mut (*msg).flags as *mut u32) } != 0 {
        return -(bindings::EFAULT as isize);
    }

    return count as isize;
}

fn __packet_deliver(rxq: &mut RrosNetRxqueue, skb: &mut RrosSkBuff, protocol: be16) -> bool {
    extern "C" {
        #[allow(improper_ctypes)]
        fn rust_helper_skb_vlan_tag_get_id(skb: *mut bindings::sk_buff) -> u16;
    }
    rros_disable_preempt();
    rxq.lock.lock_noguard();
    let dev = skb.dev().unwrap();
    let mut delivered = false;

    // list_for_each_entry, there are `continue` and `break` here, so write it directly.
    let mut rsk = list_first_entry!(&mut rxq.subscribers, RrosSocket, next_sub);
    while !list_entry_is_head!(rsk, &mut rxq.subscribers, next_sub) {
        let ref_rsk = unsafe { &mut *rsk };
        let ifindex = ref_rsk.binding.real_ifindex; // TODO: read_once
        if ifindex != 0 {
            if ifindex != dev.ifindex() {
                rsk = list_next_entry!(rsk, RrosSocket, next_sub);
                continue;
            }
            let vlan_id = ref_rsk.binding.vlan_id;
            if unsafe { rust_helper_skb_vlan_tag_get_id(skb.0.as_ptr()) } != vlan_id {
                continue;
            }
        }
        if !ref_rsk.charge_socket_rmem(skb) {
            continue;
        }

        let mut qskb = if protocol == be16::from(bindings::ETH_P_ALL as u16) {
            if let Some(clone) = skb.net_clone_skb() {
                clone
            } else {
                unimplemented!();
            }
        } else {
            RrosSkBuff::from_raw_ptr(skb.0.as_ptr()) // DANGEROUS
        };
        ref_rsk.input_wait.lock.raw_spin_lock();
        list_add_tail!(qskb.list_mut(), &mut ref_rsk.input);
        if ref_rsk.input_wait.is_active() {
            ref_rsk.input_wait.wake_up_head();
        }
        ref_rsk.input_wait.lock.raw_spin_unlock();

        // rros_signal_poll_events(&esk->poll_head,	POLLIN|POLLRDNORM);
        delivered = true;

        if protocol != be16::from(bindings::ETH_P_ALL as u16) {
            break;
        }

        rsk = list_next_entry!(rsk, RrosSocket, next_sub);
    }
    unsafe { rxq.lock.unlock() };

    delivered
}

fn packet_deliver(skb: &mut RrosSkBuff, protocol: be16) -> bool {
    let hkey = get_protol_hash(protocol);

    let mut ret = false;
    let flags = PROTOCOL_HASHTABLE.irq_lock_noguard();

    if let Some(mut rxq) = find_rxqueue(hkey) {
        ret = __packet_deliver(unsafe { rxq.as_mut() }, skb, protocol);
    }

    PROTOCOL_HASHTABLE.irq_unlock_noguard(flags);
    ret
}

pub fn rros_net_packet_deliver(skb: &mut RrosSkBuff) -> bool {
    // rros_net_packet_deliver
    packet_deliver(skb, be16::from(bindings::ETH_P_ALL as u16));

    // return packet_deliver(skb, be16::new(skb.protocol));
    false
}

fn find_xmit_device(rsk: &mut RrosSocket, msghdr: &mut UserOobMsghdr) -> Result<NetDevice> {
    extern "C" {
        fn rust_helper_raw_get_user(result: *mut u32, addr: *const u32) -> isize;
        fn rust_helper_raw_get_user_64(result: *mut u64, addr: *const u64) -> isize;
        fn rust_helper_raw_copy_from_user(dst: *mut u8, src: *const u8, size: usize) -> usize;
    }
    let mut name_ptr: u64 = 0;
    let mut namelen: u32 = 0;
    let ret =
        unsafe { rust_helper_raw_get_user_64(&mut name_ptr, &mut (*msghdr).name_ptr as *mut u64) };
    if ret != 0 {
        return Err(Error::EFAULT);
    }

    let ret =
        unsafe { rust_helper_raw_get_user(&mut namelen, &mut (*msghdr).name_len as *mut u32) };
    if ret != 0 {
        return Err(Error::EFAULT);
    }

    let dev = if name_ptr == 0 {
        if namelen != 0 {
            return Err(Error::EINVAL);
        }
        let proto = rsk.proto.unwrap();
        proto.get_netif(rsk)
    } else {
        if namelen < core::mem::size_of::<if_packet::SockaddrLL>() as u32 {
            return Err(Error::EINVAL);
        }

        let u_addr: *mut if_packet::SockaddrLL = unsafe { transmute(name_ptr) };
        let mut addr: if_packet::SockaddrLL = if_packet::SockaddrLL::default();
        let ret = unsafe {
            rust_helper_raw_copy_from_user(
                &mut addr as *const _ as *mut u8,
                u_addr as *const _ as *const u8,
                core::mem::size_of::<if_packet::SockaddrLL>(),
            )
        };
        if ret != 0 {
            return Err(Error::EFAULT);
        }

        if addr.get_mut().sll_family != bindings::AF_PACKET as u16
            && addr.get_mut().sll_family != bindings::AF_UNSPEC as u16
        {
            return Err(Error::EINVAL);
        }
        NetDevice::net_get_dev_by_index(rsk.net, addr.get_mut().sll_ifindex)
    };
    if dev.is_none() {
        return Err(Error::EFAULT); // TODO: ENXIO
    }
    Ok(dev.unwrap())
}
