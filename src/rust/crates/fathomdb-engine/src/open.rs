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
