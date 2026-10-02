use super::{
    acquire_lock_without_metadata_mutation, install_post_probe_visibility_fault_for_test,
    EmbedderChoice, Engine, EngineConfig, EngineOpenError,
};
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use tempfile::TempDir;

struct FixedProvider;

impl Embedder for FixedProvider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice90-real-error", "r1", 2)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0, 0.0])
    }
}

struct HeldProvider {
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}

impl Embedder for HeldProvider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice90-real-error", "r1", 2)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        self.entered.send(()).expect("report provider entry");
        self.release.lock().expect("release lock").recv().expect("release provider");
        Ok(vec![1.0, 0.0])
    }
}

#[test]
fn real_post_probe_visibility_error_uses_shared_unwind_and_retains_provider_only() {
    let dir = TempDir::new().expect("temp db");
    let path = dir.path().join("visibility-error.sqlite");
    let initial = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Caller(Arc::new(FixedProvider)),
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
    let result = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Caller(provider),
        EngineConfig { embedder_call_timeout_ms: Some(50), ..EngineConfig::default() },
    );
    let error = match result {
        Ok(opened) => {
            opened.engine.close().expect("close unexpected open");
            panic!("missing visibility singleton was accepted")
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

    assert!(
        matches!(error, EngineOpenError::Io { message } if message == "could not load frozen-read visibility generation")
    );
    assert_eq!(retained_at_return, 1, "timed-out provider remains counted");
    assert_eq!(owners_at_return, 0, "all SQLite owners must leave on real error path");
    assert!(admission_released, "real error path must release admission lock");
}
