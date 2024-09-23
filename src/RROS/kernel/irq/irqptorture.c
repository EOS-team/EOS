/*
 * SPDX-License-Identifier: GPL-2.0
 *
 * Copyright (C) 2017 Philippe Gerum  <rpm@xenomai.org>.
 */
#include <linux/kernel.h>
#include <linux/ktime.h>
#include <linux/torture.h>
#include <linux/printk.h>
#include <linux/delay.h>
#include <linux/tick.h>
#include <linux/smp.h>
#include <linux/cpumask.h>
#include <linux/clockchips.h>
#include <linux/interrupt.h>
#include <linux/irq.h>
#include <linux/irq_pipeline.h>
#include <linux/stop_machine.h>
#include <linux/irq_work.h>
#include <linux/completion.h>
#include <linux/slab.h>
#include "settings.h"

static void torture_event_handler(struct clock_event_device *dev)
{
	pr_info("tick test!!!");
	/*
	 * We are running on the oob stage, in NMI-like mode. Schedule
	 * a tick on the proxy device to satisfy the corresponding
	 * timing request asap.
	 */
	tick_notify_proxy();
}

static void setup_proxy(struct clock_proxy_device *dev)
{
	dev->handle_oob_event = torture_event_handler;
}

static int start_tick_takeover_test(void)
{
	return tick_install_proxy(setup_proxy, cpu_online_mask);
}

static void stop_tick_takeover_test(void)
{
	tick_uninstall_proxy(cpu_online_mask);
}

struct stop_machine_p_data {
	int origin_cpu;
	cpumask_var_t disable_mask;
};

static int stop_machine_handler(void *arg)
{
	struct stop_machine_p_data *p = arg;
	int cpu = raw_smp_processor_id();

	/*
	 * The stop_machine() handler must run with hard
	 * IRQs off, note the current state in the result mask.
	 */
	if (hard_irqs_disabled())
		cpumask_set_cpu(cpu, p->disable_mask);

	if (cpu != p->origin_cpu)
		pr_alert("irq_pipeline" TORTURE_FLAG
			 " CPU%d responds to stop_machine()\n", cpu);
	return 0;
}

/*
 * We test stop_machine() as a way to validate IPI handling in a
 * pipelined interrupt context.
 */
static int test_stop_machine(void)
{
	struct stop_machine_p_data d;
	cpumask_var_t tmp_mask;
	int ret = -ENOMEM, cpu;

	if (!zalloc_cpumask_var(&d.disable_mask, GFP_KERNEL)) {
		WARN_ON(1);
		return ret;
	}

	if (!alloc_cpumask_var(&tmp_mask, GFP_KERNEL)) {
		WARN_ON(1);
		goto fail;
	}

	ret = -EINVAL;
	d.origin_cpu = raw_smp_processor_id();
	pr_alert("irq_pipeline" TORTURE_FLAG
		 " CPU%d initiates stop_machine()\n",
		 d.origin_cpu);

	ret = stop_machine(stop_machine_handler, &d, cpu_online_mask);
	WARN_ON(ret);
	if (ret)
		goto fail;

	/*
	 * Check whether all handlers did run with hard IRQs off. If
	 * some of them did not, then we have a problem with the stop
	 * IRQ delivery.
	 */
	cpumask_xor(tmp_mask, cpu_online_mask, d.disable_mask);
	if (!cpumask_empty(tmp_mask)) {
		for_each_cpu(cpu, tmp_mask)
			pr_alert("irq_pipeline" TORTURE_FLAG
				 " CPU%d: hard IRQs ON in stop_machine()"
				 " handler!\n", cpu);
	}

	free_cpumask_var(tmp_mask);
fail:
	free_cpumask_var(d.disable_mask);

	return ret;
}

static struct irq_work_tester {
	struct irq_work work;
	struct completion done;
} irq_work_tester;

static void irq_work_handler(struct irq_work *work)
{
	int cpu = raw_smp_processor_id();

	if (!running_inband()) {
		pr_alert("irq_pipeline" TORTURE_FLAG
			 " CPU%d: irq_work handler not running on"
			 " in-band stage?!\n", cpu);
		return;
	}

	if (work != &irq_work_tester.work)
		pr_alert("irq_pipeline" TORTURE_FLAG
			 " CPU%d: irq_work handler received broken"
			 " arg?!\n", cpu);
	else {
		pr_alert("irq_pipeline" TORTURE_FLAG
			 " CPU%d: irq_work handled\n", cpu);
		complete(&irq_work_tester.done);
	}
}

static int trigger_oob_work(void *arg)
{
	int cpu = raw_smp_processor_id();

	if (!running_oob()) {
		pr_alert("irq_pipeline" TORTURE_FLAG
			 " CPU%d: escalated request not running on"
			 " oob stage?!\n", cpu);
		return -EINVAL;
	}

	if ((struct irq_work_tester *)arg != &irq_work_tester) {
		pr_alert("irq_pipeline" TORTURE_FLAG
			 " CPU%d: escalation handler received broken"
			 " arg?!\n", cpu);
		return -EINVAL;
	}

	irq_work_queue(&irq_work_tester.work);
	pr_alert("irq_pipeline" TORTURE_FLAG
		 " CPU%d: stage escalation request works\n",
		 cpu);

	return 0;
}

static int test_interstage_work_injection(void)
{
	struct irq_work_tester *p = &irq_work_tester;
	int ret, cpu = raw_smp_processor_id();
	unsigned long rem;

	init_completion(&p->done);
	init_irq_work(&p->work, irq_work_handler);

	/* Trigger over the in-band stage. */
	irq_work_queue(&p->work);
	rem = wait_for_completion_timeout(&p->done, HZ / 10);
	if (!rem) {
		pr_alert("irq_pipeline" TORTURE_FLAG
			 " CPU%d: irq_work trigger from in-band stage not handled!\n",
			 cpu);
		return -EINVAL;
	}

	pr_alert("irq_pipeline" TORTURE_FLAG
		 " CPU%d: in-band->in-band irq_work trigger works\n", cpu);

	reinit_completion(&p->done);

	/* Now try over the oob stage. */
	ret = run_oob_call(trigger_oob_work, p);
	if (ret)
		return ret;

	ret = wait_for_completion_timeout(&p->done, HZ / 10);
	if (!rem) {
		pr_alert("irq_pipeline" TORTURE_FLAG
			 " CPU%d: irq_work trigger from oob"
			 " stage not handled!\n", cpu);
		return -EINVAL;
	}

	pr_alert("irq_pipeline" TORTURE_FLAG
		 " CPU%d: oob->in-band irq_work trigger works\n",
		 cpu);

	return 0;
}

static int __init irqp_torture_init(void)
{
	int ret;

	pr_info("Starting IRQ pipeline tests...");

	ret = enable_oob_stage("torture");
	if (ret) {
		if (ret == -EBUSY)
			pr_alert("irq_pipeline" TORTURE_FLAG
			 " won't run, oob stage '%s' is already installed",
			 oob_stage.name);

		return ret;
	}

	ret = test_stop_machine();
	if (ret)
		goto out;

	ret = start_tick_takeover_test();
	if (ret)
		goto out;

	ret = test_interstage_work_injection();
	if (!ret)
		msleep(1000);

	stop_tick_takeover_test();
out:
	disable_oob_stage();
	pr_info("IRQ pipeline tests %s.", ret ? "FAILED" : "OK");

	return 0;
}
late_initcall(irqp_torture_init);
