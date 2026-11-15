use super::test_fake::{Fake, FIRST_CONTEXT_ID};
use super::*;
use candle_core::cuda::{
    cudarc::{
        cublas::result::CublasError,
        driver::{sys::CUresult, NativeLibrary, PrimaryContextLoss, PrimaryContextLossCause},
    },
    CudaError,
};

fn lost_error(cause: PrimaryContextLossCause, observed_id: Option<u64>) -> candle_core::Error {
    candle_core::Error::Cuda(Box::new(CudaError::Cublas(CublasError::Lost(PrimaryContextLoss {
        recorded_id: FIRST_CONTEXT_ID,
        observed_id,
        cause,
    }))))
}

#[test]
fn first_token_oom_is_context_loss_before_pool_exhaustion() {
    let driver = Fake::default();
    let decision = driver.private_decision();
    let error =
        lost_error(PrimaryContextLossCause::DriverStatus(CUresult::CUDA_ERROR_OUT_OF_MEMORY), None);
    let actual = classify_candle_fault(Some(&decision), &driver, &error, 0, "forward");
    assert!(matches!(actual, Some(CudaPoolFailure::ContextLost { .. })), "{actual:?}");
    assert_eq!(driver.state_reads.load(Ordering::SeqCst), 0);
    assert_eq!(driver.id_reads.load(Ordering::SeqCst), 0);
}

#[test]
fn cached_loss_keeps_native_or_synthetic_provenance() {
    for (cause, id, expected) in [
        (
            PrimaryContextLossCause::DriverStatus(CUresult::CUDA_ERROR_UNKNOWN),
            None,
            "native driver status 999",
        ),
        (PrimaryContextLossCause::PrimaryInactive, None, "synthetic 709: primary inactive"),
        (
            PrimaryContextLossCause::IdentityChanged,
            Some(9),
            "synthetic 709: context identity changed",
        ),
        (
            PrimaryContextLossCause::NativeLibraryCleanup {
                library: NativeLibrary::Curand,
                status: 201,
            },
            None,
            "Curand cleanup status 201; synthetic 999",
        ),
        (PrimaryContextLossCause::SyntheticUnknown, None, "synthetic 999: unknown cause"),
    ] {
        let driver = Fake::default();
        let decision = driver.private_decision();
        let error = lost_error(cause, id).context("outer");
        let actual = classify_candle_fault(Some(&decision), &driver, &error, 0, "batch forward");
        let Some(CudaPoolFailure::ContextLost {
            recorded_context_id,
            current_context_id,
            driver_error,
            operation,
        }) = actual
        else {
            panic!("lost error became {actual:?}")
        };
        assert_eq!(recorded_context_id, FIRST_CONTEXT_ID);
        assert_eq!(current_context_id, id);
        assert!(driver_error.contains(expected), "{driver_error:?}");
        assert_eq!(operation, "batch forward");
        assert_eq!(driver.state_reads.load(Ordering::SeqCst), 0);
        assert_eq!(driver.id_reads.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn ordinary_private_oom_without_loss_stays_exhausted() {
    let driver = Fake::default();
    let decision = driver.private_decision();
    let error = candle_core::Error::Cuda(Box::new(CudaError::Cuda(
        candle_core::cuda::cudarc::driver::DriverError(CUresult::CUDA_ERROR_OUT_OF_MEMORY),
    )));
    assert!(matches!(
        classify_candle_fault(Some(&decision), &driver, &error, 0, "forward"),
        Some(CudaPoolFailure::Exhausted { .. })
    ));
    assert_eq!(driver.state_reads.load(Ordering::SeqCst), 0);
}

#[test]
fn cached_library_loss_is_typed_on_default_allocator() {
    let driver = Fake::default();
    let error = lost_error(PrimaryContextLossCause::PrimaryInactive, None);
    let actual = classify_candle_fault(None, &driver, &error, 0, "forward");
    assert!(matches!(actual, Some(CudaPoolFailure::ContextLost { .. })), "{actual:?}");
    assert_eq!(driver.state_reads.load(Ordering::SeqCst), 0);
}

#[test]
fn library_driver_status_keeps_driver_identity() {
    use candle_core::cuda::cudarc::{curand::result::CurandError, driver::DriverError};
    for error in [
        CudaError::Cublas(CublasError::Driver(DriverError(CUresult::CUDA_ERROR_OUT_OF_MEMORY))),
        CudaError::Curand(CurandError::Driver(DriverError(CUresult::CUDA_ERROR_INVALID_CONTEXT))),
    ] {
        let expected = match &error {
            CudaError::Cublas(_) => 2,
            _ => 201,
        };
        assert_eq!(candle_driver_code(&candle_core::Error::Cuda(Box::new(error))), Some(expected));
    }
}
