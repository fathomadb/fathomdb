use super::*;

// ===== Slice 20 (G5/G6) — graph traversal ==============================
//
// `graphNeighbors` (G5) — bounded BFS from a root node, returning the
// reachable `NodeRecord`s within `depth` hops. `searchExpand` (G6)
// composes G1 search + G5 expansion with deduplication.

/// Slice 20 — one expanded node entry in `SearchExpandResult.expanded`.
/// `hopCount` is the BFS distance from the nearest search-hit root.
#[napi(object)]
pub struct ExpandedNode {
    pub node: NodeRecord,
    pub hop_count: u32,
}

/// Slice 20 (G6) — result of `searchExpand`.
///
/// `searchHits` — original RRF-scored results from the search step.
/// `expanded`   — nodes reachable from any hit within `depth` hops that are
///                NOT in `searchHits` (deduplication: search score wins).
/// `allLogicalIds` — deduplicated union of both sets.
#[napi(object)]
pub struct SearchExpandResult {
    pub search_hits: Vec<SearchHit>,
    pub expanded: Vec<ExpandedNode>,
    pub all_logical_ids: Vec<String>,
}

impl SearchExpandResult {
    pub(crate) fn from_rust(r: RustSearchExpandResult) -> Self {
        Self {
            search_hits: r.search_hits.iter().map(SearchHit::from_rust).collect(),
            expanded: r
                .expanded
                .into_iter()
                .map(|(node, hop_count)| ExpandedNode {
                    node: NodeRecord::from_rust(&node),
                    hop_count,
                })
                .collect(),
            all_logical_ids: r.all_logical_ids,
        }
    }
}

pub(crate) fn parse_direction_napi(direction: &str) -> Result<RustTraversalDirection> {
    match direction {
        "outgoing" => Ok(RustTraversalDirection::Outgoing),
        "incoming" => Ok(RustTraversalDirection::Incoming),
        "both" => Ok(RustTraversalDirection::Both),
        other => Err(typed_error(
            CODE_INVALID_ARGUMENT,
            format!("direction must be 'outgoing', 'incoming', or 'both'; got '{other}'"),
            JsonValue::Null,
        )),
    }
}

/// Slice 20 (G5) — bounded BFS from `logicalId` over `canonical_edges`.
///
/// `depth` must be 1–3; rejects depth > 3 with `InvalidArgumentError`.
/// `direction` is `"outgoing"`, `"incoming"`, or `"both"`.
/// Returns up to 50 `NodeRecord`s reachable within `depth` hops.
/// Edges with `t_invalid` in the past are not traversed.
#[napi(js_name = "graphNeighbors")]
pub async fn graph_neighbors(
    engine: &Engine,
    logical_id: String,
    depth: u32,
    direction: String,
    view: Option<ReadViewInput>,
) -> Result<Vec<NodeRecord>> {
    validate_ffi_string_napi(&logical_id)?;
    let dir = parse_direction_napi(&direction)?;
    let view = read_view_or_default(view);
    let inner = Arc::clone(&engine.inner);
    let nodes = call_engine(move || inner.graph_neighbors(&logical_id, depth, dir, &view)).await?;
    Ok(nodes.iter().map(NodeRecord::from_rust).collect())
}

/// 0.8.20 Slice 10b (R-20-NV) — nodes that crossed a validity boundary in
/// `(since, view-instant]`. `since` is INTEGER epoch SECONDS.
#[napi(js_name = "crossedBoundarySince")]
pub async fn crossed_boundary_since(
    engine: &Engine,
    since: i64,
    view: Option<ReadViewInput>,
) -> Result<Vec<BoundaryCrossing>> {
    let view = read_view_or_default(view);
    let inner = Arc::clone(&engine.inner);
    let rows = call_engine(move || inner.crossed_boundary_since(since, &view)).await?;
    Ok(rows.iter().map(BoundaryCrossing::from_rust).collect())
}

/// Slice 20 (G6) — FTS/vector search followed by bounded BFS expansion.
///
/// Runs `search(query, filter)` (G1), then expands each hit via
/// `graph_neighbors(depth, both)`. Nodes appearing in both sets appear
/// only in `searchHits` (deduplication: search score takes priority).
#[napi(js_name = "searchExpand")]
#[allow(clippy::too_many_arguments)]
pub async fn search_expand(
    engine: &Engine,
    query: String,
    depth: u32,
    source_type: Option<String>,
    kind: Option<String>,
    created_after: Option<i64>,
    status: Option<String>,
    search_limit: Option<u32>,
) -> Result<SearchExpandResult> {
    validate_ffi_string_napi(&query)?;
    let filter =
        if source_type.is_some() || kind.is_some() || created_after.is_some() || status.is_some() {
            // `#[non_exhaustive]` (0.8.20 Slice 15e fix-2): no out-of-crate struct
            // literal; build from `default()`. `attributes` stays engine-internal.
            let mut f = RustSearchFilter::default();
            f.source_type = source_type;
            f.kind = kind;
            f.created_after = created_after;
            f.status = status;
            Some(f)
        } else {
            None
        };
    let inner = Arc::clone(&engine.inner);
    let limit = search_limit.unwrap_or(10) as usize;
    let result =
        call_engine(move || inner.search_expand_with_limit(&query, filter, depth, limit)).await?;
    Ok(SearchExpandResult::from_rust(result))
}

#[napi(object)]
pub struct EvidenceSidecarEntryV1 {
    pub schema_version: u32,
    pub result_index: u32,
    pub artifact_revision_id: String,
    pub evidence_ref: String,
}

impl From<&RustEvidenceSidecarEntryV1> for EvidenceSidecarEntryV1 {
    fn from(value: &RustEvidenceSidecarEntryV1) -> Self {
        Self {
            schema_version: value.schema_version,
            result_index: value.result_index,
            artifact_revision_id: value.artifact_revision_id.clone(),
            evidence_ref: value.evidence_ref.as_str().to_string(),
        }
    }
}

#[napi(object)]
pub struct EvidenceSearchResultV1 {
    pub schema_version: u32,
    pub search_result: SearchResult,
    pub evidence: Vec<EvidenceSidecarEntryV1>,
}

impl From<RustEvidenceSearchResultV1> for EvidenceSearchResultV1 {
    fn from(value: RustEvidenceSearchResultV1) -> Self {
        Self {
            schema_version: value.schema_version,
            search_result: SearchResult::from_rust(value.search_result),
            evidence: value.evidence.iter().map(Into::into).collect(),
        }
    }
}

#[napi(object)]
pub struct EvidenceContributionV1 {
    pub schema_version: u32,
    pub vector_rank: Option<u32>,
    pub text_rank: Option<u32>,
    pub graph_rank: Option<u32>,
    pub fused_score: f64,
    pub ce_score: Option<f64>,
    pub blended_score: f64,
    pub importance: Option<f64>,
    pub confidence: Option<f64>,
}

impl From<RustEvidenceContributionV1> for EvidenceContributionV1 {
    fn from(value: RustEvidenceContributionV1) -> Self {
        Self {
            schema_version: value.schema_version,
            vector_rank: value.vector_rank,
            text_rank: value.text_rank,
            graph_rank: value.graph_rank,
            fused_score: value.fused_score,
            ce_score: value.ce_score,
            blended_score: value.blended_score,
            importance: value.importance,
            confidence: value.confidence,
        }
    }
}

#[napi(object)]
pub struct EvidenceGraphOriginV1 {
    pub kind: String,
    pub edge_artifact_revision_id: Option<String>,
    pub hop_count: Option<u32>,
}

#[napi(object)]
pub struct EvidenceProjectionOriginV1 {
    pub schema_version: u32,
    pub artifact_class: String,
    pub representative_arm: String,
    pub projection_generation_id: String,
    pub graph_origin: Option<EvidenceGraphOriginV1>,
}

impl From<RustEvidenceProjectionOriginV1> for EvidenceProjectionOriginV1 {
    fn from(value: RustEvidenceProjectionOriginV1) -> Self {
        let graph_origin = value.graph_origin.map(|origin| match origin {
            RustEvidenceGraphOriginV1::EdgeSeed { edge_artifact_revision_id } => {
                EvidenceGraphOriginV1 {
                    kind: "edge_seed".to_string(),
                    edge_artifact_revision_id: Some(edge_artifact_revision_id),
                    hop_count: None,
                }
            }
            RustEvidenceGraphOriginV1::Traversal { edge_artifact_revision_id, hop_count } => {
                EvidenceGraphOriginV1 {
                    kind: "traversal".to_string(),
                    edge_artifact_revision_id: Some(edge_artifact_revision_id),
                    hop_count: Some(hop_count),
                }
            }
        });
        Self {
            schema_version: value.schema_version,
            artifact_class: value.artifact_class.as_str().to_string(),
            representative_arm: value.representative_arm.as_str().to_string(),
            projection_generation_id: value.projection_generation_id.as_str().to_string(),
            graph_origin,
        }
    }
}

#[napi(object)]
pub struct EvidenceLocatorV1 {
    pub kind: String,
    pub start_inclusive: Option<String>,
    pub end_exclusive: Option<String>,
}

#[napi(object)]
pub struct EvidenceArtifactLifecycleV1 {
    pub kind: String,
    pub state: Option<String>,
    pub superseded: bool,
    pub valid_at_effective: Option<bool>,
}

#[napi(object)]
pub struct ResolvedEvidenceV1 {
    pub schema_version: u32,
    pub logical_id: Option<String>,
    pub artifact_revision_id: String,
    pub source_id: String,
    pub source_version_id: String,
    pub source_revision_id: String,
    pub locator: EvidenceLocatorV1,
    pub canonical_source_body: String,
    pub evidence_text: String,
    pub canonical_source_hash: String,
    pub effective_valid_at: i64,
    pub artifact_lifecycle: EvidenceArtifactLifecycleV1,
    pub source_lifecycle_state: String,
    pub projection_origin: EvidenceProjectionOriginV1,
    pub retrieval_contribution: EvidenceContributionV1,
    pub dependency: Option<SourceDependencyV1>,
}

impl From<RustResolvedEvidenceV1> for ResolvedEvidenceV1 {
    fn from(value: RustResolvedEvidenceV1) -> Self {
        let locator = match value.locator {
            SourceLocator::WholeBody => EvidenceLocatorV1 {
                kind: "whole_body".to_string(),
                start_inclusive: None,
                end_exclusive: None,
            },
            SourceLocator::Utf8Bytes { start_inclusive, end_exclusive } => EvidenceLocatorV1 {
                kind: "utf8_bytes".to_string(),
                start_inclusive: Some(start_inclusive.to_string()),
                end_exclusive: Some(end_exclusive.to_string()),
            },
        };
        let artifact_lifecycle = match value.artifact_lifecycle {
            RustEvidenceArtifactLifecycleV1::Node { state, superseded } => {
                EvidenceArtifactLifecycleV1 {
                    kind: "node".to_string(),
                    state: Some(state.as_str().to_string()),
                    superseded,
                    valid_at_effective: None,
                }
            }
            RustEvidenceArtifactLifecycleV1::Edge { superseded, valid_at_effective } => {
                EvidenceArtifactLifecycleV1 {
                    kind: "edge".to_string(),
                    state: None,
                    superseded,
                    valid_at_effective: Some(valid_at_effective),
                }
            }
        };
        Self {
            schema_version: value.schema_version,
            logical_id: value.logical_id,
            artifact_revision_id: value.artifact_revision_id,
            source_id: value.source_id,
            source_version_id: value.source_version_id,
            source_revision_id: value.source_revision_id,
            locator,
            canonical_source_body: value.canonical_source_body,
            evidence_text: value.evidence_text,
            canonical_source_hash: value.canonical_source_hash.digest_hex().to_string(),
            effective_valid_at: value.effective_valid_at,
            artifact_lifecycle,
            source_lifecycle_state: value.source_lifecycle_state.as_str().to_string(),
            projection_origin: value.projection_origin.into(),
            retrieval_contribution: value.retrieval_contribution.into(),
            dependency: value.dependency.map(Into::into),
        }
    }
}

/// 0.8.8 EXP-OBS (Slice 10) — query-level retrieval trace (mirror of engine
/// `QueryTrace`). napi maps snake_case → camelCase JS (`queryChars`,
/// `rerankDepth`, `useGraphArm`, `embedderId`, `ceActive`, `vectorHits`, …).
#[napi(object)]
pub struct QueryTrace {
    pub query_chars: u32,
    pub k: u32,
    pub rerank_depth: u32,
    pub pool_n: u32,
    pub alpha: f64,
    pub use_graph_arm: bool,
    pub recency: bool,
    pub embedder_id: String,
    pub ce_active: bool,
    pub vector_hits: u32,
    pub text_hits: u32,
    pub graph_hits: u32,
    pub dropped_edge_hits: u32,
}

impl QueryTrace {
    pub(crate) fn from_rust(t: &RustQueryTrace) -> Self {
        Self {
            query_chars: t.query_chars,
            k: t.k,
            rerank_depth: t.rerank_depth,
            pool_n: t.pool_n,
            alpha: t.alpha,
            use_graph_arm: t.use_graph_arm,
            recency: t.recency,
            embedder_id: t.embedder_id.clone(),
            ce_active: t.ce_active,
            vector_hits: t.vector_hits,
            text_hits: t.text_hits,
            graph_hits: t.graph_hits,
            dropped_edge_hits: t.dropped_edge_hits,
        }
    }
}

/// 0.8.8 EXP-OBS (Slice 10) — per-hit provenance + score breakdown (mirror of
/// engine `PerHitExplain`). `id` is the hit's engine-internal positional
/// `write_cursor` (the pre-0.8.19 `SearchHit.id`), `as i64` → JS number (NO
/// BigInt/string promotion). Post-C-2 the caller-facing `SearchHit.id` is the
/// typed `{ space, value }` object; correlate a `PerHitExplain` to its `SearchHit`
/// by position (both arrays are 1:1, same order). napi maps snake_case →
/// camelCase (`vectorRank`, `fusedScore`, `ceScore`).
#[napi(object)]
pub struct PerHitExplain {
    pub id: i64,
    pub arm: String,
    pub vector_rank: Option<u32>,
    pub text_rank: Option<u32>,
    pub graph_rank: Option<u32>,
    pub fused_score: f64,
    pub ce_score: Option<f64>,
    pub blended: f64,
    /// 0.8.16 Slice 5 / F9 — node importance / edge confidence applied to this
    /// hit's contribution (`None` = graceful-absent / neutral). Mirrors the
    /// engine `PerHitExplain` additive fields (napi → `importance`, `confidence`).
    pub importance: Option<f64>,
    pub confidence: Option<f64>,
    pub structural: StructuralInclusionV1,
}

#[napi(object)]
pub struct StructuralInclusionV1 {
    pub schema_version: u32,
    pub inclusion_state: String,
    pub projection_origin: String,
    pub dependency_state: String,
    pub lifecycle_state: String,
    pub degradation_codes: Vec<String>,
}

impl StructuralInclusionV1 {
    pub(crate) fn from_rust(value: &RustStructuralInclusionV1) -> Self {
        Self {
            schema_version: value.schema_version,
            inclusion_state: match value.inclusion_state {
                RustStructuralInclusionStateV1::Included => "included",
                RustStructuralInclusionStateV1::Degraded => "degraded",
            }
            .to_string(),
            projection_origin: match value.projection_origin {
                RustStructuralProjectionOriginV1::SynchronousBodyFts => "synchronous_body_fts",
                RustStructuralProjectionOriginV1::CurrentDenseGeneration => {
                    "current_dense_generation"
                }
                RustStructuralProjectionOriginV1::GraphTraversal => "graph_traversal",
            }
            .to_string(),
            dependency_state: match value.dependency_state {
                RustStructuralDependencyStateV1::NotApplicable => "not_applicable",
                RustStructuralDependencyStateV1::NotRegistered => "not_registered",
                RustStructuralDependencyStateV1::Registered => "registered",
            }
            .to_string(),
            lifecycle_state: match value.lifecycle_state {
                RustStructuralLifecycleStateV1::NodePending => "node_pending",
                RustStructuralLifecycleStateV1::NodeActive => "node_active",
                RustStructuralLifecycleStateV1::NodeDeleted => "node_deleted",
                RustStructuralLifecycleStateV1::EdgeValid => "edge_valid",
            }
            .to_string(),
            degradation_codes: value
                .degradation_codes
                .iter()
                .map(|code| {
                    match code {
                        RustStructuralDegradationCodeV1::SoftFallbackText => "soft_fallback_text",
                        RustStructuralDegradationCodeV1::SoftFallbackTextEdge => {
                            "soft_fallback_text_edge"
                        }
                        RustStructuralDegradationCodeV1::ProjectionLegacyUnverified => {
                            "projection_legacy_unverified"
                        }
                        RustStructuralDegradationCodeV1::ProjectionBlocked => "projection_blocked",
                        RustStructuralDegradationCodeV1::ProjectionDeferred => {
                            "projection_deferred"
                        }
                        RustStructuralDegradationCodeV1::GraphBoundReached => "graph_bound_reached",
                    }
                    .to_string()
                })
                .collect(),
        }
    }
}

impl PerHitExplain {
    pub(crate) fn from_rust(p: &RustPerHitExplain) -> Self {
        Self {
            id: p.id as i64,
            arm: match p.arm {
                SoftFallbackBranch::Vector => "vector".to_string(),
                SoftFallbackBranch::Text => "text".to_string(),
                SoftFallbackBranch::TextEdge => "text_edge".to_string(),
                SoftFallbackBranch::GraphArm => "graph_arm".to_string(),
            },
            vector_rank: p.vector_rank,
            text_rank: p.text_rank,
            graph_rank: p.graph_rank,
            fused_score: p.fused_score,
            ce_score: p.ce_score,
            blended: p.blended,
            importance: p.importance,
            confidence: p.confidence,
            structural: StructuralInclusionV1::from_rust(&p.structural),
        }
    }
}

#[cfg(test)]
mod per_hit_explain_tests {
    use super::*;
    use fathomdb_engine::{Engine as EngEngine, PreparedWrite as EngPreparedWrite};

    // 0.8.16 Slice 5 / F9 (codex §9 fix-1, FINDING 2) — the N-API `PerHitExplain`
    // mirror must copy the new `importance`/`confidence` fields from the engine
    // type, or Node/TS `searchExplained` callers cannot observe the F9 contribution
    // (the 0.8.14 `embed_batch_cls` binding blind-spot). The engine `PerHitExplain`
    // is `#[non_exhaustive]`, so it cannot be built by literal cross-crate — the
    // source value comes from a REAL `search_explained` run (F9 reweight ON, graph
    // arm ON). Runs under `cargo test` (no maturin / node needed).
    #[test]
    fn from_rust_copies_f9_importance_and_confidence() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join(format!("napi_f9_explain{}", fathomdb_schema::SQLITE_SUFFIX));
        let opened = EngEngine::open(&path).expect("open");
        let engine = &opened.engine;
        let receipt = engine
            .write(&[
                EngPreparedWrite::Node {
                    kind: "doc".to_string(),
                    body: "zephyr anchor entity".to_string(),
                    source_id: fathomdb_engine::SourceId::new("test:fixture")
                        .expect("test source id"),
                    logical_id: Some("zephyr".to_string()),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                },
                EngPreparedWrite::Node {
                    kind: "doc".to_string(),
                    body: "beta reachable payload node".to_string(),
                    source_id: fathomdb_engine::SourceId::new("test:fixture")
                        .expect("test source id"),
                    logical_id: Some("beta".to_string()),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                },
                EngPreparedWrite::Edge {
                    kind: "link".to_string(),
                    from: "zephyr".to_string(),
                    to: "beta".to_string(),
                    source_id: fathomdb_engine::SourceId::new("test:fixture")
                        .expect("test source id"),
                    logical_id: Some("e-zb".to_string()),
                    body: Some("collaboration record".to_string()),
                    t_valid: None,
                    t_invalid: None,
                    confidence: Some(0.90),
                    extractor_model_id: None,
                    temporal_fallback: None,
                },
            ])
            .expect("write");
        let beta_cursor = receipt.row_cursors[1];
        engine.write_node_importance(beta_cursor, 0.25).expect("set importance");
        engine.set_importance_reweight_enabled_for_test(true);

        let explained =
            engine.search_explained("zephyr", None, 0, true, 0.3, 0).expect("search_explained");
        let exp = explained.explanation.expect("explanation sidecar present");
        let entry = exp
            .per_hit
            .iter()
            .find(|p| p.id == beta_cursor)
            .expect("per_hit entry for the graph-reached beta node");
        // The engine populated both fields on the source explain entry.
        assert_eq!(entry.importance, Some(0.25), "source explain carries node importance");
        assert_eq!(entry.confidence, Some(0.90), "source explain carries edge confidence");

        // The binding mirror MUST copy them (the fix).
        let mirror = PerHitExplain::from_rust(entry);
        assert_eq!(mirror.importance, Some(0.25), "importance must propagate to the N-API mirror");
        assert_eq!(mirror.confidence, Some(0.90), "confidence must propagate to the N-API mirror");

        opened.engine.close().unwrap();
    }
}
