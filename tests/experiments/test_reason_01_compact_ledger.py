"""REASON-01 compact-ledger offshoot contracts."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from experiments import reason_01_compact_ledger as compact


HITS = [
    {"logical_id": "m1", "source_id": "s1", "body": "Alpha happened once in May."},
    {"logical_id": "m2", "source_id": "s2", "body": "Beta happened twice in June."},
]


def test_arm_order_rotates_all_three_arms() -> None:
    assert compact.arm_order(0) == compact.ARMS
    assert compact.arm_order(1) == compact.ARMS[1:] + compact.ARMS[:1]
    assert compact.arm_order(2) == compact.ARMS[2:] + compact.ARMS[:2]


def test_ledger_enriches_only_unique_exact_quotes() -> None:
    result = compact.parse_ledger(
        json.dumps(
            {
                "requirements": ["Find Alpha"],
                "evidence": [{"id": "m1", "quote": "Alpha happened once", "requirements": [0]}],
                "missing": [],
                "conflicts": [],
            }
        ),
        HITS,
    )
    strip = result["evidence"][0]
    assert strip["start"] == 0
    assert strip["end"] == len("Alpha happened once")
    assert strip["source_id"] == "s1"
    assert len(strip["body_sha256"]) == 64

    duplicated = [{"logical_id": "m1", "source_id": "s1", "body": "same quote; same quote"}]
    with pytest.raises(compact.CompactLedgerError, match="unique exact quote"):
        compact.parse_ledger(
            '{"requirements":["r"],"evidence":[{"id":"m1","quote":"same quote",'
            '"requirements":[0]}],"missing":[],"conflicts":[]}',
            duplicated,
        )

    oversized = [{"logical_id": "m1", "source_id": "s1", "body": "x" * 601}]
    with pytest.raises(compact.CompactLedgerError, match=r"m1.*601.*8-600"):
        compact.parse_ledger(
            '{"requirements":["r"],"evidence":[{"id":"m1","quote":"'
            + "x" * 601
            + '","requirements":[0]}],"missing":[],"conflicts":[]}',
            oversized,
        )

    with pytest.raises(compact.CompactLedgerError, match=r"m1.*more than once.*do not split"):
        compact.parse_ledger(
            '{"requirements":["r"],"evidence":['
            '{"id":"m1","quote":"Alpha happened once","requirements":[0]},'
            '{"id":"m1","quote":"happened once in May","requirements":[0]}],'
            '"missing":[],"conflicts":[]}',
            HITS,
        )


def test_quote_mark_only_drift_canonicalizes_to_exact_source() -> None:
    hits = [
        {
            "logical_id": "m1",
            "source_id": "s1",
            "body": 'The user watched "Joker" at AFI Fest.',
        }
    ]
    result = compact.parse_ledger(
        '{"requirements":["film"],"evidence":[{"id":"m1",'
        '"quote":"The user watched \'Joker\' at AFI Fest.","requirements":[0]}],'
        '"missing":[],"conflicts":[]}',
        hits,
    )
    assert result["evidence"][0]["quote"] == 'The user watched "Joker" at AFI Fest.'
    assert result["evidence"][0]["match_mode"] == "quote_mark_equivalent"

    curly = compact.parse_ledger(
        '{"requirements":["film"],"evidence":[{"id":"m1",'
        '"quote":"The user watched ‘Joker’ at AFI Fest.","requirements":[0]}],'
        '"missing":[],"conflicts":[]}',
        hits,
    )
    assert curly["evidence"][0]["quote"] == 'The user watched "Joker" at AFI Fest.'

    exact_first_hits = [
        {
            "logical_id": "m1",
            "source_id": "s1",
            "body": 'Saw "Joker". Saw \'Joker\'.',
        }
    ]
    exact = compact.parse_ledger(
        '{"requirements":["film"],"evidence":[{"id":"m1",'
        '"quote":"Saw \'Joker\'.","requirements":[0]}],"missing":[],"conflicts":[]}',
        exact_first_hits,
    )
    assert exact["evidence"][0]["match_mode"] == "exact"

    ambiguous_hits = [
        {"logical_id": "m1", "source_id": "s1", "body": 'Saw "Joker". Saw ‘Joker’.'}
    ]
    with pytest.raises(compact.CompactLedgerError, match="unique exact quote"):
        compact.parse_ledger(
            '{"requirements":["film"],"evidence":[{"id":"m1",'
            '"quote":"Saw \'Joker\'.","requirements":[0]}],"missing":[],"conflicts":[]}',
            ambiguous_hits,
        )

    for changed in (
        "The user enjoyed 'Joker' at AFI Fest.",
        "The  user watched 'Joker' at AFI Fest.",
        "The user watched 'Joker'—at AFI Fest.",
        "The user watched 'Joker', at AFI Fest.",
        "The user watched 'Joker' at AFI Fest!",
    ):
        with pytest.raises(compact.CompactLedgerError, match="unique exact quote"):
            compact.parse_ledger(
                json.dumps(
                    {
                        "requirements": ["film"],
                        "evidence": [{"id": "m1", "quote": changed, "requirements": [0]}],
                        "missing": [],
                        "conflicts": [],
                    }
                ),
                hits,
            )

    with pytest.raises(compact.CompactLedgerError, match="unique exact quote"):
        compact.parse_ledger(
            '{"requirements":["film"],"evidence":[{"id":"m1",'
            '"quote":"The user enjoyed \'Joker\' at AFI Fest.","requirements":[0]}],'
            '"missing":[],"conflicts":[]}',
            hits,
        )


def test_compact_context_cannot_exceed_a0_or_absolute_budget() -> None:
    ledger = compact.parse_ledger(
        '{"requirements":["Find Alpha"],"evidence":[{"id":"m1",'
        '"quote":"Alpha happened once","requirements":[0]}],"missing":[],"conflicts":[]}',
        HITS,
    )
    messages = compact.compact_answer_messages(
        {"question": "When?"}, ledger, a0_user_chars=1000, absolute_max_chars=12000
    )
    assert len(messages[-1]["content"]) <= 1000
    with pytest.raises(compact.CompactLedgerError, match="budget"):
        compact.compact_answer_messages(
            {"question": "When?"}, ledger, a0_user_chars=20, absolute_max_chars=12000
        )


def test_evidence_judge_sees_raw_body_but_only_compact_strip() -> None:
    answer = {"answer": "Alpha happened.", "citations": ["m1"]}
    raw = compact.evidence_messages(answer, HITS, evidence_kind="raw")
    ledger = compact.parse_ledger(
        '{"requirements":["Find Alpha"],"evidence":[{"id":"m1",'
        '"quote":"Alpha happened once","requirements":[0]}],"missing":[],"conflicts":[]}',
        HITS,
    )
    stripped = compact.evidence_messages(answer, ledger["evidence"], evidence_kind="strip")
    assert "in May" in raw[-1]["content"]
    assert "in May" not in stripped[-1]["content"]
    assert "Alpha happened once" in stripped[-1]["content"]


def test_checkpoint_cache_deduplicates_identical_blinded_inputs(tmp_path: Path) -> None:
    checkpoint = compact.Checkpoint.open(
        tmp_path / "checkpoint.json", binding_sha256="a" * 64, question_ids=["q1"]
    )
    first = checkpoint.cache_cell("correctness", "hash", "a0_raw||q1")
    second = checkpoint.cache_cell("correctness", "hash", "protected_raw||q1")
    assert first is second
    assert first["consumers"] == ["a0_raw||q1", "protected_raw||q1"]


def test_cost_reservation_fails_closed() -> None:
    with pytest.raises(compact.CompactLedgerError, match="cap"):
        compact.reserve_cost(spent=9.9, reserved=0.2, cap=10.0)


def test_evidence_result_requires_per_claim_support() -> None:
    result = compact.parse_evidence_result(
        '{"claims":[{"text":"Alpha was in May","entailed":true,'
        '"supporting_citations":["m1"]}],"grounded":true,"attributed":true}',
        {"m1"},
    )
    assert result["attributed"] is True
    with pytest.raises(compact.CompactLedgerError, match="support"):
        compact.parse_evidence_result(
            '{"claims":[{"text":"Alpha was in May","entailed":true,'
            '"supporting_citations":[]}],"grounded":true,"attributed":true}',
            {"m1"},
        )


def test_uncited_answer_is_a_terminal_quality_failure_not_a_retry() -> None:
    result = compact.parse_answer(
        '{"answer":"There is no information about visits.","citations":[]}',
        {"m1"},
        allow_empty=True,
    )
    assert result == {
        "answer": "There is no information about visits.",
        "citations": [],
        "citation_contract_valid": False,
    }


def test_semantic_retry_receives_prior_output_and_validation_error(tmp_path: Path) -> None:
    class Client:
        def __init__(self) -> None:
            self.messages: list[list[dict[str, str]]] = []
            self.seeds: list[int] = []

        def complete(self, model: str, messages: object, **kwargs: object) -> compact.Reply:
            del model
            self.seeds.append(int(kwargs["seed"]))
            copied = [dict(row) for row in messages]  # type: ignore[arg-type]
            self.messages.append(copied)
            content = "invalid" if len(self.messages) == 1 else '{"ok":true}'
            return compact.Reply(content, 1, 1)

    checkpoint = compact.Checkpoint.open(
        tmp_path / "checkpoint.json", binding_sha256="a" * 64, question_ids=["q1"]
    )
    cell = checkpoint.direct_cell("answers", "a0_raw||q1")
    client = Client()
    result = compact._ensure_result(
        checkpoint,
        cell,
        client=client,  # type: ignore[arg-type]
        model={
            "model": "reader",
            "input_per_million": 0,
            "output_per_million": 0,
        },
        messages=[{"role": "user", "content": "Return JSON"}],
        max_tokens=10,
        response_format=compact.ANSWER_FORMAT,
        parser=lambda content: compact._json_object(content, "test"),
        semantic_attempts=2,
        semantic_retry_seeds=[100, 101],
        cap=1,
    )
    assert result == {"ok": True}
    assert client.messages[1][-2] == {"role": "assistant", "content": "invalid"}
    assert "failed deterministic validation" in client.messages[1][-1]["content"]
    assert client.seeds == [100, 101]
