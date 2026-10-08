#!/usr/bin/env python3
"""Score complete evidence-set availability with exact query denominators."""

from __future__ import annotations

from collections import defaultdict
from typing import Any


def score_version(
    queries: list[dict[str, Any]], observations: list[dict[str, Any]],
    allowed_ids: set[str], *, limit: int = 20,
) -> dict[str, Any]:
    """Score support recall and complete-set coverage at 10 and 20.

    Missing observations invalidate the campaign. Search errors retain their
    query in the denominator and receive zero evidence credit.
    """
    if not isinstance(queries, list) or not queries or not isinstance(observations, list):
        raise ValueError("missing query or observation list")
    if not isinstance(allowed_ids, set) or not allowed_ids or limit < 20:
        raise ValueError("invalid corpus or search limit")
    gold = {}
    for query in queries:
        if not isinstance(query, dict):
            raise ValueError("malformed query")
        query_id = query.get("query_id")
        query_class = query.get("query_class")
        required = query.get("required_ids")
        if (not isinstance(query_id, str) or not query_id or query_id in gold
                or not isinstance(query_class, str) or not query_class
                or not isinstance(query.get("text"), str) or not query["text"].strip()
                or not isinstance(required, list) or not required
                or len(set(required)) != len(required)
                or any(not isinstance(item, str) or item not in allowed_ids for item in required)):
            raise ValueError("invalid support-set gold")
        gold[query_id] = (query_class, set(required), query.get("strict_multi_session") is True)
    if len(observations) != len(gold):
        raise ValueError("query denominator changed")

    per_query = {}
    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    strict = []
    for observation in observations:
        if not isinstance(observation, dict):
            raise ValueError("malformed observation")
        query_id = observation.get("query_id")
        if not isinstance(query_id, str) or query_id not in gold or query_id in per_query:
            raise ValueError("missing, duplicate or unknown query")
        query_class, required, strict_multi = gold[query_id]
        status = observation.get("status")
        hits = observation.get("hits")
        if not isinstance(hits, list) or len(hits) > limit:
            raise ValueError("invalid hit list")
        if status == "error":
            if hits or not isinstance(observation.get("error_type"), str) or not observation["error_type"]:
                raise ValueError("invalid search error")
            ranked = []
        elif status == "ok":
            if "error_type" in observation:
                raise ValueError("success carries error")
            ranked = []
            for hit in hits:
                if (not isinstance(hit, dict) or not isinstance(hit.get("doc_id"), str)
                        or hit["doc_id"] not in allowed_ids
                        or not isinstance(hit.get("branch"), str) or not hit["branch"]
                        or hit["doc_id"] in ranked):
                    raise ValueError("unknown, malformed or duplicate hit")
                ranked.append(hit["doc_id"])
        else:
            raise ValueError("invalid status")
        ranks = {doc_id: ranked.index(doc_id) + 1 for doc_id in required if doc_id in ranked}
        row = {
            "class": query_class, "status": status,
            "required_count": len(required),
            "support_found_at_10": sum(rank <= 10 for rank in ranks.values()),
            "support_found_at_20": sum(rank <= 20 for rank in ranks.values()),
            "complete_at_10": len(ranks) == len(required) and max(ranks.values()) <= 10,
            "complete_at_20": len(ranks) == len(required) and max(ranks.values()) <= 20,
            "last_required_rank": max(ranks.values()) if len(ranks) == len(required) else None,
        }
        per_query[query_id] = row
        groups[query_class].append(row)
        groups["overall"].append(row)
        if strict_multi:
            strict.append(row)
    if set(per_query) != set(gold):
        raise ValueError("query set incomplete")

    def summarize(rows: list[dict[str, Any]]) -> dict[str, Any]:
        denominator = len(rows)
        if not denominator:
            raise ValueError("empty score group")
        return {
            "denominator": denominator,
            "errors": sum(row["status"] == "error" for row in rows),
            "complete_at_10": sum(row["complete_at_10"] for row in rows) / denominator,
            "complete_at_20": sum(row["complete_at_20"] for row in rows) / denominator,
            "mean_support_recall_at_10": sum(
                row["support_found_at_10"] / row["required_count"] for row in rows
            ) / denominator,
            "mean_support_recall_at_20": sum(
                row["support_found_at_20"] / row["required_count"] for row in rows
            ) / denominator,
        }

    return {
        "by_class": {key: summarize(value) for key, value in sorted(groups.items())},
        "strict_multi_session": summarize(strict) if strict else None,
        "per_query": per_query,
    }
