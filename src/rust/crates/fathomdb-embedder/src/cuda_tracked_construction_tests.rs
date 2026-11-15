use super::test_fake::{Fake, FIRST_CONTEXT_ID};
use super::*;

#[test]
fn lost_default_device_refuses_before_wrap() {
    let driver = Fake { init: ModuleLoadInit::OptedOut, ..Fake::default() };
    *driver.cached_loss.lock().unwrap() = Some(CudaPoolFailure::ContextLost {
        recorded_context_id: FIRST_CONTEXT_ID,
        current_context_id: None,
        driver_error: "native driver status 709".into(),
        operation: "earlier drop".into(),
    });
    let cell = OnceLock::new();
    let actual = build_device(&cell, &driver, 0);
    assert!(
        matches!(actual, Err(DeviceBuildError::Pool(CudaPoolFailure::ContextLost { .. }))),
        "{actual:?}"
    );
    assert_eq!(driver.default_builds.load(Ordering::SeqCst), 0);
    assert_eq!(driver.context_wraps.load(Ordering::SeqCst), 0);
}

#[test]
fn loss_during_initial_pool_decision_never_falls_back_or_wraps() {
    let driver = Fake::default();
    driver.lose_on_pool_create.store(true, Ordering::SeqCst);
    let actual = build_device(&OnceLock::new(), &driver, 0);
    assert!(
        matches!(actual, Err(DeviceBuildError::Pool(CudaPoolFailure::ContextLost { .. }))),
        "{actual:?}"
    );
    assert_eq!(driver.default_builds.load(Ordering::SeqCst), 0);
    assert_eq!(driver.context_wraps.load(Ordering::SeqCst), 0);
}
