use super::*;

/// Native-only passive pool JSON; identities never cross JavaScript as numbers.
#[napi(js_name = "cudaPoolObservationForQualification")]
pub fn observe_pool() -> Result<String> {
    call_engine_sync(|| {
        fathomdb_engine::Engine::cuda_pool_observation_for_qualification()
            .map_err(engine_error_to_napi)
    })
}
