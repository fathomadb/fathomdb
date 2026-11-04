use fathomdb_engine::{
    lifecycle::{ProfileRecord, Subscriber},
    *,
};
use std::sync::{Arc, Mutex, Weak};

struct Attempts {
    engine: Weak<Engine>,
    results: Mutex<Vec<bool>>,
}
impl Subscriber for Attempts {
    fn on_event(&self, _: &fathomdb_engine::lifecycle::Event) {}
    fn on_profile(&self, _: &ProfileRecord) {
        let Some(engine) = self.engine.upgrade() else { return };
        let refused_as_self_pin = matches!(
            engine.begin_erasure_maintenance(0),
            Err(EngineError::Maintenance(MaintenanceError::SelfPinnedReader))
        );
        self.results.lock().unwrap().push(refused_as_self_pin);
    }
}

#[test]
fn reader_profile_callback_acquisition_reports_self_pin_without_waiting() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Arc::new(
        Engine::open_with_choice_and_config(
            dir.path().join("callback.sqlite"),
            EmbedderChoice::None,
            EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
        )
        .unwrap()
        .engine,
    );
    let attempts =
        Arc::new(Attempts { engine: Arc::downgrade(&engine), results: Mutex::new(Vec::new()) });
    let _subscription = engine.subscribe(attempts.clone());
    engine.set_profiling(true).unwrap();
    engine.read_get("absent", &ReadView::default()).unwrap();
    let results = attempts.results.lock().unwrap();
    assert!(!results.is_empty(), "real reader profile callback must execute");
    assert!(
        results.iter().all(|result| *result),
        "callback begin must report self-pin, never attempt drain"
    );
    drop(results);
    engine.close().unwrap();
}
