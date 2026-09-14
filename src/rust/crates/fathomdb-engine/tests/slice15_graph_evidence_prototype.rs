//! Transient Slice 15 RED oracles for the graph-evidence decision spike.

#![cfg(feature = "test-hooks")]

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    arm_erasure_before_primary_lock_hook_for_test,
    arm_evidence_before_resolve_return_hook_for_test, encode_graph_expand_result_v1,
    ActuationBatchV1, ActuationOperationV1, ArtifactRevisionId, CanonicalHash, Engine,
    EvidenceErrorReasonV1, EvidenceRefV1, GraphArtifactClassForTest, GraphExpandRequestV1,
    GraphReadContextV1, GraphSeedV1, IdSpace, InitialState, PreparedWrite, ProjectionRole,
    ProjectionSpec, ProjectionVector, ProvenancedEdgeV1, ProvenancedNodeV1, ReadContextV1,
    ReadView, SearchFilter, SourceDependencyRegistrationV1, SourceId, SourceLocator,
    SourceRevisionId, SourceVersionId, TraversalDirection, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use std::collections::BTreeSet;
use std::sync::{Arc, Barrier, Mutex, MutexGuard};
use std::thread;

static FIXTURE_SERIALIZATION: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug)]
struct EligibilityEmbedder;

impl Embedder for EligibilityEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice15-filter", "r1", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    }
}

fn serialize_fixture() -> MutexGuard<'static, ()> {
    FIXTURE_SERIALIZATION.lock().unwrap_or_else(|error| error.into_inner())
}

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn register_dependencies(engine: &Engine, operation: &str, revisions: &[String]) {
    let operations = revisions
        .iter()
        .map(|revision| {
            ActuationOperationV1::RegisterSourceDependency(
                SourceDependencyRegistrationV1::new(
                    format!("dependency-{revision}"),
                    "source-r1",
                    revision,
                )
                .unwrap(),
            )
        })
        .collect();
    engine.actuate(ActuationBatchV1::new(operation, operations).unwrap()).unwrap();
}

fn fixture_with_source(source: &str) -> (TempDir, Engine, GraphExpandRequestV1) {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join(format!("slice15{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let provenance = |revision: &str| {
        WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new("source-v1").unwrap(),
            SourceRevisionId::new("source-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest(source)).unwrap(),
        )
    };
    opened
        .engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("source".into()),
                kind: "document".into(),
                body: source.into(),
                source_id: SourceId::new("source-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("source-r1").unwrap(),
                    SourceVersionId::new("source-v1").unwrap(),
                ),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("root".into()),
                kind: "claim".into(),
                body: "root".into(),
                source_id: SourceId::new("source-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: provenance("root-r1"),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("target".into()),
                kind: "claim".into(),
                body: "target".into(),
                source_id: SourceId::new("source-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: provenance("target-r1"),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("z-later".into()),
                kind: "supports".into(),
                from: "root".into(),
                to: "target".into(),
                source_id: SourceId::new("source-owner").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.7),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: provenance("edge-later-r1"),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("a-winner".into()),
                kind: "supports".into(),
                from: "root".into(),
                to: "target".into(),
                source_id: SourceId::new("source-owner").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.9),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: provenance("edge-winner-r1"),
            }),
        ])
        .unwrap();
    register_dependencies(
        &opened.engine,
        "slice15-base-dependencies",
        &["target-r1".to_string(), "edge-later-r1".to_string(), "edge-winner-r1".to_string()],
    );
    opened.engine.drain(30_000).unwrap();
    let mut filter = SearchFilter::default();
    filter.kind = Some("claim".into());
    let context = ReadContextV1::new(
        ReadView { valid_as_of: Some(1_800_000_000), ..ReadView::default() },
        filter,
    )
    .unwrap();
    let frozen = opened.engine.freeze_read_context(&context).unwrap();
    let request = GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("root")],
        },
        direction: TraversalDirection::Outgoing,
        edge_kinds: vec!["supports".into()],
        target_kinds: vec!["claim".into()],
        context: GraphReadContextV1::Frozen { schema_version: 1, context: frozen },
        max_depth: 1,
        result_limit: 50,
        max_work_units: 10_000,
        include_explanation: false,
    };
    (directory, opened.engine, request)
}

fn fixture() -> (TempDir, Engine, GraphExpandRequestV1) {
    fixture_with_source("canonical source bytes")
}

fn fixture_1k() -> (TempDir, Engine, GraphExpandRequestV1) {
    fixture_with_source(&"x".repeat(1_024))
}

fn eligibility_fixture() -> (TempDir, Engine, ReadView) {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join(format!("eligibility{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(EligibilityEmbedder)).unwrap();
    opened
        .engine
        .configure_projections(
            &[ProjectionSpec {
                name: "owner".into(),
                roles: BTreeSet::from([ProjectionRole::Filterable, ProjectionRole::Searchable]),
                fts: None,
                vector: Some(ProjectionVector::default()),
                source: None,
            }],
            &[],
        )
        .unwrap();
    let source = r#"{"owner":"alice","text":"canonical source"}"#;
    let provenance = |revision: &str| {
        WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new("filter-v1").unwrap(),
            SourceRevisionId::new("filter-source-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest(source)).unwrap(),
        )
    };
    opened
        .engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("filter-source".into()),
                kind: "note".into(),
                body: source.into(),
                source_id: SourceId::new("filter-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("filter-source-r1").unwrap(),
                    SourceVersionId::new("filter-v1").unwrap(),
                ),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("filter-root".into()),
                kind: "note".into(),
                body: r#"{"owner":"alice","text":"root"}"#.into(),
                source_id: SourceId::new("filter-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: provenance("filter-root-r1"),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("filter-target".into()),
                kind: "note".into(),
                body: r#"{"owner":"alice","text":"target"}"#.into(),
                source_id: SourceId::new("filter-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: provenance("filter-target-r1"),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("filter-edge".into()),
                kind: "supports".into(),
                from: "filter-root".into(),
                to: "filter-target".into(),
                source_id: SourceId::new("filter-owner").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.8),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: provenance("filter-edge-r1"),
            }),
        ])
        .unwrap();
    register_dependencies(
        &opened.engine,
        "slice15-filter-dependencies",
        &["filter-target-r1".into(), "filter-edge-r1".into()],
    );
    opened.engine.drain(30_000).unwrap();
    (directory, opened.engine, ReadView { valid_as_of: Some(1_800_000_000), ..ReadView::default() })
}

fn eligibility_request(
    engine: &Engine,
    view: &ReadView,
    filter: SearchFilter,
) -> GraphExpandRequestV1 {
    let context = ReadContextV1::new(*view, filter).unwrap();
    GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("filter-root")],
        },
        direction: TraversalDirection::Outgoing,
        edge_kinds: vec!["supports".into()],
        target_kinds: vec!["note".into()],
        context: GraphReadContextV1::Frozen {
            schema_version: 1,
            context: engine.freeze_read_context(&context).unwrap(),
        },
        max_depth: 1,
        result_limit: 50,
        max_work_units: 10_000,
        include_explanation: false,
    }
}

fn unavailable(error: fathomdb_engine::EngineError) {
    assert!(matches!(error, fathomdb_engine::EngineError::Evidence(ref value)
        if value.reason == EvidenceErrorReasonV1::EvidenceUnavailable
            && value.field_path == "/evidenceRef"));
}

fn fixture_many(count: usize) -> (TempDir, Engine, GraphExpandRequestV1) {
    let (directory, engine, mut request) = fixture();
    let source = "canonical source bytes";
    let mut writes = Vec::new();
    let mut revisions = Vec::new();
    for index in 1..count {
        let provenance = |revision: String| {
            WriteProvenanceV1::derived(
                ArtifactRevisionId::new(revision).unwrap(),
                SourceVersionId::new("source-v1").unwrap(),
                SourceRevisionId::new("source-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest(source)).unwrap(),
            )
        };
        writes.push(PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            logical_id: Some(format!("target-{index:02}")),
            kind: "claim".into(),
            body: format!("target {index}"),
            source_id: SourceId::new("source-owner").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: provenance(format!("target-{index:02}-r1")),
        }));
        revisions.push(format!("target-{index:02}-r1"));
        writes.push(PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
            logical_id: Some(format!("edge-{index:02}")),
            kind: "supports".into(),
            from: "root".into(),
            to: format!("target-{index:02}"),
            source_id: SourceId::new("source-owner").unwrap(),
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: Some(0.8),
            extractor_model_id: None,
            temporal_fallback: None,
            provenance: provenance(format!("edge-{index:02}-r1")),
        }));
        revisions.push(format!("edge-{index:02}-r1"));
    }
    engine.write(&writes).unwrap();
    register_dependencies(&engine, "slice15-many-dependencies", &revisions);
    engine.drain(30_000).unwrap();
    let context = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.context.clone(),
        _ => unreachable!(),
    };
    request.context = GraphReadContextV1::Frozen {
        schema_version: 1,
        context: engine.freeze_read_context(&context).unwrap(),
    };
    (directory, engine, request)
}

fn fixture_max_work() -> (TempDir, Engine, GraphExpandRequestV1) {
    let (directory, engine, mut request) = fixture_many(50);
    let mut writes = Vec::with_capacity(9_949);
    for index in 0..9_949 {
        writes.push(PreparedWrite::Edge {
            logical_id: Some(format!("noise-edge-{index:04}")),
            kind: "noise".into(),
            from: "root".into(),
            to: format!("missing-noise-target-{index:04}"),
            source_id: SourceId::new("noise-owner").unwrap(),
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
        });
    }
    engine.write(&writes).unwrap();
    engine.drain(30_000).unwrap();
    let context = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.context.clone(),
        _ => unreachable!(),
    };
    request.context = GraphReadContextV1::Frozen {
        schema_version: 1,
        context: engine.freeze_read_context(&context).unwrap(),
    };
    (directory, engine, request)
}

#[test]
fn treatment_off_preserves_literal_bytes_and_winning_parallel_edge() {
    let _serial = serialize_fixture();
    let (_directory, engine, request) = fixture();
    let ordinary = engine.graph_expand(&request).unwrap();
    let bytes = encode_graph_expand_result_v1(&ordinary).unwrap();
    assert_eq!(bytes, br#"{"schemaVersion":1,"seeds":[{"schemaVersion":1,"logicalId":"root","seedOrdinal":0,"queryScore":null}],"targets":[{"schemaVersion":1,"logicalId":"target","kind":"claim","body":"target","writeCursor":"3","origin":{"schemaVersion":1,"seedLogicalId":"root","seedOrdinal":0,"predecessorLogicalId":"root","targetLogicalId":"target","hopCount":1,"terminalEdgeKind":"supports","terminalDirection":"outgoing"}}],"complete":true,"workUnits":"2","degradationCodes":[],"explanation":null}"#);
    let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    assert_eq!(treated.graph, ordinary);
    assert_eq!(treated.evidence.len(), 1);
    assert_eq!(treated.evidence[0].target_revision_id, "target-r1");
    assert_eq!(treated.evidence[0].terminal_edge_revision_id.as_deref(), Some("edge-winner-r1"));
}

#[test]
fn authenticated_target_and_edge_refs_resolve_intrinsic_evidence() {
    let _serial = serialize_fixture();
    let (_directory, engine, request) = fixture();
    let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    let frozen = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context,
        _ => unreachable!(),
    };
    let target =
        engine.resolve_graph_evidence_for_test(&treated.evidence[0].target_ref, frozen).unwrap();
    let edge = engine
        .resolve_graph_evidence_for_test(
            treated.evidence[0].terminal_edge_ref.as_ref().unwrap(),
            frozen,
        )
        .unwrap();
    assert_eq!(target.artifact_class, GraphArtifactClassForTest::Node);
    assert_eq!(target.artifact_revision_id, "target-r1");
    assert_eq!(edge.artifact_class, GraphArtifactClassForTest::Edge);
    assert_eq!(edge.artifact_revision_id, "edge-winner-r1");
    assert_eq!(target.canonical_source_body, "canonical source bytes");
    assert_eq!(edge.canonical_source_body, "canonical source bytes");
    assert!(treated.evidence[0].target_ref.as_str().starts_with("fdgi1."));
    assert!(!treated.evidence[0].target_ref.as_str().contains("fde1."));
    assert_eq!(target.artifact_kind, "claim");
    assert_eq!(target.artifact_body.as_deref(), Some("target"));
    assert_eq!(target.source_id, "source-owner");
    assert_eq!(target.source_version_id, "source-v1");
    assert_eq!(target.source_revision_id, "source-r1");
    assert_eq!(target.canonical_source_hash, digest("canonical source bytes"));
    assert_eq!(target.source_locator, SourceLocator::WholeBody);
    assert_eq!(target.canonical_source_span, (0, 22));
    assert_eq!(target.node_state.as_deref(), Some("active"));
    assert!(target.edge_t_valid.is_none());
    assert_eq!(target.source_state, "active");
    let dependency = target.dependency.as_ref().unwrap();
    assert_eq!(dependency.source_revision_id.as_str(), "source-r1");
    assert_eq!(dependency.derived_revision_id.as_str(), "target-r1");
    assert!(dependency.registered_dependency_generation > 0);
    assert_eq!(edge.artifact_kind, "supports");
    assert_eq!(edge.edge_from.as_deref(), Some("root"));
    assert_eq!(edge.edge_to.as_deref(), Some("target"));
    assert_eq!(edge.edge_direction, Some(TraversalDirection::Outgoing));
    assert!(edge.node_state.is_none());
    assert!(edge.edge_t_valid.is_none());
    assert!(!edge.edge_temporal_fallback);
    assert_eq!(edge.dependency.as_ref().unwrap().derived_revision_id.as_str(), "edge-winner-r1");
}

#[test]
fn target_filters_govern_nodes_but_do_not_reject_disclosed_terminal_edges() {
    let _serial = serialize_fixture();
    let (_directory, engine, view) = eligibility_fixture();
    let mut source_type = SearchFilter::default();
    source_type.source_type = Some("note".into());
    let mut status = SearchFilter::default();
    status.status = Some(String::new());
    let mut created_after = SearchFilter::default();
    created_after.created_after = Some(0);
    let mut attributes = SearchFilter::default();
    attributes.attributes = vec![("owner".into(), "alice".into())];
    let filters = [
        ("source_type", source_type),
        ("status", status),
        ("created_after", created_after),
        ("attributes", attributes),
    ];
    for (name, filter) in filters {
        let request = eligibility_request(&engine, &view, filter);
        let treated = engine
            .graph_expand_with_graph_evidence_for_test(&request)
            .unwrap_or_else(|error| panic!("{name} eligibility failed: {error:?}"));
        assert_eq!(treated.graph.targets[0].logical_id, "filter-target");
        let frozen = match &request.context {
            GraphReadContextV1::Frozen { context, .. } => context,
            _ => unreachable!(),
        };
        engine.resolve_graph_evidence_for_test(&treated.evidence[0].target_ref, frozen).unwrap();
        engine
            .resolve_graph_evidence_for_test(
                treated.evidence[0].terminal_edge_ref.as_ref().unwrap(),
                frozen,
            )
            .unwrap();
    }
}

#[test]
fn current_context_and_incomplete_provenance_refuse_atomically() {
    let _serial = serialize_fixture();
    let (_directory, engine, mut request) = fixture();
    let context = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.context.clone(),
        _ => unreachable!(),
    };
    request.context = GraphReadContextV1::Current { schema_version: 1, context };
    assert!(engine.graph_expand_with_graph_evidence_for_test(&request).is_err());
}

#[test]
fn preflight_is_two_indexed_class_specific_statements() {
    let _serial = serialize_fixture();
    let (_directory, engine, request) = fixture();
    let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    assert_eq!(treated.preflight_data_statement_count, 2);
    assert_eq!(treated.preflight_node_rows, 1);
    assert_eq!(treated.preflight_edge_rows, 1);
    assert_eq!(treated.preflight_source_hash_count, 1);
    assert_eq!(treated.preflight_source_bytes_hashed, "canonical source bytes".len());
    let plans = engine.explain_graph_evidence_preflights_for_test().unwrap();
    assert_eq!(plans.len(), 2);
    for plan in plans {
        assert!(plan.contains("_fathomdb_artifact_revisions"));
        assert!(plan.contains("INDEX"), "unindexed preflight: {plan}");
    }
}

#[test]
fn one_kib_point_fixture_really_contains_1024_source_bytes() {
    let _serial = serialize_fixture();
    let (_directory, engine, request) = fixture_1k();
    let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    let frozen = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context,
        _ => unreachable!(),
    };
    let target =
        engine.resolve_graph_evidence_for_test(&treated.evidence[0].target_ref, frozen).unwrap();
    assert_eq!(target.canonical_source_body.len(), 1_024);
}

#[test]
fn restart_tamper_and_incomplete_provenance_are_fail_closed() {
    let _serial = serialize_fixture();
    let (directory, engine, request) = fixture();
    let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    let frozen = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.clone(),
        _ => unreachable!(),
    };
    let reference = treated.evidence[0].target_ref.clone();
    engine.close().unwrap();
    let path = directory.path().join(format!("slice15{SQLITE_SUFFIX}"));
    let reopened = Engine::open(&path).unwrap().engine;
    assert_eq!(
        reopened.resolve_graph_evidence_for_test(&reference, &frozen).unwrap().artifact_revision_id,
        "target-r1"
    );
    let mut tampered = reference.as_str().as_bytes().to_vec();
    *tampered.last_mut().unwrap() = if tampered.last() == Some(&b'0') { b'1' } else { b'0' };
    let tampered = EvidenceRefV1::new(String::from_utf8(tampered).unwrap()).unwrap();
    let error = reopened.resolve_graph_evidence_for_test(&tampered, &frozen).unwrap_err();
    assert!(matches!(error, fathomdb_engine::EngineError::Evidence(ref value)
        if value.reason == EvidenceErrorReasonV1::EvidenceUnavailable));
    reopened.close().unwrap();
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE _fathomdb_artifact_revisions SET completeness='migrated_incomplete' \
         WHERE revision_id='target-r1'",
            [],
        )
        .unwrap();
    let incomplete = Engine::open(&path).unwrap().engine;
    let context = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.context.clone(),
        _ => unreachable!(),
    };
    let mut incomplete_request = request;
    incomplete_request.context = GraphReadContextV1::Frozen {
        schema_version: 1,
        context: incomplete.freeze_read_context(&context).unwrap(),
    };
    assert!(incomplete.graph_expand_with_graph_evidence_for_test(&incomplete_request).is_err());
}

#[test]
fn corrupt_hash_and_locator_refuse_the_entire_treatment() {
    let _serial = serialize_fixture();
    for (name, sql) in [
        (
            "hash",
            "UPDATE _fathomdb_source_links SET hash_digest=\
             'ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff' \
             WHERE artifact_revision_id='target-r1'",
        ),
        (
            "locator",
            "UPDATE _fathomdb_source_links SET locator_kind='utf8_bytes',start_byte=999,end_byte=1000 \
             WHERE artifact_revision_id='target-r1'",
        ),
    ] {
        let (directory, engine, mut request) = fixture();
        engine.close().unwrap();
        let path = directory.path().join(format!("slice15{SQLITE_SUFFIX}"));
        rusqlite::Connection::open(&path).unwrap().execute(sql, []).unwrap();
        let reopened = Engine::open(&path).unwrap().engine;
        let context = match &request.context {
            GraphReadContextV1::Frozen { context, .. } => context.context.clone(),
            _ => unreachable!(),
        };
        request.context = GraphReadContextV1::Frozen {
            schema_version: 1,
            context: reopened.freeze_read_context(&context).unwrap(),
        };
        let error = reopened.graph_expand_with_graph_evidence_for_test(&request).unwrap_err();
        assert!(matches!(error, fathomdb_engine::EngineError::Evidence(_)), "{name}: {error:?}");
    }
}

#[test]
fn context_mismatch_foreign_context_and_post_erasure_share_nondisclosure() {
    let _serial = serialize_fixture();
    let (_directory, engine, request) = fixture();
    let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    let reference = treated.evidence[0].target_ref.clone();
    let context = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.context.clone(),
        _ => unreachable!(),
    };
    let mut different_filter = SearchFilter::default();
    different_filter.kind = Some("document".into());
    let mismatch = engine
        .freeze_read_context(&ReadContextV1::new(context.view, different_filter).unwrap())
        .unwrap();
    unavailable(engine.resolve_graph_evidence_for_test(&reference, &mismatch).unwrap_err());

    let foreign_directory = TempDir::new().unwrap();
    let foreign = Engine::open(foreign_directory.path().join(format!("foreign{SQLITE_SUFFIX}")))
        .unwrap()
        .engine;
    let foreign_context = foreign.freeze_read_context(&context).unwrap();
    unavailable(engine.resolve_graph_evidence_for_test(&reference, &foreign_context).unwrap_err());

    engine.erase_source("source-owner").unwrap();
    unavailable(
        engine
            .resolve_graph_evidence_for_test(
                &reference,
                &match &request.context {
                    GraphReadContextV1::Frozen { context, .. } => context.clone(),
                    _ => unreachable!(),
                },
            )
            .unwrap_err(),
    );
}

#[test]
fn held_wal_reader_preserves_typed_erasure_incomplete() {
    let _serial = serialize_fixture();
    let (directory, engine, request) = fixture();
    let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    let reference = treated.evidence[0].target_ref.clone();
    let frozen = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.clone(),
        _ => unreachable!(),
    };
    let path = directory.path().join(format!("slice15{SQLITE_SUFFIX}"));
    let holder = rusqlite::Connection::open(&path).unwrap();
    holder.execute_batch("BEGIN; SELECT COUNT(*) FROM canonical_nodes;").unwrap();
    let result = engine.erase_source("source-owner");
    holder.execute_batch("ROLLBACK").unwrap();
    match result {
        Err(fathomdb_engine::EngineError::ErasureIncomplete { stage, .. }) => {
            assert_eq!(stage, "wal_checkpoint");
        }
        other => panic!("expected typed held-WAL refusal, got {other:?}"),
    }
    unavailable(engine.resolve_graph_evidence_for_test(&reference, &frozen).unwrap_err());
}

#[test]
fn resolver_bytes_are_released_before_erasure_can_complete() {
    let _serial = serialize_fixture();
    let (_directory, engine, request) = fixture();
    let engine = Arc::new(engine);
    let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    let frozen = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.clone(),
        _ => unreachable!(),
    };
    let reference = treated.evidence[0].target_ref.clone();
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let hook_entered = Arc::clone(&entered);
    let hook_release = Arc::clone(&release);
    arm_evidence_before_resolve_return_hook_for_test(Box::new(move || {
        hook_entered.wait();
        hook_release.wait();
    }));
    let resolver_engine = Arc::clone(&engine);
    let resolver =
        thread::spawn(move || resolver_engine.resolve_graph_evidence_for_test(&reference, &frozen));
    entered.wait();
    let (finished_send, finished_receive) = std::sync::mpsc::sync_channel(1);
    let (attempted_send, attempted_receive) = std::sync::mpsc::sync_channel(1);
    arm_erasure_before_primary_lock_hook_for_test(Box::new(move || {
        attempted_send.send(()).unwrap();
    }));
    let eraser_engine = Arc::clone(&engine);
    let eraser = thread::spawn(move || {
        let result = eraser_engine.erase_source("source-owner");
        finished_send.send(()).unwrap();
        result
    });
    attempted_receive.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
    assert!(finished_receive.recv_timeout(std::time::Duration::from_millis(50)).is_err());
    release.wait();
    assert_eq!(resolver.join().unwrap().unwrap().canonical_source_body, "canonical source bytes");
    let _ = eraser.join().unwrap();
    finished_receive.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
}

#[test]
#[ignore = "Slice 15 controlled release-mode measurement"]
fn measurement_matrix_emits_raw_samples() {
    let _serial = serialize_fixture();
    use std::time::Instant;

    let (_directory, engine, request) = fixture();
    let engine = Arc::new(engine);
    for _ in 0..50 {
        engine.graph_expand(&request).unwrap();
        engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    }
    let mut control_us = Vec::with_capacity(1_000);
    let mut hydrated_us = Vec::with_capacity(1_000);
    for index in 0..1_000 {
        if index % 2 == 0 {
            let start = Instant::now();
            engine.graph_expand(&request).unwrap();
            control_us.push(start.elapsed().as_nanos() as u64 / 1_000);
            let start = Instant::now();
            engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
            hydrated_us.push(start.elapsed().as_nanos() as u64 / 1_000);
        } else {
            let start = Instant::now();
            engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
            hydrated_us.push(start.elapsed().as_nanos() as u64 / 1_000);
            let start = Instant::now();
            engine.graph_expand(&request).unwrap();
            control_us.push(start.elapsed().as_nanos() as u64 / 1_000);
        }
    }
    let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    let (_point_directory, point_engine, point_request) = fixture_1k();
    let point_engine = Arc::new(point_engine);
    let point_treated =
        point_engine.graph_expand_with_graph_evidence_for_test(&point_request).unwrap();
    let point_frozen = match &point_request.context {
        GraphReadContextV1::Frozen { context, .. } => context,
        _ => unreachable!(),
    };
    let mut point_node_us = Vec::with_capacity(1_000);
    let mut point_edge_us = Vec::with_capacity(1_000);
    for _ in 0..1_000 {
        let start = Instant::now();
        point_engine
            .resolve_graph_evidence_for_test(&point_treated.evidence[0].target_ref, point_frozen)
            .unwrap();
        point_node_us.push(start.elapsed().as_nanos() as u64 / 1_000);
        let start = Instant::now();
        point_engine
            .resolve_graph_evidence_for_test(
                point_treated.evidence[0].terminal_edge_ref.as_ref().unwrap(),
                point_frozen,
            )
            .unwrap();
        point_edge_us.push(start.elapsed().as_nanos() as u64 / 1_000);
    }
    let mut concurrent_control_us = Vec::with_capacity(1_600);
    let mut concurrent_hydrated_us = Vec::with_capacity(1_600);
    for hydrated in [false, true] {
        let barrier = Arc::new(Barrier::new(9));
        let mut workers = Vec::new();
        for _ in 0..8 {
            let engine = Arc::clone(&engine);
            let request = request.clone();
            let barrier = Arc::clone(&barrier);
            workers.push(thread::spawn(move || {
                barrier.wait();
                (0..200)
                    .map(|_| {
                        let start = Instant::now();
                        if hydrated {
                            engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
                        } else {
                            engine.graph_expand(&request).unwrap();
                        }
                        start.elapsed().as_nanos() as u64 / 1_000
                    })
                    .collect::<Vec<_>>()
            }));
        }
        barrier.wait();
        let destination =
            if hydrated { &mut concurrent_hydrated_us } else { &mut concurrent_control_us };
        for worker in workers {
            destination.extend(worker.join().unwrap());
        }
    }
    let mut concurrent_point_us = Vec::with_capacity(1_600);
    let barrier = Arc::new(Barrier::new(9));
    let mut workers = Vec::new();
    for _ in 0..8 {
        let engine = Arc::clone(&point_engine);
        let frozen = point_frozen.clone();
        let reference = point_treated.evidence[0].target_ref.clone();
        let barrier = Arc::clone(&barrier);
        workers.push(thread::spawn(move || {
            barrier.wait();
            (0..200)
                .map(|_| {
                    let start = Instant::now();
                    engine.resolve_graph_evidence_for_test(&reference, &frozen).unwrap();
                    start.elapsed().as_nanos() as u64 / 1_000
                })
                .collect::<Vec<_>>()
        }));
    }
    barrier.wait();
    for worker in workers {
        concurrent_point_us.extend(worker.join().unwrap());
    }
    let (_many_directory, many, many_request) = fixture_many(50);
    for _ in 0..20 {
        many.graph_expand(&many_request).unwrap();
        many.graph_expand_with_graph_evidence_for_test(&many_request).unwrap();
    }
    let mut control_50_us = Vec::with_capacity(1_000);
    let mut hydrated_50_us = Vec::with_capacity(1_000);
    for index in 0..1_000 {
        let control_first = index % 2 == 0;
        let measure_control = || {
            let started = Instant::now();
            many.graph_expand(&many_request).unwrap();
            started.elapsed().as_nanos() as u64 / 1_000
        };
        let measure_hydrated = || {
            let started = Instant::now();
            many.graph_expand_with_graph_evidence_for_test(&many_request).unwrap();
            started.elapsed().as_nanos() as u64 / 1_000
        };
        if control_first {
            control_50_us.push(measure_control());
            hydrated_50_us.push(measure_hydrated());
        } else {
            hydrated_50_us.push(measure_hydrated());
            control_50_us.push(measure_control());
        }
    }
    let many_treated = many.graph_expand_with_graph_evidence_for_test(&many_request).unwrap();
    let many_frozen = match &many_request.context {
        GraphReadContextV1::Frozen { context, .. } => context,
        _ => unreachable!(),
    };
    let mut memex_batch_50_us = Vec::with_capacity(30);
    for _ in 0..30 {
        let started = Instant::now();
        for entry in many_treated.evidence.iter().take(25) {
            many.resolve_graph_evidence_for_test(&entry.target_ref, many_frozen).unwrap();
            many.resolve_graph_evidence_for_test(
                entry.terminal_edge_ref.as_ref().unwrap(),
                many_frozen,
            )
            .unwrap();
        }
        memex_batch_50_us.push(started.elapsed().as_nanos() as u64 / 1_000);
    }
    let mut concurrent_control_50_us = Vec::with_capacity(1_600);
    let mut concurrent_hydrated_50_us = Vec::with_capacity(1_600);
    let many = Arc::new(many);
    for hydrated in [false, true] {
        let barrier = Arc::new(Barrier::new(9));
        let mut workers = Vec::new();
        for _ in 0..8 {
            let engine = Arc::clone(&many);
            let request = many_request.clone();
            let barrier = Arc::clone(&barrier);
            workers.push(thread::spawn(move || {
                barrier.wait();
                (0..200)
                    .map(|_| {
                        let started = Instant::now();
                        if hydrated {
                            engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
                        } else {
                            engine.graph_expand(&request).unwrap();
                        }
                        started.elapsed().as_nanos() as u64 / 1_000
                    })
                    .collect::<Vec<_>>()
            }));
        }
        barrier.wait();
        let destination =
            if hydrated { &mut concurrent_hydrated_50_us } else { &mut concurrent_control_50_us };
        for worker in workers {
            destination.extend(worker.join().unwrap());
        }
    }

    let (_max_directory, max_engine, max_request) = fixture_max_work();
    assert_eq!(max_engine.graph_expand(&max_request).unwrap().work_units, 10_000);
    let mut control_10k_us = Vec::with_capacity(30);
    let mut hydrated_10k_us = Vec::with_capacity(30);
    for index in 0..30 {
        let measure_control = || {
            let started = Instant::now();
            let result = max_engine.graph_expand(&max_request).unwrap();
            assert_eq!(result.work_units, 10_000);
            started.elapsed().as_nanos() as u64 / 1_000
        };
        let measure_hydrated = || {
            let started = Instant::now();
            let result =
                max_engine.graph_expand_with_graph_evidence_for_test(&max_request).unwrap();
            assert_eq!(result.graph.work_units, 10_000);
            started.elapsed().as_nanos() as u64 / 1_000
        };
        if index % 2 == 0 {
            control_10k_us.push(measure_control());
            hydrated_10k_us.push(measure_hydrated());
        } else {
            hydrated_10k_us.push(measure_hydrated());
            control_10k_us.push(measure_control());
        }
    }

    let large_source = "x".repeat(100 * 1_024);
    let (_large_directory, large, large_request) = fixture_with_source(&large_source);
    let large_treated = large.graph_expand_with_graph_evidence_for_test(&large_request).unwrap();
    let large_frozen = match &large_request.context {
        GraphReadContextV1::Frozen { context, .. } => context,
        _ => unreachable!(),
    };
    let mut point_node_100k_us = Vec::with_capacity(100);
    let mut point_edge_100k_us = Vec::with_capacity(100);
    for index in 0..100 {
        let (first, second) = if index % 2 == 0 {
            (
                large_treated.evidence[0].target_ref.clone(),
                large_treated.evidence[0].terminal_edge_ref.clone().unwrap(),
            )
        } else {
            (
                large_treated.evidence[0].terminal_edge_ref.clone().unwrap(),
                large_treated.evidence[0].target_ref.clone(),
            )
        };
        let started = Instant::now();
        large.resolve_graph_evidence_for_test(&first, large_frozen).unwrap();
        let first_us = started.elapsed().as_nanos() as u64 / 1_000;
        let started = Instant::now();
        large.resolve_graph_evidence_for_test(&second, large_frozen).unwrap();
        let second_us = started.elapsed().as_nanos() as u64 / 1_000;
        if index % 2 == 0 {
            point_node_100k_us.push(first_us);
            point_edge_100k_us.push(second_us);
        } else {
            point_edge_100k_us.push(first_us);
            point_node_100k_us.push(second_us);
        }
    }

    let sidecar_1_bytes = serde_json::to_vec(&serde_json::json!({
        "graph": serde_json::from_slice::<serde_json::Value>(&encode_graph_expand_result_v1(&treated.graph).unwrap()).unwrap(),
        "evidence": treated.evidence.iter().map(|entry| serde_json::json!({
            "targetRevisionId": entry.target_revision_id,
            "targetEvidenceRef": entry.target_ref.as_str(),
            "terminalEdgeRevisionId": entry.terminal_edge_revision_id,
            "terminalEdgeEvidenceRef": entry.terminal_edge_ref.as_ref().map(EvidenceRefV1::as_str),
        })).collect::<Vec<_>>(),
    })).unwrap().len();
    let sidecar_50_bytes = serde_json::to_vec(&serde_json::json!({
        "graph": serde_json::from_slice::<serde_json::Value>(&encode_graph_expand_result_v1(&many_treated.graph).unwrap()).unwrap(),
        "evidence": many_treated.evidence.iter().map(|entry| serde_json::json!({
            "targetRevisionId": entry.target_revision_id,
            "targetEvidenceRef": entry.target_ref.as_str(),
            "terminalEdgeRevisionId": entry.terminal_edge_revision_id,
            "terminalEdgeEvidenceRef": entry.terminal_edge_ref.as_ref().map(EvidenceRefV1::as_str),
        })).collect::<Vec<_>>(),
    })).unwrap().len();
    let encode_inline =
        |graph: &fathomdb_engine::GraphExpandResultV1,
         evidence: &[fathomdb_engine::GraphEvidenceEntryForTest]| {
            let mut value = serde_json::from_slice::<serde_json::Value>(
                &encode_graph_expand_result_v1(graph).unwrap(),
            )
            .unwrap();
            for (target, entry) in value["targets"].as_array_mut().unwrap().iter_mut().zip(evidence)
            {
                let target = target.as_object_mut().unwrap();
                target.insert(
                    "targetRevisionId".into(),
                    serde_json::Value::String(entry.target_revision_id.clone()),
                );
                target.insert(
                    "targetEvidenceRef".into(),
                    serde_json::Value::String(entry.target_ref.as_str().into()),
                );
                target.insert(
                    "terminalEdgeRevisionId".into(),
                    entry
                        .terminal_edge_revision_id
                        .as_ref()
                        .map_or(serde_json::Value::Null, |value| {
                            serde_json::Value::String(value.clone())
                        }),
                );
                target.insert(
                    "terminalEdgeEvidenceRef".into(),
                    entry.terminal_edge_ref.as_ref().map_or(serde_json::Value::Null, |value| {
                        serde_json::Value::String(value.as_str().into())
                    }),
                );
            }
            serde_json::to_vec(&value).unwrap().len()
        };
    let inline_1_bytes = encode_inline(&treated.graph, &treated.evidence);
    let inline_50_bytes = encode_inline(&many_treated.graph, &many_treated.evidence);
    let preflight_plans = engine.explain_graph_evidence_preflights_for_test().unwrap();
    println!(
        "SLICE15_RAW={}",
        serde_json::json!({
            "unit": "microseconds", "control_1": control_us, "hydrated_1": hydrated_us,
            "point_node_1k": point_node_us, "point_edge_1k": point_edge_us,
            "canonical_source_1k_bytes": 1_024,
            "concurrent_control_8x200": concurrent_control_us,
            "concurrent_hydrated_8x200": concurrent_hydrated_us,
            "concurrent_point_8x200": concurrent_point_us,
            "control_50": control_50_us, "hydrated_50": hydrated_50_us,
            "concurrent_control_50_8x200": concurrent_control_50_us,
            "concurrent_hydrated_50_8x200": concurrent_hydrated_50_us,
            "control_10k_work": control_10k_us, "hydrated_10k_work": hydrated_10k_us,
            "memex_batch_50": memex_batch_50_us,
            "point_node_100k": point_node_100k_us, "point_edge_100k": point_edge_100k_us,
            "canonical_source_100k_bytes": large_source.len(),
            "control_response_50_bytes": encode_graph_expand_result_v1(&many_treated.graph).unwrap().len(),
            "control_response_bytes": encode_graph_expand_result_v1(&treated.graph).unwrap().len(),
            "sidecar_1_bytes": sidecar_1_bytes, "sidecar_50_bytes": sidecar_50_bytes,
            "inline_1_bytes": inline_1_bytes, "inline_50_bytes": inline_50_bytes,
            "preflight_plans": preflight_plans,
            "preflight_one": {
                "data_statement_count": treated.preflight_data_statement_count,
                "node_rows": treated.preflight_node_rows,
                "edge_rows": treated.preflight_edge_rows,
                "source_hash_count": treated.preflight_source_hash_count,
                "source_bytes_hashed": treated.preflight_source_bytes_hashed,
            },
            "preflight_fifty": {
                "data_statement_count": many_treated.preflight_data_statement_count,
                "node_rows": many_treated.preflight_node_rows,
                "edge_rows": many_treated.preflight_edge_rows,
                "source_hash_count": many_treated.preflight_source_hash_count,
                "source_bytes_hashed": many_treated.preflight_source_bytes_hashed,
            },
            "sidecar_reference_bytes": treated.evidence[0].target_ref.as_str().len()
                + treated.evidence[0].terminal_edge_ref.as_ref().unwrap().as_str().len()
                + treated.evidence[0].target_revision_id.len()
                + treated.evidence[0].terminal_edge_revision_id.as_ref().unwrap().len(),
        })
    );
}

struct ForegroundWriterSample {
    elapsed: std::time::Duration,
    foreground_operations: usize,
    latencies_us: Vec<u64>,
}

fn run_foreground_writer_window(
    engine: &Engine,
    campaign: usize,
    mode: &str,
    duration: std::time::Duration,
) -> ForegroundWriterSample {
    let mut latencies_us = Vec::new();
    let started = std::time::Instant::now();
    let mut index = 0_usize;
    while started.elapsed() < duration {
        let write_started = std::time::Instant::now();
        engine
            .write(&[PreparedWrite::Node {
                logical_id: Some(format!("writer-{campaign}-{mode}-{index}")),
                kind: "noise".into(),
                body: "writer interference payload".into(),
                source_id: SourceId::new("writer-campaign").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .unwrap();
        latencies_us.push(write_started.elapsed().as_nanos() as u64 / 1_000);
        index += 1;
        thread::sleep(std::time::Duration::from_millis(2));
    }
    ForegroundWriterSample {
        elapsed: started.elapsed(),
        foreground_operations: index,
        latencies_us,
    }
}

#[test]
fn foreground_writer_window_is_fixed_duration_and_load_independent() {
    let _serial = serialize_fixture();
    let (_directory, engine, _request) = fixture();
    let duration = std::time::Duration::from_millis(25);

    let sample = run_foreground_writer_window(&engine, 0, "control", duration);

    assert!(sample.elapsed >= duration);
    assert!(sample.foreground_operations > 0);
    assert_eq!(sample.latencies_us.len(), sample.foreground_operations);
}

#[test]
#[ignore = "Slice 15 writer-interference campaigns"]
fn writer_interference_emits_campaigns() {
    let _serial = serialize_fixture();
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    let mut output = Vec::new();
    for campaign in 0..5 {
        let order = match campaign % 3 {
            0 => ["alone", "graph", "point"],
            1 => ["graph", "point", "alone"],
            _ => ["point", "alone", "graph"],
        };
        for mode in order {
            let (_directory, engine, request) = fixture();
            let engine = Arc::new(engine);
            let running = Arc::new(AtomicBool::new(true));
            let completed = Arc::new(AtomicU64::new(0));
            let (background, ready) = if mode == "alone" {
                (None, None)
            } else {
                let engine = Arc::clone(&engine);
                let running = Arc::clone(&running);
                let template = request.clone();
                let completed = Arc::clone(&completed);
                let (ready_send, ready_receive) = std::sync::mpsc::sync_channel(1);
                let worker = thread::spawn(move || {
                    let context = match &template.context {
                        GraphReadContextV1::Frozen { context, .. } => context.context.clone(),
                        _ => unreachable!(),
                    };
                    let mut successes = 0_u64;
                    let mut announced = false;
                    while running.load(Ordering::Acquire) {
                        let Ok(frozen) = engine.freeze_read_context(&context) else { continue };
                        let mut request = template.clone();
                        request.context = GraphReadContextV1::Frozen {
                            schema_version: 1,
                            context: frozen.clone(),
                        };
                        if let Ok(treated) =
                            engine.graph_expand_with_graph_evidence_for_test(&request)
                        {
                            let operation_completed = mode != "point"
                                || engine
                                    .resolve_graph_evidence_for_test(
                                        &treated.evidence[0].target_ref,
                                        &frozen,
                                    )
                                    .is_ok();
                            if operation_completed {
                                successes += 1;
                                completed.fetch_add(1, Ordering::Release);
                                if !announced {
                                    ready_send.send(()).unwrap();
                                    announced = true;
                                }
                            }
                        }
                    }
                    successes
                });
                (Some(worker), Some(ready_receive))
            };
            if let Some(ready) = ready {
                ready.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
            }
            let completed_before_timing = completed.load(Ordering::Acquire);
            let sample = run_foreground_writer_window(
                &engine,
                campaign,
                mode,
                std::time::Duration::from_secs(1),
            );
            let completed_after_timing = completed.load(Ordering::Acquire);
            running.store(false, Ordering::Release);
            let _total_successful_background_ops =
                background.map_or(0, |background| background.join().unwrap());
            let successful_background_ops = completed_after_timing - completed_before_timing;
            assert!(
                mode == "alone" || successful_background_ops >= 20,
                "campaign {campaign} {mode} completed only {successful_background_ops} timed background operations"
            );
            output.push(serde_json::json!({
                "campaign": campaign, "mode": mode,
                "elapsed_us": sample.elapsed.as_micros() as u64,
                "foreground_operations": sample.foreground_operations,
                "throughput_per_s": sample.foreground_operations as f64 / sample.elapsed.as_secs_f64(),
                "latencies_us": sample.latencies_us,
                "successful_background_ops": successful_background_ops,
            }));
        }
    }
    println!("SLICE15_WRITER={}", serde_json::Value::Array(output));
}

#[cfg(target_os = "linux")]
fn peak_rss_bytes() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|value| value.split_whitespace().next())
        .unwrap()
        .parse::<u64>()
        .unwrap()
        * 1_024
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "Slice 15 process-isolated RSS campaigns"]
fn isolated_rss_emits_campaigns() {
    let _serial = serialize_fixture();
    const CHILD: &str = "FATHOMDB_SLICE15_RSS_CHILD";
    const PREFIX: &str = "SLICE15_RSS_CHILD=";
    if let Ok(mode) = std::env::var(CHILD) {
        let (_directory, engine, request) = fixture_many(50);
        let baseline = peak_rss_bytes();
        if mode == "treatment" {
            engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
        } else {
            engine.graph_expand(&request).unwrap();
        }
        println!("{PREFIX}{}", peak_rss_bytes().saturating_sub(baseline));
        return;
    }

    let executable = std::env::current_exe().unwrap();
    let mut samples = Vec::new();
    for campaign in 0..5 {
        let order =
            if campaign % 2 == 0 { ["control", "treatment"] } else { ["treatment", "control"] };
        for mode in order {
            let output = std::process::Command::new(&executable)
                .arg("--exact")
                .arg("isolated_rss_emits_campaigns")
                .arg("--ignored")
                .arg("--nocapture")
                .arg("--test-threads=1")
                .env(CHILD, mode)
                .output()
                .unwrap();
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            let stdout = String::from_utf8(output.stdout).unwrap();
            let bytes = stdout
                .lines()
                .find_map(|line| line.split_once(PREFIX).map(|(_, value)| value))
                .unwrap()
                .parse::<u64>()
                .unwrap();
            samples.push(serde_json::json!({
                "campaign": campaign, "mode": mode, "peak_rss_delta_bytes": bytes,
            }));
        }
    }
    println!("SLICE15_RSS={}", serde_json::Value::Array(samples));
}

#[cfg(feature = "operator")]
#[test]
#[ignore = "Slice 15 paired erase/excise latency campaigns"]
fn erasure_latency_emits_paired_observations() {
    let _serial = serialize_fixture();
    use std::time::Instant;

    fn held_observation(excise: bool) -> (u64, &'static str) {
        let (_directory, engine, request) = fixture();
        let engine = Arc::new(engine);
        let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
        let frozen = match &request.context {
            GraphReadContextV1::Frozen { context, .. } => context.clone(),
            _ => unreachable!(),
        };
        let reference = treated.evidence[0].target_ref.clone();
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let hook_entered = Arc::clone(&entered);
        let hook_release = Arc::clone(&release);
        arm_evidence_before_resolve_return_hook_for_test(Box::new(move || {
            hook_entered.wait();
            hook_release.wait();
        }));
        let resolver_engine = Arc::clone(&engine);
        let resolver = thread::spawn(move || {
            resolver_engine.resolve_graph_evidence_for_test(&reference, &frozen)
        });
        entered.wait();
        let (attempted_send, attempted_receive) = std::sync::mpsc::sync_channel(1);
        arm_erasure_before_primary_lock_hook_for_test(Box::new(move || {
            attempted_send.send(()).unwrap();
        }));
        let eraser_engine = Arc::clone(&engine);
        let eraser = thread::spawn(move || {
            let started = Instant::now();
            let outcome = if excise {
                eraser_engine.excise_source("source-owner").map(|_| ())
            } else {
                eraser_engine.erase_source("source-owner").map(|_| ())
            };
            (started.elapsed().as_nanos() as u64 / 1_000, outcome)
        });
        attempted_receive.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        release.wait();
        resolver.join().unwrap().unwrap();
        let (elapsed, outcome) = eraser.join().unwrap();
        let label = match outcome {
            Ok(()) => "complete",
            Err(fathomdb_engine::EngineError::ErasureIncomplete { stage, .. })
                if stage == "wal_checkpoint" =>
            {
                "wal_checkpoint"
            }
            Err(error) => panic!("unexpected erasure result: {error:?}"),
        };
        (elapsed, label)
    }

    let mut samples = Vec::new();
    for campaign in 0..20 {
        let operation_order = if campaign % 2 == 0 { [false, true] } else { [true, false] };
        for excise in operation_order {
            let idle_observation = || {
                let (_directory, engine, _request) = fixture();
                let started = Instant::now();
                let outcome = if excise {
                    engine.excise_source("source-owner").map(|_| ())
                } else {
                    engine.erase_source("source-owner").map(|_| ())
                };
                assert!(outcome.is_ok());
                started.elapsed().as_nanos() as u64 / 1_000
            };
            let (idle_us, held_us, outcome, pair_order) = if campaign % 2 == 0 {
                let idle_us = idle_observation();
                let (held_us, outcome) = held_observation(excise);
                (idle_us, held_us, outcome, "idle_then_held")
            } else {
                let (held_us, outcome) = held_observation(excise);
                let idle_us = idle_observation();
                (idle_us, held_us, outcome, "held_then_idle")
            };
            samples.push(serde_json::json!({
                "campaign": campaign,
                "operation": if excise { "excise" } else { "erase" },
                "idle_us": idle_us, "held_us": held_us, "held_outcome": outcome,
                "pair_order": pair_order,
            }));
        }
    }
    println!("SLICE15_ERASURE={}", serde_json::Value::Array(samples));
}
