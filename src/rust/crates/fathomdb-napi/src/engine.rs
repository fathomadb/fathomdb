use super::*;

// ===== Engine =========================================================

#[napi]
pub struct Engine {
    pub(crate) inner: Arc<RustEngine>,
    pub(crate) open_report: Arc<RustOpenReport>,
    pub(crate) subscriber: Mutex<subscriber::SubscriberState>,
}

#[napi]
impl Engine {
    /// Promise-returning open per `dev/interfaces/typescript.md`. The
    /// blocking SQLite work runs on a tokio blocking thread.
    #[napi(factory)]
    pub async fn open(path: String, options: Option<EngineOpenOptions>) -> Result<Engine> {
        validate_ffi_string_napi(&path)?;
        let (config, use_default_embedder) = match options {
            Some(options) => (options.engine_config, options.use_default_embedder.unwrap_or(false)),
            None => (None, false),
        };
        let config = config.map(EngineConfig::into_rust).transpose()?.unwrap_or_default();
        // EU-6: `useDefaultEmbedder: true` → EmbedderChoice::Default
        // (engine materialises the pinned bge-small embedder via the
        // EU-3 loader); `false`/unset → EmbedderChoice::None (engine
        // opens; vector writes fail EmbedderNotConfigured). Caller-
        // supplied custom embedders are deferred per
        // ADR-0.6.0-embedder-protocol Invariant 3.
        let join_result = tokio::task::spawn_blocking(move || {
            catch_unwind(AssertUnwindSafe(|| {
                let choice = if use_default_embedder {
                    EmbedderChoice::Default
                } else {
                    EmbedderChoice::None
                };
                RustEngine::open_with_choice_and_config(path, choice, config)
            }))
        })
        .await;
        let opened = match join_result {
            Ok(Ok(Ok(opened))) => opened,
            Ok(Ok(Err(err))) => return Err(engine_open_error_to_napi(err)),
            Ok(Err(_panic)) => return Err(panic_error()),
            Err(join_err) => {
                return Err(typed_error(
                    CODE_PANIC,
                    format!("spawn_blocking join error: {join_err}"),
                    JsonValue::Null,
                ))
            }
        };
        Ok(Engine {
            inner: Arc::new(opened.engine),
            open_report: Arc::new(opened.report),
            subscriber: Mutex::new(subscriber::SubscriberState::default()),
        })
    }

    /// Structured open report captured at [`Engine::open`] time. Sync
    /// accessor (no Promise — the data lives on the engine struct
    /// after open). Idempotent: each call returns a fresh copy of the
    /// same snapshot.
    #[napi]
    pub fn open_report(&self) -> Result<OpenReport> {
        let report = Arc::clone(&self.open_report);
        call_engine_sync(move || Ok(OpenReport::from_rust(&report)))
    }

    #[napi]
    pub async fn write(&self, batch: Vec<JsonValue>) -> Result<WriteReceipt> {
        let prepared = translate_batch(batch)?;
        let engine = Arc::clone(&self.inner);
        let receipt = call_engine(move || engine.write(&prepared)).await?;
        Ok(WriteReceipt::from_rust(receipt))
    }

    #[napi]
    pub async fn actuate(&self, request: JsonValue) -> Result<ActuationReceiptV1> {
        let request = translate_actuation_request(&request)?;
        let engine = Arc::clone(&self.inner);
        call_engine(move || engine.actuate(request)).await?.try_into()
    }

    #[napi]
    pub async fn register_source_dependency(
        &self,
        request: JsonValue,
    ) -> Result<SourceDependencyV1> {
        let request = translate_dependency_registration(&request)?;
        let engine = Arc::clone(&self.inner);
        call_engine(move || engine.register_source_dependency(request)).await.map(Into::into)
    }

    #[napi]
    pub async fn dependencies_for_source(&self, request: JsonValue) -> Result<DependencyListV1> {
        let request = translate_dependency_source_lookup(&request)?;
        let engine = Arc::clone(&self.inner);
        call_engine(move || engine.dependencies_for_source(request)).await.map(Into::into)
    }

    #[napi]
    pub async fn dependency_for_derived(
        &self,
        request: JsonValue,
    ) -> Result<Option<SourceDependencyV1>> {
        let request = translate_dependency_derived_lookup(&request)?;
        let engine = Arc::clone(&self.inner);
        call_engine(move || engine.dependency_for_derived(request))
            .await
            .map(|value| value.map(Into::into))
    }

    /// Return the current status for one opaque dependency-closure operation.
    #[napi]
    pub async fn read_dependency_closure(&self, request: JsonValue) -> Result<Option<JsonValue>> {
        let request = translate_closure_lookup(&request)?;
        let engine = Arc::clone(&self.inner);
        call_engine(move || engine.read_dependency_closure(request))
            .await
            .map(|value| value.map(closure_status_json))
    }

    /// OPP-12 Phase-1 (0.8.19 Slice 10) — `transition` lifecycle verb. Thin
    /// pass-through: enforces the legal-transition table + `reason`
    /// clear-on-admit/set-on-exclude semantics (design §2/§3). Keys on the bare
    /// `logicalId` (`l:` only); a non-`l:` id → `NotLifecycleAddressableError`; an
    /// illegal move → `IllegalTransitionError { fromState, toState, legal }`.
    #[napi]
    pub async fn transition(
        &self,
        logical_id: String,
        to_state: String,
        reason: Option<String>,
    ) -> Result<()> {
        validate_ffi_string_napi(&logical_id)?;
        if let Some(r) = reason.as_deref() {
            validate_ffi_string_napi(r)?;
        }
        // Full LifecycleState vocabulary accepted so illegal targets surface a
        // typed IllegalTransitionError from the engine; only an unknown string is
        // rejected at the boundary.
        let to_state = RustLifecycleState::from_str_opt(&to_state).ok_or_else(|| {
            typed_error(
                CODE_INVALID_ARGUMENT,
                format!(
                    "unknown lifecycle state {to_state:?}: expected one of pending/active/deleted/purged"
                ),
                JsonValue::Null,
            )
        })?;
        let engine = Arc::clone(&self.inner);
        call_engine(move || engine.transition(&logical_id, to_state, reason)).await
    }

    /// OPP-12 Phase-1 (0.8.19 Slice 10) — `purge` lifecycle verb. Thin
    /// pass-through: deleted-first, idempotent hard-erase across every row-owned
    /// target (design §3). Keys on the bare `logicalId` (`l:` only); a non-`l:` id
    /// → `NotLifecycleAddressableError`; a non-`deleted` node →
    /// `IllegalTransitionError`.
    #[napi]
    pub async fn purge(&self, logical_id: String) -> Result<()> {
        validate_ffi_string_napi(&logical_id)?;
        let engine = Arc::clone(&self.inner);
        call_engine(move || engine.purge(&logical_id)).await
    }

    /// 0.8.20 Slice 5d (R-20-E4, design §4 item 9b) — `eraseSource` lifecycle
    /// verb. Deletes every canonical row carrying `sourceId`, plus its
    /// row-owned projections, and finishes the erasure at rest.
    ///
    /// The COMPANION to `purge`, not a duplicate: `purge` addresses a governed
    /// node by `logicalId`; `eraseSource` addresses ANONYMOUS content (rows
    /// with no `logicalId`) by its provenance, which `purge` cannot reach.
    /// Together they make every canonical row erasable from the SDK alone,
    /// with no CLI on `PATH`.
    ///
    /// Idempotent (an absent source is a zero-count success). Throws
    /// `WriteValidationError` for an empty, whitespace-only or reserved
    /// (`_`-prefixed) `sourceId`. NOT a recovery-denylist name — AC-041 holds.
    #[napi]
    pub async fn erase_source(&self, source_id: String) -> Result<EraseReport> {
        validate_ffi_string_napi(&source_id)?;
        let engine = Arc::clone(&self.inner);
        let report = call_engine(move || engine.erase_source(&source_id)).await?;
        Ok(EraseReport::from_rust(report))
    }

    /// 0.8.20 Slice 15d (R-20-PR / C-1) — the `configureProjections` governed
    /// verb. Declarative + idempotent: the engine diffs `specs` against the
    /// durable registry and backfills the difference. `drop` is EXPLICIT
    /// (omission never drops); a destructive change to a live projection without
    /// a drop throws a `FDB_PROJECTION_DESTRUCTIVE` error carrying `{name, delta}`.
    #[napi]
    pub async fn configure_projections(
        &self,
        specs: Vec<ProjectionSpec>,
        drop: Option<Vec<String>>,
    ) -> Result<ProjectionDelta> {
        let rust_specs: Vec<RustProjectionSpec> =
            specs.iter().map(ProjectionSpec::to_rust).collect::<Result<_>>()?;
        let drop = drop.unwrap_or_default();
        // AC-068a/b — the `drop` list is a caller-supplied FFI-string vector too;
        // validate each entry before the engine call, like the spec strings.
        for name in &drop {
            validate_ffi_string_napi(name)?;
        }
        let engine = Arc::clone(&self.inner);
        let delta = call_engine(move || engine.configure_projections(&rust_specs, &drop)).await?;
        Ok(ProjectionDelta::from_rust(&delta))
    }

    /// 0.8.20 Slice 15d (R-20-PR) — `read.projections` introspection. Returns
    /// every declared `ProjectionSpec` (sorted by name).
    #[napi]
    pub async fn read_projections(&self) -> Result<Vec<ProjectionSpec>> {
        let engine = Arc::clone(&self.inner);
        let specs = call_engine(move || engine.read_projections()).await?;
        Ok(specs.iter().map(ProjectionSpec::from_rust).collect())
    }

    /// Read current projection-runtime facts without changing configuration or
    /// scheduling work. This pure query may take the ordinarily opened engine
    /// connection lock; it is not a `ReaderWorkerPool` request and does not open
    /// a separately read-only SQLite connection. This is the native peer of
    /// `read.projectionStatus`.
    #[napi]
    pub async fn read_projection_status(&self) -> Result<ProjectionRuntimeStatus> {
        let engine = Arc::clone(&self.inner);
        let status = call_engine(move || engine.read_projection_status()).await?;
        Ok(ProjectionRuntimeStatus::from_rust(&status))
    }

    #[napi]
    pub async fn read_projection_generation_status(&self) -> Result<ProjectionGenerationStatusV1> {
        let engine = Arc::clone(&self.inner);
        call_engine(move || engine.read_projection_generation_status()).await.map(Into::into)
    }

    #[napi]
    pub async fn read_mutation_projection_status(
        &self,
        request: JsonValue,
    ) -> Result<MutationProjectionStatusV1> {
        let request = translate_mutation_projection_status_request(&request)?;
        let engine = Arc::clone(&self.inner);
        call_engine(move || engine.read_mutation_projection_status(request)).await.map(Into::into)
    }

    #[napi]
    pub async fn read_embedding_readiness(&self) -> Result<EmbeddingReadiness> {
        let engine = Arc::clone(&self.inner);
        let readiness = call_engine(move || engine.read_embedding_readiness()).await?;
        Ok(EmbeddingReadiness::from_rust(&readiness))
    }

    /// Mint a restart-stable authenticated read context.
    #[napi]
    pub async fn freeze_read_context(&self, context: ReadContextV1) -> Result<FrozenReadContextV1> {
        let context = read_context_to_rust(context)?;
        let engine = Arc::clone(&self.inner);
        let frozen = call_engine(move || engine.freeze_read_context(&context)).await?;
        Ok(frozen_context_from_rust(frozen))
    }

    /// Validate a frozen context before JavaScript converts dynamic controls.
    #[napi]
    pub async fn validate_frozen_read_context(&self, context: FrozenReadContextV1) -> Result<()> {
        let context = frozen_context_to_rust(context)?;
        let engine = Arc::clone(&self.inner);
        call_engine(move || engine.validate_frozen_read_context_for_binding(&context)).await
    }

    /// Execute one canonical JSON graph-expansion request.
    #[napi]
    pub async fn graph_expand(&self, request_json: String) -> Result<String> {
        validate_ffi_string_napi(&request_json)?;
        let request = decode_graph_expand_request_v1(request_json.as_bytes())
            .map_err(|error| graph_expansion_error_to_napi(&error))?;
        let engine = Arc::clone(&self.inner);
        let result = call_engine(move || engine.graph_expand(&request)).await?;
        let encoded = encode_graph_expand_result_v1(&result)
            .map_err(|error| graph_expansion_error_to_napi(&error))?;
        String::from_utf8(encoded).map_err(|_| {
            typed_error(
                CODE_GRAPH_EXPANSION,
                "graph_corrupt at ",
                json!({ "reason": "graph_corrupt", "fieldPath": "" }),
            )
        })
    }

    /// Return canonical version-1 JSON for one governed dependency trace.
    #[napi]
    pub async fn trace_dependency(
        &self,
        root_revision_id: String,
        direction: String,
        context: FrozenReadContextV1,
        max_relations: Option<u32>,
        max_work_units: Option<u32>,
    ) -> Result<String> {
        validate_ffi_string_napi(&root_revision_id)?;
        let direction = match direction.as_str() {
            "to_source" => RustDependencyTraceDirectionV1::ToSource,
            "to_dependents" => RustDependencyTraceDirectionV1::ToDependents,
            _ => {
                return Err(typed_error(
                    CODE_DEPENDENCY_TRACE,
                    "trace_direction_invalid at /direction",
                    json!({ "reason": "trace_direction_invalid", "fieldPath": "/direction" }),
                ));
            }
        };
        let request = RustDependencyTraceRequestV1::new(
            root_revision_id,
            direction,
            frozen_context_to_rust(context)?,
        )
        .and_then(|request| {
            request.with_bounds(max_relations.unwrap_or(100), max_work_units.unwrap_or(101))
        })
        .map_err(|error| engine_error_to_napi(error.into()))?;
        let engine = Arc::clone(&self.inner);
        let result = call_engine(move || engine.trace_dependency(request)).await?;
        let bytes = encode_dependency_trace_result_v1(&result)
            .map_err(|error| engine_error_to_napi(error.into()))?;
        String::from_utf8(bytes)
            .map_err(|_| typed_error(CODE_DEPENDENCY_TRACE, "trace codec failure", JsonValue::Null))
    }

    /// Search under a frozen validity and eligibility context.
    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub async fn search_frozen(
        &self,
        query: String,
        context: FrozenReadContextV1,
        rerank_depth: Option<i64>,
        use_graph_arm: Option<bool>,
        alpha: Option<f64>,
        pool_n: Option<i64>,
        explain: Option<bool>,
        limit: Option<i64>,
    ) -> Result<SearchResult> {
        let context = frozen_context_to_rust(context)?;
        let depth = usize::try_from(rerank_depth.unwrap_or(0)).unwrap_or(usize::MAX);
        let graph_arm = use_graph_arm.unwrap_or(false);
        let alpha = alpha.unwrap_or(0.3);
        let pool_n =
            pool_n.map(|value| usize::try_from(value).unwrap_or(usize::MAX)).unwrap_or(depth);
        let explain = explain.unwrap_or(false);
        let limit = usize::try_from(limit.unwrap_or(10)).unwrap_or(usize::MAX);
        let engine = Arc::clone(&self.inner);
        let result = call_engine(move || {
            engine.search_frozen(&query, &context, depth, graph_arm, alpha, pool_n, explain, limit)
        })
        .await?;
        Ok(SearchResult::from_rust(result))
    }

    /// Search under a frozen context and attach one evidence reference per hit.
    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub async fn search_with_evidence(
        &self,
        query: String,
        context: FrozenReadContextV1,
        rerank_depth: Option<i64>,
        use_graph_arm: Option<bool>,
        alpha: Option<f64>,
        pool_n: Option<i64>,
        include_explanation: Option<bool>,
        limit: Option<i64>,
    ) -> Result<EvidenceSearchResultV1> {
        let rerank_depth = u32::try_from(rerank_depth.unwrap_or(0)).map_err(|_| {
            engine_error_to_napi(RustEngineError::InvalidArgument {
                msg: "rerank_depth must be in 0..=4294967295".to_string(),
            })
        })?;
        let pool_n = u32::try_from(pool_n.unwrap_or(0)).map_err(|_| {
            engine_error_to_napi(RustEngineError::InvalidArgument {
                msg: "pool_n must be in 0..=4294967295".to_string(),
            })
        })?;
        let limit = u32::try_from(limit.unwrap_or(10)).map_err(|_| {
            engine_error_to_napi(RustEngineError::InvalidArgument {
                msg: "limit must be in 0..=4294967295".to_string(),
            })
        })?;
        let request = RustEvidenceSearchRequestV1 {
            schema_version: 1,
            query,
            context: frozen_context_to_rust(context)?,
            rerank_depth,
            use_graph_arm: use_graph_arm.unwrap_or(false),
            alpha: alpha.unwrap_or(0.3),
            pool_n,
            include_explanation: include_explanation.unwrap_or(false),
            limit,
        };
        let engine = Arc::clone(&self.inner);
        call_engine(move || engine.search_with_evidence(&request)).await.map(Into::into)
    }

    /// Resolve exact source bytes under an equivalent frozen context.
    #[napi]
    pub async fn resolve_evidence(
        &self,
        evidence_ref: String,
        context: FrozenReadContextV1,
    ) -> Result<ResolvedEvidenceV1> {
        let request = RustEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: RustEvidenceRefV1::new(evidence_ref)
                .map_err(|error| engine_error_to_napi(RustEngineError::Evidence(error)))?,
            context: frozen_context_to_rust(context)?,
        };
        let engine = Arc::clone(&self.inner);
        call_engine(move || engine.resolve_evidence(&request)).await.map(Into::into)
    }

    /// Resolve one exact artifact disclosed by frozen graph expansion.
    #[napi]
    pub async fn resolve_graph_evidence(
        &self,
        evidence_ref: String,
        context: FrozenReadContextV1,
    ) -> Result<String> {
        let request = RustGraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: RustGraphEvidenceRefV1::new(evidence_ref)
                .map_err(|error| engine_error_to_napi(RustEngineError::Evidence(error)))?,
            context: frozen_context_to_rust(context)?,
        };
        let engine = Arc::clone(&self.inner);
        let value = call_engine(move || engine.resolve_graph_evidence(&request)).await?;
        let bytes = encode_resolved_graph_evidence_v1(&value)
            .map_err(|error| engine_error_to_napi(RustEngineError::Evidence(error)))?;
        String::from_utf8(bytes)
            .map_err(|_| typed_error(CODE_EVIDENCE, "evidence_corrupt at ", JsonValue::Null))
    }

    /// Search and expand on one frozen reader transaction.
    #[napi]
    pub async fn search_expand_frozen(
        &self,
        query: String,
        context: FrozenReadContextV1,
        depth: i64,
        limit: Option<i64>,
    ) -> Result<SearchExpandResult> {
        let context = frozen_context_to_rust(context)?;
        let depth = u32::try_from(depth).unwrap_or(u32::MAX);
        let limit = usize::try_from(limit.unwrap_or(10)).unwrap_or(usize::MAX);
        let engine = Arc::clone(&self.inner);
        let result =
            call_engine(move || engine.search_expand_frozen(&query, &context, depth, limit))
                .await?;
        Ok(SearchExpandResult::from_rust(result))
    }

    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub async fn search(
        &self,
        query: String,
        filter: Option<SearchFilterInput>,
        rerank_depth: Option<u32>,
        // 0.8.1 R3 (Slice 30) — when true, seed a BFS over temporal fact-edges
        // from the top-10 fused hits and fuse the reachable nodes as a third RRF arm.
        // Default false → byte-identical to the pre-Slice-30 two-arm pipeline.
        use_graph_arm: Option<bool>,
        // 0.8.5 (EXP-0) — CE-rerank knobs. `alpha` (default 0.3, clamped to [0,1] in
        // the engine) is the CE-blend weight; `poolN` (default = rerankDepth) is the
        // reranked-pool size. Omitting both reproduces the byte-identical default order.
        alpha: Option<f64>,
        pool_n: Option<u32>,
        // 0.8.8 EXP-OBS (Slice 10) — when true, populate `SearchResult.explanation`
        // with per-hit provenance + score breakdown + query trace. Default false
        // returns `explanation=null` and a byte-identical result (R-OBS-2 zero-cost).
        explain: Option<bool>,
        // 0.8.20 Slice 15b fix-2 (R-20-NV / R-20-RV) — optional validity view,
        // the same trailing options object the five read verbs take. Omitted /
        // `undefined` is the strict view: active-only, non-superseded, and valid
        // AT QUERY TIME. `{ includeOutOfWindow: true }` returns hits whatever
        // their window; `{ validAsOf: t }` evaluates validity at the bound
        // instant `t`. The existence flags are REFUSED here (typed
        // `FDB_INVALID_ARGUMENT`), never silently ignored.
        view: Option<ReadViewInput>,
        limit: Option<u32>,
    ) -> Result<SearchResult> {
        validate_ffi_string_napi(&query)?;
        if query.trim().is_empty() {
            return Err(typed_error(
                CODE_WRITE_VALIDATION,
                "query must not be empty",
                JsonValue::Null,
            ));
        }
        // G10 filter strings cross the FFI exactly like `query` and the write
        // fields, so they go through the same Rust-side guard BEFORE the engine
        // is touched (defense-in-depth for embedded NUL, as on the write path).
        // `created_after` is numeric — no string validation. Lone UTF-16
        // surrogates are napi-rs-lossy here (replaced with U+FFFD before Rust
        // sees them), so — like write/configure — the TS `search` wrapper guards
        // those JS-side (see src/validation.ts). Validating here leaves the
        // all-`None` collapse below (the byte-identical unfiltered path) intact.
        let filter = search_filter_input_to_rust(filter)?;
        // 0.8.1 R1: rerank_depth=None or 0 → soft-fallback (identity).
        let depth = rerank_depth.unwrap_or(0) as usize;
        // 0.8.1 R3: use_graph_arm=None or false → two-arm byte-identical path.
        let graph_arm = use_graph_arm.unwrap_or(false);
        // 0.8.5 (D4): resolve the binding defaults — α=0.3, pool_n=rerankDepth — so an
        // unset call reproduces the pre-slice ranking. α is clamped in the engine.
        let alpha = alpha.unwrap_or(0.3);
        let pool_n = pool_n.map(|p| p as usize).unwrap_or(depth);
        // 0.8.8 EXP-OBS: explain=true routes to search_explained (same retrieval +
        // the sidecar); default stays on search_reranked (byte-identical).
        let explain = explain.unwrap_or(false);
        let view = read_view_or_default(view);
        let limit = limit.unwrap_or(10) as usize;
        let engine = Arc::clone(&self.inner);
        // fix-2: ONE call — `explain` is a parameter of the full-arity view entry
        // point, so the two arms can no longer drift on `view`.
        let result = call_engine(move || {
            engine.search_reranked_view_with_limit(
                &query, filter, depth, graph_arm, alpha, pool_n, explain, &view, limit,
            )
        })
        .await?;
        Ok(SearchResult::from_rust(result))
    }

    /// Lexically search one declared property-FTS projection only.
    #[napi]
    pub async fn search_projected_text(
        &self,
        query: String,
        name: String,
        filter: Option<SearchFilterInput>,
        view: Option<ReadViewInput>,
        limit: Option<u32>,
    ) -> Result<SearchResult> {
        validate_ffi_string_napi(&query)?;
        validate_ffi_string_napi(&name)?;
        let filter = search_filter_input_to_rust(filter)?;
        let view = read_view_or_default(view);
        let limit = limit.unwrap_or(10) as usize;
        let engine = Arc::clone(&self.inner);
        let result = call_engine(move || {
            engine.search_projected_text_with_limit(&query, &name, filter, &view, limit)
        })
        .await?;
        Ok(SearchResult::from_rust(result))
    }

    /// 0.8.18 Slice 5 (#5 vector-equivalence probe, R-VEQ-4) — the explicit
    /// text-only / FTS-only search path. Does NOT embed the query and NEVER raises
    /// `FDB_VECTOR_EQUIVALENCE_MISMATCH`, so it stays serviceable when the engine
    /// opened in the degraded `denseDisabled` state. Matching node- and edge-body
    /// FTS candidates are deterministically body-deduplicated and ranked before
    /// `limit`; no vector recall, CE rerank, or graph arm runs on this path.
    #[napi]
    ///
    /// 0.8.20 Slice 15b fix-2 — takes the same optional `view` as `search`.
    pub async fn search_text_only(
        &self,
        query: String,
        view: Option<ReadViewInput>,
        limit: Option<u32>,
    ) -> Result<SearchResult> {
        validate_ffi_string_napi(&query)?;
        if query.trim().is_empty() {
            return Err(typed_error(
                CODE_WRITE_VALIDATION,
                "query must not be empty",
                JsonValue::Null,
            ));
        }
        let view = read_view_or_default(view);
        let limit = limit.unwrap_or(10) as usize;
        let engine = Arc::clone(&self.inner);
        let result =
            call_engine(move || engine.search_text_only_view_with_limit(&query, &view, limit))
                .await?;
        Ok(SearchResult::from_rust(result))
    }

    /// 0.8.18 Slice 5 (R-VEQ-6) — `true` iff the engine opened degraded (the #5
    /// self-check found a vector-equivalence divergence and every dense arm is
    /// refusing). Mirrors `OpenReport.denseDisabled`.
    #[napi]
    pub fn dense_disabled(&self) -> bool {
        self.inner.dense_disabled()
    }

    /// 0.8.18 Slice 5 (R-VEQ-6) — the human-readable reason for the degraded state,
    /// or `null` when dense is healthy.
    #[napi]
    pub fn dense_disabled_reason(&self) -> Option<String> {
        self.inner.dense_disabled_reason()
    }

    /// 0.8.18 Slice 5 (R-VEQ-6) — telemetry counter: query-time dense-arm refusals
    /// raised because the engine opened degraded.
    #[napi]
    pub fn vector_equivalence_refusal_count(&self) -> i64 {
        self.inner.vector_equivalence_refusal_count() as i64
    }

    #[napi(ts_return_type = "Promise<void>")]
    pub fn close(&self, env: Env) -> Result<JsObject> {
        let (deferred, promise) = env.create_deferred::<(), _>()?;
        self.subscriber.lock().unwrap_or_else(|poison| poison.into_inner()).close();
        let engine = Arc::clone(&self.inner);
        napi::bindgen_prelude::spawn(async move {
            match call_engine(move || engine.close()).await {
                Ok(()) => deferred.resolve(|_| Ok(())),
                Err(error) => deferred.reject(error),
            }
        });
        Ok(promise)
    }

    #[napi]
    pub async fn drain(&self, timeout_ms: u32) -> Result<()> {
        let engine = Arc::clone(&self.inner);
        let ms = timeout_ms as u64;
        call_engine(move || engine.drain(ms)).await
    }

    /// 0.8.8 Slice 15 (OPP-9) — enable opt-in local telemetry capture to a JSONL
    /// `sinkPath`. Off by default; local file only (no egress).
    #[napi]
    pub async fn enable_telemetry(&self, sink_path: String) -> Result<()> {
        validate_ffi_string_napi(&sink_path)?;
        let engine = Arc::clone(&self.inner);
        call_engine(move || engine.enable_telemetry(&sink_path)).await
    }

    /// 0.8.8 Slice 15 — the most-recent captured `queryId` (for `recordFeedback`),
    /// or `null` when telemetry is off / no query captured yet.
    #[napi]
    pub fn last_telemetry_query_id(&self) -> Option<String> {
        self.inner.last_telemetry_query_id()
    }

    /// 0.8.8 Slice 15 — attach agent relevance labels for a captured `queryId`.
    /// Ids are the positional `write_cursor` keys emitted in the telemetry
    /// `result_ids` array (the pre-0.8.19 `SearchHit.id` space), NOT the post-C-2
    /// typed `SearchHit.id`. Errors if telemetry is off.
    #[napi]
    pub async fn record_feedback(
        &self,
        query_id: String,
        relevant_ids: Vec<i64>,
        irrelevant_ids: Vec<i64>,
        label_source: String,
    ) -> Result<()> {
        validate_ffi_string_napi(&query_id)?;
        validate_ffi_string_napi(&label_source)?;
        // codex §9 [P2] (parity): ids are non-negative `u64` (the telemetry
        // `result_ids` / `write_cursor` key space). A direct napi caller bypassing
        // the TS wrapper could pass a negative `i64` which `as u64` would wrap to a
        // huge value; reject it here to match the TS/Python wrapper guards.
        let rel = checked_ids_napi("relevantIds", &relevant_ids)?;
        let irr = checked_ids_napi("irrelevantIds", &irrelevant_ids)?;
        let engine = Arc::clone(&self.inner);
        call_engine(move || engine.record_feedback(&query_id, &rel, &irr, &label_source)).await
    }

    /// G11 (Slice 15) — BYO-LLM ingest. `cmd` is the argv to spawn (first
    /// element = program, rest = args). `documents` is an array of objects
    /// with `sourceDocId` and `body` string properties.
    #[napi]
    pub async fn ingest_with_extractor(
        &self,
        cmd: Vec<String>,
        documents: Vec<JsonValue>,
    ) -> Result<IngestWithExtractorReceipt> {
        let docs: Vec<RustExtractDocument> = documents
            .iter()
            .map(|item| {
                let source_doc_id = item
                    .get("sourceDocId")
                    .or_else(|| item.get("source_doc_id"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        typed_error(
                            CODE_WRITE_VALIDATION,
                            "document must have sourceDocId",
                            JsonValue::Null,
                        )
                    })?
                    .to_string();
                let body = item
                    .get("body")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        typed_error(
                            CODE_WRITE_VALIDATION,
                            "document must have body",
                            JsonValue::Null,
                        )
                    })?
                    .to_string();
                Ok(RustExtractDocument { source_doc_id, body })
            })
            .collect::<Result<_>>()?;

        let engine = Arc::clone(&self.inner);
        let receipt = call_engine(move || {
            let cmd_refs: Vec<&str> = cmd.iter().map(|s| s.as_str()).collect();
            engine.ingest_with_extractor(&cmd_refs, &docs)
        })
        .await?;
        Ok(IngestWithExtractorReceipt::from_rust(receipt))
    }

    /// 0.8.12 Slice 15 (OPP-2) — BYO-LLM consolidation. `cmd` is the argv to
    /// spawn a caller-supplied harness speaking `fathomdb.consolidate.v1` (the
    /// SAME transport as extraction). `axes` is an array of objects with
    /// `subjectLogicalId` and `relation` string properties. FathomDB assembles
    /// each competing fact-edge cluster deterministically and applies the harness
    /// verdicts as supersession/recency metadata (bodies never rewritten).
    #[napi]
    pub async fn consolidate_with_provider(
        &self,
        cmd: Vec<String>,
        axes: Vec<JsonValue>,
    ) -> Result<ConsolidateReceipt> {
        let rust_axes: Vec<RustConsolidateAxis> = axes
            .iter()
            .map(|item| {
                let subject_logical_id = item
                    .get("subjectLogicalId")
                    .or_else(|| item.get("subject_logical_id"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        typed_error(
                            CODE_WRITE_VALIDATION,
                            "axis must have subjectLogicalId",
                            JsonValue::Null,
                        )
                    })?
                    .to_string();
                let relation = item
                    .get("relation")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        typed_error(
                            CODE_WRITE_VALIDATION,
                            "axis must have relation",
                            JsonValue::Null,
                        )
                    })?
                    .to_string();
                Ok(RustConsolidateAxis { subject_logical_id, relation })
            })
            .collect::<Result<_>>()?;

        let engine = Arc::clone(&self.inner);
        let receipt = call_engine(move || {
            let cmd_refs: Vec<&str> = cmd.iter().map(|s| s.as_str()).collect();
            engine.consolidate_with_provider(&cmd_refs, &rust_axes)
        })
        .await?;
        Ok(ConsolidateReceipt::from_rust(receipt))
    }

    /// Embed `text` with the engine's pinned default embedder
    /// (`fathomdb-bge-small-en-v1.5`) and return the raw vector.
    ///
    /// Read-path primitive (mirror of the Python `Engine.embed`) for callers
    /// that need vectors under the engine's own embedder identity (e.g.
    /// coverage-index clustering) rather than a parallel, possibly-divergent
    /// embedder. Rejects with `FDB_EMBEDDER_NOT_CONFIGURED` if the engine was
    /// opened without an embedder (`useDefaultEmbedder: false`).
    #[napi]
    pub async fn embed(&self, text: String) -> Result<Vec<f64>> {
        let engine = Arc::clone(&self.inner);
        let vector = call_engine(move || engine.embed_text(&text)).await?;
        Ok(vector.into_iter().map(|x| x as f64).collect())
    }

    #[napi]
    pub fn counters(&self) -> Result<CounterSnapshot> {
        let engine = Arc::clone(&self.inner);
        call_engine_sync(move || {
            let snap = engine.counters();
            Ok(CounterSnapshot {
                queries: snap.queries as i64,
                writes: snap.writes as i64,
                write_rows: snap.write_rows as i64,
                admin_ops: snap.admin_ops as i64,
                cache_hit: snap.cache_hit as i64,
                cache_miss: snap.cache_miss as i64,
            })
        })
    }

    #[napi]
    pub fn set_profiling(&self, enabled: bool) -> Result<()> {
        let engine = Arc::clone(&self.inner);
        call_engine_sync(move || engine.set_profiling(enabled).map_err(engine_error_to_napi))
    }

    #[napi]
    pub fn set_slow_threshold_ms(&self, value: u32) -> Result<()> {
        let engine = Arc::clone(&self.inner);
        call_engine_sync(move || {
            engine.set_slow_threshold_ms(value as u64).map_err(engine_error_to_napi)
        })
    }

    #[napi(
        ts_args_type = "callback: (event: { kind: 'event' | 'profile' | 'slowStatement' | 'stressFailure'; droppedRecordsTotal: string; [key: string]: unknown }) => void"
    )]
    pub fn attach_subscriber(&self, env: Env, callback: JsFunction) -> Result<()> {
        let attachment = subscriber::Attachment::new(env, callback)?;
        let mut state = self.subscriber.lock().unwrap_or_else(|poison| poison.into_inner());
        if state.closing {
            attachment.detach();
            return Err(typed_error(CODE_CLOSING, "engine is closing", JsonValue::Null));
        }
        let subscription = self.inner.subscribe(attachment.clone());
        state.replace(attachment, subscription);
        Ok(())
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.subscriber.lock().unwrap_or_else(|poison| poison.into_inner()).close();
    }
}
