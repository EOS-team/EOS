<div align=center>
<img src=https://github.com/Richardhongyu/RROS/assets/33805041/d315241e-5a35-4a36-b0c4-1cd2255b0e0a width=60% />
</div>

--------------------------------------------------------------------------------

[![Documentation](https://img.shields.io/badge/view-docs-blue)](https://BUPT-OS.github.io/website/docs/)
[![.github/workflows/ci.yaml](https://github.com/BUPT-OS/RROS/actions/workflows/ci.yaml/badge.svg)](https://github.com/BUPT-OS/RROS/actions/workflows/ci.yaml)
[![Zulip chat](https://img.shields.io/badge/chat-on%20zulip-brightgreen)](https://rros.zulipchat.com/)
![RROS](https://img.shields.io/badge/RROS-0.0.1-orange)
[![en](https://img.shields.io/badge/lang-en-yellow.svg)](https://github.com/BUPT-OS/RROS/blob/master/README.md)
[![zh](https://img.shields.io/badge/lang-中文-yellow.svg)](https://github.com/BUPT-OS/RROS/blob/master/README.zh.md)

RROS（Rust实时操作系统）是一个双内核操作系统，由实时内核（使用Rust编写）和通用内核（Linux）组成。 RROS几乎可以兼容所有的Linux程序，并提供比RT Linux更好的实时性能。RROS目前正在作为在轨卫星载荷的操作系统进行实验[“天算星座”项目](http://www.tiansuan.org.cn/)。

可以从[这里](https://bupt-os.github.io/website/docs/introduction/架构图.png)看到RROS的架构图和一个[演示视频](https://bupt-os.github.io/website/docs/introduction/demo.mp4)。

## 新闻

- [2024.07.13] 山宇轩在2024 Rust Meetup北京站分享关于RROS的研究。([photos](https://bupt-os.github.io/website/news/2024_07_13/rust_meetup/))
- [2024.07.10] :fire::fire: “Rust-for-Linux 的实证研究：成功、不满和妥协”荣获 USENIX ATC 2024 最佳论文奖!!! ([photos](https://bupt-os.github.io/website/news/2024_07_11/atc2024_bestpaper/))
- [2024.06.15] 李弘宇在第26届中国计算机系统研讨会（ChinaSys）做口头报告。([photos](https://bupt-os.github.io/website/news/2024_06_15/chinasys_26/))
- [2024.03.30] RROS在2024年开源操作系统年度技术大会上亮相。([photos](https://bupt-os.github.io/website/news/2024_03_30/os2atc/))
- [2023.12.09] :fire::fire: RROS 成功升空！ ([照片](https://mp.weixin.qq.com/s/4CukKyJe0OUi04Y4DWiUOQ)).
- [2023.11.30] RROS 在 Xenomai meetup 2023 上展示。（[照片](https://bupt-os.github.io/website/news/2023_11_30/xenomai_workshop/)）。
- [2023.11.28] :fire: RROS 开源了！

## 为什么选择 RROS

RROS 主要针对的是卫星场景（星务计算机、卫星载荷等）。其关键动力是如今卫星既承担传统的卫星实时任务（例如，通信和定位），又需要成熟、复杂的软件支持来执行通用任务（例如，数据压缩和机器学习）。这促使RROS采用了双内核架构。但是比传统的双内核更进一步，RROS 的实时内核完全用 Rust 实现，以提高安全性和鲁棒性。当然，RROS 也可用于自动汽车、物联网、工业控制等场景。


RROS 的优势包括：

* **硬实时**：
相较RT-Linux等软实时操作系统，RROS提供了硬实时能力，能够满足大多数场景的实时需求。通过其高效的任务调度程序，可以快速响应外部事件，减少任务切换和处理的延迟。
* **兼容性**：
RROS 与几乎所有 Linux 程序兼容，允许复杂 Linux 应用程序（如 TensorFlow 和 Kubernetes）的无缝迁移。您也可以轻松地将通用 Linux 程序修改为更实时的对应程序。
* **易于使用**：
RROS 便于实时程序的编程和调试。RROS 使用 libevl 接口调用用户程序的实时 API，允许您使用 gdb、kgdb 和 QEMU 等工具。
* **鲁棒性**：
RROS 的实时内核经过精心编写，使用 Rust，使其在内存和并发问题上更安全、更稳健。

## 快速开始

[RROS极速入门手册](https://bupt-os.github.io/website/docs/introduction/quick-start/)：如何启动、运行、测试和开发 RROS。

## 文档

可以从这里查看我们的[文档](https://bupt-os.github.io/website/docs/)，包括：
* [快速上手](https://bupt-os.github.io/website/docs/introduction/quick-start)
* [配置环境](https://bupt-os.github.io/website/docs/tutorial/setup-the-environment)
* [选择文件系统](https://bupt-os.github.io/website/docs/tutorial/choose-a-file-system)
* [部署到树莓派环境](https://bupt-os.github.io/website/docs/tutorial/deploy-rros-on-the-raspberry-pi) 
* [如何调试RROS](https://bupt-os.github.io/website/docs/tutorial/debug)
* [内核常用工具](https://bupt-os.github.io/website/docs/tutorial/kernel-tools)

## 联系方式与贡献方法

您可以通过 [Zulip 论坛](https://rros.zulipchat.com/) 或电子邮件 `buptrros AT gmail.com` 与我们联系。

我们非常欢迎大家共建社区！详情请看[贡献指南](https://bupt-os.github.io/website/docs/contributing/contributing/)。

## 路线图

查看[路线图](https://bupt-os.github.io/website/docs/roadmap/roadmap)以了解我们未来的规划。

## 我们是谁

我们是北京邮电大学的一个[研究小组](https://bupt-os.github.io/website/docs/team/team/)。

## 发行版

RROS 依赖于 dovetail 和 Rust for Linux(RFL)，目前两者都没有提供补丁。将两者最新的代码高频率地回合到一个项目非常具有挑战性。因此，RROS 暂时被锁定在 Linux 内核版本 5.13 上，这是基于 linux-dovetail-v5.13 构建的，这个版本的linux与 RFL 补丁 v1 兼容。同时幸运的是，RFL 正在迅速进入主线 Linux 内核。我们计划当我们依赖的大部分 RFL API 被合入 linux-dovetail 主线后，我们将切换新的dovetail版本。届时，我们将进一步考虑长期支持版本（LTS）的选择。

## 致谢

RROS 受益于以下项目/资源。
- [Evl/xenomai (linux-evl)](https://evlproject.org/core/)。我们从 evl 内核的代码中学习了如何实现双内核，并使用 dovetail 进行中断虚拟化，并对接[libevl](https://source.denx.de/Xenomai/xenomai4/libevl)作为用户库。感谢xenomai/evl创始人 [Philippe](https://source.denx.de/PhilippeGerum) 的杰出工作和在riot论坛中的耐心答疑！
- [Rust-for-Linux](https://github.com/Rust-for-Linux/linux)：我们使用 RFL 在 Linux 中编写 RROS。我们在 RFL Zulip 上提出了很多如何构建安全抽象层的问题，感谢 RFL 社区的 @ojeda、@Wedson、@Alex、@boqun、@Gary、@Björn 等成员的耐心帮助。希望我们的工作能为RFL贡献更多的安全抽象层！
- [Muduo](https://www.cnblogs.com/wsg1100/p/13836497.html)：他详细的博客让我们对 xenomai/evl 项目有了深入的了解。
- 所有未来对 RROS 的潜在贡献者！

## 引用
```
@misc{li2023rros,
    title = {RROS: A Dual-kernel Real-time Operating System in Rust},
    url = {https://github.com/BUPT-OS/RROS},
    author = {Hongyu Li and Jiangtao Hu and Qichen Qiu and Yuxuan Shan and Bochen Wang and Jiajun Du and Yexuan Yang and Xinge Wang and Shangguang Wang and Mengwei Xu},
    month = {December},
    year = {2023}
}
```

## 许可证

RROS 的源代码遵循 GPL-2.0 许可证。
