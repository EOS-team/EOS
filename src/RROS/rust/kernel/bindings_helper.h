/* SPDX-License-Identifier: GPL-2.0 */

#include <linux/cdev.h>
#include <linux/errname.h>
#include <linux/fs.h>
#include <linux/module.h>
#include <linux/random.h>
#include <linux/slab.h>
#include <linux/sysctl.h>
#include <linux/uaccess.h>
#include <linux/uio.h>
#include <linux/version.h>
#include <linux/miscdevice.h>
#include <linux/poll.h>
#include <linux/mm.h>
#include <linux/file.h>
#include <uapi/linux/android/binder.h>
#include <linux/platform_device.h>
#include <linux/of_platform.h>
#include <linux/security.h>
#include <linux/vmalloc.h>
#include <linux/slab.h>
#include <linux/irq_work.h>
#include <linux/interrupt.h>
#include <linux/list.h>
#include <linux/tick.h>
#include <uapi/linux/sched/types.h>
#include <linux/irq_pipeline.h>
#include <net/net_namespace.h>
#include <linux/netdevice.h>
#include <uapi/linux/unistd.h>
#include <net/sock.h>
#include <linux/net.h>
#include <linux/bottom_half.h>
#include <uapi/linux/unistd.h>
#include <linux/capability.h>
#include <linux/anon_inodes.h>

// `bindgen` gets confused at certain things
const gfp_t BINDINGS_GFP_KERNEL = GFP_KERNEL;
const gfp_t BINDINGS___GFP_ZERO = __GFP_ZERO;
