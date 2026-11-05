use fathomdb_sdk::{
    Engine, InitialState, OpenOptions, PreparedWrite, SearchFilter, SourceId, TextSearchOptions,
};

#[test]
fn text_options_forward_the_filter_to_the_core() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().join("s90.sqlite"), OpenOptions::default()).unwrap();
    engine
        .write(&[
            PreparedWrite::Node {
                kind: "doc".into(),
                body: "s90sdk unwanted".into(),
                source_id: SourceId::new("slice90").unwrap(),
                logical_id: Some("doc".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            },
            PreparedWrite::Node {
                kind: "note".into(),
                body: "s90sdk wanted".into(),
                source_id: SourceId::new("slice90").unwrap(),
                logical_id: Some("note".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            },
        ])
        .unwrap();
    let mut filter = SearchFilter::default();
    filter.kind = Some("note".into());
    let mut options = TextSearchOptions::default();
    options.filter = Some(filter);
    options.limit = 1;
    let result = engine.search_text_only("s90sdk", options).unwrap();
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].id.value, "note");
    engine.close().unwrap();
}
