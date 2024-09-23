use crate::list::*;
use crate::{list_entry, list_first_entry, list_last_entry};
use kernel::prelude::*;
#[allow(dead_code)]
struct ListTest {
    num: i32,
    head: ListHead,
}

impl ListTest {
    #[allow(dead_code)]
    fn new(num: i32, head: ListHead) -> ListTest {
        ListTest { num, head }
    }
}

#[allow(dead_code)]
fn traverse_list(head: &ListHead) -> i32 {
    let mut count = 0;
    if head as *const ListHead == 0 as *const ListHead {
    } else if head.is_empty() {
        count = count + 1;
    } else {
        count = count + 1;
        let mut p: *mut ListHead = head.next;
        while p as *const ListHead != head as *const ListHead {
            count = count + 1;
            unsafe {
                p = (*p).next;
            }
        }
    }
    //pr_debug!("list count is {}",count);
    return count;
}

#[allow(dead_code)]
fn test_list_method() {
    let mut head = ListHead::default();
    let mut t1 = ListHead::default();
    let mut t2 = ListHead::default();

    // Test add.
    head.add(&mut t1 as *mut ListHead);
    head.add(&mut t2 as *mut ListHead);
    if traverse_list(&head) == 3 {
        pr_debug!("test_list_add success");
    } else {
        pr_debug!("test_list_add failed");
    }

    // Test list_drop.
    unsafe {
        (*head.next).list_drop();
    }
    //head.next = &mut t2 as *mut ListHead;
    if traverse_list(&head) == 2 {
        pr_debug!("test_list_drop success");
    } else {
        pr_debug!("test_list_drop failed");
    }

    // Test last_is.
    if head.last_is(&mut t1 as *mut ListHead) {
        pr_debug!("test_list_last_is success");
    } else {
        pr_debug!("test_list_last_is failed");
    }

    unsafe {
        (*head.next).list_drop();
    }

    // Test empty.
    if head.is_empty() {
        pr_debug!("test_list_is_empty success");
    } else {
        pr_debug!("test_list_is_empty failed");
    }
}

#[allow(dead_code)]
pub fn test_entry() {
    let mut t1 = ListTest::new(111, ListHead::default());
    let mut t2 = ListTest::new(222, ListHead::default());
    let mut t3 = ListTest::new(333, ListHead::default());
    t1.head.add(&mut t2.head as *mut ListHead);
    t1.head.add(&mut t3.head as *mut ListHead);
    let _t1 = list_entry!(&mut t1.head as *mut ListHead, ListTest, head);
    unsafe {
        if (*_t1).num == t1.num {
            pr_debug!("test_list_entry success!");
        } else {
            pr_debug!("test_list_entry failed!");
        }
    }
    let _t2 = list_first_entry!(t1.head.next, ListTest, head);
    unsafe {
        if (*_t2).num == t2.num {
            pr_debug!("test_list_first_entry success!");
        } else {
            pr_debug!("test_list_first_entry failed!");
        }
    }
    let _t3 = list_last_entry!(t1.head.prev, ListTest, head);
    unsafe {
        if (*_t3).num == t3.num {
            pr_debug!("test_list_last_entry success!");
        } else {
            pr_debug!("test_list_last_entry failed!");
        }
    }
}

#[allow(dead_code)]
pub fn test_list() {
    test_list_method();
    test_entry();
}
