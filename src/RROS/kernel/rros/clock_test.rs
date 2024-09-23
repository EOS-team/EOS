use crate::{clock::*, timer::*};

use kernel::{ktime::*, prelude::*, spinlock_init, sync::SpinLock};

#[allow(dead_code)]
pub fn test_do_clock_tick() -> Result<usize> {
    pr_debug!("~~~test_do_clock_tick begin~~~");
    unsafe {
        let tmb = rros_percpu_timers(&RROS_MONO_CLOCK, 0);
        let mut a = SpinLock::new(RrosTimer::new(580000000));
        let pinned = Pin::new_unchecked(&mut a);
        spinlock_init!(pinned, "zbw");

        let xx = Arc::try_new(a)?;
        xx.lock().add_status(RROS_TIMER_DEQUEUED);
        xx.lock().add_status(RROS_TIMER_PERIODIC);
        xx.lock().add_status(RROS_TIMER_RUNNING);
        xx.lock().set_clock(&mut RROS_MONO_CLOCK as *mut RrosClock);
        xx.lock().set_interval(1000);

        (*tmb).q.add_head(xx.clone());

        pr_debug!("before do_clock_tick");
        do_clock_tick(&mut RROS_MONO_CLOCK, tmb);
        pr_debug!("len of tmb is {}", (*tmb).q.len());
    }
    pr_debug!("~~~test_do_clock_tick end~~~");
    Ok(0)
}

#[allow(dead_code)]
pub fn test_adjust_timer() -> Result<usize> {
    pr_debug!("~~~test_adjust_timer begin~~~");
    unsafe {
        let tmb = rros_percpu_timers(&RROS_MONO_CLOCK, 0);
        let mut a = SpinLock::new(RrosTimer::new(580000000));
        let pinned = Pin::new_unchecked(&mut a);
        spinlock_init!(pinned, "a");

        let xx = Arc::try_new(a)?;
        xx.lock().add_status(RROS_TIMER_DEQUEUED);
        xx.lock().add_status(RROS_TIMER_PERIODIC);
        xx.lock().add_status(RROS_TIMER_RUNNING);
        xx.lock().set_clock(&mut RROS_MONO_CLOCK as *mut RrosClock);
        xx.lock().set_interval(1000);

        // (*tmb).q.add_head(xx.clone());

        pr_debug!("before adjust_timer");
        adjust_timer(&RROS_MONO_CLOCK, xx.clone(), &mut (*tmb).q, 100);
        pr_debug!("len of tmb is {}", (*tmb).q.len());
    }
    pr_debug!("~~~test_adjust_timer end~~~");
    Ok(0)
}

#[allow(dead_code)]
pub fn test_rros_adjust_timers() -> Result<usize> {
    pr_debug!("~~~test_rros_adjust_timers begin~~~");
    unsafe {
        let tmb = rros_percpu_timers(&RROS_MONO_CLOCK, 0);
        let mut a = SpinLock::new(RrosTimer::new(580000000));
        let pinned = Pin::new_unchecked(&mut a);
        spinlock_init!(pinned, "a");

        let mut b = SpinLock::new(RrosTimer::new(580000000));
        let pinned = Pin::new_unchecked(&mut b);
        spinlock_init!(pinned, "b");

        let xx = Arc::try_new(a)?;
        let yy = Arc::try_new(b)?;

        let add1 = &mut xx.lock().start_date as *mut KtimeT;
        pr_debug!("add1 is {:p}", add1);

        let interval_add = &mut xx.lock().interval as *mut KtimeT;
        pr_debug!("add interval is {:p}", interval_add);

        let add2 = &mut xx.lock().start_date as *mut KtimeT;
        pr_debug!("add2 is {:p}", add2);

        // xx.lock().add_status(RROS_TIMER_FIRED);
        xx.lock().add_status(RROS_TIMER_PERIODIC);
        xx.lock().set_clock(&mut RROS_MONO_CLOCK as *mut RrosClock);
        xx.lock().set_interval(1000);

        yy.lock().add_status(RROS_TIMER_PERIODIC);
        yy.lock().set_clock(&mut RROS_MONO_CLOCK as *mut RrosClock);
        yy.lock().set_interval(1000);

        (*tmb).q.add_head(xx.clone());
        (*tmb).q.add_head(yy.clone());

        pr_debug!("before adjust_timer");
        rros_adjust_timers(&mut RROS_MONO_CLOCK, 100)?;
        pr_debug!("len of tmb is {}", (*tmb).q.len());
    }
    pr_debug!("~~~test_rros_adjust_timers end~~~");
    Ok(0)
}

#[allow(dead_code)]
pub fn test_rros_stop_timers() -> Result<usize> {
    pr_debug!("~~~test_rros_stop_timers begin~~~");
    unsafe {
        let tmb = rros_percpu_timers(&RROS_MONO_CLOCK, 0);
        let mut a = SpinLock::new(RrosTimer::new(580000000));
        let pinned = Pin::new_unchecked(&mut a);
        spinlock_init!(pinned, "a");

        let mut b = SpinLock::new(RrosTimer::new(580000000));
        let pinned = Pin::new_unchecked(&mut b);
        spinlock_init!(pinned, "b");

        let xx = Arc::try_new(a)?;
        let yy = Arc::try_new(b)?;

        xx.lock().add_status(RROS_TIMER_PERIODIC);
        xx.lock().add_status(RROS_TIMER_DEQUEUED);
        xx.lock().set_clock(&mut RROS_MONO_CLOCK as *mut RrosClock);
        xx.lock().set_interval(1000);
        xx.lock().set_base(tmb);

        yy.lock().add_status(RROS_TIMER_PERIODIC);
        yy.lock().add_status(RROS_TIMER_DEQUEUED);
        yy.lock().set_clock(&mut RROS_MONO_CLOCK as *mut RrosClock);
        yy.lock().set_interval(1000);
        yy.lock().set_base(tmb);

        (*tmb).q.add_head(xx.clone());
        (*tmb).q.add_head(yy.clone());

        pr_debug!("before rros_adjust_timers");
        rros_stop_timers(&RROS_MONO_CLOCK);
        pr_debug!("len of tmb is {}", (*tmb).q.len());
    }
    pr_debug!("~~~test_rros_stop_timers end~~~");
    Ok(0)
}
