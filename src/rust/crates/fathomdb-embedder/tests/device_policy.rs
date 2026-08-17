//! Slice 70 public embed-device policy contract.
//!
//! These tests deliberately exercise the policy resolver through an injected
//! CUDA provider.  They must remain hardware-independent: `cpu` must not
//! touch the provider, while `auto` and forced CUDA expose their probe result
//! rather than silently changing the requested policy.

use std::str::FromStr;

use fathomdb_embedder::{
    resolve_embed_device_policy, CudaDeviceInfo, CudaProbeError, CudaProvider,
    DeviceResolutionError, DeviceResolutionReason, EffectiveEmbedDevice, EmbedDevicePolicy,
};

#[derive(Debug)]
struct RecordingProvider {
    calls: Vec<usize>,
    response: Result<CudaDeviceInfo, CudaProbeError>,
}

impl RecordingProvider {
    fn compatible(ordinal: usize) -> Self {
        Self {
            calls: Vec::new(),
            response: Ok(CudaDeviceInfo {
                ordinal,
                name: Some("RTX 3090".to_owned()),
                driver_version: Some("555.42".to_owned()),
                compute_capability: Some("8.6".to_owned()),
                cuda_toolkit_version: Some("12.6".to_owned()),
            }),
        }
    }

    fn unavailable(error: CudaProbeError) -> Self {
        Self { calls: Vec::new(), response: Err(error) }
    }
}

impl CudaProvider for RecordingProvider {
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
    assert_eq!(provider.calls, vec![0]);
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
        EffectiveEmbedDevice::Cuda(CudaDeviceInfo {
            ordinal: 0,
            name: Some("RTX 3090".to_owned()),
            driver_version: Some("555.42".to_owned()),
            compute_capability: Some("8.6".to_owned()),
            cuda_toolkit_version: Some("12.6".to_owned()),
        })
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
    assert_eq!(provider.calls, vec![3]);
}
