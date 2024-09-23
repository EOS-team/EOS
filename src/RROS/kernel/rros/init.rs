#![no_std]
#![feature(allocator_api, global_asm)]
#![feature(
    const_fn_transmute,
    array_map,
    get_mut_unchecked,
    maybe_uninit_extra,
    new_uninit
)]

//! This file is the entry point of the rros kernel module.
//! Importing necessary features and modules

use kernel::{
    bindings, c_types, chrdev, cpumask::CpumaskT, dovetail, irqstage, percpu, prelude::*, str::CStr,
};

use core::str;
use core::sync::atomic::{AtomicU8, Ordering};

mod control;
mod idle;
mod poll;
mod queue;
mod sched;
use sched::rros_init_sched;
mod observable;
mod rros;
mod thread;
// mod weak;
mod fifo;
mod tick;
mod tp;
use tick::rros_enable_tick;

mod stat;
mod timeout;

mod clock;
mod clock_test;
use clock::rros_clock_init;
#[macro_use]
mod list;
mod list_test;
mod lock;
mod memory;
mod memory_test;
mod monitor;
// mod mutex;
mod guard;
mod sched_test;
mod stax;
mod syscall;
mod thread_test;
mod timer;
mod timer_test;
#[macro_use]
mod arch;
mod double_linked_list_test;
mod factory;
mod fifo_test;
mod uapi;
use factory::rros_early_init_factories;

use crate::sched::{this_rros_rq, RROS_CPU_AFFINITY};
use kernel::memory_rros::rros_init_memory;
mod crossing;
mod file;
mod flags;
mod work;

/// This module contains the types used in the application.
/// The `macro_use` attribute indicates that this module also defines macros that are used elsewhere.
#[macro_use]
pub mod types;
mod proxy;
mod smp_test;
mod types_test;
mod wait;
mod xbuf;

#[cfg(CONFIG_NET)]
mod net;

mod drivers;
// pub use net::netif_oob_switch_port;

module! {
    type: Rros,
    name: b"rros",
    author: b"Hongyu Li",
    description: b"A rust realtime os",
    license: b"GPL v2",
    params: {
        oobcpus_arg: str {
            default: b"0-7\0",
            permissions: 0o444,
            description: b"which cpus in the oob",
        },
        init_state_arg: str {
            default: b"enabled",
            permissions: 0o444,
            description: b"inital state of rros",
        },
        sysheap_size_arg: u32{
            default: 0,
            permissions: 0o444,
            description: b"system heap size",
        },
    },
}

/// Data associated with each CPU in the machine.
pub struct RrosMachineCpuData {}

/// Pointer to the machine CPU data.
pub static mut RROS_MACHINE_CPUDATA: *mut RrosMachineCpuData = 0 as *mut RrosMachineCpuData;

enum RrosRunStates {
    RrosStateDisabled = 1,
    #[allow(dead_code)]
    RrosStateRunning = 2,
    RrosStateStopped = 3,
    #[allow(dead_code)]
    RrosStateTeardown = 4,
    RrosStateWarmup = 5,
}

/// The real-time operating system.
pub struct Rros {
    /// Factory for creating character devices.
    pub factory: Pin<Box<chrdev::Registration<{ factory::NR_FACTORIES }>>>,
}

struct InitState {
    label: &'static str,
    state: RrosRunStates,
}

#[allow(dead_code)]
static RUN_FLAG: AtomicU8 = AtomicU8::new(0);
#[allow(dead_code)]
static SCHED_FLAG: AtomicU8 = AtomicU8::new(0);
// static RUN_FLAG: AtomicU8 = AtomicU8::new(0);
static RROS_RUNSTATE: AtomicU8 = AtomicU8::new(RrosRunStates::RrosStateWarmup as u8);
static mut RROS_OOB_CPUS: CpumaskT = CpumaskT::from_int(1 as u64);

fn setup_init_state(init_state_var: &'static str) {
    let warn_bad_state: &str = "invalid init state '{}'\n";

    let init_states: [InitState; 3] = [
        InitState {
            label: "disabled",
            state: RrosRunStates::RrosStateDisabled,
        },
        InitState {
            label: "stopped",
            state: RrosRunStates::RrosStateStopped,
        },
        InitState {
            label: "enabled",
            state: RrosRunStates::RrosStateWarmup,
        },
    ];

    for init_state in init_states {
        if init_state.label == init_state_var {
            set_rros_state(init_state.state);
            pr_info!("{}", init_state_var);
            return;
        }
    }
    pr_warn!("{} {}", warn_bad_state, init_state_var);
}

fn set_rros_state(state: RrosRunStates) {
    RROS_RUNSTATE.store(state as u8, Ordering::Relaxed);
}

fn init_core() -> Result<Pin<Box<chrdev::Registration<{ factory::NR_FACTORIES }>>>> {
    let res =
        irqstage::enable_oob_stage(CStr::from_bytes_with_nul("rros\0".as_bytes())?.as_char_ptr());
    pr_info!("hello");
    match res {
        Ok(_o) => (),
        Err(_e) => {
            pr_warn!("rros cannot be enabled");
            return Err(kernel::Error::EINVAL);
        }
    }
    pr_info!("hella");
    let res = rros_init_memory();
    match res {
        Ok(_o) => (),
        Err(_e) => {
            pr_warn!("memory init wrong");
            return Err(_e);
        }
    }
    let res = rros_early_init_factories(&THIS_MODULE);
    let fac_reg;
    match res {
        Ok(_o) => fac_reg = _o,
        Err(_e) => {
            pr_warn!("factory init wrong");
            return Err(_e);
        }
    }
    pr_info!("haly");

    let res = rros_clock_init();
    match res {
        Ok(_o) => (),
        Err(_e) => {
            pr_warn!("clock init wrong");
            return Err(_e);
        }
    }

    let res = rros_init_sched();
    match res {
        Ok(_o) => (),
        Err(_e) => {
            pr_warn!("sched init wrong");
            return Err(_e);
        }
    }

    let _rq = this_rros_rq();
    pr_debug!("rq add is {:p}", this_rros_rq());
    let res = rros_enable_tick();
    match res {
        Ok(_o) => (),
        Err(_e) => {
            pr_warn!("tick enable wrong");
            return Err(_e);
        }
    }

    let res = dovetail::dovetail_start();
    match res {
        Ok(_o) => (),
        Err(_e) => {
            pr_warn!("dovetail start wrong");
        }
    }

    Ok(fac_reg)
}

#[allow(dead_code)]
fn test_clock() {
    //clock_test::test_do_clock_tick();
    //clock_test::test_adjust_timer();
    let res = clock_test::test_rros_adjust_timers();
    match res {
        Ok(_o) => (),
        Err(_e) => {
            pr_warn!("clock timers adjust wrong");
        }
    }
    //clock_test::test_rros_stop_timers();
}

#[allow(dead_code)]
fn test_timer() {
    // timer_test::test_timer_at_front();
    // timer_test::test_rros_timer_deactivate();
    // timer_test::test_rros_get_timer_gravity();
    // timer_test::test_rros_update_timer_date();
    // timer_test::test_rros_get_timer_next_date();
    // timer_test::test_rros_get_timer_expiry();
    //timer_test::test_rros_get_timer_delta();
    //timer_test::test_rros_get_timer_date();
    //timer_test::test_rros_insert_tnode();
    //timer_test::test_rros_enqueue_timer();
    // timer_test::test_program_timer();
    // timer_test::test_rros_start_timer();
    // timer_test::test_stop_timer_locked();
    // timer_test::test_rros_destroy_timer();
    //timer_test::test_get_handler();
}

#[allow(dead_code)]
fn test_double_linked_list() {
    let res = double_linked_list_test::test_enqueue_by_index();
    match res {
        Ok(_o) => (),
        Err(_e) => {
            pr_warn!("enqueue by index wrong");
        }
    }
}

fn test_thread() {
    thread_test::test_thread_context_switch();
    // thread_test::test_NetKthreadRunner();
}

// fn test_tp(){
//     tp::test_tp();
// }

#[allow(dead_code)]
fn test_sched() {
    // sched_test::test_this_rros_rq_thread();
    // sched_test::test_cpu_smp();
    let res = sched_test::test_rros_set_resched();
    match res {
        Ok(_o) => (),
        Err(_e) => {
            pr_warn!("set resched wrong");
        }
    }
}

#[allow(dead_code)]
fn test_fifo() {
    let res = fifo_test::test_rros_enqueue_fifo_thread();
    match res {
        Ok(_o) => (),
        Err(_e) => {
            pr_warn!("enqueue fifo wrong");
        }
    }
}
fn test_mem() {
    memory_test::mem_test();
}

fn test_lantency() {
    rros::latmus::test_latmus();
}

#[allow(dead_code)]
fn test_smp() {
    smp_test::smp_test_parallel_execution();
}

impl KernelModule for Rros {
    fn init() -> Result<Self> {
        pr_info!("Hello world from rros!\n");
        let init_state_arg_str = str::from_utf8(init_state_arg.read())?;
        setup_init_state(init_state_arg_str);

        if RROS_RUNSTATE.load(Ordering::Relaxed) != RrosRunStates::RrosStateWarmup as u8 {
            pr_warn!("disabled on kernel command line\n");
            return Err(kernel::Error::EINVAL);
        }

        let cpu_online_mask = unsafe { CpumaskT::read_cpu_online_mask() };
        // When size_of is 0, align_of is 4, alloc reports an error.
        unsafe {
            RROS_MACHINE_CPUDATA =
                percpu::alloc_per_cpu(4 as usize, 4 as usize) as *mut RrosMachineCpuData
        };
        if str::from_utf8(oobcpus_arg.read())? != "" {
            let res = unsafe {
                RROS_OOB_CPUS
                    .cpulist_parse(CStr::from_bytes_with_nul(oobcpus_arg.read())?.as_char_ptr())
            };
            match res {
                Ok(_o) => (pr_info!("load parameters {}\n", str::from_utf8(oobcpus_arg.read())?)),
                Err(_e) => {
                    pr_warn!("wrong oobcpus_arg");
                    unsafe {
                        RROS_OOB_CPUS.cpumask_copy(&cpu_online_mask);
                    }
                }
            }
        } else {
            unsafe {
                RROS_OOB_CPUS.cpumask_copy(&cpu_online_mask);
            }
        }

        unsafe {
            RROS_CPU_AFFINITY.cpumask_copy(&RROS_OOB_CPUS);
        }

        let res = init_core(); //*sysheap_size_arg.read()
        let fac_reg;

        // test_timer();
        // test_double_linked_list();

        // test_clock();
        test_thread();
        //test_double_linked_list();
        // wait::wait_test();
        let ret = net::init();
        match ret {
            Ok(_o) => (),
            Err(_e) => {
                pr_warn!("net init wrong");
                return Err(_e);
            }
        }

        test_mem();
        match res {
            Ok(_o) => {
                pr_info!("Success boot the rros.");
                fac_reg = _o;
            }
            Err(_e) => {
                pr_warn!("Boot failed!\n");
                return Err(_e);
            }
        }
        test_lantency();

        test_smp();

        // let mut rros_kthread1 = rros_kthread::new(fn1);
        // let mut rros_kthread2 = rros_kthread::new(fn2);
        // pr_debug!("before thread 1");
        // kthread::kthread_run(Some(threadfn), &mut rros_kthread1 as *mut rros_kthread as *mut c_types::c_void, c_str!("%s").as_char_ptr(),
        //  format_args!("hongyu1"));

        //  pr_debug!("between 1 and 2");

        // kthread::kthread_run(Some(threadfn), &mut rros_kthread2 as *mut rros_kthread as *mut c_types::c_void, c_str!("%s").as_char_ptr(),
        //  format_args!("hongyu2"));

        Ok(Rros { factory: fac_reg })
    }
}

#[no_mangle]
unsafe extern "C" fn helloworld() {
    pr_info!("hello world! from C to rust");
}

impl Drop for Rros {
    fn drop(&mut self) {
        pr_info!("Bye world from rros!\n");
    }
}
