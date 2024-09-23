use crate::sched;
use kernel::prelude::*;

#[allow(dead_code)]
pub fn test_this_rros_rq_thread() -> Result<usize> {
    pr_debug!("~~~test_this_rros_rq_thread begin~~~");
    let curr = sched::this_rros_rq_thread();
    match curr {
        None => {
            pr_debug!("curr is None");
        }
        Some(_x) => {
            pr_debug!("curr is not None ");
        }
    };
    pr_debug!("~~~test_this_rros_rq_thread end~~~");
    Ok(0)
}

#[allow(dead_code)]
pub fn test_cpu_smp() -> Result<usize> {
    pr_debug!("~~~test_cpu_smp begin~~~");
    let rq = sched::this_rros_rq();
    unsafe {
        pr_debug!("cpu is {}", (*rq).cpu);
    }
    pr_debug!("~~~test_cpu_smp end~~~");
    Ok(0)
}

#[allow(dead_code)]
pub fn test_rros_set_resched() -> Result<usize> {
    pr_debug!("~~~test_rros_set_resched begin~~~");
    let rq = sched::this_rros_rq();
    unsafe {
        pr_debug!("before this_rros_rq flags is {}", (*rq).flags);
    }
    sched::rros_set_resched(Some(rq));
    unsafe {
        pr_debug!("after this_rros_rq flags is {}", (*rq).flags);
    }
    pr_debug!("~~~test_rros_set_resched end~~~");
    Ok(0)
}

#[allow(dead_code)]
pub fn test_rros_in_irq() -> Result<usize> {
    pr_debug!("~~~test_rros_set_resched begin~~~");
    let rq = sched::this_rros_rq();
    unsafe {
        pr_debug!("before this_rros_rq flags is {}", (*rq).flags);
    }
    sched::rros_set_resched(Some(rq));
    unsafe {
        pr_debug!("after this_rros_rq flags is {}", (*rq).flags);
    }
    pr_debug!("~~~test_rros_set_resched end~~~");
    Ok(0)
}
