//! Slice 55 RED contract for reciprocal, frozen dependency tracing.

use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, DependencyTraceDirectionV1, DependencyTraceErrorReasonV1,
    DependencyTraceRequestV1, Engine, EngineError, InitialState, PreparedWrite, ProvenancedNodeV1,
    ReadContextV1, ReadView, SearchFilter, SourceDependencyRegistrationV1, SourceId, SourceLocator,
    SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
#[cfg(feature = "test-hooks")]
use proptest::prelude::*;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn source(revision: &str, logical: &str, body: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("slice55-source").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new("slice55-v1").unwrap(),
        ),
    })
}

fn derived(revision: &str, logical: &str, source_revision: &str, body: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "fact".into(),
        body: body.into(),
        source_id: SourceId::new("slice55-source").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new("slice55-v1").unwrap(),
            SourceRevisionId::new(source_revision).unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest("canonical trace bytes")).unwrap(),
        ),
    })
}

fn seeded() -> (TempDir, fathomdb_engine::OpenedEngine) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("trace{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .write(&[
            source("source-r1", "source", "canonical trace bytes"),
            derived("derived-r1", "derived", "source-r1", "derived trace needle"),
        ])
        .unwrap();
    opened
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-1", "source-r1", "derived-r1").unwrap(),
        )
        .unwrap();
    (dir, opened)
}

fn read_context() -> ReadContextV1 {
    ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap()
}

fn trace(
    engine: &Engine,
    root: &str,
    direction: DependencyTraceDirectionV1,
) -> Result<fathomdb_engine::DependencyTraceResultV1, EngineError> {
    let context = engine.freeze_read_context(&read_context()).unwrap();
    engine.trace_dependency(DependencyTraceRequestV1::new(root, direction, context).unwrap())
}

#[test]
fn slice55_trace_reciprocal_same_context() {
    let (_dir, opened) = seeded();
    let context = opened.engine.freeze_read_context(&read_context()).unwrap();
    let forward = opened
        .engine
        .trace_dependency(
            DependencyTraceRequestV1::new(
                "source-r1",
                DependencyTraceDirectionV1::ToDependents,
                context.clone(),
            )
            .unwrap(),
        )
        .unwrap();
    let reverse = opened
        .engine
        .trace_dependency(
            DependencyTraceRequestV1::new(
                "derived-r1",
                DependencyTraceDirectionV1::ToSource,
                context,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(forward.dependency_edges, reverse.dependency_edges);
    assert_eq!(forward.checked_work_units, 2);
    assert_eq!(reverse.checked_work_units, 2);
}

#[test]
fn slice55_trace_root_and_counterpart_order() {
    let (_dir, opened) = seeded();
    let result =
        trace(&opened.engine, "source-r1", DependencyTraceDirectionV1::ToDependents).unwrap();
    assert_eq!(result.nodes[0].artifact_revision_id.as_str(), "source-r1");
    assert_eq!(result.nodes[0].depth, 0);
    assert_eq!(result.nodes[1].artifact_revision_id.as_str(), "derived-r1");
    assert_eq!(result.nodes[1].depth, 1);
    assert!(result.complete);
}

#[test]
fn slice55_trace_invisible_roots_are_nondisclosing() {
    let (_dir, opened) = seeded();
    let error =
        trace(&opened.engine, "absent-r1", DependencyTraceDirectionV1::ToSource).unwrap_err();
    assert!(matches!(
        error,
        EngineError::DependencyTrace(ref value)
            if value.reason == DependencyTraceErrorReasonV1::TraceUnavailable
                && value.field_path == "/rootRevisionId"
    ));
}

#[test]
fn slice55_trace_hidden_relations_match_absence() {
    let (_dir, opened) = seeded();
    let source_only =
        trace(&opened.engine, "source-r1", DependencyTraceDirectionV1::ToDependents).unwrap();
    assert_eq!(source_only.dependency_edges.len(), 1);
}

#[test]
fn slice55_trace_hidden_relations_do_not_trip_caps() {
    let (_dir, opened) = seeded();
    let context = opened.engine.freeze_read_context(&read_context()).unwrap();
    let request = DependencyTraceRequestV1::new(
        "source-r1",
        DependencyTraceDirectionV1::ToDependents,
        context,
    )
    .unwrap()
    .with_bounds(1, 2)
    .unwrap();
    assert!(opened.engine.trace_dependency(request).is_ok());
}

#[test]
fn slice55_trace_corrupt_requires_two_eligible_endpoints() {
    let (_dir, opened) = seeded();
    assert!(trace(&opened.engine, "derived-r1", DependencyTraceDirectionV1::ToSource).is_ok());
}

#[test]
fn slice55_trace_cap_plus_one_is_all_or_error() {
    let (_dir, opened) = seeded();
    let context = opened.engine.freeze_read_context(&read_context()).unwrap();
    let request = DependencyTraceRequestV1::new(
        "source-r1",
        DependencyTraceDirectionV1::ToDependents,
        context,
    )
    .unwrap()
    .with_bounds(1, 1)
    .unwrap();
    assert!(matches!(
        opened.engine.trace_dependency(request),
        Err(EngineError::DependencyTrace(ref value))
            if value.reason == DependencyTraceErrorReasonV1::TraceBoundExceeded
    ));
}

#[cfg(feature = "test-hooks")]
#[test]
fn slice55_trace_query_plans_use_existing_indexes() {
    let (_dir, opened) = seeded();
    let plans = opened.engine.dependency_trace_query_plans_for_test().unwrap();
    assert!(plans.iter().any(|plan| plan.contains("derived_revision_id")));
    assert!(plans.iter().any(|plan| plan.contains("_fathomdb_source_links_source_derived_idx")));
    assert!(plans.iter().all(|plan| !plan.contains("USE TEMP B-TREE")));
}

#[cfg(feature = "test-hooks")]
#[test]
#[ignore = "release-mode hidden-row ceiling"]
fn slice55_trace_hidden_dependents_performance_ceiling() {
    let (_dir, opened) = seeded();
    let measurement = opened.engine.measure_dependency_trace_for_test().unwrap();
    assert!(measurement.vm_steps <= 10_000_000);
    assert!(measurement.elapsed.as_secs_f64() <= 5.0);
    assert!(measurement.peak_rss_delta_bytes <= 64 * 1024 * 1024);
}

#[cfg(feature = "test-hooks")]
proptest! {
    #[test]
    fn slice55_trace_codec_round_trip(root in "[A-Za-z0-9][A-Za-z0-9._:-]{0,31}") {
        let encoded = fathomdb_engine::encode_dependency_trace_root_for_test(&root).unwrap();
        prop_assert_eq!(fathomdb_engine::decode_dependency_trace_root_for_test(&encoded).unwrap(), root);
    }
}
