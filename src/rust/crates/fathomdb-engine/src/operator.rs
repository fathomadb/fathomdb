use super::*;

/// Typed outcome of [`Engine::verify_embedder`]. Mismatches do not raise
/// `EngineError`; the operator workflow needs to see the stored vs.
/// supplied pair to decide on next action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerifyEmbedderStatus {
    Match,
    IdentityMismatch,
    DimensionMismatch,
    BothMismatch,
}

/// Result of [`Engine::verify_embedder`]. `stored_identity` is the
/// `name:revision` pair persisted in `_fathomdb_embedder_profiles`;
/// `supplied_identity` echoes the operator's input verbatim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifyEmbedderReport {
    pub stored_identity: String,
    pub stored_dimension: u32,
    pub supplied_identity: String,
    pub supplied_dimension: u32,
    pub status: VerifyEmbedderStatus,
}

/// Single table or index entry emitted by [`Engine::dump_schema`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaObject {
    pub name: String,
    pub sql: String,
}

/// Result of [`Engine::dump_schema`]. `user_version` is the
/// `PRAGMA user_version` sentinel. Canonical tables appear first per
/// [`fathomdb_schema::CANONICAL_TABLES`], then remaining non-`sqlite_*`
/// tables alphabetically. Indexes follow the same alphabetical rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DumpSchemaReport {
    pub user_version: u32,
    pub tables: Vec<SchemaObject>,
    pub indexes: Vec<SchemaObject>,
}

/// Single canonical-table row count emitted by [`Engine::dump_row_counts`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableRowCount {
    pub name: String,
    pub rows: u64,
}

/// Result of [`Engine::dump_row_counts`]. Canonical tables only;
/// projection / FTS / vec0 shadow tables are excluded. Order matches
/// [`fathomdb_schema::CANONICAL_TABLES`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DumpRowCountsReport {
    pub counts: Vec<TableRowCount>,
}

/// 0.8.20 Slice 5d (R-20-E8) — one `source_id` bucket in an
/// [`OrphanProvenanceReport`]. `source_id` is `None` for the NULL-provenance
/// bucket, which after migration step 21 should contain ONLY governed NODES.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrphanProvenanceSource {
    /// `None` = the NULL-`source_id` bucket.
    pub source_id: Option<String>,
    /// Canonical rows (nodes + edges) carrying this provenance.
    pub rows: u64,
    /// How many of `rows` carry a `logical_id`.
    ///
    /// NOT the same thing as "purge-addressable": only a NODE's `logical_id`
    /// confers purge-addressability. An EDGE's `logical_id` is a supersession
    /// identity and reaches no erasure verb (see
    /// [`Engine::orphan_provenance`]), so governed edges are counted here but
    /// are NOT subtracted from
    /// [`OrphanProvenanceReport::unerasable_rows`].
    pub governed_rows: u64,
    /// True for the engine's reserved `_`-prefixed namespace (`_engine:*`,
    /// `_legacy:pre-0.8.20`). Reserved buckets are reachable only through the
    /// operator seam `excise_source`, never through the governed
    /// [`Engine::erase_source`].
    pub reserved: bool,
}

/// Result of [`Engine::orphan_provenance`] — the per-`source_id` census behind
/// `fathomdb doctor orphan-provenance` (design §4 item 11).
///
/// `unerasable_rows` is the load-bearing field: canonical rows carrying
/// NEITHER a `source_id` NOR a `logical_id`. Such a row is reachable by no
/// erasure verb at all — `purge` keys on `logical_id`, `erase_source` keys on
/// `source_id` — so it can never be deleted on request. Slice 5c made that
/// state unwritable and migration step 21 back-filled the historical cases, so
/// a non-zero count means the invariant has been violated and the verb exits
/// `DOCTOR_FOUND_ISSUES`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrphanProvenanceReport {
    /// Per-`source_id` buckets, ordered by descending `rows` then `source_id`
    /// so the output is deterministic (a diagnostic that reorders between runs
    /// cannot be diffed).
    pub sources: Vec<OrphanProvenanceSource>,
    /// Total canonical rows surveyed.
    pub total_rows: u64,
    /// Rows with NO `source_id` AND NO `logical_id` — un-erasable by any verb.
    pub unerasable_rows: u64,
}

/// Result of [`Engine::dump_profile`]. Mirrors the open-time embedder
/// posture + the per-kind vector configuration registered in
/// `_fathomdb_vector_kinds`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DumpProfileReport {
    pub embedder_identity: String,
    pub embedder_dimension: u32,
    pub vectorized_kinds: Vec<String>,
}

impl Engine {
    /// Doctor `verify-embedder` seam (AC-040a). Compares the
    /// `_fathomdb_embedder_profiles` row to the operator-supplied
    /// `name:revision` identity + dimension; never raises on mismatch.
    #[cfg(feature = "operator")]
    pub fn verify_embedder(
        &self,
        supplied_identity: &str,
        supplied_dimension: u32,
    ) -> Result<VerifyEmbedderReport, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let stored = load_default_profile(connection).map_err(|_| EngineError::Storage)?;
        let stored_identity = format!("{}:{}", stored.name, stored.revision);
        let identity_match = stored_identity == supplied_identity;
        let dimension_match = stored.dimension == supplied_dimension;
        let status = match (identity_match, dimension_match) {
            (true, true) => VerifyEmbedderStatus::Match,
            (false, true) => VerifyEmbedderStatus::IdentityMismatch,
            (true, false) => VerifyEmbedderStatus::DimensionMismatch,
            (false, false) => VerifyEmbedderStatus::BothMismatch,
        };
        Ok(VerifyEmbedderReport {
            stored_identity,
            stored_dimension: stored.dimension,
            supplied_identity: supplied_identity.to_string(),
            supplied_dimension,
            status,
        })
    }

    /// Doctor `dump-schema` seam (AC-040a). Returns the
    /// `PRAGMA user_version` sentinel plus the table + index inventory
    /// from `sqlite_schema`, excluding `sqlite_*` internal rows.
    /// Canonical tables appear first per [`CANONICAL_TABLES`].
    #[cfg(feature = "operator")]
    pub fn dump_schema(&self) -> Result<DumpSchemaReport, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let user_version: u32 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|_| EngineError::Storage)?;
        let tables = read_schema_objects(connection, "table")?;
        let indexes = read_schema_objects(connection, "index")?;
        Ok(DumpSchemaReport { user_version, tables: order_canonical_first(tables), indexes })
    }

    /// Doctor `dump-row-counts` seam (AC-040a). Emits canonical-table
    /// counts only; projection / FTS / vec0 shadow tables are excluded.
    #[cfg(feature = "operator")]
    pub fn dump_row_counts(&self) -> Result<DumpRowCountsReport, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let mut counts = Vec::with_capacity(CANONICAL_TABLES.len());
        for name in CANONICAL_TABLES {
            let rows: u64 = connection
                .query_row(&format!("SELECT COUNT(*) FROM {name}"), [], |row| row.get(0))
                .map_err(|_| EngineError::Storage)?;
            counts.push(TableRowCount { name: (*name).to_string(), rows });
        }
        Ok(DumpRowCountsReport { counts })
    }

    /// 0.8.20 Slice 5d (R-20-E8, design §4 item 11) — doctor
    /// `orphan-provenance` seam: a **read-only** per-`source_id` census over
    /// `canonical_nodes` + `canonical_edges`.
    ///
    /// Answers the operator question the erasure work made askable: *"for this
    /// database, is every row actually reachable by some erasure verb?"* A row
    /// is reachable by `erase_source` / `excise_source` via `source_id`, or —
    /// **if it is a NODE** — by `purge` via `logical_id`. A row with neither is
    /// un-erasable, and is counted into
    /// [`OrphanProvenanceReport::unerasable_rows`].
    ///
    /// The node/edge asymmetry is load-bearing and mirrors migration step 21:
    /// an EDGE's `logical_id` is a supersession identity only and confers no
    /// purge-addressability, so a NULL-`source_id` edge is un-erasable however
    /// governed it looks. See the query comment below.
    ///
    /// CLI-only (no SDK parity), matching the `dump-*` diagnostic family.
    ///
    /// Read-only by construction: this method issues SELECTs exclusively and
    /// opens no transaction.
    #[cfg(feature = "operator")]
    pub fn orphan_provenance(&self) -> Result<OrphanProvenanceReport, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;

        // One UNION ALL over both canonical tables so a source that spans nodes
        // AND edges reports as a single bucket.
        //
        // TWO DIFFERENT SUMS, and the difference is the whole point:
        //
        // * `governed` counts `logical_id` carriers — a reporting figure;
        // * `purge_addressable` counts rows that `purge` can actually reach,
        //   and it is NODE-ONLY (the edge arm contributes a literal 0).
        //
        // This is the same node/edge asymmetry migration step 21 carries, for
        // the same reason, and the two must stay in step: `purge_inner`
        // resolves its target exclusively through `canonical_nodes` (`SELECT
        // state FROM canonical_nodes WHERE logical_id = ?1`) and then erases
        // edges by ENDPOINT (`from_id`/`to_id`). It NEVER resolves an edge by
        // edge `logical_id` — an edge `logical_id` is only a SUPERSESSION
        // identity and confers no purge-addressability whatsoever.
        //
        // Crediting an edge's `logical_id` here made the diagnostic subtract
        // exactly the rows it exists to find: a NULL-`source_id` edge is
        // reachable by no erasure verb at all, yet `orphan-provenance` would
        // exit CLEAN on precisely the legacy/corrupt shape step 21 closes.
        // False assurance from a governance verb is worse than no verb.
        // (codex §9 [P2]; `null_source_governed_edge_counts_as_unerasable`.)
        let mut stmt = connection
            .prepare(
                "SELECT source_id,
                        COUNT(*) AS rows_total,
                        SUM(CASE WHEN logical_id IS NOT NULL THEN 1 ELSE 0 END) AS governed,
                        SUM(purge_addressable) AS purge_addressable
                   FROM (SELECT source_id,
                                logical_id,
                                CASE WHEN logical_id IS NOT NULL THEN 1 ELSE 0 END
                                    AS purge_addressable
                           FROM canonical_nodes
                         UNION ALL
                         SELECT source_id, logical_id, 0 AS purge_addressable
                           FROM canonical_edges)
                  GROUP BY source_id
                  ORDER BY rows_total DESC, source_id",
            )
            .map_err(|_| EngineError::Storage)?;

        let rows = stmt
            .query_map([], |row| {
                let source_id: Option<String> = row.get(0)?;
                let rows: i64 = row.get(1)?;
                let governed: i64 = row.get(2)?;
                let purge_addressable: i64 = row.get(3)?;
                Ok((source_id, rows, governed, purge_addressable))
            })
            .map_err(|_| EngineError::Storage)?;

        let mut sources = Vec::new();
        let mut total_rows: u64 = 0;
        let mut unerasable_rows: u64 = 0;
        for row in rows {
            let (source_id, rows, governed, purge_addressable) =
                row.map_err(|_| EngineError::Storage)?;
            let rows = u64::try_from(rows).unwrap_or(0);
            let governed_rows = u64::try_from(governed).unwrap_or(0);
            let purge_addressable = u64::try_from(purge_addressable).unwrap_or(0);
            total_rows = total_rows.saturating_add(rows);
            if source_id.is_none() {
                // No provenance: only the PURGE-ADDRESSABLE subset (governed
                // NODES) is reachable. The remainder — including every governed
                // EDGE, whose `logical_id` reaches nothing — is reachable by no
                // erasure verb at all.
                unerasable_rows =
                    unerasable_rows.saturating_add(rows - purge_addressable.min(rows));
            }
            let reserved = source_id.as_deref().is_some_and(|s| s.starts_with('_'));
            sources.push(OrphanProvenanceSource { source_id, rows, governed_rows, reserved });
        }

        Ok(OrphanProvenanceReport { sources, total_rows, unerasable_rows })
    }

    /// Doctor `dump-profile` seam (AC-040a). Returns the stored
    /// embedder identity + dimension plus the registered vectorized
    /// kinds from `_fathomdb_vector_kinds`.
    #[cfg(feature = "operator")]
    pub fn dump_profile(&self) -> Result<DumpProfileReport, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let stored = load_default_profile(connection).map_err(|_| EngineError::Storage)?;
        let mut stmt = connection
            .prepare("SELECT kind FROM _fathomdb_vector_kinds ORDER BY kind")
            .map_err(|_| EngineError::Storage)?;
        let rows =
            stmt.query_map([], |row| row.get::<_, String>(0)).map_err(|_| EngineError::Storage)?;
        let mut vectorized_kinds = Vec::new();
        for row in rows {
            vectorized_kinds.push(row.map_err(|_| EngineError::Storage)?);
        }
        Ok(DumpProfileReport {
            embedder_identity: format!("{}:{}", stored.name, stored.revision),
            embedder_dimension: stored.dimension,
            vectorized_kinds,
        })
    }
}

#[cfg(feature = "operator")]
fn read_schema_objects(
    connection: &Connection,
    obj_type: &str,
) -> Result<Vec<SchemaObject>, EngineError> {
    let mut stmt = connection
        .prepare(
            "SELECT name, sql FROM sqlite_schema
             WHERE type = ?1 AND name NOT LIKE 'sqlite_%' AND sql IS NOT NULL
             ORDER BY name",
        )
        .map_err(|_| EngineError::Storage)?;
    let rows = stmt
        .query_map([obj_type], |row| {
            Ok(SchemaObject { name: row.get::<_, String>(0)?, sql: row.get::<_, String>(1)? })
        })
        .map_err(|_| EngineError::Storage)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|_| EngineError::Storage)?);
    }
    Ok(out)
}

#[cfg(feature = "operator")]
fn order_canonical_first(mut objects: Vec<SchemaObject>) -> Vec<SchemaObject> {
    let mut canonical: Vec<SchemaObject> = Vec::new();
    for name in CANONICAL_TABLES {
        if let Some(pos) = objects.iter().position(|o| o.name == *name) {
            canonical.push(objects.remove(pos));
        }
    }
    canonical.extend(objects);
    canonical
}
