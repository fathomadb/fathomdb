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
import os
from collections.abc import Callable, Iterable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


EXPERIMENT = "mcp-agent-loop-locomo-conv0"
SCHEMA_VERSION = "mcp-agent-loop.v1"

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
