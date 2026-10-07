use super::*;

// Shared native carriers and conversions.

#[pyclass(module = "fathomdb._fathomdb", name = "NodeRecord", frozen, get_all, skip_from_py_object)]
#[derive(Clone)]
pub(super) struct PyNodeRecord {
    logical_id: String,
    kind: String,
    body: String,
    write_cursor: u64,
}

impl PyNodeRecord {
    pub(super) fn from_rust(r: &RustNodeRecord) -> Self {
        Self {
            logical_id: r.logical_id.clone(),
            kind: r.kind.clone(),
            body: r.body.clone(),
            write_cursor: r.write_cursor,
        }
    }
}

/// 0.8.20 Slice 10b (R-20-RV / R-20-NV) — the Python face of `ReadView`.
///
/// Idiomatic `snake_case` keyword arguments; every one defaults to the STRICT
/// view, so `ReadView()` (and passing no `view=` at all) reproduces the shipped
/// read behaviour exactly.
///
/// World-time only — there is deliberately no `history_as_of`.
#[pyclass(module = "fathomdb._fathomdb", name = "ReadView", frozen, get_all, skip_from_py_object)]
#[derive(Clone, Default)]
pub(super) struct PyReadView {
    /// Relax `superseded_at IS NULL` — include historical versions.
    include_superseded: bool,
    /// Relax `state = 'active'` — include non-active lifecycle states.
    include_inactive: bool,
    /// Relax the validity window entirely (ignores `valid_as_of`).
    include_out_of_window: bool,
    /// Validity instant, INTEGER epoch SECONDS. `None` = now.
    valid_as_of: Option<i64>,
}

#[pymethods]
impl PyReadView {
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        include_superseded = false,
        include_inactive = false,
        include_out_of_window = false,
        valid_as_of = None,
    ))]
    fn new(
        include_superseded: bool,
        include_inactive: bool,
        include_out_of_window: bool,
        valid_as_of: Option<i64>,
    ) -> Self {
        Self { include_superseded, include_inactive, include_out_of_window, valid_as_of }
    }
}

impl PyReadView {
    pub(super) fn to_rust(&self) -> RustReadView {
        RustReadView {
            include_superseded: self.include_superseded,
            include_inactive: self.include_inactive,
            include_out_of_window: self.include_out_of_window,
            valid_as_of: self.valid_as_of,
        }
    }
}

/// `view=None` means the strict default view.
pub(super) fn read_view_or_default(view: Option<&PyReadView>) -> RustReadView {
    view.map(PyReadView::to_rust).unwrap_or_default()
}

/// Versioned eligibility and validity context for frozen search.
#[pyclass(module = "fathomdb._fathomdb", name = "ReadContextV1", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(super) struct PyReadContextV1 {
    pub(super) inner: RustReadContextV1,
}

#[pymethods]
impl PyReadContextV1 {
    #[new]
    #[pyo3(signature = (
        view=None, source_type=None, kind=None, created_after=None, status=None,
        attributes=None, schema_version=1
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        view: Option<&PyReadView>,
        source_type: Option<String>,
        kind: Option<String>,
        created_after: Option<i64>,
        status: Option<String>,
        attributes: Option<Vec<(String, String)>>,
        schema_version: u32,
    ) -> PyResult<Self> {
        let mut eligibility = RustSearchFilter::default();
        eligibility.source_type = source_type;
        eligibility.kind = kind;
        eligibility.created_after = created_after;
        eligibility.status = status;
        eligibility.attributes = attributes.unwrap_or_default();
        if schema_version != 1 {
            return Err(frozen_read_error_to_py(&fathomdb_engine::FrozenReadError {
                reason: fathomdb_engine::FrozenReadErrorReason::UnsupportedSchemaVersion,
                field_path: "/schemaVersion".into(),
            }));
        }
        let inner = RustReadContextV1::new(read_view_or_default(view), eligibility)
            .map_err(engine_error_to_py)?;
        Ok(Self { inner })
    }

    #[getter]
    fn schema_version(&self) -> u32 {
        self.inner.schema_version
    }
}

/// Engine-minted authenticated read context.
#[pyclass(module = "fathomdb._fathomdb", name = "FrozenReadContextV1", frozen, skip_from_py_object)]
#[derive(Clone)]
pub(super) struct PyFrozenReadContextV1 {
    pub(super) inner: RustFrozenReadContextV1,
}

#[pymethods]
impl PyFrozenReadContextV1 {
    #[new]
    #[pyo3(signature = (effective_valid_at, context, token, schema_version=1))]
    fn new(
        effective_valid_at: i64,
        context: &PyReadContextV1,
        token: String,
        schema_version: u32,
    ) -> Self {
        Self {
            inner: RustFrozenReadContextV1 {
                schema_version,
                effective_valid_at,
                context: context.inner.clone(),
                token,
            },
        }
    }

    #[getter]
    fn schema_version(&self) -> u32 {
        self.inner.schema_version
    }

    #[getter]
    fn effective_valid_at(&self) -> i64 {
        self.inner.effective_valid_at
    }

    #[getter]
    fn token(&self) -> &str {
        &self.inner.token
    }

    #[getter]
    fn context(&self) -> PyReadContextV1 {
        PyReadContextV1 { inner: self.inner.context.clone() }
    }
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "CounterSnapshot",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyCounterSnapshot {
    pub(super) queries: u64,
    pub(super) writes: u64,
    pub(super) write_rows: u64,
    pub(super) admin_ops: u64,
    pub(super) cache_hit: u64,
    pub(super) cache_miss: u64,
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "MigrationStepReport",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyMigrationStepReport {
    step_id: u32,
    duration_ms: Option<u64>,
    failed: bool,
}

impl PyMigrationStepReport {
    pub(super) fn from_rust(r: &RustMigrationStepReport) -> Self {
        Self { step_id: r.step_id, duration_ms: r.duration_ms, failed: r.failed }
    }
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "EmbedderIdentity",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyEmbedderIdentity {
    name: String,
    revision: String,
    dimension: u32,
}

impl PyEmbedderIdentity {
    pub(super) fn from_rust(id: &RustEmbedderIdentity) -> Self {
        Self { name: id.name.clone(), revision: id.revision.clone(), dimension: id.dimension }
    }
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "CudaDeviceInfo",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyCudaDeviceInfo {
    pub(super) ordinal: usize,
    pub(super) uuid: Option<String>,
    pub(super) name: Option<String>,
    pub(super) driver_version: Option<String>,
    pub(super) compute_capability: Option<String>,
    pub(super) cuda_toolkit_version: Option<String>,
}

impl PyCudaDeviceInfo {
    pub(super) fn from_rust(info: &RustCudaDeviceInfo) -> Self {
        Self {
            ordinal: info.ordinal,
            uuid: info.uuid.clone(),
            name: info.name.clone(),
            driver_version: info.driver_version.clone(),
            compute_capability: info.compute_capability.clone(),
            cuda_toolkit_version: info.cuda_toolkit_version.clone(),
        }
    }
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "CudaVisibleDevice",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyCudaVisibleDevice {
    pub(super) visible_ordinal: usize,
    pub(super) uuid: String,
    pub(super) name: String,
    pub(super) compute_capability: Option<String>,
}

impl PyCudaVisibleDevice {
    pub(super) fn from_rust(device: &RustCudaVisibleDevice) -> Self {
        Self {
            visible_ordinal: device.visible_ordinal,
            uuid: device.uuid.clone(),
            name: device.name.clone(),
            compute_capability: device.compute_capability.clone(),
        }
    }
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "EffectiveEmbedDevice",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyEffectiveEmbedDevice {
    pub(super) kind: String,
    pub(super) cuda_device: Option<PyCudaDeviceInfo>,
}

impl PyEffectiveEmbedDevice {
    pub(super) fn from_rust(device: &RustEffectiveEmbedDevice) -> Self {
        match device {
            RustEffectiveEmbedDevice::Cpu => Self { kind: "cpu".to_string(), cuda_device: None },
            RustEffectiveEmbedDevice::Cuda(info) => Self {
                kind: "cuda".to_string(),
                cuda_device: Some(PyCudaDeviceInfo::from_rust(info)),
            },
        }
    }
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "DeviceResolution",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyDeviceResolution {
    pub(super) requested_policy: String,
    pub(super) cuda_compiled: bool,
    pub(super) effective_device: PyEffectiveEmbedDevice,
    pub(super) visible_cuda_devices: Vec<PyCudaVisibleDevice>,
    pub(super) selected_cuda_uuid: Option<String>,
    pub(super) reason: Option<String>,
}

impl PyDeviceResolution {
    pub(super) fn from_rust(resolution: &RustDeviceResolution) -> Self {
        let requested_policy = match resolution.requested_policy {
            RustEmbedDevicePolicy::Auto => "auto".to_string(),
            RustEmbedDevicePolicy::Cpu => "cpu".to_string(),
            RustEmbedDevicePolicy::Cuda(ordinal) => format!("cuda:{ordinal}"),
        };
        Self {
            requested_policy,
            cuda_compiled: resolution.cuda_compiled,
            effective_device: PyEffectiveEmbedDevice::from_rust(&resolution.effective_device),
            visible_cuda_devices: resolution
                .visible_cuda_devices
                .iter()
                .map(PyCudaVisibleDevice::from_rust)
                .collect(),
            selected_cuda_uuid: resolution.selected_cuda_uuid.clone(),
            reason: resolution.reason.map(|reason| reason.as_str().to_string()),
        }
    }

    fn from_reranker(resolution: &RustRerankerDeviceResolution) -> Self {
        let requested_policy = match resolution.requested_policy {
            RustRerankerDevicePolicy::Auto => "auto".to_string(),
            RustRerankerDevicePolicy::Cpu => "cpu".to_string(),
            RustRerankerDevicePolicy::Cuda(ordinal) => format!("cuda:{ordinal}"),
        };
        let effective_device = match &resolution.effective_device {
            RustEffectiveRerankerDevice::Cpu => {
                PyEffectiveEmbedDevice { kind: "cpu".to_string(), cuda_device: None }
            }
            RustEffectiveRerankerDevice::Cuda(info) => PyEffectiveEmbedDevice {
                kind: "cuda".to_string(),
                cuda_device: Some(PyCudaDeviceInfo::from_rust(info)),
            },
        };
        Self {
            requested_policy,
            cuda_compiled: resolution.cuda_compiled,
            effective_device,
            visible_cuda_devices: resolution
                .visible_cuda_devices
                .iter()
                .map(PyCudaVisibleDevice::from_rust)
                .collect(),
            selected_cuda_uuid: resolution.selected_cuda_uuid.clone(),
            reason: resolution.reason.map(|reason| reason.as_str().to_string()),
        }
    }
}

/// 0.8.23 Slice 80.6 (D-80.6-6, R80-13) — the retained
/// `fathomdb.tegra-gpu-allocation-witness/v1` record, measured in this
/// process by the artifact under test.
///
/// Every number the verdict used is carried, so a reader re-derives the
/// verdict instead of trusting it: the raw free-memory samples bracketing the
/// model load, the declared floor they were judged against, and the deliberate
/// control allocation that proves the shared iGPU counter was live and
/// attributable at the time. Byte counts are exact Python ints — the deltas
/// are `i128` in the core and are not narrowed here.
#[pyclass(
    module = "fathomdb._fathomdb",
    name = "GpuAllocationWitness",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyGpuAllocationWitness {
    /// Schema string of the retained record.
    pub(super) schema: String,
    /// The precondition the witness run states rather than assumes.
    pub(super) sole_gpu_consumer_precondition: String,
    pub(super) device_ordinal_requested: usize,
    pub(super) device_ordinal_actual: usize,
    pub(super) device_uuid: String,
    pub(super) device_name: String,
    pub(super) compute_capability: String,
    pub(super) free_before_bytes: u64,
    pub(super) free_after_bytes: u64,
    pub(super) total_bytes: u64,
    pub(super) delta_bytes: i128,
    pub(super) delta_floor_bytes: u64,
    pub(super) control_allocation_request_bytes: u64,
    pub(super) control_block_count: usize,
    pub(super) control_free_before_bytes: u64,
    pub(super) control_free_after_bytes: u64,
    pub(super) control_delta_bytes: i128,
    pub(super) embedded_vector_dim: usize,
}

impl PyGpuAllocationWitness {
    pub(super) fn from_rust(witness: &RustGpuAllocationWitness) -> Self {
        Self {
            schema: TEGRA_GPU_ALLOCATION_WITNESS_SCHEMA.to_string(),
            sole_gpu_consumer_precondition: SOLE_GPU_CONSUMER_PRECONDITION.to_string(),
            device_ordinal_requested: witness.device_ordinal_requested,
            device_ordinal_actual: witness.device_ordinal_actual,
            device_uuid: witness.device_uuid.clone(),
            device_name: witness.device_name.clone(),
            compute_capability: witness.compute_capability.clone(),
            free_before_bytes: witness.free_before_bytes,
            free_after_bytes: witness.free_after_bytes,
            total_bytes: witness.total_bytes,
            delta_bytes: witness.delta_bytes,
            delta_floor_bytes: witness.delta_floor_bytes,
            control_allocation_request_bytes: witness.control_allocation_request_bytes,
            control_block_count: witness.control_block_count,
            control_free_before_bytes: witness.control_free_before_bytes,
            control_free_after_bytes: witness.control_free_after_bytes,
            control_delta_bytes: witness.control_delta_bytes,
            embedded_vector_dim: witness.embedded_vector_dim,
        }
    }
}

#[pyclass(module = "fathomdb._fathomdb", name = "OpenReport", frozen, get_all)]
pub(super) struct PyOpenReport {
    pub(super) schema_version_before: u32,
    pub(super) schema_version_after: u32,
    pub(super) migration_steps: Vec<PyMigrationStepReport>,
    pub(super) embedder_warmup_ms: u64,
    pub(super) query_backend: String,
    pub(super) default_embedder: PyEmbedderIdentity,
    // EU-5a1/5a2/5b — surfaced to Python verbatim (snake_case).
    /// Wall-time milliseconds the EU-3 loader spent fetching default-
    /// embedder weights, or `None` on full cache hit / caller-supplied
    /// embedder. See `dev/design/embedder.md` §7.
    pub(super) embedder_download_ms: Option<u64>,
    /// Structured loader events (downloads, cache hits, mean-vec pin).
    /// Each item is a `dict` keyed by `"kind"` with variant-specific
    /// payload keys. See [`embedder_event_to_py`] for the per-variant
    /// shape.
    pub(super) embedder_events: Vec<Py<PyAny>>,
    /// Static identity capability — true when the configured default
    /// embedder requires mean-centering (e.g. bge-small).
    pub(super) embedder_mean_centering_required: bool,
    /// Dynamic workspace state — true iff
    /// `_fathomdb_embedder_profiles.mean_vec IS NOT NULL`.
    pub(super) embedder_mean_vec_pinned: bool,
    /// 0.8.18 Slice 5 (#5 vector-equivalence probe, R-VEQ-6) — `True` iff the
    /// open-time #5 self-check found a vector-equivalence divergence beyond the
    /// D4 floor and every vector-dependent arm now refuses at query time with
    /// `VectorEquivalenceMismatchError`. The text-only/FTS-only path
    /// (`search_text_only`) stays serviceable.
    pub(super) dense_disabled: bool,
    /// R-VEQ-6 — human-readable reason for `dense_disabled` (which representation
    /// tripped), or `None` when dense is healthy.
    pub(super) dense_disabled_reason: Option<String>,
    /// Strict CPU/CUDA selection used to construct the embedder, or `None` when
    /// no embedder was configured.
    pub(super) embedder_device_resolution: Option<PyDeviceResolution>,
    pub(super) reranker_device_resolution: Option<PyDeviceResolution>,
    /// 0.8.23 Slice 80.6 (D-80.6-6, AC80-6) — the in-process GPU allocation
    /// witness, or `None` when this open measured none.
    ///
    /// `None` means **no witness was taken**, never "a witness measured
    /// nothing": a zero, negative, or below-floor allocation delta is a typed
    /// failure inside the witness and fails the open, so a zero-valued record
    /// is not reachable here.
    pub(super) embedder_gpu_allocation_witness: Option<PyGpuAllocationWitness>,
}

impl PyOpenReport {
    pub(super) fn from_rust(py: Python<'_>, r: &RustOpenReport) -> Self {
        let embedder_events =
            r.embedder_events.iter().map(|ev| embedder_event_to_py(py, ev)).collect();
        Self {
            schema_version_before: r.schema_version_before,
            schema_version_after: r.schema_version_after,
            migration_steps: r
                .migration_steps
                .iter()
                .map(PyMigrationStepReport::from_rust)
                .collect(),
            embedder_warmup_ms: r.embedder_warmup_ms,
            query_backend: r.query_backend.to_string(),
            default_embedder: PyEmbedderIdentity::from_rust(&r.default_embedder),
            embedder_download_ms: r.embedder_download_ms,
            embedder_events,
            embedder_mean_centering_required: r.embedder_mean_centering_required,
            embedder_mean_vec_pinned: r.embedder_mean_vec_pinned,
            dense_disabled: r.dense_disabled,
            dense_disabled_reason: r.dense_disabled_reason.clone(),
            embedder_device_resolution: r
                .embedder_device_resolution
                .as_ref()
                .map(PyDeviceResolution::from_rust),
            reranker_device_resolution: r
                .reranker_device_resolution
                .as_ref()
                .map(PyDeviceResolution::from_reranker),
            embedder_gpu_allocation_witness: r
                .embedder_gpu_allocation_witness
                .as_ref()
                .map(PyGpuAllocationWitness::from_rust),
        }
    }
}

/// Serialise one [`RustEmbedderEvent`] as a Python `dict`. The `kind`
/// key carries the variant name (`"DefaultEmbedderDownload"`,
/// `"DefaultEmbedderCacheHit"`, `"MeanVecPinned"`); the remaining keys
/// carry the variant payload in snake_case. We pick a dict (rather than
/// a per-variant `#[pyclass]`) so callers can pattern-match on the
/// `"kind"` discriminant without importing leaf classes.
pub(super) fn embedder_event_to_py(py: Python<'_>, ev: &RustEmbedderEvent) -> Py<PyAny> {
    let dict = PyDict::new(py);
    match ev {
        RustEmbedderEvent::DefaultEmbedderDownload {
            file,
            url,
            bytes,
            sha256,
            cache_path,
            duration_ms,
        } => {
            let _ = dict.set_item("kind", "DefaultEmbedderDownload");
            let _ = dict.set_item("file", file);
            let _ = dict.set_item("url", url);
            let _ = dict.set_item("bytes", *bytes);
            let _ = dict.set_item("sha256", sha256);
            let _ = dict.set_item("cache_path", cache_path.display().to_string());
            let _ = dict.set_item("duration_ms", *duration_ms);
        }
        RustEmbedderEvent::DefaultEmbedderCacheHit { file, sha256, cache_path } => {
            let _ = dict.set_item("kind", "DefaultEmbedderCacheHit");
            let _ = dict.set_item("file", file);
            let _ = dict.set_item("sha256", sha256);
            let _ = dict.set_item("cache_path", cache_path.display().to_string());
        }
        RustEmbedderEvent::MeanVecPinned { dim, doc_count } => {
            let _ = dict.set_item("kind", "MeanVecPinned");
            let _ = dict.set_item("dim", *dim);
            let _ = dict.set_item("doc_count", *doc_count);
        }
        RustEmbedderEvent::MeanVecRecomputed { dim, doc_count, trigger } => {
            let _ = dict.set_item("kind", "MeanVecRecomputed");
            let _ = dict.set_item("dim", *dim);
            let _ = dict.set_item("doc_count", *doc_count);
            let _ = dict.set_item("trigger", trigger.as_str());
        }
    }
    dict.into()
}
