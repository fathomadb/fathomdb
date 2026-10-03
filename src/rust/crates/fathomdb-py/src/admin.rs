use super::*;

// ===== admin.configure ================================================

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "RuntimeConfiguration",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyRuntimeConfiguration {
    sqlite_mode: String,
}

impl From<RustRuntimeConfiguration> for PyRuntimeConfiguration {
    fn from(value: RustRuntimeConfiguration) -> Self {
        Self {
            sqlite_mode: match value.sqlite_mode {
                RustRuntimeSqliteMode::Performance => "performance",
                RustRuntimeSqliteMode::Diagnostics => "diagnostics",
            }
            .to_string(),
        }
    }
}

#[pyfunction]
pub(super) fn admin_configure_runtime(sqlite_mode: &str) -> PyResult<PyRuntimeConfiguration> {
    let mode = match sqlite_mode {
        "performance" => RustRuntimeSqliteMode::Performance,
        "diagnostics" => RustRuntimeSqliteMode::Diagnostics,
        _ => {
            return Err(PyValueError::new_err("sqlite_mode must be 'performance' or 'diagnostics'"))
        }
    };
    fathomdb_engine::configure_runtime(mode)
        .map(PyRuntimeConfiguration::from)
        .map_err(runtime_configuration_error_to_py)
}

#[pyfunction]
#[pyo3(signature = (engine, name, body))]
pub(super) fn admin_configure(
    py: Python<'_>,
    engine: &PyEngine,
    name: &Bound<'_, PyAny>,
    body: &Bound<'_, PyAny>,
) -> PyResult<PyWriteReceipt> {
    let name = extract_validated_str(name)?;
    let body = extract_validated_str(body)?;
    if name.is_empty() {
        return Err(PyValueError::new_err("admin.configure requires a non-empty name"));
    }
    // why: `dev/interfaces/python.md` § Runtime surface pins the
    // admin.configure(name=, body=) signature; the engine's
    // `PreparedWrite::AdminSchema` requires `kind ∈ {latest_state,
    // append_only_log}`. The Python verb is sugar over latest-state
    // collection registration in 0.6.0; an explicit `kind` knob lands
    // in a later 0.6.x slice if needed.
    let batch = vec![PreparedWrite::AdminSchema {
        name,
        kind: "latest_state".to_string(),
        schema_json: body,
        retention_json: "{}".to_string(),
    }];
    let inner = Arc::clone(&engine.inner);
    let receipt = call_engine(py, move || inner.write(&batch))?;
    Ok(PyWriteReceipt::from_rust(receipt))
}

// ===== read.* (G2/G3) =================================================
//
// Slice 30 — the governed `read.*` namespace native fns. `read.get` /
// `read.get_many` are active-only point lookups by `logical_id` (not-found is a
// normal `None`, never an exception — a typed NotFound class is reserved-gap
// Slice 31). `read.collection` / `read.mutations` are the paginated op-store
// read-back with a MANDATORY limit + after-id cursor. All four ride the engine's
// ReaderWorkerPool DEFERRED-tx path inside the engine; the binding only marshals.

// OPP-12 Phase-1 (0.8.19 Slice 10) — the `transition`/`purge` lifecycle verbs.
// Thin pass-throughs to the engine (no client-side logic): `transition` enforces
// the legal-transition table + `reason` clear-on-admit/set-on-exclude semantics;
// `purge` is the deleted-first, idempotent hard-erase. Both key on the bare
// `logical_id` (`l:` only); a non-`l:` id raises `NotLifecycleAddressableError`.

#[pyfunction]
#[pyo3(signature = (engine, logical_id, to_state, reason=None))]
pub(super) fn transition(
    py: Python<'_>,
    engine: &PyEngine,
    logical_id: &Bound<'_, PyAny>,
    to_state: &str,
    reason: Option<&Bound<'_, PyAny>>,
) -> PyResult<()> {
    let logical_id = extract_validated_str(logical_id)?;
    let reason = extract_opt_validated_str(reason)?;
    // The full LifecycleState vocabulary is accepted at the boundary so illegal
    // targets (`pending`/`purged`) reach the engine and surface a typed
    // IllegalTransitionError; only an out-of-vocabulary string is rejected here.
    let to_state = RustLifecycleState::from_str_opt(to_state).ok_or_else(|| {
        InvalidArgumentError::new_err(format!(
            "unknown lifecycle state {to_state:?}: expected one of pending/active/deleted/purged"
        ))
    })?;
    let inner = Arc::clone(&engine.inner);
    call_engine(py, move || inner.transition(&logical_id, to_state, reason))
}

#[pyfunction]
#[pyo3(signature = (engine, logical_id))]
pub(super) fn purge(
    py: Python<'_>,
    engine: &PyEngine,
    logical_id: &Bound<'_, PyAny>,
) -> PyResult<()> {
    let logical_id = extract_validated_str(logical_id)?;
    let inner = Arc::clone(&engine.inner);
    call_engine(py, move || inner.purge(&logical_id))
}

/// 0.8.20 Slice 5d (R-20-E4, design §4 item 9b) — the `erase_source` lifecycle
/// verb. Deletes every canonical row carrying `source_id`, plus its row-owned
/// projections, and finishes the erasure at rest.
///
/// The COMPANION to `purge`, not a duplicate of it: `purge` addresses a
/// governed node by `logical_id`; `erase_source` addresses ANONYMOUS content
/// (rows with no `logical_id`) by its provenance, which `purge` cannot reach.
/// Together they make every canonical row erasable from the SDK alone, with no
/// CLI on `PATH` (R-20-E4).
///
/// NOT a recovery verb: `erase_source` carries no REQ-054 denylist name
/// (`recover`/`restore`/`repair`/`fix`/`rebuild`), so AC-041 is unaffected.
///
/// Raises `WriteValidationError` for an empty, whitespace-only or reserved
/// (`_`-prefixed) `source_id` — the engine's reserved namespace is reachable
/// only through the CLI recovery seam.
#[pyfunction]
#[pyo3(signature = (engine, source_id))]
pub(super) fn erase_source(
    py: Python<'_>,
    engine: &PyEngine,
    source_id: &Bound<'_, PyAny>,
) -> PyResult<PyEraseReport> {
    let source_id = extract_validated_str(source_id)?;
    let inner = Arc::clone(&engine.inner);
    let report = call_engine(py, move || inner.erase_source(&source_id))?;
    Ok(PyEraseReport::from_rust(report))
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "SourceDependencyV1",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PySourceDependencyV1 {
    schema_version: u32,
    dependency_id: String,
    source_revision_id: String,
    derived_revision_id: String,
    registered_dependency_generation: String,
}

impl From<RustSourceDependencyV1> for PySourceDependencyV1 {
    fn from(value: RustSourceDependencyV1) -> Self {
        Self {
            schema_version: value.schema_version,
            dependency_id: value.dependency_id.as_str().to_string(),
            source_revision_id: value.source_revision_id.as_str().to_string(),
            derived_revision_id: value.derived_revision_id.as_str().to_string(),
            registered_dependency_generation: value.registered_dependency_generation.to_string(),
        }
    }
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "DependencyListV1",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyDependencyListV1 {
    schema_version: u32,
    items: Vec<PySourceDependencyV1>,
}

impl From<RustDependencyListV1> for PyDependencyListV1 {
    fn from(value: RustDependencyListV1) -> Self {
        Self {
            schema_version: value.schema_version,
            items: value.items.into_iter().map(Into::into).collect(),
        }
    }
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "ClosureProofV1",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyClosureProofV1 {
    schema_version: u32,
    proof_write_boundary: String,
    current_active_dependent_nodes: String,
    current_derived_edges: String,
    view_eligible_dependents: String,
    ownerless_projection_rows: String,
    post_admission_registrations: String,
    remaining_dependency_rows: Option<String>,
    remaining_canonical_rows: Option<String>,
    remaining_projection_rows: Option<String>,
    remaining_receipt_reference_rows: Option<String>,
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "ClosureStatusV1",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyClosureStatusV1 {
    schema_version: u32,
    closure_operation_id: String,
    root_type: String,
    source_revision_id: Option<String>,
    source_id: Option<String>,
    cause: String,
    phase: String,
    effective_at_epoch_s: String,
    admitted_write_boundary: String,
    admitted_dependency_generation: String,
    affected_count: String,
    blocker_code: Option<String>,
    proof: Option<PyClosureProofV1>,
}

impl From<RustClosureStatusV1> for PyClosureStatusV1 {
    fn from(value: RustClosureStatusV1) -> Self {
        let (root_type, source_revision_id, source_id) = match value.root {
            ClosureRootV1::SourceRevision { source_revision_id } => {
                ("source_revision".to_string(), Some(source_revision_id.as_str().to_string()), None)
            }
            ClosureRootV1::SourceBucket { source_id } => {
                ("source_bucket".to_string(), None, Some(source_id.as_str().to_string()))
            }
        };
        Self {
            schema_version: value.schema_version,
            closure_operation_id: value.closure_operation_id.as_str().to_string(),
            root_type,
            source_revision_id,
            source_id,
            cause: value.cause.as_str().to_string(),
            phase: value.phase.as_str().to_string(),
            effective_at_epoch_s: value.effective_at_epoch_s.to_string(),
            admitted_write_boundary: value.admitted_write_boundary.to_string(),
            admitted_dependency_generation: value.admitted_dependency_generation.to_string(),
            affected_count: value.affected_count.to_string(),
            blocker_code: value.blocker_code,
            proof: value.proof.map(|proof| PyClosureProofV1 {
                schema_version: proof.schema_version,
                proof_write_boundary: proof.proof_write_boundary.to_string(),
                current_active_dependent_nodes: proof.current_active_dependent_nodes.to_string(),
                current_derived_edges: proof.current_derived_edges.to_string(),
                view_eligible_dependents: proof.view_eligible_dependents.to_string(),
                ownerless_projection_rows: proof.ownerless_projection_rows.to_string(),
                post_admission_registrations: proof.post_admission_registrations.to_string(),
                remaining_dependency_rows: proof
                    .remaining_dependency_rows
                    .map(|item| item.to_string()),
                remaining_canonical_rows: proof
                    .remaining_canonical_rows
                    .map(|item| item.to_string()),
                remaining_projection_rows: proof
                    .remaining_projection_rows
                    .map(|item| item.to_string()),
                remaining_receipt_reference_rows: proof
                    .remaining_receipt_reference_rows
                    .map(|item| item.to_string()),
            }),
        }
    }
}

/// 0.8.20 Slice 5d (R-20-E4) — outcome of the `erase_source` lifecycle verb.
/// Mirrors the Rust `ExciseReport` field-for-field. `projections_invalidated`
/// counts the row-owned projection rows (FTS5 + vec0 + `search_index_v2`)
/// dropped alongside the canonical rows.
#[pyclass(
    module = "fathomdb._fathomdb",
    name = "EraseReport",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyEraseReport {
    source_ref: String,
    nodes_excised: u64,
    edges_excised: u64,
    projections_invalidated: u64,
}

impl PyEraseReport {
    pub(super) fn from_rust(r: RustExciseReport) -> Self {
        Self {
            source_ref: r.source_ref,
            nodes_excised: r.nodes_excised,
            edges_excised: r.edges_excised,
            projections_invalidated: r.projections_invalidated,
        }
    }
}
