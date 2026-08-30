"""GRAPH-01 protected bridge-completion contract tests."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from experiments.graph_01 import (
    Graph01Error,
    PaidState,
    admit_relations,
    answer_f1_decision,
    bootstrap_paired_mean,
    parse_answer,
    parse_edge_audit,
    projection_items,
    protected_bridge_ranking,
    retrieval_decision,
)


def _paragraphs() -> list[dict[str, object]]:
    return [
        {"idx": 0, "title": "Seed", "text": "Beta is a seeded organization."},
        {"idx": 1, "title": "Distractor", "text": "Nothing useful."},
        {"idx": 10, "title": "Bridge", "text": "Alpha works with Beta."},
    ]


def _extractions() -> dict[str, dict[str, object]]:
    return {
        "q1#0": {
            "entities": [{"name": "Beta", "type": "Organization"}],
            "relations": [],
        },
        "q1#1": {"entities": [], "relations": []},
        "q1#10": {
            "entities": [
                {"name": "Alpha", "type": "Person"},
                {"name": "Beta", "type": "Organization"},
            ],
            "relations": [
                {"subject": "Alpha", "predicate": "works with", "object": "Beta"}
            ],
        },
    }


def test_relation_admission_requires_verbatim_typed_endpoints_and_exact_source() -> None:
    admitted, report = admit_relations("q1", _paragraphs(), _extractions())
    assert len(admitted) == 1
    edge = admitted[0]
    assert edge.subject == "alpha"
    assert edge.object == "beta"
    assert edge.source_id == "q1#10"
    assert report["source_link_completeness"] == 1.0
    assert report["endpoint_orphans"] == 0

    broken = _extractions()
    broken["q1#10"]["entities"] = [{"name": "Alpha", "type": "Person"}]
    admitted, report = admit_relations("q1", _paragraphs(), broken)
    assert admitted == []
    assert report["rejected_missing_endpoint"] == 1


def test_type_conflict_and_generic_entities_are_ineligible() -> None:
    extractions = _extractions()
    extractions["q1#0"]["entities"] = [
        {"name": "Beta", "type": "Place"},
        {"name": "Entity", "type": "Concept"},
    ]
    extractions["q1#0"]["relations"] = [
        {"subject": "Entity", "predicate": "mentions", "object": "Beta"}
    ]
    admitted, report = admit_relations("q1", _paragraphs(), extractions)
    assert admitted == []
    assert report["type_conflict_entities"] == 1
    # Both extracted relations touch the conflicting Beta entity.
    assert report["rejected_type_conflict"] == 2
    assert report["rejected_generic_endpoint"] == 1


def test_projection_items_keep_paragraph_provenance_on_edges() -> None:
    admitted, _ = admit_relations("q1", _paragraphs(), _extractions())
    items = projection_items("q1", _paragraphs(), _extractions(), admitted)
    edges = [item["edge"] for item in items if "edge" in item]
    assert edges == [
        {
            "kind": "relation",
            "from": "q1|ent:alpha",
            "to": "q1|ent:beta",
            "logical_id": "q1#10|edge:0",
            "source_id": "q1#10",
        }
    ]


def test_bridge_ranking_promotes_only_bounded_candidate_and_protects_top_eight() -> None:
    admitted, _ = admit_relations("q1", _paragraphs(), _extractions())
    baseline = list(range(20))
    result = protected_bridge_ranking(
        question="What organization does Alpha work with?",
        baseline=baseline,
        paragraph_entities={0: {"beta"}, 1: set(), 10: {"alpha", "beta"}},
        edges=admitted,
        seed_passages=5,
        protected_ranks=8,
        promotion_max=2,
        candidate_depth=20,
        context_passages=10,
    )
    assert result.ranking[:8] == baseline[:8]
    assert result.ranking == [0, 1, 2, 3, 4, 5, 6, 7, 10, 8]
    assert result.promoted == (10,)
    assert result.path_depths == (1,)


def test_bridge_ranking_does_not_promote_outside_candidate_depth() -> None:
    admitted, _ = admit_relations("q1", _paragraphs(), _extractions())
    # Put paragraph 10 at rank 21, strictly outside the registered top-20 pool.
    baseline = list(range(10)) + list(range(11, 21)) + [10]
    result = protected_bridge_ranking(
        question="What organization does Alpha work with?",
        baseline=baseline,
        paragraph_entities={0: {"beta"}, 10: {"alpha", "beta"}},
        edges=admitted,
        seed_passages=5,
        protected_ranks=8,
        promotion_max=2,
        candidate_depth=20,
        context_passages=10,
    )
    assert result.ranking == baseline[:10]
    assert result.promoted == ()


def test_bootstrap_and_retrieval_rule_are_paired_and_all_boundary() -> None:
    delta = bootstrap_paired_mean([0.0] * 20, [1.0] * 20, draws=200, seed=7)
    assert delta["point"] == 1.0
    assert delta["ci95"] == [1.0, 1.0]
    passing = {
        "quality_eligible": True,
        "lifecycle_eligible": True,
        "complete_bridge_delta": {"point": 0.05, "ci95": [0.01, 0.09]},
        "supporting_recall_delta": 0.01,
        "two_hop_complete_bridge_delta": -0.01,
        "distinct_question_rate": 0.2,
        "graph_addon_p95_ms": 2.0,
        "storage_amplification": 0.8,
    }
    assert retrieval_decision(passing)["passed"] is True
    passing["complete_bridge_delta"] = {"point": 0.05, "ci95": [0.0, 0.09]}
    assert retrieval_decision(passing)["passed"] is False


def test_answer_decision_allows_registered_retrieval_or_material_answer_route() -> None:
    assert answer_f1_decision(
        retrieval_passed=True,
        supporting_recall_delta=0.01,
        answer_delta={"point": 0.0, "ci95": [-0.01, 0.02]},
    )["accepted"]
    assert answer_f1_decision(
        retrieval_passed=False,
        supporting_recall_delta=0.01,
        answer_delta={"point": 0.05, "ci95": [0.01, 0.08]},
    )["accepted"]
    assert not answer_f1_decision(
        retrieval_passed=True,
        supporting_recall_delta=0.01,
        answer_delta={"point": -0.03, "ci95": [-0.05, 0.0]},
    )["accepted"]


def test_paid_state_is_atomic_resumable_and_fails_before_crossing_cap(tmp_path: Path) -> None:
    path = tmp_path / "checkpoint.json"
    state = PaidState.new("abc", 1.0)
    state.complete("judge/0", {"ok": True}, cost_usd=0.4)
    state.save(path)
    loaded = PaidState.load(path, "abc", 1.0)
    assert loaded.missing(["judge/0", "judge/1"]) == ["judge/1"]
    with pytest.raises(Graph01Error, match="cost cap"):
        loaded.complete("judge/1", {"ok": True}, cost_usd=0.7)
    assert json.loads(path.read_text())["cost_usd"] == pytest.approx(0.4)


def test_model_outputs_are_strictly_parsed() -> None:
    audit = parse_edge_audit(
        '{"edges":[{"edge_id":"e1","supported":true},'
        '{"edge_id":"e2","supported":false}]}'
    )
    assert audit == {"e1": True, "e2": False}
    assert parse_answer('{"answer":"Beta"}') == "Beta"
    with pytest.raises(Graph01Error):
        parse_edge_audit('{"edges":[{"edge_id":"e1","supported":"yes"}]}')
    with pytest.raises(Graph01Error):
        parse_answer('{"answer":"Beta","explanation":"extra"}')
