use super::*;
use crate::vector_storage::{DEFAULT_VECTOR_PARTITION, DEFAULT_VECTOR_PROFILE};

/// 0.8.20 Slice 15d (R-20-PR, C-1) — one member of a [`ProjectionSpec`]'s role
/// set. **Exactly three members** (HITL-ratified S8, `api-surface.md:87`):
/// `searchable→FTS` and `searchable→vector` are NOT roles — they are tier labels
/// carried by the `fts`/`vector` sub-objects of the spec, so an attribute is
/// `Searchable` once and the sub-objects select FTS-only / vector-only / both.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ProjectionRole {
    /// Projects into the EAV store + its `(attr_name, attr_value)` composite
    /// index — cheap equality/range, built same-transaction.
    Filterable,
    /// The F9 importance/recency signal. **Graceful-absent (Q6a):** declaring
    /// it is legal and never errors, but the engine DEFERS the build until F9
    /// exists and grafts it on the next idempotent `configure_projections`.
    Rankable,
    /// Full-text / dense recall of the meaning text. The `fts`/`vector`
    /// sub-objects select the sub-target.
    Searchable,
}

impl ProjectionRole {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectionRole::Filterable => "filterable",
            ProjectionRole::Rankable => "rankable",
            ProjectionRole::Searchable => "searchable",
        }
    }

    #[must_use]
    pub fn from_str_opt(value: &str) -> Option<Self> {
        match value {
            "filterable" => Some(ProjectionRole::Filterable),
            "rankable" => Some(ProjectionRole::Rankable),
            "searchable" => Some(ProjectionRole::Searchable),
            _ => None,
        }
    }
}

/// 0.8.20 Slice 15d (R-20-PR) — the `searchable→FTS` sub-target selector.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectionFts {
    /// Optional tokenizer override; `None` ⇒ the engine default FTS5 tokenizer
    /// (`body`-FTS's `porter unicode61 remove_diacritics 2`). A custom
    /// per-attr tokenizer is the ≥0.9.x multi-field FTS work — recorded but
    /// not honoured here (graceful-graft later, same as `rankable`).
    pub tokenizer: Option<String>,
}

/// 0.8.20 Slice 20 (R-20-DR) — the ENGINE-SET readiness of the
/// `searchable→vector` projection, per
/// `dev/design/record-lifecycle-protocol/projection-registry-and-async-embed.md`
/// §3.
///
/// **Exactly three members.** `filterable` and `searchable→FTS` are
/// same-transaction (non-stale on commit) so they need no readiness axis at all;
/// `searchable→vector` is **async, rebuild-durable**, so it carries one.
///
/// **Naming discipline (load-bearing).** The token **`pending` is RESERVED for
/// the admission axis** (quarantine/trust — an app judgment). Index-readiness is
/// a DIFFERENT, orthogonal dimension (a record can be
/// `active ∧ is_latest ∧ admissible` yet `dense_readiness = embedding`), so this
/// enum deliberately does **not** reuse that word: the non-ready member is
/// `Embedding`, never `Pending`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DenseReadiness {
    /// Engine-selected state for a session with no usable dense runtime (an
    /// absent embedder or refused vector equivalence). Caller input remains
    /// accept-inert; reads select this through the shared runtime predicate.
    Unavailable,
    /// At least one row in the vector projection's row set has not yet reached a
    /// projection terminal — embedding is outstanding. This is the ONLY
    /// tolerable torn state: readiness `embedding` with the vector absent (the
    /// dense arm reads as partial and RRF under-ranks; it does not hide).
    Embedding,
    /// Every row in the vector projection's row set has reached a projection
    /// terminal — the dense arm is caught up. Because the vector INSERT and the
    /// terminal record are written in ONE transaction
    /// ([`commit_projection_outcomes`]), `Ready` can never be observed with the
    /// vector row absent (design §4.1 invariant 1).
    Ready,
}

impl DenseReadiness {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            DenseReadiness::Unavailable => "unavailable",
            DenseReadiness::Embedding => "embedding",
            DenseReadiness::Ready => "ready",
        }
    }

    /// The three accepted spellings. `"pending"` is DELIBERATELY not one of them
    /// (reserved for the admission axis) and so parses to `None`.
    #[must_use]
    pub fn from_str_opt(value: &str) -> Option<Self> {
        match value {
            "unavailable" => Some(DenseReadiness::Unavailable),
            "embedding" => Some(DenseReadiness::Embedding),
            "ready" => Some(DenseReadiness::Ready),
            _ => None,
        }
    }
}

/// 0.8.20 Slice 15d (R-20-PR) — the `searchable→vector` sub-target selector.
///
/// **Slice 20 (R-20-DR) attached `dense_readiness` HERE, additively:** this
/// sub-object is STORED by 15d (so the shape exists and a caller can declare a
/// vector projection); Slice 20 hangs the READ-METADATA readiness flag off it.
/// Nothing in 15d's persisted shape changed (the registry columns
/// `vector_embedder` + `vector_declared` still round-trip the declaration) —
/// **readiness is DERIVED, never stored**, so there is no schema step and no
/// separate flag that could tear (see [`derive_dense_readiness`]).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectionVector {
    /// Optional embedder override; `None` ⇒ the engine's shipped default.
    pub embedder: Option<String>,
    /// 0.8.20 Slice 20 (R-20-DR) — **READ METADATA, engine-set.** Populated by
    /// [`Engine::read_projections`]; `None` on every caller-authored spec.
    ///
    /// It is **not part of the declaration**: `configure_projections` neither
    /// stores nor honours it (see [`StoredProjection::from_spec`], which reads
    /// only `embedder`), so a value supplied here is INERT — the engine always
    /// reports the derived truth. This is deliberately accept-inert rather than
    /// hard-reject so `read.projections` output stays feedable straight back
    /// into `configure_projections` (the fix-4 read→configure round-trip, which
    /// both bindings pin with a test).
    ///
    /// **0.8.20 Slice 23 (`R-20-SV`) correction (TC-39 class).** This doc used to
    /// justify accept-inert by analogy with "the already-audited accept-inert
    /// ruling on an `fts`/`vector` sub-object declared without the `searchable`
    /// role". **That ruling is OVERRULED** — the HITL ruled the shape an INVALID
    /// SPEC on 2026-07-24 and `apply_projection_config` now rejects it with
    /// [`EngineError::WriteValidation`]. `dense_readiness` accept-inert is
    /// UNCHANGED and stands on its own footing: it is engine-set READ METADATA,
    /// never part of the declaration, so there is nothing about it to reject.
    ///
    /// The bindings still HARD-REJECT the shapes that could
    /// not round-trip: a readiness supplied with `vector = false`, and any
    /// spelling outside `{unavailable, embedding, ready}`.
    pub dense_readiness: Option<DenseReadiness>,
}

/// 0.8.20 Slice 15d (R-20-PR / C-1) — a single declarative projection
/// declaration. HITL-ratified shape (`api-surface.md:85-89`):
/// `{ name, roles: Set<ProjectionRole>, fts?, vector? }`. `roles` carries SET
/// semantics (dedup + membership; an attribute can be `Filterable` AND
/// `Searchable`) — encoded here as a sorted, de-duplicated `BTreeSet`. Named
/// `roles`, not `kind` (`kind` is the node/edge type discriminator).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionSpec {
    pub name: String,
    pub roles: BTreeSet<ProjectionRole>,
    pub fts: Option<ProjectionFts>,
    pub vector: Option<ProjectionVector>,
    /// Optional ordered literal object-member path in the canonical node body.
    /// `None` preserves the legacy direct top-level lookup by `name`.
    pub source: Option<Vec<String>>,
}

/// 0.8.20 Slice 15d (R-20-PR) — the diff [`Engine::configure_projections`]
/// applied. Idempotent re-registration yields `unchanged == true` with all
/// vecs empty (the "re-registration is a no-op" acceptance signal). A
/// destructive change without an explicit `drop` is an `Err`, not a delta.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectionDelta {
    /// Attribute names whose same-transaction projections (EAV / property-FTS)
    /// were (re)built by this apply.
    pub built: Vec<String>,
    /// Attribute names dropped (explicit `drop` list) — their EAV + property-FTS
    /// rows and registry row removed.
    pub dropped: Vec<String>,
    /// Attribute names whose declared roles were persisted but NOT built:
    /// `rankable` (F9 not yet live) and the `searchable→vector` sub-target
    /// (Slice 20). These graft on a future idempotent apply. No error.
    pub deferred: Vec<String>,
    /// True iff nothing was built, dropped, or newly deferred — the whole apply
    /// diffed to a no-op.
    pub unchanged: bool,
    /// 0.8.20 Slice 22 (R-20-VC / **TC-67**) — **node KINDS, not attribute
    /// names.** The vector-eligible node kinds present in the corpus that the
    /// vector writer can NEVER commit, so no `searchable→vector` declaration
    /// will ever produce an embedding for them.
    ///
    /// # Why this field exists — the silence it replaces
    ///
    /// [`kind_is_vector_committable`] (Slice 20c fix-2) restricted enrolment to
    /// the kinds [`resolve_source_type`] maps, because enrolling any other kind
    /// is a permanent liveness wedge. That fix was correct and is unchanged —
    /// but it made the exclusion **silent**: the declaration persists, its name
    /// is pushed onto [`ProjectionDelta::deferred`], and the caller cannot tell
    /// "waiting on the embedder" (transient) from "this kind will never be
    /// embedded" (permanent). Per the HITL ruling on TC-67 the remedy is
    /// option **(c) REPORT** — the vocabulary is NOT grown and the Pack-1 D3
    /// partition-key lock is NOT touched (`dev/design/0.7.0-vector-quant-pack1.md`).
    ///
    /// # Axis, and why the name is what it is
    ///
    /// `built` / `dropped` / `deferred` are all lists of **projection attribute
    /// names**. This one is a list of **node kinds** — a different axis entirely,
    /// so the name says `kinds` explicitly and is prefixed `vector_` to bind it
    /// to the dense arm (an unsupported kind is still fully FTS/lexically
    /// searchable). Sorted and de-duplicated (`SELECT DISTINCT … ORDER BY kind`).
    ///
    /// # It is a STATE report, not a diff
    ///
    /// Unlike the other three vectors it does not describe what this call
    /// changed; it describes the corpus as it stands. So it is populated on an
    /// idempotent re-apply too (where `unchanged == true` and the other three
    /// are empty), and it deliberately does NOT feed [`ProjectionDelta::unchanged`].
    /// That is what makes the declare-time residual cheap to live with: to
    /// refresh the report after writing new kinds, re-apply the same spec — a
    /// no-op that still returns a current report.
    ///
    /// # Independent of the embedder
    ///
    /// Computed whenever a `searchable→vector` projection is declared, whether
    /// or not this session has a usable dense runtime. The vocabulary is static, so
    /// "this kind can never be embedded" is true in a no-embedder session too —
    /// and must not be conflated with the Q6a graceful-absent deferral, which is
    /// transient and is reported through `deferred`.
    ///
    /// Empty (never absent) when there is nothing to report.
    pub vector_unsupported_kinds: Vec<String>,
}

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
fn is_valid_attribute_name(name: &str) -> bool {
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
fn projection_json_path(name: &str, stored: &StoredProjection) -> String {
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
fn is_valid_projection_source(source: &[String]) -> bool {
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
fn validate_projection_source_backfill(
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
fn persist_projection_row(
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
fn remove_projection_row(tx: &Connection, name: &str) -> rusqlite::Result<()> {
    tx.execute("DELETE FROM _fathomdb_projection_registry WHERE name = ?1", params![name])?;
    Ok(())
}

/// 0.8.20 Slice 15d (R-20-EAV) — delete every EAV + property-FTS row for one
/// attribute `name` (all owning nodes). The idempotent-rebuild primitive: a
/// changed or dropped projection clears its rows before (re)backfill.
fn clear_attribute_projection(tx: &Connection, name: &str) -> rusqlite::Result<()> {
    tx.execute("DELETE FROM property_search_index WHERE attr_name = ?1", params![name])?;
    tx.execute("DELETE FROM canonical_attributes WHERE attr_name = ?1", params![name])?;
    Ok(())
}

/// 0.8.20 Slice 15d (R-20-EAV) — project ONE attribute value for ONE node row
/// into the EAV store and (if `searchable→FTS`) the property-FTS shadow. Skips a
/// NULL/absent extraction (an absent attribute means no row, so a `filterable`
/// equality simply never matches it — correct). Shared by the write path and
/// the backfill so they cannot drift.
fn project_one_attribute(
    tx: &Connection,
    cursor: i64,
    body: &str,
    name: &str,
    stored: &StoredProjection,
) -> rusqlite::Result<()> {
    if !stored.wants_eav() {
        return Ok(());
    }
    // json_extract over a non-JSON body would error; guard with json_valid so a
    // plain-text body simply yields no attribute rows. The canonical scalar
    // extraction is shared with the Slice-15e vec0 pre-KNN column via
    // [`extract_scalar_attribute`], so the EAV value and the `attr_<hex>` value
    // are IDENTICAL by construction.
    //
    // fix-1 finding 1 [P2] — project EVERY JSON scalar type, not just strings.
    // The prior form read the extraction as `Option<String>`; for a JSON number
    // or bool, `json_extract` returns an INTEGER/REAL, the `get::<Option<String>>`
    // conversion FAILED, and `.unwrap_or(None)` silently treated the attribute as
    // absent — so a numeric/boolean filterable value never projected. We now
    // render a single canonical TEXT form per JSON type, keyed on `json_type` so
    // the stored value is deterministic and the SAME value flows to BOTH
    // `canonical_attributes` and `property_search_index` (consistency by
    // construction — one `value` binding below):
    //   - string  -> the text verbatim
    //   - integer -> decimal text (CAST AS TEXT); e.g. 3 -> "3"
    //   - real    -> decimal text (CAST AS TEXT); e.g. 3.5 -> "3.5"
    //   - true    -> "true", false -> "false"  (preserve the JSON literal, NOT the
    //                SQLite `1`/`0` that a bare `CAST(json_extract(...) AS TEXT)`
    //                would yield — so a bool filter matches the value the caller
    //                wrote, and "true" never collides with the number 1).
    //   - null / absent path -> SQL NULL -> no row (an absent attribute correctly
    //                never matches a `filterable` equality).
    //   - object / array -> DELIBERATELY SKIPPED (SQL NULL -> no row): a composite
    //                value is not a scalar filter/FTS target in 15d; projecting its
    //                raw JSON text would be a footgun (nested-field filtering is the
    //                >=0.9.x multi-field work). Skipping is deliberate, not an
    //                accidental type-conversion drop — no scalar type is dropped.
    let Some(value) = extract_scalar_attribute(tx, body, name, stored)? else {
        return Ok(());
    };
    tx.execute(
        "INSERT INTO canonical_attributes(write_cursor, attr_name, attr_value)
         VALUES(?1, ?2, ?3)",
        params![cursor, name, value],
    )?;
    if stored.wants_property_fts() {
        tx.execute(
            "INSERT INTO property_search_index(attr_value, attr_name, write_cursor)
             VALUES(?1, ?2, ?3)",
            params![value, name, cursor],
        )?;
    }
    Ok(())
}

/// 0.8.20 Slice 15e fix-3 [P2] — the leading marker byte that the vec0
/// `attr_<hex>` FILTER column prepends to every PRESENT scalar value, so that
/// PRESENT and ABSENT are DISJOINT in a NOT-NULL TEXT column.
///
/// The `''` empty-string sentinel used to mean BOTH "attribute absent" AND
/// "attribute present with value `''`", so a `status == ""` equality filter
/// false-matched every absent row. vec0 TEXT metadata is NOT-NULL-able (TC-46
/// condition #3), so absent cannot be `NULL`; instead absent stays `''` and every
/// PRESENT value `V` is encoded `enc(V) = "\x01" || V`. This is injective and
/// non-empty for ALL `V` (including `V=""`, whose encoding is the bare marker),
/// so `attr_<hex> = enc("")` matches present-empty but NEVER the `''`-absent rows.
///
/// This encoding is CONFINED to the vec0 filter column and the filter-value
/// lowering ([`vector_filter_values`]). `property_search_index` (the searchable→FTS
/// projection) and `canonical_attributes.attr_value` keep the RAW value — the FTS
/// arm distinguishes absent from present-empty by canonical_attributes row
/// EXISTENCE instead (see [`hit_attributes_pass_filter`]).
const ATTR_VEC0_PRESENT_MARKER: char = '\u{1}';

/// 0.8.20 Slice 15e fix-3 — encode a PRESENT scalar value for the vec0 filter
/// column / filter-value lowering (see [`ATTR_VEC0_PRESENT_MARKER`]). ABSENT is
/// NOT encoded (it stays the bare `''` sentinel), so this is only ever called on a
/// value known to be present.
pub(crate) fn encode_attr_vec0_present(value: &str) -> String {
    let mut s = String::with_capacity(value.len() + 1);
    s.push(ATTR_VEC0_PRESENT_MARKER);
    s.push_str(value);
    s
}

/// 0.8.20 Slice 15e — extract the canonical TEXT form of attribute `name` from a
/// node `body`, using the SAME `json_type` CASE as [`project_one_attribute`] so
/// the vec0 pre-KNN `attr_<hex>` column value equals the EAV
/// `canonical_attributes` value (consistency by construction — a `filterable`
/// filter routed pre-KNN sees exactly what the EAV path stored). Returns `None`
/// for an absent / null / object / array / non-JSON extraction (⇒ the `''`
/// sentinel at the vec0 column, ⇒ fail-to-match).
pub(crate) fn extract_scalar_attribute(
    conn: &Connection,
    body: &str,
    name: &str,
    stored: &StoredProjection,
) -> rusqlite::Result<Option<String>> {
    let path = projection_json_path(name, stored);
    let value: Option<String> = conn
        .query_row(
            "SELECT CASE WHEN json_valid(?1) THEN
                 CASE json_type(?1, ?2)
                     WHEN 'true'   THEN 'true'
                     WHEN 'false'  THEN 'false'
                     WHEN 'null'   THEN NULL
                     WHEN 'object' THEN NULL
                     WHEN 'array'  THEN NULL
                     ELSE CAST(json_extract(?1, ?2) AS TEXT)
                 END
             END",
            params![body, path],
            |row| row.get::<_, Option<String>>(0),
        )
        .unwrap_or(None);
    Ok(value)
}

/// 0.8.20 Slice 15e — for a node `body`, build the `, attr_<hex>` column suffix,
/// the `, ?N` placeholder suffix (numbered from `start_idx`), and the bound TEXT
/// values for EVERY attribute column CURRENTLY on the live `vector_default` (read
/// from the table's own SQL, so the INSERT always matches the table shape exactly —
/// vec0 rejects a partial-column INSERT). Each value is the body's canonical
/// scalar extraction, or the `''` sentinel when absent. Returns empty fragments
/// (and no values) when the table has no attribute columns, so the INSERT stays
/// byte-identical to the shipped statement.
pub(crate) fn vector_attr_insert_fragments(
    conn: &Connection,
    body: &str,
    start_idx: usize,
) -> rusqlite::Result<(String, String, Vec<rusqlite::types::Value>)> {
    let cols = actual_vector_attr_columns(conn)?;
    let registry = load_projection_registry(conn)?;
    let mut col_sql = String::new();
    let mut ph_sql = String::new();
    let mut values: Vec<rusqlite::types::Value> = Vec::new();
    for (i, col) in cols.iter().enumerate() {
        let name = decode_attr_vec0_column(col).unwrap_or_default();
        // fix-3 [P2] — a PRESENT scalar value is encoded `\x01 || V` so it is
        // DISJOINT from the `''`-absent sentinel (present-empty ⇒ the bare marker,
        // never `''`). Absent stays the bare `''` sentinel.
        let scalar = match registry.get(&name) {
            Some(stored) => extract_scalar_attribute(conn, body, &name, stored)?,
            None => None,
        };
        let value = match scalar {
            Some(v) => encode_attr_vec0_present(&v),
            None => String::new(),
        };
        col_sql.push_str(&format!(", {col}"));
        ph_sql.push_str(&format!(", ?{}", start_idx + i));
        values.push(rusqlite::types::Value::Text(value));
    }
    Ok((col_sql, ph_sql, values))
}

/// 0.8.20 Slice 15d (R-20-EAV) — the write-path attribute projector: for a
/// just-inserted node, project EVERY declared attribute (reading the live
/// registry from `tx`). Same-transaction, so the node is filter/FTS-retrievable
/// on commit. A no-op when the registry is empty (the pre-`configure_projections`
/// default), so it costs one empty-table scan per node and is behaviour-neutral
/// until a projection is declared.
pub(crate) fn project_node_attributes(
    tx: &Connection,
    cursor: i64,
    body: &str,
) -> rusqlite::Result<()> {
    let registry = load_projection_registry(tx)?;
    for (name, stored) in &registry {
        project_one_attribute(tx, cursor, body, name, stored)?;
    }
    Ok(())
}

/// 0.8.20 Slice 15d (R-20-PR) — backfill ONE attribute across every ACTIVE,
/// non-superseded canonical node. Called by `configure_projections` when a
/// projection is added/changed (after `clear_attribute_projection`), and by boot
/// re-derive. Idempotent when paired with the clear.
fn backfill_attribute(
    tx: &Connection,
    name: &str,
    stored: &StoredProjection,
) -> rusqlite::Result<()> {
    if !stored.wants_eav() {
        return Ok(());
    }
    let rows: Vec<(i64, String)> = {
        let mut stmt = tx.prepare(
            "SELECT write_cursor, body FROM canonical_nodes
             WHERE superseded_at IS NULL AND state = 'active'",
        )?;
        let collected = stmt
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        collected
    };
    for (cursor, body) in rows {
        project_one_attribute(tx, cursor, &body, name, stored)?;
    }
    Ok(())
}

/// 0.8.20 Slice 15d (R-20-PR) — is `desired` an INCOMPATIBLE/DESTRUCTIVE change
/// to a live `existing` projection? A destructive change discards an
/// expensive-to-rebuild resource and so REQUIRES an explicit `drop` (C3): a role
/// REMOVAL, dropping the `fts`/`vector` sub-target, or changing the tokenizer /
/// embedder. Purely ADDITIVE changes (adding a role, adding an `fts`/`vector`
/// sub-object) are non-destructive and applied in place.
fn is_destructive_projection_change(
    existing: &StoredProjection,
    desired: &StoredProjection,
) -> bool {
    if existing.roles.iter().any(|r| !desired.roles.contains(r)) {
        return true;
    }
    if existing.fts_present
        && (!desired.fts_present || existing.fts_tokenizer != desired.fts_tokenizer)
    {
        return true;
    }
    if existing.vector_declared
        && (!desired.vector_declared || existing.vector_embedder != desired.vector_embedder)
    {
        return true;
    }
    if existing.source != desired.source {
        return true;
    }
    false
}

/// Human-readable summary of the destructive delta, surfaced in
/// [`EngineError::ProjectionDestructive`] so the caller sees WHAT it must drop.
fn describe_projection_delta(existing: &StoredProjection, desired: &StoredProjection) -> String {
    let mut parts: Vec<String> = Vec::new();
    for r in &existing.roles {
        if !desired.roles.contains(r) {
            parts.push(format!("role '{}' removed", r.as_str()));
        }
    }
    if existing.fts_present && !desired.fts_present {
        parts.push("fts sub-target removed".to_string());
    } else if existing.fts_present && existing.fts_tokenizer != desired.fts_tokenizer {
        parts.push("fts tokenizer changed".to_string());
    }
    if existing.vector_declared && !desired.vector_declared {
        parts.push("vector sub-target removed".to_string());
    } else if existing.vector_declared && existing.vector_embedder != desired.vector_embedder {
        parts.push("vector embedder changed".to_string());
    }
    if existing.source != desired.source {
        parts.push("source path changed".to_string());
    }
    if parts.is_empty() {
        "incompatible change".to_string()
    } else {
        parts.join("; ")
    }
}

/// 0.8.20 Slice 15d (R-20-PR) — the declarative, idempotent diff+backfill apply
/// that backs [`Engine::configure_projections`]. Runs inside the caller's write
/// transaction `tx`. Order: apply `drop`s first (so a drop+re-declare in one
/// call rebuilds fresh), then diff each spec. Idempotent re-registration diffs to
/// an empty delta (`unchanged`). A destructive change without an explicit drop is
/// refused with [`EngineError::ProjectionDestructive`].
///
/// 0.8.20 Slice 20c (R-20-DR remainder) — returns `(delta, enqueued_backfill)`.
/// The second member is `true` iff [`enqueue_declared_vector_backfill`] put
/// deferred embed work on the queue, in which case the CALLER must
/// `notify_new_work()` after committing (the flag cannot ride on
/// [`ProjectionDelta`]: that is the caller-facing diff, and this is a runtime
/// signal, not part of the declaration's result).
///
/// 0.8.20 Slice 20c fix-1 (codex §9 [P2]) — and the symmetric inverse: a call
/// that removes the LAST `searchable→vector` declaration un-enrols the node kinds
/// the forward path enrolled ([`unenrol_registry_vector_node_kinds`]), on this
/// same transaction. It deletes no embedding.
///
/// 0.8.20 Slice 21 fix-1 (codex §9 round 1 [P2]) — and, beside that transition, a
/// state-keyed RECONCILIATION ([`registry_governs_an_inert_dense_arm`]) so that a
/// database already carrying an inert enrolment from before the Slice-21c role
/// gate is healed by any `configure_projections` call, not only by a
/// searchable-vector-to-none transition it may never perform. The boot arm is
/// [`reconcile_inert_vector_enrolments_on_boot`].
fn apply_projection_config(
    tx: &Connection,
    specs: &[ProjectionSpec],
    drop: &[String],
    dense_arm_live: bool,
) -> Result<(ProjectionDelta, bool), EngineError> {
    // Validate up-front so a bad name aborts before any write.
    //
    // fix-6 finding [P2] — REJECT a duplicate projection `name` within `specs`
    // (and a duplicate entry within `drop`) up front. The diff loop below diffs
    // every spec against the ONE pre-loop registry snapshot, so a name repeated
    // in `specs` diffed the SECOND spec against state that never saw the first
    // spec's just-persisted row: on a fresh DB `[status(searchable+fts),
    // status(rankable-only)]` reported `built=[status]` in the delta while the
    // registry ended rankable-only (which builds nothing) — the returned delta
    // DIVERGED from the persisted registry, breaking the fix-4 "accept ⟹ correct"
    // contract. A duplicate `drop` entry likewise reported the drop twice though
    // the row was removed once. A single request naming the same projection twice
    // is ambiguous/malformed, so we refuse it (rejection, not last-wins coalesce)
    // — a rejected request is a total no-op, keeping the registry and delta
    // consistent with the accepted input. A name that appears in BOTH `specs` and
    // `drop` is NOT a duplicate: that is the documented drop-then-rebuild-fresh
    // pattern (drops apply first, then the fresh spec builds), so it is allowed.
    let mut seen_spec_names: BTreeSet<&str> = BTreeSet::new();
    for spec in specs {
        if !is_valid_attribute_name(&spec.name) {
            return Err(EngineError::InvalidArgument {
                msg: format!("invalid projection attribute name: {:?}", spec.name),
            });
        }
        if spec.roles.is_empty() {
            return Err(EngineError::InvalidArgument {
                msg: format!("projection '{}' declares no roles", spec.name),
            });
        }
        if let Some(source) = &spec.source {
            if !is_valid_projection_source(source) {
                return Err(EngineError::InvalidArgument {
                    msg: format!("invalid projection source path for {:?}", spec.name),
                });
            }
        }
        if !seen_spec_names.insert(spec.name.as_str()) {
            return Err(EngineError::InvalidArgument {
                msg: format!("duplicate projection name in one request: '{}'", spec.name),
            });
        }
    }
    let mut seen_drop_names: BTreeSet<&str> = BTreeSet::new();
    for name in drop {
        if !is_valid_attribute_name(name) {
            return Err(EngineError::InvalidArgument {
                msg: format!("invalid projection drop name: {name:?}"),
            });
        }
        if !seen_drop_names.insert(name.as_str()) {
            return Err(EngineError::InvalidArgument {
                msg: format!("duplicate projection drop in one request: '{name}'"),
            });
        }
    }

    // 0.8.20 Slice 23 (`R-20-SV`) — REJECT an `fts` or `vector` sub-object
    // declared WITHOUT the `searchable` role.
    //
    // HITL ruling 2026-07-24 (`dev/plans/plan-0.8.20.md` §11 item 4, option (b)):
    // *"it is a meaningless config; fail-fast matches the hard-reject philosophy,
    // and additive strictness is safe pre-1.0"*, to be implemented "at the next
    // `configure_projections` slice". This OVERTURNS the shipped 15d fix-4
    // position, which accepted the shape because it round-tripped faithfully.
    //
    // WHY it is meaningless: `searchable→FTS` and `searchable→vector` are TIER
    // LABELS, not roles ([`ProjectionRole`] has exactly three members). The
    // sub-objects SELECT a sub-target of `searchable`; they do not CONFER one —
    // both build predicates ([`StoredProjection::wants_property_fts`] and
    // [`StoredProjection::wants_vector`]) are conjunctions with
    // `roles.contains(Searchable)`. So without the role the declaration builds no
    // property-FTS, enrols no kind and embeds nothing: it names a sub-target of a
    // projection that does not exist. The reject is therefore keyed on the
    // ABSENCE of `searchable` and on nothing else — `filterable` / `rankable` are
    // orthogonal axes that neither supply nor substitute for it.
    //
    // FAMILY: [`EngineError::WriteValidation`], per decision #18 (0.8.20 Slice 22)
    // — the write-SHAPE boundary is ONE family, and this is a shape rejection.
    // Deliberately a SEPARATE loop from the name checks above: those are NAME
    // rejections that keep `InvalidArgument { msg }` because the message naming
    // the offending value is the caller's only handle on it. `dev/design/errors.md`
    // ("Validation boundary") states that split; keeping the two loops apart keeps
    // the split visible in the code and this change one-line-reversible.
    //
    // KNOWN COST (TC-95/TC-98, HITL-deferred): `WriteValidation` is a UNIT
    // variant, so this refusal cannot name WHICH spec in `specs` was invalid —
    // strictly worse than the name rejections above. Recorded, not worked around.
    for spec in specs {
        if spec.roles.contains(&ProjectionRole::Searchable) {
            continue;
        }
        if spec.fts.is_some() || spec.vector.is_some() {
            return Err(EngineError::WriteValidation);
        }
    }

    // A destructive source change retains the normal drop-first error precedence.
    // Check the pre-drop registry before inspecting a proposed source's backfill
    // rows; otherwise a composite at that source could mask ProjectionDestructive.
    let pre_drop = load_projection_registry(tx).map_err(|_| EngineError::Storage)?;
    let mut refresh_vector_attributes = false;
    for spec in specs {
        let desired = StoredProjection::from_spec(spec);
        if let Some(existing) = pre_drop.get(&spec.name) {
            let replacing = drop.iter().any(|name| name == &spec.name);
            if !replacing && is_destructive_projection_change(existing, &desired) {
                return Err(EngineError::ProjectionDestructive {
                    name: spec.name.clone(),
                    delta: describe_projection_delta(existing, &desired),
                });
            }
            if replacing
                && existing.source != desired.source
                && desired.roles.contains(&ProjectionRole::Filterable)
            {
                refresh_vector_attributes = true;
            }
        }
    }

    // A declared nested source is scalar-only. Validate the complete backfill
    // set before any registry mutation so a composite terminal rolls the whole
    // configuration request back with the existing write-validation family.
    for spec in specs {
        let desired = StoredProjection::from_spec(spec);
        validate_projection_source_backfill(tx, &spec.name, &desired)?;
    }

    let mut delta = ProjectionDelta::default();

    // 0.8.20 Slice 20c fix-1 (codex §9 [P2]) — snapshot "is the dense arm
    // declared?" BEFORE any registry mutation. Together with the same read taken
    // after them it identifies the ONE transition that owns the inverse of this
    // slice's enrolment: declared -> not-declared. See
    // [`unenrol_registry_vector_node_kinds`] for why the inverse is keyed to that
    // TRANSITION rather than to the bare post-state.
    let vector_declared_before =
        vector_projection_declared(tx).map_err(|_| EngineError::Storage)?;

    // (1) Explicit drops. Omission never drops (C3); only this list does.
    let before_drop = load_projection_registry(tx).map_err(|_| EngineError::Storage)?;
    for name in drop {
        if before_drop.contains_key(name) {
            clear_attribute_projection(tx, name).map_err(|_| EngineError::Storage)?;
            remove_projection_row(tx, name).map_err(|_| EngineError::Storage)?;
            delta.dropped.push(name.clone());
        }
        // dropping an absent projection is an idempotent no-op, not an error.
    }

    // (2) Diff each spec against the post-drop registry.
    let current = load_projection_registry(tx).map_err(|_| EngineError::Storage)?;
    for spec in specs {
        let desired = StoredProjection::from_spec(spec);
        match current.get(&spec.name) {
            Some(existing) if existing == &desired => {
                // Idempotent re-registration — no-op (the keystone acceptance).
            }
            Some(existing) => {
                if is_destructive_projection_change(existing, &desired) {
                    return Err(EngineError::ProjectionDestructive {
                        name: spec.name.clone(),
                        delta: describe_projection_delta(existing, &desired),
                    });
                }
                persist_projection_row(tx, &spec.name, &desired)
                    .map_err(|_| EngineError::Storage)?;
                clear_attribute_projection(tx, &spec.name).map_err(|_| EngineError::Storage)?;
                backfill_attribute(tx, &spec.name, &desired).map_err(|_| EngineError::Storage)?;
                if desired.wants_eav() {
                    delta.built.push(spec.name.clone());
                }
                // 0.8.20 Slice 15e fix-2 finding 2 [P2] — this arm ONLY runs when
                // the registry row actually CHANGED (`existing != desired`), so the
                // delta MUST reflect that change; otherwise an accepted mutation
                // reports `unchanged = true` — a no-op lie to SDK callers. The prior
                // `&& !existing.has_deferred()` guard suppressed a deferred-ONLY
                // change (e.g. `rankable` → `rankable + vector`, which builds no EAV
                // so `built` stays empty): the row persisted but `delta` came back
                // empty. Mirror the fresh-registration push (`if
                // desired.has_deferred()`). Every valid non-empty spec has
                // `wants_eav()` OR `has_deferred()`, so on a real change at least one
                // of `built`/`deferred` is now populated ⇒ `unchanged` can never be
                // `true` on a persisted change. A genuine no-op (identical spec)
                // takes the idempotent arm above and is untouched.
                if desired.has_deferred() {
                    delta.deferred.push(spec.name.clone());
                }
            }
            None => {
                persist_projection_row(tx, &spec.name, &desired)
                    .map_err(|_| EngineError::Storage)?;
                clear_attribute_projection(tx, &spec.name).map_err(|_| EngineError::Storage)?;
                backfill_attribute(tx, &spec.name, &desired).map_err(|_| EngineError::Storage)?;
                if desired.wants_eav() {
                    delta.built.push(spec.name.clone());
                }
                if desired.has_deferred() {
                    delta.deferred.push(spec.name.clone());
                }
            }
        }
    }

    // 0.8.20 Slice 15e — after the registry mutations, reconcile the live vec0
    // shape with the (possibly changed) `filterable` set: a NON-DESTRUCTIVE
    // reshape adds/removes the `attr_<hex>` pre-KNN columns preserving every
    // row's embedding (TC-46, HITL Option 1). Runs on the caller's write
    // transaction, so the reshape commits atomically with the registry row. On an
    // idempotent re-registration the desired set equals the live set, so this is a
    // no-op (vec0 untouched) and `delta.unchanged` above is unaffected. Skipped
    // when there is no embedder profile (⇒ no `vector_default` to reshape).
    if let Ok(dimension) = default_profile_dimension(tx) {
        if refresh_vector_attributes {
            refresh_vector_attr_values(tx, dimension).map_err(|_| EngineError::Storage)?;
        } else {
            reconcile_vector_attr_columns(tx, dimension).map_err(|_| EngineError::Storage)?;
        }
    }

    delta.unchanged =
        delta.built.is_empty() && delta.dropped.is_empty() && delta.deferred.is_empty();

    // 0.8.20 Slice 20c (R-20-DR remainder) — THE C4 RIDER. Everything above has
    // only *persisted* the `searchable→vector` declaration and pushed its name
    // onto `delta.deferred`. Acknowledging deferred work and then dropping it on
    // the floor is what made `drain` a FALSE-READY barrier; this call is where the
    // deferred work is actually enqueued onto the runtime `drain` waits on.
    //
    // fix-1 (codex §9 [P2]) — and its SYMMETRIC INVERSE, on the same
    // transaction. If this call removed the last `searchable→vector` declaration,
    // un-enrol the node kinds the forward path enrols; otherwise enrolment is a
    // one-way door and the write path keeps embedding for a projection the
    // registry no longer declares.
    let vector_declared_after = vector_projection_declared(tx).map_err(|_| EngineError::Storage)?;
    let enqueued = if vector_declared_after {
        // 0.8.20 Slice 22 (R-20-VC / TC-67) — THE REPORT. Scoped to a live dense
        // -arm declaration (with no `searchable→vector` projection there is
        // nothing for a kind to be unsupported FOR, so reporting would be noise
        // on every `filterable`/FTS-only call), but deliberately OUTSIDE the
        // `dense_arm_live` gate below — see [`unsupported_vector_kinds`].
        //
        // Placed AFTER `delta.unchanged` is computed, and it does not feed it:
        // this is a STATE report, not a diff, so an idempotent re-apply still
        // carries it (that is also the documented refresh path for the
        // declare-time residual).
        delta.vector_unsupported_kinds =
            unsupported_vector_kinds(tx).map_err(|_| EngineError::Storage)?;
        if dense_arm_live {
            enqueue_declared_vector_backfill(tx).map_err(|_| EngineError::Storage)?
        } else {
            false
        }
    } else {
        // fix-1 (codex §9 round 1 [P2], ledger `TC-71`) — the transition arm is
        // KEPT as-is and a state-keyed reconciliation is added BESIDE it; neither
        // subsumes the other. The transition fires when this very call removed the
        // last dense-arm declaration, including the case where it removed the last
        // `vector` sub-object with it (which leaves
        // `registry_governs_an_inert_dense_arm` false). The reconciliation covers
        // the ALREADY-AFFECTED database whose user calls `configure_projections`
        // again with anything at all: there `before` is already `false`, so the
        // transition arm is inert and the inert enrolment used to survive
        // indefinitely. `||` short-circuits, so the transition case pays nothing
        // extra; the other case pays two cached `EXISTS` probes on a governed call,
        // never on the hot write path.
        if vector_declared_before
            || registry_governs_an_inert_dense_arm(tx).map_err(|_| EngineError::Storage)?
        {
            unenrol_registry_vector_node_kinds(tx).map_err(|_| EngineError::Storage)?;
        }
        false
    };
    Ok((delta, enqueued))
}

impl Engine {
    /// 0.8.20 Slice 15d (R-20-PR / C-1) — the projection registry as a
    /// DECLARATIVE, IDEMPOTENT apply. The engine is the SOLE projection authority
    /// (Q3): it diffs the supplied `specs` against the durable registry and
    /// backfills the difference in ONE transaction. Cheap projections
    /// (`filterable`, `searchable→FTS`) are built same-transaction; `rankable`
    /// and the `searchable→vector` sub-target are PERSISTED but deferred (F9 /
    /// Slice 20) — declaring them never errors (graceful-absent, Q6a).
    ///
    /// 0.8.20 Slice 23 (`R-20-SV`) — **SPEC VALIDATION.** A spec that carries an
    /// `fts` or `vector` sub-object WITHOUT [`ProjectionRole::Searchable`] is an
    /// INVALID SPEC and is refused with [`EngineError::WriteValidation`] (HITL
    /// 2026-07-24; see [`apply_projection_config`] for the full rationale). A
    /// rejected request is a TOTAL no-op. `read_projections` is unaffected — it
    /// is a pure read — so a LEGACY row in that shape still reports verbatim but
    /// can no longer be re-applied.
    ///
    /// `drop` is EXPLICIT (C3, `api-surface.md:27`): omission of a live
    /// projection from `specs` does NOT drop it; removal requires naming it in
    /// `drop`. An incompatible/destructive change to a live projection that is
    /// NOT in `drop` is refused with [`EngineError::ProjectionDestructive`], the
    /// destructive delta surfaced — never silent data loss. Re-applying an
    /// unchanged spec diffs to a no-op ([`ProjectionDelta::unchanged`]).
    ///
    /// Pair with [`Engine::read_projections`] to see current state before
    /// applying.
    pub fn configure_projections(
        &self,
        specs: &[ProjectionSpec],
        drop: &[String],
    ) -> Result<ProjectionDelta, EngineError> {
        self.ensure_open()?;
        // Settle in-flight async projection work first. The worker commits on its
        // own connection with `BEGIN IMMEDIATE`; a backfill issued in that write
        // window would SQLITE_BUSY.
        self.drain_for_non_embedding_mutation()?;

        // The backfill is gated on a usable dense runtime. Without one, the
        // declaration persists and defers rather than queueing unsafe work. A
        // later approved open grafts it; idempotent apply remains a repair door.
        let dense_arm_live = self.usable_dense_runtime();
        let (delta, enqueued_backfill) = {
            let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
            let connection = connection.as_mut().ok_or(EngineError::Closing)?;
            dependency_closure::maintain_before_writer(connection)?;
            let tx = connection
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|_| EngineError::Storage)?;
            dependency_closure::guard_no_pending_physical(&tx)?;
            let applied = apply_projection_config(&tx, specs, drop, dense_arm_live)?;
            if !applied.0.unchanged {
                projection_generation::transition(
                    &tx,
                    ProjectionGenerationOriginV1::Configuration,
                )?;
            }
            tx.commit().map_err(|_| EngineError::Storage)?;
            applied
        };
        // 0.8.20 Slice 20c (R-20-DR remainder) — the C4 rider's second half. The
        // enrolment + terminal-clear committed above; now WAKE the dispatcher, or
        // it sleeps on `pending_scan == false` and the very next `drain` burns its
        // whole timeout waiting for work nobody scheduled. `drain` itself stays
        // PASSIVE (a barrier, never a trigger) — the notify belongs here, on the
        // enqueue side. Deliberately after the connection guard is dropped: the
        // dispatcher immediately opens its own connection to scan.
        if enqueued_backfill {
            self.projection_runtime.notify_new_work();
        }
        self.counters.record_admin();
        Ok(delta)
    }

    /// 0.8.20 Slice 15d (R-20-PR) — read the current projection registry (C5
    /// introspection: `read.projections`). Returns every declared
    /// [`ProjectionSpec`] sorted by name, so a caller can inspect current state
    /// (and the destructive delta a change would cause) BEFORE applying. Pure
    /// read; never mutates.
    ///
    /// 0.8.20 Slice 20 (R-20-DR) — this is ALSO the surface that populates the
    /// engine-set [`ProjectionVector::dense_readiness`] READ METADATA. It is
    /// derived here, on the way out (see [`derive_dense_readiness`]); the durable
    /// registry stores no readiness. Only a spec that declares the
    /// `searchable→vector` sub-object carries one — `filterable` and
    /// `searchable→FTS` are same-transaction and have no readiness axis.
    pub fn read_projections(&self) -> Result<Vec<ProjectionSpec>, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let registry = load_projection_registry(connection).map_err(|_| EngineError::Storage)?;
        // Derived ONCE per call, so every vector projection in one read reports a
        // consistent readiness (they are all served by the one vector pipeline).
        // Skipped entirely when no vector projection is declared, keeping the
        // no-vector default path free of the extra probe.
        let mut readiness: Option<DenseReadiness> = None;
        let mut specs: Vec<ProjectionSpec> =
            registry.iter().map(|(name, stored)| stored.to_spec(name)).collect();
        for spec in &mut specs {
            if let Some(vector) = spec.vector.as_mut() {
                let value = match readiness {
                    Some(value) => value,
                    None => {
                        let value =
                            derive_dense_readiness(connection, self.usable_dense_runtime())?;
                        readiness = Some(value);
                        value
                    }
                };
                vector.dense_readiness = Some(value);
            }
        }
        Ok(specs)
    }
}

/// 0.8.20 Slice 20c fix-1 (codex §9 [P2] "Stop embedding after vector projection
/// drops") — **the inverse of [`enqueue_declared_vector_backfill`]'s enrolment.**
///
/// Slice 20c gave `_fathomdb_vector_kinds` its first governed-call-reachable
/// writer for a NODE kind (before it, the only one was the `#[doc(hidden)]`
/// `configure_vector_kind_for_test` hook). Forward without reverse is the defect:
/// after `drop`ping the last `searchable→vector` declaration,
/// [`project_canonical_node_row`]'s `kind_is_vector_indexed` gate and
/// [`connection_has_pending_projection_work`] both still see the enrolment, so
/// subsequent writes keep enqueueing embeds and `drain` keeps waiting on work for
/// a projection [`Engine::read_projections`] no longer reports.
///
/// # It DELETES NO EMBEDDING — that is the point
///
/// The shipped drop arm ([`clear_attribute_projection`] +
/// [`remove_projection_row`]) has never touched vec0, `_fathomdb_vector_rows` or
/// `_fathomdb_vector_kinds`, so "vectors already at rest survive a drop" is
/// ALREADY the shipped contract. Removing one registry row PRESERVES it; deleting
/// embeddings would be the destructive delta, and is not done here.
///
/// # Why keyed to the TRANSITION, not to the bare post-state
///
/// The rule is "this call removed the last vector declaration"
/// (`declared_before && !declared_after`), not "no vector declaration exists
/// now". A workspace can hold enrolments this registry never made — the test hook
/// does exactly that, and several shipped suites enrol a kind through it and then
/// declare an unrelated `filterable`-only projection (e.g.
/// `slice15e_prekn_filterable`). Firing on the bare post-state would un-enrol
/// those and silently kill a dense arm the registry never owned. In production
/// the two readings coincide: before this slice
/// `production_vector_kind_surface=[]`, so a node kind can only be enrolled
/// because a `searchable→vector` declaration existed.
///
/// It is still STATE-keyed, not delta-keyed: both members are reads of the
/// registry, never "was this spec new". Re-applying the same drop finds
/// `declared_before == false` and is a total no-op, and nothing re-enrols it
/// ([`Engine::enrol_vector_kind_if_declared`] is gated on
/// [`vector_projection_declared`]).
///
/// # 0.8.20 Slice 21 fix-1 — a SECOND, narrower authorisation now exists
///
/// The reasoning above is why the bare post-state cannot authorise this DELETE,
/// and it still stands. What it does not cover is a database that ran the
/// PRE-Slice-21c code and enrolled node kinds off a `{filterable, vector}`
/// declaration: there the registry DID own the enrolment, and no transition will
/// ever fire for it. [`registry_governs_an_inert_dense_arm`] adds exactly that
/// case — positively conditioned on the registry existing AND declaring a
/// `vector` sub-object AND declaring no `searchable→vector` projection, which is
/// strictly narrower than the bare post-state and in particular excludes every
/// workspace whose enrolment the registry never made. Its callers are
/// [`reconcile_inert_vector_enrolments_on_boot`] and the drop arm of
/// [`apply_projection_config`].
///
/// # `'edge_fact'` is excluded, deliberately
///
/// [`project_canonical_edge_row`] (G11) auto-registers `'edge_fact'` off the
/// presence of an edge BODY, unconditionally and independently of the projection
/// registry. That lifecycle predates this slice and is not the registry's to end,
/// so a node-projection drop must not take the edge dense arm down with it.
///
/// # What it deliberately does NOT do
///
/// It touches no `_fathomdb_projection_terminal` row and no readiness watermark.
/// A row enqueued-but-not-yet-embedded when the drop lands keeps its absent
/// terminal, which pins the watermark below it — harmless, because both the
/// scheduler and the pending-work probe join `_fathomdb_vector_kinds` and so no
/// longer see it, and it is precisely what lets a later RE-declaration pick the
/// row up again instead of stranding it.
fn unenrol_registry_vector_node_kinds(tx: &Connection) -> rusqlite::Result<()> {
    tx.execute("DELETE FROM _fathomdb_vector_kinds WHERE kind <> 'edge_fact'", [])?;
    Ok(())
}

/// 0.8.20 Slice 21 fix-1 (codex §9 round 1 `[P2]`, ledger `TC-71`) — **does the
/// registry GOVERN the dense arm while declaring none?** The narrow,
/// positively-conditioned predicate that authorises
/// [`unenrol_registry_vector_node_kinds`] on a bare STATE rather than on the
/// `declared_before && !declared_after` transition.
///
/// # Why a state-keyed authorisation exists at all
///
/// Slice 21c gated the dense arm on the `searchable` ROLE, which closes the three
/// FORWARD doors. It cannot reach a database that already ran the old code: those
/// node kinds are already in `_fathomdb_vector_kinds`, and
///
/// - [`Engine::vector_kind_needs_enrolment`] returns early the moment
///   [`kind_is_vector_indexed`] is true, so it never consults the new role-aware
///   predicate for an EXISTING registration; and
/// - [`project_canonical_node_row`] gates the embed enqueue solely on registry
///   membership (deliberately — that is the hot write path, and the decision is
///   meant to live upstream).
///
/// So without this, upgrading does not actually stop the billable, unexpected
/// embeddings for exactly the population TC-71 was raised for — the finding's
/// whole harm survives the fix unless the user happens to perform a
/// searchable-vector-to-none transition later.
///
/// # THE TRAP: why it is not `!vector_projection_declared`
///
/// [`vector_projection_declared`] answers `false` when the registry table is
/// ABSENT (pre-step-24) or merely EMPTY — which is every LEGACY database, many of
/// which have a legitimately working dense arm enrolled by other means (the
/// `#[doc(hidden)]` `configure_vector_kind_for_test` hook is one; before this
/// slice `production_vector_kind_surface=[]`, but a workspace is not obliged to
/// have reached its enrolment through the registry). Un-enrolling on that bare
/// negative would silently switch vector search OFF for all of them — a far worse
/// regression than TC-71 itself. So the rule is POSITIVE on all three counts:
///
///   1. `_fathomdb_projection_registry` EXISTS; **and**
///   2. at least one row carries a `vector` sub-object (`vector_declared = 1`) —
///      someone actually asked for a dense arm through the registry, which is
///      precisely what identifies the affected population; **and**
///   3. NO projection satisfies [`StoredProjection::wants_vector`], i.e. none of
///      them is `searchable`.
///
/// Condition 2 is the load-bearing one. It leaves untouched a registry-governed
/// database that declares no `vector` sub-object at all but holds enrolments from
/// a pre-registry era (`slice15e_prekn_filterable` is exactly that shape). Being
/// conservative here is the correct direction: never destroy a working dense arm.
///
/// Conditions 1+2 are the SAME two `prepare_cached` `EXISTS` probes
/// [`vector_projection_declared`] opens with, so a workspace that never declared
/// a `vector` sub-object — the overwhelmingly common shape — pays nothing beyond
/// them and never reaches the typed [`load_projection_registry`] read. Condition 3
/// is delegated to [`vector_projection_declared`] verbatim rather than re-derived,
/// so the authorisation and the gate cannot drift.
fn registry_governs_an_inert_dense_arm(conn: &Connection) -> rusqlite::Result<bool> {
    // (1) the registry must EXIST. A pre-step-24 database has no registry at all
    // and is therefore not registry-governed — hands off.
    let table_exists: bool = conn
        .prepare_cached(
            "SELECT EXISTS(
                 SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = '_fathomdb_projection_registry'
             )",
        )?
        .query_row([], |row| row.get(0))?;
    if !table_exists {
        return Ok(false);
    }
    // (2) …and it must actually DECLARE a `vector` sub-object somewhere. An empty
    // or vector-less registry governs no dense arm, so any enrolment present came
    // from outside it and is not ours to remove.
    let any_vector_subobject: bool = conn
        .prepare_cached(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_projection_registry WHERE vector_declared = 1)",
        )?
        .query_row([], |row| row.get(0))?;
    if !any_vector_subobject {
        return Ok(false);
    }
    // (3) …while declaring no `searchable→vector` projection. THE predicate,
    // reused, so this can never disagree with the gate the write path applies.
    Ok(!vector_projection_declared(conn)?)
}

/// 0.8.20 Slice 21 fix-1 (codex §9 round 1 `[P2]`) — the BOOT arm of the
/// reconciliation: on every open, bring an already-enrolled inert vector kind
/// into agreement with the role-aware decision, so an affected database
/// self-heals without the user calling anything. Returns `true` iff it un-enrolled
/// something.
///
/// Authorised by [`registry_governs_an_inert_dense_arm`] (read that for the trap
/// this must not fall into), and performed by
/// [`unenrol_registry_vector_node_kinds`] — the SAME writer the drop inverse uses,
/// so `'edge_fact'` is excluded (G11 auto-registers it off the presence of an edge
/// body, independently of the projection registry) and **no embedding is deleted**.
///
/// # It mirrors the drop inverse exactly, because that inverse does nothing else
///
/// `apply_projection_config`'s drop arm is a single call to
/// [`unenrol_registry_vector_node_kinds`]: no terminal record is touched, no
/// readiness watermark is rewound, no row is un-stranded, and nothing is notified
/// (it returns `enqueued = false`). So leaving the database in "the state a drop
/// transition would have left it in" is exactly that one `DELETE`, and there is
/// no second half to mirror.
///
/// # Cheap when there is nothing to do, and idempotent
///
/// A workspace with no `vector` sub-object pays only the two cached `EXISTS`
/// probes the authorisation opens with. When the authorisation DOES fire, a third
/// cached `EXISTS` checks whether any node kind is actually enrolled, so the
/// steady state after the first healing open is a pure READ — no write
/// transaction, no `DELETE`, nothing to oscillate. `DELETE … WHERE kind <>
/// 'edge_fact'` is a single statement, hence atomic on its own; no explicit
/// transaction is opened around it.
///
/// # Placement
///
/// Runs inside `open_locked` on the writer connection, single-threaded, before
/// readers and the projection workers spawn — alongside the other boot
/// reconciliations ([`rederive_projections_on_boot`],
/// [`reconcile_vector_attr_columns`]) and therefore BEFORE
/// [`run_vector_equivalence_probe`], which is deliberate: on a database whose only
/// enrolment was the inert one, reconciling first leaves `_fathomdb_vector_kinds`
/// empty, so the probe correctly finds no dense arm to guard and the healing open
/// spends no embed calls at all.
///
/// # Not a data migration
///
/// It removes a registration row inside ONE live database to match that
/// database's own declarations. It converts no row across a version step; the
/// reconciliation itself introduces no migration.
pub(crate) fn reconcile_inert_vector_enrolments_on_boot(
    conn: &Connection,
) -> rusqlite::Result<bool> {
    if !registry_governs_an_inert_dense_arm(conn)? {
        return Ok(false);
    }
    // Nothing enrolled beyond the G11 edge arm ⇒ nothing to do. Keeps the steady
    // state a pure read instead of a no-op write transaction on every open.
    let any_node_kind: bool = conn
        .prepare_cached(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_vector_kinds WHERE kind <> 'edge_fact')",
        )?
        .query_row([], |row| row.get(0))?;
    if !any_node_kind {
        return Ok(false);
    }
    unenrol_registry_vector_node_kinds(conn)?;
    Ok(true)
}

/// 0.8.20 Slice 20c (R-20-DR remainder) — is ANY `searchable→vector` projection
/// declared in the durable registry?
///
/// This is the corpus-wide "the dense arm is live" predicate. It is corpus-wide
/// rather than per-attribute for the same reason [`derive_dense_readiness`] is:
/// Slice 15d persists the `searchable→vector` sub-object but defers building any
/// per-attribute embedding, so every declared vector projection is served by the
/// ONE engine vector pipeline. When per-attribute embedding lands, this is where
/// the scoping goes — the same seam as readiness.
///
/// Safe on a pre-step-24 schema (the registry table is created by step 24): an
/// absent table means nothing is declared, not an error. Mirrors the guard in
/// [`load_projection_registry`], and uses `prepare_cached` because the write path
/// calls this once per un-registered-kind row.
///
/// # 0.8.20 Slice 21c (ledger `TC-71`) — it requires the `searchable` ROLE
///
/// This used to answer `EXISTS(… WHERE vector_declared = 1)`, reading the stored
/// `vector` sub-object and never the `roles` column. But the sub-object SELECTS
/// a sub-target of `searchable`; it does not confer one (exactly as `fts` does
/// not — see [`StoredProjection::wants_property_fts`]). So
/// `{roles: [filterable], vector: {}}`, which Slice 15d documents as
/// inert-but-round-trippable, turned the dense arm ON in any session with a live
/// embedder: it enrolled node kinds, backfilled the corpus, and made every later
/// write of those kinds enqueue an embedding. Wasted embed work and unexpected
/// vectors at rest for a projection meant to do nothing. The answer now comes
/// from [`StoredProjection::wants_vector`], the ONE predicate, so the three
/// gated paths cannot drift.
///
/// **This flips the forward AND inverse arms of [`apply_projection_config`] at
/// once**, which is a real semantic consequence and not an accident: demoting
/// the last `{searchable, vector}` projection to `{filterable, vector}` (or
/// dropping it while an inert `{filterable, vector}` sibling survives) now reads
/// `declared → not-declared` and therefore UN-ENROLS, where before the surviving
/// `vector_declared = 1` row masked the transition and the write path kept
/// embedding. Pinned in `tests/slice21c_vector_role_gate.rs`.
///
/// # Why the cheap `EXISTS` survives as a pre-filter
///
/// The write path calls this once per un-registered-kind row, and the
/// overwhelmingly common shape is a workspace that declared no `vector`
/// sub-object at all. `EXISTS(… vector_declared = 1)` is a NECESSARY condition
/// for [`StoredProjection::wants_vector`], so keeping it as a fast negative
/// leaves that workspace paying exactly the two cached `EXISTS` probes it paid
/// before — no typed load, no `BTreeMap`, no uncached `prepare`. Only a
/// workspace that HAS a `vector` sub-object somewhere pays the
/// [`load_projection_registry`] read, and there the registry is a handful of
/// app-declared rows; in the ordinary `searchable→vector` case the kind is
/// enrolled after the first probe and `kind_is_vector_indexed` short-circuits
/// this call entirely from then on.
pub(crate) fn vector_projection_declared(conn: &Connection) -> rusqlite::Result<bool> {
    let table_exists: bool = conn
        .prepare_cached(
            "SELECT EXISTS(
                 SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = '_fathomdb_projection_registry'
             )",
        )?
        .query_row([], |row| row.get(0))?;
    if !table_exists {
        return Ok(false);
    }
    // Fast negative: no `vector` sub-object anywhere ⇒ certainly no dense arm.
    let any_vector_subobject: bool = conn
        .prepare_cached(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_projection_registry WHERE vector_declared = 1)",
        )?
        .query_row([], |row| row.get(0))?;
    if !any_vector_subobject {
        return Ok(false);
    }
    // `roles` is persisted as a comma-joined sorted string, so it is not a
    // trustworthy SQL predicate (a `LIKE` would match a forward-compat token that
    // merely CONTAINS a role spelling). Answer through the typed registry and the
    // ONE predicate instead.
    Ok(load_projection_registry(conn)?.values().any(StoredProjection::wants_vector))
}

/// 0.8.20 Slice 20c (R-20-DR remainder) — enrol `kind` in the vector pipeline.
///
/// `INSERT OR IGNORE`, so it is idempotent and never disturbs an existing
/// registration's `profile`/`created_at`. Same statement shape the G11 edge path
/// uses for `'edge_fact'` ([`project_canonical_edge_row`]).
pub(crate) fn register_vector_kind(tx: &Connection, kind: &str) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT OR IGNORE INTO _fathomdb_vector_kinds(kind, profile, created_at)
         VALUES(?1, ?2, 0)",
        params![kind, DEFAULT_VECTOR_PROFILE],
    )?;
    Ok(())
}

/// 0.8.20 Slice 20c (R-20-DR remainder) — **the flush barrier's enqueue half**
/// (`api-surface.md` **C4** rider: `drain` is a barrier, not a trigger, so
/// deferred/backfill rows must be enqueued on the same projection runtime `drain`
/// waits on).
///
/// Runs on the caller's `configure_projections` write transaction, AFTER the
/// registry mutations, so the enrolment + re-enqueue commit atomically with the
/// declaration that caused them. Returns `true` iff work was enqueued — the
/// caller must then `notify_new_work()` (after the commit; the dispatcher opens
/// its own connection).
///
/// # The defect this closes
///
/// `project_canonical_node_row` writes a PERMANENT `'up_to_date'` terminal for
/// any row whose kind was not vector-registered *at write time*, and before this
/// slice NOTHING but the `#[doc(hidden)]` test hook ever registered a node kind
/// (`slice-G0-design.md`: `production_vector_kind_surface=[]`). So the ordinary
/// "turn the dense arm on over an existing corpus" flow — write rows, then
/// declare `searchable→vector` — left every row terminally marked done with no
/// vector and no way to get one short of an operator `rebuild`. Both
/// `drain`/`wait_for_idle` and `derive_dense_readiness` read that terminal
/// through [`connection_has_pending_projection_work`], so the corpus reported
/// `ready` while nothing would ever embed it.
///
/// # Shape (deliberately the `run_rebuild` shape, scoped)
///
/// `run_rebuild` truncates the readiness terminals and rewinds the projection
/// cursor so the scheduler re-walks the corpus. This does the same, but scoped to
/// the rows the declaration newly covers, and it does NOT truncate anything else:
///
/// 1. enrol every vector-eligible node kind present in `canonical_nodes`
///    (`row_kind IN ('leaf','coverage')` — the `index_targets_for_row_kind`
///    vector-eligibility predicate; `graph` rows are lexically searchable but
///    never embedded, so enrolling on them would silently start embedding
///    structural rows) **that the vector writer can commit**
///    ([`kind_is_vector_committable`], fix-2 / codex §9 [P1]);
/// 2. (and 3.) un-strand the rows that enrolment now covers, via
///    [`reenqueue_stranded_vector_rows`] — shared verbatim with the write path's
///    late enrolment.
///
/// # Why it is IDEMPOTENT (R-20-PR: "re-registration is a no-op")
///
/// Every step keys off *state*, not off "was this declaration new": step 1 is
/// `INSERT OR IGNORE`; steps 2-3 act only on rows that are stranded RIGHT NOW.
/// Once the backfill has been drained those rows carry vectors, so a re-apply
/// finds an empty stranded set, returns `false`, and touches neither the
/// terminals nor the cursor. No rewind, no re-embed, no spurious `embedding`
/// window.
///
/// # Not a data migration
///
/// This re-enqueues embed work inside ONE live database at the caller's request.
/// It converts no rows across a version step and introduces no migration (HITL
/// 2026-07-21; cf. TC-46's in-place vec0 reshape).
/// 0.8.20 Slice 22 (R-20-VC / **TC-67**) — the ONE scan of "which node kinds in
/// this corpus are candidates for the dense arm?".
///
/// `row_kind IN ('leaf', 'coverage')` is the `index_targets_for_row_kind` vector
/// -eligibility predicate: `graph` rows are lexically searchable but NEVER
/// embedded, so they are excluded here on a ROW-KIND axis that has nothing to do
/// with the `kind` vocabulary — including them would make TC-67 report structural
/// rows as "unsupported kinds", which is a different (and false) statement.
///
/// Extracted so [`enqueue_declared_vector_backfill`] (which enrols the
/// commit-able half) and [`unsupported_vector_kinds`] (which reports the other
/// half) partition ONE list rather than running two hand-copied queries that
/// could drift — the same TC-56 anti-drift discipline that made
/// [`kind_is_vector_committable`] delegate to [`resolve_source_type`].
///
/// `SELECT DISTINCT … ORDER BY kind` gives the caller a sorted, de-duplicated
/// list for free, which is the reported ordering.
fn vector_eligible_node_kinds(tx: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = tx.prepare(
        "SELECT DISTINCT kind FROM canonical_nodes
         WHERE row_kind IN ('leaf', 'coverage')
         ORDER BY kind",
    )?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect::<rusqlite::Result<Vec<String>>>()
}

/// 0.8.20 Slice 22 (R-20-VC / **TC-67**) — **the report that replaces the
/// silence.** The vector-eligible node kinds present in the corpus that
/// [`kind_is_vector_committable`] excludes, i.e. the exact complement of the set
/// [`enqueue_declared_vector_backfill`] enrols.
///
/// Populates [`ProjectionDelta::vector_unsupported_kinds`]. Read that field's
/// doc-comment for the naming, the state-not-diff semantics and the residual;
/// what belongs HERE is the one thing the call SITE decides:
///
/// **It is deliberately NOT gated on `dense_arm_live`.** The enrolment it mirrors
/// is (`apply_projection_config` only calls `enqueue_declared_vector_backfill`
/// with a usable dense runtime, the Q6a graceful-absent path), but this answer does not
/// depend on the session: [`resolve_source_type`]'s vocabulary is a compile-time
/// constant, so "this kind can never be embedded" is equally true with no
/// embedder attached. Gating it would hide the permanent fact behind the
/// transient one, which is the very conflation TC-67 exists to end — a
/// no-embedder caller is exactly the caller who most needs to know that
/// attaching an embedder later will still not embed these kinds.
pub(crate) fn unsupported_vector_kinds(tx: &Connection) -> rusqlite::Result<Vec<String>> {
    Ok(vector_eligible_node_kinds(tx)?
        .into_iter()
        .filter(|kind| !kind_is_vector_committable(kind))
        .collect())
}

fn enqueue_declared_vector_backfill(tx: &Connection) -> rusqlite::Result<bool> {
    if !vector_projection_declared(tx)? {
        return Ok(false);
    }

    // (1) Enrol the vector-eligible kinds the live corpus actually contains —
    // RESTRICTED to the ones the vector writer can actually commit
    // ([`kind_is_vector_committable`], fix-2 / codex §9 [P1]). Enrolling a kind
    // outside `resolve_source_type`'s locked vocabulary wedges the projection
    // worker forever and starves every other kind with it.
    //
    // 0.8.20 Slice 22 (TC-67) — the kinds this filter DROPS are what
    // [`unsupported_vector_kinds`] reports; both read the same scan through
    // [`vector_eligible_node_kinds`] so the report can never describe a
    // different set from the one actually excluded.
    let kinds = vector_eligible_node_kinds(tx)?;
    for kind in kinds.iter().filter(|kind| kind_is_vector_committable(kind)) {
        register_vector_kind(tx, kind)?;
    }

    // (2)+(3) Un-strand the rows the new enrolment now covers.
    reenqueue_stranded_vector_rows(tx)
}

/// Enrol and requeue a durable vector declaration during a safe open. The
/// prospective-equivalence guard runs before this function; this function owns
/// the one durable transaction that makes a crash converge to either the old
/// state or a fully queued repair.
pub(crate) fn boot_graft_declared_vector_backfill(
    connection: &Connection,
) -> rusqlite::Result<bool> {
    if !vector_projection_declared(connection)? {
        return Ok(false);
    }
    connection.execute_batch("BEGIN IMMEDIATE")?;
    match enqueue_declared_vector_backfill(connection) {
        Ok(enqueued) => {
            connection.execute_batch("COMMIT")?;
            Ok(enqueued)
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

/// 0.8.20 Slice 20c — steps (2) and (3) of the declared-backfill above, as their
/// own function because **both** enrolment doors owe this treatment.
///
/// fix-2 (codex §9 [P2]): [`enqueue_declared_vector_backfill`] is the DECLARE-time
/// door; [`Engine::batch_vector_kinds_needing_enrolment`] selects the WRITE-time
/// enrolment set, and the old write-time path used to
/// enrol a kind while enqueueing only the row in its own batch. A database that
/// persisted a `searchable→vector` declaration while opened WITHOUT an embedder
/// (Q6a graceful-absent: it defers, enrolling nothing), then reopened WITH one and
/// wrote the same kind BEFORE re-applying the projection, therefore drained the new
/// row and reported `ready` while every row from the no-embedder session kept its
/// permanent `'up_to_date'` terminal and no vector. That is a FALSE READY — the
/// exact defect class R-20-DR exists to eliminate — so the two doors share ONE
/// implementation rather than one of them carrying a partial copy.
///
/// Returns `true` iff work was re-enqueued; the caller must then `notify_new_work()`
/// (after its commit — the dispatcher opens its own connection).
///
///   2. find the STRANDED rows — vector-eligible, now vector-kind-registered,
///      carrying an `'up_to_date'` terminal, and carrying NO `_fathomdb_vector_rows`
///      row — and delete their terminals so the scheduler's `terminal IS NULL`
///      predicate sees them again;
///   3. rewind the readiness watermark to just below the lowest stranded cursor, so
///      the scheduler's `write_cursor > cursor` filter reaches them.
///
/// The `_fathomdb_vector_kinds` join is what scopes this to the dense arm: a kind
/// that is not enrolled (including one that is not commit-able, per
/// [`kind_is_vector_committable`]) is not stranded — it has no dense arm to be
/// behind on.
///
/// Idempotent by construction: it acts only on rows that are stranded RIGHT NOW, so
/// once drained the set is empty, it returns `false`, and neither the terminals nor
/// the cursor are touched. A `'failed'` terminal is deliberately NOT re-enqueued
/// (the filter is `'up_to_date'`): re-enqueueing it would loop a permanently-failing
/// row forever, and the documented failure boundary is that a terminally-failed
/// embed stops being outstanding work (see [`derive_dense_readiness`]).
pub(crate) fn reenqueue_stranded_vector_rows(tx: &Connection) -> rusqlite::Result<bool> {
    // (2) The stranded set: covered by the dense arm, terminally marked done, no
    // vector. `MIN` first so a no-op apply costs one indexed probe and stops.
    let lowest_stranded: Option<u64> = tx.query_row(
        "SELECT MIN(n.write_cursor)
         FROM canonical_nodes n
         JOIN _fathomdb_vector_kinds k ON k.kind = n.kind
         JOIN _fathomdb_projection_terminal t ON t.write_cursor = n.write_cursor
         LEFT JOIN _fathomdb_vector_rows v ON v.write_cursor = n.write_cursor
         WHERE n.row_kind IN ('leaf', 'coverage')
           AND t.state = 'up_to_date'
           AND v.write_cursor IS NULL",
        [],
        |row| row.get::<_, Option<u64>>(0),
    )?;
    let Some(lowest_stranded) = lowest_stranded else {
        return Ok(false);
    };

    tx.execute(
        "DELETE FROM _fathomdb_projection_terminal
         WHERE write_cursor IN (
             SELECT n.write_cursor
             FROM canonical_nodes n
             JOIN _fathomdb_vector_kinds k ON k.kind = n.kind
             JOIN _fathomdb_projection_terminal t ON t.write_cursor = n.write_cursor
             LEFT JOIN _fathomdb_vector_rows v ON v.write_cursor = n.write_cursor
             WHERE n.row_kind IN ('leaf', 'coverage')
               AND t.state = 'up_to_date'
               AND v.write_cursor IS NULL
         )",
        [],
    )?;

    // (3) Rewind the readiness watermark just below the lowest stranded row so the
    // scheduler's `write_cursor > cursor` filter reaches it. Never move it
    // FORWARD: rows above the watermark that still hold their terminals are
    // skipped by the scheduler's `terminal IS NULL` predicate, and
    // `advance_projection_cursor` walks the watermark back up over them.
    let rewind_to = lowest_stranded.saturating_sub(1);
    if load_projection_cursor(tx)? > rewind_to {
        store_projection_cursor(tx, rewind_to)?;
    }
    Ok(true)
}

/// 0.8.20 Slice 15d (R-20-PR, Q5) — BOOT re-derive: the engine `ProjectionSpec`
/// is a DERIVED cache, re-driven idempotently on boot. For every persisted
/// registry declaration, clear + backfill its EAV / property-FTS rows from the
/// canonical nodes — so a DB whose registry row survives but whose projection
/// rows are missing/partial (a crash window, a restored registry) CONVERGES on
/// the next open. A no-op (single empty-table read) when no projections are
/// declared — which is every pre-`configure_projections` DB. Runs on the writer
/// connection, single-threaded, before readers spawn.
pub(crate) fn rederive_projections_on_boot(conn: &Connection) -> rusqlite::Result<()> {
    let registry = load_projection_registry(conn)?;
    if registry.is_empty() {
        return Ok(());
    }
    let persisted = projection_registry_cache_snapshot(conn)?;
    let expected = expected_projection_registry_cache_snapshot(conn, &registry)?;
    if persisted == expected {
        return Ok(());
    }
    conn.execute_batch("BEGIN IMMEDIATE")?;
    let result = (|| {
        for (name, stored) in &registry {
            clear_attribute_projection(conn, name)?;
            backfill_attribute(conn, name, stored)?;
        }
        Ok(())
    })();
    match result {
        Ok(()) => conn.execute_batch("COMMIT"),
        Err(err) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(err)
        }
    }
}

type ProjectionAttributeCacheRow = (i64, String, Option<String>);

type ProjectionPropertyFtsCacheRow = (i64, String, String);

type ProjectionRegistryCacheSnapshot =
    (Vec<ProjectionAttributeCacheRow>, Vec<ProjectionPropertyFtsCacheRow>);

fn expected_projection_registry_cache_snapshot(
    conn: &Connection,
    registry: &BTreeMap<String, StoredProjection>,
) -> rusqlite::Result<ProjectionRegistryCacheSnapshot> {
    let nodes = {
        let mut statement = conn.prepare(
            "SELECT write_cursor,body FROM canonical_nodes \
             WHERE superseded_at IS NULL AND state='active' ORDER BY write_cursor",
        )?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    let mut attributes = Vec::new();
    let mut property_fts = Vec::new();
    for (name, stored) in registry {
        if !stored.wants_eav() {
            continue;
        }
        for (cursor, body) in &nodes {
            if let Some(value) = extract_scalar_attribute(conn, body, name, stored)? {
                attributes.push((*cursor, name.clone(), Some(value.clone())));
                if stored.wants_property_fts() {
                    property_fts.push((*cursor, name.clone(), value));
                }
            }
        }
    }
    attributes.sort_by(|left, right| {
        left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)).then_with(|| left.2.cmp(&right.2))
    });
    property_fts.sort_by(|left, right| {
        left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)).then_with(|| left.2.cmp(&right.2))
    });
    Ok((attributes, property_fts))
}

fn projection_registry_cache_snapshot(
    conn: &Connection,
) -> rusqlite::Result<ProjectionRegistryCacheSnapshot> {
    let attributes = {
        let mut statement = conn.prepare(
            "SELECT write_cursor,attr_name,attr_value FROM canonical_attributes \
             ORDER BY write_cursor,CAST(attr_name AS BLOB),CAST(attr_value AS BLOB)",
        )?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    let property_fts = {
        let mut statement = conn.prepare(
            "SELECT write_cursor,attr_name,attr_value FROM property_search_index \
             ORDER BY write_cursor,CAST(attr_name AS BLOB),CAST(attr_value AS BLOB)",
        )?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    Ok((attributes, property_fts))
}
