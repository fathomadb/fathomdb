//! Slice 55 RED contract for bounded operator data-plane integrity.

use fathomdb_engine::{
    DataPlaneIntegrityCheckV1, DataPlaneIntegrityErrorReasonV1, DataPlaneIntegrityRequestV1,
    Engine, EngineError,
};
use fathomdb_schema::SQLITE_SUFFIX;
use proptest::prelude::*;
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
    assert_eq!(result.checked_count, result.check_counts.iter().map(|c| c.checked_count).sum());
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
    let (_dir, opened) = opened();
    let result = opened
        .engine
        .check_data_plane_integrity(request(DataPlaneIntegrityCheckV1::DependencyChain, 10_000))
        .unwrap();
    assert!(result.findings.is_empty());
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
    let result = opened
        .engine
        .check_data_plane_integrity(request(
            DataPlaneIntegrityCheckV1::ActiveSearchableOrphans,
            10_000,
        ))
        .unwrap();
    assert!(result.complete);
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
clean_projection_case!(
    slice55_missing_node_body_fts,
    DataPlaneIntegrityCheckV1::ActiveSearchableOrphans
);
clean_projection_case!(
    slice55_missing_node_body_fts_v2,
    DataPlaneIntegrityCheckV1::ActiveSearchableOrphans
);
clean_projection_case!(
    slice55_missing_edge_body_fts,
    DataPlaneIntegrityCheckV1::ActiveSearchableOrphans
);
clean_projection_case!(
    slice55_missing_canonical_attribute,
    DataPlaneIntegrityCheckV1::ActiveSearchableOrphans
);
clean_projection_case!(
    slice55_missing_property_fts,
    DataPlaneIntegrityCheckV1::ActiveSearchableOrphans
);
clean_projection_case!(
    slice55_dense_state_reuses_slice40_classifier,
    DataPlaneIntegrityCheckV1::ProjectionGeneration
);
clean_projection_case!(
    slice55_projection_generation_error_mapping,
    DataPlaneIntegrityCheckV1::ProjectionGeneration
);
clean_projection_case!(
    slice55_mutation_readiness_receipt_matrix,
    DataPlaneIntegrityCheckV1::MutationReadiness
);
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
