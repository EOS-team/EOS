/*
 * Definitions for user-mappable clock sources.
 *
 * Gilles Chanteperdrix <gilles.chanteperdrix@xenomai.org>
 */
#ifndef _UAPI_LINUX_CLOCKSOURCE_H
#define _UAPI_LINUX_CLOCKSOURCE_H

enum clksrc_user_mmio_type {
	CLKSRC_MMIO_L_UP,
	CLKSRC_MMIO_L_DOWN,
	CLKSRC_MMIO_W_UP,
	CLKSRC_MMIO_W_DOWN,
	CLKSRC_DMMIO_L_UP,
	CLKSRC_DMMIO_W_UP,

	CLKSRC_MMIO_TYPE_NR,
};

struct clksrc_user_mmio_info {
	enum clksrc_user_mmio_type type;
	void *reg_lower;
	unsigned int mask_lower;
	unsigned int bits_lower;
	void *reg_upper;
	unsigned int mask_upper;
};

#define CLKSRC_USER_MMIO_MAX 16

#define CLKSRC_USER_MMIO_MAP _IOWR(0xC1, 0, struct clksrc_user_mmio_info)

#endif /* _UAPI_LINUX_CLOCKSOURCE_H */
