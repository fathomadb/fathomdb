//! Process-wide record of CUDA driver initialisation on aarch64 Linux.
//!
//! On Jetson (measured on the AGX Orin 64 GB, L4T R36, CUDA 12.6) `cuInit`
//! reserves one large range of the process address space inside
//! [8 GiB, 128 GiB) and returns `CUDA_ERROR_OUT_OF_MEMORY` when no hole of at
//! least 4 GiB is left there. A JavaScript heap can fragment that window, so
//! the Node addon initialises the driver when it is registered, and a later
//! forced-CUDA refusal needs to know whether `cuInit` was the call that failed.
//! The typed probe reasons cannot carry that detail without changing their
//! public shape, so each caller's most recent `cuInit` outcome is kept here.
//! A refusal reads the most recent outcome of its own caller kind, so another
//! caller's later `cuInit` cannot change it. A later probe of the same kind
//! does replace it: the reranker's refusal is memoized, but its probe runs
//! again at every engine open, so that refusal can lose its cause.
//!
//! Binding support, not an SDK surface: the public items are
//! `#[doc(hidden)]` and unstable (`dev/interfaces/rust.md`). The driver calls
//! and the record compile only for aarch64 Linux builds with `embed-cuda` or
//! `rerank-cuda`, so every such build (the Node addon, the Tegra Python
//! wheel, the CLI) contains them; only the Node addon reads them. The record
//! type alone is also compiled for unit tests everywhere.

#[cfg(all(
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
use std::sync::{Mutex, PoisonError};

#[cfg(all(
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
use candle_core::cuda::cudarc::driver::{result, sys, DriverError};

/// The outcome of one attempt to initialise the CUDA driver.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CudaDriverInit {
    /// No CUDA driver library could be loaded, so `cuInit` was not called.
    DriverLibraryAbsent,
    /// `cuInit(0)` returned `CUDA_SUCCESS`.
    Initialized,
    /// `cuInit(0)` failed with this raw `CUresult` value.
    Failed(u32),
}

impl CudaDriverInit {
    /// The raw `CUresult` of the `cuInit` call: `Some(0)` on success, `None`
    /// when the driver library was absent and `cuInit` was never called.
    #[must_use]
    pub const fn cu_result(self) -> Option<u32> {
        match self {
            Self::DriverLibraryAbsent => None,
            Self::Initialized => Some(0),
            Self::Failed(code) => Some(code),
        }
    }
}

/// Who attempted a `cuInit`.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CudaInitCaller {
    /// The Node addon's registration hook ([`initialize_cuda_driver`]).
    ModuleLoad,
    /// The default embedder's CUDA probe.
    EmbedderProbe,
    /// The cross-encoder reranker's CUDA probe.
    RerankerProbe,
}

/// The most recent outcome overall and per caller.
#[derive(Debug)]
pub(crate) struct InitRecord {
    last: Option<CudaDriverInit>,
    module_load: Option<CudaDriverInit>,
    embedder_probe: Option<CudaDriverInit>,
    reranker_probe: Option<CudaDriverInit>,
}

impl InitRecord {
    pub(crate) const fn new() -> Self {
        Self { last: None, module_load: None, embedder_probe: None, reranker_probe: None }
    }

    pub(crate) fn record(&mut self, caller: CudaInitCaller, outcome: CudaDriverInit) {
        self.last = Some(outcome);
        *self.slot(caller) = Some(outcome);
    }

    pub(crate) const fn last(&self) -> Option<CudaDriverInit> {
        self.last
    }

    pub(crate) fn seen_by(&self, caller: CudaInitCaller) -> Option<CudaDriverInit> {
        match caller {
            CudaInitCaller::ModuleLoad => self.module_load,
            CudaInitCaller::EmbedderProbe => self.embedder_probe,
            CudaInitCaller::RerankerProbe => self.reranker_probe,
        }
    }

    fn slot(&mut self, caller: CudaInitCaller) -> &mut Option<CudaDriverInit> {
        match caller {
            CudaInitCaller::ModuleLoad => &mut self.module_load,
            CudaInitCaller::EmbedderProbe => &mut self.embedder_probe,
            CudaInitCaller::RerankerProbe => &mut self.reranker_probe,
        }
    }
}

#[cfg(all(
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
static RECORD: Mutex<InitRecord> = Mutex::new(InitRecord::new());

#[cfg(all(
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
fn record(caller: CudaInitCaller, outcome: CudaDriverInit) -> CudaDriverInit {
    RECORD.lock().unwrap_or_else(PoisonError::into_inner).record(caller, outcome);
    outcome
}

/// Records the result of a `cuInit` call made by `caller`.
#[cfg(all(
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
pub(crate) fn record_init_result(
    caller: CudaInitCaller,
    init: &Result<(), DriverError>,
) -> CudaDriverInit {
    record(
        caller,
        match init {
            Ok(()) => CudaDriverInit::Initialized,
            Err(DriverError(code)) => CudaDriverInit::Failed(*code as u32),
        },
    )
}

/// Records that `caller` found no driver library to load.
#[cfg(all(
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
pub(crate) fn record_driver_library_absent(caller: CudaInitCaller) {
    record(caller, CudaDriverInit::DriverLibraryAbsent);
}

/// Loads the CUDA driver library if present and calls `cuInit(0)`, recording
/// the outcome as [`CudaInitCaller::ModuleLoad`].
///
/// Never panics on a missing driver: presence is checked before any driver
/// symbol is called. `cuInit` is process-wide and idempotent. A failed call
/// was not sticky on the measured Jetson (it succeeded once address space was
/// freed), so a later probe calls it again rather than trusting this result.
#[doc(hidden)]
#[cfg(all(
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
pub fn initialize_cuda_driver() -> CudaDriverInit {
    // SAFETY: only attempts to locate the CUDA driver shared library; no
    // driver symbol is invoked unless it was found.
    if !unsafe { sys::is_culib_present() } {
        return record(CudaInitCaller::ModuleLoad, CudaDriverInit::DriverLibraryAbsent);
    }
    record_init_result(CudaInitCaller::ModuleLoad, &result::init())
}

/// The outcome of the most recent `cuInit` attempt in this process by any
/// caller. `None` when none has run.
#[doc(hidden)]
#[must_use]
#[cfg(all(
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
pub fn last_cuda_driver_init() -> Option<CudaDriverInit> {
    RECORD.lock().unwrap_or_else(PoisonError::into_inner).last()
}

/// The outcome of the most recent `cuInit` attempt made by `caller`. A
/// forced-CUDA refusal reads its own probe's entry, which a later call by
/// another caller does not overwrite.
#[doc(hidden)]
#[must_use]
#[cfg(all(
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
pub fn cuda_driver_init_seen_by(caller: CudaInitCaller) -> Option<CudaDriverInit> {
    RECORD.lock().unwrap_or_else(PoisonError::into_inner).seen_by(caller)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OUT_OF_MEMORY: u32 = 2;

    #[test]
    fn a_probe_keeps_the_cu_init_it_saw_when_another_caller_succeeds_later() {
        // A refusal's cause is the cuInit its own caller kind saw last, not
        // whatever any caller ran last.
        let mut record = InitRecord::new();
        record.record(CudaInitCaller::RerankerProbe, CudaDriverInit::Failed(OUT_OF_MEMORY));
        record.record(CudaInitCaller::EmbedderProbe, CudaDriverInit::Initialized);
        assert_eq!(
            record.seen_by(CudaInitCaller::RerankerProbe),
            Some(CudaDriverInit::Failed(OUT_OF_MEMORY))
        );
        assert_eq!(
            record.seen_by(CudaInitCaller::EmbedderProbe),
            Some(CudaDriverInit::Initialized)
        );
        assert_eq!(record.seen_by(CudaInitCaller::ModuleLoad), None);
        assert_eq!(record.last(), Some(CudaDriverInit::Initialized));
    }

    #[test]
    fn each_caller_keeps_only_its_most_recent_outcome() {
        let mut record = InitRecord::new();
        record.record(CudaInitCaller::EmbedderProbe, CudaDriverInit::Failed(OUT_OF_MEMORY));
        record.record(CudaInitCaller::EmbedderProbe, CudaDriverInit::Initialized);
        assert_eq!(
            record.seen_by(CudaInitCaller::EmbedderProbe),
            Some(CudaDriverInit::Initialized)
        );
        record.record(CudaInitCaller::ModuleLoad, CudaDriverInit::DriverLibraryAbsent);
        assert_eq!(record.last(), Some(CudaDriverInit::DriverLibraryAbsent));
        assert_eq!(CudaDriverInit::DriverLibraryAbsent.cu_result(), None);
        assert_eq!(CudaDriverInit::Initialized.cu_result(), Some(0));
        assert_eq!(CudaDriverInit::Failed(OUT_OF_MEMORY).cu_result(), Some(OUT_OF_MEMORY));
    }
}
