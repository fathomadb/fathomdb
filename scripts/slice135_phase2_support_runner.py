#!/usr/bin/env python3
"""Collect paired installed-wheel evidence-set retrieval on fresh databases."""

from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import importlib.util
import json
from pathlib import Path
import platform
import sys
from typing import Any


def sha256(path: Path) -> str:
    """Hash one local artifact."""
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _hash_text(value: str) -> str:
    return hashlib.sha256(value.encode()).hexdigest()


def validate_inputs(data: dict[str, Any], denominator: dict[str, Any]) -> dict[str, Any]:
    """Require unique corpus IDs, complete gold and declared denominators."""
    if not isinstance(data, dict) or data.get("dataset") not in {"musique", "locomo"}:
        raise ValueError("invalid derived dataset")
    documents = data.get("documents")
    queries = data.get("queries")
    if not isinstance(documents, list) or not documents or not isinstance(queries, list) or not queries:
        raise ValueError("missing corpus or query list")
    ids: set[str] = set()
    for document in documents:
        if not isinstance(document, dict):
            raise ValueError("malformed document")
        logical_id = document.get("logical_id")
        if (not isinstance(logical_id, str) or not logical_id or logical_id in ids
                or not isinstance(document.get("body"), str) or not document["body"].strip()
                or not isinstance(document.get("source_id"), str) or not document["source_id"]):
            raise ValueError("invalid or duplicate document")
        ids.add(logical_id)
    query_ids: set[str] = set()
    classes = Counter()
    for query in queries:
        if not isinstance(query, dict):
            raise ValueError("malformed query")
        query_id = query.get("query_id")
        required = query.get("required_ids")
        query_class = query.get("query_class")
        if (not isinstance(query_id, str) or not query_id or query_id in query_ids
                or not isinstance(query_class, str) or not query_class
                or not isinstance(query.get("text"), str) or not query["text"].strip()
                or not isinstance(required, list) or not required
                or any(not isinstance(item, str) or item not in ids for item in required)
                or len(set(required)) != len(required)
                or not isinstance(query.get("answers"), list) or not query["answers"]):
            raise ValueError("invalid or incomplete support-set query")
        query_ids.add(query_id)
        classes[query_class] += 1
    if (len(documents) != denominator.get("documents")
            or len(queries) != denominator.get("queries")
            or dict(classes) != denominator.get("by_class")
            or sum(q.get("strict_multi_session") is True for q in queries)
            != denominator.get("strict_multi_session", 0)):
        raise ValueError("declared denominator changed")
    return data


def _module(path: Path, name: str) -> Any:
    specification = importlib.util.spec_from_file_location(name, path)
    assert specification is not None and specification.loader is not None
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


def run(
    *, protocol_path: Path, wheel: Path, version: str, source: Path,
    derived: Path, output: Path,
) -> Path:
    """Run one frozen support-set campaign on a fresh installed-wheel DB."""
    import fathomdb

    protocol_bytes = protocol_path.read_bytes()
    protocol = json.loads(protocol_bytes)
    if (protocol.get("schema_version") != 1 or version not in protocol.get("versions", {})
            or protocol.get("result_limit") != 20):
        raise ValueError("protocol schema, version or result limit mismatch")
    scripts = Path(__file__).resolve().parent
    dependencies = {
        "runner_sha256": Path(__file__).resolve(),
        "scorer_sha256": scripts / "slice135_phase2_support_scorer.py",
        "inputs_sha256": scripts / "slice135_phase2_support_inputs.py",
        "ir_runner_sha256": scripts / "slice135_phase2_ir_runner.py",
        "locomo_loader_sha256": scripts.parent / "src/python/eval/locomo_loader.py",
    }
    for key, path in dependencies.items():
        if sha256(path) != protocol.get(key):
            raise ValueError(f"frozen {key} mismatch")
    if sha256(source) != protocol["source"]["sha256"]:
        raise ValueError("source hash mismatch")
    if sha256(derived) != protocol["derived"]["sha256"]:
        raise ValueError("derived input hash mismatch")
    data = validate_inputs(json.loads(derived.read_text()), protocol["denominator"])
    if data["dataset"] != protocol["dataset"]:
        raise ValueError("dataset identity mismatch")
    specification = protocol["versions"][version]
    ir = _module(dependencies["ir_runner_sha256"], "slice135_ir_identity")
    identity = ir.installed_identity(
        wheel, specification["wheel_sha256"], specification["native_sha256"],
    )
    output.mkdir(parents=True, exist_ok=False)
    database = output / "support.sqlite"
    engine = fathomdb.Engine.open(str(database), use_default_embedder=True)
    try:
        if engine.open_report().default_embedder.name != protocol["model_name"]:
            raise ValueError("model identity mismatch")
        engine.configure_projections([fathomdb.ProjectionSpec(
            name="summary", roles=frozenset({fathomdb.ProjectionRole.SEARCHABLE}), vector=True,
        )])
        documents = data["documents"]
        batch_size = protocol["write_batch_size"]
        for start in range(0, len(documents), batch_size):
            engine.write([{
                "kind": "doc", "logical_id": row["logical_id"],
                "source_id": row["source_id"],
                "body": json.dumps({"summary": row["body"]}, ensure_ascii=False, separators=(",", ":")),
            } for row in documents[start:start + batch_size]])
            engine.drain(timeout_s=600)
            print(f"{version}: projected {min(start + batch_size, len(documents))}/{len(documents)}", file=sys.stderr, flush=True)
        readiness = next(spec.vector_dense_readiness for spec in fathomdb.read.projections(engine)
                         if spec.name == "summary")
        if readiness != "ready":
            raise ValueError("vector projection not ready")
    finally:
        engine.close()

    engine = fathomdb.Engine.open(str(database), use_default_embedder=True)
    observations = []
    try:
        if engine.open_report().default_embedder.name != protocol["model_name"]:
            raise ValueError("reopened model identity mismatch")
        for index, query in enumerate(data["queries"], 1):
            record: dict[str, Any] = {
                "query_id": query["query_id"], "query_class": query["query_class"],
                "text": query["text"], "text_sha256": _hash_text(query["text"]),
                "required_ids": query["required_ids"],
                "strict_multi_session": query.get("strict_multi_session", False),
            }
            try:
                hits = engine.search(query["text"], limit=20).results
                record.update({"status": "ok", "hits": [
                    {"doc_id": hit.id.value, "branch": hit.branch} for hit in hits
                ]})
            except Exception as error:
                record.update({"status": "error", "error_type": type(error).__name__, "hits": []})
            observations.append(record)
            if index % 100 == 0:
                print(f"{version}: queried {index}/{len(data['queries'])}", file=sys.stderr, flush=True)
    finally:
        engine.close()
    raw = {
        "schema_version": 1, "version": version, "dataset": protocol["dataset"],
        "source_sha": specification["source_sha"],
        "wheel_sha256": specification["wheel_sha256"],
        "native_sha256": specification["native_sha256"],
        "protocol_sha256": hashlib.sha256(protocol_bytes).hexdigest(),
        "runner_sha256": sha256(Path(__file__).resolve()),
        "scorer_sha256": sha256(dependencies["scorer_sha256"]),
        "inputs_sha256": sha256(dependencies["inputs_sha256"]),
        "source_sha256": sha256(source), "derived_sha256": sha256(derived),
        "database_sha256": sha256(database), "identity": identity,
        "environment": {"python_executable": str(Path(sys.executable).resolve()),
                        "python_version": sys.version, "platform": platform.platform()},
        "model_name": protocol["model_name"],
        "documents": [{"logical_id": row["logical_id"],
                       "body_sha256": _hash_text(json.dumps(
                           {"summary": row["body"]}, ensure_ascii=False, separators=(",", ":")
                       ))} for row in data["documents"]],
        "queries": observations,
    }
    path = output / "raw.json"
    path.write_text(json.dumps(raw, indent=2, sort_keys=True, ensure_ascii=False, allow_nan=False) + "\n")
    return path


def main() -> None:
    """Read one exact-identity support-set run request."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--protocol", required=True, type=Path)
    parser.add_argument("--wheel", required=True, type=Path)
    parser.add_argument("--version", required=True, choices=("baseline", "candidate"))
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--derived", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    print(run(protocol_path=args.protocol, wheel=args.wheel, version=args.version,
              source=args.source, derived=args.derived, output=args.output))


if __name__ == "__main__":
    main()
