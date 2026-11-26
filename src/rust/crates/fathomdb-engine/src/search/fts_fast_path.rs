use crate::dependency_closure;
use crate::filter::{self, append_edge_eligibility_sql, hit_attributes_pass_filter, SearchFilter};
use crate::fusion::{FusionHit, FusionOwnership, FusionRecord, RecordKey};
use crate::identity::{derive_stable_id, IdSpace};
use crate::properties::decode_properties_row_col;
use crate::search_types::{SearchHit, SoftFallbackBranch};
use crate::snippet::SnippetOptions;
use crate::snippet_query::{FtsRows, FtsTable};
#[cfg(feature = "tc5-benchmark")]
use crate::tc5_benchmark;
use crate::temporal::{edge_validity_sql_for_view, FrozenView};
use fathomdb_query::CompiledQuery;
use rusqlite::{CachedStatement, Connection, OptionalExtension, Statement};
#[cfg(feature = "test-hooks")]
use std::{fs::OpenOptions, io::Write, sync::Mutex};

pub(crate) struct SearchStatement<'connection> {
    statement: CachedStatement<'connection>,
}

pub(crate) fn prepare_search_statement<'connection>(
    connection: &'connection Connection,
    sql: &str,
) -> rusqlite::Result<SearchStatement<'connection>> {
    connection.prepare_cached(sql).map(|statement| SearchStatement { statement })
}

#[cfg(test)]
impl SearchStatement<'_> {
    pub(crate) fn was_reused(&self) -> bool {
        self.statement.get_status(rusqlite::StatementStatus::Run) > 0
    }

    pub(crate) fn reprepare_count(&self) -> i32 {
        self.statement.get_status(rusqlite::StatementStatus::RePrepare)
    }
}

impl<'connection> std::ops::Deref for SearchStatement<'connection> {
    type Target = Statement<'connection>;

    fn deref(&self) -> &Self::Target {
        &self.statement
    }
}

impl std::ops::DerefMut for SearchStatement<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.statement
    }
}

#[inline]
pub(super) fn observe_slice30_guard(
    phase: &'static str,
    reason: &'static str,
    blocked_value: bool,
    value: bool,
) -> bool {
    #[cfg(any(test, feature = "slice30-observer"))]
    if value == blocked_value {
        crate::slice30_observer::not_run(phase, reason);
    }
    #[cfg(not(any(test, feature = "slice30-observer")))]
    let _ = (phase, reason, blocked_value);
    value
}

pub(super) fn search_body_from_row(
    row: &rusqlite::Row<'_>,
    index: usize,
    vector: bool,
) -> rusqlite::Result<String> {
    let body = row.get::<_, String>(index)?;
    #[cfg(all(feature = "slice50-measurement", any(test, feature = "slice30-observer")))]
    crate::slice30_observer::slice50_body(vector, body.len());
    #[cfg(not(all(feature = "slice50-measurement", any(test, feature = "slice30-observer"))))]
    let _ = vector;
    Ok(body)
}

fn rank_search_hit_from_row(
    row: &rusqlite::Row<'_>,
    properties_col: usize,
) -> rusqlite::Result<(SearchHit, i64)> {
    let body = search_body_from_row(row, 0, false)?;
    let logical_id = row.get::<_, Option<String>>(4)?;
    let hit = SearchHit {
        id: derive_stable_id(logical_id.as_deref(), &body),
        body,
        kind: row.get::<_, String>(1)?,
        write_cursor: row.get::<_, i64>(2)? as u64,
        score: row.get::<_, f64>(3)?,
        branch: SoftFallbackBranch::Text,
        source_id: row.get::<_, Option<String>>(5)?,
        ce_score: None,
        properties: decode_properties_row_col(row, properties_col)?,
    };
    Ok((hit, row.get::<_, i64>(6)?))
}

pub(super) fn collect_complete_rank_boundary(
    rows: &mut rusqlite::Rows<'_>,
    limit: usize,
    _fts_rows: Option<&mut FtsRows>,
) -> rusqlite::Result<(Vec<SearchHit>, bool)> {
    let mut candidates = Vec::with_capacity(limit.saturating_add(1).min(4096));
    let mut boundary_score = None;

    while let Some(row) = rows.next()? {
        let (candidate, _rowid) = rank_search_hit_from_row(row, 7)?;
        if candidates.len() < limit {
            if candidates.len().saturating_add(1) == limit {
                boundary_score = Some(candidate.score);
            }
            candidates.push(candidate);
            continue;
        }
        if boundary_score.is_some_and(|score: f64| {
            score.total_cmp(&candidate.score) == std::cmp::Ordering::Equal
        }) {
            candidates.push(candidate);
            continue;
        }
        break;
    }

    let crossed_boundary_tie = candidates.len() > limit;
    candidates = retain_complete_rank_boundary_candidates(candidates, limit);
    Ok((candidates, crossed_boundary_tie))
}

fn node_fts_ownership(row: &rusqlite::Row<'_>) -> rusqlite::Result<FusionOwnership> {
    let owner = row.get::<_, Option<i64>>(7)?;
    if owner.is_some() {
        if !row.get::<_, bool>(8)? || !row.get::<_, bool>(9)? {
            return Err(rusqlite::Error::InvalidQuery);
        }
        Ok(FusionOwnership::CanonicalNode)
    } else {
        Ok(FusionOwnership::OwnerlessNodeFts { rowid: row.get(6)? })
    }
}

// Constructed only after comparing FTS body/kind to its canonical owner in
// this read transaction. Ownerless identity is its actual unique FTS rowid.
pub(super) struct ConsistentNodeFts {
    pub(super) rowid: i64,
    pub(super) write_cursor: u64,
    pub(super) ownership: FusionOwnership,
    pub(super) score: f64,
}

pub(super) enum DeferredFusionHit {
    Hydrated(FusionHit),
    NodeFts(ConsistentNodeFts),
}

impl FusionRecord for DeferredFusionHit {
    fn record_key(&self) -> RecordKey {
        match self {
            Self::Hydrated(hit) => hit.key(),
            Self::NodeFts(hit) => hit.ownership.record_key(hit.write_cursor),
        }
    }
    fn bodies_conflict(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Hydrated(left), Self::Hydrated(right)) => left.body != right.body,
            // A canonical thin row already proved equality to the same stored
            // record that hydrated the vector. Ownerless keys cannot intersect
            // vector keys, and repeated ownerless keys name the same FTS row.
            _ => false,
        }
    }
    fn set_fused_score(&mut self, score: f64) {
        match self {
            Self::Hydrated(hit) => hit.score = score,
            Self::NodeFts(hit) => hit.score = score,
        }
    }
}

impl DeferredFusionHit {
    pub(super) fn hydrate(
        self,
        statement: &mut rusqlite::Statement<'_>,
    ) -> rusqlite::Result<FusionHit> {
        match self {
            Self::Hydrated(hit) => Ok(hit),
            Self::NodeFts(thin) => {
                let (mut hit, _) = statement.query_row([thin.rowid], node_fusion_hit_from_row)?;
                if hit.key() != thin.ownership.record_key(thin.write_cursor) {
                    return Err(rusqlite::Error::InvalidQuery);
                }
                hit.score = thin.score;
                Ok(hit)
            }
        }
    }
}

fn node_fusion_hit_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(FusionHit, i64)> {
    let (hit, rowid) = rank_search_hit_from_row(row, 10)?;
    Ok((FusionHit { hit, ownership: node_fts_ownership(row)? }, rowid))
}

fn edge_fusion_hit_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(FusionHit, i64)> {
    if !row.get::<_, bool>(7)? || !row.get::<_, bool>(8)? {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let (mut hit, rowid) = rank_search_hit_from_row(row, 9)?;
    hit.branch = SoftFallbackBranch::TextEdge;
    Ok((FusionHit { hit, ownership: FusionOwnership::CanonicalEdge }, rowid))
}

fn collect_complete_fusion_rank_boundary(
    rows: &mut rusqlite::Rows<'_>,
    limit: usize,
    mut fts_rows: Option<&mut FtsRows>,
) -> rusqlite::Result<(Vec<FusionHit>, bool)> {
    let mut candidates = Vec::with_capacity(limit.saturating_add(1).min(4096));
    let mut boundary_score = None;
    while let Some(row) = rows.next()? {
        let (candidate, rowid) = node_fusion_hit_from_row(row)?;
        if let Some(fts_rows) = fts_rows.as_deref_mut() {
            fts_rows.entry(candidate.key()).or_insert((FtsTable::Nodes, rowid));
        }
        if candidates.len() < limit {
            if candidates.len().saturating_add(1) == limit {
                boundary_score = Some(candidate.score);
            }
            candidates.push(candidate);
        } else if boundary_score.is_some_and(|score: f64| score.total_cmp(&candidate.score).is_eq())
        {
            candidates.push(candidate);
        } else {
            break;
        }
    }
    let crossed = candidates.len() > limit;
    candidates.sort_by(|left, right| compare_text_rank(left, right));
    candidates.truncate(limit);
    Ok((candidates, crossed))
}

fn compare_text_rank(left: &SearchHit, right: &SearchHit) -> std::cmp::Ordering {
    left.score.total_cmp(&right.score).then_with(|| left.write_cursor.cmp(&right.write_cursor))
}

fn classify_legacy_node_rows(
    tx: &Connection,
    rows: Vec<(SearchHit, i64)>,
) -> rusqlite::Result<Vec<(FusionHit, i64)>> {
    let mut owner = tx.prepare("SELECT body,kind FROM canonical_nodes WHERE write_cursor=?1")?;
    rows.into_iter()
        .map(|(hit, rowid)| {
            let stored: Option<(String, String)> = owner
                .query_row([hit.write_cursor], |row| Ok((row.get(0)?, row.get(1)?)))
                .optional()?;
            let ownership = if let Some((body, kind)) = stored {
                if body != hit.body || kind != hit.kind {
                    return Err(rusqlite::Error::InvalidQuery);
                }
                FusionOwnership::CanonicalNode
            } else {
                FusionOwnership::OwnerlessNodeFts { rowid }
            };
            Ok((FusionHit { hit, ownership }, rowid))
        })
        .collect()
}

pub(crate) fn retain_complete_rank_boundary_candidates(
    mut candidates: Vec<SearchHit>,
    limit: usize,
) -> Vec<SearchHit> {
    candidates.sort_by(compare_text_rank);
    candidates.truncate(limit);
    candidates
}

#[cfg(feature = "test-hooks")]
pub(crate) fn append_json_witness_for_test(variable: &str, value: &serde_json::Value) {
    let Some(path) = std::env::var_os(variable) else {
        return;
    };
    static WITNESS_LOCK: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();
    let Ok(_guard) = WITNESS_LOCK.get_or_init(|| Mutex::new(())).lock() else {
        return;
    };
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    if let Ok(mut file) = OpenOptions::open(&options, path) {
        let _ = writeln!(file, "{value}");
    }
}

#[cfg(feature = "test-hooks")]
pub(super) fn record_fts_route_for_test(route: &str) {
    append_json_witness_for_test(
        "FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST",
        &serde_json::json!({"route": route}),
    );
}

#[cfg(feature = "test-hooks")]
pub(crate) fn slice71_search_statement_trace() -> &'static Mutex<Vec<String>> {
    static TRACE: std::sync::OnceLock<Mutex<Vec<String>>> = std::sync::OnceLock::new();
    TRACE.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(feature = "test-hooks")]
static SLICE41_SQL_TRACE_ENABLED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(feature = "test-hooks")]
pub(crate) fn slice41_sql_trace() -> &'static Mutex<Vec<String>> {
    static TRACE: std::sync::OnceLock<Mutex<Vec<String>>> = std::sync::OnceLock::new();
    TRACE.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(feature = "test-hooks")]
pub(crate) fn set_slice41_sql_trace_enabled(enabled: bool) {
    SLICE41_SQL_TRACE_ENABLED.store(enabled, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(feature = "test-hooks")]
pub(crate) fn record_slice71_profile_statement_for_test(sql: &str) {
    if SLICE41_SQL_TRACE_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        if let Ok(mut trace) = slice41_sql_trace().lock() {
            trace.push(sql.to_string());
        }
    }
    let identity = if sql
        .contains("SELECT l.source_revision_id FROM _fathomdb_artifact_revisions r")
        && sql.contains("WHERE r.write_cursor=")
    {
        Some("post_filter_source_lookup")
    } else if sql.trim() == crate::vector_spans::SCORED_EXTENT_SQL.trim() {
        Some("snippet_scored_extent")
    } else if sql.contains("highlight(") {
        Some("snippet_highlight")
    } else if is_write_statement(sql) {
        Some("write_statement")
    } else {
        None
    };
    if let Some(identity) = identity {
        if let Ok(mut trace) = slice71_search_statement_trace().lock() {
            trace.push(identity.to_string());
        }
    }
}

#[cfg(feature = "test-hooks")]
fn is_write_statement(sql: &str) -> bool {
    let head = sql.trim_start().to_ascii_uppercase();
    ["INSERT", "UPDATE", "DELETE", "REPLACE", "CREATE", "DROP", "ALTER"]
        .iter()
        .any(|verb| head.starts_with(verb))
        || head.starts_with("BEGIN IMMEDIATE")
        || head.starts_with("BEGIN EXCLUSIVE")
}

#[cfg(feature = "test-hooks")]
pub(super) fn record_fts_query_plan_for_test(
    transaction: &rusqlite::Transaction<'_>,
    statement: &str,
    parameters: &[rusqlite::types::Value],
) {
    let explain = format!("EXPLAIN QUERY PLAN {statement}");
    let observation = transaction
        .prepare(&explain)
        .and_then(|mut prepared| {
            let details = prepared
                .query_map(rusqlite::params_from_iter(parameters.iter()), |row| {
                    row.get::<_, String>(3)
                })?;
            details.collect::<rusqlite::Result<Vec<_>>>()
        })
        .map(|details| {
            serde_json::json!({
                "uses_temp_btree_for_order_by": details
                    .iter()
                    .any(|detail| detail.contains("USE TEMP B-TREE FOR ORDER BY")),
                "details": details,
            })
        });
    if let Ok(observation) = observation {
        append_json_witness_for_test("FATHOMDB_FTS_QUERY_PLAN_WITNESS_FOR_TEST", &observation);
    }
}

/// Splits `(hit, rowid)` rows, recording each FTS rowid when snippets
/// are requested.
fn keep_rowids(
    rows: Vec<(FusionHit, i64)>,
    table: FtsTable,
    keep: bool,
    fts_rows: &mut FtsRows,
) -> Vec<FusionHit> {
    rows.into_iter()
        .map(|(hit, rowid)| {
            if keep {
                fts_rows.entry(hit.key()).or_insert((table, rowid));
            }
            hit
        })
        .collect()
}

pub(super) struct NodeFtsOutcome {
    pub(super) candidates: Vec<FusionHit>,
    pub(super) deferred_node_candidates: Option<Vec<ConsistentNodeFts>>,
    pub(super) deferred_text_identity: bool,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn collect_node_fts_candidates(
    tx: &rusqlite::Transaction<'_>,
    compiled: &CompiledQuery,
    query_vector: Option<&str>,
    final_limit: usize,
    direct_text_candidate_limit: Option<usize>,
    filter: Option<&SearchFilter>,
    recency_enabled: bool,
    importance_enabled: bool,
    vector_stage_only: bool,
    rerank_depth: usize,
    pool_n: usize,
    use_graph_arm: bool,
    track_contributions: bool,
    has_vector_kinds: bool,
    vector_rows_visible: bool,
    view: FrozenView,
    now_param: Option<i64>,
    has_source_dependencies: bool,
    read_dependency_eligibility: &dyn Fn(&str, usize) -> String,
    scope: &crate::scope::ScopePredicate,
    snippet: Option<&SnippetOptions>,
    fts_rows: &mut FtsRows,
) -> rusqlite::Result<NodeFtsOutcome> {
    #[cfg(any(test, feature = "slice30-observer"))]
    let node_observation = crate::slice30_observer::span("node_fts_candidates");
    #[cfg(feature = "tc5-benchmark")]
    tc5_benchmark::record_fts_route();
    #[cfg(test)]
    let rank_probe = crate::slice30_observer::rank_probe_route();
    // Optional experimental FTS5 LIMIT cap. It applies only when
    // FATHOMDB_PERF_EXPERIMENTS is present and
    // FATHOMDB_PERF_SEARCH_LIMIT parses as a number. The default path
    // keeps the uncapped candidate set.
    let perf_limit: Option<usize> = if std::env::var_os("FATHOMDB_PERF_EXPERIMENTS").is_some() {
        std::env::var("FATHOMDB_PERF_SEARCH_LIMIT").ok().and_then(|s| s.parse().ok())
    } else {
        None
    };
    let fts_only_safe = !use_graph_arm && !recency_enabled && !importance_enabled;
    let effective_fts_cap =
        if rerank_depth > 0 { final_limit.max(rerank_depth).max(pool_n) } else { final_limit };
    let vector_arm_structurally_empty =
        query_vector.is_none() || (!vector_rows_visible && !has_vector_kinds);
    let fts_only_limit = direct_text_candidate_limit
        .or_else(|| (fts_only_safe && vector_arm_structurally_empty).then_some(effective_fts_cap));
    let limit_clause =
        fts_only_limit.or(perf_limit).map(|k| format!(" LIMIT {k}")).unwrap_or_default();
    let text_existence = if scope.is_unscoped() {
        view.view.ownerless_existence_sql("cn")
    } else {
        format!(
            "{} AND cn.source_id IS NOT NULL{}",
            view.view.edge_existence_sql("cn"),
            if view.view.include_inactive { "" } else { " AND cn.state = 'active'" }
        )
    };
    let text_validity = view.validity_sql("cn", 2);
    let text_eligibility = read_dependency_eligibility("cn", 2);
    let mut text_params: Vec<rusqlite::types::Value> =
        vec![rusqlite::types::Value::Text(compiled.match_expression.clone())];
    if let Some(now) = now_param {
        text_params.push(rusqlite::types::Value::Integer(now));
    }
    let mut scope_clause = String::new();
    scope.append_sql_values("cn.scope", &mut scope_clause, &mut text_params);
    let text_scope = if scope.is_unscoped() {
        format!(" AND ({scope_clause} OR cn.write_cursor IS NULL)")
    } else {
        format!(" AND {scope_clause}")
    };
    let text_filter =
        filter::append_node_eligibility_sql_view(tx, filter, "cn", &mut text_params, &view.view)?;
    let fts_kind_clause = if let Some(kind) = filter.and_then(|f| f.kind.as_deref()) {
        let kind_idx = text_params.len() + 1;
        text_params.push(rusqlite::types::Value::Text(kind.to_string()));
        format!(" AND search_index.kind = ?{kind_idx}")
    } else {
        String::new()
    };
    #[cfg(feature = "test-hooks")]
    let force_full_sort =
        std::env::var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST").is_ok_and(|value| value == "1");
    #[cfg(not(feature = "test-hooks"))]
    let force_full_sort = false;
    #[cfg(test)]
    let force_full_sort = rank_probe.map_or(force_full_sort, |route| {
        route == crate::slice30_observer::RankProbeRoute::Reference
    });
    let can_defer_text_identity_cheap = scope.is_unscoped()
        && observe_slice30_guard(
            "all_nodes_directly_eligible",
            "vector_unavailable",
            false,
            query_vector.is_some(),
        )
        && observe_slice30_guard("all_nodes_directly_eligible", "filter", false, filter.is_none())
        && observe_slice30_guard(
            "all_nodes_directly_eligible",
            "perf_limit",
            false,
            perf_limit.is_none(),
        )
        && observe_slice30_guard(
            "all_nodes_directly_eligible",
            "fts_only_limit",
            false,
            fts_only_limit.is_none(),
        )
        && observe_slice30_guard(
            "all_nodes_directly_eligible",
            "source_dependencies",
            false,
            !has_source_dependencies,
        )
        && observe_slice30_guard(
            "all_nodes_directly_eligible",
            "include_superseded",
            false,
            !view.view.include_superseded,
        )
        && observe_slice30_guard(
            "all_nodes_directly_eligible",
            "include_inactive",
            false,
            !view.view.include_inactive,
        )
        && observe_slice30_guard(
            "all_nodes_directly_eligible",
            "include_out_of_window",
            false,
            !view.view.include_out_of_window,
        )
        && observe_slice30_guard("all_nodes_directly_eligible", "recency", false, !recency_enabled)
        && observe_slice30_guard(
            "all_nodes_directly_eligible",
            "importance",
            false,
            !importance_enabled,
        )
        && observe_slice30_guard("all_nodes_directly_eligible", "rerank", false, rerank_depth == 0)
        && observe_slice30_guard("all_nodes_directly_eligible", "graph", false, !use_graph_arm)
        && observe_slice30_guard(
            "all_nodes_directly_eligible",
            "explain",
            false,
            !track_contributions,
        )
        && observe_slice30_guard(
            "all_nodes_directly_eligible",
            "force_full_sort",
            false,
            !force_full_sort,
        );
    let can_defer_text_identity = can_defer_text_identity_cheap
        && tx
            .prepare_cached(
                "SELECT NOT (\
                    EXISTS(SELECT 1 FROM canonical_nodes WHERE scope < '' LIMIT 1) \
                    OR EXISTS(SELECT 1 FROM canonical_nodes WHERE scope > '' LIMIT 1)\
                )",
            )
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, bool>(0)))
            .unwrap_or(false);
    let join_eligibility = if scope.is_unscoped() {
        format!("{text_scope}{text_eligibility}")
    } else {
        format!(" AND cn.source_id IS NOT NULL{text_scope}{text_eligibility}")
    };
    let join_sql = filter::body_fts_rank_sql_view(
        &fts_kind_clause,
        &text_validity,
        &join_eligibility,
        &text_filter,
        &limit_clause,
        &view.view,
    );
    let rank_stream_requested = fts_only_limit.is_some() && !force_full_sort && !vector_stage_only;
    let rank_stream_eligible = rank_stream_requested
        && tx
            .query_row("SELECT NOT EXISTS(SELECT 1 FROM search_index_edges LIMIT 1)", [], |row| {
                row.get::<_, bool>(0)
            })
            .unwrap_or(false);
    let rank_stream_candidates = if rank_stream_eligible {
        let rank_candidate_limit = fts_only_limit.unwrap_or(final_limit);
        #[cfg(feature = "test-hooks")]
        let rank_mapping = if std::env::var_os("FATHOMDB_FTS_FAIL_STREAM_FOR_TEST").is_some() {
            "missing_ranker()"
        } else {
            "bm25()"
        };
        #[cfg(not(feature = "test-hooks"))]
        let rank_mapping = "bm25()";
        let cn_join = if !scope.is_unscoped() || !text_filter.is_empty() {
            "CROSS JOIN"
        } else {
            "LEFT JOIN"
        };
        let rank_sql = format!(
            "SELECT search_index.body, search_index.kind, search_index.write_cursor, \
             bm25(search_index), cn.logical_id, cn.source_id, search_index.rowid, cn.write_cursor, \
             search_index.body IS cn.body, search_index.kind IS cn.kind, cn.properties \
             FROM search_index \
             {cn_join} canonical_nodes cn ON cn.write_cursor = search_index.write_cursor \
             WHERE search_index MATCH ?1{fts_kind_clause} \
               {text_existence}{text_scope}{text_validity}{text_eligibility}{text_filter} \
               AND rank MATCH '{rank_mapping}' \
             ORDER BY rank"
        );
        #[cfg(feature = "test-hooks")]
        record_fts_query_plan_for_test(tx, &rank_sql, &text_params);
        tx.prepare(&rank_sql)
            .and_then(|mut statement| {
                let mut rows = statement.query(rusqlite::params_from_iter(text_params.iter()))?;
                collect_complete_fusion_rank_boundary(
                    &mut rows,
                    rank_candidate_limit,
                    snippet.is_some().then_some(fts_rows),
                )
            })
            .ok()
    } else {
        None
    };
    // Mixed node/edge text ranks stay on the reference implementation until
    // that distinct route has its own exactness and work proof.
    #[cfg(feature = "slice50-measurement")]
    let eager_text = std::env::var("FATHOMDB_SLICE50_EAGER_TEXT_FOR_MEASUREMENT")
        .is_ok_and(|value| value == "1");
    #[cfg(not(feature = "slice50-measurement"))]
    let eager_text = false;
    #[cfg(test)]
    let eager_text = rank_probe.is_none() && eager_text;
    // The node-only route already joins each matched owner before ranking.
    let defer_full_hydration = can_defer_text_identity
        && !eager_text
        && !vector_stage_only
        && tx.query_row(
            "SELECT NOT EXISTS(SELECT 1 FROM search_index_edges LIMIT 1)",
            [],
            |row| row.get::<_, bool>(0),
        )?;
    let defer_text_identity = can_defer_text_identity
        && !defer_full_hydration
        && dependency_closure::all_nodes_directly_eligible(
            tx,
            view.view.include_superseded,
            view.view.include_inactive,
            view.view.include_out_of_window,
            view.edge_now(),
            scope,
        )?;
    let mut deferred_node_candidates: Option<Vec<ConsistentNodeFts>> = None;
    let deferred_candidates = if defer_full_hydration {
        let sql = format!(
            "SELECT si.rowid, si.write_cursor, bm25(search_index), cn.write_cursor, \
             si.body IS cn.body, si.kind IS cn.kind, si.body, si.kind, cn.logical_id, cn.source_id \
             FROM search_index si \
             LEFT JOIN canonical_nodes cn ON cn.write_cursor=si.write_cursor \
             WHERE search_index MATCH ?1 {text_existence}{text_scope}{text_validity}"
        );
        let mut statement = prepare_search_statement(tx, &sql)?;
        let mut requires_legacy_proof = false;
        let rows = statement.query_map(rusqlite::params_from_iter(text_params.iter()), |row| {
            // Validate losing matches before deferring their allocations.
            row.get_ref(6)?.as_str().map_err(|_| rusqlite::Error::InvalidQuery)?;
            row.get_ref(7)?.as_str().map_err(|_| rusqlite::Error::InvalidQuery)?;
            let rowid = row.get::<_, i64>(0)?;
            let raw_cursor = row.get::<_, i64>(1)?;
            requires_legacy_proof = requires_legacy_proof
                || raw_cursor < 0
                || row.get_ref(8)?.as_str_or_null().is_err()
                || row.get_ref(9)?.as_str_or_null().is_err();
            let write_cursor = raw_cursor as u64;
            let ownership = if row.get::<_, Option<i64>>(3)?.is_some() {
                if !row.get::<_, bool>(4)? || !row.get::<_, bool>(5)? {
                    return Err(rusqlite::Error::InvalidQuery);
                }
                FusionOwnership::CanonicalNode
            } else {
                FusionOwnership::OwnerlessNodeFts { rowid }
            };
            let score = row.get(2)?;
            #[cfg(all(feature = "slice50-measurement", any(test, feature = "slice30-observer")))]
            crate::slice30_observer::slice50_lightweight_row();
            Ok(ConsistentNodeFts { rowid, write_cursor, ownership, score })
        })?;
        let mut candidates = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        // An unusual matched row may require the legacy sort or decoding.
        if requires_legacy_proof
            && !dependency_closure::all_nodes_directly_eligible(
                tx,
                view.view.include_superseded,
                view.view.include_inactive,
                view.view.include_out_of_window,
                view.edge_now(),
                scope,
            )?
        {
            None
        } else {
            #[cfg(any(test, feature = "slice30-observer"))]
            if !requires_legacy_proof {
                crate::slice30_observer::not_run(
                    "all_nodes_directly_eligible",
                    "match_local_eligibility",
                );
            }
            #[cfg(all(feature = "slice50-measurement", any(test, feature = "slice30-observer")))]
            crate::slice30_observer::slice50_route("winner_hydration");
            candidates.sort_by(|left, right| {
                left.score
                    .total_cmp(&right.score)
                    .then_with(|| left.write_cursor.cmp(&right.write_cursor))
            });
            if snippet.is_some() {
                for hit in &candidates {
                    fts_rows
                        .entry(hit.ownership.record_key(hit.write_cursor))
                        .or_insert((FtsTable::Nodes, hit.rowid));
                }
            }
            deferred_node_candidates = Some(candidates);
            Some(Vec::new())
        }
    } else if defer_text_identity {
        #[cfg(all(feature = "slice50-measurement", any(test, feature = "slice30-observer")))]
        crate::slice30_observer::slice50_route("eager_text_identity");
        let now_binding = now_param.map(|_| " AND (?2 IS NULL OR ?2 IS NOT NULL)").unwrap_or("");
        let sql = format!(
            "SELECT si.body, si.kind, si.write_cursor, bm25(search_index), NULL, NULL, si.rowid, cn.write_cursor, \
             si.body IS cn.body, si.kind IS cn.kind FROM search_index si \
             LEFT JOIN canonical_nodes cn ON cn.write_cursor=si.write_cursor \
             WHERE search_index MATCH ?1{now_binding}{text_scope}"
        );
        let candidates = prepare_search_statement(tx, &sql).and_then(|mut statement| {
            let rows =
                statement.query_map(rusqlite::params_from_iter(text_params.iter()), |row| {
                    let body = search_body_from_row(row, 0, false)?;
                    let hit = SearchHit {
                        id: IdSpace::content(String::new()),
                        body,
                        kind: row.get::<_, String>(1)?,
                        write_cursor: row.get::<_, i64>(2)? as u64,
                        score: row.get::<_, f64>(3)?,
                        branch: SoftFallbackBranch::Text,
                        source_id: None,
                        ce_score: None,
                        properties: None,
                    };
                    Ok((
                        FusionHit { hit, ownership: node_fts_ownership(row)? },
                        row.get::<_, i64>(6)?,
                    ))
                })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })?;
        let mut candidates = keep_rowids(candidates, FtsTable::Nodes, snippet.is_some(), fts_rows);
        candidates.sort_by(|left, right| {
            left.score
                .total_cmp(&right.score)
                .then_with(|| left.write_cursor.cmp(&right.write_cursor))
        });
        Some(candidates)
    } else {
        None
    };
    let mut deferred_text_identity = false;
    let candidates = if let Some(candidates) = deferred_candidates {
        deferred_text_identity = !defer_full_hydration;
        #[cfg(feature = "test-hooks")]
        record_fts_route_for_test("hybrid_deferred_identity");
        candidates
    } else if let Some((candidates, _crossed_boundary_tie)) = rank_stream_candidates {
        #[cfg(feature = "test-hooks")]
        record_fts_route_for_test(if _crossed_boundary_tie {
            "rank_stream_tie_completed"
        } else {
            "rank_stream_strict_boundary"
        });
        candidates
    } else if let Ok(mut statement) = tx.prepare(&join_sql) {
        #[cfg(test)]
        if force_full_sort && query_vector.is_some() && rank_probe.is_some() {
            crate::slice30_observer::record_rank_probe(
                "s50_ranking_route",
                serde_json::json!("hybrid_full_sort_forced"),
            );
        }
        #[cfg(feature = "test-hooks")]
        if query_vector.is_some() && force_full_sort {
            record_fts_route_for_test("hybrid_full_sort_forced");
        } else if rank_stream_eligible {
            record_fts_route_for_test("full_sort_fallback");
        } else if rank_stream_requested {
            record_fts_route_for_test("full_sort_ineligible");
        }
        let rows = statement
            .query_map(rusqlite::params_from_iter(text_params.iter()), node_fusion_hit_from_row)?;
        let hits = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        keep_rowids(hits, FtsTable::Nodes, snippet.is_some(), fts_rows)
    } else {
        let source_sql = format!(
            "SELECT search_index.body, search_index.kind, search_index.write_cursor, \
             bm25(search_index), cn.source_id, search_index.rowid FROM search_index \
             LEFT JOIN canonical_nodes cn ON cn.write_cursor = search_index.write_cursor \
             WHERE search_index MATCH ?1 \
             ORDER BY bm25(search_index), search_index.write_cursor{limit_clause}"
        );
        if let Ok(mut statement) = tx.prepare(&source_sql) {
            let rows = statement.query_map([compiled.match_expression.as_str()], |row| {
                let body = search_body_from_row(row, 0, false)?;
                let hit = SearchHit {
                    // No logical_id column on this schema → content-hash id.
                    id: derive_stable_id(None, &body),
                    body,
                    kind: row.get::<_, String>(1)?,
                    write_cursor: row.get::<_, i64>(2)? as u64,
                    score: row.get::<_, f64>(3)?,
                    branch: SoftFallbackBranch::Text,
                    source_id: row.get::<_, Option<String>>(4)?,
                    ce_score: None,
                    properties: None,
                };
                Ok((hit, row.get::<_, i64>(5)?))
            })?;
            keep_rowids(
                classify_legacy_node_rows(tx, rows.collect::<rusqlite::Result<Vec<_>>>()?)?,
                FtsTable::Nodes,
                snippet.is_some(),
                fts_rows,
            )
        } else {
            // Pre-step-8: no `source_id` column anywhere. Byte-identical to
            // the historical statement.
            let plain_sql = format!(
                "SELECT body, kind, write_cursor, bm25(search_index), rowid FROM search_index \
                 WHERE search_index MATCH ?1 \
                 ORDER BY bm25(search_index), write_cursor{limit_clause}"
            );
            let mut statement = tx.prepare(&plain_sql)?;
            let rows = statement.query_map([compiled.match_expression.as_str()], |row| {
                let body = search_body_from_row(row, 0, false)?;
                let hit = SearchHit {
                    // No logical_id column on this schema → content-hash id.
                    id: derive_stable_id(None, &body),
                    body,
                    kind: row.get::<_, String>(1)?,
                    write_cursor: row.get::<_, i64>(2)? as u64,
                    score: row.get::<_, f64>(3)?,
                    branch: SoftFallbackBranch::Text,
                    source_id: None,
                    ce_score: None,
                    properties: None,
                };
                Ok((hit, row.get::<_, i64>(4)?))
            })?;
            keep_rowids(
                classify_legacy_node_rows(tx, rows.collect::<rusqlite::Result<Vec<_>>>()?)?,
                FtsTable::Nodes,
                snippet.is_some(),
                fts_rows,
            )
        }
    };
    #[cfg(any(test, feature = "slice30-observer"))]
    node_observation
        .counted_success(deferred_node_candidates.as_ref().map_or(candidates.len(), Vec::len));
    Ok(NodeFtsOutcome { candidates, deferred_node_candidates, deferred_text_identity })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn collect_edge_fts_candidates(
    tx: &Connection,
    compiled: &CompiledQuery,
    view: FrozenView,
    filter: Option<&SearchFilter>,
    read_dependency_eligibility: &dyn Fn(&str, usize) -> String,
    scope: &crate::scope::ScopePredicate,
    keep_snippets: bool,
    fts_rows: &mut FtsRows,
) -> rusqlite::Result<Vec<FusionHit>> {
    #[cfg(any(test, feature = "slice30-observer"))]
    let edge_observation = crate::slice30_observer::span("edge_fts_candidates");
    let edge_validity = edge_validity_sql_for_view("ce", 2, &view.view);
    let edge_dependency = read_dependency_eligibility("ce", 2);
    let mut edge_params = vec![
        rusqlite::types::Value::Text(compiled.match_expression.clone()),
        rusqlite::types::Value::Integer(view.edge_now()),
    ];
    let mut edge_scope_sql = String::new();
    scope.append_sql_values("ce.scope", &mut edge_scope_sql, &mut edge_params);
    let edge_filter = append_edge_eligibility_sql(filter, "ce", &mut edge_params);
    let edge_eligibility = format!(" AND {edge_scope_sql}{edge_dependency}{edge_filter}");
    let edge_sql = filter::edge_fts_rank_sql_view(&edge_validity, &edge_eligibility, &view.view);
    let mut stmt = prepare_search_statement(tx, &edge_sql)?;
    let rows =
        stmt.query_map(rusqlite::params_from_iter(edge_params.iter()), edge_fusion_hit_from_row)?;
    let hits = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    let candidates = keep_rowids(hits, FtsTable::Edges, keep_snippets, fts_rows);
    #[cfg(any(test, feature = "slice30-observer"))]
    edge_observation.counted_success(candidates.len());
    Ok(candidates)
}

pub(super) fn count_dropped_edge_hits(
    tx: &Connection,
    compiled: &CompiledQuery,
    view: FrozenView,
    filter: Option<&SearchFilter>,
    scope: &crate::scope::ScopePredicate,
    explain: bool,
) -> rusqlite::Result<u32> {
    if !(explain
        && filter.is_some_and(|active_filter| {
            !active_filter.attributes.is_empty() || !active_filter.property_predicates.is_empty()
        }))
    {
        return Ok(0);
    }
    let mut non_attribute_filter = filter.cloned().unwrap_or_default();
    non_attribute_filter.attributes.clear();
    non_attribute_filter.property_predicates.clear();
    let edge_validity = edge_validity_sql_for_view("ce", 2, &view.view);
    let mut params = vec![
        rusqlite::types::Value::Text(compiled.match_expression.clone()),
        rusqlite::types::Value::Integer(view.edge_now()),
    ];
    let mut edge_scope_sql = String::new();
    scope.append_sql_values("ce.scope", &mut edge_scope_sql, &mut params);
    let eligibility = append_edge_eligibility_sql(Some(&non_attribute_filter), "ce", &mut params);
    #[cfg(feature = "test-hooks")]
    let cursor_column =
        if std::env::var_os("FATHOMDB_EDGE_EXPLANATION_BAD_CURSOR_FOR_TEST").is_some() {
            "CAST(ce.write_cursor AS BLOB)"
        } else {
            "ce.write_cursor"
        };
    #[cfg(not(feature = "test-hooks"))]
    let cursor_column = "ce.write_cursor";
    let sql = format!(
        "SELECT {cursor_column} FROM search_index_edges sei \
         JOIN canonical_edges ce ON ce.write_cursor=sei.write_cursor \
         WHERE search_index_edges MATCH ?1 AND {edge_scope_sql} AND ce.superseded_at IS NULL\
         {edge_validity}{eligibility}"
    );
    let mut dropped = 0_u32;
    let mut statement = tx.prepare(&sql)?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(params.iter()), |row| row.get::<_, i64>(0))?;
    for cursor in rows {
        if !hit_attributes_pass_filter(
            tx,
            cursor? as u64,
            filter.expect("attribute filter checked above"),
        )? {
            dropped = dropped.saturating_add(1);
        }
    }
    Ok(dropped)
}

pub(super) fn hydrate_deferred_node_candidates(
    tx: &Connection,
    fused: Vec<DeferredFusionHit>,
    final_limit: usize,
) -> rusqlite::Result<Vec<FusionHit>> {
    let mut hydrate = tx.prepare_cached(
        "SELECT si.body, si.kind, si.write_cursor, 0.0, cn.logical_id, cn.source_id, \
         si.rowid, cn.write_cursor, si.body IS cn.body, si.kind IS cn.kind, cn.properties \
         FROM search_index si LEFT JOIN canonical_nodes cn \
         ON cn.write_cursor=si.write_cursor WHERE si.rowid=?1",
    )?;
    fused.into_iter().take(final_limit).map(|candidate| candidate.hydrate(&mut hydrate)).collect()
}

pub(super) fn hydrate_deferred_text_identity(
    tx: &Connection,
    results: &mut [FusionHit],
    scope: &crate::scope::ScopePredicate,
) -> rusqlite::Result<()> {
    let (scope_sql, _) = scope.sql_clause("scope", 2);
    let mut identity_stmt = tx.prepare_cached(&format!(
        "SELECT logical_id, source_id, properties FROM canonical_nodes \
         WHERE write_cursor=?1 AND {scope_sql} LIMIT 1"
    ))?;
    for hit in results {
        if hit.branch != SoftFallbackBranch::Text {
            continue;
        }
        let mut params = vec![rusqlite::types::Value::Integer(hit.write_cursor as i64)];
        scope.push_values(&mut params);
        if let Some((logical_id, source_id, properties)) = identity_stmt
            .query_row(rusqlite::params_from_iter(params.iter()), |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    decode_properties_row_col(row, 2)?,
                ))
            })
            .optional()?
        {
            hit.id = derive_stable_id(logical_id.as_deref(), &hit.body);
            hit.source_id = source_id;
            hit.properties = properties;
        } else {
            hit.id = derive_stable_id(None, &hit.body);
        }
    }
    Ok(())
}
