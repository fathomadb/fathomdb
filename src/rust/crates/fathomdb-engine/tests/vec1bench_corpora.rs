//! Opt-in full-scale retrieval measurements for the four vec1bench corpora.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{Engine, InitialState, PreparedWrite, SourceId};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

const REGISTRY: &str =
    include_str!("../../../../../dev/vector-bench-corpora/vec1bench/datasets.json");
const VECTOR_EQUIVALENCE_PROBES: &str = include_str!("../src/vector_equivalence_probes.txt");
const QUERY_SAMPLES: usize = 100;
const INGEST_BATCH: usize = 512;
const DRAIN_TIMEOUT_MS: u64 = 7_200_000;

#[derive(Deserialize)]
struct Registry {
    datasets: Vec<DatasetSpec>,
}

#[derive(Deserialize)]
struct DatasetSpec {
    id: String,
    sha256: String,
    train_rows: usize,
    dimension: usize,
    source_distance: String,
    evaluation_distance: String,
    fathom_transform: String,
}

#[derive(Deserialize)]
struct PreparedCorpus {
    schema_version: u32,
    dataset_id: String,
    source_sha256: String,
    train_rows: usize,
    test_rows: usize,
    dimension: usize,
    source_distance: String,
    fathom_distance: String,
    transform: String,
    train_sha256: String,
    test_sha256: String,
    neighbors10_sha256: String,
}

struct CorpusEmbedder {
    identity: EmbedderIdentity,
    train: Mutex<File>,
    queries: Vec<Vector>,
    dimension: usize,
}

impl CorpusEmbedder {
    fn new(train: File, queries: Vec<Vector>, spec: &DatasetSpec) -> Self {
        Self {
            identity: EmbedderIdentity::new(
                format!("vec1bench/{}", spec.id),
                format!("{}:{}", spec.sha256, spec.fathom_transform),
                spec.dimension as u32,
            ),
            train: Mutex::new(train),
            queries,
            dimension: spec.dimension,
        }
    }

    fn data_vector(&self, index: usize) -> Result<Vector, EmbedderError> {
        let mut file = self.train.lock().map_err(|_| EmbedderError::Failed {
            message: "corpus vector file lock was poisoned".to_owned(),
        })?;
        read_vector_at(&mut file, index, self.dimension).map_err(|error| EmbedderError::Failed {
            message: format!("cannot read corpus vector {index}: {error}"),
        })
    }

    fn probe_vector(&self, input: &str) -> Vector {
        let mut digest = Sha256::new();
        for field in [&self.identity.name, &self.identity.revision, input] {
            digest.update((field.len() as u64).to_le_bytes());
            digest.update(field.as_bytes());
        }
        let hash = digest.finalize();
        let mut state = u64::from_le_bytes(hash[..8].try_into().expect("SHA256 prefix")) | 1;
        (0..self.dimension)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                ((state >> 40) as f32 / 8_388_608.0) - 1.0
            })
            .collect()
    }
}

impl Embedder for CorpusEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        self.identity.clone()
    }

    fn embed(&self, input: &str) -> Result<Vector, EmbedderError> {
        if let Some(index) = input.strip_prefix("vecdata") {
            let index = index.parse::<usize>().map_err(|error| EmbedderError::Failed {
                message: format!("invalid corpus row ID: {error}"),
            })?;
            return self.data_vector(index);
        }
        if let Some(index) = input.strip_prefix("vecquery") {
            let index = index.parse::<usize>().map_err(|error| EmbedderError::Failed {
                message: format!("invalid query row ID: {error}"),
            })?;
            return self.queries.get(index).cloned().ok_or_else(|| EmbedderError::Failed {
                message: format!("query row {index} is outside the prepared corpus"),
            });
        }
        if VECTOR_EQUIVALENCE_PROBES
            .lines()
            .any(|probe| !probe.is_empty() && !probe.starts_with('#') && probe == input)
        {
            return Ok(self.probe_vector(input));
        }
        Err(EmbedderError::Failed { message: "unknown vec1bench input ID".to_owned() })
    }
}

fn read_vector_at(file: &mut File, index: usize, dimension: usize) -> std::io::Result<Vector> {
    let stride = dimension
        .checked_mul(4)
        .and_then(|value| index.checked_mul(value))
        .ok_or_else(|| std::io::Error::other("vector file offset overflow"))?;
    file.seek(SeekFrom::Start(stride as u64))?;
    let mut bytes = vec![0_u8; dimension * 4];
    file.read_exact(&mut bytes)?;
    Ok(bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("four bytes")))
        .collect())
}

fn file_sha256(path: &Path) -> String {
    let mut stream = File::open(path).expect("open prepared file for SHA256");
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; 8 * 1024 * 1024];
    loop {
        let n = stream.read(&mut buffer).expect("read prepared file for SHA256");
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    digest.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sealed_file(directory: &Path, name: &str, expected_bytes: usize, expected_sha: &str) -> PathBuf {
    let path = directory.join(name);
    let metadata = fs::metadata(&path).unwrap_or_else(|error| {
        panic!("prepared corpus file absent at {}: {error}", path.display())
    });
    assert_eq!(metadata.len(), expected_bytes as u64, "wrong size for {}", path.display());
    assert_eq!(file_sha256(&path), expected_sha, "prepared corpus SHA256 mismatch: {name}");
    path
}

fn load_prepared(
    spec: &DatasetSpec,
    directory: &Path,
) -> (PreparedCorpus, PathBuf, Vec<Vector>, Vec<u32>) {
    let manifest_path = directory.join("prepared.json");
    let manifest_text = fs::read_to_string(&manifest_path).unwrap_or_else(|error| {
        panic!("prepared corpus absent at {}: {error}", manifest_path.display())
    });
    let prepared: PreparedCorpus =
        serde_json::from_str(&manifest_text).expect("prepared corpus JSON");
    assert_eq!(prepared.schema_version, 1);
    assert_eq!(prepared.dataset_id, spec.id);
    assert_eq!(prepared.source_sha256, spec.sha256);
    assert_eq!(prepared.train_rows, spec.train_rows);
    assert_eq!(prepared.dimension, spec.dimension);
    assert_eq!(prepared.source_distance, spec.source_distance);
    assert_eq!(prepared.fathom_distance, "l2");
    assert_eq!(prepared.transform, spec.fathom_transform);
    assert!(prepared.test_rows >= QUERY_SAMPLES);

    let train_bytes = prepared.train_rows * prepared.dimension * 4;
    let test_bytes = prepared.test_rows * prepared.dimension * 4;
    let gold_bytes = prepared.test_rows * 10 * 4;
    let train = sealed_file(directory, "train.f32", train_bytes, &prepared.train_sha256);
    let test = sealed_file(directory, "test.f32", test_bytes, &prepared.test_sha256);
    let gold = sealed_file(directory, "neighbors10.u32", gold_bytes, &prepared.neighbors10_sha256);

    let mut query_file = File::open(test).expect("open test vectors");
    let queries = (0..prepared.test_rows)
        .map(|index| {
            read_vector_at(&mut query_file, index, prepared.dimension).expect("read query vector")
        })
        .collect();
    let raw_gold = fs::read(gold).expect("read exact gold");
    let neighbors = raw_gold
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes(chunk.try_into().expect("four bytes")))
        .collect::<Vec<_>>();
    assert!(neighbors.iter().all(|&id| id < prepared.train_rows as u32));
    (prepared, train, queries, neighbors)
}

fn data_body(index: usize) -> String {
    format!("vecdata{index:07}")
}

fn query_body(index: usize) -> String {
    format!("vecquery{index:07}")
}

fn query_sample(n_test: usize) -> Vec<usize> {
    (0..QUERY_SAMPLES).map(|i| i * (n_test - 1) / (QUERY_SAMPLES - 1)).collect()
}

fn nearest_rank(samples: &[u128], numerator: usize, denominator: usize) -> u128 {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let index = (sorted.len() * numerator).div_ceil(denominator).saturating_sub(1);
    sorted[index]
}

fn run_dataset(dataset_id: &str) {
    let root = std::env::var_os("FATHOMDB_VEC1BENCH_ROOT").unwrap_or_else(|| {
        panic!("FATHOMDB_VEC1BENCH_ROOT is required for opt-in vec1bench tests")
    });
    let registry: Registry = serde_json::from_str(REGISTRY).expect("vec1bench registry JSON");
    let spec =
        registry.datasets.iter().find(|item| item.id == dataset_id).expect("registered dataset");
    let directory = PathBuf::from(root).join(dataset_id);
    let (prepared, train, queries, neighbors) = load_prepared(spec, &directory);
    let embedder = Arc::new(CorpusEmbedder::new(
        File::open(train).expect("open train vectors"),
        queries,
        spec,
    ));
    let database = directory.join("fathom-benchmark.sqlite");
    let opened =
        Engine::open_with_embedder_for_test(&database, embedder).expect("open benchmark database");
    assert!(
        !opened.report.dense_disabled,
        "benchmark database cannot serve dense retrieval: {:?}",
        opened.report.dense_disabled_reason
    );
    let engine = &opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("configure vector kind");

    // Batches commit in source-row order, so a stopped run resumes at the next
    // canonical row; the projection drain below also catches any outstanding work.
    let existing = engine
        .query_i64_col_for_test("SELECT COUNT(*) FROM canonical_nodes WHERE kind = 'doc'")
        .expect("count ingested corpus nodes")[0] as usize;
    assert!(existing <= prepared.train_rows, "benchmark database has too many corpus rows");
    let seed_started = Instant::now();
    for start in (existing..prepared.train_rows).step_by(INGEST_BATCH) {
        let end = (start + INGEST_BATCH).min(prepared.train_rows);
        let batch = (start..end)
            .map(|index| PreparedWrite::Node {
                kind: "doc".to_owned(),
                body: data_body(index),
                source_id: SourceId::new("benchmark:vec1bench").expect("benchmark source ID"),
                logical_id: Some(data_body(index)),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect::<Vec<_>>();
        engine.write(&batch).expect("ingest pinned vector corpus batch");
    }
    let seed_elapsed = seed_started.elapsed();
    let drain_started = Instant::now();
    engine.drain(DRAIN_TIMEOUT_MS).expect("drain all corpus projections");
    let drain_elapsed = drain_started.elapsed();
    assert_eq!(
        engine.vector_row_count_for_test().expect("count vector rows") as usize,
        prepared.train_rows,
        "all source vectors must be present before any measurement"
    );

    engine.set_vector_stage_only_for_test(true);
    let indices = query_sample(prepared.test_rows);
    for &index in indices.iter().take(5) {
        for limit in [1, 10] {
            assert_eq!(
                engine.search_with_limit(&query_body(index), limit).expect("warmup").results.len(),
                limit
            );
        }
    }
    let mut latency_at_1_ns = Vec::with_capacity(indices.len());
    let mut latency_at_10_ns = Vec::with_capacity(indices.len());
    let mut top1_hits = 0_usize;
    let mut top10_hits = 0_usize;
    for &index in &indices {
        let query = query_body(index);
        let started_at_1 = Instant::now();
        let result_at_1 =
            engine.search_with_limit(&query, 1).expect("measured top-1 vector search");
        latency_at_1_ns.push(started_at_1.elapsed().as_nanos());
        assert_eq!(result_at_1.results.len(), 1, "query returned incomplete top-1");
        let started_at_10 = Instant::now();
        let result_at_10 =
            engine.search_with_limit(&query, 10).expect("measured top-10 vector search");
        latency_at_10_ns.push(started_at_10.elapsed().as_nanos());
        assert_eq!(result_at_10.results.len(), 10, "query returned incomplete top-10");
        let ranked = result_at_10
            .results
            .iter()
            .map(|hit| {
                hit.body
                    .strip_prefix("vecdata")
                    .expect("result must be from prepared corpus")
                    .parse::<u32>()
                    .expect("numeric corpus row ID")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            ranked.iter().copied().collect::<std::collections::HashSet<_>>().len(),
            10,
            "vector-stage search returned duplicate corpus row IDs"
        );
        let exact = &neighbors[index * 10..(index + 1) * 10];
        let first = result_at_1.results[0]
            .body
            .strip_prefix("vecdata")
            .expect("top-1 result must be from prepared corpus")
            .parse::<u32>()
            .expect("numeric corpus row ID");
        top1_hits += usize::from(first == exact[0]);
        top10_hits += ranked.iter().filter(|id| exact.contains(id)).count();
    }
    let recall_at_1 = top1_hits as f64 / indices.len() as f64;
    let recall_at_10 = top10_hits as f64 / (indices.len() * 10) as f64;
    let p50_at_1_ns = nearest_rank(&latency_at_1_ns, 50, 100);
    let p99_at_1_ns = nearest_rank(&latency_at_1_ns, 99, 100);
    let p50_at_10_ns = nearest_rank(&latency_at_10_ns, 50, 100);
    let p99_at_10_ns = nearest_rank(&latency_at_10_ns, 99, 100);
    let report = json!({
        "schema_version": 1,
        "dataset_id": dataset_id,
        "source_sha256": spec.sha256,
        "train_rows": prepared.train_rows,
        "dimension": prepared.dimension,
        "source_distance": prepared.source_distance,
        "evaluation_distance": spec.evaluation_distance,
        "fathom_distance": "l2",
        "transform": prepared.transform,
        "fathomdb_version": env!("CARGO_PKG_VERSION"),
        "retrieval": "FathomDB search_with_limit vector-stage-only test seam; product bit-KNN plus f32 rerank supplies results, with FTS/fusion excluded from ranking",
        "latency_boundary": "warm in-process Engine::search_with_limit, including in-memory query-vector lookup, SQLite reader, bit-KNN, f32 rerank, possible empty FTS lookup and result hydration; excluding ingestion and exact-gold lookup",
        "query_selection": "100 evenly spaced held-out HDF5 test rows, including first and last; separate top-1 and top-10 searches",
        "warmup_queries_per_limit": 5,
        "measured_queries_per_limit": indices.len(),
        "existing_rows_before_run": existing,
        "incremental_seed_write_ms": seed_elapsed.as_millis(),
        "incremental_projection_drain_ms": drain_elapsed.as_millis(),
        "recall_at_1": recall_at_1,
        "recall_at_10": recall_at_10,
        "latency_top1_p50_ns": p50_at_1_ns,
        "latency_top1_p99_ns": p99_at_1_ns,
        "latency_top10_p50_ns": p50_at_10_ns,
        "latency_top10_p99_ns": p99_at_10_ns,
        "latency_top1_samples_ns": latency_at_1_ns,
        "latency_top10_samples_ns": latency_at_10_ns,
    });
    let temp = directory.join("fathom-benchmark-result.json.tmp");
    let final_path = directory.join("fathom-benchmark-result.json");
    fs::write(&temp, format!("{}\n", serde_json::to_string_pretty(&report).expect("report JSON")))
        .expect("write benchmark result");
    fs::rename(temp, &final_path).expect("publish benchmark result");
    eprintln!(
        "VEC1BENCH_RESULT dataset={dataset_id} n={} q={} recall@1={recall_at_1:.4} recall@10={recall_at_10:.4} top1_p50_ms={:.3} top1_p99_ms={:.3} top10_p50_ms={:.3} top10_p99_ms={:.3} result={}",
        prepared.train_rows,
        indices.len(),
        p50_at_1_ns as f64 / 1_000_000.0,
        p99_at_1_ns as f64 / 1_000_000.0,
        p50_at_10_ns as f64 / 1_000_000.0,
        p99_at_10_ns as f64 / 1_000_000.0,
        final_path.display(),
    );
}

fn run_tiny_fixture() {
    let directory = tempfile::tempdir().expect("temporary real-Engine corpus");
    let path = directory.path().join("tiny-train.f32");
    let vectors: [[f32; 8]; 4] = [
        [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ];
    let mut file = File::create(&path).expect("create tiny corpus");
    for vector in vectors {
        for value in vector {
            file.write_all(&value.to_le_bytes()).expect("write tiny vector");
        }
    }
    drop(file);
    let spec = DatasetSpec {
        id: "tiny-real-engine".to_owned(),
        sha256: "tiny-fixture".to_owned(),
        train_rows: 4,
        dimension: 8,
        source_distance: "euclidean".to_owned(),
        evaluation_distance: "l2".to_owned(),
        fathom_transform: "identity".to_owned(),
    };
    let make_embedder = || {
        Arc::new(CorpusEmbedder::new(
            File::open(&path).expect("open tiny corpus"),
            vec![vec![0.9, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]],
            &spec,
        ))
    };
    let database = directory.path().join("tiny.sqlite");
    let opened = Engine::open_with_embedder_for_test(&database, make_embedder())
        .expect("open real Engine for tiny fixture");
    opened.engine.configure_vector_kind_for_test("doc").expect("configure vector kind");
    for index in 0..4 {
        opened
            .engine
            .write(&[PreparedWrite::Node {
                kind: "doc".to_owned(),
                body: data_body(index),
                source_id: SourceId::new("benchmark:vec1bench").expect("source ID"),
                logical_id: Some(data_body(index)),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .expect("write real vector row");
    }
    opened.engine.drain(30_000).expect("drain tiny vectors");
    assert_eq!(opened.engine.vector_row_count_for_test().expect("count vector rows"), 4);
    opened.engine.set_vector_stage_only_for_test(true);
    let result =
        opened.engine.search_with_limit(&query_body(0), 1).expect("real vector-stage search");
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].body, data_body(0));

    drop(opened);
    let reopened = Engine::open_with_embedder_for_test(&database, make_embedder())
        .expect("reopen populated real Engine for tiny fixture");
    assert!(
        !reopened.report.dense_disabled,
        "reopening a benchmark corpus must preserve dense retrieval: {:?}",
        reopened.report.dense_disabled_reason
    );
    reopened.engine.set_vector_stage_only_for_test(true);
    let after_restart = reopened
        .engine
        .search_with_limit(&query_body(0), 1)
        .expect("real vector-stage search after reopen");
    assert_eq!(after_restart.results.len(), 1);
    assert_eq!(after_restart.results[0].body, data_body(0));

    drop(reopened);
    let verified = Engine::open_with_embedder_for_test(&database, make_embedder())
        .expect("reopen benchmark after probe baseline was established");
    assert!(!verified.report.dense_disabled, "same-identity probe baseline must verify");
    verified.engine.set_vector_stage_only_for_test(true);
    let after_verification = verified
        .engine
        .search_with_limit(&query_body(0), 1)
        .expect("real vector-stage search after probe verification");
    assert_eq!(after_verification.results.len(), 1);
    assert_eq!(after_verification.results[0].body, data_body(0));
}

#[test]
fn tiny_real_engine_fixture_exercises_external_vector_retrieval() {
    run_tiny_fixture();
}

#[test]
#[ignore = "requires explicitly prepared multi-GB source corpus and FATHOMDB_VEC1BENCH_ROOT"]
fn sift_128_euclidean_full_scale() {
    run_dataset("sift-128-euclidean");
}

#[test]
#[ignore = "requires explicitly prepared multi-GB source corpus and FATHOMDB_VEC1BENCH_ROOT"]
fn imagenet_clip_512_normalized_full_scale() {
    run_dataset("imagenet-clip-512-normalized");
}

#[test]
#[ignore = "requires explicitly prepared multi-GB source corpus and FATHOMDB_VEC1BENCH_ROOT"]
fn landmark_dino_768_cosine_full_scale() {
    run_dataset("landmark-dino-768-cosine");
}

#[test]
#[ignore = "requires explicitly prepared multi-GB source corpus and FATHOMDB_VEC1BENCH_ROOT"]
fn agnews_mxbai_1024_euclidean_full_scale() {
    run_dataset("agnews-mxbai-1024-euclidean");
}
