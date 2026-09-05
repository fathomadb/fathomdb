#![cfg(feature = "test-hooks")]

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ArtifactRevisionId, Engine,
    MutationProjectionStatusRequestV1, ProjectionGenerationErrorReason, ProjectionRole,
    ProjectionSpec, ProjectionVector, ProvenancedNodeV1, SourceId, SourceVersionId,
    WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

#[derive(Debug)]
struct RaceEmbedder;

impl Embedder for RaceEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice40-race", "r1", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![0.25; 8])
    }
}

#[derive(Debug)]
struct BlockingRaceEmbedder {
    armed: Arc<AtomicBool>,
    calls: Arc<AtomicUsize>,
    started: Arc<Barrier>,
    release: Arc<Barrier>,
}

impl Embedder for BlockingRaceEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice40-blocking-race", "r1", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.armed.load(Ordering::SeqCst) {
            self.started.wait();
            self.release.wait();
        }
        Ok(vec![0.5; 8])
    }
}

#[derive(Debug)]
struct CountingRaceEmbedder {
    calls: Arc<AtomicUsize>,
}

impl Embedder for CountingRaceEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice40-counting-race", "r1", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(vec![call as f32; 8])
    }
}

fn vector_spec() -> ProjectionSpec {
    ProjectionSpec {
        name: "memory".into(),
        roles: [ProjectionRole::Searchable].into_iter().collect(),
        fts: None,
        vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
        source: None,
    }
}

fn actuate_race_node(engine: &Engine, suffix: &str) -> fathomdb_engine::ActuationReceiptV1 {
    engine
        .actuate(
            ActuationBatchV1::new(
                format!("slice40-{suffix}"),
                vec![ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
                    kind: "doc".into(),
                    body: format!("generation race {suffix}"),
                    source_id: SourceId::new(format!("source:slice40-{suffix}")).unwrap(),
                    logical_id: Some(format!("slice40-{suffix}-node")),
                    state: fathomdb_engine::InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: WriteProvenanceV1::canonical(
                        ArtifactRevisionId::new(format!("slice40-{suffix}-r1")).unwrap(),
                        SourceVersionId::new(format!("slice40-{suffix}-v1")).unwrap(),
                    ),
                })],
            )
            .unwrap(),
        )
        .unwrap()
}

#[test]
fn stale_worker_result_cannot_publish_into_a_new_generation() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("stale-publication{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(RaceEmbedder)).unwrap();
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    opened.engine.set_projection_scheduler_frozen_for_test(true);

    let receipt = opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                "slice40-stale-publication",
                vec![ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
                    kind: "doc".into(),
                    body: "generation-bound evidence".into(),
                    source_id: SourceId::new("source:slice40-race").unwrap(),
                    logical_id: Some("slice40-race-node".into()),
                    state: fathomdb_engine::InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: WriteProvenanceV1::canonical(
                        ArtifactRevisionId::new("slice40-race-r1").unwrap(),
                        SourceVersionId::new("slice40-race-v1").unwrap(),
                    ),
                })],
            )
            .unwrap(),
        )
        .unwrap();
    let cursor = receipt.pending_projection_write_cursors[0];
    let old_generation = receipt.projection_generation_id.unwrap();

    let new_generation = opened.engine.transition_projection_generation_for_test().unwrap();
    assert_ne!(new_generation, old_generation);
    opened
        .engine
        .publish_projection_success_for_test(cursor, "doc", old_generation.clone())
        .unwrap();
    assert!(!opened.engine.has_vector_for_cursor_for_test(cursor).unwrap());

    let error = opened
        .engine
        .read_mutation_projection_status(MutationProjectionStatusRequestV1 {
            schema_version: 1,
            operation_id: receipt.operation_id,
            write_cursor: cursor,
            expected_generation_id: old_generation,
        })
        .unwrap_err();
    assert!(matches!(
        error,
        fathomdb_engine::EngineError::ProjectionGeneration(error)
            if error.reason
                == ProjectionGenerationErrorReason::ProjectionGenerationUnavailable
    ));

    opened.engine.set_projection_scheduler_frozen_for_test(false);
    opened.engine.drain(5_000).unwrap();
    let status = opened.engine.read_projection_generation_status().unwrap();
    assert_eq!(status.generation_id, new_generation);
    assert_eq!(status.readiness.as_str(), "ready");
    assert!(opened.engine.has_vector_for_cursor_for_test(cursor).unwrap());
}

#[test]
fn worker_computing_across_generation_transition_discards_stale_result() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("computing-transition{SQLITE_SUFFIX}"));
    let armed = Arc::new(AtomicBool::new(false));
    let calls = Arc::new(AtomicUsize::new(0));
    let started = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let opened = Engine::open_with_embedder_for_test(
        &path,
        Arc::new(BlockingRaceEmbedder {
            armed: Arc::clone(&armed),
            calls: Arc::clone(&calls),
            started: Arc::clone(&started),
            release: Arc::clone(&release),
        }),
    )
    .unwrap();
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    armed.store(true, Ordering::SeqCst);
    let receipt = opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                "slice40-computing-transition",
                vec![ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
                    kind: "doc".into(),
                    body: "blocked generation-bound evidence".into(),
                    source_id: SourceId::new("source:slice40-computing").unwrap(),
                    logical_id: Some("slice40-computing-node".into()),
                    state: fathomdb_engine::InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: WriteProvenanceV1::canonical(
                        ArtifactRevisionId::new("slice40-computing-r1").unwrap(),
                        SourceVersionId::new("slice40-computing-v1").unwrap(),
                    ),
                })],
            )
            .unwrap(),
        )
        .unwrap();
    let cursor = receipt.pending_projection_write_cursors[0];
    let old_generation = receipt.projection_generation_id.unwrap();
    started.wait();
    let new_generation = opened.engine.transition_projection_generation_for_test().unwrap();
    assert_ne!(new_generation, old_generation);
    assert!(!opened.engine.has_vector_for_cursor_for_test(cursor).unwrap());
    armed.store(false, Ordering::SeqCst);
    release.wait();
    opened.engine.drain(5_000).unwrap();

    let status = opened.engine.read_projection_generation_status().unwrap();
    assert_eq!(status.generation_id, new_generation);
    assert_eq!(status.readiness.as_str(), "ready");
    assert!(opened.engine.has_vector_for_cursor_for_test(cursor).unwrap());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn queued_worker_across_generation_transition_is_rediscovered() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("queued-transition{SQLITE_SUFFIX}"));
    let calls = Arc::new(AtomicUsize::new(0));
    let opened = Engine::open_with_embedder_for_test(
        &path,
        Arc::new(CountingRaceEmbedder { calls: Arc::clone(&calls) }),
    )
    .unwrap();
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    let (queued, release) = opened.engine.pause_projection_worker_while_queued_for_test();
    let receipt = actuate_race_node(&opened.engine, "queued-transition");
    let cursor = receipt.pending_projection_write_cursors[0];
    let old_generation = receipt.projection_generation_id.unwrap();
    queued.wait();
    let new_generation = opened.engine.transition_projection_generation_for_test().unwrap();
    assert!(!opened.engine.has_vector_for_cursor_for_test(cursor).unwrap());
    release.wait();
    opened.engine.drain(5_000).unwrap();

    assert_ne!(new_generation, old_generation);
    assert_eq!(
        opened.engine.read_projection_generation_status().unwrap().generation_id,
        new_generation
    );
    assert!(opened.engine.has_vector_for_cursor_for_test(cursor).unwrap());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn worker_at_write_lock_boundary_discards_after_transition() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("write-lock-transition{SQLITE_SUFFIX}"));
    let calls = Arc::new(AtomicUsize::new(0));
    let opened = Engine::open_with_embedder_for_test(
        &path,
        Arc::new(CountingRaceEmbedder { calls: Arc::clone(&calls) }),
    )
    .unwrap();
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    let (waiting, release) = opened.engine.pause_projection_worker_before_write_lock_for_test();
    let receipt = actuate_race_node(&opened.engine, "write-lock-transition");
    let cursor = receipt.pending_projection_write_cursors[0];
    let old_generation = receipt.projection_generation_id.unwrap();
    waiting.wait();
    let new_generation = opened.engine.transition_projection_generation_for_test().unwrap();
    assert!(!opened.engine.has_vector_for_cursor_for_test(cursor).unwrap());
    release.wait();
    opened.engine.drain(5_000).unwrap();

    assert_ne!(new_generation, old_generation);
    assert_eq!(
        opened.engine.read_projection_generation_status().unwrap().generation_id,
        new_generation
    );
    assert!(opened.engine.has_vector_for_cursor_for_test(cursor).unwrap());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn publication_holding_write_lock_linearizes_before_transition() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("publication-transition{SQLITE_SUFFIX}"));
    let calls = Arc::new(AtomicUsize::new(0));
    let opened = Engine::open_with_embedder_for_test(
        &path,
        Arc::new(CountingRaceEmbedder { calls: Arc::clone(&calls) }),
    )
    .unwrap();
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    let (transaction_ready, release) =
        opened.engine.pause_projection_worker_after_wal_transaction_for_test();
    let receipt = actuate_race_node(&opened.engine, "publication-transition");
    let cursor = receipt.pending_projection_write_cursors[0];
    let old_generation = receipt.projection_generation_id.unwrap();
    transaction_ready.wait();
    assert!(!opened.engine.has_vector_for_cursor_for_test(cursor).unwrap());
    let new_generation = thread::scope(|scope| {
        let transition =
            scope.spawn(|| opened.engine.transition_projection_generation_for_test().unwrap());
        release.wait();
        transition.join().unwrap()
    });
    opened.engine.drain(5_000).unwrap();

    assert_ne!(new_generation, old_generation);
    assert!(opened.engine.has_vector_for_cursor_for_test(cursor).unwrap());
    let status = opened.engine.read_projection_generation_status().unwrap();
    assert_eq!(status.generation_id, new_generation);
    assert_eq!(status.readiness.as_str(), "ready");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
