"""Agent-in-the-loop LoCoMo answer evaluation of FathomDB through ``fathomdb-mcp``.

Arms (conversation 0, categories 1-4 with evidence):

- ``A1`` / ``A2``: one MCP ``search`` (``mode`` text / hybrid, limit 10), then
  ``fetch`` of the ten refs; the bodies become the answerer context exactly as
  the official LOCOMO harness builds it for the FathomDB arm of the Mem0
  comparison (gpt-4o-mini answerer).
- ``B-4o`` / ``B-5mini``: a function-calling agent loop over the server's own
  ``tools/list`` with a 4-call / 3-search budget.

Every arm is graded by the official evidence-aware judge prompt with
gpt-4o-mini. Raw predictions, transcripts and per-question checkpoints stay in
an external output directory; only content-free aggregates reach the repository.
"""

from __future__ import annotations

import json
import math
import os
import random
import re
import time
from collections.abc import Callable, Iterable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


EXPERIMENT = "mcp-agent-loop-locomo-conv0"
SCHEMA_VERSION = "mcp-agent-loop.v1"

AGENT_SYSTEM_PROMPT = (
    "The tools give you access to the memory of a conversation between two people. "
    "Use them to answer the user's question. Answer concisely. "
    "If the memory does not contain the answer, say that you do not know."
)
MAX_TOOL_CALLS = 4
MAX_SEARCH_CALLS = 3
MAX_MODEL_TURNS = 8

# USD per million tokens. Reasoning tokens are part of completion tokens and
# bill at the output price.
PRICES_PER_MTOK: dict[str, dict[str, float]] = {
    "gpt-4o-mini": {"input": 0.15, "cached_input": 0.075, "output": 0.60},
    "gpt-5-mini": {"input": 0.25, "cached_input": 0.025, "output": 2.00},
}


class BudgetExceeded(RuntimeError):
    """Raised when the cumulative spend for the task reaches the hard cap."""


@dataclass(frozen=True)
class Usage:
    """Token usage reported by the API for one call."""

    input_tokens: int
    cached_input_tokens: int
    output_tokens: int
    reasoning_tokens: int


def cost_usd(model: str, usage: Usage) -> float:
    """Price one call from its reported usage; raises ``KeyError`` for an unpriced model."""
    price = PRICES_PER_MTOK[model]
    uncached = usage.input_tokens - usage.cached_input_tokens
    return (
        uncached * price["input"]
        + usage.cached_input_tokens * price["cached_input"]
        + usage.output_tokens * price["output"]
    ) / 1_000_000


class CostLedger:
    """Append-only, restart-safe spend ledger shared by every arm of one task.

    Each call is one JSON line in ``<directory>/<arm>.jsonl``. The charge for a
    line is the higher of the table price and the route-reported cost, so the
    cap errs on the side of stopping early.
    """

    def __init__(self, directory: str | Path, *, cap_usd: float) -> None:
        self.directory = Path(directory)
        self.cap_usd = cap_usd

    @staticmethod
    def _charge(entry: dict[str, Any]) -> float:
        route = entry.get("route_cost_usd")
        table = float(entry["cost_usd"])
        return max(table, float(route)) if route is not None else table

    def _entries(self) -> Iterable[dict[str, Any]]:
        if not self.directory.is_dir():
            return
        for path in sorted(self.directory.glob("*.jsonl")):
            for line in path.read_text(encoding="utf-8").splitlines():
                if line.strip():
                    yield json.loads(line)

    def record(self, *, arm: str, qid: str, role: str, model: str, usage: Usage,
               route_cost_usd: float | None, served_model: str | None = None) -> float:
        """Append one priced call and return its charge."""
        entry = {
            "arm": arm, "qid": qid, "role": role, "model": model, "served_model": served_model,
            "input_tokens": usage.input_tokens, "cached_input_tokens": usage.cached_input_tokens,
            "output_tokens": usage.output_tokens, "reasoning_tokens": usage.reasoning_tokens,
            "cost_usd": cost_usd(model, usage), "route_cost_usd": route_cost_usd,
        }
        self.directory.mkdir(parents=True, exist_ok=True)
        with (self.directory / f"{arm}.jsonl").open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(entry, sort_keys=True) + "\n")
            handle.flush()
            os.fsync(handle.fileno())
        return self._charge(entry)

    def total_usd(self) -> float:
        """Cumulative charge across every arm, including calls of unfinished questions."""
        return sum(self._charge(entry) for entry in self._entries())

    def arm_totals(self) -> dict[str, float]:
        """Cumulative charge per arm."""
        totals: dict[str, float] = {}
        for entry in self._entries():
            totals[entry["arm"]] = totals.get(entry["arm"], 0.0) + self._charge(entry)
        return totals

    def check(self) -> None:
        """Raise ``BudgetExceeded`` once the cumulative charge reaches the cap."""
        total = self.total_usd()
        if total >= self.cap_usd:
            raise BudgetExceeded(f"cumulative spend ${total:.4f} reached the ${self.cap_usd:.2f} cap")


@dataclass(frozen=True)
class Question:
    """One scored LoCoMo QA item and its gold turn references."""

    qid: str
    category: int
    question: str
    answer: str
    evidence: list[str]
    gold_refs: list[str] = field(default_factory=list)


class ArmCheckpoint:
    """One atomic JSON file per completed question."""

    def __init__(self, directory: str | Path) -> None:
        self.directory = Path(directory)

    def _load(self, path: Path) -> dict[str, Any] | None:
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return None
        if not isinstance(data, dict) or data.get("complete") is not True or data.get("qid") != path.stem:
            return None
        return data

    def load_all(self) -> dict[str, dict[str, Any]]:
        """Return every complete checkpoint keyed by question id."""
        if not self.directory.is_dir():
            return {}
        results: dict[str, dict[str, Any]] = {}
        for path in sorted(self.directory.glob("*.json")):
            data = self._load(path)
            if data is not None:
                results[path.stem] = data
        return results

    def done_ids(self) -> set[str]:
        """Question ids with a complete checkpoint."""
        return set(self.load_all())

    def has_any(self) -> bool:
        """True when the directory holds any checkpoint file, complete or not."""
        return self.directory.is_dir() and any(self.directory.glob("*.json"))

    def save(self, qid: str, result: dict[str, Any]) -> None:
        """Write one completed question atomically."""
        self.directory.mkdir(parents=True, exist_ok=True)
        payload = {**result, "qid": qid, "complete": True}
        tmp = self.directory / f".{qid}.json.tmp"
        tmp.write_text(json.dumps(payload, indent=1, sort_keys=True) + "\n", encoding="utf-8")
        os.replace(tmp, self.directory / f"{qid}.json")


def run_arm(
    questions: list[Question],
    process: Callable[[Question], dict[str, Any]],
    checkpoint: ArmCheckpoint,
    *,
    resume: bool,
    ledger: CostLedger | None = None,
) -> int:
    """Process every question that has no complete checkpoint; return how many ran.

    A fresh run (``resume=False``) refuses a directory that already holds
    checkpoints. The budget is checked before each question.
    """
    if not resume and checkpoint.has_any():
        raise ValueError(f"checkpoints already exist in {checkpoint.directory}; pass --resume")
    done = checkpoint.done_ids()
    processed = 0
    for question in questions:
        if question.qid in done:
            continue
        if ledger is not None:
            ledger.check()
        result = process(question)
        checkpoint.save(question.qid, result)
        processed += 1
    return processed


# --- resilience --------------------------------------------------------------


def _status_code(exc: BaseException) -> int | None:
    status = getattr(exc, "status_code", None)
    return status if isinstance(status, int) else None


def _retryable(exc: BaseException) -> bool:
    status = _status_code(exc)
    if status is not None:
        return status in {408, 409, 429} or status >= 500
    return isinstance(exc, (ConnectionError, TimeoutError)) or type(exc).__name__ in {
        "APIConnectionError", "APITimeoutError",
    }


def _retry_after_seconds(exc: BaseException) -> float | None:
    response = getattr(exc, "response", None)
    headers = getattr(response, "headers", None)
    if headers is None:
        return None
    for key, value in headers.items():
        if str(key).lower() == "retry-after":
            try:
                return max(0.0, float(value))
            except (TypeError, ValueError):
                return None
    return None


def with_backoff(
    call: Callable[[], Any],
    *,
    max_attempts: int = 8,
    base_seconds: float = 2.0,
    max_seconds: float = 120.0,
    sleep: Callable[[float], Any] = time.sleep,
) -> Any:
    """Run ``call``, retrying 408/409/429/5xx and connection faults with exponential backoff.

    A ``Retry-After`` header is honoured (up to 15 minutes) when it is longer
    than the exponential delay. Other errors, and the last failure, propagate.
    """
    for attempt in range(max_attempts):
        try:
            return call()
        except Exception as exc:
            if not _retryable(exc) or attempt == max_attempts - 1:
                raise
            delay = min(max_seconds, base_seconds * (2 ** attempt))
            retry_after = _retry_after_seconds(exc)
            if retry_after is not None:
                delay = max(delay, min(retry_after, 900.0))
            sleep(delay)
    raise AssertionError("unreachable")


# --- gold mapping and the official harness's parsing -------------------------


_REF = re.compile(r"l:\d+:D\d+:\d+")


def _canonical_id(token: str) -> str | None:
    if not token.startswith("D"):
        return None
    rest = token[1:]
    if rest.startswith(":"):
        rest = rest[1:]
    session, sep, turn = rest.partition(":")
    if not sep or not session.isdigit() or not turn.isdigit():
        return None
    return f"D{int(session)}:{int(turn)}"


def normalize_evidence(raw: str) -> list[str]:
    """Split one raw evidence entry into ``D<session>:<turn>`` ids.

    Mirrors ``fathomdb-mcp/examples/locomo_eval/dataset.rs``: ``;`` and
    whitespace separate ids, ``D:11:26`` and zero-padded turns are repaired,
    and non-id tokens are dropped.
    """
    return [cid for token in re.split(r"[;\s]+", raw) if (cid := _canonical_id(token))]


def refs_in_text(text: str) -> set[str]:
    """Every ``l:<conv>:D<s>:<t>`` reference that appears in tool output text."""
    return set(_REF.findall(text))


def unwrap_judge_json(raw: str) -> dict[str, Any]:
    """Parse judge JSON and unwrap a lone ``final`` wrapper, as the official client does."""
    parsed = json.loads(raw.strip())
    if isinstance(parsed, dict) and "final" in parsed and len(parsed) == 1:
        inner = parsed["final"]
        if isinstance(inner, str):
            parsed = json.loads(inner)
        elif isinstance(inner, dict):
            parsed = inner
    return parsed


def judge_score(parsed: object) -> float:
    """1.0 when the judge's label is CORRECT (case-insensitive), else 0.0."""
    if not isinstance(parsed, dict):
        return 0.0
    return 1.0 if str(parsed.get("label", "")).upper() == "CORRECT" else 0.0


def extract_answer(text: str) -> str:
    """Text after the last ``ANSWER:`` marker, or the whole text when there is none."""
    text = text.strip()
    if "ANSWER:" in text:
        return text.rsplit("ANSWER:", 1)[-1].strip()
    return text


def tools_to_functions(tools: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Convert MCP ``tools/list`` entries to chat-completions functions; ``outputSchema`` is dropped."""
    return [
        {"type": "function", "function": {
            "name": tool["name"], "description": tool.get("description", ""), "parameters": tool["inputSchema"],
        }}
        for tool in tools
    ]


# --- agent loop --------------------------------------------------------------


@dataclass(frozen=True)
class ToolCall:
    """One function call requested by the model."""

    id: str
    name: str
    arguments: str


@dataclass
class ModelTurn:
    """One chat-completions response."""

    content: str | None
    tool_calls: list[ToolCall]
    usage: Usage
    served_model: str | None = None
    route_cost_usd: float | None = None
    finish_reason: str | None = None


@dataclass
class AgentTrace:
    """Everything one agent episode did."""

    final_answer: str
    messages: list[dict[str, Any]]
    tool_log: list[dict[str, Any]]
    turn_usage: list[ModelTurn]
    forced_final: bool
    forced_reason: str | None
    refs_seen: set[str]


ModelTurnFn = Callable[[list[dict[str, Any]], list[dict[str, Any]], str], ModelTurn]
ToolFn = Callable[[str, dict[str, Any]], tuple[str, bool]]


def _function_name(function: dict[str, Any]) -> str:
    return function["function"]["name"]


def run_agent_loop(
    question: str,
    *,
    system_prompt: str,
    functions: list[dict[str, Any]],
    model_turn: ModelTurnFn,
    call_tool: ToolFn,
    max_calls: int = MAX_TOOL_CALLS,
    max_searches: int = MAX_SEARCH_CALLS,
    max_turns: int = MAX_MODEL_TURNS,
) -> AgentTrace:
    """Run one budgeted tool-use episode and return its trace.

    At most ``max_calls`` tool calls execute, at most ``max_searches`` of them
    ``search``; ``search`` stops being offered once its budget is spent. Calls
    past a budget are answered with a not-executed message, so every call id
    gets a tool message. When the call budget (or the turn cap) is spent, one
    final turn runs with ``tool_choice="none"``.
    """
    messages: list[dict[str, Any]] = [
        {"role": "system", "content": system_prompt},
        {"role": "user", "content": question},
    ]
    names = {_function_name(f) for f in functions}
    tool_log: list[dict[str, Any]] = []
    turns: list[ModelTurn] = []
    refs_seen: set[str] = set()
    executed = searches = 0
    forced_reason: str | None = None
    final_answer: str | None = None
    while True:
        if executed >= max_calls:
            forced_reason = "call_budget"
            break
        if len(turns) >= max_turns:
            forced_reason = "turn_cap"
            break
        offered = [f for f in functions if not (_function_name(f) == "search" and searches >= max_searches)]
        turn = model_turn(messages, offered, "auto")
        turns.append(turn)
        if not turn.tool_calls:
            final_answer = turn.content or ""
            messages.append({"role": "assistant", "content": final_answer})
            break
        messages.append({
            "role": "assistant", "content": turn.content,
            "tool_calls": [
                {"id": c.id, "type": "function", "function": {"name": c.name, "arguments": c.arguments}}
                for c in turn.tool_calls
            ],
        })
        for call in turn.tool_calls:
            entry: dict[str, Any] = {
                "turn": len(turns), "name": call.name, "args_chars": len(call.arguments or ""),
                "executed": False, "is_error": False, "not_executed_reason": None,
            }
            try:
                args = json.loads(call.arguments) if call.arguments else {}
            except json.JSONDecodeError:
                args = None
            entry["arguments"] = args if isinstance(args, dict) else call.arguments
            if executed >= max_calls:
                text, entry["not_executed_reason"] = (
                    f"Not executed: the tool-call budget ({max_calls} calls) is spent.", "call_budget")
            elif call.name == "search" and searches >= max_searches:
                text, entry["not_executed_reason"] = (
                    f"Not executed: the search budget ({max_searches} searches) is spent.", "search_budget")
            elif call.name not in names:
                text, entry["not_executed_reason"] = (f"Not executed: unknown tool {call.name!r}.", "unknown_tool")
            elif not isinstance(args, dict):
                text, entry["not_executed_reason"] = (
                    "Not executed: the arguments are not a JSON object.", "bad_arguments")
            else:
                text, is_error = call_tool(call.name, args)
                entry["executed"] = True
                entry["is_error"] = bool(is_error)
                executed += 1
                if call.name == "search":
                    searches += 1
                refs_seen |= refs_in_text(text)
            entry["result_chars"] = len(text)
            tool_log.append(entry)
            messages.append({"role": "tool", "tool_call_id": call.id, "content": text})
    if final_answer is None:
        turn = model_turn(messages, functions, "none")
        turns.append(turn)
        final_answer = turn.content or ""
        messages.append({"role": "assistant", "content": final_answer})
    return AgentTrace(
        final_answer=final_answer, messages=messages, tool_log=tool_log, turn_usage=turns,
        forced_final=forced_reason is not None, forced_reason=forced_reason, refs_seen=refs_seen,
    )


# --- statistics --------------------------------------------------------------


def bootstrap_ci(values: list[float], *, seed: int, resamples: int) -> tuple[float, float]:
    """Two-sided 95% percentile bootstrap interval of the mean."""
    if not values:
        raise ValueError("bootstrap needs at least one value")
    rng = random.Random(seed)
    n = len(values)
    means = sorted(sum(rng.choices(values, k=n)) / n for _ in range(resamples))
    lower = means[max(0, math.floor(0.025 * resamples))]
    upper = means[min(resamples - 1, math.ceil(0.975 * resamples) - 1)]
    return lower, upper
