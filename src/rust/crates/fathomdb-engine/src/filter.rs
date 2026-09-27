use super::*;

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
        let path = self.path();
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
