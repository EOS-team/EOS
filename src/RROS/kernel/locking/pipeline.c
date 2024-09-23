/*
 * SPDX-License-Identifier: GPL-2.0
 *
 * Copyright (C) 2016 Philippe Gerum  <rpm@xenomai.org>.
 */
#include <linux/linkage.h>
#include <linux/preempt.h>
#include <linux/spinlock.h>
#include <linux/interrupt.h>
#include <linux/irq_pipeline.h>
#include <linux/kconfig.h>

/*
 * A hybrid spinlock behaves in different ways depending on the
 * current interrupt stage on entry.
 *
 * Such spinlock always leaves hard IRQs disabled once locked. In
 * addition, it stalls the in-band stage when protecting a critical
 * section there, disabling preemption like regular spinlocks do as
 * well. This combination preserves the regular locking logic when
 * called from the in-band stage, while fully disabling preemption by
 * other interrupt stages.
 *
 * When taken from the pipeline entry context, a hybrid lock behaves
 * like a hard spinlock, assuming that hard IRQs are already disabled.
 *
 * The irq descriptor lock (struct irq_desc) is a typical example of
 * such lock, which properly serializes accesses regardless of the
 * calling context.
 */
void __hybrid_spin_lock(struct raw_spinlock *rlock)
{
	struct hybrid_spinlock *lock;
	unsigned long __flags;

	if (running_inband())
		preempt_disable();

	__flags = hard_local_irq_save();
	hard_lock_acquire(rlock, 0, _RET_IP_);
	lock = container_of(rlock, struct hybrid_spinlock, rlock);
	lock->hwflags = __flags;
}
EXPORT_SYMBOL(__hybrid_spin_lock);

void __hybrid_spin_lock_nested(struct raw_spinlock *rlock, int subclass)
{
	struct hybrid_spinlock *lock;
	unsigned long __flags;

	if (running_inband())
		preempt_disable();

	__flags = hard_local_irq_save();
	hard_lock_acquire_nested(rlock, subclass, _RET_IP_);
	lock = container_of(rlock, struct hybrid_spinlock, rlock);
	lock->hwflags = __flags;
}
EXPORT_SYMBOL(__hybrid_spin_lock_nested);

void __hybrid_spin_unlock(struct raw_spinlock *rlock)
{
	struct hybrid_spinlock *lock;
	unsigned long __flags;

	/* Pick the flags before releasing the lock. */
	lock = container_of(rlock, struct hybrid_spinlock, rlock);
	__flags = lock->hwflags;
	hard_lock_release(rlock, _RET_IP_);
	hard_local_irq_restore(__flags);

	if (running_inband())
		preempt_enable();
}
EXPORT_SYMBOL(__hybrid_spin_unlock);

void __hybrid_spin_lock_irq(struct raw_spinlock *rlock)
{
	struct hybrid_spinlock *lock;
	unsigned long __flags;

	__flags = hard_local_irq_save();

	if (running_inband()) {
		stall_inband();
		trace_hardirqs_off();
		preempt_disable();
	}

	hard_lock_acquire(rlock, 0, _RET_IP_);
	lock = container_of(rlock, struct hybrid_spinlock, rlock);
	lock->hwflags = __flags;
}
EXPORT_SYMBOL(__hybrid_spin_lock_irq);

void __hybrid_spin_unlock_irq(struct raw_spinlock *rlock)
{
	struct hybrid_spinlock *lock;
	unsigned long __flags;

	/* Pick the flags before releasing the lock. */
	lock = container_of(rlock, struct hybrid_spinlock, rlock);
	__flags = lock->hwflags;
	hard_lock_release(rlock, _RET_IP_);

	if (running_inband()) {
		trace_hardirqs_on();
		unstall_inband_nocheck();
		hard_local_irq_restore(__flags);
		preempt_enable();
		return;
	}

	hard_local_irq_restore(__flags);
}
EXPORT_SYMBOL(__hybrid_spin_unlock_irq);

unsigned long __hybrid_spin_lock_irqsave(struct raw_spinlock *rlock)
{
	struct hybrid_spinlock *lock;
	unsigned long __flags, flags;

	__flags = flags = hard_local_irq_save();

	if (running_inband()) {
		flags = test_and_stall_inband();
		trace_hardirqs_off();
		preempt_disable();
	}

	hard_lock_acquire(rlock, 0, _RET_IP_);
	lock = container_of(rlock, struct hybrid_spinlock, rlock);
	lock->hwflags = __flags;

	return flags;
}
EXPORT_SYMBOL(__hybrid_spin_lock_irqsave);

void __hybrid_spin_unlock_irqrestore(struct raw_spinlock *rlock,
				      unsigned long flags)
{
	struct hybrid_spinlock *lock;
	unsigned long __flags;

	/* Pick the flags before releasing the lock. */
	lock = container_of(rlock, struct hybrid_spinlock, rlock);
	__flags = lock->hwflags;
	hard_lock_release(rlock, _RET_IP_);

	if (running_inband()) {
		if (!flags) {
			trace_hardirqs_on();
			unstall_inband_nocheck();
		}
		hard_local_irq_restore(__flags);
		preempt_enable();
		return;
	}

	hard_local_irq_restore(__flags);
}
EXPORT_SYMBOL(__hybrid_spin_unlock_irqrestore);

int __hybrid_spin_trylock(struct raw_spinlock *rlock)
{
	struct hybrid_spinlock *lock;
	unsigned long __flags;

	if (running_inband())
		preempt_disable();

	lock = container_of(rlock, struct hybrid_spinlock, rlock);
	__flags = hard_local_irq_save();

	hard_spin_trylock_prepare(rlock);
	if (do_raw_spin_trylock(rlock)) {
		lock->hwflags = __flags;
		hard_trylock_acquire(rlock, 1, _RET_IP_);
		return 1;
	}

	hard_spin_trylock_fail(rlock);
	hard_local_irq_restore(__flags);

	if (running_inband())
		preempt_enable();

	return 0;
}
EXPORT_SYMBOL(__hybrid_spin_trylock);

int __hybrid_spin_trylock_irqsave(struct raw_spinlock *rlock,
				   unsigned long *flags)
{
	struct hybrid_spinlock *lock;
	unsigned long __flags;
	bool inband;

	inband = running_inband();

	__flags = *flags = hard_local_irq_save();

	lock = container_of(rlock, struct hybrid_spinlock, rlock);
	if (inband) {
		*flags = test_and_stall_inband();
		trace_hardirqs_off();
		preempt_disable();
	}

	hard_spin_trylock_prepare(rlock);
	if (do_raw_spin_trylock(rlock)) {
		hard_trylock_acquire(rlock, 1, _RET_IP_);
		lock->hwflags = __flags;
		return 1;
	}

	hard_spin_trylock_fail(rlock);

	if (inband && !*flags) {
		trace_hardirqs_on();
		unstall_inband_nocheck();
	}

	hard_local_irq_restore(__flags);

	if (inband)
		preempt_enable();

	return 0;
}
EXPORT_SYMBOL(__hybrid_spin_trylock_irqsave);
