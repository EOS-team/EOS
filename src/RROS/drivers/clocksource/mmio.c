// SPDX-License-Identifier: GPL-2.0-only
/*
 * Generic MMIO clocksource support
 */
#include <linux/clocksource.h>
#include <linux/errno.h>
#include <linux/init.h>
#include <linux/slab.h>
#include <linux/spinlock.h>
#include <linux/uaccess.h>
#include <linux/miscdevice.h>
#include <linux/list.h>
#include <linux/slab.h>
#include <linux/fs.h>
#include <linux/mm.h>
#include <linux/mman.h>
#include <linux/device.h>

struct clocksource_user_mapping {
	struct mm_struct *mm;
	struct clocksource_user_mmio *ucs;
	void *regs;
	struct hlist_node link;
	atomic_t refs;
};

static struct class *user_mmio_class;
static dev_t user_mmio_devt;

static DEFINE_SPINLOCK(user_clksrcs_lock);
static unsigned int user_clksrcs_count;
static LIST_HEAD(user_clksrcs);

static inline struct clocksource_mmio *to_mmio_clksrc(struct clocksource *c)
{
	return container_of(c, struct clocksource_mmio, clksrc);
}

u64 clocksource_mmio_readl_up(struct clocksource *c)
{
	return (u64)readl_relaxed(to_mmio_clksrc(c)->reg);
}

u64 clocksource_mmio_readl_down(struct clocksource *c)
{
	return ~(u64)readl_relaxed(to_mmio_clksrc(c)->reg) & c->mask;
}

u64 clocksource_mmio_readw_up(struct clocksource *c)
{
	return (u64)readw_relaxed(to_mmio_clksrc(c)->reg);
}

u64 clocksource_mmio_readw_down(struct clocksource *c)
{
	return ~(u64)readw_relaxed(to_mmio_clksrc(c)->reg) & c->mask;
}

static inline struct clocksource_user_mmio *
to_mmio_ucs(struct clocksource *c)
{
	return container_of(c, struct clocksource_user_mmio, mmio.clksrc);
}

u64 clocksource_dual_mmio_readl_up(struct clocksource *c)
{
	struct clocksource_user_mmio *ucs = to_mmio_ucs(c);
	u32 upper, old_upper, lower;

	upper = readl_relaxed(ucs->reg_upper);
	do {
		old_upper = upper;
		lower = readl_relaxed(ucs->mmio.reg);
		upper = readl_relaxed(ucs->reg_upper);
	} while (upper != old_upper);

	return (((u64)upper) << ucs->bits_lower) | lower;
}

u64 clocksource_dual_mmio_readw_up(struct clocksource *c)
{
	struct clocksource_user_mmio *ucs = to_mmio_ucs(c);
	u16 upper, old_upper, lower;

	upper = readw_relaxed(ucs->reg_upper);
	do {
		old_upper = upper;
		lower = readw_relaxed(ucs->mmio.reg);
		upper = readw_relaxed(ucs->reg_upper);
	} while (upper != old_upper);

	return (((u64)upper) << ucs->bits_lower) | lower;
}

static void mmio_base_init(const char *name,int rating, unsigned int bits,
			   u64 (*read)(struct clocksource *),
			   struct clocksource *cs)
{
	cs->name = name;
	cs->rating = rating;
	cs->read = read;
	cs->mask = CLOCKSOURCE_MASK(bits);
	cs->flags = CLOCK_SOURCE_IS_CONTINUOUS;
}

/**
 * clocksource_mmio_init - Initialize a simple mmio based clocksource
 * @base:	Virtual address of the clock readout register
 * @name:	Name of the clocksource
 * @hz:		Frequency of the clocksource in Hz
 * @rating:	Rating of the clocksource
 * @bits:	Number of valid bits
 * @read:	One of clocksource_mmio_read*() above
 */
int __init clocksource_mmio_init(void __iomem *base, const char *name,
	unsigned long hz, int rating, unsigned bits,
	u64 (*read)(struct clocksource *))
{
	struct clocksource_mmio *cs;
	int err;

	if (bits > 64 || bits < 16)
		return -EINVAL;

	cs = kzalloc(sizeof(struct clocksource_mmio), GFP_KERNEL);
	if (!cs)
		return -ENOMEM;

	cs->reg = base;
	mmio_base_init(name, rating, bits, read, &cs->clksrc);

	err = clocksource_register_hz(&cs->clksrc, hz);
	if (err < 0) {
		kfree(cs);
		return err;
	}

	return err;
}

static void mmio_ucs_vmopen(struct vm_area_struct *vma)
{
	struct clocksource_user_mapping *mapping, *clone;
	struct clocksource_user_mmio *ucs;
	unsigned long h_key;

	mapping = vma->vm_private_data;

	if (mapping->mm == vma->vm_mm) {
		atomic_inc(&mapping->refs);
	} else if (mapping->mm) {
		/*
		 * We must be duplicating the original mm upon fork(),
		 * clone the parent ucs mapping struct then rehash it
		 * on the child mm key. If we cannot get memory for
		 * this, mitigate the issue for users by preventing a
		 * stale parent mm from being matched later on by a
		 * process which reused its mm_struct (h_key is based
		 * on this struct address).
		 */
		clone = kmalloc(sizeof(*mapping), GFP_KERNEL);
		if (clone == NULL) {
			pr_alert("out-of-memory for UCS mapping!\n");
			atomic_inc(&mapping->refs);
			mapping->mm = NULL;
			return;
		}
		ucs = mapping->ucs;
		clone->mm = vma->vm_mm;
		clone->ucs = ucs;
		clone->regs = mapping->regs;
		atomic_set(&clone->refs, 1);
		vma->vm_private_data = clone;
		h_key = (unsigned long)vma->vm_mm / sizeof(*vma->vm_mm);
		spin_lock(&ucs->lock);
		hash_add(ucs->mappings, &clone->link, h_key);
		spin_unlock(&ucs->lock);
	}
}

static void mmio_ucs_vmclose(struct vm_area_struct *vma)
{
	struct clocksource_user_mapping *mapping;

	mapping = vma->vm_private_data;

	if (atomic_dec_and_test(&mapping->refs)) {
		spin_lock(&mapping->ucs->lock);
		hash_del(&mapping->link);
		spin_unlock(&mapping->ucs->lock);
		kfree(mapping);
	}
}

static const struct vm_operations_struct mmio_ucs_vmops = {
	.open = mmio_ucs_vmopen,
	.close = mmio_ucs_vmclose,
};

static int mmio_ucs_mmap(struct file *file, struct vm_area_struct *vma)
{
	unsigned long addr, upper_pfn, lower_pfn;
	struct clocksource_user_mapping *mapping, *tmp;
	struct clocksource_user_mmio *ucs;
	unsigned int bits_upper;
	unsigned long h_key;
	pgprot_t prot;
	size_t pages;
	int err;

	pages = (vma->vm_end - vma->vm_start) >> PAGE_SHIFT;
	if (pages > 2)
		return -EINVAL;

	vma->vm_private_data = NULL;

	ucs = file->private_data;
	upper_pfn = ucs->phys_upper >> PAGE_SHIFT;
	lower_pfn = ucs->phys_lower >> PAGE_SHIFT;
	bits_upper = fls(ucs->mmio.clksrc.mask) - ucs->bits_lower;
	if (pages == 2 && (!bits_upper || upper_pfn == lower_pfn))
		return -EINVAL;

	mapping = kmalloc(sizeof(*mapping), GFP_KERNEL);
	if (!mapping)
		return -ENOSPC;

	mapping->mm = vma->vm_mm;
	mapping->ucs = ucs;
	mapping->regs = (void *)vma->vm_start;
	atomic_set(&mapping->refs, 1);

	vma->vm_private_data = mapping;
	vma->vm_ops = &mmio_ucs_vmops;
	prot = pgprot_noncached(vma->vm_page_prot);
	addr = vma->vm_start;

	err = remap_pfn_range(vma, addr, lower_pfn, PAGE_SIZE, prot);
	if (err < 0)
		goto fail;

	if (pages > 1) {
		addr += PAGE_SIZE;
		err = remap_pfn_range(vma, addr, upper_pfn, PAGE_SIZE, prot);
		if (err < 0)
			goto fail;
	}

	h_key = (unsigned long)vma->vm_mm / sizeof(*vma->vm_mm);

	spin_lock(&ucs->lock);
	hash_for_each_possible(ucs->mappings, tmp, link, h_key) {
		if (tmp->mm == vma->vm_mm) {
			spin_unlock(&ucs->lock);
			err = -EBUSY;
			goto fail;
		}
	}
	hash_add(ucs->mappings, &mapping->link, h_key);
	spin_unlock(&ucs->lock);

	return 0;
fail:
	kfree(mapping);

	return err;
}

static long
mmio_ucs_ioctl(struct file *file, unsigned int cmd, unsigned long arg)
{
	struct clocksource_user_mapping *mapping;
	struct clksrc_user_mmio_info __user *u;
	unsigned long upper_pfn, lower_pfn;
	struct clksrc_user_mmio_info info;
	struct clocksource_user_mmio *ucs;
	unsigned int bits_upper;
	void __user *map_base;
	unsigned long h_key;
	size_t size;

	u = (struct clksrc_user_mmio_info __user *)arg;

	switch (cmd) {
	case CLKSRC_USER_MMIO_MAP:
		break;
	default:
		return -ENOTTY;
	}

	h_key = (unsigned long)current->mm / sizeof(*current->mm);

	ucs = file->private_data;
	upper_pfn = ucs->phys_upper >> PAGE_SHIFT;
	lower_pfn = ucs->phys_lower >> PAGE_SHIFT;
	bits_upper = fls(ucs->mmio.clksrc.mask) - ucs->bits_lower;
	size = PAGE_SIZE;
	if (bits_upper && upper_pfn != lower_pfn)
		size += PAGE_SIZE;

	do {
		spin_lock(&ucs->lock);
		hash_for_each_possible(ucs->mappings, mapping, link, h_key) {
			if (mapping->mm == current->mm) {
				spin_unlock(&ucs->lock);
				map_base = mapping->regs;
				goto found;
			}
		}
		spin_unlock(&ucs->lock);

		map_base = (void *)
			vm_mmap(file, 0, size, PROT_READ, MAP_SHARED, 0);
	} while (IS_ERR(map_base) && PTR_ERR(map_base) == -EBUSY);

	if (IS_ERR(map_base))
		return PTR_ERR(map_base);

found:
	info.type = ucs->type;
	info.reg_lower = map_base + offset_in_page(ucs->phys_lower);
	info.mask_lower = ucs->mmio.clksrc.mask;
	info.bits_lower = ucs->bits_lower;
	info.reg_upper = NULL;
	if (ucs->phys_upper)
		info.reg_upper = map_base + (size - PAGE_SIZE)
			+ offset_in_page(ucs->phys_upper);
	info.mask_upper = ucs->mask_upper;

	return copy_to_user(u, &info, sizeof(*u));
}

static int mmio_ucs_open(struct inode *inode, struct file *file)
{
	struct clocksource_user_mmio *ucs;

	if (file->f_mode & FMODE_WRITE)
		return -EINVAL;

	ucs = container_of(inode->i_cdev, typeof(*ucs), cdev);
	file->private_data = ucs;

	return 0;
}

static const struct file_operations mmio_ucs_fops = {
	.owner		= THIS_MODULE,
	.unlocked_ioctl = mmio_ucs_ioctl,
	.open		= mmio_ucs_open,
	.mmap		= mmio_ucs_mmap,
};

static int __init
ucs_create_cdev(struct class *class, struct clocksource_user_mmio *ucs)
{
	int err;

	ucs->dev = device_create(class, NULL,
				MKDEV(MAJOR(user_mmio_devt), ucs->id),
				ucs, "ucs/%d", ucs->id);
	if (IS_ERR(ucs->dev))
		return PTR_ERR(ucs->dev);

	spin_lock_init(&ucs->lock);
	hash_init(ucs->mappings);

	cdev_init(&ucs->cdev, &mmio_ucs_fops);
	ucs->cdev.kobj.parent = &ucs->dev->kobj;

	err = cdev_add(&ucs->cdev, ucs->dev->devt, 1);
	if (err < 0)
		goto err_device_destroy;

	return 0;

err_device_destroy:
	device_destroy(class, MKDEV(MAJOR(user_mmio_devt), ucs->id));
	return err;
}

static unsigned long default_revmap(void *virt)
{
	struct vm_struct *vm;

	vm = find_vm_area(virt);
	if (!vm)
		return 0;

	return vm->phys_addr + (virt - vm->addr);
}

int __init clocksource_user_mmio_init(struct clocksource_user_mmio *ucs,
				      const struct clocksource_mmio_regs *regs,
				      unsigned long hz)
{
	static u64 (*user_types[CLKSRC_MMIO_TYPE_NR])(struct clocksource *) = {
		[CLKSRC_MMIO_L_UP] = clocksource_mmio_readl_up,
		[CLKSRC_MMIO_L_DOWN] = clocksource_mmio_readl_down,
		[CLKSRC_DMMIO_L_UP] = clocksource_dual_mmio_readl_up,
		[CLKSRC_MMIO_W_UP] = clocksource_mmio_readw_up,
		[CLKSRC_MMIO_W_DOWN] = clocksource_mmio_readw_down,
		[CLKSRC_DMMIO_W_UP] = clocksource_dual_mmio_readw_up,
	};
	const char *name = ucs->mmio.clksrc.name;
	unsigned long phys_upper = 0, phys_lower;
	enum clksrc_user_mmio_type type;
	unsigned long (*revmap)(void *);
	int err;

	if (regs->bits_lower > 32 || regs->bits_lower < 16 ||
	    regs->bits_upper > 32)
		return -EINVAL;

	for (type = 0; type < ARRAY_SIZE(user_types); type++)
		if (ucs->mmio.clksrc.read == user_types[type])
			break;

	if (type == ARRAY_SIZE(user_types))
		return -EINVAL;

	if (!(ucs->mmio.clksrc.flags & CLOCK_SOURCE_IS_CONTINUOUS))
		return -EINVAL;

	revmap = regs->revmap;
	if (!revmap)
		revmap = default_revmap;

	phys_lower = revmap(regs->reg_lower);
	if (!phys_lower)
		return -EINVAL;

	if (regs->bits_upper) {
		phys_upper = revmap(regs->reg_upper);
		if (!phys_upper)
			return -EINVAL;
	}

	ucs->mmio.reg = regs->reg_lower;
	ucs->type = type;
	ucs->bits_lower = regs->bits_lower;
	ucs->reg_upper = regs->reg_upper;
	ucs->mask_lower = CLOCKSOURCE_MASK(regs->bits_lower);
	ucs->mask_upper = CLOCKSOURCE_MASK(regs->bits_upper);
	ucs->phys_lower = phys_lower;
	ucs->phys_upper = phys_upper;
	spin_lock_init(&ucs->lock);

	err = clocksource_register_hz(&ucs->mmio.clksrc, hz);
	if (err < 0)
		return err;

	spin_lock(&user_clksrcs_lock);

	ucs->id = user_clksrcs_count++;
	if (ucs->id < CLKSRC_USER_MMIO_MAX)
		list_add_tail(&ucs->link, &user_clksrcs);

	spin_unlock(&user_clksrcs_lock);

	if (ucs->id >= CLKSRC_USER_MMIO_MAX) {
		pr_warn("%s: Too many clocksources\n", name);
		err = -EAGAIN;
		goto fail;
	}

	ucs->mmio.clksrc.vdso_type = CLOCKSOURCE_VDSO_MMIO + ucs->id;

	if (user_mmio_class) {
		err = ucs_create_cdev(user_mmio_class, ucs);
		if (err < 0) {
			pr_warn("%s: Failed to add character device\n", name);
			goto fail;
		}
	}

	return 0;

fail:
	clocksource_unregister(&ucs->mmio.clksrc);

	return err;
}

int __init clocksource_user_single_mmio_init(
	void __iomem *base, const char *name,
	unsigned long hz, int rating, unsigned int bits,
	u64 (*read)(struct clocksource *))
{
	struct clocksource_user_mmio *ucs;
	struct clocksource_mmio_regs regs;
	int ret;

	ucs = kzalloc(sizeof(*ucs), GFP_KERNEL);
	if (!ucs)
		return -ENOMEM;

	mmio_base_init(name, rating, bits, read, &ucs->mmio.clksrc);
	regs.reg_lower = base;
	regs.reg_upper = NULL;
	regs.bits_lower = bits;
	regs.bits_upper = 0;
	regs.revmap = NULL;

	ret = clocksource_user_mmio_init(ucs, &regs, hz);
	if (ret)
		kfree(ucs);

	return ret;
}

static int __init mmio_clksrc_chr_dev_init(void)
{
	struct clocksource_user_mmio *ucs;
	struct class *class;
	int err;

	class = class_create(THIS_MODULE, "mmio_ucs");
	if (IS_ERR(class)) {
		pr_err("couldn't create user mmio clocksources class\n");
		return PTR_ERR(class);
	}

	err = alloc_chrdev_region(&user_mmio_devt, 0, CLKSRC_USER_MMIO_MAX,
				  "mmio_ucs");
	if (err < 0) {
		pr_err("failed to allocate user mmio clocksources character devivces region\n");
		goto err_class_destroy;
	}

	/*
	 * Calling list_for_each_entry is safe here: clocksources are always
	 * added to the list tail, never removed.
	 */
	spin_lock(&user_clksrcs_lock);
	list_for_each_entry(ucs, &user_clksrcs, link) {
		spin_unlock(&user_clksrcs_lock);

		err = ucs_create_cdev(class, ucs);
		if (err < 0)
			pr_err("%s: Failed to add character device\n",
			       ucs->mmio.clksrc.name);

		spin_lock(&user_clksrcs_lock);
	}
	user_mmio_class = class;
	spin_unlock(&user_clksrcs_lock);

	return 0;

err_class_destroy:
	class_destroy(class);
	return err;
}
device_initcall(mmio_clksrc_chr_dev_init);
