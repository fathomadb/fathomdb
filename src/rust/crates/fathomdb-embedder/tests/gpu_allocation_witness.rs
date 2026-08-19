//! 0.8.23 Slice 80.5 — host-independent contract for the Tegra-portable GPU
//! allocation witness (`dev/design/0.8.23-aarch64-tegra.md` § 7 80.5,
//! D-80.5-1 … D-80.5-6).
//!
//! Every arm here runs on a host with **no GPU and no `embed-cuda` feature**:
//! the verdict logic, the typed failures (R80-12), the floor comparison
//! (D-80.5-2), the control-allocation attribution check (D-80.5-3), the
//! UUID normalization (§ 2.7) and the canonical serialization are pure
//! (AC80-18, AC80-20). The real-hardware arm lives in
//! `slice80_gpu_allocation_witness.rs`.

use fathomdb_embedder::{
    evaluate_allocation_witness, normalize_cuda_uuid, observe_control_allocation,
    AllocationWitnessInputs, ControlAllocationObservation, CudaDeviceInfo, DeviceResolutionReason,
    EffectiveEmbedDevice, GpuControlAllocator, GpuMemorySample, GpuMemorySampler, GpuWitnessError,
    GpuWitnessSkip, WitnessStage, CUDA_ERROR_INVALID_CONTEXT, DEFAULT_CONTROL_ALLOCATION_BYTES,
    DEFAULT_DELTA_FLOOR_BYTES, MAX_CONTROL_BLOCKS, SOLE_GPU_CONSUMER_PRECONDITION,
    TEGRA_GPU_ALLOCATION_WITNESS_SCHEMA,
};

/// § 2.7 measured this host: 61 GiB of unified system memory.
const TOTAL_BYTES: u64 = 65_879_896_064;
/// § 2.7 measured this host's driver-API UUID rendering.
const DEVICE_UUID: &str = "GPU-bbbe9f37-7028-556a-930b-54e5f3b67a82";
/// § 2.7: `nvidia-smi --query-gpu=uuid` on Tegra omits the `GPU-` prefix.
const NVIDIA_SMI_UUID: &str = "bbbe9f37-7028-556a-930b-54e5f3b67a82";

fn cuda_device_info() -> CudaDeviceInfo {
    CudaDeviceInfo {
        ordinal: 0,
        uuid: Some(DEVICE_UUID.to_owned()),
        name: Some("Orin".to_owned()),
        driver_version: None,
        compute_capability: Some("8.7".to_owned()),
        cuda_toolkit_version: None,
    }
}

/// The numbers of an actual witnessed run on this Orin (2026-08-18): the
/// kernel page pool absorbed eight 1 GiB control blocks and charged for the
/// ninth, and the model load that followed charged 143_622_144 bytes.
fn valid_inputs() -> AllocationWitnessInputs {
    AllocationWitnessInputs {
        requested_ordinal: 0,
        effective_device: EffectiveEmbedDevice::Cuda(cuda_device_info()),
        cpu_reason: None,
        retained_ordinal: 0,
        retained_device_uuid: DEVICE_UUID.to_owned(),
        before: Some(GpuMemorySample { free_bytes: 50_213_253_120, total_bytes: TOTAL_BYTES }),
        after: Some(GpuMemorySample { free_bytes: 50_069_630_976, total_bytes: TOTAL_BYTES }),
        control: Some(ControlAllocationObservation {
            requested_bytes: DEFAULT_CONTROL_ALLOCATION_BYTES,
            block_count: 9,
            free_before_bytes: 51_290_562_560,
            free_after_bytes: 50_213_253_120,
        }),
        delta_floor_bytes: DEFAULT_DELTA_FLOOR_BYTES,
        embedded_vector_dim: Some(384),
    }
}

#[test]
fn declared_floor_sits_above_measured_idle_jitter_and_below_the_model() {
    // D-80.5-2: the floor is an order of magnitude above the measured 0-byte
    // idle jitter and roughly half the measured 133_466_304-byte weight file.
    assert_eq!(DEFAULT_DELTA_FLOOR_BYTES, 67_108_864);
    const { assert!(DEFAULT_DELTA_FLOOR_BYTES < 133_466_304) };
    // The three witnessed loads on this Orin charged 143_364_096,
    // 143_622_144 and 143_880_192 bytes; the floor clears each by >2x.
    const { assert!(DEFAULT_DELTA_FLOOR_BYTES * 2 < 143_364_096) };
    // 1 GiB control blocks, 16 of them at most (D-80.5-3, as measured).
    assert_eq!(DEFAULT_CONTROL_ALLOCATION_BYTES, 1_073_741_824);
    assert_eq!(MAX_CONTROL_BLOCKS, 16);
}

#[test]
fn schema_is_the_new_tegra_record_not_the_shared_observation() {
    // D-80.5-5: a separate record, never a bump of cuda-device-observation/v1.
    assert_eq!(TEGRA_GPU_ALLOCATION_WITNESS_SCHEMA, "fathomdb.tegra-gpu-allocation-witness/v1");
}

#[test]
fn valid_samples_yield_a_re_derivable_witness() {
    let witness = evaluate_allocation_witness(valid_inputs()).expect("valid witness");
    assert_eq!(witness.device_ordinal_requested, 0);
    assert_eq!(witness.device_ordinal_actual, 0);
    assert_eq!(witness.device_uuid, DEVICE_UUID);
    assert_eq!(witness.device_name, "Orin");
    assert_eq!(witness.compute_capability, "8.7");
    assert_eq!(witness.total_bytes, TOTAL_BYTES);
    assert_eq!(witness.free_before_bytes, 50_213_253_120);
    assert_eq!(witness.free_after_bytes, 50_069_630_976);
    // R80-13: the verdict is re-derivable from the retained raw samples.
    assert_eq!(
        witness.delta_bytes,
        i128::from(witness.free_before_bytes) - i128::from(witness.free_after_bytes)
    );
    assert_eq!(witness.delta_floor_bytes, DEFAULT_DELTA_FLOOR_BYTES);
    assert!(witness.delta_bytes >= i128::from(witness.delta_floor_bytes));
    assert_eq!(witness.control_delta_bytes, 1_077_309_440);
    assert_eq!(witness.control_block_count, 9);
    assert_eq!(witness.embedded_vector_dim, 384);
}

#[test]
fn cpu_resolution_is_a_named_failure_not_a_pass() {
    // AC80-16 / R80-12.
    let inputs = AllocationWitnessInputs {
        effective_device: EffectiveEmbedDevice::Cpu,
        cpu_reason: Some(DeviceResolutionReason::NoVisibleCudaDevice),
        ..valid_inputs()
    };
    match evaluate_allocation_witness(inputs) {
        Err(GpuWitnessError::CpuFallback { reason }) => {
            assert_eq!(reason, "no_visible_cuda_device");
        }
        other => panic!("expected a named CPU-fallback failure, got {other:?}"),
    }
}

#[test]
fn cuda_not_compiled_cpu_resolution_names_that_reason() {
    let inputs = AllocationWitnessInputs {
        effective_device: EffectiveEmbedDevice::Cpu,
        cpu_reason: Some(DeviceResolutionReason::CudaNotCompiled),
        ..valid_inputs()
    };
    assert!(matches!(
        evaluate_allocation_witness(inputs),
        Err(GpuWitnessError::CpuFallback { reason }) if reason == "cuda_not_compiled"
    ));
}

#[test]
fn retained_ordinal_must_match_the_requested_one() {
    // D-80.5-1 step 2.
    let inputs = AllocationWitnessInputs { retained_ordinal: 1, ..valid_inputs() };
    match evaluate_allocation_witness(inputs) {
        Err(GpuWitnessError::OrdinalMismatch { requested, retained }) => {
            assert_eq!((requested, retained), (0, 1));
        }
        other => panic!("expected an ordinal mismatch, got {other:?}"),
    }
}

#[test]
fn probed_and_retained_uuids_must_correlate() {
    let inputs = AllocationWitnessInputs {
        retained_device_uuid: "GPU-00000000-0000-0000-0000-000000000000".to_owned(),
        ..valid_inputs()
    };
    assert!(matches!(
        evaluate_allocation_witness(inputs),
        Err(GpuWitnessError::UuidMismatch { .. })
    ));
}

#[test]
fn uuid_correlation_normalizes_the_tegra_prefix_difference() {
    // § 2.7: Tegra's nvidia-smi drops the `GPU-` prefix; the driver bytes are
    // identical, so the comparison must normalize rather than string-equal.
    assert_eq!(normalize_cuda_uuid(DEVICE_UUID), normalize_cuda_uuid(NVIDIA_SMI_UUID));
    assert_eq!(normalize_cuda_uuid("GPU-BBBE9F37-7028-556A-930B-54E5F3B67A82"), NVIDIA_SMI_UUID);
    let inputs = AllocationWitnessInputs {
        retained_device_uuid: NVIDIA_SMI_UUID.to_owned(),
        ..valid_inputs()
    };
    let witness = evaluate_allocation_witness(inputs).expect("prefix difference is not a mismatch");
    assert_eq!(witness.device_uuid, DEVICE_UUID);
}

#[test]
fn a_missing_sample_is_never_treated_as_zero() {
    // § 2.7's ordering invariant: a witness that samples too early gets a
    // typed error; treating that as a zero sample manufactures a false delta.
    for (inputs, stage) in [
        (AllocationWitnessInputs { before: None, ..valid_inputs() }, WitnessStage::LoadBefore),
        (AllocationWitnessInputs { after: None, ..valid_inputs() }, WitnessStage::LoadAfter),
        (
            AllocationWitnessInputs { control: None, ..valid_inputs() },
            WitnessStage::ControlAllocation,
        ),
        (
            AllocationWitnessInputs { embedded_vector_dim: None, ..valid_inputs() },
            WitnessStage::ForwardPass,
        ),
    ] {
        match evaluate_allocation_witness(inputs) {
            Err(GpuWitnessError::MissingSample { stage: observed }) => {
                assert_eq!(observed, stage);
            }
            other => panic!("expected a missing {stage:?} sample, got {other:?}"),
        }
    }
}

#[test]
fn zero_negative_and_below_floor_deltas_all_fail_closed() {
    // D-80.5-2: `>= floor`, never `> 0`.
    let cases: [(u64, &str); 3] = [
        (50_213_253_120, "zero delta"),
        (50_213_253_121, "negative delta"),
        (50_213_253_120 - 1_048_576, "1 MiB, below the 64 MiB floor"),
    ];
    for (free_after, label) in cases {
        let inputs = AllocationWitnessInputs {
            after: Some(GpuMemorySample { free_bytes: free_after, total_bytes: TOTAL_BYTES }),
            ..valid_inputs()
        };
        match evaluate_allocation_witness(inputs) {
            Err(GpuWitnessError::InsufficientDelta { delta_bytes, floor_bytes }) => {
                assert!(delta_bytes < i128::from(floor_bytes), "{label}");
                assert_eq!(floor_bytes, DEFAULT_DELTA_FLOOR_BYTES, "{label}");
            }
            other => panic!("expected an insufficient delta for {label}, got {other:?}"),
        }
    }
}

#[test]
fn a_control_allocation_the_counter_never_saw_is_unattributable() {
    // D-80.5-3: the control proves the counter is live and attributable on
    // this host at witness time; an unattributable delta is a named failure.
    let inputs = AllocationWitnessInputs {
        control: Some(ControlAllocationObservation {
            requested_bytes: DEFAULT_CONTROL_ALLOCATION_BYTES,
            block_count: 1,
            free_before_bytes: 51_290_562_560,
            free_after_bytes: 51_289_562_560,
        }),
        ..valid_inputs()
    };
    match evaluate_allocation_witness(inputs) {
        Err(GpuWitnessError::ControlAllocationNotObserved { requested_bytes, delta_bytes }) => {
            assert_eq!(requested_bytes, DEFAULT_CONTROL_ALLOCATION_BYTES);
            assert_eq!(delta_bytes, 1_000_000);
        }
        other => panic!("expected an unattributable control allocation, got {other:?}"),
    }
}

#[test]
fn inconsistent_totals_between_samples_are_a_probe_failure() {
    let inputs = AllocationWitnessInputs {
        after: Some(GpuMemorySample {
            free_bytes: 59_790_000_000,
            total_bytes: TOTAL_BYTES - 4096,
        }),
        ..valid_inputs()
    };
    assert!(matches!(
        evaluate_allocation_witness(inputs),
        Err(GpuWitnessError::ProbeFailed { .. })
    ));
}

#[test]
fn a_forward_pass_of_the_wrong_dimension_is_not_a_witness() {
    // D-80.5-1 step 6: allocated-but-never-computed is not a witness.
    let inputs = AllocationWitnessInputs { embedded_vector_dim: Some(768), ..valid_inputs() };
    assert!(matches!(
        evaluate_allocation_witness(inputs),
        Err(GpuWitnessError::ProbeFailed { .. })
    ));
}

#[test]
fn cuda_error_invalid_context_maps_to_the_named_no_context_failure() {
    // AC80-17, against the measured § 2.7 constant.
    assert_eq!(CUDA_ERROR_INVALID_CONTEXT, 201);
    let error = GpuWitnessError::from_driver_status(CUDA_ERROR_INVALID_CONTEXT, "invalid context");
    assert!(matches!(error, GpuWitnessError::NoCudaContext { .. }), "got {error:?}");
    assert_eq!(error.as_str(), "no_cuda_context");
    let other = GpuWitnessError::from_driver_status(999, "unknown");
    assert!(matches!(other, GpuWitnessError::ProbeFailed { .. }), "got {other:?}");
}

#[test]
fn every_failure_carries_a_stable_machine_readable_tag() {
    let tags: Vec<&str> = vec![
        GpuWitnessError::CpuFallback { reason: "cpu".into() }.as_str(),
        GpuWitnessError::NoCudaContext { message: String::new() }.as_str(),
        GpuWitnessError::OrdinalMismatch { requested: 0, retained: 1 }.as_str(),
        GpuWitnessError::UuidMismatch { probed: String::new(), retained: String::new() }.as_str(),
        GpuWitnessError::MissingSample { stage: WitnessStage::LoadBefore }.as_str(),
        GpuWitnessError::InsufficientDelta { delta_bytes: 0, floor_bytes: 1 }.as_str(),
        GpuWitnessError::ControlAllocationNotObserved { requested_bytes: 1, delta_bytes: 0 }
            .as_str(),
        GpuWitnessError::ProbeFailed { message: String::new() }.as_str(),
    ];
    assert_eq!(
        tags,
        vec![
            "cpu_fallback",
            "no_cuda_context",
            "ordinal_mismatch",
            "uuid_mismatch",
            "missing_sample",
            "insufficient_delta",
            "control_allocation_not_observed",
            "probe_failed",
        ]
    );
}

#[test]
fn skips_are_named_never_silent() {
    // AC80-20: a host with no visible CUDA device SKIPs with a named reason.
    assert_eq!(GpuWitnessSkip::CudaNotCompiled.as_str(), "cuda_not_compiled");
    assert_eq!(GpuWitnessSkip::NoVisibleCudaDevice.as_str(), "no_visible_cuda_device");
    assert_eq!(GpuWitnessSkip::NotOptedIn.as_str(), "not_opted_in");
    for skip in [
        GpuWitnessSkip::CudaNotCompiled,
        GpuWitnessSkip::NoVisibleCudaDevice,
        GpuWitnessSkip::NotOptedIn,
    ] {
        assert!(skip.to_string().contains(skip.as_str()));
    }
}

#[test]
fn witness_serializes_as_canonical_json_with_a_trailing_newline() {
    let witness = evaluate_allocation_witness(valid_inputs()).expect("valid witness");
    let expected = concat!(
        "{",
        r#""compute_capability":"8.7","#,
        r#""control_allocation_request_bytes":1073741824,"#,
        r#""control_block_count":9,"#,
        r#""control_delta_bytes":1077309440,"#,
        r#""control_free_after_bytes":50213253120,"#,
        r#""control_free_before_bytes":51290562560,"#,
        r#""delta_bytes":143622144,"#,
        r#""delta_floor_bytes":67108864,"#,
        r#""device_name":"Orin","#,
        r#""device_ordinal_actual":0,"#,
        r#""device_ordinal_requested":0,"#,
        r#""device_uuid":"GPU-bbbe9f37-7028-556a-930b-54e5f3b67a82","#,
        r#""embedded_vector_dim":384,"#,
        r#""free_after_bytes":50069630976,"#,
        r#""free_before_bytes":50213253120,"#,
        r#""schema":"fathomdb.tegra-gpu-allocation-witness/v1","#,
        r#""sole_gpu_consumer_precondition":"#,
        r#""the witness run must be the sole GPU consumer: cuMemGetInfo reports a shared, system-wide counter on an integrated GPU","#,
        r#""total_bytes":65879896064"#,
        "}\n",
    );
    assert_eq!(witness.to_canonical_json(), expected);
    assert_eq!(
        SOLE_GPU_CONSUMER_PRECONDITION,
        "the witness run must be the sole GPU consumer: cuMemGetInfo reports a shared, system-wide counter on an integrated GPU"
    );
}

/// Scripted sampler/allocator standing in for the CUDA driver boundary, in
/// the shape `FixtureCudaProvider` uses for `CudaProvider`.
struct ScriptedSampler {
    samples: Vec<Result<GpuMemorySample, GpuWitnessError>>,
}

impl GpuMemorySampler for ScriptedSampler {
    fn sample(&mut self) -> Result<GpuMemorySample, GpuWitnessError> {
        if self.samples.is_empty() {
            return Err(GpuWitnessError::MissingSample { stage: WitnessStage::ControlAllocation });
        }
        self.samples.remove(0)
    }
}

#[derive(Default)]
struct RecordingAllocator {
    allocated: Vec<u64>,
    released: usize,
    fail: bool,
}

impl GpuControlAllocator for RecordingAllocator {
    fn allocate(&mut self, bytes: u64) -> Result<(), GpuWitnessError> {
        if self.fail {
            return Err(GpuWitnessError::ProbeFailed { message: "out of memory".to_owned() });
        }
        self.allocated.push(bytes);
        Ok(())
    }

    fn release(&mut self) {
        self.released += 1;
    }
}

#[test]
fn the_control_step_stops_at_the_first_block_the_counter_charges_for() {
    // D-80.5-3, as commissioned against this Orin: the CUDA driver holds a
    // pre-reserved arena, so early blocks can be absorbed without the counter
    // moving. The control step allocates and HOLDS 256 MiB blocks until one
    // is charged in full, which both proves the counter is live and leaves no
    // spare arena for the model load to hide inside.
    let mut sampler = ScriptedSampler {
        samples: vec![
            Ok(GpuMemorySample { free_bytes: 52_365_336_576, total_bytes: TOTAL_BYTES }),
            // Block 1: absorbed by the kernel page pool, 1 MiB of movement.
            Ok(GpuMemorySample { free_bytes: 52_364_304_384, total_bytes: TOTAL_BYTES }),
            // Block 2: charged in full.
            Ok(GpuMemorySample { free_bytes: 51_286_994_944, total_bytes: TOTAL_BYTES }),
        ],
    };
    let mut allocator = RecordingAllocator::default();
    let observation = observe_control_allocation(
        &mut sampler,
        &mut allocator,
        DEFAULT_CONTROL_ALLOCATION_BYTES,
        MAX_CONTROL_BLOCKS,
    )
    .expect("control observation");
    assert_eq!(
        allocator.allocated,
        vec![DEFAULT_CONTROL_ALLOCATION_BYTES, DEFAULT_CONTROL_ALLOCATION_BYTES]
    );
    assert_eq!(allocator.released, 0, "the blocks stay resident across the load");
    assert_eq!(observation.requested_bytes, DEFAULT_CONTROL_ALLOCATION_BYTES);
    assert_eq!(observation.block_count, 2);
    assert_eq!(observation.free_before_bytes, 52_364_304_384);
    assert_eq!(observation.delta_bytes(), 1_077_309_440);
}

#[test]
fn a_counter_that_never_charges_exhausts_the_block_budget_and_fails() {
    let mut sampler = ScriptedSampler {
        samples: (0..=MAX_CONTROL_BLOCKS)
            .map(|index| {
                Ok(GpuMemorySample {
                    free_bytes: 52_365_336_576 - (index as u64 * 1_048_576),
                    total_bytes: TOTAL_BYTES,
                })
            })
            .collect(),
    };
    let mut allocator = RecordingAllocator::default();
    let error = observe_control_allocation(
        &mut sampler,
        &mut allocator,
        DEFAULT_CONTROL_ALLOCATION_BYTES,
        MAX_CONTROL_BLOCKS,
    )
    .expect_err("a counter that never charges is unattributable");
    match error {
        GpuWitnessError::ControlAllocationNotObserved { requested_bytes, delta_bytes } => {
            assert_eq!(requested_bytes, DEFAULT_CONTROL_ALLOCATION_BYTES);
            assert_eq!(delta_bytes, 1_048_576);
        }
        other => panic!("expected an unattributable control allocation, got {other:?}"),
    }
    assert_eq!(allocator.allocated.len(), MAX_CONTROL_BLOCKS);
    assert_eq!(allocator.released, 1, "a failed control step releases its blocks");
}

#[test]
fn a_failed_control_allocation_propagates_and_still_releases() {
    let mut sampler = ScriptedSampler {
        samples: vec![Ok(GpuMemorySample { free_bytes: 1, total_bytes: TOTAL_BYTES })],
    };
    let mut allocator = RecordingAllocator { fail: true, ..RecordingAllocator::default() };
    let error = observe_control_allocation(
        &mut sampler,
        &mut allocator,
        DEFAULT_CONTROL_ALLOCATION_BYTES,
        MAX_CONTROL_BLOCKS,
    )
    .expect_err("a failed control allocation is never a pass");
    assert!(matches!(error, GpuWitnessError::ProbeFailed { .. }), "got {error:?}");
    assert_eq!(allocator.released, 1);
}

#[test]
fn a_sampler_error_during_the_control_step_is_never_swallowed() {
    let mut sampler = ScriptedSampler {
        samples: vec![Err(GpuWitnessError::NoCudaContext { message: "no context".to_owned() })],
    };
    let mut allocator = RecordingAllocator::default();
    let error = observe_control_allocation(
        &mut sampler,
        &mut allocator,
        DEFAULT_CONTROL_ALLOCATION_BYTES,
        MAX_CONTROL_BLOCKS,
    )
    .expect_err("sampler failure is a witness failure");
    assert!(matches!(error, GpuWitnessError::NoCudaContext { .. }), "got {error:?}");
    assert!(allocator.allocated.is_empty());
}
