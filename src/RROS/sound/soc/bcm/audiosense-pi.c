/*
 * ASoC Driver for AudioSense add on soundcard
 * Author:
 *	Bhargav A K <anur.bhargav@gmail.com>
 *	Copyright 2017
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
#include <linux/clk.h>
#include <linux/i2c.h>
#include <sound/core.h>
#include <sound/pcm.h>
#include <sound/pcm_params.h>
#include <sound/soc.h>
#include <sound/jack.h>
#include <sound/control.h>

#include <sound/tlv320aic32x4.h>
#include "../codecs/tlv320aic32x4.h"

#define AIC32X4_SYSCLK_XTAL	0x00

/*
 * Setup Codec Sample Rates and Channels
 * Supported Rates:
 *	8000, 11025, 16000, 22050, 32000, 44100, 48000, 64000, 88200, 96000,
 */
static const unsigned int audiosense_pi_rate[] = {
	48000,
};

static struct snd_pcm_hw_constraint_list audiosense_constraints_rates = {
	.list = audiosense_pi_rate,
	.count = ARRAY_SIZE(audiosense_pi_rate),
};

static const unsigned int audiosense_pi_channels[] = {
	2,
};

static struct snd_pcm_hw_constraint_list audiosense_constraints_ch = {
	.count = ARRAY_SIZE(audiosense_pi_channels),
	.list = audiosense_pi_channels,
	.mask = 0,
};

/* Setup DAPM widgets and paths */
static const struct snd_soc_dapm_widget audiosense_pi_dapm_widgets[] = {
	SND_SOC_DAPM_HP("Headphone Jack", NULL),
	SND_SOC_DAPM_LINE("Line Out", NULL),
	SND_SOC_DAPM_LINE("Line In", NULL),
	SND_SOC_DAPM_INPUT("CM_L"),
	SND_SOC_DAPM_INPUT("CM_R"),
};

static const struct snd_soc_dapm_route audiosense_pi_audio_map[] = {
	/* Line Inputs are connected to
	 * (IN1_L | IN1_R)
	 * (IN2_L | IN2_R)
	 * (IN3_L | IN3_R)
	 */
	{"IN1_L", NULL, "Line In"},
	{"IN1_R", NULL, "Line In"},
	{"IN2_L", NULL, "Line In"},
	{"IN2_R", NULL, "Line In"},
	{"IN3_L", NULL, "Line In"},
	{"IN3_R", NULL, "Line In"},

	/* Mic is connected to IN2_L and IN2_R */
	{"Left ADC", NULL, "Mic Bias"},
	{"Right ADC", NULL, "Mic Bias"},

	/* Headphone connected to HPL, HPR */
	{"Headphone Jack", NULL, "HPL"},
	{"Headphone Jack", NULL, "HPR"},

	/* Speakers connected to LOL and LOR */
	{"Line Out", NULL, "LOL"},
	{"Line Out", NULL, "LOR"},
};

static int audiosense_pi_card_init(struct snd_soc_pcm_runtime *rtd)
{
	/* TODO: init of the codec specific dapm data, ignore suspend/resume */
	struct snd_soc_component *component = asoc_rtd_to_codec(rtd, 0)->component;

	snd_soc_component_update_bits(component, AIC32X4_MICBIAS, 0x78,
				      AIC32X4_MICBIAS_LDOIN |
				      AIC32X4_MICBIAS_2075V);
	snd_soc_component_update_bits(component, AIC32X4_PWRCFG, 0x08,
				      AIC32X4_AVDDWEAKDISABLE);
	snd_soc_component_update_bits(component, AIC32X4_LDOCTL, 0x01,
				      AIC32X4_LDOCTLEN);

	return 0;
}

static int audiosense_pi_card_hw_params(
		struct snd_pcm_substream *substream,
		struct snd_pcm_hw_params *params)
{
	int ret = 0;
	struct snd_soc_pcm_runtime *rtd = substream->private_data;
	struct snd_soc_dai *codec_dai = asoc_rtd_to_codec(rtd, 0);

	/* Set the codec system clock, there is a 12 MHz XTAL on the board */
	ret = snd_soc_dai_set_sysclk(codec_dai, AIC32X4_SYSCLK_XTAL,
				     12000000, SND_SOC_CLOCK_IN);
	if (ret) {
		dev_err(rtd->card->dev,
			"could not set codec driver clock params\n");
		return ret;
	}
	return 0;
}

static int audiosense_pi_card_startup(struct snd_pcm_substream *substream)
{
	struct snd_pcm_runtime *runtime = substream->runtime;

	/*
	 * Set codec to 48Khz Sampling, Stereo I/O and 16 bit audio
	 */
	runtime->hw.channels_max = 2;
	snd_pcm_hw_constraint_list(runtime, 0, SNDRV_PCM_HW_PARAM_CHANNELS,
				   &audiosense_constraints_ch);

	runtime->hw.formats = SNDRV_PCM_FMTBIT_S16_LE;
	snd_pcm_hw_constraint_msbits(runtime, 0, 16, 16);


	snd_pcm_hw_constraint_list(substream->runtime, 0,
				   SNDRV_PCM_HW_PARAM_RATE,
				   &audiosense_constraints_rates);
	return 0;
}

static struct snd_soc_ops audiosense_pi_card_ops = {
	.startup = audiosense_pi_card_startup,
	.hw_params = audiosense_pi_card_hw_params,
};

SND_SOC_DAILINK_DEFS(audiosense_pi,
	DAILINK_COMP_ARRAY(COMP_CPU("bcm2708-i2s.0")),
	DAILINK_COMP_ARRAY(COMP_CODEC("tlv320aic32x4.1-0018", "tlv320aic32x4-hifi")),
	DAILINK_COMP_ARRAY(COMP_PLATFORM("bcm2708-i2s.0")));

static struct snd_soc_dai_link audiosense_pi_card_dai[] = {
	{
		.name           = "TLV320AIC3204 Audio",
		.stream_name    = "TLV320AIC3204 Hifi Audio",
		.dai_fmt        = SND_SOC_DAIFMT_I2S | SND_SOC_DAIFMT_NB_NF |
			SND_SOC_DAIFMT_CBM_CFM,
		.ops            = &audiosense_pi_card_ops,
		.init           = audiosense_pi_card_init,
		SND_SOC_DAILINK_REG(audiosense_pi),
	},
};

static struct snd_soc_card audiosense_pi_card = {
	.name		= "audiosense-pi",
	.driver_name	= "audiosense-pi",
	.dai_link	= audiosense_pi_card_dai,
	.owner		= THIS_MODULE,
	.num_links	= ARRAY_SIZE(audiosense_pi_card_dai),
	.dapm_widgets	= audiosense_pi_dapm_widgets,
	.num_dapm_widgets = ARRAY_SIZE(audiosense_pi_dapm_widgets),
	.dapm_routes	= audiosense_pi_audio_map,
	.num_dapm_routes = ARRAY_SIZE(audiosense_pi_audio_map),
};

static int audiosense_pi_card_probe(struct platform_device *pdev)
{
	int ret = 0;
	struct snd_soc_card *card = &audiosense_pi_card;
	struct snd_soc_dai_link *dai = &audiosense_pi_card_dai[0];
	struct device_node *i2s_node = pdev->dev.of_node;

	card->dev = &pdev->dev;

	if (!dai) {
		dev_err(&pdev->dev, "DAI not found. Missing or Invalid\n");
		return -EINVAL;
	}

	i2s_node = of_parse_phandle(pdev->dev.of_node, "i2s-controller", 0);
	if (!i2s_node) {
		dev_err(&pdev->dev,
			"Property 'i2s-controller' missing or invalid\n");
		return -EINVAL;
	}

	dai->cpus->dai_name = NULL;
	dai->cpus->of_node = i2s_node;
	dai->platforms->name = NULL;
	dai->platforms->of_node = i2s_node;

	of_node_put(i2s_node);

	ret = snd_soc_register_card(card);
	if (ret && ret != -EPROBE_DEFER)
		dev_err(&pdev->dev,
			"snd_soc_register_card() failed: %d\n", ret);

	return ret;
}

static int audiosense_pi_card_remove(struct platform_device *pdev)
{
	struct snd_soc_card *card = platform_get_drvdata(pdev);

	return snd_soc_unregister_card(card);

}

static const struct of_device_id audiosense_pi_card_of_match[] = {
	{ .compatible = "as,audiosense-pi", },
	{},
};
MODULE_DEVICE_TABLE(of, audiosense_pi_card_of_match);

static struct platform_driver audiosense_pi_card_driver = {
	.driver = {
		.name = "audiosense-snd-card",
		.owner = THIS_MODULE,
		.of_match_table = audiosense_pi_card_of_match,
	},
	.probe = audiosense_pi_card_probe,
	.remove = audiosense_pi_card_remove,
};

module_platform_driver(audiosense_pi_card_driver);

MODULE_AUTHOR("Bhargav AK <anur.bhargav@gmail.com>");
MODULE_DESCRIPTION("ASoC Driver for TLV320AIC3204 Audio");
MODULE_LICENSE("GPL v2");
MODULE_ALIAS("platform:audiosense-pi");

