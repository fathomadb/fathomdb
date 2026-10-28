//! Hybrid-versus-text retrieval evaluation through the `fathomdb-mcp` server
//! on LoCoMo (MR-021; `dev/experiments/mcp-hybrid-vs-keyword/`).
//!
//! `locomo_eval <locomo10.json> <stores-dir> <fathomdb-mcp> <out-dir>`
//!
//! `<stores-dir>` holds `conv<i>.sqlite` from `locomo_load`. The evaluator
//! writes an MCP config whose one root is `<stores-dir>`, spawns the server
//! over stdio, and for every scored question calls `search` on that
//! question's conversation store with `mode: "text"` and `mode: "hybrid"`
//! (`limit: 10`). It writes `<out-dir>/per_query.jsonl` (refs in rank order,
//! arms, completeness, degradation, per-query metrics) and
//! `<out-dir>/summary.json` (counts, metrics overall and per category, paired
//! hybrid-minus-text bootstrap intervals). Records name a question by
//! `<conversation>:<qa index>` and carry no LoCoMo text, so they can be
//! committed while the dataset stays out of git. The server inherits the
//! environment, so `FATHOMDB_EMBED_DEVICE` selects the query-embedding device.

mod dataset;
mod metrics;

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Instant;

use serde_json::{json, Value};

use crate::dataset::normalize_evidence;
use crate::metrics::{
    hit_at_k, mean, paired_bootstrap_ci, precision_at_k, recall_at_k, reciprocal_rank_at_k,
};

const LIMIT: u64 = 10;
const RESAMPLES: usize = 10_000;
const SEED: u64 = 20_261_017;
const MODES: [&str; 2] = ["text", "hybrid"];
const METRICS: [&str; 5] = ["precision@3", "recall@5", "recall@10", "mrr@10", "hit@1"];
/// LoCoMo's category numbers, as most published evaluations label them.
const CATEGORY_NAMES: [(u64, &str); 5] =
    [(1, "multi-hop"), (2, "temporal"), (3, "open-domain"), (4, "single-hop"), (5, "adversarial")];

struct Query {
    id: String,
    conversation: usize,
    category: u64,
    question: String,
    gold: BTreeSet<String>,
    dropped_evidence: Vec<String>,
}

#[derive(Default)]
struct Counts {
    qa_total: usize,
    excluded_category_5: usize,
    excluded_empty_evidence: usize,
    excluded_unmappable_evidence: usize,
    dropped_evidence_ids: Vec<String>,
}

fn dialog_ids(conversation: &Value) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for (key, value) in conversation.as_object().expect("conversation object") {
        let is_session = key
            .strip_prefix("session_")
            .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()));
        if is_session {
            for turn in value.as_array().expect("session array") {
                ids.insert(turn["dia_id"].as_str().expect("dia_id").to_string());
            }
        }
    }
    ids
}

fn queries(data: &Value) -> (Vec<Query>, Counts) {
    let mut counts = Counts::default();
    let mut out = Vec::new();
    for (conversation, sample) in data.as_array().expect("samples").iter().enumerate() {
        let ids = dialog_ids(&sample["conversation"]);
        for (index, qa) in sample["qa"].as_array().expect("qa").iter().enumerate() {
            counts.qa_total += 1;
            let category = qa["category"].as_u64().expect("category");
            let raw: Vec<&str> = qa["evidence"]
                .as_array()
                .map(|items| items.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            if category == 5 {
                counts.excluded_category_5 += 1;
                continue;
            }
            if raw.is_empty() {
                counts.excluded_empty_evidence += 1;
                continue;
            }
            let mut gold = BTreeSet::new();
            let mut dropped = Vec::new();
            for entry in raw {
                let normalized = normalize_evidence(entry);
                if normalized.is_empty() {
                    dropped.push(entry.to_string());
                }
                for id in normalized {
                    if ids.contains(&id) {
                        gold.insert(format!("l:{conversation}:{id}"));
                    } else {
                        dropped.push(id);
                    }
                }
            }
            for id in &dropped {
                counts.dropped_evidence_ids.push(format!("{conversation}:{id}"));
            }
            if gold.is_empty() {
                counts.excluded_unmappable_evidence += 1;
                continue;
            }
            out.push(Query {
                id: format!("{conversation}:{index}"),
                conversation,
                category,
                question: qa["question"].as_str().expect("question").to_string(),
                gold,
                dropped_evidence: dropped,
            });
        }
    }
    (out, counts)
}

struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl Server {
    fn start(binary: &Path, config: &Path, stderr: File) -> Self {
        let mut child = Command::new(binary)
            .arg("--config")
            .arg(config)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr)
            .spawn()
            .expect("spawn fathomdb-mcp");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        let mut server = Self { child, stdin, stdout, next_id: 1 };
        server.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "locomo-eval", "version": "0"}
            }),
        );
        server
            .send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}));
        server
    }

    fn send(&mut self, message: &Value) {
        writeln!(self.stdin, "{message}").expect("write request");
        self.stdin.flush().expect("flush request");
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let mut line = String::new();
            let read = self.stdout.read_line(&mut line).expect("read response");
            assert!(read > 0, "fathomdb-mcp closed stdout during {method}");
            let message: Value = serde_json::from_str(&line).expect("stdout line is JSON");
            if message["id"] == json!(id) {
                assert!(message.get("error").is_none(), "{method} failed: {message}");
                return message["result"].clone();
            }
        }
    }

    fn tool(&mut self, name: &str, arguments: Value) -> (bool, Value) {
        let result = self.request("tools/call", json!({"name": name, "arguments": arguments}));
        (result["isError"] == json!(true), result["structuredContent"].clone())
    }

    fn finish(mut self) {
        drop(self.stdin);
        let status = self.child.wait().expect("wait for fathomdb-mcp");
        assert!(status.success(), "fathomdb-mcp exited with {status}");
    }
}

struct Run {
    refs: Vec<String>,
    arms: Vec<String>,
    completeness: String,
    degradation: Value,
    error: Option<Value>,
    millis: f64,
}

fn search(server: &mut Server, store: &str, query: &str, mode: &str) -> Run {
    let started = Instant::now();
    let (is_error, content) = server
        .tool("search", json!({"store": store, "query": query, "mode": mode, "limit": LIMIT}));
    let millis = started.elapsed().as_secs_f64() * 1000.0;
    if is_error {
        return Run {
            refs: Vec::new(),
            arms: Vec::new(),
            completeness: "error".into(),
            degradation: json!([]),
            error: Some(content),
            millis,
        };
    }
    let results = content["results"].as_array().cloned().unwrap_or_default();
    Run {
        refs: results.iter().map(|hit| hit["ref"].as_str().unwrap_or("").to_string()).collect(),
        arms: results.iter().map(|hit| hit["arm"].as_str().unwrap_or("").to_string()).collect(),
        completeness: content["completeness"].as_str().unwrap_or("").to_string(),
        degradation: content["degradation"].clone(),
        error: None,
        millis,
    }
}

fn scores(refs: &[String], gold: &BTreeSet<String>) -> [f64; 5] {
    [
        precision_at_k(refs, gold, 3),
        recall_at_k(refs, gold, 5),
        recall_at_k(refs, gold, 10),
        reciprocal_rank_at_k(refs, gold, 10),
        hit_at_k(refs, gold, 1),
    ]
}

/// Per-mode means and paired hybrid-minus-text intervals over `rows`, each
/// `(text scores, hybrid scores)`.
fn table(rows: &[([f64; 5], [f64; 5])]) -> Value {
    let mut out = serde_json::Map::new();
    out.insert("queries".into(), json!(rows.len()));
    for (index, metric) in METRICS.iter().enumerate() {
        let text: Vec<f64> = rows.iter().map(|(text, _)| text[index]).collect();
        let hybrid: Vec<f64> = rows.iter().map(|(_, hybrid)| hybrid[index]).collect();
        let diffs: Vec<f64> =
            rows.iter().map(|(text, hybrid)| hybrid[index] - text[index]).collect();
        let ci = paired_bootstrap_ci(&diffs, RESAMPLES, SEED);
        out.insert(
            (*metric).into(),
            json!({
                "text": mean(&text),
                "hybrid": mean(&hybrid),
                "hybrid_minus_text": ci.mean,
                "ci95": [ci.low, ci.high],
                "hybrid_better": diffs.iter().filter(|d| **d > 0.0).count(),
                "text_better": diffs.iter().filter(|d| **d < 0.0).count(),
                "tied": diffs.iter().filter(|d| **d == 0.0).count(),
            }),
        );
    }
    Value::Object(out)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [input, stores_dir, binary, out_dir] = args.as_slice() else {
        eprintln!("usage: locomo_eval <locomo10.json> <stores-dir> <fathomdb-mcp> <out-dir>");
        std::process::exit(2);
    };
    let stores_dir = std::fs::canonicalize(stores_dir).expect("stores-dir");
    let out_dir = PathBuf::from(out_dir);
    std::fs::create_dir_all(&out_dir).expect("create out-dir");
    let data: Value =
        serde_json::from_str(&std::fs::read_to_string(input).expect("read LoCoMo")).expect("JSON");
    let (queries, counts) = queries(&data);

    let config = json!({
        "roots": [{"path": stores_dir, "suffix": ".sqlite"}],
        "use_default_embedder": true,
        "default_max_tokens": 200_000,
        "call_deadline_ms": 120_000,
        "idle_timeout_secs": 0,
        "snippet_bytes": 40
    });
    let config_path = out_dir.join("mcp-config.json");
    std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();
    let stderr = File::create(out_dir.join("server-stderr.log")).expect("stderr log");
    let started = Instant::now();
    let mut server = Server::start(Path::new(binary), &config_path, stderr);

    let mut orient = BTreeMap::new();
    let mut by_store: BTreeMap<usize, Vec<&Query>> = BTreeMap::new();
    for query in &queries {
        by_store.entry(query.conversation).or_default().push(query);
    }
    let mut writer =
        BufWriter::new(File::create(out_dir.join("per_query.jsonl")).expect("per_query.jsonl"));
    let mut rows: Vec<(u64, [f64; 5], [f64; 5])> = Vec::new();
    let mut tallies: BTreeMap<&str, BTreeMap<String, u64>> = BTreeMap::new();
    let mut millis: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
    for (conversation, store_queries) in &by_store {
        let store = format!("conv{conversation}");
        let (is_error, content) = server.tool("orient", json!({"store": store}));
        assert!(!is_error, "orient {store}: {content}");
        orient.insert(store.clone(), content["embedding_readiness"].clone());
        for query in store_queries {
            let mut record = json!({
                "id": query.id,
                "store": store,
                "category": query.category,
                "gold": query.gold,
                "dropped_evidence": query.dropped_evidence,
            });
            let mut per_mode = Vec::new();
            for mode in MODES {
                let run = search(&mut server, &store, &query.question, mode);
                let score = scores(&run.refs, &query.gold);
                let tally = tallies.entry(mode).or_default();
                *tally.entry(format!("completeness:{}", run.completeness)).or_default() += 1;
                if run.refs.is_empty() {
                    *tally.entry("zero_results".into()).or_default() += 1;
                }
                if run.error.is_some() {
                    *tally.entry("errors".into()).or_default() += 1;
                }
                for entry in run.degradation.as_array().into_iter().flatten() {
                    let key = format!("degradation:{}:{}", entry["arm"], entry["reason"]);
                    *tally.entry(key.replace('"', "")).or_default() += 1;
                }
                for arm in &run.arms {
                    *tally.entry(format!("hit_arm:{arm}")).or_default() += 1;
                }
                millis.entry(mode).or_default().push(run.millis);
                record[mode] = json!({
                    "refs": run.refs,
                    "arms": run.arms,
                    "completeness": run.completeness,
                    "degradation": run.degradation,
                    "error": run.error,
                    "metrics": METRICS.iter().zip(score).map(|(m, s)| (m.to_string(), json!(s)))
                        .collect::<serde_json::Map<_, _>>(),
                });
                per_mode.push(score);
            }
            writeln!(writer, "{record}").expect("write per_query");
            rows.push((query.category, per_mode[0], per_mode[1]));
        }
    }
    writer.flush().expect("flush per_query");
    server.finish();

    let overall: Vec<([f64; 5], [f64; 5])> = rows.iter().map(|(_, t, h)| (*t, *h)).collect();
    let mut per_category = serde_json::Map::new();
    for (number, name) in CATEGORY_NAMES {
        let subset: Vec<([f64; 5], [f64; 5])> =
            rows.iter().filter(|(c, _, _)| *c == number).map(|(_, t, h)| (*t, *h)).collect();
        if !subset.is_empty() {
            per_category.insert(format!("{number} {name}"), table(&subset));
        }
    }
    let latency: serde_json::Map<String, Value> = millis
        .iter()
        .map(|(mode, values)| {
            let mut sorted = values.clone();
            sorted.sort_by(f64::total_cmp);
            (
                mode.to_string(),
                json!({
                    "mean_ms": mean(values),
                    "p50_ms": metrics::percentile(&sorted, 0.5),
                    "p95_ms": metrics::percentile(&sorted, 0.95),
                }),
            )
        })
        .collect();
    let summary = json!({
        "dataset": "LoCoMo (locomo10.json)",
        "limit": LIMIT,
        "bootstrap": {"resamples": RESAMPLES, "seed": SEED, "interval": "percentile 95%, paired over queries"},
        "counts": {
            "qa_total": counts.qa_total,
            "excluded_category_5_adversarial": counts.excluded_category_5,
            "excluded_empty_evidence": counts.excluded_empty_evidence,
            "excluded_unmappable_evidence": counts.excluded_unmappable_evidence,
            "scored": rows.len(),
            "dropped_evidence_ids": counts.dropped_evidence_ids,
        },
        "categories": CATEGORY_NAMES.iter().map(|(n, name)| (n.to_string(), json!(name)))
            .collect::<serde_json::Map<_, _>>(),
        "overall": table(&overall),
        "per_category": per_category,
        "tallies": tallies,
        "latency": latency,
        "orient_embedding_readiness": orient,
        "elapsed_seconds": started.elapsed().as_secs_f64(),
    });
    let text = serde_json::to_string_pretty(&summary).unwrap();
    std::fs::write(out_dir.join("summary.json"), &text).expect("write summary");
    println!("{text}");
}
