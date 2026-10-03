use super::*;
use crate::dependency_trace::{DEPENDENCY_GENERATION_KEY, SOURCE_DEPENDENCY_SCHEMA_VERSION};
use crate::errors::{map_migration_error, map_open_sqlite_error};
use crate::lifecycle::emit_open_error_event;
use crate::temporal::EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION;
use crate::vector_storage::DEFAULT_VECTOR_PROFILE;

struct OpenEmbedDispatchGuard(Option<Arc<EmbedDispatcher>>);

impl OpenEmbedDispatchGuard {
    fn disarm(&mut self) {
        self.0.take();
    }
}

impl Drop for OpenEmbedDispatchGuard {
    fn drop(&mut self) {
        if let Some(dispatch) = self.0.take() {
            dispatch.close();
            let _ = dispatch.join_after_quiescence();
        }
    }
}

struct OpenPostProbeGuard {
    connection: Option<Connection>,
    readers: Option<Vec<Connection>>,
    projection_runtime: Option<ProjectionRuntime>,
    #[allow(clippy::vec_box)]
    profile_contexts: Vec<Box<ProfileContext>>,
    lock: Option<File>,
    #[cfg(any(test, feature = "test-hooks"))]
    writer_registration: Option<ManagedConnectionRegistration>,
    embed_dispatch: Arc<EmbedDispatcher>,
    #[cfg(test)]
    profile_release_observer: Option<Arc<ProfileReleaseObserver>>,
}

struct OpenPostProbeParts {
    connection: Connection,
    readers: Vec<Connection>,
    projection_runtime: ProjectionRuntime,
    profile_contexts: ProfileContexts,
    lock: File,
    #[cfg(any(test, feature = "test-hooks"))]
    writer_registration: ManagedConnectionRegistration,
}

impl OpenPostProbeGuard {
    fn new(
        connection: Connection,
        readers: Vec<Connection>,
        lock: File,
        embed_dispatch: Arc<EmbedDispatcher>,
    ) -> Self {
        Self {
            connection: Some(connection),
            readers: Some(readers),
            projection_runtime: None,
            profile_contexts: Vec::new(),
            lock: Some(lock),
            #[cfg(any(test, feature = "test-hooks"))]
            writer_registration: None,
            embed_dispatch,
            #[cfg(test)]
            profile_release_observer: None,
        }
    }

    fn connection(&self) -> &Connection {
        self.connection.as_ref().expect("open writer connection")
    }

    fn install_profiles(
        &mut self,
        subscribers: &Arc<lifecycle::SubscriberRegistry>,
        profiling_enabled: &Arc<AtomicBool>,
        slow_threshold_ms: &Arc<AtomicU64>,
    ) {
        install_profile_callback(
            self.connection.as_ref().expect("open writer connection"),
            subscribers,
            profiling_enabled,
            slow_threshold_ms,
            &mut self.profile_contexts,
        );
        for reader in self.readers.as_ref().expect("open readers") {
            install_profile_callback(
                reader,
                subscribers,
                profiling_enabled,
                slow_threshold_ms,
                &mut self.profile_contexts,
            );
        }
    }

    fn into_parts(mut self) -> OpenPostProbeParts {
        #[cfg(test)]
        let profile_contexts = ProfileContexts::from(std::mem::take(&mut self.profile_contexts));
        #[cfg(not(test))]
        let profile_contexts = std::mem::take(&mut self.profile_contexts);
        OpenPostProbeParts {
            connection: self.connection.take().expect("open writer connection"),
            readers: self.readers.take().expect("open readers"),
            projection_runtime: self.projection_runtime.take().expect("open projection runtime"),
            profile_contexts,
            lock: self.lock.take().expect("open admission lock"),
            #[cfg(any(test, feature = "test-hooks"))]
            writer_registration: self.writer_registration.take().expect("writer registration"),
        }
    }
}

impl Drop for OpenPostProbeGuard {
    fn drop(&mut self) {
        if self.connection.is_none() {
            return;
        }
        self.embed_dispatch.close();
        if let Some(runtime) = self.projection_runtime.take() {
            runtime.stop();
        }
        if let Some(readers) = self.readers.as_ref() {
            for reader in readers {
                uninstall_profile_callback(reader);
            }
        }
        self.readers.take();
        if let Some(connection) = self.connection.as_ref() {
            uninstall_profile_callback(connection);
        }
        self.connection.take();
        #[cfg(any(test, feature = "test-hooks"))]
        self.writer_registration.take();
        #[cfg(test)]
        if let Some(observer) = self.profile_release_observer.take() {
            let mut contexts = ProfileContexts {
                contexts: std::mem::take(&mut self.profile_contexts),
                observer: Some(observer),
            };
            contexts.clear();
        } else {
            self.profile_contexts.clear();
        }
        #[cfg(not(test))]
        self.profile_contexts.clear();
        self.lock.take();
    }
}

#[cfg(test)]
struct AdmissionLockedHookForTest {
    path: PathBuf,
    rendezvous: Arc<Barrier>,
}

#[cfg(test)]
static ADMISSION_LOCKED_HOOK_FOR_TEST: Mutex<Option<AdmissionLockedHookForTest>> = Mutex::new(None);

#[cfg(test)]
struct PostProbeStartupFaultForTest {
    path: PathBuf,
    observed: std::sync::mpsc::Sender<PostProbeStartupObservationForTest>,
}

#[cfg(test)]
pub(crate) struct PostProbeStartupObservationForTest {
    pub(crate) accounting: embed_dispatch::DispatchAccounting,
    pub(crate) registry: Arc<ManagedConnectionRegistry>,
    pub(crate) profiles_at_fault: usize,
    pub(crate) profile_releases: Arc<ProfileReleaseObserver>,
}

#[cfg(test)]
static POST_PROBE_STARTUP_FAULT_FOR_TEST: Mutex<Option<PostProbeStartupFaultForTest>> =
    Mutex::new(None);

#[cfg(test)]
struct PostProbeVisibilityFaultForTest {
    path: PathBuf,
    observed: std::sync::mpsc::Sender<(
        embed_dispatch::DispatchAccounting,
        Arc<ManagedConnectionRegistry>,
    )>,
}

#[cfg(test)]
static POST_PROBE_VISIBILITY_FAULT_FOR_TEST: Mutex<Option<PostProbeVisibilityFaultForTest>> =
    Mutex::new(None);

#[cfg(test)]
pub(crate) fn install_post_probe_visibility_fault_for_test(
    path: PathBuf,
    observed: std::sync::mpsc::Sender<(
        embed_dispatch::DispatchAccounting,
        Arc<ManagedConnectionRegistry>,
    )>,
) {
    *POST_PROBE_VISIBILITY_FAULT_FOR_TEST.lock().expect("visibility fault hook lock") =
        Some(PostProbeVisibilityFaultForTest { path, observed });
}

#[cfg(test)]
fn take_post_probe_visibility_fault_for_test(
    path: &Path,
) -> Option<PostProbeVisibilityFaultForTest> {
    let mut hook = POST_PROBE_VISIBILITY_FAULT_FOR_TEST.lock().expect("visibility fault hook lock");
    if hook.as_ref().is_some_and(|candidate| candidate.path == path) {
        hook.take()
    } else {
        None
    }
}

#[cfg(test)]
pub(crate) fn install_post_probe_startup_fault_for_test(
    path: PathBuf,
    observed: std::sync::mpsc::Sender<PostProbeStartupObservationForTest>,
) {
    *POST_PROBE_STARTUP_FAULT_FOR_TEST.lock().expect("startup fault hook lock") =
        Some(PostProbeStartupFaultForTest { path, observed });
}

#[cfg(test)]
fn take_post_probe_startup_fault_for_test(path: &Path) -> Option<PostProbeStartupFaultForTest> {
    let mut hook = POST_PROBE_STARTUP_FAULT_FOR_TEST.lock().expect("startup fault hook lock");
    if hook.as_ref().is_some_and(|candidate| candidate.path == path) {
        hook.take()
    } else {
        None
    }
}

#[cfg(test)]
pub(crate) fn install_admission_locked_hook_for_test(path: PathBuf, rendezvous: Arc<Barrier>) {
    *ADMISSION_LOCKED_HOOK_FOR_TEST.lock().expect("admission hook lock") =
        Some(AdmissionLockedHookForTest { path, rendezvous });
}

#[cfg(test)]
fn run_admission_locked_hook_for_test(path: &Path) {
    let rendezvous = {
        let mut hook = ADMISSION_LOCKED_HOOK_FOR_TEST.lock().expect("admission hook lock");
        match hook.as_ref() {
            Some(candidate) if candidate.path == path => {
                hook.take().map(|candidate| candidate.rendezvous)
            }
            _ => None,
        }
    };
    if let Some(rendezvous) = rendezvous {
        rendezvous.wait();
        rendezvous.wait();
    }
}

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

impl Engine {
    pub fn open(path: impl Into<PathBuf>) -> Result<OpenedEngine, EngineOpenError> {
        Self::open_with_embedder_and_subscriber(
            path,
            default_embedder_identity(),
            None,
            None,
            None,
            &mut |_| {},
        )
    }

    /// Open an engine with an explicit [`EmbedderChoice`].
    ///
    /// Per `dev/design/embedder.md` §0 + the 0.7.1 EU-5 campaign, this is
    /// the canonical entry point for selecting how the workspace's
    /// default embedder is supplied. See [`EmbedderChoice`] for the
    /// semantics of each variant; in particular `Default` materializes
    /// the pinned BGE embedder via the loader when the `default-embedder`
    /// feature is enabled.
    pub fn open_with_choice(
        path: impl Into<PathBuf>,
        choice: EmbedderChoice,
    ) -> Result<OpenedEngine, EngineOpenError> {
        Self::open_with_choice_and_config(path, choice, EngineConfig::default())
    }

    /// Open with an embedder choice and per-engine settings. Invalid settings
    /// fail before path, lock, provider, or SQLite side effects.
    pub fn open_with_choice_and_config(
        path: impl Into<PathBuf>,
        choice: EmbedderChoice,
        config: EngineConfig,
    ) -> Result<OpenedEngine, EngineOpenError> {
        ResolvedRuntimeConfiguration::resolve(&config)
            .map_err(EngineOpenError::EngineConfiguration)?;
        match choice {
            EmbedderChoice::Default => Self::open_default_embedder(path, config),
            EmbedderChoice::Caller(embedder) => {
                let identity = embedder.identity();
                Self::open_with_embedder_and_subscriber_config(
                    path,
                    identity,
                    Some(embedder),
                    None,
                    None,
                    &mut |_| {},
                    config,
                )
            }
            EmbedderChoice::CallerWithDeviceResolution { embedder, device_resolution } => {
                let identity = embedder.identity();
                Self::open_with_embedder_and_subscriber_config(
                    path,
                    identity,
                    Some(embedder),
                    Some(LoaderInfo {
                        download_ms: None,
                        events: Vec::new(),
                        device_resolution,
                        // A caller-supplied embedder was not constructed here,
                        // so nothing in this process measured an allocation.
                        gpu_allocation_witness: None,
                    }),
                    None,
                    &mut |_| {},
                    config,
                )
            }
            EmbedderChoice::None => Self::open_with_embedder_and_subscriber_config(
                path,
                default_embedder_identity(),
                None,
                None,
                None,
                &mut |_| {},
                config,
            ),
        }
    }

    /// EU-5b: materialize the engine's pinned default embedder
    /// (`CandleBgeEmbedder` backed by the EU-3 loader) and open the
    /// workspace with it. Without the `default-embedder` feature, fails
    /// with a typed `Embedder` error rather than touching the network.
    #[cfg(feature = "default-embedder")]
    fn open_default_embedder(
        path: impl Into<PathBuf>,
        config: EngineConfig,
    ) -> Result<OpenedEngine, EngineOpenError> {
        use std::time::Instant as DownloadInstant;
        let device_resolution = fathomdb_embedder::resolve_default_embedder_device_from_env()
            .map_err(EngineOpenError::EmbedDevicePolicy)?;
        // 0.8.23 Slice 80.6 (D-80.6-6) — the witness runs BEFORE this open's
        // own model reaches the device, so its `free_before`/`free_after`
        // bracket surrounds nothing but the load it is measuring. Opted in
        // only; see `ENV_GPU_ALLOCATION_WITNESS`.
        let gpu_allocation_witness = witness_gpu_allocation_if_requested(&device_resolution)?;
        let download_start = DownloadInstant::now();
        let weights = fathomdb_embedder::loader::load_pinned_default_embedder().map_err(|err| {
            EngineOpenError::Embedder(RuntimeEmbedderError::Failed {
                message: format!("default embedder loader: {err}"),
            })
        })?;
        let events = weights.events.clone();
        let download_ms = if weights.bytes_downloaded > 0 {
            Some(u64::try_from(download_start.elapsed().as_millis()).unwrap_or(u64::MAX))
        } else {
            None
        };
        let embedder =
            fathomdb_embedder::CandleBgeEmbedder::new_from_weights_with_device_resolution(
                weights,
                &device_resolution,
            )
            .map_err(|err| {
                EngineOpenError::Embedder(RuntimeEmbedderError::Failed {
                    message: format!("default embedder construct: {err}"),
                })
            })?;
        let embedder: Arc<dyn Embedder> = Arc::new(embedder);
        let identity = embedder.identity();
        let loader_info =
            LoaderInfo { download_ms, events, device_resolution, gpu_allocation_witness };
        Self::open_with_embedder_and_subscriber_config(
            path,
            identity,
            Some(embedder),
            Some(loader_info),
            None,
            &mut |_| {},
            config,
        )
    }

    #[cfg(not(feature = "default-embedder"))]
    fn open_default_embedder(
        _path: impl Into<PathBuf>,
        _config: EngineConfig,
    ) -> Result<OpenedEngine, EngineOpenError> {
        Err(EngineOpenError::Embedder(RuntimeEmbedderError::Failed {
            message: "EmbedderChoice::Default requires the `default-embedder` Cargo feature"
                .to_string(),
        }))
    }

    pub fn open_with_migration_event_sink(
        path: impl Into<PathBuf>,
        mut emit_migration_event: impl FnMut(&MigrationStepReport),
    ) -> Result<OpenedEngine, EngineOpenError> {
        Self::open_with_embedder_and_subscriber(
            path,
            default_embedder_identity(),
            None,
            None,
            None,
            &mut emit_migration_event,
        )
    }

    #[cfg(feature = "migration-test-hooks")]
    #[doc(hidden)]
    pub fn open_with_migrations_for_test(
        path: impl Into<PathBuf>,
        migrations: &'static [fathomdb_schema::Migration],
        mut emit_migration_event: impl FnMut(&MigrationStepReport),
    ) -> Result<OpenedEngine, EngineOpenError> {
        Self::open_with_migrations(
            path,
            DatabaseOpenPlan {
                migrations,
                admission: DatabaseAdmission::TestMigrations,
                config: EngineConfig::default(),
            },
            default_embedder_identity(),
            None,
            None,
            &mut emit_migration_event,
            None,
        )
    }

    #[doc(hidden)]
    pub fn open_with_subscriber_for_test(
        path: impl Into<PathBuf>,
        subscriber: Arc<dyn lifecycle::Subscriber>,
    ) -> Result<OpenedEngine, EngineOpenError> {
        Self::open_with_embedder_and_subscriber(
            path,
            default_embedder_identity(),
            None,
            None,
            Some(subscriber),
            &mut |_| {},
        )
    }

    #[doc(hidden)]
    pub fn open_without_embedder_for_test(
        path: impl Into<PathBuf>,
    ) -> Result<OpenedEngine, EngineOpenError> {
        Self::open_with_embedder_and_subscriber(
            path,
            default_embedder_identity(),
            None,
            None,
            None,
            &mut |_| {},
        )
    }

    #[doc(hidden)]
    pub fn open_with_embedder_for_test(
        path: impl Into<PathBuf>,
        embedder: Arc<dyn Embedder>,
    ) -> Result<OpenedEngine, EngineOpenError> {
        let identity = embedder.identity();
        Self::open_with_embedder_and_subscriber(
            path,
            identity,
            Some(embedder),
            None,
            None,
            &mut |_| {},
        )
    }

    pub(crate) fn open_with_embedder_and_subscriber(
        path: impl Into<PathBuf>,
        embedder_identity: EmbedderIdentity,
        runtime_embedder: Option<Arc<dyn Embedder>>,
        loader_info: Option<LoaderInfo>,
        initial_subscriber: Option<Arc<dyn lifecycle::Subscriber>>,
        emit_migration_event: &mut impl FnMut(&MigrationStepReport),
    ) -> Result<OpenedEngine, EngineOpenError> {
        Self::open_with_embedder_and_subscriber_config(
            path,
            embedder_identity,
            runtime_embedder,
            loader_info,
            initial_subscriber,
            emit_migration_event,
            EngineConfig::default(),
        )
    }

    fn open_with_embedder_and_subscriber_config(
        path: impl Into<PathBuf>,
        embedder_identity: EmbedderIdentity,
        runtime_embedder: Option<Arc<dyn Embedder>>,
        loader_info: Option<LoaderInfo>,
        initial_subscriber: Option<Arc<dyn lifecycle::Subscriber>>,
        emit_migration_event: &mut impl FnMut(&MigrationStepReport),
        config: EngineConfig,
    ) -> Result<OpenedEngine, EngineOpenError> {
        Self::open_with_migrations(
            path,
            DatabaseOpenPlan {
                migrations: MIGRATIONS,
                admission: DatabaseAdmission::CurrentOnly,
                config,
            },
            embedder_identity,
            runtime_embedder,
            loader_info,
            emit_migration_event,
            initial_subscriber,
        )
    }

    fn open_with_migrations(
        path: impl Into<PathBuf>,
        plan: DatabaseOpenPlan,
        embedder_identity: EmbedderIdentity,
        runtime_embedder: Option<Arc<dyn Embedder>>,
        loader_info: Option<LoaderInfo>,
        emit_migration_event: &mut impl FnMut(&MigrationStepReport),
        initial_subscriber: Option<Arc<dyn lifecycle::Subscriber>>,
    ) -> Result<OpenedEngine, EngineOpenError> {
        let resolved_config = ResolvedRuntimeConfiguration::resolve(&plan.config)
            .map_err(EngineOpenError::EngineConfiguration)?;
        let config = plan.config.clone();
        // Resolve at open rather than piggybacking on embedding selection. This
        // probes no model/cache/database and makes an invalid or forced CUDA
        // policy visible to every SDK before a query could silently fall back.
        #[cfg(feature = "default-reranker")]
        let reranker_device_resolution = Some(
            fathomdb_embedder::resolve_default_reranker_device_from_env()
                .map_err(EngineOpenError::RerankerDevicePolicy)?,
        );
        #[cfg(not(feature = "default-reranker"))]
        let reranker_device_resolution = None;
        let canonical_path = canonical_database_path(&path.into())?;
        let report_preopen_error = |error| {
            if let Some(subscriber) = initial_subscriber.as_ref() {
                emit_open_error_event(subscriber, &error);
            }
            error
        };
        let embed_dispatch = Arc::new(
            EmbedDispatcher::new(
                runtime_embedder.clone(),
                resolved_config.embedder_pool_size,
                Duration::from_millis(resolved_config.embedder_call_timeout_ms),
            )
            .map_err(|error| EngineOpenError::Io { message: error.to_string() })
            .map_err(&report_preopen_error)?,
        );
        let mut embed_dispatch_guard = OpenEmbedDispatchGuard(Some(Arc::clone(&embed_dispatch)));
        let pending_lock = acquire_lock_without_metadata_mutation(&canonical_path)
            .map_err(&report_preopen_error)?;
        configure_runtime_for_open()
            .map_err(EngineOpenError::RuntimeConfiguration)
            .map_err(&report_preopen_error)?;
        #[cfg(test)]
        run_admission_locked_hook_for_test(&canonical_path);
        if plan.admission == DatabaseAdmission::CurrentOnly {
            admit_current_database(&canonical_path).map_err(&report_preopen_error)?;
        }
        let lock = pending_lock.initialize().map_err(&report_preopen_error)?;
        #[cfg(any(test, feature = "test-hooks"))]
        let managed_connections = Arc::new(ManagedConnectionRegistry::default());
        #[cfg(feature = "migration-test-hooks")]
        let allow_populated_legacy_projection_bootstrap =
            plan.admission == DatabaseAdmission::TestMigrations;
        #[cfg(not(feature = "migration-test-hooks"))]
        let allow_populated_legacy_projection_bootstrap = false;
        let open_result = Self::open_locked(
            canonical_path.clone(),
            plan.migrations,
            allow_populated_legacy_projection_bootstrap,
            &embedder_identity,
            emit_migration_event,
            #[cfg(any(test, feature = "test-hooks"))]
            Arc::clone(&managed_connections),
        );

        match open_result {
            Ok((connection, readers, mut report, reader_lookaside_rcs)) => {
                // EU-5b — splice the loader's measurements + structured
                // events into the report. The loader path is the only
                // surface that produces these today; caller-supplied
                // embedders and EmbedderChoice::None leave them as the
                // open_locked defaults (None / empty).
                if let Some(info) = loader_info {
                    if info.download_ms.is_some() {
                        report.embedder_download_ms = info.download_ms;
                    }
                    if !info.events.is_empty() {
                        report.embedder_events = info.events;
                    }
                    report.embedder_device_resolution = Some(info.device_resolution);
                    // D-80.6-6 — assigned, not merged: `None` here means this
                    // open measured no witness, and there is no earlier value
                    // that a `None` could be hiding.
                    report.embedder_gpu_allocation_witness = info.gpu_allocation_witness;
                }
                report.reranker_device_resolution = reranker_device_resolution;

                // 0.8.18 Slice 5 (#5 vector-equivalence probe KEYSTONE) — run the
                // open-time self-check on the FINAL post-recovery connection (the
                // mean is already pinned/recovered inside open_locked, U1-b). First
                // registration persists the 45 UN-centered f32 references; a
                // subsequent open re-embeds + asserts P1 (mean-centered flip count,
                // floor 0) and P2 (un-centered L2 ε). Divergence ⇒ degraded-open
                // (`dense_disabled=true`), surfaced on the OpenReport (R-VEQ-6); the
                // query-time refusal fires later at `search_inner_with_stats`.
                // A durable declaration is a prospective dense arm even before
                // it has enrolled a kind. Check it before the boot graft below:
                // a refused backend may leave the declaration at rest, but must
                // not enrol, requeue, dispatch, or write any dense work.
                let prospective_dense_arm =
                    vector_projection_declared(&connection).map_err(|_| EngineOpenError::Io {
                        message: "could not inspect declared vector projection on open".to_string(),
                    })?;
                let veq = run_vector_equivalence_probe(
                    &connection,
                    runtime_embedder.as_ref().map(|_| embed_dispatch.as_ref()),
                    &embedder_identity,
                    report.embedder_mean_vec_pinned,
                    prospective_dense_arm,
                );
                report.dense_disabled = veq.dense_disabled;
                report.dense_disabled_reason = veq.reason.clone();

                let mut startup =
                    OpenPostProbeGuard::new(connection, readers, lock, Arc::clone(&embed_dispatch));

                let dense_runtime_usable =
                    usable_dense_runtime(runtime_embedder.as_deref(), veq.dense_disabled);
                let boot_graft_enqueued = if dense_runtime_usable {
                    boot_graft_declared_vector_backfill(startup.connection()).map_err(|_| {
                        EngineOpenError::Io {
                            message: "could not graft declared vector projection on boot"
                                .to_string(),
                        }
                    })?
                } else {
                    false
                };

                let next_cursor = load_next_cursor(startup.connection());
                #[cfg(test)]
                if let Some(fault) = take_post_probe_visibility_fault_for_test(&canonical_path) {
                    startup.embed_dispatch.set_drain_budget_ms_for_test(40);
                    startup
                        .connection()
                        .execute_batch(
                            "DELETE FROM _fathomdb_read_visibility_state WHERE singleton=1",
                        )
                        .expect("inject missing visibility singleton");
                    fault
                        .observed
                        .send((
                            startup.embed_dispatch.accounting().expect("provider accounting"),
                            Arc::clone(&managed_connections),
                        ))
                        .expect("report visibility fault");
                }
                let read_visibility_generation = frozen_read::load_visibility_generation(
                    startup.connection(),
                )
                .map_err(|_| EngineOpenError::Io {
                    message: "could not load frozen-read visibility generation".to_string(),
                })?;
                let subscribers = Arc::new(lifecycle::SubscriberRegistry::new());
                let profiling_enabled = Arc::new(AtomicBool::new(false));
                let slow_threshold_ms = Arc::new(AtomicU64::new(resolved_config.slow_threshold_ms));
                let wal_attribution = Arc::new(WalAttributionCollector::new());
                wal_attribution.register(WalAttributionRole::Writer, 0);
                #[cfg(any(test, feature = "test-hooks"))]
                {
                    startup.writer_registration =
                        Some(managed_connections.register(WalAttributionRole::Writer, 0));
                }
                let scheduler_embedder =
                    if dense_runtime_usable { runtime_embedder.clone() } else { None };
                let projection_runtime = ProjectionRuntime::new(
                    canonical_path.clone(),
                    scheduler_embedder,
                    Arc::clone(&embed_dispatch),
                    embedder_identity.clone(),
                    report.embedder_mean_vec_pinned,
                    Arc::clone(&subscribers),
                    Arc::clone(&wal_attribution),
                    resolved_config,
                    #[cfg(any(test, feature = "test-hooks"))]
                    Arc::clone(&managed_connections),
                )?;
                startup.projection_runtime = Some(projection_runtime);
                startup.install_profiles(&subscribers, &profiling_enabled, &slow_threshold_ms);

                #[cfg(test)]
                if let Some(fault) = take_post_probe_startup_fault_for_test(&canonical_path) {
                    embed_dispatch.set_drain_budget_ms_for_test(40);
                    let profile_releases = Arc::new(ProfileReleaseObserver {
                        registry: Arc::clone(&managed_connections),
                        live_workers: Arc::new(AtomicUsize::new(0)),
                        releases: Mutex::new(Vec::new()),
                        custody: Mutex::new(Vec::new()),
                    });
                    fault
                        .observed
                        .send(PostProbeStartupObservationForTest {
                            accounting: embed_dispatch.accounting().expect("provider accounting"),
                            registry: Arc::clone(&managed_connections),
                            profiles_at_fault: startup.profile_contexts.len(),
                            profile_releases: Arc::clone(&profile_releases),
                        })
                        .expect("report injected startup fault");
                    startup.profile_release_observer = Some(profile_releases);
                    return Err(EngineOpenError::Io {
                        message: "injected post-probe startup failure".to_owned(),
                    });
                }

                let parts = startup.into_parts();
                let opened = OpenedEngine {
                    engine: Self {
                        path: canonical_path.clone(),
                        requested_config: config,
                        resolved_config,
                        next_cursor: AtomicU64::new(next_cursor),
                        read_visibility_generation: Arc::new(AtomicU64::new(
                            read_visibility_generation,
                        )),
                        projection_generation_status_cache: Mutex::new(None),
                        mutation_projection_status_cache: Mutex::new(None),
                        #[cfg(feature = "test-hooks")]
                        projection_generation_status_full_owner_scan_count: AtomicU64::new(0),
                        #[cfg(feature = "test-hooks")]
                        graph_expand_rss_baseline_bytes: AtomicU64::new(0),
                        #[cfg(feature = "test-hooks")]
                        graph_expand_rss_delta_bytes: AtomicU64::new(0),
                        #[cfg(feature = "test-hooks")]
                        graph_evidence_before_resolve_return_hook: Mutex::new(None),
                        #[cfg(feature = "test-hooks")]
                        erasure_before_primary_lock_hook: Mutex::new(None),
                        closed: AtomicBool::new(false),
                        close_lock: Mutex::new(()),
                        lock: Mutex::new(Some(parts.lock)),
                        connection: Mutex::new(Some(parts.connection)),
                        reader_pool: ReaderWorkerPool::new(
                            parts.readers,
                            Arc::clone(&wal_attribution),
                            #[cfg(any(test, feature = "test-hooks"))]
                            Arc::clone(&managed_connections),
                        ),
                        counters: lifecycle::Counters::new(),
                        subscribers,
                        profiling_enabled,
                        slow_threshold_ms,
                        runtime_embedder,
                        embed_dispatch,
                        runtime_embedder_identity: embedder_identity,
                        projection_runtime: parts.projection_runtime,
                        wal_attribution,
                        #[cfg(any(test, feature = "test-hooks"))]
                        managed_connections,
                        #[cfg(any(test, feature = "test-hooks"))]
                        writer_connection_registration: Mutex::new(Some(parts.writer_registration)),
                        #[cfg(any(test, feature = "test-hooks"))]
                        actual_checkpoint_observations: Mutex::new(None),
                        #[cfg(any(test, feature = "test-hooks"))]
                        binding_native_state_observations: Mutex::new(None),
                        provenance_row_cap: AtomicU64::new(resolved_config.provenance_row_cap),
                        profile_contexts: Mutex::new(parts.profile_contexts),
                        reader_lookaside_rcs,
                        telemetry: Mutex::new(None),
                        telemetry_enabled: AtomicBool::new(false),
                        explanation_open_nonce: mint_explanation_open_nonce(),
                        explanation_sequence: AtomicU64::new(0),
                        dense_disabled: AtomicBool::new(veq.dense_disabled),
                        dense_disabled_reason: Mutex::new(veq.reason),
                        vector_equivalence_refusals: AtomicU64::new(0),
                        #[cfg(debug_assertions)]
                        force_next_commit_failure: AtomicBool::new(false),
                        #[cfg(debug_assertions)]
                        actuation_after_initial_lookup_delay_ms: AtomicU64::new(0),
                        #[cfg(debug_assertions)]
                        actuation_failure_after_operation: AtomicUsize::new(usize::MAX),
                    },
                    report,
                };
                if let Some(subscriber) = initial_subscriber {
                    opened.engine.subscribers.attach_persistent(subscriber);
                }
                if dense_runtime_usable
                    && (boot_graft_enqueued
                        || database_has_pending_projection_work(
                            &canonical_path,
                            #[cfg(any(test, feature = "test-hooks"))]
                            &opened.engine.managed_connections,
                        )
                        .unwrap_or(false))
                {
                    opened.engine.projection_runtime.notify_new_work();
                }
                embed_dispatch_guard.disarm();
                Ok(opened)
            }
            Err(err) => {
                if let Some(subscriber) = initial_subscriber {
                    emit_open_error_event(&subscriber, &err);
                }
                drop(lock);
                Err(err)
            }
        }
    }

    fn open_locked(
        path: PathBuf,
        migrations: &'static [fathomdb_schema::Migration],
        allow_populated_legacy_projection_bootstrap: bool,
        embedder_identity: &EmbedderIdentity,
        emit_migration_event: &mut impl FnMut(&MigrationStepReport),
        #[cfg(any(test, feature = "test-hooks"))] managed_connections: Arc<
            ManagedConnectionRegistry,
        >,
    ) -> Result<(Connection, Vec<Connection>, OpenReport, Vec<i32>), EngineOpenError> {
        register_sqlite_vec_extension();
        let mut connection = open_managed_connection(
            &path,
            #[cfg(any(test, feature = "test-hooks"))]
            ManagedConnectionCategory::Writer,
            #[cfg(any(test, feature = "test-hooks"))]
            &managed_connections,
        )
        .map_err(|err| map_open_sqlite_error(err, OpenStage::HeaderProbe))?;
        // Order pinned by `dev/design/errors.md` § OpenStage matrix: each
        // step routes its own SQLite-level error to a distinct
        // `CorruptionKind` (Header → WalReplay → Schema → EmbedderIdentity).
        // The schema and WAL probes both happen BEFORE `pragma WAL`
        // because that pragma also reads page 1 — letting it run first
        // would reclassify schema-side corruption as a WAL replay
        // failure, breaking the AC-035b stable-code contract.
        probe_database_header(&connection)?;
        probe_open_integrity(&connection)?;
        probe_wal_sidecar(&path)?;
        // 0.7.0 perf-experiments: apply writer-side experiment PRAGMAs
        // (page_size, etc.) BEFORE journal_mode + migrations. page_size
        // is silently ignored once any table exists; this is the only
        // legal window to set it on a fresh DB. Gated on
        // FATHOMDB_PERF_EXPERIMENTS=1; no-op in production.
        apply_perf_experiment_writer_pragmas(&connection);
        // OPP-12 Phase-1 (0.8.19 Slice 10, design §3 gap-4) — standing
        // `secure_delete=ON` on the writer, applied at EVERY open (fresh + migrated).
        // It zeroes every page freed by a future DELETE, so the Slice-10 `purge`
        // hard-erase is complete WITHOUT a per-purge `VACUUM`. It is a connection
        // PRAGMA (not schema DDL), so it belongs here, not in the 19→20 migration.
        // RESIDUAL (documented, not forced): pages freed on a pre-20 DB BEFORE this
        // was enabled are not retroactively scrubbed; there is no migration-time
        // full `VACUUM` (O(db-size)). NOTE: this is a standing pragma set at EVERY
        // connection open (writer here, plus the reader-pool and
        // `open_runtime_connection`), NOT the writer alone — non-writer connections
        // also free pages (projection / vector-rewrite DELETEs), so a writer-only
        // `secure_delete` would leak freed content on disk. See the matching
        // reader/runtime open comment (~lines 3335-3336).
        connection
            .pragma_update(None, "secure_delete", "ON")
            .map_err(|err| map_open_sqlite_error(err, OpenStage::WalReplay))?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(|err| map_open_sqlite_error(err, OpenStage::WalReplay))?;
        connection
            .pragma_update(None, "synchronous", "NORMAL")
            .map_err(|err| map_open_sqlite_error(err, OpenStage::WalReplay))?;
        #[cfg(feature = "test-hooks")]
        record_writer_pragma_witness_for_test(&connection);

        reject_legacy_shape(&connection)?;
        let migration = migrate_with_event_sink(&connection, migrations, emit_migration_event)
            .map_err(map_migration_error)?;
        validate_dependency_generation_on_open(&connection, migration.schema_version_after)?;
        frozen_read::validate_on_open(&connection, migration.schema_version_after)
            .map_err(|message| EngineOpenError::Io { message })?;
        dependency_closure::validate_closure_state_on_open(
            &connection,
            migration.schema_version_after,
        )?;
        // 0.8.0 Slice 5 (G1) — global FTS5 tokenizer-default upgrade. Step 11
        // drops + recreates `search_index` with the new tokenizer, leaving it
        // EMPTY on a migrated DB. The projection scheduler will NOT
        // repopulate it (`database_has_pending_projection_work` keys "pending"
        // off `_fathomdb_projection_terminal`, which the migration does not
        // clear). Re-tokenize from the canonical source rows here, on the
        // writer connection, single-threaded, before readers spawn —
        // projection-only, no source-record migration.
        //
        // Crash-retryable (fix-1): step 11 commits `user_version = 11` with an
        // empty index in its OWN transaction; this reproject commits in a
        // LATER transaction. A crash in that window leaves a durable v11 + empty
        // index, on which a boundary-crossing guard (`before < 11`) is FALSE,
        // skipping repair forever. So gate on the completion marker's ABSENCE
        // (written atomically with the reindex) instead: idempotent, and a
        // crash before the reindex commit simply re-runs on the next open.
        if migration.schema_version_after >= SEARCH_INDEX_TOKENIZER_SCHEMA_VERSION
            && !search_index_tokenizer_reproject_complete(&connection).map_err(|_| {
                EngineOpenError::Io {
                    message: "could not read search_index tokenizer reproject marker".to_string(),
                }
            })?
        {
            reproject_search_index_after_tokenizer_upgrade(&connection).map_err(|_| {
                EngineOpenError::Io {
                    message: "could not re-tokenize search_index after tokenizer upgrade"
                        .to_string(),
                }
            })?;
        }
        let mut embedder_mean_vec_pinned = check_embedder_profile(&connection, embedder_identity)?;
        ensure_vector_partition(&mut connection, embedder_identity.dimension).map_err(|_| {
            EngineOpenError::Io { message: "could not initialize vector partition".to_string() }
        })?;
        projection_generation::bootstrap(
            &mut connection,
            migration.schema_version_after,
            allow_populated_legacy_projection_bootstrap,
        )
        .map_err(|error| match error {
            EngineError::ProjectionGeneration(_) => EngineOpenError::Corruption(CorruptionDetail {
                kind: CorruptionKind::ProjectionGenerationDrift,
                stage: OpenStage::ProjectionGeneration,
                locator: CorruptionLocator::TableRow {
                    table: "_fathomdb_projection_generation_current",
                    rowid: 1,
                },
                recovery_hint: RecoveryHint {
                    code: "E_CORRUPT_PROJECTION_GENERATION",
                    doc_anchor: "design/recovery-0.8.25.md#projection-generation",
                },
            }),
            _ => EngineOpenError::Io {
                message: "could not initialize projection generation".to_string(),
            },
        })?;

        // 0.8.20 Slice 15c (TC-33) fix-6 [codex §9 P1] — the step-23
        // `canonical_edges` recreate drops every edge row (NO DATA MIGRATION) and
        // removes their `_fathomdb_vector_rows` sidecar rows, but the vec0
        // `vector_default` shadow those mirror is engine-created + dim-aware, so
        // the migration cannot delete its rows. Left behind, an orphaned edge vec0
        // row (whose `canonical_edges` row is gone) still occupies a top-K KNN
        // candidate slot — `build_vector_phase1_sql` reads candidates DIRECTLY
        // from `vector_default` before hydrating them through the canonical tables
        // — and is then discarded at hydration, so an upgraded DB silently returns
        // too few / no vector results. Prune the orphans now that
        // `ensure_vector_partition` guarantees `vector_default` exists, BEFORE the
        // mean-vec row-count recovery below (so the count excludes them). One-time
        // and crash-retryable via the durable completion marker; a no-op on any
        // healthy corpus (every vec0 row has a sidecar entry), so recall / eu7
        // fidelity are unchanged on a DB that never dropped edges.
        if migration.schema_version_after >= EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION
            && !edge_vector_prune_complete(&connection).map_err(|_| EngineOpenError::Io {
                message: "could not read edge-vector prune marker".to_string(),
            })?
        {
            prune_orphaned_edge_vectors(&connection).map_err(|_| EngineOpenError::Io {
                message: "could not prune orphaned edge vector rows".to_string(),
            })?;
        }

        // 0.8.20 Slice 15d (R-20-PR, Q5) — boot re-derive the projection registry
        // (the engine `ProjectionSpec` is a derived cache). For every persisted
        // declaration, clear + backfill its EAV / property-FTS rows from the
        // canonical nodes so a crash window (registry row survives, projection
        // rows partial) self-heals idempotently. A no-op single empty-table read
        // on every DB that has not declared a projection. On the writer
        // connection, single-threaded, before readers spawn — like the tokenizer
        // reproject above. Runs after the fix-6 edge-vector prune above; the two
        // are independent boot reconciliations.
        rederive_projections_on_boot(&connection).map_err(|_| EngineOpenError::Io {
            message: "could not re-derive projection registry on boot".to_string(),
        })?;

        // 0.8.20 Slice 21 fix-1 (codex §9 round 1 [P2], ledger `TC-71`) — bring an
        // ALREADY-ENROLLED inert vector kind into agreement with the role-aware
        // decision. Slice 21c closed the three forward doors, but a database that
        // already ran the old code under `{roles:[filterable], vector:{}}` keeps
        // its `_fathomdb_vector_kinds` rows — `vector_kind_needs_enrolment`
        // short-circuits on `kind_is_vector_indexed` and never reaches the new
        // predicate, and `project_canonical_node_row` reads only the registry
        // membership — so upgrading did not actually stop the unwanted embeddings.
        // Narrowly authorised (registry EXISTS, declares a `vector` sub-object,
        // and declares no `searchable→vector` projection) so a LEGACY workspace
        // with a working dense arm is never touched; see
        // [`registry_governs_an_inert_dense_arm`]. Deletes no embedding. Runs
        // BEFORE `run_vector_equivalence_probe` (which fires after `open_locked`
        // returns), so a database whose only enrolment was the inert one pays no
        // probe embeds on the healing open. Another boot reconciliation on the
        // writer connection, single-threaded, before readers spawn.
        reconcile_inert_vector_enrolments_on_boot(&connection).map_err(|_| {
            EngineOpenError::Io {
                message: "could not reconcile inert vector kind enrolments on boot".to_string(),
            }
        })?;

        // 0.8.20 Slice 15e — reconcile the live `vector_default` attribute columns
        // with the registry's `filterable` set. On a DB whose vec0 shape already
        // matches the registry (the common case, incl. every reopen of a DB that
        // declared filterable projections in a prior session) this is a pure
        // no-op: the diff is empty, so boot never re-inserts and NEVER silently
        // wipes the corpus. It converges only a shape that drifted from the
        // registry (e.g. a restored registry row). A no-op when the table is
        // absent (no embedder). Runs on the writer connection, single-threaded,
        // before readers spawn — like the boot re-derive above.
        {
            let tx = connection.transaction().map_err(|_| EngineOpenError::Io {
                message: "could not begin vector-attr reconcile on boot".to_string(),
            })?;
            reconcile_vector_attr_columns(&tx, embedder_identity.dimension).map_err(|_| {
                EngineOpenError::Io {
                    message: "could not reconcile vector attribute columns on boot".to_string(),
                }
            })?;
            tx.commit().map_err(|_| EngineOpenError::Io {
                message: "could not commit vector-attr reconcile on boot".to_string(),
            })?;
        }

        // EU-5f — recovery pin (`dev/design/embedder.md` §0.3, Hazard 4). If
        // the identity is MC-required, no mean is pinned, yet the workspace
        // already holds >= MEAN_VEC_PIN_THRESHOLD vector rows (e.g. a crash
        // between the threshold-crossing write and its pin commit), derive
        // the mean from the existing un-centered rows and pin+re-quantize
        // now, single-threaded, before the projection workers spawn. The
        // NULL guard makes this idempotent on subsequent opens.
        if identity_requires_mean_centering(embedder_identity) && !embedder_mean_vec_pinned {
            let row_count: u64 = connection
                .query_row("SELECT COUNT(*) FROM vector_default", [], |row| row.get(0))
                .unwrap_or(0);
            if row_count >= MEAN_VEC_PIN_THRESHOLD {
                recover_mean_vec_pin(&mut connection, embedder_identity).map_err(|_| {
                    EngineOpenError::Io {
                        message: "could not recover mean-centering pin".to_string(),
                    }
                })?;
                embedder_mean_vec_pinned = true;
            }
        }

        let warmup_started = Instant::now();
        // Static identity capability — see `dev/design/embedder.md`
        // §0.6. Today only the bge-small identity reports `true`; the
        // noop scaffolding identity is `false`. EU-5b's identity flip
        // makes the Default path return `true` here automatically.
        let embedder_mean_centering_required = embedder_identity.name == BGE_SMALL_EMBEDDER_NAME;
        // EU-5a2 — populated from `_fathomdb_embedder_profiles.mean_vec`
        // by `check_embedder_profile` above (was hard-coded `false` in
        // EU-5a1). Dimension invariant (§0.2) enforced by that check.
        let report = OpenReport {
            schema_version_before: migration.schema_version_before,
            schema_version_after: migration.schema_version_after,
            migration_steps: migration.migration_steps,
            embedder_warmup_ms: u64::try_from(warmup_started.elapsed().as_millis())
                .unwrap_or(u64::MAX),
            query_backend: "fathomdb-query + sqlite-vec",
            default_embedder: embedder_identity.clone(),
            // TODO(EU-5b): surface `LoadedWeights.download_ms` from the
            // loader once the Default path materializes through it.
            embedder_download_ms: None,
            // TODO(EU-5b): surface `LoadedWeights.events` from the loader.
            embedder_events: Vec::new(),
            embedder_mean_centering_required,
            embedder_mean_vec_pinned,
            // 0.8.18 Slice 5 — set by the #5 self-check in `open_with_migrations`
            // (which has the runtime embedder in scope). `open_locked` returns the
            // non-degraded default; the probe runs after this returns.
            dense_disabled: false,
            dense_disabled_reason: None,
            embedder_device_resolution: None,
            reranker_device_resolution: None,
            // 0.8.23 Slice 80.6 (D-80.6-6) — set by `open_with_migrations`
            // only when an opted-in CUDA default-embedder open measured one.
            embedder_gpu_allocation_witness: None,
        };

        let mut readers = Vec::with_capacity(READER_POOL_SIZE);
        let mut lookaside_rcs: Vec<i32> = Vec::with_capacity(READER_POOL_SIZE);
        for _ in 0..READER_POOL_SIZE {
            let reader = open_managed_connection(
                &path,
                #[cfg(any(test, feature = "test-hooks"))]
                ManagedConnectionCategory::ReaderWorker,
                #[cfg(any(test, feature = "test-hooks"))]
                &managed_connections,
            )
            .map_err(|err| map_open_sqlite_error(err, OpenStage::HeaderProbe))?;
            // Pack 6.G G.1: configure per-connection lookaside BEFORE
            // any PRAGMA / prepare runs on this reader. Reordering this
            // after the journal-mode / query_only PRAGMAs would let
            // SQLite silently ignore the lookaside setting.
            let rc: i32 = configure_reader_lookaside(&reader);
            debug_assert_eq!(
                rc,
                rusqlite::ffi::SQLITE_OK,
                "sqlite3_db_config(LOOKASIDE) must return SQLITE_OK on a freshly opened reader",
            );
            lookaside_rcs.push(rc);
            reader
                .pragma_update(None, "journal_mode", "WAL")
                .map_err(|err| map_open_sqlite_error(err, OpenStage::WalReplay))?;
            // OPP-12 Phase-1 (0.8.19 Slice 10, design §3 gap-4) — `secure_delete=ON`
            // at EVERY connection open, not just the writer. `secure_delete` is a
            // per-connection pager flag, so a reader-pool connection that frees a
            // page (vector-rewrite / projection DELETEs run off non-writer
            // connections) would otherwise leave that freed content on disk,
            // defeating GDPR erasure. Set BEFORE `query_only=ON` so the ordering is
            // unambiguous (the flag is a pager setting, not a DB write).
            reader
                .pragma_update(None, "secure_delete", "ON")
                .map_err(|err| map_open_sqlite_error(err, OpenStage::WalReplay))?;
            reader
                .pragma_update(None, "query_only", "ON")
                .map_err(|err| map_open_sqlite_error(err, OpenStage::SchemaProbe))?;
            apply_perf_experiment_reader_pragmas(&reader);
            readers.push(reader);
        }

        Ok((connection, readers, report, lookaside_rcs))
    }
}

/// 0.8.20 Slice 15c (TC-33) fix-6 — `_fathomdb_open_state` key set once the
/// one-time edge-vector prune commits durably (written in the SAME transaction
/// as the vec0 DELETEs). Gating repair on this marker's ABSENCE — not on
/// crossing the step-23 boundary — makes it crash-retryable: the step-23
/// migration commits `user_version = 23` (edges dropped, sidecar cleared) in its
/// own transaction, and the prune runs in a later transaction on open. A crash
/// in that window leaves a durable `user_version = 23` with orphaned vec0 rows;
/// a boundary-crossing gate (`before < 23`) would skip the prune forever on the
/// next open (it sees `before == 23`). The marker is absent on any DB upgraded
/// before this fix shipped, so the prune runs once and cleans the lingering
/// orphans; thereafter the paired vec0/sidecar insert+delete keeps the invariant
/// so no new orphans arise.
const EDGE_VECTOR_PRUNE_MARKER_KEY: &str = "tc33_edge_vector_prune_complete";

/// 0.8.20 Slice 15c (TC-33) fix-6 — has the one-time edge-vector prune committed
/// durably on this DB? Keys off the [`EDGE_VECTOR_PRUNE_MARKER_KEY`] row written
/// inside the prune transaction; its absence means the prune never ran (a DB
/// upgraded before this fix shipped, or a crash between the step-23 commit and
/// the prune commit) and must (re-)run.
///
/// A MISSING `_fathomdb_open_state` table is reported as "complete" (skip the
/// prune) — that table is created by migration step 1, so its absence means the
/// DB never ran our migrations (a synthetic/foreign shape rejected downstream);
/// the prune must not run, and must not mask those errors, on it. Mirrors
/// [`search_index_tokenizer_reproject_complete`].
fn edge_vector_prune_complete(connection: &Connection) -> rusqlite::Result<bool> {
    match connection.query_row(
        "SELECT value FROM _fathomdb_open_state WHERE key = ?1",
        [EDGE_VECTOR_PRUNE_MARKER_KEY],
        |row| row.get::<_, String>(0),
    ) {
        Ok(value) => Ok(value == "1"),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(rusqlite::Error::SqliteFailure(_, Some(ref message)))
            if message.contains("no such table") =>
        {
            Ok(true)
        }
        Err(err) => Err(err),
    }
}

/// 0.8.20 Slice 15c (TC-33) fix-6 — delete every `vector_default` (vec0) row that
/// has NO `_fathomdb_vector_rows` sidecar entry, then record the durable
/// completion marker, all in one `BEGIN IMMEDIATE` transaction (crash-retryable:
/// a crash before COMMIT leaves no marker and the next open re-runs).
///
/// A vec0 row and its sidecar row are written and deleted TOGETHER (same
/// transaction) on every steady-state path, so a sidecar-less vec0 row is ONLY
/// ever produced by the step-23 recreate, which drops the edge rows and their
/// sidecar entries but cannot reach the engine-created vec0 table. So this
/// targets exactly the dropped edges' orphans and touches NOTHING on a healthy
/// corpus. Node vec0 rows keep their sidecar entry, so they are never pruned —
/// node recall is unaffected.
///
/// The orphans are gathered with plain scans (both proven vec0 forms — a full
/// `SELECT rowid FROM vector_default` and per-`rowid` `DELETE`) and diffed in
/// Rust, rather than relying on a compound `DELETE ... WHERE rowid NOT IN (...)`
/// over the virtual table.
fn prune_orphaned_edge_vectors(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch("BEGIN IMMEDIATE")?;
    let result = (|| {
        let sidecar: std::collections::HashSet<i64> = {
            let mut statement =
                connection.prepare("SELECT write_cursor FROM _fathomdb_vector_rows")?;
            let rows = statement.query_map([], |row| row.get::<_, i64>(0))?;
            let mut set = std::collections::HashSet::new();
            for r in rows {
                set.insert(r?);
            }
            set
        };
        let vec_rowids: Vec<i64> = {
            let mut statement = connection.prepare("SELECT rowid FROM vector_default")?;
            let rows = statement.query_map([], |row| row.get::<_, i64>(0))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r?);
            }
            out
        };
        for rowid in vec_rowids {
            if !sidecar.contains(&rowid) {
                // vec0 rowid IS the canonical write_cursor; delete by rowid (the
                // proven vec0 delete form, as `prune_edge_projection_shadows`),
                // through the one TC-76-safe vec0-delete primitive.
                delete_vector_partition_row(connection, rowid)?;
            }
        }
        connection.execute(
            "INSERT INTO _fathomdb_open_state(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![EDGE_VECTOR_PRUNE_MARKER_KEY, "1"],
        )?;
        Ok(())
    })();
    match result {
        Ok(()) => connection.execute_batch("COMMIT"),
        Err(err) => {
            let _ = connection.execute_batch("ROLLBACK");
            Err(err)
        }
    }
}

pub(crate) fn probe_open_integrity(connection: &Connection) -> Result<(), EngineOpenError> {
    // `SELECT COUNT(*) FROM sqlite_schema` forces a full traversal of the
    // sqlite_schema b-tree; this surfaces page-1 b-tree corruption that a
    // bare `PRAGMA schema_version` (which only reads the schema cookie
    // out of the file header) would miss.
    connection
        .query_row("SELECT COUNT(*) FROM sqlite_schema", [], |row| row.get::<_, i64>(0))
        .map(|_| ())
        .map_err(|err| map_open_sqlite_error(err, OpenStage::SchemaProbe))
}

pub(crate) fn probe_database_header(connection: &Connection) -> Result<(), EngineOpenError> {
    connection
        .query_row("PRAGMA application_id", [], |row| row.get::<_, i64>(0))
        .map(|_| ())
        .map_err(|err| map_open_sqlite_error(err, OpenStage::HeaderProbe))
}

/// Pre-`pragma WAL` sidecar validation. SQLite silently discards a WAL
/// file whose header magic is wrong or whose advertised page size is
/// outside `[512, SQLITE_MAX_PAGE_SIZE]`, which would cause us to lose
/// committed frames at open time. AC-035a requires that we instead
/// refuse to open with `Corruption(WalReplayFailure)` rather than
/// silently rebuild from a truncated WAL.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WalSidecarHeader {
    AbsentOrShort,
    Valid,
    Malformed { offset: u64 },
}

pub(crate) fn classify_wal_sidecar(db_path: &Path) -> Result<WalSidecarHeader, EngineOpenError> {
    let mut wal_path = db_path.as_os_str().to_owned();
    wal_path.push("-wal");
    let wal_path = PathBuf::from(wal_path);
    // Bounded read: the WAL header is fixed-layout in the first 32
    // bytes (magic + format + page-size + checkpoint-seq + salts +
    // checksums); frame data starts at offset 32 and is irrelevant to
    // the magic + page-size pre-check. A `std::fs::read` of the whole
    // sidecar would force an unclean-shutdown open path to allocate
    // and copy the entire WAL into memory before SQLite touches
    // recovery — a real latency + RSS regression on AC-035.
    use std::io::Read;
    let mut file = match std::fs::File::open(&wal_path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(WalSidecarHeader::AbsentOrShort)
        }
        Err(_) => {
            return Err(EngineOpenError::Io {
                message: "database WAL sidecar is not accessible".to_string(),
            })
        }
    };
    let mut bytes = [0u8; 32];
    if let Err(error) = file.read_exact(&mut bytes) {
        if error.kind() == std::io::ErrorKind::UnexpectedEof {
            // A short (< 32-byte) sidecar carries no committed frames;
            // SQLite treats it as empty and re-initializes WAL state.
            return Ok(WalSidecarHeader::AbsentOrShort);
        }
        return Err(EngineOpenError::Io {
            message: "database WAL sidecar could not be read".to_string(),
        });
    }
    let magic = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let page_size = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    // WAL_MAGIC mask per SQLite `walIndexRecover`: low bit distinguishes
    // big-endian vs little-endian checksum encoding; the rest of the
    // magic is fixed.
    const WAL_MAGIC_MASK: u32 = 0xFFFF_FFFE;
    const WAL_MAGIC: u32 = 0x377F_0682;
    const SQLITE_MAX_PAGE_SIZE: u32 = 65536;
    let magic_ok = (magic & WAL_MAGIC_MASK) == WAL_MAGIC;
    let page_size_ok =
        page_size.is_power_of_two() && (512..=SQLITE_MAX_PAGE_SIZE).contains(&page_size);
    if magic_ok && page_size_ok {
        return Ok(WalSidecarHeader::Valid);
    }
    Ok(WalSidecarHeader::Malformed { offset: if !magic_ok { 0 } else { 8 } })
}

fn probe_wal_sidecar(db_path: &Path) -> Result<(), EngineOpenError> {
    let WalSidecarHeader::Malformed { offset } = classify_wal_sidecar(db_path)? else {
        return Ok(());
    };
    Err(EngineOpenError::Corruption(CorruptionDetail {
        kind: CorruptionKind::WalReplayFailure,
        stage: OpenStage::WalReplay,
        locator: CorruptionLocator::FileOffset { offset },
        recovery_hint: RecoveryHint {
            code: "E_CORRUPT_WAL_REPLAY",
            doc_anchor: "design/recovery.md#wal-replay-failures",
        },
    }))
}

pub(crate) fn reject_legacy_shape(connection: &Connection) -> Result<(), EngineOpenError> {
    let has_legacy_table = table_exists(connection, "fathom_nodes")
        || table_exists(connection, "fathom_edges")
        || table_exists(connection, "fathom_chunks");
    if !has_legacy_table {
        return Ok(());
    }

    let seen =
        connection.query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0)).unwrap_or(0);
    Err(EngineOpenError::IncompatibleSchemaVersion { seen, supported: SCHEMA_VERSION })
}

pub(crate) fn validate_dependency_generation_on_open(
    connection: &Connection,
    schema_version: u32,
) -> Result<(), EngineOpenError> {
    if schema_version < SOURCE_DEPENDENCY_SCHEMA_VERSION {
        return Ok(());
    }
    let valid = (|| -> Result<bool, rusqlite::Error> {
        let value: String = connection.query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key=?1",
            [DEPENDENCY_GENERATION_KEY],
            |row| row.get(0),
        )?;
        let Some(generation) = canonical_dependency_generation(&value) else {
            return Ok(false);
        };
        let max_generation: i64 = connection.query_row(
            "SELECT COALESCE(MAX(registered_dependency_generation), 0) \
             FROM _fathomdb_source_dependencies",
            [],
            |row| row.get(0),
        )?;
        Ok(max_generation >= 0 && generation >= max_generation as u64)
    })()
    .unwrap_or(false);
    if valid {
        return Ok(());
    }
    Err(EngineOpenError::Corruption(CorruptionDetail {
        kind: CorruptionKind::SchemaInconsistent,
        stage: OpenStage::SchemaProbe,
        locator: CorruptionLocator::TableRow { table: "_fathomdb_open_state", rowid: 0 },
        recovery_hint: RecoveryHint {
            code: "E_CORRUPT_SCHEMA",
            doc_anchor: "design/recovery.md#schema-inconsistent",
        },
    }))
}

fn table_exists(connection: &Connection, table: &str) -> bool {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = ?1",
            [table],
            |_row| Ok(()),
        )
        .is_ok()
}

// EU-5b lock-flip: the engine's default embedder identity is now the
// pinned bge-small variant. Pre-existing 0.7.0 workspaces opened with
// `EmbedderChoice::Default` will fail-closed on identity mismatch per
// ADR-0.6.0-vector-identity-embedder-owned; callers can still hold an
// older noop profile by supplying `EmbedderChoice::Caller(NoopEmbedder)`.
pub(crate) const DEFAULT_EMBEDDER_NAME: &str = "fathomdb-bge-small-en-v1.5";
pub(crate) const DEFAULT_EMBEDDER_REVISION: &str = "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a";
pub(crate) const DEFAULT_EMBEDDER_DIMENSION: u32 = 384;

/// Identity name of the bge-small embedder. `OpenReport.embedder_mean_centering_required`
/// is `true` iff the live embedder identity reports this name. NoopEmbedder
/// is `false`. Lifted out as a constant so the EU-5b lock-flip (when the
/// engine's default identity becomes bge-small) is a single-line change.
///
/// TODO(EU-5b): when `DEFAULT_EMBEDDER_NAME` flips to this constant, the
/// Default path will populate `embedder_mean_centering_required = true`
/// without further engine work. Caller-supplied bge-small (rare today)
/// already does the right thing.
pub(crate) const BGE_SMALL_EMBEDDER_NAME: &str = "fathomdb-bge-small-en-v1.5";

static EXPLANATION_OPEN_NONCE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn mint_explanation_open_nonce() -> u128 {
    let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let sequence = EXPLANATION_OPEN_NONCE_SEQUENCE.fetch_add(1, Ordering::Relaxed) as u128;
    time.rotate_left(17) ^ sequence
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenReport {
    pub schema_version_before: u32,
    pub schema_version_after: u32,
    pub migration_steps: Vec<MigrationStepReport>,
    pub embedder_warmup_ms: u64,
    pub query_backend: &'static str,
    pub default_embedder: EmbedderIdentity,
    /// Total wall time the loader spent materializing default-embedder
    /// weights — covers HF GETs, sha256 verification, atomic rename,
    /// parent-dir fsync (POSIX), and cache directory writes. This is
    /// the "engine open paid by the embedder" envelope, useful for SLA
    /// budgeting; it is intentionally wider than just the bytes-flowing
    /// time so callers see the full first-use cost.
    ///
    /// `Some(ms)` when network bytes flowed (`bytes_downloaded > 0`);
    /// `None` for caller-supplied embedders (loader bypassed) and on
    /// full cache hits (no bytes flowed). For pure per-file network
    /// analysis, use the `DefaultEmbedderDownload` events on
    /// [`embedder_events`](Self::embedder_events) — each event carries
    /// the file's bytes + sha256 + cache path.
    pub embedder_download_ms: Option<u64>,
    /// Structured loader events (`dev/design/embedder.md` §7). Empty for
    /// caller-supplied embedders; populated from `LoadedWeights.events`
    /// for the Default path.
    pub embedder_events: Vec<EmbedderEvent>,
    /// Static identity capability (`dev/design/embedder.md` §0.6). True
    /// iff the live embedder identity is the bge-small default, which is
    /// the only identity that ships with the EU-5a2 mean-centering apply
    /// paths. `false` for `fathomdb-noop` and for any other
    /// caller-supplied identity. EU-5b's identity flip makes the Default
    /// path return `true` here.
    pub embedder_mean_centering_required: bool,
    /// Dynamic workspace state (`dev/design/embedder.md` §0.6). True iff
    /// `_fathomdb_embedder_profiles.mean_vec IS NOT NULL` for the default
    /// profile. EU-5a2 reads from the schema column added in migration
    /// step 10; the value is dimension-validated (§0.2) at open time
    /// and fails closed via `EmbedderIdentityMismatch` on drift.
    pub embedder_mean_vec_pinned: bool,
    /// 0.8.18 Slice 5 (#5 vector-equivalence probe, R-VEQ-6) — degraded-open
    /// observability. `true` iff the open-time #5 self-check re-embedded the 45
    /// committed probes and found a divergence beyond the frozen D4 floor (a
    /// Phase-1 mean-centered `embedding_bin` sign flip OR a Phase-2 un-centered
    /// L2 over `VECTOR_EQUIVALENCE_L2_EPSILON`). When `true`, `Engine::open`
    /// SUCCEEDED but every vector-dependent arm refuses at query time with
    /// `EngineError::VectorEquivalenceMismatch`; the text-only/FTS-only path stays
    /// serviceable. The state is RE-DERIVED at every open (the probe re-runs), so
    /// a reopen with a still-divergent backend stays degraded (never silently
    /// re-enables dense) and a reopen with a matching backend clears it.
    pub dense_disabled: bool,
    /// R-VEQ-6 — human-readable reason for `dense_disabled` (which representation
    /// tripped: P1 flip count or P2 L2). `None` when `dense_disabled == false`.
    pub dense_disabled_reason: Option<String>,
    /// Strict CPU/CUDA policy resolution used to construct the default
    /// embedder. `None` when the caller supplied an embedder or selected none.
    /// A present report is the one selection passed into default-embedder
    /// construction; forced CUDA failures return
    /// [`EngineOpenError::EmbedDevicePolicy`] rather than report CPU.
    pub embedder_device_resolution: Option<DeviceResolution>,
    /// Independent CPU/CUDA selection for the optional cross-encoder. This is
    /// never inferred from embedding-device state and makes no claim about
    /// database candidate retrieval or scoring.
    pub reranker_device_resolution: Option<RerankerDeviceResolution>,
    /// 0.8.23 Slice 80.6 (D-80.6-6, AC80-6, R80-13) — the in-process GPU
    /// allocation witness, when one was measured during this open.
    ///
    /// This carries the *retained record* of `fathomdb-embedder`'s
    /// `fathomdb.tegra-gpu-allocation-witness/v1`: the ordinal Candle actually
    /// retained, the driver-API UUID, and every raw number the verdict used
    /// (`free_before_bytes`, `free_after_bytes`, `total_bytes`, `delta_bytes`,
    /// `delta_floor_bytes`, plus the deliberate control allocation), so a
    /// reader re-derives the verdict rather than trusting it (R80-13). Its
    /// point is that the *installed artifact's own process* holds the
    /// evidence, rather than a sibling Rust process — which is what makes
    /// AC80-6's "in-process" clause as strong on Tegra as on x86_64.
    ///
    /// `None` is the normal case and means **no witness was measured** — never
    /// "a witness measured nothing". A zero, negative, or below-floor delta is
    /// a typed failure inside the witness (R80-12) and fails the open, so a
    /// zero-valued record is not reachable through this field.
    ///
    /// Populated only by an opted-in default-embedder open
    /// ([`ENV_GPU_ALLOCATION_WITNESS`]) on a CUDA-capable artifact whose
    /// device policy actually selected CUDA. It is deliberately opt-in: the
    /// witness holds a multi-gigabyte deliberate control allocation and loads
    /// the model a second time, which is evidence-run behavior and must not be
    /// imposed on ordinary opens (§ 12 non-goals).
    pub embedder_gpu_allocation_witness: Option<GpuAllocationWitness>,
}

#[derive(Debug)]
pub struct OpenedEngine {
    pub engine: Engine,
    pub report: OpenReport,
}

impl OpenedEngine {
    /// Return the process-wide SQLite configuration effective for this open.
    pub fn runtime_configuration(&self) -> RuntimeConfiguration {
        effective_runtime_configuration()
    }
}

/// EU-5b — loader-supplied open-time telemetry threaded into
/// `OpenReport.embedder_download_ms` and `OpenReport.embedder_events`.
#[derive(Clone, Debug)]
pub(crate) struct LoaderInfo {
    pub(crate) download_ms: Option<u64>,
    pub(crate) events: Vec<EmbedderEvent>,
    pub(crate) device_resolution: DeviceResolution,
    /// 0.8.23 Slice 80.6 (D-80.6-6) — the opted-in in-process GPU allocation
    /// witness. `None` for every path that measured none.
    pub(crate) gpu_allocation_witness: Option<GpuAllocationWitness>,
}

/// 0.8.23 Slice 80.6 (D-80.6-6) — opt-in switch for the in-process GPU
/// allocation witness carried on [`OpenReport::embedder_gpu_allocation_witness`].
///
/// Opt-in rather than automatic, and deliberately so. Producing the witness
/// costs a second load of the pinned model plus the multi-gigabyte deliberate
/// control allocation D-80.5-3 requires in order to prove the shared iGPU
/// memory counter is live and attributable. That is evidence-run behavior;
/// imposing it on every CUDA open would be exactly the runtime-contract change
/// § 12 of `dev/design/0.8.23-aarch64-tegra.md` rules out.
///
/// `1`/`true` enable it; unset, empty, `0`/`false` disable it. Any other value
/// is **rejected at open time** rather than read as "off", so a typo cannot
/// silently turn the evidence off — the same fail-closed posture R80-12 puts
/// on the witness itself.
pub const ENV_GPU_ALLOCATION_WITNESS: &str = "FATHOMDB_GPU_ALLOCATION_WITNESS";

/// Parse [`ENV_GPU_ALLOCATION_WITNESS`]. Pure, so every arm is testable on a
/// host with no GPU and on a build with no CUDA.
#[cfg(any(feature = "default-embedder", test))]
pub(crate) fn parse_gpu_allocation_witness_opt_in(raw: Option<&str>) -> Result<bool, String> {
    match raw.map(str::trim) {
        None | Some("") => Ok(false),
        Some(value) => match value.to_ascii_lowercase().as_str() {
            "1" | "true" => Ok(true),
            "0" | "false" => Ok(false),
            other => Err(format!(
                "{ENV_GPU_ALLOCATION_WITNESS} must be 1/true or 0/false, got {other:?}"
            )),
        },
    }
}

/// Measure the in-process GPU allocation witness when the operator asked for
/// one, and only then.
///
/// The contract is deliberately binary: opted in means this open carries a
/// witness or it fails, and not opted in means `None`. There is no third
/// outcome where the field is `None` while the operator believes a witness was
/// taken, because that is how a missing measurement becomes indistinguishable
/// from a measurement of zero (R80-12).
#[cfg(feature = "default-embedder")]
fn witness_gpu_allocation_if_requested(
    device_resolution: &DeviceResolution,
) -> Result<Option<GpuAllocationWitness>, EngineOpenError> {
    let raw = std::env::var(ENV_GPU_ALLOCATION_WITNESS).ok();
    let requested = parse_gpu_allocation_witness_opt_in(raw.as_deref())
        .map_err(|message| EngineOpenError::Embedder(RuntimeEmbedderError::Failed { message }))?;
    if !requested {
        return Ok(None);
    }
    run_requested_gpu_allocation_witness(device_resolution).map(Some)
}

/// Name a refusal rather than degrading to `None`, carrying the witness's own
/// stable failure tag so the caller reads the same vocabulary the retained
/// record uses.
#[cfg(feature = "default-embedder")]
fn gpu_allocation_witness_refusal(tag: &str, detail: &str) -> EngineOpenError {
    EngineOpenError::Embedder(RuntimeEmbedderError::Failed {
        message: format!(
            "{ENV_GPU_ALLOCATION_WITNESS} was requested but no GPU allocation witness \
             could be produced ({tag}): {detail}"
        ),
    })
}

#[cfg(all(feature = "default-embedder", feature = "embed-cuda"))]
fn run_requested_gpu_allocation_witness(
    device_resolution: &DeviceResolution,
) -> Result<GpuAllocationWitness, EngineOpenError> {
    use fathomdb_embedder::{AllocationWitnessConfig, EffectiveEmbedDevice};

    let ordinal = match &device_resolution.effective_device {
        EffectiveEmbedDevice::Cuda(info) => info.ordinal,
        EffectiveEmbedDevice::Cpu => {
            return Err(gpu_allocation_witness_refusal(
                "cpu_fallback",
                "the embedder device policy resolved to CPU, so there is no GPU \
                 allocation to witness",
            ));
        }
    };
    fathomdb_embedder::run_default_embedder_allocation_witness(AllocationWitnessConfig {
        ordinal,
        ..AllocationWitnessConfig::default()
    })
    .map_err(|error| gpu_allocation_witness_refusal(error.as_str(), &error.to_string()))
}

#[cfg(all(feature = "default-embedder", not(feature = "embed-cuda")))]
fn run_requested_gpu_allocation_witness(
    _device_resolution: &DeviceResolution,
) -> Result<GpuAllocationWitness, EngineOpenError> {
    Err(gpu_allocation_witness_refusal(
        "cuda_not_compiled",
        "this artifact has no CUDA provider compiled in",
    ))
}

/// Caller-facing selector for the embedder used by an opened engine
/// (`dev/design/embedder.md` §0).
#[derive(Clone)]
pub enum EmbedderChoice {
    /// Use the engine's default embedder. With the `default-embedder`
    /// Cargo feature enabled, this materializes a `CandleBgeEmbedder`
    /// via the EU-3 loader at `Engine::open`; on first use the loader
    /// downloads pinned bge-small-en-v1.5 weights from HuggingFace per
    /// `ADR-0.7.1-default-embedder-weight-fetch`. Without the feature,
    /// this returns `EmbedderError::Failed` directing the caller to
    /// rebuild with `--features default-embedder` or supply
    /// `EmbedderChoice::Caller`.
    Default,
    /// Caller supplies the embedder instance. The supplied embedder's
    /// `identity()` becomes the workspace's default-profile identity.
    Caller(Arc<dyn Embedder>),
    /// Caller supplies an embedder plus its already-resolved device outcome.
    ///
    /// The resolution is recorded in [`OpenReport::embedder_device_resolution`]
    /// exactly once. This is for opt-in embedders, such as ONNX Runtime, whose
    /// final CUDA/CPU outcome is known only after their own construction.
    CallerWithDeviceResolution {
        /// The caller-supplied runtime embedder.
        embedder: Arc<dyn Embedder>,
        /// The embedder's final CPU/CUDA resolution.
        device_resolution: DeviceResolution,
    },
    /// No embedder configured. Engine opens; subsequent vector writes
    /// fail with `EngineError::EmbedderNotConfigured`. Useful for
    /// read-only or canonical-only flows.
    None,
}

fn default_embedder_identity() -> EmbedderIdentity {
    EmbedderIdentity::new(
        DEFAULT_EMBEDDER_NAME,
        DEFAULT_EMBEDDER_REVISION,
        DEFAULT_EMBEDDER_DIMENSION,
    )
}

fn check_embedder_profile(
    connection: &Connection,
    supplied: &EmbedderIdentity,
) -> Result<bool, EngineOpenError> {
    // Returns `true` iff `_fathomdb_embedder_profiles.mean_vec IS NOT NULL`
    // for the default profile (and its byte length matches `4 * dimension`
    // per `dev/design/embedder.md` §0.2). EU-5a2: column lands in step 10.
    let mut statement = match connection.prepare(
        "SELECT name, revision, dimension, mean_vec FROM _fathomdb_embedder_profiles WHERE profile = 'default'",
    ) {
        Ok(statement) => statement,
        Err(_) => return Ok(false),
    };
    let mut rows = statement.query([]).map_err(|_| {
        EngineOpenError::Corruption(CorruptionDetail {
            kind: CorruptionKind::EmbedderIdentityDrift,
            stage: OpenStage::EmbedderIdentity,
            locator: CorruptionLocator::OpaqueSqliteError { sqlite_extended_code: 0 },
            recovery_hint: RecoveryHint {
                code: "E_CORRUPT_EMBEDDER_IDENTITY",
                doc_anchor: "design/recovery.md#embedder-identity-drift",
            },
        })
    })?;

    let Some(row) = rows.next().map_err(|_| {
        EngineOpenError::Corruption(CorruptionDetail {
            kind: CorruptionKind::EmbedderIdentityDrift,
            stage: OpenStage::EmbedderIdentity,
            locator: CorruptionLocator::OpaqueSqliteError { sqlite_extended_code: 0 },
            recovery_hint: RecoveryHint {
                code: "E_CORRUPT_EMBEDDER_IDENTITY",
                doc_anchor: "design/recovery.md#embedder-identity-drift",
            },
        })
    })?
    else {
        connection
            .execute(
                "INSERT INTO _fathomdb_embedder_profiles(profile, name, revision, dimension)
                 VALUES(?1, ?2, ?3, ?4)",
                params![
                    DEFAULT_VECTOR_PROFILE,
                    supplied.name,
                    supplied.revision,
                    supplied.dimension
                ],
            )
            .map_err(|_| EngineOpenError::Io {
                message: "could not persist embedder profile".to_string(),
            })?;
        return Ok(false);
    };

    let stored_name = row.get::<_, String>(0).map_err(|_| {
        EngineOpenError::Corruption(CorruptionDetail {
            kind: CorruptionKind::EmbedderIdentityDrift,
            stage: OpenStage::EmbedderIdentity,
            locator: CorruptionLocator::TableRow { table: "_fathomdb_embedder_profiles", rowid: 0 },
            recovery_hint: RecoveryHint {
                code: "E_CORRUPT_EMBEDDER_IDENTITY",
                doc_anchor: "design/recovery.md#embedder-identity-drift",
            },
        })
    })?;
    let stored_revision = row.get::<_, String>(1).map_err(|_| {
        EngineOpenError::Corruption(CorruptionDetail {
            kind: CorruptionKind::EmbedderIdentityDrift,
            stage: OpenStage::EmbedderIdentity,
            locator: CorruptionLocator::TableRow { table: "_fathomdb_embedder_profiles", rowid: 0 },
            recovery_hint: RecoveryHint {
                code: "E_CORRUPT_EMBEDDER_IDENTITY",
                doc_anchor: "design/recovery.md#embedder-identity-drift",
            },
        })
    })?;
    let dimension = row.get::<_, u32>(2).map_err(|_| {
        EngineOpenError::Corruption(CorruptionDetail {
            kind: CorruptionKind::EmbedderIdentityDrift,
            stage: OpenStage::EmbedderIdentity,
            locator: CorruptionLocator::TableRow { table: "_fathomdb_embedder_profiles", rowid: 0 },
            recovery_hint: RecoveryHint {
                code: "E_CORRUPT_EMBEDDER_IDENTITY",
                doc_anchor: "design/recovery.md#embedder-identity-drift",
            },
        })
    })?;

    let stored = EmbedderIdentity::new(stored_name, stored_revision, dimension);

    if stored.name != supplied.name || stored.revision != supplied.revision {
        return Err(EngineOpenError::EmbedderIdentityMismatch {
            stored,
            supplied: supplied.clone(),
        });
    }
    if dimension != supplied.dimension {
        return Err(EngineOpenError::EmbedderDimensionMismatch {
            stored: dimension,
            supplied: supplied.dimension,
        });
    }

    // EU-5a2 / `dev/design/embedder.md` §0.2 invariant: if `mean_vec` is
    // populated, byte length MUST equal `4 * dimension`. Debug builds
    // assert; release builds fail closed via EmbedderIdentityMismatch
    // (the same fail-closed channel the rest of profile drift takes).
    let mean_vec: Option<Vec<u8>> = row.get::<_, Option<Vec<u8>>>(3).map_err(|_| {
        EngineOpenError::Corruption(CorruptionDetail {
            kind: CorruptionKind::EmbedderIdentityDrift,
            stage: OpenStage::EmbedderIdentity,
            locator: CorruptionLocator::TableRow { table: "_fathomdb_embedder_profiles", rowid: 0 },
            recovery_hint: RecoveryHint {
                code: "E_CORRUPT_EMBEDDER_IDENTITY",
                doc_anchor: "design/recovery.md#embedder-identity-drift",
            },
        })
    })?;
    let pinned = match mean_vec {
        Some(bytes) => {
            let expected_len = (dimension as usize).saturating_mul(4);
            // `dev/design/embedder.md` §0.2 invariant: when populated,
            // `mean_vec` byte length MUST equal `4 * dimension`. Fail
            // closed via the existing identity-drift channel in both
            // debug and release builds — tests deliberately poke
            // malformed values to exercise this branch.
            if bytes.len() != expected_len {
                return Err(EngineOpenError::EmbedderIdentityMismatch {
                    stored,
                    supplied: supplied.clone(),
                });
            }
            true
        }
        None => false,
    };

    Ok(pinned)
}

pub(crate) fn read_only_sqlite_uri(path: &Path) -> String {
    sqlite_uri(path, "mode=ro")
}

pub(crate) fn sqlite_uri(path: &Path, query: &str) -> String {
    let mut uri = String::from("file:");
    for byte in path.as_os_str().as_encoded_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                uri.push(char::from(*byte))
            }
            byte => {
                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                uri.push('%');
                uri.push(char::from(HEX[usize::from(byte >> 4)]));
                uri.push(char::from(HEX[usize::from(byte & 0x0f)]));
            }
        }
    }
    uri.push('?');
    uri.push_str(query);
    uri
}
