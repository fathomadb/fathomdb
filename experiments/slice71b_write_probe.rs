//! Versioned external write probe for the bounded 0.8.25 Slice 71B campaign.
//!
//! The runner compiles this source in a temporary crate whose path dependencies
//! point at an exact product checkout. The probe is therefore identical across
//! historical, unchanged-current, and corrected product sources.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{Engine, InitialState, PreparedWrite, SourceId};
use rusqlite::Connection;
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fixture {
    Scale02,
    Ac013,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Treatment {
    Production,
    GenerationOnly,
    NoOp,
}

#[derive(Debug)]
struct Args {
    fixture: Fixture,
    treatment: Treatment,
    records: usize,
    batch_size: usize,
    database: PathBuf,
    input_jsonl: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut fixture = None;
    let mut treatment = None;
    let mut records = None;
    let mut batch_size = None;
    let mut database = None;
    let mut input_jsonl = None;
    let mut arguments = std::env::args().skip(1);
    while let Some(flag) = arguments.next() {
        let value = arguments.next().ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--fixture" => {
                fixture = Some(match value.as_str() {
                    "scale02" => Fixture::Scale02,
                    "ac013" => Fixture::Ac013,
                    _ => return Err("fixture must be scale02 or ac013".to_string()),
                });
            }
            "--treatment" => {
                treatment = Some(match value.as_str() {
                    "production" => Treatment::Production,
                    "generation_only" => Treatment::GenerationOnly,
                    "no_op" => Treatment::NoOp,
                    _ => {
                        return Err(
                            "treatment must be production, generation_only, or no_op".to_string()
                        )
                    }
                });
            }
            "--records" => {
                records = Some(value.parse::<usize>().map_err(|_| "records must be an integer")?)
            }
            "--batch-size" => {
                batch_size =
                    Some(value.parse::<usize>().map_err(|_| "batch-size must be an integer")?)
            }
            "--database" => database = Some(PathBuf::from(value)),
            "--input-jsonl" => input_jsonl = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    let args = Args {
        fixture: fixture.ok_or("missing --fixture")?,
        treatment: treatment.ok_or("missing --treatment")?,
        records: records.ok_or("missing --records")?,
        batch_size: batch_size.ok_or("missing --batch-size")?,
        database: database.ok_or("missing --database")?,
        input_jsonl,
    };
    if args.records == 0 || args.batch_size == 0 {
        return Err("records and batch-size must be positive".to_string());
    }
    if args.fixture == Fixture::Scale02 && args.input_jsonl.is_none() {
        return Err("scale02 requires --input-jsonl".to_string());
    }
    Ok(args)
}

#[derive(Clone, Debug)]
struct VaryingEmbedder {
    identity: EmbedderIdentity,
    dim: u32,
}

impl VaryingEmbedder {
    fn new(dim: u32) -> Self {
        Self { identity: EmbedderIdentity::new("varying", "perf-gates-dense", dim), dim }
    }

    fn vector_for(&self, text: &str) -> Vector {
        let mut hash = 0xcbf29ce484222325_u64;
        for &byte in text.as_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        if hash == 0 {
            hash = 0xdeadbeef_cafebabe;
        }
        let mut state = hash;
        let mut vector = Vec::with_capacity(self.dim as usize);
        for _ in 0..self.dim {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            vector.push(((state as u32) as i32 as f32) / (i32::MAX as f32 + 1.0));
        }
        let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt().max(1e-6);
        for value in &mut vector {
            *value /= norm;
        }
        vector
    }
}

impl Embedder for VaryingEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        self.identity.clone()
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        Ok(self.vector_for(text))
    }
}

struct SeededRng {
    state: u64,
}

impl SeededRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1) }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.state
    }

    fn next_in(&mut self, bound: usize) -> usize {
        (self.next_u64() as usize) % bound
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1_u64 << 53) as f64)
    }
}

fn perf_vocab() -> Vec<String> {
    (0..1024)
        .map(|index| {
            let a = (b'a' + ((index / 26 / 26) % 26) as u8) as char;
            let b = (b'a' + ((index / 26) % 26) as u8) as char;
            let c = (b'a' + (index % 26) as u8) as char;
            format!("{a}{b}{c}{index:04}")
        })
        .collect()
}

fn zipf_cumulative(size: usize) -> Vec<f64> {
    let mut sum = 0.0;
    (1..=size)
        .map(|rank| {
            sum += 1.0 / rank as f64;
            sum
        })
        .collect()
}

fn zipf_index(rng: &mut SeededRng, cumulative: &[f64]) -> usize {
    let sample = rng.next_f64() * cumulative[cumulative.len() - 1];
    match cumulative
        .binary_search_by(|weight| weight.partial_cmp(&sample).unwrap_or(std::cmp::Ordering::Equal))
    {
        Ok(index) => index,
        Err(index) => index.min(cumulative.len() - 1),
    }
}

fn synth_chunk_body(rng: &mut SeededRng, vocabulary: &[String], cumulative: &[f64]) -> String {
    let mut body = String::with_capacity(512);
    for index in 0..(55 + rng.next_in(20)) {
        if index > 0 {
            body.push(' ');
        }
        body.push_str(&vocabulary[zipf_index(rng, cumulative)]);
    }
    body
}

fn ac013_rows(records: usize) -> Result<Vec<PreparedWrite>, String> {
    let vocabulary = perf_vocab();
    let cumulative = zipf_cumulative(vocabulary.len());
    let mut rng = SeededRng::new(0x0AC0_13D0_13D0);
    let source_id = SourceId::new("test:fixture").map_err(|error| error.to_string())?;
    Ok((0..records)
        .map(|_| PreparedWrite::Node {
            kind: "doc".to_string(),
            body: synth_chunk_body(&mut rng, &vocabulary, &cumulative),
            source_id: source_id.clone(),
            logical_id: None,
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect())
}

fn scale02_rows(path: &Path, records: usize) -> Result<Vec<PreparedWrite>, String> {
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut rows = Vec::with_capacity(records);
    for (index, line) in text.lines().take(records).enumerate() {
        let value: Value = serde_json::from_str(line).map_err(|error| error.to_string())?;
        let field = |name: &str| {
            value[name]
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("line {} lacks {name}", index + 1))
        };
        rows.push(PreparedWrite::Node {
            kind: "scale_02_memory".to_string(),
            body: field("body")?,
            source_id: SourceId::new(field("source_id")?).map_err(|error| error.to_string())?,
            logical_id: Some(field("logical_id")?),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        });
    }
    if rows.len() != records {
        return Err(format!("input contains {} rows, expected {records}", rows.len()));
    }
    Ok(rows)
}

fn visibility(connection: &Connection) -> Result<(i64, String), String> {
    connection
        .query_row(
            "SELECT generation,state_nonce FROM _fathomdb_read_visibility_state WHERE singleton=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| format!("visibility observation failed: {error}"))
}

fn trigger_inventory(connection: &Connection) -> Result<i64, String> {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND name LIKE '_fathomdb_read_visibility_%'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("trigger inventory failed: {error}"))
}

fn sqlite_identity(connection: &Connection) -> Result<(String, String), String> {
    let version = connection
        .query_row("SELECT sqlite_version()", [], |row| row.get(0))
        .map_err(|error| format!("SQLite version query failed: {error}"))?;
    let source_id = connection
        .query_row("SELECT sqlite_source_id()", [], |row| row.get(0))
        .map_err(|error| format!("SQLite source-id query failed: {error}"))?;
    Ok((version, source_id))
}

fn install_treatment(path: &Path, treatment: Treatment) -> Result<(), String> {
    if treatment == Treatment::Production {
        return Ok(());
    }
    let mut connection = Connection::open(path).map_err(|error| error.to_string())?;
    let triggers = {
        let mut statement = connection
            .prepare(
                "SELECT name,tbl_name,sql FROM sqlite_master WHERE type='trigger' AND name LIKE '_fathomdb_read_visibility_%' ORDER BY name",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        rows
    };
    if triggers.is_empty() {
        return Err("diagnostic treatment requires visibility triggers".to_string());
    }
    let transaction = connection.transaction().map_err(|error| error.to_string())?;
    for (name, table, sql) in triggers {
        let operation = if name.ends_with("_ai") {
            "INSERT"
        } else if name.ends_with("_au") {
            "UPDATE"
        } else if name.ends_with("_ad") {
            "DELETE"
        } else {
            return Err(format!("unexpected visibility trigger {name}"));
        };
        transaction
            .execute_batch(&format!("DROP TRIGGER {name}"))
            .map_err(|error| error.to_string())?;
        let replacement = match treatment {
            Treatment::GenerationOnly => {
                let changed = sql.replace(",state_nonce=lower(hex(randomblob(32)))", "");
                if changed == sql {
                    return Err(format!("trigger {name} has no nonce expression"));
                }
                changed
            }
            Treatment::NoOp => {
                format!("CREATE TRIGGER {name} AFTER {operation} ON {table} BEGIN SELECT 1; END")
            }
            Treatment::Production => unreachable!(),
        };
        transaction.execute_batch(&replacement).map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())
}

fn process_usage() -> (f64, u64) {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return (0.000_001, 0);
    }
    let usage = unsafe { usage.assume_init() };
    let seconds = usage.ru_utime.tv_sec as f64
        + usage.ru_utime.tv_usec as f64 / 1_000_000.0
        + usage.ru_stime.tv_sec as f64
        + usage.ru_stime.tv_usec as f64 / 1_000_000.0;
    (seconds.max(0.000_001), (usage.ru_maxrss as u64).saturating_mul(1024))
}

fn storage_bytes(path: &Path) -> (u64, u64) {
    let main = fs::metadata(path).map(|metadata| metadata.len()).unwrap_or(0);
    let wal =
        fs::metadata(format!("{}-wal", path.display())).map(|metadata| metadata.len()).unwrap_or(0);
    (main, wal)
}

fn run(args: &Args) -> Result<Value, String> {
    if args.database.exists() {
        return Err("database path must be fresh".to_string());
    }
    let (engine, rows) = match args.fixture {
        Fixture::Scale02 => {
            let opened = Engine::open_without_embedder_for_test(&args.database)
                .map_err(|error| error.to_string())?;
            let rows = scale02_rows(args.input_jsonl.as_deref().unwrap(), args.records)?;
            (opened.engine, rows)
        }
        Fixture::Ac013 => {
            let opened = Engine::open_with_embedder_for_test(
                &args.database,
                Arc::new(VaryingEmbedder::new(384)),
            )
            .map_err(|error| error.to_string())?;
            opened
                .engine
                .configure_vector_kind_for_test("doc")
                .map_err(|error| error.to_string())?;
            (opened.engine, ac013_rows(args.records)?)
        }
    };
    install_treatment(&args.database, args.treatment)?;
    let observation = Connection::open(&args.database).map_err(|error| error.to_string())?;
    let (sqlite_version, sqlite_source_id) = sqlite_identity(&observation)?;
    let before = visibility(&observation)?;
    let trigger_count = trigger_inventory(&observation)?;
    let cpu_before = process_usage().0;
    let ingest_started = Instant::now();
    for batch in rows.chunks(args.batch_size) {
        engine.write(batch).map_err(|error| error.to_string())?;
    }
    let ingest_ack_ms = ingest_started.elapsed().as_secs_f64() * 1000.0;
    let after_ack = visibility(&observation)?;
    engine.drain(1_800_000).map_err(|error| error.to_string())?;
    let total_ms = ingest_started.elapsed().as_secs_f64() * 1000.0;
    let projection_drain_ms = total_ms - ingest_ack_ms;
    let after_drain = visibility(&observation)?;
    let (cpu_after, peak_rss_bytes) = process_usage();
    let (database_bytes, wal_bytes) = storage_bytes(&args.database);
    Ok(json!({
        "schema_version": "slice71b-write-probe.v1",
        "sqlite_version": sqlite_version,
        "sqlite_source_id": sqlite_source_id,
        "fixture": match args.fixture { Fixture::Scale02 => "scale02", Fixture::Ac013 => "ac013" },
        "treatment": match args.treatment { Treatment::Production => "production", Treatment::GenerationOnly => "generation_only", Treatment::NoOp => "no_op" },
        "records": args.records,
        "batch_size": args.batch_size,
        "transactions": args.records.div_ceil(args.batch_size),
        "ingest_ack_ms": ingest_ack_ms,
        "projection_drain_ms": projection_drain_ms,
        "total_ms": total_ms,
        "generation_before": before.0,
        "generation_after_ack": after_ack.0,
        "generation_after_drain": after_drain.0,
        "nonce_before": before.1,
        "nonce_after_ack": after_ack.1,
        "nonce_after_drain": after_drain.1,
        "trigger_inventory": trigger_count,
        "database_bytes": database_bytes,
        "wal_bytes": wal_bytes,
        "process_cpu_seconds": (cpu_after - cpu_before).max(0.000_001),
        "peak_rss_bytes": peak_rss_bytes,
    }))
}

fn main() {
    let result = parse_args().and_then(|args| run(&args));
    match result {
        Ok(document) => println!("{document}"),
        Err(error) => {
            eprintln!("slice71b probe failed: {error}");
            std::process::exit(1);
        }
    }
}
