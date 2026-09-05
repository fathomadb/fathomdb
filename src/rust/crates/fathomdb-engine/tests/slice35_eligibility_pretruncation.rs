use fathomdb_engine::{
    slice35_ranked_eligibility_sql_for_test, Engine, InitialState, PreparedWrite, ReadContextV1,
    ReadView, SearchFilter, SourceId,
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
