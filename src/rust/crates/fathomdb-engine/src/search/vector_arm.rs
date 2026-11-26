use super::fts_fast_path::{prepare_search_statement, search_body_from_row};
use super::SearchReaderError;
use crate::filter::{build_vector_phase1_plan, SearchFilter};
use crate::fusion::{FusionHit, FusionOwnership, RecordKey};
use crate::identity::derive_stable_id;
use crate::projection_commit::projection_physical_tuple_is_empty;
use crate::projection_generation::{member_completion, Completion, ProjectionRuntimeStateV1};
use crate::projection_registry::vector_projection_declared;
use crate::properties::decode_properties_row_col;
use crate::search_types::{
    SearchHit, SoftFallback, SoftFallbackBranch, TOP_K_BIT_CANDIDATES, VECTOR_FUSION_POOL,
};
use crate::temporal::{edge_validity_sql_for_view, FrozenView};
use crate::vector_storage::kind_is_vector_committable_on;
use fathomdb_query::CompiledQuery;
use rusqlite::{Connection, OptionalExtension};
use std::collections::HashMap;

pub(super) struct VectorArmOutput {
    pub(super) hits: Vec<FusionHit>,
    pub(super) cosines: HashMap<RecordKey, Option<f64>>,
}

pub(crate) fn cosine_similarity_from_distance(distance: Option<f64>) -> Option<f64> {
    let d = distance?;
    if !d.is_finite() {
        return None;
    }
    let sim = 1.0 - d;
    if !sim.is_finite() {
        return None;
    }
    let clamped = sim.clamp(-1.0, 1.0);
    Some(if clamped == 0.0 { 0.0 } else { clamped })
}

pub(super) fn historical_dense_incomplete(
    tx: &Connection,
    scope: &crate::scope::ScopePredicate,
    view: FrozenView,
) -> rusqlite::Result<bool> {
    let node_predicate = view.node_sql("n", 1);
    let edge_predicate = format!(
        "{}{}",
        view.view.edge_existence_sql("e"),
        edge_validity_sql_for_view("e", 1, &view.view)
    );
    let declared = vector_projection_declared(tx)?;
    let mut binds = vec![
        rusqlite::types::Value::Integer(view.edge_now()),
        rusqlite::types::Value::Integer(i64::from(declared)),
    ];
    let mut node_scope_sql = String::new();
    scope.append_sql_values("n.scope", &mut node_scope_sql, &mut binds);
    let mut edge_scope_sql = String::new();
    scope.append_sql_values("e.scope", &mut edge_scope_sql, &mut binds);
    let mut stmt = tx.prepare(&format!(
        "SELECT n.write_cursor,n.kind,0 FROM canonical_nodes n
        WHERE n.row_kind IN ('leaf','coverage') AND ({node_scope_sql}){node_predicate}
        AND (?2 OR EXISTS(SELECT 1 FROM _fathomdb_vector_kinds vk WHERE vk.kind=n.kind))
        UNION ALL SELECT e.write_cursor,'edge_fact',1 FROM canonical_edges e
        WHERE e.body IS NOT NULL AND ({edge_scope_sql}){edge_predicate}"
    ))?;
    let rows = stmt.query_map(rusqlite::params_from_iter(binds.iter()), |r| {
        Ok((r.get::<_, u64>(0)?, r.get::<_, String>(1)?, r.get::<_, bool>(2)?))
    })?;
    let mut incomplete = false;
    for row in rows {
        let (cursor, kind, edge) = row?;
        if !edge && !kind_is_vector_committable_on(tx, &kind) {
            continue;
        }
        if projection_physical_tuple_is_empty(tx, cursor)? {
            incomplete = true;
            continue;
        }
        let completion =
            member_completion(tx, cursor, &kind, edge, ProjectionRuntimeStateV1::Usable)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
        if completion != Completion::Complete {
            incomplete = true;
        }
    }
    Ok(incomplete)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_vector_arm(
    tx: &rusqlite::Transaction<'_>,
    query_vector: Option<&str>,
    query_vector_bin: Option<&str>,
    vector_eligibility_degraded: bool,
    final_limit: usize,
    candidate_limit: usize,
    filter: Option<&SearchFilter>,
    view: FrozenView,
    now_param: Option<i64>,
    scope: &crate::scope::ScopePredicate,
    read_dependency_eligibility: &dyn Fn(&str, usize) -> String,
    explain: bool,
) -> Result<VectorArmOutput, SearchReaderError> {
    let Some(query_vector) = query_vector.filter(|_| !vector_eligibility_degraded) else {
        #[cfg(any(test, feature = "slice30-observer"))]
        crate::slice30_observer::not_run("vector_retrieval", "vector_unavailable_or_ineligible");
        return Ok(VectorArmOutput { hits: Vec::new(), cosines: HashMap::new() });
    };
    #[cfg(test)]
    let rank_probe = crate::slice30_observer::rank_probe_route();
    #[cfg(any(test, feature = "slice30-observer"))]
    let vector_observation = crate::slice30_observer::span("vector_retrieval");
    let mut rowids: Vec<(i64, f64, Option<f64>)> = Vec::new();
    let bin_vector = query_vector_bin.unwrap_or(query_vector);
    {
        #[cfg(any(test, feature = "slice30-observer"))]
        let sql_observation = crate::slice30_observer::span("vector_phase1_f32_sql");
        let candidate_limit = candidate_limit.max(final_limit).max(TOP_K_BIT_CANDIDATES);
        let run_phase1_for = |phase_scope: &crate::scope::ScopePredicate| -> Result<Vec<(i64, f64, Option<f64>)>, rusqlite::Error> {
            let (sql, tail_params) = build_vector_phase1_plan(
                Some(tx),
                filter,
                candidate_limit,
                &view.view,
                view.edge_now(),
                phase_scope,
                explain,
            )?;
            let mut params: Vec<rusqlite::types::Value> = vec![
                rusqlite::types::Value::Text(bin_vector.to_string()),
                rusqlite::types::Value::Text(query_vector.to_string()),
            ];
            params.extend(tail_params);
            let mut statement = prepare_search_statement(tx, &sql)?;
            let rows = statement.query_map(rusqlite::params_from_iter(params.iter()), |row| {
                let cos_dist = if explain { row.get::<_, Option<f64>>(2)? } else { None };
                Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?, cos_dist))
            })?;
            rows.collect()
        };
        if scope.read_scopes().len() > 1
            && !crate::filter::filter_needs_preselection(Some(tx), filter, &view.view)?
        {
            for single in scope.read_scopes() {
                let single_scope = crate::scope::ScopePredicate::single_for_query(single);
                rowids.extend(run_phase1_for(&single_scope)?);
            }
            rowids.sort_by(|left, right| {
                left.1.total_cmp(&right.1).then_with(|| left.0.cmp(&right.0))
            });
            rowids.truncate(candidate_limit);
        } else {
            rowids = run_phase1_for(scope)?;
        }
        #[cfg(test)]
        if rank_probe.is_some() {
            let candidates: Vec<_> =
                rowids.iter().map(|(rowid, distance, _)| (rowid, distance.to_bits())).collect();
            crate::slice30_observer::record_rank_probe(
                "s50_vector_candidates",
                serde_json::json!(candidates),
            );
        }
        #[cfg(any(test, feature = "slice30-observer"))]
        sql_observation.counted_success(rowids.len());
    }
    #[cfg(any(test, feature = "slice30-observer"))]
    let hydration_observation = crate::slice30_observer::span("vector_hydration");
    let mut results = Vec::new();
    let mut cosines: HashMap<RecordKey, Option<f64>> = HashMap::new();
    let node_existence = view.view.existence_sql("canonical_nodes");
    let node_validity = view.validity_sql("canonical_nodes", 2);
    let node_eligibility = read_dependency_eligibility("canonical_nodes", 2);
    let node_scope_start = if now_param.is_some() { 3 } else { 2 };
    let (node_scope_sql, _) = scope.sql_clause("scope", node_scope_start);
    let mut node_stmt = prepare_search_statement(
        tx,
        &format!(
            "SELECT kind, body, logical_id, source_id, properties FROM canonical_nodes \
         WHERE write_cursor = ?1 AND {node_scope_sql}{node_existence}\
         {node_validity}{node_eligibility} LIMIT 1"
        ),
    )?;
    let edge_existence = view.view.edge_existence_sql("canonical_edges");
    let edge_validity = edge_validity_sql_for_view("canonical_edges", 2, &view.view);
    let edge_eligibility = read_dependency_eligibility("canonical_edges", 2);
    let (edge_scope_sql, _) = scope.sql_clause("scope", 3);
    let mut edge_stmt = prepare_search_statement(
        tx,
        &format!(
            "SELECT body, logical_id, source_id, properties FROM canonical_edges \
         WHERE write_cursor = ?1 AND {edge_scope_sql} AND body IS NOT NULL{edge_existence}\
         {edge_validity}{edge_eligibility} LIMIT 1"
        ),
    )?;
    let node_params = |rowid: i64| -> Vec<rusqlite::types::Value> {
        let mut p = vec![rusqlite::types::Value::Integer(rowid)];
        if let Some(now) = now_param {
            p.push(rusqlite::types::Value::Integer(now));
        }
        scope.push_values(&mut p);
        p
    };
    for (rowid, score, cos_dist) in rowids {
        if results.len() >= final_limit.max(VECTOR_FUSION_POOL) {
            break;
        }
        if let Some((kind, body, logical_id, source_id, properties)) = node_stmt
            .query_row(rusqlite::params_from_iter(node_params(rowid)), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    search_body_from_row(row, 1, true)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    decode_properties_row_col(row, 4)?,
                ))
            })
            .optional()?
        {
            let id = derive_stable_id(logical_id.as_deref(), &body);
            let hit = FusionHit {
                ownership: FusionOwnership::CanonicalNode,
                hit: SearchHit {
                    id,
                    write_cursor: rowid as u64,
                    kind,
                    body,
                    score,
                    branch: SoftFallbackBranch::Vector,
                    source_id,
                    ce_score: None,
                    properties,
                },
            };
            if explain {
                cosines
                    .entry(hit.key())
                    .or_insert_with(|| cosine_similarity_from_distance(cos_dist));
            }
            results.push(hit);
        } else {
            let mut edge_params = vec![
                rusqlite::types::Value::Integer(rowid),
                rusqlite::types::Value::Integer(view.edge_now()),
            ];
            scope.push_values(&mut edge_params);
            if let Some((body, logical_id, source_id, properties)) = edge_stmt
                .query_row(rusqlite::params_from_iter(edge_params.iter()), |row| {
                    Ok((
                        search_body_from_row(row, 0, true)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        decode_properties_row_col(row, 3)?,
                    ))
                })
                .optional()?
            {
                let id = derive_stable_id(logical_id.as_deref(), &body);
                let hit = FusionHit {
                    ownership: FusionOwnership::CanonicalEdge,
                    hit: SearchHit {
                        id,
                        write_cursor: rowid as u64,
                        kind: "edge_fact".to_string(),
                        body,
                        score,
                        branch: SoftFallbackBranch::TextEdge,
                        source_id,
                        ce_score: None,
                        properties,
                    },
                };
                if explain {
                    cosines
                        .entry(hit.key())
                        .or_insert_with(|| cosine_similarity_from_distance(cos_dist));
                }
                results.push(hit);
            }
        }
    }
    #[cfg(any(test, feature = "slice30-observer"))]
    {
        hydration_observation.counted_success(results.len());
        vector_observation.counted_success(results.len());
    }
    Ok(VectorArmOutput { hits: results, cosines })
}

pub(super) fn detect_vector_soft_fallback(
    tx: &Connection,
    compiled: &CompiledQuery,
    query_vector: Option<&str>,
    vector_eligibility_degraded: bool,
    vector_rows_visible: bool,
    has_vector_kinds: bool,
    scope: &crate::scope::ScopePredicate,
) -> rusqlite::Result<Option<SoftFallback>> {
    #[cfg(any(test, feature = "slice30-observer"))]
    let observation = crate::slice30_observer::span("source_fallback_selection");
    #[cfg(any(test, feature = "slice30-observer"))]
    let mut probe_attempted = false;
    let selected = if vector_eligibility_degraded {
        Some(SoftFallback { branch: SoftFallbackBranch::Vector })
    } else if query_vector.is_some() && !vector_rows_visible && has_vector_kinds {
        #[cfg(any(test, feature = "slice30-observer"))]
        {
            probe_attempted = true;
        }
        let mut params = vec![rusqlite::types::Value::Text(compiled.match_expression.clone())];
        let mut scope_sql = String::new();
        scope.append_sql_values("cn.scope", &mut scope_sql, &mut params);
        let sql = format!(
            "SELECT 1
             FROM search_index
             JOIN canonical_nodes cn ON cn.write_cursor = search_index.write_cursor
             JOIN _fathomdb_vector_kinds ON _fathomdb_vector_kinds.kind = search_index.kind
             LEFT JOIN _fathomdb_projection_terminal
               ON _fathomdb_projection_terminal.write_cursor = search_index.write_cursor
             WHERE search_index MATCH ?1
              AND {scope_sql}
              AND _fathomdb_projection_terminal.write_cursor IS NULL
             LIMIT 1"
        );
        tx.query_row(&sql, rusqlite::params_from_iter(params.iter()), |_row| {
            Ok(SoftFallback { branch: SoftFallbackBranch::Vector })
        })
        .optional()?
    } else {
        None
    };
    #[cfg(any(test, feature = "slice30-observer"))]
    observation.boolean_result(&Ok::<bool, rusqlite::Error>(selected.is_some()), probe_attempted);
    Ok(selected)
}

#[cfg(test)]
#[test]
fn cosine_similarity_from_distance_contract_and_sqlite_vec_truth_table() {
    // 1. Pure helper edge cases
    assert_eq!(cosine_similarity_from_distance(Some(0.0)), Some(1.0));
    let zero = cosine_similarity_from_distance(Some(1.0)).unwrap();
    assert_eq!(zero, 0.0);
    assert!(!zero.is_sign_negative(), "1.0 - 1.0 must normalize to +0.0");
    assert_eq!(cosine_similarity_from_distance(Some(2.0)), Some(-1.0));
    assert_eq!(cosine_similarity_from_distance(Some(-0.0001)), Some(1.0));
    assert_eq!(cosine_similarity_from_distance(Some(2.0001)), Some(-1.0));
    assert_eq!(cosine_similarity_from_distance(None), None);
    assert_eq!(cosine_similarity_from_distance(Some(f64::NAN)), None);
    assert_eq!(cosine_similarity_from_distance(Some(f64::INFINITY)), None);
    assert_eq!(cosine_similarity_from_distance(Some(f64::NEG_INFINITY)), None);

    // 2. Direct sqlite-vec = "=0.1.9" vec_distance_cosine truth table
    let conn = crate::vector_storage::slice70_mapping_fixture(false);
    let query_cos = |a: &str, b: &str| -> Option<f64> {
        let raw: Option<f64> = conn
            .query_row(
                "SELECT vec_distance_cosine(vec_f32(?1), vec_f32(?2))",
                rusqlite::params![a, b],
                |row| row.get(0),
            )
            .unwrap();
        cosine_similarity_from_distance(raw)
    };

    // Identical unit vectors -> ~1.0
    let ident = query_cos("[1,0,0,0,0,0,0,0]", "[1,0,0,0,0,0,0,0]").unwrap();
    assert!((ident - 1.0).abs() < 1e-6, "identical: {ident}");

    // Orthogonal vectors -> +0.0 (never -0.0)
    let ortho = query_cos("[1,0,0,0,0,0,0,0]", "[0,1,0,0,0,0,0,0]").unwrap();
    assert!(ortho.abs() < 1e-6, "orthogonal: {ortho}");
    if ortho == 0.0 {
        assert!(!ortho.is_sign_negative(), "orthogonal zero must be +0.0");
    }

    // Opposite vectors -> ~-1.0
    let opp = query_cos("[1,0,0,0,0,0,0,0]", "[-1,0,0,0,0,0,0,0]").unwrap();
    assert!((opp + 1.0).abs() < 1e-6, "opposite: {opp}");

    // Non-unit-norm scaled vectors -> ~1.0
    let scaled = query_cos("[3,4,0,0,0,0,0,0]", "[6,8,0,0,0,0,0,0]").unwrap();
    assert!((scaled - 1.0).abs() < 1e-6, "scaled non-unit: {scaled}");

    // Zero-norm stored or query vector -> None (never NaN, never 0.0)
    assert_eq!(query_cos("[0,0,0,0,0,0,0,0]", "[1,0,0,0,0,0,0,0]"), None);
    assert_eq!(query_cos("[1,0,0,0,0,0,0,0]", "[0,0,0,0,0,0,0,0]"), None);
    assert_eq!(query_cos("[0,0,0,0,0,0,0,0]", "[0,0,0,0,0,0,0,0]"), None);
}
