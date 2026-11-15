//! Optional checked pool observations from already-published tracked ownership.

/// Cached first-loss details; collecting these fields never queries the driver.
#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CudaPoolObservationError {
    /// The source domain of the cached error.
    pub domain: &'static str,
    /// Actual raw status when recorded, never a fabricated projection.
    pub code: Option<u32>,
    /// Already-recorded diagnostic text.
    pub message: String,
    /// Actual observed context identity, if it was recorded.
    pub observed_context_id: Option<u64>,
    /// The operation associated with the first loss.
    pub operation: Option<String>,
}

/// One passive process-pool observation, without engine/model ownership.
#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CudaPoolObservation {
    /// Named success/unavailable state; only `private` permits current counters.
    pub status: &'static str,
    /// Current process identifier.
    pub pid: u32,
    /// Ordinal from the existing private decision, if available.
    pub device_ordinal: Option<u32>,
    /// Checked device UUID; unavailable without actual tracked identity reads.
    pub uuid: Option<String>,
    /// Context identity already recorded in the decision.
    pub context_id: Option<u64>,
    /// Opaque process-local pool identity, if checked.
    pub pool_id: Option<String>,
    /// Existing model count before observation; zero is meaningful metadata.
    pub live_models_before: Option<usize>,
    /// Existing model count after observation; equality does not exclude ABA.
    pub live_models_after: Option<usize>,
    /// Successful current USED bytes; unavailable is never filled with zero.
    pub used_current_bytes: Option<u64>,
    /// Successful current RESERVED bytes; unavailable is never filled with zero.
    pub reserved_current_bytes: Option<u64>,
    /// Configured private pool cap already recorded by the decision.
    pub configured_max_size_bytes: Option<u64>,
    /// Configured `0` or `max` threshold, if known.
    pub release_threshold: Option<&'static str>,
    /// Recorded fallback or unavailable reason.
    pub reason: Option<String>,
    /// Cached first loss or an actual checked-read error, if available.
    pub error: Option<CudaPoolObservationError>,
}

impl CudaPoolObservation {
    pub(crate) fn unavailable(status: &'static str) -> Self {
        Self {
            status,
            pid: std::process::id(),
            device_ordinal: None,
            uuid: None,
            context_id: None,
            pool_id: None,
            live_models_before: None,
            live_models_after: None,
            used_current_bytes: None,
            reserved_current_bytes: None,
            configured_max_size_bytes: None,
            release_threshold: None,
            reason: None,
            error: None,
        }
    }
}

/// Inspect already-decided ownership with checked native-token admission.
///
/// A tracked private decision reads current counters only while admitted. Loss
/// or a failed read returns unavailable counters with cached/actual error details.
/// UUID and context identity are cached; pool identity is opaque and never
/// dereferenced. This function never creates a token/context, initializes CUDA,
/// retains native ownership, allocates GPU memory, synchronizes, trims or resets
/// counters. Differing live-model counts invalidate a sample; equality does not
/// exclude intervening close/open activity.
#[cfg(feature = "cuda-pool-observer")]
#[doc(hidden)]
pub fn observe_cuda_pool_for_qualification() -> CudaPoolObservation {
    crate::cuda_pool_policy::observe_pool()
}
