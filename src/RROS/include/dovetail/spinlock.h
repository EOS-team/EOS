/* SPDX-License-Identifier: GPL-2.0 */
#ifndef _DOVETAIL_SPINLOCK_H
#define _DOVETAIL_SPINLOCK_H

/* Placeholders for hard/hybrid spinlock modifiers. */

struct raw_spinlock;

static inline void hard_spin_lock_prepare(struct raw_spinlock *lock)
{ }

static inline void hard_spin_unlock_finish(struct raw_spinlock *lock)
{ }

static inline void hard_spin_trylock_prepare(struct raw_spinlock *lock)
{ }

static inline void hard_spin_trylock_fail(struct raw_spinlock *lock)
{ }

#endif /* !_DOVETAIL_SPINLOCK_H */
