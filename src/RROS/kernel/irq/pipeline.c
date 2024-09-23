/*
 * SPDX-License-Identifier: GPL-2.0
 *
 * Copyright (C) 2016 Philippe Gerum  <rpm@xenomai.org>.
 */
#include <linux/kernel.h>
#include <linux/sched.h>
#include <linux/interrupt.h>
#include <linux/irq.h>
#include <linux/irqdomain.h>
#include <linux/irq_pipeline.h>
#include <linux/irq_work.h>
#include <linux/jhash.h>
#include <linux/debug_locks.h>
#include <linux/dovetail.h>
#include <dovetail/irq.h>
#include <trace/events/irq.h>
#include "internals.h"

#ifdef CONFIG_DEBUG_IRQ_PIPELINE
#define trace_on_debug
#else
#define trace_on_debug  notrace
#endif

struct irq_stage inband_stage = {
	.name = "Linux",
};
EXPORT_SYMBOL_GPL(inband_stage);

struct irq_stage oob_stage;
EXPORT_SYMBOL_GPL(oob_stage);

struct irq_domain *synthetic_irq_domain;
EXPORT_SYMBOL_GPL(synthetic_irq_domain);

bool irq_pipeline_oopsing;
EXPORT_SYMBOL_GPL(irq_pipeline_oopsing);

bool irq_pipeline_active;
EXPORT_SYMBOL_GPL(irq_pipeline_active);

#define IRQ_L1_MAPSZ	BITS_PER_LONG
#define IRQ_L2_MAPSZ	(BITS_PER_LONG * BITS_PER_LONG)
#define IRQ_FLAT_MAPSZ	DIV_ROUND_UP(IRQ_BITMAP_BITS, BITS_PER_LONG)

#if IRQ_FLAT_MAPSZ > IRQ_L2_MAPSZ
#define __IRQ_STAGE_MAP_LEVELS	4	/* up to 4/16M vectors */
#elif IRQ_FLAT_MAPSZ > IRQ_L1_MAPSZ
#define __IRQ_STAGE_MAP_LEVELS	3	/* up to 64/256M vectors */
#else
#define __IRQ_STAGE_MAP_LEVELS	2	/* up to 1024/4096 vectors */
#endif

struct irq_event_map {
#if __IRQ_STAGE_MAP_LEVELS >= 3
	unsigned long index_1[IRQ_L1_MAPSZ];
#if __IRQ_STAGE_MAP_LEVELS >= 4
	unsigned long index_2[IRQ_L2_MAPSZ];
#endif
#endif
	unsigned long flat[IRQ_FLAT_MAPSZ];
};

#ifdef CONFIG_SMP

static struct irq_event_map bootup_irq_map __initdata;

static DEFINE_PER_CPU(struct irq_event_map, irq_map_array[2]);

DEFINE_PER_CPU(struct irq_pipeline_data, irq_pipeline) = {
	.stages = {
		[0] = {
			.log = {
				.map = &bootup_irq_map,
			},
			.stage = &inband_stage,
		},
	},
};

#else /* !CONFIG_SMP */

static struct irq_event_map inband_irq_map;

static struct irq_event_map oob_irq_map;

DEFINE_PER_CPU(struct irq_pipeline_data, irq_pipeline) = {
	.stages = {
		[0] = {
			.log = {
				.map = &inband_irq_map,
			},
			.stage = &inband_stage,
		},
		[1] = {
			.log = {
				.map = &oob_irq_map,
			},
		},
	},
};

#endif /* !CONFIG_SMP */

EXPORT_PER_CPU_SYMBOL(irq_pipeline);

static void sirq_noop(struct irq_data *data) { }

/* Virtual interrupt controller for synthetic IRQs. */
static struct irq_chip sirq_chip = {
	.name		= "SIRQC",
	.irq_enable	= sirq_noop,
	.irq_disable	= sirq_noop,
	.flags		= IRQCHIP_PIPELINE_SAFE | IRQCHIP_SKIP_SET_WAKE,
};

static int sirq_map(struct irq_domain *d, unsigned int irq,
		    irq_hw_number_t hwirq)
{
	irq_set_percpu_devid(irq);
	irq_set_chip_and_handler(irq, &sirq_chip, handle_synthetic_irq);

	return 0;
}

static struct irq_domain_ops sirq_domain_ops = {
	.map	= sirq_map,
};

#ifdef CONFIG_SPARSE_IRQ
/*
 * The performances of the radix tree in sparse mode are really ugly
 * under mm stress on some hw, use a local descriptor cache to ease
 * the pain.
 */
#define DESC_CACHE_SZ  128

static struct irq_desc *desc_cache[DESC_CACHE_SZ] __cacheline_aligned;

static inline u32 hash_irq(unsigned int irq)
{
	return jhash(&irq, sizeof(irq), irq) % DESC_CACHE_SZ;
}

static __always_inline
struct irq_desc *irq_to_cached_desc(unsigned int irq)
{
	int hval = hash_irq(irq);
	struct irq_desc *desc = desc_cache[hval];

	if (unlikely(desc == NULL || irq_desc_get_irq(desc) != irq)) {
		desc = irq_to_desc(irq);
		desc_cache[hval] = desc;
	}

	return desc;
}

void uncache_irq_desc(unsigned int irq)
{
	int hval = hash_irq(irq);

	desc_cache[hval] = NULL;
}

#else

static struct irq_desc *irq_to_cached_desc(unsigned int irq)
{
	return irq_to_desc(irq);
}

#endif

/**
 *	handle_synthetic_irq -  synthetic irq handler
 *	@desc:	the interrupt description structure for this irq
 *
 *	Handles synthetic interrupts flowing down the IRQ pipeline
 *	with per-CPU semantics.
 *
 *      CAUTION: synthetic IRQs may be used to map hardware-generated
 *      events (e.g. IPIs or traps), we must start handling them as
 *      common interrupts.
 */
void handle_synthetic_irq(struct irq_desc *desc)
{
	unsigned int irq = irq_desc_get_irq(desc);
	struct irqaction *action;
	irqreturn_t ret;
	void *dev_id;

	if (on_pipeline_entry()) {
		handle_oob_irq(desc);
		return;
	}

	action = desc->action;
	if (action == NULL) {
		if (printk_ratelimit())
			printk(KERN_WARNING
			       "CPU%d: WARNING: synthetic IRQ%d has no action.\n",
			       smp_processor_id(), irq);
		return;
	}

	__kstat_incr_irqs_this_cpu(desc);
	trace_irq_handler_entry(irq, action);
	dev_id = raw_cpu_ptr(action->percpu_dev_id);
	ret = action->handler(irq, dev_id);
	trace_irq_handler_exit(irq, action, ret);
}

void sync_irq_stage(struct irq_stage *top)
{
	struct irq_stage_data *p;
	struct irq_stage *stage;

	/* We must enter over the inband stage with hardirqs off. */
	if (irq_pipeline_debug()) {
		WARN_ON_ONCE(!hard_irqs_disabled());
		WARN_ON_ONCE(current_irq_stage != &inband_stage);
	}

	stage = top;

	for (;;) {
		if (stage == &inband_stage) {
			if (test_inband_stall())
				break;
		} else {
			if (test_oob_stall())
				break;
		}

		p = this_staged(stage);
		if (stage_irqs_pending(p)) {
			if (stage == &inband_stage)
				sync_current_irq_stage();
			else {
				/* Switch to oob before synchronizing. */
				switch_oob(p);
				sync_current_irq_stage();
				/* Then back to the inband stage. */
				switch_inband(this_inband_staged());
			}
		}

		if (stage == &inband_stage)
			break;

		stage = &inband_stage;
	}
}

void synchronize_pipeline(void) /* hardirqs off */
{
	struct irq_stage *top = &oob_stage;
	int stalled = test_oob_stall();

	if (unlikely(!oob_stage_present())) {
		top = &inband_stage;
		stalled = test_inband_stall();
	}

	if (current_irq_stage != top)
		sync_irq_stage(top);
	else if (!stalled)
		sync_current_irq_stage();
}

static void __inband_irq_enable(void)
{
	struct irq_stage_data *p;
	unsigned long flags;

	check_inband_stage();

	flags = hard_local_irq_save();

	unstall_inband_nocheck();

	p = this_inband_staged();
	if (unlikely(stage_irqs_pending(p) && !in_pipeline())) {
		sync_current_irq_stage();
		hard_local_irq_restore(flags);
		preempt_check_resched();
	} else {
		hard_local_irq_restore(flags);
	}
}

/**
 *	inband_irq_enable - enable interrupts for the inband stage
 *
 *	Enable interrupts for the inband stage, allowing interrupts to
 *	preempt the in-band code. If in-band IRQs are pending for the
 *	inband stage in the per-CPU log at the time of this call, they
 *	are played back.
 *
 *      The caller is expected to tell the tracer about the change, by
 *      calling trace_hardirqs_on().
 */
notrace void inband_irq_enable(void)
{
	/*
	 * We are NOT supposed to enter this code with hard IRQs off.
	 * If we do, then the caller might be wrongly assuming that
	 * invoking local_irq_enable() implies enabling hard
	 * interrupts like the legacy I-pipe did, which is not the
	 * case anymore. Relax this requirement when oopsing, since
	 * the kernel may be in a weird state.
	 */
	WARN_ON_ONCE(irq_pipeline_debug() && hard_irqs_disabled());
	__inband_irq_enable();
}
EXPORT_SYMBOL(inband_irq_enable);

/**
 *	inband_irq_disable - disable interrupts for the inband stage
 *
 *	Disable interrupts for the inband stage, disabling in-band
 *	interrupts. Out-of-band interrupts can still be taken and
 *	delivered to their respective handlers though.
 */
notrace void inband_irq_disable(void)
{
	check_inband_stage();
	stall_inband_nocheck();
}
EXPORT_SYMBOL(inband_irq_disable);

/**
 *	inband_irqs_disabled - test the virtual interrupt state
 *
 *	Returns non-zero if interrupts are currently disabled for the
 *	inband stage, zero otherwise.
 *
 *	May be used from the oob stage too (e.g. for tracing
 *	purpose).
 */
noinstr int inband_irqs_disabled(void)
{
	return test_inband_stall();
}
EXPORT_SYMBOL(inband_irqs_disabled);

/**
 *	inband_irq_save - test and disable (virtual) interrupts
 *
 *	Save the virtual interrupt state then disables interrupts for
 *	the inband stage.
 *
 *      Returns the original interrupt state.
 */
trace_on_debug unsigned long inband_irq_save(void)
{
	check_inband_stage();
	return test_and_stall_inband_nocheck();
}
EXPORT_SYMBOL(inband_irq_save);

/**
 *	inband_irq_restore - restore the (virtual) interrupt state
 *      @x:	Interrupt state to restore
 *
 *	Restore the virtual interrupt state from x. If the inband
 *	stage is unstalled as a consequence of this operation, any
 *	interrupt pending for the inband stage in the per-CPU log is
 *	played back.
 */
trace_on_debug void inband_irq_restore(unsigned long flags)
{
	if (flags)
		inband_irq_disable();
	else
		__inband_irq_enable();
}
EXPORT_SYMBOL(inband_irq_restore);

/**
 *	oob_irq_enable - enable interrupts in the CPU
 *
 *	Enable interrupts in the CPU, allowing out-of-band interrupts
 *	to preempt any code. If out-of-band IRQs are pending in the
 *	per-CPU log for the oob stage at the time of this call, they
 *	are played back.
 */
trace_on_debug void oob_irq_enable(void)
{
	struct irq_stage_data *p;

	hard_local_irq_disable();

	unstall_oob();

	p = this_oob_staged();
	if (unlikely(stage_irqs_pending(p)))
		synchronize_pipeline();

	hard_local_irq_enable();
}
EXPORT_SYMBOL(oob_irq_enable);

/**
 *	oob_irq_restore - restore the hardware interrupt state
 *      @x:	Interrupt state to restore
 *
 *	Restore the harware interrupt state from x. If the oob stage
 *	is unstalled as a consequence of this operation, any interrupt
 *	pending for the oob stage in the per-CPU log is played back
 *	prior to turning IRQs on.
 *
 *      NOTE: Stalling the oob stage must always be paired with
 *      disabling hard irqs and conversely when calling
 *      oob_irq_restore(), otherwise the latter would badly misbehave
 *      in unbalanced conditions.
 */
trace_on_debug void __oob_irq_restore(unsigned long flags) /* hw interrupt off */
{
	struct irq_stage_data *p = this_oob_staged();

	check_hard_irqs_disabled();

	if (!flags) {
		unstall_oob();
		if (unlikely(stage_irqs_pending(p)))
			synchronize_pipeline();
		hard_local_irq_enable();
	}
}
EXPORT_SYMBOL(__oob_irq_restore);

/**
 *	stage_disabled - test the interrupt state of the current stage
 *
 *	Returns non-zero if interrupts are currently disabled for the
 *	current interrupt stage, zero otherwise.
 *      In other words, returns non-zero either if:
 *      - interrupts are disabled for the OOB context (i.e. hard disabled),
 *      - the inband stage is current and inband interrupts are disabled.
 */
noinstr bool stage_disabled(void)
{
	bool ret = true;

	if (!hard_irqs_disabled()) {
		ret = false;
		if (running_inband())
			ret = test_inband_stall();
	}

	return ret;
}
EXPORT_SYMBOL_GPL(stage_disabled);

/**
 *	test_and_lock_stage - test and disable interrupts for the current stage
 *	@irqsoff:	Pointer to boolean denoting stage_disabled()
 *                      on entry
 *
 *	Fully disables interrupts for the current stage. When the
 *	inband stage is current, the stall bit is raised and hardware
 *	IRQs are masked as well. Only the latter operation is
 *	performed when the oob stage is current.
 *
 *      Returns the combined interrupt state on entry including the
 *      real/hardware (in CPU) and virtual (inband stage) states. For
 *      this reason, [test_and_]lock_stage() must be paired with
 *      unlock_stage() exclusively. The combined irq state returned by
 *      the former may NOT be passed to hard_local_irq_restore().
 *
 *      The interrupt state of the current stage in the return value
 *      (i.e. stall bit for the inband stage, hardware interrupt bit
 *      for the oob stage) must be testable using
 *      arch_irqs_disabled_flags().
 *
 *	Notice that test_and_lock_stage(), unlock_stage() are raw
 *	level ops, which substitute to raw_local_irq_save(),
 *	raw_local_irq_restore() in lockdep code. Therefore, changes to
 *	the in-band stall bit must not be propagated to the tracing
 *	core (i.e. no trace_hardirqs_*() annotations).
 */
noinstr unsigned long test_and_lock_stage(int *irqsoff)
{
	unsigned long flags;
	int stalled, dummy;

	if (irqsoff == NULL)
		irqsoff = &dummy;

	/*
	 * Combine the hard irq flag and the stall bit into a single
	 * state word. We need to fill in the stall bit only if the
	 * inband stage is current, otherwise it is not relevant.
	 */
	flags = hard_local_irq_save();
	*irqsoff = hard_irqs_disabled_flags(flags);
	if (running_inband()) {
		stalled = test_and_stall_inband_nocheck();
		flags = irqs_merge_flags(flags, stalled);
		if (stalled)
			*irqsoff = 1;
	}

	/*
	 * CAUTION: don't ever pass this verbatim to
	 * hard_local_irq_restore(). Only unlock_stage() knows how to
	 * decode and use a combined state word.
	 */
	return flags;
}
EXPORT_SYMBOL_GPL(test_and_lock_stage);

/**
 *	unlock_stage - restore interrupts for the current stage
 *	@flags: 	Combined interrupt state to restore as received from
 *              	test_and_lock_stage()
 *
 *	Restore the virtual interrupt state if the inband stage is
 *      current, and the hardware interrupt state unconditionally.
 *      The per-CPU log is not played for any stage.
 */
noinstr void unlock_stage(unsigned long irqstate)
{
	unsigned long flags = irqstate;
	int stalled;

	WARN_ON_ONCE(irq_pipeline_debug_locking() && !hard_irqs_disabled());

	if (running_inband()) {
		flags = irqs_split_flags(irqstate, &stalled);
		if (!stalled)
			unstall_inband_nocheck();
	}

	/*
	 * The hardware interrupt bit is the only flag which may be
	 * present in the combined state at this point, all other
	 * status bits have been cleared by irqs_merge_flags(), so
	 * don't ever try to reload the hardware status register with
	 * such value directly!
	 */
	if (!hard_irqs_disabled_flags(flags))
		hard_local_irq_enable();
}
EXPORT_SYMBOL_GPL(unlock_stage);

/**
 * sync_inband_irqs	- Synchronize the inband log
 *
 * Play any deferred interrupt which might have been logged for the
 * in-band stage while running with hard irqs on but stalled.
 *
 * Called from the unstalled in-band stage. Returns with hard irqs off.
 */
void sync_inband_irqs(void)
{
	struct irq_stage_data *p;

	check_inband_stage();
	WARN_ON_ONCE(irq_pipeline_debug() && irqs_disabled());

	if (!hard_irqs_disabled())
		hard_local_irq_disable();

	p = this_inband_staged();
	if (unlikely(stage_irqs_pending(p))) {
		/* Do not pile up preemption frames. */
		preempt_disable_notrace();
		sync_current_irq_stage();
		preempt_enable_no_resched_notrace();
	}
}

static inline bool irq_post_check(struct irq_stage *stage, unsigned int irq)
{
	if (irq_pipeline_debug()) {
		if (WARN_ONCE(!hard_irqs_disabled(),
				"hard irqs on posting IRQ%u to %s\n",
				irq, stage->name))
			return true;
		if (WARN_ONCE(irq >= IRQ_BITMAP_BITS,
				"cannot post invalid IRQ%u to %s\n",
				irq, stage->name))
			return true;
	}

	return false;
}

#if __IRQ_STAGE_MAP_LEVELS == 4

/* Must be called hard irqs off. */
void irq_post_stage(struct irq_stage *stage, unsigned int irq)
{
	struct irq_stage_data *p = this_staged(stage);
	int l0b, l1b, l2b;

	if (irq_post_check(stage, irq))
		return;

	l0b = irq / (BITS_PER_LONG * BITS_PER_LONG * BITS_PER_LONG);
	l1b = irq / (BITS_PER_LONG * BITS_PER_LONG);
	l2b = irq / BITS_PER_LONG;

	__set_bit(irq, p->log.map->flat);
	__set_bit(l2b, p->log.map->index_2);
	__set_bit(l1b, p->log.map->index_1);
	__set_bit(l0b, &p->log.index_0);
}
EXPORT_SYMBOL_GPL(irq_post_stage);

#define ltob_1(__n)  ((__n) * BITS_PER_LONG)
#define ltob_2(__n)  (ltob_1(__n) * BITS_PER_LONG)
#define ltob_3(__n)  (ltob_2(__n) * BITS_PER_LONG)

static inline int pull_next_irq(struct irq_stage_data *p)
{
	unsigned long l0m, l1m, l2m, l3m;
	int l0b, l1b, l2b, l3b;
	unsigned int irq;

	l0m = p->log.index_0;
	if (l0m == 0)
		return -1;
	l0b = __ffs(l0m);
	irq = ltob_3(l0b);

	l1m = p->log.map->index_1[l0b];
	if (unlikely(l1m == 0)) {
		WARN_ON_ONCE(1);
		return -1;
	}
	l1b = __ffs(l1m);
	irq += ltob_2(l1b);

	l2m = p->log.map->index_2[ltob_1(l0b) + l1b];
	if (unlikely(l2m == 0)) {
		WARN_ON_ONCE(1);
		return -1;
	}
	l2b = __ffs(l2m);
	irq += ltob_1(l2b);

	l3m = p->log.map->flat[ltob_2(l0b) + ltob_1(l1b) + l2b];
	if (unlikely(l3m == 0))
		return -1;
	l3b = __ffs(l3m);
	irq += l3b;

	__clear_bit(irq, p->log.map->flat);
	if (p->log.map->flat[irq / BITS_PER_LONG] == 0) {
		__clear_bit(l2b, &p->log.map->index_2[ltob_1(l0b) + l1b]);
		if (p->log.map->index_2[ltob_1(l0b) + l1b] == 0) {
			__clear_bit(l1b, &p->log.map->index_1[l0b]);
			if (p->log.map->index_1[l0b] == 0)
				__clear_bit(l0b, &p->log.index_0);
		}
	}

	return irq;
}

#elif __IRQ_STAGE_MAP_LEVELS == 3

/* Must be called hard irqs off. */
void irq_post_stage(struct irq_stage *stage, unsigned int irq)
{
	struct irq_stage_data *p = this_staged(stage);
	int l0b, l1b;

	if (irq_post_check(stage, irq))
		return;

	l0b = irq / (BITS_PER_LONG * BITS_PER_LONG);
	l1b = irq / BITS_PER_LONG;

	__set_bit(irq, p->log.map->flat);
	__set_bit(l1b, p->log.map->index_1);
	__set_bit(l0b, &p->log.index_0);
}
EXPORT_SYMBOL_GPL(irq_post_stage);

static inline int pull_next_irq(struct irq_stage_data *p)
{
	unsigned long l0m, l1m, l2m;
	int l0b, l1b, l2b, irq;

	l0m = p->log.index_0;
	if (unlikely(l0m == 0))
		return -1;

	l0b = __ffs(l0m);
	l1m = p->log.map->index_1[l0b];
	if (l1m == 0)
		return -1;

	l1b = __ffs(l1m) + l0b * BITS_PER_LONG;
	l2m = p->log.map->flat[l1b];
	if (unlikely(l2m == 0)) {
		WARN_ON_ONCE(1);
		return -1;
	}

	l2b = __ffs(l2m);
	irq = l1b * BITS_PER_LONG + l2b;

	__clear_bit(irq, p->log.map->flat);
	if (p->log.map->flat[l1b] == 0) {
		__clear_bit(l1b, p->log.map->index_1);
		if (p->log.map->index_1[l0b] == 0)
			__clear_bit(l0b, &p->log.index_0);
	}

	return irq;
}

#else /* __IRQ_STAGE_MAP_LEVELS == 2 */

/* Must be called hard irqs off. */
void irq_post_stage(struct irq_stage *stage, unsigned int irq)
{
	struct irq_stage_data *p = this_staged(stage);
	int l0b = irq / BITS_PER_LONG;

	if (irq_post_check(stage, irq))
		return;

	__set_bit(irq, p->log.map->flat);
	__set_bit(l0b, &p->log.index_0);
}
EXPORT_SYMBOL_GPL(irq_post_stage);

static inline int pull_next_irq(struct irq_stage_data *p)
{
	unsigned long l0m, l1m;
	int l0b, l1b;

	l0m = p->log.index_0;
	if (l0m == 0)
		return -1;

	l0b = __ffs(l0m);
	l1m = p->log.map->flat[l0b];
	if (unlikely(l1m == 0)) {
		WARN_ON_ONCE(1);
		return -1;
	}

	l1b = __ffs(l1m);
	__clear_bit(l1b, &p->log.map->flat[l0b]);
	if (p->log.map->flat[l0b] == 0)
		__clear_bit(l0b, &p->log.index_0);

	return l0b * BITS_PER_LONG + l1b;
}

#endif  /* __IRQ_STAGE_MAP_LEVELS == 2 */

/**
 *	hard_preempt_disable - Disable preemption the hard way
 *
 *      Disable hardware interrupts in the CPU, and disable preemption
 *      if currently running in-band code on the inband stage.
 *
 *      Return the hardware interrupt state.
 */
unsigned long hard_preempt_disable(void)
{
	unsigned long flags = hard_local_irq_save();

	if (running_inband())
		preempt_disable();

	return flags;
}
EXPORT_SYMBOL_GPL(hard_preempt_disable);

/**
 *	hard_preempt_enable - Enable preemption the hard way
 *
 *      Enable preemption if currently running in-band code on the
 *      inband stage, restoring the hardware interrupt state in the CPU.
 *      The per-CPU log is not played for the oob stage.
 */
void hard_preempt_enable(unsigned long flags)
{
	if (running_inband()) {
		preempt_enable_no_resched();
		hard_local_irq_restore(flags);
		if (!hard_irqs_disabled_flags(flags))
			preempt_check_resched();
	} else
		hard_local_irq_restore(flags);
}
EXPORT_SYMBOL_GPL(hard_preempt_enable);

static void handle_unexpected_irq(struct irq_desc *desc, irqreturn_t ret)
{
	unsigned int irq = irq_desc_get_irq(desc);
	struct irqaction *action;

	/*
	 * Since IRQ_HANDLED was not received from any handler, we may
	 * have a problem dealing with an OOB interrupt. The error
	 * detection logic is as follows:
	 *
	 * - check and complain about any bogus return value from a
	 * out-of-band IRQ handler: we only allow IRQ_HANDLED and
	 * IRQ_NONE from those routines.
	 *
	 * - filter out spurious IRQs which may have been due to bus
	 * asynchronicity, those tend to happen infrequently and
	 * should not cause us to pull the break (see
	 * note_interrupt()).
	 *
	 * - otherwise, stop pipelining the IRQ line after a thousand
	 * consecutive unhandled events.
	 *
	 * NOTE: we should already be holding desc->lock for non
	 * per-cpu IRQs, since we should only get there from the
	 * pipeline entry context.
	 */

	WARN_ON_ONCE(irq_pipeline_debug() &&
		     !irq_settings_is_per_cpu(desc) &&
		     !raw_spin_is_locked(&desc->lock));

	if (ret != IRQ_NONE) {
		printk(KERN_ERR "out-of-band irq event %d: bogus return value %x\n",
		       irq, ret);
		for_each_action_of_desc(desc, action)
			printk(KERN_ERR "[<%p>] %pf",
			       action->handler, action->handler);
		printk(KERN_CONT "\n");
		return;
	}

	if (time_after(jiffies, desc->last_unhandled + HZ/10))
		desc->irqs_unhandled = 0;
	else
		desc->irqs_unhandled++;

	desc->last_unhandled = jiffies;

	if (unlikely(desc->irqs_unhandled > 1000)) {
		printk(KERN_ERR "out-of-band irq %d: stuck or unexpected\n", irq);
		irq_settings_clr_oob(desc);
		desc->istate |= IRQS_SPURIOUS_DISABLED;
		irq_disable(desc);
	}
}

static inline void incr_irq_kstat(struct irq_desc *desc)
{
	if (irq_settings_is_per_cpu_devid(desc))
		__kstat_incr_irqs_this_cpu(desc);
	else
		kstat_incr_irqs_this_cpu(desc);
}

/*
 * do_oob_irq() - Handles interrupts over the oob stage. Hard irqs
 * off.
 */
static void do_oob_irq(struct irq_desc *desc)
{
	bool percpu_devid = irq_settings_is_per_cpu_devid(desc);
	unsigned int irq = irq_desc_get_irq(desc);
	irqreturn_t ret = IRQ_NONE, res;
	struct irqaction *action;
	void *dev_id;

	action = desc->action;
	if (unlikely(action == NULL))
		goto done;

	if (percpu_devid) {
		trace_irq_handler_entry(irq, action);
		dev_id = raw_cpu_ptr(action->percpu_dev_id);
		ret = action->handler(irq, dev_id);
		trace_irq_handler_exit(irq, action, ret);
	} else {
		desc->istate &= ~IRQS_PENDING;
		if (unlikely(irqd_irq_disabled(&desc->irq_data)))
			return;
		irqd_set(&desc->irq_data, IRQD_IRQ_INPROGRESS);
		raw_spin_unlock(&desc->lock);
		for_each_action_of_desc(desc, action) {
			trace_irq_handler_entry(irq, action);
			dev_id = action->dev_id;
			res = action->handler(irq, dev_id);
			trace_irq_handler_exit(irq, action, res);
			ret |= res;
		}
		raw_spin_lock(&desc->lock);
		irqd_clear(&desc->irq_data, IRQD_IRQ_INPROGRESS);
	}
done:
	incr_irq_kstat(desc);

	if (likely(ret & IRQ_HANDLED)) {
		desc->irqs_unhandled = 0;
		return;
	}

	handle_unexpected_irq(desc, ret);
}

/*
 * Over the inband stage, IRQs must be dispatched by the arch-specific
 * arch_do_IRQ_pipelined() routine.
 *
 * Entered with hardirqs on, inband stalled.
 */
static inline
void do_inband_irq(struct irq_desc *desc)
{
	arch_do_IRQ_pipelined(desc);
	WARN_ON_ONCE(irq_pipeline_debug() && !irqs_disabled());
}

static inline bool is_active_edge_event(struct irq_desc *desc)
{
	return (desc->istate & IRQS_PENDING) &&
		!irqd_irq_disabled(&desc->irq_data);
}

bool handle_oob_irq(struct irq_desc *desc) /* hardirqs off */
{
	struct irq_stage_data *oobd = this_oob_staged();
	unsigned int irq = irq_desc_get_irq(desc);
	int stalled;

	/*
	 * Flow handlers of chained interrupts have no business
	 * running here: they should decode the event, invoking
	 * generic_handle_irq() for each cascaded IRQ.
	 */
	if (WARN_ON_ONCE(irq_pipeline_debug() &&
			 irq_settings_is_chained(desc)))
		return false;

	/*
	 * If no oob stage is present, all interrupts must go to the
	 * inband stage through the interrupt log. Otherwise,
	 * out-of-band IRQs are immediately delivered to the oob
	 * stage, while in-band IRQs still go through the inband stage
	 * log.
	 *
	 * This routine returns a boolean status telling the caller
	 * whether an out-of-band interrupt was delivered.
	 */
	if (!oob_stage_present() || !irq_settings_is_oob(desc)) {
		irq_post_stage(&inband_stage, irq);
		return false;
	}

	if (WARN_ON_ONCE(irq_pipeline_debug() && running_inband()))
		return false;

	stalled = test_and_stall_oob();

	if (unlikely(desc->istate & IRQS_EDGE)) {
		do {
			if (is_active_edge_event(desc))  {
				if (irqd_irq_masked(&desc->irq_data))
					unmask_irq(desc);
			}
			do_oob_irq(desc);
		} while (is_active_edge_event(desc));
	} else {
		do_oob_irq(desc);
	}

	/*
	 * Cascaded interrupts enter handle_oob_irq() on the stalled
	 * out-of-band stage during the parent invocation. Make sure
	 * to restore the stall bit accordingly.
	 */
	if (likely(!stalled))
		unstall_oob();

	/*
	 * CPU migration and/or stage switching over the handler are
	 * NOT allowed. These should take place over
	 * irq_exit_pipeline().
	 */
	if (irq_pipeline_debug()) {
		/* No CPU migration allowed. */
		WARN_ON_ONCE(this_oob_staged() != oobd);
		/* No stage migration allowed. */
		WARN_ON_ONCE(current_irq_staged != oobd);
	}

	return true;
}

static inline
void copy_timer_regs(struct irq_desc *desc, struct pt_regs *regs)
{
	struct irq_pipeline_data *p;

	if (desc->action == NULL || !(desc->action->flags & __IRQF_TIMER))
		return;
	/*
	 * Given our deferred dispatching model for regular IRQs, we
	 * record the preempted context registers only for the latest
	 * timer interrupt, so that the regular tick handler charges
	 * CPU times properly. It is assumed that no other interrupt
	 * handler cares for such information.
	 */
	p = raw_cpu_ptr(&irq_pipeline);
	arch_save_timer_regs(&p->tick_regs, regs);
}

static __always_inline
struct irq_stage_data *switch_stage_on_irq(void)
{
	struct irq_stage_data *prevd = current_irq_staged, *nextd;

	if (oob_stage_present()) {
		nextd = this_oob_staged();
		if (prevd != nextd)
			switch_oob(nextd);
	}

	return prevd;
}

static __always_inline
void restore_stage_on_irq(struct irq_stage_data *prevd)
{
	/*
	 * CPU migration and/or stage switching over
	 * irq_exit_pipeline() are allowed.  Our exit logic is as
	 * follows:
	 *
	 *    ENTRY      EXIT      EPILOGUE
	 *
	 *    oob        oob       nop
	 *    inband     oob       switch inband
	 *    oob        inband    nop
	 *    inband     inband    nop
	 */
	if (prevd->stage == &inband_stage &&
		current_irq_staged == this_oob_staged())
		switch_inband(this_inband_staged());
}

/**
 *	generic_pipeline_irq_desc - Pass an IRQ to the pipeline
 *	@desc:	Descriptor of the IRQ to pass
 *	@regs:	Register file coming from the low-level handling code
 *
 *	Inject an IRQ into the pipeline from a CPU interrupt or trap
 *	context.  A flow handler runs next for this IRQ.
 *
 *      Hard irqs must be off on entry. Caller should have pushed the
 *      IRQ regs using set_irq_regs().
 */
void generic_pipeline_irq_desc(struct irq_desc *desc, struct pt_regs *regs)
{
	int irq = irq_desc_get_irq(desc);

	if (irq_pipeline_debug() && !hard_irqs_disabled()) {
		hard_local_irq_disable();
		pr_err("IRQ pipeline: interrupts enabled on entry (IRQ%u)\n", irq);
	}

	trace_irq_pipeline_entry(irq);
	copy_timer_regs(desc, regs);
	generic_handle_irq_desc(desc);
	trace_irq_pipeline_exit(irq);
}

void generic_pipeline_irq(unsigned int irq, struct pt_regs *regs)
{
	struct irq_desc *desc = irq_to_cached_desc(irq);
	struct pt_regs *old_regs;

	old_regs = set_irq_regs(regs);
	generic_pipeline_irq_desc(desc, regs);
	set_irq_regs(old_regs);
}

struct irq_stage_data *handle_irq_pipelined_prepare(struct pt_regs *regs)
{
	struct irq_stage_data *prevd;

	/*
	 * Running with the oob stage stalled implies hardirqs off.
	 * For this reason, if the oob stage is stalled when we
	 * receive an interrupt from the hardware, something is badly
	 * broken in our interrupt state. Try fixing up, but without
	 * great hopes.
	 */
	if (irq_pipeline_debug()) {
		if (test_oob_stall()) {
			pr_err("IRQ pipeline: out-of-band stage stalled on IRQ entry\n");
			unstall_oob();
		}
		WARN_ON(on_pipeline_entry());
	}

	/*
	 * Switch early on to the out-of-band stage if present,
	 * anticipating a companion kernel is going to handle the
	 * incoming event. If not, never mind, we will switch back
	 * in-band before synchronizing interrupts.
	 */
	prevd = switch_stage_on_irq();

	/* Tell the companion core about the entry. */
	irq_enter_pipeline();

	/*
	 * Invariant: IRQs may not pile up in the section covered by
	 * the PIPELINE_OFFSET marker, because:
	 *
	 * - out-of-band handlers called from handle_oob_irq() may NOT
	 * re-enable hard interrupts. Ever.
	 *
	 * - synchronizing the in-band log with hard interrupts
	 * enabled is done outside of this section.
	 */
	preempt_count_add(PIPELINE_OFFSET);

	/*
	 * From the standpoint of the in-band context when pipelining
	 * is in effect, an interrupt entry is unsafe in a similar way
	 * a NMI is, since it may preempt almost anywhere as IRQs are
	 * only virtually masked most of the time, including inside
	 * (virtually) interrupt-free sections. Declare a NMI entry so
	 * that the low handling code is allowed to enter RCU read
	 * sides (e.g. handle_domain_irq() needs this to resolve IRQ
	 * mappings).
	 */
	rcu_nmi_enter();

	return prevd;
}

int handle_irq_pipelined_finish(struct irq_stage_data *prevd,
				struct pt_regs *regs)
{
	/*
	 * Leave the (pseudo-)NMI entry for RCU before the out-of-band
	 * core might reschedule in irq_exit_pipeline(), and
	 * interrupts are hard enabled again on this CPU as a result
	 * of switching context.
	 */
	rcu_nmi_exit();

	/*
	 * Make sure to leave the pipeline entry context before
	 * allowing the companion core to reschedule, and eventually
	 * synchronizing interrupts.
	 */
	preempt_count_sub(PIPELINE_OFFSET);

	/* Allow the companion core to reschedule. */
	irq_exit_pipeline();

	/* Back to the preempted stage. */
	restore_stage_on_irq(prevd);

	/*
	 * We have to synchronize interrupts because some might have
	 * been logged while we were busy handling an out-of-band
	 * event coming from the hardware:
	 *
	 * - as a result of calling an out-of-band handler which in
	 * turn posted them.
	 *
	 * - because we posted them directly for scheduling the
	 * interrupt to happen from the in-band stage.
	 */
	synchronize_pipeline_on_irq();

#ifdef CONFIG_DOVETAIL
	/*
	 * Sending MAYDAY is in essence a rare case, so prefer test
	 * then maybe clear over test_and_clear.
	 */
	if (user_mode(regs) && test_thread_flag(TIF_MAYDAY))
		dovetail_call_mayday(regs);
#endif

	return running_inband() && !irqs_disabled();
}

int handle_irq_pipelined(struct pt_regs *regs)
{
	struct irq_stage_data *prevd;

	prevd = handle_irq_pipelined_prepare(regs);
	arch_handle_irq_pipelined(regs);
	return handle_irq_pipelined_finish(prevd, regs);
}

/**
 *	irq_inject_pipeline - Inject a software-generated IRQ into the
 *	pipeline @irq: IRQ to inject
 *
 *	Inject an IRQ into the pipeline by software as if such
 *	hardware event had happened on the current CPU.
 */
int irq_inject_pipeline(unsigned int irq)
{
	struct irq_stage_data *oobd, *prevd;
	struct irq_desc *desc;
	unsigned long flags;

	desc = irq_to_cached_desc(irq);
	if (desc == NULL)
		return -EINVAL;

	flags = hard_local_irq_save();

	/*
	 * Handle the case of an IRQ sent to a stalled oob stage here,
	 * which allows to trap the same condition in handle_oob_irq()
	 * in a debug check (see comment there).
	 */
	oobd = this_oob_staged();
	if (oob_stage_present() &&
		irq_settings_is_oob(desc) &&
		test_oob_stall()) {
		irq_post_stage(&oob_stage, irq);
	} else {
		prevd = switch_stage_on_irq();
		irq_enter_pipeline();
		handle_oob_irq(desc);
		irq_exit_pipeline();
		restore_stage_on_irq(prevd);
		synchronize_pipeline_on_irq();
	}

	hard_local_irq_restore(flags);

	return 0;

}
EXPORT_SYMBOL_GPL(irq_inject_pipeline);

/*
 * sync_current_irq_stage() -- Flush the pending IRQs for the current
 * stage (and processor). This routine flushes the interrupt log (see
 * "Optimistic interrupt protection" from D. Stodolsky et al. for more
 * on the deferred interrupt scheme). Every interrupt which has
 * occurred while the pipeline was stalled gets played.
 *
 * CAUTION: CPU migration may occur over this routine if running over
 * the inband stage.
 */
void sync_current_irq_stage(void) /* hard irqs off */
{
	struct irq_stage_data *p;
	struct irq_stage *stage;
	struct irq_desc *desc;
	int irq;

	WARN_ON_ONCE(irq_pipeline_debug() && on_pipeline_entry());
	check_hard_irqs_disabled();

	p = current_irq_staged;
respin:
	stage = p->stage;
	if (stage == &inband_stage) {
		/*
		 * Since we manipulate the stall bit directly, we have
		 * to open code the IRQ state tracing.
		 */
		stall_inband_nocheck();
		trace_hardirqs_off();
	} else {
		stall_oob();
	}

	for (;;) {
		irq = pull_next_irq(p);
		if (irq < 0)
			break;
		/*
		 * Make sure the compiler does not reorder wrongly, so
		 * that all updates to maps are done before the
		 * handler gets called.
		 */
		barrier();

		desc = irq_to_cached_desc(irq);

		if (stage == &inband_stage) {
			hard_local_irq_enable();
			do_inband_irq(desc);
			hard_local_irq_disable();
		} else {
			do_oob_irq(desc);
		}

		/*
		 * We might have switched from the oob stage to the
		 * in-band one on return from the handler, in which
		 * case we might also have migrated to a different CPU
		 * (the converse in-band -> oob switch is NOT allowed
		 * though). Reload the current per-cpu context
		 * pointer, so that we further pull pending interrupts
		 * from the proper in-band log.
		 */
		p = current_irq_staged;
		if (p->stage != stage) {
			if (WARN_ON_ONCE(irq_pipeline_debug() &&
					stage == &inband_stage))
				break;
			goto respin;
		}
	}

	if (stage == &inband_stage) {
		trace_hardirqs_on();
		unstall_inband_nocheck();
	} else {
		unstall_oob();
	}
}

#ifndef CONFIG_GENERIC_ENTRY

/*
 * These helpers are normally called from the kernel entry/exit code
 * in the asm section by architectures which do not use the generic
 * kernel entry code, in order to save the interrupt and lockdep
 * states for the in-band stage on entry, restoring them when leaving
 * the kernel.  The per-architecture arch_kentry_set/get_irqstate()
 * calls determine where this information should be kept while running
 * in kernel context, indexed on the current register frame.
 */

#define KENTRY_STALL_BIT      BIT(0) /* Tracks INBAND_STALL_BIT */
#define KENTRY_LOCKDEP_BIT    BIT(1) /* Tracks hardirqs_enabled */

asmlinkage __visible noinstr void kentry_enter_pipelined(struct pt_regs *regs)
{
	long irqstate = 0;

	WARN_ON(irq_pipeline_debug() && !hard_irqs_disabled());

	if (!running_inband())
		return;

	if (lockdep_read_irqs_state())
		irqstate |= KENTRY_LOCKDEP_BIT;

	if (irqs_disabled())
		irqstate |= KENTRY_STALL_BIT;
	else
		trace_hardirqs_off();

	arch_kentry_set_irqstate(regs, irqstate);
}

asmlinkage void __visible noinstr kentry_exit_pipelined(struct pt_regs *regs)
{
	long irqstate;

	WARN_ON(irq_pipeline_debug() && !hard_irqs_disabled());

	if (!running_inband())
		return;

	/*
	 * If the in-band stage of the kernel is current but the IRQ
	 * is not going to be delivered because the latter is stalled,
	 * keep the tracing logic unaware of the receipt, so that no
	 * false positive is triggered in lockdep (e.g. IN-HARDIRQ-W
	 * -> HARDIRQ-ON-W). In this case, we still have to restore
	 * the lockdep irq state independently, since it might not be
	 * in sync with the stall bit (e.g. raw_local_irq_disable/save
	 * do flip the stall bit, but are not tracked by lockdep).
	 */

	irqstate = arch_kentry_get_irqstate(regs);
	if (!(irqstate & KENTRY_STALL_BIT)) {
		stall_inband_nocheck();
		trace_hardirqs_on();
		unstall_inband_nocheck();
	} else {
		lockdep_write_irqs_state(!!(irqstate & KENTRY_LOCKDEP_BIT));
	}
}

#endif /* !CONFIG_GENERIC_ENTRY */

/**
 *      run_oob_call - escalate function call to the oob stage
 *      @fn:    address of routine
 *      @arg:   routine argument
 *
 *      Make the specified function run on the oob stage, switching
 *      the current stage accordingly if needed. The escalated call is
 *      allowed to perform a stage migration in the process.
 */
int notrace run_oob_call(int (*fn)(void *arg), void *arg)
{
	struct irq_stage_data *p, *old;
	struct irq_stage *oob;
	unsigned long flags;
	int ret, s;

	flags = hard_local_irq_save();

	/* Switch to the oob stage if not current. */
	p = this_oob_staged();
	oob = p->stage;
	old = current_irq_staged;
	if (old != p)
		switch_oob(p);

	s = test_and_stall_oob();
	barrier();
	ret = fn(arg);
	hard_local_irq_disable();
	if (!s)
		unstall_oob();

	/*
	 * The exit logic is as follows:
	 *
	 *    ON-ENTRY  AFTER-CALL  EPILOGUE
	 *
	 *    oob       oob         sync current stage if !stalled
	 *    inband    oob         switch to inband + sync all stages
	 *    oob       inband      sync all stages
	 *    inband    inband      sync all stages
	 *
	 * Each path which has stalled the oob stage while running on
	 * the inband stage at some point during the escalation
	 * process must synchronize all stages of the pipeline on
	 * exit. Otherwise, we may restrict the synchronization scope
	 * to the current stage when the whole sequence ran on the oob
	 * stage.
	 */
	p = this_oob_staged();
	if (likely(current_irq_staged == p)) {
		if (old->stage == oob) {
			if (!s && stage_irqs_pending(p))
				sync_current_irq_stage();
			goto out;
		}
		switch_inband(this_inband_staged());
	}

	sync_irq_stage(oob);
out:
	hard_local_irq_restore(flags);

	return ret;
}
EXPORT_SYMBOL_GPL(run_oob_call);

int enable_oob_stage(const char *name)
{
	struct irq_event_map *map;
	struct irq_stage_data *p;
	int cpu, ret;

	if (oob_stage_present())
		return -EBUSY;

	/* Set up the out-of-band interrupt stage on all CPUs. */

	for_each_possible_cpu(cpu) {
		p = &per_cpu(irq_pipeline.stages, cpu)[1];
		map = p->log.map; /* save/restore after memset(). */
		memset(p, 0, sizeof(*p));
		p->stage = &oob_stage;
		memset(map, 0, sizeof(struct irq_event_map));
		p->log.map = map;
#ifdef CONFIG_DEBUG_IRQ_PIPELINE
		p->cpu = cpu;
#endif
	}

	ret = arch_enable_oob_stage();
	if (ret)
		return ret;

	oob_stage.name = name;
	smp_wmb();
	oob_stage.index = 1;

	pr_info("IRQ pipeline: high-priority %s stage added.\n", name);

	return 0;
}
EXPORT_SYMBOL_GPL(enable_oob_stage);

void disable_oob_stage(void)
{
	const char *name = oob_stage.name;

	WARN_ON(!running_inband() || !oob_stage_present());

	oob_stage.index = 0;
	smp_wmb();

	pr_info("IRQ pipeline: %s stage removed.\n", name);
}
EXPORT_SYMBOL_GPL(disable_oob_stage);

void irq_pipeline_oops(void)
{
	irq_pipeline_oopsing = true;
	local_irq_disable_full();
}

/*
 * Used to save/restore the status bits of the inband stage across runs
 * of NMI-triggered code, so that we can restore the original pipeline
 * state before leaving NMI context.
 */
static DEFINE_PER_CPU(unsigned long, nmi_saved_stall_bits);

noinstr void irq_pipeline_nmi_enter(void)
{
	raw_cpu_write(nmi_saved_stall_bits, current->stall_bits);

}
EXPORT_SYMBOL(irq_pipeline_nmi_enter);

noinstr void irq_pipeline_nmi_exit(void)
{
	current->stall_bits = raw_cpu_read(nmi_saved_stall_bits);
}
EXPORT_SYMBOL(irq_pipeline_nmi_exit);

bool __weak irq_cpuidle_control(struct cpuidle_device *dev,
				struct cpuidle_state *state)
{
	/*
	 * Allow entering the idle state by default, matching the
	 * original behavior when CPU_IDLE is turned
	 * on. irq_cpuidle_control() may be overriden by an
	 * out-of-band code for determining whether the CPU may
	 * actually enter the idle state.
	 */
	return true;
}

/**
 *	irq_cpuidle_enter - Prepare for entering the next idle state
 *	@dev: CPUIDLE device
 *	@state: CPUIDLE state to be entered
 *
 *	Flush the in-band interrupt log before the caller idles, so
 *	that no event lingers before we actually wait for the next
 *	IRQ, in which case we ask the caller to abort the idling
 *	process altogether. The companion core is also given the
 *	opportunity to block the idling process by having
 *	irq_cpuidle_control() return @false.
 *
 *	Returns @true if caller may proceed with idling, @false
 *	otherwise. The in-band log is guaranteed empty on return, hard
 *	irqs left off so that no event might sneak in until the caller
 *	actually idles.
 */
bool irq_cpuidle_enter(struct cpuidle_device *dev,
		       struct cpuidle_state *state)
{
	WARN_ON_ONCE(irq_pipeline_debug() && !irqs_disabled());

	hard_local_irq_disable();

	if (stage_irqs_pending(this_inband_staged())) {
		unstall_inband_nocheck();
		synchronize_pipeline();
		stall_inband_nocheck();
		trace_hardirqs_off();
		return false;
	}

	return irq_cpuidle_control(dev, state);
}

static unsigned int inband_work_sirq;

static irqreturn_t inband_work_interrupt(int sirq, void *dev_id)
{
	irq_work_run();

	return IRQ_HANDLED;
}

static struct irqaction inband_work = {
	.handler = inband_work_interrupt,
	.name = "in-band work",
	.flags = IRQF_NO_THREAD,
};

void irq_local_work_raise(void)
{
	unsigned long flags;

	/*
	 * irq_work_queue() may be called from the in-band stage too
	 * in case we want to delay a work until the hard irqs are on
	 * again, so we may only sync the in-band log when unstalled,
	 * with hard irqs on.
	 */
	flags = hard_local_irq_save();
	irq_post_inband(inband_work_sirq);
	if (running_inband() &&
	    !hard_irqs_disabled_flags(flags) && !irqs_disabled())
		sync_current_irq_stage();
	hard_local_irq_restore(flags);
}

#ifdef CONFIG_DEBUG_IRQ_PIPELINE

#ifdef CONFIG_LOCKDEP
static inline bool lockdep_on_error(void)
{
	return !debug_locks;
}
#else
static inline bool lockdep_on_error(void)
{
	return false;
}
#endif

notrace void check_inband_stage(void)
{
	struct irq_stage *this_stage;
	unsigned long flags;

	flags = hard_local_irq_save();

	this_stage = current_irq_stage;
	if (likely(this_stage == &inband_stage && !test_oob_stall())) {
		hard_local_irq_restore(flags);
		return;
	}

	if (in_nmi() || irq_pipeline_oopsing || lockdep_on_error()) {
		hard_local_irq_restore(flags);
		return;
	}

	/*
	 * This will disable all further pipeline debug checks, since
	 * a wrecked interrupt state is likely to trigger many of
	 * them, ending up in a terrible mess. IOW, the current
	 * situation must be fixed prior to investigating any
	 * subsequent issue that might still exist.
	 */
	irq_pipeline_oopsing = true;

	hard_local_irq_restore(flags);

	if (this_stage != &inband_stage)
		pr_err("IRQ pipeline: some code running in oob context '%s'\n"
		       "              called an in-band only routine\n",
		       this_stage->name);
	else
		pr_err("IRQ pipeline: oob stage found stalled while modifying in-band\n"
		       "              interrupt state and/or running sleeping code\n");

	dump_stack();
}
EXPORT_SYMBOL(check_inband_stage);

void check_spinlock_context(void)
{
	WARN_ON_ONCE(in_pipeline() || running_oob());

}
EXPORT_SYMBOL(check_spinlock_context);

#endif /* CONFIG_DEBUG_IRQ_PIPELINE */

static inline void fixup_percpu_data(void)
{
#ifdef CONFIG_SMP
	struct irq_pipeline_data *p;
	int cpu;

	/*
	 * A temporary event log is used by the inband stage during the
	 * early boot up (bootup_irq_map), until the per-cpu areas
	 * have been set up.
	 *
	 * Obviously, this code must run over the boot CPU, before SMP
	 * operations start, with hard IRQs off so that nothing can
	 * change under our feet.
	 */
	WARN_ON(!hard_irqs_disabled());

	memcpy(&per_cpu(irq_map_array, 0)[0], &bootup_irq_map,
	       sizeof(struct irq_event_map));

	for_each_possible_cpu(cpu) {
		p = &per_cpu(irq_pipeline, cpu);
		p->stages[0].stage = &inband_stage;
		p->stages[0].log.map = &per_cpu(irq_map_array, cpu)[0];
		p->stages[1].log.map = &per_cpu(irq_map_array, cpu)[1];
#ifdef CONFIG_DEBUG_IRQ_PIPELINE
		p->stages[0].cpu = cpu;
		p->stages[1].cpu = cpu;
#endif
	}
#endif
}

void __init irq_pipeline_init_early(void)
{
	/*
	 * This is called early from start_kernel(), even before the
	 * actual number of IRQs is known. We are running on the boot
	 * CPU, hw interrupts are off, and secondary CPUs are still
	 * lost in space. Careful.
	 */
	fixup_percpu_data();
}

/**
 *	irq_pipeline_init - Main pipeline core inits
 *
 *	This is step #2 of the 3-step pipeline initialization, which
 *	should happen right after init_IRQ() has run. The internal
 *	service interrupts are created along with the synthetic IRQ
 *	domain, and the arch-specific init chores are performed too.
 *
 *	Interrupt pipelining should be fully functional when this
 *	routine returns.
 */
void __init irq_pipeline_init(void)
{
	WARN_ON(!hard_irqs_disabled());

	synthetic_irq_domain = irq_domain_add_nomap(NULL, ~0,
						    &sirq_domain_ops,
						    NULL);
	inband_work_sirq = irq_create_direct_mapping(synthetic_irq_domain);
	setup_percpu_irq(inband_work_sirq, &inband_work);

	/*
	 * We are running on the boot CPU, hw interrupts are off, and
	 * secondary CPUs are still lost in space. Now we may run
	 * arch-specific code for enabling the pipeline.
	 */
	arch_irq_pipeline_init();

	irq_pipeline_active = true;

	pr_info("IRQ pipeline enabled\n");
}

#ifndef CONFIG_SPARSE_IRQ
EXPORT_SYMBOL_GPL(irq_desc);
#endif
