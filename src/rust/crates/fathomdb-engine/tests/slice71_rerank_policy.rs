//! Public standalone rerank observes strict Slice 71 policy failures.

use fathomdb_engine::rerank_passages;
use std::sync::Mutex;

static RERANK_DEVICE_ENV_LOCK: Mutex<()> = Mutex::new(());

fn input() -> Vec<(u64, String, f64)> {
    vec![(1, "candidate".to_owned(), 1.0)]
}

#[test]
fn forced_cuda_does_not_become_a_cpu_rerank_on_a_cpu_artifact() {
    // This test binary is isolated from all other tests, and the resolution
    // happens before model-cache loading or inference.
    let _lock = RERANK_DEVICE_ENV_LOCK.lock().expect("environment lock");
    let previous = std::env::var_os("FATHOMDB_RERANK_DEVICE");
    unsafe { std::env::set_var("FATHOMDB_RERANK_DEVICE", "cuda:0") };
    let error = rerank_passages("query", input(), 1, 0.3, 1)
        .expect_err("forced CUDA must not silently score on CPU");
    assert!(error.contains("reranker device policy"));
    assert!(error.contains("built without CUDA"));
    restore(previous);
}

#[test]
fn malformed_policy_is_not_accepted_as_a_cpu_fallback() {
    let _lock = RERANK_DEVICE_ENV_LOCK.lock().expect("environment lock");
    let previous = std::env::var_os("FATHOMDB_RERANK_DEVICE");
    unsafe { std::env::set_var("FATHOMDB_RERANK_DEVICE", "cuda") };
    let error = rerank_passages("query", input(), 1, 0.3, 1)
        .expect_err("legacy bare CUDA spelling must fail");
    assert!(error.contains("invalid FATHOMDB_RERANK_DEVICE"));
    restore(previous);
}

fn restore(previous: Option<std::ffi::OsString>) {
    match previous {
        Some(value) => unsafe { std::env::set_var("FATHOMDB_RERANK_DEVICE", value) },
        None => unsafe { std::env::remove_var("FATHOMDB_RERANK_DEVICE") },
    }
}
