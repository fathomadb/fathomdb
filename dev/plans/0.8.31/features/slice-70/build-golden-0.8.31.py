#!/usr/bin/env python3
"""Deterministic builder and integrity verifier for the closed FathomDB 0.8.31 Schema 38 golden DB.

Produces `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.31-schema38.sqlite`
outside the repository and emits `golden-0.8.31-manifest.json` for 0.8.34 migration
qualification (`R34-02`, `R34-05`).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sqlite3
import subprocess
from collections.abc import Callable, Sequence
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[5]
DEFAULT_OUT_DB = Path("/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.31-schema38.sqlite")
DEFAULT_MANIFEST = (
    Path(__file__).resolve().parent / "golden-0.8.31-manifest.json"
)
DEFAULT_CANARIES: tuple[bytes, ...] = (
    b"CANARY_SECRET_BODY_0831_ERASE",
    b"CANARY_SECRET_PROP_0831_ERASE",
    b"CANARY_SECRET_EDGE_0831_ERASE",
    b"CANARY_TEMP_PURGE_0831_ERASE",
)
EXACT_I64_VALUE = 9007199254740993  # 2^53 + 1: verifies exact i64 fidelity beyond f64/JS safe int


def sha256_file(path: Path) -> str:
    """Compute lowercase SHA-256 hex digest of a file."""
    digest = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def scan_file_for_canaries(
    path: Path,
    canaries: Sequence[bytes],
    *,
    chunk_size: int = 1 << 20,
) -> dict[str, int]:
    """Scan raw file bytes in streaming chunks (with boundary overlap) for canary occurrences."""
    if not path.is_file():
        raise FileNotFoundError(f"database file not found: {path}")
    active = [c for c in canaries if c]
    counts: dict[str, int] = {c.decode("utf-8", errors="replace"): 0 for c in active}
    if not active:
        return counts
    max_len = max(len(c) for c in active)
    overlap = max(0, max_len - 1)
    tail = b""
    with path.open("rb") as fh:
        while True:
            chunk = fh.read(chunk_size)
            if not chunk:
                break
            window = tail + chunk
            prefix_len = len(tail)
            for canary in active:
                key = canary.decode("utf-8", errors="replace")
                start = 0
                while True:
                    pos = window.find(canary, start)
                    if pos == -1:
                        break
                    # Only count matches whose end extends into the newly read chunk
                    # so overlapped prefix matches are never double-counted.
                    if pos + len(canary) > prefix_len:
                        counts[key] += 1
                    start = pos + 1
            tail = window[-overlap:] if overlap else b""
    return counts


def verify_erased_canaries_absent(db_path: Path, canaries: Sequence[bytes]) -> None:
    """Fail closed if a non-empty WAL exists or any erased canary byte sequence survives."""
    wal_path = Path(f"{db_path}-wal")
    if wal_path.exists() and wal_path.stat().st_size > 0:
        raise RuntimeError(
            f"closed golden DB has non-empty WAL ({wal_path.stat().st_size} bytes): {wal_path}"
        )
    hits = scan_file_for_canaries(db_path, canaries)
    leaked = {k: v for k, v in hits.items() if v > 0}
    if leaked:
        raise RuntimeError(f"erased canary bytes reappeared in {db_path}: {leaked}")


def resolve_fathomdb_bin(explicit: Path | None = None) -> Path:
    """Resolve the compiled `fathomdb` CLI binary."""
    if explicit is not None:
        resolved = explicit.resolve()
        if not resolved.is_file():
            raise FileNotFoundError(f"--fathomdb-bin not found: {resolved}")
        return resolved
    for rel in ("target/release/fathomdb", "target/debug/fathomdb"):
        candidate = REPO_ROOT / rel
        if candidate.is_file():
            return candidate
    raise FileNotFoundError(
        "could not locate fathomdb CLI binary under target/release/fathomdb or target/debug/fathomdb"
    )


def verify_doctor_integrity(
    db_path: Path,
    fathomdb_bin: Path,
    *,
    runner: Callable[[list[str]], subprocess.CompletedProcess[str]] | None = None,
) -> dict[str, Any]:
    """Run `fathomdb doctor check-integrity --full --json` and fail closed on any finding."""
    cmd = [
        str(fathomdb_bin),
        "doctor",
        "check-integrity",
        "--full",
        "--json",
        str(db_path),
    ]
    if runner is None:
        proc = subprocess.run(cmd, text=True, capture_output=True, check=False)
    else:
        proc = runner(cmd)

    if proc.returncode != 0:
        raise RuntimeError(
            f"doctor check-integrity failed with exit code {proc.returncode}: "
            f"stdout={proc.stdout.strip()!r} stderr={proc.stderr.strip()!r}"
        )
    try:
        payload = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        raise RuntimeError(
            f"doctor check-integrity produced invalid JSON: {exc}; stdout={proc.stdout!r}"
        ) from exc

    if not isinstance(payload, dict):
        raise RuntimeError(f"doctor check-integrity JSON must be an object, got {type(payload)}")

    for section in ("physical", "logical", "semantic"):
        block = payload.get(section)
        if not isinstance(block, dict):
            raise RuntimeError(f"doctor check-integrity missing section {section!r}: {payload}")
        status = block.get("status")
        findings = block.get("findings")
        if status != "clean" or not isinstance(findings, list) or len(findings) != 0:
            raise RuntimeError(
                f"doctor check-integrity {section} not clean: status={status!r}, "
                f"findings={findings!r}"
            )
    return payload


def _audit_sqlite_state(db_path: Path, *, expect_vectors: bool) -> dict[str, Any]:
    """Read-only SQLite audit asserting Schema 38 tables, scope='', and zero erased remnants."""
    con = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
    try:
        user_version = int(con.execute("PRAGMA user_version").fetchone()[0])
        if user_version != 38:
            raise RuntimeError(f"expected PRAGMA user_version == 38, got {user_version}")

        # Verify scope == '' across every canonical and operational row table carrying scope.
        scope_tables = (
            "canonical_nodes",
            "canonical_edges",
            "operational_state",
            "operational_mutations",
        )
        scope_non_empty: dict[str, int] = {}
        for tbl in scope_tables:
            bad = int(
                con.execute(
                    f"SELECT COUNT(*) FROM {tbl} WHERE scope IS NULL OR scope != ''"
                ).fetchone()[0]
            )
            if bad != 0:
                scope_non_empty[tbl] = bad
        if scope_non_empty:
            raise RuntimeError(f"non-empty scope rows detected: {scope_non_empty}")

        # Verify erased logical IDs are completely absent.
        erased_nodes = int(
            con.execute(
                "SELECT COUNT(*) FROM canonical_nodes WHERE logical_id IN ('secret:1', 'temp:1')"
            ).fetchone()[0]
        )
        erased_edges = int(
            con.execute(
                "SELECT COUNT(*) FROM canonical_edges WHERE logical_id = 'edge:secret:1'"
            ).fetchone()[0]
        )
        if erased_nodes != 0 or erased_edges != 0:
            raise RuntimeError(
                f"erased records survived in canonical tables: nodes={erased_nodes}, edges={erased_edges}"
            )

        commit_timestamps = [
            {"write_cursor": int(row[0]), "committed_at_ms": int(row[1])}
            for row in con.execute(
                "SELECT write_cursor, committed_at_ms FROM _fathomdb_commit_timestamps ORDER BY write_cursor ASC"
            ).fetchall()
        ]
        if len(commit_timestamps) < 3:
            raise RuntimeError(
                f"expected >= 3 distinct _fathomdb_commit_timestamps batches, got {len(commit_timestamps)}"
            )

        enrolled_vector_kinds = sorted(
            str(row[0])
            for row in con.execute("SELECT kind FROM _fathomdb_vector_kinds").fetchall()
        )
        if "domain_fact" not in enrolled_vector_kinds:
            raise RuntimeError(
                f"expected 'domain_fact' in _fathomdb_vector_kinds, got {enrolled_vector_kinds}"
            )

        vector_rows_total = int(
            con.execute("SELECT COUNT(*) FROM _fathomdb_vector_rows").fetchone()[0]
        )
        vector_rows_domain_fact = int(
            con.execute(
                "SELECT COUNT(*) FROM _fathomdb_vector_rows WHERE kind = 'domain_fact'"
            ).fetchone()[0]
        )
        if expect_vectors and vector_rows_domain_fact < 1:
            raise RuntimeError(
                "expected materialized vector rows for enrolled custom kind 'domain_fact'"
            )

        counts = {
            "canonical_nodes_total": int(
                con.execute("SELECT COUNT(*) FROM canonical_nodes").fetchone()[0]
            ),
            "canonical_nodes_active": int(
                con.execute(
                    "SELECT COUNT(*) FROM canonical_nodes WHERE state = 'active' AND superseded_at IS NULL"
                ).fetchone()[0]
            ),
            "canonical_nodes_superseded": int(
                con.execute(
                    "SELECT COUNT(*) FROM canonical_nodes WHERE superseded_at IS NOT NULL"
                ).fetchone()[0]
            ),
            "canonical_nodes_null_properties": int(
                con.execute(
                    "SELECT COUNT(*) FROM canonical_nodes WHERE state = 'active' AND superseded_at IS NULL AND properties IS NULL"
                ).fetchone()[0]
            ),
            "canonical_edges_total": int(
                con.execute("SELECT COUNT(*) FROM canonical_edges").fetchone()[0]
            ),
            "canonical_edges_active": int(
                con.execute(
                    "SELECT COUNT(*) FROM canonical_edges WHERE superseded_at IS NULL"
                ).fetchone()[0]
            ),
            "canonical_edges_superseded": int(
                con.execute(
                    "SELECT COUNT(*) FROM canonical_edges WHERE superseded_at IS NOT NULL"
                ).fetchone()[0]
            ),
            "canonical_property_values_total": int(
                con.execute("SELECT COUNT(*) FROM canonical_property_values").fetchone()[0]
            ),
            "canonical_property_values_node": int(
                con.execute(
                    "SELECT COUNT(*) FROM canonical_property_values WHERE entity_class = 'node'"
                ).fetchone()[0]
            ),
            "canonical_property_values_edge": int(
                con.execute(
                    "SELECT COUNT(*) FROM canonical_property_values WHERE entity_class = 'edge'"
                ).fetchone()[0]
            ),
            "commit_timestamps_count": len(commit_timestamps),
            "vector_rows_total": vector_rows_total,
            "vector_rows_domain_fact": vector_rows_domain_fact,
            "operational_collections_count": int(
                con.execute("SELECT COUNT(*) FROM operational_collections").fetchone()[0]
            ),
            "operational_state_count": int(
                con.execute("SELECT COUNT(*) FROM operational_state").fetchone()[0]
            ),
            "operational_mutations_count": int(
                con.execute("SELECT COUNT(*) FROM operational_mutations").fetchone()[0]
            ),
        }
        if counts["canonical_nodes_null_properties"] < 1:
            raise RuntimeError("expected at least one legacy-shape properties=NULL active node")
        if counts["canonical_property_values_node"] < 1 or counts["canonical_property_values_edge"] < 1:
            raise RuntimeError(
                f"expected both node and edge canonical_property_values rows: {counts}"
            )
        return {
            "schema_version": user_version,
            "commit_timestamps": commit_timestamps,
            "enrolled_vector_kinds": enrolled_vector_kinds,
            "counts": counts,
        }
    finally:
        con.close()


def build_golden_schema38(
    out_db: Path,
    manifest_path: Path,
    *,
    fathomdb_bin: Path,
    use_default_embedder: bool = True,
) -> dict[str, Any]:
    """Build, verify, and manifest the closed FathomDB 0.8.31 Schema 38 golden archive."""
    from fathomdb import Engine, ProjectionRole, ProjectionSpec, read

    out_db.parent.mkdir(parents=True, exist_ok=True)
    for suffix in ("", "-wal", "-shm", ".lock"):
        p = Path(f"{out_db}{suffix}")
        if p.exists():
            p.unlink()

    eng = Engine.open(str(out_db), use_default_embedder=use_default_embedder)
    try:
        # 1. Declare Filterable node + edge projections and enroll custom vector kind 'domain_fact'.
        eng.configure_projections(
            [
                ProjectionSpec(
                    name="domain_tier",
                    roles=frozenset({ProjectionRole.FILTERABLE}),
                    entity_class="node",
                ),
                ProjectionSpec(
                    name="priority_rank",
                    roles=frozenset({ProjectionRole.FILTERABLE}),
                    entity_class="node",
                ),
                ProjectionSpec(
                    name="verified",
                    roles=frozenset({ProjectionRole.FILTERABLE}),
                    entity_class="node",
                ),
                ProjectionSpec(
                    name="weight_bp",
                    roles=frozenset({ProjectionRole.FILTERABLE}),
                    entity_class="edge",
                ),
                ProjectionSpec(
                    name="dense_body",
                    roles=frozenset({ProjectionRole.SEARCHABLE}),
                    vector=True,
                    vector_kinds=["domain_fact"],
                ),
            ]
        )

        # 2. Register operational collections (latest_state + append_only_log).
        eng.write(
            [
                {
                    "admin_schema": {
                        "name": "agent_checkpoints",
                        "kind": "latest_state",
                        "schema_json": '{"type":"object"}',
                        "retention_json": "{}",
                    }
                },
                {
                    "admin_schema": {
                        "name": "agent_audit",
                        "kind": "append_only_log",
                        "schema_json": '{"type":"object"}',
                        "retention_json": "{}",
                    }
                },
            ]
        )

        # 3. Batch 1: initial canonical nodes (v1 of req:1, ver:1, domain_fact fact:1,
        #    unenrolled raw_stub raw:1, legacy-shape properties=NULL legacy:1,
        #    plus secret:1 and temp:1 for later physical erasure / purge).
        eng.write(
            [
                {
                    "kind": "note",
                    "body": "Initial architecture requirement R1 v1 baseline specification.",
                    "source_id": "src-keep-1",
                    "logical_id": "req:1",
                    "properties": {
                        "domain_tier": "gold",
                        "priority_rank": 1,
                        "verified": False,
                        "confidence_f": 0.75,
                    },
                },
                {
                    "kind": "note",
                    "body": "Verification report V1 confirming R1 schema invariants.",
                    "source_id": "src-keep-1",
                    "logical_id": "ver:1",
                    "properties": {
                        "domain_tier": "gold",
                        "priority_rank": 2,
                        "verified": True,
                        "exact_i64": EXACT_I64_VALUE,
                        "ratio_f": 0.985,
                    },
                },
                {
                    "kind": "domain_fact",
                    "body": "Schema 38 inline properties store typed scalars outside canonical text body.",
                    "source_id": "src-keep-1",
                    "logical_id": "fact:1",
                    "properties": {
                        "domain_tier": "platinum",
                        "priority_rank": 10,
                        "verified": True,
                        "exact_i64": EXACT_I64_VALUE,
                    },
                },
                {
                    "kind": "raw_stub",
                    "body": "Unenrolled custom kind node retained for vector_unsupported_kinds proof.",
                    "source_id": "src-keep-1",
                    "logical_id": "raw:1",
                    "properties": {"domain_tier": "silver", "priority_rank": 50, "verified": False},
                },
                {
                    "kind": "note",
                    "body": "Legacy-shape canonical node with NULL inline properties.",
                    "source_id": "src-keep-1",
                    "logical_id": "legacy:1",
                },
                {
                    "kind": "domain_fact",
                    "body": "Transient secret payload CANARY_SECRET_BODY_0831_ERASE for erasure.",
                    "source_id": "src-erase-1",
                    "logical_id": "secret:1",
                    "properties": {
                        "domain_tier": "temp",
                        "erasure_canary_marker": "CANARY_SECRET_PROP_0831_ERASE",
                        "priority_rank": 999,
                        "verified": False,
                    },
                },
                {
                    "kind": "note",
                    "body": "Temporary node CANARY_TEMP_PURGE_0831_ERASE destined for lifecycle purge.",
                    "source_id": "src-temp-1",
                    "logical_id": "temp:1",
                    "properties": {"domain_tier": "temp", "priority_rank": 998, "verified": False},
                },
            ]
        )

        # 4. Batch 2: supersede req:1 -> v2, write v1 edge:verifies:1 with [t_valid, t_invalid),
        #    active edge:supports:1, secret edge, and operational collection rows.
        eng.write(
            [
                {
                    "kind": "note",
                    "body": "Updated architecture requirement R1 v2 ratified for schema 38.",
                    "source_id": "src-keep-2",
                    "logical_id": "req:1",
                    "properties": {
                        "domain_tier": "gold",
                        "priority_rank": 1,
                        "verified": True,
                        "exact_i64": EXACT_I64_VALUE,
                        "confidence_f": 0.99,
                    },
                },
                {
                    "edge": {
                        "kind": "verifies",
                        "from": "ver:1",
                        "to": "req:1",
                        "body": "Verification link v1 window",
                        "source_id": "src-keep-1",
                        "logical_id": "edge:verifies:1",
                        "t_valid": 1700000000,
                        "t_invalid": 1700086400,
                        "properties": {
                            "weight_bp": 8500,
                            "reviewer": "rev-v1",
                            "exact_i64": EXACT_I64_VALUE,
                            "active_flag": False,
                            "score_f": 0.85,
                        },
                    }
                },
                {
                    "edge": {
                        "kind": "supports",
                        "from": "fact:1",
                        "to": "req:1",
                        "body": "Domain fact supports schema 38 requirement",
                        "source_id": "src-keep-1",
                        "logical_id": "edge:supports:1",
                        "t_valid": 1700000000,
                        "properties": {
                            "weight_bp": 9500,
                            "reviewer": "rev-fact",
                            "exact_i64": EXACT_I64_VALUE,
                            "active_flag": True,
                            "score_f": 0.95,
                        },
                    }
                },
                {
                    "edge": {
                        "kind": "relates_to",
                        "from": "secret:1",
                        "to": "req:1",
                        "body": "Secret edge CANARY_SECRET_EDGE_0831_ERASE",
                        "source_id": "src-erase-1",
                        "logical_id": "edge:secret:1",
                        "properties": {"weight_bp": 100},
                    }
                },
                {
                    "op_store": {
                        "collection": "agent_checkpoints",
                        "record_key": "step-1",
                        "body": '{"status":"done","revision":1}',
                    }
                },
                {
                    "op_store": {
                        "collection": "agent_audit",
                        "record_key": "evt-1",
                        "body": '{"event":"seeded-schema38"}',
                    }
                },
            ]
        )

        # 5. Batch 3: supersede edge:verifies:1 -> v2 with distinct [t_valid, None) window.
        eng.write(
            [
                {
                    "edge": {
                        "kind": "verifies",
                        "from": "ver:1",
                        "to": "req:1",
                        "body": "Verification link v2 current window",
                        "source_id": "src-keep-2",
                        "logical_id": "edge:verifies:1",
                        "t_valid": 1700086400,
                        "properties": {
                            "weight_bp": 9800,
                            "reviewer": "rev-v2",
                            "exact_i64": EXACT_I64_VALUE,
                            "active_flag": True,
                            "score_f": 0.98,
                        },
                    }
                },
                {
                    "op_store": {
                        "collection": "agent_checkpoints",
                        "record_key": "step-2",
                        "body": '{"status":"verified","revision":2}',
                    }
                },
                {
                    "op_store": {
                        "collection": "agent_audit",
                        "record_key": "evt-2",
                        "body": '{"event":"superseded-edge"}',
                    }
                },
            ]
        )

        # Drain vectors so both fact:1 and secret:1 materialize in vector_default / _fathomdb_vector_rows
        # before secret:1 is physically erased.
        if use_default_embedder:
            eng.drain(timeout_s=300)

        # Verify exact i64 property readback and NULL legacy property before erasure/close.
        req_rec = read.get(eng, "req:1")
        if req_rec is None or req_rec.properties is None:
            raise RuntimeError("expected active req:1 with inline properties")
        if req_rec.properties.get("exact_i64") != EXACT_I64_VALUE:
            raise RuntimeError(
                f"i64 precision drift on req:1: got {req_rec.properties.get('exact_i64')!r}"
            )
        legacy_rec = read.get(eng, "legacy:1")
        if legacy_rec is None or legacy_rec.properties is not None:
            raise RuntimeError(f"expected legacy:1 properties=None, got {legacy_rec!r}")

        # Verify operational collections readback (built-in projection_failures may also be present).
        op_cols = {c.name: c.kind for c in read.operational_collections(eng)}
        expected_op_cols = {
            "agent_checkpoints": "latest_state",
            "agent_audit": "append_only_log",
        }
        if any(op_cols.get(name) != kind for name, kind in expected_op_cols.items()):
            raise RuntimeError(f"missing expected operational collections: {op_cols}")
        if len(read.collection(eng, "agent_audit", limit=10)) < 2:
            raise RuntimeError("expected >= 2 rows in agent_audit operational collection")

        # Verify unenrolled custom kind 'raw_stub' is surfaced in vector_unsupported_kinds.
        proj_status = read.projection_status(eng)
        unsupported_kinds = sorted(proj_status.vector_unsupported_kinds)
        if "raw_stub" not in unsupported_kinds:
            raise RuntimeError(
                f"expected 'raw_stub' in vector_unsupported_kinds, got {unsupported_kinds}"
            )

        # 6. Lifecycle-purge temp:1 and physically erase src-erase-1 (secret:1 + edge:secret:1 + vector).
        eng.transition("temp:1", "deleted")
        eng.purge("temp:1")
        erase_report = eng.erase_source("src-erase-1")
        if use_default_embedder:
            eng.drain(timeout_s=300)
        open_rep = eng.open_report()
        schema_ver = open_rep.schema_version_after
    finally:
        eng.close()

    # 7. Post-close verification (BEFORE chmod 0444): zero WAL, zero canary bytes,
    #    SQLite read-only invariant audit, and `fathomdb doctor check-integrity --full --json`.
    verify_erased_canaries_absent(out_db, DEFAULT_CANARIES)
    sqlite_audit = _audit_sqlite_state(out_db, expect_vectors=use_default_embedder)
    doctor_summary = verify_doctor_integrity(out_db, fathomdb_bin)

    git_sha = subprocess.check_output(
        ["git", "-C", str(REPO_ROOT), "rev-parse", "HEAD"], text=True
    ).strip()

    manifest: dict[str, Any] = {
        "schema": "fathomdb.golden-fixture/v1",
        "producer_release": "0.8.31",
        "schema_version": schema_ver,
        "git_sha": git_sha,
        "path": str(out_db),
        "bytes": out_db.stat().st_size,
        "sha256": sha256_file(out_db),
        "use_default_embedder": use_default_embedder,
        "exact_i64_canary": EXACT_I64_VALUE,
        "commit_timestamps": sqlite_audit["commit_timestamps"],
        "enrolled_vector_kinds": sqlite_audit["enrolled_vector_kinds"],
        "vector_unsupported_kinds": unsupported_kinds,
        "counts": sqlite_audit["counts"],
        "expected": {
            "active_nodes": ["fact:1", "legacy:1", "raw:1", "req:1", "ver:1"],
            "null_property_node": "legacy:1",
            "superseded_node": "req:1",
            "active_edges": ["edge:supports:1", "edge:verifies:1"],
            "superseded_edge": "edge:verifies:1",
            "erased_source": "src-erase-1",
            "erased_logical_ids": ["secret:1", "edge:secret:1"],
            "purged_logical_ids": ["temp:1"],
            "erased_nodes_count": int(erase_report.nodes_excised),
            "operational_collections": {
                "agent_checkpoints": "latest_state",
                "agent_audit": "append_only_log",
            },
            "canaries_verified_absent": [c.decode("utf-8") for c in DEFAULT_CANARIES],
        },
        "doctor_check_integrity": doctor_summary,
    }
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT_DB)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--fathomdb-bin", type=Path, default=None)
    parser.add_argument(
        "--no-default-embedder",
        action="store_true",
        help="Diagnostic override only; official Slice 70 archive runs with use_default_embedder=True.",
    )
    args = parser.parse_args(argv)

    fathomdb_bin = resolve_fathomdb_bin(args.fathomdb_bin)
    manifest = build_golden_schema38(
        args.out,
        args.manifest,
        fathomdb_bin=fathomdb_bin,
        use_default_embedder=not args.no_default_embedder,
    )
    print(
        json.dumps(
            {
                "path": manifest["path"],
                "sha256": manifest["sha256"],
                "bytes": manifest["bytes"],
                "schema_version": manifest["schema_version"],
                "counts": manifest["counts"],
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
