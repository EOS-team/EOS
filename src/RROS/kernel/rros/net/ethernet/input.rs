use crate::{
    net::{input::rros_net_receive, packet::rros_net_packet_deliver, skb::RrosSkBuff},
    DECLARE_BITMAP,
};
use core::ops::Deref;
use kernel::{bindings, bitmap, c_types::c_void, endian::be16, if_vlan::VlanEthhdr, prelude::*};

fn pick(skb: RrosSkBuff) -> bool {
    rros_net_receive(skb, net_ether_ingress);
    true
}

fn untag(mut skb: RrosSkBuff, ehdr: &mut VlanEthhdr, mac_hdr: *mut u8) -> bool {
    extern "C" {
        #[allow(improper_ctypes)]
        fn rust_helper__vlan_hwaccel_put_tag(
            skb: *mut bindings::sk_buff,
            vlan_proto: be16,
            vlan_tci: u16,
        );
    }
    skb.protocol = ehdr.get().unwrap().h_vlan_encapsulated_proto;
    unsafe {
        rust_helper__vlan_hwaccel_put_tag(
            skb.0.as_ptr(),
            be16::new(ehdr.get().unwrap().h_vlan_proto),
            u16::from(be16::new(ehdr.get().unwrap().h_vlan_TCI)),
        )
    };
    kernel::skbuff::skb_pull(skb.0.as_ptr(), bindings::VLAN_HLEN as u32);
    let mac_len = unsafe { skb.deref().data.offset_from(mac_hdr) as usize };
    if mac_len > (bindings::VLAN_HLEN as usize + bindings::ETH_TLEN as usize) as usize {
        kernel::str::memmove(
            unsafe { mac_hdr.offset(bindings::VLAN_HLEN as isize) as *mut c_void },
            mac_hdr as *mut c_void,
            (mac_len - bindings::VLAN_HLEN as usize - bindings::ETH_TLEN as usize) as u64,
        );
    }
    skb.mac_header += bindings::VLAN_HLEN as u16;
    return pick(skb);
}

pub fn rros_net_ether_accept(skb: RrosSkBuff) -> bool {
    extern "C" {
        fn rust_helper_test_bit(nr: i32, addr: *const usize) -> bool; // TODO: The addr here should be volatile.
        fn rust_helper_eth_type_vlan(ethertype: be16) -> bool;
        #[allow(improper_ctypes)]
        fn rust_helper__vlan_hwaccel_get_tag(
            skb: *mut bindings::sk_buff,
            vlan_tci: *mut u16,
        ) -> i32;
    }
    let vlan_tci: u16 = 0;
    let (tag, test_bit) = unsafe {
        let tag =
            rust_helper__vlan_hwaccel_get_tag(skb.0.as_ptr(), &vlan_tci as *const _ as *mut u16)
                == 0;
        let test_bit = rust_helper_test_bit(
            (vlan_tci & VLAN_VID_MASK as u16) as i32,
            VLAN_MAP.as_ptr() as *const usize,
        );
        (tag, test_bit)
    };
    pr_debug!("tag:{},test_bit:{}\n", tag, test_bit);
    if tag && test_bit {
        pr_debug!("tag && test_bit\n");
        return pick(skb);
    }
    // TODO: The following path has not been tested.
    if skb.vlan_present() == 0 && unsafe { rust_helper_eth_type_vlan(be16::new(skb.protocol)) } {
        pr_debug!("this path is not tested\n");
        let mac_hdr = unsafe { skb.head.offset(skb.mac_header as isize) as *mut u8 };
        let ehdr = mac_hdr as *mut VlanEthhdr;
        pr_debug!("(*ehdr).h_vlan_encapsulated_proto {} ", unsafe {
            (*ehdr).get_mut().h_vlan_encapsulated_proto
        });
        if be16::new(unsafe { (*ehdr).get().unwrap().h_vlan_encapsulated_proto })
            == be16::from(bindings::ETH_P_IP as u16)
        {
            pr_debug!("handle the real packege\n");
            let vlan_tci = unsafe { u16::from(be16::new((*ehdr).get().unwrap().h_vlan_TCI)) };
            pr_debug!("h_vlan_TCI {}\n", vlan_tci);
            if unsafe {
                rust_helper_test_bit(
                    (vlan_tci & VLAN_VID_MASK as u16) as i32,
                    VLAN_MAP.as_ptr() as *const usize,
                )
            } {
                return untag(skb, unsafe { &mut (*ehdr) }, mac_hdr);
            }
        }
    }
    return false;
}

fn net_ether_ingress(mut skb: RrosSkBuff) {
    if rros_net_packet_deliver(&mut skb) {
        return;
    }
    let protocol = u16::from(be16::new(skb.protocol)) as u32;
    match protocol {
        bindings::ETH_P_IP => { /* TODO:Try UDP.. */ }
        _ => {}
    }
    skb.free();
}

const VLAN_N_VID: usize = 4096;
const VLAN_VID_MASK: u32 = 0x0fff;
DECLARE_BITMAP!(VLAN_MAP, VLAN_N_VID);

#[no_mangle]
pub fn rros_net_store_vlans(buf: *const u8, len: usize) -> i32 {
    extern "C" {
        fn rust_helper_test_bit(nr: i32, addr: *const usize) -> bool;
        fn rust_helper_bitmap_copy(dst: *mut u64, src: *const u64, nbit: u32);
    }
    let new_map = bitmap::bitmap_zalloc(VLAN_N_VID as u32, bindings::GFP_KERNEL);
    let mut ret = bitmap::bitmap_parselist(buf as *const i8, new_map, VLAN_N_VID as i32);

    if ret == 0 && unsafe { rust_helper_test_bit(0, new_map as *const usize) }
        || unsafe { rust_helper_test_bit(VLAN_VID_MASK as i32, new_map as *const usize) }
    {
        ret = -(bindings::EINVAL as i32);
    }
    if ret != 0 {
        bitmap::bitmap_free(new_map as *const u64);
        return ret;
    }
    unsafe {
        rust_helper_bitmap_copy(
            VLAN_MAP.as_mut_ptr() as *const _ as *mut u64,
            new_map,
            VLAN_N_VID as u32,
        );
        bitmap::bitmap_free(new_map as *const u64);
    }
    return len as i32;
}

#[allow(unused)]
pub fn rros_show_vlans() {
    extern "C" {
        fn rust_helper_test_bit(nr: i32, addr: *const usize) -> bool;
    }
    let mut counter = 0;
    let mut buffer = [0; 10];
    let mut overflow = false;
    for i in 0..4096 {
        if unsafe { rust_helper_test_bit(i, VLAN_MAP.as_ptr() as *const usize) } {
            buffer[counter] = i;
            counter += 1;
            if counter == 10 {
                overflow = true;
                break;
            }
        }
    }
    if overflow {
        pr_info!("oob net port: {:?} ...(more)", buffer);
    } else {
        pr_info!("oob net port: {:?}", buffer);
    }
}
