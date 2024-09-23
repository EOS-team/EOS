/*
 * SPDX-License-Identifier: GPL-2.0
 *
 * Copyright (C) 2016, 2019 Philippe Gerum  <rpm@xenomai.org>.
 */
#ifndef _LINUX_IRQSTAGE_H
#define _LINUX_IRQSTAGE_H

#ifdef CONFIG_IRQ_PIPELINE

#include <linux/percpu.h>
#include <linux/bitops.h>
#include <linux/preempt.h>
#include <linux/sched.h>
#include <asm/irq_pipeline.h>

struct kvm_oob_notifier;

struct irq_stage {
	int index;
	const char *name;
};

extern struct irq_stage inband_stage;

extern struct irq_stage oob_stage;

struct irq_event_map;

struct irq_log {
	unsigned long index_0;
	struct irq_event_map *map;
};

/* Per-CPU, per-stage data. */
struct irq_stage_data {
	struct irq_log log;
	struct irq_stage *stage;
#ifdef CONFIG_DEBUG_IRQ_PIPELINE
	int cpu;
#endif
};

/* Per-CPU pipeline descriptor. */
struct irq_pipeline_data {
	struct irq_stage_data stages[2];
	struct pt_regs tick_regs;
#ifdef CONFIG_DOVETAIL
	struct task_struct *task_inflight;
	struct task_struct *rqlock_owner;
#ifdef CONFIG_KVM
	struct kvm_oob_notifier *vcpu_notify;
#endif
#endif
};

DECLARE_PER_CPU(struct irq_pipeline_data, irq_pipeline);

/*
 * The low-level stall bit accessors. Should be used by the Dovetail
 * core implementation exclusively, inband_irq_*() and oob_irq_*()
 * accessors are available to common code.
 */

#define INBAND_STALL_BIT  0
#define OOB_STALL_BIT     1

static __always_inline void init_task_stall_bits(struct task_struct *p)
{
	__set_bit(INBAND_STALL_BIT, &p->stall_bits);
	__clear_bit(OOB_STALL_BIT, &p->stall_bits);
}

static __always_inline void stall_inband_nocheck(void)
{
	__set_bit(INBAND_STALL_BIT, &current->stall_bits);
	barrier();
}

static __always_inline void stall_inband(void)
{
	WARN_ON_ONCE(irq_pipeline_debug() && running_oob());
	stall_inband_nocheck();
}

static __always_inline void unstall_inband_nocheck(void)
{
	barrier();
	__clear_bit(INBAND_STALL_BIT, &current->stall_bits);
}

static __always_inline void unstall_inband(void)
{
	WARN_ON_ONCE(irq_pipeline_debug() && running_oob());
	unstall_inband_nocheck();
}

static __always_inline int test_and_stall_inband_nocheck(void)
{
	return __test_and_set_bit(INBAND_STALL_BIT, &current->stall_bits);
}

static __always_inline int test_and_stall_inband(void)
{
	WARN_ON_ONCE(irq_pipeline_debug() && running_oob());
	return test_and_stall_inband_nocheck();
}

static __always_inline int test_inband_stall(void)
{
	return test_bit(INBAND_STALL_BIT, &current->stall_bits);
}

static __always_inline void stall_oob(void)
{
	__set_bit(OOB_STALL_BIT, &current->stall_bits);
	barrier();
}

static __always_inline void unstall_oob(void)
{
	barrier();
	__clear_bit(OOB_STALL_BIT, &current->stall_bits);
}

static __always_inline int test_and_stall_oob(void)
{
	return __test_and_set_bit(OOB_STALL_BIT, &current->stall_bits);
}

static __always_inline int test_oob_stall(void)
{
	return test_bit(OOB_STALL_BIT, &current->stall_bits);
}

/**
 * this_staged - IRQ stage data on the current CPU
 *
 * Return the address of @stage's data on the current CPU. IRQs must
 * be hard disabled to prevent CPU migration.
 */
static __always_inline
struct irq_stage_data *this_staged(struct irq_stage *stage)
{
	return &raw_cpu_ptr(irq_pipeline.stages)[stage->index];
}

/**
 * percpu_inband_staged - IRQ stage data on specified CPU
 *
 * Return the address of @stage's data on @cpu.
 *
 * This is the slowest accessor, use it carefully. Prefer
 * this_staged() for requests referring to the current
 * CPU. Additionally, if the target stage is known at build time,
 * consider using this_{inband, oob}_staged() instead.
 */
static __always_inline
struct irq_stage_data *percpu_inband_staged(struct irq_stage *stage, int cpu)
{
	return &per_cpu(irq_pipeline.stages, cpu)[stage->index];
}

/**
 * this_inband_staged - return the address of the pipeline context
 * data for the inband stage on the current CPU. CPU migration must be
 * disabled.
 *
 * This accessor is recommended when the stage we refer to is known at
 * build time to be the inband one.
 */
static __always_inline struct irq_stage_data *this_inband_staged(void)
{
	return raw_cpu_ptr(&irq_pipeline.stages[0]);
}

/**
 * this_oob_staged - return the address of the pipeline context data
 * for the registered oob stage on the current CPU. CPU migration must
 * be disabled.
 *
 * This accessor is recommended when the stage we refer to is known at
 * build time to be the registered oob stage. This address is always
 * different from the context data of the inband stage, even in
 * absence of registered oob stage.
 */
static __always_inline struct irq_stage_data *this_oob_staged(void)
{
	return raw_cpu_ptr(&irq_pipeline.stages[1]);
}

static __always_inline struct irq_stage_data *__current_irq_staged(void)
{
	return &raw_cpu_ptr(irq_pipeline.stages)[stage_level()];
}

/**
 * current_irq_staged - return the address of the pipeline context
 * data for the current stage. CPU migration must be disabled.
 */
#define current_irq_staged __current_irq_staged()

static __always_inline
void check_staged_locality(struct irq_stage_data *pd)
{
#ifdef CONFIG_DEBUG_IRQ_PIPELINE
	/*
	 * Setting our context with another processor's is a really
	 * bad idea, our caller definitely went loopy.
	 */
	WARN_ON_ONCE(raw_smp_processor_id() != pd->cpu);
#endif
}

/**
 * switch_oob(), switch_inband() - switch the current CPU to the
 * specified stage context. CPU migration must be disabled.
 *
 * Calling these routines is the only sane and safe way to change the
 * interrupt stage for the current CPU. Don't bypass them, ever.
 * Really.
 */
static __always_inline
void switch_oob(struct irq_stage_data *pd)
{
	check_staged_locality(pd);
	if (!(preempt_count() & STAGE_MASK))
		preempt_count_add(STAGE_OFFSET);
}

static __always_inline
void switch_inband(struct irq_stage_data *pd)
{
	check_staged_locality(pd);
	if (preempt_count() & STAGE_MASK)
		preempt_count_sub(STAGE_OFFSET);
}

static __always_inline
void set_current_irq_staged(struct irq_stage_data *pd)
{
	if (pd->stage == &inband_stage)
		switch_inband(pd);
	else
		switch_oob(pd);
}

static __always_inline struct irq_stage *__current_irq_stage(void)
{
	/*
	 * We don't have to hard disable irqs while accessing the
	 * per-CPU stage data here, because there is no way we could
	 * switch stage and CPU at the same time.
	 */
	return __current_irq_staged()->stage;
}

#define current_irq_stage	__current_irq_stage()

static __always_inline bool oob_stage_present(void)
{
	return oob_stage.index != 0;
}

/**
 * stage_irqs_pending() - Whether we have interrupts pending
 * (i.e. logged) on the current CPU for the given stage. Hard IRQs
 * must be disabled.
 */
static __always_inline int stage_irqs_pending(struct irq_stage_data *pd)
{
	return pd->log.index_0 != 0;
}

void sync_current_irq_stage(void);

void sync_irq_stage(struct irq_stage *top);

void irq_post_stage(struct irq_stage *stage,
		    unsigned int irq);

static __always_inline void irq_post_oob(unsigned int irq)
{
	irq_post_stage(&oob_stage, irq);
}

static __always_inline void irq_post_inband(unsigned int irq)
{
	irq_post_stage(&inband_stage, irq);
}

static __always_inline void oob_irq_disable(void)
{
	hard_local_irq_disable();
	stall_oob();
}

static __always_inline unsigned long oob_irq_save(void)
{
	hard_local_irq_disable();
	return test_and_stall_oob();
}

static __always_inline int oob_irqs_disabled(void)
{
	return test_oob_stall();
}

void oob_irq_enable(void);

void __oob_irq_restore(unsigned long x);

static __always_inline void oob_irq_restore(unsigned long x)
{
	if ((x ^ test_oob_stall()) & 1)
		__oob_irq_restore(x);
}

bool stage_disabled(void);

unsigned long test_and_lock_stage(int *irqsoff);

void unlock_stage(unsigned long irqstate);

#define stage_save_flags(__irqstate)					\
  	do {								\
	  unsigned long __flags = hard_local_save_flags();		\
	  (__irqstate) = irqs_merge_flags(__flags,			\
					  irqs_disabled());		\
	} while (0)

int enable_oob_stage(const char *name);

int arch_enable_oob_stage(void);

void disable_oob_stage(void);

#else /* !CONFIG_IRQ_PIPELINE */

#include <linux/irqflags.h>

void call_is_nop_without_pipelining(void);

static __always_inline void stall_inband(void) { }

static __always_inline void unstall_inband(void) { }

static __always_inline int test_and_stall_inband(void)
{
	return false;
}

static __always_inline int test_inband_stall(void)
{
	return false;
}

static __always_inline bool oob_stage_present(void)
{
	return false;
}

static __always_inline bool stage_disabled(void)
{
	return irqs_disabled();
}

static __always_inline void irq_post_inband(unsigned int irq)
{
	call_is_nop_without_pipelining();
}

#define test_and_lock_stage(__irqsoff)				\
	({							\
		unsigned long __flags;				\
		raw_local_irq_save(__flags);			\
		*(__irqsoff) = irqs_disabled_flags(__flags);	\
		__flags;					\
	})

#define unlock_stage(__flags)		raw_local_irq_restore(__flags)

#define stage_save_flags(__flags)	raw_local_save_flags(__flags)

static __always_inline void stall_inband_nocheck(void)
{ }

static __always_inline void unstall_inband_nocheck(void)
{ }

static __always_inline int test_and_stall_inband_nocheck(void)
{
	return irqs_disabled();
}

#endif /* !CONFIG_IRQ_PIPELINE */

#endif	/* !_LINUX_IRQSTAGE_H */
