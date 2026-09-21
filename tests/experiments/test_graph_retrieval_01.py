from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path
from types import SimpleNamespace

import pytest

from experiments import graph_retrieval_01


ROOT = Path(__file__).resolve().parents[2]
CONFIG = ROOT / "experiments/configs/graph-retrieval-01/musique-native-expand.v1.json"


def test_checked_in_config_freezes_candidate_and_evaluation_policy() -> None:
    config = graph_retrieval_01.load_config(CONFIG)

    assert config.program_track == "GRAPH-RETRIEVAL-01"
    assert config.policy == {
        "evaluation_depth": 20,
        "context_depth": 10,
        "protected_prefix": 8,
        "promotion_limit": 2,
        "deduplicate_by": "paragraph_id",
        "tie_break": ["hop", "seed_rank", "passage_id"],
    }
    assert config.bootstrap == {"draws": 2000, "seed": 20260921}

    document = copy.deepcopy(config.resolved)
    document["policy"]["promotion_limit"] = 3
    with pytest.raises(graph_retrieval_01.GraphRetrievalBenchmarkError, match="policy"):
        graph_retrieval_01.resolve_config(document)


def _seed_manifest() -> dict[str, object]:
    rankings = [
        {"question_id": "q1", "passage_ids": [f"p{i:02d}" for i in range(20)]},
        {"question_id": "q2", "passage_ids": [f"x{i:02d}" for i in range(20)]},
    ]
    digest = hashlib.sha256(
        json.dumps(rankings, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    return {
        "schema_version": "graph-retrieval-01.seed-manifest.v1",
        "cohort": "historical-300",
        "question_count": 2,
        "ranking_depth": 20,
        "model": {
            "repository": "BAAI/bge-small-en-v1.5",
            "revision": "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a",
            "pooling": "cls",
            "device": "cuda",
            "gpu_uuid": "GPU-11111111-1111-1111-1111-111111111111",
        },
        "historical_top10_parity": {"matched": 2, "required": 2},
        "rankings": rankings,
        "rankings_sha256": digest,
        "materializer_sha256": "a" * 64,
    }


def test_seed_manifest_rejects_cpu_drift_and_rank_drift() -> None:
    manifest = _seed_manifest()
    graph_retrieval_01.validate_seed_manifest(manifest)

    cpu = copy.deepcopy(manifest)
    cpu["model"]["device"] = "cpu"
    with pytest.raises(graph_retrieval_01.GraphRetrievalBenchmarkError, match="CUDA"):
        graph_retrieval_01.validate_seed_manifest(cpu)

    drifted = copy.deepcopy(manifest)
    drifted["rankings"][0]["passage_ids"][0] = "other"
    with pytest.raises(graph_retrieval_01.GraphRetrievalBenchmarkError, match="digest"):
        graph_retrieval_01.validate_seed_manifest(drifted)


def test_seed_ranking_uses_one_cls_batch_with_the_historical_query_prefix() -> None:
    calls: list[list[str]] = []

    def embed_batch_cls(texts: list[str]) -> list[list[float]]:
        calls.append(texts)
        return [[1.0, 0.0], [0.0, 1.0], [1.0, 0.0]]

    question = SimpleNamespace(
        question="needle",
        paragraphs=[
            SimpleNamespace(body="unrelated"),
            SimpleNamespace(body="needle"),
        ],
    )

    ranking = graph_retrieval_01._fused_seed_ranking(question, embed_batch_cls)

    assert calls == [
        [
            "Represent this sentence for searching relevant passages: needle",
            "unrelated",
            "needle",
        ]
    ]
    assert ranking == [1, 0]


def test_historical_source_revision_ids_are_unique_per_artifact() -> None:
    first = graph_retrieval_01._historical_source_revision_ids("q1", 0, "same")
    repeated = graph_retrieval_01._historical_source_revision_ids("q1", 0, "same")
    other_question = graph_retrieval_01._historical_source_revision_ids(
        "q2", 0, "same"
    )

    assert first == repeated
    assert first != other_question


def test_candidate_policy_preserves_top20_and_top10_context() -> None:
    baseline = [f"p{i:02d}" for i in range(20)]
    result = graph_retrieval_01.promote_candidates(
        baseline,
        [
            {"passage_id": "bridge-b", "hop": 2, "seed_rank": 1},
            {"passage_id": "bridge-a", "hop": 1, "seed_rank": 3},
            {"passage_id": "bridge-a", "hop": 1, "seed_rank": 4},
        ],
        protected_prefix=8,
        promotion_limit=2,
        evaluation_depth=20,
    )

    assert result[:8] == baseline[:8]
    assert result[8:10] == ["bridge-a", "bridge-b"]
    assert len(result) == 20
    assert len(set(result)) == 20


def test_retrieval_metrics_are_separate_and_hop_stratified() -> None:
    rows = [
        {
            "question_id": "q1",
            "hop_count": 2,
            "gold": ["p1", "p2"],
            "control": ["p1", "x", "y", "z", "w"],
            "treatment": ["p1", "p2", "x", "y", "z"],
        },
        {
            "question_id": "q2",
            "hop_count": 3,
            "gold": ["g"],
            "control": ["g", "x"],
            "treatment": ["x", "g"],
        },
    ]
    metrics = graph_retrieval_01.score_rankings(rows, cutoffs=(5, 10, 20), draws=20, seed=7)

    assert set(metrics) == {"retrieval_quality", "paired_deltas", "denominators"}
    assert metrics["retrieval_quality"]["overall"]["complete_support_at_5"] == 1.0
    assert set(metrics["retrieval_quality"]["by_hop"]) == {"2", "3"}
    assert metrics["denominators"]["attempted"] == 2


def test_stark_remains_a_typed_blocker_without_a_score() -> None:
    status = graph_retrieval_01.stark_qualification_state()

    assert status["state"] == "blocked_prerequisite"
    assert status["numeric_tolerance"] == 1e-12
    assert status["score"] is None
