<div align=center>
<img src=https://github.com/Richardhongyu/RROS/assets/33805041/d8ccbeb0-c0f5-4634-8108-017367d35a4b width=70% />
</div>

--------------------------------------------------------------------------------

[![Documentation](https://img.shields.io/badge/view-docs-blue)](https://BUPT-OS.github.io/website/docs/)
[![.github/workflows/ci.yaml](https://github.com/BUPT-OS/RROS/actions/workflows/ci.yaml/badge.svg)](https://github.com/BUPT-OS/RROS/actions/workflows/ci.yaml)
[![Zulip chat](https://img.shields.io/badge/chat-on%20zulip-brightgreen)](https://rros.zulipchat.com/)
![RROS](https://img.shields.io/badge/RROS-0.0.1-orange)
[![en](https://img.shields.io/badge/lang-en-yellow.svg)](https://github.com/BUPT-OS/RROS/blob/master/README.md)
[![zh](https://img.shields.io/badge/lang-中文-yellow.svg)](https://github.com/BUPT-OS/RROS/blob/master/README.zh.md)


RROS is a dual-kernel OS, consisting of a real-time kernel (in Rust) and a general-purpose kernel (Linux). RROS is compatible with almost all native Linux programs and offers real-time performance superior to RT-Linux. It is also being experimented with as the host OS for in-orbit satellites ([Tiansuan Project](http://www.tiansuan.org.cn/)).

Here is an [architecture diagram](https://bupt-os.github.io/website/docs/introduction/architecture.jpg) and a [demo video](https://bupt-os.github.io/website/docs/introduction/demo.mp4) of RROS.

## News

- [2024.07.13] Shan Yuxuan shared research on RROS at the 2024 Rust Meetup Beijing. ([photos](https://bupt-os.github.io/website/news/2024_07_13/rust_meetup/))
- [2024.07.10] :fire::fire: “An Empirical Study of Rust-for-Linux: The Success, Dissatisfaction, and Compromise” is presented USENIX ATC 2024 Best Paper Award!!!([photos](https://bupt-os.github.io/website/news/2024_07_11/atc2024_bestpaper/))
- [2024.06.15] Li Hongyu gave an oral talk at the 26th ACM SIGOPS ChinaSys. ([photos](https://bupt-os.github.io/website/news/2024_06_15/chinasys_26/))
- [2024.03.30] RROS is presented in Open Source Operating System Annual Technical Conference 2024. ([photos](https://bupt-os.github.io/website/news/2024_03_30/os2atc/))
- [2023.12.09] :fire::fire: RROS is successfully launched into space! ([photos](https://bupt-os.github.io/website/news/2023_12_9/satellite_launch/)).
- [2023.11.30] RROS is presented at Xenomai Workshop 2023 ([photos](https://bupt-os.github.io/website/news/2023_11_30/xenomai_workshop/)).
- [2023.11.28] :fire: RROS is open-sourced!

## Why RROS

RROS is primarily intended for satellites (onboard computers, payloads, etc). The key incentive is the trend that nowadays satellites serve both traditional satellite-borne real-time tasks (e.g., communication and positioning) and general-purpose tasks that need mature, complicated software support (e.g., data compression and machine learning). That catalyzes the dual-kernel architecture of RROS. Taking a step further, the real-time kernel of RROS is fully implemented in Rust for better safety and robustness. However, RROS can be used in scenarios like automatic cars, IoTs, industrial control, etc.

The advantages of RROS are:

* **Hard real-time**: 
RROS offers superior real-time performance compared to RT-Linux. RROS is designed with an efficient task scheduler that can quickly respond to external events, reducing task switching and processing delays.
* **Compatibility**: 
RROS is compatible with almost every Linux program, allowing seamless migration of complex Linux applications such as TensorFlow and Kubernetes. You can also easily modify your general Linux programs into a more real-time counterpart.
* **Easy to use**: 
RROS facilitates easy programming and debugging of real-time programs. RROS uses the libevl interface to call real-time APIs for user programs, allowing you to use tools like gdb, kgdb, and QEMU.
* **Robustness**:
The real-time kernel of RROS is carefully written in Rust, making it safer and more robust, especially for memory and concurrency issues.

## Quick start

[Kick off with RROS easily](https://bupt-os.github.io/website/docs/introduction/quick-start/): boot, run, test, and develop with RROS.

## Document

See the [document](https://bupt-os.github.io/website/docs/) including:
* [Quick Start](https://bupt-os.github.io/website/docs/introduction/quick-start)
* [Setup the Environment](https://bupt-os.github.io/website/docs/tutorial/setup-the-environment)
* [Choose a file System](https://bupt-os.github.io/website/docs/tutorial/choose-a-file-system)
* [Deploy RROS on the Raspberry Pi](https://bupt-os.github.io/website/docs/tutorial/deploy-rros-on-the-raspberry-pi) 
* [Debug](https://bupt-os.github.io/website/docs/tutorial/debug)
* [Kernel Tools](https://bupt-os.github.io/website/docs/tutorial/kernel-tools)


## Communication & Contribution

Contact us at [Zulip Forum](https://rros.zulipchat.com/) or with email `buptrros AT gmail.com`.

Contributions are also very welcomed! [Check it out](https://bupt-os.github.io/website/docs/contributing/contributing/).

## Roadmap

See [here](https://bupt-os.github.io/website/docs/roadmap/roadmap) for our future roadmap.

## Who are we

We are a [research group](https://bupt-os.github.io/website/docs/team/team/) at BUPT.

## Release

The RROS relies on both the dovetail and the Rust for Linux(RFL), neither of which currently provides patches. Integrating one into the other at a high frequency is challenging. As a result, RROS is currently tied to Linux kernel version 5.13, built on top of linux-dovetail-v5.13, and readily compatible with RFL patch v1. Fortunately, RFL is swiftly making its way into the mainline Linux kernel. We plan to release new versions once most of the RFL APIs we depend on are available in the linux-dovetail mainline. At that point, we will further consider Long-Term Support (LTS) versions.

## Acknowledgements

RROS has benefitted from the following projects/resources.
- [Evl/xenomai (linux-evl)](https://evlproject.org/core/). We learned from evl core how to implement a dual kernel and use dovetail for interrupt virtualization and libevl for user library. Thanks, @Philippe for his genius work and patient explanation in the riot!
- [Rust-for-Linux](https://github.com/Rust-for-Linux/linux): We use RFL to write RROS in Linux. We ask a lot of questions on RFL Zulip and constructing safety abstractions. Kudos to @ojeda, @Wedson,  @Alex, @boqun, @Gary, @Björn in the RFL community for patiently helping us. We hope to bring more safety abstraction back to the RFL in return!
- [Muduo](https://www.cnblogs.com/wsg1100/p/13836497.html): His detailed blog gives us insights for xenomai/evl project.
- All prospective contributors to RROS in the future!

## Citation
```
@misc{li2023rros,
    title = {RROS: A Dual-kernel Real-time Operating System in Rust},
    url = {https://github.com/BUPT-OS/RROS},
    author = {Hongyu Li and Jiangtao Hu and Qichen Qiu and Yuxuan Shan and Bochen Wang and Jiajun Du and Yexuan Yang and Xinge Wang and Shangguang Wang and Mengwei Xu},
    month = {December},
    year = {2023}
}
```

## License

The source code of RROS is under the License of GPL-2.0.
