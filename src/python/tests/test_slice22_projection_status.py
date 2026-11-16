"""Slice 22 projection-runtime status parity through the real PyO3 database."""

from __future__ import annotations

from dataclasses import FrozenInstanceError

import pytest

from fathomdb import (
    Engine,
    ProjectionRole,
    ProjectionRuntimeStatus,
    ProjectionRuntimeStatusEntry,
    ProjectionSpec,
    read,
)


def _open(path: str) -> Engine:
    return Engine.open(path, use_default_embedder=False)


def _node(kind: str, logical_id: str) -> dict[str, str]:
    return {
        "kind": kind,
        "logical_id": logical_id,
        "body": '{"alpha":"dense meaning","zeta":"plain value"}',
        "source_id": "py-test:slice22-status",
    }


def test_projection_status_is_a_frozen_typed_current_read(tmp_path) -> None:
    """Exact Python wires mirror the signed Rust facade, not `ProjectionDelta`."""

    engine = _open(str(tmp_path / "status.sqlite"))
    try:
        engine.configure_projections(
            [
                ProjectionSpec(
                    name="zeta", roles=frozenset({ProjectionRole.FILTERABLE})
                ),
                ProjectionSpec(
                    name="alpha", roles=frozenset({ProjectionRole.SEARCHABLE}), vector=True),
            ]
        )
        engine.write([_node("invoice", "I1")])
        engine.write([_node("doc", "D1")])
        engine.write([_node("entity", "E1")])
        engine.write([_node("invoice", "I2")])

        status = read.projection_status(engine)
        assert isinstance(status, ProjectionRuntimeStatus)
        assert all(isinstance(item, ProjectionRuntimeStatusEntry) for item in status.projections)
        assert status.runtime_embedder_available is False
        assert status.runtime_unavailability_reason == "no_runtime"
        assert [(item.name, item.dense_readiness) for item in status.projections] == [
            ("alpha", "unavailable"),
            ("zeta", "not_declared"),
        ]
        assert status.vector_unsupported_kinds == ("entity", "invoice")
        assert read.projection_status(engine) == status, "repeated reads have no side effects"

        with pytest.raises(FrozenInstanceError):
            status.runtime_embedder_available = True  # type: ignore[misc]
    finally:
        engine.close()


def test_s90_installed_projection_status_exposes_stored_mapped_coverage(tmp_path) -> None:
    """The real installed native/wrapper boundary retains the new aggregate."""
    engine = _open(str(tmp_path / "coverage.sqlite"))
    try:
        status = read.projection_status(engine)
        coverage = getattr(status, "vector_coverage", None)
        assert coverage is not None, "installed projection status drops vector_coverage"
        assert coverage.population == "stored_mapped_spans"
        assert coverage.profile.key == "default"
        assert type(coverage.profile.dimension) is int
        assert coverage.profile.dimension > 0
        assert coverage.status in ("available", "unavailable")
        if coverage.status == "available":
            assert coverage.records == 0
            assert coverage.known_records == 0
            assert coverage.unknown_records == 0
            assert coverage.truncated_known_share is None
            assert coverage.unrepresented_known_byte_share is None
        else:
            assert coverage.reason == "cache_cold"
    finally:
        engine.close()


def test_s90_aggregate_property_matches_exact_utf8_body_totals() -> None:
    """Legal short stored heads retain exact integer totals and unreduced shares."""
    import tempfile

    from hypothesis import given, settings, strategies as st

    @settings(max_examples=8, deadline=None)
    @given(st.lists(st.text(alphabet="abcé🦀\u0301", min_size=1, max_size=16), min_size=1, max_size=4))
    def check(bodies: list[str]) -> None:
        with tempfile.TemporaryDirectory() as directory:
            engine = Engine.open(f"{directory}/property.sqlite", use_default_embedder=True)
            try:
                engine.configure_projections([ProjectionSpec(
                    name="body", roles=frozenset({ProjectionRole.SEARCHABLE}), vector=True,
                )])
                engine.write([{
                    "kind": "doc", "body": body, "logical_id": f"n{i}",
                    "source_id": "py-test:s90-property",
                } for i, body in enumerate(bodies)])
                engine.drain(timeout_s=30)
                read.projection_generation_status(engine)
                coverage = getattr(read.projection_status(engine), "vector_coverage", None)
                assert coverage is not None, "installed aggregate property drops vector_coverage"
                assert coverage.status == "available"
                assert coverage.records == coverage.known_records == len(bodies)
                assert coverage.unknown_records == coverage.truncated_known_records == 0
                assert coverage.known_body_bytes == sum(len(body.encode()) for body in bodies)
                assert coverage.unrepresented_known_bytes == coverage.unattributed_heads == 0
                assert (coverage.truncated_known_share.numerator, coverage.truncated_known_share.denominator) == (0, len(bodies))
                assert (coverage.unrepresented_known_byte_share.numerator, coverage.unrepresented_known_byte_share.denominator) == (0, coverage.known_body_bytes)
            finally:
                engine.close()

    check()
