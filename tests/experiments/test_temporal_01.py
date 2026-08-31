"""Human-intended guardrails for TEMPORAL-01 factual preparation."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from experiments import temporal_01


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _write_fixture_inputs(root: Path) -> tuple[dict[str, object], Path]:
    timeqa = root / "timeqa"
    timeqa.mkdir()
    for name in ("test.easy.json", "test.hard.json"):
        (timeqa / name).write_text(
            json.dumps(
                {
                    "idx": name,
                    "context": "external corpus text",
                    "paragraphs": [{"title": "title", "text": "text"}],
                    "question": "external corpus question",
                    "targets": [] if name == "test.hard.json" else ["answer"],
                }
            )
            + "\n",
            encoding="utf-8",
        )
    longmemeval = root / "longmemeval-cleaned"
    longmemeval.mkdir()
    lme_path = longmemeval / "longmemeval_s_cleaned.json"
    lme_path.write_text(
        json.dumps(
            [
                {
                    "question_id": "external-id",
                    "question_type": "temporal-reasoning",
                    "question": "external corpus question",
                    "answer": "external answer",
                    "question_date": "2025-01-02",
                    "haystack_session_ids": ["session-1"],
                    "haystack_dates": ["2025-01-01"],
                    "haystack_sessions": [[{"role": "user", "content": "external", "has_answer": True}]],
                    "answer_session_ids": ["session-1"],
                }
            ]
        ),
        encoding="utf-8",
    )
    timelineqa = root / "timelineqa"
    timelineqa.mkdir()
    (timelineqa / "index.json").write_text(
        json.dumps(
            {
                "final_year": 2022,
                "seed_base": 12345,
                "n_personas_per_density": 1,
                "total_personas": 3,
                "total_atomic_qa": 30,
                "per_density": {
                    density: {"personas": 1, "atomic_qa": 10}
                    for density in ("sparse", "medium", "dense")
                },
            }
        ),
        encoding="utf-8",
    )
    registry = {
        "sources": {
            "timeqa": {
                "license": "BSD-3-Clause",
                "files": {
                    name: {"sha256": _sha256(timeqa / name), "qa_count": 1}
                    for name in ("test.easy.json", "test.hard.json")
                },
            },
            "longmemeval": {
                "license": "MIT",
                "upstream": {"revision": "98d7416c24c778c2fee6e6f3006e7a073259d48f"},
                "license_sha256": "a" * 64,
                "files": {
                    "longmemeval_s_cleaned.json": {
                        "sha256": _sha256(lme_path),
                        "instances": 1,
                        "question_type_counts": {"temporal-reasoning": 1},
                    }
                },
            },
            "timelineqa": {
                "license": "CC-BY-NC-4.0",
                "upstream": {
                    "revision": "abcdef123456",
                    "params": {
                        "final_year": 2022,
                        "seed_base": 12345,
                        "n_personas_per_density": 1,
                        "densities": ["sparse", "medium", "dense"],
                    },
                },
                "per_density": {
                    density: {"personas": 1, "atomic_qa": 10}
                    for density in ("sparse", "medium", "dense")
                },
            },
        }
    }
    return registry, root


def _policy(registry: dict[str, object]) -> dict[str, object]:
    return {
        "schema_version": "temporal-01-factual-preflight-policy.v1",
        "program_track": "TEMPORAL-01",
        "preflight_id": "temporal-01-input-facts-v1",
        "registry_sha256": temporal_01.canonical_sha256(registry),
        "inputs": {
            "timeqa": {"files": ["test.easy.json", "test.hard.json"]},
            "longmemeval": {"file": "longmemeval_s_cleaned.json", "class": "temporal-reasoning"},
            "timelineqa": {"densities": ["sparse", "medium", "dense"]},
        },
        "baseline": "a0_turn_fts",
        "treatment": "readview_valid_as_of",
        "claim_boundary": "world_time_validity_only_no_history_as_of_or_supersession_claim",
    }


def test_preflight_binds_registered_inputs_and_does_not_emit_payload(tmp_path):
    registry, data_root = _write_fixture_inputs(tmp_path)

    result = temporal_01.preflight(_policy(registry), registry, data_root)

    assert result.state == "input_facts_confirmed_treatment_blocked"
    report = result.safe_report()
    assert report["corpora"]["timeqa"]["question_count"] == 2
    assert report["corpora"]["timeqa"]["unanswerable_count"] == 1
    assert report["corpora"]["longmemeval"]["temporal_question_count"] == 1
    assert report["corpora"]["timelineqa"]["atomic_qa_count"] == 30
    assert report["blockers"] == [
        "external_validity_window_manifest_missing",
        "history_as_of_not_supported",
        "temporal_adapter_not_implemented",
    ]
    serialized = json.dumps(report)
    assert "external corpus question" not in serialized
    assert "external answer" not in serialized
    assert str(data_root) not in serialized


def test_preflight_fails_closed_on_a_registered_payload_digest_mismatch(tmp_path):
    registry, data_root = _write_fixture_inputs(tmp_path)
    registry["sources"]["timeqa"]["files"]["test.easy.json"]["sha256"] = "0" * 64  # type: ignore[index]

    with pytest.raises(temporal_01.Temporal01PreflightError, match="TimeQA payload digest"):
        temporal_01.preflight(_policy(registry), registry, data_root)


def test_safe_report_writer_rejects_unknown_fields(tmp_path):
    registry, data_root = _write_fixture_inputs(tmp_path)
    report = temporal_01.preflight(_policy(registry), registry, data_root).safe_report()
    report["raw_payload"] = "must not be written"  # type: ignore[index]
    external = tmp_path / "external"
    external.mkdir()

    with pytest.raises(temporal_01.Temporal01PreflightError, match="report keys"):
        temporal_01.write_report(report, output_root=external, report_path=external / "preflight.json")


def test_safe_report_writer_rejects_a_repository_destination(tmp_path):
    registry, data_root = _write_fixture_inputs(tmp_path)
    report = temporal_01.preflight(_policy(registry), registry, data_root).safe_report()
    external = tmp_path / "external"
    external.mkdir()
    repository = Path(__file__).resolve().parents[2]

    with pytest.raises(temporal_01.Temporal01PreflightError, match="outside the repository"):
        temporal_01.write_report(report, output_root=external, report_path=repository / "preflight.json")
