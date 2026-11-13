//! Slice30 engine/session/operation ownership on real stores; no model workload.
use fathomdb_engine::{Engine, EngineError, InitialState, PreparedWrite, SourceId};
use std::panic::{catch_unwind, AssertUnwindSafe};

fn seed(path: &std::path::Path) -> Engine {
    let opened = Engine::open(path).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "literal needle".into(),
            logical_id: Some("alpha".into()),
            source_id: SourceId::new("test:slice30-owner").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    opened.engine
}

#[test]
fn observation_is_owned_bounded_and_transported_to_the_actual_reader() {
    let directory = tempfile::tempdir().unwrap();
    let a = seed(&directory.path().join("a.sqlite"));
    let b = seed(&directory.path().join("b.sqlite"));
    let guard = a.begin_slice30_observation_for_test(32).unwrap();
    guard.with_operation(41, || {
        assert_eq!(a.search_text_only("needle").unwrap().results[0].id.value, "alpha")
    });
    guard.with_operation(42, || {
        assert_eq!(b.search_text_only("needle").unwrap().results[0].id.value, "alpha")
    });
    let first = guard.take();
    assert!(
        first.records.iter().any(|r| r.phase == "api_text"),
        "actual engine A API interval is missing"
    );
    assert!(
        first.records.iter().any(|r| r.phase == "reader_transaction"),
        "reader must receive the actual operation token"
    );
    assert!(first.records.iter().all(|r| r.operation_id == 41));
    assert_eq!(first.dropped_records, 0);
    assert!(first.session_sequence > 0);
    assert!(matches!(
        a.begin_slice30_observation_for_test(2),
        Err(EngineError::InvalidArgument { .. })
    ));
    assert!(catch_unwind(AssertUnwindSafe(|| guard.with_operation(7, || {
        a.search_text_only("needle").unwrap();
        panic!("contained observation unwind");
    })))
    .is_err());
    a.search_text_only("needle").unwrap();
    guard.with_operation(8, || {
        a.search_text_only("needle").unwrap();
    });
    let after = guard.take();
    assert!(after.records.iter().any(|r| r.operation_id == 7));
    assert!(after.records.iter().any(|r| r.operation_id == 8));
    assert!(after.records.iter().all(|r| r.operation_id == 7 || r.operation_id == 8));
    drop(guard);
    assert!(matches!(
        a.begin_slice30_observation_for_test(0),
        Err(EngineError::InvalidArgument { .. })
    ));
    assert!(matches!(
        a.begin_slice30_observation_for_test(32769),
        Err(EngineError::InvalidArgument { .. })
    ));
    let bounded = a.begin_slice30_observation_for_test(2).unwrap();
    for id in 0..5 {
        bounded.with_operation(id, || {
            a.search_text_only("needle").unwrap();
        });
    }
    let batch = bounded.take();
    assert_eq!(batch.records.len(), 2);
    assert!(batch.dropped_records > 0);
    a.close().unwrap();
    drop(bounded);
    assert!(matches!(a.begin_slice30_observation_for_test(2), Err(EngineError::Closing)));
    b.close().unwrap();
}

struct FixedVector;
impl fathomdb_embedder_api::Embedder for FixedVector {
    fn identity(&self) -> fathomdb_embedder_api::EmbedderIdentity {
        fathomdb_embedder_api::EmbedderIdentity::new("slice30-observer-unit", "v1", 8)
    }
    fn embed(
        &self,
        _text: &str,
    ) -> Result<fathomdb_embedder_api::Vector, fathomdb_embedder_api::EmbedderError> {
        Ok(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    }
}

fn seed_dense(path: &std::path::Path) -> Engine {
    let engine =
        Engine::open_with_embedder_for_test(path, std::sync::Arc::new(FixedVector)).unwrap().engine;
    engine
        .configure_projections(
            &[fathomdb_engine::ProjectionSpec {
                name: "doc".into(),
                roles: [fathomdb_engine::ProjectionRole::Searchable].into_iter().collect(),
                fts: None,
                vector: Some(fathomdb_engine::ProjectionVector::default()),
                source: None,
            }],
            &[],
        )
        .unwrap();
    let rows =
        [("alpha", "needle alpha"), ("beta", "beta")].map(|(id, body)| PreparedWrite::Node {
            kind: "doc".into(),
            body: body.into(),
            logical_id: Some(id.into()),
            source_id: SourceId::new("test:slice30-dense-unit").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        });
    engine.write(&rows).unwrap();
    engine.drain(5_000).unwrap();
    assert_eq!(engine.read_embedding_readiness().unwrap().pending_count, 0);
    engine
}

#[test]
fn no_dependency_hybrid_observes_each_actual_guard_once_without_invented_sql() {
    let directory = tempfile::tempdir().unwrap();
    let engine = seed_dense(&directory.path().join("dense.sqlite"));
    let observation = engine.begin_slice30_observation_for_test(128).unwrap();
    let result = observation.with_operation(100, || engine.search("needle")).unwrap();
    assert_eq!(result.results[0].id.value, "alpha");
    let batch = observation.take();
    let phase =
        |name| batch.records.iter().filter(|record| record.phase == name).collect::<Vec<_>>();
    let dependencies = phase("has_source_dependencies");
    assert_eq!(dependencies.len(), 1, "one actual dependency EXISTS invocation must be observed");
    assert_eq!(dependencies[0].facts["result"], false);
    assert_eq!(dependencies[0].facts["sql_executed"], true);
    let fallback = phase("vector_arm_requires_fallback");
    assert_eq!(fallback.len(), 1);
    assert_eq!(fallback[0].facts["result"], false);
    assert_eq!(
        fallback[0].facts["sql_executed"], false,
        "no-dependency early return executes no fallback SQL"
    );
    let direct = phase("all_nodes_directly_eligible");
    assert_eq!(direct.len(), 1);
    assert_eq!(direct[0].facts["result"], true);
    assert_eq!(direct[0].facts["sql_executed"], true);
    assert!(dependencies
        .iter()
        .chain(&fallback)
        .chain(&direct)
        .all(|record| record.status == "ok" && record.operation_id == 100));
    assert_eq!(batch.dropped_records, 0);
    engine.close().unwrap();
}

#[test]
fn skipped_guards_remain_explicit_for_direct_text_and_filtered_hybrid() {
    let directory = tempfile::tempdir().unwrap();
    let engine = seed_dense(&directory.path().join("guards.sqlite"));
    let observer = engine.begin_slice30_observation_for_test(128).unwrap();
    observer.with_operation(201, || {
        assert_eq!(engine.search_text_only("needle").unwrap().results[0].id.value, "alpha");
    });
    let direct = observer.take();
    for phase in ["all_nodes_directly_eligible", "vector_arm_requires_fallback"] {
        let skipped =
            direct.records.iter().filter(|record| record.phase == phase).collect::<Vec<_>>();
        assert_eq!(skipped.len(), 1, "the direct-text skipped guard is missing: {phase}");
        assert_eq!(skipped[0].status, "not_run");
        assert_eq!(skipped[0].facts["reason"], "vector_unavailable");
        assert_eq!(skipped[0].facts["called"], false);
        assert!(skipped[0].started_ns.is_none() && skipped[0].ended_ns.is_none());
    }
    let mut filter = fathomdb_engine::SearchFilter::default();
    filter.kind = Some("doc".into());
    observer.with_operation(202, || {
        assert_eq!(
            engine.search_filtered_with_limit("needle", Some(filter), 10).unwrap().results[0]
                .id
                .value,
            "alpha"
        );
    });
    let filtered = observer.take();
    let skipped = filtered
        .records
        .iter()
        .filter(|record| record.phase == "all_nodes_directly_eligible")
        .collect::<Vec<_>>();
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].status, "not_run");
    assert_eq!(skipped[0].facts["reason"], "filter");
    assert_eq!(skipped[0].facts["called"], false);
    assert!(skipped[0].started_ns.is_none() && skipped[0].ended_ns.is_none());
    assert_eq!(filtered.dropped_records, 0);
    engine.close().unwrap();
}
