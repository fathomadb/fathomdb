use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::Mutex;

use super::projection_runtime::PROJECTION_COMMIT_BATCH;
use super::provenance::DEFAULT_PROVENANCE_ROW_CAP;
use super::telemetry::DEFAULT_SLOW_THRESHOLD_MS;
use super::DEFAULT_EMBED_TIMEOUT_MS;

/// SQLite runtime mode selected before FathomDB opens its first connection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeSqliteMode {
    /// Disable SQLite's process-global memory statistics and heap-limit
    /// enforcement to remove their shared allocator lock from read traffic.
    Performance,
    /// Enable SQLite's process-global memory statistics and heap-limit
    /// enforcement for diagnostic applications.
    Diagnostics,
}

/// Effective process-wide SQLite runtime configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeConfiguration {
    pub sqlite_mode: RuntimeSqliteMode,
}

/// Startup configuration failure. Changing modes requires a process restart.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeConfigurationError {
    /// SQLite was already initialized before FathomDB could configure it.
    TooLate,
    /// The runtime is already configured in a different mode.
    Conflict { requested: RuntimeSqliteMode, effective: RuntimeSqliteMode },
    /// SQLite rejected configuration or initialization with this result code.
    SqliteFailure { code: i32 },
}

impl Display for RuntimeConfigurationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLate => write!(f, "SQLite runtime configuration is too late; restart required"),
            Self::Conflict { requested, effective } => write!(
                f,
                "SQLite runtime is already configured as {effective:?}, not {requested:?}; restart required"
            ),
            Self::SqliteFailure { code } => {
                write!(f, "SQLite runtime configuration failed with code {code}")
            }
        }
    }
}

impl Error for RuntimeConfigurationError {}

#[derive(Clone, Copy, Debug)]
enum RuntimeState {
    Unconfigured,
    Configured(RuntimeConfiguration),
    Failed(RuntimeConfigurationError),
}

static SQLITE_RUNTIME_STATE: Mutex<RuntimeState> = Mutex::new(RuntimeState::Unconfigured);

/// Configure the SQLite runtime before opening any FathomDB Engine.
///
/// Repeating the same mode is idempotent. Selecting a different mode or
/// configuring after any SQLite initialization fails without shutdown or
/// reconfiguration. The setting applies to the SQLite image linked into this
/// artifact and persists for the process lifetime.
pub fn configure_runtime(
    sqlite_mode: RuntimeSqliteMode,
) -> Result<RuntimeConfiguration, RuntimeConfigurationError> {
    configure_runtime_locked(Some(sqlite_mode))
}

pub(crate) fn configure_runtime_for_open() -> Result<RuntimeConfiguration, RuntimeConfigurationError>
{
    configure_runtime_locked(None)
}

fn configure_runtime_locked(
    requested: Option<RuntimeSqliteMode>,
) -> Result<RuntimeConfiguration, RuntimeConfigurationError> {
    let mut state = SQLITE_RUNTIME_STATE.lock().map_err(|_| {
        RuntimeConfigurationError::SqliteFailure { code: rusqlite::ffi::SQLITE_ERROR }
    })?;
    match *state {
        RuntimeState::Configured(effective) => {
            if requested.is_none_or(|mode| mode == effective.sqlite_mode) {
                return Ok(effective);
            }
            return Err(RuntimeConfigurationError::Conflict {
                requested: requested.expect("checked Some"),
                effective: effective.sqlite_mode,
            });
        }
        RuntimeState::Failed(error) => return Err(error),
        RuntimeState::Unconfigured => {}
    }

    let mode = requested.unwrap_or(RuntimeSqliteMode::Performance);
    let memstatus = match mode {
        RuntimeSqliteMode::Performance => 0_i32,
        RuntimeSqliteMode::Diagnostics => 1_i32,
    };
    let config_rc =
        unsafe { rusqlite::ffi::sqlite3_config(rusqlite::ffi::SQLITE_CONFIG_MEMSTATUS, memstatus) };
    if config_rc != rusqlite::ffi::SQLITE_OK {
        let error = if config_rc == rusqlite::ffi::SQLITE_MISUSE {
            RuntimeConfigurationError::TooLate
        } else {
            RuntimeConfigurationError::SqliteFailure { code: config_rc }
        };
        *state = RuntimeState::Failed(error);
        return Err(error);
    }
    let initialize_rc = unsafe { rusqlite::ffi::sqlite3_initialize() };
    if initialize_rc != rusqlite::ffi::SQLITE_OK {
        let error = RuntimeConfigurationError::SqliteFailure { code: initialize_rc };
        *state = RuntimeState::Failed(error);
        return Err(error);
    }
    let effective = RuntimeConfiguration { sqlite_mode: mode };
    *state = RuntimeState::Configured(effective);
    Ok(effective)
}

pub(crate) fn effective_runtime_configuration() -> RuntimeConfiguration {
    match *SQLITE_RUNTIME_STATE.lock().expect("SQLite runtime state") {
        RuntimeState::Configured(configuration) => configuration,
        RuntimeState::Unconfigured | RuntimeState::Failed(_) => {
            unreachable!("an opened Engine always has a configured SQLite runtime")
        }
    }
}

const MAX_SAFE_INTEGER: u64 = (1_u64 << 53) - 1;

/// Requested per-engine settings for an open. `None` selects the documented default.
///
/// All values are validated before the open touches the filesystem or an embedder.
/// This requested value is retained unchanged even if an effective setting is
/// subsequently changed by an engine setter.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EngineConfig {
    /// Projection orchestration threads, `1..=64`; default `2`.
    pub scheduler_runtime_threads: Option<u64>,
    /// Provider dispatch workers, `1..=64`; default `5`.
    pub embedder_pool_size: Option<u64>,
    /// Absolute provider-call deadline in milliseconds, `1..=u32::MAX`; default `30_000`.
    pub embedder_call_timeout_ms: Option<u64>,
    /// Retained provenance rows, `0..=2^53-1`; default `1_000_000`. Zero disables retention.
    pub provenance_row_cap: Option<u64>,
    /// Slow-operation threshold in milliseconds, `0..=2^53-1`; default `100`.
    pub slow_threshold_ms: Option<u64>,
}

/// Invalid per-engine settings. Process-wide SQLite mode failures use
/// [`RuntimeConfigurationError`] instead.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineConfigurationError {
    /// A requested value is outside its inclusive accepted range.
    OutOfRange { field: &'static str, value: u64, min: u64, max: u64 },
    /// A derived bounded capacity cannot fit the native platform width.
    CapacityOverflow { field: &'static str },
}

impl Display for EngineConfigurationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfRange { field, value, min, max } => {
                write!(f, "engine configuration {field}={value} is outside {min}..={max}")
            }
            Self::CapacityOverflow { field } => {
                write!(f, "engine configuration capacity {field} exceeds the platform width")
            }
        }
    }
}

impl Error for EngineConfigurationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedRuntimeConfiguration {
    pub(crate) scheduler_runtime_threads: usize,
    pub(crate) embedder_pool_size: usize,
    pub(crate) embedder_call_timeout_ms: u64,
    pub(crate) provenance_row_cap: u64,
    pub(crate) slow_threshold_ms: u64,
    pub(crate) embed_waiting_capacity: usize,
    pub(crate) projection_admission_capacity: usize,
}

impl ResolvedRuntimeConfiguration {
    pub(crate) fn resolve(requested: &EngineConfig) -> Result<Self, EngineConfigurationError> {
        let scheduler = checked_value(
            "scheduler_runtime_threads",
            requested.scheduler_runtime_threads,
            2,
            1,
            64,
        )?;
        let embedder = checked_value("embedder_pool_size", requested.embedder_pool_size, 5, 1, 64)?;
        let embedder_call_timeout_ms = checked_value(
            "embedder_call_timeout_ms",
            requested.embedder_call_timeout_ms,
            DEFAULT_EMBED_TIMEOUT_MS,
            1,
            u64::from(u32::MAX),
        )?;
        let provenance_row_cap = checked_value(
            "provenance_row_cap",
            requested.provenance_row_cap,
            DEFAULT_PROVENANCE_ROW_CAP,
            0,
            MAX_SAFE_INTEGER,
        )?;
        let slow_threshold_ms = checked_value(
            "slow_threshold_ms",
            requested.slow_threshold_ms,
            DEFAULT_SLOW_THRESHOLD_MS,
            0,
            MAX_SAFE_INTEGER,
        )?;
        let scheduler_runtime_threads = usize::try_from(scheduler).map_err(|_| {
            EngineConfigurationError::CapacityOverflow { field: "scheduler_runtime_threads" }
        })?;
        let embedder_pool_size = usize::try_from(embedder).map_err(|_| {
            EngineConfigurationError::CapacityOverflow { field: "embedder_pool_size" }
        })?;
        let embed_waiting_capacity = embedder_pool_size.checked_mul(4).ok_or(
            EngineConfigurationError::CapacityOverflow { field: "embed_waiting_capacity" },
        )?;
        let projection_admission_capacity = scheduler_runtime_threads
            .checked_mul(PROJECTION_COMMIT_BATCH)
            .ok_or(EngineConfigurationError::CapacityOverflow {
                field: "projection_admission_capacity",
            })?;
        Ok(Self {
            scheduler_runtime_threads,
            embedder_pool_size,
            embedder_call_timeout_ms,
            provenance_row_cap,
            slow_threshold_ms,
            embed_waiting_capacity,
            projection_admission_capacity,
        })
    }
}

fn checked_value(
    field: &'static str,
    requested: Option<u64>,
    default: u64,
    min: u64,
    max: u64,
) -> Result<u64, EngineConfigurationError> {
    let value = requested.unwrap_or(default);
    if !(min..=max).contains(&value) {
        return Err(EngineConfigurationError::OutOfRange { field, value, min, max });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_accepted_scheduler_and_embed_pair_has_exact_distinct_capacities() {
        for scheduler in 1..=64_u64 {
            for embedder in 1..=64_u64 {
                let resolved = ResolvedRuntimeConfiguration::resolve(&EngineConfig {
                    scheduler_runtime_threads: Some(scheduler),
                    embedder_pool_size: Some(embedder),
                    ..EngineConfig::default()
                })
                .unwrap();
                assert_eq!(
                    resolved.projection_admission_capacity,
                    scheduler as usize * PROJECTION_COMMIT_BATCH
                );
                assert_eq!(resolved.embed_waiting_capacity, embedder as usize * 4);
            }
        }
    }

    #[test]
    fn omission_and_explicit_zero_keep_their_distinct_meaning() {
        let default = ResolvedRuntimeConfiguration::resolve(&EngineConfig::default()).unwrap();
        assert_eq!((default.scheduler_runtime_threads, default.embedder_pool_size), (2, 5));
        assert_eq!(default.embed_waiting_capacity, 20);
        assert_eq!(default.embedder_call_timeout_ms, 30_000);
        assert_eq!((default.provenance_row_cap, default.slow_threshold_ms), (1_000_000, 100));
        let zero = ResolvedRuntimeConfiguration::resolve(&EngineConfig {
            provenance_row_cap: Some(0),
            slow_threshold_ms: Some(0),
            ..EngineConfig::default()
        })
        .unwrap();
        assert_eq!((zero.provenance_row_cap, zero.slow_threshold_ms), (0, 0));
    }

    #[test]
    fn upper_bound_and_overflow_policy_are_exact() {
        let max = ResolvedRuntimeConfiguration::resolve(&EngineConfig {
            scheduler_runtime_threads: Some(64),
            embedder_pool_size: Some(64),
            embedder_call_timeout_ms: Some(u64::from(u32::MAX)),
            provenance_row_cap: Some(MAX_SAFE_INTEGER),
            slow_threshold_ms: Some(MAX_SAFE_INTEGER),
        })
        .unwrap();
        assert_eq!(max.embed_waiting_capacity, 256);
        assert_eq!(max.projection_admission_capacity, 64 * PROJECTION_COMMIT_BATCH);
        assert_eq!(max.embedder_call_timeout_ms, u64::from(u32::MAX));
        assert_eq!(max.provenance_row_cap, MAX_SAFE_INTEGER);
        assert_eq!(max.slow_threshold_ms, MAX_SAFE_INTEGER);

        let invalid = ResolvedRuntimeConfiguration::resolve(&EngineConfig {
            scheduler_runtime_threads: Some(u64::MAX),
            ..EngineConfig::default()
        });
        assert_eq!(
            invalid,
            Err(EngineConfigurationError::OutOfRange {
                field: "scheduler_runtime_threads",
                value: u64::MAX,
                min: 1,
                max: 64,
            })
        );
    }
}
