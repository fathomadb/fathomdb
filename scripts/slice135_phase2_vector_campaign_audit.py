#!/usr/bin/env python3
"""Audit paired installed-wheel vector fidelity from source, DB and raw vectors."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import random
import sqlite3
from typing import Any
import zipfile


SCORER_PATH = Path(__file__).with_name("slice135_phase2_vector_audit.py")
_SCORE_SPEC = importlib.util.spec_from_file_location("slice135_phase2_vector_audit", SCORER_PATH)
assert _SCORE_SPEC is not None and _SCORE_SPEC.loader is not None
_SCORER = importlib.util.module_from_spec(_SCORE_SPEC)
_SCORE_SPEC.loader.exec_module(_SCORER)

LEXICAL_TABLES = ("search_index", "search_index_v2", "search_index_edges")


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _bytes_hash(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _canonical_hash(rows: list[tuple[str, str, str, str]]) -> str:
    return _bytes_hash(json.dumps(rows, separators=(",", ":")).encode())


def _lexical_counts(connection: sqlite3.Connection) -> dict[str, int]:
    return {
        name: connection.execute(f"SELECT COUNT(*) FROM {name}").fetchone()[0]
        for name in LEXICAL_TABLES
    }


def _vector_tables_hash(connection: sqlite3.Connection) -> str:
    names = [row[0] for row in connection.execute(
        "SELECT name FROM sqlite_master WHERE type='table' "
        "AND (name LIKE 'vector_default_%' OR name IN "
        "('_fathomdb_vector_rows','_fathomdb_vector_kinds')) ORDER BY name"
    )]
    if "_fathomdb_vector_rows" not in names or "vector_default_rowids" not in names:
        raise ValueError("vector storage tables missing")
    digest = hashlib.sha256()
    for name in names:
        digest.update(name.encode())
        for row in connection.execute(f'SELECT * FROM "{name}" ORDER BY rowid'):
            digest.update(repr(row).encode())
    return digest.hexdigest()


def audit_databases(
    pre_isolation: Path, post_isolation: Path,
    expected_rows: list[tuple[str, str, str, str]], *, vector_count: int | None,
) -> dict[str, Any]:
    """Check exact canonical rows, preserved vector storage and removed FTS rows."""
    if pre_isolation.resolve() == post_isolation.resolve():
        raise ValueError("pre- and post-isolation databases are the same file")
    result: dict[str, Any] = {}
    vector_hashes: list[str] = []
    for label, path in (("pre", pre_isolation), ("post", post_isolation)):
        if not path.is_file():
            raise ValueError("database file missing")
        try:
            with sqlite3.connect(f"file:{path}?mode=ro", uri=True) as connection:
                if connection.execute("PRAGMA integrity_check").fetchone()[0] != "ok":
                    raise ValueError(f"{label}: SQLite integrity failed")
                rows = connection.execute(
                    "SELECT logical_id, kind, body, source_id FROM canonical_nodes ORDER BY logical_id"
                ).fetchall()
                if rows != sorted(expected_rows):
                    raise ValueError(f"{label}: canonical rows differ from source corpus")
                counts = _lexical_counts(connection)
                if label == "pre":
                    if counts["search_index"] != len(expected_rows) or counts["search_index_v2"] != len(expected_rows):
                        raise ValueError("pre-isolation lexical index incomplete")
                elif any(counts.values()):
                    raise ValueError("post-isolation lexical residue")
                if vector_count is not None:
                    for table in ("_fathomdb_vector_rows", "vector_default_rowids"):
                        count = connection.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0]
                        if count != vector_count:
                            raise ValueError(f"{label}: vector row denominator changed")
                    vector_hashes.append(_vector_tables_hash(connection))
                result[label] = counts
        except sqlite3.Error as error:
            raise ValueError(f"{label}: SQLite inspection failed") from error
    if vector_count is not None and vector_hashes[0] != vector_hashes[1]:
        raise ValueError("lexical isolation changed vector storage bytes")
    result["canonical_nodes"] = len(expected_rows)
    result["canonical_sha256"] = _canonical_hash(sorted(expected_rows))
    if vector_hashes:
        result["vector_storage_sha256"] = vector_hashes[0]
    return result


def paired_summary(
    scores: dict[str, dict[str, Any]], expected_query_ids: list[str],
) -> dict[str, Any]:
    """Compute fixed-denominator paired deltas and deterministic bootstrap CI."""
    if set(scores) != {"baseline", "candidate"} or not expected_query_ids:
        raise ValueError("incomplete paired fidelity receipt")
    for version in ("baseline", "candidate"):
        score = scores[version]
        if score.get("query_count") != len(expected_query_ids) or set(score.get("per_query", {})) != set(expected_query_ids):
            raise ValueError("paired query denominator differs")
    deltas = {
        query_id: scores["candidate"]["per_query"][query_id]
        - scores["baseline"]["per_query"][query_id]
        for query_id in expected_query_ids
    }
    values = list(deltas.values())
    mean = sum(values) / len(values)
    rng = random.Random(135)
    boot = sorted(
        sum(values[rng.randrange(len(values))] for _ in values) / len(values)
        for _ in range(2000)
    )
    return {
        "query_count": len(values), "mean_delta": mean,
        "per_query_delta": deltas,
        "delta_bootstrap_95pct": [boot[49], boot[1949]],
        "candidate_better": sum(value > 0 for value in values),
        "equal": sum(value == 0 for value in values),
        "candidate_worse": sum(value < 0 for value in values),
    }


def _query_text(row: dict[str, Any]) -> str:
    title = (row.get("title") or "").strip()
    body = row["body"].strip()
    if len(title) >= 6 and title.lower() != "untitled" and title != body:
        return title
    cleaned = " ".join(line.lstrip("-*#> \t") for line in body.splitlines()).strip()
    out = ""
    for char in cleaned[:140]:
        out += char
        if char in ".!?" and len(out.strip()) >= 12:
            break
    if not out.strip() or out.strip() == body:
        raise ValueError("selected query target has no nonverbatim title or lead")
    return out.strip()


def _expected_inputs(protocol: dict[str, Any], source_root: Path) -> tuple[list[dict[str, str]], list[dict[str, str]]]:
    corpus = protocol["corpus"]
    documents: list[dict[str, str]] = []
    queries: list[dict[str, str]] = []
    seen_bodies: set[str] = set()
    for item in corpus["sources"]:
        path = source_root / item["filename"]
        if _sha256(path) != item["sha256"]:
            raise ValueError(f"{item['name']}: source SHA-256 mismatch")
        sorted_rows = sorted(
            (json.loads(line) for line in path.read_text().splitlines() if line.strip()),
            key=lambda row: row["doc_id"],
        )
        rows = []
        for row in sorted_rows:
            if not row.get("body"):
                continue
            body_hash = _bytes_hash(json.dumps({"summary": row["body"]}).encode())
            if body_hash in seen_bodies:
                continue
            seen_bodies.add(body_hash)
            rows.append(row)
            if len(rows) == corpus["per_source"]:
                break
        if len(rows) != corpus["per_source"]:
            raise ValueError("source corpus denominator changed")
        for index, row in enumerate(rows):
            logical_id = f"{item['name']}-{index:03d}"
            body = json.dumps({"summary": row["body"]})
            documents.append({
                "logical_id": logical_id, "source_doc_id": row["doc_id"],
                "body": body, "body_sha256": _bytes_hash(body.encode()),
                "source_id": f"slice135-fidelity:{item['name']}",
            })
            if index % corpus["query_stride"] == 0:
                text = _query_text(row)
                queries.append({
                    "query_id": f"{item['name']}:{index:03d}", "text": text,
                    "text_sha256": _bytes_hash(text.encode()),
                    "target_id": logical_id,
                    "target_body_sha256": _bytes_hash(body.encode()),
                })
    return documents, queries


def _check_wheel(path: Path, specification: dict[str, str]) -> None:
    if _sha256(path) != specification["wheel_sha256"]:
        raise ValueError("wheel SHA-256 mismatch")
    try:
        with zipfile.ZipFile(path) as archive:
            native = archive.read("fathomdb/_fathomdb.abi3.so")
    except (OSError, KeyError, zipfile.BadZipFile) as error:
        raise ValueError("wheel native member unavailable") from error
    if _bytes_hash(native) != specification["native_sha256"]:
        raise ValueError("wheel native SHA-256 mismatch")


def audit_campaign(
    *, protocol_path: Path, runner_path: Path, source_root: Path,
    wheels: dict[str, Path], version_dirs: dict[str, Path],
) -> dict[str, Any]:
    """Recompute paired exact-f32 fidelity from frozen inputs and raw files."""
    if set(wheels) != {"baseline", "candidate"} or set(version_dirs) != {"baseline", "candidate"}:
        raise ValueError("paired version files missing")
    if version_dirs["baseline"].resolve() == version_dirs["candidate"].resolve():
        raise ValueError("paired versions share one database directory")
    protocol_bytes = protocol_path.read_bytes()
    protocol = json.loads(protocol_bytes)
    if protocol.get("schema_version") != 1 or set(protocol.get("versions", {})) != {"baseline", "candidate"}:
        raise ValueError("protocol schema or version set mismatch")
    if protocol.get("runner_sha256") != _sha256(runner_path):
        raise ValueError("runner bytes differ from frozen protocol")
    if protocol.get("auditor_sha256") != _sha256(Path(__file__).resolve()) or protocol.get("scorer_sha256") != _sha256(SCORER_PATH):
        raise ValueError("auditor bytes differ from frozen protocol")
    expected_docs, expected_queries = _expected_inputs(protocol, source_root)
    if len(expected_docs) != protocol["denominator"]["documents_per_version"] or len(expected_queries) != protocol["denominator"]["queries_per_version"]:
        raise ValueError("protocol denominator differs from sources")
    expected_rows = [
        (row["logical_id"], "doc", row["body"], row["source_id"])
        for row in expected_docs
    ]
    document_metadata = [
        {key: row[key] for key in ("logical_id", "source_doc_id", "body_sha256")}
        for row in expected_docs
    ]
    query_metadata = [
        {key: row[key] for key in ("query_id", "text", "text_sha256", "target_id", "target_body_sha256")}
        for row in expected_queries
    ]
    scores: dict[str, dict[str, Any]] = {}
    database_audits: dict[str, dict[str, Any]] = {}
    for version in ("baseline", "candidate"):
        specification = protocol["versions"][version]
        _check_wheel(wheels[version], specification)
        directory = version_dirs[version]
        raw = json.loads((directory / "raw.json").read_text())
        database = directory / "fidelity.sqlite"
        before = directory / "pre-isolation.sqlite"
        for key, expected in (
            ("schema_version", 1), ("version", version),
            ("source_sha", specification["source_sha"]),
            ("wheel_sha256", specification["wheel_sha256"]),
            ("native_sha256", specification["native_sha256"]),
            ("protocol_sha256", _bytes_hash(protocol_bytes)),
            ("runner_sha256", _sha256(runner_path)),
            ("database_sha256", _sha256(database)),
            ("pre_isolation_sha256", _sha256(before)),
            ("model_name", protocol["model_name"]),
        ):
            if raw.get(key) != expected:
                raise ValueError(f"{version}: {key} mismatch")
        if not isinstance(raw.get("identity"), dict) or not isinstance(raw.get("environment"), dict):
            raise ValueError(f"{version}: installed identity or environment missing")
        for key in ("module_path", "native_path"):
            if not isinstance(raw["identity"].get(key), str) or not raw["identity"][key]:
                raise ValueError(f"{version}: installed identity path missing")
        for key in ("python_executable", "python_version", "platform"):
            if not isinstance(raw["environment"].get(key), str) or not raw["environment"][key]:
                raise ValueError(f"{version}: environment field missing")
        observed_docs = raw.get("documents")
        observed_queries = raw.get("queries")
        if not isinstance(observed_docs, list) or not isinstance(observed_queries, list):
            raise ValueError(f"{version}: document or query receipt missing")
        if [
            {key: row.get(key) for key in ("logical_id", "source_doc_id", "body_sha256")}
            for row in observed_docs
        ] != document_metadata:
            raise ValueError(f"{version}: source-to-document mapping changed")
        if [
            {key: row.get(key) for key in ("query_id", "text", "text_sha256", "target_id", "target_body_sha256")}
            for row in observed_queries
        ] != query_metadata:
            raise ValueError(f"{version}: source-to-query mapping changed")
        if any(len(row.get("hits", [])) > protocol["result_limit"] for row in observed_queries):
            raise ValueError(f"{version}: result limit exceeded")
        database_audit = audit_databases(before, database, expected_rows, vector_count=len(expected_docs))
        isolation = raw.get("isolation")
        if not isinstance(isolation, dict) or any(
            isolation.get(key) != expected for key, expected in (
                ("pre", database_audit["pre"]),
                ("post", database_audit["post"]),
                ("canonical_count", database_audit["canonical_nodes"]),
                ("canonical_sha256", database_audit["canonical_sha256"]),
                ("pre_integrity", "ok"), ("post_integrity", "ok"),
            )
        ):
            raise ValueError(f"{version}: isolation witness mismatch")
        scores[version] = _SCORER.score_version(
            raw, [row["query_id"] for row in expected_queries],
            k=protocol["k"], dimension=protocol["dimension"],
        )
        database_audits[version] = database_audit
    return {
        "protocol_sha256": _bytes_hash(protocol_bytes),
        "scores": scores,
        "paired": paired_summary(scores, [row["query_id"] for row in expected_queries]),
        "databases": database_audits,
    }


def main() -> None:
    """Audit one complete paired campaign from explicit local input paths."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--protocol", required=True, type=Path)
    parser.add_argument("--runner", required=True, type=Path)
    parser.add_argument("--source-root", required=True, type=Path)
    parser.add_argument("--baseline-wheel", required=True, type=Path)
    parser.add_argument("--candidate-wheel", required=True, type=Path)
    parser.add_argument("--baseline-dir", required=True, type=Path)
    parser.add_argument("--candidate-dir", required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(audit_campaign(
        protocol_path=args.protocol, runner_path=args.runner, source_root=args.source_root,
        wheels={"baseline": args.baseline_wheel, "candidate": args.candidate_wheel},
        version_dirs={"baseline": args.baseline_dir, "candidate": args.candidate_dir},
    ), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
