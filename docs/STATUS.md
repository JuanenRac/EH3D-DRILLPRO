# EH3D-DRILLPRO: status

## Current state

EH3D-DRILLPRO is a working program on a computer: the screens of the original in a window (`--features gui`) and in a terminal (`--features console`), a command line to plan on paper, and a protocol crate that runs a whole job against a simulator, including a wrapper that makes the link fail on purpose. The reference run of the three axes and the moves of every axis were tried on one real Bungard CCD MTC, by the README; the manifest still records `simulator` because a whole job has not run, and the rack of the ATC models has not run on a real machine. The suite of tests runs on Windows and Linux in the CI.

## Capabilities

The official state of each capability is in `REQUIREMENTS.md`. The register has 47 requirements: 32 implemented, 9 prototype, 6 planned and 0 validated. By progress: 32 done, 9 partial, 0 pending and 6 blocked. This page never states a state that `REQUIREMENTS.md` does not.

## Open questions

* Whether the reference run and the moves of every axis, which were tried on one real machine, are recorded as bench evidence, and by whom.
* Which parts of the protocol that are hypotheses in `docs/PROTOCOL.md` should be confirmed first on the machine.
* When the Excellon reader of this repository is replaced by the one of the shared libraries.

## What it needs from other repositories

* `EH3D-COMMON`: the archive, formats, fleet, levelling, summary, view and server client crates, as a sibling folder at the version named in the workflows.
* A Bungard CCD machine and a person, to run a whole job and confirm the steps per millimetre, the spindle and the vacuum outputs.
* Files from real boards, to try the readers on.

## Known technical debt and omitted tests

* The steps per millimetre are only checked against the original's configuration, and the spindle and vacuum outputs are untried.
* The file readers have only been tried with small sample files written for the tests.
* The server connection has only been exercised against the simulator.
* The tool changer of the ATC models has not run on a real machine.
* Chinese and Japanese need a CJK font among the system fonts.
* Tests that need a real machine or a screen are not written: the register names each as `planned:` with the test that will show it.
