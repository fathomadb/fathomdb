"""Reference the accepted G records in place without changing their measurement subject."""

import ast
import json
from pathlib import Path
import subprocess

BASE_SOURCE = "60f471aaf042203d07a15025662ebf572f6cc242"
BASE_UUID = "5e89e1dc-8fbd-4848-bc2a-8dc84e478f16"
BASE_CELLS = {"g0-smoke", "g1-n10000", "g-r-n10000-r0", "g-r-n10000-r1"}


def collect_reuse(current, original_root):
    """Validate equivalent accepted subjects and return original-path references or refuse."""
    from g_freeze import SOURCE
    from g_runner import qualify_step

    root = Path(original_root).resolve()
    original = json.loads((root / "manifest.json").read_text())
    if original.get("root") != str(root):
        raise ValueError("reuse root does not name its original manifest")
    for key in ("schema", "compiler_inputs", "lock", "spec", "plan", "inputs", "model_files",
                "binaries", "build_receipt", "python_receipt", "python_native", "allocation",
                "device_policy", "format", "environment"):
        if original.get(key) != current.get(key):
            raise ValueError(f"reuse measurement equivalence mismatch: {key}")
    for key in ("uuid", "ordinal", "name", "driver_version", "primary_cuda"):
        if original["gpu"].get(key) != current["gpu"].get(key):
            raise ValueError(f"reuse device equivalence mismatch: {key}")
    # The native fingerprint covers measured code; preserve the actual qualifier as well.
    relative = "dev/experiments/0.8.29-slice30-measurements/g_runner.py"
    old_source = subprocess.check_output(["git", "show", f"{original['source_sha']}:{relative}"], cwd=SOURCE).decode()
    def qualifier(source):
        return [ast.dump(node, include_attributes=False) for node in ast.parse(source).body
                if isinstance(node, ast.FunctionDef) and node.name in ("qualify_step", "execute_cell")]
    if qualifier(old_source) != qualifier((SOURCE / relative).read_text()):
        raise ValueError("reuse qualification semantics differ from original source")
    selected = set(BASE_CELLS)
    inherited = {}
    if (original.get("source_sha"), original.get("campaign_uuid")) != (BASE_SOURCE, BASE_UUID):
        if original.get("stage") != "branch-controls" or not original.get("reuse_completed_from"):
            raise ValueError("reuse source is not the accepted base or its qualified branch stage")
        inherited = collect_reuse(current, original["reuse_completed_from"])
        selected = {"g-o-branches"}
    rows = {row["id"]: row for row in json.loads((root / "results/g-outcomes.json").read_text())}
    invocation = json.loads((root / "last-run.json").read_text())
    if invocation.get("environment_ok") is not True:
        raise ValueError("reuse original actor/environment cleanup is not qualified")
    references = dict(inherited)
    for identifier in selected:
        row = rows.get(identifier) or {}
        cell = next(item for item in current["plan"] if item["id"] == identifier)
        attempt = Path(row.get("attempt", "")).resolve()
        if (row.get("status") != "complete" or row.get("cell") != cell or row.get("exit") != 0
                or row.get("cleanup_uncertain") is not False or row.get("live_owned_pids") != []
                or not attempt.is_relative_to(root / "results")):
            raise ValueError(f"reuse original outcome is incomplete: {identifier}")
        result = json.loads((attempt / "result.json").read_text())
        if (result.get("complete") is not True or result.get("qualification_issues") != []
                or not result.get("steps")):
            raise ValueError(f"reuse original qualification is incomplete: {identifier}")
        for step in result["steps"]:
            if qualify_step(step, original):
                raise ValueError(f"reuse original route evidence is incomplete: {identifier}")
            for close in (step.get("result") or {}).get("close_outcomes", []):
                if close.get("status") != "ok":
                    raise ValueError(f"reuse original close failed: {identifier}")
        if not any(sample.get("pid") == step.get("pid") and sample.get("pid_memory_mib", 0) > 0
                   and sample.get("gpu_memory_mib", 0) > sample.get("baseline_gpu_memory_mib", 0)
                   and sample.get("utilization_percent", 0) > 0
                   for step in result["steps"] for sample in step.get("gpu_samples", [])):
            raise ValueError(f"reuse original GPU engagement is incomplete: {identifier}")
        references[identifier] = dict(row, status="reused_complete", origin={
            "root": str(root), "source_sha": original["source_sha"],
            "campaign_uuid": original["campaign_uuid"],
            "outcome_path": str(root / "results/g-outcomes.json")})
    return references
