"""0.8.31 Slice 50 SDK ergonomics: resolve_evidence_many, search(hints=), and OpenReport booleans."""

from __future__ import annotations

import hashlib
from pathlib import Path
from typing import Any

import fathomdb
import pytest  # pyright: ignore[reportMissingImports]
from fathomdb import (
    EVIDENCE_RESOLVE_MANY_MAX,
    Engine,
    EvidenceError,
    EvidenceResolveManyItemV1,
    EvidenceResolveManyRequestV1,
    EvidenceSearchRequestV1,
    Hint,
    ReadContextV1,
    ReadView,
    SearchFilter,
)
from fathomdb.errors import InvalidArgumentError, WriteValidationError


def _seed_provenanced_pair(engine: Engine) -> tuple[str, str]:
    body_a = "canonical alpha evidence text for slice50"
    body_b = "canonical beta evidence text for slice50"
    engine.write(
        [
            {
                "kind": "document",
                "body": body_a,
                "source_id": "slice50-src-a",
                "logical_id": "slice50-doc-a",
                "provenance": {
                    "schema_version": 1,
                    "role": "canonical",
                    "artifact_revision_id": "slice50-doc-a-r1",
                    "source_version_id": "slice50-ver-a-1",
                },
            },
            {
                "kind": "fact",
                "body": "slice50needle alpha fact",
                "source_id": "slice50-src-a",
                "logical_id": "slice50-fact-a",
                "provenance": {
                    "schema_version": 1,
                    "role": "derived",
                    "artifact_revision_id": "slice50-fact-a-r1",
                    "source_version_id": "slice50-ver-a-1",
                    "source_revision_id": "slice50-doc-a-r1",
                    "source_locator": {"kind": "whole_body"},
                    "canonical_source_hash": {
                        "algorithm": "sha256",
                        "digest_hex": hashlib.sha256(body_a.encode()).hexdigest(),
                    },
                },
            },
            {
                "kind": "document",
                "body": body_b,
                "source_id": "slice50-src-b",
                "logical_id": "slice50-doc-b",
                "provenance": {
                    "schema_version": 1,
                    "role": "canonical",
                    "artifact_revision_id": "slice50-doc-b-r1",
                    "source_version_id": "slice50-ver-b-1",
                },
            },
            {
                "kind": "fact",
                "body": "slice50needle beta fact",
                "source_id": "slice50-src-b",
                "logical_id": "slice50-fact-b",
                "provenance": {
                    "schema_version": 1,
                    "role": "derived",
                    "artifact_revision_id": "slice50-fact-b-r1",
                    "source_version_id": "slice50-ver-b-1",
                    "source_revision_id": "slice50-doc-b-r1",
                    "source_locator": {"kind": "whole_body"},
                    "canonical_source_hash": {
                        "algorithm": "sha256",
                        "digest_hex": hashlib.sha256(body_b.encode()).hexdigest(),
                    },
                },
            },
        ]
    )
    return body_a, body_b


def _tamper_token(token: str) -> str:
    last = "0" if token[-1] != "0" else "1"
    return token[:-1] + last


def test_resolve_evidence_many_validation_and_bounds(db_path: str) -> None:
    assert EVIDENCE_RESOLVE_MANY_MAX == 50
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        _seed_provenanced_pair(engine)
        frozen = engine.freeze_read_context(ReadContextV1(view=ReadView()))
        ev_search = engine.search_with_evidence(
            EvidenceSearchRequestV1(query="slice50needle", context=frozen, limit=10)
        )
        valid_ref = ev_search.evidence[0].evidence_ref

        with pytest.raises(TypeError):
            engine.resolve_evidence_many("not-a-request")  # type: ignore[arg-type]

        for bad_schema in (0, 2, True):
            with pytest.raises(EvidenceError) as exc_info:
                engine.resolve_evidence_many(
                    EvidenceResolveManyRequestV1(
                        evidence_refs=[valid_ref],
                        context=frozen,
                        schema_version=bad_schema,  # type: ignore[arg-type]
                    )
                )
            assert exc_info.value.reason == "unsupported_schema_version"
            assert exc_info.value.field_path == "/schemaVersion"

        for bad_container in (valid_ref, 123, None, {"a": valid_ref}):
            with pytest.raises(TypeError):
                engine.resolve_evidence_many(
                    EvidenceResolveManyRequestV1(
                        evidence_refs=bad_container,  # type: ignore[arg-type]
                        context=frozen,
                    )
                )

        with pytest.raises(TypeError):
            engine.resolve_evidence_many(
                EvidenceResolveManyRequestV1(
                    evidence_refs=[valid_ref, 42],  # type: ignore[list-item]
                    context=frozen,
                )
            )

        with pytest.raises(WriteValidationError):
            engine.resolve_evidence_many(
                EvidenceResolveManyRequestV1(
                    evidence_refs=[valid_ref, "bad\x00token"],
                    context=frozen,
                )
            )

        with pytest.raises(InvalidArgumentError):
            engine.resolve_evidence_many(
                EvidenceResolveManyRequestV1(evidence_refs=[], context=frozen)
            )

        with pytest.raises(InvalidArgumentError):
            engine.resolve_evidence_many(
                EvidenceResolveManyRequestV1(
                    evidence_refs=[valid_ref] * (EVIDENCE_RESOLVE_MANY_MAX + 1),
                    context=frozen,
                )
            )
    finally:
        engine.close()


def test_resolve_evidence_many_duplicates_partial_outcomes_and_drift(
    db_path: str,
    tmp_path: Path,
) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    other_db = str(tmp_path / "other-slice50.sqlite")
    other = Engine.open(other_db, use_default_embedder=False)
    try:
        body_a, body_b = _seed_provenanced_pair(engine)
        _seed_provenanced_pair(other)

        frozen = engine.freeze_read_context(ReadContextV1(view=ReadView()))
        other_frozen = other.freeze_read_context(ReadContextV1(view=ReadView()))
        filtered_frozen = engine.freeze_read_context(
            ReadContextV1(view=ReadView(), eligibility=SearchFilter(kind="fact"))
        )

        ev_search = engine.search_with_evidence(
            EvidenceSearchRequestV1(query="slice50needle", context=frozen, limit=10)
        )
        assert len(ev_search.evidence) == 2
        ref_0 = ev_search.evidence[0].evidence_ref
        ref_1 = ev_search.evidence[1].evidence_ref

        other_ev = other.search_with_evidence(
            EvidenceSearchRequestV1(query="slice50needle", context=other_frozen, limit=10)
        )
        wrong_db_ref = other_ev.evidence[0].evidence_ref

        filtered_ev = engine.search_with_evidence(
            EvidenceSearchRequestV1(query="slice50needle", context=filtered_frozen, limit=10)
        )
        wrong_context_ref = filtered_ev.evidence[0].evidence_ref

        # Duplicate preservation & tuple acceptance.
        dup_items = engine.resolve_evidence_many(
            EvidenceResolveManyRequestV1(
                evidence_refs=(ref_0, ref_1, ref_0),
                context=frozen,
            )
        )
        assert len(dup_items) == 3
        assert all(isinstance(item, EvidenceResolveManyItemV1) for item in dup_items)
        assert [item.status for item in dup_items] == ["resolved", "resolved", "resolved"]
        assert dup_items[0].resolved == dup_items[2].resolved
        assert dup_items[0].resolved is not None
        assert dup_items[1].resolved is not None
        assert {
            dup_items[0].resolved.canonical_source_body,
            dup_items[1].resolved.canonical_source_body,
        } == {body_a, body_b}

        # Per-slot nondisclosure partial outcomes across empty, malformed, oversized,
        # tampered, wrong-DB, and wrong-context tokens.
        oversized = "x" * 2049
        mixed = engine.resolve_evidence_many(
            EvidenceResolveManyRequestV1(
                evidence_refs=[
                    ref_0,
                    "",
                    "not-a-valid-token",
                    _tamper_token(ref_0),
                    oversized,
                    wrong_db_ref,
                    wrong_context_ref,
                    ref_1,
                ],
                context=frozen,
            )
        )
        assert len(mixed) == 8
        assert mixed[0].status == "resolved" and mixed[0].resolved == dup_items[0].resolved
        assert mixed[7].status == "resolved" and mixed[7].resolved == dup_items[1].resolved
        for bad_idx in range(1, 7):
            assert mixed[bad_idx].status == "refused"
            assert mixed[bad_idx].resolved is None
            assert mixed[bad_idx].reason == "evidence_unavailable"
            assert mixed[bad_idx].field_path == f"/evidenceRefs/{bad_idx}"

        # Mutate the database so the previously minted frozen context drifts;
        # resolve_evidence_many must refuse the whole call with EvidenceUnavailable.
        engine.write(
            [
                {
                    "kind": "fact",
                    "body": "slice50needle alpha fact v2",
                    "source_id": "slice50-src-a",
                    "logical_id": "slice50-fact-a",
                    "provenance": {
                        "schema_version": 1,
                        "role": "derived",
                        "artifact_revision_id": "slice50-fact-a-r2",
                        "source_version_id": "slice50-ver-a-1",
                        "source_revision_id": "slice50-doc-a-r1",
                        "source_locator": {"kind": "whole_body"},
                        "canonical_source_hash": {
                            "algorithm": "sha256",
                            "digest_hex": hashlib.sha256(body_a.encode()).hexdigest(),
                        },
                    },
                }
            ]
        )
        with pytest.raises(EvidenceError) as drift_exc:
            engine.resolve_evidence_many(
                EvidenceResolveManyRequestV1(evidence_refs=[ref_0], context=frozen)
            )
        assert drift_exc.value.reason == "evidence_unavailable"

        # Under a fresh frozen context, the old token minted for the pre-supersede
        # snapshot is refused per-slot while a freshly minted token resolves.
        fresh_frozen = engine.freeze_read_context(ReadContextV1(view=ReadView()))
        fresh_ev = engine.search_with_evidence(
            EvidenceSearchRequestV1(query="slice50needle", context=fresh_frozen, limit=10)
        )
        fresh_ref = fresh_ev.evidence[0].evidence_ref
        post_mutate = engine.resolve_evidence_many(
            EvidenceResolveManyRequestV1(
                evidence_refs=[fresh_ref, ref_0],
                context=fresh_frozen,
            )
        )
        assert post_mutate[0].status == "resolved" and post_mutate[0].resolved is not None
        assert post_mutate[1].status == "refused"
        assert post_mutate[1].reason == "evidence_unavailable"
        assert post_mutate[1].field_path == "/evidenceRefs/1"
    finally:
        other.close()
        engine.close()


def test_search_hints_across_search_frozen_and_evidence(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        engine.write(
            [
                {
                    "kind": "doc",
                    "body": f"hintstarget document number {idx}",
                    "source_id": f"hints-src-{idx}",
                    "logical_id": f"hints-doc-{idx}",
                    "provenance": {
                        "schema_version": 1,
                        "role": "canonical",
                        "artifact_revision_id": f"hints-doc-{idx}-r1",
                        "source_version_id": f"hints-ver-{idx}-1",
                    },
                }
                for idx in range(4)
            ]
        )
        frozen = engine.freeze_read_context(ReadContextV1(view=ReadView()))

        # Default and explicit hints=False leave hints=None and preserve hit equality.
        default_res = engine.search("hintstarget", limit=2)
        false_res = engine.search("hintstarget", limit=2, hints=False)
        assert default_res.hints is None
        assert false_res.hints is None
        assert default_res.results == false_res.results

        # hints=True on limit-saturated page emits more_results hint.
        more_res = engine.search("hintstarget", limit=2, hints=True)
        assert more_res.results == default_res.results
        assert more_res.hints is not None
        assert any(
            isinstance(h, Hint)
            and h.action == "search"
            and h.reason == "more_results"
            and h.limit == 4
            for h in more_res.hints
        )

        # hints=True on zero-hit page emits no_results hint.
        empty_res = engine.search("definitelyabsenttokenxyz", limit=5, hints=True)
        assert empty_res.results == []
        assert empty_res.hints is not None
        assert any(
            isinstance(h, Hint) and h.action == "search" and h.reason == "no_results"
            for h in empty_res.hints
        )

        # search_frozen parity.
        frozen_default = engine.search_frozen("hintstarget", frozen, limit=2)
        assert frozen_default.hints is None
        frozen_hints = engine.search_frozen("hintstarget", frozen, limit=2, hints=True)
        assert frozen_hints.results == frozen_default.results
        assert frozen_hints.hints is not None
        assert any(h.reason == "more_results" and h.limit == 4 for h in frozen_hints.hints)

        # search_with_evidence parity.
        ev_default = engine.search_with_evidence(
            EvidenceSearchRequestV1(query="hintstarget", context=frozen, limit=2)
        )
        assert ev_default.search_result.hints is None
        ev_hints = engine.search_with_evidence(
            EvidenceSearchRequestV1(query="hintstarget", context=frozen, limit=2, hints=True)
        )
        assert ev_hints.search_result.results == ev_default.search_result.results
        assert ev_hints.search_result.hints is not None
        assert any(
            h.reason == "more_results" and h.limit == 4 for h in ev_hints.search_result.hints
        )

        # Non-bool hints raises TypeError across all three entry points.
        for bad_hints in (1, 0, "true", None):
            bad_any: Any = bad_hints
            with pytest.raises(TypeError):
                engine.search("hintstarget", hints=bad_any)
            with pytest.raises(TypeError):
                engine.search_frozen("hintstarget", frozen, hints=bad_any)
            with pytest.raises(TypeError):
                engine.search_with_evidence(
                    EvidenceSearchRequestV1(query="hintstarget", context=frozen, hints=bad_any)
                )
    finally:
        engine.close()


def test_open_report_capability_booleans_contract(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        report = engine.open_report()
        assert isinstance(report.reranker_compiled, bool)
        assert isinstance(report.cuda_compiled, bool)
        assert isinstance(report.rerank_available, bool)
        if not report.reranker_compiled:
            assert report.rerank_available is False
        # Top-level re-export sanity.
        assert fathomdb.Hint is Hint
        assert fathomdb.EvidenceResolveManyRequestV1 is EvidenceResolveManyRequestV1
        assert fathomdb.EvidenceResolveManyItemV1 is EvidenceResolveManyItemV1
        assert fathomdb.EVIDENCE_RESOLVE_MANY_MAX == 50
    finally:
        engine.close()
