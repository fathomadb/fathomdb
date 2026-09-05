use std::collections::BTreeSet;

use fathomdb_engine::{
    slice35_ranked_eligibility_sql_for_test, Engine, InitialState, PreparedWrite, ProjectionRole,
    ProjectionSpec, ReadContextV1, ReadView, SearchFilter, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

fn node(id: &str, kind: &str) -> PreparedWrite {
    PreparedWrite::Node {
        logical_id: Some(id.to_string()),
        kind: kind.to_string(),
        body: "common eligibility needle".to_string(),
        source_id: SourceId::new("test:slice35-pretruncation").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

#[test]
fn body_fts_filters_before_the_ranked_limit() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(dir.path().join(format!("body-cap{SQLITE_SUFFIX}"))).unwrap();
    let mut writes =
        (0..150).map(|index| node(&format!("excluded-{index:03}"), "note")).collect::<Vec<_>>();
    writes.push(node("eligible", "doc"));
    opened.engine.write(&writes).unwrap();

    let mut eligibility = SearchFilter::default();
    eligibility.kind = Some("doc".to_string());
    let frozen = opened
        .engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), eligibility).unwrap())
        .unwrap();
    let result = opened
        .engine
        .search_frozen("common eligibility needle", &frozen, 0, false, 0.3, 0, false, 1)
        .unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].id.value, "eligible");
}

#[test]
fn ranked_sql_places_real_column_and_eav_eligibility_before_ranking_and_limits() {
    let mut filter = SearchFilter::default();
    filter.kind = Some("doc".to_string());
    filter.attributes = vec![("owner".to_string(), "alice".to_string())];

    for (arm, sql) in slice35_ranked_eligibility_sql_for_test(&filter) {
        let kind = sql.find(".kind =").unwrap_or_else(|| panic!("{arm}: missing kind: {sql}"));
        let eav = sql
            .find("canonical_attributes")
            .unwrap_or_else(|| panic!("{arm}: missing EAV predicate: {sql}"));
        let rank = sql.find("ORDER BY").unwrap_or_else(|| panic!("{arm}: missing rank: {sql}"));
        assert!(kind < rank, "{arm}: kind eligibility follows ranking: {sql}");
        assert!(eav < rank, "{arm}: EAV eligibility follows ranking: {sql}");
        if let Some(limit) = sql.find("LIMIT") {
            assert!(rank < limit, "{arm}: limit precedes ranking: {sql}");
        }
    }
}

#[test]
fn representative_body_plan_uses_fts_and_indexed_eav_before_ranking() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("body-plan{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .configure_projections(
            &[ProjectionSpec {
                name: "owner".to_string(),
                roles: BTreeSet::from([ProjectionRole::Filterable]),
                fts: None,
                vector: None,
                source: None,
            }],
            &[],
        )
        .unwrap();

    let mut filter = SearchFilter::default();
    filter.kind = Some("doc".to_string());
    filter.attributes = vec![("owner".to_string(), "alice".to_string())];
    let (_, sql) = slice35_ranked_eligibility_sql_for_test(&filter)
        .into_iter()
        .find(|(arm, _)| *arm == "body_fts")
        .unwrap();
    let connection = rusqlite::Connection::open(path).unwrap();
    let plan = connection
        .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
        .unwrap()
        .query_map(rusqlite::params!["needle", "doc", "owner", "alice"], |row| {
            row.get::<_, String>(3)
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert!(
        plan.iter().any(|step| step.contains("search_index") && step.contains("VIRTUAL TABLE")),
        "body FTS virtual index absent: {plan:?}"
    );
    assert!(
        plan.iter().any(|step| {
            step.contains("canonical_attributes")
                && step.contains("INDEX")
                && !step.contains("SCAN")
        }),
        "indexed EAV lookup absent: {plan:?}"
    );
}
