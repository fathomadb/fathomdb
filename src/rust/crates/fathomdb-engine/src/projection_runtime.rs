use super::*;

#[derive(Clone, Debug)]
pub(crate) struct ProjectionJob {
    pub(crate) cursor: u64,
    pub(crate) kind: String,
    pub(crate) body: String,
    pub(crate) generation_id: ProjectionGenerationId,
}

#[derive(Debug, Default)]
pub(crate) struct ProjectionRuntimeState {
    pub(crate) active_jobs: usize,
    pub(crate) queued_jobs: usize,
    pub(crate) frozen: bool,
    pub(crate) pending_scan: bool,
    pub(crate) next_temporal_scan_epoch_s: Option<i64>,
    pub(crate) stopping: bool,
    pub(crate) in_flight: BTreeSet<u64>,
}

pub(crate) struct ProjectionRuntimeShared {
    pub(crate) path: PathBuf,
    pub(crate) worker_count: usize,
    pub(crate) admission_capacity: usize,
    pub(crate) embedder: Option<Arc<dyn Embedder>>,
    pub(crate) embedder_identity: EmbedderIdentity,
    /// Host-owned lifecycle diagnostics for worker failures, which occur on
    /// background connections rather than through an `Engine` method call.
    pub(crate) subscribers: Arc<lifecycle::SubscriberRegistry>,
    pub(crate) wal_attribution: Arc<WalAttributionCollector>,
    #[cfg(any(test, feature = "test-hooks"))]
    pub(crate) managed_connections: Arc<ManagedConnectionRegistry>,
    #[cfg(any(test, feature = "test-hooks"))]
    pub(crate) runtime_inventory_request: Mutex<Option<RuntimeConnectionInventoryRequest>>,
    #[cfg(any(test, feature = "test-hooks"))]
    pub(crate) runtime_native_state_request: Mutex<Option<RuntimeNativeStateRequest>>,
    pub(crate) state: Mutex<ProjectionRuntimeState>,
    pub(crate) state_cvar: Condvar,
    pub(crate) queue: Mutex<VecDeque<ProjectionJob>>,
    pub(crate) queue_cvar: Condvar,
    pub(crate) retry_delays_ms: Mutex<Vec<u64>>,
    pub(crate) embed_dispatch: Arc<EmbedDispatcher>,
    /// EU-5b — streaming mean accumulator for the per-workspace mean
    /// pinning lifecycle (`dev/design/embedder.md` §0.3). `Some(_)` iff
    /// the identity is MC-required AND no mean has been pinned yet on
    /// disk. The accumulator graduates to `None` after the at-pin
    /// commit; subsequent docs feed nothing.
    pub(crate) mean_accumulator: Mutex<Option<MeanAccumulator>>,
    /// EU-5b — `MeanVecPinned` events queued by the projection-commit
    /// transaction for the next test-seam drain. Production callers
    /// consume these via the `OpenReport.embedder_events` channel; the
    /// drain seam is `Engine::drain_mean_centering_events_for_test`.
    pub(crate) pending_events: Mutex<Vec<EmbedderEvent>>,
    /// EU-5f — serializes the body of `commit_projection_outcomes` across
    /// the configured projection worker connections. Each worker commits on
    /// its own connection; holding this gate for the whole commit makes the
    /// commit transactions totally ordered, which is what makes the at-pin
    /// re-quantize pass provably complete (every row is wholly before or
    /// after the unique pin tx, so none can survive un-centered). Embedding
    /// (`run_projection_job`) runs OUTSIDE the gate and stays parallel.
    pub(crate) commit_gate: Mutex<()>,
    /// Test-only vector-candidate fanout override for the search hot path.
    /// It defaults to `SEARCH_RERANK_LIMIT` (10); the test seam may raise it
    /// so recall tests can inspect deeper vector candidates. It never changes
    /// the caller-requested final result limit or visible result cardinality.
    pub(crate) search_limit_override: AtomicUsize,
    /// Slice 10 / G12-recency — dedicated recency-reweight flag, **off by
    /// default** (NOT `fusion_mode`). When set, fused hits are reweighted toward
    /// the more recent `write_cursor` AFTER bit-KNN. Flipped by the
    /// `set_recency_reweight_enabled_for_test` seam; no production toggle yet.
    pub(crate) recency_reweight_enabled: AtomicBool,
    /// 0.8.16 Slice 5 / F9 — dedicated importance/confidence reweight flag,
    /// **off by default** (mirrors `recency_reweight_enabled`; NOT `fusion_mode`).
    /// When set, fused hits are multiplicatively reweighted by node `importance`
    /// (`canonical_nodes.importance`) and edge `confidence`
    /// (`canonical_edges.confidence`) AFTER bit-KNN + RRF fusion — `NULL ⇒ neutral
    /// (1.0)`. Flipped by `set_importance_reweight_enabled_for_test`; no production
    /// toggle yet (F9 ships OFF-by-default as a MECHANISM, no eval-quality claim).
    pub(crate) importance_reweight_enabled: AtomicBool,
    /// GA-2 / Slice-40 (◆ B-1) measurement seam, **off by default**. When set,
    /// `read_search_in_tx` returns the pre-fusion VECTOR-branch ranking
    /// (bit-KNN K=192 + f32 rerank) verbatim — the ANN-quantization fidelity
    /// signal — INSTEAD of the unconditional RRF-fused result. This changes
    /// nothing for any production caller (the flag is never set outside the
    /// `eu7` recall harness via `set_vector_stage_only_for_test`); it does NOT
    /// reintroduce a `fusion_mode` knob (RRF stays unconditional) and does NOT
    /// alter `fuse_rrf` / `rerank_fused` / recency. It only lets the AC-075
    /// recall gate measure ANN+ vector top-10 vs the exact-f32 VECTOR top-10
    /// ground truth in isolation (the quantization-FIDELITY axis the 0.90 floor
    /// is defined to measure), not the hybrid `search()` output.
    pub(crate) vector_stage_only_for_test: AtomicBool,
    /// 0.7.2 PR-2b — debug-only fault injection: when set, `recompute_mean_in_tx`
    /// errors AFTER writing `mean_vec` but BEFORE finishing the re-quantize
    /// pass, so the crash-atomicity test can prove the whole recompute rolls
    /// back (no half-recentered corpus). One-shot (cleared on consume).
    #[cfg(debug_assertions)]
    pub(crate) force_recompute_failure: AtomicBool,
    /// TC-91 — one-shot worker-commit fault seam. `0` is disabled, `1`
    /// requests a synthetic SQLite busy error, and `2` a rusqlite-layer
    /// storage error immediately before commit.
    /// Kept entirely in the runtime and compiled only for tests.
    #[cfg(debug_assertions)]
    pub(crate) force_projection_commit_failure: AtomicUsize,
    /// TC-91 test-only rendezvous after error reporting and before worker
    /// cleanup. It proves a stop in that window leaves canonical pending work
    /// for the next open rather than relying on an in-memory retry queue.
    #[cfg(debug_assertions)]
    pub(crate) projection_commit_failure_pause: Mutex<Option<(Arc<Barrier>, Arc<Barrier>)>>,
    /// Slice 65: test-only rendezvous after a projection worker acquires its
    /// real `BEGIN IMMEDIATE` transaction. It must not report queued work as a
    /// live transaction.
    #[cfg(debug_assertions)]
    pub(crate) projection_worker_transaction_pause: Mutex<Option<(mpsc::Sender<()>, Receiver<()>)>>,
    /// Slice 40: one-shot rendezvous after dispatch has queued a captured-
    /// generation job and before any worker removes it from the queue.
    #[cfg(feature = "test-hooks")]
    pub(crate) projection_worker_queued_pause: Mutex<Option<(Arc<Barrier>, Arc<Barrier>)>>,
    /// Slice 40: one-shot rendezvous after embedding and immediately before
    /// the worker attempts to acquire SQLite's write lock.
    #[cfg(feature = "test-hooks")]
    pub(crate) projection_worker_before_write_lock_pause:
        Mutex<Option<(Arc<Barrier>, Arc<Barrier>)>>,
    /// TC-91 test-only acknowledgement after `stopping` is set and before a
    /// close joins workers, used with `projection_commit_failure_pause`.
    #[cfg(debug_assertions)]
    projection_stop_ack: Mutex<Option<Arc<Barrier>>>,
}

impl std::fmt::Debug for ProjectionRuntimeShared {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectionRuntimeShared")
            .field("path", &self.path)
            .field("embedder_identity", &self.embedder_identity)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(crate) struct ProjectionRuntime {
    pub(crate) shared: Arc<ProjectionRuntimeShared>,
    dispatcher: Mutex<Option<JoinHandle<()>>>,
    workers: Mutex<Vec<JoinHandle<()>>>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ProjectionRuntimeStartupRole {
    Dispatcher(usize),
    Worker(usize),
}

impl ProjectionRuntimeStartupRole {
    fn label(self) -> String {
        match self {
            Self::Dispatcher(index) => format!("dispatcher:{index}"),
            Self::Worker(index) => format!("worker:{index}"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ProjectionRuntimeStartupReport {
    pub(crate) role: ProjectionRuntimeStartupRole,
    pub(crate) stage: Result<(), &'static str>,
}

#[derive(Debug)]
pub(crate) enum ProjectionRuntimeStartupMessage {
    Setup(ProjectionRuntimeStartupReport),
    ServiceReady(ProjectionRuntimeStartupRole),
}

#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum ProjectionRuntimeStartupFaultForTest {
    SetupFailure(ProjectionRuntimeStartupRole),
    DuplicateReport {
        role: ProjectionRuntimeStartupRole,
        reported_as: ProjectionRuntimeStartupRole,
    },
    MissingReport(ProjectionRuntimeStartupRole),
    StallUntilStop(ProjectionRuntimeStartupRole),
    ExitAfterReport(ProjectionRuntimeStartupRole),
}

#[cfg(test)]
impl ProjectionRuntimeStartupFaultForTest {
    pub(crate) fn targets(self, role: ProjectionRuntimeStartupRole) -> bool {
        match self {
            Self::SetupFailure(target)
            | Self::MissingReport(target)
            | Self::StallUntilStop(target)
            | Self::ExitAfterReport(target) => target == role,
            Self::DuplicateReport { role: target, .. } => target == role,
        }
    }
}

fn missing_projection_runtime_roles(
    expected: &BTreeSet<ProjectionRuntimeStartupRole>,
    observed: &BTreeSet<ProjectionRuntimeStartupRole>,
) -> String {
    expected.difference(observed).map(|role| role.label()).collect::<Vec<_>>().join(",")
}

fn projection_startup_roles(worker_count: usize) -> BTreeSet<ProjectionRuntimeStartupRole> {
    std::iter::once(ProjectionRuntimeStartupRole::Dispatcher(0))
        .chain((0..worker_count).map(ProjectionRuntimeStartupRole::Worker))
        .collect()
}

#[cfg(any(test, feature = "test-hooks"))]
pub(crate) fn projection_wal_roles(worker_count: usize) -> BTreeSet<(WalAttributionRole, usize)> {
    std::iter::once((WalAttributionRole::ProjectionDispatcher, 0))
        .chain((0..worker_count).map(|index| (WalAttributionRole::ProjectionWorker, index)))
        .collect()
}

impl ProjectionRuntime {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        path: PathBuf,
        embedder: Option<Arc<dyn Embedder>>,
        embed_dispatch: Arc<EmbedDispatcher>,
        embedder_identity: EmbedderIdentity,
        mean_already_pinned: bool,
        subscribers: Arc<lifecycle::SubscriberRegistry>,
        wal_attribution: Arc<WalAttributionCollector>,
        config: ResolvedRuntimeConfiguration,
        #[cfg(any(test, feature = "test-hooks"))] managed_connections: Arc<
            ManagedConnectionRegistry,
        >,
    ) -> Result<Self, EngineOpenError> {
        Self::new_with_startup_control(
            path,
            embedder,
            embed_dispatch,
            embedder_identity,
            mean_already_pinned,
            subscribers,
            wal_attribution,
            config,
            #[cfg(any(test, feature = "test-hooks"))]
            managed_connections,
            PROJECTION_RUNTIME_STARTUP_TIMEOUT,
            #[cfg(test)]
            None,
        )
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_for_test(
        path: PathBuf,
        embedder: Option<Arc<dyn Embedder>>,
        embedder_identity: EmbedderIdentity,
        mean_already_pinned: bool,
        subscribers: Arc<lifecycle::SubscriberRegistry>,
        wal_attribution: Arc<WalAttributionCollector>,
        config: ResolvedRuntimeConfiguration,
        managed_connections: Arc<ManagedConnectionRegistry>,
        startup_timeout: Duration,
        startup_fault: Option<ProjectionRuntimeStartupFaultForTest>,
    ) -> Result<Self, EngineOpenError> {
        let embed_dispatch = Arc::new(
            EmbedDispatcher::new(
                embedder.clone(),
                config.embedder_pool_size,
                Duration::from_millis(config.embedder_call_timeout_ms),
            )
            .map_err(|error| EngineOpenError::Io { message: error.to_string() })?,
        );
        Self::new_with_startup_control(
            path,
            embedder,
            embed_dispatch,
            embedder_identity,
            mean_already_pinned,
            subscribers,
            wal_attribution,
            config,
            managed_connections,
            startup_timeout,
            startup_fault,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_startup_control(
        path: PathBuf,
        embedder: Option<Arc<dyn Embedder>>,
        embed_dispatch: Arc<EmbedDispatcher>,
        embedder_identity: EmbedderIdentity,
        mean_already_pinned: bool,
        subscribers: Arc<lifecycle::SubscriberRegistry>,
        wal_attribution: Arc<WalAttributionCollector>,
        config: ResolvedRuntimeConfiguration,
        #[cfg(any(test, feature = "test-hooks"))] managed_connections: Arc<
            ManagedConnectionRegistry,
        >,
        startup_timeout: Duration,
        #[cfg(test)] startup_fault: Option<ProjectionRuntimeStartupFaultForTest>,
    ) -> Result<Self, EngineOpenError> {
        // EU-5b/EU-5f — only allocate the streaming accumulator when the
        // workspace's identity is MC-required AND no mean has been pinned
        // yet on disk. Allocating it for an already-pinned workspace would
        // let a later 256-doc run RE-pin and overwrite the compute-once
        // mean (violating `dev/design/embedder.md` §0.3). Other identities
        // pay no memory cost (`Option::None`).
        let mc_required = identity_requires_mean_centering(&embedder_identity);
        let mean_accumulator = if mc_required && !mean_already_pinned {
            Some(MeanAccumulator::new(embedder_identity.dimension as usize))
        } else {
            None
        };
        let shared = Arc::new(ProjectionRuntimeShared {
            path,
            worker_count: config.scheduler_runtime_threads,
            admission_capacity: config.projection_admission_capacity,
            embedder,
            embed_dispatch,
            embedder_identity,
            subscribers,
            wal_attribution,
            #[cfg(any(test, feature = "test-hooks"))]
            managed_connections,
            #[cfg(any(test, feature = "test-hooks"))]
            runtime_inventory_request: Mutex::new(None),
            #[cfg(any(test, feature = "test-hooks"))]
            runtime_native_state_request: Mutex::new(None),
            state: Mutex::new(ProjectionRuntimeState::default()),
            state_cvar: Condvar::new(),
            queue: Mutex::new(VecDeque::new()),
            queue_cvar: Condvar::new(),
            retry_delays_ms: Mutex::new(DEFAULT_PROJECTION_RETRY_DELAYS_MS.to_vec()),
            mean_accumulator: Mutex::new(mean_accumulator),
            pending_events: Mutex::new(Vec::new()),
            commit_gate: Mutex::new(()),
            search_limit_override: AtomicUsize::new(SEARCH_RERANK_LIMIT),
            recency_reweight_enabled: AtomicBool::new(false),
            importance_reweight_enabled: AtomicBool::new(false),
            vector_stage_only_for_test: AtomicBool::new(false),
            #[cfg(debug_assertions)]
            force_recompute_failure: AtomicBool::new(false),
            #[cfg(debug_assertions)]
            force_projection_commit_failure: AtomicUsize::new(0),
            #[cfg(debug_assertions)]
            projection_commit_failure_pause: Mutex::new(None),
            #[cfg(debug_assertions)]
            projection_worker_transaction_pause: Mutex::new(None),
            #[cfg(feature = "test-hooks")]
            projection_worker_queued_pause: Mutex::new(None),
            #[cfg(feature = "test-hooks")]
            projection_worker_before_write_lock_pause: Mutex::new(None),
            #[cfg(debug_assertions)]
            projection_stop_ack: Mutex::new(None),
        });

        let (startup_send, startup_receive) = mpsc::channel();
        let mut service_requests = BTreeMap::new();
        let (dispatcher_service_send, dispatcher_service_receive) = mpsc::channel();
        service_requests
            .insert(ProjectionRuntimeStartupRole::Dispatcher(0), dispatcher_service_send);
        let dispatcher_shared = Arc::clone(&shared);
        let dispatcher_startup = startup_send.clone();
        let dispatcher = thread::Builder::new()
            .name("fathomdb-projection-dispatcher-0".to_string())
            .spawn(move || {
                projection_dispatcher_loop(
                    dispatcher_shared,
                    0,
                    dispatcher_startup,
                    dispatcher_service_receive,
                    #[cfg(test)]
                    startup_fault,
                )
            })
            .map_err(|_| EngineOpenError::Io {
                message: "could not start projection dispatcher".to_string(),
            })?;

        let mut workers = Vec::with_capacity(shared.worker_count);
        for worker_idx in 0..shared.worker_count {
            let (worker_service_send, worker_service_receive) = mpsc::channel();
            service_requests
                .insert(ProjectionRuntimeStartupRole::Worker(worker_idx), worker_service_send);
            let worker_shared = Arc::clone(&shared);
            let worker_startup = startup_send.clone();
            let spawn_result = thread::Builder::new()
                .name(format!("fathomdb-projection-worker-{worker_idx}"))
                .spawn(move || {
                    projection_worker_loop(
                        worker_shared,
                        worker_idx,
                        worker_startup,
                        worker_service_receive,
                        #[cfg(test)]
                        startup_fault,
                    )
                });
            match spawn_result {
                Ok(worker) => workers.push(worker),
                Err(_) => {
                    drop(startup_send);
                    drop(service_requests);
                    let runtime = Self {
                        shared,
                        dispatcher: Mutex::new(Some(dispatcher)),
                        workers: Mutex::new(workers),
                    };
                    runtime.stop();
                    return Err(EngineOpenError::Io {
                        message: format!("could not start projection worker:{worker_idx}"),
                    });
                }
            }
        }
        drop(startup_send);

        let runtime =
            Self { shared, dispatcher: Mutex::new(Some(dispatcher)), workers: Mutex::new(workers) };
        if let Err(error) =
            runtime.await_startup(startup_receive, service_requests, startup_timeout)
        {
            runtime.stop();
            return Err(error);
        }
        Ok(runtime)
    }

    fn await_startup(
        &self,
        startup: Receiver<ProjectionRuntimeStartupMessage>,
        service_requests: BTreeMap<ProjectionRuntimeStartupRole, mpsc::Sender<()>>,
        timeout: Duration,
    ) -> Result<(), EngineOpenError> {
        let expected = projection_startup_roles(self.shared.worker_count);
        let mut observed = BTreeSet::new();
        let deadline = Instant::now() + timeout;

        while observed.len() < expected.len() {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let message = match startup.recv_timeout(remaining) {
                Ok(message) => message,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(EngineOpenError::Io {
                        message: format!(
                            "projection runtime startup timed out after {}ms; missing roles: {}",
                            timeout.as_millis(),
                            missing_projection_runtime_roles(&expected, &observed)
                        ),
                    });
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(EngineOpenError::Io {
                        message: format!(
                            "projection runtime startup disconnected; missing roles: {}",
                            missing_projection_runtime_roles(&expected, &observed)
                        ),
                    });
                }
            };
            let ProjectionRuntimeStartupMessage::Setup(report) = message else {
                return Err(EngineOpenError::Io {
                    message: "projection runtime reported service readiness before setup completed"
                        .to_string(),
                });
            };
            if !expected.contains(&report.role) {
                return Err(EngineOpenError::Io {
                    message: format!(
                        "projection runtime startup reported unexpected role {}",
                        report.role.label()
                    ),
                });
            }
            if !observed.insert(report.role) {
                return Err(EngineOpenError::Io {
                    message: format!(
                        "projection runtime startup reported duplicate role {}",
                        report.role.label()
                    ),
                });
            }
            if let Err(stage) = report.stage {
                return Err(EngineOpenError::Io {
                    message: format!(
                        "projection runtime {} failed during {stage}",
                        report.role.label()
                    ),
                });
            }
        }

        for (role, request) in service_requests {
            request.send(()).map_err(|_| EngineOpenError::Io {
                message: format!(
                    "projection runtime service request disconnected for {}",
                    role.label()
                ),
            })?;
        }

        let mut service_ready = BTreeSet::new();
        while service_ready.len() < expected.len() {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let message = match startup.recv_timeout(remaining) {
                Ok(message) => message,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(EngineOpenError::Io {
                        message: format!(
                            "projection runtime service readiness timed out after {}ms; missing roles: {}",
                            timeout.as_millis(),
                            missing_projection_runtime_roles(&expected, &service_ready)
                        ),
                    });
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(EngineOpenError::Io {
                        message: format!(
                            "projection runtime service readiness disconnected; missing roles: {}",
                            missing_projection_runtime_roles(&expected, &service_ready)
                        ),
                    });
                }
            };
            let ProjectionRuntimeStartupMessage::ServiceReady(role) = message else {
                return Err(EngineOpenError::Io {
                    message: "projection runtime reported an extra setup message during service readiness"
                        .to_string(),
                });
            };
            if !expected.contains(&role) {
                return Err(EngineOpenError::Io {
                    message: format!(
                        "projection runtime service readiness reported unexpected role {}",
                        role.label()
                    ),
                });
            }
            if !service_ready.insert(role) {
                return Err(EngineOpenError::Io {
                    message: format!(
                        "projection runtime service readiness reported duplicate role {}",
                        role.label()
                    ),
                });
            }
        }
        Ok(())
    }

    pub(crate) fn notify_new_work(&self) {
        if let Ok(mut state) = self.shared.state.lock() {
            state.pending_scan = true;
            state.next_temporal_scan_epoch_s = None;
            self.shared.state_cvar.notify_all();
        }
    }

    #[cfg(any(test, feature = "test-hooks"))]
    pub(crate) fn report_runtime_connection_inventory_for_test(
        &self,
    ) -> Result<Vec<(WalAttributionRole, usize, bool)>, &'static str> {
        let pending = projection_wal_roles(self.shared.worker_count);
        let expected_count = pending.len();
        let (respond, received) = mpsc::sync_channel(expected_count);
        let mut request =
            self.shared.runtime_inventory_request.lock().map_err(|_| "request_lock")?;
        if request.is_some() {
            return Err("request_already_active");
        }
        *request = Some(RuntimeConnectionInventoryRequest { pending, respond });
        drop(request);
        self.shared.state_cvar.notify_all();
        self.shared.queue_cvar.notify_all();

        let mut facts = Vec::with_capacity(expected_count);
        for _ in 0..expected_count {
            facts.push(
                received
                    .recv_timeout(Duration::from_secs(2))
                    .map_err(|_| "runtime_reply_timeout")?,
            );
        }
        Ok(facts)
    }

    #[cfg(any(test, feature = "test-hooks"))]
    pub(crate) fn report_runtime_native_state_inventory_for_test(
        &self,
    ) -> Result<Vec<NativeConnectionStateFact>, &'static str> {
        let pending = projection_wal_roles(self.shared.worker_count);
        let expected_count = pending.len();
        let (respond, received) = mpsc::sync_channel(expected_count);
        let mut request =
            self.shared.runtime_native_state_request.lock().map_err(|_| "native_request_lock")?;
        if request.is_some() {
            return Err("native_request_already_active");
        }
        *request = Some(RuntimeNativeStateRequest { pending, respond });
        drop(request);
        self.shared.state_cvar.notify_all();
        self.shared.queue_cvar.notify_all();

        let result = (|| {
            let mut facts = Vec::with_capacity(expected_count);
            for _ in 0..expected_count {
                facts.push(
                    received
                        .recv_timeout(Duration::from_millis(250))
                        .map_err(|_| "runtime_native_reply_timeout")?,
                );
            }
            Ok(facts)
        })();
        if result.is_err() {
            if let Ok(mut request) = self.shared.runtime_native_state_request.lock() {
                *request = None;
            }
        }
        result
    }

    pub(crate) fn set_frozen(&self, frozen: bool) {
        if let Ok(mut state) = self.shared.state.lock() {
            state.frozen = frozen;
            if !frozen {
                state.pending_scan = true;
                state.next_temporal_scan_epoch_s = None;
            }
            self.shared.state_cvar.notify_all();
        }
    }

    pub(crate) fn pending_scan_for_test(&self) -> bool {
        self.shared.state.lock().map(|state| state.pending_scan).unwrap_or(true)
    }

    pub(crate) fn wait_for_idle(
        &self,
        timeout_ms: u64,
        mut has_pending_projection_work: impl FnMut() -> Option<bool>,
    ) -> bool {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let mut state = match self.shared.state.lock() {
            Ok(state) => state,
            Err(_) => return false,
        };
        loop {
            let mut retry_pending_probe = false;
            if state.active_jobs == 0 && state.queued_jobs == 0 {
                drop(state);
                match has_pending_projection_work() {
                    Some(false) => return true,
                    Some(true) => {}
                    None => retry_pending_probe = true,
                }
                state = match self.shared.state.lock() {
                    Ok(state) => state,
                    Err(_) => return false,
                };
            }
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            let mut wait = deadline.saturating_duration_since(now);
            if retry_pending_probe {
                wait = wait.min(Duration::from_millis(1));
            }
            let Ok((next_state, _)) = self.shared.state_cvar.wait_timeout(state, wait) else {
                return false;
            };
            state = next_state;
        }
    }

    /// Wait until the runtime has no queued or active jobs, deliberately
    /// without consulting durable pending work. Erasure uses this only after it
    /// has frozen the dispatcher for a session with no configured embedder:
    /// those pending rows cannot be projected in that session and will instead
    /// be deleted by the erasure transaction.
    pub(crate) fn wait_for_workers_idle(&self, timeout_ms: u64) -> bool {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let mut state = match self.shared.state.lock() {
            Ok(state) => state,
            Err(_) => return false,
        };
        loop {
            if state.active_jobs == 0 && state.queued_jobs == 0 {
                return true;
            }
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            let wait = deadline.saturating_duration_since(now);
            let Ok((next_state, _)) = self.shared.state_cvar.wait_timeout(state, wait) else {
                return false;
            };
            state = next_state;
        }
    }

    pub(crate) fn set_retry_delays_for_test(&self, delays_ms: &[u64]) {
        if let Ok(mut delays) = self.shared.retry_delays_ms.lock() {
            *delays = delays_ms.to_vec();
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn force_next_projection_commit_failure_for_test(&self) {
        self.shared.force_projection_commit_failure.store(1, Ordering::SeqCst);
    }

    #[cfg(debug_assertions)]
    pub(crate) fn force_next_projection_storage_failure_for_test(&self) {
        self.shared.force_projection_commit_failure.store(2, Ordering::SeqCst);
    }

    #[cfg(debug_assertions)]
    pub(crate) fn pause_projection_commit_failure_cleanup_for_test(
        &self,
        reported: Arc<Barrier>,
        release: Arc<Barrier>,
    ) {
        *self
            .shared
            .projection_commit_failure_pause
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some((reported, release));
    }

    #[cfg(debug_assertions)]
    #[allow(dead_code)]
    pub(crate) fn pause_projection_worker_after_wal_transaction_for_test(
        &self,
    ) -> ProjectionWorkerTransactionPauseForTest {
        let (transaction_ready_tx, transaction_ready_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        *self
            .shared
            .projection_worker_transaction_pause
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) =
            Some((transaction_ready_tx, release_rx));
        ProjectionWorkerTransactionPauseForTest {
            ready: Some(transaction_ready_rx),
            release: Some(release_tx),
            ready_observed: false,
        }
    }

    #[cfg(feature = "test-hooks")]
    pub(crate) fn pause_projection_worker_while_queued_for_test(
        &self,
    ) -> (Arc<Barrier>, Arc<Barrier>) {
        let queued = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        *self
            .shared
            .projection_worker_queued_pause
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) =
            Some((Arc::clone(&queued), Arc::clone(&release)));
        (queued, release)
    }

    #[cfg(feature = "test-hooks")]
    pub(crate) fn pause_projection_worker_before_write_lock_for_test(
        &self,
    ) -> (Arc<Barrier>, Arc<Barrier>) {
        let waiting = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        *self
            .shared
            .projection_worker_before_write_lock_pause
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) =
            Some((Arc::clone(&waiting), Arc::clone(&release)));
        (waiting, release)
    }

    #[cfg(debug_assertions)]
    pub(crate) fn acknowledge_projection_stop_for_test(&self, acknowledged: Arc<Barrier>) {
        *self.shared.projection_stop_ack.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) =
            Some(acknowledged);
    }

    pub(crate) fn set_embed_timeout_ms_for_test(&self, timeout_ms: u64) {
        self.shared.embed_dispatch.set_timeout_ms_for_test(timeout_ms);
    }

    pub(crate) fn stop(&self) {
        if let Ok(mut state) = self.shared.state.lock() {
            if state.stopping {
                return;
            }
            state.stopping = true;
            state.pending_scan = false;
            self.shared.state_cvar.notify_all();
        }
        self.shared.embed_dispatch.close();
        #[cfg(debug_assertions)]
        if let Some(acknowledged) = self
            .shared
            .projection_stop_ack
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            acknowledged.wait();
        }
        if let Ok(mut queue) = self.shared.queue.lock() {
            queue.clear();
            self.shared.queue_cvar.notify_all();
        }

        if let Ok(mut dispatcher) = self.dispatcher.lock() {
            if let Some(handle) = dispatcher.take() {
                let _ = handle.join();
            }
        }
        if let Ok(mut workers) = self.workers.lock() {
            for handle in workers.drain(..) {
                let _ = handle.join();
            }
        }
    }
}
