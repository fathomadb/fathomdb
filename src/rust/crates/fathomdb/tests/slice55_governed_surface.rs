//! Slice 55 facade and surface-separation contract.

use fathomdb::{
    DataPlaneIntegrityRequestV1, DependencyTraceDirectionV1, DependencyTraceRequestV1,
    DependencyTraceResultV1, Engine,
};

#[test]
fn slice55_governed_and_operator_surfaces_are_separate() {
    let _trace_request: Option<DependencyTraceRequestV1> = None;
    let _trace_result: Option<DependencyTraceResultV1> = None;
    let _trace_direction = DependencyTraceDirectionV1::ToSource;
    let _operator_request: Option<DataPlaneIntegrityRequestV1> = None;
    let _governed: fn(&Engine, DependencyTraceRequestV1) -> _ = Engine::trace_dependency;
}
