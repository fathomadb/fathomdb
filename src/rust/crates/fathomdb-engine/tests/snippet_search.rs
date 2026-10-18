//! EP-17 match-centred snippets on real stores
//! (`dev/design/agent-access/match-centred-snippets.md` §§ 6, 7.2, 8).

use std::sync::{Arc, Mutex, MutexGuard};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    Engine, InitialState, OpenedEngine, PreparedWrite, ReadContextV1, ReadView, SearchFilter,
    SearchResult, SnippetOptions, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

/// Serialises the tests: some read the process-wide statement trace.
static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("snippet-search", "v1", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let mut vector = vec![0.0; 8];
        for (index, byte) in text.bytes().enumerate() {
            vector[index % 8] += f32::from(byte) / 255.0;
        }
        Ok(vector)
    }
}

struct Store {
    _dir: TempDir,
    opened: OpenedEngine,
}

impl Store {
    fn engine(&self) -> &Engine {
        &self.opened.engine
    }
}

fn store(bodies: &[&str]) -> Store {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("snippets{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
    opened.engine.configure_vector_kind_for_test("doc").expect("vector kind");
    let writes: Vec<PreparedWrite> =
        bodies.iter().enumerate().map(|(i, body)| node(&format!("n{i}"), body)).collect();
    if !writes.is_empty() {
        opened.engine.write(&writes).expect("write");
    }
    opened.engine.drain(10_000).expect("drain");
    Store { _dir: dir, opened }
}

fn node(logical_id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("test:snippets").expect("source id"),
        logical_id: Some(logical_id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn text(
    engine: &Engine,
    query: &str,
    limit: usize,
    snippet: Option<&SnippetOptions>,
) -> SearchResult {
    engine
        .search_text_only_view_with_snippets(query, &ReadView::default(), limit, snippet)
        .expect("text search")
}

fn hybrid(engine: &Engine, query: &str, snippet: Option<&SnippetOptions>) -> SearchResult {
    engine
        .search_reranked_view_with_snippets(
            query,
            None,
            0,
            false,
            0.3,
            0,
            false,
            &ReadView::default(),
            10,
            snippet,
        )
        .expect("hybrid search")
}

fn frozen(engine: &Engine, query: &str, snippet: Option<&SnippetOptions>) -> SearchResult {
    let context = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("context"),
        )
        .expect("freeze");
    engine
        .search_frozen_with_snippets(query, &context, 0, false, 0.3, 0, false, 10, snippet)
        .expect("frozen search")
}

/// A body of `len` bytes of filler words with `term` placed at byte `at`.
fn long_body(at: usize, term: &str, len: usize) -> String {
    let mut body = String::new();
    while body.len() < at {
        body.push_str("lorem ");
    }
    body.truncate(at);
    body.push_str(term);
    while body.len() < len {
        body.push_str(" ipsum");
    }
    body
}

// § 7.2: the sidecar is present, parallel to `results`, when asked for, on
// every search entry point that takes snippet options.
#[test]
fn sidecar_is_some_when_requested() {
    let _guard = lock();
    let s = store(&["a short pear note", &long_body(3000, "pear", 4096)]);
    let options = SnippetOptions::default();
    for (path, result) in [
        ("text", text(s.engine(), "pear", 10, Some(&options))),
        ("hybrid", hybrid(s.engine(), "pear", Some(&options))),
        ("frozen", frozen(s.engine(), "pear", Some(&options))),
    ] {
        assert!(!result.results.is_empty(), "{path}: no hits");
        let snippets = result.snippets.as_ref().unwrap_or_else(|| panic!("{path}: no sidecar"));
        assert_eq!(snippets.len(), result.results.len(), "{path}");
    }
    assert!(text(s.engine(), "pear", 10, None).snippets.is_none());
}
