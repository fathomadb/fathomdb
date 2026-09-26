//! R27-70C: a failed background projection commit leaves no durable residue.
//!
//! The cleanup pause fires after the forced commit error dropped the
//! uncommitted worker transaction and before redispatch, so a separate
//! connection observes exactly what that transaction left behind.

// Both hooks are `#[cfg(debug_assertions)]`-only.
#![cfg(debug_assertions)]

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{Engine, InitialState, PreparedWrite, SourceId};
use rusqlite::Connection;
use std::sync::{Arc, Barrier};
use tempfile::TempDir;

struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice70-residue", "v1", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![0.25; 8])
    }
}

struct FailingEmbedder;

impl Embedder for FailingEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice70-residue", "v1", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Err(EmbedderError::Failed { message: "slice70 always fails".to_string() })
    }
}

fn node(label: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".to_string(),
        body: format!(r#"{{"summary":"slice70 residue {label}"}}"#),
        source_id: SourceId::new("test:slice70-residue").expect("source id"),
        logical_id: Some(format!("slice70-residue-{label}")),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

/// Terminal, sidecar, and failure-audit rows for one cursor, plus every vec0
/// row (each fixture database holds exactly one write).
fn residue(path: &std::path::Path, cursor: u64) -> (u64, u64, u64, u64) {
    Connection::open(path)
        .expect("observer connection")
        .query_row(
            "SELECT \
               (SELECT COUNT(*) FROM _fathomdb_projection_terminal WHERE write_cursor=?1),\
               (SELECT COUNT(*) FROM _fathomdb_vector_rows WHERE write_cursor=?1),\
               (SELECT COUNT(*) FROM vector_default),\
               (SELECT COUNT(*) FROM operational_mutations \
                  WHERE collection_name='projection_failures' AND record_key=CAST(?1 AS TEXT))",
            [cursor],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("residue query")
}

fn arm_failure_pause(engine: &Engine) -> (Arc<Barrier>, Arc<Barrier>) {
    let reported = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    engine.force_next_projection_commit_failure_for_test();
    engine.pause_projection_commit_failure_cleanup_for_test(
        Arc::clone(&reported),
        Arc::clone(&release),
    );
    (reported, release)
}

#[test]
fn failed_success_commit_leaves_no_terminal_sidecar_or_vector() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("slice70-residue-success.db");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
    opened.engine.configure_vector_kind_for_test("doc").expect("vector kind");
    let (reported, release) = arm_failure_pause(&opened.engine);

    let cursor = opened.engine.write(&[node("success")]).expect("caller write").cursor;
    reported.wait();
    // Release before asserting: a panic while the worker is paused would
    // deadlock `Engine` drop instead of failing the test.
    let observed = residue(&path, cursor);
    release.wait();
    assert_eq!(observed, (0, 0, 0, 0), "rolled-back publication left residue");

    opened.engine.drain(10_000).expect("redispatch reaches idle");
    assert!(opened.engine.has_vector_for_cursor_for_test(cursor).expect("vector state"));
}

#[test]
fn failed_failure_outcome_commit_leaves_no_terminal_or_audit() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("slice70-residue-failed.db");
    let opened =
        Engine::open_with_embedder_for_test(&path, Arc::new(FailingEmbedder)).expect("open");
    opened.engine.configure_vector_kind_for_test("doc").expect("vector kind");
    opened.engine.set_projection_retry_delays_for_test(&[0, 0, 0]);
    let (reported, release) = arm_failure_pause(&opened.engine);

    let cursor = opened.engine.write(&[node("failed")]).expect("caller write").cursor;
    reported.wait();
    // Release before asserting: a panic while the worker is paused would
    // deadlock `Engine` drop instead of failing the test.
    let observed = residue(&path, cursor);
    release.wait();
    assert_eq!(observed, (0, 0, 0, 0), "rolled-back failure outcome left residue");

    opened.engine.drain(10_000).expect("redispatch reaches idle");
    assert_eq!(opened.engine.projection_failure_count_for_test(cursor).expect("audit"), 1);
}
