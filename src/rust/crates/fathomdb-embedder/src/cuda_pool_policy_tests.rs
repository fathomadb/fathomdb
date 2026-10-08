use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex, Once, OnceLock};

use super::*;

const GIB: u64 = 1 << 30;
const MIB: u64 = 1 << 20;
/// `cuDeviceTotalMem` on the AGX Orin 64 GB (61.36 GiB).
const ORIN_64_TOTAL: u64 = 65_879_896_064;

fn settings(
    mode: Option<&str>,
    max_size: Option<&str>,
    threshold: Option<&str>,
) -> RawPoolSettings {
    RawPoolSettings {
        mode: mode.map(str::to_owned),
        max_size: max_size.map(str::to_owned),
        release_threshold: threshold.map(str::to_owned),
    }
}

fn defaults() -> RawPoolSettings {
    settings(None, None, None)
}

fn orin_64() -> DeviceFacts {
    DeviceFacts {
        integrated: true,
        pools_supported: true,
        total_mem: ORIN_64_TOTAL,
        compute_capability: (8, 7),
        tegra: true,
    }
}

fn plan(settings: &RawPoolSettings, facts: DeviceFacts) -> Plan {
    decide(settings, ModuleLoadInit::Ran, || Some(facts))
}

fn fallback(reason: CudaAllocatorReason) -> Plan {
    Plan::Fallback { reason }
}

fn private(max_size_bytes: u64) -> Plan {
    Plan::Private { max_size_bytes, release_threshold: ReleaseThreshold::Zero }
}

// ---- constants (design § 2.3) ---------------------------------------------

#[test]
fn pool_size_device_divisor_is_twenty() {
    assert_eq!(POOL_SIZE_DEVICE_DIVISOR, 20);
}

#[test]
fn pool_size_floor_is_two_gib() {
    assert_eq!(POOL_SIZE_FLOOR, 2 * GIB);
}

#[test]
fn pool_size_ceiling_is_three_gib() {
    assert_eq!(POOL_SIZE_CEILING, 3 * GIB);
}

#[test]
fn pool_floor_max_share_divisor_is_four() {
    assert_eq!(POOL_FLOOR_MAX_SHARE_DIVISOR, 4);
}

#[test]
fn maxsize_min_is_thirty_two_mib() {
    assert_eq!(MAXSIZE_MIN, 32 * MIB);
}

#[test]
fn measured_class_min_total_is_forty_eight_gib() {
    assert_eq!(MEASURED_CLASS_MIN_TOTAL, 48 * GIB);
}

#[test]
fn measured_class_compute_capability_is_sm_87() {
    assert_eq!(MEASURED_CLASS_COMPUTE_CAPABILITY, (8, 7));
}

#[test]
fn probe_bytes_is_four() {
    assert_eq!(PROBE_BYTES, 4);
}

#[test]
fn setting_names_are_the_documented_variables() {
    assert_eq!(ENV_POOL_MODE, "FATHOMDB_POOL_MODE");
    assert_eq!(ENV_POOL_MAXSIZE, "FATHOMDB_POOL_MAXSIZE");
    assert_eq!(ENV_POOL_RELEASE_THRESHOLD, "FATHOMDB_POOL_RELEASE_THRESHOLD");
}

// ---- setting parsers (design § 2.2) ----------------------------------------

#[test]
fn mode_is_auto_on_or_off_and_unset_is_auto() {
    assert_eq!(parse_pool_mode(None), Ok(PoolMode::Auto));
    assert_eq!(parse_pool_mode(Some("auto")), Ok(PoolMode::Auto));
    assert_eq!(parse_pool_mode(Some("on")), Ok(PoolMode::On));
    assert_eq!(parse_pool_mode(Some("off")), Ok(PoolMode::Off));
    for bad in ["", "AUTO", "On", " off", "1", "true", "private"] {
        let error = parse_pool_mode(Some(bad)).expect_err(bad);
        assert_eq!(error.variable, ENV_POOL_MODE, "{bad:?}");
    }
}

#[test]
fn max_size_accepts_bytes_gib_and_mib_suffixes() {
    assert_eq!(parse_max_size(None), Ok(None));
    assert_eq!(parse_max_size(Some("3G")), Ok(Some(3 * GIB)));
    assert_eq!(parse_max_size(Some("512M")), Ok(Some(512 * MIB)));
    assert_eq!(parse_max_size(Some("4096")), Ok(Some(4096)));
    for bad in ["", "G", "M", "3g", "3GiB", "-1", "1.5G", " 3G", "3G ", "99999999999999999999G"] {
        let error = parse_max_size(Some(bad)).expect_err(bad);
        assert_eq!(error.variable, ENV_POOL_MAXSIZE, "{bad:?}");
    }
}

#[test]
fn release_threshold_is_zero_or_max_and_unset_is_zero() {
    assert_eq!(parse_release_threshold(None), Ok(ReleaseThreshold::Zero));
    assert_eq!(parse_release_threshold(Some("0")), Ok(ReleaseThreshold::Zero));
    assert_eq!(parse_release_threshold(Some("max")), Ok(ReleaseThreshold::Max));
    for bad in ["", "MAX", "1", "4096", "none"] {
        let error = parse_release_threshold(Some(bad)).expect_err(bad);
        assert_eq!(error.variable, ENV_POOL_RELEASE_THRESHOLD, "{bad:?}");
    }
    assert_eq!(ReleaseThreshold::Zero.bytes(), 0);
    assert_eq!(ReleaseThreshold::Max.bytes(), u64::MAX);
}

#[test]
fn settings_are_read_from_the_named_variables() {
    let raw = RawPoolSettings::from_env(|name| match name {
        "FATHOMDB_POOL_MODE" => Some("on".to_owned()),
        "FATHOMDB_POOL_MAXSIZE" => Some("1G".to_owned()),
        "FATHOMDB_POOL_RELEASE_THRESHOLD" => Some("max".to_owned()),
        _ => None,
    });
    assert_eq!(raw, settings(Some("on"), Some("1G"), Some("max")));
}

// ---- sizing (design § 2.1) -------------------------------------------------

#[test]
fn pool_size_is_a_twentieth_of_device_memory_between_floor_and_ceiling() {
    assert_eq!(pool_size(ORIN_64_TOTAL), 3 * GIB);
    assert_eq!(pool_size(122 * GIB), 3 * GIB);
    assert_eq!(pool_size(30 * GIB), 2 * GIB);
    assert_eq!(pool_size(50 * GIB), 50 * GIB / 20);
}

// ---- gates, in order (design § 2.1) ----------------------------------------

#[test]
fn the_64_gb_orin_gets_a_three_gib_private_pool_in_auto() {
    assert_eq!(plan(&defaults(), orin_64()), private(3 * GIB));
    assert_eq!(plan(&settings(Some("auto"), None, None), orin_64()), private(3 * GIB));
    assert_eq!(plan(&settings(Some("on"), None, None), orin_64()), private(3 * GIB));
}

#[test]
fn off_wins_over_every_other_gate_and_reads_no_device_facts() {
    let read = AtomicBool::new(false);
    let result = decide(
        &settings(Some("off"), Some("bogus"), Some("bogus")),
        ModuleLoadInit::NotAtLoad,
        || {
            read.store(true, Ordering::SeqCst);
            Some(orin_64())
        },
    );
    assert_eq!(result, fallback(CudaAllocatorReason::ModeOff));
    assert!(!read.load(Ordering::SeqCst), "mode off must not touch the device");
}

#[test]
fn a_malformed_setting_is_invalid_setting_and_never_panics() {
    for raw in [
        settings(Some("bogus"), None, None),
        settings(None, Some("3GiB"), None),
        settings(None, None, Some("1")),
        settings(Some("on"), Some(""), None),
    ] {
        assert_eq!(plan(&raw, orin_64()), fallback(CudaAllocatorReason::InvalidSetting), "{raw:?}");
    }
}

#[test]
fn an_out_of_bounds_max_size_is_invalid_setting() {
    for raw in ["1M", "31M", "0", "62G", "64G"] {
        assert_eq!(
            plan(&settings(None, Some(raw), None), orin_64()),
            fallback(CudaAllocatorReason::InvalidSetting),
            "{raw}"
        );
    }
}

#[test]
fn an_in_bounds_max_size_replaces_the_derived_size_once_every_gate_passes() {
    assert_eq!(plan(&settings(None, Some("32M"), None), orin_64()), private(32 * MIB));
    assert_eq!(plan(&settings(None, Some("1G"), None), orin_64()), private(GIB));
    assert_eq!(
        plan(&settings(None, Some(&ORIN_64_TOTAL.to_string()), None), orin_64()),
        private(ORIN_64_TOTAL)
    );
    // An explicit size never lifts a gate.
    let discrete = DeviceFacts { integrated: false, ..orin_64() };
    assert_eq!(
        plan(&settings(Some("on"), Some("1G"), None), discrete),
        fallback(CudaAllocatorReason::Discrete)
    );
}

#[test]
fn the_release_threshold_setting_reaches_the_plan() {
    assert_eq!(
        plan(&settings(None, None, Some("max")), orin_64()),
        Plan::Private { max_size_bytes: 3 * GIB, release_threshold: ReleaseThreshold::Max }
    );
}

#[test]
fn every_module_load_state_but_ran_is_its_own_reason_and_reads_no_device_facts() {
    let cases = [
        (ModuleLoadInit::OptedOut, CudaAllocatorReason::CuinitOptedOut),
        (ModuleLoadInit::SkippedCpuOnly, CudaAllocatorReason::CuinitSkippedCpuOnly),
        (ModuleLoadInit::DriverAbsent, CudaAllocatorReason::CuinitDriverAbsent),
        (ModuleLoadInit::Failed(2), CudaAllocatorReason::CuinitFailed),
        (ModuleLoadInit::NotAtLoad, CudaAllocatorReason::CuinitNotAtLoad),
    ];
    for (init, reason) in cases {
        for mode in [None, Some("on")] {
            let read = AtomicBool::new(false);
            let result = decide(&settings(mode, None, None), init, || {
                read.store(true, Ordering::SeqCst);
                Some(orin_64())
            });
            assert_eq!(result, fallback(reason), "{init:?} mode={mode:?}");
            assert!(!read.load(Ordering::SeqCst), "{init:?}");
        }
    }
}

#[test]
fn a_device_fact_query_failure_is_pool_create_failed() {
    assert_eq!(
        decide(&defaults(), ModuleLoadInit::Ran, || None),
        fallback(CudaAllocatorReason::PoolCreateFailed)
    );
}

#[test]
fn a_discrete_gpu_is_discrete_even_in_on() {
    let discrete =
        DeviceFacts { integrated: false, tegra: false, total_mem: 80 * GIB, ..orin_64() };
    for mode in [None, Some("auto"), Some("on")] {
        assert_eq!(
            plan(&settings(mode, None, None), discrete),
            fallback(CudaAllocatorReason::Discrete),
            "{mode:?}"
        );
    }
}

#[test]
fn a_device_without_pools_is_no_pools_even_in_on() {
    let poolless = DeviceFacts { pools_supported: false, ..orin_64() };
    for mode in [None, Some("on")] {
        assert_eq!(
            plan(&settings(mode, None, None), poolless),
            fallback(CudaAllocatorReason::NoPools),
            "{mode:?}"
        );
    }
}

#[test]
fn an_orin_nano_8_gb_is_too_small_in_every_mode_that_reaches_the_gate() {
    let nano = DeviceFacts { total_mem: 7 * GIB + 4 * GIB / 10, ..orin_64() };
    for mode in [None, Some("auto"), Some("on")] {
        assert_eq!(
            plan(&settings(mode, None, None), nano),
            fallback(CudaAllocatorReason::TooSmall),
            "{mode:?}"
        );
        assert_eq!(
            plan(&settings(mode, Some("64M"), None), nano),
            fallback(CudaAllocatorReason::TooSmall),
            "an explicit size does not lift too_small: {mode:?}"
        );
    }
    assert_eq!(
        plan(&settings(Some("off"), None, None), nano),
        fallback(CudaAllocatorReason::ModeOff)
    );
}

#[test]
fn a_non_tegra_integrated_gpu_is_not_tegra_in_auto_and_sized_by_rule_in_on() {
    // GB10-class: integrated, sm_121, 128 GB, no Tegra identity.
    let gb10 = DeviceFacts {
        tegra: false,
        compute_capability: (12, 1),
        total_mem: 119 * GIB,
        ..orin_64()
    };
    assert_eq!(plan(&defaults(), gb10), fallback(CudaAllocatorReason::NotTegra));
    assert_eq!(plan(&settings(Some("on"), None, None), gb10), private(3 * GIB));
}

#[test]
fn thor_sm_110_is_unmeasured_class_in_auto_and_sized_by_rule_in_on() {
    let thor = DeviceFacts { compute_capability: (11, 0), total_mem: 122 * GIB, ..orin_64() };
    assert_eq!(plan(&defaults(), thor), fallback(CudaAllocatorReason::UnmeasuredClass));
    assert_eq!(plan(&settings(Some("on"), None, None), thor), private(3 * GIB));
}

#[test]
fn a_32_gb_orin_is_unmeasured_class_in_auto_and_sized_by_rule_in_on() {
    let orin_32 = DeviceFacts { total_mem: 30 * GIB, ..orin_64() };
    assert_eq!(plan(&defaults(), orin_32), fallback(CudaAllocatorReason::UnmeasuredClass));
    assert_eq!(plan(&settings(Some("on"), None, None), orin_32), private(2 * GIB));
}

#[test]
fn the_measured_class_needs_both_sm_87_and_forty_eight_gib() {
    let just_under = DeviceFacts { total_mem: 48 * GIB - 1, ..orin_64() };
    assert_eq!(plan(&defaults(), just_under), fallback(CudaAllocatorReason::UnmeasuredClass));
    let at_floor = DeviceFacts { total_mem: 48 * GIB, ..orin_64() };
    assert_eq!(plan(&defaults(), at_floor), private(48 * GIB / 20));
    let other_minor = DeviceFacts { compute_capability: (8, 6), ..orin_64() };
    assert_eq!(plan(&defaults(), other_minor), fallback(CudaAllocatorReason::UnmeasuredClass));
}

#[test]
fn tegra_identity_needs_either_source() {
    // DQ-1: an Orin without /etc/nv_tegra_release but with the device-tree
    // entry still passes.
    assert!(tegra_identity(
        false,
        Some(b"nvidia,p3737-0000+p3701-0005\0nvidia,p3701-0005\0nvidia,tegra234\0")
    ));
    assert!(tegra_identity(true, None));
    assert!(tegra_identity(true, Some(b"raspberrypi,4-model-b\0brcm,bcm2711\0")));
    assert!(!tegra_identity(false, Some(b"raspberrypi,4-model-b\0brcm,bcm2711\0")));
    assert!(!tegra_identity(false, None));
}

// ---- names -----------------------------------------------------------------

#[test]
fn every_reason_has_a_stable_lowercase_name() {
    use CudaAllocatorReason as R;
    let cases = [
        (R::PrivatePool, "private_pool"),
        (R::ModeOff, "mode_off"),
        (R::InvalidSetting, "invalid_setting"),
        (R::CuinitOptedOut, "cuinit_opted_out"),
        (R::CuinitSkippedCpuOnly, "cuinit_skipped_cpu_only"),
        (R::CuinitDriverAbsent, "cuinit_driver_absent"),
        (R::CuinitFailed, "cuinit_failed"),
        (R::CuinitNotAtLoad, "cuinit_not_at_load"),
        (R::Discrete, "discrete"),
        (R::NoPools, "no_pools"),
        (R::TooSmall, "too_small"),
        (R::NotTegra, "not_tegra"),
        (R::UnmeasuredClass, "unmeasured_class"),
        (R::PoolCreateFailed, "pool_create_failed"),
        (R::ProbeFailed, "probe_failed"),
        (R::PrivateBuildFailed, "private_build_failed"),
        (R::DecisionPanicked, "decision_panicked"),
        (R::OtherOrdinal, "other_ordinal"),
        (R::NotBuilt, "not_built"),
    ];
    for (reason, name) in cases {
        assert_eq!(reason.as_str(), name);
    }
}

#[test]
fn paths_thresholds_and_module_load_states_have_stable_names() {
    assert_eq!(CudaAllocatorPath::Private.as_str(), "private");
    assert_eq!(CudaAllocatorPath::DefaultPool.as_str(), "default_pool");
    assert_eq!(CudaAllocatorPath::Synchronous.as_str(), "synchronous");
    assert_eq!(ReleaseThreshold::Zero.as_str(), "0");
    assert_eq!(ReleaseThreshold::Max.as_str(), "max");
    assert_eq!(ModuleLoadInit::Ran.as_str(), "ran");
    assert_eq!(ModuleLoadInit::OptedOut.as_str(), "opted_out");
    assert_eq!(ModuleLoadInit::SkippedCpuOnly.as_str(), "skipped_cpu_only");
    assert_eq!(ModuleLoadInit::DriverAbsent.as_str(), "driver_absent");
    assert_eq!(ModuleLoadInit::Failed(2).as_str(), "failed");
    assert_eq!(ModuleLoadInit::NotAtLoad.as_str(), "not_at_load");
    assert_eq!(ModuleLoadInit::Failed(2).cu_result(), Some(2));
    assert_eq!(ModuleLoadInit::Ran.cu_result(), None);
}

// ---- pool exhaustion -------------------------------------------------------

#[test]
fn only_an_out_of_memory_error_in_a_private_process_is_pool_exhaustion() {
    assert!(is_pool_exhaustion(true, Some(CUDA_ERROR_OUT_OF_MEMORY)));
    assert!(!is_pool_exhaustion(true, Some(999)));
    assert!(!is_pool_exhaustion(true, None));
    assert!(!is_pool_exhaustion(false, Some(CUDA_ERROR_OUT_OF_MEMORY)));
    assert_eq!(CUDA_ERROR_OUT_OF_MEMORY, 2);
}

// ---- not_built (design § 2.6) ----------------------------------------------

#[test]
fn not_built_is_reported_only_for_aarch64_linux_cuda_builds() {
    let report = not_built_report(true, ModuleLoadInit::Ran).expect("aarch64 Linux CUDA build");
    assert_eq!(report.path, None);
    assert_eq!(report.reason, CudaAllocatorReason::NotBuilt);
    assert_eq!(report.pool_max_size_bytes, None);
    assert_eq!(report.release_threshold, None);
    assert_eq!(report.module_load_init, ModuleLoadInit::Ran);
    assert_eq!(not_built_report(false, ModuleLoadInit::NotAtLoad), None);
}

#[cfg(all(
    not(feature = "tegra-pool"),
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
#[test]
fn this_build_without_tegra_pool_reports_not_built() {
    let report = allocator_report(&candle_core::Device::Cpu).expect("aarch64 Linux CUDA build");
    assert_eq!(report.path, None);
    assert_eq!(report.reason, CudaAllocatorReason::NotBuilt);
}

#[cfg(all(
    any(feature = "default-embedder", feature = "default-reranker"),
    not(all(
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    ))
))]
#[test]
fn this_build_reports_no_allocator() {
    assert_eq!(allocator_report(&candle_core::Device::Cpu), None);
}

#[cfg(all(
    any(feature = "default-embedder", feature = "default-reranker"),
    not(all(
        feature = "tegra-pool",
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    ))
))]
#[test]
fn without_the_driver_part_a_forward_error_is_failed_with_its_context() {
    let error = forward_error(candle_core::Error::Msg("boom".to_owned()), "batch forward");
    assert_eq!(
        error,
        fathomdb_embedder_api::EmbedderError::Failed { message: "batch forward: boom".to_owned() }
    );
    assert!(forward_failure(&candle_core::Error::Msg("boom".to_owned()), "forward").is_none());
}

// ---- typed failures --------------------------------------------------------

#[test]
fn each_pool_failure_maps_to_its_embedder_error() {
    use fathomdb_embedder_api::EmbedderError;
    assert_eq!(
        CudaPoolFailure::Exhausted { ordinal: 0, max_size_bytes: 3 * GIB, message: "m".to_owned() }
            .into_embedder_error(),
        EmbedderError::CudaPoolExhausted {
            ordinal: 0,
            max_size_bytes: 3 * GIB,
            message: "m".to_owned()
        }
    );
    assert_eq!(
        CudaPoolFailure::ContextLost {
            recorded_context_id: 7,
            current_context_id: Some(9),
            driver_error: "CUDA_ERROR_CONTEXT_IS_DESTROYED".to_owned(),
            operation: "forward".to_owned(),
        }
        .into_embedder_error(),
        EmbedderError::CudaContextLost {
            recorded_context_id: 7,
            current_context_id: Some(9),
            driver_error: "CUDA_ERROR_CONTEXT_IS_DESTROYED".to_owned(),
            operation: "forward".to_owned(),
        }
    );
    assert_eq!(
        CudaPoolFailure::PrivateBuildRefused { ordinal: 0, message: "m".to_owned() }
            .into_embedder_error(),
        EmbedderError::CudaPrivateBuildRefused { ordinal: 0, message: "m".to_owned() }
    );
}

#[test]
fn each_pool_failure_maps_to_its_reranker_policy_error_kind() {
    let cases = [
        (
            CudaPoolFailure::Exhausted { ordinal: 0, max_size_bytes: 1, message: "m".to_owned() },
            "cuda_pool_exhausted",
            Some(0),
        ),
        (
            CudaPoolFailure::ContextLost {
                recorded_context_id: 1,
                current_context_id: None,
                driver_error: "e".to_owned(),
                operation: "o".to_owned(),
            },
            "cuda_context_lost",
            None,
        ),
        (
            CudaPoolFailure::PrivateBuildRefused { ordinal: 1, message: "m".to_owned() },
            "cuda_private_build_refused",
            Some(1),
        ),
    ];
    for (failure, kind, ordinal) in cases {
        let error = failure.into_reranker_policy_error();
        assert_eq!(error.kind(), kind);
        assert_eq!(error.ordinal(), ordinal);
    }
}

// ---- the decision, through an injected driver ------------------------------

#[derive(Default)]
struct FakeDriver {
    settings: RawPoolSettings,
    init: Option<ModuleLoadInit>,
    facts: Option<DeviceFacts>,
    panic_in_facts: bool,
    fail_create: bool,
    fail_probe: bool,
    fail_first_context: bool,
    later_context_error: Option<u32>,
    fail_context_id: bool,
    facts_reads: AtomicUsize,
    pools_created: AtomicUsize,
    pools_dropped: Arc<AtomicUsize>,
    contexts_built: AtomicUsize,
    retained: AtomicUsize,
    /// `cuDevicePrimaryCtxGetState`: `Err(code)` fails the query; `None` is
    /// an active context with flags 0.
    primary_state: Option<Result<CudaPrimaryContextState, u32>>,
    /// The retained primary context's id: `Err(code)` fails the query;
    /// `None` is the decision's own first-context id (1000).
    current_id: Option<Result<u64, u32>>,
    state_reads: AtomicUsize,
    id_reads: AtomicUsize,
    snapshot: TestOnce,
    snapshots: Mutex<Vec<String>>,
}

struct TestOnce(Once);

impl Default for TestOnce {
    fn default() -> Self {
        Self(Once::new())
    }
}

struct FakePool {
    dropped: Arc<AtomicUsize>,
}

impl Drop for FakePool {
    fn drop(&mut self) {
        self.dropped.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FakeContext(usize);

#[derive(Debug, PartialEq, Eq)]
enum FakeDevice {
    Private(FakeContext),
    Default(usize),
}

fn failure(code: u32) -> DriverFailure {
    DriverFailure { code: Some(code), message: format!("driver error {code}") }
}

impl PoolDriver for FakeDriver {
    type Pool = FakePool;
    type Context = FakeContext;
    type Device = FakeDevice;
    type DefaultError = String;

    fn raw_settings(&self) -> RawPoolSettings {
        self.settings.clone()
    }

    fn module_load_init(&self) -> ModuleLoadInit {
        self.init.unwrap_or(ModuleLoadInit::Ran)
    }

    fn with_primary_retained<R>(
        &self,
        _ordinal: usize,
        body: impl FnOnce() -> R,
    ) -> Result<R, DriverFailure> {
        self.retained.fetch_add(1, Ordering::SeqCst);
        Ok(body())
    }

    fn device_facts(&self, _ordinal: usize) -> Result<DeviceFacts, DriverFailure> {
        self.facts_reads.fetch_add(1, Ordering::SeqCst);
        assert!(!self.panic_in_facts, "injected panic");
        Ok(self.facts.unwrap_or_else(orin_64))
    }

    fn create_pool(
        &self,
        _ordinal: usize,
        _max_size_bytes: u64,
        _threshold: ReleaseThreshold,
    ) -> Result<Self::Pool, DriverFailure> {
        self.pools_created.fetch_add(1, Ordering::SeqCst);
        // Widens the window in which concurrent first users race.
        std::thread::sleep(std::time::Duration::from_millis(20));
        if self.fail_create {
            return Err(failure(CUDA_ERROR_OUT_OF_MEMORY));
        }
        Ok(FakePool { dropped: Arc::clone(&self.pools_dropped) })
    }

    fn probe_pool(&self, _pool: &Self::Pool) -> Result<(), DriverFailure> {
        if self.fail_probe {
            return Err(failure(CUDA_ERROR_OUT_OF_MEMORY));
        }
        Ok(())
    }

    fn new_private_context(
        &self,
        _ordinal: usize,
        _pool: &Self::Pool,
    ) -> Result<Self::Context, DriverFailure> {
        let n = self.contexts_built.fetch_add(1, Ordering::SeqCst);
        if n == 0 && self.fail_first_context {
            return Err(failure(1));
        }
        if n > 0 {
            if let Some(code) = self.later_context_error {
                return Err(failure(code));
            }
        }
        Ok(FakeContext(n))
    }

    fn context_id(&self, context: &Self::Context) -> Result<u64, DriverFailure> {
        if self.fail_context_id {
            return Err(failure(201));
        }
        Ok(1000 + context.0 as u64)
    }

    fn wrap_context(&self, context: Self::Context) -> Result<Self::Device, DriverFailure> {
        Ok(FakeDevice::Private(context))
    }

    fn default_device(&self, ordinal: usize) -> Result<Self::Device, Self::DefaultError> {
        Ok(FakeDevice::Default(ordinal))
    }

    fn primary_state(&self, _ordinal: usize) -> Result<CudaPrimaryContextState, DriverFailure> {
        self.state_reads.fetch_add(1, Ordering::SeqCst);
        match self.primary_state {
            None => Ok(CudaPrimaryContextState { active: true, flags: 0 }),
            Some(Ok(state)) => Ok(state),
            Some(Err(code)) => Err(failure(code)),
        }
    }

    fn retained_primary_context_id(&self, _ordinal: usize) -> Result<u64, DriverFailure> {
        self.id_reads.fetch_add(1, Ordering::SeqCst);
        match self.current_id {
            None => Ok(1000),
            Some(Ok(id)) => Ok(id),
            Some(Err(code)) => Err(failure(code)),
        }
    }

    fn pool_counters(&self, _pool: &Self::Pool) -> Result<PoolCounters, DriverFailure> {
        Ok(PoolCounters {
            reserved_current: 32 * MIB,
            reserved_high: 64 * MIB,
            used_current: 4 * MIB,
            used_high: 48 * MIB,
        })
    }

    fn live_private_contexts(&self, _pool: &Self::Pool) -> usize {
        2
    }

    fn cuda_libraries(&self) -> Vec<String> {
        vec!["/usr/lib/aarch64-linux-gnu/nvidia/libcuda.so.1.1".to_owned()]
    }

    fn snapshot_once(&self) -> &Once {
        &self.snapshot.0
    }

    fn emit_snapshot(&self, line: &str) {
        self.snapshots.lock().expect("snapshots").push(line.to_owned());
    }
}

type FakeDecision = Decision<FakePool, FakeContext>;

#[test]
fn eight_concurrent_first_users_get_one_decision() {
    let driver = FakeDriver::default();
    let cell: OnceLock<FakeDecision> = OnceLock::new();
    let barrier = Barrier::new(8);
    let devices: Vec<FakeDevice> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    build_device(&cell, &driver, 0).expect("private build")
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().expect("thread")).collect()
    });
    assert_eq!(driver.pools_created.load(Ordering::SeqCst), 1);
    assert_eq!(driver.facts_reads.load(Ordering::SeqCst), 1);
    assert_eq!(driver.retained.load(Ordering::SeqCst), 1);
    // One first context from the decision plus seven later builds.
    assert_eq!(driver.contexts_built.load(Ordering::SeqCst), 8);
    let mut seen: Vec<usize> = devices
        .iter()
        .map(|device| match device {
            FakeDevice::Private(FakeContext(n)) => *n,
            FakeDevice::Default(_) => panic!("a private process built a default device"),
        })
        .collect();
    seen.sort_unstable();
    assert_eq!(seen, (0..8).collect::<Vec<_>>(), "the first context is handed out exactly once");
    assert!(matches!(cell.get(), Some(Decision::Private(_))));
}

#[test]
fn a_panicking_decision_becomes_decision_panicked_and_is_not_retried() {
    let driver = FakeDriver { panic_in_facts: true, ..FakeDriver::default() };
    let cell: OnceLock<FakeDecision> = OnceLock::new();
    assert_eq!(build_device(&cell, &driver, 0), Ok(FakeDevice::Default(0)));
    assert_eq!(build_device(&cell, &driver, 0), Ok(FakeDevice::Default(0)));
    assert_eq!(driver.facts_reads.load(Ordering::SeqCst), 1);
    let decision = cell.get().expect("decided");
    assert_eq!(decision.report(0, None).reason, CudaAllocatorReason::DecisionPanicked);
}

fn decided_reason(driver: &FakeDriver) -> CudaAllocatorReason {
    let cell: OnceLock<FakeDecision> = OnceLock::new();
    assert_eq!(
        build_device(&cell, driver, 0),
        Ok(FakeDevice::Default(0)),
        "fallback is the 0.8.27 path"
    );
    cell.get().expect("decided").report(0, Some(CudaAllocatorPath::DefaultPool)).reason
}

#[test]
fn a_pool_create_failure_falls_back_with_its_reason() {
    let driver = FakeDriver { fail_create: true, ..FakeDriver::default() };
    assert_eq!(decided_reason(&driver), CudaAllocatorReason::PoolCreateFailed);
}

#[test]
fn a_probe_failure_falls_back_and_destroys_the_pool() {
    let driver = FakeDriver { fail_probe: true, ..FakeDriver::default() };
    assert_eq!(decided_reason(&driver), CudaAllocatorReason::ProbeFailed);
    assert_eq!(driver.pools_dropped.load(Ordering::SeqCst), 1);
}

#[test]
fn a_first_private_build_failure_falls_back_and_destroys_the_pool() {
    let driver = FakeDriver { fail_first_context: true, ..FakeDriver::default() };
    assert_eq!(decided_reason(&driver), CudaAllocatorReason::PrivateBuildFailed);
    assert_eq!(driver.pools_dropped.load(Ordering::SeqCst), 1);
}

#[test]
fn a_context_id_failure_is_a_first_private_build_failure() {
    let driver = FakeDriver { fail_context_id: true, ..FakeDriver::default() };
    assert_eq!(decided_reason(&driver), CudaAllocatorReason::PrivateBuildFailed);
}

#[test]
fn a_gate_that_is_off_falls_back_without_creating_a_pool() {
    let driver = FakeDriver { init: Some(ModuleLoadInit::NotAtLoad), ..FakeDriver::default() };
    assert_eq!(decided_reason(&driver), CudaAllocatorReason::CuinitNotAtLoad);
    assert_eq!(driver.retained.load(Ordering::SeqCst), 0);
    assert_eq!(driver.pools_created.load(Ordering::SeqCst), 0);

    let driver =
        FakeDriver { settings: settings(Some("off"), None, None), ..FakeDriver::default() };
    assert_eq!(decided_reason(&driver), CudaAllocatorReason::ModeOff);
    assert_eq!(driver.retained.load(Ordering::SeqCst), 0);

    let discrete = DeviceFacts { integrated: false, ..orin_64() };
    let driver = FakeDriver { facts: Some(discrete), ..FakeDriver::default() };
    assert_eq!(decided_reason(&driver), CudaAllocatorReason::Discrete);
    assert_eq!(driver.pools_created.load(Ordering::SeqCst), 0);
}

#[test]
fn a_private_decision_records_the_first_context_id_and_reports_the_pool() {
    let driver = FakeDriver::default();
    let cell: OnceLock<FakeDecision> = OnceLock::new();
    assert_eq!(build_device(&cell, &driver, 0), Ok(FakeDevice::Private(FakeContext(0))));
    let Some(Decision::Private(private)) = cell.get() else {
        panic!("expected a private decision")
    };
    assert_eq!(private.context_id, 1000);
    assert_eq!(private.ordinal, 0);
    let report = cell.get().expect("decided").report(0, Some(CudaAllocatorPath::Private));
    assert_eq!(report.path, Some(CudaAllocatorPath::Private));
    assert_eq!(report.reason, CudaAllocatorReason::PrivatePool);
    assert_eq!(report.pool_max_size_bytes, Some(3 * GIB));
    assert_eq!(report.release_threshold, Some(ReleaseThreshold::Zero));
    assert_eq!(report.module_load_init, ModuleLoadInit::Ran);
}

#[test]
fn after_a_private_decision_a_later_out_of_memory_build_is_pool_exhausted() {
    let driver =
        FakeDriver { later_context_error: Some(CUDA_ERROR_OUT_OF_MEMORY), ..FakeDriver::default() };
    let cell: OnceLock<FakeDecision> = OnceLock::new();
    assert_eq!(build_device(&cell, &driver, 0), Ok(FakeDevice::Private(FakeContext(0))));
    match build_device(&cell, &driver, 0) {
        Err(DeviceBuildError::Pool(CudaPoolFailure::Exhausted {
            ordinal, max_size_bytes, ..
        })) => {
            assert_eq!((ordinal, max_size_bytes), (0, 3 * GIB));
        }
        other => panic!("expected pool exhaustion, got {other:?}"),
    }
}

#[test]
fn after_a_private_decision_any_other_build_failure_is_refused_never_default() {
    let driver = FakeDriver { later_context_error: Some(999), ..FakeDriver::default() };
    let cell: OnceLock<FakeDecision> = OnceLock::new();
    assert_eq!(build_device(&cell, &driver, 0), Ok(FakeDevice::Private(FakeContext(0))));
    match build_device(&cell, &driver, 0) {
        Err(DeviceBuildError::Pool(CudaPoolFailure::PrivateBuildRefused { ordinal, message })) => {
            assert_eq!(ordinal, 0);
            assert!(message.contains("999"), "{message}");
        }
        other => panic!("expected a refused private build, got {other:?}"),
    }
}

#[test]
fn another_ordinal_takes_the_default_path_and_reports_other_ordinal() {
    let driver = FakeDriver::default();
    let cell: OnceLock<FakeDecision> = OnceLock::new();
    assert_eq!(build_device(&cell, &driver, 0), Ok(FakeDevice::Private(FakeContext(0))));
    assert_eq!(build_device(&cell, &driver, 1), Ok(FakeDevice::Default(1)));
    assert_eq!(driver.contexts_built.load(Ordering::SeqCst), 1);
    let report = cell.get().expect("decided").report(1, Some(CudaAllocatorPath::DefaultPool));
    assert_eq!(report.reason, CudaAllocatorReason::OtherOrdinal);
    assert_eq!(report.path, Some(CudaAllocatorPath::DefaultPool));
    assert_eq!(report.pool_max_size_bytes, None);
}

#[test]
fn a_forward_out_of_memory_is_exhaustion_only_in_a_private_process() {
    let driver = FakeDriver::default();
    let cell: OnceLock<FakeDecision> = OnceLock::new();
    assert_eq!(classify_forward(cell.get(), Some(CUDA_ERROR_OUT_OF_MEMORY), "m".to_owned()), None);
    build_device(&cell, &driver, 0).expect("private");
    assert_eq!(
        classify_forward(cell.get(), Some(CUDA_ERROR_OUT_OF_MEMORY), "m".to_owned()),
        Some(CudaPoolFailure::Exhausted {
            ordinal: 0,
            max_size_bytes: 3 * GIB,
            message: "m".to_owned()
        })
    );
    assert_eq!(classify_forward(cell.get(), Some(999), "m".to_owned()), None);

    let fallback_driver =
        FakeDriver { settings: settings(Some("off"), None, None), ..FakeDriver::default() };
    let fallback_cell: OnceLock<FakeDecision> = OnceLock::new();
    build_device(&fallback_cell, &fallback_driver, 0).expect("default");
    assert_eq!(
        classify_forward(fallback_cell.get(), Some(CUDA_ERROR_OUT_OF_MEMORY), "m".to_owned()),
        None
    );
}

// ---- context loss (design § 3.5) -------------------------------------------

const INACTIVE: CudaPrimaryContextState = CudaPrimaryContextState { active: false, flags: 0 };

fn private_cell(driver: &FakeDriver) -> OnceLock<FakeDecision> {
    let cell: OnceLock<FakeDecision> = OnceLock::new();
    assert_eq!(build_device(&cell, driver, 0), Ok(FakeDevice::Private(FakeContext(0))));
    cell
}

fn cuda_fault(code: Option<u32>) -> ForwardFault {
    ForwardFault { cuda: true, code, error: format!("DriverError({code:?})") }
}

fn lost(failure: Option<CudaPoolFailure>) -> (u64, Option<u64>, String, String) {
    match failure {
        Some(CudaPoolFailure::ContextLost {
            recorded_context_id,
            current_context_id,
            driver_error,
            operation,
        }) => (recorded_context_id, current_context_id, driver_error, operation),
        other => panic!("expected a lost context, got {other:?}"),
    }
}

#[test]
fn context_loss_error_codes_are_the_driver_values() {
    assert_eq!(CUDA_ERROR_INVALID_CONTEXT, 201);
    assert_eq!(CUDA_ERROR_CONTEXT_IS_DESTROYED, 709);
    assert_eq!(CONTEXT_LOST_SNAPSHOT_PREFIX, "fathomdb-cuda-context-lost ");
}

#[test]
fn an_inactive_primary_context_is_lost_without_retaining_it() {
    let driver = FakeDriver { primary_state: Some(Ok(INACTIVE)), ..FakeDriver::default() };
    let cell = private_cell(&driver);
    let (recorded, current, driver_error, operation) =
        lost(classify_forward_fault(cell.get(), &driver, cuda_fault(Some(999)), "forward"));
    assert_eq!(recorded, 1000);
    assert_eq!(current, None);
    assert_eq!(driver_error, "DriverError(Some(999))");
    assert_eq!(operation, "forward");
    assert_eq!(driver.state_reads.load(Ordering::SeqCst), 1);
    assert_eq!(driver.id_reads.load(Ordering::SeqCst), 0, "an inactive context is never retained");
}

#[test]
fn an_active_primary_context_with_another_id_is_lost() {
    let driver = FakeDriver { current_id: Some(Ok(2000)), ..FakeDriver::default() };
    let cell = private_cell(&driver);
    let (recorded, current, _, operation) =
        lost(classify_forward_fault(cell.get(), &driver, cuda_fault(Some(999)), "batch forward"));
    assert_eq!((recorded, current), (1000, Some(2000)));
    assert_eq!(operation, "batch forward");
    assert_eq!(driver.id_reads.load(Ordering::SeqCst), 1);
}

#[test]
fn a_failed_state_query_is_a_lost_context() {
    let driver = FakeDriver { primary_state: Some(Err(3)), ..FakeDriver::default() };
    let cell = private_cell(&driver);
    let (_, current, _, _) =
        lost(classify_forward_fault(cell.get(), &driver, cuda_fault(Some(999)), "forward"));
    assert_eq!(current, None);
    assert_eq!(driver.id_reads.load(Ordering::SeqCst), 0);
}

#[test]
fn a_failed_id_query_is_a_lost_context() {
    let driver = FakeDriver { current_id: Some(Err(201)), ..FakeDriver::default() };
    let cell = private_cell(&driver);
    let (_, current, _, _) =
        lost(classify_forward_fault(cell.get(), &driver, cuda_fault(Some(999)), "forward"));
    assert_eq!(current, None);
}

#[test]
fn a_destroyed_or_invalid_context_error_is_lost_even_when_the_ids_match() {
    for code in [CUDA_ERROR_CONTEXT_IS_DESTROYED, CUDA_ERROR_INVALID_CONTEXT] {
        let driver = FakeDriver::default();
        let cell = private_cell(&driver);
        let (recorded, current, _, _) =
            lost(classify_forward_fault(cell.get(), &driver, cuda_fault(Some(code)), "forward"));
        assert_eq!((recorded, current), (1000, Some(1000)), "code {code}");
    }
}

#[test]
fn a_live_unchanged_context_keeps_the_existing_mapping() {
    let driver = FakeDriver::default();
    let cell = private_cell(&driver);
    assert_eq!(classify_forward_fault(cell.get(), &driver, cuda_fault(Some(999)), "forward"), None);
    assert_eq!(driver.state_reads.load(Ordering::SeqCst), 1);
    assert_eq!(driver.id_reads.load(Ordering::SeqCst), 1);
    assert!(driver.snapshots.lock().expect("snapshots").is_empty());
}

#[test]
fn out_of_memory_is_exhaustion_and_never_queries_the_context() {
    let driver = FakeDriver { primary_state: Some(Ok(INACTIVE)), ..FakeDriver::default() };
    let cell = private_cell(&driver);
    assert!(matches!(
        classify_forward_fault(
            cell.get(),
            &driver,
            cuda_fault(Some(CUDA_ERROR_OUT_OF_MEMORY)),
            "forward"
        ),
        Some(CudaPoolFailure::Exhausted { .. })
    ));
    assert_eq!(driver.state_reads.load(Ordering::SeqCst), 0);
}

#[test]
fn detection_runs_only_for_cuda_errors_in_a_private_process() {
    let driver = FakeDriver { primary_state: Some(Ok(INACTIVE)), ..FakeDriver::default() };
    let cell = private_cell(&driver);
    let not_cuda = ForwardFault { cuda: false, code: None, error: "shape".to_owned() };
    assert_eq!(classify_forward_fault(cell.get(), &driver, not_cuda, "forward"), None);

    let fallback_driver = FakeDriver {
        settings: settings(Some("off"), None, None),
        primary_state: Some(Ok(INACTIVE)),
        ..FakeDriver::default()
    };
    let fallback_cell: OnceLock<FakeDecision> = OnceLock::new();
    build_device(&fallback_cell, &fallback_driver, 0).expect("default");
    assert_eq!(
        classify_forward_fault(fallback_cell.get(), &fallback_driver, cuda_fault(Some(709)), "f"),
        None
    );
    assert_eq!(classify_forward_fault(None, &fallback_driver, cuda_fault(Some(709)), "f"), None);
    assert_eq!(driver.state_reads.load(Ordering::SeqCst), 0);
    assert_eq!(fallback_driver.state_reads.load(Ordering::SeqCst), 0);
}

#[test]
fn a_later_private_build_on_a_lost_context_is_context_lost_not_refused() {
    let driver = FakeDriver {
        later_context_error: Some(CUDA_ERROR_CONTEXT_IS_DESTROYED),
        primary_state: Some(Ok(INACTIVE)),
        ..FakeDriver::default()
    };
    let cell = private_cell(&driver);
    match build_device(&cell, &driver, 0) {
        Err(DeviceBuildError::Pool(failure)) => {
            let (recorded, current, driver_error, operation) = lost(Some(failure));
            assert_eq!((recorded, current), (1000, None));
            assert!(driver_error.contains("709"), "{driver_error}");
            assert_eq!(operation, DEVICE_BUILD_OPERATION);
            assert_eq!(DEVICE_BUILD_OPERATION, "device build");
        }
        other => panic!("expected a lost context, got {other:?}"),
    }
}

#[test]
fn a_later_private_build_on_a_live_context_is_still_refused() {
    let driver = FakeDriver { later_context_error: Some(999), ..FakeDriver::default() };
    let cell = private_cell(&driver);
    assert!(matches!(
        build_device(&cell, &driver, 0),
        Err(DeviceBuildError::Pool(CudaPoolFailure::PrivateBuildRefused { .. }))
    ));
    assert_eq!(driver.state_reads.load(Ordering::SeqCst), 1);
}

#[test]
fn the_first_loss_writes_one_snapshot_line_with_every_field() {
    let driver = FakeDriver {
        primary_state: Some(Ok(CudaPrimaryContextState { active: true, flags: 4 })),
        current_id: Some(Ok(2000)),
        ..FakeDriver::default()
    };
    let cell = private_cell(&driver);
    let fault = ForwardFault { cuda: true, code: Some(999), error: "bad \"ctx\"\n".to_owned() };
    lost(classify_forward_fault(cell.get(), &driver, fault, "forward"));
    lost(classify_forward_fault(cell.get(), &driver, cuda_fault(Some(709)), "forward"));
    let snapshots = driver.snapshots.lock().expect("snapshots");
    assert_eq!(snapshots.len(), 1, "one snapshot per process");
    let line = &snapshots[0];
    let json = line.strip_prefix(CONTEXT_LOST_SNAPSHOT_PREFIX).expect("prefix");
    assert!(!line.contains('\n'), "one line: {line}");
    assert_eq!(
        json,
        concat!(
            r#"{"recorded_context_id":1000,"current_context_id":2000,"#,
            r#""driver_error":"bad \"ctx\"\n","operation":"forward","#,
            r#""primary_context":{"active":true,"flags":4},"#,
            r#""cuda_libraries":["/usr/lib/aarch64-linux-gnu/nvidia/libcuda.so.1.1"],"#,
            r#""pool":{"reserved_mem_current":33554432,"reserved_mem_high":67108864,"#,
            r#""used_mem_current":4194304,"used_mem_high":50331648},"#,
            r#""live_private_contexts":2}"#
        )
    );
}

#[test]
fn a_snapshot_after_a_failed_state_query_records_nulls() {
    let driver = FakeDriver { primary_state: Some(Err(3)), ..FakeDriver::default() };
    let cell = private_cell(&driver);
    lost(classify_forward_fault(cell.get(), &driver, cuda_fault(Some(999)), "forward"));
    let snapshots = driver.snapshots.lock().expect("snapshots");
    assert_eq!(snapshots.len(), 1);
    assert!(snapshots[0].contains(r#""current_context_id":null"#), "{}", snapshots[0]);
    assert!(snapshots[0].contains(r#""primary_context":null"#), "{}", snapshots[0]);
}

#[test]
fn two_concurrent_detections_write_one_snapshot_and_both_are_typed() {
    let driver = FakeDriver { primary_state: Some(Ok(INACTIVE)), ..FakeDriver::default() };
    let cell = private_cell(&driver);
    let barrier = Barrier::new(2);
    let results: Vec<Option<CudaPoolFailure>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..2)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    classify_forward_fault(cell.get(), &driver, cuda_fault(Some(709)), "forward")
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().expect("thread")).collect()
    });
    for result in results {
        lost(result);
    }
    assert_eq!(driver.snapshots.lock().expect("snapshots").len(), 1);
}

#[test]
fn cuda_libraries_are_read_from_proc_maps_once_each_in_order() {
    let maps = "\
aaaa-bbbb r-xp 00000000 103:02 1 /usr/lib/aarch64-linux-gnu/nvidia/libcuda.so.1.1
bbbb-cccc r--p 00010000 103:02 1 /usr/lib/aarch64-linux-gnu/nvidia/libcuda.so.1.1
cccc-dddd r-xp 00000000 103:02 2 /usr/lib/aarch64-linux-gnu/libc.so.6
dddd-eeee r-xp 00000000 103:02 3 /usr/local/cuda-12.6/lib64/libcublasLt.so.12.6.1.4
eeee-ffff r-xp 00000000 103:02 4 /usr/local/cuda/lib64/libnvrtc.so.12
ffff-1111 rw-p 00000000 00:00 0 [heap]
1111-2222 r-xp 00000000 103:02 5 /opt/my libs/libcudart.so.12
2222-3333 r-xp 00000000 103:02 6 /usr/lib/aarch64-linux-gnu/nvidia/libnvrm_gpu.so
";
    assert_eq!(
        cuda_libraries_in_maps(maps),
        vec![
            "/usr/lib/aarch64-linux-gnu/nvidia/libcuda.so.1.1",
            "/usr/local/cuda-12.6/lib64/libcublasLt.so.12.6.1.4",
            "/usr/local/cuda/lib64/libnvrtc.so.12",
            "/opt/my libs/libcudart.so.12",
            "/usr/lib/aarch64-linux-gnu/nvidia/libnvrm_gpu.so",
        ]
    );
}

#[cfg(not(all(
    feature = "tegra-pool",
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
)))]
#[test]
fn without_the_driver_part_there_is_no_context_state() {
    assert_eq!(cuda_context_state(0), None);
}

// ---- GPU smoke (Jetson AGX Orin 64 GB) -------------------------------------

#[cfg(all(
    feature = "tegra-pool",
    feature = "embed-cuda",
    target_os = "linux",
    target_arch = "aarch64"
))]
#[test]
#[ignore = "needs the Jetson AGX Orin 64 GB; run under flock /tmp/fathomdb-gpu.lock"]
fn gpu_orin_64_builds_every_device_on_a_three_gib_private_pool() {
    use candle_core::cuda::cudarc::driver::AllocMode;

    let init = crate::cuda_driver_init::run_module_load_early_init(true, true);
    assert_eq!(init, ModuleLoadInit::Ran);
    for _ in 0..2 {
        let device = new_cuda_device(0).expect("private device");
        let cuda = device.as_cuda_device().expect("cuda device");
        assert_eq!(cuda.cuda_stream().context().alloc_mode(), AllocMode::Private);
        candle_core::Tensor::zeros(1024, candle_core::DType::F32, &device).expect("allocation");
        let report = allocator_report(&device).expect("report");
        assert_eq!(report.path, Some(CudaAllocatorPath::Private));
        assert_eq!(report.reason, CudaAllocatorReason::PrivatePool);
        assert_eq!(report.pool_max_size_bytes, Some(3 * GIB));
        assert_eq!(report.release_threshold, Some(ReleaseThreshold::Zero));
        assert_eq!(report.module_load_init, ModuleLoadInit::Ran);
    }
}

#[cfg(all(
    feature = "tegra-pool",
    feature = "embed-cuda",
    target_os = "linux",
    target_arch = "aarch64"
))]
const RESET_CHILD_ENV: &str = "FATHOMDB_TEST_CONTEXT_RESET_CHILD";

/// Characterizes what a co-resident `cuDevicePrimaryCtxReset` does to a
/// private-pool process (results § 13.4), in a child process so the reset
/// cannot reach other tests. The child builds a private device, computes on
/// it, resets the primary context the way the study's `pool_reset.c` does,
/// and computes again: that error must map to `cuda_context_lost`, with one
/// snapshot line on stderr. The child then drops its tensors and device.
/// That teardown dies with SIGSEGV, as the study recorded: cudarc's
/// `CudaSlice` drop waits on events of the destroyed context. 0.8.29's fix
/// flips the teardown assertion.
#[cfg(all(
    feature = "tegra-pool",
    feature = "embed-cuda",
    target_os = "linux",
    target_arch = "aarch64"
))]
#[test]
#[ignore = "needs the Jetson AGX Orin 64 GB; run under flock /tmp/fathomdb-gpu.lock"]
fn gpu_orin_64_a_primary_context_reset_is_context_lost_and_teardown_segfaults() {
    use std::os::unix::process::ExitStatusExt;

    let exe = std::env::current_exe().expect("test binary");
    let output = std::process::Command::new(exe)
        .args([
            "cuda_pool_policy::tests::gpu_orin_64_context_reset_child",
            "--exact",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(RESET_CHILD_ENV, "1")
        .output()
        .expect("child process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let report = format!("status {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}", output.status);
    assert!(stdout.contains("context-reset-child: context_lost"), "{report}");
    let snapshots: Vec<&str> =
        stderr.lines().filter(|line| line.starts_with(CONTEXT_LOST_SNAPSHOT_PREFIX)).collect();
    assert_eq!(snapshots.len(), 1, "one snapshot per process\n{report}");
    let snapshot = snapshots[0];
    assert!(snapshot.contains(r#""current_context_id":null"#), "{report}");
    assert!(snapshot.contains(r#""primary_context":{"active":false"#), "{report}");
    assert!(snapshot.contains(r#""live_private_contexts":1"#), "{report}");
    assert!(snapshot.contains("libcuda.so"), "{report}");
    assert!(snapshot.contains(r#""pool":{"reserved_mem_current":"#), "{report}");
    assert!(stdout.contains("context-reset-child: teardown"), "{report}");
    assert!(!stdout.contains("context-reset-child: torn down"), "{report}");
    assert_eq!(output.status.signal(), Some(11), "teardown dies with SIGSEGV\n{report}");
}

#[cfg(all(
    feature = "tegra-pool",
    feature = "embed-cuda",
    target_os = "linux",
    target_arch = "aarch64"
))]
#[test]
#[ignore = "run by gpu_orin_64_a_primary_context_reset_is_context_lost_and_teardown_segfaults"]
fn gpu_orin_64_context_reset_child() {
    use candle_core::cuda::cudarc::driver::{result, sys};
    use candle_core::{DType, Tensor};
    use fathomdb_embedder_api::EmbedderError;

    if std::env::var_os(RESET_CHILD_ENV).is_none() {
        return;
    }
    let init = crate::cuda_driver_init::run_module_load_early_init(true, true);
    assert_eq!(init, ModuleLoadInit::Ran);
    let device = new_cuda_device(0).expect("private device");
    let ones = Tensor::ones(1024, DType::F32, &device).expect("allocation");
    let doubled = (&ones * 2.0).expect("compute");
    let sum: f32 = doubled.sum_all().and_then(|t| t.to_scalar()).expect("read back");
    assert!((sum - 2048.0).abs() < f32::EPSILON);
    assert_eq!(cuda_context_state(0), Some(CudaPrimaryContextState { active: true, flags: 0 }));

    let cu_device = result::device::get(0).expect("device");
    // SAFETY: cu_device comes from cuDeviceGet. This is the co-resident
    // library's reset the test characterizes.
    unsafe { sys::cuDevicePrimaryCtxReset_v2(cu_device) }.result().expect("reset");
    assert_eq!(cuda_context_state(0).map(|state| state.active), Some(false));

    for operation in ["forward", "batch forward"] {
        let error = (&ones * 2.0)
            .and_then(|t| t.to_vec1::<f32>())
            .expect_err("computing on the reset context fails");
        match forward_error(error, operation) {
            EmbedderError::CudaContextLost {
                recorded_context_id,
                current_context_id,
                driver_error,
                operation: reported,
            } => {
                assert_eq!(current_context_id, None);
                assert_eq!(reported, operation);
                println!(
                    "context-reset-child: context_lost recorded={recorded_context_id} \
                     driver_error={driver_error}"
                );
            }
            other => panic!("expected cuda_context_lost, got {other:?}"),
        }
    }
    assert_eq!(
        cuda_context_state(0).map(|state| state.active),
        Some(false),
        "detection never creates a primary context"
    );

    println!("context-reset-child: teardown");
    drop(doubled);
    drop(ones);
    drop(device);
    println!("context-reset-child: torn down");
}
