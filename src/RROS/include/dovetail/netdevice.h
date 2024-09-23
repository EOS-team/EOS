/* SPDX-License-Identifier: GPL-2.0 */
#ifndef _DOVETAIL_NETDEVICE_H
#define _DOVETAIL_NETDEVICE_H

/*
 * Placeholder for per-device state information defined by the
 * out-of-band network stack.
 */
#ifndef _ASM_GENERIC_EVL_NETDEVICE_H
struct oob_netdev_state {
    void * wrapper;
};

#endif /* !_ASM_GENERIC_EVL_NETDEVICE_H */

#endif /* !_DOVETAIL_NETDEVICE_H */
