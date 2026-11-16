"""Explicit failed predecessor history and collision-safe original invocation logs."""

import copy
import json
from pathlib import Path
import tempfile
import unittest

import m_runner
from m_controller import load_plan
from test_m_calibration import LIMITS


class RetryTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.old = self.base / "failed-original"
        self.new = self.base / "explicit-retry"
        self.cell = load_plan()[6]
        self.manifest = {"schema": "slice30-m-freeze/v1", "scope": "registered-M-selected-cell",
            "allocation": str(self.base), "root": str(self.new), "uuid": "retry",
            "source_sha": "new-python-only", "compiler_inputs": "same-native",
            "closed_subject": {"main": "original-closed"}, "package": ["same-package"],
            "inputs": {"queries": "same-500"}, "model": {"pin": "same-model"},
            "gpu": {"uuid": "same-selected"}, "builds": {"ordinary": {"source_sha": "original-native"}},
            "plan": [self.cell], "cell_allowance": LIMITS, "retry_from": str(self.old)}
        self.attempt = self.old / "results" / self.cell["id"] / "attempt-0001"
        self.attempt.mkdir(parents=True)
        original = dict(self.manifest, root=str(self.old), uuid="original", source_sha="original-python")
        original.pop("retry_from")
        (self.old / "manifest.json").write_text(json.dumps(original))
        outcome = {"id": self.cell["id"], "cell": self.cell, "status": "failed", "complete": False,
            "exit": 1, "cleanup_uncertain": False, "live_owned_pids": [], "attempt": str(self.attempt)}
        (self.old / "results/campaign-state.json").write_text(json.dumps([outcome]))
        (self.attempt / "attempt.json").write_text(json.dumps(outcome))
        (self.attempt / "result.json").write_text('{"complete":false,"gpu":{"failures":["foreign"]}}')
        guard = self.old / "guardians" / "original"
        guard.mkdir(parents=True)
        (guard / "guardian.json").write_text(json.dumps({"exit": 1, "child_exit": 1,
            "cleanup_uncertain": False, "boundary_populated_at_controller_exit": False,
            "boundary": {"status": "quiescent", "populated_after": False,
                         "removed_exact_empty_boundary": True}}))
        self.budget = {"used_ns": 26599341664363, "selected_cell_usage": {self.cell["id"]: {
            "limits": LIMITS, "wall_ns": 272193177669, "new_retained_bytes": 765323980,
            "roots": [str(self.old), str(self.new)]}}}
        (self.base / "g-budget.json").write_text(json.dumps(self.budget))
        self.new.mkdir()
        (self.new / "manifest.json").write_text(json.dumps(self.manifest))

    def test_failed_predecessor_stays_in_all46_report_without_credit_or_retagging(self):
        before = {p: p.read_bytes() for p in self.old.rglob('*') if p.is_file()}
        budget_before = (self.base / "g-budget.json").read_bytes()
        value = m_runner.report(self.new)
        row = next(row for row in value["rows"] if row["id"] == self.cell["id"])
        self.assertEqual(len(value["rows"]), 46)
        self.assertFalse(row["complete"])
        self.assertEqual(len(row["attempts"]), 1)
        self.assertEqual(row["attempts"][0]["attempt"], str(self.attempt))
        self.assertFalse(row["attempts"][0]["complete"])
        self.assertEqual(before, {p: p.read_bytes() for p in self.old.rglob('*') if p.is_file()})
        self.assertEqual((self.base / "g-budget.json").read_bytes(), budget_before)

    def test_uncertain_predecessor_blocks_retry_reporting(self):
        path = self.old / "guardians/original/guardian.json"
        original = json.loads(path.read_text())
        original["cleanup_uncertain"] = True
        path.write_text(json.dumps(original))
        with self.assertRaisesRegex(ValueError, "predecessor"):
            m_runner.report(self.new)

    def test_changed_native_or_allowance_cannot_adopt_failed_history(self):
        for field, value in (("compiler_inputs", "different-native"),
                             ("cell_allowance", dict(LIMITS, phase_ns=899000000000))):
            with self.subTest(field=field):
                changed = copy.deepcopy(self.manifest)
                changed[field] = value
                (self.new / "manifest.json").write_text(json.dumps(changed))
                with self.assertRaisesRegex(ValueError, "predecessor"):
                    m_runner.report(self.new)

    def test_a_new_root_never_overwrites_the_old_invocation_log(self):
        opener = getattr(m_runner, "open_invocation_log", None)
        self.assertTrue(callable(opener), "existing supervisor lacks a collision-safe log route")
        first = self.base / "original-cell"
        retry = self.base / "retry-cell"
        with opener(first, "service") as output:
            output.write("original raw\n")
        with self.assertRaises(FileExistsError):
            opener(first, "service")
        with opener(retry, "service") as output:
            output.write("retry raw\n")
        self.assertEqual((self.base / "original-cell-service.log").read_text(), "original raw\n")
        self.assertEqual((self.base / "retry-cell-service.log").read_text(), "retry raw\n")


if __name__ == "__main__":
    unittest.main()
