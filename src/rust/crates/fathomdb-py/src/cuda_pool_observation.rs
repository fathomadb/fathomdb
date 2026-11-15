use super::*;

/// Native-only passive pool JSON; no application wrapper or model ownership.
#[pyfunction(name = "_cuda_pool_observation_for_qualification")]
pub(super) fn observe_pool(py: Python<'_>) -> PyResult<String> {
    call_engine(py, RustEngine::cuda_pool_observation_for_qualification)
}
