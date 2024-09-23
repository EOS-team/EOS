#include <linux/kernel.h>
#include <linux/console.h>
#include <linux/init.h>

/*
 * If both CONFIG_DEBUG_LL and CONFIG_RAW_PRINTK are set, create a
 * console device sending the raw output to printascii().
 */
void printascii(const char *s);

static void raw_console_write(struct console *co,
			      const char *s, unsigned count)
{
	printascii(s);
}

static struct console raw_console = {
	.name		= "rawcon",
	.write_raw	= raw_console_write,
	.flags		= CON_PRINTBUFFER | CON_ENABLED,
	.index		= -1,
};

static int __init raw_console_init(void)
{
	register_console(&raw_console);

	return 0;
}
console_initcall(raw_console_init);
