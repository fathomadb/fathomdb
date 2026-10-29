//! The AC-013 workload through the `fathomdb-mcp` server and its admission
//! queue (`dev/experiments/ac013-page-cache/`). Needs `--features
//! default-embedder`: the server can only open a store with the pinned
//! default embedder, so the store is embedded with it (384-d) rather than with
//! the gate's 768-d `VaryingEmbedder`, and every search embeds its query.
//!
//! ```text
//! ac013_mcp seed <store.sqlite>
//! ac013_mcp drain <store.sqlite>
//! ac013_mcp sdk  <store.sqlite> <out-dir>
//! ac013_mcp mcp  <store.sqlite> <fathomdb-mcp binary> <out-dir> [levels] [requests]
//! ```
//!
//! - `seed` writes the gate's corpus (n = `AC013_CORPUS_N`, default 10,000;
//!   same generator, corpus seed and 1,024-row batches) into a searchable
//!   vector projection on kind `doc`, drains, and checks that every node has a
//!   vector row. `drain` reopens a store whose seed drain timed out
//!   (`AC013_DRAIN_TIMEOUT_MS`) and finishes it.
//! - `sdk` opens the store with the default embedder in this process and runs
//!   the gate's loop on `Engine::search` (1,000 warm-up, 1,000 timed, limit
//!   10), then times `Engine::embed` alone on the same queries.
//! - `mcp` spawns the real server over stdio (store eager, hybrid default,
//!   every queue setting at its default) and sends `search` calls with
//!   `mode: hybrid` and `limit: 10`, without a progress token. Phase `c1` is
//!   the gate's loop. Then, for each level in `levels` (default
//!   `4,8,16,32`), it keeps that many calls in flight until `requests`
//!   (default 1,000) have completed. The server inherits the environment;
//!   `FATHOMDB_MCP_LOG` defaults to `debug` so its "admitted after a wait"
//!   lines give the queue wait of every call that waited.
//!
//! Env knobs: `AC013_CORPUS_N` (seed; default 10,000), `AC013_SAMPLES`
//! (warm-up and timed calls per c1 pass, default 1,000; also the default
//! `requests`), `FATHOMDB_MCP_LOG` (server log level, default `debug`) and the
//! engine's `FATHOMDB_PERF_*` PRAGMA levers, which the server inherits.
//!
//! Each command prints one JSON summary line; `sdk`/`mcp` also write raw
//! per-request samples (microseconds, one per line) to `<out-dir>`. In the
//! `mcp` summary, `ok` is end-to-end latency of the successful calls as the
//! client saw it (write of the request line to read of the response line);
//! `queue_wait` comes from the server's log and covers only the calls that
//! waited for a permit, so per-call service time is not observable, only its
//! mean (`ok.mean_ms - queue_wait.mean_over_all_calls_ms`); `errors` counts
//! tool errors by `class/code[/busy reason]`.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use fathomdb_sdk::{
    read, Engine, InitialState, OpenOptions, PreparedWrite, ProjectionRole, ProjectionSpec,
    ProjectionVector, SearchOptions, SourceId,
};
use serde_json::{json, Value};

/// Warm-up and timed searches per pass; `AC013_SAMPLES` overrides (gate: 1,000).
fn samples() -> usize {
    std::env::var("AC013_SAMPLES").ok().and_then(|v| v.parse().ok()).unwrap_or(1_000)
}
const CORPUS_SEED: u64 = 0x0AC0_13D0_13D0;
const QUERY_SEED: u64 = 0x0AC0_130D_EC0D_E000;
const KIND: &str = "doc";
const LIMIT: u64 = 10;

// --- The gate's generator (perf_gates.rs), copied. --------------------------

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
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }
}

fn perf_vocab() -> Vec<String> {
    (0..1024)
        .map(|i: usize| {
            let a = (b'a' + ((i / 26 / 26) % 26) as u8) as char;
            let b = (b'a' + ((i / 26) % 26) as u8) as char;
            let c = (b'a' + (i % 26) as u8) as char;
            format!("{a}{b}{c}{i:04}")
        })
        .collect()
}

fn zipf_cumulative(vocab_size: usize) -> Vec<f64> {
    let mut acc = 0.0_f64;
    (1..=vocab_size)
        .map(|k| {
            acc += 1.0_f64 / k as f64;
            acc
        })
        .collect()
}

fn zipf_index(rng: &mut SeededRng, cumulative: &[f64]) -> usize {
    let r = rng.next_f64() * cumulative[cumulative.len() - 1];
    match cumulative.binary_search_by(|w| w.partial_cmp(&r).unwrap_or(std::cmp::Ordering::Equal)) {
        Ok(idx) => idx,
        Err(idx) => idx.min(cumulative.len() - 1),
    }
}

fn synth_chunk_body(rng: &mut SeededRng, vocab: &[String], cumulative: &[f64]) -> String {
    let mut body = String::with_capacity(512);
    let token_count = 55 + rng.next_in(20);
    for i in 0..token_count {
        if i > 0 {
            body.push(' ');
        }
        body.push_str(&vocab[zipf_index(rng, cumulative)]);
    }
    body
}

fn bodies(seed: u64, count: usize) -> Vec<String> {
    let vocab = perf_vocab();
    let cumulative = zipf_cumulative(vocab.len());
    let mut rng = SeededRng::new(seed);
    (0..count).map(|_| synth_chunk_body(&mut rng, &vocab, &cumulative)).collect()
}

/// FNV-1a over the texts, each followed by a 0 byte; the engine probe
/// (`fathomdb-engine/tests/exp_ac013_page_cache.rs`) prints the same digest.
fn digest(texts: &[String]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for text in texts {
        for &b in text.as_bytes().iter().chain(std::iter::once(&0u8)) {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    format!("{h:016x}")
}

/// `AC013_DRAIN_TIMEOUT_MS`, default 30 min (the gate's default). Embedding
/// 10,000 rows with the default embedder on a busy CPU can take longer; a
/// timed-out `seed` is finished with `drain`.
fn drain_timeout_ms() -> u64 {
    std::env::var("AC013_DRAIN_TIMEOUT_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(1_800_000)
}

fn corpus_n() -> usize {
    std::env::var("AC013_CORPUS_N").ok().and_then(|v| v.parse().ok()).unwrap_or(10_000)
}

// --- Statistics. -------------------------------------------------------------

/// The gate's percentile: the ceil(len * p)-th smallest sample.
fn percentile(samples_us: &[u64], numerator: usize, denominator: usize) -> f64 {
    if samples_us.is_empty() {
        return f64::NAN;
    }
    let mut sorted = samples_us.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() * numerator).div_ceil(denominator)).saturating_sub(1);
    sorted[index] as f64 / 1000.0
}

fn summary(samples_us: &[u64]) -> Value {
    let mean = if samples_us.is_empty() {
        f64::NAN
    } else {
        samples_us.iter().sum::<u64>() as f64 / samples_us.len() as f64 / 1000.0
    };
    json!({
        "count": samples_us.len(),
        "p50_ms": percentile(samples_us, 50, 100),
        "p99_ms": percentile(samples_us, 99, 100),
        "mean_ms": mean,
    })
}

fn write_samples(dir: &Path, name: &str, samples_us: &[u64]) {
    let text: String = samples_us.iter().map(|s| format!("{s}\n")).collect();
    std::fs::write(dir.join(name), text).expect("write samples");
}

// --- seed. -------------------------------------------------------------------

fn open(path: &Path) -> Engine {
    Engine::open(path, OpenOptions { use_default_embedder: true, ..Default::default() })
        .unwrap_or_else(|error| {
            panic!("open {} with the default embedder: {error}", path.display())
        })
}

fn vector_coverage(path: &Path) -> (u64, u64) {
    let connection =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("open read-only");
    let nodes: u64 = connection
        .query_row("SELECT COUNT(*) FROM canonical_nodes WHERE kind = ?1", [KIND], |row| row.get(0))
        .expect("count nodes");
    let embedded: u64 = connection
        .query_row(
            "SELECT COUNT(*) FROM canonical_nodes n JOIN _fathomdb_vector_rows v \
             ON v.write_cursor = n.write_cursor WHERE n.kind = ?1",
            [KIND],
            |row| row.get(0),
        )
        .expect("count vectors");
    (nodes, embedded)
}

fn seed(path: &Path) {
    assert!(!path.exists(), "{} exists; seed into a new path", path.display());
    let n = corpus_n();
    let corpus = bodies(CORPUS_SEED, n);
    let engine = open(path);
    let spec = ProjectionSpec {
        name: KIND.into(),
        roles: [ProjectionRole::Searchable].into_iter().collect(),
        fts: None,
        vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
        source: None,
    };
    let delta = engine.configure_projections(&[spec], &[]).expect("configure projections");
    assert!(delta.vector_unsupported_kinds.is_empty(), "{:?}", delta.vector_unsupported_kinds);
    let started = Instant::now();
    for chunk in corpus.chunks(1024) {
        let batch: Vec<PreparedWrite> = chunk
            .iter()
            .map(|body| PreparedWrite::Node {
                kind: KIND.into(),
                body: body.clone(),
                source_id: SourceId::new("test:fixture").expect("source id"),
                logical_id: None,
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect();
        engine.write(&batch).expect("seed write");
    }
    let seed_write = started.elapsed();
    let drain_started = Instant::now();
    engine.drain(drain_timeout_ms()).expect("drain");
    let drain = drain_started.elapsed();
    let readiness = read::embedding_readiness(&engine).expect("embedding readiness");
    engine.close().expect("close");
    let (nodes, embedded) = vector_coverage(path);
    println!(
        "{}",
        json!({
            "command": "seed",
            "n": n,
            "corpus_digest": digest(&corpus),
            "query_digest": digest(&bodies(QUERY_SEED, samples())),
            "seed_write_ms": seed_write.as_secs_f64() * 1000.0,
            "drain_ms": drain.as_secs_f64() * 1000.0,
            "nodes": nodes,
            "vector_rows": embedded,
            "usable_embedder": readiness.usable_embedder,
            "pending": readiness.pending_count,
        })
    );
    assert!(nodes == n as u64 && embedded == nodes, "embedding incomplete: {nodes} {embedded}");
}

/// Reopen a store whose `seed` drain timed out and finish embedding it.
fn drain(path: &Path) {
    let engine = open(path);
    let started = Instant::now();
    engine.drain(drain_timeout_ms()).expect("drain");
    let drained = started.elapsed();
    engine.close().expect("close");
    let (nodes, embedded) = vector_coverage(path);
    println!(
        "{}",
        json!({"command": "drain", "drain_ms": drained.as_secs_f64() * 1000.0,
               "nodes": nodes, "vector_rows": embedded})
    );
    assert!(nodes == corpus_n() as u64 && embedded == nodes, "embedding incomplete");
}

// --- sdk. --------------------------------------------------------------------

fn sdk(path: &Path, out: &Path) {
    std::fs::create_dir_all(out).expect("out dir");
    let queries = bodies(QUERY_SEED, samples());
    let engine = open(path);
    let options = || {
        let mut options = SearchOptions::default();
        options.limit = LIMIT as usize;
        options
    };
    for q in &queries {
        engine.search(q, options()).expect("warmup search");
    }
    let mut search_us = Vec::with_capacity(samples());
    let mut hits = 0usize;
    for q in &queries {
        let started = Instant::now();
        let result = engine.search(q, options()).expect("search");
        search_us.push(started.elapsed().as_micros() as u64);
        hits += result.results.len();
    }
    let mut embed_us = Vec::with_capacity(samples());
    for q in &queries {
        let started = Instant::now();
        engine.embed(q).expect("embed");
        embed_us.push(started.elapsed().as_micros() as u64);
    }
    let dense_disabled = engine.dense_disabled();
    engine.close().expect("close");
    write_samples(out, "sdk_search_us.txt", &search_us);
    write_samples(out, "sdk_embed_us.txt", &embed_us);
    println!(
        "{}",
        json!({
            "command": "sdk",
            "query_digest": digest(&queries),
            "search": summary(&search_us),
            "embed_only": summary(&embed_us),
            "mean_hits": hits as f64 / samples() as f64,
            "dense_disabled": dense_disabled,
        })
    );
}

// --- mcp. --------------------------------------------------------------------

struct Server {
    child: std::process::Child,
    stdin: ChildStdin,
    lines: Receiver<(Instant, String)>,
    stderr: Arc<Mutex<Vec<(Instant, String)>>>,
    next_id: u64,
}

impl Server {
    fn start(binary: &Path, config: &Path) -> Self {
        let mut command = Command::new(binary);
        command.arg("--config").arg(config);
        if std::env::var_os("FATHOMDB_MCP_LOG").is_none() {
            command.env("FATHOMDB_MCP_LOG", "debug");
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn fathomdb-mcp");
        let stdout = child.stdout.take().expect("stdout");
        let stderr_pipe = child.stderr.take().expect("stderr");
        let stdin = child.stdin.take().expect("stdin");
        let (sender, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if sender.send((Instant::now(), line)).is_err() {
                    break;
                }
            }
        });
        let stderr = Arc::new(Mutex::new(Vec::new()));
        let sink = stderr.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr_pipe).lines() {
                let Ok(line) = line else { break };
                sink.lock().unwrap().push((Instant::now(), line));
            }
        });
        Self { child, stdin, lines, stderr, next_id: 1 }
    }

    fn send(&mut self, method: &str, params: Value) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let line = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        writeln!(self.stdin, "{line}").expect("write request");
        self.stdin.flush().expect("flush");
        id
    }

    fn notify(&mut self, method: &str) {
        writeln!(self.stdin, "{}", json!({"jsonrpc": "2.0", "method": method, "params": {}}))
            .expect("write notification");
        self.stdin.flush().expect("flush");
    }

    /// The next response line: its arrival time, id and parsed message.
    fn next_response(&self) -> (Instant, u64, Value) {
        loop {
            let (at, line) = self
                .lines
                .recv_timeout(Duration::from_secs(120))
                .unwrap_or_else(|error| panic!("no response from the server: {error}"));
            let message: Value = serde_json::from_str(&line).expect("JSON-RPC line");
            if let Some(id) = message.get("id").and_then(Value::as_u64) {
                return (at, id, message);
            }
        }
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.send(method, params);
        loop {
            let (_, got, message) = self.next_response();
            if got == id {
                return message;
            }
        }
    }
}

/// What one search call answered.
struct Answer {
    e2e_us: u64,
    error: Option<String>,
    hits: usize,
    vector_hits: usize,
    degraded: bool,
}

fn answer(sent: Instant, at: Instant, message: &Value) -> Answer {
    let e2e_us = at.duration_since(sent).as_micros() as u64;
    let result = &message["result"];
    let structured = &result["structuredContent"];
    if message.get("error").is_some() {
        return Answer {
            e2e_us,
            error: Some(format!("protocol/{}", message["error"]["code"])),
            hits: 0,
            vector_hits: 0,
            degraded: false,
        };
    }
    if result["isError"] == true {
        let error = &structured["error"];
        let mut label = format!(
            "{}/{}",
            error["class"].as_str().unwrap_or("?"),
            error["code"].as_str().unwrap_or("?")
        );
        if let Some(reason) = error["busy"]["reason"].as_str() {
            label.push_str(&format!("/{reason}"));
        }
        return Answer { e2e_us, error: Some(label), hits: 0, vector_hits: 0, degraded: false };
    }
    let results = structured["results"].as_array().cloned().unwrap_or_default();
    let vector_hits =
        results.iter().filter(|hit| hit["arm"].as_str().is_some_and(|arm| arm != "text")).count();
    let degraded = structured["degradation"].as_array().is_some_and(|d| !d.is_empty());
    Answer { e2e_us, error: None, hits: results.len(), vector_hits, degraded }
}

fn search_params(query: &str) -> Value {
    json!({
        "name": "search",
        "arguments": {"query": query, "mode": "hybrid", "limit": LIMIT},
    })
}

/// Keep `concurrency` searches in flight until `total` have completed.
/// Returns the answers in completion order and the phase's wall time.
fn closed_loop(
    server: &mut Server,
    queries: &[String],
    start: usize,
    concurrency: usize,
    total: usize,
) -> (Vec<Answer>, Duration) {
    let mut pending: HashMap<u64, Instant> = HashMap::new();
    let mut answers = Vec::with_capacity(total);
    let mut sent = 0usize;
    let began = Instant::now();
    while sent < concurrency.min(total) {
        let q = &queries[(start + sent) % queries.len()];
        let id = server.send("tools/call", search_params(q));
        pending.insert(id, Instant::now());
        sent += 1;
    }
    while answers.len() < total {
        let (at, id, message) = server.next_response();
        let Some(sent_at) = pending.remove(&id) else { continue };
        answers.push(answer(sent_at, at, &message));
        if sent < total {
            let q = &queries[(start + sent) % queries.len()];
            let id = server.send("tools/call", search_params(q));
            pending.insert(id, Instant::now());
            sent += 1;
        }
    }
    (answers, began.elapsed())
}

fn waited_ms(line: &str) -> Option<u64> {
    if !line.contains("admitted after a wait") {
        return None;
    }
    let rest = &line[line.find("waited_ms=")? + "waited_ms=".len()..];
    rest.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
}

fn phase_report(
    server: &Server,
    name: &str,
    concurrency: usize,
    answers: &[Answer],
    wall: Duration,
    window: (Instant, Instant),
    out: &Path,
) -> Value {
    let ok: Vec<u64> = answers.iter().filter(|a| a.error.is_none()).map(|a| a.e2e_us).collect();
    let mut errors: HashMap<String, usize> = HashMap::new();
    for a in answers {
        if let Some(e) = &a.error {
            *errors.entry(e.clone()).or_default() += 1;
        }
    }
    let waits: Vec<u64> = server
        .stderr
        .lock()
        .unwrap()
        .iter()
        .filter(|(at, _)| *at >= window.0 && *at <= window.1)
        .filter_map(|(_, line)| waited_ms(line))
        .map(|ms| ms * 1000)
        .collect();
    let all: Vec<u64> = answers.iter().map(|a| a.e2e_us).collect();
    write_samples(out, &format!("mcp_{name}_e2e_us.txt"), &all);
    let n = answers.len().max(1) as f64;
    json!({
        "phase": name,
        "concurrency": concurrency,
        "requests": answers.len(),
        "ok": summary(&ok),
        "throughput_per_s": ok.len() as f64 / wall.as_secs_f64(),
        "errors": errors,
        "rejections": answers.iter().filter(|a| a.error.is_some()).count(),
        "queue_wait": {
            "waited_calls": waits.len(),
            "p50_ms": percentile(&waits, 50, 100),
            "p99_ms": percentile(&waits, 99, 100),
            "mean_over_all_calls_ms": waits.iter().sum::<u64>() as f64 / 1000.0 / n,
        },
        "mean_hits": answers.iter().map(|a| a.hits).sum::<usize>() as f64 / n,
        "mean_vector_hits": answers.iter().map(|a| a.vector_hits).sum::<usize>() as f64 / n,
        "degraded_calls": answers.iter().filter(|a| a.degraded).count(),
    })
}

fn mcp(path: &Path, binary: &Path, out: &Path, levels: &[usize], requests: usize) {
    std::fs::create_dir_all(out).expect("out dir");
    let store = std::fs::canonicalize(path).expect("store path");
    let config = out.join("mcp-config.json");
    std::fs::write(
        &config,
        json!({
            "stores": {"ac013": {"path": store, "eager": true}},
            "default_store": "ac013",
            "use_default_embedder": true,
            "search_mode": "hybrid",
        })
        .to_string(),
    )
    .expect("write config");
    let queries = bodies(QUERY_SEED, samples());
    let mut server = Server::start(binary, &config);
    let init = server.request(
        "initialize",
        json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "ac013_mcp", "version": "0"}
        }),
    );
    assert!(init.get("result").is_some(), "initialize failed: {init}");
    server.notify("notifications/initialized");
    // An eager store may still be opening; wait for the first answer.
    let first = server.request("tools/call", search_params(&queries[0]));
    assert_ne!(first["result"]["isError"], true, "first search failed: {first}");

    let mut phases = Vec::new();
    let t0 = Instant::now();
    let (warm, wall) = closed_loop(&mut server, &queries, 0, 1, samples());
    phases.push(phase_report(&server, "c1_warmup", 1, &warm, wall, (t0, Instant::now()), out));
    let t0 = Instant::now();
    let (timed, wall) = closed_loop(&mut server, &queries, 0, 1, samples());
    phases.push(phase_report(&server, "c1", 1, &timed, wall, (t0, Instant::now()), out));
    for &level in levels {
        let t0 = Instant::now();
        let (answers, wall) = closed_loop(&mut server, &queries, 0, level, requests);
        let name = format!("c{level}");
        phases.push(phase_report(&server, &name, level, &answers, wall, (t0, Instant::now()), out));
    }
    drop(server.stdin);
    let _ = server.child.wait();
    println!(
        "{}",
        json!({
            "command": "mcp",
            "query_digest": digest(&queries),
            "mcp_log": std::env::var("FATHOMDB_MCP_LOG").unwrap_or_else(|_| "debug".into()),
            "phases": phases,
        })
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = || -> ! {
        eprintln!(
            "usage: ac013_mcp seed <store> | drain <store> | sdk <store> <out-dir> | \
             mcp <store> <fathomdb-mcp> <out-dir> [levels] [requests]"
        );
        std::process::exit(2);
    };
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["seed", store] => seed(Path::new(store)),
        ["drain", store] => drain(Path::new(store)),
        ["sdk", store, out] => sdk(Path::new(store), Path::new(out)),
        ["mcp", store, binary, out, rest @ ..] => {
            let levels: Vec<usize> = rest
                .first()
                .map(|s| {
                    s.split(',')
                        .filter(|v| !v.is_empty() && *v != "-")
                        .map(|v| v.parse().expect("level"))
                        .collect()
                })
                .unwrap_or_else(|| vec![4, 8, 16, 32]);
            let requests = rest.get(1).map(|s| s.parse().expect("requests")).unwrap_or(samples());
            mcp(Path::new(store), Path::new(binary), Path::new(out), &levels, requests);
        }
        _ => usage(),
    }
}
