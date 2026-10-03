//! Fixed, engine-owned provider dispatch. This module owns no database state.

mod core;
pub(crate) use core::*;
#[cfg(feature = "test-hooks")]
pub(crate) mod d27_observation;

#[cfg(feature = "test-hooks")]
use crate::D27Observation;
use crate::Engine;
#[cfg(feature = "test-hooks")]
use std::time::Instant;

pub(crate) const DEFAULT_EMBED_TIMEOUT_MS: u64 = 30_000;

impl Engine {
    /// Start the private D27 collector after the warm-up drain.
    #[cfg(feature = "test-hooks")]
    pub fn begin_d27_observation_for_test(&self, origin: Instant) {
        self.embed_dispatch.begin_d27_observation(origin);
    }

    /// Run one measured foreground operation under its engine dispatch owner.
    #[cfg(feature = "test-hooks")]
    pub fn with_d27_foreground_owner_for_test<R>(
        &self,
        sequence: usize,
        work: impl FnOnce() -> R,
    ) -> R {
        d27_observation::with_owner(
            Some(d27_observation::Owner::Foreground { operation_sequence: sequence }),
            work,
        )
    }

    /// Snapshot the engine-owned D27 records after measured projection drain.
    #[cfg(feature = "test-hooks")]
    pub fn d27_observation_for_test(&self) -> Option<D27Observation> {
        self.embed_dispatch.d27_observation(
            self.resolved_config.scheduler_runtime_threads,
            self.resolved_config.embedder_pool_size,
        )
    }
}
