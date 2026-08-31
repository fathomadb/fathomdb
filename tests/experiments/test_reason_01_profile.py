"""Human-intended contract tests for the REASON-01 protected profile."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from types import SimpleNamespace

import pytest
from hypothesis import given, strategies as st

from experiments import reason_01_equivalence, reason_01_preflight, reason_01_profile


REGISTRY = Path("experiments/configs/reason-01/profile-registry.v1.json")


@dataclass(frozen=True)
class _Id:
    value: str


@dataclass(frozen=True)
class _Hit:
    id: _Id
    body: str
    source_id: str


def _hit(value: str, *, body: str | None = None, source: str | None = None) -> _Hit:
    return _Hit(_Id(value), body or f"body-{value}", source or f"source-{value}")


def _registry() -> dict[str, object]:
    return dict(reason_01_profile.load_registry(REGISTRY))


def _runtime() -> dict[str, object]:
    profile = _registry()["profiles"][0]  # type: ignore[index]
    runtime = dict(profile["runtime"])
    runtime["doctor_ok"] = True
    return runtime


def test_registry_accepts_only_the_frozen_experimental_profile(tmp_path):
    registry = _registry()
    profile = registry["profiles"][0]  # type: ignore[index]
    assert profile["id"] == "protected_multiquery_v1"
    assert profile["status"] == "experimental"
    assert profile["intent"] == "relationship"
    assert profile["recipe"] == {
        "fts_prefix_limit": 10,
        "max_queries": 3,
        "hybrid_alpha": 1.0,
        "hybrid_pool_n": 20,
        "hybrid_rerank_depth": 20,
        "use_graph_arm": False,
        "explain": True,
        "hybrid_limit": 20,
        "context_limit": 20,
    }

    drifted = json.loads(REGISTRY.read_text(encoding="utf-8"))
    drifted["profiles"][0]["recipe"]["unexpected"] = True
    path = tmp_path / "registry.json"
    path.write_text(json.dumps(drifted), encoding="utf-8")
    with pytest.raises(reason_01_profile.Reason01Error, match="keys drifted"):
        reason_01_profile.load_registry(path)


@pytest.mark.parametrize("intent", [None, "exact", "semantic", "timeline", "global", "fast"])
def test_resolver_keeps_general_and_known_nonrelationship_intents_on_a0(intent):
    assert reason_01_profile.resolve_profile(_registry(), intent=intent) == "a0"


def test_resolver_requires_explicit_consistent_relationship_authority():
    registry = _registry()
    assert reason_01_profile.resolve_profile(registry, intent="relationship") == "protected_multiquery_v1"
    assert reason_01_profile.resolve_profile(registry, profile_override="protected_multiquery_v1") == "protected_multiquery_v1"
    assert reason_01_profile.resolve_profile(
        registry, intent="relationship", profile_override="protected_multiquery_v1"
    ) == "protected_multiquery_v1"
    with pytest.raises(reason_01_profile.Reason01Error, match="conflict"):
        reason_01_profile.resolve_profile(
            registry, intent="exact", profile_override="protected_multiquery_v1"
        )
    with pytest.raises(reason_01_profile.Reason01Error, match="unknown intent"):
        reason_01_profile.resolve_profile(registry, intent="multi_hop")
    with pytest.raises(reason_01_profile.Reason01Error, match="unknown profile"):
        reason_01_profile.resolve_profile(registry, profile_override="deep_compact_v1")


class _Engine:
    def __init__(self, *, cursor: int = 17) -> None:
        self.cursor = cursor
        self.calls: list[tuple[str, str, dict[str, object]]] = []

    def search_text_only(self, query: str, view: object = None, *, limit: int):
        self.calls.append(("fts", query, {"view": view, "limit": limit}))
        return SimpleNamespace(
            projection_cursor=self.cursor,
            results=[_hit("a"), _hit("b")],
            soft_fallback=None,
        )

    def search(self, query: str, filter: object = None, **kwargs: object):
        self.calls.append(("hybrid", query, {"filter": filter, **kwargs}))
        rows = {
            "How did Alice and Bob solve the database problem?": [_hit("a"), _hit("c")],
            "How Alice and Bob solve database problem": [_hit("d"), _hit("b")],
            "Alice Bob How and mentioned sessions": [_hit("e")],
        }
        return SimpleNamespace(
            projection_cursor=self.cursor,
            results=rows[query],
            soft_fallback=None,
        )


def test_executor_uses_the_exact_recipe_and_same_read_view():
    engine = _Engine()
    view = object()
    result = reason_01_profile.execute_profile(
        engine,
        "How did Alice and Bob solve the database problem?",
        _registry(),
        intent="relationship",
        view=view,
        runtime_attestation=_runtime(),
    )

    assert [hit.id.value for hit in result.hits] == ["a", "b", "c", "d", "e"]
    assert engine.calls[0] == ("fts", "How did Alice and Bob solve the database problem?", {"view": view, "limit": 10})
    assert [call[1] for call in engine.calls[1:]] == [
        "How did Alice and Bob solve the database problem?",
        "How Alice and Bob solve database problem",
        "Alice Bob How and mentioned sessions",
    ]
    for kind, _, kwargs in engine.calls[1:]:
        assert kind == "hybrid"
        assert kwargs == {
            "filter": None,
            "rerank_depth": 20,
            "use_graph_arm": False,
            "alpha": 1.0,
            "pool_n": 20,
            "explain": True,
            "view": view,
            "limit": 20,
        }


@given(
    prefix=st.lists(st.text(min_size=1, max_size=8), max_size=20),
    branches=st.lists(st.lists(st.text(min_size=1, max_size=8), max_size=25), max_size=3),
    limit=st.integers(min_value=1, max_value=20),
)
def test_merge_preserves_unique_prefix_is_deduplicated_bounded_and_deterministic(
    prefix: list[str], branches: list[list[str]], limit: int
):
    prefix_hits = [_hit(value) for value in prefix]
    branch_hits = [[_hit(value) for value in branch] for branch in branches]
    first = reason_01_profile.merge_hits(prefix_hits, branch_hits, limit=limit)
    second = reason_01_profile.merge_hits(prefix_hits, branch_hits, limit=limit)
    ids = [hit.id.value for hit in first]
    unique_prefix = list(dict.fromkeys(prefix))[:limit]
    assert ids[: len(unique_prefix)] == unique_prefix
    assert len(ids) == len(set(ids))
    assert len(ids) <= limit
    assert first == second


def test_executor_refuses_before_calls_without_authority_runtime_or_filter():
    for kwargs, message in [
        ({}, "profile is A0"),
        ({"intent": "relationship"}, "runtime attestation"),
        ({"intent": "relationship", "runtime_attestation": _runtime(), "metadata_filter": object()}, "metadata filter"),
    ]:
        engine = _Engine()
        with pytest.raises(reason_01_profile.Reason01Error, match=message):
            reason_01_profile.execute_profile(engine, "query", _registry(), **kwargs)
        assert engine.calls == []


def test_executor_rejects_runtime_identity_drift_before_engine_calls():
    engine = _Engine()
    runtime = _runtime()
    runtime["fathomdb_version"] = "0.8.22"

    with pytest.raises(reason_01_profile.Reason01Error, match="runtime attestation"):
        reason_01_profile.execute_profile(
            engine,
            "query",
            _registry(),
            intent="relationship",
            runtime_attestation=runtime,
        )

    assert engine.calls == []


def test_executor_refuses_projection_drift_and_attribution_conflict():
    engine = _Engine()
    original_search = engine.search

    def drifted(query: str, filter: object = None, **kwargs: object):
        result = original_search(query, filter, **kwargs)
        result.projection_cursor = 18
        return result

    engine.search = drifted  # type: ignore[method-assign]
    with pytest.raises(reason_01_profile.Reason01Error, match="projection cursor"):
        reason_01_profile.execute_profile(
            engine, "How did Alice and Bob solve the database problem?", _registry(),
            intent="relationship", runtime_attestation=_runtime()
        )

    conflict = _Engine()

    def conflicting(query: str, filter: object = None, **kwargs: object):
        result = _Engine.search(conflict, query, filter, **kwargs)
        if query == "How Alice and Bob solve database problem":
            result.results = [_hit("a", body="conflicting")]
        return result

    conflict.search = conflicting  # type: ignore[method-assign]
    with pytest.raises(reason_01_profile.Reason01Error, match="attribution"):
        reason_01_profile.execute_profile(
            conflict, "How did Alice and Bob solve the database problem?", _registry(),
            intent="relationship", runtime_attestation=_runtime()
        )


def test_safe_trace_contains_only_domain_separated_hashes():
    raw_query = "private relationship question"
    raw_body = "private corpus body"
    trace = reason_01_profile.make_safe_trace(
        profile_id="protected_multiquery_v1",
        intent="relationship",
        profile_override=None,
        queries=[raw_query, "private expansion"],
        selected_hits=[_hit("private-id", body=raw_body)],
        branch_counts=[1, 1],
        elapsed_ms=1.25,
    )
    serialized = json.dumps(trace, sort_keys=True)
    assert raw_query not in serialized
    assert raw_body not in serialized
    assert "private-id" not in serialized
    assert all(len(value) == 64 for value in trace["query_sha256s"])
    assert all(len(value) == 64 for value in trace["selected_identity_sha256s"])
    assert trace["query_sha256s"] != trace["selected_identity_sha256s"]


def test_equivalence_requires_exact_question_order_and_selected_ids():
    expected = [
        {"question_id": "q1", "arm_turn_ids": ["a", "b"]},
        {"question_id": "q2", "arm_turn_ids": ["c"]},
    ]
    assert reason_01_equivalence.verify_equivalence(expected, expected) == {
        "question_count": 2,
        "selected_id_rows_equal": True,
    }
    drifted = [*expected[:-1], {"question_id": "q2", "arm_turn_ids": ["d"]}]
    with pytest.raises(reason_01_equivalence.EquivalenceError, match="selected IDs drifted"):
        reason_01_equivalence.verify_equivalence(drifted, expected)


def test_preflight_selects_all_untouched_multisession_cases_and_rejects_overlap():
    rows = [
        {"question_id": "prior", "question_type": "multi-session", "answer_session_ids": ["s1"]},
        {"question_id": "keep-1", "question_type": "multi-session", "answer_session_ids": ["s2", "s3"]},
        {"question_id": "single", "question_type": "single-session-user", "answer_session_ids": ["s4"]},
        {"question_id": "keep-2", "question_type": "multi-session", "answer_session_ids": ["s5"]},
    ]
    selected = reason_01_preflight.select_heldout_cases(rows, ["prior"], expected_count=2)
    assert [row["question_id"] for row in selected] == ["keep-1", "keep-2"]
    manifest = reason_01_preflight.build_content_free_manifest(
        selected,
        source_sha256="a" * 64,
        exclusions_sha256="b" * 64,
        selection_code_sha256="c" * 64,
        config_sha256="d" * 64,
    )
    assert manifest["question_ids"] == ["keep-1", "keep-2"]
    assert "answer_session_ids" not in json.dumps(manifest)
    with pytest.raises(reason_01_preflight.PreflightError, match="expected 3"):
        reason_01_preflight.select_heldout_cases(rows, ["prior"], expected_count=3)


def test_preflight_skips_source_empty_messages_without_renumbering_identity():
    case = {
        "question_id": "heldout-1",
        "haystack_session_ids": ["session-1"],
        "haystack_sessions": [[
            {"role": "user", "content": "kept"},
            {"role": "assistant", "content": ""},
            {"role": "assistant", "content": "also kept"},
        ]],
    }

    rows = reason_01_preflight._case_rows(case)

    assert [row["logical_id"] for row in rows] == [
        "longmemeval-heldout-1-session-1-0",
        "longmemeval-heldout-1-session-1-2",
    ]
