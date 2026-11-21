# EH3D-DRILLPRO: not yet

Good ideas that are not in the current version. Writing them here keeps anybody from implementing them early or twice.

| Idea | Why not yet | Revisit when |
|---|---|---|
| A whole drilling job on a real machine | Only the reference run and the moves of every axis were tried on one machine; the steps per millimetre, the spindle and the vacuum are untried | a person runs a test board on the machine, following the hardware checklist |
| The tool changer of the ATC models on a real machine | It is written from a recording of the original and has never run on a real machine | a person with an ATC machine runs it |
| The XL models | They are not covered by the recordings the program is written from | a machine of that model is recorded |
| The readers tried with files from real boards | Only small sample files written for the tests have been used | a person gives files from real boards |
| The server connection with a real server and machine | It has only been exercised against the simulator | the server drives a real machine |
| The Excellon reader of the shared libraries | This repository has its own reader, which the tests cover; replacing it needs the shared one to read the same files | the shared reader is ready and reads every example |
| Confirming the hypotheses of the protocol | Parts of `docs/PROTOCOL.md` are recorded as hypotheses; the protocol layer refuses to move anything until it has been armed | each is checked on the machine |
| A font for Chinese and Japanese inside the program | The program uses the CJK font of the system, which keeps the program small | a system without such a font has to be supported |
