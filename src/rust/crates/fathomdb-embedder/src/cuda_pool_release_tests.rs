//! 0.8.28 Slice 31 N1 (design § 14.1, owner rulings Q29 and Q34): the close
//! release. Each model built on the private pool counts while it is open;
//! when the last one closes, the decision's context is synchronized and the
//! pool's unused memory trimmed, unless the context is lost. Any error or
//! panic is discarded.

use std::sync::atomic::Ordering;

use super::test_fake::{Fake, FakeDecision, FIRST_CONTEXT_ID};
use super::*;

fn live(decision: &FakeDecision) -> usize {
    match decision {
        Decision::Private(private) => private.live_models.load(Ordering::SeqCst),
        Decision::Fallback { .. } => panic!("expected a private decision"),
    }
}

#[test]
fn only_a_model_on_the_decided_private_pool_counts() {
    let driver = Fake::default();
    let decision = driver.private_decision();
    // A CPU model, or a CUDA model on the 0.8.27 path, gets an inert guard.
    assert!(!register_private_model(Some(&decision), 0, false));
    // Another ordinal never allocates from the pool.
    assert!(!register_private_model(Some(&decision), 1, true));
    assert_eq!(live(&decision), 0);
    assert!(register_private_model(Some(&decision), 0, true));
    assert_eq!(live(&decision), 1);
}

#[test]
fn no_model_counts_without_a_private_decision() {
    let driver = Fake { init: ModuleLoadInit::OptedOut, ..Fake::default() };
    let fallback = decide_and_build(&driver, 0);
    assert!(matches!(fallback, Decision::Fallback { .. }));
    assert!(!register_private_model(Some(&fallback), 0, true));
    assert!(!register_private_model::<(), ()>(None, 0, true));
    assert_eq!(release_unused_pool_memory(Some(&fallback), &driver), ReleaseOutcome::NotPrivate);
    assert_eq!(release_unused_pool_memory::<Fake>(None, &driver), ReleaseOutcome::NotPrivate);
    assert_eq!(driver.calls(), (0, 0));
}

#[test]
fn the_pool_is_trimmed_only_when_the_last_model_closes() {
    let driver = Fake::default();
    let decision = driver.private_decision();
    assert!(register_private_model(Some(&decision), 0, true));
    assert!(register_private_model(Some(&decision), 0, true));
    assert_eq!(release_private_model(Some(&decision), &driver), ReleaseOutcome::StillOpen);
    assert_eq!(driver.calls(), (0, 0), "a close while another model is open must not synchronize");
    assert_eq!(release_private_model(Some(&decision), &driver), ReleaseOutcome::Trimmed);
    assert_eq!(driver.calls(), (1, 1));
    assert_eq!(live(&decision), 0);
}

#[test]
fn a_model_held_for_the_process_life_keeps_the_pool_reserved() {
    // A module-level singleton is registered and never dropped.
    let driver = Fake::default();
    let decision = driver.private_decision();
    assert!(register_private_model(Some(&decision), 0, true));
    for _ in 0..3 {
        assert!(register_private_model(Some(&decision), 0, true));
        assert_eq!(release_private_model(Some(&decision), &driver), ReleaseOutcome::StillOpen);
    }
    assert_eq!(driver.calls(), (0, 0));
    assert_eq!(live(&decision), 1);
}

#[test]
fn the_release_is_skipped_when_the_context_is_lost() {
    let inactive = Fake::default();
    let decision = inactive.private_decision();
    inactive.primary_active.store(false, Ordering::SeqCst);
    assert_eq!(release_unused_pool_memory(Some(&decision), &inactive), ReleaseOutcome::ContextLost);
    assert_eq!(inactive.calls(), (0, 0));

    let replaced = Fake::default();
    let decision = replaced.private_decision();
    *replaced.current_id.lock().unwrap() = Ok(FIRST_CONTEXT_ID + 1);
    assert_eq!(release_unused_pool_memory(Some(&decision), &replaced), ReleaseOutcome::ContextLost);
    assert_eq!(replaced.calls(), (0, 0));

    let unreadable = Fake::default();
    let decision = unreadable.private_decision();
    *unreadable.current_id.lock().unwrap() = Err(CUDA_ERROR_CONTEXT_IS_DESTROYED);
    assert_eq!(
        release_unused_pool_memory(Some(&decision), &unreadable),
        ReleaseOutcome::ContextLost
    );
    assert_eq!(unreadable.calls(), (0, 0));
}

#[test]
fn a_driver_error_is_discarded() {
    let sync_fails = Fake::default();
    let decision = sync_fails.private_decision();
    sync_fails.fail_synchronize.store(true, Ordering::SeqCst);
    assert_eq!(
        release_unused_pool_memory(Some(&decision), &sync_fails),
        ReleaseOutcome::DriverError
    );
    assert_eq!(sync_fails.calls(), (1, 0), "no trim after a failed synchronize");

    let trim_fails = Fake::default();
    let decision = trim_fails.private_decision();
    trim_fails.fail_trim.store(true, Ordering::SeqCst);
    assert_eq!(
        release_unused_pool_memory(Some(&decision), &trim_fails),
        ReleaseOutcome::DriverError
    );
    assert_eq!(trim_fails.calls(), (1, 1));
}

#[test]
fn a_panic_in_the_release_is_caught() {
    let driver = Fake::default();
    let decision = driver.private_decision();
    driver.panic_in_trim.store(true, Ordering::SeqCst);
    assert!(register_private_model(Some(&decision), 0, true));
    assert_eq!(release_private_model(Some(&decision), &driver), ReleaseOutcome::Panicked);
    assert_eq!(live(&decision), 0);
}

#[test]
fn a_release_without_a_registered_model_never_underflows() {
    let driver = Fake::default();
    let decision = driver.private_decision();
    assert_eq!(release_private_model(Some(&decision), &driver), ReleaseOutcome::StillOpen);
    assert_eq!(live(&decision), 0);
    assert_eq!(driver.calls(), (0, 0));
}
