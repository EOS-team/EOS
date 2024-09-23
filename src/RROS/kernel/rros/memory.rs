use core::{mem::size_of, mem::zeroed};

use kernel::{
    mm,
    prelude::*,
    premmpt, rbtree, spinlock_init,
    sync::{self, SpinLock},
    vmalloc,
};

use crate::{list, monitor};

// Some examples about init_static_sync:
//
// pub struct Test {
//     a: u32,
//     b: u32,
// }

// init_static_sync! {
//     static A: SpinLock<Test> = Test { a: 10, b: 20 };

//     /// Documentation for `B`.
//     pub static B: SpinLock<u32> = 0;

//     pub(crate) static C: SpinLock<Test> = Test { a: 10, b: 20 };
//     // static D: CondVar;

//     // static E: RevocableMutex<Test> = Test { a: 30, b: 40 };
// }

#[repr(C)]
union RrosHeapPgentry_item {
    map: u32,
    bsize: u32,
}

struct RrosHeapPgentry {
    #[allow(dead_code)]
    prev: u32,
    #[allow(dead_code)]
    next: u32,
    #[allow(dead_code)]
    ttype: u32,
    #[allow(dead_code)]
    item: RrosHeapPgentry_item,
}

impl Default for RrosHeapPgentry {
    fn default() -> Self {
        unsafe { zeroed() }
    }
}

#[allow(dead_code)]
struct RrosHeap {
    membase: Arc<u8>,
    addr_tree: rbtree::RBTree<u32, Arc<RrosHeapRange>>,
    size_tree: rbtree::RBTree<u32, Arc<RrosHeapRange>>,
    rros_heap_pgentry: Arc<RrosHeapPgentry>,
    usable_size: usize,
    used_size: usize,
    buckets: [u32; RROS_HEAP_MAX_BUCKETS as usize],
    lock: sync::SpinLock<i32>,
    next: list::ListHead,
}

impl RrosHeap {
    #[allow(dead_code)]
    fn new() -> Result<Self> {
        Ok(Self {
            membase: Arc::try_new(0)?,
            addr_tree: rbtree::RBTree::new(),
            size_tree: rbtree::RBTree::new(),
            rros_heap_pgentry: Arc::try_new(RrosHeapPgentry::default())?,
            usable_size: 0,
            used_size: 0,
            buckets: [0; RROS_HEAP_MAX_BUCKETS as usize],
            lock: unsafe { sync::SpinLock::<i32>::new(0) },
            next: list::ListHead::default(),
        })
    }
}

#[allow(dead_code)]
struct RrosHeapRange {
    addr: u32,
    size: u32,
    page_size: usize,
}

#[allow(dead_code)]
struct RrosUserWindow {
    state: u32,
    info: u32,
    pp_pending: u32,
}

#[allow(dead_code)]
const PAGE_SIZE: u32 = 4096 as u32;
#[allow(dead_code)]
const RROS_HEAP_PAGE_SHIFT: u32 = 9 as u32;
// const RROS_HEAP_PAGE_SIZE: u32 = (1UL << RROS_HEAP_PAGE_SHIFT) as u32;
// const RROS_HEAP_PAGE_MASK: u32 = (~(RROS_HEAP_PAGE_SIZE - 1)) as u32;
#[allow(dead_code)]
const RROS_HEAP_MIN_LOG2: u32 = 4 as u32;
#[allow(dead_code)]
const RROS_HEAP_MAX_BUCKETS: u32 = (RROS_HEAP_PAGE_SHIFT - RROS_HEAP_MIN_LOG2) as u32;
// const RROS_HEAP_MIN_ALIGN: u32 = (1U << RROS_HEAP_MIN_LOG2) as u32;
#[allow(dead_code)]
const RROS_HEAP_MAX_HEAPSZ: u32 = (4294967295 - PAGE_SIZE + 1) as u32;
#[allow(dead_code)]
const RROS_HEAP_PGENT_BITS: u32 = (32 - RROS_HEAP_PAGE_SHIFT) as u32;
// const RROS_HEAP_PGMAP_BYTES: u32 = sizeof(struct rros_heap_pgentry) as u32;

#[allow(dead_code)]
const CONFIG_RROS_COREMEM_SIZE: usize = 2048;
#[allow(dead_code)]
const CONFIG_RROS_NR_THREADS: usize = 256;
#[allow(dead_code)]
const CONFIG_RROS_NR_MONITORS: usize = 512;

#[allow(dead_code)]
pub static mut RROS_SHM_SIZE: usize = 0;

#[allow(dead_code)]
pub fn init_memory(sysheap_size_arg: u32) -> Result<usize> {
    let rros_system_heap: Arc<SpinLock<RrosHeap>> =
        Arc::try_new(unsafe { SpinLock::new(RrosHeap::new()?) })?;
    let rros_shared_heap: Arc<SpinLock<RrosHeap>> =
        Arc::try_new(unsafe { SpinLock::new(RrosHeap::new()?) })?;
    // let mut RrosHeapRangeManage: Arc<SpinLock<RrosHeapPgentry>> =
    //     Arc::try_new(unsafe{SpinLock::new(RrosHeapPgentry::new()?)})?;

    init_system_heap(rros_system_heap.clone(), sysheap_size_arg)?; //RrosHeapRangeManage.clone()
    init_shared_heap(rros_shared_heap.clone())?; // RrosHeapRangeManage.clone()

    Ok(0)
}

#[allow(dead_code)]
fn init_system_heap(heap: Arc<SpinLock<RrosHeap>>, sysheap_size_arg: u32) -> Result<usize> {
    //hrm: Arc<SpinLock<RrosHeapRangeManage>>
    let mut size: usize = sysheap_size_arg as usize;
    if sysheap_size_arg == 0 {
        size = CONFIG_RROS_COREMEM_SIZE * 1024;
    }

    let vmalloc_res = vmalloc::c_vmalloc(size as u64);
    let memptr;
    match vmalloc_res {
        Some(ptr) => memptr = ptr,
        None => return Err(kernel::Error::ENOMEM),
    }

    let res = rros_init_heap(heap, memptr as *mut u8, size);
    match res {
        Ok(_o) => pr_info!("Rros core system memory success init!\n"),
        Err(_e) => {
            vmalloc::c_vfree(memptr);
            return Err(_e);
        }
    }

    Ok(0)
}

#[allow(dead_code)]
fn init_shared_heap(heap: Arc<SpinLock<RrosHeap>>) -> Result<usize> {
    //hrm: Arc<SpinLock<RrosHeapRangeManage>>
    let mut size: usize = CONFIG_RROS_NR_THREADS * size_of::<RrosUserWindow>()
        + CONFIG_RROS_NR_MONITORS * size_of::<monitor::RrosMonitorState>();
    size = mm::page_align(size)?;
    mm::page_aligned(size)?;

    let kzalloc_res = vmalloc::c_kzalloc(size as u64);
    let mem;
    match kzalloc_res {
        Some(ptr) => mem = ptr,
        None => return Err(kernel::Error::ENOMEM),
    }

    let res = rros_init_heap(heap, mem as *mut u8, size);
    match res {
        Ok(_o) => pr_err!("Rros core shared memory success init!\n"),
        Err(_e) => {
            vmalloc::c_kzfree(mem);
            return Err(_e);
        }
    }

    unsafe { RROS_SHM_SIZE = size };

    Ok(0)
}

fn rros_init_heap(heap: Arc<SpinLock<RrosHeap>>, membase: *mut u8, size: usize) -> Result<usize> {
    let nrpages;

    premmpt::running_inband()?;

    mm::page_aligned(size)?;
    if size > RROS_HEAP_MAX_HEAPSZ as usize {
        return Err(kernel::Error::EINVAL);
    }

    let mut op_lock = heap.lock();
    for i in op_lock.buckets.iter_mut() {
        *i = u32::MAX;
    }

    op_lock.lock = unsafe { SpinLock::new(1) };
    let pinned = unsafe { Pin::new_unchecked(&mut op_lock.lock) };
    spinlock_init!(pinned, "value");

    nrpages = size >> RROS_HEAP_PAGE_SHIFT;
    let a: u64 = size_of::<RrosHeapPgentry>() as u64;
    let _kzalloc_res = vmalloc::c_kzalloc(a * nrpages as u64);
    // match kzalloc_res {
    //     Some(ptr) => {
    //         // let ptr_u8 = ptr as *mut u8;
    //         op_lock.rros_heap_pgentry = Arc::try_new(unsafe{*(ptr_u8) as &mut RrosHeapPgentry)})?;
    //     }
    //     None => return Err(kernel::Error::ENOMEM),
    // }

    op_lock.membase = Arc::try_new(unsafe { *membase })?;
    op_lock.usable_size = size;
    op_lock.used_size = 0;

    op_lock.size_tree = rbtree::RBTree::new();
    op_lock.addr_tree = rbtree::RBTree::new();
    release_page_range(heap.clone(), membase, size);

    Ok(0)
}

#[allow(dead_code)]
fn release_page_range(_heap: Arc<SpinLock<RrosHeap>>, _membase: *mut u8, _size: usize) {
    // let freed: *mut RrosHeapRange = membase as *mut RrosHeapRange;
    // insert_range_byaddr(heap.clone(), freed);
    // insert_range_bysize(heap.clone(), freed);
}

#[allow(dead_code)]
fn insert_range_byaddr(heap: Arc<SpinLock<RrosHeap>>, _freed: *mut RrosHeapRange) -> Result<usize> {
    let mut _op_lock = heap.lock();
    // unsafe {
    //     op_lock.addr_tree.try_insert(0, Arc::try_new(*freed)?);
    // }
    Ok(0)
}

#[allow(dead_code)]
fn insert_range_bysize(heap: Arc<SpinLock<RrosHeap>>, _freed: *mut RrosHeapRange) -> Result<usize> {
    let mut _op_lock = heap.lock();
    // unsafe {
    //     op_lock.size_tree.try_insert(0, Arc::try_new(*freed)?);
    // }
    Ok(0)
}
