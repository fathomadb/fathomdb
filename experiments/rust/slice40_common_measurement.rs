//! Baseline-compatible Slice 40 write, storage, and reopen measurement worker.
//!
//! The Python runner copies this source unchanged into each source checkout so
//! the Slice 35 baseline and Slice 40 candidate execute the same workload code.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ArtifactRevisionId, Engine, InitialState,
    ProjectionRole, ProjectionSpec, ProjectionVector, ProvenancedNodeV1, SourceId, SourceVersionId,
    WriteProvenanceV1,
};
use rusqlite::{Connection, OptionalExtension};
use serde_json::json;

#[derive(Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice40-common-measurement", "v1", 8)
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

fn node(index: usize) -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "doc".into(),
        body: format!("slice40 representative record {index}"),
        source_id: SourceId::new(format!("source:slice40-common-{index}"))
            .expect("valid source ID"),
        logical_id: Some(format!("slice40-common-{index}")),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(format!("slice40-common-r{index}")).expect("valid revision ID"),
            SourceVersionId::new(format!("slice40-common-v{index}"))
                .expect("valid source-version ID"),
        ),
    }
}

fn percentile_ms(values: &mut [u128], percentile: usize) -> f64 {
    values.sort_unstable();
    let index = (values.len() * percentile).div_ceil(100).saturating_sub(1);
    values[index] as f64 / 1_000_000.0
}

fn file_len(path: &Path) -> u64 {
    fs::metadata(path).map_or(0, |metadata| metadata.len())
}

fn table_rows(connection: &Connection, table: &str) -> Option<u64> {
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name=?1)",
            [table],
            |row| row.get(0),
        )
        .expect("inspect table existence");
    exists.then(|| {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))
            .expect("count measurement table")
    })
}

fn main() {
    let arguments = std::env::args().collect::<Vec<_>>();
    assert_eq!(
        arguments.len(),
        7,
        "usage: worker TREATMENT DB RECORDS BATCH OPEN_WARMUPS OPEN_SAMPLES"
    );
    let treatment = &arguments[1];
    let path = PathBuf::from(&arguments[2]);
    let records = arguments[3].parse::<usize>().expect("records");
    let batch_size = arguments[4].parse::<usize>().expect("batch size");
    let open_warmups = arguments[5].parse::<usize>().expect("open warmups");
    let open_samples = arguments[6].parse::<usize>().expect("open samples");
    assert!(records > 0 && batch_size > 0 && open_samples > 0);

    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder))
        .expect("open measurement database");
    opened
        .engine
        .configure_projections(&[vector_spec()], &[])
        .expect("configure vector projection");
    opened.engine.set_projection_scheduler_frozen_for_test(true);

    let mut write_ns = Vec::new();
    let mut receipt_count = 0_u64;
    let mut operation_count = 0_u64;
    let mut pending_receipt_count = 0_u64;
    let mut pending_cursor_count = 0_u64;
    for start in (0..records).step_by(batch_size) {
        let end = (start + batch_size).min(records);
        let operations = (start..end)
            .map(|index| ActuationOperationV1::PutCanonicalNode(node(index)))
            .collect::<Vec<_>>();
        let operation_id = format!("slice40-common-batch-{receipt_count:04}");
        let request = ActuationBatchV1::new(operation_id, operations).expect("valid batch");
        let started = Instant::now();
        let receipt = opened.engine.actuate(request).expect("apply representative batch");
        write_ns.push(started.elapsed().as_nanos());
        receipt_count += 1;
        operation_count += u64::try_from(end - start).expect("operation count");
        if !receipt.pending_projection_write_cursors.is_empty() {
            pending_receipt_count += 1;
        }
        pending_cursor_count +=
            u64::try_from(receipt.pending_projection_write_cursors.len()).expect("pending count");
    }
    let mut p50_values = write_ns.clone();
    let mut p95_values = write_ns;
    let write_p50_ms = percentile_ms(&mut p50_values, 50);
    let write_p95_ms = percentile_ms(&mut p95_values, 95);

    let wal_path = PathBuf::from(format!("{}-wal", path.display()));
    let wal_bytes_before_checkpoint = file_len(&wal_path);
    drop(opened);
    let connection = Connection::open(&path).expect("open for checkpoint");
    connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)").expect("checkpoint WAL");
    let database_bytes_after_checkpoint = file_len(&path);
    let wal_bytes_after_checkpoint = file_len(&wal_path);
    let generation_table_bytes: Option<u64> = connection
        .query_row(
            "SELECT SUM(pgsize) FROM dbstat WHERE name IN ('_fathomdb_projection_generations','_fathomdb_projection_generation_current')",
            [],
            |row| row.get(0),
        )
        .optional()
        .ok()
        .flatten();
    let receipt_table_bytes: Option<u64> = connection
        .query_row(
            "SELECT SUM(pgsize) FROM dbstat WHERE name='_fathomdb_actuation_receipts'",
            [],
            |row| row.get(0),
        )
        .optional()
        .ok()
        .flatten();
    let has_generation_column: bool = connection
        .prepare("SELECT name FROM pragma_table_info('_fathomdb_actuation_receipts') WHERE name='projection_generation_id'")
        .and_then(|mut statement| statement.exists([]))
        .unwrap_or(false);
    let receipt_generation_payload_bytes: Option<u64> = has_generation_column.then(|| {
        connection
            .query_row(
                "SELECT COALESCE(SUM(length(projection_generation_id)),0) FROM _fathomdb_actuation_receipts",
                [],
                |row| row.get(0),
            )
            .expect("generation payload")
    });
    drop(connection);

    let settled = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder))
        .expect("reopen for projection completion");
    settled.engine.drain(120_000).expect("complete pending projections");
    drop(settled);
    let settled_connection = Connection::open(&path).expect("inspect settled database");
    let settled_row_counts = [
        "canonical_nodes",
        "_fathomdb_artifact_revisions",
        "_fathomdb_source_versions",
        "_fathomdb_source_links",
        "_fathomdb_actuation_receipts",
        "_fathomdb_projection_generations",
        "_fathomdb_projection_terminal",
        "_fathomdb_vector_rows",
        "vector_default",
    ]
    .into_iter()
    .map(|table| (table.to_string(), json!(table_rows(&settled_connection, table))))
    .collect::<serde_json::Map<_, _>>();
    drop(settled_connection);

    let mut open_ns = Vec::with_capacity(open_samples);
    for index in 0..(open_warmups + open_samples) {
        let started = Instant::now();
        let reopened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder))
            .expect("timed reopen");
        let elapsed = started.elapsed().as_nanos();
        drop(reopened);
        if index >= open_warmups {
            open_ns.push(elapsed);
        }
    }
    let mut open_p95_values = open_ns.clone();
    let open_p95_ms = percentile_ms(&mut open_p95_values, 95);

    println!(
        "{}",
        json!({
            "schema_version": "scale-02-slice40-worker.v1",
            "treatment": treatment,
            "records": records,
            "batch_size": batch_size,
            "receipt_count": receipt_count,
            "operation_count": operation_count,
            "pending_receipt_count": pending_receipt_count,
            "pending_cursor_count": pending_cursor_count,
            "write_p50_ms": write_p50_ms,
            "write_p95_ms": write_p95_ms,
            "open_p95_ms": open_p95_ms,
            "database_bytes_after_checkpoint": database_bytes_after_checkpoint,
            "wal_bytes_before_checkpoint": wal_bytes_before_checkpoint,
            "wal_bytes_after_checkpoint": wal_bytes_after_checkpoint,
            "generation_table_bytes": generation_table_bytes,
            "receipt_table_bytes": receipt_table_bytes,
            "receipt_generation_payload_bytes": receipt_generation_payload_bytes,
            "settled_row_counts": settled_row_counts,
            "errors": 0,
            "timeouts": 0,
        })
    );
}
