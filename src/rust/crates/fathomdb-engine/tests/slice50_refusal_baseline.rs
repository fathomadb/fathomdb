//! Current-behavior characterization only; no D50-1/D50-2 implementation.
//! Run the ignored matrix as documented in dev/experiments/slice50-refusal-baseline/README.md.
#![cfg(all(feature = "test-hooks", feature = "operator"))]

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ArtifactRevisionId, Engine, EngineError, InitialState,
    LifecycleActuationV1, LifecycleState, PreparedWrite, ProvenancedNodeV1, SourceId,
    SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::{types::ValueRef, Connection};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};
use tempfile::TempDir;

#[derive(Debug)]
struct Controlled {
    identity: EmbedderIdentity,
    divergent: bool,
    calls: Arc<AtomicUsize>,
}
impl Embedder for Controlled {
    fn identity(&self) -> EmbedderIdentity {
        self.identity.clone()
    }
    fn embed(&self, _: &str) -> Result<Vector, EmbedderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut vector = vec![0.0; self.identity.dimension as usize];
        vector[usize::from(self.divergent)] = 1.0;
        Ok(vector)
    }
}
fn runtime(identity: &EmbedderIdentity, divergent: bool) -> Arc<Controlled> {
    Arc::new(Controlled {
        identity: identity.clone(),
        divergent,
        calls: Arc::new(AtomicUsize::new(0)),
    })
}
fn node(kind: &str, logical: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: kind.into(),
        body: "baseline characterization body".into(),
        source_id: SourceId::new("baseline-source").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(format!("{logical}-r1")).unwrap(),
            SourceVersionId::new(format!("{logical}-v1")).unwrap(),
        ),
    })
}
fn edge() -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "relates_to".into(),
        from: "a".into(),
        to: "b".into(),
        source_id: SourceId::new("baseline-source").unwrap(),
        logical_id: Some("pending-edge".into()),
        body: Some("pending edge body".into()),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}
fn snapshot(path: &Path) -> Value {
    let conn = Connection::open(path).unwrap();
    let names: Vec<String> = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap()
        .query_map([], |row| row.get(0)).unwrap().collect::<Result<_,_>>().unwrap();
    let mut result = serde_json::Map::new();
    for name in names {
        let mut stmt =
            conn.prepare(&format!("SELECT * FROM \"{}\"", name.replace('"', "\"\""))).unwrap();
        let cols = stmt.column_count();
        let mut rows = stmt.query([]).unwrap();
        let mut values = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            let mut cells = Vec::new();
            for col in 0..cols {
                let value = match row.get_ref(col).unwrap() {
                    ValueRef::Null => "null".to_string(),
                    ValueRef::Integer(v) => format!("i:{v}"),
                    ValueRef::Real(v) => format!("f:{:016x}", v.to_bits()),
                    ValueRef::Text(v) => format!("t:{}", String::from_utf8_lossy(v)),
                    ValueRef::Blob(v) => format!("b:{v:?}"),
                };
                cells.push(value);
            }
            values.push(serde_json::to_string(&cells).unwrap());
        }
        values.sort();
        let digest: String = Sha256::digest(serde_json::to_vec(&values).unwrap())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        result.insert(name, json!({"rows": values.len(), "sha256": digest}));
    }
    Value::Object(result)
}
fn classify<T>(result: &Result<T, EngineError>) -> &'static str {
    match result {
        Ok(_) => "Ok",
        Err(EngineError::Scheduler) => "Scheduler",
        Err(EngineError::EmbedderRequired(_)) => "EmbedderRequired",
        Err(_) => "OtherError",
    }
}
fn invoke(engine: &Engine, operation: &str) -> Result<(), EngineError> {
    match operation {
        "drain" => engine.drain(3000),
        "transition" => engine.transition("target", LifecycleState::Deleted, None),
        "configure_projections" => {
            engine.configure_projections(&engine.read_projections()?, &[]).map(|_| ())
        }
        "actuate" => engine
            .actuate(
                ActuationBatchV1::new(
                    "baseline-delete",
                    vec![ActuationOperationV1::TransitionLifecycle(
                        LifecycleActuationV1::new(
                            "target",
                            ArtifactRevisionId::new("target-r1").unwrap(),
                            LifecycleState::Deleted,
                            None,
                        )
                        .unwrap(),
                    )],
                )
                .unwrap(),
            )
            .map(|receipt| {
                assert!(
                    receipt.outcome == fathomdb_engine::ActuationOutcomeV1::Committed,
                    "{receipt:?}"
                );
            }),
        "purge" => engine.purge("target"),
        "erase_source" => engine.erase_source("baseline-source").map(|_| ()),
        "excise_source" => engine.excise_source("baseline-source").map(|_| ()),
        "rebuild_projections" => engine.rebuild_projections().map(|_| ()),
        "rebuild_vec0" => engine.rebuild_vec0().map(|_| ()),
        _ => panic!("unknown operation"),
    }
}
fn case(operation: &str, state: &str, repeat: usize) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("baseline{SQLITE_SUFFIX}"));
    Engine::open(&path).unwrap().engine.close().unwrap();
    let conn = Connection::open(&path).unwrap();
    let identity = conn.query_row("SELECT name,revision,dimension FROM _fathomdb_embedder_profiles WHERE profile='default'", [],
        |row| Ok(EmbedderIdentity::new(row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get(2)?))).unwrap();
    drop(conn);
    let setup = Engine::open_with_embedder_for_test(&path, runtime(&identity, false)).unwrap();
    setup.engine.configure_vector_kind_for_test("doc").unwrap();
    setup.engine.write(&[node("doc", "anchor")]).unwrap();
    setup.engine.drain(5000).unwrap();
    setup.engine.close().unwrap();
    let baseline = Engine::open_with_embedder_for_test(&path, runtime(&identity, false)).unwrap();
    baseline.engine.close().unwrap();
    let pending = state != "refused_empty";
    let seed = Engine::open(&path).unwrap();
    seed.engine.write(&[node("note", "target")]).unwrap();
    if operation == "purge" {
        seed.engine.transition("target", LifecycleState::Deleted, None).unwrap();
    }
    if pending {
        seed.engine.write(&[edge()]).unwrap();
    }
    seed.engine.close().unwrap();
    let invalidate = state.starts_with("refused") || state == "accepted_ready";
    if invalidate {
        Connection::open(&path).unwrap().execute("UPDATE _fathomdb_open_state SET value='baseline-force-verification' WHERE key='vector_equivalence_verified_fingerprint'", []).unwrap();
    }
    let probe_runtime = runtime(&identity, state != "accepted_ready");
    let open_start = Instant::now();
    let result = if state == "absent_pending" {
        Engine::open(&path)
    } else {
        Engine::open_with_embedder_for_test(&path, probe_runtime.clone())
    };
    // Historical refused-engine measurements remain in committed data. Under
    // the successful-open policy these cases terminate at admission instead.
    if state.starts_with("refused") {
        let error = result.err().expect("divergent provider must fail open");
        assert!(matches!(
            error,
            fathomdb_engine::EngineOpenError::VectorEquivalenceMismatch { .. }
        ));
        println!(
            "REFUSAL_ADMISSION {}",
            json!({"operation": operation, "state": state,
            "repeat":repeat,"open_ms":open_start.elapsed().as_secs_f64()*1000.0,
            "error":format!("{error:?}"),"operation_invoked":false})
        );
        let recovered =
            Engine::open_with_embedder_for_test(&path, runtime(&identity, false)).unwrap();
        recovered.engine.drain(5000).unwrap();
        assert_eq!(recovered.engine.read_embedding_readiness().unwrap().pending_count, 0);
        recovered.engine.close().unwrap();
        return;
    }
    let opened = result.unwrap();
    let open_ms = open_start.elapsed().as_secs_f64() * 1000.0;
    let open_calls = probe_runtime.calls.load(Ordering::SeqCst);
    // Accepted control settles its seed before the measured call; it does not test rebuild admission races.
    if state == "accepted_ready" {
        opened.engine.drain(5000).unwrap();
    }
    let ready_before = opened.engine.read_embedding_readiness().unwrap();
    if state.starts_with("refused") || state == "absent_pending" {
        assert_eq!(ready_before.pending_count, u64::from(pending));
    }
    let before = snapshot(&path);
    let calls_before = probe_runtime.calls.load(Ordering::SeqCst);
    let started = Instant::now();
    let result = invoke(&opened.engine, operation);
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    let ready_after = opened.engine.read_embedding_readiness().unwrap();
    let after = snapshot(&path);
    let expected = if state == "refused_pending" {
        "Scheduler"
    } else if state == "absent_pending"
        && (operation == "drain" || operation.starts_with("rebuild"))
    {
        "EmbedderRequired"
    } else {
        "Ok"
    };
    println!(
        "REFUSAL_BASELINE {}",
        json!({"operation": operation,"state": state,"repeat": repeat,
        "open_ms": open_ms,"embed_calls_at_open_return": open_calls,"open_report": format!("{:?}",opened.report),
        "elapsed_ms": elapsed_ms,"result":classify(&result),"result_debug":format!("{result:?}"),
        "readiness_before":format!("{ready_before:?}"),"readiness_after":format!("{ready_after:?}"),
        "pending_before":ready_before.pending_count,"pending_after":ready_after.pending_count,
        "embed_calls_during_operation":probe_runtime.calls.load(Ordering::SeqCst)-calls_before,
        "unchanged":before==after,"tables_before":before,"tables_after":after})
    );
    assert_eq!(classify(&result), expected, "{operation} {state}: {result:?}");
    if result.is_err() {
        assert_eq!(before, after, "failed operation mutated durable state");
        assert_eq!(ready_before.pending_count, ready_after.pending_count);
    }
    if state == "refused_pending" {
        assert!(elapsed_ms >= if operation == "drain" { 2900.0 } else { 29000.0 });
        assert_eq!(
            probe_runtime.calls.load(Ordering::SeqCst),
            calls_before,
            "refused runtime must not embed"
        );
    }
    opened.engine.close().unwrap();
    // Every case must remain openable and drainable with the original accepted backend.
    let recovered = Engine::open_with_embedder_for_test(&path, runtime(&identity, false)).unwrap();
    recovered.engine.drain(5000).unwrap();
    let recovered_ready = recovered.engine.read_embedding_readiness().unwrap();
    assert_eq!(recovered_ready.pending_count, 0);
    println!(
        "REFUSAL_RECOVERY {}",
        json!({"operation":operation,"state":state,"repeat":repeat,
        "pending":recovered_ready.pending_count,"provider_accepted":true})
    );
    recovered.engine.close().unwrap();
}
fn matrix(operation: &str) {
    let repeats =
        std::env::var("REFUSAL_REPEATS").unwrap_or_else(|_| "3".into()).parse::<usize>().unwrap();
    assert!(repeats > 0);
    for repeat in 0..repeats {
        for state in ["refused_pending", "refused_empty", "absent_pending", "accepted_ready"] {
            case(operation, state, repeat);
        }
    }
}
macro_rules! baseline {
    ($($name:ident),*) => {$ (
        #[test]
        #[ignore = "30-second lifecycle barriers; explicit characterization only"]
        fn $name() { matrix(stringify!($name)); }
    )*};
}
baseline!(
    drain,
    transition,
    configure_projections,
    actuate,
    purge,
    erase_source,
    excise_source,
    rebuild_projections,
    rebuild_vec0
);
