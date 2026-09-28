use super::*;

/// Thread-affine reader worker pool (Pack 6 F.0).
///
/// Per `dev/design/engine.md` § Writer / reader split, reader connections
/// must not serialize behind a single mutex. Each worker thread owns
/// exactly one read-only `Connection` for its lifetime; `Connection`
/// objects never cross thread boundaries after startup. `Engine::search`
/// dispatches a request via a per-worker bounded channel using a
/// lock-free round-robin counter on the hot path.
pub(crate) struct ReaderWorkerPool {
    senders: Vec<SyncSender<ReaderRequest>>,
    handles: Mutex<Option<Vec<JoinHandle<()>>>>,
    next: AtomicUsize,
    shutdown: AtomicBool,
    live_workers: Arc<AtomicUsize>,
}

impl std::fmt::Debug for ReaderWorkerPool {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReaderWorkerPool")
            .field("worker_count", &self.senders.len())
            .field("live_workers", &self.live_workers.load(Ordering::Relaxed))
            .field("shutdown", &self.shutdown.load(Ordering::Relaxed))
            .finish()
    }
}

pub(crate) struct SearchReaderRequest {
    work: SearchReaderWork,
    respond: SyncSender<ReaderResponse>,
}

pub(crate) struct EvidenceSearchReaderRequest {
    work: SearchReaderWork,
    frozen: FrozenReadContextV1,
    include_explanation: bool,
    respond: SyncSender<EvidenceReaderResponse>,
}

pub(crate) struct CanonicalPageReaderRequest {
    kind: String,
    frozen: FrozenReadContextV1,
    page: PageRequestV1,
    respond: SyncSender<Result<PageV1<NodeRecord>, PageReaderError>>,
}

pub(crate) struct GraphExpandReaderRequest {
    request: GraphExpandRequestV1,
    frozen_binding: Option<Box<frozen_read::FrozenReadBinding>>,
    projection_runtime_state: ProjectionRuntimeStateV1,
    evidence_authority: Option<evidence::GraphEvidenceAuthority>,
    #[cfg(feature = "test-hooks")]
    test_controls: graph_expand::GraphExpandReaderControlsForTest,
    respond: SyncSender<Result<GraphExpandResultV1, EngineError>>,
}

pub(crate) struct OperationalStateReaderRequest {
    collection: String,
    record_key: String,
    frozen: Option<FrozenReadContextV1>,
    respond: SyncSender<Result<Option<OperationalStateRecordV1>, PageReaderError>>,
}

pub(crate) struct OperationalStatePageReaderRequest {
    collection: String,
    frozen: FrozenReadContextV1,
    page: PageRequestV1,
    respond: SyncSender<Result<PageV1<OperationalStateRecordV1>, PageReaderError>>,
}

#[cfg(feature = "test-hooks")]
pub(crate) struct CanonicalPageBaselineReaderRequest {
    kind: String,
    context: FrozenReadContextV1,
    limit: usize,
    respond: SyncSender<Result<Vec<NodeRecord>, PageReaderError>>,
}

/// One request handled by exactly one reader worker. The response is
/// returned through a fresh oneshot channel so requests cannot be
/// routed to or duplicated across workers.
pub(crate) enum ReaderRequest {
    #[cfg(feature = "tc5-benchmark")]
    /// Benchmark-only direct vector pipeline. This is intentionally separate
    /// from `Search`: it has no text, fusion, graph, or CE fields.
    VectorStage {
        request: tc5_benchmark::VectorStageRequest,
        respond:
            SyncSender<Result<tc5_benchmark::VectorStageResult, tc5_benchmark::VectorStageError>>,
    },
    /// Slice 60 — property-FTS search stays on a reader-owned connection, never
    /// the writer connection. It shares the snapshot-local filter validation of
    /// the hybrid search path.
    SearchProjectedText {
        query: String,
        name: String,
        filter: Option<Box<SearchFilter>>,
        limit: usize,
        view: ReadView,
        respond: SyncSender<ProjectedTextReaderResponse>,
    },
    Search(Box<SearchReaderRequest>),
    /// Slice 50 — opt-in evidence capture uses the same search algorithm but a
    /// distinct response channel and compile-time capture strategy. Ordinary
    /// search therefore carries no evidence branch or allocation.
    SearchEvidence(Box<EvidenceSearchReaderRequest>),
    /// Slice 30 (G2) — active-only point lookup by `logical_id`. Returns one
    /// slot per requested id, in request order, `None` where no active row
    /// carries that id. Its own typed `respond` channel keeps the `Search`
    /// `ReaderResponse` byte-identical (no Search regression).
    GetById {
        logical_ids: Vec<String>,
        /// R-20-RV — the read view this lookup runs under. `ReadView::default()`
        /// is the strict (pre-slice) view.
        view: ReadView,
        respond: SyncSender<rusqlite::Result<Vec<Option<NodeRecord>>>>,
    },
    /// Slice 30 (G3) — paginated op-store read-back over `operational_mutations`
    /// for a `collection`, `ORDER BY id`, with a MANDATORY (already-clamped)
    /// limit + optional after-id cursor.
    ReadCollection {
        collection: String,
        after_id: Option<i64>,
        limit: usize,
        respond: SyncSender<rusqlite::Result<Vec<OpStoreRow>>>,
    },
    /// Slice 35 (G4) — list active canonical nodes of a `kind`, filtered by
    /// zero or more `Predicate`s (AND-combined), up to `limit` rows.
    /// Path validation already happened at `Predicate` construction time;
    /// the worker only compiles + executes parameterized SQL.
    ReadList {
        kind: String,
        predicates: Vec<Predicate>,
        limit: usize,
        /// R-20-RV — the read view this listing runs under.
        view: ReadView,
        respond: SyncSender<rusqlite::Result<Vec<NodeRecord>>>,
    },
    ReadCanonicalPage(Box<CanonicalPageReaderRequest>),
    ReadOperationalState(Box<OperationalStateReaderRequest>),
    ReadOperationalStatePage(Box<OperationalStatePageReaderRequest>),
    #[cfg(feature = "test-hooks")]
    ReadCanonicalPageBaseline(Box<CanonicalPageBaselineReaderRequest>),
    /// Slice 20 (G5) — bounded BFS from a single root node over
    /// `canonical_edges`. Returns the set of reachable nodes (excluding the
    /// root) within `depth` hops, limited to the hard cap 50.
    GraphNeighbors {
        root_logical_id: String,
        depth: u32,
        direction: TraversalDirection,
        /// R-20-RV — the read view applied at EVERY node position of the BFS
        /// CTE (anchor, recursive join, final projection), for every direction.
        view: ReadView,
        respond: SyncSender<rusqlite::Result<Vec<NodeRecord>>>,
    },
    GraphExpand(Box<GraphExpandReaderRequest>),
    /// 0.8.20 Slice 10b (R-20-NV) — nodes that crossed a validity boundary in
    /// `(since, view-instant]`.
    CrossedBoundarySince {
        since: i64,
        view: ReadView,
        respond: SyncSender<rusqlite::Result<Vec<BoundaryCrossing>>>,
    },
    /// Slice 20 (G6) — compose the previous search result with BFS expansion.
    /// Resolves search hit `write_cursor`s to `logical_id`s, runs G5 traversal
    /// for each root, deduplicates, and returns a `SearchExpandResult`.
    SearchExpand {
        search_hits: Vec<SearchHit>,
        depth: u32,
        view: ReadView,
        filter: Option<Box<SearchFilter>>,
        frozen_binding: Option<Box<frozen_read::FrozenReadBinding>>,
        respond: SyncSender<Result<SearchExpandResult, graph_expand::SearchExpandHandlerError>>,
    },
    /// Slice 20 test seam — run `EXPLAIN QUERY PLAN` on the BFS CTE SQL for
    /// the given root/depth/direction and return the plan detail lines.
    #[doc(hidden)]
    ExplainGraphNeighbors {
        root_logical_id: String,
        depth: u32,
        direction: TraversalDirection,
        respond: SyncSender<rusqlite::Result<Vec<String>>>,
    },
    Shutdown,
    /// Pack 6.G G.1 — debug-only request that asks a worker to read its
    /// own connection's `SQLITE_DBSTATUS_LOOKASIDE_USED` and return the
    /// high-water mark (`hiwtr` out-param). Used solely by the integration
    /// test that asserts post-warmup lookaside slots were consumed; not
    /// on any production path.
    #[cfg(debug_assertions)]
    LookasideStatus {
        respond: SyncSender<i32>,
    },
    /// Pack 6.G G.3.5 — debug-only request that asks a worker to read
    /// `SQLITE_DBSTATUS_CACHE_HIT`, `_CACHE_MISS`, and `_CACHE_USED`
    /// off its own connection and return them as `(hit, miss, used_bytes)`.
    /// `snapshot_label` is opaque to the worker; the caller uses it to
    /// distinguish pre/post snapshots in its own bookkeeping.
    #[cfg(debug_assertions)]
    CacheStatus {
        snapshot_label: String,
        respond: SyncSender<(String, i32, i32, i32)>,
    },
    /// OPP-12 Phase-1 (0.8.19 Slice 10, design §3 gap-4) — debug-only request
    /// that asks a worker to read its own connection's `PRAGMA secure_delete`
    /// and return it (`0`/`1`). Used solely by the gap-4 test that asserts the
    /// standing secure_delete flag is ON at EVERY open, not just the writer.
    #[cfg(debug_assertions)]
    SecureDeleteStatus {
        respond: SyncSender<i64>,
    },
    /// Slice 65 test-only real SQLite control. The worker opens a deferred
    /// transaction, executes a query to acquire its WAL snapshot, reports the
    /// rendezvous, and holds the snapshot until released. It is never compiled
    /// into a release SDK artifact.
    // `test` added alongside `debug_assertions`/`test-hooks` so the crate's own
    // `--release --tests` lib-test build (cfg(test) true, debug_assertions
    // false) can still see this variant; it stays absent from any shipped build.
    #[cfg(any(test, debug_assertions, feature = "test-hooks"))]
    #[allow(dead_code)]
    HoldWalSnapshot {
        snapshot_ready: Arc<Barrier>,
        release: Arc<Barrier>,
    },
    /// Slice 80 test-only variant with bounded/disconnect-safe release. The
    /// sender dropping is cancellation, so a failed test cannot strand close.
    #[cfg(debug_assertions)]
    HoldWalSnapshotBounded {
        snapshot_ready: SyncSender<usize>,
        release: Receiver<()>,
    },
    /// Slice 65 follow-on: the acknowledgement fires only after SQLite has
    /// completed `COMMIT` and the collector has returned to idle. This is a
    /// private test diagnostic, not a release signal or SDK synchronization
    /// surface.
    #[cfg(test)]
    HoldWalSnapshotWithCommitAck {
        snapshot_ready: Arc<Barrier>,
        release: Arc<Barrier>,
        committed: Arc<Barrier>,
    },
    /// Slice 65 follow-on: reports this reader's direct SQLite transaction
    /// state for the private complete-connection inventory.
    #[cfg(any(test, feature = "test-hooks"))]
    WalConnectionInventory {
        respond: SyncSender<bool>,
    },
    /// Slice 65 N23-WAL-BINDING-NATIVE-STATE: report direct SQLite state
    /// from the thread that owns this reader connection. The reply carries no
    /// SQL, path, request, or user data.
    #[cfg(any(test, feature = "test-hooks"))]
    WalNativeStateInventory {
        respond: SyncSender<NativeConnectionStateFact>,
    },
}

impl ReaderRequest {
    #[cfg(feature = "tc5-benchmark")]
    pub(crate) fn vector_stage(
        request: tc5_benchmark::VectorStageRequest,
        respond: SyncSender<
            Result<tc5_benchmark::VectorStageResult, tc5_benchmark::VectorStageError>,
        >,
    ) -> Self {
        Self::VectorStage { request, respond }
    }

    pub(crate) fn projected_text(
        query: String,
        name: String,
        filter: Option<SearchFilter>,
        limit: usize,
        view: ReadView,
        respond: SyncSender<ProjectedTextReaderResponse>,
    ) -> Self {
        Self::SearchProjectedText {
            query,
            name,
            filter: filter.map(Box::new),
            limit,
            view,
            respond,
        }
    }

    pub(crate) fn search(work: SearchReaderWork, respond: SyncSender<ReaderResponse>) -> Self {
        Self::Search(Box::new(SearchReaderRequest { work, respond }))
    }

    pub(crate) fn search_evidence(
        work: SearchReaderWork,
        frozen: FrozenReadContextV1,
        include_explanation: bool,
        respond: SyncSender<EvidenceReaderResponse>,
    ) -> Self {
        Self::SearchEvidence(Box::new(EvidenceSearchReaderRequest {
            work,
            frozen,
            include_explanation,
            respond,
        }))
    }

    pub(crate) fn get_by_id(
        logical_ids: Vec<String>,
        view: ReadView,
        respond: SyncSender<rusqlite::Result<Vec<Option<NodeRecord>>>>,
    ) -> Self {
        Self::GetById { logical_ids, view, respond }
    }

    pub(crate) fn read_collection(
        collection: String,
        after_id: Option<i64>,
        limit: usize,
        respond: SyncSender<rusqlite::Result<Vec<OpStoreRow>>>,
    ) -> Self {
        Self::ReadCollection { collection, after_id, limit, respond }
    }

    pub(crate) fn read_list(
        kind: String,
        predicates: Vec<Predicate>,
        limit: usize,
        view: ReadView,
        respond: SyncSender<rusqlite::Result<Vec<NodeRecord>>>,
    ) -> Self {
        Self::ReadList { kind, predicates, limit, view, respond }
    }

    pub(crate) fn canonical_page(
        kind: String,
        frozen: FrozenReadContextV1,
        page: PageRequestV1,
        respond: SyncSender<Result<PageV1<NodeRecord>, PageReaderError>>,
    ) -> Self {
        Self::ReadCanonicalPage(Box::new(CanonicalPageReaderRequest {
            kind,
            frozen,
            page,
            respond,
        }))
    }

    pub(crate) fn operational_state(
        collection: String,
        record_key: String,
        frozen: Option<FrozenReadContextV1>,
        respond: SyncSender<Result<Option<OperationalStateRecordV1>, PageReaderError>>,
    ) -> Self {
        Self::ReadOperationalState(Box::new(OperationalStateReaderRequest {
            collection,
            record_key,
            frozen,
            respond,
        }))
    }

    pub(crate) fn operational_state_page(
        collection: String,
        frozen: FrozenReadContextV1,
        page: PageRequestV1,
        respond: SyncSender<Result<PageV1<OperationalStateRecordV1>, PageReaderError>>,
    ) -> Self {
        Self::ReadOperationalStatePage(Box::new(OperationalStatePageReaderRequest {
            collection,
            frozen,
            page,
            respond,
        }))
    }

    #[cfg(feature = "test-hooks")]
    pub(crate) fn canonical_page_baseline(
        kind: String,
        context: FrozenReadContextV1,
        limit: usize,
        respond: SyncSender<Result<Vec<NodeRecord>, PageReaderError>>,
    ) -> Self {
        Self::ReadCanonicalPageBaseline(Box::new(CanonicalPageBaselineReaderRequest {
            kind,
            context,
            limit,
            respond,
        }))
    }

    pub(crate) fn graph_neighbors(
        root_logical_id: String,
        depth: u32,
        direction: TraversalDirection,
        view: ReadView,
        respond: SyncSender<rusqlite::Result<Vec<NodeRecord>>>,
    ) -> Self {
        Self::GraphNeighbors { root_logical_id, depth, direction, view, respond }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn graph_expand(
        request: GraphExpandRequestV1,
        frozen_binding: Option<Box<frozen_read::FrozenReadBinding>>,
        projection_runtime_state: ProjectionRuntimeStateV1,
        evidence_authority: Option<evidence::GraphEvidenceAuthority>,
        #[cfg(feature = "test-hooks")]
        test_controls: graph_expand::GraphExpandReaderControlsForTest,
        respond: SyncSender<Result<GraphExpandResultV1, EngineError>>,
    ) -> Self {
        Self::GraphExpand(Box::new(GraphExpandReaderRequest {
            request,
            frozen_binding,
            projection_runtime_state,
            evidence_authority,
            #[cfg(feature = "test-hooks")]
            test_controls,
            respond,
        }))
    }

    pub(crate) fn crossed_boundary_since(
        since: i64,
        view: ReadView,
        respond: SyncSender<rusqlite::Result<Vec<BoundaryCrossing>>>,
    ) -> Self {
        Self::CrossedBoundarySince { since, view, respond }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn search_expand(
        search_hits: Vec<SearchHit>,
        depth: u32,
        view: ReadView,
        filter: Option<SearchFilter>,
        frozen_binding: Option<frozen_read::FrozenReadBinding>,
        respond: SyncSender<Result<SearchExpandResult, graph_expand::SearchExpandHandlerError>>,
    ) -> Self {
        Self::SearchExpand {
            search_hits,
            depth,
            view,
            filter: filter.map(Box::new),
            frozen_binding: frozen_binding.map(Box::new),
            respond,
        }
    }

    pub(crate) fn explain_graph_neighbors(
        root_logical_id: String,
        depth: u32,
        direction: TraversalDirection,
        respond: SyncSender<rusqlite::Result<Vec<String>>>,
    ) -> Self {
        Self::ExplainGraphNeighbors { root_logical_id, depth, direction, respond }
    }
}

// G0 Phase-2: the Search response carries a 4th element — the graph-arm frontier
// meter (`GraphFrontierStats`). It rides the internal channel but is dropped before
// `SearchResult` is built (kept OFF the governed surface); the
// `_graph_frontier_stats_for_test` seam captures it. Default (all-zero) on non-graph paths.
// 0.8.8 EXP-OBS (Slice 5): the Search response carries a 5th element — the opt-in
// retrieval `Explanation` (`None` on every default `explain=false` path; `Some`
// only on the `search_explained` path). Like the `GraphFrontierStats` 4th element
// it rides the internal channel as a side-channel; unlike it, the explanation IS
// surfaced (onto `SearchResult.explanation`) when requested.
pub(crate) type ReaderResponse = Result<
    (
        u64,
        Option<SoftFallback>,
        Vec<SearchHit>,
        GraphFrontierStats,
        Option<Explanation>,
        Option<SearchExpandResult>,
    ),
    SearchReaderError,
>;

pub(crate) type EvidenceReaderResponse = Result<EvidenceSearchResultV1, SearchReaderError>;

pub(crate) type ProjectedTextReaderResponse = Result<SearchResult, SearchReaderError>;

/// Pack 6.G G.3.5 — per-worker cache-pressure snapshot. Carried only on
/// the debug-only `CacheStatus` broadcast path and the test accessor;
/// not part of the public 0.6.0 surface.
#[cfg(debug_assertions)]
#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct CacheStatusReply {
    pub worker_idx: usize,
    pub snapshot_label: String,
    pub cache_hit: i32,
    pub cache_miss: i32,
    pub cache_used_bytes: i32,
}

/// Per-worker outbound channel capacity. Round-robin dispatch keeps
/// queue depth at ~0 on hot paths; the small slack absorbs jitter
/// without a runtime mutex.
const READER_WORKER_CHANNEL_CAPACITY: usize = 4;

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
                    EvidenceCapture::new(frozen, include_explanation),
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

    #[cfg(any(test, debug_assertions, feature = "test-hooks"))]
    pub(crate) fn hold_worker_zero_wal_snapshot(
        &self,
        snapshot_ready: Arc<Barrier>,
        release: Arc<Barrier>,
    ) {
        self.senders[0]
            .send(ReaderRequest::HoldWalSnapshot { snapshot_ready, release })
            .expect("reader worker must be live for WAL attribution control");
    }

    #[cfg(debug_assertions)]
    pub(crate) fn hold_worker_zero_wal_snapshot_bounded(
        &self,
        snapshot_ready: SyncSender<usize>,
        release: Receiver<()>,
    ) {
        self.senders[0]
            .send(ReaderRequest::HoldWalSnapshotBounded { snapshot_ready, release })
            .expect("reader worker must be live for independence control");
    }

    #[cfg(test)]
    pub(crate) fn hold_worker_zero_wal_snapshot_with_commit_ack(
        &self,
        snapshot_ready: Arc<Barrier>,
        release: Arc<Barrier>,
        committed: Arc<Barrier>,
    ) {
        self.senders[0]
            .send(ReaderRequest::HoldWalSnapshotWithCommitAck {
                snapshot_ready,
                release,
                committed,
            })
            .expect("reader worker must be live for post-COMMIT WAL acknowledgement");
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
