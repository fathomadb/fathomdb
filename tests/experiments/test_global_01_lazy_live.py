"""Execution controls for the paid GLOBAL-01 lazy-coverage comparison."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

import pytest

from experiments import global_01_lazy_live


def test_retry_after_accepts_seconds_and_http_date():
    now = datetime(2026, 8, 29, 18, 0, tzinfo=timezone.utc)

    assert global_01_lazy_live.retry_delay(
        {"Retry-After": "17"}, fallback=2.0, now=now
    ) == 17.0
    assert global_01_lazy_live.retry_delay(
        {"Retry-After": "Sat, 29 Aug 2026 18:00:31 GMT"},
        fallback=2.0,
        now=now,
    ) == 31.0
    assert global_01_lazy_live.retry_delay({}, fallback=2.0, now=now) == 2.0


def test_checkpoint_round_trip_is_bound_and_resumes_only_missing(tmp_path: Path):
    path = tmp_path / "checkpoint.json"
    state = global_01_lazy_live.LazyRunState.new("a" * 64, 12.0)
    state.complete("aa/answer-1/0/ab", {"verdicts": {}}, cost_usd=0.25)
    state.save(path)

    restored = global_01_lazy_live.LazyRunState.load(path, "a" * 64, 12.0)

    assert restored.cost_usd == 0.25
    assert restored.missing(["aa/answer-1/0/ab", "aa/answer-1/0/ba"]) == [
        "aa/answer-1/0/ba"
    ]
    with pytest.raises(global_01_lazy_live.Global01LazyLiveError, match="drifted"):
        global_01_lazy_live.LazyRunState.load(path, "b" * 64, 12.0)


def test_client_refuses_worst_case_cost_before_network(monkeypatch):
    called = False

    def unexpected(*args, **kwargs):
        nonlocal called
        called = True
        raise AssertionError("network must not be called")

    monkeypatch.setattr(global_01_lazy_live.urllib.request, "urlopen", unexpected)
    client = global_01_lazy_live.AirlockClient(
        "http://127.0.0.1:4000",
        "secret",
        {
            "execution": {
                "retry_attempts": 1,
                "retry_backoff_seconds": [1],
            },
            "pricing": {
                "deepseek-v4-pro": {
                    "input_per_million": 1.32,
                    "output_per_million": 3.96,
                }
            },
        },
    )

    with pytest.raises(global_01_lazy_live.Global01LazyLiveError, match="cost cap"):
        client.complete(
            "deepseek-v4-pro",
            "x" * 10_000,
            max_tokens=1_000,
            temperature=0.0,
            remaining_cost_usd=0.000001,
        )

    assert called is False


def test_decomposition_requires_exact_bounded_unique_queries():
    value = global_01_lazy_live.parse_subqueries(
        '{"subqueries":["one area","second area","third area","fourth area"]}',
        count=4,
    )
    assert value == ["one area", "second area", "third area", "fourth area"]

    with pytest.raises(global_01_lazy_live.Global01LazyLiveError, match="subqueries"):
        global_01_lazy_live.parse_subqueries(
            '{"subqueries":["same","same","third","fourth"]}', count=4
        )


def test_map_prompt_bounds_claim_count_and_length():
    prompt = global_01_lazy_live._map_prompt(
        "Question?",
        [
            {
                "source_id": "source-1",
                "content_sha256": "a" * 64,
                "body": "Evidence.",
            }
        ],
        max_claims=2,
    )

    assert "at most 2 claims" in prompt
    assert "at most 30 words" in prompt
    assert "SOURCE_REF=S0" in prompt
    assert '"source_refs":["S0"]' in prompt


def test_mapped_claim_source_refs_restore_canonical_attribution():
    claims = global_01_lazy_live._validate_mapped_claims(
        {
            "claims": [
                {
                    "text": "A concise supported claim.",
                    "source_refs": ["S0"],
                }
            ]
        },
        known_source_refs={
            "S0": {
                "source_id": "source-1",
                "content_sha256": "a" * 64,
            }
        },
        prefix="control-q-00",
        max_claims=2,
        max_words=30,
    )

    assert claims == [
        {
            "claim_id": "control-q-00-000",
            "text": "A concise supported claim.",
            "sources": [
                {
                    "source_id": "source-1",
                    "content_sha256": "a" * 64,
                }
            ],
        }
    ]


def test_semantic_revision_preserves_legacy_invalid_cells(tmp_path: Path):
    class FakeClient:
        def __init__(self):
            self.responses = iter(
                [
                    ("{}", {"prompt_tokens": 1, "completion_tokens": 1}, 0.01, 1.0),
                    (
                        '{"ok":true}',
                        {"prompt_tokens": 1, "completion_tokens": 1},
                        0.01,
                        1.0,
                    ),
                ]
            )

        def complete(self, *args, **kwargs):
            return next(self.responses)

    checkpoint = tmp_path / "checkpoint.json"
    state = global_01_lazy_live.LazyRunState.new("d" * 64, 1.0)
    state.complete("invalid/maps/control/q/0/0", {"legacy": True}, cost_usd=0.01)

    value = global_01_lazy_live._complete_json_cell(
        FakeClient(),
        state=state,
        checkpoint_path=checkpoint,
        cell="maps/control/q/0",
        model="model",
        prompt="prompt",
        max_tokens=300,
        temperature=0.0,
        validator=lambda raw: raw
        if raw.get("ok") is True
        else (_ for _ in ()).throw(global_01_lazy_live.Global01LazyLiveError("bad")),
    )

    assert value == {"ok": True}
    assert state.cells["invalid/maps/control/q/0/0"] == {"legacy": True}
    assert f"invalid/{global_01_lazy_live.SEMANTIC_REVISION}/maps/control/q/0/0" in state.cells


def test_assertion_score_requires_all_final_claims_and_valid_indices():
    answer = {
        "claims": [
            {"claim_id": "final-1"},
            {"claim_id": "final-2"},
        ]
    }
    score = {
        "passed_assertion_indices": [0, 2],
        "claim_support": [
            {"claim_id": "final-1", "supported": True},
            {"claim_id": "final-2", "supported": False},
        ],
    }

    assert global_01_lazy_live.validate_assertion_score(
        score, answer=answer, assertion_count=3
    ) == score

    bad = json.loads(json.dumps(score))
    bad["claim_support"].pop()
    with pytest.raises(global_01_lazy_live.Global01LazyLiveError, match="claim"):
        global_01_lazy_live.validate_assertion_score(
            bad, answer=answer, assertion_count=3
        )


def test_scorer_excerpt_is_bounded_and_selects_claim_relevant_text():
    body = (
        "Unrelated opening material. " * 200
        + "The regional hospital expanded mental health crisis services. "
        + "Unrelated closing material. " * 200
    )

    excerpt = global_01_lazy_live.best_source_excerpt(
        body,
        "mental health crisis services expanded",
        max_chars=240,
    )

    assert len(excerpt) <= 240
    assert "mental health crisis services" in excerpt


def test_completeness_guard_blocks_partial_verdict():
    state = global_01_lazy_live.LazyRunState.new("c" * 64, 12.0)

    with pytest.raises(global_01_lazy_live.Global01LazyLiveError, match="incomplete"):
        global_01_lazy_live.assert_cells_complete(state, ["one", "two"])


def test_paid_runner_requires_matching_safe_preflight(tmp_path: Path):
    config = json.loads(
        Path(
            "experiments/configs/global-01/apnews-global-lazy-coverage.v1.json"
        ).read_text(encoding="utf-8")
    )
    report = tmp_path / "safe-preflight.json"
    report.write_text(
        json.dumps(
            {
                "schema_version": "global-01.lazy-preflight.v1",
                "state": "ready_for_hitl",
                "config_sha256": global_01_lazy_live._canonical_sha256(config),
                "cost_usd": 0.0,
                "lifecycle": {
                    "strict_current_supersession": "pass",
                    "temporal_failures": 0,
                    "erasure": "pass",
                    "derived_rows_written": 0,
                },
            }
        ),
        encoding="utf-8",
    )

    assert global_01_lazy_live.validate_safe_preflight(config, report)["state"] == (
        "ready_for_hitl"
    )
    value = json.loads(report.read_text(encoding="utf-8"))
    value["config_sha256"] = "0" * 64
    report.write_text(json.dumps(value), encoding="utf-8")
    with pytest.raises(global_01_lazy_live.Global01LazyLiveError, match="preflight"):
        global_01_lazy_live.validate_safe_preflight(config, report)


def test_acceptance_verdict_requires_every_registered_boundary():
    boundaries = {
        "headline_win_rates": True,
        "headline_ci_lower_bounds": True,
        "assertion_recall_delta": True,
        "directness": True,
        "unsupported_claim_delta": True,
        "source_link_completeness": True,
        "lifecycle": True,
        "token_cost_ratio": True,
        "end_to_end_p95_ratio": True,
    }

    assert global_01_lazy_live.acceptance_verdict(boundaries) == "accept"
    boundaries["directness"] = False
    assert global_01_lazy_live.acceptance_verdict(boundaries) == "reject"
