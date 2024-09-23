# <img src="https://github.com/kuibu/embodied-intelligence/blob/main/images/EI-logo_and_name.png" width="220" height="130" alt="EOS">

--------------------------------------------------------------------------------

[![Zulip chat](https://img.shields.io/badge/chat-on%20zulip-brightgreen)](https://eos24.zulipchat.com/)
[![en](https://img.shields.io/badge/lang-en-yellow.svg)](https://github.com/EOS-OS/EOS/blob/master/README.md)
[![zh](https://img.shields.io/badge/lang-中文-yellow.svg)](https://github.com/EOS-OS/EOS/blob/master/README.zh.md)

## Introduction
**Embodied Intelligence** is an embodied intelligence and spatial intelligence algorithm library launched by CAICT and BUPT. It mainly serves the embodied intelligence operating system EOS, but it also supports multiple platforms such as robots, drones, AR/VR and mobile phones.

**EOS** is an embodied intelligence operating system release based on the dual-kernel real-time kernel [RROS](https://github.com/BUPT-OS/RROS).
It aims to build **an easy-to-use platform to collect all the software needed to create an intelligent robot application**.
Specifically, there are three important steps:
   - Build a robot package manager to collect related libraries/framworks/algorithms
   - Improve the RROS ability in the robot development
   - Optimize the package performance in the package platform

![image](https://github.com/user-attachments/assets/87a4dd65-8a43-49e3-bf0a-812f4374b5f8)


## Usage

[Instructions on how to use the system go here.]

## Contributing

### How to Contribute Your Package to the User Repository
The EOS user software repository is currently built using the Copr project. If you are already familiar with using Copr, feel free to skip this section and start using the EOS user repository directly.
#### Preparation
The EOS user repository is built on the Fedora Copr project. You can refer to [the official Copr documentation](https://docs.pagure.org/copr.copr/index.html) for more detailed guidance on using Copr.
First, register for a Fedora account via [Fedora Accounts (fedoraproject.org)](https://accounts.fedoraproject.org/).
Then, visit the user repository [homepage](https://eos.eaishow.com) and log in using your Fedora ID.

<img width="1616" alt="1725976528436" src="https://github.com/user-attachments/assets/5c361a5b-a70f-4ecf-b845-8931689a826c">

After successfully logging in, go to your personal dashboard:

![image-20240910215916777](https://github.com/user-attachments/assets/cc044bcf-f3c8-405c-b0b8-db37d3c5a91d)

#### Create a New Project
Click the `New Project` button to create a new project. Enter the project name, select the desired build environment (Chroots) for the package, and configure other optional settings as needed.

![image-20240910220838008](https://github.com/user-attachments/assets/09f956e6-768b-40f5-959b-6a98fcac7de4)

After completing the configuration, click `Create` to create the project.

<img width="937" alt="1725977723240" src="https://github.com/user-attachments/assets/dfa92d99-c1f3-4f18-ae67-aa387475c6f9">

#### Build a Package
On the project homepage, select `Builds` and click `New Build`.

<img width="609" alt="1725978624151" src="https://github.com/user-attachments/assets/13b30b0d-282b-4d22-8516-1d4a8eddbf1d">

Here, we will build the package by directly uploading the `srpm` file. Under the `Upload` section, click the `Browse` button to upload your local `srpm` file. If you're unfamiliar with how to build an `srpm` package, you can refer to this [guide](https://rpm-packaging-guide.github.io/) for more details.

<img width="596" alt="1725978723980" src="https://github.com/user-attachments/assets/049f294f-a810-4852-bdcb-e0c89afd2879">

Select the local `srpm` file and click `Open` to upload.

![image-20240910223352577](https://github.com/user-attachments/assets/d3aff3a2-71b3-4c28-96c7-3f2dd34d116a)

Finally, click `Build` to start building the package.

<img width="620" alt="1725978912129" src="https://github.com/user-attachments/assets/c49afa55-e35d-4b33-8675-e70ea89a8a91">

Build successful.

<img width="628" alt="1725979056196" src="https://github.com/user-attachments/assets/f0bb61f9-5ea7-471b-a30f-aeb9f411c4bd">

Click on the corresponding `Build ID` to view the build results.

<img width="629" alt="1725979186212" src="https://github.com/user-attachments/assets/65a40e03-963f-42b4-8d29-7b7e600a8dfe">

![image-20240910224043177](https://github.com/user-attachments/assets/e52ae6ca-8981-4592-be48-a84b12aa4fb6)

Copr also supports various other methods to build your package. For more details, please refer to [the official documentation](https://docs.pagure.org/copr.copr/index.html).

#### Using Your Personal Repository
First, return to the project homepage and obtain the repository configuration file for the corresponding version. Click on the version number.

<img width="984" alt="1725979554852" src="https://github.com/user-attachments/assets/93907da0-9d24-4b9f-a419-ca4e87fe91ea">

Copy the link.

<img width="643" alt="1725979632482" src="https://github.com/user-attachments/assets/4e9cb374-ffc9-484a-b763-6d6b169c471f">

Navigate to the `/etc/yum/repos.d/` directory and download the corresponding configuration.

<img width="1608" alt="1725980030845" src="https://github.com/user-attachments/assets/47d9cc03-c223-4683-8e41-f102dcf7dcad">

You can then download the package.

![image-20240910225455708](https://github.com/user-attachments/assets/0dace3b5-feb7-4e7a-9c18-fef782cdf59f)

### How to use community packages integrated with ROS
Currently, we offer a COPR platform for building RPM packages. Support for Deb and other package formats will be added in the future. To install RPM packages, you need to add the EOS repository to your system. Here's an example for Fedora:
```bash
sudo wget -O /etc/yum.repos.d/stevenfreeto-hello-eos-fedora-39.repo http://eos.eaishow.com:9250/coprs/stevenfreeto/hello-eos/repo/fedora-39/stevenfreeto-hello-eos-fedora-39.repo
sudo dnf makecache
# sudo yum makecache    # or if you use yum
```

Additionally, you need to add the EOS rosdep to your ROS environment. Edit the rosdep configuration file at `/etc/ros/rosdep/sources.list.d/20-default.list` and append the following line:
```
yaml https://raw.githubusercontent.com/EOS-OS/EOS/main/package-manager/rosdep/base.yaml
```

After these steps, you can use ROS packages that depend on EOS community packages.

## Where you can find us

At the [EOS Zulip](https://eos24.zulipchat.com/join/lnwy7yspqiiu4hqqlat45vlv/) or email `eosrros AT gmail.com`.

## Who are we

We are a group of robotic developers from research institutes, schools, and robotics enterprises.
We hope `EOS` can unite the strengths of different aspects to accelerate robot development and shorten the communication path between developers and end users.

## Roadmap

![image](https://github.com/user-attachments/assets/5c7addf2-8b04-47ea-aa35-ccc7cbc8134c)

1. **Estabiliing a package manager tool for the robot development**
   - [X] A package image source
   - [ ] A client for package management
     - [X] **Kernel Layer**: Integrate core system libraries to ensure system stability and compatibility.
        - [X] A platform based on the `Copr` to collect packages
     - [ ] **EI Translator (Embodied Intelligence Translator)**: Convert kernel-layer libraries into internal libraries for the middleware layer, facilitating seamless integration across multiple system layers, including ROS/Dart.
     - [ ] **Middleware Layer:**
       - [ ] **ROS**: Provide middleware support for the Robot Operating System, enhancing automation and robotics development capabilities.
       - [ ] **Dora**: Offer advanced middleware support for distributed systems, increasing system scalability and flexibility.
2. **Use RROS kernel to enhance the realtime ability of robot development**
   - [ ] Realtime ability
     - [ ] Adapt ROS
     - [ ] Adapt Dora
     - [ ] Adapt EtherCAT protocol
   - [ ] Adaptation and optimization for domestic Chips
     - [X] x86 Series
     - [X] ARM Series
     - [X] LoongArch (Works on the single CPU core)
     - [ ] RISC-V
3. **Development of upper-layer applications based on the RROS kernel**
   - [ ] Provide a rich API to support application developers in fully leveraging the powerful features of the RROS kernel, accelerating application development and deployment.
   - [ ] Accelerate the applications with the RROS APIs.
