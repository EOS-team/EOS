// SPDX-License-Identifier: GPL-2.0-only
/*
 * Virtual DMA channel support for DMAengine
 *
 * Copyright (C) 2012 Russell King
 */
#include <linux/device.h>
#include <linux/dmaengine.h>
#include <linux/module.h>
#include <linux/spinlock.h>

#include "virt-dma.h"

static struct virt_dma_desc *to_virt_desc(struct dma_async_tx_descriptor *tx)
{
	return container_of(tx, struct virt_dma_desc, tx);
}

dma_cookie_t vchan_tx_submit(struct dma_async_tx_descriptor *tx)
{
	struct virt_dma_chan *vc = to_virt_chan(tx->chan);
	struct virt_dma_desc *vd = to_virt_desc(tx);
	unsigned long flags;
	dma_cookie_t cookie;

	vchan_lock_irqsave(vc, flags);
	cookie = dma_cookie_assign(tx);

	list_move_tail(&vd->node, &vc->desc_submitted);
	vchan_unlock_irqrestore(vc, flags);

	dev_dbg(vc->chan.device->dev, "vchan %p: txd %p[%x]: submitted\n",
		vc, vd, cookie);

	return cookie;
}
EXPORT_SYMBOL_GPL(vchan_tx_submit);

/**
 * vchan_tx_desc_free - free a reusable descriptor
 * @tx: the transfer
 *
 * This function frees a previously allocated reusable descriptor. The only
 * other way is to clear the DMA_CTRL_REUSE flag and submit one last time the
 * transfer.
 *
 * Returns 0 upon success
 */
int vchan_tx_desc_free(struct dma_async_tx_descriptor *tx)
{
	struct virt_dma_chan *vc = to_virt_chan(tx->chan);
	struct virt_dma_desc *vd = to_virt_desc(tx);
	unsigned long flags;

	vchan_lock_irqsave(vc, flags);
	list_del(&vd->node);
	vchan_unlock_irqrestore(vc, flags);

	dev_dbg(vc->chan.device->dev, "vchan %p: txd %p[%x]: freeing\n",
		vc, vd, vd->tx.cookie);
	vc->desc_free(vd);
	return 0;
}
EXPORT_SYMBOL_GPL(vchan_tx_desc_free);

struct virt_dma_desc *vchan_find_desc(struct virt_dma_chan *vc,
	dma_cookie_t cookie)
{
	struct virt_dma_desc *vd;

	list_for_each_entry(vd, &vc->desc_issued, node)
		if (vd->tx.cookie == cookie)
			return vd;

	return NULL;
}
EXPORT_SYMBOL_GPL(vchan_find_desc);

/*
 * This tasklet handles the completion of a DMA descriptor by
 * calling its callback and freeing it.
 */
static void vchan_complete(struct tasklet_struct *t)
{
	struct virt_dma_chan *vc = from_tasklet(vc, t, task);
	struct virt_dma_desc *vd, *_vd;
	struct dmaengine_desc_callback cb;
	LIST_HEAD(head);

	vchan_lock_irq(vc);
	list_splice_tail_init(&vc->desc_completed, &head);
	vd = vc->cyclic;
	if (vd) {
		vc->cyclic = NULL;
		dmaengine_desc_get_callback(&vd->tx, &cb);
	} else {
		memset(&cb, 0, sizeof(cb));
	}
	vchan_unlock_irq(vc);

	dmaengine_desc_callback_invoke(&cb, &vd->tx_result);

	list_for_each_entry_safe(vd, _vd, &head, node) {
		dmaengine_desc_get_callback(&vd->tx, &cb);

		list_del(&vd->node);
		dmaengine_desc_callback_invoke(&cb, &vd->tx_result);
		vchan_vdesc_fini(vd);
	}
}

void vchan_dma_desc_free_list(struct virt_dma_chan *vc, struct list_head *head)
{
	struct virt_dma_desc *vd, *_vd;

	list_for_each_entry_safe(vd, _vd, head, node) {
		list_del(&vd->node);
		vchan_vdesc_fini(vd);
	}
}
EXPORT_SYMBOL_GPL(vchan_dma_desc_free_list);

#ifdef CONFIG_DMA_VIRTUAL_CHANNELS_OOB

static void inband_init_chan_lock(struct virt_dma_chan *vc)
{
	spin_lock_init(&vc->lock);
}

static void inband_lock_chan(struct virt_dma_chan *vc)
{
	spin_lock(&vc->lock);
}

static void inband_unlock_chan(struct virt_dma_chan *vc)
{
	spin_unlock(&vc->lock);
}

static void inband_lock_irq_chan(struct virt_dma_chan *vc)
{
	spin_lock_irq(&vc->lock);
}

static void inband_unlock_irq_chan(struct virt_dma_chan *vc)
{
	spin_unlock_irq(&vc->lock);
}

static unsigned long inband_lock_irqsave_chan(struct virt_dma_chan *vc)
{
	unsigned long flags;

	spin_lock_irqsave(&vc->lock, flags);

	return flags;
}

static void inband_unlock_irqrestore_chan(struct virt_dma_chan *vc,
			unsigned long flags)
{
	spin_unlock_irqrestore(&vc->lock, flags);
}

static struct virt_dma_lockops inband_lock_ops = {
	.init			= inband_init_chan_lock,
	.lock			= inband_lock_chan,
	.unlock			= inband_unlock_chan,
	.lock_irq		= inband_lock_irq_chan,
	.unlock_irq		= inband_unlock_irq_chan,
	.lock_irqsave		= inband_lock_irqsave_chan,
	.unlock_irqrestore	= inband_unlock_irqrestore_chan,
};

static void oob_init_chan_lock(struct virt_dma_chan *vc)
{
	raw_spin_lock_init(&vc->oob_lock);
}

static void oob_lock_chan(struct virt_dma_chan *vc)
{
	raw_spin_lock(&vc->oob_lock);
}

static void oob_unlock_chan(struct virt_dma_chan *vc)
{
	raw_spin_unlock(&vc->oob_lock);
}

static void oob_lock_irq_chan(struct virt_dma_chan *vc)
{
	raw_spin_lock_irq(&vc->oob_lock);
}

static void oob_unlock_irq_chan(struct virt_dma_chan *vc)
{
	raw_spin_unlock_irq(&vc->oob_lock);
}

static unsigned long oob_lock_irqsave_chan(struct virt_dma_chan *vc)
{
	unsigned long flags;

	raw_spin_lock_irqsave(&vc->oob_lock, flags);

	return flags;
}

static void oob_unlock_irqrestore_chan(struct virt_dma_chan *vc,
				unsigned long flags)
{
	raw_spin_unlock_irqrestore(&vc->oob_lock, flags);
}

static struct virt_dma_lockops oob_lock_ops = {
	.init			= oob_init_chan_lock,
	.lock			= oob_lock_chan,
	.unlock			= oob_unlock_chan,
	.lock_irq		= oob_lock_irq_chan,
	.unlock_irq		= oob_unlock_irq_chan,
	.lock_irqsave		= oob_lock_irqsave_chan,
	.unlock_irqrestore	= oob_unlock_irqrestore_chan,
};

#endif

void vchan_init(struct virt_dma_chan *vc, struct dma_device *dmadev)
{
	dma_cookie_init(&vc->chan);

#ifdef CONFIG_DMA_VIRTUAL_CHANNELS_OOB
	vc->lock_ops = test_bit(DMA_OOB, dmadev->cap_mask.bits) ?
		&oob_lock_ops : &inband_lock_ops;
#endif
	vchan_lock_init(vc);
	INIT_LIST_HEAD(&vc->desc_allocated);
	INIT_LIST_HEAD(&vc->desc_submitted);
	INIT_LIST_HEAD(&vc->desc_issued);
	INIT_LIST_HEAD(&vc->desc_completed);
	INIT_LIST_HEAD(&vc->desc_terminated);

	tasklet_setup(&vc->task, vchan_complete);

	vc->chan.device = dmadev;
	list_add_tail(&vc->chan.device_node, &dmadev->channels);
}
EXPORT_SYMBOL_GPL(vchan_init);

MODULE_AUTHOR("Russell King");
MODULE_LICENSE("GPL");
