use super::*;

pub(crate) const DEFAULT_VECTOR_PROFILE: &str = "default";
pub(crate) const DEFAULT_VECTOR_PARTITION: &str = "vector_default";

pub(crate) fn load_default_profile(connection: &Connection) -> rusqlite::Result<EmbedderIdentity> {
    connection.query_row(
        "SELECT name, revision, dimension FROM _fathomdb_embedder_profiles WHERE profile = ?1",
        [DEFAULT_VECTOR_PROFILE],
        |row| {
            Ok(EmbedderIdentity::new(
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u32>(2)?,
            ))
        },
    )
}

pub(crate) fn default_profile_dimension(connection: &Connection) -> Result<u32, EngineError> {
    load_default_profile(connection)
        .map(|identity| identity.dimension)
        .map_err(|_| EngineError::Storage)
}

pub(crate) fn kind_is_vector_indexed(
    connection: &Connection,
    kind: &str,
) -> Result<bool, EngineError> {
    connection
        .prepare_cached("SELECT 1 FROM _fathomdb_vector_kinds WHERE kind = ?1")
        .map_err(|_| EngineError::Storage)?
        .query_row([kind], |_row| Ok(()))
        .map(|_| true)
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(false),
            _ => Err(EngineError::Storage),
        })
}

pub(crate) fn ensure_vector_partition(
    connection: &Connection,
    dimension: u32,
) -> rusqlite::Result<()> {
    let existing_sql: Option<String> = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name=?1",
            [DEFAULT_VECTOR_PARTITION],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    match existing_sql {
        None => create_vector_partition(connection, dimension),
        Some(sql) => {
            let attrs = actual_vector_attr_columns(connection)?;
            let normalize = |sql: &str| {
                sql.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase()
            };
            let actual = normalize(&sql);
            let actual_shape = actual.split_once("usingvec0").map(|(_, shape)| shape);
            let expected_v39 = normalize(&vector_partition_create_sql(dimension, false, &attrs));
            let expected_v39_shape = expected_v39.split_once("usingvec0").map(|(_, shape)| shape);
            if actual_shape == expected_v39_shape {
                return Ok(());
            }
            let expected_v38 =
                normalize(&vector_partition_create_sql_with_scope(dimension, false, &attrs, false));
            let expected_v38_shape = expected_v38.split_once("usingvec0").map(|(_, shape)| shape);
            let user_version: u32 =
                connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
            if user_version < 39 && actual_shape == expected_v38_shape {
                Ok(())
            } else {
                Err(rusqlite::Error::InvalidQuery)
            }
        }
    }
}

/// Format-39 layout: plain metadata is filtered inside KNN; declared node
/// properties share the sixteen-column metadata budget with eight built-in
/// metadata columns (`source_type`, `kind`, `created_at`, `status`,
/// `lifecycle`, `valid_from`, `valid_until`, `scope`). Every partition,
/// including attribute reshapes, uses 64-row chunks.
pub(crate) const MAX_VEC0_NODE_PROPERTY_SLOTS: usize = 8;
fn vector_partition_create_sql_with_scope(
    dimension: u32,
    if_not_exists: bool,
    attr_cols: &[String],
    include_scope: bool,
) -> String {
    let guard = if if_not_exists { "IF NOT EXISTS " } else { "" };
    let scope_col = if include_scope { ",scope TEXT" } else { "" };
    let mut attrs = String::new();
    for col in attr_cols {
        attrs.push_str(&format!(",{col} TEXT"));
    }
    format!(
        "CREATE VIRTUAL TABLE {guard}{DEFAULT_VECTOR_PARTITION} USING vec0(\
            embedding float[{dimension}],\
            embedding_bin bit[{dimension}],\
            source_type TEXT partition key,\
            kind TEXT,\
            created_at INTEGER,\
            status TEXT,lifecycle INTEGER,valid_from INTEGER,valid_until INTEGER{scope_col}{attrs},chunk_size=64\
         )"
    )
}

fn vector_partition_create_sql(
    dimension: u32,
    if_not_exists: bool,
    attr_cols: &[String],
) -> String {
    vector_partition_create_sql_with_scope(dimension, if_not_exists, attr_cols, true)
}

#[cfg(feature = "operator")]
pub(crate) fn vector_partition_dimension(conn: &Connection) -> rusqlite::Result<Option<u32>> {
    let sql: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name=?1",
            [DEFAULT_VECTOR_PARTITION],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(sql) = sql else {
        return Ok(None);
    };
    let lower = sql.to_ascii_lowercase();
    let Some((_, after)) = lower.split_once("embedding float[") else {
        return Err(rusqlite::Error::InvalidQuery);
    };
    let Some((digits, _)) = after.split_once(']') else {
        return Err(rusqlite::Error::InvalidQuery);
    };
    let dim = digits.trim().parse::<u32>().map_err(|_| rusqlite::Error::InvalidQuery)?;
    Ok(Some(dim))
}

fn create_vector_partition(connection: &Connection, dimension: u32) -> rusqlite::Result<()> {
    connection.execute_batch(&vector_partition_create_sql(dimension, true, &[]))
}

/// 0.8.20 Slice 15e — encode an arbitrary registry attribute NAME into a vec0-safe
/// column identifier: `attr_` + lowercase hex of the name's UTF-8 bytes.
///
/// vec0 rejects quoted column identifiers, and a Slice-15d-validated attribute
/// name may contain spaces / unicode / `-`, so the raw name cannot be a column
/// identifier. Hex is injective (so the map is reversible by
/// [`decode_attr_vec0_column`]), matches `^attr_[0-9a-f]+$`, and can never collide
/// with a built-in metadata column (`embedding`, `embedding_bin`, `source_type`,
/// `kind`, `created_at`, `status` — none carry the `attr_` prefix followed by an
/// even-length hex string of the name).
pub(crate) fn attr_vec0_column(name: &str) -> String {
    let mut s = String::from("attr_");
    for b in name.as_bytes() {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// 0.8.20 Slice 15e — inverse of [`attr_vec0_column`]. Returns the original
/// attribute name for an `attr_<hex>` column, or `None` if `col` is not a
/// well-formed encoded attribute column (so the built-in metadata columns and any
/// vec0 shadow columns are skipped when enumerating a live table's attribute set).
pub(crate) fn decode_attr_vec0_column(col: &str) -> Option<String> {
    let hex = col.strip_prefix("attr_")?;
    if hex.is_empty() || hex.len() % 2 != 0 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    let raw = hex.as_bytes();
    let mut i = 0;
    while i < raw.len() {
        let hi = (raw[i] as char).to_digit(16)?;
        let lo = (raw[i + 1] as char).to_digit(16)?;
        bytes.push((hi * 16 + lo) as u8);
        i += 2;
    }
    String::from_utf8(bytes).ok()
}

/// 0.8.20 Slice 15e + 0.8.31 Slice 20 — the DESIRED `attr_<hex>` columns implied
/// by the durable projection registry: at most `MAX_VEC0_NODE_PROPERTY_SLOTS` (`8`)
/// node `Filterable` projections (excluding edge properties and leaving 1 slot
/// reserved for future `scope`), sorted deterministically by attribute name.
pub(crate) fn desired_vector_attr_columns(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let registry = load_projection_registry(conn)?;
    let mut names: Vec<&String> = registry
        .iter()
        .filter(|((entity_class, _), stored)| {
            *entity_class == PropertyEntityClass::Node
                && stored.roles.contains(&ProjectionRole::Filterable)
        })
        .map(|((_, name), _)| name)
        .collect();
    names.sort();
    Ok(names
        .into_iter()
        .take(MAX_VEC0_NODE_PROPERTY_SLOTS)
        .map(|name| attr_vec0_column(name))
        .collect())
}

fn vector_partition_column_tokens(conn: &Connection) -> rusqlite::Result<Option<Vec<String>>> {
    let sql: Option<String> = conn
        .prepare_cached("SELECT sql FROM sqlite_master WHERE type='table' AND name=?1")
        .and_then(|mut statement| {
            statement.query_row([DEFAULT_VECTOR_PARTITION], |row| row.get::<_, String>(0))
        })
        .optional()?;
    let Some(sql) = sql else {
        return Ok(None);
    };
    let mut tokens: Vec<String> = Vec::new();
    let mut token = String::new();
    let flush = |token: &mut String, tokens: &mut Vec<String>| {
        if !token.is_empty() {
            if !tokens.contains(token) {
                tokens.push(token.clone());
            }
            token.clear();
        }
    };
    for ch in sql.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            token.push(ch);
        } else {
            flush(&mut token, &mut tokens);
        }
    }
    flush(&mut token, &mut tokens);
    Ok(Some(tokens))
}

pub(crate) fn vector_partition_has_scope_column(conn: &Connection) -> rusqlite::Result<bool> {
    let Some(tokens) = vector_partition_column_tokens(conn)? else {
        return Ok(false);
    };
    Ok(tokens.iter().any(|t| t == "scope"))
}

/// 0.8.20 Slice 15e — the `attr_<hex>` columns actually present on the live
/// `vector_default` vec0 table, parsed from its `CREATE VIRTUAL TABLE` SQL and
/// sorted. Empty when the table is absent. Parsing the SQL (rather than a PRAGMA)
/// keeps this robust across vec0 versions and shadow-table layouts.
pub(crate) fn actual_vector_attr_columns(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let Some(tokens) = vector_partition_column_tokens(conn)? else {
        return Ok(Vec::new());
    };
    let mut cols: Vec<String> =
        tokens.into_iter().filter(|token| decode_attr_vec0_column(token).is_some()).collect();
    cols.sort();
    Ok(cols)
}

/// TC-76 deletes one `vector_default` row by rowid. The bare vec0 regression
/// test proves sqlite-vec 0.1.9 removes long TEXT metadata without a workaround.
pub(crate) fn delete_vector_partition_row(
    conn: &Connection,
    rowid: i64,
) -> rusqlite::Result<usize> {
    conn.execute(&format!("DELETE FROM {DEFAULT_VECTOR_PARTITION} WHERE rowid = ?1"), [rowid])
}

/// 0.8.20 Slice 15e — reconcile the live `vector_default` attribute columns with
/// the registry's `filterable` set (TC-46: HITL-ratified NON-DESTRUCTIVE reshape,
/// within the current store format).
///
/// Diffs the DESIRED columns (from the registry) against the ACTUAL columns (on
/// the live table). When they already match — which is EVERY idempotent
/// re-registration and every boot re-derive that replays the same set — this is a
/// pure no-op: no reshape, no re-insert, vec0 untouched (so boot never silently
/// wipes a corpus). When they differ, performs ONE non-destructive reshape.
///
/// Returns `true` iff a reshape was performed. Runs the DDL directly on the passed
/// connection/transaction (no nested transaction), so a caller already inside a
/// write transaction (`configure_projections`) gets the reshape atomically with
/// its registry mutation. A no-op (and returns `false`) when `vector_default` does
/// not exist (a DB opened without an embedder).
pub(crate) fn reconcile_vector_attr_columns(
    conn: &Connection,
    dimension: u32,
) -> rusqlite::Result<bool> {
    let table_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
            [DEFAULT_VECTOR_PARTITION],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if !table_exists {
        return Ok(false);
    }
    let desired = desired_vector_attr_columns(conn)?;
    let actual = actual_vector_attr_columns(conn)?;
    let has_scope = vector_partition_has_scope_column(conn)?;
    if desired == actual && has_scope {
        return Ok(false);
    }
    reshape_vector_partition_nondestructive(conn, dimension, &desired, &actual, false, has_scope)?;
    Ok(true)
}

/// Rebuild vec0 attribute metadata from canonical bodies and registry sources while
/// preserving the existing column set. A changed nested source can leave the
/// `attr_<hex>` schema untouched while changing every value behind that column.
pub(crate) fn refresh_vector_attr_values(
    conn: &Connection,
    dimension: u32,
) -> rusqlite::Result<()> {
    let desired = desired_vector_attr_columns(conn)?;
    let actual = actual_vector_attr_columns(conn)?;
    let has_scope = vector_partition_has_scope_column(conn)?;
    if desired != actual || !desired.is_empty() || !has_scope {
        reshape_vector_partition_nondestructive(
            conn, dimension, &desired, &actual, true, has_scope,
        )?;
    }
    Ok(())
}

/// Refresh one reactivated node's vec0 metadata without reshaping the corpus.
///
/// Activation re-projects this node's canonical attributes after a source change
/// that could have happened while it was deleted. vec0 accepts metadata `UPDATE`s,
/// so only this row needs to be refreshed; a full partition reshape belongs to
/// registry shape/source reconciliation, not the ordinary lifecycle path.
pub(crate) fn refresh_vector_attr_values_for_row(
    conn: &Connection,
    rowid: i64,
    body: &str,
    properties: Option<&PropertyMap>,
) -> rusqlite::Result<()> {
    let (cols_sql, _, values) = vector_attr_insert_fragments(conn, body, properties, 1)?;
    if cols_sql.is_empty() {
        return Ok(());
    }
    let assignments = cols_sql
        .trim_start_matches(", ")
        .split(", ")
        .enumerate()
        .map(|(index, col)| format!("{col} = ?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let rowid_index = values.len() + 1;
    let ids = vector_ids_for_cursor(conn, rowid)?;
    for id in ids {
        let mut row_values = values.clone();
        row_values.push(rusqlite::types::Value::Integer(id));
        conn.execute(
            &format!(
                "UPDATE {DEFAULT_VECTOR_PARTITION} SET {assignments} WHERE rowid = ?{rowid_index}"
            ),
            rusqlite::params_from_iter(row_values.iter()),
        )?;
    }
    Ok(())
}

fn reshape_vector_partition_nondestructive(
    conn: &Connection,
    dimension: u32,
    desired_cols: &[String],
    actual_cols: &[String],
    refresh_values: bool,
    actual_has_scope: bool,
) -> rusqlite::Result<()> {
    // Stage: base columns + optional `scope` + every ACTUAL attribute column
    // (so no at-rest value is lost, even for a column being dropped).
    let mut stage_defs = String::new();
    let mut stage_names =
        String::from("rowid, embedding, embedding_bin, source_type, kind, created_at, status, lifecycle, valid_from, valid_until");
    if actual_has_scope {
        stage_defs.push_str(",\n             scope         TEXT");
        stage_names.push_str(", scope");
    }
    for col in actual_cols {
        stage_defs.push_str(&format!(",\n             {col} TEXT"));
        stage_names.push_str(&format!(", {col}"));
    }
    conn.execute_batch(&format!(
        "CREATE TABLE _fathomdb_vector_reshape_stage (
             rowid         INTEGER PRIMARY KEY,
             embedding     BLOB NOT NULL,
             embedding_bin BLOB NOT NULL,
             source_type   TEXT,
             kind          TEXT,
             created_at    INTEGER,
             status        TEXT,
             lifecycle     INTEGER,
             valid_from    INTEGER,
             valid_until   INTEGER{stage_defs}
         );
         INSERT INTO _fathomdb_vector_reshape_stage({stage_names})
             SELECT {stage_names} FROM {DEFAULT_VECTOR_PARTITION};
         DROP TABLE {DEFAULT_VECTOR_PARTITION};"
    ))?;

    // Recreate at the DESIRED Schema-39 8-col shape (scope + plain TEXT attr columns).
    conn.execute_batch(&vector_partition_create_sql(dimension, false, desired_cols))?;

    let mut insert_cols =
        String::from("rowid, embedding, embedding_bin, source_type, kind, created_at, status, lifecycle, valid_from, valid_until, scope");
    let mut placeholders = String::from("?1, ?2, vec_bit(?3), ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11");
    let mut stage_select_actual = String::new();
    let mut need_owner_source = false;
    let mut col_plan: Vec<(usize, Option<(String, StoredProjection)>)> =
        Vec::with_capacity(desired_cols.len());
    let registry = load_projection_registry(conn)?;
    for (idx, col) in desired_cols.iter().enumerate() {
        insert_cols.push_str(&format!(", {col}"));
        placeholders.push_str(&format!(", ?{}", 12 + idx));
        if let Some(actual_pos) = actual_cols.iter().position(|a| a == col) {
            if !refresh_values {
                stage_select_actual.push_str(&format!(", s.{col}"));
                col_plan.push((actual_pos, None));
                continue;
            }
        }
        need_owner_source = true;
        if let Some(name) = decode_attr_vec0_column(col) {
            let stored = registry
                .get(&(PropertyEntityClass::Node, name.clone()))
                .ok_or(rusqlite::Error::InvalidQuery)?
                .clone();
            col_plan.push((usize::MAX, Some((name, stored))));
        } else {
            col_plan.push((usize::MAX, None));
        }
    }

    let scope_expr =
        if actual_has_scope { "COALESCE(s.scope, '')" } else { "COALESCE(cn.scope, ce.scope, '')" };
    let select_stage_sql = if !actual_has_scope || need_owner_source {
        let extra_owner_cols = if need_owner_source { ", cn.body, cn.properties" } else { "" };
        format!(
            "SELECT s.rowid, s.embedding, s.embedding_bin, s.source_type, s.kind, s.created_at, \
             s.status, s.lifecycle, s.valid_from, s.valid_until, {scope_expr}{extra_owner_cols}{stage_select_actual} \
             FROM _fathomdb_vector_reshape_stage s \
             LEFT JOIN _fathomdb_vector_rows vr ON vr.rowid = s.rowid \
             LEFT JOIN canonical_nodes cn ON cn.write_cursor = vr.write_cursor \
             LEFT JOIN canonical_edges ce ON ce.write_cursor = vr.write_cursor"
        )
    } else {
        format!(
            "SELECT s.rowid, s.embedding, s.embedding_bin, s.source_type, s.kind, s.created_at, \
             s.status, s.lifecycle, s.valid_from, s.valid_until, {scope_expr}{stage_select_actual} \
             FROM _fathomdb_vector_reshape_stage s"
        )
    };
    let insert_sql =
        format!("INSERT INTO {DEFAULT_VECTOR_PARTITION}({insert_cols}) VALUES({placeholders})");
    {
        let mut read_stmt = conn.prepare(&select_stage_sql)?;
        let mut write_stmt = conn.prepare(&insert_sql)?;
        let mut rows = read_stmt.query([])?;
        let mut surviving_idx_map = Vec::with_capacity(col_plan.len());
        let mut next_surviving_col = if need_owner_source { 13_usize } else { 11_usize };
        for (actual_pos, back_spec) in &col_plan {
            if *actual_pos != usize::MAX && back_spec.is_none() {
                surviving_idx_map.push(Some(next_surviving_col));
                next_surviving_col += 1;
            } else {
                surviving_idx_map.push(None);
            }
        }
        while let Some(row) = rows.next()? {
            let mut binds: Vec<rusqlite::types::Value> =
                Vec::with_capacity(11 + desired_cols.len());
            for i in 0..11 {
                let val: rusqlite::types::Value = row.get(i)?;
                if i == 6 || i == 10 {
                    // status and scope: ensure non-NULL for vec0 NOT-NULL semantics
                    match val {
                        rusqlite::types::Value::Text(s) => {
                            binds.push(rusqlite::types::Value::Text(s))
                        }
                        _ => binds.push(rusqlite::types::Value::Text(String::new())),
                    }
                } else {
                    binds.push(val);
                }
            }
            let (owner_body, owner_props): (Option<String>, Option<PropertyMap>) =
                if need_owner_source {
                    (
                        row.get::<_, Option<String>>(11)?,
                        crate::properties::decode_properties_row_col(row, 12)?,
                    )
                } else {
                    (None, None)
                };
            for (slot_idx, (_, back_spec)) in col_plan.iter().enumerate() {
                if let Some(surv_col) = surviving_idx_map[slot_idx] {
                    let s: Option<String> = row.get(surv_col)?;
                    binds.push(rusqlite::types::Value::Text(s.unwrap_or_default()));
                } else if let (Some(body), Some((name, stored))) =
                    (owner_body.as_deref(), back_spec)
                {
                    let typed = crate::projection_registry::extract_typed_node_property(
                        conn,
                        body,
                        owner_props.as_ref(),
                        name,
                        stored,
                    )?;
                    binds.push(rusqlite::types::Value::Text(
                        crate::projection_registry::encode_property_vec0_metadata(typed.as_ref()),
                    ));
                } else {
                    binds.push(rusqlite::types::Value::Text(String::new()));
                }
            }
            write_stmt.execute(rusqlite::params_from_iter(binds.iter()))?;
        }
    }
    conn.execute_batch("DROP TABLE _fathomdb_vector_reshape_stage;")?;
    Ok(())
}

/// SQL fragment implementing the D3 `kind -> source_type` map.
/// Historical mapping oracle for the drift-detection unit test.
#[cfg(test)]
pub(crate) const KIND_TO_SOURCE_TYPE_CASE_SQL: &str = "CASE s.kind
    WHEN 'email'   THEN 'email'
    WHEN 'article' THEN 'article'
    WHEN 'paper'   THEN 'paper'
    WHEN 'meeting' THEN 'meeting'
    WHEN 'note'    THEN 'note'
    WHEN 'todo'    THEN 'todo'
    WHEN 'doc'     THEN 'article'
    ELSE 'article'
END";

pub(crate) fn encode_vector_blob(vector: &[f32]) -> Vec<u8> {
    vector.iter().flat_map(|value| value.to_le_bytes()).collect()
}

pub(crate) fn decode_vector_blob(bytes: &[u8]) -> Vec<f32> {
    debug_assert_eq!(bytes.len() % 4, 0, "f32 BLOB length must be multiple of 4");
    bytes.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

/// 0.8.18 Slice 5 — produce the packed 1-bit `embedding_bin` blob for a (possibly
/// mean-centered) f32 vector via the SAME SQL `vec_quantize_binary` the production
/// Phase-1 path uses, so the probe's bits are byte-equal to the engine's
/// `embedding_bin` production. `None` on any SQL/serialization error.
pub(crate) fn quantize_binary_via_sql(connection: &Connection, vector: &[f32]) -> Option<Vec<u8>> {
    let json = serde_json::to_string(vector).ok()?;
    connection
        .query_row("SELECT vec_quantize_binary(vec_f32(?1))", [json], |row| {
            row.get::<_, Vec<u8>>(0)
        })
        .ok()
}

/// 0.8.18 Slice 5 — Hamming distance (differing bit count) between two equal-length
/// packed bit blobs. Unequal lengths ⇒ count every bit of the length delta as
/// differing (a shape divergence is a divergence).
pub(crate) fn hamming_bytes(a: &[u8], b: &[u8]) -> u64 {
    let common = a.len().min(b.len());
    let mut flips: u64 = 0;
    for i in 0..common {
        flips += u64::from((a[i] ^ b[i]).count_ones());
    }
    let extra = a.len().abs_diff(b.len());
    flips + (extra as u64) * 8
}

/// Maps the writer-facing `kind` value to the locked Pack 1
/// `source_type` partition-key vocabulary. Must stay in lockstep with
/// the CASE WHEN inlined in migration step 9
/// (`fathomdb-schema/src/lib.rs`); the drift-detection unit test in
/// this module's `tests` mod enforces that. Per
/// `dev/design/0.7.0-vector-quant-pack1.md` D3.
pub(crate) const VECTOR_COMMITTABLE_NODE_KIND_SOURCE_TYPES: [(&str, &str); 7] = [
    ("email", "email"),
    ("article", "article"),
    ("paper", "paper"),
    ("meeting", "meeting"),
    ("note", "note"),
    ("todo", "todo"),
    // Synthetic AC-013 test fixture; coerced so the 6-value HITL lock holds.
    ("doc", "article"),
];

pub(crate) fn resolve_builtin_source_type(kind: &str) -> Option<&'static str> {
    if kind == "edge_fact" {
        return Some("edge_fact");
    }
    VECTOR_COMMITTABLE_NODE_KIND_SOURCE_TYPES
        .iter()
        .find_map(|(candidate, source_type)| (*candidate == kind).then_some(*source_type))
}

pub(crate) fn kind_is_builtin_vector_committable(kind: &str) -> bool {
    resolve_builtin_source_type(kind).is_some()
}

pub(crate) fn resolve_source_type_on<'a>(
    conn: &Connection,
    kind: &'a str,
) -> Result<std::borrow::Cow<'a, str>, EngineError> {
    if let Some(builtin) = resolve_builtin_source_type(kind) {
        return Ok(std::borrow::Cow::Borrowed(builtin));
    }
    if kind == "edge_fact" {
        return Ok(std::borrow::Cow::Borrowed("edge_fact"));
    }
    let enrolled: bool = conn
        .prepare_cached(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_vector_kinds WHERE kind = ?1 AND kind <> 'edge_fact')",
        )
        .map_err(|_| EngineError::Storage)?
        .query_row([kind], |row| row.get(0))
        .map_err(|_| EngineError::Storage)?;
    if enrolled {
        Ok(std::borrow::Cow::Borrowed(kind))
    } else {
        Err(EngineError::Storage)
    }
}

pub(crate) fn kind_is_vector_committable_on(conn: &Connection, kind: &str) -> bool {
    resolve_source_type_on(conn, kind).is_ok()
}

impl Engine {
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn write_vector_for_test(
        &self,
        kind: &str,
        text: &str,
    ) -> Result<WriteReceipt, EngineError> {
        let _operation = self.ensure_open()?;
        let embedder = self
            .runtime_embedder
            .as_ref()
            .and_then(Weak::upgrade)
            .ok_or(EngineError::EmbedderNotConfigured)?;

        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        if !kind_is_vector_indexed(connection, kind)? {
            return Err(EngineError::KindNotVectorIndexed);
        }

        let expected = default_profile_dimension(connection)?;
        ensure_vector_partition(connection, expected).map_err(|_| EngineError::Storage)?;
        let vector = embedder.embed(text).map_err(map_runtime_embedder_error)?;
        let actual = u32::try_from(vector.len()).unwrap_or(u32::MAX);
        if actual != expected {
            return Err(EngineError::EmbedderDimensionMismatch { expected, actual });
        }

        let cursor = self.next_cursor.load(Ordering::SeqCst).saturating_add(1);
        // EU-5a2 mean-centering apply path (write side). f32 BLOB stored
        // is ALWAYS un-centered; the sign-quant input is the centered
        // vector iff the identity is MC-required AND a `mean_vec` is
        // pinned. NoopEmbedder identity (the only EU-5a2 live one) is
        // NOT MC-required, so this is a no-op until EU-5b's flip.
        let blob = encode_vector_blob(&vector);
        let bin_blob = if identity_requires_mean_centering(&self.runtime_embedder_identity) {
            match read_pinned_mean_vec(connection, self.runtime_embedder_identity.dimension)? {
                Some(mean) => encode_vector_blob(&subtract_mean(&vector, &mean)),
                None => blob.clone(),
            }
        } else {
            blob.clone()
        };
        let source_type = resolve_source_type_on(connection, kind)?;
        let now_unix =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;

        // EU-5b — feed the streaming mean accumulator (if live) and detect
        // a threshold-crossing pin. The mean materialization, pre-pin
        // re-quantize, and `MeanVecPinned` event emission all happen in
        // the SAME SQLite transaction as the row INSERT.
        let pin_event = {
            let runtime = &self.projection_runtime.shared;
            let mut accumulator =
                runtime.mean_accumulator.lock().map_err(|_| EngineError::Storage)?;
            if let Some(acc) = accumulator.as_mut() {
                acc.add(&vector);
                if acc.count() >= mean::MEAN_VEC_PIN_THRESHOLD {
                    let mean = acc.materialize();
                    *accumulator = None;
                    Some(mean)
                } else {
                    None
                }
            } else {
                None
            }
        };

        let tx = connection.transaction().map_err(|_| EngineError::Storage)?;
        tx.execute(
            "INSERT INTO _fathomdb_vector_rows(rowid, kind, write_cursor) VALUES(?1, ?2, ?3)",
            params![cursor, kind, cursor],
        )
        .map_err(|_| EngineError::Storage)?;
        tx.execute(
            "INSERT INTO _fathomdb_vector_spans(vector_rowid,write_cursor,window_number,byte_start,byte_end,body_length) VALUES(?1,?1,0,0,NULL,?2)",
            params![cursor, text.len()],
        ).map_err(|_| EngineError::Storage)?;
        // Slice 10 / G10 — `status` ships an empty-string sentinel only: vec0 TEXT
        // metadata columns are NOT NULL-able ("Expected text for TEXT metadata
        // column"), so the "no real population yet" state is `''`, not NULL.
        //
        // 0.8.20 Slice 15e — this test helper carries no JSON body, so every live
        // `filterable` `attr_<hex>` column binds the `''` sentinel (an empty body
        // extracts nothing). When the table has no attr columns the statement is
        // byte-identical to the shipped form.
        let scope: String = tx
            .query_row(
                "SELECT scope FROM canonical_nodes WHERE write_cursor = ?1
                 UNION ALL SELECT scope FROM canonical_edges WHERE write_cursor = ?1 LIMIT 1",
                [cursor as i64],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| EngineError::Storage)?
            .unwrap_or_default();
        let (cols_sql, ph_sql, attr_vals) =
            vector_attr_insert_fragments(&tx, "", None, 8).map_err(|_| EngineError::Storage)?;
        let sql = format!(
            "INSERT INTO vector_default(
                rowid, embedding, embedding_bin, source_type, kind, created_at, status,lifecycle,valid_from,valid_until,scope{cols_sql}
             ) VALUES(?1, ?2, vec_quantize_binary(?3), ?4, ?5, ?6, '',0,-9223372036854775808,9223372036854775807,?7{ph_sql})"
        );
        let mut pv: Vec<rusqlite::types::Value> = vec![
            rusqlite::types::Value::Integer(cursor as i64),
            rusqlite::types::Value::Blob(blob.clone()),
            rusqlite::types::Value::Blob(bin_blob.clone()),
            rusqlite::types::Value::Text(source_type.to_string()),
            rusqlite::types::Value::Text(kind.to_string()),
            rusqlite::types::Value::Integer(now_unix),
            rusqlite::types::Value::Text(scope),
        ];
        pv.extend(attr_vals);
        tx.execute(&sql, rusqlite::params_from_iter(pv.iter()))
            .map_err(|_| EngineError::Storage)?;

        let mut emitted_event: Option<EmbedderEvent> = None;
        if let Some(mean_vec) = pin_event {
            let mean_bytes = encode_vector_blob(&mean_vec);
            tx.execute(
                "UPDATE _fathomdb_embedder_profiles SET mean_vec = ?1 WHERE profile = 'default'",
                params![mean_bytes],
            )
            .map_err(|_| EngineError::Storage)?;
            // Read all pre-pin (rowid, embedding) and re-quantize within
            // the same tx. The just-inserted row above is also covered.
            let rows: Vec<(i64, Vec<u8>)> = {
                let mut statement = tx
                    .prepare("SELECT rowid, embedding FROM vector_default ORDER BY rowid")
                    .map_err(|_| EngineError::Storage)?;
                let mapped = statement
                    .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?)))
                    .map_err(|_| EngineError::Storage)?;
                let mut out = Vec::new();
                for r in mapped {
                    out.push(r.map_err(|_| EngineError::Storage)?);
                }
                out
            };
            let (doc_count, _) = run_pin_and_requantize_pass(&tx, &rows, &mean_vec)?;
            emitted_event = Some(EmbedderEvent::MeanVecPinned {
                dim: u32::try_from(mean_vec.len()).unwrap_or(u32::MAX),
                doc_count,
            });
        }

        tx.commit().map_err(|_| EngineError::Storage)?;

        if let Some(ev) = emitted_event {
            if let Ok(mut events) = self.projection_runtime.shared.pending_events.lock() {
                events.push(ev);
            }
        }

        self.next_cursor.store(cursor, Ordering::SeqCst);
        // G8 — this path (embedder-profile pin) commits no canonical edges, so
        // no endpoint can dangle.
        Ok(WriteReceipt { cursor, row_cursors: vec![cursor], dangling_edge_endpoints: 0 })
    }

    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn vector_row_count_for_test(&self) -> Result<u64, EngineError> {
        let _operation = self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        connection
            .query_row("SELECT COUNT(*) FROM vector_default", [], |row| row.get::<_, u64>(0))
            .map_err(|_| EngineError::Storage)
    }

    /// Test-only distinction between a physical vec0 row and its terminal
    /// readiness record. Closure may terminalize an ineligible projection
    /// without publishing vector bytes, so readiness alone is not a row oracle.
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn has_vector_row_for_cursor_for_test(&self, cursor: u64) -> Result<bool, EngineError> {
        let _operation = self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM vector_default WHERE rowid = ?1)",
                [i64::try_from(cursor).map_err(|_| EngineError::Storage)?],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|_| EngineError::Storage)
    }

    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn read_vector_blob_for_test(&self, rowid: i64) -> Result<Vec<u8>, EngineError> {
        let _operation = self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        connection
            .query_row("SELECT embedding FROM vector_default WHERE rowid = ?1", [rowid], |row| {
                row.get::<_, Vec<u8>>(0)
            })
            .map_err(|_| EngineError::Storage)
    }

    /// 0.8.20 Slice 15e — read a row's raw `embedding_bin` blob bytes (the
    /// sign-quantized vector). Used to prove the non-destructive reshape copies the
    /// bits VERBATIM (condition #4): the pre-reshape and post-reshape bytes must be
    /// byte-identical.
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn read_vector_bin_for_test(&self, rowid: i64) -> Result<Vec<u8>, EngineError> {
        let _operation = self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        connection
            .query_row(
                "SELECT embedding_bin FROM vector_default WHERE rowid = ?1",
                [rowid],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .map_err(|_| EngineError::Storage)
    }
}

pub(crate) fn vector_validity_bounds(from: Option<i64>, until: Option<i64>) -> (i64, i64) {
    match until {
        Some(i64::MIN) => (i64::MAX, i64::MIN),
        Some(end) => (from.unwrap_or(i64::MIN), end - 1),
        None => (from.unwrap_or(i64::MIN), i64::MAX),
    }
}

pub(crate) fn vector_eligibility_and_scope_for_cursor(
    conn: &Connection,
    cursor: i64,
) -> rusqlite::Result<(i64, i64, i64, String)> {
    let canonical: Option<(i64, Option<i64>, Option<i64>, String)> = conn.query_row(
        "SELECT (CASE WHEN superseded_at IS NULL THEN 0 ELSE 2 END) + (CASE WHEN state='active' THEN 0 ELSE 1 END),valid_from,valid_until,scope FROM canonical_nodes WHERE write_cursor=?1
         UNION ALL SELECT CASE WHEN superseded_at IS NULL THEN 0 ELSE 2 END,NULL,t_invalid,scope FROM canonical_edges WHERE write_cursor=?1 LIMIT 1",
        [cursor], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
    ).optional()?;
    let (lifecycle, from, until, scope) = canonical.unwrap_or((0, None, None, String::new()));
    let (from, until) = vector_validity_bounds(from, until);
    Ok((lifecycle, from, until, scope))
}

pub(crate) fn vector_eligibility_for_cursor(
    conn: &Connection,
    cursor: i64,
) -> rusqlite::Result<(i64, i64, i64)> {
    let (lifecycle, from, until, _) = vector_eligibility_and_scope_for_cursor(conn, cursor)?;
    Ok((lifecycle, from, until))
}

pub(crate) fn refresh_vector_eligibility_for_cursor(
    conn: &Connection,
    cursor: i64,
) -> rusqlite::Result<()> {
    let (lifecycle, from, until) = vector_eligibility_for_cursor(conn, cursor)?;
    let ids = conn
        .prepare_cached("SELECT rowid FROM _fathomdb_vector_rows WHERE write_cursor=?1")?
        .query_map([cursor], |r| r.get::<_, i64>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for id in ids {
        conn.execute(
            "UPDATE vector_default SET lifecycle=?1,valid_from=?2,valid_until=?3 WHERE rowid=?4",
            params![lifecycle, from, until, id],
        )?;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) use tests::slice70_mapping_fixture;

#[cfg(test)]
mod tests {
    use super::*;
    pub(crate) fn slice70_mapping_fixture(with_attribute: bool) -> Connection {
        let dir = tempfile::TempDir::new().unwrap();
        let opened = Engine::open(dir.path().join("init.sqlite")).unwrap();
        let conn = Connection::open_in_memory().unwrap();
        fathomdb_schema::migrate(&conn).unwrap();
        if with_attribute {
            conn.execute(
                "INSERT INTO _fathomdb_projection_registry(name,roles) VALUES('owner','filterable')",
                [],
            )
            .unwrap();
        }
        let attrs = if with_attribute { vec![attr_vec0_column("owner")] } else { vec![] };
        conn.execute_batch(&vector_partition_create_sql(8, false, &attrs)).unwrap();
        let blob = encode_vector_blob(&[1.0; 8]);
        for id in [101, 102] {
            conn.execute("INSERT INTO _fathomdb_vector_rows VALUES(?1,'doc',10)", [id]).unwrap();
            let attr = if with_attribute {
                format!(",{}", attr_vec0_column("owner"))
            } else {
                String::new()
            };
            let value = if with_attribute { ",''" } else { "" };
            conn.execute(&format!("INSERT INTO vector_default(rowid,embedding,embedding_bin,source_type,kind,created_at,status,lifecycle,valid_from,valid_until,scope{attr}) VALUES(?1,?2,vec_quantize_binary(?2),'article','doc',0,'',0,-9223372036854775808,9223372036854775807,''{value})"),params![id,blob]).unwrap();
        }
        opened.engine.close().unwrap();
        conn
    }
}

#[cfg(test)]
#[test]
fn slice70_record_erasure_removes_all_mapped_vector_ids() {
    let conn = slice70_mapping_fixture(false);
    erase_row_projections(&conn, 10).unwrap();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM vector_default", [], |r| r.get::<_, i64>(0)).unwrap(),
        0
    );
}

#[cfg(test)]
#[test]
fn slice70_attribute_refresh_uses_record_mapping() {
    let conn = slice70_mapping_fixture(true);
    refresh_vector_attr_values_for_row(&conn, 10, r#"{"owner":"alice"}"#, None).unwrap();
    let col = attr_vec0_column("owner");
    let values = conn
        .prepare(&format!("SELECT {col} FROM vector_default"))
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(values, vec!["\u{1}alice"; 2]);
}

#[cfg(test)]
#[test]
fn slice70_attribute_backfill_uses_record_mapping() {
    let conn = slice70_mapping_fixture(false);
    conn.execute(
        "INSERT INTO _fathomdb_projection_registry(name,roles) VALUES('owner','filterable')",
        [],
    )
    .unwrap();
    conn.execute("INSERT INTO canonical_nodes(write_cursor,kind,body,state,superseded_at) VALUES(10,'doc',?1,'deleted',11)", [r#"{"owner":"alice"}"#]).unwrap();
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM canonical_attributes", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );

    let col = attr_vec0_column("owner");
    reshape_vector_partition_nondestructive(&conn, 8, std::slice::from_ref(&col), &[], false, true)
        .unwrap();
    let values = conn
        .prepare(&format!("SELECT {col} FROM vector_default"))
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(values, vec!["\u{1}alice"; 2]);
}

pub(crate) fn vector_ids_for_cursor(conn: &Connection, cursor: i64) -> rusqlite::Result<Vec<i64>> {
    conn.prepare_cached("SELECT rowid FROM _fathomdb_vector_rows WHERE write_cursor=?1")?
        .query_map([cursor], |r| r.get(0))?
        .collect()
}

pub(crate) fn delete_vector_record_rows(conn: &Connection, cursor: i64) -> rusqlite::Result<usize> {
    // A missing head mapping loses ownership evidence. Refuse rather than
    // claim complete erasure or delete an ID that may belong to another record.
    let unmapped_head: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM vector_default WHERE rowid=?1) AND NOT EXISTS(SELECT 1 FROM _fathomdb_vector_rows WHERE rowid=?1)",
        [cursor], |r| r.get(0),
    )?;
    if unmapped_head {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let mut count = 0;
    for id in vector_ids_for_cursor(conn, cursor)? {
        count += delete_vector_partition_row(conn, id)?;
    }
    Ok(count)
}

#[cfg(test)]
#[test]
fn slice70_inclusive_bounds_match_nullable_reference() {
    let strategy = (
        proptest::option::of(proptest::prelude::any::<i64>()),
        proptest::option::of(proptest::prelude::any::<i64>()),
        proptest::prelude::any::<i64>(),
    );
    proptest::test_runner::TestRunner::default()
        .run(&strategy, |(from, until, now)| {
            let (lower, upper) = vector_validity_bounds(from, until);
            let expected =
                from.is_none_or(|start| start <= now) && until.is_none_or(|end| end > now);
            if (lower <= now && upper >= now) == expected {
                Ok(())
            } else {
                Err(proptest::test_runner::TestCaseError::fail(
                    "inclusive bounds differ from nullable canonical predicate",
                ))
            }
        })
        .unwrap();
}

#[cfg(test)]
#[test]
fn slice70_mapped_lifecycle_rollback_and_recenter_preserve_metadata() {
    let mut conn = slice70_mapping_fixture(false);
    conn.execute("INSERT INTO canonical_nodes(write_cursor,kind,body,state,valid_from,valid_until) VALUES(10,'doc','body','active',5,20)",[]).unwrap();
    refresh_vector_eligibility_for_cursor(&conn, 10).unwrap();
    {
        let tx = conn.transaction().unwrap();
        tx.execute(
            "UPDATE canonical_nodes SET state='deleted',superseded_at=50 WHERE write_cursor=10",
            [],
        )
        .unwrap();
        refresh_vector_eligibility_for_cursor(&tx, 10).unwrap();
        for id in [101, 102] {
            assert_eq!(
                tx.query_row("SELECT lifecycle FROM vector_default WHERE rowid=?1", [id], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                3
            );
        }
        tx.rollback().unwrap();
    }
    let tx = conn.transaction().unwrap();
    let rows = vec![(101, encode_vector_blob(&[1.0; 8])), (102, encode_vector_blob(&[1.0; 8]))];
    crate::mean::run_pin_and_requantize_pass(&tx, &rows, &[0.1; 8]).unwrap();
    for id in [101, 102] {
        let actual=tx.query_row("SELECT lifecycle,valid_from,valid_until,embedding FROM vector_default WHERE rowid=?1",[id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?,r.get::<_,Vec<u8>>(3)?))).unwrap();
        assert_eq!(actual, (0, 5, 19, encode_vector_blob(&[1.0; 8])));
    }
    assert_eq!(
        tx.query_row("SELECT state FROM canonical_nodes WHERE write_cursor=10", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "active"
    );
    tx.rollback().unwrap();
}

#[cfg(test)]
#[test]
fn slice70_erasure_does_not_delete_another_records_vector_id() {
    let conn = slice70_mapping_fixture(false);
    conn.execute("UPDATE _fathomdb_vector_rows SET write_cursor=20 WHERE rowid=101", []).unwrap();
    assert_eq!(delete_vector_record_rows(&conn, 101).unwrap(), 0);
    erase_row_projections(&conn, 10).unwrap();
    let ids = conn
        .prepare("SELECT rowid FROM vector_default")
        .unwrap()
        .query_map([], |r| r.get::<_, i64>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(ids, [101]);
}
