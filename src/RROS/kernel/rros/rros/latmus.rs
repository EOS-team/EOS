use crate::{
    clock,
    thread::{self, KthreadRunner},
};
use kernel::prelude::*;
use kernel::{c_str, c_types, ktime};

static mut KTHREAD_RUNNER_1: KthreadRunner = KthreadRunner::new_empty();

fn kthread_handler(ptr: *mut c_types::c_void) {
    // thread::rros_sleep(10000000000);
    let k_runner = unsafe { &mut *(ptr as *mut KthreadRunner) };
    // void kthread_handler(void *arg)
    // {
    // 	struct kthread_runner *k_runner = arg;
    // 	ktime_t now;
    // 	int ret = 0;
    // TODO: add the ret value
    let _kernel_lantency = [0; 1000];
    let mut _ret = 0;

    loop {
        // TODO: add the should stop function
        // if thread::should_stop() {
        //     break;
        // }

        // TODO: add the wait flag function
        // ret = rros_wait_flag(&k_runner->barrier);
        // if (ret)
        //     break;

        // TODO: change the runner period flag when change
        thread::rros_set_period(
            unsafe { &mut clock::RROS_MONO_CLOCK },
            k_runner.1 as i64,
            k_runner.2.period as i64,
            1,
        );

        // TODO: error handle
        // if (ret)
        //     break;

        for _i in 0..10 {
            pr_debug!("I'm going to wait\n");
            let res = thread::rros_wait_period();
            match res {
                Ok(_) => (),
                Err(_) => {
                    pr_err!("there is an error\n");
                }
            }
            pr_debug!("I'm in the loop\n");
            // if (ret && ret != -ETIMEDOUT) {
            //     // done_sampling(&k_runner.runner, ret);
            //     rros_stop_kthread(&k_runner.kthread);
            //     return;
            // }

            let now = unsafe { clock::rros_read_clock(&clock::RROS_MONO_CLOCK) };
            // if (k_runner.runner.add_sample(&k_runner.runner, now)) {
            if add_measurement_sample(k_runner, now) == 1 {
                unsafe {
                    thread::rros_set_period(&mut clock::RROS_MONO_CLOCK, 0, 0, 0);
                }
                break;
            }
        }

        break;
    }

    pr_debug!(
        "k_runner.2.state.min_latency: {}\n",
        k_runner.2.state.min_latency
    );
    pr_debug!(
        "k_runner.2.state.max_latency: {}\n",
        k_runner.2.state.max_latency
    );
    pr_debug!(
        "k_runner.2.state.avg_latency: {}\n",
        k_runner.2.state.avg_latency
    );
    // 	for (;;) {
    // 		if (rros_kthread_should_stop())
    // 			break;

    // 		ret = rros_wait_flag(&k_runner->barrier);
    // 		if (ret)
    // 			break;

    // 		ret = rros_set_period(&rros_mono_clock,
    // 				k_runner->start_time,
    // 				k_runner->runner.period);
    // 		if (ret)
    // 			break;

    // 		for (;;) {
    // 			ret = rros_wait_period(NULL);
    // 			if (ret && ret != -ETIMEDOUT)
    // 				goto out;

    // 			now = rros_read_clock(&rros_mono_clock);
    // 			if (k_runner->runner.add_sample(&k_runner->runner, now)) {
    // 				rros_set_period(NULL, 0, 0);
    // 				break;
    // 			}
    // 		}
    // 	}
    // out:
    // 	done_sampling(&k_runner->runner, ret);
    // 	rros_stop_kthread(&k_runner->kthread);
    // }
}

pub fn test_latmus() {
    unsafe {
        KTHREAD_RUNNER_1.init(
            Box::try_new(move || {
                let now = clock::rros_read_clock(&clock::RROS_MONO_CLOCK);
                KTHREAD_RUNNER_1.1 = now as u64;
                KTHREAD_RUNNER_1.2.period = 700000000;
                KTHREAD_RUNNER_1.2.state.ideal = now as u64;
                KTHREAD_RUNNER_1.2.state.offset = 0;
                kthread_handler(
                    &mut KTHREAD_RUNNER_1 as *mut KthreadRunner as *mut c_types::c_void,
                );
            })
            .unwrap(),
        );
        KTHREAD_RUNNER_1.run(c_str!("latmus_thread"));
    }
}

fn add_measurement_sample(runner: &mut KthreadRunner, timestamp: ktime::KtimeT) -> i32 {
    let period = runner.2.period as i64;
    let mut state = &mut runner.2.state;
    let mut delta = ktime::ktime_to_ns(ktime::ktime_sub(timestamp, state.ideal as i64)) as u64;
    pr_debug!("the delta is {}\n", delta);
    pr_debug!("the offset is {}\n", timestamp);
    pr_debug!("the ideal is {}\n", state.ideal);
    let offset_delta = (delta - state.offset) as u64;

    if offset_delta < state.min_latency {
        state.min_latency = offset_delta;
    } else if offset_delta > state.max_latency {
        state.max_latency = offset_delta;
    }

    pr_debug!("the offset_delta is {}\n", offset_delta);
    pr_debug!("the avg_latency is {}\n", state.avg_latency);
    state.avg_latency += offset_delta;
    state.ideal = ktime::ktime_add(state.ideal as i64, period) as u64;
    // else if offset_delta > state.allmax_lat {
    // state.allmax_lat = offset_delta;
    // trace_rros_latspot(offset_delta);
    // trace_rros_trigger("latmus");
    // }

    while delta > 0 && delta > (ktime::ktime_to_ns(period) as u64) {
        /* period > 0 */
        // let pexpect_ticks = unsafe{(*timer.locked_data().get()).get_pexpect_ticks() + 1};
        // unsafe{(*timer.locked_data().get()).set_pexpect_ticks(pexpect_ticks);}
        state.ideal = ktime::ktime_add(state.ideal as i64, period) as u64;
        delta -= ktime::ktime_to_ns(period) as u64;
    }

    0
}

// TODO: move this to a file
// struct Latmus;

// impl KernelModule for Latmus {
//     fn init() -> Result<Self> {
//         // unsafe{Arc::try_new(SpinLock::new(RrosThread::new().unwrap())).unwrap()},
//         unsafe{
//             KTHREAD_RUNNER_1.init(Box::try_new(move || {
//                 kthread_handler(&mut KTHREAD_RUNNER_1 as *mut KthreadRunner as *mut c_types::c_void);
//             }).unwrap());
//             KTHREAD_RUNNER_1.run(c_str!("latmus_thread"));
//         }

//         pr_debug!("Hello world from latmus!\n");
//         Ok(Rros)
//     }
// }

// impl Drop for Rros {
//     fn drop(&mut self) {
//         pr_debug!("Bye world from latmus!\n");
//     }
// }
