use super::*;

/// 0.8.0 Slice 5 (G1) — schema version that introduces the global FTS5
/// tokenizer-default upgrade (`SCHEMA_VERSION` 11, migration step 11). A DB
/// migrated to (or past) this version re-tokenizes `search_index` from
/// canonical source rows on open (the drop+recreate leaves the FTS index
/// empty). Repair is keyed off the completion marker below — NOT off crossing
/// the step boundary — so it is crash-retryable (see
/// `SEARCH_INDEX_TOKENIZER_REPROJECT_MARKER_KEY`).
pub(crate) const SEARCH_INDEX_TOKENIZER_SCHEMA_VERSION: u32 = 11;
/// 0.8.0 Slice 5 (G1) fix-1 — `_fathomdb_open_state` key set, in the SAME
/// transaction as the reproject DELETE+INSERT, once the post-tokenizer-upgrade
/// re-tokenization commits durably. Step 11 commits `user_version = 11` with an
/// EMPTY `search_index` in its own transaction; the reproject runs in a later
/// transaction on open. A crash in that window leaves a durable `user_version =
/// 11` + empty index. Gating repair on a boundary crossing (`before < 11`)
/// would skip it on the next open (it sees `before == 11`), stranding the index
/// empty forever. Gating on this marker's ABSENCE instead makes repair
/// idempotent and crash-retryable: written atomically with the reindex, so a
/// crash before commit leaves no marker and the next open re-runs.
pub(crate) const SEARCH_INDEX_TOKENIZER_REPROJECT_MARKER_KEY: &str =
    "search_index_tokenizer_reproject_complete";
pub(crate) fn restore_registered_derived_projections(
    tx: &Connection,
    cursor: i64,
    kind: &str,
    body: &str,
    row_kind: &str,
) -> Result<bool, EngineError> {
    let registered: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_artifact_revisions r \
               JOIN _fathomdb_source_dependencies d ON d.derived_revision_id=r.revision_id \
               WHERE r.artifact_class='node' AND r.write_cursor=?1)",
            [cursor],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    if !registered {
        return Ok(false);
    }
    let legacy_fts: Vec<(String, String)> = tx
        .prepare("SELECT body,kind FROM search_index WHERE write_cursor=?1")
        .and_then(|mut statement| {
            statement.query_map([cursor], |row| Ok((row.get(0)?, row.get(1)?)))?.collect()
        })
        .map_err(|_| EngineError::Storage)?;
    let fielded_fts: Vec<(String, String, String)> = tx
        .prepare("SELECT kind,body,status FROM search_index_v2 WHERE write_cursor=?1")
        .and_then(|mut statement| {
            statement
                .query_map([cursor], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
                .collect()
        })
        .map_err(|_| EngineError::Storage)?;
    let expected_status: String = tx
        .query_row(
            "SELECT CASE WHEN json_valid(?1) \
               THEN COALESCE(json_extract(?1,'$.status'),'') ELSE '' END",
            [body],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    match (legacy_fts.as_slice(), fielded_fts.as_slice()) {
        ([(stored_body, stored_kind)], [(stored_kind_v2, stored_body_v2, stored_status)])
            if stored_body == body
                && stored_kind == kind
                && stored_kind_v2 == kind
                && stored_body_v2 == body
                && stored_status == &expected_status => {}
        ([], []) => {
            let row_kind = match row_kind {
                "leaf" => RowKind::Leaf,
                "coverage" => RowKind::Coverage,
                "graph" => RowKind::Graph,
                _ => return Err(EngineError::Storage),
            };
            project_canonical_node_row(
                tx,
                u64::try_from(cursor).map_err(|_| EngineError::Storage)?,
                kind,
                body,
                row_kind,
                ProjectionPass::FtsOnly,
                true,
            )
            .map_err(|_| EngineError::Storage)?;
        }
        _ => return Err(projection_generation::corruption_error()),
    }
    if !vector_projection_declared(tx).map_err(|_| EngineError::Storage)?
        || !matches!(row_kind, "leaf" | "coverage")
        || !kind_is_vector_committable(kind)
    {
        return Ok(false);
    }
    let terminal: Option<String> = tx
        .query_row(
            "SELECT state FROM _fathomdb_projection_terminal WHERE write_cursor=?1",
            [cursor],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let sidecar: Option<String> = tx
        .query_row(
            "SELECT kind FROM _fathomdb_vector_rows WHERE write_cursor=?1",
            [cursor],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let physical: Option<(String, String)> = tx
        .query_row("SELECT source_type,kind FROM vector_default WHERE rowid=?1", [cursor], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let expected_source_type = resolve_source_type(kind)?;
    if terminal.as_deref() == Some("up_to_date")
        && sidecar.as_deref() == Some(kind)
        && physical.as_ref().is_some_and(|(source_type, physical_kind)| {
            source_type == expected_source_type && physical_kind == kind
        })
    {
        return Ok(false);
    }
    if !matches!(terminal.as_deref(), None | Some("up_to_date"))
        || sidecar.is_some()
        || physical.is_some()
    {
        return Err(projection_generation::corruption_error());
    }
    tx.execute(
        "INSERT OR IGNORE INTO _fathomdb_vector_kinds(kind,profile,created_at) \
         VALUES(?1,'default',0)",
        [kind],
    )
    .map_err(|_| EngineError::Storage)?;
    tx.execute("DELETE FROM _fathomdb_projection_terminal WHERE write_cursor=?1", [cursor])
        .map_err(|_| EngineError::Storage)?;
    tx.execute(
        "INSERT INTO _fathomdb_projection_state(kind,last_enqueued_cursor,updated_at) \
         VALUES(?1,?2,0) ON CONFLICT(kind) DO UPDATE SET \
         last_enqueued_cursor=MAX(last_enqueued_cursor,excluded.last_enqueued_cursor)",
        params![kind, cursor],
    )
    .map_err(|_| EngineError::Storage)?;
    let cursor = u64::try_from(cursor).map_err(|_| EngineError::Storage)?;
    if load_projection_cursor(tx).map_err(|_| EngineError::Storage)? >= cursor {
        store_projection_cursor(tx, cursor.saturating_sub(1)).map_err(|_| EngineError::Storage)?;
    }
    Ok(true)
}
pub(crate) struct CanonicalNodeRow {
    pub(crate) cursor: u64,
    pub(crate) kind: String,
    pub(crate) body: String,
    pub(crate) row_kind: RowKind,
    /// fix-2 [P2] — whether this row is in the attribute projection's row set
    /// (`state = 'active' AND superseded_at IS NULL`, the exact `backfill_attribute`
    /// predicate). A projector-replay rebuild uses this to gate the attribute
    /// projection so it does not re-surface a pending / superseded node's values.
    /// Node-FTS / vector shadows are rebuilt for every row (their stale versions
    /// are excluded by the read-side lifecycle join, unchanged from before).
    pub(crate) attr_projected: bool,
}

/// 0.8.0 Slice 5 (G1) — re-tokenize `search_index` from the canonical source
/// rows after the step-11 tokenizer-default upgrade drops + recreates the FTS5
/// virtual table. Projection-only: it reads `canonical_nodes` (the source of
/// truth, untouched) and rewrites the FTS shadow; it performs **no**
/// source-record migration. Every canonical node already carries an FTS row at
/// write time (the projection-time INSERT is unconditional), so reinserting
/// every node exactly reproduces the prior index content under the new
/// tokenizer. Runs in a single transaction on the writer connection before
/// readers spawn.
///
/// Crash-retryable (fix-1): the reindex and its durable completion marker
/// (`SEARCH_INDEX_TOKENIZER_REPROJECT_MARKER_KEY` in `_fathomdb_open_state`)
/// commit together in ONE `BEGIN IMMEDIATE…COMMIT`. A crash before the commit
/// rolls both back, leaving no marker; the next open re-runs. A crash after
/// the commit finds the marker present and skips. Idempotent.
pub(crate) fn reproject_search_index_after_tokenizer_upgrade(
    connection: &Connection,
) -> rusqlite::Result<()> {
    let rows = canonical_node_rows(connection)?;
    connection.execute_batch("BEGIN IMMEDIATE")?;
    let result = (|| {
        // 0.8.20 Slice 5a (R-20-E1) — registry-driven: re-tokenize EVERY
        // node-FTS projection, not just `search_index`. `search_index_v2` uses
        // the SAME tokenizer (`porter unicode61 remove_diacritics 2`), so it is
        // equally invalidated by a tokenizer-default upgrade; before this slice
        // it was neither cleared nor re-tokenized here. Edge FTS is out of scope
        // for this open-path repair (it postdates the step-11 upgrade and is
        // rebuilt by `rebuild_projections`).
        truncate_row_projections_in(connection, &[ProjectionClass::NodeFts])?;
        for row in &rows {
            project_canonical_node_row(
                connection,
                row.cursor,
                &row.kind,
                &row.body,
                row.row_kind,
                ProjectionPass::FtsOnly,
                // FtsOnly never touches the attribute store (predates step 24), so
                // `node_active` is inert here; forward the row's flag anyway (it is
                // the backfill's active-and-non-superseded predicate) so the field
                // has a reader in every build configuration.
                row.attr_projected,
            )?;
        }
        connection.execute(
            "INSERT INTO _fathomdb_open_state(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![SEARCH_INDEX_TOKENIZER_REPROJECT_MARKER_KEY, "1"],
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

/// 0.8.0 Slice 5 (G1) fix-1 — has the post-tokenizer-upgrade re-tokenization
/// committed durably on this DB? Keys off the
/// `SEARCH_INDEX_TOKENIZER_REPROJECT_MARKER_KEY` row written inside the reindex
/// transaction; its absence on a v11 DB means the reindex never committed
/// (fresh-after-step-11 or crash-in-window) and must (re-)run.
///
/// A MISSING `_fathomdb_open_state` table is reported as "complete" (skip the
/// reproject): that table is created by migration step 1, so its absence means
/// the DB never ran our migrations (e.g. a synthetic DB whose `user_version`
/// was stamped to 11 by hand, or a legacy/foreign shape). Such DBs are
/// rejected by the downstream embedder-identity/integrity probes; the reproject
/// must not run — and must not mask those errors — on them. On a genuinely
/// migrated DB the table always exists, so the crash-repair path is unaffected.
pub(crate) fn search_index_tokenizer_reproject_complete(
    connection: &Connection,
) -> rusqlite::Result<bool> {
    match connection.query_row(
        "SELECT value FROM _fathomdb_open_state WHERE key = ?1",
        [SEARCH_INDEX_TOKENIZER_REPROJECT_MARKER_KEY],
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

pub(crate) fn canonical_node_rows(
    connection: &Connection,
) -> rusqlite::Result<Vec<CanonicalNodeRow>> {
    // fix-2 [P2] — also read `state` + `superseded_at` so a replay rebuild can gate
    // the attribute projection to the backfill's row set. `attr_projected` mirrors
    // the exact `backfill_attribute` predicate (`state = 'active' AND
    // superseded_at IS NULL`): a NULL/foreign state is NOT 'active' and so is
    // excluded, identical to the SQL equality.
    let mut statement = connection.prepare(
        "SELECT write_cursor, kind, body, row_kind, state, superseded_at \
         FROM canonical_nodes ORDER BY write_cursor",
    )?;
    let rows = statement.query_map([], |row| {
        let state: Option<String> = row.get::<_, Option<String>>(4)?;
        let superseded_at: Option<i64> = row.get::<_, Option<i64>>(5)?;
        Ok(CanonicalNodeRow {
            cursor: row.get::<_, u64>(0)?,
            kind: row.get::<_, String>(1)?,
            body: row.get::<_, String>(2)?,
            row_kind: row_kind_from_column(&row.get::<_, String>(3)?),
            attr_projected: state.as_deref() == Some("active") && superseded_at.is_none(),
        })
    })?;
    rows.collect()
}

/// 0.8.20 Slice 5a — inverse of [`RowKind::as_str`] for the stored
/// `canonical_nodes.row_kind` column. An unrecognized spelling degrades to
/// `Leaf`, the column DEFAULT and the shape every pre-EXP-S row carries; that
/// keeps a projector replay behavior-identical to the pre-registry rebuild,
/// which ignored `row_kind` entirely.
pub(crate) fn row_kind_from_column(value: &str) -> RowKind {
    match value {
        "coverage" => RowKind::Coverage,
        "graph" => RowKind::Graph,
        _ => RowKind::Leaf,
    }
}
/// EXP-S (0.8.14 Slice 5, D2) — the set of coexisting indexes a `row_kind`
/// projects into. `fts` = the FTS index (`search_index`), written SYNCHRONOUSLY
/// in the write transaction; `vector` = the vec0 vector index, written
/// ASYNCHRONOUSLY by the projection worker pool (and additionally gated per
/// doc-type `kind` by [`kind_is_vector_indexed`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct IndexTargetSet {
    pub(crate) fts: bool,
    pub(crate) vector: bool,
}

/// EXP-S (0.8.14 Slice 5) — the `row_kind -> index-target set` dispatch
/// (ADR-0.8.14 §D2), and the OPP-12 forward-compat seam (ADR-0.8.14 §D5(a) /
/// ledger `TC-1`).
///
/// This is deliberately a per-kind LOOKUP rather than branching inlined at each
/// write call-site: it is the single seam a later declarative OPP-12 projection
/// registry (`dev/design/projection-registry-and-async-embed.md`) would wrap to
/// populate `row_kind -> {filterable, searchable->FTS (same-txn), searchable->
/// vector (async)}` without reshaping the substrate. Per D5, EXP-S implements
/// NO OPP-12 surface here (OPP-12 lands >=0.9.x; re-check at its scheduling) —
/// this function only records the index-target intent so the async-vs-sync split
/// (D5(b)) and the per-kind-extensible terminal-cursor readiness (D5(c)) stay
/// wrappable.
///
/// `Leaf` MUST preserve today's behavior exactly: FTS (sync) + vector (async,
/// gated by `kind_is_vector_indexed`).
pub(crate) fn index_targets_for_row_kind(row_kind: RowKind) -> IndexTargetSet {
    match row_kind {
        // Normal record — identical to pre-EXP-S behavior.
        RowKind::Leaf => IndexTargetSet { fts: true, vector: true },
        // Coverage/summary rows — searchable and embeddable.
        RowKind::Coverage => IndexTargetSet { fts: true, vector: true },
        // Graph structural rows — lexically searchable, not embedded.
        RowKind::Graph => IndexTargetSet { fts: true, vector: false },
    }
}

/// EXP-S (0.8.14 Slice 5, D2/D5) — apply the per-`row_kind` index-target
/// dispatch for one just-inserted canonical node row (write_cursor `cursor`).
///
/// Preserves the OPP-12-shaped split (D5(b)): FTS is written in THIS
/// transaction (same-txn `searchable->FTS`); vector work is only *enqueued*
/// here into `_fathomdb_projection_state` and embedded later, asynchronously,
/// by the projection worker pool (`searchable->vector`). When the row projects
/// into no async vector index, its readiness is terminated up-front (D5(c),
/// per-kind-extensible) so `advance_projection_cursor` can walk past it.
///
/// Returns `true` iff async vector work was enqueued (the caller must then
/// `notify_new_work`). For `RowKind::Leaf` this is behavior-identical to the
/// pre-EXP-S inline node path.
pub(crate) fn project_canonical_node_row(
    tx: &Connection,
    cursor: u64,
    kind: &str,
    body: &str,
    row_kind: RowKind,
    pass: ProjectionPass,
    node_active: bool,
) -> rusqlite::Result<bool> {
    let targets = index_targets_for_row_kind(row_kind);
    if targets.fts && pass.writes_fts() {
        tx.prepare_cached("INSERT INTO search_index(body, kind, write_cursor) VALUES(?1, ?2, ?3)")?
            .execute(params![body, kind, cursor])?;
        // F5 (0.8.14 Slice 10) — same coexisting `searchable->FTS` target also
        // populates the multi-column `search_index_v2` (kind/body/status) so a
        // BM25F query can field-weight the lexical arm. Written SYNCHRONOUSLY in
        // THIS transaction, exactly like `search_index` (rowid==write_cursor
        // identity preserved). The `status` field mirrors the migration-17
        // O(N) re-index: `$.status` from a JSON body, guarded by `json_valid` so
        // non-JSON bodies index an empty status. NOTE (codex fix-1 finding 2):
        // this is F5's OWN `$.status`-derived field for the BM25F `status`
        // column — it is NOT (yet) the value the shipped G10 SearchFilter reads.
        // G10 filtering reads the vec0 `status` column, which is still hardwired
        // to the empty-string sentinel; wiring G10 onto this field is out of
        // scope for F5. Determinism (R-SUB-2) is preserved: the derivation is
        // a pure function of `body`, evaluated in-SQL identically on every run.
        tx.prepare_cached(
            "INSERT INTO search_index_v2(kind, body, status, write_cursor)
             VALUES(
                 ?1,
                 ?2,
                 CASE WHEN json_valid(?2)
                      THEN COALESCE(json_extract(?2, '$.status'), '')
                      ELSE '' END,
                 ?3
             )",
        )?
        .execute(params![kind, body, cursor])?;
    }
    // 0.8.20 Slice 15d (R-20-EAV) — same-transaction attribute projection. Only
    // the full `Write` pass re-derives attributes (see `writes_attributes`): the
    // FtsOnly tokenizer reproject predates step 24 and must not touch the
    // registry/attribute tables; VectorOnly rebuilds only vector shadows. A full
    // operator FTS rebuild uses `Write`, so it re-derives attributes after the
    // truncate.
    //
    // fix-2 [P2]: gated on `node_active`. The at-rest attribute projection tracks
    // EXACTLY the backfill's row set — `state = 'active' AND superseded_at IS NULL`
    // (see `backfill_attribute`). Unlike node-FTS / vector shadows (whose stale
    // versions are excluded by the canonical read path's `superseded_at IS NULL`
    // / `state = 'active'` join), the property tables carry NO read-side lifecycle
    // filter (`property_search_index` is an FTS5 table that cannot), so a pending
    // or superseded node's attribute values would otherwise LEAK into a
    // same-session property filter / property-FTS. The write path passes
    // `state == Active`; a projector-replay rebuild passes `active ∧ non-superseded`
    // per row. Lifecycle transitions maintain the store directly (see
    // `Engine::transition`). Passes where `writes_attributes()` is false ignore the
    // flag entirely.
    if pass.writes_attributes() && node_active {
        project_node_attributes(tx, cursor as i64, body)?;
    }
    // 0.8.20 Slice 20c (R-20-DR remainder) — UNCHANGED, deliberately. Late
    // enrolment of a kind first written AFTER a `searchable→vector` declaration
    // happens in [`Engine::enrol_vector_kind_if_declared`], upstream of this
    // transaction, NOT here: the decision needs the engine's usable-runtime
    // predicate, which a free function holding only a `Connection` cannot see.
    // Enrolling without one would queue embeds that cannot safely run.
    let enqueue_vector = targets.vector && kind_is_vector_indexed(tx, kind).unwrap_or(false);
    if pass.writes_vector_state() {
        if enqueue_vector {
            tx.prepare_cached(
                "INSERT INTO _fathomdb_projection_state(kind, last_enqueued_cursor, updated_at)
                 VALUES(?1, ?2, 0)
                 ON CONFLICT(kind) DO UPDATE SET last_enqueued_cursor = excluded.last_enqueued_cursor",
            )?
            .execute(params![kind, cursor])?;
        } else {
            // Never-vector-projected rows terminate the cursor up-front so
            // `advance_projection_cursor` can advance the readiness watermark.
            record_projection_terminal(tx, cursor, "up_to_date")?;
        }
    }
    Ok(enqueue_vector)
}

/// 0.8.20 Slice 5a (R-20-E1, work item 1) — the EDGE half of the total
/// projector, extracted verbatim from the inlined `commit_batch` edge arm.
///
/// Before this extraction there was NO edge projector function: `commit_batch`
/// inlined the edge FTS insert + the edge vector enqueue, and
/// `rebuild_shadow_state` re-implemented a SUBSET of it (edge FTS only, and only
/// for body-carrying edges), so a projector-replay rebuild silently dropped the
/// rest — notably the `up_to_date` readiness terminal that the write path
/// records for a body-less structural edge. With both sites now calling this one
/// function, the write path and the rebuild path produce identical edge
/// projections by construction.
///
/// Mirrors [`project_canonical_node_row`]'s split (ADR-0.8.14 §D5(b)): FTS in
/// THIS transaction; vector work only ENQUEUED, embedded later by the worker
/// pool. Edge bodies enqueue under the fixed kind `"edge_fact"` so
/// `resolve_source_type` maps them to `source_type = "edge_fact"` in
/// `vector_default` (partition correctness); that kind is auto-registered in
/// `_fathomdb_vector_kinds` (idempotent).
///
/// Returns `true` iff async vector work was enqueued.
pub(crate) fn project_canonical_edge_row(
    tx: &Connection,
    cursor: u64,
    kind: &str,
    body: Option<&str>,
    pass: ProjectionPass,
) -> rusqlite::Result<bool> {
    // G11 — edge FTS projection into `search_index_edges` (separate table from
    // node-body `search_index` — Option B partition). Body-less structural
    // edges carry no lexical content and project no FTS row.
    if pass.writes_fts() {
        if let Some(edge_body) = body {
            tx.execute(
                "INSERT INTO search_index_edges(body, kind, write_cursor)
                 VALUES(?1, ?2, ?3)",
                params![edge_body, kind, cursor],
            )?;
        }
    }
    let enqueue_vector = body.is_some();
    if pass.writes_vector_state() {
        if enqueue_vector {
            let now_unix =
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
            tx.execute(
                "INSERT OR IGNORE INTO _fathomdb_vector_kinds(kind, profile, created_at)
                 VALUES('edge_fact', 'default', ?1)",
                params![now_unix],
            )?;
            tx.execute(
                "INSERT INTO _fathomdb_projection_state(
                     kind, last_enqueued_cursor, updated_at
                 ) VALUES('edge_fact', ?1, 0)
                 ON CONFLICT(kind) DO UPDATE
                     SET last_enqueued_cursor = excluded.last_enqueued_cursor",
                params![cursor],
            )?;
            // Do NOT call record_projection_terminal — let the scheduler embed
            // the body and mark it terminal after projection.
        } else {
            record_projection_terminal(tx, cursor, "up_to_date")?;
        }
    }
    Ok(enqueue_vector)
}
