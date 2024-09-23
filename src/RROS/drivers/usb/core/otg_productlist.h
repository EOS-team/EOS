/* SPDX-License-Identifier: GPL-2.0+ */
/* Copyright (C) 2004 Texas Instruments */

/*
 * This OTG and Embedded Host list is "Targeted Peripheral List".
 * It should mostly use of USB_DEVICE() or USB_DEVICE_VER() entries..
 *
 * YOU _SHOULD_ CHANGE THIS LIST TO MATCH YOUR PRODUCT AND ITS TESTING!
 */

static struct usb_device_id productlist_table[] = {

/* hubs are optional in OTG, but very handy ... */
#define CERT_WITHOUT_HUBS
#if defined(CERT_WITHOUT_HUBS)
{ USB_DEVICE( 0x0000, 0x0000 ), }, /* Root HUB Only*/
#else
{ USB_DEVICE_INFO(USB_CLASS_HUB, 0, 0), },
{ USB_DEVICE_INFO(USB_CLASS_HUB, 0, 1), },
{ USB_DEVICE_INFO(USB_CLASS_HUB, 0, 2), },
#endif

#ifdef	CONFIG_USB_PRINTER		/* ignoring nonstatic linkage! */
/* FIXME actually, printers are NOT supposed to use device classes;
 * they're supposed to use interface classes...
 */
//{ USB_DEVICE_INFO(7, 1, 1) },
//{ USB_DEVICE_INFO(7, 1, 2) },
//{ USB_DEVICE_INFO(7, 1, 3) },
#endif

#ifdef	CONFIG_USB_NET_CDCETHER
/* Linux-USB CDC Ethernet gadget */
//{ USB_DEVICE(0x0525, 0xa4a1), },
/* Linux-USB CDC Ethernet + RNDIS gadget */
//{ USB_DEVICE(0x0525, 0xa4a2), },
#endif

#if	IS_ENABLED(CONFIG_USB_TEST)
/* gadget zero, for testing */
//{ USB_DEVICE(0x0525, 0xa4a0), },
#endif

/* OPT Tester */
{ USB_DEVICE( 0x1a0a, 0x0101 ), }, /* TEST_SE0_NAK */
{ USB_DEVICE( 0x1a0a, 0x0102 ), }, /* Test_J */
{ USB_DEVICE( 0x1a0a, 0x0103 ), }, /* Test_K */
{ USB_DEVICE( 0x1a0a, 0x0104 ), }, /* Test_PACKET */
{ USB_DEVICE( 0x1a0a, 0x0105 ), }, /* Test_FORCE_ENABLE */
{ USB_DEVICE( 0x1a0a, 0x0106 ), }, /* HS_PORT_SUSPEND_RESUME  */
{ USB_DEVICE( 0x1a0a, 0x0107 ), }, /* SINGLE_STEP_GET_DESCRIPTOR setup */
{ USB_DEVICE( 0x1a0a, 0x0108 ), }, /* SINGLE_STEP_GET_DESCRIPTOR execute */

/* Sony cameras */
{ USB_DEVICE_VER(0x054c,0x0010,0x0410, 0x0500), },

/* Memory Devices */
//{ USB_DEVICE( 0x0781, 0x5150 ), }, /* SanDisk */
//{ USB_DEVICE( 0x05DC, 0x0080 ), }, /* Lexar */
//{ USB_DEVICE( 0x4146, 0x9281 ), }, /* IOMEGA */
//{ USB_DEVICE( 0x067b, 0x2507 ), }, /* Hammer 20GB External HD  */
{ USB_DEVICE( 0x0EA0, 0x2168 ), }, /* Ours Technology Inc. (BUFFALO ClipDrive)*/
//{ USB_DEVICE( 0x0457, 0x0150 ), }, /* Silicon Integrated Systems Corp. */

/* HP Printers */
//{ USB_DEVICE( 0x03F0, 0x1102 ), }, /* HP Photosmart 245 */
//{ USB_DEVICE( 0x03F0, 0x1302 ), }, /* HP Photosmart 370 Series */

/* Speakers */
//{ USB_DEVICE( 0x0499, 0x3002 ), }, /* YAMAHA YST-MS35D USB Speakers */
//{ USB_DEVICE( 0x0672, 0x1041 ), }, /* Labtec USB Headset */

{ }	/* Terminating entry */
};

static inline void report_errors(struct usb_device *dev)
{
	/* OTG MESSAGE: report errors here, customize to match your product */
	dev_info(&dev->dev, "device Vendor:%04x Product:%04x is not supported\n",
		 le16_to_cpu(dev->descriptor.idVendor),
		 le16_to_cpu(dev->descriptor.idProduct));
        if (USB_CLASS_HUB == dev->descriptor.bDeviceClass){
                dev_printk(KERN_CRIT, &dev->dev, "Unsupported Hub Topology\n");
        } else {
                dev_printk(KERN_CRIT, &dev->dev, "Attached Device is not Supported\n");
        }
}


static int is_targeted(struct usb_device *dev)
{
	struct usb_device_id	*id = productlist_table;

	/* HNP test device is _never_ targeted (see OTG spec 6.6.6) */
	if ((le16_to_cpu(dev->descriptor.idVendor) == 0x1a0a &&
	     le16_to_cpu(dev->descriptor.idProduct) == 0xbadd))
		return 0;

	/* OTG PET device is always targeted (see OTG 2.0 ECN 6.4.2) */
	if ((le16_to_cpu(dev->descriptor.idVendor) == 0x1a0a &&
	     le16_to_cpu(dev->descriptor.idProduct) == 0x0200))
		return 1;

	/* NOTE: can't use usb_match_id() since interface caches
	 * aren't set up yet. this is cut/paste from that code.
	 */
	for (id = productlist_table; id->match_flags; id++) {
		if ((id->match_flags & USB_DEVICE_ID_MATCH_VENDOR) &&
		    id->idVendor != le16_to_cpu(dev->descriptor.idVendor))
			continue;

		if ((id->match_flags & USB_DEVICE_ID_MATCH_PRODUCT) &&
		    id->idProduct != le16_to_cpu(dev->descriptor.idProduct))
			continue;

		/* No need to test id->bcdDevice_lo != 0, since 0 is never
		   greater than any unsigned number. */
		if ((id->match_flags & USB_DEVICE_ID_MATCH_DEV_LO) &&
		    (id->bcdDevice_lo > le16_to_cpu(dev->descriptor.bcdDevice)))
			continue;

		if ((id->match_flags & USB_DEVICE_ID_MATCH_DEV_HI) &&
		    (id->bcdDevice_hi < le16_to_cpu(dev->descriptor.bcdDevice)))
			continue;

		if ((id->match_flags & USB_DEVICE_ID_MATCH_DEV_CLASS) &&
		    (id->bDeviceClass != dev->descriptor.bDeviceClass))
			continue;

		if ((id->match_flags & USB_DEVICE_ID_MATCH_DEV_SUBCLASS) &&
		    (id->bDeviceSubClass != dev->descriptor.bDeviceSubClass))
			continue;

		if ((id->match_flags & USB_DEVICE_ID_MATCH_DEV_PROTOCOL) &&
		    (id->bDeviceProtocol != dev->descriptor.bDeviceProtocol))
			continue;

		return 1;
		/* NOTE: can't use usb_match_id() since interface caches
		 * aren't set up yet. this is cut/paste from that code.
		 */
		for (id = productlist_table; id->match_flags; id++) {
#ifdef DEBUG
			dev_dbg(&dev->dev,
				"ID: V:%04x P:%04x DC:%04x SC:%04x PR:%04x \n",
				id->idVendor,
				id->idProduct,
				id->bDeviceClass,
				id->bDeviceSubClass,
				id->bDeviceProtocol);
#endif

			if ((id->match_flags & USB_DEVICE_ID_MATCH_VENDOR) &&
			    id->idVendor != le16_to_cpu(dev->descriptor.idVendor))
				continue;

			if ((id->match_flags & USB_DEVICE_ID_MATCH_PRODUCT) &&
			    id->idProduct != le16_to_cpu(dev->descriptor.idProduct))
				continue;

			/* No need to test id->bcdDevice_lo != 0, since 0 is never
			   greater than any unsigned number. */
			if ((id->match_flags & USB_DEVICE_ID_MATCH_DEV_LO) &&
			    (id->bcdDevice_lo > le16_to_cpu(dev->descriptor.bcdDevice)))
				continue;

			if ((id->match_flags & USB_DEVICE_ID_MATCH_DEV_HI) &&
			    (id->bcdDevice_hi < le16_to_cpu(dev->descriptor.bcdDevice)))
				continue;

			if ((id->match_flags & USB_DEVICE_ID_MATCH_DEV_CLASS) &&
			    (id->bDeviceClass != dev->descriptor.bDeviceClass))
				continue;

			if ((id->match_flags & USB_DEVICE_ID_MATCH_DEV_SUBCLASS) &&
			    (id->bDeviceSubClass != dev->descriptor.bDeviceSubClass))
				continue;

			if ((id->match_flags & USB_DEVICE_ID_MATCH_DEV_PROTOCOL) &&
			    (id->bDeviceProtocol != dev->descriptor.bDeviceProtocol))
				continue;

			return 1;
		}
	}

	/* add other match criteria here ... */

	report_errors(dev);
	return 0;
}

