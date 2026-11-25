//! 0.8.31 Slice 30 (`R31-04`, superseding Slice 20 G8 flag-and-count) — strict
//! write-time endpoint integrity: every edge written via `Engine::write` must
//! reference active, non-superseded canonical nodes (`superseded_at IS NULL AND
//! state = 'active'`). Violations fail atomically with
//! `EngineError::WriteValidation` and commit zero rows. Committed batches always
//! return `WriteReceipt.dangling_edge_endpoints == 0`.
//!
//! The probe is `logical_id`-alone against the partial/indexed
//! `canonical_nodes_logical_active_idx` (or `canonical_nodes_logical_sup_idx`)
//! with `AND state = 'active'`, running post-row-insert inside the write
//! transaction so same-batch later-inserted nodes succeed while every edge in
//! the batch (including in-batch superseded edges) is strictly validated.

use fathomdb_engine::{configure_runtime, Engine, EngineError, PreparedWrite, RuntimeSqliteMode};
use fathomdb_schema::{migrate, SQLITE_SUFFIX};
use rusqlite::Connection;
use tempfile::TempDir;

// The SQLite runtime latches on the first open in the process; a raw
// `Connection::open` in a parallel test would otherwise make every later
// `Engine::open` fail with `RuntimeConfiguration(TooLate)`.
fn configure_test_runtime() {
    configure_runtime(RuntimeSqliteMode::Performance).expect("configure test runtime");
}

fn db_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn node(kind: &str, body: &str, logical_id: Option<&str>) -> PreparedWrite {
    PreparedWrite::Node {
        kind: kind.to_string(),
        body: body.to_string(),
        source_id: fathomdb_engine::SourceId::new("test:fixture").expect("test source id"),
        logical_id: logical_id.map(str::to_string),
        state: fathomdb_engine::InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
    }
}

fn edge(kind: &str, from: &str, to: &str, logical_id: Option<&str>) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: kind.to_string(),
        from: from.to_string(),
        to: to.to_string(),
        source_id: fathomdb_engine::SourceId::new("test:fixture").expect("test source id"),
        logical_id: logical_id.map(str::to_string),
        body: None,
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        properties: None,
    }
}

fn count_rows(path: &std::path::Path, table: &str) -> i64 {
    Connection::open(path)
        .expect("open sqlite")
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .expect("count rows")
}

/// (a) — an edge with ONE missing endpoint (the other endpoint is a live node)
/// is strictly rejected with `WriteValidation` and rolls back all rows.
#[test]
fn s20_edge_to_one_missing_endpoint_is_rejected_atomically() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "one_missing");
    {
        let opened = Engine::open(&path).expect("open");
        let err = opened
            .engine
            .write(&[node("doc", "a", Some("A")), edge("rel", "A", "MISSING", None)])
            .expect_err("missing endpoint must be rejected");
        assert!(matches!(err, EngineError::WriteValidation), "got {err:?}");
        opened.engine.close().unwrap();
    }
    assert_eq!(count_rows(&path, "canonical_nodes"), 0);
    assert_eq!(count_rows(&path, "canonical_edges"), 0);
}

/// (b) — cross-row: an edge whose endpoints are nodes inserted LATER in the same
/// batch is NOT flagged and commits with `dangling_edge_endpoints == 0`.
#[test]
fn s20_same_batch_later_inserted_node_is_not_flagged() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(db_path(&dir, "same_batch")).expect("open");

    // Edge FIRST, then both its endpoints — exactly the bulk-loader ordering.
    let receipt = opened
        .engine
        .write(&[
            edge("rel", "N1", "N2", None),
            node("doc", "n1", Some("N1")),
            node("doc", "n2", Some("N2")),
        ])
        .expect("write");

    assert_eq!(
        receipt.dangling_edge_endpoints, 0,
        "same-batch later-inserted endpoints must commit cleanly"
    );
    opened.engine.close().unwrap();
}

/// (c) — an edge to a SUPERSEDED node (its active version tombstoned, no active
/// version remains) is strictly rejected with `WriteValidation`.
#[test]
fn s20_edge_to_superseded_node_is_rejected_atomically() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "superseded");

    // 1) Write an active node `S` through the engine.
    {
        let opened = Engine::open(&path).expect("open");
        opened.engine.write(&[node("doc", "s-v1", Some("S"))]).expect("write S");
        opened.engine.close().unwrap();
    }
    // 2) Tombstone S's only active version (no re-insert) — now S has zero active
    //    versions. The engine must be closed to take the raw connection.
    {
        let conn = Connection::open(&path).expect("open sqlite");
        let n = conn
            .execute(
                "UPDATE canonical_nodes SET superseded_at = 999
                 WHERE logical_id = 'S' AND superseded_at IS NULL",
                [],
            )
            .expect("tombstone S");
        assert_eq!(n, 1, "exactly one active S row tombstoned");
    }
    // 3) Write an edge -> S; S has no active version, so it is strictly rejected.
    {
        let opened = Engine::open(&path).expect("reopen");
        let err = opened
            .engine
            .write(&[edge("rel", "S", "S", None)])
            .expect_err("write edge to superseded must be rejected");
        assert!(matches!(err, EngineError::WriteValidation), "got {err:?}");
        opened.engine.close().unwrap();
    }
    assert_eq!(count_rows(&path, "canonical_edges"), 0);
}

/// (d) — strict mode rejects a batch with missing endpoints and commits zero
/// rows to disk.
#[test]
fn s20_strict_write_refuses_dangling_batch_atomically() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "strict_refuse");
    {
        let opened = Engine::open(&path).expect("open");
        let err = opened
            .engine
            .write(&[edge("rel", "GHOST_A", "GHOST_B", None)])
            .expect_err("write must reject dangling edge");
        assert!(matches!(err, EngineError::WriteValidation), "got {err:?}");
        opened.engine.close().unwrap();
    }
    assert_eq!(count_rows(&path, "canonical_edges"), 0);
}

/// (e) — a batch mixing a clean edge and a dangling edge fails atomically so
/// neither the nodes nor either edge is committed.
#[test]
fn s20_mixed_clean_and_dangling_batch_rolls_back_completely() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "mixed_rollback");
    {
        let opened = Engine::open(&path).expect("open");
        let err = opened
            .engine
            .write(&[
                node("doc", "p", Some("P")),
                node("doc", "q", Some("Q")),
                edge("rel", "P", "Q", None),
                edge("rel", "X1", "X2", None),
            ])
            .expect_err("batch with any dangling edge must reject atomically");
        assert!(matches!(err, EngineError::WriteValidation), "got {err:?}");
        opened.engine.close().unwrap();
    }
    assert_eq!(count_rows(&path, "canonical_nodes"), 0);
    assert_eq!(count_rows(&path, "canonical_edges"), 0);
}

/// (g) — strict mode checks EVERY edge in the batch, including an earlier edge
/// superseded later in the same batch under G0 `logical_id` rules. An earlier
/// dangling edge cannot slip into superseded history even if a later same-batch
/// edge replaces it with valid endpoints.
#[test]
fn s20_on_supersession_rejects_earlier_in_batch_dangling_edge() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "strict_supersession");
    {
        let opened = Engine::open(&path).expect("open");
        let err = opened
            .engine
            .write(&[
                node("doc", "a", Some("A")),
                edge("rel", "A", "GHOST", Some("E")),
                edge("rel", "A", "A", Some("F")),
                edge("rel", "A", "A", Some("E")),
            ])
            .expect_err("in-batch superseded dangling edge must be rejected");
        assert!(matches!(err, EngineError::WriteValidation), "got {err:?}");
        opened.engine.close().unwrap();
    }
    assert_eq!(count_rows(&path, "canonical_nodes"), 0);
    assert_eq!(count_rows(&path, "canonical_edges"), 0);
}

/// (f) [latency / plan gate] — the per-endpoint active probe hits
/// `canonical_nodes_scope_logical_active_idx` or `canonical_nodes_scope_logical_sup_idx`
/// via indexed `SEARCH` and does NOT do a `SCAN canonical_nodes`.
#[test]
fn s20_endpoint_probe_hits_partial_index_no_scan() {
    configure_test_runtime();
    let dir = TempDir::new().unwrap();
    let conn = Connection::open(db_path(&dir, "plan")).expect("open sqlite");
    migrate(&conn).expect("migrate to head");

    let plan: Vec<String> = {
        let mut stmt = conn
            .prepare(
                "EXPLAIN QUERY PLAN
                 SELECT 1 FROM canonical_nodes WHERE scope = ?1 AND logical_id = ?2 AND superseded_at IS NULL AND state = 'active' LIMIT 1",
            )
            .expect("prepare EXPLAIN");
        let rows = stmt
            .query_map(["", "any"], |row| row.get::<_, String>(3))
            .expect("query plan")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect plan");
        rows
    };
    let detail = plan.join(" | ");

    assert!(
        detail.contains("SEARCH canonical_nodes USING INDEX canonical_nodes_scope_logical_active_idx")
            || detail.contains("SEARCH canonical_nodes USING INDEX canonical_nodes_scope_logical_sup_idx"),
        "probe must use indexed SEARCH on canonical_nodes_scope_logical_active_idx or canonical_nodes_scope_logical_sup_idx; plan: {detail}"
    );
    assert!(
        !detail.contains("SCAN canonical_nodes"),
        "probe must not full-scan canonical_nodes; plan: {detail}"
    );
}

/// Legacy NULL `logical_id` nodes are not matchable by edge endpoints, so an
/// edge referencing them is strictly rejected with `WriteValidation` and rolls
/// back all rows.
#[test]
fn s20_legacy_null_logical_id_endpoints_are_rejected_atomically() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "legacy");
    {
        let opened = Engine::open(&path).expect("open");
        let err = opened
            .engine
            .write(&[
                node("doc", "l0", None),
                node("doc", "l1", None),
                edge("rel", "l0", "l1", None),
            ])
            .expect_err("edges referencing NULL-logical_id nodes must be rejected");
        assert!(matches!(err, EngineError::WriteValidation), "got {err:?}");
        opened.engine.close().unwrap();
    }
    assert_eq!(count_rows(&path, "canonical_nodes"), 0);
    assert_eq!(count_rows(&path, "canonical_edges"), 0);
}
