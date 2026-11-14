"""Existing normal-controller preservation with real harmless children, never G measurements."""

import json
from pathlib import Path
import sys
import tempfile
import unittest

from g_campaign import run_g_plan
from g_freeze import freeze
from test_g_normal_resume import accepted_protocol_chain, CHARGES, dump, SIX, SPEC


class GNormalSchedulePreservationTests(unittest.TestCase):
    def test_normal_order_resume_forecasts_and_original_charges_are_preserved(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with accepted_protocol_chain(root) as (options, expected):
                manifest = freeze(options)
                token = {"consumed": True, "cell": "g1-n100000", "root": str(root / "seed/results")}
                usage = dict(CHARGES, **{"g-r-n100000-r0": 2997368, "g-r-n100000-r1": 3102917})
                dump(root / "g-budget.json", {"schema": "slice30-gd-budget/v1", "used_ns": 1030482596607,
                    "inflight": None, "cell_used_ns": usage, "exploratory_seed100k": token})
                calls = root / "calls.jsonl"
                command = [sys.executable, "-c", "\n".join([
                    "import json,pathlib,sys",
                    "cell=json.loads(pathlib.Path(sys.argv[1]).read_text()); out=pathlib.Path(sys.argv[2])",
                    f"with pathlib.Path({str(calls)!r}).open('a') as stream: stream.write(json.dumps(cell)+'\\n')",
                    "(out/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
                ])]
                result_root = root / "normal/results"
                first = run_g_plan(SPEC, result_root, command, allocation_root=root, reused=manifest["reused"])
                actual = [json.loads(line) for line in calls.read_text().splitlines()]
                self.assertEqual([row["id"] for row in actual], ["g-r-n100000-r0",
                    "g-o-natural-r0-off", "g-o-natural-r0-on", "g-o-natural-r1-on", "g-o-natural-r1-off"])
                self.assertTrue(SIX.isdisjoint(row["id"] for row in actual))
                self.assertTrue(all(row["n"] == 100000 and row["fresh_seed"] is False for row in actual))
                self.assertEqual((actual[0]["adapters"], actual[0]["routes"], actual[0]["warmup"], actual[0]["samples"]),
                                 (["engine", "sdk"], ["text", "hybrid", "embed"], 500, 1000))
                self.assertTrue(all(row["samples"] == 50 and row["warmup"] == 0
                                    and row["record_first_call"] is False for row in actual[1:]))
                self.assertEqual([row["observe"] for row in actual[1:]], [False, True, True, False])
                rows = {row["id"]: row for row in first}
                self.assertEqual(len(rows), 84)
                self.assertEqual(sum(row["status"] == "complete" for row in first), 5)
                self.assertEqual(sum(row["status"] == "not_run_cap" for row in first), 73)
                for identifier in SIX:
                    self.assertEqual(rows[identifier], expected[identifier])
                r1 = rows["g-r-n100000-r1"]
                self.assertTrue(r1["cell"]["fresh_seed"])
                self.assertEqual((r1["status"], r1["reason"]), ("not_run_cap", "cap_feasibility"))
                self.assertEqual(r1["prediction"]["seed_ns"], 1315871925088)
                writes = [row for row in first if row["cell"]["stage"] == "G-W"]
                self.assertEqual(len(writes), 72)
                self.assertTrue(all(row["cell"]["fresh_seed"] and row["status"] == "not_run_cap"
                                    and row["prediction"]["seed_ns"] == 1315871925088 for row in writes))
                second = run_g_plan(SPEC, result_root, command, allocation_root=root, reused=manifest["reused"])
                self.assertEqual(len(calls.read_text().splitlines()), 5, "resume redispatched completed cases")
                self.assertEqual(len(second), 84)
                budget = json.loads((root / "g-budget.json").read_text())
                self.assertEqual(budget["exploratory_seed100k"], token)
                self.assertGreater(budget["used_ns"], 1030482596607)
                self.assertIsNone(budget["inflight"])
                for identifier, charge in CHARGES.items():
                    self.assertEqual(budget["cell_used_ns"][identifier], charge)
                self.assertGreater(budget["cell_used_ns"]["g-r-n100000-r0"], 2997368)
                self.assertGreater(budget["cell_used_ns"]["g-r-n100000-r1"], 3102917)

    def test_existing_stages_keep_their_sources_and_normal_entry_requires_seed_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with accepted_protocol_chain(root) as (options, _expected):
                for label in ("base", "branch"):
                    with self.subTest(normal_source=label):
                        with self.assertRaisesRegex(ValueError, "normal G reuse requires the qualified exploratory seed source"):
                            freeze(dict(options, reuse_completed_from=str(root / label)))
                for stage in ("branch-controls", "first-100k-seed-exploratory"):
                    with self.subTest(staged_seed_source=stage):
                        with self.assertRaisesRegex(ValueError, "exploratory seed references require normal G entry"):
                            freeze(dict(options, stage=stage))
                for stage, prior, count in (("branch-controls", "base", 4),
                                             ("first-100k-seed-exploratory", "branch", 5)):
                    with self.subTest(preserved_stage=stage):
                        actual = freeze(dict(options, stage=stage, reuse_completed_from=str(root / prior),
                                             out=str(root / (stage + "-preserved"))))
                        self.assertEqual(actual["stage"], stage)
                        self.assertEqual(len(actual["reused"]), count)
                        self.assertEqual(len(actual["plan"]), 84)
