"""Measurement-correctness tests for the MCP agent-loop LoCoMo harness."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from experiments import mcp_agent_loop as mal


# --- cost tally --------------------------------------------------------------


def test_cost_bills_uncached_cached_and_output_tokens_at_table_prices() -> None:
    usage = mal.Usage(input_tokens=1000, cached_input_tokens=200, output_tokens=100, reasoning_tokens=0)
    # 800 uncached * 0.15 + 200 cached * 0.075 + 100 out * 0.60, per million.
    assert mal.cost_usd("gpt-4o-mini", usage) == pytest.approx(195e-6)


def test_reasoning_tokens_bill_as_output_once() -> None:
    # The API counts reasoning tokens inside completion tokens; they must not be added twice.
    usage = mal.Usage(input_tokens=1000, cached_input_tokens=0, output_tokens=300, reasoning_tokens=200)
    assert mal.cost_usd("gpt-5-mini", usage) == pytest.approx((1000 * 0.25 + 300 * 2.00) / 1e6)


def test_unknown_model_has_no_price() -> None:
    with pytest.raises(KeyError):
        mal.cost_usd("not-a-model", mal.Usage(1, 0, 1, 0))


def test_ledger_total_survives_a_restart(tmp_path: Path) -> None:
    ledger = mal.CostLedger(tmp_path, cap_usd=5.0)
    ledger.record(arm="A1", qid="conv0_q0", role="answerer", model="gpt-4o-mini",
                  usage=mal.Usage(1000, 0, 100, 0), route_cost_usd=None)
    ledger.record(arm="B-4o", qid="conv0_q0", role="agent", model="gpt-4o-mini",
                  usage=mal.Usage(2000, 0, 0, 0), route_cost_usd=None)
    expected = (1000 * 0.15 + 100 * 0.60 + 2000 * 0.15) / 1e6
    assert ledger.total_usd() == pytest.approx(expected)
    assert mal.CostLedger(tmp_path, cap_usd=5.0).total_usd() == pytest.approx(expected)
    assert mal.CostLedger(tmp_path, cap_usd=5.0).arm_totals()["A1"] == pytest.approx((150 + 60) / 1e6)


def test_ledger_charges_the_higher_of_table_and_route_cost(tmp_path: Path) -> None:
    ledger = mal.CostLedger(tmp_path, cap_usd=5.0)
    ledger.record(arm="A1", qid="q", role="judge", model="gpt-4o-mini",
                  usage=mal.Usage(1000, 0, 0, 0), route_cost_usd=0.01)
    assert ledger.total_usd() == pytest.approx(0.01)


def test_ledger_refuses_calls_once_the_cap_is_reached(tmp_path: Path) -> None:
    ledger = mal.CostLedger(tmp_path, cap_usd=0.001)
    ledger.check()
    ledger.record(arm="A1", qid="q", role="judge", model="gpt-4o-mini",
                  usage=mal.Usage(10_000, 0, 0, 0), route_cost_usd=None)  # $0.0015
    with pytest.raises(mal.BudgetExceeded):
        ledger.check()
    with pytest.raises(mal.BudgetExceeded):
        mal.CostLedger(tmp_path, cap_usd=0.001).check()


# --- checkpoint / resume -----------------------------------------------------


def _questions(n: int) -> list[mal.Question]:
    return [
        mal.Question(qid=f"conv0_q{i}", category=1 + i % 4, question=f"q{i}", answer="a",
                     evidence=["D1:1"], gold_refs=["l:0:D1:1"])
        for i in range(n)
    ]


def test_each_question_is_checkpointed_as_it_completes(tmp_path: Path) -> None:
    seen: list[str] = []

    def process(question: mal.Question) -> dict:
        if question.qid == "conv0_q3":
            raise RuntimeError("process died")
        seen.append(question.qid)
        return {"score": 1.0}

    checkpoint = mal.ArmCheckpoint(tmp_path / "A1")
    with pytest.raises(RuntimeError):
        mal.run_arm(_questions(6), process, checkpoint, resume=False)
    assert seen == ["conv0_q0", "conv0_q1", "conv0_q2"]
    assert checkpoint.done_ids() == {"conv0_q0", "conv0_q1", "conv0_q2"}


def test_resume_processes_only_missing_questions(tmp_path: Path) -> None:
    checkpoint = mal.ArmCheckpoint(tmp_path / "A1")
    mal.run_arm(_questions(6)[:2], lambda q: {"score": 0.0}, checkpoint, resume=False)
    seen: list[str] = []

    def process(question: mal.Question) -> dict:
        seen.append(question.qid)
        return {"score": 1.0}

    processed = mal.run_arm(_questions(6), process, checkpoint, resume=True)
    assert processed == 4
    assert seen == ["conv0_q2", "conv0_q3", "conv0_q4", "conv0_q5"]
    results = checkpoint.load_all()
    assert set(results) == {f"conv0_q{i}" for i in range(6)}
    assert results["conv0_q0"]["score"] == 0.0  # earlier results are kept, not redone
    assert results["conv0_q5"]["qid"] == "conv0_q5"


def test_a_fresh_run_refuses_to_overwrite_checkpoints(tmp_path: Path) -> None:
    checkpoint = mal.ArmCheckpoint(tmp_path / "A1")
    mal.run_arm(_questions(1), lambda q: {"score": 1.0}, checkpoint, resume=False)
    with pytest.raises(ValueError):
        mal.run_arm(_questions(2), lambda q: {"score": 1.0}, checkpoint, resume=False)


def test_an_incomplete_checkpoint_file_is_redone(tmp_path: Path) -> None:
    checkpoint = mal.ArmCheckpoint(tmp_path / "A1")
    checkpoint.directory.mkdir(parents=True)
    (checkpoint.directory / "conv0_q0.json").write_text(json.dumps({"qid": "conv0_q0"}), encoding="utf-8")
    (checkpoint.directory / "conv0_q1.json").write_text("{not json", encoding="utf-8")
    assert checkpoint.done_ids() == set()
    seen: list[str] = []
    mal.run_arm(_questions(2), lambda q: seen.append(q.qid) or {"score": 1.0}, checkpoint, resume=True)
    assert seen == ["conv0_q0", "conv0_q1"]


def test_run_arm_checks_the_budget_before_each_question(tmp_path: Path) -> None:
    ledger = mal.CostLedger(tmp_path / "ledger", cap_usd=0.0001)
    seen: list[str] = []

    def process(question: mal.Question) -> dict:
        seen.append(question.qid)
        ledger.record(arm="A1", qid=question.qid, role="judge", model="gpt-4o-mini",
                      usage=mal.Usage(1000, 0, 0, 0), route_cost_usd=None)  # $0.00015 each
        return {"score": 1.0}

    checkpoint = mal.ArmCheckpoint(tmp_path / "A1")
    with pytest.raises(mal.BudgetExceeded):
        mal.run_arm(_questions(3), process, checkpoint, resume=False, ledger=ledger)
    assert seen == ["conv0_q0"]
    assert checkpoint.done_ids() == {"conv0_q0"}
