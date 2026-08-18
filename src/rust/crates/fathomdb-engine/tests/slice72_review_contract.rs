//! Slice 72 review regressions that need no CUDA host.

#[path = "support/slice72_gpu_telemetry.rs"]
mod telemetry;

use telemetry::{ForwardRendezvous, Receipt, TelemetrySnapshot};

#[test]
fn selected_uuid_is_filtered_from_multi_gpu_rows_and_receipt_requires_all_phases() {
    let before = TelemetrySnapshot::parse_gpu_csv(
        "GPU-other, 3, 1, 8192, 400, 7792\nGPU-selected, 37, 12, 24576, 1200, 23376\n",
        "GPU-other, 9, other, 20\nGPU-selected, 4242, fathomdb, 840\n",
        "GPU-selected",
        4242,
        10,
    )
    .expect("selected row is retained from a multi-GPU sample");
    assert_eq!(before.gpu_uuid, "GPU-selected");
    assert_eq!(before.compute_app.as_ref().map(|app| app.pid), Some(4242));
    assert_eq!(before.compute_app.as_ref().map(|app| app.used_vram_bytes), Some(840 * 1024 * 1024));

    let mut receipt = Receipt::for_test("review", "GPU-selected", 4242);
    receipt.push_phase("before_warm", before.clone()).expect("before warm");
    let directory = tempfile::tempdir().expect("receipt directory");
    assert!(receipt.write_success(directory.path()).is_err(), "overlap phase is mandatory");
    receipt.push_phase("warmed", before.clone()).expect("warmed");
    receipt.push_phase("overlap", before).expect("overlap");
    receipt.write_success(directory.path()).expect("complete receipt");
}

#[test]
fn rendezvous_is_single_use_and_failure_receipt_is_retained() {
    let rendezvous = ForwardRendezvous::new();
    assert!(rendezvous.run_contract_fixture().overlaps());
    assert_eq!(rendezvous.capture_count(), 1);
    assert!(rendezvous.run_contract_fixture().is_err(), "later forwards cannot overwrite capture");

    let directory = tempfile::tempdir().expect("receipt directory");
    let receipt = Receipt::for_test("failure", "GPU-selected", 4242);
    receipt
        .write_failure(directory.path(), "typed_failure", "forced CUDA model load failed")
        .expect("failure receipt retained");
    assert!(directory.path().join("slice72-failure-4242.json").is_file());
}
