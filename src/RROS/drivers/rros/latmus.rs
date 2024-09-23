#![no_std]
#![feature(allocator_api, global_asm)]
use init::thread::{self, rros_sleep, KthreadRunner};
use rros::thread_test::KthreadRunner;

static mut KTHREAD_RUNNER_1: KthreadRunner = KthreadRunner::new_empty();

fn kthread_handler(ptr: *mut c_types::c_void) {
    let k_runner = unsafe { &mut *(ptr as *mut KthreadRunner) };
    // void kthread_handler(void *arg)
    // {
    // 	struct kthread_runner *k_runner = arg;
    // 	ktime_t now;
    // 	int ret = 0;
    // TODO: add the ret value
    let kernel_lantency = [0; 1000];
    let mut ret = 0;

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
        ret = thread::rros_set_period(
            &clock::RROS_MONO_CLOCK,
            k_runner.start_time,
            k_runner.runner.period,
        );

        // TODO: error handle
        // if (ret)
        //     break;

        loop {
            ret = timer::rros_wait_period(None);
            if (ret && ret != -ETIMEDOUT) {
                done_sampling(&k_runner.runner, ret);
                rros_stop_kthread(&k_runner.kthread);
                return;
            }

            now = timer::rros_read_clock(&clock::RROS_MONO_CLOCK);
            if (k_runner.runner.add_sample(&k_runner.runner, now)) {
                thread::rros_set_period(None, 0, 0);
                break;
            }
        }
    }
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

fn test_latmus() {
    KTHREAD_RUNNER_1.init(
        Box::try_new(move || {
            kthread_handler(&mut KTHREAD_RUNNER_1 as *mut KthreadRunner as *mut c_types::c_void);
        })
        .unwrap(),
    );
    KTHREAD_RUNNER_1.run(c_str!("latmus_thread"));
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

//         pr_info!("Hello world from latmus!\n");
//         Ok(Rros)
//     }
// }

// impl Drop for Rros {
//     fn drop(&mut self) {
//         pr_info!("Bye world from latmus!\n");
//     }
// }
