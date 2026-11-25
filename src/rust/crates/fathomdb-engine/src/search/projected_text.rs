use super::fts_fast_path::collect_complete_rank_boundary;
#[cfg(feature = "test-hooks")]
use super::fts_fast_path::{record_fts_query_plan_for_test, record_fts_route_for_test};
use super::{open_historical_property_corpus, ProjectedTextReaderResponse, SearchReaderError};
use crate::dependency_closure;
use crate::filter::{
    self, property_fts_cohort_stream_sql, property_fts_rank_sql, property_fts_rank_stream_sql,
    validate_filter_attributes_on_snapshot, SearchFilter,
};
use crate::identity::derive_stable_id;
use crate::projection_commit::load_projection_cursor;
use crate::projection_registry::{
    extract_scalar_attribute_with_properties, load_projection_registry, StoredProjection,
};
use crate::properties::{decode_properties_row_col, PropertyEntityClass, PropertyMap};
use crate::reader_transaction::begin_attributed_reader_tx;
use crate::search_types::{SearchHit, SearchResult, SoftFallbackBranch};
use crate::temporal::{FrozenView, ReadView};
use crate::wal_attribution::WalAttributionCollector;
use fathomdb_query::{compile_text_query_with, StopWordSet};
use rusqlite::{params, Connection};
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

/// Read projection cursor and matching body rows inside one read tx.
#[allow(clippy::too_many_arguments)]
pub(crate) fn read_projected_text_in_tx(
    reader: &mut Connection,
    query: &str,
    name: &str,
    filter: Option<&SearchFilter>,
    limit: usize,
    view: ReadView,
    scope: &crate::scope::ScopePredicate,
    stop_words: &StopWordSet,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> ProjectedTextReaderResponse {
    let compiled = compile_text_query_with(query, stop_words);
    let frozen = view.freeze();
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)?;
    let registry = load_projection_registry(&tx)?;
    let declared =
        registry.get(&(PropertyEntityClass::Node, name.to_string())).ok_or_else(|| {
            SearchReaderError::InvalidFilter(format!(
                "projected text field {name:?} is not declared"
            ))
        })?;
    if !declared.wants_property_fts() {
        return Err(SearchReaderError::InvalidFilter(format!(
            "projected text field {name:?} is not a declared `searchable` projection with property FTS"
        )));
    }
    if let Some(filter) = filter {
        validate_filter_attributes_on_snapshot(&tx, filter)?;
    }
    if view.include_superseded || view.include_inactive {
        return read_historical_projected_text(
            &tx,
            &compiled.match_expression,
            name,
            filter,
            limit,
            frozen,
            scope,
            &registry,
            attribution,
        );
    }
    let mut node_predicate = frozen.node_sql("n", 3);
    let mut params = vec![
        rusqlite::types::Value::Text(name.to_string()),
        rusqlite::types::Value::Text(compiled.match_expression),
    ];
    if let Some(now) = frozen.now_param() {
        params.push(rusqlite::types::Value::Integer(now));
    }
    let mut scope_sql = String::new();
    scope.append_sql_values("n.scope", &mut scope_sql, &mut params);
    node_predicate.push_str(&format!(" AND {scope_sql}"));
    let filter_predicate =
        filter::append_node_eligibility_sql_view(&tx, filter, "n", &mut params, &view)?;
    #[cfg(feature = "test-hooks")]
    let force_full_sort =
        std::env::var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST").is_ok_and(|value| value == "1");
    #[cfg(not(feature = "test-hooks"))]
    let force_full_sort = false;
    if !force_full_sort {
        if let Some(results) = read_projected_cohort_stream(
            &tx,
            filter,
            &node_predicate,
            &filter_predicate,
            &params,
            scope,
            limit,
        )? {
            let projection_cursor = load_projection_cursor(&tx)?;
            return Ok(SearchResult {
                projection_cursor,
                soft_fallback: None,
                results,
                explanation: None,
                degradations: Vec::new(),
                snippets: None,
                hints: None,
            });
        }
    }
    // Like the body path's rank stream: FTS5 yields rows in bm25 order, so only
    // rows up to the limit's tie group are joined and read, instead of sorting
    // every match with its body. The full sort stays as the fallback.
    let streamed = if force_full_sort {
        None
    } else {
        #[cfg(feature = "test-hooks")]
        let rank_mapping = if std::env::var_os("FATHOMDB_FTS_FAIL_STREAM_FOR_TEST").is_some() {
            "missing_ranker()"
        } else {
            "bm25()"
        };
        #[cfg(not(feature = "test-hooks"))]
        let rank_mapping = "bm25()";
        let stream_sql =
            property_fts_rank_stream_sql(&node_predicate, &filter_predicate, rank_mapping);
        #[cfg(feature = "test-hooks")]
        record_fts_query_plan_for_test(&tx, &stream_sql, &params);
        tx.prepare(&stream_sql)
            .and_then(|mut statement| {
                let mut rows = statement.query(rusqlite::params_from_iter(params.iter()))?;
                collect_complete_rank_boundary(&mut rows, limit, None)
            })
            .ok()
    };
    if let Some((mut results, _crossed_boundary_tie)) = streamed {
        #[cfg(feature = "test-hooks")]
        record_fts_route_for_test(if _crossed_boundary_tie {
            "projected_rank_stream_tie_completed"
        } else {
            "projected_rank_stream_strict_boundary"
        });
        for hit in &mut results {
            hit.score = -hit.score;
        }
        let projection_cursor = load_projection_cursor(&tx)?;
        return Ok(SearchResult {
            projection_cursor,
            soft_fallback: None,
            results,
            explanation: None,
            degradations: Vec::new(),
            snippets: None,
            hints: None,
        });
    }
    #[cfg(feature = "test-hooks")]
    record_fts_route_for_test(if force_full_sort {
        "projected_full_sort_forced"
    } else {
        "projected_full_sort_fallback"
    });
    let sql = property_fts_rank_sql(&node_predicate, &filter_predicate);
    #[cfg(feature = "test-hooks")]
    record_fts_query_plan_for_test(&tx, &sql, &params);
    let mut stmt = tx.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            decode_properties_row_col(row, 6)?,
        ))
    })?;
    let mut results = Vec::new();
    for row in rows {
        let (cursor, bm25, kind, body, logical_id, source_id, properties) = row?;
        results.push(SearchHit {
            id: derive_stable_id(logical_id.as_deref(), &body),
            write_cursor: cursor as u64,
            kind,
            body,
            score: -bm25,
            branch: SoftFallbackBranch::Text,
            source_id,
            ce_score: None,
            properties,
        });
        if results.len() >= limit {
            break;
        }
    }
    let projection_cursor = load_projection_cursor(&tx)?;
    Ok(SearchResult {
        projection_cursor,
        soft_fallback: None,
        results,
        explanation: None,
        degradations: Vec::new(),
        snippets: None,
        hints: None,
    })
}

#[allow(clippy::too_many_arguments)] // snapshot corpus and test-only reader rendezvous
fn read_historical_projected_text(
    tx: &Connection,
    match_expression: &str,
    name: &str,
    filter: Option<&SearchFilter>,
    limit: usize,
    view: FrozenView,
    scope: &crate::scope::ScopePredicate,
    registry: &BTreeMap<(PropertyEntityClass, String), StoredProjection>,
    _attribution: &WalAttributionCollector,
) -> ProjectedTextReaderResponse {
    // The caller holds one main-store snapshot through registry, corpus and
    // winner hydration. The private database and its temp sorters are memory
    // only and are dropped before the pooled reader sends success or error.
    let mut corpus = open_historical_property_corpus()?;
    let mut params = vec![rusqlite::types::Value::Integer(view.edge_now())];
    let mut elig_scope_sql = String::new();
    scope.append_sql_values("n.scope", &mut elig_scope_sql, &mut params);
    let eligibility =
        filter::append_node_eligibility_sql_view(tx, filter, "n", &mut params, &view.view)?;
    let predicate = view.node_sql("n", 1);
    let eligible: HashSet<u64> = {
        let mut stmt = tx.prepare(&format!(
            "SELECT n.write_cursor FROM canonical_nodes n \
             WHERE ?1 IS NOT NULL AND {elig_scope_sql}{predicate}{eligibility}"
        ))?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |r| r.get(0))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    let corpus_tx = corpus.transaction()?;
    {
        let mut insert = corpus_tx.prepare(
            "INSERT INTO history_property(attr_value,attr_name,write_cursor,eligible) VALUES(?1,?2,?3,?4)",
        )?;
        let mut baseline = HashSet::new();
        // BM25 statistics belong to the caller-authorized scope's persistent property corpus,
        // including other fields and rows excluded by this query's filters.
        let mut corpus_params: Vec<rusqlite::types::Value> = Vec::new();
        let mut corpus_scope_sql = String::new();
        scope.append_sql_values("n.scope", &mut corpus_scope_sql, &mut corpus_params);
        let mut stmt = tx.prepare(&format!(
            "SELECT p.attr_value, p.attr_name, p.write_cursor
             FROM property_search_index p
             JOIN canonical_nodes n ON n.write_cursor = p.write_cursor
             WHERE {corpus_scope_sql}
             ORDER BY p.rowid"
        ))?;
        let rows = stmt.query_map(rusqlite::params_from_iter(corpus_params.iter()), |r| {
            Ok((r.get::<_, Option<String>>(0)?, r.get::<_, String>(1)?, r.get::<_, u64>(2)?))
        })?;
        for row in rows {
            let (value, attribute, cursor) = row?;
            insert.execute(params![value, attribute, cursor, eligible.contains(&cursor)])?;
            baseline.insert((cursor, attribute));
        }
        // Add only lifecycle-relaxed owners, with source authority at the
        // same frozen instant. Own validity affects output, not corpus stats.
        let existence = view.view.existence_sql("n");
        let dependency = dependency_closure::read_eligibility_sql(
            "n",
            view.view.include_superseded,
            view.view.include_inactive,
            view.view.include_out_of_window,
            1,
        );
        let mut relaxed_params = vec![rusqlite::types::Value::Integer(view.edge_now())];
        let mut relaxed_scope_sql = String::new();
        scope.append_sql_values("n.scope", &mut relaxed_scope_sql, &mut relaxed_params);
        let relaxed_rows: Vec<(u64, String, Option<PropertyMap>)> = {
            let mut additions = tx.prepare(&format!(
                "SELECT n.write_cursor,n.body,n.properties FROM canonical_nodes n
                WHERE ?1 IS NOT NULL AND {relaxed_scope_sql} AND (n.superseded_at IS NOT NULL OR n.state!='active')
                {existence}{dependency} ORDER BY n.write_cursor"
            ))?;
            let rows =
                additions.query_map(rusqlite::params_from_iter(relaxed_params.iter()), |r| {
                    Ok((
                        r.get::<_, u64>(0)?,
                        r.get::<_, String>(1)?,
                        decode_properties_row_col(r, 2)?,
                    ))
                })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        for ((_, attribute), stored) in registry.iter().filter(|((entity_class, _), stored)| {
            *entity_class == PropertyEntityClass::Node && stored.wants_property_fts()
        }) {
            for (cursor, body, props) in &relaxed_rows {
                let value = extract_scalar_attribute_with_properties(
                    tx,
                    body,
                    props.as_ref(),
                    attribute,
                    stored,
                )?;
                if let Some(value) = value {
                    if !baseline.contains(&(*cursor, attribute.clone())) {
                        insert.execute(params![
                            value,
                            attribute,
                            *cursor,
                            eligible.contains(cursor)
                        ])?;
                    }
                }
            }
        }
    }
    corpus_tx.commit()?;
    #[cfg(feature = "test-hooks")]
    crate::test_hooks::property_corpus_hook::fire(_attribution)?;
    let mut rank = corpus.prepare(
        "SELECT write_cursor,bm25(history_property) FROM history_property
        WHERE history_property MATCH ?1 AND attr_name=?2 AND eligible=1
        ORDER BY bm25(history_property),write_cursor LIMIT ?3",
    )?;
    let winners = rank.query_map(params![match_expression, name, limit as i64], |r| {
        Ok((r.get::<_, u64>(0)?, r.get::<_, f64>(1)?))
    })?;
    let mut hydrate = tx.prepare(
        "SELECT body,kind,logical_id,source_id,properties FROM canonical_nodes WHERE write_cursor=?1",
    )?;
    let mut results = Vec::new();
    for winner in winners {
        let (cursor, score) = winner?;
        let hit = hydrate.query_row([cursor], |r| {
            let body: String = r.get(0)?;
            let logical_id: Option<String> = r.get(2)?;
            Ok(SearchHit {
                id: derive_stable_id(logical_id.as_deref(), &body),
                body,
                kind: r.get(1)?,
                write_cursor: cursor,
                source_id: r.get(3)?,
                score: -score,
                branch: SoftFallbackBranch::Text,
                ce_score: None,
                properties: decode_properties_row_col(r, 4)?,
            })
        })?;
        results.push(hit);
    }
    Ok(SearchResult {
        projection_cursor: load_projection_cursor(tx)?,
        soft_fallback: None,
        results,
        explanation: None,
        degradations: Vec::new(),
        snippets: None,
        hints: None,
    })
}

/// Largest attribute cohort that takes the cohort-prefiltered rank stream.
/// Fixed by the Slice 40 H3 rule from a plan probe, before measurement; the
/// engine timing report in the Slice 40 test plan (§ 3g.4) shows the same
/// direction. Not tuned from timings.
const COHORT_PREFILTER_MAX_ROWS: i64 = 256;

#[derive(Clone, Copy, PartialEq, Eq)]
enum CohortPrefilterMode {
    Auto,
    #[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
    On,
    #[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
    Off,
}

fn cohort_prefilter_mode() -> CohortPrefilterMode {
    #[cfg(feature = "test-hooks")]
    match std::env::var("FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST").as_deref() {
        Ok("on") => return CohortPrefilterMode::On,
        Ok("off") => return CohortPrefilterMode::Off,
        _ => {}
    }
    CohortPrefilterMode::Auto
}

/// The attribute pair with the smallest cohort, first in filter order on ties,
/// with its count capped at `COHORT_PREFILTER_MAX_ROWS + 1`. Each probe reads
/// at most that many entries of `canonical_attributes_name_value_idx`.
fn smallest_cohort<'f>(
    tx: &rusqlite::Transaction<'_>,
    filter: &'f SearchFilter,
    scope: &crate::scope::ScopePredicate,
) -> Result<Option<(&'f str, &'f str, i64)>, SearchReaderError> {
    let mut scope_sql = String::new();
    let (clause, _) = scope.sql_clause("cn.scope", 4);
    scope_sql.push_str(&clause);
    let sql = format!(
        "SELECT count(*) FROM (\
         SELECT 1 FROM canonical_attributes ca \
         JOIN canonical_nodes cn ON cn.write_cursor = ca.write_cursor \
         WHERE ca.attr_name = ?1 AND ca.attr_value = ?2 AND {scope_sql} \
         LIMIT ?3)"
    );
    let mut statement = tx.prepare_cached(&sql)?;
    let mut best: Option<(&str, &str, i64)> = None;
    for (name, value) in &filter.attributes {
        let mut params = vec![
            rusqlite::types::Value::Text(name.clone()),
            rusqlite::types::Value::Text(value.clone()),
            rusqlite::types::Value::Integer(COHORT_PREFILTER_MAX_ROWS + 1),
        ];
        scope.push_values(&mut params);
        let count: i64 =
            statement.query_row(rusqlite::params_from_iter(params.iter()), |row| row.get(0))?;
        if best.is_none_or(|(_, _, smallest)| count < smallest) {
            best = Some((name.as_str(), value.as_str(), count));
        }
    }
    Ok(best)
}

/// The cohort-prefiltered rank stream, when the filter's smallest attribute
/// cohort is at most `COHORT_PREFILTER_MAX_ROWS`. `None` means the caller
/// continues with the Defect C stream: no attribute pair, a cohort above the
/// threshold, or a cohort statement that failed (recorded as the fallback).
fn read_projected_cohort_stream(
    tx: &rusqlite::Transaction<'_>,
    filter: Option<&SearchFilter>,
    node_predicate: &str,
    filter_predicate: &str,
    params: &[rusqlite::types::Value],
    scope: &crate::scope::ScopePredicate,
    limit: usize,
) -> Result<Option<Vec<SearchHit>>, SearchReaderError> {
    let mode = cohort_prefilter_mode();
    let Some(filter) = filter else {
        return Ok(None);
    };
    if mode == CohortPrefilterMode::Off || filter.attributes.is_empty() {
        return Ok(None);
    }
    let Some((name, value, count)) = smallest_cohort(tx, filter, scope)? else {
        return Ok(None);
    };
    if mode == CohortPrefilterMode::Auto && count > COHORT_PREFILTER_MAX_ROWS {
        return Ok(None);
    }
    let mut cohort_params = params.to_vec();
    cohort_params.push(rusqlite::types::Value::Text(name.to_string()));
    let name_index = cohort_params.len();
    cohort_params.push(rusqlite::types::Value::Text(value.to_string()));
    let value_index = cohort_params.len();
    #[cfg(feature = "test-hooks")]
    let attributes_table = if std::env::var_os("FATHOMDB_FTS_FAIL_COHORT_FOR_TEST").is_some() {
        "missing_cohort_attributes"
    } else {
        "canonical_attributes"
    };
    #[cfg(not(feature = "test-hooks"))]
    let attributes_table = "canonical_attributes";
    let sql = property_fts_cohort_stream_sql(
        node_predicate,
        filter_predicate,
        "bm25()",
        attributes_table,
        name_index,
        value_index,
    );
    #[cfg(feature = "test-hooks")]
    record_fts_query_plan_for_test(tx, &sql, &cohort_params);
    let streamed = tx.prepare(&sql).and_then(|mut statement| {
        let mut rows = statement.query(rusqlite::params_from_iter(cohort_params.iter()))?;
        collect_complete_rank_boundary(&mut rows, limit, None)
    });
    match streamed {
        Ok((mut results, _crossed_boundary_tie)) => {
            #[cfg(feature = "test-hooks")]
            record_fts_route_for_test(if _crossed_boundary_tie {
                "projected_cohort_stream_tie_completed"
            } else {
                "projected_cohort_stream_strict_boundary"
            });
            for hit in &mut results {
                hit.score = -hit.score;
            }
            Ok(Some(results))
        }
        Err(_) => {
            #[cfg(feature = "test-hooks")]
            record_fts_route_for_test("projected_cohort_stream_fallback");
            Ok(None)
        }
    }
}
