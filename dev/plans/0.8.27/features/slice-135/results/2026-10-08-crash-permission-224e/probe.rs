use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{Engine, EngineError, InitialState, PreparedWrite, ReadView, SourceId};
use serde_json::json;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

struct FixedEmbedder;
impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice135-crash-probe", "v1", 384)
    }
    fn embed(&self, _: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0; 384])
    }
}

fn node(id: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".to_owned(),
        body: format!("durable body {id}"),
        source_id: SourceId::new("slice135:crash-probe").unwrap(),
        logical_id: Some(id.to_owned()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

#[test]
#[ignore = "victim controlled by parent"]
fn failed_commit_cleanup_victim() {
    let Ok(path) = std::env::var("SLICE135_CRASH_DB") else { return; };
    let marker = PathBuf::from(std::env::var("SLICE135_CRASH_MARKER").unwrap());
    let opened = Engine::open_with_embedder_for_test(path, Arc::new(FixedEmbedder)).unwrap();
    let engine = &opened.engine;
    engine.configure_vector_kind_for_test("doc").unwrap();
    let reported = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    engine.pause_projection_commit_failure_cleanup_for_test(reported.clone(), release);
    engine.force_next_projection_commit_failure_for_test();
    let receipt = engine.write(&[node("crash-pending")]).unwrap();
    reported.wait();
    assert!(engine.read_get("crash-pending", &ReadView::default()).unwrap().is_some());
    std::fs::write(marker, format!("{}", receipt.cursor)).unwrap();
    loop { thread::sleep(Duration::from_secs(1)); }
}

#[test]
fn process_kill_after_commit_failure_before_queue_cleanup_reopens() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("queue.sqlite");
    let marker = dir.path().join("ready");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "failed_commit_cleanup_victim", "--nocapture"])
        .env("SLICE135_CRASH_DB", &db)
        .env("SLICE135_CRASH_MARKER", &marker)
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !marker.exists() {
        assert!(child.try_wait().unwrap().is_none(), "victim exited before queue marker");
        assert!(Instant::now() < deadline, "queue marker timeout");
        thread::sleep(Duration::from_millis(10));
    }
    let cursor: u64 = std::fs::read_to_string(&marker).unwrap().parse().unwrap();
    child.kill().unwrap();
    let status = child.wait().unwrap();
    let opened = Engine::open_with_embedder_for_test(&db, Arc::new(FixedEmbedder)).unwrap();
    let engine = &opened.engine;
    engine.drain(30_000).unwrap();
    let row = engine.read_get("crash-pending", &ReadView::default()).unwrap().unwrap();
    let vector = engine.has_vector_for_cursor_for_test(cursor).unwrap();
    let integrity = engine.query_text_col_for_test("PRAGMA integrity_check").unwrap();
    let failure_count = engine.projection_failure_count_for_test(cursor).unwrap();
    eprintln!("SLICE135_CRASH {}", json!({"point":"after failed projection commit and before queue cleanup", "victim_status":format!("{status:?}"), "cursor":cursor, "body":row.body, "vector":vector, "integrity":integrity, "failure_count":failure_count}));
    assert_eq!(row.body, "durable body crash-pending");
    assert!(vector);
    assert_eq!(integrity, vec!["ok"]);
    assert_eq!(failure_count, 0);
    engine.close().unwrap();
}

#[test]
fn persistent_connection_readonly_write_refusal_reopens() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("readonly.sqlite");
    let opened = Engine::open(&db).unwrap();
    let engine = &opened.engine;
    engine.write(&[node("base")]).unwrap();
    engine.execute_for_test("PRAGMA query_only=ON").unwrap();
    let errors: Vec<_> = (0..3).map(|n| engine.write(&[node(&format!("blocked-{n}"))]).unwrap_err()).collect();
    assert!(errors.iter().all(|e| matches!(e, EngineError::Storage)));
    engine.execute_for_test("PRAGMA query_only=OFF").unwrap();
    engine.write(&[node("recovery")]).unwrap();
    engine.close().unwrap();
    let reopened = Engine::open(&db).unwrap();
    let ids = ["base", "blocked-0", "blocked-1", "blocked-2", "recovery"];
    let state: Vec<_> = ids.iter().map(|id| reopened.engine.read_get(id, &ReadView::default()).unwrap().map(|r| r.body)).collect();
    let integrity = reopened.engine.query_text_col_for_test("PRAGMA integrity_check").unwrap();
    eprintln!("SLICE135_READONLY {}", json!({"point":"writer connection PRAGMA query_only=ON before governed write", "errors":errors.iter().map(|e| format!("{e:?}")).collect::<Vec<_>>(), "state":state, "integrity":integrity}));
    assert_eq!(state, vec![Some("durable body base".into()), None, None, None, Some("durable body recovery".into())]);
    assert_eq!(integrity, vec!["ok"]);
    reopened.engine.close().unwrap();
}

#[test]
fn persistent_sqlite_busy_refusal_reopens() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("busy.sqlite");
    let opened = Engine::open(&db).unwrap();
    let engine = &opened.engine;
    engine.write(&[node("base")]).unwrap();
    let locker = rusqlite::Connection::open(&db).unwrap();
    locker.execute_batch("BEGIN IMMEDIATE").unwrap();
    let started = Instant::now();
    let errors: Vec<_> = (0..3).map(|n| engine.write(&[node(&format!("busy-{n}"))]).unwrap_err()).collect();
    let elapsed_ms = started.elapsed().as_millis();
    assert!(errors.iter().all(|e| matches!(e, EngineError::Storage)));
    locker.execute_batch("ROLLBACK").unwrap();
    engine.write(&[node("recovery")]).unwrap();
    engine.close().unwrap();
    let reopened = Engine::open(&db).unwrap();
    let ids = ["base", "busy-0", "busy-1", "busy-2", "recovery"];
    let state: Vec<_> = ids.iter().map(|id| reopened.engine.read_get(id, &ReadView::default()).unwrap().map(|r| r.body)).collect();
    let integrity = reopened.engine.query_text_col_for_test("PRAGMA integrity_check").unwrap();
    eprintln!("SLICE135_BUSY {}", json!({"point":"external SQLite BEGIN IMMEDIATE lock around three governed writes", "errors":errors.iter().map(|e| format!("{e:?}")).collect::<Vec<_>>(), "elapsed_ms":elapsed_ms, "state":state, "integrity":integrity}));
    assert_eq!(state, vec![Some("durable body base".into()), None, None, None, Some("durable body recovery".into())]);
    assert_eq!(integrity, vec!["ok"]);
    reopened.engine.close().unwrap();
}
