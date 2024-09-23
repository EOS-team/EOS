/*
 * SPDX-License-Identifier: GPL-2.0
 *
 * Copyright (C) 2020 Philippe Gerum  <rpm@xenomai.org>
 */

#include <linux/netdevice.h>
#include <linux/skbuff.h>
#include <net/pkt_sched.h>
#include <net/pkt_cls.h>

/*
 * With Qdisc[2], 0=oob_fallback and 1=inband. User can graft whatever
 * qdisc on these slots; both preset to pfifo_ops. skb->oob is checked
 * to determine which qdisc should handle the packet eventually.
 */

struct oob_qdisc_priv {
	struct Qdisc *qdisc[2];	/* 0=oob_fallback, 1=in-band */
	struct tcf_proto __rcu *filter_list;
	struct tcf_block *block;
};

static int oob_enqueue(struct sk_buff *skb, struct Qdisc *sch,
		struct sk_buff **to_free)
{
	struct oob_qdisc_priv *p = qdisc_priv(sch);
	struct net_device *dev = skb->dev;
	struct Qdisc *qdisc;
	int ret;

	/*
	 * If the device accepts oob traffic and can handle it
	 * directly from the oob stage, pass the outgoing packet to
	 * the transmit handler of the oob stack. This makes sure that
	 * all traffic, including the in-band one, flows through the
	 * oob stack which may implement its own queuing discipline.
	 *
	 * netif_xmit_oob() might fail handling the packet, in which
	 * case we leave it to the in-band packet scheduler, applying
	 * a best-effort strategy by giving higher priority to oob
	 * packets over mere in-band traffic.
	 */
	if (dev && netif_oob_diversion(dev) && netdev_is_oob_capable(dev)) {
		ret = netif_xmit_oob(skb);
		if (ret == NET_XMIT_SUCCESS)
			return NET_XMIT_SUCCESS;
	}

	/*
	 * Out-of-band fast lane is closed. Best effort: use a special
	 * 'high priority' queue for oob packets we handle from
	 * in-band context the usual way through the common stack.
	 */
	qdisc = skb->oob ? p->qdisc[0] : p->qdisc[1];
	ret = qdisc_enqueue(skb, qdisc, to_free);
	if (ret == NET_XMIT_SUCCESS) {
		sch->q.qlen++;
		return NET_XMIT_SUCCESS;
	}

	if (net_xmit_drop_count(ret))
		qdisc_qstats_drop(sch);

	return ret;
}

static struct sk_buff *oob_dequeue(struct Qdisc *sch)
{
	struct oob_qdisc_priv *p = qdisc_priv(sch);
	struct sk_buff *skb;
	struct Qdisc *qdisc;
	int band;

	/*
	 * First try to dequeue pending out-of-band packets. If none,
	 * then check for in-band traffic.
	 */
	for (band = 0; band < 2; band++) {
		qdisc = p->qdisc[band];
		skb = qdisc->dequeue(qdisc);
		if (skb) {
			qdisc_bstats_update(sch, skb);
			sch->q.qlen--;
			return skb;
		}
	}

	return NULL;
}

static struct sk_buff *oob_peek(struct Qdisc *sch)
{
	struct oob_qdisc_priv *p = qdisc_priv(sch);
	struct sk_buff *skb;
	struct Qdisc *qdisc;
	int band;

	for (band = 0; band < 2; band++) {
		qdisc = p->qdisc[band];
		skb = qdisc->ops->peek(qdisc);
		if (skb)
			return skb;
	}

	return NULL;
}

static int oob_init(struct Qdisc *sch, struct nlattr *opt,
		struct netlink_ext_ack *extack)
{
	struct oob_qdisc_priv *p = qdisc_priv(sch);
	int ret;

	ret = tcf_block_get(&p->block, &p->filter_list, sch, extack);
	if (ret)
		return ret;

	p->qdisc[0] = qdisc_create_dflt(sch->dev_queue,
					&pfifo_qdisc_ops, sch->handle,
					extack);
	p->qdisc[1] = qdisc_create_dflt(sch->dev_queue,
					&pfifo_fast_ops, sch->handle,
					extack);

	return 0;
}

static void oob_reset(struct Qdisc *sch)
{
	struct oob_qdisc_priv *p = qdisc_priv(sch);

	qdisc_reset(p->qdisc[0]);
	qdisc_reset(p->qdisc[1]);
	sch->q.qlen = 0;
}

static void oob_destroy(struct Qdisc *sch)
{
	struct oob_qdisc_priv *p = qdisc_priv(sch);

	tcf_block_put(p->block);
	qdisc_put(p->qdisc[0]);
	qdisc_put(p->qdisc[1]);
}

static int oob_tune(struct Qdisc *sch, struct nlattr *opt,
		struct netlink_ext_ack *extack)
{
	return 0;
}

static int oob_dump(struct Qdisc *sch, struct sk_buff *skb)
{
	return skb->len;
}

static int oob_graft(struct Qdisc *sch, unsigned long arg, struct Qdisc *new,
		struct Qdisc **old, struct netlink_ext_ack *extack)
{
	struct oob_qdisc_priv *p = qdisc_priv(sch);
	unsigned long band = arg - 1;

	if (new == NULL)
		new = &noop_qdisc;

	*old = qdisc_replace(sch, new, &p->qdisc[band]);

	return 0;
}

static struct Qdisc *
oob_leaf(struct Qdisc *sch, unsigned long arg)
{
	struct oob_qdisc_priv *p = qdisc_priv(sch);
	unsigned long band = arg - 1;

	return p->qdisc[band];
}

static unsigned long oob_find(struct Qdisc *sch, u32 classid)
{
	unsigned long band = TC_H_MIN(classid);

	return band - 1 >= 2 ? 0 : band;
}

static int oob_dump_class(struct Qdisc *sch, unsigned long cl,
			struct sk_buff *skb, struct tcmsg *tcm)
{
	struct oob_qdisc_priv *p = qdisc_priv(sch);

	tcm->tcm_handle |= TC_H_MIN(cl);
	tcm->tcm_info = p->qdisc[cl - 1]->handle;

	return 0;
}

static int oob_dump_class_stats(struct Qdisc *sch, unsigned long cl,
				struct gnet_dump *d)
{
	struct oob_qdisc_priv *p = qdisc_priv(sch);
	struct Qdisc *cl_q = p->qdisc[cl - 1];

	if (gnet_stats_copy_basic(qdisc_root_sleeping_running(sch),
				  d, cl_q->cpu_bstats, &cl_q->bstats) < 0 ||
	    qdisc_qstats_copy(d, cl_q) < 0)
		return -1;

	return 0;
}

static void oob_walk(struct Qdisc *sch, struct qdisc_walker *arg)
{
	int band;

	if (arg->stop)
		return;

	for (band = 0; band < 2; band++) {
		if (arg->count < arg->skip) {
			arg->count++;
			continue;
		}
		if (arg->fn(sch, band + 1, arg) < 0) {
			arg->stop = 1;
			break;
		}
		arg->count++;
	}
}

static unsigned long oob_tcf_bind(struct Qdisc *sch, unsigned long parent,
				 u32 classid)
{
	return oob_find(sch, classid);
}

static void oob_tcf_unbind(struct Qdisc *q, unsigned long cl)
{
}

static struct tcf_block *oob_tcf_block(struct Qdisc *sch, unsigned long cl,
				       struct netlink_ext_ack *extack)
{
	struct oob_qdisc_priv *p = qdisc_priv(sch);

	if (cl)
		return NULL;

	return p->block;
}

static const struct Qdisc_class_ops oob_class_ops = {
	.graft		=	oob_graft,
	.leaf		=	oob_leaf,
	.find		=	oob_find,
	.walk		=	oob_walk,
	.dump		=	oob_dump_class,
	.dump_stats	=	oob_dump_class_stats,
	.tcf_block	=	oob_tcf_block,
	.bind_tcf	=	oob_tcf_bind,
	.unbind_tcf	=	oob_tcf_unbind,
};

static struct Qdisc_ops oob_qdisc_ops __read_mostly = {
	.cl_ops		=	&oob_class_ops,
	.id		=	"oob",
	.priv_size	=	sizeof(struct oob_qdisc_priv),
	.enqueue	=	oob_enqueue,
	.dequeue	=	oob_dequeue,
	.peek		=	oob_peek,
	.init		=	oob_init,
	.reset		=	oob_reset,
	.destroy	=	oob_destroy,
	.change		=	oob_tune,
	.dump		=	oob_dump,
	.owner		=	THIS_MODULE,
};

static int __init oob_module_init(void)
{
	return register_qdisc(&oob_qdisc_ops);
}

static void __exit oob_module_exit(void)
{
	unregister_qdisc(&oob_qdisc_ops);
}

module_init(oob_module_init)
module_exit(oob_module_exit)

MODULE_LICENSE("GPL");
