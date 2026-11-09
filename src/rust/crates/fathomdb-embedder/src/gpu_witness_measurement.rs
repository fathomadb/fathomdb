use crate::gpu_witness::{ControlAllocationObservation, SOLE_GPU_CONSUMER_PRECONDITION};

/// The per-pool observation window required by the private USED method.
pub const PRIVATE_POOL_EXCLUSIVITY_PRECONDITION: &str = "no competing allocations or frees may occur in the witnessed private pool during model and control observation";

/// Method-specific raw evidence in a current GPU allocation witness.
///
/// Exactly one variant is retained. Global free memory decreases on allocation;
/// live private-pool USED increases. Neither method substitutes for the other.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuAllocationMeasurement {
    /// Discrete-device global counter and its bounded raw-allocation control.
    GlobalFree {
        /// Bytes requested per raw control block.
        control_allocation_request_bytes: u64,
        /// Number of resident blocks, including the decisive one.
        control_block_count: usize,
        /// Global free bytes immediately before the decisive block.
        control_free_before_bytes: u64,
        /// Global free bytes while the decisive block is resident.
        control_free_after_bytes: u64,
        /// Signed before-minus-after control movement.
        control_delta_bytes: i128,
        /// Canonical selected-device exclusivity precondition.
        sole_gpu_consumer_precondition: String,
    },
    /// Integrated-device live allocation accounting from its held private pool.
    PrivatePoolUsed {
        /// Checked uint64 CUDA context ID, represented losslessly in decimal.
        context_id: String,
        /// Opaque identity of the retained private pool in this process.
        pool_identity: String,
        /// Checked current USED before loading the model.
        used_before_bytes: u64,
        /// Checked current USED after model-load completion.
        used_after_bytes: u64,
        /// Signed after-minus-before model movement.
        delta_bytes: i128,
        /// Exact touched control size, 64 MiB.
        control_request_bytes: u64,
        /// Post-forward current USED before the same-pool control.
        control_before_bytes: u64,
        /// Current USED while that control is live and complete.
        control_live_bytes: u64,
        /// Current USED after releasing only that control and completing.
        control_released_bytes: u64,
        /// Canonical same-pool exclusivity precondition.
        pool_exclusivity_precondition: String,
    },
}

impl GpuAllocationMeasurement {
    /// Retain a raw global control without changing its observations.
    #[must_use]
    pub fn from_global_control(control: ControlAllocationObservation) -> Self {
        Self::GlobalFree {
            control_allocation_request_bytes: control.requested_bytes,
            control_block_count: control.block_count,
            control_free_before_bytes: control.free_before_bytes,
            control_free_after_bytes: control.free_after_bytes,
            control_delta_bytes: control.delta_bytes(),
            sole_gpu_consumer_precondition: SOLE_GPU_CONSUMER_PRECONDITION.to_owned(),
        }
    }

    /// Stable tag used by canonical records and both SDK tagged unions.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::GlobalFree { .. } => "global_free",
            Self::PrivatePoolUsed { .. } => "private_pool_used",
        }
    }
}
