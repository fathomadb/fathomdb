"""Literal full-population evidence may satisfy only its identical registered row."""

import json
from pathlib import Path
import tempfile
import unittest

from m_calibration import CALIBRATION_ID, CELL_ID, selected_cell
from m_controller import load_plan
from m_runner import reused_rows, report


class ReuseTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.old = self.base / "old"
        self.old.mkdir()
        self.attempt = self.old / "results" / CALIBRATION_ID / "attempt-0001"
        wire = self.attempt / "wire"
        wire.mkdir(parents=True)
        self.guardian = {"exit": 0, "child_exit": 0, "cleanup_uncertain": False,
            "boundary_populated_at_controller_exit": False,
            "boundary": {"status": "quiescent", "populated_after": False, "removed_exact_empty_boundary": True}}
        guard = self.old / "guardians" / "original"
        guard.mkdir(parents=True)
        self.guard_path = guard / "guardian.json"
        self.guard_path.write_text(json.dumps(self.guardian))
        build = {"source_sha": "actual-old-native", "compiler_inputs": "same-compiler",
                 "binary": {"path": "owned-native", "sha256": "literal-native-identity"}}
        self.manifest = {"allocation": str(self.base), "compiler_inputs": "same-compiler",
            "closed_subject": {"main": "immutable-input"}, "package": ["literal-package"],
            "inputs": {"queries": "all500"}, "model": {"pin": "same-model"},
            "gpu": {"uuid": "same-gpu"}, "builds": {"ordinary": build}}
        original = dict(self.manifest, schema="slice30-m-calibration/v1",
            scope="bounded-natural-calibration", root=str(self.old), uuid="original-freeze",
            source_sha="actual-old-tools", qualifies_primary=False, plan=[selected_cell(CELL_ID)])
        (self.old / "manifest.json").write_text(json.dumps(original))
        outcome = {"id": CALIBRATION_ID, "cell": selected_cell(CELL_ID), "status": "complete",
            "complete": True, "exit": 0, "cleanup_uncertain": False, "live_owned_pids": [],
            "attempt": str(self.attempt), "scope": "bounded-natural-calibration", "qualifies_primary": False}
        (self.old / "results/campaign-state.json").write_text(json.dumps([outcome]))
        (self.attempt / "attempt.json").write_text(json.dumps(outcome))
        result = {"complete": True, "cell": selected_cell(CELL_ID), "freeze_uuid": "original-freeze",
            "selected_build": build, "pid": 7, "scope": "bounded-natural-calibration", "qualifies_primary": False,
            "gpu": {"complete": True, "failures": [], "witnesses": 1, "samples": 2}, "phases": {}}
        frames = []
        for phase, count in (("first", 1), ("warmup", 500), ("timed", 1000)):
            ids = list(range(len(frames) + 3, len(frames) + 3 + count))
            result["phases"][phase] = {"submitted": ids, "completed": ids, "summary": {
                "submitted": count, "completed": count, "success": count, "unresolved": 0, "errors": {}}}
            frames.extend({"phase": phase, "query_index": i % 500, "rpc_id": rpc,
                           "pid": 7, "status": "completed"} for i, rpc in enumerate(ids))
        for name in ("submissions.jsonl", "invocations.jsonl"):
            (wire / name).write_text(''.join(json.dumps(frame) + '\n' for frame in frames))
        self.result_path = self.attempt / "result.json"
        self.result_path.write_text(json.dumps(result))
        self.manifest["reuse_from"] = [{"root": str(self.old), "cell_id": CELL_ID}]

    def test_full_calibration_satisfies_one_primary_by_reference_without_retagging(self):
        before = (self.old / "manifest.json").read_bytes(), self.result_path.read_bytes()
        rows = reused_rows(self.manifest)
        self.assertIn(CELL_ID, rows, "complete identical calibration was not referenced")
        self.assertEqual(set(rows), {CELL_ID})
        self.assertTrue(rows[CELL_ID]["complete"])
        self.assertEqual(rows[CELL_ID]["satisfied_by"]["accounting_id"], CALIBRATION_ID)
        self.assertEqual(rows[CELL_ID]["satisfied_by"]["source_sha"], "actual-old-tools")
        self.assertEqual(before, ((self.old / "manifest.json").read_bytes(), self.result_path.read_bytes()))
        target = self.base / "current"
        target.mkdir()
        manifest = dict(self.manifest, scope="registered-M-selected-cell", uuid="new-freeze",
                        source_sha="new-tools", plan=[load_plan()[3]])
        (target / "manifest.json").write_text(json.dumps(manifest))
        value = report(target)
        self.assertEqual(len(value["rows"]), 46)
        self.assertEqual(sum(row["complete"] for row in value["rows"]), 1)
        self.assertFalse(value["complete"])

    def test_failed_original_guardian_cannot_supply_primary_credit(self):
        self.guardian["exit"] = 124
        self.guard_path.write_text(json.dumps(self.guardian))
        with self.assertRaisesRegex(ValueError, "guardian"):
            reused_rows(self.manifest)

    def test_partial_timed_population_cannot_supply_primary_credit(self):
        result = json.loads(self.result_path.read_text())
        result["phases"]["timed"]["summary"]["completed"] = 999
        self.result_path.write_text(json.dumps(result))
        with self.assertRaisesRegex(ValueError, "population"):
            reused_rows(self.manifest)


if __name__ == "__main__":
    unittest.main()
