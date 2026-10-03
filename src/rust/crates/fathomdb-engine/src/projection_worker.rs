use super::*;
use crate::embedding::EDGE_FACT_KIND;
use crate::lifecycle::sqlite_extended_code_name;
use crate::projection_runtime::PROJECTION_COMMIT_BATCH;
use crate::projection_runtime::PROJECTION_TEMPORAL_WAKE_POLL;

#[cfg(any(test, feature = "test-hooks"))]
fn report_runtime_connection_inventory_for_test(
    shared: &ProjectionRuntimeShared,
    connection: &Connection,
    role: WalAttributionRole,
    index: usize,
) {
    let respond = {
        let Ok(mut request_slot) = shared.runtime_inventory_request.lock() else {
            return;
        };
        let Some(request) = request_slot.as_mut() else {
            return;
        };
        if !request.pending.remove(&(role, index)) {
            return;
        }
        let respond = request.respond.clone();
        if request.pending.is_empty() {
            *request_slot = None;
        }
        respond
    };
    let _ = respond.send((role, index, connection.is_autocommit()));
}

#[cfg(any(test, feature = "test-hooks"))]
fn report_runtime_native_state_inventory_for_test(
    shared: &ProjectionRuntimeShared,
    connection: &Connection,
    role: WalAttributionRole,
    index: usize,
) {
    let respond = {
        let Ok(mut request_slot) = shared.runtime_native_state_request.lock() else {
            return;
        };
        let Some(request) = request_slot.as_mut() else {
            return;
        };
        if !request.pending.remove(&(role, index)) {
            return;
        }
        let respond = request.respond.clone();
        if request.pending.is_empty() {
            *request_slot = None;
        }
        respond
    };
    let _ = respond.send(native_connection_state_for_test(connection, role, index));
}

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
                    || state.active_jobs + state.queued_jobs >= shared.admission_capacity)
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
                    && state.active_jobs + state.queued_jobs < shared.admission_capacity;
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
            shared.admission_capacity.saturating_sub(state.active_jobs + state.queued_jobs)
        };
        let fetch_cap = budget.clamp(1, shared.admission_capacity);
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
                    #[cfg(feature = "test-hooks")]
                    shared
                        .embed_dispatch
                        .observe_projection_admission(state.active_jobs + state.queued_jobs);
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
    /// The absent-embedder check, dispatch capacity waits interrupted by close,
    /// and dispatch cancellation all leave this same durable pending state.
    /// Provider failures and started timeouts instead use the retry budget.
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

/// The optional batch fast path uses the same fixed provider slot and single
/// absolute deadline as per-row projection. A failed batch falls back to the
/// row retry policy after the dispatch reply is released.
fn projection_batch_enabled() -> bool {
    matches!(
        std::env::var("FATHOMDB_PROJECTION_BATCH").ok().as_deref(),
        Some("1") | Some("true") | Some("on")
    )
}

fn wait_for_projection_retry(shared: &ProjectionRuntimeShared, delay: Duration) -> bool {
    let deadline = Instant::now() + delay;
    let mut state = match shared.state.lock() {
        Ok(state) => state,
        Err(_) => return false,
    };
    loop {
        if state.stopping {
            return false;
        }
        let now = Instant::now();
        if now >= deadline {
            return true;
        }
        state = match shared.state_cvar.wait_timeout(state, deadline.saturating_duration_since(now))
        {
            Ok((state, _)) => state,
            Err(_) => return false,
        };
    }
}

fn embed_projection_batch(
    shared: &ProjectionRuntimeShared,
    jobs: &[ProjectionJob],
) -> Vec<ProjectionOutcome> {
    let per_job = || jobs.iter().map(|job| run_projection_job(shared, job)).collect();
    if shared.embedder.is_none() || jobs.len() < 2 || !projection_batch_enabled() {
        return per_job();
    }
    let bodies: Vec<String> = jobs.iter().map(|job| job.body.clone()).collect();
    let vectors = loop {
        #[cfg(feature = "test-hooks")]
        let owner = Some(crate::embed_dispatch::d27_observation::Owner::Projection {
            projection_cursors: jobs.iter().map(|job| job.cursor).collect(),
        });
        #[cfg(feature = "test-hooks")]
        let result = crate::embed_dispatch::d27_observation::with_owner(owner, || {
            shared.embed_dispatch.submit_batch(bodies.clone()).and_then(EmbedReply::wait)
        });
        #[cfg(not(feature = "test-hooks"))]
        let result = shared.embed_dispatch.submit_batch(bodies.clone()).and_then(EmbedReply::wait);
        match result {
            Ok(EmbedOutput::Batch(vectors)) => break vectors,
            Err(DispatchError::Saturated | DispatchError::QueuedExpired) => {
                if !wait_for_projection_retry(shared, Duration::from_millis(25)) {
                    return jobs.iter().map(|_| ProjectionOutcome::Deferred).collect();
                }
            }
            Err(DispatchError::Closing | DispatchError::Cancelled) => {
                return jobs.iter().map(|_| ProjectionOutcome::Deferred).collect();
            }
            Err(DispatchError::Panic(payload)) => std::panic::resume_unwind(payload),
            _ => return per_job(),
        }
    };
    if vectors.len() != jobs.len() {
        return per_job();
    }
    jobs.iter()
        .zip(vectors)
        .map(|(job, vector)| {
            let blob = encode_vector_blob(&vector);
            ProjectionOutcome::Success {
                cursor: job.cursor,
                kind: job.kind.clone(),
                bin_blob: blob.clone(),
                blob,
                generation_id: job.generation_id.clone(),
            }
        })
        .collect()
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
    let delays = shared.retry_delays_ms.lock().map(|delays| delays.clone()).unwrap_or_default();
    let mut spent_attempts = 0_usize;
    let mut retry_delay_due = false;
    let mut last_code = "EmbedderError";
    loop {
        if retry_delay_due {
            if !wait_for_projection_retry(shared, Duration::from_millis(delays[spent_attempts - 1]))
            {
                return ProjectionOutcome::Deferred;
            }
            retry_delay_due = false;
        }
        if shared.state.lock().map(|state| state.stopping).unwrap_or(true) {
            return ProjectionOutcome::Deferred;
        }
        #[cfg(feature = "test-hooks")]
        let result = crate::embed_dispatch::d27_observation::with_owner(
            Some(crate::embed_dispatch::d27_observation::Owner::Projection {
                projection_cursors: vec![job.cursor],
            }),
            || shared.embed_dispatch.submit_text(job.body.clone()).and_then(EmbedReply::wait),
        );
        #[cfg(not(feature = "test-hooks"))]
        let result = shared.embed_dispatch.submit_text(job.body.clone()).and_then(EmbedReply::wait);
        let vector = match result {
            Ok(EmbedOutput::One(vector)) => vector,
            Err(DispatchError::Saturated | DispatchError::QueuedExpired) => {
                if !wait_for_projection_retry(shared, Duration::from_millis(25)) {
                    return ProjectionOutcome::Deferred;
                }
                continue;
            }
            Err(DispatchError::Closing | DispatchError::Cancelled) => {
                return ProjectionOutcome::Deferred;
            }
            Err(DispatchError::Panic(payload)) => std::panic::resume_unwind(payload),
            Err(DispatchError::NotConfigured) => {
                last_code = "EmbedderNotConfiguredError";
                Vec::new()
            }
            Err(DispatchError::InvalidOutput) => {
                last_code = "EmbedderDimensionMismatchError";
                Vec::new()
            }
            Err(DispatchError::StartedTimeout | DispatchError::Provider(_)) => {
                last_code = "EmbedderError";
                Vec::new()
            }
            Ok(EmbedOutput::Batch(_)) => unreachable!("single embedding returned a batch"),
        };
        if vector.is_empty() {
            if spent_attempts >= delays.len() {
                break;
            }
            spent_attempts += 1;
            retry_delay_due = true;
            continue;
        }
        if u32::try_from(vector.len()).unwrap_or(u32::MAX) != shared.embedder_identity.dimension {
            last_code = "EmbedderDimensionMismatchError";
            if spent_attempts >= delays.len() {
                break;
            }
            spent_attempts += 1;
            retry_delay_due = true;
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

/// 0.8.20 Slice 20 fix-1 (codex §9 [P2]) — the ONE definition of "a canonical
/// EDGE row the vector pipeline still owes an embed for".
///
/// Two call sites must agree on this predicate and had drifted:
///
/// - [`next_pending_projection_jobs`] — the SCHEDULER, and therefore the
///   authority on what will actually be embedded. It joins
///   `_fathomdb_vector_kinds` on `'edge_fact'`, so an edge body is only ever
///   scheduled when that kind is registered.
/// - [`connection_has_pending_projection_work`] — the PROBE behind
///   `drain`/`wait_for_idle` and, since this slice, `dense_readiness`. It
///   omitted that join.
///
/// The consequence of the drift: a live edge body written while `edge_fact` was
/// not a registered vector kind (e.g. edges carried forward from before the G11
/// edge-vector pipeline, which is what auto-registers the kind) counted as
/// outstanding work the scheduler would NEVER take. `dense_readiness` reported
/// `embedding` forever and `drain` could never report idle — both the mirror
/// image of R-20-DR's property. Building both edge arms from this one fragment
/// makes a repeat drift unrepresentable.
///
/// Emits the `FROM`/`JOIN` clauses plus the shared `WHERE` predicates, with the
/// edge table aliased `ce` and the projection terminal aliased `pt`; `now_idx`
/// is the 1-based bind index of the `:now` seam that [`edge_validity_sql`]
/// consumes. Callers may append further `AND` predicates.
///
/// **The one predicate deliberately NOT shared** is the scheduler's
/// `write_cursor > :cursor` watermark filter, which the scheduler appends and
/// the probe must not: per the G11 (Slice 15) fix-1 note on the probe, the
/// probe has to see edge bodies left un-projected BELOW the watermark when the
/// engine closed mid-flight, or `drain` would report idle with edge vectors
/// still missing on reopen. That asymmetry is intentional and load-bearing; the
/// row-eligibility predicates above are not, and are shared.
fn pending_edge_projection_from_where(now_idx: usize) -> String {
    format!(
        "FROM canonical_edges ce
         JOIN _fathomdb_vector_kinds
           ON _fathomdb_vector_kinds.kind = 'edge_fact'
         LEFT JOIN _fathomdb_projection_terminal pt
           ON pt.write_cursor = ce.write_cursor
         WHERE ce.body IS NOT NULL
           AND ce.superseded_at IS NULL{}
           AND pt.write_cursor IS NULL",
        edge_validity_sql("ce", now_idx)
    )
}

const PROJECTION_CANDIDATE_PAGE: usize = 256;

fn pending_projection_candidate_page(
    connection: &Connection,
    after_cursor: u64,
    through_cursor: Option<u64>,
    effective_at: i64,
) -> rusqlite::Result<Vec<(u64, String)>> {
    let sql = format!(
        "SELECT pending.write_cursor,pending.kind FROM (
           SELECT n.write_cursor,n.kind
           FROM canonical_nodes n
           JOIN _fathomdb_vector_kinds vk ON vk.kind=n.kind
           LEFT JOIN _fathomdb_projection_terminal pt ON pt.write_cursor=n.write_cursor
           WHERE n.write_cursor>?1 AND (?2 IS NULL OR n.write_cursor<=?2)
             AND pt.write_cursor IS NULL
           UNION ALL
           SELECT ce.write_cursor,'edge_fact'
           {}
             AND ce.write_cursor>?1 AND (?2 IS NULL OR ce.write_cursor<=?2)
         ) AS pending ORDER BY pending.write_cursor LIMIT {PROJECTION_CANDIDATE_PAGE}",
        pending_edge_projection_from_where(3),
    );
    connection
        .prepare_cached(&sql)?
        .query_map(params![after_cursor, through_cursor, effective_at], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
        .collect()
}

fn pending_projection_members_in_range(
    connection: &Connection,
    after_cursor: u64,
    through_cursor: Option<u64>,
    effective_at: i64,
) -> rusqlite::Result<Vec<(u64, String)>> {
    let mut members = Vec::new();
    let mut page_cursor = after_cursor;
    loop {
        let page = pending_projection_candidate_page(
            connection,
            page_cursor,
            through_cursor,
            effective_at,
        )?;
        let page_len = page.len();
        for (cursor, kind) in page {
            page_cursor = cursor;
            if projection_generation::dense_member_kind_at(connection, cursor, effective_at)
                .map_err(|_| rusqlite::Error::InvalidQuery)?
                .as_deref()
                == Some(kind.as_str())
            {
                members.push((cursor, kind));
            }
        }
        if page_len < PROJECTION_CANDIDATE_PAGE {
            break;
        }
    }
    Ok(members)
}

fn first_pending_projection_member_in_range(
    connection: &Connection,
    after_cursor: u64,
    through_cursor: Option<u64>,
    effective_at: i64,
) -> rusqlite::Result<Option<(u64, String)>> {
    let mut page_cursor = after_cursor;
    loop {
        let page = pending_projection_candidate_page(
            connection,
            page_cursor,
            through_cursor,
            effective_at,
        )?;
        let page_len = page.len();
        for (cursor, kind) in page {
            page_cursor = cursor;
            if projection_generation::dense_member_kind_at(connection, cursor, effective_at)
                .map_err(|_| rusqlite::Error::InvalidQuery)?
                .as_deref()
                == Some(kind.as_str())
            {
                return Ok(Some((cursor, kind)));
            }
        }
        if page_len < PROJECTION_CANDIDATE_PAGE {
            return Ok(None);
        }
    }
}

fn lowest_pending_projection_member_at_or_below(
    connection: &Connection,
    watermark: u64,
    effective_at: i64,
) -> rusqlite::Result<Option<u64>> {
    if watermark == 0 {
        return Ok(None);
    }
    Ok(first_pending_projection_member_in_range(connection, 0, Some(watermark), effective_at)?
        .map(|(cursor, _)| cursor))
}

/// The SCHEDULER's scan: the next `max_jobs` pending projection jobs in
/// `write_cursor` order.
///
/// `dense_arm_live` is `ProjectionRuntimeShared::embedder.is_some()`, read once
/// per dispatcher because it is fixed for the session's lifetime.
///
/// # No configured embedder
///
/// A session with no configured runtime does not dispatch either node or edge
/// embedding jobs. Dispatching an edge into the retry ladder used to record a
/// durable `failed` terminal, contradicting Slice 30's immediate
/// `FDB_EMBEDDER_REQUIRED` configuration feedback and losing work that a later
/// configured session must be able to complete. The durable rows stay pending;
/// `drain` names the configuration error and an erasure verb can remove them.
fn next_pending_projection_jobs(
    connection: &Connection,
    in_flight: &BTreeSet<u64>,
    max_jobs: usize,
    dense_arm_live: bool,
    attribution: &Arc<WalAttributionCollector>,
    dispatcher_idx: usize,
) -> rusqlite::Result<Vec<ProjectionJob>> {
    if max_jobs == 0 || !dense_arm_live {
        return Ok(Vec::new());
    }
    let effective_at = current_epoch_seconds();
    let persisted_cursor = load_projection_cursor(connection)?;
    // G11 (Slice 15) — UNION extends the projection queue to include edge bodies.
    // Edge bodies use kind `'edge_fact'` so `resolve_source_type` maps them to
    // `source_type = 'edge_fact'` in `vector_default` (partition correctness).
    // The UNION is ordered by write_cursor so projection proceeds in
    // insertion order across nodes and edges.
    //
    let sql = format!(
        "SELECT pending.write_cursor, pending.kind, pending.body, current.generation_id FROM (
             SELECT canonical_nodes.write_cursor AS write_cursor,
                    canonical_nodes.kind AS kind,
                    canonical_nodes.body AS body
             FROM canonical_nodes
             JOIN _fathomdb_vector_kinds
               ON _fathomdb_vector_kinds.kind = canonical_nodes.kind
             LEFT JOIN _fathomdb_projection_terminal
               ON _fathomdb_projection_terminal.write_cursor = canonical_nodes.write_cursor
             WHERE canonical_nodes.write_cursor > ?1
               AND _fathomdb_projection_terminal.write_cursor IS NULL

             UNION ALL

             SELECT ce.write_cursor AS write_cursor,
                    'edge_fact' AS kind,
                    ce.body AS body
             {edge_arm}
               AND ce.write_cursor > ?1
         ) AS pending
         CROSS JOIN _fathomdb_projection_generation_current AS current
         WHERE current.singleton=1
         ORDER BY pending.write_cursor
         LIMIT {PROJECTION_CANDIDATE_PAGE}",
        // The persisted projection cursor remains the healthy fast path. TC-33:
        // `?1` is the page cursor and the edge validity instant binds at `?2`.
        edge_arm = pending_edge_projection_from_where(2),
    );
    let mut statement = connection.prepare_cached(&sql)?;
    // SQLite starts the implicit read transaction at statement execution; do
    // not call this a transaction before `query_map` succeeds.
    let _activity = attribution.enabled.then(|| {
        WalAttributionActivity::begin(
            Arc::clone(attribution),
            WalAttributionRole::ProjectionDispatcher,
            dispatcher_idx,
            "snapshot_acquired",
        )
    });
    let mut scan_cursor = persisted_cursor;
    let mut checked_below_watermark = false;
    loop {
        let mut jobs = Vec::with_capacity(max_jobs);
        let mut after_cursor = scan_cursor;
        loop {
            let page = statement
                .query_map(params![after_cursor, effective_at], |row| {
                    let generation_id =
                        projection_generation::parse_persisted_generation_id(row.get(3)?)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?;
                    Ok(ProjectionJob {
                        cursor: row.get(0)?,
                        kind: row.get(1)?,
                        body: row.get(2)?,
                        generation_id,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let page_len = page.len();
            for job in page {
                after_cursor = job.cursor;
                if in_flight.contains(&job.cursor) {
                    continue;
                }
                if projection_generation::dense_member_kind_at(connection, job.cursor, effective_at)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?
                    .as_deref()
                    != Some(job.kind.as_str())
                {
                    continue;
                }
                jobs.push(job);
                if jobs.len() >= max_jobs {
                    return Ok(jobs);
                }
            }
            if page_len < PROJECTION_CANDIDATE_PAGE {
                break;
            }
        }
        if !jobs.is_empty() || checked_below_watermark {
            return Ok(jobs);
        }
        checked_below_watermark = true;
        let Some(cursor) = lowest_pending_projection_member_at_or_below(
            connection,
            persisted_cursor,
            effective_at,
        )?
        else {
            return Ok(Vec::new());
        };
        scan_cursor = cursor.saturating_sub(1);
        if scan_cursor < persisted_cursor {
            store_projection_cursor(connection, scan_cursor)?;
        }
    }
}

pub(crate) fn database_has_pending_projection_work(
    path: &Path,
    #[cfg(any(test, feature = "test-hooks"))] managed_connections: &Arc<ManagedConnectionRegistry>,
) -> rusqlite::Result<bool> {
    let connection = open_runtime_connection(
        path,
        #[cfg(any(test, feature = "test-hooks"))]
        ManagedConnectionCategory::RuntimeProbe,
        #[cfg(any(test, feature = "test-hooks"))]
        managed_connections,
    )?;
    connection_has_pending_projection_work(&connection)
}

/// 0.8.20 Slice 20 (R-20-DR) — the body of
/// [`database_has_pending_projection_work`], lifted so it can also run on a
/// connection the caller ALREADY holds (the engine's own connection, inside
/// [`Engine::read_projections`]) instead of opening a runtime connection from a
/// path. Both callers run the same two arms and the same predicates — which is
/// the point. Readiness and `drain`/`wait_for_idle` must key off ONE definition
/// of "outstanding embed", or readiness could report `ready` for work `drain`
/// still waits on.
///
/// fix-1 (codex §9 [P2]) — the edge arm is no longer a hand-copied mirror of
/// the scheduler's: both are built from
/// [`pending_edge_projection_from_where`]. The copy had lost the
/// `_fathomdb_vector_kinds` join, so this probe reported permanent pending work
/// for edge bodies the scheduler would never schedule. That was PRE-EXISTING —
/// it reached `Engine::drain` through `wait_for_idle` before readiness existed.
pub(crate) fn connection_has_pending_projection_work(
    connection: &Connection,
) -> rusqlite::Result<bool> {
    // G11 (Slice 15) fix-1 [P2] — also check canonical_edges for edge bodies
    // that were not projected before the engine closed. Without this check,
    // drain() returns idle while edge vectors remain unembedded on reopen.
    // fix-31 [P2]: exclude superseded edges from the pending check so the
    // scheduler does not pick up stale tombstoned rows as projection work.
    // 0.8.12 Slice A (R-CON-2 named default-ON blocker; Slice-20 codex §9
    // [P2]) — also exclude t_invalid-excluded (recency-consolidated) edges,
    // mirroring `next_pending_projection_jobs`'s edge arm. Required: without
    // this mirror, a rebuild-truncated t_invalid edge that
    // `next_pending_projection_jobs` now correctly skips would never gain a
    // `_fathomdb_projection_terminal` row, so this probe would flag it as
    // phantom-pending forever and `drain()`/`wait_for_idle` would hang.
    // Slice-20 fix-1 [P2]: the mirror is now STRUCTURAL — the arm is built from
    // `pending_edge_projection_from_where`, the same fragment the scheduler
    // uses — because the hand-copied mirror had already lost the
    // `_fathomdb_vector_kinds` join and produced exactly the phantom-pending
    // hang described above for edge bodies under an unregistered `edge_fact`.
    let effective_at = current_epoch_seconds();
    let watermark = load_projection_cursor(connection)?;
    if first_pending_projection_member_in_range(connection, watermark, None, effective_at)?
        .is_some()
    {
        return Ok(true);
    }
    Ok(lowest_pending_projection_member_at_or_below(connection, watermark, effective_at)?.is_some())
}

/// One pure, count-preserving view of the projection rows that are presently
/// eligible for embedding. It shares the scheduler and drain predicates, so a
/// readiness report never names work that `drain` does not wait for.
pub(crate) fn pending_embedding_work(
    connection: &Connection,
) -> rusqlite::Result<Vec<(String, u64)>> {
    let effective_at = current_epoch_seconds();
    let watermark = load_projection_cursor(connection)?;
    let mut rows = pending_projection_members_in_range(connection, watermark, None, effective_at)?;
    rows.extend(pending_projection_members_in_range(connection, 0, Some(watermark), effective_at)?);
    let mut counts = BTreeMap::<String, u64>::new();
    for (_, kind) in rows {
        let count = counts.entry(kind).or_default();
        *count = count.saturating_add(1);
    }
    Ok(counts.into_iter().collect())
}
