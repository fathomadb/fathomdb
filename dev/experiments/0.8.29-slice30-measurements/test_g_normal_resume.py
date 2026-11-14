"""Normal-entry protocol and real scheduling controls; no native/model/GPU evidence."""

from contextlib import contextmanager, ExitStack
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import g_freeze
from g_campaign import make_g_plan
from g_reuse import BASE_CELLS, BASE_SOURCE, BASE_UUID
from g_runner import qualify_step

SPEC = json.loads(Path(__file__).with_name("g-cells.json").read_text())
SIX = BASE_CELLS | {"g-o-branches", "g1-n100000"}
CHARGES = {"g0-smoke": 13957850996, "g1-n10000": 65331797247,
           "g-r-n10000-r0": 98923117167, "g-r-n10000-r1": 164160351743,
           "g-o-branches": 29701336420, "g1-n100000": 657935962544}


def dump(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value) + "\n")


@contextmanager
def accepted_protocol_chain(root):
    """Declare synthetic typed entry metadata; actual qualification still requires real pinned inputs."""
    source = root / "source"
    runner_source = Path(__file__).with_name("g_runner.py").read_bytes()
    runner = source / "dev/experiments/0.8.29-slice30-measurements/g_runner.py"
    runner.parent.mkdir(parents=True)
    runner.write_bytes(runner_source)
    (source / "Cargo.lock").write_text("protocol lock\n")
    inputs = {}
    for key, count in (("corpus", 100000), ("queries", 500), ("license", 1)):
        path = root / key
        path.write_text('"protocol-only"\n' * count)
        inputs[key] = g_freeze.identify(path)
    hf = root / "hf"
    cache = hf / "hub/models--BAAI--bge-small-en-v1.5/snapshots" / SPEC["model"]["revision"]
    cache.mkdir(parents=True)
    model = {}
    for name in g_freeze.MODEL_FILES:
        (cache / name).write_bytes(b"protocol bytes, not a model")
        model[name] = g_freeze.identify(cache / name)
    binaries = {}
    for variant in g_freeze.FEATURES:
        path = root / variant
        path.write_bytes(b"protocol bytes, not an executable")
        binaries[variant] = g_freeze.identify(path)
    build_path = root / "build.json"
    dump(build_path, {"schema": "slice30-g-build/v1", "compiler_inputs": "protocol-inputs",
        "primary_cuda": "/usr/local/cuda-12.6", "binaries": {
            key: dict(value, features=g_freeze.FEATURES[key]) for key, value in binaries.items()}})
    module = source / ".venv/protocol-native"
    module.parent.mkdir()
    module.write_bytes(b"protocol bytes, never imported")
    native = g_freeze.identify(module)
    receipt = source / ".cache/agent/python-native.json"
    dump(receipt, {"source_sha": "protocol-native-source", "source_delta_sha256": hashlib.sha256(b"").hexdigest(),
                   "native": {"module": native["path"], "sha256": native["sha256"]}})
    gpu = {"uuid": "GPU-protocol-only", "name": "RTX 3090 protocol record", "ordinal": 0,
           "driver_version": "580.178.04", "primary_cuda": "/usr/local/cuda-12.6", "applications": []}
    environment = {"HF_HOME": str(hf), "XDG_CACHE_HOME": str(root / "xdg"),
                   "CUDA_CACHE_PATH": str(root / "cuda"), "CUDA_VISIBLE_DEVICES": gpu["uuid"]}
    common = {"schema": "slice30-g-campaign/v1", "compiler_inputs": "protocol-inputs",
        "lock": g_freeze.identify(source / "Cargo.lock"), "spec": SPEC, "plan": make_g_plan(SPEC),
        "inputs": inputs, "model_files": model, "binaries": binaries,
        "build_receipt": g_freeze.identify(build_path), "python_receipt": g_freeze.identify(receipt),
        "python_native": native, "allocation": str(root), "gpu": gpu,
        "environment": environment, "device_policy": "cuda:0", "format": 37}
    references = {}
    previous = None
    for label, selected, stage in (("base", BASE_CELLS, None), ("branch", {"g-o-branches"}, "branch-controls"),
                                   ("seed", {"g1-n100000"}, "first-100k-seed-exploratory")):
        original = root / label
        manifest = dict(common, root=str(original), source_sha=BASE_SOURCE if stage is None else "protocol-staged-source",
                        campaign_uuid=BASE_UUID if stage is None else label + "-protocol-uuid")
        if stage:
            manifest.update(stage=stage, reuse_completed_from=str(previous))
        dump(original / "manifest.json", manifest)
        dump(original / "last-run.json", {"environment_ok": True, "stage_complete": True})
        rows = []
        for cell in common["plan"]:
            if cell["id"] not in selected:
                continue
            attempt = original / "results" / cell["id"] / "attempt-0001"
            family = "G-R" if cell["kind"] in ("read", "smoke") else "G-seed"
            config = dict(cell, family=family)
            config_path = attempt / "native-config.json"
            dump(config_path, config)
            report = {"pid": 70001, "executable": binaries[cell["artifact"]]["path"], "available": True,
                "requested_provider": "default", "schema_version_after": 37,
                "embedder_identity": {"name": "fathomdb-bge-small-en-v1.5", "revision": SPEC["model"]["revision"], "dimension": 384},
                "embedder_device_resolution": {"requested_policy": "cuda:0", "cuda_compiled": True,
                    "selected_cuda_uuid": gpu["uuid"], "effective_device": {
                        "kind": "cuda", "uuid": gpu["uuid"], "ordinal": 0, "name": gpu["name"]}}}
            fixture = {"qualified_dense_fixture": True, "format": 37, "natural_records": cell["n"],
                "vector_rows": cell["n"], "physical_vector_rows": cell["n"], "span_rows": cell["n"],
                "pending_count": 0, "generation_failed": 0}
            result = {"complete": True, "open_reports": [report, report], "fixtures": [fixture],
                      "close_outcomes": [{"status": "ok"}]}
            if family == "G-R":
                scale = len(cell["adapters"]) * len(cell["routes"])
                result.update(sample_count=cell["samples"] * scale, warmup_count=cell["warmup"] * scale,
                              first_call_count=int(cell["record_first_call"]) * scale)
            step = {"family": family, "config": str(config_path), "complete": True, "result": result,
                "pid": 70001, "executable": report["executable"], "gpu_samples": [{"pid": 70001,
                    "pid_memory_mib": 100, "gpu_memory_mib": 102, "baseline_gpu_memory_mib": 2, "utilization_percent": 20}]}
            assert qualify_step(step, manifest) == [], "invalid typed protocol fixture, not a semantic RED"
            dump(attempt / "result.json", {"complete": True, "qualification_issues": [], "steps": [step]})
            row = {"id": cell["id"], "cell": cell, "status": "complete", "attempt": str(attempt),
                "exit": 0, "cleanup_uncertain": False, "live_owned_pids": [],
                "elapsed_ns": 657095755807 if cell["id"] == "g1-n100000" else 65326146421}
            rows.append(row)
            references[cell["id"]] = dict(row, status="reused_complete", origin={"root": str(original),
                "source_sha": manifest["source_sha"], "campaign_uuid": manifest["campaign_uuid"],
                "outcome_path": str(original / "results/g-outcomes.json")})
        dump(original / "results/g-outcomes.json", rows)
        previous = original
    options = {key: value["path"] for key, value in inputs.items()}
    options.update({key: value["path"] for key, value in binaries.items()})
    options.update(build_receipt=str(build_path), model_cache=str(cache), allocation=str(root),
                   out=str(root / "normal"), gpu_uuid=gpu["uuid"], gpu_ordinal=0, reuse_completed_from=str(previous))
    with ExitStack() as stack:
        # Entry metadata is synthetic; file identity/qualifier logic itself still executes.
        stack.enter_context(patch.dict(os.environ, environment))
        stack.enter_context(patch.object(g_freeze, "SOURCE", source))
        stack.enter_context(patch.object(g_freeze, "PINS", {key: value["sha256"] for key, value in inputs.items()}))
        stack.enter_context(patch.object(g_freeze, "MODEL_FILES", {key: (value["bytes"], value["sha256"]) for key, value in model.items()}))
        stack.enter_context(patch.object(g_freeze, "compiler_inputs", return_value="protocol-inputs"))
        stack.enter_context(patch.object(g_freeze, "git", side_effect=lambda *args: "" if args[0] == "status" else "protocol-candidate-source"))
        stack.enter_context(patch("g_gpu.inspect_gpu", return_value=gpu))
        stack.enter_context(patch("g_reuse.subprocess.check_output", return_value=runner_source))
        yield options, references


class GNormalReferenceEntryTests(unittest.TestCase):
    def test_normal_freeze_accepts_six_original_references_without_a_stage(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with accepted_protocol_chain(root) as (options, expected):
                try:
                    actual = g_freeze.freeze(options)
                except ValueError as error:
                    self.fail(f"normal freeze refused valid accepted seed-reference chain: {error}")
                self.assertNotIn("stage", actual)
                self.assertEqual(len(actual["plan"]), 84)
                self.assertEqual(set(actual["reused"]), SIX)
                self.assertEqual(actual["reused"], expected)
                self.assertEqual(json.loads((root / "normal/manifest.json").read_text()), actual)
