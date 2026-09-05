"""Contract tests for the preregistered Slice 40 CUDA throughput witness."""

from __future__ import annotations

import copy
import hashlib
import json
import math
import zipfile
from pathlib import Path

import pytest

from experiments import scale_02_slice40 as throughput


def _config() -> dict[str, object]:
    return {
        "schema_version": "scale-02-slice40-cuda-throughput.v3",
        "program_track": "SCALE-02",
        "release": "0.8.25",
        "campaign": "cuda-throughput",
        "approval": {
            "state": "approved",
            "approved_by": "HITL",
            "approved_at": "2026-09-05",
        },
        "candidate": {
            "product_commit": "2313fd34ea5ca68346a468d5bba58ba245306c08",
            "artifact_build_commit": "47afff50ec0e9f906c24c4422186eb407b2ff467",
            "wheel_sha256": "a" * 64,
            "cli_source_commit": "92faf4b693ad1c5ea6f76acb0f883d9e112793b4",
            "cli_sha256": (
                "fd2e26a487b48d8d18d0ed7bdc91497aa3e43ab3d57cb4abdea8b89bde546df9"
            ),
        },
        "execution": {
            "runner_sha256": "b" * 64,
            "record_library_sha256": "c" * 64,
            "classification_library_sha256": "d" * 64,
            "test_setup_library_sha256": "e" * 64,
        },
        "measurement_plan": {
            "path": (
                "experiments/configs/scale-02/"
                "slice40-cuda-throughput-measurement-plan.v2.json"
            ),
            "sha256": "f" * 64,
            "plan_id": "slice40-cuda-throughput-v2",
        },
        "workload": {
            "records": 1024,
            "write_batch_size": 128,
            "repetitions": 5,
            "warmup_records": 64,
            "drain_timeout_seconds": 900,
        },
        "runtime": {
            "embed_device": "cuda:0",
            "cuda_uuid": "GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b",
            "embedder": {
                "name": "fathomdb-bge-small-en-v1.5",
                "revision": "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a",
                "dimension": 384,
            },
            "network": "offline_environment",
        },
        "policy": {
            "classification": "descriptive",
            "decision_threshold": None,
            "excluded_time": ["artifact_install", "model_load", "model_warmup"],
            "included_time": ["configure_projection", "projection_backfill", "drain"],
        },
        "artifact_root": (
            "/home/coreyt/projects/fathomdb/data/performance-benchmarking/"
            "scale-02/slice40-runs"
        ),
        "claim_boundary": "cuda_projection_backfill_throughput_descriptive",
    }


def test_config_is_closed_and_hash_bound() -> None:
    resolved = throughput.resolve_cuda_throughput_config(
        _config(), validate_repository=False
    )
    assert resolved["workload"]["records"] == 1024

    drift = copy.deepcopy(_config())
    drift["workload"]["repetitions"] = 3  # type: ignore[index]
    with pytest.raises(throughput.Slice40ScaleError, match="workload"):
        throughput.resolve_cuda_throughput_config(drift, validate_repository=False)

    unknown = copy.deepcopy(_config())
    unknown["policy"]["percentile"] = 95  # type: ignore[index]
    with pytest.raises(throughput.Slice40ScaleError, match="policy keys"):
        throughput.resolve_cuda_throughput_config(unknown, validate_repository=False)


def test_summary_rejects_non_cuda_or_incomplete_projection() -> None:
    observations = [
        {
            "repetition": index,
            "elapsed_seconds": 2.0,
            "records": 1024,
            "records_per_second": 512.0,
            "selected_device": "cuda:0",
            "selected_cuda_uuid": "GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b",
            "pre_vector_rows": 0,
            "projection_delta": {
                "built": ["slice40_throughput"],
                "deferred": ["slice40_throughput"],
                "unchanged": False,
            },
            "vector_rows": 1024,
            "readiness": "ready",
        }
        for index in range(1, 6)
    ]
    summary = throughput.summarize_cuda_throughput_observations(
        observations, _config()
    )
    assert summary == {
        "classification": "descriptive",
        "repetitions": 5,
        "records_per_repetition": 1024,
        "throughput_records_per_second": {
            "minimum": 512.0,
            "median": 512.0,
            "maximum": 512.0,
        },
        "verdict": "observed",
    }

    observations[2]["selected_device"] = "cpu"
    with pytest.raises(throughput.Slice40ScaleError, match="cuda:0"):
        throughput.summarize_cuda_throughput_observations(observations, _config())

    observations[2]["selected_device"] = "cuda:0"
    observations[2]["vector_rows"] = 1023
    with pytest.raises(throughput.Slice40ScaleError, match="vector_rows"):
        throughput.summarize_cuda_throughput_observations(observations, _config())

    observations[2]["vector_rows"] = 1024
    observations[2]["pre_vector_rows"] = 1
    with pytest.raises(throughput.Slice40ScaleError, match="pre_vector_rows"):
        throughput.summarize_cuda_throughput_observations(observations, _config())


def test_summary_requires_the_shipped_searchable_vector_delta() -> None:
    observation = {
        "repetition": 1,
        "elapsed_seconds": 2.0,
        "records": 1024,
        "records_per_second": 512.0,
        "selected_device": "cuda:0",
        "selected_cuda_uuid": "GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b",
        "pre_vector_rows": 0,
        "projection_delta": {
            "built": ["slice40_throughput"],
            "deferred": ["slice40_throughput"],
            "unchanged": False,
        },
        "vector_rows": 1024,
        "readiness": "ready",
    }
    observations = [{**observation, "repetition": index} for index in range(1, 6)]
    observations[0]["projection_delta"] = {
        "built": [],
        "deferred": [],
        "unchanged": True,
    }
    with pytest.raises(throughput.Slice40ScaleError, match="non-noop"):
        throughput.summarize_cuda_throughput_observations(observations, _config())

    observations[0]["projection_delta"] = {
        "built": [],
        "deferred": ["slice40_throughput"],
        "unchanged": False,
    }
    with pytest.raises(throughput.Slice40ScaleError, match="non-noop"):
        throughput.summarize_cuda_throughput_observations(observations, _config())


@pytest.mark.parametrize("bad_rate", [math.nan, math.inf, -math.inf, 511.0])
def test_summary_rejects_non_finite_or_unreconciled_rates(bad_rate: float) -> None:
    observation = {
        "elapsed_seconds": 2.0,
        "records": 1024,
        "records_per_second": bad_rate,
        "selected_device": "cuda:0",
        "selected_cuda_uuid": "GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b",
        "pre_vector_rows": 0,
        "projection_delta": {
            "built": ["slice40_throughput"],
            "deferred": ["slice40_throughput"],
            "unchanged": False,
        },
        "vector_rows": 1024,
        "readiness": "ready",
    }
    observations = [
        {**observation, "repetition": repetition} for repetition in range(1, 6)
    ]
    with pytest.raises(throughput.Slice40ScaleError, match="rate"):
        throughput.summarize_cuda_throughput_observations(observations, _config())


def _write_test_wheel(path: Path, members: dict[str, bytes]) -> None:
    with zipfile.ZipFile(path, "w") as archive:
        for name, body in members.items():
            archive.writestr(name, body)


def test_package_attestation_rejects_mixed_or_extra_install_files(
    tmp_path: Path,
) -> None:
    members = {
        "fathomdb/__init__.py": b"# init\n",
        "fathomdb/engine.py": b"# engine\n",
        "fathomdb/read.py": b"# read\n",
        "fathomdb/types.py": b"# types\n",
        "fathomdb/_fathomdb.abi3.so": b"native",
    }
    wheel = tmp_path / "fathomdb.whl"
    _write_test_wheel(wheel, members)
    prefix = tmp_path / "venv"
    package = prefix / "lib/python3.11/site-packages/fathomdb"
    package.mkdir(parents=True)
    for member, body in members.items():
        target = package.parent / member
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(body)

    attested = throughput._verify_installed_package_files(
        wheel, package, prefix=prefix
    )
    assert {entry["wheel_member"] for entry in attested} == set(members)

    (package / "engine.py").write_text("# mixed\n", encoding="utf-8")
    with pytest.raises(throughput.Slice40ScaleError, match="match the wheel"):
        throughput._verify_installed_package_files(wheel, package, prefix=prefix)

    (package / "engine.py").write_bytes(members["fathomdb/engine.py"])
    (package / "stale.py").write_text("# stale\n", encoding="utf-8")
    with pytest.raises(throughput.Slice40ScaleError, match="extra"):
        throughput._verify_installed_package_files(wheel, package, prefix=prefix)


def test_cli_attestation_requires_the_preregistered_binary(
    tmp_path: Path,
) -> None:
    binary = tmp_path / "fathomdb"
    binary.write_bytes(b"reviewed-cli")
    binary.chmod(0o755)
    config = _config()
    config["candidate"]["cli_sha256"] = hashlib.sha256(  # type: ignore[index]
        b"reviewed-cli"
    ).hexdigest()

    attestation = throughput._cli_artifact_attestation(binary, config)
    assert attestation["cli_sha256"] == config["candidate"]["cli_sha256"]  # type: ignore[index]

    binary.write_bytes(b"different-cli")
    with pytest.raises(throughput.Slice40ScaleError, match="CLI SHA-256"):
        throughput._cli_artifact_attestation(binary, config)


def test_attestation_failure_is_registered_as_blocked(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    config = _config()
    config["artifact_root"] = str(tmp_path)
    captured: dict[str, object] = {}
    regenerated = False

    monkeypatch.setattr(
        throughput,
        "load_cuda_throughput_config",
        lambda _path: config,
    )
    monkeypatch.setattr(
        throughput._lib,
        "git_info",
        lambda: {"git_sha": "1" * 40, "dirty": False},
    )
    monkeypatch.setattr(throughput._lib, "env_info", lambda **_kwargs: {})
    monkeypatch.setattr(
        throughput,
        "_installed_artifact_attestation",
        lambda _wheel, _config: (_ for _ in ()).throw(
            throughput.Slice40ScaleError("mixed install")
        ),
    )

    def register(**kwargs: object) -> tuple[str, Path]:
        captured.update(kwargs)
        return "blocked-run", tmp_path / "record"

    def regen() -> None:
        nonlocal regenerated
        regenerated = True

    monkeypatch.setattr(throughput, "_register_run", register)
    monkeypatch.setattr(throughput._lib, "regen_index_md", regen)

    with pytest.raises(throughput.Slice40ScaleError, match="blocked"):
        throughput.run_cuda_throughput(
            tmp_path / "config.json",
            wheel=tmp_path / "candidate.whl",
            fathomdb_bin=tmp_path / "fathomdb",
        )

    assert captured["classification_outcome"] == "blocked"
    assert captured["blocked_reason"]["detail"]["error"] == "mixed install"  # type: ignore[index]
    assert captured["record_arguments"]["artifacts"][0]["kind"] == (  # type: ignore[index]
        "external_safe_summary"
    )
    assert regenerated
    details = list(tmp_path.glob("slice40-cuda-throughput-*/result-detail.json"))
    assert len(details) == 1
    assert json.loads(details[0].read_text(encoding="utf-8"))["installed_artifact"] is None
