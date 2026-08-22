"""Minimal GPU-primary SCALE-01 TC-5 v2 runner.

The runner creates one fresh FathomDB per arm, uses the released 0.8.23 Python
wheel for GPU ingestion, then invokes the cache-only private Rust vector-stage
executable once per query. Raw manifests stay in the external run directory;
the aggregate receipt contains no corpus text, paths, predictions, or latency
claim.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import sqlite3
import statistics
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping, Sequence

from experiments.fathomdb_test_setup import prepare_test_database


class Tc5GpuV2Error(ValueError):
    """Raised when a v2 control, input, device, or result fails closed."""


@dataclass(frozen=True)
class Tc5GpuConfig:
    """Strict, minimal configuration for the two admitted TC-5 GPU arms."""

    arms: dict[str, int]
    candidate_k: int
    top_k: int
    query_count: int
    bootstrap_resamples: int
    query_select_seed: str
    bootstrap_seed: str
    corpus_root: Path
    corpus_index: str
    corpus_index_sha256: str
    qualified_manifest: Path
    qualified_manifest_sha256: str
    model_asset_directory: Path
    model_asset_digest: str
    python: Path
    fathomdb_bin: Path
    cuda_uuid: str


@dataclass(frozen=True)
class ArmInputs:
    """Validated external document and query inputs for one arm."""

    documents: tuple[dict[str, str], ...]
    queries: tuple[dict[str, str], ...]
    fixture_digest: str


_TOP_KEYS = {
    "schema_version",
    "program_track",
    "release",
    "arms",
    "measurement",
    "inputs",
    "runtime",
    "claim_boundary",
}
_MEASUREMENT_KEYS = {
    "candidate_k",
    "top_k",
    "query_count",
    "bootstrap_resamples",
    "query_select_seed",
    "bootstrap_seed",
    "ground_truth",
    "sut",
}
_INPUT_KEYS = {
    "corpus_root",
    "corpus_index",
    "corpus_index_sha256",
    "qualified_manifest",
    "qualified_manifest_sha256",
    "bridge_selection",
    "model_asset_directory",
    "model_asset_digest",
}
_RUNTIME_KEYS = {"python", "fathomdb_bin", "embed_device", "cuda_uuid", "binary_feature"}
_DIGEST = 64


def _json_bytes(value: object) -> bytes:
    return json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=True,
        allow_nan=False,
    ).encode("utf-8")


def _sha_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _sha_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _is_digest(value: object) -> bool:
    return (
        isinstance(value, str)
        and len(value) == _DIGEST
        and all(character in "0123456789abcdef" for character in value)
    )


def load_config(path: str | Path) -> Tc5GpuConfig:
    """Load and enforce the deliberately closed GPU-primary v2 configuration."""
    try:
        document = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise Tc5GpuV2Error("TC-5 v2 configuration is unavailable or invalid") from exc
    if not isinstance(document, dict) or set(document) != _TOP_KEYS:
        raise Tc5GpuV2Error("TC-5 v2 configuration keys drifted")
    measurement, inputs, runtime = document["measurement"], document["inputs"], document["runtime"]
    if (
        document["schema_version"] != "tc5-gpu-execution.v2"
        or document["program_track"] != "SCALE-01"
        or document["release"] != "0.8.23"
        or document["arms"] != {"bridge": 7667, "primary": 17272}
        or document["claim_boundary"] != "fidelity_and_uncertainty_only"
        or not isinstance(measurement, dict)
        or set(measurement) != _MEASUREMENT_KEYS
        or not isinstance(inputs, dict)
        or set(inputs) != _INPUT_KEYS
        or not isinstance(runtime, dict)
        or set(runtime) != _RUNTIME_KEYS
    ):
        raise Tc5GpuV2Error("TC-5 v2 fixed envelope drifted")
    if measurement != {
        "candidate_k": 192,
        "top_k": 10,
        "query_count": 100,
        "bootstrap_resamples": 1000,
        "query_select_seed": "0x0E77C0125E1EC7",
        "bootstrap_seed": "0x0E77B007574A9",
        "ground_truth": "exact-f32-same-model-top-10",
        "sut": "pre-fusion-1bit-k192-f32-rerank-vector-stage",
    }:
        raise Tc5GpuV2Error("TC-5 v2 fidelity pins drifted")
    if runtime["embed_device"] != "cuda:0" or runtime["binary_feature"] != "tc5-benchmark-cuda":
        raise Tc5GpuV2Error("TC-5 v2 requires the explicit CUDA benchmark runtime")
    if inputs["bridge_selection"] != "all-query-sources-then-qualified-manifest-order":
        raise Tc5GpuV2Error("TC-5 v2 bridge selection rule drifted")
    if not isinstance(runtime["cuda_uuid"], str) or not runtime["cuda_uuid"].startswith("GPU-"):
        raise Tc5GpuV2Error("TC-5 v2 requires a pinned CUDA UUID")
    for key in ("corpus_index_sha256", "qualified_manifest_sha256", "model_asset_digest"):
        if not _is_digest(inputs[key]):
            raise Tc5GpuV2Error(f"{key} is not a lowercase sha256")
    return Tc5GpuConfig(
        arms=dict(document["arms"]),
        candidate_k=measurement["candidate_k"],
        top_k=measurement["top_k"],
        query_count=measurement["query_count"],
        bootstrap_resamples=measurement["bootstrap_resamples"],
        query_select_seed=measurement["query_select_seed"],
        bootstrap_seed=measurement["bootstrap_seed"],
        corpus_root=Path(inputs["corpus_root"]),
        corpus_index=inputs["corpus_index"],
        corpus_index_sha256=inputs["corpus_index_sha256"],
        qualified_manifest=Path(inputs["qualified_manifest"]),
        qualified_manifest_sha256=inputs["qualified_manifest_sha256"],
        model_asset_directory=Path(inputs["model_asset_directory"]),
        model_asset_digest=inputs["model_asset_digest"],
        python=Path(runtime["python"]),
        fathomdb_bin=Path(runtime["fathomdb_bin"]),
        cuda_uuid=runtime["cuda_uuid"],
    )


def binary_environment(environment: Mapping[str, str], cuda_uuid: str) -> dict[str, str]:
    """Return a cache-only executor environment with exactly one visible GPU."""
    result = {
        key: value
        for key, value in environment.items()
        if not key.startswith("FATHOMDB_") and key != "NVIDIA_VISIBLE_DEVICES"
    }
    result["CUDA_VISIBLE_DEVICES"] = cuda_uuid
    return result


def selection_digest(kind: str, exclude_logical_id: str) -> str:
    """Match the benchmark executable's manifest-derived selection identity."""
    return _sha_bytes(f"kind:{kind}\nexclude_logical_id:{exclude_logical_id}".encode())


def _canonical_digest(fields: Sequence[tuple[str, str]]) -> str:
    digest = hashlib.sha256()
    for name, value in fields:
        for part in (name.encode(), value.encode()):
            digest.update(len(part).to_bytes(8, "big"))
            digest.update(part)
    return digest.hexdigest()


def settings_digest(spec: Mapping[str, object], manifest: Mapping[str, object]) -> str:
    """Match the Rust executor's length-prefixed resolved-settings digest."""
    manifest_identity = _canonical_digest(
        [
            ("model_asset_digest", str(manifest["model_asset_digest"])),
            ("expected_vector_rows", str(manifest["expected_vector_rows"])),
            ("allowed_candidate_k", ",".join(map(str, manifest["allowed_candidate_k"]))),
            ("allowed_top_k", ",".join(map(str, manifest["allowed_top_k"]))),
            ("fixture_digest", str(manifest["fixture_digest"])),
            ("index_digest", str(manifest["index_digest"])),
            ("index_construction_device", str(manifest["index_construction_device"])),
            ("query_digest", str(manifest["query_digest"])),
            ("seed_digest", str(manifest["seed_digest"])),
            ("runtime_identity", str(manifest["runtime_identity"])),
            ("build_identity", str(manifest["build_identity"])),
            ("scope_kind", str(manifest["scope_kind"])),
            ("exclude_logical_id", str(manifest["exclude_logical_id"])),
            ("selection_digest", str(manifest["selection_digest"])),
            ("cuda_uuid", str(manifest["cuda_uuid"])),
        ]
    )
    return _canonical_digest(
        [
            ("version", str(spec["version"])),
            ("workload", str(spec["workload"])),
            ("algorithm", str(spec["algorithm"])),
            ("rerank", str(spec["rerank"])),
            ("candidate_k", str(spec["candidate_k"])),
            ("top_k", str(spec["top_k"])),
            ("warmups", str(spec["warmups"])),
            ("repetitions", str(spec["repetitions"])),
            ("single_process", str(spec["single_process"]).lower()),
            ("manifest_identity", manifest_identity),
        ]
    )


def build_query_job(
    config: Tc5GpuConfig,
    *,
    query: Mapping[str, str],
    database_path: Path,
    document_count: int,
    fixture_digest: str,
    index_digest: str,
    binary_digest: str,
    manifest_path: Path,
) -> tuple[dict[str, object], dict[str, object]]:
    """Build one strict, source-excluding private-executor job."""
    query_text = query["query"]
    exclude = query["exclude_document_id"]
    manifest: dict[str, object] = {
        "version": 1,
        "model_asset_directory": str(config.model_asset_directory),
        "model_asset_digest": config.model_asset_digest,
        "database": str(database_path),
        "scope_kind": "doc",
        "exclude_logical_id": exclude,
        "selection_digest": selection_digest("doc", exclude),
        "query": query_text,
        "expected_vector_rows": document_count - 1,
        "allowed_candidate_k": [config.candidate_k],
        "allowed_top_k": [config.top_k],
        "fixture_digest": fixture_digest,
        "index_digest": index_digest,
        "index_construction_device": "cuda:0/default-embedder",
        "query_digest": _sha_bytes(query_text.encode()),
        "seed_digest": _sha_bytes(
            f"{config.query_select_seed}\n{config.bootstrap_seed}".encode()
        ),
        "runtime_identity": "fathomdb-0.8.23-tc5-cuda",
        "build_identity": binary_digest,
        "cuda_uuid": config.cuda_uuid,
    }
    spec: dict[str, object] = {
        "version": 1,
        "workload": "vector_stage_v1",
        "algorithm": "bit_knn_f32_rerank_v1",
        "rerank": "exact_f32",
        "candidate_k": config.candidate_k,
        "top_k": config.top_k,
        "warmups": 0,
        "repetitions": 1,
        "single_process": True,
        "manifest": str(manifest_path),
        "settings_digest": "",
    }
    spec["settings_digest"] = settings_digest(spec, manifest)
    return manifest, spec


def _safe_relative_file(root: Path, relative: object) -> Path:
    if not isinstance(relative, str) or not relative or Path(relative).is_absolute():
        raise Tc5GpuV2Error("corpus payload path is not a safe relative path")
    path = (root / relative).resolve()
    if not path.is_relative_to(root.resolve()) or not path.is_file():
        raise Tc5GpuV2Error("corpus payload is missing or escapes its root")
    return path


def _read_verified_text(root: Path, row: Mapping[str, object]) -> str:
    path = _safe_relative_file(root, row["relative_path"])
    text = path.read_text(encoding="utf-8")
    if not text or _sha_bytes(text.encode()) != row["content_sha256"]:
        raise Tc5GpuV2Error("corpus payload digest drifted")
    return text


def _load_arm_inputs(config: Tc5GpuConfig, arm: str) -> ArmInputs:
    if arm not in config.arms:
        raise Tc5GpuV2Error("TC-5 v2 arm must be bridge or primary")
    index_path = config.corpus_root / config.corpus_index
    if _sha_file(index_path) != config.corpus_index_sha256:
        raise Tc5GpuV2Error("corpus index digest drifted")
    if _sha_file(config.qualified_manifest) != config.qualified_manifest_sha256:
        raise Tc5GpuV2Error("qualified manifest digest drifted")
    index = json.loads(index_path.read_text(encoding="utf-8"))
    qualified = json.loads(config.qualified_manifest.read_text(encoding="utf-8"))
    count = config.arms[arm]
    raw_documents, raw_queries = index.get("documents"), index.get("queries")
    qualified_documents = qualified.get("documents")
    if (
        index.get("schema_version") != "tc5-corpus-input.v1"
        or index.get("query_select_seed") != config.query_select_seed
        or not isinstance(raw_documents, list)
        or len(raw_documents) != 17272
        or not isinstance(qualified_documents, list)
        or len(qualified_documents) != 17272
        or not isinstance(raw_queries, list)
        or len(raw_queries) != config.query_count
    ):
        raise Tc5GpuV2Error("qualified corpus or query envelope drifted")
    raw_by_id = {row.get("document_id"): row for row in raw_documents if isinstance(row, dict)}
    qualified_by_id = {
        row.get("document_id"): row for row in qualified_documents if isinstance(row, dict)
    }
    qualified_ids = [row.get("document_id") for row in qualified_documents]
    if arm == "bridge":
        required = list(dict.fromkeys(row.get("exclude_document_id") for row in raw_queries))
        selected_ids = required + [identifier for identifier in qualified_ids if identifier not in required]
        selected_ids = selected_ids[:count]
    else:
        selected_ids = qualified_ids
    if (
        not isinstance(selected_ids, list)
        or len(selected_ids) != count
        or len(set(selected_ids)) != count
        or any(identifier not in raw_by_id or identifier not in qualified_by_id for identifier in selected_ids)
    ):
        raise Tc5GpuV2Error("arm document selection drifted")
    selected = [raw_by_id[identifier] for identifier in selected_ids]
    documents: list[dict[str, str]] = []
    for raw in selected:
        expected = qualified_by_id[raw["document_id"]]
        if (
            not isinstance(raw, dict)
            or raw.get("document_id") != expected.get("document_id")
            or raw.get("content_sha256") != expected.get("content_sha256")
        ):
            raise Tc5GpuV2Error("document selection drifted from the qualified manifest")
        text = _read_verified_text(config.corpus_root, raw)
        documents.append({"document_id": raw["document_id"], "text": text})
    known = {row["document_id"] for row in documents}
    queries: list[dict[str, str]] = []
    for raw in raw_queries:
        if not isinstance(raw, dict) or raw.get("exclude_document_id") not in known:
            raise Tc5GpuV2Error("query source exclusion is outside the selected arm")
        text = _read_verified_text(config.corpus_root, raw)
        queries.append(
            {
                "query_id": raw["query_id"],
                "query": text,
                "exclude_document_id": raw["exclude_document_id"],
            }
        )
    fixture_digest = _sha_bytes(
        _json_bytes(
            [
                {"document_id": raw["document_id"], "content_sha256": raw["content_sha256"]}
                for raw in selected
            ]
        )
    )
    return ArmInputs(tuple(documents), tuple(queries), fixture_digest)


def dry_run(config_path: str | Path, arm: str, *, output_root: Path) -> dict[str, object]:
    """Validate all external inputs and controls without creating run state."""
    config = load_config(config_path)
    inputs = _load_arm_inputs(config, arm)
    missing = [
        str(path)
        for path in (config.python, config.fathomdb_bin, config.model_asset_directory)
        if not path.exists()
    ]
    if missing:
        raise Tc5GpuV2Error("runtime or cache input is unavailable")
    return {
        "schema_version": "tc5-gpu-dry-run.v2",
        "state": "ready",
        "arm": arm,
        "document_count": len(inputs.documents),
        "query_count": len(inputs.queries),
        "fixture_digest": inputs.fixture_digest,
        "cuda_uuid": config.cuda_uuid,
        "embedding_execution": "cuda:0",
        "candidate_execution": "cpu/sqlite-vec",
        "exact_f32_rerank_execution": "cpu/sqlite-vec",
        "cross_encoder_route": "disabled",
        "new_database_required": True,
        "output_root_exists": output_root.exists(),
    }


def _bootstrap(config: Tc5GpuConfig, values: tuple[float, ...]) -> tuple[float, float, float]:
    state = int(config.bootstrap_seed, 16)
    means: list[float] = []
    for _ in range(config.bootstrap_resamples):
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
    return (
        means[int(config.bootstrap_resamples * 0.025)],
        means[min(int(config.bootstrap_resamples * 0.975), config.bootstrap_resamples - 1)],
        statistics.pstdev(means),
    )


def aggregate_results(
    config: Tc5GpuConfig,
    *,
    arm: str,
    results: Sequence[Mapping[str, object]],
    fixture_digest: str,
    index_digest: str,
    binary_digest: str,
) -> dict[str, object]:
    """Validate 100 direct-route results and reduce them to safe fidelity evidence."""
    if arm not in config.arms or len(results) != config.query_count:
        raise Tc5GpuV2Error("TC-5 requires 100 complete query results")
    recalls: list[float] = []
    rerank_digests: list[str] = []
    truth_digests: list[str] = []
    expected_rows = config.arms[arm] - 1
    for result in results:
        recall = result.get("recall_at_top_k")
        routes = (
            result.get("vector_stage_route_count"),
            result.get("search_route_count"),
            result.get("fts_route_count"),
            result.get("fusion_route_count"),
            result.get("graph_route_count"),
            result.get("cross_encoder_route_count"),
        )
        if (
            result.get("version") != 1
            or result.get("status") != "measurement_complete"
            or result.get("candidate_k") != config.candidate_k
            or result.get("top_k") != config.top_k
            or result.get("embedding_device") != "cuda:0"
            or result.get("cuda_uuid") != config.cuda_uuid
            or result.get("selected_vector_rows") != expected_rows
            or result.get("candidate_count") != config.candidate_k
            or result.get("rerank_count") != config.top_k
            or result.get("ground_truth_count") != config.top_k
            or result.get("candidate_execution") != "cpu/sqlite-vec"
            or result.get("rerank_execution") != "cpu/sqlite-vec"
            or result.get("model_asset_digest") != config.model_asset_digest
            or routes != (1, 0, 0, 0, 0, 0)
            or not isinstance(recall, (int, float))
            or not math.isfinite(recall)
            or not 0.0 <= recall <= 1.0
            or not _is_digest(result.get("rerank_ids_digest"))
            or not _is_digest(result.get("ground_truth_ids_digest"))
        ):
            raise Tc5GpuV2Error("TC-5 query result is partial or violates the GPU direct-route contract")
        recalls.append(float(recall))
        rerank_digests.append(str(result["rerank_ids_digest"]))
        truth_digests.append(str(result["ground_truth_ids_digest"]))
    low, high, sigma = _bootstrap(config, tuple(recalls))
    return {
        "schema_version": "tc5-gpu-arm-result.v2",
        "program_track": "SCALE-01",
        "action": "tc5-gpu-smoke",
        "arm": arm,
        "document_count": config.arms[arm],
        "query_completion_count": config.query_count,
        "bootstrap_resamples": config.bootstrap_resamples,
        "synthetic_document_count": 0,
        "fixture_digest": fixture_digest,
        "index_digest": index_digest,
        "binary_digest": binary_digest,
        "ground_truth_sha256": _sha_bytes(_json_bytes(truth_digests)),
        "sut_result_sha256": _sha_bytes(_json_bytes(rerank_digests)),
        "metrics": {
            "recall_at_10": statistics.fmean(recalls),
            "ci_95": [low, high],
            "bootstrap_sigma": sigma,
        },
        "provenance": {
            "release": "0.8.23",
            "embed_device": "cuda:0",
            "embedding_execution": "cuda:0",
            "candidate_execution": "cpu/sqlite-vec",
            "exact_f32_rerank_execution": "cpu/sqlite-vec",
            "cross_encoder_route": "disabled",
            "cuda_uuid": config.cuda_uuid,
            "model_asset_digest": config.model_asset_digest,
            "candidate_k": config.candidate_k,
            "top_k": config.top_k,
            "query_select_seed": config.query_select_seed,
            "bootstrap_seed": config.bootstrap_seed,
            "ground_truth": "exact-f32-same-model-top-10",
            "sut": "pre-fusion-1bit-k192-f32-rerank-vector-stage",
        },
        "claim_boundary": "fidelity_and_uncertainty_only",
    }


def _write_new_json(path: Path, value: object) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "wb") as handle:
        handle.write(_json_bytes(value) + b"\n")


def _ingest_database(config: Tc5GpuConfig, inputs: ArmInputs, output_root: Path) -> Path:
    prior = {
        key: os.environ.get(key)
        for key in ("FATHOMDB_EMBED_DEVICE", "FATHOMDB_RERANK_DEVICE", "CUDA_VISIBLE_DEVICES")
    }
    os.environ.update(
        {
            "FATHOMDB_EMBED_DEVICE": "cuda:0",
            "FATHOMDB_RERANK_DEVICE": "cpu",
            "CUDA_VISIBLE_DEVICES": config.cuda_uuid,
        }
    )
    try:
        prepared = prepare_test_database(
            output_root,
            test_id="database",
            embed_device="cuda:0",
            # This policy belongs to the unused product cross-encoder. TC-5's
            # exact-f32 SQLite rerank is separately attested as CPU execution.
            rerank_device="cpu",
            embedder="default",
            warm_cache=True,
            fathomdb_bin=str(config.fathomdb_bin),
        )
        from fathomdb import Engine

        engine = Engine.open(str(prepared.database_path), use_default_embedder=True)
        try:
            for offset in range(0, len(inputs.documents), 256):
                engine.write(
                    [
                        {
                            "kind": "doc",
                            "body": row["text"],
                            "source_id": "tc5-real-only-v2",
                            "logical_id": row["document_id"],
                        }
                        for row in inputs.documents[offset : offset + 256]
                    ]
                )
            engine.drain(timeout_s=7200)
        finally:
            engine.close()
    finally:
        for key, value in prior.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value
    with sqlite3.connect(prepared.database_path) as connection:
        vector_count = connection.execute(
            "SELECT COUNT(*) FROM _fathomdb_vector_rows WHERE kind = 'doc'"
        ).fetchone()[0]
        logical_count = connection.execute(
            "SELECT COUNT(*) FROM canonical_nodes WHERE kind = 'doc' AND logical_id IS NOT NULL"
        ).fetchone()[0]
    if vector_count != len(inputs.documents) or logical_count != len(inputs.documents):
        raise Tc5GpuV2Error("fresh database did not project the complete selected arm")
    return prepared.database_path


def run_arm(
    config_path: str | Path,
    arm: str,
    *,
    output_root: Path,
    binary: Path,
) -> Path:
    """Create and execute one new GPU arm, returning its safe receipt path."""
    config = load_config(config_path)
    inputs = _load_arm_inputs(config, arm)
    if output_root.exists() or output_root.is_symlink():
        raise Tc5GpuV2Error("TC-5 output root must be new")
    if not binary.is_file():
        raise Tc5GpuV2Error("TC-5 CUDA benchmark binary is unavailable")
    output_root.mkdir(mode=0o700, parents=True)
    jobs_root = output_root / "query-jobs"
    jobs_root.mkdir(mode=0o700)
    database = _ingest_database(config, inputs, output_root)
    index_digest = _sha_file(database)
    binary_digest = _sha_file(binary)
    environment = binary_environment(os.environ, config.cuda_uuid)
    results: list[dict[str, object]] = []
    for offset, query in enumerate(inputs.queries, start=1):
        stem = f"query-{offset:03d}"
        manifest_path = jobs_root / f"{stem}.manifest.json"
        spec_path = jobs_root / f"{stem}.spec.json"
        result_path = jobs_root / f"{stem}.result.json"
        manifest, spec = build_query_job(
            config,
            query=query,
            database_path=database,
            document_count=len(inputs.documents),
            fixture_digest=inputs.fixture_digest,
            index_digest=index_digest,
            binary_digest=binary_digest,
            manifest_path=manifest_path,
        )
        _write_new_json(manifest_path, manifest)
        _write_new_json(spec_path, spec)
        completed = subprocess.run(
            [str(binary), "--spec", str(spec_path), "--result", str(result_path)],
            check=False,
            capture_output=True,
            text=True,
            env=environment,
        )
        if completed.returncode != 0 or not result_path.is_file():
            raise Tc5GpuV2Error(f"TC-5 private executor failed at query {offset:03d}")
        result = json.loads(result_path.read_text(encoding="utf-8"))
        if not isinstance(result, dict):
            raise Tc5GpuV2Error("TC-5 private executor emitted an invalid result")
        results.append(result)
    receipt = aggregate_results(
        config,
        arm=arm,
        results=results,
        fixture_digest=inputs.fixture_digest,
        index_digest=index_digest,
        binary_digest=binary_digest,
    )
    receipt_path = output_root / "tc5-gpu-arm-result.v2.json"
    _write_new_json(receipt_path, receipt)
    return receipt_path


def main(argv: Sequence[str] | None = None) -> int:
    """Validate or execute one admitted GPU-primary TC-5 arm."""
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    for command in ("dry-run", "run"):
        child = subparsers.add_parser(command)
        child.add_argument("--config", type=Path, required=True)
        child.add_argument("--arm", choices=("bridge", "primary"), required=True)
        child.add_argument("--output-root", type=Path, required=True)
        if command == "run":
            child.add_argument("--binary", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.command == "dry-run":
        result = dry_run(args.config, args.arm, output_root=args.output_root)
        print(json.dumps(result, sort_keys=True))
    else:
        receipt = run_arm(args.config, args.arm, output_root=args.output_root, binary=args.binary)
        print(json.dumps({"receipt": str(receipt)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
