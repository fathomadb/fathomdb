use crate::errors::EngineError;
use crate::filter::{
    append_node_eligibility_sql, validate_filter_attributes_on_snapshot, Predicate, SearchFilter,
    SnapshotFilterError,
};
use crate::frozen_read::{self, FrozenReadContextV1, FrozenReadError};
use crate::pagination::{self, PageErrorReason, PageRequestV1, PageV1};
use crate::properties::PropertyMap;
use crate::reader_transaction::begin_attributed_reader_tx;
use crate::scope::ScopePredicate;
use crate::temporal::ReadView;
#[cfg(any(test, feature = "test-hooks"))]
use crate::test_hooks::frozen_after_validation_hook;
use crate::wal_attribution::WalAttributionCollector;
use rusqlite::{Connection, OptionalExtension};
use std::collections::HashMap;
use std::sync::Arc;

/// Slice 30 (G2) — an active canonical node row returned by `read.get` /
/// `read.get_many`.
///
/// `logical_id` is the queried stable identity (echoed). `write_cursor` is the
/// interim id carrier (same column `SearchHit.id` carries). Only ACTIVE rows
/// (`superseded_at IS NULL`) are ever materialised into this shape; a missing or
/// superseded `logical_id` is a normal absence (`None`), never an error.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeRecord {
    pub logical_id: String,
    pub kind: String,
    pub body: String,
    pub write_cursor: u64,
    /// Optional typed scalar property map stored outside `body`.
    pub properties: Option<PropertyMap>,
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

/// Content-free visibility snapshot of the database (`EP-01`), read in one
/// deferred reader transaction without acquiring the writer lock.
///
/// Note that append-only `operational_mutations` does not carry a
/// read-visibility trigger, so `canonical_write_boundary` can advance without
/// advancing `generation` on append-only mutation writes.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct VisibilityStateV1 {
    /// Wire schema version. Always `1`.
    pub schema_version: u32,
    /// Database identity (32 lowercase hex characters from `_fathomdb_open_state`).
    pub database_id: String,
    /// Monotonic read-visibility generation from `_fathomdb_read_visibility_state`.
    pub generation: u64,
    /// Branch-sensitive 64 lowercase hex nonce paired with `generation`.
    pub nonce: String,
    /// Canonical write boundary (`load_next_cursor`) observed on this snapshot.
    pub canonical_write_boundary: u64,
    /// Persisted projection cursor (`_fathomdb_open_state`) observed on this snapshot.
    pub projection_cursor: u64,
}

fn is_lower_hex_len(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// `EP-01` — read `VisibilityStateV1` in one deferred reader transaction
/// without acquiring the writer lock.
pub(crate) fn read_visibility_state_in_tx(
    reader: &mut Connection,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> Result<VisibilityStateV1, EngineError> {
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)
        .map_err(|_| EngineError::Storage)?;
    let database_id = frozen_read::read_open_state(&tx, "_fathomdb_database_id")?;
    let (generation_i64, nonce): (i64, String) = tx
        .query_row(
            "SELECT generation, state_nonce FROM _fathomdb_read_visibility_state WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| EngineError::Storage)?;
    let generation = u64::try_from(generation_i64).map_err(|_| EngineError::Storage)?;
    if !is_lower_hex_len(&database_id, 32) || !is_lower_hex_len(&nonce, 64) {
        return Err(EngineError::Storage);
    }
    let canonical_write_boundary = crate::write_commit::load_next_cursor(&tx);
    let projection_cursor =
        crate::projection_commit::load_projection_cursor(&tx).map_err(|_| EngineError::Storage)?;
    Ok(VisibilityStateV1 {
        schema_version: 1,
        database_id,
        generation,
        nonce,
        canonical_write_boundary,
        projection_cursor,
    })
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
    scope: &ScopePredicate,
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
        let mut binds: Vec<rusqlite::types::Value> = Vec::with_capacity(unique.len() + 4);
        let mut scope_sql = String::new();
        scope.append_sql_values("scope", &mut scope_sql, &mut binds);
        let lid_start = binds.len() + 1;
        let placeholders = (lid_start..lid_start + unique.len())
            .map(|idx| format!("?{idx}"))
            .collect::<Vec<_>>()
            .join(", ");
        for s in &unique {
            binds.push(rusqlite::types::Value::Text((*s).clone()));
        }
        let now_idx = binds.len() + 1;
        let node_sql = view.node_sql("canonical_nodes", now_idx);
        if let Some(now) = view.now_param() {
            binds.push(rusqlite::types::Value::Integer(now));
        }
        // R-20-RV: with `include_superseded` a logical_id can match several
        // rows. `ORDER BY write_cursor` + last-write-wins into `found` resolves
        // the slot DETERMINISTICALLY to the most recent version, rather than
        // leaving it at the mercy of scan order.
        let sql = format!(
            "SELECT logical_id, kind, body, write_cursor, properties
             FROM canonical_nodes
             WHERE {scope_sql} AND logical_id IN ({placeholders}){node_sql}
             ORDER BY write_cursor"
        );
        let mut statement = tx.prepare(&sql)?;
        let rows = statement.query_map(rusqlite::params_from_iter(binds.iter()), |row| {
            let logical_id: String = row.get(0)?;
            Ok(NodeRecord {
                logical_id,
                kind: row.get(1)?,
                body: row.get(2)?,
                write_cursor: row.get::<_, i64>(3)? as u64,
                properties: crate::properties::decode_properties_row_col(row, 4)?,
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
    scope: &ScopePredicate,
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
    let mut binds: Vec<rusqlite::types::Value> = Vec::with_capacity(4);
    let mut scope_sql = String::new();
    scope.append_sql_values("scope", &mut scope_sql, &mut binds);
    let coll_idx = binds.len() + 1;
    binds.push(rusqlite::types::Value::Text(collection.to_string()));
    let after_idx = binds.len() + 1;
    binds.push(rusqlite::types::Value::Integer(after));
    let limit_idx = binds.len() + 1;
    binds.push(rusqlite::types::Value::Integer(clamped));
    let sql = format!(
        "SELECT id, collection_name, record_key, op_kind, payload_json, schema_id, write_cursor
         FROM operational_mutations
         WHERE {scope_sql} AND collection_name = ?{coll_idx} AND id > ?{after_idx}
         ORDER BY id
         LIMIT ?{limit_idx}"
    );
    let mut statement = tx.prepare(&sql)?;
    let rows = statement.query_map(rusqlite::params_from_iter(binds.iter()), |row| {
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

/// 0.8.31 Slice 20 — list all registered operational collections ordered by
/// `name ASC` inside a DEFERRED reader transaction.
pub(crate) fn read_operational_collections_in_tx(
    reader: &mut Connection,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> rusqlite::Result<Vec<crate::properties::OperationalCollectionInfo>> {
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)?;
    let mut statement = tx.prepare_cached(
        "SELECT name, kind, schema_json, retention_json, format_version, created_at
         FROM operational_collections
         ORDER BY name ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(crate::properties::OperationalCollectionInfo {
            name: row.get(0)?,
            kind: row.get(1)?,
            schema_json: row.get(2)?,
            retention_json: row.get(3)?,
            format_version: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// 0.8.31 Slice 20 — point lookup of one registered operational collection by
/// `name` inside a DEFERRED reader transaction.
pub(crate) fn read_operational_collection_in_tx(
    reader: &mut Connection,
    name: &str,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> rusqlite::Result<Option<crate::properties::OperationalCollectionInfo>> {
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)?;
    let mut statement = tx.prepare_cached(
        "SELECT name, kind, schema_json, retention_json, format_version, created_at
         FROM operational_collections
         WHERE name = ?1",
    )?;
    statement
        .query_row([name], |row| {
            Ok(crate::properties::OperationalCollectionInfo {
                name: row.get(0)?,
                kind: row.get(1)?,
                schema_json: row.get(2)?,
                retention_json: row.get(3)?,
                format_version: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .optional()
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
pub(crate) struct ReadListQuery<'a> {
    pub(crate) kind: &'a str,
    pub(crate) predicates: &'a [Predicate],
    pub(crate) source_types: &'a [String],
    pub(crate) property_predicates: &'a [crate::properties::PropertyPredicate],
    pub(crate) limit: usize,
    pub(crate) view: &'a ReadView,
    pub(crate) scope: &'a ScopePredicate,
}

pub(crate) fn read_list_in_tx(
    reader: &mut Connection,
    query: ReadListQuery<'_>,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> Result<Vec<NodeRecord>, PageReaderError> {
    let ReadListQuery { kind, predicates, source_types, property_predicates, limit, view, scope } =
        query;
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)?;
    if !property_predicates.is_empty() {
        crate::filter::validate_property_predicates_on_snapshot(&tx, property_predicates)
            .map_err(page_filter_error)?;
    }
    for expected_source_type in source_types {
        match crate::resolve_source_type_on(&tx, kind) {
            Ok(resolved) if resolved == expected_source_type.as_str() => {}
            _ => return Ok(Vec::new()),
        }
    }
    if limit == 0 {
        return Ok(Vec::new());
    }
    let json_valid_guard = if predicates.is_empty() { "" } else { " AND json_valid(cn.body)" };

    // Bind parameters in exact placeholder order: [scope..., kind, predicate_values..., optional now, property_predicates...].
    let mut params: Vec<rusqlite::types::Value> = Vec::with_capacity(4 + predicates.len());
    let mut scope_sql = String::new();
    scope.append_sql_values("cn.scope", &mut scope_sql, &mut params);
    let kind_idx = params.len() + 1;
    params.push(rusqlite::types::Value::Text(kind.to_string()));
    let pred_start = params.len() + 1;
    for pred in predicates {
        params.push(Predicate::bind_value(pred));
    }
    let now_idx = params.len() + 1;
    let node_sql = view.node_sql("cn", now_idx);
    if let Some(now) = view.now_param() {
        params.push(rusqlite::types::Value::Integer(now));
    }

    let mut sql = format!(
        "SELECT cn.logical_id, cn.kind, cn.body, cn.write_cursor, cn.properties \
         FROM canonical_nodes cn \
         WHERE {scope_sql} AND cn.kind = ?{kind_idx}{node_sql} \
         AND cn.logical_id IS NOT NULL{json_valid_guard}"
    );

    for (i, pred) in predicates.iter().enumerate() {
        let param_idx = pred_start + i;
        sql.push_str(" AND ");
        sql.push_str(&Predicate::to_sql_clause(pred, param_idx));
    }
    if !property_predicates.is_empty() {
        let prop_filter = SearchFilter {
            property_predicates: property_predicates.to_vec(),
            ..Default::default()
        };
        let prop_sql = crate::filter::append_node_eligibility_sql_view(
            &tx,
            Some(&prop_filter),
            "cn",
            &mut params,
            view,
        )?;
        sql.push_str(&prop_sql);
    }
    sql.push_str(&format!(" LIMIT {limit}"));

    let mut statement = tx.prepare(&sql)?;

    let rows = statement.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        Ok(NodeRecord {
            logical_id: row.get(0)?,
            kind: row.get(1)?,
            body: row.get(2)?,
            write_cursor: row.get::<_, i64>(3)? as u64,
            properties: crate::properties::decode_properties_row_col(row, 4)?,
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
    scope: &ScopePredicate,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> Result<PageV1<NodeRecord>, PageReaderError> {
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)?;
    let cursor = pagination::authenticate_cursor(&tx, page)?;
    let binding = frozen_read::authenticate(&tx, frozen, scope)?;
    let after = pagination::resume_after(
        cursor.as_ref(),
        pagination::PageOperation::CanonicalNode,
        kind,
        frozen,
        page,
    )?;
    frozen_read::validate_snapshot(&tx, &binding)?;
    #[cfg(any(test, feature = "test-hooks"))]
    frozen_after_validation_hook::fire(attribution);
    validate_filter_attributes_on_snapshot(&tx, &frozen.context.eligibility)
        .map_err(page_filter_error)?;

    let (mut items, has_more) = query_canonical_page_rows(
        &tx,
        kind,
        &frozen.context.view,
        &frozen.context.eligibility,
        after,
        page.limit,
        scope,
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
    scope: &ScopePredicate,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> Result<Vec<NodeRecord>, PageReaderError> {
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)?;
    validate_filter_attributes_on_snapshot(&tx, &context.context.eligibility)
        .map_err(page_filter_error)?;
    let (mut items, has_more) = query_canonical_page_rows(
        &tx,
        kind,
        &context.context.view,
        &context.context.eligibility,
        0,
        limit,
        scope,
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
    scope: &ScopePredicate,
) -> rusqlite::Result<(Vec<NodeRecord>, bool)> {
    let (sql, binds) =
        canonical_page_query_on(Some(connection), kind, view, filter, after, limit, scope)?;
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(rusqlite::params_from_iter(binds.iter()), |row| {
        Ok(NodeRecord {
            logical_id: row.get(0)?,
            kind: row.get(1)?,
            body: row.get(2)?,
            write_cursor: row.get::<_, i64>(3)? as u64,
            properties: crate::properties::decode_properties_row_col(row, 4)?,
        })
    })?;
    let items = rows.collect::<Result<Vec<_>, _>>()?;
    let has_more = items.len() > limit;
    Ok((items, has_more))
}

#[cfg(feature = "test-hooks")]
pub(crate) fn canonical_page_query(
    kind: &str,
    view: &ReadView,
    filter: &SearchFilter,
    after: u64,
    limit: usize,
    scope: &ScopePredicate,
) -> rusqlite::Result<(String, Vec<rusqlite::types::Value>)> {
    canonical_page_query_on(None, kind, view, filter, after, limit, scope)
}

fn canonical_page_query_on(
    connection: Option<&Connection>,
    kind: &str,
    view: &ReadView,
    filter: &SearchFilter,
    after: u64,
    limit: usize,
    scope: &ScopePredicate,
) -> rusqlite::Result<(String, Vec<rusqlite::types::Value>)> {
    let mut binds = Vec::with_capacity(6);
    let mut scope_sql = String::new();
    scope.append_sql_values("n.scope", &mut scope_sql, &mut binds);
    let kind_idx = binds.len() + 1;
    binds.push(rusqlite::types::Value::Text(kind.to_string()));
    let after_idx = binds.len() + 1;
    binds.push(rusqlite::types::Value::Integer(
        i64::try_from(after).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, i64::MAX))?,
    ));
    let now_idx = binds.len() + 1;
    let visibility = view.node_sql("n", now_idx);
    if let Some(now) = view.now_param() {
        binds.push(rusqlite::types::Value::Integer(now));
    }
    let eligibility = match connection {
        Some(conn) => crate::filter::append_node_eligibility_sql_view(
            conn,
            Some(filter),
            "n",
            &mut binds,
            view,
        )?,
        None => append_node_eligibility_sql(Some(filter), "n", &mut binds),
    };
    let limit_idx = binds.len() + 1;
    let query_limit = limit.checked_add(1).ok_or(rusqlite::Error::InvalidQuery)?;
    binds.push(rusqlite::types::Value::Integer(
        i64::try_from(query_limit)
            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, i64::MAX))?,
    ));
    let sql = format!(
        "SELECT n.logical_id,n.kind,n.body,n.write_cursor,n.properties \
         FROM canonical_nodes n \
         WHERE {scope_sql} AND n.kind=?{kind_idx} AND n.write_cursor>?{after_idx} AND n.logical_id IS NOT NULL \
         {visibility}{eligibility} \
         ORDER BY n.write_cursor ASC LIMIT ?{limit_idx}"
    );
    Ok((sql, binds))
}

pub(crate) fn operational_state_point_sql(scope: &ScopePredicate) -> String {
    let (scope_sql, _) = scope.sql_clause("scope", 3);
    let order_tail = if scope.single_read_scope().is_some() {
        ""
    } else {
        " ORDER BY write_cursor DESC LIMIT 1"
    };
    format!(
        "SELECT collection_name,record_key,payload_json,schema_id,write_cursor \
         FROM operational_state WHERE {scope_sql} AND collection_name=?1 AND record_key=?2{order_tail}"
    )
}

pub(crate) fn operational_state_page_sql(scope: &ScopePredicate) -> String {
    let (scope_sql, _) = scope.sql_clause("scope", 4);
    format!(
        "SELECT collection_name,record_key,payload_json,schema_id,write_cursor \
         FROM operational_state \
         WHERE {scope_sql} AND collection_name=?1 AND write_cursor>?2 \
         ORDER BY write_cursor ASC LIMIT ?3"
    )
}

pub(crate) fn read_operational_state_in_tx(
    reader: &mut Connection,
    collection: &str,
    record_key: &str,
    frozen: Option<&FrozenReadContextV1>,
    scope: &ScopePredicate,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> Result<Option<OperationalStateRecordV1>, PageReaderError> {
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)?;
    if let Some(frozen) = frozen {
        let binding = frozen_read::authenticate(&tx, frozen, scope)?;
        frozen_read::validate_snapshot(&tx, &binding)?;
        #[cfg(any(test, feature = "test-hooks"))]
        frozen_after_validation_hook::fire(attribution);
        validate_operational_context(frozen)?;
    }
    validate_operational_collection(&tx, collection)?;
    let mut binds = vec![
        rusqlite::types::Value::Text(collection.to_string()),
        rusqlite::types::Value::Text(record_key.to_string()),
    ];
    scope.push_values(&mut binds);
    let record = tx
        .query_row(
            &operational_state_point_sql(scope),
            rusqlite::params_from_iter(binds.iter()),
            |row| {
                Ok(OperationalStateRecordV1 {
                    schema_version: 1,
                    collection: row.get(0)?,
                    record_key: row.get(1)?,
                    payload: row.get(2)?,
                    schema_id: row.get(3)?,
                    write_cursor: row.get::<_, i64>(4)? as u64,
                })
            },
        )
        .optional()?;
    tx.commit()?;
    Ok(record)
}

pub(crate) fn read_operational_state_page_in_tx(
    reader: &mut Connection,
    collection: &str,
    frozen: &FrozenReadContextV1,
    page: &PageRequestV1,
    scope: &ScopePredicate,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> Result<PageV1<OperationalStateRecordV1>, PageReaderError> {
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)?;
    let cursor = pagination::authenticate_cursor(&tx, page)?;
    let binding = frozen_read::authenticate(&tx, frozen, scope)?;
    let after = pagination::resume_after(
        cursor.as_ref(),
        pagination::PageOperation::OperationalState,
        collection,
        frozen,
        page,
    )?;
    frozen_read::validate_snapshot(&tx, &binding)?;
    #[cfg(any(test, feature = "test-hooks"))]
    frozen_after_validation_hook::fire(attribution);
    validate_operational_context(frozen)?;
    validate_operational_collection(&tx, collection)?;
    let limit = i64::try_from(page.limit + 1).map_err(|_| EngineError::Storage)?;
    let after = i64::try_from(after).map_err(|_| EngineError::Storage)?;
    let mut binds = vec![
        rusqlite::types::Value::Text(collection.to_string()),
        rusqlite::types::Value::Integer(after),
        rusqlite::types::Value::Integer(limit),
    ];
    scope.push_values(&mut binds);
    let mut statement = tx.prepare(&operational_state_page_sql(scope))?;
    let rows = statement.query_map(rusqlite::params_from_iter(binds.iter()), |row| {
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
        || !SearchFilter::is_unfiltered(&frozen.context.eligibility)
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

fn page_filter_error(error: SnapshotFilterError) -> PageReaderError {
    match error {
        SnapshotFilterError::Sqlite(error) => PageReaderError::Sqlite(error),
        SnapshotFilterError::InvalidFilter(reason) => {
            PageReaderError::Engine(EngineError::InvalidFilter { reason })
        }
    }
}

pub(crate) enum PageReaderError {
    Sqlite(rusqlite::Error),
    Engine(EngineError),
}

impl From<rusqlite::Error> for PageReaderError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<EngineError> for PageReaderError {
    fn from(error: EngineError) -> Self {
        Self::Engine(error)
    }
}

impl From<FrozenReadError> for PageReaderError {
    fn from(error: FrozenReadError) -> Self {
        Self::Engine(EngineError::FrozenRead(error))
    }
}
