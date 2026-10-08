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
//!
//! The module-load early `cuInit` itself is decided here too, so that every
//! binding that runs it at load (the Node addon, the Tegra Python wheel)
//! applies the same opt-out and CPU-only skip. Its outcome is kept as a
//! [`ModuleLoadInit`], which the private memory pool's decision reads.

// Off aarch64 Linux CUDA builds only the load-time decision and the
// `not_at_load` reader are live; the record types stay compiled for their
// tests.
#![cfg_attr(
    not(all(
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    )),
    allow(dead_code)
)]

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

use crate::cuda_pool_policy::ModuleLoadInit;
use crate::{EmbedDevicePolicy, RerankerDevicePolicy};

/// The environment variable that turns the module-load `cuInit` off while
/// keeping CUDA: open then initialises the driver as before.
#[doc(hidden)]
pub const ENV_CUDA_EARLY_INIT: &str = "FATHOMDB_CUDA_EARLY_INIT";

/// Whether loading a binding should initialise the CUDA driver.
///
/// Only an exact `cpu` policy rules CUDA out, using the same parsers as open:
/// unset means `auto`, and a malformed value is refused at open rather than
/// here. A component built without CUDA cannot use it whatever its policy,
/// so each caller passes its own compiled flags.
#[doc(hidden)]
#[must_use]
pub fn early_cuda_init_wanted(
    embed_cuda_compiled: bool,
    rerank_cuda_compiled: bool,
    embed_policy: Option<&str>,
    rerank_policy: Option<&str>,
) -> bool {
    let embed_cpu = embed_policy
        .is_some_and(|raw| matches!(raw.parse::<EmbedDevicePolicy>(), Ok(EmbedDevicePolicy::Cpu)));
    let rerank_cpu = rerank_policy.is_some_and(|raw| {
        matches!(raw.parse::<RerankerDevicePolicy>(), Ok(RerankerDevicePolicy::Cpu))
    });
    (embed_cuda_compiled && !embed_cpu) || (rerank_cuda_compiled && !rerank_cpu)
}

/// Whether `FATHOMDB_CUDA_EARLY_INIT` opts out. Only the exact value `off`
/// does; a binding cannot report a malformed value while it loads, so
/// anything else keeps the default.
#[doc(hidden)]
#[must_use]
pub fn early_cuda_init_opted_out(raw: Option<&str>) -> bool {
    raw == Some("off")
}

/// The module-load outcome: `init` runs only when not opted out and wanted.
pub(crate) fn module_load_outcome(
    opted_out: bool,
    wanted: bool,
    init: impl FnOnce() -> CudaDriverInit,
) -> ModuleLoadInit {
    if opted_out {
        return ModuleLoadInit::OptedOut;
    }
    if !wanted {
        return ModuleLoadInit::SkippedCpuOnly;
    }
    match init() {
        CudaDriverInit::Initialized => ModuleLoadInit::Ran,
        CudaDriverInit::DriverLibraryAbsent => ModuleLoadInit::DriverAbsent,
        CudaDriverInit::Failed(code) => ModuleLoadInit::Failed(code),
    }
}

pub(crate) const fn module_load_init_or_not_at_load(
    recorded: Option<ModuleLoadInit>,
) -> ModuleLoadInit {
    match recorded {
        Some(init) => init,
        None => ModuleLoadInit::NotAtLoad,
    }
}

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

#[cfg(any(
    test,
    all(
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    )
))]
/// The most recent outcome overall and per caller.
#[derive(Debug)]
pub(crate) struct InitRecord {
    last: Option<CudaDriverInit>,
    module_load: Option<CudaDriverInit>,
    embedder_probe: Option<CudaDriverInit>,
    reranker_probe: Option<CudaDriverInit>,
}

#[cfg(any(
    test,
    all(
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    )
))]
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

#[cfg(all(
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
static MODULE_LOAD_INIT: Mutex<Option<ModuleLoadInit>> = Mutex::new(None);

/// Runs the module-load early `cuInit` unless `FATHOMDB_CUDA_EARLY_INIT=off`
/// or every component compiled with CUDA has an exact `cpu` policy, and
/// records the outcome for the allocator decision.
///
/// `embed_cuda_compiled` and `rerank_cuda_compiled` are the caller's own
/// `cfg!` flags, so that feature unification inside this crate cannot change
/// what the caller's artifact can use. Call it from the binding's load hook,
/// once per process, before any export is reachable; a later call replaces
/// the record. Never fails; a missing driver is recorded, not raised.
#[doc(hidden)]
#[cfg(all(
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
pub fn run_module_load_early_init(
    embed_cuda_compiled: bool,
    rerank_cuda_compiled: bool,
) -> ModuleLoadInit {
    let opted_out = early_cuda_init_opted_out(std::env::var(ENV_CUDA_EARLY_INIT).ok().as_deref());
    let embed_policy = std::env::var("FATHOMDB_EMBED_DEVICE").ok();
    let rerank_policy = std::env::var(crate::ENV_RERANK_DEVICE).ok();
    let wanted = early_cuda_init_wanted(
        embed_cuda_compiled,
        rerank_cuda_compiled,
        embed_policy.as_deref(),
        rerank_policy.as_deref(),
    );
    let outcome = module_load_outcome(opted_out, wanted, initialize_cuda_driver);
    *MODULE_LOAD_INIT.lock().unwrap_or_else(PoisonError::into_inner) = Some(outcome);
    outcome
}

/// The recorded module-load outcome; [`ModuleLoadInit::NotAtLoad`] when the
/// load hook never ran (always, off aarch64 Linux CUDA builds).
pub(crate) fn recorded_module_load_init() -> ModuleLoadInit {
    #[cfg(all(
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    ))]
    let recorded = *MODULE_LOAD_INIT.lock().unwrap_or_else(PoisonError::into_inner);
    #[cfg(not(all(
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    )))]
    let recorded = None;
    module_load_init_or_not_at_load(recorded)
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

    #[test]
    fn the_module_load_outcome_maps_every_init_result() {
        use crate::cuda_pool_policy::ModuleLoadInit;
        let never = || -> CudaDriverInit { panic!("cuInit must not run") };
        assert_eq!(module_load_outcome(true, true, never), ModuleLoadInit::OptedOut);
        assert_eq!(module_load_outcome(true, false, never), ModuleLoadInit::OptedOut);
        assert_eq!(module_load_outcome(false, false, never), ModuleLoadInit::SkippedCpuOnly);
        assert_eq!(
            module_load_outcome(false, true, || CudaDriverInit::Initialized),
            ModuleLoadInit::Ran
        );
        assert_eq!(
            module_load_outcome(false, true, || CudaDriverInit::DriverLibraryAbsent),
            ModuleLoadInit::DriverAbsent
        );
        assert_eq!(
            module_load_outcome(false, true, || CudaDriverInit::Failed(OUT_OF_MEMORY)),
            ModuleLoadInit::Failed(OUT_OF_MEMORY)
        );
    }

    #[test]
    fn an_unrecorded_module_load_is_not_at_load() {
        use crate::cuda_pool_policy::ModuleLoadInit;
        assert_eq!(module_load_init_or_not_at_load(None), ModuleLoadInit::NotAtLoad);
        assert_eq!(
            module_load_init_or_not_at_load(Some(ModuleLoadInit::OptedOut)),
            ModuleLoadInit::OptedOut
        );
    }

    #[test]
    fn early_init_is_skipped_only_when_every_cuda_component_is_exactly_cpu() {
        assert!(early_cuda_init_wanted(true, true, None, None));
        assert!(early_cuda_init_wanted(true, true, Some("cpu"), None));
        assert!(!early_cuda_init_wanted(true, true, Some("cpu"), Some("cpu")));
        assert!(!early_cuda_init_wanted(true, false, Some("cpu"), Some("cuda:0")));
        assert!(early_cuda_init_wanted(false, true, Some("cpu"), Some("auto")));
        assert!(!early_cuda_init_wanted(false, false, None, None));
        assert!(early_cuda_init_wanted(true, true, Some("CPU"), Some("cpu")));
    }

    #[test]
    fn only_the_exact_value_off_opts_out() {
        assert_eq!(ENV_CUDA_EARLY_INIT, "FATHOMDB_CUDA_EARLY_INIT");
        assert!(early_cuda_init_opted_out(Some("off")));
        for raw in [None, Some("on"), Some(""), Some("OFF"), Some(" off"), Some("0")] {
            assert!(!early_cuda_init_opted_out(raw), "{raw:?}");
        }
    }
}
