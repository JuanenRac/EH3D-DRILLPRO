# Changelog

All notable changes to this project are documented here.

## [0.0.1] - First version: file readers, job planning, text-mode interface and a simulated machine

- **Excellon reader:** `.NCD` drill files with or without the `M48` header, modal coordinates and imperial units of 1 mil, with the tool table (`.DTD`) and the machine configuration (`.DCF`).
- **Job planner:** orders the holes as in the file, X first, Y first or by shortest path, grouped by tool.
- **Interface:** an 80x25 text-mode screen with CGA colours, menus, a file dialog and a vector view of the holes, in eight languages (English, Spanish, Italian, German, French, Portuguese, Chinese, Japanese).
- **Protocol library (`eh3d-drillpro-protocol`):** byte transport (serial port or simulator), motion planning in millimetres with acceleration ramps, and a safety gate that refuses motion until it is armed with verified settings.
- **Simulator:** the whole job runs to the end without hardware.
- **Real machine:** a machine file (`machine.cfg`, shared by DRILLPRO and ROUTEPRO) chooses the simulator, the computer's serial port, a USB serial adapter or the EH3D-NODE-BUNGARD; every tuning value lives in it. `Esc` stops a drilling job and switches the drivers off, and a red banner says the machine is real.
- **Reference run:** Z, X and Y to their limit switches with the control unit's "all home" line read once the last search has gone out (reading it in the middle of the stream made the Y motor lose steps on the CH340), a short unstick move when parked, a Z search that covers the whole travel (`z_search_steps`) and a smooth calibration jump. Tried on a Bungard CCD MTC; the CCD/2 MTC is driven the same way and has not been tried. Spindle and vacuum outputs have not been tried; the two ATC models are not covered.
- **Spindle of the node:** `spindle = node` in the machine file makes the job ask the EH3D-NODE-BUNGARD for the spindle (a variable-frequency drive on the node's own outputs, any speed up to the node's maximum) instead of writing the control unit's spindle bits; the spindle is stopped on any stop. The node link also reads the machine's lamps and can ask the node to drop the rest of a search as soon as an axis is home.
- **Server connection:** `connection = server` sends the drilling job to the EH3D-SERVER, which plans it with the constants of the machine, runs it through the machine's node and tells the program how it goes; the program shows the progress, answers the dialogs and can stop the job, and closing it does not stop the job.
- **Steady motion:** moves go out as one continuous stream paced by the adapter's UART, Z moves byte by byte.
- **Documentation:** protocol notes, the connection guide with the CH340 findings, and a machine file example with every value.
- 123 tests, `cargo clippy -D warnings` and `cargo fmt --check` in CI on Linux and Windows, and a repository check of the manifest, the versions, the changelog and the eight README files.
- **Ready-made packages for Windows, Linux and macOS:** the `Release binaries` workflow builds the program with its window for Windows (zip), Linux (tar.gz; X11 and Wayland) and macOS (Apple silicon and Intel, tar.gz), each package with its SHA-256, and the release carries them.
