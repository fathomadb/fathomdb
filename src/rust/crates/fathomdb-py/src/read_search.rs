use super::*;

// ===== read/search and pages ==========================================

#[pyfunction]
#[pyo3(signature = (engine, logical_id, view = None))]
pub(super) fn read_get(
    py: Python<'_>,
    engine: &PyEngine,
    logical_id: &Bound<'_, PyAny>,
    view: Option<&PyReadView>,
) -> PyResult<Option<PyNodeRecord>> {
    let logical_id = extract_validated_str(logical_id)?;
    let view = read_view_or_default(view);
    let inner = Arc::clone(&engine.inner);
    let record = call_engine(py, move || inner.read_get(&logical_id, &view))?;
    Ok(record.as_ref().map(PyNodeRecord::from_rust))
}

#[pyfunction]
#[pyo3(signature = (engine, logical_ids, view = None))]
pub(super) fn read_get_many(
    py: Python<'_>,
    engine: &PyEngine,
    logical_ids: &Bound<'_, PyList>,
    view: Option<&PyReadView>,
) -> PyResult<Vec<Option<PyNodeRecord>>> {
    let mut ids = Vec::with_capacity(logical_ids.len());
    for item in logical_ids.iter() {
        ids.push(extract_validated_str(&item)?);
    }
    let view = read_view_or_default(view);
    let inner = Arc::clone(&engine.inner);
    let rows = call_engine(py, move || inner.read_get_many(&ids, &view))?;
    Ok(rows.iter().map(|r| r.as_ref().map(PyNodeRecord::from_rust)).collect())
}

#[pyfunction]
#[pyo3(signature = (engine, collection, after_id=None, limit=0))]
pub(super) fn read_collection(
    py: Python<'_>,
    engine: &PyEngine,
    collection: &Bound<'_, PyAny>,
    after_id: Option<i64>,
    limit: u64,
) -> PyResult<Vec<PyOpStoreRow>> {
    read_collection_impl(py, engine, collection, after_id, limit)
}

#[pyfunction]
#[pyo3(signature = (engine, collection, after_id=None, limit=0))]
pub(super) fn read_mutations(
    py: Python<'_>,
    engine: &PyEngine,
    collection: &Bound<'_, PyAny>,
    after_id: Option<i64>,
    limit: u64,
) -> PyResult<Vec<PyOpStoreRow>> {
    read_collection_impl(py, engine, collection, after_id, limit)
}

pub(super) fn read_collection_impl(
    py: Python<'_>,
    engine: &PyEngine,
    collection: &Bound<'_, PyAny>,
    after_id: Option<i64>,
    limit: u64,
) -> PyResult<Vec<PyOpStoreRow>> {
    let collection = extract_validated_str(collection)?;
    let limit = limit as usize;
    let inner = Arc::clone(&engine.inner);
    let rows = call_engine(py, move || inner.read_collection(&collection, after_id, limit))?;
    Ok(rows.iter().map(PyOpStoreRow::from_rust).collect())
}

// ===== read.list (G4 / Slice 35) ======================================
//
// `read.list(engine, kind, predicates?, limit)` — list active canonical nodes
// of a given `kind`, optionally filtered by a list of `Predicate` dicts.
// Each predicate dict has the shape:
//   { "type": "eq"|"gt"|"gte"|"lt"|"lte", "path": str, "value": str|int|bool }
// Path validation happens in Rust (InvalidFilterError on non-allowlisted path).

pub(super) fn py_predicate_to_rust(pred: &Bound<'_, PyAny>) -> PyResult<RustPredicate> {
    let type_item = pred.get_item("type")?;
    let type_str = extract_validated_str(&type_item)?;
    let path_item = pred.get_item("path")?;
    let path = extract_validated_str(&path_item)?;
    let value_obj = pred.get_item("value")?;

    // Extract the value — try bool first (Python bool is a subclass of int, so
    // bool must be checked before int to avoid misclassifying True/False).
    // String values are validated through extract_validated_str for FFI safety.
    let scalar: RustScalarValue = if let Ok(b) = value_obj.extract::<bool>() {
        RustScalarValue::Bool(b)
    } else if let Ok(i) = value_obj.extract::<i64>() {
        RustScalarValue::Integer(i)
    } else {
        RustScalarValue::Text(extract_validated_str(&value_obj)?)
    };

    match type_str.as_str() {
        "eq" => RustPredicate::json_path_eq(path, scalar).map_err(engine_error_to_py),
        "gt" => RustPredicate::json_path_compare(path, RustComparisonOp::Gt, scalar)
            .map_err(engine_error_to_py),
        "gte" => RustPredicate::json_path_compare(path, RustComparisonOp::Gte, scalar)
            .map_err(engine_error_to_py),
        "lt" => RustPredicate::json_path_compare(path, RustComparisonOp::Lt, scalar)
            .map_err(engine_error_to_py),
        "lte" => RustPredicate::json_path_compare(path, RustComparisonOp::Lte, scalar)
            .map_err(engine_error_to_py),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "unknown predicate type '{other}'; expected 'eq', 'gt', 'gte', 'lt', or 'lte'"
        ))),
    }
}

#[pyfunction]
#[pyo3(signature = (engine, kind, predicates=None, limit=100, view=None))]
pub(super) fn read_list(
    py: Python<'_>,
    engine: &PyEngine,
    kind: &Bound<'_, PyAny>,
    predicates: Option<&Bound<'_, PyList>>,
    limit: u64,
    view: Option<&PyReadView>,
) -> PyResult<Vec<PyNodeRecord>> {
    let kind = extract_validated_str(kind)?;
    let mut rust_predicates: Vec<RustPredicate> = Vec::new();
    if let Some(plist) = predicates {
        for item in plist.iter() {
            rust_predicates.push(py_predicate_to_rust(&item)?);
        }
    }
    let limit = limit as usize;
    let view = read_view_or_default(view);
    let inner = Arc::clone(&engine.inner);
    let rows = call_engine(py, move || inner.read_list(&kind, &rust_predicates, limit, &view))?;
    Ok(rows.iter().map(PyNodeRecord::from_rust).collect())
}

// 0.8.11 Slice 40 (#17) — unified `Filter` → `read.list` backend. Each term dict:
//   { "term": "source_type"|"kind"|"created_after"|"status", "value": str|int }
//   { "term": "json", "predicate": { "type", "path", "value" } }
// The engine performs the authoritative total dispatch (Json json_extract;
// SourceType/Kind constant-fold vs the partition kind via resolve_source_type).
pub(super) fn py_filter_term_to_rust(term: &Bound<'_, PyAny>) -> PyResult<RustFilterTerm> {
    let term_kind_item = term.get_item("term")?;
    let term_kind = extract_validated_str(&term_kind_item)?;
    match term_kind.as_str() {
        "source_type" => {
            Ok(RustFilterTerm::SourceType(extract_validated_str(&term.get_item("value")?)?))
        }
        "kind" => Ok(RustFilterTerm::Kind(extract_validated_str(&term.get_item("value")?)?)),
        "created_after" => {
            Ok(RustFilterTerm::CreatedAfter(term.get_item("value")?.extract::<i64>()?))
        }
        "status" => Ok(RustFilterTerm::Status(extract_validated_str(&term.get_item("value")?)?)),
        "json" => Ok(RustFilterTerm::Json(py_predicate_to_rust(&term.get_item("predicate")?)?)),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "unknown filter term '{other}'; expected source_type/kind/created_after/status/json"
        ))),
    }
}

#[pyfunction]
#[pyo3(signature = (engine, kind, terms=None, limit=100, view=None))]
pub(super) fn read_list_filter(
    py: Python<'_>,
    engine: &PyEngine,
    kind: &Bound<'_, PyAny>,
    terms: Option<&Bound<'_, PyList>>,
    limit: u64,
    view: Option<&PyReadView>,
) -> PyResult<Vec<PyNodeRecord>> {
    let kind = extract_validated_str(kind)?;
    let mut rust_terms: Vec<RustFilterTerm> = Vec::new();
    if let Some(tlist) = terms {
        for item in tlist.iter() {
            rust_terms.push(py_filter_term_to_rust(&item)?);
        }
    }
    let filter = RustFilter { terms: rust_terms };
    let limit = limit as usize;
    let inner = Arc::clone(&engine.inner);
    let view = read_view_or_default(view);
    let rows = call_engine(py, move || inner.read_list_filter(&kind, &filter, limit, &view))?;
    Ok(rows.iter().map(PyNodeRecord::from_rust).collect())
}

#[pyfunction]
#[pyo3(signature = (engine, kind, context, limit, cursor=None, schema_version=1))]
pub(super) fn read_canonical_page(
    py: Python<'_>,
    engine: &PyEngine,
    kind: &Bound<'_, PyAny>,
    context: &PyFrozenReadContextV1,
    limit: i64,
    cursor: Option<String>,
    schema_version: u32,
) -> PyResult<PyNodePageV1> {
    let kind = extract_validated_str(kind)?;
    if let Some(value) = &cursor {
        validate_ffi_string_py(value)?;
    }
    let limit = usize::try_from(limit).map_err(|_| page_error("invalid_page_limit", "/limit"))?;
    let page = RustPageRequestV1 { schema_version, limit, cursor: cursor.map(RustPageCursor) };
    let frozen = context.inner.clone();
    let inner = Arc::clone(&engine.inner);
    call_engine(py, move || inner.read_canonical_page(&kind, &frozen, &page))
        .map(PyNodePageV1::from_rust)
}

#[pyfunction]
#[pyo3(signature = (engine, collection, record_key, context=None))]
pub(super) fn read_operational_state(
    py: Python<'_>,
    engine: &PyEngine,
    collection: &Bound<'_, PyAny>,
    record_key: &Bound<'_, PyAny>,
    context: Option<&PyFrozenReadContextV1>,
) -> PyResult<Option<PyOperationalStateRecordV1>> {
    let collection = extract_validated_str(collection)?;
    let record_key = extract_validated_str(record_key)?;
    let frozen = context.map(|value| value.inner.clone());
    let inner = Arc::clone(&engine.inner);
    call_engine(py, move || inner.read_operational_state(&collection, &record_key, frozen.as_ref()))
        .map(|value| value.as_ref().map(PyOperationalStateRecordV1::from_rust))
}

#[pyfunction]
#[pyo3(signature = (engine, collection, context, limit, cursor=None, schema_version=1))]
pub(super) fn read_operational_state_page(
    py: Python<'_>,
    engine: &PyEngine,
    collection: &Bound<'_, PyAny>,
    context: &PyFrozenReadContextV1,
    limit: i64,
    cursor: Option<String>,
    schema_version: u32,
) -> PyResult<PyOperationalStatePageV1> {
    let collection = extract_validated_str(collection)?;
    if let Some(value) = &cursor {
        validate_ffi_string_py(value)?;
    }
    let limit = usize::try_from(limit).map_err(|_| page_error("invalid_page_limit", "/limit"))?;
    let page = RustPageRequestV1 { schema_version, limit, cursor: cursor.map(RustPageCursor) };
    let frozen = context.inner.clone();
    let inner = Arc::clone(&engine.inner);
    call_engine(py, move || inner.read_operational_state_page(&collection, &frozen, &page))
        .map(PyOperationalStatePageV1::from_rust)
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "SoftFallback",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PySoftFallback {
    branch: String,
}

impl PySoftFallback {
    pub(super) fn from_rust(s: &RustSoftFallback) -> Self {
        Self {
            branch: match s.branch {
                SoftFallbackBranch::Vector => "vector".to_string(),
                SoftFallbackBranch::Text => "text".to_string(),
                SoftFallbackBranch::TextEdge => "text_edge".to_string(),
                SoftFallbackBranch::GraphArm => "graph_arm".to_string(),
            },
        }
    }
}

/// C-2 (0.8.19 / OPP-12 Phase-1, TC-8) — the typed id-space carrier for
/// [`PySearchHit::id`], surfaced to Python as an `IdSpace` with `space` +
/// `value` attributes. `space` is the lowercase discriminant (`"logical"` |
/// `"content"` | `"passage"`), mirroring the engine's `IdSpaceKind` enum (the
/// C-2 binding — a typed carrier, not a magic-prefixed string). `value` is the
/// bare id (id-space prefix stripped).
#[pyclass(module = "fathomdb._fathomdb", name = "IdSpace", frozen, get_all, skip_from_py_object)]
#[derive(Clone)]
pub(super) struct PyIdSpace {
    space: String,
    value: String,
}

impl PyIdSpace {
    pub(super) fn from_rust(id: &RustIdSpace) -> Self {
        Self { space: id.space.as_str().to_string(), value: id.value.clone() }
    }
}

#[pyclass(module = "fathomdb._fathomdb", name = "SearchHit", frozen, get_all, skip_from_py_object)]
#[derive(Clone)]
pub(super) struct PySearchHit {
    /// C-2 (0.8.19 / TC-8) — the typed, non-null, id-space-total hit id
    /// (`IdSpace` with `space` + `value`). Governed hits are `logical` (`"l:"`),
    /// doc-seeded hits `content` (`"h:"`), synthetic passages `passage`
    /// (`"p:"`). Its `value` equals the pre-0.8.19 `stable_id` (which this
    /// subsumes) so cross-session real-gold keying continues on `id`. The pre-C-2
    /// positional `write_cursor` id is engine-internal and no longer surfaced.
    id: PyIdSpace,
    kind: String,
    body: String,
    score: f64,
    branch: String,
    /// Source-document provenance — the identifier `erase_source` consumes.
    /// TC-31 (0.8.20): populated on EVERY hit path, not just the graph arm.
    /// Node hits (text/vector) carry the node's own `source_id`; edge hits
    /// (edge-FTS, vector edge-fact) carry the edge's own; graph-arm hits carry
    /// the traversed edge's (unchanged). `None` only when the stored row really
    /// has NULL provenance: written before 0.8.20, or a governed row spared by
    /// the step-21 backfill under the TC-11 pin.
    source_id: Option<String>,
    /// 0.8.5 (EXP-0) — per-candidate CE score `ce_norm = sigmoid(ce_logit)`.
    /// `Some` only for hits inside the reranked pool; `None` otherwise.
    ce_score: Option<f64>,
}

impl PySearchHit {
    pub(super) fn from_rust(h: &RustSearchHit) -> Self {
        Self {
            id: PyIdSpace::from_rust(&h.id),
            kind: h.kind.clone(),
            body: h.body.clone(),
            score: h.score,
            branch: match h.branch {
                SoftFallbackBranch::Vector => "vector".to_string(),
                SoftFallbackBranch::Text => "text".to_string(),
                SoftFallbackBranch::TextEdge => "text_edge".to_string(),
                SoftFallbackBranch::GraphArm => "graph_arm".to_string(),
            },
            source_id: h.source_id.clone(),
            ce_score: h.ce_score,
        }
    }
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "SearchResult",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PySearchResult {
    projection_cursor: u64,
    soft_fallback: Option<PySoftFallback>,
    results: Vec<PySearchHit>,
    /// 0.8.8 EXP-OBS (Slice 10) — opt-in retrieval explanation sidecar. `Some`
    /// only when `search(..., explain=True)`; `None` (default) keeps the payload
    /// byte-identical to the pre-0.8.8 shape.
    explanation: Option<PyExplanation>,
}

impl PySearchResult {
    pub(super) fn from_rust(r: RustSearchResult) -> Self {
        Self {
            projection_cursor: r.projection_cursor,
            soft_fallback: r.soft_fallback.as_ref().map(PySoftFallback::from_rust),
            results: r.results.iter().map(PySearchHit::from_rust).collect(),
            explanation: r.explanation.as_ref().map(PyExplanation::from_rust),
        }
    }
}

#[pyclass(module = "fathomdb._fathomdb", name = "OpStoreRow", frozen, get_all, skip_from_py_object)]
#[derive(Clone)]
pub(super) struct PyOpStoreRow {
    id: i64,
    collection: String,
    record_key: String,
    op_kind: String,
    payload: String,
    schema_id: Option<String>,
    write_cursor: u64,
}

impl PyOpStoreRow {
    pub(super) fn from_rust(r: &RustOpStoreRow) -> Self {
        Self {
            id: r.id,
            collection: r.collection.clone(),
            record_key: r.record_key.clone(),
            op_kind: r.op_kind.clone(),
            payload: r.payload.clone(),
            schema_id: r.schema_id.clone(),
            write_cursor: r.write_cursor,
        }
    }
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "OperationalStateRecordV1",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyOperationalStateRecordV1 {
    schema_version: u32,
    collection: String,
    record_key: String,
    payload: String,
    schema_id: Option<String>,
    write_cursor: u64,
}

impl PyOperationalStateRecordV1 {
    pub(super) fn from_rust(record: &RustOperationalStateRecordV1) -> Self {
        Self {
            schema_version: record.schema_version,
            collection: record.collection.clone(),
            record_key: record.record_key.clone(),
            payload: record.payload.clone(),
            schema_id: record.schema_id.clone(),
            write_cursor: record.write_cursor,
        }
    }
}

#[pyclass(module = "fathomdb._fathomdb", name = "NodePageV1", frozen, get_all, skip_from_py_object)]
#[derive(Clone)]
pub(super) struct PyNodePageV1 {
    schema_version: u32,
    items: Vec<PyNodeRecord>,
    next_cursor: Option<String>,
}

impl PyNodePageV1 {
    pub(super) fn from_rust(page: RustPageV1<RustNodeRecord>) -> Self {
        Self {
            schema_version: page.schema_version,
            items: page.items.iter().map(PyNodeRecord::from_rust).collect(),
            next_cursor: page.next_cursor.map(|cursor| cursor.0),
        }
    }
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "OperationalStatePageV1",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyOperationalStatePageV1 {
    schema_version: u32,
    items: Vec<PyOperationalStateRecordV1>,
    next_cursor: Option<String>,
}

impl PyOperationalStatePageV1 {
    pub(super) fn from_rust(page: RustPageV1<RustOperationalStateRecordV1>) -> Self {
        Self {
            schema_version: page.schema_version,
            items: page.items.iter().map(PyOperationalStateRecordV1::from_rust).collect(),
            next_cursor: page.next_cursor.map(|cursor| cursor.0),
        }
    }
}
