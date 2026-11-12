//! Deterministic768 fixture recipe from exp_ac013_page_cache; unknown span end retained.

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    Engine, InitialState, PreparedWrite, ProjectionRole, ProjectionSpec, ProjectionVector,
    SourceId, TestEmbedderChoice,
};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

struct VaryingEmbedder;

impl Embedder for VaryingEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("varying", "perf-gates-dense", 768)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in text.as_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        if h == 0 {
            h = 0xdeadbeef_cafebabe;
        }
        let mut state = h;
        let mut vector = Vec::with_capacity(768);
        for _ in 0..768 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            vector.push(((state as u32) as i32 as f32) / (i32::MAX as f32 + 1.0));
        }
        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
        for x in &mut vector {
            *x /= norm;
        }
        Ok(vector)
    }
}

pub(super) fn run(config: &Value, out: &Path) {
    std::fs::create_dir_all(out).unwrap();
    let store = out.join("template.sqlite");
    assert!(!store.exists(), "template fixture must be fresh");
    let opened = Engine::open_with_test_choice_for_test(
        &store,
        TestEmbedderChoice::Caller(Arc::new(VaryingEmbedder)),
    )
    .unwrap();
    let engine = &opened.engine;
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "s30_dense".into(),
                roles: [ProjectionRole::Searchable].into_iter().collect(),
                fts: None,
                vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
                source: None,
            }],
            &[],
        )
        .unwrap();
    let records = config["records"].as_array().unwrap();
    assert!(!records.is_empty());
    let writes: Vec<_> = records
        .iter()
        .map(|row| PreparedWrite::Node {
            kind: row["kind"].as_str().unwrap().into(),
            body: row["body"].as_str().unwrap().into(),
            source_id: SourceId::new("test:slice30-d-template").unwrap(),
            logical_id: Some(row["id"].as_str().unwrap().into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect();
    let writing = std::time::Instant::now();
    let receipt = engine.write(&writes).unwrap();
    let write_ns = u64::try_from(writing.elapsed().as_nanos()).unwrap();
    let draining = std::time::Instant::now();
    engine.drain(5000).unwrap();
    let drain_ns = u64::try_from(draining.elapsed().as_nanos()).unwrap();
    let pending = engine.read_embedding_readiness().unwrap().pending_count;
    assert_eq!(pending, 0);
    // Observe resolved configuration only after work, not as worker concurrency evidence.
    engine.begin_d27_observation_for_test(std::time::Instant::now());
    let configuration = engine.d27_observation_for_test().unwrap().configuration_observation;
    assert_eq!((configuration.scheduler_runtime_threads, configuration.embedder_pool_size), (2, 5));
    let closing = std::time::Instant::now();
    engine.close().unwrap();
    let close_ns = u64::try_from(closing.elapsed().as_nanos()).unwrap();
    let proof = json!({"api_complete":true,"write_calls":1,"write_rows":writes.len(),
        "receipt_cursor":receipt.cursor,"pending":pending,"write_ns":write_ns,"drain_ns":drain_ns,"close_ns":close_ns,
        "scheduler_runtime_threads":configuration.scheduler_runtime_threads,
        "embedder_pool_size":configuration.embedder_pool_size,
        "configuration_observation":"existing D27 configuration snapshot after drain",
        "provider":"test-hooks varying/perf-gates-dense768; no BGE","span_end":"unknown"});
    std::fs::write(out.join("api-proof.json"), serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
}
