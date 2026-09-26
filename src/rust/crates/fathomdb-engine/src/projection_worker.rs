use super::*;

fn report_projection_runtime_startup_failure(
    startup: &mpsc::Sender<ProjectionRuntimeStartupMessage>,
    role: ProjectionRuntimeStartupRole,
    stage: &'static str,
) {
    let _ = startup.send(ProjectionRuntimeStartupMessage::Setup(ProjectionRuntimeStartupReport {
        role,
        stage: Err(stage),
    }));
}

#[cfg(test)]
fn projection_runtime_injected_setup_failure(
    startup: &mpsc::Sender<ProjectionRuntimeStartupMessage>,
    role: ProjectionRuntimeStartupRole,
    fault: Option<ProjectionRuntimeStartupFaultForTest>,
) -> bool {
    if matches!(fault, Some(fault) if fault.targets(role))
        && matches!(fault, Some(ProjectionRuntimeStartupFaultForTest::SetupFailure(_)))
    {
        report_projection_runtime_startup_failure(startup, role, "injected_setup");
        return true;
    }
    false
}

fn complete_projection_runtime_startup(
    _shared: &ProjectionRuntimeShared,
    startup: mpsc::Sender<ProjectionRuntimeStartupMessage>,
    service_request: Receiver<()>,
    role: ProjectionRuntimeStartupRole,
    #[cfg(test)] fault: Option<ProjectionRuntimeStartupFaultForTest>,
) -> Option<mpsc::Sender<ProjectionRuntimeStartupMessage>> {
    #[cfg(test)]
    let setup_role = if let Some(fault) = fault.filter(|fault| fault.targets(role)) {
        match fault {
            ProjectionRuntimeStartupFaultForTest::SetupFailure(_) => unreachable!(),
            ProjectionRuntimeStartupFaultForTest::DuplicateReport { reported_as, .. } => {
                Some(reported_as)
            }
            ProjectionRuntimeStartupFaultForTest::MissingReport(_) => None,
            ProjectionRuntimeStartupFaultForTest::StallUntilStop(_) => {
                let mut state = match _shared.state.lock() {
                    Ok(state) => state,
                    Err(_) => return None,
                };
                while !state.stopping {
                    state = match _shared.state_cvar.wait(state) {
                        Ok(state) => state,
                        Err(_) => return None,
                    };
                }
                return None;
            }
            ProjectionRuntimeStartupFaultForTest::ExitAfterReport(_) => {
                let _ = startup.send(ProjectionRuntimeStartupMessage::Setup(
                    ProjectionRuntimeStartupReport { role, stage: Ok(()) },
                ));
                return None;
            }
        }
    } else {
        Some(role)
    };
    #[cfg(not(test))]
    let setup_role = Some(role);

    if let Some(setup_role) = setup_role {
        if startup
            .send(ProjectionRuntimeStartupMessage::Setup(ProjectionRuntimeStartupReport {
                role: setup_role,
                stage: Ok(()),
            }))
            .is_err()
        {
            return None;
        }
    }
    service_request.recv().ok().map(|()| startup)
}

pub(crate) fn projection_dispatcher_loop(
    shared: Arc<ProjectionRuntimeShared>,
    dispatcher_idx: usize,
    startup: mpsc::Sender<ProjectionRuntimeStartupMessage>,
    service_request: Receiver<()>,
    #[cfg(test)] startup_fault: Option<ProjectionRuntimeStartupFaultForTest>,
) {
    let startup_role = ProjectionRuntimeStartupRole::Dispatcher(dispatcher_idx);
    #[cfg(test)]
    if projection_runtime_injected_setup_failure(&startup, startup_role, startup_fault) {
        return;
    }
    let connection = match open_runtime_connection(
        &shared.path,
        #[cfg(any(test, feature = "test-hooks"))]
        ManagedConnectionCategory::ProjectionDispatcher,
        #[cfg(any(test, feature = "test-hooks"))]
        &shared.managed_connections,
    ) {
        Ok(connection) => connection,
        Err(_) => {
            report_projection_runtime_startup_failure(&startup, startup_role, "connection_setup");
            return;
        }
    };
    #[cfg(any(test, feature = "test-hooks"))]
    let _connection_registration = shared
        .managed_connections
        .register(WalAttributionRole::ProjectionDispatcher, dispatcher_idx);
    shared.wal_attribution.register(WalAttributionRole::ProjectionDispatcher, dispatcher_idx);
    // 0.8.20 Slice 20c fix-4 (codex §9 round 3 [P1]) — read ONCE:
    // `ProjectionRuntimeShared::embedder` is fixed for the session's lifetime.
    let dense_arm_live = shared.embedder.is_some();
    let mut startup = complete_projection_runtime_startup(
        &shared,
        startup,
        service_request,
        startup_role,
        #[cfg(test)]
        startup_fault,
    );
    if startup.is_none() {
        return;
    }
    loop {
        #[cfg(any(test, feature = "test-hooks"))]
        report_runtime_connection_inventory_for_test(
            &shared,
            &connection,
            WalAttributionRole::ProjectionDispatcher,
            dispatcher_idx,
        );
        #[cfg(any(test, feature = "test-hooks"))]
        report_runtime_native_state_inventory_for_test(
            &shared,
            &connection,
            WalAttributionRole::ProjectionDispatcher,
            dispatcher_idx,
        );
        if let Some(startup) = startup.take() {
            if startup.send(ProjectionRuntimeStartupMessage::ServiceReady(startup_role)).is_err() {
                return;
            }
        }
        let in_flight = {
            let mut state = match shared.state.lock() {
                Ok(state) => state,
                Err(_) => return,
            };
            while !state.stopping
                && (!state.pending_scan
                    || state.frozen
                    || state.active_jobs + state.queued_jobs >= PROJECTION_INFLIGHT_LIMIT)
            {
                #[cfg(any(test, feature = "test-hooks"))]
                report_runtime_connection_inventory_for_test(
                    &shared,
                    &connection,
                    WalAttributionRole::ProjectionDispatcher,
                    dispatcher_idx,
                );
                #[cfg(any(test, feature = "test-hooks"))]
                report_runtime_native_state_inventory_for_test(
                    &shared,
                    &connection,
                    WalAttributionRole::ProjectionDispatcher,
                    dispatcher_idx,
                );
                let can_arm_temporal_scan = !state.frozen
                    && !state.pending_scan
                    && state.active_jobs + state.queued_jobs < PROJECTION_INFLIGHT_LIMIT;
                if can_arm_temporal_scan {
                    if let Some(boundary) = state.next_temporal_scan_epoch_s {
                        let now = current_epoch_seconds();
                        if now >= boundary {
                            state.next_temporal_scan_epoch_s = None;
                            state.pending_scan = true;
                            continue;
                        }
                        let until_boundary = Duration::from_secs(
                            u64::try_from(boundary.saturating_sub(now)).unwrap_or(u64::MAX),
                        );
                        let wait = until_boundary.min(PROJECTION_TEMPORAL_WAKE_POLL);
                        state = match shared.state_cvar.wait_timeout(state, wait) {
                            Ok((state, _)) => state,
                            Err(_) => return,
                        };
                        continue;
                    }
                }
                state = match shared.state_cvar.wait(state) {
                    Ok(state) => state,
                    Err(_) => return,
                };
            }
            if state.stopping {
                return;
            }
            state.pending_scan = false;
            state.next_temporal_scan_epoch_s = None;
            state.in_flight.clone()
        };

        // Fetch up to the in-flight budget in one SQL roundtrip and
        // enqueue them as a batch — previously this loop fetched ONE job
        // per cycle, which capped projection throughput at one row per
        // scanner/worker handshake regardless of how much work was queued
        // in canonical_nodes.
        let budget = {
            let state = match shared.state.lock() {
                Ok(state) => state,
                Err(_) => return,
            };
            PROJECTION_INFLIGHT_LIMIT.saturating_sub(state.active_jobs + state.queued_jobs)
        };
        let fetch_cap = budget.clamp(1, PROJECTION_SCAN_FETCH);
        // A session without a configured runtime dispatches no embedding jobs.
        // Its pending rows stay recoverable for a later configured session; see
        // `next_pending_projection_jobs` for the Slice-30 no-dispatch boundary.
        let fetched = next_pending_projection_jobs(
            &connection,
            &in_flight,
            fetch_cap,
            dense_arm_live,
            &shared.wal_attribution,
            dispatcher_idx,
        )
        .and_then(|jobs| {
            let next_temporal_scan_epoch_s = if dense_arm_live && jobs.is_empty() {
                projection_generation::next_status_membership_boundary(
                    &connection,
                    current_epoch_seconds(),
                )
                .map_err(|_| rusqlite::Error::InvalidQuery)?
            } else {
                None
            };
            Ok((jobs, next_temporal_scan_epoch_s))
        });
        // Keep the no-runtime no-dispatch contract local to the dispatcher too:
        // a future scan change must not turn configuration absence into a worker
        // terminal behind the caller's back.
        debug_assert!(
            fetched.as_ref().map(|(jobs, _)| dense_arm_live || jobs.is_empty()).unwrap_or(true),
            "no-runtime scan returned embedding jobs despite the no-dispatch contract"
        );
        match fetched {
            Ok((jobs, _)) if !jobs.is_empty() => {
                if let Ok(mut state) = shared.state.lock() {
                    state.queued_jobs = state.queued_jobs.saturating_add(jobs.len());
                    for job in &jobs {
                        state.in_flight.insert(job.cursor);
                    }
                    state.pending_scan = true;
                    shared.state_cvar.notify_all();
                }
                if let Ok(mut queue) = shared.queue.lock() {
                    for job in jobs {
                        queue.push_back(job);
                    }
                    shared.queue_cvar.notify_all();
                }
            }
            Ok((_, next_temporal_scan_epoch_s)) => {
                if let Ok(mut state) = shared.state.lock() {
                    state.next_temporal_scan_epoch_s = next_temporal_scan_epoch_s;
                    shared.state_cvar.notify_all();
                }
            }
            Err(_) => {
                if let Ok(mut state) = shared.state.lock() {
                    state.pending_scan = false;
                    state.next_temporal_scan_epoch_s = None;
                    shared.state_cvar.notify_all();
                }
            }
        }
    }
}

pub(crate) fn projection_worker_loop(
    shared: Arc<ProjectionRuntimeShared>,
    worker_idx: usize,
    startup: mpsc::Sender<ProjectionRuntimeStartupMessage>,
    service_request: Receiver<()>,
    #[cfg(test)] startup_fault: Option<ProjectionRuntimeStartupFaultForTest>,
) {
    let startup_role = ProjectionRuntimeStartupRole::Worker(worker_idx);
    #[cfg(test)]
    if projection_runtime_injected_setup_failure(&startup, startup_role, startup_fault) {
        return;
    }
    let mut connection = match open_runtime_connection(
        &shared.path,
        #[cfg(any(test, feature = "test-hooks"))]
        ManagedConnectionCategory::ProjectionWorker,
        #[cfg(any(test, feature = "test-hooks"))]
        &shared.managed_connections,
    ) {
        Ok(connection) => connection,
        Err(_) => {
            report_projection_runtime_startup_failure(&startup, startup_role, "connection_setup");
            return;
        }
    };
    if ensure_vector_partition(&mut connection, shared.embedder_identity.dimension).is_err() {
        report_projection_runtime_startup_failure(&startup, startup_role, "vector_partition_setup");
        return;
    }
    #[cfg(any(test, feature = "test-hooks"))]
    let _connection_registration =
        shared.managed_connections.register(WalAttributionRole::ProjectionWorker, worker_idx);
    shared.wal_attribution.register(WalAttributionRole::ProjectionWorker, worker_idx);
    let mut startup = complete_projection_runtime_startup(
        &shared,
        startup,
        service_request,
        startup_role,
        #[cfg(test)]
        startup_fault,
    );
    if startup.is_none() {
        return;
    }
    loop {
        // A failed trigger-state restoration poisons this connection. Never
        // reuse it for another projection commit with schema triggers disabled.
        if !connection.db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER).unwrap_or(false) {
            return;
        }
        #[cfg(any(test, feature = "test-hooks"))]
        report_runtime_connection_inventory_for_test(
            &shared,
            &connection,
            WalAttributionRole::ProjectionWorker,
            worker_idx,
        );
        #[cfg(any(test, feature = "test-hooks"))]
        report_runtime_native_state_inventory_for_test(
            &shared,
            &connection,
            WalAttributionRole::ProjectionWorker,
            worker_idx,
        );
        if let Some(startup) = startup.take() {
            if startup.send(ProjectionRuntimeStartupMessage::ServiceReady(startup_role)).is_err() {
                return;
            }
        }
        let jobs = {
            let mut queue = match shared.queue.lock() {
                Ok(queue) => queue,
                Err(_) => return,
            };
            loop {
                let stopping = shared.state.lock().map(|state| state.stopping).unwrap_or(true);
                if stopping && queue.is_empty() {
                    return;
                }
                #[cfg(feature = "test-hooks")]
                if !queue.is_empty() {
                    if let Some((queued, release)) = shared
                        .projection_worker_queued_pause
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .take()
                    {
                        queued.wait();
                        release.wait();
                    }
                }
                if let Some(job) = queue.pop_front() {
                    let mut jobs = vec![job];
                    while jobs.len() < PROJECTION_COMMIT_BATCH {
                        let Some(job) = queue.pop_front() else {
                            break;
                        };
                        jobs.push(job);
                    }
                    if let Ok(mut state) = shared.state.lock() {
                        state.queued_jobs = state.queued_jobs.saturating_sub(jobs.len());
                        state.active_jobs = state.active_jobs.saturating_add(jobs.len());
                        shared.state_cvar.notify_all();
                    }
                    break jobs;
                }
                #[cfg(any(test, feature = "test-hooks"))]
                report_runtime_connection_inventory_for_test(
                    &shared,
                    &connection,
                    WalAttributionRole::ProjectionWorker,
                    worker_idx,
                );
                #[cfg(any(test, feature = "test-hooks"))]
                report_runtime_native_state_inventory_for_test(
                    &shared,
                    &connection,
                    WalAttributionRole::ProjectionWorker,
                    worker_idx,
                );
                queue = match shared.queue_cvar.wait(queue) {
                    Ok(queue) => queue,
                    Err(_) => return,
                };
            }
        };

        // EU-5f — isolate worker faults. A panic inside `embed()` (or the
        // commit) must not skip the state cleanup below, or `active_jobs`
        // would stay elevated forever and `wait_for_idle` / `drain` would
        // wedge into `EngineError::Scheduler` (Finding A). Mirrors the
        // reader pool's `LiveGuard` panic-safety. The local commit tx rolls
        // back on unwind, leaving the connection clean for reuse.
        let commit_result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_projection_jobs(&shared, &mut connection, &jobs, worker_idx)
        })) {
            Ok(result) => result,
            Err(_)
                if connection
                    .db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER)
                    .unwrap_or(false) =>
            {
                commit_projection_panic_failures(&shared, &mut connection, &jobs, worker_idx)
            }
            Err(_) => Err(rusqlite::Error::InvalidQuery),
        };
        if let Err(err) = commit_result {
            // Host subscribers are arbitrary application code. Their panic must
            // not bypass the mandatory state cleanup below, or the durable
            // pending row would stay stranded in `in_flight` forever.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                report_projection_commit_failure(&shared, &err);
            }));
            #[cfg(debug_assertions)]
            if let Some((reported, release)) = shared
                .projection_commit_failure_pause
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .take()
            {
                reported.wait();
                release.wait();
            }
        }

        if let Ok(mut state) = shared.state.lock() {
            state.active_jobs = state.active_jobs.saturating_sub(jobs.len());
            for job in &jobs {
                state.in_flight.remove(&job.cursor);
            }
            if !state.stopping {
                state.pending_scan = true;
            }
            shared.state_cvar.notify_all();
        }
    }
}

pub(crate) enum ProjectionOutcome {
    /// `blob` is the un-centered f32 BLOB persisted to
    /// `vector_default.embedding`. `bin_blob` is the (possibly centered)
    /// f32 BLOB fed to `vec_quantize_binary` for the sign-bit column.
    /// EU-5a2: `bin_blob == blob` unless the identity is MC-required
    /// AND a mean_vec is pinned.
    Success {
        cursor: u64,
        kind: String,
        blob: Vec<u8>,
        bin_blob: Vec<u8>,
        generation_id: ProjectionGenerationId,
    },
    Failure {
        cursor: u64,
        failure_code: &'static str,
        generation_id: ProjectionGenerationId,
    },
    /// 0.8.20 Slice 20c fix-4 (codex §9 round 3 [P1]) — the ENVIRONMENT could
    /// not serve this row, as distinct from the embed FAILING. Records nothing
    /// at all: no `projection_failures` audit row and, decisively, **no
    /// terminal**. The row stays `terminal IS NULL`, i.e. PENDING, so the next
    /// session that DOES have an embedder picks it up through the ordinary
    /// scheduler — no graft path and no recovery machinery.
    ///
    /// The only producer is the absent-embedder check at the top of
    /// [`run_projection_job`]. That condition cannot change within a session, so
    /// this can never become a retry loop for a genuinely-failing row.
    ///
    /// It carries NO cursor, deliberately: the other two variants carry one
    /// because they identify the row they are about to WRITE, and this variant
    /// writes nothing at all. A row's deferral is represented on disk by the
    /// continued ABSENCE of its `_fathomdb_projection_terminal` row, which is
    /// exactly the state it was already in.
    Deferred,
}

fn run_projection_jobs(
    shared: &ProjectionRuntimeShared,
    connection: &mut Connection,
    jobs: &[ProjectionJob],
    worker_idx: usize,
) -> rusqlite::Result<()> {
    let outcomes = embed_projection_batch(shared, jobs);
    commit_projection_outcomes(connection, &outcomes, shared, worker_idx)
}

/// Embed a whole commit-batch in ONE `embed_batch` call (amortizes per-call
/// overhead; saturates the GPU — minutes -> seconds on a full-corpus embed). The
/// batched path is the fast HAPPY path only; on ANY anomaly — no embedder, breaker
/// open, single job, batch timeout/failure, row-count or per-row dimension mismatch
/// — it falls back to the proven per-job [`run_projection_job`], which carries the
/// full retry + circuit-breaker + failure-isolation semantics. So batching can only
/// make the common case faster, never change correctness. A panic inside the batch
/// embed resume-unwinds exactly like the per-embed watchdog, so the worker's
/// batch-level `catch_unwind` records `ProjectionPanic` as before.
///
/// Batching is **opt-in** via `FATHOMDB_PROJECTION_BATCH=1` (`true`/`on` accepted).
/// It reshapes the PR-9 per-embed watchdog/breaker accounting into per-batch, so the
/// conservative DEFAULT keeps the proven per-job path — leaving every PR-9 safety
/// test (watchdog, serialization, circuit breaker) behaving exactly as before. The
/// eval GPU-embed run sets the env to get the batched-forward speedup (minutes ->
/// seconds), where the per-job fallback below still backs every error case.
fn projection_batch_enabled() -> bool {
    matches!(
        std::env::var("FATHOMDB_PROJECTION_BATCH").ok().as_deref(),
        Some("1") | Some("true") | Some("on")
    )
}

fn embed_projection_batch(
    shared: &ProjectionRuntimeShared,
    jobs: &[ProjectionJob],
) -> Vec<ProjectionOutcome> {
    let per_job = || jobs.iter().map(|job| run_projection_job(shared, job)).collect();

    let Some(embedder) = shared.embedder.as_ref() else {
        return per_job();
    };
    if jobs.len() < 2
        || shared.embed_circuit_open.load(Ordering::Relaxed)
        || !projection_batch_enabled()
    {
        return per_job();
    }

    let bodies: Vec<String> = jobs.iter().map(|job| job.body.clone()).collect();
    let embed_timeout = Duration::from_millis(shared.embed_timeout_ms.load(Ordering::Relaxed));
    // Each row keeps its single-embed budget worst-case (batch <= COMMIT_BATCH=64).
    let batch_timeout = embed_timeout.saturating_mul(jobs.len() as u32);

    let vectors = {
        // PR-9 — serialize the embedder call (ONE batched call at a time) and make
        // the breaker decision with the guard held (race-free vs other workers),
        // mirroring `run_projection_job`. The batch thread counts as one live embed.
        let _embed_permit =
            shared.embed_serialize.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let threshold = shared.embed_circuit_threshold.load(Ordering::Relaxed);
        if shared.embed_circuit_open.load(Ordering::Relaxed)
            || (threshold != 0 && shared.live_embed_threads.load(Ordering::Relaxed) >= threshold)
        {
            shared.embed_circuit_open.store(true, Ordering::Relaxed);
            return per_job();
        }
        match embed_batch_with_watchdog(
            embedder,
            &bodies,
            batch_timeout,
            &shared.live_embed_threads,
        ) {
            Ok(vectors) => vectors,
            // Timeout / failed / disconnected -> the per-job path retries each row
            // and engages the breaker exactly as before.
            Err(_) => return per_job(),
        }
    };

    if vectors.len() != jobs.len() {
        return per_job();
    }
    let mut outcomes = Vec::with_capacity(jobs.len());
    for (job, vector) in jobs.iter().zip(vectors) {
        if u32::try_from(vector.len()).unwrap_or(u32::MAX) != shared.embedder_identity.dimension {
            // A row came back wrong-dim: fall back per-job for the whole batch
            // (rare; keeps the dimension-mismatch failure path identical).
            return per_job();
        }
        // Mirror run_projection_job's post-embed step exactly: persisted f32 BLOB is
        // un-centered; centering for the binary column is finalized in
        // commit_projection_outcomes (so bin_blob == blob here).
        let blob = encode_vector_blob(&vector);
        let bin_blob = blob.clone();
        outcomes.push(ProjectionOutcome::Success {
            cursor: job.cursor,
            kind: job.kind.clone(),
            blob,
            bin_blob,
            generation_id: job.generation_id.clone(),
        });
    }
    outcomes
}

/// EU-5f — record every job in a panicked batch as a terminal projection
/// failure so the scheduler does not re-enqueue and re-panic on the same
/// cursors. Best-effort; runs after the worker caught a panic.
fn commit_projection_panic_failures(
    shared: &ProjectionRuntimeShared,
    connection: &mut Connection,
    jobs: &[ProjectionJob],
    worker_idx: usize,
) -> rusqlite::Result<()> {
    let outcomes: Vec<ProjectionOutcome> = jobs
        .iter()
        .map(|job| ProjectionOutcome::Failure {
            cursor: job.cursor,
            failure_code: "ProjectionPanic",
            generation_id: job.generation_id.clone(),
        })
        .collect();
    commit_projection_outcomes(connection, &outcomes, shared, worker_idx)
}

/// Route a background projection-commit failure through the engine's existing
/// host subscriber path. A SQLite error retains its stable SQLite code; a
/// rusqlite-layer error is an engine storage failure rather than a fabricated
/// SQLite diagnostic.
fn report_projection_commit_failure(shared: &ProjectionRuntimeShared, err: &rusqlite::Error) {
    let event = if let Some(code) = sqlite_extended_code_name(err) {
        lifecycle::Event {
            phase: lifecycle::Phase::Failed,
            source: lifecycle::EventSource::SqliteInternal,
            category: lifecycle::EventCategory::Error,
            code: Some(code),
        }
    } else {
        lifecycle::Event {
            phase: lifecycle::Phase::Failed,
            source: lifecycle::EventSource::Engine,
            category: lifecycle::EventCategory::Error,
            code: Some("StorageError"),
        }
    };
    shared.subscribers.dispatch(&event);
}

fn run_projection_job(shared: &ProjectionRuntimeShared, job: &ProjectionJob) -> ProjectionOutcome {
    // 0.8.20 Slice 20c fix-4 (codex §9 round 3 [P1]) — an ABSENT embedder is an
    // ENVIRONMENT fact, not an embed failure, and it CANNOT appear mid-job:
    // `ProjectionRuntimeShared::embedder` is fixed for the whole session. So for a
    // NODE row the whole retry ladder (0 + 1 + 4 + 16 s) can only reach a
    // conclusion that was already knowable at entry — answer it NOW, with the
    // NON-TERMINAL `Deferred`: no audit row, no terminal, and therefore a write
    // the next live-embedder session can still recover.
    //
    // That is codex's finding. With the kind already ENROLLED, the shipped
    // `'failed'` terminal was PERMANENT (nothing reopens one, and nothing should:
    // re-enqueueing one would loop a genuinely-failing row forever), so the write
    // was lost while `dense_readiness` read `ready`.
    //
    // `projection_dispatcher_loop` already declines to dispatch node jobs in a
    // no-embedder session — it must, or the still-pending row would be re-scanned
    // in a hot loop. This check is the LOCAL backstop for the same invariant:
    // whatever reaches a worker with no embedder must not be TERMINATED. Keeping
    // the invariant beside the code that would otherwise write the terminal is
    // what makes it hold if that dispatcher-side filter is ever loosened.
    //
    // EDGE rows deliberately fall THROUGH to the shipped path, ladder and all.
    // `'edge_fact'` is auto-registered by the edge write itself, UN-gated on the
    // embedder (`project_canonical_edge_row`, G11 — see the note in
    // `batch_vector_kinds_needing_enrolment`), so an edge body written with no embedder is
    // outstanding the moment it lands. Deferring it would leave `drain` and
    // `excise_source` returning `EngineError::Scheduler` on paths with nothing to
    // do with the dense arm (MEASURED: 4 shipped tests across
    // `tc31_source_id_on_every_hit`, `provenance_mandatory` and
    // `multidoc_extractor_provenance`). Making edges recoverable needs their
    // enrolment gated the way fix-2 gated node kinds — reported as OOS-13, and
    // outside codex's finding, which is the node path.
    //
    // Their LADDER is left alone for a second, separately MEASURED reason:
    // shortening it makes the worker's terminal-commit land while a caller's own
    // write is still open, which used to trip the governed write-race. Measured on
    // `consolidate_provider` under 6-way concurrency: 0/48 failures with the
    // ladder, 8/48 without. Left byte-for-byte as shipped; the ladder length is
    // reported as OOS-17 rather than newly exposed by a fix round.
    //
    // 0.8.20 Slice 21 (TC-57) — this note used to name that race
    // `SQLITE_BUSY_SNAPSHOT` and call it PRE-EXISTING. Both are corrected: the
    // characterized mechanism is plain `SQLITE_BUSY` (5) on a read→write lock
    // PROMOTION, with the busy handler invoked ZERO times (SQLite skips it for
    // deadlock avoidance), so no `busy_timeout` could absorb it;
    // `SQLITE_BUSY_SNAPSHOT` (517) is only a second, narrower exit of the same
    // shape. And the race is FIXED — `commit_batch` now takes `BEGIN IMMEDIATE`
    // (see the note there), so the governed path never promotes. The 0/48-vs-8/48
    // measurement above stands as the reason not to shorten the ladder, but it is
    // no longer load-bearing for correctness of the governed write path.
    if shared.embedder.is_none() && job.kind != EDGE_FACT_KIND {
        return ProjectionOutcome::Deferred;
    }
    // PR-9 — embed circuit breaker (see `embed_circuit_open`). Once abandoned
    // (timed-out) embed threads have piled up to the threshold the embedder is
    // treated as broken; fail subsequent jobs fast WITHOUT attempting an embed,
    // so a wedged embedder cannot keep leaking abandoned watchdog threads. This
    // entry check is the fast path; the latch decision itself is made under the
    // embed guard below (race-free against other workers).
    if shared.embed_circuit_open.load(Ordering::Relaxed) {
        return ProjectionOutcome::Failure {
            cursor: job.cursor,
            failure_code: "EmbedderError",
            generation_id: job.generation_id.clone(),
        };
    }
    let delays = shared.retry_delays_ms.lock().map(|delays| delays.clone()).unwrap_or_default();
    let mut last_code = "EmbedderError";
    for (attempt, delay_ms) in std::iter::once(0_u64).chain(delays.iter().copied()).enumerate() {
        if attempt > 0 {
            if shared.state.lock().map(|state| state.stopping).unwrap_or(true) {
                return ProjectionOutcome::Failure {
                    cursor: job.cursor,
                    failure_code: last_code,
                    generation_id: job.generation_id.clone(),
                };
            }
            thread::sleep(Duration::from_millis(delay_ms));
        }
        // PR-9 — re-check the breaker on every attempt, not just at entry:
        // another worker (or an earlier attempt of this job) may have latched
        // it while we were sleeping between retries. Bail before spawning yet
        // another timeout-bound watchdog thread, so the abandoned-thread leak
        // stays bounded even on the multi-retry path.
        if shared.embed_circuit_open.load(Ordering::Relaxed) {
            return ProjectionOutcome::Failure {
                cursor: job.cursor,
                failure_code: last_code,
                generation_id: job.generation_id.clone(),
            };
        }
        // PR-9 / ADR-0.6.0 Invariant 5 — every embed runs under the per-call
        // watchdog deadline so a hung embed surfaces Timeout instead of
        // parking this worker forever.
        let embed_timeout = Duration::from_millis(shared.embed_timeout_ms.load(Ordering::Relaxed));
        let vector = match shared.embedder.as_ref() {
            Some(embedder) => {
                // PR-9 — serialize the embed call engine-side (see
                // `embed_serialize`): the shared embedder is invoked one call
                // at a time, for SAFETY with arbitrary caller-supplied
                // embedders (throughput is ~neutral on the candle default).
                // The guard is held across the watchdog call and released
                // here, so commit/IO below stays parallel and a timed-out
                // embed frees it. The guard owns no data; a panic-resumed
                // embed poisons it, so we recover the inner guard rather than
                // wedge the whole pool.
                let _embed_permit =
                    shared.embed_serialize.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                // PR-9 — breaker decision, made WITH the guard held so it is
                // race-free against other workers: if abandoned embed threads
                // from earlier timeouts have piled up to the threshold, latch
                // the breaker and fail fast WITHOUT spawning another one. The
                // live count is checked here (also covers a breaker latched by
                // another worker while we were queued on the lock), bounding
                // the abandoned-thread leak to ~threshold regardless of whether
                // the embedder hangs always or only intermittently.
                let threshold = shared.embed_circuit_threshold.load(Ordering::Relaxed);
                if shared.embed_circuit_open.load(Ordering::Relaxed)
                    || (threshold != 0
                        && shared.live_embed_threads.load(Ordering::Relaxed) >= threshold)
                {
                    shared.embed_circuit_open.store(true, Ordering::Relaxed);
                    return ProjectionOutcome::Failure {
                        cursor: job.cursor,
                        failure_code: last_code,
                        generation_id: job.generation_id.clone(),
                    };
                }
                match embed_with_watchdog(
                    embedder,
                    &job.body,
                    embed_timeout,
                    &shared.live_embed_threads,
                ) {
                    Ok(vector) => vector,
                    Err(RuntimeEmbedderError::Timeout) => {
                        // The embed thread is now abandoned (still counted in
                        // live_embed_threads until it returns); the breaker
                        // check above caps how many can accumulate.
                        last_code = "EmbedderError";
                        continue;
                    }
                    Err(RuntimeEmbedderError::Failed { .. }) => {
                        last_code = "EmbedderError";
                        continue;
                    }
                }
            }
            None => {
                last_code = "EmbedderNotConfiguredError";
                continue;
            }
        };

        if u32::try_from(vector.len()).unwrap_or(u32::MAX) != shared.embedder_identity.dimension {
            last_code = "EmbedderDimensionMismatchError";
            continue;
        }

        let blob = encode_vector_blob(&vector);
        // EU-5a2 mean-centering apply path (projection write side). The
        // f32 BLOB persisted is ALWAYS un-centered; `bin_blob` carries
        // the (possibly centered) f32 fed to `vec_quantize_binary`. The
        // centering decision is finalized in `commit_projection_outcomes`
        // where the writer connection is in-hand and the read of
        // `_fathomdb_embedder_profiles.mean_vec` is in the same tx as
        // the INSERT. NoopEmbedder (EU-5a2's only live identity) is not
        // MC-required, so `bin_blob == blob` throughout EU-5a2.
        let bin_blob = blob.clone();
        return ProjectionOutcome::Success {
            cursor: job.cursor,
            kind: job.kind.clone(),
            blob,
            bin_blob,
            generation_id: job.generation_id.clone(),
        };
    }

    ProjectionOutcome::Failure {
        cursor: job.cursor,
        failure_code: last_code,
        generation_id: job.generation_id.clone(),
    }
}
