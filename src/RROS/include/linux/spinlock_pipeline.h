/*
 * SPDX-License-Identifier: GPL-2.0
 *
 * Copyright (C) 2016 Philippe Gerum  <rpm@xenomai.org>.
 */
#ifndef __LINUX_SPINLOCK_PIPELINE_H
#define __LINUX_SPINLOCK_PIPELINE_H

#ifndef __LINUX_SPINLOCK_H
# error "Please don't include this file directly. Use spinlock.h."
#endif

#include <dovetail/spinlock.h>

#define hard_spin_lock_irqsave(__rlock, __flags)		\
	do {							\
		(__flags) = __hard_spin_lock_irqsave(__rlock);	\
	} while (0)

#define hard_spin_trylock_irqsave(__rlock, __flags)			\
	({								\
		int __locked;						\
		(__flags) = __hard_spin_trylock_irqsave(__rlock, &__locked); \
		__locked;						\
	})

#define hybrid_spin_lock_init(__rlock)	hard_spin_lock_init(__rlock)

/*
 * CAUTION: We don't want the hand-coded irq-enable of
 * do_raw_spin_lock_flags(), hard locked sections assume that
 * interrupts are not re-enabled during lock-acquire.
 */
#define hard_lock_acquire(__rlock, __try, __ip)				\
	do {								\
		hard_spin_lock_prepare(__rlock);			\
		if (irq_pipeline_debug_locking()) {			\
			spin_acquire(&(__rlock)->dep_map, 0, __try, __ip); \
			LOCK_CONTENDED(__rlock, do_raw_spin_trylock, do_raw_spin_lock); \
		} else {						\
			do_raw_spin_lock(__rlock);			\
		}							\
	} while (0)

#define hard_lock_acquire_nested(__rlock, __subclass, __ip)		\
	do {								\
		hard_spin_lock_prepare(__rlock);			\
		if (irq_pipeline_debug_locking()) {			\
			spin_acquire(&(__rlock)->dep_map, __subclass, 0, __ip); \
			LOCK_CONTENDED(__rlock, do_raw_spin_trylock, do_raw_spin_lock); \
		} else {						\
			do_raw_spin_lock(__rlock);			\
		}							\
	} while (0)

#define hard_trylock_acquire(__rlock, __try, __ip)			\
	do {								\
		if (irq_pipeline_debug_locking())			\
			spin_acquire(&(__rlock)->dep_map, 0, __try, __ip); \
	} while (0)

#define hard_lock_release(__rlock, __ip)				\
	do {								\
		if (irq_pipeline_debug_locking())			\
			spin_release(&(__rlock)->dep_map, __ip);	\
		do_raw_spin_unlock(__rlock);				\
		hard_spin_unlock_finish(__rlock);			\
	} while (0)

#if defined(CONFIG_SMP) || defined(CONFIG_DEBUG_SPINLOCK)

#ifdef CONFIG_DEBUG_SPINLOCK
#define hard_spin_lock_init(__lock)				\
	do {							\
		static struct lock_class_key __key;		\
		__raw_spin_lock_init((raw_spinlock_t *)__lock, #__lock, &__key, LD_WAIT_SPIN); \
	} while (0)
#else
#define hard_spin_lock_init(__rlock)				\
	do { *(__rlock) = __HARD_SPIN_LOCK_UNLOCKED(__rlock); } while (0)
#endif

/*
 * XXX: no preempt_enable/disable when hard locking.
 */

static inline
void hard_spin_lock(struct raw_spinlock *rlock)
{
	hard_lock_acquire(rlock, 0, _THIS_IP_);
}

#ifdef CONFIG_DEBUG_LOCK_ALLOC
static inline
void hard_spin_lock_nested(struct raw_spinlock *rlock, int subclass)
{
	hard_lock_acquire_nested(rlock, subclass, _THIS_IP_);
}
#else
static inline
void hard_spin_lock_nested(struct raw_spinlock *rlock, int subclass)
{
	hard_spin_lock(rlock);
}
#endif

static inline
void hard_spin_unlock(struct raw_spinlock *rlock)
{
	hard_lock_release(rlock, _THIS_IP_);
}

static inline
void hard_spin_lock_irq(struct raw_spinlock *rlock)
{
	hard_local_irq_disable();
	hard_lock_acquire(rlock, 0, _THIS_IP_);
}

static inline
void hard_spin_unlock_irq(struct raw_spinlock *rlock)
{
	hard_lock_release(rlock, _THIS_IP_);
	hard_local_irq_enable();
}

static inline
void hard_spin_unlock_irqrestore(struct raw_spinlock *rlock,
				 unsigned long flags)
{
	hard_lock_release(rlock, _THIS_IP_);
	hard_local_irq_restore(flags);
}

static inline
unsigned long __hard_spin_lock_irqsave(struct raw_spinlock *rlock)
{
	unsigned long flags = hard_local_irq_save();

	hard_lock_acquire(rlock, 0, _THIS_IP_);

	return flags;
}

static inline
int hard_spin_trylock(struct raw_spinlock *rlock)
{
	hard_spin_trylock_prepare(rlock);

	if (do_raw_spin_trylock(rlock)) {
		hard_trylock_acquire(rlock, 1, _THIS_IP_);
		return 1;
	}

	hard_spin_trylock_fail(rlock);

	return 0;
}

static inline
unsigned long __hard_spin_trylock_irqsave(struct raw_spinlock *rlock,
					  int *locked)
{
	unsigned long flags = hard_local_irq_save();
	*locked = hard_spin_trylock(rlock);
	return *locked ? flags : ({ hard_local_irq_restore(flags); flags; });
}

static inline
int hard_spin_trylock_irq(struct raw_spinlock *rlock)
{
	hard_local_irq_disable();
	return hard_spin_trylock(rlock) ? : ({ hard_local_irq_enable(); 0; });
}

static inline
int hard_spin_is_locked(struct raw_spinlock *rlock)
{
	return arch_spin_is_locked(&rlock->raw_lock);
}

static inline
int hard_spin_is_contended(struct raw_spinlock *rlock)
{
#ifdef CONFIG_GENERIC_LOCKBREAK
	return rlock->break_lock;
#elif defined(arch_spin_is_contended)
	return arch_spin_is_contended(&rlock->raw_lock);
#else
	return 0;
#endif
}

#else  /* !SMP && !DEBUG_SPINLOCK */

#define hard_spin_lock_init(__rlock)	do { (void)(__rlock); } while (0)
#define hard_spin_lock(__rlock)		__HARD_LOCK(__rlock)
#define hard_spin_lock_nested(__rlock, __subclass)  \
	do { __HARD_LOCK(__rlock); (void)(__subclass); } while (0)
#define hard_spin_unlock(__rlock)	__HARD_UNLOCK(__rlock)
#define hard_spin_lock_irq(__rlock)	__HARD_LOCK_IRQ(__rlock)
#define hard_spin_unlock_irq(__rlock)	__HARD_UNLOCK_IRQ(__rlock)
#define hard_spin_unlock_irqrestore(__rlock, __flags)	\
	__HARD_UNLOCK_IRQRESTORE(__rlock, __flags)
#define __hard_spin_lock_irqsave(__rlock)		\
	({						\
		unsigned long __flags;			\
		__HARD_LOCK_IRQSAVE(__rlock, __flags);	\
		__flags;				\
	})
#define __hard_spin_trylock_irqsave(__rlock, __locked)	\
	({						\
		unsigned long __flags;			\
		__HARD_LOCK_IRQSAVE(__rlock, __flags);	\
		*(__locked) = 1;			\
		__flags;				\
	})
#define hard_spin_trylock(__rlock)	({ __HARD_LOCK(__rlock); 1; })
#define hard_spin_trylock_irq(__rlock)	({ __HARD_LOCK_IRQ(__rlock); 1; })
#define hard_spin_is_locked(__rlock)	((void)(__rlock), 0)
#define hard_spin_is_contended(__rlock)	((void)(__rlock), 0)
#endif	/* !SMP && !DEBUG_SPINLOCK */

/*
 * In the pipeline entry context, the regular preemption and root
 * stall logic do not apply since we may actually have preempted any
 * critical section of the kernel which is protected by regular
 * locking (spin or stall), or we may even have preempted the
 * out-of-band stage. Therefore, we just need to grab the raw spinlock
 * underlying a hybrid spinlock to exclude other CPUs.
 *
 * NOTE: When entering the pipeline, IRQs are already hard disabled.
 */

void __hybrid_spin_lock(struct raw_spinlock *rlock);
void __hybrid_spin_lock_nested(struct raw_spinlock *rlock, int subclass);

static inline void hybrid_spin_lock(struct raw_spinlock *rlock)
{
	if (in_pipeline())
		hard_lock_acquire(rlock, 0, _THIS_IP_);
	else
		__hybrid_spin_lock(rlock);
}

#ifdef CONFIG_DEBUG_LOCK_ALLOC
static inline
void hybrid_spin_lock_nested(struct raw_spinlock *rlock, int subclass)
{
	if (in_pipeline())
		hard_lock_acquire_nested(rlock, subclass, _THIS_IP_);
	else
		__hybrid_spin_lock_nested(rlock, subclass);
}
#else
static inline
void hybrid_spin_lock_nested(struct raw_spinlock *rlock, int subclass)
{
	hybrid_spin_lock(rlock);
}
#endif

void __hybrid_spin_unlock(struct raw_spinlock *rlock);

static inline void hybrid_spin_unlock(struct raw_spinlock *rlock)
{
	if (in_pipeline())
		hard_lock_release(rlock, _THIS_IP_);
	else
		__hybrid_spin_unlock(rlock);
}

void __hybrid_spin_lock_irq(struct raw_spinlock *rlock);

static inline void hybrid_spin_lock_irq(struct raw_spinlock *rlock)
{
	if (in_pipeline())
		hard_lock_acquire(rlock, 0, _THIS_IP_);
	else
		__hybrid_spin_lock_irq(rlock);
}

void __hybrid_spin_unlock_irq(struct raw_spinlock *rlock);

static inline void hybrid_spin_unlock_irq(struct raw_spinlock *rlock)
{
	if (in_pipeline())
		hard_lock_release(rlock, _THIS_IP_);
	else
		__hybrid_spin_unlock_irq(rlock);
}

unsigned long __hybrid_spin_lock_irqsave(struct raw_spinlock *rlock);

#define hybrid_spin_lock_irqsave(__rlock, __flags)			\
	do {								\
		if (in_pipeline()) {					\
			hard_lock_acquire(__rlock, 0, _THIS_IP_);	\
			(__flags) = hard_local_save_flags();		\
		} else							\
			(__flags) = __hybrid_spin_lock_irqsave(__rlock); \
	} while (0)

void __hybrid_spin_unlock_irqrestore(struct raw_spinlock *rlock,
				      unsigned long flags);

static inline void hybrid_spin_unlock_irqrestore(struct raw_spinlock *rlock,
						  unsigned long flags)
{

	if (in_pipeline())
		hard_lock_release(rlock, _THIS_IP_);
	else
		__hybrid_spin_unlock_irqrestore(rlock, flags);
}

int __hybrid_spin_trylock(struct raw_spinlock *rlock);

static inline int hybrid_spin_trylock(struct raw_spinlock *rlock)
{
	if (in_pipeline()) {
		hard_spin_trylock_prepare(rlock);
		if (do_raw_spin_trylock(rlock)) {
			hard_trylock_acquire(rlock, 1, _THIS_IP_);
			return 1;
		}
		hard_spin_trylock_fail(rlock);
		return 0;
	}

	return __hybrid_spin_trylock(rlock);
}

int __hybrid_spin_trylock_irqsave(struct raw_spinlock *rlock,
				   unsigned long *flags);

#define hybrid_spin_trylock_irqsave(__rlock, __flags)			\
	({								\
		int __ret = 1;						\
		if (in_pipeline()) {					\
			hard_spin_trylock_prepare(__rlock);		\
			if (do_raw_spin_trylock(__rlock)) {		\
				hard_trylock_acquire(__rlock, 1, _THIS_IP_); \
				(__flags) = hard_local_save_flags();	\
			} else {					\
				hard_spin_trylock_fail(__rlock);	\
				__ret = 0;				\
			}						\
		} else {						\
			__ret = __hybrid_spin_trylock_irqsave(__rlock, &(__flags)); \
		}							\
		__ret;							\
	})

static inline int hybrid_spin_trylock_irq(struct raw_spinlock *rlock)
{
	unsigned long flags;
	return hybrid_spin_trylock_irqsave(rlock, flags);
}

static inline
int hybrid_spin_is_locked(struct raw_spinlock *rlock)
{
	return hard_spin_is_locked(rlock);
}

static inline
int hybrid_spin_is_contended(struct raw_spinlock *rlock)
{
	return hard_spin_is_contended(rlock);
}

#ifdef CONFIG_DEBUG_IRQ_PIPELINE
void check_spinlock_context(void);
#else
static inline void check_spinlock_context(void) { }
#endif

#endif /* __LINUX_SPINLOCK_PIPELINE_H */
