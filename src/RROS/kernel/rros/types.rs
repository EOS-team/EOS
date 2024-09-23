use kernel::bindings;

/// Unofficial Wrap for some binding struct.
/// Now are mostly used in net module.
/// Here's a simple list:
/// * macro for list_head

/// List macro and method

/// Macro to get the list entry from a given pointer.
/// This macro takes a pointer, a type, and a field, and returns a pointer to the entry containing the list head.
#[macro_export]
macro_rules! list_entry {
    ($ptr:expr, $type:ty, $($f:tt)*) => {
        unsafe{kernel::container_of!($ptr, $type, $($f)*) as *mut $type}
    }
}

/// Macro to get the first entry from a list.
/// This macro takes a pointer to the list head, a type, and a field, and returns a pointer to the first entry in the list.
#[macro_export]
macro_rules! list_first_entry {
    ($ptr:expr, $type:ty, $($f:tt)*) => {
        list_entry!((*$ptr).next, $type, $($f)*)
    }
}

/// Macro to get the last entry from a list.
/// This macro takes a pointer to the list head, a type, and a field, and returns a pointer to the last entry in the list.
#[macro_export]
macro_rules! list_last_entry {
    ($ptr:expr, $type:ty, $($f:tt)*) => {
        list_entry!((*$ptr).prev, $type, $($f)*)
    }
}

/// Macro to get the next entry from a list.
/// This macro takes a pointer to a list entry, a type, and a field, and returns a pointer to the next entry in the list.
#[macro_export]
macro_rules! list_next_entry {
    ($pos:expr, $type:ty,$($f:tt)*) => {
        list_entry!(((*$pos).$($f)*).next, $type, $($f)*)
    }
}

/// Macro to get the previous entry from a list.
/// This macro takes a pointer to a list entry, a type, and a field, and returns a pointer to the previous entry in the list.
#[macro_export]
macro_rules! list_prev_entry {
    ($pos:expr, $type:ty,$($f:tt)*) => {
        list_entry!(((*$pos).$($f)*).prev, $type, $($f)*)
    }
}

/// Macro to check if a list entry is the head of the list.
/// This macro takes a pointer to a list entry and a pointer to the list head, and returns true if the entry is the head of the list.
#[macro_export]
macro_rules! list_entry_is_head {
    ($pos:expr,$head:expr,$($f:tt)*) => {
        unsafe{
            core::ptr::eq(&(*$pos).$($f)*,$head)
        }
    };
}

/// Macro to initialize a list head.
/// This macro takes a pointer to a list head and initializes it.
/// It uses the `rust_helper_INIT_LIST_HEAD` function from the C bindings.
#[macro_export]
macro_rules! init_list_head {
    ($list:expr) => {{
        extern "C" {
            #[allow(dead_code)]
            fn rust_helper_INIT_LIST_HEAD(list: *mut $crate::bindings::list_head);
        }
        unsafe {
            rust_helper_INIT_LIST_HEAD($list as *mut $crate::bindings::list_head);
        }
    }};
}

/// Function to check if a list is empty.
/// This function takes a pointer to a list head and returns true if the list is empty.
#[inline]
pub fn list_empty(list: *const bindings::list_head) -> bool {
    unsafe { (*list).next as *const bindings::list_head == list }
}

/// Macro to check if a list is empty.
/// This macro takes a pointer to a list head and returns true if the list is empty.
/// It uses the `rust_helper_list_empty` function from the C bindings.
#[macro_export]
macro_rules! list_empty {
    ($list_head_ptr:expr) => {{
        extern "C" {
            fn rust_helper_list_empty(list: *const $crate::bindings::list_head) -> bool;
        }
        unsafe { rust_helper_list_empty($list_head_ptr as *const $crate::bindings::list_head) }
    }};
}

/// Macro to delete a list entry.
/// This macro takes a pointer to a list entry and removes it from the list.
/// It uses the `rust_helper_list_del` function from the C bindings.
#[macro_export]
macro_rules! list_del {
    ($list:expr) => {
        extern "C" {
            #[allow(dead_code)]
            fn rust_helper_list_del(list: *mut $crate::bindings::list_head);
        }
        unsafe {
            rust_helper_list_del($list as *mut $crate::bindings::list_head);
        }
    };
}

/// Macro to delete a list entry and reinitialize it.
/// This macro takes a pointer to a list entry, removes it from the list, and reinitializes it.
/// It uses the `rust_helper_list_del_init` function from the C bindings.
#[macro_export]
macro_rules! list_del_init {
    ($list:expr) => {
        extern "C" {
            fn rust_helper_list_del_init(list: *mut $crate::bindings::list_head);
        }
        unsafe {
            rust_helper_list_del_init($list as *mut $crate::bindings::list_head);
        }
    };
}

/// Macro to get the first entry from a list and remove it from the list.
/// This macro takes a pointer to the list head, a type, and a field, and returns a pointer to the first entry in the list.
/// The entry is also removed from the list.
#[macro_export]
macro_rules! list_get_entry{
    ($head:expr,$type:ty,$($f:tt)*) => {
        {
            let item = $crate::list_first_entry!($head,$type,$($f)*);
            $crate::list_del!(&mut (*item).$($f)*);
            item
        }
    };
}

/// Macro to add a new entry at the end of the list.
/// This macro takes a new entry and a list head, and adds the new entry at the end of the list.
/// It uses the `rust_helper_list_add_tail` function from the C bindings.
#[macro_export]
macro_rules! list_add_tail {
    ($new:expr,$head:expr) => {
        extern "C" {
            fn rust_helper_list_add_tail(
                new: *mut $crate::bindings::list_head,
                head: *mut $crate::bindings::list_head,
            );
        }
        unsafe {
            rust_helper_list_add_tail(
                $new as *mut $crate::bindings::list_head,
                $head as *mut $crate::bindings::list_head,
            );
        }
    };
}

/// Macro to add a new entry at the beginning of the list.
/// This macro takes a new entry and a list head, and adds the new entry at the beginning of the list.
/// It uses the `rust_helper_list_add` function from the C bindings.
#[macro_export]
macro_rules! list_add {
    ($new:expr,$head:expr) => {
        extern "C" {
            fn rust_helper_list_add(
                new: *mut $crate::bindings::list_head,
                head: *mut $crate::bindings::list_head,
            );
        }
        unsafe {
            rust_helper_list_add(
                $new as *mut $crate::bindings::list_head,
                $head as *mut $crate::bindings::list_head,
            );
        }
    };
}

/// Macro to iterate over a list of given type.
/// This macro takes an identifier for the loop cursor, a list head, a type, a block of code to execute for each entry, and a list of fields.
/// It starts from the first entry and continues until it reaches the head of the list, executing the block of code for each entry.
#[macro_export]
macro_rules! list_for_each_entry{
    ($pos:ident,$head:expr,$type:ty,$e:block,$($f:tt)*) => {
        let mut $pos = list_first_entry!($head,$type,$($f)*);
        while !list_entry_is_head!($pos,$head,$($f)*){
            $e;
            $pos = list_next_entry!($pos,$type,$($f)*);
        }
    };
}

/// Macro to safely iterate over a list of given type.
/// This macro takes two identifiers for the loop cursor and a temporary storage, a list head, a type, a block of code to execute for each entry, and a list of fields.
/// It starts from the first entry and continues until it reaches the head of the list, executing the block of code for each entry.
/// The temporary storage is used to hold the next entry in the list, allowing safe removal of the current entry during iteration.
#[macro_export]
macro_rules! list_for_each_entry_safe {
    ($pos:ident,$n:ident,$head:expr,$type:ty,$e:block,$($f:tt)*) => {
        let mut $pos = list_first_entry!($head,$type,$($f)*);
        let mut $n = list_next_entry!($pos,$type,$($f)*);
        while !list_entry_is_head!($pos,$head,$($f)*){
            $e;
            $pos = $n;
            $n = list_next_entry!($n,$type,$($f)*);
        }
    };
}

/// Macro to iterate over a list of given type in reverse order.
/// This macro takes an identifier for the loop cursor, a list head, a type, a block of code to execute for each entry, and a list of fields.
/// It starts from the last entry and continues until it reaches the head of the list, executing the block of code for each entry.
#[macro_export]
macro_rules! list_for_each_entry_reverse {
    ($pos:ident,$head:expr,$type:ty,$e:block,$($f:tt)*) => {
        let mut $pos = list_last_entry!($head,$type,$($f)*);
        while !list_entry_is_head!($pos,$head,$($f)*){
            $e;
            $pos = list_prev_entry!($pos,$type,$($f)*);
        }
    };
}

/// Macro to add a new entry to a list in a priority-first manner.
/// This macro takes a new entry, a list head, two identifiers for the priority and next fields, and a type.
/// It adds the new entry to the list in such a way that the entries are sorted in descending order of their priority.
/// If the list is empty, the new entry is added at the beginning of the list.
/// Otherwise, the macro iterates over the list in reverse order and inserts the new entry before the first entry that has a lower or equal priority.
/// The `rust_helper_list_add` function from the C bindings is used to add the new entry to the list.
#[macro_export]
macro_rules! list_add_priff {
    ($new:expr,$head:expr, $member_pri:ident,$member_next:ident,$tp:ty) => {{
        extern "C" {
            fn rust_helper_list_add(
                new: *mut $crate::bindings::list_head,
                head: *mut $crate::bindings::list_head,
            );
        }
        let next = (*$head).next;
        if core::ptr::eq(next, $head) {
            unsafe {
                rust_helper_list_add(
                    &mut (*$new).$member_next as *mut $crate::bindings::list_head,
                    $head as *mut $crate::bindings::list_head,
                )
            };
        } else {
            let mut _pos: *mut $tp;
            $crate::list_for_each_entry_reverse!(
                pos,
                $head,
                $tp,
                {
                    if (*$new).$member_pri <= unsafe { (*pos).$member_pri } {
                        break;
                    }
                },
                $member_next
            );
            unsafe {
                rust_helper_list_add(
                    &mut (*$new).$member_next as *mut $crate::bindings::list_head,
                    &mut (*pos).$member_next as *mut $crate::bindings::list_head,
                )
            };
        }
    }};
}

/// Macro to calculate the number of long integers needed to represent a given number of bits.
/// This macro takes a number of bits and returns the number of long integers needed to represent that many bits.
#[macro_export]
macro_rules! bits_to_long {
    ($bits:expr) => {
        ((($bits) + (64) - 1) / (64))
    };
}

/// Macro to declare a bitmap.
/// This macro takes an identifier for the bitmap and a number of bits.
/// It declares a static mutable array of long integers with the given identifier and size calculated from the number of bits.
#[macro_export]
macro_rules! DECLARE_BITMAP {
    ($name:ident,$bits:expr) => {
        static mut $name: [usize; bits_to_long!($bits)] = [0; bits_to_long!($bits)];
    };
}

// pub struct HardSpinlock<T>{
//     spin_lock: Opaque<bindings::spinlock>,
//     flags:usize,
//     _pin: PhantomPinned,
//     data: UnsafeCell<T>,
// }

// unsafe impl Sync for HardSpinlock {}
// unsafe impl Send for HardSpinlock {}

// impl<T> HardSpinlock<T>{
//     pub fn new(data:T) -> Self{
//         extern "C"{
//             fn rust_helper_raw_spin_lock_init(lock:*mut bindings::spinlock_t);
//         }
//         let t = bindings::hard_spinlock_t::default();
//         unsafe{
//             rust_helper_raw_spin_lock_init(&t as *const _ as *mut bindings::spinlock_t);
//         }
//         Self{
//             spin_lock : Opaque(t),
//             flags:0,
//             _pin:PhantomPinned,
//             data:UnsafeCell::new(data),
//         }
//     }

//     pub fn lock(&mut self) -> usize{
//         unsafe{
//             _raw_spin_lock_irqsave(&mut self.0 as *const _ as *mut bindings::raw_spinlock_t) as usize
//         }
//     }

//     pub fn unlock(&mut self,flags:usize){
//         unsafe{
//             _raw_spin_unlock_irqrestore(&mut self.0 as *const _ as *mut bindings::raw_spinlock_t,flags as c_ulong);
//         }
//     }
// }
