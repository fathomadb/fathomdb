"""Paid, checkpointed GLOBAL-01 lazy-coverage witness and held-out run."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import random
import re
import statistics
import time
import urllib.error
import urllib.request
from dataclasses import asdict, dataclass, field
from datetime import datetime, timedelta, timezone
from email.utils import parsedate_to_datetime
from pathlib import Path
from typing import Any, Mapping, Sequence

from experiments import _lib, global_01_lazy
from experiments.fathomdb_test_setup import prepare_test_database


CHECKPOINT_SCHEMA = "global-01.lazy-checkpoint.v1"
RESULT_SCHEMA = "global-01.lazy-result.v1"
SEMANTIC_REVISION = "v9-compact-reduction"
METRICS = ("comprehensiveness", "diversity", "empowerment", "directness")
HEADLINE_METRICS = METRICS[:3]
OUTPUT_LIMIT_ERROR = "response reached the output token limit"
_JSON_FENCE = re.compile(r"^```(?:json)?\s*|\s*```$", re.IGNORECASE)


class Global01LazyLiveError(RuntimeError):
    """Raised when paid GLOBAL-01 execution cannot preserve its contract."""


def _canonical_sha256(value: object) -> str:
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def retry_delay(
    headers: Mapping[str, str],
    *,
    fallback: float,
    now: datetime | None = None,
) -> float:
    """Honor numeric or HTTP-date Retry-After before exponential fallback."""
    value = headers.get("Retry-After")
    if value is not None:
        try:
            return max(0.0, float(value))
        except ValueError:
            try:
                parsed = parsedate_to_datetime(value)
                reference = now or datetime.now(timezone.utc)
                return max(0.0, (parsed - reference).total_seconds())
            except (TypeError, ValueError, OverflowError):
                pass
    return max(0.0, fallback)


@dataclass
class LazyRunState:
    """Incrementally persisted paid cells and spend for idempotent resume."""

    schema_version: str
    config_sha256: str
    cost_cap_usd: float
    started_at: str
    cost_usd: float = 0.0
    cells: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def new(cls, config_sha256: str, cost_cap_usd: float) -> LazyRunState:
        started = datetime.now(timezone.utc).replace(second=0, microsecond=0)
        return cls(CHECKPOINT_SCHEMA, config_sha256, cost_cap_usd, started.isoformat())

    @classmethod
    def load(
        cls, path: Path, config_sha256: str, cost_cap_usd: float
    ) -> LazyRunState:
        value = json.loads(path.read_text(encoding="utf-8"))
        if (
            value.get("schema_version") != CHECKPOINT_SCHEMA
            or value.get("config_sha256") != config_sha256
            or value.get("cost_cap_usd") != cost_cap_usd
        ):
            raise Global01LazyLiveError("checkpoint configuration or cost cap drifted")
        return cls(**value)

    @property
    def remaining_cost_usd(self) -> float:
        return self.cost_cap_usd - self.cost_usd

    def complete(self, cell: str, value: object, *, cost_usd: float = 0.0) -> None:
        if cell in self.cells:
            raise Global01LazyLiveError(f"cell already completed: {cell}")
        next_cost = self.cost_usd + cost_usd
        if next_cost > self.cost_cap_usd + 1e-9:
            raise Global01LazyLiveError("GLOBAL-01 cost cap would be exceeded")
        self.cost_usd = next_cost
        self.cells[cell] = value

    def missing(self, cells: Sequence[str]) -> list[str]:
        return [cell for cell in cells if cell not in self.cells]

    def save(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary = path.with_suffix(path.suffix + ".tmp")
        temporary.write_text(
            json.dumps(asdict(self), indent=2) + "\n", encoding="utf-8"
        )
        temporary.chmod(0o600)
        os.replace(temporary, path)


def assert_cells_complete(state: LazyRunState, required: Sequence[str]) -> None:
    """Refuse a result whenever any registered cell remains incomplete."""
    missing = state.missing(required)
    if missing:
        raise Global01LazyLiveError(
            f"run is incomplete; {len(missing)} required cells are missing"
        )


def validate_safe_preflight(
    config: Mapping[str, Any], report_path: Path
) -> dict[str, Any]:
    """Bind paid execution to a passing, same-configuration zero-spend receipt."""
    report = json.loads(report_path.read_text(encoding="utf-8"))
    lifecycle = report.get("lifecycle", {})
    if (
        report.get("schema_version") != "global-01.lazy-preflight.v1"
        or report.get("state") != "ready_for_hitl"
        or report.get("config_sha256") != _canonical_sha256(config)
        or report.get("cost_usd") != 0.0
        or lifecycle.get("strict_current_supersession") != "pass"
        or lifecycle.get("temporal_failures") != 0
        or lifecycle.get("erasure") != "pass"
        or lifecycle.get("derived_rows_written") != 0
    ):
        raise Global01LazyLiveError("safe preflight is absent, stale, or failed")
    return report


def _usage_cost(config: Mapping[str, Any], model: str, usage: Mapping[str, Any]) -> float:
    pricing = config["pricing"][model]
    return (
        int(usage["prompt_tokens"]) * pricing["input_per_million"] / 1_000_000
        + int(usage["completion_tokens"])
        * pricing["output_per_million"]
        / 1_000_000
    )


def _maximum_request_cost(
    config: Mapping[str, Any], model: str, prompt: str, max_tokens: int
) -> float:
    """Use UTF-8 bytes as a conservative upper bound on input tokens."""
    pricing = config["pricing"][model]
    return (
        len(prompt.encode("utf-8")) * pricing["input_per_million"] / 1_000_000
        + max_tokens * pricing["output_per_million"] / 1_000_000
    )


class AirlockClient:
    """Serialized OpenAI-compatible caller with full provider backoff."""

    def __init__(self, base_url: str, key: str, config: Mapping[str, Any]) -> None:
        self.url = f"{base_url.rstrip('/')}/v1/chat/completions"
        self.key = key
        self.config = config

    def complete(
        self,
        model: str,
        prompt: str,
        *,
        max_tokens: int,
        temperature: float,
        remaining_cost_usd: float,
    ) -> tuple[str, dict[str, Any], float, float]:
        reservation = _maximum_request_cost(
            self.config, model, prompt, max_tokens
        )
        if reservation > remaining_cost_usd + 1e-9:
            raise Global01LazyLiveError(
                "GLOBAL-01 cost cap cannot reserve the next request"
            )
        body: dict[str, Any] = {
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": max_tokens,
            "temperature": temperature,
            "response_format": {"type": "json_object"},
        }
        if model == self.config["models"]["generator"]["model"]:
            body["thinking"] = {
                "type": self.config["models"]["generator"]["thinking_mode"]
            }
        attempts = self.config["execution"]["retry_attempts"]
        backoff = self.config["execution"]["retry_backoff_seconds"]
        started = time.perf_counter()
        for attempt in range(attempts):
            request = urllib.request.Request(
                self.url,
                data=json.dumps(body).encode("utf-8"),
                headers={
                    "Authorization": f"Bearer {self.key}",
                    "Content-Type": "application/json",
                },
            )
            try:
                with urllib.request.urlopen(request, timeout=330) as response:
                    payload = json.load(response)
                choice = payload["choices"][0]
                content = choice["message"]["content"] or ""
                usage = {
                    "prompt_tokens": int(
                        payload.get("usage", {}).get("prompt_tokens", 0)
                    ),
                    "completion_tokens": int(
                        payload.get("usage", {}).get("completion_tokens", 0)
                    ),
                    "finish_reason": str(choice.get("finish_reason") or ""),
                }
                if (
                    not content.strip()
                    or usage["prompt_tokens"] <= 0
                    or usage["completion_tokens"] <= 0
                ):
                    raise Global01LazyLiveError(
                        f"{model} returned empty content or missing usage"
                    )
                return (
                    content,
                    usage,
                    _usage_cost(self.config, model, usage),
                    (time.perf_counter() - started) * 1000,
                )
            except urllib.error.HTTPError as exc:
                if exc.code != 429 and exc.code < 500:
                    raise Global01LazyLiveError(
                        f"Airlock returned HTTP {exc.code}"
                    ) from exc
                if attempt + 1 == attempts:
                    raise Global01LazyLiveError(
                        "Airlock retry budget exhausted"
                    ) from exc
                time.sleep(
                    retry_delay(
                        dict(exc.headers.items()), fallback=float(backoff[attempt])
                    )
                )
            except (TimeoutError, urllib.error.URLError) as exc:
                if attempt + 1 == attempts:
                    raise Global01LazyLiveError(
                        "Airlock timeout retry budget exhausted"
                    ) from exc
                time.sleep(float(backoff[attempt]))
        raise AssertionError("unreachable")


def _json_object(text: str) -> dict[str, Any]:
    stripped = _JSON_FENCE.sub("", text.strip()).strip()
    try:
        value = json.loads(stripped)
    except json.JSONDecodeError as exc:
        match = re.search(r"\{.*\}", stripped, re.DOTALL)
        if match is None:
            raise Global01LazyLiveError("model response is not JSON") from exc
        value = json.loads(match.group(0))
    if not isinstance(value, dict):
        raise Global01LazyLiveError("model response is not a JSON object")
    return value


def parse_subqueries(text: str, *, count: int) -> list[str]:
    """Require exactly the registered number of unique bounded subqueries."""
    value = _json_object(text)
    queries = value.get("subqueries")
    if (
        not isinstance(queries, list)
        or len(queries) != count
        or any(not isinstance(item, str) or not item.strip() for item in queries)
    ):
        raise Global01LazyLiveError("decomposition did not return exact subqueries")
    normalized = [" ".join(item.split()) for item in queries]
    if len({item.casefold() for item in normalized}) != count:
        raise Global01LazyLiveError("decomposition subqueries are not unique")
    return normalized


def _validate_mapped_claims(
    value: object,
    *,
    known_source_refs: Mapping[str, Mapping[str, str]],
    prefix: str,
    max_claims: int,
    max_words: int,
) -> list[dict[str, Any]]:
    if not isinstance(value, dict) or set(value) != {"claims"}:
        raise Global01LazyLiveError("claim map must contain only claims")
    claims = value["claims"]
    if not isinstance(claims, list):
        raise Global01LazyLiveError("claim map claims are not a list")
    claims = claims[:max_claims]
    known_sources = {
        (source["source_id"], source["content_sha256"])
        for source in known_source_refs.values()
    }
    result = []
    for ordinal, claim in enumerate(claims):
        if not isinstance(claim, dict) or set(claim) not in (
            {"text", "source_refs"},
            {"text", "sources"},
        ):
            raise Global01LazyLiveError("mapped claim shape is invalid")
        if not isinstance(claim["text"], str) or not claim["text"].strip():
            raise Global01LazyLiveError("mapped claim text is empty")
        if len(claim["text"].split()) > max_words:
            raise Global01LazyLiveError("mapped claim text exceeds word limit")
        sources: list[dict[str, str]] = []
        if "source_refs" in claim:
            source_refs = claim["source_refs"]
            if (
                not isinstance(source_refs, list)
                or not source_refs
                or any(not isinstance(source_ref, str) for source_ref in source_refs)
                or len(source_refs) != len(set(source_refs))
            ):
                raise Global01LazyLiveError("mapped claim lacks canonical sources")
            for source_ref in source_refs:
                source = known_source_refs.get(source_ref)
                if source is None:
                    raise Global01LazyLiveError(
                        "mapped claim source binding drifted"
                    )
                sources.append(dict(source))
        else:
            canonical_sources = claim["sources"]
            if not isinstance(canonical_sources, list) or not canonical_sources:
                raise Global01LazyLiveError("mapped claim lacks canonical sources")
            seen_sources: set[tuple[str, str]] = set()
            for source in canonical_sources:
                if not isinstance(source, dict) or set(source) != {
                    "source_id",
                    "content_sha256",
                }:
                    raise Global01LazyLiveError(
                        "mapped claim source binding drifted"
                    )
                identity = (source["source_id"], source["content_sha256"])
                if identity not in known_sources or identity in seen_sources:
                    raise Global01LazyLiveError(
                        "mapped claim source binding drifted"
                    )
                seen_sources.add(identity)
                sources.append(dict(source))
        result.append(
            {
                "claim_id": f"{prefix}-{ordinal:03d}",
                "text": claim["text"].strip(),
                "sources": sources,
            }
        )
    return result


def validate_assertion_score(
    value: object, *, answer: Mapping[str, Any], assertion_count: int
) -> dict[str, Any]:
    """Require valid assertion indices and exactly one score per final claim."""
    if not isinstance(value, dict) or set(value) != {
        "passed_assertion_indices",
        "claim_support",
    }:
        raise Global01LazyLiveError("assertion score shape is invalid")
    indices = value["passed_assertion_indices"]
    if (
        not isinstance(indices, list)
        or any(not isinstance(index, int) for index in indices)
        or len(set(indices)) != len(indices)
        or any(index < 0 or index >= assertion_count for index in indices)
    ):
        raise Global01LazyLiveError("assertion score indices are invalid")
    support = value["claim_support"]
    final_ids = {claim["claim_id"] for claim in answer["claims"]}
    if not isinstance(support, list):
        raise Global01LazyLiveError("claim support is not a list")
    support_ids = set()
    for row in support:
        if (
            not isinstance(row, dict)
            or set(row) != {"claim_id", "supported"}
            or row["claim_id"] in support_ids
            or not isinstance(row["supported"], bool)
        ):
            raise Global01LazyLiveError("claim support row is invalid")
        support_ids.add(row["claim_id"])
    if support_ids != final_ids:
        raise Global01LazyLiveError("claim support does not cover all final claims")
    return json.loads(json.dumps(value))


def _pairwise_prompt(question: str, answer_a: str, answer_b: str) -> str:
    return (
        "Judge two answers independently on four dimensions. Comprehensiveness "
        "means coverage of relevant scope; diversity means varied relevant "
        "detail; empowerment means support for informed reasoning; directness "
        "means concise focus and must not reward verbosity.\n\n"
        f"QUESTION:\n{question}\n\nANSWER A:\n{answer_a}\n\n"
        f"ANSWER B:\n{answer_b}\n\n"
        'Return JSON only with keys "comprehensiveness", "diversity", '
        '"empowerment", and "directness". Each value must be "A", "B", or '
        '"tie".'
    )


def _parse_judgment(value: object) -> dict[str, str]:
    if (
        not isinstance(value, dict)
        or set(value) != set(METRICS)
        or any(value[metric] not in {"A", "B", "tie"} for metric in METRICS)
    ):
        raise Global01LazyLiveError("judge response is incomplete")
    return {metric: value[metric] for metric in METRICS}


def _complete_json_cell(
    client: AirlockClient,
    *,
    state: LazyRunState,
    checkpoint_path: Path,
    cell: str,
    model: str,
    prompt: str,
    max_tokens: int,
    temperature: float,
    validator: Any,
) -> dict[str, Any]:
    if cell in state.cells:
        return state.cells[cell]["value"]
    validation_error: str | None = None
    for attempt in range(3):
        invalid = f"invalid/{SEMANTIC_REVISION}/{cell}/{attempt}"
        if invalid in state.cells:
            validation_error = state.cells[invalid]["semantic_failure"]["error"]
            continue
        attempt_prompt = prompt
        if validation_error is not None:
            attempt_prompt += (
                "\n\nVALIDATION CORRECTION: The previous response failed because: "
                f"{validation_error}. Return a corrected JSON object that satisfies "
                "the original instructions. Do not repeat the invalid response."
            )
            if validation_error == OUTPUT_LIMIT_ERROR:
                attempt_prompt += (
                    " Fit the complete JSON within the output limit: shorten prose "
                    "while retaining every required schema entry."
                )
        response, usage, cost, latency_ms = client.complete(
            model,
            attempt_prompt,
            max_tokens=max_tokens,
            temperature=temperature,
            remaining_cost_usd=state.remaining_cost_usd,
        )
        raw: dict[str, Any] | None = None
        try:
            raw = _json_object(response)
            parsed = validator(raw)
        except (
            Global01LazyLiveError,
            global_01_lazy.Global01LazyError,
            json.JSONDecodeError,
            KeyError,
            TypeError,
        ) as exc:
            failure_metadata = semantic_failure_metadata(raw, exc)
            if (
                usage.get("finish_reason") == "length"
                or usage["completion_tokens"] >= max_tokens
            ):
                failure_metadata["error"] = OUTPUT_LIMIT_ERROR
            state.complete(
                invalid,
                {
                    "response_sha256": hashlib.sha256(
                        response.encode("utf-8")
                    ).hexdigest(),
                    "usage": usage,
                    "latency_ms": latency_ms,
                    "semantic_failure": failure_metadata,
                },
                cost_usd=cost,
            )
            state.save(checkpoint_path)
            validation_error = failure_metadata["error"]
            continue
        state.complete(
            cell,
            {"value": parsed, "usage": usage, "latency_ms": latency_ms},
            cost_usd=cost,
        )
        state.save(checkpoint_path)
        return parsed
    raise Global01LazyLiveError(f"semantic retry budget exhausted for {cell}")


def semantic_failure_metadata(
    value: Mapping[str, Any] | None,
    failure: Exception,
) -> dict[str, Any]:
    """Describe a semantic failure without retaining response content."""
    message = str(failure)
    result: dict[str, Any] = {
        "error": message if len(message) <= 120 else type(failure).__name__,
        "top_level_keys": sorted(value) if isinstance(value, Mapping) else [],
    }
    claims = value.get("claims") if isinstance(value, Mapping) else None
    if isinstance(claims, list):
        result["claim_count"] = len(claims)
        result["claim_key_sets"] = [
            sorted(claim) if isinstance(claim, Mapping) else [] for claim in claims
        ]
    return result


def _load_aa_answers(config: Mapping[str, Any], repository_root: Path) -> list[dict[str, str]]:
    aa = config["evaluation"]["aa"]
    checkpoint_path = repository_root / aa["checkpoint_path"]
    if global_01_lazy.file_sha256(checkpoint_path) != aa["checkpoint_sha256"]:
        raise Global01LazyLiveError("A/A checkpoint identity drifted")
    checkpoint = json.loads(checkpoint_path.read_text(encoding="utf-8"))
    first_manifest = json.loads(
        (
            repository_root
            / config["questions"]["split"]["first_run_private_manifest"]
        ).read_text(encoding="utf-8")
    )
    question_ids = [row["question_id"] for row in first_manifest["questions"]]
    by_arm = {
        "graphrag": [
            checkpoint["cells"][f"answers/graphrag/{qid}"]["answer"]
            for qid in question_ids
        ],
        "fathomdb": [
            checkpoint["cells"][f"answers/fathomdb/{qid}/reduce"]["answer"]
            for qid in question_ids
        ],
    }
    manifest = {
        arm: [hashlib.sha256(answer.encode("utf-8")).hexdigest() for answer in answers]
        for arm, answers in by_arm.items()
    }
    if _canonical_sha256(manifest) != aa["answer_manifest_sha256"]:
        raise Global01LazyLiveError("A/A answer manifest drifted")
    result = []
    for arm, answers in by_arm.items():
        for ordinal, answer in enumerate(answers):
            result.append(
                {
                    "answer_id": f"{arm}-{ordinal:02d}",
                    "answer": answer,
                    "question": first_manifest["questions"][ordinal]["text"],
                }
            )
    if len(result) != aa["answer_count"]:
        raise Global01LazyLiveError("A/A answer count drifted")
    return result


def _run_aa(
    config: Mapping[str, Any],
    repository_root: Path,
    client: AirlockClient,
    state: LazyRunState,
    checkpoint_path: Path,
) -> dict[str, Any]:
    answers = _load_aa_answers(config, repository_root)
    model = config["models"]["pairwise_judge"]
    required = []
    for row in answers:
        for order in ("ab", "ba"):
            cell = f"aa/{row['answer_id']}/0/{order}"
            required.append(cell)
            if cell in state.cells:
                continue
            prompt = _pairwise_prompt(row["question"], row["answer"], row["answer"])
            _complete_json_cell(
                client,
                state=state,
                checkpoint_path=checkpoint_path,
                cell=cell,
                model=model["model"],
                prompt=prompt,
                max_tokens=400,
                temperature=model["temperature"],
                validator=_parse_judgment,
            )
    assert_cells_complete(state, required)
    verdicts = [
        state.cells[cell]["value"][metric]
        for cell in required
        for metric in METRICS
    ]
    tie_rate = verdicts.count("tie") / len(verdicts)
    side_preference = max(verdicts.count("A"), verdicts.count("B")) / len(verdicts)
    passed = (
        tie_rate >= config["evaluation"]["aa"]["tie_rate_min"]
        and side_preference <= config["evaluation"]["aa"]["side_preference_max"]
    )
    result = {
        "answer_count": len(answers),
        "judgment_count": len(verdicts),
        "tie_rate": tie_rate,
        "maximum_side_preference": side_preference,
        "passed": passed,
    }
    if not passed:
        raise Global01LazyLiveError("A/A judge validity boundary failed")
    if "gate/aa" not in state.cells:
        state.complete("gate/aa", result)
        state.save(checkpoint_path)
    return result


def _source_context(rows: Sequence[Mapping[str, Any]]) -> str:
    return "\n\n".join(
        f"SOURCE_ID={row['source_id']}\nCONTENT_SHA256={row['content_sha256']}\n"
        f"TEXT={row['body']}"
        for row in rows
    )


def _map_source_context(rows: Sequence[Mapping[str, Any]]) -> str:
    return "\n\n".join(
        f"SOURCE_REF=S{ordinal}\nSOURCE_ID={row['source_id']}\n"
        f"CONTENT_SHA256={row['content_sha256']}\nTEXT={row['body']}"
        for ordinal, row in enumerate(rows)
    )


def best_source_excerpt(body: str, claim: str, *, max_chars: int) -> str:
    """Select a bounded deterministic source window with maximum claim overlap."""
    if max_chars <= 0:
        raise Global01LazyLiveError("source excerpt boundary must be positive")
    if len(body) <= max_chars:
        return body
    terms = {term for term in re.findall(r"[a-z0-9]+", claim.casefold()) if len(term) >= 3}
    step = max(1, max_chars // 4)
    candidates = []
    for start in range(0, len(body), step):
        window = body[start : start + max_chars]
        window_terms = set(re.findall(r"[a-z0-9]+", window.casefold()))
        candidates.append((len(terms & window_terms), -start, window))
        if start + max_chars >= len(body):
            break
    return max(candidates)[2]


def _map_prompt(
    question: str,
    rows: Sequence[Mapping[str, Any]],
    *,
    max_claims: int,
) -> str:
    return (
        "Extract only source-grounded claims relevant to the question. Cite claims "
        "only with supplied SOURCE_REF values; the caller resolves them to the "
        "shown canonical source ID and hash. An empty "
        f"claim list is valid. Return at most {max_claims} claims. Keep each claim "
        "to at most 30 words (counted as whitespace-separated tokens) and cite "
        "only the minimum sufficient supplied "
        "sources.\n\n"
        f"QUESTION:\n{question}\n\nSOURCES:\n{_map_source_context(rows)}\n\n"
        'Return compact JSON only: {"claims":[{"text":"...",'
        '"source_refs":["S0"]}]}.'
    )


def _reduce_prompt(
    question: str, claims: Sequence[Mapping[str, Any]]
) -> tuple[str, dict[str, dict[str, str]], dict[str, str]]:
    source_refs: dict[str, dict[str, str]] = {}
    source_ref_by_identity: dict[tuple[str, str], str] = {}
    mapped_refs: dict[str, str] = {}
    compact_claims = []
    for ordinal, claim in enumerate(claims):
        mapped_ref = f"M{ordinal}"
        mapped_refs[mapped_ref] = claim["claim_id"]
        claim_source_refs = []
        for source in claim["sources"]:
            identity = (source["source_id"], source["content_sha256"])
            source_ref = source_ref_by_identity.get(identity)
            if source_ref is None:
                source_ref = f"S{len(source_refs)}"
                source_ref_by_identity[identity] = source_ref
                source_refs[source_ref] = dict(source)
            claim_source_refs.append(source_ref)
        compact_claims.append(
            {
                "mapped_claim_ref": mapped_ref,
                "text": claim["text"],
                "source_refs": claim_source_refs,
            }
        )
    prompt = (
        "Synthesize a direct, comprehensive global answer using only the mapped "
        "claims. Cite every final claim with supplied SOURCE_REF values. Account "
        "for every mapped claim exactly once in the coverage ledger. Allowed "
        "dispositions: included, redundant, irrelevant, conflicting-or-uncertain, "
        "omitted-for-budget.\n\n"
        f"QUESTION:\n{question}\n\nMAPPED CLAIMS:\n"
        f"{json.dumps(compact_claims, separators=(',', ':'))}\n\n"
        'Return compact JSON only: {"answer":"...","claims":'
        '[{"claim_ref":"F0","text":"...","source_refs":["S0"]}],'
        '"coverage_ledger":[{"mapped_claim_ref":"M0",'
        '"disposition":"included","final_claim_refs":["F0"]}]}. '
        "Use short claim_ref values and return complete JSON within the limit."
    )
    return prompt, source_refs, mapped_refs


def _legacy_reduce_prompt(
    question: str, claims: Sequence[Mapping[str, Any]]
) -> str:
    return (
        "Synthesize a direct, comprehensive global answer using only the mapped "
        "claims. Preserve canonical citations on every final claim. Account for "
        "every mapped claim exactly once in the coverage ledger. Allowed "
        "dispositions: included, redundant, irrelevant, conflicting-or-uncertain, "
        "omitted-for-budget.\n\n"
        f"QUESTION:\n{question}\n\nMAPPED CLAIMS:\n"
        f"{json.dumps(list(claims), separators=(',', ':'))}\n\n"
        'Return JSON only: {"answer":"...","claims":[{"claim_id":"final-1",'
        '"text":"...","sources":[{"source_id":"...","content_sha256":"..."}]}],'
        '"coverage_ledger":[{"mapped_claim_id":"...","disposition":"included",'
        '"final_claim_ids":["final-1"]}]}. '
    )


def _validate_compact_reduction(
    value: object,
    *,
    known_source_refs: Mapping[str, Mapping[str, str]],
    known_mapped_refs: Mapping[str, str],
) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != {
        "answer",
        "claims",
        "coverage_ledger",
    }:
        raise Global01LazyLiveError("compact reduction keys are incomplete")
    if not isinstance(value["answer"], str) or not value["answer"].strip():
        raise Global01LazyLiveError("compact reduction answer is empty")
    claims = value["claims"]
    if not isinstance(claims, list) or not claims:
        raise Global01LazyLiveError("compact reduction has no claims")
    final_ref_to_id: dict[str, str] = {}
    canonical_claims = []
    for ordinal, claim in enumerate(claims):
        if not isinstance(claim, dict) or set(claim) != {
            "claim_ref",
            "text",
            "source_refs",
        }:
            raise Global01LazyLiveError("compact final claim keys are incomplete")
        claim_ref = claim["claim_ref"]
        if (
            not isinstance(claim_ref, str)
            or not claim_ref
            or claim_ref in final_ref_to_id
        ):
            raise Global01LazyLiveError("compact final claim reference is invalid")
        refs = claim["source_refs"]
        if (
            not isinstance(claim["text"], str)
            or not claim["text"].strip()
            or not isinstance(refs, list)
            or not refs
            or any(
                not isinstance(ref, str) or ref not in known_source_refs
                for ref in refs
            )
            or len(refs) != len(set(refs))
        ):
            raise Global01LazyLiveError("compact final claim is invalid")
        claim_id = f"final-{ordinal + 1}"
        final_ref_to_id[claim_ref] = claim_id
        canonical_claims.append(
            {
                "claim_id": claim_id,
                "text": claim["text"].strip(),
                "sources": [dict(known_source_refs[ref]) for ref in refs],
            }
        )
    ledger = value["coverage_ledger"]
    if not isinstance(ledger, list):
        raise Global01LazyLiveError("compact coverage ledger is missing")
    canonical_ledger = []
    for row in ledger:
        if not isinstance(row, dict) or set(row) != {
            "mapped_claim_ref",
            "disposition",
            "final_claim_refs",
        }:
            raise Global01LazyLiveError("compact coverage row is incomplete")
        mapped_ref = row["mapped_claim_ref"]
        final_refs = row["final_claim_refs"]
        if (
            not isinstance(mapped_ref, str)
            or mapped_ref not in known_mapped_refs
            or row["disposition"] not in global_01_lazy.DISPOSITIONS
            or not isinstance(final_refs, list)
            or any(ref not in final_ref_to_id for ref in final_refs)
        ):
            raise Global01LazyLiveError("compact coverage row is invalid")
        canonical_ledger.append(
            {
                "mapped_claim_id": known_mapped_refs[mapped_ref],
                "disposition": row["disposition"],
                "final_claim_ids": [final_ref_to_id[ref] for ref in final_refs],
            }
        )
    return global_01_lazy.validate_structured_answer(
        {
            "answer": value["answer"].strip(),
            "claims": canonical_claims,
            "coverage_ledger": canonical_ledger,
        },
        known_sources={
            source["source_id"]: source["content_sha256"]
            for source in known_source_refs.values()
        },
        mapped_claim_ids=set(known_mapped_refs.values()),
    )


def _score_prompt(
    question: Mapping[str, Any],
    answer: Mapping[str, Any],
    documents: Mapping[str, Mapping[str, Any]],
    *,
    max_excerpt_chars: int,
) -> str:
    excerpts = []
    seen = set()
    for claim in answer["claims"]:
        for source in claim["sources"]:
            source_id = source["source_id"]
            document = documents[source_id]
            excerpt = best_source_excerpt(
                document["body"], claim["text"], max_chars=max_excerpt_chars
            )
            identity = (source_id, hashlib.sha256(excerpt.encode("utf-8")).hexdigest())
            if identity in seen:
                continue
            seen.add(identity)
            excerpts.append(
                {
                    "source_id": source_id,
                    "content_sha256": document["content_sha256"],
                    "body": excerpt,
                }
            )
    assertions = [
        {"index": index, "statement": statement}
        for index, statement in enumerate(question["qualified_assertions"])
    ]
    return (
        "Score the answer against the qualified assertions and cited canonical "
        "sources. Pass an assertion only when the answer entails it. Mark a final "
        "claim supported only when its cited excerpts support it.\n\n"
        f"QUESTION:\n{question['text']}\n\nASSERTIONS:\n"
        f"{json.dumps(assertions, separators=(',', ':'))}\n\nANSWER:\n"
        f"{json.dumps(answer, separators=(',', ':'))}\n\nCITED SOURCES:\n"
        f"{_source_context(excerpts)}\n\n"
        'Return JSON only: {"passed_assertion_indices":[0],"claim_support":'
        '[{"claim_id":"final-1","supported":true}]}.'
    )


def _ensure_database(
    config: Mapping[str, Any],
    repository_root: Path,
    artifact_root: Path,
    state: LazyRunState,
    checkpoint_path: Path,
    documents: Sequence[Mapping[str, Any]],
) -> Path:
    database_path = artifact_root / "database/global-01-lazy-run/fathomdb.sqlite"
    if "runtime/database" in state.cells:
        if not database_path.is_file():
            raise Global01LazyLiveError("checkpointed database is missing")
        return database_path
    cli_path = (repository_root / config["execution"]["fathomdb_cli"]).resolve()
    prepared = prepare_test_database(
        artifact_root / "database",
        test_id="global-01-lazy-run",
        embed_device="cpu",
        rerank_device="cpu",
        embedder="none",
        warm_cache=False,
        check_reranker=False,
        fathomdb_bin=str(cli_path),
    )
    from fathomdb import Engine, __version__ as fathomdb_version  # type: ignore[import-not-found]

    if fathomdb_version != config["execution"]["fathomdb_version"]:
        raise Global01LazyLiveError("FathomDB Python runtime version drifted")
    engine = Engine.open(str(prepared.database_path), use_default_embedder=False)
    try:
        for start in range(0, len(documents), 100):
            engine.write(
                [
                    {
                        "kind": "document",
                        "logical_id": f"global-01-ap-{row['ordinal']:04d}",
                        "source_id": row["source_id"],
                        "body": row["text"],
                    }
                    for row in documents[start : start + 100]
                ]
            )
        engine.drain(timeout_s=180)
    finally:
        engine.close()
    state.complete(
        "runtime/database",
        {
            "article_count": len(documents),
            "doctor_sha256": global_01_lazy.file_sha256(prepared.doctor_path),
        },
    )
    state.save(checkpoint_path)
    return prepared.database_path


def _retrieve_arm(
    *,
    arm: str,
    question: Mapping[str, Any],
    engine: Any,
    view: Any,
    config: Mapping[str, Any],
    source_hashes: Mapping[str, str],
    subqueries: Sequence[str] | None,
) -> list[dict[str, Any]]:
    if arm == "control":
        return global_01_lazy.retrieve_candidates(
            engine,
            [question["text"]],
            candidate_depth=config["profiles"]["control"]["candidate_depth"],
            source_hashes=source_hashes,
            read_view=view,
        )
    treatment = config["profiles"]["treatment"]
    candidates = global_01_lazy.retrieve_candidates(
        engine,
        [question["text"], *(subqueries or [])],
        candidate_depth=treatment["candidate_depth_per_query"],
        source_hashes=source_hashes,
        read_view=view,
    )
    return global_01_lazy.select_candidates(
        candidates,
        rrf_k=treatment["rrf_k"],
        relevance_weight=treatment["relevance_weight"],
        novelty_weight=treatment["novelty_weight"],
        max_documents_per_group=treatment["max_documents_per_group"],
        max_documents_total=treatment["max_documents_total"],
        context_max_tokens=treatment["context_max_tokens"],
    )


def _run_answer_arm(
    *,
    arm: str,
    question: Mapping[str, Any],
    engine: Any,
    view: Any,
    client: AirlockClient,
    config: Mapping[str, Any],
    source_hashes: Mapping[str, str],
    state: LazyRunState,
    checkpoint_path: Path,
) -> dict[str, Any]:
    qid = question["question_id"]
    answer_cell = f"answers/{arm}/{qid}"
    if answer_cell in state.cells:
        return state.cells[answer_cell]["answer"]
    started = time.perf_counter()
    generator = config["models"]["generator"]
    subqueries: list[str] | None = None
    if arm == "treatment":
        decomposition_cell = f"decomposition/treatment/{qid}"
        prompt = (
            "Decompose this global-sensemaking question into exactly four unique, "
            "bounded retrieval queries that cover distinct relevant aspects.\n\n"
            f"QUESTION:\n{question['text']}\n\n"
            'Return JSON only: {"subqueries":["...","...","...","..."]}.'
        )
        value = _complete_json_cell(
            client,
            state=state,
            checkpoint_path=checkpoint_path,
            cell=decomposition_cell,
            model=generator["model"],
            prompt=prompt,
            max_tokens=generator["decomposition_max_tokens"],
            temperature=generator["temperature"],
            validator=lambda raw: {
                "subqueries": parse_subqueries(
                    json.dumps(raw),
                    count=config["profiles"]["treatment"]["subquery_count"],
                )
            },
        )
        subqueries = value["subqueries"]
    retrieval_cell = f"retrieval/{arm}/{qid}"
    if retrieval_cell not in state.cells:
        rows = _retrieve_arm(
            arm=arm,
            question=question,
            engine=engine,
            view=view,
            config=config,
            source_hashes=source_hashes,
            subqueries=subqueries,
        )
        if not rows:
            raise Global01LazyLiveError(f"{arm} retrieved no sources for {qid}")
        state.complete(retrieval_cell, {"rows": rows})
        state.save(checkpoint_path)
    rows = state.cells[retrieval_cell]["rows"]
    if arm == "control":
        batch_size = config["profiles"]["control"]["map_batch_documents"]
        batches = [
            rows[start : start + batch_size]
            for start in range(0, len(rows), batch_size)
        ]
        map_limit = config["profiles"]["control"]["map_max_tokens"]
        map_max_claims = 2
    else:
        groups: dict[int, list[dict[str, Any]]] = {}
        for row in rows:
            groups.setdefault(int(row["group_ordinal"]), []).append(row)
        batches = [groups[key] for key in sorted(groups)]
        map_limit = config["profiles"]["treatment"]["map_max_tokens"]
        map_max_claims = 4
    mapped: list[dict[str, Any]] = []
    for ordinal, batch in enumerate(batches):
        map_cell = f"maps/{arm}/{qid}/{ordinal}"
        known_source_refs = {
            f"S{source_ordinal}": {
                "source_id": row["source_id"],
                "content_sha256": row["content_sha256"],
            }
            for source_ordinal, row in enumerate(batch)
        }
        value = _complete_json_cell(
            client,
            state=state,
            checkpoint_path=checkpoint_path,
            cell=map_cell,
            model=generator["model"],
            prompt=_map_prompt(
                question["text"], batch, max_claims=map_max_claims
            ),
            max_tokens=map_limit,
            temperature=generator["temperature"],
            validator=lambda raw, known_source_refs=known_source_refs, ordinal=ordinal: {
                "claims": _validate_mapped_claims(
                    raw,
                    known_source_refs=known_source_refs,
                    prefix=f"{arm}-{qid}-{ordinal:02d}",
                    max_claims=map_max_claims,
                    max_words=30,
                )
            },
        )
        mapped.extend(value["claims"])
    if not mapped:
        raise Global01LazyLiveError(f"{arm} produced no mapped claims for {qid}")
    reduce_limit = config["profiles"][arm]["reduce_max_tokens"]
    known_all = {row["source_id"]: row["content_sha256"] for row in rows}
    mapped_ids = {claim["claim_id"] for claim in mapped}
    reduce_cell = f"reductions/{arm}/{qid}"
    if config["execution"].get("reduce_output_adapter") == (
        "compact_refs_with_canonical_restore_v1"
    ):
        reduce_prompt, reduce_source_refs, reduce_mapped_refs = _reduce_prompt(
            question["text"], mapped
        )
        def reduce_validator(raw: object) -> dict[str, Any]:
            return _validate_compact_reduction(
                raw,
                known_source_refs=reduce_source_refs,
                known_mapped_refs=reduce_mapped_refs,
            )
    else:
        reduce_prompt = _legacy_reduce_prompt(question["text"], mapped)

        def reduce_validator(raw: object) -> dict[str, Any]:
            return global_01_lazy.validate_structured_answer(
                raw, known_sources=known_all, mapped_claim_ids=mapped_ids
            )
    answer = _complete_json_cell(
        client,
        state=state,
        checkpoint_path=checkpoint_path,
        cell=reduce_cell,
        model=generator["model"],
        prompt=reduce_prompt,
        max_tokens=reduce_limit,
        temperature=generator["temperature"],
        validator=reduce_validator,
    )
    state.complete(
        answer_cell,
        {
            "answer": answer,
            "source_count": len(rows),
            "mapped_claim_count": len(mapped),
            "end_to_end_ms": (time.perf_counter() - started) * 1000,
        },
    )
    state.save(checkpoint_path)
    return answer


def _run_scores_and_judges(
    *,
    question: Mapping[str, Any],
    answers: Mapping[str, Mapping[str, Any]],
    documents: Mapping[str, Mapping[str, Any]],
    client: AirlockClient,
    config: Mapping[str, Any],
    state: LazyRunState,
    checkpoint_path: Path,
) -> list[str]:
    qid = question["question_id"]
    required = []
    scorer = config["models"]["assertion_scorer"]
    for arm in ("control", "treatment"):
        for trial in range(scorer["trials"]):
            cell = f"scorer/{arm}/{qid}/{trial}"
            required.append(cell)
            _complete_json_cell(
                client,
                state=state,
                checkpoint_path=checkpoint_path,
                cell=cell,
                model=scorer["model"],
                prompt=_score_prompt(
                    question,
                    answers[arm],
                    documents,
                    max_excerpt_chars=scorer["source_excerpt_max_chars"],
                ),
                max_tokens=scorer["max_tokens"],
                temperature=scorer["temperature"],
                validator=lambda raw, arm=arm: validate_assertion_score(
                    raw,
                    answer=answers[arm],
                    assertion_count=len(question["qualified_assertions"]),
                ),
            )
    judge = config["models"]["pairwise_judge"]
    for repetition in range(judge["repetitions"]):
        for order in ("ct", "tc"):
            cell = f"judge/{qid}/{repetition}/{order}"
            required.append(cell)
            answer_a, answer_b = (
                (answers["control"]["answer"], answers["treatment"]["answer"])
                if order == "ct"
                else (answers["treatment"]["answer"], answers["control"]["answer"])
            )
            _complete_json_cell(
                client,
                state=state,
                checkpoint_path=checkpoint_path,
                cell=cell,
                model=judge["model"],
                prompt=_pairwise_prompt(question["text"], answer_a, answer_b),
                max_tokens=400,
                temperature=judge["temperature"],
                validator=_parse_judgment,
            )
    return required


def _run_questions(
    *,
    questions: Sequence[Mapping[str, Any]],
    engine: Any,
    view: Any,
    client: AirlockClient,
    config: Mapping[str, Any],
    documents: Sequence[Mapping[str, Any]],
    state: LazyRunState,
    checkpoint_path: Path,
) -> list[str]:
    source_hashes = {row["source_id"]: row["content_sha256"] for row in documents}
    document_map = {
        row["source_id"]: {
            "source_id": row["source_id"],
            "content_sha256": row["content_sha256"],
            "body": row["text"],
        }
        for row in documents
    }
    required = []
    for ordinal, question in enumerate(questions):
        answers: dict[str, Mapping[str, Any]] = {}
        arms = ("control", "treatment") if ordinal % 2 == 0 else ("treatment", "control")
        for arm in arms:
            answers[arm] = _run_answer_arm(
                arm=arm,
                question=question,
                engine=engine,
                view=view,
                client=client,
                config=config,
                source_hashes=source_hashes,
                state=state,
                checkpoint_path=checkpoint_path,
            )
            required.append(f"answers/{arm}/{question['question_id']}")
        required.extend(
            _run_scores_and_judges(
                question=question,
                answers=answers,
                documents=document_map,
                client=client,
                config=config,
                state=state,
                checkpoint_path=checkpoint_path,
            )
        )
    assert_cells_complete(state, required)
    return required


def _bootstrap(
    values_by_question: Mapping[str, Sequence[float]], *, draws: int, seed: int
) -> list[float]:
    question_ids = sorted(values_by_question)
    generator = random.Random(seed)
    estimates = []
    for _ in range(draws):
        sampled = [generator.choice(question_ids) for _ in question_ids]
        estimates.append(
            statistics.fmean(
                value for qid in sampled for value in values_by_question[qid]
            )
        )
    estimates.sort()
    return [
        estimates[int(0.025 * len(estimates))],
        estimates[min(len(estimates) - 1, int(0.975 * len(estimates)))],
    ]


def _summarize(
    config: Mapping[str, Any],
    state: LazyRunState,
    questions: Sequence[Mapping[str, Any]],
) -> dict[str, Any]:
    pairwise = {}
    repetitions = config["models"]["pairwise_judge"]["repetitions"]
    for metric in METRICS:
        by_question: dict[str, list[float]] = {}
        for question in questions:
            qid = question["question_id"]
            values = []
            for repetition in range(repetitions):
                for order in ("ct", "tc"):
                    verdict = state.cells[f"judge/{qid}/{repetition}/{order}"][
                        "value"
                    ][metric]
                    if verdict == "tie":
                        values.append(0.5)
                    elif (order == "ct" and verdict == "B") or (
                        order == "tc" and verdict == "A"
                    ):
                        values.append(1.0)
                    else:
                        values.append(0.0)
            by_question[qid] = values
        flat = [value for values in by_question.values() for value in values]
        pairwise[metric] = {
            "treatment_win_rate": statistics.fmean(flat),
            "question_clustered_ci95": _bootstrap(
                by_question,
                draws=config["evaluation"]["bootstrap_draws"],
                seed=config["evaluation"]["bootstrap_seed"],
            ),
            "judgments": len(flat),
        }
    score_summary = {}
    trials = config["models"]["assertion_scorer"]["trials"]
    for arm in ("control", "treatment"):
        recalls = []
        unsupported = []
        for question in questions:
            for trial in range(trials):
                score = state.cells[
                    f"scorer/{arm}/{question['question_id']}/{trial}"
                ]["value"]
                recalls.append(
                    len(score["passed_assertion_indices"])
                    / len(question["qualified_assertions"])
                )
                support = score["claim_support"]
                unsupported.append(
                    sum(not row["supported"] for row in support) / len(support)
                    if support
                    else 0.0
                )
        score_summary[arm] = {
            "qualified_assertion_recall": statistics.fmean(recalls),
            "unsupported_claim_rate": statistics.fmean(unsupported),
        }
    score_summary["paired_deltas"] = {
        "assertion_recall": score_summary["treatment"][
            "qualified_assertion_recall"
        ]
        - score_summary["control"]["qualified_assertion_recall"],
        "unsupported_claim_rate": score_summary["treatment"][
            "unsupported_claim_rate"
        ]
        - score_summary["control"]["unsupported_claim_rate"],
    }
    return {"pairwise": pairwise, "scoring": score_summary}


def _percentile(values: Sequence[float], fraction: float) -> float:
    ordered = sorted(values)
    if not ordered:
        raise Global01LazyLiveError("operational latency sample is empty")
    index = min(len(ordered) - 1, max(0, int(len(ordered) * fraction + 0.999) - 1))
    return ordered[index]


def _operational_summary(
    config: Mapping[str, Any],
    state: LazyRunState,
    questions: Sequence[Mapping[str, Any]],
) -> dict[str, Any]:
    model = config["models"]["generator"]["model"]
    summary = {}
    for arm in ("control", "treatment"):
        prefixes = [f"maps/{arm}/", f"reductions/{arm}/"]
        if arm == "treatment":
            prefixes.append("decomposition/treatment/")
        cells = [
            value
            for cell, value in state.cells.items()
            if any(cell.startswith(prefix) for prefix in prefixes)
        ]
        usage = [value["usage"] for value in cells]
        latencies = [
            float(state.cells[f"answers/{arm}/{row['question_id']}"]["end_to_end_ms"])
            for row in questions
        ]
        summary[arm] = {
            "model_calls": len(cells),
            "prompt_tokens": sum(row["prompt_tokens"] for row in usage),
            "completion_tokens": sum(row["completion_tokens"] for row in usage),
            "generator_cost_usd": sum(
                _usage_cost(config, model, row) for row in usage
            ),
            "end_to_end_p50_ms": statistics.median(latencies),
            "end_to_end_p95_ms": _percentile(latencies, 0.95),
        }
    control_cost = summary["control"]["generator_cost_usd"]
    control_p95 = summary["control"]["end_to_end_p95_ms"]
    if control_cost <= 0 or control_p95 <= 0:
        raise Global01LazyLiveError("control operational denominator is invalid")
    summary["ratios"] = {
        "treatment_to_control_token_cost": summary["treatment"][
            "generator_cost_usd"
        ]
        / control_cost,
        "treatment_to_control_end_to_end_p95": summary["treatment"][
            "end_to_end_p95_ms"
        ]
        / control_p95,
    }
    return summary


def acceptance_verdict(boundaries: Mapping[str, bool]) -> str:
    """Accept only when every registered boundary passes."""
    return "accept" if boundaries and all(boundaries.values()) else "reject"


def invalid_witness_summary(
    state: LazyRunState,
    *,
    witness_question_ids: set[str],
    failure: Global01LazyLiveError,
) -> dict[str, Any]:
    """Build a content-free receipt for a stopped, decision-ineligible witness."""
    answer_cells = [
        cell
        for cell in state.cells
        if cell.startswith("answers/") and len(cell.split("/")) == 3
    ]
    witness_answers = [
        cell for cell in answer_cells if cell.split("/")[2] in witness_question_ids
    ]
    heldout_answers = [
        cell for cell in answer_cells if cell.split("/")[2] not in witness_question_ids
    ]
    failure_category = (
        "semantic_retry_budget_exhausted"
        if str(failure).startswith("semantic retry budget exhausted")
        else "witness_execution_contract_failed"
    )
    return {
        "schema_version": "global-01.lazy-witness-invalid.v1",
        "program_track": "GLOBAL-01",
        "config_sha256": state.config_sha256,
        "state": "invalid_witness",
        "decision_eligible": False,
        "failure_category": failure_category,
        "aa_passed": bool(state.cells.get("gate/aa", {}).get("passed")),
        "witness_question_count": len(witness_question_ids),
        "completed_witness_answers": len(witness_answers),
        "heldout_answers": len(heldout_answers),
        "completed_map_cells": sum(
            cell.startswith("maps/") for cell in state.cells
        ),
        "invalid_semantic_submissions": sum(
            cell.startswith("invalid/") for cell in state.cells
        ),
        "cost_usd": state.cost_usd,
        "cost_cap_usd": state.cost_cap_usd,
    }


def _acceptance_summary(
    config: Mapping[str, Any],
    summary: Mapping[str, Any],
    operations: Mapping[str, Any],
    lifecycle: Mapping[str, Any],
    source_link_completeness: float,
) -> dict[str, Any]:
    boundary = config["evaluation"]["acceptance"]
    pairwise = summary["pairwise"]
    scoring = summary["scoring"]
    checks = {
        "headline_win_rates": all(
            pairwise[metric]["treatment_win_rate"]
            >= boundary["headline_win_rate_min"]
            for metric in HEADLINE_METRICS
        ),
        "headline_ci_lower_bounds": all(
            pairwise[metric]["question_clustered_ci95"][0]
            > boundary["headline_ci_lower_strictly_above"]
            for metric in HEADLINE_METRICS
        ),
        "assertion_recall_delta": scoring["paired_deltas"]["assertion_recall"]
        >= boundary["assertion_recall_delta_min"],
        "directness": pairwise["directness"]["treatment_win_rate"]
        >= boundary["directness_win_rate_min"],
        "unsupported_claim_delta": scoring["paired_deltas"][
            "unsupported_claim_rate"
        ]
        <= boundary["unsupported_claim_delta_max"],
        "source_link_completeness": source_link_completeness
        == boundary["canonical_source_link_completeness"],
        "lifecycle": lifecycle["temporal_failures"]
        <= boundary["lifecycle_failure_max"]
        and lifecycle["strict_current_supersession"] == "pass"
        and lifecycle["erasure"] == "pass"
        and lifecycle["derived_rows_written"] == 0,
        "token_cost_ratio": operations["ratios"][
            "treatment_to_control_token_cost"
        ]
        <= boundary["token_cost_ratio_max"],
        "end_to_end_p95_ratio": operations["ratios"][
            "treatment_to_control_end_to_end_p95"
        ]
        <= boundary["end_to_end_p95_ratio_max"],
    }
    return {"verdict": acceptance_verdict(checks), "boundaries": checks}


def _register_result_receipt(
    *,
    config: Mapping[str, Any],
    config_path: Path,
    state: LazyRunState,
    result: Mapping[str, Any],
    result_path: Path,
    checkpoint_path: Path,
    base_dir: Path,
) -> tuple[str, Path]:
    timestamp = datetime.fromisoformat(state.started_at)
    verdict = str(result["verdict"])
    run_id, run_dir = _lib.write_record(
        "global-01-lazy-coverage",
        ts=timestamp,
        config_obj=config,
        metrics=result,
        verdict=verdict,
        read=(
            "GLOBAL-01 held-out lazy-coverage comparison completed under its "
            f"registered all-boundary rule: {verdict}."
        ),
        code=_lib.git_info(),
        corpus={
            "source": "AP News BenchmarkQED",
            "manifest_sha256": config["corpus"]["archive_sha256"],
            "datasets": [
                {
                    "name": "AP News BenchmarkQED",
                    "documents": config["corpus"]["article_count"],
                    "questions": result["question_count"],
                    "selection_sha256": config["questions"]["split"][
                        "heldout_selection_sha256"
                    ],
                }
            ],
        },
        seeds={"clustered_bootstrap": config["evaluation"]["bootstrap_seed"]},
        env=_lib.env_info(
            key_deps={
                "fathomdb": config["execution"]["fathomdb_version"],
                "generator": config["models"]["generator"]["model"],
                "judge": config["models"]["pairwise_judge"]["model"],
            }
        ),
        cost_usd=float(result["cost_usd"]),
        headline={
            "program_track": "GLOBAL-01",
            "status": "complete",
            "decision": verdict,
        },
        n=int(result["question_count"]),
        config_path=str(config_path),
        tests=[
            "tests/experiments/test_global_01_lazy.py",
            "tests/experiments/test_global_01_lazy_live.py",
        ],
        artifacts=[
            {"path": str(result_path), "sha256": global_01_lazy.file_sha256(result_path)},
            {
                "path": str(checkpoint_path),
                "sha256": global_01_lazy.file_sha256(checkpoint_path),
            },
        ],
        open_questions=[],
        base_dir=base_dir,
    )
    _lib.regen_index_md(
        index_path=base_dir / "index.jsonl", md_path=base_dir / "INDEX.md"
    )
    return run_id, run_dir


def _register_invalid_witness_receipt(
    *,
    config: Mapping[str, Any],
    config_path: Path,
    state: LazyRunState,
    summary: Mapping[str, Any],
    summary_path: Path,
    checkpoint_path: Path,
    base_dir: Path,
) -> tuple[str, Path]:
    receipt_timestamp = _invalid_receipt_timestamp(config, state, base_dir)
    run_id, run_dir = _lib.write_record(
        "global-01-lazy-witness",
        ts=receipt_timestamp,
        config_obj=config,
        metrics=summary,
        verdict="invalid_witness",
        read=(
            "GLOBAL-01 A/A passed, but the development witness exhausted its "
            "semantic retry boundary before any complete answer or held-out "
            "execution; no quality decision is eligible."
        ),
        code=_lib.git_info(),
        corpus={
            "source": "AP News BenchmarkQED",
            "manifest_sha256": config["corpus"]["archive_sha256"],
            "datasets": [
                {
                    "name": "AP News BenchmarkQED",
                    "documents": config["corpus"]["article_count"],
                    "questions": summary["witness_question_count"],
                    "selection_sha256": config["questions"]["split"][
                        "witness_selection_sha256"
                    ],
                }
            ],
        },
        seeds={"split": config["questions"]["split"]["seed"]},
        env=_lib.env_info(
            key_deps={
                "fathomdb": config["execution"]["fathomdb_version"],
                "generator": config["models"]["generator"]["model"],
                "judge": config["models"]["pairwise_judge"]["model"],
            }
        ),
        cost_usd=float(summary["cost_usd"]),
        headline={
            "program_track": "GLOBAL-01",
            "status": "invalid_witness",
            "decision_eligible": False,
        },
        n=int(summary["completed_witness_answers"]),
        config_path=str(config_path),
        tests=[
            "tests/experiments/test_global_01_lazy.py",
            "tests/experiments/test_global_01_lazy_live.py",
        ],
        artifacts=[
            {
                "path": str(summary_path),
                "sha256": global_01_lazy.file_sha256(summary_path),
            },
            {
                "path": str(checkpoint_path),
                "sha256": global_01_lazy.file_sha256(checkpoint_path),
            },
        ],
        open_questions=[],
        base_dir=base_dir,
    )
    _lib.regen_index_md(
        index_path=base_dir / "index.jsonl", md_path=base_dir / "INDEX.md"
    )
    return run_id, run_dir


def _invalid_receipt_timestamp(
    config: Mapping[str, Any],
    state: LazyRunState,
    base_dir: Path,
    *,
    now: datetime | None = None,
) -> datetime:
    """Select a new receipt ID on resume without rewriting prior evidence."""
    experiment = "global-01-lazy-witness"
    config_sha = _lib.config_sha256(config)
    started = datetime.fromisoformat(state.started_at)
    started_id = _lib.make_run_id(experiment, started, config_sha)
    if not (base_dir / "runs" / started_id).exists():
        return started
    candidate = (now or datetime.now(timezone.utc)).replace(second=0, microsecond=0)
    while (
        base_dir
        / "runs"
        / _lib.make_run_id(experiment, candidate, config_sha)
    ).exists():
        candidate += timedelta(minutes=1)
    return candidate


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=("aa", "witness", "heldout"))
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--preflight-report", type=Path, required=True)
    parser.add_argument("--artifact-root", type=Path, required=True)
    parser.add_argument("--repository-root", type=Path, default=Path.cwd())
    parser.add_argument("--base-dir", type=Path, default=Path("experiments"))
    parser.add_argument("--airlock-url", default="http://127.0.0.1:4000")
    args = parser.parse_args()

    config = global_01_lazy.validate_config(
        json.loads(args.config.read_text(encoding="utf-8"))
    )
    global_01_lazy.assert_execution_authorized(config)
    preflight = validate_safe_preflight(config, args.preflight_report)
    config_hash = _canonical_sha256(config)
    cap = float(config["approval"]["cost_cap_usd"])
    checkpoint_path = args.artifact_root / "checkpoint.json"
    if checkpoint_path.is_file():
        state = LazyRunState.load(checkpoint_path, config_hash, cap)
    else:
        args.artifact_root.mkdir(parents=True, exist_ok=False, mode=0o700)
        state = LazyRunState.new(config_hash, cap)
        state.save(checkpoint_path)
    key = os.environ.get("AIRLOCK_VIRTUAL_KEY")
    if not key:
        raise Global01LazyLiveError("AIRLOCK_VIRTUAL_KEY is required")
    client = AirlockClient(args.airlock_url, key, config)
    aa = _run_aa(config, args.repository_root, client, state, checkpoint_path)
    if args.phase == "aa":
        print(json.dumps({"state": "aa_valid", "cost_usd": state.cost_usd, **aa}))
        return 0

    documents, _ = global_01_lazy.load_documents(config, args.repository_root)
    _, private_manifest = global_01_lazy.inspect_question_inputs(
        config, args.repository_root
    )
    database_path = _ensure_database(
        config,
        args.repository_root,
        args.artifact_root,
        state,
        checkpoint_path,
        documents,
    )
    from fathomdb import Engine  # type: ignore[import-not-found]
    from fathomdb.types import ReadView  # type: ignore[import-not-found]

    engine = Engine.open(str(database_path), use_default_embedder=False)
    try:
        view = ReadView()
        witness = [row for row in private_manifest["questions"] if row["witness"]]
        try:
            witness_required = _run_questions(
                questions=witness,
                engine=engine,
                view=view,
                client=client,
                config=config,
                documents=documents,
                state=state,
                checkpoint_path=checkpoint_path,
            )
        except Global01LazyLiveError as exc:
            invalid = invalid_witness_summary(
                state,
                witness_question_ids={row["question_id"] for row in witness},
                failure=exc,
            )
            invalid_path = args.artifact_root / "witness-invalid.json"
            invalid_path.write_text(
                json.dumps(invalid, indent=2) + "\n", encoding="utf-8"
            )
            invalid_path.chmod(0o600)
            run_id, run_dir = _register_invalid_witness_receipt(
                config=config,
                config_path=args.config,
                state=state,
                summary=invalid,
                summary_path=invalid_path,
                checkpoint_path=checkpoint_path,
                base_dir=args.base_dir,
            )
            print(
                json.dumps(
                    {
                        "state": "invalid_witness",
                        "decision_eligible": False,
                        "cost_usd": state.cost_usd,
                        "run_id": run_id,
                        "run_dir": str(run_dir),
                    }
                )
            )
            return 2
        assert_cells_complete(state, witness_required)
        if "gate/witness" not in state.cells:
            state.complete(
                "gate/witness",
                {
                    "state": "valid",
                    "question_count": len(witness),
                    "selection_sha256": config["questions"]["split"][
                        "witness_selection_sha256"
                    ],
                },
            )
            state.save(checkpoint_path)
        if args.phase == "witness":
            print(
                json.dumps(
                    {
                        "state": "witness_valid",
                        "question_count": len(witness),
                        "cost_usd": state.cost_usd,
                    }
                )
            )
            return 0
        heldout = [
            row for row in private_manifest["questions"] if row["split"] == "heldout"
        ]
        heldout_required = _run_questions(
            questions=heldout,
            engine=engine,
            view=view,
            client=client,
            config=config,
            documents=documents,
            state=state,
            checkpoint_path=checkpoint_path,
        )
    finally:
        engine.close()
    assert_cells_complete(state, heldout_required)
    summary = _summarize(config, state, heldout)
    operations = _operational_summary(config, state, heldout)
    claims = [
        claim
        for question in heldout
        for arm in ("control", "treatment")
        for claim in state.cells[f"answers/{arm}/{question['question_id']}"][
            "answer"
        ]["claims"]
    ]
    linked_claims = sum(bool(claim["sources"]) for claim in claims)
    source_link_completeness = linked_claims / len(claims) if claims else 0.0
    acceptance = _acceptance_summary(
        config,
        summary,
        operations,
        preflight["lifecycle"],
        source_link_completeness,
    )
    result = {
        "schema_version": RESULT_SCHEMA,
        "program_track": "GLOBAL-01",
        "config_sha256": config_hash,
        "state": "complete",
        "verdict": acceptance["verdict"],
        "question_count": len(heldout),
        "cost_usd": state.cost_usd,
        "cost_cap_usd": cap,
        "preflight_sha256": global_01_lazy.file_sha256(args.preflight_report),
        "canonical_source_link_completeness": source_link_completeness,
        "lifecycle": preflight["lifecycle"],
        "operations": operations,
        "acceptance": acceptance,
        **summary,
    }
    result_path = args.artifact_root / "result.json"
    result_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    result_path.chmod(0o600)
    run_id, run_dir = _register_result_receipt(
        config=config,
        config_path=args.config,
        state=state,
        result=result,
        result_path=result_path,
        checkpoint_path=checkpoint_path,
        base_dir=args.base_dir,
    )
    print(
        json.dumps(
            {
                "state": "complete",
                "verdict": result["verdict"],
                "cost_usd": state.cost_usd,
                "result": str(result_path),
                "run_id": run_id,
                "run_dir": str(run_dir),
            }
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
