use super::SearchOriginCapture;
use crate::dependency_closure;
use crate::filter::{self, SearchFilter};
use crate::fusion::{FusionHit, FusionOwnership, RecordKey};
use crate::graph_expand::execution::append_cpv_exists_prefilter;
use crate::identity::{derive_stable_id, IdSpaceKind};
use crate::projection_registry::{load_projection_registry, ProjectionRole};
use crate::properties::{
    decode_properties_row_col, eval_property_predicate, PropertyEntityClass, PropertyMap,
    PropertyPredicate,
};
use crate::search_types::{
    CapturedGraphOrigin, GraphArmOptions, GraphArmSeedSource, GraphFrontierStats, GraphSeedMode,
    SearchHit, SoftFallbackBranch,
};
#[cfg(any(test, feature = "test-hooks"))]
use crate::temporal::ReadView;
use crate::temporal::{edge_validity_sql_for_view, FrozenView};
use rusqlite::{Connection, OptionalExtension};
use std::collections::{HashMap, HashSet, VecDeque};

/// R3 (Slice 30) + C1 (0.8.1 graph-arm seeding) — graph-arm BFS candidate generation.
///
/// **C1 seeding (the BLOCK-1 fix):** the frontier is seeded from the graph's OWN
/// query-matched text surfaces — NOT from doc-node hits (doc nodes carry
/// `logical_id = NULL`, so the old doc-seeding produced an empty frontier). Two
/// seed sources are unioned on `match_expression` (the compiled FTS query):
///   A. **edge-fact FTS** (`search_index_edges`) — both endpoints (`from_id`,
///      `to_id`) of matched, temporally-live, non-fallback edges;
///   B. **entity-node FTS** (`search_index` ⋈ `canonical_nodes`) — matched nodes
///      with `logical_id IS NOT NULL` (excludes doc nodes — the bug surface).
/// Each distinct candidate `logical_id` is counted in `seeds_considered`; those
/// confirmed active in `canonical_nodes` are `seeds_resolved` and pushed onto the
/// frontier (dangling edge endpoints count considered-but-unresolved).
///
/// Phase 2 is unchanged: BFS over `canonical_edges` with the temporal filter,
/// carrying each traversed edge's `source_id` (G0 BLOCK-2) onto the emitted hit.
/// Collects reachable node bodies (up to `cap`) as [`SearchHit`]s tagged
/// `SoftFallbackBranch::GraphArm`. Score = `1.0 / (1.0 + hop_count)` with a
/// synthesized-node penalty (`kind = 'unknown'` → score *= 0.3). Records already
/// present in `fused_hits` are excluded (already covered by the two-arm result).
///
/// **F9 (0.8.16 Slice 5) confidence carry:** the third tuple element maps each
/// emitted graph-arm hit's `write_cursor` (its `SearchHit.id`, a NODE cursor) to
/// the `confidence` of the EDGE traversed to reach that node — the input the F9
/// reweight (`graph_rrf_score(edge) = confidence × 1/(K+bfs_rank)`) consumes.
/// `build_importance_confidence_maps` keys edge confidence on the EDGE
/// `write_cursor`, which never equals a reached node's cursor, so without this
/// carry edge confidence never reaches a graph-arm hit. **Determinism rule (matches
/// the BLOCK-2 provenance carry):** when several edges reach the same node, the
/// FIRST edge to claim the node in the `visited` dedup wins — i.e. the edge that
/// produced the node's winning `bfs_rank` (seeds are considered before Phase-2
/// neighbors; within a phase, `ORDER BY write_cursor` makes the earliest-written
/// edge win). A NULL edge confidence is simply not inserted ⇒ neutral (1.0).
fn append_graph_arm_edge_kinds_sql(
    alias: &str,
    edge_kinds: &[String],
    params: &mut Vec<rusqlite::types::Value>,
) -> String {
    if edge_kinds.len() == 1 {
        params.push(rusqlite::types::Value::Text(edge_kinds[0].clone()));
        let idx = params.len();
        format!(" AND {alias}.kind = ?{idx}")
    } else if !edge_kinds.is_empty() {
        let mut sql = format!(" AND {alias}.kind IN (");
        for (i, kind) in edge_kinds.iter().enumerate() {
            if i > 0 {
                sql.push(',');
            }
            params.push(rusqlite::types::Value::Text(kind.clone()));
            let idx = params.len();
            sql.push_str(&format!("?{idx}"));
        }
        sql.push(')');
        sql
    } else {
        String::new()
    }
}

fn build_graph_arm_phase2_query(
    tx: &Connection,
    view: FrozenView,
    filter: Option<&SearchFilter>,
    graph_options: &GraphArmOptions,
    pushed_edge_predicates: &[&PropertyPredicate],
) -> rusqlite::Result<(String, Vec<rusqlite::types::Value>)> {
    let now_param = view.now_param();
    let edge_existence = view.view.edge_existence_sql("e");
    let edge_validity = edge_validity_sql_for_view("e", 2, &view.view);
    let target_node = view.node_sql("target", 3);
    let mut edge_params_template = vec![
        rusqlite::types::Value::Text(String::new()),
        rusqlite::types::Value::Integer(view.edge_now()),
    ];
    if let Some(now) = now_param {
        edge_params_template.push(rusqlite::types::Value::Integer(now));
    }
    let target_filter = filter::append_node_eligibility_sql_view(
        tx,
        filter,
        "target",
        &mut edge_params_template,
        &view.view,
    )?;
    let edge_kinds_sql = append_graph_arm_edge_kinds_sql(
        "e",
        &graph_options.graph_edge_kinds,
        &mut edge_params_template,
    );
    let mut edge_cpv_sql = String::new();
    for (idx, pred) in pushed_edge_predicates.iter().enumerate() {
        append_cpv_exists_prefilter(
            "edge",
            "e.write_cursor",
            &format!("cpv_p2_{idx}"),
            pred,
            &mut edge_params_template,
            &mut edge_cpv_sql,
        );
    }
    let limit_sql = if graph_options.graph_edge_predicates.is_empty() { " LIMIT 64" } else { "" };
    let sql = format!(
        "SELECT e.from_id, e.to_id, e.source_id, e.confidence, e.write_cursor AS write_cursor, e.properties \
         FROM canonical_edges e \
         WHERE e.from_id = ?1 \
           {edge_existence}{edge_validity} \
           AND (e.temporal_fallback IS NULL OR e.temporal_fallback = 0) \
           {edge_kinds_sql}{edge_cpv_sql} \
           AND EXISTS(SELECT 1 FROM canonical_nodes target WHERE target.logical_id = e.to_id {target_node}{target_filter}) \
         UNION ALL \
         SELECT e.from_id, e.to_id, e.source_id, e.confidence, e.write_cursor AS write_cursor, e.properties \
         FROM canonical_edges e \
         WHERE e.to_id = ?1 AND e.from_id != ?1 \
           {edge_existence}{edge_validity} \
           AND (e.temporal_fallback IS NULL OR e.temporal_fallback = 0) \
           {edge_kinds_sql}{edge_cpv_sql} \
           AND EXISTS(SELECT 1 FROM canonical_nodes target WHERE target.logical_id = e.from_id {target_node}{target_filter}) \
         ORDER BY write_cursor{limit_sql}"
    );
    Ok((sql, edge_params_template))
}

#[cfg(any(test, feature = "test-hooks"))]
pub(crate) fn explain_graph_arm_phase2_query_for_test(
    connection: &Connection,
    edge_kinds: &[String],
) -> rusqlite::Result<Vec<String>> {
    let view = ReadView { valid_as_of: Some(1_750_000_000), ..ReadView::default() }.freeze();
    let opts = GraphArmOptions { graph_edge_kinds: edge_kinds.to_vec(), ..Default::default() };
    let (sql, mut params) = build_graph_arm_phase2_query(connection, view, None, &opts, &[])?;
    params[0] = rusqlite::types::Value::Text("fixture".to_string());
    let mut stmt = connection.prepare(&format!("EXPLAIN QUERY PLAN {sql}"))?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| row.get(3))?;
    rows.collect()
}

struct BfsCandidateSeed {
    logical_id: String,
    source_id: Option<String>,
    confidence: Option<f64>,
    graph_origin: Option<CapturedGraphOrigin>,
    seed_source: GraphArmSeedSource,
}

pub(super) struct BfsGraphArmOutput {
    pub(super) candidates: Vec<FusionHit>,
    pub(super) stats: GraphFrontierStats,
    pub(super) edge_confidence_by_cursor: HashMap<u64, f64>,
    pub(super) graph_origin_by_cursor: HashMap<u64, (GraphArmSeedSource, u32)>,
}

#[allow(clippy::too_many_arguments)] // Shared graph capture preserves the zero-cost default path.
pub(super) fn bfs_graph_arm_candidates<C: SearchOriginCapture>(
    tx: &Connection,
    fused_hits: &[FusionHit],
    fused_seed_ranking: &[FusionHit],
    match_expression: &str,
    max_depth: u32,
    cap: usize,
    view: FrozenView,
    filter: Option<&SearchFilter>,
    graph_options: &GraphArmOptions,
    explain: bool,
    capture: &mut C,
) -> rusqlite::Result<BfsGraphArmOutput> {
    let now_param = view.now_param();
    const SEED_FTS_N: usize = 10;
    const MAX_NEIGHBORS_PER_HOP: usize = 64;
    const SYNTHESIZED_PENALTY: f64 = 0.3;

    let seed_records: HashSet<RecordKey> = fused_hits.iter().map(FusionHit::key).collect();

    // Load declared edge `Filterable` projections once when edge predicates are active
    // and `!view.include_superseded` (sound CPV B-tree pre-filter + exact inline verification).
    let registry_owned =
        if !graph_options.graph_edge_predicates.is_empty() && !view.view.include_superseded {
            Some(load_projection_registry(tx)?)
        } else {
            None
        };
    let pushed_edge_predicates: Vec<&PropertyPredicate> =
        if let Some(registry) = registry_owned.as_ref() {
            graph_options
                .graph_edge_predicates
                .iter()
                .filter(|pred| {
                    registry.get(&pred.key).is_some_and(|stored| {
                        stored.entity_class == PropertyEntityClass::Edge
                            && stored.roles.contains(&ProjectionRole::Filterable)
                            && stored.source.is_none()
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

    let mut frontier: VecDeque<(String, u32, GraphArmSeedSource)> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut candidates: Vec<FusionHit> = Vec::new();
    let mut edge_confidence_by_cursor: HashMap<u64, f64> = HashMap::new();
    let mut graph_origin_by_cursor: HashMap<u64, (GraphArmSeedSource, u32)> = HashMap::new();
    let mut stats = GraphFrontierStats::default();

    {
        let active_existence = view.view.existence_sql("canonical_nodes");
        let active_validity = view.validity_sql("canonical_nodes", 2);
        let active_eligibility = dependency_closure::read_eligibility_sql(
            "canonical_nodes",
            view.view.include_superseded,
            view.view.include_inactive,
            view.view.include_out_of_window,
            2,
        );
        let mut active_params_template = vec![rusqlite::types::Value::Text(String::new())];
        if let Some(now) = now_param {
            active_params_template.push(rusqlite::types::Value::Integer(now));
        }
        let active_filter = filter::append_node_eligibility_sql_view(
            tx,
            filter,
            "canonical_nodes",
            &mut active_params_template,
            &view.view,
        )?;
        let mut active_stmt = tx.prepare(&format!(
            "SELECT kind, body, write_cursor, source_id, properties FROM canonical_nodes \
             WHERE logical_id = ?1{active_existence}\
             {active_validity}{active_eligibility}{active_filter} ORDER BY write_cursor DESC LIMIT 1"
        ))?;

        let mut seen_candidates: HashSet<String> = HashSet::new();
        let mut candidate_seeds: Vec<BfsCandidateSeed> = Vec::new();

        // Seed source 0 (opt-in): `FusedTopK` canonical nodes (`logical_id IS NOT NULL`),
        // enqueued FIRST in fixed depth-0 order `(FusedTopK, EdgeFts, EntityFts)`.
        if matches!(graph_options.graph_seed, GraphSeedMode::FusedTopK | GraphSeedMode::Both) {
            let mut fused_seeds_admitted = 0usize;
            for hit in fused_seed_ranking {
                if fused_seeds_admitted >= graph_options.graph_seed_k {
                    break;
                }
                if !matches!(hit.ownership, FusionOwnership::CanonicalNode)
                    || hit.hit.id.space != IdSpaceKind::Logical
                {
                    continue;
                }
                let lid = &hit.hit.id.value;
                if seen_candidates.contains(lid) {
                    continue;
                }
                let mut active_params = active_params_template.clone();
                active_params[0] = rusqlite::types::Value::Text(lid.clone());
                let eligible = active_stmt
                    .query_row(rusqlite::params_from_iter(active_params.iter()), |r| {
                        r.get::<_, i64>(2)
                    })
                    .optional()?
                    .is_some();
                if eligible && seen_candidates.insert(lid.clone()) {
                    stats.seeds_considered += 1;
                    stats.seeds_resolved += 1;
                    if visited.insert(lid.clone()) {
                        frontier.push_back((lid.clone(), 0, GraphArmSeedSource::FusedTopK));
                    }
                    fused_seeds_admitted += 1;
                }
            }
        }

        if matches!(graph_options.graph_seed, GraphSeedMode::GraphFts | GraphSeedMode::Both) {
            // Seed source A — edge-fact endpoints (primary). Project `from_eligible` and
            // `to_eligible` without `LIMIT 10` pre-truncating across mixed/duplicate/predicate-filtered
            // edges, push ONLY eligible endpoints, and count `SEED_FTS_N = 10` against distinct
            // newly admitted eligible edge-seed slots.
            let mut edge_seed_params: Vec<rusqlite::types::Value> = vec![
                rusqlite::types::Value::Text(match_expression.to_string()),
                rusqlite::types::Value::Integer(view.edge_now()),
            ];
            let endpoint_now_index = 3;
            if let Some(now) = now_param {
                edge_seed_params.push(rusqlite::types::Value::Integer(now));
            }
            let from_filter = filter::append_node_eligibility_sql_view(
                tx,
                filter,
                "ef",
                &mut edge_seed_params,
                &view.view,
            )?;
            let to_filter = filter::append_node_eligibility_sql_view(
                tx,
                filter,
                "et",
                &mut edge_seed_params,
                &view.view,
            )?;
            let edge_kinds_sql = append_graph_arm_edge_kinds_sql(
                "ce",
                &graph_options.graph_edge_kinds,
                &mut edge_seed_params,
            );
            let mut edge_cpv_sql = String::new();
            for (idx, pred) in pushed_edge_predicates.iter().enumerate() {
                append_cpv_exists_prefilter(
                    "edge",
                    "ce.write_cursor",
                    &format!("cpv_s{idx}"),
                    pred,
                    &mut edge_seed_params,
                    &mut edge_cpv_sql,
                );
            }
            let from_existence = view.view.existence_sql("ef");
            let to_existence = view.view.existence_sql("et");
            let edge_seed_existence = view.view.edge_existence_sql("ce");
            let from_validity = view.validity_sql("ef", endpoint_now_index);
            let to_validity = view.validity_sql("et", endpoint_now_index);
            let from_dependency = dependency_closure::read_eligibility_sql(
                "ef",
                view.view.include_superseded,
                view.view.include_inactive,
                view.view.include_out_of_window,
                endpoint_now_index,
            );
            let to_dependency = dependency_closure::read_eligibility_sql(
                "et",
                view.view.include_superseded,
                view.view.include_inactive,
                view.view.include_out_of_window,
                endpoint_now_index,
            );
            let edge_validity = edge_validity_sql_for_view("ce", 2, &view.view);
            if let Ok(mut edge_seed_stmt) = tx.prepare(&format!(
                "SELECT ce.from_id, ce.to_id, ce.source_id, ce.confidence, ce.write_cursor, \
                        ce.properties, \
                        EXISTS(SELECT 1 FROM canonical_nodes ef WHERE ef.logical_id=ce.from_id \
                            {from_existence}{from_validity}{from_dependency}{from_filter}) AS from_eligible, \
                        EXISTS(SELECT 1 FROM canonical_nodes et WHERE et.logical_id=ce.to_id \
                            {to_existence}{to_validity}{to_dependency}{to_filter}) AS to_eligible \
                 FROM search_index_edges sei \
                 JOIN canonical_edges ce ON ce.write_cursor = sei.write_cursor \
                 WHERE search_index_edges MATCH ?1 \
                   {edge_seed_existence}{edge_validity} \
                   AND (ce.temporal_fallback IS NULL OR ce.temporal_fallback = 0) \
                   {edge_kinds_sql}{edge_cpv_sql} \
                 ORDER BY bm25(search_index_edges), sei.write_cursor"
            )) {
                let mut rows =
                    edge_seed_stmt.query(rusqlite::params_from_iter(edge_seed_params.iter()))?;
                let mut distinct_edge_seeds = 0usize;
                while let Some(row) = rows.next()? {
                    let from_eligible: bool = row.get(6)?;
                    let to_eligible: bool = row.get(7)?;
                    if !from_eligible && !to_eligible {
                        continue;
                    }
                    if !graph_options.graph_edge_predicates.is_empty() {
                        let props = decode_properties_row_col(row, 5)?;
                        if !graph_options
                            .graph_edge_predicates
                            .iter()
                            .all(|pred| eval_property_predicate(props.as_ref(), pred))
                        {
                            continue;
                        }
                    }
                    let from_id: String = row.get(0)?;
                    let to_id: String = row.get(1)?;
                    let source_id: Option<String> = row.get(2)?;
                    let confidence: Option<f64> = row.get(3)?;
                    let edge_cursor = row.get::<_, i64>(4)? as u64;

                    if from_eligible
                        && distinct_edge_seeds < SEED_FTS_N
                        && seen_candidates.insert(from_id.clone())
                    {
                        candidate_seeds.push(BfsCandidateSeed {
                            logical_id: from_id,
                            source_id: source_id.clone(),
                            confidence,
                            graph_origin: Some(CapturedGraphOrigin::EdgeSeed { edge_cursor }),
                            seed_source: GraphArmSeedSource::EdgeFts,
                        });
                        distinct_edge_seeds += 1;
                    }
                    if to_eligible
                        && distinct_edge_seeds < SEED_FTS_N
                        && seen_candidates.insert(to_id.clone())
                    {
                        candidate_seeds.push(BfsCandidateSeed {
                            logical_id: to_id,
                            source_id,
                            confidence,
                            graph_origin: Some(CapturedGraphOrigin::EdgeSeed { edge_cursor }),
                            seed_source: GraphArmSeedSource::EdgeFts,
                        });
                        distinct_edge_seeds += 1;
                    }
                    if distinct_edge_seeds >= SEED_FTS_N {
                        break;
                    }
                }
            }

            // Seed source B — entity-node FTS (`logical_id IS NOT NULL`).
            {
                let seed_existence = view.view.existence_sql("cn");
                let seed_validity = view.validity_sql("cn", 3);
                let seed_eligibility = dependency_closure::read_eligibility_sql(
                    "cn",
                    view.view.include_superseded,
                    view.view.include_inactive,
                    view.view.include_out_of_window,
                    3,
                );
                let mut seed_params: Vec<rusqlite::types::Value> = vec![
                    rusqlite::types::Value::Text(match_expression.to_string()),
                    rusqlite::types::Value::Integer(SEED_FTS_N as i64),
                ];
                if let Some(now) = now_param {
                    seed_params.push(rusqlite::types::Value::Integer(now));
                }
                let seed_filter = filter::append_node_eligibility_sql_view(
                    tx,
                    filter,
                    "cn",
                    &mut seed_params,
                    &view.view,
                )?;
                let mut node_seed_stmt = tx.prepare(&format!(
                    "SELECT cn.logical_id, cn.source_id \
                     FROM search_index si \
                     JOIN canonical_nodes cn ON cn.write_cursor = si.write_cursor \
                     WHERE search_index MATCH ?1 \
                       {seed_existence} AND cn.logical_id IS NOT NULL\
                       {seed_validity}{seed_eligibility}{seed_filter} \
                     ORDER BY bm25(search_index), si.write_cursor \
                     LIMIT ?2"
                ))?;
                let rows = node_seed_stmt
                    .query_map(rusqlite::params_from_iter(seed_params.iter()), |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
                    })?;
                for pair in rows {
                    let (lid, source_id) = pair?;
                    if seen_candidates.insert(lid.clone()) {
                        candidate_seeds.push(BfsCandidateSeed {
                            logical_id: lid,
                            source_id,
                            confidence: None,
                            graph_origin: None,
                            seed_source: GraphArmSeedSource::EntityFts,
                        });
                    }
                }
            }
        }

        // Resolve + emit depth-0 seeds from EdgeFts / EntityFts.
        for BfsCandidateSeed {
            logical_id: lid,
            source_id,
            confidence: seed_confidence,
            graph_origin,
            seed_source,
        } in candidate_seeds
        {
            stats.seeds_considered += 1;
            let mut active_params = active_params_template.clone();
            active_params[0] = rusqlite::types::Value::Text(lid.clone());
            struct BfsActiveNodeRow {
                kind: String,
                body: String,
                write_cursor: i64,
                selected_source: Option<String>,
                properties: Option<PropertyMap>,
            }
            let row: Option<BfsActiveNodeRow> = active_stmt
                .query_row(rusqlite::params_from_iter(active_params.iter()), |r| {
                    Ok(BfsActiveNodeRow {
                        kind: r.get(0)?,
                        body: r.get(1)?,
                        write_cursor: r.get(2)?,
                        selected_source: r.get(3)?,
                        properties: decode_properties_row_col(r, 4)?,
                    })
                })
                .optional()?;
            if let Some(BfsActiveNodeRow {
                kind,
                body,
                write_cursor,
                selected_source,
                properties,
            }) = row
            {
                let source_id = if seed_source == GraphArmSeedSource::EntityFts {
                    selected_source
                } else {
                    source_id
                };
                stats.seeds_resolved += 1;
                if visited.insert(lid.clone()) {
                    let id = derive_stable_id(Some(&lid), &body);
                    frontier.push_back((lid, 0, seed_source));
                    if (graph_origin.is_some() || !capture.requires_edge_origin())
                        && !seed_records.contains(&RecordKey::CanonicalNode(write_cursor as u64))
                        && candidates.len() < cap
                    {
                        let score = if kind == "unknown" { SYNTHESIZED_PENALTY } else { 1.0 };
                        if let Some(c) = seed_confidence {
                            edge_confidence_by_cursor.insert(write_cursor as u64, c);
                        }
                        if explain {
                            graph_origin_by_cursor.insert(write_cursor as u64, (seed_source, 0));
                        }
                        if let Some(origin) = graph_origin {
                            capture.record_graph_origin(write_cursor as u64, origin);
                        }
                        candidates.push(FusionHit {
                            ownership: FusionOwnership::CanonicalNode,
                            hit: SearchHit {
                                id,
                                write_cursor: write_cursor as u64,
                                kind,
                                body,
                                score,
                                branch: SoftFallbackBranch::GraphArm,
                                source_id,
                                ce_score: None,
                                properties,
                            },
                        });
                    }
                }
            }
        }
    }
    stats.frontier_nonempty = !frontier.is_empty();

    // Phase 2: BFS over canonical_edges via indexed UNION ALL.
    let (edge_sql, edge_params_template) =
        build_graph_arm_phase2_query(tx, view, filter, graph_options, &pushed_edge_predicates)?;
    let mut edge_stmt = tx.prepare(&edge_sql)?;

    let body_existence = view.view.existence_sql("canonical_nodes");
    let body_validity = view.validity_sql("canonical_nodes", 2);
    let body_eligibility = dependency_closure::read_eligibility_sql(
        "canonical_nodes",
        view.view.include_superseded,
        view.view.include_inactive,
        view.view.include_out_of_window,
        2,
    );
    let mut body_params_template = vec![rusqlite::types::Value::Text(String::new())];
    if let Some(now) = now_param {
        body_params_template.push(rusqlite::types::Value::Integer(now));
    }
    let body_filter = filter::append_node_eligibility_sql_view(
        tx,
        filter,
        "canonical_nodes",
        &mut body_params_template,
        &view.view,
    )?;
    let mut body_stmt = tx.prepare(&format!(
        "SELECT kind, body, write_cursor, properties FROM canonical_nodes \
         WHERE logical_id = ?1{body_existence}\
         {body_validity}{body_eligibility}{body_filter} \
         ORDER BY write_cursor DESC LIMIT 1"
    ))?;

    'frontier: while let Some((lid, depth, seed_source)) = frontier.pop_front() {
        if depth >= max_depth {
            continue;
        }

        let neighbors: Vec<(String, Option<String>, Option<f64>, u64)> = {
            let mut edge_params = edge_params_template.clone();
            edge_params[0] = rusqlite::types::Value::Text(lid.clone());
            let mut rows = edge_stmt.query(rusqlite::params_from_iter(edge_params.iter()))?;
            let mut out = Vec::new();
            while let Some(row) = rows.next()? {
                // Always verify inline `graph_edge_predicates` BEFORE `visited.insert(neighbor)`
                // and BEFORE counting toward the 64-neighbor-per-hop cap.
                if !graph_options.graph_edge_predicates.is_empty() {
                    let props = decode_properties_row_col(row, 5)?;
                    if !graph_options
                        .graph_edge_predicates
                        .iter()
                        .all(|pred| eval_property_predicate(props.as_ref(), pred))
                    {
                        continue;
                    }
                }
                let from_id: String = row.get(0)?;
                let to_id: String = row.get(1)?;
                let source_id: Option<String> = row.get(2)?;
                let confidence: Option<f64> = row.get(3)?;
                let edge_cursor = row.get::<_, i64>(4)? as u64;
                let neighbor = if from_id == lid { to_id } else { from_id };
                out.push((neighbor, source_id, confidence, edge_cursor));
                if out.len() >= MAX_NEIGHBORS_PER_HOP {
                    break;
                }
            }
            out
        };

        for (neighbor, edge_source_id, edge_confidence, edge_cursor) in neighbors {
            if visited.contains(&neighbor) {
                continue;
            }
            visited.insert(neighbor.clone());

            let mut body_params = body_params_template.clone();
            body_params[0] = rusqlite::types::Value::Text(neighbor.clone());
            let row: Option<(String, String, i64, Option<PropertyMap>)> = body_stmt
                .query_row(rusqlite::params_from_iter(body_params.iter()), |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        decode_properties_row_col(row, 3)?,
                    ))
                })
                .optional()?;

            if let Some((kind, body, write_cursor, properties)) = row {
                if !seed_records.contains(&RecordKey::CanonicalNode(write_cursor as u64)) {
                    if candidates.len() >= cap {
                        stats.bound_reached = true;
                        break 'frontier;
                    }
                    let hop_count = depth + 1;
                    let hop_score = 1.0 / (1.0 + hop_count as f64);
                    let score =
                        if kind == "unknown" { hop_score * SYNTHESIZED_PENALTY } else { hop_score };
                    let id = derive_stable_id(Some(&neighbor), &body);
                    if let Some(c) = edge_confidence {
                        edge_confidence_by_cursor.insert(write_cursor as u64, c);
                    }
                    if explain {
                        graph_origin_by_cursor
                            .insert(write_cursor as u64, (seed_source, hop_count));
                    }
                    capture.record_graph_origin(
                        write_cursor as u64,
                        CapturedGraphOrigin::Traversal { edge_cursor, hop_count, seed_source },
                    );
                    candidates.push(FusionHit {
                        ownership: FusionOwnership::CanonicalNode,
                        hit: SearchHit {
                            id,
                            write_cursor: write_cursor as u64,
                            kind,
                            body,
                            score,
                            branch: SoftFallbackBranch::GraphArm,
                            source_id: edge_source_id.clone(),
                            ce_score: None,
                            properties,
                        },
                    });
                }
                frontier.push_back((neighbor, depth + 1, seed_source));
            }
        }
    }

    drop(edge_stmt);
    drop(body_stmt);
    stats.graph_candidates_emitted = candidates.len() as u32;
    Ok(BfsGraphArmOutput { candidates, stats, edge_confidence_by_cursor, graph_origin_by_cursor })
}
