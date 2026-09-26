use super::*;

/// 0.8.20 Slice 5a (R-20-E1) — the class a row-owned projection table belongs
/// to, so the four maintenance sites can each truncate exactly the subset they
/// own without re-deriving a hand-rolled table list.
///
/// - `NodeFts` — same-txn lexical projection of a canonical NODE body.
/// - `EdgeFts` — same-txn lexical projection of a canonical EDGE body.
/// - `Vector` — the async vec0 materialization (written by the embed worker,
///   not by the write path — see [`project_canonical_node_row`]).
/// - `Readiness` — the terminal-cursor bookkeeping that lets
///   `advance_projection_cursor` walk past a row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProjectionClass {
    NodeFts,
    EdgeFts,
    Vector,
    Readiness,
    /// 0.8.20 Slice 15d (R-20-EAV) — the EAV attribute store (`filterable` +
    /// the value-at-rest for `searchable`). Same-transaction, row-owned.
    Attribute,
    /// 0.8.20 Slice 15d (R-20-EAV) — the property-FTS5 shadow of attribute
    /// values (`searchable→FTS`). Same-transaction, row-owned.
    PropertyFts,
}

/// 0.8.20 Slice 5a (R-20-E1) — one ROW-OWNED projection table: a shadow whose
/// rows are 1:1 with a canonical row's `write_cursor` and therefore MUST die
/// with that row.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RowOwnedProjection {
    /// Table name. `'static` and never caller-derived: safe to interpolate.
    pub(crate) table: &'static str,
    /// The column carrying the owning canonical row's `write_cursor`. For the
    /// vec0 table this is `rowid` — vec0 rowid IS the write_cursor (see the
    /// `_fathomdb_vector_rows.write_cursor UNIQUE` identity).
    pub(crate) cursor_column: &'static str,
    class: ProjectionClass,
}

/// 0.8.20 Slice 5a (R-20-E1) — **the** registry of row-owned projections.
///
/// Every table here is 1:1 with a canonical `write_cursor` and is erased by
/// [`erase_row_projections`] whenever that canonical row is erased. Adding a
/// projection table WITHOUT registering it here re-opens the defect this slice
/// closes (`search_index_v2` was written by one site and deleted by one site,
/// out of five that maintain projections — so `excise_source` left the erased
/// body on disk in a content-storing FTS5 table). The `guard_row_owned_registry`
/// unit test introspects `sqlite_master` and fails if a `write_cursor`-keyed
/// table is missing from this list.
///
/// **NOT here, deliberately (design v5 §1.1): `_fathomdb_projection_state`.**
/// That table is KIND-owned — keyed by `kind`, holding a per-kind enqueue
/// watermark. Erasing one row must NOT rewind a whole kind's watermark, so it
/// must never be deleted per-cursor. A rebuild resets it deliberately; erasure
/// leaves it alone.
pub(crate) const ROW_OWNED_PROJECTIONS: &[RowOwnedProjection] = &[
    RowOwnedProjection {
        table: "search_index",
        cursor_column: "write_cursor",
        class: ProjectionClass::NodeFts,
    },
    RowOwnedProjection {
        table: "search_index_v2",
        cursor_column: "write_cursor",
        class: ProjectionClass::NodeFts,
    },
    RowOwnedProjection {
        table: "search_index_edges",
        cursor_column: "write_cursor",
        class: ProjectionClass::EdgeFts,
    },
    RowOwnedProjection {
        table: "vector_default",
        cursor_column: "rowid",
        class: ProjectionClass::Vector,
    },
    RowOwnedProjection {
        table: "_fathomdb_vector_rows",
        cursor_column: "write_cursor",
        class: ProjectionClass::Vector,
    },
    RowOwnedProjection {
        table: "_fathomdb_projection_terminal",
        cursor_column: "write_cursor",
        class: ProjectionClass::Readiness,
    },
    // 0.8.20 Slice 15d (R-20-EAV) — the EAV attribute store and its property-FTS
    // shadow both hold declared attribute VALUES at rest (potential PII), keyed
    // 1:1 with the owning node's write_cursor. They MUST be reachable by
    // `purge`/`excise_source`: registering them here is what makes
    // `erase_row_projections` delete them without a hand-rolled list (an
    // unregistered content-storing table is exactly the `search_index_v2` leak
    // class this registry closes). The `guard_row_owned_registry` unit test
    // FAILS if either is left unregistered.
    RowOwnedProjection {
        table: "canonical_attributes",
        cursor_column: "write_cursor",
        class: ProjectionClass::Attribute,
    },
    RowOwnedProjection {
        table: "property_search_index",
        cursor_column: "write_cursor",
        class: ProjectionClass::PropertyFts,
    },
];

/// 0.8.20 Slice 5a (R-20-E1) — erase EVERY row-owned projection for one
/// canonical `write_cursor`. Returns the number of shadow rows deleted.
///
/// This is the single erasure primitive: `purge_inner` and `excise_source_inner`
/// both call it, so a new projection table becomes erasable by registering it in
/// [`ROW_OWNED_PROJECTIONS`] — not by remembering to patch two hand-rolled
/// delete lists (the omission that left erased bodies in `search_index_v2`).
pub(crate) fn erase_row_projections(tx: &Connection, write_cursor: i64) -> rusqlite::Result<u64> {
    let mut deleted: u64 = 0;
    for projection in ROW_OWNED_PROJECTIONS {
        deleted =
            saturating_add_u64(deleted, delete_row_owned_projection(tx, projection, write_cursor)?);
    }
    Ok(deleted)
}

/// TC-76 — delete one row-owned projection's rows for one `write_cursor`.
/// The vec0 partition is routed through the shared direct-delete primitive;
/// every other table is the plain registry-driven statement.
fn delete_row_owned_projection(
    tx: &Connection,
    projection: &RowOwnedProjection,
    write_cursor: i64,
) -> rusqlite::Result<usize> {
    if projection.table == DEFAULT_VECTOR_PARTITION {
        return delete_vector_partition_row(tx, write_cursor);
    }
    let sql = format!("DELETE FROM {} WHERE {} = ?1", projection.table, projection.cursor_column);
    tx.execute(&sql, [write_cursor])
}

fn saturating_add_u64(acc: u64, n: usize) -> u64 {
    acc.saturating_add(n as u64)
}

/// 0.8.20 Slice 15d fix-1 finding 2 [P2] — purge the row-owned projections in
/// `classes` for ONE canonical `write_cursor`. Same registry-driven mechanism as
/// [`erase_row_projections`] (iterate [`ROW_OWNED_PROJECTIONS`], delete by the
/// declared cursor column) but scoped to a class SUBSET, so the write path can
/// drop a SUPERSEDED node's `Attribute` + `PropertyFts` rows — making the at-rest
/// property projection active-only — WITHOUT touching the `NodeFts`/`Vector`
/// shadows, whose stale rows the node read path already excludes by joining
/// `canonical_nodes WHERE superseded_at IS NULL`. Consistent with the erasure
/// model: an unregistered table is unreachable here, exactly as with erasure.
pub(crate) fn purge_row_projections_for_cursor_in(
    tx: &Connection,
    write_cursor: i64,
    classes: &[ProjectionClass],
) -> rusqlite::Result<u64> {
    let mut deleted: u64 = 0;
    for projection in ROW_OWNED_PROJECTIONS.iter().filter(|p| classes.contains(&p.class)) {
        deleted =
            saturating_add_u64(deleted, delete_row_owned_projection(tx, projection, write_cursor)?);
    }
    Ok(deleted)
}

/// 0.8.20 Slice 5a (R-20-E1) — truncate the row-owned projections in `classes`.
/// Returns the number of shadow rows deleted.
pub(crate) fn truncate_row_projections_in(
    tx: &Connection,
    classes: &[ProjectionClass],
) -> rusqlite::Result<u64> {
    let mut deleted: u64 = 0;
    for projection in ROW_OWNED_PROJECTIONS.iter().filter(|p| classes.contains(&p.class)) {
        let sql = format!("DELETE FROM {}", projection.table);
        deleted = deleted.saturating_add(tx.execute(&sql, [])? as u64);
    }
    Ok(deleted)
}

/// 0.8.20 Slice 5a (R-20-E1) — truncate EVERY row-owned projection (the full
/// `rebuild_projections` invalidation). Kind-owned watermark state
/// (`_fathomdb_projection_state`) is deliberately untouched; the rebuild resets
/// readiness by rewinding the projection cursor instead.
#[cfg(feature = "operator")]
pub(crate) fn truncate_all_row_projections(tx: &Connection) -> rusqlite::Result<u64> {
    truncate_row_projections_in(
        tx,
        &[
            ProjectionClass::NodeFts,
            ProjectionClass::EdgeFts,
            ProjectionClass::Vector,
            ProjectionClass::Readiness,
            ProjectionClass::Attribute,
            ProjectionClass::PropertyFts,
        ],
    )
}

/// 0.8.20 Slice 5a (R-20-E1) — which half of a projector's work a call site
/// wants. The projectors are TOTAL (they own every row-owned projection for a
/// canonical row); the pass selects the subset a replay site is rebuilding, so
/// no call site re-implements projection SQL inline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProjectionPass {
    /// The write path: same-txn FTS **and** async vector enqueue / readiness
    /// termination.
    Write,
    /// Lexical replay only (the open-path tokenizer reproject). Readiness and
    /// vector state are already correct and must not be perturbed.
    FtsOnly,
    /// Readiness + async-vector replay only (`rebuild_vec0`, i.e. a rebuild with
    /// `include_fts = false`): the FTS shadows are not being rebuilt in this
    /// pass, so they must not be written.
    ///
    /// Only the `operator` rebuild seam constructs this pass, so the DEFAULT
    /// (recovery-clean) build sees it as unconstructed — same gate rationale as
    /// the operator methods themselves (feature = gate, not delete).
    #[cfg_attr(not(feature = "operator"), allow(dead_code))]
    VectorOnly,
}

impl ProjectionPass {
    pub(crate) fn writes_fts(self) -> bool {
        matches!(self, ProjectionPass::Write | ProjectionPass::FtsOnly)
    }

    pub(crate) fn writes_vector_state(self) -> bool {
        matches!(self, ProjectionPass::Write | ProjectionPass::VectorOnly)
    }

    /// 0.8.20 Slice 15d (R-20-EAV) — whether this pass (re)projects the declared
    /// attribute set into the EAV store + property-FTS. Only the full `Write`
    /// pass does: the `FtsOnly` tokenizer-upgrade reproject predates step 24 (no
    /// registry/attribute tables exist at that migration point, so it must not
    /// touch them), and `VectorOnly` rebuilds only the async vector shadows. The
    /// operator FTS rebuild uses `Write`, so a full `rebuild_projections`
    /// re-derives attributes cleanly after `truncate_all_row_projections` clears
    /// the two attribute classes.
    pub(crate) fn writes_attributes(self) -> bool {
        matches!(self, ProjectionPass::Write)
    }
}

/// 0.8.20 Slice 15d (R-20-PR) — the on-disk registry row for one declared
/// projection, read back from `_fathomdb_projection_registry`.
///
/// **On-disk encoding of the optional sub-objects.** The `fts_tokenizer` column
/// is tri-valued: SQL `NULL` = no `fts` sub-object; empty string `""` = `fts`
/// present with the engine-default tokenizer; a non-empty string = `fts` with a
/// custom tokenizer. This is what lets `searchable→FTS with default tokenizer`
/// be distinguished durably from `searchable` with no FTS sub-target. `vector`
/// mirrors it with an explicit `vector_declared` bit plus a nullable
/// `vector_embedder`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StoredProjection {
    pub(crate) roles: BTreeSet<ProjectionRole>,
    pub(crate) fts_present: bool,
    /// `Some(custom)` custom tokenizer; `None` = engine default (only
    /// meaningful when `fts_present`).
    pub(crate) fts_tokenizer: Option<String>,
    pub(crate) vector_declared: bool,
    pub(crate) vector_embedder: Option<String>,
    pub(crate) source: Option<Vec<String>>,
}

impl StoredProjection {
    /// True iff the declared roles want the attribute VALUE stored at rest in
    /// the EAV store: `filterable` (the value IS the filter target) or
    /// `searchable` (the value is the retrievable meaning, and Slice 20's vector
    /// embed will read it from here). `rankable`-only wants no value at rest.
    pub(crate) fn wants_eav(&self) -> bool {
        self.roles.contains(&ProjectionRole::Filterable)
            || self.roles.contains(&ProjectionRole::Searchable)
    }

    /// True iff a `searchable→FTS` property-FTS row should be written: the
    /// `searchable` role AND an `fts` sub-object.
    pub(crate) fn wants_property_fts(&self) -> bool {
        self.roles.contains(&ProjectionRole::Searchable) && self.fts_present
    }

    /// 0.8.20 Slice 21c (ledger `TC-71`) — **THE `searchable→vector` predicate.**
    /// True iff this declaration puts the attribute on the dense arm: the
    /// `searchable` role AND a `vector` sub-object. The exact analogue of
    /// [`StoredProjection::wants_property_fts`], for the same reason — the
    /// sub-object SELECTS a sub-target of `searchable`; it does not confer one.
    ///
    /// [`vector_projection_declared`] — the corpus-wide predicate gating all
    /// three enrolment paths (declare-time backfill, its drop inverse, and the
    /// write path's late enrolment) — routes through this so no call site can
    /// re-derive the rule and drift. Before it existed, that predicate keyed off
    /// `vector_declared` ALONE, so `{roles: [filterable], vector: {}}` — the
    /// combination Slice 15d documented as inert-but-round-trippable — enrolled
    /// node kinds, backfilled the corpus and made every later write enqueue an
    /// embedding in any session with a live embedder.
    ///
    /// **0.8.20 Slice 23 (`R-20-SV`):** that combination is no longer DECLARABLE
    /// — [`apply_projection_config`] rejects it as an invalid spec. This
    /// predicate still governs, because the shape survives at rest in every
    /// database that declared it while the engine accepted it, and it is read
    /// from the registry, not from a caller's spec.
    ///
    /// **Deliberately NOT the same as [`StoredProjection::has_deferred`]**, which
    /// keys off `vector_declared` alone and must keep doing so: that one feeds
    /// `ProjectionDelta.deferred`, a REPORTING field, and the round-trip contract
    /// wants a stored-but-unbuilt `vector` sub-object reported however it was
    /// declared. TC-71 changes what the engine DOES, not what it says.
    pub(crate) fn wants_vector(&self) -> bool {
        self.roles.contains(&ProjectionRole::Searchable) && self.vector_declared
    }

    /// The `fts_tokenizer` column value: `None` (SQL NULL) when no `fts`
    /// sub-object, else the custom tokenizer or `""` for engine-default.
    fn fts_column(&self) -> Option<String> {
        if self.fts_present {
            Some(self.fts_tokenizer.clone().unwrap_or_default())
        } else {
            None
        }
    }

    /// Build from the public [`ProjectionSpec`].
    ///
    /// 0.8.20 Slice 20 (R-20-DR) — note what is DELIBERATELY not read here:
    /// `spec.vector.dense_readiness`. Readiness is engine-set READ METADATA, not
    /// part of the declaration, so it never reaches the durable registry. That
    /// is what makes a caller-supplied value INERT (the engine always reports the
    /// derived truth) and what keeps it out of the destructive-change diff — a
    /// readiness difference can never look like a projection change.
    pub(crate) fn from_spec(spec: &ProjectionSpec) -> Self {
        StoredProjection {
            roles: spec.roles.clone(),
            fts_present: spec.fts.is_some(),
            fts_tokenizer: spec
                .fts
                .as_ref()
                .and_then(|f| f.tokenizer.clone())
                .filter(|t| !t.is_empty()),
            vector_declared: spec.vector.is_some(),
            vector_embedder: spec
                .vector
                .as_ref()
                .and_then(|v| v.embedder.clone())
                .filter(|e| !e.is_empty()),
            source: spec.source.clone(),
        }
    }

    /// Reconstruct the public [`ProjectionSpec`] for `read_projections`.
    pub(crate) fn to_spec(&self, name: &str) -> ProjectionSpec {
        ProjectionSpec {
            name: name.to_string(),
            roles: self.roles.clone(),
            fts: if self.fts_present {
                Some(ProjectionFts { tokenizer: self.fts_tokenizer.clone() })
            } else {
                None
            },
            vector: if self.vector_declared {
                // 0.8.20 Slice 20 (R-20-DR) — the registry knows nothing about
                // readiness (it is DERIVED, never stored), so the durable shape
                // reconstructs with `dense_readiness: None`.
                // [`Engine::read_projections`] fills it from
                // [`derive_dense_readiness`] on the way out.
                Some(ProjectionVector {
                    embedder: self.vector_embedder.clone(),
                    dense_readiness: None,
                })
            } else {
                None
            },
            source: self.source.clone(),
        }
    }

    /// The set of ROLE spellings this declaration DEFERS rather than builds:
    /// `rankable` (F9 not live) and, since 15d builds no embedding, the
    /// `searchable→vector` sub-target. Used to populate `ProjectionDelta.deferred`.
    pub(crate) fn has_deferred(&self) -> bool {
        self.roles.contains(&ProjectionRole::Rankable) || self.vector_declared
    }
}

/// 0.8.20 Slice 15d (R-20-PR) — is `name` a well-formed attribute name?
///
/// Establishes the invariant "a name that `configure_projections` ACCEPTS must be
/// POPULATABLE": the write-path extraction compiles the SQLite JSON path
/// `$."<name>"` (double-quoted key). A name must therefore round-trip through
/// that quoted-key form unchanged. Rejects:
///   - empty;
///   - a double-quote `"` (would terminate the quoted key early → malformed path,
///     ERRORing inside the write transaction);
///   - a BACKSLASH `\` (fix-4 finding 1 [P2]): SQLite treats `\` as an escape
///     introducer inside the double-quoted JSON-path key, so a body key literally
///     containing `\` (e.g. `a\b`) is NOT matched by `$."a\b"`. Pre-fix the name
///     was accepted yet the attribute silently NEVER populated
///     `canonical_attributes` — an accept-then-never-populate footgun. Rejecting
///     it keeps the accept ⟹ works contract (mirrors the TC-33 hard-reject
///     philosophy);
///   - any ASCII control char (incl. NUL): not a safe/legible key spelling and
///     not reliably matchable through the quoted-key form.
///
/// Projection names are app-declared identifiers, so this charset restriction is
/// a legitimate contract. Caller-supplied, so it is validated at
/// `configure_projections` time (spec names AND the `drop` list).
pub(crate) fn is_valid_attribute_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('"')
        && !name.contains('\\')
        && !name.chars().any(|c| c.is_control())
}

/// 0.8.20 Slice 15d — the SQLite JSON path that extracts attribute `name` from a
/// node body. `name` is pre-validated by [`is_valid_attribute_name`]; the path
/// is bound as a PARAMETER (never interpolated into SQL), so this is not an
/// injection surface even before that validation.
fn attribute_json_path(name: &str) -> String {
    format!("$.\"{name}\"")
}

/// SQLite JSON path for one declared projection. A declared source is an ordered
/// list of literal object-member names; it is never a caller-provided JSONPath.
/// Every segment is validated before persistence, and the resulting path is
/// always bound as a parameter rather than interpolated into SQL.
pub(crate) fn projection_json_path(name: &str, stored: &StoredProjection) -> String {
    match &stored.source {
        None => attribute_json_path(name),
        Some(segments) => {
            let mut path = String::from("$");
            for segment in segments {
                path.push_str(".\"");
                path.push_str(segment);
                path.push('"');
            }
            path
        }
    }
}

/// Refuse a source path that cannot be safely represented as SQLite quoted
/// member selectors. Empty paths would select the whole body rather than one
/// member and therefore do not meet the scalar-projection contract.
pub(crate) fn is_valid_projection_source(source: &[String]) -> bool {
    !source.is_empty() && source.iter().all(|segment| is_valid_attribute_name(segment))
}

/// Reject only nested-source object/array terminals. Legacy top-level
/// projections retain their shipped skip-composite behaviour for compatibility.
fn nested_projection_terminal_is_composite(
    conn: &Connection,
    body: &str,
    name: &str,
    stored: &StoredProjection,
) -> rusqlite::Result<bool> {
    if stored.source.is_none() {
        return Ok(false);
    }
    let path = projection_json_path(name, stored);
    let terminal: Option<String> = conn.query_row(
        "SELECT CASE WHEN json_valid(?1) THEN json_type(?1, ?2) END",
        params![body, path],
        |row| row.get(0),
    )?;
    Ok(matches!(terminal.as_deref(), Some("object") | Some("array")))
}

/// Validate nested source terminals across the active rows a declaration would
/// backfill. The caller owns the transaction, so `WriteValidation` aborts it
/// rather than leaving a partly reconfigured registry.
pub(crate) fn validate_projection_source_backfill(
    conn: &Connection,
    name: &str,
    stored: &StoredProjection,
) -> Result<(), EngineError> {
    if stored.source.is_none() {
        return Ok(());
    }
    let mut stmt = conn
        .prepare(
            "SELECT body FROM canonical_nodes
             WHERE superseded_at IS NULL AND state = 'active'",
        )
        .map_err(|_| EngineError::Storage)?;
    let bodies =
        stmt.query_map([], |row| row.get::<_, String>(0)).map_err(|_| EngineError::Storage)?;
    for body in bodies {
        let body = body.map_err(|_| EngineError::Storage)?;
        if nested_projection_terminal_is_composite(conn, &body, name, stored)
            .map_err(|_| EngineError::Storage)?
        {
            return Err(EngineError::WriteValidation);
        }
    }
    Ok(())
}

/// Validate every nested source present in a normal node write before the write
/// transaction starts. This keeps an object/array terminal in the existing
/// `WriteValidation` family and guarantees the whole batch is rejected.
pub(crate) fn validate_nested_projection_sources_for_write(
    conn: &Connection,
    batch: &[PreparedWrite],
) -> Result<(), EngineError> {
    let registry = load_projection_registry(conn).map_err(|_| EngineError::Storage)?;
    if registry.values().all(|stored| stored.source.is_none()) {
        return Ok(());
    }
    for write in batch {
        let write = storage_write_shape(write);
        let PreparedWrite::Node { body, .. } = write.as_ref() else { continue };
        validate_nested_projection_sources_for_body_in_registry(conn, body, &registry)?;
    }
    Ok(())
}

pub(crate) fn validate_nested_projection_sources_for_body(
    conn: &Connection,
    body: &str,
) -> Result<(), EngineError> {
    let registry = load_projection_registry(conn).map_err(|_| EngineError::Storage)?;
    validate_nested_projection_sources_for_body_in_registry(conn, body, &registry)
}

/// Validate one body against a registry snapshot owned by the caller. Batch
/// writes load this snapshot once, keeping validation proportional to bodies
/// plus declarations rather than repeating registry I/O for every row.
fn validate_nested_projection_sources_for_body_in_registry(
    conn: &Connection,
    body: &str,
    registry: &BTreeMap<String, StoredProjection>,
) -> Result<(), EngineError> {
    for (name, stored) in registry {
        if nested_projection_terminal_is_composite(conn, body, name, stored)
            .map_err(|_| EngineError::Storage)?
        {
            return Err(EngineError::WriteValidation);
        }
    }
    Ok(())
}

/// 0.8.20 Slice 15d (R-20-PR) — load the durable projection registry
/// (`_fathomdb_projection_registry`) into a name→[`StoredProjection`] map. This
/// is the derived-cache source (Q5) that boot re-derive and every
/// `configure_projections` diff read.
pub(crate) fn load_projection_registry(
    conn: &Connection,
) -> rusqlite::Result<BTreeMap<String, StoredProjection>> {
    let mut out = BTreeMap::new();
    // The registry table is created by schema step 24; a DB migrated to a
    // pre-24 head (e.g. a compatibility/partial-migration test open) does not
    // have it. Absent ⇒ no projections declared ⇒ empty registry, not an error.
    // This keeps boot re-derive and the write-path attribute projector safe on
    // every pre-24 schema.
    let table_exists: bool = conn
        .prepare_cached(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '_fathomdb_projection_registry'",
        )?
        .query_row([], |_| Ok(true))
        .optional()?
        .unwrap_or(false);
    if !table_exists {
        return Ok(out);
    }
    let mut stmt = conn.prepare_cached(
        "SELECT name, roles, fts_tokenizer, vector_embedder, vector_declared, source
         FROM _fathomdb_projection_registry",
    )?;
    let rows = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let roles_json: String = row.get(1)?;
        let fts_tokenizer: Option<String> = row.get(2)?;
        let vector_embedder: Option<String> = row.get(3)?;
        let vector_declared: i64 = row.get(4)?;
        let source_json: Option<String> = row.get(5)?;
        Ok((name, roles_json, fts_tokenizer, vector_embedder, vector_declared, source_json))
    })?;
    for row in rows {
        let (name, roles_json, fts_col, vector_embedder, vector_declared, source_json) = row?;
        let roles: BTreeSet<ProjectionRole> = parse_roles_json(&roles_json);
        let fts_present = fts_col.is_some();
        let fts_tokenizer = fts_col.filter(|t| !t.is_empty());
        let source = source_json
            .map(|encoded| serde_json::from_str::<Vec<String>>(&encoded))
            .transpose()
            .map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;
        out.insert(
            name,
            StoredProjection {
                roles,
                fts_present,
                fts_tokenizer,
                vector_declared: vector_declared != 0,
                vector_embedder,
                source,
            },
        );
    }
    Ok(out)
}

#[cfg(feature = "operator")]
pub(crate) fn load_projection_registry_row(
    conn: &Connection,
    name: &str,
) -> rusqlite::Result<Option<StoredProjection>> {
    conn.query_row(
        "SELECT roles,fts_tokenizer,vector_embedder,vector_declared,source \
         FROM _fathomdb_projection_registry WHERE name=?1",
        [name],
        |row| {
            let roles_json: String = row.get(0)?;
            let fts_column: Option<String> = row.get(1)?;
            let vector_embedder: Option<String> = row.get(2)?;
            let vector_declared: i64 = row.get(3)?;
            let source_json: Option<String> = row.get(4)?;
            let source = source_json
                .map(|encoded| serde_json::from_str::<Vec<String>>(&encoded))
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            Ok(StoredProjection {
                roles: parse_roles_json(&roles_json),
                fts_present: fts_column.is_some(),
                fts_tokenizer: fts_column.filter(|value| !value.is_empty()),
                vector_declared: vector_declared != 0,
                vector_embedder,
                source,
            })
        },
    )
    .optional()
}

/// Roles are persisted as a compact, sorted, comma-separated list (set
/// semantics; order-independent). Unknown tokens are ignored (forward-compat).
fn parse_roles_json(s: &str) -> BTreeSet<ProjectionRole> {
    s.split(',').filter_map(|t| ProjectionRole::from_str_opt(t.trim())).collect()
}

fn roles_to_storage(roles: &BTreeSet<ProjectionRole>) -> String {
    roles.iter().map(|r| r.as_str()).collect::<Vec<_>>().join(",")
}

/// 0.8.20 Slice 15d (R-20-PR) — write/overwrite one registry row.
pub(crate) fn persist_projection_row(
    tx: &Connection,
    name: &str,
    stored: &StoredProjection,
) -> rusqlite::Result<()> {
    let source = stored
        .source
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
    tx.execute(
        "INSERT INTO _fathomdb_projection_registry
             (name, roles, fts_tokenizer, vector_embedder, vector_declared, source)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(name) DO UPDATE SET
             roles = excluded.roles,
             fts_tokenizer = excluded.fts_tokenizer,
             vector_embedder = excluded.vector_embedder,
             vector_declared = excluded.vector_declared,
             source = excluded.source",
        params![
            name,
            roles_to_storage(&stored.roles),
            stored.fts_column(),
            stored.vector_embedder,
            i64::from(stored.vector_declared),
            source,
        ],
    )?;
    Ok(())
}

/// 0.8.20 Slice 15d (R-20-PR) — delete one registry row.
pub(crate) fn remove_projection_row(tx: &Connection, name: &str) -> rusqlite::Result<()> {
    tx.execute("DELETE FROM _fathomdb_projection_registry WHERE name = ?1", params![name])?;
    Ok(())
}
