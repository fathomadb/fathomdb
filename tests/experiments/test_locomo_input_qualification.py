"""Content-free factual-preflight tests for LOCOMO/PARENT external inputs."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from experiments import locomo_input_qualification


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _write_json(path: Path, value: object) -> Path:
    path.write_text(json.dumps(value), encoding="utf-8")
    return path


def _phase_b(tmp_path: Path, *, corpus_sha256: str, turn_sha256: str, session_sha256: str, subset_sha256: str) -> Path:
    return _write_json(
        tmp_path / "phase-b.json",
        {
            "external_inputs": {
                "corpus": {"sha256": corpus_sha256, "question_count": 2},
                "turn_provenance": {"sha256": turn_sha256},
                "session_provenance": {"sha256": session_sha256},
                "dry_run_subset": {"sha256": subset_sha256, "question_count": 2},
            }
        },
    )


def _matrix(tmp_path: Path) -> Path:
    return _write_json(
        tmp_path / "matrix.json",
        {
            "corpora": [
                {
                    "corpus_id": "locomo",
                    "license": "CC-BY-NC-4.0",
                    "payload_rule": "external_eval_only_gitignored",
                    "supported_categories": ["time_scoped_validity"],
                    "supported_claims": ["conversation_retrieval"],
                    "unsupported_claims": ["source_erasure"],
                }
            ]
        },
    )


def _inputs(tmp_path: Path) -> dict[str, Path]:
    corpus = _write_json(
        tmp_path / "corpus.json",
        [
            {
                "conversation": {"session_1": []},
                "qa": [{"question": "external only", "answer": "not emitted"}, {"question": "external only", "answer": "not emitted"}],
            }
        ],
    )
    turns = _write_json(
        tmp_path / "turn.json",
        {
            "schema_version": "locomo-provenance.v1",
            "entries": [
                {"fingerprint": "a" * 64, "conversation_id": "locomo-0", "session_id": "session_1", "turn_ids": ["turn-1"]},
                {"fingerprint": "b" * 64, "conversation_id": "locomo-0", "session_id": "session_1", "turn_ids": ["turn-2"]},
            ],
        },
    )
    sessions = _write_json(
        tmp_path / "session.json",
        {
            "schema_version": "locomo-provenance.v1",
            "entries": [
                {"fingerprint": "c" * 64, "conversation_id": "locomo-0", "session_id": "session_1", "turn_ids": ["turn-1", "turn-2"]}
            ],
        },
    )
    subset = _write_json(
        tmp_path / "subset.json",
        {"schema_version": "locomo-fixed-subset.v1", "question_ids": ["locomo-0-q-0", "locomo-0-q-1"]},
    )
    return {"corpus": corpus, "turn": turns, "session": sessions, "subset": subset}


def test_qualification_writes_hash_bound_content_free_trace_parent_and_blocker_report(tmp_path):
    inputs = _inputs(tmp_path)
    phase_b = _phase_b(
        tmp_path,
        corpus_sha256=_sha256(inputs["corpus"]),
        turn_sha256=_sha256(inputs["turn"]),
        session_sha256=_sha256(inputs["session"]),
        subset_sha256="0" * 64,
    )
    artifacts = tmp_path / "artifacts"

    report_path = locomo_input_qualification.qualify(
        phase_b_path=phase_b,
        corpus_matrix_path=_matrix(tmp_path),
        corpus_path=inputs["corpus"],
        turn_provenance_path=inputs["turn"],
        session_provenance_path=inputs["session"],
        dry_run_subset_path=inputs["subset"],
        artifact_root=artifacts,
    )

    report = json.loads(report_path.read_text(encoding="utf-8"))
    assert report["schema_version"] == "locomo-input-qualification-report.v1"
    assert report["qualification_status"] == "blocked"
    assert report["blockers"] == ["dry_run_subset_sha256_mismatch"]
    assert report["artifacts"] == {
        "parent_relation_proof": {"sha256": _sha256(artifacts / "locomo-parent-relation-proof.v1.json"), "entry_count": 2},
        "trace_projection": {"sha256": _sha256(artifacts / "trace-projection.v1.json"), "source_count": 1},
    }
    assert "external only" not in json.dumps(report)
    assert "answer" not in json.dumps(report)
    assert report["report_sha256"] == locomo_input_qualification.report_sha256(report)


def test_qualification_fails_closed_with_a_signed_blocker_report_when_a_required_input_is_missing(tmp_path):
    inputs = _inputs(tmp_path)
    phase_b = _phase_b(
        tmp_path,
        corpus_sha256=_sha256(inputs["corpus"]),
        turn_sha256=_sha256(inputs["turn"]),
        session_sha256=_sha256(inputs["session"]),
        subset_sha256=_sha256(inputs["subset"]),
    )
    artifacts = tmp_path / "artifacts"

    report_path = locomo_input_qualification.qualify(
        phase_b_path=phase_b,
        corpus_matrix_path=_matrix(tmp_path),
        corpus_path=tmp_path / "missing-corpus.json",
        turn_provenance_path=inputs["turn"],
        session_provenance_path=inputs["session"],
        dry_run_subset_path=inputs["subset"],
        artifact_root=artifacts,
    )
    report = json.loads(report_path.read_text(encoding="utf-8"))
    assert report["qualification_status"] == "blocked"
    assert report["blockers"] == ["corpus_input_unavailable"]
    assert set(report["artifacts"]) == {"trace_projection", "parent_relation_proof"}
    assert report["report_sha256"] == locomo_input_qualification.report_sha256(report)


def test_qualification_records_ambiguous_parent_membership_without_emitting_a_relation_proof(tmp_path):
    inputs = _inputs(tmp_path)
    turns = json.loads(inputs["turn"].read_text(encoding="utf-8"))
    turns["entries"][1]["turn_ids"] = ["turn-1"]
    _write_json(inputs["turn"], turns)
    phase_b = _phase_b(
        tmp_path,
        corpus_sha256=_sha256(inputs["corpus"]),
        turn_sha256=_sha256(inputs["turn"]),
        session_sha256=_sha256(inputs["session"]),
        subset_sha256=_sha256(inputs["subset"]),
    )
    artifacts = tmp_path / "artifacts"

    report_path = locomo_input_qualification.qualify(
        phase_b_path=phase_b,
        corpus_matrix_path=_matrix(tmp_path),
        corpus_path=inputs["corpus"],
        turn_provenance_path=inputs["turn"],
        session_provenance_path=inputs["session"],
        dry_run_subset_path=inputs["subset"],
        artifact_root=artifacts,
    )

    report = json.loads(report_path.read_text(encoding="utf-8"))
    assert report["qualification_status"] == "blocked"
    assert report["blockers"] == ["parent_relation_proof_ambiguous_child_identifier"]
    assert set(report["artifacts"]) == {"trace_projection"}


@pytest.mark.parametrize(
    ("target", "contents", "blocker"),
    [
        ("corpus", "{not-json", "corpus_json_invalid"),
        ("subset", "{not-json", "dry_run_subset_json_invalid"),
        ("subset", "{}", "dry_run_subset_schema_invalid"),
        ("corpus", "{}", "corpus_shape_invalid"),
    ],
)
def test_external_corpus_and_subset_failures_write_signed_blocker_reports(
    tmp_path, target, contents, blocker
):
    inputs = _inputs(tmp_path)
    inputs[target].write_text(contents, encoding="utf-8")
    phase_b = _phase_b(
        tmp_path,
        corpus_sha256=_sha256(inputs["corpus"]),
        turn_sha256=_sha256(inputs["turn"]),
        session_sha256=_sha256(inputs["session"]),
        subset_sha256=_sha256(inputs["subset"]),
    )

    report_path = locomo_input_qualification.qualify(
        phase_b_path=phase_b,
        corpus_matrix_path=_matrix(tmp_path),
        corpus_path=inputs["corpus"],
        turn_provenance_path=inputs["turn"],
        session_provenance_path=inputs["session"],
        dry_run_subset_path=inputs["subset"],
        artifact_root=tmp_path / "artifacts",
    )

    report = json.loads(report_path.read_text(encoding="utf-8"))
    assert report["qualification_status"] == "blocked"
    assert report["blockers"] == [blocker]
    assert report["report_sha256"] == locomo_input_qualification.report_sha256(report)
    assert set(report["artifacts"]) == {"trace_projection", "parent_relation_proof"}


def test_unsafe_phase_b_control_document_still_hard_rejects(tmp_path):
    inputs = _inputs(tmp_path)
    phase_b = _write_json(tmp_path / "phase-b.json", {"external_inputs": {}})

    with pytest.raises(locomo_input_qualification.QualificationError, match="Phase-B configuration"):
        locomo_input_qualification.qualify(
            phase_b_path=phase_b,
            corpus_matrix_path=_matrix(tmp_path),
            corpus_path=inputs["corpus"],
            turn_provenance_path=inputs["turn"],
            session_provenance_path=inputs["session"],
            dry_run_subset_path=inputs["subset"],
            artifact_root=tmp_path / "artifacts",
        )
