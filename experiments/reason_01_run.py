"""Checkpointed REASON-01 held-out retrieval and answer-quality run."""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import math
import os
import random
import statistics
import sys
import time
import urllib.error
import urllib.request
from contextlib import contextmanager
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
from pathlib import Path
from typing import Any, Iterator, Mapping, Sequence

from experiments import reason_01_preflight, reason_01_profile
from experiments.reason_01_equivalence import runtime_attestation


CHECKPOINT_SCHEMA = "reason01.heldout-checkpoint.v1"
ARMS = ("a0", "protected_multiquery_v1")


class Reason01RunError(RuntimeError):
    """A REASON-01 held-out execution or evidence refusal."""


def _atomic_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    temporary.chmod(0o600)
    os.replace(temporary, path)


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _canonical_sha256(value: object) -> str:
    payload = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(payload.encode()).hexdigest()


@dataclass
class RunState:
    """External resumable state; raw prompts and responses never enter Git."""

    schema_version: str
    config_sha256: str
    question_ids: list[str]
    retrievals: dict[str, dict[str, Any]] = field(default_factory=dict)
    answers: dict[str, dict[str, Any]] = field(default_factory=dict)
    judgments: dict[str, dict[str, Any]] = field(default_factory=dict)
    cost_usd: float = 0.0
    amendments: list[dict[str, str]] = field(default_factory=list)


class Checkpoint:
    """Atomically persist every retrieval and paid response."""

    def __init__(self, path: Path, state: RunState) -> None:
        self.path = path
        self.state = state

    @classmethod
    def open(
        cls,
        path: Path,
        *,
        config_sha256: str,
        question_ids: list[str],
        prior_config_sha256: str | None = None,
        amendment: str | None = None,
    ) -> Checkpoint:
        if not path.exists():
            checkpoint = cls(
                path,
                RunState(CHECKPOINT_SCHEMA, config_sha256, question_ids),
            )
            checkpoint.save()
            return checkpoint
        try:
            state = RunState(**json.loads(path.read_text(encoding="utf-8")))
        except (OSError, json.JSONDecodeError, TypeError) as exc:
            raise Reason01RunError("checkpoint is invalid") from exc
        if state.schema_version != CHECKPOINT_SCHEMA or state.question_ids != question_ids:
            raise Reason01RunError("checkpoint does not match this run")
        checkpoint = cls(path, state)
        if state.config_sha256 == config_sha256:
            return checkpoint
        if (
            prior_config_sha256 is None
            or amendment is None
            or not amendment.strip()
            or state.config_sha256 != prior_config_sha256
            or state.amendments
        ):
            raise Reason01RunError("checkpoint does not match this run")
        state.amendments.append(
            {
                "from_config_sha256": state.config_sha256,
                "to_config_sha256": config_sha256,
                "reason": amendment,
            }
        )
        state.config_sha256 = config_sha256
        checkpoint.save()
        return checkpoint

    def save(self) -> None:
        _atomic_json(self.path, asdict(self.state))

    def retrieval(self, arm: str, question_id: str) -> dict[str, Any] | None:
        return self.state.retrievals.get(f"{arm}||{question_id}")

    def put_retrieval(self, arm: str, question_id: str, value: dict[str, Any]) -> None:
        self.state.retrievals[f"{arm}||{question_id}"] = value
        self.save()

    def record_attempt(
        self,
        section: str,
        cell_id: str,
        *,
        model: str,
        content: str,
        prompt_tokens: int,
        completion_tokens: int,
        cost_usd: float,
    ) -> None:
        if section not in {"answers", "judgments"}:
            raise Reason01RunError("unknown checkpoint section")
        cells: dict[str, dict[str, Any]] = getattr(self.state, section)
        cell = cells.setdefault(cell_id, {"attempts": []})
        cell["attempts"].append(
            {
                "model": model,
                "content": content,
                "prompt_tokens": prompt_tokens,
                "completion_tokens": completion_tokens,
                "cost_usd": cost_usd,
            }
        )
        self.state.cost_usd += cost_usd
        self.save()

    def put_result(self, section: str, cell_id: str, result: Mapping[str, object]) -> None:
        cells: dict[str, dict[str, Any]] = getattr(self.state, section)
        cells.setdefault(cell_id, {"attempts": []})["result"] = dict(result)
        self.save()

    def reserve(self, amount: float, cap: float) -> None:
        if not math.isfinite(amount) or amount < 0 or self.state.cost_usd + amount > cap:
            raise Reason01RunError("REASON-01 cost cap would be exceeded before call")


def _json_object(content: str, label: str) -> dict[str, Any]:
    start, end = content.find("{"), content.rfind("}")
    if start < 0 or end < start:
        raise Reason01RunError(f"{label} is not JSON")
    try:
        value = json.loads(content[start : end + 1])
    except json.JSONDecodeError as exc:
        raise Reason01RunError(f"{label} is invalid JSON") from exc
    if not isinstance(value, dict):
        raise Reason01RunError(f"{label} is not an object")
    return value


def parse_answer(content: str, hits: Sequence[Mapping[str, str]]) -> dict[str, Any]:
    """Validate the exact answer/citation response contract."""
    value = _json_object(content, "answer response")
    if set(value) != {"answer", "citations"}:
        raise Reason01RunError("answer response keys drifted")
    answer, citations = value["answer"], value["citations"]
    if not isinstance(answer, str) or not isinstance(citations, list) or any(
        not isinstance(item, str) for item in citations
    ):
        raise Reason01RunError("answer response types drifted")
    known = {hit["logical_id"] for hit in hits}
    if len(set(citations)) != len(citations) or any(item not in known for item in citations):
        raise Reason01RunError("answer citation is not a selected canonical identity")
    if not answer.strip() and citations:
        raise Reason01RunError("empty answer cannot cite memories")
    return {"answer": answer.strip(), "citations": citations}


def parse_judgment(content: str) -> dict[str, bool]:
    """Validate the exact answer-quality judgment contract."""
    value = _json_object(content, "judgment response")
    keys = {"answer_correct", "grounded", "attributed"}
    if set(value) != keys or any(not isinstance(value[key], bool) for key in keys):
        raise Reason01RunError("judgment response contract drifted")
    return {key: value[key] for key in sorted(keys)}


def paired_bootstrap(
    control: Sequence[float],
    treatment: Sequence[float],
    *,
    draws: int,
    seed: int,
) -> dict[str, float]:
    """Return paired point delta and one-sided percentile lower bound."""
    if len(control) != len(treatment) or not control or draws <= 0:
        raise Reason01RunError("paired bootstrap inputs are invalid")
    deltas = [b - a for a, b in zip(control, treatment, strict=True)]
    rng = random.Random(seed)
    samples = [
        statistics.fmean(deltas[rng.randrange(len(deltas))] for _ in deltas)
        for _ in range(draws)
    ]
    samples.sort()
    lower = samples[max(0, math.ceil(0.05 * draws) - 1)]
    return {
        "delta": statistics.fmean(deltas),
        "one_sided_95_lower": lower,
    }


def retry_after_seconds(headers: Mapping[str, str], *, fallback: float) -> float:
    """Honor numeric or HTTP-date Retry-After without shortening it."""
    raw = headers.get("Retry-After") or headers.get("retry-after")
    if raw is None:
        return fallback
    try:
        return max(0.0, float(raw))
    except ValueError:
        try:
            target = parsedate_to_datetime(raw)
            if target.tzinfo is None:
                target = target.replace(tzinfo=timezone.utc)
            return max(0.0, (target - datetime.now(timezone.utc)).total_seconds())
        except (TypeError, ValueError, OverflowError):
            return fallback


@dataclass(frozen=True)
class ModelReply:
    """One model reply with exact usage for caller-side cost accounting."""

    content: str
    prompt_tokens: int
    completion_tokens: int


class AirlockClient:
    """Authenticated loopback client with bounded provider-respecting retries."""

    def __init__(self, base_url: str, key: str, *, attempts: int) -> None:
        if not base_url.startswith(("http://127.0.0.1:", "http://localhost:")):
            raise Reason01RunError("Airlock route must be loopback")
        if not key or attempts < 1:
            raise Reason01RunError("Airlock credentials or retry policy are invalid")
        self.url = f"{base_url.rstrip('/')}/v1"
        self.key = key
        self.attempts = attempts

    def _request(self, path: str, payload: object | None = None) -> dict[str, Any]:
        data = None if payload is None else json.dumps(payload).encode()
        request = urllib.request.Request(
            self.url + path,
            data=data,
            method="GET" if data is None else "POST",
            headers={
                "Authorization": f"Bearer {self.key}",
                "Content-Type": "application/json",
                "X-Airlock-Client": "reason-01",
            },
        )
        for attempt in range(self.attempts):
            try:
                with urllib.request.urlopen(request, timeout=330) as response:  # noqa: S310
                    value = json.load(response)
                if not isinstance(value, dict):
                    raise Reason01RunError("Airlock returned a non-object")
                return value
            except urllib.error.HTTPError as exc:
                if (exc.code != 429 and exc.code < 500) or attempt + 1 == self.attempts:
                    raise Reason01RunError(f"Airlock HTTP {exc.code}") from exc
                time.sleep(
                    retry_after_seconds(
                        dict(exc.headers.items()), fallback=min(60.0, 2.0**attempt)
                    )
                )
            except (TimeoutError, urllib.error.URLError) as exc:
                if attempt + 1 == self.attempts:
                    raise Reason01RunError("Airlock retry budget exhausted") from exc
                time.sleep(min(60.0, 2.0**attempt))
        raise AssertionError("unreachable")

    def models(self) -> set[str]:
        rows = self._request("/models").get("data")
        if not isinstance(rows, list):
            raise Reason01RunError("Airlock model catalog is invalid")
        return {
            row["id"]
            for row in rows
            if isinstance(row, Mapping) and isinstance(row.get("id"), str)
        }

    def complete(
        self,
        model: str,
        messages: Sequence[Mapping[str, str]],
        *,
        max_tokens: int,
    ) -> ModelReply:
        payload: dict[str, object] = {
            "model": model,
            "messages": list(messages),
            "max_tokens": max_tokens,
            "temperature": 0,
            "response_format": {"type": "json_object"},
        }
        if model == "deepseek-v4-pro":
            payload["thinking"] = {"type": "disabled"}
        value = self._request("/chat/completions", payload)
        try:
            content = value["choices"][0]["message"]["content"]
        except (KeyError, IndexError, TypeError) as exc:
            raise Reason01RunError("Airlock response is incomplete") from exc
        usage = value.get("usage", {})
        if not isinstance(content, str) or not isinstance(usage, Mapping):
            raise Reason01RunError("Airlock response lacks content or usage")
        prompt = usage.get("prompt_tokens")
        completion = usage.get("completion_tokens")
        if not isinstance(prompt, int) or not isinstance(completion, int):
            raise Reason01RunError("Airlock usage is invalid")
        return ModelReply(content, prompt, completion)


def _model_cost(model: Mapping[str, Any], reply: ModelReply) -> float:
    return (
        reply.prompt_tokens * float(model["input_per_million"])
        + reply.completion_tokens * float(model["output_per_million"])
    ) / 1_000_000


def _reserve_cost(
    model: Mapping[str, Any], messages: Sequence[Mapping[str, str]], max_tokens: int
) -> float:
    input_tokens = math.ceil(sum(len(row["content"]) for row in messages) / 3)
    return (
        input_tokens * float(model["input_per_million"])
        + max_tokens * float(model["output_per_million"])
    ) / 1_000_000


@contextmanager
def _devices() -> Iterator[None]:
    keys = ("FATHOMDB_EMBED_DEVICE", "FATHOMDB_RERANK_DEVICE", "CUDA_VISIBLE_DEVICES")
    prior = {key: os.environ.get(key) for key in keys}
    os.environ.update(
        {
            "FATHOMDB_EMBED_DEVICE": "cuda:0",
            "FATHOMDB_RERANK_DEVICE": "cuda:0",
            "CUDA_VISIBLE_DEVICES": "0",
        }
    )
    try:
        yield
    finally:
        for key, value in prior.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value


def _hits(result: object) -> list[dict[str, str]]:
    rows = getattr(result, "results", None)
    if not isinstance(rows, Sequence) or getattr(result, "soft_fallback", None) is not None:
        raise Reason01RunError("retrieval result is invalid or used fallback")
    hits: list[dict[str, str]] = []
    for hit in rows:
        logical_id = getattr(getattr(hit, "id", None), "value", None)
        source_id = getattr(hit, "source_id", None)
        body = getattr(hit, "body", None)
        if not all(isinstance(value, str) and value for value in (logical_id, source_id, body)):
            raise Reason01RunError("retrieved context lacks canonical attribution")
        hits.append(
            {
                "logical_id": str(logical_id),
                "source_id": str(source_id),
                "body": str(body),
            }
        )
    return hits


def _session_ids(question_id: str, hits: Sequence[Mapping[str, str]]) -> list[str]:
    prefix = f"longmemeval-{question_id}-"
    sessions: list[str] = []
    for hit in hits:
        source = hit["source_id"]
        if not source.startswith(prefix):
            raise Reason01RunError("retrieved source is outside the held-out case")
        session = source[len(prefix) :]
        if session not in sessions:
            sessions.append(session)
    return sessions


def _retrieve_once(
    engine: object,
    arm: str,
    query: str,
    registry: Mapping[str, object],
    attestation: Mapping[str, object],
) -> tuple[list[dict[str, str]], Mapping[str, object]]:
    if arm == "a0":
        started = time.perf_counter()
        result = engine.search_text_only(query, None, limit=10)  # type: ignore[attr-defined]
        elapsed = (time.perf_counter() - started) * 1000
        if getattr(result, "projection_cursor", None) is None:
            raise Reason01RunError("A0 result lacks projection cursor")
        hits = _hits(result)
        trace = {
            "schema_version": "reason01.a0-safe-trace.v1",
            "profile_id": "a0",
            "query_sha256": hashlib.sha256(f"reason01:query:{query}".encode()).hexdigest(),
            "selected_count": len(hits),
            "elapsed_ms": elapsed,
        }
        return hits, trace
    execution = reason_01_profile.execute_profile(
        engine,  # type: ignore[arg-type]
        query,
        registry,
        intent="relationship",
        runtime_attestation=attestation,
    )
    return _hits(type("Result", (), {"results": execution.hits, "soft_fallback": None})()), execution.trace


def run_retrievals(
    checkpoint: Checkpoint,
    cases: Sequence[Mapping[str, object]],
    databases: Sequence[Mapping[str, object]],
    *,
    registry: Mapping[str, object],
    attestation: Mapping[str, object],
) -> None:
    """Run or resume both arms, recording cold and three steady repetitions."""
    Engine = getattr(importlib.import_module("fathomdb"), "Engine")
    for ordinal, (case, database) in enumerate(zip(cases, databases, strict=True)):
        question_id = str(case["question_id"])
        order = ARMS if ordinal % 2 == 0 else tuple(reversed(ARMS))
        for arm in order:
            if checkpoint.retrieval(arm, question_id) is not None:
                continue
            database_path = Path(str(database["database_path"]))
            if not database_path.is_file():
                raise Reason01RunError(f"preflight database is missing: {database_path}")
            with _devices():
                engine = Engine.open(str(database_path), use_default_embedder=True)
                try:
                    started = time.perf_counter()
                    hits, trace = _retrieve_once(
                        engine, arm, str(case["question"]), registry, attestation
                    )
                    cold_ms = (time.perf_counter() - started) * 1000
                    selected = [hit["logical_id"] for hit in hits]
                    steady_ms: list[float] = []
                    for _ in range(3):
                        started = time.perf_counter()
                        repeated, _trace = _retrieve_once(
                            engine, arm, str(case["question"]), registry, attestation
                        )
                        steady_ms.append((time.perf_counter() - started) * 1000)
                        if [hit["logical_id"] for hit in repeated] != selected:
                            raise Reason01RunError("retrieval selection changed across repetitions")
                finally:
                    engine.close()
            gold = set(case["answer_session_ids"])  # type: ignore[arg-type]
            selected_sessions = set(_session_ids(question_id, hits))
            recovered = len(gold & selected_sessions)
            checkpoint.put_retrieval(
                arm,
                question_id,
                {
                    "hits": hits,
                    "trace": trace,
                    "gold_session_count": len(gold),
                    "recovered_gold_session_count": recovered,
                    "fractional_gold_session_recall": recovered / len(gold),
                    "any_gold_session": recovered > 0,
                    "all_gold_sessions": recovered == len(gold),
                    "cold_ms": cold_ms,
                    "steady_ms": steady_ms,
                },
            )
            print(
                f"REASON-01 retrieval {len(checkpoint.state.retrievals)}/{len(cases) * 2}",
                file=sys.stderr,
                flush=True,
            )


def _answer_messages(
    case: Mapping[str, object], hits: Sequence[Mapping[str, str]]
) -> list[dict[str, str]]:
    context = "\n".join(f"[{hit['logical_id']}] {hit['body']}" for hit in hits)
    return [
        {
            "role": "system",
            "content": (
                "Answer only from the supplied memories. Return exactly one JSON object "
                'with keys "answer" (string) and "citations" (array of memory IDs). '
                "Cite every material claim. If evidence is insufficient, return an empty "
                "answer and empty citations."
            ),
        },
        {
            "role": "user",
            "content": f"Question:\n{case['question']}\n\nMemories:\n{context}",
        },
    ]


def _judge_messages(
    case: Mapping[str, object],
    hits: Sequence[Mapping[str, str]],
    answer: Mapping[str, object],
) -> list[dict[str, str]]:
    context = "\n".join(f"[{hit['logical_id']}] {hit['body']}" for hit in hits)
    return [
        {
            "role": "system",
            "content": (
                "Do not explain your reasoning. Output JSON immediately. Judge independently "
                "of retrieval treatment. Return exactly one JSON object "
                "with boolean keys answer_correct, grounded, and attributed. answer_correct "
                "means semantically correct against the reference; grounded means every "
                "material claim is supported by the supplied memories; attributed means every "
                "material claim is supported by the cited memory IDs."
            ),
        },
        {
            "role": "user",
            "content": (
                f"Question:\n{case['question']}\n\nReference answer:\n{case['answer']}"
                f"\n\nMemories:\n{context}\n\nCandidate answer:\n{answer['answer']}"
                f"\n\nCitations:\n{json.dumps(answer['citations'])}"
                '\n\nReturn only {"answer_correct":true|false,"grounded":true|false,'
                '"attributed":true|false}. The first character must be {.'
            ),
        },
    ]


def _ensure_parsed_cell(
    checkpoint: Checkpoint,
    client: AirlockClient,
    *,
    section: str,
    cell_id: str,
    model: Mapping[str, Any],
    messages: Sequence[Mapping[str, str]],
    max_tokens: int,
    parser: Any,
    parser_args: tuple[object, ...],
    cap: float,
    semantic_attempts: int,
) -> Mapping[str, object]:
    cells: dict[str, dict[str, Any]] = getattr(checkpoint.state, section)
    cell = cells.setdefault(cell_id, {"attempts": []})
    if isinstance(cell.get("result"), Mapping):
        return cell["result"]
    for attempt in cell["attempts"]:
        try:
            result = parser(attempt["content"], *parser_args)
        except Reason01RunError:
            continue
        checkpoint.put_result(section, cell_id, result)
        return result
    while len(cell["attempts"]) < semantic_attempts:
        checkpoint.reserve(_reserve_cost(model, messages, max_tokens), cap)
        reply = client.complete(str(model["model"]), messages, max_tokens=max_tokens)
        checkpoint.record_attempt(
            section,
            cell_id,
            model=str(model["model"]),
            content=reply.content,
            prompt_tokens=reply.prompt_tokens,
            completion_tokens=reply.completion_tokens,
            cost_usd=_model_cost(model, reply),
        )
        cell = cells[cell_id]
        try:
            result = parser(reply.content, *parser_args)
        except Reason01RunError:
            continue
        checkpoint.put_result(section, cell_id, result)
        return result
    raise Reason01RunError(f"{section} semantic retry budget exhausted")


def run_live(
    checkpoint: Checkpoint,
    cases: Sequence[Mapping[str, object]],
    *,
    config: Mapping[str, Any],
    client: AirlockClient,
) -> None:
    """Run or resume paired answer generation and independent judging."""
    required = {config["models"][role]["model"] for role in ("answerer", "judge")}
    if not required <= client.models():
        raise Reason01RunError("Airlock does not expose the registered models")
    total = len(cases) * 2
    for ordinal, case in enumerate(cases):
        question_id = str(case["question_id"])
        order = ARMS if ordinal % 2 == 0 else tuple(reversed(ARMS))
        for arm in order:
            retrieval = checkpoint.retrieval(arm, question_id)
            if retrieval is None:
                raise Reason01RunError("live scoring requires complete retrieval")
            hits = retrieval["hits"]
            cell_id = f"{arm}||{question_id}"
            answer = _ensure_parsed_cell(
                checkpoint,
                client,
                section="answers",
                cell_id=cell_id,
                model=config["models"]["answerer"],
                messages=_answer_messages(case, hits),
                max_tokens=int(config["models"]["answerer"]["max_tokens"]),
                parser=parse_answer,
                parser_args=(hits,),
                cap=float(config["max_usd"]),
                semantic_attempts=int(config["semantic_attempts"]),
            )
            _ensure_parsed_cell(
                checkpoint,
                client,
                section="judgments",
                cell_id=cell_id,
                model=config["models"]["judge"],
                messages=_judge_messages(case, hits, answer),
                max_tokens=int(config["models"]["judge"]["max_tokens"]),
                parser=parse_judgment,
                parser_args=(),
                cap=float(config["max_usd"]),
                semantic_attempts=int(config["semantic_attempts"]),
            )
            complete = sum(
                isinstance(value.get("result"), Mapping)
                for value in checkpoint.state.judgments.values()
            )
            print(f"REASON-01 scored {complete}/{total}", file=sys.stderr, flush=True)


def _p95(values: Sequence[float]) -> float:
    ordered = sorted(values)
    return ordered[math.ceil(0.95 * len(ordered)) - 1]


def summarize(
    checkpoint: Checkpoint,
    cases: Sequence[Mapping[str, object]],
    *,
    config: Mapping[str, Any],
) -> dict[str, Any]:
    """Build the content-free decision summary and apply the frozen boundaries."""
    retrieval_complete = len(checkpoint.state.retrievals) == len(cases) * 2
    live_complete = all(
        isinstance(checkpoint.state.judgments.get(f"{arm}||{case['question_id']}", {}).get("result"), Mapping)
        for case in cases
        for arm in ARMS
    )
    summary: dict[str, Any] = {
        "schema_version": "reason01.heldout-summary.v1",
        "question_count": len(cases),
        "retrieval_complete": retrieval_complete,
        "live_complete": live_complete,
        "cost_usd": checkpoint.state.cost_usd,
        "decision_eligible": False,
    }
    if not retrieval_complete:
        return summary
    by_arm: dict[str, Any] = {}
    for arm in ARMS:
        rows = [checkpoint.retrieval(arm, str(case["question_id"])) for case in cases]
        assert all(row is not None for row in rows)
        by_arm[arm] = {
            "fractional_gold_session_recall": statistics.fmean(
                float(row["fractional_gold_session_recall"]) for row in rows if row
            ),
            "any_gold_session_rate": statistics.fmean(
                float(row["any_gold_session"]) for row in rows if row
            ),
            "all_gold_sessions_rate": statistics.fmean(
                float(row["all_gold_sessions"]) for row in rows if row
            ),
            "cold_p95_ms": _p95([float(row["cold_ms"]) for row in rows if row]),
            "steady_p95_ms": _p95(
                [value for row in rows if row for value in map(float, row["steady_ms"])]
            ),
            "context_max": max(len(row["hits"]) for row in rows if row),
        }
    summary["arms"] = by_arm
    recall = paired_bootstrap(
        [float(checkpoint.retrieval("a0", str(case["question_id"]))["fractional_gold_session_recall"]) for case in cases],  # type: ignore[index]
        [float(checkpoint.retrieval("protected_multiquery_v1", str(case["question_id"]))["fractional_gold_session_recall"]) for case in cases],  # type: ignore[index]
        draws=int(config["bootstrap_draws"]),
        seed=int(config["bootstrap_seed"]),
    )
    summary["paired_retrieval"] = recall
    if not live_complete:
        return summary
    for arm in ARMS:
        judgments = [
            checkpoint.state.judgments[f"{arm}||{case['question_id']}"]["result"]
            for case in cases
        ]
        by_arm[arm].update(
            {
                "answer_accuracy": statistics.fmean(float(row["answer_correct"]) for row in judgments),
                "grounded_rate": statistics.fmean(float(row["grounded"]) for row in judgments),
                "attribution_rate": statistics.fmean(float(row["attributed"]) for row in judgments),
                "citation_contract_validity": 1.0,
            }
        )
    answer = paired_bootstrap(
        [float(checkpoint.state.judgments[f"a0||{case['question_id']}"]["result"]["answer_correct"]) for case in cases],
        [float(checkpoint.state.judgments[f"protected_multiquery_v1||{case['question_id']}"]["result"]["answer_correct"]) for case in cases],
        draws=int(config["bootstrap_draws"]),
        seed=int(config["bootstrap_seed"]),
    )
    summary["paired_answer"] = answer
    control, treatment = by_arm["a0"], by_arm["protected_multiquery_v1"]
    eligible = (
        recall["one_sided_95_lower"] >= 0
        and answer["one_sided_95_lower"] >= 0
        and treatment["grounded_rate"] >= control["grounded_rate"]
        and treatment["attribution_rate"] >= control["attribution_rate"]
        and treatment["citation_contract_validity"] == 1.0
        and treatment["context_max"] <= 20
        and treatment["cold_p95_ms"] <= 100
        and treatment["steady_p95_ms"] <= 75
        and checkpoint.state.cost_usd <= float(config["max_usd"])
    )
    summary.update({"decision_eligible": True, "decision": "eligible" if eligible else "ineligible"})
    return summary


def _load_json(path: Path, label: str) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise Reason01RunError(f"{label} is unavailable") from exc


def _load_config(path: Path) -> Mapping[str, Any]:
    value = _load_json(path, "run config")
    if not isinstance(value, Mapping) or value.get("schema_version") not in {
        "reason01.heldout-run.v1",
        "reason01.heldout-run.v2",
    }:
        raise Reason01RunError("run config drifted")
    if value.get("arms") != list(ARMS) or value.get("question_count") != 109:
        raise Reason01RunError("run cohort or arms drifted")
    expected_semantic_attempts = 5 if value["schema_version"].endswith(".v2") else 3
    if (
        value.get("max_usd") != 5.0
        or value.get("semantic_attempts") != expected_semantic_attempts
    ):
        raise Reason01RunError("run spend or retry boundary drifted")
    if value["schema_version"].endswith(".v2"):
        recovery = value.get("recovery")
        if (
            not isinstance(recovery, Mapping)
            or recovery.get("prior_binding_sha256")
            != "070f171cd2fe0eb9be6fa0d7aa24ff4e359a526245974441b3f17e439155ece8"
            or recovery.get("reason") != "judge JSON-shape transport correction"
        ):
            raise Reason01RunError("run recovery binding drifted")
    return value


def _load_bound_inputs(args: argparse.Namespace) -> tuple[list[Mapping[str, object]], list[Mapping[str, object]]]:
    source = _load_json(args.source, "LongMemEval source")
    prior_ids = reason_01_preflight._prior_ids(args.prior_receipt)
    cases = list(reason_01_preflight.select_heldout_cases(source, prior_ids, expected_count=109))
    manifest = _load_json(args.manifest, "held-out manifest")
    ids = [str(case["question_id"]) for case in cases]
    if manifest.get("question_ids") != ids or manifest.get("source_sha256") != _sha256(args.source):
        raise Reason01RunError("held-out manifest binding drifted")
    environment = _load_json(args.environment, "environment receipt")
    databases = environment.get("databases")
    if not isinstance(databases, list) or len(databases) != len(cases):
        raise Reason01RunError("environment database binding drifted")
    for case, database in zip(cases, databases, strict=True):
        expected = hashlib.sha256(f"reason01:question:{case['question_id']}".encode()).hexdigest()
        if not isinstance(database, Mapping) or database.get("question_id_sha256") != expected:
            raise Reason01RunError("environment question binding drifted")
    return cases, databases


def main(argv: Sequence[str] | None = None) -> int:
    """Run or resume the authorized held-out measurement."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--prior-receipt", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--environment", type=Path, required=True)
    parser.add_argument("--registry", type=Path, required=True)
    parser.add_argument("--artifact-root", type=Path, required=True)
    parser.add_argument("--fathomdb-bin", type=Path, required=True)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--airlock-base", default="http://127.0.0.1:4000")
    parser.add_argument("--phase", choices=("retrieval", "live", "all"), default="all")
    args = parser.parse_args(argv)

    config = _load_config(args.config)
    cases, databases = _load_bound_inputs(args)
    binding = {
        "config": _sha256(args.config),
        "manifest": _sha256(args.manifest),
        "environment": _sha256(args.environment),
        "registry": _sha256(args.registry),
        "runner": _sha256(Path(__file__)),
    }
    args.artifact_root.mkdir(parents=True, exist_ok=True)
    recovery = config.get("recovery")
    checkpoint = Checkpoint.open(
        args.artifact_root / "reason01-heldout-checkpoint.v1.json",
        config_sha256=_canonical_sha256(binding),
        question_ids=[str(case["question_id"]) for case in cases],
        prior_config_sha256=(
            str(recovery["prior_binding_sha256"])
            if isinstance(recovery, Mapping)
            else None
        ),
        amendment=(str(recovery["reason"]) if isinstance(recovery, Mapping) else None),
    )
    registry = reason_01_profile.load_registry(args.registry)
    attestation = runtime_attestation(
        registry=registry, fathomdb_bin=args.fathomdb_bin, repo_root=args.repo_root
    )
    if args.phase in {"retrieval", "all"}:
        run_retrievals(
            checkpoint,
            cases,
            databases,
            registry=registry,
            attestation=attestation,
        )
    if args.phase in {"live", "all"}:
        if len(checkpoint.state.retrievals) != len(cases) * 2:
            raise Reason01RunError("live phase requires complete retrieval")
        key = os.environ.get("AIRLOCK_VIRTUAL_KEY") or os.environ.get("AIRLOCK_MASTER_KEY")
        if not key:
            raise Reason01RunError("AIRLOCK_VIRTUAL_KEY or AIRLOCK_MASTER_KEY is required")
        run_live(
            checkpoint,
            cases,
            config=config,
            client=AirlockClient(args.airlock_base, key, attempts=int(config["http_attempts"])),
        )
    summary = summarize(checkpoint, cases, config=config)
    _atomic_json(args.artifact_root / "reason01-heldout-summary.v1.json", summary)
    _atomic_json(
        args.artifact_root / "reason01-heldout-receipt.v1.json",
        {
            "schema_version": "reason01.heldout-receipt.v1",
            "binding": binding,
            "models": config["models"],
            "runtime": attestation,
            "checkpoint_sha256": _sha256(checkpoint.path),
            "summary": summary,
        },
    )
    print(json.dumps(summary, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
