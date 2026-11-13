#![cfg(target_os = "linux")]

#[path = "../examples/slice30/native_budget.rs"]
mod native_budget;

use fathomdb_engine::{EmbedderChoice, Engine, InitialState, PreparedWrite, SourceId};
use proptest::prelude::*;
use proptest::test_runner::RngSeed;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn invoke(config: Value) -> (Value, PathBuf) {
    let root = tempfile::Builder::new().prefix("s30-native-budget-").tempdir().unwrap().keep();
    let binary = PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").unwrap())
        .join("release/examples/slice30_gd_diagnostic");
    let hash: String = Sha256::digest(std::fs::read(&binary).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    println!("native budget {} binary {hash}", root.display());
    let path = root.join("config.json");
    std::fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
    let out = root.join("out");
    let mut child = Command::new(binary)
        .arg(path)
        .arg(&out)
        .stdout(Stdio::from(std::fs::File::create(root.join("stdout.log")).unwrap()))
        .stderr(Stdio::from(std::fs::File::create(root.join("stderr.log")).unwrap()))
        .spawn()
        .unwrap();
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > Duration::from_secs(10) {
            let _ = child.kill();
            let cleanup = Instant::now() + Duration::from_secs(10);
            while Instant::now() < cleanup {
                if child.try_wait().unwrap().is_some() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            panic!("budget source fixture timeout NOT_RED");
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    assert!(
        status.success(),
        "native/setup NOT_RED {}",
        std::fs::read_to_string(root.join("stderr.log")).unwrap()
    );
    let report = serde_json::from_slice(&std::fs::read(out.join("budget.json")).unwrap()).unwrap();
    (report, out)
}

fn check_expired(which: usize, expired: u64) {
    let config = json!({"family":"NATIVE_BUDGET_CONTROL","work_deadline_ns":u64::MAX,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX});
    let (healthy, out) = invoke(config.clone());
    assert_eq!(healthy["open_calls"], 1, "real None-provider open/close control NOT_RED");
    assert!(out.join("store.sqlite").exists());
    let mut config = config;
    config[["work_deadline_ns", "hard_deadline_ns", "shared_deadline_ns"][which]] = json!(expired);
    let (report, out) = invoke(config);
    assert_eq!(
        report["open_calls"], 0,
        "expired absolute native admission must prevent real database open"
    );
    assert!(!out.join("store.sqlite").exists());
    assert_eq!(report["disposition"], "deadline_refused");
}
#[test]
fn expired_work_prevents_real_database_open() {
    check_expired(0, 0);
}
#[test]
fn expired_hard_prevents_real_database_open() {
    check_expired(1, 0);
}
#[test]
fn expired_shared_prevents_real_database_open() {
    check_expired(2, 0);
}
proptest! {
    #![proptest_config(ProptestConfig{cases:4,max_shrink_iters:0,failure_persistence:None,rng_seed:RngSeed::Fixed(0x301401),..ProptestConfig::default()})]
    #[test]
    fn any_expired_absolute_bound_prevents_database_work(which in 0usize..3,expired in 0u64..1000){check_expired(which,expired);}
}

fn drain_case(milliseconds: u64) {
    let root = tempfile::Builder::new().prefix("s30-real-drain-").tempdir().unwrap().keep();
    let engine =
        Engine::open_with_choice(root.join("store.sqlite"), EmbedderChoice::None).unwrap().engine;
    let deadline = native_budget::monotonic_ns().unwrap() + milliseconds * 1_000_000;
    let config = json!({"work_deadline_ns":deadline,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX});
    let report = native_budget::drain(&engine, &config).unwrap();
    engine.close().unwrap();
    std::fs::write(root.join("drain.json"), serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    assert_eq!(report["drain_calls"], 1, "actual ready None-store drain control");
    let expected = (deadline - report["start_ns"].as_u64().unwrap()) / 1_000_000;
    assert_eq!(
        report["timeout_ms"], expected,
        "fixed source timeout must use actual remaining whole milliseconds"
    );
    // The included source control keeps the shared module's real API path selected too.
    let out = root.join("control");
    native_budget::run_control(
        &json!({"work_deadline_ns":u64::MAX,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX}),
        &out,
    );
    assert!(out.join("store.sqlite").exists());
}
#[test]
fn actual_drain_receives_remaining_budget_instead_of_fresh_five_seconds() {
    drain_case(1500);
}
#[test]
fn submillisecond_or_expired_remaining_budget_refuses_actual_drain() {
    let root = tempfile::Builder::new().prefix("s30-zero-drain-").tempdir().unwrap().keep();
    let engine =
        Engine::open_with_choice(root.join("store.sqlite"), EmbedderChoice::None).unwrap().engine;
    let deadline = native_budget::monotonic_ns().unwrap() + 500_000;
    let config = json!({"work_deadline_ns":deadline,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX});
    let report = native_budget::drain(&engine, &config).unwrap();
    engine.close().unwrap();
    assert_eq!(report["drain_calls"], 0, "no drain when remaining whole milliseconds is zero");
    assert_eq!(report["disposition"], "deadline_refused");
}
proptest! {
    #![proptest_config(ProptestConfig{cases:4,max_shrink_iters:0,failure_persistence:None,rng_seed:RngSeed::Fixed(0x301402),..ProptestConfig::default()})]
    #[test]
    fn actual_drain_argument_is_floor_of_remaining_absolute_budget(milliseconds in 1000u64..4000){drain_case(milliseconds);}
}

fn copied_database_refusal(rows: usize) {
    let binary = std::env::current_exe().unwrap();
    let hash: String = Sha256::digest(std::fs::read(&binary).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let root =
        tempfile::Builder::new().prefix("s30-post-template-refusal-").tempdir().unwrap().keep();
    println!(
        "real copied database refusal {} test_binary={} sha256={hash}",
        root.display(),
        binary.display()
    );
    let source = root.join("template.sqlite");
    let engine = Engine::open_with_choice(&source, EmbedderChoice::None).unwrap().engine;
    let nodes: Vec<_> = (0..rows)
        .map(|i| PreparedWrite::Node {
            kind: "doc".into(),
            body: format!("actual copied Unicode é {i}"),
            source_id: SourceId::new("test:slice30-budget-report").unwrap(),
            logical_id: Some(format!("copied-{i}")),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect();
    engine.write(&nodes).unwrap();
    engine.close().unwrap();
    let copied = root.join("query.sqlite");
    std::fs::copy(&source, &copied).unwrap();
    let connection =
        rusqlite::Connection::open_with_flags(&copied, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let observed: usize =
        connection.query_row("SELECT count(*) FROM canonical_nodes", [], |row| row.get(0)).unwrap();
    assert_eq!(observed, rows, "actual governed DB/closed-copy controls NOT_RED");
    drop(connection);
    let config =
        json!({"work_deadline_ns":0,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX});
    assert!(native_budget::query_open(&config, &root).is_err());
    assert!(source.exists() && copied.exists());
    let report: Value =
        serde_json::from_slice(&std::fs::read(root.join("budget.json")).unwrap()).unwrap();
    println!("actual existing SQLite refusal {report}");
    assert_eq!(report["phase"], "query_open", "post-copy refusal must not claim untouched entry");
    assert!(
        report.get("sqlite_created").is_none(),
        "already copied SQLite must not be reported absent"
    );
    assert!(
        report.get("open_calls").is_none(),
        "initial zero-work counters do not apply after preparation"
    );
    assert_eq!(report["disposition"], "deadline_refused");
}

#[test]
fn post_template_refusal_does_not_claim_no_sqlite_work() {
    copied_database_refusal(2);
}
proptest! {
    #![proptest_config(ProptestConfig{cases:3,max_shrink_iters:0,failure_persistence:None,rng_seed:RngSeed::Fixed(0x301403),..ProptestConfig::default()})]
    #[test]
    fn copied_population_refusal_preserves_truthful_phase(rows in 1usize..4) { copied_database_refusal(rows); }
}
