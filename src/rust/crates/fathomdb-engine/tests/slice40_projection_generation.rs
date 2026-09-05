use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    Engine, ProjectionGenerationOriginV1, ProjectionReadinessV1, ProjectionRuntimeStateV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

#[derive(Clone, Debug)]
struct CustomEmbedder;

impl Embedder for CustomEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice40-custom", "r1", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0; 8])
    }
}

fn assert_generation_id(value: &str) {
    let suffix = value.strip_prefix("pgen1:").expect("versioned generation prefix");
    assert_eq!(suffix.len(), 32);
    assert!(suffix.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
}

#[test]
fn fresh_generation_is_stable_across_restart() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("fresh{SQLITE_SUFFIX}"));
    let first = Engine::open(&path).unwrap();
    let status = first.engine.read_projection_generation_status().unwrap();
    assert_eq!(status.origin, ProjectionGenerationOriginV1::Fresh);
    assert_eq!(status.readiness, ProjectionReadinessV1::Ready);
    assert_eq!(status.runtime_state, ProjectionRuntimeStateV1::Absent);
    assert_generation_id(status.generation_id.as_str());
    let id = status.generation_id.clone();
    first.engine.close().unwrap();

    let second = Engine::open(&path).unwrap();
    assert_eq!(second.engine.read_projection_generation_status().unwrap().generation_id, id);
}

#[test]
fn caller_embedder_does_not_make_an_empty_database_legacy() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("caller{SQLITE_SUFFIX}"));
    let first = Engine::open_with_embedder_for_test(&path, Arc::new(CustomEmbedder)).unwrap();
    let status = first.engine.read_projection_generation_status().unwrap();
    assert_eq!(status.origin, ProjectionGenerationOriginV1::Fresh);
    let id = status.generation_id;
    first.engine.close().unwrap();

    let second = Engine::open_with_embedder_for_test(&path, Arc::new(CustomEmbedder)).unwrap();
    assert_eq!(second.engine.read_projection_generation_status().unwrap().generation_id, id);
}
