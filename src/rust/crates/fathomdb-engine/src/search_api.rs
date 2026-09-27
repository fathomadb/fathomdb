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
        let work = SearchReaderWork {
            compiled: None,
            query_vector: None,
            query_vector_bin: None,
            result_limit,
            candidate_limit,
            direct_text_candidate_limit: None,
            filter: Some(Box::new(request.context.context.eligibility.clone())),
            recency_enabled: self
                .projection_runtime
                .shared
                .recency_reweight_enabled
                .load(Ordering::SeqCst),
            importance_enabled: self
                .projection_runtime
                .shared
                .importance_reweight_enabled
                .load(Ordering::SeqCst),
            vector_stage_only: self
                .projection_runtime
                .shared
                .vector_stage_only_for_test
                .load(Ordering::SeqCst),
            raw_query: Box::from(request.query.as_str()),
            rerank_depth: request.rerank_depth as usize,
            use_graph_arm: request.use_graph_arm,
            alpha: request.alpha,
            pool_n: request.pool_n as usize,
            explain: true,
            projection_runtime_state: if self.runtime_embedder.is_none() {
                ProjectionRuntimeStateV1::Absent
            } else if self.dense_disabled.load(Ordering::Acquire) {
                ProjectionRuntimeStateV1::Refused
            } else {
                ProjectionRuntimeStateV1::Usable
            },
            view: request.context.context.view,
            frozen_binding: Some(Box::new(binding)),
            frozen_query_runtime: Some(Box::new(FrozenQueryRuntime {
                embedder: self.runtime_embedder.clone(),
                embedder_identity: self.runtime_embedder_identity.clone(),
                dense_disabled_reason,
                observed_generation: Arc::clone(&self.read_visibility_generation),
            })),
            expand_depth: None,
        };
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
    pub(crate) fn search_inner_with_frozen_binding_and_stats(
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
    pub(crate) fn search_inner_with_frozen_binding_and_expansion(
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
            Box::new(FrozenQueryRuntime {
                embedder: self.runtime_embedder.clone(),
                embedder_identity: self.runtime_embedder_identity.clone(),
                dense_disabled_reason,
                observed_generation: Arc::clone(&self.read_visibility_generation),
            })
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
            work: SearchReaderWork {
                compiled,
                query_vector,
                query_vector_bin,
                result_limit,
                candidate_limit,
                direct_text_candidate_limit: None,
                filter: filter.map(Box::new),
                recency_enabled,
                importance_enabled,
                vector_stage_only,
                raw_query: Box::from(query), // FIX-4: Box<str> (16B) not String (24B)
                rerank_depth,
                use_graph_arm,
                alpha,
                pool_n,
                explain,
                projection_runtime_state: if self.runtime_embedder.is_none() {
                    ProjectionRuntimeStateV1::Absent
                } else if self.dense_disabled.load(Ordering::Acquire) {
                    ProjectionRuntimeStateV1::Refused
                } else {
                    ProjectionRuntimeStateV1::Usable
                },
                view,
                frozen_binding: frozen_binding.map(Box::new),
                frozen_query_runtime,
                expand_depth,
            },
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
