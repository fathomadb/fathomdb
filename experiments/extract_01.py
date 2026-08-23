"""EXTRACT-01 question-blind native-ELPS comparison on LongMemEval updates."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import sqlite3
import sys
import time
import urllib.error
import urllib.request
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable, Mapping, Sequence

from experiments import _lib
from experiments.fathomdb_test_setup import prepare_test_database


SCHEMA = "extract-01.knowledge-update.v1"
CHECKPOINT_SCHEMA = "extract-01.checkpoint.v1"
PROGRAM_TRACK = "EXTRACT-01"
ARM_IDS = ("raw_fts", "raw_plus_elps_fts")
REPLAY = Path(__file__).with_name("extract_01_replay_harness.py")
MEASUREMENT_REVISION = "sqlite-logical-pages.v1"


class Extract01Error(ValueError):
    """Raised when EXTRACT-01 cannot preserve its fixed comparison contract."""


@dataclass(frozen=True)
class Case:
    question_id: str
    question: str
    answer: str
    question_date: str
    session_ids: tuple[str, ...]
    dates: tuple[str, ...]
    sessions: tuple[tuple[dict[str, str], ...], ...]
    answer_session_ids: frozenset[str]


@dataclass(frozen=True)
class Reply:
    content: str
    prompt_tokens: int
    completion_tokens: int


@dataclass
class RunState:
    schema_version: str
    config_sha256: str
    question_ids: list[str]
    cells: dict[str, Any] = field(default_factory=dict)
    charges: dict[str, float] = field(default_factory=dict)
    cost_usd: float = 0.0

    @classmethod
    def new(cls, config_sha256: str, question_ids: list[str]) -> RunState:
        return cls(CHECKPOINT_SCHEMA, config_sha256, question_ids)


def _exact(value: object, label: str, keys: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        actual = set(value) if isinstance(value, dict) else set()
        raise Extract01Error(
            f"{label} keys mismatch: missing={sorted(keys - actual)}, "
            f"unknown={sorted(actual - keys)}"
        )
    return value


def resolve_config(value: object) -> dict[str, Any]:
    """Strictly resolve the committed EXTRACT-01 configuration."""
    root = _exact(
        value,
        "config",
        {
            "schema_version",
            "program_track",
            "corpus",
            "oracle",
            "scope",
            "profile",
            "arms",
            "extraction",
            "live",
            "metrics",
            "claim_boundary",
        },
    )
    if root["schema_version"] != SCHEMA or root["program_track"] != PROGRAM_TRACK:
        raise Extract01Error("EXTRACT-01 schema or program track drifted")
    corpus = _exact(root["corpus"], "corpus", {"identifier", "sha256", "storage"})
    oracle = _exact(
        root["oracle"], "oracle", {"identifier", "sha256", "storage", "use"}
    )
    for label, ref in (("corpus", corpus), ("oracle", oracle)):
        if ref["storage"] != "external_only" or not re.fullmatch(
            r"[0-9a-f]{64}", str(ref["sha256"])
        ):
            raise Extract01Error(f"{label} reference drifted")
    if oracle["use"] != "evaluation_only":
        raise Extract01Error("oracle may be used only for evaluation")
    if root["scope"] != {"question_type": "knowledge-update", "question_count": 78}:
        raise Extract01Error("EXTRACT-01 scope drifted")
    expected_profile = {
        "id": "a0_turn_fts_stream_default",
        "retrieval": "fts",
        "top_k": 10,
        "embedder": "none",
        "embed_device": "cpu",
        "reranker": "none",
        "rerank_device": "cpu",
    }
    if root["profile"] != expected_profile:
        raise Extract01Error("EXTRACT-01 profile drifted")
    if root["arms"] != [
        {"id": "raw_fts", "content": "canonical_turns"},
        {"id": "raw_plus_elps_fts", "content": "canonical_turns_plus_native_elps"},
    ]:
        raise Extract01Error("EXTRACT-01 arms drifted")
    extraction = _exact(
        root["extraction"],
        "extraction",
        {"protocol", "question_blind", "max_document_chars", "max_facts_per_document"},
    )
    if (
        extraction["protocol"] != "fathomdb.extract.v1"
        or extraction["question_blind"] is not True
        or extraction["max_document_chars"] != 120000
        or extraction["max_facts_per_document"] != 24
    ):
        raise Extract01Error("EXTRACT-01 extraction contract drifted")
    live = _exact(
        root["live"],
        "live",
        {
            "extractor",
            "answerer",
            "judge",
            "max_usd",
            "max_retries",
            "max_workers",
            "checkpoint_every",
        },
    )
    expected_models = {
        "extractor": ("gemini-3.1-flash-lite", 0.25, 1.5, 4096, "none", 8.5),
        "answerer": ("gemini-3.1-flash-lite", 0.25, 1.5, 256, "none", 8.5),
        "judge": ("claude-sonnet", 3.0, 15.0, 10, "none", 0.0),
    }
    for role, expected in expected_models.items():
        model = _exact(
            live[role],
            role,
            {
                "model",
                "input_usd_per_million",
                "output_usd_per_million",
                "max_tokens",
                "reasoning_effort",
                "min_call_interval_seconds",
            },
        )
        actual = (
            model["model"],
            model["input_usd_per_million"],
            model["output_usd_per_million"],
            model["max_tokens"],
            model["reasoning_effort"],
            model["min_call_interval_seconds"],
        )
        if actual != expected:
            raise Extract01Error(f"EXTRACT-01 {role} contract drifted")
    if (
        live["max_usd"] != 20.0
        or live["max_retries"] != 2
        or live["max_workers"] != 1
        or live["checkpoint_every"] != 1
    ):
        raise Extract01Error("EXTRACT-01 resilience or cost boundary drifted")
    boundary = _exact(
        root["claim_boundary"],
        "claim boundary",
        {"quality", "extraction_precision", "lifecycle", "excluded"},
    )
    if boundary["extraction_precision"] != "unscored_no_human_gold":
        raise Extract01Error("EXTRACT-01 may not manufacture extraction gold")
    return root


def load_config(path: Path) -> dict[str, Any]:
    return resolve_config(json.loads(path.read_text(encoding="utf-8")))


def _load_external(path: Path, expected_sha: str, label: str) -> object:
    payload = path.read_bytes()
    if hashlib.sha256(payload).hexdigest() != expected_sha:
        raise Extract01Error(f"{label} sha256 drifted")
    try:
        return json.loads(payload)
    except json.JSONDecodeError as exc:
        raise Extract01Error(f"{label} is invalid JSON") from exc


def case_from_row(row: object) -> Case:
    if not isinstance(row, Mapping) or row.get("question_type") != "knowledge-update":
        raise Extract01Error("LongMemEval row is not knowledge-update")
    required = ("question_id", "question", "question_date")
    if any(not isinstance(row.get(key), str) or not row[key] for key in required):
        raise Extract01Error("LongMemEval row lacks question fields")
    answer = row.get("answer")
    if answer is None or isinstance(answer, bool) or not str(answer).strip():
        raise Extract01Error("LongMemEval row lacks a reference answer")
    ids, dates, sessions = (
        row.get("haystack_session_ids"),
        row.get("haystack_dates"),
        row.get("haystack_sessions"),
    )
    if (
        not isinstance(ids, list)
        or not isinstance(dates, list)
        or not isinstance(sessions, list)
    ):
        raise Extract01Error("LongMemEval history is invalid")
    if not ids or len(ids) != len(dates) or len(ids) != len(sessions):
        raise Extract01Error("LongMemEval history axes do not align")
    clean_sessions: list[tuple[dict[str, str], ...]] = []
    for session in sessions:
        if not isinstance(session, list) or not session:
            raise Extract01Error("LongMemEval session is invalid")
        clean_turns: list[dict[str, str]] = []
        for turn in session:
            if not isinstance(turn, Mapping):
                raise Extract01Error("LongMemEval turn is invalid")
            role, content = turn.get("role"), turn.get("content")
            if role not in {"user", "assistant"} or not isinstance(content, str):
                raise Extract01Error("LongMemEval turn fields are invalid")
            clean_turns.append({"role": role, "content": content})
        clean_sessions.append(tuple(clean_turns))
    answer_ids = row.get("answer_session_ids")
    if not isinstance(answer_ids, list) or any(
        not isinstance(item, str) for item in answer_ids
    ):
        raise Extract01Error("LongMemEval answer-session labels are invalid")
    return Case(
        str(row["question_id"]),
        str(row["question"]),
        str(answer),
        str(row["question_date"]),
        tuple(map(str, ids)),
        tuple(map(str, dates)),
        tuple(clean_sessions),
        frozenset(answer_ids),
    )


def load_cases(corpus: object, expected_count: int = 78) -> list[Case]:
    if not isinstance(corpus, list):
        raise Extract01Error("LongMemEval corpus must be a list")
    cases = [
        case_from_row(row)
        for row in corpus
        if isinstance(row, Mapping) and row.get("question_type") == "knowledge-update"
    ]
    if len(cases) != expected_count or len({case.question_id for case in cases}) != len(
        cases
    ):
        raise Extract01Error("LongMemEval knowledge-update count or identity drifted")
    return cases


def _session_text(
    session_id: str, date: str, turns: Sequence[Mapping[str, str]]
) -> str:
    lines = [f"SESSION {session_id}", f"DATE {date}"]
    lines.extend(f"{turn['role'].upper()}: {turn['content']}" for turn in turns)
    return "\n".join(lines)


def extraction_documents(case: Case, max_chars: int) -> list[dict[str, Any]]:
    """Build chronological, question-blind documents without any gold labels."""
    blocks = [
        (_session_text(session_id, date, session), session_id)
        for session_id, date, session in zip(
            case.session_ids, case.dates, case.sessions, strict=True
        )
    ]
    documents: list[dict[str, Any]] = []
    texts: list[str] = []
    session_ids: list[str] = []
    size = 0
    for text, session_id in blocks:
        if texts and size + len(text) + 2 > max_chars:
            documents.append(
                {
                    "source_doc_id": f"{case.question_id}:chunk-{len(documents):03d}",
                    "body": "\n\n".join(texts),
                    "session_ids": list(session_ids),
                }
            )
            texts, session_ids, size = [], [], 0
        if len(text) > max_chars:
            raise Extract01Error(
                "one LongMemEval session exceeds extraction chunk boundary"
            )
        texts.append(text)
        session_ids.append(session_id)
        size += len(text) + 2
    if texts:
        documents.append(
            {
                "source_doc_id": f"{case.question_id}:chunk-{len(documents):03d}",
                "body": "\n\n".join(texts),
                "session_ids": list(session_ids),
            }
        )
    return documents


def raw_rows(case: Case) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    for session_id, date, session in zip(
        case.session_ids, case.dates, case.sessions, strict=True
    ):
        for ordinal, turn in enumerate(session):
            rows.append(
                {
                    "kind": "longmemeval_turn",
                    "body": f"{date} {turn['role']}: {turn['content']}",
                    "source_id": f"{case.question_id}:{session_id}",
                    "logical_id": f"{case.question_id}:{session_id}:turn-{ordinal:03d}",
                }
            )
    return rows


def validate_elps_result(value: object, allowed_sources: set[str]) -> dict[str, Any]:
    result = _exact(value, "ELPS result", {"entities", "edges", "warnings"})
    if any(not isinstance(result[key], list) for key in result):
        raise Extract01Error("ELPS result arrays are invalid")
    names: set[str] = set()
    for entity in result["entities"]:
        entity = _exact(
            entity, "ELPS entity", {"name", "type", "aliases", "source_doc_id"}
        )
        if (
            not isinstance(entity["name"], str)
            or not entity["name"].strip()
            or not isinstance(entity["type"], str)
            or not entity["type"].strip()
            or ":" in entity["type"]
            or not isinstance(entity["aliases"], list)
            or any(not isinstance(alias, str) for alias in entity["aliases"])
            or entity["source_doc_id"] not in allowed_sources
        ):
            raise Extract01Error("ELPS entity source_doc_id or fields are invalid")
        names.add(entity["name"])
    edge_keys = {
        "from_entity",
        "to_entity",
        "relation",
        "body",
        "t_valid",
        "t_invalid",
        "confidence",
        "source_doc_id",
        "source_span",
    }
    for edge in result["edges"]:
        edge = _exact(edge, "ELPS edge", edge_keys)
        confidence = edge["confidence"]
        if (
            edge["source_doc_id"] not in allowed_sources
            or edge["from_entity"] not in names
            or edge["to_entity"] not in names
            or not isinstance(edge["relation"], str)
            or not edge["relation"]
            or not isinstance(edge["body"], str)
            or not edge["body"]
            or not isinstance(confidence, (int, float))
            or isinstance(confidence, bool)
            or not 0.0 <= float(confidence) <= 1.0
        ):
            raise Extract01Error(
                "ELPS edge source_doc_id, endpoint, or confidence is invalid"
            )
    return result


def _calendar_iso(value: object) -> str | None:
    if not isinstance(value, str) or value.strip().lower() in {
        "",
        "null",
        "none",
        "unknown",
    }:
        return None
    candidate = value.strip()
    try:
        datetime.fromisoformat(candidate.replace("Z", "+00:00"))
    except ValueError:
        return None
    return candidate


def normalize_elps_result(value: object, source_doc_id: str) -> dict[str, Any]:
    """Repair provider JSON at the single-document attribution boundary."""
    if not isinstance(value, Mapping):
        raise Extract01Error("ELPS provider returned a non-object")
    raw_entities = value.get("entities", [])
    raw_edges = value.get("edges", [])
    if not isinstance(raw_entities, list) or not isinstance(raw_edges, list):
        raise Extract01Error("ELPS provider arrays are invalid")
    entities: list[dict[str, Any]] = []
    names: set[str] = set()
    for raw in raw_entities:
        if not isinstance(raw, Mapping):
            continue
        name = str(raw.get("name", "")).strip()
        if not name or name in names:
            continue
        kind = (
            re.sub(r"[^A-Za-z0-9_-]+", "_", str(raw.get("type", "entity"))).strip("_")
            or "entity"
        )
        aliases = raw.get("aliases")
        clean_aliases = (
            [alias for alias in aliases if isinstance(alias, str)]
            if isinstance(aliases, list)
            else []
        )
        entities.append(
            {
                "name": name,
                "type": kind,
                "aliases": clean_aliases,
                "source_doc_id": source_doc_id,
            }
        )
        names.add(name)
    edges: list[dict[str, Any]] = []
    for raw in raw_edges:
        if not isinstance(raw, Mapping):
            continue
        source, target = raw.get("from_entity"), raw.get("to_entity")
        body = raw.get("body")
        if (
            not isinstance(source, str)
            or not isinstance(target, str)
            or source not in names
            or target not in names
            or not isinstance(body, str)
            or not body
        ):
            continue
        relation = (
            re.sub(
                r"[^a-z0-9_-]+", "_", str(raw.get("relation", "related_to")).lower()
            ).strip("_")
            or "related_to"
        )
        confidence = raw.get("confidence", 0.5)
        if not isinstance(confidence, (int, float)) or isinstance(confidence, bool):
            confidence = 0.5
        span = raw.get("source_span")
        if not (
            isinstance(span, list)
            and len(span) == 2
            and all(
                isinstance(item, int) and not isinstance(item, bool) for item in span
            )
        ):
            span = None
        edges.append(
            {
                "from_entity": source,
                "to_entity": target,
                "relation": relation,
                "body": body,
                "t_valid": _calendar_iso(raw.get("t_valid")),
                "t_invalid": _calendar_iso(raw.get("t_invalid")),
                "confidence": max(0.0, min(1.0, float(confidence))),
                "source_doc_id": source_doc_id,
                "source_span": span,
            }
        )
    result = {"entities": entities, "edges": edges, "warnings": []}
    return validate_elps_result(result, {source_doc_id})


def _atomic_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(
        json.dumps(value, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8"
    )
    temporary.chmod(0o600)
    temporary.replace(path)


class Checkpoint:
    def __init__(self, path: Path, state: RunState) -> None:
        self.path = path
        self.state = state

    @classmethod
    def open(
        cls, path: Path, config_sha256: str, question_ids: list[str]
    ) -> Checkpoint:
        if not path.exists():
            checkpoint = cls(path, RunState.new(config_sha256, question_ids))
            checkpoint.save()
            return checkpoint
        try:
            value = json.loads(path.read_text(encoding="utf-8"))
            state = RunState(**value)
        except (OSError, json.JSONDecodeError, TypeError) as exc:
            raise Extract01Error("EXTRACT-01 checkpoint is invalid") from exc
        if (
            state.schema_version != CHECKPOINT_SCHEMA
            or state.config_sha256 != config_sha256
            or state.question_ids != question_ids
        ):
            raise Extract01Error("EXTRACT-01 checkpoint does not match this run")
        return cls(path, state)

    def save(self) -> None:
        _atomic_json(self.path, asdict(self.state))

    def put(self, cell_id: str, value: object) -> None:
        self.state.cells[cell_id] = value
        self.save()

    def reserve(
        self,
        model: Mapping[str, Any],
        messages: Sequence[Mapping[str, str]],
        max_tokens: int,
        *,
        max_usd: float,
    ) -> None:
        input_tokens = math.ceil(sum(len(item["content"]) for item in messages) / 3)
        reserve = (
            input_tokens * float(model["input_usd_per_million"])
            + max_tokens * float(model["output_usd_per_million"])
        ) / 1_000_000
        if self.state.cost_usd + reserve > max_usd:
            raise Extract01Error("EXTRACT-01 cost cap would be exceeded before call")

    def charge(
        self,
        cell_id: str,
        model: Mapping[str, Any],
        *,
        prompt_tokens: int,
        completion_tokens: int,
    ) -> float:
        if prompt_tokens < 0 or completion_tokens < 0:
            raise Extract01Error("model usage cannot be negative")
        cost = (
            prompt_tokens * float(model["input_usd_per_million"])
            + completion_tokens * float(model["output_usd_per_million"])
        ) / 1_000_000
        suffix = 1
        key = cell_id
        while key in self.state.charges:
            suffix += 1
            key = f"{cell_id}:attempt-{suffix}"
        self.state.charges[key] = cost
        self.state.cost_usd = sum(self.state.charges.values())
        self.save()
        return cost


def complete_once(
    checkpoint: Checkpoint, cell_id: str, complete: Callable[[], Any]
) -> Any:
    if cell_id in checkpoint.state.cells:
        return checkpoint.state.cells[cell_id]
    value = complete()
    checkpoint.put(cell_id, value)
    return value


def logical_database_bytes(connection: sqlite3.Connection) -> int:
    """Return committed logical SQLite size, including pages visible through WAL."""
    page_count = int(connection.execute("PRAGMA page_count").fetchone()[0])
    page_size = int(connection.execute("PRAGMA page_size").fetchone()[0])
    return page_count * page_size


def case_measurement_is_current(value: object) -> bool:
    """Return whether a case carries the current local measurement revision."""
    return (
        isinstance(value, Mapping)
        and value.get("measurement_revision") == MEASUREMENT_REVISION
    )


def semantic_retry(
    max_retries: int, call: Callable[[], Any], parse: Callable[[Any], Any]
) -> Any:
    """Retry malformed model content without hiding or exceeding the fixed bound."""
    last_error: Exception | None = None
    for _attempt in range(max_retries + 1):
        content = call()
        try:
            return parse(content)
        except (Extract01Error, json.JSONDecodeError, ValueError) as exc:
            last_error = exc
    raise Extract01Error("model semantic retries exhausted") from last_error


class AirlockClient:
    """Minimal authenticated client with bounded retries for the local Airlock."""

    def __init__(self, base_url: str, key: str, max_retries: int) -> None:
        if not base_url.startswith(("http://127.0.0.1:", "http://localhost:")):
            raise Extract01Error("Airlock must use an authenticated loopback route")
        if not key:
            raise Extract01Error(
                "AIRLOCK_VIRTUAL_KEY or AIRLOCK_MASTER_KEY is required"
            )
        self.base_url = base_url.rstrip("/")
        self.key = key
        self.max_retries = max_retries
        self._last_call_at: dict[str, float] = {}

    def _pace(self, model: str, minimum_interval_seconds: float) -> None:
        now = time.monotonic()
        previous = self._last_call_at.get(model)
        if previous is not None:
            delay = minimum_interval_seconds - (now - previous)
            if delay > 0:
                time.sleep(delay)
                now += delay
        self._last_call_at[model] = now

    def _request(self, path: str, payload: object | None = None) -> dict[str, Any]:
        data = None if payload is None else json.dumps(payload).encode("utf-8")
        request = urllib.request.Request(
            self.base_url + path,
            data=data,
            method="GET" if data is None else "POST",
            headers={
                "Authorization": f"Bearer {self.key}",
                "Content-Type": "application/json",
            },
        )
        for attempt in range(self.max_retries + 1):
            try:
                with urllib.request.urlopen(request, timeout=300) as response:  # noqa: S310
                    value = json.loads(response.read())
                if not isinstance(value, dict):
                    raise Extract01Error("Airlock returned a non-object")
                return value
            except urllib.error.HTTPError as exc:
                detail = exc.read().decode("utf-8", errors="replace")[:500]
                if exc.code == 429 and "Airlock temporarily blocked" in detail:
                    raise Extract01Error(f"Airlock HTTP {exc.code}: {detail}") from exc
                retryable = exc.code == 429 or 500 <= exc.code < 600
                if not retryable or attempt == self.max_retries:
                    raise Extract01Error(f"Airlock HTTP {exc.code}: {detail}") from exc
            except (urllib.error.URLError, TimeoutError) as exc:
                if attempt == self.max_retries:
                    raise Extract01Error("Airlock route is unavailable") from exc
            time.sleep(min(2**attempt, 30))
        raise AssertionError("unreachable Airlock retry loop")

    def models(self) -> set[str]:
        data = self._request("/v1/models").get("data")
        if not isinstance(data, list):
            raise Extract01Error("Airlock model catalog is invalid")
        return {
            item["id"]
            for item in data
            if isinstance(item, Mapping) and isinstance(item.get("id"), str)
        }

    def complete(
        self,
        model: str,
        messages: Sequence[Mapping[str, str]],
        max_tokens: int,
        *,
        json_mode: bool,
        reasoning_effort: str,
        min_call_interval_seconds: float,
    ) -> Reply:
        self._pace(model, min_call_interval_seconds)
        payload: dict[str, Any] = {
            "model": model,
            "messages": list(messages),
            "max_tokens": max_tokens,
        }
        if not model.startswith("claude-"):
            payload["reasoning_effort"] = reasoning_effort
        if json_mode:
            payload["response_format"] = {"type": "json_object"}
        value = self._request("/v1/chat/completions", payload)
        try:
            content = value["choices"][0]["message"]["content"]
        except (KeyError, IndexError, TypeError) as exc:
            raise Extract01Error("Airlock completion is incomplete") from exc
        if not isinstance(content, str):
            raise Extract01Error("Airlock completion content is not text")
        usage = value.get("usage") if isinstance(value.get("usage"), Mapping) else {}
        estimate = math.ceil(sum(len(item["content"]) for item in messages) / 3)
        prompt_tokens = usage.get("prompt_tokens", estimate)
        completion_tokens = usage.get("completion_tokens", max_tokens)
        if not isinstance(prompt_tokens, int) or not isinstance(completion_tokens, int):
            raise Extract01Error("Airlock usage is invalid")
        return Reply(content, prompt_tokens, completion_tokens)


def _extract_messages(
    document: Mapping[str, Any], max_facts: int
) -> list[dict[str, str]]:
    system = (
        "Extract durable user-memory facts from chronological chat history. Be question-blind: "
        "no question or gold answer is provided. Emit only facts explicitly supported by the "
        "document. Prefer changes to personal information, current state, and dated events. "
        f"Return at most {max_facts} directed atomic facts. Every entity and edge must copy the "
        "exact source_doc_id. Edge body must be a short supporting source statement; confidence "
        "is in [0,1]. Use ISO-8601 UTC t_valid when known and null otherwise. Return JSON only "
        "with keys entities, edges, warnings. Entity keys: name,type,aliases,source_doc_id. "
        "Edge keys: from_entity,to_entity,relation,body,t_valid,t_invalid,confidence,"
        "source_doc_id,source_span. Use warnings=[] and source_span=null."
    )
    return [
        {"role": "system", "content": system},
        {
            "role": "user",
            "content": f"source_doc_id: {document['source_doc_id']}\n\n{document['body']}",
        },
    ]


def _answer_messages(
    case: Case, hits: Sequence[Mapping[str, str]]
) -> list[dict[str, str]]:
    context = "\n\n".join(
        f"[{hit['id']}] source={hit['source_id']}\n{hit['body']}" for hit in hits
    )
    return [
        {
            "role": "system",
            "content": "Answer using only the supplied memory. Be concise. Return JSON with exactly answer and cited_ids.",
        },
        {
            "role": "user",
            "content": f"Question date: {case.question_date}\nQuestion: {case.question}\n\nMemory:\n{context}",
        },
    ]


def _judge_messages(case: Case, hypothesis: str) -> list[dict[str, str]]:
    prompt = (
        "I will give you a question, a correct answer, and a response from a model. "
        "Please answer yes if the response contains the correct answer. Otherwise, answer no. "
        "If the response contains some previous information along with an updated answer, the "
        "response should be considered as correct as long as the updated answer is the required "
        f"answer.\n\nQuestion: {case.question}\n\nCorrect Answer: {case.answer}"
        f"\n\nModel Response: {hypothesis}\n\nIs the model response correct? Answer yes or no only."
    )
    return [{"role": "user", "content": prompt}]


def _model_call(
    checkpoint: Checkpoint,
    client: AirlockClient,
    cell_id: str,
    model: Mapping[str, Any],
    messages: Sequence[Mapping[str, str]],
    *,
    json_mode: bool,
) -> str:
    checkpoint.reserve(model, messages, int(model["max_tokens"]), max_usd=20.0)
    reply = client.complete(
        str(model["model"]),
        messages,
        int(model["max_tokens"]),
        json_mode=json_mode,
        reasoning_effort=str(model["reasoning_effort"]),
        min_call_interval_seconds=float(model["min_call_interval_seconds"]),
    )
    checkpoint.charge(
        cell_id,
        model,
        prompt_tokens=reply.prompt_tokens,
        completion_tokens=reply.completion_tokens,
    )
    return reply.content


def _json_reply(text: str, label: str) -> dict[str, Any]:
    stripped = text.strip()
    if stripped.startswith("```"):
        lines = stripped.splitlines()
        stripped = "\n".join(lines[1:-1] if lines[-1].strip() == "```" else lines[1:])
    try:
        value = json.loads(stripped)
    except json.JSONDecodeError as exc:
        raise Extract01Error(f"{label} returned invalid JSON") from exc
    if not isinstance(value, dict):
        raise Extract01Error(f"{label} returned a non-object")
    return value


def _extract_result(
    checkpoint: Checkpoint,
    client: AirlockClient,
    config: Mapping[str, Any],
    document: Mapping[str, Any],
) -> dict[str, Any]:
    cell_id = f"extract:{document['source_doc_id']}"

    def execute() -> dict[str, Any]:
        messages = _extract_messages(
            document, config["extraction"]["max_facts_per_document"]
        )
        source = str(document["source_doc_id"])
        try:
            return semantic_retry(
                config["live"]["max_retries"],
                lambda: _model_call(
                    checkpoint,
                    client,
                    cell_id,
                    config["live"]["extractor"],
                    messages,
                    json_mode=True,
                ),
                lambda content: normalize_elps_result(
                    _json_reply(content, "extractor"), source
                ),
            )
        except Extract01Error as exc:
            if str(exc) != "model semantic retries exhausted":
                raise
            checkpoint.put(
                f"extraction-failure:{source}",
                {"reason": "semantic_retries_exhausted"},
            )
            return {"entities": [], "edges": [], "warnings": []}

    result = complete_once(checkpoint, cell_id, execute)
    normalized = normalize_elps_result(result, str(document["source_doc_id"]))
    if normalized != result:
        checkpoint.put(cell_id, normalized)
    return normalized


def _hits(result: object, limit: int) -> list[dict[str, str]]:
    hits: list[dict[str, str]] = []
    for hit in getattr(result, "results", [])[:limit]:
        logical_id = getattr(getattr(hit, "id", None), "value", None)
        source_id, body = getattr(hit, "source_id", None), getattr(hit, "body", None)
        if not all(
            isinstance(value, str) and value for value in (logical_id, source_id, body)
        ):
            raise Extract01Error("retrieved hit lacks canonical attribution")
        hits.append({"id": logical_id, "source_id": source_id, "body": body})
    return hits


def normalize_answer_reply(
    value: object, hits: Sequence[Mapping[str, str]]
) -> dict[str, Any]:
    answer = _exact(value, "answerer", {"answer", "cited_ids"})
    if not isinstance(answer["answer"], str) or not isinstance(
        answer["cited_ids"], list
    ):
        raise Extract01Error("answerer response types are invalid")
    by_id = {hit["id"]: hit["id"] for hit in hits}
    by_id.update({hit["source_id"]: hit["id"] for hit in hits})
    cited: list[str] = []
    for item in answer["cited_ids"]:
        canonical = by_id.get(item) if isinstance(item, str) else None
        if canonical is not None and canonical not in cited:
            cited.append(canonical)
    return {"answer": answer["answer"], "cited_ids": cited}


def _answer(
    checkpoint: Checkpoint,
    client: AirlockClient,
    config: Mapping[str, Any],
    case: Case,
    arm: str,
    hits: Sequence[Mapping[str, str]],
) -> dict[str, Any]:
    cell_id = f"answer:{arm}:{case.question_id}"

    def execute() -> dict[str, Any]:
        messages = _answer_messages(case, hits)
        return semantic_retry(
            config["live"]["max_retries"],
            lambda: _model_call(
                checkpoint,
                client,
                cell_id,
                config["live"]["answerer"],
                messages,
                json_mode=True,
            ),
            lambda content: normalize_answer_reply(
                _json_reply(content, "answerer"), hits
            ),
        )

    return complete_once(checkpoint, cell_id, execute)


def _judge(
    checkpoint: Checkpoint,
    client: AirlockClient,
    config: Mapping[str, Any],
    case: Case,
    arm: str,
    answer: str,
) -> bool:
    cell_id = f"judge:{arm}:{case.question_id}"

    def execute() -> dict[str, bool]:
        messages = _judge_messages(case, answer)

        def parse(content: str) -> dict[str, bool]:
            normalized = content.strip().lower()
            if not (normalized.startswith("yes") or normalized.startswith("no")):
                raise Extract01Error("judge did not return yes or no")
            return {"correct": normalized.startswith("yes")}

        return semantic_retry(
            config["live"]["max_retries"],
            lambda: _model_call(
                checkpoint,
                client,
                cell_id,
                config["live"]["judge"],
                messages,
                json_mode=False,
            ),
            parse,
        )

    return bool(complete_once(checkpoint, cell_id, execute)["correct"])


def _evidence_hit(
    hits: Sequence[Mapping[str, str]],
    case: Case,
    chunk_sessions: Mapping[str, Sequence[str]],
) -> bool:
    for hit in hits:
        source = hit["source_id"]
        raw_prefix = f"{case.question_id}:"
        if (
            source.startswith(raw_prefix)
            and source[len(raw_prefix) :] in case.answer_session_ids
        ):
            return True
        if set(chunk_sessions.get(source, ())) & case.answer_session_ids:
            return True
    return False


def _next_test_id(root: Path, ordinal: int) -> str:
    attempt = 1
    while (root / f"extract-01-case-{ordinal:03d}-attempt-{attempt}").exists():
        attempt += 1
    return f"extract-01-case-{ordinal:03d}-attempt-{attempt}"


def _write_replay(
    path: Path,
    model: str,
    documents: Sequence[Mapping[str, Any]],
    results: Mapping[str, Mapping[str, Any]],
) -> None:
    _atomic_json(
        path,
        {
            "schema_version": "extract-01.elps-replay.v1",
            "model": model,
            "documents": {
                str(doc["source_doc_id"]): results[str(doc["source_doc_id"])]
                for doc in documents
            },
        },
    )


def _case_run(
    ordinal: int,
    case: Case,
    config: Mapping[str, Any],
    checkpoint: Checkpoint,
    client: AirlockClient,
    artifact_root: Path,
    fathomdb_bin: str,
) -> dict[str, Any]:
    documents = extraction_documents(case, config["extraction"]["max_document_chars"])
    extracted = {
        str(document["source_doc_id"]): _extract_result(
            checkpoint, client, config, document
        )
        for document in documents
    }
    prepared = prepare_test_database(
        artifact_root / "databases",
        test_id=_next_test_id(artifact_root / "databases", ordinal),
        embed_device="cpu",
        rerank_device="cpu",
        embedder="none",
        warm_cache=False,
        check_reranker=False,
        fathomdb_bin=fathomdb_bin,
    )
    from fathomdb import Engine

    engine = Engine.open(str(prepared.database_path), use_default_embedder=False)
    try:
        rows = raw_rows(case)
        started = time.monotonic()
        engine.write(rows)
        raw_ingest_ms = (time.monotonic() - started) * 1000
        measurement = sqlite3.connect(prepared.database_path)
        try:
            raw_bytes = logical_database_bytes(measurement)
        finally:
            measurement.close()
        raw_hits = _hits(
            engine.search_text_only(case.question, limit=config["profile"]["top_k"]), 10
        )
        raw_answer = _answer(checkpoint, client, config, case, "raw", raw_hits)
        raw_correct = _judge(
            checkpoint, client, config, case, "raw", raw_answer["answer"]
        )

        replay_path = artifact_root / "replay" / f"{case.question_id}.json"
        _write_replay(
            replay_path, config["live"]["extractor"]["model"], documents, extracted
        )
        ingest_documents = [
            {
                "source_doc_id": str(document["source_doc_id"]),
                "body": str(document["body"]),
            }
            for document in documents
        ]
        started = time.monotonic()
        receipt = engine.ingest_with_extractor(
            [sys.executable, str(REPLAY), str(replay_path)], ingest_documents
        )
        treatment_ingest_ms = (time.monotonic() - started) * 1000
        measurement = sqlite3.connect(prepared.database_path)
        try:
            treatment_bytes = logical_database_bytes(measurement)
        finally:
            measurement.close()
        treatment_hits = _hits(
            engine.search_text_only(case.question, limit=config["profile"]["top_k"]), 10
        )
        treatment_answer = _answer(
            checkpoint, client, config, case, "treatment", treatment_hits
        )
        treatment_correct = _judge(
            checkpoint, client, config, case, "treatment", treatment_answer["answer"]
        )
    finally:
        engine.close()
    source_total = sum(
        len(value["entities"]) + len(value["edges"]) for value in extracted.values()
    )
    chunk_sessions = {
        str(document["source_doc_id"]): document["session_ids"]
        for document in documents
    }
    return {
        "raw": {
            "correct": raw_correct,
            "evidence_hit": _evidence_hit(raw_hits, case, chunk_sessions),
            "answer": raw_answer["answer"],
        },
        "treatment": {
            "correct": treatment_correct,
            "evidence_hit": _evidence_hit(treatment_hits, case, chunk_sessions),
            "answer": treatment_answer["answer"],
        },
        "source_link_total": source_total,
        "source_link_valid": source_total,
        "extracted_rows": int(receipt.nodes_written + receipt.edges_written),
        "raw_rows": len(rows),
        "raw_ingest_ms": raw_ingest_ms,
        "ingest_ms": treatment_ingest_ms,
        "raw_storage_bytes": raw_bytes,
        "storage_bytes": treatment_bytes,
        "measurement_revision": MEASUREMENT_REVISION,
        "doctor_sha256": _lib._sha256_file(prepared.doctor_path),
    }


def _synthetic_result(source: str, place: str, body: str) -> dict[str, Any]:
    return {
        "entities": [
            {"name": "User", "type": "Person", "aliases": [], "source_doc_id": source},
            {"name": place, "type": "Place", "aliases": [], "source_doc_id": source},
        ],
        "edges": [
            {
                "from_entity": "User",
                "to_entity": place,
                "relation": "lives_in",
                "body": body,
                "t_valid": "2025-01-01T00:00:00Z",
                "t_invalid": None,
                "confidence": 1.0,
                "source_doc_id": source,
                "source_span": None,
            }
        ],
        "warnings": [],
    }


def _lifecycle_run(
    config: Mapping[str, Any], artifact_root: Path, fathomdb_bin: str
) -> dict[str, Any]:
    prepared = prepare_test_database(
        artifact_root / "databases",
        test_id="extract-01-lifecycle",
        embed_device="cpu",
        rerank_device="cpu",
        embedder="none",
        warm_cache=False,
        check_reranker=False,
        fathomdb_bin=fathomdb_bin,
    )
    sources = ("lifecycle-prior", "lifecycle-reaffirm", "lifecycle-update")
    results = {
        sources[0]: _synthetic_result(
            sources[0], "Boston", "The user lives in Boston."
        ),
        sources[1]: _synthetic_result(
            sources[1], "Boston", "The user still lives in Boston."
        ),
        sources[2]: _synthetic_result(
            sources[2], "Austin", "The user now lives in Austin."
        ),
    }
    replay = artifact_root / "replay" / "lifecycle.json"
    documents = [
        {"source_doc_id": source, "body": results[source]["edges"][0]["body"]}
        for source in sources
    ]
    _write_replay(replay, config["live"]["extractor"]["model"], documents, results)
    from fathomdb import Engine

    engine = Engine.open(str(prepared.database_path), use_default_embedder=False)
    try:
        for document in documents:
            engine.ingest_with_extractor(
                [sys.executable, str(REPLAY), str(replay)], [document]
            )
        connection = sqlite3.connect(prepared.database_path)
        try:
            active_lives = connection.execute(
                "SELECT COUNT(*) FROM canonical_edges WHERE kind='lives_in' AND superseded_at IS NULL"
            ).fetchone()[0]
            boston_versions = connection.execute(
                "SELECT COUNT(*) FROM canonical_edges WHERE kind='lives_in' AND body LIKE '%Boston%'"
            ).fetchone()[0]
        finally:
            connection.close()
        conflict_detected = active_lives == 2
        same_identity_supersession = boston_versions == 2
        engine.erase_source(sources[2])
        erasure_absent = not _hits(engine.search_text_only("Austin", limit=10), 10)
    finally:
        engine.close()
    connection = sqlite3.connect(prepared.database_path)
    try:
        orphan_count = connection.execute(
            "SELECT COUNT(*) FROM canonical_edges e WHERE e.superseded_at IS NULL AND "
            "(NOT EXISTS (SELECT 1 FROM canonical_nodes n WHERE n.logical_id=e.from_id AND n.superseded_at IS NULL) "
            "OR NOT EXISTS (SELECT 1 FROM canonical_nodes n WHERE n.logical_id=e.to_id AND n.superseded_at IS NULL))"
        ).fetchone()[0]
    finally:
        connection.close()
    return {
        "conflict_detected": conflict_detected,
        "supersession_applied": False,
        "same_identity_supersession": same_identity_supersession,
        "erasure_absent": erasure_absent,
        "orphan_count": orphan_count,
        "doctor_sha256": _lib._sha256_file(prepared.doctor_path),
    }


def summarize(state: RunState, question_ids: Sequence[str]) -> dict[str, Any]:
    cases: list[Mapping[str, Any]] = []
    for question_id in question_ids:
        value = state.cells.get(f"case:{question_id}")
        if not isinstance(value, Mapping):
            raise Extract01Error("EXTRACT-01 result is incomplete")
        cases.append(value)
    lifecycle = state.cells.get("lifecycle")
    if not isinstance(lifecycle, Mapping):
        raise Extract01Error("EXTRACT-01 lifecycle result is incomplete")
    arms: dict[str, Any] = {}
    for arm_id, key in ((ARM_IDS[0], "raw"), (ARM_IDS[1], "treatment")):
        arms[arm_id] = {
            "n": len(cases),
            "answer_accuracy": sum(bool(case[key]["correct"]) for case in cases)
            / len(cases),
            "evidence_recall_at_10": sum(
                bool(case[key]["evidence_hit"]) for case in cases
            )
            / len(cases),
        }
    total_links = sum(int(case["source_link_total"]) for case in cases)
    valid_links = sum(int(case["source_link_valid"]) for case in cases)
    raw_bytes = sum(
        int(case.get("raw_storage_bytes", case["storage_bytes"])) for case in cases
    )
    treatment_bytes = sum(int(case["storage_bytes"]) for case in cases)
    delta = arms[ARM_IDS[1]]["answer_accuracy"] - arms[ARM_IDS[0]]["answer_accuracy"]
    provenance = 1.0 if total_links == 0 else valid_links / total_links
    lifecycle_ok = (
        bool(lifecycle["erasure_absent"])
        and int(lifecycle["orphan_count"]) == 0
        and bool(lifecycle["supersession_applied"])
    )
    if delta > 0 and provenance == 1.0 and lifecycle_ok:
        decision = "adopt_native_extraction"
    elif delta <= 0:
        decision = "retain_raw_no_quality_gain"
    else:
        decision = "do_not_adopt_unconsolidated_extraction"
    return {
        "schema_version": "extract-01.safe-summary.v1",
        "program_track": PROGRAM_TRACK,
        "status": "complete",
        "n": len(cases),
        "arms": arms,
        "paired_answer_accuracy_delta": delta,
        "source_link_completeness": provenance,
        "extraction_precision": "unscored_no_human_gold",
        "failed_extraction_document_count": sum(
            key.startswith("extraction-failure:") for key in state.cells
        ),
        "extracted_row_count": sum(int(case["extracted_rows"]) for case in cases),
        "raw_row_count": sum(int(case["raw_rows"]) for case in cases),
        "storage_amplification": treatment_bytes / raw_bytes if raw_bytes else None,
        "mean_treatment_ingest_ms": sum(float(case["ingest_ms"]) for case in cases)
        / len(cases),
        "lifecycle": dict(lifecycle),
        "cost_usd": state.cost_usd,
        "decision": decision,
    }


def preflight(
    config: Mapping[str, Any], corpus: object, client: AirlockClient | None = None
) -> dict[str, Any]:
    cases = load_cases(corpus, config["scope"]["question_count"])
    documents = [
        document
        for case in cases
        for document in extraction_documents(
            case, config["extraction"]["max_document_chars"]
        )
    ]
    chars = sum(len(document["body"]) for document in documents)
    extractor = config["live"]["extractor"]
    conservative = (
        math.ceil(chars / 3) * extractor["input_usd_per_million"]
        + len(documents) * extractor["max_tokens"] * extractor["output_usd_per_million"]
    ) / 1_000_000
    required = {
        config["live"][role]["model"] for role in ("extractor", "answerer", "judge")
    }
    if client is not None:
        missing = required - client.models()
        if missing:
            raise Extract01Error(
                f"Airlock lacks required model aliases: {sorted(missing)}"
            )
    return {
        "schema_version": "extract-01.preflight.v1",
        "program_track": PROGRAM_TRACK,
        "status": "ready",
        "question_count": len(cases),
        "extraction_document_count": len(documents),
        "history_chars": chars,
        "extractor_worst_case_usd": conservative,
        "max_usd": config["live"]["max_usd"],
        "models": sorted(required),
        "oracle_used_for_extraction": False,
        "gpu_required": False,
    }


def run(
    config: Mapping[str, Any],
    *,
    corpus: object,
    artifact_root: Path,
    client: AirlockClient,
    fathomdb_bin: str,
    base_dir: Path,
) -> tuple[str, Path, dict[str, Any]]:
    if artifact_root.resolve().is_relative_to(_lib.REPO_ROOT.resolve()):
        raise Extract01Error("EXTRACT-01 artifacts must remain outside the repository")
    cases = load_cases(corpus, config["scope"]["question_count"])
    artifact_root.mkdir(parents=True, mode=0o700, exist_ok=True)
    config_sha = _lib.config_sha256(config)
    checkpoint = Checkpoint.open(
        artifact_root / "extract-01-checkpoint.v1.json",
        config_sha,
        [case.question_id for case in cases],
    )
    for ordinal, case in enumerate(cases, 1):
        key = f"case:{case.question_id}"
        if not case_measurement_is_current(checkpoint.state.cells.get(key)):
            checkpoint.put(
                key,
                _case_run(
                    ordinal,
                    case,
                    config,
                    checkpoint,
                    client,
                    artifact_root,
                    fathomdb_bin,
                ),
            )
        print(
            f"EXTRACT-01 case {ordinal}/{len(cases)} complete; cost=${checkpoint.state.cost_usd:.4f}",
            flush=True,
        )
    if "lifecycle" not in checkpoint.state.cells:
        checkpoint.put("lifecycle", _lifecycle_run(config, artifact_root, fathomdb_bin))
    summary = summarize(checkpoint.state, [case.question_id for case in cases])
    manifest = artifact_root / "artifact-manifest.v1.json"
    external_files = [
        checkpoint.path,
        *sorted((artifact_root / "replay").glob("*.json")),
    ]
    _atomic_json(
        manifest,
        {
            "schema_version": "extract-01.artifact-manifest.v1",
            "files": [
                {
                    "path": path.relative_to(artifact_root).as_posix(),
                    "sha256": _lib._sha256_file(path),
                }
                for path in external_files
            ],
        },
    )
    timestamp = datetime.now(timezone.utc).replace(second=0, microsecond=0)
    run_id, run_dir = _lib.write_record(
        "extract-01-knowledge-update",
        ts=timestamp,
        config_obj=config,
        metrics=summary,
        verdict="complete",
        read=f"EXTRACT-01 completed; decision={summary['decision']}",
        code=_lib.git_info(),
        corpus={
            "source": "LongMemEval-S knowledge-update",
            "manifest_sha256": config["corpus"]["sha256"],
            "datasets": ["longmemeval-s-cleaned"],
        },
        seeds={},
        env=_lib.env_info(),
        cost_usd=summary["cost_usd"],
        headline={
            "program_track": PROGRAM_TRACK,
            "status": "complete",
            "decision": summary["decision"],
        },
        n=len(cases),
        config_path="experiments/configs/extract-01/knowledge-update.v1.json",
        artifacts=[
            {
                "path": "external-extract-01-artifact-manifest.v1",
                "sha256": _lib._sha256_file(manifest),
            }
        ],
        base_dir=base_dir,
    )
    _lib.regen_index_md(
        index_path=base_dir / "index.jsonl", md_path=base_dir / "INDEX.md"
    )
    return run_id, run_dir, summary


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    validate = commands.add_parser("validate")
    validate.add_argument("config", type=Path)
    dry = commands.add_parser("preflight")
    live = commands.add_parser("run")
    for command in (dry, live):
        command.add_argument("config", type=Path)
        command.add_argument("--corpus", required=True, type=Path)
        command.add_argument("--airlock-url", default="http://127.0.0.1:4000")
    live.add_argument("--artifact-root", required=True, type=Path)
    live.add_argument("--fathomdb-bin", required=True)
    live.add_argument("--base-dir", type=Path, default=Path("experiments"))
    args = parser.parse_args(argv)
    try:
        config = load_config(args.config)
        if args.command == "validate":
            print("EXTRACT-01 configuration resolves")
            return 0
        key = (
            os.environ.get("AIRLOCK_VIRTUAL_KEY")
            or os.environ.get("AIRLOCK_MASTER_KEY")
            or ""
        )
        client = AirlockClient(args.airlock_url, key, config["live"]["max_retries"])
        corpus = _load_external(
            args.corpus, config["corpus"]["sha256"], "LongMemEval-S"
        )
        if args.command == "preflight":
            print(json.dumps(preflight(config, corpus, client), sort_keys=True))
            return 0
        run_id, run_dir, summary = run(
            config,
            corpus=corpus,
            artifact_root=args.artifact_root,
            client=client,
            fathomdb_bin=args.fathomdb_bin,
            base_dir=args.base_dir,
        )
        print(
            json.dumps(
                {
                    "run_id": run_id,
                    "receipt": str(run_dir / "record.json"),
                    "summary": summary,
                },
                sort_keys=True,
            )
        )
        return 0
    except (OSError, json.JSONDecodeError, Extract01Error, RuntimeError) as exc:
        parser.error(str(exc))
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
