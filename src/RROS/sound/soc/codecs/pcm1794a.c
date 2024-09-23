/*
 * Driver for the PCM1794A codec
 *
 * Author:	Florian Meier <florian.meier@koalo.de>
 *		Copyright 2013
 *
 * This program is free software; you can redistribute it and/or
 * modify it under the terms of the GNU General Public License
 * version 2 as published by the Free Software Foundation.
 *
 * This program is distributed in the hope that it will be useful, but
 * WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 * General Public License for more details.
 */


#include <linux/init.h>
#include <linux/module.h>
#include <linux/platform_device.h>

#include <sound/soc.h>

static struct snd_soc_dai_driver pcm1794a_dai = {
	.name = "pcm1794a-hifi",
	.playback = {
		.channels_min = 2,
		.channels_max = 2,
		.rates = SNDRV_PCM_RATE_8000_192000,
		.formats = SNDRV_PCM_FMTBIT_S16_LE |
			   SNDRV_PCM_FMTBIT_S24_LE
	},
};

static struct snd_soc_component_driver soc_component_dev_pcm1794a;

static int pcm1794a_probe(struct platform_device *pdev)
{
	return snd_soc_register_component(&pdev->dev, &soc_component_dev_pcm1794a,
			&pcm1794a_dai, 1);
}

static int pcm1794a_remove(struct platform_device *pdev)
{
	snd_soc_unregister_component(&pdev->dev);
	return 0;
}

static const struct of_device_id pcm1794a_of_match[] = {
	{ .compatible = "ti,pcm1794a", },
	{ }
};
MODULE_DEVICE_TABLE(of, pcm1794a_of_match);

static struct platform_driver pcm1794a_component_driver = {
	.probe 		= pcm1794a_probe,
	.remove 	= pcm1794a_remove,
	.driver		= {
		.name	= "pcm1794a-codec",
		.owner	= THIS_MODULE,
		.of_match_table = of_match_ptr(pcm1794a_of_match),
	},
};

module_platform_driver(pcm1794a_component_driver);

MODULE_DESCRIPTION("ASoC PCM1794A codec driver");
MODULE_AUTHOR("Florian Meier <florian.meier@koalo.de>");
MODULE_LICENSE("GPL v2");
