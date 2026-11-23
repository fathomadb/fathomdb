//! Real source-SDK D lifecycle controls; every selected case is a fresh Orin child.
//! Build with embed-cuda,rerank-cuda,cuda-pool-observer, then select one ignored
//! lifecycle_* case under the approved GPU reservation. Default test runs do not
//! qualify native hardware. No reset or observer-driven synchronization occurs.
#![cfg(all(feature = "embed-cuda", feature = "rerank-cuda", feature = "cuda-pool-observer"))]

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use fathomdb_sdk::{
    Engine, InitialState, OpenOptions, PreparedWrite, RerankOptions, RerankPassage, SourceId,
};
use serde_json::Value;

const TEXT: &str = "fathomdb sync allocator repair experiment";
const CHILD: &str = "FATHOMDB_SLICE130_CASE";

fn observation(phase: &str) -> Value {
    let raw = Engine::cuda_pool_observation_for_qualification().expect("passive observer");
    println!("slice130 {phase} {raw}");
    let value: Value = serde_json::from_str(&raw).expect("actual observation JSON");
    assert_eq!(value["pid"].as_u64(), Some(u64::from(std::process::id())));
    value
}

fn number(value: &Value, field: &str) -> u64 {
    value[field].as_str().expect("lossless string counter").parse().expect("u64 counter")
}

fn checked(value: &Value, expected: Option<&Value>) {
    assert_eq!(value["status"], "private", "{value}");
    assert_eq!(number(value, "live_models_before"), number(value, "live_models_after"));
    assert!(value["uuid"].as_str().is_some_and(|v| !v.is_empty()));
    assert!(value["pool_id"].as_str().is_some_and(|v| !v.is_empty()));
    let _ = number(value, "context_id");
    let _ = number(value, "used_current_bytes");
    let _ = number(value, "reserved_current_bytes");
    if let Some(expected) = expected {
        for field in ["pid", "uuid", "device_ordinal", "context_id", "pool_id"] {
            assert_eq!(value[field], expected[field], "changed {field}");
        }
    }
}

fn open(path: &std::path::Path) -> Engine {
    let options = OpenOptions { use_default_embedder: true, ..OpenOptions::default() };
    let engine = Engine::open(path, options).expect("real SDK open");
    engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: TEXT.into(),
            source_id: SourceId::new("slice130-lifecycle").expect("source"),
            logical_id: None,
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        }])
        .expect("real write");
    engine.drain(60_000).expect("accepted work drained");
    let vector = engine.embed(TEXT).expect("actual default-model forward");
    assert_eq!(vector.len(), 384);
    assert!(vector.iter().all(|v| v.is_finite()));
    engine
}

fn close_watchdog() -> Instant {
    let started = Instant::now();
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(5));
        eprintln!("slice130 close/drop-to-exit exceeded 5s");
        std::process::exit(124);
    });
    started
}

fn zero(expected: &Value, started: Instant) {
    loop {
        let value = observation("last_release");
        checked(&value, Some(expected));
        assert!(started.elapsed() <= Duration::from_secs(5), "late final observation");
        if number(&value, "live_models_after") == 0
            && number(&value, "used_current_bytes") == 0
            && number(&value, "reserved_current_bytes") == 0
        {
            return;
        }
        assert!(started.elapsed() < Duration::from_secs(5), "last model did not release");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn child(case: &str) {
    assert_eq!(std::env::consts::ARCH, "aarch64", "Orin-only live fixture");
    assert_eq!(std::env::var("FATHOMDB_REQUIRE_LIVE").as_deref(), Ok("1"));
    assert_eq!(observation("initial")["status"], "not_decided");
    let directory = tempfile::tempdir().expect("fresh DB directory");
    let first = open(&directory.path().join("first.fdb"));
    if case == "exit-alive" {
        let value = observation("unclosed");
        checked(&value, None);
        assert_eq!(number(&value, "live_models_after"), 1);
        assert!(number(&value, "used_current_bytes") > 0);
        println!("slice130 unclosed model exits normally");
        std::process::exit(0);
    }
    if case == "cls" || case == "reranker" {
        if case == "cls" {
            let vectors = fathomdb_sdk::embed_batch_cls(&[TEXT]).expect("CLS singleton forward");
            assert_eq!(vectors.len(), 1);
            assert_eq!(vectors[0].len(), 384);
        } else {
            let passages = [RerankPassage { id: 1, body: TEXT.into(), score: 0.5 }];
            let ranked = fathomdb_sdk::rerank(TEXT, &passages, 1, RerankOptions::default())
                .expect("reranker singleton forward");
            assert_eq!(ranked.len(), 1);
            assert!(ranked[0].ce_score.is_some_and(f64::is_finite));
        }
        let expected = observation("singleton_alive");
        checked(&expected, None);
        close_watchdog();
        first.close().expect("engine close with intentional singleton");
        let retained = observation("singleton_retained");
        checked(&retained, Some(&expected));
        assert!(number(&retained, "live_models_after") >= 1);
        assert!(number(&retained, "used_current_bytes") > 0);
        assert!(number(&retained, "reserved_current_bytes") > 0);
        return;
    }
    let second = open(&directory.path().join("second.fdb"));
    let same_input = second.embed(TEXT).expect("survivor reference");
    let before = observation("before_first_close");
    checked(&before, None);
    assert_eq!(number(&before, "live_models_after"), 2);
    if case == "drop" {
        let started = close_watchdog();
        drop(first);
        drop(second);
        zero(&before, started);
        return;
    }
    first.close().expect("first awaited close");
    let after = observation("after_first_close");
    checked(&after, Some(&before));
    assert_eq!(number(&after, "live_models_after"), 1);
    if case == "max" {
        assert!(number(&before, "reserved_current_bytes") > 0);
        assert_eq!(
            number(&after, "reserved_current_bytes"),
            number(&before, "reserved_current_bytes")
        );
    }
    assert_eq!(second.embed(TEXT).expect("surviving model forward"), same_input);
    let started = close_watchdog();
    second.close().expect("last awaited close");
    zero(&before, started);
}

fn run(case: &str) {
    let mut process = Command::new(std::env::current_exe().expect("current fixture"))
        .args(["--ignored", "--exact", "slice130_child", "--nocapture", "--test-threads=1"])
        .env(CHILD, case)
        .env("FATHOMDB_REQUIRE_LIVE", "1")
        .env("FATHOMDB_EMBED_DEVICE", "cuda:0")
        .env("FATHOMDB_RERANK_DEVICE", "cuda:0")
        .env("FATHOMDB_POOL_MODE", "on")
        .env("FATHOMDB_POOL_RELEASE_THRESHOLD", if case == "zero" { "0" } else { "max" })
        .env("HF_HUB_OFFLINE", "1")
        .env("TRANSFORMERS_OFFLINE", "1")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("fresh lifecycle child");
    let started = Instant::now();
    loop {
        if let Some(status) = process.try_wait().expect("child wait") {
            assert!(status.success(), "actual child exit {status}");
            return;
        }
        if started.elapsed() >= Duration::from_secs(120) {
            process.kill().expect("bounded child stop");
            let status = process.wait().expect("reap timed-out child");
            panic!("lifecycle child exceeded120s: {status}");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

macro_rules! case {
    ($name:ident, $value:literal) => {
        #[test]
        #[ignore = "requires approved reserved Orin CUDA12.6/model window"]
        fn $name() {
            run($value);
        }
    };
}
case!(lifecycle_max, "max");
case!(lifecycle_zero, "zero");
case!(lifecycle_drop, "drop");
case!(lifecycle_exit_alive, "exit-alive");
case!(lifecycle_cls_retains, "cls");
case!(lifecycle_reranker_retains, "reranker");

#[test]
#[ignore = "only spawned by the bounded fresh-process lifecycle parent"]
fn slice130_child() {
    child(&std::env::var(CHILD).expect("must select a parent lifecycle case"));
}
