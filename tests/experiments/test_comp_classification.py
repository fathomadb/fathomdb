"""Every final-head COMP receipt carries a valid measurement classification."""

from __future__ import annotations

import json

import pytest

from experiments import comp03_classification as cc

FINAL_HEAD = [
    "comp-01-comp01-3633-20261016T1755Z-b9aa1214",
    "comp-01-comp01-hybrid-3633-20261016T1755Z-c1693bec",
    "comp-02a-comp02a-10000-20261016T1756Z-09ac6bd7",
    "comp-02a-comp02a-17272-20261016T1756Z-eede35ef",
    "comp-02b-comp02b-10000-20261016T1757Z-d91933b9",
    "comp-02b-latency-20261016T1700Z-4f96fad0",
    "comp-03-retrieval-sweep-20261016T1725Z-17773486",
    "comp-03-hermes-answer-20261016T1609Z-1c856c71",
    "comp-03-hermes-answer-20261016T1609Z-22869ddc",
    "comp-03-hermes-answer-20261016T1610Z-33d78241",
    "comp-03-hermes-answer-20261016T1732Z-33d78241",
    "comp-03-hermes-answer-20261016T1732Z-50579949",
    "comp-03-hermes-answer-20261016T1747Z-4e56afa9",
]


@pytest.mark.parametrize("run_id", FINAL_HEAD)
def test_final_head_receipt_classification_validates(run_id):
    cc.validate([run_id])


@pytest.mark.parametrize("run_id", [
    "comp-03-retrieval-sweep-20261016T1406Z-17773486",
    "comp-03-hermes-answer-20261016T1422Z-c807ed90",
])
def test_rebuilding_a_classified_receipt_reproduces_its_committed_plan(run_id):
    plan, _, plan_path = cc.build(run_id)
    assert plan == json.loads(plan_path.read_text(encoding="utf-8"))
