"""Unreadable closure and in-progress admission cannot create fictitious capacity."""

import json
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch

from g_campaign import run_g_plan
from g_serial_storage import capture_closure
from test_g_serial_controller import child, prepared, GiB


class GSerialReviewFixTests(unittest.TestCase):
    def test_unreadable_subject_is_unavailable_but_confirmed_absent_companions_are_zero(self):
        with tempfile.TemporaryDirectory() as temporary:
            attempt = Path(temporary)
            subject = attempt/"subject"
            subject.mkdir()
            (subject/"store.sqlite").write_bytes(b"x"*4096)
            outcome = {"attempt": str(attempt), "cleanup_uncertain": False,
                       "live_owned_pids": [], "pid": 1, "child_exit": 0}
            subject.chmod(0)
            try:
                with self.assertRaises(OSError, msg="unreadable subject was accepted as a known-zero closed store"):
                    capture_closure(outcome)
            finally:
                subject.chmod(0o700)
            witness = capture_closure(outcome)
            self.assertEqual(witness["bytes"], {"main": 4096, "wal": 0, "shm": 0,
                                               "output": 0, "total": 4096})

    def test_preparation_and_final_sampling_consume_shared_complete_table_margin_once(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            spec, reused = prepared()
            (root/"source").mkdir()
            spec["_serial_resource_roots"] = {"source": str(root/"source"), "task": str(root)}
            initial = 43200*10**9-600*10**9-72-150_000_000
            (root/"g-budget.json").write_text(json.dumps({"schema": "slice30-gd-budget/v1",
                "used_ns": initial, "inflight": None, "cell_used_ns": {}, "common_used_ns": {}}))
            samples = 0
            def sample(_source, _task):
                nonlocal samples
                samples += 1
                if samples == 2:
                    time.sleep(.12)
                return {"owned": 1, "task_free": 1000*GiB, "root_free": 1000*GiB,
                        "memory_available": 64*GiB}
            def command(_cell, _deadline):
                time.sleep(.08)
                return child(root)
            started = time.monotonic_ns()
            with patch("g_resource.resource_sample", side_effect=sample):
                rows = run_g_plan(spec, root/"results", command, allocation_root=root, reused=reused)
            elapsed = time.monotonic_ns()-started
            self.assertGreaterEqual(samples, 2, "control did not reach the final admission sample")
            self.assertFalse((root/"calls").exists(),
                             "committed budget hid preparation/sampling time and dispatched an unfunded complete table")
            self.assertEqual(rows[12]["error"], "serial_shared_time_forecast")
            budget = json.loads((root/"g-budget.json").read_text())
            self.assertGreaterEqual(budget["used_ns"]-initial, 200_000_000)
            self.assertLessEqual(budget["used_ns"]-initial, elapsed,
                                 "admission time was charged twice instead of only reserved prospectively")
            self.assertIsNone(budget["inflight"])
