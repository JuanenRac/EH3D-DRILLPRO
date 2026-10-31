<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-DRILLPRO banner" width="100%">
</p>

# 🔩 EH3D-DRILLPRO

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | 🇨🇳 <b>中文</b> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### 用 Rust 跨平台重新实现 Bungard CCD 机器经典 DOS 软件的钻孔流程，提供类似原版的文本模式界面，支持八种语言

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.7-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**诚实性核对 - 今天真正能运行的内容：** 一个带 80x25 文本模式界面的 Rust 程序，读取 Excellon `.NCD` 钻孔文件、刀具表和机器配置，对孔排序并显示，然后**在模拟器上**（默认）运行整个作业；若机器文件指定，则通过电脑串口、USB 串口适配器或 EH3D-NODE-BUNGARD **在真实机器上**运行。电脑上运行 237 项测试。在一台真实的 Bungard CCD MTC 上，已经试过三轴回参考点（Z、X、Y 到各自限位开关，并现场确认控制单元的“三轴全部到位”信号）以及每个轴的运动；**完整的钻孔作业尚未在机器上运行**，每毫米步数也只是与原版配置核对过。 默认显示 MS-DOS 程序的经典界面，已逐个单元格与原版核对；`--modern` 启动菜单式界面。 可用键盘和鼠标操作，窗口可最大化（画面从 640x400 缩放到 1080p 及更大），配置文件和刀具表可在对话框中编辑并保存，“关于”按钮列出 Bungard CCD 和 CCD/2 机型（支持 MTC，暂不支持 ATC）。 控制台版本（`drillpro-console`）在终端中显示同样的界面，支持键盘和鼠标，不需要窗口系统（SSH、文本控制台、旧电脑）。 每个版本的软件包都带有逐行解释的基本配置，以及从一个孔到两万个孔的 `examples` 文件夹。ATC 机型（自动换刀）的换刀功能依据原程序的录制数据实现，尚未在真实机器上运行。

---

> **安全（safety）与信息安全现状：** 本程序不是安全系统。使用真实连接（机器文件中的 `connection`）时，**“开始”会让机器动起来**：先取下刀具，一只手放在机器电源开关上，并随时准备按 `Esc`，它会立即停止作业并关闭步进驱动器。限位开关由机器自己的控制单元监视（它会在开关处自行停住轴）；本软件没有自己的紧急停止，错误的步进字节或速度可能使刀具撞向工件或工作台，或使电机失步。协议层在被解锁之前拒绝移动任何东西。本项目是独立的：它不是 Bungard Elektronik 制造、认可或支持的，产品名称只是用来说明它面向哪些文件格式和机器。

## 📋 速览

| | |
|---|---|
| 版本 | `0.0.7` |
| 成熟度 | 脚手架 |
| 测试 | 237 |
| 技术栈 | Rust |
| 真实硬件 | 是，在一台 Bungard CCD MTC 上：回参考点和每个轴的运动；完整作业尚未运行。没有机器文件时在模拟器上运行 |

**跳转到：** [概述](#-概述) · [架构](#-架构) · [快速开始](#-快速开始) · [测试](#-测试) · [结构](#-仓库结构) · [相关项目](#-相关项目)

## 🚦 项目状态

每一行是本仓库的一个部分，灯号表示它今天做到了哪一步。内容依据仓库能够展示的证据（测试、构建、运行），而不是计划。

🟢 已完成并验证 · 🟡 可用，有已知限制 · 🟠 部分完成或尚未验证 · 🔴 未完成或不可用

| 领域 | 状态 | 在这里的含义 |
|---|---|---|
| 软件 | 🟡 | 带 80x25 文本界面的 Rust 程序读取钻孔文件、刀具表和机床文件，排列孔的顺序，并在模拟器上运行整个作业；机床文件指定时则在真实机床上运行。 |
| 测试 | 🟢 | 237 项测试通过（`cargo test --workspace`）：各种格式、作业规划、模拟器上的协议以及八种语言的界面。 |
| 文件读取器 | 🟡 | 可读取钻孔文件 `.NCD`、刀具表 `DRILLPRO.DTD` 和机床文件 `DRILLPRO.DCF`；只用为测试编写的小型样例文件验证过，没有用过真实电路板的文件。 |
| 机床协议 | 🟠 | 控制单元的字节协议是通过逆向工程重建的，`docs/PROTOCOL.md` 中仍有属于假设的部分；协议层在上膛之前拒绝移动任何东西。 |
| 界面 | 🟡 | 文本界面、菜单、文件对话框和孔的矢量视图支持全部八种语言（已测试）；中文和日文需要系统中有 CJK 字体。 |
| 真实硬件 | 🟠 | 在一台 Bungard CCD MTC 上试过三个轴的参考点回零和每个轴的移动；整个钻孔作业没有运行过，每毫米步数只与原版配置核对过，主轴和吸尘器输出尚未试过。 |
| 使用安全 | 🟡 | 按 `Esc` 可在极短时间内停止作业并关闭步进驱动器，限位开关由机床自己的控制单元监视；软件本身没有急停。 |
| 服务器连接 | 🟠 | 设置 `connection = server` 后，程序把作业发给 EH3D-SERVER 并跟踪进度；这条路径只在模拟器上跑过。 |
| 现成安装包 | 🟡 | 首个发布版本带有 Windows、Linux 和 macOS（Apple 芯片与 Intel）的安装包，各附 SHA-256，由发布工作流构建。 |
| 文档 | 🟢 | 八种语言的 README、标明仍属假设之处的协议说明，以及列出每个调校值的机床文件指南。 |

## 🎯 概述

* **文件读取器**：Excellon `.NCD` 钻孔文件（带或不带 `M48` 文件头，模态坐标，1 mil 英制单位）、刀具表 `DRILLPRO.DTD` 和机床配置 `DRILLPRO.DCF`。
* **作业规划器**：对孔排序（按文件顺序、先 X、先 Y 或最短路径），并按刀具保持分组。
* **界面**：80x25 的文本模式屏幕，带有 CGA 颜色、菜单、文件对话框和孔的矢量视图，显示在一个窗口中。支持英语、西班牙语、意大利语、德语、法语、葡萄牙语、中文和日语：可在菜单中选择语言，或用 `--lang` 启动。
* **机床协议**（`protocol/`，crate `eh3d-drillpro-protocol`）：一个没有界面的库：字节传输（串口或模拟器）、带加减速曲线的毫米级运动规划，以及在解除保险之前拒绝运动的安全闸。
* **模拟器**：整个作业无需硬件即可运行到结束；测试使用它。
* **机器**：控制单元通过串口连接、使用过 DOS 软件 4.6 的机器：Bungard CCD MTC 和 CCD/2 MTC（手动换刀；CCD/2 台面较小，325 x 270 mm）。台面尺寸等机器常数来自机器的配置文件。只在一台 CCD MTC 上试过，主轴（钻头）和吸尘器启动输出尚未试过。两种 ATC 型号（CCD ATC 和 CCD/2 ATC，自动换刀）将另做独立的应用，利用逆向工程所能得到的信息。XL 型号及之后的机器带有自己的现代控制单元和软件，不在支持范围内。
* **机器文件**（`machine.cfg`，DRILLPRO 与 ROUTEPRO 共用）：作业如何到达机器（`demo` 模拟器，`rs232` 电脑自己的串口，`usb` USB 串口适配器，`node` 经网络的 EH3D-NODE-BUNGARD），并且每行一个值，记录在真实机器上调出的所有参数：搜索速度和加减速斜坡、校准跳、电机极限。
* **回参考点**：Z 先上升到限位开关，然后 X 和 Y，最后做一次柔和的校准跳。控制单元在 RI 线上报告“三轴全部到位”，程序监视这条线；机器已停在原点时，它跳过搜索，先让 X 和 Y 离开开关 10 mm，再重新搜索，从而现场看到它们到位。
* **平稳运动**：运动以一条连续数据流发出，字节位置携带时间（否则 USB 适配器会成批地交付字节），Z 逐字节运动，`Esc` 在一瞬间取消作业并关闭步进驱动器。
* **主轴**：默认使用控制单元自己的（锁存位，最高 60000 rpm）；在机器文件中设置 `spindle = node` 后，作业改为向 EH3D-NODE-BUNGARD 要求主轴（节点输出端的变频器，节点最大值以内的任意转速，任何停止时都会停转）。使用节点时，程序还会读取机器的指示灯，因此每个轴到达原点都能实时看到，无需在数据流中间读取原点线。
* **服务器连接**（机器文件中的 `connection = server`，配合 `server_address`、`server_token` 和 `server_machine`）：程序不再自己驱动机器，而是把作业发给 [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER)，由它按机器的常数规划、通过机器的节点执行，并告诉程序进展如何；程序照常显示进度并回答对话框，`Esc` 会停止服务器上的作业，关闭程序则不会停止它。
* **作业压缩包、Offset 窗口和模拟**：作业可以打包成 ZIP 压缩包（Select File 窗口可进入其中，`drillpro JOB.ZIP` 会载入其中第一个钻孔文件）；文件名旁的箭头列出 `*.NCD`、`*.ZIP` 和 `*.*` 模式；刀具窗口和配置窗口的 Load 按钮用于选择要载入的文件；在真实机器上，Offset 窗口会让机器回零、跟随窗口的每一次移动并再次回零；模拟作业所用的时间与机器实际所需相同。
* **提示音**：机器文件中的 `sound_finish` 和 `sound_tool_change` 分别可以是 off（默认）、`default`（程序自带的音调）或你自己的 WAV 文件路径；前者在作业自行结束时响起，后者在机器要求换刀时响起。 `sound_check_ok` 和 `sound_check_bad` 对使用钻头摄像头进行的电路板目视检查起同样作用（默认关闭；语音会说明检查合格或不合格）。
* **多台机床和工位**：当 `connection = server` 时，`drillpro --machines` 列出服务器的所有机床，按工位分组，显示每台的状态、作业和就绪作业（服务器一次回答所有机床），`drillpro --machine ID` 把本程序的作业发给该机床，而不是机床文件中的那台；各机床同时运行，每台一个作业。
* **在“配置”窗口中选择连接：** 下拉列表列出作业到达机床的五种方式（模拟器、串口、USB 适配器、EH3D 节点和 EH3D 服务器），下方显示每种方式所需的内容：端口、地址和令牌、服务器上的机床；所选内容会经过检查，用“确定”应用，并写入机床文件而不改动其其他行。
* **闪烁的光标：** 正在编辑的数字或文本字段的光标会闪烁（亮半秒、灭半秒），与其所遵循的界面一致。
* **带连接点的拼板与可继续的作业：** 拼板外侧可留连接点（几个、多宽、在哪里），作业可跳过已完成的孔，停止后能继续；窗口以最大化打开。
* **不平的工作台、语音提示和钻头节点：** 作业选项可以带上工作台高度图，无论工作台高低，每个孔在板上的钻深都相同（没有高度图则不变）；作业结束和请求刀具的声音可以是语音（`sound_finish = voice:es:f`），取自程序旁的 `voices` 文件夹；机床文件可以写明钻头节点（`head_node`、`head_address`、`head_token`、`head_camera`），供现代界面使用。

### 🆚 原版与本版本

原版是 Bungard 控制 CCD 机床的闭源 MS-DOS 程序。本版本根据对原版行为的录制（它的屏幕、文件以及发送给控制单元的内容）重新编写，不含原版的任何代码。相同之处与不同之处如下：

| 方面 | 原版 (MS-DOS) | 本版本 |
|---|---|---|
| 电脑 | 只能是 MS-DOS 电脑（或模拟器） | Windows、Linux 和 macOS，32 位和 64 位；另有用于 SSH 和旧电脑的控制台版本 |
| 界面 | 80x25 文本模式 | 默认是同样的经典界面，已与原版逐格核对，支持鼠标，窗口可从 640x400 缩放到 1080p 及更大；`--modern` 提供菜单式界面，EH3D-BUNGARD-GUI 提供图形界面 |
| 语言 | 程序自带的语言 | 八种：英语、西班牙语、意大利语、德语、法语、葡萄牙语、中文和日语，可从菜单或用 `--lang` 切换 |
| 拼板 | 在工作台上复制板子 | 在工作台上复制板子；中途中断的作业可从下一个孔继续 |
| 与机床的连接 | 电脑串口连接控制单元 | 默认是模拟器；真机上可用串口、USB 串口适配器、通过网络的 EH3D-NODE-BUNGARD 或 EH3D-SERVER，在 Config 窗口的下拉列表中选择 |
| 新增 | 以上都没有 | 可运行整个作业的模拟器、作业结束和请求刀具的提示音、通过服务器同时管理多台机床、ZIP 作业压缩包、DXF 和 Gerber 轮廓、闪烁光标，以及每次改动都会运行的测试集 |
| 源码与许可 | 闭源、专有 | 开源（GPL-3.0-or-later），用 Rust 编写 |
| 成熟度 | 在真实机床上使用多年 | 年轻得多：参考回零和各轴运动已在一台真实的 Bungard CCD MTC 上试过；整个作业尚未在机床上运行过，ATC 型号的换刀器也未在真机上运行过 |

## 🧩 在 EH3D 中的角色

EH3D-DRILLPRO 是 EH3D 家族中的独立程序，面向其控制单元需要由 PC 生成每一个电机步进的机床。它的铣削姊妹项目 EH3D-ROUTEPRO 复用其协议、屏幕和语言支持。对于打包成 ZIP 压缩包的作业，它使用 EH3D-COMMON 的一个 crate（`eh3d-archive`），编译时必须放在它旁边；以后可以用 EH3D-CORE-LIBS 的 Excellon 读取器替换自己的读取器。 机器的文件（钻孔文件、HP-GL 文件、刀具表和机器配置）来自 EH3D-COMMON 的 crate `eh3d-bungard-formats`，它同样必须放在旁边。

## 🏛️ 架构

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-DRILLPRO 的架构：谁在使用、包含什么、依赖什么" width="100%">
</p>

本仓库各部分如何协作：谁在使用它、它包含什么、它依赖什么。图片由 `docs/architecture.json` 绘制，并与代码核对。

## 🖼️ 屏幕截图

<p align="center">
  <img src="images/screenshots/main.png" alt="main screen" width="80%">
</p>

<p align="center"><em>经典程序的主界面，与机器自带程序的画法一致：比例、偏移和裁剪、作业的矢量以及按钮。</em></p>

<p align="center">
  <img src="images/screenshots/tools.png" alt="tool data" width="80%">
</p>

<p align="center"><em>刀具数据：换刀器每个位置的附加行程、进给和主轴转速，以及每把刀具的使用量。</em></p>

<p align="center">
  <img src="images/screenshots/config.png" alt="parameters" width="80%">
</p>

<p align="center"><em>机器参数：文件的坐标、孔的排序、速度、加减速、修正和语言。</em></p>

<p align="center">
  <img src="images/screenshots/connection.png" alt="connection" width="80%">
</p>

<p align="center"><em>与机床的连接：在下拉列表中选择的方式及其所需内容（此处为 EH3D 服务器）。</em></p>

## 🚀 快速开始

需要稳定版的 Rust 工具链。

```bash
cargo build --release --features gui
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
python -I tools/ci_validate.py
```

用 `run.bat`（Windows）或 `./run.sh`（Linux 和 macOS）运行；两者都会在需要时先构建程序：`run.bat [文件.NCD] [--lang en|es|it|de|fr|pt|zh|ja] [--data DIR] [--dump]`。`--data` 是存放 `DRILLPRO.DTD` 和 `DRILLPRO.DCF` 的文件夹（默认：`testdata/`），`--dump` 将屏幕以纯文本输出。按键：`Alt` 加高亮字母打开菜单，方向键移动，`Enter` 确认，`Esc` 返回；也可以点击菜单栏。中文和日文需要系统字体中有 CJK 字体。 `--connection FILE` 指向不同于默认的机器文件（Windows 上默认为 `%APPDATA%\EH3D\machine.cfg`，其他系统为 `~/.config/eh3d/machine.cfg`）；没有机器文件时程序在模拟器上运行。 `--modern` 启动菜单式界面，而不是经典界面。

## 🧪 测试

测试套件共有 237 个测试（`cargo test --workspace`）。它们检查文件格式（钻孔文件、刀具表和机器文件，以及写回）、作业规划、模拟器上的协议（字节、运动、回参考点、锁存位、与节点的连接，以及未启用前拒绝运动的安全门）、逐个字符单元核对的经典界面，以及八种语言的界面。CI 还会运行 `cargo fmt --check` 和把警告当作错误的 `cargo clippy`，并由 `python -I tools/ci_validate.py` 检查清单、版本、变更日志和八个 README。

## 🏗️ 构建与发布

版本号位于 `Cargo.toml` 和 `eh3d.project.json` 中，每个版本在 [CHANGELOG.md](CHANGELOG.md) 中都有条目。发布版本在 GitHub 上公开。

**预编译软件包：** 每个发布版本都附带适用于 Windows（zip）、Linux（tar.gz）和 macOS（Apple 芯片与 Intel，tar.gz）的软件包，内含程序、许可证和本 README，旁边附有各自的 SHA-256。它们由本仓库的 `Release binaries` 工作流构建。请从 Releases 页面下载；在 Linux 上，窗口需要 X11 或 Wayland。

## 📂 仓库结构

<p align="center">
  <img src="images/repository-structure.svg" alt="EH3D-DRILLPRO 的仓库结构" width="100%">
</p>

<details>
<summary>目录树的文本版</summary>

```text
EH3D-DRILLPRO/
├── src/
│   ├── classic/                                 MS-DOS 程序的经典界面、对话框和按键
│   ├── job.rs runner.rs                         排列孔位，并在独立线程中运行作业
│   ├── app.rs screen.rs textapp.rs              80x25 文本屏幕和现代菜单
│   ├── i18n.rs texts.rs                         八种语言及其文本
│   ├── gui.rs console.rs bin/                   窗口、终端版本和可执行文件
│   └── server_link.rs cli.rs                    与 EH3D-SERVER 的连接和命令行
├── protocol/src/
│   ├── safety.rs                                未启用前拒绝运动的保护门
│   ├── motion.rs positioner.rs                  带加减速的毫米级运动
│   ├── execute.rs job.rs                        把作业转换成字节流
│   ├── homing.rs latches.rs                     回参考点和锁存位
│   ├── serial.rs node.rs nullmodem.rs           串口、节点和模拟器
│   └── connection.rs tuning.rs                  机器文件及其调校值
├── tests/                                       格式、作业规划、经典界面和所有语言的界面
├── docs/                                        协议说明、经典界面、连接指南和含全部取值的机器文件
├── testdata/                                    为测试编写的小型示例文件
├── config/                                      逐行解释的基本配置
├── examples/                                    示例：从1个孔到2万个孔
├── tools/                                       CI 检查、README 检查和图片工具
├── images/                                      横幅图、两张图片和屏幕截图
├── .github/                                     CI 与发布工作流、Issue 和 PR 模板
├── build.sh build.bat                           构建和测试脚本
├── run.sh run.bat                               启动程序
├── eh3d.project.json                            清单：版本、角色、成熟度和构建命令
├── Cargo.toml Cargo.lock                        Rust 包及其锁定的依赖
├── CHANGELOG.md                                 每个版本的变更
├── SECURITY.md SUPPORT.md                       如何报告漏洞及在哪里提问
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   参与贡献、行为准则和许可证
```

</details>

## 🗺️ 路线图与已知限制

**目前已知的限制**

* 尚未在真实机器上运行过完整的钻孔作业：只在一台 Bungard CCD MTC 上试过三个轴的回参考点和每个轴的运动，每毫米步数只与原版配置核对过，主轴和吸尘器输出尚未试过。
* `docs/PROTOCOL.md` 仍列出协议中属于假设的部分；协议层在被解锁之前拒绝移动任何东西。
* 文件读取器只用为测试编写的小型示例文件试过，没有用真实电路板的文件试过。
* 服务器连接（`connection = server`）只在模拟器上练过。
* 只支持 MTC 型号；ATC 型号尚不支持，XL 型号不在范围内。中文和日文需要系统字体中有 CJK 字体。

**下一步**

* 在机器上运行完整的钻孔作业，并确认每毫米步数、主轴和吸尘器输出。
* 用真实电路板的文件试验文件读取器。
* 为 ATC 型号单独开发应用程序，以逆向工程所能提供的信息为基础。
* 待 [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS) 的 Excellon 读取器就绪后，用它替换自带的读取器。

## 🌍 语言

README 和文本提供英语、西班牙语、法语、意大利语、德语、汉语、日语和欧洲葡萄牙语版本。

## 🔗 相关项目

* [EH3D-ROUTEPRO](https://github.com/JuanenRac/EH3D-ROUTEPRO): 铣削姊妹项目；使用本项目的协议、屏幕和语言
* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): 套件共享的 Excellon 和 G-code 读取器
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): 套件的项目容器和机床遥测类型
* [EH3D-NODE-BUNGARD](https://github.com/JuanenRac/EH3D-NODE-BUNGARD)：装在机器箱内的 ESP32-S3 节点（以太网或 Wi-Fi），通过自己的串口发出步进，时序是 USB 适配器保持不了的，并读取机器的指示灯、控制主轴驱动器；它有自己的网页面板
* [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER)： EH3D 家族的中心：接收本程序（`connection = server`）和其他客户端的作业，并通过节点在机器上执行

## 📚 文档

* [协议说明](docs/PROTOCOL.md): 已知的控制单元驱动方式，以及哪些仍是假设。
* [连接与调整](docs/CONNECTION.md)：机器文件、三种连接方式和每个值
* [经典界面](docs/CLASSIC.md)：如何复现并核对 MS-DOS 程序的界面
* [更新日志](CHANGELOG.md)

## 🤝 参与贡献与社区

* 提交 Pull Request 之前请运行 `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace` 和 `python -I tools/ci_validate.py`：必须全部通过。参见 [CONTRIBUTING.md](CONTRIBUTING.md) 和[行为准则](CODE_OF_CONDUCT.md)。
* README 有八种语言：修改文字时要同步修改全部八个文件，两张图片由 `tools/make_architecture.py` 和 `tools/make_structure.py` 根据 `docs/architecture.json` 和 `docs/structure.json` 绘制。
* 不要把仅在模拟器上试过的东西描述为已在机器上运行。
* 请私下报告漏洞：[SECURITY.md](SECURITY.md)。问题、想法和报告：electrohobby3d@gmail.com（[SUPPORT.md](SUPPORT.md)）。

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 许可证

软件：GPL-3.0-or-later。见 [LICENSE](LICENSE)。套件的硬件设计使用 CERN OHL-S。
