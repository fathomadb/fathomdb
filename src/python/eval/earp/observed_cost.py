"""One-run cost evidence emitted beside an EARP quality result.

This module deliberately records observations, not benchmark conclusions.
Repeated sampling, percentiles, and support claims belong to
``eval.performance``. Keeping the distinction in types prevents a single EARP
execution from being accidentally presented as p95 or QPS evidence.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Mapping

OBSERVED_COST_NAME = "earp.observed-cost.v2.json"
SCHEMA_VERSION = "earp.observed-cost.v2"


def capture_sqlite_storage(database_path: Path) -> dict[str, int]:
    """Return byte counts for SQLite's main database and adjacent WAL files.

    The values are a checkpoint observation only: an absent WAL or SHM file is
    recorded as zero rather than inferred to mean that no write occurred.
    """
    path = Path(database_path)
    return {
        "database_bytes": _byte_size(path),
        "wal_bytes": _byte_size(Path(f"{path}-wal")),
        "shm_bytes": _byte_size(Path(f"{path}-shm")),
    }


def _byte_size(path: Path) -> int:
    return path.stat().st_size if path.is_file() else 0


def combine_arm_observations(
    *, config_sha256: str, arms: Mapping[str, Mapping[str, Any]]
) -> dict[str, Any]:
    """Flatten per-arm observations without pooling their timings.

    The arm prefix is intentional: an aggregate comparison has no truthful
    single ``query`` duration. Consumers must select an arm explicitly rather
    than treating control and treatment samples as one distribution.
    """
    phases: dict[str, float] = {}
    counts: dict[str, int] = {}
    storage: dict[str, int] = {}
    for arm, document in arms.items():
        for source, target in (
            (document.get("phases_ms"), phases),
            (document.get("counts"), counts),
            (document.get("storage"), storage),
        ):
            if not isinstance(source, Mapping):
                continue
            for name, value in source.items():
                target[f"arm.{arm}.{name}"] = value
    return Observation(
        evidence_family_id="pending-writer-binding",
        config_sha256=config_sha256,
        phases_ms=phases,
        counts=counts,
        storage=storage,
    ).as_document()


@dataclass(frozen=True)
class Observation:
    """The durable one-run observed-cost contract for an EARP execution."""

    evidence_family_id: str
    config_sha256: str
    phases_ms: Mapping[str, float]
    counts: Mapping[str, int]
    storage: Mapping[str, int]
    query_samples: tuple[Mapping[str, Any], ...] = ()
    unavailable: Mapping[str, Mapping[str, str]] = field(default_factory=dict)
    provenance: Mapping[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if not self.evidence_family_id:
            raise ValueError("evidence_family_id must be non-empty")
        if len(self.config_sha256) != 64:
            raise ValueError("config_sha256 must be a full SHA-256")
        _validate_nonnegative(self.phases_ms, "phase")
        _validate_nonnegative(self.counts, "count")
        _validate_nonnegative(self.storage, "storage")
        unavailable = self.unavailable
        provenance = self.provenance
        if not isinstance(unavailable, Mapping) or not isinstance(provenance, Mapping):
            raise ValueError("unavailable and provenance must be mappings")
        for field_name, reason in unavailable.items():
            if not field_name or not isinstance(reason, Mapping):
                raise ValueError("unavailable fields need typed reasons")
            if not isinstance(reason.get("code"), str) or not isinstance(reason.get("message"), str):
                raise ValueError("unavailable reason needs code and message")
        for sample in self.query_samples:
            if not isinstance(sample, Mapping) or not isinstance(sample.get("query_id"), str):
                raise ValueError("query_samples require query_id")
            if sample.get("outcome") not in {"complete", "failed"}:
                raise ValueError("query_samples require a complete or failed outcome")
            if sample.get("outcome") == "complete":
                _validate_nonnegative({"wall_ms": sample.get("wall_ms")}, "query sample")
                _validate_nonnegative({"result_count": sample.get("result_count")}, "query sample")
        candidate = provenance.get("candidate_sha")
        if candidate is not None and (not isinstance(candidate, str) or not candidate):
            raise ValueError("provenance candidate_sha must be non-empty when present")

    def as_document(self) -> dict[str, Any]:
        """Return the versioned, JSON-ready sidecar document."""
        document = {
            "schema_version": SCHEMA_VERSION,
            "scope": "one_run_observation",
            "evidence_family_id": self.evidence_family_id,
            "config_sha256": self.config_sha256,
            "phases_ms": {key: float(value) for key, value in self.phases_ms.items()},
            "counts": {key: int(value) for key, value in self.counts.items()},
            "storage": {key: int(value) for key, value in self.storage.items()},
            "query_samples": [dict(sample) for sample in self.query_samples],
            "unavailable": {key: dict(value) for key, value in self.unavailable.items()},
            "provenance": dict(self.provenance),
        }
        return document


def _validate_nonnegative(values: Mapping[str, Any], label: str) -> None:
    for name, value in values.items():
        if not name:
            raise ValueError(f"{label} name must be non-empty")
        if isinstance(value, bool) or not isinstance(value, (int, float)) or value < 0:
            raise ValueError(f"{label} {name!r} must be a non-negative number")


__all__ = [
    "OBSERVED_COST_NAME",
    "Observation",
    "SCHEMA_VERSION",
    "capture_sqlite_storage",
    "combine_arm_observations",
]
