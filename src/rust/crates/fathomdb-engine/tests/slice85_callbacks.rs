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

struct Toggle(Weak<Engine>);
impl Subscriber for Toggle {
    fn on_event(&self, _: &fathomdb_engine::lifecycle::Event) {}
    fn on_profile(&self, _: &ProfileRecord) {
        self.0.upgrade().unwrap().set_profiling(false).unwrap();
    }
}
#[test]
#[ignore]
fn profile_toggle_child() {
    let dir = tempfile::tempdir().unwrap();
    let policy = if std::env::var("S85_TOGGLE_POLICY").unwrap() == "explicit" {
        PhysicalErasurePolicy::ExplicitMaintenance
    } else {
        PhysicalErasurePolicy::Autonomous
    };
    let engine = Arc::new(
        Engine::open_with_choice_and_config(
            dir.path().join("toggle.sqlite"),
            EmbedderChoice::None,
            EngineConfig { physical_erasure_policy: Some(policy), ..Default::default() },
        )
        .unwrap()
        .engine,
    );
    let _sub = engine.subscribe(Arc::new(Toggle(Arc::downgrade(&engine))));
    engine.set_profiling(true).unwrap();
    assert_eq!(engine.query_i64_col_for_test("SELECT 1").unwrap(), vec![1]);
    engine.close().unwrap();
}
#[test]
fn profile_callback_can_toggle_profiling_without_recursively_locking_sqlite() {
    let mut outcomes = Vec::new();
    for policy in ["autonomous", "explicit"] {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "profile_toggle_child", "--ignored", "--nocapture"])
            .env("S85_TOGGLE_POLICY", policy)
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break Some(status);
            }
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                break None;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        outcomes.push((policy, status.is_some_and(|s| s.success())));
    }
    assert!(outcomes.iter().all(|(_, ok)| *ok), "profile callback setter deadlocked: {outcomes:?}");
}
