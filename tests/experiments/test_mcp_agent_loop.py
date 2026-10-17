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


def test_gpt_5_4_mini_bills_at_its_table_prices() -> None:
    # 800 uncached * 0.75 + 200 cached * 0.075 + 300 out (incl. 200 reasoning) * 4.50, per million.
    usage = mal.Usage(input_tokens=1000, cached_input_tokens=200, output_tokens=300, reasoning_tokens=200)
    assert mal.cost_usd("gpt-5.4-mini", usage) == pytest.approx((800 * 0.75 + 200 * 0.075 + 300 * 4.50) / 1e6)


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


# --- backoff -----------------------------------------------------------------


class _HttpError(Exception):
    def __init__(self, status_code: int, headers: dict | None = None) -> None:
        super().__init__(f"http {status_code}")
        self.status_code = status_code
        self.response = type("R", (), {"headers": headers or {}})()


def test_backoff_retries_429_honouring_retry_after_then_succeeds() -> None:
    sleeps: list[float] = []
    attempts = iter([_HttpError(429, {"retry-after": "30"}), _HttpError(503), "ok"])

    def call() -> str:
        item = next(attempts)
        if isinstance(item, Exception):
            raise item
        return item

    assert mal.with_backoff(call, sleep=sleeps.append, base_seconds=2.0) == "ok"
    assert sleeps[0] == pytest.approx(30.0)
    assert len(sleeps) == 2 and sleeps[1] >= 2.0


def test_backoff_does_not_retry_client_errors() -> None:
    sleeps: list[float] = []

    def call() -> str:
        raise _HttpError(400)

    with pytest.raises(_HttpError):
        mal.with_backoff(call, sleep=sleeps.append)
    assert sleeps == []


def test_backoff_gives_up_after_the_attempt_limit() -> None:
    sleeps: list[float] = []
    calls: list[int] = []

    def call() -> str:
        calls.append(1)
        raise _HttpError(500)

    with pytest.raises(_HttpError):
        mal.with_backoff(call, sleep=sleeps.append, max_attempts=3)
    assert len(calls) == 3 and len(sleeps) == 2


# --- gold mapping and parsing ------------------------------------------------


def test_evidence_ids_are_split_repaired_and_filtered_like_the_rust_evaluator() -> None:
    assert mal.normalize_evidence("D1:3") == ["D1:3"]
    assert mal.normalize_evidence("D8:6; D9:17") == ["D8:6", "D9:17"]
    assert mal.normalize_evidence("D9:1 D4:4 D4:6") == ["D9:1", "D4:4", "D4:6"]
    assert mal.normalize_evidence("D:11:26") == ["D11:26"]
    assert mal.normalize_evidence("D30:05") == ["D30:5"]
    assert mal.normalize_evidence("D") == []


def test_refs_are_extracted_from_tool_text() -> None:
    text = '1. l:0:D1:17 kind="note"\nl:0:D12:3 body="x l:3:D1:1"'
    assert mal.refs_in_text(text) == {"l:0:D1:17", "l:0:D12:3", "l:3:D1:1"}


def test_judge_label_parsing_matches_the_official_harness() -> None:
    assert mal.judge_score({"label": "CORRECT", "reasoning": "r"}) == 1.0
    assert mal.judge_score({"label": "correct"}) == 1.0
    assert mal.judge_score({"label": "WRONG"}) == 0.0
    assert mal.judge_score({}) == 0.0
    assert mal.unwrap_judge_json('{"final": {"label": "CORRECT"}}') == {"label": "CORRECT"}
    assert mal.unwrap_judge_json('{"final": "{\\"label\\": \\"WRONG\\"}"}') == {"label": "WRONG"}


def test_answer_extraction_takes_text_after_the_last_answer_marker() -> None:
    assert mal.extract_answer("Step 1 ... ANSWER: x\nANSWER: Paris") == "Paris"
    assert mal.extract_answer("Paris") == "Paris"


def test_tools_become_functions_without_output_schema() -> None:
    tools = [{"name": "search", "description": "Search.", "inputSchema": {"type": "object"},
              "outputSchema": {"type": "object"}}]
    assert mal.tools_to_functions(tools) == [
        {"type": "function", "function": {"name": "search", "description": "Search.",
                                          "parameters": {"type": "object"}}}
    ]


# --- agent budget ------------------------------------------------------------


_FUNCTIONS = mal.tools_to_functions([
    {"name": name, "description": name, "inputSchema": {"type": "object"}}
    for name in ("search", "fetch", "orient")
])


class _Model:
    """Scripted model: each turn returns the next list of (name, args) calls, or a final answer."""

    def __init__(self, script: list) -> None:
        self.script = list(script)
        self.offered: list[tuple[list[str], str]] = []

    def __call__(self, messages: list[dict], functions: list[dict], tool_choice: str) -> mal.ModelTurn:
        self.offered.append(([f["function"]["name"] for f in functions], tool_choice))
        usage = mal.Usage(100, 0, 10, 0)
        if tool_choice == "none":
            return mal.ModelTurn(content="forced answer", tool_calls=[], usage=usage)
        step = self.script.pop(0) if self.script else "done"
        if isinstance(step, str):
            return mal.ModelTurn(content=step, tool_calls=[], usage=usage)
        calls = [mal.ToolCall(id=f"c{len(self.offered)}-{i}", name=n, arguments=json.dumps(a))
                 for i, (n, a) in enumerate(step)]
        return mal.ModelTurn(content=None, tool_calls=calls, usage=usage)


def _tool(name: str, args: dict) -> tuple[str, bool]:
    return f"{name} result l:0:D1:1", False


def test_agent_stops_offering_search_after_three_searches() -> None:
    model = _Model([[("search", {"query": "a"})], [("search", {"query": "b"})],
                    [("search", {"query": "c"})], [("orient", {})], "final"])
    trace = mal.run_agent_loop("q?", system_prompt="s", functions=_FUNCTIONS, model_turn=model, call_tool=_tool)
    executed = [c["name"] for c in trace.tool_log if c["executed"]]
    assert executed == ["search", "search", "search", "orient"]
    assert "search" not in model.offered[3][0]
    assert trace.forced_final is True  # the fourth call spent the budget
    assert trace.final_answer == "forced answer"
    assert model.offered[-1][1] == "none"


def test_agent_never_executes_more_than_four_calls_and_answers_every_call_id() -> None:
    model = _Model([[("fetch", {"refs": ["l:0:D1:1"]})] * 3, [("fetch", {}), ("search", {"query": "x"})]])
    seen_messages: list[list[dict]] = []

    def recording_model(messages: list[dict], functions: list[dict], tool_choice: str) -> mal.ModelTurn:
        seen_messages.append([dict(m) for m in messages])
        return model(messages, functions, tool_choice)

    trace = mal.run_agent_loop("q?", system_prompt="s", functions=_FUNCTIONS, model_turn=recording_model,
                               call_tool=_tool)
    assert sum(c["executed"] for c in trace.tool_log) == 4
    assert [c["executed"] for c in trace.tool_log] == [True, True, True, True, False]
    final_messages = seen_messages[-1]
    call_ids = [tc["id"] for m in final_messages if m["role"] == "assistant" for tc in m.get("tool_calls", [])]
    answered = [m["tool_call_id"] for m in final_messages if m["role"] == "tool"]
    assert call_ids == answered
    assert trace.forced_final is True


def test_agent_that_answers_directly_is_not_forced() -> None:
    model = _Model([[("search", {"query": "a"})], "the answer"])
    trace = mal.run_agent_loop("q?", system_prompt="s", functions=_FUNCTIONS, model_turn=model, call_tool=_tool)
    assert trace.final_answer == "the answer"
    assert trace.forced_final is False
    assert trace.refs_seen == {"l:0:D1:1"}
    assert len(trace.turn_usage) == 2


def test_tool_errors_are_passed_back_as_their_text() -> None:
    model = _Model([[("search", {"query": "a"})], "x"])

    def failing(name: str, args: dict) -> tuple[str, bool]:
        return "error invalid_request/bad_argument: nope", True

    trace = mal.run_agent_loop("q?", system_prompt="s", functions=_FUNCTIONS, model_turn=model, call_tool=failing)
    assert trace.tool_log[0]["is_error"] is True
    assert trace.messages[3]["content"] == "error invalid_request/bad_argument: nope"


def test_bootstrap_ci_is_deterministic_for_a_seed() -> None:
    values = [1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 1.0]
    first = mal.bootstrap_ci(values, seed=7, resamples=2000)
    assert first == mal.bootstrap_ci(values, seed=7, resamples=2000)
    assert first[0] <= sum(values) / len(values) <= first[1]
    assert mal.bootstrap_ci([1.0] * 5, seed=1, resamples=100) == (1.0, 1.0)


# --- prompt variants ---------------------------------------------------------


def test_guided_prompt_is_the_server_instructions_then_the_client_prompt() -> None:
    instructions = "FathomDB memory stores, read-only. Always search first."
    prompt = mal.agent_system_prompt("guided", instructions=instructions)
    assert prompt == instructions + "\n\n" + mal.GUIDED_CLIENT_PROMPT
    assert mal.GUIDED_CLIENT_PROMPT.startswith("You answer questions from a memory store")


def test_neutral_prompt_excludes_the_server_instructions() -> None:
    prompt = mal.agent_system_prompt("neutral", instructions="FathomDB memory stores, read-only.")
    assert prompt == mal.AGENT_SYSTEM_PROMPT
    assert "FathomDB" not in prompt


@pytest.mark.parametrize("instructions", [None, "", "   "])
def test_guided_prompt_refuses_missing_instructions(instructions: str | None) -> None:
    with pytest.raises(ValueError):
        mal.agent_system_prompt("guided", instructions=instructions)


def test_agent_arms_carry_their_prompt_variant_and_model() -> None:
    expected = {
        "B-4o": ("neutral", "gpt-4o-mini"),
        "B-5.4mini": ("neutral", "gpt-5.4-mini"),
        "B-4o-guided": ("guided", "gpt-4o-mini"),
        "B-5.4mini-guided": ("guided", "gpt-5.4-mini"),
    }
    for arm, (variant, model) in expected.items():
        spec = mal.ARM_SPECS[arm]
        assert (spec["kind"], spec["mode"], spec["prompt"], spec["model"]) == ("agent", "hybrid", variant, model)


# --- model routing -----------------------------------------------------------


@pytest.mark.parametrize("served", ["openai/gpt-5.4-mini", "openai/gpt-5.4-mini-2026-03-17"])
def test_served_model_guard_accepts_gpt_5_4_mini(served: str) -> None:
    mal.check_served_model("gpt-5.4-mini", served)


@pytest.mark.parametrize("served", [None, "openai/gpt-5-mini", "openai/gpt-4o-mini", "gpt-5.4-mini",
                                    "openrouter/openai/gpt-5.4-mini"])
def test_served_model_guard_rejects_other_models_for_gpt_5_4_mini(served: str | None) -> None:
    with pytest.raises(mal.ServedModelMismatch):
        mal.check_served_model("gpt-5.4-mini", served)


def test_gpt_5_4_mini_is_requested_by_its_provider_id() -> None:
    assert mal.request_model_id("gpt-5.4-mini") == "openai/gpt-5.4-mini"
    assert mal.request_model_id("gpt-4o-mini") == "gpt-4o-mini"


def test_gpt_5_4_mini_runs_at_low_reasoning_effort() -> None:
    params = mal.request_params("gpt-5.4-mini", max_tokens=4096)
    assert params == {"reasoning_effort": "low", "max_completion_tokens": 4096}
    assert mal.request_params("gpt-4o-mini", max_tokens=4096) == {"temperature": 0, "max_tokens": 4096}


def test_served_model_guard_still_rejects_the_gpt_5_mini_remap() -> None:
    with pytest.raises(mal.ServedModelMismatch):
        mal.check_served_model("gpt-5-mini", "openai/gpt-5.4-mini")
