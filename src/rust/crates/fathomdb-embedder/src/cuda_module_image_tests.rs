use super::{classify_candle_cuda_error, cuda_uuid_string};
use crate::device_policy::{
    resolve_embed_device_policy, CudaDeviceInfo, CudaProbeError, CudaProvider, CudaVisibleDevice,
    DeviceResolutionError, DeviceResolutionReason, EffectiveEmbedDevice, EmbedDevicePolicy,
};
use candle_core::cuda::cudarc::{
    driver::{sys::CUresult, CudaContext, DriverError},
    nvrtc::Ptx,
};

struct ObservedModuleFailure {
    visible: CudaVisibleDevice,
    failure: CudaProbeError,
}

impl CudaProvider for ObservedModuleFailure {
    fn enumerate_visible_cuda_devices(&mut self) -> Result<Vec<CudaVisibleDevice>, CudaProbeError> {
        Ok(vec![self.visible.clone()])
    }

    fn probe_cuda(&mut self, ordinal: usize) -> Result<CudaDeviceInfo, CudaProbeError> {
        assert_eq!(ordinal, self.visible.visible_ordinal);
        Err(self.failure.clone())
    }
}

fn assert_native_failure_policy(
    visible: &CudaVisibleDevice,
    error: DriverError,
    reason: DeviceResolutionReason,
) {
    let failure = classify_candle_cuda_error(candle_core::Error::Cuda(Box::new(
        candle_core::cuda::CudaError::Cuda(error),
    )));
    match reason {
        DeviceResolutionReason::CudaIncompatible => {
            assert!(matches!(failure, CudaProbeError::Incompatible { .. }));
        }
        DeviceResolutionReason::CudaProbeFailed => {
            assert!(matches!(failure, CudaProbeError::ProbeFailed { .. }));
        }
        _ => panic!("unexpected selected module refusal"),
    }
    let mut observed = ObservedModuleFailure { visible: visible.clone(), failure };
    let automatic = resolve_embed_device_policy(EmbedDevicePolicy::Auto, true, &mut observed)
        .expect("ordinary unavailable module permits the existing auto fallback");
    assert_eq!(automatic.effective_device, EffectiveEmbedDevice::Cpu);
    assert_eq!(automatic.reason, Some(reason));
    let forced = resolve_embed_device_policy(EmbedDevicePolicy::Cuda(0), true, &mut observed)
        .expect_err("forced CUDA must remain typed without CPU substitution");
    assert_eq!(forced, DeviceResolutionError::ForcedCudaUnavailable { ordinal: 0, reason });
}

#[test]
#[ignore = "exact native CUDA child and committed sm90 fixture; exclusive lease and resource bounds"]
fn native_wrong_image_and_missing_function_preserve_classifier_and_device_policy() {
    assert_eq!(std::env::var("FATHOMDB_REQUIRE_LIVE").as_deref(), Ok("1"));
    let image = std::env::var("FATHOMDB_TEST_SM90_CUBIN")
        .expect("use the inspected CUDA12.6 sm90-only cubin from the owning Candle fixture");
    let context = CudaContext::new(0).expect("actual selected native context");
    let (major, minor) = context.compute_capability().unwrap();
    assert!(matches!((major, minor), (8, 6) | (8, 7)));
    let visible = CudaVisibleDevice {
        visible_ordinal: context.ordinal(),
        uuid: cuda_uuid_string(context.uuid().unwrap().bytes),
        name: context.name().unwrap(),
        compute_capability: Some(format!("{major}.{minor}")),
    };
    let wrong = context.load_module(Ptx::from_file(image)).unwrap_err();
    assert_eq!(wrong.0, CUresult::CUDA_ERROR_NO_BINARY_FOR_GPU);
    assert_native_failure_policy(&visible, wrong, DeviceResolutionReason::CudaIncompatible);

    let module = context.load_module(candle_core::cuda::kernels::FILL.ptx().into()).unwrap();
    let missing = module.load_function("fathomdb_deliberately_absent_function").unwrap_err();
    assert_eq!(missing.0, CUresult::CUDA_ERROR_NOT_FOUND);
    assert_native_failure_policy(&visible, missing, DeviceResolutionReason::CudaProbeFailed);
    println!("module-policy actual-device={} uuid={} wrong-image=209 missing-function=500 auto-reasons-and-forced-errors-validated", visible.name, visible.uuid);
}
