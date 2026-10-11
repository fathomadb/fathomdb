use super::*;

// ===== Engine =========================================================

pub(super) fn optional_engine_config_value(
    config: &Bound<'_, PyDict>,
    name: &str,
    minimum: u64,
    maximum: u64,
) -> PyResult<Option<u64>> {
    let Some(value) = config.get_item(name)? else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    if value.is_instance_of::<PyBool>() || !value.is_instance_of::<PyInt>() {
        return Err(PyTypeError::new_err(format!("{name} must be an integer")));
    }
    let integer = value
        .extract::<i128>()
        .map_err(|_| PyValueError::new_err(format!("{name} is outside the accepted range")))?;
    if integer < i128::from(minimum) || integer > i128::from(maximum) {
        return Err(PyValueError::new_err(format!("{name} must be in {minimum}..={maximum}")));
    }
    Ok(Some(integer as u64))
}

pub(super) fn engine_config_from_py(
    config: Option<&Bound<'_, PyDict>>,
) -> PyResult<RustEngineConfig> {
    let Some(config) = config else {
        return Ok(RustEngineConfig::default());
    };
    for (key, _) in config.iter() {
        let name = key
            .extract::<String>()
            .map_err(|_| PyTypeError::new_err("config keys must be strings"))?;
        if !matches!(
            name.as_str(),
            "scheduler_runtime_threads"
                | "embedder_pool_size"
                | "embedder_call_timeout_ms"
                | "provenance_row_cap"
                | "slow_threshold_ms"
        ) {
            return Err(PyTypeError::new_err(format!("unknown engine config field: {name}")));
        }
    }
    Ok(RustEngineConfig {
        scheduler_runtime_threads: optional_engine_config_value(
            config,
            "scheduler_runtime_threads",
            1,
            64,
        )?,
        embedder_pool_size: optional_engine_config_value(config, "embedder_pool_size", 1, 64)?,
        embedder_call_timeout_ms: optional_engine_config_value(
            config,
            "embedder_call_timeout_ms",
            1,
            u64::from(u32::MAX),
        )?,
        provenance_row_cap: optional_engine_config_value(
            config,
            "provenance_row_cap",
            0,
            (1_u64 << 53) - 1,
        )?,
        slow_threshold_ms: optional_engine_config_value(
            config,
            "slow_threshold_ms",
            0,
            (1_u64 << 53) - 1,
        )?,
    })
}

#[pyclass(module = "fathomdb._fathomdb", name = "Engine")]
pub(super) struct PyEngine {
    pub(super) inner: Arc<RustEngine>,
    pub(super) open_report: Arc<RustOpenReport>,
    pub(super) logging: logging_subscriber::LoggingSlot,
}

#[pymethods]
impl PyEngine {
    #[staticmethod]
    #[pyo3(signature = (path, use_default_embedder = false, config = None))]
    pub(super) fn open(
        py: Python<'_>,
        path: String,
        use_default_embedder: bool,
        config: Option<Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        logging_subscriber::reject_reentry()?;
        validate_ffi_string_py(&path)?;
        let config = engine_config_from_py(config.as_ref())?;
        let opened = py
            .detach(|| {
                catch_unwind(AssertUnwindSafe(|| {
                    // EU-6: True → `EmbedderChoice::Default` (engine
                    // materialises the pinned bge-small embedder via the
                    // EU-3 loader); False → `EmbedderChoice::None`
                    // (engine opens; vector writes fail
                    // EmbedderNotConfigured). Caller-supplied custom
                    // embedders are deferred to a future slice per
                    // ADR-0.6.0-embedder-protocol Invariant 3.
                    let choice = if use_default_embedder {
                        EmbedderChoice::Default
                    } else {
                        EmbedderChoice::None
                    };
                    RustEngine::open_with_choice_and_config(path, choice, config)
                }))
            })
            .map_err(|_| PanicException::new_err("engine panic during open"))?
            .map_err(engine_open_error_to_py)?;
        let _ = py; // used inside the conversion below via the GIL handle.
        Ok(Self {
            inner: Arc::new(opened.engine),
            open_report: Arc::new(opened.report),
            logging: logging_subscriber::LoggingSlot::new(),
        })
    }

    fn open_report(&self, py: Python<'_>) -> PyOpenReport {
        PyOpenReport::from_rust(py, &self.open_report)
    }

    /// Mint a restart-stable authenticated read context.
    fn freeze_read_context(
        &self,
        py: Python<'_>,
        context: &PyReadContextV1,
    ) -> PyResult<PyFrozenReadContextV1> {
        let engine = Arc::clone(&self.inner);
        let context = context.inner.clone();
        call_engine(py, move || engine.freeze_read_context(&context))
            .map(|inner| PyFrozenReadContextV1 { inner })
    }

    /// Validate a frozen context before Python converts dynamic controls.
    fn validate_frozen_read_context(
        &self,
        py: Python<'_>,
        context: &PyFrozenReadContextV1,
    ) -> PyResult<()> {
        let engine = Arc::clone(&self.inner);
        let context = context.inner.clone();
        call_engine(py, move || engine.validate_frozen_read_context_for_binding(&context))
    }

    /// Execute one canonical JSON graph-expansion request.
    fn graph_expand(&self, py: Python<'_>, request_json: String) -> PyResult<String> {
        validate_ffi_string_py(&request_json)?;
        let request = decode_graph_expand_request_v1(request_json.as_bytes())
            .map_err(|error| graph_expansion_error_to_py(&error))?;
        let engine = Arc::clone(&self.inner);
        let result = call_engine(py, move || engine.graph_expand(&request))?;
        let encoded = encode_graph_expand_result_v1(&result)
            .map_err(|error| graph_expansion_error_to_py(&error))?;
        String::from_utf8(encoded).map_err(|_| GraphExpansionError::new_err("graph_corrupt at "))
    }

    /// Return canonical version-1 JSON for one governed dependency trace.
    #[pyo3(signature = (root_revision_id, direction, context, max_relations=100, max_work_units=101))]
    fn trace_dependency(
        &self,
        py: Python<'_>,
        root_revision_id: String,
        direction: &str,
        context: &PyFrozenReadContextV1,
        max_relations: u32,
        max_work_units: u32,
    ) -> PyResult<String> {
        validate_ffi_string_py(&root_revision_id)?;
        let direction = match direction {
            "to_source" => RustDependencyTraceDirectionV1::ToSource,
            "to_dependents" => RustDependencyTraceDirectionV1::ToDependents,
            _ => return Err(dependency_trace_error("trace_direction_invalid", "/direction")),
        };
        let request =
            RustDependencyTraceRequestV1::new(root_revision_id, direction, context.inner.clone())
                .and_then(|request| request.with_bounds(max_relations, max_work_units))
                .map_err(|error| engine_error_to_py(error.into()))?;
        let engine = Arc::clone(&self.inner);
        let result = call_engine(py, move || engine.trace_dependency(request))?;
        let bytes = encode_dependency_trace_result_v1(&result)
            .map_err(|error| engine_error_to_py(error.into()))?;
        String::from_utf8(bytes).map_err(|_| EngineError::new_err("trace codec failure"))
    }

    /// Search using eligibility and validity authenticated by a frozen context.
    #[pyo3(signature = (
        query, context, rerank_depth=0, use_graph_arm=false, alpha=0.3,
        pool_n=None, explain=false, limit=10
    ))]
    #[allow(clippy::too_many_arguments)]
    fn search_frozen(
        &self,
        py: Python<'_>,
        query: &str,
        context: &PyFrozenReadContextV1,
        rerank_depth: i64,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: Option<i64>,
        explain: bool,
        limit: i64,
    ) -> PyResult<PySearchResult> {
        let pool_n = pool_n.unwrap_or(rerank_depth);
        let engine = Arc::clone(&self.inner);
        let query = query.to_string();
        let context = context.inner.clone();
        call_engine(py, move || {
            engine.search_frozen(
                &query,
                &context,
                usize::try_from(rerank_depth).unwrap_or(usize::MAX),
                use_graph_arm,
                alpha,
                usize::try_from(pool_n).unwrap_or(usize::MAX),
                explain,
                usize::try_from(limit).unwrap_or(usize::MAX),
            )
        })
        .map(PySearchResult::from_rust)
    }

    /// Search and attach one authenticated evidence reference per result.
    #[pyo3(signature = (
        query, context, rerank_depth=0, use_graph_arm=false, alpha=0.3,
        pool_n=0, include_explanation=false, limit=10
    ))]
    #[allow(clippy::too_many_arguments)]
    fn search_with_evidence(
        &self,
        py: Python<'_>,
        query: &str,
        context: &PyFrozenReadContextV1,
        rerank_depth: i64,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: i64,
        include_explanation: bool,
        limit: i64,
    ) -> PyResult<PyEvidenceSearchResultV1> {
        let rerank_depth = u32::try_from(rerank_depth).map_err(|_| {
            engine_error_to_py(RustEngineError::InvalidArgument {
                msg: "rerank_depth must be in 0..=4294967295".to_string(),
            })
        })?;
        let pool_n = u32::try_from(pool_n).map_err(|_| {
            engine_error_to_py(RustEngineError::InvalidArgument {
                msg: "pool_n must be in 0..=4294967295".to_string(),
            })
        })?;
        let limit = u32::try_from(limit).map_err(|_| {
            engine_error_to_py(RustEngineError::InvalidArgument {
                msg: "limit must be in 0..=4294967295".to_string(),
            })
        })?;
        let request = RustEvidenceSearchRequestV1 {
            schema_version: 1,
            query: query.to_string(),
            context: context.inner.clone(),
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            include_explanation,
            limit,
        };
        let engine = Arc::clone(&self.inner);
        call_engine(py, move || engine.search_with_evidence(&request)).map(Into::into)
    }

    /// Resolve one authenticated evidence reference under an equivalent context.
    fn resolve_evidence(
        &self,
        py: Python<'_>,
        evidence_ref: &str,
        context: &PyFrozenReadContextV1,
    ) -> PyResult<PyResolvedEvidenceV1> {
        let request = RustEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: RustEvidenceRefV1::new(evidence_ref)
                .map_err(|error| engine_error_to_py(RustEngineError::Evidence(error)))?,
            context: context.inner.clone(),
        };
        let engine = Arc::clone(&self.inner);
        call_engine(py, move || engine.resolve_evidence(&request)).map(Into::into)
    }

    /// Resolve one exact artifact disclosed by frozen graph expansion.
    fn resolve_graph_evidence(
        &self,
        py: Python<'_>,
        evidence_ref: String,
        context: &PyFrozenReadContextV1,
    ) -> PyResult<String> {
        validate_ffi_string_py(&evidence_ref)?;
        let request = RustGraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: RustGraphEvidenceRefV1::new(evidence_ref)
                .map_err(|error| engine_error_to_py(RustEngineError::Evidence(error)))?,
            context: context.inner.clone(),
        };
        let engine = Arc::clone(&self.inner);
        let value = call_engine(py, move || engine.resolve_graph_evidence(&request))?;
        let bytes = encode_resolved_graph_evidence_v1(&value)
            .map_err(|error| engine_error_to_py(RustEngineError::Evidence(error)))?;
        String::from_utf8(bytes).map_err(|_| PyValueError::new_err("evidence_corrupt at "))
    }

    /// Search and expand under a frozen context.
    #[pyo3(signature = (query, context, depth, limit=10))]
    fn search_expand_frozen(
        &self,
        py: Python<'_>,
        query: &str,
        context: &PyFrozenReadContextV1,
        depth: i64,
        limit: i64,
    ) -> PyResult<PySearchExpandResult> {
        let engine = Arc::clone(&self.inner);
        let query = query.to_string();
        let context = context.inner.clone();
        call_engine(py, move || {
            engine.search_expand_frozen(
                &query,
                &context,
                u32::try_from(depth).unwrap_or(u32::MAX),
                usize::try_from(limit).unwrap_or(usize::MAX),
            )
        })
        .map(PySearchExpandResult::from_rust)
    }

    fn write(&self, py: Python<'_>, batch: Bound<'_, PyList>) -> PyResult<PyWriteReceipt> {
        let prepared = translate_batch(&batch)?;
        let engine = Arc::clone(&self.inner);
        let receipt = call_engine(py, move || engine.write(&prepared))?;
        Ok(PyWriteReceipt::from_rust(receipt))
    }

    fn actuate(
        &self,
        py: Python<'_>,
        request: &Bound<'_, PyAny>,
    ) -> PyResult<PyActuationReceiptV1> {
        let request = translate_actuation_request(request)?;
        let engine = Arc::clone(&self.inner);
        call_engine(py, move || engine.actuate(request)).map(Into::into)
    }

    fn register_source_dependency(
        &self,
        py: Python<'_>,
        request: &Bound<'_, PyAny>,
    ) -> PyResult<PySourceDependencyV1> {
        let request = translate_dependency_registration(request)?;
        let engine = Arc::clone(&self.inner);
        call_engine(py, move || engine.register_source_dependency(request)).map(Into::into)
    }

    fn dependencies_for_source(
        &self,
        py: Python<'_>,
        request: &Bound<'_, PyAny>,
    ) -> PyResult<PyDependencyListV1> {
        let request = translate_dependency_source_lookup(request)?;
        let engine = Arc::clone(&self.inner);
        call_engine(py, move || engine.dependencies_for_source(request)).map(Into::into)
    }

    fn dependency_for_derived(
        &self,
        py: Python<'_>,
        request: &Bound<'_, PyAny>,
    ) -> PyResult<Option<PySourceDependencyV1>> {
        let request = translate_dependency_derived_lookup(request)?;
        let engine = Arc::clone(&self.inner);
        call_engine(py, move || engine.dependency_for_derived(request))
            .map(|value| value.map(Into::into))
    }

    fn read_dependency_closure(
        &self,
        py: Python<'_>,
        request: &Bound<'_, PyAny>,
    ) -> PyResult<Option<PyClosureStatusV1>> {
        let request = translate_closure_lookup(request)?;
        let engine = Arc::clone(&self.inner);
        call_engine(py, move || engine.read_dependency_closure(request))
            .map(|value| value.map(Into::into))
    }

    /// G10 + 0.8.1 R1 — hybrid search with an optional closed metadata filter
    /// and an optional CE rerank depth. Each filter field is an optional kwarg;
    /// all-`None` is the unfiltered (byte-identical) path. `rerank_depth=0`
    /// (default) keeps the identity / soft-fallback path. `rerank_depth > 0`
    /// activates CE reranking over the top-N fused hits (when the
    /// `default-reranker` feature is enabled and the model is loaded; otherwise
    /// falls back to identity).
    // 0.8.1 R1/R3: rerank_depth and use_graph_arm add 8th arg; suppress lint.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(
        signature = (query, source_type=None, kind=None, created_after=None,
                     status=None, rerank_depth=0, use_graph_arm=false,
                     alpha=None, pool_n=None, explain=false, attributes=None, view=None, limit=10)
    )]
    fn search(
        &self,
        py: Python<'_>,
        query: &str,
        source_type: Option<Bound<'_, PyAny>>,
        kind: Option<Bound<'_, PyAny>>,
        created_after: Option<i64>,
        status: Option<Bound<'_, PyAny>>,
        rerank_depth: usize,
        // 0.8.1 R3 (Slice 30) — when True, seed BFS over temporal fact-edges
        // from the top-10 fused hits and fuse reachable nodes as a third RRF arm.
        // Default False → byte-identical to the pre-Slice-30 two-arm pipeline.
        use_graph_arm: bool,
        // 0.8.5 (EXP-0) — CE-rerank knobs. `alpha` (default 0.3) is the CE-blend
        // weight, clamped to [0,1] in the engine; `pool_n` (default = rerank_depth)
        // is the reranked-pool size. Omitting both reproduces the byte-identical
        // default ranking; `alpha=1.0, pool_n=10` is the measured-parity config.
        alpha: Option<f64>,
        pool_n: Option<usize>,
        // 0.8.8 EXP-OBS (Slice 10) — when True, populate `SearchResult.explanation`
        // with per-hit provenance + score breakdown + query trace. Default False
        // returns `explanation=None` and a byte-identical result (R-OBS-2 zero-cost).
        explain: bool,
        attributes: Option<Vec<(String, String)>>,
        // 0.8.20 Slice 15b fix-2 (R-20-NV / R-20-RV) — optional validity view,
        // the same kwarg the five read verbs take. `None` (the default) is the
        // strict view: active-only, non-superseded, and valid AT QUERY TIME.
        // `ReadView(include_out_of_window=True)` returns hits whatever their
        // window; `ReadView(valid_as_of=t)` evaluates validity at the bound
        // instant `t`. The existence flags are REFUSED here (typed
        // `InvalidArgumentError`), never silently ignored — see
        // `Engine::search_view`.
        view: Option<&PyReadView>,
        limit: usize,
    ) -> PyResult<PySearchResult> {
        validate_ffi_string_py(query)?;
        // G10 filter strings cross the FFI exactly like `query` and the write
        // fields, so they go through the same validation gate
        // (`extract_validated_str`: rejects embedded NUL and lone UTF-16
        // surrogate as the typed `WriteValidationError`). `None` stays `None`
        // so the all-`None` filter remains the byte-identical unfiltered path.
        let source_type = extract_opt_validated_str(source_type.as_ref())?;
        let kind = extract_opt_validated_str(kind.as_ref())?;
        let status = extract_opt_validated_str(status.as_ref())?;
        let attributes = attributes.unwrap_or_default();
        for (name, value) in &attributes {
            validate_ffi_string_py(name)?;
            validate_ffi_string_py(value)?;
        }
        let engine = Arc::clone(&self.inner);
        let query = query.to_string();
        let filter = if source_type.is_some()
            || kind.is_some()
            || created_after.is_some()
            || status.is_some()
            || !attributes.is_empty()
        {
            // `RustSearchFilter` is `#[non_exhaustive]` (0.8.20 Slice 15e fix-2),
            // so an out-of-defining-crate struct literal — even with
            // `..Default::default()` — is rejected; build from `default()` and
            // set the four legacy metadata fields. `attributes` is NOT exposed on
            // the Py wire in 0.8.20 (engine-internal), so it is left at its default.
            let mut f = RustSearchFilter::default();
            f.source_type = source_type;
            f.kind = kind;
            f.created_after = created_after;
            f.status = status;
            f.attributes = attributes;
            Some(f)
        } else {
            None
        };
        // 0.8.1 R1: use search_reranked so rerank_depth=0 is a no-op (identity)
        // and rerank_depth>0 activates the CE path.
        // 0.8.1 R3: use_graph_arm=True activates the graph-BFS third arm.
        // 0.8.5 (D4): resolve the binding-side defaults — α=0.3, pool_n=rerank_depth —
        // so an unset call reproduces the pre-slice ranking exactly. α is clamped in
        // the engine's `ce_rerank`.
        let alpha = alpha.unwrap_or(0.3);
        let pool_n = pool_n.unwrap_or(rerank_depth);
        // Resolved BEFORE the GIL is released, exactly as the read verbs do.
        let view = read_view_or_default(view);
        // 0.8.8 EXP-OBS: `explain=True` routes to `search_explained` (same retrieval,
        // plus the sidecar); `explain=False` (default) stays on `search_reranked`.
        // fix-2: ONE call now that the view rides the full-arity entry point —
        // `explain` is a parameter of it, so the explain/non-explain split no
        // longer duplicates the argument list (and cannot drift on `view`).
        let result = call_engine(py, move || {
            engine.search_reranked_view_with_limit(
                &query,
                filter,
                rerank_depth,
                use_graph_arm,
                alpha,
                pool_n,
                explain,
                &view,
                limit,
            )
        })?;
        Ok(PySearchResult::from_rust(result))
    }

    /// Lexically search exactly one declared `searchable→FTS` projection.
    #[pyo3(signature = (query, name, source_type=None, kind=None, created_after=None, status=None, attributes=None, view=None, limit=10))]
    #[allow(clippy::too_many_arguments)]
    fn search_projected_text(
        &self,
        py: Python<'_>,
        query: &str,
        name: &str,
        source_type: Option<Bound<'_, PyAny>>,
        kind: Option<Bound<'_, PyAny>>,
        created_after: Option<i64>,
        status: Option<Bound<'_, PyAny>>,
        attributes: Option<Vec<(String, String)>>,
        view: Option<&PyReadView>,
        limit: usize,
    ) -> PyResult<PySearchResult> {
        validate_ffi_string_py(query)?;
        validate_ffi_string_py(name)?;
        let source_type = extract_opt_validated_str(source_type.as_ref())?;
        let kind = extract_opt_validated_str(kind.as_ref())?;
        let status = extract_opt_validated_str(status.as_ref())?;
        let attributes = attributes.unwrap_or_default();
        for (attribute, value) in &attributes {
            validate_ffi_string_py(attribute)?;
            validate_ffi_string_py(value)?;
        }
        let filter = if source_type.is_some()
            || kind.is_some()
            || created_after.is_some()
            || status.is_some()
            || !attributes.is_empty()
        {
            let mut filter = RustSearchFilter::default();
            filter.source_type = source_type;
            filter.kind = kind;
            filter.created_after = created_after;
            filter.status = status;
            filter.attributes = attributes;
            Some(filter)
        } else {
            None
        };
        let view = read_view_or_default(view);
        let engine = Arc::clone(&self.inner);
        let query = query.to_string();
        let name = name.to_string();
        let result = call_engine(py, move || {
            engine.search_projected_text_with_limit(&query, &name, filter, &view, limit)
        })?;
        Ok(PySearchResult::from_rust(result))
    }

    /// 0.8.18 Slice 5 (#5 vector-equivalence probe, R-VEQ-4) — the explicit
    /// text-only / FTS-only search path. Does NOT embed the query and NEVER raises
    /// `VectorEquivalenceMismatchError`, so it stays serviceable when the engine
    /// opened in the degraded `dense_disabled` state. Matching node- and edge-body
    /// FTS candidates are deterministically body-deduplicated and ranked before
    /// `limit`; no vector recall, CE rerank, or graph arm runs on this path.
    ///
    /// 0.8.20 Slice 15b fix-2 — takes the same optional `view` as `search`.
    #[pyo3(signature = (query, view=None, limit=10))]
    fn search_text_only(
        &self,
        py: Python<'_>,
        query: &str,
        view: Option<&PyReadView>,
        limit: usize,
    ) -> PyResult<PySearchResult> {
        validate_ffi_string_py(query)?;
        let engine = Arc::clone(&self.inner);
        let query = query.to_string();
        let view = read_view_or_default(view);
        let result =
            call_engine(py, move || engine.search_text_only_view_with_limit(&query, &view, limit))?;
        Ok(PySearchResult::from_rust(result))
    }

    /// 0.8.18 Slice 5 (R-VEQ-6) — `True` iff the engine opened degraded (the #5
    /// self-check found a vector-equivalence divergence and every dense arm is
    /// refusing). Mirrors `OpenReport.dense_disabled`.
    fn dense_disabled(&self) -> bool {
        self.inner.dense_disabled()
    }

    /// 0.8.18 Slice 5 (R-VEQ-6) — the human-readable reason for the degraded state,
    /// or `None` when dense is healthy.
    fn dense_disabled_reason(&self) -> Option<String> {
        self.inner.dense_disabled_reason()
    }

    /// 0.8.18 Slice 5 (R-VEQ-6) — telemetry counter: query-time dense-arm refusals
    /// raised because the engine opened degraded.
    fn vector_equivalence_refusal_count(&self) -> u64 {
        self.inner.vector_equivalence_refusal_count()
    }

    fn close(&self, py: Python<'_>) -> PyResult<()> {
        logging_subscriber::reject_reentry()?;
        self.logging.close_start();
        let engine = Arc::clone(&self.inner);
        let result = call_engine(py, move || engine.close());
        self.logging.close_finish();
        result
    }

    #[cfg(feature = "test-hooks")]
    fn _arm_next_reader_snapshot_pause_for_test(&self) -> PyWalSnapshotPause {
        let (snapshot_ready, release, reader_native_state) =
            self.inner.arm_next_reader_snapshot_pause_for_test();
        PyWalSnapshotPause {
            snapshot_ready,
            release,
            reader_autocommit: None,
            reader_native_state: Some(reader_native_state),
        }
    }

    #[cfg(feature = "test-hooks")]
    fn _arm_next_reader_completion_pause_for_test(&self) -> PyWalSnapshotPause {
        let (snapshot_ready, release, reader_autocommit) =
            self.inner.arm_next_reader_completion_pause_for_test();
        PyWalSnapshotPause {
            snapshot_ready,
            release,
            reader_autocommit: Some(reader_autocommit),
            reader_native_state: None,
        }
    }

    #[cfg(feature = "test-hooks")]
    fn _wal_attribution_checkpoint_records_for_test(
        &self,
    ) -> Vec<(usize, bool, String, Vec<String>)> {
        self.inner.wal_attribution_checkpoint_records_for_test()
    }

    #[cfg(feature = "test-hooks")]
    fn _wal_attribution_snapshot_for_test<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let record = PyDict::new(py);
        record.set_item("no_owned_snapshot", self.inner.wal_attribution_idle_for_test())?;
        Ok(record)
    }

    #[cfg(feature = "test-hooks")]
    fn _arm_actual_checkpoint_observation_for_test(&self) {
        self.inner.arm_python_serial_actual_checkpoint_observation_for_test();
    }

    #[cfg(feature = "test-hooks")]
    fn _drain_actual_checkpoint_observations_for_test(&self) -> Vec<String> {
        self.inner.drain_actual_checkpoint_observations_for_test()
    }

    #[cfg(feature = "test-hooks")]
    fn _wal_attribution_binding_inventory_for_test(&self, py: Python<'_>) -> PyResult<String> {
        let engine = Arc::clone(&self.inner);
        call_engine(py, move || engine.binding_connection_inventory_for_test())
    }

    #[cfg(feature = "test-hooks")]
    fn _wal_attribution_binding_native_state_inventory_for_test(
        &self,
        py: Python<'_>,
    ) -> PyResult<String> {
        let engine = Arc::clone(&self.inner);
        call_engine(py, move || engine.binding_native_state_inventory_for_test())
    }

    #[cfg(feature = "test-hooks")]
    fn _arm_binding_native_state_observation_for_test(&self) {
        self.inner.arm_binding_native_state_observation_for_test();
    }

    #[cfg(feature = "test-hooks")]
    fn _drain_binding_native_state_observations_for_test(&self) -> Vec<String> {
        self.inner.drain_binding_native_state_observations_for_test()
    }

    #[cfg(feature = "test-hooks")]
    fn _checkpoint_at_rest_for_test(&self, py: Python<'_>) -> PyResult<Vec<(bool, u32, u32)>> {
        let engine = Arc::clone(&self.inner);
        call_engine(py, move || engine.checkpoint_at_rest_for_test())
    }

    #[pyo3(signature = (timeout_s = 0.0))]
    fn drain(&self, py: Python<'_>, timeout_s: f64) -> PyResult<()> {
        let ms =
            if timeout_s.is_finite() && timeout_s > 0.0 { (timeout_s * 1000.0) as u64 } else { 0 };
        let engine = Arc::clone(&self.inner);
        call_engine(py, move || engine.drain(ms))
    }

    /// 0.8.8 Slice 15 (OPP-9) — enable opt-in local telemetry capture to a JSONL
    /// `sink_path`. Off by default; local file only (no egress).
    fn enable_telemetry(&self, py: Python<'_>, sink_path: &str) -> PyResult<()> {
        validate_ffi_string_py(sink_path)?;
        let engine = Arc::clone(&self.inner);
        let path = sink_path.to_string();
        call_engine(py, move || engine.enable_telemetry(&path))
    }

    /// 0.8.8 Slice 15 — the most-recent captured `query_id` (for `record_feedback`),
    /// or `None` when telemetry is off / no query captured yet.
    fn last_telemetry_query_id(&self) -> Option<String> {
        self.inner.last_telemetry_query_id()
    }

    /// 0.8.8 Slice 15 — attach agent relevance labels for a captured `query_id`.
    /// Ids are the positional `write_cursor` keys emitted in the telemetry
    /// `result_ids` array (the pre-0.8.19 `SearchHit.id` space), NOT the post-C-2
    /// typed `SearchHit.id`. Errors if telemetry is off.
    fn record_feedback(
        &self,
        py: Python<'_>,
        query_id: &str,
        relevant_ids: Vec<u64>,
        irrelevant_ids: Vec<u64>,
        label_source: &str,
    ) -> PyResult<()> {
        validate_ffi_string_py(query_id)?;
        validate_ffi_string_py(label_source)?;
        let engine = Arc::clone(&self.inner);
        let qid = query_id.to_string();
        let ls = label_source.to_string();
        call_engine(py, move || engine.record_feedback(&qid, &relevant_ids, &irrelevant_ids, &ls))
    }

    /// G11 (Slice 15) — BYO-LLM ingest. `cmd` is the argv to spawn
    /// (first element = program, rest = args). `documents` is a list of
    /// dicts with `source_doc_id` and `body` keys.
    fn ingest_with_extractor(
        &self,
        py: Python<'_>,
        cmd: Bound<'_, PyList>,
        documents: Bound<'_, PyList>,
    ) -> PyResult<PyIngestWithExtractorReceipt> {
        // Translate cmd list to Vec<String>.
        let cmd_strings: Vec<String> = cmd
            .iter()
            .map(|item| {
                item.extract::<String>()
                    .map_err(|_| WriteValidationError::new_err("cmd elements must be strings"))
            })
            .collect::<PyResult<_>>()?;

        // Translate documents list of dicts to Vec<ExtractDocument>.
        let docs: Vec<RustExtractDocument> = documents
            .iter()
            .map(|item| {
                let dict = item
                    .cast::<PyDict>()
                    .map_err(|_| WriteValidationError::new_err("document must be a dict"))?;
                let source_doc_id = dict_str_required(dict, "source_doc_id")?;
                let body = dict_str_required(dict, "body")?;
                Ok(RustExtractDocument { source_doc_id, body })
            })
            .collect::<PyResult<_>>()?;

        let cmd_refs: Vec<&str> = cmd_strings.iter().map(|s| s.as_str()).collect();
        let engine = Arc::clone(&self.inner);
        let receipt = call_engine(py, move || engine.ingest_with_extractor(&cmd_refs, &docs))?;
        Ok(PyIngestWithExtractorReceipt::from_rust(receipt))
    }

    /// 0.8.12 Slice 15 (OPP-2) — BYO-LLM consolidation. `cmd` is the argv to
    /// spawn a caller-supplied harness speaking `fathomdb.consolidate.v1` (the
    /// SAME transport as extraction). `axes` is a list of dicts with
    /// `subject_logical_id` and `relation` keys. FathomDB assembles the competing
    /// fact-edge cluster for each axis deterministically and applies the harness
    /// verdicts as supersession/recency metadata (bodies never rewritten).
    fn consolidate_with_provider(
        &self,
        py: Python<'_>,
        cmd: Bound<'_, PyList>,
        axes: Bound<'_, PyList>,
    ) -> PyResult<PyConsolidateReceipt> {
        let cmd_strings: Vec<String> = cmd
            .iter()
            .map(|item| {
                item.extract::<String>()
                    .map_err(|_| WriteValidationError::new_err("cmd elements must be strings"))
            })
            .collect::<PyResult<_>>()?;

        let rust_axes: Vec<RustConsolidateAxis> = axes
            .iter()
            .map(|item| {
                let dict = item
                    .cast::<PyDict>()
                    .map_err(|_| WriteValidationError::new_err("axis must be a dict"))?;
                let subject_logical_id = dict_str_required(dict, "subject_logical_id")?;
                let relation = dict_str_required(dict, "relation")?;
                Ok(RustConsolidateAxis { subject_logical_id, relation })
            })
            .collect::<PyResult<_>>()?;

        let cmd_refs: Vec<&str> = cmd_strings.iter().map(|s| s.as_str()).collect();
        let engine = Arc::clone(&self.inner);
        let receipt =
            call_engine(py, move || engine.consolidate_with_provider(&cmd_refs, &rust_axes))?;
        Ok(PyConsolidateReceipt::from_rust(receipt))
    }

    fn counters(&self) -> PyCounterSnapshot {
        let snap = self.inner.counters();
        PyCounterSnapshot {
            queries: snap.queries,
            writes: snap.writes,
            write_rows: snap.write_rows,
            admin_ops: snap.admin_ops,
            cache_hit: snap.cache_hit,
            cache_miss: snap.cache_miss,
        }
    }

    fn set_profiling(&self, enabled: bool) -> PyResult<()> {
        self.inner.set_profiling(enabled).map_err(engine_error_to_py)
    }

    fn set_slow_threshold_ms(&self, value: u64) -> PyResult<()> {
        self.inner.set_slow_threshold_ms(value).map_err(engine_error_to_py)
    }

    /// Embed arbitrary text with the engine's pinned default embedder
    /// (`fathomdb-bge-small-en-v1.5`), returning the raw vector as a list of
    /// floats. Raises `EmbedderNotConfiguredError` if the engine was opened
    /// without an embedder (`use_default_embedder=False`).
    fn embed(&self, py: Python<'_>, text: &str) -> PyResult<Vec<f32>> {
        validate_ffi_string_py(text)?;
        let engine = Arc::clone(&self.inner);
        let text = text.to_string();
        call_engine(py, move || engine.embed_text(&text))
    }

    // EU-6 — test-hooks-gated vector write seam. Lets Python tests
    // exercise the 0.5/§7 mean-vec pin transition end-to-end through the
    // binding (the public Python surface does not yet expose typed
    // vector writes; that is its own multi-slice campaign). Compiled out
    // of release wheels by the `test-hooks` cfg.
    #[cfg(any(test, feature = "test-hooks"))]
    fn _configure_vector_kind_for_test(&self, py: Python<'_>, kind: &str) -> PyResult<()> {
        validate_ffi_string_py(kind)?;
        let engine = Arc::clone(&self.inner);
        let kind = kind.to_string();
        call_engine(py, move || engine.configure_vector_kind_for_test(&kind))
    }

    #[cfg(any(test, feature = "test-hooks"))]
    fn _write_vector_for_test(&self, py: Python<'_>, kind: &str, text: &str) -> PyResult<()> {
        validate_ffi_string_py(kind)?;
        validate_ffi_string_py(text)?;
        let engine = Arc::clone(&self.inner);
        let kind = kind.to_string();
        let text = text.to_string();
        let _ = call_engine(py, move || engine.write_vector_for_test(&kind, &text))?;
        Ok(())
    }

    #[cfg(feature = "test-hooks")]
    fn _set_legacy_projection_search_subobjects_for_test(
        &self,
        py: Python<'_>,
        name: &str,
    ) -> PyResult<()> {
        validate_ffi_string_py(name)?;
        let engine = Arc::clone(&self.inner);
        let name = name.to_string();
        call_engine(py, move || engine.set_legacy_projection_search_subobjects_for_test(&name))
    }

    fn attach_logging_subscriber(&self, py: Python<'_>, logger: Bound<'_, PyAny>) -> PyResult<()> {
        self.logging.attach(py, &self.inner, &logger)
    }
}
