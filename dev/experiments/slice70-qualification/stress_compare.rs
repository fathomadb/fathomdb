//! Diagnose only serialized-provider contention on an existing drained EU7 database.
#[path = "../../../src/rust/crates/fathomdb-engine/tests/support/eu7_bge.rs"]
mod canonical;
#[path = "../../../src/rust/crates/fathomdb-engine/tests/support/corpus_subset.rs"]
mod corpus_subset;
use corpus_subset::{load_subset_or_skip, Doc};
use fathomdb_embedder::CandleBgeEmbedder;
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{Engine, SearchResult, TestEmbedderChoice};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant};
use std::{fs, thread};

struct ComparedProvider {
    inner: CandleBgeEmbedder,
    lock: Option<Mutex<()>>,
}
impl Embedder for ComparedProvider {
    fn identity(&self) -> EmbedderIdentity {
        self.inner.identity()
    }
    fn truncation_offset(&self, text: &str) -> Result<Option<usize>, EmbedderError> {
        self.inner.truncation_offset(text)
    }
    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let _held = self.lock.as_ref().map(|lock| lock.lock().expect("provider lock"));
        self.inner.embed(text)
    }
    fn embed_batch(&self, inputs: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        let _held = self.lock.as_ref().map(|lock| lock.lock().expect("provider lock"));
        self.inner.embed_batch(inputs)
    }
}
fn digest(path: &Path) -> String {
    Sha256::digest(fs::read(path).expect("read input"))
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
struct SplitMix64 {
    state: u64,
}
impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn next_in(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

const QUERY_SELECT_SEED: u64 = 0x000E_77C0_125E_1EC7; // EU-7 query selection
fn percentile_ceil(samples: &[Duration], numerator: usize, denominator: usize) -> Duration {
    assert!(!samples.is_empty());
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() * numerator).div_ceil(denominator)).saturating_sub(1);
    sorted[index]
}

// ── Query synthesis (EU-0 §1.2 methodology) ────────────────────────────
// Title if usable (>= 6 chars, not "untitled", not equal to body);
// otherwise the lead sentence / first ~140 chars at a sentence boundary.
// The synthesized text must NOT equal the body verbatim (else the query
// is the document and recall is trivially self-fulfilling).
const LEAD_MAX_CHARS: usize = 140;

fn synth_query(doc: &Doc) -> Option<String> {
    if let Some(title) = &doc.title {
        let t = title.trim();
        if t.len() >= 6 && !t.eq_ignore_ascii_case("untitled") && t != doc.body.trim() {
            return Some(t.to_string());
        }
    }
    let body = doc.body.trim();
    if body.is_empty() {
        return None;
    }
    let lead = lead_sentence(body, LEAD_MAX_CHARS);
    // Skip docs whose entire body is shorter than/equal to the lead we'd
    // extract — we can't form a non-verbatim query from them.
    if lead.trim().is_empty() || lead.trim() == body {
        return None;
    }
    Some(lead)
}

/// First sentence (up to `.`/`!`/`?`/newline) or `max_chars` at a char
/// boundary, whichever comes first. Skips leading markdown bullet noise.
fn lead_sentence(body: &str, max_chars: usize) -> String {
    let cleaned: String = body
        .lines()
        .map(|l| l.trim_start_matches(['-', '*', '#', '>', ' ', '\t']))
        .collect::<Vec<_>>()
        .join(" ");
    let cleaned = cleaned.trim();
    let mut out = String::new();
    for (i, ch) in cleaned.chars().enumerate() {
        if i >= max_chars {
            break;
        }
        out.push(ch);
        if matches!(ch, '.' | '!' | '?') && out.trim().len() >= 12 {
            break;
        }
    }
    out.trim().to_string()
}

struct QueryItem {
    text: String,
}

/// Load all real docs (deterministic order), build the fixed query set
/// from the first 1,000 real docs (present in every N >= 1000 so recall
/// is apples-to-apples across haystack sizes), and return both.
fn load_real_and_queries(num_queries: usize) -> Option<(Vec<Doc>, Vec<QueryItem>)> {
    // per_source = usize::MAX -> load the full corpus (~7,667 docs).
    let real = load_subset_or_skip(usize::MAX)?;
    if real.is_empty() {
        return None;
    }
    // Query-target pool: first up-to-1000 real docs (the floor of the N
    // sweep), so the same query set is valid at every N.
    let pool_len = real.len().min(1000);
    let mut indices: Vec<usize> = (0..pool_len).collect();
    // Deterministic Fisher-Yates shuffle so the 100 queries spread across
    // sources rather than clustering at the head of the sorted list.
    let mut rng = SplitMix64::new(QUERY_SELECT_SEED);
    for i in (1..indices.len()).rev() {
        let j = rng.next_in(i + 1);
        indices.swap(i, j);
    }
    let mut queries = Vec::with_capacity(num_queries);
    for &idx in &indices {
        if queries.len() >= num_queries {
            break;
        }
        if let Some(text) = synth_query(&real[idx]) {
            queries.push(QueryItem { text });
        }
    }
    Some((real, queries))
}

fn checked(result: SearchResult, expected: &[String]) {
    assert!(result.soft_fallback.is_none() && result.degradations.is_empty());
    assert_eq!(result.results.iter().map(|hit| hit.id.to_prefixed()).collect::<Vec<_>>(), expected);
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 5, "MODE SNAPSHOT EXPECTED_SHA FRESH_DIRECTORY");
    assert!(matches!(args[1].as_str(), "serialized" | "direct" | "historical-direct"));
    assert_eq!(std::env::var("FATHOMDB_EMBED_DEVICE").as_deref(), Ok("cpu"));
    assert_eq!(std::env::var("CUDA_VISIBLE_DEVICES").as_deref(), Ok(""));
    let snapshot = Path::new(&args[2]);
    assert_eq!(digest(snapshot), args[3]);
    let directory = Path::new(&args[4]);
    fs::create_dir(directory).expect("new arm directory");
    let database = directory.join("fathomdb.sqlite");
    fs::copy(snapshot, &database).expect("copy snapshot");
    assert_eq!(digest(&database), args[3]);
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Performance)
        .expect("runtime before SQLite");
    let connection = rusqlite::Connection::open_with_flags(
        &database,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    for table in ["canonical_nodes", "_fathomdb_vector_rows"] {
        let count: usize = connection
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 7667);
    }
    drop(connection);
    let (_, queries) = load_real_and_queries(100).expect("original real corpus");
    assert_eq!(queries.len(), 100);
    let texts: Arc<Vec<_>> = Arc::new(queries.into_iter().map(|q| q.text).collect());
    let query_sha = Sha256::digest(serde_json::to_vec(&*texts).unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let inner = CandleBgeEmbedder::new().expect("real CPU BGE");
    let provider: Arc<dyn Embedder> = if args[1] == "historical-direct" {
        Arc::new(canonical::Eu7Bge::new(inner))
    } else {
        Arc::new(ComparedProvider {
            inner,
            lock: (args[1] == "serialized").then(|| Mutex::new(())),
        })
    };
    let opened =
        Engine::open_with_test_choice_for_test(&database, TestEmbedderChoice::Caller(provider))
            .expect("engine open");
    assert_eq!(opened.report.default_embedder.name, "fathomdb-bge-small-en-v1.5");
    let engine = Arc::new(opened.engine);
    engine.set_search_limit_for_test(15);
    engine.set_vector_stage_only_for_test(false);
    let mut witnesses = Vec::new();
    let mut expected = Vec::new();
    for text in texts.iter() {
        let result = engine
            .search_explained_with_limit(text, None, 0, false, 0.6, 20, 10)
            .expect("witness search");
        assert!(result.soft_fallback.is_none() && result.degradations.is_empty());
        let vector_hits = result.explanation.as_ref().unwrap().trace.vector_hits;
        assert!(vector_hits > 0);
        let ids = result.results.iter().map(|hit| hit.id.to_prefixed()).collect::<Vec<_>>();
        witnesses.push(json!({"ids": ids, "vector_hits": vector_hits}));
        expected.push(ids);
    }
    fs::write(directory.join("witnesses.json"), serde_json::to_vec_pretty(&witnesses).unwrap())
        .unwrap();
    let expected = Arc::new(expected);
    for i in 0..1000 {
        checked(engine.search(&texts[i % 100]).expect("warmup"), &expected[i % 100]);
    }
    let mut baseline = Vec::new();
    for i in 0..1000 {
        let start = Instant::now();
        let result = engine.search(&texts[i % 100]).expect("baseline");
        baseline.push(start.elapsed());
        checked(result, &expected[i % 100]);
    }
    let baseline_p99 = percentile_ceil(&baseline, 99, 100);
    eprintln!("BASELINE mode={} p99_ms={}", args[1], baseline_p99.as_millis());
    let barrier = Arc::new(Barrier::new(9));
    let mut handles = Vec::new();
    for tid in 0..8 {
        let (engine, texts, barrier, expected) =
            (engine.clone(), texts.clone(), barrier.clone(), expected.clone());
        handles.push(thread::spawn(move || {
            let mut samples = Vec::new();
            barrier.wait();
            for i in 0..250 {
                let start = Instant::now();
                let index = (tid * 250 + i) % 100;
                let result = engine.search(&texts[index]).expect("stress");
                samples.push(start.elapsed());
                checked(result, &expected[index]);
            }
            samples
        }));
    }
    barrier.wait();
    let all: Vec<_> = handles.into_iter().flat_map(|h| h.join().unwrap()).collect();
    assert_eq!(all.len(), 2000);
    let stress = percentile_ceil(&all, 99, 100);
    let bound = (baseline_p99 * 10).max(Duration::from_millis(150));
    println!(
        "{}",
        json!({"mode":args[1], "snapshot_sha256":args[3],"query_sha256":query_sha,"baseline_samples":1000,"stress_samples":all.len(),"baseline_p99_us":baseline_p99.as_micros(),"stress_p99_us":stress.as_micros(),"bound_us":bound.as_micros(),"passed":stress<=bound})
    );
}
