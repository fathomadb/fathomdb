impl Engine {
    /// Mint an authenticated read context bound to the current database state.
    ///
    /// The returned context is portable across process restarts for this
    /// database. Any visibility-affecting mutation makes later consumption fail
    /// with [`FrozenReadErrorReason::StateDrifted`].
    pub fn freeze_read_context(
        &self,
        context: &ReadContextV1,
    ) -> Result<FrozenReadContextV1, EngineError> {
        let _operation = self.ensure_open()?;
        let scope = self.scope_predicate();
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        let (frozen, generation) = frozen_read::mint(connection, context, &scope)?;
        let previous = self.read_visibility_generation.fetch_max(generation, Ordering::AcqRel);
        if generation < previous {
            return Err(FrozenReadError {
                reason: FrozenReadErrorReason::StateUnavailable,
                field_path: "/token".to_string(),
            }
            .into());
        }
        Ok(frozen)
    }

    /// Binding-only preflight for dynamic-language argument precedence.
    ///
    /// This is not a read consumer: it returns no data and does not retain a
    /// snapshot. Dynamic-language bindings use it before converting ranking
    /// controls so authenticated frozen-context failures retain the Engine's
    /// documented precedence. The consuming operation authenticates and
    /// validates again on its own reader transaction.
    #[doc(hidden)]
    pub fn validate_frozen_read_context_for_binding(
        &self,
        frozen: &FrozenReadContextV1,
    ) -> Result<(), EngineError> {
        let _operation = self.ensure_open()?;
        let scope = self.scope_predicate();
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let binding = frozen_read::authenticate(connection, frozen, &scope)?;
        frozen_read::validate_snapshot(connection, &binding)?;
        Ok(())
    }

    /// Search using an authenticated frozen context.
    ///
    /// Eligibility and validity come exclusively from `frozen`; callers cannot
    /// weaken them while consuming the token. A stale or foreign token is a
    /// typed [`EngineError::FrozenRead`] refusal. When `explain` is true, the
    /// returned explanation carries a finalized non-empty correlation identity.
    #[allow(clippy::too_many_arguments)]
    pub fn search_frozen(
        &self,
        query: &str,
        frozen: &FrozenReadContextV1,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        explain: bool,
        limit: usize,
    ) -> Result<SearchResult, EngineError> {
        self.search_frozen_inner(
            query,
            frozen,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            explain,
            limit,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn search_frozen_inner(
        &self,
        query: &str,
        frozen: &FrozenReadContextV1,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        explain: bool,
        limit: usize,
        snippet: Option<SnippetOptions>,
        graph_options: Option<GraphArmOptions>,
    ) -> Result<SearchResult, EngineError> {
        let _operation = self.ensure_open()?;
        let scope = self.scope_predicate();
        let binding = {
            let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
            let connection = connection.as_ref().ok_or(EngineError::Closing)?;
            frozen_read::authenticate(connection, frozen, &scope)?
        };
        self.search_inner_with_frozen_binding_and_stats(
            query,
            Some(frozen.context.eligibility.clone()),
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            explain,
            frozen.context.view,
            limit,
            Some(binding),
            snippet,
            graph_options,
        )
        .map(|(mut result, _stats, _expanded)| {
            if result.explanation.is_some() {
                self.finalize_search_observability(query, &mut result);
            }
            result
        })
    }

    /// [`search_frozen`][Engine::search_frozen] with optional EP-17
    /// match-centred snippets. Snippet options are presentation: they are not
    /// eligibility and are not part of the frozen context.
    ///
    /// # Errors
    ///
    /// As [`search_frozen`][Engine::search_frozen], plus
    /// [`EngineError::InvalidArgument`] when `snippet` fails
    /// [`SnippetOptions::validate`].
    #[allow(clippy::too_many_arguments)]
    pub fn search_frozen_with_snippets(
        &self,
        query: &str,
        frozen: &FrozenReadContextV1,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        explain: bool,
        limit: usize,
        snippet: Option<&SnippetOptions>,
    ) -> Result<SearchResult, EngineError> {
        validate_snippet(snippet)?;
        self.search_frozen_inner(
            query,
            frozen,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            explain,
            limit,
            snippet.copied(),
            None,
        )
    }

    /// [`search_frozen_with_snippets`][Engine::search_frozen_with_snippets]
    /// with opt-in graph-arm seeding and edge-traversal controls.
    #[allow(clippy::too_many_arguments)]
    pub fn search_frozen_with_graph_options(
        &self,
        query: &str,
        frozen: &FrozenReadContextV1,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        explain: bool,
        limit: usize,
        snippet: Option<&SnippetOptions>,
        graph_options: &GraphArmOptions,
    ) -> Result<SearchResult, EngineError> {
        self.search_frozen_with_hints(
            query,
            frozen,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            explain,
            limit,
            snippet,
            graph_options,
            false,
        )
    }

    /// [`search_frozen_with_graph_options`][Engine::search_frozen_with_graph_options]
    /// with opt-in agent search hints (`hints = true`).
    #[allow(clippy::too_many_arguments)]
    pub fn search_frozen_with_hints(
        &self,
        query: &str,
        frozen: &FrozenReadContextV1,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        explain: bool,
        limit: usize,
        snippet: Option<&SnippetOptions>,
        graph_options: &GraphArmOptions,
        hints: bool,
    ) -> Result<SearchResult, EngineError> {
        validate_snippet(snippet)?;
        let mut result = self.search_frozen_inner(
            query,
            frozen,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            explain,
            limit,
            snippet.copied(),
            Some(graph_options.clone()),
        )?;
        if hints {
            result.hints = Some(self.compute_search_hints(&result, limit));
        }
        Ok(result)
    }

    /// Run an opt-in frozen search and return one authenticated evidence
    /// reference for every result.
    ///
    /// The ordinary [`SearchHit`] and [`SearchResult`] contracts are unchanged.
    /// Evidence creation fails as a whole if any returned artifact lacks complete
    /// source provenance. When `include_explanation` is true, the nested result's
    /// explanation carries a finalized non-empty correlation identity.
    pub fn search_with_evidence(
        &self,
        request: &EvidenceSearchRequestV1,
    ) -> Result<EvidenceSearchResultV1, EngineError> {
        if request.schema_version != 1 {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::UnsupportedSchemaVersion,
                "/schemaVersion",
            )
            .into());
        }
        let _operation = self.ensure_open()?;
        let scope = self.scope_predicate();
        let binding = {
            let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
            let connection = connection.as_ref().ok_or(EngineError::Closing)?;
            frozen_read::authenticate(connection, &request.context, &scope)
                .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?
        };
        let result_limit = request.limit as usize;
        let candidate_limit = self
            .projection_runtime
            .shared
            .search_limit_override
            .load(Ordering::SeqCst)
            .max(result_limit);
        let work = SearchReaderWork::evidence(
            result_limit,
            candidate_limit,
            request.context.context.eligibility.clone(),
            self.projection_runtime.shared.recency_reweight_enabled.load(Ordering::SeqCst),
            self.projection_runtime.shared.importance_reweight_enabled.load(Ordering::SeqCst),
            self.projection_runtime.shared.vector_stage_only_for_test.load(Ordering::SeqCst),
            &request.query,
            request.rerank_depth as usize,
            request.use_graph_arm,
            request.alpha,
            request.pool_n as usize,
            request.include_explanation,
            if self.runtime_embedder.is_none() {
                ProjectionRuntimeStateV1::Absent
            } else {
                ProjectionRuntimeStateV1::Usable
            },
            request.context.context.view,
            binding,
            FrozenQueryRuntime::new(
                Arc::clone(&self.embed_dispatch),
                self.runtime_embedder_identity.clone(),
                Arc::clone(&self.read_visibility_generation),
            ),
        )
        .with_graph_options(Some(request.graph_arm_options()));
        let (response_tx, response_rx) = mpsc::sync_channel::<EvidenceReaderResponse>(1);
        self.reader_pool
            .dispatch(ReaderRequest::search_evidence(
                work,
                request.context.clone(),
                request.include_explanation,
                #[cfg(any(test, feature = "test-hooks"))]
                Arc::clone(&self.evidence_before_sidecar_hook),
                response_tx,
            ))
            .map_err(|_| EngineError::Closing)?;
        let mut result = match response_rx.recv().map_err(|_| EngineError::Storage)? {
            Ok(result) => result,
            Err(SearchReaderError::Evidence(error)) => return Err(error),
            Err(SearchReaderError::InvalidFilter(reason)) => {
                return Err(EngineError::InvalidFilter { reason });
            }
            Err(SearchReaderError::RerankerDevicePolicy(error)) => {
                return Err(EngineError::from(error));
            }
            Err(SearchReaderError::FrozenRead(_)) => {
                return Err(EngineError::Evidence(EvidenceErrorV1::unavailable()))
            }
            Err(SearchReaderError::WriteValidation) => return Err(EngineError::WriteValidation),
            Err(SearchReaderError::InvalidArgument(msg)) => {
                return Err(EngineError::InvalidArgument { msg });
            }
            Err(SearchReaderError::Sqlite(error)) => {
                self.emit_sqlite_internal_error(&error);
                return Err(EngineError::Storage);
            }
        };
        if let Some(explanation) = result.search_result.explanation.as_mut() {
            let identity = &self.runtime_embedder_identity;
            explanation.trace.embedder_id =
                format!("{}@{} (dim={})", identity.name, identity.revision, identity.dimension);
        }
        if result.search_result.explanation.is_some() {
            self.finalize_search_observability(&request.query, &mut result.search_result);
        }
        if request.hints {
            result.search_result.hints =
                Some(self.compute_search_hints(&result.search_result, result_limit));
        }
        Ok(result)
    }

    /// Resolve one evidence reference under a newly supplied equivalent frozen
    /// context.
    ///
    /// A reference is never authority. Malformed, foreign, mismatched, stale,
    /// invisible, superseded, erased, or closure-fenced state returns the same
    /// privacy-preserving `evidence_unavailable` outcome.
    pub fn resolve_evidence(
        &self,
        request: &EvidenceResolveRequestV1,
    ) -> Result<ResolvedEvidenceV1, EngineError> {
        if request.schema_version != 1 {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::UnsupportedSchemaVersion,
                "/schemaVersion",
            )
            .into());
        }
        let _operation = self.ensure_open()?;
        let scope = self.scope_predicate();
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        let tx = connection.transaction().map_err(|_| EngineError::Storage)?;
        let binding = frozen_read::authenticate(&tx, &request.context, &scope)
            .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
        frozen_read::validate_snapshot(&tx, &binding)
            .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
        let result = evidence::resolve(&tx, request, &scope)?;
        #[cfg(any(test, feature = "test-hooks"))]
        self.evidence_before_resolve_return_hook.fire();
        frozen_read::validate_snapshot(&tx, &binding)
            .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
        tx.commit().map_err(|_| EngineError::Storage)?;
        Ok(result)
    }

    /// Resolve a bounded batch (`1..=EVIDENCE_RESOLVE_MANY_MAX`) of evidence
    /// references under a single authenticated frozen read context.
    ///
    /// Precedence:
    /// 1. `schema_version != 1` -> whole-call `UnsupportedSchemaVersion` at `"/schemaVersion"`.
    /// 2. `evidence_refs.is_empty() || evidence_refs.len() > EVIDENCE_RESOLVE_MANY_MAX`
    ///    (raw count before deduplication, before DB access) -> whole-call `InvalidArgument`.
    /// 3. Single connection lock + `BEGIN DEFERRED` read transaction: authenticate
    ///    `request.context` once and validate snapshot (collapsing failures into
    ///    whole-call `EvidenceUnavailable`).
    /// 4. Deduplicate unique tokens while preserving exact positional order and
    ///    arity, returning per-slot `Resolved` or nondisclosing `Refused` items
    ///    with `field_path = "/evidenceRefs/{i}"`.
    /// 5. Re-validate snapshot before returning (collapsing post-resolve drift
    ///    into whole-call `EvidenceUnavailable`).
    pub fn resolve_evidence_many(
        &self,
        request: &EvidenceResolveManyRequestV1,
    ) -> Result<Vec<EvidenceResolveManyItemV1>, EngineError> {
        let raw_refs: Vec<&str> = request.evidence_refs.iter().map(EvidenceRefV1::as_str).collect();
        self.resolve_evidence_many_slice(request.schema_version, &raw_refs, &request.context)
    }

    /// Binding helper accepting raw string references so FFI boundaries can
    /// route empty or oversized (`> 2048` bytes) tokens into per-slot
    /// nondisclosing `Refused(EvidenceUnavailable, "/evidenceRefs/{i}")`
    /// outcomes rather than aborting the whole batch.
    #[doc(hidden)]
    pub fn resolve_evidence_many_raw_refs(
        &self,
        schema_version: u32,
        raw_refs: &[String],
        context: &FrozenReadContextV1,
    ) -> Result<Vec<EvidenceResolveManyItemV1>, EngineError> {
        let refs: Vec<&str> = raw_refs.iter().map(String::as_str).collect();
        self.resolve_evidence_many_slice(schema_version, &refs, context)
    }

    fn resolve_evidence_many_slice(
        &self,
        schema_version: u32,
        raw_refs: &[&str],
        context: &FrozenReadContextV1,
    ) -> Result<Vec<EvidenceResolveManyItemV1>, EngineError> {
        if schema_version != 1 {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::UnsupportedSchemaVersion,
                "/schemaVersion",
            )
            .into());
        }
        if raw_refs.is_empty() || raw_refs.len() > EVIDENCE_RESOLVE_MANY_MAX {
            return Err(EngineError::InvalidArgument {
                msg: format!(
                    "evidence_refs must contain between 1 and {EVIDENCE_RESOLVE_MANY_MAX} references; got {}",
                    raw_refs.len()
                ),
            });
        }
        let _operation = self.ensure_open()?;
        let scope = self.scope_predicate();
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        let tx = connection.transaction().map_err(|_| EngineError::Storage)?;
        let binding = frozen_read::authenticate(&tx, context, &scope)
            .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
        frozen_read::validate_snapshot(&tx, &binding)
            .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
        let items = evidence::resolve_many_raw(&tx, raw_refs, context, &scope)?;
        #[cfg(any(test, feature = "test-hooks"))]
        self.evidence_before_resolve_return_hook.fire();
        frozen_read::validate_snapshot(&tx, &binding)
            .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
        tx.commit().map_err(|_| EngineError::Storage)?;
        Ok(items)
    }

    /// Resolve one exact graph artifact previously disclosed by frozen expansion.
    ///
    /// The opaque reference is not authority: invalid, foreign, stale, erased,
    /// or context-mismatched references share the nondisclosing unavailable error.
    pub fn resolve_graph_evidence(
        &self,
        request: &GraphEvidenceResolveRequestV1,
    ) -> Result<ResolvedGraphEvidenceV1, EngineError> {
        if request.schema_version != 1 {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::UnsupportedSchemaVersion,
                "/schemaVersion",
            )
            .into());
        }
        let _operation = self.ensure_open()?;
        let scope = self.scope_predicate();
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        let tx = connection.transaction().map_err(|_| EngineError::Storage)?;
        let binding = frozen_read::authenticate(&tx, &request.context, &scope)
            .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
        frozen_read::validate_snapshot(&tx, &binding)
            .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
        let resolved =
            evidence::resolve_graph_evidence(&tx, &request.evidence_ref, &request.context, &scope)?;
        #[cfg(feature = "test-hooks")]
        if let Some(hook) = self
            .graph_evidence_before_resolve_return_hook
            .lock()
            .map_err(|_| EngineError::Storage)?
            .take()
        {
            hook();
        }
        frozen_read::validate_snapshot(&tx, &binding)
            .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
        let artifact = match resolved.artifact_class {
            EvidenceArtifactClassV1::Node => GraphEvidenceArtifactV1::Node {
                logical_id: resolved.logical_id.ok_or_else(EvidenceErrorV1::unavailable)?,
                kind: resolved.artifact_kind,
                body: resolved.artifact_body.ok_or_else(EvidenceErrorV1::unavailable)?,
                properties: resolved.artifact_properties,
            },
            EvidenceArtifactClassV1::Edge => GraphEvidenceArtifactV1::Edge {
                logical_id: resolved.logical_id,
                kind: resolved.artifact_kind,
                body: resolved.artifact_body,
                properties: resolved.artifact_properties,
                from: resolved.edge_from.ok_or_else(EvidenceErrorV1::unavailable)?,
                to: resolved.edge_to.ok_or_else(EvidenceErrorV1::unavailable)?,
            },
        };
        let artifact_lifecycle = match resolved.artifact_class {
            EvidenceArtifactClassV1::Node => EvidenceArtifactLifecycleV1::Node {
                state: LifecycleState::from_str_opt(resolved.node_state.as_deref().unwrap_or(""))
                    .ok_or_else(EvidenceErrorV1::unavailable)?,
                superseded: resolved.artifact_superseded_at.is_some(),
            },
            EvidenceArtifactClassV1::Edge => EvidenceArtifactLifecycleV1::Edge {
                superseded: resolved.artifact_superseded_at.is_some(),
                valid_at_effective: resolved
                    .edge_t_valid
                    .is_none_or(|at| at <= request.context.effective_valid_at)
                    && resolved
                        .edge_t_invalid
                        .is_none_or(|at| at > request.context.effective_valid_at),
            },
        };
        let value = ResolvedGraphEvidenceV1 {
            schema_version: 1,
            artifact_revision_id: ArtifactRevisionId::new(resolved.artifact_revision_id)
                .map_err(|_| EvidenceErrorV1::unavailable())?,
            artifact,
            source_id: resolved.source_id,
            source_version_id: resolved.source_version_id,
            source_revision_id: SourceRevisionId::new(resolved.source_revision_id)
                .map_err(|_| EvidenceErrorV1::unavailable())?,
            locator: resolved.source_locator,
            canonical_source_body: resolved.canonical_source_body,
            evidence_text: resolved.evidence_text,
            canonical_source_hash: CanonicalHash::sha256(resolved.canonical_source_hash)
                .map_err(|_| EvidenceErrorV1::unavailable())?,
            effective_valid_at: request.context.effective_valid_at,
            artifact_lifecycle,
            source_lifecycle_state: LifecycleState::from_str_opt(&resolved.source_state)
                .ok_or_else(EvidenceErrorV1::unavailable)?,
            dependency: resolved.dependency,
        };
        tx.commit().map_err(|_| EngineError::Storage)?;
        Ok(value)
    }

    /// Hybrid search plus bounded expansion on one reader transaction under an
    /// authenticated state binding.
    pub fn search_expand_frozen(
        &self,
        query: &str,
        frozen: &FrozenReadContextV1,
        depth: u32,
        limit: usize,
    ) -> Result<SearchExpandResult, EngineError> {
        let _operation = self.ensure_open()?;
        let scope = self.scope_predicate();
        let binding = {
            let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
            let connection = connection.as_ref().ok_or(EngineError::Closing)?;
            frozen_read::authenticate(connection, frozen, &scope)?
        };
        let (_search, _stats, expanded) = self.search_inner_with_frozen_binding_and_expansion(
            query,
            Some(frozen.context.eligibility.clone()),
            0,
            false,
            0.3,
            0,
            false,
            frozen.context.view,
            limit,
            Some(binding),
            Some(depth),
            None,
            None,
        )?;
        expanded.ok_or(EngineError::Storage)
    }

    /// Executes the TC-5 benchmark-only direct vector stage on one reader-worker
    /// snapshot. This symbol exists only with the `tc5-benchmark` feature and is
    /// deliberately not re-exported by the facade crate.
    #[cfg(feature = "tc5-benchmark")]
    pub fn tc5_vector_stage(
        &self,
        request: tc5_benchmark::VectorStageRequest,
    ) -> Result<tc5_benchmark::VectorStageResult, tc5_benchmark::VectorStageError> {
        self.ensure_open().map_err(|_| tc5_benchmark::VectorStageError::Closing)?;
        let (respond, received) = mpsc::sync_channel(1);
        self.reader_pool
            .dispatch(ReaderRequest::vector_stage(request, respond))
            .map_err(|_| tc5_benchmark::VectorStageError::Closing)?;
        received.recv().map_err(|_| tc5_benchmark::VectorStageError::Closing)?
    }

    /// Thin wrapper: the production search path that discards the G0 Phase-2
    /// frontier meter (it never reaches `SearchResult` / the governed surface).
    #[allow(clippy::too_many_arguments)] // mirrors search_reranked + the explain flag
    pub(crate) fn search_inner(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        explain: bool,
        view: ReadView,
        result_limit: usize,
    ) -> Result<SearchResult, EngineError> {
        self.search_inner_with_stats(
            query,
            filter,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            explain,
            view,
            result_limit,
        )
        .map(|(result, _stats)| result)
    }

    /// G0 Phase-2: the search body, additionally returning the graph-arm frontier
    /// meter. Only the `_graph_frontier_stats_for_test` seam consumes the stats;
    /// `search_inner` (and thus `search_reranked` / `search`) drops them.
    #[allow(clippy::too_many_arguments)] // mirrors search_reranked + the explain flag
    pub(crate) fn search_inner_with_stats(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        explain: bool,
        view: ReadView,
        result_limit: usize,
    ) -> Result<(SearchResult, GraphFrontierStats), EngineError> {
        self.search_inner_with_frozen_binding_and_stats(
            query,
            filter,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            explain,
            view,
            result_limit,
            None,
            None,
            None,
        )
        .map(|(result, stats, _expanded)| (result, stats))
    }

    #[allow(clippy::too_many_arguments)]
    fn search_inner_with_frozen_binding_and_stats(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        explain: bool,
        view: ReadView,
        result_limit: usize,
        frozen_binding: Option<frozen_read::FrozenReadBinding>,
        snippet: Option<SnippetOptions>,
        graph_options: Option<GraphArmOptions>,
    ) -> Result<(SearchResult, GraphFrontierStats, Option<SearchExpandResult>), EngineError> {
        self.search_inner_with_frozen_binding_and_expansion(
            query,
            filter,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            explain,
            view,
            result_limit,
            frozen_binding,
            None,
            snippet,
            graph_options,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn search_inner_with_frozen_binding_and_expansion(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        explain: bool,
        view: ReadView,
        result_limit: usize,
        frozen_binding: Option<frozen_read::FrozenReadBinding>,
        expand_depth: Option<u32>,
        snippet: Option<SnippetOptions>,
        graph_options: Option<GraphArmOptions>,
    ) -> Result<(SearchResult, GraphFrontierStats, Option<SearchExpandResult>), EngineError> {
        #[cfg(any(test, feature = "slice30-observer"))]
        let _observation_scope = crate::slice30_observer::enter_engine(&self.wal_attribution);
        #[cfg(any(test, feature = "slice30-observer"))]
        let observation = crate::slice30_observer::span("api_hybrid");
        let _operation = self.ensure_open()?;
        let is_frozen = frozen_binding.is_some();
        if !is_frozen && query.trim().is_empty() {
            return Err(EngineError::WriteValidation);
        }

        // 0.8.20 Slice 15e fix-2 finding 1 [P2] — every filter attribute name is
        // validated against the declared `filterable` registry set BEFORE any arm
        // runs, so an UNDECLARED name is a typed `InvalidFilter` rejection instead
        // of an opaque `no such column` `Storage` crash (vector arm) or a silent
        // no-match (FTS arm). ADR-0.8.11 D3: every filter term has a DEFINED outcome
        // ("compiles" or "typed rejection") IDENTICAL across arms.
        //
        // keystone closeout fix-3 (codex §9 [P2], TOCTOU): that validation is NO
        // LONGER performed here on `self.connection` before dispatch. fix-2 checked
        // the registry on the WRITER connection and then let the reader prepare the
        // vec0 query on a DIFFERENT connection/snapshot — a `configure_projections`
        // DROP landing in the window between the check and the reader snapshot could
        // still make the `attr_<hex>` column vanish AFTER validation passed, i.e. the
        // exact untyped `Storage` failure fix-2 meant to prevent. The check now runs
        // INSIDE the reader's deferred transaction (see
        // `validate_filter_attributes_on_snapshot`, called from `read_search_in_tx`),
        // so the registry it reads and the vec0 columns the query compiles against are
        // ONE snapshot — the race is closed and BOTH arms still see the same typed
        // `InvalidFilter`. Moving it there also removes a per-filtered-search writer
        // lock and the fix-2 concurrent-ADD false-reject (the reader snapshot sees a
        // freshly-added declaration and accepts).
        #[cfg(any(test, feature = "slice30-observer"))]
        if is_frozen {
            crate::slice30_observer::not_run("query_compile", "frozen_reader_runtime");
            crate::slice30_observer::not_run(
                "query_embedding_dispatch_wall",
                "frozen_reader_runtime",
            );
        }
        let compiled = (!is_frozen).then(|| {
            #[cfg(any(test, feature = "slice30-observer"))]
            let phase = crate::slice30_observer::span("query_compile");
            let compiled = compile_text_query_with(query, &self.query_stop_words);
            #[cfg(any(test, feature = "slice30-observer"))]
            phase.success();
            compiled
        });
        // REQ-013 / AC-059b / REQ-055: the cursor returned with a search
        // MUST be derived from the same WAL snapshot the data was read
        // from. Loading `next_cursor` from the writer-side atomic before
        // the reader transaction acquires its snapshot races against
        // concurrent writers — see `dev/design/engine.md` § Cursor
        // contract. Run cursor probe + body query inside one read tx
        // (BEGIN DEFERRED on a `query_only=ON` connection in WAL mode is
        // a snapshot-stable read).
        // EU-5a2 mean-centering apply path (query side). `query_vector`
        // is ALWAYS un-centered (used by the f32 vec_distance_l2 rerank
        // in phase 2). `query_vector_bin` is the (possibly centered) f32
        // fed to `vec_quantize_binary` in phase 1. The centering decision
        // mirrors the write path: identity must be MC-required AND a
        // mean_vec must be pinned. NoopEmbedder collapses to
        // `query_vector_bin == query_vector` until EU-5b.
        let mut degradations = Vec::new();
        let raw_query_vector = if is_frozen {
            None
        } else {
            #[cfg(any(test, feature = "slice30-observer"))]
            let phase = crate::slice30_observer::span("query_embedding_dispatch_wall");
            let vector = search_query_vector(&self.embed_dispatch, query, &mut degradations)?;
            #[cfg(any(test, feature = "slice30-observer"))]
            phase.success();
            vector
        };
        let (query_vector_bin, query_vector) = {
            #[cfg(any(test, feature = "slice30-observer"))]
            let phase = crate::slice30_observer::span("query_mean_serialization");
            let query_vector_bin = match raw_query_vector.as_ref() {
                Some(vector)
                    if identity_requires_mean_centering(&self.runtime_embedder_identity) =>
                {
                    let pinned = {
                        let connection =
                            self.connection.lock().map_err(|_| EngineError::Storage)?;
                        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
                        read_pinned_mean_vec(connection, self.runtime_embedder_identity.dimension)?
                    };
                    match pinned {
                        Some(mean) => serde_json::to_string(&subtract_mean(vector, &mean)).ok(),
                        None => serde_json::to_string(vector).ok(),
                    }
                }
                Some(vector) => serde_json::to_string(vector).ok(),
                None => None,
            };
            let query_vector =
                raw_query_vector.and_then(|vector| serde_json::to_string(&vector).ok());
            #[cfg(any(test, feature = "slice30-observer"))]
            phase.success();
            (query_vector_bin, query_vector)
        };
        let frozen_query_runtime = is_frozen.then(|| {
            Box::new(FrozenQueryRuntime::new(
                Arc::clone(&self.embed_dispatch),
                self.runtime_embedder_identity.clone(),
                Arc::clone(&self.read_visibility_generation),
            ))
        });
        // The public result limit is independent of the test-only vector
        // candidate fanout. The seam may raise this fanout for recall tests,
        // but the reader still truncates visible results to `result_limit`.
        let candidate_limit = self
            .projection_runtime
            .shared
            .search_limit_override
            .load(Ordering::SeqCst)
            .max(result_limit);
        let recency_enabled =
            self.projection_runtime.shared.recency_reweight_enabled.load(Ordering::SeqCst);
        let importance_enabled =
            self.projection_runtime.shared.importance_reweight_enabled.load(Ordering::SeqCst);
        let vector_stage_only =
            self.projection_runtime.shared.vector_stage_only_for_test.load(Ordering::SeqCst);
        let (response_tx, response_rx) = mpsc::sync_channel::<ReaderResponse>(1);
        let request = ReaderRequest::search(
            SearchReaderWork::hybrid(
                compiled,
                query_vector,
                query_vector_bin,
                result_limit,
                candidate_limit,
                filter,
                recency_enabled,
                importance_enabled,
                vector_stage_only,
                query,
                rerank_depth,
                use_graph_arm,
                alpha,
                pool_n,
                explain,
                if self.runtime_embedder.is_none() {
                    ProjectionRuntimeStateV1::Absent
                } else {
                    ProjectionRuntimeStateV1::Usable
                },
                view,
                frozen_binding,
                frozen_query_runtime,
                expand_depth,
            )
            .with_snippet(snippet)
            .with_graph_options(graph_options),
            response_tx,
        );
        let search_result = {
            #[cfg(any(test, feature = "slice30-observer"))]
            let phase = crate::slice30_observer::span("reader_wait");
            if self.reader_pool.dispatch(request).is_err() {
                return Err(EngineError::Closing);
            }
            let result = response_rx.recv().map_err(|_| EngineError::Storage)?;
            #[cfg(any(test, feature = "slice30-observer"))]
            phase.success();
            result
        };
        #[cfg(any(test, feature = "slice30-observer"))]
        let response_phase = crate::slice30_observer::span("final_response");
        let (
            cursor,
            soft_fallback,
            results,
            graph_stats,
            explanation,
            mut expanded,
            frozen_degradations,
            snippets,
        ) = match search_result {
            Ok(result) => result,
            // fix-3 (codex §9 [P2]) — an undeclared `filterable` attribute is caught
            // on the reader's OWN snapshot (validate + vec0 query = one transaction),
            // so it surfaces as the EXISTING typed `InvalidFilter` and can never be
            // the opaque `no such column` `Storage` error the TOCTOU race produced.
            Err(SearchReaderError::InvalidFilter(reason)) => {
                return Err(EngineError::InvalidFilter { reason });
            }
            Err(SearchReaderError::Evidence(error)) => return Err(error),
            Err(SearchReaderError::RerankerDevicePolicy(error)) => {
                return Err(EngineError::from(error));
            }
            Err(SearchReaderError::FrozenRead(error)) => {
                return Err(EngineError::FrozenRead(error));
            }
            Err(SearchReaderError::WriteValidation) => {
                return Err(EngineError::WriteValidation);
            }
            Err(SearchReaderError::InvalidArgument(msg)) => {
                return Err(EngineError::InvalidArgument { msg });
            }
            Err(SearchReaderError::Sqlite(err)) => {
                self.emit_sqlite_internal_error(&err);
                return Err(EngineError::Storage);
            }
        };

        // Exactly one side embeds the query: this function when not frozen, the
        // reader when frozen.
        degradations.extend(frozen_degradations);
        if let Some(expanded) = expanded.as_mut() {
            expanded.degradations.clone_from(&degradations);
        }

        // The worker (`read_search_in_tx`) has no embedder identity; fill the
        // trace's `embedder_id` here, where `self.runtime_embedder_identity` is in
        // scope. Only on the explain path (`explanation` is `Some`).
        let explanation = explanation.map(|mut exp| {
            let id = &self.runtime_embedder_identity;
            exp.trace.embedder_id = format!("{}@{} (dim={})", id.name, id.revision, id.dimension);
            exp
        });

        let result = (
            SearchResult {
                projection_cursor: cursor,
                soft_fallback,
                results,
                explanation,
                degradations,
                snippets,
                hints: None,
            },
            graph_stats,
            expanded,
        );
        #[cfg(any(test, feature = "slice30-observer"))]
        response_phase.success();
        #[cfg(any(test, feature = "slice30-observer"))]
        observation.success();
        Ok(result)
    }
}

impl Engine {
    pub fn search(&self, query: &str) -> Result<SearchResult, EngineError> {
        self.search_with_limit(query, DEFAULT_SEARCH_RESULT_LIMIT)
    }

    /// Hybrid search with an explicit ranked-result limit in `1..=100`.
    pub fn search_with_limit(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<SearchResult, EngineError> {
        self.search_filtered_with_limit(query, None, limit)
    }

    /// 0.8.20 Slice 15b fix-2 (R-20-NV / R-20-RV) — `search` under an explicit
    /// [`ReadView`], the escape hatch matching the one the five read verbs got in
    /// Slice 10b. `search(query)` is exactly `search_view(query, &ReadView::default())`.
    ///
    /// Validity, supersession and inactive-state flags compose independently.
    /// Historical hybrid search falls back from the whole vector arm when its
    /// visible historical vectors are unavailable; malformed tuples are errors.
    pub fn search_view(&self, query: &str, view: &ReadView) -> Result<SearchResult, EngineError> {
        self.search_view_with_limit(query, view, DEFAULT_SEARCH_RESULT_LIMIT)
    }

    /// Hybrid search under a [`ReadView`] with an explicit ranked-result limit.
    pub fn search_view_with_limit(
        &self,
        query: &str,
        view: &ReadView,
        limit: usize,
    ) -> Result<SearchResult, EngineError> {
        self.search_reranked_view_with_limit(query, None, 0, false, 0.3, 0, false, view, limit)
    }

    /// 0.8.20 Slice 15b fix-2 (R-20-NV / R-20-RV) — the FULL-arity view entry
    /// point: [`search_reranked`][Engine::search_reranked] /
    /// [`search_explained`][Engine::search_explained] under an explicit
    /// [`ReadView`]. This is what the Python and TypeScript `search(..., view=)`
    /// bindings call, so a caller can combine a content filter, the CE knobs and
    /// a validity view in one query — passing `view` must not silently disable
    /// the filter, and passing a filter must not silently disable `view`.
    ///
    /// `search_reranked(q, f, d, g, a, p)` is exactly
    /// `search_reranked_view(q, f, d, g, a, p, false, &ReadView::default())`.
    ///
    /// All view axes compose independently. See
    /// [`search_view`][Engine::search_view].
    ///
    #[allow(clippy::too_many_arguments)] // mirrors search_explained + the view
    pub fn search_reranked_view(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        explain: bool,
        view: &ReadView,
    ) -> Result<SearchResult, EngineError> {
        self.search_reranked_view_with_limit(
            query,
            filter,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            explain,
            view,
            DEFAULT_SEARCH_RESULT_LIMIT,
        )
    }

    /// Full-arity hybrid search under a [`ReadView`] with an explicit ranked-result limit.
    #[allow(clippy::too_many_arguments)]
    pub fn search_reranked_view_with_limit(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        explain: bool,
        view: &ReadView,
        limit: usize,
    ) -> Result<SearchResult, EngineError> {
        let limit = validate_search_result_limit(limit)?;
        self.search_reranked_with_explain(
            query,
            filter,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            explain,
            *view,
            limit,
            None,
            None,
        )
    }

    /// [`search_reranked_view_with_limit`][Engine::search_reranked_view_with_limit]
    /// with optional EP-17 match-centred snippets. `Some(options)` fills
    /// [`SearchResult::snippets`] with one [`SnippetV1`](crate::SnippetV1) per hit, computed on the
    /// search's reader snapshot; `None` is the plain search.
    ///
    /// # Errors
    ///
    /// As the plain search, plus [`EngineError::InvalidArgument`] when
    /// `snippet` fails [`SnippetOptions::validate`].
    #[allow(clippy::too_many_arguments)]
    pub fn search_reranked_view_with_snippets(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        explain: bool,
        view: &ReadView,
        limit: usize,
        snippet: Option<&SnippetOptions>,
    ) -> Result<SearchResult, EngineError> {
        let limit = validate_search_result_limit(limit)?;
        validate_snippet(snippet)?;
        self.search_reranked_with_explain(
            query,
            filter,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            explain,
            *view,
            limit,
            snippet.copied(),
            None,
        )
    }

    /// [`search_reranked_view_with_snippets`][Engine::search_reranked_view_with_snippets]
    /// with opt-in graph-arm seeding and edge-traversal controls.
    ///
    /// # Errors
    ///
    /// Validates `limit`, `snippet`, and `graph_options` at request entry
    /// regardless of `use_graph_arm`.
    #[allow(clippy::too_many_arguments)]
    pub fn search_reranked_view_with_graph_options(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        explain: bool,
        view: &ReadView,
        limit: usize,
        snippet: Option<&SnippetOptions>,
        graph_options: &GraphArmOptions,
    ) -> Result<SearchResult, EngineError> {
        self.search_reranked_view_with_hints(
            query,
            filter,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            explain,
            view,
            limit,
            snippet,
            graph_options,
            false,
        )
    }

    /// [`search_reranked_view_with_graph_options`][Engine::search_reranked_view_with_graph_options]
    /// with opt-in agent search hints (`hints = true`).
    #[allow(clippy::too_many_arguments)]
    pub fn search_reranked_view_with_hints(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        explain: bool,
        view: &ReadView,
        limit: usize,
        snippet: Option<&SnippetOptions>,
        graph_options: &GraphArmOptions,
        hints: bool,
    ) -> Result<SearchResult, EngineError> {
        let limit = validate_search_result_limit(limit)?;
        validate_snippet(snippet)?;
        graph_options.validate()?;
        let mut result = self.search_reranked_with_explain(
            query,
            filter,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            explain,
            *view,
            limit,
            snippet.copied(),
            Some(graph_options.clone()),
        )?;
        if hints {
            result.hints = Some(self.compute_search_hints(&result, limit));
        }
        Ok(result)
    }

    fn compute_search_hints(&self, result: &SearchResult, limit: usize) -> Vec<crate::hints::Hint> {
        let first = result.degradations.first().map(|entry| {
            let arm = match entry.arm {
                SoftFallbackBranch::Vector => crate::hints::HintArm::Vector,
                SoftFallbackBranch::Text => crate::hints::HintArm::Text,
                SoftFallbackBranch::TextEdge => crate::hints::HintArm::TextEdge,
                SoftFallbackBranch::GraphArm => crate::hints::HintArm::GraphArm,
            };
            let cause = match entry.reason {
                DegradationReason::EmbedderAbsent => crate::hints::HintCause::EmbedderAbsent,
                DegradationReason::EmbedderTimeout => crate::hints::HintCause::EmbedderTimeout,
                DegradationReason::EmbedderOverloaded => {
                    crate::hints::HintCause::EmbedderOverloaded
                }
            };
            (arm, cause)
        });
        let hybrid_available = self.runtime_embedder.is_some()
            && !result.degradations.iter().any(|entry| entry.arm == SoftFallbackBranch::Vector);
        let facts = crate::hints::SearchFacts::new(
            result.results.len(),
            limit,
            crate::hints::HintMode::Hybrid,
        )
        .with_degradations(first.as_slice())
        .with_hybrid_available(hybrid_available);
        let mut out = Vec::new();
        crate::hints::search_hints(&facts, &mut out);
        out
    }

    /// G10 — hybrid `search` with an optional closed [`SearchFilter`]. `None`
    /// (or an all-`None` filter) is the unfiltered path whose phase-1 SQL is
    /// subject to native lifecycle and validity eligibility. The filter prunes the vector branch in the
    /// single phase-1 candidates statement and constrains the text branch by the
    /// same metadata. Ranking is the unconditional G9 RRF fusion.
    pub fn search_filtered(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
    ) -> Result<SearchResult, EngineError> {
        self.search_filtered_with_limit(query, filter, DEFAULT_SEARCH_RESULT_LIMIT)
    }

    /// Hybrid search with an optional [`SearchFilter`] and explicit ranked-result limit.
    pub fn search_filtered_with_limit(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        limit: usize,
    ) -> Result<SearchResult, EngineError> {
        // 0.8.11 Slice 40 (R-FIL-2): re-express the shipped G10 `SearchFilter`
        // sugar through the unified `Filter` type, then lower back to the vec0
        // backend's `SearchFilter` (D4). The round-trip is lossless +
        // canonical-order-preserving, so the produced phase-1 SQL stays
        // guarded by native lifecycle and validity on every path. `SearchFilter`
        // never carries a `Json` term, so `to_search_filter` never rejects here.
        //
        // 0.8.20 Slice 15e — the unified `Filter`/`FilterTerm` grammar does not yet
        // carry `filterable`-attribute terms (a later slice adds that surface), so
        // the round-trip would drop `SearchFilter.attributes`. Carry them across
        // explicitly: they already route pre-KNN through `vector_filter_clause`.
        let lowered = filter
            .map(|mut sf| {
                let attributes = sf.attributes.clone();
                // Attribute predicates are intentionally absent from the unified
                // grammar, but this legacy/hybrid entry point owns their existing
                // pre-KNN lowering. Remove them only for the metadata round-trip,
                // then restore them on its `SearchFilter` output.
                sf.attributes.clear();
                Filter::try_from(&sf).and_then(|filter| {
                    Filter::to_search_filter(&filter).map(|mut lo| {
                        lo.attributes = attributes;
                        lo
                    })
                })
            })
            .transpose()?;
        // FIX-6: delegate to search_reranked(depth=0, use_graph_arm=false) to eliminate the
        // ~26-line duplicate body that would otherwise drift with search_reranked.
        // 0.8.5: depth=0 is inert, so the α/pool_n defaults (0.3, 0) never reach the blend.
        self.search_reranked_with_limit(query, lowered, 0, false, 0.3, 0, limit)
    }

    /// 0.8.11 Slice 40 (#17) — unified-`Filter` entry point for the vec0 search
    /// backend. Lowers the metadata subset to the indexed pre-KNN `WHERE` and
    /// **typed-rejects** a [`FilterTerm::Json`] term with
    /// [`EngineError::InvalidFilter`] (D3 no-demotion guarantee). This is the
    /// unified surface the 0.8.15 router `constraints` block reasons over; the
    /// shipped [`Engine::search_filtered`]`(query, Option<SearchFilter>)` stays
    /// as sugar over the same path.
    pub fn search_filter(&self, query: &str, filter: &Filter) -> Result<SearchResult, EngineError> {
        self.search_filter_with_limit(query, filter, DEFAULT_SEARCH_RESULT_LIMIT)
    }

    /// Unified-filter hybrid search with an explicit ranked-result limit.
    pub fn search_filter_with_limit(
        &self,
        query: &str,
        filter: &Filter,
        limit: usize,
    ) -> Result<SearchResult, EngineError> {
        let sf = filter.to_search_filter()?;
        self.search_reranked_with_limit(query, Some(sf), 0, false, 0.3, 0, limit)
    }

    /// 0.8.1 Slice 10 (R1) / Slice 30 (R3) — `search_reranked`: hybrid search
    /// with optional CE reranking and optional graph-BFS third arm. `rerank_depth
    /// = 0` is the identity (soft-fallback) path, byte-identical to
    /// [`search_filtered`][Engine::search_filtered]. `rerank_depth = N > 0`
    /// applies the cross-encoder over the top-N fused hits (when the
    /// `default-reranker` feature is enabled and the model is loaded); without the
    /// model, the call falls back to the fused order.
    ///
    /// `use_graph_arm = false` (the default) produces byte-identical results to
    /// the pre-Slice-30 two-arm pipeline. `use_graph_arm = true` seeds a BFS over
    /// temporal fact-edges from the top-10 fused hits and fuses the reachable
    /// nodes as a third RRF arm.
    ///
    /// Governed surface: re-exported from `fathomdb` facade.
    pub fn search_reranked(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
    ) -> Result<SearchResult, EngineError> {
        self.search_reranked_with_limit(
            query,
            filter,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            DEFAULT_SEARCH_RESULT_LIMIT,
        )
    }

    /// Hybrid search with optional reranking and an explicit ranked-result limit.
    #[allow(clippy::too_many_arguments)]
    pub fn search_reranked_with_limit(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        limit: usize,
    ) -> Result<SearchResult, EngineError> {
        let limit = validate_search_result_limit(limit)?;
        // explain=false → `SearchResult.explanation == None`, byte-identical results.
        self.search_reranked_with_explain(
            query,
            filter,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            false,
            ReadView::default(),
            limit,
            None,
            None,
        )
    }

    /// 0.8.8 EXP-OBS (Slice 5) — `search_explained`: the opt-in `explain=true`
    /// surface. Identical retrieval to [`search_reranked`][Engine::search_reranked]
    /// (same fused/CE ranking, same `results`), additionally returning a
    /// [`Explanation`] sidecar on `SearchResult.explanation` with per-hit arm
    /// provenance + score breakdown + a query-level [`QueryTrace`]. The default
    /// `search`/`search_filtered`/`search_reranked` paths are unaffected and stay
    /// byte-identical (R-OBS-2).
    ///
    /// Governed surface: re-exported from `fathomdb` facade.
    pub fn search_explained(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
    ) -> Result<SearchResult, EngineError> {
        self.search_explained_with_limit(
            query,
            filter,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            DEFAULT_SEARCH_RESULT_LIMIT,
        )
    }

    /// Explained hybrid search with an explicit ranked-result limit.
    #[allow(clippy::too_many_arguments)]
    pub fn search_explained_with_limit(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        limit: usize,
    ) -> Result<SearchResult, EngineError> {
        let limit = validate_search_result_limit(limit)?;
        self.search_reranked_with_explain(
            query,
            filter,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            true,
            ReadView::default(),
            limit,
            None,
            None,
        )
    }

    /// Search node and edge bodies using FTS only, without embedding the query,
    /// vector recall, CE reranking or graph expansion. Available on a successfully
    /// opened engine, including an explicit no-provider recovery session after
    /// failed compatibility admission. Matching node- and edge-body
    /// candidates are body-deduplicated and deterministically ranked before
    /// the requested result limit is applied.
    ///
    /// Governed surface: re-exported from the `fathomdb` facade + Py/TS bindings.
    pub fn search_text_only(&self, query: &str) -> Result<SearchResult, EngineError> {
        self.search_text_only_with_limit(query, DEFAULT_SEARCH_RESULT_LIMIT)
    }

    /// Text-only search with an explicit ranked-result limit in `1..=100`.
    ///
    /// For the same immutable selection and effective validity time, a smaller
    /// limit's ordered results are the prefix of a larger limit's results.
    pub fn search_text_only_with_limit(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<SearchResult, EngineError> {
        self.search_text_only_view_with_limit(query, &ReadView::default(), limit)
    }

    /// 0.8.20 Slice 15b fix-2 (R-20-NV / R-20-RV) — [`search_text_only`][Engine::search_text_only]
    /// under an explicit [`ReadView`]. Honors all independent view axes without
    /// reporting fallback for the unused vector arm.
    ///
    pub fn search_text_only_view(
        &self,
        query: &str,
        view: &ReadView,
    ) -> Result<SearchResult, EngineError> {
        self.search_text_only_view_with_limit(query, view, DEFAULT_SEARCH_RESULT_LIMIT)
    }

    /// Text-only search under a [`ReadView`] with an explicit ranked-result limit.
    ///
    /// For the same immutable selection and explicit effective validity time, a
    /// smaller limit's ordered results are the prefix of a larger limit's
    /// results. `ReadView::valid_as_of = None` resolves independently per call,
    /// so callers comparing calls must provide a fixed value.
    pub fn search_text_only_view_with_limit(
        &self,
        query: &str,
        view: &ReadView,
        limit: usize,
    ) -> Result<SearchResult, EngineError> {
        self.search_text_only_filtered(query, None, view, limit)
    }

    /// Search node and edge bodies using FTS only with a closed metadata filter.
    ///
    /// The filter is validated on the reader snapshot and applied before candidate
    /// bounds, body deduplication and ranking. Attribute predicates are node-scoped.
    /// Status and creation-time predicates require existing projected metadata;
    /// missing metadata fails to match. No embedding or vector retrieval runs.
    /// `view` controls validity and existence; `limit` must be in `1..=100`.
    /// At fixed corpus, view time and filter, smaller limits are ordered prefixes.
    ///
    /// # Errors
    /// Returns `InvalidFilter` for undeclared/non-filterable attributes,
    /// `InvalidArgument` for invalid limits, `WriteValidation` for blank queries,
    /// and the existing closing/storage errors.
    pub fn search_text_only_filtered(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        view: &ReadView,
        limit: usize,
    ) -> Result<SearchResult, EngineError> {
        self.search_text_only_filtered_with_snippets(query, filter, view, limit, None)
    }

    /// Filter body FTS before ranking and optionally extract snippets from the same reader snapshot.
    ///
    /// Inputs and errors follow [`Engine::search_text_only_filtered`]; invalid snippet
    /// options return [`EngineError::InvalidArgument`]. Snippets preserve hit order and eligibility.
    pub fn search_text_only_filtered_with_snippets(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        view: &ReadView,
        limit: usize,
        snippet: Option<&SnippetOptions>,
    ) -> Result<SearchResult, EngineError> {
        #[cfg(any(test, feature = "slice30-observer"))]
        let _observation_scope = crate::slice30_observer::enter_engine(&self.wal_attribution);
        #[cfg(any(test, feature = "slice30-observer"))]
        let observation = crate::slice30_observer::span("api_text");
        let filter = filter.filter(|filter| !filter.is_unfiltered());
        let limit = validate_search_result_limit(limit)?;
        validate_snippet(snippet)?;
        let _operation = self.ensure_open()?;
        if query.trim().is_empty() {
            return Err(EngineError::WriteValidation);
        }
        let compiled = {
            #[cfg(any(test, feature = "slice30-observer"))]
            let phase = crate::slice30_observer::span("query_compile");
            let compiled = compile_text_query_with(query, &self.query_stop_words);
            #[cfg(any(test, feature = "slice30-observer"))]
            phase.success();
            compiled
        };
        let candidate_limit =
            self.projection_runtime.shared.search_limit_override.load(Ordering::SeqCst).max(limit);
        let (response_tx, response_rx) = mpsc::sync_channel::<ReaderResponse>(1);
        // This explicit marker distinguishes direct text-only search from a hybrid
        // request whose embedder yields no vector. Only the direct path gets the
        // fixed node candidate bound before node/edge body deduplication and RRF.
        let request = ReaderRequest::search(
            SearchReaderWork::text_only(compiled, query, limit, candidate_limit, *view, filter)
                .with_snippet(snippet.copied()),
            response_tx,
        );
        let search_result = {
            #[cfg(any(test, feature = "slice30-observer"))]
            let phase = crate::slice30_observer::span("reader_wait");
            if self.reader_pool.dispatch(request).is_err() {
                return Err(EngineError::Closing);
            }
            let result = response_rx.recv().map_err(|_| EngineError::Storage)?;
            #[cfg(any(test, feature = "slice30-observer"))]
            phase.success();
            result
        };
        #[cfg(any(test, feature = "slice30-observer"))]
        let response_phase = crate::slice30_observer::span("final_response");
        let (
            cursor,
            soft_fallback,
            results,
            _graph_stats,
            explanation,
            _expanded,
            _degradations,
            snippets,
        ) = match search_result {
            Ok(result) => result,
            // fix-3 (codex §9 [P2]) — carry the reader-snapshot validation verdict
            // through: an undeclared `filterable` attribute is the EXISTING typed
            // `InvalidFilter`, never collapsed to `Storage`.
            Err(SearchReaderError::InvalidFilter(reason)) => {
                return Err(EngineError::InvalidFilter { reason });
            }
            Err(SearchReaderError::Evidence(error)) => return Err(error),
            Err(SearchReaderError::RerankerDevicePolicy(error)) => {
                return Err(EngineError::from(error));
            }
            Err(SearchReaderError::FrozenRead(error)) => {
                return Err(EngineError::FrozenRead(error));
            }
            Err(SearchReaderError::WriteValidation) => {
                return Err(EngineError::WriteValidation);
            }
            Err(SearchReaderError::InvalidArgument(msg)) => {
                return Err(EngineError::InvalidArgument { msg });
            }
            Err(SearchReaderError::Sqlite(err)) => {
                self.emit_sqlite_internal_error(&err);
                return Err(EngineError::Storage);
            }
        };
        let result = SearchResult {
            projection_cursor: cursor,
            soft_fallback,
            results,
            explanation,
            degradations: Vec::new(),
            snippets,
            hints: None,
        };
        #[cfg(any(test, feature = "slice30-observer"))]
        response_phase.success();
        #[cfg(any(test, feature = "slice30-observer"))]
        observation.success();
        Ok(result)
    }

    /// [`search_text_only_view_with_limit`][Engine::search_text_only_view_with_limit]
    /// with optional EP-17 match-centred snippets (see
    /// [`search_reranked_view_with_snippets`][Engine::search_reranked_view_with_snippets]).
    ///
    /// # Errors
    ///
    /// As the plain search, plus [`EngineError::InvalidArgument`] when
    /// `snippet` fails [`SnippetOptions::validate`].
    pub fn search_text_only_view_with_snippets(
        &self,
        query: &str,
        view: &ReadView,
        limit: usize,
        snippet: Option<&SnippetOptions>,
    ) -> Result<SearchResult, EngineError> {
        self.search_text_only_filtered_with_snippets(query, None, view, limit, snippet)
    }

    /// Search one declared `searchable→FTS` projection without invoking body
    /// search, vector search, score fusion, or a fallback arm. Results carry the
    /// ordinary text branch shape and are ordered by property-FTS bm25 ascending
    /// then write cursor ascending.
    pub fn search_projected_text(
        &self,
        query: &str,
        name: &str,
        filter: Option<SearchFilter>,
        view: &ReadView,
    ) -> Result<SearchResult, EngineError> {
        self.search_projected_text_with_limit(
            query,
            name,
            filter,
            view,
            DEFAULT_SEARCH_RESULT_LIMIT,
        )
    }

    /// Search one declared property-FTS projection with an explicit ranked-result limit.
    pub fn search_projected_text_with_limit(
        &self,
        query: &str,
        name: &str,
        filter: Option<SearchFilter>,
        view: &ReadView,
        limit: usize,
    ) -> Result<SearchResult, EngineError> {
        let limit = validate_search_result_limit(limit)?;
        let _operation = self.ensure_open()?;
        if query.trim().is_empty() {
            return Err(EngineError::WriteValidation);
        }

        let (response_tx, response_rx) = mpsc::sync_channel(1);
        let request = ReaderRequest::projected_text(
            query.to_string(),
            name.to_string(),
            filter,
            limit,
            *view,
            response_tx,
        );
        if self.reader_pool.dispatch(request).is_err() {
            return Err(EngineError::Closing);
        }
        match response_rx.recv().map_err(|_| EngineError::Storage)? {
            Ok(result) => Ok(result),
            Err(SearchReaderError::Evidence(error)) => Err(error),
            Err(SearchReaderError::InvalidFilter(reason)) => {
                Err(EngineError::InvalidFilter { reason })
            }
            Err(SearchReaderError::RerankerDevicePolicy(error)) => Err(EngineError::from(error)),
            Err(SearchReaderError::FrozenRead(error)) => Err(EngineError::FrozenRead(error)),
            Err(SearchReaderError::WriteValidation) => Err(EngineError::WriteValidation),
            Err(SearchReaderError::InvalidArgument(msg)) => {
                Err(EngineError::InvalidArgument { msg })
            }
            Err(SearchReaderError::Sqlite(err)) => {
                self.emit_sqlite_internal_error(&err);
                Err(EngineError::Storage)
            }
        }
    }

    /// Shared event-wrapped body for [`search_reranked`][Engine::search_reranked]
    /// (`explain=false`) and [`search_explained`][Engine::search_explained]
    /// (`explain=true`). Keeps the Started/Finished/Failed lifecycle emissions +
    /// slow detection in one place.
    #[allow(clippy::too_many_arguments)] // mirrors search_reranked + the explain flag
    fn search_reranked_with_explain(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        explain: bool,
        view: ReadView,
        limit: usize,
        snippet: Option<SnippetOptions>,
        graph_options: Option<GraphArmOptions>,
    ) -> Result<SearchResult, EngineError> {
        let _operation = self.ensure_open()?;
        self.emit_event(lifecycle::Phase::Started, lifecycle::EventCategory::Search, None);
        let started = Instant::now();
        let outcome = self
            .search_inner_with_frozen_binding_and_stats(
                query,
                filter,
                rerank_depth,
                use_graph_arm,
                alpha,
                pool_n,
                explain,
                view,
                limit,
                None,
                snippet,
                graph_options,
            )
            .map(|(result, _stats, _expanded)| result);
        self.detect_slow(started, lifecycle::EventCategory::Search);
        match outcome {
            Ok(mut result) => {
                self.counters.record_query();
                self.finalize_search_observability(query, &mut result);
                self.emit_event(lifecycle::Phase::Finished, lifecycle::EventCategory::Search, None);
                Ok(result)
            }
            Err(err) => {
                let code = err.stable_code();
                self.counters.record_error(code);
                self.emit_event(
                    lifecycle::Phase::Failed,
                    lifecycle::EventCategory::Search,
                    Some(code),
                );
                self.emit_event(
                    lifecycle::Phase::Failed,
                    lifecycle::EventCategory::Error,
                    Some(code),
                );
                Err(err)
            }
        }
    }

    /// F5 (0.8.14 Slice 10) — the fielded BM25F lexical arm over
    /// `search_index_v2`. Recalls candidate rows through the FTS5 index
    /// (`search_index_v2 MATCH`) and scores them with a textbook BM25F using the
    /// plan's tunable per-field `weights` and tunable `b`/`k1`, returning
    /// `(write_cursor, score)` in descending score order (write_cursor asc as the
    /// deterministic tiebreak). Superseded node versions are excluded (join to
    /// `canonical_nodes WHERE superseded_at IS NULL`).
    ///
    /// This is the engine-internal `BM25fQueryPlan` compiler path (`ADR-0.8.1`
    /// §3.2); there is no public Py/TS SDK surface this release. The score is
    /// computed in-engine (not via SQLite's `bm25()`, which cannot express a
    /// tunable `b`); the FTS5 index remains load-bearing for candidate recall.
    #[doc(hidden)]
    pub fn bm25f_search(
        &self,
        query: &str,
        plan: &Bm25fQueryPlan,
    ) -> Result<Vec<(u64, f64)>, EngineError> {
        let _operation = self.ensure_open()?;
        let scope = self.scope_predicate();
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        bm25f_search_inner(connection, query, plan, &scope).map_err(|_| EngineError::Storage)
    }
}

impl Engine {
    /// Slice 20 (G6) — `search_expand`: hybrid search (`G1+G9`) followed by
    /// bounded BFS expansion (`G5`) of each search hit. Returns the original
    /// search hits (with RRF scores) plus nodes reachable from any hit via
    /// up to `depth` hops that are NOT already in the search hit set.
    ///
    /// Returns `Err(EngineError::InvalidArgument)` for `depth > 3`.
    /// A `depth = 0` call returns search hits with their logical_ids resolved
    /// but no BFS expansion. Reads ride the `ReaderWorkerPool` DEFERRED-tx path.
    ///
    /// **Snapshot note:** the search phase (`search_inner`) and the expansion
    /// phase (`SearchExpand` reader request) run in separate DEFERRED reader
    /// transactions; a write that lands between them is visible to expansion
    /// but not search (or vice-versa). In practice the window is negligible for
    /// single-process embedded use. The expansion phase mitigates drift by
    /// filtering `search_hits` to only include hits whose `write_cursor` is
    /// still active in the expansion snapshot (superseded hits are dropped from
    /// the result rather than surfaced with stale data).
    pub fn search_expand(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        depth: u32,
        view: Option<ReadView>,
    ) -> Result<SearchExpandResult, EngineError> {
        self.search_expand_with_limit(query, filter, depth, DEFAULT_SEARCH_RESULT_LIMIT, view)
    }

    /// Hybrid search followed by graph expansion, with an explicit limit for
    /// the initial ranked `search_hits` set.
    pub fn search_expand_with_limit(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        depth: u32,
        limit: usize,
        view: Option<ReadView>,
    ) -> Result<SearchExpandResult, EngineError> {
        let effective_view = view.unwrap_or_default();
        let limit = validate_search_result_limit(limit)?;
        let _operation = self.ensure_open()?;
        if depth > 3 {
            return Err(EngineError::InvalidArgument {
                msg: format!("traversal depth {depth} exceeds the SDK ceiling of 3"),
            });
        }
        // Step 1: run the hybrid search to get initial hits (no CE reranking in expand).
        // 0.8.5: depth=0 → no rerank, so α/pool_n (0.3, 0) are inert here.
        let search_result =
            self.search_inner(query, filter, 0, false, 0.3, 0, false, effective_view, limit)?;
        let degradations = search_result.degradations;
        if search_result.results.is_empty() {
            return Ok(SearchExpandResult {
                search_hits: Vec::new(),
                expanded: Vec::new(),
                all_logical_ids: Vec::new(),
                degradations,
            });
        }
        // Step 2: dispatch to the reader pool to resolve logical_ids and run BFS.
        // depth=0 is forwarded to the reader so it can populate all_logical_ids
        // (the union of search-hit logical_ids), even with no expansion.
        let (response_tx, response_rx) = mpsc::sync_channel(1);
        let request = ReaderRequest::search_expand(
            search_result.results,
            depth,
            effective_view,
            None,
            None,
            response_tx,
        );
        if self.reader_pool.dispatch(request).is_err() {
            return Err(EngineError::Closing);
        }
        match response_rx.recv().map_err(|_| EngineError::Storage)? {
            Ok(result) => Ok(SearchExpandResult { degradations, ..result }),
            Err(graph_expand::SearchExpandHandlerError::InvalidFilter(reason)) => {
                Err(EngineError::InvalidFilter { reason })
            }
            Err(graph_expand::SearchExpandHandlerError::FrozenRead(error)) => {
                Err(EngineError::FrozenRead(error))
            }
            Err(graph_expand::SearchExpandHandlerError::Sqlite(err)) => {
                self.emit_sqlite_internal_error(&err);
                Err(EngineError::Storage)
            }
        }
    }
}
use crate::embedding::search_query_vector;
use crate::errors::EngineError;
use crate::evidence::{
    self, EvidenceArtifactClassV1, EvidenceArtifactLifecycleV1, EvidenceErrorReasonV1,
    EvidenceErrorV1, EvidenceRefV1, EvidenceResolveManyItemV1, EvidenceResolveManyRequestV1,
    EvidenceResolveRequestV1, EvidenceSearchRequestV1, EvidenceSearchResultV1,
    GraphEvidenceArtifactV1, GraphEvidenceResolveRequestV1, ResolvedEvidenceV1,
    ResolvedGraphEvidenceV1, EVIDENCE_RESOLVE_MANY_MAX,
};
#[cfg(any(test, feature = "test-hooks"))]
use crate::filter::{
    append_edge_eligibility_sql, append_node_eligibility_sql, body_fts_rank_sql,
    build_vector_phase1_sql, edge_fts_rank_sql, property_fts_rank_sql,
};
use crate::filter::{Filter, SearchFilter};
use crate::frozen_read::{
    self, FrozenReadContextV1, FrozenReadError, FrozenReadErrorReason, ReadContextV1,
};
use crate::graph_expand::{self, SearchExpandResult};
use crate::identity::{ArtifactRevisionId, CanonicalHash, SourceRevisionId};
use crate::lifecycle;
use crate::mean::{identity_requires_mean_centering, read_pinned_mean_vec, subtract_mean};
use crate::projection_generation::ProjectionRuntimeStateV1;
use crate::reader_pool::{EvidenceReaderResponse, ReaderRequest, ReaderResponse};
use crate::record_lifecycle::LifecycleState;
#[cfg(feature = "test-hooks")]
use crate::search::slice71_search_statement_trace;
use crate::search::{bm25f_search_inner, FrozenQueryRuntime, SearchReaderError, SearchReaderWork};
#[cfg(any(test, feature = "test-hooks"))]
use crate::search_types::SEARCH_RERANK_LIMIT;
use crate::search_types::{
    validate_search_result_limit, Bm25fQueryPlan, DegradationReason, GraphArmOptions,
    GraphFrontierStats, SearchResult, SoftFallbackBranch, DEFAULT_SEARCH_RESULT_LIMIT,
};
use crate::snippet::SnippetOptions;
#[cfg(feature = "tc5-benchmark")]
use crate::tc5_benchmark;
use crate::temporal::ReadView;
use crate::Engine;
use fathomdb_query::compile_text_query_with;
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc};
use std::time::Instant;

impl Engine {
    /// G0 Phase-2 (BLOCK-1) test seam — runs the graph-arm retrieval path and
    /// returns the frontier meter (`GraphFrontierStats`) for `query`. Mirrors the
    /// sanctioned `set_vector_stage_only_for_test` / `_configure_vector_kind_for_test`
    /// pattern: kept OFF the governed surface (test/eval-only), so the meter never
    /// appears on `SearchResult`. Used by the recall harness to prove the
    /// doc-seeded frontier is empty (`resolved_seed_rate == 0.0`) and, post-C1, the
    /// 0→>0 flip.
    pub fn _graph_frontier_stats_for_test(
        &self,
        query: &str,
    ) -> Result<GraphFrontierStats, EngineError> {
        self.search_inner_with_stats(
            query,
            None,
            0,
            true,
            0.3,
            0,
            false,
            ReadView::default(),
            DEFAULT_SEARCH_RESULT_LIMIT,
        )
        .map(|(_result, stats)| stats)
    }

    /// Test seam — runs the graph-arm retrieval path with explicit filter, view,
    /// and [`GraphArmOptions`] and returns its frontier meter.
    #[doc(hidden)]
    pub fn _graph_frontier_stats_with_options_for_test(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        view: &ReadView,
        graph_options: &GraphArmOptions,
    ) -> Result<GraphFrontierStats, EngineError> {
        graph_options.validate()?;
        self.search_inner_with_frozen_binding_and_stats(
            query,
            filter,
            0,
            true,
            0.3,
            0,
            false,
            *view,
            DEFAULT_SEARCH_RESULT_LIMIT,
            None,
            None,
            Some(graph_options.clone()),
        )
        .map(|(_result, stats, _expanded)| stats)
    }

    /// Test seam — runs SQLite `EXPLAIN QUERY PLAN` on the exact `use_graph_arm`
    /// Phase 2 `UNION ALL` edge lookup SQL against the live database.
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn explain_graph_arm_phase2_for_test(
        &self,
        edge_kinds: &[String],
    ) -> Result<Vec<String>, EngineError> {
        let _operation = self.ensure_open()?;
        let scope = self.scope_predicate();
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        crate::search::explain_graph_arm_phase2_query_for_test(connection, edge_kinds, &scope)
            .map_err(|_| EngineError::Storage)
    }

    /// Test seam that raises vector-candidate fanout for recall tests.
    ///
    /// This does not alter the caller-requested final result limit or the
    /// caller-visible result cardinality. Production uses the default and
    /// never consults an environment variable.
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn set_search_limit_for_test(&self, limit: usize) {
        self.projection_runtime.shared.search_limit_override.store(limit, Ordering::SeqCst);
    }

    /// Slice 10 / G12-recency test seam — flip the dedicated recency-reweight
    /// flag (off by default). The reweight runs AFTER bit-KNN on the fused hits;
    /// it is never a vec0 predicate and is NOT `fusion_mode`.
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn set_recency_reweight_enabled_for_test(&self, enabled: bool) {
        self.projection_runtime.shared.recency_reweight_enabled.store(enabled, Ordering::SeqCst);
    }

    /// 0.8.16 Slice 5 / F9 test seam — flip the dedicated importance/confidence
    /// reweight flag (off by default). The reweight runs AFTER bit-KNN + RRF on
    /// the fused hits (multiplicative-on-fused, `NULL ⇒ neutral`); it is never a
    /// vec0 predicate and is NOT `fusion_mode`. Mirrors
    /// `set_recency_reweight_enabled_for_test`.
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn set_importance_reweight_enabled_for_test(&self, enabled: bool) {
        self.projection_runtime.shared.importance_reweight_enabled.store(enabled, Ordering::SeqCst);
    }

    /// GA-2 / Slice-40 (◆ B-1) measurement seam — make `search()` return the
    /// pre-fusion VECTOR-branch ranking (the ANN+ bit-KNN K=192 + f32 rerank
    /// signal) instead of the unconditional RRF-fused result, so the eu7 recall
    /// gate (AC-075) can measure ANN-quantization FIDELITY — vector top-10 vs
    /// the exact-f32 VECTOR top-10 ground truth — in isolation. Off by default;
    /// never set on any production path. This is NOT a `fusion_mode` knob:
    /// production RRF fusion stays unconditional and `fuse_rrf`/`rerank_fused`/
    /// recency are unchanged. Mirrors `set_recency_reweight_enabled_for_test`
    /// (available in release fixtures with explicit `test-hooks`).
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn set_vector_stage_only_for_test(&self, enabled: bool) {
        self.projection_runtime.shared.vector_stage_only_for_test.store(enabled, Ordering::SeqCst);
    }

    /// Test seam — runs SQLite `EXPLAIN QUERY PLAN` on the exact phase-1 vector
    /// SQL built by [`build_vector_phase1_plan`] against the live database.
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn explain_vector_phase1_for_test(
        &self,
        filter: Option<&SearchFilter>,
    ) -> Result<Vec<String>, EngineError> {
        let _operation = self.ensure_open()?;
        let scope = self.scope_predicate();
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let (sql, tail_params) = crate::filter::build_vector_phase1_plan(
            Some(connection),
            filter,
            SEARCH_RERANK_LIMIT,
            &ReadView::default(),
            0,
            &scope,
            false,
        )
        .map_err(|_| EngineError::Storage)?;
        let mut all_params = vec![
            rusqlite::types::Value::Text("[1,0,0,0,0,0,0,0]".to_string()),
            rusqlite::types::Value::Text("[1,0,0,0,0,0,0,0]".to_string()),
        ];
        all_params.extend(tail_params);
        let mut stmt = connection
            .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
            .map_err(|_| EngineError::Storage)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(all_params.iter()), |row| row.get::<_, String>(3))
            .map_err(|_| EngineError::Storage)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|_| EngineError::Storage)
    }
}

/// Test seam — exposes [`build_vector_phase1_sql`] at the production
/// `SEARCH_RERANK_LIMIT` so tests can pin pre-cap eligibility and filter bindings.
#[cfg(any(test, feature = "test-hooks"))]
#[doc(hidden)]
#[must_use]
pub fn vector_phase1_sql_for_test(filter: Option<&SearchFilter>) -> String {
    build_vector_phase1_sql(filter, SEARCH_RERANK_LIMIT, &ReadView::default())
}

/// Test seam pinning production ranked SQL around the shared eligibility
/// compiler. Eligibility must precede ranking and any SQL candidate limit.
#[cfg(any(test, feature = "test-hooks"))]
#[doc(hidden)]
#[must_use]
pub fn slice35_ranked_eligibility_sql_for_test(
    filter: &SearchFilter,
) -> [(&'static str, String); 3] {
    let mut body_params = vec![rusqlite::types::Value::Text("fixture".to_string())];
    let body_filter = append_node_eligibility_sql(Some(filter), "cn", &mut body_params);
    let mut edge_params = vec![
        rusqlite::types::Value::Text("fixture".to_string()),
        rusqlite::types::Value::Integer(0),
    ];
    let edge_filter = append_edge_eligibility_sql(Some(filter), "ce", &mut edge_params);
    let mut property_params = vec![
        rusqlite::types::Value::Text("owner".to_string()),
        rusqlite::types::Value::Text("fixture".to_string()),
    ];
    let property_filter = append_node_eligibility_sql(Some(filter), "n", &mut property_params);
    [
        ("body_fts", body_fts_rank_sql("", "", &body_filter, " LIMIT 10")),
        ("edge_fts", edge_fts_rank_sql("", &edge_filter)),
        ("property_fts", property_fts_rank_sql("", &property_filter)),
    ]
}

/// Return and clear the normalized Slice 71 search-statement trace.
#[cfg(feature = "test-hooks")]
#[doc(hidden)]
pub fn take_slice71_search_statement_trace_for_test() -> Vec<String> {
    slice71_search_statement_trace()
        .lock()
        .map(|mut trace| std::mem::take(&mut *trace))
        .unwrap_or_default()
}

/// Enable or disable Slice 41 raw SQL statement tracing across managed connections.
#[cfg(feature = "test-hooks")]
#[doc(hidden)]
pub fn set_slice41_sql_trace_enabled_for_test(enabled: bool) {
    crate::search::set_slice41_sql_trace_enabled(enabled);
}

/// Return and clear the captured Slice 41 raw SQL statement trace.
#[cfg(feature = "test-hooks")]
#[doc(hidden)]
pub fn take_slice41_sql_trace_for_test() -> Vec<String> {
    crate::search::slice41_sql_trace()
        .lock()
        .map(|mut trace| std::mem::take(&mut *trace))
        .unwrap_or_default()
}

fn validate_snippet(snippet: Option<&SnippetOptions>) -> Result<(), EngineError> {
    snippet.map_or(Ok(()), SnippetOptions::validate)
}
