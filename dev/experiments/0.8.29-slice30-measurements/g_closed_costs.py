"""Matched100k calibration cost extraction; actual qualification precedes aggregation."""

import ast
import json
from pathlib import Path
import subprocess

from g_campaign import active_bytes


def calibration_costs(records, adapter):
    """Aggregate qualified dense/sparse records for one adapter, or leave costs unavailable.

    Host remainder subtracts disjoint matched workload/warmup intervals from the
    same native host wall. It includes open/close/I/O and is not exclusive GPU
    attribution. Retained nonbaseline bytes may overlap per-row growth, so their
    separate forecast use is deliberately conservative.
    """
    expected = [f"g-cal-{adapter}-b10-s1", f"g-cal-{adapter}-b1-s100"]
    selected = {row["cell"]["id"]: row for row in records if row["cell"]["adapters"] == [adapter]}
    if set(selected) != set(expected):
        return {}
    costs = {"n": 100000, "adapter": adapter, "cells": expected}
    def retain(key, value):
        if type(value) is not int or value < 0:
            raise ValueError("negative or unavailable matched calibration bound")
        costs[key] = max(costs.get(key, 0), value)
    try:
        for identifier in expected:
            row = selected[identifier]
            cell, actual = row["cell"], row["actual"]
            if cell["n"] != 100000 or actual["preparation"]["validated"] is not True:
                return {}
            preparation, step = actual["preparation"], actual["steps"][0]
            result = step["result"]
            retain("copy_ns", preparation["copy_ns"])
            retain("validation_ns", preparation["validation_ns"])
            retain("copy_bytes", preparation["copied_bytes"])
            remainder = (step["host_wall_ns"] - sum(item["wall_ns"] for item in result["passes"])
                         - sum(item["elapsed_ns"] for item in result["warmups"]))
            retain("driver_remainder_ns", remainder)
            retain("observer_tail_max_ns", step["observation_max_wall_ns"])
            rows = ((cell["samples"] + cell["searches_per_batch"] - 1)
                    // cell["searches_per_batch"]) * cell["batch_size"]
            retain("physical_growth_per_row",
                   (row["peak_store_bytes"] - preparation["copied_bytes"] + rows - 1) // rows)
            retain("fixed_bytes", row["retained_bytes"] - preparation["copied_bytes"])
            for batch in result["batches"]:
                retain("write_row_max_ns", (batch["write_ns"] + batch["rows"] - 1) // batch["rows"])
                if batch["adjacent_readiness"]["available"] is not True:
                    return {}
                retain("readiness_max_ns", batch["adjacent_readiness"]["elapsed_ns"])
            for probe in result["probes"]:
                retain("point_probe_max_ns", probe["point_probe"]["elapsed_ns"])
                retain("text_probe_max_ns", probe["text_probe"]["elapsed_ns"])
    except (KeyError, TypeError, ValueError, ZeroDivisionError):
        return {}
    required = {"copy_ns", "validation_ns", "copy_bytes", "driver_remainder_ns", "observer_tail_max_ns",
                "physical_growth_per_row", "fixed_bytes", "write_row_max_ns",
                "readiness_max_ns", "point_probe_max_ns", "text_probe_max_ns"}
    return costs if required <= costs.keys() else {}


def load_calibration(root, current):
    """Load only four qualified matched records with exact native inputs and known cleanup."""
    from g_freeze import SOURCE
    from g_reuse import BASE_CELLS, NORMAL_CELLS
    from g_runner import qualify_step
    root = Path(root).resolve()
    original = json.loads((root / "manifest.json").read_text())
    if original.get("root") != str(root) or original.get("stage") != "closed-copy-calibration":
        raise ValueError("cost source is not the closed fixed calibration")
    for key in ("schema", "compiler_inputs", "lock", "inputs", "model_files", "binaries",
                "build_receipt", "python_receipt", "python_native", "allocation", "device_policy",
                "format", "environment", "closed_subject"):
        if original.get(key) != current.get(key):
            raise ValueError(f"calibration input/native equivalence differs: {key}")
    for key in ("uuid", "ordinal", "name", "driver_version", "primary_cuda"):
        if original["gpu"].get(key) != current["gpu"].get(key):
            raise ValueError(f"calibration device differs: {key}")
    eleven = BASE_CELLS | NORMAL_CELLS | {"g-o-branches", "g1-n100000"}
    if (set(original.get("reused", {})) != eleven
            or any(original["reused"][key] != current.get("reused", {}).get(key) for key in eleven)):
        raise ValueError("calibration does not retain the exact accepted11 references")
    relative = "dev/experiments/0.8.29-slice30-measurements/g_runner.py"
    old = subprocess.check_output(["git", "show", f"{original['source_sha']}:{relative}"], cwd=SOURCE).decode()
    def qualifier(text):
        return [ast.dump(node, include_attributes=False) for node in ast.parse(text).body
                if isinstance(node, ast.FunctionDef) and node.name in ("qualify_step", "execute_cell")]
    if qualifier(old) != qualifier((SOURCE / relative).read_text()):
        raise ValueError("calibration qualification semantics differ")
    invocation = json.loads((root / "last-run.json").read_text())
    if invocation.get("environment_ok") is not True or invocation.get("stage_complete") is not True:
        raise ValueError("calibration qualification/cleanup is incomplete")
    receipt = json.loads(Path(invocation["guardian_receipt"]).read_text())
    boundary = receipt.get("boundary") or {}
    if (receipt.get("exit") not in (0, 1) or receipt.get("resource_stop") is not None
            or receipt.get("cleanup_uncertain") is not False or receipt.get("live_owned_pids") != []
            or receipt.get("final_sample_error") is not None
            or receipt.get("sha_start") != original["source_sha"] or receipt.get("sha_end") != original["source_sha"]
            or receipt.get("delta_start") != "" or receipt.get("delta_end") != ""
            or boundary.get("status") != "quiescent" or boundary.get("populated_after") is not False
            or boundary.get("removed_exact_empty_boundary") is not True):
        raise ValueError("calibration final guardian/source evidence is unavailable")
    rows = json.loads((root / "results/calibration-outcomes.json").read_text())
    expected = [f"g-cal-{adapter}-b{batch}-s{ratio}" for batch, ratio in ((10, 1), (1, 100))
                for adapter in ("engine", "sdk")]
    if [row["id"] for row in rows] != expected:
        raise ValueError("calibration table/order is incomplete")
    records = []
    for row in rows:
        cell = row["cell"]
        if (row.get("status") != "complete" or row.get("exit") != 0 or row.get("cleanup_uncertain") is not False
                or row.get("live_owned_pids") != [] or cell.get("n") != 100000
                or cell.get("warmup") != 500 or cell.get("samples") != 1000
                or cell.get("fresh_copy") is not True or cell.get("fresh_seed") is not False):
            raise ValueError("calibration actual population/cleanup is incomplete")
        actual = json.loads(Path(row["attempt"], "result.json").read_text())
        if (actual.get("complete") is not True or actual.get("qualification_issues") != []
                or len(actual.get("steps", [])) != 1):
            raise ValueError("calibration actual native result did not qualify")
        step = actual["steps"][0]
        if (qualify_step(step, original) or step["executable"] != original["binaries"]["ordinary"]["path"]
                or not any(sample.get("pid") == step.get("pid") and sample.get("pid_memory_mib", 0) > 0
                           and sample.get("gpu_memory_mib", 0) > sample.get("baseline_gpu_memory_mib", 0)
                           and sample.get("utilization_percent", 0) > 0 for sample in step.get("gpu_samples", []))):
            raise ValueError("calibration actual same-PID Default/GPU evidence is unavailable")
        records.append(dict(row, actual=actual, retained_bytes=active_bytes(row["attempt"])))
    costs = {adapter: calibration_costs(records, adapter) for adapter in ("engine", "sdk")}
    if any(not value for value in costs.values()):
        raise ValueError("matched calibration costs are unavailable")
    return {"records": records, "costs": costs,
            "origin": {"root": str(root), "source_sha": original["source_sha"],
                       "campaign_uuid": original["campaign_uuid"], "guardian_receipt": invocation["guardian_receipt"]}}
