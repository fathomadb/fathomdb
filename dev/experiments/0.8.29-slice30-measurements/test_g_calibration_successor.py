"""One source-corrected calibration admission; synthetic metadata never qualifies GPU work."""

from contextlib import contextmanager
import copy
import json
import os
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import g_campaign
import g_closed_entry
import g_runner
from test_g_closed_modes import references
from test_g_closed_plan import SPEC
from test_g_normal_resume import dump

MODE = "closed-copy-calibration"
ISSUE = "positive_default_cuda_evidence_missing"


@contextmanager
def _fixture(allocation):
    old, new = allocation / "stopped", allocation / "corrected"
    spec = dict(SPEC, preparation="closed-copy-v1")
    cell = g_campaign.calibration_plan(spec)[0]
    attempt = old / "results/calibration" / cell["id"] / "attempt-0001"
    binary = str(allocation / "protocol-ordinary-not-executed")
    gpu = {"uuid": "GPU-protocol-only", "ordinal": 0, "name": "RTX 3090 protocol record",
           "driver_version": "580.178.04", "primary_cuda": "/usr/local/cuda-12.6"}
    reused = references()
    g0 = allocation / "accepted-g0"
    dump(g0 / "result.json", {"complete": True, "qualification_issues": []})
    reused["g0-smoke"]["attempt"] = str(g0)
    manifest = {"schema": "slice30-g-campaign/v1", "source_sha": "repaired-policy-source",
        "compiler_inputs": "same-compiled-inputs", "allocation": str(allocation), "root": str(new),
        "stage": MODE, "preparation": "closed-copy-v1", "spec": spec,
        "plan": g_campaign.make_g_plan(spec), "gpu": gpu, "device_policy": "cuda:0", "format": 37,
        "inputs": {key: {"path": str(allocation / key)} for key in ("corpus", "queries")},
        "model_files": {}, "binaries": {"ordinary": {"path": binary}}, "closed_subject": {"fixture": "same"},
        "lock": {"same": "lock"}, "build_receipt": {"same": "build"}, "python_receipt": {"same": "receipt"},
        "python_native": {"same": "native"}, "environment": {}, "reused": reused,
        "closed_limits": {"cell_seconds": 1500, "campaign_seconds": 14400, "active_bytes": 16*1024**3,
                          "common_seconds": 600, "common_bytes": 1024**3, "calibration_cell_bytes": 3*1024**3}}
    dump(new / "manifest.json", manifest)
    original = dict(manifest, source_sha="original-policy-source", root=str(old))
    dump(old / "manifest.json", original)
    report = {"pid": 70001, "executable": binary, "available": True, "requested_provider": "default",
        "schema_version_after": 37, "embedder_identity": dict(SPEC["model"]),
        "embedder_device_resolution": {"requested_policy": "auto", "cuda_compiled": True,
            "selected_cuda_uuid": gpu["uuid"], "effective_device": {
                "kind": "cuda", "uuid": gpu["uuid"], "ordinal": 0, "name": gpu["name"]}}}
    report["embedder_identity"].pop("head_tokens")
    dense = {"qualified_dense_fixture": True, "format": 37, "natural_records": 100000,
             "vector_rows": 100000, "physical_vector_rows": 100000, "span_rows": 100000,
             "pending_count": 0, "generation_failed": 0}
    step = {"family": "G-W", "config": str(attempt / "native-config.json"), "output": str(attempt / "write"),
        "complete": True, "exit": 0, "pid": 70001, "executable": binary,
        "result": {"complete": True, "open_report": report, "before_workload": dense,
                   "sample_count": 1000, "warmup_count": 500, "first_call_count": 0},
        "gpu_samples": [{"pid": 70001, "pid_memory_mib": 100, "gpu_memory_mib": 102,
                         "baseline_gpu_memory_mib": 2, "utilization_percent": 20}]}
    dump(Path(step["config"]), dict(cell, family="G-W"))
    actual = {"complete": False, "qualification_issues": [ISSUE], "steps": [step],
              "preparation": {"validated": True}}
    dump(attempt / "result.json", actual)
    outcome = {"id": cell["id"], "cell": cell, "status": "failed", "attempt": str(attempt),
               "exit": 1, "child_exit": 1, "error": None, "cleanup_uncertain": False,
               "live_owned_pids": [], "elapsed_ns": 132*10**9}
    rows = [outcome] + [{"id": item["id"], "cell": item, "status": "not_run_prerequisite",
                        "complete": False, "reason": "calibration_stopped"}
                       for item in g_campaign.calibration_plan(spec)[1:]]
    stop = {"cell": cell["id"], "outcome": outcome, "path": str(old / "results/calibration/g-outcomes.json")}
    dump(Path(stop["path"]), rows)
    dump(old / "results/calibration-outcomes.json", rows)
    receipt = old / "guardian.json"
    guardian = {"exit": 1, "resource_stop": None, "cleanup_uncertain": False, "live_owned_pids": [],
        "final_sample_error": None, "sha_start": original["source_sha"], "sha_end": original["source_sha"],
        "delta_start": "", "delta_end": "", "boundary": {"status": "quiescent", "populated_after": False,
                                                          "removed_exact_empty_boundary": True}}
    dump(receipt, guardian)
    dump(old / "last-run.json", {"environment_ok": True, "stage_complete": False, "guardian_receipt": str(receipt)})
    budget_path = allocation / "g-budget.json"
    budget = {"schema": "slice30-gd-budget/v1", "used_ns": 2004*10**9, "inflight": None,
        "cell_used_ns": {cell["id"]: 132*10**9, "accepted-seed": 657*10**9},
        "common_used_ns": {MODE: 27*10**9, "other-mode": 123},
        "exploratory_seed100k": {"consumed": True, "original": "preserved"},
        "closed_calibration_stop": stop}
    dump(budget_path, budget)
    fixture = SimpleNamespace(old=old, new=new, cell=cell, old_cell=attempt.parent, actual=actual,
        original=original, manifest=manifest, outcome=outcome, stop=stop, guardian=guardian,
        budget=budget, budget_path=budget_path, calls=allocation / "calls.jsonl",
        after_preflight=None, before_plan=None, crash=False, succeed=False, growth=0)
    real_plan = g_campaign.run_g_plan

    def command(item, deadline):
        code = "\n".join([
            "import json,pathlib,sys,time",
            "out=pathlib.Path(sys.argv[2])",
            f"budget=json.loads(pathlib.Path({str(budget_path)!r}).read_text())",
            f"record=dict(id={item['id']!r},remaining_ns={deadline}-time.monotonic_ns(),budget=budget)",
            f"with pathlib.Path({str(fixture.calls)!r}).open('a') as stream:stream.write(json.dumps(record)+'\\n')",
            f"(out/'growth').write_bytes(b'x'*{fixture.growth})",
            f"(out/'result.json').write_text(json.dumps(dict(complete={fixture.succeed!r},runner_only=True)))",
        ])
        return [sys.executable, "-c", code]

    def controlled_plan(specification, root, _command, **options):
        if fixture.before_plan:
            fixture.before_plan()
        if fixture.crash:
            raise RuntimeError("controller crash fixture")
        return real_plan(specification, root, command, **options)

    def common(_allocation, _mode, action, options):
        if action == "preflight":
            prepared = g_closed_entry._preflight(Path(options["root"]), options.get("corrected_calibration_from"))
            if fixture.after_preflight:
                fixture.after_preflight(prepared)
            return prepared
        return None

    def invoke(explicit=True, selected=None):
        selected = selected or new
        if selected != new:
            dump(selected / "manifest.json", dict(manifest, root=str(selected)))
        try:
            return g_runner.run_frozen(selected, old) if explicit else g_runner.run_frozen(selected)
        except (ValueError, RuntimeError) as error:
            return error

    fixture.invoke = invoke
    fixture.records = lambda: [json.loads(line) for line in fixture.calls.read_text().splitlines()] if fixture.calls.exists() else []
    fixture.read_budget = lambda: json.loads(budget_path.read_text())
    with patch.dict(os.environ, S30_TASK=str(allocation)), patch("g_boundary.require_known_containment"), \
            patch("g_freeze.validate_frozen", side_effect=lambda selected: json.loads((selected / "manifest.json").read_text())), \
            patch("g_gpu.inspect_gpu", return_value={"applications": []}), \
            patch.object(g_closed_entry, "_call", common), patch.object(g_campaign, "run_g_plan", controlled_plan):
        yield fixture


class GCalibrationSuccessorTests(unittest.TestCase):
    def test_default_new_root_stays_stopped_but_explicit_corrected_successor_dispatches_once(self):
        with tempfile.TemporaryDirectory() as temporary, _fixture(Path(temporary)) as f:
            originals = {path: path.read_bytes() for path in f.old.rglob("*") if path.is_file()}
            f.invoke(explicit=False)
            self.assertEqual(f.records(), [], "default new root bypassed the retained calibration stop")
            result = f.invoke()
            detail = str(result) if isinstance(result, Exception) else "retained original stop"
            self.assertEqual(len(f.records()), 1, f"eligible public successor must dispatch once: {detail}")
            budget = f.read_budget()
            self.assertEqual(budget["closed_calibration_stop"], f.stop)
            slot = budget["closed_calibration_successor"]
            self.assertEqual((slot["from_root"], slot["root"]), (str(f.old), str(f.new)))
            self.assertEqual((slot["from_source_sha"], slot["source_sha"]), (f.original["source_sha"], f.manifest["source_sha"]))
            self.assertEqual(slot["original_stop"], f.stop)
            self.assertEqual(slot["stop"]["cell"], f.cell["id"])
            self.assertEqual({path: path.read_bytes() for path in originals}, originals)
            self.assertEqual(budget["common_used_ns"], f.budget["common_used_ns"])
            self.assertEqual(budget["exploratory_seed100k"], f.budget["exploratory_seed100k"])
            self.assertEqual(budget["cell_used_ns"]["accepted-seed"], 657*10**9)
            charged = budget["cell_used_ns"][f.cell["id"]] - 132*10**9
            self.assertGreater(charged, 0)
            self.assertEqual(budget["used_ns"] - f.budget["used_ns"], charged)
            self.assertLessEqual(f.records()[0]["remaining_ns"], (1500-132-40)*10**9)
            self.assertIsNone(budget["inflight"])
            f.invoke()
            f.invoke(selected=Path(temporary) / "another-root")
            self.assertEqual(len(f.records()), 1, "consumed admission was renewed by replay/new root")
            self.assertEqual(f.read_budget()["closed_calibration_successor"], slot)

    def test_only_completed_auto_policy_defect_with_strict_cleanup_is_eligible(self):
        cases = ("wrong_issue", "extra_issue", "native_incomplete", "step_incomplete", "wrong_policy",
                 "already_explicit", "wrong_device", "no_engagement", "population", "preparation",
                 "success", "cell_cleanup", "outer_cleanup", "resource_stop", "inflight", "compiler_inputs")
        for case in cases:
            with self.subTest(case=case), tempfile.TemporaryDirectory() as temporary, _fixture(Path(temporary)) as f:
                step = f.actual["steps"][0]
                device = step["result"]["open_report"]["embedder_device_resolution"]
                if case == "wrong_issue": f.actual["qualification_issues"] = ["actual_call_population_mismatch"]
                elif case == "extra_issue": f.actual["qualification_issues"].append("route_error_or_nonfinite384")
                elif case == "native_incomplete": step["result"]["complete"] = False
                elif case == "step_incomplete": step["complete"] = False
                elif case == "wrong_policy": device["requested_policy"] = "cpu"
                elif case == "already_explicit": device["requested_policy"] = "cuda:0"
                elif case == "wrong_device": device["effective_device"]["uuid"] = "other-GPU"
                elif case == "no_engagement": step["gpu_samples"] = []
                elif case == "population": step["result"]["sample_count"] = 999
                elif case == "preparation": f.actual["preparation"]["validated"] = False
                elif case == "success": f.outcome["status"] = "complete"
                elif case == "cell_cleanup": f.outcome["cleanup_uncertain"] = True
                elif case == "outer_cleanup": f.guardian["cleanup_uncertain"] = True
                elif case == "resource_stop": f.guardian["resource_stop"] = "host_memory"
                elif case == "inflight": f.budget["inflight"] = {"status": "cleanup_uncertain"}
                elif case == "compiler_inputs": f.manifest["compiler_inputs"] = "different-compiled-inputs"
                dump(Path(f.outcome["attempt"]) / "result.json", f.actual)
                dump(f.old / "guardian.json", f.guardian)
                dump(f.budget_path, f.budget)
                dump(f.new / "manifest.json", f.manifest)
                f.invoke()
                self.assertEqual(f.records(), [], f"unrelated failure became policy-defect authority: {case}")
                self.assertNotIn("closed_calibration_successor", f.read_budget())

    def test_stop_binding_is_rechecked_after_preflight_before_consumption(self):
        with tempfile.TemporaryDirectory() as temporary, _fixture(Path(temporary)) as f:
            def changed_stop(_prepared):
                budget = f.read_budget()
                budget["closed_calibration_stop"]["outcome"]["error"] = "new stop after preflight"
                dump(f.budget_path, budget)
            f.after_preflight = changed_stop
            f.invoke()
            self.assertEqual(f.records(), [])
            self.assertNotIn("closed_calibration_successor", f.read_budget())

    def test_active_permit_cannot_suppress_new_slot_stop_or_different_binding(self):
        for change in ("slot_stop", "slot_binding", "original_stop"):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as temporary, _fixture(Path(temporary)) as f:
                def change_slot():
                    budget = f.read_budget()
                    slot = budget.get("closed_calibration_successor")
                    if slot is None:
                        return
                    if change == "slot_stop":
                        slot["stop"] = {"cell": f.cell["id"], "outcome": dict(f.outcome, error="new successor stop"),
                                        "path": str(f.new / "results/calibration/g-outcomes.json")}
                    elif change == "slot_binding": slot["source_sha"] = "different-successor-source"
                    else: budget["closed_calibration_stop"]["outcome"]["error"] = "different-original-stop"
                    dump(f.budget_path, budget)
                f.before_plan = change_slot
                f.invoke()
                self.assertEqual(f.records(), [], f"active permit suppressed {change}")
                slot = f.read_budget().get("closed_calibration_successor")
                self.assertIsNotNone(slot, "valid explicit invocation never consumed its one slot")
                if change == "slot_stop": self.assertEqual(slot["stop"]["outcome"]["error"], "new successor stop")

    def test_consumption_survives_controller_crash_without_payload_or_renewal(self):
        with tempfile.TemporaryDirectory() as temporary, _fixture(Path(temporary)) as f:
            f.crash = True
            f.invoke()
            slot = f.read_budget().get("closed_calibration_successor")
            self.assertIsNotNone(slot, "admission was not durable before controller entry")
            self.assertEqual(f.records(), [])
            f.crash = False
            f.invoke()
            self.assertEqual(f.records(), [], "controller crash renewed one-shot admission")
            self.assertEqual(f.read_budget()["closed_calibration_successor"], slot)

    def test_success_does_not_enable_another_successor(self):
        with tempfile.TemporaryDirectory() as temporary, _fixture(Path(temporary)) as f:
            f.succeed = True
            f.invoke()
            self.assertEqual(len(f.records()), 4, "valid successor did not retain the fixed four-cell order")
            slot = f.read_budget()["closed_calibration_successor"]
            self.assertNotIn("stop", slot)
            f.invoke(selected=Path(temporary) / "after-success")
            self.assertEqual(len(f.records()), 4, "success reset the consumed successor slot")
            self.assertEqual(f.read_budget()["closed_calibration_successor"], slot)

    def test_exhausted_cumulative_cell_time_refuses_without_renewing_admission(self):
        with tempfile.TemporaryDirectory() as temporary, _fixture(Path(temporary)) as f:
            f.budget["cell_used_ns"][f.cell["id"]] = 1500*10**9
            dump(f.budget_path, f.budget)
            f.invoke()
            self.assertEqual(f.records(), [])
            budget = f.read_budget()
            self.assertEqual(budget["cell_used_ns"], f.budget["cell_used_ns"])
            self.assertEqual(budget["closed_calibration_successor"]["stop"]["outcome"]["reason"], "cell_time_exhausted")
            f.invoke()
            self.assertEqual(f.records(), [])

    def test_exhausted_prior_cell_storage_refuses_before_any_new_payload(self):
        with tempfile.TemporaryDirectory() as temporary, _fixture(Path(temporary)) as f:
            cap = 65536
            (f.old_cell / "retained").write_bytes(b"x" * (cap - g_campaign.active_bytes(f.old_cell)))
            with patch.object(g_campaign, "CALIBRATION_CELL_BYTES", cap):
                f.invoke()
            self.assertEqual(f.records(), [])
            budget = f.read_budget()
            self.assertEqual(budget["closed_calibration_stop"], f.stop)
            self.assertEqual(budget["closed_calibration_successor"]["stop"]["outcome"]["reason"],
                             "calibration_cell_storage_exhausted")

    def test_reservation_deducts_already_retained_same_cell_bytes(self):
        with tempfile.TemporaryDirectory() as temporary, _fixture(Path(temporary)) as f:
            cap, retained = 65536, 32768
            (f.old_cell / "retained").write_bytes(b"x" * (retained - g_campaign.active_bytes(f.old_cell)))
            actual_bytes = g_campaign.active_bytes
            allocation = Path(temporary)
            observed = 16*1024**3 - (4*cap + 1024**3) + retained//2
            def retained_bytes(path):
                return observed if Path(path) == allocation else actual_bytes(path)
            with patch.object(g_campaign, "CALIBRATION_CELL_BYTES", cap), \
                    patch.object(g_campaign, "active_bytes", retained_bytes):
                f.invoke()
            self.assertEqual(len(f.records()), 1, "phase reservation renewed prior retained cell bytes")

    def test_live_growth_counts_predecessor_and_new_cell_bytes_together(self):
        with tempfile.TemporaryDirectory() as temporary, _fixture(Path(temporary)) as f:
            cap, retained = 65536, 32768
            (f.old_cell / "retained").write_bytes(b"x" * (retained - g_campaign.active_bytes(f.old_cell)))
            f.growth = 40000
            with patch.object(g_campaign, "CALIBRATION_CELL_BYTES", cap):
                f.invoke()
            self.assertEqual(len(f.records()), 1, "live cumulative-storage fixture did not start its harmless child")
            actual = json.loads((f.new / "results/calibration-outcomes.json").read_text())
            self.assertEqual(actual[0]["status"], "failed")
            self.assertIn("calibration_cell_storage_exhausted", actual[0]["error"])
            self.assertLess(g_campaign.active_bytes(f.new / "results/calibration" / f.cell["id"]), cap,
                            "fixture exceeded current-root cap without needing predecessor bytes")
            self.assertTrue(all(row["status"].startswith("not_run_") for row in actual[1:]))
            self.assertEqual(f.read_budget()["closed_calibration_successor"]["stop"]["outcome"], actual[0])
            self.assertEqual(f.read_budget()["closed_calibration_stop"], f.stop)
