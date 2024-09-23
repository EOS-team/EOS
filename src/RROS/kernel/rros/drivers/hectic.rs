use core::result::Result::Err;

use alloc::sync::{Arc, Weak};

use crate::{
    file::{rros_open_file, rros_release_file, RrosFile},
    flags::RrosFlag,
    guard::Guard,
    stax::Stax,
    thread::RrosKthread,
    timer::*,
};

use kernel::{
    c_str, chrdev, file_operations, irq_work::IrqWork, prelude::*, spinlock_init, sync::SpinLock,
    KernelModule,
};

const RROS_HECIOC_LOCK_STAX: u32 = 18441;
const RROS_HECIOC_UNLOCK_STAX: u32 = 18442;

module! {
    type: Hecticdev,
    name: b"hectic",
    author: b"wxg",
    description: b"hectic driver",
    license: b"GPL v2",
}

#[repr(C)]
struct HecticTaskIndex {
    index: u32,
    flags: u32,
}

#[repr(C)]
struct HecticSwitchReq {
    from: u32,
    to: u32,
}

#[allow(dead_code)]
struct HecticError {
    last_switch: HecticSwitchReq,
    fp_val: u32,
}

#[allow(dead_code)]
struct RtswitchTask {
    base: HecticTaskIndex,
    rt_synch: RrosFlag,
    //TODO: nrt_synch: semaphore
    kthread: RrosKthread,
    last_switch: u32,
    ctx: Option<Weak<RtswitchTask>>,
}

#[allow(dead_code)]
pub struct RtswitchContext {
    tasks: Vec<Arc<RtswitchTask>>,
    tasks_count: u32,
    next_index: u32,
    //lock: Semaphore,
    cpu: u32,
    switches_count: u32,
    pause_us: u64,
    next_task: u32,
    wake_up_delay: Arc<SpinLock<RrosTimer>>,
    failed: bool,
    error: HecticError,
    utask: Option<Weak<RtswitchTask>>,
    wake_utask: IrqWork,
    stax: Pin<Box<Stax<()>>>,
    o_guard: SpinLock<Vec<usize>>,
    i_guard: SpinLock<Vec<usize>>,
    rfile: RrosFile,
}

impl RtswitchTask {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            base: HecticTaskIndex { index: 0, flags: 0 },
            rt_synch: RrosFlag::new(),
            kthread: RrosKthread::new(None),
            last_switch: 0,
            ctx: None,
        }
    }
}

impl RtswitchContext {
    fn new() -> Result<Self> {
        // Spinlock and Stax have to be initialized after new.So their new function is unsafe.
        // Safety: We promise that the SpinLock and Stax will be initialized before they are used.
        let ctx = RtswitchContext {
            tasks: Vec::new(),
            tasks_count: 0,
            next_index: 0,
            //lock: Semaphore::new(),
            cpu: 0,
            switches_count: 0,
            pause_us: 0,
            next_task: 0,
            wake_up_delay: Arc::try_new(unsafe { SpinLock::new(RrosTimer::new(0)) })?,
            failed: false,
            error: HecticError {
                last_switch: HecticSwitchReq {
                    from: u32::MAX,
                    to: u32::MAX,
                },
                fp_val: 0,
            },
            utask: None,
            wake_utask: IrqWork::new(),
            stax: unsafe { Pin::from(Box::try_new(Stax::new(()))?) },
            o_guard: unsafe { SpinLock::new(Vec::new()) },
            i_guard: unsafe { SpinLock::new(Vec::new()) },
            rfile: RrosFile::new(),
        };
        Ok(ctx)
    }

    fn init(&mut self) -> Result<()> {
        Stax::init((&mut self.stax).as_mut())?;
        let o_pinned = unsafe { Pin::new_unchecked(&mut self.o_guard) };
        spinlock_init!(o_pinned, "o_guard");
        let i_pinned = unsafe { Pin::new_unchecked(&mut self.i_guard) };
        spinlock_init!(i_pinned, "i_guard");
        // TODO: some initialization which is not needed by stax test
        return Ok(());
    }
}
pub struct HecticFile;

impl file_operations::FileOpener<u8> for HecticFile {
    fn open(_context: &u8, file: &kernel::file::File) -> kernel::Result<Self::Wrapper> {
        let mut ctx: Box<RtswitchContext> = Box::try_new(RtswitchContext::new()?)?;
        let ctx_ref = ctx.as_mut();
        ctx_ref.init()?;
        let rfile = &mut ctx_ref.rfile;
        rros_open_file(rfile, file.get_ptr())?;
        Ok(ctx)
    }
}

fn lock_stax(ctx: &RtswitchContext, is_inband: bool) -> Result<i32> {
    match ctx.stax.as_ref().get_ref().lock() {
        Ok(g) => {
            let tmp = Box::try_new(g)?;
            let mut guard = if is_inband {
                ctx.i_guard.lock()
            } else {
                ctx.o_guard.lock()
            };
            guard.try_push(Box::into_raw(tmp) as usize)?;
            Ok(0)
        }
        Err(e) => {
            return Err(e);
        }
    }
}

fn unlock_stax(ctx: &RtswitchContext, is_inband: bool) -> Result<i32> {
    let tmp;
    let mut count = 0;
    loop {
        count += 1;
        let mut guard = if is_inband {
            ctx.i_guard.lock()
        } else {
            ctx.o_guard.lock()
        };
        if let Some(t) = guard.pop() {
            tmp = t;
            break;
        }
        if count > 1000 {
            return Ok(0);
        }
    }
    // Safety: Every pointer will be pushed and poped in a pair. And the pointer is vaild before it is poped.
    unsafe {
        Box::from_raw(tmp as *mut Guard<'_, Stax<()>>);
    }
    Ok(0)
}

impl file_operations::FileOperations for HecticFile {
    kernel::declare_file_operations!(ioctl, oob_ioctl, compat_ioctl, compat_oob_ioctl);

    type Wrapper = Box<RtswitchContext>;

    fn release(mut ctx: Self::Wrapper, _file: &kernel::file::File) {
        rros_release_file(&mut ctx.rfile).expect("release file failed");
        pr_debug!("hectic release!\n");
    }

    fn ioctl(
        ctx: &<<Self::Wrapper as kernel::types::PointerWrapper>::Borrowed as core::ops::Deref>::Target,
        _file: &kernel::file::File,
        cmd: &mut file_operations::IoctlCommand,
    ) -> kernel::Result<i32> {
        pr_debug!("Hectic ioctl\n");
        match cmd.cmd {
            RROS_HECIOC_LOCK_STAX => lock_stax(ctx, true),
            RROS_HECIOC_UNLOCK_STAX => unlock_stax(ctx, true),
            _ => Ok(0),
        }
    }

    fn oob_ioctl(
        ctx: &<<Self::Wrapper as kernel::types::PointerWrapper>::Borrowed as core::ops::Deref>::Target,
        _file: &kernel::file::File,
        cmd: &mut file_operations::IoctlCommand,
    ) -> kernel::Result<i32> {
        pr_debug!("Hectic oob_ioctl\n");
        match cmd.cmd {
            RROS_HECIOC_LOCK_STAX => lock_stax(ctx, false),
            RROS_HECIOC_UNLOCK_STAX => unlock_stax(ctx, false),
            _ => Ok(0),
        }
    }
}

pub struct Hecticdev {
    pub dev: Pin<Box<chrdev::Registration<1>>>,
}

impl KernelModule for Hecticdev {
    fn init() -> Result<Self> {
        let mut _dev = chrdev::Registration::new_pinned(c_str!("hecticdev"), 0, &THIS_MODULE)?;

        _dev.as_mut().register::<HecticFile>()?;

        Ok(Hecticdev { dev: _dev })
    }
}

impl Drop for Hecticdev {
    fn drop(&mut self) {
        pr_debug!("Hectic exit\n");
    }
}
