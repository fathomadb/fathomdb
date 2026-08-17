#!/usr/bin/env python3
"""External-only SCALE-01 TC-5 arm driver.

Copy this file to the access-controlled external runtime named by an accepted
``tc5-execution-release.v1``.  It has no command-line interface: the released
live executor supplies its complete, explicit ``TC5_*`` environment ABI.  The
driver writes exactly one content-free ``tc5-arm-result.v1`` sidecar and never
writes a repository artifact or experiment-index row.
"""

from __future__ import annotations

import hashlib
import json
import math
import os
import re
import statistics
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Mapping, Protocol, Sequence

from experiments.tc5_manifest import (
    BOOTSTRAP_RESAMPLES,
    BOOTSTRAP_SEED,
    BRIDGE_DOCUMENT_COUNT,
    CANDIDATE_BREADTH,
    PRIMARY_DOCUMENT_COUNT,
    PROGRAM_TRACK,
    QUERY_COUNT,
    QUERY_SELECT_SEED,
    Tc5Manifest,
    load_manifest,
)


TC5_ARM_RESULT_V1 = "tc5-arm-result.v1"
TC5_CORPUS_INPUT_V1 = "tc5-corpus-input.v1"
_SAFE_ID = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$")
_SHA256 = re.compile(r"^[0-9a-f]{64}$")
_ARM_COUNTS = {"bridge": BRIDGE_DOCUMENT_COUNT, "primary": PRIMARY_DOCUMENT_COUNT}
_ACTIONS = {"tc5-smoke", "tc5-long-cpu-characterization"}
_REQUIRED_ENV = {
    "TC5_ACTION",
    "TC5_ARM",
    "TC5_DOCUMENT_COUNT",
    "TC5_MANIFEST_PATH",
    "TC5_MANIFEST_SHA256",
    "TC5_CORPUS_ROOT",
    "TC5_OUTPUT_ROOT",
    "TC5_ARM_RESULT_PATH",
    "TC5_EMBED_DEVICE",
    "TC5_MODEL_IDENTITY",
    "TC5_MODEL_ASSET_SHA256",
    "TC5_CANDIDATE_BREADTH",
    "TC5_QUERY_COUNT",
    "TC5_QUERY_SELECT_SEED",
    "TC5_BOOTSTRAP_RESAMPLES",
    "TC5_BOOTSTRAP_SEED",
    "TC5_GROUND_TRUTH",
    "TC5_SUT",
}
_PROVENANCE_KEYS = {
    "source_artifact_sha256",
    "source_commit",
    "cargo_lock_sha256",
    "model_asset_sha256",
    "rust_version",
    "engine_features",
    "cpu_identity",
    "os_identity",
    "embed_device",
    "model_identity",
    "candidate_breadth",
    "query_count",
    "query_select_seed",
    "bootstrap_seed",
    "ground_truth",
    "sut",
}


class Tc5ExternalDriverError(ValueError):
    """Raised before a TC-5 arm can emit a safe qualified result."""


@dataclass(frozen=True)
class Tc5Document:
    """One external all-real document held only during the arm invocation."""

    document_id: str
    text: str


@dataclass(frozen=True)
class Tc5Query:
    """One external fixed-query record held only during the arm invocation."""

    query_id: str
    text: str
    exclude_document_id: str


@dataclass(frozen=True)
class Tc5ArmInputs:
    """External corpus and query material validated against one TC-5 manifest."""

    documents: tuple[Tc5Document, ...]
    queries: tuple[Tc5Query, ...]
    manifest_sha256: str = "a" * 64
    provenance: Mapping[str, object] = field(
        default_factory=lambda: {
            "source_artifact_sha256": "c" * 64,
            "source_commit": "d" * 40,
            "cargo_lock_sha256": "e" * 64,
            "model_asset_sha256": "b" * 64,
            "rust_version": "1.90.0",
            "engine_features": ["default-embedder", "operator"],
            "cpu_identity": "cpu-identity",
            "os_identity": "os-identity",
        }
    )


@dataclass(frozen=True)
class Tc5RuntimeRequest:
    """Pinned inputs passed to an exact CPU vector-stage implementation."""

    action: str
    arm: str
    output_root: Path
    inputs: Tc5ArmInputs


@dataclass(frozen=True)
class Tc5Measurement:
    """Internal exact-ranked output reduced to aggregate-safe arm evidence."""

    ground_truth_sha256: str
    sut_result_sha256: str
    per_query_recall: tuple[float, ...]


class Tc5Runtime(Protocol):
    """Exact external CPU TC-5 runtime; no fused or GPU substitute is valid."""

    def measure(self, request: Tc5RuntimeRequest) -> Tc5Measurement:
        """Measure vector-stage recall against same-model exact-f32 ground truth."""


@dataclass(frozen=True)
class _DriverRequest:
    action: str
    arm: str
    document_count: int
    manifest_path: Path
    manifest_sha256: str
    corpus_root: Path
    output_root: Path
    result_path: Path
    model_asset_sha256: str


def _canonical(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True, allow_nan=False).encode(
        "utf-8"
    )


def _repository_root() -> Path | None:
    """Find a checkout only when this source is executing from one."""
    for candidate in Path(__file__).resolve().parents:
        if (candidate / ".git").exists():
            return candidate
    return None


def _is_repository_path(path: Path) -> bool:
    root = _repository_root()
    return root is not None and path.is_relative_to(root)


def _no_duplicate_keys(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise Tc5ExternalDriverError("duplicate JSON key")
        result[key] = value
    return result


def _identifier(value: object, label: str) -> str:
    if not isinstance(value, str) or _SAFE_ID.fullmatch(value) is None:
        raise Tc5ExternalDriverError(f"{label} must be a safe identifier")
    return value


def _sha256(value: object, label: str) -> str:
    if not isinstance(value, str) or _SHA256.fullmatch(value) is None:
        raise Tc5ExternalDriverError(f"{label} must be a lowercase sha256")
    return value


def _external_file(value: str, label: str) -> Path:
    path = Path(value).resolve()
    if _is_repository_path(path) or not path.is_file():
        raise Tc5ExternalDriverError(f"{label} must be an existing external file")
    return path


def _external_directory(value: str, label: str) -> Path:
    path = Path(value).resolve()
    if _is_repository_path(path) or not path.is_dir():
        raise Tc5ExternalDriverError(f"{label} must be an existing external directory")
    return path


def _safe_result_path(output_root: Path, value: str) -> Path:
    candidate = Path(value)
    if not candidate.is_absolute():
        raise Tc5ExternalDriverError("arm result path must be absolute")
    resolved = candidate.resolve(strict=False)
    if not resolved.is_relative_to(output_root) or _is_repository_path(resolved):
        raise Tc5ExternalDriverError("arm result path must remain under the external output root")
    return resolved


def _parse_environment(environment: Mapping[str, str]) -> _DriverRequest:
    tc5_keys = {key for key in environment if key.startswith("TC5_")}
    if tc5_keys != _REQUIRED_ENV:
        raise Tc5ExternalDriverError("TC5 environment keys do not match the released ABI")
    if environment.get("CUDA_VISIBLE_DEVICES") not in {None, ""} or environment.get(
        "NVIDIA_VISIBLE_DEVICES"
    ) not in {None, "", "void", "none"}:
        raise Tc5ExternalDriverError("GPU visibility is forbidden for the CPU-only TC-5 driver")
    if environment.get("FATHOMDB_EMBED_DEVICE") not in {None, "cpu"}:
        raise Tc5ExternalDriverError("FATHOMDB_EMBED_DEVICE must remain CPU-only")
    action, arm = environment["TC5_ACTION"], environment["TC5_ARM"]
    if action not in _ACTIONS or arm not in _ARM_COUNTS:
        raise Tc5ExternalDriverError("action or arm is outside the frozen TC-5 scope")
    if environment["TC5_EMBED_DEVICE"] != "cpu":
        raise Tc5ExternalDriverError("TC-5 is CPU-only")
    expected_pins = {
        "TC5_MODEL_IDENTITY": "fathomdb-bge-small-en-v1.5",
        "TC5_CANDIDATE_BREADTH": str(CANDIDATE_BREADTH),
        "TC5_QUERY_COUNT": str(QUERY_COUNT),
        "TC5_QUERY_SELECT_SEED": QUERY_SELECT_SEED,
        "TC5_BOOTSTRAP_RESAMPLES": str(BOOTSTRAP_RESAMPLES),
        "TC5_BOOTSTRAP_SEED": BOOTSTRAP_SEED,
        "TC5_GROUND_TRUTH": "exact-f32-same-model-top-10",
        "TC5_SUT": "pre-fusion-1bit-k192-f32-rerank-vector-stage",
    }
    if any(environment[key] != expected for key, expected in expected_pins.items()):
        raise Tc5ExternalDriverError("frozen TC-5 model, query, bootstrap, or SUT pin drifted")
    if environment["TC5_DOCUMENT_COUNT"] != str(_ARM_COUNTS[arm]):
        raise Tc5ExternalDriverError("arm document count does not match the frozen all-real envelope")
    manifest = _external_file(environment["TC5_MANIFEST_PATH"], "manifest path")
    corpus = _external_directory(environment["TC5_CORPUS_ROOT"], "corpus root")
    output = _external_directory(environment["TC5_OUTPUT_ROOT"], "output root")
    result_path = _safe_result_path(output, environment["TC5_ARM_RESULT_PATH"])
    if not result_path.parent.is_dir():
        raise Tc5ExternalDriverError("arm result parent directory must be pre-created by the executor")
    return _DriverRequest(
        action=action,
        arm=arm,
        document_count=_ARM_COUNTS[arm],
        manifest_path=manifest,
        manifest_sha256=_sha256(environment["TC5_MANIFEST_SHA256"], "manifest sha256"),
        corpus_root=corpus,
        output_root=output,
        result_path=result_path,
        model_asset_sha256=_sha256(environment["TC5_MODEL_ASSET_SHA256"], "model asset sha256"),
    )


def _load_json(path: Path, label: str) -> dict[str, object]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=_no_duplicate_keys)
    except (OSError, json.JSONDecodeError, Tc5ExternalDriverError) as exc:
        raise Tc5ExternalDriverError(f"{label} is unavailable or invalid") from exc
    if not isinstance(value, dict):
        raise Tc5ExternalDriverError(f"{label} must be a JSON object")
    return value


def _relative_file(root: Path, value: object, label: str) -> Path:
    if not isinstance(value, str) or not value or Path(value).is_absolute():
        raise Tc5ExternalDriverError(f"{label} must be a non-empty relative path")
    candidate = root.joinpath(value)
    resolved = candidate.resolve()
    if not resolved.is_relative_to(root) or not resolved.is_file():
        raise Tc5ExternalDriverError(f"{label} escapes the declared corpus root")
    return resolved


def _read_text(path: Path, expected_sha: object, label: str) -> str:
    digest = _sha256(expected_sha, f"{label} sha256")
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise Tc5ExternalDriverError(f"{label} cannot be read") from exc
    if not text or hashlib.sha256(text.encode("utf-8")).hexdigest() != digest:
        raise Tc5ExternalDriverError(f"{label} content digest drifted")
    return text


def _manifest_provenance(manifest: Tc5Manifest, request: _DriverRequest) -> dict[str, object]:
    provenance = manifest.provenance
    if manifest.manifest_sha256 != request.manifest_sha256:
        raise Tc5ExternalDriverError("qualified manifest digest drifted")
    if provenance["model_asset_sha256"] != request.model_asset_sha256:
        raise Tc5ExternalDriverError("model asset digest drifted from the qualified manifest")
    return {
        "source_artifact_sha256": manifest.source_artifact_sha256,
        "source_commit": provenance["source_commit"],
        "cargo_lock_sha256": provenance["cargo_lock_sha256"],
        "model_asset_sha256": provenance["model_asset_sha256"],
        "rust_version": provenance["rust_version"],
        "engine_features": list(provenance["engine_features"]),
        "cpu_identity": provenance["cpu_identity"],
        "os_identity": provenance["os_identity"],
    }


def load_arm_inputs(request: _DriverRequest) -> Tc5ArmInputs:
    """Load only the qualified external corpus/query material after ABI validation."""
    manifest = load_manifest(request.manifest_path)
    provenance = _manifest_provenance(manifest, request)
    index = _load_json(request.corpus_root / "tc5-corpus-input.v1.json", "TC-5 corpus input")
    expected_keys = {"schema_version", "manifest_sha256", "query_select_seed", "documents", "queries"}
    if set(index) != expected_keys or index["schema_version"] != TC5_CORPUS_INPUT_V1:
        raise Tc5ExternalDriverError("TC-5 corpus input schema drifted")
    if index["manifest_sha256"] != manifest.manifest_sha256 or index["query_select_seed"] != QUERY_SELECT_SEED:
        raise Tc5ExternalDriverError("TC-5 corpus input provenance or query selection drifted")
    documents_raw, queries_raw = index["documents"], index["queries"]
    if not isinstance(documents_raw, list) or not isinstance(queries_raw, list):
        raise Tc5ExternalDriverError("TC-5 corpus input rows are unsafe")
    expected_documents = manifest.documents[: request.document_count]
    if len(documents_raw) != len(expected_documents) or len(queries_raw) != QUERY_COUNT:
        raise Tc5ExternalDriverError("TC-5 corpus input count does not match the frozen arm or query envelope")
    documents: list[Tc5Document] = []
    for raw, expected in zip(documents_raw, expected_documents, strict=True):
        if not isinstance(raw, dict) or set(raw) != {"document_id", "content_sha256", "relative_path"}:
            raise Tc5ExternalDriverError("TC-5 corpus document row schema drifted")
        if raw["document_id"] != expected["document_id"] or raw["content_sha256"] != expected["content_sha256"]:
            raise Tc5ExternalDriverError("TC-5 corpus document does not bind the canonical manifest")
        document_id = _identifier(raw["document_id"], "document id")
        text = _read_text(
            _relative_file(request.corpus_root, raw["relative_path"], "document payload"),
            raw["content_sha256"],
            "document payload",
        )
        documents.append(Tc5Document(document_id=document_id, text=text))
    queries: list[Tc5Query] = []
    query_ids: set[str] = set()
    known_documents = {item.document_id for item in documents}
    for raw in queries_raw:
        if not isinstance(raw, dict) or set(raw) != {
            "query_id",
            "content_sha256",
            "relative_path",
            "exclude_document_id",
        }:
            raise Tc5ExternalDriverError("TC-5 query row schema drifted")
        query_id = _identifier(raw["query_id"], "query id")
        exclude = _identifier(raw["exclude_document_id"], "query exclude document id")
        if query_id in query_ids or exclude not in known_documents:
            raise Tc5ExternalDriverError("TC-5 query identity or exclusion is unsafe")
        query_ids.add(query_id)
        text = _read_text(
            _relative_file(request.corpus_root, raw["relative_path"], "query payload"),
            raw["content_sha256"],
            "query payload",
        )
        queries.append(Tc5Query(query_id=query_id, text=text, exclude_document_id=exclude))
    if [item.query_id for item in queries] != sorted(query_ids):
        raise Tc5ExternalDriverError("TC-5 query ordering must be canonical")
    return Tc5ArmInputs(
        documents=tuple(documents),
        queries=tuple(queries),
        manifest_sha256=manifest.manifest_sha256,
        provenance=provenance,
    )


def _recall_at_10(ranked: Sequence[str], ground_truth: Sequence[str]) -> float:
    if len(ranked) != 10 or len(ground_truth) != 10 or len(set(ranked)) != 10 or len(set(ground_truth)) != 10:
        raise Tc5ExternalDriverError("exact and vector-stage ranks must each contain ten unique records")
    return len(set(ranked) & set(ground_truth)) / 10.0


class FathomDBTc5Runtime:
    """Fail closed until a separately reviewed exact vector-stage runtime is supplied.

    The supported Python binding deliberately has no vector-stage selector. A
    public ``Engine.search`` call would measure fused retrieval instead, so the
    driver never treats it as a substitute. An external operational deployment
    supplies a reviewed :class:`Tc5Runtime` implementation directly to this
    module's sealed runner package; the repository-side executable remains
    synthetic-fixture-testable without expanding a shared binding surface.
    """

    def __init__(self, _request: _DriverRequest) -> None:
        """Retain the factory shape without accepting a runtime control input."""

    def measure(self, _request: Tc5RuntimeRequest) -> Tc5Measurement:
        """Refuse a fused-search fallback when the exact runtime is absent."""
        raise Tc5ExternalDriverError(
            "external TC-5 deployment lacks a separately reviewed exact vector-stage runtime"
        )


def _bootstrap(values: tuple[float, ...]) -> tuple[float, float, float]:
    if len(values) != QUERY_COUNT or not all(math.isfinite(value) and 0.0 <= value <= 1.0 for value in values):
        raise Tc5ExternalDriverError("query completion or recall values are incomplete or non-finite")
    state = int(BOOTSTRAP_SEED, 16)
    means: list[float] = []
    for _ in range(BOOTSTRAP_RESAMPLES):
        total = 0.0
        for _ in values:
            state = (state + 0x9E3779B97F4A7C15) & ((1 << 64) - 1)
            value = state
            value = ((value ^ (value >> 30)) * 0xBF58476D1CE4E5B9) & ((1 << 64) - 1)
            value = ((value ^ (value >> 27)) * 0x94D049BB133111EB) & ((1 << 64) - 1)
            value ^= value >> 31
            total += values[value % len(values)]
        means.append(total / len(values))
    means.sort()
    low = means[int(BOOTSTRAP_RESAMPLES * 0.025)]
    high = means[min(int(BOOTSTRAP_RESAMPLES * 0.975), BOOTSTRAP_RESAMPLES - 1)]
    return low, high, statistics.pstdev(means)


def _result(request: _DriverRequest, inputs: Tc5ArmInputs, measurement: Tc5Measurement) -> dict[str, object]:
    if len(inputs.documents) != request.document_count or len(inputs.queries) != QUERY_COUNT:
        raise Tc5ExternalDriverError("loaded corpus or query count does not match the released arm")
    if inputs.manifest_sha256 != request.manifest_sha256:
        raise Tc5ExternalDriverError("loaded corpus inputs do not bind the released manifest")
    _sha256(measurement.ground_truth_sha256, "ground truth digest")
    _sha256(measurement.sut_result_sha256, "SUT result digest")
    ci_low, ci_high, sigma = _bootstrap(measurement.per_query_recall)
    recall = sum(measurement.per_query_recall) / len(measurement.per_query_recall)
    if not all(math.isfinite(value) for value in (recall, ci_low, ci_high, sigma)):
        raise Tc5ExternalDriverError("aggregate TC-5 statistics must be finite")
    raw_provenance = dict(inputs.provenance)
    if set(raw_provenance) != {
        "source_artifact_sha256",
        "source_commit",
        "cargo_lock_sha256",
        "model_asset_sha256",
        "rust_version",
        "engine_features",
        "cpu_identity",
        "os_identity",
    }:
        raise Tc5ExternalDriverError("qualified manifest provenance keys drifted")
    if raw_provenance["model_asset_sha256"] != request.model_asset_sha256:
        raise Tc5ExternalDriverError("qualified manifest model asset provenance drifted")
    engine_features = raw_provenance["engine_features"]
    if not isinstance(engine_features, list) or engine_features != sorted(engine_features):
        raise Tc5ExternalDriverError("qualified manifest engine feature provenance drifted")
    provenance = {
        **raw_provenance,
        "embed_device": "cpu",
        "model_identity": "fathomdb-bge-small-en-v1.5",
        "candidate_breadth": CANDIDATE_BREADTH,
        "query_count": QUERY_COUNT,
        "query_select_seed": QUERY_SELECT_SEED,
        "bootstrap_seed": BOOTSTRAP_SEED,
        "ground_truth": "exact-f32-same-model-top-10",
        "sut": "pre-fusion-1bit-k192-f32-rerank-vector-stage",
    }
    if set(provenance) != _PROVENANCE_KEYS:
        raise Tc5ExternalDriverError("result provenance shape drifted")
    return {
        "schema_version": TC5_ARM_RESULT_V1,
        "program_track": PROGRAM_TRACK,
        "action": request.action,
        "arm": request.arm,
        "document_count": request.document_count,
        "manifest_sha256": request.manifest_sha256,
        "ground_truth_sha256": measurement.ground_truth_sha256,
        "sut_result_sha256": measurement.sut_result_sha256,
        "query_completion_count": QUERY_COUNT,
        "bootstrap_resamples": BOOTSTRAP_RESAMPLES,
        "synthetic_document_count": 0,
        "metrics": {"recall_at_10": recall, "ci_95": [ci_low, ci_high], "bootstrap_sigma": sigma},
        "provenance": provenance,
    }


def run_driver(
    environment: Mapping[str, str] | None = None,
    *,
    input_loader: Callable[[_DriverRequest], Tc5ArmInputs] = load_arm_inputs,
    runtime_factory: Callable[[_DriverRequest], Tc5Runtime] = FathomDBTc5Runtime,
) -> Path:
    """Validate one released arm environment, run it, and emit its safe sidecar."""
    request = _parse_environment(dict(os.environ) if environment is None else environment)
    if request.result_path.exists():
        raise Tc5ExternalDriverError(
            "arm result destination already exists; resume belongs to tc5_live_executor"
        )
    inputs = input_loader(request)
    runtime = runtime_factory(request)
    result = _result(
        request,
        inputs,
        runtime.measure(Tc5RuntimeRequest(request.action, request.arm, request.output_root, inputs)),
    )
    request.result_path.write_bytes(_canonical(result) + b"\n")
    return request.result_path


def main(argv: Sequence[str] | None = None) -> int:
    """Run exactly one released external arm without accepting command-line inputs."""
    arguments = list(sys.argv[1:] if argv is None else argv)
    if arguments:
        raise Tc5ExternalDriverError("TC-5 external driver accepts only the released environment ABI")
    run_driver()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
