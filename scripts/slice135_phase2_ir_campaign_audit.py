#!/usr/bin/env python3
"""Independently audit paired installed-wheel judged retrieval receipts."""

from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import hashlib
import importlib.util
import json
from pathlib import Path
import random
import sqlite3
from typing import Any
import zipfile

SCORER_PATH = Path(__file__).with_name("slice135_phase2_ir_scorer.py")
SCORER_SPEC = importlib.util.spec_from_file_location("slice135_phase2_ir_scorer", SCORER_PATH)
assert SCORER_SPEC is not None and SCORER_SPEC.loader is not None
SCORER = importlib.util.module_from_spec(SCORER_SPEC)
SCORER_SPEC.loader.exec_module(SCORER)
score_version = SCORER.score_version


def sha256(path: Path) -> str:
    """Hash a local input without loading it into memory."""
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _hash_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def audit_database(
    database: Path, expected_rows: list[tuple[str, str, str, str]],
    *, vector_count: int | None,
) -> dict[str, Any]:
    """Compare persisted canonical rows with source-derived bytes and indexes."""
    if not database.is_file() or not expected_rows:
        raise ValueError("database or expected canonical rows missing")
    with sqlite3.connect(f"file:{database}?mode=ro", uri=True) as connection:
        integrity = connection.execute("PRAGMA integrity_check").fetchone()[0]
        if integrity != "ok":
            raise ValueError("database integrity failure")
        actual = connection.execute(
            "SELECT logical_id, kind, body, source_id FROM canonical_nodes ORDER BY logical_id"
        ).fetchall()
        if actual != sorted(expected_rows):
            raise ValueError("persisted canonical rows differ from source mapping")
        counts = {"canonical_nodes": len(actual), "integrity": integrity}
        if vector_count is not None:
            counts["vector_rows"] = connection.execute(
                "SELECT COUNT(*) FROM vector_default_rowids"
            ).fetchone()[0]
            counts["search_index"] = connection.execute(
                "SELECT COUNT(*) FROM search_index"
            ).fetchone()[0]
            counts["search_index_v2"] = connection.execute(
                "SELECT COUNT(*) FROM search_index_v2"
            ).fetchone()[0]
            if any(counts[key] != vector_count for key in (
                "vector_rows", "search_index", "search_index_v2"
            )):
                raise ValueError("persisted retrieval index denominator changed")
    return counts


def _source_documents(
    source_root: Path, specifications: list[dict[str, str]],
) -> tuple[list[dict[str, str]], list[tuple[str, str, str, str]]]:
    if not specifications:
        raise ValueError("missing source specifications")
    metadata = []
    expected_rows = []
    seen_ids: set[str] = set()
    for source in specifications:
        name = source["name"]
        path = source_root / source["filename"]
        if sha256(path) != source["sha256"]:
            raise ValueError(f"{name}: source hash mismatch")
        rows = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
        for row in sorted(rows, key=lambda item: item["doc_id"]):
            doc_id = row.get("doc_id")
            body = row.get("body")
            if not isinstance(doc_id, str) or not doc_id or not isinstance(body, str) or not body.strip():
                raise ValueError("malformed source document")
            logical_id = f"{name}:{doc_id}"
            if logical_id in seen_ids:
                raise ValueError("duplicate source document ID")
            seen_ids.add(logical_id)
            stored_body = json.dumps({"summary": body}, ensure_ascii=False, separators=(",", ":"))
            expected_rows.append((logical_id, "doc", stored_body, f"slice135-ir:{name}"))
            metadata.append({
                "logical_id": logical_id, "source": name, "source_doc_id": doc_id,
                "body_sha256": _hash_bytes(stored_body.encode()),
            })
    return metadata, expected_rows


def _positive_gold(gold: dict[str, Any], allowed_ids: set[str]) -> tuple[list[dict[str, str]], int]:
    raw_queries = gold.get("queries")
    if not isinstance(raw_queries, list) or not raw_queries:
        raise ValueError("missing gold queries")
    positive = []
    negative = 0
    seen: set[str] = set()
    for query in raw_queries:
        if not isinstance(query, dict):
            raise ValueError("malformed gold query")
        query_id = query.get("query_id")
        source = query.get("source")
        text = query.get("query")
        query_class = query.get("query_class")
        expected = query.get("expected_top_k_doc_ids")
        evidence = query.get("required_evidence")
        if (not isinstance(query_id, str) or not query_id or query_id in seen
                or not isinstance(source, str) or not source
                or not isinstance(text, str) or not text.strip()
                or not isinstance(expected, list) or not isinstance(evidence, list)):
            raise ValueError("duplicate or malformed gold query")
        seen.add(query_id)
        if query_class == "negative":
            if expected or evidence:
                raise ValueError("negative query has required evidence")
            negative += 1
            continue
        if query_class not in {"exact_fact", "exploratory"}:
            raise ValueError("unsupported gold class")
        if (len(expected) != 1 or not isinstance(expected[0], str) or not expected[0]
                or len(evidence) != 1 or not isinstance(evidence[0], dict)
                or evidence[0].get("doc_id") != expected[0]
                or evidence[0].get("necessity") != "required"):
            raise ValueError("required evidence disagreement")
        required_id = f"{source}:{expected[0]}"
        if required_id not in allowed_ids:
            raise ValueError("required gold document absent from source")
        positive.append({
            "query_id": query_id, "query_class": query_class,
            "source": source, "text": text, "required_id": required_id,
        })
    return positive, negative


def _verify_wheel(wheel: Path, specification: dict[str, str]) -> None:
    if sha256(wheel) != specification["wheel_sha256"]:
        raise ValueError("wheel hash mismatch")
    with zipfile.ZipFile(wheel) as archive:
        if _hash_bytes(archive.read("fathomdb/_fathomdb.abi3.so")) != specification["native_sha256"]:
            raise ValueError("wheel native hash mismatch")


def _audit_receipt(
    *, raw_dir: Path, version: str, protocol: dict[str, Any], protocol_hash: str,
    wheel: Path, metadata: list[dict[str, str]], expected_rows: list[tuple[str, str, str, str]],
    positive: list[dict[str, str]], negative: int, gold: dict[str, Any],
) -> tuple[dict[str, Any], dict[str, Any]]:
    raw_path = raw_dir / "raw.json"
    database = raw_dir / "retrieval.sqlite"
    raw = json.loads(raw_path.read_text())
    specification = protocol["versions"][version]
    _verify_wheel(wheel, specification)
    required = {
        "schema_version": 1, "version": version,
        "source_sha": specification["source_sha"],
        "wheel_sha256": specification["wheel_sha256"],
        "native_sha256": specification["native_sha256"],
        "protocol_sha256": protocol_hash,
        "runner_sha256": protocol["runner_sha256"],
        "scorer_sha256": protocol["scorer_sha256"],
        "gold_sha256": protocol["gold"]["sha256"],
        "model_name": protocol["model_name"],
        "negative_queries_omitted": negative,
        "database_sha256": sha256(database),
        "documents": metadata,
    }
    for key, value in required.items():
        if raw.get(key) != value:
            raise ValueError(f"{version}: receipt {key} mismatch")
    identity = raw.get("identity")
    environment = raw.get("environment")
    if not isinstance(identity, dict) or not isinstance(environment, dict):
        raise ValueError("missing installed identity")
    native_path = Path(identity.get("native_path", ""))
    package_path = Path(identity.get("module_path", ""))
    python_path = Path(environment.get("python_executable", ""))
    if (not native_path.is_file() or not package_path.is_file() or not python_path.is_file()
            or sha256(native_path) != specification["native_sha256"]):
        raise ValueError("installed native identity mismatch")
    with zipfile.ZipFile(wheel) as archive:
        if sha256(package_path) != _hash_bytes(archive.read("fathomdb/__init__.py")):
            raise ValueError("installed package differs from wheel")
    observations = raw.get("queries")
    if not isinstance(observations, list) or len(observations) != len(positive):
        raise ValueError("query denominator changed")
    for expected, actual in zip(positive, observations):
        if not isinstance(actual, dict):
            raise ValueError("malformed query receipt")
        for key in ("query_id", "query_class", "source", "text", "required_id"):
            if actual.get(key) != expected[key]:
                raise ValueError(f"query {key} mismatch")
        if actual.get("text_sha256") != _hash_bytes(expected["text"].encode()):
            raise ValueError("query text hash mismatch")
    database_result = audit_database(database, expected_rows, vector_count=len(metadata))
    scores = score_version(gold["queries"], observations, {item["logical_id"] for item in metadata})
    if scores["negative_queries_omitted"] != negative:
        raise ValueError("negative query count mismatch")
    return scores, database_result


def paired_summary(scores: dict[str, dict[str, Any]], expected_query_ids: list[str]) -> dict[str, Any]:
    """Compare complete paired required-document ranks and class metrics."""
    if set(scores) != {"baseline", "candidate"}:
        raise ValueError("paired baseline and candidate receipts required")
    baseline = scores["baseline"]
    candidate = scores["candidate"]
    if (not expected_query_ids or len(set(expected_query_ids)) != len(expected_query_ids)
            or set(baseline["per_query"]) != set(expected_query_ids)
            or set(candidate["per_query"]) != set(expected_query_ids)):
        raise ValueError("paired query ID set mismatch")
    by_class: dict[str, dict[str, float]] = {}
    if set(baseline["by_class"]) != set(candidate["by_class"]):
        raise ValueError("paired class set mismatch")
    for query_class in baseline["by_class"]:
        left = baseline["by_class"][query_class]
        right = candidate["by_class"][query_class]
        by_class[query_class] = {
            "recall_at_10_delta": right["recall_at_10"] - left["recall_at_10"],
        }
        if "mrr_at_10" in left and "mrr_at_10" in right:
            by_class[query_class]["mrr_at_10_delta"] = right["mrr_at_10"] - left["mrr_at_10"]
    deltas = {}
    wins = Counter()
    grouped: dict[str, list[float]] = defaultdict(list)
    for query_id in expected_query_ids:
        left = baseline["per_query"][query_id]
        right = candidate["per_query"][query_id]
        if left["class"] != right["class"]:
            raise ValueError("paired query class mismatch")
        lr = left["required_rank"]
        rr = right["required_rank"]
        delta = (0.0 if rr is None else 1.0 / rr) - (0.0 if lr is None else 1.0 / lr)
        deltas[query_id] = delta
        grouped[left["class"]].append(delta)
        wins["candidate_better" if delta > 0 else "candidate_worse" if delta < 0 else "equal"] += 1
    intervals = {}
    rng = random.Random(135)
    for query_class, values in grouped.items():
        samples = []
        for _ in range(10_000):
            samples.append(sum(rng.choices(values, k=len(values))) / len(values))
        samples.sort()
        intervals[query_class] = [samples[249], samples[9749]]
    return {
        "query_count": len(expected_query_ids),
        "by_class": by_class,
        "mrr_delta_bootstrap_95pct_by_class": intervals,
        "per_query_mrr_delta": deltas,
        "candidate_better": wins["candidate_better"],
        "candidate_worse": wins["candidate_worse"],
        "equal": wins["equal"],
    }


def audit(
    *, protocol_path: Path, runner: Path, scorer: Path, source_root: Path,
    gold_path: Path, baseline_wheel: Path, candidate_wheel: Path,
    baseline_dir: Path, candidate_dir: Path,
) -> dict[str, Any]:
    """Rebuild expected inputs, inspect both SQLite files and recompute scores."""
    protocol_bytes = protocol_path.read_bytes()
    protocol = json.loads(protocol_bytes)
    if protocol.get("schema_version") != 1 or set(protocol.get("versions", {})) != {"baseline", "candidate"}:
        raise ValueError("protocol versions or schema mismatch")
    if sha256(runner) != protocol.get("runner_sha256") or sha256(scorer) != protocol.get("scorer_sha256"):
        raise ValueError("frozen evaluator code hash mismatch")
    if sha256(gold_path) != protocol["gold"]["sha256"]:
        raise ValueError("gold hash mismatch")
    gold = json.loads(gold_path.read_text())
    if gold.get("corpus_hash") != protocol["gold"]["corpus_hash"] or gold.get("qrels_version") != protocol["gold"]["qrels_version"]:
        raise ValueError("gold identity mismatch")
    metadata, expected_rows = _source_documents(source_root, protocol["sources"])
    positive, negative = _positive_gold(gold, {row["logical_id"] for row in metadata})
    denominator = protocol["denominator"]
    if (len(metadata) != denominator["documents"] or len(positive) != denominator["positive_queries"]
            or negative != denominator["negative_queries_omitted"]
            or dict(Counter(row["source"] for row in metadata)) != denominator["documents_by_source"]
            or dict(Counter(row["query_class"] for row in positive)) != denominator["positive_by_class"]):
        raise ValueError("declared corpus or query denominator changed")
    scores = {}
    databases = {}
    for version, wheel, raw_dir in (
        ("baseline", baseline_wheel, baseline_dir),
        ("candidate", candidate_wheel, candidate_dir),
    ):
        scores[version], databases[version] = _audit_receipt(
            raw_dir=raw_dir, version=version, protocol=protocol,
            protocol_hash=_hash_bytes(protocol_bytes), wheel=wheel,
            metadata=metadata, expected_rows=expected_rows, positive=positive,
            negative=negative, gold=gold,
        )
    return {
        "protocol_sha256": _hash_bytes(protocol_bytes),
        "databases": databases,
        "scores": scores,
        "paired": paired_summary(scores, [query["query_id"] for query in positive]),
    }


def main() -> None:
    """Audit one frozen baseline/candidate retrieval pair."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--protocol", required=True, type=Path)
    parser.add_argument("--runner", required=True, type=Path)
    parser.add_argument("--scorer", required=True, type=Path)
    parser.add_argument("--source-root", required=True, type=Path)
    parser.add_argument("--gold", required=True, type=Path)
    parser.add_argument("--baseline-wheel", required=True, type=Path)
    parser.add_argument("--candidate-wheel", required=True, type=Path)
    parser.add_argument("--baseline-dir", required=True, type=Path)
    parser.add_argument("--candidate-dir", required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(audit(
        protocol_path=args.protocol, runner=args.runner, scorer=args.scorer,
        source_root=args.source_root, gold_path=args.gold,
        baseline_wheel=args.baseline_wheel, candidate_wheel=args.candidate_wheel,
        baseline_dir=args.baseline_dir, candidate_dir=args.candidate_dir,
    ), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
