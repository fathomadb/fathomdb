"""Thin native replay of the shared, authored object expectations."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

import fathomdb
from fathomdb import read


def test_shared_identity_unicode_provenance_time_filter_cursor_and_reopen(tmp_path: Path) -> None:
    fixture = json.loads(
        (Path(__file__).resolve().parents[2] / "conformance/slice55-behavior.json").read_text(
            encoding="utf-8"
        )
    )
    path = str(tmp_path / "shared.sqlite")
    engine = fathomdb.Engine.open(path, use_default_embedder=False)
    try:
        engine.configure_projections(
            [
                fathomdb.ProjectionSpec(
                    name="group", roles=frozenset({fathomdb.ProjectionRole.FILTERABLE})
                )
            ]
        )
        engine.write(
            [
                {
                    "kind": "doc",
                    "body": fixture["body"],
                    "logical_id": n["id"],
                    "source_id": n["source"],
                    "valid_from": 1_000_000_000,
                    "valid_until": fixture["instant"] + 1 if i == 0 else None,
                    "provenance": {
                        "schema_version": 1,
                        "role": "canonical",
                        "artifact_revision_id": n["revision"],
                        "source_version_id": n["version"],
                    },
                }
                for i, n in enumerate(fixture["nodes"])
            ]
        )
        engine.drain(timeout_s=10)
        result = engine.search_text_only(fixture["query"])
        assert [h.id.value for h in result.results] == fixture["currentIds"]
        assert [h.body for h in result.results] == [fixture["body"]] * 2
        assert [h.source_id for h in result.results] == [n["source"] for n in fixture["nodes"]]
        assert [
            h.id.value
            for h in engine.search_text_only(
                fixture["query"], view=fathomdb.ReadView(valid_as_of=fixture["instant"] + 1)
            ).results
        ] == fixture["expiredIds"]
        assert (
            len(
                engine.search_text_only(
                    fixture["query"], filter=fathomdb.SearchFilter(attributes=(("group", "kept"),))
                ).results
            )
            == 2
        )
        assert (
            engine.search_text_only(
                fixture["query"], filter=fathomdb.SearchFilter(attributes=(("group", "absent"),))
            ).results
            == []
        )
        frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
        first = read.canonical_page(engine, "doc", frozen, fathomdb.PageRequestV1(limit=1))
        assert first.next_cursor is not None
        second = read.canonical_page(
            engine, "doc", frozen, fathomdb.PageRequestV1(limit=1, cursor=first.next_cursor)
        )
        assert [r.logical_id for r in first.items + second.items] == fixture["currentIds"]
        assert second.next_cursor is None
        with pytest.raises(fathomdb.errors.WriteValidationError):
            engine.search_text_only(fixture["invalidQuery"])
        evidence = engine.search_with_evidence(
            fathomdb.EvidenceSearchRequestV1(query=fixture["query"], context=frozen)
        )
        assert [e.artifact_revision_id for e in evidence.evidence] == [
            n["revision"] for n in fixture["nodes"]
        ]
        for entry, expected in zip(evidence.evidence, fixture["nodes"], strict=True):
            resolved = engine.resolve_evidence(
                fathomdb.EvidenceResolveRequestV1(evidence_ref=entry.evidence_ref, context=frozen)
            )
            assert resolved.canonical_source_body == fixture["body"]
            assert resolved.source_revision_id == expected["revision"]
            assert resolved.artifact_revision_id == expected["revision"]
            assert resolved.source_id == expected["source"]
    finally:
        engine.close()
    reopened = fathomdb.Engine.open(path, use_default_embedder=False)
    try:
        for node in fixture["nodes"]:
            row = read.get(reopened, node["id"])
            assert row is not None
            assert row.body == fixture["body"]
        assert read.get(reopened, "absent") is None
    finally:
        reopened.close()
