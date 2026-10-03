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
    connection: &mut Connection,
    dimension: u32,
) -> rusqlite::Result<()> {
    // 0.7.0 Pack 1 schema per dev/design/0.7.0-vector-quant-pack1.md D1/D2:
    // f32 `embedding` + binary-quant sibling `embedding_bin` + `source_type`
    // partition key + `kind` + `created_at`. The vec0 column type is
    // dim-parameterized, so the reshape lives here rather than in the
    // SQL-only migration framework — see fathomdb-schema migration step 9
    // and dev/plans/runs/0.7.0-PVQ-P1-IMPL-output.json for the deviation
    // from the design memo's "Choose (a)" guidance.
    //
    // Three paths:
    //   (1) no vector_default       -> CREATE at new shape.
    //   (2) old single-column shape -> stage + drop + recreate at new shape
    //                                  + repopulate with vec_quantize_binary.
    //   (3) already new shape       -> no-op.
    let existing_sql: Option<String> = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name=?1",
            [DEFAULT_VECTOR_PARTITION],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    // Slice 10 / G10 — 3-way shape-sentinel (fixes the prior
    // `contains("embedding_bin")` no-op that hid the `status` column from
    // existing Pack-1 DBs):
    //   `status` present       -> Pack-2 (current) shape, no-op.
    //   `embedding_bin` present -> Pack-1 -> stage + recreate + back-fill status.
    //   neither                 -> legacy single-column -> migrate to current.
    match existing_sql {
        None => create_vector_partition(connection, dimension),
        Some(sql) if sql.contains("status") => Ok(()),
        Some(sql) if sql.contains("embedding_bin") => {
            migrate_vector_partition_pack1_to_pack2(connection, dimension)
        }
        Some(_) => migrate_vector_partition_to_pack1(connection, dimension),
    }
}

/// The current (Pack-2) `vector_default` vec0 shape. Slice 10 / G10 adds a plain
/// `status TEXT` metadata column — **not** aux (`+status`): aux columns
/// hard-error under a KNN `WHERE`, and the G10 filter constrains `status` in the
/// phase-1 KNN statement. `status` ships NULL plumbing only (no population source
/// yet).
///
/// 0.8.20 Slice 15e — `attr_cols` are the declared-`filterable` attribute columns
/// (byte-safe `attr_<hex>` identifiers, see [`attr_vec0_column`]), each a PLAIN
/// `TEXT` metadata column (never aux `+`), appended after `status`. **When
/// `attr_cols` is empty the produced SQL is byte-identical to the shipped shape**
/// — every existing caller passes `&[]`, so no shipped behaviour changes.
fn vector_partition_create_sql(
    dimension: u32,
    if_not_exists: bool,
    attr_cols: &[String],
) -> String {
    let guard = if if_not_exists { "IF NOT EXISTS " } else { "" };
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
            status TEXT{attrs}\
         )"
    )
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

/// 0.8.20 Slice 15e — the DESIRED `attr_<hex>` columns implied by the durable
/// projection registry: one per `filterable` projection, sorted by attribute name
/// (⇒ sorted by column, since hex encoding preserves byte order). This is the
/// derived-cache source the reshape reconciles the live vec0 shape against.
fn desired_vector_attr_columns(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let registry = load_projection_registry(conn)?;
    let mut cols: Vec<String> = registry
        .iter()
        .filter(|(_, stored)| stored.roles.contains(&ProjectionRole::Filterable))
        .map(|(name, _)| attr_vec0_column(name))
        .collect();
    cols.sort();
    Ok(cols)
}

/// 0.8.20 Slice 15e — the `attr_<hex>` columns actually present on the live
/// `vector_default` vec0 table, parsed from its `CREATE VIRTUAL TABLE` SQL and
/// sorted. Empty when the table is absent. Parsing the SQL (rather than a PRAGMA)
/// keeps this robust across vec0 versions and shadow-table layouts.
pub(crate) fn actual_vector_attr_columns(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let sql: Option<String> = conn
        .prepare_cached("SELECT sql FROM sqlite_master WHERE type='table' AND name=?1")
        .and_then(|mut statement| {
            statement.query_row([DEFAULT_VECTOR_PARTITION], |row| row.get::<_, String>(0))
        })
        .optional()?;
    let Some(sql) = sql else {
        return Ok(Vec::new());
    };
    let mut cols: Vec<String> = Vec::new();
    // Tokenize on any non-identifier byte; a token is an attribute column iff it
    // decodes as a well-formed `attr_<hex>` identifier.
    let mut token = String::new();
    let flush = |token: &mut String, cols: &mut Vec<String>| {
        if !token.is_empty() {
            if decode_attr_vec0_column(token).is_some() && !cols.contains(token) {
                cols.push(token.clone());
            }
            token.clear();
        }
    };
    for ch in sql.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            token.push(ch);
        } else {
            flush(&mut token, &mut cols);
        }
    }
    flush(&mut token, &mut cols);
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
/// following the shipped `migrate_vector_partition_pack1_to_pack2` precedent).
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
    if desired == actual {
        return Ok(false);
    }
    reshape_vector_partition_nondestructive(conn, dimension, &desired, &actual, false)?;
    Ok(true)
}

/// Rebuild vec0 attribute metadata from the canonical EAV projection while
/// preserving the existing column set. A changed nested source can leave the
/// `attr_<hex>` schema untouched while changing every value behind that column.
pub(crate) fn refresh_vector_attr_values(
    conn: &Connection,
    dimension: u32,
) -> rusqlite::Result<()> {
    let desired = desired_vector_attr_columns(conn)?;
    let actual = actual_vector_attr_columns(conn)?;
    if desired != actual || !desired.is_empty() {
        reshape_vector_partition_nondestructive(conn, dimension, &desired, &actual, true)?;
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
) -> rusqlite::Result<()> {
    let (cols_sql, _, mut values) = vector_attr_insert_fragments(conn, body, 1)?;
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
    values.push(rusqlite::types::Value::Integer(rowid));
    let rowid_index = values.len();
    conn.execute(
        &format!(
            "UPDATE {DEFAULT_VECTOR_PARTITION} SET {assignments} WHERE rowid = ?{rowid_index}"
        ),
        rusqlite::params_from_iter(values.iter()),
    )?;
    Ok(())
}

/// 0.8.20 Slice 15e — the NON-DESTRUCTIVE reshape itself. Stages every live row
/// (base columns + all ACTUAL attribute columns), drops + recreates
/// `vector_default` at the DESIRED shape, then re-inserts each row.
///
/// THE FOUR LOAD-BEARING CONDITIONS (any one broken ⇒ silently wrong results):
///   1. `rowid` is listed EXPLICITLY in the re-insert (a vec0 row maps to its node
///      by `rowid == write_cursor`; auto-assigned rowids would decouple every
///      embedding from its node);
///   2. each attribute column is PLAIN `TEXT` metadata (via
///      [`vector_partition_create_sql`]), never a vec0 `aux`/`+` column (aux
///      hard-errors a filtered KNN);
///   3. a DESIRED column with no ACTUAL predecessor back-fills each row from the
///      already-populated `canonical_attributes` (fix-1 finding 2) so a
///      pre-existing row whose body carries the attribute is immediately
///      filterable; the `''` sentinel (vec0 TEXT metadata is NOT-NULL-able) is
///      used ONLY where the attribute is genuinely absent, so an absent row
///      cleanly fails-to-match instead of erroring;
///   4. `embedding_bin` is copied VERBATIM via `vec_bit(...)` — NOT re-quantized —
///      so old rows keep their (possibly mean-centered) bits and stay Hamming-
///      comparable to new rows.
///
/// Runs on `conn` directly (the caller owns the transaction). Transactional
/// atomicity + reader isolation are the caller's responsibility, exactly as for
/// `migrate_vector_partition_pack1_to_pack2`.
fn reshape_vector_partition_nondestructive(
    conn: &Connection,
    dimension: u32,
    desired_cols: &[String],
    actual_cols: &[String],
    refresh_values: bool,
) -> rusqlite::Result<()> {
    // Stage: base columns + every ACTUAL attribute column (so no at-rest value is
    // lost, even for a column being dropped).
    let mut stage_defs = String::new();
    let mut stage_names =
        String::from("rowid, embedding, embedding_bin, source_type, kind, created_at, status");
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
             status        TEXT{stage_defs}
         );
         INSERT INTO _fathomdb_vector_reshape_stage({stage_names})
             SELECT {stage_names} FROM {DEFAULT_VECTOR_PARTITION};
         DROP TABLE {DEFAULT_VECTOR_PARTITION};"
    ))?;

    // Recreate at the DESIRED shape (plain TEXT attr columns — condition #2).
    conn.execute_batch(&vector_partition_create_sql(dimension, false, desired_cols))?;

    // Re-insert. `rowid` explicit (condition #1); `vec_bit(embedding_bin)`
    // verbatim, never re-quantized (condition #4); `status` and each surviving
    // attribute column carried forward; each NEW desired column back-fills from
    // `canonical_attributes` (the `''` sentinel only where genuinely absent —
    // condition #3).
    let mut insert_cols =
        String::from("rowid, embedding, embedding_bin, source_type, kind, created_at, status");
    let mut select_exprs = String::from(
        "rowid, embedding, vec_bit(embedding_bin), source_type, kind, created_at, status",
    );
    for col in desired_cols {
        insert_cols.push_str(&format!(", {col}"));
        if actual_cols.iter().any(|a| a == col) && !refresh_values {
            // Surviving column — carry its value forward (never NULL: vec0 TEXT
            // metadata is `''`-sentinelled, but COALESCE defends the stage table).
            select_exprs.push_str(&format!(", COALESCE({col}, '')"));
        } else {
            // New column — back-fill from the ALREADY-populated `canonical_attributes`
            // (fix-1 finding 2 [P2]). `configure_projections` runs `backfill_attribute`
            // (which fills `canonical_attributes` from each active row's body) BEFORE
            // this reshape, so a pre-existing row whose body carries the attribute is
            // immediately filterable — no false negative until a re-embed. The `''`
            // sentinel (condition #3) is used ONLY where the attribute is genuinely
            // ABSENT for that row (no `canonical_attributes` row ⇒ COALESCE → '').
            // The EAV value equals the vec0 write-time value by construction (both go
            // through `extract_scalar_attribute`), so pre-existing and freshly-written
            // rows share one filter semantics.
            match decode_attr_vec0_column(col) {
                Some(name) => {
                    // vec0/execute_batch takes no bind params; embed the decoded name
                    // as a SQL string literal, escaping single quotes.
                    //
                    // fix-3 [P2] — a PRESENT row (a canonical_attributes row exists)
                    // encodes its RAW `attr_value` as `\x01 || attr_value` (`char(1) ||
                    // ca.attr_value`), matching the write-time vec0 encoding so a
                    // pre-existing present-empty row (attr_value='') becomes the bare
                    // marker, NOT `''`. An ABSENT row (no canonical_attributes row) is
                    // the COALESCE default `''` (condition #3). `canonical_attributes`
                    // itself stays RAW — only this vec0 column is encoded.
                    let escaped = name.replace('\'', "''");
                    select_exprs.push_str(&format!(
                        ", COALESCE((SELECT char(1) || ca.attr_value FROM canonical_attributes ca \
                         WHERE ca.write_cursor = _fathomdb_vector_reshape_stage.rowid \
                           AND ca.attr_name = '{escaped}' LIMIT 1), '')"
                    ));
                }
                // A desired column always decodes (built by `attr_vec0_column`); if it
                // somehow does not, fall back to the sentinel rather than panic.
                None => select_exprs.push_str(", ''"),
            }
        }
    }
    conn.execute_batch(&format!(
        "INSERT INTO {DEFAULT_VECTOR_PARTITION}({insert_cols})
             SELECT {select_exprs} FROM _fathomdb_vector_reshape_stage;
         DROP TABLE _fathomdb_vector_reshape_stage;"
    ))?;
    Ok(())
}

/// Slice 10 / G10 — stage + recreate + back-fill upgrade of an existing
/// **Pack-1** `vector_default` (has `embedding_bin`, lacks `status`) to the
/// Pack-2 shape. The existing `embedding_bin` blob is preserved verbatim (it may
/// be mean-centered; re-quantizing from `embedding` would drop the centering),
/// and `status` back-fills NULL. Same transactional discipline as
/// `migrate_vector_partition_to_pack1`: a single `Connection::transaction()`;
/// reader handles are not opened until `ensure_vector_partition` returns, and
/// cross-process access is serialized by the sidecar lock, so readers never see
/// a partial reshape.
fn migrate_vector_partition_pack1_to_pack2(
    connection: &mut Connection,
    dimension: u32,
) -> rusqlite::Result<()> {
    let tx = connection.transaction()?;
    tx.execute_batch(
        "CREATE TABLE _fathomdb_vector_pack2_stage (
             rowid         INTEGER PRIMARY KEY,
             embedding     BLOB NOT NULL,
             embedding_bin BLOB NOT NULL,
             source_type   TEXT,
             kind          TEXT,
             created_at    INTEGER
         );
         INSERT INTO _fathomdb_vector_pack2_stage(
             rowid, embedding, embedding_bin, source_type, kind, created_at
         )
             SELECT rowid, embedding, embedding_bin, source_type, kind, created_at
             FROM vector_default;
         DROP TABLE vector_default;",
    )?;
    tx.execute_batch(&vector_partition_create_sql(dimension, false, &[]))?;
    // `vec_bit(...)` re-tags the staged blob with the BIT subtype vec0's bit
    // column requires (a raw blob loses the subtype and fails the type check).
    // This preserves the existing (possibly mean-centered) bits verbatim — no
    // re-quantize, so centering survives the upgrade. `status` back-fills the
    // empty-string sentinel (vec0 TEXT metadata is NOT NULL-able; reserved-gap
    // candidate 13).
    tx.execute_batch(
        "INSERT INTO vector_default(
             rowid, embedding, embedding_bin, source_type, kind, created_at, status
         )
             SELECT rowid, embedding, vec_bit(embedding_bin), source_type, kind, created_at, ''
             FROM _fathomdb_vector_pack2_stage;
         DROP TABLE _fathomdb_vector_pack2_stage;",
    )?;
    tx.commit()
}

/// SQL fragment implementing the D3 `kind -> source_type` map.
/// Used both by the Pack 1 reshape migration and by the drift-detection
/// unit test that pins it to [`resolve_source_type`].
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

/// Pack 1 in-place reshape of `vector_default`. Stages the existing
/// f32 corpus + each row's `kind`, drops the old single-column vec0
/// table, recreates at the runtime `dimension` with the Pack 1
/// columns, then repopulates with SQL-side `vec_quantize_binary` +
/// the D3 `kind -> source_type` mapping. The preflight CHECK on
/// unknown kinds has already run as migration step 9 by the time we
/// get here.
///
/// Atomicity: the DROP+CREATE+repopulate sequence runs inside a
/// rusqlite `Connection::transaction()` (DEFERRED begin per rusqlite
/// `transaction.rs:417`). Cross-process serialization is provided by
/// the engine's sidecar `acquire_lock` at `open_with_migrations`
/// (`lib.rs:1127` area); reader handles are not opened until
/// `ensure_vector_partition` returns (`lib.rs:1241` area), so readers
/// never observe a partial reshape.
fn migrate_vector_partition_to_pack1(
    connection: &mut Connection,
    dimension: u32,
) -> rusqlite::Result<()> {
    let tx = connection.transaction()?;
    tx.execute_batch(
        "CREATE TABLE _fathomdb_vector_migration_v0_7_0 (
             rowid     INTEGER PRIMARY KEY,
             embedding BLOB NOT NULL,
             kind      TEXT NOT NULL
         );
         INSERT INTO _fathomdb_vector_migration_v0_7_0(rowid, embedding, kind)
             SELECT v.rowid, v.embedding, r.kind
             FROM vector_default v
             JOIN _fathomdb_vector_rows r ON r.rowid = v.rowid;
         DROP TABLE vector_default;",
    )?;
    // Slice 10 / G10 — recreate directly at the Pack-2 shape (adds `status`), so
    // a legacy single-column DB lands the current shape in one reshape.
    tx.execute_batch(&vector_partition_create_sql(dimension, false, &[]))?;
    // `status` back-fills the empty-string sentinel (vec0 TEXT metadata is NOT
    // NULL-able; reserved-gap candidate 13). Legacy single-column DBs predate
    // mean-centering, so re-quantizing from the un-centered `embedding` is
    // correct here.
    let repopulate_sql = format!(
        "INSERT INTO vector_default(
             rowid, embedding, embedding_bin, source_type, kind, created_at, status
         )
         SELECT
             s.rowid,
             s.embedding,
             vec_quantize_binary(s.embedding),
             {KIND_TO_SOURCE_TYPE_CASE_SQL},
             s.kind,
             strftime('%s', 'now'),
             ''
         FROM _fathomdb_vector_migration_v0_7_0 s;
         DROP TABLE _fathomdb_vector_migration_v0_7_0;"
    );
    tx.execute_batch(&repopulate_sql)?;
    tx.commit()
}

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

pub(crate) fn resolve_source_type(kind: &str) -> Result<&'static str, EngineError> {
    if kind == "edge_fact" {
        // G11 (Slice 15) — edge-body projection; separate `source_type` partition
        // key distinguishes edge vectors from node vectors in `vector_default`.
        return Ok("edge_fact");
    }
    VECTOR_COMMITTABLE_NODE_KIND_SOURCE_TYPES
        .iter()
        .find_map(|(candidate, source_type)| (*candidate == kind).then_some(*source_type))
        .ok_or(EngineError::Storage)
}

/// 0.8.20 Slice 20c fix-2 (codex §9 [P1]) — **can the vector writer COMMIT a row
/// of this kind?** The ONE definition of the vector pipeline's kind domain, shared
/// by every enrolment path.
///
/// [`commit_projection_outcomes`] resolves `kind -> source_type` through
/// [`resolve_source_type`] and returns `Err` for anything outside its locked
/// vocabulary — *before* it records the row's terminal. `PreparedWrite::Node`, by
/// contrast, accepts ANY non-empty `kind` (`validate_write` constrains the body,
/// the identity and the validity window, never the kind against that vocabulary),
/// so a corpus can legitimately hold e.g. an `"invoice"` node.
///
/// Enrolling such a kind is therefore a permanent LIVENESS WEDGE for the whole
/// workspace: the scheduler picks the row up, the commit fails, no terminal is
/// ever written, the scanner re-enqueues it forever, `drain` burns its entire
/// timeout into [`EngineError::Scheduler`] and `dense_readiness` sticks on
/// `embedding` — starving the rows whose kinds ARE commit-able along with it.
///
/// So enrolment is RESTRICTED to this predicate rather than the vector writer
/// being taught arbitrary kinds (which would reach into `resolve_source_type`'s
/// locked Pack-1 partition-key semantics — `dev/design/0.7.0-vector-quant-pack1.md`
/// D3, a HITL lock). A non-commit-able kind simply gets NO dense arm, which is
/// precisely its pre-slice status quo; it is deliberately **not** a new typed
/// error and adds no governed surface.
///
/// It DELEGATES to `resolve_source_type` instead of restating the list. A
/// hand-copied second vocabulary is the TC-56 defect shape (a mirror that silently
/// drifts from its original), and here the drift would be silent in the worst
/// direction: a kind added to `resolve_source_type` but missing from a copied
/// filter would just never be embedded.
pub(crate) fn kind_is_vector_committable(kind: &str) -> bool {
    resolve_source_type(kind).is_ok()
}

impl Engine {
    #[doc(hidden)]
    pub fn write_vector_for_test(
        &self,
        kind: &str,
        text: &str,
    ) -> Result<WriteReceipt, EngineError> {
        self.ensure_open()?;
        let embedder =
            self.runtime_embedder.as_ref().cloned().ok_or(EngineError::EmbedderNotConfigured)?;

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
        let source_type = resolve_source_type(kind)?;
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
        // Slice 10 / G10 — `status` ships an empty-string sentinel only: vec0 TEXT
        // metadata columns are NOT NULL-able ("Expected text for TEXT metadata
        // column"), so the "no real population yet" state is `''`, not NULL.
        //
        // 0.8.20 Slice 15e — this test helper carries no JSON body, so every live
        // `filterable` `attr_<hex>` column binds the `''` sentinel (an empty body
        // extracts nothing). When the table has no attr columns the statement is
        // byte-identical to the shipped form.
        let (cols_sql, ph_sql, attr_vals) =
            vector_attr_insert_fragments(&tx, "", 7).map_err(|_| EngineError::Storage)?;
        let sql = format!(
            "INSERT INTO vector_default(
                rowid, embedding, embedding_bin, source_type, kind, created_at, status{cols_sql}
             ) VALUES(?1, ?2, vec_quantize_binary(?3), ?4, ?5, ?6, ''{ph_sql})"
        );
        let mut pv: Vec<rusqlite::types::Value> = vec![
            rusqlite::types::Value::Integer(cursor as i64),
            rusqlite::types::Value::Blob(blob.clone()),
            rusqlite::types::Value::Blob(bin_blob.clone()),
            rusqlite::types::Value::Text(source_type.to_string()),
            rusqlite::types::Value::Text(kind.to_string()),
            rusqlite::types::Value::Integer(now_unix),
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

    #[doc(hidden)]
    pub fn vector_row_count_for_test(&self) -> Result<u64, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        connection
            .query_row("SELECT COUNT(*) FROM vector_default", [], |row| row.get::<_, u64>(0))
            .map_err(|_| EngineError::Storage)
    }

    /// Test-only distinction between a physical vec0 row and its terminal
    /// readiness record. Closure may terminalize an ineligible projection
    /// without publishing vector bytes, so readiness alone is not a row oracle.
    #[doc(hidden)]
    pub fn has_vector_row_for_cursor_for_test(&self, cursor: u64) -> Result<bool, EngineError> {
        self.ensure_open()?;
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

    #[doc(hidden)]
    pub fn read_vector_blob_for_test(&self, rowid: i64) -> Result<Vec<u8>, EngineError> {
        self.ensure_open()?;
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
    #[doc(hidden)]
    pub fn read_vector_bin_for_test(&self, rowid: i64) -> Result<Vec<u8>, EngineError> {
        self.ensure_open()?;
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
