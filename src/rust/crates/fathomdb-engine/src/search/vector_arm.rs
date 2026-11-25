use super::fts_fast_path::{prepare_search_statement, search_body_from_row};
use super::SearchReaderError;
use crate::filter::{build_vector_phase1_plan, SearchFilter};
use crate::fusion::{FusionHit, FusionOwnership};
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
use rusqlite::{params, Connection, OptionalExtension};

pub(super) fn historical_dense_incomplete(
    tx: &Connection,
    view: FrozenView,
) -> rusqlite::Result<bool> {
    let node_predicate = view.node_sql("n", 1);
    let edge_predicate = format!(
        "{}{}",
        view.view.edge_existence_sql("e"),
        edge_validity_sql_for_view("e", 1, &view.view)
    );
    let declared = vector_projection_declared(tx)?;
    let mut stmt = tx.prepare(&format!(
        "SELECT n.write_cursor,n.kind,0 FROM canonical_nodes n
        WHERE n.row_kind IN ('leaf','coverage') {node_predicate}
        AND (?2 OR EXISTS(SELECT 1 FROM _fathomdb_vector_kinds vk WHERE vk.kind=n.kind))
        UNION ALL SELECT e.write_cursor,'edge_fact',1 FROM canonical_edges e
        WHERE e.body IS NOT NULL {edge_predicate}"
    ))?;
    let rows = stmt.query_map(params![view.edge_now(), declared], |r| {
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
    read_dependency_eligibility: &dyn Fn(&str, usize) -> String,
) -> Result<Vec<FusionHit>, SearchReaderError> {
    let Some(query_vector) = query_vector.filter(|_| !vector_eligibility_degraded) else {
        #[cfg(any(test, feature = "slice30-observer"))]
        crate::slice30_observer::not_run("vector_retrieval", "vector_unavailable_or_ineligible");
        return Ok(Vec::new());
    };
    #[cfg(test)]
    let rank_probe = crate::slice30_observer::rank_probe_route();
    #[cfg(any(test, feature = "slice30-observer"))]
    let vector_observation = crate::slice30_observer::span("vector_retrieval");
    let mut rowids = Vec::new();
    let bin_vector = query_vector_bin.unwrap_or(query_vector);
    {
        #[cfg(any(test, feature = "slice30-observer"))]
        let sql_observation = crate::slice30_observer::span("vector_phase1_f32_sql");
        let candidate_limit = candidate_limit.max(final_limit).max(TOP_K_BIT_CANDIDATES);
        let (sql, tail_params) = build_vector_phase1_plan(
            Some(tx),
            filter,
            candidate_limit,
            &view.view,
            view.edge_now(),
        )?;
        let mut params: Vec<rusqlite::types::Value> = vec![
            rusqlite::types::Value::Text(bin_vector.to_string()),
            rusqlite::types::Value::Text(query_vector.to_string()),
        ];
        params.extend(tail_params);
        let mut statement = prepare_search_statement(tx, &sql)?;
        let rows = statement.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
        })?;
        for row in rows {
            rowids.push(row?);
        }
        #[cfg(test)]
        if rank_probe.is_some() {
            let candidates: Vec<_> =
                rowids.iter().map(|(rowid, distance)| (rowid, distance.to_bits())).collect();
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
    let node_existence = view.view.existence_sql("canonical_nodes");
    let node_validity = view.validity_sql("canonical_nodes", 2);
    let node_eligibility = read_dependency_eligibility("canonical_nodes", 2);
    let mut node_stmt = prepare_search_statement(
        tx,
        &format!(
            "SELECT kind, body, logical_id, source_id, properties FROM canonical_nodes \
         WHERE write_cursor = ?1{node_existence}\
         {node_validity}{node_eligibility} LIMIT 1"
        ),
    )?;
    let edge_existence = view.view.edge_existence_sql("canonical_edges");
    let edge_validity = edge_validity_sql_for_view("canonical_edges", 2, &view.view);
    let edge_eligibility = read_dependency_eligibility("canonical_edges", 2);
    let mut edge_stmt = prepare_search_statement(
        tx,
        &format!(
            "SELECT body, logical_id, source_id, properties FROM canonical_edges \
         WHERE write_cursor = ?1 AND body IS NOT NULL{edge_existence}\
         {edge_validity}{edge_eligibility} LIMIT 1"
        ),
    )?;
    let node_params = |rowid: i64| -> Vec<rusqlite::types::Value> {
        let mut p = vec![rusqlite::types::Value::Integer(rowid)];
        if let Some(now) = now_param {
            p.push(rusqlite::types::Value::Integer(now));
        }
        p
    };
    for (rowid, score) in rowids {
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
            results.push(FusionHit {
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
            });
        } else if let Some((body, logical_id, source_id, properties)) = edge_stmt
            .query_row(rusqlite::params![rowid, view.edge_now()], |row| {
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
            results.push(FusionHit {
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
            });
        }
    }
    #[cfg(any(test, feature = "slice30-observer"))]
    {
        hydration_observation.counted_success(results.len());
        vector_observation.counted_success(results.len());
    }
    Ok(results)
}

pub(super) fn detect_vector_soft_fallback(
    tx: &Connection,
    compiled: &CompiledQuery,
    query_vector: Option<&str>,
    vector_eligibility_degraded: bool,
    vector_rows_visible: bool,
) -> rusqlite::Result<Option<SoftFallback>> {
    #[cfg(any(test, feature = "slice30-observer"))]
    let observation = crate::slice30_observer::span("source_fallback_selection");
    #[cfg(any(test, feature = "slice30-observer"))]
    let mut probe_attempted = false;
    let selected = if vector_eligibility_degraded {
        Some(SoftFallback { branch: SoftFallbackBranch::Vector })
    } else if query_vector.is_some() && !vector_rows_visible {
        #[cfg(any(test, feature = "slice30-observer"))]
        {
            probe_attempted = true;
        }
        tx.query_row(
            "SELECT 1
             FROM search_index
             JOIN _fathomdb_vector_kinds ON _fathomdb_vector_kinds.kind = search_index.kind
             LEFT JOIN _fathomdb_projection_terminal
               ON _fathomdb_projection_terminal.write_cursor = search_index.write_cursor
             WHERE search_index MATCH ?1
              AND _fathomdb_projection_terminal.write_cursor IS NULL
             LIMIT 1",
            [compiled.match_expression.as_str()],
            |_row| Ok(SoftFallback { branch: SoftFallbackBranch::Vector }),
        )
        .optional()?
    } else {
        None
    };
    #[cfg(any(test, feature = "slice30-observer"))]
    observation.boolean_result(&Ok::<bool, rusqlite::Error>(selected.is_some()), probe_attempted);
    Ok(selected)
}
