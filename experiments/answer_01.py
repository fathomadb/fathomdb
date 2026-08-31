"""ANSWER-01 fixed-subset answer-scoring preparation and $0 dry run."""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import math
import os
import re
import time
import urllib.error
import urllib.request
from contextlib import contextmanager
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable, Iterator, Mapping, Sequence

from experiments import _lib
from experiments.fathomdb_test_setup import prepare_test_database


SCHEMA = "answer-01.shortlist-scoring.v1"
CHECKPOINT_SCHEMA = "answer-01.checkpoint.v1"
LIVE_CHECKPOINT_SCHEMA = "answer-01.live-checkpoint.v1"
PROGRAM_TRACK = "ANSWER-01"
ARM_IDS = ("a0_turn_fts", "hybrid_ce_alpha_10_pool_20")
REPORTING_CLASSES = ("factoid", "temporal", "multi_session")
_CLASS_BY_CATEGORY = {4: "factoid", 2: "temporal", 1: "multi_session"}
_ROOT_KEYS = {
    "schema_version",
    "program_track",
    "corpus",
    "subset",
    "arms",
    "dry_run",
    "live",
    "reporting",
}
_ARM_KEYS = {
    "id",
    "database_id",
    "retrieval",
    "top_k",
    "embedder",
    "embed_device",
    "rerank_device",
    "search",
}
_SAFE_SHA = re.compile(r"^[0-9a-f]{64}$")


class Answer01Error(ValueError):
    """Raised when ANSWER-01 inputs or execution state fail closed."""


@dataclass(frozen=True)
class Question:
    """One fixed LOCOMO question retained only in external run state."""

    question_id: str
    reporting_class: str
    query: str
    answer: str


@dataclass(frozen=True)
class ContextHit:
    """One retrieved context with canonical attribution."""

    logical_id: str
    source_id: str
    body: str


@dataclass(frozen=True)
class ModelReply:
    """One model response plus token counts used for cost accounting."""

    content: str
    prompt_tokens: int
    completion_tokens: int


def _exact(value: object, label: str, keys: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        actual = set(value) if isinstance(value, dict) else set()
        raise Answer01Error(
            f"{label} keys mismatch: missing={sorted(keys - actual)}, "
            f"unknown={sorted(actual - keys)}"
        )
    return value


def _sha(value: object, label: str) -> str:
    if not isinstance(value, str) or _SAFE_SHA.fullmatch(value) is None:
        raise Answer01Error(f"{label} must be a lowercase sha256")
    return value


def _load_ref(value: object, label: str, *, subset: bool) -> dict[str, Any]:
    keys = {"identifier", "sha256", "storage"}
    if subset:
        keys.add("question_count")
    ref = _exact(value, label, keys)
    if not isinstance(ref["identifier"], str) or not ref["identifier"]:
        raise Answer01Error(f"{label} identifier is invalid")
    _sha(ref["sha256"], f"{label} sha256")
    if ref["storage"] != "external_only":
        raise Answer01Error(f"{label} must remain external_only")
    if subset and ref["question_count"] != 32:
        raise Answer01Error("ANSWER-01 dry subset must contain 32 questions")
    return ref


def load_config(value: object) -> dict[str, Any]:
    """Strictly validate the small ANSWER-01 execution configuration."""
    config = _exact(value, "config", _ROOT_KEYS)
    if config["schema_version"] != SCHEMA or config["program_track"] != PROGRAM_TRACK:
        raise Answer01Error("ANSWER-01 schema or program track mismatch")
    _load_ref(config["corpus"], "corpus", subset=False)
    _load_ref(config["subset"], "subset", subset=True)
    if not isinstance(config["arms"], list) or len(config["arms"]) != 2:
        raise Answer01Error("ANSWER-01 requires exactly two arms")
    arms = [_exact(arm, "arm", _ARM_KEYS) for arm in config["arms"]]
    if tuple(arm["id"] for arm in arms) != ARM_IDS:
        raise Answer01Error("ANSWER-01 arm order or identity drifted")
    expected = (
        {
            "retrieval": "fts",
            "top_k": 10,
            "embedder": "none",
            "embed_device": "cpu",
            "rerank_device": "cpu",
            "search": {},
        },
        {
            "retrieval": "hybrid",
            "top_k": 10,
            "embedder": "default",
            "embed_device": "cuda:0",
            "rerank_device": "cuda:0",
            "search": {"alpha": 1.0, "pool_n": 20, "rerank_depth": 20},
        },
    )
    for arm, frozen in zip(arms, expected, strict=True):
        if any(arm[key] != expected_value for key, expected_value in frozen.items()):
            raise Answer01Error(f"ANSWER-01 arm {arm['id']} drifted")
        if not isinstance(arm["database_id"], str) or not arm["database_id"]:
            raise Answer01Error("ANSWER-01 database identity is invalid")
    dry = _exact(
        config["dry_run"],
        "dry_run",
        {"answerer", "judge", "network", "question_count"},
    )
    if dry != {
        "answerer": "stub-deterministic-v1",
        "judge": "stub-grounded-attribution-v1",
        "network": False,
        "question_count": 32,
    }:
        raise Answer01Error("ANSWER-01 dry-run boundary drifted")
    live = _exact(
        config["live"],
        "live",
        {"answerer", "judge", "max_usd", "max_workers", "max_retries", "checkpoint_every"},
    )
    for role, model in (("answerer", "gpt-5.4"), ("judge", "gemini-3.1-flash-lite")):
        model_config = _exact(
            live[role],
            f"live {role}",
            {"model", "input_usd_per_million", "output_usd_per_million"},
        )
        if model_config["model"] != model:
            raise Answer01Error(f"ANSWER-01 live {role} model drifted")
        if any(
            not isinstance(model_config[key], (int, float)) or model_config[key] <= 0
            for key in ("input_usd_per_million", "output_usd_per_million")
        ):
            raise Answer01Error(f"ANSWER-01 live {role} pricing is invalid")
    if (
        live["max_usd"] != 3.0
        or live["max_workers"] != 1
        or live["max_retries"] != 1
        or live["checkpoint_every"] != 1
    ):
        raise Answer01Error("ANSWER-01 live resilience or cost boundary drifted")
    reporting = _exact(config["reporting"], "reporting", {"classes", "metrics"})
    if reporting["classes"] != list(REPORTING_CLASSES) or reporting["metrics"] != [
        "answer_accuracy",
        "temporal_correctness",
        "grounded_rate",
        "attribution_rate",
    ]:
        raise Answer01Error("ANSWER-01 reporting boundary drifted")
    return config


def _question_rows(corpus: object) -> dict[str, Question]:
    if not isinstance(corpus, list):
        raise Answer01Error("LOCOMO corpus must be a list")
    questions: dict[str, Question] = {}
    for conversation_index, item in enumerate(corpus):
        if not isinstance(item, Mapping) or not isinstance(item.get("qa"), list):
            raise Answer01Error("LOCOMO question group is invalid")
        for question_index, raw in enumerate(item["qa"]):
            if not isinstance(raw, Mapping):
                raise Answer01Error("LOCOMO question is invalid")
            category = raw.get("category")
            reporting_class = (
                _CLASS_BY_CATEGORY.get(category) if isinstance(category, int) else None
            )
            if reporting_class is None:
                continue
            query, answer, evidence = raw.get("question"), raw.get("answer"), raw.get("evidence")
            answer_text = "" if answer is None or isinstance(answer, bool) else str(answer).strip()
            if (
                not isinstance(query, str)
                or not query.strip()
                or not answer_text
                or not isinstance(evidence, list)
                or not evidence
            ):
                raise Answer01Error("LOCOMO eligible question lacks query, answer, or evidence")
            question_id = f"locomo-{conversation_index}-q-{question_index}"
            questions[question_id] = Question(
                question_id=question_id,
                reporting_class=reporting_class,
                query=query.strip(),
                answer=answer_text,
            )
    return questions


def select_questions(corpus: object, subset: object) -> list[Question]:
    """Resolve the external 32-question control against raw LOCOMO."""
    control = _exact(subset, "subset", {"schema_version", "question_ids"})
    ids = control["question_ids"]
    if (
        control["schema_version"] != "locomo-fixed-subset.v1"
        or not isinstance(ids, list)
        or len(ids) != 32
        or len(set(ids)) != 32
        or any(not isinstance(item, str) for item in ids)
    ):
        raise Answer01Error("fixed subset must contain 32 unique question IDs")
    available = _question_rows(corpus)
    try:
        selected = [available[item] for item in ids]
    except KeyError as exc:
        raise Answer01Error("fixed subset question is absent or ineligible") from exc
    if {item.reporting_class for item in selected} != set(REPORTING_CLASSES):
        raise Answer01Error("fixed subset does not cover every reporting class")
    return selected


def _canonical_json_sha256(value: object) -> str:
    payload = json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def _normalize(value: str | None) -> str:
    if not value:
        return ""
    return " ".join(re.findall(r"[a-z0-9]+", value.lower()))


def _atomic_write(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(json.dumps(value, sort_keys=True) + "\n", encoding="utf-8")
    temporary.chmod(0o600)
    temporary.replace(path)


def _checkpoint(config: Mapping[str, Any], questions: Sequence[Question], path: Path) -> dict[str, Any]:
    config_sha = _canonical_json_sha256(config)
    question_ids = [question.question_id for question in questions]
    if not path.exists():
        return {
            "schema_version": CHECKPOINT_SCHEMA,
            "config_sha256": config_sha,
            "question_ids": question_ids,
            "records": {},
        }
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise Answer01Error("ANSWER-01 checkpoint is invalid") from exc
    expected = {"schema_version", "config_sha256", "question_ids", "records"}
    document = _exact(value, "checkpoint", expected)
    if (
        document["schema_version"] != CHECKPOINT_SCHEMA
        or document["config_sha256"] != config_sha
        or document["question_ids"] != question_ids
        or not isinstance(document["records"], dict)
    ):
        raise Answer01Error("ANSWER-01 checkpoint does not match this run")
    return document


def _mean(records: Sequence[Mapping[str, Any]], key: str) -> float:
    return sum(float(record[key]) for record in records) / len(records)


def _summary(records: Mapping[str, Mapping[str, Any]], questions: Sequence[Question]) -> dict[str, Any]:
    by_arm: dict[str, Any] = {}
    for arm_id in ARM_IDS:
        arm_records = [record for record in records.values() if record["arm_id"] == arm_id]
        if len(arm_records) != len(questions):
            raise Answer01Error(f"ANSWER-01 arm {arm_id} is incomplete")
        classes: dict[str, Any] = {}
        for reporting_class in REPORTING_CLASSES:
            class_records = [
                record for record in arm_records if record["reporting_class"] == reporting_class
            ]
            if not class_records:
                raise Answer01Error(f"ANSWER-01 class {reporting_class} is empty")
            classes[reporting_class] = {
                "n": len(class_records),
                "answer_accuracy": _mean(class_records, "answer_correct"),
                "grounded_rate": _mean(class_records, "grounded"),
                "attribution_rate": _mean(class_records, "attributed"),
            }
        by_arm[arm_id] = {
            "n": len(arm_records),
            "answer_accuracy": _mean(arm_records, "answer_correct"),
            "temporal_correctness": classes["temporal"]["answer_accuracy"],
            "grounded_rate": _mean(arm_records, "grounded"),
            "attribution_rate": _mean(arm_records, "attributed"),
            "by_class": classes,
        }
    complete_pairs = sum(
        all(f"{arm_id}||{question.question_id}" in records for arm_id in ARM_IDS)
        for question in questions
    )
    return {
        "schema_version": "answer-01.safe-summary.v1",
        "status": "dry_run_proof",
        "question_count": len(questions),
        "pair_count": len(records),
        "complete_pair_count": complete_pairs,
        "arms": by_arm,
        "cost_usd": 0.0,
        "live_calls": 0,
        "decision_eligible": False,
    }


Retrieve = Callable[[str, Question], list[ContextHit]]


def score_stub_dry_run(
    config: Mapping[str, Any],
    questions: Sequence[Question],
    *,
    retrieve: Retrieve,
    checkpoint_path: Path,
) -> dict[str, Any]:
    """Run or resume all 64 stubbed cells, checkpointing every completed cell."""
    document = _checkpoint(config, questions, checkpoint_path)
    records = document["records"]
    assert isinstance(records, dict)
    for question in questions:
        for arm_id in ARM_IDS:
            cell_id = f"{arm_id}||{question.question_id}"
            if cell_id in records:
                continue
            hits = retrieve(arm_id, question)
            candidate = hits[0].body if hits else None
            normalized_candidate = _normalize(candidate)
            answer_correct = bool(_normalize(question.answer)) and _normalize(question.answer) in normalized_candidate
            grounded = bool(candidate) and any(
                normalized_candidate and normalized_candidate in _normalize(hit.body) for hit in hits
            )
            attributed = bool(hits) and bool(hits[0].logical_id) and bool(hits[0].source_id)
            records[cell_id] = {
                "arm_id": arm_id,
                "question_id": question.question_id,
                "reporting_class": question.reporting_class,
                "question": question.query,
                "gold_answer": question.answer,
                "candidate": candidate,
                "hits": [
                    {
                        "logical_id": hit.logical_id,
                        "source_id": hit.source_id,
                        "body": hit.body,
                    }
                    for hit in hits
                ],
                "answer_correct": answer_correct,
                "grounded": grounded,
                "attributed": attributed,
            }
            _atomic_write(checkpoint_path, document)
    return _summary(records, questions)


def _load_complete_dry_checkpoint(
    config: Mapping[str, Any], questions: Sequence[Question], path: Path
) -> dict[str, Any]:
    document = _checkpoint(config, questions, path)
    records = document["records"]
    if not isinstance(records, dict) or any(
        f"{arm_id}||{question.question_id}" not in records
        for question in questions
        for arm_id in ARM_IDS
    ):
        raise Answer01Error("ANSWER-01 live run requires a complete dry checkpoint")
    return document


def _live_checkpoint(
    config: Mapping[str, Any], questions: Sequence[Question], path: Path
) -> dict[str, Any]:
    config_sha = _canonical_json_sha256(config)
    question_ids = [question.question_id for question in questions]
    if not path.exists():
        return {
            "schema_version": LIVE_CHECKPOINT_SCHEMA,
            "config_sha256": config_sha,
            "question_ids": question_ids,
            "records": {},
        }
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise Answer01Error("ANSWER-01 live checkpoint is invalid") from exc
    document = _exact(
        value,
        "live checkpoint",
        {"schema_version", "config_sha256", "question_ids", "records"},
    )
    if (
        document["schema_version"] != LIVE_CHECKPOINT_SCHEMA
        or document["config_sha256"] != config_sha
        or document["question_ids"] != question_ids
        or not isinstance(document["records"], dict)
    ):
        raise Answer01Error("ANSWER-01 live checkpoint does not match this run")
    return document


def _json_object(content: str, label: str) -> dict[str, Any]:
    start, end = content.find("{"), content.rfind("}")
    if start < 0 or end < start:
        raise Answer01Error(f"ANSWER-01 {label} did not return a JSON object")
    try:
        value = json.loads(content[start : end + 1])
    except json.JSONDecodeError as exc:
        raise Answer01Error(f"ANSWER-01 {label} returned invalid JSON") from exc
    if not isinstance(value, dict):
        raise Answer01Error(f"ANSWER-01 {label} did not return a JSON object")
    return value


def _answer_messages(
    question: str, hits: Sequence[Mapping[str, Any]]
) -> list[dict[str, str]]:
    context = "\n".join(f"[{hit['logical_id']}] {hit['body']}" for hit in hits)
    return [
        {
            "role": "system",
            "content": (
                "Answer only from the supplied memories. Return one JSON object with "
                'keys "answer" (string) and "citations" (an array of memory IDs). '
                "Use an empty answer and citations when the memories are insufficient."
            ),
        },
        {"role": "user", "content": f"Question:\n{question}\n\nMemories:\n{context}"},
    ]


def _judge_messages(
    question: str,
    gold_answer: str,
    hits: Sequence[Mapping[str, Any]],
    candidate_answer: str,
    citations: Sequence[str],
) -> list[dict[str, str]]:
    context = "\n".join(f"[{hit['logical_id']}] {hit['body']}" for hit in hits)
    return [
        {
            "role": "system",
            "content": (
                "Judge the candidate independently of retrieval treatment. Return one JSON "
                "object with boolean keys answer_correct, grounded, and attributed. "
                "answer_correct means semantically correct against the reference; grounded "
                "means fully supported by supplied memories; attributed means every material "
                "claim is supported by the cited memory IDs."
            ),
        },
        {
            "role": "user",
            "content": (
                f"Question:\n{question}\n\nReference answer:\n{gold_answer}"
                f"\n\nMemories:\n{context}\n\nCandidate answer:\n{candidate_answer}"
                f"\n\nCited memory IDs:\n{json.dumps(list(citations))}"
            ),
        },
    ]


def _estimated_cost(
    model_config: Mapping[str, Any],
    messages: Sequence[Mapping[str, str]],
    max_tokens: int,
) -> float:
    input_chars = sum(len(message["content"]) for message in messages)
    input_tokens = math.ceil(input_chars / 3)
    return (
        input_tokens * float(model_config["input_usd_per_million"])
        + max_tokens * float(model_config["output_usd_per_million"])
    ) / 1_000_000


def _reply_cost(model_config: Mapping[str, Any], reply: ModelReply) -> float:
    if reply.prompt_tokens < 0 or reply.completion_tokens < 0:
        raise Answer01Error("ANSWER-01 model usage contains a negative token count")
    return (
        reply.prompt_tokens * float(model_config["input_usd_per_million"])
        + reply.completion_tokens * float(model_config["output_usd_per_million"])
    ) / 1_000_000


def _live_summary(
    records: Mapping[str, Mapping[str, Any]], questions: Sequence[Question], status: str
) -> dict[str, Any]:
    completed = {
        key: value
        for key, value in records.items()
        if all(metric in value for metric in ("answer_correct", "grounded", "attributed"))
    }
    live_calls = sum(
        int("answer_reply" in record) + int("judge_reply" in record)
        for record in records.values()
    )
    cost_usd = sum(
        float(record.get("answer_cost_usd", 0.0))
        + float(record.get("judge_cost_usd", 0.0))
        for record in records.values()
    )
    complete_pairs = sum(
        all(f"{arm_id}||{question.question_id}" in completed for arm_id in ARM_IDS)
        for question in questions
    )
    summary: dict[str, Any] = {
        "schema_version": "answer-01.safe-summary.v1",
        "status": status,
        "question_count": len(questions),
        "pair_count": len(completed),
        "complete_pair_count": complete_pairs,
        "cost_usd": cost_usd,
        "live_calls": live_calls,
        "decision_eligible": False,
    }
    if len(completed) != len(questions) * len(ARM_IDS):
        return summary
    summary.update(_summary(completed, questions))
    summary.update({"status": status, "cost_usd": cost_usd, "live_calls": live_calls})
    baseline = summary["arms"][ARM_IDS[0]]
    winner = summary["arms"][ARM_IDS[1]]
    deltas = {
        metric: winner[metric] - baseline[metric]
        for metric in (
            "answer_accuracy",
            "temporal_correctness",
            "grounded_rate",
            "attribution_rate",
        )
    }
    if deltas["answer_accuracy"] < 0:
        decision = "retain_a0"
    elif deltas["answer_accuracy"] > 0 and all(
        deltas[metric] >= 0
        for metric in ("temporal_correctness", "grounded_rate", "attribution_rate")
    ):
        decision = "advance_winner"
    else:
        decision = "inconclusive"
    summary.update(
        {
            "paired_deltas": deltas,
            "decision": decision,
            "decision_eligible": status == "complete",
        }
    )
    return summary


Complete = Callable[[str, str, Sequence[Mapping[str, str]], int], ModelReply]


def score_live(
    config: Mapping[str, Any],
    questions: Sequence[Question],
    *,
    dry_checkpoint_path: Path,
    checkpoint_path: Path,
    complete: Complete,
) -> dict[str, Any]:
    """Run or resume paired answer/judge scoring under the fixed cost cap."""
    dry = _load_complete_dry_checkpoint(config, questions, dry_checkpoint_path)
    dry_records = dry["records"]
    document = _live_checkpoint(config, questions, checkpoint_path)
    records = document["records"]
    assert isinstance(dry_records, dict) and isinstance(records, dict)
    live = config["live"]
    for question_index, question in enumerate(questions):
        arm_order = ARM_IDS if question_index % 2 == 0 else tuple(reversed(ARM_IDS))
        for arm_id in arm_order:
            cell_id = f"{arm_id}||{question.question_id}"
            source = dry_records[cell_id]
            record = records.setdefault(
                cell_id,
                {
                    "arm_id": arm_id,
                    "question_id": question.question_id,
                    "reporting_class": question.reporting_class,
                },
            )
            if "answer_reply" not in record:
                messages = _answer_messages(source["question"], source["hits"])
                reserve = _estimated_cost(live["answerer"], messages, 256)
                spent = _live_summary(records, questions, "cost_cap")["cost_usd"]
                if spent + reserve > live["max_usd"]:
                    _atomic_write(checkpoint_path, document)
                    return _live_summary(records, questions, "cost_cap")
                reply = complete("answerer", live["answerer"]["model"], messages, 256)
                record.update(
                    {
                        "answer_reply": {
                            "content": reply.content,
                            "prompt_tokens": reply.prompt_tokens,
                            "completion_tokens": reply.completion_tokens,
                        },
                        "answer_cost_usd": _reply_cost(live["answerer"], reply),
                    }
                )
                _atomic_write(checkpoint_path, document)
            if "candidate_answer" not in record:
                answer = _exact(
                    _json_object(record["answer_reply"]["content"], "answerer"),
                    "answerer response",
                    {"answer", "citations"},
                )
                if not isinstance(answer["answer"], str) or not isinstance(
                    answer["citations"], list
                ):
                    raise Answer01Error("ANSWER-01 answerer response types are invalid")
                if any(not isinstance(item, str) for item in answer["citations"]):
                    raise Answer01Error("ANSWER-01 answerer citations are invalid")
                record.update(
                    {
                        "candidate_answer": answer["answer"],
                        "citations": answer["citations"],
                    }
                )
                _atomic_write(checkpoint_path, document)
            if "judge_reply" not in record:
                messages = _judge_messages(
                    source["question"],
                    source["gold_answer"],
                    source["hits"],
                    record["candidate_answer"],
                    record["citations"],
                )
                reserve = _estimated_cost(live["judge"], messages, 128)
                spent = _live_summary(records, questions, "cost_cap")["cost_usd"]
                if spent + reserve > live["max_usd"]:
                    _atomic_write(checkpoint_path, document)
                    return _live_summary(records, questions, "cost_cap")
                reply = complete("judge", live["judge"]["model"], messages, 128)
                record.update(
                    {
                        "judge_reply": {
                            "content": reply.content,
                            "prompt_tokens": reply.prompt_tokens,
                            "completion_tokens": reply.completion_tokens,
                        },
                        "judge_cost_usd": _reply_cost(live["judge"], reply),
                    }
                )
                _atomic_write(checkpoint_path, document)
            if "answer_correct" not in record:
                judgment = _exact(
                    _json_object(record["judge_reply"]["content"], "judge"),
                    "judge response",
                    {"answer_correct", "grounded", "attributed"},
                )
                if any(not isinstance(judgment[key], bool) for key in judgment):
                    raise Answer01Error("ANSWER-01 judge response types are invalid")
                record.update(judgment)
                _atomic_write(checkpoint_path, document)
    return _live_summary(records, questions, "complete")


def _ingest_rows(corpus: object) -> list[dict[str, str]]:
    if not isinstance(corpus, list):
        raise Answer01Error("LOCOMO corpus must be a list")
    rows: list[dict[str, str]] = []
    for conversation_index, item in enumerate(corpus):
        conversation = item.get("conversation") if isinstance(item, Mapping) else None
        if not isinstance(conversation, Mapping):
            raise Answer01Error("LOCOMO conversation is invalid")
        for session_id in sorted(
            key for key in conversation if re.fullmatch(r"session_[0-9]+", str(key))
        ):
            turns = conversation[session_id]
            if not isinstance(turns, list):
                raise Answer01Error("LOCOMO session is invalid")
            for ordinal, turn in enumerate(turns):
                if not isinstance(turn, Mapping):
                    raise Answer01Error("LOCOMO turn is invalid")
                speaker, text = turn.get("speaker"), turn.get("text")
                raw_id = turn.get("dia_id", f"turn-{ordinal}")
                if not isinstance(speaker, str) or not isinstance(text, str) or not isinstance(raw_id, str):
                    raise Answer01Error("LOCOMO turn fields are invalid")
                rows.append(
                    {
                        "kind": "locomo_message_chunk",
                        "body": f"{speaker}: {text}",
                        "source_id": f"locomo-{conversation_index}",
                        "logical_id": f"locomo-{conversation_index}:{session_id}:{raw_id}",
                    }
                )
    if not rows:
        raise Answer01Error("LOCOMO corpus contains no turns")
    return rows


@contextmanager
def _devices(embed_device: str, rerank_device: str) -> Iterator[None]:
    prior = {
        "FATHOMDB_EMBED_DEVICE": os.environ.get("FATHOMDB_EMBED_DEVICE"),
        "FATHOMDB_RERANK_DEVICE": os.environ.get("FATHOMDB_RERANK_DEVICE"),
    }
    os.environ.update(
        {"FATHOMDB_EMBED_DEVICE": embed_device, "FATHOMDB_RERANK_DEVICE": rerank_device}
    )
    try:
        yield
    finally:
        for key, value in prior.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value


class FathomContextProvider:
    """Create and own one new FathomDB database for each ANSWER-01 arm."""

    def __init__(
        self,
        config: Mapping[str, Any],
        corpus: object,
        *,
        artifact_root: Path,
        fathomdb_bin: str,
    ) -> None:
        engine_class = importlib.import_module("fathomdb").Engine

        self._engines: dict[str, tuple[Any, Mapping[str, Any]]] = {}
        self.doctor_paths: list[Path] = []
        rows = _ingest_rows(corpus)
        database_root = artifact_root / "databases"
        database_root.mkdir(parents=True, exist_ok=False)
        for arm in config["arms"]:
            prepared = prepare_test_database(
                database_root,
                test_id=arm["database_id"],
                embed_device=arm["embed_device"],
                rerank_device=arm["rerank_device"],
                embedder=arm["embedder"],
                warm_cache=arm["embedder"] == "default",
                fathomdb_bin=fathomdb_bin,
            )
            self.doctor_paths.append(prepared.doctor_path)
            with _devices(arm["embed_device"], arm["rerank_device"]):
                engine = engine_class.open(
                    str(prepared.database_path),
                    use_default_embedder=arm["embedder"] == "default",
                )
                engine.write(rows)
                engine.drain(timeout_s=180)
            self._engines[arm["id"]] = (engine, arm)

    def retrieve(self, arm_id: str, question: Question) -> list[ContextHit]:
        try:
            engine, arm = self._engines[arm_id]
        except KeyError as exc:
            raise Answer01Error(f"unknown ANSWER-01 arm: {arm_id}") from exc
        with _devices(arm["embed_device"], arm["rerank_device"]):
            if arm["retrieval"] == "fts":
                result = engine.search_text_only(question.query, limit=arm["top_k"])
            else:
                result = engine.search(question.query, limit=arm["top_k"], **arm["search"])
                if getattr(engine, "dense_disabled", lambda: False)():
                    raise Answer01Error("winner arm silently disabled dense retrieval")
                if getattr(result, "soft_fallback", None):
                    raise Answer01Error("winner arm used a soft fallback")
        hits: list[ContextHit] = []
        for hit in result.results[: arm["top_k"]]:
            logical_id = getattr(getattr(hit, "id", None), "value", None)
            source_id = getattr(hit, "source_id", None)
            body = getattr(hit, "body", None)
            if (
                not isinstance(logical_id, str)
                or not logical_id
                or not isinstance(source_id, str)
                or not source_id
                or not isinstance(body, str)
                or not body
            ):
                raise Answer01Error("retrieved context lacks body or canonical attribution")
            hits.append(ContextHit(logical_id, source_id, body))
        return hits

    def close(self) -> None:
        for engine, _arm in self._engines.values():
            engine.close()


class AirlockClient:
    """Minimal authenticated OpenAI-compatible client for the fixed live run."""

    def __init__(self, base_url: str, key: str, *, max_retries: int) -> None:
        if not base_url.startswith(("http://127.0.0.1:", "http://localhost:")):
            raise Answer01Error("ANSWER-01 Airlock route must be loopback")
        if not key:
            raise Answer01Error("ANSWER-01 requires an Airlock virtual or master key")
        self._base_url = base_url.rstrip("/")
        self._key = key
        self._max_retries = max_retries
        self._calls = 0

    def _request(self, path: str, payload: object | None = None) -> dict[str, Any]:
        body = None if payload is None else json.dumps(payload).encode("utf-8")
        request = urllib.request.Request(
            self._base_url + path,
            data=body,
            method="GET" if body is None else "POST",
            headers={
                "Authorization": f"Bearer {self._key}",
                "Content-Type": "application/json",
            },
        )
        for attempt in range(self._max_retries + 1):
            try:
                with urllib.request.urlopen(request, timeout=180) as response:
                    value = json.loads(response.read())
                if not isinstance(value, dict):
                    raise Answer01Error("Airlock returned a non-object response")
                return value
            except urllib.error.HTTPError as exc:
                retryable = exc.code == 429 or 500 <= exc.code < 600
                if not retryable or attempt == self._max_retries:
                    raise Answer01Error(f"Airlock HTTP {exc.code}") from exc
                retry_after = exc.headers.get("Retry-After", "1")
                try:
                    delay = float(retry_after)
                except ValueError:
                    delay = 1.0
                time.sleep(min(max(delay, 0.0), 30.0))
            except urllib.error.URLError as exc:
                if attempt == self._max_retries:
                    raise Answer01Error("Airlock route is unavailable") from exc
                time.sleep(1.0)
            except json.JSONDecodeError as exc:
                raise Answer01Error("Airlock returned invalid JSON") from exc
        raise AssertionError("Airlock retry loop exhausted without returning")

    def available_models(self) -> set[str]:
        """Return the authenticated model aliases currently exposed by Airlock."""
        value = self._request("/v1/models")
        data = value.get("data")
        if not isinstance(data, list):
            raise Answer01Error("Airlock model catalog is invalid")
        return {
            item["id"]
            for item in data
            if isinstance(item, Mapping) and isinstance(item.get("id"), str)
        }

    def complete(
        self,
        role: str,
        model: str,
        messages: Sequence[Mapping[str, str]],
        max_tokens: int,
    ) -> ModelReply:
        """Make one chat completion and retain conservative usage if absent."""
        value = self._request(
            "/v1/chat/completions",
            {
                "model": model,
                "messages": list(messages),
                "max_tokens": max_tokens,
                "response_format": {"type": "json_object"},
            },
        )
        try:
            content = value["choices"][0]["message"]["content"]
        except (KeyError, IndexError, TypeError) as exc:
            raise Answer01Error("Airlock completion response is incomplete") from exc
        if not isinstance(content, str):
            raise Answer01Error("Airlock completion content is not text")
        usage = value.get("usage", {})
        if not isinstance(usage, Mapping):
            usage = {}
        estimated_prompt = math.ceil(
            sum(len(message["content"]) for message in messages) / 3
        )
        prompt_tokens = usage.get("prompt_tokens", estimated_prompt)
        completion_tokens = usage.get("completion_tokens", max_tokens)
        if not isinstance(prompt_tokens, int) or not isinstance(completion_tokens, int):
            raise Answer01Error("Airlock completion usage is invalid")
        self._calls += 1
        print(f"ANSWER-01 live call {self._calls}: {role} complete", flush=True)
        return ModelReply(content, prompt_tokens, completion_tokens)


def _load_external(path: Path, expected_sha: str, label: str) -> object:
    try:
        payload = path.read_bytes()
        value = json.loads(payload)
    except (OSError, json.JSONDecodeError) as exc:
        raise Answer01Error(f"{label} is unavailable or invalid") from exc
    if hashlib.sha256(payload).hexdigest() != expected_sha:
        raise Answer01Error(f"{label} sha256 drifted")
    return value


def run_dry(
    config: Mapping[str, Any],
    *,
    corpus_path: Path,
    subset_path: Path,
    artifact_root: Path,
    fathomdb_bin: str,
    base_dir: Path,
) -> tuple[str, Path, dict[str, Any]]:
    """Execute the approved no-model dry run and write its safe repository receipt."""
    if artifact_root.resolve().is_relative_to(_lib.REPO_ROOT.resolve()):
        raise Answer01Error("ANSWER-01 artifacts must remain outside the repository")
    if artifact_root.exists():
        raise Answer01Error("ANSWER-01 artifact root must be new")
    artifact_root.mkdir(parents=True, mode=0o700)
    corpus = _load_external(corpus_path, config["corpus"]["sha256"], "LOCOMO corpus")
    subset = _load_external(subset_path, config["subset"]["sha256"], "fixed subset")
    questions = select_questions(corpus, subset)
    provider = FathomContextProvider(
        config, corpus, artifact_root=artifact_root, fathomdb_bin=fathomdb_bin
    )
    checkpoint_path = artifact_root / "answer-01-checkpoint.v1.json"
    try:
        summary = score_stub_dry_run(
            config,
            questions,
            retrieve=provider.retrieve,
            checkpoint_path=checkpoint_path,
        )
    finally:
        provider.close()
    artifacts = [
        {"path": "external-answer-01-checkpoint.v1", "sha256": _lib._sha256_file(checkpoint_path)}
    ]
    artifacts.extend(
        {
            "path": f"external-fathomdb-doctor-{index}.v1",
            "sha256": _lib._sha256_file(path),
        }
        for index, path in enumerate(provider.doctor_paths, start=1)
    )
    timestamp = datetime.now(timezone.utc).replace(second=0, microsecond=0)
    run_id, run_dir = _lib.write_record(
        "answer-01-shortlist-dry-run",
        ts=timestamp,
        config_obj=config,
        metrics=summary,
        verdict="complete",
        read="ANSWER-01 two-arm $0 dry-run wiring complete; no answer-quality decision",
        code=_lib.git_info(),
        corpus={
            "source": "LOCOMO",
            "manifest_sha256": config["corpus"]["sha256"],
            "datasets": [],
        },
        seeds={},
        env=_lib.env_info(),
        cost_usd=0.0,
        headline={"program_track": PROGRAM_TRACK, "status": "dry_run_proof"},
        n=len(questions),
        config_path="experiments/configs/answer-01/shortlist-scoring.v1.json",
        artifacts=artifacts,
        base_dir=base_dir,
    )
    _lib.regen_index_md(
        index_path=base_dir / "index.jsonl", md_path=base_dir / "INDEX.md"
    )
    return run_id, run_dir, summary


def run_live(
    config: Mapping[str, Any],
    *,
    corpus_path: Path,
    subset_path: Path,
    dry_checkpoint_path: Path,
    artifact_root: Path,
    client: AirlockClient,
    base_dir: Path,
) -> tuple[str, Path, dict[str, Any]]:
    """Execute the capped live run and write a safe repository receipt."""
    if artifact_root.resolve().is_relative_to(_lib.REPO_ROOT.resolve()):
        raise Answer01Error("ANSWER-01 artifacts must remain outside the repository")
    if artifact_root.exists():
        raise Answer01Error("ANSWER-01 live artifact root must be new")
    required_models = {
        config["live"]["answerer"]["model"],
        config["live"]["judge"]["model"],
    }
    missing_models = required_models - client.available_models()
    if missing_models:
        raise Answer01Error(f"Airlock lacks required model aliases: {sorted(missing_models)}")
    corpus = _load_external(corpus_path, config["corpus"]["sha256"], "LOCOMO corpus")
    subset = _load_external(subset_path, config["subset"]["sha256"], "fixed subset")
    questions = select_questions(corpus, subset)
    artifact_root.mkdir(parents=True, mode=0o700)
    checkpoint_path = artifact_root / "answer-01-live-checkpoint.v1.json"
    summary = score_live(
        config,
        questions,
        dry_checkpoint_path=dry_checkpoint_path,
        checkpoint_path=checkpoint_path,
        complete=client.complete,
    )
    complete = summary["status"] == "complete"
    decision = summary.get("decision", "not_eligible")
    timestamp = datetime.now(timezone.utc).replace(second=0, microsecond=0)
    run_id, run_dir = _lib.write_record(
        "answer-01-shortlist-live",
        ts=timestamp,
        config_obj=config,
        metrics=summary,
        verdict="complete" if complete else "blocked_prerequisite",
        read=f"ANSWER-01 paired live scoring {summary['status']}; decision={decision}",
        code=_lib.git_info(),
        corpus={
            "source": "LOCOMO",
            "manifest_sha256": config["corpus"]["sha256"],
            "datasets": [],
        },
        seeds={},
        env=_lib.env_info(),
        cost_usd=summary["cost_usd"],
        headline={
            "program_track": PROGRAM_TRACK,
            "status": summary["status"],
            "decision": decision,
        },
        n=len(questions),
        config_path="experiments/configs/answer-01/shortlist-scoring.v1.json",
        artifacts=[
            {
                "path": "external-answer-01-live-checkpoint.v1",
                "sha256": _lib._sha256_file(checkpoint_path),
            },
            {
                "path": "external-answer-01-dry-checkpoint.v1",
                "sha256": _lib._sha256_file(dry_checkpoint_path),
            },
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
    dry = commands.add_parser("dry-run")
    dry.add_argument("config", type=Path)
    dry.add_argument("--corpus", required=True, type=Path)
    dry.add_argument("--subset", required=True, type=Path)
    dry.add_argument("--artifact-root", required=True, type=Path)
    dry.add_argument("--fathomdb-bin", required=True)
    dry.add_argument("--base-dir", type=Path, default=Path("experiments"))
    live = commands.add_parser("live-run")
    live.add_argument("config", type=Path)
    live.add_argument("--corpus", required=True, type=Path)
    live.add_argument("--subset", required=True, type=Path)
    live.add_argument("--dry-checkpoint", required=True, type=Path)
    live.add_argument("--artifact-root", required=True, type=Path)
    live.add_argument("--airlock-url", default="http://127.0.0.1:4000")
    live.add_argument("--base-dir", type=Path, default=Path("experiments"))
    args = parser.parse_args(argv)
    try:
        config = load_config(json.loads(args.config.read_text(encoding="utf-8")))
        if args.command == "validate":
            print("ANSWER-01 configuration resolves")
            return 0
        if args.command == "dry-run":
            run_id, run_dir, summary = run_dry(
                config,
                corpus_path=args.corpus,
                subset_path=args.subset,
                artifact_root=args.artifact_root,
                fathomdb_bin=args.fathomdb_bin,
                base_dir=args.base_dir,
            )
        else:
            key = os.environ.get("AIRLOCK_VIRTUAL_KEY") or os.environ.get(
                "AIRLOCK_MASTER_KEY"
            )
            client = AirlockClient(
                args.airlock_url,
                key or "",
                max_retries=config["live"]["max_retries"],
            )
            run_id, run_dir, summary = run_live(
                config,
                corpus_path=args.corpus,
                subset_path=args.subset,
                dry_checkpoint_path=args.dry_checkpoint,
                artifact_root=args.artifact_root,
                client=client,
                base_dir=args.base_dir,
            )
    except (OSError, json.JSONDecodeError, Answer01Error, RuntimeError) as exc:
        parser.error(str(exc))
    print(
        json.dumps(
            {
                "run_id": run_id,
                "receipt": str(run_dir / "record.json"),
                "status": summary["status"],
                "live_calls": summary["live_calls"],
            }
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
