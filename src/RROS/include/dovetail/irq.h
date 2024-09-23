/* SPDX-License-Identifier: GPL-2.0 */
#ifndef _DOVETAIL_IRQ_H
#define _DOVETAIL_IRQ_H

/* Placeholders for pre- and post-IRQ handling. */

extern void this_rros_rq_enter_irq_local_flags(void);
extern int this_rros_rq_exit_irq_local_flags(void);
extern void rros_schedule(void);

static inline void irq_enter_pipeline(void) { 
    this_rros_rq_enter_irq_local_flags();
}

static inline void irq_exit_pipeline(void) { 
    // rros_flag = 1;
    int ret = this_rros_rq_exit_irq_local_flags();
    if (ret == 1)
    {
        rros_schedule();
    }
 }

#endif /* !_DOVETAIL_IRQ_H */
