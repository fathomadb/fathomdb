"""Unmasked red replay for the pre-hardening TC-5 executor revision.

This checkpoint intentionally imports ``0aa3d173`` rather than the hardened
working-tree module.  Its legacy-valid sidecar contains precisely the fields
accepted by that revision, so each failure below demonstrates the missing
review requirement itself rather than an unrelated provenance-key mismatch.
It is a one-time TDD evidence artifact and is removed after the replay.
"""

from __future__ import annotations

import json
import runpy
import subprocess
import sys
import types
from pathlib import Path

import pytest


_PRE_HARDENING_COMMIT = "0aa3d1730183b124eeb8f06cebcd64477c26957f"


def _helpers() -> dict[str, object]:
    return runpy.run_path(str(Path(__file__).with_name("test_tc5_live_executor.py")))


def _legacy_executor() -> types.ModuleType:
    source = subprocess.check_output(
        ["git", "show", f"{_PRE_HARDENING_COMMIT}:experiments/tc5_live_executor.py"], text=True
    )
    module = types.ModuleType("historical_tc5_live_executor")
    module.__file__ = str(Path(__file__).resolve().parents[2] / "experiments/tc5_live_executor.py")
    sys.modules[module.__name__] = module
    exec(compile(source, module.__file__, "exec"), module.__dict__)
    return module


def _legacy_runner(path: Path, *, sigma: str = "0.01") -> Path:
    path.write_text(
        "#!/usr/bin/env python3\n"
        "import hashlib, json, os\n"
        "arm = os.environ['TC5_ARM']\n"
        "result = {\n"
        "  'schema_version': 'tc5-arm-result.v1', 'program_track': 'SCALE-01',\n"
        "  'action': os.environ['TC5_ACTION'], 'arm': arm,\n"
        "  'document_count': int(os.environ['TC5_DOCUMENT_COUNT']),\n"
        "  'manifest_sha256': os.environ['TC5_MANIFEST_SHA256'],\n"
        "  'ground_truth_sha256': hashlib.sha256((arm + '-ground-truth').encode()).hexdigest(),\n"
        "  'sut_result_sha256': hashlib.sha256((arm + '-sut').encode()).hexdigest(),\n"
        "  'query_completion_count': 100, 'bootstrap_resamples': 1000,\n"
        "  'synthetic_document_count': 0,\n"
        "  'metrics': {'recall_at_10': 0.91, 'ci_95': [0.90, 0.92], 'bootstrap_sigma': "
        + sigma
        + "},\n"
        "  'provenance': {\n"
        "    'source_artifact_sha256': '" + "a" * 64 + "',\n"
        "    'source_commit': '" + "b" * 40 + "',\n"
        "    'cargo_lock_sha256': '" + "c" * 64 + "',\n"
        "    'model_asset_sha256': '" + "d" * 64 + "',\n"
        "    'cpu_identity': 'cpu-identity', 'os_identity': 'os-identity',\n"
        "    'embed_device': 'cpu', 'model_identity': 'fathomdb-bge-small-en-v1.5',\n"
        "    'candidate_breadth': 192, 'query_count': 100,\n"
        "    'query_select_seed': '0x0E77C0125E1EC7', 'bootstrap_seed': '0x0E77B007574A9',\n"
        "    'ground_truth': 'exact-f32-same-model-top-10',\n"
        "    'sut': 'pre-fusion-1bit-k192-f32-rerank-vector-stage'\n"
        "  }\n"
        "}\n"
        "json.dump(result, open(os.environ['TC5_ARM_RESULT_PATH'], 'w', encoding='utf-8'), sort_keys=True)\n",
        encoding="utf-8",
    )
    path.chmod(0o700)
    return path


def _request(tmp_path: Path, *, sigma: str = "0.01", manifest_mutation=None):
    helpers = _helpers()
    base = helpers["_base_config"]()
    live = helpers["_live_config"](base)
    manifest = helpers["_manifest"]()
    if manifest_mutation is not None:
        manifest_mutation(manifest)
    write_json = helpers["_write_json"]
    base_path = write_json(tmp_path / "execution.json", base)
    live_path = write_json(tmp_path / "live.json", live)
    manifest_path = write_json(tmp_path / "manifest.json", manifest)
    corpus_root = tmp_path / "corpus"
    output_root = tmp_path / "output"
    corpus_root.mkdir()
    output_root.mkdir()
    runner = _legacy_runner(tmp_path / "legacy-runner", sigma=sigma)
    release = helpers["_release"](live_config=live, manifest=manifest, executable=runner)
    release_path = write_json(tmp_path / "release.json", release)
    return base_path, live_path, manifest_path, corpus_root, output_root, release_path


def _execute(legacy: types.ModuleType, request: tuple[Path, Path, Path, Path, Path, Path]) -> Path:
    base, live, manifest, corpus, output, release = request
    return legacy.execute_tc5(
        action="tc5-smoke", release_path=release, live_config_path=live,
        base_execution_config_path=base, manifest_path=manifest,
        corpus_root=corpus, output_root=output,
    )


def test_legacy_baseline_payload_is_valid_before_each_targeted_mutation(tmp_path):
    receipt = _execute(_legacy_executor(), _request(tmp_path))

    assert receipt.is_file()


def test_legacy_does_not_reject_arm_symlink_escape_before_driver(tmp_path):
    legacy = _legacy_executor()
    request = _request(tmp_path)
    output = request[4]
    escaped = tmp_path / "escaped-arm"
    escaped.mkdir()
    (output / "tc5-smoke").symlink_to(escaped, target_is_directory=True)

    with pytest.raises(legacy.Tc5LiveExecutorError, match="escapes the declared external output root"):
        _execute(legacy, request)


def test_legacy_does_not_reject_receipt_symlink_escape_before_driver(tmp_path):
    legacy = _legacy_executor()
    request = _request(tmp_path)
    output = request[4]
    (output / "tc5-tc5-smoke-receipt.json").symlink_to(tmp_path / "escaped-receipt.json")

    with pytest.raises(legacy.Tc5LiveExecutorError, match="escapes the declared external output root"):
        _execute(legacy, request)


def test_legacy_receipt_lacks_required_input_digest_projection(tmp_path):
    receipt = json.loads(_execute(_legacy_executor(), _request(tmp_path)).read_text(encoding="utf-8"))

    assert "input_digests" in receipt


@pytest.mark.parametrize("sigma", ["float('nan')", "float('inf')", "float('-inf')"])
def test_legacy_does_not_reject_each_non_finite_sigma_as_non_finite(tmp_path, sigma):
    legacy = _legacy_executor()

    with pytest.raises(legacy.Tc5LiveExecutorError, match="finite"):
        _execute(legacy, _request(tmp_path, sigma=sigma))


@pytest.mark.parametrize(
    ("mutation", "field"),
    [
        (lambda manifest: manifest["provenance"].__setitem__("rust_version", "1.91.0"), "rust_version"),
        (
            lambda manifest: manifest["provenance"].__setitem__(
                "engine_features", ["default-embedder", "operator-extra"]
            ),
            "engine_features",
        ),
    ],
)
def test_legacy_does_not_validate_manifest_rust_or_engine_feature_drift(tmp_path, mutation, field):
    legacy = _legacy_executor()

    with pytest.raises(legacy.Tc5LiveExecutorError, match=field):
        _execute(legacy, _request(tmp_path, manifest_mutation=mutation))
