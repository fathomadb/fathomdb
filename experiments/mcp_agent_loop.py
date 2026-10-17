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
# The guided variant's client half; the server half is the live initialize
# ``instructions``, prepended as a real MCP client does.
GUIDED_CLIENT_PROMPT = (
    "You answer questions from a memory store reached through the FathomDB tools. Always search before "
    "answering, even if you think the memory will not have it. Use the key names, objects and events from "
    "the question as the query, and keep the default limit of 10. Snippets show only the start of each "
    "record; when a result looks relevant but its snippet is cut off or does not show the answer, fetch it. "
    "If the first search misses, rephrase (synonyms, the other person's name, a related event) and search "
    "again. Records may begin with a timestamp: use it to resolve relative times like \"last week\" into "
    "absolute dates. Answer concisely from what you retrieved. Say you do not know only after at least two "
    "searches found nothing relevant."
)
PROMPT_VARIANTS = ("neutral", "guided")


def agent_system_prompt(variant: str, *, instructions: str | None) -> str:
    """The agent's system prompt for a prompt variant.

    ``neutral`` is the run-1 prompt and ignores ``instructions``. ``guided`` is
    the server's initialize ``instructions``, a blank line, then
    ``GUIDED_CLIENT_PROMPT``; it raises ``ValueError`` when the server sent no
    instructions. Any other variant raises ``ValueError``.
    """
    if variant == "neutral":
        return AGENT_SYSTEM_PROMPT
    if variant == "guided":
        if instructions is None or not instructions.strip():
            raise ValueError("the guided prompt needs the server's initialize instructions")
        return instructions + "\n\n" + GUIDED_CLIENT_PROMPT
    raise ValueError(f"unknown prompt variant {variant!r}")


MAX_TOOL_CALLS = 4
MAX_SEARCH_CALLS = 3
MAX_MODEL_TURNS = 8

# USD per million tokens. Reasoning tokens are part of completion tokens and
# bill at the output price.
PRICES_PER_MTOK: dict[str, dict[str, float]] = {
    "gpt-4o-mini": {"input": 0.15, "cached_input": 0.075, "output": 0.60},
    "gpt-5-mini": {"input": 0.25, "cached_input": 0.025, "output": 2.00},
    "gpt-5.4-mini": {"input": 0.75, "cached_input": 0.075, "output": 4.50},
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
    rest = token[1:].removeprefix(":")
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


# --- live inputs (external; nothing below is exercised by unit tests) --------

DATASET_RAW_SHA256 = "79fa87e90f04081343b8c8debecb80a9a6842b76a7aa537dc9fdf651ea698ff4"
# benchmarks/locomo/prompts.py of mem0ai/memory-benchmarks at 4b61c5d; the
# MEMORY-01 resilience patch (41d1e633) does not touch it.
HARNESS_PROMPTS_SHA256 = "8ebac1ef60e9ab5caf99079fdaac038b85472e81491ed35e2d2655f3927c76c2"
HARNESS_UPSTREAM_SHA = "4b61c5d31b9c668a12b4f5e78064248a02c82d2b"
BOOTSTRAP_SEED = 20260814
BOOTSTRAP_RESAMPLES = 10_000
ARM_SPECS: dict[str, dict[str, str]] = {
    "A1": {"kind": "single", "mode": "text", "model": "gpt-4o-mini"},
    "A2": {"kind": "single", "mode": "hybrid", "model": "gpt-4o-mini"},
    "B-4o": {"kind": "agent", "mode": "hybrid", "model": "gpt-4o-mini", "prompt": "neutral"},
    "B-5mini": {"kind": "agent", "mode": "hybrid", "model": "gpt-5-mini", "prompt": "neutral"},
    "B-5.4mini": {"kind": "agent", "mode": "hybrid", "model": "gpt-5.4-mini", "prompt": "neutral"},
    "B-4o-guided": {"kind": "agent", "mode": "hybrid", "model": "gpt-4o-mini", "prompt": "guided"},
    "B-5.4mini-guided": {"kind": "agent", "mode": "hybrid", "model": "gpt-5.4-mini", "prompt": "guided"},
}
JUDGE_MODEL = "gpt-4o-mini"
# The route must serve exactly the requested model; an alias remap stops the run.
SERVED_MODEL_PREFIX = {
    "gpt-4o-mini": "openai/gpt-4o-mini", "gpt-5-mini": "openai/gpt-5-mini", "gpt-5.4-mini": "openai/gpt-5.4-mini",
}


class ServedModelMismatch(RuntimeError):
    """Raised when the route answers with a different model than requested."""


# Labels whose route model id differs from the label.
REQUEST_MODEL_ID = {"gpt-5.4-mini": "openai/gpt-5.4-mini"}


def request_model_id(model: str) -> str:
    """The model id sent to the route for a model label."""
    return REQUEST_MODEL_ID.get(model, model)


def request_params(model: str, *, max_tokens: int) -> dict[str, Any]:
    """Sampling parameters: reasoning models run at effort ``low``; others at temperature 0."""
    if model.startswith("gpt-5"):
        return {"reasoning_effort": "low", "max_completion_tokens": max_tokens}
    return {"temperature": 0, "max_tokens": max_tokens}


def check_served_model(model: str, served: str | None) -> None:
    """Raise ``ServedModelMismatch`` unless ``served`` starts with the model's provider id."""
    if served is None or not served.startswith(SERVED_MODEL_PREFIX[model]):
        raise ServedModelMismatch(f"route served {served!r} for requested {model!r}")


def load_harness_prompts(checkout: Path) -> Any:
    """Import the official LOCOMO prompt module from its pinned external checkout."""
    import hashlib
    import importlib.util

    path = checkout / "benchmarks" / "locomo" / "prompts.py"
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if digest != HARNESS_PROMPTS_SHA256:
        raise ValueError(f"harness prompts.py sha256 mismatch: {digest}")
    spec = importlib.util.spec_from_file_location("locomo_harness_prompts", path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def _parse_locomo_date(text: str) -> Any:
    from datetime import datetime

    for fmt in ("%I:%M %p on %d %B, %Y", "%I:%M %p on %d %b, %Y"):
        try:
            return datetime.strptime(text, fmt)
        except (ValueError, TypeError):
            continue
    return None


def reference_date(conversation: dict[str, Any]) -> str:
    """The last session's raw date string, as the official ``run.py`` passes to the answerer."""
    from datetime import datetime

    keys = [k for k in conversation if re.match(r"^session_\d+$", k)]

    def sort_key(key: str) -> tuple:
        parsed = _parse_locomo_date(conversation.get(f"{key}_date_time", ""))
        if parsed:
            return (0, parsed)
        return (1, datetime(2000, 1, int(re.search(r"\d+", key).group())))

    return conversation.get(f"{max(keys, key=sort_key)}_date_time", "")


def evidence_lookup(conversation: dict[str, Any]) -> dict[str, str]:
    """``dia_id`` to judge evidence line, as the official ``load_evidence_lookup`` builds it."""
    session_dates = {
        key.replace("session_", "").replace("_date_time", ""): value
        for key, value in conversation.items()
        if key.endswith("_date_time") and key.startswith("session_")
    }
    lookup: dict[str, str] = {}
    for key, turns in conversation.items():
        if not key.startswith("session_") or key.endswith("date_time") or not isinstance(turns, list):
            continue
        for turn in turns:
            dia_id = turn.get("dia_id", "")
            if not dia_id:
                continue
            match = re.match(r"D(\d+):", dia_id)
            suffix = ""
            if match and session_dates.get(match.group(1), ""):
                suffix = f", said on {session_dates[match.group(1)]}"
            lookup[dia_id] = f'[{dia_id}{suffix}] {turn.get("speaker", "")}: "{turn.get("text", "")}"'
    return lookup


def load_questions(dataset_path: Path, conv_idx: int = 0) -> tuple[list[Question], dict[str, Any]]:
    """Category 1-4 items of one conversation with at least one evidence id that maps to a turn."""
    import hashlib

    raw = dataset_path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != DATASET_RAW_SHA256:
        raise ValueError("LoCoMo dataset sha256 does not match the Mem0 comparison corpus")
    entry = json.loads(raw)[conv_idx]
    conversation = entry["conversation"]
    dia_ids = {
        turn["dia_id"] for key, turns in conversation.items()
        if key.startswith("session_") and isinstance(turns, list) for turn in turns if turn.get("dia_id")
    }
    questions: list[Question] = []
    for qi, qa in enumerate(entry.get("qa", [])):
        if qa.get("category") not in {1, 2, 3, 4}:
            continue
        gold: list[str] = []
        for item in qa.get("evidence", []):
            for dia in normalize_evidence(str(item)):
                ref = f"l:{conv_idx}:{dia}"
                if dia in dia_ids and ref not in gold:
                    gold.append(ref)
        if not gold:
            continue
        questions.append(Question(
            qid=f"conv{conv_idx}_q{qi}", category=qa["category"], question=qa["question"],
            answer=str(qa.get("answer", "")), evidence=[str(e) for e in qa.get("evidence", [])], gold_refs=gold,
        ))
    return questions, conversation


class McpSession:
    """Newline-delimited JSON-RPC client for one ``fathomdb-mcp`` stdio process."""

    def __init__(self, binary: Path, config_path: Path, *, env: dict[str, str], stderr_path: Path) -> None:
        import subprocess

        self._stderr = stderr_path.open("a", encoding="utf-8")
        self.process = subprocess.Popen(
            [str(binary), "--config", str(config_path)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=self._stderr, env=env, text=True, bufsize=1,
        )
        self._next_id = 0
        init = self._request("initialize", {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "mcp-agent-loop", "version": "1"},
        })
        self.server_info = init["result"].get("serverInfo", {})
        self.instructions: str | None = init["result"].get("instructions")
        self._send({"jsonrpc": "2.0", "method": "notifications/initialized"})

    def _send(self, message: dict[str, Any]) -> None:
        assert self.process.stdin is not None
        self.process.stdin.write(json.dumps(message) + "\n")
        self.process.stdin.flush()

    def _request(self, method: str, params: dict[str, Any], *, timeout_s: float = 300.0) -> dict[str, Any]:
        import select

        self._next_id += 1
        request_id = self._next_id
        self._send({"jsonrpc": "2.0", "id": request_id, "method": method, "params": params})
        assert self.process.stdout is not None
        deadline = time.monotonic() + timeout_s
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError(f"MCP {method} timed out")
            ready, _, _ = select.select([self.process.stdout], [], [], remaining)
            if not ready:
                continue
            line = self.process.stdout.readline()
            if not line:
                raise RuntimeError("fathomdb-mcp exited")
            message = json.loads(line)
            if message.get("id") == request_id:
                return message

    def list_tools(self) -> list[dict[str, Any]]:
        """The server's ``tools/list``."""
        return self._request("tools/list", {})["result"]["tools"]

    def call_tool(self, name: str, arguments: dict[str, Any]) -> tuple[str, bool, dict[str, Any] | None]:
        """Call one tool; return its text content, ``isError`` and structured content."""
        response = self._request("tools/call", {"name": name, "arguments": arguments})
        if "error" in response:
            return f"error: {response['error'].get('message', 'protocol error')}", True, None
        result = response["result"]
        text = "\n".join(block.get("text", "") for block in result.get("content", []) if block.get("type") == "text")
        return text, bool(result.get("isError")), result.get("structuredContent")

    def close(self) -> None:
        """Close stdin and wait for the server to exit."""
        try:
            if self.process.stdin:
                self.process.stdin.close()
            self.process.wait(timeout=30)
        finally:
            self._stderr.close()


def read_env_file(path: Path) -> dict[str, str]:
    """Parse a ``KEY=value`` env file without exporting it."""
    values: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line and not line.startswith("#") and "=" in line:
            key, value = line.split("=", 1)
            values[key.strip()] = value.strip().strip('"').strip("'")
    return values


class ChatClient:
    """OpenAI-compatible chat client on the Airlock route; every call is metered."""

    def __init__(self, base_url: str, api_key: str, ledger: CostLedger, *, arm: str) -> None:
        import openai

        self._client = openai.OpenAI(base_url=base_url, api_key=api_key, timeout=180.0, max_retries=0)
        self.ledger = ledger
        self.arm = arm
        self.served: dict[str, set[str]] = {}

    def complete(self, *, qid: str, role: str, model: str, messages: list[dict[str, Any]],
                 functions: list[dict[str, Any]] | None = None, tool_choice: str | None = None,
                 json_object: bool = False, max_tokens: int = 4096) -> ModelTurn:
        """One metered completion; raises ``BudgetExceeded`` before the call when the cap is reached."""
        self.ledger.check()
        kwargs: dict[str, Any] = {"model": request_model_id(model), "messages": messages,
                                  **request_params(model, max_tokens=max_tokens)}
        if functions is not None:
            kwargs["tools"] = functions
            kwargs["tool_choice"] = tool_choice or "auto"
        if json_object:
            kwargs["response_format"] = {"type": "json_object"}
        raw = with_backoff(lambda: self._client.chat.completions.with_raw_response.create(**kwargs))
        response = raw.parse()
        usage_obj = response.usage
        prompt_details = getattr(usage_obj, "prompt_tokens_details", None)
        completion_details = getattr(usage_obj, "completion_tokens_details", None)
        usage = Usage(
            input_tokens=usage_obj.prompt_tokens,
            cached_input_tokens=getattr(prompt_details, "cached_tokens", None) or 0,
            output_tokens=usage_obj.completion_tokens,
            reasoning_tokens=getattr(completion_details, "reasoning_tokens", None) or 0,
        )
        served = raw.headers.get("x-litellm-model-name")
        route_cost = raw.headers.get("x-litellm-response-cost")
        route_cost_usd = float(route_cost) if route_cost not in (None, "") else None
        self.ledger.record(arm=self.arm, qid=qid, role=role, model=model, usage=usage,
                           route_cost_usd=route_cost_usd, served_model=served)
        self.served.setdefault(model, set()).add(str(served))
        check_served_model(model, served)
        message = response.choices[0].message
        calls = [ToolCall(id=c.id, name=c.function.name, arguments=c.function.arguments or "")
                 for c in (message.tool_calls or [])]
        return ModelTurn(content=message.content, tool_calls=calls, usage=usage, served_model=served,
                         route_cost_usd=route_cost_usd, finish_reason=response.choices[0].finish_reason)


def _usage_dict(turn: ModelTurn, role: str, model: str) -> dict[str, Any]:
    return {
        "role": role, "model": model, "served_model": turn.served_model, "finish_reason": turn.finish_reason,
        "input_tokens": turn.usage.input_tokens, "cached_input_tokens": turn.usage.cached_input_tokens,
        "output_tokens": turn.usage.output_tokens, "reasoning_tokens": turn.usage.reasoning_tokens,
        "cost_usd": cost_usd(model, turn.usage), "route_cost_usd": turn.route_cost_usd,
    }


class Grader:
    """The official answer prompt and evidence-aware judge, called as the official client calls them."""

    def __init__(self, prompts: Any, conversation: dict[str, Any], client: ChatClient) -> None:
        self.prompts = prompts
        self.reference_date = reference_date(conversation)
        self.lookup = evidence_lookup(conversation)
        self.client = client

    def answer(self, question: Question, bodies: list[str], calls: list[dict[str, Any]]) -> str:
        """Generate an answer from ranked bodies; the official FathomDB arm carries no ``created_at``."""
        prompt = self.prompts.get_answer_generation_prompt(
            question.question, [{"memory": body} for body in bodies], reference_date=self.reference_date,
        )
        for attempt in range(5):
            turn = self.client.complete(qid=question.qid, role="answerer", model="gpt-4o-mini",
                                        messages=[{"role": "user", "content": prompt}])
            calls.append(_usage_dict(turn, "answerer", "gpt-4o-mini"))
            if turn.content is not None:
                return extract_answer(turn.content)
            time.sleep(2 * (attempt + 1))
        return ""

    def judge(self, question: Question, generated: str, calls: list[dict[str, Any]]) -> dict[str, Any]:
        """Grade one answer with the evidence-aware prompt (plain prompt when no evidence resolves)."""
        gold = self.prompts.preprocess_answer(question.category, question.answer)
        evidence = "\n".join(self.lookup[ref] for ref in question.evidence if ref in self.lookup).strip()
        if evidence:
            prompt = self.prompts.get_judge_prompt_with_evidence(
                question.category, question.question, gold, generated, evidence)
        else:
            prompt = self.prompts.get_judge_prompt(question.category, question.question, gold, generated)
        parsed: dict[str, Any] = {}
        for attempt in range(5):
            turn = self.client.complete(
                qid=question.qid, role="judge", model=JUDGE_MODEL, json_object=True,
                messages=[{"role": "system", "content": self.prompts.JUDGE_SYSTEM_PROMPT},
                          {"role": "user", "content": prompt}],
            )
            calls.append(_usage_dict(turn, "judge", JUDGE_MODEL))
            if turn.content:
                try:
                    parsed = unwrap_judge_json(turn.content)
                    break
                except (json.JSONDecodeError, ValueError):
                    pass
            if attempt < 4:
                time.sleep(2 * (attempt + 1))
        reasoning = parsed.get("reasoning", "") if isinstance(parsed, dict) else ""
        return {"label": parsed.get("label") if isinstance(parsed, dict) else None,
                "reasoning": reasoning, "score": judge_score(parsed), "evidence_aware": bool(evidence)}


def _single_search(question: Question, *, mcp: McpSession, mode: str, grader: Grader) -> dict[str, Any]:
    calls: list[dict[str, Any]] = []
    text, is_error, structured = mcp.call_tool("search", {"query": question.question, "mode": mode, "limit": 10})
    if is_error or structured is None:
        raise RuntimeError(f"search failed for {question.qid}: {text[:200]}")
    hits = structured.get("results", [])
    refs = [hit["ref"] for hit in hits]
    bodies: list[str] = []
    fetch_errors = 0
    if refs:
        fetch_text, fetch_error, fetched = mcp.call_tool("fetch", {"refs": refs})
        if fetch_error or fetched is None:
            raise RuntimeError(f"fetch failed for {question.qid}: {fetch_text[:200]}")
        for item in fetched.get("items", []):
            if "record" in item:
                bodies.append(item["record"]["body"])
            else:
                fetch_errors += 1
    generated = grader.answer(question, bodies, calls)
    verdict = grader.judge(question, generated, calls)
    gold = set(question.gold_refs)
    return {
        "category": question.category, "search_refs": refs, "search_arms": [hit.get("arm") for hit in hits],
        "completeness": structured.get("completeness"), "degradation": structured.get("degradation"),
        "fetch_errors": fetch_errors, "context_count": len(bodies),
        "gold_in_context": bool(gold & set(refs)), "gold_hit_any": bool(gold & set(refs)),
        "answer": generated, "judge": verdict, "score": verdict["score"], "calls": calls,
    }


def _agent(question: Question, *, mcp: McpSession, functions: list[dict[str, Any]], model: str,
           system_prompt: str, grader: Grader) -> dict[str, Any]:
    calls: list[dict[str, Any]] = []
    # The session default is per server; reset it so one question's use_store cannot leak into the next.
    mcp.call_tool("use_store", {"store": "conv0"})

    def model_turn(messages: list[dict[str, Any]], offered: list[dict[str, Any]], tool_choice: str) -> ModelTurn:
        turn = grader.client.complete(qid=question.qid, role="agent", model=model, messages=messages,
                                      functions=offered, tool_choice=tool_choice)
        calls.append({**_usage_dict(turn, "agent", model), "tool_choice": tool_choice,
                      "offered": [_function_name(f) for f in offered]})
        return turn

    results: list[dict[str, Any]] = []

    def call_tool(name: str, args: dict[str, Any]) -> tuple[str, bool]:
        text, is_error, structured = mcp.call_tool(name, args)
        info: dict[str, Any] = {"name": name, "is_error": is_error}
        if isinstance(structured, dict):
            if is_error:
                error = structured.get("error", {})
                info["error"] = f"{error.get('class')}/{error.get('code')}"
            info["store"] = structured.get("store")
            if name == "search":
                info["results"] = len(structured.get("results", []))
                info["completeness"] = structured.get("completeness")
            elif name == "expand":
                info["targets"] = len(structured.get("targets", []))
            elif name == "fetch":
                info["items"] = len(structured.get("items", []))
        results.append(info)
        return text, is_error

    trace = run_agent_loop(question.question, system_prompt=system_prompt, functions=functions,
                           model_turn=model_turn, call_tool=call_tool)
    generated = extract_answer(trace.final_answer)
    verdict = grader.judge(question, generated, calls)
    gold = set(question.gold_refs)
    asked = question.question.strip().casefold()
    searches = [c for c in trace.tool_log if c["executed"] and c["name"] == "search"]
    return {
        "category": question.category, "answer": generated, "judge": verdict, "score": verdict["score"],
        "tool_log": trace.tool_log, "tool_results": results, "forced_final": trace.forced_final,
        "forced_reason": trace.forced_reason, "refs_seen": sorted(trace.refs_seen),
        "gold_hit_any": bool(gold & trace.refs_seen), "n_searches": len(searches),
        "search_verbatim": [
            isinstance(c["arguments"], dict) and str(c["arguments"].get("query", "")).strip().casefold() == asked
            for c in searches
        ],
        "transcript": trace.messages, "calls": calls,
    }


def server_config(stores_dir: Path, mode: str) -> dict[str, Any]:
    """The fixed server configuration; snippet_chars and default_max_tokens stay at their defaults."""
    return {
        "roots": [{"path": str(stores_dir), "suffix": ".sqlite"}], "default_store": "conv0",
        "use_default_embedder": True, "idle_timeout_secs": 0, "call_deadline_ms": 120000,
        "search_mode": mode,
    }


def run_live(args: Any) -> int:
    """Run one arm against the live server and route, checkpointing each question."""
    import hashlib

    spec = ARM_SPECS[args.arm]
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    questions, conversation = load_questions(Path(args.dataset), 0)
    if len(questions) != 150:
        raise ValueError(f"expected 150 conversation-0 questions, found {len(questions)}")
    if args.limit is not None:
        questions = questions[: args.limit]
    prompts = load_harness_prompts(Path(args.harness))
    secrets = read_env_file(Path(args.env_file))
    ledger = CostLedger(out / "ledger", cap_usd=args.cap)
    client = ChatClient(secrets["R2_JUDGE_BASE_URL"], secrets["R2_JUDGE_API_KEY"], ledger, arm=args.arm)
    grader = Grader(prompts, conversation, client)
    config_path = out / f"server-{args.arm}.json"
    config_path.write_text(json.dumps(server_config(Path(args.stores), spec["mode"]), indent=1), encoding="utf-8")
    env = dict(os.environ, CUDA_VISIBLE_DEVICES="0", FATHOMDB_EMBED_DEVICE="cuda:0")
    mcp = McpSession(Path(args.binary), config_path, env=env, stderr_path=out / f"server-{args.arm}.stderr")
    try:
        tools = mcp.list_tools()
        (out / "tools_list.json").write_text(json.dumps(tools, indent=1, sort_keys=True), encoding="utf-8")
        functions = tools_to_functions(tools)
        if spec["kind"] == "single":
            def process(question: Question) -> dict[str, Any]:
                return _single_search(question, mcp=mcp, mode=spec["mode"], grader=grader)
        else:
            system_prompt = agent_system_prompt(spec["prompt"], instructions=mcp.instructions)
            prompt_record = {"arm": args.arm, "variant": spec["prompt"],
                             "system_prompt_sha256": hashlib.sha256(system_prompt.encode()).hexdigest(),
                             "instructions_sha256": hashlib.sha256((mcp.instructions or "").encode()).hexdigest(),
                             "instructions_bytes": len((mcp.instructions or "").encode())}
            (out / f"prompt-{args.arm}.json").write_text(json.dumps(prompt_record, indent=1), encoding="utf-8")

            def process(question: Question) -> dict[str, Any]:
                return _agent(question, mcp=mcp, functions=functions, model=spec["model"],
                              system_prompt=system_prompt, grader=grader)
        checkpoint = ArmCheckpoint(out / "arms" / args.arm)
        processed = run_arm(questions, lambda q: {"arm": args.arm, **process(q)}, checkpoint,
                            resume=args.resume, ledger=ledger)
    finally:
        mcp.close()
    print(json.dumps({"arm": args.arm, "processed": processed, "total_usd": round(ledger.total_usd(), 6),
                      "arm_usd": round(ledger.arm_totals().get(args.arm, 0.0), 6),
                      "served": {k: sorted(v) for k, v in client.served.items()}}))
    return 0


# --- content-free aggregation ------------------------------------------------


def _rate(flags: list[bool]) -> float | None:
    return sum(flags) / len(flags) if flags else None


def _histogram(values: list[int]) -> dict[str, int]:
    counts: dict[str, int] = {}
    for value in values:
        counts[str(value)] = counts.get(str(value), 0) + 1
    return dict(sorted(counts.items(), key=lambda item: int(item[0])))


def _accuracy_block(scores: list[float], *, seed: int, resamples: int) -> dict[str, Any]:
    low, high = bootstrap_ci(scores, seed=seed, resamples=resamples)
    return {"n": len(scores), "accuracy": sum(scores) / len(scores), "ci95": [low, high]}


def _token_totals(results: dict[str, dict[str, Any]]) -> dict[str, Any]:
    roles: dict[str, dict[str, float]] = {}
    for result in results.values():
        for call in result["calls"]:
            row = roles.setdefault(call["role"], {
                "calls": 0, "input_tokens": 0, "cached_input_tokens": 0, "output_tokens": 0,
                "reasoning_tokens": 0, "cost_usd": 0.0, "route_cost_usd": 0.0,
            })
            row["calls"] += 1
            for key in ("input_tokens", "cached_input_tokens", "output_tokens", "reasoning_tokens", "cost_usd"):
                row[key] += call[key]
            row["route_cost_usd"] += call["route_cost_usd"] or 0.0
    return roles


def _agent_usage(results: dict[str, dict[str, Any]]) -> dict[str, Any]:
    tool_names = ("search", "fetch", "expand", "orient", "list_stores", "use_store", "feedback")
    per_tool: dict[str, Any] = {}
    for name in tool_names:
        counts = [sum(1 for c in r["tool_log"] if c["executed"] and c["name"] == name) for r in results.values()]
        per_tool[name] = {"calls": sum(counts), "questions_using": sum(1 for c in counts if c),
                          "per_question": _histogram(counts)}
    mode_values: dict[str, int] = {}
    override_flags: list[bool] = []
    other_store_flags: list[bool] = []
    for result in results.values():
        searches = [c for c in result["tool_log"] if c["executed"] and c["name"] == "search"]
        modes = [c["arguments"].get("mode") for c in searches if isinstance(c["arguments"], dict)]
        for mode in modes:
            mode_values[str(mode)] = mode_values.get(str(mode), 0) + 1
        override_flags.append(any(mode is not None for mode in modes))
        other_store_flags.append(any(info.get("store") not in (None, "conv0") for info in result["tool_results"]))
    limits = _histogram_str([c["arguments"].get("limit") for r in results.values() for c in r["tool_log"]
                             if c["executed"] and c["name"] == "search" and isinstance(c["arguments"], dict)])
    expand_targets = [info.get("targets", 0) for r in results.values() for info in r["tool_results"]
                      if info["name"] == "expand" and not info["is_error"]]
    searches_per_q = [r["n_searches"] for r in results.values()]
    calls_per_q = [sum(1 for c in r["tool_log"] if c["executed"]) for r in results.values()]
    return {
        "tool_calls_per_question": _histogram(calls_per_q),
        "searches_per_question": _histogram(searches_per_q),
        "mean_searches": sum(searches_per_q) / len(searches_per_q),
        "mean_tool_calls": sum(calls_per_q) / len(calls_per_q),
        "per_tool": per_tool,
        "searched_again_rate": _rate([n > 1 for n in searches_per_q]),
        "rephrased_rate": _rate([not all(r["search_verbatim"]) for r in results.values() if r["search_verbatim"]]),
        "first_search_verbatim_rate": _rate([r["search_verbatim"][0] for r in results.values()
                                             if r["search_verbatim"]]),
        "no_search_rate": _rate([n == 0 for n in searches_per_q]),
        "no_search_accuracy": _rate([r["score"] == 1.0 for r in results.values() if r["n_searches"] == 0]),
        "search_limit_values": limits,
        "mode_override_rate": _rate(override_flags),
        "mode_values": dict(sorted(mode_values.items())),
        "orient_rate": _rate([per > 0 for per in (
            sum(1 for c in r["tool_log"] if c["executed"] and c["name"] == "orient") for r in results.values())]),
        "fetch_rate": _rate([per > 0 for per in (
            sum(1 for c in r["tool_log"] if c["executed"] and c["name"] == "fetch") for r in results.values())]),
        "expand_calls_with_targets": sum(1 for t in expand_targets if t > 0),
        "expand_calls_ok": len(expand_targets),
        "other_store_rate": _rate(other_store_flags),
        "budget_hit_rate": _rate([r["forced_reason"] == "call_budget" for r in results.values()]),
        "turn_cap_rate": _rate([r["forced_reason"] == "turn_cap" for r in results.values()]),
        "not_executed_calls": sum(1 for r in results.values() for c in r["tool_log"] if not c["executed"]),
        "tool_errors": sum(1 for r in results.values() for c in r["tool_log"] if c["executed"] and c["is_error"]),
        "error_codes": _histogram_str([info.get("error") for r in results.values() for info in r["tool_results"]
                                       if info["is_error"]]),
        "agent_turns_per_question": _histogram([sum(1 for c in r["calls"] if c["role"] == "agent")
                                                for r in results.values()]),
    }


def _histogram_str(values: list[Any]) -> dict[str, int]:
    counts: dict[str, int] = {}
    for value in values:
        counts[str(value)] = counts.get(str(value), 0) + 1
    return dict(sorted(counts.items()))


PAIRS = (("A2", "A1"), ("B-4o", "A2"), ("B-5mini", "A2"), ("B-5mini", "B-4o"))
# A heuristic for "the answer says the memory lacks it"; reported as a rough rate only.
ABSTENTION_PHRASE = re.compile(
    r"(do not know|don't know|does not (contain|provide|specify|mention)|not (specified|mentioned|available)"
    r"|no (specific )?information)", re.IGNORECASE)


def summarize(out: Path, *, seed: int = BOOTSTRAP_SEED, resamples: int = BOOTSTRAP_RESAMPLES) -> dict[str, Any]:
    """Content-free aggregates over every arm's checkpoints and the spend ledger."""
    import hashlib

    arms: dict[str, dict[str, dict[str, Any]]] = {}
    for arm in ARM_SPECS:
        results = ArmCheckpoint(out / "arms" / arm).load_all()
        if results:
            arms[arm] = results
    ledger = CostLedger(out / "ledger", cap_usd=float("inf"))
    ledger_totals = ledger.arm_totals()
    summary: dict[str, Any] = {"arms": {}, "paired": {}, "ledger_usd": {
        "by_arm": dict(sorted(ledger_totals.items())), "total": ledger.total_usd()}}
    rows: list[list[Any]] = []
    for arm, results in arms.items():
        ordered = sorted(results)
        scores = [results[q]["score"] for q in ordered]
        rows.extend([arm, q, results[q]["score"]] for q in ordered)
        by_category: dict[str, Any] = {}
        for category in (1, 2, 3, 4):
            cat_scores = [results[q]["score"] for q in ordered if results[q]["category"] == category]
            if cat_scores:
                by_category[str(category)] = _accuracy_block(cat_scores, seed=seed, resamples=resamples)
        block: dict[str, Any] = {
            "overall": _accuracy_block(scores, seed=seed, resamples=resamples),
            "by_category": by_category,
            "gold_hit_rate": _rate([bool(results[q]["gold_hit_any"]) for q in ordered]),
            "abstention_phrase_rate": _rate([bool(ABSTENTION_PHRASE.search(results[q]["answer"])) for q in ordered]),
            "abstention_phrase_accuracy": _rate([results[q]["score"] == 1.0 for q in ordered
                                                 if ABSTENTION_PHRASE.search(results[q]["answer"])]),
            "judge_evidence_aware_rate": _rate([bool(results[q]["judge"]["evidence_aware"]) for q in ordered]),
            "tokens_by_role": _token_totals(results),
            "ledger_usd": ledger_totals.get(arm, 0.0),
        }
        if ARM_SPECS[arm]["kind"] == "agent":
            block["tool_usage"] = _agent_usage(results)
            block["accuracy_by_gold_hit"] = {
                str(flag): _rate([results[q]["score"] == 1.0 for q in ordered
                                  if bool(results[q]["gold_hit_any"]) is flag])
                for flag in (True, False)
            }
        else:
            block["search_degradation_count"] = sum(1 for q in ordered if results[q]["degradation"])
            block["completeness"] = _histogram_str([results[q]["completeness"] for q in ordered])
        summary["arms"][arm] = block
    for left, right in PAIRS:
        if left not in arms or right not in arms:
            continue
        common = sorted(set(arms[left]) & set(arms[right]))
        deltas = [arms[left][q]["score"] - arms[right][q]["score"] for q in common]
        if not deltas:
            continue
        low, high = bootstrap_ci(deltas, seed=seed, resamples=resamples)
        summary["paired"][f"{left} - {right}"] = {
            "n": len(common), "difference": sum(deltas) / len(deltas), "ci95": [low, high],
            "left_only_correct": sum(1 for d in deltas if d > 0), "right_only_correct": sum(1 for d in deltas if d < 0),
        }
    summary["score_manifest_sha256"] = hashlib.sha256(
        json.dumps(sorted(rows), separators=(",", ":")).encode()).hexdigest()
    summary["bootstrap"] = {"seed": seed, "resamples": resamples, "method": "percentile, two-sided 95%"}
    return summary


def estimate(out: Path, *, full_n: int = 150) -> dict[str, Any]:
    """Per-question token use and cost from the ledger, extrapolated to ``full_n`` questions."""
    ledger = CostLedger(out / "ledger", cap_usd=float("inf"))
    per_arm: dict[str, Any] = {}
    for entry in ledger._entries():
        row = per_arm.setdefault(entry["arm"], {"qids": set(), "usd": 0.0, "roles": {}})
        row["qids"].add(entry["qid"])
        row["usd"] += CostLedger._charge(entry)
        role = row["roles"].setdefault(entry["role"], {"input": 0, "cached": 0, "output": 0, "reasoning": 0})
        role["input"] += entry["input_tokens"]
        role["cached"] += entry["cached_input_tokens"]
        role["output"] += entry["output_tokens"]
        role["reasoning"] += entry["reasoning_tokens"]
    report: dict[str, Any] = {"arms": {}, "extrapolated_total_usd": 0.0}
    for arm, row in sorted(per_arm.items()):
        n = len(row["qids"])
        per_q = row["usd"] / n
        report["arms"][arm] = {
            "questions": n, "usd": row["usd"], "usd_per_question": per_q, "extrapolated_usd": per_q * full_n,
            "tokens_per_question": {role: {k: v / n for k, v in t.items()} for role, t in row["roles"].items()},
        }
        report["extrapolated_total_usd"] += per_q * full_n
    report["spent_usd"] = ledger.total_usd()
    return report


def write_receipt(out: Path, *, summary: dict[str, Any], resolved: dict[str, Any], verdict: str, read: str,
                  ts: Any, base_dir: Path | None = None) -> tuple[str, Path]:
    """Write the content-free receipt and regenerate ``INDEX.md``."""
    from experiments import _lib

    base = base_dir or _lib.EXPERIMENTS_DIR
    headline = {arm: block["overall"]["accuracy"] for arm, block in summary["arms"].items()}
    run_id, run_dir = _lib.write_record(
        EXPERIMENT, ts=ts, config_obj=resolved, metrics=summary, verdict=verdict, read=read,
        code=_lib.git_info(), corpus={"source": "LOCOMO", "manifest_sha256": DATASET_RAW_SHA256, "datasets": []},
        seeds={"bootstrap": str(BOOTSTRAP_SEED)},
        env=_lib.env_info(key_deps={"harness_prompts_sha256": HARNESS_PROMPTS_SHA256}),
        cost_usd=round(summary["ledger_usd"]["total"], 6), headline=headline,
        n=max((block["overall"]["n"] for block in summary["arms"].values()), default=0),
        base_dir=base,
    )
    _lib.regen_index_md(index_path=Path(base) / "index.jsonl", md_path=Path(base) / "INDEX.md")
    return run_id, run_dir


def gpu_evidence(csv_path: Path) -> dict[str, Any]:
    """Peak ``fathomdb-mcp`` device memory and GPU utilization from the 1 Hz ``nvidia-smi`` samples."""
    index_of: dict[str, str] = {}
    peak_util: dict[str, int] = {}
    peak_mib: dict[str, int] = {}
    samples = 0
    if not csv_path.is_file():
        return {"available": False}
    for line in csv_path.read_text(encoding="utf-8").splitlines():
        parts = [part.strip() for part in line.split(",")]
        if len(parts) >= 6 and parts[1] == "gpu":
            index_of[parts[3]] = parts[2]
            util = int(parts[4].split()[0])
            peak_util[parts[2]] = max(peak_util.get(parts[2], 0), util)
        elif len(parts) >= 6 and parts[1] == "app" and parts[4].endswith("fathomdb-mcp-cuda"):
            samples += 1
            peak_mib[parts[2]] = max(peak_mib.get(parts[2], 0), int(parts[5].split()[0]))
    return {
        "available": True,
        "fathomdb_mcp_samples": samples,
        "fathomdb_mcp_peak_mib_by_gpu_index": {index_of.get(uuid, uuid): mib for uuid, mib in peak_mib.items()},
        "peak_utilization_pct_by_gpu_index": dict(sorted(peak_util.items())),
    }


def resolved_config(*, binary_sha256: str, arm_status: dict[str, str]) -> dict[str, Any]:
    """The non-secret, path-free configuration recorded in the receipt."""
    import hashlib

    server = server_config(Path("external-stores-dir"), "per-arm")
    return {
        "schema_version": SCHEMA_VERSION,
        "campaign": "locomo_conv0_mcp_agent_loop",
        "workload": {
            "dataset_raw_sha256": DATASET_RAW_SHA256, "conversation": 0, "categories": [1, 2, 3, 4],
            "selection": "category 1-4 with at least one evidence id that maps to a turn", "questions": 150,
        },
        "server": {
            "config": server, "binary_sha256": binary_sha256, "cargo_features": ["embed-cuda"],
            "device_env": {"CUDA_VISIBLE_DEVICES": "0", "FATHOMDB_EMBED_DEVICE": "cuda:0"},
            "store": "conv0.sqlite from dev/experiments/mcp-hybrid-vs-keyword (locomo_load, bge-small-en-v1.5)",
        },
        "arms": {
            arm: {**spec, "status": arm_status.get(arm, "not_run")} for arm, spec in ARM_SPECS.items()
        },
        "single_search": {"search_args": {"mode": "per arm", "limit": 10}, "fetch": "the returned refs",
                          "context": "fetched bodies in rank order, no created_at"},
        "agent": {
            "system_prompt_sha256": hashlib.sha256(AGENT_SYSTEM_PROMPT.encode()).hexdigest(),
            "max_tool_calls": MAX_TOOL_CALLS, "max_search_calls": MAX_SEARCH_CALLS,
            "max_model_turns": MAX_MODEL_TURNS, "final_turn_tool_choice": "none",
            "tool_result": "MCP text content block; error text when isError",
            "params": {"gpt-4o-mini": {"temperature": 0, "max_tokens": 4096},
                       "gpt-5-mini": {"reasoning_effort": "low", "max_completion_tokens": 4096}},
        },
        "scoring": {
            "answerer_model": "gpt-4o-mini", "judge_model": JUDGE_MODEL,
            "provider": "openai_compatible_airlock",
            "served_models": {"gpt-4o-mini": "openai/gpt-4o-mini"},
            "evidence_aware": True, "harness_upstream_git_sha": HARNESS_UPSTREAM_SHA,
            "harness_prompts_sha256": HARNESS_PROMPTS_SHA256,
            "answer_params": {"temperature": 0, "max_tokens": 4096},
            "judge_params": {"temperature": 0, "max_tokens": 4096, "response_format": "json_object"},
        },
        "budget": {"validation_questions": 10, "extrapolation_stop_usd": 4.0, "hard_cap_usd": 5.0,
                   "prices_per_mtok": PRICES_PER_MTOK, "charge": "max(table, route-reported)"},
        "bootstrap": {"seed": BOOTSTRAP_SEED, "resamples": BOOTSTRAP_RESAMPLES},
    }


def main(argv: list[str] | None = None) -> int:
    """CLI: ``run`` one arm, ``estimate`` spend, ``summarize`` results, write the ``receipt``."""
    import argparse

    parser = argparse.ArgumentParser(description="MCP agent-loop LoCoMo evaluation")
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run")
    run.add_argument("--arm", choices=sorted(ARM_SPECS), required=True)
    run.add_argument("--out", required=True)
    run.add_argument("--dataset", required=True)
    run.add_argument("--harness", required=True)
    run.add_argument("--stores", required=True)
    run.add_argument("--binary", required=True)
    run.add_argument("--env-file", required=True)
    run.add_argument("--limit", type=int)
    run.add_argument("--cap", type=float, default=5.0)
    run.add_argument("--resume", action="store_true")
    est = sub.add_parser("estimate")
    est.add_argument("--out", required=True)
    summ = sub.add_parser("summarize")
    summ.add_argument("--out", required=True)
    rec = sub.add_parser("receipt")
    rec.add_argument("--out", required=True)
    rec.add_argument("--binary", required=True)
    rec.add_argument("--arm-status", action="append", default=[], help="ARM=status")
    rec.add_argument("--verdict", required=True)
    rec.add_argument("--read", required=True)
    rec.add_argument("--blocker", action="append", default=[])
    args = parser.parse_args(argv)
    if args.command == "run":
        return run_live(args)
    if args.command == "estimate":
        print(json.dumps(estimate(Path(args.out)), indent=1))
        return 0
    if args.command == "summarize":
        print(json.dumps(summarize(Path(args.out)), indent=1))
        return 0
    import hashlib
    from datetime import datetime, timezone

    out = Path(args.out)
    summary = summarize(out)
    summary["gpu"] = gpu_evidence(out / "gpu.csv")
    validation = out / "estimate-10.json"
    if validation.is_file():
        summary["validation_estimate"] = json.loads(validation.read_text(encoding="utf-8"))
    summary["blockers"] = list(args.blocker)
    status = dict(item.split("=", 1) for item in args.arm_status)
    resolved = resolved_config(binary_sha256=hashlib.sha256(Path(args.binary).read_bytes()).hexdigest(),
                               arm_status=status)
    run_id, run_dir = write_receipt(out, summary=summary, resolved=resolved, verdict=args.verdict,
                                    read=args.read, ts=datetime.now(timezone.utc).replace(second=0, microsecond=0))
    print(json.dumps({"run_id": run_id, "run_dir": str(run_dir)}))
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
