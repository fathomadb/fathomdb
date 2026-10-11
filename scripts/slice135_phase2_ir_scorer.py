#!/usr/bin/env python3
"""Score named required-document retrieval with complete query denominators."""

from __future__ import annotations

from collections import defaultdict
from typing import Any


POSITIVE_CLASSES = frozenset({"exact_fact", "exploratory"})


def _required_document(query: dict[str, Any]) -> str | None:
    query_class = query.get("query_class")
    source = query.get("source")
    query_id = query.get("query_id")
    if (query_class not in POSITIVE_CLASSES | {"negative"}
            or not isinstance(source, str) or not source
            or not isinstance(query_id, str) or not query_id):
        raise ValueError("invalid gold query class or identity")
    expected = query.get("expected_top_k_doc_ids")
    evidence = query.get("required_evidence")
    if not isinstance(expected, list) or not isinstance(evidence, list):
        raise ValueError("invalid required-document gold")
    if query_class == "negative":
        if expected or evidence:
            raise ValueError("negative query has required document")
        return None
    if (len(expected) != 1 or not isinstance(expected[0], str) or not expected[0]
            or len(evidence) != 1 or not isinstance(evidence[0], dict)
            or evidence[0].get("doc_id") != expected[0]
            or evidence[0].get("necessity") != "required"):
        raise ValueError("required evidence disagrees with named document")
    return f"{source}:{expected[0]}"


def score_version(
    gold_queries: list[dict[str, Any]], observations: list[dict[str, Any]],
    allowed_ids: set[str],
) -> dict[str, Any]:
    """Return Recall@1/5/10 and MRR@10, rejecting incomplete or malformed input.

    Every positive query remains in its source class denominator. An explicit
    search error scores zero; a missing observation is invalid. Negative
    answer-abstention queries are outside the retrieval denominator.
    """
    if not isinstance(gold_queries, list) or not gold_queries or not isinstance(observations, list):
        raise ValueError("missing gold query or observation list")
    if not isinstance(allowed_ids, set) or not allowed_ids or any(
        not isinstance(item, str) or not item for item in allowed_ids
    ):
        raise ValueError("invalid allowed document IDs")
    required_by_query: dict[str, tuple[str, str]] = {}
    negative_count = 0
    for query in gold_queries:
        if not isinstance(query, dict):
            raise ValueError("malformed gold query")
        query_id = query.get("query_id")
        if not isinstance(query_id, str) or query_id in required_by_query:
            raise ValueError("duplicate or invalid query ID")
        required = _required_document(query)
        if required is None:
            negative_count += 1
            continue
        if required not in allowed_ids:
            raise ValueError("required document absent from corpus")
        required_by_query[query_id] = (query["query_class"], required)
    if len(observations) != len(required_by_query):
        raise ValueError("query denominator changed")

    per_query: dict[str, dict[str, Any]] = {}
    accum: dict[str, dict[str, float | int]] = defaultdict(lambda: {
        "denominator": 0, "errors": 0, "hit_1": 0, "hit_5": 0,
        "hit_10": 0, "reciprocal_rank": 0.0,
    })
    for observation in observations:
        if not isinstance(observation, dict):
            raise ValueError("malformed query observation")
        query_id = observation.get("query_id")
        if not isinstance(query_id, str) or query_id not in required_by_query or query_id in per_query:
            raise ValueError("query ID missing, duplicated or outside positive gold")
        query_class, required = required_by_query[query_id]
        status = observation.get("status")
        hits = observation.get("hits")
        if not isinstance(hits, list) or len(hits) > 10:
            raise ValueError("invalid hit list")
        if status == "error":
            if hits or not isinstance(observation.get("error_type"), str) or not observation["error_type"]:
                raise ValueError("invalid search error receipt")
            rank = None
            accum[query_class]["errors"] += 1
        elif status == "ok":
            if "error_type" in observation:
                raise ValueError("successful search carries error")
            seen: set[str] = set()
            rank = None
            for position, hit in enumerate(hits, 1):
                if (not isinstance(hit, dict) or not isinstance(hit.get("doc_id"), str)
                        or hit["doc_id"] not in allowed_ids
                        or not isinstance(hit.get("branch"), str) or not hit["branch"]):
                    raise ValueError("unknown or malformed hit")
                if hit["doc_id"] in seen:
                    raise ValueError("duplicate hit")
                seen.add(hit["doc_id"])
                if hit["doc_id"] == required:
                    rank = position
        else:
            raise ValueError("invalid search status")
        per_query[query_id] = {"class": query_class, "status": status, "required_rank": rank}
        summary = accum[query_class]
        summary["denominator"] += 1
        if rank is not None:
            summary["hit_10"] += 1
            summary["reciprocal_rank"] += 1.0 / rank
            if rank <= 5:
                summary["hit_5"] += 1
            if rank == 1:
                summary["hit_1"] += 1
    if set(per_query) != set(required_by_query):
        raise ValueError("query ID set incomplete")
    by_class = {}
    for query_class in sorted(accum):
        summary = accum[query_class]
        denominator = int(summary["denominator"])
        by_class[query_class] = {
            "denominator": denominator,
            "errors": int(summary["errors"]),
            "recall_at_1": summary["hit_1"] / denominator,
            "recall_at_5": summary["hit_5"] / denominator,
            "recall_at_10": summary["hit_10"] / denominator,
            "mrr_at_10": summary["reciprocal_rank"] / denominator,
        }
    return {
        "negative_queries_omitted": negative_count,
        "by_class": by_class,
        "per_query": per_query,
    }
