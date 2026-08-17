"""Synthetic, no-live contract tests for the LOCOMO external cell adapter."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from experiments import locomo_external_adapter as adapter


def _write_inputs(tmp_path: Path, *, question_count: int = 32) -> dict[str, str]:
    questions = [
        {
            "question": f"question {index}",
            "evidence": ["turn-1"],
            "category": (index % 3) + 1,
        }
        for index in range(question_count)
    ]
    corpus = [
        {
            "conversation": {
                "speaker_a": "Ada",
                "session_1": [
                    {
                        "speaker": "Ada",
                        "text": "raw corpus text must not leak",
                        "dia_id": "turn-1",
                    },
                ],
            },
            "qa": questions,
        }
    ]
    corpus_path = tmp_path / "corpus.json"
    corpus_path.write_text(json.dumps(corpus), encoding="utf-8")
    turns, sessions = adapter.provenance_documents_for_corpus(corpus)
    turn_path, session_path = tmp_path / "turn.json", tmp_path / "session.json"
    turn_path.write_text(json.dumps(turns), encoding="utf-8")
    session_path.write_text(json.dumps(sessions), encoding="utf-8")
    subset_path = tmp_path / "subset.json"
    subset_path.write_text(
        json.dumps(
            {
                "schema_version": "locomo-fixed-subset.v1",
                "question_ids": [
                    f"locomo-0-q-{index}" for index in range(question_count)
                ],
            }
        ),
        encoding="utf-8",
    )
    trace_path = tmp_path / "trace.json"
    trace_path.write_text(
        json.dumps(adapter.synthetic_trace_sidecar("source-1")), encoding="utf-8"
    )
    relation_path = tmp_path / "relations.json"
    relation_path.write_text(
        json.dumps(
            adapter.relation_proof_document(
                turns,
                sessions,
                child_id="turn-1",
                session_id="session_1",
                source_id="source-1",
            )
        ),
        encoding="utf-8",
    )
    return {
        "corpus": str(corpus_path),
        "turn_provenance": str(turn_path),
        "session_provenance": str(session_path),
        "dry_run_subset": str(subset_path),
        "trace_projection": str(trace_path),
        "parent_relation_proof": str(relation_path),
    }


def _request(
    tmp_path: Path, *, device: str = "cpu", treatment: str = "fts_only"
) -> dict[str, object]:
    return {
        "schema_version": "locomo-live-executor.request.v1",
        "release_id": "release-1",
        "action": "fixed_subset_dry_run",
        "mode": "dry_run",
        "cell": {
            "cell_id": f"turn--{treatment}--{device}--cold",
            "program_track": "LOCOMO-01",
            "ingest_unit": "turn",
            "treatment": treatment,
            "retrieval": "fts" if treatment.startswith("fts") else "hybrid",
            "runtime": {"device": device, "cache_state": "cold"},
            "parent_child": None,
        },
        "external_inputs": _write_inputs(tmp_path),
        "output_root": str(tmp_path / "output"),
    }


class _Hit:
    def __init__(self, logical_id: str) -> None:
        self.id = type("Id", (), {"value": logical_id})()


class _Result:
    def __init__(self, logical_id: str) -> None:
        self.results = [_Hit(logical_id)]


class _Engine:
    def __init__(self) -> None:
        self.calls: list[tuple[str, object]] = []
        self.logical_id = "turn-1"

    def write(self, rows):  # noqa: ANN001
        self.calls.append(("write", len(rows)))

    def drain(self, *, timeout_s: int) -> None:
        self.calls.append(("drain", timeout_s))

    def search_text_only(self, query: str, *, limit: int):
        self.calls.append(("fts", query))
        return _Result(self.logical_id)

    def search(self, query: str, **kwargs):
        self.calls.append(("hybrid", kwargs))
        return _Result(self.logical_id)

    def dense_disabled(self) -> bool:
        return False

    def close(self) -> None:
        self.calls.append(("close", None))


def test_adapter_emits_complete_safe_cell_result_and_uses_the_fts_engine_path(tmp_path):
    engine = _Engine()
    result = adapter.execute_request(
        _request(tmp_path), engine_factory=lambda _path, _dense: engine
    )

    assert set(result) == {
        "schema_version",
        "cell_id",
        "mode",
        "external_metrics_ref",
        "external_metrics_sha256",
        "metric_summary",
    }
    assert result["schema_version"] == "locomo-live-executor.cell-result.v1"
    assert result["metric_summary"]["m1"]["r_at_10"] == 1.0
    assert any(name == "fts" for name, _ in engine.calls)
    serialized = json.dumps(result)
    metrics = next((tmp_path / "output").glob("*.json")).read_text(encoding="utf-8")
    assert "raw corpus text must not leak" not in serialized
    assert "question 0" not in serialized
    assert "raw corpus text must not leak" not in metrics
    assert "question 0" not in metrics


def test_adapter_rejects_unknown_request_fields_before_loading_external_inputs(
    tmp_path,
):
    request = _request(tmp_path)
    request["unexpected"] = "bypass"

    with pytest.raises(adapter.AdapterError, match="request keys mismatch"):
        adapter.execute_request(request, engine_factory=lambda _path, _dense: _Engine())


def test_gpu_cell_never_silently_uses_cpu_when_cuda_attestation_fails(
    tmp_path, monkeypatch
):
    request = _request(tmp_path, device="gpu", treatment="hybrid")
    monkeypatch.setattr(
        adapter,
        "_require_single_visible_cuda",
        lambda: (_ for _ in ()).throw(
            adapter.AdapterError("CUDA device is unavailable")
        ),
    )

    with pytest.raises(adapter.AdapterError, match="CUDA device is unavailable"):
        adapter.execute_request(request, engine_factory=lambda _path, _dense: _Engine())


def test_parent_cell_uses_hybrid_retrieval_and_returns_only_proven_parent_hits(
    tmp_path,
):
    engine = _Engine()
    request = _request(tmp_path, treatment="parent_child_turn_session_v1")
    request["cell"]["program_track"] = "PARENT-01"
    request["cell"]["retrieval"] = "hybrid"
    request["cell"]["parent_child"] = adapter.PARENT_CHILD_FROZEN

    result = adapter.execute_request(
        request, engine_factory=lambda _path, _dense: engine
    )

    assert any(name == "hybrid" for name, _ in engine.calls)
    assert result["parent_hits"] == [
        {
            "child_id": "turn-1",
            "rank": 1,
            "child_provenance": {
                "parent_session_ids": ["session_1"],
                "ordinal": 0,
                "trace_source_id": "source-1",
            },
            "neighbors": [],
        }
    ]
    assert "raw corpus text must not leak" not in json.dumps(result)


def test_adapter_stdout_parser_rejects_duplicate_keys_and_parent_requires_relation_proof(
    tmp_path,
):
    with pytest.raises(adapter.AdapterError, match="duplicate JSON key"):
        adapter.parse_request_json('{"action":"one","action":"two"}')

    request = _request(tmp_path, treatment="parent_child_turn_session_v1")
    request["cell"]["program_track"] = "PARENT-01"
    request["cell"]["retrieval"] = "hybrid"
    request["cell"]["parent_child"] = adapter.PARENT_CHILD_FROZEN
    Path(request["external_inputs"]["parent_relation_proof"]).write_text(
        "{}", encoding="utf-8"
    )
    with pytest.raises(adapter.AdapterError, match="parent relation proof"):
        adapter.execute_request(request, engine_factory=lambda _path, _dense: _Engine())
