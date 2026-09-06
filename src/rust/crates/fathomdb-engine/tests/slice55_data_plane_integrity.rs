//! Slice 55 RED contract for bounded operator data-plane integrity.

use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ArtifactRevisionId, CanonicalHash,
    DataPlaneIntegrityCheckV1, DataPlaneIntegrityErrorReasonV1, DataPlaneIntegrityFindingCodeV1,
    DataPlaneIntegrityRequestV1, Engine, EngineError, InitialState, PreparedWrite, ProjectionFts,
    ProjectionRole, ProjectionSpec, ProvenancedNodeV1, SourceDependencyRegistrationV1, SourceId,
    SourceLocator, SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use proptest::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use tempfile::TempDir;

fn opened() -> (TempDir, fathomdb_engine::OpenedEngine) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("integrity{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    (dir, opened)
}

fn request(check: DataPlaneIntegrityCheckV1, max_work: u32) -> DataPlaneIntegrityRequestV1 {
    DataPlaneIntegrityRequestV1::new(vec![check], max_work, 100).unwrap()
}

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn canonical(revision: &str, logical: &str, body: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("slice55-integrity-source").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new(format!("version-{revision}")).unwrap(),
        ),
    })
}

fn derived(revision: &str, logical: &str, source_revision: &str, body: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "fact".into(),
        body: body.into(),
        source_id: SourceId::new("slice55-integrity-source").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new(format!("version-{source_revision}")).unwrap(),
            SourceRevisionId::new(source_revision).unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest("slice55 integrity canonical")).unwrap(),
        ),
    })
}

fn edge(from: &str, to: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "mentions".into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new("slice55-integrity-source").unwrap(),
        logical_id: Some("integrity-edge".into()),
        body: Some(body.into()),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}

fn property_spec() -> ProjectionSpec {
    ProjectionSpec {
        name: "title".into(),
        roles: BTreeSet::from([ProjectionRole::Filterable, ProjectionRole::Searchable]),
        fts: Some(ProjectionFts { tokenizer: None }),
        vector: None,
        source: None,
    }
}

fn dependency_seeded() -> (TempDir, fathomdb_engine::OpenedEngine) {
    let (dir, opened) = opened();
    opened
        .engine
        .write(&[
            canonical("integrity-source-r1", "source", "slice55 integrity canonical"),
            derived(
                "integrity-derived-r1",
                "derived",
                "integrity-source-r1",
                "slice55 integrity derived",
            ),
        ])
        .unwrap();
    opened
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new(
                "integrity-dep-1",
                "integrity-source-r1",
                "integrity-derived-r1",
            )
            .unwrap(),
        )
        .unwrap();
    (dir, opened)
}

fn finding_codes(
    opened: &fathomdb_engine::OpenedEngine,
    check: DataPlaneIntegrityCheckV1,
) -> Vec<DataPlaneIntegrityFindingCodeV1> {
    opened
        .engine
        .check_data_plane_integrity(request(check, 10_000))
        .unwrap()
        .findings
        .into_iter()
        .map(|finding| finding.code)
        .collect()
}

fn actuate_pending(opened: &fathomdb_engine::OpenedEngine, operation_id: &str, revision: &str) {
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    let batch = ActuationBatchV1::new(
        operation_id,
        vec![ActuationOperationV1::PutCanonicalNode(
            match canonical(revision, operation_id, "pending body") {
                PreparedWrite::ProvenancedNode(node) => node,
                _ => unreachable!(),
            },
        )],
    )
    .unwrap();
    opened.engine.actuate(batch).unwrap();
}

#[test]
fn slice55_integrity_work_cap_counts_authority_rows() {
    let (_dir, opened) = opened();
    let result = opened
        .engine
        .check_data_plane_integrity(request(DataPlaneIntegrityCheckV1::DependencyChain, 1))
        .unwrap();
    assert_eq!(result.checked_count, 1);
    assert!(result.complete);
}

#[test]
fn slice55_integrity_empty_receipt_counts_one_row() {
    let (_dir, opened) = opened();
    let result = opened
        .engine
        .check_data_plane_integrity(request(DataPlaneIntegrityCheckV1::MutationReadiness, 1))
        .unwrap();
    assert_eq!(result.checked_count, 0);
}

#[test]
fn slice55_integrity_uses_one_reader_snapshot() {
    let (_dir, opened) = opened();
    let result = opened
        .engine
        .check_data_plane_integrity(DataPlaneIntegrityRequestV1::all(10_000, 100).unwrap())
        .unwrap();
    assert_eq!(
        result.checked_count,
        result.check_counts.iter().map(|c| c.checked_count).sum::<u32>()
    );
}

#[test]
fn slice55_integrity_check_order_is_canonical() {
    let (_dir, opened) = opened();
    let result = opened
        .engine
        .check_data_plane_integrity(
            DataPlaneIntegrityRequestV1::new(
                vec![
                    DataPlaneIntegrityCheckV1::MutationReadiness,
                    DataPlaneIntegrityCheckV1::DependencyChain,
                ],
                10_000,
                100,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(result.check_counts[0].check, DataPlaneIntegrityCheckV1::DependencyChain);
    assert_eq!(result.check_counts[1].check, DataPlaneIntegrityCheckV1::MutationReadiness);
}

#[test]
fn slice55_integrity_execution_boundary_revalidates_public_struct_literals() {
    let (_dir, opened) = opened();
    let cases = [
        (
            DataPlaneIntegrityRequestV1 {
                schema_version: 2,
                checks: vec![DataPlaneIntegrityCheckV1::DependencyChain],
                max_work_units: 1,
                max_findings: 1,
            },
            DataPlaneIntegrityErrorReasonV1::UnsupportedSchemaVersion,
            "/schemaVersion",
        ),
        (
            DataPlaneIntegrityRequestV1 {
                schema_version: 1,
                checks: vec![],
                max_work_units: 1,
                max_findings: 1,
            },
            DataPlaneIntegrityErrorReasonV1::ChecksEmpty,
            "/checks",
        ),
        (
            DataPlaneIntegrityRequestV1 {
                schema_version: 1,
                checks: vec![
                    DataPlaneIntegrityCheckV1::DependencyChain,
                    DataPlaneIntegrityCheckV1::DependencyChain,
                ],
                max_work_units: 1,
                max_findings: 1,
            },
            DataPlaneIntegrityErrorReasonV1::DuplicateCheck,
            "/checks/1",
        ),
        (
            DataPlaneIntegrityRequestV1 {
                schema_version: 1,
                checks: vec![DataPlaneIntegrityCheckV1::DependencyChain],
                max_work_units: 10_001,
                max_findings: 1,
            },
            DataPlaneIntegrityErrorReasonV1::IntegrityLimitInvalid,
            "/maxWorkUnits",
        ),
        (
            DataPlaneIntegrityRequestV1 {
                schema_version: 1,
                checks: vec![DataPlaneIntegrityCheckV1::DependencyChain],
                max_work_units: 1,
                max_findings: 101,
            },
            DataPlaneIntegrityErrorReasonV1::IntegrityLimitInvalid,
            "/maxFindings",
        ),
    ];
    for (request, reason, path) in cases {
        let error = opened.engine.check_data_plane_integrity(request).unwrap_err();
        assert!(matches!(
            error,
            EngineError::DataPlaneIntegrity(ref value)
                if value.schema_version == 1 && value.reason == reason && value.field_path == path
        ));
    }
}

#[test]
fn slice55_dependency_chain_fault_matrix() {
    let (_dir, opened) = dependency_seeded();
    opened
        .engine
        .execute_for_test(
            "PRAGMA ignore_check_constraints=ON; \
             UPDATE _fathomdb_source_dependencies SET schema_version=2 \
             WHERE dependency_id='integrity-dep-1'",
        )
        .unwrap();
    assert_eq!(
        finding_codes(&opened, DataPlaneIntegrityCheckV1::DependencyChain),
        [DataPlaneIntegrityFindingCodeV1::DependencyRowInvalid]
    );
}

#[test]
fn slice55_dependency_chain_missing_owner_is_critical() {
    let (_dir, opened) = dependency_seeded();
    opened
        .engine
        .execute_for_test(
            "DELETE FROM _fathomdb_artifact_revisions \
             WHERE revision_id='integrity-derived-r1'",
        )
        .unwrap();
    assert_eq!(
        finding_codes(&opened, DataPlaneIntegrityCheckV1::DependencyChain),
        [DataPlaneIntegrityFindingCodeV1::DependencyDerivedOwnerMissing]
    );
}

#[test]
fn slice55_no_reverse_row_contract() {
    let (_dir, opened) = opened();
    assert!(!opened
        .engine
        .schema_objects_for_test()
        .unwrap()
        .iter()
        .any(|name| name.contains("reverse")));
}

#[test]
fn slice55_searchable_orphan_finding_matrix() {
    let (_dir, opened) = opened();
    opened
        .engine
        .execute_for_test(
            "INSERT INTO search_index(rowid,body,kind,write_cursor) \
             VALUES(900,'orphan body','doc',900)",
        )
        .unwrap();
    let result = opened
        .engine
        .check_data_plane_integrity(request(
            DataPlaneIntegrityCheckV1::ActiveSearchableOrphans,
            10_000,
        ))
        .unwrap();
    assert_eq!(result.checked_count, 1);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(
        result.findings[0].code,
        DataPlaneIntegrityFindingCodeV1::SearchProjectionOwnerMissing
    );
    assert_eq!(result.findings[0].write_cursor, Some(900));
    assert!(result.findings[0].artifact_revision_ids.is_empty());
}

#[test]
fn slice55_missing_synchronous_projection_matrix() {
    let (_dir, opened) = opened();
    opened.engine.write(&[canonical("missing-fts-r1", "missing-fts", "missing fts body")]).unwrap();
    opened.engine.execute_for_test("DELETE FROM search_index WHERE write_cursor=1").unwrap();
    assert_eq!(
        finding_codes(&opened, DataPlaneIntegrityCheckV1::ActiveSearchableOrphans),
        [DataPlaneIntegrityFindingCodeV1::NodeBodyFtsMissing]
    );
}

#[test]
fn slice55_projection_generation_matrix_reports_missing_current_authority() {
    let (_dir, opened) = opened();
    opened.engine.execute_for_test("DELETE FROM _fathomdb_projection_generation_current").unwrap();
    assert_eq!(
        finding_codes(&opened, DataPlaneIntegrityCheckV1::ProjectionGeneration),
        [DataPlaneIntegrityFindingCodeV1::ProjectionGenerationCorrupt]
    );
}

#[test]
fn slice55_projection_generation_enumerates_and_attributes_corrupt_members() {
    let (_dir, opened) = opened();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    opened.engine.write(&[canonical("generation-member-r1", "generation-member", "body")]).unwrap();
    opened
        .engine
        .execute_for_test(
            "INSERT OR REPLACE INTO _fathomdb_vector_rows(rowid,kind,write_cursor) \
             VALUES(1,'doc',1); DELETE FROM _fathomdb_projection_terminal WHERE write_cursor=1",
        )
        .unwrap();
    let result = opened
        .engine
        .check_data_plane_integrity(request(DataPlaneIntegrityCheckV1::ProjectionGeneration, 3))
        .unwrap();
    assert_eq!(result.checked_count, 3);
    assert_eq!(result.findings.len(), 1);
    let finding = &result.findings[0];
    assert_eq!(finding.code, DataPlaneIntegrityFindingCodeV1::ProjectionMemberCorrupt);
    assert_eq!(finding.write_cursor, Some(1));
    assert_eq!(finding.artifact_revision_ids, ["generation-member-r1"]);
    assert!(finding.projection_generation_id.as_deref().is_some_and(|id| id.starts_with("pgen1:")));
}

#[test]
fn slice55_projection_generation_member_cap_is_all_or_error() {
    let (_dir, opened) = opened();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    opened
        .engine
        .write(&[
            canonical("generation-cap-r1", "generation-cap-1", "one"),
            canonical("generation-cap-r2", "generation-cap-2", "two"),
        ])
        .unwrap();
    let error = opened
        .engine
        .check_data_plane_integrity(request(DataPlaneIntegrityCheckV1::ProjectionGeneration, 3))
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::DataPlaneIntegrity(ref value)
            if value.reason == DataPlaneIntegrityErrorReasonV1::IntegrityBoundExceeded
    ));
}

#[test]
fn slice55_mutation_readiness_receipt_matrix_reports_guarded_corruption() {
    let (_dir, opened) = opened();
    let batch = ActuationBatchV1::new(
        "slice55-readiness",
        vec![ActuationOperationV1::PutCanonicalNode(
            match canonical("receipt-r1", "receipt", "receipt body") {
                PreparedWrite::ProvenancedNode(node) => node,
                _ => unreachable!(),
            },
        )],
    )
    .unwrap();
    opened.engine.actuate(batch).unwrap();
    opened
        .engine
        .execute_for_test(
            "PRAGMA ignore_check_constraints=ON; \
             UPDATE _fathomdb_actuation_receipts SET outcome='not-an-outcome' \
             WHERE operation_id='slice55-readiness'",
        )
        .unwrap();
    assert_eq!(
        finding_codes(&opened, DataPlaneIntegrityCheckV1::MutationReadiness),
        [DataPlaneIntegrityFindingCodeV1::MutationReceiptCorrupt]
    );
}

#[test]
fn slice55_projection_scan_plans_use_indexed_order() {
    let (_dir, opened) = opened();
    let candidates = opened.engine.data_plane_integrity_candidate_queries_for_test();
    assert_eq!(candidates.len(), 7);
    for (table, index) in [
        ("canonical_nodes", "canonical_nodes_write_cursor_idx"),
        ("canonical_edges", "canonical_edges_write_cursor_idx"),
        ("search_index", "search_index"),
        ("search_index_v2", "search_index_v2"),
        ("search_index_edges", "search_index_edges"),
        ("canonical_attributes", "canonical_attributes"),
        ("property_search_index", "property_search_index"),
    ] {
        let sql = candidates
            .iter()
            .find(|sql| sql.contains(&format!("FROM {table}")))
            .unwrap_or_else(|| panic!("missing production candidate query for {table}"));
        assert!(sql.contains(">?1"), "missing after-key bound: {sql}");
        assert!(sql.contains("LIMIT ?2"), "missing remaining+1 bound: {sql}");
        if table.starts_with("canonical_") && !table.ends_with("attributes") {
            assert!(sql.contains(&format!("INDEXED BY {index}")), "missing {index}: {sql}");
        }
    }
    let plans = opened.engine.data_plane_integrity_query_plans_for_test().unwrap();
    for required in [
        "canonical_nodes_write_cursor_idx",
        "canonical_edges_write_cursor_idx",
        "search_index",
        "search_index_v2",
        "search_index_edges",
        "canonical_attributes",
        "property_search_index",
    ] {
        assert!(plans.iter().any(|plan| plan.contains(required)), "missing {required}: {plans:#?}");
    }
    assert!(plans
        .iter()
        .all(|plan| { !plan.contains("USE TEMP B-TREE") && !plan.contains("MATERIALIZE") }));
}

#[test]
fn slice55_projection_registry_cap_precedes_unselected_row_decode() {
    let (_dir, opened) = opened();
    opened
        .engine
        .execute_for_test(
            "INSERT INTO _fathomdb_projection_registry(\
               name,roles,fts_tokenizer,vector_embedder,vector_declared,source) \
             VALUES('a','rankable',NULL,NULL,0,NULL); \
             INSERT INTO _fathomdb_projection_registry(\
               name,roles,fts_tokenizer,vector_embedder,vector_declared,source) \
             VALUES('z','rankable',NULL,NULL,0,'not-json')",
        )
        .unwrap();
    let error = opened
        .engine
        .check_data_plane_integrity(request(DataPlaneIntegrityCheckV1::ActiveSearchableOrphans, 1))
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::DataPlaneIntegrity(ref value)
            if value.reason == DataPlaneIntegrityErrorReasonV1::IntegrityBoundExceeded
    ));
}

#[test]
fn slice55_missing_node_body_fts() {
    let (_dir, opened) = opened();
    opened.engine.write(&[canonical("node-fts-r1", "node-fts", "node fts")]).unwrap();
    opened.engine.execute_for_test("DELETE FROM search_index").unwrap();
    assert_eq!(
        finding_codes(&opened, DataPlaneIntegrityCheckV1::ActiveSearchableOrphans),
        [DataPlaneIntegrityFindingCodeV1::NodeBodyFtsMissing]
    );
}

#[test]
fn slice55_missing_node_body_fts_v2() {
    let (_dir, opened) = opened();
    opened.engine.write(&[canonical("node-fts-v2-r1", "node-fts-v2", "node fts v2")]).unwrap();
    opened.engine.execute_for_test("DELETE FROM search_index_v2").unwrap();
    assert_eq!(
        finding_codes(&opened, DataPlaneIntegrityCheckV1::ActiveSearchableOrphans),
        [DataPlaneIntegrityFindingCodeV1::NodeBodyFtsV2Missing]
    );
}

#[test]
fn slice55_missing_edge_body_fts() {
    let (_dir, opened) = opened();
    opened
        .engine
        .write(&[
            canonical("edge-from-r1", "edge-from", "edge from"),
            canonical("edge-to-r1", "edge-to", "edge to"),
            edge("edge-from", "edge-to", "edge relation"),
        ])
        .unwrap();
    opened.engine.execute_for_test("DELETE FROM search_index_edges").unwrap();
    assert_eq!(
        finding_codes(&opened, DataPlaneIntegrityCheckV1::ActiveSearchableOrphans),
        [DataPlaneIntegrityFindingCodeV1::EdgeBodyFtsMissing]
    );
}

#[test]
fn slice55_missing_canonical_attribute() {
    let (_dir, opened) = opened();
    opened.engine.configure_projections(&[property_spec()], &[]).unwrap();
    opened
        .engine
        .write(&[canonical("attribute-r1", "attribute", r#"{"title":"integrity"}"#)])
        .unwrap();
    opened.engine.execute_for_test("DELETE FROM canonical_attributes").unwrap();
    assert_eq!(
        finding_codes(&opened, DataPlaneIntegrityCheckV1::ActiveSearchableOrphans),
        [DataPlaneIntegrityFindingCodeV1::CanonicalAttributeMissing]
    );
}

#[test]
fn slice55_missing_property_fts() {
    let (_dir, opened) = opened();
    opened.engine.configure_projections(&[property_spec()], &[]).unwrap();
    opened
        .engine
        .write(&[canonical("property-r1", "property", r#"{"title":"integrity"}"#)])
        .unwrap();
    opened.engine.execute_for_test("DELETE FROM property_search_index").unwrap();
    assert_eq!(
        finding_codes(&opened, DataPlaneIntegrityCheckV1::ActiveSearchableOrphans),
        [DataPlaneIntegrityFindingCodeV1::PropertyFtsMissing]
    );
}

#[test]
fn slice55_sparse_attribute_owners_cannot_hide_later_required_members() {
    let (_dir, opened) = opened();
    opened.engine.configure_projections(&[property_spec()], &[]).unwrap();
    let mut writes = (0..200)
        .map(|index| {
            canonical(&format!("sparse-empty-{index}-r1"), &format!("sparse-empty-{index}"), "{}")
        })
        .collect::<Vec<_>>();
    writes.push(canonical("sparse-required-r1", "sparse-required", r#"{"title":"required"}"#));
    opened.engine.write(&writes).unwrap();
    opened
        .engine
        .execute_for_test("DELETE FROM canonical_attributes; DELETE FROM property_search_index")
        .unwrap();

    let result = opened
        .engine
        .check_data_plane_integrity(request(
            DataPlaneIntegrityCheckV1::ActiveSearchableOrphans,
            1_100,
        ))
        .unwrap();
    assert_eq!(
        result.findings.iter().map(|finding| finding.code).collect::<Vec<_>>(),
        [
            DataPlaneIntegrityFindingCodeV1::CanonicalAttributeMissing,
            DataPlaneIntegrityFindingCodeV1::PropertyFtsMissing,
        ]
    );
    assert!(result
        .findings
        .iter()
        .all(|finding| finding.artifact_revision_ids == ["sparse-required-r1"]));
}

#[test]
fn slice55_dense_state_reuses_slice40_classifier() {
    let (_dir, opened) = opened();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    opened.engine.write(&[canonical("dense-r1", "dense", "dense body")]).unwrap();
    opened
        .engine
        .execute_for_test(
            "INSERT OR REPLACE INTO _fathomdb_vector_rows(rowid,kind,write_cursor) \
             VALUES(1,'doc',1); DELETE FROM _fathomdb_projection_terminal WHERE write_cursor=1",
        )
        .unwrap();
    assert_eq!(
        finding_codes(&opened, DataPlaneIntegrityCheckV1::ActiveSearchableOrphans),
        [DataPlaneIntegrityFindingCodeV1::DenseProjectionPartial]
    );
}

#[test]
fn slice55_terminal_only_residue_is_enumerated_by_both_integrity_checks() {
    let (_dir, opened) = opened();
    opened
        .engine
        .execute_for_test(
            "INSERT INTO _fathomdb_projection_terminal(write_cursor,state) \
             VALUES(900,'up_to_date')",
        )
        .unwrap();
    for (check, code) in [
        (
            DataPlaneIntegrityCheckV1::ActiveSearchableOrphans,
            DataPlaneIntegrityFindingCodeV1::DenseProjectionOwnerMissing,
        ),
        (
            DataPlaneIntegrityCheckV1::ProjectionGeneration,
            DataPlaneIntegrityFindingCodeV1::ProjectionMemberCorrupt,
        ),
    ] {
        let result = opened.engine.check_data_plane_integrity(request(check, 10_000)).unwrap();
        assert_eq!(result.findings.len(), 1, "{check:?}: {result:#?}");
        assert_eq!(result.findings[0].code, code);
        assert_eq!(result.findings[0].write_cursor, Some(900));
    }
}

#[test]
fn slice55_projection_generation_error_mapping_uses_real_corrupt_authority() {
    let (_dir, opened) = opened();
    opened
        .engine
        .execute_for_test(
            "DROP TRIGGER _fathomdb_projection_generation_immutable; \
             PRAGMA ignore_check_constraints=ON; \
             UPDATE _fathomdb_projection_generations SET origin='invalid' \
             WHERE role='serving'",
        )
        .unwrap();
    assert_eq!(
        finding_codes(&opened, DataPlaneIntegrityCheckV1::ProjectionGeneration),
        [DataPlaneIntegrityFindingCodeV1::ProjectionGenerationCorrupt]
    );
}
#[test]
fn slice55_mutation_readiness_receipt_matrix() {
    let (_dir, opened) = opened();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    let batch = ActuationBatchV1::new(
        "slice55-readiness-classifier",
        vec![ActuationOperationV1::PutCanonicalNode(
            match canonical("readiness-classifier-r1", "readiness-classifier", "body") {
                PreparedWrite::ProvenancedNode(node) => node,
                _ => unreachable!(),
            },
        )],
    )
    .unwrap();
    opened.engine.actuate(batch).unwrap();
    opened
        .engine
        .execute_for_test(
            "INSERT OR REPLACE INTO _fathomdb_vector_rows(rowid,kind,write_cursor) \
             VALUES(1,'doc',1); DELETE FROM _fathomdb_projection_terminal WHERE write_cursor=1",
        )
        .unwrap();
    assert_eq!(
        finding_codes(&opened, DataPlaneIntegrityCheckV1::MutationReadiness),
        [DataPlaneIntegrityFindingCodeV1::MutationReadinessCorrupt]
    );
}

#[test]
fn slice55_receipt_reserves_pending_work_before_fetch_and_parse() {
    let (_dir, opened) = opened();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    let operations = [("reserve-r1", "reserve-1"), ("reserve-r2", "reserve-2")]
        .into_iter()
        .map(|(revision, logical)| {
            ActuationOperationV1::PutCanonicalNode(match canonical(revision, logical, "body") {
                PreparedWrite::ProvenancedNode(node) => node,
                _ => unreachable!(),
            })
        })
        .collect();
    opened.engine.actuate(ActuationBatchV1::new("slice55-reserve", operations).unwrap()).unwrap();
    opened
        .engine
        .execute_for_test(
            "UPDATE _fathomdb_actuation_receipts \
             SET pending_projection_write_cursors_json='[\"1\",\"not-a-cursor\"]' \
             WHERE operation_id='slice55-reserve'",
        )
        .unwrap();
    let error = opened
        .engine
        .check_data_plane_integrity(request(DataPlaneIntegrityCheckV1::MutationReadiness, 1))
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::DataPlaneIntegrity(ref value)
            if value.reason == DataPlaneIntegrityErrorReasonV1::IntegrityBoundExceeded
    ));
}
#[test]
fn slice55_mutation_readiness_selects_only_bounded_subset() {
    let (_dir, opened) = opened();
    actuate_pending(&opened, "a-receipt", "bounded-a-r1");
    actuate_pending(&opened, "z-receipt", "bounded-z-r1");
    opened
        .engine
        .execute_for_test(
            "PRAGMA ignore_check_constraints=ON; \
             UPDATE _fathomdb_actuation_receipts SET operation_id=zeroblob(129) \
             WHERE operation_id='z-receipt'",
        )
        .unwrap();
    let error = opened
        .engine
        .check_data_plane_integrity(request(DataPlaneIntegrityCheckV1::MutationReadiness, 1))
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::DataPlaneIntegrity(ref value)
            if value.reason == DataPlaneIntegrityErrorReasonV1::IntegrityBoundExceeded
    ));
}

#[test]
fn slice55_receipt_variable_field_guards_precede_fetch() {
    let (_dir, opened) = opened();
    actuate_pending(&opened, "guarded-operation", "guarded-r1");
    opened
        .engine
        .execute_for_test(
            "PRAGMA ignore_check_constraints=ON; \
             UPDATE _fathomdb_actuation_receipts SET operation_id=zeroblob(129) \
             WHERE operation_id='guarded-operation'",
        )
        .unwrap();
    let result = opened
        .engine
        .check_data_plane_integrity(request(DataPlaneIntegrityCheckV1::MutationReadiness, 10))
        .unwrap();
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].code, DataPlaneIntegrityFindingCodeV1::MutationReceiptCorrupt);
    assert!(result.findings[0].operation_id.is_none());
}

#[test]
fn slice55_unrelated_receipt_json_is_out_of_scope() {
    let (_dir, opened) = opened();
    actuate_pending(&opened, "unrelated-json", "unrelated-r1");
    opened
        .engine
        .execute_for_test(
            "PRAGMA ignore_check_constraints=ON; \
             UPDATE _fathomdb_actuation_receipts SET reason_codes_json='not-json' \
             WHERE operation_id='unrelated-json'",
        )
        .unwrap();
    let result = opened
        .engine
        .check_data_plane_integrity(request(DataPlaneIntegrityCheckV1::MutationReadiness, 10))
        .unwrap();
    assert!(result.findings.is_empty(), "{result:#?}");
}

#[test]
fn slice55_receipt_boundary_covers_every_pending_cursor() {
    let (_dir, opened) = opened();
    actuate_pending(&opened, "boundary-receipt", "boundary-r1");
    opened
        .engine
        .execute_for_test(
            "UPDATE _fathomdb_actuation_receipts \
             SET resulting_write_boundary=0,pending_projection_write_cursors_json='[\"1\"]', \
                 projection_generation_id=(SELECT generation_id \
                   FROM _fathomdb_projection_generation_current WHERE singleton=1) \
             WHERE operation_id='boundary-receipt'",
        )
        .unwrap();
    assert_eq!(
        finding_codes(&opened, DataPlaneIntegrityCheckV1::MutationReadiness),
        [DataPlaneIntegrityFindingCodeV1::MutationReceiptCorrupt]
    );
}

#[test]
fn slice55_receipt_generation_must_be_current_authority() {
    let (_dir, opened) = opened();
    actuate_pending(&opened, "generation-receipt", "generation-r1");
    opened
        .engine
        .execute_for_test(
            "UPDATE _fathomdb_actuation_receipts \
             SET pending_projection_write_cursors_json='[\"1\"]', \
                 projection_generation_id='pgen1:11111111111111111111111111111111' \
             WHERE operation_id='generation-receipt'",
        )
        .unwrap();
    assert_eq!(
        finding_codes(&opened, DataPlaneIntegrityCheckV1::MutationReadiness),
        [DataPlaneIntegrityFindingCodeV1::MutationReceiptCorrupt]
    );
}

#[test]
fn slice55_integrity_receipt_malformed_oversized_and_cap_plus_one() {
    let (_dir, opened) = opened();
    actuate_pending(&opened, "oversized-receipt", "oversized-r1");
    opened
        .engine
        .execute_for_test(
            "PRAGMA ignore_check_constraints=ON; \
             UPDATE _fathomdb_actuation_receipts \
             SET pending_projection_write_cursors_json='[' || quote(printf('%02950d',0)) || ']' \
             WHERE operation_id='oversized-receipt'",
        )
        .unwrap();
    assert_eq!(
        finding_codes(&opened, DataPlaneIntegrityCheckV1::MutationReadiness),
        [DataPlaneIntegrityFindingCodeV1::MutationReceiptCorrupt]
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(8))]
    #[test]
    fn slice55_receipt_boundary_property(
        count in 1u8..=4,
        boundary in 0u8..4,
    ) {
        prop_assume!(boundary < count);
        let (_dir, opened) = opened();
        opened.engine.configure_vector_kind_for_test("doc").unwrap();
        let operations = (0..count)
            .map(|index| {
                let revision = format!("property-boundary-r{index}");
                let logical = format!("property-boundary-{index}");
                ActuationOperationV1::PutCanonicalNode(match canonical(
                    &revision,
                    &logical,
                    "property pending body",
                ) {
                    PreparedWrite::ProvenancedNode(node) => node,
                    _ => unreachable!(),
                })
            })
            .collect();
        opened.engine.actuate(
            ActuationBatchV1::new("property-receipt", operations).unwrap()
        ).unwrap();
        let pending = (1..=count)
            .map(|cursor| format!("\"{cursor}\""))
            .collect::<Vec<_>>()
            .join(",");
        opened.engine.execute_for_test(&format!(
            "UPDATE _fathomdb_actuation_receipts \
             SET resulting_write_boundary={boundary}, \
                 pending_projection_write_cursors_json='[{pending}]', \
                 projection_generation_id=(SELECT generation_id \
                   FROM _fathomdb_projection_generation_current WHERE singleton=1) \
             WHERE operation_id='property-receipt'"
        )).unwrap();
        let result = opened.engine.check_data_plane_integrity(request(
            DataPlaneIntegrityCheckV1::MutationReadiness, 10_000,
        )).unwrap();
        prop_assert_eq!(result.findings.len(), 1);
        prop_assert_eq!(
            result.findings[0].code,
            DataPlaneIntegrityFindingCodeV1::MutationReceiptCorrupt
        );
    }
}
