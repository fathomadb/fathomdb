use fathomdb_engine::{DependencyTraceRequestV1, DependencyTraceResultV1, Engine, EngineError};

#[test]
fn trace_dependency_is_available_without_features() {
    let _: fn(&Engine, DependencyTraceRequestV1) -> Result<DependencyTraceResultV1, EngineError> =
        Engine::trace_dependency;
}
