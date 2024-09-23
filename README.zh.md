# <img src="https://github.com/EOS-team/EOS/blob/main/images/EOS%401x.png" width="130" height="130" alt="EOS">

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

### 如何贡献您的软件包到用户仓
EOS 用户软件仓目前使用Copr项目构建。如果您已经很熟悉Copr，可以挑过这部分教程直接使用EOS用户软件仓。
#### 准备工作
EOS的用户仓基于fedora-copr项目构建，您可以通过copr官方文档获得对copr使用更详细的指导。
首先注册一个fedora账号[Fedora Accounts (fedoraproject.org)](https://accounts.fedoraproject.org/)。
进入用户软件仓[首页](https://eos.eaishow.com)，使用fedora ID进行登录。

<img width="1616" alt="1725976528436" src="https://github.com/user-attachments/assets/17b0fdad-509e-4f7a-bcc5-7e62e71a45ac">

登录成功后进入个人界面：

![image-20240910215916777](https://github.com/user-attachments/assets/307899c3-a23c-42b1-9072-05b07447e40b)


#### 新建项目
点击 `New Project` 按钮创建新项目，填写项目名(Project Name)，选择需要的软件包构建环境(Chroots)，并根据需要配置其他可选项。

![image-20240910220838008](https://github.com/user-attachments/assets/4b52d63c-782b-4a16-ac2e-dc04cfc275d1)

配置完成后点击 `Create` 创建项目。

<img width="937" alt="1725977723240" src="https://github.com/user-attachments/assets/dafe396c-bbd0-4e64-9182-d0d5fdcee720">

#### 构建软件包
在项目主页选择 `Builds` ，点击 `New Build` 。

<img width="609" alt="1725978624151" src="https://github.com/user-attachments/assets/856d0a0e-f512-47d8-9a2e-17a09d0028b4">

这里我们直接通过上传 `sprm` 文件的方式进行构建，在 `Upload` 处点击 `Browse` 按钮上传本地 `srpm` 文件。如果您不了解如何构建 `sprm` 包，可以查看该[指导](https://rpm-packaging-guide.github.io/)获取更多细节。

<img width="596" alt="1725978723980" src="https://github.com/user-attachments/assets/09bc7083-e9d6-45dd-a93f-5b390a56a029">

选择本地 `sprm` 文件，点击 `Open` 进行上传。

![image-20240910223352577](https://github.com/user-attachments/assets/87f7e1f4-a45e-4b61-b62e-4c11ec4deb05)

最后点击 `Build` 开始构建软件包。

<img width="620" alt="1725978912129" src="https://github.com/user-attachments/assets/5d7fed52-6224-45d6-b51c-a9b2199301cb">

构建成功。

<img width="628" alt="1725979056196" src="https://github.com/user-attachments/assets/fafc6eab-ce13-4a2e-95ea-a6ae23af576f">

点击对应的 `Build ID` 可以看到构建结果。

<img width="629" alt="1725979186212" src="https://github.com/user-attachments/assets/359aa4b9-7661-4ae7-812f-b0306467afce">

![image-20240910224043177](https://github.com/user-attachments/assets/f8f7e09e-f3c4-423c-9976-9590a56f4ec4)

Copr还支持其他多种方式构建您的软件包，更多细节请查看[官方文档](https://docs.pagure.org/copr.copr/index.html)。

#### 使用个人软件仓
首先回到项目首页，获取对应版本的仓库配置文件。点击版本号。

<img width="984" alt="1725979554852" src="https://github.com/user-attachments/assets/3e2191fd-70da-4b2e-bea8-3657d2ff8c1e">

复制链接。

<img width="643" alt="1725979632482" src="https://github.com/user-attachments/assets/9a792acf-574e-4924-b2cc-7ac060dceba4">

进入 `/etc/yum/repos.d/` 文件夹下下载对应配置。

<img width="1608" alt="1725980030845" src="https://github.com/user-attachments/assets/691e04e5-aabd-4f05-8f27-58042e09de87">

然后即可进行下载。

![image-20240910225455708](https://github.com/user-attachments/assets/1b0ea018-af71-48c1-8622-6eda2d8d79b5)

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
