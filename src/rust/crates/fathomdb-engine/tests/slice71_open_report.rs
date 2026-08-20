//! Cross-encoder policy is surfaced independently from embedding state.

use fathomdb_engine::Engine;

#[test]
fn open_report_records_cpu_reranker_without_cuda_probe_claims() {
    let root = tempfile::tempdir().expect("temporary directory");
    let previous = std::env::var_os("FATHOMDB_RERANK_DEVICE");
    unsafe { std::env::set_var("FATHOMDB_RERANK_DEVICE", "cpu") };
    let opened = Engine::open(root.path().join("slice71.sqlite")).expect("open succeeds");
    let resolution = opened
        .report
        .reranker_device_resolution
        .expect("default-reranker feature reports its own policy");
    assert_eq!(format!("{:?}", resolution.effective_device), "Cpu");
    assert!(resolution.visible_cuda_devices.is_empty());
    assert_eq!(resolution.reason, None);
    match previous {
        Some(value) => unsafe { std::env::set_var("FATHOMDB_RERANK_DEVICE", value) },
        None => unsafe { std::env::remove_var("FATHOMDB_RERANK_DEVICE") },
    }
}
