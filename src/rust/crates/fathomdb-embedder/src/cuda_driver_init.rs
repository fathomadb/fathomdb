//! Process-wide record of CUDA driver initialisation on aarch64 Linux.
//!
//! On Jetson (measured on the AGX Orin 64 GB, L4T R36, CUDA 12.6) `cuInit`
//! reserves one large range of the process address space inside
//! [8 GiB, 128 GiB) and returns `CUDA_ERROR_OUT_OF_MEMORY` when no hole of at
//! least 4 GiB is left there. A JavaScript heap can fragment that window, so
//! the Node addon initialises the driver when it is loaded, and a later
//! forced-CUDA refusal needs to know whether `cuInit` was the call that failed.
//! The typed probe reasons cannot carry that detail without changing their
//! public shape, so the most recent `cuInit` outcome is kept here instead.
//!
//! This is binding support, not an SDK surface. It is compiled only for
//! aarch64 Linux builds with `embed-cuda` or `rerank-cuda`.

use std::sync::{Mutex, PoisonError};

use candle_core::cuda::cudarc::driver::{result, sys, DriverError};

/// The outcome of one attempt to initialise the CUDA driver.
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

static LAST_INIT: Mutex<Option<CudaDriverInit>> = Mutex::new(None);

fn record(outcome: CudaDriverInit) -> CudaDriverInit {
    *LAST_INIT.lock().unwrap_or_else(PoisonError::into_inner) = Some(outcome);
    outcome
}

/// Records the result of a `cuInit` call.
pub(crate) fn record_init_result(init: &Result<(), DriverError>) -> CudaDriverInit {
    record(match init {
        Ok(()) => CudaDriverInit::Initialized,
        Err(DriverError(code)) => CudaDriverInit::Failed(*code as u32),
    })
}

/// Records that a CUDA probe found no driver library to load.
pub(crate) fn record_driver_library_absent() {
    record(CudaDriverInit::DriverLibraryAbsent);
}

/// Loads the CUDA driver library if present and calls `cuInit(0)`, recording
/// the outcome for [`last_cuda_driver_init`].
///
/// Never panics on a missing driver: presence is checked before any driver
/// symbol is called. `cuInit` is process-wide and idempotent. A failed call
/// was not sticky on the measured Jetson (it succeeded once address space was
/// freed), so a later probe calls it again rather than trusting this result.
pub fn initialize_cuda_driver() -> CudaDriverInit {
    // SAFETY: only attempts to locate the CUDA driver shared library; no
    // driver symbol is invoked unless it was found.
    if !unsafe { sys::is_culib_present() } {
        return record(CudaDriverInit::DriverLibraryAbsent);
    }
    record_init_result(&result::init())
}

/// The outcome of the most recent `cuInit` attempt in this process, made by
/// [`initialize_cuda_driver`] or by a default embedder or reranker CUDA probe.
/// `None` when none has run.
#[must_use]
pub fn last_cuda_driver_init() -> Option<CudaDriverInit> {
    *LAST_INIT.lock().unwrap_or_else(PoisonError::into_inner)
}
