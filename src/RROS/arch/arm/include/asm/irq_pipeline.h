/*
 * SPDX-License-Identifier: GPL-2.0
 *
 * Copyright (C) 2016 Philippe Gerum  <rpm@xenomai.org>.
 */
#ifndef _ASM_ARM_IRQ_PIPELINE_H
#define _ASM_ARM_IRQ_PIPELINE_H

#include <asm-generic/irq_pipeline.h>

#ifdef CONFIG_IRQ_PIPELINE

/*
 * In order to cope with the limited number of SGIs available to us,
 * In-band IPI messages are multiplexed over SGI0, whereas out-of-band
 * IPIs are directly mapped to SGI1-2.
 */
#define OOB_NR_IPI		2
#define OOB_IPI_OFFSET		1 /* SGI1 */
#define TIMER_OOB_IPI		(ipi_irq_base + OOB_IPI_OFFSET)
#define RESCHEDULE_OOB_IPI	(TIMER_OOB_IPI + 1)

extern int ipi_irq_base;

static inline notrace
unsigned long arch_irqs_virtual_to_native_flags(int stalled)
{
	return (!!stalled) << IRQMASK_I_POS;
}

static inline notrace
unsigned long arch_irqs_native_to_virtual_flags(unsigned long flags)
{
	return (!!hard_irqs_disabled_flags(flags)) << IRQMASK_i_POS;
}

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

static inline int arch_irqs_disabled_flags(unsigned long flags)
{
	return native_irqs_disabled_flags(flags);
}

static inline notrace void arch_local_irq_restore(unsigned long flags)
{
	inband_irq_restore(arch_irqs_disabled_flags(flags));
	barrier();
}

static inline
void arch_save_timer_regs(struct pt_regs *dst, struct pt_regs *src)
{
	dst->ARM_cpsr = src->ARM_cpsr;
	dst->ARM_pc = src->ARM_pc;
}

static inline bool arch_steal_pipelined_tick(struct pt_regs *regs)
{
	return !!(regs->ARM_cpsr & IRQMASK_I_BIT);
}

static inline int arch_enable_oob_stage(void)
{
	return 0;
}

extern void (*handle_arch_irq)(struct pt_regs *);

static inline void arch_handle_irq_pipelined(struct pt_regs *regs)
{
	handle_arch_irq(regs);
}

#define arch_kentry_get_irqstate(__regs)		\
	({						\
		to_svc_pt_regs(__regs)->irqstate;	\
	})

#define arch_kentry_set_irqstate(__regs, __irqstate)		\
	do {							\
		to_svc_pt_regs(__regs)->irqstate = __irqstate;	\
	} while (0)

#else /* !CONFIG_IRQ_PIPELINE */

static inline unsigned long arch_local_irq_save(void)
{
	return native_irq_save();
}

static inline void arch_local_irq_enable(void)
{
	native_irq_enable();
}

static inline void arch_local_irq_disable(void)
{
	native_irq_disable();
}

static inline unsigned long arch_local_save_flags(void)
{
	return native_save_flags();
}

static inline void arch_local_irq_restore(unsigned long flags)
{
	native_irq_restore(flags);
}

static inline int arch_irqs_disabled_flags(unsigned long flags)
{
	return native_irqs_disabled_flags(flags);
}

#endif /* !CONFIG_IRQ_PIPELINE */

#endif /* _ASM_ARM_IRQ_PIPELINE_H */
