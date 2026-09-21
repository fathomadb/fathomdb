"""GRAPH-EVIDENCE-01 strict configuration and semantic fidelity scorer."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from experiments import graph_benchmarks
from experiments.fathomdb_test_setup import prepare_test_database


SCHEMA_VERSION = "graph-evidence-01.config.v1"
PROGRAM_TRACK = "GRAPH-EVIDENCE-01"
_ROOT_KEYS = {
    "schema_version",
    "program_track",
    "cell_id",
    "claim_boundary",
    "measurement",
    "fixture",
    "matrix",
    "outputs",
}
_MEASUREMENT_KEYS = {
    "warmup_operations",
    "measured_operations_per_cell",
    "repetitions",
    "percentile_estimator",
}
_FIXTURE_KEYS = {"id", "edge_sources", "refusal_classes", "validator_sha256"}
_MATRIX_KEYS = {"seed", "direction", "depth"}
_OUTPUT_KEYS = {"fidelity_section", "performance_section"}


class GraphEvidenceBenchmarkError(ValueError):
    """Report an invalid evidence benchmark contract or observation."""


@dataclass(frozen=True)
class Config:
    """Resolved GRAPH-EVIDENCE-01 execution contract."""

    program_track: str
    cell_id: str
    measurement: dict[str, object]
    fixture: dict[str, object]
    matrix: tuple[dict[str, object], ...]
    resolved: dict[str, object]


def _exact(value: object, label: str, keys: set[str]) -> dict[str, Any]:
    try:
        return graph_benchmarks.exact_mapping(value, label, keys)
    except graph_benchmarks.GraphBenchmarkError as error:
        raise GraphEvidenceBenchmarkError(str(error)) from error


def _positive_int(value: object, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
        raise GraphEvidenceBenchmarkError(f"{label} must be a positive integer")
    return value


def resolve_config(document: object) -> Config:
    """Strictly validate and resolve a GRAPH-EVIDENCE-01 configuration."""
    root = _exact(document, "config", _ROOT_KEYS)
    if root["schema_version"] != SCHEMA_VERSION:
        raise GraphEvidenceBenchmarkError("schema_version is unsupported")
    if root["program_track"] != PROGRAM_TRACK:
        raise GraphEvidenceBenchmarkError("program_track is invalid")
    if root["cell_id"] != "graph-evidence01":
        raise GraphEvidenceBenchmarkError("cell_id is invalid")
    if root["claim_boundary"] != "fathomdb_exact_graph_evidence_fidelity_and_cost":
        raise GraphEvidenceBenchmarkError("claim_boundary is invalid")
    measurement = _exact(root["measurement"], "measurement", _MEASUREMENT_KEYS)
    expected_measurement = {
        "warmup_operations": 5,
        "measured_operations_per_cell": 20,
        "repetitions": 3,
        "percentile_estimator": "linear-p-n-minus-1",
    }
    if measurement != expected_measurement:
        raise GraphEvidenceBenchmarkError("measurement identity drifted")
    for key in ("warmup_operations", "measured_operations_per_cell", "repetitions"):
        _positive_int(measurement[key], f"measurement.{key}")
    fixture = _exact(root["fixture"], "fixture", _FIXTURE_KEYS)
    if fixture["id"] != "graph-evidence-synthetic-v1":
        raise GraphEvidenceBenchmarkError("fixture identity drifted")
    if fixture["edge_sources"] != ["ordinary", "actuated", "parallel-winner"]:
        raise GraphEvidenceBenchmarkError("fixture edge sources drifted")
    matrix_value = root["matrix"]
    if not isinstance(matrix_value, list):
        raise GraphEvidenceBenchmarkError("matrix must be a list")
    matrix = tuple(_exact(item, "matrix cell", _MATRIX_KEYS) for item in matrix_value)
    expected_matrix = {
        (seed, direction, depth)
        for seed in ("explicit", "query")
        for direction in ("outgoing", "incoming", "both")
        for depth in (1, 2)
    }
    actual_matrix = {
        (item["seed"], item["direction"], item["depth"]) for item in matrix
    }
    if len(matrix) != 12 or actual_matrix != expected_matrix:
        raise GraphEvidenceBenchmarkError("matrix identity drifted")
    outputs = _exact(root["outputs"], "outputs", _OUTPUT_KEYS)
    if outputs != {
        "fidelity_section": "fidelity",
        "performance_section": "performance",
    }:
        raise GraphEvidenceBenchmarkError("output sections drifted")
    return Config(
        program_track=PROGRAM_TRACK,
        cell_id="graph-evidence01",
        measurement=dict(measurement),
        fixture=dict(fixture),
        matrix=tuple(dict(item) for item in matrix),
        resolved=dict(root),
    )


def load_config(path: str | Path) -> Config:
    """Load and strictly resolve one checked-in JSON configuration."""
    try:
        document = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise GraphEvidenceBenchmarkError(
            "configuration is unavailable or invalid"
        ) from error
    return resolve_config(document)


def score_observations(
    expectations: dict[str, object], observations: list[dict[str, object]]
) -> dict[str, object]:
    """Score observations against fixture-owned semantics and refusal classes."""
    positive = expectations.get("positive")
    refusals = expectations.get("refusals")
    if not isinstance(positive, dict) or not isinstance(refusals, dict):
        raise GraphEvidenceBenchmarkError("expectations are invalid")
    completed = typed_refusals = errors = 0
    target_exact = edge_exact = source_exact = 0
    semantic_by_case: dict[str, set[str]] = {}
    expand_ms: list[float] = []
    resolve_ms: list[float] = []
    for observation in observations:
        case_id = observation.get("case_id")
        outcome = observation.get("outcome")
        if not isinstance(case_id, str):
            raise GraphEvidenceBenchmarkError("observation case_id is invalid")
        if outcome == "resolved":
            expected = positive.get(case_id)
            if not isinstance(expected, dict):
                raise GraphEvidenceBenchmarkError(
                    f"unexpected positive case: {case_id}"
                )
            checks = (
                ("target_revision", "target revision"),
                ("edge_revision", "edge revision"),
                ("edge_from", "edge from"),
                ("edge_to", "edge to"),
                ("edge_kind", "edge kind"),
                ("source_sha256", "source hash"),
                ("source_body", "source body"),
            )
            for field, label in checks:
                if observation.get(field) != expected.get(field):
                    raise GraphEvidenceBenchmarkError(f"{label} mismatch for {case_id}")
            completed += 1
            target_exact += 1
            edge_exact += 1
            source_exact += 1
            semantic = observation.get("semantic_digest")
            if not isinstance(semantic, str) or not semantic:
                raise GraphEvidenceBenchmarkError("semantic digest is unavailable")
            semantic_by_case.setdefault(case_id, set()).add(semantic)
            for field, destination in (
                ("expand_ms", expand_ms),
                ("resolve_ms", resolve_ms),
            ):
                value = observation.get(field)
                if isinstance(value, (int, float)) and not isinstance(value, bool):
                    destination.append(float(value))
        elif outcome == "refused":
            expected_refusal = refusals.get(case_id)
            if observation.get("refusal_class") != expected_refusal:
                raise GraphEvidenceBenchmarkError(
                    f"refusal class mismatch for {case_id}"
                )
            typed_refusals += 1
        elif outcome == "error":
            errors += 1
        else:
            raise GraphEvidenceBenchmarkError(
                f"unknown observation outcome: {outcome!r}"
            )
    denominators = graph_benchmarks.reconcile_denominators(
        attempted=len(observations),
        completed=completed,
        errors=errors,
        typed_refusals=typed_refusals,
    )

    def rate(count: int) -> float:
        return count / completed if completed else 0.0

    performance: dict[str, object] = {"expand_latency_ms": {}, "resolve_latency_ms": {}}
    for name, values in (
        ("expand_latency_ms", expand_ms),
        ("resolve_latency_ms", resolve_ms),
    ):
        if values:
            performance[name] = {
                "p50": graph_benchmarks.percentile(values, 0.50),
                "p95": graph_benchmarks.percentile(values, 0.95),
                "p99": graph_benchmarks.percentile(values, 0.99),
            }
    return {
        "fidelity": {
            "target_revision_exact_rate": rate(target_exact),
            "terminal_edge_exact_rate": rate(edge_exact),
            "canonical_source_exact_rate": rate(source_exact),
            "typed_refusal_rate": typed_refusals / len(refusals) if refusals else 1.0,
            "semantic_identity_consistent": all(
                len(values) == 1 for values in semantic_by_case.values()
            ),
        },
        "performance": performance,
        "denominators": denominators,
    }


def run_smoke(
    config: Config, artifact_root: Path, *, config_path: Path
) -> dict[str, object]:
    """Exercise native evidence generation, resolution, and tamper refusal."""
    import fathomdb

    fathomdb_bin = os.environ.get(
        "FATHOMDB_BIN",
        str(Path(__file__).resolve().parent.parent / "target/debug/fathomdb"),
    )
    prepared = prepare_test_database(
        artifact_root,
        test_id="graph-evidence01",
        embed_device="cpu",
        rerank_device="cpu",
        embedder="none",
        check_reranker=False,
        fathomdb_bin=fathomdb_bin,
    )
    source_body = "canonical graph evidence bytes"
    source_digest = hashlib.sha256(source_body.encode()).hexdigest()

    def derived(revision: str) -> dict[str, object]:
        return {
            "schema_version": 1,
            "role": "derived",
            "artifact_revision_id": revision,
            "source_version_id": "source-v1",
            "source_revision_id": "source-r1",
            "source_locator": {"kind": "whole_body"},
            "canonical_source_hash": {
                "algorithm": "sha256",
                "digest_hex": source_digest,
            },
        }

    engine = fathomdb.Engine.open(
        str(prepared.database_path), use_default_embedder=False
    )
    repo = Path(__file__).resolve().parent.parent
    frozen_runner_path = repo / "scripts/release/smoke/frozen-evidence-python.py"
    validator_path = repo / "scripts/release/slice50-evidence-matrix.py"
    expected_validators = config.fixture["validator_sha256"]
    if not isinstance(expected_validators, dict):
        raise GraphEvidenceBenchmarkError("validator identities are unavailable")
    for path in (frozen_runner_path, validator_path):
        if (
            graph_benchmarks.sha256_file(path)
            != expected_validators[str(path.relative_to(repo))]
        ):
            raise GraphEvidenceBenchmarkError("evidence validator identity drifted")

    def load_script(name: str, path: Path) -> object:
        spec = importlib.util.spec_from_file_location(name, path)
        if spec is None or spec.loader is None:
            raise GraphEvidenceBenchmarkError(f"cannot load evidence helper {path}")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    frozen_runner = load_script("graph_evidence_frozen_runner", frozen_runner_path)
    validator = load_script("graph_evidence_matrix_validator", validator_path)
    try:
        engine.write(
            [
                {
                    "kind": "document",
                    "body": source_body,
                    "source_id": "owner",
                    "logical_id": "source",
                    "provenance": {
                        "schema_version": 1,
                        "role": "canonical",
                        "artifact_revision_id": "source-r1",
                        "source_version_id": "source-v1",
                    },
                },
                {
                    "kind": "claim",
                    "body": "root graphseed-evidence",
                    "source_id": "owner",
                    "logical_id": "root",
                    "provenance": derived("root-r1"),
                },
                {
                    "kind": "claim",
                    "body": "target",
                    "source_id": "owner",
                    "logical_id": "target",
                    "provenance": derived("target-r1"),
                },
                {
                    "edge": {
                        "kind": "supports",
                        "from": "root",
                        "to": "target",
                        "source_id": "owner",
                        "logical_id": "winner",
                        "provenance": derived("edge-r1"),
                    }
                },
            ]
        )
        engine.drain(timeout_s=30)
        frozen_runner._seed(engine, "slice10 canonical wheel evidence bytes")
        frozen = engine.freeze_read_context(fathomdb.ReadContextV1())

        def request(include_evidence: bool) -> object:
            return fathomdb.GraphExpandRequestV1(
                schema_version=1,
                seed=fathomdb.GraphExplicitSeedV1(
                    schema_version=1,
                    type="explicit",
                    logical_ids=(fathomdb.IdSpace(space="logical", value="root"),),
                ),
                direction="outgoing",
                edge_kinds=("supports",),
                target_kinds=("claim",),
                context=fathomdb.FrozenGraphReadContextV1(
                    schema_version=1, type="frozen", context=frozen
                ),
                max_depth=1,
                result_limit=10,
                max_work_units="10",
                include_explanation=False,
                include_evidence=include_evidence,
            )

        baseline = fathomdb.graph.expand(engine, request(False))
        started = time.perf_counter_ns()
        result = fathomdb.graph.expand(engine, request(True))
        expand_ms = (time.perf_counter_ns() - started) / 1_000_000
        if result.evidence is None or len(result.evidence.entries) != 1:
            raise GraphEvidenceBenchmarkError("native evidence sidecar is unavailable")
        if (
            result.targets != baseline.targets
            or result.work_units != baseline.work_units
        ):
            raise GraphEvidenceBenchmarkError(
                "include_evidence changed traversal semantics"
            )
        entry = result.evidence.entries[0]
        started = time.perf_counter_ns()
        target = engine.resolve_graph_evidence(
            fathomdb.GraphEvidenceResolveRequestV1(
                evidence_ref=entry.target_evidence_ref, context=frozen
            )
        )
        edge = engine.resolve_graph_evidence(
            fathomdb.GraphEvidenceResolveRequestV1(
                evidence_ref=entry.terminal_edge_evidence_ref, context=frozen
            )
        )
        resolve_ms = (time.perf_counter_ns() - started) / 1_000_000
        target_hash = target.canonical_source_hash
        target_digest = (
            target_hash["digest_hex"]
            if isinstance(target_hash, dict)
            else target_hash.digest_hex
        )
        observations = [
            {
                "case_id": "outgoing",
                "outcome": "resolved",
                "target_revision": target.artifact_revision_id,
                "edge_revision": edge.artifact_revision_id,
                "edge_from": edge.artifact.from_id,
                "edge_to": edge.artifact.to_id,
                "edge_kind": edge.artifact.kind,
                "source_sha256": target_digest,
                "source_body": target.canonical_source_body,
                "semantic_digest": graph_benchmarks.canonical_sha256(
                    [
                        target.artifact_revision_id,
                        edge.artifact_revision_id,
                        target_digest,
                    ]
                ),
                "expand_ms": expand_ms,
                "resolve_ms": resolve_ms,
            }
        ]
        tampered = entry.target_evidence_ref[:-1] + (
            "0" if entry.target_evidence_ref[-1] != "0" else "1"
        )
        try:
            engine.resolve_graph_evidence(
                fathomdb.GraphEvidenceResolveRequestV1(
                    evidence_ref=tampered, context=frozen
                )
            )
        except Exception:
            observations.append(
                {
                    "case_id": "tampered",
                    "outcome": "refused",
                    "refusal_class": "invalid_evidence_reference",
                }
            )
        else:
            raise GraphEvidenceBenchmarkError(
                "tampered evidence reference was accepted"
            )
        matrix_started = time.perf_counter_ns()
        matrix_rows = frozen_runner._graph_evidence_matrix(engine, frozen)
        validator.validate_rows(matrix_rows)
        matrix_ms = (time.perf_counter_ns() - matrix_started) / 1_000_000
    finally:
        engine.close()
    expectations = {
        "positive": {
            "outgoing": {
                "target_revision": "target-r1",
                "edge_revision": "edge-r1",
                "edge_from": "root",
                "edge_to": "target",
                "edge_kind": "supports",
                "source_sha256": source_digest,
                "source_body": source_body,
            }
        },
        "refusals": {"tampered": "invalid_evidence_reference"},
    }
    metrics = score_observations(expectations, observations)
    metrics["fidelity"].update(
        {
            "matrix_cases_complete": len(
                {(row["seed"], row["direction"], row["depth"]) for row in matrix_rows}
            )
            == 12,
            "matrix_rows": len(matrix_rows),
            "ordinary_and_actuated_edges": {row["edge_source"] for row in matrix_rows}
            == {"ordinary", "actuated"},
        }
    )
    metrics["performance"]["matrix_latency_ms"] = matrix_ms
    raw = artifact_root / "graph-evidence01-observations.json"
    raw.write_text(
        json.dumps(
            {"observations": observations, "matrix_rows": matrix_rows},
            indent=2,
            sort_keys=True,
        )
        + "\n"
    )
    record = graph_benchmarks.write_benchmark_record(
        experiment=config.cell_id,
        config=config.resolved,
        metrics=metrics,
        base_dir=artifact_root,
        config_path=config_path,
        corpus_manifest_sha256=graph_benchmarks.canonical_sha256(expectations),
        datasets=["graph-evidence-synthetic-v1"],
    )
    return {"state": "complete", "metrics": metrics, "record": str(record)}


def main(argv: list[str] | None = None) -> int:
    """Validate the configuration or execute the live native smoke."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("validate", "dry-run", "smoke"))
    parser.add_argument("config", type=Path)
    parser.add_argument("artifact_root", type=Path, nargs="?")
    args = parser.parse_args(argv)
    config = load_config(args.config)
    if args.artifact_root is not None and args.artifact_root.exists():
        raise GraphEvidenceBenchmarkError("artifact root must be new")
    if args.command == "smoke":
        if args.artifact_root is None:
            raise GraphEvidenceBenchmarkError("smoke artifact root is required")
        args.artifact_root.mkdir(parents=True)
        print(
            json.dumps(
                run_smoke(config, args.artifact_root, config_path=args.config),
                sort_keys=True,
            )
        )
        return 0
    print(json.dumps({"state": "ready", "cell": config.cell_id}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
