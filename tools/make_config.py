#!/usr/bin/env python3
"""Writes the basic configuration of the release into config/: DRILLPRO.DCF (the machine), DRILLPRO.DTD (the tools) and a machine.cfg.

Every parameter is explained in plain words after its value (what it is, the unit, a typical value, what happens if it is wrong). The
files are plain text in the layout the program reads; run `python3 tools/make_config.py` and commit the result.
"""
import os
import shutil

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "config")

# (value, explanation) of every data line, in the order the program reads them
DCF = [
    ("0000001", "SERIAL NUMBER of your machine. Only a label shown on the main screen; write anything up to 12 characters."),
    ("EH3D.000000A", "VERSION label of this file. Only a label; leave it."),
    (r".\examples", "FOLDER where the Load button looks for your drill files first. Relative to the folder you start the program from. Example: .\\examples"),
    ("*.NCD", "FILE PATTERN of the Load list. *.NCD shows the NCD files; *.DRL shows the DRL files. The signs * and ? stand for any letters."),
    ("1.0000", "SCALE X of the job. 1.0000 = the drawing as it is; 1.0010 makes it 0.1 percent bigger. From 0 to 10. Use it to correct a film that shrank."),
    ("1.0000", "SCALE Y of the job (same as X)."),
    ("02.50", "OFFSET X in mm: how far from the home corner the board starts. Typical 2.50. From -495 to 495. The Offset button sets it by moving the head."),
    ("02.50", "OFFSET Y in mm (same as X). From -325 to 325."),
    ("495.00", "CLIPPING X in mm: the usable length of your table in X. Nothing farther than this is ever drilled. Set it to your real table (CCD 495, CCD/2 325)."),
    ("325.00", "CLIPPING Y in mm: the usable width of your table in Y (CCD 325, CCD/2 270)."),
    ("1.00", "BOARD THICKNESS in mm. The drill goes through the board plus the extra stroke of the tool. 1.50 is the usual 1.6 mm board."),
    ("12.00", "BASE SHEET THICKNESS in mm: the sacrificial plate under the board. The drill goes into it by the extra stroke only, never deeper."),
    ("4000", "XY SPEED, FASTEST (mm/min) when moving between holes. If the motors lose steps or stall, lower it (try 2500). Too high = missed holes."),
    ("1000", "XY SPEED of the REFERENCE run (mm/min): the slow search for the home switches. Leave it."),
    ("1000", "XY START SPEED of the ramp (mm/min): the speed a move begins with before it speeds up. Leave it."),
    ("5000", "Z SPEED, FASTEST (mm/min): the quick rise and approach of the drill. Lower it if Z stalls."),
    ("1000", "Z SPEED of the REFERENCE run (mm/min): the slow search of the Z home switch. Leave it."),
    ("1000", "Z START SPEED of the ramp (mm/min). Leave it."),
    ("10", "ACCELERATION of X and Y (how gently they speed up). Higher = harder start. If the motors stall at the start of a move, try 5."),
    ("20", "ACCELERATION of Z (same idea as X and Y)."),
    ("0.9992", "CORRECTION X: calibration so that 100 mm drilled are 100 mm. Drill two holes 200 mm apart, measure, multiply this by wanted/measured. 1.0000 = none."),
    ("0.9998", "CORRECTION Y (same as X)."),
    ("0.8988", "CORRECTION Z (the same for the depth). Leave it unless you measured a depth error."),
    ("37.30", "MAXIMUM Z DEPTH in mm: the whole travel of the Z axis from its top switch. Too large drives the drill into the table."),
    ("16.00", "DISTANCE TOOL TO BOARD in mm: how far above the board the quick approach stops and the slow drilling feed begins."),
    ("0.50", "SPINDLE START TIME: seconds to wait for each 10000 rpm for the spindle to reach its speed after it starts."),
    ("1", "UNIT of the numbers in your drill files: 0 = 1/10 mil, 1 = mil (0.001 inch, the usual one), 2 = 0.025 mm, 3 = 0.01 mm. A wrong unit puts the holes in the wrong place."),
    ("3", "ORDER OF DRILLING: 0 as in the file, 1 X first, 2 Y first, 3 special (a meander that saves travel: the usual one)."),
    ("0 0", "RESERVED. Do not change."),
    ("4", "CONNECTION: 0 = COM1, 1 = COM2, 2 = COM3, 3 = COM4, 4 = NO MACHINE (the simulator: safe, try anything). A real machine is chosen in the machine file."),
    ("2", "SERIAL SPEED: 0 = 9600 baud, 1 = 19200, 2 = 38400 (the one the control unit uses)."),
]

# comment-only lines placed before the data line with that index
SECTIONS = {
    0: ["* DRILLPRO machine configuration, explained line by line.",
        "* Each line is a value, some spaces, a * and what it means. Change only the value, keep the * and the text after it.",
        "* Save a copy of this file before you change anything. The decimal separator is a point.",
        "*"],
    4: ["* MAIN SCREEN"],
    12: ["* MACHINE CONSTANTS"],
    26: ["* UNITS AND ORDER"],
    29: ["* SERIAL LINE"],
}

DTD = [
    "Basic tool table of EH3D-DRILLPRO",
    "",
    "Written for this program; it is not a copy of any other file.",
    "",
    "Table of tool data. 15 lines of data begin at line 20; lines 1 to 19 are comments like these.",
    "Each data line is four numbers separated by spaces, with a point for decimals:",
    "",
    "nn   d.d   vvvv   sssss",
    "",
    "nn    = the tool number in your drill file (two digits): T01 in the file uses the line that begins 01",
    "d.d   = extra stroke in mm: how far the tip goes below the board into the base sheet (0.5 is usual for small drills)",
    "vvvv  = feed of the drill going down in mm/min (smaller for small, fragile drills: 500 to 1500; larger ones go faster)",
    "sssss = spindle speed in turns per minute: 30000 to 60000 on the CCD",
    "",
    "A tool that is not in the table gets a safe default. Change the numbers, never the number of lines.",
    "Too fast a feed breaks small drills; too slow burns the board. Try on a scrap board first.",
    "",
    "Delimiter between the numbers: one or more spaces.",
    "",
    "04 0.5 1200 60000",
    "06 0.5 2400 60000",
    "07 0.5 3600 60000",
    "08 0.5 3600 60000",
    "09 0.5 3600 60000",
    "10 1.0 3600 60000",
    "12 1.0 3500 60000",
    "13 1.0 3500 50000",
    "15 1.0 3600 40000",
    "20 1.2 1500 60000",
    "25 1.5 1000 60000",
    "30 1.5 0500 60000",
    "01 0.0 1000 60000",
    "02 0.0 1000 60000",
    "03 0.0 1000 60000",
]


def write(name, lines):
    with open(os.path.join(OUT, name), "w", newline="\r\n", encoding="latin-1") as f:
        f.write("\n".join(lines) + "\n")


def main():
    os.makedirs(OUT, exist_ok=True)
    lines = []
    for i, (value, text) in enumerate(DCF):
        for c in SECTIONS.get(i, []):
            lines.append(" " * 14 + c)
        lines.append(f"{value:<14}* {text}")
    lines.append(" " * 14 + "* End of the configuration file.")
    write("DRILLPRO.DCF", lines)
    assert len(DTD) == 19 + 15, len(DTD)
    write("DRILLPRO.DTD", DTD)
    shutil.copyfile(os.path.join(ROOT, "docs", "machine.cfg.example"), os.path.join(OUT, "machine.cfg.example"))
    print("config written:", sorted(os.listdir(OUT)))


if __name__ == "__main__":
    main()
