"""Tests of tools/machine_selftest.py: the DCF checks against the model, the stopwatch maths and the proposal file."""
import contextlib
import hashlib
import io
import os
import tempfile
import unittest

import machine_selftest as ms

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
DCF = os.path.join(ROOT, "config", "DRILLPRO.DCF")


def read(path):
    with open(path, encoding="utf-8") as f:
        return f.read()


def run(argv, ask=lambda _: ""):
    out = io.StringIO()
    with tempfile.TemporaryDirectory() as d, contextlib.redirect_stdout(out):
        code = ms.main(argv + ["--out-dir", d], ask=ask)
        reports = [f for f in os.listdir(d) if f.startswith("machine_selftest_")]
        report = read(os.path.join(d, reports[0])) if reports else ""
        proposals = {f: read(os.path.join(d, f)) for f in os.listdir(d) if f.endswith(".DCF")}
    return code, out.getvalue(), report, proposals


def modified(**changes):
    """A copy of the shipped DCF with some data lines changed; returns the path."""
    text = read(DCF)
    values, where = ms.parse_dcf(text)
    lines = text.splitlines()
    for name, new in changes.items():
        raw = lines[where[name]]
        lines[where[name]] = new + raw[len(raw.split()[0]):]
    f = tempfile.NamedTemporaryFile("w", suffix=".DCF", delete=False, encoding="utf-8")
    f.write("\n".join(lines) + "\n")
    f.close()
    return f.name


class Maths(unittest.TestCase):
    def test_acceleration_round_trip(self):
        # a machine that really has ACCELERATION 10: the ramp is 40 steps; time a 100 mm move at full speed and recover 10
        fast, start = 4000.0, 1000.0
        a = ms.implied_accel_mm_s2(fast, start, 40)
        v = fast / 60.0
        seconds = 100.0 / v + v / a
        got_a, value = ms.accel_value_from_measure(100.0, seconds, fast, start, 40, 10)
        self.assertAlmostEqual(got_a, a, places=6)
        self.assertAlmostEqual(value, 10.0, places=6)

    def test_acceleration_rejects_impossible_times(self):
        with self.assertRaises(ValueError):
            ms.accel_value_from_measure(100.0, 1.0, 4000.0, 1000.0, 40, 10)   # faster than full speed all the way
        with self.assertRaises(ValueError):
            ms.accel_value_from_measure(-1.0, 1.0, 4000.0, 1000.0, 40, 10)

    def test_spindle_time(self):
        self.assertAlmostEqual(ms.spindle_time_from_measure(12000, 6.0), 5.0)
        with self.assertRaises(ValueError):
            ms.spindle_time_from_measure(0, 1)


class Parse(unittest.TestCase):
    def test_shipped_dcf(self):
        values, where = ms.parse_dcf(read(DCF))
        self.assertEqual(values["accel_xy"], 10.0)
        self.assertEqual(values["spindle_start"], 0.5)
        self.assertEqual(values["clip_x"], 495.0)
        self.assertEqual(len(values), 31)

    def test_short_or_broken_file(self):
        with self.assertRaises(ValueError):
            ms.parse_dcf("1 * a\n2 * b\n")
        with self.assertRaises(ValueError):
            ms.parse_dcf("a * x\nb * y\nc * z\nd * w\nnot-a-number * scale\n" + "1 * q\n" * 30)

    def test_tool_table(self):
        rows = ms.parse_dtd("comment\n04 0.5 1200 60000\n20 1.2 1500 40000\n")
        self.assertEqual(rows, [(4, 0.5, 1200, 60000), (20, 1.2, 1500, 40000)])


class Checks(unittest.TestCase):
    def test_shipped_dcf_passes(self):
        code, out, report, _ = run(["--dcf", DCF])
        self.assertEqual(code, 0, report)
        self.assertIn("OK       DCF: acceleration XY against the model", report)
        self.assertIn("OMITIDO  measure | not requested", report)

    def test_speed_above_the_line_fails(self):
        path = modified(xy_fast="9000")
        try:
            code, out, report, _ = run(["--dcf", path])
            self.assertEqual(code, 1)
            self.assertIn("FALLO    DCF: XY speeds against the line", report)
        finally:
            os.unlink(path)

    def test_zero_acceleration_and_zero_start_time_fail(self):
        path = modified(accel_xy="0", spindle_start="0")
        try:
            code, out, report, _ = run(["--dcf", path])
            self.assertEqual(code, 1)
            self.assertIn("FALLO    DCF: acceleration XY", report)
            self.assertIn("FALLO    DCF: spindle start time", report)
        finally:
            os.unlink(path)

    def test_vfd_spindle_cannot_reach_the_tool_table_speeds(self):
        code, out, report, _ = run(["--dcf", DCF, "--spindle", "vfd"])
        self.assertEqual(code, 1)
        self.assertIn("FALLO    tools: spindle speeds within the spindle", report)

    def test_machine_cfg_limit(self):
        with tempfile.NamedTemporaryFile("w", suffix=".cfg", delete=False) as f:
            f.write("xy_hz_limit = 2000\n")
        try:
            code, out, report, _ = run(["--dcf", DCF, "--machine-cfg", f.name])
            self.assertIn("FALLO    DCF: XY speeds against the line | XY fastest is 2625 steps/s, above the 2000", report)
        finally:
            os.unlink(f.name)

    def test_missing_dcf(self):
        code, out, report, _ = run(["--dcf", os.path.join(HERE, "nothing.DCF")])
        self.assertEqual(code, 1)
        self.assertIn("FALLO    DCF: read", report)


class Measure(unittest.TestCase):
    def script(self, *answers):
        it = iter(answers)

        def ask(_):
            try:
                return next(it)
            except StopIteration:
                raise EOFError
        return ask

    def test_measure_writes_a_new_file_and_leaves_the_dcf_alone(self):
        before = hashlib.sha256(read(DCF).encode()).hexdigest()
        a = ms.implied_accel_mm_s2(4000.0, 1000.0, 80)           # a machine whose ramp is 80 steps: ACCELERATION 5
        v = 4000.0 / 60
        t_xy = 200.0 / v + v / a
        az = ms.implied_accel_mm_s2(5000.0, 1000.0, 400)         # Z ramp 400 steps: ACCELERATION 10
        vz = 5000.0 / 60
        t_z = 100.0 / vz + vz / az
        code, out, report, proposals = run(["--dcf", DCF, "--measure"], self.script("200", repr(t_xy), "100", repr(t_z), "12000", "6"))
        self.assertEqual(code, 0, report)
        self.assertEqual(len(proposals), 1)
        text = list(proposals.values())[0]
        values, _ = ms.parse_dcf(text)
        self.assertEqual(values["accel_xy"], 5.0)
        self.assertEqual(values["accel_z"], 10.0)
        self.assertEqual(values["spindle_start"], 5.0)
        self.assertEqual(values["clip_x"], 495.0)                  # everything else is as it was
        self.assertIn("* SPINDLE START TIME", text)                # and the comments are kept
        self.assertEqual(hashlib.sha256(read(DCF).encode()).hexdigest(), before)

    def test_quit_writes_nothing(self):
        code, out, report, proposals = run(["--dcf", DCF, "--measure"], self.script("q"))
        self.assertEqual(proposals, {})
        self.assertIn("OMITIDO  measure: XY acceleration | measuring cancelled", report)
        self.assertIn("OMITIDO  measure: proposal file | nothing was measured", report)

    def test_impossible_time_is_a_failure_not_a_proposal(self):
        code, out, report, proposals = run(["--dcf", DCF, "--measure"], self.script("200", "0.5", "q"))
        self.assertEqual(code, 1)
        self.assertIn("FALLO    measure: XY acceleration", report)
        self.assertEqual(proposals, {})

    def test_proposal_never_overwrites(self):
        text = read(DCF)
        values, where = ms.parse_dcf(text)
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, "P.DCF")
            ms.write_proposal(text, where, {"accel_xy": "7"}, path)
            with self.assertRaises(FileExistsError):
                ms.write_proposal(text, where, {"accel_xy": "8"}, path)
            self.assertEqual(ms.parse_dcf(read(path))[0]["accel_xy"], 7.0)

    def test_no_answers_at_all(self):
        code, out, report, proposals = run(["--dcf", DCF, "--measure"], self.script())
        self.assertEqual(code, 1)
        self.assertEqual(proposals, {})


if __name__ == "__main__":
    unittest.main()
