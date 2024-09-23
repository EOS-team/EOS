use core::ptr::NonNull;

use kernel::{
    hash_for_each_possible, initialize_lock_hashtable,
    linked_list::{GetLinks, Links, List},
    pr_debug,
    prelude::*,
    types::*,
};

#[allow(dead_code)]
struct HashEntry {
    pub data: u32,
    pub hash: HlistNode,
}

impl HashEntry {
    #[allow(dead_code)]
    pub fn new(val: u32) -> Self {
        HashEntry {
            data: val,
            hash: HlistNode::new(),
        }
    }
}

initialize_lock_hashtable!(TABLE, 256);

#[allow(dead_code)]
fn test_hashtable() {
    let mut result = [0u32; 10];
    let mut entry1 = HashEntry::new(1);
    let mut entry2 = HashEntry::new(2);
    let mut entry3 = HashEntry::new(3);
    let mut table_guard = TABLE.lock();
    table_guard.add(&mut entry1.hash.0, 1);
    table_guard.add(&mut entry2.hash.0, 1);
    table_guard.add(&mut entry3.hash.0, 1);
    let mut counter = 0;
    hash_for_each_possible!(cur, table_guard.head(1), HashEntry, hash, {
        result[counter] = unsafe { (*cur).data };
        counter += 1;
    });
    assert_eq!(result[0], 3);
    assert_eq!(result[1], 2);
    assert_eq!(result[2], 1);
    table_guard.del(&mut entry2.hash.0);

    counter = 0;
    hash_for_each_possible!(cur, table_guard.head(1), HashEntry, hash, {
        result[counter] = unsafe { (*cur).data };
        counter += 1;
    });
    assert_eq!(result[0], 3);
    assert_eq!(result[1], 1);

    pr_debug!("hash table test ok.");
}

#[allow(dead_code)]
struct ListTest2 {
    num: i32,
    head: kernel::bindings::list_head,
}

impl ListTest2 {
    #[allow(dead_code)]
    fn new(num: i32) -> ListTest2 {
        ListTest2 {
            num,
            head: kernel::bindings::list_head::default(),
        }
    }
    #[allow(dead_code)]
    fn init_list_head(&mut self) {
        init_list_head!(&mut self.head);
    }
}

#[allow(dead_code)]
pub fn test_list_macro() {
    extern "C" {
        fn rust_helper_list_add_tail(
            new: *mut kernel::bindings::list_head,
            head: *mut kernel::bindings::list_head,
        );
        fn rust_helper_list_del_init(list: *mut kernel::bindings::list_head);
    }
    let mut t1 = ListTest2::new(1);
    let mut t2 = ListTest2::new(2);
    let mut t3 = ListTest2::new(3);
    t1.init_list_head();
    t2.init_list_head();
    t3.init_list_head();

    let mut head = kernel::bindings::list_head::default();
    init_list_head!(&mut head);
    unsafe {
        rust_helper_list_add_tail(&mut t1.head, &mut head);
        rust_helper_list_add_tail(&mut t2.head, &mut head);
        rust_helper_list_add_tail(&mut t3.head, &mut head);
    }
    let x = list_prev_entry!(&mut t2, ListTest2, head);
    assert_eq!(unsafe { (*x).num }, 1);
    let x = list_next_entry!(&mut t1, ListTest2, head);
    assert_eq!(unsafe { (*x).num }, 2);

    let mut array = [0; 10];
    let mut counter = 0;
    list_for_each_entry!(
        cur,
        &head,
        ListTest2,
        {
            // unsafe{
            //     pr_debug!("cur num is {}",(*cur).num);
            // }
            array[counter] = unsafe { (*cur).num };
            counter += 1;
        },
        head
    );
    assert_eq!(counter, 3);
    assert_eq!(array[0], 1);
    assert_eq!(array[1], 2);
    assert_eq!(array[2], 3);

    counter = 0;
    list_for_each_entry_reverse!(
        cur,
        &head,
        ListTest2,
        {
            // unsafe{
            //     pr_debug!("cur num is {}",(*cur).num);
            // }
            array[counter] = unsafe { (*cur).num };
            counter += 1;
        },
        head
    );
    assert_eq!(counter, 3);
    assert_eq!(array[0], 3);
    assert_eq!(array[1], 2);
    assert_eq!(array[2], 1);

    list_for_each_entry_safe!(
        cur,
        next,
        &head,
        ListTest2,
        {
            unsafe {
                // pr_debug!("cur num is {}",(*cur).num);
                if (*cur).num == 2 {
                    rust_helper_list_del_init(&mut (*cur).head);
                }
            }
        },
        head
    );

    counter = 0;
    list_for_each_entry!(
        cur,
        &head,
        ListTest2,
        {
            // unsafe{
            //     pr_debug!("cur num is {}",(*cur).num);
            // }
            array[counter] = unsafe { (*cur).num };
            counter += 1;
        },
        head
    );
    assert_eq!(counter, 2);
    assert_eq!(array[0], 1);
    assert_eq!(array[1], 3);

    counter = 0;
    list_for_each_entry_reverse!(
        cur,
        &head,
        ListTest2,
        {
            array[counter] = unsafe { (*cur).num };
            counter += 1;
        },
        head
    );
    assert_eq!(counter, 2);
    assert_eq!(array[0], 3);
    assert_eq!(array[2], 1);

    pr_debug!("list_for_each ok!. ")
}

// struct KernelListExample{
//     num: i32,
//     head: Links<KernelListExample>
// }

// impl KernelListExample{
//     fn new(num: i32) -> KernelListExample{
//         KernelListExample{
//             num,
//             head: Links::new()
//         }
//     }
// }

// impl GetLinks for KernelListExample{
//     type EntryType = KernelListExample;
//     fn get_links(data: &Self::EntryType) -> &Links<Self::EntryType>{
//         &data.head
//     }
// }

// pub fn test_kernel_list(){
//     let mut list: List<&KernelListExample> = List::new();
//     let data1 = KernelListExample::new(5);
//     let data2 = KernelListExample::new(6);
//     let data3 = KernelListExample::new(7);

//     list.push_back(&data1);
//     list.push_back(&data2);
//     list.push_back(&data3);

// }

#[allow(dead_code)]
pub fn test_rros_list_add_priff() {
    extern "C" {
        fn rust_helper_INIT_LIST_HEAD(list: *mut kernel::bindings::list_head);
    }
    let mut t1 = ListTest2::new(1);
    let mut t2 = ListTest2::new(2);
    let mut t3 = ListTest2::new(3);
    let mut head = kernel::bindings::list_head::default();

    // test 1
    t1.init_list_head();
    t2.init_list_head();
    t3.init_list_head();
    unsafe {
        rust_helper_INIT_LIST_HEAD(&mut head as *mut kernel::bindings::list_head);
    }

    list_add_priff!(&mut t3, &mut head, num, head, ListTest2);
    list_add_priff!(&mut t2, &mut head, num, head, ListTest2);
    list_add_priff!(&mut t1, &mut head, num, head, ListTest2);

    let mut array = [0; 10];
    let mut counter = 0;
    list_for_each_entry!(
        cur,
        &head,
        ListTest2,
        {
            array[counter] = unsafe { (*cur).num };
            counter += 1;
        },
        head
    );
    assert_eq!(counter, 3);
    assert_eq!(array[0], 3);
    assert_eq!(array[1], 2);
    assert_eq!(array[2], 1);

    // test2
    t1.init_list_head();
    t2.init_list_head();
    t3.init_list_head();
    unsafe {
        rust_helper_INIT_LIST_HEAD(&mut head as *mut kernel::bindings::list_head);
    }

    list_add_priff!(&mut t2, &mut head, num, head, ListTest2);
    list_add_priff!(&mut t3, &mut head, num, head, ListTest2);
    list_add_priff!(&mut t1, &mut head, num, head, ListTest2);

    let mut array = [0; 10];
    let mut counter = 0;
    list_for_each_entry!(
        cur,
        &head,
        ListTest2,
        {
            array[counter] = unsafe { (*cur).num };
            counter += 1;
        },
        head
    );
    assert_eq!(counter, 3);
    assert_eq!(array[0], 3);
    assert_eq!(array[1], 2);
    assert_eq!(array[2], 1);

    // test3
    t1.init_list_head();
    t2.init_list_head();
    t3.init_list_head();
    unsafe {
        rust_helper_INIT_LIST_HEAD(&mut head as *mut kernel::bindings::list_head);
    }

    list_add_priff!(&mut t1, &mut head, num, head, ListTest2);
    list_add_priff!(&mut t3, &mut head, num, head, ListTest2);
    list_add_priff!(&mut t2, &mut head, num, head, ListTest2);

    let mut array = [0; 10];
    let mut counter = 0;
    list_for_each_entry!(
        cur,
        &head,
        ListTest2,
        {
            array[counter] = unsafe { (*cur).num };
            counter += 1;
        },
        head
    );
    assert_eq!(counter, 3);
    assert_eq!(array[0], 3);
    assert_eq!(array[1], 2);
    assert_eq!(array[2], 1);

    pr_debug!("test list_add_priff(rros priority function) ok!")
}

struct KernelListTest {
    #[allow(dead_code)]
    pub prio: i32,
    head: Links<KernelListTest>,
}
impl GetLinks for KernelListTest {
    type EntryType = KernelListTest;
    fn get_links(data: &Self::EntryType) -> &Links<Self::EntryType> {
        &data.head
    }
}

impl KernelListTest {
    #[allow(dead_code)]
    fn new(prio: i32) -> KernelListTest {
        KernelListTest {
            prio,
            head: Links::new(),
        }
    }
}

#[allow(dead_code)]
fn add_by_prio(list: &mut List<Box<KernelListTest>>, item: Box<KernelListTest>) {
    if list.is_empty() {
        list.push_back(item);
        return;
    } else {
        let mut last = list.cursor_back_mut();
        while let Some(cur) = last.current() {
            if item.prio <= cur.prio {
                let cur = NonNull::new(cur as *const _ as *mut KernelListTest).unwrap();
                unsafe { list.insert_after(cur, item) };
                return;
            }
            last.move_prev();
        }
        list.push_front(item);
    }
}

#[allow(dead_code)]
pub fn test_kernel_list() {
    {
        let prio0 = Box::try_new(KernelListTest::new(0)).unwrap();

        let prio1 = Box::try_new(KernelListTest::new(1)).unwrap();
        let prio2 = Box::try_new(KernelListTest::new(2)).unwrap();
        let prio3 = Box::try_new(KernelListTest::new(3)).unwrap();

        let mut list: List<Box<KernelListTest>> = List::new();
        list.push_front(prio0);
        list.push_front(prio1);
        list.push_front(prio2);
        list.push_front(prio3);

        let mut data = [0; 5];
        let mut counter = 0;
        let mut start = list.cursor_front();
        while let Some(item) = start.current() {
            data[counter] = item.prio;
            counter += 1;
            start.move_next();
        }
        assert_eq!(counter, 4);
        assert_eq!(data[0], 3);
        assert_eq!(data[1], 2);
        assert_eq!(data[2], 1);
        assert_eq!(data[3], 0);
    }

    {
        let prio0 = Box::try_new(KernelListTest::new(0)).unwrap();

        let prio1 = Box::try_new(KernelListTest::new(1)).unwrap();
        let prio2 = Box::try_new(KernelListTest::new(2)).unwrap();
        let prio3 = Box::try_new(KernelListTest::new(3)).unwrap();

        let mut list: List<Box<KernelListTest>> = List::new();
        add_by_prio(&mut list, prio3);
        add_by_prio(&mut list, prio0);
        add_by_prio(&mut list, prio2);
        add_by_prio(&mut list, prio1);

        let mut data = [0; 5];
        let mut counter = 0;
        let mut start = list.cursor_front();
        while let Some(item) = start.current() {
            data[counter] = item.prio;
            counter += 1;
            start.move_next();
        }
    }
    pr_debug!("test kernel list ok!");
}

#[allow(dead_code)]
pub fn run_tests() {
    test_hashtable();
    test_list_macro();
    test_kernel_list();
}
