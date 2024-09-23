use crate::net::{device::NetDevice, output::rros_net_transmit, skb::RrosSkBuff};
use kernel::{bindings, endian::be16};

pub fn rros_net_ether_transmit(dev: &mut NetDevice, skb: &mut RrosSkBuff) -> i32 {
    extern "C" {
        #[allow(improper_ctypes)]
        fn rust_helper_vlan_dev_vlan_proto(dev: *mut bindings::net_device) -> be16;
        #[allow(improper_ctypes)]
        fn rust_helper_vlan_dev_vlan_id(dev: *const bindings::net_device) -> u16;
        #[allow(improper_ctypes)]
        fn rust_helper_vlan_dev_get_egress_qos_mask(
            dev: *mut bindings::net_device,
            mask: u32,
        ) -> u16;
        #[allow(improper_ctypes)]
        fn rust_helper__vlan_hwaccel_put_tag(
            skb: *mut bindings::sk_buff,
            vlan_proto: be16,
            vlan_tci: u16,
        );
    }
    if !dev.is_vlan_dev() {
        return -(bindings::EINVAL as i32);
    }
    let vlan_proto = unsafe { rust_helper_vlan_dev_vlan_proto(dev.0.as_ptr()) };
    let mut vlan_tci =
        unsafe { rust_helper_vlan_dev_vlan_id(dev.0.as_ptr() as *const bindings::net_device) };
    vlan_tci |= unsafe { rust_helper_vlan_dev_get_egress_qos_mask(dev.0.as_ptr(), skb.priority) };
    unsafe {
        rust_helper__vlan_hwaccel_put_tag(skb.0.as_ptr(), vlan_proto, vlan_tci);
    }
    let ret = rros_net_transmit(skb);
    if ret.is_ok() {
        return 0;
    } else {
        return ret.err().unwrap().to_kernel_errno();
    }
}
