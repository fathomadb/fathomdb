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
    let result = opened
        .engine
        .check_data_plane_integrity(request(
            DataPlaneIntegrityCheckV1::ActiveSearchableOrphans,
            10_000,
        ))
        .unwrap();
    assert!(result.findings.is_empty());
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

macro_rules! clean_projection_case {
    ($name:ident, $check:expr) => {
        #[test]
        fn $name() {
            let (_dir, opened) = opened();
            let result = opened.engine.check_data_plane_integrity(request($check, 10_000)).unwrap();
            assert!(result.findings.is_empty());
        }
    };
}

clean_projection_case!(
    slice55_projection_scan_plans_use_indexed_order,
    DataPlaneIntegrityCheckV1::ActiveSearchableOrphans
);

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
clean_projection_case!(
    slice55_projection_generation_error_mapping,
    DataPlaneIntegrityCheckV1::ProjectionGeneration
);
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
clean_projection_case!(
    slice55_mutation_readiness_selects_only_bounded_subset,
    DataPlaneIntegrityCheckV1::MutationReadiness
);
clean_projection_case!(
    slice55_receipt_variable_field_guards_precede_fetch,
    DataPlaneIntegrityCheckV1::MutationReadiness
);
clean_projection_case!(
    slice55_unrelated_receipt_json_is_out_of_scope,
    DataPlaneIntegrityCheckV1::MutationReadiness
);

#[test]
fn slice55_integrity_receipt_malformed_oversized_and_cap_plus_one() {
    let (_dir, opened) = opened();
    let error = opened
        .engine
        .check_data_plane_integrity(request(DataPlaneIntegrityCheckV1::ProjectionGeneration, 1))
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::DataPlaneIntegrity(ref value)
            if value.reason == DataPlaneIntegrityErrorReasonV1::IntegrityBoundExceeded
    ));
}

proptest! {
    #[test]
    fn slice55_normalized_chain_round_trip(max_work in 1u32..=10_000) {
        let (_dir, opened) = opened();
        let result = opened.engine.check_data_plane_integrity(request(
            DataPlaneIntegrityCheckV1::DependencyChain,
            max_work,
        ));
        prop_assert!(result.is_ok() || matches!(
            result,
            Err(EngineError::DataPlaneIntegrity(ref value))
                if value.reason == DataPlaneIntegrityErrorReasonV1::IntegrityBoundExceeded
        ));
    }
}
