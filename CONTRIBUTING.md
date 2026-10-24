# Contributing to EH3D-DRILLPRO

Thank you for looking. This program is meant to drive machines one day, so changes are small and carefully tested.

1. **Nothing moves a real machine without proof.** A change that sends bytes to a real control unit needs a note of what was observed on hardware (or on a logic analyser) and a test with the simulator. Until then the machine link stays in simulation.
2. **Every behaviour has a test**, including the refusals: a file that does not parse, a move before the machine is armed, a missing language text.
3. **Before a pull request:** `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `python -I tools/ci_validate.py` must pass.
4. **Texts exist in eight languages** (English, Spanish, French, Italian, German, Chinese, Japanese and European Portuguese) wherever the program shows text to people. The README files keep the same structure.
5. **No files of other people's software.** Sample files in `testdata/` are written for the tests; do not add programs, manuals or demo files of the original software.
6. Commits are written in English, small and about one thing.
