//! 0.8.23 Slice 80.6 (D-80.6-6, AC80-6, R80-13) — the GPU allocation witness
//! is carried on `OpenReport` so the *installed artifact's own process* holds
//! the evidence, not a sibling Rust process.
//!
//! Everything here runs on a host with no GPU and on a build with no
//! `embed-cuda`, because the load-bearing claim is the **absence** rule: the
//! field is `None` unless a real CUDA-selected embedder construction produced
//! a witness. A zero-valued struct must never be reachable — that would make a
//! zero allocation delta representable as success, which R80-12 forbids.

#[cfg(feature = "test-hooks")]
use fathomdb_embedder::{
    CudaDeviceInfo, DeviceResolution, DeviceResolutionReason, EffectiveEmbedDevice,
    EmbedDevicePolicy, NoopEmbedder,
};
use fathomdb_engine::Engine;
#[cfg(feature = "test-hooks")]
use fathomdb_engine::TestEmbedderChoice;
#[cfg(feature = "test-hooks")]
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
#[cfg(feature = "test-hooks")]
fn a_caller_supplied_cuda_resolution_still_reports_no_witness() {
    // The strongest form of the absence rule: a report that *does* carry a
    // CUDA `DeviceResolution` must still carry no witness. Device selection is
    // a policy outcome; the witness is a measurement, and one may never be
    // synthesized from the other.
    let resolution = DeviceResolution {
        requested_policy: EmbedDevicePolicy::Cuda(0),
        cuda_compiled: true,
        effective_device: EffectiveEmbedDevice::Cuda(CudaDeviceInfo::new(
            0,
            Some("GPU-11111111-2222-3333-4444-555555555555".to_string()),
            Some("Orin".to_string()),
            Some("540.4.0".to_string()),
            Some("8.7".to_string()),
            Some("12.6".to_string()),
        )),
        visible_cuda_devices: Vec::new(),
        selected_cuda_uuid: Some("GPU-11111111-2222-3333-4444-555555555555".to_string()),
        reason: None,
    };

    let directory = tempdir().expect("temporary database directory");
    let opened = Engine::open_with_test_choice_for_test(
        directory.path().join("caller-cuda.sqlite"),
        TestEmbedderChoice::CallerWithDeviceResolution {
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
#[cfg(feature = "test-hooks")]
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
    let opened = Engine::open_with_test_choice_for_test(
        directory.path().join("caller-cpu.sqlite"),
        TestEmbedderChoice::CallerWithDeviceResolution {
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
        fathomdb_engine::EmbedderChoice::Default,
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

    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("a requested witness that cannot be produced fails the open"),
    };
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

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(24))]

    #[test]
    fn current_witness_tagged_measurements_preserve_canonical_observations(
        private in proptest::bool::ANY,
        before in 0_u64..1_048_576,
        growth in 67_108_864_u64..200_000_000,
        context_id in proptest::prop_oneof![proptest::strategy::Just(0_u64), proptest::strategy::Just(u64::MAX), proptest::num::u64::ANY],
        pool_suffix in "[a-zA-Z0-9\"\\\\]{1,12}",
    ) {
        use fathomdb_embedder::{
            ControlAllocationObservation, GpuAllocationMeasurement, GpuAllocationWitness,
            PRIVATE_POOL_EXCLUSIVITY_PRECONDITION,
        };
        let pool_identity = format!("pool:{pool_suffix}");
        let context_text = context_id.to_string();
        let measurement = if private {
            GpuAllocationMeasurement::PrivatePoolUsed {
                context_id: context_text.clone(), pool_identity: pool_identity.clone(),
                used_before_bytes: before, used_after_bytes: before + growth,
                delta_bytes: i128::from(growth), control_request_bytes: 67_108_864,
                control_before_bytes: before + growth,
                control_live_bytes: before + growth + 67_108_864,
                control_released_bytes: before + growth,
                pool_exclusivity_precondition: PRIVATE_POOL_EXCLUSIVITY_PRECONDITION.into(),
            }
        } else {
            GpuAllocationMeasurement::from_global_control(ControlAllocationObservation {
                requested_bytes: 268_435_456, block_count: 1,
                free_before_bytes: 42_000_000_000, free_after_bytes: 41_700_000_000,
            })
        };
        let free_before = 40_000_000_000;
        let free_after = if private { free_before + 1234 } else { free_before - growth };
        let witness = GpuAllocationWitness {
            process_id: 4242, device_integrated: private,
            device_ordinal_requested: 0, device_ordinal_actual: 0,
            device_uuid: "GPU-11111111-2222-3333-4444-555555555555".into(),
            device_name: "fixture GPU".into(), compute_capability: "8.7".into(),
            free_before_bytes: free_before, free_after_bytes: free_after,
            total_bytes: 48_000_000_000,
            delta_bytes: i128::from(free_before) - i128::from(free_after),
            delta_floor_bytes: 67_108_864, embedded_vector_dim: 384, measurement,
        };
        let raw = witness.to_canonical_json();
        let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
        proptest::prop_assert_eq!(&raw, &format!("{}\n", serde_json::to_string(&value).unwrap()));
        proptest::prop_assert_eq!(value["schema"].as_str(), Some("fathomdb.tegra-gpu-allocation-witness/v2"));
        proptest::prop_assert_eq!(value["process_id"].as_u64(), Some(4242));
        proptest::prop_assert_eq!(value["device_integrated"].as_bool(), Some(private));
        proptest::prop_assert_eq!(value["delta_bytes"].as_i64(), Some(if private { -1234 } else { growth as i64 }));
        let proof = &value["measurement"];
        let mut keys: Vec<&str> = proof.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        if private {
            proptest::prop_assert_eq!(keys, vec!["context_id", "control_before_bytes", "control_live_bytes", "control_released_bytes", "control_request_bytes", "delta_bytes", "kind", "pool_exclusivity_precondition", "pool_identity", "used_after_bytes", "used_before_bytes"]);
            proptest::prop_assert_eq!(proof["kind"].as_str(), Some("private_pool_used"));
            proptest::prop_assert_eq!(proof["context_id"].as_str(), Some(context_text.as_str()));
            proptest::prop_assert_eq!(proof["pool_identity"].as_str(), Some(pool_identity.as_str()));
            proptest::prop_assert_eq!(proof["used_before_bytes"].as_u64(), Some(before));
            proptest::prop_assert_eq!(proof["used_after_bytes"].as_u64(), Some(before + growth));
            proptest::prop_assert_eq!(proof["delta_bytes"].as_u64(), Some(growth));
            proptest::prop_assert_eq!(proof["control_before_bytes"].as_u64(), Some(before + growth));
            proptest::prop_assert_eq!(proof["control_request_bytes"].as_u64(), Some(67_108_864));
            proptest::prop_assert_eq!(proof["pool_exclusivity_precondition"].as_str(), Some("no competing allocations or frees may occur in the witnessed private pool during model and control observation"));
            proptest::prop_assert_eq!(proof["control_live_bytes"].as_u64(), Some(before + growth + 67_108_864));
            proptest::prop_assert_eq!(proof["control_released_bytes"].as_u64(), Some(before + growth));
        } else {
            proptest::prop_assert_eq!(keys, vec!["control_allocation_request_bytes", "control_block_count", "control_delta_bytes", "control_free_after_bytes", "control_free_before_bytes", "kind", "sole_gpu_consumer_precondition"]);
            proptest::prop_assert_eq!(proof["kind"].as_str(), Some("global_free"));
            proptest::prop_assert_eq!(proof["control_allocation_request_bytes"].as_u64(), Some(268_435_456));
            proptest::prop_assert_eq!(proof["control_delta_bytes"].as_u64(), Some(300_000_000));
            proptest::prop_assert_eq!(proof["control_block_count"].as_u64(), Some(1));
            proptest::prop_assert_eq!(proof["control_free_before_bytes"].as_u64(), Some(42_000_000_000));
            proptest::prop_assert_eq!(proof["control_free_after_bytes"].as_u64(), Some(41_700_000_000));
            proptest::prop_assert_eq!(proof["sole_gpu_consumer_precondition"].as_str(), Some("the witness process must be the sole GPU consumer of the selected discrete device"));
        }
    }
}
