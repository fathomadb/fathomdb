#!/usr/bin/env python3
"""Deterministic builder and integrity verifier for the closed FathomDB 0.8.32 Schema 39 golden DB.

Produces `/mnt/data/worktrees/fathomdb-0.8.32-golden/golden-0.8.32-schema39.sqlite`
outside the repository and emits `golden-0.8.32-manifest.json` after verifying zero
WAL, erased-canary byte absence, `fathomdb doctor check-integrity --full --json`, and
`fathomdb doctor data-plane-integrity --json` strictly BEFORE `chmod 0444`.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sqlite3
import subprocess
from collections.abc import Callable, Sequence
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[5]
DEFAULT_OUT_DB = Path(
    "/mnt/data/worktrees/fathomdb-0.8.32-golden/golden-0.8.32-schema39.sqlite"
)
DEFAULT_MANIFEST = (
    Path(__file__).resolve().parent / "golden-0.8.32-manifest.json"
)
DEFAULT_CANARIES: tuple[bytes, ...] = (
    b"CANARY_SECRET_BODY_0832_ERASE",
    b"CANARY_SECRET_PROP_0832_ERASE",
    b"CANARY_SECRET_EDGE_0832_ERASE",
    b"CANARY_TEMP_PURGE_0832_ERASE",
)
EXPECTED_SCOPES: tuple[str, ...] = ("", "team-a", "team-b")
SHARED_PROJECTION_NAME = "domain_tier"
EXACT_I64_VALUE = 9007199254740993  # 2^53 + 1: verifies exact i64 fidelity beyond f64/JS safe int
DoctorRunner = Callable[[list[str]], subprocess.CompletedProcess[str]]


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
    runner: DoctorRunner | None = None,
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
    proc = (
        subprocess.run(cmd, text=True, capture_output=True, check=False)
        if runner is None
        else runner(cmd)
    )

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


def verify_doctor_data_plane_integrity(
    db_path: Path,
    fathomdb_bin: Path,
    *,
    runner: DoctorRunner | None = None,
) -> dict[str, Any]:
    """Run `fathomdb doctor data-plane-integrity --json` and fail closed on any finding."""
    cmd = [
        str(fathomdb_bin),
        "doctor",
        "data-plane-integrity",
        "--json",
        str(db_path),
    ]
    proc = (
        subprocess.run(cmd, text=True, capture_output=True, check=False)
        if runner is None
        else runner(cmd)
    )

    if proc.returncode != 0:
        raise RuntimeError(
            f"doctor data-plane-integrity failed with exit code {proc.returncode}: "
            f"stdout={proc.stdout.strip()!r} stderr={proc.stderr.strip()!r}"
        )
    try:
        payload = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        raise RuntimeError(
            f"doctor data-plane-integrity produced invalid JSON: {exc}; stdout={proc.stdout!r}"
        ) from exc

    if not isinstance(payload, dict):
        raise RuntimeError(
            f"doctor data-plane-integrity JSON must be an object, got {type(payload)}"
        )

    status = payload.get("status")
    report = payload.get("report")
    if status != "clean" or not isinstance(report, dict):
        raise RuntimeError(
            f"doctor data-plane-integrity not clean: status={status!r}, payload={payload!r}"
        )
    findings = report.get("findings")
    complete = report.get("complete")
    if complete is not True or not isinstance(findings, list) or len(findings) != 0:
        raise RuntimeError(
            f"doctor data-plane-integrity reported incomplete or non-empty findings: "
            f"complete={complete!r}, findings={findings!r}"
        )
    return payload


def _audit_sqlite_state(db_path: Path, *, expect_vectors: bool) -> dict[str, Any]:
    """Read-only SQLite audit asserting Schema 39 tables, scopes, Gate1-D9 PK, and watermarks."""
    con = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
    try:
        user_version = int(con.execute("PRAGMA user_version").fetchone()[0])
        if user_version != 39:
            raise RuntimeError(f"expected PRAGMA user_version == 39, got {user_version}")

        # Verify composite PRIMARY KEY(entity_class, name) on _fathomdb_projection_registry (Gate1-D9).
        pk_cols = [
            col_name
            for _, col_name in sorted(
                (int(row[5]), str(row[1]))
                for row in con.execute("PRAGMA table_info(_fathomdb_projection_registry)").fetchall()
                if int(row[5]) > 0
            )
        ]
        if pk_cols != ["entity_class", "name"]:
            raise RuntimeError(
                f"expected composite PRIMARY KEY(entity_class, name) on _fathomdb_projection_registry, "
                f"got {pk_cols}"
            )
        shared_classes = sorted(
            str(row[0])
            for row in con.execute(
                "SELECT entity_class FROM _fathomdb_projection_registry WHERE name = ?",
                (SHARED_PROJECTION_NAME,),
            ).fetchall()
        )
        if shared_classes != ["edge", "node"]:
            raise RuntimeError(
                f"expected shared projection {SHARED_PROJECTION_NAME!r} on both 'edge' and 'node', "
                f"got {shared_classes}"
            )

        # Verify multi-scope presence across '', 'team-a', and 'team-b'.
        node_scopes = sorted(
            str(row[0])
            for row in con.execute("SELECT DISTINCT scope FROM canonical_nodes").fetchall()
        )
        edge_scopes = sorted(
            str(row[0])
            for row in con.execute("SELECT DISTINCT scope FROM canonical_edges").fetchall()
        )
        if tuple(node_scopes) != EXPECTED_SCOPES:
            raise RuntimeError(
                f"expected canonical_nodes scopes {list(EXPECTED_SCOPES)}, got {node_scopes}"
            )
        if tuple(edge_scopes) != EXPECTED_SCOPES:
            raise RuntimeError(
                f"expected canonical_edges scopes {list(EXPECTED_SCOPES)}, got {edge_scopes}"
            )

        # Verify _fathomdb_scope_visibility_watermarks is populated for all expected scopes.
        watermark_rows = con.execute(
            "SELECT scope, partition_kind, add_generation, max_cursor, destructive_generation "
            "FROM _fathomdb_scope_visibility_watermarks ORDER BY scope, partition_kind"
        ).fetchall()
        watermark_scopes = sorted({str(row[0]) for row in watermark_rows})
        if tuple(watermark_scopes) != EXPECTED_SCOPES:
            raise RuntimeError(
                f"expected _fathomdb_scope_visibility_watermarks scopes {list(EXPECTED_SCOPES)}, "
                f"got {watermark_scopes}"
            )

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
        if len(commit_timestamps) < 5:
            raise RuntimeError(
                f"expected >= 5 distinct _fathomdb_commit_timestamps batches, got {len(commit_timestamps)}"
            )

        # Verify 8-column vec0 schema (`vector_default` includes `scope TEXT`).
        vec0_ddl_row = con.execute(
            "SELECT sql FROM sqlite_master WHERE name = 'vector_default'"
        ).fetchone()
        vec0_ddl = str(vec0_ddl_row[0]) if vec0_ddl_row and vec0_ddl_row[0] else ""
        if "USING vec0(" not in vec0_ddl or "scope TEXT" not in vec0_ddl:
            raise RuntimeError(
                f"expected 8-column vec0 vector_default with scope column, got DDL: {vec0_ddl!r}"
            )

        # Verify dense_body projection declares vector_kinds=['domain_fact'] in registry.
        vec_reg_row = con.execute(
            "SELECT vector_declared, vector_kinds FROM _fathomdb_projection_registry "
            "WHERE entity_class = 'node' AND name = 'dense_body'"
        ).fetchone()
        declared_vec_kinds = (
            json.loads(vec_reg_row[1])
            if vec_reg_row and int(vec_reg_row[0]) == 1 and vec_reg_row[1]
            else []
        )
        if "domain_fact" not in declared_vec_kinds:
            raise RuntimeError(
                f"expected 'domain_fact' in _fathomdb_projection_registry.vector_kinds for dense_body, "
                f"got {declared_vec_kinds!r}"
            )

        enrolled_vector_kinds = sorted(
            str(row[0])
            for row in con.execute("SELECT kind FROM _fathomdb_vector_kinds").fetchall()
        )
        if expect_vectors and "domain_fact" not in enrolled_vector_kinds:
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
        vector_spans_total = int(
            con.execute("SELECT COUNT(*) FROM _fathomdb_vector_spans").fetchone()[0]
        )
        if expect_vectors and (vector_rows_domain_fact < 3 or vector_spans_total < 3):
            raise RuntimeError(
                f"expected >= 3 materialized domain_fact vector rows & spans across scopes, "
                f"got rows={vector_rows_domain_fact}, spans={vector_spans_total}"
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
            "scope_visibility_watermarks_count": len(watermark_rows),
            "vector_rows_total": vector_rows_total,
            "vector_rows_domain_fact": vector_rows_domain_fact,
            "vector_spans_total": vector_spans_total,
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
        if (
            counts["canonical_property_values_node"] < 1
            or counts["canonical_property_values_edge"] < 1
        ):
            raise RuntimeError(
                f"expected both node and edge canonical_property_values rows: {counts}"
            )
        return {
            "schema_version": user_version,
            "scopes_present": node_scopes,
            "projection_registry_pk": pk_cols,
            "shared_projection_classes": shared_classes,
            "vector_default_ddl": vec0_ddl,
            "commit_timestamps": commit_timestamps,
            "enrolled_vector_kinds": enrolled_vector_kinds,
            "counts": counts,
        }
    finally:
        con.close()


def _unlink_existing_db_files(out_db: Path) -> None:
    for suffix in ("", "-wal", "-shm", ".lock"):
        p = Path(f"{out_db}{suffix}")
        if p.exists():
            os.chmod(p, 0o644)
            p.unlink()


def build_golden_schema39(
    out_db: Path,
    manifest_path: Path,
    *,
    fathomdb_bin: Path,
    use_default_embedder: bool = True,
    doctor_runner: DoctorRunner | None = None,
) -> dict[str, Any]:
    """Build, verify (strictly BEFORE chmod 0444), seal, and manifest the Schema 39 golden DB."""
    from fathomdb import (
        Engine,
        ProjectionRole,
        ProjectionSpec,
        ProjectionTarget,
        ScopePolicy,
        ScopeViolationError,
        read,
    )

    out_db.parent.mkdir(parents=True, exist_ok=True)
    _unlink_existing_db_files(out_db)

    # Phase 1: Default scope ('') setup, per-class ProjectionTarget (shared name on node & edge),
    # operational collections, typed inline properties, as_of history, and canary rows.
    eng = Engine.open(str(out_db), use_default_embedder=False)
    try:
        delta = eng.configure_projections(
            [
                ProjectionSpec(
                    name=SHARED_PROJECTION_NAME,
                    roles=frozenset({ProjectionRole.FILTERABLE}),
                    entity_class="node",
                ),
                ProjectionSpec(
                    name=SHARED_PROJECTION_NAME,
                    roles=frozenset({ProjectionRole.FILTERABLE}),
                    entity_class="edge",
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
                    entity_class="node",
                    vector=True,
                    vector_kinds=["domain_fact"],
                ),
            ]
        )
        expected_shared = {
            ProjectionTarget(name=SHARED_PROJECTION_NAME, entity_class="node"),
            ProjectionTarget(name=SHARED_PROJECTION_NAME, entity_class="edge"),
        }
        if not expected_shared.issubset(set(delta.built)):
            raise RuntimeError(
                f"expected both node and edge ProjectionTarget for {SHARED_PROJECTION_NAME!r}, "
                f"got built={delta.built!r}"
            )

        # Register operational collections (latest_state + append_only_log).
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

        # Batch 1 (default scope ''): initial canonical nodes + canary records for erasure/purge.
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
                    "body": "Verification report V1 confirming R1 schema 39 invariants.",
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
                    "body": "Schema 39 default-scope domain fact with 8-column vec0 scope support.",
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
                    "properties": {
                        "domain_tier": "silver",
                        "priority_rank": 50,
                        "verified": False,
                    },
                },
                {
                    "kind": "note",
                    "body": "Legacy-shape canonical node with NULL inline properties.",
                    "source_id": "src-keep-1",
                    "logical_id": "legacy:1",
                },
                {
                    "kind": "domain_fact",
                    "body": "Transient secret payload CANARY_SECRET_BODY_0832_ERASE for erasure.",
                    "source_id": "src-erase-1",
                    "logical_id": "secret:1",
                    "properties": {
                        "domain_tier": "temp",
                        "erasure_canary_marker": "CANARY_SECRET_PROP_0832_ERASE",
                        "priority_rank": 999,
                        "verified": False,
                    },
                },
                {
                    "kind": "note",
                    "body": "Temporary node CANARY_TEMP_PURGE_0832_ERASE destined for lifecycle purge.",
                    "source_id": "src-temp-1",
                    "logical_id": "temp:1",
                    "properties": {
                        "domain_tier": "temp",
                        "priority_rank": 998,
                        "verified": False,
                    },
                },
            ]
        )

        # Batch 2 (default scope ''): supersede req:1 -> v2, write v1 edge:verifies:1,
        # active edge:supports:1, secret edge, and operational collection rows.
        eng.write(
            [
                {
                    "kind": "note",
                    "body": "Updated architecture requirement R1 v2 ratified for schema 39.",
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
                            "domain_tier": "edge-gold-v1",
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
                        "body": "Domain fact supports schema 39 requirement",
                        "source_id": "src-keep-1",
                        "logical_id": "edge:supports:1",
                        "t_valid": 1700000000,
                        "properties": {
                            "domain_tier": "edge-platinum",
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
                        "body": "Secret edge CANARY_SECRET_EDGE_0832_ERASE",
                        "source_id": "src-erase-1",
                        "logical_id": "edge:secret:1",
                        "properties": {"domain_tier": "edge-temp", "weight_bp": 100},
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
                        "body": '{"event":"seeded-schema39"}',
                    }
                },
            ]
        )

        # Batch 3 (default scope ''): supersede edge:verifies:1 -> v2 for as_of history.
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
                            "domain_tier": "edge-gold-v2",
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
    finally:
        eng.close()

    # Phase 2: Explicit named scopes 'team-a' and 'team-b' via ScopePolicy.
    for team, rank_base in (("team-a", 100), ("team-b", 200)):
        scoped_eng = Engine.open(
            str(out_db),
            use_default_embedder=False,
            scope_policy=ScopePolicy.single(team),
        )
        try:
            scoped_eng.write(
                [
                    {
                        "kind": "note",
                        "body": f"Scoped operational note exclusive to {team}.",
                        "source_id": f"src-{team}",
                        "logical_id": f"{team}:note:1",
                        "properties": {
                            "domain_tier": f"{team}-tier",
                            "priority_rank": rank_base + 1,
                            "verified": True,
                            "exact_i64": EXACT_I64_VALUE,
                            "score_f": 0.91,
                        },
                    },
                    {
                        "kind": "domain_fact",
                        "body": f"Scoped vector-enrolled domain fact owned by {team}.",
                        "source_id": f"src-{team}",
                        "logical_id": f"{team}:fact:1",
                        "properties": {
                            "domain_tier": f"{team}-platinum",
                            "priority_rank": rank_base + 2,
                            "verified": True,
                            "exact_i64": EXACT_I64_VALUE,
                        },
                    },
                    {
                        "edge": {
                            "kind": "supports",
                            "from": f"{team}:fact:1",
                            "to": f"{team}:note:1",
                            "body": f"Scoped support edge inside {team}",
                            "source_id": f"src-{team}",
                            "logical_id": f"{team}:edge:1",
                            "t_valid": 1700086400,
                            "properties": {
                                "domain_tier": f"{team}-edge",
                                "weight_bp": 9000 + rank_base,
                                "active_flag": True,
                            },
                        }
                    },
                    {
                        "op_store": {
                            "collection": "agent_checkpoints",
                            "record_key": f"{team}-checkpoint",
                            "body": json.dumps({"scope": team, "status": "active"}),
                        }
                    },
                    {
                        "op_store": {
                            "collection": "agent_audit",
                            "record_key": f"{team}-evt-1",
                            "body": json.dumps({"scope": team, "event": "seeded"}),
                        }
                    },
                ]
            )
        finally:
            scoped_eng.close()

    # Phase 3: Verify strict scope read isolation and read-only write refusal.
    for own_scope, other_scope in (("team-a", "team-b"), ("team-b", "team-a")):
        ro_eng = Engine.open(
            str(out_db),
            use_default_embedder=False,
            scope_policy=ScopePolicy.read_only([own_scope]),
        )
        try:
            if read.get(ro_eng, f"{own_scope}:note:1") is None:
                raise RuntimeError(f"scoped reader {own_scope!r} could not read its own note")
            if read.get(ro_eng, f"{other_scope}:note:1") is not None:
                raise RuntimeError(
                    f"scope isolation breach: {own_scope!r} observed {other_scope!r} note"
                )
            if read.get(ro_eng, "req:1") is not None:
                raise RuntimeError(
                    f"scope isolation breach: {own_scope!r} observed default-scope req:1"
                )
            try:
                ro_eng.write(
                    [
                        {
                            "kind": "note",
                            "body": "must be refused",
                            "source_id": "src-ro-refused",
                            "logical_id": f"{own_scope}:blocked",
                        }
                    ]
                )
            except ScopeViolationError:
                pass
            else:
                raise RuntimeError(
                    f"read_only ScopePolicy for {own_scope!r} unexpectedly permitted write"
                )
        finally:
            ro_eng.close()

    # Phase 4: Unscoped reopen with embedder (when enabled) to materialize 8-column vec0 rows
    # across all scopes, verify readbacks, execute purge + erase_source, and re-drain.
    eng = Engine.open(str(out_db), use_default_embedder=use_default_embedder)
    try:
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

        # Verify operational collections readback.
        op_cols = {c.name: c.kind for c in read.operational_collections(eng)}
        expected_op_cols = {
            "agent_checkpoints": "latest_state",
            "agent_audit": "append_only_log",
        }
        if any(op_cols.get(name) != kind for name, kind in expected_op_cols.items()):
            raise RuntimeError(f"missing expected operational collections: {op_cols}")
        if len(read.collection(eng, "agent_audit", limit=20)) < 2:
            raise RuntimeError("expected >= 2 rows in agent_audit operational collection")

        # Verify unenrolled custom kind 'raw_stub' is surfaced in vector_unsupported_kinds.
        proj_status = read.projection_status(eng)
        unsupported_kinds = sorted(proj_status.vector_unsupported_kinds)
        if "raw_stub" not in unsupported_kinds:
            raise RuntimeError(
                f"expected 'raw_stub' in vector_unsupported_kinds, got {unsupported_kinds}"
            )

        # Lifecycle-purge temp:1 and physically erase src-erase-1 (secret:1 + edge:secret:1 + vector).
        eng.transition("temp:1", "deleted")
        eng.purge("temp:1")
        erase_report = eng.erase_source("src-erase-1")
        if use_default_embedder:
            eng.drain(timeout_s=300)
        schema_ver = int(eng.open_report().schema_version_after)
    finally:
        eng.close()

    # Phase 5 (CRITICAL ORDERING per AC32-60-08):
    # Strictly BEFORE chmod 0444: zero WAL, zero erased canary bytes, read-only SQLite audit,
    # `fathomdb doctor check-integrity --full --json`, and
    # `fathomdb doctor data-plane-integrity --json`.
    verify_erased_canaries_absent(out_db, DEFAULT_CANARIES)
    sqlite_audit = _audit_sqlite_state(out_db, expect_vectors=use_default_embedder)
    doctor_check_summary = verify_doctor_integrity(
        out_db, fathomdb_bin, runner=doctor_runner
    )
    doctor_dpi_summary = verify_doctor_data_plane_integrity(
        out_db, fathomdb_bin, runner=doctor_runner
    )

    # Seal read-only strictly AFTER all doctor integrity checks pass.
    os.chmod(out_db, 0o444)

    git_sha = subprocess.check_output(
        ["git", "-C", str(REPO_ROOT), "rev-parse", "HEAD"], text=True
    ).strip()

    manifest: dict[str, Any] = {
        "schema": "fathomdb.golden-fixture/v2",
        "producer_release": "0.8.32",
        "schema_version": schema_ver,
        "git_sha": git_sha,
        "path": str(out_db),
        "bytes": out_db.stat().st_size,
        "sha256": sha256_file(out_db),
        "file_mode": "0444",
        "use_default_embedder": use_default_embedder,
        "exact_i64_canary": EXACT_I64_VALUE,
        "scopes_present": sqlite_audit["scopes_present"],
        "shared_projection_name": SHARED_PROJECTION_NAME,
        "shared_projection_classes": sqlite_audit["shared_projection_classes"],
        "projection_registry_pk": sqlite_audit["projection_registry_pk"],
        "vector_default_ddl": sqlite_audit["vector_default_ddl"],
        "commit_timestamps": sqlite_audit["commit_timestamps"],
        "enrolled_vector_kinds": sqlite_audit["enrolled_vector_kinds"],
        "vector_unsupported_kinds": unsupported_kinds,
        "counts": sqlite_audit["counts"],
        "expected": {
            "scopes": list(EXPECTED_SCOPES),
            "default_scope_active_nodes": ["fact:1", "legacy:1", "raw:1", "req:1", "ver:1"],
            "team_a_active_nodes": ["team-a:fact:1", "team-a:note:1"],
            "team_b_active_nodes": ["team-b:fact:1", "team-b:note:1"],
            "null_property_node": "legacy:1",
            "superseded_node": "req:1",
            "default_scope_active_edges": ["edge:supports:1", "edge:verifies:1"],
            "scoped_active_edges": ["team-a:edge:1", "team-b:edge:1"],
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
        "doctor_check_integrity": doctor_check_summary,
        "doctor_data_plane_integrity": doctor_dpi_summary,
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
        help="Diagnostic override only; official Slice 60 archive runs with use_default_embedder=True.",
    )
    args = parser.parse_args(argv)

    fathomdb_bin = resolve_fathomdb_bin(args.fathomdb_bin)
    manifest = build_golden_schema39(
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
                "file_mode": manifest["file_mode"],
                "schema_version": manifest["schema_version"],
                "scopes_present": manifest["scopes_present"],
                "counts": manifest["counts"],
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
