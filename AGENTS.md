# AGENTS.md: rules for anyone who changes EH3D-DRILLPRO

EH3D-DRILLPRO: the screens, the job planning and the machine protocol of the Bungard CCD drilling machines. Software licence: GPL-3.0-or-later; every source file starts with its header.

## 1. Before you start

1. Read `docs/STATUS.md`: what exists, what is missing, what the last change decided.
2. Read `docs/SCOPE.md` (what the program is for and what it never decides) and `docs/REQUIREMENTS.md` (the register of requirements and the test that shows each one).
3. Read `docs/NOT_YET.md` before adding an idea: it may be there on purpose.

## 2. Build and test

- The repository builds next to `EH3D-COMMON` (a sibling folder); the version it needs is in the workflows (`SIBLING_REFS`) and in `eh3d.project.json`. The protocol crate (`protocol/`) is a member of the workspace.
- Run before every commit, with the same flags as the CI: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --features console -- -D warnings`, `cargo clippy --workspace --all-targets --features gui -- -D warnings`, `cargo test --workspace --all-targets --features console` and `cargo test --workspace --all-targets --features gui`, `python -I tools/ci_validate.py` and `python tools/make_structure.py --check --readmes`.
- A heavy build runs in the CI, not on a machine that is short of memory; a change that adds a file adds it to `docs/structure.json` and regenerates the tree and the picture.

## 3. Code rules

- A reader never panics and never guesses. What it cannot understand is an error that names what and, when it has one, the number of its line.
- Every input file is hostile until it is shown otherwise: sizes, counts, number formats and nesting are checked before use, and everything is bounded.
- No `unwrap` or `expect` in code that handles an external value. No `unsafe` (the crate forbids it).
- A file that exists is never written over by accident, and a file that is read is never written to.
- Every value that is tuned (a limit, a tolerance, a time) is a setting in a configuration file, not a constant in the code; every option of security has a switch with the conservative value as its default.
- The safety gate of the protocol only ever makes motion refused: nothing moves until everything is verified and the machine is armed, and a change that would move without that is a defect; the tests that fail for it must stay.
- The classic screens are checked cell by cell against recordings of the original: a change to a screen changes its golden file in the same commit and says why.
- Output carries no date, host name or user name that the person did not give.

## 4. What the program must not do

- It does not move a real machine unless the machine file says so and the machine is armed.
- It does not copy code of the original program: the screens and the protocol are written from recordings of what it does.
- Its documents and its code do not name anything private: no dates, no internal notes, no names of people or of tools that wrote the change.

## 5. Evidence

- A state in `docs/REQUIREMENTS.md` is `validated` only when a person has run the capability on the real machine it names. Nothing is green that has never run on the real thing.
- A requirement names a test that exists; when a requirement changes state, the row and `docs/STATUS.md` change in the same commit, and `tests/reqs.rs` says whether they agree.

## 6. Documents and languages

- The README exists in eight languages (English, Spanish, German, French, Italian, Japanese, Chinese and Portuguese of Portugal) and every word of the window and of the messages exists in the eight where the program has them.
- The file tree and the pictures are generated from `docs/structure.json` and `docs/architecture.json`; they are not edited by hand. The count of tests in the READMEs is the real one.
- Each change has a line in the `[Unreleased]` part of `CHANGELOG.md`, in English.

## 7. Clean room

- Ideas, flows and behaviours that can be seen from outside may be taken from other programs; their code, their texts, their files and their wording are not copied.
- Formats are learned from public specifications and from files of this project. The files of another program are read, never written.

## 8. Reporting

- A problem with a part of the program is reported with a small file that shows it. Real boards and real machine files are private: do not paste them into a report.
