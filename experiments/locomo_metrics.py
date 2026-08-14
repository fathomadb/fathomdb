"""Safe retrieval metrics over external LOCOMO predictions and provenance."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path

from experiments.locomo_provenance import search_request_fingerprint


_PROVENANCE_SCHEMA = "locomo-facade-provenance.v1"
_SAFE_KEYS = {"conversation_id", "session_id", "turn_ids"}


def _validated_requests(sidecar: object) -> dict[str, list[dict[str, object]]]:
    if not isinstance(sidecar, dict) or set(sidecar) != {"schema_version", "requests"}:
        raise ValueError("provenance sidecar keys mismatch")
    requests = sidecar.get("requests")
    if sidecar.get("schema_version") != _PROVENANCE_SCHEMA or not isinstance(requests, dict):
        raise ValueError("invalid provenance sidecar schema")
    for request_hash, results in requests.items():
        if not isinstance(request_hash, str) or len(request_hash) != 64 or not isinstance(results, list):
            raise ValueError("invalid provenance request entry")
        for result in results:
            if not isinstance(result, dict) or set(result) != _SAFE_KEYS or not isinstance(result["turn_ids"], list):
                raise ValueError("invalid safe provenance result")
    return requests


def _dcg(relevances: list[bool], *, limit: int) -> float:
    return sum(1 / math.log2(rank + 1) for rank, relevant in enumerate(relevances[:limit], start=1) if relevant)


def summarize_predictions(predictions_dir: str | Path, provenance_sidecar: object) -> dict[str, object]:
    """Summarize retrieval metrics without returning question, answer, or hit text."""
    requests = _validated_requests(provenance_sidecar)
    per_question: list[dict[str, object]] = []
    mrr_values: list[float] = []
    r1_values: list[float] = []
    r5_values: list[float] = []
    r10_values: list[float] = []
    r20_values: list[float] = []
    ndcg_values: list[float] = []
    temporal_values: list[float] = []
    excluded_no_evidence = 0
    for path in sorted(Path(predictions_dir).glob("conv*_q*.json")):
        try:
            item = json.loads(path.read_text(encoding="utf-8"))
            question_id, user_id, query, evidence, category = (
                item["question_id"], item["user_id"], item["question"], item["evidence"], item["category"],
            )
        except (KeyError, TypeError, json.JSONDecodeError) as exc:
            raise ValueError(f"invalid prediction result: {path}") from exc
        if not isinstance(question_id, str) or not isinstance(evidence, list) or not isinstance(category, int):
            raise ValueError(f"invalid prediction fields: {path}")
        gold = {value for value in evidence if isinstance(value, str) and value}
        if not gold:
            excluded_no_evidence += 1
            continue
        try:
            provenance = requests[search_request_fingerprint(user_id, query)]
        except KeyError as exc:
            raise ValueError(f"prediction is missing safe provenance: {path.name}") from exc
        relevances = [bool(gold.intersection(result["turn_ids"])) for result in provenance]
        first_rank = next((rank for rank, relevant in enumerate(relevances, start=1) if relevant), None)
        r1 = float(any(relevances[:1]))
        r5 = float(any(relevances[:5]))
        r10 = float(any(relevances[:10]))
        r20 = float(any(relevances[:20]))
        mrr = 1 / first_rank if first_rank is not None else 0.0
        ideal = _dcg([True] * min(len(gold), 10), limit=10)
        ndcg = _dcg(relevances, limit=10) / ideal if ideal else 0.0
        per_question.append({"question_id": question_id, "category": category, "r_at_10": r10})
        mrr_values.append(mrr)
        r1_values.append(r1)
        r5_values.append(r5)
        r10_values.append(r10)
        r20_values.append(r20)
        ndcg_values.append(ndcg)
        if category == 2:
            temporal_values.append(r10)
    if not per_question:
        raise ValueError("no evidence-backed prediction result files found")
    def mean(values: list[float]) -> float:
        return sum(values) / len(values)
    return {
        "aggregate": {
            "n": len(per_question), "excluded_no_evidence": excluded_no_evidence,
            "r_at_5": mean(r5_values), "r_at_10": mean(r10_values),
            "r_at_20": mean(r20_values), "mrr": mean(mrr_values), "r_at_1": mean(r1_values),
            "ndcg_at_10": mean(ndcg_values),
            "temporal_evidence_recall": mean(temporal_values) if temporal_values else None,
        },
        "per_question": per_question,
    }


def main(argv: list[str] | None = None) -> int:
    """Write a content-free retrieval metric sidecar for one external run."""
    parser = argparse.ArgumentParser(description="Summarize LOCOMO retrieval predictions")
    parser.add_argument("--predictions", type=Path, required=True)
    parser.add_argument("--provenance", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        sidecar = json.loads(args.provenance.read_text(encoding="utf-8"))
        summary = summarize_predictions(args.predictions, sidecar)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        parser.error(str(exc))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
