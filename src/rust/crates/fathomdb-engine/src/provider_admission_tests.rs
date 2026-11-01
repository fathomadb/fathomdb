use super::*;
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use std::sync::Arc;
use tempfile::TempDir;

struct Provider {
    identity: EmbedderIdentity,
    value: f32,
}
impl Embedder for Provider {
    fn identity(&self) -> EmbedderIdentity {
        self.identity.clone()
    }
    fn embed(&self, _: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![self.value; self.identity.dimension as usize])
    }
}
fn seed(path: &std::path::Path, identity: &EmbedderIdentity) {
    let provider = || Arc::new(Provider { identity: identity.clone(), value: 1.0 });
    let opened = Engine::open_with_embedder_for_test(path, provider()).unwrap();
    opened.engine.configure_vector_kind_for_test("note").unwrap();
    opened.engine.close().unwrap();
    Engine::open_with_embedder_for_test(path, provider()).unwrap().engine.close().unwrap();
    Connection::open(path)
        .unwrap()
        .execute(
            "DELETE FROM _fathomdb_open_state WHERE key='vector_equivalence_verified_fingerprint'",
            [],
        )
        .unwrap();
}

#[test]
fn verification_mean_read_failure_is_storage_not_invalid_state() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("mean-read.fathomdb");
    let identity = EmbedderIdentity::new("fathomdb-bge-small-en-v1.5", "test", 8);
    seed(&path, &identity);
    let mut connection = Connection::open(&path).unwrap();
    connection
        .authorizer(Some(|context: AuthContext<'_>| match context.action {
            AuthAction::Read { table_name: "_fathomdb_embedder_profiles", .. } => {
                Authorization::Deny
            }
            _ => Authorization::Allow,
        }))
        .unwrap();
    let dispatch = EmbedDispatcher::new(
        Some(Arc::new(Provider { identity: identity.clone(), value: 1.0 })),
        1,
        Duration::from_secs(1),
    )
    .unwrap();
    let error = run_vector_equivalence_probe(&connection, Some(&dispatch), &identity, true, false)
        .unwrap_err();
    dispatch.close();
    drop(dispatch.join_after_quiescence());
    assert!(
        matches!(error, EngineOpenError::Io { .. }),
        "SQL read failure must retain storage classification: {error:?}"
    );
}

use std::sync::Mutex;

#[derive(Default)]
struct Events(Mutex<Vec<lifecycle::Event>>);
impl lifecycle::Subscriber for Events {
    fn on_event(&self, event: &lifecycle::Event) {
        self.0.lock().unwrap().push(event.clone());
    }
}

#[test]
fn initial_subscriber_receives_actual_verification_refusal() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("subscriber.fathomdb");
    let identity = EmbedderIdentity::new("subscriber-provider", "test", 8);
    seed(&path, &identity);
    let events = Arc::new(Events::default());
    let subscriber: Arc<dyn lifecycle::Subscriber> = events.clone();
    let error = Engine::open_with_embedder_and_subscriber(
        &path,
        identity.clone(),
        Some(Arc::new(Provider { identity, value: -1.0 })),
        None,
        Some(subscriber),
        &mut |_| {},
    )
    .err()
    .unwrap();
    assert!(matches!(error, EngineOpenError::VectorEquivalenceMismatch { .. }));
    let events = events.0.lock().unwrap();
    assert_eq!(events.len(), 1, "initial subscriber must receive exactly one refusal");
    assert_eq!(events[0].phase, lifecycle::Phase::Failed);
    assert_eq!(events[0].source, lifecycle::EventSource::Engine);
    assert_eq!(events[0].category, lifecycle::EventCategory::Error);
    assert_eq!(events[0].code, Some("VectorEquivalenceMismatchError"));
}
