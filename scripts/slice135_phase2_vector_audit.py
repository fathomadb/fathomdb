#!/usr/bin/env python3
"""Independently score Phase 2 vector-stage hits against same-model f32 truth."""

from __future__ import annotations

import math
import struct
from typing import Any


def _f32(value: float) -> float:
    if not math.isfinite(value):
        raise ValueError("non-finite vector value")
    try:
        converted = struct.unpack("<f", struct.pack("<f", value))[0]
    except OverflowError as error:
        raise ValueError("vector value exceeds f32 range") from error
    if not math.isfinite(converted):
        raise ValueError("vector value exceeds finite f32 range")
    return converted


def _vector(value: object, dimension: int) -> list[float]:
    if not isinstance(value, list) or len(value) != dimension:
        raise ValueError("vector dimension mismatch")
    if any(isinstance(item, bool) or not isinstance(item, (int, float)) for item in value):
        raise ValueError("vector has nonnumeric component")
    return [_f32(float(item)) for item in value]


def _hash(value: object) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(
        char in "0123456789abcdef" for char in value
    )


def f32_squared_l2(left: list[float], right: list[float]) -> float:
    """Match the Rust f32 squared-distance fold over same-model vectors."""
    if len(left) != len(right) or not left:
        raise ValueError("vector dimension mismatch")
    total = _f32(0.0)
    for a, b in zip(left, right):
        difference = _f32(_f32(a) - _f32(b))
        total = _f32(total + _f32(difference * difference))
    return total


def exact_neighbors(
    documents: list[dict[str, Any]], query_vector: list[float],
    target_body_sha256: str, k: int,
) -> list[str]:
    """Rank unique non-target bodies by exact f32 distance, then body hash."""
    if k < 1 or not _hash(target_body_sha256):
        raise ValueError("invalid K or target body hash")
    ranked = sorted(
        (
            f32_squared_l2(document["vector"], query_vector),
            document["body_sha256"],
        )
        for document in documents
        if document["body_sha256"] != target_body_sha256
    )
    result: list[str] = []
    seen: set[str] = set()
    for _, body_hash in ranked:
        if body_hash in seen:
            continue
        seen.add(body_hash)
        result.append(body_hash)
        if len(result) == k:
            return result
    raise ValueError("fewer than K eligible unique bodies")


def score_version(
    receipt: dict[str, Any], expected_query_ids: list[str], *, k: int = 10,
    dimension: int = 384,
) -> dict[str, Any]:
    """Validate one complete vector-only receipt and recompute recall@K."""
    if k < 1 or dimension < 1 or not expected_query_ids or len(set(expected_query_ids)) != len(expected_query_ids):
        raise ValueError("invalid scoring configuration")
    documents = receipt.get("documents")
    queries = receipt.get("queries")
    if not isinstance(documents, list) or not isinstance(queries, list):
        raise ValueError("missing documents or queries")
    by_id: dict[str, dict[str, Any]] = {}
    normalized_documents: list[dict[str, Any]] = []
    body_vectors: dict[str, list[float]] = {}
    for document in documents:
        if not isinstance(document, dict):
            raise ValueError("malformed document")
        logical_id = document.get("logical_id")
        body_hash = document.get("body_sha256")
        if not isinstance(logical_id, str) or not logical_id or logical_id in by_id or not _hash(body_hash):
            raise ValueError("invalid or duplicate document identity")
        vector = _vector(document.get("vector"), dimension)
        previous = body_vectors.setdefault(body_hash, vector)
        if previous != vector:
            raise ValueError("duplicate body has inconsistent vectors")
        normalized = {"logical_id": logical_id, "body_sha256": body_hash, "vector": vector}
        normalized_documents.append(normalized)
        by_id[logical_id] = normalized
    if len(queries) != len(expected_query_ids):
        raise ValueError("query denominator changed")
    per_query: dict[str, float] = {}
    for query in queries:
        if not isinstance(query, dict):
            raise ValueError("malformed query")
        query_id = query.get("query_id")
        target = query.get("target_body_sha256")
        if not isinstance(query_id, str) or query_id not in expected_query_ids or query_id in per_query:
            raise ValueError("query ID set missing or changed")
        if not _hash(target) or target not in body_vectors:
            raise ValueError("query target body missing")
        if query.get("text_only_count") != 0 or isinstance(query.get("text_only_count"), bool):
            raise ValueError("lexical arm not isolated")
        query_vector = _vector(query.get("vector"), dimension)
        hits = query.get("hits")
        if not isinstance(hits, list):
            raise ValueError("missing hit list")
        exact = set(exact_neighbors(normalized_documents, query_vector, target, k))
        seen: set[str] = set()
        observed: list[str] = []
        previous_distance: float | None = None
        for hit in hits:
            if not isinstance(hit, dict) or hit.get("branch") != "vector":
                raise ValueError("non-vector hit in isolated stage")
            logical_id = hit.get("logical_id")
            if not isinstance(logical_id, str) or logical_id not in by_id:
                raise ValueError("unknown hit ID")
            body_hash = by_id[logical_id]["body_sha256"]
            if hit.get("body_sha256") != body_hash:
                raise ValueError("hit body hash mismatch")
            distance = f32_squared_l2(by_id[logical_id]["vector"], query_vector)
            if previous_distance is not None and distance < previous_distance:
                raise ValueError("vector-only hit rank inversion against exact f32 distance")
            previous_distance = distance
            if body_hash == target or body_hash in seen:
                continue
            seen.add(body_hash)
            observed.append(body_hash)
            if len(observed) == k:
                break
        per_query[query_id] = len(exact.intersection(observed)) / k
    if set(per_query) != set(expected_query_ids):
        raise ValueError("query ID set missing or changed")
    return {
        "query_count": len(expected_query_ids),
        "k": k,
        "per_query": per_query,
        "mean_recall": sum(per_query.values()) / len(per_query),
    }
