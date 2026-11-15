//! Exactness controls for the complete thin lexical rank census.

use std::sync::{Arc, Mutex};
static ENV_LOCK: Mutex<()> = Mutex::new(());
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    Engine, EngineError, IdSpace, InitialState, PreparedWrite, ReadView, SearchResult,
    SnippetOptions, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;

#[derive(Debug)]
struct Fixed;
impl Embedder for Fixed {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice50-thin-controls", "v1", 8)
    }
    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    }
}
fn node(id: Option<&str>, kind: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: kind.into(),
        body: body.into(),
        logical_id: id.map(str::to_owned),
        source_id: SourceId::new(format!("src:{}", id.unwrap_or("anonymous"))).unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}
fn query(engine: &Engine) -> Result<SearchResult, EngineError> {
    let mut snippet = SnippetOptions::default();
    snippet.max_bytes = 40;
    snippet.highlights = true;
    engine.search_reranked_view_with_snippets(
        "needle",
        None,
        0,
        false,
        0.3,
        0,
        false,
        &ReadView::default(),
        10,
        Some(&snippet),
    )
}
struct Restore(Option<std::ffi::OsString>);
impl Drop for Restore {
    fn drop(&mut self) {
        match &self.0 {
            Some(value) => unsafe {
                std::env::set_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", value)
            },
            None => unsafe { std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST") },
        }
    }
}
fn paired(engine: &Engine) -> SearchResult {
    let _restore = Restore(std::env::var_os("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST"));
    unsafe { std::env::set_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1") };
    let reference = query(engine).unwrap();
    unsafe { std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST") };
    let result = query(engine).unwrap();
    assert_eq!(result, reference);
    result
}
#[test]
fn thin_hydration_preserves_equal_body_records_and_exact_ownerless_locators() {
    let _serial = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("thin{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Fixed)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    let twins = "needle equal stored record";
    let receipt = opened
        .engine
        .write(&[
            node(None, "note", twins),
            node(Some("governed"), "note", twins),
            node(Some("vector"), "doc", "unmatched vector body"),
        ])
        .unwrap();
    opened.engine.drain(10_000).unwrap();
    let raw = rusqlite::Connection::open(&path).unwrap();
    let first = "needle alpha alpha alpha alpha alpha alpha alpha alpha alpha";
    let second = "beta beta beta beta beta beta beta beta needle omega omega";
    for body in [first, second] {
        raw.execute(
            "INSERT INTO search_index(body,kind,write_cursor) VALUES(?1,'note',999999)",
            [body],
        )
        .unwrap();
    }
    raw.close().unwrap();
    let result = paired(&opened.engine);
    assert_eq!(result.results.len(), 5);
    let twin_hits: Vec<_> = result.results.iter().filter(|hit| hit.body == twins).collect();
    assert_eq!(twin_hits.len(), 2);
    assert_eq!(twin_hits[0].write_cursor, receipt.row_cursors[0]);
    assert_eq!(twin_hits[0].id.space, fathomdb_engine::IdSpaceKind::Content);
    assert_eq!(twin_hits[1].id, IdSpace::logical("governed"));
    assert_eq!(twin_hits[0].source_id.as_deref(), Some("src:anonymous"));
    assert_eq!(twin_hits[1].source_id.as_deref(), Some("src:governed"));
    let snippets = result.snippets.as_ref().unwrap();
    for body in [first, second] {
        let index = result.results.iter().position(|hit| hit.body == body).unwrap();
        assert_eq!(result.results[index].write_cursor, 999999);
        assert_eq!(result.results[index].source_id, None);
        let snippet = &snippets[index];
        assert!(!snippet.spans.is_empty());
        assert!(snippet
            .spans
            .iter()
            .all(|&(start, end)| &body[start as usize..end as usize] == "needle"));
    }
    opened.engine.close().unwrap();
}
#[test]
fn complete_thin_census_refuses_corrupt_losing_body_and_kind() {
    let _serial = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let _restore = Restore(std::env::var_os("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST"));
    unsafe { std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST") };
    for (column, value) in [("body", "needle forged"), ("kind", "forged-kind")] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!("corrupt{SQLITE_SUFFIX}"));
        let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Fixed)).unwrap();
        opened.engine.configure_vector_kind_for_test("doc").unwrap();
        let receipt = opened
            .engine
            .write(&[
                node(Some("vector"), "doc", "needle"),
                node(Some("loser"), "note", "needle padding padding padding padding"),
            ])
            .unwrap();
        opened.engine.drain(10_000).unwrap();
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute(
            &format!("UPDATE search_index SET {column}=?1 WHERE write_cursor=?2"),
            rusqlite::params![value, receipt.row_cursors[1]],
        )
        .unwrap();
        raw.close().unwrap();
        let result = opened.engine.search_reranked_view_with_limit(
            "needle",
            None,
            0,
            false,
            0.3,
            0,
            false,
            &ReadView::default(),
            1,
        );
        assert!(
            matches!(result, Err(EngineError::Storage)),
            "{column} must refuse even outside the selected page: {result:?}"
        );
        opened.engine.close().unwrap();
    }
}
#[test]
fn edge_presence_keeps_exact_mixed_text_reference_ranks() {
    let _serial = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("edges{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Fixed)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    let receipt = opened
        .engine
        .write(&[
            node(Some("node"), "doc", "needle twin"),
            PreparedWrite::Edge {
                kind: "link".into(),
                from: "node".into(),
                to: "missing".into(),
                body: Some("needle twin".into()),
                source_id: SourceId::new("src:edge").unwrap(),
                logical_id: Some("edge".into()),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .unwrap();
    opened.engine.drain(10_000).unwrap();
    let result = paired(&opened.engine);
    assert_eq!(result.results.len(), 2);
    assert_eq!(result.results[0].write_cursor, receipt.row_cursors[0]);
    assert_eq!(result.results[1].write_cursor, receipt.row_cursors[1]);
    assert_eq!(result.results[1].source_id.as_deref(), Some("src:edge"));
    opened.engine.close().unwrap();
}
