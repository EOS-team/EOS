/* SPDX-License-Identifier: GPL-2.0 */
#ifndef _X86_IRQFLAGS_H_
#define _X86_IRQFLAGS_H_

#include <asm/processor-flags.h>

#ifndef __ASSEMBLY__

#include <asm/nospec-branch.h>

/* Provide __cpuidle; we can't safely include <linux/cpu.h> */
#define __cpuidle __section(".cpuidle.text")

/*
 * Interrupt control:
 */

/* Declaration required for gcc < 4.9 to prevent -Werror=missing-prototypes */
extern inline unsigned long native_save_fl(void);
extern __always_inline unsigned long native_save_fl(void)
{
	unsigned long flags;

	/*
	 * "=rm" is safe here, because "pop" adjusts the stack before
	 * it evaluates its effective address -- this is part of the
	 * documented behavior of the "pop" instruction.
	 */
	asm volatile("# __native_save_flags\n\t"
		     "pushf ; pop %0"
		     : "=rm" (flags)
		     : /* no input */
		     : "memory");

	return flags;
}

extern inline void native_restore_fl(unsigned long flags);
extern __always_inline void native_restore_fl(unsigned long flags)
{
	asm volatile("push %0 ; popf"
		     : /* no output */
		     :"g" (flags)
		     :"memory", "cc");
}

static __always_inline void native_irq_disable(void)
{
	asm volatile("cli": : :"memory");
}

static __always_inline void native_irq_enable(void)
{
	asm volatile("sti": : :"memory");
}

static inline unsigned long native_save_flags(void)
{
	return native_save_fl();
}

static __always_inline void native_irq_sync(void)
{
	asm volatile("sti ; nop ; cli": : :"memory");
}

static __always_inline unsigned long native_irq_save(void)
{
	unsigned long flags;

	flags = native_save_flags();

	native_irq_disable();

	return flags;
}

static __always_inline int native_irqs_disabled_flags(unsigned long flags)
{
	return !(flags & X86_EFLAGS_IF);
}

static __always_inline void native_irq_restore(unsigned long flags)
{
	/*
	 * CAUTION: the hard_irq_* API may be used to bracket code
	 * which re-enables interrupts inside save/restore pairs, so
	 * do not try to be (too) smart: do restore the original flags
	 * unconditionally.
	 */
	native_restore_fl(flags);
}

static __always_inline bool native_irqs_disabled(void)
{
	unsigned long flags = native_save_flags();
	return native_irqs_disabled_flags(flags);
}

static inline __cpuidle void native_safe_halt(void)
{
	mds_idle_clear_cpu_buffers();
	asm volatile("sti; hlt": : :"memory");
}

static inline __cpuidle void native_halt(void)
{
	mds_idle_clear_cpu_buffers();
	asm volatile("hlt": : :"memory");
}

#endif

#ifdef CONFIG_PARAVIRT_XXL
#include <asm/paravirt.h>
#else
#ifndef __ASSEMBLY__
#include <linux/types.h>
#include <asm/irq_pipeline.h>

/*
 * Used in the idle loop; sti takes one instruction cycle
 * to complete:
 */
static inline __cpuidle void arch_safe_halt(void)
{
	native_safe_halt();
}

/*
 * Used when interrupts are already enabled or to
 * shutdown the processor:
 */
static inline __cpuidle void halt(void)
{
	native_halt();
}

#else

#ifdef CONFIG_X86_64
#ifdef CONFIG_DEBUG_ENTRY
#define SAVE_FLAGS		pushfq; popq %rax
#endif

#define INTERRUPT_RETURN	jmp native_iret

#endif

#endif /* __ASSEMBLY__ */
#endif /* CONFIG_PARAVIRT_XXL */

#ifndef __ASSEMBLY__
static __always_inline int arch_irqs_disabled_flags(unsigned long flags)
{
	return native_irqs_disabled_flags(flags);
}

static __always_inline int arch_irqs_disabled(void)
{
	unsigned long flags = arch_local_save_flags();

	return arch_irqs_disabled_flags(flags);
}

#ifndef CONFIG_IRQ_PIPELINE
static inline notrace void arch_local_irq_restore(unsigned long flags)
{
	if (!arch_irqs_disabled_flags(flags))
		arch_local_irq_enable();
}
#endif

#else
#ifdef CONFIG_X86_64
#ifdef CONFIG_XEN_PV
#define SWAPGS	ALTERNATIVE "swapgs", "", X86_FEATURE_XENPV
#else
#define SWAPGS	swapgs
#endif
#endif
#endif /* !__ASSEMBLY__ */

#endif
