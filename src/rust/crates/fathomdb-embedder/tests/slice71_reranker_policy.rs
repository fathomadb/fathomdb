//! Behavioral contract for Slice 71's strict cross-encoder device resolver.

use fathomdb_embedder::{
    resolve_reranker_device_policy, CudaDeviceInfo, CudaProbeError, CudaProvider,
    CudaVisibleDevice, EffectiveRerankerDevice, RerankerDevicePolicy, RerankerDevicePolicyError,
    RerankerDevicePolicyParseError, RerankerDeviceResolutionError, RerankerDeviceResolutionReason,
};

struct Provider {
    enumerate: Result<Vec<CudaVisibleDevice>, CudaProbeError>,
    probe: Result<CudaDeviceInfo, CudaProbeError>,
    enumerations: usize,
    probes: usize,
}

impl Default for Provider {
    fn default() -> Self {
        Self {
            enumerate: Ok(Vec::new()),
            probe: Err(CudaProbeError::NoVisibleDevice),
            enumerations: 0,
            probes: 0,
        }
    }
}

impl Provider {
    fn selected() -> Self {
        Self {
            enumerate: Ok(vec![CudaVisibleDevice {
                visible_ordinal: 0,
                uuid: "GPU-slice71".to_owned(),
                name: "slice71-test".to_owned(),
                compute_capability: Some("8.6".to_owned()),
            }]),
            probe: Ok(CudaDeviceInfo {
                ordinal: 0,
                uuid: Some("GPU-slice71".to_owned()),
                name: Some("slice71-test".to_owned()),
                driver_version: None,
                compute_capability: Some("8.6".to_owned()),
                cuda_toolkit_version: None,
            }),
            ..Self::default()
        }
    }
}

impl CudaProvider for Provider {
    fn enumerate_visible_cuda_devices(&mut self) -> Result<Vec<CudaVisibleDevice>, CudaProbeError> {
        self.enumerations += 1;
        self.enumerate.clone()
    }

    fn probe_cuda(&mut self, _ordinal: usize) -> Result<CudaDeviceInfo, CudaProbeError> {
        self.probes += 1;
        self.probe.clone()
    }
}

#[test]
fn grammar_is_exact_and_retires_legacy_spellings() {
    for raw in ["", "cuda", "cuda:", "cuda:-1", " CUDA:0", "metal", "cpu "] {
        assert_eq!(
            raw.parse::<RerankerDevicePolicy>(),
            Err(RerankerDevicePolicyParseError::InvalidPolicy { raw: raw.to_owned() })
        );
    }
    assert_eq!("auto".parse(), Ok(RerankerDevicePolicy::Auto));
    assert_eq!("cpu".parse(), Ok(RerankerDevicePolicy::Cpu));
    assert_eq!("cuda:12".parse(), Ok(RerankerDevicePolicy::Cuda(12)));
}

#[test]
fn cpu_never_initializes_cuda_even_in_cuda_capable_artifact() {
    let mut provider = Provider::selected();
    let result = resolve_reranker_device_policy(RerankerDevicePolicy::Cpu, true, &mut provider)
        .expect("cpu policy is valid");
    assert_eq!(result.effective_device, EffectiveRerankerDevice::Cpu);
    assert_eq!(provider.enumerations, 0);
    assert_eq!(provider.probes, 0);
}

#[test]
fn auto_selects_a_compatible_gpu_and_retains_its_identity() {
    let mut provider = Provider::selected();
    let result = resolve_reranker_device_policy(RerankerDevicePolicy::Auto, true, &mut provider)
        .expect("compatible GPU must be selected");
    assert!(matches!(result.effective_device, EffectiveRerankerDevice::Cuda(_)));
    assert_eq!(result.selected_cuda_uuid.as_deref(), Some("GPU-slice71"));
    assert_eq!(result.reason, None);
}

#[test]
fn auto_cpu_fallback_is_typed_for_unavailable_incompatible_and_probe_failure() {
    for (enumerate, probe, reason) in [
        (
            Ok(vec![]),
            Err(CudaProbeError::NoVisibleDevice),
            RerankerDeviceResolutionReason::NoVisibleCudaDevice,
        ),
        (
            Ok(vec![visible()]),
            Err(CudaProbeError::Incompatible { message: "old driver".to_owned() }),
            RerankerDeviceResolutionReason::CudaIncompatible,
        ),
        (
            Err(CudaProbeError::ProbeFailed { message: "driver fault".to_owned() }),
            Err(CudaProbeError::NoVisibleDevice),
            RerankerDeviceResolutionReason::CudaProbeFailed,
        ),
    ] {
        let mut provider = Provider { enumerate, probe, ..Provider::default() };
        let result =
            resolve_reranker_device_policy(RerankerDevicePolicy::Auto, true, &mut provider)
                .expect("auto may use CPU only with a classified reason");
        assert_eq!(result.effective_device, EffectiveRerankerDevice::Cpu);
        assert_eq!(result.reason, Some(reason));
    }
}

#[test]
fn forced_cuda_never_retries_or_falls_back_to_cpu() {
    for (enumerate, probe, reason) in [
        (
            Ok(vec![]),
            Err(CudaProbeError::NoVisibleDevice),
            RerankerDeviceResolutionReason::NoVisibleCudaDevice,
        ),
        (
            Ok(vec![visible()]),
            Err(CudaProbeError::Incompatible { message: "old driver".to_owned() }),
            RerankerDeviceResolutionReason::CudaIncompatible,
        ),
        (
            Ok(vec![visible()]),
            Err(CudaProbeError::ProbeFailed { message: "session failed".to_owned() }),
            RerankerDeviceResolutionReason::CudaProbeFailed,
        ),
    ] {
        let mut provider = Provider { enumerate, probe, ..Provider::default() };
        assert_eq!(
            resolve_reranker_device_policy(RerankerDevicePolicy::Cuda(0), true, &mut provider),
            Err(RerankerDeviceResolutionError::ForcedCudaUnavailable { ordinal: 0, reason })
        );
        assert_eq!(provider.enumerations, 1);
        assert!(provider.probes <= 1, "no CPU retry is permitted");
    }
}

#[test]
fn cpu_only_artifact_is_typed_for_auto_and_forced_cuda() {
    let mut provider = Provider::selected();
    let auto = resolve_reranker_device_policy(RerankerDevicePolicy::Auto, false, &mut provider)
        .expect("auto on a CPU-only artifact stays CPU");
    assert_eq!(auto.reason, Some(RerankerDeviceResolutionReason::CudaNotCompiled));
    assert_eq!(provider.enumerations, 0);
    assert_eq!(
        resolve_reranker_device_policy(RerankerDevicePolicy::Cuda(3), false, &mut provider),
        Err(RerankerDeviceResolutionError::CudaNotCompiled { ordinal: 3 })
    );
}

#[test]
fn public_error_preserves_kind_and_ordinal() {
    let error = RerankerDevicePolicyError::Resolution(
        RerankerDeviceResolutionError::ForcedCudaUnavailable {
            ordinal: 4,
            reason: RerankerDeviceResolutionReason::CudaProbeFailed,
        },
    );
    assert_eq!(error.kind(), "cuda_probe_failed");
    assert_eq!(error.ordinal(), Some(4));
}

fn visible() -> CudaVisibleDevice {
    CudaVisibleDevice {
        visible_ordinal: 0,
        uuid: "GPU-slice71".to_owned(),
        name: "slice71-test".to_owned(),
        compute_capability: Some("8.6".to_owned()),
    }
}
