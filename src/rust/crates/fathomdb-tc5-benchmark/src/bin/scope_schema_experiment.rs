//! WP-10D Empirical Experiment: Schema 38 vs. Schema 39 `scope` Performance + `SyncMode` Write Latency.
//!
//! Compares three database layouts on identical seeded 100k-node / 150k-edge corpora:
//! - `B0`: Exact 0.8.31 Schema 38 DDL & 7-col `vec0` (`scope == ""`)
//! - `S38-PARTIAL`: Non-breaking Schema 38 + partial indexes `WHERE scope != ''`,
//!   measuring both (A1) lazy 7-col `vec0` pre-selection and (A2) reshaped 8-col `vec0` (`scope TEXT`)
//! - `S39-COMPOSITE`: Breaking Schema 39 composite PK/unique/prefix indexes + native 8-col `vec0`
//!
//! Also measures `SyncMode::Normal` vs. `SyncMode::Full` write/upsert/checkpoint latency from
//! identical restored WAL-truncated snapshots, subprocess `process_open`/`warm_reopen`/`peak_rss_bytes`,
//! `recall@10` against exact L2 ground truth, `database_bytes`, and `EXPLAIN QUERY PLAN`.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::Once;
use std::time::Instant;

use fathomdb_engine::{
    Engine, InitialState, PreparedWrite, ProjectionRole, ProjectionSpec, PropertyEntityClass,
    PropertyMap, PropertyValue, SourceId,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const DEFAULT_SEED: u64 = 20261124;
const DEFAULT_NODES: usize = 100_000;
const DEFAULT_OP_STATE_ROWS: usize = 10_000;
const DEFAULT_OP_MUTATION_ROWS: usize = 10_000;
const EMBED_DIM: usize = 384;
const NUM_TOPICS: usize = 20;
const NUM_CALLERS: usize = 10;
const TOP_K_BIT_CANDIDATES: usize = 192;
const TOP_K_RESULTS: usize = 10;
const PAGE_LIMIT: usize = 25;
const BATCH_SIZE: usize = 200;
const WARMUP_ROUNDS: usize = 5;
const OPEN_TRIALS: usize = 30;
const WRITE_TRIALS: usize = 100;
const READ_TRIALS: usize = 200;
const BOOTSTRAP_RESAMPLES: usize = 10_000;
const READER_POOL_SIZE: usize = 4;
const READER_LOOKASIDE_SLOT_SIZE: std::os::raw::c_int = 1200;
const READER_LOOKASIDE_SLOT_COUNT: std::os::raw::c_int = 500;
const TOPICS: [&str; 5] = ["alpha", "beta", "gamma", "delta", "epsilon"];
const TIERS: [&str; 3] = ["gold", "silver", "bronze"];
const REGIONS: [&str; 4] = ["us-east", "us-west", "eu-central", "ap-south"];
const ENVS: [&str; 3] = ["prod", "staging", "dev"];
const TEAMS: [&str; 4] = ["core", "infra", "ml", "platform"];
const OP_COLLECTIONS: [&str; 5] =
    ["agent_checkpoints", "session_state", "workflow_cursors", "task_checkpoints", "policy_state"];
const OP_LOG_COLLECTION: &str = "agent_audit";
const NODE_PROP_NAMES: [&str; 8] =
    ["tier", "topic", "region", "env", "bucket", "priority_label", "owner_team", "active_flag"];

// ---------------------------------------------------------------------------
// Data models & configuration
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ProfileKind {
    SingleScope,
    MultiScopeUniform,
    MultiScopeSkewed,
}

impl ProfileKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::SingleScope => "single_scope",
            Self::MultiScopeUniform => "multi_scope_uniform",
            Self::MultiScopeSkewed => "multi_scope_skewed",
        }
    }

    fn callers(self) -> Vec<String> {
        match self {
            Self::SingleScope => vec![String::new()],
            Self::MultiScopeUniform => (0..NUM_CALLERS).map(|k| format!("tenant_{k}")).collect(),
            Self::MultiScopeSkewed => {
                let mut v = vec!["heavy_0".to_string()];
                for k in 1..NUM_CALLERS {
                    v.push(format!("light_{k}"));
                }
                v
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LayoutId {
    B0,
    S38PartialA1,
    S38PartialA2,
    S39Composite,
}

impl LayoutId {
    fn as_str(self) -> &'static str {
        match self {
            Self::B0 => "B0",
            Self::S38PartialA1 => "S38-PARTIAL-A1",
            Self::S38PartialA2 => "S38-PARTIAL-A2",
            Self::S39Composite => "S39-COMPOSITE",
        }
    }

    fn cli_flag(self) -> &'static str {
        match self {
            Self::B0 => "b0",
            Self::S38PartialA1 => "s38_a1",
            Self::S38PartialA2 => "s38_a2",
            Self::S39Composite => "s39_b",
        }
    }

    fn parse_flag(s: &str) -> Result<Self, String> {
        match s {
            "b0" => Ok(Self::B0),
            "s38_a1" => Ok(Self::S38PartialA1),
            "s38_a2" => Ok(Self::S38PartialA2),
            "s39_b" => Ok(Self::S39Composite),
            other => Err(format!("unknown layout flag: {other}")),
        }
    }

    fn has_vec0_scope_col(self) -> bool {
        matches!(self, Self::S38PartialA2 | Self::S39Composite)
    }

    fn is_s39(self) -> bool {
        matches!(self, Self::S39Composite)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SyncMode {
    Normal,
    Full,
}

impl SyncMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Full => "FULL",
        }
    }

    fn cli_flag(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Full => "full",
        }
    }

    fn parse_flag(s: &str) -> Result<Self, String> {
        match s {
            "normal" => Ok(Self::Normal),
            "full" => Ok(Self::Full),
            other => Err(format!("unknown sync mode: {other}")),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct LatencySummary {
    n: usize,
    p50_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
    min_ms: f64,
    max_ms: f64,
    mean_ms: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct BootstrapCi {
    pct: f64,
    n: usize,
    resamples: usize,
    seed: u64,
    baseline_ms: f64,
    candidate_ms: f64,
    observed_delta_ms: f64,
    ci_95_lower_ms: f64,
    ci_95_upper_ms: f64,
}

#[derive(Clone, Debug, Serialize)]
struct PairedComparison {
    baseline_label: String,
    candidate_label: String,
    p50: BootstrapCi,
    p95: BootstrapCi,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct ExplainPlanAudit {
    query_name: String,
    details: Vec<String>,
    real_table_scans: Vec<String>,
    uses_temp_btree_order_by: bool,
    clean: bool,
}

#[derive(Clone, Debug, Serialize)]
struct OpenProbeResult {
    layout: String,
    sync_mode: String,
    probe_mode: String,
    real_engine: bool,
    process_open: LatencySummary,
    warm_reopen: LatencySummary,
    peak_rss_bytes_max: u64,
    peak_rss_bytes_p50: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ChildProbeOutput {
    samples_ms: Vec<f64>,
    peak_rss_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
struct VectorRecallSummary {
    queries: usize,
    mean_recall_at_10: f64,
    min_recall_at_10: f64,
    mean_returned_count: f64,
    min_returned_count: usize,
    empty_result_count: usize,
}

#[derive(Clone, Debug, Serialize)]
struct DatabaseSizeReport {
    layout: String,
    file_bytes: u64,
    page_size: u64,
    page_count: u64,
    freelist_count: u64,
    live_btree_bytes: u64,
    delta_bytes_vs_b0: Option<i64>,
    relative_delta_vs_b0: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
struct WriteSuiteResult {
    layout: String,
    sync_mode: String,
    write_node_batch_200_fresh: LatencySummary,
    write_node_batch_200_upsert: LatencySummary,
    write_edge_batch_200: LatencySummary,
    single_row_checkpoint_write_fresh: LatencySummary,
    single_row_checkpoint_write_upsert: LatencySummary,
    ownership_guard_verified: bool,
}

#[derive(Clone, Debug, Serialize)]
struct ReadSuiteResult {
    layout: String,
    cohort: String,
    search_text_only: LatencySummary,
    search_vector_k10: LatencySummary,
    search_vector_recall: VectorRecallSummary,
    search_hybrid: LatencySummary,
    graph_expand_2hop: LatencySummary,
    operational_state_point_read: LatencySummary,
    operational_state_page: LatencySummary,
}

#[derive(Clone, Debug, Serialize)]
struct ProfileReport {
    profile: String,
    num_nodes: usize,
    num_edges: usize,
    num_op_state: usize,
    num_op_mutations: usize,
    scope_distribution: BTreeMap<String, usize>,
    database_sizes: Vec<DatabaseSizeReport>,
    open_probes: Vec<OpenProbeResult>,
    open_comparisons: Vec<PairedComparison>,
    write_suites: Vec<WriteSuiteResult>,
    write_comparisons: Vec<PairedComparison>,
    read_suites: Vec<ReadSuiteResult>,
    read_comparisons: Vec<PairedComparison>,
    explain_plans: Vec<ExplainPlanAudit>,
}

#[derive(Clone, Debug, Serialize)]
struct ExperimentEnvironment {
    cpu_model: String,
    cpu_count: usize,
    sqlite_version: String,
    sqlite_vec_version: String,
    schema_version_baseline: u32,
    rust_profile: &'static str,
    seed: u64,
    warmup_rounds: usize,
    open_trials: usize,
    write_trials: usize,
    read_trials: usize,
    bootstrap_resamples: usize,
}

#[derive(Clone, Debug, Serialize)]
struct ExperimentReport {
    schema: &'static str,
    generated_at_unix_s: u64,
    environment: ExperimentEnvironment,
    vec0_default_clause_supported: bool,
    vacuum_supported_on_vec0: bool,
    profiles: Vec<ProfileReport>,
}

// ---------------------------------------------------------------------------
// Deterministic PRNG & math utilities
// ---------------------------------------------------------------------------

#[derive(Clone)]
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

    fn next_f32_signed(&mut self) -> f32 {
        // Uniform in [-1.0, 1.0)
        let bits = (self.next_u64() >> 40) as u32;
        (bits as f32 / (1_u32 << 23) as f32) - 1.0
    }

    fn gen_range_usize(&mut self, bound: usize) -> usize {
        if bound <= 1 {
            return 0;
        }
        (self.next_u64() % (bound as u64)) as usize
    }
}

fn round4(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}

fn nearest_rank_percentile(values: &[f64], pct: f64) -> f64 {
    assert!(!values.is_empty(), "percentile requires non-empty values");
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let idx = (((pct / 100.0) * (ordered.len() - 1) as f64).round() as usize)
        .min(ordered.len().saturating_sub(1));
    ordered[idx]
}

fn summarize_samples(samples_ms: &[f64]) -> LatencySummary {
    assert!(!samples_ms.is_empty(), "summarize_samples requires non-empty samples");
    let sum: f64 = samples_ms.iter().sum();
    let min_v = samples_ms.iter().copied().fold(f64::INFINITY, f64::min);
    let max_v = samples_ms.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    LatencySummary {
        n: samples_ms.len(),
        p50_ms: round4(nearest_rank_percentile(samples_ms, 50.0)),
        p95_ms: round4(nearest_rank_percentile(samples_ms, 95.0)),
        p99_ms: round4(nearest_rank_percentile(samples_ms, 99.0)),
        min_ms: round4(min_v),
        max_ms: round4(max_v),
        mean_ms: round4(sum / samples_ms.len() as f64),
    }
}

fn paired_bootstrap_ci(
    baseline: &[f64],
    candidate: &[f64],
    pct: f64,
    resamples: usize,
    seed: u64,
) -> BootstrapCi {
    assert_eq!(baseline.len(), candidate.len(), "paired bootstrap lengths must match");
    let n = baseline.len();
    assert!(n > 0 && resamples > 0);
    let k_idx = (((pct / 100.0) * (n - 1) as f64).round() as usize).min(n - 1);

    let mut base_sorted = baseline.to_vec();
    base_sorted.sort_by(f64::total_cmp);
    let mut cand_sorted = candidate.to_vec();
    cand_sorted.sort_by(f64::total_cmp);
    let base_stat = base_sorted[k_idx];
    let cand_stat = cand_sorted[k_idx];
    let observed_delta = cand_stat - base_stat;

    let mut rng = SplitMix64::new(seed);
    let mut deltas = Vec::with_capacity(resamples);
    let mut bx = Vec::with_capacity(n);
    let mut cy = Vec::with_capacity(n);
    for _ in 0..resamples {
        bx.clear();
        cy.clear();
        for _ in 0..n {
            let i = rng.gen_range_usize(n);
            bx.push(baseline[i]);
            cy.push(candidate[i]);
        }
        bx.sort_by(f64::total_cmp);
        cy.sort_by(f64::total_cmp);
        deltas.push(cy[k_idx] - bx[k_idx]);
    }
    deltas.sort_by(f64::total_cmp);
    let lo_idx = ((0.025 * resamples as f64).round() as usize).saturating_sub(1);
    let hi_idx =
        (((0.975 * resamples as f64).round() as usize).saturating_sub(1)).min(resamples - 1);

    BootstrapCi {
        pct,
        n,
        resamples,
        seed,
        baseline_ms: round4(base_stat),
        candidate_ms: round4(cand_stat),
        observed_delta_ms: round4(observed_delta),
        ci_95_lower_ms: round4(deltas[lo_idx]),
        ci_95_upper_ms: round4(deltas[hi_idx]),
    }
}

fn compare_paired(
    baseline_label: &str,
    candidate_label: &str,
    baseline: &[f64],
    candidate: &[f64],
    seed: u64,
) -> PairedComparison {
    PairedComparison {
        baseline_label: baseline_label.to_string(),
        candidate_label: candidate_label.to_string(),
        p50: paired_bootstrap_ci(baseline, candidate, 50.0, BOOTSTRAP_RESAMPLES, seed),
        p95: paired_bootstrap_ci(baseline, candidate, 95.0, BOOTSTRAP_RESAMPLES, seed ^ 0x5555),
    }
}

fn compute_recall_at_k(predicted: &[i64], truth: &[i64], k: usize) -> f64 {
    let denom = truth.len().min(k);
    if denom == 0 {
        return 1.0;
    }
    let truth_set: HashSet<i64> = truth.iter().take(denom).copied().collect();
    let hits =
        predicted.iter().take(k).filter(|id| truth_set.contains(id)).collect::<HashSet<_>>().len();
    hits as f64 / denom as f64
}

fn audit_explain_plan_named(query_name: &str, details: Vec<String>) -> ExplainPlanAudit {
    let has_candidates_cte = details
        .iter()
        .any(|line| line == "CO-ROUTINE candidates" || line == "MATERIALIZE candidates");
    let real_table_scans: Vec<String> = details
        .iter()
        .filter(|line| {
            line.starts_with("SCAN ")
                && !line.contains("VIRTUAL TABLE INDEX")
                && !(has_candidates_cte && (*line == "SCAN c" || *line == "SCAN candidates"))
        })
        .cloned()
        .collect();
    let uses_temp_btree_order_by =
        details.iter().any(|line| line.contains("USE TEMP B-TREE FOR ORDER BY"));
    let clean = real_table_scans.is_empty() && !uses_temp_btree_order_by;
    ExplainPlanAudit {
        query_name: query_name.to_string(),
        details,
        real_table_scans,
        uses_temp_btree_order_by,
        clean,
    }
}

#[cfg(test)]
fn audit_explain_plan(details: Vec<String>) -> ExplainPlanAudit {
    audit_explain_plan_named("test", details)
}

fn explain_sql(
    conn: &Connection,
    query_name: &str,
    sql: &str,
    params: &[rusqlite::types::Value],
) -> rusqlite::Result<ExplainPlanAudit> {
    let mut stmt = conn.prepare(&format!("EXPLAIN QUERY PLAN {sql}"))?;
    let rows =
        stmt.query_map(rusqlite::params_from_iter(params.iter()), |r| r.get::<_, String>(3))?;
    let details = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(audit_explain_plan_named(query_name, details))
}

// ---------------------------------------------------------------------------
// Deterministic 384-dim clustered embeddings & corpus layout
// ---------------------------------------------------------------------------

fn l2_normalize(vec: &mut [f32]) {
    let norm_sq: f64 = vec.iter().map(|&v| (v as f64) * (v as f64)).sum();
    let norm = norm_sq.sqrt().max(1e-12);
    for v in vec.iter_mut() {
        *v = ((*v as f64) / norm) as f32;
    }
}

struct EmbeddingGenerator {
    centroids: Vec<Vec<f32>>,
    subcentroids: Vec<Vec<f32>>,
}

impl EmbeddingGenerator {
    fn new(seed: u64) -> Self {
        let mut rng = SplitMix64::new(seed ^ 0xA5A5_5A5A);
        let mut centroids = Vec::with_capacity(NUM_TOPICS);
        for _ in 0..NUM_TOPICS {
            let mut c = (0..EMBED_DIM).map(|_| rng.next_f32_signed()).collect::<Vec<_>>();
            l2_normalize(&mut c);
            centroids.push(c);
        }
        let mut subcentroids = Vec::with_capacity(64);
        for _ in 0..64 {
            let mut s = (0..EMBED_DIM).map(|_| rng.next_f32_signed()).collect::<Vec<_>>();
            l2_normalize(&mut s);
            subcentroids.push(s);
        }
        Self { centroids, subcentroids }
    }

    fn node_vector(&self, node_idx: usize) -> Vec<f32> {
        let c = &self.centroids[node_idx % NUM_TOPICS];
        let s = &self.subcentroids[(node_idx / NUM_TOPICS) % self.subcentroids.len()];
        let mut rng = SplitMix64::new(DEFAULT_SEED.wrapping_add(node_idx as u64 * 17 + 1));
        let mut out = Vec::with_capacity(EMBED_DIM);
        for d in 0..EMBED_DIM {
            let noise = rng.next_f32_signed();
            out.push(c[d] + 0.35 * s[d] + 0.18 * noise);
        }
        l2_normalize(&mut out);
        out
    }

    fn query_vector(&self, query_idx: usize) -> Vec<f32> {
        let c = &self.centroids[query_idx % NUM_TOPICS];
        let s = &self.subcentroids[(query_idx / NUM_TOPICS) % self.subcentroids.len()];
        let mut rng =
            SplitMix64::new(DEFAULT_SEED.wrapping_add(10_000_000 + query_idx as u64 * 31));
        let mut out = Vec::with_capacity(EMBED_DIM);
        for d in 0..EMBED_DIM {
            let noise = rng.next_f32_signed();
            out.push(c[d] + 0.30 * s[d] + 0.12 * noise);
        }
        l2_normalize(&mut out);
        out
    }
}

fn encode_f32_blob(vector: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(vector.len() * 4);
    for v in vector {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn scope_for_node(profile: ProfileKind, idx: usize) -> String {
    match profile {
        ProfileKind::SingleScope => String::new(),
        ProfileKind::MultiScopeUniform => format!("tenant_{}", idx / 10_000),
        ProfileKind::MultiScopeSkewed => {
            if idx < 90_000 {
                "heavy_0".to_string()
            } else {
                let rem = idx - 90_000;
                // 10,000 nodes split across light_1..light_9:
                // light_1 gets 1,112; light_2..light_9 get 1,111 each.
                let caller = if rem < 1_112 { 1 } else { 2 + (rem - 1_112) / 1_111 };
                format!("light_{caller}")
            }
        }
    }
}

/// Range `[start, end)` of node indices owned by the same scope as `idx` (under `num_nodes == 100_000`
/// or scaled proportionally for unit tests).
fn scope_node_bounds(profile: ProfileKind, idx: usize, num_nodes: usize) -> (usize, usize) {
    if num_nodes < 100_000 {
        return match profile {
            ProfileKind::SingleScope => (0, num_nodes),
            ProfileKind::MultiScopeUniform => {
                let block = (num_nodes / NUM_CALLERS).max(1);
                let bucket = (idx / block).min(NUM_CALLERS - 1);
                let start = bucket * block;
                let end = if bucket + 1 == NUM_CALLERS { num_nodes } else { (bucket + 1) * block };
                (start, end)
            }
            ProfileKind::MultiScopeSkewed => {
                let heavy = (num_nodes * 9) / 10;
                if idx < heavy {
                    (0, heavy)
                } else {
                    let rem_total = num_nodes - heavy;
                    let light_callers = NUM_CALLERS - 1;
                    let per = (rem_total / light_callers).max(1);
                    let offset = idx - heavy;
                    let bucket = (offset / per).min(light_callers - 1);
                    let start = heavy + bucket * per;
                    let end = if bucket + 1 == light_callers {
                        num_nodes
                    } else {
                        heavy + (bucket + 1) * per
                    };
                    (start, end)
                }
            }
        };
    }
    match profile {
        ProfileKind::SingleScope => (0, 100_000),
        ProfileKind::MultiScopeUniform => {
            let bucket = idx / 10_000;
            (bucket * 10_000, (bucket + 1) * 10_000)
        }
        ProfileKind::MultiScopeSkewed => {
            if idx < 90_000 {
                (0, 90_000)
            } else {
                let rem = idx - 90_000;
                if rem < 1_112 {
                    (90_000, 91_112)
                } else {
                    let b = (rem - 1_112) / 1_111;
                    let s = 91_112 + b * 1_111;
                    (s, (s + 1_111).min(100_000))
                }
            }
        }
    }
}

fn scope_for_node_scaled(profile: ProfileKind, idx: usize, num_nodes: usize) -> String {
    if num_nodes == 100_000 {
        return scope_for_node(profile, idx);
    }
    match profile {
        ProfileKind::SingleScope => String::new(),
        ProfileKind::MultiScopeUniform => {
            let block = (num_nodes / NUM_CALLERS).max(1);
            let bucket = (idx / block).min(NUM_CALLERS - 1);
            format!("tenant_{bucket}")
        }
        ProfileKind::MultiScopeSkewed => {
            let heavy = (num_nodes * 9) / 10;
            if idx < heavy {
                "heavy_0".to_string()
            } else {
                let rem = idx - heavy;
                let light_callers = NUM_CALLERS - 1;
                let per = ((num_nodes - heavy) / light_callers).max(1);
                let bucket = (rem / per).min(light_callers - 1) + 1;
                format!("light_{bucket}")
            }
        }
    }
}

fn node_properties(i: usize) -> PropertyMap {
    let mut m = PropertyMap::new();
    m.insert("tier".to_string(), PropertyValue::String(TIERS[i % TIERS.len()].to_string()));
    m.insert("topic".to_string(), PropertyValue::String(TOPICS[i % TOPICS.len()].to_string()));
    m.insert("region".to_string(), PropertyValue::String(REGIONS[i % REGIONS.len()].to_string()));
    m.insert("env".to_string(), PropertyValue::String(ENVS[i % ENVS.len()].to_string()));
    m.insert("bucket".to_string(), PropertyValue::Int((i % 500) as i64));
    m.insert("priority_label".to_string(), PropertyValue::String(format!("p{}", i % 4)));
    m.insert("owner_team".to_string(), PropertyValue::String(TEAMS[i % TEAMS.len()].to_string()));
    m.insert("active_flag".to_string(), PropertyValue::Bool(!i.is_multiple_of(10)));
    m
}

fn attr_vec0_column(name: &str) -> String {
    let mut s = String::from("attr_");
    for b in name.as_bytes() {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn encode_attr_vec0_text(val: &PropertyValue) -> String {
    match val {
        PropertyValue::String(s) => format!("\u{1}{s}"),
        PropertyValue::Bool(b) => format!("\u{1}{}", if *b { "true" } else { "false" }),
        PropertyValue::Int(_) | PropertyValue::Float(_) => String::new(),
    }
}

// ---------------------------------------------------------------------------
// Process / SQLite runtime helpers
// ---------------------------------------------------------------------------

fn register_sqlite_vec() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let dir = tempfile::tempdir().expect("create temp dir to register sqlite-vec");
        let opened = Engine::open(dir.path().join("init_sqlite_vec.sqlite"))
            .expect("Engine::open registers sqlite-vec extension");
        opened.engine.close().expect("close init engine");
    });
}

fn read_vm_hwm_bytes() -> u64 {
    let Ok(status) = fs::read_to_string("/proc/self/status") else {
        return 0;
    };
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            let kb_str = rest.trim().trim_end_matches("kB").trim();
            if let Ok(kb) = kb_str.parse::<u64>() {
                return kb * 1024;
            }
        }
    }
    0
}

fn configure_lookaside(conn: &Connection) {
    // Mirrors `connection_runtime.rs::configure_reader_lookaside` before any PRAGMA.
    unsafe {
        let rc = rusqlite::ffi::sqlite3_db_config(
            conn.handle(),
            rusqlite::ffi::SQLITE_DBCONFIG_LOOKASIDE,
            std::ptr::null_mut::<std::ffi::c_void>(),
            READER_LOOKASIDE_SLOT_SIZE,
            READER_LOOKASIDE_SLOT_COUNT,
        );
        assert_eq!(rc, rusqlite::ffi::SQLITE_OK);
    }
}

fn open_writer_conn(path: &Path, sync_mode: SyncMode) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.set_prepared_statement_cache_capacity(64);
    conn.busy_timeout(std::time::Duration::from_millis(5_000))?;
    conn.pragma_update(None, "secure_delete", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", sync_mode.as_str())?;
    Ok(conn)
}

fn open_reader_conn(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    configure_lookaside(&conn);
    conn.set_prepared_statement_cache_capacity(64);
    conn.busy_timeout(std::time::Duration::from_millis(5_000))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "secure_delete", "ON")?;
    conn.pragma_update(None, "query_only", "ON")?;
    Ok(conn)
}

fn checkpoint_truncate_and_size(path: &Path) -> rusqlite::Result<DatabaseSizeReport> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    let _: (i64, i64, i64) = conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    })?;
    let page_size: u64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0))?;
    let page_count: u64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0))?;
    let freelist_count: u64 = conn.query_row("PRAGMA freelist_count", [], |r| r.get(0))?;
    drop(conn);
    let file_bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let live_btree_bytes = page_count.saturating_sub(freelist_count) * page_size;
    Ok(DatabaseSizeReport {
        layout: String::new(),
        file_bytes,
        page_size,
        page_count,
        freelist_count,
        live_btree_bytes,
        delta_bytes_vs_b0: None,
        relative_delta_vs_b0: None,
    })
}

fn check_vec0_16col_support(conn: &Connection) -> rusqlite::Result<Vec<i64>> {
    conn.execute_batch(
        "CREATE VIRTUAL TABLE _probe_vec0_16 USING vec0(
            embedding float[8],
            embedding_bin bit[8],
            source_type TEXT partition key,
            kind TEXT,
            created_at INTEGER,
            status TEXT,
            lifecycle INTEGER,
            valid_from INTEGER,
            valid_until INTEGER,
            scope TEXT,
            attr_a1 TEXT, attr_a2 TEXT, attr_a3 TEXT, attr_a4 TEXT,
            attr_a5 TEXT, attr_a6 TEXT, attr_a7 TEXT, attr_a8 TEXT,
            chunk_size=64
        );",
    )?;
    let v1 = encode_f32_blob(&[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    let v2 = encode_f32_blob(&[0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    conn.execute(
        "INSERT INTO _probe_vec0_16(
            rowid, embedding, embedding_bin, source_type, kind, created_at, status,
            lifecycle, valid_from, valid_until, scope,
            attr_a1, attr_a2, attr_a3, attr_a4, attr_a5, attr_a6, attr_a7, attr_a8
         ) VALUES (1, ?1, vec_quantize_binary(?1), 'article', 'note', 100, '', 0, -10, 9999999, 'alpha', '', '', '', '', '', '', '', '')",
        [&v1],
    )?;
    conn.execute(
        "INSERT INTO _probe_vec0_16(
            rowid, embedding, embedding_bin, source_type, kind, created_at, status,
            lifecycle, valid_from, valid_until, scope,
            attr_a1, attr_a2, attr_a3, attr_a4, attr_a5, attr_a6, attr_a7, attr_a8
         ) VALUES (2, ?1, vec_quantize_binary(?1), 'article', 'note', 100, '', 0, -10, 9999999, 'beta', '', '', '', '', '', '', '', '')",
        [&v2],
    )?;
    let mut stmt = conn.prepare(
        "SELECT rowid FROM _probe_vec0_16
         WHERE embedding_bin MATCH vec_quantize_binary(?1)
           AND scope = 'beta'
           AND lifecycle = 0
         ORDER BY distance LIMIT 10",
    )?;
    let rows = stmt.query_map([&v1], |r| r.get::<_, i64>(0))?;
    rows.collect()
}

fn probe_vec0_default_clause() -> bool {
    register_sqlite_vec();
    let Ok(conn) = Connection::open_in_memory() else {
        return false;
    };
    conn.execute_batch(
        "CREATE VIRTUAL TABLE _probe_def USING vec0(embedding float[4], scope TEXT DEFAULT '')",
    )
    .is_ok()
}

fn probe_vacuum_on_vec0() -> bool {
    register_sqlite_vec();
    let Ok(dir) = tempfile::tempdir() else {
        return false;
    };
    let p = dir.path().join("vac.sqlite");
    let Ok(conn) = Connection::open(&p) else {
        return false;
    };
    if check_vec0_16col_support(&conn).is_err() {
        return false;
    }
    conn.execute_batch("VACUUM;").is_ok()
}

// ---------------------------------------------------------------------------
// Real engine anchor build (`B0`) + layout derivation (`A1`, `A2`, `S39`)
// ---------------------------------------------------------------------------

fn copy_sqlite_clean(src: &Path, dst: &Path) -> Result<(), String> {
    for suffix in ["", "-wal", "-shm", ".lock"] {
        let p = PathBuf::from(format!("{}{suffix}", dst.display()));
        let _ = fs::remove_file(p);
    }
    fs::copy(src, dst).map_err(|e| format!("copy {} -> {}: {e}", src.display(), dst.display()))?;
    Ok(())
}

fn build_anchor_database(
    db_path: &Path,
    profile: ProfileKind,
    num_nodes: usize,
    num_op_state: usize,
    num_op_mutations: usize,
    emb_gen: &EmbeddingGenerator,
) -> Result<(), String> {
    for suffix in ["", "-wal", "-shm", ".lock"] {
        let p = PathBuf::from(format!("{}{suffix}", db_path.display()));
        let _ = fs::remove_file(p);
    }

    let opened = Engine::open(db_path).map_err(|e| format!("Engine::open anchor: {e:?}"))?;
    let engine = &opened.engine;

    // Register 8 Filterable Node projections (MAX_VEC0_NODE_PROPERTY_SLOTS = 8) + 1 Edge projection.
    let mut specs: Vec<ProjectionSpec> = NODE_PROP_NAMES
        .iter()
        .map(|&name| ProjectionSpec {
            name: name.to_string(),
            roles: BTreeSet::from([ProjectionRole::Filterable]),
            fts: None,
            vector: None,
            source: None,
            entity_class: PropertyEntityClass::Node,
        })
        .collect();
    specs.push(ProjectionSpec {
        name: "edge_tag".to_string(),
        roles: BTreeSet::from([ProjectionRole::Filterable]),
        fts: None,
        vector: None,
        source: None,
        entity_class: PropertyEntityClass::Edge,
    });
    engine
        .configure_projections(&specs, &[])
        .map_err(|e| format!("configure_projections: {e:?}"))?;

    // Register 5 operational latest_state collections + 1 append_only_log collection.
    let mut admin_writes: Vec<PreparedWrite> = OP_COLLECTIONS
        .iter()
        .map(|&col| PreparedWrite::AdminSchema {
            name: col.to_string(),
            kind: "latest_state".to_string(),
            schema_json: r#"{"type":"object"}"#.to_string(),
            retention_json: "{}".to_string(),
        })
        .collect();
    admin_writes.push(PreparedWrite::AdminSchema {
        name: OP_LOG_COLLECTION.to_string(),
        kind: "append_only_log".to_string(),
        schema_json: r#"{"type":"object"}"#.to_string(),
        retention_json: "{}".to_string(),
    });
    engine.write(&admin_writes).map_err(|e| format!("write admin schemas: {e:?}"))?;

    // Write nodes in batches of 200.
    let mut node_cursors = Vec::with_capacity(num_nodes);
    let mut node_body_lens = Vec::with_capacity(num_nodes);
    for start in (0..num_nodes).step_by(BATCH_SIZE) {
        let end = (start + BATCH_SIZE).min(num_nodes);
        let mut batch = Vec::with_capacity(end - start);
        for i in start..end {
            let topic = TOPICS[i % TOPICS.len()];
            let region = REGIONS[i % REGIONS.len()];
            let bucket = i % 500;
            let signal = i % 19;
            let body = format!(
                "Document {i} covers {topic} telemetry in {region} verification bucket_{bucket} metric_{signal} architecture benchmark"
            );
            node_body_lens.push(body.len() as i64);
            batch.push(PreparedWrite::Node {
                kind: "note".to_string(),
                body,
                source_id: SourceId::new(format!("src-{}", i % 50))
                    .map_err(|e| format!("SourceId: {e:?}"))?,
                logical_id: Some(format!("node:{i}")),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: Some(node_properties(i)),
            });
        }
        let receipt =
            engine.write(&batch).map_err(|e| format!("write node batch {start}: {e:?}"))?;
        node_cursors.extend_from_slice(&receipt.row_cursors);
    }

    // Write 1.5x intra-scope edges in batches of 200.
    let mut edge_cursors = Vec::with_capacity(num_nodes + num_nodes.div_ceil(2));
    let mut edge_owner_nodes = Vec::with_capacity(num_nodes + num_nodes.div_ceil(2));
    let mut edge_batch = Vec::with_capacity(BATCH_SIZE);
    for i in 0..num_nodes {
        let (s_start, s_end) = scope_node_bounds(profile, i, num_nodes);
        let span = (s_end - s_start).max(1);
        let to1 = s_start + ((i - s_start + 1) % span);
        let mut eprops1 = PropertyMap::new();
        eprops1.insert(
            "edge_tag".to_string(),
            PropertyValue::String(if i % 5 == 0 { "hot" } else { "warm" }.to_string()),
        );
        edge_batch.push(PreparedWrite::Edge {
            kind: "relates_to".to_string(),
            from: format!("node:{i}"),
            to: format!("node:{to1}"),
            source_id: SourceId::new(format!("src-{}", i % 50)).unwrap(),
            logical_id: Some(format!("edge:a:{i}")),
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: Some(0.5 + (i % 50) as f64 * 0.01),
            extractor_model_id: None,
            temporal_fallback: None,
            properties: Some(eprops1),
        });
        edge_owner_nodes.push(i);
        if i % 2 == 0 {
            let to2 = s_start + ((i - s_start + 7) % span);
            let mut eprops2 = PropertyMap::new();
            eprops2.insert("edge_tag".to_string(), PropertyValue::String("hot".to_string()));
            edge_batch.push(PreparedWrite::Edge {
                kind: "verifies".to_string(),
                from: format!("node:{i}"),
                to: format!("node:{to2}"),
                source_id: SourceId::new(format!("src-{}", i % 50)).unwrap(),
                logical_id: Some(format!("edge:b:{i}")),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.95),
                extractor_model_id: None,
                temporal_fallback: None,
                properties: Some(eprops2),
            });
            edge_owner_nodes.push(i);
        }
        if edge_batch.len() >= BATCH_SIZE {
            let drain_len = BATCH_SIZE.min(edge_batch.len());
            let chunk: Vec<PreparedWrite> = edge_batch.drain(..drain_len).collect();
            let receipt =
                engine.write(&chunk).map_err(|e| format!("write edge batch at {i}: {e:?}"))?;
            edge_cursors.extend_from_slice(&receipt.row_cursors);
        }
    }
    if !edge_batch.is_empty() {
        let receipt =
            engine.write(&edge_batch).map_err(|e| format!("write tail edge batch: {e:?}"))?;
        edge_cursors.extend_from_slice(&receipt.row_cursors);
    }

    // Write operational_state checkpoints & operational_mutations audit log rows.
    for start in (0..num_op_state).step_by(BATCH_SIZE) {
        let end = (start + BATCH_SIZE).min(num_op_state);
        let mut batch = Vec::with_capacity(end - start);
        for i in start..end {
            let col = OP_COLLECTIONS[i % OP_COLLECTIONS.len()];
            batch.push(PreparedWrite::OpStore {
                collection: col.to_string(),
                record_key: format!("cp:{i}"),
                schema_id: Some(col.to_string()),
                body: format!(r#"{{"step":{i},"status":"ok","seq":{}}}"#, i * 3),
            });
        }
        engine.write(&batch).map_err(|e| format!("write op_state batch {start}: {e:?}"))?;
    }
    for start in (0..num_op_mutations).step_by(BATCH_SIZE) {
        let end = (start + BATCH_SIZE).min(num_op_mutations);
        let mut batch = Vec::with_capacity(end - start);
        for i in start..end {
            batch.push(PreparedWrite::OpStore {
                collection: OP_LOG_COLLECTION.to_string(),
                record_key: format!("audit:{i}"),
                schema_id: Some(OP_LOG_COLLECTION.to_string()),
                body: format!(r#"{{"event":"write","seq":{i}}}"#),
            });
        }
        engine.write(&batch).map_err(|e| format!("write op_mutations batch {start}: {e:?}"))?;
    }

    engine.drain(30_000).map_err(|e| format!("drain anchor: {e:?}"))?;
    opened.engine.close().map_err(|e| format!("close anchor: {e:?}"))?;

    // Populate realistic 384-dim deterministic embeddings into the engine-created `vector_default`
    // table (which has chunk_size=64 and all 8 `attr_<hex>` slots from `configure_projections`).
    let mut conn = open_writer_conn(db_path, SyncMode::Normal)
        .map_err(|e| format!("open writer for vec0 seed: {e}"))?;
    // Give one-time anchor setup/VACUUM a 64 MiB cache; timed open/write/read connections still
    // use default production pragmas via `open_writer_conn` / `open_reader_conn`.
    let _ = conn.pragma_update(None, "cache_size", -65_536);
    conn.execute(
        "INSERT OR IGNORE INTO _fathomdb_vector_kinds(kind, profile, created_at) VALUES('note', 'default', 1)",
        [],
    )
    .map_err(|e| format!("enroll note vector kind: {e}"))?;

    let attr_cols: Vec<String> = NODE_PROP_NAMES.iter().map(|n| attr_vec0_column(n)).collect();
    let cols_joined = attr_cols.join(", ");
    let vec_insert_sql = format!(
        "INSERT INTO vector_default(\
            rowid, embedding, embedding_bin, source_type, kind, created_at, status, \
            lifecycle, valid_from, valid_until, scope, {cols_joined}\
         ) VALUES(?1, ?2, vec_quantize_binary(?2), 'note', 'note', ?3, '', 0, -9223372036854775808, 9223372036854775807, \
                  ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"
    );

    let chunk_tx_size = 2000;
    for start in (0..num_nodes).step_by(chunk_tx_size) {
        let end = (start + chunk_tx_size).min(num_nodes);
        let tx = conn.transaction().map_err(|e| format!("vec0 tx begin: {e}"))?;
        {
            let mut stmt_vr = tx
                .prepare_cached("INSERT INTO _fathomdb_vector_rows(rowid, kind, write_cursor) VALUES(?1, 'note', ?1)")
                .map_err(|e| format!("prep vr: {e}"))?;
            let mut stmt_vs = tx
                .prepare_cached("INSERT INTO _fathomdb_vector_spans(vector_rowid, write_cursor, window_number, byte_start, byte_end, body_length) VALUES(?1, ?1, 0, 0, ?2, ?2)")
                .map_err(|e| format!("prep vs: {e}"))?;
            let mut stmt_vec =
                tx.prepare_cached(&vec_insert_sql).map_err(|e| format!("prep vec0: {e}"))?;
            let mut stmt_term = tx
                .prepare_cached("INSERT OR REPLACE INTO _fathomdb_projection_terminal(write_cursor, state) VALUES(?1, 'up_to_date')")
                .map_err(|e| format!("prep term: {e}"))?;

            for i in start..end {
                let cursor = node_cursors[i] as i64;
                let body_len = node_body_lens[i];
                let vec_blob = encode_f32_blob(&emb_gen.node_vector(i));
                let scope = scope_for_node_scaled(profile, i, num_nodes);
                let props = node_properties(i);
                let a0 = encode_attr_vec0_text(&props["tier"]);
                let a1 = encode_attr_vec0_text(&props["topic"]);
                let a2 = encode_attr_vec0_text(&props["region"]);
                let a3 = encode_attr_vec0_text(&props["env"]);
                let a4 = encode_attr_vec0_text(&props["bucket"]);
                let a5 = encode_attr_vec0_text(&props["priority_label"]);
                let a6 = encode_attr_vec0_text(&props["owner_team"]);
                let a7 = encode_attr_vec0_text(&props["active_flag"]);
                stmt_vr.execute([cursor]).map_err(|e| format!("exec vr: {e}"))?;
                stmt_vs.execute(params![cursor, body_len]).map_err(|e| format!("exec vs: {e}"))?;
                stmt_vec
                    .execute(params![
                        cursor,
                        vec_blob,
                        1_700_000_000_i64 + i as i64,
                        scope,
                        a0,
                        a1,
                        a2,
                        a3,
                        a4,
                        a5,
                        a6,
                        a7
                    ])
                    .map_err(|e| format!("exec vec0: {e}"))?;
                stmt_term.execute([cursor]).map_err(|e| format!("exec term: {e}"))?;
            }
        }
        tx.commit().map_err(|e| format!("vec0 tx commit: {e}"))?;
    }

    // Populate scope on multi-scope profiles cleanly before snapshots/indexes are derived.
    // Every UPDATE hits an indexed integer PK or `write_cursor` / PK index (zero O(N²) table scans).
    if profile != ProfileKind::SingleScope {
        let tx = conn.transaction().map_err(|e| format!("scope update tx: {e}"))?;
        tx.execute(
            "UPDATE _fathomdb_read_visibility_state SET coalescing=1 WHERE singleton=1 AND coalescing=0",
            [],
        )
        .map_err(|e| format!("arm vis setup: {e}"))?;
        {
            let mut upd_node = tx
                .prepare_cached("UPDATE canonical_nodes SET scope = ?1 WHERE write_cursor = ?2")
                .map_err(|e| format!("prep upd node: {e}"))?;
            for (i, &cursor) in node_cursors.iter().enumerate() {
                let sc = scope_for_node_scaled(profile, i, num_nodes);
                upd_node
                    .execute(params![sc, cursor as i64])
                    .map_err(|e| format!("upd node scope: {e}"))?;
            }
            let mut upd_edge = tx
                .prepare_cached("UPDATE canonical_edges SET scope = ?1 WHERE write_cursor = ?2")
                .map_err(|e| format!("prep upd edge: {e}"))?;
            for (&ecursor, &owner_idx) in edge_cursors.iter().zip(edge_owner_nodes.iter()) {
                let sc = scope_for_node_scaled(profile, owner_idx, num_nodes);
                upd_edge
                    .execute(params![sc, ecursor as i64])
                    .map_err(|e| format!("upd edge scope: {e}"))?;
            }

            let mut upd_state = tx
                .prepare_cached(
                    "UPDATE operational_state SET scope = ?1 WHERE collection_name = ?2 AND record_key = ?3",
                )
                .map_err(|e| format!("prep upd op_state: {e}"))?;
            for i in 0..num_op_state {
                // Mirror node skew/uniformity ratio across op_state keys.
                let scaled_idx = (i * num_nodes) / num_op_state.max(1);
                let sc = scope_for_node_scaled(profile, scaled_idx, num_nodes);
                let col = OP_COLLECTIONS[i % OP_COLLECTIONS.len()];
                upd_state
                    .execute(params![sc, col, format!("cp:{i}")])
                    .map_err(|e| format!("upd op_state scope: {e}"))?;
            }

            let op_mut_rows: Vec<(i64, String)> = {
                let mut stmt = tx
                    .prepare("SELECT id, record_key FROM operational_mutations ORDER BY id ASC")
                    .map_err(|e| format!("prep select op_mut ids: {e}"))?;
                let rows = stmt
                    .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
                    .map_err(|e| format!("query op_mut ids: {e}"))?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(|e| format!("collect op_mut ids: {e}"))?
            };
            let mut upd_mut = tx
                .prepare_cached("UPDATE operational_mutations SET scope = ?1 WHERE id = ?2")
                .map_err(|e| format!("prep upd op_mut: {e}"))?;
            for (mut_id, record_key) in op_mut_rows {
                let idx: usize = record_key
                    .strip_prefix("audit:")
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| format!("unexpected audit record_key: {record_key}"))?;
                let scaled_idx = (idx * num_nodes) / num_op_mutations.max(1);
                let sc = scope_for_node_scaled(profile, scaled_idx, num_nodes);
                upd_mut
                    .execute(params![sc, mut_id])
                    .map_err(|e| format!("upd op_mut scope: {e}"))?;
            }
        }
        tx.execute(
            "UPDATE _fathomdb_read_visibility_state
             SET coalescing=0,
                 generation=generation+1,
                 state_nonce=lower(hex(randomblob(32)))
             WHERE singleton=1 AND coalescing=1",
            [],
        )
        .map_err(|e| format!("disarm vis setup: {e}"))?;
        tx.commit().map_err(|e| format!("scope update commit: {e}"))?;
    }

    // Compact any page splits / freelist pages on the anchor across ALL profiles so B0, A1,
    // A2, and S39 start at freelist_count = 0 for apples-to-apples database_bytes comparisons.
    conn.execute_batch("VACUUM;").map_err(|e| format!("vacuum anchor: {e}"))?;

    let _: (i64, i64, i64) = conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(|e| format!("checkpoint anchor: {e}"))?;
    Ok(())
}

fn apply_s38_partial_indexes(db_path: &Path) -> Result<(), String> {
    let conn =
        open_writer_conn(db_path, SyncMode::Normal).map_err(|e| format!("open s38_a1: {e}"))?;
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS canonical_nodes_scope_cursor_idx
             ON canonical_nodes(scope, write_cursor) WHERE scope != '';
         CREATE INDEX IF NOT EXISTS canonical_nodes_scope_logical_active_idx
             ON canonical_nodes(scope, logical_id)
             WHERE scope != '' AND superseded_at IS NULL;
         CREATE INDEX IF NOT EXISTS canonical_nodes_scope_active_cursor_idx
             ON canonical_nodes(scope, write_cursor)
             WHERE scope != '' AND superseded_at IS NULL AND state = 'active';
         CREATE INDEX IF NOT EXISTS canonical_nodes_scope_kind_cursor_idx
             ON canonical_nodes(scope, kind, write_cursor)
             WHERE scope != '' AND logical_id IS NOT NULL;
         CREATE INDEX IF NOT EXISTS canonical_edges_scope_from_active_idx
             ON canonical_edges(scope, from_id, kind)
             WHERE scope != '' AND superseded_at IS NULL;
         CREATE INDEX IF NOT EXISTS canonical_edges_scope_to_active_idx
             ON canonical_edges(scope, to_id, kind)
             WHERE scope != '' AND superseded_at IS NULL;
         CREATE INDEX IF NOT EXISTS canonical_edges_scope_cursor_idx
             ON canonical_edges(scope, write_cursor)
             WHERE scope != '';
         CREATE INDEX IF NOT EXISTS operational_state_scope_pk_idx
             ON operational_state(scope, collection_name, record_key)
             WHERE scope != '';
         CREATE INDEX IF NOT EXISTS operational_state_scope_collection_cursor_idx
             ON operational_state(scope, collection_name, write_cursor)
             WHERE scope != '';
         CREATE INDEX IF NOT EXISTS operational_mutations_scope_collection_id_idx
             ON operational_mutations(scope, collection_name, id)
             WHERE scope != '';",
    )
    .map_err(|e| format!("create partial indexes: {e}"))?;
    let _: (i64, i64, i64) = conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(|e| format!("checkpoint s38_a1: {e}"))?;
    Ok(())
}

fn reshape_vec0_to_8col(db_path: &Path) -> Result<(), String> {
    let mut conn = open_writer_conn(db_path, SyncMode::Normal)
        .map_err(|e| format!("open for vec0 reshape: {e}"))?;
    let _ = conn.pragma_update(None, "cache_size", -65_536);
    let attr_cols: Vec<String> = NODE_PROP_NAMES.iter().map(|n| attr_vec0_column(n)).collect();
    let stage_attr_defs: String = attr_cols.iter().map(|c| format!(", {c} TEXT")).collect();
    let stage_attr_cols: String = attr_cols.iter().map(|c| format!(", {c}")).collect();

    let tx = conn.transaction().map_err(|e| format!("reshape tx: {e}"))?;
    tx.execute_batch(&format!(
        "CREATE TABLE _fathomdb_vec_reshape_tmp (
            rowid INTEGER PRIMARY KEY,
            embedding BLOB NOT NULL,
            embedding_bin BLOB NOT NULL,
            source_type TEXT,
            kind TEXT,
            created_at INTEGER,
            status TEXT,
            lifecycle INTEGER,
            valid_from INTEGER,
            valid_until INTEGER{stage_attr_defs}
         );
         INSERT INTO _fathomdb_vec_reshape_tmp(
            rowid, embedding, embedding_bin, source_type, kind, created_at,
            status, lifecycle, valid_from, valid_until{stage_attr_cols}
         )
         SELECT rowid, embedding, embedding_bin, source_type, kind, created_at,
                status, lifecycle, valid_from, valid_until{stage_attr_cols}
         FROM vector_default;
         DROP TABLE vector_default;
         CREATE VIRTUAL TABLE vector_default USING vec0(
            embedding float[{EMBED_DIM}],
            embedding_bin bit[{EMBED_DIM}],
            source_type TEXT partition key,
            kind TEXT,
            created_at INTEGER,
            status TEXT,
            lifecycle INTEGER,
            valid_from INTEGER,
            valid_until INTEGER,
            scope TEXT{stage_attr_defs},
            chunk_size=64
         );"
    ))
    .map_err(|e| format!("stage/drop/recreate vec0: {e}"))?;

    let insert_sql = format!(
        "INSERT INTO vector_default(
            rowid, embedding, embedding_bin, source_type, kind, created_at,
            status, lifecycle, valid_from, valid_until, scope{stage_attr_cols}
         ) VALUES(?1, ?2, vec_bit(?3), ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)"
    );
    let select_sql = format!(
        "SELECT s.rowid, s.embedding, s.embedding_bin, s.source_type, s.kind, s.created_at,
                s.status, s.lifecycle, s.valid_from, s.valid_until,
                COALESCE(cn.scope, ce.scope, '') AS resolved_scope{stage_attr_cols}
         FROM _fathomdb_vec_reshape_tmp s
         LEFT JOIN _fathomdb_vector_rows vr ON vr.rowid = s.rowid
         LEFT JOIN canonical_nodes cn ON cn.write_cursor = vr.write_cursor
         LEFT JOIN canonical_edges ce ON ce.write_cursor = vr.write_cursor"
    );
    {
        let mut rstmt = tx.prepare(&select_sql).map_err(|e| format!("prep select reshape: {e}"))?;
        let mut wstmt = tx.prepare(&insert_sql).map_err(|e| format!("prep insert reshape: {e}"))?;
        let mut rows = rstmt.query([]).map_err(|e| format!("query reshape: {e}"))?;
        while let Some(row) = rows.next().map_err(|e| format!("next reshape: {e}"))? {
            let mut vals: Vec<rusqlite::types::Value> = Vec::with_capacity(19);
            for c in 0..19 {
                vals.push(row.get(c).map_err(|e| format!("get col {c}: {e}"))?);
            }
            wstmt
                .execute(rusqlite::params_from_iter(vals.iter()))
                .map_err(|e| format!("insert reshaped row: {e}"))?;
        }
    }
    tx.execute_batch("DROP TABLE _fathomdb_vec_reshape_tmp;")
        .map_err(|e| format!("drop stage: {e}"))?;
    tx.commit().map_err(|e| format!("commit reshape: {e}"))?;

    // Compact freed pages from DROP TABLE vector_default so file_bytes reflects true 8-col size.
    conn.execute_batch("VACUUM;").map_err(|e| format!("vacuum after reshape: {e}"))?;
    let _: (i64, i64, i64) = conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(|e| format!("checkpoint after reshape: {e}"))?;
    Ok(())
}

fn transform_to_s39_composite(db_path: &Path) -> Result<(), String> {
    let mut conn =
        open_writer_conn(db_path, SyncMode::Normal).map_err(|e| format!("open s39: {e}"))?;
    let _ = conn.pragma_update(None, "cache_size", -65_536);
    let trigger_sqls: Vec<String> = {
        let mut stmt = conn
            .prepare(
                "SELECT sql FROM sqlite_master
                 WHERE type = 'trigger'
                   AND tbl_name IN ('canonical_nodes', 'canonical_edges', 'operational_state', 'operational_mutations')
                   AND sql IS NOT NULL",
            )
            .map_err(|e| format!("read triggers: {e}"))?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| format!("query triggers: {e}"))?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| format!("collect triggers: {e}"))?
    };

    let tx = conn.transaction().map_err(|e| format!("s39 tx: {e}"))?;
    tx.execute_batch(
        "CREATE TABLE _s39_nodes_tmp AS SELECT * FROM canonical_nodes;
         DROP TABLE canonical_nodes;
         CREATE TABLE canonical_nodes(
             write_cursor INTEGER NOT NULL,
             kind TEXT NOT NULL,
             body TEXT NOT NULL,
             source_id TEXT,
             logical_id TEXT,
             superseded_at INTEGER,
             row_kind TEXT NOT NULL DEFAULT 'leaf',
             importance REAL,
             state TEXT NOT NULL DEFAULT 'active',
             reason TEXT,
             valid_from INTEGER,
             valid_until INTEGER,
             properties TEXT,
             scope TEXT NOT NULL DEFAULT ''
         );
         INSERT INTO canonical_nodes(
             write_cursor, kind, body, source_id, logical_id, superseded_at,
             row_kind, importance, state, reason, valid_from, valid_until, properties, scope
         ) SELECT write_cursor, kind, body, source_id, logical_id, superseded_at,
                  row_kind, importance, state, reason, valid_from, valid_until, properties, scope
           FROM _s39_nodes_tmp;
         DROP TABLE _s39_nodes_tmp;
         CREATE INDEX canonical_nodes_scope_source_id_idx ON canonical_nodes(scope, source_id);
         CREATE UNIQUE INDEX canonical_nodes_scope_logical_active_idx
             ON canonical_nodes(scope, logical_id) WHERE superseded_at IS NULL;
         CREATE INDEX canonical_nodes_scope_kind_idx ON canonical_nodes(scope, kind);
         CREATE INDEX canonical_nodes_scope_state_active_idx
             ON canonical_nodes(scope, write_cursor) WHERE state = 'active' AND superseded_at IS NULL;
         CREATE INDEX canonical_nodes_scope_validity_idx
             ON canonical_nodes(scope, valid_from, valid_until)
             WHERE superseded_at IS NULL AND state = 'active';
         CREATE INDEX canonical_nodes_write_cursor_idx ON canonical_nodes(write_cursor);
         CREATE INDEX canonical_nodes_scope_cursor_idx ON canonical_nodes(scope, write_cursor);
         CREATE UNIQUE INDEX canonical_nodes_scope_kind_cursor_page_idx
             ON canonical_nodes(scope, kind, write_cursor) WHERE logical_id IS NOT NULL;
         CREATE INDEX canonical_nodes_scope_logical_sup_idx
             ON canonical_nodes(scope, logical_id, superseded_at, write_cursor)
             WHERE superseded_at IS NOT NULL;

         CREATE TABLE _s39_edges_tmp AS SELECT * FROM canonical_edges;
         DROP TABLE canonical_edges;
         CREATE TABLE canonical_edges(
             write_cursor INTEGER NOT NULL,
             kind TEXT NOT NULL,
             from_id TEXT NOT NULL,
             to_id TEXT NOT NULL,
             source_id TEXT,
             logical_id TEXT,
             superseded_at INTEGER,
             body TEXT,
             t_valid INTEGER CHECK (t_valid IS NULL OR typeof(t_valid) = 'integer'),
             t_invalid INTEGER CHECK (t_invalid IS NULL OR typeof(t_invalid) = 'integer'),
             confidence REAL,
             extractor_model_id TEXT,
             temporal_fallback INTEGER,
             properties TEXT,
             scope TEXT NOT NULL DEFAULT ''
         );
         INSERT INTO canonical_edges(
             write_cursor, kind, from_id, to_id, source_id, logical_id, superseded_at,
             body, t_valid, t_invalid, confidence, extractor_model_id, temporal_fallback,
             properties, scope
         ) SELECT write_cursor, kind, from_id, to_id, source_id, logical_id, superseded_at,
                  body, t_valid, t_invalid, confidence, extractor_model_id, temporal_fallback,
                  properties, scope
           FROM _s39_edges_tmp;
         DROP TABLE _s39_edges_tmp;
         CREATE INDEX canonical_edges_scope_source_id_idx ON canonical_edges(scope, source_id);
         CREATE UNIQUE INDEX canonical_edges_scope_logical_active_idx
             ON canonical_edges(scope, logical_id) WHERE superseded_at IS NULL;
         CREATE INDEX canonical_edges_scope_from_id_idx ON canonical_edges(scope, from_id);
         CREATE INDEX canonical_edges_scope_to_id_idx ON canonical_edges(scope, to_id);
         CREATE INDEX canonical_edges_write_cursor_idx ON canonical_edges(write_cursor);
         CREATE INDEX canonical_edges_scope_cursor_idx ON canonical_edges(scope, write_cursor);
         CREATE INDEX canonical_edges_scope_from_active_idx
             ON canonical_edges(scope, from_id, kind) WHERE superseded_at IS NULL;
         CREATE INDEX canonical_edges_scope_to_active_idx
             ON canonical_edges(scope, to_id, kind) WHERE superseded_at IS NULL;
         CREATE INDEX canonical_edges_scope_from_sup_kind_idx
             ON canonical_edges(scope, from_id, superseded_at, kind) WHERE superseded_at IS NOT NULL;
         CREATE INDEX canonical_edges_scope_to_sup_kind_idx
             ON canonical_edges(scope, to_id, superseded_at, kind) WHERE superseded_at IS NOT NULL;

         CREATE TABLE _s39_op_state_tmp AS SELECT * FROM operational_state;
         DROP TABLE operational_state;
         CREATE TABLE operational_state(
             collection_name TEXT NOT NULL,
             record_key TEXT NOT NULL,
             payload_json TEXT NOT NULL,
             schema_id TEXT,
             write_cursor INTEGER NOT NULL,
             scope TEXT NOT NULL DEFAULT '',
             PRIMARY KEY(scope, collection_name, record_key)
         );
         INSERT INTO operational_state(collection_name, record_key, payload_json, schema_id, write_cursor, scope)
         SELECT collection_name, record_key, payload_json, schema_id, write_cursor, scope FROM _s39_op_state_tmp;
         DROP TABLE _s39_op_state_tmp;
         CREATE UNIQUE INDEX operational_state_scope_collection_cursor_page_idx
             ON operational_state(scope, collection_name, write_cursor);
         CREATE INDEX operational_state_write_cursor_idx ON operational_state(write_cursor);

         CREATE TABLE _s39_op_mut_tmp AS SELECT * FROM operational_mutations;
         DROP TABLE operational_mutations;
         CREATE TABLE operational_mutations(
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             collection_name TEXT NOT NULL,
             record_key TEXT NOT NULL,
             op_kind TEXT NOT NULL CHECK(op_kind = 'append'),
             payload_json TEXT NOT NULL,
             schema_id TEXT,
             write_cursor INTEGER NOT NULL,
             scope TEXT NOT NULL DEFAULT ''
         );
         INSERT INTO operational_mutations(id, collection_name, record_key, op_kind, payload_json, schema_id, write_cursor, scope)
         SELECT id, collection_name, record_key, op_kind, payload_json, schema_id, write_cursor, scope FROM _s39_op_mut_tmp;
         DROP TABLE _s39_op_mut_tmp;
         CREATE INDEX operational_mutations_scope_collection_id_idx
             ON operational_mutations(scope, collection_name, id);

         PRAGMA user_version = 39;",
    )
    .map_err(|e| format!("recreate s39 tables: {e}"))?;

    for sql in &trigger_sqls {
        tx.execute_batch(sql).map_err(|e| format!("restore trigger on s39: {e}: {sql}"))?;
    }
    tx.commit().map_err(|e| format!("commit s39: {e}"))?;

    // Reshape vec0 to native 8-col and vacuum inside reshape_vec0_to_8col.
    reshape_vec0_to_8col(db_path)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Open probes (fresh subprocesses via `--open-probe`)
// ---------------------------------------------------------------------------

fn run_engine_equivalent_open_once(
    path: &Path,
    layout: LayoutId,
    sync_mode: SyncMode,
) -> Result<(), String> {
    let conn = open_writer_conn(path, sync_mode).map_err(|e| format!("open writer: {e}"))?;
    let _: i64 = conn
        .query_row("PRAGMA application_id", [], |r| r.get(0))
        .map_err(|e| format!("app id: {e}"))?;
    let _: i64 = conn
        .query_row("SELECT COUNT(*) FROM sqlite_schema", [], |r| r.get(0))
        .map_err(|e| format!("schema probe: {e}"))?;
    let expected_ver = if layout.is_s39() { 39_u32 } else { fathomdb_schema::SCHEMA_VERSION };
    let seen_ver: u32 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|e| format!("user_version: {e}"))?;
    if seen_ver != expected_ver {
        return Err(format!("schema version mismatch: got {seen_ver}, expected {expected_ver}"));
    }
    let _: String = conn
        .query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key = '_fathomdb_dependency_generation'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| format!("dep gen: {e}"))?;
    let _: (i64, String) = conn
        .query_row(
            "SELECT generation, state_nonce FROM _fathomdb_read_visibility_state WHERE singleton = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| format!("visibility state: {e}"))?;
    let _: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='vector_default'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| format!("vec0 master check: {e}"))?;
    let _: (u64, u64, u64, u64) = conn
        .query_row(
            "SELECT
                (SELECT COALESCE(MAX(write_cursor),0) FROM canonical_nodes),
                (SELECT COALESCE(MAX(write_cursor),0) FROM canonical_edges),
                (SELECT COALESCE(MAX(write_cursor),0) FROM operational_mutations),
                (SELECT COALESCE(MAX(write_cursor),0) FROM operational_state)",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|e| format!("next cursor check: {e}"))?;
    let _: i64 = conn
        .query_row("SELECT COUNT(*) FROM _fathomdb_projection_registry", [], |r| r.get(0))
        .map_err(|e| format!("registry count: {e}"))?;

    let mut readers = Vec::with_capacity(READER_POOL_SIZE);
    for _ in 0..READER_POOL_SIZE {
        readers.push(open_reader_conn(path).map_err(|e| format!("open reader: {e}"))?);
    }
    drop(readers);
    drop(conn);
    Ok(())
}

fn run_open_probe_child(
    db_path: &Path,
    layout: LayoutId,
    sync_mode: SyncMode,
    warmup: usize,
    count: usize,
    real_engine: bool,
) -> Result<ChildProbeOutput, String> {
    if !real_engine {
        register_sqlite_vec();
    }
    let mut samples = Vec::with_capacity(count);
    for i in 0..(warmup + count) {
        let t0 = Instant::now();
        if real_engine {
            let opened = Engine::open(db_path).map_err(|e| format!("Engine::open: {e:?}"))?;
            opened.engine.close().map_err(|e| format!("Engine::close: {e:?}"))?;
        } else {
            run_engine_equivalent_open_once(db_path, layout, sync_mode)?;
        }
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        if i >= warmup {
            samples.push(ms);
        }
    }
    Ok(ChildProbeOutput { samples_ms: samples, peak_rss_bytes: read_vm_hwm_bytes() })
}

fn invoke_open_probe(
    exe: &Path,
    db_path: &Path,
    layout: LayoutId,
    sync_mode: SyncMode,
    warmup: usize,
    count: usize,
    real_engine: bool,
) -> Result<ChildProbeOutput, String> {
    let mut cmd = Command::new(exe);
    cmd.args([
        "--open-probe",
        "--db",
        &db_path.to_string_lossy(),
        "--layout",
        layout.cli_flag(),
        "--sync",
        sync_mode.cli_flag(),
        "--warmup",
        &warmup.to_string(),
        "--count",
        &count.to_string(),
    ]);
    if real_engine {
        cmd.arg("--real-engine");
    }
    let out = cmd.output().map_err(|e| format!("spawn open-probe: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "open-probe failed (rc={:?}): stderr={}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("parse open-probe json: {e}"))
}

fn measure_open_suite(
    exe: &Path,
    db_paths: &[(LayoutId, PathBuf)],
    real_engine_layouts: &[LayoutId],
) -> Result<(Vec<OpenProbeResult>, Vec<PairedComparison>), String> {
    let mut results = Vec::new();
    let mut proc_by_key: HashMap<String, Vec<f64>> = HashMap::new();
    let mut warm_by_key: HashMap<String, Vec<f64>> = HashMap::new();

    // 1. Engine-equivalent subprocess probe across all layouts (apples-to-apples).
    let mut cold_samples: HashMap<LayoutId, Vec<f64>> = HashMap::new();
    let mut cold_rss: HashMap<LayoutId, Vec<u64>> = HashMap::new();
    for t in 0..(WARMUP_ROUNDS + OPEN_TRIALS) {
        let mut order: Vec<(LayoutId, PathBuf)> = db_paths.to_vec();
        if t % 2 == 1 {
            order.reverse();
        }
        for (layout, path) in order {
            let out = invoke_open_probe(exe, &path, layout, SyncMode::Normal, 0, 1, false)?;
            if t >= WARMUP_ROUNDS {
                cold_samples.entry(layout).or_default().push(out.samples_ms[0]);
                cold_rss.entry(layout).or_default().push(out.peak_rss_bytes);
            }
        }
    }

    for (layout, path) in db_paths {
        let half = OPEN_TRIALS.div_ceil(2);
        let mut warm_samples = Vec::with_capacity(OPEN_TRIALS);
        let mut max_rss = *cold_rss[layout].iter().max().unwrap_or(&0);
        for count in [half, OPEN_TRIALS - half] {
            let out = invoke_open_probe(
                exe,
                path,
                *layout,
                SyncMode::Normal,
                WARMUP_ROUNDS,
                count,
                false,
            )?;
            warm_samples.extend(out.samples_ms);
            max_rss = max_rss.max(out.peak_rss_bytes);
        }
        let rss_sorted = {
            let mut v = cold_rss[layout].clone();
            v.sort_unstable();
            v
        };
        let p50_rss = rss_sorted[rss_sorted.len() / 2];
        let key = format!("equiv:{}", layout.as_str());
        proc_by_key.insert(key.clone(), cold_samples[layout].clone());
        warm_by_key.insert(key, warm_samples.clone());
        results.push(OpenProbeResult {
            layout: layout.as_str().to_string(),
            sync_mode: "NORMAL".to_string(),
            probe_mode: "engine_equivalent_subprocess".to_string(),
            real_engine: false,
            process_open: summarize_samples(&cold_samples[layout]),
            warm_reopen: summarize_samples(&warm_samples),
            peak_rss_bytes_max: max_rss,
            peak_rss_bytes_p50: p50_rss,
        });
    }

    // 2. Real Engine::open subprocess probe on Schema-38 compatible layouts (`B0` & `S38-PARTIAL-A1`).
    let real_targets: Vec<(LayoutId, PathBuf)> =
        db_paths.iter().filter(|(l, _)| real_engine_layouts.contains(l)).cloned().collect();
    if !real_targets.is_empty() {
        let mut real_cold: HashMap<LayoutId, Vec<f64>> = HashMap::new();
        let mut real_rss: HashMap<LayoutId, Vec<u64>> = HashMap::new();
        for t in 0..(WARMUP_ROUNDS + OPEN_TRIALS) {
            let mut order = real_targets.clone();
            if t % 2 == 1 {
                order.reverse();
            }
            for (layout, path) in order {
                let out = invoke_open_probe(exe, &path, layout, SyncMode::Normal, 0, 1, true)?;
                if t >= WARMUP_ROUNDS {
                    real_cold.entry(layout).or_default().push(out.samples_ms[0]);
                    real_rss.entry(layout).or_default().push(out.peak_rss_bytes);
                }
            }
        }
        for (layout, path) in &real_targets {
            let half = OPEN_TRIALS.div_ceil(2);
            let mut warm_samples = Vec::with_capacity(OPEN_TRIALS);
            let mut max_rss = *real_rss[layout].iter().max().unwrap_or(&0);
            for count in [half, OPEN_TRIALS - half] {
                let out = invoke_open_probe(
                    exe,
                    path,
                    *layout,
                    SyncMode::Normal,
                    WARMUP_ROUNDS,
                    count,
                    true,
                )?;
                warm_samples.extend(out.samples_ms);
                max_rss = max_rss.max(out.peak_rss_bytes);
            }
            let rss_sorted = {
                let mut v = real_rss[layout].clone();
                v.sort_unstable();
                v
            };
            let p50_rss = rss_sorted[rss_sorted.len() / 2];
            let key = format!("real:{}", layout.as_str());
            proc_by_key.insert(key.clone(), real_cold[layout].clone());
            warm_by_key.insert(key, warm_samples.clone());
            results.push(OpenProbeResult {
                layout: layout.as_str().to_string(),
                sync_mode: "NORMAL".to_string(),
                probe_mode: "real_engine_open_subprocess".to_string(),
                real_engine: true,
                process_open: summarize_samples(&real_cold[layout]),
                warm_reopen: summarize_samples(&warm_samples),
                peak_rss_bytes_max: max_rss,
                peak_rss_bytes_p50: p50_rss,
            });
        }
    }

    let mut comparisons = Vec::new();
    let base_equiv =
        if proc_by_key.contains_key("equiv:B0") { "equiv:B0" } else { "equiv:S38-PARTIAL-A1" };
    let base_name = base_equiv.strip_prefix("equiv:").unwrap();
    for (layout, _) in db_paths {
        let cand_key = format!("equiv:{}", layout.as_str());
        if cand_key != base_equiv {
            comparisons.push(compare_paired(
                &format!("process_open:{base_name}"),
                &format!("process_open:{}", layout.as_str()),
                &proc_by_key[base_equiv],
                &proc_by_key[&cand_key],
                DEFAULT_SEED,
            ));
            comparisons.push(compare_paired(
                &format!("warm_reopen:{base_name}"),
                &format!("warm_reopen:{}", layout.as_str()),
                &warm_by_key[base_equiv],
                &warm_by_key[&cand_key],
                DEFAULT_SEED + 1,
            ));
        }
    }
    if proc_by_key.contains_key("real:B0") && proc_by_key.contains_key("real:S38-PARTIAL-A1") {
        comparisons.push(compare_paired(
            "real_process_open:B0",
            "real_process_open:S38-PARTIAL-A1",
            &proc_by_key["real:B0"],
            &proc_by_key["real:S38-PARTIAL-A1"],
            DEFAULT_SEED + 2,
        ));
        comparisons.push(compare_paired(
            "real_warm_reopen:B0",
            "real_warm_reopen:S38-PARTIAL-A1",
            &warm_by_key["real:B0"],
            &warm_by_key["real:S38-PARTIAL-A1"],
            DEFAULT_SEED + 3,
        ));
    }

    Ok((results, comparisons))
}

// ---------------------------------------------------------------------------
// Write suite (fresh/upsert nodes, edges, operational_state checkpoints)
// ---------------------------------------------------------------------------

fn commit_node_batch(
    conn: &mut Connection,
    layout: LayoutId,
    scope: &str,
    cursor_base: &mut i64,
    batch_items: &[(usize, String, String, PropertyMap)],
    is_upsert: bool,
) -> Result<(), String> {
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| format!("begin node tx: {e}"))?;
    // Visibility coalescing arm.
    tx.prepare_cached(
        "UPDATE _fathomdb_read_visibility_state SET coalescing=1 WHERE singleton=1 AND coalescing=0",
    )
    .map_err(|e| format!("arm vis: {e}"))?
    .execute([])
    .map_err(|e| format!("exec arm vis: {e}"))?;

    for (local_idx, (item_id, logical_id, body, props)) in batch_items.iter().enumerate() {
        *cursor_base += 1;
        let cursor = *cursor_base;

        if is_upsert {
            if layout.is_s39() {
                let prior: Option<i64> = tx
                    .prepare_cached(
                        "SELECT write_cursor FROM canonical_nodes
                         WHERE scope = ?1 AND logical_id = ?2 AND superseded_at IS NULL LIMIT 1",
                    )
                    .map_err(|e| format!("prep s39 prior: {e}"))?
                    .query_row(params![scope, logical_id], |r| r.get(0))
                    .optional()
                    .map_err(|e| format!("s39 prior: {e}"))?;
                if let Some(prev) = prior {
                    tx.prepare_cached(
                        "UPDATE canonical_nodes SET superseded_at = ?1
                         WHERE scope = ?2 AND logical_id = ?3 AND superseded_at IS NULL",
                    )
                    .map_err(|e| format!("prep s39 sup: {e}"))?
                    .execute(params![cursor, scope, logical_id])
                    .map_err(|e| format!("s39 sup: {e}"))?;
                    tx.prepare_cached("UPDATE vector_default SET lifecycle = 2 WHERE rowid = ?1")
                        .map_err(|e| format!("prep vec sup: {e}"))?
                        .execute([prev])
                        .map_err(|e| format!("exec vec sup: {e}"))?;
                    tx.prepare_cached("DELETE FROM canonical_attributes WHERE write_cursor = ?1")
                        .map_err(|e| format!("prep del ca: {e}"))?
                        .execute([prev])
                        .map_err(|e| format!("del ca: {e}"))?;
                    tx.prepare_cached(
                        "DELETE FROM canonical_property_values WHERE entity_class='node' AND write_cursor = ?1",
                    )
                    .map_err(|e| format!("prep del cpv: {e}"))?
                    .execute([prev])
                    .map_err(|e| format!("del cpv: {e}"))?;
                }
            } else {
                // B0 and S38-PARTIAL: query prior active row; S38-PARTIAL also checks `scope` ownership!
                let prior: Option<(i64, String)> = tx
                    .prepare_cached(
                        "SELECT write_cursor, scope FROM canonical_nodes
                         WHERE logical_id = ?1 AND superseded_at IS NULL LIMIT 1",
                    )
                    .map_err(|e| format!("prep s38 prior: {e}"))?
                    .query_row([logical_id], |r| Ok((r.get(0)?, r.get(1)?)))
                    .optional()
                    .map_err(|e| format!("s38 prior: {e}"))?;
                if let Some((prev, owner_scope)) = prior {
                    if matches!(layout, LayoutId::S38PartialA1 | LayoutId::S38PartialA2)
                        && owner_scope != scope
                    {
                        return Err(format!(
                            "ScopeViolation: {logical_id} owned by {owner_scope:?}, not {scope:?}"
                        ));
                    }
                    tx.prepare_cached(
                        "UPDATE canonical_nodes SET superseded_at = ?1
                         WHERE logical_id = ?2 AND superseded_at IS NULL",
                    )
                    .map_err(|e| format!("prep s38 sup: {e}"))?
                    .execute(params![cursor, logical_id])
                    .map_err(|e| format!("s38 sup: {e}"))?;
                    tx.prepare_cached("UPDATE vector_default SET lifecycle = 2 WHERE rowid = ?1")
                        .map_err(|e| format!("prep vec sup: {e}"))?
                        .execute([prev])
                        .map_err(|e| format!("exec vec sup: {e}"))?;
                    tx.prepare_cached("DELETE FROM canonical_attributes WHERE write_cursor = ?1")
                        .map_err(|e| format!("prep del ca: {e}"))?
                        .execute([prev])
                        .map_err(|e| format!("del ca: {e}"))?;
                    tx.prepare_cached(
                        "DELETE FROM canonical_property_values WHERE entity_class='node' AND write_cursor = ?1",
                    )
                    .map_err(|e| format!("prep del cpv: {e}"))?
                    .execute([prev])
                    .map_err(|e| format!("del cpv: {e}"))?;
                }
            }
        } else if matches!(layout, LayoutId::S38PartialA1 | LayoutId::S38PartialA2) {
            // Even on fresh inserts with logical_id, S38-PARTIAL checks ownership before insert.
            let owner: Option<String> = tx
                .prepare_cached(
                    "SELECT scope FROM canonical_nodes WHERE logical_id = ?1 AND superseded_at IS NULL LIMIT 1",
                )
                .map_err(|e| format!("prep fresh ownership: {e}"))?
                .query_row([logical_id], |r| r.get(0))
                .optional()
                .map_err(|e| format!("fresh ownership: {e}"))?;
            if let Some(owner_scope) = owner {
                if owner_scope != scope {
                    return Err(format!("ScopeViolation: {logical_id} owned by {owner_scope:?}"));
                }
            }
        }

        let props_json = serde_json::to_string(&serde_json::json!({
            "tier": match &props["tier"] { PropertyValue::String(s) => s.as_str(), _ => "" },
            "topic": match &props["topic"] { PropertyValue::String(s) => s.as_str(), _ => "" },
            "region": match &props["region"] { PropertyValue::String(s) => s.as_str(), _ => "" },
            "env": match &props["env"] { PropertyValue::String(s) => s.as_str(), _ => "" },
            "bucket": match &props["bucket"] { PropertyValue::Int(v) => *v, _ => 0 },
            "priority_label": match &props["priority_label"] { PropertyValue::String(s) => s.as_str(), _ => "" },
            "owner_team": match &props["owner_team"] { PropertyValue::String(s) => s.as_str(), _ => "" },
            "active_flag": match &props["active_flag"] { PropertyValue::Bool(b) => *b, _ => true },
        }))
        .unwrap();

        tx.prepare_cached(
            "INSERT INTO canonical_nodes(
                write_cursor, kind, body, source_id, logical_id, row_kind,
                state, reason, valid_from, valid_until, properties, scope
             ) VALUES(?1, 'note', ?2, ?3, ?4, 'leaf', 'active', NULL, NULL, NULL, ?5, ?6)",
        )
        .map_err(|e| format!("prep ins node: {e}"))?
        .execute(params![
            cursor,
            body,
            format!("src-{}", item_id % 50),
            logical_id,
            props_json,
            scope
        ])
        .map_err(|e| format!("ins node: {e}"))?;

        tx.prepare_cached(
            "INSERT INTO search_index(body, kind, write_cursor) VALUES(?1, 'note', ?2)",
        )
        .map_err(|e| format!("prep fts1: {e}"))?
        .execute(params![body, cursor])
        .map_err(|e| format!("fts1: {e}"))?;
        tx.prepare_cached("INSERT INTO search_index_v2(kind, body, status, write_cursor) VALUES('note', ?1, '', ?2)")
            .map_err(|e| format!("prep fts2: {e}"))?
            .execute(params![body, cursor])
            .map_err(|e| format!("fts2: {e}"))?;

        for (k, v) in props {
            match v {
                PropertyValue::String(s) => {
                    tx.prepare_cached(
                        "INSERT INTO canonical_attributes(write_cursor, attr_name, attr_value) VALUES(?1, ?2, ?3)",
                    )
                    .map_err(|e| format!("prep ca s: {e}"))?
                    .execute(params![cursor, k, s])
                    .map_err(|e| format!("ca s: {e}"))?;
                    tx.prepare_cached(
                        "INSERT INTO canonical_property_values(entity_class, write_cursor, attr_name, value_type, text_val)
                         VALUES('node', ?1, ?2, 'string', ?3)",
                    )
                    .map_err(|e| format!("prep cpv s: {e}"))?
                    .execute(params![cursor, k, s])
                    .map_err(|e| format!("cpv s: {e}"))?;
                }
                PropertyValue::Int(iv) => {
                    tx.prepare_cached(
                        "INSERT INTO canonical_attributes(write_cursor, attr_name, attr_value) VALUES(?1, ?2, ?3)",
                    )
                    .map_err(|e| format!("prep ca i: {e}"))?
                    .execute(params![cursor, k, iv.to_string()])
                    .map_err(|e| format!("ca i: {e}"))?;
                    tx.prepare_cached(
                        "INSERT INTO canonical_property_values(entity_class, write_cursor, attr_name, value_type, num_val)
                         VALUES('node', ?1, ?2, 'integer', ?3)",
                    )
                    .map_err(|e| format!("prep cpv i: {e}"))?
                    .execute(params![cursor, k, *iv])
                    .map_err(|e| format!("cpv i: {e}"))?;
                }
                PropertyValue::Bool(bv) => {
                    tx.prepare_cached(
                        "INSERT INTO canonical_attributes(write_cursor, attr_name, attr_value) VALUES(?1, ?2, ?3)",
                    )
                    .map_err(|e| format!("prep ca b: {e}"))?
                    .execute(params![cursor, k, if *bv { "true" } else { "false" }])
                    .map_err(|e| format!("ca b: {e}"))?;
                    tx.prepare_cached(
                        "INSERT INTO canonical_property_values(entity_class, write_cursor, attr_name, value_type, bool_val)
                         VALUES('node', ?1, ?2, 'boolean', ?3)",
                    )
                    .map_err(|e| format!("prep cpv b: {e}"))?
                    .execute(params![cursor, k, i64::from(*bv)])
                    .map_err(|e| format!("cpv b: {e}"))?;
                }
                PropertyValue::Float(fv) => {
                    tx.prepare_cached(
                        "INSERT INTO canonical_property_values(entity_class, write_cursor, attr_name, value_type, num_val)
                         VALUES('node', ?1, ?2, 'float', ?3)",
                    )
                    .map_err(|e| format!("prep cpv f: {e}"))?
                    .execute(params![cursor, k, *fv])
                    .map_err(|e| format!("cpv f: {e}"))?;
                }
            }
        }
        let rev_id = format!("_fdb:r:{cursor}_{local_idx}");
        tx.prepare_cached(
            "INSERT INTO _fathomdb_artifact_revisions(schema_version, revision_id, artifact_class, write_cursor, artifact_role, completeness)
             VALUES(1, ?1, 'node', ?2, 'legacy', 'migrated_incomplete')",
        )
        .map_err(|e| format!("prep rev: {e}"))?
        .execute(params![rev_id, cursor])
        .map_err(|e| format!("rev: {e}"))?;
    }

    tx.prepare_cached(
        "INSERT INTO _fathomdb_commit_timestamps(write_cursor, committed_at_ms) VALUES(?1, ?2)",
    )
    .map_err(|e| format!("prep ts: {e}"))?
    .execute(params![*cursor_base, 1_795_500_000_000_i64 + *cursor_base])
    .map_err(|e| format!("ts: {e}"))?;
    tx.prepare_cached(
        "UPDATE _fathomdb_read_visibility_state
         SET generation=generation+1, state_nonce=lower(hex(randomblob(32))), coalescing=0
         WHERE singleton=1",
    )
    .map_err(|e| format!("prep vis adv: {e}"))?
    .execute([])
    .map_err(|e| format!("vis adv: {e}"))?;
    tx.commit().map_err(|e| format!("commit node tx: {e}"))?;
    Ok(())
}

fn commit_edge_batch(
    conn: &mut Connection,
    layout: LayoutId,
    scope: &str,
    cursor_base: &mut i64,
    edges: &[(usize, String, String, String)],
) -> Result<(), String> {
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| format!("begin edge tx: {e}"))?;
    tx.prepare_cached(
        "UPDATE _fathomdb_read_visibility_state SET coalescing=1 WHERE singleton=1 AND coalescing=0",
    )
    .map_err(|e| format!("arm vis: {e}"))?
    .execute([])
    .map_err(|e| format!("exec arm vis: {e}"))?;

    let mut distinct_endpoints: Vec<&str> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    for (_, _, from_id, to_id) in edges {
        if seen.insert(from_id.as_str()) {
            distinct_endpoints.push(from_id.as_str());
        }
        if seen.insert(to_id.as_str()) {
            distinct_endpoints.push(to_id.as_str());
        }
    }

    // Strict endpoint validation in chunks of 64 (mirrors `write_commit.rs:1043-1072`).
    for chunk in distinct_endpoints.chunks(64) {
        let mut placeholders = String::new();
        for idx in 0..chunk.len() {
            if idx > 0 {
                placeholders.push(',');
            }
            placeholders.push_str(&format!("?{}", idx + 1));
        }
        let (sql, extra_scope) = match layout {
            LayoutId::B0 => (
                format!(
                    "SELECT COUNT(*) FROM canonical_nodes
                     WHERE logical_id IN ({placeholders}) AND superseded_at IS NULL AND state = 'active'"
                ),
                false,
            ),
            LayoutId::S38PartialA1 | LayoutId::S38PartialA2 => {
                let scope_idx = chunk.len() + 1;
                (
                    format!(
                        "SELECT COUNT(*) FROM canonical_nodes
                         WHERE logical_id IN ({placeholders}) AND superseded_at IS NULL AND state = 'active' AND scope = ?{scope_idx}"
                    ),
                    true,
                )
            }
            LayoutId::S39Composite => {
                let scope_idx = chunk.len() + 1;
                (
                    format!(
                        "SELECT COUNT(*) FROM canonical_nodes
                         WHERE scope = ?{scope_idx} AND logical_id IN ({placeholders}) AND superseded_at IS NULL AND state = 'active'"
                    ),
                    true,
                )
            }
        };
        let mut bind_vals: Vec<rusqlite::types::Value> =
            chunk.iter().map(|&s| rusqlite::types::Value::Text(s.to_string())).collect();
        if extra_scope {
            bind_vals.push(rusqlite::types::Value::Text(scope.to_string()));
        }
        let count: i64 = tx
            .prepare_cached(&sql)
            .map_err(|e| format!("prep endpoint check: {e}"))?
            .query_row(rusqlite::params_from_iter(bind_vals.iter()), |r| r.get(0))
            .map_err(|e| format!("exec endpoint check: {e}"))?;
        if count as usize != chunk.len() {
            return Err(format!("WriteValidation: endpoint mismatch {count} != {}", chunk.len()));
        }
    }

    for (local_idx, (eid, logical_id, from_id, to_id)) in edges.iter().enumerate() {
        *cursor_base += 1;
        let cursor = *cursor_base;
        if matches!(layout, LayoutId::S38PartialA1 | LayoutId::S38PartialA2) {
            let owner: Option<String> = tx
                .prepare_cached(
                    "SELECT scope FROM canonical_edges WHERE logical_id = ?1 AND superseded_at IS NULL LIMIT 1",
                )
                .map_err(|e| format!("prep edge own: {e}"))?
                .query_row([logical_id], |r| r.get(0))
                .optional()
                .map_err(|e| format!("edge own: {e}"))?;
            if let Some(os) = owner {
                if os != scope {
                    return Err(format!("ScopeViolation edge {logical_id}"));
                }
            }
        }
        tx.prepare_cached(
            "INSERT INTO canonical_edges(
                write_cursor, kind, from_id, to_id, source_id, logical_id,
                body, t_valid, t_invalid, confidence, extractor_model_id, temporal_fallback, properties, scope
             ) VALUES(?1, 'relates_to', ?2, ?3, ?4, ?5, NULL, NULL, NULL, 0.9, NULL, NULL, '{\"edge_tag\":\"hot\"}', ?6)",
        )
        .map_err(|e| format!("prep ins edge: {e}"))?
        .execute(params![
            cursor,
            from_id,
            to_id,
            format!("src-{}", eid % 50),
            logical_id,
            scope
        ])
        .map_err(|e| format!("ins edge: {e}"))?;

        let rev_id = format!("_fdb:r:e:{cursor}_{local_idx}");
        tx.prepare_cached(
            "INSERT INTO _fathomdb_artifact_revisions(schema_version, revision_id, artifact_class, write_cursor, artifact_role, completeness)
             VALUES(1, ?1, 'edge', ?2, 'legacy', 'migrated_incomplete')",
        )
        .map_err(|e| format!("prep edge rev: {e}"))?
        .execute(params![rev_id, cursor])
        .map_err(|e| format!("edge rev: {e}"))?;
    }

    tx.prepare_cached(
        "INSERT INTO _fathomdb_commit_timestamps(write_cursor, committed_at_ms) VALUES(?1, ?2)",
    )
    .map_err(|e| format!("prep ts: {e}"))?
    .execute(params![*cursor_base, 1_795_500_000_000_i64 + *cursor_base])
    .map_err(|e| format!("ts: {e}"))?;
    tx.prepare_cached(
        "UPDATE _fathomdb_read_visibility_state
         SET generation=generation+1, state_nonce=lower(hex(randomblob(32))), coalescing=0
         WHERE singleton=1",
    )
    .map_err(|e| format!("prep vis adv: {e}"))?
    .execute([])
    .map_err(|e| format!("vis adv: {e}"))?;
    tx.commit().map_err(|e| format!("commit edge tx: {e}"))?;
    Ok(())
}

fn commit_single_checkpoint(
    conn: &mut Connection,
    layout: LayoutId,
    scope: &str,
    collection: &str,
    record_key: &str,
    payload: &str,
    cursor_base: &mut i64,
) -> Result<(), String> {
    *cursor_base += 1;
    let cursor = *cursor_base;
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| format!("begin cp tx: {e}"))?;
    match layout {
        LayoutId::B0 => {
            tx.prepare_cached(
                "INSERT INTO operational_state(collection_name, record_key, payload_json, schema_id, write_cursor, scope)
                 VALUES(?1, ?2, ?3, 'checkpoint.v1', ?4, '')
                 ON CONFLICT(collection_name, record_key) DO UPDATE SET
                    payload_json = excluded.payload_json,
                    schema_id = excluded.schema_id,
                    write_cursor = excluded.write_cursor",
            )
            .map_err(|e| format!("prep b0 cp: {e}"))?
            .execute(params![collection, record_key, payload, cursor])
            .map_err(|e| format!("exec b0 cp: {e}"))?;
        }
        LayoutId::S38PartialA1 | LayoutId::S38PartialA2 => {
            let existing_scope: Option<String> = tx
                .prepare_cached(
                    "SELECT scope FROM operational_state WHERE collection_name = ?1 AND record_key = ?2 LIMIT 1",
                )
                .map_err(|e| format!("prep s38 cp check: {e}"))?
                .query_row(params![collection, record_key], |r| r.get(0))
                .optional()
                .map_err(|e| format!("exec s38 cp check: {e}"))?;
            if let Some(owner) = existing_scope {
                if owner != scope {
                    return Err(format!(
                        "ScopeViolation checkpoint ({collection}, {record_key}) owned by {owner:?}"
                    ));
                }
            }
            tx.prepare_cached(
                "INSERT INTO operational_state(collection_name, record_key, payload_json, schema_id, write_cursor, scope)
                 VALUES(?1, ?2, ?3, 'checkpoint.v1', ?4, ?5)
                 ON CONFLICT(collection_name, record_key) DO UPDATE SET
                    payload_json = excluded.payload_json,
                    schema_id = excluded.schema_id,
                    write_cursor = excluded.write_cursor,
                    scope = excluded.scope",
            )
            .map_err(|e| format!("prep s38 cp ins: {e}"))?
            .execute(params![collection, record_key, payload, cursor, scope])
            .map_err(|e| format!("exec s38 cp ins: {e}"))?;
        }
        LayoutId::S39Composite => {
            tx.prepare_cached(
                "INSERT INTO operational_state(scope, collection_name, record_key, payload_json, schema_id, write_cursor)
                 VALUES(?1, ?2, ?3, ?4, 'checkpoint.v1', ?5)
                 ON CONFLICT(scope, collection_name, record_key) DO UPDATE SET
                    payload_json = excluded.payload_json,
                    schema_id = excluded.schema_id,
                    write_cursor = excluded.write_cursor",
            )
            .map_err(|e| format!("prep s39 cp ins: {e}"))?
            .execute(params![scope, collection, record_key, payload, cursor])
            .map_err(|e| format!("exec s39 cp ins: {e}"))?;
        }
    }
    tx.prepare_cached(
        "INSERT OR REPLACE INTO _fathomdb_projection_terminal(write_cursor, state) VALUES(?1, 'up_to_date')",
    )
    .map_err(|e| format!("prep cp term: {e}"))?
    .execute([cursor])
    .map_err(|e| format!("exec cp term: {e}"))?;
    tx.prepare_cached(
        "INSERT INTO _fathomdb_commit_timestamps(write_cursor, committed_at_ms) VALUES(?1, ?2)",
    )
    .map_err(|e| format!("prep cp ts: {e}"))?
    .execute(params![cursor, 1_795_500_000_000_i64 + cursor])
    .map_err(|e| format!("exec cp ts: {e}"))?;
    tx.commit().map_err(|e| format!("commit cp tx: {e}"))?;
    Ok(())
}

fn verify_ownership_guard(
    conn: &mut Connection,
    layout: LayoutId,
    profile: ProfileKind,
    num_nodes: usize,
    cursor_base: &mut i64,
) -> Result<bool, String> {
    if profile == ProfileKind::SingleScope || layout == LayoutId::B0 {
        return Ok(true);
    }
    let callers = profile.callers();
    let owner_scope = &callers[0];
    let attacker_scope = &callers[1];
    let target_nid = 0_usize;
    let cp_col = OP_COLLECTIONS[0];
    let cp_key = "cp:0";
    // Find actual owner of node:0 and cp:0.
    let actual_node_owner = scope_for_node_scaled(profile, target_nid, num_nodes);
    let _ = owner_scope;
    let node_item = vec![(
        9_999_999,
        format!("node:{target_nid}"),
        "spoofed node body".to_string(),
        node_properties(target_nid),
    )];
    let node_res = commit_node_batch(conn, layout, attacker_scope, cursor_base, &node_item, true);
    let cp_res = commit_single_checkpoint(
        conn,
        layout,
        attacker_scope,
        cp_col,
        cp_key,
        r#"{"spoof":true}"#,
        cursor_base,
    );
    match layout {
        LayoutId::S38PartialA1 | LayoutId::S38PartialA2 => {
            if actual_node_owner == *attacker_scope {
                return Ok(false);
            }
            Ok(node_res.is_err() && cp_res.is_err())
        }
        LayoutId::S39Composite => {
            // In S39, attacker gets its own isolated record without overwriting owner!
            let owner_body: String = conn
                .query_row(
                    "SELECT body FROM canonical_nodes WHERE scope = ?1 AND logical_id = ?2 AND superseded_at IS NULL",
                    params![actual_node_owner, format!("node:{target_nid}")],
                    |r| r.get(0),
                )
                .map_err(|e| format!("verify s39 owner intact: {e}"))?;
            Ok(node_res.is_ok() && cp_res.is_ok() && !owner_body.contains("spoofed"))
        }
        LayoutId::B0 => Ok(true),
    }
}

// ---------------------------------------------------------------------------
// Read / Search / Graph / OpState / Recall@10 / EXPLAIN QUERY PLAN suite
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct ReadTrialSpec {
    scope: String,
    use_scope_predicate: bool,
    fts_query: String,
    query_vec_blob: Vec<u8>,
    seed_logical_id: String,
    op_collection: String,
    op_record_key: String,
    op_page_after: i64,
}

fn build_read_trial_specs(
    profile: ProfileKind,
    cohort: &str,
    num_nodes: usize,
    num_op_state: usize,
    emb_gen: &EmbeddingGenerator,
) -> Vec<ReadTrialSpec> {
    let total = WARMUP_ROUNDS + READ_TRIALS;
    let mut specs = Vec::with_capacity(total);
    for t in 0..total {
        let (scope, use_scope_predicate, seed_idx) = match (profile, cohort) {
            (ProfileKind::SingleScope, "unscoped_fast_path") => {
                (String::new(), false, (t * 499) % num_nodes)
            }
            (ProfileKind::SingleScope, "scoped_empty_explicit") => {
                (String::new(), true, (t * 499) % num_nodes)
            }
            (ProfileKind::MultiScopeUniform, _) => {
                let c = t % NUM_CALLERS;
                let block = (num_nodes / NUM_CALLERS).max(1);
                let idx = c * block + ((t * 97) % block);
                (format!("tenant_{c}"), true, idx)
            }
            (ProfileKind::MultiScopeSkewed, "heavy_90pct") => {
                let heavy = (num_nodes * 9) / 10;
                let idx = (t * 439) % heavy.max(1);
                ("heavy_0".to_string(), true, idx)
            }
            (ProfileKind::MultiScopeSkewed, "light_1pct") => {
                let c = 1 + (t % (NUM_CALLERS - 1));
                let heavy = (num_nodes * 9) / 10;
                let rem_total = (num_nodes - heavy).max(1);
                let per = (rem_total / (NUM_CALLERS - 1)).max(1);
                let approx_idx = heavy + (c - 1) * per + ((t * 37) % per);
                let (s_start, s_end) =
                    scope_node_bounds(profile, approx_idx.min(num_nodes - 1), num_nodes);
                let span = (s_end - s_start).max(1);
                let idx = s_start + ((t * 53) % span);
                (format!("light_{c}"), true, idx)
            }
            _ => (String::new(), false, t % num_nodes),
        };
        let topic = TOPICS[t % TOPICS.len()];
        let signal = t % 19;
        let fts_query = format!("\"{topic}\" AND \"metric_{signal}\"");
        let query_vec_blob = encode_f32_blob(&emb_gen.query_vector(t));
        let op_idx =
            ((seed_idx * num_op_state) / num_nodes.max(1)).min(num_op_state.saturating_sub(1));
        let op_collection = OP_COLLECTIONS[op_idx % OP_COLLECTIONS.len()].to_string();
        let op_record_key = format!("cp:{op_idx}");

        specs.push(ReadTrialSpec {
            scope,
            use_scope_predicate,
            fts_query,
            query_vec_blob,
            seed_logical_id: format!("node:{seed_idx}"),
            op_collection,
            op_record_key,
            op_page_after: 0,
        });
    }
    specs
}

fn effective_spec_for_layout(
    layout: LayoutId,
    spec: &ReadTrialSpec,
) -> std::borrow::Cow<'_, ReadTrialSpec> {
    if layout.is_s39() && !spec.use_scope_predicate {
        // Schema 39 primary keys and unique indexes lead with `scope`; single-scope calls
        // natively bind `scope = ''` rather than omitting `scope`.
        let mut owned = spec.clone();
        owned.use_scope_predicate = true;
        std::borrow::Cow::Owned(owned)
    } else {
        std::borrow::Cow::Borrowed(spec)
    }
}

fn scope_pred_clause(
    layout: LayoutId,
    spec: &ReadTrialSpec,
    qualified_col: &str,
    param_idx: usize,
) -> String {
    if !spec.use_scope_predicate {
        return String::new();
    }
    if matches!(layout, LayoutId::S38PartialA1 | LayoutId::S38PartialA2) && !spec.scope.is_empty() {
        // SQLite cannot statically prove `? != ''` for partial indexes `WHERE scope != ''`;
        // emitting explicit `col != ''` lets the planner select the partial indexes on S38.
        format!(" AND {qualified_col} = ?{param_idx} AND {qualified_col} != ''")
    } else {
        format!(" AND {qualified_col} = ?{param_idx}")
    }
}

fn exec_search_text_only(
    conn: &Connection,
    layout: LayoutId,
    spec: &ReadTrialSpec,
) -> Result<Vec<(i64, String, f64)>, String> {
    let spec_eff = effective_spec_for_layout(layout, spec);
    let spec = spec_eff.as_ref();
    // Uses FTS5 rank stream (`AND rank MATCH 'bm25()' ORDER BY rank`) with CROSS JOIN
    // so SQLite keeps FTS5 as the driving table and avoids `USE TEMP B-TREE FOR ORDER BY`
    // even when a light scope (1.1%) is more selective than a frequent FTS term.
    let scope_clause = scope_pred_clause(layout, spec, "cn.scope", 2);
    let join_kind = if spec.use_scope_predicate { "CROSS JOIN" } else { "LEFT JOIN" };
    let ownerless_clause = if spec.use_scope_predicate {
        "AND cn.superseded_at IS NULL AND cn.state = 'active'"
    } else {
        "AND (cn.write_cursor IS NULL OR (cn.superseded_at IS NULL AND cn.state = 'active'))"
    };
    let sql = format!(
        "SELECT si.write_cursor, cn.logical_id, bm25(search_index)
         FROM search_index si
         {join_kind} canonical_nodes cn ON cn.write_cursor = si.write_cursor
         WHERE search_index MATCH ?1
           {ownerless_clause}{scope_clause}
           AND rank MATCH 'bm25()'
         ORDER BY rank"
    );
    let mut stmt = conn.prepare_cached(&sql).map_err(|e| format!("prep text: {e}"))?;
    let mut params_v = vec![rusqlite::types::Value::Text(spec.fts_query.clone())];
    if spec.use_scope_predicate {
        params_v.push(rusqlite::types::Value::Text(spec.scope.clone()));
    }
    let mut rows = stmt
        .query(rusqlite::params_from_iter(params_v.iter()))
        .map_err(|e| format!("query text: {e}"))?;
    let mut out = Vec::with_capacity(TOP_K_RESULTS);
    let mut boundary_score: Option<f64> = None;
    while let Some(row) = rows.next().map_err(|e| format!("next text: {e}"))? {
        let wc: i64 = row.get(0).map_err(|e| format!("wc: {e}"))?;
        let lid: String =
            row.get::<_, Option<String>>(1).map_err(|e| format!("lid: {e}"))?.unwrap_or_default();
        let score: f64 = row.get(2).map_err(|e| format!("bm25: {e}"))?;
        if out.len() < TOP_K_RESULTS {
            if out.len() + 1 == TOP_K_RESULTS {
                boundary_score = Some(score);
            }
            out.push((wc, lid, -score));
        } else if boundary_score.is_some_and(|b| b.total_cmp(&score).is_eq()) {
            out.push((wc, lid, -score));
        } else {
            break;
        }
    }
    out.sort_by(|a, b| b.2.total_cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
    out.truncate(TOP_K_RESULTS);
    Ok(out)
}

fn exec_search_vector_k10(
    conn: &Connection,
    layout: LayoutId,
    spec: &ReadTrialSpec,
) -> Result<Vec<(i64, f64)>, String> {
    let spec_eff = effective_spec_for_layout(layout, spec);
    let spec = spec_eff.as_ref();
    let mut candidates: Vec<(i64, f64)> = Vec::with_capacity(TOP_K_RESULTS);
    if !spec.use_scope_predicate {
        // Zero-overhead unscoped bit-KNN + L2 rerank.
        let sql = format!(
            "WITH candidates AS (
                SELECT rowid, distance
                FROM vector_default
                WHERE embedding_bin MATCH vec_quantize_binary(?1)
                  AND lifecycle = 0
                  AND valid_from <= ?2 AND valid_until >= ?2
                ORDER BY distance LIMIT {TOP_K_BIT_CANDIDATES}
             )
             SELECT c.rowid, vec_distance_l2(v.embedding, ?1) AS l2
             FROM candidates c JOIN vector_default v ON v.rowid = c.rowid
             ORDER BY l2 LIMIT {TOP_K_RESULTS}"
        );
        let mut stmt = conn.prepare_cached(&sql).map_err(|e| format!("prep vec0 fast: {e}"))?;
        let rows = stmt
            .query_map(params![spec.query_vec_blob, 1_800_000_000_i64], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?))
            })
            .map_err(|e| format!("query vec0 fast: {e}"))?;
        for r in rows {
            candidates.push(r.map_err(|e| format!("row vec0 fast: {e}"))?);
        }
    } else if !layout.has_vec0_scope_col() {
        // A1: Lazy 7-col vec0 pre-selection (`filter.rs:1081-1089`).
        let scope_clause = scope_pred_clause(layout, spec, "cn.scope", 2);
        let sql = format!(
            "WITH candidates AS (
                SELECT cn.write_cursor AS rowid
                FROM canonical_nodes cn
                WHERE cn.superseded_at IS NULL
                  AND cn.state = 'active'{scope_clause}
                  AND EXISTS(SELECT 1 FROM _fathomdb_vector_rows vr WHERE vr.write_cursor = cn.write_cursor)
             )
             SELECT c.rowid, vec_distance_l2(v.embedding, ?1) AS l2
             FROM candidates c JOIN vector_default v ON v.rowid = c.rowid
             WHERE v.lifecycle = 0 AND v.valid_from <= ?3 AND v.valid_until >= ?3
             ORDER BY l2 LIMIT {TOP_K_RESULTS}"
        );
        let mut stmt = conn.prepare_cached(&sql).map_err(|e| format!("prep a1: {e}"))?;
        let rows = stmt
            .query_map(params![spec.query_vec_blob, spec.scope, 1_800_000_000_i64], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?))
            })
            .map_err(|e| format!("query a1: {e}"))?;
        for r in rows {
            candidates.push(r.map_err(|e| format!("row a1: {e}"))?);
        }
    } else {
        // A2 & B: Native 8-col `vec0` with `scope = ?` pushed pre-KNN!
        let sql = format!(
            "WITH candidates AS (
                SELECT rowid, distance
                FROM vector_default
                WHERE embedding_bin MATCH vec_quantize_binary(?1)
                  AND scope = ?2
                  AND lifecycle = 0
                  AND valid_from <= ?3 AND valid_until >= ?3
                ORDER BY distance LIMIT {TOP_K_BIT_CANDIDATES}
             )
             SELECT c.rowid, vec_distance_l2(v.embedding, ?1) AS l2
             FROM candidates c JOIN vector_default v ON v.rowid = c.rowid
             ORDER BY l2 LIMIT {TOP_K_RESULTS}"
        );
        let mut stmt = conn.prepare_cached(&sql).map_err(|e| format!("prep 8col vec0: {e}"))?;
        let rows = stmt
            .query_map(params![spec.query_vec_blob, spec.scope, 1_800_000_000_i64], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?))
            })
            .map_err(|e| format!("query 8col vec0: {e}"))?;
        for r in rows {
            candidates.push(r.map_err(|e| format!("row 8col vec0: {e}"))?);
        }
    }

    // Hydrate top-k from canonical_nodes (mirrors `search.rs:2528-2590`).
    let scope_hydr = if spec.use_scope_predicate { " AND scope = ?2" } else { "" };
    let hydr_sql = format!(
        "SELECT kind, body, logical_id FROM canonical_nodes
         WHERE write_cursor = ?1 AND superseded_at IS NULL AND state = 'active'{scope_hydr} LIMIT 1"
    );
    let mut hydr_stmt = conn.prepare_cached(&hydr_sql).map_err(|e| format!("prep hydr: {e}"))?;
    let mut out = Vec::with_capacity(candidates.len());
    for (rowid, dist) in candidates {
        let ok: Option<String> = if spec.use_scope_predicate {
            hydr_stmt
                .query_row(params![rowid, spec.scope], |r| r.get::<_, Option<String>>(2))
                .optional()
                .map_err(|e| format!("hydr row: {e}"))?
                .flatten()
        } else {
            hydr_stmt
                .query_row([rowid], |r| r.get::<_, Option<String>>(2))
                .optional()
                .map_err(|e| format!("hydr row: {e}"))?
                .flatten()
        };
        if ok.is_some() {
            out.push((rowid, dist));
        }
    }
    Ok(out)
}

fn exact_l2_ground_truth(
    conn: &Connection,
    layout: LayoutId,
    spec: &ReadTrialSpec,
) -> Result<Vec<i64>, String> {
    let scope_clause = scope_pred_clause(layout, spec, "cn.scope", 2);
    let sql = format!(
        "SELECT cn.write_cursor, vec_distance_l2(v.embedding, ?1) AS l2
         FROM canonical_nodes cn
         JOIN vector_default v ON v.rowid = cn.write_cursor
         WHERE cn.superseded_at IS NULL AND cn.state = 'active'{scope_clause}
         ORDER BY l2 ASC, cn.write_cursor ASC LIMIT {TOP_K_RESULTS}"
    );
    let mut stmt = conn.prepare_cached(&sql).map_err(|e| format!("prep gt: {e}"))?;
    let mut params_v = vec![rusqlite::types::Value::Blob(spec.query_vec_blob.clone())];
    if spec.use_scope_predicate {
        params_v.push(rusqlite::types::Value::Text(spec.scope.clone()));
    }
    let rows = stmt
        .query_map(rusqlite::params_from_iter(params_v.iter()), |r| r.get::<_, i64>(0))
        .map_err(|e| format!("query gt: {e}"))?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| format!("collect gt: {e}"))
}

fn exec_search_hybrid(
    conn: &Connection,
    layout: LayoutId,
    spec: &ReadTrialSpec,
) -> Result<Vec<i64>, String> {
    let vec_hits = exec_search_vector_k10(conn, layout, spec)?;
    let text_hits = exec_search_text_only(conn, layout, spec)?;
    // Simplified equal-weight RRF (K = 60.0) benchmark fusion (production fusion.rs uses RRF_K = 30.0, 3:1 weights).
    let mut scores: HashMap<i64, f64> = HashMap::new();
    for (rank, (wc, _)) in vec_hits.iter().enumerate() {
        *scores.entry(*wc).or_default() += 1.0 / (60.0 + (rank as f64 + 1.0));
    }
    for (rank, (wc, _, _)) in text_hits.iter().enumerate() {
        *scores.entry(*wc).or_default() += 1.0 / (60.0 + (rank as f64 + 1.0));
    }
    let mut ranked: Vec<(i64, f64)> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked.truncate(TOP_K_RESULTS);
    Ok(ranked.into_iter().map(|(wc, _)| wc).collect())
}

fn exec_graph_expand_2hop(
    conn: &Connection,
    layout: LayoutId,
    spec: &ReadTrialSpec,
) -> Result<Vec<String>, String> {
    let spec_eff = effective_spec_for_layout(layout, spec);
    let spec = spec_eff.as_ref();
    let scope_node_clause = scope_pred_clause(layout, spec, "scope", 2);
    let scope_edge_clause = scope_pred_clause(layout, spec, "canonical_edges.scope", 2);

    let node_sql = format!(
        "SELECT kind, body, write_cursor FROM canonical_nodes
         WHERE logical_id = ?1 AND superseded_at IS NULL AND state = 'active'{scope_node_clause}
         LIMIT 1"
    );
    let edge_sql = format!(
        "SELECT canonical_edges.write_cursor, canonical_edges.kind, canonical_edges.to_id
         FROM canonical_edges
         WHERE canonical_edges.from_id = ?1
           AND canonical_edges.superseded_at IS NULL
           AND canonical_edges.kind IN ('relates_to', 'verifies')
           {scope_edge_clause}
         LIMIT 50"
    );

    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(spec.seed_logical_id.clone());

    let seed_ok: Option<i64> = if spec.use_scope_predicate {
        conn.prepare_cached(&node_sql)
            .map_err(|e| format!("prep seed: {e}"))?
            .query_row(params![spec.seed_logical_id, spec.scope], |r| r.get(2))
            .optional()
            .map_err(|e| format!("seed: {e}"))?
    } else {
        conn.prepare_cached(&node_sql)
            .map_err(|e| format!("prep seed: {e}"))?
            .query_row([&spec.seed_logical_id], |r| r.get(2))
            .optional()
            .map_err(|e| format!("seed: {e}"))?
    };
    if seed_ok.is_none() {
        return Ok(Vec::new());
    }

    let mut frontier = vec![spec.seed_logical_id.clone()];
    let mut targets = Vec::new();
    for _depth in 0..2 {
        let mut next_frontier = Vec::new();
        for current in &frontier {
            let to_ids: Vec<String> = {
                let mut estmt =
                    conn.prepare_cached(&edge_sql).map_err(|e| format!("prep edge: {e}"))?;
                if spec.use_scope_predicate {
                    let rows = estmt
                        .query_map(params![current, spec.scope], |r| r.get::<_, String>(2))
                        .map_err(|e| format!("query edge: {e}"))?;
                    rows.collect::<rusqlite::Result<Vec<_>>>()
                        .map_err(|e| format!("collect edge: {e}"))?
                } else {
                    let rows = estmt
                        .query_map([current], |r| r.get::<_, String>(2))
                        .map_err(|e| format!("query edge: {e}"))?;
                    rows.collect::<rusqlite::Result<Vec<_>>>()
                        .map_err(|e| format!("collect edge: {e}"))?
                }
            };
            for to_id in to_ids {
                if !visited.insert(to_id.clone()) {
                    continue;
                }
                let node_exists: Option<i64> = if spec.use_scope_predicate {
                    conn.prepare_cached(&node_sql)
                        .map_err(|e| format!("prep target: {e}"))?
                        .query_row(params![to_id, spec.scope], |r| r.get(2))
                        .optional()
                        .map_err(|e| format!("target: {e}"))?
                } else {
                    conn.prepare_cached(&node_sql)
                        .map_err(|e| format!("prep target: {e}"))?
                        .query_row([&to_id], |r| r.get(2))
                        .optional()
                        .map_err(|e| format!("target: {e}"))?
                };
                if node_exists.is_some() {
                    targets.push(to_id.clone());
                    next_frontier.push(to_id);
                    if targets.len() >= 20 {
                        return Ok(targets);
                    }
                }
            }
        }
        frontier = next_frontier;
        if frontier.is_empty() {
            break;
        }
    }
    Ok(targets)
}

fn exec_op_state_point_read(
    conn: &Connection,
    layout: LayoutId,
    spec: &ReadTrialSpec,
) -> Result<Option<i64>, String> {
    let spec_eff = effective_spec_for_layout(layout, spec);
    let spec = spec_eff.as_ref();
    let scope_clause = scope_pred_clause(layout, spec, "scope", 3);
    let sql = format!(
        "SELECT write_cursor FROM operational_state WHERE collection_name = ?1 AND record_key = ?2{scope_clause}"
    );
    let mut stmt = conn.prepare_cached(&sql).map_err(|e| format!("prep op point: {e}"))?;
    if spec.use_scope_predicate {
        stmt.query_row(params![spec.op_collection, spec.op_record_key, spec.scope], |r| r.get(0))
            .optional()
            .map_err(|e| format!("exec op point: {e}"))
    } else {
        stmt.query_row(params![spec.op_collection, spec.op_record_key], |r| r.get(0))
            .optional()
            .map_err(|e| format!("exec op point: {e}"))
    }
}

fn exec_op_state_page(
    conn: &Connection,
    layout: LayoutId,
    spec: &ReadTrialSpec,
) -> Result<Vec<i64>, String> {
    let spec_eff = effective_spec_for_layout(layout, spec);
    let spec = spec_eff.as_ref();
    let scope_clause = scope_pred_clause(layout, spec, "scope", 4);
    let sql = format!(
        "SELECT write_cursor FROM operational_state
         WHERE collection_name = ?1 AND write_cursor > ?2{scope_clause}
         ORDER BY write_cursor ASC LIMIT ?3"
    );
    let mut stmt = conn.prepare_cached(&sql).map_err(|e| format!("prep op page: {e}"))?;
    if spec.use_scope_predicate {
        let rows = stmt
            .query_map(
                params![spec.op_collection, spec.op_page_after, PAGE_LIMIT as i64, spec.scope],
                |r| r.get::<_, i64>(0),
            )
            .map_err(|e| format!("exec op page: {e}"))?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| format!("collect op page: {e}"))
    } else {
        let rows = stmt
            .query_map(params![spec.op_collection, spec.op_page_after, PAGE_LIMIT as i64], |r| {
                r.get::<_, i64>(0)
            })
            .map_err(|e| format!("exec op page: {e}"))?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| format!("collect op page: {e}"))
    }
}

fn capture_explain_plans(
    conn: &Connection,
    layout: LayoutId,
    cohort: &str,
    spec: &ReadTrialSpec,
) -> Result<Vec<ExplainPlanAudit>, String> {
    let spec_eff = effective_spec_for_layout(layout, spec);
    let spec = spec_eff.as_ref();
    let mut plans = Vec::new();
    let tag = format!("{}:{}", layout.as_str(), cohort);

    // 1. search_text_only (FTS5 + scope join)
    {
        let scope_clause = scope_pred_clause(layout, spec, "cn.scope", 2);
        let join_kind = if spec.use_scope_predicate { "CROSS JOIN" } else { "LEFT JOIN" };
        let ownerless = if spec.use_scope_predicate {
            "AND cn.superseded_at IS NULL AND cn.state = 'active'"
        } else {
            "AND (cn.write_cursor IS NULL OR (cn.superseded_at IS NULL AND cn.state = 'active'))"
        };
        let sql = format!(
            "SELECT si.write_cursor, cn.logical_id, bm25(search_index)
             FROM search_index si
             {join_kind} canonical_nodes cn ON cn.write_cursor = si.write_cursor
             WHERE search_index MATCH ?1 {ownerless}{scope_clause} AND rank MATCH 'bm25()'
             ORDER BY rank"
        );
        let mut p = vec![rusqlite::types::Value::Text(spec.fts_query.clone())];
        if spec.use_scope_predicate {
            p.push(rusqlite::types::Value::Text(spec.scope.clone()));
        }
        plans.push(
            explain_sql(conn, &format!("{tag}/search_text_only"), &sql, &p)
                .map_err(|e| format!("eqp text: {e}"))?,
        );
    }

    // 2. search_vector_k10
    {
        if !spec.use_scope_predicate {
            let sql = format!(
                "WITH candidates AS (
                    SELECT rowid, distance FROM vector_default
                    WHERE embedding_bin MATCH vec_quantize_binary(?1)
                      AND lifecycle = 0 AND valid_from <= ?2 AND valid_until >= ?2
                    ORDER BY distance LIMIT {TOP_K_BIT_CANDIDATES}
                 )
                 SELECT c.rowid, vec_distance_l2(v.embedding, ?1) AS l2
                 FROM candidates c JOIN vector_default v ON v.rowid = c.rowid
                 ORDER BY l2 LIMIT {TOP_K_RESULTS}"
            );
            let p = vec![
                rusqlite::types::Value::Blob(spec.query_vec_blob.clone()),
                rusqlite::types::Value::Integer(1_800_000_000),
            ];
            plans.push(
                explain_sql(conn, &format!("{tag}/search_vector_k10"), &sql, &p)
                    .map_err(|e| format!("eqp vec fast: {e}"))?,
            );
        } else if !layout.has_vec0_scope_col() {
            let scope_clause = scope_pred_clause(layout, spec, "cn.scope", 2);
            let sql = format!(
                "WITH candidates AS (
                    SELECT cn.write_cursor AS rowid
                    FROM canonical_nodes cn
                    WHERE cn.superseded_at IS NULL AND cn.state = 'active'{scope_clause}
                      AND EXISTS(SELECT 1 FROM _fathomdb_vector_rows vr WHERE vr.write_cursor = cn.write_cursor)
                 )
                 SELECT c.rowid, vec_distance_l2(v.embedding, ?1) AS l2
                 FROM candidates c JOIN vector_default v ON v.rowid = c.rowid
                 WHERE v.lifecycle = 0 AND v.valid_from <= ?3 AND v.valid_until >= ?3
                 ORDER BY l2 LIMIT {TOP_K_RESULTS}"
            );
            let p = vec![
                rusqlite::types::Value::Blob(spec.query_vec_blob.clone()),
                rusqlite::types::Value::Text(spec.scope.clone()),
                rusqlite::types::Value::Integer(1_800_000_000),
            ];
            plans.push(
                explain_sql(conn, &format!("{tag}/search_vector_k10"), &sql, &p)
                    .map_err(|e| format!("eqp vec a1: {e}"))?,
            );
        } else {
            let sql = format!(
                "WITH candidates AS (
                    SELECT rowid, distance FROM vector_default
                    WHERE embedding_bin MATCH vec_quantize_binary(?1)
                      AND scope = ?2 AND lifecycle = 0 AND valid_from <= ?3 AND valid_until >= ?3
                    ORDER BY distance LIMIT {TOP_K_BIT_CANDIDATES}
                 )
                 SELECT c.rowid, vec_distance_l2(v.embedding, ?1) AS l2
                 FROM candidates c JOIN vector_default v ON v.rowid = c.rowid
                 ORDER BY l2 LIMIT {TOP_K_RESULTS}"
            );
            let p = vec![
                rusqlite::types::Value::Blob(spec.query_vec_blob.clone()),
                rusqlite::types::Value::Text(spec.scope.clone()),
                rusqlite::types::Value::Integer(1_800_000_000),
            ];
            plans.push(
                explain_sql(conn, &format!("{tag}/search_vector_k10"), &sql, &p)
                    .map_err(|e| format!("eqp vec 8col: {e}"))?,
            );
        }
    }

    // 3. graph_expand_2hop edge + node lookups
    {
        let scope_node = scope_pred_clause(layout, spec, "scope", 2);
        let scope_edge = scope_pred_clause(layout, spec, "canonical_edges.scope", 2);
        let nsql = format!(
            "SELECT kind, body, write_cursor FROM canonical_nodes
             WHERE logical_id = ?1 AND superseded_at IS NULL AND state = 'active'{scope_node} LIMIT 1"
        );
        let esql = format!(
            "SELECT canonical_edges.write_cursor, canonical_edges.kind, canonical_edges.to_id
             FROM canonical_edges
             WHERE canonical_edges.from_id = ?1 AND canonical_edges.superseded_at IS NULL
               AND canonical_edges.kind IN ('relates_to', 'verifies'){scope_edge} LIMIT 50"
        );
        let mut p = vec![rusqlite::types::Value::Text(spec.seed_logical_id.clone())];
        if spec.use_scope_predicate {
            p.push(rusqlite::types::Value::Text(spec.scope.clone()));
        }
        plans.push(
            explain_sql(conn, &format!("{tag}/graph_node_lookup"), &nsql, &p)
                .map_err(|e| format!("eqp gn: {e}"))?,
        );
        plans.push(
            explain_sql(conn, &format!("{tag}/graph_edge_incident"), &esql, &p)
                .map_err(|e| format!("eqp ge: {e}"))?,
        );
    }

    // 4. operational_state point & page
    {
        let point_scope = scope_pred_clause(layout, spec, "scope", 3);
        let psql = format!(
            "SELECT write_cursor FROM operational_state WHERE collection_name = ?1 AND record_key = ?2{point_scope}"
        );
        let pp = if spec.use_scope_predicate {
            vec![
                rusqlite::types::Value::Text(spec.op_collection.clone()),
                rusqlite::types::Value::Text(spec.op_record_key.clone()),
                rusqlite::types::Value::Text(spec.scope.clone()),
            ]
        } else {
            vec![
                rusqlite::types::Value::Text(spec.op_collection.clone()),
                rusqlite::types::Value::Text(spec.op_record_key.clone()),
            ]
        };
        plans.push(
            explain_sql(conn, &format!("{tag}/operational_state_point"), &psql, &pp)
                .map_err(|e| format!("eqp opp: {e}"))?,
        );

        let page_scope = scope_pred_clause(layout, spec, "scope", 4);
        let pgsql = format!(
            "SELECT write_cursor FROM operational_state WHERE collection_name = ?1 AND write_cursor > ?2{page_scope} ORDER BY write_cursor ASC LIMIT ?3"
        );
        let pgp = if spec.use_scope_predicate {
            vec![
                rusqlite::types::Value::Text(spec.op_collection.clone()),
                rusqlite::types::Value::Integer(spec.op_page_after),
                rusqlite::types::Value::Integer(PAGE_LIMIT as i64),
                rusqlite::types::Value::Text(spec.scope.clone()),
            ]
        } else {
            vec![
                rusqlite::types::Value::Text(spec.op_collection.clone()),
                rusqlite::types::Value::Integer(spec.op_page_after),
                rusqlite::types::Value::Integer(PAGE_LIMIT as i64),
            ]
        };
        plans.push(
            explain_sql(conn, &format!("{tag}/operational_state_page"), &pgsql, &pgp)
                .map_err(|e| format!("eqp oppg: {e}"))?,
        );
    }

    Ok(plans)
}

#[derive(Default)]
struct ReadSamplesAccumulator {
    text_ms: Vec<f64>,
    vec_ms: Vec<f64>,
    hyb_ms: Vec<f64>,
    graph_ms: Vec<f64>,
    op_point_ms: Vec<f64>,
    op_page_ms: Vec<f64>,
    recalls: Vec<f64>,
    returned_counts: Vec<usize>,
}

type ReadCohortOutcome = (Vec<ReadSuiteResult>, Vec<PairedComparison>, Vec<ExplainPlanAudit>);

fn measure_read_cohort(
    db_paths: &[(LayoutId, PathBuf)],
    profile: ProfileKind,
    cohort: &str,
    num_nodes: usize,
    num_op_state: usize,
    emb_gen: &EmbeddingGenerator,
) -> Result<ReadCohortOutcome, String> {
    let specs = build_read_trial_specs(profile, cohort, num_nodes, num_op_state, emb_gen);
    let mut conns: Vec<(LayoutId, Connection)> = Vec::with_capacity(db_paths.len());
    for (layout, path) in db_paths {
        conns.push((
            *layout,
            open_reader_conn(path).map_err(|e| format!("open reader {}: {e}", layout.as_str()))?,
        ));
    }

    // Precompute exact L2 ground truth once per query on the first layout (identical corpus across all layouts!).
    let mut ground_truths: Vec<Vec<i64>> = Vec::with_capacity(specs.len());
    {
        let (_, ref_conn) = &conns[0];
        for (i, spec) in specs.iter().enumerate() {
            if i >= WARMUP_ROUNDS {
                ground_truths.push(exact_l2_ground_truth(ref_conn, db_paths[0].0, spec)?);
            } else {
                ground_truths.push(Vec::new());
            }
        }
    }

    let mut explain_plans = Vec::new();
    for (layout, conn) in &conns {
        explain_plans.extend(capture_explain_plans(conn, *layout, cohort, &specs[WARMUP_ROUNDS])?);
    }

    let mut accs: HashMap<LayoutId, ReadSamplesAccumulator> =
        db_paths.iter().map(|(l, _)| (*l, ReadSamplesAccumulator::default())).collect();

    // Execute paired interleaved trials (alternating layout iteration order per trial).
    for (t_idx, spec) in specs.iter().enumerate() {
        let mut order: Vec<usize> = (0..conns.len()).collect();
        if t_idx % 2 == 1 {
            order.reverse();
        }
        for &c_idx in &order {
            let (layout, conn) = &conns[c_idx];

            let t0 = Instant::now();
            let text_res = exec_search_text_only(conn, *layout, spec)?;
            let dt_text = t0.elapsed().as_secs_f64() * 1000.0;
            let _ = text_res.len();

            let t1 = Instant::now();
            let vec_res = exec_search_vector_k10(conn, *layout, spec)?;
            let dt_vec = t1.elapsed().as_secs_f64() * 1000.0;

            let t2 = Instant::now();
            let hyb_res = exec_search_hybrid(conn, *layout, spec)?;
            let dt_hyb = t2.elapsed().as_secs_f64() * 1000.0;
            let _ = hyb_res.len();

            let t3 = Instant::now();
            let graph_res = exec_graph_expand_2hop(conn, *layout, spec)?;
            let dt_graph = t3.elapsed().as_secs_f64() * 1000.0;
            let _ = graph_res.len();

            let t4 = Instant::now();
            let op_pt = exec_op_state_point_read(conn, *layout, spec)?;
            let dt_pt = t4.elapsed().as_secs_f64() * 1000.0;
            let _ = op_pt;

            let t5 = Instant::now();
            let op_pg = exec_op_state_page(conn, *layout, spec)?;
            let dt_pg = t5.elapsed().as_secs_f64() * 1000.0;
            let _ = op_pg.len();

            if t_idx >= WARMUP_ROUNDS {
                let acc = accs.get_mut(layout).unwrap();
                acc.text_ms.push(dt_text);
                acc.vec_ms.push(dt_vec);
                acc.hyb_ms.push(dt_hyb);
                acc.graph_ms.push(dt_graph);
                acc.op_point_ms.push(dt_pt);
                acc.op_page_ms.push(dt_pg);
                let pred_ids: Vec<i64> = vec_res.iter().map(|(id, _)| *id).collect();
                let r = compute_recall_at_k(&pred_ids, &ground_truths[t_idx], TOP_K_RESULTS);
                acc.recalls.push(r);
                acc.returned_counts.push(pred_ids.len());
            }
        }
    }

    let mut suites = Vec::new();
    for (layout, _) in db_paths {
        let acc = &accs[layout];
        let qn = acc.recalls.len();
        let mean_rec = acc.recalls.iter().sum::<f64>() / qn.max(1) as f64;
        let min_rec = acc.recalls.iter().copied().fold(1.0_f64, f64::min);
        let mean_cnt = acc.returned_counts.iter().sum::<usize>() as f64 / qn.max(1) as f64;
        let min_cnt = *acc.returned_counts.iter().min().unwrap_or(&0);
        let empty_cnt = acc.returned_counts.iter().filter(|&&c| c == 0).count();
        suites.push(ReadSuiteResult {
            layout: layout.as_str().to_string(),
            cohort: cohort.to_string(),
            search_text_only: summarize_samples(&acc.text_ms),
            search_vector_k10: summarize_samples(&acc.vec_ms),
            search_vector_recall: VectorRecallSummary {
                queries: qn,
                mean_recall_at_10: round4(mean_rec),
                min_recall_at_10: round4(min_rec),
                mean_returned_count: round4(mean_cnt),
                min_returned_count: min_cnt,
                empty_result_count: empty_cnt,
            },
            search_hybrid: summarize_samples(&acc.hyb_ms),
            graph_expand_2hop: summarize_samples(&acc.graph_ms),
            operational_state_point_read: summarize_samples(&acc.op_point_ms),
            operational_state_page: summarize_samples(&acc.op_page_ms),
        });
    }

    let mut comparisons = Vec::new();
    let base_layout = db_paths[0].0;
    for (cand_layout, _) in &db_paths[1..] {
        let b = &accs[&base_layout];
        let c = &accs[cand_layout];
        let ops: [(&str, &[f64], &[f64]); 6] = [
            ("search_text_only", &b.text_ms, &c.text_ms),
            ("search_vector_k10", &b.vec_ms, &c.vec_ms),
            ("search_hybrid", &b.hyb_ms, &c.hyb_ms),
            ("graph_expand_2hop", &b.graph_ms, &c.graph_ms),
            ("operational_state_point_read", &b.op_point_ms, &c.op_point_ms),
            ("operational_state_page", &b.op_page_ms, &c.op_page_ms),
        ];
        for (op_idx, (op_name, bv, cv)) in ops.iter().enumerate() {
            comparisons.push(compare_paired(
                &format!("{op_name}:{cohort}:{}", base_layout.as_str()),
                &format!("{op_name}:{cohort}:{}", cand_layout.as_str()),
                bv,
                cv,
                DEFAULT_SEED + 100 + op_idx as u64,
            ));
        }
    }
    // Also compare A1 vs A2 directly when both are present.
    if accs.contains_key(&LayoutId::S38PartialA1) && accs.contains_key(&LayoutId::S38PartialA2) {
        let a1 = &accs[&LayoutId::S38PartialA1];
        let a2 = &accs[&LayoutId::S38PartialA2];
        comparisons.push(compare_paired(
            &format!("search_vector_k10:{cohort}:S38-PARTIAL-A1"),
            &format!("search_vector_k10:{cohort}:S38-PARTIAL-A2"),
            &a1.vec_ms,
            &a2.vec_ms,
            DEFAULT_SEED + 200,
        ));
        comparisons.push(compare_paired(
            &format!("search_hybrid:{cohort}:S38-PARTIAL-A1"),
            &format!("search_hybrid:{cohort}:S38-PARTIAL-A2"),
            &a1.hyb_ms,
            &a2.hyb_ms,
            DEFAULT_SEED + 201,
        ));
    }

    Ok((suites, comparisons, explain_plans))
}

// ---------------------------------------------------------------------------
// Profile orchestrator
// ---------------------------------------------------------------------------

fn run_profile(
    exe: &Path,
    work_root: &Path,
    profile: ProfileKind,
    num_nodes: usize,
    num_op_state: usize,
    num_op_mutations: usize,
    emb_gen: &EmbeddingGenerator,
) -> Result<ProfileReport, String> {
    eprintln!("[wp10d] starting profile {} (nodes={num_nodes})...", profile.as_str());
    let pdir = work_root.join(profile.as_str());
    fs::create_dir_all(&pdir).map_err(|e| format!("create pdir: {e}"))?;

    let mut scope_distribution: BTreeMap<String, usize> = BTreeMap::new();
    for i in 0..num_nodes {
        let sc = scope_for_node_scaled(profile, i, num_nodes);
        let key = if sc.is_empty() { "<unscoped>".to_string() } else { sc };
        *scope_distribution.entry(key).or_default() += 1;
    }

    let b0_path = pdir.join("master_b0.sqlite");
    let a1_path = pdir.join("master_s38_a1.sqlite");
    let a2_path = pdir.join("master_s38_a2.sqlite");
    let b_path = pdir.join("master_s39_b.sqlite");

    build_anchor_database(&b0_path, profile, num_nodes, num_op_state, num_op_mutations, emb_gen)?;
    eprintln!("[wp10d]   anchor built at {}", b0_path.display());

    // Build derived layouts cleanly.
    copy_sqlite_clean(&b0_path, &a1_path)?;
    apply_s38_partial_indexes(&a1_path)?;

    copy_sqlite_clean(&a1_path, &a2_path)?;
    reshape_vec0_to_8col(&a2_path)?;

    copy_sqlite_clean(&b0_path, &b_path)?;
    transform_to_s39_composite(&b_path)?;
    eprintln!("[wp10d]   all layouts prepared (B0/A1/A2/S39)");

    // Measure WAL-truncated database sizes.
    let mut size_targets = Vec::new();
    if profile == ProfileKind::SingleScope {
        size_targets.push((LayoutId::B0, b0_path.clone()));
    }
    size_targets.push((LayoutId::S38PartialA1, a1_path.clone()));
    size_targets.push((LayoutId::S38PartialA2, a2_path.clone()));
    size_targets.push((LayoutId::S39Composite, b_path.clone()));

    let b0_size = checkpoint_truncate_and_size(&b0_path).map_err(|e| format!("size b0: {e}"))?;
    let mut database_sizes = Vec::new();
    if profile != ProfileKind::SingleScope {
        let mut b0_rep = b0_size.clone();
        b0_rep.layout = LayoutId::B0.as_str().to_string();
        database_sizes.push(b0_rep);
    }
    for (layout, path) in &size_targets {
        let mut rep = checkpoint_truncate_and_size(path)
            .map_err(|e| format!("size {}: {e}", layout.as_str()))?;
        rep.layout = layout.as_str().to_string();
        if *layout != LayoutId::B0 {
            let delta = rep.file_bytes as i64 - b0_size.file_bytes as i64;
            rep.delta_bytes_vs_b0 = Some(delta);
            rep.relative_delta_vs_b0 =
                Some(round4(delta as f64 / b0_size.file_bytes.max(1) as f64));
        }
        database_sizes.push(rep);
    }

    // Subprocess open probes (`N = 30`).
    let real_engine_layouts = if profile == ProfileKind::SingleScope {
        vec![LayoutId::B0, LayoutId::S38PartialA1]
    } else {
        vec![LayoutId::S38PartialA1]
    };
    let (open_probes, open_comparisons) =
        measure_open_suite(exe, &size_targets, &real_engine_layouts)?;
    eprintln!("[wp10d]   open probes complete");

    // Write suites (`N = 100` each under NORMAL and FULL from clean restored snapshots)
    // plus 10,000-resample paired bootstrap CIs (`NORMAL` vs `FULL`, and candidate vs baseline).
    let (write_suites, write_comparisons) =
        run_write_suites_with_cis(&pdir, &size_targets, profile, num_nodes, num_op_state)?;
    eprintln!("[wp10d]   write suites (NORMAL & FULL) complete");

    // Read / Search / Graph / OpState / Recall@10 / EXPLAIN QUERY PLAN suites (`N = 200`).
    let mut read_suites = Vec::new();
    let mut read_comparisons = Vec::new();
    let mut explain_plans = Vec::new();
    let cohorts: Vec<&str> = match profile {
        ProfileKind::SingleScope => vec!["unscoped_fast_path"],
        ProfileKind::MultiScopeUniform => vec!["uniform_10pct"],
        ProfileKind::MultiScopeSkewed => vec!["heavy_90pct", "light_1pct"],
    };
    for cohort in cohorts {
        let (rs, rc, eqp) =
            measure_read_cohort(&size_targets, profile, cohort, num_nodes, num_op_state, emb_gen)?;
        read_suites.extend(rs);
        read_comparisons.extend(rc);
        explain_plans.extend(eqp);
        eprintln!("[wp10d]   read cohort {cohort} complete");
    }

    // Clean up profile temp directory immediately to keep /tmp peak disk low.
    let _ = fs::remove_dir_all(&pdir);

    let num_edges = num_nodes + num_nodes.div_ceil(2);
    Ok(ProfileReport {
        profile: profile.as_str().to_string(),
        num_nodes,
        num_edges,
        num_op_state,
        num_op_mutations,
        scope_distribution,
        database_sizes,
        open_probes,
        open_comparisons,
        write_suites,
        write_comparisons,
        read_suites,
        read_comparisons,
        explain_plans,
    })
}

struct WriteSuiteRaw {
    summary: WriteSuiteResult,
    fresh_node_ms: Vec<f64>,
    upsert_node_ms: Vec<f64>,
    edge_ms: Vec<f64>,
    cp_fresh_ms: Vec<f64>,
    cp_upsert_ms: Vec<f64>,
}

fn measure_write_suite_raw(
    master_db: &Path,
    work_db: &Path,
    layout: LayoutId,
    profile: ProfileKind,
    sync_mode: SyncMode,
    num_nodes: usize,
    num_op_state: usize,
) -> Result<WriteSuiteRaw, String> {
    copy_sqlite_clean(master_db, work_db)?;
    let mut conn =
        open_writer_conn(work_db, sync_mode).map_err(|e| format!("open write arm: {e}"))?;
    let _: (i64, i64, i64) = conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(|e| format!("arm truncate: {e}"))?;

    let mut cursor_base: i64 = conn
        .query_row(
            "SELECT MAX(m) FROM (
                SELECT COALESCE(MAX(write_cursor),0) AS m FROM canonical_nodes
                UNION ALL SELECT COALESCE(MAX(write_cursor),0) FROM canonical_edges
                UNION ALL SELECT COALESCE(MAX(write_cursor),0) FROM operational_state
                UNION ALL SELECT COALESCE(MAX(write_cursor),0) FROM operational_mutations
             )",
            [],
            |r| r.get(0),
        )
        .map_err(|e| format!("load max cursor: {e}"))?;

    let callers = profile.callers();
    let total_batches = WARMUP_ROUNDS + WRITE_TRIALS;

    let mut fresh_node_ms = Vec::with_capacity(WRITE_TRIALS);
    for t in 0..total_batches {
        let scope = &callers[t % callers.len()];
        let mut items = Vec::with_capacity(BATCH_SIZE);
        for j in 0..BATCH_SIZE {
            let id_num = 1_000_000 + t * BATCH_SIZE + j;
            let topic = TOPICS[id_num % TOPICS.len()];
            items.push((
                id_num,
                format!("fresh_node:{id_num}"),
                format!("Fresh document {id_num} telemetry {topic} architecture verification"),
                node_properties(id_num),
            ));
        }
        let t0 = Instant::now();
        commit_node_batch(&mut conn, layout, scope, &mut cursor_base, &items, false)?;
        let elapsed = t0.elapsed().as_secs_f64() * 1000.0;
        if t >= WARMUP_ROUNDS {
            fresh_node_ms.push(elapsed);
        }
    }

    let mut upsert_node_ms = Vec::with_capacity(WRITE_TRIALS);
    for t in 0..total_batches {
        let seed_idx = match profile {
            ProfileKind::SingleScope => (t * BATCH_SIZE) % num_nodes,
            ProfileKind::MultiScopeUniform => {
                let c = t % NUM_CALLERS;
                let block = (num_nodes / NUM_CALLERS).max(1);
                c * block
                    + ((t / NUM_CALLERS) * BATCH_SIZE) % block.saturating_sub(BATCH_SIZE).max(1)
            }
            ProfileKind::MultiScopeSkewed => {
                if t % 2 == 0 {
                    (t * BATCH_SIZE) % ((num_nodes * 9) / 10).saturating_sub(BATCH_SIZE).max(1)
                } else {
                    let c = 1 + (t % (NUM_CALLERS - 1));
                    let heavy = (num_nodes * 9) / 10;
                    let per = ((num_nodes - heavy) / (NUM_CALLERS - 1)).max(1);
                    let approx = (heavy + (c - 1) * per).min(num_nodes - 1);
                    let (s_start, s_end) = scope_node_bounds(profile, approx, num_nodes);
                    let span = (s_end - s_start).saturating_sub(BATCH_SIZE).max(1);
                    s_start + ((t * 17) % span)
                }
            }
        };
        let scope = scope_for_node_scaled(profile, seed_idx, num_nodes);
        let (s_start, s_end) = scope_node_bounds(profile, seed_idx, num_nodes);
        let span = (s_end - s_start).max(1);
        let mut items = Vec::with_capacity(BATCH_SIZE);
        for j in 0..BATCH_SIZE {
            let target_idx = s_start + ((seed_idx - s_start + j) % span);
            let topic = TOPICS[(target_idx + t) % TOPICS.len()];
            items.push((
                target_idx,
                format!("node:{target_idx}"),
                format!("Updated document {target_idx} rev {t} {topic} verification benchmark"),
                node_properties(target_idx + t),
            ));
        }
        let t0 = Instant::now();
        commit_node_batch(&mut conn, layout, &scope, &mut cursor_base, &items, true)?;
        let elapsed = t0.elapsed().as_secs_f64() * 1000.0;
        if t >= WARMUP_ROUNDS {
            upsert_node_ms.push(elapsed);
        }
    }

    let mut edge_ms = Vec::with_capacity(WRITE_TRIALS);
    for t in 0..total_batches {
        let seed_idx = match profile {
            ProfileKind::SingleScope => (t * 37) % num_nodes,
            ProfileKind::MultiScopeUniform => (t % NUM_CALLERS) * (num_nodes / NUM_CALLERS),
            ProfileKind::MultiScopeSkewed => {
                if t % 2 == 0 {
                    0
                } else {
                    let c = 1 + (t % (NUM_CALLERS - 1));
                    let heavy = (num_nodes * 9) / 10;
                    let per = ((num_nodes - heavy) / (NUM_CALLERS - 1)).max(1);
                    (heavy + (c - 1) * per).min(num_nodes - 1)
                }
            }
        };
        let scope = scope_for_node_scaled(profile, seed_idx, num_nodes);
        let (s_start, s_end) = scope_node_bounds(profile, seed_idx, num_nodes);
        let span = (s_end - s_start).max(1);
        let mut edges = Vec::with_capacity(BATCH_SIZE);
        for j in 0..BATCH_SIZE {
            let from_idx = s_start + ((t * 13 + j) % span);
            let to_idx = s_start + ((t * 13 + j + 5) % span);
            let eid = 2_000_000 + t * BATCH_SIZE + j;
            edges.push((
                eid,
                format!("fresh_edge:{eid}"),
                format!("node:{from_idx}"),
                format!("node:{to_idx}"),
            ));
        }
        let t0 = Instant::now();
        commit_edge_batch(&mut conn, layout, &scope, &mut cursor_base, &edges)?;
        let elapsed = t0.elapsed().as_secs_f64() * 1000.0;
        if t >= WARMUP_ROUNDS {
            edge_ms.push(elapsed);
        }
    }

    let mut cp_fresh_ms = Vec::with_capacity(WRITE_TRIALS);
    for t in 0..total_batches {
        let scope = &callers[t % callers.len()];
        let col = OP_COLLECTIONS[t % OP_COLLECTIONS.len()];
        let key = format!("fresh_cp:{t}");
        let payload = format!(r#"{{"step":{t},"status":"active","fresh":true}}"#);
        let t0 = Instant::now();
        commit_single_checkpoint(&mut conn, layout, scope, col, &key, &payload, &mut cursor_base)?;
        let elapsed = t0.elapsed().as_secs_f64() * 1000.0;
        if t >= WARMUP_ROUNDS {
            cp_fresh_ms.push(elapsed);
        }
    }

    let mut cp_upsert_ms = Vec::with_capacity(WRITE_TRIALS);
    for t in 0..total_batches {
        let idx = (t * 7) % num_op_state.max(1);
        let scaled_idx = (idx * num_nodes) / num_op_state.max(1);
        let scope = scope_for_node_scaled(profile, scaled_idx, num_nodes);
        let col = OP_COLLECTIONS[idx % OP_COLLECTIONS.len()];
        let key = format!("cp:{idx}");
        let payload = format!(r#"{{"step":{idx},"rev":{t},"status":"updated"}}"#);
        let t0 = Instant::now();
        commit_single_checkpoint(&mut conn, layout, &scope, col, &key, &payload, &mut cursor_base)?;
        let elapsed = t0.elapsed().as_secs_f64() * 1000.0;
        if t >= WARMUP_ROUNDS {
            cp_upsert_ms.push(elapsed);
        }
    }

    let ownership_guard_verified =
        verify_ownership_guard(&mut conn, layout, profile, num_nodes, &mut cursor_base)?;

    drop(conn);
    for suffix in ["", "-wal", "-shm", ".lock"] {
        let _ = fs::remove_file(format!("{}{suffix}", work_db.display()));
    }

    Ok(WriteSuiteRaw {
        summary: WriteSuiteResult {
            layout: layout.as_str().to_string(),
            sync_mode: sync_mode.as_str().to_string(),
            write_node_batch_200_fresh: summarize_samples(&fresh_node_ms),
            write_node_batch_200_upsert: summarize_samples(&upsert_node_ms),
            write_edge_batch_200: summarize_samples(&edge_ms),
            single_row_checkpoint_write_fresh: summarize_samples(&cp_fresh_ms),
            single_row_checkpoint_write_upsert: summarize_samples(&cp_upsert_ms),
            ownership_guard_verified,
        },
        fresh_node_ms,
        upsert_node_ms,
        edge_ms,
        cp_fresh_ms,
        cp_upsert_ms,
    })
}

fn run_write_suites_with_cis(
    pdir: &Path,
    targets: &[(LayoutId, PathBuf)],
    profile: ProfileKind,
    num_nodes: usize,
    num_op_state: usize,
) -> Result<(Vec<WriteSuiteResult>, Vec<PairedComparison>), String> {
    let mut raws: HashMap<(LayoutId, &'static str), WriteSuiteRaw> = HashMap::new();
    let mut summaries = Vec::new();
    for (layout, master_path) in targets {
        for sync_mode in [SyncMode::Normal, SyncMode::Full] {
            let work_db =
                pdir.join(format!("w_{}_{}.sqlite", layout.cli_flag(), sync_mode.cli_flag()));
            let raw = measure_write_suite_raw(
                master_path,
                &work_db,
                *layout,
                profile,
                sync_mode,
                num_nodes,
                num_op_state,
            )?;
            summaries.push(raw.summary.clone());
            raws.insert((*layout, sync_mode.as_str()), raw);
        }
    }

    let mut comparisons = Vec::new();
    // 1. SyncMode::Normal vs SyncMode::Full within every layout.
    for (layout, _) in targets {
        let n = &raws[&(*layout, "NORMAL")];
        let f = &raws[&(*layout, "FULL")];
        let ops: [(&str, &[f64], &[f64]); 5] = [
            ("write_node_batch_200_fresh", &n.fresh_node_ms, &f.fresh_node_ms),
            ("write_node_batch_200_upsert", &n.upsert_node_ms, &f.upsert_node_ms),
            ("write_edge_batch_200", &n.edge_ms, &f.edge_ms),
            ("single_row_checkpoint_write_fresh", &n.cp_fresh_ms, &f.cp_fresh_ms),
            ("single_row_checkpoint_write_upsert", &n.cp_upsert_ms, &f.cp_upsert_ms),
        ];
        for (i, (op, nv, fv)) in ops.iter().enumerate() {
            comparisons.push(compare_paired(
                &format!("{op}:{}:NORMAL", layout.as_str()),
                &format!("{op}:{}:FULL", layout.as_str()),
                nv,
                fv,
                DEFAULT_SEED + 300 + i as u64,
            ));
        }
    }

    // 2. Layout comparisons under NORMAL and FULL against first layout (`B0` or `S38-PARTIAL-A1`).
    let base_layout = targets[0].0;
    for sync_str in ["NORMAL", "FULL"] {
        let b = &raws[&(base_layout, sync_str)];
        for (cand_layout, _) in &targets[1..] {
            let c = &raws[&(*cand_layout, sync_str)];
            let ops: [(&str, &[f64], &[f64]); 5] = [
                ("write_node_batch_200_fresh", &b.fresh_node_ms, &c.fresh_node_ms),
                ("write_node_batch_200_upsert", &b.upsert_node_ms, &c.upsert_node_ms),
                ("write_edge_batch_200", &b.edge_ms, &c.edge_ms),
                ("single_row_checkpoint_write_fresh", &b.cp_fresh_ms, &c.cp_fresh_ms),
                ("single_row_checkpoint_write_upsert", &b.cp_upsert_ms, &c.cp_upsert_ms),
            ];
            for (i, (op, bv, cv)) in ops.iter().enumerate() {
                comparisons.push(compare_paired(
                    &format!("{op}:{}:{sync_str}", base_layout.as_str()),
                    &format!("{op}:{}:{sync_str}", cand_layout.as_str()),
                    bv,
                    cv,
                    DEFAULT_SEED + 400 + i as u64,
                ));
            }
        }
    }

    Ok((summaries, comparisons))
}

// ---------------------------------------------------------------------------
// Environment introspection & CLI entrypoint
// ---------------------------------------------------------------------------

fn cpu_model() -> String {
    if let Ok(text) = fs::read_to_string("/proc/cpuinfo") {
        for line in text.lines() {
            if line.starts_with("model name") {
                if let Some((_, val)) = line.split_once(':') {
                    return val.trim().to_string();
                }
            }
        }
    }
    std::env::consts::ARCH.to_string()
}

#[cfg(test)]
fn check_s38_vs_s39_ownership_and_isolation(
    db_path: &Path,
    is_s39: bool,
) -> Result<(bool, bool), String> {
    register_sqlite_vec();
    let emb_gen = EmbeddingGenerator::new(DEFAULT_SEED);
    build_anchor_database(db_path, ProfileKind::MultiScopeUniform, 200, 50, 50, &emb_gen)?;
    let layout = if is_s39 {
        transform_to_s39_composite(db_path)?;
        LayoutId::S39Composite
    } else {
        apply_s38_partial_indexes(db_path)?;
        LayoutId::S38PartialA1
    };
    let mut conn = open_writer_conn(db_path, SyncMode::Normal).map_err(|e| format!("{e}"))?;
    let mut cursor_base: i64 = 10_000;
    let verified = verify_ownership_guard(
        &mut conn,
        layout,
        ProfileKind::MultiScopeUniform,
        200,
        &mut cursor_base,
    )?;
    if is_s39 {
        let count_lids: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM canonical_nodes WHERE logical_id = 'node:0' AND superseded_at IS NULL",
                [],
                |r| r.get(0),
            )
            .map_err(|e| format!("{e}"))?;
        Ok((false, verified && count_lids == 2))
    } else {
        Ok((verified, false))
    }
}

fn run_experiment(out_path: &Path, work_root: &Path, num_nodes: usize) -> Result<(), String> {
    register_sqlite_vec();
    let exe = env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    fs::create_dir_all(work_root).map_err(|e| format!("create work_root: {e}"))?;
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create out parent: {e}"))?;
    }

    let emb_gen = EmbeddingGenerator::new(DEFAULT_SEED);
    let num_op_state = (DEFAULT_OP_STATE_ROWS * num_nodes) / DEFAULT_NODES;
    let num_op_mutations = (DEFAULT_OP_MUTATION_ROWS * num_nodes) / DEFAULT_NODES;

    let vec0_default_clause_supported = probe_vec0_default_clause();
    let vacuum_supported_on_vec0 = probe_vacuum_on_vec0();
    let sqlite_vec_version = {
        let c = Connection::open_in_memory().map_err(|e| format!("mem conn: {e}"))?;
        c.query_row("SELECT vec_version()", [], |r| r.get::<_, String>(0))
            .map_err(|e| format!("vec_version: {e}"))?
    };

    let mut profiles = Vec::new();
    for profile in
        [ProfileKind::SingleScope, ProfileKind::MultiScopeUniform, ProfileKind::MultiScopeSkewed]
    {
        profiles.push(run_profile(
            &exe,
            work_root,
            profile,
            num_nodes,
            num_op_state.max(20),
            num_op_mutations.max(20),
            &emb_gen,
        )?);
    }

    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let report = ExperimentReport {
        schema: "fathomdb.0832-wp10d-scope-schema-experiment/v1",
        generated_at_unix_s: now_unix,
        environment: ExperimentEnvironment {
            cpu_model: cpu_model(),
            cpu_count: std::thread::available_parallelism().map_or(1, |n| n.get()),
            sqlite_version: rusqlite::version().to_string(),
            sqlite_vec_version,
            schema_version_baseline: fathomdb_schema::SCHEMA_VERSION,
            rust_profile: if cfg!(debug_assertions) { "debug" } else { "release" },
            seed: DEFAULT_SEED,
            warmup_rounds: WARMUP_ROUNDS,
            open_trials: OPEN_TRIALS,
            write_trials: WRITE_TRIALS,
            read_trials: READ_TRIALS,
            bootstrap_resamples: BOOTSTRAP_RESAMPLES,
        },
        vec0_default_clause_supported,
        vacuum_supported_on_vec0,
        profiles,
    };

    let json = serde_json::to_vec_pretty(&report).map_err(|e| format!("serialize report: {e}"))?;
    let mut sha = Sha256::new();
    sha.update(&json);
    let digest_hex: String = sha.finalize().iter().map(|b| format!("{b:02x}")).collect();
    fs::write(out_path, &json).map_err(|e| format!("write {}: {e}", out_path.display()))?;
    eprintln!("[wp10d] wrote {} ({} bytes, sha256={digest_hex})", out_path.display(), json.len(),);
    let _ = fs::remove_dir_all(work_root);
    Ok(())
}

fn parse_flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find_map(|w| (w[0] == flag).then(|| w[1].clone()))
}

fn run_cli() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "--open-probe") {
        let db = PathBuf::from(parse_flag_value(&args, "--db").ok_or("missing --db")?);
        let layout =
            LayoutId::parse_flag(&parse_flag_value(&args, "--layout").ok_or("missing --layout")?)?;
        let sync =
            SyncMode::parse_flag(&parse_flag_value(&args, "--sync").ok_or("missing --sync")?)?;
        let warmup: usize = parse_flag_value(&args, "--warmup")
            .ok_or("missing --warmup")?
            .parse()
            .map_err(|e| format!("bad --warmup: {e}"))?;
        let count: usize = parse_flag_value(&args, "--count")
            .ok_or("missing --count")?
            .parse()
            .map_err(|e| format!("bad --count: {e}"))?;
        let real_engine = args.iter().any(|a| a == "--real-engine");
        let res = run_open_probe_child(&db, layout, sync, warmup, count, real_engine)?;
        println!("{}", serde_json::to_string(&res).map_err(|e| format!("serialize child: {e}"))?);
        return Ok(());
    }

    let out_path = PathBuf::from(parse_flag_value(&args, "--out").unwrap_or_else(|| {
        "dev/plans/0.8.32/features/slice-10/scope-schema-results.json".to_string()
    }));
    let work_dir = PathBuf::from(
        parse_flag_value(&args, "--work-dir")
            .unwrap_or_else(|| format!("/tmp/fathomdb-wp10d-bench-{}", std::process::id())),
    );
    let num_nodes: usize = parse_flag_value(&args, "--nodes")
        .map(|s| s.parse().map_err(|e| format!("bad --nodes: {e}")))
        .transpose()?
        .unwrap_or(DEFAULT_NODES);

    run_experiment(&out_path, &work_dir, num_nodes)
}

fn main() -> ExitCode {
    match run_cli() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("scope_schema_experiment error: {err}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank_and_bootstrap_ci_match_paired_gate_contract() {
        let base = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let cand = vec![12.0, 22.0, 32.0, 42.0, 52.0];
        assert!((nearest_rank_percentile(&base, 50.0) - 30.0).abs() < 1e-9);
        assert!((nearest_rank_percentile(&base, 95.0) - 50.0).abs() < 1e-9);
        let ci = paired_bootstrap_ci(&base, &cand, 50.0, 1000, 20261124);
        assert!((ci.observed_delta_ms - 2.0).abs() < 1e-9);
        assert!(ci.ci_95_lower_ms <= ci.observed_delta_ms);
        assert!(ci.ci_95_upper_ms >= ci.observed_delta_ms);
    }

    #[test]
    fn recall_at_k_handles_partial_and_exact_sets() {
        let truth = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let pred = vec![10, 9, 8, 7, 6, 99, 98, 97, 96, 95];
        let r = compute_recall_at_k(&pred, &truth, 10);
        assert!((r - 0.5).abs() < 1e-9);
        assert!((compute_recall_at_k(&truth, &truth, 10) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn explain_audit_distinguishes_virtual_table_from_real_scan_and_catches_temp_btree() {
        let clean = audit_explain_plan(vec![
            "SCAN si VIRTUAL TABLE INDEX 0:M1".to_string(),
            "SEARCH cn USING INDEX canonical_nodes_scope_cursor_idx (scope=? AND write_cursor=?)"
                .to_string(),
        ]);
        assert!(clean.real_table_scans.is_empty());
        assert!(!clean.uses_temp_btree_order_by);
        assert!(clean.clean);

        let bad = audit_explain_plan(vec![
            "SCAN cn".to_string(),
            "USE TEMP B-TREE FOR ORDER BY".to_string(),
        ]);
        assert_eq!(bad.real_table_scans, vec!["SCAN cn".to_string()]);
        assert!(bad.uses_temp_btree_order_by);
        assert!(!bad.clean);
    }

    #[test]
    fn profile_scope_distributions_match_100k_specification() {
        let mut uniform_counts: BTreeMap<String, usize> = BTreeMap::new();
        let mut skewed_counts: BTreeMap<String, usize> = BTreeMap::new();
        for i in 0..100_000 {
            assert_eq!(scope_for_node(ProfileKind::SingleScope, i), "");
            *uniform_counts
                .entry(scope_for_node(ProfileKind::MultiScopeUniform, i))
                .or_default() += 1;
            *skewed_counts.entry(scope_for_node(ProfileKind::MultiScopeSkewed, i)).or_default() +=
                1;
        }
        assert_eq!(uniform_counts.len(), 10);
        assert!(uniform_counts.values().all(|&c| c == 10_000));
        assert_eq!(skewed_counts.len(), 10);
        assert_eq!(skewed_counts.get("heavy_0"), Some(&90_000));
        let light_sum: usize =
            skewed_counts.iter().filter(|(k, _)| k.starts_with("light_")).map(|(_, &v)| v).sum();
        assert_eq!(light_sum, 10_000);
    }

    #[test]
    fn vec0_supports_8_builtin_including_scope_plus_8_attr_slots() {
        register_sqlite_vec();
        let conn = Connection::open_in_memory().unwrap();
        let ids = check_vec0_16col_support(&conn).unwrap();
        assert_eq!(ids, vec![2]);
    }

    #[test]
    fn s38_partial_rejects_cross_scope_overwrite_while_s39_isolates_same_key() {
        let dir = tempfile::tempdir().unwrap();
        let s38_db = dir.path().join("s38.sqlite");
        let (s38_rejected_cross_scope, _) =
            check_s38_vs_s39_ownership_and_isolation(&s38_db, false).unwrap();
        assert!(s38_rejected_cross_scope);

        let s39_db = dir.path().join("s39.sqlite");
        let (_, s39_isolated_both_rows) =
            check_s38_vs_s39_ownership_and_isolation(&s39_db, true).unwrap();
        assert!(s39_isolated_both_rows);
    }

    #[test]
    fn real_engine_reopens_populated_b0_and_a1_and_plans_are_clean() {
        register_sqlite_vec();
        let dir = tempfile::tempdir().unwrap();
        let b0 = dir.path().join("b0_smoke.sqlite");
        let a1 = dir.path().join("a1_smoke.sqlite");
        let a2 = dir.path().join("a2_smoke.sqlite");
        let s39 = dir.path().join("s39_smoke.sqlite");
        let emb_gen = EmbeddingGenerator::new(DEFAULT_SEED);

        build_anchor_database(&b0, ProfileKind::MultiScopeSkewed, 500, 100, 100, &emb_gen).unwrap();
        copy_sqlite_clean(&b0, &a1).unwrap();
        apply_s38_partial_indexes(&a1).unwrap();
        copy_sqlite_clean(&a1, &a2).unwrap();
        reshape_vec0_to_8col(&a2).unwrap();
        copy_sqlite_clean(&b0, &s39).unwrap();
        transform_to_s39_composite(&s39).unwrap();

        // Real Engine::open must succeed on both B0 and S38-PARTIAL-A1.
        for path in [&b0, &a1] {
            let opened = Engine::open(path).unwrap();
            opened.engine.close().unwrap();
        }

        // Verify scope assignment populated all 4 tables without leaving empty-scope rows.
        {
            let check_conn = open_reader_conn(&b0).unwrap();
            for tbl in
                ["canonical_nodes", "canonical_edges", "operational_state", "operational_mutations"]
            {
                let empty_cnt: i64 = check_conn
                    .query_row(&format!("SELECT COUNT(*) FROM {tbl} WHERE scope = ''"), [], |r| {
                        r.get(0)
                    })
                    .unwrap();
                assert_eq!(empty_cnt, 0, "expected zero empty-scope rows in {tbl}");
            }
        }

        // Verify EXPLAIN QUERY PLAN cleanliness on light caller in Skewed profile, plus
        // SingleScope S39Composite native `scope = ''` binding.
        let light_spec =
            build_read_trial_specs(ProfileKind::MultiScopeSkewed, "light_1pct", 500, 100, &emb_gen)
                .into_iter()
                .nth(WARMUP_ROUNDS)
                .unwrap();
        for (layout, path) in [
            (LayoutId::S38PartialA1, &a1),
            (LayoutId::S38PartialA2, &a2),
            (LayoutId::S39Composite, &s39),
        ] {
            let conn = open_reader_conn(path).unwrap();
            let plans = capture_explain_plans(&conn, layout, "light_1pct", &light_spec).unwrap();
            for p in plans {
                assert!(
                    p.real_table_scans.is_empty(),
                    "expected zero real-table SCAN for {}: {:?}",
                    p.query_name,
                    p
                );
                if !p.query_name.ends_with("/search_vector_k10") {
                    assert!(
                        !p.uses_temp_btree_order_by,
                        "expected zero USE TEMP B-TREE FOR ORDER BY for {}: {:?}",
                        p.query_name, p
                    );
                }
            }
        }
        {
            let unscoped_spec = build_read_trial_specs(
                ProfileKind::SingleScope,
                "unscoped_fast_path",
                500,
                100,
                &emb_gen,
            )
            .into_iter()
            .nth(WARMUP_ROUNDS)
            .unwrap();
            let s39_conn = open_reader_conn(&s39).unwrap();
            for p in capture_explain_plans(
                &s39_conn,
                LayoutId::S39Composite,
                "unscoped_fast_path",
                &unscoped_spec,
            )
            .unwrap()
            {
                assert!(
                    p.real_table_scans.is_empty(),
                    "expected zero real-table SCAN on S39 SingleScope for {}: {:?}",
                    p.query_name,
                    p
                );
                if !p.query_name.ends_with("/search_vector_k10") {
                    assert!(
                        !p.uses_temp_btree_order_by,
                        "expected zero TEMP B-TREE on S39 SingleScope for {}: {:?}",
                        p.query_name, p
                    );
                }
            }
        }
    }
}
