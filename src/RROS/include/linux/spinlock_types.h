#ifndef __LINUX_SPINLOCK_TYPES_H
#define __LINUX_SPINLOCK_TYPES_H

/*
 * include/linux/spinlock_types.h - generic spinlock type definitions
 *                                  and initializers
 *
 * portions Copyright 2005, Red Hat, Inc., Ingo Molnar
 * Released under the General Public License (GPL).
 */

#if defined(CONFIG_SMP)
# include <asm/spinlock_types.h>
#else
# include <linux/spinlock_types_up.h>
#endif

#include <linux/lockdep_types.h>

typedef struct raw_spinlock {
	arch_spinlock_t raw_lock;
#ifdef CONFIG_DEBUG_SPINLOCK
	unsigned int magic, owner_cpu;
	void *owner;
#endif
#ifdef CONFIG_DEBUG_LOCK_ALLOC
	struct lockdep_map dep_map;
#endif
} raw_spinlock_t;

#define SPINLOCK_MAGIC		0xdead4ead

#define SPINLOCK_OWNER_INIT	((void *)-1L)

#ifdef CONFIG_DEBUG_LOCK_ALLOC
# define RAW_SPIN_DEP_MAP_INIT(lockname)		\
	.dep_map = {					\
		.name = #lockname,			\
		.wait_type_inner = LD_WAIT_SPIN,	\
	}
# define SPIN_DEP_MAP_INIT(lockname)			\
	.dep_map = {					\
		.name = #lockname,			\
		.wait_type_inner = LD_WAIT_CONFIG,	\
	}
#else
# define RAW_SPIN_DEP_MAP_INIT(lockname)
# define SPIN_DEP_MAP_INIT(lockname)
#endif

#ifdef CONFIG_DEBUG_SPINLOCK
# define SPIN_DEBUG_INIT(lockname)		\
	.magic = SPINLOCK_MAGIC,		\
	.owner_cpu = -1,			\
	.owner = SPINLOCK_OWNER_INIT,
#else
# define SPIN_DEBUG_INIT(lockname)
#endif

#define __RAW_SPIN_LOCK_INITIALIZER(lockname)	\
	{					\
	.raw_lock = __ARCH_SPIN_LOCK_UNLOCKED,	\
	SPIN_DEBUG_INIT(lockname)		\
	RAW_SPIN_DEP_MAP_INIT(lockname) }

#define __RAW_SPIN_LOCK_UNLOCKED(lockname)	\
	(raw_spinlock_t) __RAW_SPIN_LOCK_INITIALIZER(lockname)

#define DEFINE_RAW_SPINLOCK(x)	raw_spinlock_t x = __RAW_SPIN_LOCK_UNLOCKED(x)

typedef struct spinlock {
	union {
		struct raw_spinlock rlock;

#ifdef CONFIG_DEBUG_LOCK_ALLOC
# define LOCK_PADSIZE (offsetof(struct raw_spinlock, dep_map))
		struct {
			u8 __padding[LOCK_PADSIZE];
			struct lockdep_map dep_map;
		};
#endif
	};
} spinlock_t;

#define ___SPIN_LOCK_INITIALIZER(lockname)	\
	{					\
	.raw_lock = __ARCH_SPIN_LOCK_UNLOCKED,	\
	SPIN_DEBUG_INIT(lockname)		\
	SPIN_DEP_MAP_INIT(lockname) }

#define __SPIN_LOCK_INITIALIZER(lockname) \
	{ { .rlock = ___SPIN_LOCK_INITIALIZER(lockname) } }

#define __SPIN_LOCK_UNLOCKED(lockname) \
	(spinlock_t) __SPIN_LOCK_INITIALIZER(lockname)

#define DEFINE_SPINLOCK(x)	spinlock_t x = __SPIN_LOCK_UNLOCKED(x)

#ifdef CONFIG_IRQ_PIPELINE

void __bad_spinlock_type(void);

#define __RAWLOCK(x) ((struct raw_spinlock *)(x))

#define LOCK_ALTERNATIVES(__lock, __base_op, __raw_form, __args...)	\
	do {								\
		if (__builtin_types_compatible_p(typeof(__lock),	\
						 raw_spinlock_t *))	\
			__raw_form;					\
		else if (__builtin_types_compatible_p(typeof(__lock),	\
						 hard_spinlock_t *))	\
			hard_ ## __base_op(__RAWLOCK(__lock), ##__args); \
		else if (__builtin_types_compatible_p(typeof(__lock),	\
						 hybrid_spinlock_t *))	\
			hybrid_ ## __base_op(__RAWLOCK(__lock), ##__args); \
		else							\
			__bad_spinlock_type();				\
	} while (0)

#define LOCK_ALTERNATIVES_RET(__lock, __base_op, __raw_form, __args...) \
	({								\
		long __ret = 0;						\
		if (__builtin_types_compatible_p(typeof(__lock),	\
						 raw_spinlock_t *))	\
			__ret = __raw_form;				\
		else if (__builtin_types_compatible_p(typeof(__lock),	\
						 hard_spinlock_t *))	\
			__ret = hard_ ## __base_op(__RAWLOCK(__lock), ##__args); \
		else if (__builtin_types_compatible_p(typeof(__lock),	\
						 hybrid_spinlock_t *))	\
			__ret = hybrid_ ## __base_op(__RAWLOCK(__lock), ##__args); \
		else							\
			__bad_spinlock_type();				\
		__ret;							\
	})

#define LOCKDEP_ALT_DEPMAP(__lock)					\
	({								\
		struct lockdep_map *__ret;				\
		if (__builtin_types_compatible_p(typeof(&(__lock)->dep_map), \
						 struct phony_lockdep_map *)) \
			__ret = &__RAWLOCK(__lock)->dep_map;		\
		else							\
			__ret = (struct lockdep_map *)(&(__lock)->dep_map); \
		__ret;							\
	})

#define LOCKDEP_HARD_DEBUG(__lock, __nodebug, __debug)	\
	do {						\
		if (__builtin_types_compatible_p(typeof(__lock),	\
						raw_spinlock_t *) ||	\
			irq_pipeline_debug_locking()) {			\
			__debug;			\
		} else {				\
			__nodebug;			\
		}					\
	} while (0)

#define LOCKDEP_HARD_DEBUG_RET(__lock, __nodebug, __debug)	\
	({						\
		typeof(__nodebug) __ret;		\
		if (__builtin_types_compatible_p(typeof(__lock),	\
						raw_spinlock_t *) ||	\
			irq_pipeline_debug_locking()) {			\
			__ret = (__debug);		\
		} else {				\
			__ret = (__nodebug);		\
		}					\
		__ret;					\
	})

#define __HARD_SPIN_LOCK_UNLOCKED(__rlock)	\
	__RAW_SPIN_LOCK_UNLOCKED(__rlock)

#define __HARD_SPIN_LOCK_INITIALIZER(__lock)				\
	{								\
		.rlock = __HARD_SPIN_LOCK_UNLOCKED((__lock).rlock),	\
	}

#define DEFINE_HARD_SPINLOCK(x)	hard_spinlock_t x = {	\
		.rlock = __HARD_SPIN_LOCK_UNLOCKED(x),	\
	}

#define DECLARE_HARD_SPINLOCK(x)	hard_spinlock_t x

struct phony_lockdep_map {
	short	wait_type_outer;
	short	wait_type_inner;
};

typedef struct hard_spinlock {
	/* XXX: offset_of(struct hard_spinlock, rlock) == 0 */
	struct raw_spinlock rlock;
	struct phony_lockdep_map dep_map;
} hard_spinlock_t;

#define DEFINE_MUTABLE_SPINLOCK(x)	hybrid_spinlock_t x = {	\
		.rlock = __RAW_SPIN_LOCK_UNLOCKED(x),			\
	}

#define DECLARE_MUTABLE_SPINLOCK(x)	hybrid_spinlock_t x

typedef struct hybrid_spinlock {
	/* XXX: offset_of(struct hybrid_spinlock, rlock) == 0 */
	struct raw_spinlock rlock;
	unsigned long hwflags;
	struct phony_lockdep_map dep_map;
} hybrid_spinlock_t;

#else

typedef raw_spinlock_t hard_spinlock_t;

typedef raw_spinlock_t hybrid_spinlock_t;

#define LOCK_ALTERNATIVES(__lock, __base_op, __raw_form, __args...)	\
	__raw_form

#define LOCK_ALTERNATIVES_RET(__lock, __base_op, __raw_form, __args...) \
	__raw_form

#define LOCKDEP_ALT_DEPMAP(__lock)	(&(__lock)->dep_map)

#define LOCKDEP_HARD_DEBUG(__lock, __nondebug, __debug)		do { __debug; } while (0)

#define LOCKDEP_HARD_DEBUG_RET(__lock, __nondebug, __debug)	({ __debug; })

#define DEFINE_HARD_SPINLOCK(x)		DEFINE_RAW_SPINLOCK(x)

#define DECLARE_HARD_SPINLOCK(x)	raw_spinlock_t x

#define DEFINE_MUTABLE_SPINLOCK(x)	DEFINE_RAW_SPINLOCK(x)

#define DECLARE_MUTABLE_SPINLOCK(x)	raw_spinlock_t x

#define __RAWLOCK(x) (x)

#define __HARD_SPIN_LOCK_UNLOCKED(__lock)	__RAW_SPIN_LOCK_UNLOCKED(__lock)

#define __HARD_SPIN_LOCK_INITIALIZER(__lock)	__RAW_SPIN_LOCK_UNLOCKED(__lock)

#endif	/* CONFIG_IRQ_PIPELINE */

#include <linux/rwlock_types.h>

#endif /* __LINUX_SPINLOCK_TYPES_H */
