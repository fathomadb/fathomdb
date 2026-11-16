"""Bounded required-write invocations retain the full inventory and actual charges."""

from copy import deepcopy
from contextlib import redirect_stdout
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import g_closed_entry
import g_runner
from g_campaign import run_g_plan
from test_g_closed_modes import harmless
from test_g_closed_write_gate import setup


class GBoundedWriteTests(unittest.TestCase):
    def test_one_new_write_retains_all_rows_and_continues_in_original_order(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            spec, reused = setup()
            spec["_max_new_writes"] = 1
            original = deepcopy(reused)
            prior = {"schema": "slice30-gd-budget/v1", "used_ns": 123456789,
                     "inflight": None, "cell_used_ns": {"historical-D-M-S50": 123456789},
                     "exploratory_seed100k": {"consumed": True}}
            (root / "g-budget.json").write_text(json.dumps(prior))
            rows = run_g_plan(spec, root / "results", harmless(root / "calls"),
                              allocation_root=root, reused=reused)
            self.assertEqual((root / "calls").read_text().splitlines(),
                             ["g-w-b1-s1-engine-text-r0"],
                             "bounded invocation dispatched beyond the first required write")
            self.assertEqual(len(rows), 84)
            self.assertEqual(sum(row["status"] == "reused_complete" for row in rows), 12)
            self.assertEqual(sum(row["status"] == "complete" for row in rows), 1)
            held = [row for row in rows if row["status"] == "not_run_stage"]
            self.assertEqual(len(held), 71)
            self.assertTrue(all(row["reason"] == "bounded_write_dispatch" for row in held))
            for identifier, reference in original.items():
                self.assertEqual(next(row for row in rows if row["id"] == identifier), reference)
            budget = json.loads((root / "g-budget.json").read_text())
            self.assertGreater(budget["used_ns"], prior["used_ns"])
            self.assertEqual(budget["cell_used_ns"]["historical-D-M-S50"], 123456789)
            self.assertEqual(budget["exploratory_seed100k"], {"consumed": True})
            self.assertIsNone(budget["inflight"])
            again = run_g_plan(spec, root / "results", harmless(root / "calls"),
                               allocation_root=root, reused=reused)
            self.assertEqual((root / "calls").read_text().splitlines(),
                             ["g-w-b1-s1-engine-text-r0", "g-w-b1-s1-engine-hybrid-r0"])
            self.assertEqual(len(again), 84)
            self.assertEqual(sum(row["status"] == "complete" for row in again), 2)

    def test_refused_first_row_does_not_select_a_successful_substitute(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            spec, reused = setup()
            spec["_max_new_writes"] = 1
            def refuse(_cell, _previous, _remaining, _retained):
                return {"status": "not_run_cap", "complete": False, "reason": "observed_capacity"}
            rows = run_g_plan(spec, root / "results", harmless(root / "forbidden"),
                              allocation_root=root, reused=reused, assess=refuse)
            self.assertFalse((root / "forbidden").exists())
            self.assertEqual(sum(row["status"] == "not_run_cap" for row in rows), 1,
                             "refusal advanced to another required write")
            self.assertEqual(sum(row["status"] == "not_run_stage" for row in rows), 71)

    def test_public_cli_forwards_limit_and_keeps_partial_exit(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "last-run.json").write_text(json.dumps({"environment_ok": True}))
            rows = [{"status": "complete"}, {"status": "not_run_stage"}]
            with patch.dict(os.environ, AGENT_LONG="1"), \
                    patch.object(sys, "argv", ["g_runner.py", "run", str(root), "--max-new-writes", "1"]), \
                    patch.object(g_runner, "run_frozen", return_value=rows) as run, redirect_stdout(io.StringIO()):
                with self.assertRaises(SystemExit) as exit_code:
                    g_runner.main()
            self.assertEqual(exit_code.exception.code, 1)
            run.assert_called_once_with(root, None, max_new_writes=1)

    def test_public_run_rejects_limit_outside_write_mode_before_common_work(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for mode in ("closed-copy-calibration", "closed-copy-read-r1", None):
                with self.subTest(mode=mode):
                    (root / "manifest.json").write_text(json.dumps({"stage": mode}))
                    with patch.object(g_closed_entry, "run_entry") as run:
                        with self.assertRaisesRegex(ValueError, "closed write-matrix"):
                            g_runner.run_frozen(root, max_new_writes=1)
                    run.assert_not_called()

    def test_public_run_passes_bound_through_supervised_entry_without_refreezing(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bootstrap = {"stage": "closed-copy-write-matrix", "preparation": "closed-copy-v1",
                         "allocation": str(root)}
            original = json.dumps(bootstrap)
            (root / "manifest.json").write_text(original)
            prepared = {"spec": setup()[0], "device_policy": "cuda:0"}
            def common(_allocation, _mode, action, _options):
                return prepared if action == "preflight" else None
            def controller(manifest, _root):
                self.assertEqual(manifest["spec"]["_max_new_writes"], 1)
                return ["bounded-protocol"]
            with patch.dict(os.environ, S30_TASK=str(root)), \
                    patch("g_boundary.require_known_containment"), \
                    patch.object(g_closed_entry, "_call", side_effect=common) as calls, \
                    patch.object(g_runner, "run_prepared", side_effect=controller):
                self.assertEqual(g_runner.run_frozen(root, max_new_writes=1), ["bounded-protocol"])
            self.assertEqual([call.args[2] for call in calls.call_args_list], ["preflight", "finish"])
            self.assertEqual((root / "manifest.json").read_text(), original)

    def test_invalid_bounds_refuse_before_any_child(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for limit in (0, -1, 73, True, 1.5):
                with self.subTest(limit=limit):
                    spec, reused = setup()
                    spec["_max_new_writes"] = limit
                    with self.assertRaisesRegex(ValueError, "1..72"):
                        run_g_plan(spec, root / "results", harmless(root / "forbidden"),
                                   allocation_root=root, reused=reused)
            self.assertFalse((root / "forbidden").exists())

    def test_failed_first_child_stops_before_second_dispatch(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            spec, reused = setup()
            spec["_max_new_writes"] = 1
            command = harmless(root / "calls")
            command[2] = command[2].replace("'complete':True", "'complete':False")
            rows = run_g_plan(spec, root / "results", command, allocation_root=root, reused=reused)
            self.assertEqual((root / "calls").read_text().splitlines(), ["g-w-b1-s1-engine-text-r0"])
            self.assertEqual(sum(row["status"] == "failed" for row in rows), 1)
            self.assertEqual(sum(row["status"] == "not_run_stage" for row in rows), 71)
