//! Experiment hooks for the projection/runtime connections
//! (`dev/experiments/ac013-seed-connections/`).
//!
//! `FATHOMDB_PERF_RUNTIME_PRAGMAS` must reach every connection opened by
//! `open_runtime_connection` (projection dispatcher and workers). The env is
//! process-global, so this binary sets one value before any engine opens and
//! every test here is compatible with it.

#![cfg(feature = "test-hooks")]

use std::sync::{Arc, Once};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::Engine;
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

const RUNTIME_CACHE_SIZE: i64 = -12_345;

fn set_runtime_pragmas() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::env::set_var("FATHOMDB_PERF_EXPERIMENTS", "1");
        std::env::set_var(
            "FATHOMDB_PERF_RUNTIME_PRAGMAS",
            format!("cache_size={RUNTIME_CACHE_SIZE}"),
        );
    });
}

#[derive(Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("fixed", "runtime-hooks", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let seed = text.len() as f32;
        Ok((0..8).map(|i| (seed + i as f32).sin()).collect())
    }
}

#[test]
fn runtime_pragma_hook_applies_cache_size_to_runtime_connections() {
    set_runtime_pragmas();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("runtime_pragmas{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
    assert_eq!(
        opened.engine.runtime_pragma_for_test("cache_size").expect("runtime cache_size"),
        RUNTIME_CACHE_SIZE,
        "open_runtime_connection must apply FATHOMDB_PERF_RUNTIME_PRAGMAS"
    );
    opened.engine.close().expect("close");
}
