//! 0.8.23 Slice 80.5 — the real-hardware arm of the Tegra GPU allocation
//! witness (AC80-15, AC80-16, AC80-17, AC80-20).
//!
//! Gated like Slice 72's GPU telemetry suite: it runs only when an operator
//! opts this host in, and otherwise **SKIPs with a named reason** printed to
//! stderr — never a silent pass (R80-12, AC80-20). The host-independent
//! contract lives in `gpu_allocation_witness.rs` and runs everywhere.
//!
//! Run it on a Jetson Orin with:
//!
//! ```text
//! export PATH=/usr/local/cuda-12.6/bin:$PATH
//! export LIBRARY_PATH=/usr/local/cuda-12.6/targets/aarch64-linux/lib:$LIBRARY_PATH
//! export LD_LIBRARY_PATH=/usr/local/cuda-12.6/targets/aarch64-linux/lib:/usr/lib/aarch64-linux-gnu/nvidia:$LD_LIBRARY_PATH
//! export CUDA_COMPUTE_CAP=87
//! FATHOMDB_SLICE80_GPU_WITNESS=1 \
//!   cargo test -p fathomdb-embedder --features embed-cuda \
//!   --test slice80_gpu_allocation_witness -- --nocapture
//! ```
//!
//! `FATHOMDB_SLICE80_WITNESS_OUT=<path>` additionally retains the canonical
//! record for `scripts/release/verify-tegra-gpu-witness.py`.

/// Opt-in switch for the real-hardware arm.
const OPT_IN: &str = "FATHOMDB_SLICE80_GPU_WITNESS";
/// Optional path the canonical witness record is written to.
const WITNESS_OUT: &str = "FATHOMDB_SLICE80_WITNESS_OUT";

#[cfg(not(feature = "embed-cuda"))]
#[test]
fn witness_skips_with_a_named_reason_without_the_cuda_feature() {
    use fathomdb_embedder::GpuWitnessSkip;

    // AC80-20: neither a pass nor a failure is claimed about GPU engagement.
    let skip = GpuWitnessSkip::CudaNotCompiled;
    eprintln!("{skip}");
    assert_eq!(skip.as_str(), "cuda_not_compiled");
    let _ = (OPT_IN, WITNESS_OUT);
}

#[cfg(feature = "embed-cuda")]
#[test]
fn tegra_gpu_allocation_witness_on_real_hardware() {
    use fathomdb_embedder::{
        diagnose_default_embedder_gpu_from_env, normalize_cuda_uuid,
        run_default_embedder_allocation_witness, sample_with_driver_initialized_only,
        AllocationWitnessConfig, GpuWitnessError, GpuWitnessSkip, DEFAULT_CONTROL_ALLOCATION_BYTES,
        DEFAULT_DELTA_FLOOR_BYTES, WITNESS_VECTOR_DIM,
    };

    // The arms below share one process-global environment variable and one
    // per-thread CUDA context, so they are ordered inside a single test rather
    // than raced across parallel test threads.
    if std::env::var(OPT_IN).as_deref() != Ok("1") {
        eprintln!("{} ({OPT_IN} is not 1)", GpuWitnessSkip::NotOptedIn);
        return;
    }

    // AC80-17, measured rather than asserted from a constant: sampling the
    // memory counter with the driver initialized but no context current is a
    // named typed error, never a zero sample. This must run before any device
    // is constructed on this thread.
    match sample_with_driver_initialized_only() {
        Err(GpuWitnessError::NoCudaContext { message }) => {
            eprintln!("AC80-17 no-context sample refused as required: {message}");
        }
        other => {
            panic!("expected a named no-CUDA-context failure before any device, got {other:?}")
        }
    }

    // AC80-16: an explicit CPU policy produces a named CPU-fallback outcome
    // and no witness at all.
    std::env::set_var("FATHOMDB_EMBED_DEVICE", "cpu");
    match run_default_embedder_allocation_witness(AllocationWitnessConfig::default()) {
        Err(GpuWitnessError::CpuFallback { reason }) => {
            eprintln!("AC80-16 CPU policy refused as required: {reason}");
        }
        other => panic!("expected a named CPU-fallback outcome under cpu policy, got {other:?}"),
    }

    std::env::set_var("FATHOMDB_EMBED_DEVICE", "cuda:0");
    let diagnostic = diagnose_default_embedder_gpu_from_env();
    if diagnostic.devices().is_empty() {
        eprintln!("{}", GpuWitnessSkip::NoVisibleCudaDevice);
        return;
    }

    let config = AllocationWitnessConfig::default();
    let witness = match run_default_embedder_allocation_witness(config) {
        Ok(witness) => witness,
        Err(error) => panic!("tegra gpu allocation witness failed [{}]: {error}", error.as_str()),
    };

    // AC80-15: ordinal, driver UUID, and a delta at or above the floor, all
    // re-derivable from the retained record (R80-13).
    assert_eq!(witness.device_ordinal_requested, 0);
    assert_eq!(witness.device_ordinal_actual, 0);
    assert_eq!(
        normalize_cuda_uuid(&witness.device_uuid),
        normalize_cuda_uuid(diagnostic.selected_uuid().expect("a selected CUDA UUID")),
    );
    assert_eq!(witness.delta_floor_bytes, DEFAULT_DELTA_FLOOR_BYTES);
    assert!(
        witness.delta_bytes >= i128::from(witness.delta_floor_bytes),
        "delta {} is below the declared floor {}",
        witness.delta_bytes,
        witness.delta_floor_bytes
    );
    assert_eq!(
        witness.delta_bytes,
        i128::from(witness.free_before_bytes) - i128::from(witness.free_after_bytes)
    );
    assert_eq!(witness.control_allocation_request_bytes, DEFAULT_CONTROL_ALLOCATION_BYTES);
    assert!(witness.control_delta_bytes >= i128::from(DEFAULT_CONTROL_ALLOCATION_BYTES));
    assert_eq!(witness.embedded_vector_dim, WITNESS_VECTOR_DIM);

    let record = witness.to_canonical_json();
    print!("{record}");
    if let Some(path) = std::env::var_os(WITNESS_OUT) {
        std::fs::write(&path, record.as_bytes()).expect("retain the canonical witness record");
        eprintln!("retained witness at {}", std::path::Path::new(&path).display());
    }
}
