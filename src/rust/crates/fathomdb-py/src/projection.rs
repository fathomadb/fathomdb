use super::*;

// ===== Projection configuration and status ============================

/// 0.8.20 Slice 15d (R-20-PR / C-1) — the `configure_projections` governed verb.
/// Declarative + idempotent: the engine diffs `specs` against the durable
/// registry and backfills the difference. `drop` is EXPLICIT (omission never
/// drops); a destructive change to a live projection without a drop raises
/// `ProjectionDestructiveError`.
#[pyfunction]
#[pyo3(signature = (engine, specs, drop = None))]
pub(super) fn configure_projections(
    py: Python<'_>,
    engine: &PyEngine,
    specs: Vec<PyProjectionSpec>,
    drop: Option<Vec<String>>,
) -> PyResult<PyProjectionDelta> {
    let rust_specs: Vec<RustProjectionSpec> =
        specs.iter().map(PyProjectionSpec::to_rust).collect::<PyResult<_>>()?;
    let drop = drop.unwrap_or_default();
    // AC-068a/b — the `drop` list is a caller-supplied FFI-string vector too;
    // validate each entry before the engine call, like the spec strings.
    for name in &drop {
        validate_ffi_string_py(name)?;
    }
    let inner = Arc::clone(&engine.inner);
    let delta = call_engine(py, move || inner.configure_projections(&rust_specs, &drop))?;
    Ok(PyProjectionDelta::from_rust(&delta))
}

/// 0.8.20 Slice 15d (R-20-PR) — `read.projections` introspection. Returns every
/// declared `ProjectionSpec` (sorted by name).
#[pyfunction]
#[pyo3(signature = (engine))]
pub(super) fn read_projections(
    py: Python<'_>,
    engine: &PyEngine,
) -> PyResult<Vec<PyProjectionSpec>> {
    let inner = Arc::clone(&engine.inner);
    let specs = call_engine(py, move || inner.read_projections())?;
    Ok(specs.iter().map(PyProjectionSpec::from_rust).collect())
}

/// Read the current projection-runtime status without changing configuration or
/// scheduling work. This pure query may take the ordinarily opened engine
/// connection lock; it is not a `ReaderWorkerPool` request and does not open a
/// separately read-only SQLite connection. The public Python wrapper converts
/// this native value into frozen SDK dataclasses with closed Literal wire
/// vocabularies.
#[pyfunction]
#[pyo3(signature = (engine))]
pub(super) fn read_projection_status(
    py: Python<'_>,
    engine: &PyEngine,
) -> PyResult<PyProjectionRuntimeStatus> {
    let inner = Arc::clone(&engine.inner);
    let status = call_engine(py, move || inner.read_projection_status())?;
    Ok(PyProjectionRuntimeStatus::from_rust(&status))
}

#[pyfunction]
#[pyo3(signature = (engine))]
pub(super) fn read_projection_generation_status(
    py: Python<'_>,
    engine: &PyEngine,
) -> PyResult<PyProjectionGenerationStatusV1> {
    let inner = Arc::clone(&engine.inner);
    call_engine(py, move || inner.read_projection_generation_status()).map(Into::into)
}

pub(super) fn projection_generation_input_error(
    reason: &str,
    field_path: impl Into<String>,
) -> PyErr {
    let field_path = field_path.into();
    let exc = ProjectionGenerationError::new_err(format!(
        "projection generation {reason} at {field_path}"
    ));
    Python::attach(|py| {
        let value = exc.value(py);
        let _ = value.setattr("reason", reason);
        let _ = value.setattr("field_path", field_path);
    });
    exc
}

pub(super) fn canonical_projection_u64(value: &Bound<'_, PyAny>, path: &str) -> PyResult<u64> {
    let text = value
        .extract::<String>()
        .map_err(|_| projection_generation_input_error("invalid_write_cursor", path))?;
    if text.is_empty()
        || (text.len() > 1 && text.starts_with('0'))
        || !text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(projection_generation_input_error("invalid_write_cursor", path));
    }
    text.parse::<u64>()
        .ok()
        .filter(|value| *value != 0)
        .ok_or_else(|| projection_generation_input_error("invalid_write_cursor", path))
}

pub(super) fn strict_projection_generation_request_dict<'py>(
    request: &'py Bound<'py, PyDict>,
) -> PyResult<&'py Bound<'py, PyDict>> {
    let allowed = ["expectedGenerationId", "operationId", "schemaVersion", "writeCursor"];
    let mut unknown = None;
    for key in request.keys().iter() {
        let key = key
            .extract::<String>()
            .map_err(|_| projection_generation_input_error("unknown_field", ""))?;
        if !allowed.contains(&key.as_str()) && unknown.as_ref().is_none_or(|current| key < *current)
        {
            unknown = Some(key);
        }
    }
    if let Some(key) = unknown {
        return Err(projection_generation_input_error(
            "unknown_field",
            format!("/{}", escape_json_pointer_token(&key)),
        ));
    }
    Ok(request)
}

#[pyfunction]
#[pyo3(signature = (engine, request))]
pub(super) fn read_mutation_projection_status(
    py: Python<'_>,
    engine: &PyEngine,
    request: &Bound<'_, PyDict>,
) -> PyResult<PyMutationProjectionStatusV1> {
    let schema_version = request
        .get_item("schemaVersion")?
        .filter(|value| !value.is_instance_of::<pyo3::types::PyBool>())
        .and_then(|value| value.extract::<u32>().ok())
        .ok_or_else(|| {
            projection_generation_input_error("unsupported_schema_version", "/schemaVersion")
        })?;
    if schema_version != 1 {
        return Err(projection_generation_input_error(
            "unsupported_schema_version",
            "/schemaVersion",
        ));
    }
    let request = strict_projection_generation_request_dict(request)?;
    let operation_id = request
        .get_item("operationId")?
        .and_then(|value| value.extract::<String>().ok())
        .ok_or_else(|| projection_generation_input_error("invalid_operation_id", "/operationId"))?;
    let operation_bytes = operation_id.as_bytes();
    let operation_valid = !operation_id.starts_with("_fdb:")
        && (1..=128).contains(&operation_bytes.len())
        && operation_bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        && operation_bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'));
    if !operation_valid {
        return Err(projection_generation_input_error("invalid_operation_id", "/operationId"));
    }
    let write_cursor = canonical_projection_u64(
        &request.get_item("writeCursor")?.ok_or_else(|| {
            projection_generation_input_error("invalid_write_cursor", "/writeCursor")
        })?,
        "/writeCursor",
    )?;
    let generation = request
        .get_item("expectedGenerationId")?
        .and_then(|value| value.extract::<String>().ok())
        .ok_or_else(|| {
            projection_generation_input_error("invalid_generation_id", "/expectedGenerationId")
        })?;
    let expected_generation_id = RustProjectionGenerationId::new(generation)
        .map_err(|error| projection_generation_error_to_py(&error))?;
    let inner = Arc::clone(&engine.inner);
    call_engine(py, move || {
        inner.read_mutation_projection_status(RustMutationProjectionStatusRequestV1 {
            schema_version,
            operation_id,
            write_cursor,
            expected_generation_id,
        })
    })
    .map(Into::into)
}

#[pyfunction]
#[pyo3(signature = (engine))]
pub(super) fn read_embedding_readiness(
    py: Python<'_>,
    engine: &PyEngine,
) -> PyResult<PyEmbeddingReadiness> {
    let inner = Arc::clone(&engine.inner);
    let readiness = call_engine(py, move || inner.read_embedding_readiness())?;
    Ok(PyEmbeddingReadiness::from_rust(&readiness))
}

// 0.8.20 Slice 15d (R-20-PR) — the Python face of a `ProjectionSpec`. Flat at
// the native boundary (the Python `fathomdb.types.ProjectionSpec` dataclass
// translates the nested `fts?`/`vector?` shape to/from these fields). `fts` /
// `vector` booleans carry the SUB-OBJECT PRESENCE; the optional tokenizer /
// embedder carry the value (None = engine default).
#[pyclass(module = "fathomdb._fathomdb", name = "ProjectionSpec", get_all, from_py_object)]
#[derive(Clone)]
pub(super) struct PyProjectionSpec {
    name: String,
    roles: Vec<String>,
    fts: bool,
    fts_tokenizer: Option<String>,
    vector: bool,
    vector_embedder: Option<String>,
    /// READ METADATA, engine-set: `"unavailable"` / `"embedding"` / `"ready"`
    /// on the way OUT of `read.projections`, `None` on every caller-authored
    /// spec. `"unavailable"` means no usable dense runtime; the other values
    /// derive from outstanding work under one. Inert on the way IN (the engine
    /// reports the derived truth), so read output still re-applies as a no-op.
    vector_dense_readiness: Option<String>,
    source: Option<Vec<String>>,
}

#[pymethods]
impl PyProjectionSpec {
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (name, roles, fts = false, fts_tokenizer = None, vector = false, vector_embedder = None, vector_dense_readiness = None, source = None))]
    fn new(
        name: String,
        roles: Vec<String>,
        fts: bool,
        fts_tokenizer: Option<String>,
        vector: bool,
        vector_embedder: Option<String>,
        vector_dense_readiness: Option<String>,
        source: Option<Vec<String>>,
    ) -> Self {
        Self {
            name,
            roles,
            fts,
            fts_tokenizer,
            vector,
            vector_embedder,
            vector_dense_readiness,
            source,
        }
    }
}

impl PyProjectionSpec {
    pub(super) fn from_rust(s: &RustProjectionSpec) -> Self {
        Self {
            name: s.name.clone(),
            roles: s.roles.iter().map(|r| r.as_str().to_string()).collect(),
            fts: s.fts.is_some(),
            fts_tokenizer: s.fts.as_ref().and_then(|f| f.tokenizer.clone()),
            vector: s.vector.is_some(),
            vector_embedder: s.vector.as_ref().and_then(|v| v.embedder.clone()),
            vector_dense_readiness: s
                .vector
                .as_ref()
                .and_then(|v| v.dense_readiness)
                .map(|r| r.as_str().to_string()),
            source: s.source.clone(),
        }
    }

    pub(super) fn to_rust(&self) -> PyResult<RustProjectionSpec> {
        // AC-068a/b — reject every string crossing the FFI into the spec BEFORE
        // the engine (writer transaction) is reached. Mirrors the per-string
        // gate applied at every other pyo3 call site (e.g. `validate_ffi_string_py`).
        validate_ffi_string_py(&self.name)?;
        if let Some(tokenizer) = &self.fts_tokenizer {
            validate_ffi_string_py(tokenizer)?;
        }
        if let Some(embedder) = &self.vector_embedder {
            validate_ffi_string_py(embedder)?;
        }
        if let Some(source) = &self.source {
            for segment in source {
                validate_ffi_string_py(segment)?;
            }
        }
        // 0.8.20 keystone closeout fix-4 — ROUND-TRIP CONSISTENCY GATE. A spec
        // the binding ACCEPTS must round-trip through `read.projections`
        // IDENTICALLY; otherwise reject it HERE with the typed validation error
        // rather than let the engine silently drop or normalize a sub-field.
        // Kept byte-for-byte in step with the napi binding (Py ≡ TS): the two
        // must refuse the same shapes the same way. `fts`/`vector` carry the
        // sub-object PRESENCE, so an `fts_tokenizer` supplied while `fts` is
        // false (or an empty `""` that the engine collapses to the default)
        // could never survive the round-trip and is refused.
        match (self.fts, self.fts_tokenizer.as_deref()) {
            (false, Some(_)) => {
                return Err(InvalidArgumentError::new_err(format!(
                    "projection {:?}: fts_tokenizer is set but fts is false — the tokenizer would be silently dropped and cannot round-trip; set fts=true or omit fts_tokenizer",
                    self.name
                )));
            }
            (true, Some("")) => {
                return Err(InvalidArgumentError::new_err(format!(
                    "projection {:?}: fts_tokenizer is an empty string, which the engine normalizes to the default and cannot round-trip; omit fts_tokenizer for the engine default",
                    self.name
                )));
            }
            _ => {}
        }
        match (self.vector, self.vector_embedder.as_deref()) {
            (false, Some(_)) => {
                return Err(InvalidArgumentError::new_err(format!(
                    "projection {:?}: vector_embedder is set but vector is false — the embedder would be silently dropped and cannot round-trip; set vector=true or omit vector_embedder",
                    self.name
                )));
            }
            (true, Some("")) => {
                return Err(InvalidArgumentError::new_err(format!(
                    "projection {:?}: vector_embedder is an empty string, which the engine normalizes to the default and cannot round-trip; omit vector_embedder for the engine default",
                    self.name
                )));
            }
            _ => {}
        }
        // 0.8.20 Slice 20 (R-20-DR) — the SAME round-trip gate applied to the
        // engine-set readiness field. It is READ METADATA, so its VALUE is inert
        // on the way in (the engine always reports the derived truth, which is
        // what keeps `read.projections` output re-appliable as a no-op — the
        // fix-4 read→configure round-trip, pinned by a test in both bindings).
        // But the two shapes that could NEVER round-trip are refused, exactly as
        // for `vector_embedder`:
        //   * supplied while `vector` is false — there is no vector sub-object
        //     to carry it, so `read.projections` could not echo it back;
        //   * an unrecognised spelling — `read.projections` only ever emits
        //     `"unavailable"` / `"embedding"` / `"ready"`, so anything else
        //     (notably `"pending"`,
        //     which is RESERVED for the orthogonal admission axis, and `""`)
        //     could not round-trip and is a caller mistake worth naming.
        if let Some(readiness) = self.vector_dense_readiness.as_deref() {
            validate_ffi_string_py(readiness)?;
            if !self.vector {
                return Err(InvalidArgumentError::new_err(format!(
                    "projection {:?}: vector_dense_readiness is set but vector is false — readiness belongs to the vector sub-object and cannot round-trip without it; set vector=true or omit vector_dense_readiness",
                    self.name
                )));
            }
            if RustDenseReadiness::from_str_opt(readiness).is_none() {
                return Err(InvalidArgumentError::new_err(format!(
                    "projection {:?}: unknown vector_dense_readiness {readiness:?}: expected \"unavailable\", \"embedding\", or \"ready\" (\"pending\" is reserved for the admission axis and is never a readiness value). It is engine-set read metadata; omit it",
                    self.name
                )));
            }
        }
        let mut roles = std::collections::BTreeSet::new();
        for r in &self.roles {
            validate_ffi_string_py(r)?;
            let role = RustProjectionRole::from_str_opt(r).ok_or_else(|| {
                InvalidArgumentError::new_err(format!(
                    "unknown projection role {r:?}: expected filterable/rankable/searchable"
                ))
            })?;
            // fix-4 — `roles` is a SET; a duplicate spelling in the flat list
            // cannot round-trip (the registry stores a de-duplicated
            // `BTreeSet`), so refuse it rather than silently coalesce.
            if !roles.insert(role) {
                return Err(InvalidArgumentError::new_err(format!(
                    "projection {:?}: role {r:?} is repeated; roles is a set and duplicates cannot round-trip",
                    self.name
                )));
            }
        }
        Ok(RustProjectionSpec {
            name: self.name.clone(),
            roles,
            fts: self.fts.then(|| RustProjectionFts { tokenizer: self.fts_tokenizer.clone() }),
            vector: self.vector.then(|| RustProjectionVector {
                embedder: self.vector_embedder.clone(),
                // 0.8.20 Slice 20 (R-20-DR) — readiness is engine-set READ
                // METADATA. Carried across so the engine can see what the caller
                // sent, but the registry never stores it and never honours it.
                dense_readiness: self
                    .vector_dense_readiness
                    .as_deref()
                    .and_then(RustDenseReadiness::from_str_opt),
            }),
            source: self.source.clone(),
        })
    }
}

// 0.8.20 Slice 15d (R-20-PR) — the Python face of the apply diff.
#[pyclass(
    module = "fathomdb._fathomdb",
    name = "ProjectionDelta",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyProjectionDelta {
    built: Vec<String>,
    dropped: Vec<String>,
    deferred: Vec<String>,
    unchanged: bool,
    // 0.8.20 Slice 22 (R-20-VC / TC-67) — node KINDS the vector writer can never
    // commit. A different axis from the three attribute-name lists above; the
    // name says so. Output-only: `configure_projections` takes specs, never a
    // delta, so there is no inbound direction to round-trip.
    vector_unsupported_kinds: Vec<String>,
}

impl PyProjectionDelta {
    pub(super) fn from_rust(d: &RustProjectionDelta) -> Self {
        Self {
            built: d.built.clone(),
            dropped: d.dropped.clone(),
            deferred: d.deferred.clone(),
            unchanged: d.unchanged,
            vector_unsupported_kinds: d.vector_unsupported_kinds.clone(),
        }
    }
}

/// The Python-native form of one entry in the pure projection-status facade.
#[pyclass(
    module = "fathomdb._fathomdb",
    name = "ProjectionRuntimeStatusEntry",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyProjectionRuntimeStatusEntry {
    name: String,
    dense_readiness: String,
}

impl PyProjectionRuntimeStatusEntry {
    pub(super) fn from_rust(entry: &RustProjectionRuntimeStatusEntry) -> Self {
        Self {
            name: entry.name.clone(),
            dense_readiness: entry.dense_readiness.as_str().to_string(),
        }
    }
}

/// The Python-native form of the pure projection-runtime status facade.
#[pyclass(
    module = "fathomdb._fathomdb",
    name = "ProjectionRuntimeStatus",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyProjectionRuntimeStatus {
    runtime_embedder_available: bool,
    runtime_unavailability_reason: String,
    projections: Vec<PyProjectionRuntimeStatusEntry>,
    vector_unsupported_kinds: Vec<String>,
}

impl PyProjectionRuntimeStatus {
    pub(super) fn from_rust(status: &RustProjectionRuntimeStatus) -> Self {
        Self {
            runtime_embedder_available: status.runtime_embedder_available,
            runtime_unavailability_reason: status
                .runtime_unavailability_reason
                .as_str()
                .to_string(),
            projections: status
                .projections
                .iter()
                .map(PyProjectionRuntimeStatusEntry::from_rust)
                .collect(),
            vector_unsupported_kinds: status.vector_unsupported_kinds.clone(),
        }
    }
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "EmbeddingReadiness",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyEmbeddingReadiness {
    state: String,
    usable_embedder: bool,
    pending_count: u64,
    affected_kinds: Vec<String>,
    code: Option<String>,
    operation: Option<String>,
    remediations: Vec<String>,
    documentation_url: Option<String>,
}

impl PyEmbeddingReadiness {
    pub(super) fn from_rust(readiness: &RustEmbeddingReadiness) -> Self {
        let blocked = readiness.blocked.as_ref();
        Self {
            state: readiness.state.as_str().to_string(),
            usable_embedder: readiness.usable_embedder,
            pending_count: readiness.pending_count,
            affected_kinds: readiness.affected_kinds.clone(),
            code: blocked.map(|b| b.code.to_string()),
            operation: blocked.map(|b| b.operation.as_str().to_string()),
            remediations: blocked
                .map(|b| b.remediations.iter().map(|s| (*s).to_string()).collect())
                .unwrap_or_default(),
            documentation_url: blocked.map(|b| b.documentation_url.to_string()),
        }
    }
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "ProjectionGenerationStatusV1",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyProjectionGenerationStatusV1 {
    schema_version: u32,
    generation_id: String,
    declaration_sha256: String,
    origin: String,
    transition_boundary: String,
    effective_at_epoch_s: i64,
    observed_boundary: String,
    ready_through: String,
    readiness: String,
    runtime_state: String,
    pending_count: String,
    failed_count: String,
}

impl From<RustProjectionGenerationStatusV1> for PyProjectionGenerationStatusV1 {
    fn from(value: RustProjectionGenerationStatusV1) -> Self {
        Self {
            schema_version: value.schema_version,
            generation_id: value.generation_id.as_str().to_string(),
            declaration_sha256: value.declaration_sha256,
            origin: value.origin.as_str().to_string(),
            transition_boundary: value.transition_boundary.to_string(),
            effective_at_epoch_s: value.effective_at_epoch_s,
            observed_boundary: value.observed_boundary.to_string(),
            ready_through: value.ready_through.to_string(),
            readiness: value.readiness.as_str().to_string(),
            runtime_state: value.runtime_state.as_str().to_string(),
            pending_count: value.pending_count.to_string(),
            failed_count: value.failed_count.to_string(),
        }
    }
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "MutationProjectionStatusV1",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyMutationProjectionStatusV1 {
    schema_version: u32,
    operation_id: String,
    write_cursor: String,
    generation_id: String,
    effective_at_epoch_s: i64,
    observed_boundary: String,
    ready_through: String,
    readiness: String,
    runtime_state: String,
    pending_count: String,
    failed_count: String,
}

impl From<RustMutationProjectionStatusV1> for PyMutationProjectionStatusV1 {
    fn from(value: RustMutationProjectionStatusV1) -> Self {
        Self {
            schema_version: value.schema_version,
            operation_id: value.operation_id,
            write_cursor: value.write_cursor.to_string(),
            generation_id: value.generation_id.as_str().to_string(),
            effective_at_epoch_s: value.effective_at_epoch_s,
            observed_boundary: value.observed_boundary.to_string(),
            ready_through: value.ready_through.to_string(),
            readiness: value.readiness.as_str().to_string(),
            runtime_state: value.runtime_state.as_str().to_string(),
            pending_count: value.pending_count.to_string(),
            failed_count: value.failed_count.to_string(),
        }
    }
}
