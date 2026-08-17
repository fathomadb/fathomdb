"""Synthetic contract tests for the SCALE-01 external TC-5 arm driver."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from experiments import tc5_external_driver as driver


def _sha(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def _environment(tmp_path: Path, *, arm: str = "bridge") -> dict[str, str]:
    output = tmp_path / "output"
    output.mkdir()
    manifest = tmp_path / "manifest.json"
    manifest.write_text("{}", encoding="utf-8")
    corpus = tmp_path / "corpus"
    corpus.mkdir()
    count = "7667" if arm == "bridge" else "18472"
    return {
        "TC5_ACTION": "tc5-smoke",
        "TC5_ARM": arm,
        "TC5_DOCUMENT_COUNT": count,
        "TC5_MANIFEST_PATH": str(manifest),
        "TC5_MANIFEST_SHA256": "a" * 64,
        "TC5_CORPUS_ROOT": str(corpus),
        "TC5_OUTPUT_ROOT": str(output),
        "TC5_ARM_RESULT_PATH": str(output / "result.json"),
        "TC5_EMBED_DEVICE": "cpu",
        "TC5_MODEL_IDENTITY": "fathomdb-bge-small-en-v1.5",
        "TC5_MODEL_ASSET_SHA256": "b" * 64,
        "TC5_CANDIDATE_BREADTH": "192",
        "TC5_QUERY_COUNT": "100",
        "TC5_QUERY_SELECT_SEED": "0x0E77C0125E1EC7",
        "TC5_BOOTSTRAP_RESAMPLES": "1000",
        "TC5_BOOTSTRAP_SEED": "0x0E77B007574A9",
        "TC5_GROUND_TRUTH": "exact-f32-same-model-top-10",
        "TC5_SUT": "pre-fusion-1bit-k192-f32-rerank-vector-stage",
    }


def _inputs(count: int) -> driver.Tc5ArmInputs:
    documents = tuple(
        driver.Tc5Document(document_id=f"document-{index}", text=f"document {index}")
        for index in range(count)
    )
    queries = tuple(
        driver.Tc5Query(
            query_id=f"query-{index:03d}",
            text=f"query {index}",
            exclude_document_id=f"document-{index % count}",
        )
        for index in range(100)
    )
    return driver.Tc5ArmInputs(documents=documents, queries=queries)


class _Runtime:
    def __init__(self) -> None:
        self.requests: list[driver.Tc5RuntimeRequest] = []

    def measure(self, request: driver.Tc5RuntimeRequest) -> driver.Tc5Measurement:
        self.requests.append(request)
        return driver.Tc5Measurement(
            ground_truth_sha256=_sha("ground-truth-" + request.arm),
            sut_result_sha256=_sha("sut-" + request.arm),
            per_query_recall=tuple(0.9 for _ in request.inputs.queries),
        )


def test_driver_uses_only_the_frozen_env_abi_and_writes_one_content_free_arm_result(tmp_path):
    environment = _environment(tmp_path)
    runtime = _Runtime()

    result_path = driver.run_driver(
        environment,
        input_loader=lambda _request: _inputs(7667),
        runtime_factory=lambda _request: runtime,
    )

    result = json.loads(result_path.read_text(encoding="utf-8"))
    assert result == {
        "schema_version": "tc5-arm-result.v1",
        "program_track": "SCALE-01",
        "action": "tc5-smoke",
        "arm": "bridge",
        "document_count": 7667,
        "manifest_sha256": "a" * 64,
        "ground_truth_sha256": _sha("ground-truth-bridge"),
        "sut_result_sha256": _sha("sut-bridge"),
        "query_completion_count": 100,
        "bootstrap_resamples": 1000,
        "synthetic_document_count": 0,
        "metrics": {
            "recall_at_10": pytest.approx(0.9),
            "ci_95": [pytest.approx(0.9), pytest.approx(0.9)],
            "bootstrap_sigma": pytest.approx(0.0),
        },
        "provenance": {
            "source_artifact_sha256": "c" * 64,
            "source_commit": "d" * 40,
            "cargo_lock_sha256": "e" * 64,
            "model_asset_sha256": "b" * 64,
            "rust_version": "1.90.0",
            "engine_features": ["default-embedder", "operator"],
            "cpu_identity": "cpu-identity",
            "os_identity": "os-identity",
            "embed_device": "cpu",
            "model_identity": "fathomdb-bge-small-en-v1.5",
            "candidate_breadth": 192,
            "query_count": 100,
            "query_select_seed": "0x0E77C0125E1EC7",
            "bootstrap_seed": "0x0E77B007574A9",
            "ground_truth": "exact-f32-same-model-top-10",
            "sut": "pre-fusion-1bit-k192-f32-rerank-vector-stage",
        },
    }
    assert runtime.requests[0].arm == "bridge"
    assert str(tmp_path / "corpus") not in result_path.read_text(encoding="utf-8")
    assert "document 0" not in result_path.read_text(encoding="utf-8")


def test_driver_rejects_unknown_tc5_env_before_loading_corpus_or_runtime(tmp_path):
    environment = _environment(tmp_path)
    environment["TC5_BYPASS"] = "1"

    with pytest.raises(driver.Tc5ExternalDriverError, match="environment keys"):
        driver.run_driver(
            environment,
            input_loader=lambda _request: pytest.fail("must not load external inputs"),
            runtime_factory=lambda _request: pytest.fail("must not load runtime"),
        )


@pytest.mark.parametrize(
    ("mutation", "message"),
    [
        (("TC5_EMBED_DEVICE", "cuda"), "CPU-only"),
        (("TC5_DOCUMENT_COUNT", "1"), "document count"),
        (("TC5_BOOTSTRAP_RESAMPLES", "999"), "bootstrap"),
        (("CUDA_VISIBLE_DEVICES", "0"), "GPU"),
    ],
)
def test_driver_rejects_pin_or_gpu_drift_before_loading_external_inputs(tmp_path, mutation, message):
    environment = _environment(tmp_path)
    environment[mutation[0]] = mutation[1]

    with pytest.raises(driver.Tc5ExternalDriverError, match=message):
        driver.run_driver(
            environment,
            input_loader=lambda _request: pytest.fail("must not load external inputs"),
            runtime_factory=lambda _request: pytest.fail("must not load runtime"),
        )


def test_driver_rejects_incomplete_queries_and_nonfinite_measurement_without_result(tmp_path):
    environment = _environment(tmp_path, arm="primary")

    class IncompleteRuntime(_Runtime):
        def measure(self, request: driver.Tc5RuntimeRequest) -> driver.Tc5Measurement:
            return driver.Tc5Measurement(
                ground_truth_sha256=_sha("ground-truth"),
                sut_result_sha256=_sha("sut"),
                per_query_recall=(1.0,) * 99,
            )

    with pytest.raises(driver.Tc5ExternalDriverError, match="query completion"):
        driver.run_driver(
            environment,
            input_loader=lambda _request: _inputs(18472),
            runtime_factory=lambda _request: IncompleteRuntime(),
        )
    assert not Path(environment["TC5_ARM_RESULT_PATH"]).exists()


def test_driver_rejects_result_path_escape_before_loading_external_inputs(tmp_path):
    environment = _environment(tmp_path)
    escaped = tmp_path / "escaped.json"
    Path(environment["TC5_ARM_RESULT_PATH"]).symlink_to(escaped)

    with pytest.raises(driver.Tc5ExternalDriverError, match="output root"):
        driver.run_driver(
            environment,
            input_loader=lambda _request: pytest.fail("must not load external inputs"),
            runtime_factory=lambda _request: pytest.fail("must not load runtime"),
        )
