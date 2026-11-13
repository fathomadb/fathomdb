"""Literal G-only stage/child contracts; no database mock or model execution."""

import json
from pathlib import Path
import sys
import tempfile
import time
import unittest

from campaign import run_campaign
from g_campaign import make_g_plan, run_g_plan


class GPlanTests(unittest.TestCase):
    def test_real_harmless_children_receive_every_required_stage_and_balanced_g_cell(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            calls = root / "calls.jsonl"
            code = "\n".join([
                "import json,pathlib,sys",
                "cell=json.loads(pathlib.Path(sys.argv[1]).read_text())",
                "out=pathlib.Path(sys.argv[2])",
                f"with pathlib.Path({str(calls)!r}).open('a') as stream: stream.write(json.dumps(cell)+'\\n')",
                "(out/'result.json').write_text(json.dumps({'complete':True,'runner_only':True}))",
            ])
            outcomes = run_g_plan(spec, root / "results", [sys.executable, "-c", code])
            actual = [json.loads(line) for line in calls.read_text().splitlines()] if calls.exists() else []
            self.assertEqual(len(actual), 84, "G table omitted required actual child cells")
            self.assertEqual([row["stage"] for row in actual],
                             ["G0", "G1", "G1"] + ["G-R"] * 4 + ["G-O"] * 5 + ["G-W"] * 72)
            self.assertEqual(len({row["id"] for row in actual}), 84)
            self.assertTrue(all(row["status"] == "complete" for row in outcomes))
            reads = [row for row in actual if row["stage"] == "G-R"]
            self.assertEqual([(row["n"], row["repeat"]) for row in reads],
                             [(10000, 0), (100000, 0), (100000, 1), (10000, 1)])
            for row in reads:
                self.assertEqual(row["adapters"], ["engine", "sdk"] if row["repeat"] == 0 else ["sdk", "engine"])
                self.assertEqual(row["routes"], ["text", "hybrid", "embed"] if row["repeat"] == 0 else ["embed", "hybrid", "text"])
                self.assertEqual((row["warmup"], row["samples"]), (500, 1000))
            natural = [row for row in actual if row["stage"] == "G-O" and row["kind"] == "natural"]
            self.assertEqual([(row["repeat"], row["observe"]) for row in natural],
                             [(0, False), (0, True), (1, True), (1, False)])
            self.assertTrue(all(row["samples"] == 50 and row["warmup"] == 0
                                and row["record_first_call"] is False for row in natural))
            writes = [row for row in actual if row["stage"] == "G-W"]
            self.assertEqual({(row["batch_size"], row["searches_per_batch"], row["adapters"][0],
                               row["routes"][0], row["repeat"]) for row in writes},
                             {(batch, ratio, adapter, route, repeat)
                              for batch in [1, 10] for ratio in [1, 10, 100]
                              for adapter in ["engine", "sdk"] for route in ["text", "hybrid", "embed"]
                              for repeat in [0, 1]})
            self.assertTrue(all(row["n"] == 100000 and row["fresh_seed"] is True for row in writes))
            self.assertTrue(all(row["cell_seconds"] == 900 and row["campaign_seconds"] == 14400
                                and row["active_bytes"] == 16 * 1024**3 for row in actual))


if __name__ == "__main__":
    unittest.main()

class GCumulativeBudgetTests(unittest.TestCase):
    def test_exhausted_shared_allocation_runs_no_child_and_retains_all_required_cells(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "g-budget.json").write_text(json.dumps({
                "schema": "slice30-gd-budget/v1", "used_ns": 14400 * 10**9,
                "inflight": None,
            }))
            calls = root / "unexpected-child"
            code = f"import pathlib; pathlib.Path({str(calls)!r}).write_text('started')"
            outcomes = run_g_plan(spec, root, [sys.executable, "-c", code])
            self.assertFalse(calls.exists(), "exhausted G/D allocation still launched an actual child")
            self.assertEqual(len(outcomes), 84)
            self.assertTrue(all(row["status"] == "not_run_cap" and row["complete"] is False
                                and row["reason"] == "shared_time_exhausted" for row in outcomes))
            self.assertEqual(json.loads((root / "g-budget.json").read_text())["used_ns"],
                             14400 * 10**9)
            self.assertEqual(len(json.loads((root / "g-outcomes.json").read_text())), 84)

class GStorageBudgetTests(unittest.TestCase):
    def test_retained_sixteen_gib_refuses_children_and_keeps_failed_artifacts(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            retained = root / "prior-failed-attempt.raw"
            with retained.open("wb") as stream:
                stream.truncate(16 * 1024**3)
            calls = root / "unexpected-child"
            code = f"import pathlib; pathlib.Path({str(calls)!r}).write_text('started')"
            outcomes = run_g_plan(spec, root, [sys.executable, "-c", code])
            self.assertFalse(calls.exists(), "retained raw artifacts exhausted cap but child still ran")
            self.assertEqual(retained.stat().st_size, 16 * 1024**3)
            self.assertEqual(len(outcomes), 84)
            self.assertTrue(all(row["status"] == "not_run_cap" and row["complete"] is False
                                and row["reason"] == "active_storage_exhausted" for row in outcomes))
            self.assertEqual(len(json.loads((root / "g-outcomes.json").read_text())), 84)


class GPreparationBudgetTests(unittest.TestCase):
    def test_actual_preparation_consumes_shared_deadline_before_any_child_dispatch(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            used = 14400 * 10**9 - 20_000_000
            (root / "g-budget.json").write_text(json.dumps({
                "schema": "slice30-gd-budget/v1", "used_ns": used, "inflight": None,
            }))
            calls = root / "unexpected-child"
            prepared = []
            def prepare(cell, allocation):
                started = time.monotonic_ns()
                value = 0
                while time.monotonic_ns() - started < 100_000_000:
                    value += 1
                prepared.append(cell["id"])
                (allocation / "actual-preparation.json").write_text(json.dumps({"iterations": value}))
                return {"prepared": True}
            code = f"import pathlib; pathlib.Path({str(calls)!r}).write_text('started')"
            outcomes = run_g_plan(spec, root, [sys.executable, "-c", code], prepare=prepare)
            self.assertFalse(calls.exists(), "child dispatched after preparation exhausted shared deadline")
            self.assertEqual(prepared, ["g0-smoke"])
            self.assertTrue((root / "actual-preparation.json").exists())
            self.assertEqual(len(outcomes), 84)
            self.assertTrue(all(row["status"] == "not_run_cap" and row["complete"] is False
                                and row["reason"] == "shared_time_exhausted" for row in outcomes))
            self.assertGreaterEqual(json.loads((root / "g-budget.json").read_text())["used_ns"] - used,
                                    100_000_000)


class GResumeBudgetTests(unittest.TestCase):
    def test_actual_completed_child_survives_exhausted_resume_and_only_remaining_cells_refuse(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            calls = root / "calls.jsonl"
            code = "\n".join([
                "import json,pathlib,sys",
                f"with pathlib.Path({str(calls)!r}).open('a') as stream: stream.write('called\\n')",
                "(pathlib.Path(sys.argv[2])/'result.json').write_text(json.dumps({'complete':True}))",
            ])
            completed = run_campaign(make_g_plan(spec)[:1], root, [sys.executable, "-c", code])
            self.assertEqual(completed[0]["status"], "complete")
            (root / "g-budget.json").write_text(json.dumps({
                "schema": "slice30-gd-budget/v1", "used_ns": 14400 * 10**9, "inflight": None,
            }))
            outcomes = run_g_plan(spec, root, [sys.executable, "-c", code])
            self.assertEqual(outcomes[0], completed[0], "exhausted resume erased actual completed child outcome")
            self.assertEqual(calls.read_text().splitlines(), ["called"])
            self.assertEqual(len(outcomes), 84)
            self.assertTrue(all(row["status"] == "not_run_cap" and row["complete"] is False
                                and row["reason"] == "shared_time_exhausted" for row in outcomes[1:]))
            self.assertEqual(json.loads((root / "g-outcomes.json").read_text()), outcomes)

class GCellRetryBudgetTests(unittest.TestCase):
    def test_failed_cell_cannot_renew_spent_nine_hundred_seconds_on_retry(self):
        spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            calls = root / "calls.jsonl"
            code = "\n".join([
                "import json,pathlib,sys",
                "cell=json.loads(pathlib.Path(sys.argv[1]).read_text())",
                f"with pathlib.Path({str(calls)!r}).open('a') as stream: stream.write(cell['id']+'\\n')",
                "(pathlib.Path(sys.argv[2])/'result.json').write_text(json.dumps({'complete':True}))",
            ])
            failed = run_campaign(make_g_plan(spec)[:1], root, [sys.executable, "-c", code + "\nraise SystemExit(1)"])
            self.assertEqual(failed[0]["status"], "failed")
            (root / "g-budget.json").write_text(json.dumps({
                "schema": "slice30-gd-budget/v1", "used_ns": 900 * 10**9,
                "cell_used_ns": {"g0-smoke": 900 * 10**9}, "inflight": None,
            }))
            outcomes = run_g_plan(spec, root, [sys.executable, "-c", code])
            actual = calls.read_text().splitlines()
            self.assertEqual(actual.count("g0-smoke"), 1, "failed G cell renewed its spent 900-second allowance")
            self.assertEqual(len(outcomes), 84)
            self.assertEqual((outcomes[0]["status"], outcomes[0]["reason"]),
                             ("not_run_cap", "cell_time_exhausted"))
            self.assertTrue(all(row["status"] == "complete" for row in outcomes[1:]))
            self.assertEqual(len(list((root / "g0-smoke").glob("attempt-*"))), 1)
            self.assertEqual(json.loads((root / "g-budget.json").read_text())["cell_used_ns"]["g0-smoke"],
                             900 * 10**9)
