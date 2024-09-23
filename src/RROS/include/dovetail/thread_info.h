/* SPDX-License-Identifier: GPL-2.0 */
#ifndef _DOVETAIL_THREAD_INFO_H
#define _DOVETAIL_THREAD_INFO_H

/*
 * Placeholder for per-thread state information defined by the
 * co-kernel.
 */

struct oob_thread_state {
    int preempt_count;

    void *thread;
    // struct rros_thread *thread;
    void *subscriber;
    // struct rros_subscriber *subscriber;
};

#endif /* !_DOVETAIL_THREAD_INFO_H */
