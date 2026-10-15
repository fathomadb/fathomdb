//! 0.8.28 Slice 31 (T-CUDA-COMPAT C4, owner ruling Q22): a device whose
//! compute capability is below the build's kernel floor is refused before
//! any device is built. Hermetic: injected devices, never a real GPU.

use fathomdb_embedder::{
    diagnose_gpu, parse_compute_capability_floor, resolve_embed_device_policy,
    resolve_reranker_device_policy, CudaDeviceInfo, CudaProbeError, CudaProvider,
    CudaVisibleDevice, DeviceResolutionError, DeviceResolutionReason, DoctorGpuStatus,
    EffectiveEmbedDevice, EffectiveRerankerDevice, EmbedDevicePolicy, RerankerDevicePolicy,
    RerankerDeviceResolutionError, RerankerDeviceResolutionReason,
};

/// Devices with fixed compute capabilities and a fixed kernel floor; every
/// `probe_cuda` call (the stand-in for device construction) is recorded.
struct FloorProvider {
    devices: Vec<CudaVisibleDevice>,
    floor: Option<(u32, u32)>,
    probed: Vec<usize>,
}

impl FloorProvider {
    fn new(capabilities: &[Option<&str>], floor: Option<(u32, u32)>) -> Self {
        let devices = capabilities
            .iter()
            .enumerate()
            .map(|(ordinal, capability)| CudaVisibleDevice {
                visible_ordinal: ordinal,
                uuid: format!("GPU-floor-{ordinal}"),
                name: format!("floor-test-{ordinal}"),
                compute_capability: capability.map(str::to_owned),
            })
            .collect();
        Self { devices, floor, probed: Vec::new() }
    }
}

impl CudaProvider for FloorProvider {
    fn enumerate_visible_cuda_devices(&mut self) -> Result<Vec<CudaVisibleDevice>, CudaProbeError> {
        Ok(self.devices.clone())
    }

    fn probe_cuda(&mut self, ordinal: usize) -> Result<CudaDeviceInfo, CudaProbeError> {
        self.probed.push(ordinal);
        let device = &self.devices[ordinal];
        Ok(CudaDeviceInfo::new(
            ordinal,
            Some(device.uuid.clone()),
            Some(device.name.clone()),
            None,
            device.compute_capability.clone(),
            None,
        ))
    }

    fn kernel_compute_floor(&self) -> Option<(u32, u32)> {
        self.floor
    }
}

const X86_FLOOR: Option<(u32, u32)> = Some((7, 5));

#[test]
fn forced_cuda_on_a_device_below_the_kernel_floor_is_cuda_incompatible_without_a_probe() {
    let mut provider = FloorProvider::new(&[Some("5.0")], X86_FLOOR);
    let result = resolve_embed_device_policy(EmbedDevicePolicy::Cuda(0), true, &mut provider);
    assert_eq!(
        result,
        Err(DeviceResolutionError::ForcedCudaUnavailable {
            ordinal: 0,
            reason: DeviceResolutionReason::CudaIncompatible,
        })
    );
    assert!(
        provider.probed.is_empty(),
        "no device may be built below the floor: {:?}",
        provider.probed
    );
}

#[test]
fn auto_with_the_lowest_ordinal_below_the_floor_falls_back_to_cpu() {
    let mut provider = FloorProvider::new(&[Some("5.0"), Some("8.6")], X86_FLOOR);
    let resolution = resolve_embed_device_policy(EmbedDevicePolicy::Auto, true, &mut provider)
        .expect("auto never fails below the floor");
    assert_eq!(resolution.effective_device, EffectiveEmbedDevice::Cpu);
    assert_eq!(resolution.reason, Some(DeviceResolutionReason::CudaIncompatible));
    assert!(provider.probed.is_empty(), "auto must not probe any device: {:?}", provider.probed);
}

#[test]
fn auto_with_the_lowest_ordinal_above_the_floor_selects_it() {
    let mut provider = FloorProvider::new(&[Some("8.6"), Some("5.0")], X86_FLOOR);
    let resolution = resolve_embed_device_policy(EmbedDevicePolicy::Auto, true, &mut provider)
        .expect("auto resolves");
    assert!(
        matches!(resolution.effective_device, EffectiveEmbedDevice::Cuda(ref info) if info.ordinal == 0)
    );
    assert_eq!(provider.probed, vec![0]);
}

#[test]
fn a_device_at_the_floor_is_probed() {
    let mut provider = FloorProvider::new(&[Some("7.5")], X86_FLOOR);
    let resolution = resolve_embed_device_policy(EmbedDevicePolicy::Cuda(0), true, &mut provider)
        .expect("a 7.5 device meets the 7.5 floor");
    assert!(matches!(resolution.effective_device, EffectiveEmbedDevice::Cuda(_)));
    assert_eq!(provider.probed, vec![0]);
}

#[test]
fn an_unknown_compute_capability_is_probed() {
    let mut provider = FloorProvider::new(&[None], X86_FLOOR);
    let resolution = resolve_embed_device_policy(EmbedDevicePolicy::Cuda(0), true, &mut provider)
        .expect("no evidence is no refusal");
    assert!(matches!(resolution.effective_device, EffectiveEmbedDevice::Cuda(_)));
    assert_eq!(provider.probed, vec![0]);
}

#[test]
fn doctor_gpu_reports_cuda_incompatible_below_the_floor() {
    let mut provider = FloorProvider::new(&[Some("5.0")], X86_FLOOR);
    let diagnostic = diagnose_gpu(EmbedDevicePolicy::Cuda(0), true, &mut provider);
    assert_eq!(diagnostic.status(), DoctorGpuStatus::CudaIncompatible);
    assert_eq!(diagnostic.reason, Some(DeviceResolutionReason::CudaIncompatible));
    assert!(
        provider.probed.is_empty(),
        "doctor gpu must not build the device: {:?}",
        provider.probed
    );
}

#[test]
fn reranker_forced_cuda_below_the_floor_is_cuda_incompatible_without_a_probe() {
    let mut provider = FloorProvider::new(&[Some("5.0")], X86_FLOOR);
    let result = resolve_reranker_device_policy(RerankerDevicePolicy::Cuda(0), true, &mut provider);
    assert_eq!(
        result,
        Err(RerankerDeviceResolutionError::ForcedCudaUnavailable {
            ordinal: 0,
            reason: RerankerDeviceResolutionReason::CudaIncompatible,
        })
    );
    assert!(
        provider.probed.is_empty(),
        "no device may be built below the floor: {:?}",
        provider.probed
    );
}

#[test]
fn reranker_auto_below_the_floor_falls_back_to_cpu() {
    let mut provider = FloorProvider::new(&[Some("5.0")], X86_FLOOR);
    let resolution =
        resolve_reranker_device_policy(RerankerDevicePolicy::Auto, true, &mut provider)
            .expect("auto never fails below the floor");
    assert_eq!(resolution.effective_device, EffectiveRerankerDevice::Cpu);
    assert_eq!(resolution.reason, Some(RerankerDeviceResolutionReason::CudaIncompatible));
    assert!(provider.probed.is_empty(), "auto must not probe any device: {:?}", provider.probed);
}

#[test]
fn the_build_floor_parses_the_contract_values() {
    assert_eq!(parse_compute_capability_floor("75"), Some((7, 5)));
    assert_eq!(parse_compute_capability_floor("87"), Some((8, 7)));
    assert_eq!(parse_compute_capability_floor("120"), Some((12, 0)));
    for invalid in ["", "7", "7.5", "sm_75", "7a", " 75"] {
        assert_eq!(parse_compute_capability_floor(invalid), None, "{invalid:?}");
    }
}
