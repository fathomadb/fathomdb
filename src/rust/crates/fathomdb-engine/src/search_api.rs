use super::*;

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
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        let (frozen, generation) = frozen_read::mint(connection, context)?;
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
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let binding = frozen_read::authenticate(connection, frozen)?;
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
        self.ensure_open()?;
        let binding = {
            let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
            let connection = connection.as_ref().ok_or(EngineError::Closing)?;
            frozen_read::authenticate(connection, frozen)?
        };
        frozen.context.view.reject_existence_relaxation_on_search()?;
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
        )
        .map(|(mut result, _stats, _expanded)| {
            if result.explanation.is_some() {
                self.finalize_search_observability(query, &mut result);
            }
            result
        })
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
        self.ensure_open()?;
        let binding = {
            let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
            let connection = connection.as_ref().ok_or(EngineError::Closing)?;
            frozen_read::authenticate(connection, &request.context)
                .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?
        };
        request.context.context.view.reject_existence_relaxation_on_search()?;
        let dense_disabled_reason = self.dense_disabled.load(Ordering::Acquire).then(|| {
            self.dense_disabled_reason
                .lock()
                .ok()
                .and_then(|guard| guard.clone())
                .unwrap_or_else(|| "open-time #5 vector-equivalence self-check failed".to_string())
        });
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
            if self.runtime_embedder.is_none() {
                ProjectionRuntimeStateV1::Absent
            } else if self.dense_disabled.load(Ordering::Acquire) {
                ProjectionRuntimeStateV1::Refused
            } else {
                ProjectionRuntimeStateV1::Usable
            },
            request.context.context.view,
            binding,
            FrozenQueryRuntime::new(
                self.runtime_embedder.clone(),
                self.runtime_embedder_identity.clone(),
                dense_disabled_reason,
                Arc::clone(&self.read_visibility_generation),
            ),
        );
        let (response_tx, response_rx) = mpsc::sync_channel::<EvidenceReaderResponse>(1);
        self.reader_pool
            .dispatch(ReaderRequest::SearchEvidence(Box::new(EvidenceSearchReaderRequest {
                work,
                frozen: request.context.clone(),
                include_explanation: request.include_explanation,
                respond: response_tx,
            })))
            .map_err(|_| EngineError::Closing)?;
        let mut result = match response_rx.recv().map_err(|_| EngineError::Storage)? {
            Ok(result) => result,
            Err(SearchReaderError::Evidence(error)) => return Err(error),
            Err(SearchReaderError::InvalidFilter(reason)) => {
                return Err(EngineError::InvalidFilter { reason });
            }
            Err(SearchReaderError::RerankerDevicePolicy(error)) => {
                return Err(EngineError::RerankerDevicePolicy(error));
            }
            Err(SearchReaderError::FrozenRead(_)) => {
                return Err(EngineError::Evidence(EvidenceErrorV1::unavailable()))
            }
            Err(SearchReaderError::VectorEquivalenceMismatch(reason)) => {
                self.vector_equivalence_refusals.fetch_add(1, Ordering::Relaxed);
                return Err(EngineError::VectorEquivalenceMismatch { reason });
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
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        let tx = connection.transaction().map_err(|_| EngineError::Storage)?;
        let binding = frozen_read::authenticate(&tx, &request.context)
            .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
        frozen_read::validate_snapshot(&tx, &binding)
            .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
        let result = evidence::resolve(&tx, request)?;
        evidence_linearization_hooks::fire_before_resolve_return();
        frozen_read::validate_snapshot(&tx, &binding)
            .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
        tx.commit().map_err(|_| EngineError::Storage)?;
        Ok(result)
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
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        let tx = connection.transaction().map_err(|_| EngineError::Storage)?;
        let binding = frozen_read::authenticate(&tx, &request.context)
            .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
        frozen_read::validate_snapshot(&tx, &binding)
            .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
        let resolved =
            evidence::resolve_graph_evidence(&tx, &request.evidence_ref, &request.context)?;
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
            },
            EvidenceArtifactClassV1::Edge => GraphEvidenceArtifactV1::Edge {
                logical_id: resolved.logical_id,
                kind: resolved.artifact_kind,
                body: resolved.artifact_body,
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
        self.ensure_open()?;
        let binding = {
            let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
            let connection = connection.as_ref().ok_or(EngineError::Closing)?;
            frozen_read::authenticate(connection, frozen)?
        };
        frozen.context.view.reject_existence_relaxation_on_search()?;
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
            .dispatch(ReaderRequest::VectorStage { request, respond })
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
    ) -> Result<(SearchResult, GraphFrontierStats, Option<SearchExpandResult>), EngineError> {
        self.ensure_open()?;
        // 0.8.18 Slice 5 (#5 vector-equivalence probe, R-VEQ-4) — the SINGLE
        // vector-dependent choke point. If the open-time self-check found a
        // divergence beyond the D4 floor, refuse EVERY vector-dependent arm
        // (search / search_expand / explain-rerank / graph-arm all funnel here)
        // BEFORE any embedding / vector SQL / graph seeding / CE rerank — no
        // silent partial results. The text-only/FTS-only path
        // (`search_text_only`) does NOT route through here, so FTS stays
        // serviceable in degraded mode.
        let is_frozen = frozen_binding.is_some();
        let dense_disabled_reason =
            self.dense_disabled.load(Ordering::Acquire).then(|| {
                self.dense_disabled_reason.lock().ok().and_then(|g| g.clone()).unwrap_or_else(
                    || "open-time #5 vector-equivalence self-check failed".to_string(),
                )
            });
        if !is_frozen {
            if let Some(reason) = dense_disabled_reason.as_ref() {
                self.vector_equivalence_refusals.fetch_add(1, Ordering::Relaxed);
                return Err(EngineError::VectorEquivalenceMismatch { reason: reason.clone() });
            }
            if query.trim().is_empty() {
                return Err(EngineError::WriteValidation);
            }
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
        let compiled = (!is_frozen).then(|| compile_text_query(query));
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
        let raw_query_vector = (!is_frozen)
            .then(|| self.runtime_embedder.as_ref().and_then(|embedder| embedder.embed(query).ok()))
            .flatten();
        let query_vector_bin = match raw_query_vector.as_ref() {
            Some(vector) if identity_requires_mean_centering(&self.runtime_embedder_identity) => {
                let pinned = {
                    let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
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
        let query_vector = raw_query_vector.and_then(|vector| serde_json::to_string(&vector).ok());
        let frozen_query_runtime = is_frozen.then(|| {
            Box::new(FrozenQueryRuntime::new(
                self.runtime_embedder.clone(),
                self.runtime_embedder_identity.clone(),
                dense_disabled_reason,
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
        let request = ReaderRequest::Search(Box::new(SearchReaderRequest {
            work: SearchReaderWork::hybrid(
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
                } else if self.dense_disabled.load(Ordering::Acquire) {
                    ProjectionRuntimeStateV1::Refused
                } else {
                    ProjectionRuntimeStateV1::Usable
                },
                view,
                frozen_binding,
                frozen_query_runtime,
                expand_depth,
            ),
            respond: response_tx,
        }));
        if self.reader_pool.dispatch(request).is_err() {
            return Err(EngineError::Closing);
        }
        let search_result = response_rx.recv().map_err(|_| EngineError::Storage)?;
        let (cursor, soft_fallback, results, graph_stats, explanation, expanded) =
            match search_result {
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
                    return Err(EngineError::RerankerDevicePolicy(error));
                }
                Err(SearchReaderError::FrozenRead(error)) => {
                    return Err(EngineError::FrozenRead(error));
                }
                Err(SearchReaderError::VectorEquivalenceMismatch(reason)) => {
                    self.vector_equivalence_refusals.fetch_add(1, Ordering::Relaxed);
                    return Err(EngineError::VectorEquivalenceMismatch { reason });
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

        // The worker (`read_search_in_tx`) has no embedder identity; fill the
        // trace's `embedder_id` here, where `self.runtime_embedder_identity` is in
        // scope. Only on the explain path (`explanation` is `Some`).
        let explanation = explanation.map(|mut exp| {
            let id = &self.runtime_embedder_identity;
            exp.trace.embedder_id = format!("{}@{} (dim={})", id.name, id.revision, id.dimension);
            exp
        });

        Ok((
            SearchResult { projection_cursor: cursor, soft_fallback, results, explanation },
            graph_stats,
            expanded,
        ))
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
    /// **Scope: the VALIDITY axis only.** `include_out_of_window` and
    /// `valid_as_of` are honoured; the EXISTENCE flags (`include_superseded`,
    /// `include_inactive`) are **refused** with
    /// [`EngineError::InvalidArgument`] rather than silently ignored. Relaxing
    /// `superseded_at IS NULL` on a retrieval path would resurrect the stale-body
    /// leak the Slice-15 fix-1 review closed, and search hydrates from projection
    /// indexes (`search_index`, `vector_default`) that are not version-complete —
    /// so "include superseded" has no truthful answer here. Refusing says that;
    /// ignoring would be the dead surface this fix exists to remove.
    ///
    /// Governed surface: PROPOSED / NOT SIGNED (0.8.20 Slice 15b fix-2).
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
    /// Validity axis only; existence flags are refused. See
    /// [`search_view`][Engine::search_view].
    ///
    /// Governed surface: PROPOSED / NOT SIGNED (0.8.20 Slice 15b fix-2).
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
        )
    }

    /// G10 — hybrid `search` with an optional closed [`SearchFilter`]. `None`
    /// (or an all-`None` filter) is the unfiltered path whose phase-1 SQL is
    /// byte-identical to 0.7.2. The filter prunes the vector branch in the
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
        // byte-identical to 0.7.2 on the `None`/all-`None` path. `SearchFilter`
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
                    filter.to_search_filter().map(|mut lo| {
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
        )
    }

    /// 0.8.18 Slice 5 (#5 vector-equivalence probe, R-VEQ-4) — the explicit
    /// **text-only / FTS-only** search path. It does NOT embed the query and does
    /// NOT route through the vector-dependent choke point
    /// [`search_inner_with_stats`][Engine::search_inner_with_stats], so it NEVER
    /// raises [`EngineError::VectorEquivalenceMismatch`] and stays serviceable when
    /// the engine opened in the degraded `dense_disabled` state (the D2 "keep FTS
    /// servable" contract; codex R2 U1-2). Results come from the node- and
    /// edge-body FTS branches only — no vector recall, no CE rerank, no graph arm.
    /// Available regardless of degraded state; when dense is healthy it is simply a
    /// text-only view of the same corpus. Matching node- and edge-body
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
    /// under an explicit [`ReadView`]. Same validity-axis-only scope, and the same
    /// typed refusal of the existence flags, as [`search_view`][Engine::search_view].
    ///
    /// Governed surface: PROPOSED / NOT SIGNED (0.8.20 Slice 15b fix-2).
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
        let limit = validate_search_result_limit(limit)?;
        self.ensure_open()?;
        view.reject_existence_relaxation_on_search()?;
        if query.trim().is_empty() {
            return Err(EngineError::WriteValidation);
        }
        let compiled = compile_text_query(query);
        let candidate_limit =
            self.projection_runtime.shared.search_limit_override.load(Ordering::SeqCst).max(limit);
        let (response_tx, response_rx) = mpsc::sync_channel::<ReaderResponse>(1);
        // This explicit marker distinguishes direct text-only search from a hybrid
        // request whose embedder yields no vector. Only the direct path gets the
        // fixed node candidate bound before node/edge body deduplication and RRF.
        let request = ReaderRequest::Search(Box::new(SearchReaderRequest {
            work: SearchReaderWork::text_only(compiled, query, limit, candidate_limit, *view),
            respond: response_tx,
        }));
        if self.reader_pool.dispatch(request).is_err() {
            return Err(EngineError::Closing);
        }
        let search_result = response_rx.recv().map_err(|_| EngineError::Storage)?;
        let (cursor, soft_fallback, results, _graph_stats, explanation, _expanded) =
            match search_result {
                Ok(result) => result,
                // fix-3 (codex §9 [P2]) — carry the reader-snapshot validation verdict
                // through: an undeclared `filterable` attribute is the EXISTING typed
                // `InvalidFilter`, never collapsed to `Storage`. (This path takes
                // `filter = None`, so it never fires here, but the match stays total.)
                Err(SearchReaderError::InvalidFilter(reason)) => {
                    return Err(EngineError::InvalidFilter { reason });
                }
                Err(SearchReaderError::Evidence(error)) => return Err(error),
                Err(SearchReaderError::RerankerDevicePolicy(error)) => {
                    return Err(EngineError::RerankerDevicePolicy(error));
                }
                Err(SearchReaderError::FrozenRead(error)) => {
                    return Err(EngineError::FrozenRead(error));
                }
                Err(SearchReaderError::VectorEquivalenceMismatch(reason)) => {
                    return Err(EngineError::VectorEquivalenceMismatch { reason });
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
        Ok(SearchResult { projection_cursor: cursor, soft_fallback, results, explanation })
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
        self.ensure_open()?;
        view.reject_existence_relaxation_on_search()?;
        if query.trim().is_empty() {
            return Err(EngineError::WriteValidation);
        }

        let (response_tx, response_rx) = mpsc::sync_channel(1);
        let request = ReaderRequest::SearchProjectedText {
            query: query.to_string(),
            name: name.to_string(),
            filter: filter.map(Box::new),
            limit,
            view: *view,
            respond: response_tx,
        };
        if self.reader_pool.dispatch(request).is_err() {
            return Err(EngineError::Closing);
        }
        match response_rx.recv().map_err(|_| EngineError::Storage)? {
            Ok(result) => Ok(result),
            Err(SearchReaderError::Evidence(error)) => Err(error),
            Err(SearchReaderError::InvalidFilter(reason)) => {
                Err(EngineError::InvalidFilter { reason })
            }
            Err(SearchReaderError::RerankerDevicePolicy(error)) => {
                Err(EngineError::RerankerDevicePolicy(error))
            }
            Err(SearchReaderError::FrozenRead(error)) => Err(EngineError::FrozenRead(error)),
            Err(SearchReaderError::VectorEquivalenceMismatch(reason)) => {
                Err(EngineError::VectorEquivalenceMismatch { reason })
            }
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

    /// 0.8.18 Slice 5 (R-VEQ-6) — degraded-open observability accessor. `true` iff
    /// the open-time #5 self-check found a vector-equivalence divergence and every
    /// vector-dependent arm is refusing. Mirrors `OpenReport.dense_disabled`; read
    /// lock-free.
    #[must_use]
    pub fn dense_disabled(&self) -> bool {
        self.dense_disabled.load(Ordering::Acquire)
    }

    /// 0.8.18 Slice 5 (R-VEQ-6) — the human-readable reason for the degraded state
    /// (which representation tripped), or `None` when dense is healthy.
    #[must_use]
    pub fn dense_disabled_reason(&self) -> Option<String> {
        self.dense_disabled_reason.lock().ok().and_then(|g| g.clone())
    }

    /// 0.8.18 Slice 5 (R-VEQ-6) — telemetry counter: number of query-time
    /// vector-dependent-arm refusals raised because the engine opened degraded.
    /// Observable pre/post-query.
    #[must_use]
    pub fn vector_equivalence_refusal_count(&self) -> u64 {
        self.vector_equivalence_refusals.load(Ordering::Relaxed)
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
    ) -> Result<SearchResult, EngineError> {
        // fix-2: refuse an existence-relaxing view BEFORE any work (and before the
        // Started event), so the refusal is a pure argument error rather than a
        // half-emitted query lifecycle.
        view.reject_existence_relaxation_on_search()?;
        self.emit_event(lifecycle::Phase::Started, lifecycle::EventCategory::Search, None);
        let started = Instant::now();
        let outcome = self.search_inner(
            query,
            filter,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            explain,
            view,
            limit,
        );
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
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        bm25f_search_inner(connection, query, plan).map_err(|_| EngineError::Storage)
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
    ) -> Result<SearchExpandResult, EngineError> {
        self.search_expand_with_limit(query, filter, depth, DEFAULT_SEARCH_RESULT_LIMIT)
    }

    /// Hybrid search followed by graph expansion, with an explicit limit for
    /// the initial ranked `search_hits` set.
    pub fn search_expand_with_limit(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
        depth: u32,
        limit: usize,
    ) -> Result<SearchExpandResult, EngineError> {
        let limit = validate_search_result_limit(limit)?;
        self.ensure_open()?;
        if depth > 3 {
            return Err(EngineError::InvalidArgument {
                msg: format!("traversal depth {depth} exceeds the SDK ceiling of 3"),
            });
        }
        // Step 1: run the hybrid search to get initial hits (no CE reranking in expand).
        // 0.8.5: depth=0 → no rerank, so α/pool_n (0.3, 0) are inert here.
        let search_result =
            self.search_inner(query, filter, 0, false, 0.3, 0, false, ReadView::default(), limit)?;
        if search_result.results.is_empty() {
            return Ok(SearchExpandResult {
                search_hits: Vec::new(),
                expanded: Vec::new(),
                all_logical_ids: Vec::new(),
            });
        }
        // Step 2: dispatch to the reader pool to resolve logical_ids and run BFS.
        // depth=0 is forwarded to the reader so it can populate all_logical_ids
        // (the union of search-hit logical_ids), even with no expansion.
        let (response_tx, response_rx) = mpsc::sync_channel(1);
        let request = ReaderRequest::SearchExpand {
            search_hits: search_result.results,
            depth,
            view: ReadView::default(),
            filter: None,
            frozen_binding: None,
            respond: response_tx,
        };
        if self.reader_pool.dispatch(request).is_err() {
            return Err(EngineError::Closing);
        }
        match response_rx.recv().map_err(|_| EngineError::Storage)? {
            Ok(result) => Ok(result),
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
