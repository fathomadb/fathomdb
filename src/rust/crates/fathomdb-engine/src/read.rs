use super::*;

/// Slice 30 (G2) — an active canonical node row returned by `read.get` /
/// `read.get_many`.
///
/// `logical_id` is the queried stable identity (echoed). `write_cursor` is the
/// interim id carrier (same column `SearchHit.id` carries). Only ACTIVE rows
/// (`superseded_at IS NULL`) are ever materialised into this shape; a missing or
/// superseded `logical_id` is a normal absence (`None`), never an error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeRecord {
    pub logical_id: String,
    pub kind: String,
    pub body: String,
    pub write_cursor: u64,
}

/// Slice 30 (G3) — one `operational_mutations` row returned by `read.collection`
/// / `read.mutations`. `id` is the autoincrement PK (the after-id cursor key).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpStoreRow {
    pub id: i64,
    pub collection: String,
    pub record_key: String,
    pub op_kind: String,
    pub payload: String,
    pub schema_id: Option<String>,
    pub write_cursor: u64,
}

/// Current value from a governed `latest_state` operational collection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationalStateRecordV1 {
    /// Wire schema version. Always 1 in this release.
    pub schema_version: u32,
    /// Registered operational collection name.
    pub collection: String,
    /// Caller-owned record key within the collection.
    pub record_key: String,
    /// Exact stored JSON payload text.
    pub payload: String,
    /// Optional registered schema identity carried by the mutation.
    pub schema_id: Option<String>,
    /// Engine-minted total-order coordinate for this current value.
    pub write_cursor: u64,
}

/// Slice 30 (G3) — the ~1M cap on a single op-store read-back page. The public
/// `read.collection` / `read.mutations` LIMIT is `min(caller_limit, this)`, so
/// no API path can issue an unbounded SELECT. Cursor/limit hardening under a
/// genuine ~1M-row append-only log is reserved-gap Slice 32.
const READ_COLLECTION_MAX_LIMIT: usize = 1_000_000;

/// Slice 30 (G2) — active-only point lookup by `logical_id` on the DEFERRED
/// reader tx (mirrors `read_search_in_tx`'s snapshot-stable BEGIN DEFERRED). One
/// returned slot per requested id, in REQUEST ORDER; `None` where no ACTIVE row
/// (`superseded_at IS NULL`) carries that id. Mirrors the canonical
/// projection columns + `logical_id`; superseded versions are never returned.
pub(crate) fn read_get_by_id_in_tx(
    reader: &mut Connection,
    logical_ids: &[String],
    view: &ReadView,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> rusqlite::Result<Vec<Option<NodeRecord>>> {
    if logical_ids.is_empty() {
        return Ok(Vec::new());
    }
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)?;
    // De-duplicate the requested ids for the IN(...) probe, then re-expand into
    // request order (a repeated id echoes the same active row).
    let mut found: HashMap<String, NodeRecord> = HashMap::new();
    {
        let unique: Vec<&String> = {
            let mut seen = std::collections::HashSet::new();
            logical_ids.iter().filter(|id| seen.insert((*id).clone())).collect()
        };
        let placeholders = std::iter::repeat_n("?", unique.len()).collect::<Vec<_>>().join(", ");
        // The `?` placeholders above auto-number 1..=unique.len(), so the
        // validity instant takes the next positional slot.
        let now_idx = unique.len() + 1;
        let node_sql = view.node_sql("canonical_nodes", now_idx);
        // R-20-RV: with `include_superseded` a logical_id can match several
        // rows. `ORDER BY write_cursor` + last-write-wins into `found` resolves
        // the slot DETERMINISTICALLY to the most recent version, rather than
        // leaving it at the mercy of scan order.
        let sql = format!(
            "SELECT logical_id, kind, body, write_cursor
             FROM canonical_nodes
             WHERE logical_id IN ({placeholders}){node_sql}
             ORDER BY write_cursor"
        );
        let mut statement = tx.prepare(&sql)?;
        let mut binds: Vec<rusqlite::types::Value> =
            unique.iter().map(|s| rusqlite::types::Value::Text((*s).clone())).collect();
        if let Some(now) = view.now_param() {
            binds.push(rusqlite::types::Value::Integer(now));
        }
        let rows = statement.query_map(rusqlite::params_from_iter(binds.iter()), |row| {
            let logical_id: String = row.get(0)?;
            Ok(NodeRecord {
                logical_id,
                kind: row.get(1)?,
                body: row.get(2)?,
                write_cursor: row.get::<_, i64>(3)? as u64,
            })
        })?;
        for row in rows {
            let record = row?;
            found.insert(record.logical_id.clone(), record);
        }
    }
    // tx is read-only; dropping it rolls back the (empty) transaction.
    let out = logical_ids.iter().map(|id| found.get(id).cloned()).collect();
    Ok(out)
}

/// Slice 30 (G3) — paginated op-store read-back over `operational_mutations` for
/// one `collection`, `ORDER BY id`, on the DEFERRED reader tx. The effective SQL
/// LIMIT is `min(limit, READ_COLLECTION_MAX_LIMIT)`; a caller `limit == 0`
/// returns an empty `Vec` without a SELECT. The after-id cursor (`id > ?`,
/// default 0) excludes the boundary row. The `_for_test` SELECTs
/// (`lib.rs` op-store probes) are a shape oracle only — this is a new statement.
///
/// Slice 33 (G3 / F4-READ) — hardened under a genuine large multi-collection log:
/// the SELECT rides the step-13 `operational_mutations(collection_name, id)`
/// index (`SEARCH … USING INDEX …(collection_name=? AND id>?)`), so the per-page
/// cost is O(page) — the leading `collection_name` equality fixes the prefix and
/// the trailing `id` serves both the cursor range and `ORDER BY id` with no temp
/// B-tree. The cursor is normalized with `.max(0)` so a negative `after_id` is
/// explicitly clamped to the start of the log (ids are ≥ 1) and is never confused
/// with a row id; `after_id` past the end and unknown collections yield empty
/// pages.
pub(crate) fn read_collection_in_tx(
    reader: &mut Connection,
    collection: &str,
    after_id: Option<i64>,
    limit: usize,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> rusqlite::Result<Vec<OpStoreRow>> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let clamped = limit.min(READ_COLLECTION_MAX_LIMIT) as i64;
    // Normalize the cursor: a negative after_id is clamped to the start of the
    // log. `operational_mutations.id` is autoincrement (≥ 1), so `id > 0` is the
    // full log; clamping removes the "is a negative cursor a sentinel or a row
    // id?" ambiguity without changing happy-path semantics.
    let after = after_id.unwrap_or(0).max(0);
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)?;
    let mut statement = tx.prepare(
        "SELECT id, collection_name, record_key, op_kind, payload_json, schema_id, write_cursor
         FROM operational_mutations
         WHERE collection_name = ?1 AND id > ?2
         ORDER BY id
         LIMIT ?3",
    )?;
    let rows = statement.query_map(params![collection, after, clamped], |row| {
        Ok(OpStoreRow {
            id: row.get(0)?,
            collection: row.get(1)?,
            record_key: row.get(2)?,
            op_kind: row.get(3)?,
            payload: row.get(4)?,
            schema_id: row.get(5)?,
            write_cursor: row.get::<_, i64>(6)? as u64,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Slice 35 (G4) — execute `read.list` inside a DEFERRED reader transaction.
///
/// Builds parameterized SQL: `kind = ?1 AND superseded_at IS NULL [AND
/// json_extract(body, '$.field') <op> ?N ...]` — injection-safe because:
///   (a) `kind` is `?1` (bound parameter);
///   (b) each predicate value is a bound `?N` parameter;
///   (c) the json_extract path is the ALLOWLIST ENTRY (a server-side constant
///       validated at `Predicate` construction time), never the raw caller string;
///   (d) `ComparisonOp` compiles to a server-side literal operator string from a
///       closed enum, not a caller-supplied string.
pub(crate) fn read_list_in_tx(
    reader: &mut Connection,
    kind: &str,
    predicates: &[Predicate],
    limit: usize,
    view: &ReadView,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> rusqlite::Result<Vec<NodeRecord>> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    // Build the SQL WHERE clauses for each predicate.
    // Parameters: ?1 = kind; ?2..?N = predicate values; limit is inlined.
    // `logical_id IS NOT NULL` is a SQL-level predicate so that LIMIT counts
    // only rows that can be represented as NodeRecord (which requires a non-null
    // String logical_id). Anonymous nodes (PreparedWrite::Node { logical_id: None })
    // cannot be included in NodeRecord results and are excluded before LIMIT.
    // When predicates are present we add `json_valid(body)` so rows with
    // non-JSON bodies are skipped rather than causing a `malformed JSON` error.
    let json_valid_guard = if predicates.is_empty() { "" } else { " AND json_valid(body)" };
    // R-20-RV/R-20-NV: the view's predicates replace the previously hard-coded
    // existence pair. The validity instant takes the positional slot AFTER the
    // predicate binds (?1 = kind, ?2..=?(1+n) = predicate values), so it is
    // `?{predicates.len() + 2}`. Positional `?N` is order-independent in SQLite,
    // so emitting it here — textually before the predicate clauses appended
    // below — is safe and unambiguous.
    let now_idx = predicates.len() + 2;
    let node_sql = view.node_sql("canonical_nodes", now_idx);
    let mut sql = format!(
        "SELECT logical_id, kind, body, write_cursor \
         FROM canonical_nodes \
         WHERE kind = ?1{node_sql} \
         AND logical_id IS NOT NULL{json_valid_guard}"
    );

    // Predicate params start at ?2.
    for (i, pred) in predicates.iter().enumerate() {
        let param_idx = i + 2; // ?1 is kind
        sql.push_str(" AND ");
        sql.push_str(&pred.to_sql_clause(param_idx));
    }
    sql.push_str(&format!(" LIMIT {limit}"));

    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)?;
    let mut statement = tx.prepare(&sql)?;

    // Bind all parameters: [kind, predicate_values...]
    let mut params: Vec<rusqlite::types::Value> = Vec::with_capacity(2 + predicates.len());
    params.push(rusqlite::types::Value::Text(kind.to_string()));
    for pred in predicates {
        params.push(pred.bind_value());
    }
    // Lands at index `now_idx` (= predicates.len() + 2), matching `?{now_idx}`
    // emitted by `ReadView::validity_sql`. Omitted entirely when the view
    // relaxes validity, in which case no `?{now_idx}` was emitted either.
    if let Some(now) = view.now_param() {
        params.push(rusqlite::types::Value::Integer(now));
    }

    let rows = statement.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        Ok(NodeRecord {
            logical_id: row.get(0)?,
            kind: row.get(1)?,
            body: row.get(2)?,
            write_cursor: row.get::<_, i64>(3)? as u64,
        })
    })?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub(crate) fn read_canonical_page_in_tx(
    reader: &mut Connection,
    kind: &str,
    frozen: &FrozenReadContextV1,
    page: &PageRequestV1,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> Result<PageV1<NodeRecord>, PageReaderError> {
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)?;
    let cursor = pagination::authenticate_cursor(&tx, page)?;
    let binding = frozen_read::authenticate(&tx, frozen)?;
    let after = pagination::resume_after(
        cursor.as_ref(),
        pagination::PageOperation::CanonicalNode,
        kind,
        frozen,
        page,
    )?;
    frozen_read::validate_snapshot(&tx, &binding)?;
    frozen_after_validation_hook::fire();
    validate_filter_attributes_on_snapshot(&tx, &frozen.context.eligibility)
        .map_err(page_search_error)?;

    let (mut items, has_more) = query_canonical_page_rows(
        &tx,
        kind,
        &frozen.context.view,
        &frozen.context.eligibility,
        after,
        page.limit,
    )?;
    if has_more {
        items.pop();
    }
    let next_cursor = if has_more {
        let last = items.last().ok_or(EngineError::Storage)?.write_cursor;
        Some(pagination::continuation(
            &tx,
            pagination::PageOperation::CanonicalNode,
            kind,
            frozen,
            page.limit,
            last,
        )?)
    } else {
        None
    };
    tx.commit()?;
    Ok(PageV1 { schema_version: 1, items, next_cursor })
}

#[cfg(feature = "test-hooks")]
pub(crate) fn read_canonical_page_baseline_in_tx(
    reader: &mut Connection,
    kind: &str,
    context: &FrozenReadContextV1,
    limit: usize,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> Result<Vec<NodeRecord>, PageReaderError> {
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)?;
    validate_filter_attributes_on_snapshot(&tx, &context.context.eligibility)
        .map_err(page_search_error)?;
    let (mut items, has_more) = query_canonical_page_rows(
        &tx,
        kind,
        &context.context.view,
        &context.context.eligibility,
        0,
        limit,
    )?;
    if has_more {
        items.pop();
    }
    tx.commit()?;
    Ok(items)
}

fn query_canonical_page_rows(
    connection: &Connection,
    kind: &str,
    view: &ReadView,
    filter: &SearchFilter,
    after: u64,
    limit: usize,
) -> rusqlite::Result<(Vec<NodeRecord>, bool)> {
    let (sql, binds) = canonical_page_query(kind, view, filter, after, limit)?;
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(rusqlite::params_from_iter(binds.iter()), |row| {
        Ok(NodeRecord {
            logical_id: row.get(0)?,
            kind: row.get(1)?,
            body: row.get(2)?,
            write_cursor: row.get::<_, i64>(3)? as u64,
        })
    })?;
    let items = rows.collect::<Result<Vec<_>, _>>()?;
    let has_more = items.len() > limit;
    Ok((items, has_more))
}

pub(crate) fn canonical_page_query(
    kind: &str,
    view: &ReadView,
    filter: &SearchFilter,
    after: u64,
    limit: usize,
) -> rusqlite::Result<(String, Vec<rusqlite::types::Value>)> {
    let mut binds = vec![
        rusqlite::types::Value::Text(kind.to_string()),
        rusqlite::types::Value::Integer(
            i64::try_from(after)
                .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, i64::MAX))?,
        ),
    ];
    let now_idx = binds.len() + 1;
    let visibility = view.node_sql("n", now_idx);
    if let Some(now) = view.now_param() {
        binds.push(rusqlite::types::Value::Integer(now));
    }
    let eligibility = append_node_eligibility_sql(Some(filter), "n", &mut binds);
    let limit_idx = binds.len() + 1;
    let query_limit = limit.checked_add(1).ok_or(rusqlite::Error::InvalidQuery)?;
    binds.push(rusqlite::types::Value::Integer(
        i64::try_from(query_limit)
            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, i64::MAX))?,
    ));
    let sql = format!(
        "SELECT n.logical_id,n.kind,n.body,n.write_cursor \
         FROM canonical_nodes n \
         WHERE n.kind=?1 AND n.write_cursor>?2 AND n.logical_id IS NOT NULL \
         {visibility}{eligibility} \
         ORDER BY n.write_cursor ASC LIMIT ?{limit_idx}"
    );
    Ok((sql, binds))
}

pub(crate) const OPERATIONAL_STATE_POINT_SQL: &str =
    "SELECT collection_name,record_key,payload_json,schema_id,write_cursor \
     FROM operational_state WHERE collection_name=?1 AND record_key=?2";

pub(crate) const OPERATIONAL_STATE_PAGE_SQL: &str =
    "SELECT collection_name,record_key,payload_json,schema_id,write_cursor \
     FROM operational_state \
     WHERE collection_name=?1 AND write_cursor>?2 \
     ORDER BY write_cursor ASC LIMIT ?3";

pub(crate) fn read_operational_state_in_tx(
    reader: &mut Connection,
    collection: &str,
    record_key: &str,
    frozen: Option<&FrozenReadContextV1>,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> Result<Option<OperationalStateRecordV1>, PageReaderError> {
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)?;
    if let Some(frozen) = frozen {
        let binding = frozen_read::authenticate(&tx, frozen)?;
        frozen_read::validate_snapshot(&tx, &binding)?;
        frozen_after_validation_hook::fire();
        validate_operational_context(frozen)?;
    }
    validate_operational_collection(&tx, collection)?;
    let record = tx
        .query_row(OPERATIONAL_STATE_POINT_SQL, params![collection, record_key], |row| {
            Ok(OperationalStateRecordV1 {
                schema_version: 1,
                collection: row.get(0)?,
                record_key: row.get(1)?,
                payload: row.get(2)?,
                schema_id: row.get(3)?,
                write_cursor: row.get::<_, i64>(4)? as u64,
            })
        })
        .optional()?;
    tx.commit()?;
    Ok(record)
}

pub(crate) fn read_operational_state_page_in_tx(
    reader: &mut Connection,
    collection: &str,
    frozen: &FrozenReadContextV1,
    page: &PageRequestV1,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> Result<PageV1<OperationalStateRecordV1>, PageReaderError> {
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)?;
    let cursor = pagination::authenticate_cursor(&tx, page)?;
    let binding = frozen_read::authenticate(&tx, frozen)?;
    let after = pagination::resume_after(
        cursor.as_ref(),
        pagination::PageOperation::OperationalState,
        collection,
        frozen,
        page,
    )?;
    frozen_read::validate_snapshot(&tx, &binding)?;
    frozen_after_validation_hook::fire();
    validate_operational_context(frozen)?;
    validate_operational_collection(&tx, collection)?;
    let limit = i64::try_from(page.limit + 1).map_err(|_| EngineError::Storage)?;
    let after = i64::try_from(after).map_err(|_| EngineError::Storage)?;
    let mut statement = tx.prepare(OPERATIONAL_STATE_PAGE_SQL)?;
    let rows = statement.query_map(params![collection, after, limit], |row| {
        Ok(OperationalStateRecordV1 {
            schema_version: 1,
            collection: row.get(0)?,
            record_key: row.get(1)?,
            payload: row.get(2)?,
            schema_id: row.get(3)?,
            write_cursor: row.get::<_, i64>(4)? as u64,
        })
    })?;
    let mut items = rows.collect::<Result<Vec<_>, _>>()?;
    let has_more = items.len() > page.limit;
    if has_more {
        items.pop();
    }
    let next_cursor = if has_more {
        let last = items.last().ok_or(EngineError::Storage)?.write_cursor;
        Some(pagination::continuation(
            &tx,
            pagination::PageOperation::OperationalState,
            collection,
            frozen,
            page.limit,
            last,
        )?)
    } else {
        None
    };
    drop(statement);
    tx.commit()?;
    Ok(PageV1 { schema_version: 1, items, next_cursor })
}

fn validate_operational_context(frozen: &FrozenReadContextV1) -> Result<(), EngineError> {
    let view = frozen.context.view;
    if view.include_superseded
        || view.include_inactive
        || view.include_out_of_window
        || !frozen.context.eligibility.is_unfiltered()
    {
        return Err(
            pagination::PageError::new(PageErrorReason::ContextNotApplicable, "/context").into()
        );
    }
    Ok(())
}

fn validate_operational_collection(
    connection: &Connection,
    collection: &str,
) -> Result<(), EngineError> {
    let registration = connection
        .query_row(
            "SELECT kind,format_version FROM operational_collections WHERE name=?1",
            [collection],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((kind, format_version)) = registration else {
        return Err(
            pagination::PageError::new(PageErrorReason::CollectionNotFound, "/collection").into()
        );
    };
    if kind != "latest_state" {
        return Err(pagination::PageError::new(
            PageErrorReason::CollectionKindMismatch,
            "/collection",
        )
        .into());
    }
    if format_version != 1 {
        return Err(pagination::PageError::new(
            PageErrorReason::CollectionFormatUnsupported,
            "/collection",
        )
        .into());
    }
    Ok(())
}

fn page_search_error(error: SearchReaderError) -> PageReaderError {
    match error {
        SearchReaderError::Sqlite(error) => PageReaderError::Sqlite(error),
        SearchReaderError::InvalidFilter(reason) => {
            PageReaderError::Engine(EngineError::InvalidFilter { reason })
        }
        _ => PageReaderError::Engine(EngineError::Storage),
    }
}

impl Engine {
    /// Slice 30 (G2) — `read.get`: active-only point lookup by `logical_id`.
    /// Delegates to [`Engine::read_get_many`]; returns the single slot. A
    /// missing/superseded id is `None` (a normal absence, not an error). Reads
    /// ride the ReaderWorkerPool DEFERRED-tx path (never the writer lock).
    pub fn read_get(
        &self,
        logical_id: &str,
        view: &ReadView,
    ) -> Result<Option<NodeRecord>, EngineError> {
        let ids = [logical_id.to_string()];
        let rows = self.read_get_many(&ids, view)?;
        Ok(rows.into_iter().next().flatten())
    }

    /// Slice 30 (G2) — `read.get_many`: active-only point lookup over many
    /// `logical_id`s. Returns one slot per requested id in REQUEST ORDER, `None`
    /// where no active row carries that id (partial, never all-or-nothing).
    pub fn read_get_many(
        &self,
        logical_ids: &[String],
        view: &ReadView,
    ) -> Result<Vec<Option<NodeRecord>>, EngineError> {
        self.ensure_open()?;
        if logical_ids.is_empty() {
            return Ok(Vec::new());
        }
        let (response_tx, response_rx) = mpsc::sync_channel(1);
        let request = ReaderRequest::GetById {
            logical_ids: logical_ids.to_vec(),
            view: *view,
            respond: response_tx,
        };
        if self.reader_pool.dispatch(request).is_err() {
            return Err(EngineError::Closing);
        }
        match response_rx.recv().map_err(|_| EngineError::Storage)? {
            Ok(rows) => Ok(rows),
            Err(err) => {
                self.emit_sqlite_internal_error(&err);
                Err(EngineError::Storage)
            }
        }
    }

    /// Slice 30 (G3) — `read.collection`: paginated op-store read-back over
    /// `operational_mutations` for `collection`, `ORDER BY id`. `limit` is
    /// MANDATORY (clamped to the ~1M cap); `after_id` is the exclusive cursor.
    /// Reads ride the ReaderWorkerPool DEFERRED-tx path.
    pub fn read_collection(
        &self,
        collection: &str,
        after_id: Option<i64>,
        limit: usize,
    ) -> Result<Vec<OpStoreRow>, EngineError> {
        self.read_collection_dispatch(collection, after_id, limit)
    }

    /// Slice 30 (G3) — `read.mutations`: the mutation-log-oriented alias surface
    /// over the SAME op-store read-back as [`Engine::read_collection`].
    pub fn read_mutations(
        &self,
        collection: &str,
        after_id: Option<i64>,
        limit: usize,
    ) -> Result<Vec<OpStoreRow>, EngineError> {
        self.read_collection_dispatch(collection, after_id, limit)
    }

    fn read_collection_dispatch(
        &self,
        collection: &str,
        after_id: Option<i64>,
        limit: usize,
    ) -> Result<Vec<OpStoreRow>, EngineError> {
        self.ensure_open()?;
        let (response_tx, response_rx) = mpsc::sync_channel(1);
        let request = ReaderRequest::ReadCollection {
            collection: collection.to_string(),
            after_id,
            limit,
            respond: response_tx,
        };
        if self.reader_pool.dispatch(request).is_err() {
            return Err(EngineError::Closing);
        }
        match response_rx.recv().map_err(|_| EngineError::Storage)? {
            Ok(rows) => Ok(rows),
            Err(err) => {
                self.emit_sqlite_internal_error(&err);
                Err(EngineError::Storage)
            }
        }
    }

    /// Slice 35 (G4) — `read.list`: list active `canonical_nodes` of a given
    /// `kind`, optionally filtered by a closed [`Predicate`] set, up to `limit`
    /// rows. Returns `Vec<NodeRecord>` (active only; `superseded_at IS NULL`).
    ///
    /// Multiple predicates are combined as AND (D-F5). An empty predicate slice
    /// returns all active nodes of the given kind up to `limit` (unfiltered path).
    /// Compilation target: `json_extract(body, '$.field') <op> ?` with bound
    /// parameters (injection-safe per D-F4). See `dev/adr/ADR-0.8.0-filter-grammar.md`.
    ///
    /// Path validation happens at [`Predicate`] construction time; `read_list`
    /// revalidates as defense-in-depth (enum variants are `pub`, so direct
    /// struct-literal construction could bypass the constructors).
    pub fn read_list(
        &self,
        kind: &str,
        predicates: &[Predicate],
        limit: usize,
        view: &ReadView,
    ) -> Result<Vec<NodeRecord>, EngineError> {
        self.ensure_open()?;
        // Defense-in-depth: revalidate paths even if the caller bypassed the
        // validated constructors by constructing enum variants directly.
        for pred in predicates {
            let path = pred.path();
            if !PREDICATE_PATH_ALLOWLIST.contains(&path) {
                return Err(EngineError::InvalidFilter {
                    reason: format!("path '{path}' is not in the predicate path allowlist"),
                });
            }
        }
        let (response_tx, response_rx) = mpsc::sync_channel(1);
        let request = ReaderRequest::ReadList {
            kind: kind.to_string(),
            predicates: predicates.to_vec(),
            limit,
            view: *view,
            respond: response_tx,
        };
        if self.reader_pool.dispatch(request).is_err() {
            return Err(EngineError::Closing);
        }
        match response_rx.recv().map_err(|_| EngineError::Storage)? {
            Ok(rows) => Ok(rows),
            Err(err) => {
                self.emit_sqlite_internal_error(&err);
                Err(EngineError::Storage)
            }
        }
    }

    /// 0.8.11 Slice 40 (#17) — unified-`Filter` entry point for the
    /// canonical_nodes `read.list` backend. Accepts the **full** [`FilterTerm`]
    /// set (D3): `Json` runs the shipped allowlisted `json_extract` path;
    /// `Status`/`CreatedAfter` lower to allowlisted json-paths; `Kind`/`SourceType`
    /// **constant-fold** against the partition `kind` (a guaranteed-empty fold
    /// returns an empty `Vec` without touching SQL). Dispatches to the same
    /// [`Engine::read_list`] machinery the shipped `Predicate` surface uses, so
    /// every inherited invariant (`superseded_at IS NULL`, `json_valid(body)`,
    /// the `canonical_nodes(kind)` index, parameterized binds) is preserved.
    pub fn read_list_filter(
        &self,
        kind: &str,
        filter: &Filter,
        limit: usize,
        view: &ReadView,
    ) -> Result<Vec<NodeRecord>, EngineError> {
        self.ensure_open()?;
        match filter.lower_for_read_list(kind)? {
            None => Ok(Vec::new()),
            Some(preds) => self.read_list(kind, &preds, limit, view),
        }
    }

    /// Read one stable keyset page of canonical logical nodes.
    ///
    /// The authenticated frozen context supplies validity and eligibility.
    /// Continuations are database-, selector-, context-, and limit-bound and
    /// refuse after relevant state drift.
    pub fn read_canonical_page(
        &self,
        kind: &str,
        context: &FrozenReadContextV1,
        page: &PageRequestV1,
    ) -> Result<PageV1<NodeRecord>, EngineError> {
        self.ensure_open()?;
        pagination::validate_request(page)?;
        let (respond, receive) = mpsc::sync_channel(1);
        self.reader_pool
            .dispatch(ReaderRequest::ReadCanonicalPage(Box::new(CanonicalPageReaderRequest {
                kind: kind.to_string(),
                frozen: context.clone(),
                page: page.clone(),
                respond,
            })))
            .map_err(|_| EngineError::Closing)?;
        self.receive_page_result(receive)
    }

    /// Read the current value of one governed `latest_state` record.
    ///
    /// Supplying a frozen context binds this point read to the same authority
    /// used by a page walk. A context is optional for an ordinary current read.
    pub fn read_operational_state(
        &self,
        collection: &str,
        record_key: &str,
        context: Option<&FrozenReadContextV1>,
    ) -> Result<Option<OperationalStateRecordV1>, EngineError> {
        self.ensure_open()?;
        let (respond, receive) = mpsc::sync_channel(1);
        self.reader_pool
            .dispatch(ReaderRequest::ReadOperationalState(Box::new(
                OperationalStateReaderRequest {
                    collection: collection.to_string(),
                    record_key: record_key.to_string(),
                    frozen: context.cloned(),
                    respond,
                },
            )))
            .map_err(|_| EngineError::Closing)?;
        self.receive_page_result(receive)
    }

    /// Read one stable keyset page of a governed `latest_state` collection.
    pub fn read_operational_state_page(
        &self,
        collection: &str,
        context: &FrozenReadContextV1,
        page: &PageRequestV1,
    ) -> Result<PageV1<OperationalStateRecordV1>, EngineError> {
        self.ensure_open()?;
        pagination::validate_request(page)?;
        let (respond, receive) = mpsc::sync_channel(1);
        self.reader_pool
            .dispatch(ReaderRequest::ReadOperationalStatePage(Box::new(
                OperationalStatePageReaderRequest {
                    collection: collection.to_string(),
                    frozen: context.clone(),
                    page: page.clone(),
                    respond,
                },
            )))
            .map_err(|_| EngineError::Closing)?;
        self.receive_page_result(receive)
    }

    pub(crate) fn receive_page_result<T>(
        &self,
        receive: Receiver<Result<T, PageReaderError>>,
    ) -> Result<T, EngineError> {
        match receive.recv().map_err(|_| EngineError::Storage)? {
            Ok(value) => Ok(value),
            Err(PageReaderError::Engine(error)) => Err(error),
            Err(PageReaderError::Sqlite(error)) => {
                self.emit_sqlite_internal_error(&error);
                Err(EngineError::Storage)
            }
        }
    }
}
