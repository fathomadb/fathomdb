use super::test_fake::Fake;
use super::*;

#[test]
fn drop_probe_loss_is_published_after_refused_cleanup_admission() {
    let driver = Fake::default();
    let decision = driver.private_decision();
    driver.tracked_context.store(true, Ordering::SeqCst);
    driver.lose_on_admission.store(true, Ordering::SeqCst);
    assert_eq!(release_unused_pool_memory(Some(&decision), &driver), ReleaseOutcome::ContextLost);
    assert!(
        driver.published_cached_loss.load(Ordering::SeqCst),
        "loss discovered by cleanup admission never reached product publication"
    );
    assert_eq!(driver.calls(), (0, 0));
    assert_eq!(driver.state_reads.load(Ordering::SeqCst), 0);
    assert_eq!(driver.id_reads.load(Ordering::SeqCst), 0);
}
