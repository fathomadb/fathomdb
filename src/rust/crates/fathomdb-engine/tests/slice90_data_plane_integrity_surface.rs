#[cfg(feature = "operator")]
use fathomdb_engine::{
    DataPlaneIntegrityRequestV1, DataPlaneIntegrityResultV1, Engine, EngineError,
};

#[cfg(feature = "operator")]
#[test]
fn data_plane_integrity_is_available_with_operator() {
    let _: fn(
        &Engine,
        DataPlaneIntegrityRequestV1,
    ) -> Result<DataPlaneIntegrityResultV1, EngineError> = Engine::check_data_plane_integrity;
}
