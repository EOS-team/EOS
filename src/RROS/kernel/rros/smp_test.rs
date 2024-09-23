use crate::{flags::RrosFlag, thread::KthreadRunner};

use kernel::{c_str, prelude::*};

static mut FLAG_PTR: *mut RrosFlag = 0 as *mut RrosFlag;

static mut SMP_KTHREAD_RUNNER_1: KthreadRunner = KthreadRunner::new_empty();
static mut SMP_KTHREAD_RUNNER_2: KthreadRunner = KthreadRunner::new_empty();
static mut SMP_KTHREAD_RUNNER_3: KthreadRunner = KthreadRunner::new_empty();
static mut SMP_KTHREAD_RUNNER_4: KthreadRunner = KthreadRunner::new_empty();
static mut SMP_KTHREAD_RUNNER_5: KthreadRunner = KthreadRunner::new_empty();
static mut SMP_KTHREAD_RUNNER_6: KthreadRunner = KthreadRunner::new_empty();

fn kthread_fn(id: i32) {
    unsafe {
        (*FLAG_PTR).wait_same();
    }

    let (mut first, mut second) = (1, 1);
    const M: i64 = 1_000_000_007;
    for _ in 0..5_000_000_0 {
        let temp = (first + second) % M;
        first = second;
        second = temp;
    }
    pr_warn!(
        "[smp_test]: kthread id is {:?}, the calculation result is {:?}",
        id,
        second
    );
}

#[allow(dead_code)]
pub fn smp_test_parallel_execution() {
    let smp_flag = Box::try_new(RrosFlag::new()).unwrap();

    unsafe {
        FLAG_PTR = Box::into_raw(smp_flag);

        SMP_KTHREAD_RUNNER_1.init(Box::try_new(|| kthread_fn(1)).unwrap());
        SMP_KTHREAD_RUNNER_1.run(c_str!("smp_test_kthread_1"));

        SMP_KTHREAD_RUNNER_2.init(Box::try_new(|| kthread_fn(2)).unwrap());
        SMP_KTHREAD_RUNNER_2.run(c_str!("smp_test_kthread_2"));

        SMP_KTHREAD_RUNNER_3.init(Box::try_new(|| kthread_fn(3)).unwrap());
        SMP_KTHREAD_RUNNER_3.run(c_str!("smp_test_kthread_3"));

        SMP_KTHREAD_RUNNER_4.init(Box::try_new(|| kthread_fn(4)).unwrap());
        SMP_KTHREAD_RUNNER_4.run(c_str!("smp_test_kthread_4"));

        SMP_KTHREAD_RUNNER_5.init(Box::try_new(|| kthread_fn(5)).unwrap());
        SMP_KTHREAD_RUNNER_5.run(c_str!("smp_test_kthread_5"));

        SMP_KTHREAD_RUNNER_6.init(Box::try_new(|| kthread_fn(6)).unwrap());
        SMP_KTHREAD_RUNNER_6.run(c_str!("smp_test_kthread_6"));

        (*FLAG_PTR).raise();
    }
}
