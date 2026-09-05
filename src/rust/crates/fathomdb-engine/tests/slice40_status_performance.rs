#![cfg(feature = "test-hooks")]

use std::sync::Arc;
use std::time::Instant;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ArtifactRevisionId, Engine, InitialState,
    MutationProjectionStatusRequestV1, PreparedWrite, ProjectionRole, ProjectionSpec,
    ProjectionVector, ProvenancedNodeV1, SourceId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

#[derive(Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice40-status-measurement", "v1", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![0.5; 8])
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

fn percentile_ns(values: &mut [u128], percentile: usize) -> u128 {
    values.sort_unstable();
    let index = (values.len() * percentile).div_ceil(100).saturating_sub(1);
    values[index]
}

#[test]
#[ignore = "preregistered Slice 40 50k measurement only"]
fn measure_generation_and_mutation_status_at_50k() {
    let records = std::env::var("FATHOM_SLICE40_STATUS_RECORDS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(50_000);
    let samples = std::env::var("FATHOM_SLICE40_STATUS_SAMPLES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1_100);
    assert!(samples >= 101);

    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("slice40-status{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).unwrap();
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    opened.engine.set_projection_scheduler_frozen_for_test(true);

    for start in (0..records.saturating_sub(1)).step_by(128) {
        let end = (start + 128).min(records.saturating_sub(1));
        let batch = (start..end)
            .map(|index| PreparedWrite::Node {
                kind: "doc".into(),
                body: format!("slice40 status record {index}"),
                source_id: SourceId::new(format!("source:slice40-status-{index}")).unwrap(),
                logical_id: None,
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect::<Vec<_>>();
        opened.engine.write(&batch).unwrap();
    }

    let index = records.saturating_sub(1);
    let receipt = opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                "slice40-status-measured-operation",
                vec![ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
                    kind: "doc".into(),
                    body: format!("slice40 status record {index}"),
                    source_id: SourceId::new(format!("source:slice40-status-{index}")).unwrap(),
                    logical_id: Some(format!("slice40-status-{index}")),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: WriteProvenanceV1::canonical(
                        ArtifactRevisionId::new(format!("slice40-status-r{index}")).unwrap(),
                        SourceVersionId::new(format!("slice40-status-v{index}")).unwrap(),
                    ),
                })],
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(receipt.pending_projection_write_cursors.len(), 1);
    let request = MutationProjectionStatusRequestV1 {
        schema_version: 1,
        operation_id: receipt.operation_id,
        write_cursor: receipt.pending_projection_write_cursors[0],
        expected_generation_id: receipt.projection_generation_id.unwrap(),
    };

    let mut generation_ns = Vec::with_capacity(samples);
    let mut mutation_ns = Vec::with_capacity(samples);
    for _ in 0..samples {
        let started = Instant::now();
        let generation = opened.engine.read_projection_generation_status().unwrap();
        generation_ns.push(started.elapsed().as_nanos());
        assert_eq!(generation.pending_count, u64::try_from(records).unwrap());

        let started = Instant::now();
        let mutation = opened.engine.read_mutation_projection_status(request.clone()).unwrap();
        mutation_ns.push(started.elapsed().as_nanos());
        assert_eq!(mutation.pending_count, 1);
    }
    let steady_generation = &mut generation_ns[100..];
    let steady_mutation = &mut mutation_ns[100..];
    let generation_p95_ms = percentile_ns(steady_generation, 95) as f64 / 1_000_000.0;
    let generation_p99_ms = percentile_ns(steady_generation, 99) as f64 / 1_000_000.0;
    let mutation_p95_ms = percentile_ns(steady_mutation, 95) as f64 / 1_000_000.0;
    let mutation_p99_ms = percentile_ns(steady_mutation, 99) as f64 / 1_000_000.0;
    println!(
        "{{\"records\":{records},\"samples\":{},\"generation_p95_ms\":{:.6},\"generation_p99_ms\":{:.6},\"mutation_p95_ms\":{:.6},\"mutation_p99_ms\":{:.6}}}",
        steady_generation.len(),
        generation_p95_ms,
        generation_p99_ms,
        mutation_p95_ms,
        mutation_p99_ms,
    );
    assert!(generation_p95_ms <= 5.0, "generation status p95 exceeded 5 ms");
    assert!(generation_p99_ms <= 10.0, "generation status p99 exceeded 10 ms");
    assert!(mutation_p95_ms <= 5.0, "mutation status p95 exceeded 5 ms");
    assert!(mutation_p99_ms <= 10.0, "mutation status p99 exceeded 10 ms");
}
