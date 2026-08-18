//! Slice 72 review regressions that need no CUDA host.

#[path = "support/slice72_gpu_telemetry.rs"]
mod telemetry;

use std::process::Command;
use std::time::Duration;
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

#[test]
fn guarded_operation_retains_panics_and_overlap_timestamp_is_inside_the_window() {
    let directory = tempfile::tempdir().expect("receipt directory");
    let receipt = Receipt::for_test("guarded", "GPU-selected", 4242);
    let result = telemetry::run_with_failure_receipt(
        &receipt,
        directory.path(),
        || -> Result<(), &'static str> { Err("typed runtime failure") },
    );
    assert!(result.is_err());
    assert!(directory.path().join("slice72-guarded-4242.json").is_file());

    let panic_directory = tempfile::tempdir().expect("panic receipt directory");
    let panic_receipt = Receipt::for_test("guarded-panic", "GPU-selected", 4242);
    assert!(std::panic::catch_unwind(|| {
        let _: Result<(), &'static str> = telemetry::run_with_failure_receipt(
            &panic_receipt,
            panic_directory.path(),
            || -> Result<(), &'static str> { panic!("forced operation panic") },
        );
    })
    .is_err());
    assert!(panic_directory.path().join("slice72-guarded-panic-4242.json").is_file());

    let rendezvous = ForwardRendezvous::new();
    let run = rendezvous.run_contract_fixture();
    assert!(run.overlaps());
    let timestamp = run.active_overlap_sample_timestamp().expect("active overlap timestamp");
    assert!(rendezvous.timestamp_is_within_captured_overlap(timestamp));
    assert!(
        rendezvous.active_overlap_sample_timestamp().is_none(),
        "a completed interval cannot be presented as an active telemetry sample"
    );
}

#[test]
fn stress_watchdog_kills_a_hung_child_before_the_global_ceiling() {
    let child =
        Command::new("sh").args(["-c", "sleep 1"]).spawn().expect("start controlled hung child");
    let result = telemetry::wait_for_child_with_watchdog(child, Duration::from_millis(10));
    assert!(result.is_err(), "watchdog must terminate a child that exceeds its deadline");
}
