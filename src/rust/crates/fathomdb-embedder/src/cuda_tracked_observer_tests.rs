use super::test_fake::Fake;
use super::*;

#[test]
fn tracked_observer_reads_only_under_admission() {
    let driver = Fake::default();
    let decision = driver.private_decision();
    driver.tracked_context.store(true, Ordering::SeqCst);
    let actual = observation_with_driver(Some(&decision), &driver);
    assert_eq!(actual.status, "private");
    assert_eq!(actual.used_current_bytes, Some(11));
    assert_eq!(actual.reserved_current_bytes, Some(29));
    assert_eq!(driver.admissions.load(Ordering::SeqCst), 1);
    assert_eq!(driver.pool_reads.load(Ordering::SeqCst), 1);
    assert_eq!(driver.calls(), (0, 0));
}

#[test]
fn observer_loss_during_reads_discards_partial_counters() {
    let driver = Fake::default();
    let decision = driver.private_decision();
    driver.tracked_context.store(true, Ordering::SeqCst);
    driver.lose_on_pool_read.store(true, Ordering::SeqCst);
    let actual = observation_with_driver(Some(&decision), &driver);
    assert_eq!(actual.status, "lost");
    assert_eq!(actual.used_current_bytes, None);
    assert_eq!(actual.reserved_current_bytes, None);
    assert!(actual.error.is_some());
    assert_eq!(driver.admissions.load(Ordering::SeqCst), 1);
    assert_eq!(driver.pool_reads.load(Ordering::SeqCst), 1);
    assert_eq!(driver.calls(), (0, 0));
}
