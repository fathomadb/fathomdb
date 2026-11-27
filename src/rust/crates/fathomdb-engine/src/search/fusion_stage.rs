use super::fts_fast_path::{
    collect_edge_fts_candidates, collect_node_fts_candidates, count_dropped_edge_hits,
    hydrate_deferred_node_candidates, hydrate_deferred_text_identity, observe_slice30_guard,
    DeferredFusionHit, NodeFtsOutcome,
};
use super::graph_arm::{bfs_graph_arm_candidates, BfsGraphArmOutput};
use super::vector_arm::{
    detect_vector_soft_fallback, execute_vector_arm, historical_dense_incomplete, VectorArmOutput,
};
use super::{
    load_projection_cursor_for_search, FrozenQueryRuntime, SearchOriginCapture, SearchReaderError,
    SearchReaderWork,
};
use crate::dependency_closure;
use crate::embedding::search_query_vector;
use crate::errors::EngineError;
use crate::filter::{validate_filter_attributes_on_snapshot, SearchFilter};
use crate::frozen_read::{self, FrozenReadBinding, FrozenReadError, FrozenReadErrorReason};
use crate::fusion::{
    apply_recency_reweight_ranked, apply_record_importance_reweight,
    build_importance_confidence_maps, fuse_records, fuse_rrf, fuse_three_arms, FusionHit,
    FusionOwnership, RecordKey,
};
use crate::graph_expand::{search_expand_on_snapshot, SearchExpandResult};
use crate::mean::{identity_requires_mean_centering, read_pinned_mean_vec, subtract_mean};
use crate::projection_generation::{
    self, ProjectionGenerationOriginV1, ProjectionReadinessV1, ProjectionRuntimeStateV1,
};
use crate::reader_transaction::begin_attributed_reader_tx;
use crate::rerank::{try_rerank_ranked, try_rerank_ranked_with_cache, CeScoreCache};
use crate::search_types::{
    validate_search_result_limit, ArmDegradation, Explanation, GraphArmOptions, GraphArmSeedSource,
    GraphFrontierStats, GraphSeedMode, PerHitExplain, QueryTrace, SearchHit, SoftFallback,
    SoftFallbackBranch, StructuralDegradationCodeV1, StructuralInclusionStateV1,
    StructuralInclusionV1, StructuralLifecycleStateV1, StructuralProjectionOriginV1,
};
use crate::snippet::{SnippetOptions, SnippetV1};
use crate::snippet_query::{snippets_for_hits, FtsRows};
use crate::structural_state::structural_dependency_state;
use crate::temporal::ReadView;
#[cfg(any(test, feature = "test-hooks"))]
use crate::test_hooks::{frozen_after_validation_hook, reader_search_hook};
use crate::vector_spans::scored_extent;
use crate::wal_attribution::WalAttributionCollector;
use fathomdb_query::{compile_text_query_with, CompiledQuery, StopWordSet};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub(crate) struct NoEvidenceCapture;

// Ordinary search retains a zero-sized capture strategy: evidence state and
// provenance collection are absent unless the explicit evidence operation is used.
const _: () = assert!(std::mem::size_of::<NoEvidenceCapture>() == 0);

impl SearchOriginCapture for NoEvidenceCapture {
    type Output = (
        u64,
        Option<SoftFallback>,
        Vec<SearchHit>,
        GraphFrontierStats,
        Option<Explanation>,
        Option<SearchExpandResult>,
        Vec<ArmDegradation>,
        Option<Vec<SnippetV1>>,
    );

    fn finish(
        self,
        _connection: &Connection,
        _scope: &crate::scope::ScopePredicate,
        cursor: u64,
        soft_fallback: Option<SoftFallback>,
        results: Vec<FusionHit>,
        graph_stats: GraphFrontierStats,
        explanation: Option<Explanation>,
        _contributions: Option<Vec<crate::evidence::EvidenceContributionV1>>,
        expanded: Option<SearchExpandResult>,
        degradations: Vec<ArmDegradation>,
        snippets: Option<Vec<SnippetV1>>,
    ) -> Result<Self::Output, SearchReaderError> {
        Ok((
            cursor,
            soft_fallback,
            results.into_iter().map(|hit| hit.hit).collect(),
            graph_stats,
            explanation,
            expanded,
            degradations,
            snippets,
        ))
    }
}

fn structural_lifecycle_state(
    tx: &Connection,
    hit: &FusionHit,
) -> rusqlite::Result<StructuralLifecycleStateV1> {
    match hit.ownership {
        FusionOwnership::CanonicalEdge => return Ok(StructuralLifecycleStateV1::EdgeValid),
        FusionOwnership::OwnerlessNodeFts { .. } => {
            return Err(rusqlite::Error::QueryReturnedNoRows)
        }
        FusionOwnership::CanonicalNode => {}
    }
    let cursor = i64::try_from(hit.write_cursor).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let state: String =
        tx.query_row("SELECT state FROM canonical_nodes WHERE write_cursor=?1", [cursor], |row| {
            row.get(0)
        })?;
    match state.as_str() {
        "pending" => Ok(StructuralLifecycleStateV1::NodePending),
        "active" => Ok(StructuralLifecycleStateV1::NodeActive),
        "deleted" => Ok(StructuralLifecycleStateV1::NodeDeleted),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

#[cfg(test)]
fn rank_probe_arm(hits: impl IntoIterator<Item = (RecordKey, f64)>) -> Vec<serde_json::Value> {
    hits.into_iter()
        .enumerate()
        .map(|(index, (key, raw))| {
            let identity = match key {
                RecordKey::CanonicalNode(cursor) => serde_json::json!(["node", cursor]),
                RecordKey::CanonicalEdge(cursor) => serde_json::json!(["edge", cursor]),
                RecordKey::OwnerlessNodeFts(rowid) => {
                    serde_json::json!(["ownerless_node_fts", rowid])
                }
            };
            serde_json::json!([index + 1, identity, raw.to_bits()])
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn read_search_work_in_tx<C: SearchOriginCapture>(
    reader: &mut Connection,
    default_scope_cache: &mut crate::scope::DefaultScopeCache,
    work: SearchReaderWork,
    capture: C,
    scope_predicate: &crate::scope::ScopePredicate,
    stop_words: &StopWordSet,
    status_cache: &projection_generation::ProjectionGenerationStatusCache,
    status_slot: projection_generation::StatusCacheConnectionSlot,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> Result<C::Output, SearchReaderError> {
    #[cfg(any(test, feature = "slice30-observer"))]
    let _scope = crate::slice30_observer::enter_token(attribution, work.slice30_owner.clone());
    read_search_in_tx(
        reader,
        default_scope_cache,
        work.compiled.as_ref(),
        work.query_vector.as_deref(),
        work.query_vector_bin.as_deref(),
        work.result_limit,
        work.candidate_limit,
        work.direct_text_candidate_limit,
        work.filter.as_deref(),
        work.recency_enabled,
        work.importance_enabled,
        work.vector_stage_only,
        &work.raw_query,
        work.rerank_depth,
        work.use_graph_arm,
        work.alpha,
        work.pool_n,
        work.explain,
        work.projection_runtime_state,
        work.view,
        work.frozen_binding.as_deref(),
        work.frozen_query_runtime.as_deref(),
        work.expand_depth,
        scope_predicate,
        stop_words,
        work.snippet.as_ref(),
        work.graph_options.as_deref(),
        status_cache,
        status_slot,
        attribution,
        worker_idx,
        capture,
    )
}

#[allow(clippy::too_many_arguments)]
fn read_search_in_tx<C: SearchOriginCapture>(
    reader: &mut Connection,
    default_scope_cache: &mut crate::scope::DefaultScopeCache,
    compiled: Option<&CompiledQuery>,
    query_vector: Option<&str>,
    query_vector_bin: Option<&str>,
    final_limit: usize,
    candidate_limit: usize,
    direct_text_candidate_limit: Option<usize>,
    filter: Option<&SearchFilter>,
    recency_enabled: bool,
    importance_enabled: bool,
    vector_stage_only: bool,
    raw_query: &str,
    rerank_depth: usize,
    use_graph_arm: bool,
    alpha: f64,
    pool_n: usize,
    explain: bool,
    projection_runtime_state: ProjectionRuntimeStateV1,
    view: ReadView,
    frozen_binding: Option<&FrozenReadBinding>,
    frozen_query_runtime: Option<&FrozenQueryRuntime>,
    expand_depth: Option<u32>,
    scope: &crate::scope::ScopePredicate,
    stop_words: &StopWordSet,
    snippet: Option<&SnippetOptions>,
    graph_options: Option<&GraphArmOptions>,
    status_cache: &projection_generation::ProjectionGenerationStatusCache,
    status_slot: projection_generation::StatusCacheConnectionSlot,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
    mut capture: C,
) -> Result<C::Output, SearchReaderError> {
    let view = view.freeze();
    let now_param = view.now_param();
    #[cfg(any(test, feature = "test-hooks"))]
    reader_search_hook::fire(attribution);
    #[cfg(any(test, feature = "slice30-observer"))]
    let observation = crate::slice30_observer::span("reader_transaction");
    let observed_epoch = explain
        .then(|| status_cache.snapshot_epoch())
        .transpose()
        .map_err(SearchReaderError::Evidence)?;
    // Snapshot the monotonic visibility floor strictly BEFORE acquiring the SQLite
    // WAL read mark so concurrent readers opening valid older WAL snapshots across
    // advancing generations never trip a false regression, while any true rollback
    // (where a commit observed before `tx` started is no longer visible in `tx`)
    // still fails closed with `StateUnavailable`.
    let floor_before_tx = frozen_query_runtime
        .as_ref()
        .map(|runtime| runtime.observed_generation.load(Ordering::Acquire));
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)?;
    if let Some(expected) = frozen_binding {
        let runtime = frozen_query_runtime.as_ref().ok_or_else(|| {
            SearchReaderError::FrozenRead(FrozenReadError {
                reason: FrozenReadErrorReason::StateUnavailable,
                field_path: "/token".to_string(),
            })
        })?;
        let surface = if use_graph_arm || expand_depth.is_some() {
            frozen_read::ReadSurface::GraphOrEdge
        } else {
            frozen_read::ReadSurface::SearchNoGraph
        };
        let generation =
            frozen_read::validate_snapshot(&tx, &runtime.context, expected, scope, surface)
                .map_err(SearchReaderError::FrozenRead)?;
        let floor = floor_before_tx.unwrap_or(0);
        runtime.observed_generation.fetch_max(generation, Ordering::AcqRel);
        if generation < floor {
            return Err(SearchReaderError::FrozenRead(FrozenReadError {
                reason: FrozenReadErrorReason::StateUnavailable,
                field_path: "/token".to_string(),
            }));
        }
        #[cfg(any(test, feature = "test-hooks"))]
        frozen_after_validation_hook::fire(attribution);
    }
    let mut degradations = Vec::new();
    let owned_compiled;
    let owned_query_vector;
    let owned_query_vector_bin;
    let (compiled, query_vector, query_vector_bin) = if let Some(runtime) = frozen_query_runtime {
        validate_search_result_limit(final_limit)
            .map_err(|error| SearchReaderError::InvalidArgument(error.to_string()))?;
        if expand_depth.is_some_and(|depth| depth > 3) {
            return Err(SearchReaderError::InvalidArgument(format!(
                "traversal depth {} exceeds the SDK ceiling of 3",
                expand_depth.unwrap_or_default()
            )));
        }
        let max_ranking_control = u32::MAX as usize;
        if rerank_depth > max_ranking_control {
            return Err(SearchReaderError::InvalidArgument(format!(
                "rerank_depth {rerank_depth} exceeds the u32 maximum"
            )));
        }
        if pool_n > max_ranking_control {
            return Err(SearchReaderError::InvalidArgument(format!(
                "pool_n {pool_n} exceeds the u32 maximum"
            )));
        }
        if !alpha.is_finite() {
            return Err(SearchReaderError::InvalidArgument(
                "alpha must be a finite number".to_string(),
            ));
        }
        if let Some(opts) = graph_options {
            opts.validate().map_err(|error| match error {
                EngineError::InvalidArgument { msg } => SearchReaderError::InvalidArgument(msg),
                EngineError::InvalidFilter { reason } => SearchReaderError::InvalidFilter(reason),
                other => SearchReaderError::Evidence(other),
            })?;
        }
        if raw_query.trim().is_empty() || raw_query.as_bytes().contains(&0) {
            return Err(SearchReaderError::WriteValidation);
        }
        owned_compiled = compile_text_query_with(raw_query, stop_words);
        let raw_vector = search_query_vector(&runtime.embed_dispatch, raw_query, &mut degradations)
            .map_err(SearchReaderError::Evidence)?;
        owned_query_vector_bin = match raw_vector.as_ref() {
            Some(vector) if identity_requires_mean_centering(&runtime.embedder_identity) => {
                let pinned = read_pinned_mean_vec(&tx, runtime.embedder_identity.dimension)
                    .map_err(|_| SearchReaderError::Sqlite(rusqlite::Error::InvalidQuery))?;
                match pinned {
                    Some(mean) => serde_json::to_string(&subtract_mean(vector, &mean)).ok(),
                    None => serde_json::to_string(vector).ok(),
                }
            }
            Some(vector) => serde_json::to_string(vector).ok(),
            None => None,
        };
        owned_query_vector = raw_vector.and_then(|vector| serde_json::to_string(&vector).ok());
        (&owned_compiled, owned_query_vector.as_deref(), owned_query_vector_bin.as_deref())
    } else {
        (
            compiled.ok_or_else(|| SearchReaderError::Sqlite(rusqlite::Error::InvalidQuery))?,
            query_vector,
            query_vector_bin,
        )
    };
    let cursor = load_projection_cursor_for_search(&tx)?;
    #[cfg(test)]
    let rank_probe = crate::slice30_observer::rank_probe_route();
    #[cfg(any(test, feature = "slice30-observer"))]
    if crate::slice30_observer::is_recording() {
        let observation = crate::slice30_observer::span("observer_snapshot_capture");
        let authority = frozen_read::slice30_authority_for_test(&tx);
        observation.snapshot_result(&authority, &view, cursor);
    }
    if let Some(filter) = filter {
        validate_filter_attributes_on_snapshot(&tx, filter)?;
    }
    let has_source_dependencies = dependency_closure::has_source_dependencies(&tx, scope)?;
    let read_dependency_eligibility = |alias: &str, now_idx: usize| {
        if has_source_dependencies {
            dependency_closure::read_eligibility_sql(
                alias,
                view.view.include_superseded,
                view.view.include_inactive,
                view.view.include_out_of_window,
                now_idx,
            )
        } else {
            String::new()
        }
    };
    let history_incomplete = if (view.view.include_superseded || view.view.include_inactive)
        && direct_text_candidate_limit.is_none()
    {
        query_vector.is_none() || historical_dense_incomplete(&tx, scope, view)?
    } else {
        false
    };
    let vector_eligibility_degraded = observe_slice30_guard(
        "vector_arm_requires_fallback",
        "history_incomplete",
        true,
        history_incomplete,
    ) || observe_slice30_guard(
        "vector_arm_requires_fallback",
        "vector_unavailable",
        false,
        query_vector.is_some(),
    ) && dependency_closure::vector_arm_requires_fallback(
        &tx,
        has_source_dependencies,
        view.view.include_superseded,
        view.view.include_inactive,
        view.view.include_out_of_window,
        view.edge_now(),
        scope,
    )?;
    let capture_contributions = capture.requires_contributions();
    let track_contributions = explain || capture_contributions;
    // Per-connection `(data_version, total_changes)`-cached probe reused by both
    // the `vec0` Phase-1 unscoped fast path and FTS deferred identity hydration.
    // Gated behind cheap booleans (`is_unscoped && filter.is_none() && query_vector.is_some()`)
    // and fail-closed on any SQLite error.
    let store_only_default_scope =
        if scope.is_unscoped() && filter.is_none() && query_vector.is_some() {
            default_scope_cache.is_only_default_scope(&tx).unwrap_or(false)
        } else {
            false
        };
    let VectorArmOutput { hits: vector_results, cosines: vector_cosines } = execute_vector_arm(
        &tx,
        query_vector,
        query_vector_bin,
        vector_eligibility_degraded,
        final_limit,
        candidate_limit,
        filter,
        view,
        now_param,
        scope,
        store_only_default_scope,
        &read_dependency_eligibility,
        explain,
    )?;
    let exp_vector_cosines = explain.then_some(vector_cosines);
    let original_vector_keys =
        snippet.map(|_| vector_results.iter().map(FusionHit::key).collect::<HashSet<RecordKey>>());
    let vector_rows_visible = !vector_results.is_empty();
    let has_vector_kinds = if query_vector.is_some() && !vector_rows_visible {
        tx.prepare_cached("SELECT EXISTS(SELECT 1 FROM _fathomdb_vector_kinds LIMIT 1)")?
            .query_row([], |row| row.get::<_, bool>(0))?
    } else {
        true
    };
    let soft_fallback = detect_vector_soft_fallback(
        &tx,
        compiled,
        query_vector,
        vector_eligibility_degraded,
        vector_rows_visible,
        has_vector_kinds,
        scope,
    )?;
    let mut fts_rows = FtsRows::new();
    let NodeFtsOutcome {
        candidates: text_candidates,
        deferred_node_candidates,
        deferred_text_identity,
    } = collect_node_fts_candidates(
        &tx,
        compiled,
        query_vector,
        final_limit,
        direct_text_candidate_limit,
        filter,
        recency_enabled,
        importance_enabled,
        vector_stage_only,
        rerank_depth,
        pool_n,
        use_graph_arm,
        track_contributions,
        has_vector_kinds,
        vector_rows_visible,
        view,
        now_param,
        has_source_dependencies,
        &read_dependency_eligibility,
        scope,
        store_only_default_scope,
        snippet,
        &mut fts_rows,
    )?;
    let mut text_results: Vec<FusionHit> = text_candidates;
    let edge_candidates = collect_edge_fts_candidates(
        &tx,
        compiled,
        view,
        filter,
        &read_dependency_eligibility,
        scope,
        snippet.is_some(),
        &mut fts_rows,
    )?;
    let dropped_edge_hits = count_dropped_edge_hits(&tx, compiled, view, filter, scope, explain)?;
    text_results.extend(edge_candidates);
    #[cfg(test)]
    if rank_probe.is_some() {
        let vector = rank_probe_arm(vector_results.iter().map(|hit| (hit.key(), hit.score)));
        let text = if let Some(candidates) = &deferred_node_candidates {
            crate::slice30_observer::record_rank_probe(
                "s50_ranking_route",
                serde_json::json!("winner_hydration"),
            );
            rank_probe_arm(
                candidates
                    .iter()
                    .map(|hit| (hit.ownership.record_key(hit.write_cursor), hit.score)),
            )
        } else {
            rank_probe_arm(text_results.iter().map(|hit| (hit.key(), hit.score)))
        };
        crate::slice30_observer::record_rank_probe(
            "s50_rank_inputs",
            serde_json::json!({"vector": vector, "text": text}),
        );
    }

    let mut graph_stats = GraphFrontierStats::default();
    let record_rank_map = |hits: &[FusionHit]| -> HashMap<RecordKey, u32> {
        let mut m: HashMap<RecordKey, u32> = HashMap::new();
        for (i, h) in hits.iter().enumerate() {
            m.entry(h.key()).or_insert(i as u32);
        }
        m
    };
    let record_score_map = |hits: &[FusionHit]| -> HashMap<RecordKey, f64> {
        hits.iter().map(|h| (h.key(), h.score)).collect()
    };

    let (exp_vector_ranks, exp_text_ranks, exp_vector_n, exp_text_n) = if track_contributions {
        (
            Some(record_rank_map(&vector_results)),
            Some(record_rank_map(&text_results)),
            vector_results.len() as u32,
            text_results.len() as u32,
        )
    } else {
        (None, None, 0, 0)
    };
    let mut exp_graph_ranks: Option<HashMap<RecordKey, u32>> = None;
    let mut exp_graph_origins: Option<HashMap<u64, (GraphArmSeedSource, u32)>> = None;
    let mut exp_fused_scores: Option<HashMap<RecordKey, f64>> = None;
    let mut exp_graph_n: u32 = 0;
    let mut exp_importance: Option<HashMap<RecordKey, f64>> = None;
    let mut exp_confidence: Option<HashMap<RecordKey, f64>> = None;
    let mut deferred_graph_edge_confidence: HashMap<u64, f64> = HashMap::new();

    #[cfg(any(test, feature = "slice30-observer"))]
    let mut thin_identity_observed = false;
    let mut results = {
        #[cfg(any(test, feature = "slice30-observer"))]
        let mut fusion_observation = Some(crate::slice30_observer::span("fusion_final_ranking"));
        let mut results = if vector_stage_only {
            vector_results
        } else if let Some(text) = deferred_node_candidates {
            let fused = fuse_records(
                vector_results.into_iter().map(DeferredFusionHit::Hydrated).collect(),
                text.into_iter().map(DeferredFusionHit::NodeFts).collect(),
                Vec::new(),
            )
            .map_err(SearchReaderError::Evidence)?;
            #[cfg(any(test, feature = "slice30-observer"))]
            let identity_observation = {
                if let Some(observation) = fusion_observation.take() {
                    observation.counted_success(fused.len().min(final_limit));
                }
                thin_identity_observed = true;
                crate::slice30_observer::span("deferred_identity_hydration")
            };
            let hydrated = hydrate_deferred_node_candidates(&tx, fused, final_limit)?;
            #[cfg(any(test, feature = "slice30-observer"))]
            identity_observation.success();
            hydrated
        } else if use_graph_arm {
            let default_graph_options = GraphArmOptions::default();
            let active_graph_options = graph_options.unwrap_or(&default_graph_options);
            let mut ce_cache = CeScoreCache::default();
            let two_arm_fused =
                fuse_rrf(vector_results, text_results).map_err(SearchReaderError::Evidence)?;
            let fused_seed_owned = if matches!(
                active_graph_options.graph_seed,
                GraphSeedMode::FusedTopK | GraphSeedMode::Both
            ) && rerank_depth > 0
            {
                Some(
                    try_rerank_ranked_with_cache(
                        raw_query,
                        two_arm_fused.clone(),
                        rerank_depth,
                        alpha,
                        pool_n,
                        &mut ce_cache,
                    )
                    .map_err(SearchReaderError::RerankerDevicePolicy)?,
                )
            } else {
                None
            };
            let fused_seed_ranking: &[FusionHit] =
                match (active_graph_options.graph_seed, fused_seed_owned.as_deref()) {
                    (GraphSeedMode::GraphFts, _) => &[],
                    (_, Some(reranked)) => reranked,
                    (_, None) => &two_arm_fused,
                };
            let BfsGraphArmOutput {
                candidates: graph_candidates,
                stats,
                edge_confidence_by_cursor: graph_edge_confidence,
                graph_origin_by_cursor: graph_origins,
            } = bfs_graph_arm_candidates(
                &tx,
                &two_arm_fused,
                fused_seed_ranking,
                compiled.match_expression.as_str(),
                3,
                50,
                view,
                filter,
                active_graph_options,
                explain,
                scope,
                &mut capture,
            )?;
            graph_stats = stats;
            if track_contributions {
                exp_graph_ranks = Some(record_rank_map(&graph_candidates));
            }
            if explain {
                exp_graph_origins = Some(graph_origins);
                exp_graph_n = graph_candidates.len() as u32;
            }
            let fused = apply_recency_reweight_ranked(
                fuse_three_arms(two_arm_fused, vec![], graph_candidates)
                    .map_err(SearchReaderError::Evidence)?,
                recency_enabled,
            );
            let fused = if importance_enabled {
                let (imp_map, mut conf_map) = build_importance_confidence_maps(&tx, &fused)?;
                for (cursor, conf) in &graph_edge_confidence {
                    conf_map.entry(RecordKey::CanonicalNode(*cursor)).or_insert(*conf);
                }
                let fused = apply_record_importance_reweight(fused, &imp_map, &conf_map, true);
                if track_contributions {
                    exp_importance = Some(imp_map);
                    exp_confidence = Some(conf_map);
                }
                fused
            } else {
                if track_contributions {
                    deferred_graph_edge_confidence = graph_edge_confidence;
                }
                fused
            };
            if track_contributions {
                exp_fused_scores = Some(record_score_map(&fused));
            }
            try_rerank_ranked_with_cache(
                raw_query,
                fused,
                rerank_depth,
                alpha,
                pool_n,
                &mut ce_cache,
            )
            .map_err(SearchReaderError::RerankerDevicePolicy)?
        } else {
            let fused = apply_recency_reweight_ranked(
                fuse_rrf(vector_results, text_results).map_err(SearchReaderError::Evidence)?,
                recency_enabled,
            );
            let fused = if importance_enabled {
                let (imp_map, conf_map) = build_importance_confidence_maps(&tx, &fused)?;
                let fused = apply_record_importance_reweight(fused, &imp_map, &conf_map, true);
                if track_contributions {
                    exp_importance = Some(imp_map);
                    exp_confidence = Some(conf_map);
                }
                fused
            } else {
                fused
            };
            if track_contributions {
                exp_fused_scores = Some(record_score_map(&fused));
            }
            try_rerank_ranked(raw_query, fused, rerank_depth, alpha, pool_n)
                .map_err(SearchReaderError::RerankerDevicePolicy)?
        };

        results.truncate(final_limit);
        #[cfg(any(test, feature = "slice30-observer"))]
        if let Some(observation) = fusion_observation.as_ref() {
            observation.counted_success(results.len());
        }
        results
    };

    if !importance_enabled && track_contributions && !vector_stage_only {
        let (imp_map, mut conf_map) = build_importance_confidence_maps(&tx, &results)?;
        for (cursor, conf) in &deferred_graph_edge_confidence {
            conf_map.entry(RecordKey::CanonicalNode(*cursor)).or_insert(*conf);
        }
        exp_importance = Some(imp_map);
        exp_confidence = Some(conf_map);
    }

    if deferred_text_identity {
        #[cfg(any(test, feature = "slice30-observer"))]
        let identity_observation = crate::slice30_observer::span("deferred_identity_hydration");
        hydrate_deferred_text_identity(&tx, &mut results, scope)?;
        #[cfg(any(test, feature = "slice30-observer"))]
        identity_observation.success();
    } else {
        #[cfg(any(test, feature = "slice30-observer"))]
        if !thin_identity_observed {
            crate::slice30_observer::not_run("deferred_identity_hydration", "not_deferred");
        }
    }

    let projection_status = observed_epoch
        .map(|epoch| {
            status_cache
                .get_or_compute(
                    &tx,
                    status_slot,
                    epoch,
                    scope,
                    projection_runtime_state,
                    view.edge_now(),
                    false,
                )
                .map(|(status, _)| status)
        })
        .transpose()
        .map_err(SearchReaderError::Evidence)?;

    let explanation = if explain {
        let per_hit: Vec<PerHitExplain> = results
            .iter()
            .map(|h| -> rusqlite::Result<PerHitExplain> {
                let (graph_seed_source, graph_hop_depth) =
                    if h.branch == SoftFallbackBranch::GraphArm {
                        exp_graph_origins
                            .as_ref()
                            .and_then(|m| m.get(&h.write_cursor).copied())
                            .map_or((None, None), |(source, depth)| (Some(source), Some(depth)))
                    } else {
                        (None, None)
                    };
                Ok(PerHitExplain {
                    id: h.write_cursor,
                    arm: h.branch,
                    vector_rank: exp_vector_ranks.as_ref().and_then(|m| m.get(&h.key()).copied()),
                    text_rank: exp_text_ranks.as_ref().and_then(|m| m.get(&h.key()).copied()),
                    graph_rank: exp_graph_ranks.as_ref().and_then(|m| m.get(&h.key()).copied()),
                    fused_score: exp_fused_scores
                        .as_ref()
                        .and_then(|m| m.get(&h.key()).copied())
                        .unwrap_or(h.score),
                    ce_score: h.ce_score,
                    blended: h.score,
                    importance: exp_importance.as_ref().and_then(|m| m.get(&h.key()).copied()),
                    confidence: exp_confidence.as_ref().and_then(|m| m.get(&h.key()).copied()),
                    graph_seed_source,
                    graph_hop_depth,
                    vector_cosine: exp_vector_cosines
                        .as_ref()
                        .and_then(|m| m.get(&h.key()).copied().flatten()),
                    structural: {
                        let projection_origin = match h.branch {
                            SoftFallbackBranch::Vector => {
                                StructuralProjectionOriginV1::CurrentDenseGeneration
                            }
                            SoftFallbackBranch::GraphArm => {
                                StructuralProjectionOriginV1::GraphTraversal
                            }
                            SoftFallbackBranch::Text | SoftFallbackBranch::TextEdge => {
                                StructuralProjectionOriginV1::SynchronousBodyFts
                            }
                        };
                        let lifecycle_state = structural_lifecycle_state(&tx, h)?;
                        let mut degradation_codes = match (&soft_fallback, h.branch) {
                            (Some(_), SoftFallbackBranch::Text) => {
                                vec![StructuralDegradationCodeV1::SoftFallbackText]
                            }
                            (Some(_), SoftFallbackBranch::TextEdge) => {
                                vec![StructuralDegradationCodeV1::SoftFallbackTextEdge]
                            }
                            _ => Vec::new(),
                        };
                        if let Some(status) = projection_status.as_ref() {
                            let code = if status.origin
                                == ProjectionGenerationOriginV1::LegacyUnverified
                            {
                                Some(StructuralDegradationCodeV1::ProjectionLegacyUnverified)
                            } else {
                                match status.readiness {
                                    ProjectionReadinessV1::Blocked => {
                                        Some(StructuralDegradationCodeV1::ProjectionBlocked)
                                    }
                                    _ => None,
                                }
                            };
                            if let Some(code) = code {
                                degradation_codes.push(code);
                            }
                        }
                        if graph_stats.bound_reached {
                            degradation_codes.push(StructuralDegradationCodeV1::GraphBoundReached);
                        }
                        degradation_codes.sort_unstable();
                        degradation_codes.dedup();
                        StructuralInclusionV1 {
                            schema_version: 1,
                            inclusion_state: if degradation_codes.is_empty() {
                                StructuralInclusionStateV1::Included
                            } else {
                                StructuralInclusionStateV1::Degraded
                            },
                            projection_origin,
                            dependency_state: structural_dependency_state(
                                &tx,
                                h.write_cursor,
                                view.edge_now(),
                            )?,
                            lifecycle_state,
                            degradation_codes,
                        }
                    },
                })
            })
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let ce_active = rerank_depth > 0 && per_hit.iter().any(|p| p.ce_score.is_some());
        Some(Explanation {
            trace: QueryTrace {
                query_chars: raw_query.chars().count() as u32,
                k: final_limit as u32,
                rerank_depth: rerank_depth as u32,
                pool_n: pool_n as u32,
                alpha,
                use_graph_arm,
                recency: recency_enabled,
                embedder_id: String::new(),
                ce_active,
                vector_hits: exp_vector_n,
                text_hits: exp_text_n,
                graph_hits: exp_graph_n,
                dropped_edge_hits,
                stop_words_sha256: stop_words.sha256().to_string(),
            },
            per_hit,
            correlation_id: String::new(),
        })
    } else {
        None
    };

    let contributions = if capture_contributions {
        let mut out = Vec::with_capacity(results.len());
        for h in &results {
            let fused_score =
                exp_fused_scores.as_ref().and_then(|m| m.get(&h.key()).copied()).unwrap_or(h.score);
            let importance = exp_importance.as_ref().and_then(|m| m.get(&h.key()).copied());
            let confidence = exp_confidence.as_ref().and_then(|m| m.get(&h.key()).copied());
            let values = [Some(fused_score), h.ce_score, Some(h.score), importance, confidence];
            if values.into_iter().flatten().any(|value| !value.is_finite()) {
                return Err(SearchReaderError::Evidence(
                    crate::evidence::EvidenceErrorV1::new(
                        crate::evidence::EvidenceErrorReasonV1::EvidenceCorrupt,
                        "/explanation/perHit",
                    )
                    .into(),
                ));
            }
            out.push(crate::evidence::EvidenceContributionV1 {
                schema_version: 1,
                vector_rank: exp_vector_ranks.as_ref().and_then(|m| m.get(&h.key()).copied()),
                text_rank: exp_text_ranks.as_ref().and_then(|m| m.get(&h.key()).copied()),
                graph_rank: exp_graph_ranks.as_ref().and_then(|m| m.get(&h.key()).copied()),
                fused_score,
                ce_score: h.ce_score,
                blended_score: h.score,
                importance,
                confidence,
            });
        }
        Some(out)
    } else {
        None
    };

    let expanded = expand_depth
        .map(|depth| search_expand_on_snapshot(&tx, &results, depth, view, filter, scope))
        .transpose()?;
    let snippets = snippet
        .map(|options| {
            let mut snippets = snippets_for_hits(
                &tx,
                &results,
                &fts_rows,
                &compiled.match_expression,
                &compiled.terms,
                options,
            )?;
            if let Some(keys) = &original_vector_keys {
                for (hit, snippet) in results.iter().zip(&mut snippets) {
                    if keys.contains(&hit.key()) {
                        snippet.scored_bytes = scored_extent(&tx, hit.key(), &hit.body)
                            .map_err(SearchReaderError::Evidence)?;
                    }
                }
            }
            Ok::<_, SearchReaderError>(snippets)
        })
        .transpose()?;
    if explanation.as_ref().is_some_and(|value| value.per_hit.len() != results.len())
        || snippets.as_ref().is_some_and(|value| value.len() != results.len())
    {
        return Err(rusqlite::Error::InvalidQuery.into());
    }
    let output = capture.finish(
        &tx,
        scope,
        cursor,
        soft_fallback,
        results,
        graph_stats,
        explanation,
        contributions,
        expanded,
        degradations,
        snippets,
    )?;
    tx.commit()?;
    #[cfg(all(feature = "slice50-measurement", any(test, feature = "slice30-observer")))]
    crate::slice30_observer::finish_slice50_work();
    #[cfg(any(test, feature = "slice30-observer"))]
    observation.success();
    Ok(output)
}
