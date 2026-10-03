use fathomdb_engine::{
    ProjectionRuntimeStatus, ProjectionRuntimeStatusEntry, ProjectionRuntimeUnavailabilityReason,
    ProjectionStatusDenseReadiness,
};

#[test]
fn runtime_status_carriers_keep_their_root_public_paths() {
    for name in [
        std::any::type_name::<ProjectionRuntimeStatus>(),
        std::any::type_name::<ProjectionRuntimeStatusEntry>(),
        std::any::type_name::<ProjectionRuntimeUnavailabilityReason>(),
        std::any::type_name::<ProjectionStatusDenseReadiness>(),
    ] {
        assert!(name.contains("projection_runtime::"), "{name}");
    }
    assert_eq!(ProjectionRuntimeUnavailabilityReason::NoRuntime.as_str(), "no_runtime");
    assert_eq!(ProjectionStatusDenseReadiness::NotDeclared.as_str(), "not_declared");
}
