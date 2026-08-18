//! Slice 72 trusted-runner contract: one CUDA-visible process can characterize
//! real BGE projection and TinyBERT CE work on the same selected device.
//!
//! This target is deliberately absent from routine test commands. See
//! `dev/design/0.8.23-slice-72-concurrent-gpu-characterization.md` for the
//! required cache-only, receipt, and runner activation contract.

#[path = "support/slice72_gpu_telemetry.rs"]
mod telemetry;

use telemetry::{ForwardRendezvous, Receipt, Slice72Run, TelemetrySnapshot};

#[test]
fn parser_and_receipt_reject_wrong_binding_and_non_monotonic_phases() {
    let snapshot = TelemetrySnapshot::parse_gpu_csv(
        "GPU-expected, 37, 12, 24576, 1200, 23376\n",
        "GPU-expected, 4242, fathomdb, 840\nGPU-other, 9, other, 20\n",
        "GPU-expected",
        4242,
        10,
    )
    .expect("selected UUID and test PID bind");
    assert_eq!(snapshot.gpu_uuid, "GPU-expected");
    assert!(snapshot.other_compute_pids.is_empty());

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
        "GPU-expected, 999, other, 840\n",
        "GPU-expected",
        4242,
        10,
    )
    .is_err());

    let mut receipt = Receipt::for_test("parser-contract", "GPU-expected", 4242);
    receipt.push_phase("warmed", snapshot.clone()).expect("first phase");
    assert!(receipt.push_phase("before_warm", snapshot).is_err());
}

#[test]
fn explicit_runner_activation_is_required() {
    assert!(Slice72Run::activation_from("", "").is_none());
    assert!(Slice72Run::activation_from("approved-nvidia", "").is_none());
    assert!(Slice72Run::activation_from("approved-nvidia", "1").is_some());
}

#[test]
fn basic_shared_cuda_device_runs_real_bge_and_ce() {
    let Some(run) = Slice72Run::preflight("basic") else {
        return;
    };
    run.basic_shared_cuda_device_runs_real_bge_and_ce();
}

#[test]
fn bounded_overlap_characterizes_shared_cuda_residency() {
    let Some(run) = Slice72Run::preflight("moderate") else {
        return;
    };
    run.bounded_overlap_characterizes_shared_cuda_residency();
}

#[test]
#[ignore = "requires FATHOMDB_SLICE72_STRESS=1 and an approved NVIDIA runner"]
fn stress_shared_cuda_device_is_bounded_and_records_outcome() {
    telemetry::run_stress_under_watchdog();
}

#[test]
#[ignore = "private Slice 72 stress watchdog child entrypoint"]
fn slice72_private_stress_watchdog_child_entrypoint() {
    if !telemetry::require_stress_watchdog_capability() {
        eprintln!(
            "PENDING_EXTERNAL Slice 72 private stress child requires parent watchdog capability"
        );
        return;
    }
    let Some(run) = Slice72Run::preflight("stress") else {
        return;
    };
    run.stress_shared_cuda_device_is_bounded_and_records_outcome();
}

#[test]
fn public_docs_state_dual_runtime_limits() {
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

#[test]
fn forward_rendezvous_requires_actual_forward_interval_overlap() {
    let rendezvous = ForwardRendezvous::new();
    let result = rendezvous.run_contract_fixture();
    assert!(result.overlaps());
}
