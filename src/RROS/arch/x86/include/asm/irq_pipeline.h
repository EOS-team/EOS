/*
 * SPDX-License-Identifier: GPL-2.0
 *
 * Copyright (C) 2019 Philippe Gerum  <rpm@xenomai.org>.
 */
#ifndef _ASM_X86_IRQ_PIPELINE_H
#define _ASM_X86_IRQ_PIPELINE_H

#include <asm-generic/irq_pipeline.h>

#ifdef CONFIG_IRQ_PIPELINE

#include <asm/ptrace.h>

#define FIRST_SYSTEM_IRQ	NR_IRQS
#define TIMER_OOB_IPI		apicm_vector_irq(TIMER_OOB_VECTOR)
#define RESCHEDULE_OOB_IPI	apicm_vector_irq(RESCHEDULE_OOB_VECTOR)
#define apicm_irq_vector(__irq) ((__irq) - FIRST_SYSTEM_IRQ + FIRST_SYSTEM_VECTOR)
#define apicm_vector_irq(__vec) ((__vec) - FIRST_SYSTEM_VECTOR + FIRST_SYSTEM_IRQ)

#define X86_EFLAGS_SS_BIT	31

static inline notrace
unsigned long arch_irqs_virtual_to_native_flags(int stalled)
{
	return (!stalled) << X86_EFLAGS_IF_BIT;
}

static inline notrace
unsigned long arch_irqs_native_to_virtual_flags(unsigned long flags)
{
	return hard_irqs_disabled_flags(flags) << X86_EFLAGS_SS_BIT;
}

#ifndef CONFIG_PARAVIRT_XXL

static inline notrace unsigned long arch_local_irq_save(void)
{
	int stalled = inband_irq_save();
	barrier();
	return arch_irqs_virtual_to_native_flags(stalled);
}

static inline notrace void arch_local_irq_enable(void)
{
	barrier();
	inband_irq_enable();
}

static inline notrace void arch_local_irq_disable(void)
{
	inband_irq_disable();
	barrier();
}

static inline notrace unsigned long arch_local_save_flags(void)
{
	int stalled = inband_irqs_disabled();
	barrier();
	return arch_irqs_virtual_to_native_flags(stalled);
}

#endif /* !CONFIG_PARAVIRT_XXL */

static inline notrace void arch_local_irq_restore(unsigned long flags)
{
	inband_irq_restore(native_irqs_disabled_flags(flags));
	barrier();
}

static inline
void arch_save_timer_regs(struct pt_regs *dst, struct pt_regs *src)
{
	dst->flags = src->flags;
	dst->cs = src->cs;
	dst->ip = src->ip;
	dst->bp = src->bp;
	dst->ss = src->ss;
	dst->sp = src->sp;
}

static inline bool arch_steal_pipelined_tick(struct pt_regs *regs)
{
	return !(regs->flags & X86_EFLAGS_IF);
}

static inline int arch_enable_oob_stage(void)
{
	return 0;
}

static inline void arch_handle_irq_pipelined(struct pt_regs *regs)
{ }

#else /* !CONFIG_IRQ_PIPELINE */

struct pt_regs;

#ifndef CONFIG_PARAVIRT_XXL

static inline notrace unsigned long arch_local_save_flags(void)
{
	return native_save_fl();
}

static inline notrace void arch_local_irq_disable(void)
{
	native_irq_disable();
}

static inline notrace void arch_local_irq_enable(void)
{
	native_irq_enable();
}

/*
 * For spinlocks, etc:
 */
static inline notrace unsigned long arch_local_irq_save(void)
{
	unsigned long flags = arch_local_save_flags();
	arch_local_irq_disable();
	return flags;
}

#endif /* !CONFIG_PARAVIRT_XXL */

#endif /* !CONFIG_IRQ_PIPELINE */

#endif /* _ASM_X86_IRQ_PIPELINE_H */
