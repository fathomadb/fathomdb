//! Slice 135 first-pass real-database robustness probes.
//!
//! Run with `--nocapture --test-threads=1` under an external process timeout.
//! The ignored negative control deliberately fails and must be run separately.
#![cfg(debug_assertions)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use fathomdb_engine::{Engine, EngineError, InitialState, PreparedWrite, ReadView, SourceId};
use rusqlite::Connection;
use serde_json::json;
use tempfile::TempDir;

const CHILD_WAIT: Duration = Duration::from_secs(5);

fn node(id: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: format!("slice135robust body {id}"),
        source_id: SourceId::new("test:slice135-robustness").expect("source id"),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn snapshot(engine: &Engine, ids: &[String]) -> Vec<Option<String>> {
    engine
        .read_get_many(ids, &ReadView::default())
        .expect("point read")
        .into_iter()
        .map(|row| row.map(|row| row.body))
        .collect()
}

fn expected(ids: &[String]) -> Vec<Option<String>> {
    ids.iter().map(|id| Some(format!("slice135robust body {id}"))).collect()
}

fn integrity(path: &Path) -> String {
    Connection::open(path)
        .expect("SQLite open for supplementary integrity check")
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .expect("integrity check")
}

fn fd_count() -> Option<usize> {
    std::fs::read_dir("/dev/fd").ok().map(|entries| entries.count())
}

fn wal_bytes(path: &Path) -> Option<u64> {
    let wal = PathBuf::from(format!("{}-wal", path.display()));
    std::fs::metadata(wal).ok().map(|meta| meta.len())
}

#[test]
#[ignore = "negative control: runner requires this test to fail"]
fn slice135_negative_missing_record_must_fail() {
    let dir = TempDir::new().expect("tempdir");
    let opened = Engine::open(dir.path().join("negative.sqlite")).expect("open");
    let ids = vec!["absent".to_owned()];
    let observed = snapshot(&opened.engine, &ids);
    eprintln!(
        "SLICE135_ROBUSTNESS {}",
        json!({"case":"negative_missing_row", "expected":expected(&ids), "observed":observed})
    );
    assert_eq!(observed, expected(&ids), "negative control must detect missing record");
}

#[test]
fn slice135_concurrent_read_write_reopens_exact_state() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("concurrent.sqlite");
    let fd_before = fd_count();
    let opened = Engine::open(&path).expect("open");
    opened.engine.write(&[node("anchor")]).expect("seed anchor");
    let before_ids = vec!["anchor".to_owned()];
    let before = snapshot(&opened.engine, &before_ids);
    let engine = Arc::new(opened.engine);
    let start = Arc::new(Barrier::new(5));
    let stop = Arc::new(AtomicBool::new(false));
    let reads = Arc::new(AtomicUsize::new(0));
    let active_writers = Arc::new(AtomicUsize::new(0));
    let overlapping_reads = Arc::new(AtomicUsize::new(0));
    let mut writers = Vec::new();
    for writer_index in 0..2 {
        let engine = Arc::clone(&engine);
        let start = Arc::clone(&start);
        let active_writers = Arc::clone(&active_writers);
        writers.push(thread::spawn(move || {
            start.wait();
            active_writers.fetch_add(1, Ordering::SeqCst);
            for row_index in 0..8 {
                let id = format!("writer-{writer_index}-{row_index}");
                engine.write(&[node(&id)]).expect("concurrent write");
                thread::sleep(Duration::from_millis(1));
            }
            active_writers.fetch_sub(1, Ordering::SeqCst);
        }));
    }
    let mut readers = Vec::new();
    for _ in 0..2 {
        let engine = Arc::clone(&engine);
        let start = Arc::clone(&start);
        let stop = Arc::clone(&stop);
        let reads = Arc::clone(&reads);
        let active_writers = Arc::clone(&active_writers);
        let overlapping_reads = Arc::clone(&overlapping_reads);
        readers.push(thread::spawn(move || {
            start.wait();
            while !stop.load(Ordering::SeqCst) {
                let row = engine.read_get("anchor", &ReadView::default()).expect("concurrent read");
                assert_eq!(row.expect("anchor is durable").body, "slice135robust body anchor");
                reads.fetch_add(1, Ordering::SeqCst);
                if active_writers.load(Ordering::SeqCst) > 0 {
                    overlapping_reads.fetch_add(1, Ordering::SeqCst);
                }
            }
        }));
    }
    let started = Instant::now();
    start.wait();
    for writer in writers {
        writer.join().expect("writer join");
    }
    stop.store(true, Ordering::SeqCst);
    for reader in readers {
        reader.join().expect("reader join");
    }
    let mut ids = before_ids.clone();
    for writer_index in 0..2 {
        for row_index in 0..8 {
            ids.push(format!("writer-{writer_index}-{row_index}"));
        }
    }
    let after = snapshot(&engine, &ids);
    let read_count = reads.load(Ordering::SeqCst);
    let overlap_count = overlapping_reads.load(Ordering::SeqCst);
    engine.close().expect("bounded close");
    drop(engine);
    let reopened = Engine::open(&path).expect("reopen");
    let reopened_state = snapshot(&reopened.engine, &ids);
    reopened.engine.close().expect("reopened close");
    let integrity = integrity(&path);
    let fd_after = fd_count();
    eprintln!(
        "SLICE135_ROBUSTNESS {}",
        json!({"case":"concurrent_read_write", "setup":{"writers":2,"writes_per_writer":8,"readers":2,"seed":before_ids}, "fault_point":"bounded simultaneous read/write schedule", "timeout_ms":30000, "before":before, "after":after, "reopened":reopened_state, "expected":expected(&ids), "reads":read_count, "reads_while_writer_active":overlap_count, "elapsed_ms":started.elapsed().as_millis(), "integrity_check":integrity, "fd_before":fd_before, "fd_after":fd_after, "wal_bytes_after_close":wal_bytes(&path)})
    );
    assert_eq!(before, expected(&ids[..1]));
    assert!(read_count > 0, "readers must execute during writer schedule");
    assert!(overlap_count > 0, "readers must overlap the active writer schedule");
    assert_eq!(after, expected(&ids));
    assert_eq!(reopened_state, expected(&ids));
    assert_eq!(integrity, "ok");
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_for(path: &Path, child: &mut Child, budget: Duration) {
    let started = Instant::now();
    while !path.exists() {
        assert!(child.try_wait().expect("poll victim").is_none(), "victim exited before marker");
        assert!(started.elapsed() < budget, "victim marker timeout: {}", path.display());
        thread::sleep(Duration::from_millis(10));
    }
}

fn run_crash_phase(path: &Path, dir: &Path, phase: &str) -> std::process::ExitStatus {
    let ready = dir.join(format!("{phase}.ready"));
    let gate = dir.join(format!("{phase}.gate"));
    let done = dir.join(format!("{phase}.done"));
    let child = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", "slice135_crash_victim", "--ignored", "--nocapture"])
        .env("SLICE135_ROBUST_DB", path)
        .env("SLICE135_ROBUST_READY", &ready)
        .env("SLICE135_ROBUST_GATE", &gate)
        .env("SLICE135_ROBUST_DONE", &done)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn victim");
    let mut guard = ChildGuard(child);
    wait_for(&ready, &mut guard.0, CHILD_WAIT);
    if phase == "after_acknowledged_write" {
        std::fs::write(&gate, b"continue").expect("release victim");
        wait_for(&done, &mut guard.0, CHILD_WAIT);
    }
    guard.0.kill().expect("kill live victim");
    guard.0.wait().expect("reap victim")
}

#[test]
#[ignore = "subprocess victim; parent controls the FathomDB write boundary"]
fn slice135_crash_victim() {
    let Some(db) = std::env::var_os("SLICE135_ROBUST_DB") else {
        return;
    };
    let ready = PathBuf::from(std::env::var_os("SLICE135_ROBUST_READY").expect("ready marker"));
    let gate = PathBuf::from(std::env::var_os("SLICE135_ROBUST_GATE").expect("gate marker"));
    let done = PathBuf::from(std::env::var_os("SLICE135_ROBUST_DONE").expect("done marker"));
    let opened = Engine::open(db).expect("victim open");
    std::fs::write(ready, b"opened-before-write").expect("ready marker write");
    let started = Instant::now();
    while !gate.exists() {
        assert!(started.elapsed() < Duration::from_secs(15), "victim gate timeout");
        thread::sleep(Duration::from_millis(10));
    }
    opened.engine.write(&[node("after")]).expect("acknowledged child write");
    let observed = snapshot(&opened.engine, &["after".to_owned()]);
    assert_eq!(observed, expected(&["after".to_owned()]));
    std::fs::write(done, b"acknowledged-and-read-back").expect("done marker write");
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

#[test]
fn slice135_process_kill_before_and_after_acknowledged_write() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("crash.sqlite");
    let opened = Engine::open(&path).expect("seed open");
    opened.engine.write(&[node("base")]).expect("seed write");
    let before = snapshot(&opened.engine, &["base".to_owned(), "after".to_owned()]);
    opened.engine.close().expect("seed close");

    let started = Instant::now();
    let before_status = run_crash_phase(&path, dir.path(), "before_write");
    let reopened_before = Engine::open(&path).expect("reopen after before-write kill");
    let state_before = snapshot(&reopened_before.engine, &["base".to_owned(), "after".to_owned()]);
    reopened_before.engine.close().expect("close after before-write kill");
    let after_status = run_crash_phase(&path, dir.path(), "after_acknowledged_write");
    let reopened_after = Engine::open(&path).expect("reopen after acknowledged-write kill");
    let state_after = snapshot(&reopened_after.engine, &["base".to_owned(), "after".to_owned()]);
    reopened_after.engine.close().expect("close after acknowledged-write kill");
    let integrity = integrity(&path);
    eprintln!(
        "SLICE135_ROBUSTNESS {}",
        json!({"case":"process_kill_at_write_boundary", "setup":"seed base; child opens same database; first kill before write; second kill after write returns and point read confirms after", "fault_point":["opened_before_write", "after_acknowledged_write"], "timeout_ms":5000, "before":before, "reopened_before_write":state_before, "reopened_after_acknowledged_write":state_after, "expected_before_write":["slice135robust body base",null], "expected_after_acknowledged_write":["slice135robust body base","slice135robust body after"], "victim_exit_statuses":[format!("{before_status:?}"),format!("{after_status:?}")], "elapsed_ms":started.elapsed().as_millis(), "integrity_check":integrity, "wal_bytes_after_close":wal_bytes(&path), "power_loss_simulated":false})
    );
    assert_eq!(before, vec![Some("slice135robust body base".into()), None]);
    assert!(!before_status.success());
    assert_eq!(state_before, before);
    assert!(!after_status.success());
    assert_eq!(state_after, expected(&["base".to_owned(), "after".to_owned()]));
    assert_eq!(integrity, "ok");
}

#[test]
fn slice135_injected_pretransaction_refusal_preserves_reopened_state() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("fault.sqlite");
    let fd_before = fd_count();
    let opened = Engine::open(&path).expect("open");
    opened.engine.write(&[node("base")]).expect("seed write");
    let ids = ["base".to_owned(), "failed".to_owned(), "recovery".to_owned()];
    let before = snapshot(&opened.engine, &ids);
    opened.engine.force_next_commit_failure_for_test();
    let error = opened.engine.write(&[node("failed")]).expect_err("injected storage failure");
    let after_failure = snapshot(&opened.engine, &ids);
    opened.engine.write(&[node("recovery")]).expect("write after fault");
    let after_recovery = snapshot(&opened.engine, &ids);
    opened.engine.close().expect("close after fault");
    let reopened = Engine::open(&path).expect("reopen after fault");
    let reopened_state = snapshot(&reopened.engine, &ids);
    reopened.engine.close().expect("reopened close");
    let integrity = integrity(&path);
    let fd_after = fd_count();
    eprintln!(
        "SLICE135_ROBUSTNESS {}",
        json!({"case":"injected_pretransaction_refusal", "setup":"real database seeded with base", "fault_point":"Engine::force_next_commit_failure_for_test after validation, before transaction BEGIN", "timeout_ms":30000, "before":before, "error":format!("{error:?}"), "after_failure":after_failure, "after_recovery":after_recovery, "reopened":reopened_state, "expected_after_failure":["slice135robust body base",null,null], "expected_reopened":["slice135robust body base",null,"slice135robust body recovery"], "integrity_check":integrity, "fd_before":fd_before, "fd_after":fd_after, "wal_bytes_after_close":wal_bytes(&path)})
    );
    assert_eq!(before, vec![Some("slice135robust body base".into()), None, None]);
    assert_eq!(error, EngineError::Storage);
    assert_eq!(after_failure, before);
    assert_eq!(
        after_recovery,
        vec![
            Some("slice135robust body base".into()),
            None,
            Some("slice135robust body recovery".into())
        ]
    );
    assert_eq!(reopened_state, after_recovery);
    assert_eq!(integrity, "ok");
}

#[test]
fn slice135_bounded_sqlite_full_write_rolls_back_and_reopens() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("sqlite-full.sqlite");
    let opened = Engine::open(&path).expect("open");
    opened.engine.write(&[node("base")]).expect("seed write");
    opened.engine.drain(10_000).expect("seed projection");
    let ids = ["base".to_owned(), "oversized".to_owned(), "recovery".to_owned()];
    let before = snapshot(&opened.engine, &ids);
    let pages = opened.engine.query_i64_col_for_test("PRAGMA page_count").expect("page count")[0];
    let limit = pages + 1;
    opened
        .engine
        .execute_for_test(&format!("PRAGMA max_page_count={limit}"))
        .expect("cap only this temporary database");
    assert_eq!(
        opened.engine.query_i64_col_for_test("PRAGMA max_page_count").expect("page cap"),
        [limit]
    );
    let result = opened.engine.write(&[PreparedWrite::Node {
        kind: "doc".into(),
        body: "x".repeat(1024 * 1024),
        source_id: SourceId::new("test:slice135-robustness").expect("source id"),
        logical_id: Some("oversized".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }]);
    let after_failure = snapshot(&opened.engine, &ids);
    let error = result.expect_err("SQLite page cap must refuse oversized write");
    assert_eq!(error, EngineError::Storage);
    assert_eq!(after_failure, before, "SQLite-full write must not partly commit");
    opened
        .engine
        .execute_for_test("PRAGMA max_page_count=4294967294")
        .expect("remove temporary size cap");
    opened.engine.write(&[node("recovery")]).expect("write after full fault");
    opened.engine.close().expect("close after recovered write");
    let reopened = Engine::open(&path).expect("reopen");
    let after_reopen = snapshot(&reopened.engine, &ids);
    reopened.engine.close().expect("reopened close");
    let expected = vec![
        Some("slice135robust body base".into()),
        None,
        Some("slice135robust body recovery".into()),
    ];
    let physical_rows: i64 = Connection::open(&path)
        .expect("independent database open")
        .query_row("SELECT count(*) FROM canonical_nodes", [], |row| row.get(0))
        .expect("canonical row count");
    eprintln!(
        "SLICE135_ROBUSTNESS {}",
        json!({"case":"bounded_sqlite_full_write", "fault_point":"governed canonical write with max_page_count=page_count+1", "page_count":pages, "max_page_count":limit, "before":before, "error":format!("{error:?}"), "after_failure":after_failure, "reopened":after_reopen, "expected_reopened":expected, "physical_canonical_rows":physical_rows, "integrity_check":integrity(&path)})
    );
    assert_eq!(after_reopen, expected);
    assert_eq!(physical_rows, 2);
    assert_eq!(integrity(&path), "ok");
}
