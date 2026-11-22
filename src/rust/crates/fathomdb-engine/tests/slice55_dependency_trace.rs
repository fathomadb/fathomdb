//! Slice 55 RED contract for reciprocal, frozen dependency tracing.

use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, DependencyTraceDirectionV1, DependencyTraceErrorReasonV1,
    DependencyTraceRequestV1, Engine, EngineError, InitialState, PreparedWrite, ProvenancedEdgeV1,
    ProvenancedNodeV1, ReadContextV1, ReadView, SearchFilter, SourceDependencyRegistrationV1,
    SourceId, SourceLocator, SourceRevisionId, SourceVersionId, TraceArtifactClassV1,
    TraceArtifactRoleV1, WriteProvenanceV1,
};
// Only used by the debug-build-only tests below.
#[cfg(debug_assertions)]
use fathomdb_engine::encode_dependency_trace_result_v1;
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
        properties: None,
    })
}

fn derived(revision: &str, logical: &str, source_revision: &str, body: &str) -> PreparedWrite {
    derived_node(
        logical,
        body,
        WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new("slice55-v1").unwrap(),
            SourceRevisionId::new(source_revision).unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest("canonical trace bytes")).unwrap(),
        ),
    )
}

fn derived_node(logical: &str, body: &str, provenance: WriteProvenanceV1) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "fact".into(),
        body: body.into(),
        source_id: SourceId::new("slice55-source").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance,
        properties: None,
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

// Seeds via `Engine::execute_for_test`, a debug-build-only hook.
#[cfg(debug_assertions)]
fn add_hidden_dependent(engine: &Engine, revision: &str, logical: &str, dependency: &str) {
    let cursor = if logical.starts_with("aaa") { -101 } else { -102 };
    engine
        .execute_for_test(&format!(
            "INSERT INTO canonical_nodes(write_cursor,kind,body,source_id,state) \
               VALUES({cursor},'hidden','hidden','slice55-source','deleted'); \
             INSERT INTO _fathomdb_artifact_revisions(\
               schema_version,revision_id,artifact_class,write_cursor,artifact_role,completeness\
             ) VALUES(1,'{revision}','node',{cursor},'derived_semantic','complete'); \
             INSERT INTO _fathomdb_source_links(\
               schema_version,artifact_revision_id,source_id,source_version_id,\
               source_revision_id,locator_kind,start_byte,end_byte,hash_algorithm,hash_digest\
             ) VALUES(1,'{revision}','slice55-source','slice55-v1','source-r1',\
                      'whole_body',NULL,NULL,'sha256',\
                      '0000000000000000000000000000000000000000000000000000000000000000'); \
             INSERT INTO _fathomdb_source_dependencies(\
               schema_version,dependency_id,derived_revision_id,registered_dependency_generation\
             ) VALUES(1,'{dependency}','{revision}',1)"
        ))
        .unwrap();
}

// Seeds via `Engine::execute_for_test`, a debug-build-only hook.
#[cfg(debug_assertions)]
fn insert_active_closure(engine: &Engine, source_revision: &str, cause: &str, sequence: u64) {
    let closure_id = format!("_fdb:c:{sequence:064x}");
    let fingerprint = format!("{sequence:064x}");
    let (phase, proof_boundary, proof_json) = if matches!(cause, "purged" | "source_erased") {
        ("at_rest_pending", "100", "'{}'")
    } else {
        ("proving", "NULL", "NULL")
    };
    engine
        .execute_for_test(&format!(
            "UPDATE _fathomdb_open_state SET value='{sequence}' \
             WHERE key='_fathomdb_closure_sequence'; \
             INSERT INTO _fathomdb_dependency_closures(\
               schema_version,closure_operation_id,root_kind,root_value,cause,\
               effective_at_epoch_s,admitted_write_boundary,admitted_dependency_generation,\
               closure_sequence,retry_fingerprint,phase,affected_count,blocker_code,\
               structural_proof_write_boundary,proof_json\
             ) VALUES(1,'{closure_id}','source_revision','{source_revision}','{cause}',\
                      0,100,0,{sequence},'{fingerprint}','{phase}',1,NULL,\
                      {proof_boundary},{proof_json})"
        ))
        .unwrap();
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

// Only used by `fixed_trace`, itself only used by the debug-build-only tests
// below.
#[cfg(debug_assertions)]
fn fixed_read_context() -> ReadContextV1 {
    ReadContextV1::new(
        ReadView { valid_as_of: Some(1), ..ReadView::default() },
        SearchFilter::default(),
    )
    .unwrap()
}

// Only called from the debug-build-only tests below (via `Engine::execute_for_test`).
#[cfg(debug_assertions)]
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

const SPAN_SOURCE: &str = "aé猫z";

fn partial_span_provenance(revision: &str, locator: SourceLocator) -> WriteProvenanceV1 {
    WriteProvenanceV1::derived(
        ArtifactRevisionId::new(revision).unwrap(),
        SourceVersionId::new("slice55-v1").unwrap(),
        SourceRevisionId::new("span-source-r1").unwrap(),
        locator,
        CanonicalHash::sha256(digest(SPAN_SOURCE)).unwrap(),
    )
}

fn partial_span_fixture() -> (TempDir, fathomdb_engine::OpenedEngine, rusqlite::Connection) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("spans{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let node = |revision, logical, locator| {
        derived_node(logical, "derived span", partial_span_provenance(revision, locator))
    };
    opened
        .engine
        .write(&[
            source("span-source-r1", "span-source", SPAN_SOURCE),
            node("node-r1", "span-node", SourceLocator::utf8_bytes(1, 3)),
            node("full-r1", "full-node", SourceLocator::utf8_bytes(0, 7)),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                kind: "relates".into(),
                from: "span-source".into(),
                to: "span-node".into(),
                source_id: SourceId::new("slice55-source").unwrap(),
                logical_id: Some("span-edge".into()),
                body: Some("derived edge".into()),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: partial_span_provenance("edge-r1", SourceLocator::utf8_bytes(1, 3)),
                properties: None,
            }),
        ])
        .unwrap();
    for (dependency, revision) in
        [("dep-node", "node-r1"), ("dep-full", "full-r1"), ("dep-edge", "edge-r1")]
    {
        opened
            .engine
            .register_source_dependency(
                SourceDependencyRegistrationV1::new(dependency, "span-source-r1", revision)
                    .unwrap(),
            )
            .unwrap();
    }
    let raw = rusqlite::Connection::open(path).unwrap();
    (dir, opened, raw)
}

#[test]
fn slice55_trace_partial_spans_authenticate_the_whole_canonical_source() {
    let (_dir, opened, raw) = partial_span_fixture();
    let engine = &opened.engine;
    let context = engine.freeze_read_context(&read_context()).unwrap();
    let forward = engine
        .trace_dependency(
            DependencyTraceRequestV1::new(
                "span-source-r1",
                DependencyTraceDirectionV1::ToDependents,
                context.clone(),
            )
            .unwrap(),
        )
        .unwrap();

    let cases = [
        ("edge-r1", "dep-edge", TraceArtifactClassV1::Edge, (1, 3), 3),
        ("full-r1", "dep-full", TraceArtifactClassV1::Node, (0, 7), 2),
        ("node-r1", "dep-node", TraceArtifactClassV1::Node, (1, 3), 1),
    ];
    let reverse: Vec<_> = cases
        .iter()
        .map(|&(revision, _, _, _, _)| {
            engine.trace_dependency(
                DependencyTraceRequestV1::new(
                    revision,
                    DependencyTraceDirectionV1::ToSource,
                    context.clone(),
                )
                .unwrap(),
            )
        })
        .collect();
    assert_eq!(
        forward.dependency_edges.iter().map(|edge| edge.dependency_id.as_str()).collect::<Vec<_>>(),
        ["dep-edge", "dep-full", "dep-node"],
        "reverse traces: {reverse:?}"
    );
    assert!(forward.complete);
    assert_eq!(forward.checked_work_units, 4);
    assert_eq!(forward.nodes.len(), 4);
    assert_eq!(forward.nodes[0].artifact_revision_id, "span-source-r1");
    assert_eq!(forward.nodes[0].role, TraceArtifactRoleV1::CanonicalSource);
    assert_eq!(forward.nodes[0].depth, 0);
    for (index, ((revision, dependency, class, (start, end), generation), reverse)) in
        cases.into_iter().zip(reverse).enumerate()
    {
        let stored: (String, i64, i64, String, String) = raw
            .query_row(
                "SELECT locator_kind,start_byte,end_byte,hash_algorithm,hash_digest
                 FROM _fathomdb_source_links WHERE artifact_revision_id=?1",
                [revision],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .unwrap();
        assert_eq!(stored, ("utf8_bytes".into(), start, end, "sha256".into(), digest(SPAN_SOURCE)));

        let forward_node = &forward.nodes[index + 1];
        assert_eq!(forward_node.artifact_revision_id, revision);
        assert_eq!(forward_node.artifact_class, class);
        assert_eq!(forward_node.role, TraceArtifactRoleV1::Derived);
        assert_eq!(forward_node.depth, 1);
        let edge = &forward.dependency_edges[index];
        assert_eq!(edge.dependency_id, dependency);
        assert_eq!(edge.source_revision_id, "span-source-r1");
        assert_eq!(edge.derived_revision_id, revision);
        assert_eq!(edge.registered_dependency_generation, generation);

        let reverse = reverse.unwrap();
        assert!(reverse.complete);
        assert_eq!(reverse.checked_work_units, 2);
        assert_eq!(reverse.read_boundary, forward.read_boundary);
        assert_eq!(reverse.dependency_edges.as_slice(), std::slice::from_ref(edge));
        assert_eq!(reverse.nodes.len(), 2);
        let mut root = forward_node.clone();
        root.depth = 0;
        let mut source = forward.nodes[0].clone();
        source.depth = 1;
        assert_eq!(reverse.nodes, [root, source]);
    }
}

#[test]
fn slice55_trace_partial_span_corruption_preserves_nondisclosure() {
    let (_dir, opened, mut raw) = partial_span_fixture();
    let assert_refused = || {
        let engine = &opened.engine;
        let context = engine.freeze_read_context(&read_context()).unwrap();
        let reverse = engine
            .trace_dependency(
                DependencyTraceRequestV1::new(
                    "node-r1",
                    DependencyTraceDirectionV1::ToSource,
                    context.clone(),
                )
                .unwrap(),
            )
            .unwrap_err();
        assert!(
            matches!(
                reverse,
                EngineError::DependencyTrace(ref error)
                    if error.reason == DependencyTraceErrorReasonV1::TraceUnavailable
                        && error.field_path == "/rootRevisionId"
            ),
            "{reverse:?}"
        );
        let forward = engine
            .trace_dependency(
                DependencyTraceRequestV1::new(
                    "span-source-r1",
                    DependencyTraceDirectionV1::ToDependents,
                    context,
                )
                .unwrap(),
            )
            .unwrap();
        assert!(forward.complete);
        assert_eq!(forward.checked_work_units, 3);
        assert_eq!(
            forward.nodes.iter().map(|n| n.artifact_revision_id.as_str()).collect::<Vec<_>>(),
            ["span-source-r1", "edge-r1", "full-r1"]
        );
        assert_eq!(
            forward
                .dependency_edges
                .iter()
                .map(|edge| edge.dependency_id.as_str())
                .collect::<Vec<_>>(),
            ["dep-edge", "dep-full"]
        );
    };

    for (start, end, hash) in [
        (1, 3, digest("é")),
        (2, 3, digest(SPAN_SOURCE)),
        (1, 8, digest(SPAN_SOURCE)),
        (1, 1, digest(SPAN_SOURCE)),
    ] {
        raw.execute(
            "UPDATE _fathomdb_source_links SET start_byte=?1,end_byte=?2,hash_digest=?3
             WHERE artifact_revision_id='node-r1'",
            rusqlite::params![start, end, hash],
        )
        .unwrap();
        assert_refused();
    }

    let changed = "bé猫z";
    assert_eq!(&changed[1..3], &SPAN_SOURCE[1..3]);
    let tx = raw.transaction().unwrap();
    tx.execute("UPDATE canonical_nodes SET body=?1 WHERE logical_id='span-source'", [changed])
        .unwrap();
    tx.execute(
        "UPDATE _fathomdb_source_links SET hash_digest=?1 WHERE artifact_revision_id!='node-r1'",
        [digest(changed)],
    )
    .unwrap();
    tx.execute(
        "UPDATE _fathomdb_source_links SET start_byte=1,end_byte=3 WHERE artifact_revision_id='node-r1'",
        [],
    )
    .unwrap();
    tx.commit().unwrap();
    assert_refused();
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

// Uses `Engine::execute_for_test` (directly or via a fixture helper), a
// debug-build-only hook.
#[cfg(debug_assertions)]
#[test]
fn slice55_trace_hidden_relations_match_absence() {
    let (dir, opened) = seeded();
    let original = dir.path().join(format!("trace{SQLITE_SUFFIX}"));
    opened.engine.close().unwrap();
    let baseline_path = dir.path().join(format!("trace-baseline{SQLITE_SUFFIX}"));
    let hidden_path = dir.path().join(format!("trace-hidden{SQLITE_SUFFIX}"));
    std::fs::copy(&original, &baseline_path).unwrap();
    std::fs::copy(&original, &hidden_path).unwrap();
    let baseline = Engine::open(baseline_path).unwrap();
    let hidden = Engine::open(hidden_path).unwrap();
    add_hidden_dependent(&hidden.engine, "aaa-hidden-r1", "aaa-hidden", "aaa-hidden-dep");
    add_hidden_dependent(&hidden.engine, "zzz-hidden-r1", "zzz-hidden", "zzz-hidden-dep");

    let baseline =
        fixed_trace(&baseline.engine, "source-r1", DependencyTraceDirectionV1::ToDependents)
            .unwrap();
    let hidden =
        fixed_trace(&hidden.engine, "source-r1", DependencyTraceDirectionV1::ToDependents).unwrap();
    assert_eq!(
        encode_dependency_trace_result_v1(&hidden),
        encode_dependency_trace_result_v1(&baseline)
    );
    assert_eq!(hidden.checked_work_units, 2);
}

// Uses `Engine::execute_for_test` (directly or via a fixture helper), a
// debug-build-only hook.
#[cfg(debug_assertions)]
#[test]
fn slice55_trace_hidden_relations_do_not_trip_caps() {
    let (_dir, opened) = seeded();
    add_hidden_dependent(&opened.engine, "aaa-hidden-r1", "aaa-hidden", "aaa-hidden-dep");
    add_hidden_dependent(&opened.engine, "zzz-hidden-r1", "zzz-hidden", "zzz-hidden-dep");
    let context = opened.engine.freeze_read_context(&read_context()).unwrap();
    let request = DependencyTraceRequestV1::new(
        "source-r1",
        DependencyTraceDirectionV1::ToDependents,
        context,
    )
    .unwrap()
    .with_bounds(1, 2)
    .unwrap();
    let result = opened.engine.trace_dependency(request).unwrap();
    assert_eq!(result.dependency_edges.len(), 1);
    assert_eq!(result.dependency_edges[0].dependency_id, "dep-1");
    assert_eq!(result.checked_work_units, 2);
}

// Uses `Engine::execute_for_test` (directly or via a fixture helper), a
// debug-build-only hook.
#[cfg(debug_assertions)]
#[test]
fn slice55_trace_hidden_corruption_is_filtered_before_decode_and_limit() {
    let (_dir, opened) = seeded();
    add_hidden_dependent(&opened.engine, "aaa-hidden-r1", "aaa-hidden", "aaa-hidden-dep");
    opened
        .engine
        .execute_for_test(
            "PRAGMA ignore_check_constraints=ON; \
             UPDATE _fathomdb_source_dependencies SET dependency_id=zeroblob(32) \
             WHERE derived_revision_id='aaa-hidden-r1'",
        )
        .unwrap();
    let context = opened.engine.freeze_read_context(&read_context()).unwrap();
    let request = DependencyTraceRequestV1::new(
        "source-r1",
        DependencyTraceDirectionV1::ToDependents,
        context,
    )
    .unwrap()
    .with_bounds(1, 2)
    .unwrap();
    let result = opened.engine.trace_dependency(request).unwrap();
    assert_eq!(result.dependency_edges.len(), 1);
    assert_eq!(result.dependency_edges[0].dependency_id, "dep-1");
    assert_eq!(result.checked_work_units, 2);
}

// Uses `Engine::execute_for_test` (directly or via a fixture helper), a
// debug-build-only hook.
#[cfg(debug_assertions)]
#[test]
fn slice55_trace_invalid_chain_precedes_relation_blob_decode_and_cap() {
    let (_dir, opened) = seeded();
    add_hidden_dependent(&opened.engine, "aaa-chain-r1", "aaa-chain", "aaa-chain-dep");
    opened
        .engine
        .execute_for_test(
            "PRAGMA ignore_check_constraints=ON; \
             UPDATE canonical_nodes SET state='active' WHERE write_cursor=-101; \
             UPDATE _fathomdb_source_dependencies SET dependency_id=zeroblob(32) \
             WHERE derived_revision_id='aaa-chain-r1'",
        )
        .unwrap();
    let result =
        fixed_trace(&opened.engine, "source-r1", DependencyTraceDirectionV1::ToDependents).unwrap();
    assert_eq!(result.dependency_edges.len(), 1);
    assert_eq!(result.dependency_edges[0].dependency_id, "dep-1");
    assert_eq!(result.checked_work_units, 2);
    let reverse = fixed_trace(&opened.engine, "aaa-chain-r1", DependencyTraceDirectionV1::ToSource)
        .unwrap_err();
    assert!(matches!(
        reverse,
        EngineError::DependencyTrace(ref value)
            if value.reason == DependencyTraceErrorReasonV1::TraceUnavailable
                && value.field_path == "/rootRevisionId"
    ));
}

// Uses `Engine::execute_for_test` (directly or via a fixture helper), a
// debug-build-only hook.
#[cfg(debug_assertions)]
#[test]
fn slice55_trace_corrupt_requires_two_eligible_endpoints() {
    let (_dir, opened) = seeded();
    opened
        .engine
        .execute_for_test(
            "PRAGMA ignore_check_constraints=ON; \
             UPDATE _fathomdb_source_dependencies SET registered_dependency_generation=0 \
             WHERE dependency_id='dep-1'",
        )
        .unwrap();
    for (root, direction) in [
        ("source-r1", DependencyTraceDirectionV1::ToDependents),
        ("derived-r1", DependencyTraceDirectionV1::ToSource),
    ] {
        assert!(matches!(
            trace(&opened.engine, root, direction),
            Err(EngineError::DependencyTrace(ref error))
                if error.reason == DependencyTraceErrorReasonV1::TraceCorrupt
                    && error.field_path.is_empty()
        ));
    }
}

// Uses `Engine::execute_for_test` (directly or via a fixture helper), a
// debug-build-only hook.
#[cfg(debug_assertions)]
#[test]
fn slice55_trace_closure_fences_are_endpoint_specific() {
    let (_dir, opened) = seeded();
    insert_active_closure(&opened.engine, "unrelated-r1", "source_erased", 1);

    let result =
        fixed_trace(&opened.engine, "source-r1", DependencyTraceDirectionV1::ToDependents).unwrap();
    assert_eq!(result.dependency_edges.len(), 1);
    assert_eq!(result.checked_work_units, 2);
}

// Uses `Engine::execute_for_test` (directly or via a fixture helper), a
// debug-build-only hook.
#[cfg(debug_assertions)]
#[test]
fn slice55_trace_closure_fences_each_relation_endpoint() {
    let (_dir, opened) = seeded();
    insert_active_closure(&opened.engine, "source-r1", "soft_deleted", 1);

    for (root, direction) in [
        ("source-r1", DependencyTraceDirectionV1::ToDependents),
        ("derived-r1", DependencyTraceDirectionV1::ToSource),
    ] {
        assert!(matches!(
            fixed_trace(&opened.engine, root, direction),
            Err(EngineError::DependencyTrace(ref error))
                if error.reason == DependencyTraceErrorReasonV1::TraceUnavailable
                    && error.field_path == "/rootRevisionId"
        ));
    }
}

// Uses `Engine::execute_for_test` (directly or via a fixture helper), a
// debug-build-only hook.
#[cfg(debug_assertions)]
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

// Uses `Engine::execute_for_test` (directly or via a fixture helper), a
// debug-build-only hook.
#[cfg(debug_assertions)]
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

// Uses `Engine::execute_for_test` (directly or via a fixture helper), a
// debug-build-only hook.
#[cfg(debug_assertions)]
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

// Uses `Engine::execute_for_test` (directly or via a fixture helper), a
// debug-build-only hook.
#[cfg(debug_assertions)]
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

// Uses `Engine::execute_for_test` (directly or via a fixture helper), a
// debug-build-only hook.
#[cfg(debug_assertions)]
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

// Uses `Engine::execute_for_test` (directly or via a fixture helper), a
// debug-build-only hook.
#[cfg(debug_assertions)]
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
    assert!(plans.iter().all(|plan| !plan.contains("MATERIALIZE")));
    let queries = opened.engine.dependency_trace_candidate_queries_for_test();
    assert!(queries[0].contains("LIMIT ?2"));
    assert!(queries[1].contains("l.artifact_revision_id>?2"));
    assert!(queries[1].contains("LIMIT ?3"));
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
