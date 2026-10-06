//! Experiment-only CUDA memory-pool policy for the 0.8.28 Tegra pool study
//! (`dev/plans/0.8.28/prework/tegra-cuda-memory-pool-experiment-protocol.md`,
//! section 2.3).
//!
//! Compiled into a product build only with the cargo feature
//! `tegra-pool-experiment` on aarch64 Linux with `embed-cuda` or
//! `rerank-cuda`; no release feature set enables it. The pure parts (variant
//! parsing, the per-variant plan, the event line) also compile for unit tests
//! on every host.
//!
//! The variant comes from `FATHOMDB_POOL_VARIANT` (`S`, `A-load-hold`,
//! `A-load-release`, `A-first-use`, `B`; unset is `S`), the pool size from
//! `FATHOMDB_POOL_MAXSIZE` (bytes, or `<n>G` / `<n>M`; default `3G`) and the
//! release threshold from `FATHOMDB_POOL_RELEASE_THRESHOLD` (`0` or `max`;
//! default `0`). A malformed value aborts the process, so a typo is never
//! measured as a variant. Every decision event is one stderr line starting
//! with `fdb-pool-exp`.
//!
//! The pool is installed before the first cudarc `CudaContext` of the process
//! exists, because cudarc decides each device's allocator once per process at
//! the first context. The policy therefore only uses result-level driver calls
//! and cudarc's `CudaMemPool` primitive, never a safe context. With `S` it
//! makes no driver call at all.

use std::fmt::Write as _;

/// The study's variants (protocol section 2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PoolVariant {
    S,
    ALoadHold,
    ALoadRelease,
    AFirstUse,
    B,
}

impl PoolVariant {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::S => "S",
            Self::ALoadHold => "A-load-hold",
            Self::ALoadRelease => "A-load-release",
            Self::AFirstUse => "A-first-use",
            Self::B => "B",
        }
    }
}

pub(crate) fn parse_variant(raw: Option<&str>) -> Result<PoolVariant, String> {
    match raw {
        None | Some("S") => Ok(PoolVariant::S),
        Some("A-load-hold") => Ok(PoolVariant::ALoadHold),
        Some("A-load-release") => Ok(PoolVariant::ALoadRelease),
        Some("A-first-use") => Ok(PoolVariant::AFirstUse),
        Some("B") => Ok(PoolVariant::B),
        Some(other) => Err(format!("FATHOMDB_POOL_VARIANT: unknown value {other:?}")),
    }
}

pub(crate) const DEFAULT_MAX_SIZE: usize = 3 << 30;

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
        None | Some("0") => Ok(0),
        Some("max") => Ok(u64::MAX),
        Some(other) => {
            Err(format!("FATHOMDB_POOL_RELEASE_THRESHOLD: must be 0 or max, got {other:?}"))
        }
    }
}

/// What the policy does at one of its two hooks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PoolAction {
    Nothing,
    /// Create and install the explicit pool; release the primary context the
    /// policy retained for it afterwards when `release_context`.
    Create {
        release_context: bool,
    },
    /// Query the default pool; create and install the explicit pool only if
    /// that query fails with `CUDA_ERROR_OUT_OF_MEMORY` (variant B).
    CreateIfDefaultOutOfMemory,
}

/// The hook that runs: at addon registration (after a successful `cuInit`) or
/// immediately before the first Candle CUDA device of the process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hook {
    Load,
    FirstUse,
}

pub(crate) const fn plan(variant: PoolVariant, hook: Hook) -> PoolAction {
    match (variant, hook) {
        (PoolVariant::ALoadHold, Hook::Load) => PoolAction::Create { release_context: false },
        (PoolVariant::ALoadRelease, Hook::Load) | (PoolVariant::AFirstUse, Hook::FirstUse) => {
            PoolAction::Create { release_context: true }
        }
        (PoolVariant::B, Hook::FirstUse) => PoolAction::CreateIfDefaultOutOfMemory,
        _ => PoolAction::Nothing,
    }
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
pub(crate) use driver::{install_pool_at_load, new_cuda_device};

#[cfg(all(
    feature = "tegra-pool-experiment",
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
mod driver {
    use super::{
        format_event, parse_max_size, parse_threshold, parse_variant, plan, Hook, PoolAction,
        PoolEvent, PoolVariant,
    };
    use candle_core::cuda::cudarc::driver::{
        result, sys, AllocMode, CudaMemPool, DriverError, MemPoolProps,
    };
    use candle_core::Device;
    use std::sync::{Mutex, Once, OnceLock, PoisonError};
    use std::time::Instant;

    struct Config {
        variant: PoolVariant,
        max_size: usize,
        threshold: u64,
        stats_every_s: Option<u64>,
        maps_dir: Option<String>,
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
                Ok::<_, String>(Config {
                    variant: parse_variant(env("FATHOMDB_POOL_VARIANT").as_deref())?,
                    max_size: parse_max_size(env("FATHOMDB_POOL_MAXSIZE").as_deref())?,
                    threshold: parse_threshold(env("FATHOMDB_POOL_RELEASE_THRESHOLD").as_deref())?,
                    stats_every_s,
                    maps_dir: env("FATHOMDB_POOL_MAPS_DIR"),
                })
            })();
            parsed.unwrap_or_else(|message| {
                eprintln!("fdb-pool-exp fatal {message}");
                std::process::abort()
            })
        })
    }

    /// The explicit pool, once created and installed; never dropped.
    static POOL: OnceLock<CudaMemPool> = OnceLock::new();
    /// The first `alloc_mode` read after a Candle CUDA device was built.
    static DECIDED: Mutex<Option<AllocMode>> = Mutex::new(None);
    static FIRST_USE: Once = Once::new();
    static LOAD: Once = Once::new();
    static EXIT_HOOK: Once = Once::new();

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

    /// Fills the pool counters from the explicit pool only: querying the
    /// device's current pool could lazily create the default pool, which S
    /// must never do.
    fn with_pool_attrs(mut e: PoolEvent) -> PoolEvent {
        if let Some(pool) = POOL.get() {
            use sys::CUmemPool_attribute::*;
            e.reserved_cur = pool.attribute(CU_MEMPOOL_ATTR_RESERVED_MEM_CURRENT).ok();
            e.reserved_high = pool.attribute(CU_MEMPOOL_ATTR_RESERVED_MEM_HIGH).ok();
            e.used_cur = pool.attribute(CU_MEMPOOL_ATTR_USED_MEM_CURRENT).ok();
            e.used_high = pool.attribute(CU_MEMPOOL_ATTR_USED_MEM_HIGH).ok();
        }
        e
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

    /// Creates, installs and probes the explicit pool on device `ordinal`
    /// with result-level calls, per `action`. Emits one `install` line.
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
            // below or deliberately kept (A-load-hold).
            let ctx = unsafe { result::primary_ctx::retain(device) }.map_err(|err| {
                e.extra = Some(format!("retain={:?}", err.0));
            })?;
            // SAFETY: ctx was just retained.
            let _ = unsafe { result::ctx::set_current(ctx) };
            let release_context = match action {
                PoolAction::Create { release_context } => release_context,
                _ => true,
            };
            let mut create = true;
            if action == PoolAction::CreateIfDefaultOutOfMemory {
                // SAFETY: device is valid and its primary context is current.
                let default = unsafe { result::device::get_default_mem_pool(device) }.map(|_| ());
                create =
                    matches!(default, Err(DriverError(sys::CUresult::CUDA_ERROR_OUT_OF_MEMORY)));
                e.default_pool = Some(rc(&default));
            }
            if create {
                save_maps("pre-pool");
                let props = MemPoolProps {
                    max_size: config().max_size,
                    release_threshold: config().threshold,
                };
                match CudaMemPool::create(ordinal, &props) {
                    Err(err) => e.create = Some(rc(&Err(err))),
                    Ok(pool) => {
                        e.create = Some(rc(&Ok(())));
                        let set = pool.install();
                        e.set = Some(rc(&set));
                        if set.is_ok() {
                            // SAFETY: the primary context is current; the
                            // probe pointer is freed on the same stream.
                            let probe = unsafe {
                                result::malloc_async(std::ptr::null_mut(), 4)
                                    .and_then(|ptr| result::free_async(ptr, std::ptr::null_mut()))
                            }
                            .and_then(|()| result::ctx::synchronize());
                            e.probe = Some(rc(&probe));
                            if probe.is_ok() {
                                let _ = POOL.set(pool);
                            }
                            // Otherwise the pool drops here: destroyed, and
                            // the device reverts to its default pool.
                        }
                    }
                }
                save_maps("post-pool");
            }
            if release_context {
                // SAFETY: releases the reference retained above.
                e.release = Some(rc(&unsafe { result::primary_ctx::release(device) }));
            }
            Ok(())
        })();
        let _ = outcome;
        e.elapsed_us = started.elapsed().as_micros();
        emit(&with_pool_attrs(e));
    }

    /// Called from the Node registration hook after a successful `cuInit`.
    /// Acts only for the A-load variants.
    pub(crate) fn install_pool_at_load() {
        LOAD.call_once(|| run_action(plan(config().variant, Hook::Load), "registration", 0));
    }

    /// Builds a Candle CUDA device, running the first-use hook before the
    /// first one of the process and recording the allocator cudarc decided.
    /// Emits one `decide` line the first time, and another only if a later
    /// device reports a different decision (a C1 failure).
    pub(crate) fn new_cuda_device(
        site: &'static str,
        ordinal: usize,
    ) -> candle_core::Result<Device> {
        FIRST_USE.call_once(|| run_action(plan(config().variant, Hook::FirstUse), site, ordinal));
        let device = Device::new_cuda(ordinal)?;
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
        for v in [
            PoolVariant::S,
            PoolVariant::ALoadHold,
            PoolVariant::ALoadRelease,
            PoolVariant::AFirstUse,
            PoolVariant::B,
        ] {
            assert_eq!(parse_variant(Some(v.name())), Ok(v));
        }
        for bad in ["", "s", "A", "a-first-use", "B ", "A-load", "pool"] {
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
    fn each_variant_acts_at_exactly_its_own_hook() {
        use PoolAction::{Create, CreateIfDefaultOutOfMemory, Nothing};
        let cases = [
            (PoolVariant::S, Nothing, Nothing),
            (PoolVariant::ALoadHold, Create { release_context: false }, Nothing),
            (PoolVariant::ALoadRelease, Create { release_context: true }, Nothing),
            (PoolVariant::AFirstUse, Nothing, Create { release_context: true }),
            (PoolVariant::B, Nothing, CreateIfDefaultOutOfMemory),
        ];
        for (variant, at_load, at_first_use) in cases {
            assert_eq!(plan(variant, Hook::Load), at_load, "{variant:?} at load");
            assert_eq!(plan(variant, Hook::FirstUse), at_first_use, "{variant:?} at first use");
        }
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
