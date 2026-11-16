"""Real tiny SQLite working copies retire only after final successful cleanup."""

import json
from pathlib import Path
import sqlite3
import tempfile
import unittest

from m_runner import retire_working_copies
from test_m_calibration import LIMITS


class RetirementTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.root = self.base / "selected"
        self.attempt = self.root / "results" / "M-L-1-alternating" / "attempt-0001"
        stores = self.attempt / "stores"
        stores.mkdir(parents=True)
        self.files = [stores / "a.sqlite", stores / "b.sqlite"]
        for path in self.files:
            with sqlite3.connect(path) as database:
                database.execute("CREATE TABLE literal (body TEXT)")
                database.execute("INSERT INTO literal VALUES ('preserved immutable input elsewhere')")
        self.immutable = self.base / "immutable.sqlite"
        self.immutable.write_bytes(self.files[0].read_bytes())
        self.before = self.immutable.read_bytes()
        self.outcome = {"id": "M-L-1-alternating", "cell": {"family": "M-L", "copies": 2},
            "attempt": str(self.attempt), "complete": True, "status": "complete",
            "exit": 0, "cleanup_uncertain": False, "live_owned_pids": []}
        (self.root / "results/campaign-state.json").write_text(json.dumps([self.outcome]))
        (self.attempt / "attempt.json").write_text(json.dumps(self.outcome))
        self.raw_result = self.attempt / "result.json"
        self.raw_result.write_text(json.dumps({"complete": True, "scientific_results": [1, 2, 3]}))
        (self.attempt / "prepared.json").write_text(json.dumps({"copies": [
            {"destination": str(path), "bytes": path.stat().st_size, "source": str(self.immutable)}
            for path in self.files]}))
        self.guardian = {"exit": 0, "child_exit": 0, "cleanup_uncertain": False,
            "boundary_populated_at_controller_exit": False,
            "boundary": {"status": "quiescent", "populated_after": False, "removed_exact_empty_boundary": True}}
        guard = self.root / "guardians" / "original"
        guard.mkdir(parents=True)
        self.guard_path = guard / "guardian.json"
        self.guard_path.write_text(json.dumps(self.guardian))
        (self.root / "manifest.json").write_text(json.dumps({"scope": "registered-M-selected-cell",
            "allocation": str(self.base), "cell_allowance": LIMITS,
            "profile": {"total_shared_ns": 14400 * 10**9, "total_shared_retained_bytes": 16 * 1024**3,
                        "reserved_other_work_ns": 0, "reserved_other_work_bytes": 0}}))
        self.ledger = self.base / "g-budget.json"
        self.ledger.write_text(json.dumps({"schema": "slice30-gd-budget/v1", "used_ns": 1000,
            "inflight": None, "m_guardian": None, "d_guardian": None, "closed_guardian": None,
            "selected_cell_usage": {"M-L-1-alternating": {"limits": LIMITS, "wall_ns": 500,
                                                       "new_retained_bytes": 12345}}}))

    def test_successful_two_store_copies_retire_but_raw_input_and_peak_charges_survive(self):
        raw = self.raw_result.read_bytes()
        retired = retire_working_copies(self.root)
        self.assertEqual(set(retired), {str(path) for path in self.files}, "successful working copies were retained")
        self.assertTrue(all(not path.exists() for path in self.files))
        self.assertEqual(self.raw_result.read_bytes(), raw)
        self.assertEqual(self.immutable.read_bytes(), self.before)
        ledger = json.loads(self.ledger.read_text())
        self.assertEqual(ledger["selected_cell_usage"]["M-L-1-alternating"]["new_retained_bytes"], 12345)
        self.assertGreater(ledger["used_ns"], 1000)
        metadata = json.loads((self.attempt / "attempt.json").read_text())
        self.assertEqual(set(metadata["retired_working_copies"]), set(retired))

    def test_foreign_s50_custody_blocks_retirement_without_mutation(self):
        budget = json.loads(self.ledger.read_text())
        budget['s50_guardian'] = {'controller_pid': 123, 'started_monotonic_ns': 456}
        self.ledger.write_text(json.dumps(budget))
        before = self.ledger.read_bytes()
        stores = [path.read_bytes() for path in self.files]
        with self.assertRaisesRegex(ValueError, 'ownership'):
            retire_working_copies(self.root)
        self.assertEqual(self.ledger.read_bytes(), before)
        self.assertEqual([path.read_bytes() for path in self.files], stores)

    def test_uncertain_original_cleanup_keeps_all_working_copies(self):
        self.guardian["cleanup_uncertain"] = True
        self.guard_path.write_text(json.dumps(self.guardian))
        with self.assertRaisesRegex(ValueError, "guardian"):
            retire_working_copies(self.root)
        self.assertTrue(all(path.exists() for path in self.files))


if __name__ == "__main__":
    unittest.main()
