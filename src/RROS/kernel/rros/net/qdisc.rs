use kernel::bindings;

use super::skb::RROSNetSkbQueue;
pub struct RROSNetQdisc {
    pub oob_ops: *const RROSNetQdiscOps,
    pub inband_q: RROSNetSkbQueue<*mut bindings::sk_buff>,
    pub pack_dropped: usize,
}
pub struct RROSNetQdiscOps {
    // name: &'static u8,
    pub priv_size: usize,
    pub init: fn(qdisc: *mut RROSNetQdisc) -> i32,
    pub destory: fn(qdisc: *mut RROSNetQdisc),
    pub enqueue: fn(qdisc: *mut RROSNetQdisc, skb: *mut bindings::sk_buff) -> i32,
    pub dequeue: fn(qdisc: *mut RROSNetQdisc) -> *mut bindings::sk_buff,
    pub next: bindings::list_head,
}
