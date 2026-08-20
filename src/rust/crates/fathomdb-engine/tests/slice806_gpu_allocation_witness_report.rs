//! 0.8.23 Slice 80.6 (D-80.6-6, AC80-6, R80-13) — the GPU allocation witness
//! is carried on `OpenReport` so the *installed artifact's own process* holds
//! the evidence, not a sibling Rust process.
//!
//! Everything here runs on a host with no GPU and on a build with no
//! `embed-cuda`, because the load-bearing claim is the **absence** rule: the
//! field is `None` unless a real CUDA-selected embedder construction produced
//! a witness. A zero-valued struct must never be reachable — that would make a
//! zero allocation delta representable as success, which R80-12 forbids.

use fathomdb_embedder::{
    CudaDeviceInfo, DeviceResolution, DeviceResolutionReason, EffectiveEmbedDevice,
    EmbedDevicePolicy, NoopEmbedder,
};
use fathomdb_engine::{EmbedderChoice, Engine};
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn a_plain_open_reports_no_gpu_allocation_witness() {
    let directory = tempdir().expect("temporary database directory");
    let opened = Engine::open(directory.path().join("plain.sqlite")).expect("open");

    assert_eq!(
        opened.report.embedder_gpu_allocation_witness, None,
        "an open that constructed no CUDA embedder has nothing to witness"
    );
}

#[test]
fn a_caller_supplied_cuda_resolution_still_reports_no_witness() {
    // The strongest form of the absence rule: a report that *does* carry a
    // CUDA `DeviceResolution` must still carry no witness. Device selection is
    // a policy outcome; the witness is a measurement, and one may never be
    // synthesized from the other.
    let resolution = DeviceResolution {
        requested_policy: EmbedDevicePolicy::Cuda(0),
        cuda_compiled: true,
        effective_device: EffectiveEmbedDevice::Cuda(CudaDeviceInfo {
            ordinal: 0,
            uuid: Some("GPU-11111111-2222-3333-4444-555555555555".to_string()),
            name: Some("Orin".to_string()),
            driver_version: Some("540.4.0".to_string()),
            compute_capability: Some("8.7".to_string()),
            cuda_toolkit_version: Some("12.6".to_string()),
        }),
        visible_cuda_devices: Vec::new(),
        selected_cuda_uuid: Some("GPU-11111111-2222-3333-4444-555555555555".to_string()),
        reason: None,
    };

    let directory = tempdir().expect("temporary database directory");
    let opened = Engine::open_with_choice(
        directory.path().join("caller-cuda.sqlite"),
        EmbedderChoice::CallerWithDeviceResolution {
            embedder: Arc::new(NoopEmbedder::default()),
            device_resolution: resolution.clone(),
        },
    )
    .expect("open with a caller-supplied embedder");

    assert_eq!(opened.report.embedder_device_resolution, Some(resolution));
    assert_eq!(
        opened.report.embedder_gpu_allocation_witness, None,
        "a device resolution is a policy outcome, never a measured witness"
    );
}

#[test]
fn a_cpu_resolution_reports_no_witness() {
    let resolution = DeviceResolution {
        requested_policy: EmbedDevicePolicy::Auto,
        cuda_compiled: false,
        effective_device: EffectiveEmbedDevice::Cpu,
        visible_cuda_devices: Vec::new(),
        selected_cuda_uuid: None,
        reason: Some(DeviceResolutionReason::CudaNotCompiled),
    };

    let directory = tempdir().expect("temporary database directory");
    let opened = Engine::open_with_choice(
        directory.path().join("caller-cpu.sqlite"),
        EmbedderChoice::CallerWithDeviceResolution {
            embedder: Arc::new(NoopEmbedder::default()),
            device_resolution: resolution,
        },
    )
    .expect("open with a caller-supplied embedder");

    assert_eq!(opened.report.embedder_gpu_allocation_witness, None);
}

/// The other half of the absence rule: once a witness is **requested**, `None`
/// stops being an acceptable outcome.
///
/// Requesting the witness while the device policy resolves to CPU (forced here,
/// so the arm is the same whether or not this artifact has CUDA compiled in)
/// must fail the open with a named reason. If it degraded to `None` instead,
/// an operator who asked for evidence would get a report indistinguishable
/// from one where they never asked — which is exactly how a missing
/// measurement gets read as a measurement of zero (R80-12).
///
/// `#[cfg]`-gated per function rather than by `required-features` so this file
/// still builds and runs on the default feature set.
#[cfg(feature = "default-embedder")]
#[test]
fn an_opted_in_witness_that_cannot_be_produced_fails_the_open() {
    use std::env;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());
    let _guard = ENV_LOCK.lock().expect("environment lock");

    let previous_device = env::var_os("FATHOMDB_EMBED_DEVICE");
    let previous_witness = env::var_os(fathomdb_engine::ENV_GPU_ALLOCATION_WITNESS);
    // SAFETY: this test serializes mutations of these process-global variables
    // and restores both before returning.
    unsafe {
        env::set_var("FATHOMDB_EMBED_DEVICE", "cpu");
        env::set_var(fathomdb_engine::ENV_GPU_ALLOCATION_WITNESS, "1");
    }

    let directory = tempdir().expect("temporary database directory");
    let result = Engine::open_with_choice(
        directory.path().join("requested-witness.sqlite"),
        EmbedderChoice::Default,
    );

    // SAFETY: paired with the serialized mutation above.
    unsafe {
        match previous_device {
            Some(value) => env::set_var("FATHOMDB_EMBED_DEVICE", value),
            None => env::remove_var("FATHOMDB_EMBED_DEVICE"),
        }
        match previous_witness {
            Some(value) => env::set_var(fathomdb_engine::ENV_GPU_ALLOCATION_WITNESS, value),
            None => env::remove_var(fathomdb_engine::ENV_GPU_ALLOCATION_WITNESS),
        }
    }

    let error = result.err().expect("a requested witness that cannot be produced fails the open");
    let rendered = error.to_string();
    assert!(
        rendered.contains("no GPU allocation witness could be produced"),
        "the refusal must name itself: {rendered}"
    );
    assert!(
        rendered.contains("cpu_fallback") || rendered.contains("cuda_not_compiled"),
        "the refusal must carry the witness's own failure tag: {rendered}"
    );
}
