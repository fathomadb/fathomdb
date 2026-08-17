"""Human-intended tests for the coordinator-released SCALE-01 TC-5 executor."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
from datetime import UTC, datetime, timedelta
from pathlib import Path

import pytest

from experiments.tc5_live_executor import (
    TC5_LIVE_EXECUTOR_CONFIG_V1,
    TC5_LIVE_EXECUTOR_RECEIPT_V1,
    TC5_RELEASE_RECORD_V1,
    Tc5LiveExecutorError,
    execute_tc5,
    load_live_executor_config,
)
from experiments.tc5_manifest import (
    BOOTSTRAP_RESAMPLES,
    BOOTSTRAP_SEED,
    BRIDGE_DOCUMENT_COUNT,
    CANDIDATE_BREADTH,
    PRIMARY_DOCUMENT_COUNT,
    QUERY_COUNT,
    QUERY_SELECT_SEED,
)


def _canonical(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")


def _sha(number: int) -> str:
    return f"{number:064x}"


def _git_head() -> str:
    return subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()


def _manifest() -> dict[str, object]:
    documents = [
        {"document_id": f"document-{number:05d}", "content_sha256": _sha(number), "origin": "real"}
        for number in range(PRIMARY_DOCUMENT_COUNT)
    ]
    return {
        "schema_version": "tc5-manifest.v1",
        "program_track": "SCALE-01",
        "manifest_id": "eu7-tc5-all-real-18472",
        "source_artifact_sha256": "a" * 64,
        "documents": documents,
        "bridge_document_ids": [row["document_id"] for row in documents[:BRIDGE_DOCUMENT_COUNT]],
        "provenance": {
            "source_commit": "b" * 40,
            "cargo_lock_sha256": "c" * 64,
            "rust_version": "1.90.0",
            "cpu_identity": "cpu-identity",
            "os_identity": "os-identity",
            "model_identity": "fathomdb-bge-small-en-v1.5",
            "model_asset_sha256": "d" * 64,
            "engine_features": ["default-embedder", "operator"],
            "embed_device": "cpu",
            "candidate_breadth": CANDIDATE_BREADTH,
            "query_count": QUERY_COUNT,
            "query_select_seed": QUERY_SELECT_SEED,
            "bootstrap_resamples": BOOTSTRAP_RESAMPLES,
            "bootstrap_seed": BOOTSTRAP_SEED,
            "ground_truth": "exact-f32-same-model-top-10",
            "sut": "pre-fusion-1bit-k192-f32-rerank-vector-stage",
        },
    }


def _base_config() -> dict[str, object]:
    return {
        "schema_version": "tc5-execution-config.v1",
        "program_track": "SCALE-01",
        "configuration_id": "tc5-cpu-all-real-v1",
        "release_state": "awaiting_independent_review_and_coordinator_release",
        "execution_enabled": False,
        "arms": [
            {"name": "bridge", "document_count": BRIDGE_DOCUMENT_COUNT},
            {"name": "primary", "document_count": PRIMARY_DOCUMENT_COUNT},
        ],
        "frozen_configuration": {
            "embed_device": "cpu",
            "model_identity": "fathomdb-bge-small-en-v1.5",
            "candidate_breadth": CANDIDATE_BREADTH,
            "query_count": QUERY_COUNT,
            "query_select_seed": QUERY_SELECT_SEED,
            "bootstrap_resamples": BOOTSTRAP_RESAMPLES,
            "bootstrap_seed": BOOTSTRAP_SEED,
            "ground_truth": "exact-f32-same-model-top-10",
            "sut": "pre-fusion-1bit-k192-f32-rerank-vector-stage",
        },
        "artifact_contract": {
            "external_manifest_required": True,
            "external_corpus_root_required": True,
            "external_output_root_required": True,
            "historical_eu7_output_forbidden": True,
            "repository_artifacts_forbidden": True,
            "experiment_index_receipt_required_after_both_arms": True,
        },
        "claim_boundary": {
            "scale_02_capacity_claim": False,
            "latency_or_slo_claim": False,
            "only_fidelity_and_uncertainty_after_complete_arms": True,
        },
    }


def _live_config(base_config: dict[str, object]) -> dict[str, object]:
    return {
        "schema_version": TC5_LIVE_EXECUTOR_CONFIG_V1,
        "program_track": "SCALE-01",
        "executor_id": "tc5-all-real-cpu-executor-v1",
        "execution_config_sha256": hashlib.sha256(_canonical(base_config)).hexdigest(),
        "arms": base_config["arms"],
        "approved_actions": ["tc5-smoke", "tc5-long-cpu-characterization"],
        "result_contract": {
            "schema_version": "tc5-arm-result.v1",
            "required_ground_truth": "exact-f32-same-model-top-10",
            "required_embed_device": "cpu",
            "prohibit_payloads_and_raw_paths": True,
            "prohibit_scale_02_claims": True,
        },
    }


def _write_json(path: Path, document: dict[str, object]) -> Path:
    path.write_bytes(_canonical(document) + b"\n")
    return path


def _release(
    *,
    live_config: dict[str, object],
    manifest: dict[str, object],
    executable: Path,
    actions: list[str] | None = None,
) -> dict[str, object]:
    document = {
        "schema_version": TC5_RELEASE_RECORD_V1,
        "program_track": "SCALE-01",
        "release_id": "scale-01-tc5-live-release-v1",
        "issued_by": "track-runner-coordinator",
        "integrated_sha": _git_head(),
        "live_config_sha256": hashlib.sha256(_canonical(live_config)).hexdigest(),
        "manifest_sha256": hashlib.sha256(_canonical(manifest)).hexdigest(),
        "approved_actions": actions or ["tc5-smoke", "tc5-long-cpu-characterization"],
        "expires_at": (datetime.now(UTC) + timedelta(days=1)).isoformat().replace("+00:00", "Z"),
        "runner_argv": [str(executable)],
        "runner_sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
    }
    document["release_sha256"] = hashlib.sha256(_canonical(document)).hexdigest()
    return document


def _fake_runner(path: Path) -> Path:
    path.write_text(
        "#!/usr/bin/env python3\n"
        "import hashlib, json, os\n"
        "expected = {\n"
        "  'TC5_EMBED_DEVICE': 'cpu', 'TC5_MODEL_IDENTITY': 'fathomdb-bge-small-en-v1.5',\n"
        "  'TC5_CANDIDATE_BREADTH': '192', 'TC5_QUERY_COUNT': '100',\n"
        "  'TC5_QUERY_SELECT_SEED': '0x0E77C0125E1EC7', 'TC5_BOOTSTRAP_RESAMPLES': '1000',\n"
        "  'TC5_BOOTSTRAP_SEED': '0x0E77B007574A9',\n"
        "  'TC5_GROUND_TRUTH': 'exact-f32-same-model-top-10',\n"
        "  'TC5_SUT': 'pre-fusion-1bit-k192-f32-rerank-vector-stage'\n"
        "}\n"
        "if any(os.environ.get(key) != value for key, value in expected.items()): raise RuntimeError('pin drift')\n"
        "arm = os.environ['TC5_ARM']\n"
        "result = {\n"
        "  'schema_version': 'tc5-arm-result.v1',\n"
        "  'program_track': 'SCALE-01',\n"
        "  'action': os.environ['TC5_ACTION'],\n"
        "  'arm': arm,\n"
        "  'document_count': int(os.environ['TC5_DOCUMENT_COUNT']),\n"
        "  'manifest_sha256': os.environ['TC5_MANIFEST_SHA256'],\n"
        "  'ground_truth_sha256': hashlib.sha256((arm + '-ground-truth').encode()).hexdigest(),\n"
        "  'sut_result_sha256': hashlib.sha256((arm + '-sut').encode()).hexdigest(),\n"
        "  'query_completion_count': 100,\n"
        "  'bootstrap_resamples': 1000,\n"
        "  'synthetic_document_count': 0,\n"
        "  'metrics': {'recall_at_10': 0.91, 'ci_95': [0.90, 0.92], 'bootstrap_sigma': 0.01},\n"
        "  'provenance': {\n"
        "    'source_artifact_sha256': '" + "a" * 64 + "',\n"
        "    'source_commit': '" + "b" * 40 + "',\n"
        "    'cargo_lock_sha256': '" + "c" * 64 + "',\n"
        "    'model_asset_sha256': '" + "d" * 64 + "',\n"
        "    'rust_version': '1.90.0', 'engine_features': ['default-embedder', 'operator'],\n"
        "    'cpu_identity': 'cpu-identity', 'os_identity': 'os-identity',\n"
        "    'embed_device': 'cpu', 'model_identity': 'fathomdb-bge-small-en-v1.5',\n"
        "    'candidate_breadth': 192, 'query_count': 100,\n"
        "    'query_select_seed': '0x0E77C0125E1EC7', 'bootstrap_seed': '0x0E77B007574A9',\n"
        "    'ground_truth': 'exact-f32-same-model-top-10',\n"
        "    'sut': 'pre-fusion-1bit-k192-f32-rerank-vector-stage'\n"
        "  }\n"
        "}\n"
        "with open(os.environ['TC5_ARM_RESULT_PATH'], 'w', encoding='utf-8') as fh:\n"
        "  json.dump(result, fh, sort_keys=True)\n",
        encoding="utf-8",
    )
    path.chmod(0o700)
    return path


def _paths(tmp_path: Path) -> tuple[Path, Path, Path, Path, Path, Path]:
    base_config = _write_json(tmp_path / "tc5-execution.json", _base_config())
    live_config = _write_json(tmp_path / "tc5-live.json", _live_config(_base_config()))
    manifest = _write_json(tmp_path / "manifest.json", _manifest())
    corpus_root = tmp_path / "corpus"
    output_root = tmp_path / "output"
    corpus_root.mkdir()
    output_root.mkdir()
    runner = _fake_runner(tmp_path / "external-runner")
    return base_config, live_config, manifest, corpus_root, output_root, runner


def test_live_config_binds_the_disabled_execution_config_and_exact_two_arm_order(tmp_path):
    base_config, live_config_path, *_ = _paths(tmp_path)

    config = load_live_executor_config(live_config_path, base_execution_config_path=base_config)

    assert config.approved_actions == ("tc5-smoke", "tc5-long-cpu-characterization")
    assert config.arms == (("bridge", BRIDGE_DOCUMENT_COUNT), ("primary", PRIMARY_DOCUMENT_COUNT))


def test_committed_live_config_binds_the_committed_disabled_execution_contract():
    repository = Path(__file__).resolve().parents[2]

    config = load_live_executor_config(
        repository / "experiments/configs/scale-01/tc5-live-executor.v1.json",
        base_execution_config_path=repository / "experiments/configs/scale-01/tc5-execution.v1.json",
    )

    assert config.executor_id == "tc5-all-real-cpu-executor-v1"


def test_executor_requires_a_current_integrity_checked_coordinator_release(tmp_path):
    base_config, live_config_path, manifest_path, corpus_root, output_root, runner = _paths(tmp_path)

    with pytest.raises(Tc5LiveExecutorError, match="release record"):
        execute_tc5(
            action="tc5-smoke",
            release_path=tmp_path / "absent-release.json",
            live_config_path=live_config_path,
            base_execution_config_path=base_config,
            manifest_path=manifest_path,
            corpus_root=corpus_root,
            output_root=output_root,
        )

    release = _release(live_config=_live_config(_base_config()), manifest=_manifest(), executable=runner)
    release["integrated_sha"] = "0" * 40
    release_path = _write_json(tmp_path / "release.json", release)
    with pytest.raises(Tc5LiveExecutorError, match="release_sha256"):
        execute_tc5(
            action="tc5-smoke", release_path=release_path, live_config_path=live_config_path,
            base_execution_config_path=base_config, manifest_path=manifest_path,
            corpus_root=corpus_root, output_root=output_root,
        )
    assert list(output_root.iterdir()) == []


def test_executor_rejects_repository_and_historical_eu7_output_roots_before_runner_start(tmp_path):
    base_config, live_config_path, manifest_path, corpus_root, _, runner = _paths(tmp_path)
    release_path = _write_json(
        tmp_path / "release.json",
        _release(live_config=_live_config(_base_config()), manifest=_manifest(), executable=runner),
    )
    repository = Path(__file__).resolve().parents[2]

    with pytest.raises(Tc5LiveExecutorError, match="output root must remain outside the repository"):
        execute_tc5(
            action="tc5-smoke", release_path=release_path, live_config_path=live_config_path,
            base_execution_config_path=base_config, manifest_path=manifest_path,
            corpus_root=corpus_root, output_root=repository / "dev/plans/runs",
        )

    assert not (repository / "dev/plans/runs/eu7-latest-measurements.json").is_dir()


def test_executor_rejects_arm_directory_and_receipt_symlink_escapes_before_runner_start(tmp_path):
    base_config, live_config_path, manifest_path, corpus_root, output_root, runner = _paths(tmp_path)
    release_path = _write_json(
        tmp_path / "release.json",
        _release(live_config=_live_config(_base_config()), manifest=_manifest(), executable=runner),
    )
    escaped_arm = tmp_path / "escaped-arm"
    escaped_arm.mkdir()
    (output_root / "tc5-smoke").symlink_to(escaped_arm, target_is_directory=True)

    with pytest.raises(Tc5LiveExecutorError, match="escapes the declared external output root"):
        execute_tc5(
            action="tc5-smoke", release_path=release_path, live_config_path=live_config_path,
            base_execution_config_path=base_config, manifest_path=manifest_path,
            corpus_root=corpus_root, output_root=output_root,
        )

    assert list(escaped_arm.iterdir()) == []


def test_executor_rejects_receipt_symlink_escape_before_any_arm_driver_runs(tmp_path):
    base_config, live_config_path, manifest_path, corpus_root, output_root, runner = _paths(tmp_path)
    release_path = _write_json(
        tmp_path / "release.json",
        _release(live_config=_live_config(_base_config()), manifest=_manifest(), executable=runner),
    )
    escaped_receipt = tmp_path / "escaped-receipt.json"
    (output_root / "tc5-tc5-smoke-receipt.json").symlink_to(escaped_receipt)

    with pytest.raises(Tc5LiveExecutorError, match="escapes the declared external output root"):
        execute_tc5(
            action="tc5-smoke", release_path=release_path, live_config_path=live_config_path,
            base_execution_config_path=base_config, manifest_path=manifest_path,
            corpus_root=corpus_root, output_root=output_root,
        )

    assert not (output_root / "tc5-smoke").exists()
    assert not escaped_receipt.exists()


def test_executor_runs_only_bridge_then_primary_and_emits_a_safe_complete_receipt(tmp_path):
    base_config, live_config_path, manifest_path, corpus_root, output_root, runner = _paths(tmp_path)
    release_path = _write_json(
        tmp_path / "release.json",
        _release(live_config=_live_config(_base_config()), manifest=_manifest(), executable=runner),
    )

    receipt_path = execute_tc5(
        action="tc5-long-cpu-characterization", release_path=release_path,
        live_config_path=live_config_path, base_execution_config_path=base_config,
        manifest_path=manifest_path, corpus_root=corpus_root, output_root=output_root,
    )

    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    assert receipt["schema_version"] == TC5_LIVE_EXECUTOR_RECEIPT_V1
    assert [arm["name"] for arm in receipt["arms"]] == ["bridge", "primary"]
    assert receipt["experiment_index_projection"] == {
        "schema_version": "experiments.index-row.v1",
        "append_status": "not_appended",
        "eligibility": "eligible_after_complete_safe_two_arm_receipt",
    }
    assert receipt["input_digests"] == {
        "live_executor_config_sha256": hashlib.sha256(_canonical(_live_config(_base_config()))).hexdigest(),
        "execution_config_sha256": hashlib.sha256(_canonical(_base_config())).hexdigest(),
        "release_record_sha256": json.loads(release_path.read_text(encoding="utf-8"))["release_sha256"],
        "runner_sha256": hashlib.sha256(runner.read_bytes()).hexdigest(),
    }
    serialized = json.dumps(receipt)
    assert str(corpus_root) not in serialized
    assert str(output_root) not in serialized
    assert "document-00000" not in serialized
    assert "scale_02" not in serialized


@pytest.mark.parametrize(
    ("mutate", "message"),
    [
        (lambda release: release.__setitem__("expires_at", "2000-01-01T00:00:00Z"), "stale"),
        (lambda release: release.__setitem__("approved_actions", ["tc5-smoke"]), "not approved"),
        (lambda release: release.__setitem__("live_config_sha256", "c" * 64), "live executor configuration"),
        (lambda release: release.__setitem__("manifest_sha256", "f" * 64), "manifest"),
        (lambda release: release.__setitem__("runner_sha256", "e" * 64), "runner"),
    ],
)
def test_release_fails_closed_on_stale_or_mismatched_authority(tmp_path, mutate, message):
    base_config, live_config_path, manifest_path, corpus_root, output_root, runner = _paths(tmp_path)
    release = _release(live_config=_live_config(_base_config()), manifest=_manifest(), executable=runner)
    mutate(release)
    release["release_sha256"] = hashlib.sha256(
        _canonical({key: value for key, value in release.items() if key != "release_sha256"})
    ).hexdigest()
    release_path = _write_json(tmp_path / "release.json", release)

    with pytest.raises(Tc5LiveExecutorError, match=message):
        execute_tc5(
            action="tc5-long-cpu-characterization", release_path=release_path,
            live_config_path=live_config_path, base_execution_config_path=base_config,
            manifest_path=manifest_path, corpus_root=corpus_root, output_root=output_root,
        )
    assert list(output_root.iterdir()) == []


@pytest.mark.parametrize(
    ("mutation", "message"),
    [
        ("gpu", "embed_device"),
        ("missing-ground-truth", "ground_truth_sha256"),
        ("partial", "query_completion_count"),
        ("substitution", "document_count"),
        ("provenance", "source_artifact_sha256"),
        ("rust-version", "rust_version"),
        ("engine-features", "engine_features"),
        ("nan-sigma", "bootstrap_sigma"),
        ("infinite-sigma", "bootstrap_sigma"),
        ("claim", "SCALE-02"),
    ],
)
def test_executor_rejects_unqualified_arm_result_without_index_eligibility(tmp_path, mutation, message):
    base_config, live_config_path, manifest_path, corpus_root, output_root, runner = _paths(tmp_path)
    release_path = _write_json(
        tmp_path / "release.json",
        _release(live_config=_live_config(_base_config()), manifest=_manifest(), executable=runner),
    )
    runner.write_text(
        runner.read_text(encoding="utf-8")
        + "\nresult_path = os.environ['TC5_ARM_RESULT_PATH']\n"
        + "result = json.load(open(result_path, encoding='utf-8'))\n"
        + {
            "gpu": "result['provenance']['embed_device'] = 'cuda'",
            "missing-ground-truth": "result.pop('ground_truth_sha256')",
            "partial": "result['query_completion_count'] = 99",
            "substitution": "result['document_count'] = 1",
            "provenance": "result['provenance']['source_artifact_sha256'] = 'e' * 64",
            "rust-version": "result['provenance']['rust_version'] = '1.91.0'",
            "engine-features": "result['provenance']['engine_features'] = ['operator', 'default-embedder']",
            "nan-sigma": "result['metrics']['bootstrap_sigma'] = float('nan')",
            "infinite-sigma": "result['metrics']['bootstrap_sigma'] = float('inf')",
            "claim": "result['scale_02_claim'] = 'supported through 50k'",
        }[mutation]
        + "\njson.dump(result, open(result_path, 'w', encoding='utf-8'), sort_keys=True)\n",
        encoding="utf-8",
    )
    runner.chmod(0o700)
    release = _release(live_config=_live_config(_base_config()), manifest=_manifest(), executable=runner)
    release_path = _write_json(tmp_path / "release.json", release)

    with pytest.raises(Tc5LiveExecutorError, match=message):
        execute_tc5(
            action="tc5-smoke", release_path=release_path, live_config_path=live_config_path,
            base_execution_config_path=base_config, manifest_path=manifest_path,
            corpus_root=corpus_root, output_root=output_root,
        )
    assert not (output_root / "tc5-tc5-smoke-receipt.json").exists()


def test_resume_does_not_rerun_a_completed_qualified_arm(tmp_path):
    base_config, live_config_path, manifest_path, corpus_root, output_root, runner = _paths(tmp_path)
    release_path = _write_json(
        tmp_path / "release.json",
        _release(live_config=_live_config(_base_config()), manifest=_manifest(), executable=runner),
    )
    receipt_one = execute_tc5(
        action="tc5-smoke", release_path=release_path, live_config_path=live_config_path,
        base_execution_config_path=base_config, manifest_path=manifest_path,
        corpus_root=corpus_root, output_root=output_root,
    )
    bridge_result = output_root / "tc5-smoke" / "bridge" / "result.json"
    before = bridge_result.stat().st_mtime_ns
    receipt_two = execute_tc5(
        action="tc5-smoke", release_path=release_path, live_config_path=live_config_path,
        base_execution_config_path=base_config, manifest_path=manifest_path,
        corpus_root=corpus_root, output_root=output_root,
    )

    assert receipt_one == receipt_two
    assert bridge_result.stat().st_mtime_ns == before
    assert os.environ.get("TC5_ARM") is None
