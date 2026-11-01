#![cfg(feature = "test-hooks")]

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{EmbedderChoice, Engine};
use std::sync::Arc;
use tempfile::TempDir;

struct HistoricProvider;
impl Embedder for HistoricProvider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("historic-provider", "historic-revision", 384)
    }
    fn embed(&self, _: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0; 384])
    }
}

#[test]
fn none_admits_historic_profile_as_metadata_without_substitution() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("historic.fathomdb");
    Engine::open_with_embedder_for_test(&path, Arc::new(HistoricProvider))
        .unwrap()
        .engine
        .close()
        .unwrap();
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None)
        .expect("explicit None must validate historic metadata, not invent a BGE provider");
    assert_eq!(opened.report.default_embedder, HistoricProvider.identity());
    assert_eq!(
        opened.engine.default_embedder_profile_for_test().unwrap(),
        HistoricProvider.identity()
    );
    opened.engine.close().unwrap();
}
