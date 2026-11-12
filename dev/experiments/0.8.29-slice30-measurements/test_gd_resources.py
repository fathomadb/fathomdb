"""Literal resource-budget and real small-child controls for G/D tooling."""

import unittest
from pathlib import Path
import sys
import tempfile

from gd_resources import aggregate_roots, reservation_ok, run_guarded, setup_growth


class AggregateTests(unittest.TestCase):
    def test_each_root_uses_its_larger_size_before_summing(self):
        roots = [{"allocated": 40, "apparent": 50},
                 {"allocated": 10, "apparent": 8}]
        self.assertEqual(aggregate_roots(roots), 60)


class SetupGrowthTests(unittest.TestCase):
    def test_raw_subtraction_precedes_each_root_maximum(self):
        roots = [{"path":"p", "allocated":40, "apparent":50},
                 {"path":"task", "allocated":30, "apparent":38}]
        raw = [{"allocated":10, "apparent":20}]
        self.assertEqual(setup_growth(roots, "task", raw, 60), 10)


class ReservationTests(unittest.TestCase):
    def test_raw_and_unused_setup_reservations_cannot_be_consumed(self):
        gib = 1024**3
        self.assertFalse(reservation_ok(73*gib, 0, 0, 0))
        self.assertTrue(reservation_ok(64*gib, 0, 0, 0))
        self.assertTrue(reservation_ok(122*gib, 50*gib, 4*gib, 4*gib))
        self.assertFalse(reservation_ok(150*gib, 97*gib, 0, 0))
        self.assertFalse(reservation_ok(150*gib, 80*gib, 17*gib, 0))
        self.assertFalse(reservation_ok(200*gib, 96*gib, 16*gib, 16*gib))


class GuardTests(unittest.TestCase):
    def test_stops_only_small_owned_child_and_preserves_its_actual_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            payload, finished = root / "payload", root / "finished"
            code = ("import pathlib,time; "
                    f"pathlib.Path({str(payload)!r}).write_bytes(b'x'*131072); "
                    "print('payload-ready',flush=True); time.sleep(0.5); "
                    f"pathlib.Path({str(finished)!r}).write_text('finished')")
            result = run_guarded([sys.executable,"-c",code], root, roots=[str(root)],
                                 baseline=0, max_owned=65536, interval=0.01)
            self.assertEqual(result["exit"], 124)
            self.assertEqual(payload.stat().st_size, 131072)
            self.assertFalse(finished.exists())
            self.assertIn("payload-ready", (root / "logs/phase.log").read_text())
            self.assertTrue((root / "receipts/phase.json").exists())


if __name__ == "__main__":
    unittest.main()
