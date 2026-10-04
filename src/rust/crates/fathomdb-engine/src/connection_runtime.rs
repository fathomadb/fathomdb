use super::*;
use sqlite_vec::sqlite3_vec_init;
use std::sync::Once;

/// Per-reader-connection lookaside slot size, in bytes. Pack 6.G G.1.
/// Picked from G.0 telemetry (`allocator_lookaside` 26.67% conc cycles
/// with 3.89× ratio) + the SQLite docs' typical-workload sizing
/// guidance (https://www.sqlite.org/malloc.html §3): 1200-byte slots
/// cover the small allocations from `sqlite3DbMallocRaw`,
/// `sqlite3Fts5ExprNew`, and `vec0Filter_knn` visible at the top of the
/// concurrent profile.
const READER_LOOKASIDE_SLOT_SIZE: std::os::raw::c_int = 1200;

/// Per-reader-connection lookaside slot count. SQLite default is 128;
/// we use 500 to absorb the per-statement allocation footprint of the
/// hybrid search workload across a sticky worker connection without
/// falling back to the glibc malloc-arena mutex.
const READER_LOOKASIDE_SLOT_COUNT: std::os::raw::c_int = 500;

/// Per-connection profile-callback context.
///
/// Holds the registry handle the callback dispatches to, plus shared
/// references to the engine's profiling toggle and slow-statement
/// threshold. The `Arc` clones here mirror the same atomics held by
/// `Engine`, so `set_profiling` / `set_slow_threshold_ms` mutations are
/// visible inside the callback without restart (REQ-006a / AC-005a /
/// AC-007b runtime-toggle contract).
#[derive(Debug)]
pub(crate) struct ProfileContext {
    subscribers: Arc<lifecycle::SubscriberRegistry>,
    profiling_enabled: Arc<AtomicBool>,
    slow_threshold_ms: Arc<AtomicU64>,
    #[cfg(test)]
    pub(crate) callback_uninstalled: AtomicBool,
}

#[cfg(not(test))]
pub(crate) type ProfileContexts = Vec<Box<ProfileContext>>;

#[cfg(test)]
pub(crate) struct ProfileContexts {
    // SQLite stores these allocation addresses; moving a context would invalidate userdata.
    #[allow(clippy::vec_box)]
    pub(crate) contexts: Vec<Box<ProfileContext>>,
    pub(crate) observer: Option<Arc<ProfileReleaseObserver>>,
}

#[cfg(test)]
impl From<Vec<Box<ProfileContext>>> for ProfileContexts {
    fn from(contexts: Vec<Box<ProfileContext>>) -> Self {
        Self { contexts, observer: None }
    }
}

#[cfg(test)]
pub(crate) struct ProfileReleaseObserver {
    pub(crate) registry: Arc<ManagedConnectionRegistry>,
    pub(crate) live_workers: Arc<AtomicUsize>,
    pub(crate) releases: Mutex<Vec<ProfileReleaseFact>>,
    // Custody preserves the original SQLite userdata even in an early-release mutant.
    #[allow(clippy::vec_box)]
    pub(crate) custody: Mutex<Vec<Box<ProfileContext>>>,
}

#[cfg(test)]
pub(crate) struct ProfileReleaseFact {
    pub(crate) callback_uninstalled: bool,
    pub(crate) live_connections: BTreeSet<(WalAttributionRole, usize)>,
    pub(crate) live_workers: usize,
}

#[cfg(test)]
impl ProfileContexts {
    pub(crate) fn clear(&mut self) {
        if let Some(observer) = &self.observer {
            for context in self.contexts.drain(..) {
                observer.releases.lock().unwrap().push(ProfileReleaseFact {
                    callback_uninstalled: context.callback_uninstalled.load(Ordering::SeqCst),
                    live_connections: observer.registry.live.lock().unwrap().clone(),
                    live_workers: observer.live_workers.load(Ordering::SeqCst),
                });
                observer.custody.lock().unwrap().push(context);
            }
        } else {
            self.contexts.clear();
        }
    }
}

#[cfg(test)]
impl Drop for ProfileContexts {
    fn drop(&mut self) {
        self.clear();
    }
}

pub(crate) fn open_managed_connection(
    path: &Path,
    #[cfg(any(test, feature = "test-hooks"))] category: ManagedConnectionCategory,
    #[cfg(any(test, feature = "test-hooks"))] managed_connections: &Arc<ManagedConnectionRegistry>,
) -> rusqlite::Result<Connection> {
    #[cfg(any(test, feature = "test-hooks"))]
    managed_connections.record_open(category);
    Connection::open(sqlite_runtime_path(path))
}

pub(crate) fn sqlite_runtime_path(path: &Path) -> &Path {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        let simplified = dunce::simplified(path);
        // SQLite also creates sidecars beside the database. Keep enough of the
        // legacy Win32 path budget for those suffixes; special and long paths
        // retain their verbatim form for the bundled VFS correction.
        if simplified.as_os_str().encode_wide().count() <= 248 {
            simplified
        } else {
            path
        }
    }
    #[cfg(not(windows))]
    {
        path
    }
}

#[cfg(windows)]
pub(crate) fn sqlite_local_verbatim_path_requires_patched_vfs(
    path: &Path,
    sqlite_version: i32,
    patched_vfs: bool,
) -> bool {
    let path_text = path.to_string_lossy();
    let bytes = path_text.as_bytes();
    let local_verbatim_drive = bytes.len() >= 7
        && &bytes[..4] == b"\\\\?\\"
        && bytes[4].is_ascii_alphabetic()
        && bytes[5] == b':'
        && bytes[6] == b'\\';
    local_verbatim_drive
        && sqlite_runtime_path(path) == path
        && sqlite_version < 3_053_003
        && !patched_vfs
}

#[cfg(windows)]
pub(crate) fn ensure_sqlite_local_verbatim_path_safe(path: &Path) -> Result<(), EngineOpenError> {
    // SQLite can drop a trailing dot while the engine lock keeps it, splitting
    // the requested database and lock-file identities.
    if sqlite_windows_path_has_trailing_dot_or_space(path) {
        return Err(EngineOpenError::Io {
            message: "Windows database path has a trailing dot or space component".to_string(),
        });
    }
    if sqlite_local_verbatim_path_requires_patched_vfs(
        path,
        rusqlite::version_number(),
        sqlite_local_verbatim_vfs_is_patched(),
    ) {
        return Err(EngineOpenError::Io {
            message: "SQLite cannot safely open this local Windows database path".to_string(),
        });
    }
    Ok(())
}

#[cfg(windows)]
pub(crate) fn sqlite_local_verbatim_vfs_is_patched() -> bool {
    unsafe {
        rusqlite::ffi::sqlite3_compileoption_used(c"FATHOMDB_WIN_VERBATIM_SHM_SAFE".as_ptr()) != 0
    }
}

#[cfg(windows)]
pub(crate) fn sqlite_windows_path_has_trailing_dot_or_space(path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Component;

    path.components().any(|component| {
        if let Component::Normal(name) = component {
            matches!(name.encode_wide().last(), Some(46 | 32))
        } else {
            false
        }
    })
}

pub(crate) fn open_runtime_connection(
    path: &Path,
    #[cfg(any(test, feature = "test-hooks"))] category: ManagedConnectionCategory,
    #[cfg(any(test, feature = "test-hooks"))] managed_connections: &Arc<ManagedConnectionRegistry>,
) -> rusqlite::Result<Connection> {
    let connection = open_managed_connection(
        path,
        #[cfg(any(test, feature = "test-hooks"))]
        category,
        #[cfg(any(test, feature = "test-hooks"))]
        managed_connections,
    )?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    // OPP-12 Phase-1 (0.8.19 Slice 10, design §3 gap-4) — `secure_delete=ON` at
    // EVERY open. The projection/vector-rewrite runtime connection performs
    // DELETEs (shadow-table rewrites), so its freed pages must be scrubbed too;
    // setting the pragma only on the writer left a GDPR-erasure leak here.
    connection.pragma_update(None, "secure_delete", "ON")?;
    Ok(connection)
}

pub(crate) fn register_sqlite_vec_extension() {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| unsafe {
        let entrypoint: unsafe extern "C" fn(
            *mut rusqlite::ffi::sqlite3,
            *mut *mut std::os::raw::c_char,
            *const rusqlite::ffi::sqlite3_api_routines,
        ) -> std::os::raw::c_int = std::mem::transmute(sqlite3_vec_init as *const ());
        rusqlite::ffi::sqlite3_auto_extension(Some(entrypoint));
    });
}

/// Install a `sqlite3_profile` callback on `connection` that dispatches
/// per-statement profile records and slow-statement signals to the
/// engine's subscriber registry.
///
/// Why FFI rather than `rusqlite::Connection::profile`: the safe API
/// (rusqlite 0.31) accepts only a `fn(&str, Duration)` with no
/// environment, so it cannot carry a per-engine subscriber-registry
/// pointer. We use `sqlite3_profile` directly with a leaked-into-`Box`
/// context whose pointer is tied to the engine's lifetime via
/// `Engine::profile_contexts`.
///
/// `sqlite3_profile` is documented as deprecated in favor of
/// `sqlite3_trace_v2`, but it remains supported and is sufficient for
/// the wall-clock + SQL-text payload required by AC-005a/b.
#[allow(clippy::vec_box)]
pub(crate) fn install_profile_callback(
    connection: &Connection,
    subscribers: &Arc<lifecycle::SubscriberRegistry>,
    profiling_enabled: &Arc<AtomicBool>,
    slow_threshold_ms: &Arc<AtomicU64>,
    contexts: &mut Vec<Box<ProfileContext>>,
) {
    let mut ctx = Box::new(ProfileContext {
        subscribers: Arc::clone(subscribers),
        profiling_enabled: Arc::clone(profiling_enabled),
        slow_threshold_ms: Arc::clone(slow_threshold_ms),
        #[cfg(test)]
        callback_uninstalled: AtomicBool::new(false),
    });
    let ctx_ptr: *mut ProfileContext = &mut *ctx;

    // SAFETY: the Box outlives the connection. Rust drops struct fields
    // in declaration order. `connection` and `reader_pool` are declared
    // before `profile_contexts`. `ReaderWorkerPool::Drop` joins every
    // reader worker, and each worker uninstalls and drops its owned
    // connection inside `reader_worker_loop` before the worker thread
    // returns. Therefore all connections — and SQLite's internal
    // profile-callback state with them — are torn down before the
    // `Box<ProfileContext>` allocations are freed. `Engine::close`
    // additionally clears the callback via
    // `sqlite3_profile(handle, None, NULL)` before connection close to
    // drain any in-flight callback dispatch.
    unsafe {
        rusqlite::ffi::sqlite3_profile(
            connection.handle(),
            Some(profile_callback_trampoline),
            ctx_ptr.cast::<std::ffi::c_void>(),
        );
    }
    contexts.push(ctx);
}

/// Uninstall the profile callback so SQLite stops calling into our
/// freed `Box<ProfileContext>` pointer once a connection is being torn
/// down. Call before dropping `profile_contexts`.
pub(crate) fn uninstall_profile_callback(connection: &Connection) {
    // SAFETY: passing `None` as the callback unregisters the previous
    // callback; SQLite documents this as legal and idempotent.
    unsafe {
        let previous =
            rusqlite::ffi::sqlite3_profile(connection.handle(), None, std::ptr::null_mut());
        #[cfg(test)]
        if !previous.is_null() {
            // The connection holds our stable context, retained until teardown completes.
            (*previous.cast::<ProfileContext>()).callback_uninstalled.store(true, Ordering::SeqCst);
        }
        #[cfg(not(test))]
        let _ = previous;
    }
}

/// Pack 6.G G.1 — configure SQLite per-connection lookaside on a reader
/// worker connection. Must be called BEFORE any statement is prepared
/// or any PRAGMA is run on `connection`; per the SQLite docs
/// (https://www.sqlite.org/malloc.html §3) lookaside is silently
/// ignored if reconfigured after the first allocation on the
/// connection. Passing `NULL` for the buffer pointer lets SQLite
/// allocate the lookaside backing memory itself.
///
/// rusqlite 0.31's `set_db_config` only handles the boolean
/// `DbConfig::*` variants; `SQLITE_DBCONFIG_LOOKASIDE` is not surfaced
/// (it is commented out in `rusqlite/src/config.rs`), so we call the
/// raw FFI directly.
///
/// Returns the rc of `sqlite3_db_config` so callers can debug-assert
/// `SQLITE_OK` and surface configuration failure under
/// `debug_assertions` test builds without expanding the public surface.
/// 0.7.0 perf-experiments hook: apply caller-supplied reader PRAGMAs
/// from the `FATHOMDB_PERF_READER_PRAGMAS` env var. Format:
/// comma-separated `name=value` pairs (e.g.
/// `cache_size=-262144,mmap_size=268435456,temp_store=MEMORY`).
///
/// **Gated on `FATHOMDB_PERF_EXPERIMENTS=1`.** No-op if the gate env
/// var is unset, so production paths are never affected. Failures to
/// apply individual PRAGMAs are logged to stderr (via `eprintln!`) but
/// do not error the connection open — experiments are best-effort,
/// not contract.
///
/// Scope: 0.7.0 perf-experiment campaign per
/// `dev/plans/0.7.0-perf-experiments.md`. Once Wave 5 picks the
/// landing combination, the chosen PRAGMAs are hardcoded as the new
/// reader-open default and this hook is removed.
/// 0.7.0 perf-experiments hook: apply writer-side PRAGMAs from
/// `FATHOMDB_PERF_WRITER_PRAGMAS` (same format as reader hook).
/// **Runs BEFORE migrations** so PRAGMAs like `page_size` that must
/// precede any table creation take effect on a fresh DB.
///
/// Gated on `FATHOMDB_PERF_EXPERIMENTS=1`. No-op otherwise.
pub(crate) fn apply_perf_experiment_writer_pragmas(connection: &Connection) {
    if std::env::var_os("FATHOMDB_PERF_EXPERIMENTS").is_none() {
        return;
    }
    let raw = match std::env::var("FATHOMDB_PERF_WRITER_PRAGMAS") {
        Ok(s) if !s.is_empty() => s,
        _ => return,
    };
    for entry in raw.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let (name, value) = match entry.split_once('=') {
            Some((n, v)) => (n.trim(), v.trim()),
            None => {
                eprintln!("perf-experiment: bad writer pragma entry (expect name=value): {entry}");
                continue;
            }
        };
        if name.is_empty() {
            eprintln!("perf-experiment: empty pragma name in writer entry: {entry}");
            continue;
        }
        match connection.pragma_update(None, name, value) {
            Ok(()) => {
                eprintln!(
                    "perf-experiment: applied PRAGMA {name}={value} on writer (pre-migration)"
                );
            }
            Err(err) => {
                eprintln!("perf-experiment: writer PRAGMA {name}={value} failed: {err}");
            }
        }
    }
}

pub(crate) fn apply_perf_experiment_reader_pragmas(connection: &Connection) {
    if std::env::var_os("FATHOMDB_PERF_EXPERIMENTS").is_none() {
        return;
    }
    let raw = match std::env::var("FATHOMDB_PERF_READER_PRAGMAS") {
        Ok(s) if !s.is_empty() => s,
        _ => return,
    };
    for entry in raw.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let (name, value) = match entry.split_once('=') {
            Some((n, v)) => (n.trim(), v.trim()),
            None => {
                eprintln!("perf-experiment: bad pragma entry (expect name=value): {entry}");
                continue;
            }
        };
        if name.is_empty() {
            eprintln!("perf-experiment: empty pragma name in entry: {entry}");
            continue;
        }
        match connection.pragma_update(None, name, value) {
            Ok(()) => {
                eprintln!("perf-experiment: applied PRAGMA {name}={value} on reader");
            }
            Err(err) => {
                eprintln!("perf-experiment: PRAGMA {name}={value} failed: {err}");
            }
        }
    }
}

pub(crate) fn configure_reader_lookaside(connection: &Connection) -> std::os::raw::c_int {
    // SAFETY: `connection.handle()` returns a valid `*mut sqlite3` for
    // the lifetime of `connection`. The variadic
    // `sqlite3_db_config(LOOKASIDE)` call expects three trailing
    // arguments of types `void*`, `int`, `int` — the prototype shape
    // documented in `sqlite3.h`. We pass a null buffer so SQLite owns
    // the lookaside backing allocation, and the slot size / count from
    // the G.1 constants. No allocations happen on the connection
    // before this call (reader open path is `Connection::open` ->
    // `configure_reader_lookaside` -> first PRAGMA).
    unsafe {
        rusqlite::ffi::sqlite3_db_config(
            connection.handle(),
            rusqlite::ffi::SQLITE_DBCONFIG_LOOKASIDE,
            std::ptr::null_mut::<std::ffi::c_void>(),
            READER_LOOKASIDE_SLOT_SIZE,
            READER_LOOKASIDE_SLOT_COUNT,
        )
    }
}

/// FFI trampoline for `sqlite3_profile`.
///
/// Invoked by SQLite at statement-finish with the SQL text and the
/// statement's wall-clock cost in nanoseconds. We dispatch a
/// `ProfileRecord` (when profiling is enabled) and a `SlowStatement`
/// signal (when `wall_clock_ms` exceeds the configured slow threshold).
///
/// Per `dev/design/lifecycle.md` § Public record shape, the public
/// payload exposes `wall_clock_ms`, `step_count`, and `cache_delta`.
/// `sqlite3_profile` does not surface per-statement step counts or
/// cache-hit deltas in its callback; we emit `0` for those fields and
/// document the hazard. AC-005b requires the fields be typed numeric,
/// not that they carry non-zero values for every backend.
unsafe extern "C" fn profile_callback_trampoline(
    user_data: *mut std::ffi::c_void,
    sql: *const std::os::raw::c_char,
    nanoseconds: u64,
) {
    if user_data.is_null() || sql.is_null() {
        return;
    }
    let ctx = unsafe { &*(user_data.cast::<ProfileContext>()) };
    let sql_text = match unsafe { std::ffi::CStr::from_ptr(sql) }.to_str() {
        Ok(s) => s,
        Err(_) => return,
    };

    #[cfg(feature = "test-hooks")]
    record_slice71_profile_statement_for_test(sql_text);

    let wall_clock_ms = nanoseconds / 1_000_000;

    if ctx.profiling_enabled.load(Ordering::Relaxed) {
        let record = lifecycle::ProfileRecord {
            wall_clock_ms,
            // step_count / cache_delta are not surfaced by
            // sqlite3_profile; placeholder 0 satisfies AC-005b's
            // "typed numeric" contract. A future profiling refactor
            // around sqlite3_stmt_status + sqlite3_db_status would
            // populate them with non-zero deltas.
            step_count: 0,
            cache_delta: 0,
        };
        ctx.subscribers.dispatch_profile(&record);
    }

    let threshold = ctx.slow_threshold_ms.load(Ordering::Relaxed);
    if wall_clock_ms > threshold {
        let signal = lifecycle::SlowStatement { statement: sql_text.to_string(), wall_clock_ms };
        ctx.subscribers.dispatch_slow_statement(&signal);
    }
}

impl Engine {
    /// Pack 6.G G.1 — return the `sqlite3_db_config(LOOKASIDE)` rc
    /// captured for each reader worker at open time, in worker index
    /// order. SQLITE_OK (= 0) means the lookaside was configured
    /// before any allocation happened on the connection.
    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn reader_lookaside_config_rcs_for_test(&self) -> Vec<i32> {
        self.reader_lookaside_rcs.clone()
    }

    /// OPP-12 Phase-1 (0.8.19 Slice 10) — read the writer connection's
    /// `PRAGMA secure_delete` (design §3 gap-4). `true` iff the standing
    /// connection-open PRAGMA is in effect, so `purge` freelist erasure is
    /// complete without a per-purge `VACUUM`.
    #[doc(hidden)]
    pub fn secure_delete_enabled_for_test(&self) -> Result<bool, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let value: i64 = connection
            .query_row("PRAGMA secure_delete", [], |r| r.get(0))
            .map_err(|_| EngineError::Storage)?;
        Ok(value != 0)
    }

    /// OPP-12 Phase-1 (0.8.19 Slice 10, design §3 gap-4) — `true` iff a freshly
    /// opened projection/runtime connection (`open_runtime_connection`) reports
    /// `PRAGMA secure_delete = ON`. The runtime connection performs the
    /// vector-rewrite/projection DELETEs, so its freed pages must be scrubbed too.
    #[doc(hidden)]
    pub fn runtime_secure_delete_enabled_for_test(&self) -> Result<bool, EngineError> {
        self.ensure_open()?;
        let connection = open_runtime_connection(
            &self.path,
            #[cfg(any(test, feature = "test-hooks"))]
            ManagedConnectionCategory::RuntimeProbe,
            #[cfg(any(test, feature = "test-hooks"))]
            &self.managed_connections,
        )
        .map_err(|_| EngineError::Storage)?;
        let value: i64 = connection
            .query_row("PRAGMA secure_delete", [], |r| r.get(0))
            .map_err(|_| EngineError::Storage)?;
        Ok(value != 0)
    }
}

#[cfg(feature = "test-hooks")]
pub(super) fn record_writer_pragma_witness_for_test(connection: &Connection) {
    let observation = (|| -> rusqlite::Result<serde_json::Value> {
        Ok(serde_json::json!({
            "role": "writer",
            "journal_mode": connection.pragma_query_value(
                None,
                "journal_mode",
                |row| row.get::<_, String>(0),
            )?,
            "synchronous": connection.pragma_query_value(
                None,
                "synchronous",
                |row| row.get::<_, i64>(0),
            )?,
        }))
    })();
    if let Ok(observation) = observation {
        append_json_witness_for_test("FATHOMDB_WRITER_PRAGMA_WITNESS_FOR_TEST", &observation);
    }
}
