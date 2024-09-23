use core::mem::size_of;

use kernel::{
    bindings,
    c_types::c_void,
    capability,
    io_buffer::IoBufferWriter,
    ktime,
    prelude::*,
    premmpt::running_inband,
    ptrace::{IrqStage, PtRegs},
    sync::{Lock, SpinLock},
    task::Task,
    uapi::time_types::{KernelOldTimespec, KernelTimespec},
    user_ptr::UserSlicePtr,
};

use crate::{
    arch::arm64::syscall::*,
    clock::{rros_read_clock, RROS_MONO_CLOCK, RROS_REALTIME_CLOCK},
    file::rros_get_file,
    is_clock_gettime, is_clock_gettime64, oob_arg1, oob_arg2, oob_arg3,
    sched::{rros_is_inband, rros_switch_inband, RrosThread},
    thread::*,
    uapi::rros::syscall::*,
    uapi::rros::thread::*,
};

const FMODE_READ: u32 = 0x1;
const FMODE_WRITE: u32 = 0x2;
const SYSCALL_PROPAGATE: i32 = 0;
const SYSCALL_STOP: i32 = 1;

fn rros_read(fd: i32, u_buf: *mut u8, size: isize) -> i64 {
    pr_debug!("oob_read syscall is called");
    let rfilp = rros_get_file(fd as u32);
    if rfilp.is_none() {
        return -(bindings::EBADF as i64);
    }
    let rfilp = unsafe { rfilp.unwrap().as_mut() };

    let ret: i64;
    let filp = rfilp.filp;
    if unsafe { (*filp).f_mode } & FMODE_READ == 0 {
        rfilp.put_file();
        return -(bindings::EBADF as i64);
    }

    if unsafe { (*(*filp).f_op).oob_read.is_none() } {
        rfilp.put_file();
        return -(bindings::EINVAL as i64);
    }
    ret =
        unsafe { (*(*filp).f_op).oob_read.unwrap()(filp, u_buf as *mut i8, size as usize) as i64 };
    rfilp.put_file();
    pr_debug!("oob_read syscall success, the value of ret is {}", ret);

    ret
}

fn rros_write(fd: i32, u_buf: *mut u8, size: isize) -> i64 {
    pr_debug!("oob_write syscall is called");
    let rfilp = rros_get_file(fd as u32);
    if rfilp.is_none() {
        return -(bindings::EBADF as i64);
    }
    let rfilp = unsafe { rfilp.unwrap().as_mut() };

    let ret: i64;
    let filp = rfilp.filp;
    if unsafe { (*filp).f_mode } & FMODE_WRITE == 0 {
        rfilp.put_file();
        return -(bindings::EBADF as i64);
    }

    if unsafe { (*(*filp).f_op).oob_write.is_none() } {
        rfilp.put_file();
        return -(bindings::EINVAL as i64);
    }
    ret =
        unsafe { (*(*filp).f_op).oob_write.unwrap()(filp, u_buf as *mut i8, size as usize) as i64 };
    rfilp.put_file();
    pr_debug!("oob_write syscall success, the value of ret is {}", ret);

    ret
}

fn rros_ioctl(fd: i32, request: u32, arg: u64) -> i64 {
    let rfilp = rros_get_file(fd as u32);
    if rfilp.is_none() {
        return -(bindings::EBADF as i64);
    }
    let rfilp = unsafe { rfilp.unwrap().as_mut() };
    // // TODO: compat oob call
    // // if (unlikely(is_compat_oob_call())) {
    // // 	if (filp->f_op->compat_oob_ioctl)
    // // 		ret = filp->f_op->compat_oob_ioctl(filp, request, arg);
    // // }
    let mut ret: i64 = 0;
    let filp = rfilp.filp;
    // let oob_ioctl = unsafe{&(*(*filp).f_op).oob_ioctl};

    // if let Some(oob_func) = oob_ioctl{
    // ret = unsafe{oob_func(filp, request, arg as u64) as i64};
    // }
    if unsafe { (*(*filp).f_op).oob_ioctl.is_some() } {
        ret = unsafe { (*(*filp).f_op).oob_ioctl.unwrap()(filp, request, arg) };
    } else if unsafe { (*(*filp).f_op).compat_oob_ioctl.is_some() } {
        ret = unsafe { (*(*filp).f_op).compat_oob_ioctl.unwrap()(filp, request, arg) };
    }
    if ret == -(bindings::ENOIOCTLCMD as i64) {
        ret = -(bindings::ENOTTY as i64);
    }
    rfilp.put_file();
    ret
}

fn invoke_syscall(nr: u32, regs: PtRegs) {
    let mut ret = 0;

    /*
     * We have only very few syscalls, prefer a plain switch to a
     * pointer indirection which ends up being fairly costly due
     * to exploit mitigations.
     */
    unsafe {
        match nr {
            // [TODO: lack __user]
            SYS_RROS_READ => {
                ret = rros_read(
                    oob_arg1!((regs.ptr)) as i32,
                    oob_arg2!((regs.ptr)) as *mut u8,
                    oob_arg3!((regs.ptr)) as isize,
                );
            }
            SYS_RROS_WRITE => {
                ret = rros_write(
                    oob_arg1!((regs.ptr)) as i32,
                    oob_arg2!((regs.ptr)) as *mut u8,
                    oob_arg3!((regs.ptr)) as isize,
                );
            }
            SYS_RROS_IOCTL => {
                ret = rros_ioctl(
                    oob_arg1!((regs.ptr)) as i32,
                    oob_arg2!((regs.ptr)) as u32,
                    oob_arg3!((regs.ptr)),
                );
            }
            _ => {
                pr_alert!("err!");
            }
        }
    }

    set_oob_retval(regs, ret);
}

fn prepare_for_signal(
    _p: *mut SpinLock<RrosThread>,
    curr: *mut SpinLock<RrosThread>,
    regs: PtRegs,
) {
    let flags;

    // /*
    // * FIXME: no restart mode flag for setting -EINTR instead of
    // * -ERESTARTSYS should be obtained from curr->local_info on a
    // * per-invocation basis, not on a per-call one (since we have
    // * 3 generic calls only).
    // */
    // /*
    // * @curr == this_rros_rq()->curr over oob so no need to grab
    // * @curr->lock (i.e. @curr cannot go away under out feet).
    // */
    // [TODO: use the curr rq lock to make smp work]
    flags = unsafe {
        (*(*(*curr).locked_data().get()).rq.unwrap())
            .lock
            .raw_spin_lock_irqsave()
    };

    // /*
    // * We are called from out-of-band mode only to act upon a
    // * pending signal receipt. We may observe signal_pending(p)
    // * which implies that T_KICKED was set too
    // * (handle_sigwake_event()), or T_KICKED alone which means
    // * that we have been unblocked from a wait for some other
    // * reason.
    // */
    let res2 = unsafe { (*(*curr).locked_data().get()).info & T_KICKED != 0 };
    unsafe {
        if res2 {
            let res1 = Task::current().signal_pending();

            if res1 {
                set_oob_error(regs, -(bindings::ERESTARTSYS as i32));
                (*(*curr).locked_data().get()).info &= !T_BREAK;
            }
            (*(*curr).locked_data().get()).info &= !T_KICKED;
        }
    }

    unsafe {
        (*(*(*curr).locked_data().get()).rq.unwrap())
            .lock
            .raw_spin_unlock_irqrestore(flags);
    }

    rros_test_cancel();

    rros_switch_inband(RROS_HMDIAG_SYSDEMOTE);
}

fn handle_vdso_fallback(nr: i32, regs: PtRegs) -> bool {
    let u_old_ts;
    let mut uts: KernelOldTimespec = KernelOldTimespec::new();
    let u_uts;
    let mut old_ts: KernelTimespec = KernelTimespec::new();

    let clock;
    let ts64;
    let clock_id;
    let mut ret: i64 = 0;

    if !is_clock_gettime!(nr) && !is_clock_gettime64!(nr) {
        return false;
    }

    clock_id = unsafe { oob_arg1!((regs.ptr)) as u32 };
    match clock_id {
        bindings::CLOCK_MONOTONIC => {
            clock = unsafe { &RROS_MONO_CLOCK };
        }
        bindings::CLOCK_REALTIME => {
            clock = unsafe { &RROS_REALTIME_CLOCK };
        }
        _ => {
            pr_alert!("error clock");
            // clock = unsafe{&RROS_MONO_CLOCK};
            return false;
        }
    }

    ts64 = ktime::ktime_to_timespec64(rros_read_clock(clock));

    if is_clock_gettime!(nr) {
        old_ts.spec.tv_sec = ts64.0.tv_sec;
        old_ts.spec.tv_nsec = ts64.0.tv_nsec;
        // [TODO: lack the size of u_old_rs]
        u_old_ts = unsafe {
            UserSlicePtr::new(
                oob_arg2!((regs.ptr)) as *mut c_void,
                size_of::<KernelTimespec>(),
            )
        };
        let res = unsafe {
            u_old_ts.writer().write_raw(
                &mut old_ts as *mut KernelTimespec as *mut u8 as *const u8,
                size_of::<KernelTimespec>(),
            )
        };
        ret = match res {
            Ok(()) => 0,
            Err(_e) => -(bindings::EFAULT as i64),
        };
    } else if is_clock_gettime64!(nr) {
        uts.spec.tv_sec = ts64.0.tv_sec;
        uts.spec.tv_nsec = ts64.0.tv_nsec;
        // [TODO: lack the size of u_uts]
        u_uts = unsafe {
            UserSlicePtr::new(
                oob_arg2!((regs.ptr)) as *mut c_void,
                size_of::<KernelOldTimespec>(),
            )
        };
        let res = unsafe {
            u_uts.writer().write_raw(
                &mut uts as *mut KernelOldTimespec as *mut u8 as *const u8,
                size_of::<KernelOldTimespec>(),
            )
        };
        ret = match res {
            Ok(()) => 0,
            Err(_e) => -(bindings::EFAULT as i64),
        };
    }

    set_oob_retval(regs, ret);

    // #undef is_clock_gettime
    // #undef is_clock_gettime64

    true
}

fn do_oob_syscall(stage: IrqStage, regs: PtRegs) -> i32 {
    let p;
    let mut nr: u32 = 0;

    if !is_oob_syscall(regs) {
        if rros_is_inband() {
            return SYSCALL_PROPAGATE;
        }

        /*
         * We don't want to trigger a stage switch whenever the
         * current request issued from the out-of-band stage is not a
         * valid in-band syscall, but rather deliver -ENOSYS directly
         * instead.  Otherwise, switch to in-band mode before
         * propagating the syscall down the pipeline. CAUTION:
         * inband_syscall_nr(regs, &nr) is valid only if
         * !is_oob_syscall(regs), which we checked earlier in
         * do_oob_syscall().
         */
        if inband_syscall_nr(regs, &mut nr as *mut u32) {
            if handle_vdso_fallback(nr as i32, regs) {
                return SYSCALL_STOP;
            }

            rros_switch_inband(RROS_HMDIAG_SYSDEMOTE);
            return SYSCALL_PROPAGATE;
        }

        // bad_syscall:
        // [TODO: add rros warning]
        pr_warn!("Warning: invalid out-of-band syscall {}", nr);
        // printk(RROS_WARNING "invalid out-of-band syscall <%#x>\n", nr);

        set_oob_error(regs, -(bindings::ENOSYS as i32));

        return SYSCALL_STOP;
    }

    nr = oob_syscall_nr(regs);
    if nr >= crate::uapi::rros::syscall::NR_RROS_SYSCALLS {
        pr_debug!("invalid out-of-band syscall <{}>", nr);

        set_oob_error(regs, -(bindings::ENOSYS as i32));
        return SYSCALL_STOP;
    }

    let curr = rros_current();
    let res1 = !(capability::KernelCapStruct::cap_raised(
        capability::KernelCapStruct::current_cap(),
        bindings::CAP_SYS_NICE as i32,
    ) != 0);
    pr_debug!("curr is {:p} res is {}", curr, res1);
    if curr == 0 as *mut SpinLock<RrosThread> || res1 {
        // [TODO: lack RROS_DEBUG]
        pr_err!("ERROR: syscall denied");
        // if (RROS_DEBUG(CORE))
        // 	printk(RROS_WARNING
        // 		"syscall <oob_%s> denied to %s[%d]\n",
        // 		rros_sysnames[nr], current->comm, task_pid_nr(current));
        set_oob_error(regs, -(bindings::EPERM as i32));
        return SYSCALL_STOP;
    }

    /*
     * If the syscall originates from in-band context, hand it
     * over to handle_inband_syscall() where the caller would be
     * switched to OOB context prior to handling the request.
     */
    if stage.ptr != IrqStage::get_oob_state().ptr {
        return SYSCALL_PROPAGATE;
    }

    // [TODO: lack the trace system]
    // trace_rros_oob_sysentry(nr);

    invoke_syscall(nr, regs);

    /* Syscall might have switched in-band, recheck. */
    if !rros_is_inband() {
        p = rros_current();
        let res1 = Task::current().signal_pending();
        let res2 = unsafe { (*(*curr).locked_data().get()).info & T_KICKED != 0 };
        let res3 = (Task::current().state() & T_WEAK) != 0;
        // [TODO: lack covert atomic in bindings to atomic in rfl]
        let res4 = unsafe {
            (*(*curr).locked_data().get())
                .inband_disable_count
                .atomic_read()
                != 0
        };
        if res1 || res2 {
            prepare_for_signal(p, curr, regs);
        } else if res3 && !res4 {
            rros_switch_inband(RROS_HMDIAG_NONE);
        }
    }

    /* Update the stats and user visible info. */
    // [TODO: lack syncing of user info]
    // rros_inc_counter(&curr->stat.sc);
    // rros_sync_uwindow(curr);

    // [TODO: lack trace]
    // trace_rros_oob_sysexit(oob_retval(regs));
    // unsafe{
    //     thread::UTHREAD = None;
    // }

    return SYSCALL_STOP;
}

fn do_inband_syscall(_stage: IrqStage, regs: PtRegs) -> i32 {
    let curr = rros_current();
    // struct RrosThread *curr = rros_current(); /* Always valid. */
    let p;
    // struct task_struct *p;
    let nr;
    // unsigned int nr;
    let ret;
    // int ret;

    /*
     * Some architectures may use special out-of-bound syscall
     * numbers which escape Dovetail's range check, e.g. when
     * handling aarch32 syscalls over an aarch64 kernel. When so,
     * assume this is an in-band syscall which we need to
     * propagate downstream to the common handler.
     */
    if curr == 0 as *mut SpinLock<RrosThread> {
        return SYSCALL_PROPAGATE;
    }

    /*
     * Catch cancellation requests pending for threads undergoing
     * the weak scheduling policy, which won't cross
     * prepare_for_signal() frequently as they run mostly in-band.
     */
    rros_test_cancel();

    /* Handle lazy schedparam updates before switching. */
    // rros_propagate_schedparam_change(curr);

    /* Propagate in-band syscalls. */
    if !is_oob_syscall(regs) {
        return SYSCALL_PROPAGATE;
    }

    /*
     * Process an OOB syscall after switching current to OOB
     * context.  do_oob_syscall() already checked the syscall
     * number.
     */
    nr = oob_syscall_nr(regs);

    // [TODO: lack trace]
    // trace_rros_inband_sysentry(nr);

    ret = rros_switch_oob();
    /*
     * -ERESTARTSYS might be received if switching oob was blocked
     * by a pending signal, otherwise -EINTR might be received
     * upon signal detection after the transition to oob context,
     * in which case the common logic applies (i.e. based on
     * T_KICKED and/or signal_pending()).
     */
    if ret == Err(kernel::Error::ERESTARTSYS) {
        set_oob_error(regs, -(bindings::ERESTARTSYS as i32));

        let res1 = unsafe { (*(*curr).locked_data().get()).local_info };
        if res1 & T_IGNOVR == 1 {
            unsafe {
                (*(*curr).locked_data().get()).local_info &= !T_IGNOVR;
            }
        }

        // [TODO: lack sync user stat]
        // rros_inc_counter(&curr->stat.sc);
        // rros_sync_uwindow(curr);

        // [TODO: lack trace]
        // trace_rros_inband_sysexit(oob_retval(regs));

        return SYSCALL_STOP;
    }

    invoke_syscall(nr, regs);

    if !rros_is_inband() {
        p = rros_current();
        let res1 = Task::current().signal_pending();
        let res2 = unsafe { (*(*curr).locked_data().get()).info & T_KICKED != 0 };
        let res3 = (Task::current().state() & T_WEAK) != 0;
        let res4 = unsafe {
            (*(*curr).locked_data().get())
                .inband_disable_count
                .atomic_read()
                != 0
        };
        if res1 || res2 {
            prepare_for_signal(p, curr, regs);
        } else if res3 && res4 {
            rros_switch_inband(RROS_HMDIAG_NONE);
        }
    }

    let res1 = unsafe { (*(*curr).locked_data().get()).local_info };
    if (res1 & T_IGNOVR) != 0 {
        unsafe {
            (*(*curr).locked_data().get()).local_info &= !T_IGNOVR;
        }
    }

    // [TODO: lack sync user stat]
    // rros_inc_counter(&curr->stat.sc);
    // rros_sync_uwindow(curr);

    // [TODO: lack trace]
    // trace_rros_inband_sysexit(oob_retval!(regs));

    return SYSCALL_STOP;
}

// gcc /root/rros_output/lib/librros.so write.c -lpthread -g -o write
// export C_INCLUDE_PATH=$C_INCLUDE_PATH:/root/rros_output/include
no_mangle_function_declaration! {
    unsafe extern "C" fn handle_pipelined_syscall(stage: IrqStage, regs: PtRegs) -> i32 {
        // [TODO: lack unlikely]
        let res = running_inband();
        let r = match res {
            Ok(_o) => true,
            Err(_e) => false,
        };
        if r {
            return do_inband_syscall(stage, regs);
        }
        return do_oob_syscall(stage, regs);
    }
}

no_mangle_function_declaration! {
    unsafe extern "C" fn handle_oob_syscall(regs: PtRegs) {
        let _ret: i32;
        // if running_inband().is_ok() {
        //     // return;
        // }

        _ret = unsafe { do_oob_syscall(IrqStage::get_oob_state(), regs) };
        // [TODO: lack warn_on]
        // RROS_WARN_ON(CORE, ret == SYSCALL_PROPAGATE);
    }
}
