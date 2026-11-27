# EH3D-DRILLPRO: scope

Status: **simulator**. EH3D-DRILLPRO is a program with the screens of the MS-DOS software of the Bungard CCD drilling machines, written again from recordings of what that software does, and a protocol crate that drives the machine. Everything it does runs against a simulator by default. The reference run of the three axes and the moves of every axis have been tried on one real Bungard CCD MTC, as the README says; a whole drilling job has not run on a machine, so nothing here is recorded as validated. This page says what it is for, what it will not do and where it sits among the other programs of the suite.

## Purpose

The program of the suite for the Bungard CCD drilling machines (MTC and CCD/2 MTC, with the tool changer of the ATC models written from a recording): it reads Excellon drill files, the tool table and the machine file, orders the holes, shows them on the classic 80x25 screens or in a menu-driven interface and runs the whole job against a simulator or, when the machine file says so, on a real machine through a serial port, a USB adapter, the EH3D node or the EH3D server.

## What it does

* Reads Excellon `.NCD` files (with or without the `M48` header, modal coordinates, imperial units), the tool table `DRILLPRO.DTD` and the machine file `DRILLPRO.DCF`, and refuses files that are too big or damaged with a message.
* Plans the job (the holes in the order of the file, X first, Y first or the shortest path; grouped by tool; panels; tabs; a job that goes on after a stop) and prints it on paper with `drillpro-plan` with no machine.
* Shows the classic screens, checked cell by cell against the original, with the keyboard and the mouse, in eight languages, and a console version for a terminal.
* Drives the machine through the protocol crate: byte transport, motion planning in millimetres with acceleration ramps, a reference run, and a safety gate that refuses motion until it is armed; the whole job runs to the end on the simulator.
* Chooses how the job reaches the machine (simulator, serial port, USB adapter, EH3D node or EH3D server) from the machine file and the Config window, with a visible warning when the token would go over a plain connection.
* Checks the plan before it starts (the summary and the dry run, the course of the machine, the tool table and the rack) and refuses a plan the server would refuse.
* Works the tool changer of the ATC models from a recording of the original, and the teach screen that jogs the head and sets an offset.

## What it will not do

* No decision about the limits of a machine, its interlocks or its emergency stop (see below).
* No drilling of a whole job on a real machine has been recorded: the evidence of every capability is `simulator` or `host-test`.
* No coverage of the XL models of the machines.
* No code of the original program: it is written again from recordings of what the original does.
* No sending of motion to a real machine unless the machine file says so and the safety gate is armed.

## Where it sits

* **It reads:** Excellon drill files (also inside a ZIP archive), the tool table, the machine files `DRILLPRO.DCF` and `machine.cfg`, DXF and Gerber outlines, height maps and, when asked, the server of the suite.
* **It writes:** the tool table and the machine file (whole, through a temporary file, keeping the lines it does not own), the plan on paper, a crash log in the configuration folder and, to a machine, bytes through the protocol crate; never over a drill file.
* **It is read by:** ROUTEPRO and the modern interface share the machine file; the EH3D server runs jobs sent to it; programs exchange work through files or through the server, never by depending on each other.
* **It depends on:** `EH3D-COMMON` (the archive, Bungard formats, fleet, levelling, summary, view and server client crates), built next to it as a sibling folder, and the protocol crate in this repository.

## What it never decides

The limits of a machine, its interlocks and its emergency stop belong to the machine and to its control unit, not to this program. The safety gate here can only refuse motion until the machine is armed, and `Esc` stops a job and switches the stepper drivers off, but no check here replaces the emergency stop of the machine or the judgement of the person who stands at it. Parts of `docs/PROTOCOL.md` are still hypotheses, and the steps per millimetre are only checked against the original configuration.

The requirements of the program, with the state of each and the test that shows it, are in `REQUIREMENTS.md`; the state in a paragraph is in `STATUS.md`.
