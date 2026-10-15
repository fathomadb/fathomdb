//! The CUDA allocator decision for the Tegra private memory pool.
//!
//! A process built with `tegra-pool` on aarch64 Linux makes one decision at
//! its first CUDA device build. If every gate passes, it creates one private
//! memory pool, probes it with a 4-byte allocation, and builds its first
//! context on it; every later device for that ordinal is built on the same
//! pool. Any gate that is off, and any failure or panic while deciding, keeps
//! the process on cudarc's own default-pool-or-synchronous path with a
//! recorded reason. Once the decision is private, a failed device build is a
//! typed error, never a context on another allocator, because a buffer must
//! be freed by the allocator that made it.
//!
//! The settings, gates, sizing, report types and the decision itself are
//! pure and compile on every host, so their tests run without a GPU through
//! an injected [`PoolDriver`]. The cudarc driver and the process-wide
//! decision compile only with `tegra-pool` on aarch64 Linux with
//! `embed-cuda` or `rerank-cuda`. Without them, devices are built with
//! `Device::new_cuda` exactly as before. `embed-cuda` and `rerank-cuda` imply
//! `tegra-pool` (slice-31 design.md § 14.3), so the gate reduces to aarch64
//! Linux with CUDA.

// Without the driver part only the report types, `not_built` and the
// candle-facing wrappers are live; the decision stays compiled so its tests
// run on every host.
#![cfg_attr(
    not(all(
        feature = "tegra-pool",
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    )),
    allow(dead_code)
)]

use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Once, OnceLock};

const MIB: u64 = 1 << 20;
const GIB: u64 = 1 << 30;

/// The derived pool size is device memory divided by this.
pub(crate) const POOL_SIZE_DEVICE_DIVISOR: u64 = 20;
/// Smallest derived pool size.
pub(crate) const POOL_SIZE_FLOOR: u64 = 2 * GIB;
/// Largest derived pool size; 1/20 of the 64 GB AGX Orin is just above it.
pub(crate) const POOL_SIZE_CEILING: u64 = 3 * GIB;
/// No pool when the floor exceeds this share (1/4) of device memory: on a
/// small device the floor would pin too much shared RAM.
pub(crate) const POOL_FLOOR_MAX_SHARE_DIVISOR: u64 = 4;
/// Smallest accepted `FATHOMDB_POOL_MAXSIZE`: the pool grows in 32 MiB
/// chunks on the measured AGX Orin.
pub(crate) const MAXSIZE_MIN: u64 = 32 * MIB;
/// The measured class is the AGX Orin 64 GB, which reports 61.36 GiB; the
/// 32 GB Orin reports about 30 GiB.
pub(crate) const MEASURED_CLASS_MIN_TOTAL: u64 = 48 * GIB;
/// Compute capability of the measured class (sm_87).
pub(crate) const MEASURED_CLASS_COMPUTE_CAPABILITY: (u32, u32) = (8, 7);
/// Bytes of the allocation that probes a new pool before any context uses it.
pub(crate) const PROBE_BYTES: usize = 4;
/// `CUDA_ERROR_OUT_OF_MEMORY`, which a pool at its cap returns.
pub(crate) const CUDA_ERROR_OUT_OF_MEMORY: u32 = 2;
/// `CUDA_ERROR_INVALID_CONTEXT`.
pub(crate) const CUDA_ERROR_INVALID_CONTEXT: u32 = 201;
/// `CUDA_ERROR_CONTEXT_IS_DESTROYED`.
pub(crate) const CUDA_ERROR_CONTEXT_IS_DESTROYED: u32 = 709;
/// Prefix of the one stderr line a process writes on its first context loss.
pub(crate) const CONTEXT_LOST_SNAPSHOT_PREFIX: &str = "fathomdb-cuda-context-lost ";

pub(crate) const ENV_POOL_MODE: &str = "FATHOMDB_POOL_MODE";
pub(crate) const ENV_POOL_MAXSIZE: &str = "FATHOMDB_POOL_MAXSIZE";
pub(crate) const ENV_POOL_RELEASE_THRESHOLD: &str = "FATHOMDB_POOL_RELEASE_THRESHOLD";

// ---- report types -----------------------------------------------------------

/// The allocator a CUDA context actually uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CudaAllocatorPath {
    /// Stream-ordered allocation from FathomDB's private memory pool.
    Private,
    /// Stream-ordered allocation from the device's current (default) pool.
    DefaultPool,
    /// Synchronous `cuMemAlloc` / `cuMemFree`.
    Synchronous,
}

impl CudaAllocatorPath {
    /// Stable lowercase name for bindings and JSON.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::DefaultPool => "default_pool",
            Self::Synchronous => "synchronous",
        }
    }
}

/// Why the process allocates the way it does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CudaAllocatorReason {
    /// Every gate passed; the device allocates from the private pool.
    PrivatePool,
    /// `FATHOMDB_POOL_MODE=off`.
    ModeOff,
    /// A pool setting was malformed or out of bounds.
    InvalidSetting,
    /// `FATHOMDB_CUDA_EARLY_INIT=off` kept the module-load `cuInit` from running.
    CuinitOptedOut,
    /// Every CUDA-capable component's policy was exactly `cpu` at module load.
    CuinitSkippedCpuOnly,
    /// No CUDA driver library was found at module load.
    CuinitDriverAbsent,
    /// The module-load `cuInit` failed.
    CuinitFailed,
    /// No module-load `cuInit` was recorded and no FathomDB device probe had
    /// initialised the driver (a Rust process whose probe ran reaches the
    /// pool with `module_load_init` still `NotAtLoad`).
    CuinitNotAtLoad,
    /// The GPU is discrete, not integrated.
    Discrete,
    /// The device does not support memory pools.
    NoPools,
    /// The smallest pool would exceed a quarter of device memory.
    TooSmall,
    /// Neither Tegra identity source is present (lifted by `on`).
    NotTegra,
    /// Not the measured class: sm_87 with at least 48 GiB (lifted by `on`).
    UnmeasuredClass,
    /// Reading the device or creating the pool failed.
    PoolCreateFailed,
    /// The probe allocation on the new pool failed.
    ProbeFailed,
    /// Building the first context on the new pool failed.
    PrivateBuildFailed,
    /// The decision panicked.
    DecisionPanicked,
    /// The process decided for another device ordinal; this one takes the
    /// default path.
    OtherOrdinal,
    /// The artifact was built without `tegra-pool`.
    NotBuilt,
}

impl CudaAllocatorReason {
    /// Stable lowercase name for bindings and JSON.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PrivatePool => "private_pool",
            Self::ModeOff => "mode_off",
            Self::InvalidSetting => "invalid_setting",
            Self::CuinitOptedOut => "cuinit_opted_out",
            Self::CuinitSkippedCpuOnly => "cuinit_skipped_cpu_only",
            Self::CuinitDriverAbsent => "cuinit_driver_absent",
            Self::CuinitFailed => "cuinit_failed",
            Self::CuinitNotAtLoad => "cuinit_not_at_load",
            Self::Discrete => "discrete",
            Self::NoPools => "no_pools",
            Self::TooSmall => "too_small",
            Self::NotTegra => "not_tegra",
            Self::UnmeasuredClass => "unmeasured_class",
            Self::PoolCreateFailed => "pool_create_failed",
            Self::ProbeFailed => "probe_failed",
            Self::PrivateBuildFailed => "private_build_failed",
            Self::DecisionPanicked => "decision_panicked",
            Self::OtherOrdinal => "other_ordinal",
            Self::NotBuilt => "not_built",
        }
    }
}

/// The private pool's release threshold (`FATHOMDB_POOL_RELEASE_THRESHOLD`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReleaseThreshold {
    /// Freed memory returns to the system automatically; FathomDB never
    /// holds freed memory.
    Zero,
    /// Freed memory stays reserved for reuse while any FathomDB model is open
    /// in the process, and is returned when the last one closes.
    Max,
}

impl ReleaseThreshold {
    /// Stable name: `"0"` or `"max"`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Zero => "0",
            Self::Max => "max",
        }
    }

    /// The `CU_MEMPOOL_ATTR_RELEASE_THRESHOLD` value in bytes.
    #[must_use]
    pub const fn bytes(self) -> u64 {
        match self {
            Self::Zero => 0,
            Self::Max => u64::MAX,
        }
    }
}

/// What the module-load early `cuInit` did in this process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModuleLoadInit {
    /// `cuInit` ran and succeeded.
    Ran,
    /// `FATHOMDB_CUDA_EARLY_INIT=off`.
    OptedOut,
    /// Every CUDA-capable component's policy was exactly `cpu`.
    SkippedCpuOnly,
    /// No CUDA driver library was found.
    DriverAbsent,
    /// `cuInit` failed with this raw `CUresult`.
    Failed(u32),
    /// Nothing ran at module load. A Rust process reports this; it reaches
    /// the pool through its device probe's `cuInit`.
    NotAtLoad,
}

impl ModuleLoadInit {
    /// Stable lowercase name; [`ModuleLoadInit::Failed`] carries its code in
    /// [`ModuleLoadInit::cu_result`].
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ran => "ran",
            Self::OptedOut => "opted_out",
            Self::SkippedCpuOnly => "skipped_cpu_only",
            Self::DriverAbsent => "driver_absent",
            Self::Failed(_) => "failed",
            Self::NotAtLoad => "not_at_load",
        }
    }

    /// The raw `CUresult` of a failed `cuInit`; `None` otherwise.
    #[must_use]
    pub const fn cu_result(self) -> Option<u32> {
        match self {
            Self::Failed(code) => Some(code),
            _ => None,
        }
    }
}

/// The CUDA allocator decision behind one device, reported on
/// [`crate::CudaDeviceInfo::cuda_allocator`].
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CudaAllocatorReport {
    /// The allocator of the context actually built; `None` when unknown
    /// (an artifact built without `tegra-pool` cannot read it).
    pub path: Option<CudaAllocatorPath>,
    /// Why the process allocates this way.
    pub reason: CudaAllocatorReason,
    /// The private pool's cap in bytes; `None` unless the path is private.
    pub pool_max_size_bytes: Option<u64>,
    /// The private pool's release threshold; `None` unless the path is private.
    pub release_threshold: Option<ReleaseThreshold>,
    /// What the module-load early `cuInit` did.
    pub module_load_init: ModuleLoadInit,
    /// The pool mode in force; `None` when `FATHOMDB_POOL_MODE` was
    /// malformed (the reason is then `invalid_setting`).
    pub pool_mode: Option<PoolMode>,
    /// Where the mode in force came from.
    pub pool_mode_source: Option<PoolSettingSource>,
    /// Where the pool size in force came from (`default`: derived from
    /// device memory).
    pub pool_max_size_source: Option<PoolSettingSource>,
    /// Where the release threshold in force came from.
    pub pool_release_threshold_source: Option<PoolSettingSource>,
}

impl CudaAllocatorReport {
    /// A report with every field given.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        path: Option<CudaAllocatorPath>,
        reason: CudaAllocatorReason,
        pool_max_size_bytes: Option<u64>,
        release_threshold: Option<ReleaseThreshold>,
        module_load_init: ModuleLoadInit,
        pool_mode: Option<PoolMode>,
        pool_mode_source: Option<PoolSettingSource>,
        pool_max_size_source: Option<PoolSettingSource>,
        pool_release_threshold_source: Option<PoolSettingSource>,
    ) -> Self {
        Self {
            path,
            reason,
            pool_max_size_bytes,
            release_threshold,
            module_load_init,
            pool_mode,
            pool_mode_source,
            pool_max_size_source,
            pool_release_threshold_source,
        }
    }

    /// A report whose pool fields come from the configuration in force.
    pub(crate) fn with_config_in_force(
        path: Option<CudaAllocatorPath>,
        reason: CudaAllocatorReason,
        pool_max_size_bytes: Option<u64>,
        release_threshold: Option<ReleaseThreshold>,
        module_load_init: ModuleLoadInit,
    ) -> Self {
        let fields = pool_config_in_force().report_fields();
        Self::new(
            path,
            reason,
            pool_max_size_bytes,
            release_threshold,
            module_load_init,
            fields.mode,
            fields.mode_source,
            fields.max_size_source,
            fields.release_threshold_source,
        )
    }
}

/// The report of an artifact built without `tegra-pool`: `not_built` on
/// aarch64 Linux CUDA builds, where the pool could exist, and none elsewhere.
#[cfg(any(
    test,
    not(all(
        feature = "tegra-pool",
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    ))
))]
pub(crate) fn not_built_report(
    aarch64_linux_cuda: bool,
    module_load_init: ModuleLoadInit,
) -> Option<CudaAllocatorReport> {
    aarch64_linux_cuda.then(|| {
        CudaAllocatorReport::with_config_in_force(
            None,
            CudaAllocatorReason::NotBuilt,
            None,
            None,
            module_load_init,
        )
    })
}

// ---- settings ---------------------------------------------------------------

/// The pool mode: the `cuda_pool` open setting, or `FATHOMDB_POOL_MODE`
/// when no open sets it. Re-exported as `CudaPoolMode` by the engine, the
/// facade and the SDK.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PoolMode {
    /// The pool engages on the measured class only (the default).
    Auto,
    /// The pool engages on any integrated GPU that passes the hard gates.
    On,
    /// No pool: the 0.8.27 allocator path.
    Off,
}

impl PoolMode {
    /// Stable name: `"auto"`, `"on"` or `"off"`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::On => "on",
            Self::Off => "off",
        }
    }
}

/// A string that is not `auto`, `on` or `off`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownPoolMode {
    /// The rejected string.
    pub value: String,
}

impl fmt::Display for UnknownPoolMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cuda_pool={:?}: expected auto, on or off", self.value)
    }
}

impl std::error::Error for UnknownPoolMode {}

impl std::str::FromStr for PoolMode {
    type Err = UnknownPoolMode;

    /// Parses the exact strings `auto`, `on` and `off`.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "auto" => Ok(Self::Auto),
            "on" => Ok(Self::On),
            "off" => Ok(Self::Off),
            other => Err(UnknownPoolMode { value: other.to_owned() }),
        }
    }
}

/// A malformed or out-of-bounds pool setting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InvalidSetting {
    pub(crate) variable: &'static str,
    pub(crate) message: String,
}

fn invalid(variable: &'static str, raw: &str, expected: &str) -> InvalidSetting {
    InvalidSetting { variable, message: format!("{variable}={raw:?}: expected {expected}") }
}

pub(crate) fn parse_pool_mode(raw: Option<&str>) -> Result<PoolMode, InvalidSetting> {
    match raw {
        None | Some("auto") => Ok(PoolMode::Auto),
        Some("on") => Ok(PoolMode::On),
        Some("off") => Ok(PoolMode::Off),
        Some(other) => Err(invalid(ENV_POOL_MODE, other, "auto, on or off")),
    }
}

/// The syntax of `FATHOMDB_POOL_MAXSIZE`: bytes, `<n>G` (GiB) or `<n>M`
/// (MiB). Bounds need the device total and are checked with the gates.
pub(crate) fn parse_max_size(raw: Option<&str>) -> Result<Option<u64>, InvalidSetting> {
    let Some(raw) = raw else { return Ok(None) };
    let bad = || invalid(ENV_POOL_MAXSIZE, raw, "a byte count, <n>G or <n>M");
    let (digits, shift) = match raw.as_bytes().last() {
        Some(b'G') => (&raw[..raw.len() - 1], 30),
        Some(b'M') => (&raw[..raw.len() - 1], 20),
        _ => (raw, 0),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    let value: u64 = digits.parse().map_err(|_| bad())?;
    let bytes = value.checked_shl(shift).filter(|b| b >> shift == value).ok_or_else(bad)?;
    Ok(Some(bytes))
}

pub(crate) fn parse_release_threshold(
    raw: Option<&str>,
) -> Result<ReleaseThreshold, InvalidSetting> {
    match raw {
        None | Some("0") => Ok(ReleaseThreshold::Zero),
        Some("max") => Ok(ReleaseThreshold::Max),
        Some(other) => Err(invalid(ENV_POOL_RELEASE_THRESHOLD, other, "0 or max")),
    }
}

/// The pool settings as read, before parsing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RawPoolSettings {
    pub(crate) mode: Option<String>,
    pub(crate) max_size: Option<String>,
    pub(crate) release_threshold: Option<String>,
}

impl RawPoolSettings {
    pub(crate) fn from_env(get: impl Fn(&str) -> Option<String>) -> Self {
        Self {
            mode: get(ENV_POOL_MODE),
            max_size: get(ENV_POOL_MAXSIZE),
            release_threshold: get(ENV_POOL_RELEASE_THRESHOLD),
        }
    }
}

// ---- the configuration in force (Slice 31 § 15) ------------------------------

/// The smallest pool size an open accepts, in bytes (32 MiB).
pub const CUDA_POOL_MAX_SIZE_MIN: u64 = MAXSIZE_MIN;

/// Where a pool setting in force came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PoolSettingSource {
    /// An open (or a CLI flag) set it.
    Sdk,
    /// Its `FATHOMDB_POOL_*` environment variable, because no open set it.
    Env,
    /// Neither: the documented default.
    Default,
}

impl PoolSettingSource {
    /// Stable name: `"sdk"`, `"env"` or `"default"`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sdk => "sdk",
            Self::Env => "env",
            Self::Default => "default",
        }
    }
}

/// One pool setting's value, as a conflict names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CudaPoolSettingValue {
    /// `cuda_pool`.
    Mode(PoolMode),
    /// `cuda_pool_max_size`, in bytes.
    MaxSize(u64),
    /// `cuda_pool_release_threshold`.
    ReleaseThreshold(ReleaseThreshold),
}

impl fmt::Display for CudaPoolSettingValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mode(mode) => f.write_str(mode.as_str()),
            Self::MaxSize(bytes) => write!(f, "{bytes}"),
            Self::ReleaseThreshold(threshold) => f.write_str(threshold.as_str()),
        }
    }
}

/// An open asked for a pool setting that differs from the one already in
/// force in this process. The pool configuration is fixed once per process
/// (by the first open that passes any pool setting, or else by the
/// environment at the first GPU use) and never reset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CudaPoolSettingConflict {
    /// `"cuda_pool"`, `"cuda_pool_max_size"` or `"cuda_pool_release_threshold"`.
    pub setting: &'static str,
    /// The value this open asked for.
    pub requested: CudaPoolSettingValue,
    /// The value in force; `None` for a device-derived default size or an
    /// invalid environment value.
    pub in_force: Option<CudaPoolSettingValue>,
    /// Where the value in force came from; `None` for an invalid
    /// environment value.
    pub source: Option<PoolSettingSource>,
}

impl CudaPoolSettingConflict {
    const fn env_var(setting: &str) -> &'static str {
        match setting.as_bytes() {
            b"cuda_pool" => ENV_POOL_MODE,
            b"cuda_pool_max_size" => ENV_POOL_MAXSIZE,
            _ => ENV_POOL_RELEASE_THRESHOLD,
        }
    }
}

impl fmt::Display for CudaPoolSettingConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let setting = self.setting;
        write!(f, "{setting}={} conflicts with ", self.requested)?;
        match (self.in_force, self.source) {
            (Some(in_force), Some(source)) => write!(
                f,
                "{setting}={in_force} already in force in this process (from {})",
                source.as_str()
            )?,
            (_, Some(_)) => {
                write!(f, "the device-derived default {setting} already in force")?;
            }
            (_, None) => {
                write!(f, "the invalid {} this process started with", Self::env_var(setting))?
            }
        }
        f.write_str("; open with the same value or omit it")
    }
}

impl std::error::Error for CudaPoolSettingConflict {}

/// The pool settings one open asks for; `None` leaves a setting to the
/// environment (or the configuration already in force).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CudaPoolRequest {
    /// `cuda_pool`.
    pub mode: Option<PoolMode>,
    /// `cuda_pool_max_size`, in bytes (at least [`CUDA_POOL_MAX_SIZE_MIN`]).
    pub max_size: Option<u64>,
    /// `cuda_pool_release_threshold`.
    pub release_threshold: Option<ReleaseThreshold>,
}

impl CudaPoolRequest {
    const fn is_empty(&self) -> bool {
        self.mode.is_none() && self.max_size.is_none() && self.release_threshold.is_none()
    }
}

/// One setting in force.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Field<T> {
    Set {
        value: T,
        source: PoolSettingSource,
    },
    /// The size only: derived from device memory at the decision.
    Default,
    /// The raw environment value, which the decision refuses as before.
    InvalidEnv(String),
}

impl<T: Copy + PartialEq> Field<T> {
    fn source(&self) -> PoolSettingSource {
        match self {
            Self::Set { source, .. } => *source,
            Self::Default => PoolSettingSource::Default,
            Self::InvalidEnv(_) => PoolSettingSource::Env,
        }
    }

    /// `Ok` when an open that asks for `requested` may share this field.
    fn check(
        &self,
        setting: &'static str,
        requested: Option<T>,
        value: fn(T) -> CudaPoolSettingValue,
    ) -> Result<(), CudaPoolSettingConflict> {
        let Some(requested) = requested else { return Ok(()) };
        let (in_force, source) = match self {
            Self::Set { value: in_force, .. } if *in_force == requested => return Ok(()),
            Self::Set { value: in_force, source } => (Some(value(*in_force)), Some(*source)),
            Self::Default => (None, Some(PoolSettingSource::Default)),
            Self::InvalidEnv(_) => (None, None),
        };
        Err(CudaPoolSettingConflict { setting, requested: value(requested), in_force, source })
    }
}

fn field<T>(
    requested: Option<T>,
    raw: Option<&str>,
    parse: impl Fn(&str) -> Option<T>,
    default: Option<T>,
) -> Field<T> {
    if let Some(value) = requested {
        return Field::Set { value, source: PoolSettingSource::Sdk };
    }
    match raw {
        Some(raw) => match parse(raw) {
            Some(value) => Field::Set { value, source: PoolSettingSource::Env },
            None => Field::InvalidEnv(raw.to_owned()),
        },
        None => match default {
            Some(value) => Field::Set { value, source: PoolSettingSource::Default },
            None => Field::Default,
        },
    }
}

/// The three pool settings in force for the process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PoolConfigInForce {
    pub(crate) mode: Field<PoolMode>,
    pub(crate) max_size: Field<u64>,
    pub(crate) release_threshold: Field<ReleaseThreshold>,
}

/// The report's view of the pool configuration in force. Binding support
/// for `doctor cuda-allocator`, not an SDK surface.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PoolSettingsInForce {
    /// The mode; `None` for a malformed `FATHOMDB_POOL_MODE`.
    pub mode: Option<PoolMode>,
    /// Where the mode came from.
    pub mode_source: Option<PoolSettingSource>,
    /// Where the size came from.
    pub max_size_source: Option<PoolSettingSource>,
    /// Where the release threshold came from.
    pub release_threshold_source: Option<PoolSettingSource>,
}

impl PoolConfigInForce {
    /// Each field from the request, else its environment value, else its
    /// default; a malformed environment value is kept as `InvalidEnv`.
    fn from_request(request: &CudaPoolRequest, env: &RawPoolSettings) -> Self {
        Self {
            mode: field(
                request.mode,
                env.mode.as_deref(),
                |raw| parse_pool_mode(Some(raw)).ok(),
                Some(PoolMode::Auto),
            ),
            max_size: field(
                request.max_size,
                env.max_size.as_deref(),
                |raw| parse_max_size(Some(raw)).ok().flatten(),
                None,
            ),
            release_threshold: field(
                request.release_threshold,
                env.release_threshold.as_deref(),
                |raw| parse_release_threshold(Some(raw)).ok(),
                Some(ReleaseThreshold::Zero),
            ),
        }
    }

    pub(crate) fn report_fields(&self) -> PoolSettingsInForce {
        PoolSettingsInForce {
            mode: match self.mode {
                Field::Set { value, .. } => Some(value),
                Field::Default | Field::InvalidEnv(_) => None,
            },
            mode_source: Some(self.mode.source()),
            max_size_source: Some(self.max_size.source()),
            release_threshold_source: Some(self.release_threshold.source()),
        }
    }
}

/// The configuration that holds after an open asking for `request`: the
/// one in force when there is one (if every field the open sets matches
/// it), otherwise a new one. The first conflicting setting, in the order
/// mode, size, threshold, is the error.
pub(crate) fn resolve_pool_config(
    in_force: Option<&PoolConfigInForce>,
    request: &CudaPoolRequest,
    env: &RawPoolSettings,
) -> Result<PoolConfigInForce, CudaPoolSettingConflict> {
    let Some(in_force) = in_force else {
        return Ok(PoolConfigInForce::from_request(request, env));
    };
    in_force.mode.check("cuda_pool", request.mode, CudaPoolSettingValue::Mode)?;
    in_force.max_size.check(
        "cuda_pool_max_size",
        request.max_size,
        CudaPoolSettingValue::MaxSize,
    )?;
    in_force.release_threshold.check(
        "cuda_pool_release_threshold",
        request.release_threshold,
        CudaPoolSettingValue::ReleaseThreshold,
    )?;
    Ok(in_force.clone())
}

/// The configuration as the strings the decision parses, so the frozen
/// gate pipeline (and its "`off` wins" rule) sees exactly what it saw when
/// the settings came only from the environment.
pub(crate) fn render_raw_settings(config: &PoolConfigInForce) -> RawPoolSettings {
    fn render<T>(field: &Field<T>, show: impl Fn(&T) -> String) -> Option<String> {
        match field {
            Field::Set { value, .. } => Some(show(value)),
            Field::Default => None,
            Field::InvalidEnv(raw) => Some(raw.clone()),
        }
    }
    RawPoolSettings {
        mode: render(&config.mode, |mode| mode.as_str().to_owned()),
        max_size: render(&config.max_size, u64::to_string),
        release_threshold: render(&config.release_threshold, |threshold| {
            threshold.as_str().to_owned()
        }),
    }
}

/// The process's one pool configuration. Never cleared: the decision and
/// its pool are permanent too.
static POOL_CONFIG_IN_FORCE: OnceLock<PoolConfigInForce> = OnceLock::new();

fn env_pool_settings() -> RawPoolSettings {
    RawPoolSettings::from_env(|name| std::env::var(name).ok())
}

/// Fixes the pool configuration for an open that asks for `request`, or
/// checks it against the one already in force. An empty request never
/// fixes and never conflicts. Engine binding support, not an SDK surface:
/// callers pass their open settings through the engine.
///
/// # Errors
///
/// [`CudaPoolSettingConflict`] when a setting the request names differs
/// from the one in force.
#[doc(hidden)]
pub fn claim_pool_config(request: &CudaPoolRequest) -> Result<(), CudaPoolSettingConflict> {
    if request.is_empty() {
        return Ok(());
    }
    let env = env_pool_settings();
    let in_force =
        POOL_CONFIG_IN_FORCE.get_or_init(|| PoolConfigInForce::from_request(request, &env));
    resolve_pool_config(Some(in_force), request, &env).map(|_| ())
}

/// Fixes the pool configuration from the environment when no open did,
/// as the process's first CUDA device build does. Binding support.
#[doc(hidden)]
pub fn fix_pool_config_from_env() {
    let _ = pool_config_in_force();
}

/// The pool configuration in force, fixed from the environment if nothing
/// fixed it yet. Binding support for `doctor cuda-allocator`.
#[doc(hidden)]
#[must_use]
pub fn pool_settings_in_force() -> PoolSettingsInForce {
    pool_config_in_force().report_fields()
}

/// The configuration in force, fixed from the environment if nothing fixed
/// it yet.
pub(crate) fn pool_config_in_force() -> &'static PoolConfigInForce {
    POOL_CONFIG_IN_FORCE.get_or_init(|| {
        PoolConfigInForce::from_request(&CudaPoolRequest::default(), &env_pool_settings())
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PoolSettings {
    mode: PoolMode,
    max_size: Option<u64>,
    release_threshold: ReleaseThreshold,
}

// ---- gates and sizing -------------------------------------------------------

/// What the decision reads from the device, with its primary context retained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DeviceFacts {
    /// `CU_DEVICE_ATTRIBUTE_INTEGRATED` is 1.
    pub(crate) integrated: bool,
    /// `CU_DEVICE_ATTRIBUTE_MEMORY_POOLS_SUPPORTED` is 1.
    pub(crate) pools_supported: bool,
    /// `cuDeviceTotalMem`.
    pub(crate) total_mem: u64,
    /// Compute capability (major, minor).
    pub(crate) compute_capability: (u32, u32),
    /// [`tegra_identity`] of the host.
    pub(crate) tegra: bool,
}

/// Whether the host is a Tegra: `/etc/nv_tegra_release` exists, or the
/// device tree's `compatible` list names `nvidia,tegra`.
pub(crate) fn tegra_identity(
    nv_tegra_release_exists: bool,
    device_tree_compatible: Option<&[u8]>,
) -> bool {
    const NEEDLE: &[u8] = b"nvidia,tegra";
    nv_tegra_release_exists
        || device_tree_compatible
            .is_some_and(|compatible| compatible.windows(NEEDLE.len()).any(|w| w == NEEDLE))
}

/// The derived pool size: 1/20 of device memory between the floor and the
/// ceiling.
pub(crate) fn pool_size(total_mem: u64) -> u64 {
    (total_mem / POOL_SIZE_DEVICE_DIVISOR).clamp(POOL_SIZE_FLOOR, POOL_SIZE_CEILING)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Plan {
    Private { max_size_bytes: u64, release_threshold: ReleaseThreshold },
    Fallback { reason: CudaAllocatorReason },
}

fn init_gate(init: ModuleLoadInit) -> Result<(), CudaAllocatorReason> {
    match init {
        ModuleLoadInit::Ran => Ok(()),
        ModuleLoadInit::OptedOut => Err(CudaAllocatorReason::CuinitOptedOut),
        ModuleLoadInit::SkippedCpuOnly => Err(CudaAllocatorReason::CuinitSkippedCpuOnly),
        ModuleLoadInit::DriverAbsent => Err(CudaAllocatorReason::CuinitDriverAbsent),
        ModuleLoadInit::Failed(_) => Err(CudaAllocatorReason::CuinitFailed),
        ModuleLoadInit::NotAtLoad => Err(CudaAllocatorReason::CuinitNotAtLoad),
    }
}

/// The gates that need no driver call: mode, settings, module-load `cuInit`.
/// `off` wins over a malformed value of any other setting. The decision
/// itself calls [`pre_device_gates_with_probe`]; this form reads no probe.
#[cfg(test)]
pub(crate) fn pre_device_gates(
    raw: &RawPoolSettings,
    init: ModuleLoadInit,
) -> Result<PoolSettings, CudaAllocatorReason> {
    pre_device_gates_with_probe(raw, init, false)
}

/// [`pre_device_gates`], admitting [`ModuleLoadInit::NotAtLoad`] when the
/// driver was already initialized by a FathomDB probe (`probe_ok`): a
/// process with no load hook (plain Rust) whose first `cuInit` was its own
/// probe's. Every other load outcome keeps its gate.
pub(crate) fn pre_device_gates_with_probe(
    raw: &RawPoolSettings,
    init: ModuleLoadInit,
    probe_ok: bool,
) -> Result<PoolSettings, CudaAllocatorReason> {
    let mode =
        parse_pool_mode(raw.mode.as_deref()).map_err(|_| CudaAllocatorReason::InvalidSetting)?;
    if mode == PoolMode::Off {
        return Err(CudaAllocatorReason::ModeOff);
    }
    let max_size =
        parse_max_size(raw.max_size.as_deref()).map_err(|_| CudaAllocatorReason::InvalidSetting)?;
    let release_threshold = parse_release_threshold(raw.release_threshold.as_deref())
        .map_err(|_| CudaAllocatorReason::InvalidSetting)?;
    if !(probe_ok && init == ModuleLoadInit::NotAtLoad) {
        init_gate(init)?;
    }
    Ok(PoolSettings { mode, max_size, release_threshold })
}

/// The device gates in order, then the size. `None` facts mean the device
/// could not be read.
pub(crate) fn device_plan(settings: &PoolSettings, facts: Option<DeviceFacts>) -> Plan {
    let fallback = |reason| Plan::Fallback { reason };
    let Some(facts) = facts else { return fallback(CudaAllocatorReason::PoolCreateFailed) };
    let lifted = settings.mode == PoolMode::On;
    if !facts.integrated {
        return fallback(CudaAllocatorReason::Discrete);
    }
    if !facts.pools_supported {
        return fallback(CudaAllocatorReason::NoPools);
    }
    if POOL_SIZE_FLOOR > facts.total_mem / POOL_FLOOR_MAX_SHARE_DIVISOR {
        return fallback(CudaAllocatorReason::TooSmall);
    }
    if !lifted && !facts.tegra {
        return fallback(CudaAllocatorReason::NotTegra);
    }
    let measured = facts.compute_capability == MEASURED_CLASS_COMPUTE_CAPABILITY
        && facts.total_mem >= MEASURED_CLASS_MIN_TOTAL;
    if !lifted && !measured {
        return fallback(CudaAllocatorReason::UnmeasuredClass);
    }
    let max_size_bytes = match settings.max_size {
        Some(bytes) if (MAXSIZE_MIN..=facts.total_mem).contains(&bytes) => bytes,
        Some(_) => return fallback(CudaAllocatorReason::InvalidSetting),
        None => pool_size(facts.total_mem),
    };
    Plan::Private { max_size_bytes, release_threshold: settings.release_threshold }
}

/// The whole gate sequence of design § 2.1. `facts` is called only once
/// every gate that needs no device has passed. The driver runs the same two
/// halves itself, because the device half runs with the primary context
/// retained.
#[cfg(test)]
pub(crate) fn decide(
    raw: &RawPoolSettings,
    init: ModuleLoadInit,
    facts: impl FnOnce() -> Option<DeviceFacts>,
) -> Plan {
    match pre_device_gates(raw, init) {
        Ok(settings) => device_plan(&settings, facts()),
        Err(reason) => Plan::Fallback { reason },
    }
}

/// Whether a driver error is pool exhaustion: `CUDA_ERROR_OUT_OF_MEMORY` in
/// a process whose devices allocate from the private pool. Elsewhere it is
/// reported as before.
pub(crate) const fn is_pool_exhaustion(private: bool, driver_code: Option<u32>) -> bool {
    private && matches!(driver_code, Some(CUDA_ERROR_OUT_OF_MEMORY))
}

// ---- context loss -----------------------------------------------------------

/// The state of a device's primary context, read with
/// `cuDevicePrimaryCtxGetState`, which neither retains nor creates it.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CudaPrimaryContextState {
    /// The primary context exists.
    pub active: bool,
    /// The primary context's creation flags.
    pub flags: u32,
}

/// The private pool's `CU_MEMPOOL_ATTR_*` byte counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PoolCounters {
    pub(crate) reserved_current: u64,
    pub(crate) reserved_high: u64,
    pub(crate) used_current: u64,
    pub(crate) used_high: u64,
}

/// A failed forward pass, as the classification needs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ForwardFault {
    /// Ordinal of the device the error happened on.
    pub(crate) ordinal: usize,
    /// The error came from the CUDA backend.
    pub(crate) cuda: bool,
    /// The driver `CUresult`, when the error carries one.
    pub(crate) code: Option<u32>,
    pub(crate) error: String,
}

/// File-name fragments of the CUDA user-space libraries a snapshot lists.
const CUDA_LIBRARY_MARKERS: [&str; 7] =
    ["libcuda", "libcublas", "libnvrtc", "libnvJitLink", "libcurand", "libnvidia", "libnvrm"];

/// The CUDA library paths named in `/proc/self/maps`, in first-seen order.
pub(crate) fn cuda_libraries_in_maps(maps: &str) -> Vec<String> {
    let mut libraries: Vec<String> = Vec::new();
    for line in maps.lines() {
        // The path is the rest of the line from its first `/`; it may hold spaces.
        let Some(path) = line.find('/').map(|start| &line[start..]) else { continue };
        let name = path.rsplit('/').next().unwrap_or(path);
        if CUDA_LIBRARY_MARKERS.iter().any(|marker| name.starts_with(marker))
            && !libraries.iter().any(|seen| seen == path)
        {
            libraries.push(path.to_owned());
        }
    }
    libraries
}

fn push_json_string(out: &mut String, value: &str) {
    use fmt::Write as _;
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// What the one snapshot line of a process records.
struct LossSnapshot<'a> {
    recorded_context_id: u64,
    current_context_id: Option<u64>,
    driver_error: &'a str,
    operation: &'a str,
    primary_context: Option<CudaPrimaryContextState>,
    cuda_libraries: &'a [String],
    pool: Option<PoolCounters>,
    live_private_contexts: usize,
}

/// The snapshot line: the prefix, then one JSON object.
fn snapshot_line(snapshot: &LossSnapshot<'_>) -> String {
    use fmt::Write as _;
    let mut out = String::from(CONTEXT_LOST_SNAPSHOT_PREFIX);
    let _ = write!(
        out,
        "{{\"recorded_context_id\":{},\"current_context_id\":",
        snapshot.recorded_context_id
    );
    match snapshot.current_context_id {
        Some(id) => {
            let _ = write!(out, "{id}");
        }
        None => out.push_str("null"),
    }
    out.push_str(",\"driver_error\":");
    push_json_string(&mut out, snapshot.driver_error);
    out.push_str(",\"operation\":");
    push_json_string(&mut out, snapshot.operation);
    out.push_str(",\"primary_context\":");
    match snapshot.primary_context {
        Some(state) => {
            let _ = write!(out, "{{\"active\":{},\"flags\":{}}}", state.active, state.flags);
        }
        None => out.push_str("null"),
    }
    out.push_str(",\"cuda_libraries\":[");
    for (index, library) in snapshot.cuda_libraries.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(&mut out, library);
    }
    out.push_str("],\"pool\":");
    match snapshot.pool {
        Some(pool) => {
            let _ = write!(
                out,
                "{{\"reserved_mem_current\":{},\"reserved_mem_high\":{},\
                 \"used_mem_current\":{},\"used_mem_high\":{}}}",
                pool.reserved_current, pool.reserved_high, pool.used_current, pool.used_high
            );
        }
        None => out.push_str("null"),
    }
    let _ = write!(out, ",\"live_private_contexts\":{}}}", snapshot.live_private_contexts);
    out
}

/// Decides whether the private context of `private` is gone after a
/// non-out-of-memory CUDA error, and on the process's first loss writes one
/// snapshot line.
///
/// The state query never retains, so an inactive primary context (a reset
/// destroyed it) is never re-created. The id query retains only an active
/// context; if a reset lands between the two queries, that retain creates a
/// new context with a new id, which reads as lost, and the release that
/// follows drops it again.
fn context_loss<D: PoolDriver>(
    driver: &D,
    private: &PrivateDecision<D::Pool, D::Context>,
    code: Option<u32>,
    driver_error: String,
    operation: &str,
) -> Option<CudaPoolFailure> {
    let mut is_lost =
        matches!(code, Some(CUDA_ERROR_CONTEXT_IS_DESTROYED | CUDA_ERROR_INVALID_CONTEXT));
    let primary_context = driver.primary_state(private.ordinal).ok();
    let mut current_context_id = None;
    match primary_context {
        Some(state) if state.active => match driver.retained_primary_context_id(private.ordinal) {
            Ok(id) => {
                current_context_id = Some(id);
                is_lost |= id != private.context_id;
            }
            Err(_) => is_lost = true,
        },
        _ => is_lost = true,
    }
    if !is_lost {
        return None;
    }
    driver.snapshot_once().call_once(|| {
        // A panic here must not poison the gate for later detections.
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let cuda_libraries = driver.cuda_libraries();
            let snapshot = LossSnapshot {
                recorded_context_id: private.context_id,
                current_context_id,
                driver_error: &driver_error,
                operation,
                primary_context,
                cuda_libraries: &cuda_libraries,
                pool: driver.pool_counters(&private.pool).ok(),
                live_private_contexts: driver.live_private_contexts(&private.pool),
            };
            driver.emit_snapshot(&snapshot_line(&snapshot));
        }));
    });
    Some(CudaPoolFailure::ContextLost {
        recorded_context_id: private.context_id,
        current_context_id,
        driver_error,
        operation: operation.to_owned(),
    })
}

// ---- typed failures ---------------------------------------------------------

/// A failure of a process that allocates from the private CUDA memory pool,
/// carried by [`crate::CudaProbeError::Pool`] and the resolution errors when
/// it happens while a device is probed. Each variant is one of the three
/// CUDA pool kinds; [`CudaPoolFailure::kind`] gives its stable name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CudaPoolFailure {
    /// `cuda_pool_exhausted`: the private pool of device `ordinal` reached
    /// its cap of `max_size_bytes`.
    Exhausted {
        /// The pool's device ordinal.
        ordinal: usize,
        /// The pool's cap in bytes.
        max_size_bytes: u64,
        /// The operation and the driver error.
        message: String,
    },
    /// `cuda_context_lost`: the recorded private context is gone;
    /// `current_context_id` is `None` when the primary context is inactive.
    ContextLost {
        /// `cuCtxGetId` of the context the pool was built with.
        recorded_context_id: u64,
        /// `cuCtxGetId` of the current primary context, when active.
        current_context_id: Option<u64>,
        /// The driver error that exposed the loss.
        driver_error: String,
        /// What was running when the loss was found.
        operation: String,
    },
    /// `cuda_private_build_refused`: another private-pool context on device
    /// `ordinal` could not be built; none on another allocator replaces it.
    PrivateBuildRefused {
        /// The pool's device ordinal.
        ordinal: usize,
        /// The driver error.
        message: String,
    },
}

impl fmt::Display for CudaPoolFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exhausted { ordinal, max_size_bytes, message } => write!(
                f,
                "cuda_pool_exhausted: the private CUDA memory pool of device {ordinal} \
                 reached its cap of {max_size_bytes} bytes: {message}"
            ),
            Self::ContextLost {
                recorded_context_id,
                current_context_id,
                driver_error,
                operation,
            } => {
                write!(
                    f,
                    "cuda_context_lost: CUDA context {recorded_context_id} is gone \
                     (current: {current_context_id:?}) at {operation}: {driver_error}"
                )
            }
            Self::PrivateBuildRefused { ordinal, message } => write!(
                f,
                "cuda_private_build_refused: could not build a private-pool CUDA \
                 context on device {ordinal}: {message}"
            ),
        }
    }
}

impl std::error::Error for CudaPoolFailure {}

impl CudaPoolFailure {
    /// Stable lowercase kind: `cuda_pool_exhausted`, `cuda_context_lost` or
    /// `cuda_private_build_refused`.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Exhausted { .. } => "cuda_pool_exhausted",
            Self::ContextLost { .. } => "cuda_context_lost",
            Self::PrivateBuildRefused { .. } => "cuda_private_build_refused",
        }
    }

    /// The device ordinal; `None` for a lost context.
    #[must_use]
    pub const fn ordinal(&self) -> Option<usize> {
        match self {
            Self::Exhausted { ordinal, .. } | Self::PrivateBuildRefused { ordinal, .. } => {
                Some(*ordinal)
            }
            Self::ContextLost { .. } => None,
        }
    }

    #[cfg(any(test, feature = "default-embedder"))]
    pub(crate) fn into_embedder_error(self) -> fathomdb_embedder_api::EmbedderError {
        use fathomdb_embedder_api::EmbedderError;
        match self {
            Self::Exhausted { ordinal, max_size_bytes, message } => {
                EmbedderError::CudaPoolExhausted { ordinal, max_size_bytes, message }
            }
            Self::ContextLost {
                recorded_context_id,
                current_context_id,
                driver_error,
                operation,
            } => EmbedderError::CudaContextLost {
                recorded_context_id,
                current_context_id,
                driver_error,
                operation,
            },
            Self::PrivateBuildRefused { ordinal, message } => {
                EmbedderError::CudaPrivateBuildRefused { ordinal, message }
            }
        }
    }

    pub(crate) fn into_reranker_policy_error(self) -> crate::RerankerDevicePolicyError {
        use crate::RerankerDevicePolicyError;
        match self {
            Self::Exhausted { ordinal, max_size_bytes, message } => {
                RerankerDevicePolicyError::CudaPoolExhausted { ordinal, max_size_bytes, message }
            }
            Self::ContextLost {
                recorded_context_id,
                current_context_id,
                driver_error,
                operation,
            } => RerankerDevicePolicyError::CudaContextLost {
                recorded_context_id,
                current_context_id,
                driver_error,
                operation,
            },
            Self::PrivateBuildRefused { ordinal, message } => {
                RerankerDevicePolicyError::CudaPrivateBuildRefused { ordinal, message }
            }
        }
    }

    #[cfg(feature = "default-embedder")]
    pub(crate) fn into_embedder_load_error(self) -> crate::loader::EmbedderLoadError {
        use crate::loader::EmbedderLoadError;
        match self {
            Self::Exhausted { ordinal, max_size_bytes, message } => {
                EmbedderLoadError::CudaPoolExhausted { ordinal, max_size_bytes, message }
            }
            Self::ContextLost {
                recorded_context_id,
                current_context_id,
                driver_error,
                operation,
            } => EmbedderLoadError::CudaContextLost {
                recorded_context_id,
                current_context_id,
                driver_error,
                operation,
            },
            Self::PrivateBuildRefused { ordinal, message } => {
                EmbedderLoadError::CudaPrivateBuildRefused { ordinal, message }
            }
        }
    }

    #[cfg(feature = "default-reranker")]
    pub(crate) fn into_reranker_load_error(self) -> crate::RerankerLoadError {
        use crate::RerankerLoadError;
        match self {
            Self::Exhausted { ordinal, max_size_bytes, message } => {
                RerankerLoadError::CudaPoolExhausted { ordinal, max_size_bytes, message }
            }
            Self::ContextLost {
                recorded_context_id,
                current_context_id,
                driver_error,
                operation,
            } => RerankerLoadError::CudaContextLost {
                recorded_context_id,
                current_context_id,
                driver_error,
                operation,
            },
            Self::PrivateBuildRefused { ordinal, message } => {
                RerankerLoadError::CudaPrivateBuildRefused { ordinal, message }
            }
        }
    }
}

/// A failed device build: the 0.8.27 path's own error, or a typed failure of
/// a private-pool process.
#[derive(Debug, PartialEq)]
pub(crate) enum DeviceBuildError<E> {
    Default(E),
    Pool(CudaPoolFailure),
}

impl<E: fmt::Display> fmt::Display for DeviceBuildError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default(error) => error.fmt(f),
            Self::Pool(failure) => failure.fmt(f),
        }
    }
}

// ---- the decision -----------------------------------------------------------

/// A driver call's failure: the raw `CUresult` when there is one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DriverFailure {
    pub(crate) code: Option<u32>,
    pub(crate) message: String,
}

/// Everything the decision and the device builds ask of the CUDA driver.
/// The production implementation is cudarc's; tests inject their own.
pub(crate) trait PoolDriver {
    type Pool;
    type Context: Clone;
    type Device;
    type DefaultError;

    fn raw_settings(&self) -> RawPoolSettings;
    fn module_load_init(&self) -> ModuleLoadInit;
    /// Whether the process's most recent `cuInit` attempt, by any FathomDB
    /// caller (in a Rust process, the device probe), succeeded. Admits a
    /// process with no module-load record (design § 14.2, owner ruling Q36).
    fn driver_initialized(&self) -> bool {
        false
    }
    /// Runs `body` with device `ordinal`'s primary context retained and
    /// current, and releases it afterwards.
    fn with_primary_retained<R>(
        &self,
        ordinal: usize,
        body: impl FnOnce() -> R,
    ) -> Result<R, DriverFailure>;
    fn device_facts(&self, ordinal: usize) -> Result<DeviceFacts, DriverFailure>;
    fn create_pool(
        &self,
        ordinal: usize,
        max_size_bytes: u64,
        threshold: ReleaseThreshold,
    ) -> Result<Self::Pool, DriverFailure>;
    /// Allocates and frees [`PROBE_BYTES`] on `pool`.
    fn probe_pool(&self, pool: &Self::Pool) -> Result<(), DriverFailure>;
    fn new_private_context(
        &self,
        ordinal: usize,
        pool: &Self::Pool,
    ) -> Result<Self::Context, DriverFailure>;
    /// `cuCtxGetId` of the context.
    fn context_id(&self, context: &Self::Context) -> Result<u64, DriverFailure>;
    fn wrap_context(&self, context: Self::Context) -> Result<Self::Device, DriverFailure>;
    /// The 0.8.27 path: cudarc's own default-pool-or-synchronous decision.
    fn default_device(&self, ordinal: usize) -> Result<Self::Device, Self::DefaultError>;
    /// `cuDevicePrimaryCtxGetState`; never retains or creates the context.
    fn primary_state(&self, ordinal: usize) -> Result<CudaPrimaryContextState, DriverFailure>;
    /// Retains the primary context, reads its `cuCtxGetId`, and releases it.
    /// Called only when the context is active.
    fn retained_primary_context_id(&self, ordinal: usize) -> Result<u64, DriverFailure>;
    fn pool_counters(&self, pool: &Self::Pool) -> Result<PoolCounters, DriverFailure>;
    /// The contexts built on `pool` that are still alive.
    fn live_private_contexts(&self, pool: &Self::Pool) -> usize;
    /// The CUDA libraries mapped into the process.
    fn cuda_libraries(&self) -> Vec<String>;
    /// The process's one-snapshot gate.
    fn snapshot_once(&self) -> &Once;
    fn emit_snapshot(&self, line: &str);
}

pub(crate) struct PrivateDecision<P, C> {
    pub(crate) ordinal: usize,
    pub(crate) pool: P,
    pub(crate) first_context: C,
    first_context_handed_out: AtomicBool,
    /// FathomDB models open on this pool (design § 14.1): the close release
    /// runs when the last one closes.
    pub(crate) live_models: AtomicUsize,
    /// `cuCtxGetId` of the first context, which is the primary context;
    /// context-loss detection compares the current primary context with it.
    pub(crate) context_id: u64,
    pub(crate) max_size_bytes: u64,
    pub(crate) release_threshold: ReleaseThreshold,
    pub(crate) module_load_init: ModuleLoadInit,
}

/// The process's one allocator decision; immutable once made.
pub(crate) enum Decision<P, C> {
    Private(PrivateDecision<P, C>),
    Fallback { reason: CudaAllocatorReason, module_load_init: ModuleLoadInit },
}

impl<P, C> Decision<P, C> {
    /// The report for a device on `ordinal` whose context allocates by `path`.
    pub(crate) fn report(
        &self,
        ordinal: usize,
        path: Option<CudaAllocatorPath>,
    ) -> CudaAllocatorReport {
        match self {
            Self::Private(private) if private.ordinal == ordinal => {
                CudaAllocatorReport::with_config_in_force(
                    path,
                    CudaAllocatorReason::PrivatePool,
                    Some(private.max_size_bytes),
                    Some(private.release_threshold),
                    private.module_load_init,
                )
            }
            Self::Private(private) => CudaAllocatorReport::with_config_in_force(
                path,
                CudaAllocatorReason::OtherOrdinal,
                None,
                None,
                private.module_load_init,
            ),
            Self::Fallback { reason, module_load_init } => {
                CudaAllocatorReport::with_config_in_force(
                    path,
                    *reason,
                    None,
                    None,
                    *module_load_init,
                )
            }
        }
    }

    fn private(&self) -> Option<&PrivateDecision<P, C>> {
        match self {
            Self::Private(private) => Some(private),
            Self::Fallback { .. } => None,
        }
    }
}

/// Reads the settings and the module-load record, runs the gates, and on a
/// pass creates the pool, probes it and builds the first private context.
/// Every failure is a fallback with its reason; a pool that is not kept is
/// dropped, which destroys it.
pub(crate) fn decide_and_build<D: PoolDriver>(
    driver: &D,
    ordinal: usize,
) -> Decision<D::Pool, D::Context> {
    let module_load_init = driver.module_load_init();
    let fallback = |reason| Decision::Fallback { reason, module_load_init };
    let settings = match pre_device_gates_with_probe(
        &driver.raw_settings(),
        module_load_init,
        driver.driver_initialized(),
    ) {
        Ok(settings) => settings,
        Err(reason) => return fallback(reason),
    };
    let built = driver.with_primary_retained(ordinal, || {
        let plan = device_plan(&settings, driver.device_facts(ordinal).ok());
        let (max_size_bytes, release_threshold) = match plan {
            Plan::Private { max_size_bytes, release_threshold } => {
                (max_size_bytes, release_threshold)
            }
            Plan::Fallback { reason } => return Err(reason),
        };
        let pool = driver
            .create_pool(ordinal, max_size_bytes, release_threshold)
            .map_err(|_| CudaAllocatorReason::PoolCreateFailed)?;
        driver.probe_pool(&pool).map_err(|_| CudaAllocatorReason::ProbeFailed)?;
        let first_context = driver
            .new_private_context(ordinal, &pool)
            .map_err(|_| CudaAllocatorReason::PrivateBuildFailed)?;
        let context_id = driver
            .context_id(&first_context)
            .map_err(|_| CudaAllocatorReason::PrivateBuildFailed)?;
        Ok(PrivateDecision {
            ordinal,
            pool,
            first_context,
            first_context_handed_out: AtomicBool::new(false),
            live_models: AtomicUsize::new(0),
            context_id,
            max_size_bytes,
            release_threshold,
            module_load_init,
        })
    });
    match built {
        Ok(Ok(private)) => Decision::Private(private),
        Ok(Err(reason)) => fallback(reason),
        Err(_) => fallback(CudaAllocatorReason::PoolCreateFailed),
    }
}

/// The process's decision, made by the first caller. Concurrent first
/// callers block until it is made; a panic while deciding becomes
/// `decision_panicked` instead of poisoning the cell.
pub(crate) fn decision<'a, D: PoolDriver>(
    cell: &'a OnceLock<Decision<D::Pool, D::Context>>,
    driver: &D,
    ordinal: usize,
) -> &'a Decision<D::Pool, D::Context> {
    cell.get_or_init(|| {
        catch_unwind(AssertUnwindSafe(|| decide_and_build(driver, ordinal))).unwrap_or_else(|_| {
            Decision::Fallback {
                reason: CudaAllocatorReason::DecisionPanicked,
                module_load_init: catch_unwind(AssertUnwindSafe(|| driver.module_load_init()))
                    .unwrap_or(ModuleLoadInit::NotAtLoad),
            }
        })
    })
}

/// Operation named by a context loss found while building a device.
pub(crate) const DEVICE_BUILD_OPERATION: &str = "device build";

/// Classifies a failed build after the decision is private: exhaustion, a
/// lost context, or a refusal.
fn private_build_failure<D: PoolDriver>(
    driver: &D,
    private: &PrivateDecision<D::Pool, D::Context>,
    failure: DriverFailure,
) -> CudaPoolFailure {
    if is_pool_exhaustion(true, failure.code) {
        return CudaPoolFailure::Exhausted {
            ordinal: private.ordinal,
            max_size_bytes: private.max_size_bytes,
            message: failure.message,
        };
    }
    context_loss(driver, private, failure.code, failure.message.clone(), DEVICE_BUILD_OPERATION)
        .unwrap_or(CudaPoolFailure::PrivateBuildRefused {
            ordinal: private.ordinal,
            message: failure.message,
        })
}

/// Builds a device on `ordinal` under the process's decision: on the private
/// pool when the decision is private for this ordinal (the first build takes
/// the decision's own context), otherwise on the 0.8.27 path.
pub(crate) fn build_device<D: PoolDriver>(
    cell: &OnceLock<Decision<D::Pool, D::Context>>,
    driver: &D,
    ordinal: usize,
) -> Result<D::Device, DeviceBuildError<D::DefaultError>> {
    match decision(cell, driver, ordinal) {
        Decision::Private(private) if private.ordinal == ordinal => {
            let built = if private.first_context_handed_out.swap(true, Ordering::AcqRel) {
                driver
                    .new_private_context(ordinal, &private.pool)
                    .and_then(|context| driver.wrap_context(context))
            } else {
                driver.wrap_context(private.first_context.clone())
            };
            built.map_err(|failure| {
                DeviceBuildError::Pool(private_build_failure(driver, private, failure))
            })
        }
        _ => driver.default_device(ordinal).map_err(DeviceBuildError::Default),
    }
}

// ---- the close release (Slice 31 § 14.1, owner rulings Q29, Q34) ------------

/// The driver calls of the close release. No default bodies, so the real
/// driver cannot silently skip them.
pub(crate) trait PoolRelease: PoolDriver {
    /// Waits for all work on `context` (the device's primary context, which
    /// every private context shares).
    fn synchronize(&self, context: &Self::Context) -> Result<(), DriverFailure>;
    /// Returns the pool's unused memory to the system (`trim_to(0)`).
    fn trim(&self, pool: &Self::Pool) -> Result<(), DriverFailure>;
}

/// What one close release did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReleaseOutcome {
    /// Another FathomDB model is still open on the pool.
    StillOpen,
    /// No private decision: nothing to release.
    NotPrivate,
    /// The private context is gone; touching it could fault.
    ContextLost,
    /// A driver call failed; the error is discarded.
    DriverError,
    /// The release panicked; the panic is caught.
    Panicked,
    /// Synchronized and trimmed.
    Trimmed,
}

/// Counts a model built on device `ordinal` when it allocates from the
/// decided private pool (`private_path`). Returns whether it counts; a CPU,
/// 0.8.27-path or other-ordinal model gets an inert guard.
pub(crate) fn register_private_model<P, C>(
    decision: Option<&Decision<P, C>>,
    ordinal: usize,
    private_path: bool,
) -> bool {
    match private_for(decision, ordinal) {
        Some(private) if private_path => {
            private.live_models.fetch_add(1, Ordering::AcqRel);
            true
        }
        _ => false,
    }
}

/// Uncounts a closed model; when it was the last one open, releases the
/// pool's unused memory.
pub(crate) fn release_private_model<D: PoolRelease>(
    decision: Option<&Decision<D::Pool, D::Context>>,
    driver: &D,
) -> ReleaseOutcome {
    let Some(private) = decision.and_then(Decision::private) else {
        return ReleaseOutcome::NotPrivate;
    };
    let previous = private
        .live_models
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |live| live.checked_sub(1));
    match previous {
        Ok(1) => release_unused_pool_memory(decision, driver),
        Ok(_) | Err(_) => ReleaseOutcome::StillOpen,
    }
}

/// Whether the decision's context is still the device's primary context:
/// active, with the recorded id. Never creates a context.
fn context_alive<D: PoolDriver>(
    driver: &D,
    private: &PrivateDecision<D::Pool, D::Context>,
) -> bool {
    match driver.primary_state(private.ordinal) {
        Ok(state) if state.active => driver
            .retained_primary_context_id(private.ordinal)
            .is_ok_and(|id| id == private.context_id),
        _ => false,
    }
}

/// Synchronizes the decision's context and trims the pool's unused memory,
/// unless the decision is not private or its context is lost. Errors and
/// panics are discarded: a model's drop must never fail or unwind.
pub(crate) fn release_unused_pool_memory<D: PoolRelease>(
    decision: Option<&Decision<D::Pool, D::Context>>,
    driver: &D,
) -> ReleaseOutcome {
    let Some(private) = decision.and_then(Decision::private) else {
        return ReleaseOutcome::NotPrivate;
    };
    catch_unwind(AssertUnwindSafe(|| {
        if !context_alive(driver, private) {
            return ReleaseOutcome::ContextLost;
        }
        if driver.synchronize(&private.first_context).is_err()
            || driver.trim(&private.pool).is_err()
        {
            return ReleaseOutcome::DriverError;
        }
        ReleaseOutcome::Trimmed
    }))
    .unwrap_or(ReleaseOutcome::Panicked)
}

/// The private decision when its pool belongs to device `ordinal`. A device
/// on another ordinal allocates on the 0.8.27 path, so its errors keep their
/// untyped report.
fn private_for<P, C>(
    decision: Option<&Decision<P, C>>,
    ordinal: usize,
) -> Option<&PrivateDecision<P, C>> {
    decision.and_then(Decision::private).filter(|private| private.ordinal == ordinal)
}

/// The typed failure for a forward error with driver code `driver_code` on
/// device `ordinal`, or `None` when it keeps its untyped report.
pub(crate) fn classify_forward<P, C>(
    decision: Option<&Decision<P, C>>,
    ordinal: usize,
    driver_code: Option<u32>,
    message: String,
) -> Option<CudaPoolFailure> {
    let private = private_for(decision, ordinal);
    is_pool_exhaustion(private.is_some(), driver_code).then(|| {
        let private = private.expect("checked by is_pool_exhaustion");
        CudaPoolFailure::Exhausted {
            ordinal: private.ordinal,
            max_size_bytes: private.max_size_bytes,
            message,
        }
    })
}

/// The typed failure for a failed forward pass, or `None` when it keeps its
/// untyped report: exhaustion first, then, for any other CUDA error on the
/// private pool's device, context-loss detection.
pub(crate) fn classify_forward_fault<D: PoolDriver>(
    decision: Option<&Decision<D::Pool, D::Context>>,
    driver: &D,
    fault: ForwardFault,
    operation: &str,
) -> Option<CudaPoolFailure> {
    let exhausted = classify_forward(
        decision,
        fault.ordinal,
        fault.code,
        format!("{operation}: {}", fault.error),
    );
    if exhausted.is_some() {
        return exhausted;
    }
    let private = private_for(decision, fault.ordinal)?;
    if !fault.cuda {
        return None;
    }
    context_loss(driver, private, fault.code, fault.error, operation)
}

// ---- the candle-facing entry points -----------------------------------------

/// Operation named by a failure while loading model weights onto a device.
pub(crate) const MODEL_LOAD_OPERATION: &str = "model load";
/// Operation named by a failure of the resolution probe's allocation.
pub(crate) const PROBE_OPERATION: &str = "device probe";

/// The driver `CUresult` inside a Candle error, looking through Candle's
/// context, path and backtrace wrappers.
#[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
pub(crate) fn candle_driver_code(error: &candle_core::Error) -> Option<u32> {
    use candle_core::{
        cuda::{cudarc::driver::DriverError, CudaError},
        Error,
    };
    match error {
        Error::Cuda(source) => match source.downcast_ref::<CudaError>() {
            Some(CudaError::Cuda(DriverError(code))) => Some(*code as u32),
            _ => None,
        },
        Error::Context { inner, .. }
        | Error::WithPath { inner, .. }
        | Error::WithBacktrace { inner, .. } => candle_driver_code(inner),
        _ => None,
    }
}

/// Whether a Candle error came from the CUDA backend, looking through the
/// same wrappers as [`candle_driver_code`].
#[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
fn is_candle_cuda_error(error: &candle_core::Error) -> bool {
    use candle_core::Error;
    match error {
        Error::Cuda(_) => true,
        Error::Context { inner, .. }
        | Error::WithPath { inner, .. }
        | Error::WithBacktrace { inner, .. } => is_candle_cuda_error(inner),
        _ => false,
    }
}

/// The typed failure for a Candle `error` on device `ordinal` during
/// `operation`, or `None` when it keeps its untyped report.
#[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
pub(crate) fn classify_candle_fault<D: PoolDriver>(
    decision: Option<&Decision<D::Pool, D::Context>>,
    driver: &D,
    error: &candle_core::Error,
    ordinal: usize,
    operation: &str,
) -> Option<CudaPoolFailure> {
    let fault = ForwardFault {
        ordinal,
        cuda: is_candle_cuda_error(error),
        code: candle_driver_code(error),
        error: error.to_string(),
    };
    classify_forward_fault(decision, driver, fault, operation)
}

#[cfg(any(feature = "default-embedder", feature = "default-reranker"))]
pub(crate) type CudaDeviceError = DeviceBuildError<candle_core::Error>;

/// The embedder error for a failed forward pass on `device`: a typed pool
/// failure, or `Failed` with `what` and the error as before.
#[cfg(all(
    any(test, feature = "default-embedder"),
    any(feature = "default-embedder", feature = "default-reranker")
))]
pub(crate) fn forward_error(
    error: candle_core::Error,
    device: &candle_core::Device,
    what: &str,
) -> fathomdb_embedder_api::EmbedderError {
    match forward_failure(&error, device, what) {
        Some(failure) => failure.into_embedder_error(),
        None => {
            fathomdb_embedder_api::EmbedderError::Failed { message: format!("{what}: {error}") }
        }
    }
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
mod fallback_only {
    #[cfg(any(test, feature = "embed-cuda", feature = "rerank-cuda"))]
    use super::{not_built_report, CudaAllocatorReport};
    use super::{CudaDeviceError, CudaPoolFailure, DeviceBuildError};
    use candle_core::Device;

    /// `Device::new_cuda`: without the driver part there is no other path.
    /// The first build still fixes the pool configuration, so the
    /// one-configuration-per-process rule holds on every platform.
    pub(crate) fn new_cuda_device(ordinal: usize) -> Result<Device, CudaDeviceError> {
        super::fix_pool_config_from_env();
        Device::new_cuda(ordinal).map_err(DeviceBuildError::Default)
    }

    /// Without the driver part no model allocates from a private pool.
    pub(crate) struct PrivatePoolModelGuard;

    impl PrivatePoolModelGuard {
        pub(crate) const fn new(_device: &Device) -> Self {
            Self
        }
    }

    pub(crate) fn forward_failure(
        _error: &candle_core::Error,
        _device: &Device,
        _what: &str,
    ) -> Option<CudaPoolFailure> {
        None
    }

    #[cfg(any(test, feature = "embed-cuda", feature = "rerank-cuda"))]
    pub(crate) fn allocator_report(_device: &Device) -> Option<CudaAllocatorReport> {
        not_built_report(
            cfg!(all(
                target_os = "linux",
                target_arch = "aarch64",
                any(feature = "embed-cuda", feature = "rerank-cuda")
            )),
            crate::cuda_driver_init::recorded_module_load_init(),
        )
    }
}

#[cfg(all(
    any(test, feature = "embed-cuda", feature = "rerank-cuda"),
    any(feature = "default-embedder", feature = "default-reranker"),
    not(all(
        feature = "tegra-pool",
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    ))
))]
pub(crate) use fallback_only::allocator_report;
#[cfg(all(
    any(feature = "default-embedder", feature = "default-reranker"),
    not(all(
        feature = "tegra-pool",
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    ))
))]
pub(crate) use fallback_only::{forward_failure, new_cuda_device, PrivatePoolModelGuard};

#[cfg(all(
    feature = "tegra-pool",
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
mod driver {
    use std::sync::{Arc, Once, OnceLock};

    use candle_core::cuda::cudarc::driver::{
        result, sys, AllocMode, CudaContext, CudaMemPool, DriverError, MemPoolProps,
    };
    use candle_core::Device;

    use super::{
        build_device, candle_driver_code, classify_candle_fault, cuda_libraries_in_maps,
        pool_config_in_force, register_private_model, release_private_model, render_raw_settings,
        tegra_identity, CudaAllocatorPath, CudaAllocatorReport, CudaDeviceError, CudaPoolFailure,
        CudaPrimaryContextState, Decision, DeviceFacts, DriverFailure, ModuleLoadInit,
        PoolCounters, PoolDriver, PoolRelease, RawPoolSettings, ReleaseThreshold, PROBE_BYTES,
    };

    static DECISION: OnceLock<Decision<Arc<CudaMemPool>, Arc<CudaContext>>> = OnceLock::new();
    static SNAPSHOT: Once = Once::new();

    struct CudarcDriver;

    fn failure(error: DriverError) -> DriverFailure {
        DriverFailure { code: Some(error.0 as u32), message: error.to_string() }
    }

    fn candle_failure(error: candle_core::Error) -> DriverFailure {
        DriverFailure { code: candle_driver_code(&error), message: error.to_string() }
    }

    /// Releases a retained primary context when dropped, so a panic in the
    /// decision does not leak the reference.
    struct RetainedPrimary(sys::CUdevice);

    impl Drop for RetainedPrimary {
        fn drop(&mut self) {
            // SAFETY: the reference was retained by `with_primary_retained`.
            let _ = unsafe { result::primary_ctx::release(self.0) };
        }
    }

    fn attribute(
        device: sys::CUdevice,
        attr: sys::CUdevice_attribute,
    ) -> Result<i32, DriverFailure> {
        // SAFETY: device comes from cuDeviceGet.
        unsafe { result::device::get_attribute(device, attr) }.map_err(failure)
    }

    impl PoolDriver for CudarcDriver {
        type Pool = Arc<CudaMemPool>;
        type Context = Arc<CudaContext>;
        type Device = Device;
        type DefaultError = candle_core::Error;

        /// The configuration in force, fixed here from the environment when
        /// no open fixed it (design § 15.2).
        fn raw_settings(&self) -> RawPoolSettings {
            render_raw_settings(pool_config_in_force())
        }

        fn module_load_init(&self) -> ModuleLoadInit {
            crate::cuda_driver_init::recorded_module_load_init()
        }

        fn driver_initialized(&self) -> bool {
            crate::cuda_driver_init::last_cuda_driver_init()
                == Some(crate::cuda_driver_init::CudaDriverInit::Initialized)
        }

        fn with_primary_retained<R>(
            &self,
            ordinal: usize,
            body: impl FnOnce() -> R,
        ) -> Result<R, DriverFailure> {
            let device = result::device::get(ordinal as i32).map_err(failure)?;
            // SAFETY: device comes from cuDeviceGet; the guard releases it.
            let context = unsafe { result::primary_ctx::retain(device) }.map_err(failure)?;
            let _retained = RetainedPrimary(device);
            // SAFETY: context was just retained.
            unsafe { result::ctx::set_current(context) }.map_err(failure)?;
            Ok(body())
        }

        fn device_facts(&self, ordinal: usize) -> Result<DeviceFacts, DriverFailure> {
            use sys::CUdevice_attribute::*;
            let device = result::device::get(ordinal as i32).map_err(failure)?;
            let integrated = attribute(device, CU_DEVICE_ATTRIBUTE_INTEGRATED)?;
            let pools = attribute(device, CU_DEVICE_ATTRIBUTE_MEMORY_POOLS_SUPPORTED)?;
            let major = attribute(device, CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR)?;
            let minor = attribute(device, CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MINOR)?;
            // SAFETY: as above.
            let total_mem = unsafe { result::device::total_mem(device) }.map_err(failure)?;
            let compatible = std::fs::read("/proc/device-tree/compatible").ok();
            Ok(DeviceFacts {
                integrated: integrated == 1,
                pools_supported: pools == 1,
                total_mem: total_mem as u64,
                compute_capability: (
                    u32::try_from(major).unwrap_or(0),
                    u32::try_from(minor).unwrap_or(0),
                ),
                tegra: tegra_identity(
                    std::path::Path::new("/etc/nv_tegra_release").exists(),
                    compatible.as_deref(),
                ),
            })
        }

        fn create_pool(
            &self,
            ordinal: usize,
            max_size_bytes: u64,
            threshold: ReleaseThreshold,
        ) -> Result<Self::Pool, DriverFailure> {
            let max_size = usize::try_from(max_size_bytes).map_err(|_| DriverFailure {
                code: None,
                message: format!("pool size {max_size_bytes} does not fit in usize"),
            })?;
            let props = MemPoolProps { max_size, release_threshold: threshold.bytes() };
            CudaMemPool::create(ordinal, &props).map(Arc::new).map_err(failure)
        }

        fn probe_pool(&self, pool: &Self::Pool) -> Result<(), DriverFailure> {
            // SAFETY: the pool is live and the primary context is current
            // (`with_primary_retained`); the pointer is freed on the stream
            // that allocated it before synchronizing.
            unsafe {
                let pointer =
                    result::mem_pool::alloc_async(pool.raw(), PROBE_BYTES, std::ptr::null_mut())
                        .map_err(failure)?;
                result::free_async(pointer, std::ptr::null_mut()).map_err(failure)?;
            }
            result::ctx::synchronize().map_err(failure)
        }

        fn new_private_context(
            &self,
            ordinal: usize,
            pool: &Self::Pool,
        ) -> Result<Self::Context, DriverFailure> {
            CudaContext::new_with_mem_pool(ordinal, Arc::clone(pool)).map_err(failure)
        }

        fn context_id(&self, context: &Self::Context) -> Result<u64, DriverFailure> {
            let mut id: std::ffi::c_ulonglong = 0;
            // SAFETY: the context is live; `id` is a valid out-pointer.
            unsafe { sys::cuCtxGetId(context.cu_ctx(), &mut id) }.result().map_err(failure)?;
            Ok(id)
        }

        fn wrap_context(&self, context: Self::Context) -> Result<Self::Device, DriverFailure> {
            Device::new_cuda_from_context(context).map_err(candle_failure)
        }

        fn default_device(&self, ordinal: usize) -> Result<Self::Device, Self::DefaultError> {
            Device::new_cuda(ordinal)
        }

        fn primary_state(&self, ordinal: usize) -> Result<CudaPrimaryContextState, DriverFailure> {
            let device = result::device::get(ordinal as i32).map_err(failure)?;
            let mut flags: std::ffi::c_uint = 0;
            let mut active: std::ffi::c_int = 0;
            // SAFETY: device comes from cuDeviceGet; both are valid out-pointers.
            unsafe { sys::cuDevicePrimaryCtxGetState(device, &mut flags, &mut active) }
                .result()
                .map_err(failure)?;
            Ok(CudaPrimaryContextState { active: active != 0, flags })
        }

        fn retained_primary_context_id(&self, ordinal: usize) -> Result<u64, DriverFailure> {
            let device = result::device::get(ordinal as i32).map_err(failure)?;
            // SAFETY: device comes from cuDeviceGet; the guard releases it.
            let context = unsafe { result::primary_ctx::retain(device) }.map_err(failure)?;
            let _retained = RetainedPrimary(device);
            let mut id: std::ffi::c_ulonglong = 0;
            // SAFETY: the context is retained; `id` is a valid out-pointer.
            unsafe { sys::cuCtxGetId(context, &mut id) }.result().map_err(failure)?;
            Ok(id)
        }

        fn pool_counters(&self, pool: &Self::Pool) -> Result<PoolCounters, DriverFailure> {
            Ok(PoolCounters {
                reserved_current: pool.reserved_current().map_err(failure)?,
                reserved_high: pool.reserved_high().map_err(failure)?,
                used_current: pool.used_current().map_err(failure)?,
                used_high: pool.used_high().map_err(failure)?,
            })
        }

        /// Every private context holds one `Arc` of the pool (cudarc's
        /// `CudaContext::mem_pool`), and the decision holds one more; slices
        /// and streams reach the pool only through their context. So the live
        /// private contexts are the strong count minus the decision's handle.
        fn live_private_contexts(&self, pool: &Self::Pool) -> usize {
            Arc::strong_count(pool).saturating_sub(1)
        }

        fn cuda_libraries(&self) -> Vec<String> {
            std::fs::read_to_string("/proc/self/maps")
                .map(|maps| cuda_libraries_in_maps(&maps))
                .unwrap_or_default()
        }

        fn snapshot_once(&self) -> &Once {
            &SNAPSHOT
        }

        fn emit_snapshot(&self, line: &str) {
            eprintln!("{line}");
        }
    }

    impl PoolRelease for CudarcDriver {
        fn synchronize(&self, context: &Self::Context) -> Result<(), DriverFailure> {
            context.synchronize().map_err(failure)
        }

        fn trim(&self, pool: &Self::Pool) -> Result<(), DriverFailure> {
            pool.trim_to(0).map_err(failure)
        }
    }

    /// Counts one FathomDB model on the private pool while it is open. A
    /// model holds it as its last field, so it drops after the model's
    /// buffers and device; the last one to drop releases the pool's unused
    /// memory (design § 14.1). A CPU, 0.8.27-path or other-ordinal model
    /// holds an inert guard.
    pub(crate) struct PrivatePoolModelGuard {
        counted: bool,
    }

    impl PrivatePoolModelGuard {
        pub(crate) fn new(device: &Device) -> Self {
            let counted = device.as_cuda_device().ok().is_some_and(|cuda| {
                let context = cuda.cuda_stream().context().clone();
                register_private_model(
                    DECISION.get(),
                    context.ordinal(),
                    context.alloc_mode() == AllocMode::Private,
                )
            });
            Self { counted }
        }
    }

    impl Drop for PrivatePoolModelGuard {
        fn drop(&mut self) {
            if self.counted {
                let _ = release_private_model(DECISION.get(), &CudarcDriver);
            }
        }
    }

    /// A Candle CUDA device on `ordinal` under the process's allocator
    /// decision, made here on first use.
    pub(crate) fn new_cuda_device(ordinal: usize) -> Result<Device, CudaDeviceError> {
        build_device(&DECISION, &CudarcDriver, ordinal)
    }

    /// The typed failure for `error` on `device`; `None` for a device that
    /// is not CUDA.
    pub(crate) fn forward_failure(
        error: &candle_core::Error,
        device: &Device,
        what: &str,
    ) -> Option<CudaPoolFailure> {
        let candle_core::DeviceLocation::Cuda { gpu_id } = device.location() else {
            return None;
        };
        classify_candle_fault(DECISION.get(), &CudarcDriver, error, gpu_id, what)
    }

    /// The primary context state of `ordinal`, or `None` when the driver
    /// cannot be read (for example, before `cuInit`). Never creates a context.
    pub(crate) fn cuda_context_state(ordinal: usize) -> Option<CudaPrimaryContextState> {
        // A build that cannot load libcuda panics in cudarc's dynamic loader.
        std::panic::catch_unwind(|| CudarcDriver.primary_state(ordinal).ok()).ok().flatten()
    }

    /// The report for a CUDA device built by [`new_cuda_device`], with the
    /// path read from its context; `None` for any other device.
    pub(crate) fn allocator_report(device: &Device) -> Option<CudaAllocatorReport> {
        let context = device.as_cuda_device().ok()?.cuda_stream().context().clone();
        let path = match context.alloc_mode() {
            AllocMode::Private => CudaAllocatorPath::Private,
            AllocMode::Default => CudaAllocatorPath::DefaultPool,
            AllocMode::Synchronous => CudaAllocatorPath::Synchronous,
        };
        DECISION.get().map(|decision| decision.report(context.ordinal(), Some(path)))
    }

    /// The private pool's counters and its decided max size, after
    /// synchronizing the decision's first context; `None` without a private
    /// decision.
    #[cfg(test)]
    pub(crate) fn private_pool_counters() -> Result<Option<(PoolCounters, u64)>, DriverFailure> {
        let Some(Decision::Private(private)) = DECISION.get() else {
            return Ok(None);
        };
        private.first_context.synchronize().map_err(failure)?;
        let counters = CudarcDriver.pool_counters(&private.pool)?;
        Ok(Some((counters, private.max_size_bytes)))
    }

    /// The private pool's counters read without any synchronization, so a
    /// release that waits for the next synchronization point stays visible;
    /// `None` without a private decision.
    #[cfg(test)]
    pub(crate) fn private_pool_counters_unsynchronized(
    ) -> Result<Option<PoolCounters>, DriverFailure> {
        let Some(Decision::Private(private)) = DECISION.get() else {
            return Ok(None);
        };
        CudarcDriver.pool_counters(&private.pool).map(Some)
    }

    /// The device default pool's `reserved_current` and `used_high` on
    /// `ordinal`. Needs an initialized driver.
    #[cfg(test)]
    pub(crate) fn default_pool_counters(ordinal: usize) -> Result<(u64, u64), DriverFailure> {
        let device = result::device::get(ordinal as i32).map_err(failure)?;
        // SAFETY: device comes from cuDeviceGet.
        let pool = unsafe { result::device::get_default_mem_pool(device) }.map_err(failure)?;
        let read = |attr: sys::CUmemPool_attribute| -> Result<u64, DriverFailure> {
            let mut value: u64 = 0;
            // SAFETY: the default pool lives as long as the device; both
            // attributes are `cuuint64_t`.
            unsafe { result::mem_pool::get_attribute(pool, attr, (&mut value as *mut u64).cast()) }
                .map_err(failure)?;
            Ok(value)
        };
        Ok((
            read(sys::CUmemPool_attribute::CU_MEMPOOL_ATTR_RESERVED_MEM_CURRENT)?,
            read(sys::CUmemPool_attribute::CU_MEMPOOL_ATTR_USED_MEM_HIGH)?,
        ))
    }
}

#[cfg(all(
    test,
    feature = "tegra-pool",
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
pub(crate) use driver::{
    default_pool_counters, private_pool_counters, private_pool_counters_unsynchronized,
};

#[cfg(all(
    feature = "tegra-pool",
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
pub(crate) use driver::{
    allocator_report, forward_failure, new_cuda_device, PrivatePoolModelGuard,
};

/// The primary context state of device `ordinal`, for `doctor
/// cuda-allocator`. Never retains or creates a context. `None` when this
/// build has no Tegra private-pool driver part (`tegra-pool` with
/// `embed-cuda` or `rerank-cuda` on aarch64 Linux), or when the driver is
/// not initialized or cannot be read.
#[doc(hidden)]
#[must_use]
pub fn cuda_context_state(ordinal: usize) -> Option<CudaPrimaryContextState> {
    #[cfg(all(
        feature = "tegra-pool",
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    ))]
    {
        driver::cuda_context_state(ordinal)
    }
    #[cfg(not(all(
        feature = "tegra-pool",
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    )))]
    {
        let _ = ordinal;
        None
    }
}

#[cfg(test)]
#[path = "cuda_pool_policy_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "cuda_pool_slice31_tests.rs"]
mod slice31_tests;

#[cfg(test)]
#[path = "cuda_pool_test_fake.rs"]
mod test_fake;

#[cfg(test)]
#[path = "cuda_pool_release_tests.rs"]
mod release_tests;

#[cfg(test)]
#[path = "cuda_pool_probe_read_tests.rs"]
mod probe_read_tests;

#[cfg(test)]
#[path = "cuda_pool_mode_tests.rs"]
mod mode_tests;
