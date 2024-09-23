/*
 * ALSA ASoC Machine Driver for Allo Piano DAC
 *
 * Author:	Baswaraj K <jaikumar@cem-solutions.net>
 *		Copyright 2016
 *		based on code by Daniel Matuschek <info@crazy-audio.com>
 *		based on code by Florian Meier <florian.meier@koalo.de>
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

#include <linux/module.h>
#include <linux/platform_device.h>

#include <sound/core.h>
#include <sound/pcm.h>
#include <sound/pcm_params.h>
#include <sound/soc.h>

static bool digital_gain_0db_limit = true;

static int snd_allo_piano_dac_init(struct snd_soc_pcm_runtime *rtd)
{
	if (digital_gain_0db_limit) {
		int ret;
		struct snd_soc_card *card = rtd->card;

		ret = snd_soc_limit_volume(card, "Digital Playback Volume",
					   207);
		if (ret < 0)
			dev_warn(card->dev, "Failed to set volume limit: %d\n",
				 ret);
	}

	return 0;
}

SND_SOC_DAILINK_DEFS(allo_piano_dai,
	DAILINK_COMP_ARRAY(COMP_CPU("bcm2708-i2s.0")),
	DAILINK_COMP_ARRAY(COMP_CODEC("pcm512x.1-004c", "pcm512x-hifi")),
	DAILINK_COMP_ARRAY(COMP_PLATFORM("bcm2708-i2s.0")));

static struct snd_soc_dai_link snd_allo_piano_dac_dai[] = {
{
	.name		= "Piano DAC",
	.stream_name	= "Piano DAC HiFi",
	.dai_fmt	= SND_SOC_DAIFMT_I2S |
			  SND_SOC_DAIFMT_NB_NF |
			  SND_SOC_DAIFMT_CBS_CFS,
	.init		= snd_allo_piano_dac_init,
	SND_SOC_DAILINK_REG(allo_piano_dai),
},
};

/* audio machine driver */
static struct snd_soc_card snd_allo_piano_dac = {
	.name         = "PianoDAC",
	.owner        = THIS_MODULE,
	.dai_link     = snd_allo_piano_dac_dai,
	.num_links    = ARRAY_SIZE(snd_allo_piano_dac_dai),
};

static int snd_allo_piano_dac_probe(struct platform_device *pdev)
{
	int ret = 0;

	snd_allo_piano_dac.dev = &pdev->dev;

	if (pdev->dev.of_node) {
		struct device_node *i2s_node;
		struct snd_soc_dai_link *dai;

		dai = &snd_allo_piano_dac_dai[0];
		i2s_node = of_parse_phandle(pdev->dev.of_node,
					    "i2s-controller", 0);

		if (i2s_node) {
			dai->cpus->dai_name = NULL;
			dai->cpus->of_node = i2s_node;
			dai->platforms->name = NULL;
			dai->platforms->of_node = i2s_node;
		}

		digital_gain_0db_limit = !of_property_read_bool(
			pdev->dev.of_node, "allo,24db_digital_gain");
	}

	ret = devm_snd_soc_register_card(&pdev->dev, &snd_allo_piano_dac);
	if (ret && ret != -EPROBE_DEFER)
		dev_err(&pdev->dev,
			"snd_soc_register_card() failed: %d\n", ret);

	return ret;
}

static const struct of_device_id snd_allo_piano_dac_of_match[] = {
	{ .compatible = "allo,piano-dac", },
	{ /* sentinel */ },
};
MODULE_DEVICE_TABLE(of, snd_allo_piano_dac_of_match);

static struct platform_driver snd_allo_piano_dac_driver = {
	.driver = {
		.name   = "snd-allo-piano-dac",
		.owner  = THIS_MODULE,
		.of_match_table = snd_allo_piano_dac_of_match,
	},
	.probe          = snd_allo_piano_dac_probe,
};

module_platform_driver(snd_allo_piano_dac_driver);

MODULE_AUTHOR("Baswaraj K <jaikumar@cem-solutions.net>");
MODULE_DESCRIPTION("ALSA ASoC Machine Driver for Allo Piano DAC");
MODULE_LICENSE("GPL v2");
