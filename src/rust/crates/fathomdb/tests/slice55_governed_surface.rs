//! Slice 55 facade and surface-separation contract.

use fathomdb::{
    DependencyTraceDirectionV1, DependencyTraceRequestV1, DependencyTraceResultV1, Engine,
};

#[cfg(feature = "operator")]
use fathomdb::DataPlaneIntegrityRequestV1;

#[test]
fn slice55_governed_and_operator_surfaces_are_separate() {
    let _trace_request: Option<DependencyTraceRequestV1> = None;
    let _trace_result: Option<DependencyTraceResultV1> = None;
    let _trace_direction = DependencyTraceDirectionV1::ToSource;
    #[cfg(feature = "operator")]
    let _operator_request: Option<DataPlaneIntegrityRequestV1> = None;
    let _governed: fn(&Engine, DependencyTraceRequestV1) -> _ = Engine::trace_dependency;
}

#[test]
fn slice55_default_facade_source_does_not_reexport_operator_integrity_types() {
    let source = include_str!("../src/lib.rs");
    let always_present = source
        .split("#[cfg(feature = \"operator\")]")
        .next()
        .expect("facade has an operator boundary");
    assert!(
        !always_present.contains("DataPlaneIntegrityRequestV1"),
        "operator-only request leaked through the default facade re-export block"
    );
}
