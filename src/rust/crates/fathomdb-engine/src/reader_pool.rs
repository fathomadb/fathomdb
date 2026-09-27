use super::*;

fn reader_worker_loop(
    mut connection: Connection,
    rx: Receiver<ReaderRequest>,
    live_workers: Arc<AtomicUsize>,
    worker_idx: usize,
    wal_attribution: Arc<WalAttributionCollector>,
    ready: SyncSender<()>,
    #[cfg(any(test, feature = "test-hooks"))] managed_connections: Arc<ManagedConnectionRegistry>,
) {
    connection.set_prepared_statement_cache_capacity(10);
    #[cfg(any(test, feature = "test-hooks"))]
    let _connection_registration =
        managed_connections.register(WalAttributionRole::ReaderWorker, worker_idx);
    wal_attribution.register(WalAttributionRole::ReaderWorker, worker_idx);
    live_workers.fetch_add(1, Ordering::SeqCst);
    ready.send(()).expect("reader worker startup receiver must remain live");
    // Drop guard so the live counter decrements even on panic.
    struct LiveGuard(Arc<AtomicUsize>);
    impl Drop for LiveGuard {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }
    let _guard = LiveGuard(live_workers);

    while let Ok(request) = rx.recv() {
        match request {
            ReaderRequest::Shutdown => break,
            #[cfg(feature = "tc5-benchmark")]
            ReaderRequest::VectorStage { request, respond } => {
                let result = tc5_benchmark::read_vector_stage_in_tx(&mut connection, request);
                finish_reader_request(&connection, &wal_attribution, worker_idx);
                let _ = respond.send(result);
            }
            ReaderRequest::SearchProjectedText { query, name, filter, limit, view, respond } => {
                let result = read_projected_text_in_tx(
                    &mut connection,
                    &query,
                    &name,
                    filter.as_deref(),
                    limit,
                    view,
                    &wal_attribution,
                    worker_idx,
                );
                finish_reader_request(&connection, &wal_attribution, worker_idx);
                let _ = respond.send(result);
            }
            ReaderRequest::Search(request) => {
                let SearchReaderRequest { work, respond } = *request;
                #[cfg(feature = "tc5-benchmark")]
                tc5_benchmark::record_search_route();
                let result = read_search_work_in_tx(
                    &mut connection,
                    work,
                    NoEvidenceCapture,
                    &wal_attribution,
                    worker_idx,
                );
                finish_reader_request(&connection, &wal_attribution, worker_idx);
                // Receiver may have been dropped if the caller went
                // away; nothing to do in that case.
                let _ = respond.send(result);
            }
            ReaderRequest::SearchEvidence(request) => {
                let EvidenceSearchReaderRequest { work, frozen, include_explanation, respond } =
                    *request;
                let result = read_search_work_in_tx(
                    &mut connection,
                    work,
                    EvidenceCapture { frozen, include_explanation, graph_origins: HashMap::new() },
                    &wal_attribution,
                    worker_idx,
                );
                finish_reader_request(&connection, &wal_attribution, worker_idx);
                let _ = respond.send(result);
            }
            ReaderRequest::GetById { logical_ids, view, respond } => {
                let result = read_get_by_id_in_tx(
                    &mut connection,
                    &logical_ids,
                    &view,
                    &wal_attribution,
                    worker_idx,
                );
                finish_reader_request(&connection, &wal_attribution, worker_idx);
                let _ = respond.send(result);
            }
            ReaderRequest::ReadCollection { collection, after_id, limit, respond } => {
                let result = read_collection_in_tx(
                    &mut connection,
                    &collection,
                    after_id,
                    limit,
                    &wal_attribution,
                    worker_idx,
                );
                finish_reader_request(&connection, &wal_attribution, worker_idx);
                let _ = respond.send(result);
            }
            ReaderRequest::ReadList { kind, predicates, limit, view, respond } => {
                let result = read_list_in_tx(
                    &mut connection,
                    &kind,
                    &predicates,
                    limit,
                    &view,
                    &wal_attribution,
                    worker_idx,
                );
                finish_reader_request(&connection, &wal_attribution, worker_idx);
                let _ = respond.send(result);
            }
            ReaderRequest::ReadCanonicalPage(request) => {
                let result = read_canonical_page_in_tx(
                    &mut connection,
                    &request.kind,
                    &request.frozen,
                    &request.page,
                    &wal_attribution,
                    worker_idx,
                );
                finish_reader_request(&connection, &wal_attribution, worker_idx);
                let _ = request.respond.send(result);
            }
            ReaderRequest::ReadOperationalState(request) => {
                let result = read_operational_state_in_tx(
                    &mut connection,
                    &request.collection,
                    &request.record_key,
                    request.frozen.as_ref(),
                    &wal_attribution,
                    worker_idx,
                );
                finish_reader_request(&connection, &wal_attribution, worker_idx);
                let _ = request.respond.send(result);
            }
            ReaderRequest::ReadOperationalStatePage(request) => {
                let result = read_operational_state_page_in_tx(
                    &mut connection,
                    &request.collection,
                    &request.frozen,
                    &request.page,
                    &wal_attribution,
                    worker_idx,
                );
                finish_reader_request(&connection, &wal_attribution, worker_idx);
                let _ = request.respond.send(result);
            }
            #[cfg(feature = "test-hooks")]
            ReaderRequest::ReadCanonicalPageBaseline(request) => {
                let result = read_canonical_page_baseline_in_tx(
                    &mut connection,
                    &request.kind,
                    &request.context,
                    request.limit,
                    &wal_attribution,
                    worker_idx,
                );
                finish_reader_request(&connection, &wal_attribution, worker_idx);
                let _ = request.respond.send(result);
            }
            ReaderRequest::GraphNeighbors { root_logical_id, depth, direction, view, respond } => {
                let result = graph_neighbors_in_tx(
                    &mut connection,
                    &root_logical_id,
                    depth,
                    direction,
                    &view,
                    &wal_attribution,
                    worker_idx,
                );
                finish_reader_request(&connection, &wal_attribution, worker_idx);
                let _ = respond.send(result);
            }
            ReaderRequest::GraphExpand(request) => {
                #[cfg(feature = "test-hooks")]
                if request.test_controls.count_sql_statements {
                    connection.trace_v2(
                        rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
                        Some(graph_expand::count_graph_expand_sql_statement),
                    );
                }
                let result = graph_expand::read_graph_expand_in_tx(
                    &mut connection,
                    &request.request,
                    request.frozen_binding.as_deref(),
                    request.projection_runtime_state,
                    request.evidence_authority.as_ref(),
                    #[cfg(feature = "test-hooks")]
                    &request.test_controls,
                    &wal_attribution,
                    worker_idx,
                );
                #[cfg(feature = "test-hooks")]
                if request.test_controls.count_sql_statements {
                    connection.trace_v2(rusqlite::trace::TraceEventCodes::empty(), None);
                }
                finish_reader_request(&connection, &wal_attribution, worker_idx);
                let _ = request.respond.send(result);
            }
            ReaderRequest::CrossedBoundarySince { since, view, respond } => {
                let result = crossed_boundary_since_in_tx(
                    &mut connection,
                    since,
                    &view,
                    &wal_attribution,
                    worker_idx,
                );
                finish_reader_request(&connection, &wal_attribution, worker_idx);
                let _ = respond.send(result);
            }
            ReaderRequest::SearchExpand {
                search_hits,
                depth,
                view,
                filter,
                frozen_binding,
                respond,
            } => {
                let result = search_expand_in_tx(
                    &mut connection,
                    &search_hits,
                    depth,
                    view,
                    filter.as_deref(),
                    frozen_binding.as_deref(),
                    &wal_attribution,
                    worker_idx,
                );
                finish_reader_request(&connection, &wal_attribution, worker_idx);
                let _ = respond.send(result);
            }
            ReaderRequest::ExplainGraphNeighbors { root_logical_id, depth, direction, respond } => {
                let result = explain_graph_neighbors_in_tx(
                    &mut connection,
                    &root_logical_id,
                    depth,
                    direction,
                    &wal_attribution,
                    worker_idx,
                );
                finish_reader_request(&connection, &wal_attribution, worker_idx);
                let _ = respond.send(result);
            }
            #[cfg(debug_assertions)]
            ReaderRequest::LookasideStatus { respond } => {
                let _ = respond.send(read_lookaside_used_hiwtr(&connection));
            }
            #[cfg(debug_assertions)]
            ReaderRequest::CacheStatus { snapshot_label, respond } => {
                let (hit, miss, used) = read_cache_status(&connection);
                let _ = respond.send((snapshot_label, hit, miss, used));
            }
            #[cfg(debug_assertions)]
            ReaderRequest::SecureDeleteStatus { respond } => {
                let value: i64 =
                    connection.query_row("PRAGMA secure_delete", [], |r| r.get(0)).unwrap_or(-1);
                let _ = respond.send(value);
            }
            #[cfg(any(test, debug_assertions, feature = "test-hooks"))]
            ReaderRequest::HoldWalSnapshot { snapshot_ready, release } => {
                connection.execute_batch("BEGIN DEFERRED").expect("begin reader transaction");
                let _: i64 = connection
                    .query_row("SELECT COUNT(*) FROM canonical_nodes", [], |row| row.get(0))
                    .expect("acquire real reader snapshot");
                wal_attribution.set(
                    WalAttributionRole::ReaderWorker,
                    worker_idx,
                    true,
                    "snapshot_acquired",
                );
                snapshot_ready.wait();
                release.wait();
                connection.execute_batch("COMMIT").expect("release reader snapshot");
                finish_reader_request(&connection, &wal_attribution, worker_idx);
            }
            #[cfg(debug_assertions)]
            ReaderRequest::HoldWalSnapshotBounded { snapshot_ready, release } => {
                connection.execute_batch("BEGIN DEFERRED").expect("begin reader transaction");
                let _: i64 = connection
                    .query_row("SELECT COUNT(*) FROM canonical_nodes", [], |row| row.get(0))
                    .expect("acquire real reader snapshot");
                let _ = snapshot_ready.send(worker_idx);
                let _ = release.recv_timeout(Duration::from_secs(5));
                connection.execute_batch("COMMIT").expect("release reader snapshot");
                finish_reader_request(&connection, &wal_attribution, worker_idx);
            }
            #[cfg(test)]
            ReaderRequest::HoldWalSnapshotWithCommitAck { snapshot_ready, release, committed } => {
                connection.execute_batch("BEGIN DEFERRED").expect("begin reader transaction");
                let _: i64 = connection
                    .query_row("SELECT COUNT(*) FROM canonical_nodes", [], |row| row.get(0))
                    .expect("acquire real reader snapshot");
                wal_attribution.set(
                    WalAttributionRole::ReaderWorker,
                    worker_idx,
                    true,
                    "snapshot_acquired",
                );
                snapshot_ready.wait();
                release.wait();
                connection.execute_batch("COMMIT").expect("release reader snapshot");
                finish_reader_request(&connection, &wal_attribution, worker_idx);
                committed.wait();
            }
            #[cfg(any(test, feature = "test-hooks"))]
            ReaderRequest::WalConnectionInventory { respond } => {
                let _ = respond.send(connection.is_autocommit());
            }
            #[cfg(any(test, feature = "test-hooks"))]
            ReaderRequest::WalNativeStateInventory { respond } => {
                let _ = respond.send(native_connection_state_for_test(
                    &connection,
                    WalAttributionRole::ReaderWorker,
                    worker_idx,
                ));
            }
        }
    }

    // Per `dev/design/engine.md` § Close path, uninstall the profile
    // callback before dropping the connection so SQLite cannot fire
    // one last callback against a `ProfileContext` whose Box is about
    // to free.
    uninstall_profile_callback(&connection);
    drop(connection);
}

/// Finish an attributed reader request after every helper-local SQLite value
/// (including its transaction) has dropped and before its caller can observe
/// the response. Keeping this at the response handoff, rather than at the
/// worker-loop tail, makes the collector represent live SQLite state rather
/// than channel scheduling.
fn finish_reader_request(
    connection: &Connection,
    wal_attribution: &WalAttributionCollector,
    worker_idx: usize,
) {
    wal_attribution.set(WalAttributionRole::ReaderWorker, worker_idx, false, "completed");
    wal_attribution.set(WalAttributionRole::ReaderWorker, worker_idx, false, "idle");
    #[cfg(test)]
    wal_attribution.fire_reader_handoff_pause();
    #[cfg(any(test, feature = "test-hooks"))]
    wal_attribution.fire_reader_completion_pause(connection.is_autocommit());
    #[cfg(not(any(test, feature = "test-hooks")))]
    let _ = connection;
}

/// Read the high-water-mark for `SQLITE_DBSTATUS_LOOKASIDE_USED` on
/// `connection`. The `current` out-param is the live checked-out slot
/// count and decays as transactions finalize, so it is unreliable as
/// post-warmup evidence. The `hiwtr` out-param latches the largest
/// observed `current` value since the last reset and is the right
/// signal that lookaside was honored at any point on this connection.
/// Reset flag is `0` so reading does not clear the high-water mark.
#[cfg(debug_assertions)]
fn read_lookaside_used_hiwtr(connection: &Connection) -> std::os::raw::c_int {
    let mut current: std::os::raw::c_int = 0;
    let mut hiwtr: std::os::raw::c_int = 0;
    // SAFETY: handle is valid; both out pointers are to local stack
    // ints; reset flag 0 is documented as legal.
    unsafe {
        rusqlite::ffi::sqlite3_db_status(
            connection.handle(),
            rusqlite::ffi::SQLITE_DBSTATUS_LOOKASIDE_USED,
            &mut current,
            &mut hiwtr,
            0,
        );
    }
    hiwtr
}

/// Pack 6.G G.3.5 — read the three page-cache pressure counters on
/// `connection`: `SQLITE_DBSTATUS_CACHE_HIT`, `_CACHE_MISS`, and
/// `_CACHE_USED`. Returns `(hit, miss, used_bytes)`. Hit/miss are
/// monotonic counters (reset flag = 0 here); used_bytes is the live
/// page-cache memory footprint at call time. The caller is expected to
/// take pre/post snapshots and do delta arithmetic explicitly.
#[cfg(debug_assertions)]
fn read_cache_status(
    connection: &Connection,
) -> (std::os::raw::c_int, std::os::raw::c_int, std::os::raw::c_int) {
    let mut hit_current: std::os::raw::c_int = 0;
    let mut hit_hiwtr: std::os::raw::c_int = 0;
    let mut miss_current: std::os::raw::c_int = 0;
    let mut miss_hiwtr: std::os::raw::c_int = 0;
    let mut used_current: std::os::raw::c_int = 0;
    let mut used_hiwtr: std::os::raw::c_int = 0;
    // SAFETY: `connection.handle()` returns a valid `*mut sqlite3` for
    // the lifetime of `connection`. All out-pointers are to local stack
    // ints. Reset flag 0 is documented as legal (no counter is reset).
    unsafe {
        rusqlite::ffi::sqlite3_db_status(
            connection.handle(),
            rusqlite::ffi::SQLITE_DBSTATUS_CACHE_HIT,
            &mut hit_current,
            &mut hit_hiwtr,
            0,
        );
        rusqlite::ffi::sqlite3_db_status(
            connection.handle(),
            rusqlite::ffi::SQLITE_DBSTATUS_CACHE_MISS,
            &mut miss_current,
            &mut miss_hiwtr,
            0,
        );
        rusqlite::ffi::sqlite3_db_status(
            connection.handle(),
            rusqlite::ffi::SQLITE_DBSTATUS_CACHE_USED,
            &mut used_current,
            &mut used_hiwtr,
            0,
        );
    }
    // CACHE_HIT / CACHE_MISS are monotonic counters reported in the
    // `current` out-param; CACHE_USED is the live byte count, also in
    // `current`. The hiwtr values are unused for this telemetry.
    (hit_current, miss_current, used_current)
}

impl ReaderWorkerPool {
    pub(crate) fn new(
        connections: Vec<Connection>,
        wal_attribution: Arc<WalAttributionCollector>,
        #[cfg(any(test, feature = "test-hooks"))] managed_connections: Arc<
            ManagedConnectionRegistry,
        >,
    ) -> Self {
        let live_workers = Arc::new(AtomicUsize::new(0));
        let mut senders = Vec::with_capacity(connections.len());
        let mut handles = Vec::with_capacity(connections.len());
        let (ready_tx, ready_rx) = mpsc::sync_channel(connections.len());
        for (idx, connection) in connections.into_iter().enumerate() {
            let (tx, rx) = mpsc::sync_channel::<ReaderRequest>(READER_WORKER_CHANNEL_CAPACITY);
            let live = Arc::clone(&live_workers);
            let attribution = Arc::clone(&wal_attribution);
            let ready = ready_tx.clone();
            #[cfg(any(test, feature = "test-hooks"))]
            let worker_connections = Arc::clone(&managed_connections);
            let handle = thread::Builder::new()
                .name(format!("fathomdb-reader-{idx}"))
                .spawn(move || {
                    reader_worker_loop(
                        connection,
                        rx,
                        live,
                        idx,
                        attribution,
                        ready,
                        #[cfg(any(test, feature = "test-hooks"))]
                        worker_connections,
                    )
                })
                .expect("spawn reader worker");
            senders.push(tx);
            handles.push(handle);
        }
        drop(ready_tx);
        for _ in 0..senders.len() {
            ready_rx.recv().expect("reader worker must register before Engine::open returns");
        }
        Self {
            senders,
            handles: Mutex::new(Some(handles)),
            next: AtomicUsize::new(0),
            shutdown: AtomicBool::new(false),
            live_workers,
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn worker_count(&self) -> usize {
        self.senders.len()
    }

    #[cfg(debug_assertions)]
    pub(crate) fn live_count(&self) -> usize {
        self.live_workers.load(Ordering::SeqCst)
    }

    #[cfg(debug_assertions)]
    pub(crate) fn next_worker_index(&self) -> usize {
        let worker_count = self.senders.len();
        assert!(worker_count > 0, "reader pool must have workers");
        self.next.load(Ordering::Relaxed) % worker_count
    }

    #[cfg(any(test, feature = "test-hooks"))]
    pub(crate) fn wal_connection_inventory_for_test(&self) -> Vec<bool> {
        self.senders
            .iter()
            .map(|sender| {
                let (respond, received) = mpsc::sync_channel(1);
                sender
                    .send(ReaderRequest::WalConnectionInventory { respond })
                    .expect("reader worker must remain live for WAL inventory");
                received.recv().expect("reader worker must report WAL inventory")
            })
            .collect()
    }

    #[cfg(any(test, feature = "test-hooks"))]
    pub(crate) fn wal_native_state_inventory_for_test(&self) -> Vec<NativeConnectionStateFact> {
        const REPLY_TIMEOUT: Duration = Duration::from_millis(250);
        self.senders
            .iter()
            .enumerate()
            .map(|(index, sender)| {
                let deadline = Instant::now() + REPLY_TIMEOUT;
                let (respond, received) = mpsc::sync_channel(1);
                let request = ReaderRequest::WalNativeStateInventory { respond };
                let mut request = Some(request);
                loop {
                    let Some(pending) = request.take() else { break };
                    match sender.try_send(pending) {
                        Ok(()) => break,
                        Err(mpsc::TrySendError::Full(pending)) if Instant::now() < deadline => {
                            request = Some(pending);
                            thread::sleep(Duration::from_millis(1));
                        }
                        Err(mpsc::TrySendError::Full(_)) => {
                            return unavailable_native_connection_state_for_test(
                                WalAttributionRole::ReaderWorker,
                                index,
                                NativeStateReply::Timeout,
                            );
                        }
                        Err(mpsc::TrySendError::Disconnected(_)) => {
                            return unavailable_native_connection_state_for_test(
                                WalAttributionRole::ReaderWorker,
                                index,
                                NativeStateReply::Error("reader_disconnected"),
                            );
                        }
                    }
                }
                match received.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                    Ok(fact)
                        if fact.role == WalAttributionRole::ReaderWorker && fact.index == index =>
                    {
                        fact
                    }
                    Ok(_) => unavailable_native_connection_state_for_test(
                        WalAttributionRole::ReaderWorker,
                        index,
                        NativeStateReply::Error("reader_wrong_role"),
                    ),
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        unavailable_native_connection_state_for_test(
                            WalAttributionRole::ReaderWorker,
                            index,
                            NativeStateReply::Timeout,
                        )
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        unavailable_native_connection_state_for_test(
                            WalAttributionRole::ReaderWorker,
                            index,
                            NativeStateReply::Error("reader_reply_disconnected"),
                        )
                    }
                }
            })
            .collect()
    }

    /// Pack 6.G G.1 — broadcast a `LookasideStatus` request to every
    /// worker (not round-robin) and collect each worker's
    /// `SQLITE_DBSTATUS_LOOKASIDE_USED`. Used only by the debug
    /// integration test for post-warmup lookaside-slot consumption.
    #[cfg(debug_assertions)]
    pub(crate) fn lookaside_used_per_worker(&self) -> Vec<i32> {
        let mut results = Vec::with_capacity(self.senders.len());
        for sender in &self.senders {
            let (tx, rx) = mpsc::sync_channel::<i32>(1);
            if sender.send(ReaderRequest::LookasideStatus { respond: tx }).is_ok() {
                results.push(rx.recv().unwrap_or(-1));
            } else {
                results.push(-1);
            }
        }
        results
    }

    /// Pack 6.G G.3.5 — broadcast a `CacheStatus` request to every
    /// worker and collect each worker's `(cache_hit, cache_miss,
    /// cache_used_bytes)` triple. Same broadcast pattern as G.1's
    /// `lookaside_used_per_worker`. Returns one `CacheStatusReply` per
    /// worker in worker-index order.
    #[cfg(debug_assertions)]
    pub(crate) fn cache_status_per_worker(&self, snapshot_label: &str) -> Vec<CacheStatusReply> {
        let mut results = Vec::with_capacity(self.senders.len());
        for (idx, sender) in self.senders.iter().enumerate() {
            let (tx, rx) = mpsc::sync_channel::<(String, i32, i32, i32)>(1);
            let request = ReaderRequest::CacheStatus {
                snapshot_label: snapshot_label.to_string(),
                respond: tx,
            };
            if sender.send(request).is_ok() {
                if let Ok((label, hit, miss, used)) = rx.recv() {
                    results.push(CacheStatusReply {
                        worker_idx: idx,
                        snapshot_label: label,
                        cache_hit: hit,
                        cache_miss: miss,
                        cache_used_bytes: used,
                    });
                    continue;
                }
            }
            results.push(CacheStatusReply {
                worker_idx: idx,
                snapshot_label: snapshot_label.to_string(),
                cache_hit: -1,
                cache_miss: -1,
                cache_used_bytes: -1,
            });
        }
        results
    }

    /// OPP-12 Phase-1 (0.8.19 Slice 10, design §3 gap-4) — broadcast a
    /// `SecureDeleteStatus` request to every worker and collect each worker's
    /// `PRAGMA secure_delete` value. Same broadcast pattern as G.1's
    /// `lookaside_used_per_worker`. Proves the standing secure_delete flag is ON
    /// on the reader-pool connections, not just the writer.
    #[cfg(debug_assertions)]
    pub(crate) fn secure_delete_per_worker(&self) -> Vec<i64> {
        let mut results = Vec::with_capacity(self.senders.len());
        for sender in &self.senders {
            let (tx, rx) = mpsc::sync_channel::<i64>(1);
            if sender.send(ReaderRequest::SecureDeleteStatus { respond: tx }).is_ok() {
                results.push(rx.recv().unwrap_or(-1));
            } else {
                results.push(-1);
            }
        }
        results
    }

    /// Hot path. Lock-free dispatch: `AtomicUsize::fetch_add` selects
    /// the worker, then a single `SyncSender::send` enqueues the
    /// request. No global mutex is taken on the request path.
    // The `Search` variant contains a SyncSender and boxed fields (filter, raw_query);
    // even after FIX-4 (raw_query: Box<str>), the variant remains large due to the
    // SyncSender channel ownership. The Err return is only ever a no-worker/shutdown
    // signal, never heap-allocated repeatedly, so the allow is justified by the
    // channel ownership model.
    #[allow(clippy::result_large_err)]
    pub(crate) fn dispatch(&self, request: ReaderRequest) -> Result<(), ReaderRequest> {
        if self.shutdown.load(Ordering::Relaxed) {
            return Err(request);
        }
        let n = self.senders.len();
        if n == 0 {
            return Err(request);
        }
        let idx = self.next.fetch_add(1, Ordering::Relaxed) % n;
        self.senders[idx].send(request).map_err(|err| err.0)
    }

    /// Signal every worker to exit and join its thread. Idempotent —
    /// safe to call from `Engine::close` and again from
    /// `ReaderWorkerPool::Drop`.
    pub(crate) fn shutdown(&self) {
        if self.shutdown.swap(true, Ordering::SeqCst) {
            return;
        }
        for sender in &self.senders {
            let _ = sender.send(ReaderRequest::Shutdown);
        }
        if let Ok(mut slot) = self.handles.lock() {
            if let Some(handles) = slot.take() {
                for handle in handles {
                    let _ = handle.join();
                }
            }
        }
    }
}

impl Drop for ReaderWorkerPool {
    fn drop(&mut self) {
        self.shutdown();
    }
}
