<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-DRILLPRO banner" width="100%">
</p>

# 🔩 EH3D-DRILLPRO

<p align="center">🇺🇸 <b>English</b> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### A cross-platform Rust re-implementation of the drilling workflow of the classic DOS software for Bungard CCD machines, with a text-mode look-alike interface in eight languages

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.4-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Honesty check - what actually runs today:** a Rust program with an 80x25 text-mode interface that reads Excellon `.NCD` drill files, the tool table and the machine configuration, orders the holes, shows them and runs the whole job either **against a simulator** (the default) or, when the machine file says so, **on a real machine** through the computer's serial port, a USB serial adapter or the EH3D-NODE-BUNGARD. 213 tests run on a computer. On one real Bungard CCD MTC the reference run of the three axes (Z, X and Y to their limit switches, with the control unit's "all three home" signal confirmed live) and the moves of every axis have been tried; **a whole drilling job has not run on the machine yet**, and the steps per millimetre are only checked against the original's configuration. The screens you get by default are the classic ones of the MS-DOS program, checked cell by cell against the original; `--modern` starts the menu-driven interface instead. They work with the keyboard and the mouse, the window can be maximized (the screen scales from 640x400 up to 1080p and beyond), the configuration and tool table files are edited and saved from the dialogs, and the About button lists the Bungard CCD and CCD/2 machines (MTC supported, ATC not yet). A console version (`drillpro-console`) draws the same screens in a terminal, with the keyboard and the mouse and no window system (SSH, a text console, an old PC). The release packages carry the basic configuration, explained line by line, and an `examples` folder from one hole to twenty thousand. The tool changer of the ATC models (automatic tool change) works from a recording of the original and has not run on a real machine.

---

> **Safety and security status:** this program is not a safety system. With a real connection (`connection` in the machine file) **Start moves the machine**: take the tool out, keep a hand on the machine's power switch and be ready to press `Esc`, which stops the job at once and switches the stepper drivers off. The limit switches are watched by the machine's own control unit (it stops an axis at its switch by itself); this software has no emergency stop of its own, and wrong step bytes or speeds can drive the tool into the work or the table or make the motors lose steps. The protocol layer refuses to move anything until it has been armed. The project is independent: it is not made, endorsed or supported by Bungard Elektronik, and product names appear only to say which file formats and machines it is meant to work with.

## 📋 At a glance

| | |
|---|---|
| Version | `0.0.4` |
| Maturity | Scaffolding |
| Tests | 213 |
| Stack | Rust |
| On real hardware | Yes, on one Bungard CCD MTC: the reference run and the moves of every axis; a whole job has not run yet. Without a machine file it runs on the simulator |

**Jump to:** [Overview](#-overview) · [Architecture](#-architecture) · [Quick start](#-quick-start) · [Testing](#-testing) · [Structure](#-repository-structure) · [Related](#-related-projects)

## 🚦 Project status

Each line is one part of this repository and the light says how far it is today. It is written from what the repository can show (tests, builds, runs), not from what is planned.

🟢 Done and verified · 🟡 Works, with known limits · 🟠 Partly done or not verified yet · 🔴 Not done or not working

| Area | State | What it means here |
|---|---|---|
| Software | 🟡 | A Rust program with an 80x25 text-mode screen reads the drill file, the tool table and the machine file, orders the holes and runs the whole job against a simulator, or on a real machine when the machine file says so. |
| Tests | 🟢 | 213 tests pass (`cargo test --workspace`): the formats, the job planning, the protocol on the simulator and the screens in all eight languages. |
| File readers | 🟡 | The `.NCD` drill file, the tool table `DRILLPRO.DTD` and the machine file `DRILLPRO.DCF` are read; only with small sample files written for the tests, not with files from real boards. |
| Machine protocol | 🟠 | The byte protocol of the control unit was rebuilt by reverse engineering and `docs/PROTOCOL.md` still lists parts that are hypotheses; the protocol layer refuses to move anything until it has been armed. |
| Interface | 🟡 | The text-mode screen, the menus, the file dialog and the vector view of the holes speak all eight languages (tested); Chinese and Japanese need a CJK font in the system. |
| Real hardware | 🟠 | On one Bungard CCD MTC the reference run of the three axes and the moves of every axis have been tried; a whole drilling job has not run, the steps per millimetre are only checked against the original's configuration, and the spindle and vacuum outputs are untried. |
| Safety in use | 🟡 | `Esc` stops the job within a fraction of a second and switches the stepper drivers off, and the machine's own control unit watches the limit switches; the software has no emergency stop of its own. |
| Server connection | 🟠 | With `connection = server` the program sends the job to EH3D-SERVER and follows its progress; this path has only been exercised against the simulator. |
| Ready-made packages | 🟡 | The first release carries packages for Windows, Linux and macOS (Apple silicon and Intel), each with its SHA-256, built by the release workflow. |
| Documentation | 🟢 | README in eight languages, the protocol notes with what is still a hypothesis, and the guide to the machine file with every tuning value. |

## 🎯 Overview

* The **file readers**: Excellon `.NCD` drill files (with or without the `M48` header, modal coordinates, imperial units of 1 mil), the tool table `DRILLPRO.DTD` and the machine configuration `DRILLPRO.DCF`.
* The **job planner**: orders the holes (as in the file, X first, Y first or shortest path) and keeps them grouped by tool.
* The **interface**: an 80x25 text-mode screen with CGA colours, menus, a file dialog and a vector view of the holes, shown in a window. It speaks English, Spanish, Italian, German, French, Portuguese, Chinese and Japanese: choose the language from the menu or start with `--lang`.
* The **machine protocol** (`protocol/`, crate `eh3d-drillpro-protocol`): a library with no interface: byte transport (serial port or simulator), motion planning in millimetres with acceleration ramps, and a safety gate that refuses motion until it is armed.
* The **simulator**: the whole job runs to the end without hardware; the tests use it.
* The **machines**: those with a serial-port control unit that ran the DOS software 4.6: Bungard CCD MTC and CCD/2 MTC (manual tool change; the CCD/2 has the smaller table of 325 x 270 mm). The table size and the other machine constants come from the machine's configuration file. It has been tried on a CCD MTC only, and the spindle and vacuum cleaner outputs have not been tried yet. The two ATC models (CCD ATC and CCD/2 ATC, automatic tool change) will get separate applications, built with what the reverse engineering allows. The XL models and the machines made since have a modern control unit and software of their own and are not covered.
* The **machine file** (`machine.cfg`, shared by DRILLPRO and ROUTEPRO): how the job reaches the machine (`demo` simulator, `rs232` the computer's own serial port, `usb` a USB serial adapter or `node` the EH3D-NODE-BUNGARD over the network) and, one value per line, everything tuned on a real machine: search speeds and ramps, the calibration jump, the limits of the motors.
* The **reference run**: Z up to its limit switch, then X and Y, then a smooth calibration jump. The control unit reports "all three axes home" on the RI line, which the program watches; when the machine is parked it skips the searches, moves X and Y 10 mm away from their switches and searches for them again, so it sees them arrive live.
* **Steady motion**: moves go out as one continuous stream whose byte positions carry the time (a USB adapter delivers bytes in bursts otherwise), Z moves byte by byte, and `Esc` cancels a job within a fraction of a second and switches the stepper drivers off.
* The **spindle**: by default the control unit's own (its latch bits, up to 60000 rpm); with `spindle = node` in the machine file the job asks the EH3D-NODE-BUNGARD for it instead (a variable-frequency drive on the node's outputs, any speed up to the node's maximum, stopped on any stop). With a node the programs also read the machine's lamps, so each axis arriving at home is seen live without reading the home line in the middle of the stream.
* The **server connection** (`connection = server` in the machine file, with `server_address`, `server_token` and `server_machine`): instead of driving the machine itself, the program sends the job to the [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER), which plans it with the constants of the machine, runs it through the machine's node, and tells the program how it goes; the program shows the progress and answers the dialogs as usual, `Esc` stops the job on the server, and closing the program does not stop it.
* The **job archives, the Offset window and the simulation**: a job can be packed in a ZIP archive (the Select File window goes inside it, and `drillpro JOB.ZIP` loads its first drilling file); the arrow next to the file name lists the patterns `*.NCD`, `*.ZIP` and `*.*`; the Load buttons of the tool and configuration windows choose the file to load; on a real machine the Offset window homes it, follows every move of the window and homes it again; and the simulated job takes the time that the machine would take.

## 🧩 Role in EH3D

EH3D-DRILLPRO is a stand-alone program of the EH3D family for machines whose control unit needs the PC to generate every motor step. EH3D-ROUTEPRO, its milling sibling, reuses its protocol, screen and language support. For the jobs packed in ZIP archives it uses a crate of EH3D-COMMON (`eh3d-archive`), which has to be next to it to build; the Excellon reader of EH3D-CORE-LIBS could replace its own one later. The files of the machine (the drill file, the HP-GL file, the tool tables and the machine configuration) come from the crate `eh3d-bungard-formats` of EH3D-COMMON, which has to be next to it as well.

## 🏛️ Architecture

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-DRILLPRO architecture: who uses it, what it holds and what it depends on" width="100%">
</p>

How the parts of this repository fit together: who uses it, what it holds and what it depends on. The picture is drawn from `docs/architecture.json` and checked against the code.

## 🚀 Quick start

You need a stable Rust toolchain.

```bash
cargo build --release --features gui
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
python -I tools/ci_validate.py
```

Run it with `run.bat` (Windows) or `./run.sh` (Linux and macOS); both build the program first if needed: `run.bat [FILE.NCD] [--lang en|es|it|de|fr|pt|zh|ja] [--data DIR] [--dump]`. `--data` is the folder with `DRILLPRO.DTD` and `DRILLPRO.DCF` (default: `testdata/`) and `--dump` prints the screen as plain text. Keys: `Alt` plus the highlighted letter opens a menu, arrows move, `Enter` accepts, `Esc` goes back; the menu bar can also be clicked. Chinese and Japanese need a CJK font among the system fonts. `--connection FILE` points to another machine file than the default one (`%APPDATA%\EH3D\machine.cfg` on Windows, `~/.config/eh3d/machine.cfg` elsewhere); without a machine file the program runs on the simulator. `--modern` starts the menu-driven interface instead of the classic screens.

## 🧪 Testing

The suite has 213 tests (`cargo test --workspace`). They check the file formats (the drill file, the tool table and the machine file, and writing them back), the job planning, the protocol on the simulator (bytes, motion, the reference run, the latch bits, the link to the node and the safety gate that refuses to move until armed), the classic screens cell by cell and the screens in all eight languages. The CI also runs `cargo fmt --check` and `cargo clippy` with warnings as errors, and `python -I tools/ci_validate.py` checks the manifest, the versions, the changelog and the eight READMEs.

## 🏗️ Build & Release

The version lives in `Cargo.toml` and in `eh3d.project.json`, and each version has its entry in [CHANGELOG.md](CHANGELOG.md). Releases are published on GitHub.

**Ready-made packages:** every release carries a package for Windows (zip), Linux (tar.gz) and macOS (Apple silicon and Intel, tar.gz) with the program, the licence and this README, each with its SHA-256 next to it. They are built by the `Release binaries` workflow of this repository. Download one from the Releases page; on Linux the window needs X11 or Wayland.

## 📂 Repository Structure

<p align="center">
  <img src="images/repository-structure.svg" alt="Repository structure of EH3D-DRILLPRO" width="100%">
</p>

<details>
<summary>Text version of the tree</summary>

```text
EH3D-DRILLPRO/
├── src/
│   ├── classic/                                 the classic screens, dialogs and keys of the MS-DOS program
│   ├── job.rs runner.rs                         ordering the holes and running the job on its own thread
│   ├── app.rs screen.rs textapp.rs              the 80x25 text-mode screen and the modern menus
│   ├── i18n.rs texts.rs                         the eight languages and their texts
│   ├── gui.rs console.rs bin/                   the window, the terminal version and the executables
│   └── server_link.rs cli.rs                    the link to EH3D-SERVER and the command line
├── protocol/src/
│   ├── safety.rs                                the gate that refuses to move until armed
│   ├── motion.rs positioner.rs                  motion in millimetres with ramps
│   ├── execute.rs job.rs                        turning a job into the byte stream
│   ├── homing.rs latches.rs                     the reference run and the control unit's latch bits
│   ├── serial.rs node.rs nullmodem.rs           serial port, node and simulator
│   └── connection.rs tuning.rs                  the machine file and its tuning values
├── tests/                                       formats, job planning, the classic screens and the screens in all languages
├── docs/                                        the protocol notes, the classic screens, the connection guide and the machine file with all its values
├── testdata/                                    small sample files written for the tests
├── config/                                      the basic configuration, explained line by line
├── examples/                                    example files, from one hole to 20000
├── tools/                                       CI check, README check and the picture tools
├── images/                                      banner and the Architecture and Repository Structure pictures
├── .github/                                     CI and release workflows, issue and pull-request templates
├── build.sh build.bat                           build and test scripts
├── run.sh run.bat                               start the program
├── eh3d.project.json                            manifest: version, role, maturity and build command
├── Cargo.toml Cargo.lock                        the Rust package and its locked dependencies
├── CHANGELOG.md                                 what changed in each version
├── SECURITY.md SUPPORT.md                       how to report a vulnerability and where to ask
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   how to contribute, the code of conduct and the licence
```

</details>

## 🗺️ Roadmap & Known limits

**Known limits today**

* A whole drilling job has not run on a real machine yet: only the reference run of the three axes and the moves of every axis have been tried on one Bungard CCD MTC, the steps per millimetre are only checked against the original's configuration, and the spindle and vacuum outputs are untried.
* `docs/PROTOCOL.md` still lists parts of the protocol that are hypotheses; the protocol layer refuses to move anything until it has been armed.
* The file readers have only been tried with small sample files written for the tests, not with files from real boards.
* The server connection (`connection = server`) has only been exercised against the simulator.
* Only the MTC models are supported; the ATC models are not yet and the XL models are not covered. Chinese and Japanese need a CJK font among the system fonts.

**Next**

* Run a whole drilling job on the machine and confirm the steps per millimetre, the spindle and the vacuum outputs.
* Try the file readers with files from real boards.
* Separate applications for the ATC models, built with what the reverse engineering allows.
* Replace its own Excellon reader with the one of [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS) when that is ready.

## 🌍 Languages

README and texts exist in English, Spanish, French, Italian, German, Chinese, Japanese and European Portuguese.

## 🔗 Related Projects

* [EH3D-ROUTEPRO](https://github.com/JuanenRac/EH3D-ROUTEPRO): the milling sibling; it uses this project's protocol, screen and languages
* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): the shared Excellon and G-code readers of the suite
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): the project container and the machine telemetry types of the suite
* [EH3D-NODE-BUNGARD](https://github.com/JuanenRac/EH3D-NODE-BUNGARD): an ESP32-S3 node (Ethernet or Wi-Fi) that sits in the machine's box, sends the steps out of its own serial port with the timing a USB adapter cannot keep, reads the machine's lamps and drives the spindle's drive; it has its own web panel
* [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER): the center of the EH3D family: it takes the jobs of this program (`connection = server`) and of the other clients, and runs them on the machine through its node

## 📚 Documentation

* [Protocol notes](docs/PROTOCOL.md): what is known about how the control unit is driven, and what is still a hypothesis.
* [Connection and tuning](docs/CONNECTION.md): the machine file, the three connections and every value
* [The classic screens](docs/CLASSIC.md): how the screens of the MS-DOS program are reproduced and checked
* [Changelog](CHANGELOG.md)

## 🤝 Contributing & Community

* Before a pull request run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `python -I tools/ci_validate.py`: all must pass. See [CONTRIBUTING.md](CONTRIBUTING.md) and the [code of conduct](CODE_OF_CONDUCT.md).
* The README exists in eight languages: a change to the text is made in all eight files, and the two pictures are drawn from `docs/architecture.json` and `docs/structure.json` by `tools/make_architecture.py` and `tools/make_structure.py`.
* Do not describe as working on a machine what has only been tried on the simulator.
* Report a vulnerability privately: [SECURITY.md](SECURITY.md). Questions, ideas and reports: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTHOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENSE

Software: GPL-3.0-or-later. See [LICENSE](LICENSE). Hardware designs of the suite use CERN OHL-S.
