"""Actual du launch controls for the optional shared campaign probe deadline."""

import os
from pathlib import Path
import shutil
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from hypothesis import Phase, given, settings, strategies as st

from gd_resources import measure_root, resource_sample


def expired_probe(size):
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        tree = root / "tree"
        tree.mkdir()
        (tree / "payload").write_bytes(b"x" * size)
        tools = root / "tools"
        tools.mkdir()
        marker = root / "calls"
        real_du = shutil.which("du")
        assert real_du
        wrapper = tools / "du"
        wrapper.write_text(f"#!{sys.executable}\nimport os,sys\nfrom pathlib import Path\np=Path({str(marker)!r})\nwith p.open('a') as out: out.write('called\\n')\nos.execv({real_du!r},[{real_du!r},*sys.argv[1:]])\n")
        wrapper.chmod(0o755)
        with patch.dict(os.environ, PATH=str(tools)+os.pathsep+os.environ["PATH"]):
            healthy = measure_root(tree)
            assert healthy["allocated"] >= 0 and healthy["apparent"] >= size
            assert marker.read_text().splitlines() == ["called", "called"]
            marker.write_text("")
            refused = False
            try:
                measure_root(tree, deadline=time.monotonic()-1)
            except RuntimeError:
                refused = True
            actual = (refused, len(marker.read_text().splitlines()))
            print(f"real du expired probe refused/calls={actual}", flush=True)
            return actual


class ProbePrelaunchTests(unittest.TestCase):
    def test_expired_shared_deadline_launches_no_real_du(self):
        self.assertEqual(expired_probe(17), (True, 0))

    @settings(max_examples=4, deadline=None, database=None, phases=[Phase.generate])
    @given(st.integers(min_value=0, max_value=4096))
    def test_expired_probe_refuses_before_launch_for_bounded_real_trees(self, size):
        self.assertEqual(expired_probe(size), (True, 0))


def live_probe(delay):
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        tree = root / "tree"
        tree.mkdir()
        (tree / "payload").write_bytes(b"real filesystem capacity")
        tools = root / "tools"
        tools.mkdir()
        pid_path, late = root / "pid", root / "late"
        real_du = shutil.which("du")
        assert real_du
        wrapper = tools / "du"
        wrapper.write_text(f"#!{sys.executable}\nimport os,sys,time\nfrom pathlib import Path\nPath({str(pid_path)!r}).write_text(str(os.getpid()))\nprint('probe-before-wait',flush=True)\nprint('probe-diagnostic',file=sys.stderr,flush=True)\ndelay=float(os.environ.get('S30_PROBE_DELAY','0'))\ntime.sleep(delay)\nif delay: Path({str(late)!r}).write_text('finished')\nos.execv({real_du!r},[{real_du!r},*sys.argv[1:]])\n")
        wrapper.chmod(0o755)
        with patch.dict(os.environ, PATH=str(tools)+os.pathsep+os.environ["PATH"]):
            healthy = measure_root(tree)
            assert healthy["allocated"] >= 0 and healthy["apparent"] >= 24
            with patch.dict(os.environ, S30_PROBE_DELAY=str(delay)):
                started = time.monotonic()
                error = ""
                try:
                    measure_root(tree, deadline=started+0.3)
                except RuntimeError as failure:
                    error = str(failure)
                elapsed = time.monotonic()-started
            assert pid_path.exists(), "owned probe startup fixture failed NOT_RED"
            pid = int(pid_path.read_text())
            gone = False
            try:
                os.kill(pid, 0)
            except ProcessLookupError:
                gone = True
            actual = (bool(error), elapsed < 0.7, not late.exists(), gone,
                      "probe-before-wait" in error and "probe-diagnostic" in error)
            print(f"owned in-flight probe elapsed={elapsed:.3f}s disposition={actual}", flush=True)
            return actual


class ProbeInFlightTests(unittest.TestCase):
    def test_in_flight_probe_stops_within_original_deadline_and_keeps_diagnostics(self):
        self.assertEqual(live_probe(1.0), (True, True, True, True, True))

    @settings(max_examples=3, deadline=None, database=None, phases=[Phase.generate])
    @given(st.sampled_from([0.9, 1.0, 1.1]))
    def test_in_flight_probe_cannot_accept_or_finish_late(self, delay):
        self.assertEqual(live_probe(delay), (True, True, True, True, True))


def sample_probe(root_count, with_raw):
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        roots = []
        for index in range(root_count):
            tree = root / f"tree-{index}"
            tree.mkdir()
            (tree / "payload").write_bytes(b"capacity")
            roots.append(str(tree))
        task = root / "task"
        task.mkdir()
        roots.append(str(task))
        if with_raw:
            for family in ["g", "d"]:
                (task / family).mkdir()
                (task / family / "payload").write_bytes(b"raw")
        tools = root / "tools"
        tools.mkdir()
        calls = root / "calls"
        real_du = shutil.which("du")
        assert real_du
        wrapper = tools / "du"
        wrapper.write_text(f"#!{sys.executable}\nimport os,sys,time\nfrom pathlib import Path\nwith Path({str(calls)!r}).open('a') as output: output.write(str(os.getpid())+'\\n')\nprint('shared-sample-probe',file=sys.stderr,flush=True)\ntime.sleep(float(os.environ.get('S30_PROBE_DELAY','0')))\nos.execv({real_du!r},[{real_du!r},*sys.argv[1:]])\n")
        wrapper.chmod(0o755)
        with patch.dict(os.environ, PATH=str(tools)+os.pathsep+os.environ["PATH"]):
            healthy = resource_sample(roots, task, 0)
            assert healthy["owned"] > 0
            assert (healthy["g_raw"] > 0) == with_raw
            calls.write_text("")
            error = ""
            with patch.dict(os.environ, S30_PROBE_DELAY="0.08"):
                started = time.monotonic()
                try:
                    resource_sample(roots, task, 0, deadline=started+0.3)
                except RuntimeError as failure:
                    error = str(failure)
                elapsed = time.monotonic()-started
            pids = [int(value) for value in calls.read_text().splitlines()]
            gone = True
            for pid in pids:
                try:
                    os.kill(pid, 0)
                    gone = False
                except ProcessLookupError:
                    pass
            expected_calls = 2*(len(roots)+(2 if with_raw else 0))
            actual = (bool(error), elapsed < 0.5, 0 < len(pids) < expected_calls, gone)
            print(f"shared sample roots={len(roots)} raw={with_raw} elapsed={elapsed:.3f} calls={len(pids)}/{expected_calls} disposition={actual}", flush=True)
            return actual


class ProbeSampleDeadlineTests(unittest.TestCase):
    def test_all_roots_and_raw_observations_share_one_deadline(self):
        self.assertEqual(sample_probe(2, True), (True, True, True, True))

    @settings(max_examples=3, deadline=None, database=None, phases=[Phase.generate])
    @given(st.integers(min_value=2, max_value=3), st.booleans())
    def test_sample_budget_is_not_renewed_per_root_or_metric(self, roots, raw):
        self.assertEqual(sample_probe(roots, raw), (True, True, True, True))


if __name__ == "__main__":
    unittest.main()
