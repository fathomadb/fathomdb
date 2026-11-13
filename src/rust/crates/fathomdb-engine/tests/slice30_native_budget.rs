#![cfg(target_os = "linux")]

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
