use super::*;

// ===== Slice 20 (G5/G6) — graph_neighbors + search_expand ============

/// Slice 20 — one expanded node entry in [`PySearchExpandResult`].
#[pyclass(name = "ExpandedNode", skip_from_py_object)]
#[derive(Clone)]
pub(super) struct PyExpandedNode {
    #[pyo3(get)]
    node: PyNodeRecord,
    #[pyo3(get)]
    hop_count: u32,
}

/// Slice 20 (G6) — result of `search_expand`. `search_hits` carries the
/// original RRF-scored hits; `expanded` is the list of nodes reachable by
/// graph traversal that are NOT in `search_hits`. `all_logical_ids` is the
/// deduplicated union.
#[pyclass(name = "SearchExpandResult", skip_from_py_object)]
#[derive(Clone)]
pub(super) struct PySearchExpandResult {
    #[pyo3(get)]
    search_hits: Vec<PySearchHit>,
    #[pyo3(get)]
    expanded: Vec<PyExpandedNode>,
    #[pyo3(get)]
    all_logical_ids: Vec<String>,
}

impl PySearchExpandResult {
    pub(super) fn from_rust(r: RustSearchExpandResult) -> Self {
        Self {
            search_hits: r.search_hits.iter().map(PySearchHit::from_rust).collect(),
            expanded: r
                .expanded
                .into_iter()
                .map(|(node, hop_count)| PyExpandedNode {
                    node: PyNodeRecord::from_rust(&node),
                    hop_count,
                })
                .collect(),
            all_logical_ids: r.all_logical_ids,
        }
    }
}

/// Parse a direction string ("outgoing" | "incoming" | "both") into the engine
/// enum. Returns `InvalidArgumentError` for unrecognized values (matches public
/// Python contract which raises `InvalidArgumentError` for invalid graph args).
pub(super) fn parse_direction(s: &str) -> PyResult<RustTraversalDirection> {
    match s {
        "outgoing" => Ok(RustTraversalDirection::Outgoing),
        "incoming" => Ok(RustTraversalDirection::Incoming),
        "both" => Ok(RustTraversalDirection::Both),
        other => Err(InvalidArgumentError::new_err(format!(
            "direction must be 'outgoing', 'incoming', or 'both'; got '{other}'"
        ))),
    }
}

/// Slice 20 (G5) — bounded BFS from `logical_id` over `canonical_edges`.
///
/// `depth` must be 1..=3; raises `InvalidArgumentError` for depth > 3.
/// `direction` accepts `"outgoing"`, `"incoming"`, or `"both"`.
/// Returns the set of reachable nodes (excluding the root) within `depth` hops,
/// hard-capped at 50.
#[pyfunction]
#[pyo3(signature = (engine, logical_id, depth, direction, view=None))]
pub(super) fn graph_neighbors(
    py: Python<'_>,
    engine: &PyEngine,
    logical_id: &Bound<'_, PyAny>,
    depth: u32,
    direction: &str,
    view: Option<&PyReadView>,
) -> PyResult<Vec<PyNodeRecord>> {
    let logical_id = extract_validated_str(logical_id)?;
    let dir = parse_direction(direction)?;
    let view = read_view_or_default(view);
    let inner = Arc::clone(&engine.inner);
    let nodes = call_engine(py, move || inner.graph_neighbors(&logical_id, depth, dir, &view))?;
    Ok(nodes.iter().map(PyNodeRecord::from_rust).collect())
}

/// 0.8.20 Slice 10b (R-20-NV) — nodes that crossed a validity boundary in
/// `(since, view-instant]`. `since` is INTEGER epoch SECONDS.
#[pyfunction]
#[pyo3(signature = (engine, since, view=None))]
pub(super) fn crossed_boundary_since(
    py: Python<'_>,
    engine: &PyEngine,
    since: i64,
    view: Option<&PyReadView>,
) -> PyResult<Vec<PyBoundaryCrossing>> {
    let view = read_view_or_default(view);
    let inner = Arc::clone(&engine.inner);
    let rows = call_engine(py, move || inner.crossed_boundary_since(since, &view))?;
    Ok(rows.iter().map(PyBoundaryCrossing::from_rust).collect())
}

/// Slice 20 (G6) — hybrid search followed by bounded BFS expansion.
///
/// `depth` must be 0..=3; raises `InvalidArgumentError` for depth > 3.
/// Returns a `SearchExpandResult` with the original search hits (RRF-scored)
/// plus expanded nodes reachable by traversal that are not already in the hit set.
#[pyfunction]
#[pyo3(
    signature = (engine, query, depth, source_type=None, kind=None, created_after=None, status=None, search_limit=10)
)]
#[allow(clippy::too_many_arguments)]
pub(super) fn search_expand(
    py: Python<'_>,
    engine: &PyEngine,
    query: &Bound<'_, PyAny>,
    depth: u32,
    source_type: Option<Bound<'_, PyAny>>,
    kind: Option<Bound<'_, PyAny>>,
    created_after: Option<i64>,
    status: Option<Bound<'_, PyAny>>,
    search_limit: usize,
) -> PyResult<PySearchExpandResult> {
    let query = extract_validated_str(query)?;
    // Use extract_opt_validated_str (same path as Engine.search) so lone UTF-16
    // surrogates are caught by the FFI guard before reaching the engine.
    let source_type = extract_opt_validated_str(source_type.as_ref())?;
    let kind = extract_opt_validated_str(kind.as_ref())?;
    let status = extract_opt_validated_str(status.as_ref())?;
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
    let result = call_engine(py, move || {
        inner.search_expand_with_limit(&query, filter, depth, search_limit)
    })?;
    Ok(PySearchExpandResult::from_rust(result))
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "EvidenceSidecarEntryV1",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyEvidenceSidecarEntryV1 {
    schema_version: u32,
    result_index: u32,
    artifact_revision_id: String,
    evidence_ref: String,
}

impl From<&RustEvidenceSidecarEntryV1> for PyEvidenceSidecarEntryV1 {
    fn from(value: &RustEvidenceSidecarEntryV1) -> Self {
        Self {
            schema_version: value.schema_version,
            result_index: value.result_index,
            artifact_revision_id: value.artifact_revision_id.clone(),
            evidence_ref: value.evidence_ref.as_str().to_string(),
        }
    }
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "EvidenceSearchResultV1",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyEvidenceSearchResultV1 {
    schema_version: u32,
    search_result: PySearchResult,
    evidence: Vec<PyEvidenceSidecarEntryV1>,
}

impl From<RustEvidenceSearchResultV1> for PyEvidenceSearchResultV1 {
    fn from(value: RustEvidenceSearchResultV1) -> Self {
        Self {
            schema_version: value.schema_version,
            search_result: PySearchResult::from_rust(value.search_result),
            evidence: value.evidence.iter().map(Into::into).collect(),
        }
    }
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "EvidenceContributionV1",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyEvidenceContributionV1 {
    schema_version: u32,
    vector_rank: Option<u32>,
    text_rank: Option<u32>,
    graph_rank: Option<u32>,
    fused_score: f64,
    ce_score: Option<f64>,
    blended_score: f64,
    importance: Option<f64>,
    confidence: Option<f64>,
}

impl From<RustEvidenceContributionV1> for PyEvidenceContributionV1 {
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

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "EvidenceProjectionOriginV1",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyEvidenceProjectionOriginV1 {
    schema_version: u32,
    artifact_class: String,
    representative_arm: String,
    projection_generation_id: String,
    graph_origin_kind: Option<String>,
    graph_edge_artifact_revision_id: Option<String>,
    graph_hop_count: Option<u32>,
}

impl From<RustEvidenceProjectionOriginV1> for PyEvidenceProjectionOriginV1 {
    fn from(value: RustEvidenceProjectionOriginV1) -> Self {
        let (graph_origin_kind, graph_edge_artifact_revision_id, graph_hop_count) = match value
            .graph_origin
        {
            None => (None, None, None),
            Some(RustEvidenceGraphOriginV1::EdgeSeed { edge_artifact_revision_id }) => {
                (Some("edge_seed".to_string()), Some(edge_artifact_revision_id), None)
            }
            Some(RustEvidenceGraphOriginV1::Traversal { edge_artifact_revision_id, hop_count }) => {
                (Some("traversal".to_string()), Some(edge_artifact_revision_id), Some(hop_count))
            }
        };
        Self {
            schema_version: value.schema_version,
            artifact_class: value.artifact_class.as_str().to_string(),
            representative_arm: value.representative_arm.as_str().to_string(),
            projection_generation_id: value.projection_generation_id.as_str().to_string(),
            graph_origin_kind,
            graph_edge_artifact_revision_id,
            graph_hop_count,
        }
    }
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "ResolvedEvidenceV1",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyResolvedEvidenceV1 {
    schema_version: u32,
    logical_id: Option<String>,
    artifact_revision_id: String,
    source_id: String,
    source_version_id: String,
    source_revision_id: String,
    locator_kind: String,
    locator_start_inclusive: Option<u64>,
    locator_end_exclusive: Option<u64>,
    canonical_source_body: String,
    evidence_text: String,
    canonical_source_hash: String,
    effective_valid_at: i64,
    artifact_lifecycle_kind: String,
    artifact_lifecycle_state: Option<String>,
    artifact_superseded: bool,
    artifact_valid_at_effective: Option<bool>,
    source_lifecycle_state: String,
    projection_origin: PyEvidenceProjectionOriginV1,
    retrieval_contribution: PyEvidenceContributionV1,
    dependency: Option<PySourceDependencyV1>,
}

impl From<RustResolvedEvidenceV1> for PyResolvedEvidenceV1 {
    fn from(value: RustResolvedEvidenceV1) -> Self {
        let (locator_kind, locator_start_inclusive, locator_end_exclusive) = match value.locator {
            SourceLocator::WholeBody => ("whole_body".to_string(), None, None),
            SourceLocator::Utf8Bytes { start_inclusive, end_exclusive } => {
                ("utf8_bytes".to_string(), Some(start_inclusive), Some(end_exclusive))
            }
        };
        let (
            artifact_lifecycle_kind,
            artifact_lifecycle_state,
            artifact_superseded,
            artifact_valid_at_effective,
        ) = match value.artifact_lifecycle {
            RustEvidenceArtifactLifecycleV1::Node { state, superseded } => {
                ("node".to_string(), Some(state.as_str().to_string()), superseded, None)
            }
            RustEvidenceArtifactLifecycleV1::Edge { superseded, valid_at_effective } => {
                ("edge".to_string(), None, superseded, Some(valid_at_effective))
            }
        };
        Self {
            schema_version: value.schema_version,
            logical_id: value.logical_id,
            artifact_revision_id: value.artifact_revision_id,
            source_id: value.source_id,
            source_version_id: value.source_version_id,
            source_revision_id: value.source_revision_id,
            locator_kind,
            locator_start_inclusive,
            locator_end_exclusive,
            canonical_source_body: value.canonical_source_body,
            evidence_text: value.evidence_text,
            canonical_source_hash: value.canonical_source_hash.digest_hex().to_string(),
            effective_valid_at: value.effective_valid_at,
            artifact_lifecycle_kind,
            artifact_lifecycle_state,
            artifact_superseded,
            artifact_valid_at_effective,
            source_lifecycle_state: value.source_lifecycle_state.as_str().to_string(),
            projection_origin: value.projection_origin.into(),
            retrieval_contribution: value.retrieval_contribution.into(),
            dependency: value.dependency.map(Into::into),
        }
    }
}

/// 0.8.8 EXP-OBS (Slice 10) — query-level retrieval trace (mirror of engine
/// `QueryTrace`). New fields append with the binding evolution rule
/// (`frozen, get_all, skip_from_py_object`).
#[pyclass(module = "fathomdb._fathomdb", name = "QueryTrace", frozen, get_all, skip_from_py_object)]
#[derive(Clone)]
pub(super) struct PyQueryTrace {
    query_chars: u32,
    k: u32,
    rerank_depth: u32,
    pool_n: u32,
    alpha: f64,
    use_graph_arm: bool,
    recency: bool,
    embedder_id: String,
    ce_active: bool,
    vector_hits: u32,
    text_hits: u32,
    graph_hits: u32,
    dropped_edge_hits: u32,
}

impl PyQueryTrace {
    pub(super) fn from_rust(t: &RustQueryTrace) -> Self {
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
/// `write_cursor` (the pre-0.8.19 `SearchHit.id`); post-C-2 the caller-facing
/// `SearchHit.id` is the typed `IdSpace`, so correlate a `PerHitExplain` to its
/// `SearchHit` by position (1:1, same order). `arm` crosses as the same lowercase
/// string as `SearchHit.branch`.
#[pyclass(
    module = "fathomdb._fathomdb",
    name = "PerHitExplain",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyPerHitExplain {
    pub(super) id: u64,
    pub(super) arm: String,
    pub(super) vector_rank: Option<u32>,
    pub(super) text_rank: Option<u32>,
    pub(super) graph_rank: Option<u32>,
    pub(super) fused_score: f64,
    pub(super) ce_score: Option<f64>,
    pub(super) blended: f64,
    /// 0.8.16 Slice 5 / F9 — node importance / edge confidence applied to this
    /// hit's contribution (`None` = graceful-absent / neutral). Mirrors the engine
    /// `PerHitExplain` additive fields; `get_all` exposes them as read-only Python
    /// attributes, symmetric with the N-API mirror.
    pub(super) importance: Option<f64>,
    pub(super) confidence: Option<f64>,
    pub(super) structural: PyStructuralInclusionV1,
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "StructuralInclusionV1",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyStructuralInclusionV1 {
    schema_version: u32,
    inclusion_state: String,
    projection_origin: String,
    dependency_state: String,
    lifecycle_state: String,
    degradation_codes: Vec<String>,
}

impl PyStructuralInclusionV1 {
    pub(super) fn from_rust(value: &RustStructuralInclusionV1) -> Self {
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

impl PyPerHitExplain {
    pub(super) fn from_rust(p: &RustPerHitExplain) -> Self {
        Self {
            id: p.id,
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
            structural: PyStructuralInclusionV1::from_rust(&p.structural),
        }
    }
}

/// 0.8.8 EXP-OBS (Slice 10) — the explanation sidecar (mirror of engine
/// `Explanation`): a query-level [`PyQueryTrace`] + a per-hit breakdown parallel
/// to (and in the same order as) `SearchResult.results`.
#[pyclass(
    module = "fathomdb._fathomdb",
    name = "Explanation",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyExplanation {
    trace: PyQueryTrace,
    per_hit: Vec<PyPerHitExplain>,
    correlation_id: String,
}

impl PyExplanation {
    pub(super) fn from_rust(e: &RustExplanation) -> Self {
        Self {
            trace: PyQueryTrace::from_rust(&e.trace),
            per_hit: e.per_hit.iter().map(PyPerHitExplain::from_rust).collect(),
            correlation_id: e.correlation_id.clone(),
        }
    }
}

/// 0.8.20 Slice 10b (R-20-NV) — the Python face of `BoundaryCrossing`.
#[pyclass(
    module = "fathomdb._fathomdb",
    name = "BoundaryCrossing",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyBoundaryCrossing {
    node: PyNodeRecord,
    became_valid_at: Option<i64>,
    became_invalid_at: Option<i64>,
}

impl PyBoundaryCrossing {
    pub(super) fn from_rust(c: &RustBoundaryCrossing) -> Self {
        Self {
            node: PyNodeRecord::from_rust(&c.node),
            became_valid_at: c.became_valid_at,
            became_invalid_at: c.became_invalid_at,
        }
    }
}
