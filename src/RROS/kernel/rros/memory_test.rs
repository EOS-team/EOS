use kernel::{
    container_of, memory_rros::*, memory_rros_test::*, prelude::*, rbtree::RBTree, vmalloc,
};

use alloc::alloc::*;
use alloc::alloc_rros::*;

pub fn mem_test() {
    // mem_test1();
    // mem_test2();
    // test_rbtree();
    // test_init_system_heap();
    // test_insert_system_heap();
    // test_alloc_chunk();
    // test_box_allocator();
    // test_chunk();
    // test_arc();
    // test_buckets();
    main_memory_rros_test();
}

#[allow(dead_code)]
fn test_buckets() {
    test_213();
    unsafe {
        pr_debug!("test_buckets: xxx is {}", RROS_SYSTEM_HEAP.buckets[1]);
    }
}

#[allow(dead_code)]
fn test_213() {
    unsafe {
        RROS_SYSTEM_HEAP.buckets[1] = 22;
    }
}

#[allow(dead_code)]
fn test_arc() {
    pr_debug!("test_arc: begin");
    let a1;
    let b1;
    let c1;
    let d1;
    let a = Arc::try_new_in(111, RrosMem);
    let b = Arc::try_new_in(222, RrosMem);
    let c = Arc::try_new_in(333, RrosMem);
    let d = Arc::try_new_in(444, RrosMem);
    match a {
        Ok(x) => {
            a1 = x;
        }
        Err(_) => {
            pr_err!("test_arc: arc alloc err");
            return;
        }
    }
    match b {
        Ok(x) => {
            b1 = x;
        }
        Err(_) => {
            pr_err!("test_arc: arc alloc err");
            return;
        }
    }
    match c {
        Ok(x) => {
            c1 = x;
        }
        Err(_) => {
            pr_err!("test_arc: arc alloc err");
            return;
        }
    }
    match d {
        Ok(x) => {
            d1 = x;
        }
        Err(_) => {
            pr_err!("test_arc: arc alloc err");
            return;
        }
    }
    pr_debug!("test_arc: a is {}", a1);
    pr_debug!("test_arc: b is {}", b1);
    pr_debug!("test_arc: c is {}", c1);
    pr_debug!("test_arc: d is {}", d1);

    pr_debug!("test_arc: end");
}

#[allow(dead_code)]
fn test_fn(x: Arc<i32, RrosMem>) {
    pr_debug!("test_fn x is {}", x);
}

#[allow(dead_code)]
fn mem_test1() {
    let x = Box::try_new_in(123, Global);
    match x {
        Err(_) => {
            pr_err!("alloc error");
        }
        Ok(y) => {
            let z = y;
            pr_debug!("z is {}", z);
        }
    }
    pr_debug!("alloc success");
}

#[allow(dead_code)]
struct MemTestxy {
    x: i32,
    y: i32,
    z: i32,
}

// The memory requested by the test is directly converted into a structure pointer:
// the conclusion is that it can be used directly.
#[allow(dead_code)]
pub fn mem_test2() -> Result<usize> {
    let vmalloc_res = vmalloc::c_vmalloc(1024 as u64);
    let memptr;
    match vmalloc_res {
        Some(ptr) => memptr = ptr,
        None => return Err(kernel::Error::ENOMEM),
    }
    let xxx = memptr as *mut MemTestxy;
    unsafe {
        (*xxx).x = 11;
        (*xxx).y = 22;
        (*xxx).z = 33;
        pr_debug!("mem_test2: z is {}", (*xxx).z);
        pr_debug!("mem_test2: x addr is {:p}", &mut (*xxx).x as *mut i32);
        pr_debug!("mem_test2: y addr is {:p}", &mut (*xxx).y as *mut i32);
        pr_debug!("mem_test2: z addr is {:p}", &mut (*xxx).z as *mut i32);
    }
    Ok(0)
}

#[allow(dead_code)]
struct Pageinfo {
    membase: u32,
    size: u32,
}

#[allow(dead_code)]
fn test_rbtree() -> Result<usize> {
    pr_debug!("~~~test_rbtree begin~~~");
    let mut root: RBTree<u32, Pageinfo> = RBTree::new();

    let x1 = Pageinfo {
        membase: 100,
        size: 200,
    };
    let x2 = Pageinfo {
        membase: 101,
        size: 200,
    };
    let x3 = Pageinfo {
        membase: 102,
        size: 200,
    };

    let node1 = RBTree::try_allocate_node(100, x1)?;
    // let mut node1: = RBTree::try_allocate_node(300,x2)?;
    let node2 = RBTree::try_allocate_node(101, x2)?;
    let node3 = RBTree::try_allocate_node(102, x3)?;
    root.insert(node1);
    root.insert(node2);
    root.insert(node3);
    // Traverse a red-black tree:
    for item in root.iter() {
        pr_debug!("item.0 is {}", item.0);
        pr_debug!("item.1.size is {}", item.1.size);
    }
    pr_debug!("~~~test_rbtree end~~~");
    Ok(0)
}

#[allow(dead_code)]
fn test_init_system_heap() {
    let _ret = init_system_heap();
}

#[allow(dead_code)]
fn test_insert_system_heap() -> Result<usize> {
    pr_debug!("~~~test_insert_system_heap begin~~~");
    let _ret = init_system_heap();

    unsafe {
        let membase = RROS_SYSTEM_HEAP.membase;
        pr_debug!("test_insert_system_heap: membase is {:p}", membase);
        let x1 = new_rros_heap_range(membase, 1024);
        let x2 = new_rros_heap_range(addr_add_size(membase, 1024), 2048);
        let x3 = new_rros_heap_range(addr_add_size(membase, 2048), 4096);

        pr_debug!("test_insert_system_heap: 1");
        RROS_SYSTEM_HEAP.insert_range_byaddr(x1);
        RROS_SYSTEM_HEAP.insert_range_byaddr(x2);
        RROS_SYSTEM_HEAP.insert_range_byaddr(x3);
        pr_debug!("test_insert_system_heap: 2");
        let rb_node = RROS_SYSTEM_HEAP.addr_tree.clone().unwrap().rb_node;
        if rb_node.is_null() {
            pr_debug!("test_insert_system_heap: root is null");
        } else {
            let p = container_of!(rb_node, RrosHeapRange, addr_node);
            pr_debug!("test_insert_system_heap root size is {}", (*p).size);
        }
        pr_debug!("test_insert_system_heap: 3");
    }
    Ok(0)
}

#[allow(dead_code)]
fn test_small_chunk() {}

#[allow(dead_code)]
fn test_chunk() {
    pr_debug!("~~~test_chunk: begin~~~");
    let _x = __rros_sys_heap_alloc(1025, 0);
    pr_debug!("~~~test_chunk: 1~~~");
    let _y = __rros_sys_heap_alloc(4, 0);
    pr_debug!("~~~test_chunk: end~~~");
}

#[allow(dead_code)]
fn test_alloc_chunk() {
    pr_debug!("~~~test_alloc_chunk begin~~~");
    unsafe {
        // View the root of the current RROS_SYSTEM_HEAP size tree.
        let rb_node = RROS_SYSTEM_HEAP.size_tree.clone().unwrap().rb_node;
        let mut p = container_of!(rb_node, RrosHeapRange, size_node);
        let raw_size = (*p).size;
        pr_debug!("test_insert_system_heap root size is {}", raw_size);
        let membase = RROS_SYSTEM_HEAP.membase;
        pr_debug!("test_alloc_chunk: membase is {}", membase as u32);
        let res = RROS_SYSTEM_HEAP.rros_alloc_chunk(1024);
        let mut x: u32 = 0;
        let mut addr = 0 as *mut u8;
        match res {
            Some(a) => {
                addr = a;
                x = a as u32;
                pr_debug!("test_alloc_chunk: alloc addr is {}", x as u32);
            }
            None => {
                pr_err!("test_alloc_chunk: alloc err");
            }
        }
        pr_debug!(
            "test_alloc_chunk: membase - alloc = {}",
            x as u32 - membase as u32
        );
        p = container_of!(rb_node, RrosHeapRange, size_node);
        let mut new_size = (*p).size;
        pr_debug!("test_insert_system_heap root size is {}", new_size);
        pr_debug!(
            "test_alloc_chunk: raw_size - new_size = {}",
            raw_size - new_size
        );
        // Test recycle.
        pr_debug!("~~~test_alloc_chunk: test free begin~~~");
        RROS_SYSTEM_HEAP.rros_free_chunk(addr);
        p = container_of!(rb_node, RrosHeapRange, size_node);
        new_size = (*p).size;
        pr_debug!("test_insert_system_heap root size is {}", new_size);
        pr_debug!("~~~test_alloc_chunk: test free end~~~");
    }
    pr_debug!("~~~test_alloc_chunk end~~~");
}

#[allow(dead_code)]
fn test_box_allocator() {
    pr_debug!("test_box_allocator: begin");
    let x = Box::try_new_in(123, RrosMem);
    match x {
        Err(_) => {
            pr_err!("test_box_allocator: alloc error");
            return;
        }
        Ok(_x) => {
            unsafe {
                let rb_node = RROS_SYSTEM_HEAP.size_tree.clone().unwrap().rb_node;
                let p = container_of!(rb_node, RrosHeapRange, size_node);
                let raw_size = (*p).size;
                pr_debug!("test_box_allocator: root size is {}", raw_size);
            }
            pr_debug!("test_box_allocator: x is {}", _x);
        }
    }
    unsafe {
        let rb_node = RROS_SYSTEM_HEAP.size_tree.clone().unwrap().rb_node;
        let p = container_of!(rb_node, RrosHeapRange, size_node);
        let raw_size = (*p).size;
        pr_debug!("test_box_allocator: root size is {}", raw_size);
    }
    pr_debug!("test_box_allocator: alloc success");
}
