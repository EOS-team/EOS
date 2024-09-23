use crate::{clock::*, sched::*, timer::*};
use kernel::{prelude::*, spinlock_init, sync::SpinLock};

#[allow(dead_code)]
pub fn test_rros_insert_tnode() -> Result<usize> {
    pr_debug!("~~~test_rros_insert_tnode begin~~~");
    unsafe {
        let tmb = rros_percpu_timers(&RROS_MONO_CLOCK, 0);
        let mut x = SpinLock::new(RrosTimer::new(12));
        let pinned = Pin::new_unchecked(&mut x);
        spinlock_init!(pinned, "x");
        let mut y = SpinLock::new(RrosTimer::new(2));
        let pinned = Pin::new_unchecked(&mut y);
        spinlock_init!(pinned, "y");
        let mut z = SpinLock::new(RrosTimer::new(31));
        let pinned = Pin::new_unchecked(&mut z);
        spinlock_init!(pinned, "z");
        let mut a = SpinLock::new(RrosTimer::new(14));
        let pinned = Pin::new_unchecked(&mut a);
        spinlock_init!(pinned, "a");

        let xx = Arc::try_new(x)?;
        let yy = Arc::try_new(y)?;
        let zz = Arc::try_new(z)?;
        let aa = Arc::try_new(a)?;

        pr_debug!("before enqueue_by_index");
        rros_insert_tnode(&mut (*tmb).q, xx);
        rros_insert_tnode(&mut (*tmb).q, yy);
        rros_insert_tnode(&mut (*tmb).q, zz);
        rros_insert_tnode(&mut (*tmb).q, aa);

        pr_debug!("len is {}", (*tmb).q.len());

        for i in 1..=(*tmb).q.len() {
            let mut _x = (*tmb).q.get_by_index(i).unwrap().value.clone();
            pr_debug!("data of x is {}", _x.lock().get_date());
        }
    }
    pr_debug!("~~~test_rros_insert_tnode end~~~");
    Ok(0)
}

#[allow(dead_code)]
pub fn test_rros_enqueue_timer() -> Result<usize> {
    pr_debug!("~~~test_rros_insert_tnode begin~~~");
    unsafe {
        let tmb = rros_percpu_timers(&RROS_MONO_CLOCK, 0);
        let mut x = SpinLock::new(RrosTimer::new(12));
        let pinned = Pin::new_unchecked(&mut x);
        spinlock_init!(pinned, "x");
        let mut y = SpinLock::new(RrosTimer::new(2));
        let pinned = Pin::new_unchecked(&mut y);
        spinlock_init!(pinned, "y");
        let mut z = SpinLock::new(RrosTimer::new(31));
        let pinned = Pin::new_unchecked(&mut z);
        spinlock_init!(pinned, "z");
        let mut a = SpinLock::new(RrosTimer::new(14));
        let pinned = Pin::new_unchecked(&mut a);
        spinlock_init!(pinned, "a");

        let xx = Arc::try_new(x)?;
        let yy = Arc::try_new(y)?;
        let zz = Arc::try_new(z)?;
        let aa = Arc::try_new(a)?;

        pr_debug!("before enqueue_by_index");
        rros_enqueue_timer(xx, &mut (*tmb).q);
        rros_enqueue_timer(yy, &mut (*tmb).q);
        rros_enqueue_timer(zz, &mut (*tmb).q);
        rros_enqueue_timer(aa, &mut (*tmb).q);

        pr_debug!("len is {}", (*tmb).q.len());

        for i in 1..=(*tmb).q.len() {
            let mut _x = (*tmb).q.get_by_index(i).unwrap().value.clone();
            pr_debug!("data of x is {}", _x.lock().get_date());
        }
        pr_debug!("qufan RROS_TIMER_DEQUEUED is {}", !RROS_TIMER_DEQUEUED);
    }
    pr_debug!("~~~test_rros_insert_tnode end~~~");
    Ok(0)
}

#[allow(dead_code)]
pub fn test_rros_get_timer_gravity() -> Result<usize> {
    pr_debug!("~~~test_rros_get_timer_gravity begin~~~");
    unsafe {
        let mut x = SpinLock::new(RrosTimer::new(1));
        let pinned = Pin::new_unchecked(&mut x);
        spinlock_init!(pinned, "x");
        let xx = Arc::try_new(x)?;
        xx.lock().set_clock(&mut RROS_MONO_CLOCK as *mut RrosClock);

        xx.lock().set_status(RROS_TIMER_KGRAVITY);
        pr_debug!("kernel gravity is {}", rros_get_timer_gravity(xx.clone()));

        xx.lock().set_status(RROS_TIMER_UGRAVITY);
        pr_debug!("user gravity is {}", rros_get_timer_gravity(xx.clone()));

        xx.lock().set_status(0);
        pr_debug!("irq gravity is {}", rros_get_timer_gravity(xx.clone()));
    }
    pr_debug!("~~~test_rros_get_timer_gravity end~~~");
    Ok(0)
}

#[allow(dead_code)]
pub fn test_rros_update_timer_date() -> Result<usize> {
    pr_debug!("~~~test_rros_update_timer_date begin~~~");
    unsafe {
        let mut x = SpinLock::new(RrosTimer::new(1));
        let pinned = Pin::new_unchecked(&mut x);
        spinlock_init!(pinned, "x");
        let xx = Arc::try_new(x)?;
        xx.lock().set_clock(&mut RROS_MONO_CLOCK as *mut RrosClock);

        xx.lock().set_start_date(2);
        xx.lock().set_periodic_ticks(3);
        xx.lock().set_interval(8);
        xx.lock().set_status(RROS_TIMER_UGRAVITY);

        rros_update_timer_date(xx.clone());
        pr_debug!("xx date is {}", xx.lock().get_date());
    }
    pr_debug!("~~~test_rros_update_timer_date end~~~");
    Ok(0)
}

#[allow(dead_code)]
pub fn test_rros_get_timer_next_date() -> Result<usize> {
    pr_debug!("~~~test_rros_get_timer_next_date begin~~~");
    unsafe {
        let mut x = SpinLock::new(RrosTimer::new(1));
        let pinned = Pin::new_unchecked(&mut x);
        spinlock_init!(pinned, "x");
        let xx = Arc::try_new(x)?;

        xx.lock().set_start_date(2);
        xx.lock().set_periodic_ticks(3);
        xx.lock().set_interval(8);

        pr_debug!("xx next date is {}", rros_get_timer_next_date(xx.clone()));
    }
    pr_debug!("~~~test_rros_get_timer_next_date end~~~");
    Ok(0)
}

#[allow(dead_code)]
pub fn test_timer_at_front() -> Result<usize> {
    unsafe {
        let tmb = rros_percpu_timers(&RROS_MONO_CLOCK, 0);
        let mut x = SpinLock::new(RrosTimer::new(1));
        let pinned = Pin::new_unchecked(&mut x);
        spinlock_init!(pinned, "x");
        let mut y = SpinLock::new(RrosTimer::new(2));
        let pinned = Pin::new_unchecked(&mut y);
        spinlock_init!(pinned, "y");
        let mut z = SpinLock::new(RrosTimer::new(3));
        let pinned = Pin::new_unchecked(&mut z);
        spinlock_init!(pinned, "z");

        let xx = Arc::try_new(x)?;
        let yy = Arc::try_new(y)?;
        let zz = Arc::try_new(z)?;

        xx.lock().set_base(tmb);
        yy.lock().set_base(tmb);
        zz.lock().set_base(tmb);
        let mut _rq = rros_rq::new()?;
        let rq = &mut _rq as *mut rros_rq;

        xx.lock().set_rq(rq);
        yy.lock().set_rq(rq);
        zz.lock().set_rq(rq);
        (*tmb).q.add_head(xx.clone());
        (*tmb).q.add_head(yy.clone());
        (*tmb).q.add_head(zz.clone());

        // Test the first if branch.
        if timer_at_front(zz.clone()) == true {
            pr_debug!("test_timer_at_front if1 true");
        } else {
            pr_debug!("test_timer_at_front if1 false");
        }

        // Test the second if branch.
        if timer_at_front(yy.clone()) == true {
            pr_debug!("test_timer_at_front if2 true");
        } else {
            pr_debug!("test_timer_at_front if2 false");
        }
    }
    Ok(0)
}

#[allow(dead_code)]
pub fn test_rros_timer_deactivate() -> Result<usize> {
    unsafe {
        let tmb = rros_percpu_timers(&RROS_MONO_CLOCK, 0);
        let mut x = SpinLock::new(RrosTimer::new(1));
        let pinned = Pin::new_unchecked(&mut x);
        spinlock_init!(pinned, "x");
        let mut y = SpinLock::new(RrosTimer::new(2));
        let pinned = Pin::new_unchecked(&mut y);
        spinlock_init!(pinned, "y");
        let mut z = SpinLock::new(RrosTimer::new(3));
        let pinned = Pin::new_unchecked(&mut z);
        spinlock_init!(pinned, "z");

        let xx = Arc::try_new(x)?;
        let yy = Arc::try_new(y)?;
        let zz = Arc::try_new(z)?;

        xx.lock().set_base(tmb);
        yy.lock().set_base(tmb);
        zz.lock().set_base(tmb);

        let mut _rq = rros_rq::new()?;
        let rq = &mut _rq as *mut rros_rq;

        xx.lock().set_rq(rq);
        yy.lock().set_rq(rq);
        zz.lock().set_rq(rq);
        (*tmb).q.add_head(xx.clone());
        (*tmb).q.add_head(yy.clone());
        (*tmb).q.add_head(zz.clone());

        zz.lock().set_status(RROS_TIMER_DEQUEUED);

        if rros_timer_deactivate(zz.clone()) {
            pr_debug!("test_rros_timer_deactivate: success");
        } else {
            pr_debug!("test_rros_timer_deactivate: failed");
        }

        pr_debug!(
            "test_rros_timer_deactivate: len of tmb is {}",
            (*tmb).q.len()
        );
    }
    Ok(0)
}

#[allow(dead_code)]
pub fn test_rros_get_timer_expiry() -> Result<usize> {
    pr_debug!("~~~test_rros_get_timer_expiry begin~~~");
    unsafe {
        let mut x = SpinLock::new(RrosTimer::new(1));
        let pinned = Pin::new_unchecked(&mut x);
        spinlock_init!(pinned, "x");
        let xx = Arc::try_new(x)?;
        xx.lock().set_clock(&mut RROS_MONO_CLOCK as *mut RrosClock);

        xx.lock().set_date(11);
        xx.lock().set_status(0);

        pr_debug!("xx next date is {}", rros_get_timer_expiry(xx.clone()));
    }
    pr_debug!("~~~test_rros_get_timer_expiry end~~~");
    Ok(0)
}

#[allow(dead_code)]
pub fn test_rros_get_timer_delta() -> Result<usize> {
    pr_debug!("~~~test_rros_get_timer_delta begin~~~");
    unsafe {
        let mut x = SpinLock::new(RrosTimer::new(1));
        let pinned = Pin::new_unchecked(&mut x);
        spinlock_init!(pinned, "x");
        let xx = Arc::try_new(x)?;
        xx.lock().set_clock(&mut RROS_MONO_CLOCK as *mut RrosClock);

        xx.lock().set_date(1111111111111);
        xx.lock().set_status(RROS_TIMER_RUNNING);

        pr_debug!("xx delta is {}", rros_get_timer_delta(xx.clone()));

        xx.lock().set_date(0);
        xx.lock().set_status(RROS_TIMER_PERIODIC);

        pr_debug!("xx delta is {}", rros_get_timer_delta(xx.clone()));
    }
    pr_debug!("~~~test_rros_get_timer_delta end~~~");
    Ok(0)
}

#[allow(dead_code)]
pub fn test_rros_get_timer_date() -> Result<usize> {
    pr_debug!("~~~test_rros_get_timer_date begin~~~");
    unsafe {
        let mut x = SpinLock::new(RrosTimer::new(1));
        let pinned = Pin::new_unchecked(&mut x);
        spinlock_init!(pinned, "x");
        let xx = Arc::try_new(x)?;
        xx.lock().set_clock(&mut RROS_MONO_CLOCK as *mut RrosClock);

        xx.lock().set_date(11);
        xx.lock().set_status(RROS_TIMER_RUNNING);

        pr_debug!("xx next date is {}", rros_get_timer_date(xx.clone()));

        xx.lock().set_status(RROS_TIMER_PERIODIC);
        pr_debug!("xx next date is {}", rros_get_timer_date(xx.clone()));
    }
    pr_debug!("~~~test_rros_get_timer_date end~~~");
    Ok(0)
}

#[allow(dead_code)]
pub fn test_program_timer() -> Result<usize> {
    pr_debug!("~~~test_program_timer begin~~~");
    unsafe {
        let tmb = rros_percpu_timers(&RROS_MONO_CLOCK, 0);
        let mut x = SpinLock::new(RrosTimer::new(1));
        let pinned = Pin::new_unchecked(&mut x);
        spinlock_init!(pinned, "x");

        let xx = Arc::try_new(x)?;

        let mut _rq = rros_rq::new()?;
        let rq = &mut _rq as *mut rros_rq;

        xx.lock().set_clock(&mut RROS_MONO_CLOCK as *mut RrosClock);
        xx.lock().set_rq(rq);
        xx.lock().set_base(tmb);
        let tmb1 = xx.lock().get_base();

        program_timer(xx.clone(), &mut (*tmb1).q);

        pr_debug!("len of tmb is {}", (*tmb).q.len());
    }
    pr_debug!("~~~test_program_timer end~~~");
    Ok(0)
}

#[allow(dead_code)]
pub fn test_rros_start_timer() -> Result<usize> {
    pr_debug!("~~~test_rros_start_timer begin~~~");
    unsafe {
        let tmb = rros_percpu_timers(&RROS_MONO_CLOCK, 0);
        let mut x = SpinLock::new(RrosTimer::new(17));
        let pinned = Pin::new_unchecked(&mut x);
        spinlock_init!(pinned, "x");

        let xx = Arc::try_new(x)?;

        let mut _rq = rros_rq::new()?;
        let rq = &mut _rq as *mut rros_rq;

        xx.lock().set_clock(&mut RROS_MONO_CLOCK as *mut RrosClock);
        xx.lock().set_rq(rq);
        xx.lock().set_base(tmb);
        pr_debug!("before program_timer");
        rros_start_timer(xx.clone(), 333, 222);

        pr_debug!("timer date is {}", xx.lock().get_date());
        pr_debug!("timer start date is {}", xx.lock().get_start_date());
        pr_debug!("timer interval is {}", xx.lock().get_interval());
        pr_debug!("timer status is {}", xx.lock().get_status());

        pr_debug!("len of tmb is {}", (*tmb).q.len());
    }
    pr_debug!("~~~test_rros_start_timer end~~~");
    Ok(0)
}

#[allow(dead_code)]
pub fn test_stop_timer_locked() -> Result<usize> {
    pr_debug!("~~~test_stop_timer_locked begin~~~");
    unsafe {
        let tmb = rros_percpu_timers(&RROS_MONO_CLOCK, 0);
        let mut x = SpinLock::new(RrosTimer::new(17));
        let pinned = Pin::new_unchecked(&mut x);
        spinlock_init!(pinned, "x");

        let xx = Arc::try_new(x)?;

        let mut _rq = rros_rq::new()?;
        let rq = &mut _rq as *mut rros_rq;

        xx.lock().set_clock(&mut RROS_MONO_CLOCK as *mut RrosClock);
        xx.lock().set_rq(rq);
        xx.lock().set_base(tmb);
        xx.lock().set_status(RROS_TIMER_RUNNING);
        pr_debug!("before stop_timer_locked");
        stop_timer_locked(xx.clone());
        pr_debug!("len of tmb is {}", (*tmb).q.len());
    }
    pr_debug!("~~~test_stop_timer_locked end~~~");
    Ok(0)
}

#[allow(dead_code)]
pub fn test_rros_destroy_timer() -> Result<usize> {
    pr_debug!("~~~test_rros_destroy_timer begin~~~");
    unsafe {
        let tmb = rros_percpu_timers(&RROS_MONO_CLOCK, 0);
        let mut x = SpinLock::new(RrosTimer::new(17));
        let pinned = Pin::new_unchecked(&mut x);
        spinlock_init!(pinned, "x");

        let xx = Arc::try_new(x)?;

        let mut _rq = rros_rq::new()?;
        let rq = &mut _rq as *mut rros_rq;

        xx.lock().set_clock(&mut RROS_MONO_CLOCK as *mut RrosClock);
        xx.lock().set_rq(rq);
        xx.lock().set_base(tmb);
        xx.lock().set_status(RROS_TIMER_RUNNING);
        pr_debug!("before rros_destroy_timer");
        rros_destroy_timer(xx.clone());
        let xx_lock_rq = xx.lock().get_rq();
        let xx_lock_base = xx.lock().get_base();
        if xx_lock_rq.is_null() {
            pr_debug!("xx rq is none");
        }
        if xx_lock_base == 0 as *mut RrosTimerbase {
            pr_debug!("xx base is none");
        }
        pr_debug!("len of tmb is {}", (*tmb).q.len());
    }
    pr_debug!("~~~test_rros_destroy_timer end~~~");
    Ok(0)
}

#[allow(dead_code)]
pub fn handler(_timer: &RrosTimer) {
    pr_info!("success");
}

#[allow(dead_code)]
pub fn test_get_handler() -> Result<usize> {
    pr_debug!("~~~test_get_handler begin~~~");
    unsafe {
        let mut x = SpinLock::new(RrosTimer::new(17));
        let pinned = Pin::new_unchecked(&mut x);
        spinlock_init!(pinned, "x");

        let mut _xx = Arc::try_new(x)?;

        //xx.lock().set_handler(Some(handler));
        //let handler = xx.lock().get_handler();
        //handler(xx.lock().deref());
    }
    pr_debug!("~~~test_get_handler end~~~");
    Ok(0)
}
