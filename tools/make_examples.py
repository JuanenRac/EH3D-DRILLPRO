#!/usr/bin/env python3
"""Writes the example drill files of the release into examples/, from the simplest to the largest.

The files are generated, not copied from anywhere: run `python3 tools/make_examples.py` and commit the result. Everything is in mil
(1 mil = 0.001 inch), the unit of the default configuration, and inside the 495 x 325 mm work area.
"""
import math
import os
import random

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "examples")


def write(name, tools, note):
    """tools: {number: [(x, y), ...]} written in tool order."""
    lines = ["%"]
    total = 0
    for number, holes in tools.items():
        lines.append(f"T{number:02d}")
        for x, y in holes:
            lines.append(f"X{int(round(x))}Y{int(round(y))}")
        total += len(holes)
    lines.append("M30")
    with open(os.path.join(OUT, name), "w", newline="\r\n") as f:
        f.write("\n".join(lines) + "\n")
    return name, total, len(tools), note


def grid(cols, rows, pitch, x0=1000, y0=1000):
    return [(x0 + c * pitch, y0 + r * pitch) for r in range(rows) for c in range(cols)]


def circle(cx, cy, r, n):
    return [(cx + r * math.cos(2 * math.pi * i / n), cy + r * math.sin(2 * math.pi * i / n)) for i in range(n)]


def main():
    os.makedirs(OUT, exist_ok=True)
    rows = []
    rows.append(write("01_one_hole.ncd", {1: [(1000, 1000)]}, "one hole: the smallest job, to see that everything moves"))
    rows.append(write("02_three_holes.ncd", {1: [(1000, 1000), (2000, 1000), (2000, 2000)]}, "three holes with one tool"))
    rows.append(
        write(
            "03_two_tools.ncd",
            {1: [(1000, 1000), (3000, 1000), (3000, 3000), (1000, 3000)], 2: [(2000, 2000), (2000, 1000)]},
            "two tools: the program asks for the change (or the changer does it)",
        )
    )
    rows.append(write("04_grid_10x10.ncd", {1: grid(10, 10, 100)}, "100 holes on a 0.1 inch grid"))
    # a small realistic board: two 20-pin headers, four mounting holes and a few vias
    board = {
        1: grid(20, 1, 100, 1500, 1500) + grid(20, 1, 100, 1500, 1800) + [(2000 + 150 * i, 2600) for i in range(6)],
        2: [(300, 300), (4300, 300), (300, 3300), (4300, 3300)],
        3: circle(2500, 2200, 400, 8),
    }
    rows.append(write("05_small_board.ncd", board, "a small board: pin headers, mounting holes and vias with three tools"))
    rows.append(
        write(
            "06_four_tools_80_holes.ncd",
            {t: [(1000 + 150 * i, 1000 + 400 * t) for i in range(20)] for t in range(1, 5)},
            "four tools of 20 holes each: many tool changes",
        )
    )
    random.seed(1)
    dense = {1: [(random.randrange(500, 18000), random.randrange(500, 11500)) for _ in range(600)]}
    dense[2] = [(random.randrange(500, 18000), random.randrange(500, 11500)) for _ in range(400)]
    rows.append(write("07_dense_1000_holes.ncd", dense, "1000 holes scattered over the whole table with two tools"))
    random.seed(2)
    big = {t: [(random.randrange(300, 18500), random.randrange(300, 12000)) for _ in range(n)] for t, n in ((1, 6000), (2, 3000), (3, 1000))}
    rows.append(write("08_large_10000_holes.ncd", big, "10000 holes with three tools: the sorting and the job planning with a big file"))
    random.seed(3)
    huge = {t: [(random.randrange(300, 18500), random.randrange(300, 12000)) for _ in range(5000)] for t in range(1, 5)}
    rows.append(write("09_huge_20000_holes.ncd", huge, "20000 holes with four tools: the largest example"))

    with open(os.path.join(OUT, "README.txt"), "w", newline="\r\n") as f:
        f.write("Example drill files of DRILLPRO (Excellon style, in mil: 1 mil = 0.001 inch)\r\n\r\n")
        f.write("Load them with the Load button. They are listed from the simplest to the largest.\r\n")
        f.write("Start with the first ones, in the simulator (the default), before sending any job to a machine.\r\n\r\n")
        for name, total, ntools, note in rows:
            f.write(f"{name:<28} {total:>6} holes, {ntools} tool(s): {note}\r\n")
    for r in rows:
        print(r)


if __name__ == "__main__":
    main()
