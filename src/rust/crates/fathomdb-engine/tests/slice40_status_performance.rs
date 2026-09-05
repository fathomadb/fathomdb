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
use serde_json::json;
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

fn measured_batch(index: usize) -> ActuationBatchV1 {
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
    .unwrap()
}

#[test]
#[ignore = "preregistered Slice 40 50k measurement only"]
fn measure_generation_and_mutation_status_at_50k() {
    let records = std::env::var("FATHOM_SLICE40_STATUS_RECORDS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(50_000);
    let warmups = std::env::var("FATHOM_SLICE40_STATUS_WARMUPS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(100);
    let samples = std::env::var("FATHOM_SLICE40_STATUS_SAMPLES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1_000);
    assert!(warmups > 0 && samples > 0);

    let requested_path = std::env::var_os("FATHOM_SLICE40_STATUS_DATABASE").map(Into::into);
    let temp_dir = requested_path.is_none().then(|| TempDir::new().unwrap());
    let path = requested_path.unwrap_or_else(|| {
        temp_dir.as_ref().unwrap().path().join(format!("slice40-status{SQLITE_SUFFIX}"))
    });
    let needs_seed = !path.exists();
    let setup = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).unwrap();
    setup.engine.configure_projections(&[vector_spec()], &[]).unwrap();

    if needs_seed {
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
            setup.engine.write(&batch).unwrap();
        }
    }

    let index = records.saturating_sub(1);
    let receipt = setup.engine.actuate(measured_batch(index)).unwrap();
    assert_eq!(receipt.pending_projection_write_cursors.len(), 1);
    setup.engine.drain(30_000).unwrap();
    drop(setup);
    if std::env::var_os("FATHOM_SLICE40_STATUS_SEED_ONLY").is_some() {
        return;
    }

    let generation_opened =
        Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).unwrap();
    let started = Instant::now();
    generation_opened.engine.read_projection_generation_status().unwrap();
    let cold_generation_ms = started.elapsed().as_secs_f64() * 1_000.0;
    drop(generation_opened);

    let mutation_opened =
        Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).unwrap();
    let receipt = mutation_opened.engine.actuate(measured_batch(index)).unwrap();
    let cold_request = MutationProjectionStatusRequestV1 {
        schema_version: 1,
        operation_id: receipt.operation_id,
        write_cursor: receipt.pending_projection_write_cursors[0],
        expected_generation_id: receipt.projection_generation_id.unwrap(),
    };
    let started = Instant::now();
    mutation_opened.engine.read_mutation_projection_status(cold_request).unwrap();
    let cold_mutation_ms = started.elapsed().as_secs_f64() * 1_000.0;
    drop(mutation_opened);

    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).unwrap();
    opened.engine.set_projection_scheduler_frozen_for_test(true);
    let receipt = opened.engine.actuate(measured_batch(index)).unwrap();
    let request = MutationProjectionStatusRequestV1 {
        schema_version: 1,
        operation_id: receipt.operation_id,
        write_cursor: receipt.pending_projection_write_cursors[0],
        expected_generation_id: receipt.projection_generation_id.unwrap(),
    };

    let mut steady_effective_at = 0;
    for _ in 0..warmups {
        let generation = opened.engine.read_projection_generation_status().unwrap();
        let mutation = opened.engine.read_mutation_projection_status(request.clone()).unwrap();
        steady_effective_at = generation.effective_at_epoch_s;
        assert_eq!(mutation.effective_at_epoch_s, steady_effective_at);
    }
    let mut generation_ns = Vec::with_capacity(samples);
    let mut mutation_ns = Vec::with_capacity(samples);
    let mut steady_full_owner_scans = 0;
    let mut epoch_rollover_full_owner_scans = 0;
    while generation_ns.len() < samples {
        let scans_before =
            opened.engine.projection_generation_status_full_owner_scan_count_for_test();
        let started = Instant::now();
        let generation = opened.engine.read_projection_generation_status().unwrap();
        let generation_elapsed = started.elapsed().as_nanos();
        assert_eq!(generation.pending_count, 0);

        let started = Instant::now();
        let mutation = opened.engine.read_mutation_projection_status(request.clone()).unwrap();
        let mutation_elapsed = started.elapsed().as_nanos();
        assert_eq!(mutation.pending_count, 0);
        let scan_delta =
            opened.engine.projection_generation_status_full_owner_scan_count_for_test()
                - scans_before;
        if generation.effective_at_epoch_s != steady_effective_at
            || mutation.effective_at_epoch_s != steady_effective_at
        {
            epoch_rollover_full_owner_scans += scan_delta;
            steady_effective_at = mutation.effective_at_epoch_s;
            continue;
        }
        steady_full_owner_scans += scan_delta;
        generation_ns.push(generation_elapsed);
        mutation_ns.push(mutation_elapsed);
    }
    let generation_p95_ms = percentile_ns(&mut generation_ns, 95) as f64 / 1_000_000.0;
    let generation_p99_ms = percentile_ns(&mut generation_ns, 99) as f64 / 1_000_000.0;
    let mutation_p95_ms = percentile_ns(&mut mutation_ns, 95) as f64 / 1_000_000.0;
    let mutation_p99_ms = percentile_ns(&mut mutation_ns, 99) as f64 / 1_000_000.0;

    let started = Instant::now();
    opened.engine.transition_projection_generation_for_test().unwrap();
    let configuration_transition_ms = started.elapsed().as_secs_f64() * 1_000.0;
    let started = Instant::now();
    opened.engine.read_projection_generation_status().unwrap();
    let post_transition_generation_ms = started.elapsed().as_secs_f64() * 1_000.0;
    let started = Instant::now();
    assert!(opened.engine.read_mutation_projection_status(request).is_err());
    let post_transition_mutation_ms = started.elapsed().as_secs_f64() * 1_000.0;
    let build_variant =
        std::env::var("FATHOM_SLICE40_STATUS_BUILD_VARIANT").unwrap_or_else(|_| "cpu".into());
    let query_plans = opened.engine.projection_generation_status_query_plans_for_test().unwrap();
    println!(
        "{}",
        json!({
            "schema_version": "scale-02-slice40-status.v2",
            "build_variant": build_variant,
            "status_compute_device": "cpu",
            "measurement_embedder": "fixed-test-cpu",
            "records": records,
            "warmups": warmups,
            "samples": samples,
            "errors": 0,
            "timeouts": 0,
            "cold_generation_ms": cold_generation_ms,
            "cold_mutation_ms": cold_mutation_ms,
            "configuration_transition_ms": configuration_transition_ms,
            "post_transition_generation_ms": post_transition_generation_ms,
            "post_transition_mutation_ms": post_transition_mutation_ms,
            "generation_p95_ms": generation_p95_ms,
            "generation_p99_ms": generation_p99_ms,
            "mutation_p95_ms": mutation_p95_ms,
            "mutation_p99_ms": mutation_p99_ms,
            "steady_full_owner_scans": steady_full_owner_scans,
            "epoch_rollover_full_owner_scans": epoch_rollover_full_owner_scans,
            "query_plans": query_plans,
        })
    );
    assert!(generation_p95_ms <= 5.0, "generation status p95 exceeded 5 ms");
    assert!(generation_p99_ms <= 10.0, "generation status p99 exceeded 10 ms");
    assert!(mutation_p95_ms <= 5.0, "mutation status p95 exceeded 5 ms");
    assert!(mutation_p99_ms <= 10.0, "mutation status p99 exceeded 10 ms");
    assert_eq!(steady_full_owner_scans, 0, "steady calls may not rescan every owner");
}
