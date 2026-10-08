#!/usr/bin/env python3
"""Collect paired installed-wheel vector-stage observations on fresh databases."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import platform
import sqlite3
import sys
from typing import Any
import zipfile


LEXICAL_TABLES = ("search_index", "search_index_v2", "search_index_edges")


def sha256(path: Path) -> str:
    """Hash a local input without loading it into memory."""
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _bytes_hash(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _lead_sentence(body: str) -> str:
    cleaned = " ".join(line.lstrip("-*#> \t") for line in body.splitlines()).strip()
    out = ""
    for char in cleaned[:140]:
        out += char
        if char in ".!?" and len(out.strip()) >= 12:
            break
    return out.strip()


def _query_text(row: dict[str, Any]) -> str:
    title = (row.get("title") or "").strip()
    body = row["body"].strip()
    if len(title) >= 6 and title.lower() != "untitled" and title != body:
        return title
    lead = _lead_sentence(body)
    if not lead or lead == body:
        raise ValueError("selected query target has no nonverbatim title or lead")
    return lead


def select_corpus_and_queries(
    source_paths: dict[str, Path], *, per_source: int, query_stride: int,
) -> tuple[list[dict[str, str]], list[dict[str, str]]]:
    """Select sorted nonempty source rows and fixed title-or-lead queries."""
    if not source_paths or per_source < 1 or query_stride < 1 or per_source % query_stride:
        raise ValueError("invalid corpus or query selection")
    documents: list[dict[str, str]] = []
    queries: list[dict[str, str]] = []
    seen_doc_ids: set[tuple[str, str]] = set()
    for source, path in source_paths.items():
        if not source or not path.is_file():
            raise ValueError("source file missing")
        rows = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
        chosen = sorted((row for row in rows if row.get("body")), key=lambda row: row["doc_id"])[
            :per_source
        ]
        if len(chosen) != per_source:
            raise ValueError(f"{source}: fewer than declared nonempty rows")
        for index, row in enumerate(chosen):
            source_doc_id = row["doc_id"]
            if not isinstance(source_doc_id, str) or (source, source_doc_id) in seen_doc_ids:
                raise ValueError("invalid or duplicate source document ID")
            seen_doc_ids.add((source, source_doc_id))
            logical_id = f"{source}-{index:03d}"
            body = json.dumps({"summary": row["body"]})
            documents.append({
                "kind": "doc", "body": body, "logical_id": logical_id,
                "source_id": f"slice135-fidelity:{source}", "source_doc_id": source_doc_id,
            })
            if index % query_stride == 0:
                queries.append({
                    "query_id": f"{source}:{index:03d}", "text": _query_text(row),
                    "target_id": logical_id,
                })
    return documents, queries


def _canonical_hash(connection: sqlite3.Connection) -> str:
    rows = connection.execute(
        "SELECT logical_id, kind, body, source_id FROM canonical_nodes ORDER BY logical_id"
    ).fetchall()
    return _bytes_hash(json.dumps(rows, separators=(",", ":")).encode())


def _table_counts(connection: sqlite3.Connection) -> dict[str, int]:
    return {
        name: connection.execute(f"SELECT COUNT(*) FROM {name}").fetchone()[0]
        for name in LEXICAL_TABLES
    }


def isolate_lexical_index(
    database: Path, pre_isolation: Path, *, expected_nodes: int | None = None,
) -> dict[str, Any]:
    """Save a pre-isolation copy, then delete FTS rows only for vector measurement."""
    if not database.is_file() or pre_isolation.exists():
        raise ValueError("database missing or pre-isolation copy already exists")
    with sqlite3.connect(database) as connection:
        with sqlite3.connect(pre_isolation) as snapshot:
            connection.backup(snapshot)
        canonical_count = connection.execute("SELECT COUNT(*) FROM canonical_nodes").fetchone()[0]
        if expected_nodes is not None and canonical_count != expected_nodes:
            raise ValueError("canonical corpus denominator changed")
        pre = _table_counts(connection)
        if pre["search_index"] != canonical_count or pre["search_index_v2"] != canonical_count:
            raise ValueError("pre-isolation lexical indexes incomplete")
        canonical_hash = _canonical_hash(connection)
        pre_integrity = connection.execute("PRAGMA integrity_check").fetchone()[0]
        if pre_integrity != "ok":
            raise ValueError("pre-isolation SQLite integrity failed")
        for table in LEXICAL_TABLES:
            connection.execute(f"DELETE FROM {table}")
        post = _table_counts(connection)
        if any(post.values()) or _canonical_hash(connection) != canonical_hash:
            raise ValueError("lexical isolation changed canonical rows or left FTS rows")
        post_integrity = connection.execute("PRAGMA integrity_check").fetchone()[0]
        if post_integrity != "ok":
            raise ValueError("post-isolation SQLite integrity failed")
    return {
        "pre": pre, "post": post, "canonical_count": canonical_count,
        "canonical_sha256": canonical_hash,
        "pre_integrity": pre_integrity, "post_integrity": post_integrity,
    }


def installed_identity(wheel: Path, wheel_hash: str, native_hash: str) -> dict[str, str]:
    """Verify the imported package and native module against the selected wheel."""
    import fathomdb
    import fathomdb._fathomdb as native

    if sha256(wheel) != wheel_hash or fathomdb.__file__ is None or native.__file__ is None:
        raise ValueError("installed wheel identity mismatch")
    package = Path(fathomdb.__file__).resolve()
    native_path = Path(native.__file__).resolve()
    prefix = Path(sys.prefix).resolve()
    if not package.is_relative_to(prefix) or not native_path.is_relative_to(prefix):
        raise ValueError("installed SDK import escaped selected environment")
    with zipfile.ZipFile(wheel) as archive:
        for member, installed in (
            ("fathomdb/__init__.py", package),
            ("fathomdb/_fathomdb.abi3.so", native_path),
        ):
            if _bytes_hash(archive.read(member)) != sha256(installed):
                raise ValueError(f"installed file differs from wheel: {member}")
    if sha256(native_path) != native_hash:
        raise ValueError("installed native module mismatch")
    return {"module_path": str(package), "native_path": str(native_path)}


def _model_name(engine: Any) -> str:
    report = engine.open_report()
    return report.default_embedder.name


def run(
    *, protocol_path: Path, wheel: Path, version: str, source_root: Path,
    output: Path,
) -> Path:
    """Run one pinned version against a fresh vector corpus and retain raw evidence."""
    import fathomdb

    protocol_bytes = protocol_path.read_bytes()
    protocol = json.loads(protocol_bytes)
    if protocol.get("schema_version") != 1 or version not in protocol.get("versions", {}):
        raise ValueError("protocol schema or version mismatch")
    if protocol.get("runner_sha256") != sha256(Path(__file__).resolve()):
        raise ValueError("runner bytes differ from frozen protocol")
    specification = protocol["versions"][version]
    identity = installed_identity(wheel, specification["wheel_sha256"], specification["native_sha256"])
    corpus_spec = protocol["corpus"]
    source_paths: dict[str, Path] = {}
    for item in corpus_spec["sources"]:
        path = source_root / item["filename"]
        if sha256(path) != item["sha256"]:
            raise ValueError(f"{item['name']}: source SHA-256 mismatch")
        source_paths[item["name"]] = path
    documents, queries = select_corpus_and_queries(
        source_paths, per_source=corpus_spec["per_source"],
        query_stride=corpus_spec["query_stride"],
    )
    if len(documents) != protocol["denominator"]["documents_per_version"] or len(queries) != protocol["denominator"]["queries_per_version"]:
        raise ValueError("predeclared corpus or query denominator changed")
    output.mkdir(parents=True, exist_ok=False)
    database = output / "fidelity.sqlite"
    pre_isolation = output / "pre-isolation.sqlite"
    engine = fathomdb.Engine.open(str(database), use_default_embedder=True)
    try:
        if _model_name(engine) != protocol["model_name"]:
            raise ValueError("default embedder identity changed")
        engine.configure_projections([
            fathomdb.ProjectionSpec(
                name="summary", roles=frozenset({fathomdb.ProjectionRole.SEARCHABLE}),
                vector=True,
            )
        ])
        for start in range(0, len(documents), 100):
            engine.write([{key: row[key] for key in ("kind", "body", "logical_id", "source_id")}
                          for row in documents[start:start + 100]])
            engine.drain(timeout_s=600)
            print(f"{version}: projected {min(start + 100, len(documents))}/{len(documents)}", file=sys.stderr, flush=True)
        readiness = next(
            spec.vector_dense_readiness for spec in fathomdb.read.projections(engine)
            if spec.name == "summary"
        )
        if readiness != "ready":
            raise ValueError("vector projection not ready")
        raw_documents = []
        for index, row in enumerate(documents):
            raw_documents.append({
                "logical_id": row["logical_id"],
                "source_doc_id": row["source_doc_id"],
                "body_sha256": _bytes_hash(row["body"].encode()),
                "vector": engine.embed(row["body"]),
            })
            if (index + 1) % 100 == 0:
                print(f"{version}: oracle-embedded {index + 1}/{len(documents)}", file=sys.stderr, flush=True)
        query_vectors = {query["query_id"]: engine.embed(query["text"]) for query in queries}
    finally:
        engine.close()
    isolation = isolate_lexical_index(database, pre_isolation, expected_nodes=len(documents))
    by_id = {row["logical_id"]: row for row in raw_documents}
    engine = fathomdb.Engine.open(str(database), use_default_embedder=True)
    try:
        if _model_name(engine) != protocol["model_name"]:
            raise ValueError("reopened embedder identity changed")
        raw_queries = []
        for query in queries:
            text_hits = engine.search_text_only(query["text"]).results
            hits = engine.search(query["text"], limit=protocol["result_limit"]).results
            raw_queries.append({
                "query_id": query["query_id"], "text": query["text"],
                "text_sha256": _bytes_hash(query["text"].encode()),
                "target_id": query["target_id"],
                "target_body_sha256": by_id[query["target_id"]]["body_sha256"],
                "vector": query_vectors[query["query_id"]],
                "text_only_count": len(text_hits),
                "hits": [
                    {"logical_id": hit.id.value, "body_sha256": _bytes_hash(hit.body.encode()),
                     "branch": hit.branch}
                    for hit in hits
                ],
            })
    finally:
        engine.close()
    raw = {
        "schema_version": 1, "version": version,
        "source_sha": specification["source_sha"],
        "wheel_sha256": specification["wheel_sha256"],
        "native_sha256": specification["native_sha256"],
        "protocol_sha256": _bytes_hash(protocol_bytes),
        "runner_sha256": sha256(Path(__file__).resolve()),
        "database_sha256": sha256(database),
        "pre_isolation_sha256": sha256(pre_isolation),
        "identity": identity,
        "environment": {
            "python_executable": str(Path(sys.executable).resolve()),
            "python_version": sys.version, "platform": platform.platform(),
        },
        "model_name": protocol["model_name"],
        "isolation": isolation,
        "documents": raw_documents,
        "queries": raw_queries,
    }
    path = output / "raw.json"
    path.write_text(json.dumps(raw, indent=2, sort_keys=True, allow_nan=False) + "\n")
    return path


def main() -> None:
    """Parse one identity-bound, fresh-database vector collection request."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--protocol", required=True, type=Path)
    parser.add_argument("--wheel", required=True, type=Path)
    parser.add_argument("--version", required=True, choices=("baseline", "candidate"))
    parser.add_argument("--source-root", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    print(run(
        protocol_path=args.protocol, wheel=args.wheel, version=args.version,
        source_root=args.source_root, output=args.output,
    ))


if __name__ == "__main__":
    main()
