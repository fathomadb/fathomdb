use std::fmt::{Display, Formatter};
use std::io::{Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use super::data_plane::shared_for_migrate::{
    immutable_uri as immutable_sqlite_uri, sidecar_path as data_plane_sidecar_path,
    validate_database_file as validate_recovery_database_file,
    validate_schema_invariants as validate_recovery_schema_invariants,
};
use super::*;
use crate::errors::map_open_sqlite_error;
use crate::open::{admit_current_database, read_only_sqlite_uri, sqlite_uri};

const SCHEMA_37: u32 = 37;
const SCHEMA_38: u32 = 38;
const SCHEMA_39: u32 = 39;

/// Preflight edge-endpoint counts observed on a Schema-37 database before upgrade.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MigratePreflightReport {
    /// Active edges (`superseded_at IS NULL`) whose `from_id` or `to_id` has
    /// zero rows with matching `logical_id` in `canonical_nodes`. Any non-zero
    /// count fails closed (`E_MIGRATE_DANGLING_EDGES`) before backup or DDL.
    pub dangling_edge_endpoints: u64,
    /// Active edges (`superseded_at IS NULL`) whose `from_id` or `to_id` has
    /// rows in `canonical_nodes`, none of which is currently non-superseded.
    /// Admitted when `dangling_edge_endpoints == 0`.
    pub superseded_edge_endpoints: u64,
}

/// Outcome of an offline Schema 37 -> 38 migration via [`migrate_37_to_38`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrateReport {
    pub from_version: u32,
    pub to_version: u32,
    pub db_path: PathBuf,
    pub backup_path: PathBuf,
    pub backup_sha256_path: PathBuf,
    pub backup_sha256: String,
    pub preflight: MigratePreflightReport,
    pub integrity: IntegrityReport,
    pub pending_erasures: Vec<PendingErasure>,
}

impl MigrateReport {
    /// True iff post-upgrade full integrity and pending-erasure checks are clean.
    #[must_use]
    pub fn integrity_clean(&self) -> bool {
        self.integrity.physical == Section::Clean
            && self.integrity.logical == Section::Clean
            && self.integrity.semantic == Section::Clean
            && self.pending_erasures.is_empty()
    }
}

/// Stable machine-readable error code for [`MigrateError`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrateErrorCode {
    /// Exclusive product lock is held or WAL checkpoint reported `busy != 0`.
    LockHeld,
    /// Database is already at the target schema version (Schema 39, or Schema 38 for `migrate_37_to_38`).
    AlreadyCurrent,
    /// Database `PRAGMA user_version` is unsupported for the requested upgrade (`38 -> 39` or chained `37 -> 38 -> 39`).
    UnsupportedVersion,
    /// Unfinished physical erasure, active/queued erasure request, or redaction debt exists.
    PendingErasure,
    /// One or more active edges reference a `from_id`/`to_id` absent from `canonical_nodes`.
    DanglingEdges,
    /// Backup directory is invalid, colliding, or unwritable.
    BackupDirInvalid,
    /// Source database is missing, empty, corrupt, or has a non-empty rollback journal / bad WAL.
    DatabaseInvalid,
    /// Schema DDL transaction (`38 -> 39` or `37 -> 38`) failed and rolled back leaving the prior schema intact.
    RollbackFailedOrDdlAborted,
}

impl MigrateErrorCode {
    /// Canonical uppercase error code string for CLI/JSON diagnostics.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LockHeld => "E_MIGRATE_LOCK_HELD",
            Self::AlreadyCurrent => "E_MIGRATE_ALREADY_CURRENT",
            Self::UnsupportedVersion => "E_MIGRATE_UNSUPPORTED_VERSION",
            Self::PendingErasure => "E_MIGRATE_PENDING_ERASURE",
            Self::DanglingEdges => "E_MIGRATE_DANGLING_EDGES",
            Self::BackupDirInvalid => "E_MIGRATE_BACKUP_DIR",
            Self::DatabaseInvalid => "E_MIGRATE_DATABASE_INVALID",
            Self::RollbackFailedOrDdlAborted => "E_MIGRATE_DDL_ABORTED",
        }
    }
}

/// Structured refusal returned by [`migrate_37_to_38`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrateError {
    pub code: MigrateErrorCode,
    pub message: String,
    pub seen_version: Option<u32>,
    pub preflight: Option<MigratePreflightReport>,
    pub backup_path: Option<PathBuf>,
    pub backup_sha256: Option<String>,
}

impl Display for MigrateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for MigrateError {}

impl MigrateError {
    fn simple(code: MigrateErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            seen_version: None,
            preflight: None,
            backup_path: None,
            backup_sha256: None,
        }
    }
}

#[cfg(any(test, feature = "migration-test-hooks"))]
type MigrateFaultHook = Box<dyn FnOnce() -> Result<(), String> + Send>;

#[cfg(any(test, feature = "migration-test-hooks"))]
static MIGRATE_BEFORE_VERSION_STAMP_HOOKS: Mutex<Vec<(PathBuf, u32, MigrateFaultHook)>> =
    Mutex::new(Vec::new());

#[cfg(any(test, feature = "migration-test-hooks"))]
fn arm_migrate_before_version_stamp_hook(
    db_path: &Path,
    target_version: u32,
    hook: impl FnOnce() -> Result<(), String> + Send + 'static,
) {
    let target = canonical_database_path(db_path).unwrap_or_else(|_| db_path.to_path_buf());
    let mut hooks = MIGRATE_BEFORE_VERSION_STAMP_HOOKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    hooks.retain(|(existing, ver, _)| existing != &target || *ver != target_version);
    hooks.push((target, target_version, Box::new(hook)));
}

/// Test-only path-scoped fault-injection hook invoked inside [`migrate_37_to_38`]'s
/// single `BEGIN IMMEDIATE` transaction immediately after `STEP_38_SQL` executes
/// and immediately before `PRAGMA user_version = 38`.
#[cfg(any(test, feature = "migration-test-hooks"))]
pub fn arm_migrate_37_to_38_before_version_stamp_hook_for_test(
    db_path: impl AsRef<Path>,
    hook: impl FnOnce() -> Result<(), String> + Send + 'static,
) {
    arm_migrate_before_version_stamp_hook(db_path.as_ref(), SCHEMA_38, hook);
}

/// Test-only path-scoped fault-injection hook invoked inside [`migrate_38_to_39`]'s
/// single `BEGIN IMMEDIATE` transaction immediately after `STEP_39_SQL` and any
/// `vec0` reshape execute and immediately before `PRAGMA user_version = 39`.
#[cfg(any(test, feature = "migration-test-hooks"))]
pub fn arm_migrate_38_to_39_before_version_stamp_hook_for_test(
    db_path: impl AsRef<Path>,
    hook: impl FnOnce() -> Result<(), String> + Send + 'static,
) {
    arm_migrate_before_version_stamp_hook(db_path.as_ref(), SCHEMA_39, hook);
}

#[cfg(any(test, feature = "migration-test-hooks"))]
fn fire_migrate_before_version_stamp_hook(
    canonical_path: &Path,
    target_version: u32,
) -> Result<(), String> {
    let hook = {
        let mut hooks = MIGRATE_BEFORE_VERSION_STAMP_HOOKS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        hooks
            .iter()
            .position(|(p, ver, _)| p == canonical_path && *ver == target_version)
            .map(|idx| hooks.remove(idx).2)
    };
    if let Some(hook) = hook {
        hook()?;
    }
    Ok(())
}

fn prepare_migration_environment(
    requested_path: &Path,
    backup_dir_requested: &Path,
    from_version: u32,
    to_version: u32,
) -> Result<(PathBuf, File, MigratePreflightReport), MigrateError> {
    // Step 1: Quiescence, exclusive product lock, rollback-journal refusal, and backup-dir precheck.
    let canonical_path = canonical_database_path(requested_path)
        .map_err(|err| map_open_error_for_migrate(err, from_version, to_version))?;
    validate_recovery_database_file(&canonical_path)
        .map_err(|err| map_open_error_for_migrate(err, from_version, to_version))?;
    precheck_backup_dir(backup_dir_requested, &canonical_path)?;

    let pending_lock = acquire_lock_without_metadata_mutation(&canonical_path)
        .map_err(|err| map_open_error_for_migrate(err, from_version, to_version))?;
    let lock = pending_lock
        .initialize()
        .map_err(|err| map_open_error_for_migrate(err, from_version, to_version))?;

    // Re-validate after acquiring lock (closes rename/truncate race).
    validate_recovery_database_file(&canonical_path)
        .map_err(|err| map_open_error_for_migrate(err, from_version, to_version))?;
    match std::fs::metadata(data_plane_sidecar_path(&canonical_path, "-journal")) {
        Ok(metadata) if metadata.len() > 0 => {
            return Err(MigrateError::simple(
                MigrateErrorCode::DatabaseInvalid,
                "non-empty rollback journal blocks offline migration",
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(MigrateError::simple(
                MigrateErrorCode::DatabaseInvalid,
                "database rollback journal is not accessible",
            ));
        }
    }

    configure_runtime_for_open().map_err(|err| {
        map_open_error_for_migrate(
            EngineOpenError::RuntimeConfiguration(err),
            from_version,
            to_version,
        )
    })?;
    register_sqlite_vec_extension();

    // Capture SHM snapshot so any refusal in Steps 2-4 restores transient sidecar state.
    let shm = ShmSnapshot::capture(&canonical_path)
        .map_err(|err| map_open_error_for_migrate(err, from_version, to_version))?;

    // Steps 2, 3, 4: Strictly read-only validation, erasure debt refusal, and edge endpoint preflight.
    let preflight = match run_readonly_preflights(&canonical_path, from_version, to_version) {
        Ok(preflight) => preflight,
        Err(err) => {
            let _ = shm.restore();
            return Err(err);
        }
    };

    Ok((canonical_path, lock, preflight))
}

fn execute_upgrade_transaction_and_verify<F>(
    connection: &Connection,
    canonical_path: &Path,
    backup_dir_requested: &Path,
    from_version: u32,
    to_version: u32,
    preflight: MigratePreflightReport,
    apply_ddl: F,
) -> Result<MigrateReport, MigrateError>
where
    F: FnOnce(&Connection) -> Result<(), String>,
{
    checkpoint_truncate_quiescent(connection, canonical_path, from_version, to_version)?;

    let (backup_path, backup_sha256_path, backup_sha256) =
        write_verified_backup(canonical_path, backup_dir_requested, from_version)?;

    // Step 6: Single `BEGIN IMMEDIATE` transaction applying schema DDL verbatim.
    connection.execute_batch("BEGIN IMMEDIATE").map_err(|err| MigrateError {
        code: MigrateErrorCode::RollbackFailedOrDdlAborted,
        message: format!("failed to begin Schema {to_version} migration transaction: {err}"),
        seen_version: Some(from_version),
        preflight: Some(preflight),
        backup_path: Some(backup_path.clone()),
        backup_sha256: Some(backup_sha256.clone()),
    })?;

    match apply_ddl(connection) {
        Ok(()) => {
            if let Err(err) = connection.execute_batch("COMMIT") {
                let _ = connection.execute_batch("ROLLBACK");
                let _ = connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))
                });
                return Err(MigrateError {
                    code: MigrateErrorCode::RollbackFailedOrDdlAborted,
                    message: format!(
                        "Schema {to_version} COMMIT failed and was rolled back: {err}"
                    ),
                    seen_version: Some(from_version),
                    preflight: Some(preflight),
                    backup_path: Some(backup_path),
                    backup_sha256: Some(backup_sha256),
                });
            }
        }
        Err(message) => {
            let _ = connection.execute_batch("ROLLBACK");
            let _ = connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))
            });
            return Err(MigrateError {
                code: MigrateErrorCode::RollbackFailedOrDdlAborted,
                message,
                seen_version: Some(from_version),
                preflight: Some(preflight),
                backup_path: Some(backup_path),
                backup_sha256: Some(backup_sha256),
            });
        }
    }

    // Step 7: Post-upgrade WAL checkpoint truncate, recovery schema invariants, and full integrity check.
    checkpoint_truncate_quiescent(connection, canonical_path, from_version, to_version)?;
    validate_recovery_schema_invariants(connection, to_version).map_err(|err| MigrateError {
        code: MigrateErrorCode::DatabaseInvalid,
        message: format!("post-upgrade Schema {to_version} invariants failed: {err}"),
        seen_version: Some(to_version),
        preflight: Some(preflight),
        backup_path: Some(backup_path.clone()),
        backup_sha256: Some(backup_sha256.clone()),
    })?;
    projection_generation::current_generation(connection).map_err(|err| MigrateError {
        code: MigrateErrorCode::DatabaseInvalid,
        message: format!(
            "post-upgrade Schema {to_version} projection generation validation failed: {err}"
        ),
        seen_version: Some(to_version),
        preflight: Some(preflight),
        backup_path: Some(backup_path.clone()),
        backup_sha256: Some(backup_sha256.clone()),
    })?;
    let integrity = IntegrityReport {
        physical: physical_section(connection, true),
        logical: logical_section(connection),
        semantic: semantic_section(connection),
    };
    let pending_erasures = collect_pending_erasures(connection).map_err(|err| MigrateError {
        code: MigrateErrorCode::DatabaseInvalid,
        message: format!("post-upgrade pending-erasure check failed: {err}"),
        seen_version: Some(to_version),
        preflight: Some(preflight),
        backup_path: Some(backup_path.clone()),
        backup_sha256: Some(backup_sha256.clone()),
    })?;

    Ok(MigrateReport {
        from_version,
        to_version,
        db_path: canonical_path.to_path_buf(),
        backup_path,
        backup_sha256_path,
        backup_sha256,
        preflight,
        integrity,
        pending_erasures,
    })
}

/// Offline, fail-closed upgrade of a quiescent Schema-37 database to Schema 38.
///
/// Implements the 7-step contract from `ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md`.
pub fn migrate_37_to_38(
    db_path: impl Into<PathBuf>,
    backup_dir: impl Into<PathBuf>,
) -> Result<MigrateReport, MigrateError> {
    let requested_path = db_path.into();
    let backup_dir_requested = backup_dir.into();
    let (canonical_path, _lock, preflight) = prepare_migration_environment(
        &requested_path,
        &backup_dir_requested,
        SCHEMA_37,
        SCHEMA_38,
    )?;

    let connection = Connection::open_with_flags(
        sqlite_uri(&canonical_path, "mode=rw"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
            | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|error| {
        map_open_error_for_migrate(
            map_open_sqlite_error(error, OpenStage::WalReplay),
            SCHEMA_37,
            SCHEMA_38,
        )
    })?;
    connection.busy_timeout(Duration::ZERO).map_err(|error| {
        map_open_error_for_migrate(
            map_open_sqlite_error(error, OpenStage::WalReplay),
            SCHEMA_37,
            SCHEMA_38,
        )
    })?;

    execute_upgrade_transaction_and_verify(
        &connection,
        &canonical_path,
        &backup_dir_requested,
        SCHEMA_37,
        SCHEMA_38,
        preflight,
        |conn| {
            conn.execute_batch(fathomdb_schema::STEP_38_SQL)
                .map_err(|err| format!("Schema 38 DDL batch failed: {err}"))?;
            projection_registry::backfill_canonical_property_values_for_schema_38(conn).map_err(
                |err| format!("failed to backfill canonical_property_values for Schema 38: {err}"),
            )?;
            #[cfg(any(test, feature = "migration-test-hooks"))]
            fire_migrate_before_version_stamp_hook(&canonical_path, SCHEMA_38)?;
            conn.pragma_update(None, "user_version", SCHEMA_38)
                .map_err(|err| format!("failed to stamp PRAGMA user_version = 38: {err}"))?;
            projection_generation::rehash_serving_declaration_for_schema_38(conn).map_err(
                |err| format!("failed to refresh Schema 38 serving declaration digest: {err}"),
            )?;
            Ok(())
        },
    )
}

/// Offline, fail-closed upgrade of a quiescent Schema-38 database to Schema 39 (`S39-COMPOSITE`).
///
/// Reuses the fail-closed preflight, `.lock` quiescence, SHA-256-verified backup
/// (`{stem}.schema38.{utc}-{pid}.sqlite`), single-`BEGIN IMMEDIATE` transaction
/// (`STEP_39_SQL` + non-destructive 7-to-8-column `vec0` reshape if `vector_default`
/// exists), post-upgrade Schema-39 invariant checks, and open admission.
pub fn migrate_38_to_39(
    db_path: impl Into<PathBuf>,
    backup_dir: impl Into<PathBuf>,
) -> Result<MigrateReport, MigrateError> {
    let requested_path = db_path.into();
    let backup_dir_requested = backup_dir.into();
    let (canonical_path, _lock, preflight) = prepare_migration_environment(
        &requested_path,
        &backup_dir_requested,
        SCHEMA_38,
        SCHEMA_39,
    )?;

    let connection = Connection::open_with_flags(
        sqlite_uri(&canonical_path, "mode=rw"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
            | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|error| {
        map_open_error_for_migrate(
            map_open_sqlite_error(error, OpenStage::WalReplay),
            SCHEMA_38,
            SCHEMA_39,
        )
    })?;
    connection.busy_timeout(Duration::ZERO).map_err(|error| {
        map_open_error_for_migrate(
            map_open_sqlite_error(error, OpenStage::WalReplay),
            SCHEMA_38,
            SCHEMA_39,
        )
    })?;

    let report = execute_upgrade_transaction_and_verify(
        &connection,
        &canonical_path,
        &backup_dir_requested,
        SCHEMA_38,
        SCHEMA_39,
        preflight,
        |conn| {
            conn.execute_batch(fathomdb_schema::STEP_39_SQL)
                .map_err(|err| format!("Schema 39 DDL batch failed: {err}"))?;
            if let Some(dim) = crate::vector_storage::vector_partition_dimension(conn)
                .map_err(|err| format!("failed to inspect Schema 38 vector_default shape: {err}"))?
            {
                crate::vector_storage::reconcile_vector_attr_columns(conn, dim).map_err(|err| {
                    format!("failed to upgrade vector_default to 8-column Schema 39 layout: {err}")
                })?;
            }
            #[cfg(any(test, feature = "migration-test-hooks"))]
            fire_migrate_before_version_stamp_hook(&canonical_path, SCHEMA_39)?;
            conn.pragma_update(None, "user_version", SCHEMA_39)
                .map_err(|err| format!("failed to stamp PRAGMA user_version = 39: {err}"))?;
            Ok(())
        },
    )?;
    drop(connection);

    // Verify upgraded DB passes Engine::open's Schema-39 admission while still holding `_lock`.
    admit_current_database(&canonical_path).map_err(|err| MigrateError {
        code: MigrateErrorCode::DatabaseInvalid,
        message: format!("post-upgrade Schema 39 admission failed: {err}"),
        seen_version: Some(SCHEMA_39),
        preflight: Some(report.preflight),
        backup_path: Some(report.backup_path.clone()),
        backup_sha256: Some(report.backup_sha256.clone()),
    })?;

    Ok(report)
}

fn precheck_backup_dir(backup_dir: &Path, canonical_db: &Path) -> Result<(), MigrateError> {
    if backup_dir.as_os_str().is_empty() {
        return Err(MigrateError::simple(
            MigrateErrorCode::BackupDirInvalid,
            "backup directory path must not be empty",
        ));
    }
    if let Ok(meta) = std::fs::metadata(backup_dir) {
        if !meta.is_dir() {
            return Err(MigrateError::simple(
                MigrateErrorCode::BackupDirInvalid,
                format!("backup path '{}' exists and is not a directory", backup_dir.display()),
            ));
        }
        if let Ok(canon_dir) = backup_dir.canonicalize() {
            if canon_dir == canonical_db {
                return Err(MigrateError::simple(
                    MigrateErrorCode::BackupDirInvalid,
                    "backup directory must not equal the live database path",
                ));
            }
        }
    }
    Ok(())
}

fn run_readonly_preflights(
    canonical_path: &Path,
    from_version: u32,
    to_version: u32,
) -> Result<MigratePreflightReport, MigrateError> {
    // Immutable probe on main database file first.
    {
        let immutable = Connection::open_with_flags(
            immutable_sqlite_uri(canonical_path),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
                | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )
        .map_err(|error| {
            map_open_error_for_migrate(
                map_open_sqlite_error(error, OpenStage::HeaderProbe),
                from_version,
                to_version,
            )
        })?;
        immutable.pragma_update(None, "query_only", "ON").map_err(|error| {
            map_open_error_for_migrate(
                map_open_sqlite_error(error, OpenStage::SchemaProbe),
                from_version,
                to_version,
            )
        })?;
        probe_database_header(&immutable)
            .map_err(|err| map_open_error_for_migrate(err, from_version, to_version))?;
        probe_open_integrity(&immutable)
            .map_err(|err| map_open_error_for_migrate(err, from_version, to_version))?;
        reject_legacy_shape(&immutable)
            .map_err(|err| map_open_error_for_migrate(err, from_version, to_version))?;
    }

    // Refuse malformed or short non-empty WAL sidecar.
    let wal_header = classify_wal_sidecar(canonical_path)
        .map_err(|err| map_open_error_for_migrate(err, from_version, to_version))?;
    let malformed_wal = matches!(wal_header, WalSidecarHeader::Malformed { .. });
    let wal_short_nonempty =
        match std::fs::metadata(data_plane_sidecar_path(canonical_path, "-wal")) {
            Ok(metadata) => metadata.len() > 0 && metadata.len() < 32,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(_) => {
                return Err(MigrateError::simple(
                    MigrateErrorCode::DatabaseInvalid,
                    "database WAL sidecar is not accessible",
                ));
            }
        };
    if malformed_wal || wal_short_nonempty {
        return Err(MigrateError::simple(
            MigrateErrorCode::DatabaseInvalid,
            "malformed WAL sidecar blocks migration; run `fathomdb recover` first",
        ));
    }

    // WAL-aware read-only snapshot for strict schema admission, erasure debt, and edge preflight.
    let ro = Connection::open_with_flags(
        read_only_sqlite_uri(canonical_path),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
            | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|error| {
        map_open_error_for_migrate(
            map_open_sqlite_error(error, OpenStage::WalReplay),
            from_version,
            to_version,
        )
    })?;
    ro.pragma_update(None, "query_only", "ON").map_err(|error| {
        map_open_error_for_migrate(
            map_open_sqlite_error(error, OpenStage::SchemaProbe),
            from_version,
            to_version,
        )
    })?;
    probe_database_header(&ro)
        .map_err(|err| map_open_error_for_migrate(err, from_version, to_version))?;
    probe_open_integrity(&ro)
        .map_err(|err| map_open_error_for_migrate(err, from_version, to_version))?;
    reject_legacy_shape(&ro)
        .map_err(|err| map_open_error_for_migrate(err, from_version, to_version))?;

    let seen = ro.pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0)).map_err(
        |error| {
            map_open_error_for_migrate(
                map_open_sqlite_error(error, OpenStage::SchemaProbe),
                from_version,
                to_version,
            )
        },
    )?;
    if seen == to_version {
        return Err(MigrateError {
            code: MigrateErrorCode::AlreadyCurrent,
            message: format!("database is already at Schema {seen}"),
            seen_version: Some(seen),
            preflight: None,
            backup_path: None,
            backup_sha256: None,
        });
    }
    if seen != from_version {
        return Err(MigrateError {
            code: MigrateErrorCode::UnsupportedVersion,
            message: format!(
                "offline migrate supports only Schema {from_version} -> {to_version}; found user_version {seen}"
            ),
            seen_version: Some(seen),
            preflight: None,
            backup_path: None,
            backup_sha256: None,
        });
    }

    validate_recovery_schema_invariants(&ro, from_version)
        .map_err(|err| map_open_error_for_migrate(err, from_version, to_version))?;
    projection_generation::current_generation(&ro).map_err(|err| {
        MigrateError::simple(
            MigrateErrorCode::DatabaseInvalid,
            format!(
                "pre-upgrade Schema {from_version} projection generation validation failed: {err}"
            ),
        )
    })?;

    // Step 3: Fail-closed unfinished-erasure preflight (all 4 gates).
    check_no_unfinished_erasures(&ro, from_version)?;

    // Step 4: Fail-closed read-only edge endpoint preflight (`dangling` vs `superseded`).
    let preflight = count_edge_endpoint_preflight(&ro)?;
    if preflight.dangling_edge_endpoints > 0 {
        return Err(MigrateError {
            code: MigrateErrorCode::DanglingEdges,
            message: format!(
                "refusing upgrade: found {} genuinely dangling active edge endpoint(s) (and {} superseded-only endpoint(s)); resolve dangling edges before migrating to Schema {to_version}",
                preflight.dangling_edge_endpoints, preflight.superseded_edge_endpoints
            ),
            seen_version: Some(from_version),
            preflight: Some(preflight),
            backup_path: None,
            backup_sha256: None,
        });
    }

    Ok(preflight)
}

fn check_no_unfinished_erasures(conn: &Connection, from_version: u32) -> Result<(), MigrateError> {
    let pending_closures = dependency_closure::pending_physical_closures(conn).map_err(|_| {
        MigrateError::simple(
            MigrateErrorCode::DatabaseInvalid,
            "could not inspect pending physical dependency closures",
        )
    })?;
    let physical_debt = physical_erasure::diagnostics(conn).map_err(|_| {
        MigrateError::simple(
            MigrateErrorCode::DatabaseInvalid,
            "could not inspect physical erasure diagnostics",
        )
    })?;
    let active_requests: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM _fathomdb_erasure_requests \
             WHERE phase IN ('queued', 'executing', 'sanitizing', 'finalizing')",
            [],
            |row| row.get(0),
        )
        .map_err(|_| {
            MigrateError::simple(
                MigrateErrorCode::DatabaseInvalid,
                "could not inspect _fathomdb_erasure_requests",
            )
        })?;
    let pending_redactions: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM operational_mutations WHERE collection_name = ?1",
            [erasure::ERASURE_PENDING_REDACTION_COLLECTION],
            |row| row.get(0),
        )
        .map_err(|_| {
            MigrateError::simple(
                MigrateErrorCode::DatabaseInvalid,
                "could not inspect telemetry redaction obligations",
            )
        })?;

    if !pending_closures.is_empty()
        || !physical_debt.is_empty()
        || active_requests > 0
        || pending_redactions > 0
    {
        return Err(MigrateError {
            code: MigrateErrorCode::PendingErasure,
            message: format!(
                "refusing upgrade with unfinished erasure obligations (pending_closures={}, physical_debt={}, active_requests={}, pending_redactions={}); complete erasures via `fathomdb doctor check-integrity` / `fathomdb recover --accept-data-loss --complete-erasures` before running `migrate`",
                pending_closures.len(),
                physical_debt.len(),
                active_requests,
                pending_redactions
            ),
            seen_version: Some(from_version),
            preflight: None,
            backup_path: None,
            backup_sha256: None,
        });
    }
    Ok(())
}

fn collect_pending_erasures(conn: &Connection) -> Result<Vec<PendingErasure>, EngineError> {
    let queue_pending: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM operational_mutations WHERE collection_name=?1",
            [erasure::ERASURE_PENDING_REDACTION_COLLECTION],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    let mut rows = dependency_closure::pending_physical_closures(conn)?
        .into_iter()
        .map(|pending| PendingErasure {
            closure_id: pending.id.as_str().to_owned(),
            cause: pending.cause,
            phase: pending.phase,
            blocker: if queue_pending > 0 {
                Some("telemetry_redaction".to_string())
            } else {
                pending.blocker
            },
            sequence: pending.sequence,
        })
        .collect::<Vec<_>>();
    rows.extend(physical_erasure::diagnostics(conn)?);
    if queue_pending > 0 {
        for row in &mut rows {
            row.blocker = Some("telemetry_redaction".into());
        }
    }
    Ok(rows)
}

fn count_edge_endpoint_preflight(
    conn: &Connection,
) -> Result<MigratePreflightReport, MigrateError> {
    let (dangling, superseded): (i64, i64) = conn
        .query_row(
            "SELECT
                COALESCE(SUM(CASE
                    WHEN NOT EXISTS (
                        SELECT 1 FROM canonical_nodes n WHERE n.logical_id = ep.endpoint_id
                    ) THEN 1 ELSE 0
                END), 0) AS dangling_count,
                COALESCE(SUM(CASE
                    WHEN EXISTS (
                        SELECT 1 FROM canonical_nodes n WHERE n.logical_id = ep.endpoint_id
                    ) AND NOT EXISTS (
                        SELECT 1 FROM canonical_nodes n
                        WHERE n.logical_id = ep.endpoint_id AND n.superseded_at IS NULL
                    ) THEN 1 ELSE 0
                END), 0) AS superseded_count
             FROM (
                SELECT from_id AS endpoint_id FROM canonical_edges WHERE superseded_at IS NULL
                UNION ALL
                SELECT to_id AS endpoint_id FROM canonical_edges WHERE superseded_at IS NULL
             ) ep",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|err| {
            MigrateError::simple(
                MigrateErrorCode::DatabaseInvalid,
                format!("failed to run read-only edge endpoint preflight: {err}"),
            )
        })?;
    Ok(MigratePreflightReport {
        dangling_edge_endpoints: u64::try_from(dangling).unwrap_or(0),
        superseded_edge_endpoints: u64::try_from(superseded).unwrap_or(0),
    })
}

fn checkpoint_truncate_quiescent(
    conn: &Connection,
    db_path: &Path,
    from_version: u32,
    to_version: u32,
) -> Result<(), MigrateError> {
    let (busy, _log_frames, _checkpointed_frames): (i64, i64, i64) = conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|error| {
            map_open_error_for_migrate(
                map_open_sqlite_error(error, OpenStage::WalReplay),
                from_version,
                to_version,
            )
        })?;
    if busy != 0 {
        return Err(MigrateError::simple(
            MigrateErrorCode::LockHeld,
            format!("WAL checkpoint(TRUNCATE) reported busy={busy}; database is not quiescent"),
        ));
    }
    match std::fs::metadata(data_plane_sidecar_path(db_path, "-wal")) {
        Ok(meta) if meta.len() > 0 => {
            return Err(MigrateError::simple(
                MigrateErrorCode::LockHeld,
                "WAL sidecar remains non-empty after TRUNCATE checkpoint",
            ));
        }
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            return Err(MigrateError::simple(
                MigrateErrorCode::DatabaseInvalid,
                format!("could not verify WAL sidecar after checkpoint: {err}"),
            ));
        }
    }
    Ok(())
}

fn write_verified_backup(
    canonical_db: &Path,
    backup_dir_requested: &Path,
    from_version: u32,
) -> Result<(PathBuf, PathBuf, String), MigrateError> {
    std::fs::create_dir_all(backup_dir_requested).map_err(|err| {
        MigrateError::simple(
            MigrateErrorCode::BackupDirInvalid,
            format!(
                "failed to create backup directory '{}': {err}",
                backup_dir_requested.display()
            ),
        )
    })?;
    let backup_dir = backup_dir_requested.canonicalize().map_err(|err| {
        MigrateError::simple(
            MigrateErrorCode::BackupDirInvalid,
            format!(
                "backup directory '{}' is not accessible: {err}",
                backup_dir_requested.display()
            ),
        )
    })?;
    if !backup_dir.is_dir() {
        return Err(MigrateError::simple(
            MigrateErrorCode::BackupDirInvalid,
            format!("backup path '{}' is not a directory", backup_dir.display()),
        ));
    }

    let source_sha256 = sha256_path(canonical_db).map_err(|err| {
        MigrateError::simple(
            MigrateErrorCode::DatabaseInvalid,
            format!("failed to hash source database before backup: {err}"),
        )
    })?;

    let stem = canonical_db
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("database");
    let utc = format_utc_compact(SystemTime::now());
    let pid = std::process::id();

    let (backup_path, mut backup_file) =
        create_unique_backup_file(&backup_dir, canonical_db, stem, from_version, &utc, pid)?;

    let copy_result: std::io::Result<()> = (|| {
        let mut src = File::open(canonical_db)?;
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = src.read(&mut buf)?;
            if n == 0 {
                break;
            }
            backup_file.write_all(&buf[..n])?;
        }
        backup_file.sync_all()?;
        Ok(())
    })();
    drop(backup_file);
    if let Err(err) = copy_result {
        let _ = std::fs::remove_file(&backup_path);
        return Err(MigrateError::simple(
            MigrateErrorCode::BackupDirInvalid,
            format!("failed to write pre-upgrade backup '{}': {err}", backup_path.display()),
        ));
    }

    let backup_sha256 = sha256_path(&backup_path).map_err(|err| {
        let _ = std::fs::remove_file(&backup_path);
        MigrateError::simple(
            MigrateErrorCode::BackupDirInvalid,
            format!("failed to re-read backup for SHA-256 verification: {err}"),
        )
    })?;
    if backup_sha256 != source_sha256 {
        let _ = std::fs::remove_file(&backup_path);
        return Err(MigrateError::simple(
            MigrateErrorCode::BackupDirInvalid,
            format!(
                "backup SHA-256 verification mismatch (source={source_sha256}, backup={backup_sha256})"
            ),
        ));
    }

    let mut sha_os = backup_path.as_os_str().to_os_string();
    sha_os.push(".sha256");
    let backup_sha256_path = PathBuf::from(sha_os);
    let backup_name = backup_path.file_name().and_then(|n| n.to_str()).unwrap_or("backup.sqlite");
    let receipt = format!("{backup_sha256}  {backup_name}\n");
    let mut sha_file =
        OpenOptions::new().write(true).create_new(true).open(&backup_sha256_path).map_err(
            |err| {
                MigrateError::simple(
                    MigrateErrorCode::BackupDirInvalid,
                    format!(
                        "failed to create SHA-256 receipt '{}': {err}",
                        backup_sha256_path.display()
                    ),
                )
            },
        )?;
    sha_file.write_all(receipt.as_bytes()).and_then(|()| sha_file.sync_all()).map_err(|err| {
        MigrateError::simple(
            MigrateErrorCode::BackupDirInvalid,
            format!("failed to write SHA-256 receipt '{}': {err}", backup_sha256_path.display()),
        )
    })?;
    drop(sha_file);

    #[cfg(unix)]
    if let Ok(dir_handle) = File::open(&backup_dir) {
        let _ = dir_handle.sync_all();
    }

    Ok((backup_path, backup_sha256_path, backup_sha256))
}

fn create_unique_backup_file(
    backup_dir: &Path,
    canonical_db: &Path,
    stem: &str,
    from_version: u32,
    utc: &str,
    pid: u32,
) -> Result<(PathBuf, File), MigrateError> {
    for attempt in 0u32..1_000 {
        let filename = if attempt == 0 {
            format!("{stem}.schema{from_version}.{utc}-{pid}.sqlite")
        } else {
            format!("{stem}.schema{from_version}.{utc}-{pid}-{attempt}.sqlite")
        };
        let candidate = backup_dir.join(&filename);
        if candidate == canonical_db {
            return Err(MigrateError::simple(
                MigrateErrorCode::BackupDirInvalid,
                "computed backup path collides with the live database path",
            ));
        }
        match OpenOptions::new().write(true).create_new(true).open(&candidate) {
            Ok(file) => return Ok((candidate, file)),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => {
                return Err(MigrateError::simple(
                    MigrateErrorCode::BackupDirInvalid,
                    format!("failed to create backup file '{}': {err}", candidate.display()),
                ));
            }
        }
    }
    Err(MigrateError::simple(
        MigrateErrorCode::BackupDirInvalid,
        "could not allocate a unique backup filename in backup directory",
    ))
}

fn sha256_path(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = sha2::Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex_encode(hasher.finalize().as_slice()))
}

/// Format a [`SystemTime`] as compact UTC `YYYYMMDDTHHMMSSZ` (Howard Hinnant civil-from-days).
fn format_utc_compact(now: SystemTime) -> String {
    let secs = now.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let hour = rem / 3_600;
    let minute = (rem % 3_600) / 60;
    let second = rem % 60;

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + i64::from(month <= 2);

    format!("{year:04}{month:02}{day:02}T{hour:02}{minute:02}{second:02}Z")
}

fn map_open_error_for_migrate(
    err: EngineOpenError,
    from_version: u32,
    to_version: u32,
) -> MigrateError {
    match err {
        EngineOpenError::DatabaseLocked { holder_pid } => MigrateError::simple(
            MigrateErrorCode::LockHeld,
            match holder_pid {
                Some(pid) => format!("database lock is held by pid {pid}"),
                None => "database lock is held by another process".to_string(),
            },
        ),
        EngineOpenError::IncompatibleSchemaVersion { seen, .. } if seen == to_version => {
            MigrateError {
                code: MigrateErrorCode::AlreadyCurrent,
                message: format!("database is already at Schema {to_version}"),
                seen_version: Some(seen),
                preflight: None,
                backup_path: None,
                backup_sha256: None,
            }
        }
        EngineOpenError::IncompatibleSchemaVersion { seen, .. } => MigrateError {
            code: MigrateErrorCode::UnsupportedVersion,
            message: format!(
                "offline migrate supports only Schema {from_version} -> {to_version}; found user_version {seen}"
            ),
            seen_version: Some(seen),
            preflight: None,
            backup_path: None,
            backup_sha256: None,
        },
        other => MigrateError::simple(MigrateErrorCode::DatabaseInvalid, other.to_string()),
    }
}
