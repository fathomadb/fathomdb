use fathomdb_engine::{RebuildKind, RebuildReport};

#[test]
fn rebuild_contract_types_remain_public_without_operator() {
    assert!(std::any::type_name::<RebuildKind>().contains("projection_rebuild::"));
    assert!(std::any::type_name::<RebuildReport>().contains("projection_rebuild::"));
    let report = RebuildReport {
        kind: RebuildKind::Vec0,
        rows_invalidated: 2,
        rows_rebuilt: 1,
        projection_cursor_after: 3,
    };
    assert_eq!(report.kind, RebuildKind::Vec0);
    assert_eq!(report.rows_invalidated, 2);
}
