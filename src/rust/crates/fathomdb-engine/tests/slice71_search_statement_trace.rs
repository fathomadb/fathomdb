#![cfg(feature = "test-hooks")]

use fathomdb_engine::{
    take_slice71_search_statement_trace_for_test, Engine, InitialState, PreparedWrite, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

#[test]
fn node_fts_search_has_no_per_hit_post_filter_statements() {
    let dir = TempDir::new().expect("tempdir");
    let opened = Engine::open(dir.path().join(format!("trace{SQLITE_SUFFIX}"))).expect("open");
    let writes = (0..24)
        .map(|index| PreparedWrite::Node {
            kind: "doc".into(),
            body: format!("slice71needle document {index}"),
            source_id: SourceId::new("slice71").expect("source id"),
            logical_id: Some(format!("slice71-{index}")),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect::<Vec<_>>();
    opened.engine.write(&writes).expect("seed");

    take_slice71_search_statement_trace_for_test();
    let result = opened.engine.search_text_only_with_limit("slice71needle", 24).expect("search");
    assert_eq!(result.results.len(), 24);
    let trace = take_slice71_search_statement_trace_for_test();
    assert_eq!(
        trace.iter().filter(|event| event.as_str() == "post_filter_source_lookup").count(),
        0,
        "eligibility must be enforced in ranked SQL, not by a statement per returned hit: {trace:?}"
    );
}
