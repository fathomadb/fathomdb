//! Slice 55 RED contract for reciprocal, frozen dependency tracing.

use fathomdb_engine::{
    encode_dependency_trace_result_v1, ArtifactRevisionId, CanonicalHash,
    DependencyTraceDirectionV1, DependencyTraceErrorReasonV1, DependencyTraceRequestV1, Engine,
    EngineError, InitialState, PreparedWrite, ProvenancedNodeV1, ReadContextV1, ReadView,
    SearchFilter, SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId,
    SourceVersionId, WriteProvenanceV1,
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

#[cfg(feature = "test-hooks")]
fn source_only() -> (TempDir, fathomdb_engine::OpenedEngine) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("trace-source-only{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    opened.engine.write(&[source("source-r1", "source", "canonical trace bytes")]).unwrap();
    (dir, opened)
}

fn read_context() -> ReadContextV1 {
    ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap()
}

fn fixed_read_context() -> ReadContextV1 {
    ReadContextV1::new(
        ReadView { valid_as_of: Some(1), ..ReadView::default() },
        SearchFilter::default(),
    )
    .unwrap()
}

fn fixed_trace(
    engine: &Engine,
    root: &str,
    direction: DependencyTraceDirectionV1,
) -> Result<fathomdb_engine::DependencyTraceResultV1, EngineError> {
    let context = engine.freeze_read_context(&fixed_read_context()).unwrap();
    engine.trace_dependency(DependencyTraceRequestV1::new(root, direction, context).unwrap())
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
fn slice55_trace_execution_boundary_revalidates_public_struct_literals() {
    let (_dir, opened) = seeded();
    let context = opened.engine.freeze_read_context(&read_context()).unwrap();
    let cases = [
        (
            DependencyTraceRequestV1 {
                schema_version: 2,
                root_revision_id: "source-r1".into(),
                direction: DependencyTraceDirectionV1::ToDependents,
                context: context.clone(),
                max_relations: 1,
                max_work_units: 2,
            },
            DependencyTraceErrorReasonV1::UnsupportedSchemaVersion,
            "/schemaVersion",
        ),
        (
            DependencyTraceRequestV1 {
                schema_version: 1,
                root_revision_id: "!".into(),
                direction: DependencyTraceDirectionV1::ToDependents,
                context: context.clone(),
                max_relations: 1,
                max_work_units: 2,
            },
            DependencyTraceErrorReasonV1::TraceRootInvalid,
            "/rootRevisionId",
        ),
        (
            DependencyTraceRequestV1 {
                schema_version: 1,
                root_revision_id: "source-r1".into(),
                direction: DependencyTraceDirectionV1::ToDependents,
                context: context.clone(),
                max_relations: 101,
                max_work_units: 2,
            },
            DependencyTraceErrorReasonV1::TraceLimitInvalid,
            "/maxRelations",
        ),
        (
            DependencyTraceRequestV1 {
                schema_version: 1,
                root_revision_id: "source-r1".into(),
                direction: DependencyTraceDirectionV1::ToDependents,
                context,
                max_relations: 1,
                max_work_units: 102,
            },
            DependencyTraceErrorReasonV1::TraceLimitInvalid,
            "/maxWorkUnits",
        ),
    ];
    for (request, reason, path) in cases {
        let error = opened.engine.trace_dependency(request).unwrap_err();
        assert!(matches!(
            error,
            EngineError::DependencyTrace(ref value)
                if value.reason == reason && value.field_path == path
        ));
    }
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
fn slice55_trace_malformed_lifecycle_collapses_to_absence() {
    let (_dir, opened) = seeded();
    opened
        .engine
        .execute_for_test(
            "PRAGMA ignore_check_constraints=ON; \
             UPDATE canonical_nodes SET state='not-a-state' WHERE write_cursor=1",
        )
        .unwrap();
    let error = fixed_trace(&opened.engine, "source-r1", DependencyTraceDirectionV1::ToDependents)
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::DependencyTrace(ref value)
            if value.reason == DependencyTraceErrorReasonV1::TraceUnavailable
                && value.field_path == "/rootRevisionId"
    ));
}

#[test]
fn slice55_trace_digest_chain_authenticates_canonical_bytes() {
    let (_dir, opened) = seeded();
    opened
        .engine
        .execute_for_test(
            "UPDATE _fathomdb_source_links SET hash_digest=\
             'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' \
             WHERE artifact_revision_id IN ('source-r1','derived-r1')",
        )
        .unwrap();
    let error = fixed_trace(&opened.engine, "source-r1", DependencyTraceDirectionV1::ToDependents)
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::DependencyTrace(ref value)
            if value.reason == DependencyTraceErrorReasonV1::TraceUnavailable
                && value.field_path == "/rootRevisionId"
    ));
}

#[test]
fn slice55_trace_hidden_chain_corruption_is_byte_equal_to_absence() {
    let (dir, seeded) = seeded();
    let original = dir.path().join(format!("trace{SQLITE_SUFFIX}"));
    seeded.engine.close().unwrap();
    let absence_path = dir.path().join(format!("absence{SQLITE_SUFFIX}"));
    let hidden_path = dir.path().join(format!("hidden{SQLITE_SUFFIX}"));
    std::fs::copy(&original, &absence_path).unwrap();
    std::fs::copy(&original, &hidden_path).unwrap();
    let absence = Engine::open(absence_path).unwrap();
    absence
        .engine
        .execute_for_test("DELETE FROM _fathomdb_source_dependencies WHERE dependency_id='dep-1'")
        .unwrap();
    let absent =
        fixed_trace(&absence.engine, "source-r1", DependencyTraceDirectionV1::ToDependents)
            .unwrap();

    let hidden = Engine::open(hidden_path).unwrap();
    hidden
        .engine
        .execute_for_test("DELETE FROM _fathomdb_artifact_revisions WHERE revision_id='derived-r1'")
        .unwrap();
    let hidden =
        fixed_trace(&hidden.engine, "source-r1", DependencyTraceDirectionV1::ToDependents).unwrap();

    assert_eq!(
        encode_dependency_trace_result_v1(&hidden),
        encode_dependency_trace_result_v1(&absent)
    );
    assert_eq!(hidden.checked_work_units, 1);
}

#[test]
fn slice55_trace_requires_complete_source_provenance_chain() {
    let (_dir, opened) = seeded();
    opened
        .engine
        .execute_for_test(
            "DELETE FROM _fathomdb_source_versions WHERE source_revision_id='source-r1'",
        )
        .unwrap();
    let error = fixed_trace(&opened.engine, "derived-r1", DependencyTraceDirectionV1::ToSource)
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::DependencyTrace(ref value)
            if value.reason == DependencyTraceErrorReasonV1::TraceUnavailable
                && value.field_path == "/rootRevisionId"
    ));
}

#[test]
fn slice55_trace_rejects_wrong_counterpart_role_without_disclosure() {
    let (_dir, opened) = seeded();
    opened
        .engine
        .execute_for_test(
            "UPDATE _fathomdb_artifact_revisions SET artifact_role='canonical_source' \
             WHERE revision_id='derived-r1'",
        )
        .unwrap();
    let result =
        fixed_trace(&opened.engine, "source-r1", DependencyTraceDirectionV1::ToDependents).unwrap();
    assert!(result.dependency_edges.is_empty());
    assert_eq!(result.nodes.len(), 1);
}

#[test]
fn slice55_trace_reports_corrupt_only_after_both_endpoints_are_authorized() {
    let (_dir, opened) = seeded();
    opened
        .engine
        .execute_for_test(
            "PRAGMA ignore_check_constraints=ON; \
             UPDATE _fathomdb_source_dependencies SET schema_version=2 \
             WHERE dependency_id='dep-1'",
        )
        .unwrap();
    assert!(matches!(
        fixed_trace(
            &opened.engine,
            "source-r1",
            DependencyTraceDirectionV1::ToDependents,
        ),
        Err(EngineError::DependencyTrace(ref error))
            if error.reason == DependencyTraceErrorReasonV1::TraceCorrupt
                && error.field_path.is_empty()
    ));
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
    let (fixture_dir, base) = source_only();
    let base_path = base.engine.path().to_path_buf();
    base.engine.close().unwrap();
    let baseline_path = fixture_dir.path().join(format!("trace-baseline{SQLITE_SUFFIX}"));
    let hidden_path = fixture_dir.path().join(format!("trace-hidden{SQLITE_SUFFIX}"));
    std::fs::copy(&base_path, &baseline_path).unwrap();
    std::fs::copy(&base_path, &hidden_path).unwrap();

    let baseline = Engine::open(baseline_path).unwrap();
    let baseline_measurement = baseline.engine.measure_dependency_trace_for_test().unwrap();
    let hidden = Engine::open(hidden_path).unwrap();
    hidden.engine.seed_hidden_dependency_trace_fixture_for_test(50_000).unwrap();
    let measurement = hidden.engine.measure_dependency_trace_for_test().unwrap();

    eprintln!(
        "slice55 trace measurement: hidden_rows=50000 vm_steps={} elapsed_ms={} peak_rss_delta_bytes={}",
        measurement.vm_steps,
        measurement.elapsed.as_millis(),
        measurement.peak_rss_delta_bytes,
    );
    assert_eq!(measurement.response_bytes, baseline_measurement.response_bytes);
    assert!(!measurement.bound_exceeded);
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
