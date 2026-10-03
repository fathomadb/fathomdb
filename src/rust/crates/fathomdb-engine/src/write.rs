use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteReceipt {
    /// The batch high-water cursor — the `write_cursor` of the last row written
    /// (also the engine's new `next_cursor`). Unchanged from 0.7.x.
    pub cursor: u64,
    /// G0 (Slice 15) — the per-row `write_cursor` of each row in the batch, 1:1
    /// with input order. This is the `write_cursor`-as-row-id identity carrier
    /// (HITL-accepted for 0.8.0; a dedicated `row_id` is deferred). For an
    /// N-row batch this is `[cursor-N+1, …, cursor]`.
    pub row_cursors: Vec<u64>,
    /// G8 (Slice 20 / F10) — count of edge endpoints in this batch that point at
    /// a non-existent **or superseded** canonical node. An endpoint is dangling
    /// when no **active** node (`superseded_at IS NULL`) carries its `logical_id`;
    /// `from_id` and `to_id` are probed independently, so one edge contributes 0,
    /// 1, or 2. This is **informational** (default FLAG-AND-COUNT: the batch
    /// commits regardless) and `0` whenever the batch committed no active edges.
    pub dangling_edge_endpoints: u64,
}

/// Batch input shape for [`Engine::write`].
///
/// Marked `#[non_exhaustive]` per ADR-0.6.0-prepared-write-shape; new
/// entity variants land in 0.6.x without a major bump. Adding fields to
/// existing variants remains a binding-coordination change.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum PreparedWrite {
    /// Schema-version-1 node carrying complete canonical or derived provenance.
    ProvenancedNode(ProvenancedNodeV1),
    /// Schema-version-1 derived edge carrying complete provenance.
    ProvenancedEdge(ProvenancedEdgeV1),
    Node {
        kind: String,
        body: String,
        /// REQ-026 / AC-028 / AC-042 recovery seam, made **structurally
        /// mandatory** in 0.8.20 (R-20-E3). Was `Option<String>`; a `None`
        /// landed NULL on disk and produced a row no `excise_source` call could
        /// reach. See [`SourceId`] for why the fix is a type change rather than
        /// a validation check.
        source_id: SourceId,
        /// G0 (Slice 15) — stable cross-re-ingestion identity. `Some(id)`
        /// makes this write a transaction-time supersession of the prior
        /// active version of `(logical_id, kind)` (tombstone-then-insert).
        /// `None` is the legacy/own-identity default: a plain insert with a
        /// NULL `logical_id` (NULL-safe — never collides with other NULLs).
        logical_id: Option<String>,
        /// OPP-12 Phase-1 (0.8.19 Slice 5) — the create-time existence state.
        /// `InitialState::Active` (the [`Default`]) is the back-compat default and
        /// lands `state = 'active'` on disk (value-identical to the migration
        /// step-20 column DEFAULT). `InitialState::Pending` creates a quarantined
        /// node excluded from default retrieval. A `deleted`/`purged` node is
        /// UNREPRESENTABLE at create time (the [`InitialState`] type is the typed
        /// rejection) — those states are reachable only via the Slice-10
        /// `transition`/`purge` verbs.
        state: InitialState,
        /// OPP-12 Phase-1 (0.8.19 Slice 5) — advisory cause for the create-time
        /// `state` (e.g. the quarantine cause for a `pending` node), stored
        /// verbatim in `canonical_nodes.reason`. Engine never interprets it. `None`
        /// lands NULL (the back-compat default).
        reason: Option<String>,
        /// 0.8.20 Slice 15b (TC-34) — world-time validity window, INCLUSIVE lower
        /// bound, INTEGER epoch SECONDS UTC. `None` lands NULL = unbounded below.
        ///
        /// Slice 10b added the `valid_from`/`valid_until` columns, the [`ReadView`]
        /// validity predicate and [`Engine::crossed_boundary_since`] but NO writer,
        /// so a window could only be authored with raw SQL. These two fields are
        /// that writer. They are deliberately FIELDS rather than a new verb,
        /// exactly as [`PreparedWrite::Edge`] already carries `t_valid`/`t_invalid`:
        /// the governed command surface is unchanged.
        ///
        /// The pair is validated together — see `valid_until`.
        valid_from: Option<i64>,
        /// 0.8.20 Slice 15b (TC-34) — world-time validity window, EXCLUSIVE upper
        /// bound, INTEGER epoch SECONDS UTC. `None` lands NULL = unbounded above.
        ///
        /// The window is half-open `[valid_from, valid_until)`, matching the read
        /// predicate in `ReadView::validity_sql` exactly. Because it is half-open,
        /// a pair with `valid_from >= valid_until` describes an EMPTY window that no
        /// instant can ever satisfy — so [`Engine::write`] refuses it with
        /// [`EngineError::WriteValidation`] rather than storing a row that no
        /// default read could ever return. A ONE-SIDED window is never empty and is
        /// never refused, however extreme its single bound.
        ///
        /// **BREAKING (0.8.20 Slice 22, decision #18).** This refusal used to be
        /// [`EngineError::InvalidArgument`] NAMING both bounds. It is now the
        /// message-less `WriteValidation` unit variant — the one family the
        /// taxonomy of record assigns to a malformed submitted write SHAPE — so
        /// **the offending bounds are no longer carried in the error**. A caller
        /// that parsed them out must validate the pair before calling.
        valid_until: Option<i64>,
    },
    Edge {
        kind: String,
        from: String,
        to: String,
        /// REQ-026 / AC-028 / AC-042 recovery seam — see Node. Structurally
        /// mandatory since 0.8.20 (R-20-E3).
        source_id: SourceId,
        /// G0 (Slice 15) — see Node. Supersession semantics are identical on
        /// edges (keyed by `(logical_id, kind)`).
        logical_id: Option<String>,
        /// G11 (Slice 15) — the fact/relationship text. When `Some`, triggers
        /// FTS projection into `search_index_edges` and vector projection via
        /// the projection scheduler (kind `"edge_fact"`). Also triggers
        /// invalidate-not-accumulate on `(from_id, to_id, kind)`.
        body: Option<String>,
        /// G11 (Slice 15) — event valid-time. NULL = unknown / still valid.
        ///
        /// **TC-33 (HITL-RATIFIED 2026-07-21): INTEGER epoch seconds (UTC), not
        /// ISO-8601.** This is the GOVERNED SDK WRITE SURFACE, which carries the
        /// same representation as storage. ISO-8601 survives ONLY on the BYO-LLM
        /// extractor wire (`fathomdb.extract.v1`), where
        /// `normalize_extractor_timestamp` converts it with hard rejection.
        t_valid: Option<i64>,
        /// G11 (Slice 15) — event invalid-time. NULL = still valid.
        ///
        /// **TC-33: INTEGER epoch seconds (UTC)** — see `t_valid`. The
        /// NULL-means-still-valid semantic is load-bearing and unchanged, which
        /// is why the schema pins the type with a `typeof` CHECK rather than
        /// `NOT NULL`.
        t_invalid: Option<i64>,
        /// G11 (Slice 15) — extraction confidence ∈ [0.0, 1.0]. NULL for
        /// non-BYO-LLM-ingested edges.
        confidence: Option<f64>,
        /// G11 (Slice 15) — opaque model/provider id from the BYO-LLM harness
        /// `ready.model` field. NULL for non-BYO-LLM edges.
        extractor_model_id: Option<String>,
        /// R3 (Slice 30, SCHEMA-GATE-1, HITL-SIGNED 2026-06-13) — set when the
        /// ELPS extractor defaulted this edge's `t_valid` to `created_at` rather
        /// than deriving it from the document text. Such edges have untrustworthy
        /// event times and are excluded from graph-arm BFS temporal queries.
        /// `None`/`false` = not a fallback; `Some(true)` = fallback.
        temporal_fallback: Option<bool>,
    },
    OpStore {
        collection: String,
        record_key: String,
        schema_id: Option<String>,
        body: String,
    },
    AdminSchema {
        name: String,
        kind: String,
        schema_json: String,
        retention_json: String,
    },
}

pub(crate) fn storage_write_shape(write: &PreparedWrite) -> std::borrow::Cow<'_, PreparedWrite> {
    match write {
        PreparedWrite::ProvenancedNode(node) => std::borrow::Cow::Owned(PreparedWrite::Node {
            kind: node.kind.clone(),
            body: node.body.clone(),
            source_id: node.source_id.clone(),
            logical_id: node.logical_id.clone(),
            state: node.state,
            reason: node.reason.clone(),
            valid_from: node.valid_from,
            valid_until: node.valid_until,
        }),
        PreparedWrite::ProvenancedEdge(edge) => std::borrow::Cow::Owned(PreparedWrite::Edge {
            kind: edge.kind.clone(),
            from: edge.from.clone(),
            to: edge.to.clone(),
            source_id: edge.source_id.clone(),
            logical_id: edge.logical_id.clone(),
            body: edge.body.clone(),
            t_valid: edge.t_valid,
            t_invalid: edge.t_invalid,
            confidence: edge.confidence,
            extractor_model_id: edge.extractor_model_id.clone(),
            temporal_fallback: edge.temporal_fallback,
        }),
        _ => std::borrow::Cow::Borrowed(write),
    }
}

fn batch_is_admin(batch: &[PreparedWrite]) -> bool {
    !batch.is_empty() && batch.iter().all(|w| matches!(w, PreparedWrite::AdminSchema { .. }))
}

impl Engine {
    /// 0.8.16 Slice 5 / F9 (R-F9-1) — set the caller-supplied `importance` ranking
    /// scalar on the `canonical_nodes` row identified by `write_cursor` (the
    /// interim id `SearchHit.id` carries). Validates `importance ∈ [0.0, 1.0]`,
    /// mirroring the existing `canonical_edges.confidence` write-path check —
    /// an out-of-range value is a deterministic [`EngineError::WriteValidation`].
    ///
    /// The 3-way sentinel: NOT calling this leaves the column `NULL` (never
    /// assigned = graceful-absent, ranks NEUTRAL); `0.0` is the explicit floor;
    /// `(0.0, 1.0]` is an explicit importance. Importance is a caller-supplied
    /// scalar — the engine does NOT compute graph-centrality importance (ADR §4
    /// non-goal). Engine-internal minimal surface for this keystone; SDK (Py/TS)
    /// exposure is a Slice-40 concern.
    pub fn write_node_importance(
        &self,
        write_cursor: u64,
        importance: f64,
    ) -> Result<(), EngineError> {
        if !importance.is_finite() || !(0.0..=1.0).contains(&importance) {
            return Err(EngineError::WriteValidation);
        }
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        dependency_closure::maintain_before_writer(connection)?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        dependency_closure::guard_no_pending_physical(&tx)?;
        tx.execute(
            "UPDATE canonical_nodes SET importance = ?1 WHERE write_cursor = ?2",
            params![importance, write_cursor],
        )
        .map_err(|_| EngineError::Storage)?;
        tx.commit().map_err(|_| EngineError::Storage)?;
        Ok(())
    }

    pub fn write(&self, batch: &[PreparedWrite]) -> Result<WriteReceipt, EngineError> {
        let category = if batch_is_admin(batch) {
            lifecycle::EventCategory::Admin
        } else {
            lifecycle::EventCategory::Writer
        };
        self.emit_event(lifecycle::Phase::Started, category, None);
        let started = Instant::now();
        let outcome = self.write_inner(batch);
        self.detect_slow(started, category);
        match outcome {
            Ok(receipt) => {
                let rows = u64::try_from(batch.len()).unwrap_or(u64::MAX);
                if batch_is_admin(batch) {
                    self.counters.record_admin();
                } else {
                    self.counters.record_write(rows);
                }
                self.emit_event(lifecycle::Phase::Finished, category, None);
                Ok(receipt)
            }
            Err(err) => {
                let code = err.stable_code();
                self.counters.record_error(code);
                // AC-003d: capture-ordinal < raise-ordinal — Failed and Error
                // events both fire before the EngineError returns to the caller.
                self.emit_event(lifecycle::Phase::Failed, category, Some(code));
                self.emit_event(
                    lifecycle::Phase::Failed,
                    lifecycle::EventCategory::Error,
                    Some(code),
                );
                Err(err)
            }
        }
    }

    /// 0.8.20 Slice 20c (R-20-DR remainder) — **late enrolment**, the write-path
    /// half of the C4 rider.
    ///
    /// `enqueue_declared_vector_backfill` enrols the kinds the corpus held AT
    /// DECLARATION TIME. A kind first written AFTERWARDS would otherwise fall
    /// through [`project_canonical_node_row`]'s `kind_is_vector_indexed` gate
    /// straight onto a permanent `'up_to_date'` terminal and be silently,
    /// irrecoverably un-embedded — the identical false-ready barrier, reached by
    /// writing second instead of declaring second.
    ///
    /// **Gated on a usable dense runtime**, which is exactly why this lives on
    /// `Engine` and not inside the projector: with `EmbedderChoice::None` or a
    /// refused vector-equivalence guard there is no dense arm at all, so
    /// enrolling a kind would only queue embeds that cannot safely run. The
    /// declaration still PERSISTS without a usable runtime — it defers until a
    /// later safe open grafts it, or until an idempotent apply in an approved
    /// session. This mirrors the `run_vector_equivalence_probe` gate.
    ///
    /// Enrolment is an idempotent `INSERT OR IGNORE`, so running it on the writer
    /// connection just OUTSIDE the batch transaction is safe: if the batch then
    /// fails, the workspace is left having enrolled a kind for which no row
    /// exists — inert.
    ///
    /// fix-1 (codex §9 [P2]) — the `vector_projection_declared` probe below is
    /// what stops this path re-enrolling immediately after
    /// `unenrol_registry_vector_node_kinds` has run: the inverse removes the
    /// registry row, and with no active declaration this returns without
    /// re-adding it. (The kind registry DOES now have delete paths: that one, plus
    /// the Slice-21 fix-1 reconciliation
    /// [`reconcile_inert_vector_enrolments_on_boot`]. Both are gated on the SAME
    /// predicate this probe reads, so neither can be undone by a later write.)
    ///
    /// Cost: the registry probe is skipped entirely once the kind is enrolled, so
    /// a workspace with a live dense arm pays nothing; a workspace that never
    /// declared a vector projection pays two `prepare_cached` `EXISTS` probes per
    /// batch-row of an unenrolled kind. (Slice 21c / `TC-71` made that probe
    /// require the `searchable` ROLE, and deliberately kept the second `EXISTS`
    /// as a fast negative so this cost is unchanged — see
    /// [`vector_projection_declared`].)
    ///
    /// fix-2 (codex §9 [P2]) — a late enrolment now runs the SAME stranded-row
    /// treatment the declare-time door runs ([`reenqueue_stranded_vector_rows`]),
    /// and returns `true` iff that re-enqueued anything. Enrolling a kind while
    /// enqueueing ONLY the batch's own row left every earlier row of that kind
    /// holding its permanent `'up_to_date'` terminal with no vector, so once the
    /// new row drained readiness reported `ready` with pre-existing vector-eligible
    /// rows unembedded — a FALSE READY. Reached, for instance, by a database that
    /// persisted the declaration while opened WITHOUT an embedder and then reopened
    /// WITH one and wrote before re-applying the projection.
    ///
    /// fix-5 (codex §9 round 4 [P2]) — the registry INSERT and the un-stranding
    /// commit as ONE `BEGIN IMMEDIATE`…`COMMIT` (the shape
    /// [`rederive_projections_on_boot`] and
    /// [`reproject_search_index_after_tokenizer_upgrade`] already use). fix-2 ran
    /// them as two, and that window is not benign: a crash or a failed repair in
    /// between leaves the kind REGISTERED with the older rows still holding their
    /// `'up_to_date'` terminals and no vectors — and that state is SELF-SEALING,
    /// because `kind_is_vector_indexed` is then true, so every later write skips
    /// this path and therefore skips the repair, while readiness reads `ready` for
    /// rows nothing will ever embed. Only a manual re-apply of the projection
    /// recovers it. No marker table and no new recovery path: the two statements
    /// simply share a transaction.
    ///
    /// Returns the kinds whose enrolment must be committed with the batch. This
    /// pre-pass is read-only: provenance validation still runs inside
    /// `commit_batch`, so making enrolment part of that same transaction is what
    /// keeps a rejected write mutation-free.
    pub(crate) fn batch_vector_kinds_needing_enrolment(
        &self,
        connection: &Connection,
        batch: &[PreparedWrite],
    ) -> Result<Vec<String>, EngineError> {
        if !self.usable_dense_runtime() {
            return Ok(Vec::new());
        }
        // READ-ONLY pre-pass. Nothing is written here, so the overwhelmingly
        // common case — every kind in the batch already enrolled, or no vector
        // projection declared at all — still pays only the probes it paid before
        // and never takes a write lock.
        let mut to_enrol: Vec<String> = Vec::new();
        for write in batch {
            let write = storage_write_shape(write);
            // Only `Node` writes: edge bodies enrol `'edge_fact'` themselves in
            // `project_canonical_edge_row` (G11), unconditionally and already.
            let PreparedWrite::Node { kind, .. } = write.as_ref() else { continue };
            if to_enrol.contains(kind) {
                continue;
            }
            if self.vector_kind_needs_enrolment(connection, kind, RowKind::Leaf)? {
                to_enrol.push(kind.clone());
            }
        }
        Ok(to_enrol)
    }

    /// 0.8.20 Slice 20c — would enrolling `kind` be correct here? The READ-ONLY
    /// half of a late enrolment; [`Engine::enrol_and_unstrand`] is the write half.
    /// The live-embedder precondition is the CALLER's (see
    /// [`Engine::batch_vector_kinds_needing_enrolment`]).
    pub(crate) fn vector_kind_needs_enrolment(
        &self,
        connection: &Connection,
        kind: &str,
        row_kind: RowKind,
    ) -> Result<bool, EngineError> {
        // `graph` rows are lexically searchable but NEVER embedded
        // (`index_targets_for_row_kind`), so they must not drag their kind into
        // the vector registry — that would start embedding every other row of
        // that kind.
        if !index_targets_for_row_kind(row_kind).vector {
            return Ok(false);
        }
        // fix-2 (codex §9 [P1]) — the SAME restriction the declare-time door
        // applies, from the SAME predicate, so the two cannot drift: a kind the
        // vector writer cannot commit must never be enrolled, or the projection
        // worker wedges on it forever. See [`kind_is_vector_committable`].
        if !kind_is_vector_committable(kind) {
            return Ok(false);
        }
        if kind_is_vector_indexed(connection, kind)? {
            return Ok(false);
        }
        if !vector_projection_declared(connection).map_err(|_| EngineError::Storage)? {
            return Ok(false);
        }
        Ok(true)
    }

    /// 0.8.20 Slice 20c fix-5 (codex §9 round 4 [P2]) — the WRITE half of a LATE
    /// enrolment: register the kinds AND repair the rows they strand, in ONE
    /// transaction. Returns `true` iff the repair re-enqueued anything (the caller
    /// must then `notify_new_work()`, since those rows are outside its batch).
    ///
    /// Used by the `#[doc(hidden)]` `write_canonical_row_with_kind_for_test`;
    /// production batch writes perform the same operations inside `commit_batch`
    /// so provenance rejection also rolls them back.
    ///
    /// `register_vector_kind` is `INSERT OR IGNORE` and
    /// [`reenqueue_stranded_vector_rows`] is idempotent, so the read-only pre-pass
    /// that chose `kinds` does not need re-validating under the write lock: the
    /// worst a stale decision costs is one no-op `MIN` probe.
    pub(crate) fn enrol_and_unstrand(
        &self,
        connection: &Connection,
        kinds: &[&str],
    ) -> Result<bool, EngineError> {
        connection.execute_batch("BEGIN IMMEDIATE").map_err(|_| EngineError::Storage)?;
        let result = (|| -> rusqlite::Result<bool> {
            for kind in kinds {
                register_vector_kind(connection, kind)?;
            }
            reenqueue_stranded_vector_rows(connection)
        })();
        match result {
            Ok(enqueued) => {
                connection.execute_batch("COMMIT").map_err(|_| EngineError::Storage)?;
                Ok(enqueued)
            }
            Err(_) => {
                let _ = connection.execute_batch("ROLLBACK");
                Err(EngineError::Storage)
            }
        }
    }

    fn write_inner(&self, batch: &[PreparedWrite]) -> Result<WriteReceipt, EngineError> {
        self.ensure_open()?;

        if batch.is_empty() {
            return Err(EngineError::WriteValidation);
        }

        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        #[cfg(feature = "test-hooks")]
        let abort_next_commit_for_test =
            crate::write_commit::take_write_commit_abort_marker_for_test(connection)
                .map_err(|_| EngineError::Storage)?;
        dependency_closure::maintain_before_writer(connection)?;
        let plans = validate_batch(connection, batch)?;
        validate_nested_projection_sources_for_write(connection, batch)?;
        // The enrolment decision is read-only here. Registration and stranded-row
        // repair commit inside the canonical batch transaction below, after all
        // provenance checks have succeeded.
        let vector_kinds_to_enrol = self.batch_vector_kinds_needing_enrolment(connection, batch)?;
        let projection_jobs = collect_projection_jobs(connection, batch)?;
        #[cfg(debug_assertions)]
        if self.force_next_commit_failure.swap(false, Ordering::SeqCst) {
            return Err(EngineError::Storage);
        }
        // One cursor per row. `base_cursor` is the last committed cursor;
        // row i in the batch gets cursor `base_cursor + i + 1`, and the
        // batch's final cursor (returned in WriteReceipt and stored as
        // the new `next_cursor`) is `base_cursor + batch.len()`. Sharing
        // one cursor across the batch previously collapsed every vec0
        // INSERT onto the same rowid via `INSERT OR IGNORE` — see
        // `dev/notes/0.7.0-engine-batch-vec0-collapse.md`.
        let base_cursor = self.next_cursor.load(Ordering::SeqCst);
        let increment = u64::try_from(batch.len()).unwrap_or(u64::MAX);
        let last_cursor = base_cursor.saturating_add(increment);
        // G11 (Slice 15) — edge bodies also need projection-runtime notification.
        // `collect_projection_jobs` only tracks Node items (pre-fetched for
        // cursor assignment); edge bodies update `_fathomdb_projection_state` in
        // `commit_batch` but need the scanner to wake up via `notify_new_work`.
        let has_edge_body_work = batch.iter().any(|write| {
            matches!(storage_write_shape(write).as_ref(), PreparedWrite::Edge { body: Some(_), .. })
        });
        let (dangling_edge_endpoints, unstranded, _closure_ids) = match commit_batch(
            connection,
            batch,
            &plans,
            base_cursor,
            self.provenance_row_cap.load(Ordering::Relaxed),
            &vector_kinds_to_enrol,
            #[cfg(feature = "test-hooks")]
            abort_next_commit_for_test,
        ) {
            Ok(count) => count,
            Err(CommitBatchError::Sql(err)) => {
                self.emit_sqlite_internal_error(&err);
                return Err(EngineError::Storage);
            }
            Err(CommitBatchError::Provenance(error)) => {
                return Err(EngineError::Provenance(error));
            }
            Err(CommitBatchError::Engine(error)) => return Err(error),
            Err(CommitBatchError::WriterPoisoned) => {
                self.closed.store(true, Ordering::SeqCst);
                return Err(EngineError::Storage);
            }
        };
        let pending_projection = !projection_jobs.is_empty()
            || !vector_kinds_to_enrol.is_empty()
            || has_edge_body_work
            || unstranded;
        self.next_cursor.store(last_cursor, Ordering::SeqCst);
        if pending_projection {
            self.projection_runtime.notify_new_work();
        }

        // G0 — surface the per-row cursors (1:1 with input order). Row i got
        // `base_cursor + i + 1`, matching the allocation in `commit_batch`.
        let row_cursors = (0..batch.len())
            .map(|i| base_cursor.saturating_add((i as u64).saturating_add(1)))
            .collect();
        Ok(WriteReceipt { cursor: last_cursor, row_cursors, dangling_edge_endpoints })
    }
}

impl Engine {
    /// EXP-S (0.8.14 Slice 5, D1) — write one canonical node row carrying an
    /// explicit structural `row_kind` (leaf/coverage/graph), routing the index
    /// projection through the SAME `row_kind -> index-target` dispatch seam
    /// (`project_canonical_node_row`) as the production `leaf` write path.
    ///
    /// This is the internal-only writer for `coverage`/`graph` rows (there is no
    /// public SDK surface for `row_kind` in 0.8.14). Cursor assignment preserves
    /// the `rowid == write_cursor == cursor` determinism identity. When the row
    /// projects into an async vector index, the worker pool is notified so the
    /// embed is scheduled exactly as for a normal write.
    #[doc(hidden)]
    pub fn write_canonical_row_with_kind_for_test(
        &self,
        kind: &str,
        body: &str,
        row_kind: RowKind,
    ) -> Result<WriteReceipt, EngineError> {
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;

        // R-20-E3 / design §4 item 6 — this writer BYPASSES `PreparedWrite`, so
        // the `SourceId` newtype cannot reach it; before 0.8.20 it inserted a
        // literal NULL `source_id` and produced a row that no `excise_source`
        // call could reach. Engine-derived rows instead take a reserved
        // `_engine:*` provenance, keyed by the structural role that produced
        // them, so they are both erasable and distinguishable from caller data.
        let engine_provenance = SourceId::engine_derived(row_kind.as_str());

        // 0.8.20 Slice 20c — same late enrolment the governed write path takes
        // (`Engine::batch_vector_kinds_needing_enrolment`), so this internal writer does not
        // silently diverge into the false-ready barrier for `coverage` rows. The
        // live-embedder precondition is checked here, as that caller does; the
        // `row_kind` gate keeps `graph` rows out of the vector registry.
        //
        // fix-2 (codex §9 [P2]) — including the un-stranding half, so this door
        // cannot diverge from the other one either. fix-5 (codex §9 round 4 [P2])
        // — and both halves commit as ONE transaction, via the same shared
        // `enrol_and_unstrand`.
        let unstranded = if self.usable_dense_runtime()
            && self.vector_kind_needs_enrolment(connection, kind, row_kind)?
        {
            self.enrol_and_unstrand(connection, &[kind])?
        } else {
            false
        };

        let cursor = self.next_cursor.load(Ordering::SeqCst).saturating_add(1);
        let enqueued = {
            let tx = connection.transaction().map_err(|_| EngineError::Storage)?;
            // 0.8.20 Slice 15b (TC-34) — this writer takes NO validity window, and
            // that is deliberate rather than an oversight. It is a `#[doc(hidden)]`
            // test-only writer for the internal `coverage`/`graph` row kinds, which
            // have no public SDK surface at all (see the doc comment above); the
            // caller-facing authoring path is `PreparedWrite::Node`, handled in
            // `commit_batch`. Omitting the columns binds NULL — the migration
            // step-22 default and the UNBOUNDED reading — so engine-derived rows
            // stay valid at every instant, which is the only correct answer for a
            // structural row that no caller can address a window to.
            tx.execute(
                "INSERT INTO canonical_nodes(write_cursor, kind, body, source_id, logical_id, row_kind)
                 VALUES(?1, ?2, ?3, ?4, NULL, ?5)",
                params![cursor, kind, body, engine_provenance.as_str(), row_kind.as_str()],
            )
            .map_err(|_| EngineError::Storage)?;
            let enqueued = project_canonical_node_row(
                &tx,
                cursor,
                kind,
                body,
                row_kind,
                ProjectionPass::Write,
                // This #[doc(hidden)] writer inserts with the column DEFAULT
                // `state = 'active'` (no state column in its INSERT), so the row
                // is always active and its attributes project.
                true,
            )
            .map_err(|_| EngineError::Storage)?;
            advance_projection_cursor(&tx).map_err(|_| EngineError::Storage)?;
            tx.commit().map_err(|_| EngineError::Storage)?;
            enqueued
        };
        self.next_cursor.store(cursor, Ordering::SeqCst);
        if enqueued || unstranded {
            self.projection_runtime.notify_new_work();
        }
        Ok(WriteReceipt { cursor, row_cursors: vec![cursor], dangling_edge_endpoints: 0 })
    }
}
