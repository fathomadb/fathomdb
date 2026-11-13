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

#[test]
fn source_window_controls_observe_actual_fallback_selection_without_clock_changes() {
    use fathomdb_engine::{
        ArtifactRevisionId, CanonicalHash, ProvenancedNodeV1, ReadView,
        SourceDependencyRegistrationV1, SourceLocator, SourceRevisionId, SourceVersionId,
        WriteProvenanceV1,
    };
    use sha2::{Digest, Sha256};
    let directory = tempfile::tempdir().unwrap();
    let engine = seed_dense(&directory.path().join("source-window.sqlite"));
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        as i64;
    // Provenance admission needs a currently eligible source; explicit future
    // query instants cross its half-open boundary without changing the host clock.
    let boundary = now + 3600;
    let source_body = "source body";
    let source = PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: source_body.into(),
        logical_id: Some("g-source".into()),
        source_id: SourceId::new("test:slice30-provenance").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: Some(now - 1),
        valid_until: Some(boundary),
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new("g-source-r1").unwrap(),
            SourceVersionId::new("v1").unwrap(),
        ),
    });
    let hash = Sha256::digest(source_body.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let derived = PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: "gderivedneedle".into(),
        logical_id: Some("g-derived".into()),
        source_id: SourceId::new("test:slice30-provenance").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new("g-derived-r1").unwrap(),
            SourceVersionId::new("v1").unwrap(),
            SourceRevisionId::new("g-source-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(hash).unwrap(),
        ),
    });
    engine.write(&[source, derived]).unwrap();
    engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("g-dependency", "g-source-r1", "g-derived-r1")
                .unwrap(),
        )
        .unwrap();
    engine.drain(5_000).unwrap();
    let observer = engine.begin_slice30_observation_for_test(256).unwrap();
    for (operation, time, expected_fallback) in [(301, boundary - 1, false), (302, boundary, true)]
    {
        let view = ReadView { valid_as_of: Some(time), ..ReadView::default() };
        let result = observer
            .with_operation(operation, || engine.search_view("gderivedneedle", &view))
            .unwrap();
        assert_eq!(
            result.results.iter().any(|hit| hit.id.value == "g-derived"),
            !expected_fallback
        );
        let batch = observer.take();
        let dependencies = batch
            .records
            .iter()
            .filter(|r| r.phase == "has_source_dependencies")
            .collect::<Vec<_>>();
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies[0].facts["result"], true);
        let fallback = batch
            .records
            .iter()
            .filter(|r| r.phase == "vector_arm_requires_fallback")
            .collect::<Vec<_>>();
        assert_eq!(fallback.len(), 1);
        assert_eq!(fallback[0].facts["result"], expected_fallback);
        assert_eq!(fallback[0].facts["sql_executed"], true);
        let selection = batch
            .records
            .iter()
            .filter(|r| r.phase == "source_fallback_selection")
            .collect::<Vec<_>>();
        assert_eq!(selection.len(), 1, "the actual fallback selection interval is missing");
        assert_eq!(selection[0].facts["result"], expected_fallback);
        assert_eq!(selection[0].status, "ok");
        assert_eq!(batch.dropped_records, 0);
    }
    engine.close().unwrap();
}

#[test]
fn pending_vector_probe_records_the_selected_fallback_not_only_eligibility() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("pending-probe.sqlite");
    let engine = Engine::open_with_embedder_for_test(&path, std::sync::Arc::new(FixedVector))
        .unwrap()
        .engine;
    engine.set_projection_scheduler_frozen_for_test(true);
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
    engine.drain(5_000).unwrap();
    let cursor = engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "pending needle".into(),
            logical_id: Some("pending".into()),
            source_id: SourceId::new("test:slice30-pending-probe").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap()
        .cursor;
    let pending = engine.read_embedding_readiness().unwrap().pending_count;
    let vector_present = engine.has_vector_for_cursor_for_test(cursor).unwrap();
    let observer = engine.begin_slice30_observation_for_test(128).unwrap();
    let result = observer.with_operation(601, || engine.search("needle")).unwrap();
    let batch = observer.take();
    engine.set_projection_scheduler_frozen_for_test(false);
    engine.drain(5_000).unwrap();
    engine.close().unwrap();
    assert_eq!(pending, 1);
    assert!(!vector_present);
    assert_eq!(result.results[0].id.value, "pending");
    assert_eq!(
        result.soft_fallback.as_ref().unwrap().branch,
        fathomdb_engine::SoftFallbackBranch::Vector
    );
    let eligibility =
        batch.records.iter().find(|r| r.phase == "vector_arm_requires_fallback").unwrap();
    assert_eq!(eligibility.facts["result"], false);
    let selection = batch.records.iter().find(|r| r.phase == "source_fallback_selection").unwrap();
    assert_eq!(selection.facts["sql_executed"], true);
    assert_eq!(
        selection.facts["result"], true,
        "the actual pending-row probe selected Some(Vector)"
    );
    assert_eq!(batch.dropped_records, 0);
}
