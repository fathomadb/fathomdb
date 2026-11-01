use super::{
    acquire_lock_without_metadata_mutation, install_post_probe_visibility_fault_for_test, Engine,
    EngineConfig, EngineOpenError, TestEmbedderChoice, VectorVerificationFailure,
};
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use tempfile::TempDir;

static VISIBILITY_FAULT_TEST_LOCK: Mutex<()> = Mutex::new(());

struct FixedProvider;

impl Embedder for FixedProvider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice90-real-error", "r1", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0; 8])
    }
}

struct HeldProvider {
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}

impl Embedder for HeldProvider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice90-real-error", "r1", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        self.entered.send(()).expect("report provider entry");
        self.release.lock().expect("release lock").recv().expect("release provider");
        Ok(vec![1.0; 8])
    }
}

#[test]
fn verification_timeout_precedes_later_visibility_fault_and_retains_provider_only() {
    let _serial = VISIBILITY_FAULT_TEST_LOCK.lock().unwrap();
    let dir = TempDir::new().expect("temp db");
    let path = dir.path().join("visibility-error.sqlite");
    let initial = Engine::open_with_test_choice_and_config_for_test(
        &path,
        TestEmbedderChoice::Caller(Arc::new(FixedProvider)),
        EngineConfig::default(),
    )
    .expect("first open");
    initial.engine.configure_vector_kind_for_test("doc").expect("enrol vector kind");
    initial.engine.close().expect("close first session");

    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let provider = Arc::new(HeldProvider { entered: entered_tx, release: Mutex::new(release_rx) });
    let (observed_tx, observed_rx) = mpsc::channel();
    install_post_probe_visibility_fault_for_test(path.clone(), observed_tx);
    let release_thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        release_tx.send(()).expect("release provider if still held");
    });
    let result = Engine::open_with_test_choice_and_config_for_test(
        &path,
        TestEmbedderChoice::Caller(provider),
        EngineConfig { embedder_call_timeout_ms: Some(50), ..EngineConfig::default() },
    );
    let error = match result {
        Ok(opened) => {
            opened.engine.close().expect("close unexpected open");
            panic!("timed-out verification was accepted")
        }
        Err(error) => error,
    };
    entered_rx.recv_timeout(Duration::from_secs(2)).expect("probe called provider");
    let (accounting, registry) =
        observed_rx.recv_timeout(Duration::from_secs(2)).expect("fault observation");
    let retained_at_return = accounting.snapshot().live_workers;
    let owners_at_return = registry.live.lock().expect("registry").len();
    let admission = acquire_lock_without_metadata_mutation(&path);
    let admission_released = admission.is_ok();
    drop(admission);
    release_thread.join().expect("release thread");
    let started = Instant::now();
    while accounting.snapshot().live_workers != 0 {
        assert!(started.elapsed() < Duration::from_secs(2), "provider worker did not exit");
        std::thread::yield_now();
    }

    assert!(matches!(
        error,
        EngineOpenError::VectorVerification {
            cause: VectorVerificationFailure::StartedTimeout,
            ..
        }
    ));
    assert_eq!(retained_at_return, 1, "timed-out provider remains counted");
    assert_eq!(owners_at_return, 0, "all SQLite owners must leave on real error path");
    assert!(admission_released, "real error path must release admission lock");
}

#[test]
fn accepted_probe_still_preserves_later_visibility_error_and_releases_database_owners() {
    let _serial = VISIBILITY_FAULT_TEST_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("accepted-visibility-error.sqlite");
    let initial = Engine::open_with_test_choice_for_test(
        &path,
        TestEmbedderChoice::Caller(Arc::new(FixedProvider)),
    )
    .unwrap();
    initial.engine.configure_vector_kind_for_test("doc").unwrap();
    initial.engine.close().unwrap();
    let (observed_tx, observed_rx) = mpsc::channel();
    install_post_probe_visibility_fault_for_test(path.clone(), observed_tx);
    let result = Engine::open_with_test_choice_for_test(
        &path,
        TestEmbedderChoice::Caller(Arc::new(FixedProvider)),
    );
    assert!(
        matches!(result, Err(EngineOpenError::Io { message }) if message == "could not load frozen-read visibility generation")
    );
    let (accounting, registry) = observed_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(accounting.snapshot().live_workers, 0);
    assert_eq!(registry.live.lock().unwrap().len(), 0);
    assert!(acquire_lock_without_metadata_mutation(&path).is_ok());
}
