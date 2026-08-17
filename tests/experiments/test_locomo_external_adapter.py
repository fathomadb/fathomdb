"""Synthetic, no-live contract tests for the LOCOMO external cell adapter."""

from __future__ import annotations

import json
import math
from pathlib import Path

import pytest

from experiments import locomo_external_adapter as adapter
from experiments.locomo_provenance import build_manifest_document, canonical_turn_id


def _canonical_turn(raw_turn_id: str) -> str:
    return canonical_turn_id("locomo-0", "session_1", raw_turn_id)


def _write_inputs(
    tmp_path: Path,
    *,
    question_count: int = 32,
    turn_ids: tuple[str, ...] = ("turn-1",),
    evidence_id: str = "turn-1",
) -> dict[str, str]:
    tmp_path.mkdir(parents=True, exist_ok=True)
    questions = [
        {
            "question": f"question {index}",
            "answer": "fixture answer",
            "evidence": [evidence_id],
            "category": (index % 3) + 1,
        }
        for index in range(question_count)
    ]
    corpus = [
        {
            "conversation": {
                "speaker_a": "Ada",
                "session_1_date_time": "01:00 PM on 15 January, 2024",
                "session_1": [
                    {
                        "speaker": "Ada",
                        "text": f"raw corpus text {turn_id} must not leak",
                        "dia_id": turn_id,
                    }
                    for turn_id in turn_ids
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
    relations = adapter.relation_proof_document(
        turns,
        sessions,
        conversation_id="locomo-0",
        child_id=turn_ids[0],
        session_id="session_1",
        source_id="source-1",
    )
    for child_id in turn_ids[1:]:
        relations["entries"].extend(
            adapter.relation_proof_document(
                turns,
                sessions,
                conversation_id="locomo-0",
                child_id=child_id,
                session_id="session_1",
                source_id="source-1",
            )["entries"]
        )
    relation_path.write_text(json.dumps(relations), encoding="utf-8")
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
    def __init__(self, logical_ids: list[str]) -> None:
        self.results = [_Hit(logical_id) for logical_id in logical_ids]


class _Engine:
    def __init__(self) -> None:
        self.calls: list[tuple[str, object]] = []
        self.logical_ids = [_canonical_turn("turn-1")]

    def write(self, rows):  # noqa: ANN001
        self.calls.append(("write", len(rows)))

    def drain(self, *, timeout_s: int) -> None:
        self.calls.append(("drain", timeout_s))

    def search_text_only(self, query: str, *, limit: int):
        self.calls.append(("fts", query))
        return _Result(self.logical_ids)

    def search(self, query: str, **kwargs):
        self.calls.append(("hybrid", kwargs))
        return _Result(self.logical_ids)

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


def test_turn_ingestion_uses_scoped_canonical_ids_for_repeated_raw_turn_ids():
    corpus = [
        {
            "conversation": {
                "speaker_a": "Ada",
                "session_1": [{"speaker": "Ada", "text": "one", "dia_id": "shared-turn"}],
            },
            "qa": [],
        },
        {
            "conversation": {
                "speaker_a": "Bea",
                "session_1": [{"speaker": "Bea", "text": "two", "dia_id": "shared-turn"}],
            },
            "qa": [],
        },
    ]
    turns, _ = adapter.provenance_documents_for_corpus(corpus)
    manifest = adapter._manifest(turns, "turn provenance")

    rows, evidence = adapter._ingest_rows(corpus, ingest_unit="turn", manifest=manifest)

    expected = {
        canonical_turn_id("locomo-0", "session_1", "shared-turn"),
        canonical_turn_id("locomo-1", "session_1", "shared-turn"),
    }
    assert {row["logical_id"] for row in rows} == expected
    assert set(evidence) == expected


def test_adapter_ingestion_accepts_the_canonical_external_provenance_manifest():
    """The deployed adapter and preflight provenance derivation share one payload identity."""
    corpus = [
        {
            "conversation": {
                "speaker_a": "Ada",
                "session_1_date_time": "01:00 PM on 15 January, 2024",
                "session_1": [
                    {"speaker": "Ada", "text": "one", "dia_id": "turn-1"},
                    {"speaker": "Bea", "text": "two", "dia_id": "turn-2"},
                ],
            },
            "qa": [],
        }
    ]
    manifest = adapter._manifest(
        build_manifest_document(corpus, ingest_unit="turn"), "turn provenance"
    )

    rows, evidence = adapter._ingest_rows(corpus, ingest_unit="turn", manifest=manifest)

    assert len(rows) == 2
    assert len(evidence) == 2


def test_parent_metrics_resolve_raw_evidence_within_the_question_conversation_scope():
    child_id = canonical_turn_id("locomo-0", "session_1", "shared-turn")
    questions = [
        {"category": category, "evidence": {"shared-turn"}, "conversation_id": "locomo-0"}
        for category in (1, 2, 4)
    ]
    bundles = [
        [{"parent_session_id": "scoped-session", "child_hit_count": 1, "ordered_neighbor_ids": []}]
        for _ in questions
    ]
    metrics = adapter._metrics(
        questions,
        [["shared-turn"] for _ in questions],
        ingest_ack_ms=1.0,
        ready_ms=1.0,
        query_ms=[1.0, 1.0, 1.0],
        parent=True,
        relations={
            child_id: {
                "parent_session_id": "scoped-session",
                "_conversation_id": "locomo-0",
                "_raw_turn_id": "shared-turn",
            }
        },
        parent_bundles=bundles,
    )

    assert metrics["parent_metrics"]["parent_session_recall"] == 1.0


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
    request["action"] = "gpu_ce_grid"
    request["mode"] = "full_grid"
    monkeypatch.setattr(
        adapter,
        "_require_single_visible_cuda",
        lambda: (_ for _ in ()).throw(
            adapter.AdapterError("CUDA device is unavailable")
        ),
    )

    with pytest.raises(adapter.AdapterError, match="CUDA device is unavailable"):
        adapter.execute_request(request, engine_factory=lambda _path, _dense: _Engine())


def test_cuda_visible_device_zero_accepts_gpu_zero_on_a_multi_gpu_host(monkeypatch):
    """A device mask, not physical GPU enumeration, determines CUDA visibility."""
    monkeypatch.setenv("CUDA_VISIBLE_DEVICES", "0")
    monkeypatch.setattr(
        adapter.subprocess,
        "run",
        lambda *_args, **_kwargs: adapter.subprocess.CompletedProcess(
            ["nvidia-smi"], 0, stdout="0\n1\n2\n", stderr=""
        ),
    )

    assert adapter._require_single_visible_cuda() == "cuda:0"


def _parent_request(tmp_path, *, turn_ids=("turn-1",), evidence_id="turn-1"):
    request = _request(tmp_path, treatment="parent_child_turn_session_v1")
    request["external_inputs"] = _write_inputs(
        tmp_path, turn_ids=turn_ids, evidence_id=evidence_id
    )
    request["cell"]["program_track"] = "PARENT-01"
    request["cell"]["retrieval"] = "hybrid"
    request["cell"]["parent_child"] = adapter.PARENT_CHILD_FROZEN
    return request


def test_remediation_rejects_nonfrozen_cells_and_action_partitions_before_loading_inputs(
    tmp_path,
):
    arbitrary = _request(tmp_path)
    arbitrary["cell"]["cell_id"] = "turn--invented--cpu--cold"
    arbitrary["cell"]["treatment"] = "invented"
    with pytest.raises(adapter.AdapterError, match="frozen"):
        adapter.execute_request(
            arbitrary, engine_factory=lambda _path, _dense: _Engine()
        )


def test_remediation_rejects_cells_outside_the_released_action_partition(tmp_path):
    wrong_partition = _request(tmp_path)
    wrong_partition["action"] = "cpu_grid"
    wrong_partition["mode"] = "full_grid"
    wrong_partition["cell"]["runtime"]["device"] = "gpu"
    wrong_partition["cell"]["cell_id"] = "turn--fts_only--gpu--cold"
    with pytest.raises(adapter.AdapterError, match="action partition"):
        adapter.execute_request(
            wrong_partition, engine_factory=lambda _path, _dense: _Engine()
        )


def test_remediation_preserves_rank_for_metrics_and_parent_child_proof(tmp_path):
    engine = _Engine()
    engine.logical_ids = [_canonical_turn("turn-2"), _canonical_turn("turn-1")]
    ranked_request = _request(tmp_path)
    ranked_request["external_inputs"] = _write_inputs(
        tmp_path, turn_ids=("turn-1", "turn-2"), evidence_id="turn-1"
    )
    locomo_result = adapter.execute_request(
        ranked_request, engine_factory=lambda _path, _dense: engine
    )
    assert locomo_result["metric_summary"]["m2"] == {
        "mrr": 0.5,
        "r_at_1": 0.0,
        "ndcg_at_10": pytest.approx(1 / math.log2(3)),
    }

    parent_engine = _Engine()
    parent_engine.logical_ids = [_canonical_turn("turn-1")]
    parent = adapter.execute_request(
        _parent_request(tmp_path / "parent"),
        engine_factory=lambda _path, _dense: parent_engine,
    )
    assert parent["parent_hits"][0]["rank"] == 1
    assert "raw corpus text" not in json.dumps(parent)


def test_remediation_derives_parent_duplicate_and_context_metrics_from_bundles(
    tmp_path,
):
    engine = _Engine()
    engine.logical_ids = [_canonical_turn("turn-2"), _canonical_turn("turn-1")]
    request = _parent_request(
        tmp_path, turn_ids=("turn-1", "turn-2", "turn-3"), evidence_id="turn-2"
    )

    result = adapter.execute_request(
        request, engine_factory=lambda _path, _dense: engine
    )

    assert any(name == "hybrid" for name, _ in engine.calls)
    assert result["parent_hits"][0]["child_id"] == _canonical_turn("turn-2")
    assert result["parent_hits"][0]["rank"] == 1
    assert result["metric_summary"]["parent_metrics"]["duplicate_rate"] == 0.5
    assert result["metric_summary"]["parent_metrics"]["context_expansion_count"] == 2


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


def test_remediation_rejects_forged_relation_ordinals_and_repository_output_escape(
    tmp_path,
):
    request = _parent_request(tmp_path)
    relation_path = Path(request["external_inputs"]["parent_relation_proof"])
    relation = json.loads(relation_path.read_text(encoding="utf-8"))
    relation["entries"][0]["ordinal"] = 9
    relation["entries"][0]["session_members"][0]["ordinal"] = 9
    relation_path.write_text(json.dumps(relation), encoding="utf-8")
    with pytest.raises(adapter.AdapterError, match="canonical provenance"):
        adapter.execute_request(request, engine_factory=lambda _path, _dense: _Engine())


def test_remediation_rejects_repository_and_historical_output_escape_at_abi_boundary(
    tmp_path,
):
    escaped = _request(tmp_path / "escape")
    escaped["output_root"] = str(
        adapter._REPOSITORY_ROOT / "experiments" / "runs" / "adapter-escape"
    )
    with pytest.raises(adapter.AdapterError, match="outside the repository"):
        adapter._validate_request(escaped)
