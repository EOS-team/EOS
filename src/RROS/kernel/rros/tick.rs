use kernel::{
    bindings, c_types, clockchips,
    clockchips::ClockEventDevice,
    cpumask::{num_possible_cpus, CpumaskT},
    interrupt,
    irq_pipeline::*,
    ktime::*,
    percpu::alloc_per_cpu,
    percpu_defs,
    prelude::*,
    str::CStr,
    sync::Lock,
    tick,
};

use crate::{
    clock::*, sched::*, thread::*, timeout::*, timer::*, RROS_MACHINE_CPUDATA, RROS_OOB_CPUS,
};
use core::{
    cmp,
    mem::{align_of, size_of},
    ptr::null_mut,
};

extern "C" {
    fn rust_helper_hard_local_irq_save() -> c_types::c_ulong;
    fn rust_helper_hard_local_irq_restore(flags: c_types::c_ulong);
    fn rust_helper_tick_notify_proxy();
    fn rust_helper_IRQF_OOB() -> c_types::c_ulong;
}

static mut PROXY_DEVICE: *mut clockchips::ClockProxyDevice = 0 as *mut clockchips::ClockProxyDevice;

pub const CLOCK_EVT_FEAT_KTIME: u32 = 0x000004;
type KtimeT = i64;

pub fn rros_program_local_tick(clock: *mut RrosClock) {
    unsafe { (*(*clock).get_master()).program_local_shot() };
}

pub fn rros_program_remote_tick(clock: *mut RrosClock, rq: *mut RrosRq) {
    #[cfg(CONFIG_SMP)]
    unsafe {
        (*(*clock).get_master()).program_remote_shot(rq)
    };
}

pub fn rros_notify_proxy_tick(rq: *mut RrosRq) {
    unsafe { (*rq).local_flags &= !RQ_TPROXY };
    unsafe { rust_helper_tick_notify_proxy() };
}

pub struct RrosProxySetNextKtime;

impl clockchips::ProxySetNextKtime for RrosProxySetNextKtime {
    fn proxy_set_next_ktime(expires: KtimeT, _arg1: clockchips::ClockEventDevice) -> i32 {
        //pr_debug!("proxy_set_next_ktime: in");
        let delta = ktime_sub(expires, ktime_get());
        let flags = unsafe { rust_helper_hard_local_irq_save() };

        let rq = this_rros_rq();

        // let inband_timer = unsafe{(*rq).get_inband_timer()};
        //unsafe{rros_program_proxy_tick(&RROS_MONO_CLOCK)};
        unsafe {
            rros_start_timer(
                (*rq).get_inband_timer(),
                rros_abs_timeout((*rq).get_inband_timer(), delta),
                RROS_INFINITE,
            )
        };
        unsafe { rust_helper_hard_local_irq_restore(flags) };
        // pr_debug!("proxy_set_next_ktime: end");
        return 0;
    }
}

pub struct RrosProxySetOneShotStopped;

impl clockchips::ProxySetOneshotStopped for RrosProxySetOneShotStopped {
    fn proxy_set_oneshot_stopped(dev: clockchips::ClockProxyDevice) -> c_types::c_int {
        pr_debug!("proxy_set_oneshot_stopped: in");
        let flags = unsafe { rust_helper_hard_local_irq_save() };
        let rq = this_rros_rq();
        unsafe {
            rros_stop_timer((*rq).get_inband_timer());
            (*rq).local_flags |= RQ_TSTOPPED;
            if (*rq).local_flags & RQ_IDLE != 0 {
                let real_dev = (*dev.get_ptr()).real_device;
                (*real_dev).set_state_oneshot_stopped.unwrap()(real_dev);
            }
        }
        unsafe { rust_helper_hard_local_irq_restore(flags) };
        pr_debug!("proxy_set_oneshot_stopped: end");
        return 0;
    }
}

#[cfg(CONFIG_SMP)]
pub struct RrosClockIpiHandler;

#[cfg(CONFIG_SMP)]
impl clockchips::ClockIpiHandler for RrosClockIpiHandler {
    fn clock_ipi_handler(_irq: c_types::c_int, _dev_id: *mut c_types::c_void) -> c_types::c_uint {
        pr_debug!("god nn");
        unsafe {
            clockchips::core_tick::<RrosCoreTick>(null_mut());
        }
        return bindings::irqreturn_IRQ_HANDLED;
    }
}

pub fn rros_enable_tick() -> Result<usize> {
    unsafe {
        PROXY_DEVICE = alloc_per_cpu(
            size_of::<*mut clockchips::ClockProxyDevice>() as usize,
            align_of::<*mut clockchips::ClockProxyDevice>() as usize,
        ) as *mut clockchips::ClockProxyDevice;
        if PROXY_DEVICE == null_mut() {
            return Err(kernel::Error::ENOMEM);
        }
        pr_debug!("PROXY_DEVICE alloc success");
    }

    pr_debug!("rros_enable_tick: in");
    #[cfg(CONFIG_SMP)]
    if num_possible_cpus() > 1 {
        pr_debug!("rros_enable_tick123");
        let ret = unsafe {
            interrupt::__request_percpu_irq(
                irq_get_timer_oob_ipi() as c_types::c_uint,
                Some(clockchips::clock_ipi_handler::<RrosClockIpiHandler>),
                rust_helper_IRQF_OOB(),
                CStr::from_bytes_with_nul_unchecked("RROS_CLOCK_REALTIME_DEV\0".as_bytes())
                    .as_char_ptr(),
                RROS_MACHINE_CPUDATA as *mut c_types::c_void,
            )
        };
        if ret != 0 {
            return Err(kernel::Error::ENOMEM);
        }
    }

    let _ret = tick::tick_install_proxy(Some(clockchips::setup_proxy::<RrosSetupProxy>), unsafe {
        RROS_OOB_CPUS.as_cpumas_ptr()
    });
    pr_info!("enable tick success!");
    // if (ret && IS_ENABLED(CONFIG_SMP) && num_possible_cpus() > 1) {
    // 	free_percpu_irq(TIMER_OOB_IPI, &rros_machine_cpudata);
    // 	return ret;
    // }

    Ok(0)
}

pub struct RrosSetupProxy;

impl clockchips::SetupProxy for RrosSetupProxy {
    // new setup
    fn setup_proxy(dev: clockchips::ClockProxyDevice) {
        let _real_dev: ClockEventDevice;
        match clockchips::ClockEventDevice::from_proxy_device(dev.get_real_device()) {
            Ok(v) => _real_dev = v,
            Err(_e) => {
                pr_warn!("1setup real ced new error!");
                return;
            }
        }

        let proxy_dev: ClockEventDevice;
        match clockchips::ClockEventDevice::from_proxy_device(dev.get_proxy_device()) {
            Ok(v) => proxy_dev = v,
            Err(_e) => {
                pr_warn!("1setup proxy ced new error!");
                return;
            }
        }

        dev.set_handle_oob_event(clockchips::core_tick::<RrosCoreTick>);
        let mut temp = proxy_dev.get_features();
        temp |= CLOCK_EVT_FEAT_KTIME;
        proxy_dev.set_features(temp);

        proxy_dev.set_set_next_ktime(clockchips::proxy_set_next_ktime::<RrosProxySetNextKtime>);
        if proxy_dev.get_set_state_oneshot_stopped().is_some() {
            proxy_dev.set_set_state_oneshot_stopped(
                clockchips::proxy_set_oneshot_stopped::<RrosProxySetOneShotStopped>,
            );
        }

        unsafe {
            let tmp_dev =
                percpu_defs::per_cpu_ptr(PROXY_DEVICE as *mut u8, percpu_defs::smp_processor_id())
                    as *mut clockchips::ClockProxyDevice;
            (*tmp_dev).ptr = dev.get_ptr();
        }
    }
}

pub fn rros_program_proxy_tick(clock: &RrosClock) {
    let tmp_dev;
    unsafe {
        tmp_dev =
            &*(percpu_defs::per_cpu_ptr(PROXY_DEVICE as *mut u8, percpu_defs::smp_processor_id())
                as *mut clockchips::ClockProxyDevice);
    }

    let dev;
    match clockchips::ClockProxyDevice::new(tmp_dev.get_ptr()) {
        Ok(v) => dev = v,
        Err(_e) => {
            pr_warn!("cpd new error!");
            return;
        }
    }

    let real_dev;
    match clockchips::ClockEventDevice::from_proxy_device(dev.get_real_device()) {
        Ok(v) => real_dev = v,
        Err(_e) => {
            pr_warn!("real ced new error!");
            return;
        }
    }
    let _proxy_device;
    match clockchips::ClockEventDevice::from_proxy_device(dev.get_proxy_device()) {
        Ok(v) => _proxy_device = v,
        Err(_e) => {
            pr_warn!("proxy ced new error!");
            return;
        }
    }
    let this_rq = this_rros_rq();
    unsafe {
        if (*this_rq).local_flags & RQ_TIMER != 0x0 {
            return;
        }
    }

    let tmb = rros_this_cpu_timers(&clock);
    unsafe {
        if (*tmb).q.is_empty() {
            (*this_rq).add_local_flags(RQ_IDLE);
            return;
        }
        (*this_rq).change_local_flags(!(RQ_TDEFER | RQ_IDLE | RQ_TSTOPPED));
    }

    let mut timer = unsafe { (*tmb).q.get_head().unwrap().value.clone() };
    let inband_timer_addr = unsafe { (*this_rq).get_inband_timer().locked_data().get() };
    let timer_addr = timer.locked_data().get();
    if timer_addr == inband_timer_addr {
        unsafe {
            let state = (*(*this_rq).get_curr().locked_data().get()).state;
            if rros_need_resched(this_rq) || state & T_ROOT == 0x0 {
                if (*tmb).q.len() > 1 {
                    (*this_rq).add_local_flags(RQ_TDEFER);
                    timer = (*tmb).q.get_by_index(2).unwrap().value.clone();
                }
            }
        }
    }
    let t = unsafe { (*timer.locked_data().get()).get_date() };
    let mut delta = ktime_to_ns(ktime_sub(t, clock.read()));
    if real_dev.get_features() as u32 & CLOCK_EVT_FEAT_KTIME != 0 {
        real_dev.set_next_ktime(t, real_dev.get_ptr());
    } else {
        if delta <= 0 {
            delta = real_dev.get_min_delta_ns() as i64;
        } else {
            delta = cmp::min(delta, real_dev.get_max_delta_ns() as i64);
            delta = cmp::max(delta, real_dev.get_min_delta_ns() as i64);
        }
        let cycles = (delta as u64 * (real_dev.get_mult() as u64)) >> real_dev.get_shift();
        let ret = real_dev.set_next_event(cycles, dev.get_real_device());
        if ret != 0 {
            real_dev.set_next_event(real_dev.get_min_delta_ticks(), dev.get_real_device());
        }
    }
}

#[cfg(CONFIG_SMP)]
pub fn rros_send_timer_ipi(_clock: &RrosClock, rq: *mut RrosRq) {
    irq_send_oob_ipi(
        irq_get_timer_oob_ipi(),
        CpumaskT::cpumask_of(rros_rq_cpu(rq) as u32),
    );
}
