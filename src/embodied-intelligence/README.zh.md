# <img src="https://github.com/EOS-OS/EOS/blob/main/images/EOS%401x.png" width="130" height="130" alt="EOS">

--------------------------------------------------------------------------------

## 介绍

**EOS** 是基于双内核实时操作系统 [RROS](https://github.com/BUPT-OS/RROS) 发布的具身智能操作系统。其目标是构建一个**易于使用的平台，收集所有创建智能机器人应用所需的软件**。具体来说，我们将通过三个步骤来实现这个目标：
   - 构建一个机器人包管理器，用于收集相关的库/框架/算法
   - 提升 RROS 在机器人开发中的实时能力
   - 基于 RROS 优化包管理平台中算法库的实时能力

![image](https://github.com/user-attachments/assets/2dfb8b35-b065-4fa8-8d0c-7c3e72c942be)


## 使用

[使用系统的说明将放在这里。]

## 贡献

[为项目做出贡献的指南将放在这里。]
### 在ROS中使用EOS仓库的包
我们目前提供一个 COPR 平台用于构建 RPM 包，未来将支持 Deb 和其他软件包格式。以 Fedora 为例，您可以通过以下步骤将 EOS 仓库添加到系统中：

```bash
sudo wget -O /etc/yum.repos.d/stevenfreeto-hello-eos-fedora-39.repo http://eos.eaishow.com:9250/coprs/stevenfreeto/hello-eos/repo/fedora-39/stevenfreeto-hello-eos-fedora-39.repo
sudo dnf makecache
# sudo yum makecache    # 如果您使用 yum，使用此命令
```

此外，您还需要将 EOS 的 rosdep 添加到 ROS 环境中。编辑 /etc/ros/rosdep/sources.list.d/20-default.list 配置文件，并添加以下内容：

```bash
yaml https://raw.githubusercontent.com/EOS-OS/EOS/main/package-manager/rosdep/base.yaml
```
当您使用rosdep时，若依赖的包归属于EOS，发行版的包管理器会首先从EOS的源中寻找，若没有找到，则从ROS的源中寻找。

## 我们在哪里

你可以在 [EOS Zulip](https://eos24.zulipchat.com/join/lnwy7yspqiiu4hqqlat45vlv/) 或发送邮件至 `eosrros AT gmail.com` 联系我们。

## 我们是谁

我们是一群来自研究机构、学校和机器人企业的机器人开发者。我们希望 `EOS` 能够团结来自不同领域的力量，加速机器人开发，并缩短开发者与最终用户之间的沟通路径。

## 路线图

![image](https://github.com/user-attachments/assets/c168d4d0-1ef5-49b0-baa6-bb8c53914a69)

1. **建立机器人开发的包管理工具**
   - [ ] 包镜像源
   - [ ] 包管理客户端
     - [X] **内核层**：集成核心系统库，确保系统的稳定性和兼容性。
        - [X] 基于 `Copr` 的平台来收集软件包
     - [ ] **EI 翻译器（具身智能翻译器）**：将内核层的库转换为中间件层的内部库，促进跨多个系统层（包括 ROS/Dora）的无缝集成。
     - [ ] **中间件层**：
       - [ ] **ROS**：提供机器人操作系统的中间件支持，增强自动化和机器人开发能力。
       - [ ] **Dora**：提供分布式系统的高级中间件支持，增强系统的可扩展性和灵活性。
2. **使用 RROS 内核增强机器人开发的实时能力**
   - [ ] 实时能力
     - [ ] 适配 ROS
     - [ ] 适配 Dora
     - [ ] 适配 Ethercat 协议
   - [ ] 对国产芯片的适配和优化
     - [X] x86 系列
     - [X] ARM 系列
     - [X] LoongArch（目前支持单CPU核心）
     - [ ] RISC-V
3. **基于 RROS 内核开发上层应用**
   - [ ] 提供丰富的 API，支持应用开发者充分利用 RROS 内核的强大功能，加速应用的开发和部署。
   - [ ] 使用 RROS API 加速应用。
