/*
 * SPDX-License-Identifier: GPL-2.0
 *
 * Copyright (C) 2017 Philippe Gerum  <rpm@xenomai.org>.
 */
#include <linux/kernel.h>
#include <linux/module.h>
#include <linux/printk.h>
#include <linux/delay.h>
#include <linux/smp.h>
#include <linux/err.h>
#include <linux/cpumask.h>
#include <linux/clockchips.h>
#include <linux/interrupt.h>
#include <linux/irq.h>
#include <linux/irq_pipeline.h>
#include <linux/stop_machine.h>
#include <linux/slab.h>
#include "tick-internal.h"

static unsigned int proxy_tick_irq;

static DEFINE_MUTEX(proxy_mutex);

static DEFINE_PER_CPU(struct clock_proxy_device, proxy_tick_device);

static inline struct clock_event_device *
get_real_tick_device(struct clock_event_device *proxy_dev)
{
	return container_of(proxy_dev, struct clock_proxy_device, proxy_device)->real_device;
}

static void proxy_event_handler(struct clock_event_device *real_dev)
{
	struct clock_proxy_device *dev = raw_cpu_ptr(&proxy_tick_device);
	struct clock_event_device *proxy_dev = &dev->proxy_device;

	proxy_dev->event_handler(proxy_dev);
}

static int proxy_set_state_oneshot(struct clock_event_device *dev)
{
	struct clock_event_device *real_dev = get_real_tick_device(dev);
	unsigned long flags;
	int ret;

	flags = hard_local_irq_save();
	ret = real_dev->set_state_oneshot(real_dev);
	hard_local_irq_restore(flags);

	return ret;
}

static int proxy_set_state_periodic(struct clock_event_device *dev)
{
	struct clock_event_device *real_dev = get_real_tick_device(dev);
	unsigned long flags;
	int ret;

	flags = hard_local_irq_save();
	ret = real_dev->set_state_periodic(real_dev);
	hard_local_irq_restore(flags);

	return ret;
}

static int proxy_set_state_oneshot_stopped(struct clock_event_device *dev)
{
        struct clock_event_device *real_dev = get_real_tick_device(dev);
	unsigned long flags;
	int ret;

	flags = hard_local_irq_save();
	ret = real_dev->set_state_oneshot_stopped(real_dev);
	hard_local_irq_restore(flags);

	return ret;
}

static int proxy_set_state_shutdown(struct clock_event_device *dev)
{
        struct clock_event_device *real_dev = get_real_tick_device(dev);
	unsigned long flags;
	int ret;

	flags = hard_local_irq_save();
	ret = real_dev->set_state_shutdown(real_dev);
	hard_local_irq_restore(flags);

	return ret;
}

static void proxy_suspend(struct clock_event_device *dev)
{
        struct clock_event_device *real_dev = get_real_tick_device(dev);
	unsigned long flags;

	flags = hard_local_irq_save();
	real_dev->suspend(real_dev);
	hard_local_irq_restore(flags);
}

static void proxy_resume(struct clock_event_device *dev)
{
        struct clock_event_device *real_dev = get_real_tick_device(dev);
	unsigned long flags;

	flags = hard_local_irq_save();
	real_dev->resume(real_dev);
	hard_local_irq_restore(flags);
}

static int proxy_tick_resume(struct clock_event_device *dev)
{
        struct clock_event_device *real_dev = get_real_tick_device(dev);
	unsigned long flags;
	int ret;

	flags = hard_local_irq_save();
	ret = real_dev->tick_resume(real_dev);
	hard_local_irq_restore(flags);

	return ret;
}

static void proxy_broadcast(const struct cpumask *mask)
{
	struct clock_proxy_device *dev = raw_cpu_ptr(&proxy_tick_device);
        struct clock_event_device *real_dev = dev->real_device;
	unsigned long flags;

	flags = hard_local_irq_save();
	real_dev->broadcast(mask);
	hard_local_irq_restore(flags);
}

static int proxy_set_next_event(unsigned long delay,
				struct clock_event_device *dev)
{
	struct clock_event_device *real_dev = get_real_tick_device(dev);
	unsigned long flags;
	int ret;

	flags = hard_local_irq_save();
	ret = real_dev->set_next_event(delay, real_dev);
	hard_local_irq_restore(flags);

	return ret;
}

static int proxy_set_next_ktime(ktime_t expires,
				struct clock_event_device *dev)
{
	struct clock_event_device *real_dev = get_real_tick_device(dev);
	unsigned long flags;
	int ret;

	flags = hard_local_irq_save();
	ret = real_dev->set_next_ktime(expires, real_dev);
	hard_local_irq_restore(flags);

	return ret;
}

static irqreturn_t proxy_irq_handler(int sirq, void *dev_id)
{
	struct clock_event_device *evt;

	/*
	 * Tricky: we may end up running this in-band IRQ handler
	 * because tick_notify_proxy() was posted either:
	 *
	 * - from the out-of-band stage via ->handle_oob_event() for
	 * emulating an in-band tick.  In this case, the active tick
	 * device for the in-band timing core is the proxy device,
	 * whose event handler is still the same than the real tick
	 * device's.
	 *
	 * - directly by the clock chip driver on the local CPU via
	 * clockevents_handle_event(), for propagating a tick to the
	 * in-band stage nobody from the out-of-band stage is
	 * interested on i.e. no proxy device was registered on the
	 * receiving CPU, which was excluded from @cpumask in the call
	 * to tick_install_proxy(). In this case, the active tick
	 * device for the in-band timing core is a real clock event
	 * device.
	 *
	 * In both cases, we are running on the in-band stage, and we
	 * should fire the event handler of the currently active tick
	 * device for the in-band timing core.
	 */
	evt = raw_cpu_ptr(&tick_cpu_device)->evtdev;
	evt->event_handler(evt);

	return IRQ_HANDLED;
}

#define interpose_proxy_handler(__proxy, __real, __h)		\
	do {							\
		if ((__real)->__h)				\
			(__proxy)->__h = proxy_ ## __h;		\
	} while (0)

/*
 * Setup a proxy which is about to override the tick device on the
 * current CPU. Called with clockevents_lock held and irqs off so that
 * the tick device does not change under our feet.
 */
int tick_setup_proxy(struct clock_proxy_device *dev)
{
	struct clock_event_device *proxy_dev, *real_dev;

	real_dev = raw_cpu_ptr(&tick_cpu_device)->evtdev;
	if ((real_dev->features &
			(CLOCK_EVT_FEAT_PIPELINE|CLOCK_EVT_FEAT_ONESHOT))
		!= (CLOCK_EVT_FEAT_PIPELINE|CLOCK_EVT_FEAT_ONESHOT)) {
		WARN(1, "cannot use clockevent device %s in proxy mode!",
			real_dev->name);
		return -ENODEV;
	}

 	/*
 	 * The assumption is that neither us nor clockevents_register_proxy()
	 * can fail afterwards, so this is ok to advertise the new proxy as
	 * built by setting dev->real_device early.
 	 */
	dev->real_device = real_dev;
	dev->__original_handler = real_dev->event_handler;

	/*
	 * Inherit the feature bits since the proxy device has the
	 * same capabilities than the real one we are overriding
	 * (including CLOCK_EVT_FEAT_C3STOP if present).
	 */
	proxy_dev = &dev->proxy_device;
	memset(proxy_dev, 0, sizeof(*proxy_dev));
	proxy_dev->features = real_dev->features |
		CLOCK_EVT_FEAT_PERCPU | CLOCK_EVT_FEAT_PROXY;
	proxy_dev->name = "proxy";
	proxy_dev->irq = real_dev->irq;
	proxy_dev->bound_on = -1;
	proxy_dev->cpumask = cpumask_of(smp_processor_id());
	proxy_dev->rating = real_dev->rating + 1;
	proxy_dev->mult = real_dev->mult;
	proxy_dev->shift = real_dev->shift;
	proxy_dev->max_delta_ticks = real_dev->max_delta_ticks;
	proxy_dev->min_delta_ticks = real_dev->min_delta_ticks;
	proxy_dev->max_delta_ns = real_dev->max_delta_ns;
	proxy_dev->min_delta_ns = real_dev->min_delta_ns;
	/*
	 * Interpose default handlers which are safe wrt preemption by
	 * the out-of-band stage.
	 */
	interpose_proxy_handler(proxy_dev, real_dev, set_state_oneshot);
	interpose_proxy_handler(proxy_dev, real_dev, set_state_oneshot_stopped);
	interpose_proxy_handler(proxy_dev, real_dev, set_state_periodic);
	interpose_proxy_handler(proxy_dev, real_dev, set_state_shutdown);
	interpose_proxy_handler(proxy_dev, real_dev, suspend);
	interpose_proxy_handler(proxy_dev, real_dev, resume);
	interpose_proxy_handler(proxy_dev, real_dev, tick_resume);
	interpose_proxy_handler(proxy_dev, real_dev, broadcast);
	interpose_proxy_handler(proxy_dev, real_dev, set_next_event);
	interpose_proxy_handler(proxy_dev, real_dev, set_next_ktime);

	dev->__setup_handler(dev);

	return 0;
}

static int enable_oob_timer(void *arg) /* hard_irqs_disabled() */
{
	struct clock_proxy_device *dev = raw_cpu_ptr(&proxy_tick_device);
	struct clock_event_device *real_dev;

	/*
	 * Install the out-of-band handler on this CPU's real clock
	 * device, then turn on out-of-band mode for the associated
	 * IRQ (duplicates are silently ignored if the IRQ is common
	 * to multiple CPUs).
	 */
	real_dev = dev->real_device;
	real_dev->event_handler = dev->handle_oob_event;
	real_dev->features |= CLOCK_EVT_FEAT_OOB;
	barrier();

	/*
	 * irq_switch_oob() grabs the IRQ descriptor lock which is
	 * mutable, so that is fine to invoke this routine with hard
	 * IRQs off.
	 */
	irq_switch_oob(real_dev->irq, true);

	return 0;
}

struct proxy_install_arg {
	void (*setup_proxy)(struct clock_proxy_device *dev);
	int result;
};

static void register_proxy_device(void *arg) /* irqs_disabled() */
{
	struct clock_proxy_device *dev = raw_cpu_ptr(&proxy_tick_device);
	struct proxy_install_arg *req = arg;
	int ret;

	dev->__setup_handler = req->setup_proxy;
	ret = clockevents_register_proxy(dev);
	if (ret) {
		if (!req->result)
			req->result = ret;
	} else {
		dev->real_device->event_handler = proxy_event_handler;
	}
}

int tick_install_proxy(void (*setup_proxy)(struct clock_proxy_device *dev),
		const struct cpumask *cpumask)
{
	struct proxy_install_arg arg;
	int ret, sirq;

	mutex_lock(&proxy_mutex);

	ret = -EAGAIN;
	if (proxy_tick_irq)
		goto out;

	sirq = irq_create_direct_mapping(synthetic_irq_domain);
	if (WARN_ON(sirq == 0))
		goto out;

	ret = __request_percpu_irq(sirq, proxy_irq_handler,
				   IRQF_NO_THREAD, /* no IRQF_TIMER here. */
				   "proxy tick",
				   &proxy_tick_device);
	if (WARN_ON(ret)) {
		irq_dispose_mapping(sirq);
		goto out;
	}

	proxy_tick_irq = sirq;
	barrier();

	/*
	 * Install a proxy tick device on each CPU. As the proxy
	 * device is picked, the previous (real) tick device is
	 * switched to reserved state by the clockevent core.
	 * Immediately after, the proxy device starts controlling the
	 * real device under the hood to carry out the timing requests
	 * it receives.
	 *
	 * For a short period of time, after the proxy device is
	 * installed, and until the real device IRQ is switched to
	 * out-of-band mode, the flow is as follows:
	 *
	 *    [inband timing request]
	 *        proxy_dev->set_next_event(proxy_dev)
	 *            oob_program_event(proxy_dev)
	 *                real_dev->set_next_event(real_dev)
	 *        ...
	 *        <tick event>
	 *        original_timer_handler() [in-band stage]
	 *            clockevents_handle_event(real_dev)
	 *               proxy_event_handler(real_dev)
	 *                  inband_event_handler(proxy_dev)
	 *
	 * Eventually, we substitute the original (in-band) clock
	 * event handler with the out-of-band handler for the real
	 * clock event device, then turn on out-of-band mode for the
	 * timer IRQ associated to the latter. These two steps are
	 * performed over a stop_machine() context, so that no tick
	 * can race with this code while we swap handlers.
	 *
	 * Once the hand over is complete, the flow is as follows:
	 *
	 *    [inband timing request]
	 *        proxy_dev->set_next_event(proxy_dev)
	 *            oob_program_event(proxy_dev)
	 *                real_dev->set_next_event(real_dev)
	 *        ...
	 *        <tick event>
	 *        inband_event_handler() [out-of-band stage]
	 *            clockevents_handle_event(real_dev)
	 *                handle_oob_event(proxy_dev)
	 *                    ...(inband tick emulation)...
	 *                         tick_notify_proxy()
	 *        ...
	 *        proxy_irq_handler(proxy_dev) [in-band stage]
	 *            clockevents_handle_event(proxy_dev)
	 *                inband_event_handler(proxy_dev)
	 */
	arg.setup_proxy = setup_proxy;
	arg.result = 0;
	on_each_cpu_mask(cpumask, register_proxy_device, &arg, true);
	if (arg.result) {
		tick_uninstall_proxy(cpumask);
		return arg.result;
	}

	/*
	 * Start ticking from the out-of-band interrupt stage upon
	 * receipt of out-of-band timer events.
	 */
	stop_machine(enable_oob_timer, NULL, cpumask);
out:
	mutex_unlock(&proxy_mutex);

	return ret;
}
EXPORT_SYMBOL_GPL(tick_install_proxy);

static int disable_oob_timer(void *arg) /* hard_irqs_disabled() */
{
	struct clock_proxy_device *dev = raw_cpu_ptr(&proxy_tick_device);
	struct clock_event_device *real_dev;

	dev = raw_cpu_ptr(&proxy_tick_device);
	real_dev = dev->real_device;
	real_dev->event_handler = dev->__original_handler;
	real_dev->features &= ~CLOCK_EVT_FEAT_OOB;
	barrier();

	irq_switch_oob(real_dev->irq, false);

	return 0;
}

static void unregister_proxy_device(void *arg) /* irqs_disabled() */
{
	struct clock_proxy_device *dev = raw_cpu_ptr(&proxy_tick_device);

	if (dev->real_device) {
		clockevents_unregister_proxy(dev);
		dev->real_device = NULL;
	}
}

void tick_uninstall_proxy(const struct cpumask *cpumask)
{
	/*
	 * Undo all we did in tick_install_proxy(), handing over
	 * control of the tick device back to the inband code.
	 */
	mutex_lock(&proxy_mutex);
	stop_machine(disable_oob_timer, NULL, cpu_online_mask);
	on_each_cpu_mask(cpumask, unregister_proxy_device, NULL, true);
	free_percpu_irq(proxy_tick_irq, &proxy_tick_device);
	irq_dispose_mapping(proxy_tick_irq);
	proxy_tick_irq = 0;
	mutex_unlock(&proxy_mutex);
}
EXPORT_SYMBOL_GPL(tick_uninstall_proxy);

void tick_notify_proxy(void)
{
	/*
	 * Schedule a tick on the proxy device to occur from the
	 * in-band stage, which will trigger proxy_irq_handler() at
	 * some point (i.e. when the in-band stage is back in control
	 * and not stalled). Note that we might be called from the
	 * in-band stage in some cases (see proxy_irq_handler()).
	 */
	irq_post_inband(proxy_tick_irq);
}
EXPORT_SYMBOL_GPL(tick_notify_proxy);
