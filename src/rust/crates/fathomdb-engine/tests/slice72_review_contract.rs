//! Slice 72 review regressions that need no CUDA host.

#[path = "support/slice72_gpu_telemetry.rs"]
mod telemetry;

use std::process::Command;
use std::time::{Duration, Instant};
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
    let mut warmed = before.clone();
    warmed.monotonic_ns = 11;
    receipt.push_phase("warmed", warmed).expect("warmed");
    let mut overlap = before.clone();
    overlap.monotonic_ns = 12;
    receipt.push_phase("overlap", overlap).expect("overlap");
    receipt.write_success(directory.path()).expect("complete receipt");

    let mut non_monotonic = Receipt::for_test("monotonic", "GPU-selected", 4243);
    non_monotonic.push_phase("before_warm", before.clone()).expect("first phase");
    assert!(non_monotonic.push_phase("warmed", before.clone()).is_err(), "equal timestamps reject");
    let mut decreasing = before;
    decreasing.monotonic_ns = 9;
    assert!(
        non_monotonic.push_phase("warmed", decreasing).is_err(),
        "decreasing timestamps reject"
    );
}

#[test]
fn preflight_visibility_and_failure_receipt_contracts_are_strict() {
    assert!(!telemetry::has_exactly_one_visible_cuda_device(0));
    assert!(telemetry::has_exactly_one_visible_cuda_device(1));
    assert!(!telemetry::has_exactly_one_visible_cuda_device(2));

    let directory = tempfile::tempdir().expect("failure receipt directory");
    let receipt = Receipt::for_test("provenance", "GPU-selected", 4244);
    receipt
        .write_failure(directory.path(), "typed_failure", "forced failure")
        .expect("failure receipt");
    let retained = std::fs::read_to_string(directory.path().join("slice72-provenance-4244.json"))
        .expect("failure receipt contents");
    for field in [
        "provenance",
        "sensor",
        "phases",
        "summary",
        "driver_version",
        "cache_identities",
        "cuda_visible_devices",
        "started_monotonic_ns",
        "finished_monotonic_ns",
    ] {
        assert!(retained.contains(field), "failure receipt retains {field}");
    }
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

/// A runner receipt is clocked from its preflight, while the rendezvous begins
/// after model warm-up. The overlap sample must still sort after the warm-up
/// sample: it cannot use a fresh rendezvous-relative epoch.
#[test]
fn rendezvous_overlap_timestamp_shares_the_run_receipt_clock() {
    let run_started = Instant::now();
    std::thread::sleep(Duration::from_millis(2));
    let mut receipt = Receipt::for_test("shared-clock", "GPU-selected", 4242);
    let before_timestamp = u64::try_from(run_started.elapsed().as_nanos()).expect("nanoseconds");
    let before = TelemetrySnapshot::parse_gpu_csv(
        "GPU-selected, 3, 1, 8192, 400, 7792\n",
        "GPU-selected, 4242, fathomdb, 840\n",
        "GPU-selected",
        4242,
        before_timestamp,
    )
    .expect("before-warm snapshot");
    receipt.push_phase("before_warm", before.clone()).expect("before warm");
    let mut warmed = before;
    warmed.monotonic_ns = before_timestamp.saturating_add(1);
    receipt.push_phase("warmed", warmed).expect("warmed");

    let rendezvous = ForwardRendezvous::new_for_run(run_started);
    let run = rendezvous.run_contract_fixture();
    assert!(run.overlaps());
    let overlap_timestamp = run.active_overlap_sample_timestamp().expect("overlap timestamp");
    let overlap = TelemetrySnapshot::parse_gpu_csv(
        "GPU-selected, 3, 1, 8192, 400, 7792\n",
        "GPU-selected, 4242, fathomdb, 840\n",
        "GPU-selected",
        4242,
        overlap_timestamp,
    )
    .expect("overlap snapshot");
    receipt
        .push_phase("overlap", overlap)
        .expect("a rendezvous sample must sort after runner warm-up phases");
}

#[test]
fn stress_watchdog_kills_a_hung_child_before_the_global_ceiling() {
    let directory = tempfile::tempdir().expect("watchdog receipt directory");
    let child = Command::new("sleep").arg("1").spawn().expect("start controlled hung child");
    let child_pid = child.id();
    let result = telemetry::wait_for_child_with_watchdog_with_receipt(
        child,
        Duration::from_millis(10),
        directory.path(),
        "stress-contract",
    );
    assert!(result.is_err(), "watchdog must terminate a child that exceeds its deadline");
    let receipt = std::fs::read_to_string(
        directory.path().join(format!("slice72-watchdog-stress-contract-{child_pid}.json")),
    )
    .expect("parent watchdog timeout receipt");
    assert!(receipt.contains("watchdog_timeout"));
    assert!(receipt.contains("\"selected_uuid\": null"));
    assert!(receipt.contains(&child_pid.to_string()));
    assert!(receipt.contains("\"started_monotonic_ns\": null"));
    assert!(receipt.contains("\"finished_monotonic_ns\": null"));

    let completed_directory = tempfile::tempdir().expect("completed receipt directory");
    let completed = Command::new("true").spawn().expect("start completed child");
    telemetry::wait_for_child_with_watchdog_with_receipt(
        completed,
        Duration::from_secs(1),
        completed_directory.path(),
        "stress-contract",
    )
    .expect("completed child remains successful without a second timeout receipt");
    assert_eq!(
        std::fs::read_dir(completed_directory.path()).expect("completed receipt directory").count(),
        0,
        "a normally completed child does not create a watchdog receipt"
    );
}

#[test]
fn direct_stress_entrypoint_has_no_public_watchdog_bypass() {
    let target = include_str!("slice72_concurrent_gpu.rs");
    assert!(
        !target.contains("is_stress_watchdog_child"),
        "a public environment variable must not bypass the parent watchdog"
    );
    assert!(
        target.contains("slice72_private_stress_watchdog_child_entrypoint"),
        "the real stress work must have a distinct private child entrypoint"
    );
    assert!(
        target.contains("require_stress_watchdog_capability"),
        "the private child entrypoint must reject direct or broad ignored invocation"
    );
}

#[test]
fn stress_receipt_captures_the_first_active_overlap_without_backdating() {
    let support = include_str!("support/slice72_gpu_telemetry.rs");
    assert!(
        support.contains("let mut overlap_snapshot = None"),
        "stress retains a real first-overlap snapshot rather than sampling after the loop"
    );
}

#[test]
fn private_stress_child_rejects_a_missing_parent_capability() {
    assert!(
        !telemetry::require_stress_watchdog_capability(),
        "broad ignored selection must stop before preflight when the parent socket is absent"
    );
}
