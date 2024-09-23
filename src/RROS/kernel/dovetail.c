/*
 * SPDX-License-Identifier: GPL-2.0
 *
 * Copyright (C) 2016 Philippe Gerum  <rpm@xenomai.org>.
 */
#include <linux/timekeeper_internal.h>
#include <linux/sched/signal.h>
#include <linux/irq_pipeline.h>
#include <linux/dovetail.h>
#include <asm/unistd.h>
#include <asm/syscall.h>
#include <uapi/asm-generic/dovetail.h>

static bool dovetail_enabled;

void __weak arch_inband_task_init(struct task_struct *p)
{
}

void inband_task_init(struct task_struct *p)
{
	struct thread_info *ti = task_thread_info(p);

	clear_ti_local_flags(ti, _TLF_DOVETAIL|_TLF_OOB|_TLF_OFFSTAGE);
	arch_inband_task_init(p);
}

void dovetail_init_altsched(struct dovetail_altsched_context *p)
{
	struct task_struct *tsk = current;
	struct mm_struct *mm = tsk->mm;

	check_inband_stage();
	p->task = tsk;
	p->active_mm = mm;
	p->borrowed_mm = false;

	/*
	 * Make sure the current process will not share any private
	 * page with its child upon fork(), sparing it the random
	 * latency induced by COW. MMF_DOVETAILED is never cleared once
	 * set. We serialize with dup_mmap() which holds the mm write
	 * lock.
	 */
	if (!(tsk->flags & PF_KTHREAD) &&
		!test_bit(MMF_DOVETAILED, &mm->flags)) {
		mmap_write_lock(mm);
		__set_bit(MMF_DOVETAILED, &mm->flags);
		mmap_write_unlock(mm);
	}
}
EXPORT_SYMBOL_GPL(dovetail_init_altsched);

void dovetail_start_altsched(void)
{
	check_inband_stage();
	set_thread_local_flags(_TLF_DOVETAIL);
}
EXPORT_SYMBOL_GPL(dovetail_start_altsched);

void dovetail_stop_altsched(void)
{
	clear_thread_local_flags(_TLF_DOVETAIL);
	clear_thread_flag(TIF_MAYDAY);
}
EXPORT_SYMBOL_GPL(dovetail_stop_altsched);

extern void handle_oob_syscall(struct pt_regs *regs);

// void __weak handle_oob_syscall(struct pt_regs *regs)
// {
// }

extern int handle_pipelined_syscall(struct irq_stage *stage,
				    struct pt_regs *regs);

// int __weak handle_pipelined_syscall(struct irq_stage *stage,
// 				    struct pt_regs *regs)
// {
// 	return 0;
// }

void __weak handle_oob_mayday(struct pt_regs *regs)
{
}

static inline
void call_mayday(struct thread_info *ti, struct pt_regs *regs)
{
	clear_ti_thread_flag(ti, TIF_MAYDAY);
	handle_oob_mayday(regs);
}

void dovetail_call_mayday(struct pt_regs *regs)
{
	struct thread_info *ti = current_thread_info();
	unsigned long flags;

	flags = hard_local_irq_save();
	call_mayday(ti, regs);
	hard_local_irq_restore(flags);
}

void inband_retuser_notify(void)
{
	clear_thread_flag(TIF_RETUSER);
	inband_event_notify(INBAND_TASK_RETUSER, current);
	/* CAUTION: we might have switched out-of-band here. */
}

int __pipeline_syscall(struct pt_regs *regs)
{
	struct thread_info *ti = current_thread_info();
	struct irq_stage *caller_stage, *target_stage;
	struct irq_stage_data *p, *this_context;
	unsigned long flags;
	int ret = 0;

	/*
	 * We should definitely not pipeline a syscall through the
	 * slow path with IRQs off.
	 */
	WARN_ON_ONCE(dovetail_debug() && hard_irqs_disabled());

	if (!dovetail_enabled)
		return 0;

	flags = hard_local_irq_save();
	caller_stage = current_irq_stage;
	this_context = current_irq_staged;
	target_stage = &oob_stage;
next:
	p = this_staged(target_stage);
	set_current_irq_staged(p);
	hard_local_irq_restore(flags);
	ret = handle_pipelined_syscall(caller_stage, regs);
	flags = hard_local_irq_save();
	/*
	 * Be careful about stage switching _and_ CPU migration that
	 * might have happened as a result of handing over the syscall
	 * to the out-of-band handler.
	 *
	 * - if a stage migration is detected, fetch the new
	 * per-stage, per-CPU context pointer.
	 *
	 * - if no stage migration happened, switch back to the
	 * initial call stage, on a possibly different CPU though.
	 */
	if (current_irq_stage != target_stage) {
		this_context = current_irq_staged;
	} else {
		p = this_staged(this_context->stage);
		set_current_irq_staged(p);
	}

	if (this_context->stage == &inband_stage) {
		if (target_stage != &inband_stage && ret == 0) {
			target_stage = &inband_stage;
			goto next;
		}
		p = this_inband_staged();
		if (stage_irqs_pending(p))
			sync_current_irq_stage();
	} else {
		if (test_ti_thread_flag(ti, TIF_MAYDAY))
			call_mayday(ti, regs);
	}

	hard_local_irq_restore(flags);

	return ret;
}

int pipeline_syscall(unsigned int nr, struct pt_regs *regs)
{
	struct thread_info *ti = current_thread_info();
	unsigned long local_flags = READ_ONCE(ti_local_flags(ti));
	int ret;

	WARN_ON_ONCE(dovetail_debug() && hard_irqs_disabled());

	/*
	 * If __OOB_SYSCALL_BIT is set into the syscall number and we
	 * are running out-of-band, pass the request directly to the
	 * companion core by calling the oob syscall handler.
	 *
	 * Otherwise, if __OOB_SYSCALL_BIT is set or alternate
	 * scheduling is enabled for the caller, propagate the syscall
	 * through the pipeline stages, so that:
	 *
	 * - the core can manipulate the current execution stage for
	 * handling the request, which includes switching the current
	 * thread back to the in-band context if the syscall is a
	 * native one, or promoting it to the oob stage if handling an
	 * oob syscall requires this.
	 *
	 * - the core can receive the initial oob syscall a thread
	 * might have to emit for enabling dovetailing from the
	 * in-band stage.
	 *
	 * Native syscalls from common (non-dovetailed) threads are
	 * not subject to pipelining, but flow down to the in-band
	 * system call handler directly.
	 *
	 * Sanity check: we bark on returning from a syscall on a
	 * stalled in-band stage, which combined with running with
	 * hard irqs on might cause interrupts to linger in the log
	 * after exiting to user.
	 */

	if ((nr & __OOB_SYSCALL_BIT) && (local_flags & _TLF_OOB)) {
		handle_oob_syscall(regs);
		local_flags = READ_ONCE(ti_local_flags(ti));
		if (local_flags & _TLF_OOB) {
			if (test_ti_thread_flag(ti, TIF_MAYDAY))
				dovetail_call_mayday(regs);
			return 1; /* don't pass down, no tail work. */
		} else {
			WARN_ON_ONCE(dovetail_debug() && irqs_disabled());
			return -1; /* don't pass down, do tail work. */
		}
	}

	if ((local_flags & _TLF_DOVETAIL) || (nr & __OOB_SYSCALL_BIT)) {
		ret = __pipeline_syscall(regs);
		local_flags = READ_ONCE(ti_local_flags(ti));
		if (local_flags & _TLF_OOB)
			return 1; /* don't pass down, no tail work. */
		if (ret) {
			WARN_ON_ONCE(dovetail_debug() && irqs_disabled());
			return -1; /* don't pass down, do tail work. */
		}
	}

	return 0; /* pass syscall down to the in-band dispatcher. */
}

void __weak handle_oob_trap_entry(unsigned int trapnr, struct pt_regs *regs)
{
}

noinstr void __oob_trap_notify(unsigned int exception,
			       struct pt_regs *regs)
{
	unsigned long flags;

	/*
	 * We send a notification about exceptions raised over a
	 * registered oob stage only. The trap_entry handler expects
	 * hard irqs off on entry. It may demote the current context
	 * to the in-band stage, may return with hard irqs on.
	 */
	if (dovetail_enabled) {
		set_thread_local_flags(_TLF_OOBTRAP);
		flags = hard_local_irq_save();
		instrumentation_begin();
		handle_oob_trap_entry(exception, regs);
		instrumentation_end();
		hard_local_irq_restore(flags);
	}
}

void __weak handle_oob_trap_exit(unsigned int trapnr, struct pt_regs *regs)
{
}

noinstr void __oob_trap_unwind(unsigned int exception, struct pt_regs *regs)
{
	/*
	 * The trap_exit handler runs only if trap_entry was called
	 * for the same trap occurrence. It expects hard irqs off on
	 * entry, may switch the current context back to the oob
	 * stage. Must return with hard irqs off.
	 */
	hard_local_irq_disable();
	clear_thread_local_flags(_TLF_OOBTRAP);
	instrumentation_begin();
	handle_oob_trap_exit(exception, regs);
	instrumentation_end();
}

extern void rust_handle_inband_event(enum inband_event_type event, void *data);

void __weak handle_inband_event(enum inband_event_type event, void *data)
{
	// pr_info("rust_handle_inband_event in");
	rust_handle_inband_event(event,data);
}

void inband_event_notify(enum inband_event_type event, void *data)
{
	check_inband_stage();

	if (dovetail_enabled)
		handle_inband_event(event, data);
}

extern void rust_resume_oob_task(void *ptr, int cpu);

void __weak resume_oob_task(struct task_struct *p)
{
	void *thread = p->thread_info.oob_state.thread;
	pr_info("the passed thread ptr is %px", thread);
	rust_resume_oob_task(thread, task_cpu(p));
}

static void finalize_oob_transition(void) /* hard IRQs off */
{
	struct irq_pipeline_data *pd;
	struct irq_stage_data *p;
	struct task_struct *t;

	pd = raw_cpu_ptr(&irq_pipeline);
	t = pd->task_inflight;
	if (t == NULL)
		return;

	/*
	 * @t which is in flight to the oob stage might have received
	 * a signal while waiting in off-stage state to be actually
	 * scheduled out. We can't act upon that signal safely from
	 * here, we simply let the task complete the migration process
	 * to the oob stage. The pending signal will be handled when
	 * the task eventually exits the out-of-band context by the
	 * converse migration.
	 */
	pd->task_inflight = NULL;

	/*
	 * The transition handler in the companion core assumes the
	 * oob stage is stalled, fix this up.
	 */
	stall_oob();
	resume_oob_task(t);
	unstall_oob();
	p = this_oob_staged();
	if (stage_irqs_pending(p))
		/* Current stage (in-band) != p->stage (oob). */
		sync_irq_stage(p->stage);
}

void oob_trampoline(void)
{
	unsigned long flags;

	check_inband_stage();
	flags = hard_local_irq_save();
	finalize_oob_transition();
	hard_local_irq_restore(flags);
}

bool inband_switch_tail(void)
{
	bool oob;

	check_hard_irqs_disabled();

	/*
	 * We may run this code either over the inband or oob
	 * contexts. If inband, we may have a thread blocked in
	 * dovetail_leave_inband(), waiting for the companion core to
	 * schedule it back in over the oob context, in which case
	 * finalize_oob_transition() should take care of it. If oob,
	 * the core just switched us back, and we may update the
	 * context markers before returning to context_switch().
	 *
	 * Since the preemption count does not reflect the active
	 * stage yet upon inband -> oob transition, we figure out
	 * which one we are on by testing _TLF_OFFSTAGE. Having this
	 * bit set when running the inband switch tail code means that
	 * we are completing such transition for the current task,
	 * switched in by dovetail_context_switch() over the oob
	 * stage. If so, update the context markers appropriately.
	 */
	oob = test_thread_local_flags(_TLF_OFFSTAGE);
	if (oob) {
		/*
		 * The companion core assumes a stalled stage on exit
		 * from dovetail_leave_inband().
		 */
		stall_oob();
		set_thread_local_flags(_TLF_OOB);
		if (!IS_ENABLED(CONFIG_HAVE_PERCPU_PREEMPT_COUNT)) {
			WARN_ON_ONCE(dovetail_debug() &&
				(preempt_count() & STAGE_MASK));
			preempt_count_add(STAGE_OFFSET);
		}
	} else {
		finalize_oob_transition();
		hard_local_irq_enable();
	}

	return oob;
}

void __weak inband_clock_was_set(void)
{
}

void __weak install_inband_fd(unsigned int fd, struct file *file,
			      struct files_struct *files)
{
}

extern void rust_uninstall_inband_fd(unsigned int fd, struct file *file,
				struct files_struct *files);

void __weak uninstall_inband_fd(unsigned int fd, struct file *file,
				struct files_struct *files)
{
	// pr_info("the address files in the c is %px", files);
	// pr_info("the address file in the c is %px", file);
	// pr_info("the fd is %d", fd);
	rust_uninstall_inband_fd(fd, file, files);
}

void __weak replace_inband_fd(unsigned int fd, struct file *file,
			      struct files_struct *files)
{
}

int dovetail_start(void)
{
	check_inband_stage();

	if (dovetail_enabled)
		return -EBUSY;

	if (!oob_stage_present())
		return -EAGAIN;

	dovetail_enabled = true;
	smp_wmb();

	return 0;
}
EXPORT_SYMBOL_GPL(dovetail_start);

void dovetail_stop(void)
{
	check_inband_stage();

	dovetail_enabled = false;
	smp_wmb();
}
EXPORT_SYMBOL_GPL(dovetail_stop);
