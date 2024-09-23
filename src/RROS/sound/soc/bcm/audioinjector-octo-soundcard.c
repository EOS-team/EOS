/*
 * ASoC Driver for AudioInjector Pi octo channel soundcard (hat)
 *
 *  Created on: 27-October-2016
 *      Author: flatmax@flatmax.org
 *              based on audioinjector-pi-soundcard.c
 *
 * Copyright (C) 2016 Flatmax Pty. Ltd.
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
#include <linux/types.h>
#include <linux/gpio/consumer.h>

#include <sound/core.h>
#include <sound/soc.h>
#include <sound/pcm_params.h>
#include <sound/control.h>

static struct gpio_descs *mult_gpios;
static struct gpio_desc *codec_rst_gpio;
static unsigned int audioinjector_octo_rate;
static bool non_stop_clocks;

static const unsigned int audioinjector_octo_rates[] = {
	96000, 48000, 32000, 24000, 16000, 8000, 88200, 44100, 29400, 22050, 14700,
};

static struct snd_pcm_hw_constraint_list audioinjector_octo_constraints = {
	.list = audioinjector_octo_rates,
	.count = ARRAY_SIZE(audioinjector_octo_rates),
};

static int audioinjector_octo_dai_init(struct snd_soc_pcm_runtime *rtd)
{
	return snd_soc_dai_set_bclk_ratio(asoc_rtd_to_cpu(rtd, 0), 64);
}

static int audioinjector_octo_startup(struct snd_pcm_substream *substream)
{
	struct snd_soc_pcm_runtime *rtd = substream->private_data;
	asoc_rtd_to_cpu(rtd, 0)->driver->playback.channels_min = 8;
	asoc_rtd_to_cpu(rtd, 0)->driver->playback.channels_max = 8;
	asoc_rtd_to_cpu(rtd, 0)->driver->capture.channels_min = 8;
	asoc_rtd_to_cpu(rtd, 0)->driver->capture.channels_max = 8;
	asoc_rtd_to_codec(rtd, 0)->driver->capture.channels_max = 8;

	snd_pcm_hw_constraint_list(substream->runtime, 0,
				SNDRV_PCM_HW_PARAM_RATE,
				&audioinjector_octo_constraints);

	return 0;
}

static void audioinjector_octo_shutdown(struct snd_pcm_substream *substream)
{
	struct snd_soc_pcm_runtime *rtd = substream->private_data;
	asoc_rtd_to_cpu(rtd, 0)->driver->playback.channels_min = 2;
	asoc_rtd_to_cpu(rtd, 0)->driver->playback.channels_max = 2;
	asoc_rtd_to_cpu(rtd, 0)->driver->capture.channels_min = 2;
	asoc_rtd_to_cpu(rtd, 0)->driver->capture.channels_max = 2;
	asoc_rtd_to_codec(rtd, 0)->driver->capture.channels_max = 6;
}

static int audioinjector_octo_hw_params(struct snd_pcm_substream *substream,
	struct snd_pcm_hw_params *params)
{
	struct snd_soc_pcm_runtime *rtd = substream->private_data;

	// set codec DAI configuration
	int ret = snd_soc_dai_set_fmt(asoc_rtd_to_codec(rtd, 0),
			SND_SOC_DAIFMT_CBS_CFS|SND_SOC_DAIFMT_DSP_A|
			SND_SOC_DAIFMT_NB_NF);
	if (ret < 0)
		return ret;

	// set cpu DAI configuration
	ret = snd_soc_dai_set_fmt(asoc_rtd_to_cpu(rtd, 0),
			SND_SOC_DAIFMT_CBM_CFM|SND_SOC_DAIFMT_I2S|
			SND_SOC_DAIFMT_NB_NF);
	if (ret < 0)
		return ret;

	audioinjector_octo_rate = params_rate(params);

	// Set the correct sysclock for the codec
	switch (audioinjector_octo_rate) {
	case 96000:
	case 48000:
		return snd_soc_dai_set_sysclk(asoc_rtd_to_codec(rtd, 0), 0, 49152000,
									0);
		break;
	case 24000:
		return snd_soc_dai_set_sysclk(asoc_rtd_to_codec(rtd, 0), 0, 49152000/2,
									0);
		break;
	case 32000:
	case 16000:
		return snd_soc_dai_set_sysclk(asoc_rtd_to_codec(rtd, 0), 0, 49152000/3,
									0);
		break;
	case 8000:
		return snd_soc_dai_set_sysclk(asoc_rtd_to_codec(rtd, 0), 0, 49152000/6,
									0);
		break;
	case 88200:
	case 44100:
		return snd_soc_dai_set_sysclk(asoc_rtd_to_codec(rtd, 0), 0, 45185400,
									0);
		break;
	case 22050:
		return snd_soc_dai_set_sysclk(asoc_rtd_to_codec(rtd, 0), 0, 45185400/2,
									0);
		break;
	case 29400:
	case 14700:
		return snd_soc_dai_set_sysclk(asoc_rtd_to_codec(rtd, 0), 0, 45185400/3,
									0);
		break;
	default:
		return -EINVAL;
	}
}

static int audioinjector_octo_trigger(struct snd_pcm_substream *substream,
								int cmd){
	DECLARE_BITMAP(mult, 4);

	memset(mult, 0, sizeof(mult));

	switch (cmd) {
	case SNDRV_PCM_TRIGGER_STOP:
	case SNDRV_PCM_TRIGGER_SUSPEND:
	case SNDRV_PCM_TRIGGER_PAUSE_PUSH:
		if (!non_stop_clocks)
			break;
		/* fall through */
	case SNDRV_PCM_TRIGGER_START:
	case SNDRV_PCM_TRIGGER_RESUME:
	case SNDRV_PCM_TRIGGER_PAUSE_RELEASE:
		switch (audioinjector_octo_rate) {
		case 96000:
			__assign_bit(3, mult, 1);
			/* fall through */
		case 88200:
			__assign_bit(1, mult, 1);
			__assign_bit(2, mult, 1);
			break;
		case 48000:
			__assign_bit(3, mult, 1);
			/* fall through */
		case 44100:
			__assign_bit(2, mult, 1);
			break;
		case 32000:
			__assign_bit(3, mult, 1);
			/* fall through */
		case 29400:
			__assign_bit(0, mult, 1);
			__assign_bit(1, mult, 1);
			break;
		case 24000:
			__assign_bit(3, mult, 1);
			/* fall through */
		case 22050:
			__assign_bit(1, mult, 1);
			break;
		case 16000:
			__assign_bit(3, mult, 1);
			/* fall through */
		case 14700:
			__assign_bit(0, mult, 1);
			break;
		case 8000:
			__assign_bit(3, mult, 1);
			break;
		default:
			return -EINVAL;
		}
		break;
	default:
		return -EINVAL;
	}
	gpiod_set_array_value_cansleep(mult_gpios->ndescs, mult_gpios->desc,
				       NULL, mult);

	return 0;
}

static struct snd_soc_ops audioinjector_octo_ops = {
	.startup	= audioinjector_octo_startup,
	.shutdown	= audioinjector_octo_shutdown,
	.hw_params = audioinjector_octo_hw_params,
	.trigger = audioinjector_octo_trigger,
};

SND_SOC_DAILINK_DEFS(audioinjector_octo,
	DAILINK_COMP_ARRAY(COMP_EMPTY()),
	DAILINK_COMP_ARRAY(COMP_CODEC(NULL, "cs42448")),
	DAILINK_COMP_ARRAY(COMP_EMPTY()));

static struct snd_soc_dai_link audioinjector_octo_dai[] = {
	{
		.name = "AudioInjector Octo",
		.stream_name = "AudioInject-HIFI",
		.ops = &audioinjector_octo_ops,
		.init = audioinjector_octo_dai_init,
		.symmetric_rate = 1,
		.symmetric_channels = 1,
		SND_SOC_DAILINK_REG(audioinjector_octo),
	},
};

static const struct snd_soc_dapm_widget audioinjector_octo_widgets[] = {
	SND_SOC_DAPM_OUTPUT("OUTPUTS0"),
	SND_SOC_DAPM_OUTPUT("OUTPUTS1"),
	SND_SOC_DAPM_OUTPUT("OUTPUTS2"),
	SND_SOC_DAPM_OUTPUT("OUTPUTS3"),
	SND_SOC_DAPM_INPUT("INPUTS0"),
	SND_SOC_DAPM_INPUT("INPUTS1"),
	SND_SOC_DAPM_INPUT("INPUTS2"),
};

static const struct snd_soc_dapm_route audioinjector_octo_route[] = {
	/* Balanced outputs */
	{"OUTPUTS0", NULL, "AOUT1L"},
	{"OUTPUTS0", NULL, "AOUT1R"},
	{"OUTPUTS1", NULL, "AOUT2L"},
	{"OUTPUTS1", NULL, "AOUT2R"},
	{"OUTPUTS2", NULL, "AOUT3L"},
	{"OUTPUTS2", NULL, "AOUT3R"},
	{"OUTPUTS3", NULL, "AOUT4L"},
	{"OUTPUTS3", NULL, "AOUT4R"},

	/* Balanced inputs */
	{"AIN1L", NULL, "INPUTS0"},
	{"AIN1R", NULL, "INPUTS0"},
	{"AIN2L", NULL, "INPUTS1"},
	{"AIN2R", NULL, "INPUTS1"},
	{"AIN3L", NULL, "INPUTS2"},
	{"AIN3R", NULL, "INPUTS2"},
};

static struct snd_soc_card snd_soc_audioinjector_octo = {
	.name = "audioinjector-octo-soundcard",
	.dai_link = audioinjector_octo_dai,
	.num_links = ARRAY_SIZE(audioinjector_octo_dai),

	.dapm_widgets = audioinjector_octo_widgets,
	.num_dapm_widgets = ARRAY_SIZE(audioinjector_octo_widgets),
	.dapm_routes = audioinjector_octo_route,
	.num_dapm_routes = ARRAY_SIZE(audioinjector_octo_route),
};

static int audioinjector_octo_probe(struct platform_device *pdev)
{
	struct snd_soc_card *card = &snd_soc_audioinjector_octo;
	int ret;

	card->dev = &pdev->dev;

	if (pdev->dev.of_node) {
		struct snd_soc_dai_link *dai = &audioinjector_octo_dai[0];
		struct device_node *i2s_node =
					of_parse_phandle(pdev->dev.of_node,
							"i2s-controller", 0);
		struct device_node *codec_node =
					of_parse_phandle(pdev->dev.of_node,
								"codec", 0);

		mult_gpios = devm_gpiod_get_array_optional(&pdev->dev, "mult",
								GPIOD_OUT_LOW);
		if (IS_ERR(mult_gpios))
			return PTR_ERR(mult_gpios);

		codec_rst_gpio = devm_gpiod_get_optional(&pdev->dev, "reset",
								GPIOD_OUT_LOW);
		if (IS_ERR(codec_rst_gpio))
			return PTR_ERR(codec_rst_gpio);

		non_stop_clocks = of_property_read_bool(pdev->dev.of_node, "non-stop-clocks");

		if (codec_rst_gpio)
			gpiod_set_value(codec_rst_gpio, 1);
		msleep(500);
		if (codec_rst_gpio)
			gpiod_set_value(codec_rst_gpio, 0);
		msleep(500);
		if (codec_rst_gpio)
			gpiod_set_value(codec_rst_gpio, 1);
		msleep(500);

		if (i2s_node && codec_node) {
			dai->cpus->dai_name = NULL;
			dai->cpus->of_node = i2s_node;
			dai->platforms->name = NULL;
			dai->platforms->of_node = i2s_node;
			dai->codecs->name = NULL;
			dai->codecs->of_node = codec_node;
		} else
			if (!i2s_node) {
				dev_err(&pdev->dev,
				"i2s-controller missing or invalid in DT\n");
				return -EINVAL;
			} else {
				dev_err(&pdev->dev,
				"Property 'codec' missing or invalid\n");
				return -EINVAL;
			}
	}

	ret = devm_snd_soc_register_card(&pdev->dev, card);
	if (ret != 0)
		dev_err(&pdev->dev, "snd_soc_register_card failed (%d)\n", ret);
	return ret;
}

static const struct of_device_id audioinjector_octo_of_match[] = {
	{ .compatible = "ai,audioinjector-octo-soundcard", },
	{},
};
MODULE_DEVICE_TABLE(of, audioinjector_octo_of_match);

static struct platform_driver audioinjector_octo_driver = {
	.driver	= {
		.name			= "audioinjector-octo",
		.owner			= THIS_MODULE,
		.of_match_table = audioinjector_octo_of_match,
	},
	.probe	= audioinjector_octo_probe,
};

module_platform_driver(audioinjector_octo_driver);
MODULE_AUTHOR("Matt Flax <flatmax@flatmax.org>");
MODULE_DESCRIPTION("AudioInjector.net octo Soundcard");
MODULE_LICENSE("GPL v2");
MODULE_ALIAS("platform:audioinjector-octo-soundcard");
