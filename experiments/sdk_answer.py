"""Shared, local answer stage for synthetic public-SDK benchmark fixtures.

Only ``first_sentence_v1`` is supported. It takes the first sentence of the
first retrieved snippet; the pinned prompt records this rule and the question
but does not drive a language model. This stage cannot measure real question
answering. The fixed judge provides exact match, token F1, substring
grounding, and source attribution against explicit answer gold. No SDK call,
database, model service, network access, or paid execution occurs here.

The coordinator must store ``external_samples`` outside safe receipts. It must
also supply the completed retrieval result and ordered-snippet evidence bytes
from the same suite attempt, with their actual SHA-256 digests in ``binding``.
"""

from __future__ import annotations

import hashlib
import json
import re
import time
from collections.abc import Callable
from typing import Any


CONFIG_SCHEMA = "sdk.answer-config.v1"
GOLD_SCHEMA = "sdk.answer-gold.v1"
EVIDENCE_SCHEMA = "sdk.retrieval-evidence.v1"
RESULT_SCHEMA = "sdk.result.v1"
PROMPT = "Return the first sentence of the first context snippet.\nQuestion: {question}\nContext: {context}"
_SHA256 = re.compile(r"[0-9a-f]{64}\Z")
_WORD = re.compile(r"\w+", re.UNICODE)


def _canonical(value: object) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


def artifact_bytes(value: object) -> bytes:
    """Serialize output exactly as the SDK coordinator's JSON writer does."""
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + "\n").encode(
        "utf-8"
    )


def _sha(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _exact(value: object, name: str, fields: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != fields:
        raise ValueError(f"{name} must contain exactly {sorted(fields)}")
    return value


def _positive(value: object, name: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
        raise ValueError(f"{name} must be a positive integer")
    return value


def _name(value: object, name: str) -> str:
    if not isinstance(value, str) or not value:
        raise ValueError(f"{name} must be a nonempty string")
    return value


def _sha_field(value: object, name: str) -> str:
    if not isinstance(value, str) or _SHA256.fullmatch(value) is None:
        raise ValueError(f"{name} must be a lowercase SHA-256")
    return value


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise ValueError(f"duplicate JSON key: {key}")
        value[key] = item
    return value


def _decode(raw: bytes, name: str) -> dict[str, Any]:
    if not isinstance(raw, bytes):
        raise ValueError(f"{name} bytes are required")
    try:
        value = json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_object)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValueError(f"invalid {name} JSON") from exc
    if not isinstance(value, dict):
        raise ValueError(f"{name} must be an object")
    return value


def validate_answer_config(config: object) -> str:
    """Validate a closed local answer contract and return its immutable digest.

    Raises ``ValueError`` for unsupported answerers, judges, prompts, or
    decoding settings. The digest excludes all attempt-specific bindings.
    """
    item = _exact(
        config,
        "answer config",
        {
            "schema_version",
            "upstream",
            "evidence_transform",
            "answerer",
            "prompt",
            "context",
            "decoding",
            "judge",
            "gold_sha256",
            "question_order",
            "cost_ceiling_usd",
        },
    )
    if item["schema_version"] != CONFIG_SCHEMA:
        raise ValueError("unsupported answer config schema")
    upstream = _exact(item["upstream"], "upstream", {"case_id", "arm_relation"})
    _name(upstream["case_id"], "upstream.case_id")
    if upstream["arm_relation"] != "same_sdk":
        raise ValueError("only same-SDK upstream retrieval is supported")
    if item["evidence_transform"] != "ordered_snippets_v1":
        raise ValueError("unsupported evidence transformation")
    if item["answerer"] != "first_sentence_v1":
        raise ValueError("unsupported answerer model")
    if item["prompt"] != PROMPT:
        raise ValueError("unsupported prompt")
    context = _exact(item["context"], "context", {"max_hits", "max_chars"})
    _positive(context["max_hits"], "context.max_hits")
    _positive(context["max_chars"], "context.max_chars")
    decoding = _exact(
        item["decoding"], "decoding", {"temperature", "seed", "max_output_chars"}
    )
    if (
        isinstance(decoding["temperature"], bool)
        or decoding["temperature"] != 0
        or isinstance(decoding["seed"], bool)
        or decoding["seed"] != 0
    ):
        raise ValueError("fixed answerer requires temperature=0 and seed=0")
    _positive(decoding["max_output_chars"], "decoding.max_output_chars")
    if item["judge"] != "exact_token_f1_grounding_v1":
        raise ValueError("unsupported judge")
    _sha_field(item["gold_sha256"], "gold_sha256")
    order = item["question_order"]
    if (
        not isinstance(order, list)
        or not order
        or any(not isinstance(x, str) or not x for x in order)
        or len(set(order)) != len(order)
    ):
        raise ValueError("question_order must have distinct nonempty IDs")
    if isinstance(item["cost_ceiling_usd"], bool) or item["cost_ceiling_usd"] != 0:
        raise ValueError("the local answerer requires a zero cost ceiling")
    return _sha(_canonical(item))


def assert_pairable(left: object, right: object) -> None:
    """Reject any answer contract drift across arms in a paired comparison."""
    if validate_answer_config(left) != validate_answer_config(right):
        raise ValueError(
            "answerer, prompt, context, decoding, judge, gold, or question order mismatch"
        )


def _gold(raw: bytes, config: dict[str, Any]) -> list[dict[str, Any]]:
    if _sha(raw) != config["gold_sha256"]:
        raise ValueError("answer gold digest mismatch")
    value = _exact(
        _decode(raw, "answer gold"), "answer gold", {"schema_version", "questions"}
    )
    if value["schema_version"] != GOLD_SCHEMA or not isinstance(
        value["questions"], list
    ):
        raise ValueError("answer gold schema mismatch")
    questions = value["questions"]
    if [q.get("id") for q in questions if isinstance(q, dict)] != config[
        "question_order"
    ]:
        raise ValueError("answer gold question order mismatch")
    for question in questions:
        q = _exact(
            question,
            "answer gold question",
            {"id", "question", "answer", "source_ids", "labels"},
        )
        _name(q["question"], "answer gold question text")
        _name(q["answer"], "answer gold answer")
        if (
            not isinstance(q["source_ids"], list)
            or not q["source_ids"]
            or any(not isinstance(s, str) or not s for s in q["source_ids"])
        ):
            raise ValueError("answer gold source IDs required")
        if not isinstance(q["labels"], list) or any(
            not isinstance(label, str) or not label for label in q["labels"]
        ):
            raise ValueError("answer gold labels invalid")
    return questions


def _retrieval(
    result_raw: bytes,
    evidence_raw: bytes,
    binding: dict[str, Any],
    config: dict[str, Any],
    suite_attempt_id: str,
    sdk: str,
    order: list[str],
) -> dict[str, list[dict[str, str]]]:
    bound = _exact(
        binding,
        "upstream binding",
        {
            "suite_attempt_id",
            "case_id",
            "sdk",
            "retrieval_result_sha256",
            "evidence_sha256",
        },
    )
    if (
        bound["suite_attempt_id"] != suite_attempt_id
        or bound["case_id"] != config["upstream"]["case_id"]
        or bound["sdk"] != sdk
    ):
        raise ValueError("upstream binding is not this attempt's declared SDK arm")
    for key, raw in (
        ("retrieval_result_sha256", result_raw),
        ("evidence_sha256", evidence_raw),
    ):
        if _sha_field(bound[key], key) != _sha(raw):
            raise ValueError(f"{key} does not match supplied bytes")
    result = _decode(result_raw, "retrieval result")
    evidence = _decode(evidence_raw, "retrieval evidence")
    for value, schema, name in (
        (result, RESULT_SCHEMA, "retrieval result"),
        (evidence, EVIDENCE_SCHEMA, "retrieval evidence"),
    ):
        if (
            value.get("schema_version") != schema
            or value.get("suite_attempt_id") != suite_attempt_id
            or value.get("case_id") != bound["case_id"]
            or value.get("sdk") != sdk
        ):
            raise ValueError(f"{name} identity mismatch")
    if result.get("family") != "retrieval" or result.get("status") != "complete":
        raise ValueError("upstream retrieval must be complete")
    ordered = result.get("ordered_ids")
    if not isinstance(ordered, dict) or set(ordered) != set(order):
        raise ValueError("upstream retrieval question set mismatch")
    entries = evidence.get("questions")
    if (
        not isinstance(entries, list)
        or [x.get("id") for x in entries if isinstance(x, dict)] != order
    ):
        raise ValueError("retrieval evidence question order mismatch")
    hits_by_question: dict[str, list[dict[str, str]]] = {}
    for entry in entries:
        row = _exact(entry, "retrieval evidence question", {"id", "hits"})
        hits = row["hits"]
        if not isinstance(hits, list):
            raise ValueError("retrieval evidence hits must be a list")
        validated = []
        for hit in hits:
            item = _exact(hit, "retrieval hit", {"id", "source_id", "text"})
            for key in item:
                _name(item[key], f"retrieval hit {key}")
            validated.append(item)
        ids = [hit["id"] for hit in validated]
        if len(set(ids)) != len(ids) or ordered[row["id"]] != ids:
            raise ValueError("retrieval evidence ordered IDs mismatch")
        hits_by_question[row["id"]] = validated
    return hits_by_question


def _normal(value: str) -> str:
    return " ".join(_WORD.findall(value.casefold()))


def _f1(answer: str, gold: str) -> float:
    from collections import Counter

    left = Counter(_WORD.findall(answer.casefold()))
    right = Counter(_WORD.findall(gold.casefold()))
    overlap = sum((left & right).values())
    return (
        2 * overlap / (sum(left.values()) + sum(right.values()))
        if left or right
        else 1.0
    )


def _answer(
    hits: list[dict[str, str]], max_hits: int, max_chars: int, max_output_chars: int
) -> tuple[str, str | None, str]:
    selected = hits[:max_hits]
    context = "\n".join(hit["text"] for hit in selected)[:max_chars]
    if not context:
        return "", None, context
    first = re.split(r"(?<=[.!?])\s+|\n", context, maxsplit=1)[0].strip()
    answer = first[:max_output_chars]
    return answer, selected[0]["source_id"] if selected else None, context


def run_answer_case(
    *,
    config: dict[str, Any],
    gold_bytes: bytes,
    retrieval_result_bytes: bytes,
    evidence_bytes: bytes,
    binding: dict[str, Any],
    suite_id: str,
    suite_attempt_id: str,
    sdk: str,
    case_id: str,
    clock_ns: Callable[[], int] = time.perf_counter_ns,
) -> dict[str, Any]:
    """Score one answer cell and return aggregate result, witness, and raw samples.

    The caller persists the result and witness as typed external artifacts and
    places raw question/answer samples only in its restricted artifact root.
    Raises ``ValueError`` before scoring for invalid gold or dependency data.
    """
    config_sha = validate_answer_config(config)
    for value, name in (
        (suite_id, "suite_id"),
        (suite_attempt_id, "suite_attempt_id"),
        (sdk, "sdk"),
        (case_id, "case_id"),
    ):
        _name(value, name)
    if case_id == config["upstream"]["case_id"]:
        raise ValueError("answer case cannot be its own retrieval dependency")
    questions = _gold(gold_bytes, config)
    hits = _retrieval(
        retrieval_result_bytes,
        evidence_bytes,
        binding,
        config,
        suite_attempt_id,
        sdk,
        config["question_order"],
    )
    samples: list[dict[str, Any]] = []
    answer_ns = judge_ns = 0
    for question in questions:
        start = clock_ns()
        answer, source_id, context = _answer(
            hits[question["id"]],
            config["context"]["max_hits"],
            config["context"]["max_chars"],
            config["decoding"]["max_output_chars"],
        )
        end = clock_ns()
        if end < start:
            raise ValueError("answer clock moved backward")
        answer_ns += end - start
        start = clock_ns()
        exact = float(_normal(answer) == _normal(question["answer"]))
        f1 = _f1(answer, question["answer"])
        grounded = float(bool(answer) and _normal(answer) in _normal(context))
        attributed = float(source_id in question["source_ids"])
        end = clock_ns()
        if end < start:
            raise ValueError("judge clock moved backward")
        judge_ns += end - start
        samples.append(
            {
                "question_id": question["id"],
                "answer": answer,
                "source_id": source_id,
                "exact": exact,
                "f1": f1,
                "grounded": grounded,
                "attributed": attributed,
                "labels": question["labels"],
            }
        )
    count = len(samples)
    metrics = {
        "answer_accuracy": sum(s["exact"] for s in samples) / count,
        "answer_f1": sum(s["f1"] for s in samples) / count,
        "groundedness": sum(s["grounded"] for s in samples) / count,
        "attribution": sum(s["attributed"] for s in samples) / count,
        "cost_usd": 0.0,
        "answer_ms": answer_ns / 1_000_000,
        "judge_ms": judge_ns / 1_000_000,
    }
    labels = sorted({label for sample in samples for label in sample["labels"]})
    slices = {
        label: {
            "n": sum(label in s["labels"] for s in samples),
            "answer_accuracy": sum(s["exact"] for s in samples if label in s["labels"])
            / sum(label in s["labels"] for s in samples),
        }
        for label in labels
    }
    upstream = dict(binding)
    result = {
        "schema_version": RESULT_SCHEMA,
        "suite_id": suite_id,
        "suite_attempt_id": suite_attempt_id,
        "case_id": case_id,
        "sdk": sdk,
        "axis": "sdk",
        "family": "answer",
        "producer": "answer_pipeline",
        "operation_id": "answer_pipeline.first_sentence_v1",
        "operation_version": 1,
        "status": "complete",
        "reason": None,
        "config_sha256": config_sha,
        "upstream": upstream,
        "metrics": metrics,
        "slices": slices,
        "sample_count": count,
        "external_samples_sha256": _sha(artifact_bytes(samples)),
    }
    witness = {
        "schema_version": "sdk.answer-execution.v1",
        "suite_id": suite_id,
        "suite_attempt_id": suite_attempt_id,
        "case_id": case_id,
        "sdk": sdk,
        "config_sha256": config_sha,
        "upstream": upstream,
        "result_sha256": _sha(artifact_bytes(result)),
    }
    return {
        "config_sha256": config_sha,
        "result": result,
        "witness": witness,
        "external_samples": samples,
    }
