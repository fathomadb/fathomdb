use super::*;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DatabaseAdmission {
    CurrentOnly,
    #[cfg(feature = "migration-test-hooks")]
    TestMigrations,
}

pub(crate) struct DatabaseOpenPlan {
    pub(crate) migrations: &'static [fathomdb_schema::Migration],
    pub(crate) admission: DatabaseAdmission,
    pub(crate) config: EngineConfig,
}

pub(crate) struct ShmSnapshot {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
}

impl ShmSnapshot {
    pub(crate) fn capture(database_path: &Path) -> Result<Self, EngineOpenError> {
        let mut shm_path = database_path.as_os_str().to_os_string();
        shm_path.push("-shm");
        let path = PathBuf::from(shm_path);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => {
                return Err(EngineOpenError::Io {
                    message: "database shared-memory sidecar is not accessible".to_string(),
                })
            }
        };
        Ok(Self { path, bytes })
    }

    pub(crate) fn restore(self) -> Result<(), EngineOpenError> {
        match self.bytes {
            Some(bytes) => std::fs::write(self.path, bytes).map_err(|_| EngineOpenError::Io {
                message: "database shared-memory sidecar could not be restored".to_string(),
            }),
            None => match std::fs::remove_file(self.path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(_) => Err(EngineOpenError::Io {
                    message: "temporary database shared-memory sidecar could not be removed"
                        .to_string(),
                }),
            },
        }
    }
}

fn read_effective_schema_version(path: &Path) -> Result<u32, EngineOpenError> {
    probe_wal_sidecar(path)?;
    let connection = Connection::open_with_flags(
        read_only_sqlite_uri(path),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
            | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|error| map_open_sqlite_error(error, OpenStage::HeaderProbe))?;
    connection
        .pragma_update(None, "query_only", "ON")
        .map_err(|error| map_open_sqlite_error(error, OpenStage::SchemaProbe))?;
    probe_database_header(&connection)?;
    probe_open_integrity(&connection)?;
    connection
        .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
        .map_err(|error| map_open_sqlite_error(error, OpenStage::SchemaProbe))
}

pub(crate) fn admit_current_database(path: &Path) -> Result<(), EngineOpenError> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => {
            return Err(EngineOpenError::Io {
                message: "database candidate metadata is not accessible".to_string(),
            })
        }
    };
    if metadata.len() == 0 {
        return Ok(());
    }
    let shm = ShmSnapshot::capture(path)?;
    match read_effective_schema_version(path) {
        Ok(seen) if seen == SCHEMA_VERSION => Ok(()),
        Ok(seen) => {
            shm.restore()?;
            Err(EngineOpenError::IncompatibleSchemaVersion { seen, supported: SCHEMA_VERSION })
        }
        Err(error) => {
            shm.restore()?;
            Err(error)
        }
    }
}

pub(crate) fn canonical_database_path(path: &Path) -> Result<PathBuf, EngineOpenError> {
    match path.canonicalize() {
        Ok(canonical) => return Ok(canonical),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(EngineOpenError::Io {
                message: "database path is not accessible".to_string(),
            })
        }
    }
    // `canonicalize` reports NotFound for both a genuinely absent file and a
    // dangling final-component symlink. Only the former is a legal fresh-path
    // bootstrap; treating the latter as a filename would create a split lock
    // namespace beside the alias.
    match std::fs::symlink_metadata(path) {
        Ok(_) => {
            return Err(EngineOpenError::Io {
                message: "database path does not resolve to an accessible file".to_string(),
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(EngineOpenError::Io {
                message: "database path is not accessible".to_string(),
            })
        }
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let canonical_parent = parent.canonicalize().map_err(|_| EngineOpenError::Io {
        message: "database parent directory is not accessible".to_string(),
    })?;
    let file_name = path.file_name().ok_or_else(|| EngineOpenError::Io {
        message: "database path has no file name".to_string(),
    })?;

    Ok(canonical_parent.join(file_name))
}

pub(crate) struct PendingDatabaseLock {
    file: Option<File>,
}

impl PendingDatabaseLock {
    pub(crate) fn initialize(mut self) -> Result<File, EngineOpenError> {
        let file = self.file.as_mut().expect("pending database lock retains its file");
        let pid = std::process::id().to_string();
        file.set_len(0).map_err(|_| EngineOpenError::Io {
            message: "could not initialize database lock file".to_string(),
        })?;
        file.seek(SeekFrom::Start(0)).map_err(|_| EngineOpenError::Io {
            message: "could not initialize database lock file".to_string(),
        })?;
        file.write_all(pid.as_bytes()).map_err(|_| EngineOpenError::Io {
            message: "could not initialize database lock file".to_string(),
        })?;
        Ok(self.file.take().expect("initialized database lock retains its file"))
    }
}

pub(crate) fn acquire_lock_without_metadata_mutation(
    path: &Path,
) -> Result<PendingDatabaseLock, EngineOpenError> {
    let lock_path = lock_path(path);
    let mut create_options = OpenOptions::new();
    create_options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    create_options.mode(0o600);
    let file = match create_options.open(&lock_path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let mut existing_options = OpenOptions::new();
            existing_options.read(true).write(true);
            existing_options.open(&lock_path).map_err(|_| EngineOpenError::Io {
                message: "could not open database lock file".to_string(),
            })?
        }
        Err(_) => {
            return Err(EngineOpenError::Io {
                message: "could not open database lock file".to_string(),
            })
        }
    };
    let pending = PendingDatabaseLock { file: Some(file) };

    match pending.file.as_ref().expect("pending database lock retains its file").try_lock() {
        Ok(()) => Ok(pending),
        Err(std::fs::TryLockError::WouldBlock) => {
            Err(EngineOpenError::DatabaseLocked { holder_pid: read_holder_pid(&lock_path) })
        }
        Err(_) => {
            Err(EngineOpenError::Io { message: "could not acquire database lock".to_string() })
        }
    }
}

pub(crate) fn lock_path(path: &Path) -> PathBuf {
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(LOCK_SUFFIX);
    PathBuf::from(lock_path)
}

fn read_holder_pid(path: &Path) -> Option<u32> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}
