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
            "passage_ids": [0, 1],
            "standalone_order": [1, 0],
            "engine_ids": [0, 1],
            "engine_ties": [],
        },
        "features": {
            "cpu": ["pyo3/extension-module", "default-reranker"],
            "cuda": ["pyo3/extension-module", "rerank-cuda"],
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
    }


def output(path: str, device: str) -> dict:
    ids = [1, 0] if path == "standalone" else [0, 1]
    shift = 0.005 if device == "cuda" else 0.0
    return {"ids": ids, "ce_scores": {"0": 0.2 + shift, "1": 0.8 + shift}}


def cell(role: str, device: str, duration: int = 100) -> dict:
    current_manifest = manifest()
    return {
        "schema_version": "fathomdb.slice72.ce-profile-cell/v1",
        "role": role,
        "commit_sha": BASELINE if role == "baseline" else CANDIDATE,
        "device": device,
        "features": current_manifest["features"][device],
        "wheel_sha256": HASH,
        "install_root": "/opt/slice72-venv",
        "module_path": "/opt/slice72-venv/lib/fathomdb/__init__.py",
        "source_imported": False,
        "effective_device": "cpu" if device == "cpu" else "cuda:0",
        "selected_uuid": None if device == "cpu" else "GPU-test",
        "allocation": None
        if device == "cpu"
        else {"pid": 4242, "gpu_uuid": "GPU-test", "vram_mib": 128},
        "model_sha256": current_manifest["model_sha256"],
        "paths": {
            path: {
                "cold": [
                    {"duration_ns": duration * 2, "output": output(path, device)}
                    for _ in range(3)
                ],
                "steady": [
                    {
                        "durations_ns": [duration] * 20,
                        "output": output(path, device),
                    }
                    for _ in range(5)
                ],
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

    def test_missing_or_unknown_cell_fails(self) -> None:
        with self.assertRaisesRegex(MODULE.ProfileError, "complete cell set"):
            MODULE.validate_and_aggregate(manifest(), cells()[:-1])
        bad = cells()
        bad[0]["unknown"] = True
        with self.assertRaisesRegex(MODULE.ProfileError, "unknown keys"):
            MODULE.validate_and_aggregate(manifest(), bad)

    def test_identity_device_and_allocation_fail_closed(self) -> None:
        for field, value, message in [
            ("source_imported", True, "source import"),
            ("effective_device", "cpu", "effective device"),
            ("allocation", None, "allocation"),
        ]:
            bad = cells()
            bad[1][field] = value
            with self.assertRaisesRegex(MODULE.ProfileError, message):
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
        with self.assertRaisesRegex(MODULE.ProfileError, "CPU/CUDA"):
            MODULE.validate_and_aggregate(manifest(), bad)

        with self.assertRaisesRegex(MODULE.ProfileError, "p95 regression"):
            MODULE.validate_and_aggregate(manifest(), cells(candidate_duration=111))


if __name__ == "__main__":
    unittest.main()
