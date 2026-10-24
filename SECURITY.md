# Security policy

EH3D-DRILLPRO reads files that other people may send: Excellon `.NCD` drill files, the tool table and the machine configuration. The readers are written not to panic on malformed input; a line they do not understand is reported, not guessed.

What this program does **not** do: it does not drive a real machine, it is not a safety system and it does not handle limit switches or an emergency stop. Do not connect a machine to it.

## Reporting a problem

Please report a vulnerability privately to **electrohobby3d@gmail.com**, with the file or the steps that show it. Do not open a public issue for it. You will get an answer as soon as possible; this is a one-person project.
