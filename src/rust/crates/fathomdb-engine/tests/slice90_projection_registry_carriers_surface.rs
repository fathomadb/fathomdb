use fathomdb_engine::{
    DenseReadiness, ProjectionDelta, ProjectionFts, ProjectionRole, ProjectionSpec,
    ProjectionVector,
};

#[test]
fn registry_contract_carriers_keep_their_root_public_paths() {
    for name in [
        std::any::type_name::<DenseReadiness>(),
        std::any::type_name::<ProjectionDelta>(),
        std::any::type_name::<ProjectionFts>(),
        std::any::type_name::<ProjectionRole>(),
        std::any::type_name::<ProjectionSpec>(),
        std::any::type_name::<ProjectionVector>(),
    ] {
        assert!(name.contains("projection_registry::"), "{name}");
    }
    assert_eq!(ProjectionRole::from_str_opt("searchable"), Some(ProjectionRole::Searchable));
    assert_eq!(ProjectionRole::Searchable.as_str(), "searchable");
    assert_eq!(DenseReadiness::from_str_opt("ready"), Some(DenseReadiness::Ready));
    assert_eq!(DenseReadiness::Ready.as_str(), "ready");
}
