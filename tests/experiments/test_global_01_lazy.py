"""Human-intended controls for GLOBAL-01 lazy coverage."""

from __future__ import annotations

import copy
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path

import pytest
from hypothesis import given, strategies as st

from experiments import global_01_lazy


CONFIG_PATH = Path(
    "experiments/configs/global-01/apnews-global-lazy-coverage.v1.json"
)
V2_CONFIG_PATH = Path(
    "experiments/configs/global-01/apnews-global-lazy-coverage.v2.json"
)
V3_CONFIG_PATH = Path(
    "experiments/configs/global-01/apnews-global-lazy-coverage.v3.json"
)
V4_CONFIG_PATH = Path(
    "experiments/configs/global-01/apnews-global-lazy-coverage.v4.json"
)
V5_CONFIG_PATH = Path(
    "experiments/configs/global-01/apnews-global-lazy-coverage.v5.json"
)
V6_CONFIG_PATH = Path(
    "experiments/configs/global-01/apnews-global-lazy-coverage.v6.json"
)


def _config() -> dict[str, object]:
    return json.loads(CONFIG_PATH.read_text(encoding="utf-8"))


def _assertion(statement: str, score: int = 4) -> dict[str, object]:
    return {
        "statement": statement,
        "validation": {
            "is_valid": True,
            "scores": {
                "grounding": score,
                "relevance": score,
                "verifiability": score,
            },
        },
    }


def test_shipped_config_requires_explicit_profile_and_bound_paid_approval():
    config = global_01_lazy.validate_config(_config())

    assert config["profiles"]["treatment"]["name"] == "global_lazy_coverage_v1"
    assert config["profiles"]["treatment"]["caller_selected"] is True
    assert config["approval"]["cost_cap_usd"] == 12.0
    global_01_lazy.assert_execution_authorized(config)

    pending = copy.deepcopy(config)
    pending["approval"] = {
        "state": "pending_hitl",
        "approved_by": None,
        "approved_at": None,
        "cost_cap_usd": None,
    }
    with pytest.raises(global_01_lazy.Global01LazyError, match="HITL approval"):
        global_01_lazy.assert_execution_authorized(pending)

    automatic = copy.deepcopy(config)
    automatic["profiles"]["treatment"]["caller_selected"] = False
    with pytest.raises(global_01_lazy.Global01LazyError, match="caller-selected"):
        global_01_lazy.validate_config(automatic)


def test_v2_config_registers_recovery_adapter_and_bound_authorization():
    config = global_01_lazy.validate_config(
        json.loads(V2_CONFIG_PATH.read_text(encoding="utf-8"))
    )

    assert config["schema_version"] == "global-01.lazy-coverage.v2"
    assert config["execution"]["map_output_adapter"] == (
        "compact_refs_or_canonical_v2"
    )
    assert config["approval"] == {
        "state": "approved",
        "approved_by": "coreyt",
        "approved_at": "2026-08-29T20:43:12Z",
        "cost_cap_usd": 12.0,
    }
    global_01_lazy.assert_execution_authorized(config)


def test_v3_config_uses_documented_output_headroom_under_the_bound_cap():
    config = global_01_lazy.validate_config(
        json.loads(V3_CONFIG_PATH.read_text(encoding="utf-8"))
    )

    assert config["schema_version"] == "global-01.lazy-coverage.v3"
    assert config["profiles"]["control"]["reduce_max_tokens"] == 4096
    assert config["profiles"]["treatment"]["reduce_max_tokens"] == 4096
    assert config["pricing"]["projected_total_usd"] == 10.4
    assert config["approval"] == {
        "state": "approved",
        "approved_by": "coreyt",
        "approved_at": "2026-08-29T21:09:07Z",
        "cost_cap_usd": 12.0,
    }
    global_01_lazy.assert_execution_authorized(config)


def test_v4_config_compacts_reduction_identity_without_raising_the_ceiling():
    config = global_01_lazy.validate_config(
        json.loads(V4_CONFIG_PATH.read_text(encoding="utf-8"))
    )

    assert config["schema_version"] == "global-01.lazy-coverage.v4"
    assert config["profiles"]["control"]["reduce_max_tokens"] == 4096
    assert config["profiles"]["treatment"]["reduce_max_tokens"] == 4096
    assert config["execution"]["reduce_output_adapter"] == (
        "compact_refs_with_canonical_restore_v1"
    )
    assert config["approval"] == {
        "state": "approved",
        "approved_by": "coreyt",
        "approved_at": "2026-08-29T21:26:15Z",
        "cost_cap_usd": 12.0,
    }
    global_01_lazy.assert_execution_authorized(config)


def test_v5_config_binds_documented_assertion_scorer_headroom():
    config = global_01_lazy.validate_config(
        json.loads(V5_CONFIG_PATH.read_text(encoding="utf-8"))
    )

    assert config["schema_version"] == "global-01.lazy-coverage.v5"
    assert config["models"]["assertion_scorer"]["max_tokens"] == 2048
    assert config["execution"]["scorer_max_output_tokens"] == 65536
    assert config["pricing"]["projected_total_usd"] == 11.72
    assert config["approval"]["cost_cap_usd"] == 12.0
    global_01_lazy.assert_execution_authorized(config)


def test_v6_config_binds_fail_closed_scorer_projection():
    config = global_01_lazy.validate_config(
        json.loads(V6_CONFIG_PATH.read_text(encoding="utf-8"))
    )

    assert config["schema_version"] == "global-01.lazy-coverage.v6"
    assert config["execution"]["scorer_output_adapter"] == (
        "required_fields_projection_v1"
    )
    assert config["models"]["assertion_scorer"]["max_tokens"] == 2048
    assert config["pricing"]["projected_total_usd"] == 11.72
    global_01_lazy.assert_execution_authorized(config)


def test_question_split_is_deterministic_and_keeps_prior_questions_out_of_development():
    rows = [
        {
            "question_id": f"question-{index}",
            "question_text": f"Question {index}?",
            "assertions": [_assertion(f"Assertion {index}")],
            "claims": [],
        }
        for index in range(8)
    ]
    rows.append(
        {
            "question_id": "unqualified",
            "question_text": "Unqualified?",
            "assertions": [_assertion("Too weak", score=3)],
            "claims": [],
        }
    )
    prior_hash = hashlib.sha256("Question 0?".encode()).hexdigest()

    first = global_01_lazy.qualify_and_split_questions(
        rows,
        first_run_question_hashes={prior_hash},
        split_seed="fixed-seed",
        development_count=3,
        witness_count=2,
    )
    second = global_01_lazy.qualify_and_split_questions(
        rows,
        first_run_question_hashes={prior_hash},
        split_seed="fixed-seed",
        development_count=3,
        witness_count=2,
    )

    assert first == second
    assert len(first["qualified"]) == 8
    assert len(first["development"]) == 3
    assert len(first["heldout"]) == 5
    assert len(first["witness"]) == 2
    assert all(row["question_sha256"] != prior_hash for row in first["development"])


def test_retrieval_applies_one_read_view_and_retains_canonical_source_hashes():
    calls: list[tuple[str, object, int]] = []
    view = object()

    class Hit:
        def __init__(self, source_id: str, body: str):
            self.source_id = source_id
            self.body = body

    class Engine:
        def search(self, query: str, *, limit: int, view: object):
            calls.append((query, view, limit))
            return type("Result", (), {"results": [Hit("source-1", "alpha beta")]})()

    rows = global_01_lazy.retrieve_candidates(
        Engine(),
        ["alpha", "beta"],
        candidate_depth=50,
        source_hashes={"source-1": "a" * 64},
        read_view=view,
    )

    assert calls == [("alpha", view, 50), ("beta", view, 50)]
    assert rows == [
        {
            "source_id": "source-1",
            "content_sha256": "a" * 64,
            "body": "alpha beta",
            "ranks": {"0": 1, "1": 1},
        }
    ]


def test_selection_is_bounded_diverse_and_provenance_preserving():
    candidates = [
        {
            "source_id": f"source-{index}",
            "content_sha256": f"{index + 1:064x}",
            "body": body,
            "ranks": {str(index % 2): index + 1},
        }
        for index, body in enumerate(
            [
                "alpha beta shared",
                "alpha beta shared duplicate",
                "gamma delta distinct",
                "epsilon zeta distinct",
                "theta iota distinct",
            ]
        )
    ]
    selected = global_01_lazy.select_candidates(
        candidates,
        rrf_k=60,
        relevance_weight=0.65,
        novelty_weight=0.35,
        max_documents_per_group=2,
        max_documents_total=3,
        context_max_tokens=100,
    )

    assert len(selected) == 3
    assert max(row["group_ordinal"] for row in selected) <= 1
    assert all(row["source_id"] and len(row["content_sha256"]) == 64 for row in selected)
    assert len({row["source_id"] for row in selected}) == len(selected)
    assert len([row for row in selected if row["group_ordinal"] == 0]) <= 2
    assert len([row for row in selected if row["group_ordinal"] == 1]) <= 2


def test_structured_answer_rejects_missing_attribution_and_incomplete_ledger():
    answer = {
        "answer": "A concise answer.",
        "claims": [
            {
                "claim_id": "final-1",
                "text": "Supported fact.",
                "sources": [
                    {"source_id": "source-1", "content_sha256": "a" * 64}
                ],
            }
        ],
        "coverage_ledger": [
            {
                "mapped_claim_id": "mapped-1",
                "disposition": "included",
                "final_claim_ids": ["final-1"],
            }
        ],
    }

    assert global_01_lazy.validate_structured_answer(
        answer,
        known_sources={"source-1": "a" * 64},
        mapped_claim_ids={"mapped-1"},
    ) == answer

    missing_source = copy.deepcopy(answer)
    missing_source["claims"][0]["sources"] = []
    with pytest.raises(global_01_lazy.Global01LazyError, match="canonical source"):
        global_01_lazy.validate_structured_answer(
            missing_source,
            known_sources={"source-1": "a" * 64},
            mapped_claim_ids={"mapped-1"},
        )

    with pytest.raises(global_01_lazy.Global01LazyError, match="coverage ledger"):
        global_01_lazy.validate_structured_answer(
            answer,
            known_sources={"source-1": "a" * 64},
            mapped_claim_ids={"mapped-1", "mapped-2"},
        )


@given(st.lists(st.text(min_size=1), min_size=1, max_size=20, unique=True))
def test_coverage_ledger_round_trip_requires_every_mapped_claim_once(claim_ids):
    answer = {
        "answer": "Answer.",
        "claims": [
            {
                "claim_id": "final-1",
                "text": "Supported.",
                "sources": [
                    {"source_id": "source-1", "content_sha256": "a" * 64}
                ],
            }
        ],
        "coverage_ledger": [
            {
                "mapped_claim_id": claim_id,
                "disposition": "included" if index == 0 else "redundant",
                "final_claim_ids": ["final-1"] if index == 0 else [],
            }
            for index, claim_id in enumerate(claim_ids)
        ],
    }

    restored = json.loads(json.dumps(answer))
    global_01_lazy.validate_structured_answer(
        restored,
        known_sources={"source-1": "a" * 64},
        mapped_claim_ids=set(claim_ids),
    )


def test_checkpoint_plan_resumes_only_missing_cells():
    cells = global_01_lazy.required_cells(
        question_ids=["q1", "q2"],
        pairwise_repetitions=2,
        scorer_trials=2,
    )
    completed = {cells[0], cells[3], cells[-1]}

    missing = global_01_lazy.missing_cells(cells, completed)

    assert missing == [cell for cell in cells if cell not in completed]
    assert len(missing) == len(cells) - len(completed)


def test_safe_preflight_receipt_is_registered_without_private_payload(tmp_path: Path):
    pending = _config()
    pending["approval"] = {
        "state": "pending_hitl",
        "approved_by": None,
        "approved_at": None,
        "cost_cap_usd": None,
    }
    config = global_01_lazy.validate_config(pending)
    report_path = tmp_path / "safe-preflight.json"
    report_path.write_text(
        json.dumps(
            {
                "schema_version": "global-01.lazy-preflight.v1",
                "state": "ready_for_hitl",
                "program_track": "GLOBAL-01",
                "config_sha256": global_01_lazy.canonical_sha256(config),
                "cost_usd": 0.0,
                "corpus": {"article_count": 1397},
                "questions": {"qualified_count": 49},
                "runtime": {"fathomdb_python_version": "0.8.23"},
                "authentication": {"required_aliases": ["deepseek-v4-pro", "claude-haiku"]},
                "retrieval": {"minimum_hit_count": 50},
                "lifecycle": {"strict_current_supersession": "pass", "erasure": "pass"},
                "resilience": {"checkpoint_every_cell": True},
                "pricing": {"projected_total_usd": 9.5, "recommended_cap_usd": 12.0},
            }
        ),
        encoding="utf-8",
    )

    run_id, run_dir = global_01_lazy.register_preflight_receipt(
        config,
        report_path=report_path,
        config_path=CONFIG_PATH,
        base_dir=tmp_path / "experiments",
        ts=datetime(2026, 8, 29, 19, 0, tzinfo=timezone.utc),
    )

    record = json.loads((run_dir / "record.json").read_text(encoding="utf-8"))
    assert run_id.startswith("global-01-lazy-preflight-")
    assert record["verdict"] == "awaiting_hitl"
    assert record["metrics"]["state"] == "ready_for_hitl"
    assert "documents" not in record["metrics"]
    assert "private_questions" not in record["metrics"]

    approved = copy.deepcopy(config)
    approved["approval"] = {
        "state": "approved",
        "approved_by": "coreyt",
        "approved_at": "2026-08-29T19:20:57Z",
        "cost_cap_usd": 12.0,
    }
    report = json.loads(report_path.read_text(encoding="utf-8"))
    report["config_sha256"] = global_01_lazy.canonical_sha256(approved)
    report_path.write_text(json.dumps(report), encoding="utf-8")
    _, approved_dir = global_01_lazy.register_preflight_receipt(
        approved,
        report_path=report_path,
        config_path=CONFIG_PATH,
        base_dir=tmp_path / "approved-experiments",
        ts=datetime(2026, 8, 29, 19, 1, tzinfo=timezone.utc),
    )
    approved_record = json.loads(
        (approved_dir / "record.json").read_text(encoding="utf-8")
    )
    assert approved_record["verdict"] == "authorized_ready"
