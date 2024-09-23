/* SPDX-License-Identifier: GPL-2.0 */
#ifndef _NET_OOBNET_H
#define _NET_OOBNET_H

#include <dovetail/netdevice.h>

/* Device supports direct out-of-band operations (RX & TX) */
#define IFF_OOB_CAPABLE		BIT(0)
/* Device is an out-of-band port */
#define IFF_OOB_PORT		BIT(1)

struct oob_netdev_context {
	int flags;
	struct oob_netdev_state dev_state;
};

#endif /* !_NET_OOBNET_H */
