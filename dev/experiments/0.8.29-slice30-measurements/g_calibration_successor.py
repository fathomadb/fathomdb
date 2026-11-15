"""One explicit successor for the completed-native closed calibration policy defect."""

import copy
import json
from pathlib import Path

from g_closed_guard import write_budget

MODE = "closed-copy-calibration"
ISSUE = "positive_default_cuda_evidence_missing"


def prepare_successor(root, manifest, predecessor):
    """Validate only the retained auto-versus-cuda:0 defect; never promote its result.

    The caller runs under supervised common preflight with the allocation lock.
    Return the exact original stop/source binding for consumption by the controller.
    Unrelated failures, changed scientific inputs and uncertain cleanup raise ValueError.
    """
    from g_campaign import calibration_plan
    from g_runner import qualify_step

    root, predecessor = Path(root).resolve(), Path(predecessor).resolve()
    allocation = Path(manifest["allocation"]).resolve()
    if (root == predecessor or not predecessor.is_relative_to(allocation)
            or manifest.get("stage") != MODE or manifest.get("preparation") != "closed-copy-v1"
            or manifest.get("root") != str(root)):
        raise ValueError("corrected calibration requires a distinct bound closed calibration root")
    original = json.loads((predecessor / "manifest.json").read_text())
    if (original.get("root") != str(predecessor) or original.get("stage") != MODE
            or original.get("source_sha") == manifest.get("source_sha")):
        raise ValueError("corrected calibration requires the original stopped source and repaired source")
    for key in ("schema", "compiler_inputs", "lock", "inputs", "model_files", "binaries", "build_receipt",
                "python_receipt", "python_native", "allocation", "device_policy", "format", "environment",
                "closed_subject", "preparation", "spec", "plan", "closed_limits", "reused"):
        if original.get(key) != manifest.get(key):
            raise ValueError(f"corrected calibration input/native equivalence differs: {key}")
    for key in ("uuid", "ordinal", "name", "driver_version", "primary_cuda"):
        if original["gpu"].get(key) != manifest["gpu"].get(key):
            raise ValueError(f"corrected calibration device differs: {key}")
    budget = json.loads((allocation / "g-budget.json").read_text())
    active = budget.get("inflight") or {}
    if (budget.get("closed_calibration_successor") is not None
            or active and (active.get("cell") != "g-common-"+MODE
                           or active.get("status", "running") != "running")
            or (budget.get("closed_guardian") or {}).get("status") == "cleanup_uncertain"):
        raise ValueError("corrected calibration admission consumed or cleanup is uncertain")
    stop = budget.get("closed_calibration_stop") or {}
    cells = calibration_plan(manifest["spec"])
    cell = cells[0]
    path = predecessor / "results/calibration/g-outcomes.json"
    rows = json.loads(path.read_text())
    outcome = stop.get("outcome") or {}
    attempt = Path(outcome.get("attempt", "")).resolve()
    if (stop.get("cell") != cell["id"] or stop.get("path") != str(path)
            or [row.get("id") for row in rows] != [item["id"] for item in cells]
            or rows[0] != outcome or outcome.get("cell") != cell or outcome.get("id") != cell["id"]
            or outcome.get("status") != "failed" or outcome.get("error") is not None
            or outcome.get("exit") != 1 or outcome.get("child_exit") != 1
            or outcome.get("cleanup_uncertain") is not False or outcome.get("live_owned_pids") != []
            or attempt.parent != path.parent / cell["id"] or not attempt.name.startswith("attempt-")
            or any(row.get("attempt") or row.get("status") != "not_run_prerequisite"
                   or row.get("reason") != "calibration_stopped" for row in rows[1:])):
        raise ValueError("original stop is not the known-cleanup first policy-defect calibration")
    actual = json.loads((attempt / "result.json").read_text())
    if (actual.get("complete") is not False or actual.get("qualification_issues") != [ISSUE]
            or actual.get("preparation", {}).get("validated") is not True or len(actual.get("steps", [])) != 1):
        raise ValueError("original calibration did not fail only the completed-native device policy")
    step = actual["steps"][0]
    native = step.get("result") or {}
    report = native.get("open_report") or {}
    device = report.get("embedder_device_resolution") or {}
    effective = device.get("effective_device") or {}
    gpu = original["gpu"]
    if (step.get("family") != "G-W" or step.get("complete") is not True or step.get("exit") != 0
            or native.get("complete") is not True or native.get("warmup_count") != 500
            or native.get("sample_count") != 1000 or native.get("first_call_count") != 0
            or original.get("device_policy") != "cuda:0" or gpu.get("ordinal") != 0
            or device.get("requested_policy") != "auto" or device.get("cuda_compiled") is not True
            or device.get("selected_cuda_uuid") != gpu["uuid"] or effective.get("kind") != "cuda"
            or effective.get("uuid") != gpu["uuid"] or effective.get("ordinal") != 0
            or "3090" not in (effective.get("name") or "") or "K620" in (effective.get("name") or "")
            or step.get("executable") != original["binaries"]["ordinary"]["path"]
            or qualify_step(step, original) != [ISSUE]
            or not any(sample.get("pid") == step.get("pid") and sample.get("pid_memory_mib", 0) > 0
                       and sample.get("gpu_memory_mib", 0) > sample.get("baseline_gpu_memory_mib", 0)
                       and sample.get("utilization_percent", 0) > 0 for sample in step.get("gpu_samples", []))):
        raise ValueError("original native workload is not exclusively the requested-auto policy defect")
    invocation = json.loads((predecessor / "last-run.json").read_text())
    receipt = json.loads(Path(invocation["guardian_receipt"]).read_text())
    boundary = receipt.get("boundary") or {}
    if (invocation.get("environment_ok") is not True or invocation.get("stage_complete") is not False
            or receipt.get("exit") != 1 or receipt.get("resource_stop") is not None
            or receipt.get("cleanup_uncertain") is not False or receipt.get("live_owned_pids") != []
            or receipt.get("final_sample_error") is not None
            or receipt.get("sha_start") != original["source_sha"] or receipt.get("sha_end") != original["source_sha"]
            or receipt.get("delta_start") != "" or receipt.get("delta_end") != ""
            or boundary.get("status") != "quiescent" or boundary.get("populated_after") is not False
            or boundary.get("removed_exact_empty_boundary") is not True):
        raise ValueError("original strict source/guardian cleanup is incomplete")
    return dict(from_root=str(predecessor), from_source_sha=original["source_sha"], root=str(root),
                source_sha=manifest["source_sha"], original_stop=copy.deepcopy(stop))


def consume_successor(prepared, root):
    """Consume the one allocation-wide successor before controller/payload dispatch.

    Recheck the supervised binding under the caller's allocation lock and publish
    atomically. Refusal or a crash after this write cannot renew the admission.
    """
    binding = prepared.pop("_calibration_successor_candidate")
    path = Path(prepared["allocation"]) / "g-budget.json"
    budget = json.loads(path.read_text())
    if (binding.get("root") != str(Path(root).resolve()) or binding.get("source_sha") != prepared["source_sha"]
            or prepared.get("stage") != MODE or budget.get("closed_calibration_successor") is not None
            or budget.get("closed_calibration_stop") != binding.get("original_stop") or budget.get("inflight")
            or (budget.get("closed_guardian") or {}).get("status") == "cleanup_uncertain"):
        raise ValueError("corrected calibration stop/binding changed or admission is consumed")
    budget["closed_calibration_successor"] = binding
    write_budget(path, budget)
    return copy.deepcopy(binding)
