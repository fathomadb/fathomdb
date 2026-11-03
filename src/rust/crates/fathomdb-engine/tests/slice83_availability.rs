use std::sync::{mpsc, Arc};
use std::time::Duration;

use fathomdb_engine::{Engine, EngineError, InitialState, PreparedWrite, SourceId};

fn node(id: &str, source: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

#[test]
fn committed_sanitation_unit_releases_primary_for_native_read() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Arc::new(Engine::open(dir.path().join("yield.sqlite")).unwrap().engine);
    engine
        .write(&[node("target", "target", "privateword"), node("survivor", "other", "publicword")])
        .unwrap();
    let other = Arc::clone(&engine);
    fathomdb_engine::arm_physical_erasure_hook_for_test(
        "after_sanitation_unit",
        Box::new(move || {
            let (tx, rx) = mpsc::channel();
            let reader = Arc::clone(&other);
            std::thread::spawn(move || {
                let _ = tx.send(reader.node_importance(2));
            });
            rx.recv_timeout(Duration::from_secs(2)).map_err(|_| EngineError::Scheduler)??;
            Ok(())
        }),
    );
    engine.erase_source("target").expect(
        "a committed sanitation unit must release the primary connection before its rendezvous",
    );
    engine.close().unwrap();
}
