"""Explicit caller-side REASON-01 relationship retrieval profile."""

from __future__ import annotations

import hashlib
import json
import math
import re
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping, Protocol, Sequence


class Reason01Error(RuntimeError):
    """A REASON-01 contract or execution refusal."""


@dataclass(frozen=True)
class ProfileExecution:
    """Selected context items and their content-free execution trace."""

    hits: tuple[Any, ...]
    trace: Mapping[str, object]


class _Engine(Protocol):
    def search_text_only(
        self, query: str, view: object | None = None, *, limit: int
    ) -> object: ...

    def search(
        self, query: str, filter: object | None = None, **kwargs: object
    ) -> object: ...


_TOP_LEVEL_KEYS = {"schema_version", "program_track", "known_intents", "profiles"}
_PROFILE_KEYS = {"id", "version", "status", "intent", "recipe", "runtime", "evidence"}
_RECIPE_KEYS = {
    "fts_prefix_limit",
    "max_queries",
    "hybrid_alpha",
    "hybrid_pool_n",
    "hybrid_rerank_depth",
    "use_graph_arm",
    "explain",
    "hybrid_limit",
    "context_limit",
}
_RUNTIME_KEYS = {
    "fathomdb_version",
    "fathomdb_source_commit",
    "embed_device",
    "rerank_device",
    "embedder_id",
    "reranker_id",
    "host",
    "gpu_uuid",
    "gpu_driver",
    "native_module_sha256",
    "cli_sha256",
    "embedder_cache_key",
    "embedder_config_sha256",
    "embedder_tokenizer_sha256",
    "embedder_model_sha256",
    "reranker_cache_key",
    "reranker_config_sha256",
    "reranker_tokenizer_sha256",
    "reranker_model_sha256",
    "doctor_required",
    "cpu_fallback",
}
_ATTESTATION_KEYS = _RUNTIME_KEYS - {"doctor_required", "cpu_fallback"}
_EVIDENCE_KEYS = {
    "scope",
    "equivalence_receipt_sha256",
    "locomo_corpus_sha256",
    "adapter_sha256",
    "identity_map_sha256",
}
_KNOWN_INTENTS = ("exact", "semantic", "timeline", "relationship", "global", "fast")
_PROFILE_ID = "protected_multiquery_v1"
_HEX_DIGEST = re.compile(r"[0-9a-f]{64}")
_TOKEN = re.compile(r"[A-Za-z0-9']+")
_STOPWORDS = frozenset(
    {
        "about",
        "according",
        "across",
        "all",
        "are",
        "conversation",
        "conversations",
        "did",
        "does",
        "had",
        "has",
        "have",
        "he",
        "her",
        "hers",
        "him",
        "his",
        "say",
        "said",
        "she",
        "tell",
        "that",
        "the",
        "their",
        "them",
        "they",
        "throughout",
        "told",
        "was",
        "were",
        "what",
        "when",
        "where",
        "which",
    }
)


def _mapping(value: object, label: str) -> Mapping[str, object]:
    if not isinstance(value, Mapping):
        raise Reason01Error(f"{label} must be an object")
    return value


def _exact_keys(value: Mapping[str, object], expected: set[str], label: str) -> None:
    if set(value) != expected:
        raise Reason01Error(f"{label} keys drifted")


def _positive_int(value: object, label: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
        raise Reason01Error(f"{label} must be a positive integer")
    return value


def _profile(registry: Mapping[str, object]) -> Mapping[str, object]:
    profiles = registry.get("profiles")
    if not isinstance(profiles, list) or len(profiles) != 1:
        raise Reason01Error("registry must contain exactly one profile")
    return _mapping(profiles[0], "profile")


def _recipe_int(recipe: Mapping[str, object], key: str) -> int:
    return _positive_int(recipe.get(key), f"profile recipe {key}")


def _recipe_float(recipe: Mapping[str, object], key: str) -> float:
    value = recipe.get(key)
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
        raise Reason01Error(f"profile recipe {key} must be a finite number")
    return float(value)


def load_registry(path: Path) -> Mapping[str, object]:
    """Load and strictly validate the REASON-01 profile registry."""
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise Reason01Error(f"registry unavailable: {path}") from exc
    registry = _mapping(document, "registry")
    _exact_keys(registry, _TOP_LEVEL_KEYS, "registry")
    if registry["schema_version"] != "reason01.profile-registry.v1":
        raise Reason01Error("registry schema version drifted")
    if registry["program_track"] != "REASON-01":
        raise Reason01Error("registry program track drifted")
    intents = registry["known_intents"]
    if not isinstance(intents, list) or tuple(intents) != _KNOWN_INTENTS:
        raise Reason01Error("registry known intents drifted")

    profile = _profile(registry)
    _exact_keys(profile, _PROFILE_KEYS, "profile")
    if (
        profile["id"] != _PROFILE_ID
        or profile["version"] != 1
        or profile["status"] != "experimental"
        or profile["intent"] != "relationship"
    ):
        raise Reason01Error("profile identity drifted")

    recipe = _mapping(profile["recipe"], "profile recipe")
    _exact_keys(recipe, _RECIPE_KEYS, "profile recipe")
    expected_recipe = {
        "fts_prefix_limit": 10,
        "max_queries": 3,
        "hybrid_alpha": 1.0,
        "hybrid_pool_n": 20,
        "hybrid_rerank_depth": 20,
        "use_graph_arm": False,
        "explain": True,
        "hybrid_limit": 20,
        "context_limit": 20,
    }
    if dict(recipe) != expected_recipe:
        raise Reason01Error("profile recipe drifted")

    runtime = _mapping(profile["runtime"], "profile runtime")
    _exact_keys(runtime, _RUNTIME_KEYS, "profile runtime")
    if (
        runtime["fathomdb_version"] != "0.8.23"
        or runtime["embed_device"] != "cuda:0"
        or runtime["rerank_device"] != "cuda:0"
        or runtime["doctor_required"] is not True
        or runtime["cpu_fallback"] is not False
    ):
        raise Reason01Error("profile runtime drifted")
    source_commit = runtime["fathomdb_source_commit"]
    if not isinstance(source_commit, str) or re.fullmatch(r"[0-9a-f]{40}", source_commit) is None:
        raise Reason01Error("profile runtime source commit is invalid")
    for key in ("embedder_id", "reranker_id"):
        value = runtime[key]
        if not isinstance(value, str) or "@" not in value:
            raise Reason01Error(f"profile runtime {key} is invalid")
    for key in _ATTESTATION_KEYS - {
        "fathomdb_version",
        "fathomdb_source_commit",
        "embed_device",
        "rerank_device",
        "embedder_id",
        "reranker_id",
        "host",
        "gpu_uuid",
        "gpu_driver",
        "embedder_cache_key",
        "reranker_cache_key",
    }:
        value = runtime[key]
        if not isinstance(value, str) or _HEX_DIGEST.fullmatch(value) is None:
            raise Reason01Error(f"profile runtime {key} is invalid")

    evidence = _mapping(profile["evidence"], "profile evidence")
    _exact_keys(evidence, _EVIDENCE_KEYS, "profile evidence")
    if evidence["scope"] != "development_only":
        raise Reason01Error("profile evidence scope drifted")
    for key in _EVIDENCE_KEYS - {"scope"}:
        value = evidence[key]
        if not isinstance(value, str) or _HEX_DIGEST.fullmatch(value) is None:
            raise Reason01Error(f"profile evidence {key} is invalid")
    return registry


def resolve_profile(
    registry: Mapping[str, object],
    *,
    intent: str | None = None,
    profile_override: str | None = None,
) -> str:
    """Resolve explicit caller authority to A0 or the protected profile."""
    profile = _profile(registry)
    known = registry.get("known_intents")
    if not isinstance(known, list) or tuple(known) != _KNOWN_INTENTS:
        raise Reason01Error("registry known intents drifted")
    if intent is not None and intent not in known:
        raise Reason01Error(f"unknown intent: {intent}")
    if profile_override is not None and profile_override != profile["id"]:
        raise Reason01Error(f"unknown profile: {profile_override}")
    if profile_override is not None and intent not in {None, "relationship"}:
        raise Reason01Error("intent and profile override conflict")
    if profile_override == profile["id"] or intent == "relationship":
        return _PROFILE_ID
    return "a0"


def constrained_queries(query: str, *, max_queries: int = 3) -> tuple[str, ...]:
    """Return the bounded deterministic model-free query forms."""
    if not isinstance(query, str) or not query.strip():
        raise Reason01Error("query must be non-empty text")
    if not isinstance(max_queries, int) or isinstance(max_queries, bool) or not 1 <= max_queries <= 3:
        raise Reason01Error("max_queries must be between 1 and 3")
    original = query.strip()
    tokens = _TOKEN.findall(original)
    content = [token for token in tokens if token.lower() not in _STOPWORDS]
    entities = [token for token in tokens[1:] if token[:1].isupper()]
    candidates = (
        original,
        " ".join(content),
        " ".join(dict.fromkeys([*entities, *content[:4], "mentioned", "sessions"])),
    )
    unique: list[str] = []
    for candidate in candidates:
        candidate = candidate.strip()
        if candidate and candidate not in unique:
            unique.append(candidate)
    return tuple(unique[:max_queries])


def merge_hits(
    prefix: Sequence[Any], branches: Sequence[Sequence[Any]], *, limit: int
) -> tuple[Any, ...]:
    """Protect the unique A0 prefix and round-robin unique shadow hits."""
    limit = _positive_int(limit, "context limit")
    selected: list[Any] = []
    seen: set[str] = set()

    def append(hit: Any) -> bool:
        identity = _hit_identity(hit)
        if identity in seen:
            return False
        seen.add(identity)
        selected.append(hit)
        return True

    for hit in prefix:
        append(hit)
        if len(selected) == limit:
            return tuple(selected)

    protected = set(seen)
    filtered: list[list[Any]] = []
    for branch in branches:
        branch_seen: set[str] = set()
        candidates: list[Any] = []
        for hit in branch:
            identity = _hit_identity(hit)
            if identity in protected or identity in branch_seen:
                continue
            branch_seen.add(identity)
            candidates.append(hit)
        filtered.append(candidates)

    rank = 0
    while len(selected) < limit and any(rank < len(branch) for branch in filtered):
        for branch in filtered:
            if rank < len(branch):
                append(branch[rank])
            if len(selected) == limit:
                break
        rank += 1
    return tuple(selected)


def _hit_identity(hit: Any) -> str:
    identity = getattr(getattr(hit, "id", None), "value", None)
    if not isinstance(identity, str) or not identity:
        raise Reason01Error("search hit has no stable identity")
    return identity


def _hash(domain: str, value: str) -> str:
    return hashlib.sha256(f"reason01:{domain}:{value}".encode()).hexdigest()


def make_safe_trace(
    *,
    profile_id: str,
    intent: str | None,
    profile_override: str | None,
    queries: Sequence[str],
    selected_hits: Sequence[Any],
    branch_counts: Sequence[int],
    elapsed_ms: float,
) -> Mapping[str, object]:
    """Project an execution into a content-free, domain-hashed trace."""
    if not math.isfinite(elapsed_ms) or elapsed_ms < 0:
        raise Reason01Error("elapsed_ms must be finite and non-negative")
    return {
        "schema_version": "reason01.safe-trace.v1",
        "profile_id": profile_id,
        "intent": intent,
        "profile_override": profile_override,
        "query_count": len(queries),
        "query_sha256s": [_hash("query", query) for query in queries],
        "selected_count": len(selected_hits),
        "selected_identity_sha256s": [
            _hash("identity", _hit_identity(hit)) for hit in selected_hits
        ],
        "branch_counts": list(branch_counts),
        "elapsed_ms": round(elapsed_ms, 6),
    }


def _validate_runtime(
    registry: Mapping[str, object], runtime_attestation: Mapping[str, object] | None
) -> None:
    if runtime_attestation is None:
        raise Reason01Error("runtime attestation is required")
    runtime = _mapping(_profile(registry)["runtime"], "profile runtime")
    if runtime_attestation.get("doctor_ok") is not True or any(
        runtime_attestation.get(key) != runtime[key] for key in _ATTESTATION_KEYS
    ):
        raise Reason01Error("runtime attestation does not satisfy the profile")


def _result_hits(result: object) -> Sequence[Any]:
    hits = getattr(result, "results", None)
    if not isinstance(hits, Sequence):
        raise Reason01Error("Engine result has no safe hit sequence")
    if getattr(result, "soft_fallback", None) is not None:
        raise Reason01Error("Engine used a forbidden soft fallback")
    return hits


def _validate_attribution(branches: Sequence[Sequence[Any]]) -> None:
    attributions: dict[str, tuple[str, str]] = {}
    for branch in branches:
        for hit in branch:
            identity = _hit_identity(hit)
            source = getattr(hit, "source_id", None)
            body = getattr(hit, "body", None)
            if not isinstance(source, str) or not source or not isinstance(body, str):
                raise Reason01Error("search hit lacks canonical attribution")
            attribution = (source, hashlib.sha256(body.encode()).hexdigest())
            prior = attributions.setdefault(identity, attribution)
            if prior != attribution:
                raise Reason01Error(f"attribution conflict for identity {identity}")


def execute_profile(
    engine: _Engine,
    query: str,
    registry: Mapping[str, object],
    *,
    intent: str | None = None,
    profile_override: str | None = None,
    view: object | None = None,
    metadata_filter: object | None = None,
    runtime_attestation: Mapping[str, object] | None = None,
) -> ProfileExecution:
    """Execute the resolved profile through public FathomDB read methods."""
    profile_id = resolve_profile(
        registry, intent=intent, profile_override=profile_override
    )
    if profile_id == "a0":
        raise Reason01Error("profile is A0; execute it through the general caller path")
    if metadata_filter is not None:
        raise Reason01Error("metadata filter is unsupported by the protected FTS prefix")
    _validate_runtime(registry, runtime_attestation)
    if getattr(engine, "dense_disabled", lambda: False)():
        raise Reason01Error("dense retrieval is disabled")

    recipe = _mapping(_profile(registry)["recipe"], "profile recipe")
    started = time.perf_counter()
    fts_result = engine.search_text_only(
        query, view, limit=_recipe_int(recipe, "fts_prefix_limit")
    )
    cursor = getattr(fts_result, "projection_cursor", None)
    if cursor is None:
        raise Reason01Error("FTS result lacks a projection cursor")
    prefix = _result_hits(fts_result)
    queries = constrained_queries(query, max_queries=_recipe_int(recipe, "max_queries"))
    branches: list[Sequence[Any]] = []
    for candidate in queries:
        result = engine.search(
            candidate,
            None,
            rerank_depth=_recipe_int(recipe, "hybrid_rerank_depth"),
            use_graph_arm=bool(recipe["use_graph_arm"]),
            alpha=_recipe_float(recipe, "hybrid_alpha"),
            pool_n=_recipe_int(recipe, "hybrid_pool_n"),
            explain=bool(recipe["explain"]),
            view=view,
            limit=_recipe_int(recipe, "hybrid_limit"),
        )
        if getattr(result, "projection_cursor", None) != cursor:
            raise Reason01Error("projection cursor drifted between branches")
        branches.append(_result_hits(result))
    _validate_attribution([prefix, *branches])
    selected = merge_hits(prefix, branches, limit=_recipe_int(recipe, "context_limit"))
    trace = make_safe_trace(
        profile_id=profile_id,
        intent=intent,
        profile_override=profile_override,
        queries=queries,
        selected_hits=selected,
        branch_counts=[len(prefix), *(len(branch) for branch in branches)],
        elapsed_ms=(time.perf_counter() - started) * 1000,
    )
    return ProfileExecution(selected, trace)
