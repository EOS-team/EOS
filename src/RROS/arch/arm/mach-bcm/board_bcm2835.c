// SPDX-License-Identifier: GPL-2.0+
/*
 * Copyright (C) 2010 Broadcom
 */

#include <linux/init.h>
#include <linux/irqchip.h>
#include <linux/mm.h>
#include <linux/of_address.h>
#include <linux/of_fdt.h>
#include <asm/system_info.h>

#include <asm/mach/arch.h>
#include <asm/mach/map.h>
#include <asm/memory.h>
#include <asm/pgtable.h>

#include "platsmp.h"

#define BCM2835_USB_VIRT_BASE   (VMALLOC_START)
#define BCM2835_USB_VIRT_MPHI   (VMALLOC_START + 0x10000)

static void __init bcm2835_init(void)
{
	struct device_node *np = of_find_node_by_path("/system");
	u32 val;
	u64 val64;

	if (!of_property_read_u32(np, "linux,revision", &val))
		system_rev = val;
	if (!of_property_read_u64(np, "linux,serial", &val64))
		system_serial_low = val64;
}

/*
 * We need to map registers that are going to be accessed by the FIQ
 * very early, before any kernel threads are spawned. Because if done
 * later, the mapping tables are not updated instantly but lazily upon
 * first access through a data abort handler. While that is fine
 * when executing regular kernel code, if the first access in a specific
 * thread happens while running FIQ code this will result in a panic.
 *
 * For more background see the following old mailing list thread:
 * https://www.spinics.net/lists/arm-kernel/msg325250.html
 */
static int __init bcm2835_map_usb(unsigned long node, const char *uname,
					int depth, void *data)
{
	struct map_desc map[2];
	const __be32 *reg;
	int len;
	unsigned long p2b_offset = *((unsigned long *) data);

	if (!of_flat_dt_is_compatible(node, "brcm,bcm2708-usb"))
		return 0;
	reg = of_get_flat_dt_prop(node, "reg", &len);
	if (!reg || len != (sizeof(unsigned long) * 4))
		return 0;

	/* Use information about the physical addresses of the
	 * registers from the device tree, but use legacy
	 * iotable_init() static mapping function to map them,
	 * as ioremap() is not functional at this stage in boot.
	 */
	map[0].virtual = (unsigned long) BCM2835_USB_VIRT_BASE;
	map[0].pfn = __phys_to_pfn(be32_to_cpu(reg[0]) - p2b_offset);
	map[0].length = be32_to_cpu(reg[1]);
	map[0].type = MT_DEVICE;
	map[1].virtual = (unsigned long) BCM2835_USB_VIRT_MPHI;
	map[1].pfn = __phys_to_pfn(be32_to_cpu(reg[2]) - p2b_offset);
	map[1].length = be32_to_cpu(reg[3]);
	map[1].type = MT_DEVICE;
		iotable_init(map, 2);

	return 1;
}

static void __init bcm2835_map_io(void)
{
	const __be32 *ranges, *address_cells;
	unsigned long root, addr_cells;
	int soc, len;
	unsigned long p2b_offset;

	debug_ll_io_init();

	root = of_get_flat_dt_root();
	/* Find out how to map bus to physical address first from soc/ranges */
	soc = of_get_flat_dt_subnode_by_name(root, "soc");
	if (soc < 0)
		return;
	address_cells = of_get_flat_dt_prop(root, "#address-cells", &len);
	if (!address_cells || len < (sizeof(unsigned long)))
		return;
	addr_cells = be32_to_cpu(address_cells[0]);
	ranges = of_get_flat_dt_prop(soc, "ranges", &len);
	if (!ranges || len < (sizeof(unsigned long) * (2 + addr_cells)))
		return;
	p2b_offset = be32_to_cpu(ranges[0]) - be32_to_cpu(ranges[addr_cells]);

	/* Now search for bcm2708-usb node in device tree */
	of_scan_flat_dt(bcm2835_map_usb, &p2b_offset);
}

static const char * const bcm2835_compat[] = {
#ifdef CONFIG_ARCH_MULTI_V6
	"brcm,bcm2835",
#endif
#ifdef CONFIG_ARCH_MULTI_V7
	"brcm,bcm2836",
	"brcm,bcm2837",
#endif
	NULL
};

DT_MACHINE_START(BCM2835, "BCM2835")
	.map_io = bcm2835_map_io,
	.init_machine = bcm2835_init,
	.dt_compat = bcm2835_compat,
	.smp = smp_ops(bcm2836_smp_ops),
MACHINE_END

static const char * const bcm2711_compat[] = {
#ifdef CONFIG_ARCH_MULTI_V7
	"brcm,bcm2711",
#endif
	NULL
};

DT_MACHINE_START(BCM2711, "BCM2711")
#if defined(CONFIG_ZONE_DMA) && defined(CONFIG_ARM_LPAE)
	.dma_zone_size	= SZ_1G,
#endif
	.map_io = bcm2835_map_io,
	.init_machine = bcm2835_init,
	.dt_compat = bcm2711_compat,
	.smp = smp_ops(bcm2836_smp_ops),
MACHINE_END
