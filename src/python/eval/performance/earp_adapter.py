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

import yaml

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
    resolved_workload: Mapping[str, Any] = field(default_factory=dict)
    predeclared_plan: Mapping[str, Any] = field(default_factory=dict)
    resolved_config_document: Mapping[str, Any] = field(default_factory=dict)
    experiments_root: Path | None = field(default=None, compare=False, repr=False)

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
            "quality_parent": {
                "manifest_path": self.parent_manifest_path,
                "manifest_sha256": self.parent_manifest_sha256,
                "candidate_sha": self.candidate_sha,
                "clean": self.quality_clean,
            },
            "quality_result": {
                "path": self.quality_result_path,
                "sha256": self.quality_result_sha256,
            },
            "resolved_config": {
                "path": self.resolved_config_path,
                "sha256": self.resolved_config_sha256,
            },
            "workload": dict(self.resolved_workload) if self.resolved_workload else {
                "config_sha256": self.config_sha256,
                "query_call": self.query_call,
                "effective_knobs": dict(self.effective_knobs),
            },
            "performance_plan": dict(self.predeclared_plan),
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


def _relative_artifact_path(value: Any) -> bool:
    """Accept one contained relative artifact path, never an escape or URI."""
    if not isinstance(value, str) or not value or Path(value).is_absolute():
        return False
    return all(part not in {"", ".", ".."} for part in Path(value).parts)


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
    if not _relative_artifact_path(parent["result_path"]) or not _relative_artifact_path(resolved["path"]):
        raise ValueError("manifest schema is invalid")
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
    try:
        config_document = json.loads(resolved["canonical_json"])
        staged_config = json.loads(config_path.read_text(encoding="utf-8"))
    except (TypeError, json.JSONDecodeError):
        try:
            config_document = json.loads(resolved["canonical_json"])
            staged_config = yaml.safe_load(config_path.read_text(encoding="utf-8"))
        except (TypeError, json.JSONDecodeError, yaml.YAMLError) as exc:
            raise ValueError("manifest canonical config is invalid") from exc
    if not isinstance(staged_config, Mapping) or not isinstance(config_document, Mapping) or _lib.canonical_json(dict(staged_config)) != _lib.canonical_json(dict(config_document)):
        raise ValueError("staged config does not match manifest canonical config")
    if not isinstance(config_document, Mapping):
        raise ValueError("manifest schema is invalid")
    try:
        result_document = json.loads(result_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise ValueError("quality sidecar is invalid") from exc
    result_scenario = result_document.get("scenario") if isinstance(result_document, Mapping) else None
    if not isinstance(result_scenario, Mapping) or (
        result_scenario.get("config_sha256") != workload["config_sha256"]
        or result_scenario.get("query_call") != workload["query_call"]
        or result_scenario.get("effective_knobs", {}) != workload["effective_knobs"]
    ):
        raise ValueError("quality sidecar does not match manifest workload")
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
        resolved_workload=dict(workload),
        predeclared_plan=dict(manifest["performance_plan"]),
        resolved_config_document=dict(config_document),
        experiments_root=Path(experiments_root),
    )


def _verify_workload_reference(experiments_root: Path, workload: WorkloadRef) -> None:
    """Re-admit a loaded reference immediately before execution/publication."""
    verified = load_earp_workload(experiments_root, workload.parent_run_id)
    if verified.as_document() != workload.as_document() or verified.parent_manifest_sha256 != workload.parent_manifest_sha256:
        raise ValueError("manifest does not match the verified workload reference")


def _require_predeclared_plan(workload: WorkloadRef, plan: PerformancePlan) -> None:
    declared = workload.predeclared_plan
    if not declared:
        return
    if plan.repetitions != declared.get("repetitions") or list(plan.treatments) != declared.get("treatments"):
        raise ValueError("plan does not match the predeclared manifest plan")


def _scenario_value(scenario: Any, name: str) -> Any:
    value = getattr(scenario, name)
    if name == "query_params" and isinstance(value, Mapping):
        # Query text is quality input, not a performance knob; the admitted
        # workload records the runner's effective knobs without it.
        return {key: item for key, item in value.items() if key != "text"}
    return value.value if hasattr(value, "value") else value


def _require_diagnostic_scenario(workload: WorkloadRef, scenario: Any) -> None:
    expected = {
        "config_sha256": workload.config_sha256,
        "query_call": workload.query_call,
        "query_params": dict(workload.effective_knobs),
    }
    defaults = {
        "retrieval_mode": "text",
        "max_measurable_k": workload.effective_knobs.get("limit", 10),
        "use_default_embedder": True,
    }
    claim_capable = True
    for key, default in defaults.items():
        if key in workload.resolved_workload:
            expected[key] = workload.resolved_workload[key]
        elif claim_capable:
            # Claim-capable manifests must govern every normalized scenario
            # component, including values implicit in their legacy shape.
            expected[key] = default
    if any(not hasattr(scenario, key) for key in expected):
        raise ValueError("diagnostic scenario lacks verified workload identity")
    if any(_scenario_value(scenario, key) != value for key, value in expected.items()):
        raise ValueError("diagnostic scenario does not match verified workload")


def _admit_canonical_config(
    workload: WorkloadRef, config_doc: Mapping[str, Any]
) -> Mapping[str, Any]:
    """Use only the admitted config, while rejecting supplied contradictions."""
    canonical = dict(workload.resolved_config_document)
    supplied = dict(config_doc)
    if _lib.canonical_json(supplied) == _lib.canonical_json(canonical):
        return canonical
    # Legacy low-level bridge callers supplied either no config at all, or the
    # characterization identity subset.  They never control execution: after
    # checking every supplied member, execution still receives canonical bytes.
    if not supplied:
        return canonical
    allowed_subset = {"corpus", "gold", "projections", "embedder", "device"}
    if set(supplied).issubset(allowed_subset) and all(
        canonical.get(name) == value for name, value in supplied.items()
    ):
        return canonical
    raise ValueError("config does not match the verified workload")


def run_repetitions(*, workload: WorkloadRef, plan: PerformancePlan, execute: Callable[[WorkloadRef, str, int], RunSample | PerformanceCell], execution_provenance: Mapping[str, Any] | None = None) -> tuple[PerformanceCell, ...]:
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
                    execution_provenance=dict(execution_provenance or {}),
                ))
                continue
            if (returned.treatment, returned.repetition) != (treatment, repetition):
                raise ValueError("executor returned a sample for a different declared cell")
            if isinstance(returned, PerformanceCell):
                cells.append(returned)
            else:
                cells.append(PerformanceCell.complete(
                    treatment=treatment, repetition=repetition, samples=(returned,), execution_provenance=dict(execution_provenance or {})
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
    unavailable = value.get("unavailable", {}) if isinstance(value, Mapping) else {}
    if not isinstance(value, Mapping) or not isinstance(unavailable, Mapping):
        raise ValueError("execution provenance is incomplete")
    missing = [key for key in required if key not in value and key not in unavailable]
    if missing:
        raise ValueError("execution provenance is incomplete")
    for key in required:
        if key not in value:
            reason = unavailable[key]
            if not isinstance(reason, Mapping) or not isinstance(reason.get("code"), str) or not isinstance(reason.get("message"), str):
                raise ValueError("execution provenance is incomplete")
    if "candidate_sha" in value and (not isinstance(value["candidate_sha"], str) or not value["candidate_sha"]):
        raise ValueError("execution provenance candidate_sha is invalid")
    if "clean" in value and not isinstance(value["clean"], bool):
        raise ValueError("execution provenance is incomplete")
    if "command" in value and (not isinstance(value["command"], str) or not value["command"]):
        raise ValueError("execution provenance is incomplete")
    if "lockfile_sha256" in value and not _sha(value["lockfile_sha256"]):
        raise ValueError("execution provenance is incomplete")
    if "toolchain" in value and not isinstance(value["toolchain"], Mapping):
        raise ValueError("execution provenance is incomplete")
    if "fixtures" in value and not isinstance(value["fixtures"], Mapping):
        raise ValueError("execution provenance is incomplete")
    device = value.get("device")
    if device is not None and (not isinstance(device, Mapping) or device.get("kind") not in {"cpu", "cuda", "metal"}):
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
        if cell.status == "invalid":
            assert cell.invalidity is not None
            if not isinstance(cell.invalidity.get("code"), str) or not cell.invalidity["code"] or not isinstance(cell.invalidity.get("message"), str) or not cell.invalidity["message"]:
                raise ValueError("invalidity must have non-empty string code and message")
        for sample in cell.raw_samples:
            if (sample.treatment, sample.repetition) != (cell.treatment, cell.repetition):
                raise ValueError("raw sample does not match its cell identity")


def write_performance_result(*, experiments_root: Path, experiment: str, ts: datetime, workload: WorkloadRef, plan: PerformancePlan, samples: Sequence[RunSample] | None = None, cells: Sequence[PerformanceCell] | None = None, execution_provenance: Mapping[str, Any] | None = None) -> PerformanceWriteOutcome:
    """Validate then atomically stage a repeated-performance artifact and record."""
    if not workload.parent_manifest_path or not _sha(workload.parent_manifest_sha256):
        raise ValueError("manifest is required for repeated performance")
    _verify_workload_reference(Path(experiments_root), workload)
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
    _require_predeclared_plan(workload, plan)
    cross = any(
        (cell.execution_provenance or provenance or {}).get("candidate_sha") not in {None, workload.candidate_sha}
        for cell in cells
    )
    normalized_cells: list[PerformanceCell] = []
    for cell in cells:
        cell_provenance = _execution_provenance(cell.execution_provenance or provenance or {})
        if cell_provenance.get("candidate_sha") not in {None, workload.candidate_sha} and cell.status == "complete":
            cell = PerformanceCell.invalid(treatment=cell.treatment, repetition=cell.repetition, raw_samples=cell.raw_samples, invalidity={"code": "cross_candidate", "message": "execution candidate differs from quality candidate"}, execution_provenance=cell_provenance)
        normalized_cells.append(cell)
    cells = tuple(normalized_cells)
    complete_treatments = {
        treatment
        for treatment in plan.treatments
        if all(cell.status == "complete" for cell in cells if cell.treatment == treatment)
    }
    complete_samples = [
        sample
        for cell in cells
        if cell.status == "complete"
        for sample in cell.raw_samples
    ]
    eligible_samples = [sample for sample in complete_samples if sample.treatment in complete_treatments]
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
        "summary": summarize_samples(eligible_samples),
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
            code={"git_sha": cells[0].execution_provenance.get("candidate_sha"), "dirty": not bool(cells[0].execution_provenance.get("clean", False)), "branch": "", "baseline_commit": None},
            corpus={"source": None, "manifest_sha256": None, "datasets": []}, seeds={},
            env={"python": str(cells[0].execution_provenance.get("toolchain", {}).get("python", "")), "lockfile_sha256": cells[0].execution_provenance.get("lockfile_sha256"), "gpu": cells[0].execution_provenance.get("device"), "key_deps": {}}, cost_usd=0.0,
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
    parent = document["parent_manifest"]
    inputs = document["inputs"]
    if not isinstance(parent, Mapping) or not _relative_artifact_path(parent.get("path")):
        raise PerformanceSchemaError("performance sidecar parent manifest path is invalid")
    if not isinstance(inputs, Mapping):
        raise PerformanceSchemaError("performance sidecar inputs are invalid")
    for name in ("quality_result", "resolved_config"):
        item = inputs.get(name)
        if not isinstance(item, Mapping) or not _relative_artifact_path(item.get("path")) or not _sha(item.get("sha256")):
            raise PerformanceSchemaError("performance sidecar input path is invalid")


def _invalidity_code(exc: Exception) -> str:
    text = f"{type(exc).__name__}: {exc}".lower()
    if "timeout" in text:
        return "timeout"
    return "execution_error"


def _bridge_provenance() -> dict[str, Any]:
    try:
        code = _lib.git_info()
    except Exception:
        code = {}
    env = _lib.env_info()
    unavailable: dict[str, dict[str, str]] = {
        "command": {"code": "not_captured", "message": "the bridge invocation command was not captured"},
        "device": {"code": "not_captured", "message": "the bridge did not capture the execution device"},
        "fixtures": {"code": "not_captured", "message": "the bridge did not capture fixture identities"},
    }
    result: dict[str, Any] = {}
    candidate = code.get("git_sha")
    if isinstance(candidate, str) and candidate and candidate not in {"unknown", "unavailable-outside-git"}:
        result["candidate_sha"] = candidate
        result["clean"] = not bool(code.get("dirty"))
    else:
        unavailable["candidate_sha"] = {"code": "git_unavailable", "message": "git provenance is unavailable"}
        unavailable["clean"] = {"code": "git_unavailable", "message": "git cleanliness is unavailable"}
    if env.get("lockfile_sha256"):
        result["lockfile_sha256"] = env["lockfile_sha256"]
    else:
        unavailable["lockfile_sha256"] = {"code": "lockfile_absent", "message": "no lockfile digest is available"}
    python = env.get("python")
    if isinstance(python, str) and python:
        result["toolchain"] = {"python": python}
    else:
        unavailable["toolchain"] = {"code": "not_captured", "message": "the bridge did not capture the Python toolchain"}
    result["unavailable"] = unavailable
    return result


def _bridge_execution_provenance(
    provenance: Mapping[str, Any],
    *,
    workload: WorkloadRef,
    plan: PerformancePlan,
    config_doc: Mapping[str, Any],
) -> dict[str, Any]:
    """Add bridge-captured facts without overwriting typed unavailability."""
    result = dict(provenance)
    unavailable = dict(result.get("unavailable", {}))

    def replace_not_captured(name: str, value: Any) -> None:
        reason = unavailable.get(name)
        if isinstance(reason, Mapping) and reason.get("code") == "not_captured":
            unavailable.pop(name)
            result[name] = value

    replace_not_captured(
        "command",
        " ".join(
            (
                "fathomdb-performance",
                str(config_doc.get("campaign", "unknown")),
                "--quality-run",
                workload.parent_run_id,
                "--repetitions",
                str(plan.repetitions),
                "--treatments",
                ",".join(plan.treatments),
            )
        ),
    )
    device = config_doc.get("device")
    if isinstance(device, Mapping) and device.get("kind") in {"cpu", "cuda", "metal"}:
        replace_not_captured("device", dict(device))
    else:
        replace_not_captured("device", {"kind": "cpu", "source": "Engine.open default"})
    fixture_identity: dict[str, str] = {"quality_manifest_sha256": workload.parent_manifest_sha256}
    scenario = config_doc.get("scenario")
    if isinstance(scenario, Mapping) and isinstance(scenario.get("fixture"), str):
        fixture = Path(scenario["fixture"])
        if fixture.is_file():
            fixture_identity["scenario_fixture_sha256"] = _file_sha(fixture)
    corpus = config_doc.get("corpus")
    if isinstance(corpus, Mapping) and isinstance(corpus.get("snapshot"), str):
        fixture_identity["corpus_snapshot"] = str(corpus["snapshot"])
    replace_not_captured("fixtures", fixture_identity)
    lockfile = Path.cwd() / "Cargo.lock"
    lockfile_reason = unavailable.get("lockfile_sha256")
    if lockfile.is_file() and isinstance(lockfile_reason, Mapping) and lockfile_reason.get("code") == "lockfile_absent":
        unavailable.pop("lockfile_sha256")
        result["lockfile_sha256"] = _file_sha(lockfile)
    if unavailable:
        result["unavailable"] = unavailable
    else:
        result.pop("unavailable", None)
    return result


def _bridge_provenance_is_complete(provenance: Mapping[str, Any]) -> bool:
    return not bool(provenance.get("unavailable"))


def _sample_from_observed(treatment: str, repetition: int, observed: Mapping[str, Any], warm: bool) -> RunSample:
    phases, counts = observed.get("phases_ms"), observed.get("counts")
    if not isinstance(phases, Mapping) or not isinstance(counts, Mapping):
        raise RuntimeError("execution did not emit observed-cost evidence")
    return RunSample(treatment, repetition, phases, counts, {"fresh_database": True, "unmeasured_query_warmup": warm, "open_write_scope": "fresh_store"})


def run_diagnostic_repetitions(*, workload: WorkloadRef, plan: PerformancePlan, scenario: Any, config_doc: Mapping[str, Any], experiments_root: Path, experiment: str, ts: datetime) -> tuple[PerformanceCell, ...]:
    from eval.earp.runner import run_diagnostic
    from eval.earp.schema.models import RunVerdict
    _verify_workload_reference(Path(experiments_root), workload)
    config_doc = _admit_canonical_config(workload, config_doc)
    _require_diagnostic_scenario(workload, scenario)
    _require_predeclared_plan(workload, plan)
    provenance = _bridge_provenance()
    def execute(_workload: WorkloadRef, treatment: str, repetition: int) -> PerformanceCell:
        result = run_diagnostic(scenario=scenario, config_doc=config_doc, experiments_root=experiments_root, experiment=experiment, ts=ts, persist=False, warmup_query=treatment == "fresh_store_warm_query")
        raw = (_sample_from_observed(treatment, repetition, result.observed_cost, treatment == "fresh_store_warm_query"),) if result.observed_cost else ()
        if result.verdict is RunVerdict.COMPLETE and _bridge_provenance_is_complete(provenance):
            return PerformanceCell.complete(treatment=treatment, repetition=repetition, samples=raw, execution_provenance=provenance)
        if result.verdict is RunVerdict.COMPLETE:
            return PerformanceCell.invalid(treatment=treatment, repetition=repetition, raw_samples=raw, invalidity={"code": "provenance_unavailable", "message": "bridge provenance is typed unavailable"}, execution_provenance=provenance)
        return PerformanceCell.invalid(treatment=treatment, repetition=repetition, raw_samples=raw, invalidity={"code": _invalidity_code(RuntimeError(result.failure or str(result.blockers))), "message": result.failure or str(result.blockers)}, execution_provenance=provenance)
    return run_repetitions(workload=workload, plan=plan, execute=execute, execution_provenance=provenance)


def run_characterization_repetitions(*, workload: WorkloadRef, plan: PerformancePlan, scenario: Any, config_doc: Mapping[str, Any], experiments_root: Path | None = None) -> tuple[PerformanceCell, ...]:
    from eval.earp.characterize import execute_arm
    root = Path(experiments_root) if experiments_root is not None else workload.experiments_root
    if root is None:
        raise ValueError("characterization requires the verified experiments root")
    _verify_workload_reference(root, workload)
    config_doc = _admit_canonical_config(workload, config_doc)
    _require_diagnostic_scenario(workload, scenario)
    corpus, gold = config_doc.get("corpus"), config_doc.get("gold")
    if not isinstance(corpus, Mapping) or not isinstance(gold, Mapping):
        raise ValueError("characterization config lacks corpus or gold identity")
    _require_predeclared_plan(workload, plan)
    if any(
        not hasattr(scenario, key)
        for key in ("config_sha256", "query_call", "query_params")
    ):
        raise ValueError("characterization scenario lacks verified workload identity")
    actual = {
        "config_sha256": _scenario_value(scenario, "config_sha256"),
        "query_call": _scenario_value(scenario, "query_call"),
        "corpus": dict(corpus),
        "gold": dict(gold),
        "projections": config_doc.get("projections", {}),
        "embedder": config_doc.get("embedder", {}),
        "device": config_doc.get("device", {"kind": "cpu"}),
        "effective_knobs": _scenario_value(scenario, "query_params"),
    }
    expected = workload.resolved_workload
    for key, value in actual.items():
        if key not in expected or expected[key] != value:
            raise ValueError("characterization inputs do not match verified workload")
    provenance = _bridge_provenance()
    def execute(_workload: WorkloadRef, treatment: str, repetition: int) -> PerformanceCell:
        arm = execute_arm(scenario=scenario, data_root=Path(str(corpus.get("data_root") or "")), snapshot_path=Path(str(corpus.get("snapshot") or "")), gold_path=Path(str(gold.get("path") or "")), gold_sha256=str(gold.get("sha256") or ""), corpus_hash=str(gold.get("corpus_hash") or ""), qrels_version=str(gold.get("qrels_version") or ""), warmup_queries=treatment == "fresh_store_warm_query")
        raw = (_sample_from_observed(treatment, repetition, arm.observed_cost, treatment == "fresh_store_warm_query"),) if arm.observed_cost else ()
        if arm.blocker is None and _bridge_provenance_is_complete(provenance):
            return PerformanceCell.complete(treatment=treatment, repetition=repetition, samples=raw, execution_provenance=provenance)
        if arm.blocker is None:
            return PerformanceCell.invalid(treatment=treatment, repetition=repetition, raw_samples=raw, invalidity={"code": "provenance_unavailable", "message": "bridge provenance is typed unavailable"}, execution_provenance=provenance)
        return PerformanceCell.invalid(treatment=treatment, repetition=repetition, raw_samples=raw, invalidity={"code": arm.blocker.code.value, "message": arm.blocker.message}, execution_provenance=provenance)
    return run_repetitions(workload=workload, plan=plan, execute=execute, execution_provenance=provenance)


def run_and_write_diagnostic_performance(**kwargs: Any) -> PerformanceWriteOutcome:
    cells = run_diagnostic_repetitions(**kwargs)
    return write_performance_result(experiments_root=kwargs["experiments_root"], experiment=kwargs["experiment"], ts=kwargs["ts"], workload=kwargs["workload"], plan=kwargs["plan"], cells=cells, execution_provenance=_bridge_provenance())


def run_and_write_characterization_performance(**kwargs: Any) -> PerformanceWriteOutcome:
    cells = run_characterization_repetitions(workload=kwargs["workload"], plan=kwargs["plan"], scenario=kwargs["scenario"], config_doc=kwargs["config_doc"], experiments_root=kwargs["experiments_root"])
    return write_performance_result(experiments_root=kwargs["experiments_root"], experiment=kwargs["experiment"], ts=kwargs["ts"], workload=kwargs["workload"], plan=kwargs["plan"], cells=cells, execution_provenance=_bridge_provenance())


def _nonnegative(values: Mapping[str, float | int], label: str) -> None:
    for name, value in values.items():
        if not name or isinstance(value, bool) or not isinstance(value, (int, float)) or value < 0:
            raise ValueError(f"{label} {name!r} must be non-negative")


__all__ = ["PERFORMANCE_RESULT_NAME", "PerformanceCell", "PerformanceCollision", "PerformancePlan", "PerformanceSchemaError", "PerformanceWriteOutcome", "RunSample", "SCHEMA_VERSION", "WorkloadRef", "load_earp_workload", "run_and_write_characterization_performance", "run_and_write_diagnostic_performance", "run_characterization_repetitions", "run_diagnostic_repetitions", "run_repetitions", "summarize_samples", "write_performance_result"]
