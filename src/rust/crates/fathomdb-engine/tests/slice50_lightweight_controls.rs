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
        properties: None,
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
    opened.engine.write(&[node(Some("missing"), "entity", "endpoint")]).unwrap();
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
                properties: None,
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

fn malformed_losing_ownerless_refuses(body_sql: &str, kind_sql: &str) {
    let _serial = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let _restore = Restore(std::env::var_os("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST"));
    unsafe { std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST") };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("ownerless-malformed{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Fixed)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    let receipt = opened.engine.write(&[node(Some("winner"), "doc", "needle")]).unwrap();
    opened.engine.drain(10_000).unwrap();
    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute(
        "INSERT INTO search_index(body,kind,write_cursor) VALUES('needle padding padding padding padding','note',999999)", [],
    ).unwrap();
    let rowid = raw.last_insert_rowid();
    let search = || {
        opened.engine.search_reranked_view_with_limit(
            "needle",
            None,
            0,
            false,
            0.3,
            0,
            false,
            &ReadView::default(),
            1,
        )
    };
    let valid = search().unwrap();
    assert_eq!(valid.results.len(), 1);
    assert_eq!(valid.results[0].write_cursor, receipt.row_cursors[0]);
    unsafe { std::env::set_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1") };
    assert_eq!(search().unwrap(), valid, "the valid ownerless candidate loses on both routes");
    raw.execute(
        &format!("UPDATE search_index SET body={body_sql},kind={kind_sql} WHERE rowid=?1"),
        [rowid],
    )
    .unwrap();
    let matching: usize = raw
        .query_row(
            "SELECT COUNT(*) FROM search_index WHERE search_index MATCH 'needle'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(matching, 2, "the malformed ownerless row remains in the complete MATCH census");
    raw.close().unwrap();
    let reference = search();
    assert!(
        matches!(reference, Err(EngineError::Storage)),
        "reference must refuse malformed ownerless body/kind: {reference:?}"
    );
    unsafe { std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST") };
    let optimized = search();
    assert!(matches!(optimized, Err(EngineError::Storage)),
        "thin census must refuse malformed losing ownerless body/kind like the reference: {optimized:?}");
    opened.engine.close().unwrap();
}

#[test]
fn losing_ownerless_blob_kind_refuses_like_full_reference() {
    malformed_losing_ownerless_refuses("'needle padding padding padding padding'", "x'6e6f7465'");
}

#[test]
fn losing_ownerless_null_kind_refuses_like_full_reference() {
    malformed_losing_ownerless_refuses("'needle padding padding padding padding'", "NULL");
}

#[test]
fn losing_ownerless_invalid_utf8_kind_refuses_like_full_reference() {
    malformed_losing_ownerless_refuses(
        "'needle padding padding padding padding'",
        "CAST(x'6e6f7465ff' AS TEXT)",
    );
}

#[test]
fn losing_ownerless_blob_body_refuses_like_full_reference() {
    malformed_losing_ownerless_refuses(
        "CAST('needle padding padding padding padding' AS BLOB)",
        "'note'",
    );
}

#[test]
fn losing_ownerless_invalid_utf8_body_refuses_like_full_reference() {
    malformed_losing_ownerless_refuses(
        "CAST(x'6e6565646c652070616464696e6720ff' AS TEXT)",
        "'note'",
    );
}

#[test]
fn mixed_visibility_uses_the_reference_order_for_negative_ownerless_cursors() {
    let _serial = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("signed-cursor.sqlite");
    let engine = Engine::open_with_embedder_for_test(&path, Arc::new(Fixed)).unwrap().engine;
    engine.configure_vector_kind_for_test("doc").unwrap();
    let mut pending = node(Some("pending"), "note", "unrelated pending node");
    if let PreparedWrite::Node { state, .. } = &mut pending {
        *state = InitialState::Pending;
    }
    engine.write(&[node(Some("vector"), "doc", "nearby vector"), pending]).unwrap();
    engine.drain(10_000).unwrap();
    let raw = rusqlite::Connection::open(&path).unwrap();
    for (body, cursor) in [("needle alpha", -1_i64), ("needle bravo", 999999)] {
        raw.execute(
            "INSERT INTO search_index(body,kind,write_cursor) VALUES(?1,'note',?2)",
            rusqlite::params![body, cursor],
        )
        .unwrap();
    }
    raw.close().unwrap();

    let result = paired(&engine);
    let cursors =
        result.results.into_iter().take(2).map(|hit| hit.write_cursor).collect::<Vec<_>>();
    assert_eq!(cursors, [u64::MAX, 999999]);
    engine.close().unwrap();
}

#[test]
fn malformed_losing_identity_keeps_the_existing_refusals_by_route() {
    let _serial = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    for (column, value) in [("source_id", "X'FF'"), ("logical_id", "CAST(X'FF' AS TEXT)")] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("malformed-losing-id.sqlite");
        let engine = Engine::open_with_embedder_for_test(&path, Arc::new(Fixed)).unwrap().engine;
        engine.configure_vector_kind_for_test("doc").unwrap();
        let mut writes = (0..15)
            .map(|i| node(Some(&format!("valid-{i}")), "note", &format!("needle candidate {i}")))
            .collect::<Vec<_>>();
        writes.push(node(Some("vector"), "doc", "unmatched vector body"));
        writes.push(node(Some("corrupt"), "note", &format!("needle {}", "dilute ".repeat(100))));
        let corrupt_cursor = engine.write(&writes).unwrap().row_cursors[16];
        engine.drain(10_000).unwrap();
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute(
            &format!("UPDATE canonical_nodes SET {column}={value} WHERE write_cursor=?1"),
            [corrupt_cursor],
        )
        .unwrap();
        assert!(query(&engine)
            .unwrap()
            .results
            .iter()
            .all(|hit| hit.write_cursor != corrupt_cursor));

        let mut pending = node(Some("pending"), "note", "unmatched pending row");
        if let PreparedWrite::Node { state, .. } = &mut pending {
            *state = InitialState::Pending;
        }
        engine.write(&[pending]).unwrap();
        let observed = query(&engine).unwrap_err();
        let _restore = Restore(std::env::var_os("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST"));
        unsafe { std::env::set_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1") };
        let reference = query(&engine).unwrap_err();
        assert!(matches!(observed, EngineError::Storage));
        assert!(matches!(reference, EngineError::Storage));
        raw.close().unwrap();
        engine.close().unwrap();
    }
}
