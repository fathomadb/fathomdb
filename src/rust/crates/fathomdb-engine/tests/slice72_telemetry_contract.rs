//! Hardware-independent checks for Slice 72's trusted-runner harness.

#[path = "support/slice72_gpu_telemetry.rs"]
mod telemetry;

use telemetry::{ForwardRendezvous, Receipt, Slice72Run, TelemetrySnapshot};

#[test]
fn parser_receipt_activation_and_docs_contract_hold_without_a_gpu() {
    let snapshot = TelemetrySnapshot::parse_gpu_csv(
        "GPU-expected, 37, 12, 24576, 1200, 23376\n",
        "GPU-expected, 4242, fathomdb, 840\nGPU-other, 9, other, 20\n",
        "GPU-expected",
        4242,
        10,
    )
    .expect("selected UUID and PID bind");
    assert!(TelemetrySnapshot::parse_gpu_csv(
        "GPU-wrong, 37, 12, 24576, 1200, 23376\n",
        "GPU-wrong, 4242, fathomdb, 840\n",
        "GPU-expected",
        4242,
        10,
    )
    .is_err());
    assert!(TelemetrySnapshot::parse_gpu_csv(
        "GPU-expected, 37, 12, 24576, 1200, 23376\n",
        "GPU-expected, 9, other, 840\n",
        "GPU-expected",
        4242,
        10,
    )
    .is_err());
    let mut receipt = Receipt::for_test("contract", "GPU-expected", 4242);
    receipt.push_phase("warmed", snapshot.clone()).expect("first phase");
    assert!(receipt.push_phase("before_warm", snapshot).is_err());

    let mut retained = Receipt::for_test("retained", "GPU-expected", 4242);
    let mut warmed = TelemetrySnapshot::parse_gpu_csv(
        "GPU-expected, 37, 12, 24576, 1200, 23376\n",
        "GPU-expected, 4242, fathomdb, 840\n",
        "GPU-expected",
        4242,
        20,
    )
    .expect("warmed sample");
    warmed.process_cpu_time_ns = 50;
    let mut before = warmed.clone();
    before.monotonic_ns = 10;
    before.process_cpu_time_ns = 10;
    retained.push_phase("before_warm", before).expect("before sample");
    retained.push_phase("warmed", warmed).expect("warm sample");
    let directory = tempfile::tempdir().expect("receipt directory");
    retained.write_success(directory.path()).expect("write receipt");
    let receipt_json = std::fs::read_to_string(directory.path().join("slice72-retained-4242.json"))
        .expect("retained receipt");
    assert!(receipt_json.contains("fathomdb.slice72.concurrent_gpu.v1"));
    assert!(receipt_json.contains("cpu_utilization_percent"));
    assert!(Slice72Run::activation_from("approved-nvidia", "0").is_some());
    assert!(Slice72Run::activation_from("", "0").is_none());
    assert!(ForwardRendezvous::new().run_contract_fixture().overlaps());

    let readme = include_str!("../../../../../README.md");
    let embedder = include_str!("../../../../../docs/embedder.md");
    let python = include_str!("../../../../../docs/reference/python-api.md");
    let typescript = include_str!("../../../../../docs/reference/typescript-api.md");
    for document in [readme, embedder, python, typescript] {
        assert!(document.contains("FATHOMDB_RERANK_DEVICE"));
        assert!(document.contains("cuda:N"));
    }
    assert!(embedder.contains("same GPU"));
    assert!(embedder.contains("no resource manager"));
    assert!(embedder.contains("CPU-only"));
    assert!(!embedder.contains("legacy `cpu|cuda|cuda:N|metal` grammar"));
    assert!(!readme.contains("optional CPU cross-encoder rerank"));
}
