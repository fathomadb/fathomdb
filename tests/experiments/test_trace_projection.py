"""Human-intended lifecycle tests for the synthetic TRACE-01 sidecar."""

from __future__ import annotations

import hashlib
import json

import pytest

from experiments.trace_projection import (
    LifecycleEvent,
    Projection,
    Source,
    TraceProjectionError,
    build_trace_projection,
    write_trace_projection,
)


PRIOR_BODY = "Ada is a member of the Platform team."
CURRENT_BODY = "Ada now leads the Platform team."


def _sha(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def _fixture() -> tuple[tuple[Source, ...], tuple[Projection, ...], dict]:
    prior = Source("source-prior", _sha(PRIOR_BODY))
    current = Source("source-current", _sha(CURRENT_BODY))
    projections = (
        Projection("text-prior", "source-prior", prior.source_sha256, "text"),
        Projection("vector-prior", "source-prior", prior.source_sha256, "vector_child"),
        Projection("summary-prior", "source-prior", prior.source_sha256, "summary"),
        Projection("fact-prior", "source-prior", prior.source_sha256, "extracted_fact"),
        Projection("entity-prior", "source-prior", prior.source_sha256, "entity"),
        Projection("edge-prior", "source-prior", prior.source_sha256, "edge"),
        Projection("text-current", "source-current", current.source_sha256, "text"),
        Projection("vector-current", "source-current", current.source_sha256, "vector_child"),
    )
    accepted_result = {
        "type": "result",
        "warnings": [{
            "kind": "supersedes",
            "source_doc_id": "source-current",
            "prior_body": PRIOR_BODY,
            "supersedes_hint": "Ada leads Platform team",
        }],
    }
    return (prior, current), projections, accepted_result


def test_warning_only_supersession_covers_every_projection_and_hides_stale_hits(tmp_path):
    sources, projections, accepted_result = _fixture()

    sidecar = build_trace_projection(sources, projections, accepted_result)
    path = write_trace_projection(tmp_path / "trace-projection.json", sidecar)

    assert sidecar["schema_version"] == "trace-projection.v1"
    assert sidecar["outcomes"] == {
        "source_count": 2,
        "active_source_count": 1,
        "superseded_source_count": 1,
        "erased_source_count": 0,
        "projection_count": 8,
        "searchable_projection_count": 2,
        "unattributed_projection_count": 0,
        "stale_searchable_projection_count": 0,
    }
    assert {row["projection_id"] for row in sidecar["projections"] if row["searchable"]} == {
        "text-current", "vector-current",
    }
    assert sidecar["supersessions"] == [{
        "source_id": "source-current",
        "prior_source_id": "source-prior",
        "prior_body_sha256": _sha(PRIOR_BODY),
    }]
    serialized = path.read_text(encoding="utf-8")
    assert PRIOR_BODY not in serialized
    assert CURRENT_BODY not in serialized
    assert "Ada leads Platform team" not in serialized


def test_ambiguous_projection_source_attribution_is_rejected():
    sources, projections, accepted_result = _fixture()
    ambiguous = projections + (
        Projection("vector-current", "source-prior", sources[0].source_sha256, "vector_child"),
    )

    with pytest.raises(TraceProjectionError, match="ambiguous source attribution"):
        build_trace_projection(sources, ambiguous, accepted_result)


def test_source_text_cannot_be_used_as_an_identifier():
    sources, projections, accepted_result = _fixture()
    unsafe_sources = (Source(PRIOR_BODY, sources[0].source_sha256), sources[1])

    with pytest.raises(TraceProjectionError, match="safe identifier"):
        build_trace_projection(unsafe_sources, projections, accepted_result)


def test_competing_elps_edge_supersession_representation_is_rejected():
    sources, projections, _ = _fixture()
    competing_result = {
        "edges": [{"supersedes_prior": True, "prior_body": PRIOR_BODY}],
        "warnings": [],
    }

    with pytest.raises(TraceProjectionError, match="warning-only"):
        build_trace_projection(sources, projections, competing_result)


def test_erasure_removes_all_searchable_projections_for_its_source():
    sources, projections, accepted_result = _fixture()

    sidecar = build_trace_projection(
        sources,
        projections,
        accepted_result,
        events=(LifecycleEvent("erase", "source-current"),),
    )

    assert sidecar["outcomes"]["erased_source_count"] == 1
    assert sidecar["outcomes"]["searchable_projection_count"] == 0
    assert sidecar["outcomes"]["stale_searchable_projection_count"] == 0
    assert {row["lifecycle"] for row in sidecar["projections"] if row["source_id"] == "source-current"} == {
        "erased"
    }


def test_reopen_recovers_only_the_erased_source_projections():
    sources, projections, accepted_result = _fixture()

    sidecar = build_trace_projection(
        sources,
        projections,
        accepted_result,
        events=(
            LifecycleEvent("erase", "source-current"),
            LifecycleEvent("reopen", "source-current"),
        ),
    )

    assert sidecar["outcomes"]["active_source_count"] == 1
    assert sidecar["outcomes"]["erased_source_count"] == 0
    assert {row["projection_id"] for row in sidecar["projections"] if row["searchable"]} == {
        "text-current", "vector-current",
    }
    assert all(PRIOR_BODY not in value for value in json.dumps(sidecar).splitlines())


def test_sidecar_writer_rejects_payload_injection_after_build(tmp_path):
    sources, projections, accepted_result = _fixture()
    sidecar = build_trace_projection(sources, projections, accepted_result)
    sidecar["diagnostics"] = [PRIOR_BODY]

    with pytest.raises(TraceProjectionError, match="diagnostic"):
        write_trace_projection(tmp_path / "unsafe-trace-projection.json", sidecar)


def test_sidecar_output_and_bytes_are_deterministic_across_input_order(tmp_path):
    sources, projections, accepted_result = _fixture()

    first = build_trace_projection(sources, projections, accepted_result)
    second = build_trace_projection(tuple(reversed(sources)), tuple(reversed(projections)), accepted_result)
    first_path = write_trace_projection(tmp_path / "first.json", first)
    second_path = write_trace_projection(tmp_path / "second.json", second)

    assert first == second
    assert first_path.read_bytes() == second_path.read_bytes()
    assert first["diagnostics"] == ["supersession-applied"]
