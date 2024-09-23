use crate::{sched::this_rros_rq, thread::KthreadRunner};

use kernel::{c_str, prelude::*};

// struct KthreadRunner {
//     pub kthread: thread::RrosKthread, // struct rros_kthread kthread;
//                                       // struct rros_flag barrier;
//                                       // KtimeT start_time;
//                                       // struct latmus_runner runner;
// }

// impl KthreadRunner {
//     pub fn new(kfn:Box<dyn FnOnce()>) -> Self {
//         // let mut thread = unsafe{SpinLock::new(thread::RrosKthread::new(kfn))};
//         // let pinned = unsafe{Pin::new_unchecked(&mut thread)};

//         // unsafe{Self { kthread: Arc::try_new(thread).unwrap()}}
//         unsafe {
//             Self {
//                 kthread: thread::RrosKthread::new(Some(kfn)),
//             }
//         }
//     }
// }

// // static mut KTHREAD_RUNNER_1:Option<> = ;
// static mut KTHREAD_RUNNER_1: Option<KthreadRunner> = None;
// static mut KTHREAD_RUNNER_2: Option<KthreadRunner> = None;
static mut KTHREAD_RUNNER_1: KthreadRunner = KthreadRunner::new_empty();
static mut KTHREAD_RUNNER_2: KthreadRunner = KthreadRunner::new_empty();

pub fn test_thread_context_switch() {
    let rq = this_rros_rq();

    // // let

    let _rq_len = unsafe { (*rq).fifo.runnable.head.clone().unwrap().len() };

    unsafe {
        KTHREAD_RUNNER_1.init(Box::try_new(kfn_1).unwrap());
        KTHREAD_RUNNER_2.init(Box::try_new(kfn_2).unwrap());
        KTHREAD_RUNNER_1.run(c_str!("kthread_1"));
        KTHREAD_RUNNER_2.run(c_str!("kthread_2"));
    }
    // pr_debug!("the init xxx length0 {}", rq_len);

    // unsafe{Arc::try_new(SpinLock::new(RrosThread::new().unwrap())).unwrap()},
    // unsafe {
    //     // let x = "hello";
    //     // let y = "world";
    //     // let kfn_3 = move||{
    //     //     loop{
    //     //         pr_debug!("{}",x);
    //     //         thread::rros_sleep(1000000000);
    //     //         pr_debug!("{}",y);
    //     //         thread::rros_sleep(1000000000);
    //     //     }
    //     // };
    //     KTHREAD_RUNNER_1 = Some(KthreadRunner::new(Box::try_new(kfn_1).unwrap()));
    //     let mut thread = SpinLock::new(sched::RrosThread::new().unwrap());
    //     let pinned = Pin::new_unchecked(&mut thread);
    //     spinlock_init!(pinned, "test_threads1");
    //     KTHREAD_RUNNER_1.as_mut().unwrap().kthread.thread = Some(Arc::try_new(thread).unwrap());

    //     let mut r = SpinLock::new(timer::RrosTimer::new(1));
    //     let pinned_r = Pin::new_unchecked(&mut r);
    //     spinlock_init!(pinned_r, "rtimer_1");

    //     let mut p = SpinLock::new(timer::RrosTimer::new(1));
    //     let pinned_p = Pin::new_unchecked(&mut p);
    //     spinlock_init!(pinned_p, "ptimer_1");

    //     KTHREAD_RUNNER_1
    //         .as_mut()
    //         .unwrap()
    //         .kthread
    //         .thread
    //         .as_mut()
    //         .unwrap()
    //         .lock()
    //         .rtimer = Some(Arc::try_new(r).unwrap());
    //     KTHREAD_RUNNER_1
    //         .as_mut()
    //         .unwrap()
    //         .kthread
    //         .thread
    //         .as_mut()
    //         .unwrap()
    //         .lock()
    //         .ptimer = Some(Arc::try_new(p).unwrap());

    //     // let mut tmb = timer::rros_percpu_timers(&clock::RROS_REALTIME_CLOCK, 0);
    //     // KTHREAD_RUNNER_1.as_mut().unwrap().kthread.thread.as_mut().unwrap().lock().rtimer.as_mut().unwrap().lock().set_base(tmb);
    //     // KTHREAD_RUNNER_1.as_mut().unwrap().kthread.thread.as_mut().unwrap().lock().ptimer.as_mut().unwrap().lock().set_base(tmb);

    //     let temp: &mut thread::RrosKthread;
    //     temp = &mut KTHREAD_RUNNER_1.as_mut().unwrap().kthread;
    //     thread::rros_run_kthread(temp, c_str!("hongyu1"));
    //     // thread::rros_run_kthread(&mut KTHREAD_RUNNER_1.as_mut().unwrap().kthread, c_str!("hongyu1")) ;
    // }

    // let rq_len = unsafe { (*rq).fifo.runnable.head.clone().unwrap().len() };
    // // pr_debug!("length1 {}", rq_len);

    // // // let
    // unsafe {
    //     KTHREAD_RUNNER_2 = Some(KthreadRunner::new(Box::try_new(kfn_2).unwrap()));
    //     let mut thread = SpinLock::new(sched::RrosThread::new().unwrap());
    //     let pinned: Pin<&mut SpinLock<sched::RrosThread>> = Pin::new_unchecked(&mut thread);
    //     spinlock_init!(pinned, "test_threads2");
    //     KTHREAD_RUNNER_2.as_mut().unwrap().kthread.thread = Some(Arc::try_new(thread).unwrap());

    //     let mut r = SpinLock::new(timer::RrosTimer::new(1));
    //     let pinned_r = Pin::new_unchecked(&mut r);
    //     spinlock_init!(pinned_r, "rtimer_2");

    //     let mut p = SpinLock::new(timer::RrosTimer::new(1));
    //     let pinned_p = Pin::new_unchecked(&mut p);
    //     spinlock_init!(pinned_p, "ptimer_2");

    //     KTHREAD_RUNNER_2
    //         .as_mut()
    //         .unwrap()
    //         .kthread
    //         .thread
    //         .as_mut()
    //         .unwrap()
    //         .lock()
    //         .rtimer = Some(Arc::try_new(r).unwrap());
    //     KTHREAD_RUNNER_2
    //         .as_mut()
    //         .unwrap()
    //         .kthread
    //         .thread
    //         .as_mut()
    //         .unwrap()
    //         .lock()
    //         .ptimer = Some(Arc::try_new(p).unwrap());

    //     // let mut tmb = timer::rros_percpu_timers(&clock::RROS_REALTIME_CLOCK, 0);
    //     // KTHREAD_RUNNER_2.as_mut().unwrap().kthread.thread.as_mut().unwrap().lock().rtimer.as_mut().unwrap().lock().set_base(tmb);
    //     // KTHREAD_RUNNER_2.as_mut().unwrap().kthread.thread.as_mut().unwrap().lock().ptimer.as_mut().unwrap().lock().set_base(tmb);

    //     pr_debug!("kfn_2 rros_run_kthread in");
    //     thread::rros_run_kthread(
    //         &mut KTHREAD_RUNNER_2.as_mut().unwrap().kthread,
    //         c_str!("hongyu2"),
    //     );
    //     pr_debug!("kfn_2 rros_run_kthread out");
    // }

    // let rq_len = unsafe { (*rq).fifo.runnable.head.clone().unwrap().len() };

    // pr_debug!("length2 {}", rq_len);

    // let mut a = 0 as u64;
    // while 1==1 {
    //     a += 1;
    // if a == 10000000000{
    //     break;
    // }
    // let a = 1;
    // pr_debug!("good");
    // if a == 10000000000{
    //     break;
    // }
    // let a = 1;
    // pr_debug!("good");
    // }
    // let mut RrosKthread1 = kthread::RrosKthread::new(fn1);
    // let mut RrosKthread2 = kthread::RrosKthread::new(fn2);
    // kthread::kthread_run(Some(threadfn), &mut RrosKthread1 as *mut kthread::RrosKthread as *mut c_types::c_void, c_str!("%s").as_char_ptr(),
    //  format_args!("hongyu1"));
    // kthread::kthread_run(Some(threadfn), &mut RrosKthread2 as *mut kthread::RrosKthread as *mut c_types::c_void, c_str!("%s").as_char_ptr(),
    //  format_args!("hongyu2"));
}

// unsafe extern "C" fn threadfn(arg: *mut c_types::c_void) -> c_types::c_int {
//     // let c;
//     // unsafe{
//     //     c = CStr::from_char_ptr(str as *const c_types::c_char ).as_bytes();
//     // }

//     // let a = str::from_utf8(c);
//     // match a {
//     //     Ok(hello_str)=>{
//     //         pr_debug!("{}\n", hello_str);
//     //     },
//     //     Err(err) => {
//     //         pr_debug!("{}", err);
//     //         return 0 as c_types::c_int;
//     //     },
//     // }
//     let mut kthread = arg as *mut kthread::RrosKthread;
// //////////////////////////////////////////////
//     unsafe {
//         bindings::dovetail_init_altsched(

//             &mut (*kthread).altsched as *mut bindings::dovetail_altsched_context,
//         )
//     };
//     unsafe{
//     let kthread_fn = (*kthread).kthread_fn;
//     match kthread_fn{
//         Some(a) => a(),
//         None => (),
//     }}
// //////////////////////////////////////////////
//     1 as c_types::c_int
// }

fn kfn_1() {
    for _i in 0..10 {
        pr_emerg!("hello! from rros~~~~~~~~~~~~");
    }
    // for t in 0..2000000 {
    // thread::rros_sleep(1000000000);
    // pr_debug!("mutex test in~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
    // pr_debug!("test_mutex rros_current address is {:p}",rros_current());
    // let mut kmutex = RrosKMutex::new();
    // let mut kmutex = &mut kmutex as *mut RrosKMutex;
    // let mut mutex = RrosMutex::new();
    // unsafe{(*kmutex).mutex = &mut mutex as *mut RrosMutex};
    // rros_init_kmutex(kmutex);
    // pr_debug!("init ok~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
    // rros_lock_kmutex(kmutex);
    // pr_debug!("lock ok~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
    // unsafe{aa+=1};
    // pr_debug!("in fn 1 t is {}, a is {}",t,a);
    // rros_unlock_kmutex(kmutex);
    // pr_debug!("unlock ok~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");

    // }
    // pr_debug!("fn 1, a is {}",aa);
    // unsafe { pr_debug!("kfn1: time end is {}", clock::RROS_REALTIME_CLOCK.read())};
    // pr_debug!("kfn1: waste time is {}",y-x);
    // pr_emerg!("hello! from rros~~~~~~~~~~~~");

    // }

    // pr_debug!("mutex test in~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
    // pr_debug!("test_mutex rros_current address is {:p}",rros_current());
    // let mut kmutex = RrosKMutex::new();
    // let mut kmutex = &mut kmutex as *mut RrosKMutex;
    // let mut mutex = RrosMutex::new();
    // unsafe{(*kmutex).mutex = &mut mutex as *mut RrosMutex};
    // rros_init_kmutex(kmutex);
    // pr_debug!("init ok~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
    // rros_lock_kmutex(kmutex);
    // pr_debug!("lock ok~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
    // unsafe{aa+=1};
    // pr_debug!("in fn 1 t is {}, a is {}",t,a);
    // rros_unlock_kmutex(kmutex);
    // pr_debug!("unlock ok~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");

    // }
    // pr_debug!("fn 1, a is {}",aa);
    // unsafe { pr_debug!("kfn1: time end is {}", clock::RROS_REALTIME_CLOCK.read())};
    // pr_debug!("kfn1: waste time is {}",y-x);
    // pr_emerg!("hello! from rros~~~~~~~~~~~~");

    // }
}

// static mut aa:i32 = 0;
// static mut aa:i32 = 0;
pub fn kfn_2() {
    //     for t in 0..1000000 {
    //         // thread::rros_sleep(1000000000);
    //         // pr_debug!("world! from rros~~~~~~~~~~~~");

    //         unsafe{a+=1};
    //         // pr_debug!("in fn 2 t is {}, a is {}",t,a);

    //     }
    //     pr_debug!("fn 2, a is {}",a);
    // }
    for _i in 0..10 {
        // thread::rros_sleep(1000000000);
        // let x = unsafe{clock::RROS_REALTIME_CLOCK.read()};
        // pr_debug!("kfn2: x is {}",x);
        // // for i in 1..100 {
        //     let a = Arc::try_new(1000);
        // // }
        // let y = unsafe{clock::RROS_REALTIME_CLOCK.read()};
        // pr_debug!("kfn2: y is {}",y);
        // pr_debug!("kfn_2: waste time is {}",y-x);
        pr_info!("world! from rros~~~~~~~~~~~~");
    }
}
