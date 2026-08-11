"""Independent repeated-performance artifacts linked to an EARP workload.

The adapter owns repeated samples and percentile summaries. It accepts the
already-resolved EARP workload identity rather than a second hand-authored set
of benchmark knobs, preserving EARP's retrieval-evaluation boundary.
"""

from __future__ import annotations

import json
import math
import statistics
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any, Callable, Mapping, Sequence

from eval.earp._experiments import lib as _lib

PERFORMANCE_RESULT_NAME = "performance.earp.v1.json"
SCHEMA_VERSION = "performance.earp.v1"
_EARP_RESULT_NAME = "earp.result.v1.json"


@dataclass(frozen=True)
class WorkloadRef:
    """Identity shared by a quality run and its performance evidence."""

    parent_run_id: str
    evidence_family_id: str
    config_sha256: str
    candidate_sha: str
    query_call: str
    effective_knobs: Mapping[str, Any]

    def __post_init__(self) -> None:
        if not self.parent_run_id or not self.evidence_family_id or not self.query_call:
            raise ValueError("workload identifiers and query_call must be non-empty")
        if len(self.config_sha256) != 64:
            raise ValueError("config_sha256 must be a full SHA-256")
        if not self.candidate_sha:
            raise ValueError("candidate_sha must be non-empty")

    def as_document(self) -> dict[str, Any]:
        """Return the immutable link back to the EARP quality workload."""
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
    """Declared treatments and repetitions for a performance run."""

    repetitions: int
    treatments: tuple[str, ...]

    def __post_init__(self) -> None:
        if self.repetitions < 1:
            raise ValueError("repetitions must be >= 1")
        if not self.treatments or len(set(self.treatments)) != len(self.treatments):
            raise ValueError("treatments must be non-empty and unique")
        if any(not treatment for treatment in self.treatments):
            raise ValueError("treatment names must be non-empty")

    def as_document(self) -> dict[str, Any]:
        return {"repetitions": self.repetitions, "treatments": list(self.treatments)}


@dataclass(frozen=True)
class RunSample:
    """One completed treatment repetition with raw phase timings."""

    treatment: str
    repetition: int
    phases_ms: Mapping[str, float]
    counts: Mapping[str, int]

    def __post_init__(self) -> None:
        if not self.treatment or self.repetition < 0:
            raise ValueError("treatment must be non-empty and repetition >= 0")
        _validate_nonnegative(self.phases_ms, "phase")
        _validate_nonnegative(self.counts, "count")

    def as_document(self) -> dict[str, Any]:
        return {
            "treatment": self.treatment,
            "repetition": self.repetition,
            "phases_ms": {key: float(value) for key, value in self.phases_ms.items()},
            "counts": {key: int(value) for key, value in self.counts.items()},
        }


@dataclass(frozen=True)
class PerformanceWriteOutcome:
    """Location of one durable repeated-performance artifact."""

    run_id: str
    run_dir: Path


def load_earp_workload(experiments_root: Path, quality_run_id: str) -> WorkloadRef:
    """Load the immutable workload reference from completed EARP artifacts.

    The adapter intentionally reads both the result sidecar (the resolved
    query contract) and shared record (candidate provenance). A missing value
    is a refusal: guessing a SHA or retyping knobs would break the one
    declaration invariant.
    """
    run_dir = Path(experiments_root) / "runs" / quality_run_id
    try:
        record = json.loads((run_dir / "record.json").read_text(encoding="utf-8"))
        result = json.loads((run_dir / _EARP_RESULT_NAME).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f"cannot read EARP quality artifacts for {quality_run_id}: {exc}") from exc
    code = record.get("code")
    scenario = result.get("scenario")
    if not isinstance(code, Mapping) or not isinstance(scenario, Mapping):
        raise ValueError("EARP artifacts lack code or scenario mappings")
    candidate_sha = code.get("git_sha")
    if not isinstance(candidate_sha, str) or not candidate_sha:
        raise ValueError("EARP quality artifact lacks a candidate SHA")
    config_sha256 = scenario.get("config_sha256")
    query_call = scenario.get("query_call")
    if not isinstance(config_sha256, str) or not isinstance(query_call, str):
        raise ValueError("EARP quality artifact lacks resolved scenario identity")
    # `effective_knobs` is being introduced under `scenario`; accept the
    # short-lived root staging shape as input too so an interrupted writer can
    # be diagnosed rather than silently losing the knobs it already recorded.
    knobs = scenario.get("effective_knobs", result.get("effective_knobs", {}))
    if not isinstance(knobs, Mapping):
        raise ValueError("EARP effective_knobs must be a mapping")
    effective_knobs = dict(knobs)
    fanout = scenario.get("fanout_used")
    if isinstance(fanout, int) and not isinstance(fanout, bool):
        effective_knobs.setdefault("limit", fanout)
    return WorkloadRef(
        parent_run_id=quality_run_id,
        evidence_family_id=quality_run_id,
        config_sha256=config_sha256,
        candidate_sha=candidate_sha,
        query_call=query_call,
        effective_knobs=effective_knobs,
    )


def run_repetitions(
    *,
    workload: WorkloadRef,
    plan: PerformancePlan,
    execute: Callable[[WorkloadRef, str, int], RunSample],
) -> tuple[RunSample, ...]:
    """Execute every declared cell once and reject a substituted workload cell.

    The caller supplies the actual database workload executor. This layer owns
    only the invariant that a timing cell is the treatment/repetition it
    claims, rather than a warm sample relabelled as fresh-store or vice versa.
    """
    samples: list[RunSample] = []
    for treatment in plan.treatments:
        for repetition in range(plan.repetitions):
            sample = execute(workload, treatment, repetition)
            if (sample.treatment, sample.repetition) != (treatment, repetition):
                raise ValueError(
                    "executor returned a sample for a different declared cell: "
                    f"expected {(treatment, repetition)}, got "
                    f"{(sample.treatment, sample.repetition)}"
                )
            samples.append(sample)
    return tuple(samples)


def run_diagnostic_repetitions(
    *,
    workload: WorkloadRef,
    plan: PerformancePlan,
    scenario: Any,
    config_doc: Mapping[str, Any],
    experiments_root: Path,
    experiment: str,
    ts: datetime,
) -> tuple[RunSample, ...]:
    """Run a resolved diagnostic workload without writing duplicate quality runs.

    ``fresh_store`` opens a new temporary engine/database for the cell in this
    process. It is deliberately not called process-cold: proving process and
    OS-cache coldness needs a subprocess executor and explicit cache witness.
    ``warm`` performs one unmeasured query after that setup before the timed
    query. The diagnostic fixture is intentionally small; corpus-scale warm
    suites remain the performance protocol's separate executor.
    """
    from eval.earp.runner import run_diagnostic  # noqa: PLC0415 -- avoids cycle
    from eval.earp.schema.models import RunVerdict  # noqa: PLC0415

    if scenario.config_sha256 != workload.config_sha256:
        raise ValueError("diagnostic scenario does not match workload config SHA")
    if scenario.query_call != workload.query_call:
        raise ValueError("diagnostic scenario does not match workload query call")
    if any(treatment not in {"fresh_store", "warm"} for treatment in plan.treatments):
        raise ValueError("diagnostic bridge supports only fresh_store and warm treatments")

    def execute(_workload: WorkloadRef, treatment: str, repetition: int) -> RunSample:
        result = run_diagnostic(
            scenario=scenario,
            config_doc=config_doc,
            experiments_root=experiments_root,
            experiment=experiment,
            ts=ts,
            persist=False,
            warmup_query=treatment == "warm",
        )
        if result.verdict is not RunVerdict.COMPLETE:
            raise RuntimeError(
                f"diagnostic performance cell {(treatment, repetition)} did not complete: "
                f"{result.failure or result.blockers}"
            )
        observed = result.observed_cost
        phases = observed.get("phases_ms")
        counts = observed.get("counts")
        if not isinstance(phases, Mapping) or not isinstance(counts, Mapping):
            raise RuntimeError("diagnostic execution did not emit observed-cost evidence")
        return RunSample(treatment, repetition, phases, counts)

    return run_repetitions(workload=workload, plan=plan, execute=execute)


def run_and_write_diagnostic_performance(
    *,
    workload: WorkloadRef,
    plan: PerformancePlan,
    scenario: Any,
    config_doc: Mapping[str, Any],
    experiments_root: Path,
    experiment: str,
    ts: datetime,
) -> PerformanceWriteOutcome:
    """Execute diagnostic repetitions, then publish one independent artifact."""
    samples = run_diagnostic_repetitions(
        workload=workload,
        plan=plan,
        scenario=scenario,
        config_doc=config_doc,
        experiments_root=experiments_root,
        experiment=experiment,
        ts=ts,
    )
    return write_performance_result(
        experiments_root=experiments_root,
        experiment=experiment,
        ts=ts,
        workload=workload,
        plan=plan,
        samples=samples,
    )


def run_characterization_repetitions(
    *,
    workload: WorkloadRef,
    plan: PerformancePlan,
    scenario: Any,
    config_doc: Mapping[str, Any],
) -> tuple[RunSample, ...]:
    """Repeat the resolved corpus-scale EARP workload without quality writes."""
    from eval.earp.characterize import execute_arm  # noqa: PLC0415 -- avoids cycle

    if scenario.config_sha256 != workload.config_sha256:
        raise ValueError("characterization scenario does not match workload config SHA")
    if scenario.query_call != workload.query_call:
        raise ValueError("characterization scenario does not match workload query call")
    corpus = config_doc.get("corpus")
    gold = config_doc.get("gold")
    if not isinstance(corpus, Mapping) or not isinstance(gold, Mapping):
        raise ValueError("characterization config lacks corpus or gold identity")
    if any(treatment not in {"fresh_store", "warm"} for treatment in plan.treatments):
        raise ValueError("characterization bridge supports only fresh_store and warm")

    def execute(_workload: WorkloadRef, treatment: str, repetition: int) -> RunSample:
        arm = execute_arm(
            scenario=scenario,
            data_root=Path(str(corpus.get("data_root") or "")),
            snapshot_path=Path(str(corpus.get("snapshot") or "")),
            gold_path=Path(str(gold.get("path") or "")),
            gold_sha256=str(gold.get("sha256") or ""),
            corpus_hash=str(gold.get("corpus_hash") or ""),
            qrels_version=str(gold.get("qrels_version") or ""),
            warmup_queries=treatment == "warm",
        )
        if arm.blocker is not None:
            raise RuntimeError(
                f"characterization performance cell {(treatment, repetition)} blocked: "
                f"{arm.blocker.message}"
            )
        observed = arm.observed_cost
        phases = observed.get("phases_ms")
        counts = observed.get("counts")
        if not isinstance(phases, Mapping) or not isinstance(counts, Mapping):
            raise RuntimeError("characterization execution did not emit observed-cost evidence")
        return RunSample(treatment, repetition, phases, counts)

    return run_repetitions(workload=workload, plan=plan, execute=execute)


def run_and_write_characterization_performance(
    *,
    workload: WorkloadRef,
    plan: PerformancePlan,
    scenario: Any,
    config_doc: Mapping[str, Any],
    experiments_root: Path,
    experiment: str,
    ts: datetime,
) -> PerformanceWriteOutcome:
    """Execute corpus-scale repetitions, then publish one independent artifact.

    This deliberately uses the same pure arm executor that EARP comparisons
    use. It creates no extra quality result, per-query sidecar, or comparison
    claim while gathering the timing distribution.
    """
    samples = run_characterization_repetitions(
        workload=workload,
        plan=plan,
        scenario=scenario,
        config_doc=config_doc,
    )
    return write_performance_result(
        experiments_root=experiments_root,
        experiment=experiment,
        ts=ts,
        workload=workload,
        plan=plan,
        samples=samples,
    )


def summarize_samples(
    samples: Sequence[RunSample],
) -> dict[str, dict[str, dict[str, float | int]]]:
    """Summarize each treatment/phase without pooling treatments.

    The percentile rule is nearest-rank, which is deterministic and leaves raw
    samples in the sidecar for later reanalysis.
    """
    grouped: dict[str, dict[str, list[float]]] = {}
    for sample in samples:
        phases = grouped.setdefault(sample.treatment, {})
        for phase, value in sample.phases_ms.items():
            phases.setdefault(phase, []).append(float(value))
    return {
        treatment: {
            phase: _summary(values)
            for phase, values in sorted(phases.items())
        }
        for treatment, phases in sorted(grouped.items())
    }


def _summary(values: Sequence[float]) -> dict[str, float | int]:
    ordered = sorted(values)
    count = len(ordered)
    if not count:
        raise ValueError("cannot summarize an empty sample set")
    return {
        "n": count,
        "min_ms": ordered[0],
        "max_ms": ordered[-1],
        "mean_ms": sum(ordered) / count,
        # p50 is the conventional median, including the midpoint between the
        # two central samples for an even-sized set. High percentiles retain a
        # nearest-rank rule, and the raw samples remain durable.
        "p50_ms": float(statistics.median(ordered)),
        "p95_ms": _nearest_rank(ordered, 0.95),
        "p99_ms": _nearest_rank(ordered, 0.99),
    }


def _nearest_rank(ordered: Sequence[float], quantile: float) -> float:
    index = max(0, math.ceil(quantile * len(ordered)) - 1)
    return ordered[index]


def write_performance_result(
    *,
    experiments_root: Path,
    experiment: str,
    ts: datetime,
    workload: WorkloadRef,
    plan: PerformancePlan,
    samples: Sequence[RunSample],
) -> PerformanceWriteOutcome:
    """Stage a performance sidecar before materializing/indexing its record."""
    _validate_plan_samples(plan, samples)
    root = Path(experiments_root)
    config = {
        "schema_version": SCHEMA_VERSION,
        "workload": workload.as_document(),
        "plan": plan.as_document(),
    }
    sha = _lib.config_sha256(config)
    run_id = _lib.make_run_id(experiment, ts, sha)
    run_dir = root / "runs" / run_id
    document = {
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "workload": workload.as_document(),
        "plan": plan.as_document(),
        "samples": [sample.as_document() for sample in samples],
        "summary": summarize_samples(samples),
        "scope": "repeated_performance_characterization",
    }
    try:
        run_dir.mkdir(parents=True, exist_ok=False)
    except FileExistsError as exc:
        raise FileExistsError(f"performance run collision at {run_dir}") from exc
    (run_dir / PERFORMANCE_RESULT_NAME).write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    _lib.write_record(
        experiment,
        ts=ts,
        config_obj=config,
        metrics=document["summary"],
        verdict="complete",
        read=(
            f"repeated performance evidence for EARP quality run {workload.parent_run_id}"
        ),
        code={
            "git_sha": workload.candidate_sha,
            "dirty": False,
            "branch": "",
            "baseline_commit": None,
        },
        corpus={"source": None, "manifest_sha256": None, "datasets": []},
        seeds={},
        env={"python": "", "lockfile_sha256": None, "gpu": None, "key_deps": {}},
        cost_usd=0.0,
        artifacts=[f"runs/{run_id}/{PERFORMANCE_RESULT_NAME}"],
        base_dir=root,
        index_path=root / "index.jsonl",
    )
    _lib.regen_index_md(index_path=root / "index.jsonl", md_path=root / "INDEX.md")
    return PerformanceWriteOutcome(run_id=run_id, run_dir=run_dir)


def _validate_plan_samples(plan: PerformancePlan, samples: Sequence[RunSample]) -> None:
    expected = {(treatment, repetition) for treatment in plan.treatments for repetition in range(plan.repetitions)}
    actual = {(sample.treatment, sample.repetition) for sample in samples}
    if actual != expected:
        raise ValueError(
            f"samples must contain exactly the declared treatment/repetition matrix: "
            f"expected {sorted(expected)}, got {sorted(actual)}"
        )


def _validate_nonnegative(values: Mapping[str, float | int], label: str) -> None:
    for name, value in values.items():
        if not name:
            raise ValueError(f"{label} name must be non-empty")
        if isinstance(value, bool) or not isinstance(value, (int, float)) or value < 0:
            raise ValueError(f"{label} {name!r} must be a non-negative number")


__all__ = [
    "PERFORMANCE_RESULT_NAME",
    "PerformancePlan",
    "PerformanceWriteOutcome",
    "RunSample",
    "SCHEMA_VERSION",
    "WorkloadRef",
    "load_earp_workload",
    "run_diagnostic_repetitions",
    "run_and_write_diagnostic_performance",
    "run_and_write_characterization_performance",
    "run_characterization_repetitions",
    "run_repetitions",
    "summarize_samples",
    "write_performance_result",
]
