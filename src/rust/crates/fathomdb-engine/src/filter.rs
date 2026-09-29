use crate::errors::EngineError;
use crate::projection_registry::{encode_attr_vec0_present, load_projection_registry};
use crate::search_types::TOP_K_BIT_CANDIDATES;
use crate::vector_storage::{attr_vec0_column, resolve_source_type};
use crate::ProjectionRole;
use rusqlite::{params, Connection, OptionalExtension};

// ===== G4 filter grammar types (Slice 35) ===============================

/// G4 (Slice 35) — scalar value for [`Predicate`] comparisons.
///
/// Shared vocabulary with G10 — defined once at the `fathomdb-engine` crate
/// root so reserved-gap 37 (full G4↔G10 unification) can import it without a
/// path change. Derives `Clone, Debug, PartialEq` per the ADR contract
/// (D-F1 exhaustiveness: exactly `{Text, Integer, Bool}`).
#[derive(Clone, Debug, PartialEq)]
pub enum ScalarValue {
    Text(String),
    Integer(i64),
    Bool(bool),
}

/// G4 (Slice 35) — comparison operator for [`Predicate::JsonPathCompare`].
///
/// Shared vocabulary (same crate-root export as `ScalarValue`). Closed
/// enum: `{Gt, Gte, Lt, Lte}` per D-F1. Derives `Clone, Debug, PartialEq`.
#[derive(Clone, Debug, PartialEq)]
pub enum ComparisonOp {
    Gt,
    Gte,
    Lt,
    Lte,
}

/// Allowed JSON paths for [`Predicate`] constructors. The SQL compilation in
/// [`Engine::read_list`] uses the **allowlist constant** (a server-side literal),
/// never the caller-supplied string, so only paths in this set reach
/// `json_extract`. Callers receive [`EngineError::InvalidFilter`] for any
/// non-allowlisted path — no passthrough, no panic.
///
/// To extend: add an entry here. No API change is needed; the constructor
/// accepts the new path string once it appears in this array.
pub(crate) const PREDICATE_PATH_ALLOWLIST: &[&str] =
    &["$.status", "$.priority", "$.tags", "$.kind", "$.created_at", "$.action_kind"];

/// G4 (Slice 35) — closed typed predicate for [`Engine::read_list`] filter.
///
/// Exactly two variants per ADR D-F1 (`{JsonPathEq, JsonPathCompare}`).
/// The fused variants (`JsonPathFused*`) and all `*_unchecked` builders are
/// explicitly EXCLUDED (ADR D-F2). Use the validated constructors
/// [`Predicate::json_path_eq`] / [`Predicate::json_path_compare`]; they
/// enforce the path allowlist at construction time.
///
/// Multiple predicates in [`Engine::read_list`] are combined by implicit AND
/// (D-F5). Compilation target: `json_extract(body, '$.field') <op> ?` with
/// a bound parameter (never interpolated — injection-safe per D-F4).
#[derive(Clone, Debug, PartialEq)]
pub enum Predicate {
    /// `json_extract(body, path) = ?` (equality).
    JsonPathEq { path: String, value: ScalarValue },
    /// `json_extract(body, path) <op> ?` (inequality).
    JsonPathCompare { path: String, op: ComparisonOp, value: ScalarValue },
}

/// G10 — closed metadata filter for [`Engine::search_filtered`] (Slice 10).
///
/// All fields are optional; a `None` field imposes no constraint, and an
/// all-`None` filter (or `None` filter) is the unfiltered path whose phase-1 SQL
/// is byte-identical to 0.7.2. This is a **closed struct**, not an open filter
/// DSL (ADR-0.8.0-agent-memory-retrieval-and-identity Q1); the filter-grammar /
/// `list` decision stays a later-slice concern.
///
/// `created_after` is a `created_at >= bound` lower bound in unix seconds.
/// `status` is wired through to the vec0 `status` metadata column. vec0 TEXT
/// metadata columns are **NOT NULL-able**, so the "no real population yet" state
/// is an **empty-string sentinel** `''` (a forced deviation from the planned
/// "NULL plumbing"; a real population source is reserved-gap candidate 13). A
/// `status = Some("open")`-style filter therefore prunes every row until that
/// population slice lands.
// 0.8.20 Slice 15e fix-2 (Finding 2) — `#[non_exhaustive]`: the `attributes`
// field was added additively in 0.8.20. Marking the struct non-exhaustive means
// EXTERNAL crates can no longer use a struct literal `SearchFilter { .. }` and
// must go through `..Default::default()` (or a constructor), so a FUTURE field
// add is not a source break for them. Internal (in-workspace) construction is
// unaffected — `#[non_exhaustive]` only constrains other crates — and every
// in-crate literal already spreads `..Default::default()`. Governed-surface
// status: PROPOSED / NOT SIGNED.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct SearchFilter {
    pub source_type: Option<String>,
    pub kind: Option<String>,
    pub created_after: Option<i64>,
    pub status: Option<String>,
    /// 0.8.20 Slice 15e (R-20-PR, ADR-0.8.11 D3) — declared-`filterable`-attribute
    /// equality predicates, each `(attribute_name, value)`. Lowered into the
    /// **indexed pre-KNN** vec0 metadata column `attr_<hex>` by
    /// [`vector_filter_clause`] (NOT a post-KNN `json_extract`). Empty ⇒ the
    /// byte-identical unfiltered path is preserved. `attribute_name` is the
    /// registry projection name; the encoded column is derived by
    /// [`attr_vec0_column`].
    pub attributes: Vec<(String, String)>,
}

// ===== 0.8.11 Slice 40 (#17) — unified filter grammar (G4 + G10) =========

/// 0.8.11 Slice 40 (#17) — a single closed `FilterTerm` of the **unified**
/// filter grammar (ADR-0.8.11-filter-grammar-unification, Option A; closes
/// reserved-gap 37). Exactly **five** variants: the four G10 shorthand metadata
/// fields (`SourceType`/`Kind`/`CreatedAfter`/`Status`) plus the general G4
/// json-path [`Predicate`] (`Json`). The shorthand fields are dedicated typed
/// variants — NOT `Json(Predicate)` over `$.source_type` etc. — precisely so the
/// vec0 search backend can lower them to the *indexed* pre-KNN metadata columns
/// while typed-rejecting an arbitrary `Json` term (D3: no demotion to post-KNN
/// `json_extract`).
///
/// The grammar stays **closed** (inherits ADR-0.8.0 D-F1/D-F2/D-F4/D-F5): no
/// DSL, no caller SQL, no `JsonPathFused*`, no `*_unchecked`, no OR/nesting
/// (implicit AND only); `Json` terms are built ONLY via the validated
/// [`Predicate::json_path_eq`] / [`Predicate::json_path_compare`] constructors
/// (path allowlist enforced at construction). The shipped `ScalarValue` /
/// `ComparisonOp` / `Predicate` vocabulary is reused verbatim — no new grammar.
#[derive(Clone, Debug, PartialEq)]
pub enum FilterTerm {
    /// vec0 partition-key metadata column `source_type` (pre-KNN). On
    /// `read.list` it **constant-folds** against `resolve_source_type(kind)`
    /// (the column does not exist in `canonical_nodes`).
    SourceType(String),
    /// `kind` — the vec0 metadata column (pre-KNN). On `read.list` it
    /// constant-folds against the partition `kind` argument (D1 impl decision:
    /// constant-fold, the simpler total option vs a redundant column clause).
    Kind(String),
    /// `created_at >= bound` (unix seconds). vec0 metadata column (pre-KNN);
    /// lowers to `json_extract(body,'$.created_at') >= ?` on `read.list`.
    CreatedAfter(i64),
    /// vec0 metadata column `status` (pre-KNN); lowers to
    /// `json_extract(body,'$.status') = ?` on `read.list`.
    Status(String),
    /// The general G4 json-path predicate (unchanged shipped grammar). Resolves
    /// **only** on the `read.list` (canonical_nodes) backend; **typed-rejected**
    /// on `search_filtered` because it would require a post-KNN `json_extract`
    /// that defeats the indexed pre-KNN filter (D3 no-demotion guarantee).
    Json(Predicate),
}

/// 0.8.11 Slice 40 (#17) — the unified closed `Filter` contract. ONE superset
/// type with implicit-AND [`FilterTerm`]s, dispatched to one of **two** internal
/// compilation backends (Option A — the TYPE unifies, the COMPILATION
/// dispatches): the vec0-metadata indexed pre-KNN `WHERE` for `search_filtered`,
/// and `json_extract` over `canonical_nodes.body` for `read.list`. The shipped
/// `SearchFilter` (G10) and `Predicate` lists (G4) re-express as sugar that
/// lowers into this type (D4); the `filter=None` byte-identical-0.7.2-SQL pin is
/// preserved because the vec0 lowering routes back through the shipped
/// `vector_filter_clause` compilation verbatim.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Filter {
    /// AND-combined terms (implicit AND, inherits D-F5). Empty = unfiltered.
    pub terms: Vec<FilterTerm>,
}

impl Predicate {
    /// Construct a `JsonPathEq` predicate with allowlist validation.
    ///
    /// Returns [`EngineError::InvalidFilter`] if `path` is not in
    /// [`PREDICATE_PATH_ALLOWLIST`]; never panics on bad input.
    pub fn json_path_eq(path: impl Into<String>, value: ScalarValue) -> Result<Self, EngineError> {
        let path = path.into();
        if !PREDICATE_PATH_ALLOWLIST.contains(&path.as_str()) {
            return Err(EngineError::InvalidFilter {
                reason: format!("path '{path}' is not in the predicate path allowlist"),
            });
        }
        Ok(Self::JsonPathEq { path, value })
    }

    /// Construct a `JsonPathCompare` predicate with allowlist validation.
    ///
    /// Returns [`EngineError::InvalidFilter`] if `path` is not in
    /// [`PREDICATE_PATH_ALLOWLIST`]; never panics on bad input.
    pub fn json_path_compare(
        path: impl Into<String>,
        op: ComparisonOp,
        value: ScalarValue,
    ) -> Result<Self, EngineError> {
        let path = path.into();
        if !PREDICATE_PATH_ALLOWLIST.contains(&path.as_str()) {
            return Err(EngineError::InvalidFilter {
                reason: format!("path '{path}' is not in the predicate path allowlist"),
            });
        }
        Ok(Self::JsonPathCompare { path, op, value })
    }

    /// Return the validated path string for use in SQL compilation.
    /// This always returns a path that is in `PREDICATE_PATH_ALLOWLIST`.
    pub(crate) fn path(&self) -> &str {
        match self {
            Self::JsonPathEq { path, .. } => path.as_str(),
            Self::JsonPathCompare { path, .. } => path.as_str(),
        }
    }

    /// Compile this predicate to a SQL WHERE clause fragment.
    /// The path is validated at construction time and is always an allowlist
    /// constant — never the raw caller-supplied string.
    pub(crate) fn to_sql_clause(&self, param_idx: usize) -> String {
        // The path is already validated against the allowlist at construction.
        // We use the allowlist entry (the stored path) directly as a SQL literal.
        // The VALUE is always a bound `?` parameter (injection-safe).
        //
        // Type guards prevent cross-type matches caused by SQLite's json_extract
        // coercing JSON booleans to integer 1/0:
        //   - Bool predicates: AND json_type IN ('true', 'false') — exclude integers
        //   - Integer predicates: AND json_type = 'integer' — exclude booleans
        // Text predicates need no guard: json_extract returns TEXT for strings and
        // the coercion never conflates TEXT with integer/bool.
        let path = Predicate::path(self);
        match self {
            Self::JsonPathEq { value, .. } => match value {
                ScalarValue::Bool(_) => format!(
                    "json_extract(body, '{path}') = ?{param_idx} \
                     AND json_type(body, '{path}') IN ('true', 'false')"
                ),
                ScalarValue::Integer(_) => format!(
                    "json_extract(body, '{path}') = ?{param_idx} \
                     AND json_type(body, '{path}') = 'integer'"
                ),
                ScalarValue::Text(_) => {
                    format!("json_extract(body, '{path}') = ?{param_idx}")
                }
            },
            Self::JsonPathCompare { op, value, .. } => {
                let op_str = match op {
                    ComparisonOp::Gt => ">",
                    ComparisonOp::Gte => ">=",
                    ComparisonOp::Lt => "<",
                    ComparisonOp::Lte => "<=",
                };
                match value {
                    ScalarValue::Bool(_) => format!(
                        "json_extract(body, '{path}') {op_str} ?{param_idx} \
                         AND json_type(body, '{path}') IN ('true', 'false')"
                    ),
                    ScalarValue::Integer(_) => format!(
                        "json_extract(body, '{path}') {op_str} ?{param_idx} \
                         AND json_type(body, '{path}') = 'integer'"
                    ),
                    ScalarValue::Text(_) => format!(
                        "json_extract(body, '{path}') {op_str} ?{param_idx} \
                         AND json_type(body, '{path}') = 'text'"
                    ),
                }
            }
        }
    }

    /// Bind the value of this predicate as a rusqlite parameter.
    pub(crate) fn bind_value(&self) -> rusqlite::types::Value {
        let value = match self {
            Self::JsonPathEq { value, .. } => value,
            Self::JsonPathCompare { value, .. } => value,
        };
        match value {
            ScalarValue::Text(s) => rusqlite::types::Value::Text(s.clone()),
            ScalarValue::Integer(i) => rusqlite::types::Value::Integer(*i),
            ScalarValue::Bool(b) => rusqlite::types::Value::Integer(i64::from(*b)),
        }
    }
}

impl SearchFilter {
    /// True when no field constrains the search — equivalent to `None`. Used to
    /// keep the unfiltered code path (and its byte-identical SQL) on the
    /// all-`None` struct.
    pub(crate) fn is_unfiltered(&self) -> bool {
        self.source_type.is_none()
            && self.kind.is_none()
            && self.created_after.is_none()
            && self.status.is_none()
            && self.attributes.is_empty()
    }
}

impl TryFrom<&SearchFilter> for Filter {
    type Error = EngineError;

    /// D4 sugar lowering — the shipped G10 [`SearchFilter`] re-expressed as the
    /// unified [`Filter`]. Attribute equality is intentionally not part of the
    /// unified grammar, so it is refused rather than silently discarded. Field
    /// → term uses canonical order (`source_type`, `kind`, `created_after`,
    /// `status`) so an attribute-free round-trip stays byte-identical.
    fn try_from(sf: &SearchFilter) -> Result<Self, Self::Error> {
        if !sf.attributes.is_empty() {
            return Err(EngineError::InvalidFilter {
                reason:
                    "projected attribute predicates are not supported by the unified Filter grammar"
                        .to_string(),
            });
        }
        let mut terms = Vec::new();
        if let Some(s) = &sf.source_type {
            terms.push(FilterTerm::SourceType(s.clone()));
        }
        if let Some(k) = &sf.kind {
            terms.push(FilterTerm::Kind(k.clone()));
        }
        if let Some(c) = sf.created_after {
            terms.push(FilterTerm::CreatedAfter(c));
        }
        if let Some(s) = &sf.status {
            terms.push(FilterTerm::Status(s.clone()));
        }
        Ok(Filter { terms })
    }
}

impl Filter {
    /// Backend dispatch for `search_filtered` (vec0 — indexed pre-KNN). Lowers
    /// the metadata subset `{SourceType, Kind, CreatedAfter, Status}` back into a
    /// [`SearchFilter`] (which the shipped `vector_filter_clause` compiles to the
    /// pre-KNN `WHERE`), and **typed-rejects** a [`FilterTerm::Json`] term with
    /// [`EngineError::InvalidFilter`] — the explicit no-demotion guarantee (D3).
    /// Field-by-variant assignment makes the output canonical-order-independent
    /// of `terms` ordering (hand-built router filters included). A later
    /// duplicate metadata term overwrites the earlier (last-wins).
    pub fn to_search_filter(&self) -> Result<SearchFilter, EngineError> {
        let mut sf = SearchFilter::default();
        for term in &self.terms {
            match term {
                FilterTerm::SourceType(s) => sf.source_type = Some(s.clone()),
                FilterTerm::Kind(k) => sf.kind = Some(k.clone()),
                FilterTerm::CreatedAfter(c) => sf.created_after = Some(*c),
                FilterTerm::Status(s) => sf.status = Some(s.clone()),
                FilterTerm::Json(_) => {
                    return Err(EngineError::InvalidFilter {
                        reason: "arbitrary json-path predicate not supported on search_filtered; \
                                 it would require a post-KNN json_extract that defeats the \
                                 indexed pre-KNN filter (ADR-0.8.11 D3 no-demotion guarantee)"
                            .to_string(),
                    });
                }
            }
        }
        Ok(sf)
    }

    /// Backend dispatch for `read.list` (canonical_nodes — `json_extract`). The
    /// full set resolves here. Returns:
    /// - `Ok(Some(preds))` — the implicit-AND [`Predicate`] list to run; or
    /// - `Ok(None)` — a constant-folded **guaranteed-empty** result (a `Kind` or
    ///   `SourceType` term that cannot match this partition), so the caller
    ///   returns an empty `Vec` without touching SQL; or
    /// - `Err(InvalidFilter)` — a non-allowlisted path (defense-in-depth; the
    ///   shorthand lowerings only ever use allowlisted paths).
    ///
    /// Lowering (D3): `Json(p)` → `p`; `Status(s)` →
    /// `json_path_eq("$.status", Text(s))`; `CreatedAfter(b)` →
    /// `json_path_compare("$.created_at", Gte, Integer(b))`; `Kind(k)` →
    /// constant-fold vs the partition `kind` arg (no-op if equal, empty if not);
    /// `SourceType(s)` → constant-fold vs `resolve_source_type(kind)` (no-op if
    /// equal, empty otherwise — the column does not exist in `body`).
    pub(crate) fn lower_for_read_list(
        &self,
        kind: &str,
    ) -> Result<Option<Vec<Predicate>>, EngineError> {
        let mut preds = Vec::new();
        for term in &self.terms {
            match term {
                FilterTerm::Json(p) => preds.push(p.clone()),
                FilterTerm::Status(s) => {
                    preds.push(Predicate::json_path_eq("$.status", ScalarValue::Text(s.clone()))?);
                }
                FilterTerm::CreatedAfter(b) => {
                    preds.push(Predicate::json_path_compare(
                        "$.created_at",
                        ComparisonOp::Gte,
                        ScalarValue::Integer(*b),
                    )?);
                }
                FilterTerm::Kind(k) => {
                    // Constant-fold vs the partition argument (D1 impl decision).
                    if k != kind {
                        return Ok(None);
                    }
                }
                FilterTerm::SourceType(s) => {
                    // source_type is NOT a canonical_nodes column; it is a pure
                    // function of `kind`. Constant-fold (D2/D3).
                    match resolve_source_type(kind) {
                        Ok(resolved) if resolved == s.as_str() => {}
                        _ => return Ok(None),
                    }
                }
            }
        }
        Ok(Some(preds))
    }

    /// 0.8.11 Slice 40 — test seam: expose the vec0 backend dispatch so the
    /// unification suite can pin the typed-rejection (RED→GREEN) and that a
    /// metadata-only Filter lowers losslessly. Returns the lowered
    /// [`SearchFilter`] (or `InvalidFilter` for a `Json` term).
    #[doc(hidden)]
    pub fn to_search_filter_for_test(&self) -> Result<SearchFilter, EngineError> {
        self.to_search_filter()
    }

    /// 0.8.11 Slice 40 — test seam: expose the `read.list` backend lowering so
    /// the unification suite can pin total dispatch incl. the `SourceType`/`Kind`
    /// constant-folds. `Ok(None)` == constant-folded-empty.
    #[doc(hidden)]
    pub fn lower_for_read_list_for_test(
        &self,
        kind: &str,
    ) -> Result<Option<Vec<Predicate>>, EngineError> {
        self.lower_for_read_list(kind)
    }
}

/// 0.8.20 Slice 15e fix-2 finding 1 [P2] + keystone closeout fix-3 (codex §9 [P2],
/// TOCTOU) — reject a search filter that names an attribute with NO declared
/// `filterable` projection, ON THE READER'S OWN SNAPSHOT, before the vec0 SQL is
/// built.
///
/// The vector arm lowers each `filter.attributes` term to `AND attr_<hex>=?`
/// against a vec0 metadata column that exists ONLY for a declared `filterable`
/// projection (the reshape in [`reconcile_vector_attr_columns`] tracks exactly the
/// registry's `filterable` set; see `desired_vector_attr_columns`). A name that
/// is not a declared `filterable` projection therefore has no column: the vec0 KNN
/// would fail with `no such column` (surfacing as an opaque `Storage` error),
/// while the FTS arm ([`hit_attributes_pass_filter`]) would silently no-match. That
/// divergence violates ADR-0.8.11 D3 (every filter term has a DEFINED, arm-uniform
/// outcome).
///
/// fix-3 snapshot contract: this is called from [`read_search_in_tx`] INSIDE the
/// reader's `DEFERRED` transaction, so the `_fathomdb_projection_registry` it reads
/// and the `vector_default` columns [`vector_filter_clause`] compiles against are
/// the SAME WAL snapshot. A concurrent `configure_projections` DROP either commits
/// before this snapshot is pinned (then BOTH the registry and the vec0 columns show
/// the attribute gone ⇒ a consistent `InvalidFilter`) or after (then it is invisible
/// to this transaction and BOTH still show it declared ⇒ the query runs against the
/// column it validated). The registry's `filterable` set is the arm-INDEPENDENT
/// authority (correct even with no embedder / no `vector_default`, where a
/// declared-`filterable` term still filters legitimately via the row-owned
/// `canonical_attributes` EAV store). The caller re-raises
/// [`SnapshotFilterError::InvalidFilter`] as the EXISTING typed
/// [`EngineError::InvalidFilter`], so both arms see the SAME rejection because it is
/// raised before either runs.
pub(crate) fn validate_filter_attributes_on_snapshot(
    conn: &Connection,
    filter: &SearchFilter,
) -> Result<(), SnapshotFilterError> {
    if filter.attributes.is_empty() {
        return Ok(());
    }
    // `?` maps a registry-read failure to `SnapshotFilterError::Sqlite` (unchanged
    // `Storage` semantics for a genuine backend fault) via the `From` impl.
    let registry = load_projection_registry(conn)?;
    for (name, _value) in &filter.attributes {
        let declared_filterable =
            registry.get(name).is_some_and(|s| s.roles.contains(&ProjectionRole::Filterable));
        if !declared_filterable {
            return Err(SnapshotFilterError::InvalidFilter(format!(
                "filter attribute {name:?} is not a declared `filterable` projection; \
                 declare it via configure_projections before filtering on it"
            )));
        }
    }
    Ok(())
}

pub(crate) enum SnapshotFilterError {
    Sqlite(rusqlite::Error),
    InvalidFilter(String),
}

impl From<rusqlite::Error> for SnapshotFilterError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

/// G10 — the `AND col=?n` predicate fragment appended to the phase-1 candidates
/// `WHERE` for the present filter fields. Placeholders are numbered from `?3`
/// (`?1` = sign-quant query, `?2` = f32 rerank query). Field order is canonical
/// (`source_type`, `kind`, `created_after`, `status`), THEN the Slice-15e
/// `filterable`-attribute predicates (`attr_<hex>=?n`) in `attributes` order, and
/// is mirrored exactly by [`vector_filter_values`]. Empty for `None`/all-`None`
/// (byte-identity path).
fn vector_filter_clause(filter: Option<&SearchFilter>) -> String {
    let Some(filter) = filter else {
        return String::new();
    };
    if filter.is_unfiltered() {
        return String::new();
    }
    // 0.8.20 Slice 15e — the attribute predicates encode the (arbitrary,
    // possibly space/unicode-bearing) registry name into the byte-safe
    // `attr_<hex>` column vec0 accepts (vec0 rejects quoted identifiers). Owned
    // strings so they live past the closure; the shipped metadata columns are
    // static `&str`.
    let mut cols: Vec<(String, &str)> = Vec::new();
    if filter.source_type.is_some() {
        cols.push(("source_type".to_string(), "="));
    }
    if filter.kind.is_some() {
        cols.push(("kind".to_string(), "="));
    }
    if filter.created_after.is_some() {
        cols.push(("created_at".to_string(), ">="));
    }
    if filter.status.is_some() {
        cols.push(("status".to_string(), "="));
    }
    for (name, _value) in &filter.attributes {
        cols.push((attr_vec0_column(name), "="));
    }
    let mut clause = String::new();
    for (i, (col, op)) in cols.iter().enumerate() {
        clause.push_str(&format!(" AND {col}{op}?{}", i + 3));
    }
    clause
}

/// G10 — the bound values for the present filter fields, in the SAME canonical
/// order as [`vector_filter_clause`] so placeholder `?{n}` lines up with value
/// `n-3`.
pub(crate) fn vector_filter_values(filter: Option<&SearchFilter>) -> Vec<rusqlite::types::Value> {
    use rusqlite::types::Value;
    let mut out = Vec::new();
    let Some(filter) = filter else {
        return out;
    };
    if filter.is_unfiltered() {
        return out;
    }
    if let Some(s) = &filter.source_type {
        out.push(Value::Text(s.clone()));
    }
    if let Some(s) = &filter.kind {
        out.push(Value::Text(s.clone()));
    }
    if let Some(c) = filter.created_after {
        out.push(Value::Integer(c));
    }
    if let Some(s) = &filter.status {
        out.push(Value::Text(s.clone()));
    }
    // 0.8.20 Slice 15e — attribute values, in the SAME order the clause appended
    // the `attr_<hex>` columns (after the four metadata fields). fix-3 [P2] — the
    // filter value is encoded `\x01 || V` to match the encoded PRESENT column
    // value, so `attr_<hex> = enc("")` matches present-empty but NEVER the
    // `''`-absent rows.
    for (_name, value) in &filter.attributes {
        out.push(Value::Text(encode_attr_vec0_present(value)));
    }
    out
}

/// G10 — build the single phase-1 candidates statement. With `filter=None` (or
/// all-`None`) the `{filter_clause}` is empty and the SQL is **byte-identical to
/// 0.7.2** (the documented behavior-compat invariant; pinned by
/// `pr_g10_filtered_knn.rs`). The KNN form (`ORDER BY distance LIMIT top_k`, no
/// `k=`) is preserved.
pub(crate) fn build_vector_phase1_sql(filter: Option<&SearchFilter>, final_limit: usize) -> String {
    let filter_clause = vector_filter_clause(filter);
    format!(
        "WITH candidates AS (
                     SELECT rowid
                     FROM vector_default
                     WHERE embedding_bin MATCH vec_quantize_binary(vec_f32(?1)){filter_clause}
                     ORDER BY distance
                     LIMIT {top_k}
                 )
                 SELECT c.rowid, vec_distance_l2(v.embedding, vec_f32(?2)) AS l2
                 FROM candidates c
                 JOIN vector_default v ON v.rowid = c.rowid
                 ORDER BY l2
                 LIMIT {final_limit}",
        top_k = TOP_K_BIT_CANDIDATES,
    )
}

/// Compile node eligibility into SQL that executes before ranking or limits.
pub(crate) fn append_node_eligibility_sql(
    filter: Option<&SearchFilter>,
    alias: &str,
    params: &mut Vec<rusqlite::types::Value>,
) -> String {
    use rusqlite::types::Value;
    let Some(filter) = filter else {
        return String::new();
    };
    let mut sql = String::new();
    let mut bind = |value: Value| {
        params.push(value);
        params.len()
    };
    if let Some(value) = &filter.source_type {
        let index = bind(Value::Text(value.clone()));
        sql.push_str(&format!(
            " AND CASE {alias}.kind WHEN 'email' THEN 'email' WHEN 'article' THEN 'article' \
             WHEN 'paper' THEN 'paper' WHEN 'meeting' THEN 'meeting' WHEN 'note' THEN 'note' \
             WHEN 'todo' THEN 'todo' WHEN 'doc' THEN 'article' END = ?{index}"
        ));
    }
    if let Some(value) = &filter.kind {
        let index = bind(Value::Text(value.clone()));
        sql.push_str(&format!(" AND {alias}.kind = ?{index}"));
    }
    if let Some(value) = filter.created_after {
        let index = bind(Value::Integer(value));
        sql.push_str(&format!(
            " AND EXISTS(SELECT 1 FROM vector_default vm WHERE vm.rowid = {alias}.write_cursor \
             AND vm.created_at >= ?{index})"
        ));
    }
    if let Some(value) = &filter.status {
        let index = bind(Value::Text(value.clone()));
        sql.push_str(&format!(
            " AND EXISTS(SELECT 1 FROM vector_default vm WHERE vm.rowid = {alias}.write_cursor \
             AND vm.status = ?{index})"
        ));
    }
    for (ordinal, (name, value)) in filter.attributes.iter().enumerate() {
        let name_index = bind(Value::Text(name.clone()));
        let value_index = bind(Value::Text(value.clone()));
        sql.push_str(&format!(
            " AND EXISTS(SELECT 1 FROM canonical_attributes ca{ordinal} \
             WHERE ca{ordinal}.write_cursor = {alias}.write_cursor \
             AND ca{ordinal}.attr_name = ?{name_index} \
             AND ca{ordinal}.attr_value = ?{value_index})"
        ));
    }
    sql
}

/// Compile edge-hit eligibility before edge FTS ranking.
pub(crate) fn append_edge_eligibility_sql(
    filter: Option<&SearchFilter>,
    alias: &str,
    params: &mut Vec<rusqlite::types::Value>,
) -> String {
    use rusqlite::types::Value;
    let Some(filter) = filter else {
        return String::new();
    };
    let mut sql = String::new();
    let mut bind = |value: Value| {
        params.push(value);
        params.len()
    };
    if let Some(value) = &filter.source_type {
        let index = bind(Value::Text(value.clone()));
        sql.push_str(&format!(" AND 'edge_fact' = ?{index}"));
    }
    if let Some(value) = &filter.kind {
        let index = bind(Value::Text(value.clone()));
        sql.push_str(&format!(" AND {alias}.kind = ?{index}"));
    }
    if let Some(value) = filter.created_after {
        let index = bind(Value::Integer(value));
        sql.push_str(&format!(
            " AND EXISTS(SELECT 1 FROM vector_default vm WHERE vm.rowid = {alias}.write_cursor \
             AND vm.created_at >= ?{index})"
        ));
    }
    if let Some(value) = &filter.status {
        let index = bind(Value::Text(value.clone()));
        sql.push_str(&format!(
            " AND EXISTS(SELECT 1 FROM vector_default vm WHERE vm.rowid = {alias}.write_cursor \
             AND vm.status = ?{index})"
        ));
    }
    for (ordinal, (name, value)) in filter.attributes.iter().enumerate() {
        let name_index = bind(Value::Text(name.clone()));
        let value_index = bind(Value::Text(value.clone()));
        sql.push_str(&format!(
            " AND EXISTS(SELECT 1 FROM canonical_attributes eca{ordinal} \
             WHERE eca{ordinal}.write_cursor = {alias}.write_cursor \
             AND eca{ordinal}.attr_name = ?{name_index} \
             AND eca{ordinal}.attr_value = ?{value_index})"
        ));
    }
    sql
}

pub(crate) fn body_fts_rank_sql(
    validity: &str,
    lifecycle: &str,
    eligibility: &str,
    limit_clause: &str,
) -> String {
    format!(
        "SELECT search_index.body, search_index.kind, search_index.write_cursor, \
         bm25(search_index), cn.logical_id, cn.source_id FROM search_index \
         LEFT JOIN canonical_nodes cn ON cn.write_cursor = search_index.write_cursor \
         WHERE search_index MATCH ?1 \
           AND cn.superseded_at IS NULL \
           AND (cn.state = 'active' OR cn.state IS NULL)\
           {validity}{lifecycle}{eligibility} \
         ORDER BY bm25(search_index), search_index.write_cursor{limit_clause}"
    )
}

pub(crate) fn edge_fts_rank_sql(validity: &str, eligibility: &str) -> String {
    format!(
        "SELECT sei.body, sei.kind, sei.write_cursor, bm25(search_index_edges), \
         ce.logical_id, ce.source_id \
         FROM search_index_edges sei \
         JOIN canonical_edges ce ON ce.write_cursor = sei.write_cursor \
         WHERE search_index_edges MATCH ?1 \
           AND ce.superseded_at IS NULL{validity}{eligibility} \
         ORDER BY bm25(search_index_edges), sei.write_cursor"
    )
}

pub(crate) fn property_fts_rank_sql(validity: &str, eligibility: &str) -> String {
    format!(
        "SELECT p.write_cursor, bm25(property_search_index), n.kind, n.body, n.logical_id, n.source_id
         FROM property_search_index p JOIN canonical_nodes n ON n.write_cursor = p.write_cursor
         WHERE p.attr_name = ?1 AND property_search_index MATCH ?2
           {validity}{eligibility}
         ORDER BY bm25(property_search_index) ASC, p.write_cursor ASC"
    )
}

/// G10 — does a text-branch hit satisfy the filter? The vector branch is
/// pruned in-SQL; the text branch is constrained here against the same metadata:
/// `kind` directly, `source_type` via [`resolve_source_type`], and
/// `created_after`/`status` from `vector_default` by `rowid == write_cursor`. A
/// text-only row absent from the vector partition cannot satisfy a
/// `created_after`/`status` predicate, so it is excluded — filtered semantic
/// search is a vector-metadata capability.
#[allow(dead_code)]
pub(crate) fn text_hit_passes_filter(
    tx: &Connection,
    id: u64,
    kind: &str,
    filter: Option<&SearchFilter>,
) -> rusqlite::Result<bool> {
    let Some(filter) = filter else {
        return Ok(true);
    };
    if filter.is_unfiltered() {
        return Ok(true);
    }
    if let Some(k) = &filter.kind {
        if kind != k {
            return Ok(false);
        }
    }
    if let Some(st) = &filter.source_type {
        match resolve_source_type(kind) {
            Ok(resolved) if resolved == st.as_str() => {}
            _ => return Ok(false),
        }
    }
    if filter.created_after.is_some() || filter.status.is_some() {
        let meta: Option<(i64, Option<String>)> = tx
            .query_row(
                "SELECT created_at, status FROM vector_default WHERE rowid = ?1 LIMIT 1",
                [id as i64],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?;
        let Some((created_at, status)) = meta else {
            // No vector-partition row: cannot satisfy a vec-metadata predicate.
            return Ok(false);
        };
        if let Some(bound) = filter.created_after {
            if created_at < bound {
                return Ok(false);
            }
        }
        if let Some(want) = &filter.status {
            if status.as_deref() != Some(want.as_str()) {
                return Ok(false);
            }
        }
    }
    // 0.8.20 Slice 15e fix-1 finding 1 [P2] — enforce the declared-`filterable`
    // attribute-equality predicates on the TEXT/FTS arm too (D3 total dispatch).
    if !hit_attributes_pass_filter(tx, id, filter)? {
        return Ok(false);
    }
    Ok(true)
}

/// 0.8.20 Slice 15e fix-1 finding 1 [P2] — do a TEXT/FTS hit's `filterable`
/// attributes satisfy `filter.attributes`?
///
/// The vector arm enforces each `(attr_name, value)` pre-KNN as `attr_<hex>=?`
/// against the vec0 metadata column. The FTS/text arm must enforce the SAME
/// equality so a hybrid RRF fusion is coherent (ADR-0.8.11 D3 — every filter term
/// has a defined outcome on EVERY surface; no arm silently ignores it). Without
/// this, a doc returned by the FTS arm that FAILS the attribute filter still
/// surfaces in a hybrid search — a false positive.
///
/// The value is read from the row-owned `canonical_attributes` EAV table (keyed
/// by the hit's `write_cursor` + `attr_name`), which Slice 15d keeps active-only
/// and populates via the SAME `extract_scalar_attribute` that fills the vec0
/// `attr_<hex>` column — so the two arms see IDENTICAL values by construction.
///
/// # fix-3 [P2]: ABSENT vs PRESENT-EMPTY
///
/// A real empty-string value (`{"status":""}`) is DISTINCT from an absent
/// attribute. On this (FTS/text) arm the distinction is ROW EXISTENCE: a present
/// attribute (even value `''`) has a `canonical_attributes` row; an absent one has
/// NONE. So a filter `("status","")` matches present-empty (row exists, RAW value
/// `''`) but NOT absent (no row), and `("status","open")` matches only the
/// `open` row. The vec0 arm reaches the SAME verdict via its `\x01`-marker
/// encoding (see `ATTR_VEC0_PRESENT_MARKER`): present-empty is the bare marker,
/// absent is `''`, so `attr_<hex> = enc("")` matches present-empty but never
/// absent. `canonical_attributes.attr_value` and `property_search_index` stay RAW.
///
/// # 0.8.20 semantics (Finding 1 → HITL ruling (A)): attribute filters are NODE-scoped
///
/// Attribute projection is `PreparedWrite::Node`-gated (see `collect_projection_jobs`
/// / `project_one_attribute`): an EDGE is never projected into
/// `canonical_attributes`, and its `vector_default` row (kind `edge_fact`) carries
/// the `''` sentinel in every `attr_<hex>` column (the async worker reads the body
/// from `canonical_nodes`, which has no row for an edge cursor). Therefore an
/// attribute filter **excludes every edge hit** — on BOTH the edge-FTS arm (this
/// helper, keyed by the edge's write_cursor, reads `''`) and the edge-vector arm
/// (the pre-KNN `attr_<hex>='…'` predicate prunes the `''`-sentinel edge row) —
/// even when the edge body itself names the attribute. This is the intended
/// 0.8.20 behaviour, pinned by `attribute_filter_excludes_edge_hits_on_both_arms`.
///
/// The reserved widening is **(D) endpoint-node filtering** (an edge passes iff its
/// endpoint node(s) satisfy the attribute predicate): **(A) is (D) with an empty
/// endpoint rule.** (B) edges-pass-through and (C) project-edge-attributes are the
/// other reserved options. None are implemented in 0.8.20 — do not add a per-query
/// flag; a widening is a deliberate, separately-governed later slice.
pub(crate) fn hit_attributes_pass_filter(
    tx: &Connection,
    id: u64,
    filter: &SearchFilter,
) -> rusqlite::Result<bool> {
    for (name, want) in &filter.attributes {
        // fix-3 [P2] — distinguish ABSENT from PRESENT-EMPTY by ROW EXISTENCE: a
        // present attribute (including one whose value is a real empty string `''`)
        // has a `canonical_attributes` row; an absent one has NONE. The outer
        // `Option` is row existence; the RAW `attr_value` is compared verbatim.
        // This mirrors the vec0 arm exactly (present-empty matches `("k","")`;
        // absent matches nothing, including `""`), so a fused hybrid search is
        // coherent. `canonical_attributes.attr_value` stays RAW (unencoded).
        let stored: Option<Option<String>> = tx
            .query_row(
                "SELECT attr_value FROM canonical_attributes \
                 WHERE write_cursor = ?1 AND attr_name = ?2 LIMIT 1",
                params![id as i64, name],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?;
        match stored {
            // Present (row exists) and the RAW value equals the filter value.
            Some(Some(v)) if v.as_str() == want.as_str() => {}
            // Absent (no row), present-but-NULL, or present-but-different ⇒ fail.
            _ => return Ok(false),
        }
    }
    Ok(true)
}

/// G11 (Slice 15) — does an edge FTS hit satisfy the filter?
///
/// Edge FTS hits always have `source_type = "edge_fact"` (the partition
/// discriminant). Their `row.kind` is the **relation** kind (e.g. `"owns"`,
/// `"works_for"`), not a node kind, so [`text_hit_passes_filter`] MUST NOT be
/// used for edge hits: `resolve_source_type(relation_kind)` returns `Err` for
/// unknown kinds, causing every edge hit to be silently rejected when a
/// `source_type` filter is set — the exact inverse of correct behaviour.
///
/// Edge bodies ARE projected into `vector_default` (rowid = `write_cursor`),
/// so `created_after` / `status` are satisfied by querying `vector_default`
/// exactly as [`text_hit_passes_filter`] does for node hits.
///
/// Rules:
/// - `source_type`: pass iff `None` **or** `== "edge_fact"`.
/// - `kind`: filter on the relation kind (`row.kind`) if specified.
/// - `created_after` / `status`: query `vector_default WHERE rowid = write_cursor`;
///   if absent from the vector partition the hit cannot satisfy a vec-metadata
///   predicate and is excluded.
#[allow(dead_code)]
pub(crate) fn edge_fts_hit_passes_filter(
    tx: &Connection,
    write_cursor: u64,
    row_kind: &str,
    filter: Option<&SearchFilter>,
) -> rusqlite::Result<bool> {
    let Some(filter) = filter else {
        return Ok(true);
    };
    if filter.is_unfiltered() {
        return Ok(true);
    }
    if let Some(ref st) = filter.source_type {
        if st != "edge_fact" {
            return Ok(false); // filter targets a specific non-edge source_type
        }
    }
    if let Some(ref k) = filter.kind {
        if k != row_kind {
            return Ok(false); // kind filter applies to the relation kind
        }
    }
    // Edge bodies are projected into vector_default; check created_after/status
    // there, the same way text_hit_passes_filter does for node hits.
    if filter.created_after.is_some() || filter.status.is_some() {
        let meta: Option<(i64, Option<String>)> = tx
            .query_row(
                "SELECT created_at, status FROM vector_default WHERE rowid = ?1 LIMIT 1",
                [write_cursor as i64],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?;
        let Some((created_at, status)) = meta else {
            // No vector-partition row: cannot satisfy a vec-metadata predicate.
            return Ok(false);
        };
        if let Some(bound) = filter.created_after {
            if created_at < bound {
                return Ok(false);
            }
        }
        if let Some(want) = &filter.status {
            if status.as_deref() != Some(want.as_str()) {
                return Ok(false);
            }
        }
    }
    // 0.8.20 Slice 15e fix-1 finding 1 [P2] — an edge body is projected into
    // vector_default (rowid = write_cursor) with the same `attr_<hex>` pre-KNN
    // columns as a node, and Slice 15d projects an edge's `filterable` attributes
    // into `canonical_attributes` keyed by its write_cursor. Enforce the same
    // attribute equality here so the edge-FTS arm matches the vector arm (D3 total
    // dispatch). An edge that carries no such attribute reads the `''` sentinel and
    // fails a non-empty equality, exactly as on the vector arm.
    if !hit_attributes_pass_filter(tx, write_cursor, filter)? {
        return Ok(false);
    }
    Ok(true)
}

/// Apply every edge filter except declared projection attributes. This isolates
/// the explanatory count from edge candidates rejected for an independent
/// source-type, relation-kind, or vec-metadata predicate.
#[allow(dead_code)]
fn edge_fts_hit_passes_non_attribute_filter(
    tx: &rusqlite::Transaction<'_>,
    write_cursor: u64,
    row_kind: &str,
    filter: Option<&SearchFilter>,
) -> rusqlite::Result<bool> {
    let Some(filter) = filter else {
        return Ok(true);
    };
    let mut non_attribute_filter = filter.clone();
    non_attribute_filter.attributes.clear();
    edge_fts_hit_passes_filter(tx, write_cursor, row_kind, Some(&non_attribute_filter))
}

#[cfg(test)]
mod slice85_boundary_tests {
    use super::{validate_filter_attributes_on_snapshot, SearchFilter, SnapshotFilterError};

    #[test]
    fn snapshot_filter_error_is_narrow_and_preserves_its_payload() {
        let storage = SnapshotFilterError::from(rusqlite::Error::InvalidQuery);
        assert!(matches!(storage, SnapshotFilterError::Sqlite(rusqlite::Error::InvalidQuery)));

        // A connection without the registry table declares no projection, so
        // the validator itself must raise the narrow InvalidFilter with its reason.
        let conn = rusqlite::Connection::open_in_memory().expect("in-memory connection");
        let mut filter = SearchFilter::default();
        filter.attributes = vec![("slice85_undeclared".to_string(), "x".to_string())];
        match validate_filter_attributes_on_snapshot(&conn, &filter) {
            Err(SnapshotFilterError::InvalidFilter(reason)) => assert_eq!(
                reason,
                "filter attribute \"slice85_undeclared\" is not a declared `filterable` \
                 projection; declare it via configure_projections before filtering on it"
            ),
            Err(SnapshotFilterError::Sqlite(error)) => {
                panic!("expected InvalidFilter, got {error}")
            }
            Ok(()) => panic!("an undeclared attribute must be rejected"),
        }
        assert!(validate_filter_attributes_on_snapshot(&conn, &SearchFilter::default()).is_ok());
    }
}
