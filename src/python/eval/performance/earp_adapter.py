"""Durable repeated-performance evidence linked to verified EARP quality work."""

from __future__ import annotations

import hashlib
import json
import math
import shutil
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any, Callable, Mapping, Sequence

from eval.earp._experiments import lib as _lib
from eval.earp.schema import WORKLOAD_MANIFEST_SCHEMA_PATH
from eval.earp.schema.validate import validate

PERFORMANCE_RESULT_NAME = "performance.earp.v1.json"
SCHEMA_VERSION = "performance.earp.v1"
WORKLOAD_MANIFEST_NAME = "earp.workload-manifest.v1.json"
_EARP_RESULT_NAME = "earp.result.v1.json"
_MANIFEST_SCHEMA = json.loads(WORKLOAD_MANIFEST_SCHEMA_PATH.read_text(encoding="utf-8"))
_PERFORMANCE_SCHEMA_PATH = Path(__file__).with_name("schema") / "performance.earp.v1.schema.json"
_PERFORMANCE_SCHEMA = json.loads(_PERFORMANCE_SCHEMA_PATH.read_text(encoding="utf-8"))


class PerformanceSchemaError(ValueError):
    """A performance publication document violates its durable schema."""


class PerformanceCollision(RuntimeError):
    """A deterministic performance identity already contains different bytes."""


@dataclass(frozen=True)
class WorkloadRef:
    """Verified quality-workload identity; never reconstructed from a record."""

    parent_run_id: str
    evidence_family_id: str
    config_sha256: str
    candidate_sha: str
    query_call: str
    effective_knobs: Mapping[str, Any]
    parent_manifest_path: str = ""
    parent_manifest_sha256: str = ""
    quality_result_path: str = _EARP_RESULT_NAME
    quality_result_sha256: str = ""
    resolved_config_path: str = "config.resolved.yaml"
    resolved_config_sha256: str = ""
    quality_clean: bool = False

    def __post_init__(self) -> None:
        if not self.parent_run_id or not self.evidence_family_id or not self.query_call:
            raise ValueError("workload identifiers and query_call must be non-empty")
        if not _sha(self.config_sha256) or not self.candidate_sha:
            raise ValueError("config_sha256 must be full and candidate_sha non-empty")

    def as_document(self) -> dict[str, Any]:
        return {
            "parent_run_id": self.parent_run_id,
            "evidence_family_id": self.evidence_family_id,
            "config_sha256": self.config_sha256,
            "candidate_sha": self.candidate_sha,
            "query_call": self.query_call,
            "effective_knobs": dict(self.effective_knobs),
        }


@dataclass(frozen=True)
class PerformancePlan:
    """Predeclared execution matrix and statistical eligibility policy."""

    repetitions: int
    treatments: tuple[str, ...]

    def __post_init__(self) -> None:
        if self.repetitions < 1 or not self.treatments or len(set(self.treatments)) != len(self.treatments):
            raise ValueError("repetitions must be >= 1 and treatments non-empty/unique")
        allowed = {"fresh_store", "fresh_store_warm_query"}
        if any(treatment not in allowed for treatment in self.treatments):
            raise ValueError("only fresh_store and fresh_store_warm_query are supported")

    def as_document(self) -> dict[str, Any]:
        return {
            "repetitions": self.repetitions,
            "treatments": list(self.treatments),
            "aggregation_rule": "descriptive_empirical_order_statistics",
            "invalid_result_policy": "typed_cell",
        }


@dataclass(frozen=True)
class RunSample:
    """One raw timing observation retained even when its cell later fails."""

    treatment: str
    repetition: int
    phases_ms: Mapping[str, float]
    counts: Mapping[str, int]
    treatment_witness: Mapping[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if not self.treatment or self.repetition < 0:
            raise ValueError("treatment must be non-empty and repetition >= 0")
        _nonnegative(self.phases_ms, "phase")
        _nonnegative(self.counts, "count")

    def as_document(self) -> dict[str, Any]:
        return {
            "treatment": self.treatment,
            "repetition": self.repetition,
            "phases_ms": {name: float(value) for name, value in self.phases_ms.items()},
            "counts": {name: int(value) for name, value in self.counts.items()},
            **({"treatment_witness": dict(self.treatment_witness)} if self.treatment_witness else {}),
        }


@dataclass(frozen=True)
class PerformanceCell:
    """A total matrix cell: complete measurements or typed invalid evidence."""

    treatment: str
    repetition: int
    status: str
    raw_samples: tuple[RunSample, ...]
    execution_provenance: Mapping[str, Any]
    invalidity: Mapping[str, str] | None = None

    @classmethod
    def complete(
        cls, *, treatment: str, repetition: int, samples: Sequence[RunSample], execution_provenance: Mapping[str, Any]
    ) -> "PerformanceCell":
        return cls(treatment, repetition, "complete", tuple(samples), dict(execution_provenance))

    @classmethod
    def invalid(
        cls, *, treatment: str, repetition: int, raw_samples: Sequence[RunSample], invalidity: Mapping[str, str], execution_provenance: Mapping[str, Any]
    ) -> "PerformanceCell":
        return cls(treatment, repetition, "invalid", tuple(raw_samples), dict(execution_provenance), dict(invalidity))

    @property
    def phases_ms(self) -> Mapping[str, float]:
        return self.raw_samples[-1].phases_ms if self.raw_samples else {}

    @property
    def counts(self) -> Mapping[str, int]:
        return self.raw_samples[-1].counts if self.raw_samples else {}

    @property
    def treatment_witness(self) -> Mapping[str, Any]:
        return self.raw_samples[-1].treatment_witness if self.raw_samples else {}

    def as_document(self) -> dict[str, Any]:
        document: dict[str, Any] = {
            "treatment": self.treatment,
            "repetition": self.repetition,
            "status": self.status,
            "raw_samples": [sample.as_document() for sample in self.raw_samples],
            "execution_provenance": dict(self.execution_provenance),
        }
        if self.invalidity is not None:
            document["invalidity"] = dict(self.invalidity)
        return document


@dataclass(frozen=True)
class PerformanceWriteOutcome:
    run_id: str
    run_dir: Path


def _file_sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _sha(value: Any) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(char in "0123456789abcdef" for char in value)


def _artifact(record: Mapping[str, Any], expected_path: str) -> Mapping[str, Any]:
    artifacts = record.get("artifacts")
    if not isinstance(artifacts, list):
        raise ValueError("quality record lacks workload manifest advertisement")
    for item in artifacts:
        if isinstance(item, Mapping) and item.get("path") == expected_path and _sha(item.get("sha256")):
            return item
    raise ValueError("quality record lacks workload manifest advertisement")


def load_earp_workload(experiments_root: Path, quality_run_id: str) -> WorkloadRef:
    """Verify the record → manifest → quality input directed digest graph."""
    run_dir = Path(experiments_root) / "runs" / quality_run_id
    try:
        record = json.loads((run_dir / "record.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f"cannot read EARP quality artifacts for {quality_run_id}: {exc}") from exc
    manifest_rel = f"runs/{quality_run_id}/{WORKLOAD_MANIFEST_NAME}"
    advertised = _artifact(record, manifest_rel)
    manifest_path = run_dir / WORKLOAD_MANIFEST_NAME
    if not manifest_path.is_file():
        raise ValueError("workload manifest digest does not match record advertisement")
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise ValueError(f"manifest schema is invalid: {exc}") from exc
    findings = validate(manifest, _MANIFEST_SCHEMA)
    if findings:
        raise ValueError("manifest schema is invalid")
    parent = manifest["quality_parent"]
    resolved = manifest["resolved_config"]
    workload = manifest["workload"]
    if not parent["candidate_sha"]:
        raise ValueError("manifest lacks candidate provenance")
    if _file_sha(manifest_path) != advertised["sha256"]:
        raise ValueError("workload manifest digest does not match record advertisement")
    if parent["run_id"] != quality_run_id or parent["evidence_family_id"] != quality_run_id:
        raise ValueError("manifest quality parent does not match requested run")
    result_path = run_dir / parent["result_path"]
    if not result_path.is_file() or _file_sha(result_path) != parent["result_sha256"]:
        raise ValueError("quality result digest does not match manifest")
    config_path = run_dir / resolved["path"]
    if not config_path.is_file() or _file_sha(config_path) != resolved["sha256"]:
        raise ValueError("resolved config digest does not match manifest")
    return WorkloadRef(
        parent_run_id=quality_run_id,
        evidence_family_id=parent["evidence_family_id"],
        config_sha256=workload["config_sha256"],
        candidate_sha=parent["candidate_sha"],
        query_call=workload["query_call"],
        effective_knobs=dict(workload["effective_knobs"]),
        parent_manifest_path=manifest_rel,
        parent_manifest_sha256=advertised["sha256"],
        quality_result_path=parent["result_path"],
        quality_result_sha256=parent["result_sha256"],
        resolved_config_path=resolved["path"],
        resolved_config_sha256=resolved["sha256"],
        quality_clean=bool(parent["clean"]),
    )


def run_repetitions(*, workload: WorkloadRef, plan: PerformancePlan, execute: Callable[[WorkloadRef, str, int], RunSample | PerformanceCell]) -> tuple[PerformanceCell, ...]:
    """Execute every planned cell once, preserving executor failure as evidence."""
    cells: list[PerformanceCell] = []
    for treatment in plan.treatments:
        for repetition in range(plan.repetitions):
            try:
                returned = execute(workload, treatment, repetition)
            except Exception as exc:  # executor errors are typed matrix evidence
                cells.append(PerformanceCell.invalid(
                    treatment=treatment, repetition=repetition, raw_samples=(),
                    invalidity={"code": _invalidity_code(exc), "message": f"{type(exc).__name__}: {exc}"},
                    execution_provenance={},
                ))
                continue
            if (returned.treatment, returned.repetition) != (treatment, repetition):
                raise ValueError("executor returned a sample for a different declared cell")
            if isinstance(returned, PerformanceCell):
                cells.append(returned)
            else:
                cells.append(PerformanceCell.complete(
                    treatment=treatment, repetition=repetition, samples=(returned,), execution_provenance={}
                ))
    return tuple(cells)


def summarize_samples(samples: Sequence[RunSample]) -> dict[str, dict[str, dict[str, float | int | str]]]:
    grouped: dict[str, dict[str, list[float]]] = {}
    for sample in samples:
        for phase, value in sample.phases_ms.items():
            grouped.setdefault(sample.treatment, {}).setdefault(phase, []).append(float(value))
    return {treatment: {phase: _summary(values) for phase, values in sorted(phases.items())} for treatment, phases in sorted(grouped.items())}


def _summary(values: Sequence[float]) -> dict[str, float | int | str]:
    ordered = sorted(values)
    result: dict[str, float | int | str] = {"n": len(ordered), "min_ms": ordered[0], "max_ms": ordered[-1], "mean_ms": sum(ordered) / len(ordered)}
    if len(ordered) >= 20:
        result.update({"p50_ms": _nearest(ordered, .5), "p95_ms": _nearest(ordered, .95), "aggregation_scope": "descriptive_empirical_order_statistic"})
    if len(ordered) >= 100:
        result["p99_ms"] = _nearest(ordered, .99)
    return result


def _nearest(values: Sequence[float], quantile: float) -> float:
    return values[max(0, math.ceil(quantile * len(values)) - 1)]


def _execution_provenance(value: Mapping[str, Any]) -> dict[str, Any]:
    required = ("candidate_sha", "clean", "command", "lockfile_sha256", "toolchain", "device", "fixtures")
    if not isinstance(value, Mapping) or any(key not in value for key in required):
        raise ValueError("execution provenance is incomplete")
    if not isinstance(value["candidate_sha"], str) or not value["candidate_sha"]:
        raise ValueError("execution provenance candidate_sha is invalid")
    if not isinstance(value["clean"], bool) or not isinstance(value["command"], str) or not value["command"]:
        raise ValueError("execution provenance is incomplete")
    if not _sha(value["lockfile_sha256"]) or not isinstance(value["toolchain"], Mapping) or not isinstance(value["fixtures"], Mapping):
        raise ValueError("execution provenance is incomplete")
    device = value["device"]
    if not isinstance(device, Mapping) or device.get("kind") not in {"cpu", "cuda", "metal"}:
        raise PerformanceSchemaError("execution provenance device is invalid")
    return dict(value)


def _validate_cells(plan: PerformancePlan, cells: Sequence[PerformanceCell]) -> None:
    expected = {(t, r) for t in plan.treatments for r in range(plan.repetitions)}
    actual = {(cell.treatment, cell.repetition) for cell in cells}
    if actual != expected or len(cells) != len(expected):
        raise ValueError("cells must contain exactly the declared complete matrix")
    for cell in cells:
        if cell.status not in {"complete", "invalid"}:
            raise PerformanceSchemaError("cell status is invalid")
        if cell.status == "complete" and not cell.raw_samples:
            raise ValueError("complete matrix cell has no raw samples")
        if cell.status == "invalid" and not cell.invalidity:
            raise ValueError("invalid cell has no typed invalidity")


def write_performance_result(*, experiments_root: Path, experiment: str, ts: datetime, workload: WorkloadRef, plan: PerformancePlan, samples: Sequence[RunSample] | None = None, cells: Sequence[PerformanceCell] | None = None, execution_provenance: Mapping[str, Any] | None = None) -> PerformanceWriteOutcome:
    """Validate then atomically stage a repeated-performance artifact and record."""
    if not workload.parent_manifest_path or not _sha(workload.parent_manifest_sha256):
        raise ValueError("manifest is required for repeated performance")
    provenance = _execution_provenance(execution_provenance) if execution_provenance is not None else None
    if cells is not None and samples is not None:
        raise ValueError("supply cells or samples, not both")
    if cells is None:
        if samples is None:
            raise ValueError("complete matrix requires samples")
        cells = tuple(PerformanceCell.complete(
            treatment=sample.treatment, repetition=sample.repetition, samples=(sample,), execution_provenance=provenance or {}
        ) for sample in samples)
    else:
        cells = tuple(cells)
        for cell in cells:
            if cell.execution_provenance:
                _execution_provenance(cell.execution_provenance)
    _validate_cells(plan, cells)
    cross = any((cell.execution_provenance or provenance or {}).get("candidate_sha") != workload.candidate_sha for cell in cells)
    normalized_cells: list[PerformanceCell] = []
    for cell in cells:
        cell_provenance = _execution_provenance(cell.execution_provenance or provenance or {})
        if cell_provenance["candidate_sha"] != workload.candidate_sha and cell.status == "complete":
            cell = PerformanceCell.invalid(treatment=cell.treatment, repetition=cell.repetition, raw_samples=cell.raw_samples, invalidity={"code": "cross_candidate", "message": "execution candidate differs from quality candidate"}, execution_provenance=cell_provenance)
        normalized_cells.append(cell)
    cells = tuple(normalized_cells)
    complete_samples = [sample for cell in cells if cell.status == "complete" for sample in cell.raw_samples]
    quality_digest = hashlib.sha256(_lib.canonical_json(workload.as_document()).encode()).hexdigest()
    execution_digest = hashlib.sha256(_lib.canonical_json({"quality": quality_digest, "provenance": [cell.execution_provenance for cell in cells]}).encode()).hexdigest()
    config = {"schema_version": SCHEMA_VERSION, "workload": workload.as_document(), "plan": plan.as_document(), "execution_workload_sha256": execution_digest}
    run_id = _lib.make_run_id(experiment, ts, _lib.config_sha256(config))
    run_dir = Path(experiments_root) / "runs" / run_id
    document = {
        "schema_version": SCHEMA_VERSION, "run_id": run_id, "scope": "repeated_performance_characterization",
        "relation": "cross_candidate_reexecution" if cross else "same_candidate_reexecution",
        "quality_workload_sha256": quality_digest, "execution_workload_sha256": execution_digest,
        "workload": workload.as_document(), "plan": plan.as_document(),
        "parent_manifest": {"path": workload.parent_manifest_path, "sha256": workload.parent_manifest_sha256},
        "inputs": {"quality_result": {"path": workload.quality_result_path, "sha256": workload.quality_result_sha256}, "resolved_config": {"path": workload.resolved_config_path, "sha256": workload.resolved_config_sha256}},
        "cells": [cell.as_document() for cell in cells],
        # Kept as a convenience rendering, while cells remain the authoritative matrix.
        "samples": [sample.as_document() for sample in complete_samples],
        "summary": summarize_samples(complete_samples),
    }
    _validate_performance_document(document)
    text = json.dumps(document, indent=2, sort_keys=True) + "\n"
    try:
        run_dir.mkdir(parents=True, exist_ok=False)
    except FileExistsError:
        sidecar = run_dir / PERFORMANCE_RESULT_NAME
        if sidecar.is_file() and sidecar.read_text(encoding="utf-8") == text:
            return PerformanceWriteOutcome(run_id, run_dir)
        raise PerformanceCollision(f"performance run collision at {run_dir}")
    try:
        sidecar_path = run_dir / PERFORMANCE_RESULT_NAME
        sidecar_path.write_text(text, encoding="utf-8")
        _lib.write_record(
            experiment, ts=ts, config_obj=config, metrics=document["summary"], verdict="complete", read=f"repeated performance evidence for EARP quality run {workload.parent_run_id}",
            code={"git_sha": cells[0].execution_provenance["candidate_sha"], "dirty": not cells[0].execution_provenance["clean"], "branch": "", "baseline_commit": None},
            corpus={"source": None, "manifest_sha256": None, "datasets": []}, seeds={},
            env={"python": str(cells[0].execution_provenance["toolchain"].get("python", "")), "lockfile_sha256": cells[0].execution_provenance["lockfile_sha256"], "gpu": cells[0].execution_provenance["device"], "key_deps": {}}, cost_usd=0.0,
            artifacts=[{"path": f"runs/{run_id}/{PERFORMANCE_RESULT_NAME}", "sha256": _file_sha(sidecar_path)}], base_dir=experiments_root, index_path=Path(experiments_root) / "index.jsonl")
        _lib.regen_index_md(index_path=Path(experiments_root) / "index.jsonl", md_path=Path(experiments_root) / "INDEX.md")
    except Exception:
        shutil.rmtree(run_dir, ignore_errors=True)
        raise
    return PerformanceWriteOutcome(run_id, run_dir)


def _validate_performance_document(document: Mapping[str, Any]) -> None:
    findings = validate(document, _PERFORMANCE_SCHEMA)
    if findings:
        raise PerformanceSchemaError(
            f"performance sidecar does not conform: {[finding.message for finding in findings][:4]}"
        )
    for name in ("parent_manifest", "inputs", "cells", "summary"):
        if name not in document:
            raise PerformanceSchemaError(f"performance sidecar lacks {name}")
    if not _sha(document["parent_manifest"].get("sha256")):
        raise PerformanceSchemaError("performance sidecar parent manifest digest is invalid")


def _invalidity_code(exc: Exception) -> str:
    text = f"{type(exc).__name__}: {exc}".lower()
    if "timeout" in text:
        return "timeout"
    return "execution_error"


def _bridge_provenance() -> dict[str, Any]:
    try:
        code = _lib.git_info()
    except Exception:
        code = {"git_sha": "unknown", "dirty": True}
    env = _lib.env_info()
    return {"candidate_sha": code["git_sha"], "clean": not code["dirty"], "command": "fathomdb-performance", "lockfile_sha256": env.get("lockfile_sha256") or "0" * 64, "toolchain": {"python": env.get("python", "")}, "device": {"kind": "cpu"}, "fixtures": {}}


def _sample_from_observed(treatment: str, repetition: int, observed: Mapping[str, Any], warm: bool) -> RunSample:
    phases, counts = observed.get("phases_ms"), observed.get("counts")
    if not isinstance(phases, Mapping) or not isinstance(counts, Mapping):
        raise RuntimeError("execution did not emit observed-cost evidence")
    return RunSample(treatment, repetition, phases, counts, {"fresh_database": True, "unmeasured_query_warmup": warm, "open_write_scope": "fresh_store"})


def run_diagnostic_repetitions(*, workload: WorkloadRef, plan: PerformancePlan, scenario: Any, config_doc: Mapping[str, Any], experiments_root: Path, experiment: str, ts: datetime) -> tuple[PerformanceCell, ...]:
    from eval.earp.runner import run_diagnostic
    from eval.earp.schema.models import RunVerdict
    if scenario.config_sha256 != workload.config_sha256 or scenario.query_call != workload.query_call:
        raise ValueError("diagnostic scenario does not match verified workload")
    provenance = _bridge_provenance()
    def execute(_workload: WorkloadRef, treatment: str, repetition: int) -> PerformanceCell:
        result = run_diagnostic(scenario=scenario, config_doc=config_doc, experiments_root=experiments_root, experiment=experiment, ts=ts, persist=False, warmup_query=treatment == "fresh_store_warm_query")
        raw = (_sample_from_observed(treatment, repetition, result.observed_cost, treatment == "fresh_store_warm_query"),) if result.observed_cost else ()
        if result.verdict is RunVerdict.COMPLETE:
            return PerformanceCell.complete(treatment=treatment, repetition=repetition, samples=raw, execution_provenance=provenance)
        return PerformanceCell.invalid(treatment=treatment, repetition=repetition, raw_samples=raw, invalidity={"code": _invalidity_code(RuntimeError(result.failure or str(result.blockers))), "message": result.failure or str(result.blockers)}, execution_provenance=provenance)
    return run_repetitions(workload=workload, plan=plan, execute=execute)


def run_characterization_repetitions(*, workload: WorkloadRef, plan: PerformancePlan, scenario: Any, config_doc: Mapping[str, Any]) -> tuple[PerformanceCell, ...]:
    from eval.earp.characterize import execute_arm
    corpus, gold = config_doc.get("corpus"), config_doc.get("gold")
    if not isinstance(corpus, Mapping) or not isinstance(gold, Mapping):
        raise ValueError("characterization config lacks corpus or gold identity")
    provenance = _bridge_provenance()
    def execute(_workload: WorkloadRef, treatment: str, repetition: int) -> PerformanceCell:
        arm = execute_arm(scenario=scenario, data_root=Path(str(corpus.get("data_root") or "")), snapshot_path=Path(str(corpus.get("snapshot") or "")), gold_path=Path(str(gold.get("path") or "")), gold_sha256=str(gold.get("sha256") or ""), corpus_hash=str(gold.get("corpus_hash") or ""), qrels_version=str(gold.get("qrels_version") or ""), warmup_queries=treatment == "fresh_store_warm_query")
        raw = (_sample_from_observed(treatment, repetition, arm.observed_cost, treatment == "fresh_store_warm_query"),) if arm.observed_cost else ()
        if arm.blocker is None:
            return PerformanceCell.complete(treatment=treatment, repetition=repetition, samples=raw, execution_provenance=provenance)
        return PerformanceCell.invalid(treatment=treatment, repetition=repetition, raw_samples=raw, invalidity={"code": arm.blocker.code.value, "message": arm.blocker.message}, execution_provenance=provenance)
    return run_repetitions(workload=workload, plan=plan, execute=execute)


def run_and_write_diagnostic_performance(**kwargs: Any) -> PerformanceWriteOutcome:
    cells = run_diagnostic_repetitions(**kwargs)
    return write_performance_result(experiments_root=kwargs["experiments_root"], experiment=kwargs["experiment"], ts=kwargs["ts"], workload=kwargs["workload"], plan=kwargs["plan"], cells=cells, execution_provenance=_bridge_provenance())


def run_and_write_characterization_performance(**kwargs: Any) -> PerformanceWriteOutcome:
    cells = run_characterization_repetitions(workload=kwargs["workload"], plan=kwargs["plan"], scenario=kwargs["scenario"], config_doc=kwargs["config_doc"])
    return write_performance_result(experiments_root=kwargs["experiments_root"], experiment=kwargs["experiment"], ts=kwargs["ts"], workload=kwargs["workload"], plan=kwargs["plan"], cells=cells, execution_provenance=_bridge_provenance())


def _nonnegative(values: Mapping[str, float | int], label: str) -> None:
    for name, value in values.items():
        if not name or isinstance(value, bool) or not isinstance(value, (int, float)) or value < 0:
            raise ValueError(f"{label} {name!r} must be non-negative")


__all__ = ["PERFORMANCE_RESULT_NAME", "PerformanceCell", "PerformanceCollision", "PerformancePlan", "PerformanceSchemaError", "PerformanceWriteOutcome", "RunSample", "SCHEMA_VERSION", "WorkloadRef", "load_earp_workload", "run_and_write_characterization_performance", "run_and_write_diagnostic_performance", "run_characterization_repetitions", "run_diagnostic_repetitions", "run_repetitions", "summarize_samples", "write_performance_result"]
