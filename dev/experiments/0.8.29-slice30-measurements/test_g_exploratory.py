"""Literal exploratory admission and real child controls under unchanged hard caps."""

import json
from pathlib import Path
import sys
import tempfile
import time
import unittest

from g_campaign import make_g_plan, run_g_plan

SPEC = json.loads(Path(__file__).with_name("g-cells.json").read_text())
STAGE = "first-100k-seed-exploratory"
LINEAR = 653317972470
CONSERVATIVE = 1306635944940


def controlled_seed(root, charged=2901306, fail=False):
    """Build scheduling-only records and harmless child, never Default/GPU evidence."""
    references = {}
    for cell in make_g_plan(SPEC):
        if cell["id"] not in {"g0-smoke", "g1-n10000", "g-r-n10000-r0", "g-r-n10000-r1", "g-o-branches"}:
            continue
        attempt = root / "original" / cell["id"]
        attempt.mkdir(parents=True)
        references[cell["id"]] = {"id": cell["id"], "cell": cell, "status": "reused_complete",
            "elapsed_ns": 65326146421, "attempt": str(attempt),
            "origin": {"root": str(root / "original"), "source_sha": "control-source", "campaign_uuid": "control-uuid"}}
    budget = {"schema": "slice30-gd-budget/v1", "used_ns": 357099039661, "inflight": None,
              "cell_used_ns": {"g1-n10000": 65331797247, "g1-n100000": charged}}
    (root / "g-budget.json").write_text(json.dumps(budget))
    calls = root / "calls.jsonl"
    command = [sys.executable, "-c", "\n".join([
        "import json,pathlib,sys",
        "cell=json.loads(pathlib.Path(sys.argv[1]).read_text()); out=pathlib.Path(sys.argv[2])",
        "assert cell['cell_seconds']==900 and cell['campaign_seconds']==14400",
        f"with pathlib.Path({str(calls)!r}).open('a') as stream: stream.write(cell['id']+'\\n')",
        f"(out/'result.json').write_text(json.dumps({{'complete':{not fail!r},'runner_only':True}}))",
        f"raise SystemExit({int(fail)})",
    ])]
    return references, command, calls


def consume_preparation(_cell, _root):
    """Use real bounded host preparation time instead of changing a clock or cap."""
    started = time.monotonic_ns()
    while time.monotonic_ns() - started < 100_000_000:
        pass


class GExploratoryAdmissionTests(unittest.TestCase):
    def test_named_seed_runs_once_with_failed_conservative_forecast_and_same_caps(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            refs, command, calls = controlled_seed(root)
            outcomes = run_g_plan(SPEC, root / "stage", command, allocation_root=root,
                                  reused=refs, stage=STAGE)
            self.assertTrue(calls.exists(), "named bounded exploratory seed was not dispatched")
            self.assertEqual(calls.read_text().splitlines(), ["g1-n100000"])
            row = next(row for row in outcomes if row["id"] == "g1-n100000")
            self.assertEqual(row["status"], "complete")
            self.assertEqual(row["prediction"]["seed_ns"], CONSERVATIVE)
            self.assertEqual(row["prediction"]["linear_seed_ns"], LINEAR)
            self.assertFalse(row["prediction"]["conservative_time_feasible"])
            self.assertEqual(row["prediction"]["total_ns"], LINEAR)
            self.assertLess(row["remaining_ns_at_dispatch"], 900 * 10**9 - 2901306)
            self.assertGreater(row["remaining_ns_at_dispatch"], LINEAR)
            self.assertEqual(len(outcomes), 84)
            self.assertEqual(sum(row["status"] == "not_run_stage" for row in outcomes), 78)
            budget = json.loads((root / "g-budget.json").read_text())
            self.assertTrue(budget["exploratory_seed100k"]["consumed"])
            self.assertGreater(budget["cell_used_ns"]["g1-n100000"], 2901306)
            self.assertGreater(budget["used_ns"], 357099039661)
            self.assertIsNone(budget["inflight"])

    def test_failed_exploratory_child_cannot_be_retried_from_a_new_freeze_root(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            refs, command, calls = controlled_seed(root, fail=True)
            first = run_g_plan(SPEC, root / "stage1", command, allocation_root=root, reused=refs, stage=STAGE)
            self.assertTrue(calls.exists(), "first exploratory child never ran")
            self.assertEqual(next(row for row in first if row["id"] == "g1-n100000")["status"], "failed")
            second = run_g_plan(SPEC, root / "stage2", command, allocation_root=root, reused=refs, stage=STAGE)
            self.assertEqual(calls.read_text().splitlines(), ["g1-n100000"], "new root renewed the one-shot admission")
            row = next(row for row in second if row["id"] == "g1-n100000")
            self.assertEqual((row["status"], row["reason"]), ("not_run_cap", "exploratory_admission_consumed"))

    def test_prior_charges_refuse_before_dispatch(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            refs, command, calls = controlled_seed(root, charged=400 * 10**9)
            outcomes = run_g_plan(SPEC, root / "stage", command, allocation_root=root, reused=refs, stage=STAGE)
            self.assertFalse(calls.exists(), "linear estimate exceeded actual charged cell allowance")
            self.assertEqual(next(row for row in outcomes if row["id"] == "g1-n100000")["reason"], "cap_feasibility")

    def test_assessment_and_preparation_each_consume_prospective_headroom(self):
        for phase in ("assessment", "preparation"):
            with self.subTest(phase=phase), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                refs, command, calls = controlled_seed(root, charged=900 * 10**9 - LINEAR - 20_000_000)
                def assess(cell, _previous, _remaining, _retained):
                    if phase == "assessment":
                        consume_preparation(cell, root)
                outcomes = run_g_plan(SPEC, root / "stage", command, allocation_root=root,
                                      reused=refs, stage=STAGE, assess=assess,
                                      prepare=consume_preparation if phase == "preparation" else None)
                self.assertFalse(calls.exists(), f"{phase} time was omitted before exploratory dispatch")
                self.assertEqual(next(row for row in outcomes if row["id"] == "g1-n100000")["reason"], "cap_feasibility")
