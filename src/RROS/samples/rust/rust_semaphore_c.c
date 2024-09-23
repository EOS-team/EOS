// SPDX-License-Identifier: GPL-2.0
/*
 * Rust semaphore sample (in C, for comparison)
 *
 * This is a C implementation of `rust_semaphore.rs`. Refer to the description
 * in that file for details on the device.
 */

#define pr_fmt(fmt) KBUILD_MODNAME ": " fmt

#include <linux/miscdevice.h>
#include <linux/module.h>
#include <linux/fs.h>
#include <linux/slab.h>
#include <linux/refcount.h>
#include <linux/wait.h>

#define IOCTL_GET_READ_COUNT _IOR('c', 1, u64)
#define IOCTL_SET_READ_COUNT _IOW('c', 1, u64)

struct semaphore_state {
	struct kref ref;
	struct miscdevice miscdev;
	wait_queue_head_t changed;
	struct mutex mutex;
	size_t count;
	size_t max_seen;
};

struct file_state {
	atomic64_t read_count;
	struct semaphore_state *shared;
};

static int semaphore_consume(struct semaphore_state *state)
{
	DEFINE_WAIT(wait);

	mutex_lock(&state->mutex);
	while (state->count == 0) {
		prepare_to_wait(&state->changed, &wait, TASK_INTERRUPTIBLE);
		mutex_unlock(&state->mutex);
		schedule();
		finish_wait(&state->changed, &wait);
		if (signal_pending(current))
			return -EINTR;
		mutex_lock(&state->mutex);
	}

	state->count--;
	mutex_unlock(&state->mutex);

	return 0;
}

static int semaphore_open(struct inode *nodp, struct file *filp)
{
	struct semaphore_state *shared =
		container_of(filp->private_data, struct semaphore_state, miscdev);
	struct file_state *state;

	state = kzalloc(sizeof(*state), GFP_KERNEL);
	if (!state)
		return -ENOMEM;

	kref_get(&shared->ref);
	state->shared = shared;
	atomic64_set(&state->read_count, 0);

	filp->private_data = state;

	return 0;
}

static ssize_t semaphore_write(struct file *filp, const char __user *buffer, size_t count,
			       loff_t *ppos)
{
	struct file_state *state = filp->private_data;
	struct semaphore_state *shared = state->shared;

	mutex_lock(&shared->mutex);

	shared->count += count;
	if (shared->count < count)
		shared->count = SIZE_MAX;

	if (shared->count > shared->max_seen)
		shared->max_seen = shared->count;

	mutex_unlock(&shared->mutex);

	wake_up_all(&shared->changed);

	return count;
}

static ssize_t semaphore_read(struct file *filp, char __user *buffer,
			      size_t count, loff_t *ppos)
{
	struct file_state *state = filp->private_data;
	char c = 0;
	int ret;

	if (count == 0 || *ppos > 0)
		return 0;

	ret = semaphore_consume(state->shared);
	if (ret)
		return ret;

	if (copy_to_user(buffer, &c, sizeof(c)))
		return -EFAULT;

	atomic64_add(1, &state->read_count);
	*ppos += 1;
	return 1;
}

static long semaphore_ioctl(struct file *filp, unsigned int cmd, unsigned long arg)
{
	struct file_state *state = filp->private_data;
	void __user *buffer = (void __user *)arg;
	u64 value;

	switch (cmd) {
	case IOCTL_GET_READ_COUNT:
		value = atomic64_read(&state->read_count);
		if (copy_to_user(buffer, &value, sizeof(value)))
			return -EFAULT;
		return 0;
	case IOCTL_SET_READ_COUNT:
		if (copy_from_user(&value, buffer, sizeof(value)))
			return -EFAULT;
		atomic64_set(&state->read_count, value);
		return 0;
	default:
		return -EINVAL;
	}
}

static void semaphore_free(struct kref *kref)
{
	struct semaphore_state *device;

	device = container_of(kref, struct semaphore_state, ref);
	kfree(device);
}

static int semaphore_release(struct inode *nodp, struct file *filp)
{
	struct file_state *state = filp->private_data;

	kref_put(&state->shared->ref, semaphore_free);
	kfree(state);
	return 0;
}

static const struct file_operations semaphore_fops = {
	.owner = THIS_MODULE,
	.open = semaphore_open,
	.read = semaphore_read,
	.write = semaphore_write,
	.compat_ioctl = semaphore_ioctl,
	.release = semaphore_release,
};

static struct semaphore_state *device;

static int __init semaphore_init(void)
{
	int ret;
	struct semaphore_state *state;

	pr_info("Rust semaphore sample (in C, for comparison) (init)\n");

	state = kzalloc(sizeof(*state), GFP_KERNEL);
	if (!state)
		return -ENOMEM;

	mutex_init(&state->mutex);
	kref_init(&state->ref);
	init_waitqueue_head(&state->changed);

	state->miscdev.fops = &semaphore_fops;
	state->miscdev.minor = MISC_DYNAMIC_MINOR;
	state->miscdev.name = "semaphore";

	ret = misc_register(&state->miscdev);
	if (ret < 0) {
		kfree(state);
		return ret;
	}

	device = state;

	return 0;
}

static void __exit semaphore_exit(void)
{
	pr_info("Rust semaphore sample (in C, for comparison) (exit)\n");

	misc_deregister(&device->miscdev);
	kref_put(&device->ref, semaphore_free);
}

module_init(semaphore_init);
module_exit(semaphore_exit);

MODULE_LICENSE("GPL v2");
MODULE_AUTHOR("Rust for Linux Contributors");
MODULE_DESCRIPTION("Rust semaphore sample (in C, for comparison)");
