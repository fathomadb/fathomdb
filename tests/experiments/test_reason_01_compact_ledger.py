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


def v2_config() -> dict[str, object]:
    return {
        "schema_version": "reason01.compact-ledger-run.v2",
        "arms": ["a0_raw", "protected_raw", "protected_evidence_ledger_v2"],
        "protocol": {"all_missing_abstention": compact.V2_ABSTENTION},
        "models": {
            "ledger": {
                "model": "ledger",
                "route": "ledger",
                "max_tokens": 10,
                "input_per_million": 0,
                "output_per_million": 0,
            },
            "reader": {
                "model": "reader",
                "route": "reader",
                "answer_max_tokens": 10,
                "input_per_million": 0,
                "output_per_million": 0,
            },
            "correctness": {
                "model": "correctness",
                "route": "correctness",
                "max_tokens": 10,
                "input_per_million": 0,
                "output_per_million": 0,
            },
            "evidence": {
                "model": "evidence",
                "route": "evidence",
                "max_tokens": 10,
                "input_per_million": 0,
                "output_per_million": 0,
            },
        },
        "max_usd": 1,
        "semantic_attempts": 2,
        "semantic_retry_seeds": [1, 2],
        "ledger_input_max_chars": 10000,
        "compact_answer_max_chars": 10000,
        "bootstrap_draws": 100,
        "bootstrap_seed": 1,
    }


def test_arm_order_rotates_all_three_arms() -> None:
    assert compact.arm_order(0) == compact.ARMS
    assert compact.arm_order(1) == compact.ARMS[1:] + compact.ARMS[:1]
    assert compact.arm_order(2) == compact.ARMS[2:] + compact.ARMS[:2]


def test_ledger_enriches_only_unique_exact_quotes() -> None:
    result = compact.parse_ledger(
        json.dumps(
            {
                "requirements": ["Find Alpha"],
                "evidence": [
                    {"id": "m1", "quote": "Alpha happened once", "requirements": [0]}
                ],
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

    duplicated = [
        {"logical_id": "m1", "source_id": "s1", "body": "same quote; same quote"}
    ]
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

    with pytest.raises(
        compact.CompactLedgerError, match=r"m1.*more than once.*do not split"
    ):
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
            "body": "Saw \"Joker\". Saw 'Joker'.",
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
                        "evidence": [
                            {"id": "m1", "quote": changed, "requirements": [0]}
                        ],
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
    stripped = compact.evidence_messages(
        answer, ledger["evidence"], evidence_kind="strip"
    )
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


def test_semantic_retry_receives_prior_output_and_validation_error(
    tmp_path: Path,
) -> None:
    class Client:
        def __init__(self) -> None:
            self.messages: list[list[dict[str, str]]] = []
            self.seeds: list[int] = []

        def complete(
            self, model: str, messages: object, **kwargs: object
        ) -> compact.Reply:
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


def test_v2_answer_schema_restricts_citations_to_exact_cell_ids() -> None:
    schema = compact.answer_format({"m2", "m1"})
    citations = schema["json_schema"]["schema"]["properties"]["citations"]
    assert citations["items"]["enum"] == ["m1", "m2"]
    assert citations["uniqueItems"] is True
    assert citations["maxItems"] == 2

    empty = compact.answer_format(set())
    empty_citations = empty["json_schema"]["schema"]["properties"]["citations"]
    assert "enum" not in empty_citations["items"]
    assert empty_citations["maxItems"] == 0


def test_v2_all_missing_emits_deterministic_attributed_abstention() -> None:
    ledger = {
        "requirements": ["Find a December museum visit"],
        "evidence": [],
        "missing": [0],
        "conflicts": [],
    }
    assert compact.all_requirements_missing(ledger) is True
    answer, evidence = compact.deterministic_abstention(
        ledger,
        "The available memories do not contain enough information to answer this question.",
    )
    assert answer == {
        "answer": "The available memories do not contain enough information to answer this question.",
        "citations": [],
        "citation_contract_valid": True,
        "abstention": True,
        "deterministic": True,
    }
    assert evidence["grounded"] is True
    assert evidence["attributed"] is True
    assert evidence["abstention_basis"] == {
        "kind": "all_requirements_missing_ledger_state",
        "requirement_indexes": [0],
    }

    with pytest.raises(compact.CompactLedgerError, match="coverage"):
        compact.validate_v2_ledger(
            {
                "requirements": ["Find support"],
                "evidence": [],
                "missing": [],
                "conflicts": [],
            }
        )


def test_v2_run_skips_reader_and_judge_for_all_missing_abstention(
    tmp_path: Path,
) -> None:
    case = {
        "question_id": "q_abs",
        "question": "What happened in December?",
        "answer": "The information is absent.",
    }
    hit = {
        "logical_id": "m1",
        "source_id": "s1",
        "body": "No relevant event is recorded." + " Context." * 80,
    }
    frozen = {
        "retrievals": {
            "a0||q_abs": {"hits": [hit]},
            "protected_multiquery_v1||q_abs": {"hits": [hit]},
        }
    }

    class Client:
        reader_calls = 0
        evidence_calls = 0

        def models(self) -> set[str]:
            return {"ledger", "reader", "correctness", "evidence"}

        def complete(
            self,
            model: str,
            messages: list[dict[str, str]],
            **kwargs: object,
        ) -> compact.Reply:
            del messages, kwargs
            if model == "ledger":
                content = json.dumps(
                    {
                        "requirements": ["Find the event"],
                        "evidence": [],
                        "missing": [0],
                        "conflicts": [],
                    }
                )
            elif model == "reader":
                self.reader_calls += 1
                content = json.dumps(
                    {"answer": "The information is absent.", "citations": ["m1"]}
                )
            elif model == "correctness":
                content = "yes"
            else:
                self.evidence_calls += 1
                content = json.dumps(
                    {
                        "claims": [
                            {
                                "text": "The information is absent.",
                                "entailed": True,
                                "supporting_citations": ["m1"],
                            }
                        ],
                        "grounded": True,
                        "attributed": True,
                    }
                )
            return compact.Reply(content, 1, 1)

    client = Client()
    checkpoint = compact.Checkpoint.open(
        tmp_path / "checkpoint.json",
        binding_sha256="c" * 64,
        question_ids=["q_abs"],
        schema_version=compact.SCHEMA_V2,
    )
    compact.run(
        checkpoint,
        [case],
        frozen,
        config=v2_config(),
        client=client,  # type: ignore[arg-type]
    )

    answer = checkpoint.state.answers["protected_evidence_ledger_v2||q_abs"]["result"]
    evidence = checkpoint.state.evidence["protected_evidence_ledger_v2||q_abs"]
    assert answer["answer"] == compact.V2_ABSTENTION
    assert answer["abstention"] is True
    assert evidence["grounded"] is True
    assert evidence["attributed"] is True
    assert client.reader_calls == 2
    assert client.evidence_calls == 1


def test_v2_semantic_exhaustion_persists_terminal_quality_failure(
    tmp_path: Path,
) -> None:
    class Client:
        def complete(
            self, model: str, messages: object, **kwargs: object
        ) -> compact.Reply:
            del model, messages, kwargs
            return compact.Reply(
                '{"answer":"unsupported","citations":["unknown"]}', 1, 1
            )

    checkpoint = compact.Checkpoint.open(
        tmp_path / "checkpoint.json",
        binding_sha256="a" * 64,
        question_ids=["q1"],
        schema_version=compact.SCHEMA_V2,
    )
    cell = checkpoint.direct_cell("answers", "protected_evidence_ledger_v2||q1")
    result = compact._ensure_result(
        checkpoint,
        cell,
        client=Client(),  # type: ignore[arg-type]
        model={"model": "reader", "input_per_million": 0, "output_per_million": 0},
        messages=[{"role": "user", "content": "Return JSON"}],
        max_tokens=10,
        response_format=compact.answer_format(set()),
        parser=lambda content: compact.parse_answer(content, set(), allow_empty=True),
        semantic_attempts=2,
        semantic_retry_seeds=[1, 2],
        cap=1,
        on_exhausted=lambda error, attempts: compact.terminal_answer_failure(
            error, attempts
        ),
    )
    assert result["terminal_quality_failure"] is True
    assert result["failure_reason"] == "answer citation contract drifted"
    assert result["semantic_attempts"] == 2
    assert cell["result"] == result


def test_v2_run_continues_after_invalid_citation_exhaustion(tmp_path: Path) -> None:
    cases = [
        {"question_id": "q1", "question": "Q1?", "answer": "A1"},
        {"question_id": "q2", "question": "Q2?", "answer": "A2"},
    ]

    def hit(identity: str) -> dict[str, str]:
        return {
            "logical_id": identity,
            "source_id": f"source-{identity}",
            "body": f"Evidence {identity} supports the answer." + " Context." * 80,
        }

    frozen = {
        "retrievals": {
            "a0||q1": {"hits": [hit("m1")]},
            "protected_multiquery_v1||q1": {"hits": [hit("m1")]},
            "a0||q2": {"hits": [hit("m2")]},
            "protected_multiquery_v1||q2": {"hits": [hit("m2")]},
        }
    }

    class Client:
        def models(self) -> set[str]:
            return {"ledger", "reader", "correctness", "evidence"}

        def complete(
            self,
            model: str,
            messages: list[dict[str, str]],
            **kwargs: object,
        ) -> compact.Reply:
            del kwargs
            combined = "\n".join(row["content"] for row in messages)
            identity = "m1" if "Q1?" in combined else "m2"
            if model == "ledger":
                content = json.dumps(
                    {
                        "requirements": ["Find support"],
                        "evidence": [
                            {
                                "id": identity,
                                "quote": f"Evidence {identity} supports the answer.",
                                "requirements": [0],
                            }
                        ],
                        "missing": [],
                        "conflicts": [],
                    }
                )
            elif (
                model == "reader"
                and "exact evidence strips" in messages[0]["content"].lower()
                and "Q1?" in combined
            ):
                content = '{"answer":"unsupported","citations":["unknown"]}'
            elif model == "reader":
                content = json.dumps(
                    {"answer": f"Answer {identity}", "citations": [identity]}
                )
            elif model == "correctness":
                content = "yes"
            else:
                content = json.dumps(
                    {
                        "claims": [
                            {
                                "text": f"Answer {identity}",
                                "entailed": True,
                                "supporting_citations": [identity],
                            }
                        ],
                        "grounded": True,
                        "attributed": True,
                    }
                )
            return compact.Reply(content, 1, 1)

    config = v2_config()
    checkpoint = compact.Checkpoint.open(
        tmp_path / "checkpoint.json",
        binding_sha256="b" * 64,
        question_ids=["q1", "q2"],
        schema_version=compact.SCHEMA_V2,
    )
    compact.run(checkpoint, cases, frozen, config=config, client=Client())  # type: ignore[arg-type]

    failed = checkpoint.state.answers["protected_evidence_ledger_v2||q1"]["result"]
    assert failed["terminal_quality_failure"] is True
    assert (
        checkpoint.state.correctness["protected_evidence_ledger_v2||q1"][
            "answer_correct"
        ]
        is False
    )
    assert (
        checkpoint.state.evidence["protected_evidence_ledger_v2||q1"]["grounded"]
        is False
    )
    assert "protected_evidence_ledger_v2||q2" in checkpoint.state.answers
    assert len(checkpoint.state.answers) == 6
    assert len(checkpoint.state.correctness) == 6
    assert len(checkpoint.state.evidence) == 6
    summary = compact.summarize(
        checkpoint,
        cases,
        frozen,
        {"q1": {"m1"}, "q2": {"m2"}},
        config=config,
    )
    assert summary["complete"] is True
    assert summary["terminal_quality_failures"]["protected_evidence_ledger_v2"] == {
        "ledger": 0,
        "answer": 1,
        "correctness": 1,
        "evidence": 1,
    }
