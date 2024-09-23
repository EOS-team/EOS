/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * Copyright (C) 2012 ARM Ltd.
 */
#ifndef __ASM_IRQFLAGS_H
#define __ASM_IRQFLAGS_H

#include <asm/alternative.h>
#include <asm/barrier.h>
#include <asm/ptrace.h>
#include <asm/sysreg.h>

#define IRQMASK_I_BIT	PSR_I_BIT
#define IRQMASK_I_POS	7
#define IRQMASK_i_POS	31

/*
 * Aarch64 has flags for masking: Debug, Asynchronous (serror), Interrupts and
 * FIQ exceptions, in the 'daif' register. We mask and unmask them in 'daif'
 * order:
 * Masking debug exceptions causes all other exceptions to be masked too/
 * Masking SError masks IRQ/FIQ, but not debug exceptions. IRQ and FIQ are
 * always masked and unmasked together, and have no side effects for other
 * flags. Keeping to this order makes it easier for entry.S to know which
 * exceptions should be unmasked.
 */

/*
 * CPU interrupt mask handling.
 */
static inline void native_irq_enable(void)
{
	if (system_has_prio_mask_debugging()) {
		u32 pmr = read_sysreg_s(SYS_ICC_PMR_EL1);

		WARN_ON_ONCE(pmr != GIC_PRIO_IRQON && pmr != GIC_PRIO_IRQOFF);
	}

	asm volatile(ALTERNATIVE(
		"msr	daifclr, #3		// native_irq_enable",
		__msr_s(SYS_ICC_PMR_EL1, "%0"),
		ARM64_HAS_IRQ_PRIO_MASKING)
		:
		: "r" ((unsigned long) GIC_PRIO_IRQON)
		: "memory");

	pmr_sync();
}

static inline void native_irq_disable(void)
{
	if (system_has_prio_mask_debugging()) {
		u32 pmr = read_sysreg_s(SYS_ICC_PMR_EL1);

		WARN_ON_ONCE(pmr != GIC_PRIO_IRQON && pmr != GIC_PRIO_IRQOFF);
	}

	asm volatile(ALTERNATIVE(
		"msr	daifset, #3		// native_irq_disable",
		__msr_s(SYS_ICC_PMR_EL1, "%0"),
		ARM64_HAS_IRQ_PRIO_MASKING)
		:
		: "r" ((unsigned long) GIC_PRIO_IRQOFF)
		: "memory");
}

static inline void native_irq_sync(void)
{
	native_irq_enable();
	isb();
	native_irq_disable();
}

/*
 * Save the current interrupt enable state.
 */
static inline unsigned long native_save_flags(void)
{
	unsigned long flags;

	asm volatile(ALTERNATIVE(
		"mrs	%0, daif",
		__mrs_s("%0", SYS_ICC_PMR_EL1),
		ARM64_HAS_IRQ_PRIO_MASKING)
		: "=&r" (flags)
		:
		: "memory");

	return flags;
}

static inline int native_irqs_disabled_flags(unsigned long flags)
{
	int res;

	asm volatile(ALTERNATIVE(
		"and	%w0, %w1, #" __stringify(PSR_I_BIT),
		"eor	%w0, %w1, #" __stringify(GIC_PRIO_IRQON),
		ARM64_HAS_IRQ_PRIO_MASKING)
		: "=&r" (res)
		: "r" ((int) flags)
		: "memory");

	return res;
}

static inline unsigned long native_irq_save(void)
{
	unsigned long flags;

	flags = native_save_flags();

	/*
	 * There are too many states with IRQs disabled, just keep the current
	 * state if interrupts are already disabled/masked.
	 */
	if (!native_irqs_disabled_flags(flags))
		native_irq_disable();

	return flags;
}

/*
 * restore saved IRQ state
 */
static inline void native_irq_restore(unsigned long flags)
{
	asm volatile(ALTERNATIVE(
		"msr	daif, %0",
		__msr_s(SYS_ICC_PMR_EL1, "%0"),
		ARM64_HAS_IRQ_PRIO_MASKING)
		:
		: "r" (flags)
		: "memory");

	pmr_sync();
}

static inline bool native_irqs_disabled(void)
{
	unsigned long flags = native_save_flags();
	return native_irqs_disabled_flags(flags);
}

#include <asm/irq_pipeline.h>

#endif /* __ASM_IRQFLAGS_H */
