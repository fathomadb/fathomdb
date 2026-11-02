use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{Engine, InitialState, PreparedWrite, ReadView, SourceId};
use std::sync::Arc;
use tempfile::TempDir;

struct Provider;
impl Embedder for Provider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice80", "test", 8)
    }
    fn embed(&self, _: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0; 8])
    }
}
fn node(id: &str, body: &str, state: InitialState) -> PreparedWrite {
    PreparedWrite::Node {
        logical_id: Some(id.into()),
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("test:slice80").unwrap(),
        state,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}
fn view(superseded: bool, inactive: bool) -> ReadView {
    ReadView { include_superseded: superseded, include_inactive: inactive, ..ReadView::default() }
}
#[test]
fn text_and_dense_history_flags_compose_independently() {
    for dense in [false, true] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("history.sqlite");
        let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
        opened.engine.configure_vector_kind_for_test("doc").unwrap();
        opened
            .engine
            .write(&[
                node("old", "history retired", InitialState::Active),
                node("inactive", "history pending", InitialState::Pending),
            ])
            .unwrap();
        opened.engine.write(&[node("old", "history current", InitialState::Active)]).unwrap();
        opened.engine.drain(10_000).unwrap();
        opened.engine.set_vector_stage_only_for_test(dense);
        for (superseded, inactive, expected) in
            [(false, false, 1), (true, false, 2), (false, true, 2), (true, true, 3)]
        {
            let result = opened.engine.search_view("history", &view(superseded, inactive)).unwrap();
            assert_eq!(
                result.results.len(),
                expected,
                "dense={dense} view={superseded}/{inactive}: {result:?}"
            );
            assert!(result.soft_fallback.is_none());
        }
        opened.engine.close().unwrap();
    }
}
