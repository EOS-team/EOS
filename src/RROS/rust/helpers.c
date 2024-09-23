// SPDX-License-Identifier: GPL-2.0

#include <asm-generic/irq_pipeline.h>
#include <linux/bug.h>
#include <linux/build_bug.h>
#include <linux/clk.h>
#include <linux/clockchips.h>
#include <linux/uaccess.h>
#include <linux/sched/signal.h>
#include <linux/gfp.h>
#include <linux/highmem.h>
#include <linux/uio.h>
#include <linux/kthread.h>
#include <linux/ktime.h>
#include <linux/errname.h>
#include <linux/mutex.h>
#include <linux/platform_device.h>
#include <linux/security.h>
#include <linux/cpumask.h>
#include <linux/kernel.h>
#include <linux/preempt.h>
#include <linux/slab.h>
#include <linux/types.h>
#include <linux/percpu-defs.h>
#include <linux/percpu.h>
#include <asm/io.h>
#include <linux/irq.h>
#include <linux/irqchip/chained_irq.h>
#include <linux/irqdomain.h>
#include <linux/amba/bus.h>
#include <linux/of_device.h>
#include <linux/skbuff.h>
#include <linux/netdevice.h>
#include <linux/device/class.h>
#include <linux/export.h>
#include <linux/module.h>
#include <linux/irq_pipeline.h>
#include <asm-generic/irq_pipeline.h>
#include <linux/tick.h>
#include <linux/netdevice.h>
#include <linux/net.h>
#include <net/net_namespace.h>
#include <linux/completion.h>
#include <linux/irqstage.h>
#include <linux/dovetail.h>
#include <linux/spinlock_pipeline.h>
#include <linux/preempt.h>
#include <linux/signal_types.h>
#include <linux/spinlock_types.h>
#include <linux/log2.h>
#include <linux/capability.h>
#include <linux/spinlock.h>
#include <linux/spinlock_types.h>
#include <linux/capability.h>
#include <asm/uaccess.h>
#include <linux/dovetail.h>
#include <linux/spinlock_pipeline.h>
#include <linux/log2.h>
#include <linux/capability.h>
#include <linux/spinlock.h>
#include <linux/spinlock_types.h>

#include <linux/wait.h>
#include <net/sock.h>
#include <linux/jhash.h>
#include <linux/bottom_half.h>
#include <linux/if_vlan.h>
#include <linux/skbuff.h>
#include <linux/hashtable.h>
#include <linux/kdev_t.h>
#include <linux/sched.h>
void rust_helper_BUG(void)
{
	BUG();
}

unsigned long rust_helper_copy_from_user(void *to, const void __user *from, unsigned long n)
{
	return copy_from_user(to, from, n);
}

unsigned long rust_helper_copy_to_user(void __user *to, const void *from, unsigned long n)
{
	return copy_to_user(to, from, n);
}

unsigned long rust_helper_clear_user(void __user *to, unsigned long n)
{
	return clear_user(to, n);
}

void rust_helper_spin_lock_init(spinlock_t *lock, const char *name,
				struct lock_class_key *key)
{
#ifdef CONFIG_DEBUG_SPINLOCK
	__spin_lock_init(lock, name, key);
#else
	spin_lock_init(lock);
#endif
}
EXPORT_SYMBOL_GPL(rust_helper_spin_lock_init);

void rust_helper_spin_lock(spinlock_t *lock)
{
	spin_lock(lock);
}
EXPORT_SYMBOL_GPL(rust_helper_spin_lock);

void rust_helper_spin_unlock(spinlock_t *lock)
{
	spin_unlock(lock);
}
EXPORT_SYMBOL_GPL(rust_helper_spin_unlock);

void rust_helper_init_wait(struct wait_queue_entry *wq_entry)
{
	init_wait(wq_entry);
}
EXPORT_SYMBOL_GPL(rust_helper_init_wait);

void rust_helper_add_wait_queue(struct wait_queue_head *wq_head, struct wait_queue_entry *wq_entry)
{
	struct list_head *head = &wq_head->head;
	struct wait_queue_entry *wq;
	list_for_each_entry(wq, &wq_head->head, entry) {
		if (!(wq->flags & (0x20)))
			break;
		head = &wq->entry;
	}
	list_add(&wq_entry->entry, head);
}
EXPORT_SYMBOL_GPL(rust_helper_add_wait_queue);

void rust_helper_set_current_state(long state_value)
{
	smp_store_mb(current->state, (state_value));
}
EXPORT_SYMBOL_GPL(rust_helper_set_current_state);

int rust_helper_signal_pending(struct task_struct *t)
{
	return signal_pending(t);
}
EXPORT_SYMBOL_GPL(rust_helper_signal_pending);

int rust_helper_wait_event_interruptible(struct wait_queue_head *wq_head, bool condition)
{
	return wait_event_interruptible(*wq_head, condition);
}
EXPORT_SYMBOL_GPL(rust_helper_wait_event_interruptible);

bool rust_helper_wq_has_sleeper(struct wait_queue_head *wq_head)
{
	return wq_has_sleeper(wq_head);
}
EXPORT_SYMBOL_GPL(rust_helper_wq_has_sleeper);

struct page *rust_helper_alloc_pages(gfp_t gfp_mask, unsigned int order)
{
	return alloc_pages(gfp_mask, order);
}
EXPORT_SYMBOL_GPL(rust_helper_alloc_pages);

void *rust_helper_kmap(struct page *page)
{
	return kmap(page);
}
EXPORT_SYMBOL_GPL(rust_helper_kmap);

void rust_helper_kunmap(struct page *page)
{
	return kunmap(page);
}
EXPORT_SYMBOL_GPL(rust_helper_kunmap);

int rust_helper_cond_resched(void)
{
	return cond_resched();
}
EXPORT_SYMBOL_GPL(rust_helper_cond_resched);

size_t rust_helper_copy_from_iter(void *addr, size_t bytes, struct iov_iter *i)
{
	return copy_from_iter(addr, bytes, i);
}
EXPORT_SYMBOL_GPL(rust_helper_copy_from_iter);

size_t rust_helper_copy_to_iter(const void *addr, size_t bytes, struct iov_iter *i)
{
	return copy_to_iter(addr, bytes, i);
}
EXPORT_SYMBOL_GPL(rust_helper_copy_to_iter);

bool rust_helper_is_err(__force const void *ptr)
{
	return IS_ERR(ptr);
}
EXPORT_SYMBOL_GPL(rust_helper_is_err);

long rust_helper_ptr_err(__force const void *ptr)
{
	return PTR_ERR(ptr);
}
EXPORT_SYMBOL_GPL(rust_helper_ptr_err);

const char *rust_helper_errname(int err)
{
	return errname(err);
}

void rust_helper_mutex_lock(struct mutex *lock)
{
	mutex_lock(lock);
}
EXPORT_SYMBOL_GPL(rust_helper_mutex_lock);

void *
rust_helper_platform_get_drvdata(const struct platform_device *pdev)
{
	return platform_get_drvdata(pdev);
}
EXPORT_SYMBOL_GPL(rust_helper_platform_get_drvdata);

void
rust_helper_platform_set_drvdata(struct platform_device *pdev,
				 void *data)
{
	return platform_set_drvdata(pdev, data);
}
EXPORT_SYMBOL_GPL(rust_helper_platform_set_drvdata);

refcount_t rust_helper_refcount_new(void)
{
	return (refcount_t)REFCOUNT_INIT(1);
}
EXPORT_SYMBOL_GPL(rust_helper_refcount_new);

void rust_helper_refcount_inc(refcount_t *r)
{
	refcount_inc(r);
}
EXPORT_SYMBOL_GPL(rust_helper_refcount_inc);

bool rust_helper_refcount_dec_and_test(refcount_t *r)
{
	return refcount_dec_and_test(r);
}
EXPORT_SYMBOL_GPL(rust_helper_refcount_dec_and_test);

void rust_helper_rb_link_node(struct rb_node *node, struct rb_node *parent,
			      struct rb_node **rb_link)
{
	rb_link_node(node, parent, rb_link);
}
EXPORT_SYMBOL_GPL(rust_helper_rb_link_node);

struct task_struct *rust_helper_get_current(void)
{
	return current;
}
EXPORT_SYMBOL_GPL(rust_helper_get_current);

void rust_helper_get_task_struct(struct task_struct * t)
{
	get_task_struct(t);
}
EXPORT_SYMBOL_GPL(rust_helper_get_task_struct);

void rust_helper_put_task_struct(struct task_struct * t)
{
	put_task_struct(t);
}
EXPORT_SYMBOL_GPL(rust_helper_put_task_struct);

int rust_helper_security_binder_set_context_mgr(struct task_struct *mgr)
{
	return security_binder_set_context_mgr(mgr);
}
EXPORT_SYMBOL_GPL(rust_helper_security_binder_set_context_mgr);

int rust_helper_security_binder_transaction(struct task_struct *from,
					    struct task_struct *to)
{
	return security_binder_transaction(from, to);
}
EXPORT_SYMBOL_GPL(rust_helper_security_binder_transaction);

int rust_helper_security_binder_transfer_binder(struct task_struct *from,
						struct task_struct *to)
{
	return security_binder_transfer_binder(from, to);
}
EXPORT_SYMBOL_GPL(rust_helper_security_binder_transfer_binder);

int rust_helper_security_binder_transfer_file(struct task_struct *from,
					      struct task_struct *to,
					      struct file *file)
{
	return security_binder_transfer_file(from, to, file);
}
EXPORT_SYMBOL_GPL(rust_helper_security_binder_transfer_file);

int rust_helper_cpulist_parse(const char *buf, struct cpumask *dstp)
{
	return cpulist_parse(buf, dstp);
}
EXPORT_SYMBOL_GPL(rust_helper_cpulist_parse); 

void rust_helper_cpumask_copy(struct cpumask *dstp,
				const struct cpumask *srcp)
{
	cpumask_copy(dstp, srcp);
}
EXPORT_SYMBOL_GPL(rust_helper_cpumask_copy); 

void rust_helper_cpumask_clear(struct cpumask *ptr) {
	cpumask_clear(ptr);
}
EXPORT_SYMBOL_GPL(rust_helper_cpumask_clear);

unsigned long rust_helper_page_align(unsigned long size)
{
	return PAGE_ALIGN(size);
}
EXPORT_SYMBOL_GPL(rust_helper_page_align); 

int rust_helper_page_aligned(unsigned long size)
{
	return PAGE_ALIGNED(size);
}
EXPORT_SYMBOL_GPL(rust_helper_page_aligned); 

size_t rust_helper_align(size_t x, unsigned long a)
{
	return ALIGN(x,a);
}
EXPORT_SYMBOL_GPL(rust_helper_align); 

bool rust_helper_running_inband(void)
{
	return running_inband();
}
EXPORT_SYMBOL_GPL(rust_helper_running_inband); 

void* rust_helper_kzalloc(size_t size, gfp_t flags)
{
	return kzalloc(size, flags);
}
EXPORT_SYMBOL_GPL(rust_helper_kzalloc); 

void* rust_helper_per_cpu_ptr(void* var, int cpu)
{
 	return per_cpu_ptr(var, cpu);	
}
EXPORT_SYMBOL_GPL(rust_helper_per_cpu_ptr); 

void* rust_helper_raw_cpu_ptr(void* var)
{
 	return raw_cpu_ptr(var);	
}
EXPORT_SYMBOL_GPL(rust_helper_raw_cpu_ptr); 

int rust_helper_smp_processor_id(void* var)
{
 	return smp_processor_id();	
}
EXPORT_SYMBOL_GPL(rust_helper_smp_processor_id); 

refcount_t rust_helper_REFCOUNT_INIT(int n)
{
	return (refcount_t)REFCOUNT_INIT(n);
}
EXPORT_SYMBOL_GPL(rust_helper_REFCOUNT_INIT);

struct class * rust_helper_class_create(struct module *this_moduel, const char* name)
{
	struct class *res = class_create(this_moduel, name);
	return res;
}
EXPORT_SYMBOL_GPL(rust_helper_class_create);

const char * rust_helper_dev_name(const struct device *dev)
{
	return dev_name(dev);
}
EXPORT_SYMBOL_GPL(rust_helper_dev_name);

void rust_helper_hash_init(struct hlist_head *ht, unsigned int sz)
{
	__hash_init(ht, sz);
}
EXPORT_SYMBOL_GPL(rust_helper_hash_init);

// const char * rust_helper_DEVICE_ATTR_RW(const struct device *dev)
// {
// 	return dev_name(dev);
// }
// EXPORT_SYMBOL_GPL(rust_helper_dev_name);

void rust_helper_INIT_LIST_HEAD(struct list_head *list)
{
	INIT_LIST_HEAD(list);
}
EXPORT_SYMBOL_GPL(rust_helper_INIT_LIST_HEAD);

void rust_helper_list_del(struct list_head *list)
{
	list_del(list);
}
EXPORT_SYMBOL_GPL(rust_helper_list_del);

void rust_helper_list_del_init(struct list_head *list)
{
	list_del_init(list);
}
EXPORT_SYMBOL_GPL(rust_helper_list_del_init);

const struct cpumask* rust_helper_cpumask_of(int cpu)
{
	return cpumask_of(cpu);
}
EXPORT_SYMBOL_GPL(rust_helper_cpumask_of);

int rust_helper_cpumask_and(struct cpumask *dstp,
			       const struct cpumask *src1p,
			       const struct cpumask *src2p)
{
	return cpumask_and(dstp, src1p, src2p);
}
EXPORT_SYMBOL_GPL(rust_helper_cpumask_and);

bool rust_helper_cpumask_empty(const struct cpumask *srcp)
{
	return cpumask_empty(srcp);
}
EXPORT_SYMBOL_GPL(rust_helper_cpumask_empty);

int rust_helper_cpumask_first(const struct cpumask *srcp)
{
	return cpumask_first(srcp);
}
EXPORT_SYMBOL_GPL(rust_helper_cpumask_first);

void rust_helper_list_add(struct list_head *new,struct list_head *head)
{
	list_add(new,head);
}
EXPORT_SYMBOL_GPL(rust_helper_list_add);

void rust_helper_list_add_tail(struct list_head *new,struct list_head *head)
{
	list_add_tail(new,head);
}
EXPORT_SYMBOL_GPL(rust_helper_list_add_tail);

void rust_helper_rcu_read_lock(void)
{
	rcu_read_lock();
}
EXPORT_SYMBOL_GPL(rust_helper_rcu_read_lock);

void rust_helper_rcu_read_unlock(void)
{
	rcu_read_unlock();
}
EXPORT_SYMBOL_GPL(rust_helper_rcu_read_unlock);

void rust_helper_synchronize_rcu(void)
{
	synchronize_rcu();
}
EXPORT_SYMBOL_GPL(rust_helper_synchronize_rcu);

void rust_helper_cpus_read_lock(void) {
	cpus_read_lock();
}
EXPORT_SYMBOL_GPL(rust_helper_cpus_read_lock);

void rust_helper_cpus_read_unlock(void) {
	cpus_read_unlock();
}
EXPORT_SYMBOL_GPL(rust_helper_cpus_read_unlock);

void* rust_helper_kthread_run(int (*threadfn)(void *data), void *data, const char namefmt[], ...)
{
	va_list args;
	va_start(args, namefmt);
	
	return kthread_run(threadfn, data, namefmt, args);
}
EXPORT_SYMBOL_GPL(rust_helper_kthread_run);

// void rust_helper_DEFINE_PER_CPU(,const char* name)
// {
// 	DEFINE_PER_CPU
// }
// EXPORT_SYMBOL_GPL(rust_helper_DEFINE_PER_CPU);

void rust_helper__this_cpu_write(struct clock_proxy_device *pcp, struct clock_proxy_device *val)
{
	__this_cpu_write(pcp, val);
}
EXPORT_SYMBOL_GPL(rust_helper__this_cpu_write);

struct clock_proxy_device* rust_helper__this_cpu_read(struct clock_proxy_device *pcp)
{
	return __this_cpu_read(pcp);
}
EXPORT_SYMBOL_GPL(rust_helper__this_cpu_read);

void rust_helper_atomic_set(atomic_t *v, int i)
{
	atomic_set(v, i);
}
EXPORT_SYMBOL_GPL(rust_helper_atomic_set);

void rust_helper_atomic_inc(atomic_t *v)
{
	atomic_inc(v);
}
EXPORT_SYMBOL_GPL(rust_helper_atomic_inc);

bool rust_helper_atomic_dec_and_test(atomic_t *v)
{
	return atomic_dec_and_test(v);
}
EXPORT_SYMBOL_GPL(rust_helper_atomic_dec_and_test);

int rust_helper_atomic_dec_return(atomic_t *v)
{
	return atomic_dec_return(v);
}
EXPORT_SYMBOL_GPL(rust_helper_atomic_dec_return);

int rust_helper_atomic_read(atomic_t *v)
{
	return atomic_read(v);
}
EXPORT_SYMBOL_GPL(rust_helper_atomic_read);

void rust_helper_atomic_add(int i, atomic_t *v)
{
	return atomic_add(i, v);
}
EXPORT_SYMBOL_GPL(rust_helper_atomic_add);

void rust_helper_atomic_sub(int i, atomic_t *v)
{
	return atomic_sub(i, v);
}
EXPORT_SYMBOL_GPL(rust_helper_atomic_sub);

int rust_helper_atomic_sub_return(int i, atomic_t *v)
{
	return atomic_sub_return(i, v);
}
EXPORT_SYMBOL_GPL(rust_helper_atomic_sub_return);

int rust_helper_atomic_cmpxchg(atomic_t *v, int old, int new)
{
	return atomic_cmpxchg(v, old, new);
}
EXPORT_SYMBOL_GPL(rust_helper_atomic_cmpxchg);

int rust_helper_atomic_add_return(int i, atomic_t *v)
{
	return atomic_add_return(i, v);
}
EXPORT_SYMBOL_GPL(rust_helper_atomic_add_return);

void rust_helper_init_irq_work(struct irq_work *work, void (*func)(struct irq_work *))
{
	init_irq_work(work, func);
}
EXPORT_SYMBOL_GPL(rust_helper_init_irq_work);

ktime_t rust_helper_ktime_sub(ktime_t lhs,ktime_t rhs)
{
	return ktime_sub(lhs, rhs);
}
EXPORT_SYMBOL_GPL(rust_helper_ktime_sub);

void rust_helper_check_inband_stage(void)
{
	check_inband_stage();
}
EXPORT_SYMBOL_GPL(rust_helper_check_inband_stage);

int rust_helper_proxy_set(ktime_t expires,
				struct clock_event_device *dev)
{
	struct clock_event_device *real_dev = container_of(dev, struct clock_proxy_device, proxy_device)->real_device;
	unsigned long flags;
	int ret;

	flags = hard_local_irq_save();
	ret = real_dev->set_next_ktime(1000000, real_dev);
	hard_local_irq_restore(flags);

	return ret;
}
EXPORT_SYMBOL_GPL(rust_helper_proxy_set);

int rust_helper_proxy_set_next_ktime(ktime_t expires,
				struct clock_event_device *dev)
{
	struct clock_event_device *real_dev = container_of(dev, struct clock_proxy_device, proxy_device)->real_device;
	unsigned long flags;
	int ret;

	flags = hard_local_irq_save();
	ret = real_dev->set_next_ktime(expires, real_dev);
	hard_local_irq_restore(flags);

	return ret;
}
EXPORT_SYMBOL_GPL(rust_helper_proxy_set_next_ktime);

unsigned int rust_helper_hard_local_irq_save(void) {
	return hard_local_irq_save();
}
EXPORT_SYMBOL_GPL(rust_helper_hard_local_irq_save);

void rust_helper_hard_local_irq_restore(unsigned int flags) {
	return hard_local_irq_restore(flags);
}
EXPORT_SYMBOL_GPL(rust_helper_hard_local_irq_restore);

void rust_helper_hard_local_irq_enable(void) {
	hard_local_irq_enable();
}
EXPORT_SYMBOL_GPL(rust_helper_hard_local_irq_enable);

void rust_helper_hard_local_irq_disable(void) {
	hard_local_irq_disable();
}
EXPORT_SYMBOL_GPL(rust_helper_hard_local_irq_disable);

// void rust_helper_arch_local_irq_enable(void) {
// 	native_irq_enable();
// }
// EXPORT_SYMBOL_GPL(rust_helper_arch_local_irq_enable);

void rust_helper_tick_notify_proxy(void)
{
	tick_notify_proxy();
}
EXPORT_SYMBOL_GPL(rust_helper_tick_notify_proxy);

ktime_t rust_helper_ktime_add_ns(ktime_t kt, u64 nsval) {
	return ktime_add_ns(kt,nsval);
}
EXPORT_SYMBOL_GPL(rust_helper_ktime_add_ns);

ktime_t rust_helper_ktime_add(ktime_t kt, ktime_t nsval) {
	return ktime_add(kt,nsval);
}
EXPORT_SYMBOL_GPL(rust_helper_ktime_add);

ktime_t rust_helper_ktime_compare(ktime_t cmp1, ktime_t cmp2) {
	return ktime_compare(cmp1,cmp2);
}
EXPORT_SYMBOL_GPL(rust_helper_ktime_compare);

ktime_t rust_helper_ktime_set(const s64 secs, const unsigned long nsecs) {
	return ktime_set(secs,nsecs);
}
EXPORT_SYMBOL_GPL(rust_helper_ktime_set);

s64 rust_helper_timespec64_to_ktime(struct timespec64 ts) {
	return ktime_set(ts.tv_sec, ts.tv_nsec);
}
EXPORT_SYMBOL_GPL(rust_helper_timespec64_to_ktime);

s64 rust_helper_ktime_divns(const ktime_t kt, s64 div) {
	return ktime_divns(kt,div);
}
EXPORT_SYMBOL_GPL(rust_helper_ktime_divns);

void rust_helper_irq_send_oob_ipi(unsigned int ipi,
		const struct cpumask *cpumask) {
	irq_send_oob_ipi(ipi, cpumask);
}
EXPORT_SYMBOL_GPL(rust_helper_irq_send_oob_ipi);

unsigned int rust_helper_irq_get_TIMER_OOB_IPI(void) {
	return TIMER_OOB_IPI;
}
EXPORT_SYMBOL_GPL(rust_helper_irq_get_TIMER_OOB_IPI);


int rust_helper_test_and_set_bit(unsigned int bit, volatile unsigned long *p) {
	return test_and_set_bit(bit, p);
}
EXPORT_SYMBOL_GPL(rust_helper_test_and_set_bit);

unsigned int rust_helper_num_possible_cpus(void) {
	return num_possible_cpus();
}
EXPORT_SYMBOL_GPL(rust_helper_num_possible_cpus);

unsigned long rust_helper_IRQF_OOB(void) {
	return IRQF_OOB;
}
EXPORT_SYMBOL_GPL(rust_helper_IRQF_OOB);

void rust_helper_dovetail_send_mayday(struct task_struct *castaway){
	dovetail_send_mayday(castaway);
}
EXPORT_SYMBOL_GPL(rust_helper_dovetail_send_mayday);

struct oob_thread_state *rust_helper_dovetail_current_state(void) {
	return dovetail_current_state();
}
EXPORT_SYMBOL_GPL(rust_helper_dovetail_current_state);

bool rust_helper_test_bit(long nr, const volatile unsigned long *addr) {
	return test_bit(nr,addr);
}
EXPORT_SYMBOL_GPL(rust_helper_test_bit);

void rust_helper_dev_hold(struct net_device *dev)
{
	return dev_hold(dev);
}
EXPORT_SYMBOL_GPL(rust_helper_dev_hold);

void rust_helper_dev_put(struct net_device *dev)
{
	return dev_put(dev);
}
EXPORT_SYMBOL_GPL(rust_helper_dev_put);

struct net *rust_helper_get_net(struct net *net)
{
	return get_net(net);
}
EXPORT_SYMBOL_GPL(rust_helper_get_net);

void rust_helper_put_net(struct net *net)
{
	return put_net(net);
}
EXPORT_SYMBOL_GPL(rust_helper_put_net);


void rust_helper_dovetail_request_ucall(struct task_struct *task) {
	dovetail_request_ucall(task);
}
EXPORT_SYMBOL_GPL(rust_helper_dovetail_request_ucall);

void rust_helper_init_completion(struct completion *x) {
	init_completion(x);
}
EXPORT_SYMBOL_GPL(rust_helper_init_completion);

void rust_helper_unstall_oob(void) {
	unstall_oob();
}
EXPORT_SYMBOL_GPL(rust_helper_unstall_oob);

void rust_helper_stall_oob(void) {
	stall_oob();
}
EXPORT_SYMBOL_GPL(rust_helper_stall_oob);

void rust_helper_preempt_disable(void) {
	preempt_disable();
}
EXPORT_SYMBOL_GPL(rust_helper_preempt_disable);

void rust_helper_dovetail_leave_oob(void) {
	dovetail_leave_oob();
}
EXPORT_SYMBOL_GPL(rust_helper_dovetail_leave_oob);


void rust_helper_hard_spin_lock(struct raw_spinlock *rlock) {
	hard_spin_lock(rlock);
}
EXPORT_SYMBOL_GPL(rust_helper_hard_spin_lock);

void rust_helper_hard_spin_unlock(struct raw_spinlock *rlock) {
	hard_spin_unlock(rlock);
}
EXPORT_SYMBOL_GPL(rust_helper_hard_spin_unlock);

void rust_helper_raw_spin_lock(hard_spinlock_t *lock) {
	raw_spin_lock(lock);
}
EXPORT_SYMBOL_GPL(rust_helper_raw_spin_lock);

void rust_helper_raw_spin_lock_nested(hard_spinlock_t *lock, unsigned int depth) {
	raw_spin_lock_nested(lock, depth);
}
EXPORT_SYMBOL_GPL(rust_helper_raw_spin_lock_nested);

unsigned int rust_helper_task_cpu(const struct task_struct *p) {
	return task_cpu(p);
}
EXPORT_SYMBOL_GPL(rust_helper_task_cpu);

unsigned int rust_helper_irq_get_RESCHEDULE_OOB_IPI(void) {
	return RESCHEDULE_OOB_IPI;
}
EXPORT_SYMBOL_GPL(rust_helper_irq_get_RESCHEDULE_OOB_IPI);

void rust_helper_raw_spin_unlock(hard_spinlock_t *lock) {
	raw_spin_unlock(lock);
}
EXPORT_SYMBOL_GPL(rust_helper_raw_spin_unlock);

inline int rust_helper_ilog2(size_t size) {
	return ilog2(size);
}
EXPORT_SYMBOL_GPL(rust_helper_ilog2);

int rust_helper_ffs(unsigned long x) {
	return ffs(x);
}
EXPORT_SYMBOL_GPL(rust_helper_ffs);


kernel_cap_t rust_helper_current_cap(void) {
	return current_cap();
}
EXPORT_SYMBOL_GPL(rust_helper_current_cap);

int rust_helper_cap_raised(kernel_cap_t c, int flag) {
	return cap_raised(c, flag);
}
EXPORT_SYMBOL_GPL(rust_helper_cap_raised);

struct timespec64 rust_helper_ktime_to_timespec64(ktime_t kt) {
	return ktime_to_timespec64(kt);
}
EXPORT_SYMBOL_GPL(rust_helper_ktime_to_timespec64);

int rust_helper_cpumask_test_cpu(int cpu, const struct cpumask *cpumask) {
	return cpumask_test_cpu(cpu, cpumask);
}
EXPORT_SYMBOL_GPL(rust_helper_cpumask_test_cpu);

unsigned long rust_helper_raw_spin_lock_irqsave(hard_spinlock_t *lock) {
	unsigned long flags;
	raw_spin_lock_irqsave(lock, flags);
	return flags;
}
EXPORT_SYMBOL_GPL(rust_helper_raw_spin_lock_irqsave);

void rust_helper_cap_raise(kernel_cap_t *c, int flag) {
	cap_raise(*c, flag);
}
EXPORT_SYMBOL_GPL(rust_helper_cap_raise);

struct oob_mm_state* rust_helper_dovetail_mm_state(void) {
	return dovetail_mm_state();
}
EXPORT_SYMBOL_GPL(rust_helper_dovetail_mm_state);

int rust_helper_put_user(int i, int *addr) {
	return put_user(i, addr);
}
EXPORT_SYMBOL_GPL(rust_helper_put_user);

void rust_helper_smp_wmb(void) {
	smp_wmb();
}
EXPORT_SYMBOL_GPL(rust_helper_smp_wmb);

void rust_helper_raw_spin_unlock_irqrestore(hard_spinlock_t *lock, unsigned long flags) {
	raw_spin_unlock_irqrestore(lock, flags);
}
EXPORT_SYMBOL_GPL(rust_helper_raw_spin_unlock_irqrestore);

void rust_helper_cpumask_set_cpu(unsigned int cpu, struct cpumask *dstp) {
	cpumask_set_cpu(cpu,dstp);
}
EXPORT_SYMBOL_GPL(rust_helper_cpumask_set_cpu);

void rust_helper_preempt_enable(void) {
	preempt_enable();
}
EXPORT_SYMBOL_GPL(rust_helper_preempt_enable);

struct net * rust_helper_sock_net(const struct sock *sk) {
	return sock_net(sk);
}
EXPORT_SYMBOL_GPL(rust_helper_sock_net);

u32 rust_helper_jhash(const void *key, u32 length, u32 initval) {
	return jhash(key,length,initval);
}
EXPORT_SYMBOL_GPL(rust_helper_jhash);

u32 rust_helper_jhash2(const void *key, u32 length, u32 initval) {
	return jhash2(key,length,initval);
}
EXPORT_SYMBOL_GPL(rust_helper_jhash2);


void rust_helper_local_bh_disable (void) {
	local_bh_disable();
}
EXPORT_SYMBOL_GPL(rust_helper_local_bh_disable);

void rust_helper_local_bh_enable (void) {
	local_bh_enable();
}
EXPORT_SYMBOL_GPL(rust_helper_local_bh_enable);

void rust_helper_skb_list_del_init(struct sk_buff *skb) {
	skb_list_del_init(skb);
}
EXPORT_SYMBOL_GPL(rust_helper_skb_list_del_init);

void rust_helper_list_splice_init(struct list_head *list,struct list_head *head) {
	list_splice_init(list,head);
}
EXPORT_SYMBOL_GPL(rust_helper_list_splice_init);

bool rust_helper_list_empty(struct list_head *head) {
	return list_empty(head);
}
EXPORT_SYMBOL_GPL(rust_helper_list_empty);

void rust_helper_spin_unlock_irqrestore(spinlock_t *lock, unsigned long flags)
{
	spin_unlock_irqrestore(lock, flags);
}
EXPORT_SYMBOL_GPL(rust_helper_spin_unlock_irqrestore);

void *rust_helper_ERR_PTR(long error)
{
	return ERR_PTR(error);
}
EXPORT_SYMBOL_GPL(rust_helper_ERR_PTR);

u_int64_t rust_helper_BITS_TO_LONGS(int nr) {
	return BITS_TO_LONGS(nr);
}
EXPORT_SYMBOL_GPL(rust_helper_BITS_TO_LONGS);

void* rust_helper_this_cpu_ptr(void* ptr) {
	return this_cpu_ptr(ptr);
}
EXPORT_SYMBOL_GPL(rust_helper_this_cpu_ptr);

__be16 rust_helper_vlan_dev_vlan_proto(const struct net_device *dev) {
	return vlan_dev_vlan_proto(dev);
}
EXPORT_SYMBOL_GPL(rust_helper_vlan_dev_vlan_proto);
__u16 rust_helper_vlan_dev_vlan_id(const struct net_device *dev) {
	return vlan_dev_vlan_id(dev);
}
EXPORT_SYMBOL_GPL(rust_helper_vlan_dev_vlan_id);

struct net_device* rust_helper_vlan_dev_real_dev(const struct net_device *dev) {
	return vlan_dev_real_dev(dev);
}
EXPORT_SYMBOL_GPL(rust_helper_vlan_dev_real_dev);

__u16 rust_helper_vlan_dev_get_egress_qos_mask(struct net_device *dev,u32 skprio) {
	return vlan_dev_get_egress_qos_mask(dev,skprio);
}
EXPORT_SYMBOL_GPL(rust_helper_vlan_dev_get_egress_qos_mask);

bool rust_helper_is_vlan_dev(const struct net_device *dev) {
	return is_vlan_dev(dev);
}
EXPORT_SYMBOL_GPL(rust_helper_is_vlan_dev);

// void rust_helper_hash_del(struct hlist_node *n){
// 	hash_del_init(n);
// }
// EXPORT_SYMBOL_GPL(rust_helper_hash_del);

void rust_helper_hash_add(struct hlist_head *hashtable,size_t length,struct hlist_node *node,u32 key)
{
	hlist_add_head(node, &hashtable[hash_min(key, ilog2(length))]);
}
EXPORT_SYMBOL_GPL(rust_helper_hash_add);

void rust_helper_hash_del(struct hlist_node* node) {
	hlist_del_init(node);
}
EXPORT_SYMBOL_GPL(rust_helper_hash_del);

struct hlist_head* rust_helper_get_hlist_head(struct hlist_head *hashtable,size_t length,u32 key) {
	return &hashtable[hash_min(key, ilog2(length))];
}
EXPORT_SYMBOL_GPL(rust_helper_get_hlist_head);

__be16 rust_helper_htons(__u16 x) {
	return htons(x);
}
EXPORT_SYMBOL_GPL(rust_helper_htons);

__u16 rust_helper_ntohs(__be16 x) {
	return ntohs(x);
}
EXPORT_SYMBOL_GPL(rust_helper_ntohs);

void rust_helper_raw_spin_lock_init(hard_spinlock_t *lock) {
	raw_spin_lock_init(lock);
}
EXPORT_SYMBOL_GPL(rust_helper_raw_spin_lock_init);

bool rust_helper_hard_irqs_disabled(void) {
	return native_irqs_disabled();
}
EXPORT_SYMBOL_GPL(rust_helper_hard_irqs_disabled);

bool rust_helper_rros_enable_preempt_top_part(void) {
	return(--dovetail_current_state()->preempt_count == 0);
}
EXPORT_SYMBOL_GPL(rust_helper_rros_enable_preempt_top_part);

void rust_helper_rros_disable_preempt(void) {
	dovetail_current_state()->preempt_count++;
}
EXPORT_SYMBOL_GPL(rust_helper_rros_disable_preempt);

unsigned int rust_helper_raw_get_user(unsigned int *x,unsigned int* ptr) {
	unsigned int tmp;
	unsigned ret = __get_user(tmp,ptr);
	*x = tmp;
	return ret;
}
EXPORT_SYMBOL_GPL(rust_helper_raw_get_user);

unsigned int rust_helper_raw_get_user_64(unsigned long *x,unsigned long* ptr) {
	unsigned long tmp;
	unsigned ret = __get_user(tmp,ptr);
	*x = tmp;
	return ret;
}
EXPORT_SYMBOL_GPL(rust_helper_raw_get_user_64);

int rust_helper_raw_put_user(unsigned int x,unsigned int* ptr) {
	return __put_user(x,ptr);
}
EXPORT_SYMBOL_GPL(rust_helper_raw_put_user);

unsigned long rust_helper_raw_copy_from_user(void* dst,const void*src,unsigned long size) {
	return raw_copy_from_user(dst,src,size);
}
EXPORT_SYMBOL_GPL(rust_helper_raw_copy_from_user);

unsigned long rust_helper_raw_copy_to_user(void* dst,const void*src,unsigned long size) {
	return raw_copy_to_user(dst,src,size);
}
EXPORT_SYMBOL_GPL(rust_helper_raw_copy_to_user);

void rust_helper_bitmap_copy(unsigned long *dst, const unsigned long *src,unsigned int nbits) {
	bitmap_copy(dst,src,nbits);
}
EXPORT_SYMBOL_GPL(rust_helper_bitmap_copy);

void rust_helper_init_work(struct work_struct*work,void (*rust_helper_work_func)(struct work_struct *work)) {
	INIT_WORK(work,rust_helper_work_func);
}
EXPORT_SYMBOL_GPL(rust_helper_init_work);

//NOTE: rust_helper for stax
unsigned long rust_helper_spin_lock_irqsave(spinlock_t *lock) {
	unsigned long flags;
	spin_lock_irqsave(lock, flags);
	return flags;
}
EXPORT_SYMBOL_GPL(rust_helper_spin_lock_irqsave);

bool rust_helper_waitqueue_active(struct wait_queue_head *wq_head) {
	return !!waitqueue_active(wq_head);
}
EXPORT_SYMBOL_GPL(rust_helper_waitqueue_active);

void rust_helper_init_waitqueue_head(struct wait_queue_head *wq_head) {
	init_waitqueue_head(wq_head);
}
EXPORT_SYMBOL_GPL(rust_helper_init_waitqueue_head);

#ifdef CONFIG_NET_OOB
void rust_helper__vlan_hwaccel_put_tag(struct sk_buff *skb, __be16 vlan_proto, __u16 vlan_tci) {
	__vlan_hwaccel_put_tag(skb,vlan_proto,vlan_tci);
}
EXPORT_SYMBOL_GPL(rust_helper__vlan_hwaccel_put_tag);

int rust_helper__vlan_hwaccel_get_tag(struct sk_buff *skb, __u16* vlan_tci) {
	return __vlan_hwaccel_get_tag(skb,vlan_tci);
}
EXPORT_SYMBOL_GPL(rust_helper__vlan_hwaccel_get_tag);

void rust_helper_netdev_is_oob_capable(struct net_device *dev) {
	netdev_is_oob_capable(dev);
}
EXPORT_SYMBOL_GPL(rust_helper_netdev_is_oob_capable);

struct sk_buff* rust_helper_netdev_alloc_oob_skb(struct net_device *dev,dma_addr_t *dma_addr) {
	return netdev_alloc_oob_skb(dev,dma_addr);
}
EXPORT_SYMBOL_GPL(rust_helper_netdev_alloc_oob_skb);

void rust_helper_set_bit(int nr, volatile unsigned long *addr) {
	set_bit(nr,addr);
}
EXPORT_SYMBOL_GPL(rust_helper_set_bit);

void rust_helper_clear_bit(int nr, volatile unsigned long *addr) {
	clear_bit(nr,addr);
}
EXPORT_SYMBOL_GPL(rust_helper_clear_bit);

struct net_device *
rust_helper_netdev_notifier_info_to_dev(const struct netdev_notifier_info *info) {
	return info->dev;
}
EXPORT_SYMBOL_GPL(rust_helper_netdev_notifier_info_to_dev);

struct net*
rust_helper_dev_net(const struct net_device *dev) {
	return dev_net(dev);
}
EXPORT_SYMBOL_GPL(rust_helper_dev_net);

void rust_helper_dev_kfree_skb(struct sk_buff *skb) {
	dev_kfree_skb(skb);
}
EXPORT_SYMBOL_GPL(rust_helper_dev_kfree_skb);

__u16 rust_helper_skb_vlan_tag_get_id(const struct sk_buff *skb) {
	return skb_vlan_tag_get_id(skb);
}
EXPORT_SYMBOL_GPL(rust_helper_skb_vlan_tag_get_id);

void rust_helper_skb_morph_oob_skb(struct sk_buff *n, struct sk_buff *skb) {
	skb_morph_oob_skb(n,skb);
}
EXPORT_SYMBOL_GPL(rust_helper_skb_morph_oob_skb);

unsigned char * rust_helper_skb_mac_header(struct sk_buff *skb) {
	return skb_mac_header(skb);
}
EXPORT_SYMBOL_GPL(rust_helper_skb_mac_header);

#include<linux/netdevice.h>
int rust_helper_dev_parse_header(const struct sk_buff *skb,unsigned char *haddr) {
	return dev_parse_header(skb,haddr);
}
EXPORT_SYMBOL_GPL(rust_helper_dev_parse_header);

bool rust_helper_eth_type_vlan(__be16 ethertype) {
	return eth_type_vlan(ethertype);
}
EXPORT_SYMBOL_GPL(rust_helper_eth_type_vlan);

void rust_helper_skb_reset_mac_header(struct sk_buff *skb) {
	skb_reset_mac_header(skb);
}
EXPORT_SYMBOL_GPL(rust_helper_skb_reset_mac_header);

int rust_helper_skb_tailroom(const struct sk_buff *skb) {
	return skb_tailroom(skb);
}
EXPORT_SYMBOL_GPL(rust_helper_skb_tailroom);

bool rust_helper_dev_validate_header(const struct net_device* dev,char* ll_header,int len) {
	return dev_validate_header(dev,ll_header,len);
}
EXPORT_SYMBOL_GPL(rust_helper_dev_validate_header);

__be16 rust_helper_dev_parse_header_protocol(const struct sk_buff *skb) {
	return dev_parse_header_protocol(skb);
}
EXPORT_SYMBOL_GPL(rust_helper_dev_parse_header_protocol);

void rust_helper_skb_set_network_header(struct sk_buff *skb,int offset) {
	skb_set_network_header(skb,offset);
}
EXPORT_SYMBOL_GPL(rust_helper_skb_set_network_header);

void rust_helper___vlan_hwaccel_put_tag(struct sk_buff *skb, __be16 vlan_proto, __u16 vlan_tci) {
	__vlan_hwaccel_put_tag(skb,vlan_proto,vlan_tci);
}
EXPORT_SYMBOL_GPL(rust_helper___vlan_hwaccel_put_tag);

void* rust_helper_kthread_run_on_cpu(int (*threadfn)(void *data), void *data, int cpu, const char namefmt[], ...)
{
	va_list args;
	va_start(args, namefmt);
	
	return kthread_run_on_cpu_new(threadfn, data, cpu, namefmt, args);
}
EXPORT_SYMBOL_GPL(rust_helper_kthread_run_on_cpu);

unsigned long rust_helper_pa(unsigned long x) {
	return __virt_to_phys(x);
}
EXPORT_SYMBOL_GPL(rust_helper_pa);

void rust_helper_schedule_work(struct work_struct*work){
	schedule_work(work);
}
EXPORT_SYMBOL_GPL(rust_helper_schedule_work);

unsigned int rust_helper_minor(dev_t dev) {
	return MINOR(dev);
}
EXPORT_SYMBOL_GPL(rust_helper_minor);

// void rust_helper_anon_inode_getfile(const char *name,
// 				const struct file_operations *fops,
// 				void *priv, int flags) {
// 	anon_inode_getfile(name, fops ,priv ,flags);
// }
// EXPORT_SYMBOL_GPL(rust_helper_anon_inode_getfile);
#endif 
/* We use bindgen's --size_t-is-usize option to bind the C size_t type
 * as the Rust usize type, so we can use it in contexts where Rust
 * expects a usize like slice (array) indices. usize is defined to be
 * the same as C's uintptr_t type (can hold any pointer) but not
 * necessarily the same as size_t (can hold the size of any single
 * object). Most modern platforms use the same concrete integer type for
 * both of them, but in case we find ourselves on a platform where
 * that's not true, fail early instead of risking ABI or
 * integer-overflow issues.
 *
 * If your platform fails this assertion, it means that you are in
 * danger of integer-overflow bugs (even if you attempt to remove
 * --size_t-is-usize). It may be easiest to change the kernel ABI on
 * your platform such that size_t matches uintptr_t (i.e., to increase
 * size_t, because uintptr_t has to be at least as big as size_t).
*/
static_assert(
	sizeof(size_t) == sizeof(uintptr_t) &&
	__alignof__(size_t) == __alignof__(uintptr_t),
	"Rust code expects C size_t to match Rust usize"
);
