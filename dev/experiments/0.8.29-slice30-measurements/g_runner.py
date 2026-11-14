"""Frozen G production runner, using current bounded driver and attempt contracts."""

import argparse
import fcntl
import os
import time
import traceback
import uuid
import json
from itertools import islice
import sys
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



def qualified_result(outcome, cache=None):
    """Read only completed current production results, never protocol-only controls."""
    if not outcome or outcome.get("status") not in ("complete", "reused_complete"):
        return None
    key = outcome["attempt"]
    if cache is not None and key in cache:
        return cache[key]
    try:
        result = json.loads(Path(outcome["attempt"], "result.json").read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return None
    qualified = result if result.get("complete") is True and result.get("qualification_issues") == [] else None
    if cache is not None:
        cache[key] = qualified
    return qualified


def observed_write_costs(cell, previous, manifest, cache):
    """Collect current qualified G1/G-R/G-O costs for the exact G-W adapter and route."""
    seed_outcome = previous.get("g1-n100000")
    read_outcome = previous.get(f"g-r-n100000-r{cell['repeat']}")
    branch_outcome = previous.get("g-o-branches")
    seed = qualified_result(seed_outcome, cache)
    read = qualified_result(read_outcome, cache)
    branches = qualified_result(branch_outcome, cache)
    if any(value is None for value in (seed, read, branches)):
        return {}
    adapter, route = cell["adapters"][0], cell["routes"][0]
    seed_step = seed["steps"][0]
    seed_report = seed_step["result"]
    read_step = next((step for step in read["steps"] if step["family"] == "G-R"), None)
    if read_step is None:
        return {}
    report = read_step["result"]
    selected = next((item for item in report["passes"] if item["adapter"] == adapter and item["route"] == route), None)
    texts = [row["elapsed_ns"] for row in report["samples"] if row["adapter"] == adapter and row["route"] == "text" and row["ok"]]
    commits = [step["result"] for step in branches["steps"] if step["family"] == "G-W"
               and json.loads(Path(step["config"]).read_text())["adapters"] == [adapter]]
    probes = [probe for item in commits for probe in item["probes"]]
    batches = [batch for item in commits for batch in item["batches"]]
    closes = [row["elapsed_ns"] for row in report["close_outcomes"] if row["adapter"] == adapter]
    if not selected or not texts or not probes or not batches or not closes:
        return {}
    raw = [path for path in Path(read_step["output"]).glob("*.json*") if path.is_file()]
    record_count = report["sample_count"] + report["warmup_count"] + report["first_call_count"]
    raw_per_record = (sum(path.stat().st_size for path in raw) + record_count - 1) // record_count
    with Path(manifest["corpus"]).open() as stream:
        body_bytes = max(len(json.dumps(json.loads(line)).encode()) for line in islice(stream, 10))
    probe_bytes = max(len(json.dumps(probe).encode()) for probe in probes) + 2 * body_bytes
    budget = json.loads(Path(manifest["allocation"], "g-budget.json").read_text())
    seed_wall_ns = max(seed_outcome["elapsed_ns"], budget.get("cell_used_ns", {}).get("g1-n100000", 0))
    return {"seed_wall_ns": seed_wall_ns,
            "seed_bytes": sum(path.stat().st_size for path in Path(seed_outcome["attempt"]).rglob("*") if path.is_file()),
            "seed_write_ns": seed_report["write_ns"], "seed_rows": seed_report["natural_records"],
            "route_wall_ns": selected["wall_ns"], "route_requests": selected["requests"],
            "open_ns": selected["open_ns"], "close_ns": max(closes), "text_max_ns": max(texts),
            "point_probe_max_ns": max(probe["point_probe"]["elapsed_ns"] for probe in probes),
            "text_probe_max_ns": max(probe["text_probe"]["elapsed_ns"] for probe in probes),
            "readiness_max_ns": max(batch["adjacent_readiness"]["elapsed_ns"] for batch in batches),
            "write_row_max_ns": max((batch["write_ns"] + batch["rows"] - 1) // batch["rows"] for batch in batches),
            "raw_bytes_per_record": max(raw_per_record, probe_bytes)}

def run_prepared(manifest, root):
    """Run the prepared G protocol; frozen entry validates identities before this layer."""
    from g_campaign import run_g_plan
    root = Path(root)
    root.mkdir(parents=True, exist_ok=True)
    path = Path(manifest["_prepared_manifest"]) if manifest.get("_prepared_manifest") else root / f"prepared-{uuid.uuid4().hex}.json"
    spec = json.loads(Path(__file__).with_name("g-cells.json").read_text())
    if manifest.get("preparation") == "closed-copy-v1":
        spec = dict(manifest["spec"], closed_mode=manifest["stage"])
        for key in ("_reviewed_costs", "_calibration_origin", "_closed_limits"):
            if key in manifest:
                spec[key] = manifest[key]
    def command(_cell, deadline):
        if not path.exists():
            path.write_text(json.dumps(manifest, indent=2) + "\n")
            path.chmod(0o444)
        if manifest.get("schema") != "slice30-g-campaign/v1":
            # Callable protocol fixtures exercise the helper without a production CLI bypass.
            code = (f"import sys;sys.path.insert(0,{str(Path(__file__).resolve().parent)!r});"
                    "from g_runner import run_cell_job;sys.exit(not run_cell_job(sys.argv[1],int(sys.argv[2]),sys.argv[3],sys.argv[4]))")
            return [sys.executable, "-c", code, str(path), str(deadline)]
        return [sys.executable, str(Path(__file__).resolve()), "_cell", str(path), str(deadline)]
    cache = {}
    predictions = {}
    def assess(cell, previous, remaining_ns, retained_bytes):
        if cell["stage"] != "G0" and qualified_result(previous.get("g0-smoke"), cache) is None:
            return {"status": "not_run_prerequisite", "reason": "g0_not_qualified"}
        if manifest.get("preparation") == "closed-copy-v1" and not cell.get("calibration"):
            prediction = manifest.get("_closed_predictions", {}).get(cell["id"])
            if prediction is None:
                return {"status": "not_run_prerequisite", "reason": "qualified_calibration_costs_required"}
            if not prediction.get("allowed"):
                return {"status": "not_run_cap", "reason": "cap_feasibility", "prediction": prediction}
            if (prediction.get("new_bytes", 0) + retained_bytes > cell["active_bytes"]
                    or prediction["total_ns"] > remaining_ns):
                return {"status": "not_run_cap", "reason": "current_remaining_feasibility", "prediction": prediction}
            predictions[cell["id"]] = prediction
            return None
        if cell["stage"] == "G-W" and not cell.get("calibration"):
            from g_forecast import forecast_write
            observed = observed_write_costs(cell, previous, manifest, cache)
            prediction = forecast_write(cell, observed, remaining_ns, retained_bytes)
            if not prediction["allowed"]:
                return {"status": "not_run_cap", "reason": "cap_feasibility", "prediction": prediction}
            predictions[cell["id"]] = prediction
        return None
    def prepare(cell, _root):
        prediction = predictions.get(cell["id"])
        context = {"prospective_prediction": prediction} if prediction is not None else {}
        if manifest.get("schema") == "slice30-g-campaign/v1":
            context["prepared_manifest"] = str(path)
        return context or None
    return run_g_plan(spec, root / "results", command, prepare=prepare,
                      allocation_root=manifest["allocation"], assess=assess, reused=manifest.get("reused"),
                      stage=manifest.get("stage"), prior_outcome_path=manifest.get("prior_outcome_path"))


def run_cell_job(manifest_path, deadline_ns, config_path, output):
    """Execute one foreground job from the fixed prepared G schedule."""
    manifest = json.loads(Path(manifest_path).read_text())
    cell = json.loads(Path(config_path).read_text())
    root = Path(manifest_path).parent
    state_path = root / "results" / "campaign-state.json"
    if cell.get("calibration"):
        state_path = root / "results" / "calibration" / "campaign-state.json"
    previous = {row["id"]: row for row in json.loads(state_path.read_text())} if state_path.exists() else {}
    g0 = previous.get("g0-smoke")
    if cell["stage"] != "G0" and (not g0 or g0.get("status") not in ("complete", "reused_complete")):
        Path(output, "result.json").write_text(json.dumps({"complete":False,"qualification_issues":["g0_not_qualified"]}) + "\n")
        return False
    manifest["command"] = manifest["commands"][cell["artifact"]]
    manifest["subjects"] = {}
    if not cell.get("fresh_copy"):
        for identifier in ["g1-n10000", "g1-n100000"]:
            prior = previous.get(identifier)
            if prior and prior.get("status") in ("complete", "reused_complete"):
                report = json.loads(Path(prior["attempt"], "result.json").read_text())
                manifest["subjects"][identifier] = str(Path(report["steps"][0]["output"]) / "store.sqlite")
    if manifest.get("gpu"):
        from g_gpu import GpuObserver
        observer = GpuObserver(manifest["gpu"])
        manifest["_observe_native"] = observer.sample
    result = execute_cell(cell, manifest, output, deadline_ns)
    return result["complete"]



def run_frozen(root):
    """Run only the frozen clean candidate, charging preflight to the same shared allocation."""
    from g_freeze import git, validate_frozen
    from g_gpu import inspect_gpu
    from g_boundary import require_known_containment
    require_known_containment(os.environ["S30_TASK"])
    root = Path(root).resolve()
    if json.loads((root / "manifest.json").read_text()).get("preparation") == "closed-copy-v1":
        from g_closed_entry import run_entry
        return run_entry(root)
    preliminary = json.loads((root / "manifest.json").read_text())
    if preliminary.get("schema") != "slice30-g-campaign/v1" or preliminary["source_sha"] != git("rev-parse", "HEAD") or git("status", "--porcelain"):
        raise ValueError("run requires the exact clean frozen G source")
    allocation = Path(preliminary["allocation"])
    if not root.is_relative_to(allocation):
        raise ValueError("frozen G root is outside the shared allocation")
    with (allocation / "g-campaign.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        budget_path = allocation / "g-budget.json"
        budget = json.loads(budget_path.read_text()) if budget_path.exists() else {"schema":"slice30-gd-budget/v1","used_ns":0,"inflight":None,"cell_used_ns":{}}
        def prepared(manifest):
            return dict(manifest, corpus=manifest["inputs"]["corpus"]["path"], queries=manifest["inputs"]["queries"]["path"],
                        commands={key:[value["path"]] for key,value in manifest["binaries"].items()})
        state_path = root / "results/campaign-state.json"
        previous = dict(preliminary.get("reused", {}))
        if state_path.exists():
            previous.update({row["id"]: row for row in json.loads(state_path.read_text())})
        from g_campaign import completed_record, stage_target
        target = stage_target(preliminary.get("stage"))
        if target and completed_record(previous.get(target)):
            validate_frozen(root)
            return run_prepared(prepared(preliminary), root)
        if all(previous.get(cell["id"], {}).get("status") in ("complete", "reused_complete")
               and previous[cell["id"]].get("cell") == cell for cell in preliminary["plan"]):
            validate_frozen(root)
            return run_prepared(prepared(preliminary), root)
        if budget.get("inflight") or budget["used_ns"] >= 14400 * 10**9:
            return run_prepared(prepared(preliminary), root)
        started = time.monotonic_ns()
        invocation = root / "invocations" / f"run-{uuid.uuid4().hex}"
        invocation.mkdir(parents=True)
        def publish_invocation(ok, error=None):
            rows_path = root / "results/g-outcomes.json"
            rows = json.loads(rows_path.read_text()) if rows_path.exists() else []
            selected = next((row for row in rows if row["id"] == target), None)
            value = {"path":str(invocation),"environment_ok":ok,"error":error,"finished_at_ns":time.time_ns(),
                     "stage": preliminary.get("stage"),
                     "stage_complete": bool(ok and target and completed_record(selected)),
                     "campaign_complete": bool(ok and len(rows) == 84 and all(completed_record(row) for row in rows))}
            (invocation / "invocation.json").write_text(json.dumps(value,indent=2)+"\n")
            (root / "last-run.json").write_text(json.dumps(value,indent=2)+"\n")
        budget["inflight"] = {"cell":"g-preflight","started_at_ns":time.time_ns()}
        budget_path.write_text(json.dumps(budget, indent=2) + "\n")
        pending = target or next((cell["id"] for cell in preliminary["plan"] if previous.get(cell["id"],{}).get("status") not in ("complete", "reused_complete")), None)
        failure = None
        try:
            manifest = validate_frozen(root)
            before = inspect_gpu(manifest["gpu"]["uuid"])
            if before["applications"]:
                raise RuntimeError("selected3090 reservation is not quiet")
            (invocation / "environment-before.json").write_text(json.dumps(before, indent=2) + "\n")
            os.environ["FATHOMDB_EMBED_DEVICE"] = manifest["device_policy"]
            os.environ["HF_HUB_OFFLINE"] = "1"
            os.environ["TRANSFORMERS_OFFLINE"] = "1"
        except Exception as error:
            failure = f"{type(error).__name__}: {error}"
            (invocation / "preflight-failure.log").write_text(traceback.format_exc())
        finally:
            elapsed = time.monotonic_ns() - started
            budget["used_ns"] += elapsed
            if pending:
                usage = budget.setdefault("cell_used_ns", {})
                usage[pending] = usage.get(pending,0) + elapsed
            budget["inflight"] = None
            budget_path.write_text(json.dumps(budget, indent=2) + "\n")
        if failure:
            outcomes = [previous[cell["id"]] if previous.get(cell["id"],{}).get("status") in ("complete", "reused_complete") else
                        {"id":cell["id"],"cell":cell,"status":"not_run_prerequisite","complete":False,"reason":"frozen_preflight_failed","error":failure}
                        for cell in preliminary["plan"]]
            (root / "results").mkdir(exist_ok=True)
            (root / "results/g-outcomes.json").write_text(json.dumps(outcomes, indent=2) + "\n")
            publish_invocation(False, failure)
            return outcomes
        outcomes = run_prepared(prepared(manifest), root)
        try:
            after = inspect_gpu(manifest["gpu"]["uuid"])
            (invocation / "environment-after.json").write_text(json.dumps(after, indent=2) + "\n")
            if after["applications"]:
                raise RuntimeError("selected GPU compute work remains after campaign")
        except Exception as error:
            (invocation / "environment-after-failure.log").write_text(traceback.format_exc())
            current = next((row for row in reversed(outcomes) if row.get("status") == "complete"
                            and row.get("attempt") != previous.get(row["id"],{}).get("attempt")), None)
            if current is not None:
                current.update(status="failed",complete=False,error="postflight_environment_unavailable")
                state = {row["id"]:row for row in json.loads(state_path.read_text())}
                state[current["id"]] = current
                state_path.write_text(json.dumps(list(state.values()),indent=2)+"\n")
                Path(current["attempt"],"controller-postflight.json").write_text(json.dumps(current,indent=2)+"\n")
                (root / "results/g-outcomes.json").write_text(json.dumps(outcomes,indent=2)+"\n")
            publish_invocation(False, f"{type(error).__name__}: {error}")
        else:
            publish_invocation(True)
        return outcomes


def main():
    """Build, freeze, run and report the exact bounded G campaign without implicit opt-in."""
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="action", required=True)
    build = commands.add_parser("build")
    build.add_argument("--out", type=Path, required=True)
    freeze_parser = commands.add_parser("freeze")
    for field in ("corpus", "queries", "license", "ordinary", "diagnostic", "build_receipt", "model_cache", "allocation", "out"):
        freeze_parser.add_argument("--"+field.replace("_","-"), dest=field, type=Path, required=True)
    from g_campaign import STAGES
    freeze_parser.add_argument("--stage", choices=tuple(STAGES))
    freeze_parser.add_argument("--reuse-completed-from", type=Path)
    freeze_parser.add_argument("--preparation", choices=("closed-copy-v1",))
    freeze_parser.add_argument("--calibration-from", type=Path)
    freeze_parser.add_argument("--cost-review-from", type=Path)
    freeze_parser.add_argument("--gpu-uuid", required=True)
    freeze_parser.add_argument("--gpu-ordinal", type=int, required=True)
    run = commands.add_parser("run")
    run.add_argument("root", type=Path)
    report = commands.add_parser("report")
    report.add_argument("root", type=Path)
    job = commands.add_parser("_cell")
    job.add_argument("manifest", type=Path)
    job.add_argument("deadline", type=int)
    job.add_argument("config", type=Path)
    job.add_argument("output", type=Path)
    common = commands.add_parser("_common")
    common.add_argument("job", type=Path)
    common.add_argument("config", type=Path)
    common.add_argument("output", type=Path)
    args = parser.parse_args()
    if args.action == "build":
        from g_freeze import build_tools
        build_tools(args.out)
    elif args.action == "freeze":
        from g_freeze import freeze
        if args.preparation == "closed-copy-v1":
            from g_closed_entry import freeze_entry
            manifest = freeze_entry(vars(args))
        else:
            manifest = freeze(vars(args))
        print(json.dumps({"campaign_uuid":manifest["campaign_uuid"],"source_sha":manifest["source_sha"],"root":manifest["root"]}))
    elif args.action == "run":
        if os.environ.get("AGENT_LONG") != "1":
            parser.error("G production campaign requires AGENT_LONG=1")
        from g_campaign import completed_record
        outcomes = run_frozen(args.root)
        print(json.dumps({"cells":len(outcomes),"complete":all(completed_record(row) for row in outcomes)}))
        if any(not completed_record(row) for row in outcomes) or not json.loads((args.root / "last-run.json").read_text()).get("environment_ok", False):
            raise SystemExit(1)
    elif args.action == "report":
        from g_campaign import active_bytes
        from g_report import summarize
        bootstrap = json.loads((args.root / "manifest.json").read_text())
        if bootstrap.get("preparation") == "closed-copy-v1":
            from g_closed_entry import report_entry
            value = report_entry(args.root)
        else:
            value = summarize(args.root)
            payload = json.dumps(value, indent=2) + "\n"
            allocation = Path(value["manifest"]["allocation"])
            if active_bytes(allocation) + len(payload.encode()) > 16 * 1024**3:
                raise RuntimeError("retained cap cannot hold a new summary; raw obligations remain retained")
            (args.root / "summary-g.json").write_text(payload)
        print(json.dumps({"cells":len(value["cells"]),"complete":value["complete"]}))
        if not value["complete"]:
            raise SystemExit(1)
    elif args.action == "_common":
        from g_closed_entry import common_worker
        common_worker(args.job, args.output)
    else:
        from g_admission import require_cell_admission
        require_cell_admission(args.manifest, args.deadline, args.config, args.output)
        if not run_cell_job(args.manifest, args.deadline, args.config, args.output):
            raise SystemExit(1)


if __name__ == "__main__":
    main()
