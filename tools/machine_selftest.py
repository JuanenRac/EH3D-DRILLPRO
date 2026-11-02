#!/usr/bin/env python3
"""Machine-day self-test of DRILLPRO: the machine file (DRILLPRO.DCF), checked against the model of the program, and a stopwatch guide to measure it.

Reads only. It never moves the machine and never writes the DCF.

    python tools/machine_selftest.py                          # check config/DRILLPRO.DCF (and the tool table beside it) against the model
    python tools/machine_selftest.py --dcf PATH --machine-cfg machine.cfg --spindle vfd
    python tools/machine_selftest.py --measure                # step-by-step measuring with a stopwatch; writes a PROPOSED file, a new one

The model (the same numbers the program uses): 1000/25.4 steps per mm; the control unit takes one byte per step, at most 3840 steps a second at 38400 baud;
ACCELERATION scales the ramp: shipped ramp steps are 40 for X and Y at ACCELERATION 10 and 200 for Z at 20, so ramp steps = 400 / A (X, Y) and 4000 / A (Z);
SPINDLE START TIME is the seconds the job waits for each 10000 rpm.

The report machine_selftest_<time>.txt is written to the current folder: one line per check (OK, FALLO, OMITIDO) with the value.
"""
import argparse
import os
import re
import sys
import time

STEPS_PER_MM = 1000.0 / 25.4
LINE_HZ = 3840.0                      # one byte per step at 38400 baud 8N1
SHIPPED_RAMP_XY, SHIPPED_ACCEL_XY = 40, 10
SHIPPED_RAMP_Z, SHIPPED_ACCEL_Z = 200, 20
SPINDLES = {"original": 63000, "vfd": 24000}   # the top speed of each spindle (the original runs only from the control unit)
MAX_START_WAIT_S = 60.0                # the program caps the wait for the spindle at one minute

# ---- COMMON BEGIN (identical in every node repo; do not edit one copy alone) ----
OK = "OK"
FAIL = "FALLO"
SKIP = "OMITIDO"


class Fail(Exception):
    """A step that did not pass."""


class Skip(Exception):
    """A step that could not run (and why)."""


class Report:
    """One line per step. Every text that goes out is cleaned of the secrets the run knows."""

    def __init__(self, node, host):
        self.node, self.host = node, host
        self.secrets = []
        self.lines = []
        self.counts = {OK: 0, FAIL: 0, SKIP: 0}
        self.started = time.strftime("%Y-%m-%d %H:%M:%S")

    def hide(self, *values):
        for v in values:
            if isinstance(v, str) and len(v) >= 4 and v not in self.secrets:
                self.secrets.append(v)

    def clean(self, text):
        text = str(text)
        for s in sorted(self.secrets, key=len, reverse=True):
            text = text.replace(s, "***")
        return text

    def add(self, status, name, detail=""):
        line = "%-9s%s" % (status, name) + (" | " + detail if detail else "")
        line = self.clean(line)
        try:
            print(line, flush=True)
        except (OSError, UnicodeError):
            pass   # a closed pipe or a console that cannot show a character must not stop the run: the report file has the line
        self.lines.append(line)
        self.counts[status] += 1

    def step(self, name, fn):
        """Runs fn; it returns the measured value (text), or raises Fail / Skip."""
        try:
            self.add(OK, name, fn() or "")
        except Skip as e:
            self.add(SKIP, name, str(e))
        except Fail as e:
            self.add(FAIL, name, str(e))
        except Exception as e:  # a broken answer or a dead node is a failed step, never a crash
            self.add(FAIL, name, "%s: %s" % (type(e).__name__, e))

    def text(self):
        head = ["selftest %s" % self.node, "host: %s" % self.host, "started: %s" % self.started, ""]
        tail = ["", "OK=%d FALLO=%d OMITIDO=%d" % (self.counts[OK], self.counts[FAIL], self.counts[SKIP])]
        return self.clean("\n".join(head + self.lines + tail)) + "\n"

    def write(self, out_dir):
        path = os.path.join(out_dir, "%s_%s_%s.txt" % (getattr(self, "prefix", "selftest"), self.node, time.strftime("%Y%m%d_%H%M%S")))
        with open(path, "w", encoding="utf-8") as f:
            f.write(self.text())
        return path


def confirm(prompt, yes=False, ask=input):
    """The interactive gate of every active test: the person types SI. --yes (asked for expressly) skips the question; no answer is a no."""
    print(prompt, flush=True)
    if yes:
        print("(--yes: confirmed by whoever runs this)", flush=True)
        return True
    try:
        return ask("Escribe SI para seguir: ").strip() == "SI"
    except (EOFError, KeyboardInterrupt):
        return False




# the data lines of a DCF, in order (the comment after the value is the program's own explanation)
FIELDS = ["serial", "version", "folder", "pattern", "scale_x", "scale_y", "offset_x", "offset_y", "clip_x", "clip_y", "board_mm", "base_mm",
          "xy_fast", "xy_ref", "xy_start", "z_fast", "z_ref", "z_start", "accel_xy", "accel_z", "corr_x", "corr_y", "corr_z", "z_depth", "tool_to_board",
          "spindle_start", "unit", "order", "reserved", "connection", "baud"]
NUMERIC = set(FIELDS[4:]) - {"reserved"}


def split_value(line):
    """'value   * comment' -> value (the comment starts at a * that follows a blank and is followed by a blank or the end)."""
    blanks = " " + chr(9) + chr(13)
    prev = True
    for i, c in enumerate(line):
        if c == "*" and prev and (i + 1 >= len(line) or line[i + 1] in blanks):
            line = line[:i]
            break
        prev = c in blanks
    return line.strip()


def parse_dcf(text):
    """Returns (values, lines) where values maps field -> float/str and lines is [(line index, field)] for rewriting. Raises ValueError on a malformed file."""
    values, where = {}, {}
    n = 0
    for idx, raw in enumerate(text.splitlines()):
        v = split_value(raw)
        if not v:
            continue
        if n >= len(FIELDS):
            break
        name = FIELDS[n]
        where[name] = idx
        if name in NUMERIC:
            try:
                values[name] = float(v)
            except ValueError:
                raise ValueError("%s (data line %d) is not a number: %r" % (name, n + 1, v))
        else:
            values[name] = v
        n += 1
    if n < len(FIELDS):
        raise ValueError("only %d data lines, a DCF has %d" % (n, len(FIELDS)))
    return values, where


def parse_dtd(text):
    """The tool table: lines 'NN stroke feed rpm'. Returns [(tool, stroke, feed, rpm)]."""
    rows = []
    for raw in text.splitlines():
        m = re.fullmatch(r"\s*(\d{1,3})\s+([0-9.]+)\s+(\d+)\s+(\d+)\s*", raw)
        if m:
            rows.append((int(m.group(1)), float(m.group(2)), int(m.group(3)), int(m.group(4))))
    return rows


def parse_machine_cfg(text):
    out = {}
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line[0] in "#;" or "=" not in line:
            continue
        k, _, v = line.partition("=")
        out[k.strip()] = v.strip()
    return out


def mmin_to_hz(mm_min):
    return mm_min / 60.0 * STEPS_PER_MM


def ramp_steps(accel, shipped_steps, shipped_accel):
    return shipped_steps * shipped_accel / accel


def implied_accel_mm_s2(fast_mm_min, start_mm_min, steps):
    """The acceleration the ramp gives: the speed climbs from start to fast over `steps` steps."""
    v, v0 = fast_mm_min / 60.0, start_mm_min / 60.0
    d = steps / STEPS_PER_MM
    return (v * v - v0 * v0) / (2.0 * d)


def accel_value_from_measure(distance_mm, seconds, fast_mm_min, start_mm_min, shipped_steps, shipped_accel):
    """From a move of distance_mm at full speed that took `seconds` (a ramp up and a ramp down): the measured acceleration (mm/s^2) and the DCF ACCELERATION value."""
    v = fast_mm_min / 60.0
    if seconds <= 0 or distance_mm <= 0:
        raise ValueError("the distance and the time must be positive")
    ramp_time = seconds - distance_mm / v
    if ramp_time <= 0:
        raise ValueError("the move took no longer than at constant full speed: the machine is not reaching full speed, or the time is too short")
    a = v / ramp_time          # symmetric ramps: T = D/v + v/a
    v0 = start_mm_min / 60.0
    steps = (v * v - v0 * v0) * STEPS_PER_MM / (2.0 * a)
    return a, shipped_steps * shipped_accel / steps


def spindle_time_from_measure(rpm, seconds):
    """SPINDLE START TIME: the seconds for each 10000 rpm."""
    if rpm <= 0 or seconds <= 0:
        raise ValueError("the speed and the time must be positive")
    return seconds / (rpm / 10000.0)


def check_dcf(rep, values, dtd, mcfg, spindle):
    def need(cond, text):
        if not cond:
            raise Fail(text)

    def clip():
        need(0 <= values["offset_x"] < values["clip_x"] and 0 <= values["offset_y"] < values["clip_y"], "offset is not inside the clipping")
        return "table %.1f x %.1f mm after the offset" % (values["clip_x"] - values["offset_x"], values["clip_y"] - values["offset_y"])

    rep.step("DCF: offset and clipping", clip)

    def speeds(prefix, label):
        fast, ref, start = values[prefix + "_fast"], values[prefix + "_ref"], values[prefix + "_start"]
        need(start > 0 and fast > 0 and ref > 0, "a speed is zero or negative")
        need(start <= fast and ref <= fast, "the start or reference speed is above the fastest")
        hz = mmin_to_hz(fast)
        need(hz <= LINE_HZ, "%s fastest %.0f mm/min is %.0f steps/s, above the %.0f the line carries" % (label, fast, hz, LINE_HZ))
        limit = mcfg.get("xy_hz_limit" if prefix == "xy" else "z_hz_limit")
        if limit is not None:
            need(hz <= float(limit), "%s fastest is %.0f steps/s, above the %s of the machine file" % (label, hz, limit))
        return "%s fastest %.0f mm/min = %.0f steps/s (line limit %.0f), start %.0f, reference %.0f" % (label, fast, hz, LINE_HZ, start, ref)

    rep.step("DCF: XY speeds against the line", lambda: speeds("xy", "XY"))
    rep.step("DCF: Z speeds against the line", lambda: speeds("z", "Z"))

    def accel(axis):
        a = values["accel_" + axis]
        need(a > 0, "ACCELERATION is zero or negative")
        if axis == "xy":
            steps = ramp_steps(a, SHIPPED_RAMP_XY, SHIPPED_ACCEL_XY)
            mm_s2 = implied_accel_mm_s2(values["xy_fast"], values["xy_start"], steps)
            travel = min(values["clip_x"], values["clip_y"])
        else:
            steps = ramp_steps(a, SHIPPED_RAMP_Z, SHIPPED_ACCEL_Z)
            mm_s2 = implied_accel_mm_s2(values["z_fast"], values["z_start"], steps)
            travel = values["z_depth"]
        mm = steps / STEPS_PER_MM
        need(mm * 2 <= travel, "the ramp (%.2f mm up and down) does not fit in the travel %.1f mm" % (mm, travel))
        return "ACCELERATION %g -> ramp %.0f steps (%.2f mm), about %.0f mm/s2" % (a, steps, mm, mm_s2)

    rep.step("DCF: acceleration XY against the model", lambda: accel("xy"))
    rep.step("DCF: acceleration Z against the model", lambda: accel("z"))

    def start_time():
        t = values["spindle_start"]
        need(t > 0, "SPINDLE START TIME is zero: the job would not wait for the spindle")
        top = SPINDLES[spindle]
        wait = top / 10000.0 * t
        need(wait <= MAX_START_WAIT_S, "the wait at %d rpm is %.1f s, above the %g s cap" % (top, wait, MAX_START_WAIT_S))
        detail = "SPINDLE START TIME %.2f s per 10000 rpm -> %.1f s to %d rpm" % (t, wait, top)
        if "spindle_start_wait_ms" in mcfg:
            detail += "; machine file waits %s ms when the DCF gives none" % mcfg["spindle_start_wait_ms"]
        return detail

    rep.step("DCF: spindle start time", start_time)

    def rpm_range():
        if not dtd:
            raise Skip("no tool table found beside the DCF")
        top = SPINDLES[spindle]
        over = [(t, r) for t, _, _, r in dtd if r > top]
        need(not over, "%d tools ask for more than the %s spindle gives (%d rpm): %s" % (len(over), spindle, top, ", ".join("T%02d=%d" % x for x in over[:5])))
        rpms = [r for _, _, _, r in dtd]
        return "%d tools, spindle %d..%d rpm (limit %d, %s)" % (len(dtd), min(rpms), max(rpms), top, spindle)

    rep.step("tools: spindle speeds within the spindle", rpm_range)

    def depth():
        stroke = max([s for _, s, _, _ in dtd] or [0.0])
        total = values["board_mm"] + values["base_mm"] + stroke
        need(total <= values["z_depth"], "board + base sheet + longest stroke is %.1f mm, above the %.1f mm of Z travel" % (total, values["z_depth"]))
        return "%.1f mm needed of %.1f mm Z travel" % (total, values["z_depth"])

    rep.step("DCF: Z depth covers board, base sheet and stroke", depth)

    def corrections():
        for k in ("corr_x", "corr_y", "corr_z"):
            need(0.8 <= values[k] <= 1.2, "%s = %g is more than 20 percent from 1.0: a calibration of that size is a wrong value" % (k, values[k]))
        return "X %g, Y %g, Z %g" % (values["corr_x"], values["corr_y"], values["corr_z"])

    rep.step("DCF: calibration factors near 1", corrections)

    def connection():
        c = int(values["connection"])
        if c == 4:
            raise Skip("the DCF says NO MACHINE (simulator): choose the machine file's connection before the machine day")
        need(c in (0, 1, 2, 3), "connection %d is not a serial port" % c)
        return "serial port COM%d, speed code %d" % (c + 1, int(values["baud"]))

    rep.step("DCF: connection", connection)


def write_proposal(dcf_text, where, new_values, path):
    """A copy of the DCF with some numbers replaced, written to a NEW file (it refuses to overwrite anything)."""
    lines = dcf_text.splitlines()
    for name, text in new_values.items():
        raw = lines[where[name]]
        m = re.match(r"^(\s*)(\S+)(\s+\*.*)$", raw)
        lines[where[name]] = (m.group(1) + text + m.group(3)) if m else text
    with open(path, "x", encoding="utf-8", newline="\n") as f:     # "x": fails if the file exists
        f.write("\n".join(lines) + "\n")
    return path


def ask_float(ask, prompt):
    while True:
        try:
            raw = ask(prompt + " ")
        except EOFError:
            raise Fail("no answer (the measuring needs a person)")
        raw = raw.strip().replace(",", ".")
        if raw.lower() in ("q", "quit"):
            raise Skip("measuring cancelled")
        try:
            v = float(raw)
        except ValueError:
            print("A number, please (q to quit).")
            continue
        if v > 0:
            return v
        print("A positive number, please.")


def measure(rep, values, where, dcf_text, out_dir, ask=input, say=print):
    """The stopwatch guide. Nothing here sends anything to the machine: the person moves it from the program and reads the clock."""
    proposal = {}
    say("MEASURING (nothing is sent by this tool; you move the machine from DRILLPRO and time it)")

    def accel_axis(axis, label, fast, start, shipped_steps, shipped_accel, field):
        say("\n%s ACCELERATION. Make a move of D mm along %s at the fastest speed (%.0f mm/min), from rest to rest, with nothing on the table." % (label, label, fast))
        say("Start the stopwatch when the head starts and stop it when it stops. Repeat 3 times and use the average.")
        d = ask_float(ask, "Distance D in mm (about %.0f works):" % (fast / 60.0 * 4))
        t = ask_float(ask, "Average time in seconds:")
        a, value = accel_value_from_measure(d, t, fast, start, shipped_steps, shipped_accel)
        proposal[field] = "%d" % max(1, round(value))
        return "measured %.0f mm/s2 -> ACCELERATION %.2f (proposed %s, the file has %g)" % (a, value, proposal[field], values[field])

    rep.step("measure: XY acceleration", lambda: accel_axis("X", "X", values["xy_fast"], values["xy_start"], SHIPPED_RAMP_XY, SHIPPED_ACCEL_XY, "accel_xy"))
    rep.step("measure: Z acceleration", lambda: accel_axis("Z", "Z", values["z_fast"], values["z_start"], SHIPPED_RAMP_Z, SHIPPED_ACCEL_Z, "accel_z"))

    def spindle():
        say("\nSPINDLE START TIME. With the spindle free and the guard closed, start it by hand at a speed R (for example 12000 rpm).")
        say("Start the stopwatch at the start command and stop it when the speed display (or your ear) says it is steady. Repeat 3 times.")
        r = ask_float(ask, "Speed R in rpm:")
        t = ask_float(ask, "Average seconds to reach it:")
        value = spindle_time_from_measure(r, t)
        proposal["spindle_start"] = "%.2f" % value
        return "%.1f s to %d rpm -> SPINDLE START TIME %.2f s per 10000 rpm (the file has %g)" % (t, r, value, values["spindle_start"])

    rep.step("measure: spindle start time", spindle)
    if not proposal:
        rep.add(SKIP, "measure: proposal file", "nothing was measured")
        return None
    stamp = time.strftime("%Y%m%d_%H%M%S")
    path = os.path.join(out_dir, "DRILLPRO_proposal_%s.DCF" % stamp)
    try:
        write_proposal(dcf_text, where, proposal, path)
    except FileExistsError:
        rep.add(FAIL, "measure: proposal file", "%s exists; nothing was written" % path)
        return None
    rep.add(OK, "measure: proposal file", "%s (a new file; your DCF is untouched; try it, then copy it over by hand)" % path)
    return path


def main(argv=None, ask=input):
    here = os.path.dirname(os.path.abspath(__file__))
    root = os.path.dirname(here)
    p = argparse.ArgumentParser(description="Checks the machine file DRILLPRO.DCF against the model; --measure guides a stopwatch measurement.")
    p.add_argument("--dcf", default=os.path.join(root, "config", "DRILLPRO.DCF"), help="the machine file (default: config/DRILLPRO.DCF)")
    p.add_argument("--dtd", default="", help="the tool table (default: DRILLPRO.DTD beside the DCF)")
    p.add_argument("--machine-cfg", default="", help="machine.cfg, for the speed limits and the spindle wait")
    p.add_argument("--spindle", choices=sorted(SPINDLES), default="original", help="which spindle the machine has (default original: 63000 rpm; vfd: 24000)")
    p.add_argument("--measure", action="store_true", help="guided stopwatch measurement; writes a proposed DCF as a new file")
    p.add_argument("--out-dir", default=".", help="where the report and the proposal go (default: the current folder)")
    args = p.parse_args(argv)
    rep = Report("drillpro", args.dcf)
    rep.prefix = "machine_selftest"
    try:
        with open(args.dcf, encoding="utf-8", errors="replace") as f:
            dcf_text = f.read()
        values, where = parse_dcf(dcf_text)
    except (OSError, ValueError) as e:
        rep.add(FAIL, "DCF: read", str(e))
        rep.write(args.out_dir)
        return 1
    rep.add(OK, "DCF: read", "%d values, serial %s, version %s" % (len(values), values["serial"], values["version"]))
    dtd_path = args.dtd or os.path.join(os.path.dirname(os.path.abspath(args.dcf)), "DRILLPRO.DTD")
    dtd = []
    if os.path.exists(dtd_path):
        with open(dtd_path, encoding="utf-8", errors="replace") as f:
            dtd = parse_dtd(f.read())
    mcfg = {}
    if args.machine_cfg:
        try:
            with open(args.machine_cfg, encoding="utf-8") as f:
                mcfg = parse_machine_cfg(f.read())
        except OSError as e:
            rep.add(FAIL, "machine.cfg: read", str(e))
    check_dcf(rep, values, dtd, mcfg, args.spindle)
    if args.measure:
        measure(rep, values, where, dcf_text, args.out_dir, ask)
    else:
        rep.add(SKIP, "measure", "not requested (add --measure)")
    print("report: " + rep.write(args.out_dir))
    return 1 if rep.counts[FAIL] else 0


if __name__ == "__main__":
    sys.exit(main())
