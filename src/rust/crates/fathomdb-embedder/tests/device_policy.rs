//! Slice 70 public embed-device policy contract.
//!
//! These tests deliberately exercise the policy resolver through an injected
//! CUDA provider.  They must remain hardware-independent: `cpu` must not
//! touch the provider, while `auto` and forced CUDA expose their probe result
//! rather than silently changing the requested policy.

use std::str::FromStr;

use fathomdb_embedder::{
    resolve_embed_device_policy, CudaDeviceInfo, CudaPoolFailure, CudaProbeError, CudaProvider,
    CudaVisibleDevice, DeviceResolutionError, DeviceResolutionReason, EffectiveEmbedDevice,
    EmbedDevicePolicy, EmbedDevicePolicyError,
};

#[derive(Debug)]
struct RecordingProvider {
    calls: Vec<usize>,
    enumerate_calls: usize,
    response: Result<CudaDeviceInfo, CudaProbeError>,
}

impl RecordingProvider {
    fn compatible(ordinal: usize) -> Self {
        Self {
            calls: Vec::new(),
            enumerate_calls: 0,
            response: Ok(CudaDeviceInfo::new(
                ordinal,
                Some(format!("GPU-{ordinal}")),
                Some("RTX 3090".to_owned()),
                Some("555.42".to_owned()),
                Some("8.6".to_owned()),
                Some("12.6".to_owned()),
            )),
        }
    }

    fn unavailable(error: CudaProbeError) -> Self {
        Self { calls: Vec::new(), enumerate_calls: 0, response: Err(error) }
    }
}

impl CudaProvider for RecordingProvider {
    fn enumerate_visible_cuda_devices(&mut self) -> Result<Vec<CudaVisibleDevice>, CudaProbeError> {
        self.enumerate_calls += 1;
        match &self.response {
            Ok(info) => Ok(vec![CudaVisibleDevice {
                visible_ordinal: info.ordinal,
                uuid: info.uuid.clone().expect("fixture UUID"),
                name: info.name.clone().unwrap_or_else(|| "fixture GPU".to_owned()),
                compute_capability: info.compute_capability.clone(),
            }]),
            Err(CudaProbeError::NoVisibleDevice) => Ok(Vec::new()),
            Err(
                CudaProbeError::Incompatible { .. }
                | CudaProbeError::ProbeFailed { .. }
                | CudaProbeError::Pool(_),
            ) => Ok(vec![CudaVisibleDevice {
                visible_ordinal: 0,
                uuid: "GPU-0".to_owned(),
                name: "fixture GPU".to_owned(),
                compute_capability: Some("8.6".to_owned()),
            }]),
        }
    }

    fn probe_cuda(&mut self, ordinal: usize) -> Result<CudaDeviceInfo, CudaProbeError> {
        self.calls.push(ordinal);
        self.response.clone()
    }
}

#[test]
fn parser_accepts_only_the_supported_policy_grammar() {
    assert_eq!(EmbedDevicePolicy::from_str("auto"), Ok(EmbedDevicePolicy::Auto));
    assert_eq!(EmbedDevicePolicy::from_str("cpu"), Ok(EmbedDevicePolicy::Cpu));
    assert_eq!(EmbedDevicePolicy::from_str("cuda:0"), Ok(EmbedDevicePolicy::Cuda(0)));
    assert_eq!(EmbedDevicePolicy::from_str("cuda:42"), Ok(EmbedDevicePolicy::Cuda(42)));

    for invalid in ["", "cuda", "CUDA:0", " cuda:0", "cuda:-1", "cuda:x", "metal"] {
        assert!(EmbedDevicePolicy::from_str(invalid).is_err(), "{invalid:?} must be rejected");
    }
}

#[test]
fn cpu_does_not_initialize_or_probe_cuda() {
    let mut provider = RecordingProvider::compatible(0);

    let report = resolve_embed_device_policy(EmbedDevicePolicy::Cpu, true, &mut provider).unwrap();

    assert_eq!(report.effective_device, EffectiveEmbedDevice::Cpu);
    assert_eq!(report.reason, None);
    assert!(provider.calls.is_empty());
}

#[test]
fn auto_on_cpu_only_artifact_reports_cuda_not_compiled_without_provider_call() {
    let mut provider = RecordingProvider::compatible(0);

    let report =
        resolve_embed_device_policy(EmbedDevicePolicy::Auto, false, &mut provider).unwrap();

    assert_eq!(report.effective_device, EffectiveEmbedDevice::Cpu);
    assert_eq!(report.reason, Some(DeviceResolutionReason::CudaNotCompiled));
    assert!(provider.calls.is_empty());
}

#[test]
fn auto_without_visible_gpu_falls_back_to_cpu_with_a_report() {
    let mut provider = RecordingProvider::unavailable(CudaProbeError::NoVisibleDevice);

    let report = resolve_embed_device_policy(EmbedDevicePolicy::Auto, true, &mut provider).unwrap();

    assert_eq!(report.effective_device, EffectiveEmbedDevice::Cpu);
    assert_eq!(report.reason, Some(DeviceResolutionReason::NoVisibleCudaDevice));
    assert_eq!(provider.enumerate_calls, 1);
    assert!(provider.calls.is_empty());
}

#[test]
fn auto_with_incompatible_gpu_falls_back_to_cpu_with_a_report() {
    let mut provider = RecordingProvider::unavailable(CudaProbeError::Incompatible {
        message: "driver too old".to_owned(),
    });

    let report = resolve_embed_device_policy(EmbedDevicePolicy::Auto, true, &mut provider).unwrap();

    assert_eq!(report.effective_device, EffectiveEmbedDevice::Cpu);
    assert_eq!(report.reason, Some(DeviceResolutionReason::CudaIncompatible));
    assert_eq!(provider.calls, vec![0]);
}

#[test]
fn auto_with_compatible_gpu_selects_cuda_and_preserves_safe_metadata() {
    let mut provider = RecordingProvider::compatible(0);

    let report = resolve_embed_device_policy(EmbedDevicePolicy::Auto, true, &mut provider).unwrap();

    assert_eq!(
        report.effective_device,
        EffectiveEmbedDevice::Cuda(CudaDeviceInfo::new(
            0,
            Some("GPU-0".to_owned()),
            Some("RTX 3090".to_owned()),
            Some("555.42".to_owned()),
            Some("8.6".to_owned()),
            Some("12.6".to_owned()),
        ))
    );
    assert_eq!(report.reason, None);
    assert_eq!(provider.calls, vec![0]);
}

#[test]
fn forced_cuda_never_falls_back_to_cpu() {
    let mut provider = RecordingProvider::unavailable(CudaProbeError::NoVisibleDevice);

    let error = resolve_embed_device_policy(EmbedDevicePolicy::Cuda(3), true, &mut provider)
        .expect_err("forced CUDA must fail closed");

    assert_eq!(
        error,
        DeviceResolutionError::ForcedCudaUnavailable {
            ordinal: 3,
            reason: DeviceResolutionReason::NoVisibleCudaDevice,
        }
    );
    assert_eq!(provider.enumerate_calls, 1);
    assert!(provider.calls.is_empty());
}

fn pool_failures() -> [CudaPoolFailure; 3] {
    [
        CudaPoolFailure::Exhausted {
            ordinal: 0,
            max_size_bytes: 3 << 30,
            message: "device probe: CUDA_ERROR_OUT_OF_MEMORY".to_owned(),
        },
        CudaPoolFailure::ContextLost {
            recorded_context_id: 7,
            current_context_id: None,
            driver_error: "CUDA_ERROR_CONTEXT_IS_DESTROYED".to_owned(),
            operation: "device probe".to_owned(),
        },
        CudaPoolFailure::PrivateBuildRefused { ordinal: 0, message: "refused".to_owned() },
    ]
}

/// A private-pool process never moves to CPU: a pool failure inside the
/// probe is the typed kind under `auto` as under forced CUDA.
#[test]
fn a_pool_failure_in_the_probe_is_typed_under_every_cuda_policy() {
    for failure in pool_failures() {
        for policy in [EmbedDevicePolicy::Auto, EmbedDevicePolicy::Cuda(0)] {
            let mut provider =
                RecordingProvider::unavailable(CudaProbeError::Pool(failure.clone()));
            assert_eq!(
                resolve_embed_device_policy(policy, true, &mut provider),
                Err(DeviceResolutionError::CudaPool(failure.clone())),
                "{policy:?}"
            );
        }
    }
}

#[test]
fn a_probe_pool_failure_keeps_its_kind_and_ordinal() {
    let kinds = ["cuda_pool_exhausted", "cuda_context_lost", "cuda_private_build_refused"];
    let ordinals = [Some(0), None, Some(0)];
    for ((failure, kind), ordinal) in pool_failures().into_iter().zip(kinds).zip(ordinals) {
        assert_eq!(failure.kind(), kind);
        let error = EmbedDevicePolicyError::Resolution(DeviceResolutionError::CudaPool(failure));
        assert_eq!(error.kind(), kind);
        assert_eq!(error.ordinal(), ordinal);
    }
}

/// Outside a private-pool process a probe failure keeps 0.8.27's contract:
/// `auto` uses CPU with a reason, forced CUDA refuses.
#[test]
fn a_non_private_probe_failure_keeps_the_auto_cpu_move() {
    let failed = || CudaProbeError::ProbeFailed { message: "probe".to_owned() };
    let mut provider = RecordingProvider::unavailable(failed());
    let auto = resolve_embed_device_policy(EmbedDevicePolicy::Auto, true, &mut provider)
        .expect("auto falls back");
    assert_eq!(auto.effective_device, EffectiveEmbedDevice::Cpu);
    assert_eq!(auto.reason, Some(DeviceResolutionReason::CudaProbeFailed));
    let mut provider = RecordingProvider::unavailable(failed());
    assert_eq!(
        resolve_embed_device_policy(EmbedDevicePolicy::Cuda(0), true, &mut provider),
        Err(DeviceResolutionError::ForcedCudaUnavailable {
            ordinal: 0,
            reason: DeviceResolutionReason::CudaProbeFailed,
        })
    );
}
