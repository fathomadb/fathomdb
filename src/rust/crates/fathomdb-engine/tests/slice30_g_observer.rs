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
            properties: None,
        }])
        .unwrap();
    opened.engine
}

#[test]
fn observation_is_owned_bounded_and_transported_to_the_actual_reader() {
    let directory = tempfile::tempdir().unwrap();
    let a = seed(&directory.path().join("a.sqlite"));
    let b = seed(&directory.path().join("b.sqlite"));
    let guard = a.begin_slice30_observation(32).unwrap();
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
    assert!(matches!(a.begin_slice30_observation(2), Err(EngineError::InvalidArgument { .. })));
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
    assert!(matches!(a.begin_slice30_observation(0), Err(EngineError::InvalidArgument { .. })));
    assert!(matches!(a.begin_slice30_observation(32769), Err(EngineError::InvalidArgument { .. })));
    let bounded = a.begin_slice30_observation(2).unwrap();
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
    assert!(matches!(a.begin_slice30_observation(2), Err(EngineError::Closing)));
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
                entity_class: fathomdb_engine::PropertyEntityClass::Node,
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
            properties: None,
        });
    engine.write(&rows).unwrap();
    engine.drain(5_000).unwrap();
    assert_eq!(engine.read_embedding_readiness().unwrap().pending_count, 0);
    engine
}

#[test]
fn mixed_edge_hybrid_observes_each_actual_guard_once_without_invented_sql() {
    let directory = tempfile::tempdir().unwrap();
    let engine = seed_dense(&directory.path().join("dense.sqlite"));
    engine
        .write(&[PreparedWrite::Edge {
            kind: "link".into(),
            from: "alpha".into(),
            to: "beta".into(),
            body: Some("unrelated relationship".into()),
            source_id: SourceId::new("test:slice30-edge").unwrap(),
            logical_id: Some("alpha-beta".into()),
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
            properties: None,
        }])
        .unwrap();
    engine.drain(5_000).unwrap();
    let observation = engine.begin_slice30_observation(128).unwrap();
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
fn node_only_hybrid_observes_the_replaced_eligibility_guard() {
    let directory = tempfile::tempdir().unwrap();
    let engine = seed_dense(&directory.path().join("node-only.sqlite"));
    let observer = engine.begin_slice30_observation(128).unwrap();
    let result = observer.with_operation(101, || engine.search("needle")).unwrap();
    assert_eq!(result.results[0].id.value, "alpha");
    let batch = observer.take();
    let guard = batch
        .records
        .iter()
        .filter(|record| record.phase == "all_nodes_directly_eligible")
        .collect::<Vec<_>>();
    assert_eq!(guard.len(), 1, "match-local eligibility should account for the skipped scan");
    let record = guard[0];
    assert_eq!(record.operation_id, 101);
    assert_eq!(record.status, "not_run");
    assert_eq!(record.facts["reason"], "match_local_eligibility");
    assert_eq!(record.facts["called"], false);
    assert_eq!(record.facts["sql_executed"], false);
    assert!(record.facts.get("result").is_none());
    assert!(record.started_ns.is_none() && record.ended_ns.is_none());
    assert_eq!(batch.dropped_records, 0);
    assert!(batch.issues.is_empty());
    engine.close().unwrap();
}

#[test]
fn skipped_guards_remain_explicit_for_direct_text_and_filtered_hybrid() {
    let directory = tempfile::tempdir().unwrap();
    let engine = seed_dense(&directory.path().join("guards.sqlite"));
    let observer = engine.begin_slice30_observation(128).unwrap();
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
        properties: None,
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
        properties: None,
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
    let observer = engine.begin_slice30_observation(256).unwrap();
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
                entity_class: fathomdb_engine::PropertyEntityClass::Node,
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
            properties: None,
        }])
        .unwrap()
        .cursor;
    let pending = engine.read_embedding_readiness().unwrap().pending_count;
    let vector_present = engine.has_vector_for_cursor_for_test(cursor).unwrap();
    let observer = engine.begin_slice30_observation(128).unwrap();
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

#[test]
fn authority_observation_uses_reader_transaction_and_frozen_validity() {
    let directory = tempfile::tempdir().unwrap();
    let engine = seed_dense(&directory.path().join("authority.sqlite"));
    let observer = engine.begin_slice30_observation(256).unwrap();
    let capture = |operation: u64, time: i64| {
        let view = fathomdb_engine::ReadView { valid_as_of: Some(time), ..Default::default() };
        let result =
            observer.with_operation(operation, || engine.search_view("needle", &view)).unwrap();
        let batch = observer.take();
        let records = batch
            .records
            .iter()
            .filter(|r| r.phase == "observer_snapshot_capture")
            .collect::<Vec<_>>();
        assert_eq!(records.len(), 1, "same-reader authority capture is missing");
        let record = records[0];
        assert_eq!(record.status, "ok");
        assert_eq!(record.operation_id, operation);
        assert_eq!(record.facts["observer_added"], true);
        assert_eq!(record.facts["effective_at_epoch_s"], time);
        assert_eq!(record.facts["validity_binding_epoch_s"], time);
        assert_eq!(record.facts["projection_cursor"], result.projection_cursor);
        assert!(record.facts["visibility_generation"].as_u64().is_some());
        assert!(record.facts["dependency_generation"].as_u64().is_some());
        assert!(!record.facts["visibility_nonce"].as_str().unwrap().is_empty());
        assert_eq!(batch.dropped_records, 0);
        (record.facts.clone(), result)
    };
    let (first, _) = capture(701, 199);
    let (second, _) = capture(702, 200);
    assert_eq!(first["visibility_generation"], second["visibility_generation"]);
    assert_eq!(first["visibility_nonce"], second["visibility_nonce"]);
    engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "updated needle".into(),
            logical_id: Some("alpha".into()),
            source_id: SourceId::new("test:slice30-dense-unit").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        }])
        .unwrap();
    let (third, result) = capture(703, 200);
    assert_eq!(result.results[0].id.value, "alpha");
    assert_eq!(result.results[0].body, "updated needle");
    assert!(
        third["visibility_generation"].as_u64().unwrap()
            > first["visibility_generation"].as_u64().unwrap()
    );
    assert_ne!(third["visibility_nonce"], first["visibility_nonce"]);
    engine.drain(5_000).unwrap();
    engine.close().unwrap();
}

#[test]
fn boundary_capture_binds_same_call_authority_to_derived_membership() {
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
        properties: None,
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
        properties: None,
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
    let observer = engine.begin_slice30_observation(256).unwrap();
    let mut before: Option<serde_json::Value> = None;
    for (operation, time, present) in [(801, boundary - 1, true), (802, boundary, false)] {
        let view = ReadView { valid_as_of: Some(time), ..ReadView::default() };
        let result = observer
            .with_operation(operation, || engine.search_view("gderivedneedle", &view))
            .unwrap();
        assert_eq!(
            result.results.iter().filter(|hit| hit.id.value == "g-derived").count(),
            usize::from(present)
        );
        let batch = observer.take();
        let captures = batch
            .records
            .iter()
            .filter(|record| record.phase == "observer_snapshot_capture")
            .collect::<Vec<_>>();
        assert_eq!(captures.len(), 1, "same-call boundary authority capture is missing");
        let capture = captures[0];
        assert_eq!(capture.status, "ok");
        assert_eq!(capture.operation_id, operation);
        assert_eq!(capture.facts["effective_at_epoch_s"], time);
        assert_eq!(capture.facts["validity_binding_epoch_s"], time);
        assert_eq!(capture.facts["projection_cursor"], result.projection_cursor);
        assert_eq!(capture.facts["observer_added"], true);
        assert_eq!(batch.dropped_records, 0);
        if let Some(before) = &before {
            for key in ["visibility_generation", "visibility_nonce", "dependency_generation"] {
                assert_eq!(
                    capture.facts[key], before[key],
                    "no write occurred between the boundary queries: {key}"
                );
            }
        } else {
            assert!(capture.facts["visibility_generation"].as_u64().is_some());
            assert!(capture.facts["dependency_generation"].as_u64().is_some());
            assert!(!capture.facts["visibility_nonce"].as_str().unwrap().is_empty());
            before = Some(capture.facts.clone());
        }
    }
    engine.close().unwrap();
}

#[test]
fn api_phases_attribute_actual_compile_dispatch_wait_and_response_per_route() {
    let directory = tempfile::tempdir().unwrap();
    let engine = seed_dense(&directory.path().join("api-phases.sqlite"));
    let observer = engine.begin_slice30_observation(256).unwrap();
    for (operation, api, phases) in [
        (
            901,
            "api_hybrid",
            vec![
                "query_compile",
                "query_embedding_dispatch_wall",
                "query_mean_serialization",
                "reader_wait",
                "final_response",
            ],
        ),
        (902, "api_text", vec!["query_compile", "reader_wait", "final_response"]),
        (903, "api_embed", vec!["query_embedding_dispatch_wall"]),
    ] {
        match operation {
            901 => {
                assert_eq!(
                    observer.with_operation(operation, || engine.search("needle")).unwrap().results
                        [0]
                    .id
                    .value,
                    "alpha"
                );
            }
            902 => {
                assert_eq!(
                    observer
                        .with_operation(operation, || engine.search_text_only("needle"))
                        .unwrap()
                        .results[0]
                        .id
                        .value,
                    "alpha"
                );
            }
            _ => {
                assert_eq!(
                    observer.with_operation(operation, || engine.embed_text("needle")).unwrap(),
                    vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
                );
            }
        }
        let batch = observer.take();
        let api_records =
            batch.records.iter().filter(|record| record.phase == api).collect::<Vec<_>>();
        assert_eq!(api_records.len(), 1, "actual route API interval is missing: {api}");
        let root = api_records[0];
        assert_eq!(root.operation_id, operation);
        assert_eq!(root.status, "ok");
        assert!(root.started_ns.unwrap() <= root.ended_ns.unwrap());
        for phase in phases {
            let records =
                batch.records.iter().filter(|record| record.phase == phase).collect::<Vec<_>>();
            assert_eq!(records.len(), 1, "actual API phase is missing or duplicated: {phase}");
            let record = records[0];
            assert_eq!(record.operation_id, operation);
            assert_eq!(record.parent_span_id, Some(root.span_id));
            assert_eq!(record.status, "ok");
            assert!(root.started_ns.unwrap() <= record.started_ns.unwrap());
            assert!(record.started_ns.unwrap() <= record.ended_ns.unwrap());
            assert!(record.ended_ns.unwrap() <= root.ended_ns.unwrap());
        }
        if operation == 902 {
            assert!(!batch
                .records
                .iter()
                .any(|record| record.phase == "query_embedding_dispatch_wall"));
        }
        assert_eq!(batch.dropped_records, 0);
    }
    engine.close().unwrap();
    let absent = seed(&directory.path().join("absent-api.sqlite"));
    let observer = absent.begin_slice30_observation(32).unwrap();
    assert!(matches!(
        observer.with_operation(904, || absent.embed_text("needle")),
        Err(EngineError::EmbedderNotConfigured)
    ));
    let batch = observer.take();
    for phase in ["api_embed", "query_embedding_dispatch_wall"] {
        let records =
            batch.records.iter().filter(|record| record.phase == phase).collect::<Vec<_>>();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].status, "error");
        assert!(records[0].started_ns.is_some());
        assert!(records[0].ended_ns.is_some());
    }
    assert_eq!(batch.dropped_records, 0);
    absent.close().unwrap();
}

#[test]
fn reader_phases_preserve_combined_vector_sql_fts_fusion_and_identity_parentage() {
    let directory = tempfile::tempdir().unwrap();
    let engine = seed_dense(&directory.path().join("reader-phases.sqlite"));
    let observer = engine.begin_slice30_observation(256).unwrap();
    let result = observer.with_operation(1001, || engine.search("needle")).unwrap();
    assert_eq!(result.results.len(), 2);
    assert_eq!(result.results[0].id.value, "alpha");
    let batch = observer.take();
    let unique = |phase| {
        let records =
            batch.records.iter().filter(|record| record.phase == phase).collect::<Vec<_>>();
        assert_eq!(records.len(), 1, "actual reader phase is missing or duplicated: {phase}");
        records[0]
    };
    let reader = unique("reader_transaction");
    let vector = unique("vector_retrieval");
    for (phase, parent, count) in [
        ("vector_retrieval", reader.span_id, 2),
        ("vector_phase1_f32_sql", vector.span_id, 2),
        ("vector_hydration", vector.span_id, 2),
        ("node_fts_candidates", reader.span_id, 1),
        ("edge_fts_candidates", reader.span_id, 0),
        ("fusion_final_ranking", reader.span_id, 2),
    ] {
        let record = unique(phase);
        assert_eq!(record.parent_span_id, Some(parent));
        assert_eq!(record.status, "ok");
        assert_eq!(record.operation_id, 1001);
        assert_eq!(record.facts["candidates"], count);
        assert!(reader.started_ns.unwrap() <= record.started_ns.unwrap());
        assert!(record.started_ns.unwrap() <= record.ended_ns.unwrap());
        assert!(record.ended_ns.unwrap() <= reader.ended_ns.unwrap());
    }
    let identity = unique("deferred_identity_hydration");
    assert_eq!(identity.status, "ok");
    assert_eq!(identity.parent_span_id, Some(reader.span_id));
    assert_eq!(batch.dropped_records, 0);
    observer.with_operation(1002, || engine.search_text_only("needle")).unwrap();
    let batch = observer.take();
    for phase in ["vector_retrieval", "deferred_identity_hydration"] {
        let records =
            batch.records.iter().filter(|record| record.phase == phase).collect::<Vec<_>>();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].status, "not_run");
        assert_eq!(records[0].started_ns, None);
        assert_eq!(records[0].ended_ns, None);
        assert_eq!(records[0].facts["called"], false);
    }
    assert!(!batch.records.iter().any(
        |record| record.phase == "vector_phase1_f32_sql" || record.phase == "vector_hydration"
    ));
    assert_eq!(batch.dropped_records, 0);
    engine.close().unwrap();
}

struct ScalarHead {
    known: bool,
}

impl fathomdb_embedder_api::Embedder for ScalarHead {
    fn identity(&self) -> fathomdb_embedder_api::EmbedderIdentity {
        fathomdb_embedder_api::EmbedderIdentity::new(
            "slice30-scalar-head-control",
            if self.known { "known" } else { "unknown" },
            8,
        )
    }

    fn embed(
        &self,
        input: &str,
    ) -> Result<fathomdb_embedder_api::Vector, fathomdb_embedder_api::EmbedderError> {
        let mut vector = vec![0.0; 8];
        vector[input.chars().next().unwrap() as usize % 8] = 1.0;
        Ok(vector)
    }

    fn truncation_offset(
        &self,
        input: &str,
    ) -> Result<Option<usize>, fathomdb_embedder_api::EmbedderError> {
        Ok(self.known.then(|| input.chars().next().unwrap().len_utf8()))
    }
}

#[test]
fn stored_span_witness_preserves_utf8_byte_ranges_and_unknown_coverage() {
    for known in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let store = directory.path().join("coverage.sqlite");
        let engine =
            Engine::open_with_embedder_for_test(&store, std::sync::Arc::new(ScalarHead { known }))
                .unwrap()
                .engine;
        engine
            .configure_projections(
                &[fathomdb_engine::ProjectionSpec {
                    name: "doc".into(),
                    roles: [fathomdb_engine::ProjectionRole::Searchable].into_iter().collect(),
                    fts: None,
                    vector: Some(fathomdb_engine::ProjectionVector::default()),
                    source: None,
                    entity_class: fathomdb_engine::PropertyEntityClass::Node,
                }],
                &[],
            )
            .unwrap();
        let writes = [("ascii", "alpha"), ("mixed", "βeta雪"), ("full", "雪")].map(|(id, body)| {
            PreparedWrite::Node {
                kind: "doc".into(),
                body: body.into(),
                logical_id: Some(id.into()),
                source_id: SourceId::new("test:slice30-span-coverage").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            }
        });
        engine.write(&writes).unwrap();
        engine.drain(5_000).unwrap();
        assert_eq!(engine.read_embedding_readiness().unwrap().pending_count, 0);
        engine.close().unwrap();
        drop(engine);
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..");
        let witness = std::process::Command::new(root.join(".venv/bin/python"))
            .arg(root.join("dev/experiments/0.8.29-slice30-measurements/g_span_coverage.py"))
            .arg(&store)
            .output()
            .unwrap();
        assert!(witness.status.success(), "{}", String::from_utf8_lossy(&witness.stderr));
        let report: serde_json::Value = serde_json::from_slice(&witness.stdout).unwrap();
        assert_eq!(report["span_count"], 3);
        let ends = if known { [Some(1), Some(2), Some(3)] } else { [None; 3] };
        let spans: Vec<_> = ends
            .into_iter()
            .zip([5, 8, 3])
            .map(|(end, length)| {
                serde_json::json!({"window_number": 0, "byte_start": 0,
                "byte_end": end, "body_length": length})
            })
            .collect();
        assert_eq!(
            report["spans"],
            serde_json::json!(spans),
            "actual stored ranges must distinguish UTF-8 bytes from scalar counts"
        );
        assert_eq!(report["full_body_spans"], if known { 1 } else { 0 });
        assert_eq!(report["partial_body_spans"], if known { 2 } else { 0 });
        assert_eq!(report["unknown_coverage_spans"], if known { 0 } else { 3 });
        assert_eq!(report["body_bytes"], 16);
        assert_eq!(
            report["known_covered_bytes"],
            if known { serde_json::json!(6) } else { serde_json::Value::Null }
        );
    }
}
