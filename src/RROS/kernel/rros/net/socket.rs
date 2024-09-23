/*
 * RROS sockets are (almost) regular sockets, extended with out-of-band
 * capabilities. In theory, this would allow us to provide out-of-band
 * services on top of any common protocol already handled by the
 * in-band network stack. RROS-specific protocols belong to the generic
 * PF_OOB family, which we use as a protocol mutiplexor.
 */
use crate::{
    bindings, c_types, clock::RROS_MONO_CLOCK, crossing::RrosCrossing, file::RrosFile,
    poll::RrosPollHead, timeout::RrosTmode, wait::RrosWaitQueue, THIS_MODULE,
};

use kernel::{
    container_of,
    endian::be16,
    iov_iter::Iovec,
    ktime::{KtimeT, Timespec64},
    mutex_init,
    net::{create_socket_callback, CreateSocket, Namespace, NetProtoFamily, Socket},
    prelude::*,
    sock::Sock,
    socket::Sockaddr,
    spinlock_init, static_init_net_proto_family,
    sync::{Mutex, SpinLock},
    types::HlistNode,
    vmalloc::{c_kzalloc, c_kzfree},
    Error,
};

use core::{
    default::Default,
    mem::transmute,
    pin::Pin,
    ptr,
    ptr::NonNull,
    sync::atomic::{AtomicI32, Ordering},
};

use super::device::NetDevice;
use super::packet::ETHERNET_NET_PROTO;
use super::skb::{free_skb_list, RrosSkBuff};

macro_rules! sk_family {
    ($x:ident) => {
        unsafe { (*$x).__sk_common.skc_family }
    };
}

pub struct RrosNetdevActivation {
    pub poolsz: u64,
    pub bufsz: u64,
}

#[repr(C)]
pub struct UserOobMsghdr {
    pub name_ptr: u64,
    pub iov_ptr: u64,
    pub ctl_ptr: u64,
    pub name_len: u32,
    pub iovlen: u32,
    pub ctllen: u32,
    pub count: i32,
    pub flags: u32,
    pub timeout: Timespec64,
    pub timestamp: Timespec64,
}

pub trait RrosNetProto {
    fn attach(&self, sock: &mut RrosSocket, protocol: be16) -> i32;
    fn detach(&self, sock: &mut RrosSocket);
    fn bind(&self, sock: &mut RrosSocket, addr: &Sockaddr, len: i32) -> i32;
    // fn ioctl(&mut self, cmd:u32, arg:u64) -> i32;
    fn oob_send(
        &self,
        sock: &mut RrosSocket,
        msg: *mut UserOobMsghdr,
        iov_vec: &mut [Iovec],
    ) -> isize;
    fn oob_receive(
        &self,
        sock: &mut RrosSocket,
        msg: *mut UserOobMsghdr,
        iov_vec: &mut [Iovec],
    ) -> isize;
    // fn oob_poll(&mut self, wait:&oob_poll_wait) -> __poll_t;
    fn get_netif(&self, sock: &mut RrosSocket) -> Option<NetDevice>;
}
pub struct Binding {
    pub real_ifindex: i32,
    pub vlan_ifindex: i32,
    pub vlan_id: u16,
    pub proto_hash: u32,
}
pub struct RrosSocket {
    pub proto: Option<&'static dyn RrosNetProto>,
    pub efile: RrosFile,
    pub lock: Mutex<()>,
    pub net: *mut Namespace,
    pub hash: HlistNode,
    pub input: kernel::bindings::list_head,
    pub input_wait: RrosWaitQueue,
    pub poll_head: RrosPollHead,
    pub next_sub: kernel::bindings::list_head,
    pub sk: *mut Sock,
    pub rmem_count: AtomicI32,
    pub rmem_max: i32,
    pub wmem_count: AtomicI32,
    pub wmem_max: i32,
    pub wmem_wait: RrosWaitQueue,
    pub wmem_drain: RrosCrossing,
    pub protocol: be16,
    pub binding: Binding,
    pub oob_lock: SpinLock<()>,
}

const RROS_SOCKIOC_RECVMSG: u32 = 3226529285;
const RROS_SOCKIOC_SENDMSG: u32 = 1079045636;

impl RrosSocket {
    pub fn from_socket(sock: Socket) -> NonNull<Self> {
        // evk_sk
        unsafe {
            NonNull::new((*(*sock.get_ptr()).sk).oob_data as *const _ as *mut RrosSocket).unwrap()
        }
    }
    #[inline]
    pub fn from_file(filp: *mut bindings::file) -> Option<NonNull<Self>> {
        unsafe {
            if !(*filp).oob_data.is_null() {
                let ptr = container_of!((*filp).oob_data, RrosSocket, efile) as *mut RrosSocket;
                Some(NonNull::new_unchecked(ptr))
            } else {
                None
            }
        }
    }

    #[inline]
    pub fn charge_socket_rmem(&mut self, skb: &RrosSkBuff) -> bool {
        if self.rmem_count.load(Ordering::Relaxed) >= self.rmem_max {
            return false;
        }
        self.rmem_count
            .fetch_add(skb.truesize as i32, Ordering::Relaxed);
        true
    }
    #[inline]
    pub fn uncharge_socke_rmem(&mut self, skb: &RrosSkBuff) {
        self.rmem_count
            .fetch_sub(skb.truesize as i32, Ordering::Relaxed);
    }

    // atomic operation. so it borrow immutable self
    #[inline]
    pub fn charge_socket_wmem(&self, mut skb: NonNull<RrosSkBuff>) -> bool {
        if self.wmem_count.load(Ordering::Relaxed) >= self.wmem_max {
            return false;
        }

        self.wmem_count
            .fetch_add(unsafe { skb.as_ref().truesize } as i32, Ordering::Relaxed);
        unsafe {
            skb.as_mut().net_cb_mut().tracker = self as *const _ as *mut RrosSocket;
        }
        self.wmem_drain.down();
        true
    }

    #[allow(dead_code)]
    pub fn charge_socket_wmem_timeout(
        &mut self,
        skb: &mut RrosSkBuff,
        timeout: KtimeT,
        tmode: RrosTmode,
    ) -> i32 {
        // rros_charge_socket_wmem
        skb.net_cb_mut().tracker = ptr::null_mut();
        if self.wmem_max == 0 {
            return 0;
        }
        let arg = unsafe { NonNull::new_unchecked(skb) };
        // TODO: Bypass mutable borrow check
        let wmem_wait = unsafe { &mut *(&self.wmem_wait as *const _ as *mut RrosWaitQueue) };
        return wmem_wait.wait_timeout(timeout, tmode, || self.charge_socket_wmem(arg));
    }

    pub fn send_or_recv(&mut self, msghdr: *mut UserOobMsghdr, cmd: i32) -> i32 {
        extern "C" {
            fn rust_helper_raw_get_user(result: *mut u32, addr: *const u32) -> isize;
            fn rust_helper_raw_get_user_64(result: *mut u64, addr: *const u64) -> isize;

            fn rust_helper_raw_put_user(x: u32, ptr: *mut u32) -> i32;
        }
        let mut fast_iov: [Iovec; 8] = [
            Iovec::default(),
            Iovec::default(),
            Iovec::default(),
            Iovec::default(),
            Iovec::default(),
            Iovec::default(),
            Iovec::default(),
            Iovec::default(),
        ];
        let mut iov_ptr: u64 = 0;
        let mut iovlen: u32 = 0;
        if unsafe { rust_helper_raw_get_user_64(&mut iov_ptr, &(*msghdr).iov_ptr) } != 0 {
            return -(bindings::EFAULT as i32);
        }

        if unsafe { rust_helper_raw_get_user(&mut iovlen, &(*msghdr).iovlen) } != 0 {
            return -(bindings::EFAULT as i32);
        }
        let u_iov: *mut Iovec = unsafe { transmute(iov_ptr) };
        let iov = load_iov(u_iov, iovlen as usize, fast_iov.as_mut_ptr()).unwrap();

        let proto = self.proto.unwrap();
        let iov_vec = unsafe { core::slice::from_raw_parts_mut(iov, iovlen as usize) };
        let count = if cmd == (RROS_SOCKIOC_SENDMSG as i32) {
            proto.oob_send(self, msghdr, iov_vec)
        } else {
            proto.oob_receive(self, msghdr, iov_vec)
        };

        if core::ptr::eq(iov, fast_iov.as_mut_ptr()) {
            // free iov
            // TODO:
        }
        if unsafe {
            rust_helper_raw_put_user(count as u32, &mut (*msghdr).count as *const _ as *mut u32)
        } != 0
        {
            return -(bindings::EFAULT as i32);
        }
        return 0;
    }
}

pub fn uncharge_socke_wmem(skb: &mut RrosSkBuff) {
    let rsk = skb.net_cb_mut().tracker;
    if rsk.is_null() {
        return;
    }
    let rsk = unsafe { &mut *rsk };
    let flags = rsk.wmem_wait.lock.raw_spin_lock_irqsave();

    skb.net_cb_mut().tracker = ptr::null_mut();

    let _count = rsk
        .wmem_count
        .fetch_sub(skb.truesize as i32, Ordering::Relaxed);
    // if count < rsk.wmem_max && rsk.wmem_wait.is_active(){
    //     rsk.wmem_wait.flush_locked(0);
    // }

    rsk.wmem_drain.up();
    rsk.wmem_wait.lock.raw_spin_unlock_irqrestore(flags);
}

// // default operation for socket
// const netproto_ops:binding::proto_ops = binding::proto_ops{
//     family = binding::PF_OOB,
//     owner = ThisModule,
//     release = rros_sock_release,
//     bind = rros_sock_bind,
//     connect = bindings::sock_no_connect,
//     socketpair = bindings::sock_no_socketpair,
//     accept = bindings::sock_no_accept,
//     getname =	bindings::sock_no_getname,
// 	// ioctl =	sock_inband_ioctl,  // TODO
// 	ioctl =	bindings::sock_no_ioctl,
// 	listen =	bindings::sock_no_listen,
// 	shutdown =	bindings::sock_no_shutdown,
// 	sendmsg =	bindings::sock_no_sendmsg,
// 	recvmsg =	bindings::sock_no_recvmsg,
// 	mmap =		bindings::sock_no_mmap,
// 	sendpage =	bindings::sock_no_sendpage,
// };
// const rros_af_oob_proto: bindings::proto = bindings::proto{
// 	name		= "RROS", //TODO: [c_types::c_char; 32usize]
// 	owner		= ThisModule,
// 	obj_size	= core::mem::size_of(RrosSocket),
// };

pub struct CreateRrosSocket;

impl CreateSocket for CreateRrosSocket {
    fn create_socket(_net: *mut Namespace, _sock: &mut Socket, _protocol: i32, _kern: i32) -> i32 {
        unimplemented!();
    }
}
//     // extern "C"{
//     //     fn sk_refcnt_debug_inc()
//     //     fn local_bh_disable();
//     //     fn sock_prot_inuse_add();
//     //     fn local_bh_enable();
//     // }
//     if kern{
//         return -(bindings::EOPNOTSUPP as i32);
//     }
//     unsafe{(*sock).state = bindings::socket_state_SS_UNCONNECTED;}
//     let sk : *mut bindings::sock = unsafe{bindings::sk_alloc(net,bindings::PF_OOB,bindings::GFP_KERNEL,0)};
//     if sk.is_null(){
//         return -(bindings::ENOBUFS as i32);
//     }
//     unsafe{(*sock).ops = &netproto_ops as *const netproto_ops};
//     unsafe{bindings::sock_init_data(sock,sk);}

//     /*
// 	 * Protocol is checked for validity when the socket is
// 	 * attached to the out-of-band core in sock_oob_attach().
// 	 */
//     unsafe{(*sock).sk_protocol = protocol};
//     // sk_refcnt_debug_inc(sk); TODO: Stuff for Debug
//     unsafe{(*sock).sk_destruct = &destroy_rros_socket as ::core::option::Option<unsafe extern "C" fn(sk: *mut sock)>};

//     // unsafe{local_bh_disable();}
// 	// sock_prot_inuse_add(net,unsafe{&rros_af_oob_proto as *mut proto}, 1 as c_types::c_int); # TODO: CONFIG_PROC_FS
// 	// unsafe{local_bh_enable();}
//     0
// }

// #[no_mangle]
// fn rros_sock_release(sock:*mut bindings::socket) -> i32
// {
//     /*
// 	 * Cleanup happens from sock_oob_detach(), so that PF_OOB
// 	 * and common protocols sockets we piggybacked on are
// 	 * released.
// 	 */
// 	0
// }

// #[no_mangle]
// fn rros_sock_bind(sock: *mut bindings::socket,u_addr:*mut bindings::sockaddr, len:i32)->i32
// {
//     let esk = rros_sk(sock).expect("rros_sk(sock) unwrap failed");
//     esk.proto.as_mut().expect("get proto failed").bind(esk,u_addr,len)
// }

no_mangle_function_declaration! {
    fn sock_oob_attach(sock: Socket) -> i32
    {
        extern "C" {
            #[allow(improper_ctypes)]
            #[allow(dead_code)]
            fn rust_helper_is_err(ptr: *const c_types::c_void) -> bool;

            #[allow(improper_ctypes)]
            #[allow(dead_code)]
            fn rust_helper_ptr_err(ptr: *const c_types::c_void) -> c_types::c_long;

            #[allow(improper_ctypes)]
            fn rust_helper_sock_net(sk: *const Sock) -> *mut Namespace;

            fn rust_helper_INIT_LIST_HEAD(ptr: *mut bindings::list_head);
        }

        let sk = unsafe { (*sock.get_ptr()).sk };
        /*
        * Try finding a suitable out-of-band protocol among those
        * registered in RROS.
        */
        let proto = find_oob_proto(
            sk_family!(sk).into(),
            unsafe { (*sk).sk_type }.into(),
            be16::new(unsafe { (*sk).sk_protocol }),
        );
        if proto.is_none() {
            return -(bindings::EPROTONOSUPPORT as i32);
        }
        let proto = proto.unwrap();

        let rsk = if sk_family!(sk) != bindings::PF_OOB as u16 {
            let tmp = c_kzalloc(core::mem::size_of::<RrosSocket>() as u64);
            if tmp.is_none() {
                return -(bindings::ENOMEM as i32);
            }
            tmp.unwrap() as *mut RrosSocket
        } else{
            sk as *const _ as *mut RrosSocket
        };
        let mut rsk = unsafe { NonNull::new(rsk).unwrap().as_mut() };
        rsk.sk = sk as *mut Sock;
        let ret = rsk.efile.rros_open_file(unsafe { (*sock.get_ptr()).file });
        if ret.is_err() {
            // TODO:
            unimplemented!();
        }
        rsk.net = unsafe { rust_helper_sock_net((*sock.get_ptr()).sk as *mut Sock as *const Sock) };
        let pinned = unsafe { Pin::new_unchecked(&mut rsk.lock) };
        mutex_init!(pinned, "net mutex");

        unsafe {
            rust_helper_INIT_LIST_HEAD(&mut rsk.input);
            rust_helper_INIT_LIST_HEAD(&mut rsk.next_sub);
            rsk.input_wait.init(&mut RROS_MONO_CLOCK, 0);
            rsk.wmem_wait.init(&mut RROS_MONO_CLOCK, 0);
        }
        // rros_init_poll_head(&esk->poll_head);
        let pinned = unsafe { Pin::new_unchecked(&mut rsk.oob_lock) };
        spinlock_init!(pinned, "net oob spinlock");

        rsk.rmem_max = unsafe { (*sk).sk_rcvbuf };
        rsk.wmem_max = unsafe { (*sk).sk_sndbuf };
        let _ret = rsk.wmem_drain.init();
        rsk.proto = Some(proto);
        proto.attach(&mut rsk, unsafe { be16::new((*sk).sk_protocol) });

        unsafe{
            (*sk).oob_data = rsk as *const _ as *mut c_types::c_void;
        }
        0
        //TODO: FAILED handling
        //     fail_attach:
        // 	rros_release_file(&esk->efile);
        // fail_open:
        // 	if (sk->sk_family != PF_OOB)
        // 		kfree(esk);

        // 	return ret;
    }
}

no_mangle_function_declaration! {
    fn sock_oob_detach(sock: Socket)
    {
        let sk = unsafe { (*sock.get_ptr()).sk };
        let rsk = unsafe { RrosSocket::from_socket(sock).as_mut() };

        let _ret = rsk.efile.rros_release_file();
        rsk.wmem_drain.pass();
        free_skb_list(&mut rsk.input);
        if let Some(proto) = rsk.proto {
            proto.detach(rsk);
        }

        if (sk_family!(sk) != bindings::PF_OOB as u16) {
            c_kzfree(rsk as *const _ as *const c_types::c_void);
        }
        unsafe { (*sk).oob_data = core::ptr::null_mut(); }
    }
}

no_mangle_function_declaration! {
    pub fn sock_oob_bind(sock: Socket, addr: *const Sockaddr, len: i32) -> i32
    {
        let sk = unsafe { (*sock.get_ptr()).sk };
        if sk_family!(sk) == bindings::PF_OOB as u16 {
            return 0;
        }
        let rsk = unsafe { RrosSocket::from_socket(sock).as_mut() };
        let proto = rsk.proto.unwrap();
        proto.bind(rsk, unsafe { &*addr }, len)
    }
}

pub fn sock_inband_ioctl(_sock: &mut Socket, _cmd: i32, _arg: u64) -> i32 {
    // TODO:
    -(bindings::ENOTTY as i32)
}

no_mangle_function_declaration! {
    pub fn sock_inband_ioctl_redirect(sock: Socket, cmd: i32, arg: u64) -> i32 {
        let ret = sock_inband_ioctl(&mut sock, cmd, arg);

        if ret == -(bindings::ENOTTY as i32) {
            -(bindings::ENOIOCTLCMD as i32)
        } else {
            ret
        }
    }
}

#[no_mangle]
pub fn sock_oob_ioctl(filp: *mut bindings::file, cmd: u32, arg: u64) -> i64 {
    let rsk = RrosSocket::from_file(filp);
    if rsk.is_none() {
        return -(bindings::EBADFD as i64);
    }
    let rsk = unsafe { rsk.unwrap().as_mut() };
    let ret: i32;

    match cmd {
        // TODO: update IOV
        RROS_SOCKIOC_RECVMSG | RROS_SOCKIOC_SENDMSG => {
            let u_msghdr: *mut UserOobMsghdr = unsafe { transmute(arg) };
            ret = rsk.send_or_recv(u_msghdr, cmd as i32);
        }
        _ => {
            ret = -(bindings::ENOTTY as i32);
        }
    }
    return ret as i64;
}

pub fn do_load_iov(iov: *mut Iovec, u_iov: *mut Iovec, iovlen: usize) -> Result<()> {
    extern "C" {
        fn rust_helper_raw_copy_from_user(dst: *mut u8, src: *const u8, size: usize) -> usize;
    }
    unsafe {
        if rust_helper_raw_copy_from_user(
            iov as *const _ as *mut u8,
            u_iov as *const _ as *mut u8,
            iovlen * core::mem::size_of::<Iovec>(),
        ) != 0
        {
            Err(Error::EFAULT)
        } else {
            Ok(())
        }
    }
}

pub fn load_iov(u_iov: *mut Iovec, iovlen: usize, fast_iov: *mut Iovec) -> Result<*mut Iovec> {
    assert!(iovlen <= 1024);
    if iovlen < 8 {
        do_load_iov(fast_iov, u_iov, iovlen)?;
        Ok(fast_iov)
    } else {
        Err(Error::EFAULT)
    }
}

pub fn rros_import_iov(
    iov_vec: &[Iovec],
    data: *mut u8,
    mut len: u64,
    remainder: Option<&mut usize>,
) -> i32 {
    extern "C" {
        fn rust_helper_raw_copy_from_user(to: *mut u8, from: *const u8, n: usize) -> usize;
    }
    let mut n = 0;
    let mut nbytes: u64;
    let mut avail = 0;
    let mut read = 0;
    for iov in iov_vec.iter() {
        if iov.get_iov_len() == 0 {
            n += 1;
            continue;
        }
        nbytes = iov.get_iov_len();
        avail += nbytes;
        if nbytes > len {
            nbytes = len;
        }
        let ret = unsafe {
            rust_helper_raw_copy_from_user(data, iov.get_iov_base() as *const u8, nbytes as usize)
        };
        if ret != 0 {
            return -(bindings::EFAULT as i32);
        }

        len -= nbytes;
        unsafe { data.offset(nbytes as isize) };
        read += nbytes;
        #[allow(unused_comparisons)]
        if read < 0 {
            return -(bindings::EFAULT as i32);
        }
        n += 1;
        if len == 0 {
            break;
        }
    }

    if let Some(remainder) = remainder {
        for iov_idx in n..iov_vec.len() {
            let iov = &iov_vec[iov_idx];
            avail += iov.get_iov_len();
        }
        *remainder = (avail - read) as usize;
    }
    return read as i32;
}

pub fn rros_export_iov(iov_vec: &mut [Iovec], mut data: *mut u8, len: usize) -> i32 {
    extern "C" {
        fn rust_helper_raw_copy_to_user(to: *mut u8, from: *const u8, n: usize) -> usize;
    }
    let mut written = 0;
    let mut len = len as u64;
    for iov in iov_vec.iter_mut() {
        if iov.get_iov_len() == 0 {
            continue;
        }
        let mut nbytes = iov.get_iov_len();
        if nbytes > len {
            nbytes = len;
        }
        let ret = unsafe {
            rust_helper_raw_copy_to_user(
                iov.get_iov_base() as *const _ as *mut u8,
                data,
                nbytes as usize,
            )
        };
        if ret != 0 {
            return -(bindings::EFAULT as i32);
        }
        len -= nbytes;
        data = unsafe { data.offset(nbytes as isize) };
        written += nbytes;
        #[allow(unused_comparisons)]
        if written < 0 {
            return -(bindings::EINVAL as i32);
        }
    }
    return written as i32;
}
// #[no_mangle]
// fn destroy_rros_socket(sk: *mut bindings::sock)
// {
//     // extern "C" {
//     //     fn rust_helper_sock_net(sk:*const bindings::sock)->*mut bindings::net;
//     // }
// 	// bindings::local_bh_disable();
// 	// unsafe{bindings::sock_prot_inuse_add(rust_helper_sock_net(sk),sk_prot!(sk), -1 as c_types::c_int)};
// 	// bindings::local_bh_enable();
//     // sk_refcnt_debug_dec(sk); // TODO: Not implemented, used for debugging in rros.
// }

// // TODO: What format is used for strings?
// #[no_mangle]
// fn sock_oob_read(flip:File, buf:*mut u8, count:usize, pos:off_t) -> SsizeT
// {
//     let rsk = rros_sk_from_file(flip);
//     if rsk.is_none(){
//         return -(bindings::EBADF as i32);
//     }
//     if count == 0{
//         return 0;
//     }
//     let iov : bindings::iovec;
//     unsafe{
//         iov.iov_base = buf;
//         iov.iov_len = count as u64;
//     }
//     let rsk = rsk.unwrap();
//     let proto = rsk.proto.as_mut().unwrap();
//     // TODO: Interface
//     proto.oob_receive(rsk,proto,core::ptr::null(),&iov as *const bindings::iovec,1 as c_types::c_int)
// }

// #[no_mangle]
// fn sock_oob_write(flip:File, buf:*mut u8, count:usize, pos:off_t) -> SsizeT
// {
//     let rsk = rros_sk_from_file(filp);
//     if rsk.is_none(){
//         return -(bindings::EBADF as i32);
//     }
//     if count == 0{
//         return 0;
//     }
//     let iov : bindings::iovec;
//     unsafe{
//         iov.iov_base = buf;
//         iov.iov_len = count as u64;
//     }
//     let proto = rsk.proto.as_mut();
//     // TODO: Interface `oob_send`.
//     proto.oob_send()
// }

// //
// // Domain management
// //
// init_static_sync!(
//     static mut DOMAIN_LOCK : Mutex<()> = Mutex::new();
//     domain_hash:domian_list_head[256]
// );

// pub struct rros_socket_domain {
//     pub af_domain: i32,
//     pub match_: fn(type_:i32, protocol :bindings::__be16) -> impl RrosNetProto,
//     pub next : *mut list_head
// }

// struct domian_list_head{
//     af_domain: i32,
//     hkey : u32,
//     hash : *mut hlist_node,
//     list : *mut list_head
// }

// initialize_lock_hashtable!(domain_hash, 8);

// struct DomainListHead{
//     af_domain: i32,
//     hkey : u32,
//     hash : bindings::hlist_node,
//     list :
// }

// #[inline]
// fn get_domain_hash(af_domain : i32) -> u32{
//     /// Calculate protocol hash.
//     let hsrc : u32 = af_domain as u32;
//     extern "C"{
//         fn rust_helper_jhash2(k :*const u32, length:u32, initval:u32) -> u32;
//     }
//     unsafe{
//         rust_helper_jhash2(&hsrc as *const u32, 1u32, 0u32)
//     }
// }

// fn fetch_domain_list(hkey : u32) -> {

// }

fn find_oob_proto(
    _domain: i32,
    _type_: i32,
    _protocol: be16,
) -> Option<&'static impl RrosNetProto> {
    // TODO: Support more protocols.
    // let hkey = get_domain_hash(domain);
    // let gurad = domain_hash.lock();
    // drop(gurad);

    Some(&ETHERNET_NET_PROTO)
}

// #[no_mangle]
// fn rros_register_socket_domain(domain : rros_socket_domain){
// 	inband_context_only();

//     let hkey = get_domain_hash(domain.af_domain);
//     let mut lock = DOMAIN_LOCK.lock();
//     let mut head = fetch_domain_list(hkey);
//     drop(lock);
// }

// /**
//  * Tools function
//  */
// #[inline]
// #[no_mangle]
// fn rros_sk_from_file(flip:File) -> Option<RrosSocket>{
// 	unsafe {
// 		let data = (*flip.ptr).oob_data;
//         if data{
//             Some(kernel::container_of!(data,RrosSocket,efile))
//         }else{
//             None
//         }
// 	}

// }

pub static RROS_FAMILY_OPS: NetProtoFamily = static_init_net_proto_family!(
    family: bindings::PF_OOB as i32,
    create: Some(create_socket_callback::<CreateRrosSocket>),
    owner: THIS_MODULE.get_ptr(),
);

// pub static mut rros_af_oob_proto : bindings::proto  = bindings::proto::default();

// pub fn init_rros_af_oob_proto(){

//     unsafe{
//         rros_af_oob_proto.name = (*b"RROS\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0").map(|u| u as i8);
//         rros_af_oob_proto.owner = &kernel::bindings::__this_module as *const _ as *mut _;
//         rros_af_oob_proto.obj_size = core::mem::size_of::<RrosSocket>() as u32;
//     }
// }

pub struct DummpyProto;
#[allow(dead_code)]
static DUMMY_PROTO: DummpyProto = DummpyProto;

impl RrosNetProto for DummpyProto {
    fn attach(&self, _sock: &mut RrosSocket, _protocol: be16) -> i32 {
        0
    }

    fn detach(&self, _sock: &mut RrosSocket) {}

    fn bind(&self, _sock: &mut RrosSocket, _addr: &Sockaddr, _len: i32) -> i32 {
        0
    }

    // fn ioctl(&mut self, cmd:u32, arg:u64) -> i32;

    fn oob_send(
        &self,
        _sock: &mut RrosSocket,
        _msg: *mut UserOobMsghdr,
        _iov_vec: &mut [Iovec],
    ) -> isize {
        0
    }

    fn oob_receive(
        &self,
        _sock: &mut RrosSocket,
        _msg: *mut UserOobMsghdr,
        _iov_vec: &mut [Iovec],
    ) -> isize {
        0
    }

    fn get_netif(&self, _sock: &mut RrosSocket) -> Option<NetDevice> {
        unimplemented!()
    }
}
