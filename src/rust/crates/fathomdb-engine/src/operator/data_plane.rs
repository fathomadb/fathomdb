use super::*;
use crate::open::{read_only_sqlite_uri, sqlite_uri};

#[cfg(feature = "operator")]
fn data_plane_inspection_error(
    reason: DataPlaneIntegrityErrorReasonV1,
    field_path: &'static str,
) -> EngineError {
    DataPlaneIntegrityErrorV1::new(reason, field_path).into()
}

#[cfg(feature = "operator")]
fn data_plane_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut sidecar = path.as_os_str().to_os_string();
    sidecar.push(suffix);
    PathBuf::from(sidecar)
}

#[cfg(feature = "operator")]
fn immutable_sqlite_uri(path: &Path) -> String {
    sqlite_uri(path, "immutable=1")
}

/// Recover a current, quiescent database by asking SQLite to truncate its WAL.
///
/// Unlike [`Engine::open`], this operator-only path may proceed when the WAL's
/// fixed header is malformed. The main database is first validated through an
/// immutable, read-only connection, and a non-empty rollback journal is always
/// refused. The canonical product lock is held across validation and the
/// checkpoint, so a live FathomDB process cannot race the recovery. SQLite owns
/// the destructive WAL checkpoint/discard. A healthy-WAL preflight snapshots
/// the transient SHM sidecar and restores it if validation refuses recovery.
///
/// `discarded_corrupt_wal` is true only when the locked pre-probe classified
/// the WAL header as malformed and SQLite subsequently reported a completed
/// truncate checkpoint. A busy checkpoint remains a successful typed report
/// with [`TruncateWalStatus::Busy`] and never claims that corrupt data was
/// discarded.
///
/// # Errors
///
/// Returns [`EngineOpenError`] when the database is missing, empty, locked,
/// corrupt, not at the current schema version, accompanied by a non-empty
/// rollback journal, or inaccessible to SQLite.
#[cfg(feature = "operator")]
pub fn recover_truncate_wal(
    path: impl Into<PathBuf>,
) -> Result<TruncateWalReport, EngineOpenError> {
    let requested_path = path.into();
    let canonical_path = canonical_database_path(&requested_path)?;
    validate_recovery_database_file(&canonical_path)?;

    let pending_lock = acquire_lock_without_metadata_mutation(&canonical_path)?;
    let _lock = pending_lock.initialize()?;

    // Recheck every admission fact after the lock is held. The first check
    // prevents bootstrap; this one closes the rename/truncate race.
    validate_recovery_database_file(&canonical_path)?;
    match std::fs::metadata(data_plane_sidecar_path(&canonical_path, "-journal")) {
        Ok(metadata) if metadata.len() > 0 => {
            return Err(EngineOpenError::Io {
                message: "non-empty rollback journal blocks WAL recovery".to_string(),
            })
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(EngineOpenError::Io {
                message: "database rollback journal is not accessible".to_string(),
            })
        }
    }

    configure_runtime_for_open().map_err(EngineOpenError::RuntimeConfiguration)?;
    register_sqlite_vec_extension();

    let validation = Connection::open_with_flags(
        immutable_sqlite_uri(&canonical_path),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
            | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|error| map_open_sqlite_error(error, OpenStage::HeaderProbe))?;
    validation
        .pragma_update(None, "query_only", "ON")
        .map_err(|error| map_open_sqlite_error(error, OpenStage::SchemaProbe))?;
    probe_database_header(&validation)?;
    probe_open_integrity(&validation)?;
    reject_legacy_shape(&validation)?;
    let main_file_schema_version = validation
        .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
        .map_err(|error| map_open_sqlite_error(error, OpenStage::SchemaProbe))?;

    let wal_header = classify_wal_sidecar(&canonical_path)?;
    let malformed_wal = matches!(wal_header, WalSidecarHeader::Malformed { .. });
    // A malformed WAL cannot contribute trustworthy schema state. Require the
    // standalone main file itself to be current before asking SQLite to discard
    // that WAL. A healthy WAL may legitimately carry the current schema cookie
    // while the main file still reports an older value, so its effective version
    // is checked through SQLite below.
    if malformed_wal && main_file_schema_version != SCHEMA_VERSION {
        return Err(EngineOpenError::IncompatibleSchemaVersion {
            seen: main_file_schema_version,
            supported: SCHEMA_VERSION,
        });
    }
    if main_file_schema_version == SCHEMA_VERSION {
        validate_recovery_schema_invariants(&validation, main_file_schema_version)?;
    }
    drop(validation);
    if !malformed_wal {
        validate_effective_recovery_schema(&canonical_path)?;
    }

    // No read/write SQLite connection is opened until every refusal condition
    // has passed. In particular, dropping a read/write connection after a
    // noncurrent effective-schema check could checkpoint a healthy WAL while
    // reporting refusal.
    let connection = Connection::open_with_flags(
        sqlite_uri(&canonical_path, "mode=rw"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
            | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|error| map_open_sqlite_error(error, OpenStage::WalReplay))?;
    connection
        .busy_timeout(Duration::ZERO)
        .map_err(|error| map_open_sqlite_error(error, OpenStage::WalReplay))?;
    let (busy, log_frames, checkpointed_frames): (i64, i64, i64) = connection
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|error| map_open_sqlite_error(error, OpenStage::WalReplay))?;
    let status = if busy == 0 { TruncateWalStatus::Done } else { TruncateWalStatus::Busy };

    Ok(TruncateWalReport {
        status,
        busy: busy.max(0) as u32,
        log_frames: log_frames.max(0) as u32,
        checkpointed_frames: checkpointed_frames.max(0) as u32,
        discarded_corrupt_wal: malformed_wal && status == TruncateWalStatus::Done,
    })
}

#[cfg(feature = "operator")]
fn validate_recovery_database_file(path: &Path) -> Result<(), EngineOpenError> {
    let metadata = std::fs::metadata(path).map_err(|_| EngineOpenError::Io {
        message: "recovery requires an existing database file".to_string(),
    })?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err(EngineOpenError::Io {
            message: "recovery requires a non-empty regular database file".to_string(),
        });
    }
    Ok(())
}

#[cfg(feature = "operator")]
fn validate_effective_recovery_schema(path: &Path) -> Result<(), EngineOpenError> {
    let shm = ShmSnapshot::capture(path)?;
    let result = (|| {
        let connection = Connection::open_with_flags(
            read_only_sqlite_uri(path),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
                | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )
        .map_err(|error| map_open_sqlite_error(error, OpenStage::WalReplay))?;
        connection
            .pragma_update(None, "query_only", "ON")
            .map_err(|error| map_open_sqlite_error(error, OpenStage::SchemaProbe))?;
        probe_database_header(&connection)?;
        probe_open_integrity(&connection)?;
        reject_legacy_shape(&connection)?;
        let seen = connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .map_err(|error| map_open_sqlite_error(error, OpenStage::SchemaProbe))?;
        if seen != SCHEMA_VERSION {
            return Err(EngineOpenError::IncompatibleSchemaVersion {
                seen,
                supported: SCHEMA_VERSION,
            });
        }
        validate_recovery_schema_invariants(&connection, seen)
    })();
    match result {
        Ok(()) => Ok(()),
        Err(error) => {
            shm.restore()?;
            Err(error)
        }
    }
}

#[cfg(feature = "operator")]
fn validate_recovery_schema_invariants(
    connection: &Connection,
    schema_version: u32,
) -> Result<(), EngineOpenError> {
    validate_dependency_generation_on_open(connection, schema_version)?;
    frozen_read::validate_on_open(connection, schema_version)
        .map_err(|_| recovery_schema_corruption("_fathomdb_read_visibility_state"))?;
    dependency_closure::validate_closure_state_on_open(connection, schema_version)?;
    Ok(())
}

#[cfg(feature = "operator")]
fn recovery_schema_corruption(table: &'static str) -> EngineOpenError {
    EngineOpenError::Corruption(CorruptionDetail {
        kind: CorruptionKind::SchemaInconsistent,
        stage: OpenStage::SchemaProbe,
        locator: CorruptionLocator::TableRow { table, rowid: 0 },
        recovery_hint: RecoveryHint {
            code: "E_CORRUPT_SCHEMA",
            doc_anchor: "design/recovery.md#schema-inconsistent",
        },
    })
}

/// Inspect a quiescent database through a strictly read-only operator boundary.
///
/// The request is validated before filesystem access. The database and its
/// pre-existing lock file must both exist, the lock must be exclusively
/// acquirable without modifying it, and non-empty WAL or rollback-journal
/// sidecars are refused. SQLite is then opened read-only with `query_only`
/// enabled; migrations, projection reconciliation, worker startup, and lock
/// metadata writes are never performed.
///
/// FathomDB writers are excluded by the product lock. Callers must also stop
/// raw external SQLite writers, which do not participate in that lock protocol,
/// before invoking this function. The connection uses a percent-encoded
/// `file:` URI with SQLite `immutable=1`, `READ_ONLY|URI`, and `query_only`.
///
/// # Errors
///
/// Returns a typed [`DataPlaneIntegrityErrorV1`] through [`EngineError`] for
/// invalid requests, unavailable or non-quiescent inputs, runtime setup,
/// incompatible schema versions, corruption, or bounded inspection failures.
#[cfg(feature = "operator")]
pub fn inspect_data_plane_integrity(
    path: impl Into<PathBuf>,
    request: DataPlaneIntegrityRequestV1,
) -> Result<DataPlaneIntegrityResultV1, EngineError> {
    data_plane_integrity::validate_request(&request)?;

    let requested_path = path.into();
    let unresolved_path = canonical_database_path(&requested_path).map_err(|_| {
        data_plane_inspection_error(
            DataPlaneIntegrityErrorReasonV1::InspectionUnavailable,
            "/dbPath",
        )
    })?;
    let canonical_path = unresolved_path.canonicalize().map_err(|_| {
        data_plane_inspection_error(
            DataPlaneIntegrityErrorReasonV1::InspectionUnavailable,
            "/dbPath",
        )
    })?;
    if !canonical_path.is_file() {
        return Err(data_plane_inspection_error(
            DataPlaneIntegrityErrorReasonV1::InspectionUnavailable,
            "/dbPath",
        ));
    }

    let inspection_lock_path = lock_path(&canonical_path);
    if !inspection_lock_path.is_file() {
        return Err(data_plane_inspection_error(
            DataPlaneIntegrityErrorReasonV1::InspectionLockMissing,
            "/dbPath",
        ));
    }
    let inspection_lock =
        OpenOptions::new().read(true).open(&inspection_lock_path).map_err(|_| {
            data_plane_inspection_error(
                DataPlaneIntegrityErrorReasonV1::InspectionUnavailable,
                "/dbPath",
            )
        })?;
    match inspection_lock.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => {
            return Err(data_plane_inspection_error(
                DataPlaneIntegrityErrorReasonV1::InspectionNotQuiescent,
                "/dbPath",
            ));
        }
        Err(_) => {
            return Err(data_plane_inspection_error(
                DataPlaneIntegrityErrorReasonV1::InspectionUnavailable,
                "/dbPath",
            ));
        }
    }

    for suffix in ["-wal", "-journal"] {
        match std::fs::metadata(data_plane_sidecar_path(&canonical_path, suffix)) {
            Ok(metadata) if metadata.len() > 0 => {
                return Err(data_plane_inspection_error(
                    DataPlaneIntegrityErrorReasonV1::InspectionNotQuiescent,
                    "/dbPath",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                return Err(data_plane_inspection_error(
                    DataPlaneIntegrityErrorReasonV1::InspectionUnavailable,
                    "/dbPath",
                ));
            }
        }
    }

    configure_runtime_for_open().map_err(|_| {
        data_plane_inspection_error(
            DataPlaneIntegrityErrorReasonV1::RuntimeConfiguration,
            "/runtimeConfiguration",
        )
    })?;
    register_sqlite_vec_extension();
    let mut connection = Connection::open_with_flags(
        immutable_sqlite_uri(&canonical_path),
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|_| {
        data_plane_inspection_error(
            DataPlaneIntegrityErrorReasonV1::InspectionUnavailable,
            "/dbPath",
        )
    })?;
    connection.pragma_update(None, "query_only", "ON").map_err(|_| {
        data_plane_inspection_error(DataPlaneIntegrityErrorReasonV1::IntegrityCorrupt, "")
    })?;
    let database_schema_version =
        connection.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0)).map_err(
            |_| data_plane_inspection_error(DataPlaneIntegrityErrorReasonV1::IntegrityCorrupt, ""),
        )?;
    if database_schema_version != i64::from(SCHEMA_VERSION) {
        return Err(data_plane_inspection_error(
            DataPlaneIntegrityErrorReasonV1::DatabaseSchemaMismatch,
            "/databaseSchemaVersion",
        ));
    }

    data_plane_integrity::execute(&mut connection, request).map_err(|error| match error {
        EngineError::DataPlaneIntegrity(_) => error,
        _ => data_plane_inspection_error(DataPlaneIntegrityErrorReasonV1::IntegrityCorrupt, ""),
    })
}
