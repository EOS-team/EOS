use crate::{clock::*, timer::*};
use kernel::{prelude::*, spinlock_init, sync::SpinLock};

#[allow(dead_code)]
pub fn test_enqueue_by_index() -> Result<usize> {
    pr_debug!("~~~test_double_linked_list begin~~~");
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
        let mut a = SpinLock::new(RrosTimer::new(4));
        let pinned = Pin::new_unchecked(&mut a);
        spinlock_init!(pinned, "a");

        let xx = Arc::try_new(x)?;
        let yy = Arc::try_new(y)?;
        let zz = Arc::try_new(z)?;
        let aa = Arc::try_new(a)?;

        (*tmb).q.add_head(xx.clone());
        (*tmb).q.add_head(yy.clone());
        (*tmb).q.add_head(zz.clone());

        pr_debug!("before enqueue_by_index");
        (*tmb).q.enqueue_by_index(2, aa);
        pr_debug!("len is {}", (*tmb).q.len());

        for i in 1..=(*tmb).q.len() {
            let mut _x = (*tmb).q.get_by_index(i).unwrap().value.clone();
            pr_debug!("data of x is {}", _x.lock().get_date());
        }
    }
    pr_debug!("~~~test_double_linked_list end~~~");
    Ok(0)
}
