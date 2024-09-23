/*
 * SPDX-License-Identifier: GPL-2.0
 *
 * Copyright (C) 2019 Philippe Gerum  <rpm@xenomai.org>.
 */
#include <linux/kernel.h>
#include <linux/smp.h>
#include <linux/irq.h>
#include <linux/irq_pipeline.h>
#include <asm/irqdomain.h>
#include <asm/apic.h>
#include <asm/traps.h>
#include <asm/irq_work.h>
#include <asm/mshyperv.h>
#include <asm/idtentry.h>

static struct irq_domain *sipic_domain;

static void sipic_irq_noop(struct irq_data *data) { }

static unsigned int sipic_irq_noop_ret(struct irq_data *data)
{
	return 0;
}

static struct irq_chip sipic_chip = {
	.name		= "SIPIC",
	.irq_startup	= sipic_irq_noop_ret,
	.irq_shutdown	= sipic_irq_noop,
	.irq_enable	= sipic_irq_noop,
	.irq_disable	= sipic_irq_noop,
	.flags		= IRQCHIP_PIPELINE_SAFE | IRQCHIP_SKIP_SET_WAKE,
};

void handle_apic_irq(struct irq_desc *desc)
{
	if (WARN_ON_ONCE(irq_pipeline_debug() && !on_pipeline_entry()))
		return;

	/*
	 * MCE events are non-maskable therefore their in-band
	 * handlers have to be oob-compatible by construction. Those
	 * handlers run immediately out of the IDT for this reason as
	 * well. We won't see them here since they are not routed via
	 * arch_handle_irq() -> generic_pipeline_irq().
	 *
	 * All we need to do at this stage is to acknowledge other
	 * APIC events, then pipeline the corresponding interrupt from
	 * our synthetic controller chip (SIPIC).
	 */
	__ack_APIC_irq();

	handle_oob_irq(desc);
}

void irq_send_oob_ipi(unsigned int ipi,
		const struct cpumask *cpumask)
{
	apic->send_IPI_mask_allbutself(cpumask,	apicm_irq_vector(ipi));
}
EXPORT_SYMBOL_GPL(irq_send_oob_ipi);

static irqentry_state_t pipeline_enter_rcu(void)
{
	irqentry_state_t state = {
		.exit_rcu = false,
		.stage_info = IRQENTRY_INBAND_UNSTALLED,
	};

	if (!IS_ENABLED(CONFIG_TINY_RCU) && is_idle_task(current)) {
		rcu_irq_enter();
		state.exit_rcu = true;
	} else {
		rcu_irq_enter_check_tick();
	}

	return state;
}

static void pipeline_exit_rcu(irqentry_state_t state)
{
	if (state.exit_rcu)
		rcu_irq_exit();
}

void uv_bau_message_interrupt(struct pt_regs *regs);

static void do_sysvec_inband(struct irq_desc *desc, struct pt_regs *regs)
{
	unsigned int irq = irq_desc_get_irq(desc);
	int vector = apicm_irq_vector(irq);

	/*
	 * This code only sees pipelined sysvec events tagged with
	 * DEFINE_IDTENTRY_SYSVEC_PIPELINED:
	 *
	 * 	arch_handle_irq(irq)
	 *		generic_pipeline_irq(irq)
	 *			handle_apic_irq(irq)
	 *				handle_oob_irq(irq)
	 *				[...irq_post_inband...]
	 *
	 *      arch_do_IRQ_pipelined(desc)
	 *           do_sysvec_inband(desc)
	 *               <switch_to_irqstack>
	 *                     |
	 *                     v
	 *	         sysvec_handler(regs)
	 *
	 * System vectors which are still tagged as
	 * DEFINE_IDTENTRY_SYSVEC/DEFINE_IDTENTRY_SYSVEC_SIMPLE are
	 * directly dispatched out of the IDT, assuming their handler
	 * is oob-safe (like NMI handlers) therefore never reach this
	 * in-band stage handler.
	 *
	 * NOTE: we expand run_sysvec_on_irqstack_cond() each time,
	 * which is ugly. But the irqstack code makes assumptions we
	 * don't want to break.
	 */

	switch (vector) {
#ifdef CONFIG_SMP
	case RESCHEDULE_VECTOR:
		run_sysvec_on_irqstack_cond(__sysvec_reschedule_ipi,
					regs);
		break;
	case CALL_FUNCTION_VECTOR:
		run_sysvec_on_irqstack_cond(__sysvec_call_function,
					regs);
		break;
	case CALL_FUNCTION_SINGLE_VECTOR:
		run_sysvec_on_irqstack_cond(__sysvec_call_function_single,
					regs);
		break;
	case REBOOT_VECTOR:
		run_sysvec_on_irqstack_cond(__sysvec_reboot, regs);
		break;
#endif
	case X86_PLATFORM_IPI_VECTOR:
		run_sysvec_on_irqstack_cond(__sysvec_x86_platform_ipi,
					regs);
		break;
	case IRQ_WORK_VECTOR:
		run_sysvec_on_irqstack_cond(__sysvec_irq_work, regs);
		break;
#ifdef CONFIG_X86_UV
	case UV_BAU_MESSAGE:
		run_sysvec_on_irqstack_cond(__sysvec_uv_bau_message,
					regs);
		break;
#endif
#ifdef CONFIG_HAVE_KVM
	case POSTED_INTR_VECTOR:
		run_sysvec_on_irqstack_cond(__sysvec_kvm_posted_intr_ipi,
					regs);
		break;
	case POSTED_INTR_WAKEUP_VECTOR:
		run_sysvec_on_irqstack_cond(__sysvec_kvm_posted_intr_wakeup_ipi,
					regs);
		break;
	case POSTED_INTR_NESTED_VECTOR:
		run_sysvec_on_irqstack_cond(__sysvec_kvm_posted_intr_nested_ipi,
					regs);
		break;
#endif
#ifdef CONFIG_HYPERV
	case HYPERVISOR_CALLBACK_VECTOR:
		run_sysvec_on_irqstack_cond(__sysvec_hyperv_callback,
					regs);
		break;
	case HYPERV_REENLIGHTENMENT_VECTOR:
		run_sysvec_on_irqstack_cond(__sysvec_hyperv_reenlightenment,
					regs);
		break;
	case HYPERV_STIMER0_VECTOR:
		run_sysvec_on_irqstack_cond(__sysvec_hyperv_stimer0,
					regs);
		break;
#endif
#ifdef CONFIG_ACRN_GUEST
	case HYPERVISOR_CALLBACK_VECTOR:
		run_sysvec_on_irqstack_cond(__sysvec_acrn_hv_callback,
					regs);
		break;
#endif
#ifdef CONFIG_XEN_PVHVM
	case HYPERVISOR_CALLBACK_VECTOR:
		run_sysvec_on_irqstack_cond(__sysvec_xen_hvm_callback,
					regs);
		break;
#endif
	case LOCAL_TIMER_VECTOR:
		run_sysvec_on_irqstack_cond(__sysvec_apic_timer_interrupt,
					regs);
		break;
	default:
		printk_once(KERN_ERR "irq_pipeline: unexpected event"
			" on vector #%.2x (irq=%u)", vector, irq);
		return;
	}
}

static void do_irq_inband(struct pt_regs *regs, u32 irq)
{

	struct irq_desc *desc = irq_to_desc(irq);

	desc->handle_irq(desc);
}

void arch_do_IRQ_pipelined(struct irq_desc *desc)
{
	struct pt_regs *regs = raw_cpu_ptr(&irq_pipeline.tick_regs), *old_regs;
	irqentry_state_t state;

	/* Emulate a kernel entry. */
	state = pipeline_enter_rcu();

	if (desc->irq_data.domain == sipic_domain) {
 		do_sysvec_inband(desc, regs);
	} else {
		/*
		 * XXX: the following is ugly, but the irqstack
		 * switching code is not that flexible. However, we
		 * don't want to provide a separate implementation of
		 * the latter in the pipelined case, so let's get
		 * along with it.
		 */
		u32 vector = irq_desc_get_irq(desc); /* irq carried as 'vector' */
		old_regs = set_irq_regs(regs);
		run_irq_on_irqstack_cond(do_irq_inband, regs, vector);
		set_irq_regs(old_regs);
	}

	pipeline_exit_rcu(state);
}

void arch_handle_irq(struct pt_regs *regs, u8 vector, bool irq_movable)
{
	struct irq_desc *desc;
	unsigned int irq;

	if (vector >= FIRST_SYSTEM_VECTOR) {
		irq = apicm_vector_irq(vector);
	} else {
		desc = __this_cpu_read(vector_irq[vector]);
		if (unlikely(IS_ERR_OR_NULL(desc))) {
			if (desc == VECTOR_UNUSED) {
				pr_emerg_ratelimited("%s: %d.%u No irq handler for vector\n",
						__func__, smp_processor_id(),
						vector);
			} else {
				__this_cpu_write(vector_irq[vector], VECTOR_UNUSED);
			}
			return;
		}
		if (irqd_is_setaffinity_pending(&desc->irq_data)) {
			raw_spin_lock(&desc->lock);
			if (irq_movable)
				irqd_clr_move_blocked(&desc->irq_data);
			else
				irqd_set_move_blocked(&desc->irq_data);
			raw_spin_unlock(&desc->lock);
		}
		irq = irq_desc_get_irq(desc);
	}

	generic_pipeline_irq(irq, regs);
}

noinstr void arch_pipeline_entry(struct pt_regs *regs, u8 vector)
{
	struct irq_stage_data *prevd;
	irqentry_state_t state;

	/*
	 * The tricky one: we distinguish the following cases:
	 *
	 * [1] entry from oob context, either kernel or user code was
	 * preempted by the IRQ, the in-band (virtual) interrupt state
	 * is 'undefined' (could be either stalled/unstalled, it is
	 * not relevant).
	 *
	 * [2] entry from in-band context while the stage is stalled,
	 * which means that some kernel code was preempted by the IRQ
	 * since in-band user code cannot run with interrupts
	 * (virtually) disabled.
	 *
	 * [3] entry from in-band context while the stage is
	 * unstalled: the common case for IRQ entry. Kernel or user
	 * code may have been preempted, we handle the event
	 * identically.
	 *
	 * [1] and [2] are processed almost the same way, except for
	 * one key aspect: the potential stage demotion of the
	 * preempted task which originally entered [1] on the oob
	 * stage, then left it for the in-band stage as a result of
	 * handling the IRQ (such demotion normally happens during
	 * handle_irq_pipelined_finish() if required). In this
	 * particular case, we want to run the common IRQ epilogue
	 * code before returning to user mode, so that all pending
	 * in-band work (_TIF_WORK_*) is carried out for the task
	 * which is about to exit kernel mode.
	 *
	 * If the task runs in-band at the exit point and a user mode
	 * context was preempted, then case [2] is excluded by
	 * definition so we know for sure that we just observed a
	 * stage demotion, therefore we have to run the work loop by
	 * calling irqentry_exit_to_user_mode().
	 */
	if (unlikely(running_oob() || irqs_disabled())) {
		instrumentation_begin();
		prevd = handle_irq_pipelined_prepare(regs);
		arch_handle_irq(regs, vector, false);
		kvm_set_cpu_l1tf_flush_l1d();
		handle_irq_pipelined_finish(prevd, regs);
		if (running_inband() && user_mode(regs)) {
			stall_inband_nocheck();
			irqentry_exit_to_user_mode(regs);
		}
		instrumentation_end();
		return;
	}

	/* In-band on entry, accepting interrupts. */
	state = irqentry_enter(regs);
	instrumentation_begin();
	/* Prep for handling, switching oob. */
	prevd = handle_irq_pipelined_prepare(regs);
	arch_handle_irq(regs, vector, true);
	kvm_set_cpu_l1tf_flush_l1d();
	/* irqentry_enter() stalled the in-band stage. */
	trace_hardirqs_on();
	unstall_inband_nocheck();
	handle_irq_pipelined_finish(prevd, regs);
	stall_inband_nocheck();
	trace_hardirqs_off();
	instrumentation_end();
	irqentry_exit(regs, state);
}

static int sipic_irq_map(struct irq_domain *d, unsigned int irq,
			irq_hw_number_t hwirq)
{
	irq_set_percpu_devid(irq);
	irq_set_chip_and_handler(irq, &sipic_chip, handle_apic_irq);

	return 0;
}

static struct irq_domain_ops sipic_domain_ops = {
	.map	= sipic_irq_map,
};

static void create_x86_apic_domain(void)
{
	sipic_domain = irq_domain_add_simple(NULL, NR_APIC_VECTORS,
					     FIRST_SYSTEM_IRQ,
					     &sipic_domain_ops, NULL);
}

#ifdef CONFIG_SMP

DEFINE_IDTENTRY_SYSVEC_PIPELINED(RESCHEDULE_OOB_VECTOR,
				 sysvec_reschedule_oob_ipi)
{ /* In-band handler is unused. */ }

DEFINE_IDTENTRY_SYSVEC_PIPELINED(TIMER_OOB_VECTOR,
				 sysvec_timer_oob_ipi)
{ /* In-band handler is unused. */ }

void handle_irq_move_cleanup(struct irq_desc *desc)
{
	if (on_pipeline_entry()) {
		/* 1. on receipt from hardware. */
		__ack_APIC_irq();
		handle_oob_irq(desc);
	} else {
		/* 2. in-band delivery. */
		__sysvec_irq_move_cleanup(NULL);
	}
}

static void smp_setup(void)
{
	int irq;

	/*
	 * The IRQ cleanup event must be pipelined to the inband
	 * stage, so we need a valid IRQ descriptor for it. Since we
	 * still are in the early boot stage on CPU0, we ask for a 1:1
	 * mapping between the vector number and IRQ number, to make
	 * things easier for us later on.
	 */
	irq = irq_alloc_desc_at(IRQ_MOVE_CLEANUP_VECTOR, 0);
	WARN_ON(IRQ_MOVE_CLEANUP_VECTOR != irq);
	/*
	 * Set up the vector_irq[] mapping array for the boot CPU,
	 * other CPUs will copy this entry when their APIC is going
	 * online (see lapic_online()).
	 */
	per_cpu(vector_irq, 0)[irq] = irq_to_desc(irq);

	irq_set_chip_and_handler(irq, &dummy_irq_chip,
				handle_irq_move_cleanup);
}

#else

static void smp_setup(void) { }

#endif

void __init arch_irq_pipeline_init(void)
{
	/*
	 * Create an IRQ domain for mapping APIC system interrupts
	 * (in-band and out-of-band), with fixed sirq numbers starting
	 * from FIRST_SYSTEM_IRQ. Upon receipt of a system interrupt,
	 * the corresponding sirq is injected into the pipeline.
	 */
	create_x86_apic_domain();

	smp_setup();
}
