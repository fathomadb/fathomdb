"""Safe synthetic lifecycle projection sidecar for the TRACE-01 canary."""

from __future__ import annotations

import hashlib
import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Mapping


TRACE_PROJECTION_V1 = "trace-projection.v1"
_SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
_PROJECTION_KINDS = frozenset({"text", "vector_child", "summary", "extracted_fact", "entity", "edge"})


class TraceProjectionError(ValueError):
    """Raised when a synthetic TRACE-01 lifecycle input is unsafe or incomplete."""


@dataclass(frozen=True)
class Source:
    """A canonical source registered for a payload-free lifecycle trace."""

    source_id: str
    source_sha256: str


@dataclass(frozen=True)
class Projection:
    """A derived retrieval row attributable to one registered source."""

    projection_id: str
    source_id: str
    source_sha256: str
    kind: str


@dataclass(frozen=True)
class LifecycleEvent:
    """A source erasure or re-open event in the synthetic lifecycle trace."""

    kind: str
    source_id: str


def _require_identifier(value: object, field: str) -> str:
    if not isinstance(value, str) or not value:
        raise TraceProjectionError(f"{field} must be a non-empty identifier")
    return value


def _require_sha256(value: object, field: str) -> str:
    if not isinstance(value, str) or _SHA256_RE.fullmatch(value) is None:
        raise TraceProjectionError(f"{field} must be a lowercase SHA-256 hex value")
    return value


def _source_map(sources: Iterable[Source]) -> dict[str, Source]:
    registered: dict[str, Source] = {}
    for source in sources:
        source_id = _require_identifier(source.source_id, "source_id")
        _require_sha256(source.source_sha256, "source_sha256")
        if source_id in registered:
            raise TraceProjectionError("duplicate registered source identifier")
        registered[source_id] = source
    if not registered:
        raise TraceProjectionError("at least one registered source is required")
    return registered


def _validate_projections(
    projections: Iterable[Projection], sources: Mapping[str, Source]
) -> list[Projection]:
    rows: list[Projection] = []
    owners: dict[str, str] = {}
    for projection in projections:
        projection_id = _require_identifier(projection.projection_id, "projection_id")
        source_id = _require_identifier(projection.source_id, "projection source_id")
        source_sha256 = _require_sha256(projection.source_sha256, "projection source_sha256")
        if projection.kind not in _PROJECTION_KINDS:
            raise TraceProjectionError("unknown projection kind")
        if source_id not in sources:
            raise TraceProjectionError("unattributed projection source")
        if source_sha256 != sources[source_id].source_sha256:
            raise TraceProjectionError("projection source hash does not match its registered source")
        if projection_id in owners:
            if owners[projection_id] != source_id:
                raise TraceProjectionError("ambiguous source attribution for projection identifier")
            raise TraceProjectionError("duplicate projection identifier")
        owners[projection_id] = source_id
        rows.append(projection)
    return rows


def _supersessions(
    result: Mapping[str, object], sources: Mapping[str, Source], states: dict[str, str]
) -> list[dict[str, str]]:
    edges = result.get("edges", [])
    if not isinstance(edges, list):
        raise TraceProjectionError("ELPS result edges must be a list")
    for edge in edges:
        if not isinstance(edge, Mapping):
            raise TraceProjectionError("ELPS result edge must be an object")
        if "supersedes_prior" in edge or "prior_body" in edge:
            raise TraceProjectionError("TRACE-01 accepts warning-only ELPS supersession representation")

    warnings = result.get("warnings", [])
    if not isinstance(warnings, list):
        raise TraceProjectionError("ELPS result warnings must be a list")

    supersessions: list[dict[str, str]] = []
    prior_sources: set[str] = set()
    for warning in warnings:
        if not isinstance(warning, Mapping):
            raise TraceProjectionError("ELPS result warning must be an object")
        if warning.get("kind") != "supersedes":
            continue
        source_id = _require_identifier(warning.get("source_doc_id"), "supersedes source_doc_id")
        prior_body = warning.get("prior_body")
        if not isinstance(prior_body, str) or not prior_body:
            raise TraceProjectionError("supersedes warning must include prior_body")
        _require_identifier(warning.get("supersedes_hint"), "supersedes supersedes_hint")
        if source_id not in sources:
            raise TraceProjectionError("supersedes warning names an unregistered source")

        prior_body_sha256 = hashlib.sha256(prior_body.encode("utf-8")).hexdigest()
        candidates = [
            candidate.source_id
            for candidate in sources.values()
            if candidate.source_id != source_id and candidate.source_sha256 == prior_body_sha256
        ]
        if len(candidates) != 1:
            raise TraceProjectionError("supersedes warning has ambiguous prior-source attribution")
        prior_source_id = candidates[0]
        if prior_source_id in prior_sources:
            raise TraceProjectionError("supersedes warning repeats a prior source")
        prior_sources.add(prior_source_id)
        states[prior_source_id] = "superseded"
        supersessions.append({
            "source_id": source_id,
            "prior_source_id": prior_source_id,
            "prior_body_sha256": prior_body_sha256,
        })
    return sorted(supersessions, key=lambda row: (row["source_id"], row["prior_source_id"]))


def _apply_events(events: Iterable[LifecycleEvent], states: dict[str, str]) -> set[str]:
    diagnostics: set[str] = set()
    for event in events:
        source_id = _require_identifier(event.source_id, "lifecycle source_id")
        if source_id not in states:
            raise TraceProjectionError("lifecycle event names an unregistered source")
        if event.kind == "erase":
            states[source_id] = "erased"
            diagnostics.add("source-erased")
        elif event.kind == "reopen":
            if states[source_id] != "erased":
                raise TraceProjectionError("reopen requires an erased source")
            states[source_id] = "active"
            diagnostics.add("source-reopened")
        else:
            raise TraceProjectionError("unknown lifecycle event kind")
    return diagnostics


def build_trace_projection(
    sources: Iterable[Source],
    projections: Iterable[Projection],
    accepted_elps_result: Mapping[str, object],
    *,
    events: Iterable[LifecycleEvent] = (),
) -> dict[str, object]:
    """Build a safe ``trace-projection.v1`` sidecar from synthetic lifecycle input.

    The accepted result is inspected only long enough to hash the warning's
    prior body for attribution. No raw extractor field is copied to the output.
    """
    registered_sources = _source_map(sources)
    projection_rows = _validate_projections(projections, registered_sources)
    states = {source_id: "active" for source_id in registered_sources}
    supersessions = _supersessions(accepted_elps_result, registered_sources, states)
    diagnostics = _apply_events(events, states)
    if supersessions:
        diagnostics.add("supersession-applied")

    sidecar_sources = [
        {
            "source_id": source.source_id,
            "source_sha256": source.source_sha256,
            "lifecycle": states[source.source_id],
        }
        for source in sorted(registered_sources.values(), key=lambda source: source.source_id)
    ]
    sidecar_projections = [
        {
            "projection_id": projection.projection_id,
            "source_id": projection.source_id,
            "source_sha256": projection.source_sha256,
            "kind": projection.kind,
            "lifecycle": states[projection.source_id],
            "searchable": states[projection.source_id] == "active",
        }
        for projection in sorted(projection_rows, key=lambda projection: projection.projection_id)
    ]
    state_counts = {state: sum(value == state for value in states.values()) for state in ("active", "superseded", "erased")}
    searchable_projection_count = sum(row["searchable"] for row in sidecar_projections)
    stale_searchable_projection_count = sum(
        row["searchable"] and row["lifecycle"] != "active" for row in sidecar_projections
    )

    return {
        "schema_version": TRACE_PROJECTION_V1,
        "sources": sidecar_sources,
        "projections": sidecar_projections,
        "supersessions": supersessions,
        "outcomes": {
            "source_count": len(sidecar_sources),
            "active_source_count": state_counts["active"],
            "superseded_source_count": state_counts["superseded"],
            "erased_source_count": state_counts["erased"],
            "projection_count": len(sidecar_projections),
            "searchable_projection_count": searchable_projection_count,
            "unattributed_projection_count": 0,
            "stale_searchable_projection_count": stale_searchable_projection_count,
        },
        "diagnostics": sorted(diagnostics),
    }


def write_trace_projection(path: Path, sidecar: Mapping[str, object]) -> Path:
    """Write a canonical, payload-free TRACE-01 sidecar to an explicit path."""
    if sidecar.get("schema_version") != TRACE_PROJECTION_V1:
        raise TraceProjectionError("sidecar schema_version must be trace-projection.v1")
    path.write_text(json.dumps(sidecar, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    return path
