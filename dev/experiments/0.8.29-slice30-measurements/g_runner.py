"""Frozen G production runner, using current bounded driver and attempt contracts."""

import json
from pathlib import Path

from g_driver import execute_stage

MODEL = {"name": "fathomdb-bge-small-en-v1.5", "revision": "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a", "dimension": 384}


def qualify_step(step, manifest):
    """Check actual current-program facts; protocol controls never qualify Default work."""
    issues = []
    result = step.get("result") or {}
    config = json.loads(Path(step["config"]).read_text())
    if result.get("runner_only"):
        issues.append("runner_only_control")
    if result.get("negative_control") or result.get("control") == "negative_no_provider":
        issues.append("negative_control_cannot_qualify")
    if not step.get("complete") or result.get("complete") is not True:
        issues.append("native_step_incomplete")
    family = step["family"]
    reports = list(result.get("open_reports", []))
    reports += [result[key] for key in ("open_report", "reopen_report") if isinstance(result.get(key), dict)]
    expected = (2 if family == "G-seed" else 1 if family in ("G-W", "G-source-fixture")
                else len(config.get("adapters", [])))
    if len(reports) != expected or not reports:
        issues.append("actual_open_reports_missing")
    gpu = manifest.get("gpu") or {}
    if not gpu.get("uuid") or type(gpu.get("ordinal")) is not int:
        issues.append("reserved_gpu_identity_missing")
    if gpu.get("driver_version") != "580.178.04" or gpu.get("primary_cuda") != "/usr/local/cuda-12.6":
        issues.append("required_host_driver_or_toolkit_evidence_missing")
    for report in reports:
        if (report.get("pid") != step.get("pid") or not step.get("pid")
                or report.get("executable") != step.get("executable")):
            issues.append("native_pid_or_executable_mismatch")
        if (report.get("available") is not True or report.get("requested_provider") != "default"
                or report.get("schema_version_after") != 37 or report.get("embedder_identity") != MODEL):
            issues.append("default_model_or_format_mismatch")
        device = report.get("embedder_device_resolution") or {}
        effective = device.get("effective_device") or {}
        if (device.get("requested_policy") != f"cuda:{gpu.get('ordinal')}"
                or device.get("cuda_compiled") is not True
                or device.get("selected_cuda_uuid") != gpu.get("uuid")
                or effective.get("kind") != "cuda" or effective.get("uuid") != gpu.get("uuid")
                or effective.get("ordinal") != gpu.get("ordinal")
                or "3090" not in (effective.get("name") or "") or "K620" in (effective.get("name") or "")):
            issues.append("positive_default_cuda_evidence_missing")
        if (effective.get("driver_version") not in (None, "580.178.04")
                or (effective.get("cuda_toolkit_version") is not None
                    and not str(effective["cuda_toolkit_version"]).startswith("12.6"))):
            issues.append("provider_driver_or_toolkit_mismatch")
        if report.get("embedder_download_ms") not in (None, 0):
            issues.append("unexpected_runtime_download")
    fixtures = list(result.get("fixtures", []))
    fixtures += [result[key] for key in ("before_close", "after_reopen", "before_workload", "fixture")
                 if isinstance(result.get(key), dict)]
    if not fixtures:
        issues.append("qualified_dense_fixture_missing")
    n = 2 if family == "G-source-fixture" else config["n"]
    for fixture in fixtures:
        if (fixture.get("qualified_dense_fixture") is not True or fixture.get("format") != 37
                or fixture.get("natural_records") != n or fixture.get("vector_rows") != n
                or fixture.get("span_rows") != n or fixture.get("physical_vector_rows") != n
                or fixture.get("pending_count") != 0 or fixture.get("generation_failed") != 0):
            issues.append("qualified_dense_fixture_missing")
    rows = result.get("samples", []) + result.get("warmups", []) + result.get("first_calls", [])
    if any(row.get("ok") is not True or (row.get("route") == "embed"
           and (row.get("dimension") != 384 or row.get("finite") is not True)) for row in rows):
        issues.append("route_error_or_nonfinite384")
    if family in ("G-R", "G-O", "G-W"):
        scale = len(config["adapters"]) * len(config["routes"])
        first = 1 if config.get("record_first_call", True) and family != "G-W" else 0
        if (result.get("sample_count") != config["samples"] * scale
                or result.get("warmup_count") != config["warmup"] * scale
                or result.get("first_call_count") != first * scale):
            issues.append("actual_call_population_mismatch")
        traces = result.get("observation_batches", [])
        if config.get("observe") and (len(traces) < len(rows) or any(
                trace.get("dropped_records") != 0 or trace.get("issues") != []
                or not trace.get("records") for trace in traces)):
            issues.append("observer_attribution_incomplete")
    return sorted(set(issues))


def execute_cell(cell, manifest, attempt, deadline_ns):
    """Execute the fixed cell and retain its complete production qualification result."""
    result = execute_stage(cell, manifest, attempt, deadline_ns)
    issues = [issue for step in result["steps"] for issue in qualify_step(step, manifest)]
    engaged = any(sample.get("pid") == step.get("pid") and sample.get("pid_memory_mib", 0) > 0
                  and sample.get("gpu_memory_mib", 0) > sample.get("baseline_gpu_memory_mib", 0)
                  and sample.get("utilization_percent", 0) > 0
                  for step in result["steps"] for sample in step.get("gpu_samples", []))
    if not engaged:
        issues.append("actual_reserved_gpu_engagement_unobserved")
    if cell["kind"] == "smoke":
        for step in result["steps"]:
            report = step.get("result") or {}
            texts = [row for row in report.get("samples", []) if row.get("route") == "text"]
            if step["family"] == "G-R" and (not texts or any(row.get("ids") != ["l:s30doc-0"] for row in texts)):
                issues.append("tiny_literal_lexical_ids_mismatch")
    result["qualification_issues"] = sorted(set(issues))
    result["complete"] = result["complete"] and not issues
    (Path(attempt) / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result
