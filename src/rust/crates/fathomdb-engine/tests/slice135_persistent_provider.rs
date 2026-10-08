//! Persistent provider failure, truthful terminal state, and explicit recovery.

#![cfg(debug_assertions)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::lifecycle::ProjectionStatus;
use fathomdb_engine::{Engine, InitialState, PreparedWrite, ReadView, SourceId};
use rusqlite::Connection;
use serde_json::json;
use tempfile::TempDir;

#[derive(Debug)]
struct SwitchableProvider {
    failing: AtomicBool,
    calls: AtomicUsize,
    projection_body_calls: AtomicUsize,
}

impl Embedder for SwitchableProvider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice135-persistent-provider", "rev-a", 8)
    }

    fn embed(&self, input: &str) -> Result<Vector, EmbedderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if input == "persistent provider fault body" {
            self.projection_body_calls.fetch_add(1, Ordering::SeqCst);
        }
        if self.failing.load(Ordering::SeqCst) {
            Err(EmbedderError::Failed { message: "persistent test fault".to_owned() })
        } else {
            Ok(vec![0.25; 8])
        }
    }
}

fn body(engine: &Engine) -> Option<String> {
    engine.read_get("provider-fault", &ReadView::default()).expect("point read").map(|row| row.body)
}

#[test]
fn persistent_provider_failure_stays_truthful_until_explicit_rebuild() {
    let retained = std::env::var_os("SLICE135_PERSISTENT_PROVIDER_ROOT").map(PathBuf::from);
    let temporary =
        if retained.is_none() { Some(TempDir::new().expect("temporary database")) } else { None };
    let directory = retained.as_deref().unwrap_or_else(|| temporary.as_ref().unwrap().path());
    std::fs::create_dir_all(directory).expect("database directory");
    let database = directory.join("provider-fault.sqlite");
    assert!(!database.exists(), "probe requires a fresh database");
    let provider = Arc::new(SwitchableProvider {
        failing: AtomicBool::new(true),
        calls: AtomicUsize::new(0),
        projection_body_calls: AtomicUsize::new(0),
    });

    let opened = Engine::open_with_embedder_for_test(&database, provider.clone()).expect("open");
    opened.engine.configure_vector_kind_for_test("doc").expect("vector kind");
    opened.engine.set_projection_retry_delays_for_test(&[0, 0, 0]);
    let receipt = opened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "doc".to_owned(),
            body: "persistent provider fault body".to_owned(),
            source_id: SourceId::new("test:slice135-persistent-provider").expect("source id"),
            logical_id: Some("provider-fault".to_owned()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .expect("canonical write");
    opened.engine.drain(10_000).expect("terminal failure drains");
    let failure_calls = provider.calls.load(Ordering::SeqCst);
    let failure_body_calls = provider.projection_body_calls.load(Ordering::SeqCst);
    let failed_body = body(&opened.engine);
    let failed_status = opened.engine.projection_status_for_test("doc").expect("failed status");
    let failure_count =
        opened.engine.projection_failure_count_for_test(receipt.cursor).expect("failure audit");
    let failed_vector =
        opened.engine.has_vector_for_cursor_for_test(receipt.cursor).expect("vector state");
    assert_eq!(failed_body.as_deref(), Some("persistent provider fault body"));
    assert_eq!(failed_status, ProjectionStatus::Failed);
    assert_eq!(failure_count, 1);
    assert!(!failed_vector);
    assert!(failure_body_calls >= 4, "all configured provider attempts must fail");
    opened.engine.close().expect("close after failure");

    provider.failing.store(false, Ordering::SeqCst);
    let reopened =
        Engine::open_with_embedder_for_test(&database, provider.clone()).expect("reopen");
    reopened.engine.drain(1_000).expect("terminal row does not retry");
    let reopened_body = body(&reopened.engine);
    let reopened_status =
        reopened.engine.projection_status_for_test("doc").expect("reopened status");
    let reopened_failure_count = reopened
        .engine
        .projection_failure_count_for_test(receipt.cursor)
        .expect("reopened failure audit");
    let reopened_vector =
        reopened.engine.has_vector_for_cursor_for_test(receipt.cursor).expect("reopened vector");
    let calls_before_rebuild = provider.calls.load(Ordering::SeqCst);
    let body_calls_before_rebuild = provider.projection_body_calls.load(Ordering::SeqCst);
    assert_eq!(reopened_body, failed_body);
    assert_eq!(reopened_status, ProjectionStatus::Failed);
    assert_eq!(reopened_failure_count, 1);
    assert!(!reopened_vector);
    assert_eq!(
        body_calls_before_rebuild, failure_body_calls,
        "reopen must not silently retry a terminal failure"
    );

    reopened.engine.rebuild_projections().expect("explicit projection recovery");
    reopened.engine.drain(10_000).expect("rebuild drains");
    let recovered_status =
        reopened.engine.projection_status_for_test("doc").expect("recovered status");
    let recovered_vector =
        reopened.engine.has_vector_for_cursor_for_test(receipt.cursor).expect("recovered vector");
    assert_eq!(recovered_status, ProjectionStatus::UpToDate);
    assert!(recovered_vector);
    assert_eq!(body(&reopened.engine), failed_body);
    reopened.engine.close().expect("close after recovery");

    let final_open =
        Engine::open_with_embedder_for_test(&database, provider.clone()).expect("final reopen");
    let final_status = final_open.engine.projection_status_for_test("doc").expect("final status");
    let final_vector =
        final_open.engine.has_vector_for_cursor_for_test(receipt.cursor).expect("final vector");
    let final_body = body(&final_open.engine);
    final_open.engine.close().expect("final close");
    let connection = Connection::open(&database).expect("independent SQLite open");
    let physical_rows: i64 = connection
        .query_row("SELECT count(*) FROM canonical_nodes", [], |row| row.get(0))
        .expect("physical canonical count");
    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .expect("SQLite integrity");
    eprintln!(
        "SLICE135_PERSISTENT_PROVIDER {}",
        json!({
            "case": "persistent_provider_failure_and_explicit_rebuild",
            "fault_point": "all provider calls fail through configured projection retry budget",
            "cursor": receipt.cursor,
            "failure_calls": failure_calls,
            "failure_body_calls": failure_body_calls,
            "failed": {"body": failed_body, "status": format!("{failed_status:?}"), "failure_audit_rows": failure_count, "has_vector": failed_vector},
            "reopened_before_rebuild": {"body": reopened_body, "status": format!("{reopened_status:?}"), "failure_audit_rows": reopened_failure_count, "has_vector": reopened_vector, "provider_calls": calls_before_rebuild, "projection_body_calls": body_calls_before_rebuild},
            "recovered": {"status": format!("{recovered_status:?}"), "has_vector": recovered_vector},
            "final_reopen": {"body": final_body, "status": format!("{final_status:?}"), "has_vector": final_vector},
            "physical_canonical_rows": physical_rows,
            "integrity_check": integrity,
        })
    );
    assert_eq!(final_status, ProjectionStatus::UpToDate);
    assert!(final_vector);
    assert_eq!(final_body, failed_body);
    assert_eq!(physical_rows, 1);
    assert_eq!(integrity, "ok");
}
