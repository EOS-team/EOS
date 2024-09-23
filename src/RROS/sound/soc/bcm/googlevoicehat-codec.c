/*
 * Driver for the Google voiceHAT audio codec for Raspberry Pi.
 *
 * Author:	Peter Malkin <petermalkin@google.com>
 *		Copyright 2016
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

#include <linux/device.h>
#include <linux/err.h>
#include <linux/gpio.h>
#include <linux/gpio/consumer.h>
#include <linux/init.h>
#include <linux/kernel.h>
#include <linux/mod_devicetable.h>
#include <linux/module.h>
#include <linux/of.h>
#include <linux/platform_device.h>
#include <linux/version.h>
#include <sound/pcm.h>
#include <sound/soc.h>
#include <sound/soc-dai.h>
#include <sound/soc-dapm.h>

#define ICS43432_RATE_MIN_HZ	7190  /* from data sheet */
#define ICS43432_RATE_MAX_HZ	52800 /* from data sheet */
/* Delay in enabling SDMODE after clock settles to remove pop */
#define SDMODE_DELAY_MS		5

struct voicehat_priv {
	struct delayed_work enable_sdmode_work;
	struct gpio_desc *sdmode_gpio;
	unsigned long sdmode_delay_jiffies;
};

static void voicehat_enable_sdmode_work(struct work_struct *work)
{
	struct voicehat_priv *voicehat = container_of(work,
						      struct voicehat_priv,
						      enable_sdmode_work.work);
	gpiod_set_value(voicehat->sdmode_gpio, 1);
}

static int voicehat_component_probe(struct snd_soc_component *component)
{
	struct voicehat_priv *voicehat =
				snd_soc_component_get_drvdata(component);

	voicehat->sdmode_gpio = devm_gpiod_get(component->dev, "sdmode",
					       GPIOD_OUT_LOW);
	if (IS_ERR(voicehat->sdmode_gpio)) {
		dev_err(component->dev, "Unable to allocate GPIO pin\n");
		return PTR_ERR(voicehat->sdmode_gpio);
	}

	INIT_DELAYED_WORK(&voicehat->enable_sdmode_work,
			  voicehat_enable_sdmode_work);
	return 0;
}

static void voicehat_component_remove(struct snd_soc_component *component)
{
	struct voicehat_priv *voicehat =
				snd_soc_component_get_drvdata(component);

	cancel_delayed_work_sync(&voicehat->enable_sdmode_work);
}

static const struct snd_soc_dapm_widget voicehat_dapm_widgets[] = {
	SND_SOC_DAPM_OUTPUT("Speaker"),
};

static const struct snd_soc_dapm_route voicehat_dapm_routes[] = {
	{"Speaker", NULL, "HiFi Playback"},
};

static const struct snd_soc_component_driver voicehat_component_driver = {
	.probe = voicehat_component_probe,
	.remove = voicehat_component_remove,
	.dapm_widgets = voicehat_dapm_widgets,
	.num_dapm_widgets = ARRAY_SIZE(voicehat_dapm_widgets),
	.dapm_routes = voicehat_dapm_routes,
	.num_dapm_routes = ARRAY_SIZE(voicehat_dapm_routes),
};

static int voicehat_daiops_trigger(struct snd_pcm_substream *substream, int cmd,
				   struct snd_soc_dai *dai)
{
	struct snd_soc_component *component = dai->component;
	struct voicehat_priv *voicehat =
				snd_soc_component_get_drvdata(component);

	if (voicehat->sdmode_delay_jiffies == 0)
		return 0;

	dev_dbg(dai->dev, "CMD             %d", cmd);
	dev_dbg(dai->dev, "Playback Active %d", dai->stream_active[SNDRV_PCM_STREAM_PLAYBACK]);
	dev_dbg(dai->dev, "Capture Active  %d", dai->stream_active[SNDRV_PCM_STREAM_CAPTURE]);

	switch (cmd) {
	case SNDRV_PCM_TRIGGER_START:
	case SNDRV_PCM_TRIGGER_RESUME:
	case SNDRV_PCM_TRIGGER_PAUSE_RELEASE:
		if (dai->stream_active[SNDRV_PCM_STREAM_PLAYBACK]) {
			dev_info(dai->dev, "Enabling audio amp...\n");
			queue_delayed_work(
				system_power_efficient_wq,
				&voicehat->enable_sdmode_work,
				voicehat->sdmode_delay_jiffies);
		}
		break;
	case SNDRV_PCM_TRIGGER_STOP:
	case SNDRV_PCM_TRIGGER_SUSPEND:
	case SNDRV_PCM_TRIGGER_PAUSE_PUSH:
		if (dai->stream_active[SNDRV_PCM_STREAM_PLAYBACK]) {
			cancel_delayed_work(&voicehat->enable_sdmode_work);
			dev_info(dai->dev, "Disabling audio amp...\n");
			gpiod_set_value(voicehat->sdmode_gpio, 0);
		}
		break;
	}
	return 0;
}

static const struct snd_soc_dai_ops voicehat_dai_ops = {
	.trigger = voicehat_daiops_trigger,
};

static struct snd_soc_dai_driver voicehat_dai = {
	.name = "voicehat-hifi",
	.capture = {
		.stream_name = "HiFi Capture",
		.channels_min = 2,
		.channels_max = 2,
		.rates = SNDRV_PCM_RATE_48000,
		.formats = SNDRV_PCM_FMTBIT_S32_LE
	},
	.playback = {
		.stream_name = "HiFi Playback",
		.channels_min = 2,
		.channels_max = 2,
		.rates = SNDRV_PCM_RATE_48000,
		.formats = SNDRV_PCM_FMTBIT_S32_LE
	},
	.ops = &voicehat_dai_ops,
	.symmetric_rate = 1
};

#ifdef CONFIG_OF
static const struct of_device_id voicehat_ids[] = {
		{ .compatible = "google,voicehat", }, {}
	};
	MODULE_DEVICE_TABLE(of, voicehat_ids);
#endif

static int voicehat_platform_probe(struct platform_device *pdev)
{
	struct voicehat_priv *voicehat;
	unsigned int sdmode_delay;
	int ret;

	voicehat = devm_kzalloc(&pdev->dev, sizeof(*voicehat), GFP_KERNEL);
	if (!voicehat)
		return -ENOMEM;

	ret = device_property_read_u32(&pdev->dev, "voicehat_sdmode_delay",
				       &sdmode_delay);

	if (ret) {
		sdmode_delay = SDMODE_DELAY_MS;
		dev_info(&pdev->dev,
			 "property 'voicehat_sdmode_delay' not found default 5 mS");
	} else {
		dev_info(&pdev->dev, "property 'voicehat_sdmode_delay' found delay= %d mS",
			 sdmode_delay);
	}
	voicehat->sdmode_delay_jiffies = msecs_to_jiffies(sdmode_delay);

	dev_set_drvdata(&pdev->dev, voicehat);

	return snd_soc_register_component(&pdev->dev,
					  &voicehat_component_driver,
					  &voicehat_dai,
					  1);
}

static int voicehat_platform_remove(struct platform_device *pdev)
{
	snd_soc_unregister_component(&pdev->dev);
	return 0;
}

static struct platform_driver voicehat_driver = {
	.driver = {
		.name = "voicehat-codec",
		.of_match_table = of_match_ptr(voicehat_ids),
	},
	.probe = voicehat_platform_probe,
	.remove = voicehat_platform_remove,
};

module_platform_driver(voicehat_driver);

MODULE_DESCRIPTION("Google voiceHAT Codec driver");
MODULE_AUTHOR("Peter Malkin <petermalkin@google.com>");
MODULE_LICENSE("GPL v2");
