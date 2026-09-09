#!/usr/bin/env python3
"""Fail-closed unit tests for the Slice 72 CE profile validator."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "scripts" / "release" / "verify-slice72-ce-profile.py"
SPEC = importlib.util.spec_from_file_location("verify_slice72_ce_profile", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

BASELINE = "1" * 40
CANDIDATE = "2" * 40
HASH = "a" * 64


def manifest() -> dict:
    return {
        "schema_version": "fathomdb.slice72.ce-profile-manifest/v1",
        "baseline_sha": BASELINE,
        "candidate_sha": CANDIDATE,
        "fixture": {
            "query": "How many people live in Berlin?",
            "passages": [
                {"id": 0, "body": "Berlin art", "score": 0.5},
                {"id": 1, "body": "Berlin population", "score": 0.49},
            ],
            "passage_ids": [0, 1],
            "standalone_order": [1, 0],
            "engine_ids": [0, 1],
            "engine_ties": [],
            "rerank_depth": 2,
            "pool_n": 2,
            "alpha": 1.0,
        },
        "features": {
            "cpu": ["pyo3/extension-module", "default-reranker"],
            "cuda": ["pyo3/extension-module", "rerank-cuda"],
        },
        "cuda": {
            "selected_uuid": "GPU-test",
            "visible_selector": "GPU-test",
            "model": "NVIDIA GeForce RTX 3090",
        },
        "model_sha256": {
            "config.json": HASH,
            "tokenizer.json": HASH,
            "model.safetensors": HASH,
        },
        "cold_repetitions": 3,
        "steady_repetitions": 5,
        "steady_calls": 20,
        "score_tolerance": 0.01,
        "p95_regression_ratio": 1.10,
        "environment": {
            "OMP_NUM_THREADS": "1",
            "OPENBLAS_NUM_THREADS": "1",
            "MKL_NUM_THREADS": "1",
            "RAYON_NUM_THREADS": "1",
        },
        "cpu_affinity": [0],
        "timer": "time.perf_counter_ns/nearest-rank",
    }


def output(path: str, device: str) -> dict:
    ids = [1, 0] if path == "standalone" else [0, 1]
    shift = 0.005 if device == "cuda" else 0.0
    return {"ids": ids, "ce_scores": {"0": 0.2 + shift, "1": 0.8 + shift}}


def artifact(role: str, device: str) -> dict:
    current_manifest = manifest()
    root = f"/tmp/slice72/{role}-{device}"
    wheel = f"{root}/wheel/fathomdb.whl"
    venv = f"{root}/venv"
    python = f"{venv}/bin/python"
    native = f"{venv}/lib/fathomdb/_fathomdb.so"
    features = current_manifest["features"][device]
    return {
        "schema_version": "fathomdb.slice72.ce-artifact/v1",
        "role": role,
        "device": device,
        "commit_sha": BASELINE if role == "baseline" else CANDIDATE,
        "features": features,
        "source_root": f"/tmp/source/{role}",
        "source_clean": True,
        "build_command": [
            "maturin", "build", "--release", "--out", f"{root}/wheel",
            "--features", ",".join(features), "-i", "/usr/bin/python3",
        ],
        "build_environment": {}
        if device == "cpu"
        else {"CUDA_HOME": "/usr/local/cuda", "PATH_prefix": "/usr/local/cuda/bin"},
        "wheel_path": wheel,
        "wheel_size": 1024,
        "wheel_sha256": HASH,
        "wheel_native_member": "fathomdb/_fathomdb.so",
        "wheel_native_sha256": HASH,
        "venv_root": venv,
        "python_path": python,
        "install_command": [
            python, "-m", "pip", "install", "--no-index", "--no-deps", wheel,
        ],
        "module_path": f"{venv}/lib/fathomdb/__init__.py",
        "native_path": native,
        "native_sha256": HASH,
    }


def runtime(role: str, device: str, pid: int) -> dict:
    current_artifact = artifact(role, device)
    uuid = None if device == "cpu" else "GPU-test"
    return {
        "pid": pid,
        "install_root": current_artifact["venv_root"],
        "module_path": current_artifact["module_path"],
        "native_path": current_artifact["native_path"],
        "native_sha256": HASH,
        "source_imported": False,
        "effective_device": "cpu" if device == "cpu" else "cuda:0",
        "selected_uuid": uuid,
        "allocation": None if device == "cpu" else {
            "pid": pid, "gpu_uuid": uuid, "vram_mib": 128,
        },
        "import_ns": 10,
        "open_ns": 20,
        "peak_rss_kib": 1024,
        "affinity": [0],
    }


def cell(role: str, device: str, duration: int = 100) -> dict:
    current_manifest = manifest()
    current_artifact = artifact(role, device)
    process_id = 4000

    def repetition(path: str, mode: str) -> dict:
        nonlocal process_id
        process_id += 1
        current_output = output(path, device)
        record = {
            "output": current_output,
            "output_digests": [MODULE.canonical_digest(current_output)]
            * (1 if mode == "cold" else 20),
            "runtime": runtime(role, device, process_id),
            "raw_sha256": HASH,
        }
        if mode == "cold":
            record["duration_ns"] = duration * 2
        else:
            record["durations_ns"] = [duration] * 20
        return record

    return {
        "schema_version": "fathomdb.slice72.ce-profile-cell/v2",
        "role": role,
        "commit_sha": BASELINE if role == "baseline" else CANDIDATE,
        "device": device,
        "features": current_manifest["features"][device],
        "artifact_receipt_sha256": MODULE.receipt_digest(current_artifact),
        "artifact": current_artifact,
        "network_policy": "offline-loopback-proxy",
        "cuda_inventory": None if device == "cpu" else {
            "uuid": "GPU-test", "model": "NVIDIA GeForce RTX 3090",
        },
        "model_sha256_before": current_manifest["model_sha256"],
        "model_sha256_after": current_manifest["model_sha256"],
        "model_files_read_only": True,
        "paths": {
            path: {
                "cold": [repetition(path, "cold") for _ in range(3)],
                "steady": [repetition(path, "steady") for _ in range(5)],
            }
            for path in ["standalone", "engine"]
        },
    }


def cells(candidate_duration: int = 105) -> list[dict]:
    return [
        cell("baseline", "cpu"),
        cell("baseline", "cuda"),
        cell("candidate", "cpu", candidate_duration),
        cell("candidate", "cuda", candidate_duration),
    ]


class CeProfileValidatorTests(unittest.TestCase):
    def test_nearest_rank_is_exact_and_hand_calculated(self) -> None:
        sample = list(range(1, 21))
        self.assertEqual(MODULE.nearest_rank(sample, 0.50), 10)
        self.assertEqual(MODULE.nearest_rank(sample, 0.95), 19)
        self.assertEqual(MODULE.nearest_rank(sample, 0.99), 20)

    def test_complete_receipt_passes(self) -> None:
        receipt = MODULE.validate_and_aggregate(manifest(), cells())
        self.assertEqual(receipt["verdict"], "PASS")
        self.assertEqual(len(receipt["comparisons"]), 4)
        self.assertEqual(len(receipt["evidence"]), 4)
        self.assertRegex(receipt["manifest_canonical_sha256"], r"^[0-9a-f]{64}$")

    def test_missing_or_unknown_cell_fails(self) -> None:
        with self.assertRaisesRegex(MODULE.ProfileError, "complete cell set"):
            MODULE.validate_and_aggregate(manifest(), cells()[:-1])
        bad = cells()
        bad[0]["unknown"] = True
        with self.assertRaisesRegex(MODULE.ProfileError, "unknown keys"):
            MODULE.validate_and_aggregate(manifest(), bad)

    def test_identity_device_and_allocation_fail_closed(self) -> None:
        for field, value, message in [
            ("source_imported", True, "native/source identity"),
            ("effective_device", "cpu", "device/allocation"),
            ("allocation", None, "device/allocation"),
        ]:
            bad = cells()
            bad[1]["paths"]["engine"]["steady"][4]["runtime"][field] = value
            with self.assertRaisesRegex(MODULE.ProfileError, message):
                MODULE.validate_and_aggregate(manifest(), bad)

    def test_artifact_and_every_steady_call_are_bound(self) -> None:
        bad = cells()
        bad[0]["artifact"]["commit_sha"] = CANDIDATE
        with self.assertRaisesRegex(MODULE.ProfileError, "artifact commit"):
            MODULE.validate_and_aggregate(manifest(), bad)

        bad = cells()
        bad[3]["paths"]["engine"]["steady"][4]["output_digests"][19] = HASH
        with self.assertRaisesRegex(MODULE.ProfileError, "per-call outputs"):
            MODULE.validate_and_aggregate(manifest(), bad)

        bad = cells()
        bad[1]["cuda_inventory"]["uuid"] = "GPU-other"
        with self.assertRaisesRegex(MODULE.ProfileError, "RTX 3090"):
            MODULE.validate_and_aggregate(manifest(), bad)

    def test_repetition_score_and_order_contracts_fail_closed(self) -> None:
        bad = cells()
        bad[0]["paths"]["standalone"]["steady"].pop()
        with self.assertRaisesRegex(MODULE.ProfileError, "steady repetitions"):
            MODULE.validate_and_aggregate(manifest(), bad)

        bad = cells()
        bad[0]["paths"]["standalone"]["steady"][0]["output"]["ce_scores"] = {
            "0": 0.5,
            "1": 0.5,
        }
        with self.assertRaisesRegex(MODULE.ProfileError, "non-degenerate"):
            MODULE.validate_and_aggregate(manifest(), bad)

        bad = cells()
        bad[3]["paths"]["standalone"]["steady"][0]["output"]["ids"] = [0, 1]
        with self.assertRaisesRegex(MODULE.ProfileError, "standalone order"):
            MODULE.validate_and_aggregate(manifest(), bad)

    def test_cpu_cuda_tolerance_and_performance_gate_fail(self) -> None:
        bad = cells()
        for path in bad[3]["paths"].values():
            for repetition in path["cold"] + path["steady"]:
                repetition["output"]["ce_scores"]["0"] = 0.5
                repetition["output_digests"] = [
                    MODULE.canonical_digest(repetition["output"])
                ] * len(repetition["output_digests"])
        with self.assertRaisesRegex(MODULE.ProfileError, "CPU/CUDA"):
            MODULE.validate_and_aggregate(manifest(), bad)

        with self.assertRaisesRegex(MODULE.ProfileError, "p95 regression"):
            MODULE.validate_and_aggregate(manifest(), cells(candidate_duration=111))


if __name__ == "__main__":
    unittest.main()
