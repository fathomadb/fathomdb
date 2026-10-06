//! Experiment-only CUDA memory-pool policy for the 0.8.28 Tegra pool study
//! (`dev/plans/0.8.28/prework/tegra-cuda-memory-pool-experiment-protocol.md`,
//! section 2.3).
//!
//! Compiled into a product build only with the cargo feature
//! `tegra-pool-experiment` on aarch64 Linux with `embed-cuda` or
//! `rerank-cuda`; no release feature set enables it. The pure parts (variant
//! parsing, the per-variant plan, the fail-closed rule, the event line) also
//! compile for unit tests on every host.
//!
//! The variant comes from `FATHOMDB_POOL_VARIANT` (`S`, `P-first-use`,
//! `A-first-use`, `B`; unset is `S`), the pool size from
//! `FATHOMDB_POOL_MAXSIZE` (bytes, or `<n>G` / `<n>M`; default `3G`) and the
//! release threshold from `FATHOMDB_POOL_RELEASE_THRESHOLD` (`0` or `max`;
//! default `0`). A malformed value aborts the process, so a typo is never
//! measured as a variant. Every decision event is one stderr line starting
//! with `fdb-pool-exp`.
//!
//! `P-first-use` creates a private pool before the first device and builds
//! every device on a cudarc context that allocates from it; the device's
//! current pool is never changed. `A-first-use` and `B` (comparison arms)
//! install the pool as the device's current pool before the first cudarc
//! `CudaContext` exists, because cudarc decides each device's allocator once
//! per process at the first context. With `S` the policy makes no driver call.

use std::fmt::Write as _;

/// The study's variants (protocol section 2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PoolVariant {
    S,
    PFirstUse,
    AFirstUse,
    B,
}

impl PoolVariant {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::S => "S",
            Self::PFirstUse => "P-first-use",
            Self::AFirstUse => "A-first-use",
            Self::B => "B",
        }
    }
}

pub(crate) fn parse_variant(raw: Option<&str>) -> Result<PoolVariant, String> {
    match raw {
        None | Some("S") => Ok(PoolVariant::S),
        Some("P-first-use") => Ok(PoolVariant::PFirstUse),
        Some("A-first-use") => Ok(PoolVariant::AFirstUse),
        Some("B") => Ok(PoolVariant::B),
        Some(other) => Err(format!("FATHOMDB_POOL_VARIANT: unknown value {other:?}")),
    }
}

/// Default pool `maxSize` (owner ruling 9): 3 GiB. On the measured AGX Orin
/// a pool can hold ceil32(`maxSize`/3) = 1 GiB, 3.8 times the workload's
/// 272 MiB high-water mark (study results § 3.1, § 3.5). On a discrete GPU
/// the same value bounds device memory instead of host memory. Override:
/// `FATHOMDB_POOL_MAXSIZE`.
pub(crate) const DEFAULT_MAX_SIZE: usize = 3 << 30;

/// Default release threshold: 0, so freed memory returns to the system at
/// each synchronization (NVIDIA's advice for unknown co-resident processes;
/// study results § 3.1). Override: `FATHOMDB_POOL_RELEASE_THRESHOLD`.
pub(crate) const DEFAULT_RELEASE_THRESHOLD: u64 = 0;

/// Idle time before the experimental trim arm (owner ruling 14) returns the
/// pool's spare memory: long enough that a burst of embeds is not trimmed
/// between calls, short enough that an idle process gives memory back within
/// seconds. Unmeasured choice; override: `FATHOMDB_POOL_TRIM_IDLE_MS`.
pub(crate) const DEFAULT_TRIM_IDLE_MS: u64 = 5_000;

/// Bytes of the one allocation that probes a new pool before it is used.
pub(crate) const PROBE_BYTES: usize = 4;

pub(crate) fn parse_max_size(raw: Option<&str>) -> Result<usize, String> {
    let Some(raw) = raw else { return Ok(DEFAULT_MAX_SIZE) };
    let bad = || format!("FATHOMDB_POOL_MAXSIZE: not a byte count, <n>G or <n>M: {raw:?}");
    let (digits, shift) = match raw.as_bytes().last() {
        Some(b'G') => (&raw[..raw.len() - 1], 30),
        Some(b'M') => (&raw[..raw.len() - 1], 20),
        _ => (raw, 0),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    let value: usize = digits.parse().map_err(|_| bad())?;
    let bytes = value.checked_shl(shift).filter(|b| b >> shift == value).ok_or_else(bad)?;
    if bytes == 0 {
        return Err(bad());
    }
    Ok(bytes)
}

pub(crate) fn parse_threshold(raw: Option<&str>) -> Result<u64, String> {
    match raw {
        None => Ok(DEFAULT_RELEASE_THRESHOLD),
        Some("0") => Ok(0),
        Some("max") => Ok(u64::MAX),
        Some(other) => {
            Err(format!("FATHOMDB_POOL_RELEASE_THRESHOLD: must be 0 or max, got {other:?}"))
        }
    }
}

/// The experimental trim arm (owner ruling 14; never the default).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrimArm {
    Off,
    /// Trim the private pool to zero spare bytes once its in-use amount has
    /// not changed for `idle_ms` (0: at every tick, a stress setting).
    Idle {
        idle_ms: u64,
    },
}

/// `FATHOMDB_POOL_TRIM` (`off` | `idle`) and `FATHOMDB_POOL_TRIM_IDLE_MS`.
pub(crate) fn parse_trim(arm: Option<&str>, idle_ms: Option<&str>) -> Result<TrimArm, String> {
    let idle = match idle_ms {
        None => None,
        Some(raw) if !raw.is_empty() && raw.bytes().all(|b| b.is_ascii_digit()) => Some(
            raw.parse::<u64>()
                .map_err(|_| format!("FATHOMDB_POOL_TRIM_IDLE_MS: out of range: {raw:?}"))?,
        ),
        Some(raw) => {
            return Err(format!("FATHOMDB_POOL_TRIM_IDLE_MS: not a millisecond count: {raw:?}"))
        }
    };
    match (arm, idle) {
        (None | Some("off"), None) => Ok(TrimArm::Off),
        (None | Some("off"), Some(_)) => {
            Err("FATHOMDB_POOL_TRIM_IDLE_MS is set but FATHOMDB_POOL_TRIM is not idle".to_owned())
        }
        (Some("idle"), idle) => Ok(TrimArm::Idle { idle_ms: idle.unwrap_or(DEFAULT_TRIM_IDLE_MS) }),
        (Some(other), _) => Err(format!("FATHOMDB_POOL_TRIM: must be off or idle, got {other:?}")),
    }
}

/// Pool samples per idle period taken by the trim thread, so a trim lands
/// within a quarter of the idle time after it is due.
pub(crate) const TRIM_TICKS_PER_IDLE: u64 = 4;
/// Shortest trim-thread sleep: idle 0 (the stress setting) trims at every
/// tick without busy-spinning.
pub(crate) const TRIM_TICK_MIN_MS: u64 = 1;
/// Longest trim-thread sleep, so a long idle time is overshot by at most this.
pub(crate) const TRIM_TICK_MAX_MS: u64 = 250;

/// How long the trim thread sleeps between pool samples.
pub(crate) const fn trim_tick_ms(idle_ms: u64) -> u64 {
    let tick = idle_ms / TRIM_TICKS_PER_IDLE;
    if tick < TRIM_TICK_MIN_MS {
        TRIM_TICK_MIN_MS
    } else if tick > TRIM_TICK_MAX_MS {
        TRIM_TICK_MAX_MS
    } else {
        tick
    }
}

/// Whether the idle arm trims now: the in-use amount has been unchanged for
/// at least `idle_ms` and the pool reserves more than it uses.
pub(crate) const fn should_trim(idle_ms: u64, unchanged_ms: u64, reserved: u64, used: u64) -> bool {
    unchanged_ms >= idle_ms && reserved > used
}

/// What the policy does immediately before the first Candle CUDA device of
/// the process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PoolAction {
    Nothing,
    /// Create and probe a private pool; never install it (P-first-use).
    CreatePrivate,
    /// Create, install and probe the explicit pool (A-first-use).
    CreateInstalled,
    /// Query the default pool; create and install the explicit pool only if
    /// that query fails with `CUDA_ERROR_OUT_OF_MEMORY` (B).
    CreateInstalledIfDefaultOutOfMemory,
}

pub(crate) const fn plan(variant: PoolVariant) -> PoolAction {
    match variant {
        PoolVariant::S => PoolAction::Nothing,
        PoolVariant::PFirstUse => PoolAction::CreatePrivate,
        PoolVariant::AFirstUse => PoolAction::CreateInstalled,
        PoolVariant::B => PoolAction::CreateInstalledIfDefaultOutOfMemory,
    }
}

/// Which kind of device the process has built so far. The first device
/// fixes it; a process never mixes private-pool and production devices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProcessMode {
    Undecided,
    Production,
    Private,
}

/// What the next device is built on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Build {
    /// `Device::new_cuda` (the production allocator rule).
    Production,
    /// A cudarc context on the private pool.
    Private,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OnPrivateFailure {
    FallBackToProduction,
    Refuse,
}

pub(crate) const fn next_build(mode: ProcessMode, private_pool_ready: bool) -> Build {
    match mode {
        ProcessMode::Private => Build::Private,
        ProcessMode::Production => Build::Production,
        ProcessMode::Undecided if private_pool_ready => Build::Private,
        ProcessMode::Undecided => Build::Production,
    }
}

/// Before any private-pool device exists the process may still become a
/// production process; after one exists a failure is refused.
pub(crate) const fn on_private_failure(mode: ProcessMode) -> OnPrivateFailure {
    match mode {
        ProcessMode::Undecided => OnPrivateFailure::FallBackToProduction,
        ProcessMode::Production | ProcessMode::Private => OnPrivateFailure::Refuse,
    }
}

pub(crate) const fn mode_after(mode: ProcessMode, built: Build) -> ProcessMode {
    match (mode, built) {
        (ProcessMode::Undecided, Build::Private) => ProcessMode::Private,
        (ProcessMode::Undecided, Build::Production) => ProcessMode::Production,
        (fixed, _) => fixed,
    }
}

/// Error kind of a private pool that reached its cap (owner ruling 15),
/// beside `cuda_probe_failed`, `cuda_incompatible` and `cuda_not_compiled`.
pub(crate) const POOL_EXHAUSTED_KIND: &str = "cuda_pool_exhausted";

/// Whether a forward error is pool exhaustion: the driver reported
/// `CUDA_ERROR_OUT_OF_MEMORY` in a process whose devices allocate from the
/// private pool (a production-mode process keeps today's report).
pub(crate) const fn is_pool_exhaustion(mode: ProcessMode, driver_out_of_memory: bool) -> bool {
    matches!(mode, ProcessMode::Private) && driver_out_of_memory
}

/// One `fdb-pool-exp` stderr line. Fields not applicable to an event are `-`.
#[derive(Clone, Debug, Default)]
pub(crate) struct PoolEvent {
    pub(crate) event: &'static str,
    pub(crate) pid: u32,
    pub(crate) variant: &'static str,
    pub(crate) site: &'static str,
    pub(crate) default_pool: Option<String>,
    pub(crate) create: Option<String>,
    pub(crate) set: Option<String>,
    pub(crate) probe: Option<String>,
    pub(crate) release: Option<String>,
    pub(crate) alloc_mode: Option<&'static str>,
    pub(crate) max_size: usize,
    pub(crate) threshold: u64,
    pub(crate) reserved_cur: Option<u64>,
    pub(crate) reserved_high: Option<u64>,
    pub(crate) used_cur: Option<u64>,
    pub(crate) used_high: Option<u64>,
    pub(crate) elapsed_us: u128,
    pub(crate) extra: Option<String>,
}

pub(crate) fn format_event(e: &PoolEvent) -> String {
    fn opt<T: std::fmt::Display>(v: &Option<T>) -> String {
        v.as_ref().map_or_else(|| "-".to_owned(), ToString::to_string)
    }
    let mut line = format!(
        "fdb-pool-exp event={} pid={} variant={} site={} default_pool={} create={} set={} \
         probe={} release={} alloc_mode={} max_size={} threshold={} reserved_cur={} \
         reserved_high={} used_cur={} used_high={} elapsed_us={}",
        e.event,
        e.pid,
        e.variant,
        e.site,
        opt(&e.default_pool),
        opt(&e.create),
        opt(&e.set),
        opt(&e.probe),
        opt(&e.release),
        e.alloc_mode.unwrap_or("-"),
        e.max_size,
        e.threshold,
        opt(&e.reserved_cur),
        opt(&e.reserved_high),
        opt(&e.used_cur),
        opt(&e.used_high),
        e.elapsed_us,
    );
    if let Some(extra) = &e.extra {
        let _ = write!(line, " {extra}");
    }
    line
}

#[cfg(all(
    feature = "tegra-pool-experiment",
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
pub(crate) use driver::{forward_error, new_cuda_device};

#[cfg(all(
    feature = "tegra-pool-experiment",
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
mod driver {
    use super::{
        format_event, mode_after, next_build, on_private_failure, parse_max_size, parse_threshold,
        parse_trim, parse_variant, plan, should_trim, Build, OnPrivateFailure, PoolAction,
        PoolEvent, PoolVariant, ProcessMode, TrimArm, PROBE_BYTES,
    };
    use super::{is_pool_exhaustion, trim_tick_ms, POOL_EXHAUSTED_KIND};
    use candle_core::cuda::cudarc::driver::{
        result, sys, AllocMode, CudaContext, CudaMemPool, DriverError, MemPoolProps,
    };
    use candle_core::Device;
    use fathomdb_embedder_api::EmbedderError;
    use std::sync::{Arc, Mutex, Once, OnceLock, PoisonError};
    use std::time::Instant;

    struct Config {
        variant: PoolVariant,
        max_size: usize,
        threshold: u64,
        stats_every_s: Option<u64>,
        maps_dir: Option<String>,
        coexist_check: bool,
        trim: TrimArm,
    }

    fn config() -> &'static Config {
        static CONFIG: OnceLock<Config> = OnceLock::new();
        CONFIG.get_or_init(|| {
            let env = |name| std::env::var(name).ok();
            let parsed = (|| {
                let stats_every_s = match env("FATHOMDB_POOL_STATS_EVERY_S") {
                    None => None,
                    Some(raw) => {
                        Some(raw.parse::<u64>().ok().filter(|s| *s > 0).ok_or_else(|| {
                            format!("FATHOMDB_POOL_STATS_EVERY_S: not a positive integer: {raw:?}")
                        })?)
                    }
                };
                let coexist_check = match env("FATHOMDB_POOL_COEXIST_CHECK").as_deref() {
                    None | Some("0") => false,
                    Some("1") => true,
                    Some(other) => {
                        return Err(format!(
                            "FATHOMDB_POOL_COEXIST_CHECK: must be 0 or 1, got {other:?}"
                        ))
                    }
                };
                Ok::<_, String>(Config {
                    variant: parse_variant(env("FATHOMDB_POOL_VARIANT").as_deref())?,
                    max_size: parse_max_size(env("FATHOMDB_POOL_MAXSIZE").as_deref())?,
                    threshold: parse_threshold(env("FATHOMDB_POOL_RELEASE_THRESHOLD").as_deref())?,
                    stats_every_s,
                    maps_dir: env("FATHOMDB_POOL_MAPS_DIR"),
                    coexist_check,
                    trim: parse_trim(
                        env("FATHOMDB_POOL_TRIM").as_deref(),
                        env("FATHOMDB_POOL_TRIM_IDLE_MS").as_deref(),
                    )?,
                })
            })();
            parsed.unwrap_or_else(|message| {
                eprintln!("fdb-pool-exp fatal {message}");
                std::process::abort()
            })
        })
    }

    /// The installed explicit pool of A-first-use and B; never dropped.
    static POOL: OnceLock<CudaMemPool> = OnceLock::new();
    /// The private pool of P-first-use, with the ordinal it was created for.
    static PRIVATE: OnceLock<(usize, Arc<CudaMemPool>)> = OnceLock::new();
    static MODE: Mutex<ProcessMode> = Mutex::new(ProcessMode::Undecided);
    /// The first `alloc_mode` read after a Candle CUDA device was built.
    static DECIDED: Mutex<Option<AllocMode>> = Mutex::new(None);
    static FIRST_USE: Once = Once::new();
    static EXIT_HOOK: Once = Once::new();
    static TRIM_THREAD: Once = Once::new();
    /// Trims done by the idle arm, and their total time in microseconds.
    static TRIMS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    static TRIM_US: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    /// The idle trim arm (ruling 14): a thread that watches the private
    /// pool's in-use amount and trims its spare memory once that amount has
    /// been unchanged for the idle time. `cuMemPoolTrimTo` never releases
    /// memory backing outstanding allocations, so live slices stay valid;
    /// whether that holds under concurrent work is what the arm's tests
    /// measure.
    fn start_trim_thread(ordinal: usize) {
        let TrimArm::Idle { idle_ms } = config().trim else { return };
        TRIM_THREAD.call_once(|| {
            std::thread::spawn(move || {
                use std::sync::atomic::Ordering::Relaxed;
                use sys::CUmemPool_attribute::*;
                let Some((_, pool)) = PRIVATE.get() else { return };
                let Ok(device) = result::device::get(ordinal as i32) else { return };
                // SAFETY: device comes from cuDeviceGet; the reference is kept
                // for the life of the process, like the pool.
                let Ok(ctx) = (unsafe { result::primary_ctx::retain(device) }) else { return };
                // SAFETY: ctx was just retained.
                let _ = unsafe { result::ctx::set_current(ctx) };
                let tick = std::time::Duration::from_millis(trim_tick_ms(idle_ms));
                let mut last_used = u64::MAX;
                let mut since = Instant::now();
                loop {
                    std::thread::sleep(tick);
                    let (Ok(used), Ok(reserved)) = (
                        pool.attribute(CU_MEMPOOL_ATTR_USED_MEM_CURRENT),
                        pool.attribute(CU_MEMPOOL_ATTR_RESERVED_MEM_CURRENT),
                    ) else {
                        continue;
                    };
                    if used != last_used {
                        last_used = used;
                        since = Instant::now();
                    }
                    let unchanged_ms = since.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
                    if should_trim(idle_ms, unchanged_ms, reserved, used) {
                        let started = Instant::now();
                        if pool.trim_to(0).is_ok() {
                            TRIMS.fetch_add(1, Relaxed);
                            let us = started.elapsed().as_micros().try_into().unwrap_or(u64::MAX);
                            TRIM_US.fetch_add(us, Relaxed);
                        }
                    }
                }
            });
        });
    }

    fn rc(r: &Result<(), DriverError>) -> String {
        match r {
            Ok(()) => "CUDA_SUCCESS".to_owned(),
            Err(DriverError(code)) => format!("{code:?}"),
        }
    }

    fn mode_name(mode: AllocMode) -> &'static str {
        match mode {
            AllocMode::Default => "default",
            AllocMode::Explicit => "explicit",
            AllocMode::Private => "private",
            AllocMode::Sync => "sync",
        }
    }

    fn base(event: &'static str, site: &'static str) -> PoolEvent {
        let c = config();
        PoolEvent {
            event,
            pid: std::process::id(),
            variant: c.variant.name(),
            site,
            max_size: c.max_size,
            threshold: c.threshold,
            ..PoolEvent::default()
        }
    }

    /// Fills the pool counters from the study's own pool only: querying the
    /// device's current pool could lazily create the default pool, which S
    /// must never do.
    fn with_pool_attrs(mut e: PoolEvent) -> PoolEvent {
        let pool = POOL.get().or_else(|| PRIVATE.get().map(|(_, pool)| pool.as_ref()));
        if let Some(pool) = pool {
            use sys::CUmemPool_attribute::*;
            e.reserved_cur = pool.attribute(CU_MEMPOOL_ATTR_RESERVED_MEM_CURRENT).ok();
            e.reserved_high = pool.attribute(CU_MEMPOOL_ATTR_RESERVED_MEM_HIGH).ok();
            e.used_cur = pool.attribute(CU_MEMPOOL_ATTR_USED_MEM_CURRENT).ok();
            e.used_high = pool.attribute(CU_MEMPOOL_ATTR_USED_MEM_HIGH).ok();
        }
        e
    }

    /// C9 observables (only with `FATHOMDB_POOL_COEXIST_CHECK=1`: reading the
    /// current pool runs the driver's lazy default-pool creation): the
    /// (`CUresult`, handle) pairs of the device's current and default pools.
    fn coexist_fields(ordinal: usize) -> Option<String> {
        if !config().coexist_check {
            return None;
        }
        let pair = |r: Result<sys::CUmemoryPool, DriverError>| match r {
            Ok(pool) => format!("CUDA_SUCCESS:{:#x}", pool as usize),
            Err(DriverError(code)) => format!("{code:?}:0x0"),
        };
        let device = result::device::get(ordinal as i32).ok()?;
        // SAFETY: device comes from cuDeviceGet.
        let current = pair(unsafe { result::device::get_mem_pool(device) });
        // SAFETY: as above.
        let default = pair(unsafe { result::device::get_default_mem_pool(device) });
        let private = PRIVATE.get().map_or(0, |(_, pool)| pool.raw() as usize);
        Some(format!(
            "current_pool={current} default_pool_pair={default} private_pool={private:#x}"
        ))
    }

    fn join_extra(a: Option<String>, b: Option<String>) -> Option<String> {
        match (a, b) {
            (Some(a), Some(b)) => Some(format!("{a} {b}")),
            (a, b) => a.or(b),
        }
    }

    fn emit(e: &PoolEvent) {
        eprintln!("{}", format_event(e));
    }

    fn save_maps(stage: &str) {
        if let Some(dir) = &config().maps_dir {
            if let Ok(maps) = std::fs::read("/proc/self/maps") {
                let _ =
                    std::fs::write(format!("{dir}/maps-{}-{stage}.txt", std::process::id()), maps);
            }
        }
    }

    extern "C" fn on_exit() {
        let mut e = with_pool_attrs(base("teardown", "exit"));
        e.alloc_mode = DECIDED.lock().unwrap_or_else(PoisonError::into_inner).map(mode_name);
        if PRIVATE.get().is_some() {
            use std::sync::atomic::Ordering::Relaxed;
            let trim = match config().trim {
                TrimArm::Off => "trim=off".to_owned(),
                TrimArm::Idle { idle_ms } => format!(
                    "trim=idle trim_idle_ms={idle_ms} trims={} trim_us={}",
                    TRIMS.load(Relaxed),
                    TRIM_US.load(Relaxed)
                ),
            };
            let ordinal = PRIVATE.get().map_or(0, |(ordinal, _)| *ordinal);
            e.extra = join_extra(Some(format!("installed=0 {trim}")), coexist_fields(ordinal));
        } else {
            // C9b: what a co-resident library would get from the default
            // pool at the end of a process of any other variant. These
            // variants keep no ordinal; the study host has one device.
            e.extra = coexist_fields(0);
        }
        emit(&e);
    }

    extern "C" {
        fn atexit(callback: extern "C" fn()) -> std::os::raw::c_int;
    }

    fn arm_exit_and_stats() {
        EXIT_HOOK.call_once(|| {
            // SAFETY: registers a plain extern "C" function with libc.
            unsafe { atexit(on_exit) };
            if let Some(every) = config().stats_every_s {
                std::thread::spawn(move || loop {
                    std::thread::sleep(std::time::Duration::from_secs(every));
                    let mut e = with_pool_attrs(base("stats", "timer"));
                    e.alloc_mode =
                        DECIDED.lock().unwrap_or_else(PoisonError::into_inner).map(mode_name);
                    emit(&e);
                });
            }
        });
    }

    fn props() -> MemPoolProps {
        MemPoolProps { max_size: config().max_size, release_threshold: config().threshold }
    }

    /// Creates the study's pool on device `ordinal` per `action`, with a
    /// retained primary context current (kept for the probe; whether
    /// `cuMemPoolCreate` needs it is the protocol's M5 hypothesis), and
    /// releases that context afterwards. Emits one `install` line.
    fn run_action(action: PoolAction, site: &'static str, ordinal: usize) {
        if action == PoolAction::Nothing {
            return;
        }
        arm_exit_and_stats();
        let started = Instant::now();
        let mut e = base("install", site);
        let outcome = (|| -> Result<(), ()> {
            let device = result::device::get(ordinal as i32).map_err(|err| {
                e.extra = Some(format!("device_get={:?}", err.0));
            })?;
            // SAFETY: device comes from cuDeviceGet; the context is released
            // below.
            let ctx = unsafe { result::primary_ctx::retain(device) }.map_err(|err| {
                e.extra = Some(format!("retain={:?}", err.0));
            })?;
            // SAFETY: ctx was just retained.
            let _ = unsafe { result::ctx::set_current(ctx) };
            let mut create = true;
            if action == PoolAction::CreateInstalledIfDefaultOutOfMemory {
                // SAFETY: device is valid and its primary context is current.
                let default = unsafe { result::device::get_default_mem_pool(device) }.map(|_| ());
                create =
                    matches!(default, Err(DriverError(sys::CUresult::CUDA_ERROR_OUT_OF_MEMORY)));
                e.default_pool = Some(rc(&default));
            }
            if action == PoolAction::CreatePrivate {
                e.extra = join_extra(Some("installed=0".to_owned()), coexist_fields(ordinal));
            }
            if create {
                save_maps("pre-pool");
                match CudaMemPool::create(ordinal, &props()) {
                    Err(err) => e.create = Some(rc(&Err(err))),
                    Ok(pool) => {
                        e.create = Some(rc(&Ok(())));
                        if action == PoolAction::CreatePrivate {
                            // SAFETY: the primary context is current; the
                            // probe pointer is freed on the same stream.
                            let probe = unsafe {
                                result::mem_pool::alloc_async(
                                    pool.raw(),
                                    PROBE_BYTES,
                                    std::ptr::null_mut(),
                                )
                                .and_then(|ptr| result::free_async(ptr, std::ptr::null_mut()))
                            }
                            .and_then(|()| result::ctx::synchronize());
                            e.probe = Some(rc(&probe));
                            if probe.is_ok() {
                                let _ = PRIVATE.set((ordinal, Arc::new(pool)));
                                start_trim_thread(ordinal);
                            }
                        } else {
                            let set = pool.install();
                            e.set = Some(rc(&set));
                            if set.is_ok() {
                                // SAFETY: as above.
                                let probe = unsafe {
                                    result::malloc_async(std::ptr::null_mut(), PROBE_BYTES)
                                        .and_then(|ptr| {
                                            result::free_async(ptr, std::ptr::null_mut())
                                        })
                                }
                                .and_then(|()| result::ctx::synchronize());
                                e.probe = Some(rc(&probe));
                                if probe.is_ok() {
                                    let _ = POOL.set(pool);
                                }
                                // Otherwise the pool drops here: destroyed,
                                // and the device reverts to its default pool.
                            }
                        }
                    }
                }
                save_maps("post-pool");
            }
            // SAFETY: releases the reference retained above.
            e.release = Some(rc(&unsafe { result::primary_ctx::release(device) }));
            Ok(())
        })();
        let _ = outcome;
        e.elapsed_us = started.elapsed().as_micros();
        emit(&with_pool_attrs(e));
    }

    /// Whether a Candle error carries cudarc's `CUDA_ERROR_OUT_OF_MEMORY`,
    /// looking through Candle's context, path and backtrace wrappers.
    fn driver_out_of_memory(error: &candle_core::Error) -> bool {
        use candle_core::{cuda::CudaError, Error};
        match error {
            Error::Cuda(source) => matches!(
                source.downcast_ref::<CudaError>(),
                Some(CudaError::Cuda(DriverError(sys::CUresult::CUDA_ERROR_OUT_OF_MEMORY)))
            ),
            Error::Context { inner, .. }
            | Error::WithPath { inner, .. }
            | Error::WithBacktrace { inner, .. } => driver_out_of_memory(inner),
            _ => false,
        }
    }

    /// The embedder's error for a failed forward pass (ruling 15): pool
    /// exhaustion in a private-pool process, otherwise `Failed` as before.
    /// Pool exhaustion also emits one `exhausted` event line.
    pub(crate) fn forward_error(error: candle_core::Error, what: &str) -> EmbedderError {
        let message = format!("{what}: {error}");
        let mode = *MODE.lock().unwrap_or_else(PoisonError::into_inner);
        if !is_pool_exhaustion(mode, driver_out_of_memory(&error)) {
            return EmbedderError::Failed { message };
        }
        let ordinal = PRIVATE.get().map_or(0, |(ordinal, _)| *ordinal);
        let mut e = with_pool_attrs(base("exhausted", "forward"));
        e.extra = Some(format!("kind={POOL_EXHAUSTED_KIND}"));
        emit(&e);
        EmbedderError::CudaPoolExhausted {
            ordinal,
            max_size_bytes: config().max_size as u64,
            message,
        }
    }

    /// A device on a fresh cudarc context that allocates from the private
    /// pool, or the driver error.
    fn private_device(ordinal: usize) -> Result<Device, String> {
        let (pool_ordinal, pool) =
            PRIVATE.get().ok_or_else(|| "private pool unavailable".to_owned())?;
        if *pool_ordinal != ordinal {
            return Err(format!("private pool is for device {pool_ordinal}, not {ordinal}"));
        }
        let ctx = CudaContext::new_with_mem_pool(ordinal, pool.clone())
            .map_err(|DriverError(code)| format!("{code:?}"))?;
        Device::new_cuda_from_context(ctx).map_err(|err| err.to_string())
    }

    /// Builds a Candle CUDA device, running the first-use hook before the
    /// first one of the process and recording the allocator cudarc decided.
    /// Emits one `decide` line the first time, and another only if a later
    /// device reports a different decision (a C1 failure).
    ///
    /// # Errors
    /// The Candle error of building the device; with P-first-use, a refusal
    /// when a private-pool device cannot be built after one already exists
    /// (the process never mixes private-pool and production devices).
    pub(crate) fn new_cuda_device(
        site: &'static str,
        ordinal: usize,
    ) -> candle_core::Result<Device> {
        FIRST_USE.call_once(|| run_action(plan(config().variant), site, ordinal));
        let device = {
            let mut mode = MODE.lock().unwrap_or_else(PoisonError::into_inner);
            let build = next_build(*mode, PRIVATE.get().is_some());
            let (device, built) = match build {
                Build::Production => (Device::new_cuda(ordinal)?, Build::Production),
                Build::Private => match private_device(ordinal) {
                    Ok(device) => (device, Build::Private),
                    Err(message) => {
                        let mut e = base("private-failure", site);
                        e.extra = Some(format!("error={}", message.replace(' ', "_")));
                        emit(&e);
                        match on_private_failure(*mode) {
                            OnPrivateFailure::FallBackToProduction => {
                                (Device::new_cuda(ordinal)?, Build::Production)
                            }
                            OnPrivateFailure::Refuse => {
                                return Err(candle_core::Error::Msg(format!(
                                    "fdb-pool-exp: private-pool device refused after a private \
                                     device exists: {message}"
                                )));
                            }
                        }
                    }
                },
            };
            *mode = mode_after(*mode, built);
            device
        };
        if let Ok(cuda) = device.as_cuda_device() {
            let mode = cuda.cuda_stream().context().alloc_mode();
            let mut decided = DECIDED.lock().unwrap_or_else(PoisonError::into_inner);
            let first = decided.is_none();
            if first || *decided != Some(mode) {
                if first {
                    *decided = Some(mode);
                }
                arm_exit_and_stats();
                let mut e = with_pool_attrs(base("decide", site));
                e.alloc_mode = Some(mode_name(mode));
                if !first {
                    e.extra = Some("mismatch=1".to_owned());
                }
                emit(&e);
            }
        }
        Ok(device)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variants_parse_exactly_and_unset_is_s() {
        assert_eq!(parse_variant(None), Ok(PoolVariant::S));
        for v in [PoolVariant::S, PoolVariant::PFirstUse, PoolVariant::AFirstUse, PoolVariant::B] {
            assert_eq!(parse_variant(Some(v.name())), Ok(v));
        }
        for bad in [
            "",
            "s",
            "A",
            "P",
            "a-first-use",
            "p-first-use",
            "B ",
            "A-load",
            "A-load-hold",
            "A-load-release",
            "pool",
        ] {
            assert!(parse_variant(Some(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn max_size_accepts_bytes_and_binary_suffixes_only() {
        assert_eq!(parse_max_size(None), Ok(3 << 30));
        assert_eq!(parse_max_size(Some("1G")), Ok(1 << 30));
        assert_eq!(parse_max_size(Some("16G")), Ok(16 << 30));
        assert_eq!(parse_max_size(Some("192M")), Ok(192 << 20));
        assert_eq!(parse_max_size(Some("4096")), Ok(4096));
        for bad in ["", "G", "0", "0G", "3g", "3GiB", "-1", "1.5G", " 3G", "99999999999999999999G"]
        {
            assert!(parse_max_size(Some(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn threshold_is_zero_or_max() {
        assert_eq!(parse_threshold(None), Ok(0));
        assert_eq!(parse_threshold(Some("0")), Ok(0));
        assert_eq!(parse_threshold(Some("max")), Ok(u64::MAX));
        for bad in ["", "MAX", "1", "4096"] {
            assert!(parse_threshold(Some(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn each_variant_has_one_first_use_action() {
        assert_eq!(plan(PoolVariant::S), PoolAction::Nothing);
        assert_eq!(plan(PoolVariant::PFirstUse), PoolAction::CreatePrivate);
        assert_eq!(plan(PoolVariant::AFirstUse), PoolAction::CreateInstalled);
        assert_eq!(plan(PoolVariant::B), PoolAction::CreateInstalledIfDefaultOutOfMemory);
    }

    /// Fail closed (protocol 2.2): the first device fixes the process's mode,
    /// and once a private-pool device exists a later failure to build one is
    /// refused, never answered with a production device.
    #[test]
    fn the_first_device_fixes_the_mode_and_private_failures_after_it_are_refused() {
        use Build::{Private, Production};
        use ProcessMode::{Private as PrivateMode, Production as ProductionMode, Undecided};

        // Pool ready: the first device is private and fixes the mode.
        assert_eq!(next_build(Undecided, true), Private);
        assert_eq!(mode_after(Undecided, Private), PrivateMode);
        assert_eq!(next_build(PrivateMode, true), Private);
        assert_eq!(on_private_failure(PrivateMode), OnPrivateFailure::Refuse);

        // No pool: every device is a production device, even if a pool
        // appears later.
        assert_eq!(next_build(Undecided, false), Production);
        assert_eq!(mode_after(Undecided, Production), ProductionMode);
        assert_eq!(next_build(ProductionMode, true), Production);

        // The first private build fails before any private device exists:
        // the process falls back and stays production.
        assert_eq!(on_private_failure(Undecided), OnPrivateFailure::FallBackToProduction);
        assert_eq!(mode_after(Undecided, Production), ProductionMode);

        // A fixed mode never changes.
        assert_eq!(mode_after(PrivateMode, Production), PrivateMode);
        assert_eq!(mode_after(ProductionMode, Private), ProductionMode);
    }

    #[test]
    fn product_defaults_are_named_and_documented_values() {
        assert_eq!(DEFAULT_MAX_SIZE, 3 << 30);
        assert_eq!(DEFAULT_RELEASE_THRESHOLD, 0);
        assert_eq!(DEFAULT_TRIM_IDLE_MS, 5_000);
        assert_eq!(PROBE_BYTES, 4);
        assert_eq!(parse_threshold(None), Ok(DEFAULT_RELEASE_THRESHOLD));
    }

    #[test]
    fn trim_is_off_unless_the_idle_arm_is_asked_for() {
        assert_eq!(parse_trim(None, None), Ok(TrimArm::Off));
        assert_eq!(parse_trim(Some("off"), None), Ok(TrimArm::Off));
        assert_eq!(
            parse_trim(Some("idle"), None),
            Ok(TrimArm::Idle { idle_ms: DEFAULT_TRIM_IDLE_MS })
        );
        assert_eq!(parse_trim(Some("idle"), Some("0")), Ok(TrimArm::Idle { idle_ms: 0 }));
        assert_eq!(parse_trim(Some("idle"), Some("250")), Ok(TrimArm::Idle { idle_ms: 250 }));
        for (arm, ms) in [
            (Some(""), None),
            (Some("on"), None),
            (Some("IDLE"), None),
            (Some("idle"), Some("-1")),
            (Some("idle"), Some("1s")),
        ] {
            assert!(parse_trim(arm, ms).is_err(), "{arm:?} {ms:?}");
        }
        assert!(
            parse_trim(Some("off"), Some("250")).is_err(),
            "an idle time without the idle arm is a typo"
        );
    }

    /// The idle arm trims only memory the pool holds beyond what is in use,
    /// and only once the in-use amount has not changed for the idle time.
    #[test]
    fn the_idle_arm_trims_spare_memory_after_the_idle_time_only() {
        let idle = 1_000;
        assert!(!should_trim(idle, 999, 64 << 20, 0), "not idle long enough");
        assert!(should_trim(idle, 1_000, 64 << 20, 0));
        assert!(should_trim(idle, 5_000, 64 << 20, 16 << 20));
        assert!(!should_trim(idle, 5_000, 16 << 20, 16 << 20), "nothing spare to trim");
        assert!(!should_trim(idle, 5_000, 0, 0));
        assert!(should_trim(0, 0, 1, 0), "idle 0 trims at every tick (stress)");
    }

    /// The trim thread samples the pool several times per idle period, so a
    /// trim lands within a fraction of the idle time, but never busy-spins
    /// and never sleeps so long that a long idle time is overshot by much.
    #[test]
    fn the_trim_thread_samples_a_few_times_per_idle_period_within_bounds() {
        assert_eq!(trim_tick_ms(0), TRIM_TICK_MIN_MS, "idle 0 (stress) still sleeps");
        assert_eq!(trim_tick_ms(200), 200 / TRIM_TICKS_PER_IDLE);
        assert_eq!(trim_tick_ms(DEFAULT_TRIM_IDLE_MS), TRIM_TICK_MAX_MS);
        assert_eq!(trim_tick_ms(u64::MAX), TRIM_TICK_MAX_MS);
    }

    /// Ruling 15: an out-of-memory driver error is pool exhaustion only in a
    /// process whose devices allocate from the private pool; elsewhere it is
    /// reported as before.
    #[test]
    fn only_a_private_pool_out_of_memory_is_pool_exhaustion() {
        assert!(is_pool_exhaustion(ProcessMode::Private, true));
        assert!(!is_pool_exhaustion(ProcessMode::Private, false));
        assert!(!is_pool_exhaustion(ProcessMode::Production, true));
        assert!(!is_pool_exhaustion(ProcessMode::Undecided, true));
        assert_eq!(POOL_EXHAUSTED_KIND, "cuda_pool_exhausted");
    }

    #[test]
    fn the_event_line_has_every_field_in_protocol_order() {
        let line = format_event(&PoolEvent {
            event: "install",
            pid: 42,
            variant: "B",
            site: "embedder-probe",
            default_pool: Some("CUDA_ERROR_OUT_OF_MEMORY".to_owned()),
            create: Some("CUDA_SUCCESS".to_owned()),
            set: Some("CUDA_SUCCESS".to_owned()),
            probe: Some("CUDA_SUCCESS".to_owned()),
            release: Some("CUDA_SUCCESS".to_owned()),
            alloc_mode: None,
            max_size: 3 << 30,
            threshold: 0,
            reserved_cur: Some(0),
            reserved_high: Some(32 << 20),
            used_cur: Some(0),
            used_high: Some(4),
            elapsed_us: 1234,
            extra: None,
        });
        assert_eq!(
            line,
            "fdb-pool-exp event=install pid=42 variant=B site=embedder-probe \
             default_pool=CUDA_ERROR_OUT_OF_MEMORY create=CUDA_SUCCESS set=CUDA_SUCCESS \
             probe=CUDA_SUCCESS release=CUDA_SUCCESS alloc_mode=- max_size=3221225472 \
             threshold=0 reserved_cur=0 reserved_high=33554432 used_cur=0 used_high=4 \
             elapsed_us=1234"
        );
        let decide = format_event(&PoolEvent {
            event: "decide",
            alloc_mode: Some("explicit"),
            extra: Some("mismatch=1".to_owned()),
            ..PoolEvent::default()
        });
        assert!(decide.contains(" alloc_mode=explicit "), "{decide}");
        assert!(decide.contains(" default_pool=- "), "{decide}");
        assert!(decide.ends_with(" mismatch=1"), "{decide}");
    }
}
