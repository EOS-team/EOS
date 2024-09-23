// SPDX-License-Identifier: GPL-2.0

//! Rros Net module.
use crate::{bindings, c_types, str::CStr, ARef, AlwaysRefCounted};
use core::{cell::UnsafeCell, ptr::NonNull};

extern "C" {
    #[allow(improper_ctypes)]
    fn rust_helper_dev_hold(dev: *mut bindings::net_device) -> ();
    #[allow(improper_ctypes)]
    fn rust_helper_dev_put(dev: *mut bindings::net_device) -> ();
    #[allow(improper_ctypes)]
    fn rust_helper_get_net(net: *mut bindings::net) -> *mut bindings::net;
    #[allow(improper_ctypes)]
    fn rust_helper_put_net(net: *mut bindings::net) -> ();
}

/// Wraps the kernel's `struct net_device`.
#[repr(transparent)]
pub struct Device(UnsafeCell<bindings::net_device>);

// SAFETY: Instances of `Device` are created on the C side. They are always refcounted.
unsafe impl AlwaysRefCounted for Device {
    fn inc_ref(&self) {
        // SAFETY: The existence of a shared reference means that the refcount is nonzero.
        unsafe { rust_helper_dev_hold(self.0.get()) };
    }

    unsafe fn dec_ref(obj: core::ptr::NonNull<Self>) {
        // SAFETY: The safety requirements guarantee that the refcount is nonzero.
        unsafe { rust_helper_dev_put(obj.cast().as_ptr()) };
    }
}

/// Wraps the kernel's `struct net`.
#[repr(transparent)]
pub struct Namespace(UnsafeCell<bindings::net>);

impl Namespace {
    /// Finds a network device with the given name in the namespace.
    pub fn dev_get_by_name(&self, name: &CStr) -> Option<ARef<Device>> {
        // SAFETY: The existence of a shared reference guarantees the refcount is nonzero.
        let ptr =
            NonNull::new(unsafe { bindings::dev_get_by_name(self.0.get(), name.as_char_ptr()) })?;
        Some(unsafe { ARef::from_raw(ptr.cast()) })
    }
}

// SAFETY: Instances of `Namespace` are created on the C side. They are always refcounted.
unsafe impl AlwaysRefCounted for Namespace {
    fn inc_ref(&self) {
        // SAFETY: The existence of a shared reference means that the refcount is nonzero.
        unsafe { rust_helper_get_net(self.0.get()) };
    }

    unsafe fn dec_ref(obj: core::ptr::NonNull<Self>) {
        // SAFETY: The safety requirements guarantee that the refcount is nonzero.
        unsafe { rust_helper_put_net(obj.cast().as_ptr()) };
    }
}

/// Returns the network namespace for the `init` process.
pub fn init_ns() -> &'static Namespace {
    unsafe { &*core::ptr::addr_of!(bindings::init_net).cast() }
}

/// A trait to implement function for Namespace.
pub trait CreateSocket {
    /// A function to create socket customized.
    fn create_socket(net: *mut Namespace, sock: &mut Socket, protocol: i32, kern: i32) -> i32;
}

/// Callback function for trait `CreateSocket`.
pub unsafe extern "C" fn create_socket_callback<T: CreateSocket>(
    net: *mut bindings::net,
    sock: *mut bindings::socket,
    protocol: i32,
    kern: i32,
) -> i32 {
    T::create_socket(
        net as *mut Namespace,
        &mut Socket::from_ptr(sock),
        protocol,
        kern,
    )
}

/// The `sock_register` function is a wrapper around the `bindings::sock_register` function from the kernel bindings.
pub fn sock_register(fam: *const bindings::net_proto_family) -> c_types::c_int {
    unsafe { bindings::sock_register(fam) }
}

/// The `NetProtoFamily` struct wraps a `bindings::net_proto_family` struct from the kernel bindings.
pub struct NetProtoFamily(pub bindings::net_proto_family);

/// Initialize a `NetProtoFamily` struct.
#[macro_export]
macro_rules! static_init_net_proto_family {
    ($($i: ident: $e: expr,)*) => {
        NetProtoFamily(bindings::net_proto_family{
            $($i: $e,)*
        })
    }
}

impl NetProtoFamily {
    /// Returns a pointer to inner struct.
    pub fn get_ptr(&self) -> *const bindings::net_proto_family {
        &self.0
    }

    /// A wrapper around the `bindings::net_proto_family` function from the kernel bindings. Constructs a new struct.
    pub fn new(
        family: c_types::c_int,
        create: Option<
            unsafe extern "C" fn(
                net: *mut bindings::net,
                sock: *mut bindings::socket,
                protocol: c_types::c_int,
                kern: c_types::c_int,
            ) -> c_types::c_int,
        >,
        owner: *mut bindings::module,
    ) -> Self {
        Self(bindings::net_proto_family {
            family,
            create,
            owner,
        })
    }
}

unsafe impl Sync for NetProtoFamily {}

/// The `Socket` struct wraps a pointer to a `bindings::socket` from the kernel bindings.
pub struct Socket {
    ptr: *mut bindings::socket,
}

impl Socket {
    /// Constructs a new struct with a pointer to `bindings::socket`.
    pub fn from_ptr(ptr: *mut bindings::socket) -> Self {
        Self { ptr }
    }

    /// Returns self's ptr.
    pub fn get_ptr(&self) -> *mut bindings::socket {
        self.ptr
    }
}
