use fathomdb_engine::{Engine, EngineError, InitialState, PreparedWrite, SourceId};
use std::{
    path::Path,
    sync::{mpsc, Arc},
    time::Duration,
};

fn setup(path: &Path) -> Engine {
    let engine = Engine::open(path).unwrap().engine;
    engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "privateuniqueterm".into(),
            source_id: SourceId::new("target").unwrap(),
            logical_id: Some("subject".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    engine
}

fn event(path: &Path) {
    std::fs::write(
        path,
        "{\"type\":\"event\",\"result_ids\":[1],\"result_stable_ids\":[\"l:subject\"]}\n",
    )
    .unwrap();
}

fn incomplete(result: Result<fathomdb_engine::ExciseReport, EngineError>) {
    assert!(
        matches!(result, Err(EngineError::ErasureIncomplete { ref stage, .. }) if stage == "telemetry_redaction"),
        "original sink obligation must remain owed: {result:?}"
    );
}

#[test]
fn unrelated_sink_cannot_discharge_original_sink_debt() {
    let dir = tempfile::tempdir().unwrap();
    let engine = setup(&dir.path().join("db.sqlite"));
    let original = dir.path().join("original.jsonl");
    let rotated = dir.path().join("rotated.jsonl");
    engine.enable_telemetry(original.to_str().unwrap()).unwrap();
    event(&original);
    std::fs::rename(&original, &rotated).unwrap();
    incomplete(engine.erase_source("target"));
    engine.enable_telemetry(dir.path().join("wrong.jsonl").to_str().unwrap()).unwrap();
    incomplete(engine.erase_source("target"));
    std::fs::rename(&rotated, &original).unwrap();
    engine.enable_telemetry(original.to_str().unwrap()).unwrap();
    engine.erase_source("target").unwrap();
    assert!(!std::fs::read_to_string(original).unwrap().contains("l:subject"));
    engine.close().unwrap();
}

#[test]
fn sequential_original_sink_reattachment_cleans_every_registered_sink() {
    let dir = tempfile::tempdir().unwrap();
    let engine = setup(&dir.path().join("db.sqlite"));
    let first = dir.path().join("first.jsonl");
    let second = dir.path().join("second.jsonl");
    for path in [&first, &second] {
        engine.enable_telemetry(path.to_str().unwrap()).unwrap();
        event(path);
    }
    incomplete(engine.erase_source("target"));
    assert!(!std::fs::read_to_string(&second).unwrap().contains("l:subject"));
    engine.enable_telemetry(first.to_str().unwrap()).unwrap();
    engine.erase_source("target").unwrap();
    assert!(!std::fs::read_to_string(first).unwrap().contains("l:subject"));
    engine.close().unwrap();
}

#[test]
fn malformed_unbound_telemetry_obligation_is_never_silently_discharged() {
    let dir = tempfile::tempdir().unwrap();
    let engine = setup(&dir.path().join("db.sqlite"));
    engine.enable_telemetry(dir.path().join("sink.jsonl").to_str().unwrap()).unwrap();
    engine.maintenance_transaction_for_test(|tx| {
        tx.execute("INSERT INTO operational_mutations(collection_name,record_key,op_kind,payload_json,write_cursor) VALUES('erasure_pending_redaction','purge','append','{\"erased_stable_ids\":\"bad\"}',1)", [])?;
        Ok(())
    }).unwrap();
    incomplete(engine.erase_source("target"));
    let rows = engine.query_i64_col_for_test("SELECT count(*) FROM operational_mutations WHERE collection_name='erasure_pending_redaction'").unwrap();
    assert!(rows[0] > 0);
    engine.close().unwrap();
}

#[test]
fn materialized_search_cannot_append_erased_ids_after_physical_completion() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Arc::new(setup(&dir.path().join("db.sqlite")));
    let sink = dir.path().join("sink.jsonl");
    engine.enable_telemetry(sink.to_str().unwrap()).unwrap();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let child_engine = engine.clone();
    let search = std::thread::spawn(move || {
        fathomdb_engine::arm_explanation_before_telemetry_lock_hook_for_test(Box::new(move || {
            ready_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(15)).unwrap();
        }));
        child_engine.search_explained("privateuniqueterm", None, 0, false, 0.3, 0).unwrap()
    });
    ready_rx.recv_timeout(Duration::from_secs(15)).unwrap();
    let erased = engine.erase_source("target");
    release_tx.send(()).unwrap();
    let result = search.join().unwrap();
    erased.unwrap();
    assert_eq!(result.results.len(), 1, "existing materialized caller result remains valid");
    assert!(
        !std::fs::read_to_string(sink).unwrap().contains("l:subject"),
        "late telemetry must not republish erased identifiers"
    );
    engine.close().unwrap();
}
