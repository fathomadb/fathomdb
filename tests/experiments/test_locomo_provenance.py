"""Tests for content-free LOCOMO ingestion provenance manifests."""

from __future__ import annotations

import json

import pytest

from experiments.locomo_provenance import (
    ProvenanceEntry,
    ProvenanceMap,
    build_manifest_document,
    payload_fingerprint,
)


def _payload(*, content: str = "raw corpus text", user_id: str = "locomo_7_run-123") -> dict:
    return {"user_id": user_id, "messages": [{"role": "user", "content": content}]}


def test_provenance_map_resolves_normalized_official_payload_and_exposes_ids_only():
    payload = _payload()
    entry = ProvenanceEntry(
        fingerprint=payload_fingerprint(payload),
        conversation_id="locomo-7",
        session_id="session-3",
        turn_ids=("D3:7",),
    )
    mapping = ProvenanceMap([entry])

    resolved = mapping.resolve(_payload(user_id="locomo_7_another-run"))

    assert resolved.safe_metadata() == {
        "conversation_id": "locomo-7",
        "session_id": "session-3",
        "turn_ids": ["D3:7"],
    }
    assert "raw corpus text" not in json.dumps(resolved.safe_metadata())


def test_provenance_map_rejects_unmapped_or_ambiguous_fingerprints():
    payload = _payload()
    fingerprint = payload_fingerprint(payload)
    first = ProvenanceEntry(fingerprint, "locomo-7", "session-3", ("D3:7",))
    second = ProvenanceEntry(fingerprint, "locomo-7", "session-3", ("D3:8",))

    with pytest.raises(ValueError, match="ambiguous"):
        ProvenanceMap([first, second])

    with pytest.raises(ValueError, match="unmapped"):
        ProvenanceMap([first]).resolve(_payload(content="unknown"))


def test_manifest_builder_has_no_corpus_text_and_covers_turn_and_session_units():
    corpus = [{"conversation": {
        "speaker_a": "Ada", "speaker_b": "Bea", "session_1_date_time": "2024-01-01",
        "session_1": [
            {"dia_id": "D1:1", "speaker": "Ada", "text": "secret alpha"},
            {"dia_id": "D1:2", "speaker": "Bea", "text": "secret beta"},
        ],
    }}]

    turns = build_manifest_document(corpus, ingest_unit="turn")
    sessions = build_manifest_document(corpus, ingest_unit="session")

    assert turns["schema_version"] == "locomo-provenance.v1"
    assert len(turns["entries"]) == 2
    assert len(sessions["entries"]) == 1
    assert sessions["entries"][0]["turn_ids"] == ["D1:1", "D1:2"]
    assert "secret alpha" not in json.dumps(turns)
    assert "secret beta" not in json.dumps(sessions)
