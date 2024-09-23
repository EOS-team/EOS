use crate::{clock::*, fifo::*, sched::*, thread::*, timeout::*, timer::*};
use core::{mem::size_of, ptr::NonNull};
use kernel::{
    c_str, c_types,
    double_linked_list::*,
    ktime,
    ktime::{ktime_to_timespec64, timespec64_to_ktime, Timespec64},
    memory_rros::*,
    prelude::*,
    spinlock_init,
    sync::{Lock, SpinLock},
    types::Atomic,
};

pub static mut RROS_SCHED_TP: RrosSchedClass = RrosSchedClass {
    sched_init: Some(tp_init),
    sched_enqueue: Some(tp_enqueue),
    sched_dequeue: Some(tp_dequeue),
    sched_requeue: Some(tp_requeue),
    sched_pick: Some(tp_pick),
    sched_migrate: Some(tp_migrate),
    sched_chkparam: Some(tp_chkparam),
    sched_setparam: Some(tp_setparam),
    sched_getparam: Some(tp_getparam),
    sched_trackprio: Some(tp_trackprio),
    sched_ceilprio: Some(tp_ceilprio),
    sched_declare: Some(tp_declare),
    sched_forget: Some(tp_forget),
    sched_show: Some(tp_show),
    sched_control: Some(tp_control),
    sched_kick: None,
    sched_tick: None,
    nthreads: 0,
    next: 0 as *mut RrosSchedClass,
    weight: 3 * RROS_CLASS_WEIGHT_FACTOR,
    policy: SCHED_TP,
    name: "tp",
    flag: 4,
};

pub const CONFIG_RROS_SCHED_TP_NR_PART: i32 = 5; // Set to 5 by default temporarily.
pub const RROS_TP_MAX_PRIO: i32 = RROS_FIFO_MAX_PRIO;
pub const RROS_TP_MIN_PRIO: i32 = RROS_FIFO_MIN_PRIO;
#[allow(dead_code)]
pub const RROS_TP_NR_PRIO: i32 = RROS_TP_MAX_PRIO - RROS_TP_MIN_PRIO + 1;

type KtimeT = i64;

pub struct RrosTpRq {
    pub runnable: RrosSchedQueue,
}
impl RrosTpRq {
    pub fn new() -> Result<Self> {
        Ok(RrosTpRq {
            runnable: RrosSchedQueue::new()?,
        })
    }
}

pub struct RrosTpWindow {
    pub w_offset: KtimeT,
    pub w_part: i32,
}

pub struct RrosTpSchedule {
    pub pwin_nr: i32,
    pub tf_duration: KtimeT,
    pub refcount: *mut Atomic,
    pub pwins: [RrosTpWindow; 1 as usize],
}

pub struct RrosSchedTp {
    pub partitions: Option<[RrosTpRq; CONFIG_RROS_SCHED_TP_NR_PART as usize]>,
    pub idle: RrosTpRq,
    pub tps: *mut RrosTpRq,
    pub tf_timer: Option<Arc<SpinLock<RrosTimer>>>,
    pub gps: *mut RrosTpSchedule,
    pub wnext: i32,
    pub tf_start: KtimeT,
    pub threads: Option<List<Arc<SpinLock<RrosThread>>>>,
}
impl RrosSchedTp {
    pub fn new() -> Result<Self> {
        Ok(RrosSchedTp {
            partitions: None,
            idle: RrosTpRq::new()?,
            tps: 0 as *mut RrosTpRq,
            tf_timer: None,
            gps: 0 as *mut RrosTpSchedule,
            wnext: 0,
            tf_start: 0,
            threads: None,
        })
    }
}

pub fn tp_schedule_next(tp: &mut RrosSchedTp) -> Result<usize> {
    let mut w: *mut RrosTpWindow;
    let rq: *mut rros_rq;
    let mut t: KtimeT;
    let mut now: KtimeT;
    let p_next: i32;

    rq = kernel::container_of!(tp as *mut RrosSchedTp, rros_rq, tp) as *mut rros_rq;
    // assert_hard_lock(&rq->lock);
    unsafe {
        w = &mut ((*tp.gps).pwins[tp.wnext as usize]) as *mut RrosTpWindow;
        p_next = (*w).w_part;
        if p_next < 0 {
            tp.tps = &mut tp.idle as *mut RrosTpRq;
        } else {
            tp.tps = &mut tp.partitions.as_mut().unwrap()[p_next as usize] as *mut RrosTpRq;
        }
        tp.wnext = (tp.wnext + 1) % (*tp.gps).pwin_nr;
        w = &mut (*tp.gps).pwins[tp.wnext as usize] as *mut RrosTpWindow;
        t = ktime::ktime_add(tp.tf_start, (*w).w_offset);
    }

    loop {
        unsafe {
            now = rros_read_clock(&RROS_MONO_CLOCK);
            if ktime::ktime_compare(now, t) <= 0 {
                break;
            }
            t = ktime::ktime_add(tp.tf_start, (*tp.gps).tf_duration);
            tp.tf_start = t;
            tp.wnext = 0;
        }
    }

    rros_start_timer(tp.tf_timer.as_mut().unwrap().clone(), t, RROS_INFINITE);
    rros_set_resched(Some(rq));
    Ok(0)
}

pub fn tp_tick_handler(timer: *mut RrosTimer) {
    unsafe {
        // There is a problem with `container_of` here.
        let rq = kernel::container_of!(timer, rros_rq, tp.tf_timer) as *mut rros_rq;
        let mut tp = &mut (*rq).tp;

        // raw_spin_lock(&rq->lock);
        if tp.wnext + 1 == (*tp.gps).pwin_nr {
            tp.tf_start = ktime::ktime_add(tp.tf_start, (*tp.gps).tf_duration);
        }
        let _ret = tp_schedule_next(tp);

        // raw_spin_unlock(&rq->lock);
    }
}

pub fn tp_init(rq: *mut rros_rq) -> Result<usize> {
    unsafe {
        let mut tp = &mut (*rq).tp;
        let r1 = RrosTpRq::new()?;
        let r2 = RrosTpRq::new()?;
        let r3 = RrosTpRq::new()?;
        let r4 = RrosTpRq::new()?;
        let r5 = RrosTpRq::new()?;
        let mut temp: [RrosTpRq; CONFIG_RROS_SCHED_TP_NR_PART as usize] = [r1, r2, r3, r4, r5];
        for n in 0..CONFIG_RROS_SCHED_TP_NR_PART {
            // temp[n as usize].runnable.head = Some(List::new(Arc::try_new(SpinLock::new(RrosThread::new()?))?));
            let mut tmp = Arc::<SpinLock<RrosThread>>::try_new_uninit()?;
            let mut tmp = {
                core::ptr::write_bytes(Arc::get_mut_unchecked(&mut tmp), 0, 1);
                tmp.assume_init()
            };
            let pinned = Pin::new_unchecked(Arc::get_mut_unchecked(&mut tmp));
            spinlock_init!(pinned, "tp kthread");

            // let mut thread = SpinLock::new(RrosThread::new()?);
            // let pinned = Pin::new_unchecked(&mut thread);
            // spinlock_init!(pinned, "rros_threads");
            // Arc::get_mut(&mut tmp).unwrap().write(thread);
            temp[n as usize].runnable.head = Some(List::new(tmp)); //Arc::try_new(thread)?
            (*temp[n as usize]
                .runnable
                .head
                .as_mut()
                .unwrap()
                .head
                .value
                .locked_data()
                .get())
            .init()?;
            // let pinned = Pin::new_unchecked(&mut *(Arc::into_raw( temp[n as usize].runnable.head.as_mut().unwrap().head.value.clone()) as *mut SpinLock<RrosThread>));
            // // &mut *Arc::into_raw( *(*rq_ptr).root_thread.clone().as_mut().unwrap()) as &mut SpinLock<RrosThread>
            // spinlock_init!(pinned, "rros_threads");
        }
        tp.partitions = Some(temp);
        tp.tps = 0 as *mut RrosTpRq;
        tp.gps = 0 as *mut RrosTpSchedule;
        tp.tf_timer = Some(Arc::try_new(SpinLock::new(RrosTimer::new(0)))?);

        let mut tf_timer = SpinLock::new(RrosTimer::new(2));
        let pinned_p = Pin::new_unchecked(&mut tf_timer);
        spinlock_init!(pinned_p, "ptimer");
        tp.tf_timer = Some(Arc::try_new(tf_timer)?);

        rros_init_timer_on_rq(
            tp.tf_timer.clone().as_mut().unwrap().clone(),
            &mut RROS_MONO_CLOCK,
            Some(tp_tick_handler),
            rq,
            c_str!("[tp-tick]"),
            RROS_TIMER_IGRAVITY,
        );
        // rros_set_timer_name(&tp->tf_timer, "[tp-tick]");
        pr_info!("tp_init ok");
        Ok(0)
    }
}

pub fn tp_setparam(
    thread: Option<Arc<SpinLock<RrosThread>>>,
    p: Option<Arc<SpinLock<RrosSchedParam>>>,
) -> Result<usize> {
    unsafe {
        let thread_clone = thread.clone();
        let thread_clone = thread_clone.unwrap();
        let rq = (*thread_clone.locked_data().get()).rq.unwrap();
        let p = p.unwrap();
        (*thread_clone.locked_data().get()).tps = &mut (*rq).tp.partitions.as_mut().unwrap()
            [(*p.locked_data().get()).tp.ptid as usize]
            as *mut RrosTpRq;
        (*thread_clone.locked_data().get()).state &= !T_WEAK;
        let prio = (*p.locked_data().get()).tp.prio;
        rros_set_effective_thread_priority(thread.clone(), prio)
    }
}

pub fn tp_getparam(
    thread: Option<Arc<SpinLock<RrosThread>>>,
    p: Option<Arc<SpinLock<RrosSchedParam>>>,
) {
    let thread = thread.unwrap();
    let p = p.unwrap();
    unsafe {
        (*p.locked_data().get()).tp.prio = (*thread.locked_data().get()).cprio;
        let p1 = (*thread.locked_data().get()).tps;
        let p2 = &mut (*(*thread.locked_data().get()).rq.unwrap())
            .tp
            .partitions
            .as_mut()
            .unwrap()[0] as *mut RrosTpRq;
        (*p.locked_data().get()).tp.ptid = p1.offset_from(p2) as i32;
    }
}

pub fn tp_trackprio(
    thread: Option<Arc<SpinLock<RrosThread>>>,
    p: Option<Arc<SpinLock<RrosSchedParam>>>,
) {
    let thread = thread.unwrap();
    unsafe {
        if p.is_some() {
            // RROS_WARN_ON(CORE,
            //     thread->base_class == &rros_sched_tp &&
            //     thread->tps - rros_thread_rq(thread)->tp.partitions
            //     != p->tp.ptid);
            let p = p.unwrap();
            (*thread.locked_data().get()).cprio = (*p.locked_data().get()).tp.prio;
        } else {
            (*thread.locked_data().get()).cprio = (*thread.locked_data().get()).bprio;
        }
    }
}

pub fn tp_ceilprio(thread: Arc<SpinLock<RrosThread>>, mut prio: i32) {
    if prio > RROS_TP_MAX_PRIO {
        prio = RROS_TP_MAX_PRIO;
    }

    unsafe { (*thread.locked_data().get()).cprio = prio };
}

pub fn tp_chkparam(
    thread: Option<Arc<SpinLock<RrosThread>>>,
    p: Option<Arc<SpinLock<RrosSchedParam>>>,
) -> Result<i32> {
    unsafe {
        let thread = thread.unwrap();
        let p = p.unwrap();
        let rq = (*thread.locked_data().get()).rq.unwrap();
        let tp = &(*rq).tp;

        let prio = (*p.locked_data().get()).tp.prio;
        let ptid = (*p.locked_data().get()).tp.ptid;
        pr_debug!("in tp_chkparam,gps = {:p}", tp.gps);
        pr_debug!("in tp_chkparam,prio = {}", prio);
        pr_debug!("in tp_chkparam,ptid = {}", ptid);
        if tp.gps == 0 as *mut RrosTpSchedule
            || prio < RROS_TP_MIN_PRIO
            || prio > RROS_TP_MAX_PRIO
            || ptid < 0
            || ptid >= CONFIG_RROS_SCHED_TP_NR_PART
        {
            pr_warn!("tp_chkparam error");
            return Err(kernel::Error::EINVAL);
        }
        // if tp.gps == 0 as *mut RrosTpSchedule{
        //     pr_warn!("in tp_chkparam,tp.gps == 0 as *mut RrosTpSchedule");
        //     return Err(kernel::Error::EINVAL);
        // }
    }
    pr_info!("tp_chkparam success");
    Ok(0)
}

pub fn tp_declare(
    thread: Option<Arc<SpinLock<RrosThread>>>,
    _p: Option<Arc<SpinLock<RrosSchedParam>>>,
) -> Result<i32> {
    let thread = thread.unwrap();
    // let p = p.unwrap();
    unsafe {
        let rq = (*thread.locked_data().get()).rq.unwrap();
        (*thread.locked_data().get()).tp_link =
            Some(Node::new(Arc::try_new(SpinLock::new(RrosThread::new()?))?));
        let tp_link = (*thread.locked_data().get()).tp_link.clone();
        (*rq).tp.threads = Some(List::new(Arc::try_new(SpinLock::new(RrosThread::new()?))?));
        if (*rq).tp.threads.clone().as_mut().unwrap().is_empty() {
            pr_debug!("tp.threads is empty!");
        }
        (*rq)
            .tp
            .threads
            .clone()
            .as_mut()
            .unwrap()
            .add_tail(tp_link.clone().as_mut().unwrap().value.clone());
    }
    pr_info!("tp_declare success!");
    Ok(0)
}

pub fn tp_forget(thread: Arc<SpinLock<RrosThread>>) -> Result<usize> {
    unsafe {
        (*thread.locked_data().get())
            .tp_link
            .clone()
            .as_mut()
            .unwrap()
            .remove();
        (*thread.locked_data().get()).tps = 0 as *mut RrosTpRq;
    }
    Ok(0)
}

pub fn tp_enqueue(thread: Arc<SpinLock<RrosThread>>) -> Result<i32> {
    unsafe {
        let head = (*((*thread.locked_data().get()).tps))
            .runnable
            .head
            .as_mut()
            .unwrap();
        if head.is_empty() {
            let node = Node::new(Arc::try_new(SpinLock::new(RrosThread::new()?))?);
            let box_node = Box::try_new(node).unwrap();
            let ptr = Box::into_raw(box_node);
            (*thread.locked_data().get()).rq_next = Some(NonNull::new(ptr).unwrap());
            let rq_next = (*thread.locked_data().get()).rq_next.clone();
            head.add_head(rq_next.clone().as_mut().unwrap().as_mut().value.clone());
        } else {
            let mut flag = 1;
            for i in head.len()..=1 {
                let thread_cprio = (*thread.locked_data().get()).cprio;
                let cprio_in_list = (*head
                    .get_by_index(i)
                    .unwrap()
                    .value
                    .clone()
                    .locked_data()
                    .get())
                .cprio;
                if thread_cprio <= cprio_in_list {
                    flag = 0;
                    let rq_next = (*thread.locked_data().get()).rq_next.clone();
                    head.enqueue_by_index(
                        i,
                        rq_next.clone().as_mut().unwrap().as_mut().value.clone(),
                    );
                    break;
                }
            }
            if flag == 1 {
                let rq_next = (*thread.locked_data().get()).rq_next.clone();
                head.add_head(rq_next.clone().as_mut().unwrap().as_mut().value.clone());
            }
        }
        Ok(0)
    }
}

pub fn tp_dequeue(thread: Arc<SpinLock<RrosThread>>) {
    unsafe {
        (*thread.locked_data().get())
            .rq_next
            .as_mut()
            .unwrap()
            .as_mut()
            .remove();
    }
}

pub fn tp_requeue(thread: Arc<SpinLock<RrosThread>>) {
    unsafe {
        let head = (*((*thread.locked_data().get()).tps))
            .runnable
            .head
            .as_mut()
            .unwrap();
        if head.is_empty() {
            let rq_next = (*thread.locked_data().get()).rq_next.clone();
            head.add_head(rq_next.clone().as_mut().unwrap().as_mut().value.clone());
        } else {
            let mut flag = 1;
            for i in head.len()..=1 {
                let thread_cprio = (*thread.locked_data().get()).cprio;
                let cprio_in_list = (*head
                    .get_by_index(i)
                    .unwrap()
                    .value
                    .clone()
                    .locked_data()
                    .get())
                .cprio;
                if thread_cprio < cprio_in_list {
                    flag = 0;
                    let rq_next = (*thread.locked_data().get()).rq_next.clone();
                    head.enqueue_by_index(
                        i,
                        rq_next.clone().as_mut().unwrap().as_mut().value.clone(),
                    );
                    break;
                }
            }
            if flag == 1 {
                let rq_next = (*thread.locked_data().get()).rq_next.clone();
                head.add_head(rq_next.clone().as_mut().unwrap().as_mut().value.clone());
            }
        }
    }
}

pub fn tp_pick(rq: Option<*mut rros_rq>) -> Result<Arc<SpinLock<RrosThread>>> {
    let rq = rq.unwrap();
    unsafe {
        let timer = Arc::into_raw((*rq).tp.tf_timer.as_mut().unwrap().clone())
            as *mut SpinLock<RrosTimer> as *mut RrosTimer;
        if rros_timer_is_running(timer) == false {
            return Err(kernel::Error::EINVAL);
        }
        let head = (*(*rq).tp.tps).runnable.head.as_mut().unwrap();
        if head.is_empty() {
            return Err(kernel::Error::EINVAL);
        }

        let __item = head.get_head().unwrap().value.clone();
        (*__item.locked_data().get())
            .rq_next
            .as_mut()
            .unwrap()
            .as_mut()
            .remove();
        return Ok(__item);
    }
}

pub fn tp_migrate(thread: Arc<SpinLock<RrosThread>>, _rq: *mut rros_rq) -> Result<usize> {
    let mut param = RrosSchedParam::new();
    unsafe {
        param.fifo.prio = (*thread.locked_data().get()).cprio;
        rros_set_thread_schedparam_locked(
            thread.clone(),
            Some(&RROS_SCHED_FIFO),
            Some(Arc::try_new(SpinLock::new(param))?),
        )?;
    }
    Ok(0)
}

pub fn tp_show(
    thread: *mut RrosThread,
    _buf: *mut c_types::c_char,
    _count: SsizeT,
) -> Result<usize> {
    unsafe {
        let p1 = (*thread).tps;
        let p2 = &mut (*(*thread).rq.unwrap()).tp.partitions.as_mut().unwrap()[0] as *mut RrosTpRq;
        let _ptid = p1.offset_from(p2) as i32;
        // return snprintf(buf, count, "%d\n", ptid);
        Ok(0)
    }
}

pub fn start_tp_schedule(rq: *mut rros_rq) {
    unsafe {
        let mut tp = &mut (*rq).tp;

        // assert_hard_lock(&rq->lock);

        if tp.gps == 0 as *mut RrosTpSchedule {
            return;
        }
        tp.wnext = 0;
        tp.tf_start = rros_read_clock(&RROS_MONO_CLOCK);
        let _ret = tp_schedule_next(tp);
    }
}

pub fn stop_tp_schedule(rq: *mut rros_rq) -> Result<usize> {
    unsafe {
        let tp = &mut (*rq).tp;
        // assert_hard_lock(&rq->lock);
        if tp.gps != 0 as *mut RrosTpSchedule {
            rros_stop_timer(tp.tf_timer.as_mut().unwrap().clone());
        }
        Ok(0)
    }
}

pub fn set_tp_schedule(rq: *mut rros_rq, gps: *mut RrosTpSchedule) -> Result<*mut RrosTpSchedule> {
    unsafe {
        let mut tp = &mut (*rq).tp;
        let mut thread;
        let old_gps: *mut RrosTpSchedule;
        let mut param = RrosSchedParam::new();
        // assert_hard_lock(&rq->lock);
        // if (RROS_WARN_ON(CORE, gps != NULL &&
        //     (gps->pwin_nr <= 0 || gps->pwins[0].w_offset != 0)))
        //     return tp->gps;
        stop_tp_schedule(rq)?;
        if tp.threads.clone().as_mut().unwrap().is_empty() == true {
            old_gps = tp.gps;
            tp.gps = gps;
            return Ok(old_gps);
        }

        for i in 1..=tp.threads.clone().as_mut().unwrap().len() {
            thread = tp
                .threads
                .clone()
                .as_mut()
                .unwrap()
                .get_by_index(i)
                .unwrap()
                .value
                .clone();
            param.fifo.prio = (*thread.locked_data().get()).cprio;
            rros_set_thread_schedparam_locked(
                thread.clone(),
                Some(&RROS_SCHED_FIFO),
                Some(Arc::try_new(SpinLock::new(param))?),
            )?;
        }
        old_gps = tp.gps;
        tp.gps = gps;
        return Ok(old_gps);
    }
}

pub fn get_tp_schedule(rq: *mut rros_rq) -> *mut RrosTpSchedule {
    let gps = unsafe { (*rq).tp.gps };

    // assert_hard_lock(&rq->lock);

    if gps == 0 as *mut RrosTpSchedule {
        return 0 as *mut RrosTpSchedule;
    }

    unsafe { (*(*gps).refcount).atomic_inc() };

    return gps;
}

pub fn put_tp_schedule(gps: *mut RrosTpSchedule) {
    unsafe {
        if (*(*gps).refcount).atomic_dec_and_test() != false {
            RROS_SYSTEM_HEAP.rros_free_chunk(gps as *mut u8);
        }
    }
}

pub fn tp_control(
    cpu: i32,
    ctlp: *mut RrosSchedCtlparam,
    infp: *mut RrosSchedCtlinfo,
) -> Result<SsizeT> {
    let pt = unsafe { &(*ctlp).tp };
    let mut offset: KtimeT;
    let mut duration: KtimeT;
    let mut next_offset: KtimeT = 0;
    let gps;
    let ogps: *mut RrosTpSchedule;
    let mut p: *mut RrosSchedTpWindow = 0 as *mut RrosSchedTpWindow;
    let mut pp: *mut RrosSchedTpWindow;
    let mut w: *mut RrosTpWindow = 0 as *mut RrosTpWindow;
    let mut pw: *mut RrosTpWindow;
    let mut it: *mut RrosTpCtlinfo;
    let rq: *mut rros_rq;
    let mut n: i32 = 0;
    let nr_windows: i32;

    if cpu < 0 || !cpu_present(cpu) || !is_threading_cpu(cpu) {
        return Err(kernel::Error::EINVAL);
    }

    rq = rros_cpu_rq(cpu);

    // raw_spin_lock_irqsave(&rq->lock, flags);
    unsafe {
        match pt.op {
            0 => {
                if pt.nr_windows > 0 {
                    // raw_spin_unlock_irqrestore(&rq->lock, flags);
                    // TODO Sizeof
                    gps = RROS_SYSTEM_HEAP.rros_alloc_chunk((8 + pt.nr_windows * 8) as usize);
                    if gps == None {
                        return Err(kernel::Error::ENOMEM);
                    }
                    let mut gps = gps.unwrap() as *mut RrosTpSchedule;
                    let mut loop_n = 0;
                    loop {
                        if loop_n == 0 {
                            n = 0;
                            p = pt.windows;
                            w = &mut (*gps).pwins[0] as *mut RrosTpWindow;
                            next_offset = 0;
                        } else {
                            n += 1;
                            p = p.offset(1);
                            w = w.offset(1);
                        }
                        if n >= pt.nr_windows {
                            break;
                        }
                        offset = timespec64_to_ktime(*(*p).offset);
                        if offset != next_offset {
                            RROS_SYSTEM_HEAP.rros_free_chunk(gps as *mut u8);
                            return Err(kernel::Error::EINVAL);
                        }

                        duration = timespec64_to_ktime(*(*p).duration);
                        if duration <= 0 {
                            RROS_SYSTEM_HEAP.rros_free_chunk(gps as *mut u8);
                            return Err(kernel::Error::EINVAL);
                        }

                        if (*p).ptid < -1 || (*p).ptid >= CONFIG_RROS_SCHED_TP_NR_PART {
                            RROS_SYSTEM_HEAP.rros_free_chunk(gps as *mut u8);
                            return Err(kernel::Error::EINVAL);
                        }

                        (*w).w_offset = next_offset;
                        (*w).w_part = (*p).ptid;
                        next_offset = ktime::ktime_add(next_offset, duration);
                        loop_n += 1;
                    }
                    (*(*gps).refcount).atomic_set(1);
                    (*gps).pwin_nr = n;
                    (*gps).tf_duration = next_offset;
                    // raw_spin_lock_irqsave(&rq->lock, flags);

                    ogps = set_tp_schedule(rq, gps).unwrap();
                    // raw_spin_unlock_irqrestore(&rq->lock, flags);
                    if ogps != 0 as *mut RrosTpSchedule {
                        put_tp_schedule(ogps);
                    }
                    rros_schedule();
                    return Ok(0);
                }
                let gps = 0 as *mut RrosTpSchedule;
                ogps = set_tp_schedule(rq, gps).unwrap();
                // raw_spin_unlock_irqrestore(&rq->lock, flags);
                if ogps != 0 as *mut RrosTpSchedule {
                    put_tp_schedule(ogps);
                }
                rros_schedule();
                return Ok(0);
            }
            1 => {
                let gps = 0 as *mut RrosTpSchedule;
                ogps = set_tp_schedule(rq, gps).unwrap();
                // raw_spin_unlock_irqrestore(&rq->lock, flags);
                if ogps != 0 as *mut RrosTpSchedule {
                    put_tp_schedule(ogps);
                }
                rros_schedule();
                return Ok(0);
            }
            2 => {
                start_tp_schedule(rq);
                // raw_spin_unlock_irqrestore(&rq->lock, flags);
                rros_schedule();
                return Ok(0);
            }
            3 => {
                stop_tp_schedule(rq)?;
                // raw_spin_unlock_irqrestore(&rq->lock, flags);
                rros_schedule();
                return Ok(0);
            }
            4 => (),
            _ => return Err(kernel::Error::EINVAL),
        }

        let gps = get_tp_schedule(rq);
        // raw_spin_unlock_irqrestore(&rq->lock, flags);
        if gps == 0 as *mut RrosTpSchedule {
            rros_schedule();
            return Ok(0);
        }

        if infp == 0 as *mut RrosSchedCtlinfo {
            put_tp_schedule(gps);
            return Err(kernel::Error::EINVAL);
        }

        it = &mut (*infp).tp as *mut RrosTpCtlinfo;
        if pt.nr_windows < (*gps).pwin_nr {
            nr_windows = pt.nr_windows;
        } else {
            nr_windows = (*gps).pwin_nr;
        }
        (*it).nr_windows = (*gps).pwin_nr;
        let mut loop_n = 0;
        loop {
            if loop_n == 0 {
                n = 0;
                p = (*it).windows;
                pp = p;
                w = &mut (*gps).pwins[0] as *mut RrosTpWindow;
                pw = w;
            } else {
                pp = p;
                p = p.offset(1);
                pw = w;
                w = w.offset(1);
                n += 1;
            }
            if n >= nr_windows {
                break;
            }
            (*p).offset = &mut ktime_to_timespec64((*w).w_offset) as *mut Timespec64;
            (*pp).duration =
                &mut ktime_to_timespec64(ktime::ktime_sub((*w).w_offset, (*pw).w_offset))
                    as *mut Timespec64;
            (*p).ptid = (*w).w_part;
            loop_n += 1;
        }
        (*pp).duration =
            &mut ktime_to_timespec64(ktime::ktime_sub((*gps).tf_duration, (*pw).w_offset))
                as *mut Timespec64;
        put_tp_schedule(gps);
        let ret = size_of::<RrosTpCtlinfo>() + size_of::<RrosTpWindow>() * nr_windows as usize;
        return Ok(ret as i64);
    }
}

pub fn rros_timer_is_running(timer: *mut RrosTimer) -> bool {
    unsafe {
        if (*timer).get_status() & RROS_TIMER_RUNNING != 0 {
            return true;
        } else {
            return false;
        }
    }
}

#[allow(dead_code)]
pub fn test_tp() {
    // pr_debug!("test_tp in ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
}

#[cfg(CONFIG_SMP)]
fn cpu_present(cpu: i32) -> bool {
    return cpu == 0;
}

#[cfg(not(CONFIG_SMP))]
fn cpu_present(cpu: i32) -> bool {
    return cpu == 0;
}
