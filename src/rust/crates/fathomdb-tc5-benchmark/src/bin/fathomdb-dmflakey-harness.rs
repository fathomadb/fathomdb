//! Guest fault-injection workload harness and external oracle verifier for
//! `AC32-DMFLAKEY-01..04` (`ADR-0.8.32` #101).
//!
//! Runs inside a disposable KVM guest against `/mnt/flakey/fathom.db`,
//! emitting two-phase `INFLIGHT` / `ACK` records to an external virtio-serial
//! or file oracle outside the faulted block device, and verifies post-reboot
//! recovery invariants under `SyncMode::Normal` and `SyncMode::Full`.

use fathomdb_engine::{
    inspect_data_plane_integrity, recover_complete_erasures, ActuationBatchV1,
    ActuationOperationV1, ArtifactRevisionId, CheckIntegrityOpts, DataPlaneIntegrityRequestV1,
    EmbedderChoice, Engine, EngineConfig, InitialState, LifecycleState, PreparedWrite,
    ProvenancedNodeV1, ReadView, ScopePolicy, Section, SourceId, SourceVersionId, SyncMode,
    WriteProvenanceV1,
};
use fathomdb_schema::LOCK_SUFFIX;
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_SCOPE: &str = "tenant-flakey";
const NODE_KIND: &str = "doc";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Scenario {
    ActiveCommit,
    WalAccum,
    CheckpointErasure,
    ReopenReplay,
    ErasureLockFault,
    ErrorWritesFull,
}

impl Scenario {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "active_commit" => Ok(Self::ActiveCommit),
            "wal_accum" => Ok(Self::WalAccum),
            "checkpoint_erasure" => Ok(Self::CheckpointErasure),
            "reopen_replay" => Ok(Self::ReopenReplay),
            "erasure_lock_fault" => Ok(Self::ErasureLockFault),
            "error_writes_full" => Ok(Self::ErrorWritesFull),
            other => Err(format!("unknown scenario: {other}")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::ActiveCommit => "active_commit",
            Self::WalAccum => "wal_accum",
            Self::CheckpointErasure => "checkpoint_erasure",
            Self::ReopenReplay => "reopen_replay",
            Self::ErasureLockFault => "erasure_lock_fault",
            Self::ErrorWritesFull => "error_writes_full",
        }
    }

    fn default_count(self) -> u64 {
        match self {
            Self::ActiveCommit => 60,
            Self::WalAccum => 40,
            Self::CheckpointErasure => 24,
            Self::ReopenReplay => 40,
            Self::ErasureLockFault => 24,
            Self::ErrorWritesFull => 12,
        }
    }
}

fn parse_sync_mode(raw: &str) -> Result<SyncMode, String> {
    match raw {
        "normal" => Ok(SyncMode::Normal),
        "full" => Ok(SyncMode::Full),
        other => Err(format!("invalid --sync-mode '{other}' (expected normal|full)")),
    }
}

fn sync_mode_str(mode: SyncMode) -> &'static str {
    match mode {
        SyncMode::Normal => "normal",
        SyncMode::Full => "full",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OpKind {
    Write,
    Actuate,
    TransitionDelete,
    Purge,
    EraseSource,
    Checkpoint,
}

impl OpKind {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "write" => Ok(Self::Write),
            "actuate" => Ok(Self::Actuate),
            "transition_delete" => Ok(Self::TransitionDelete),
            "purge" => Ok(Self::Purge),
            "erase_source" => Ok(Self::EraseSource),
            "checkpoint" => Ok(Self::Checkpoint),
            other => Err(format!("unknown op_kind: {other}")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Write => "write",
            Self::Actuate => "actuate",
            Self::TransitionDelete => "transition_delete",
            Self::Purge => "purge",
            Self::EraseSource => "erase_source",
            Self::Checkpoint => "checkpoint",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EventStatus {
    InflightOnly,
    Acked,
    Errored,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OracleEvent {
    seq: u64,
    op_kind: OpKind,
    target_id: String,
    status: EventStatus,
    cursor: Option<u64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct OracleTimeline {
    baseline_seq: Option<u64>,
    events: Vec<OracleEvent>,
    ready_for_error_writes: Option<u64>,
    error_writes_observed: Option<(u64, u64)>,
}

impl OracleTimeline {
    fn last_acked_index(&self) -> usize {
        self.events
            .iter()
            .enumerate()
            .rev()
            .find(|(_, ev)| ev.status == EventStatus::Acked)
            .map_or(0, |(idx, _)| idx + 1)
    }

    fn last_acked_write_record_seq(&self) -> u64 {
        let mut max_seq = self.baseline_seq.unwrap_or(0);
        for ev in &self.events {
            if ev.status == EventStatus::Acked
                && matches!(ev.op_kind, OpKind::Write | OpKind::Actuate)
            {
                if let Some(num) = parse_dmf_base_seq(&ev.target_id) {
                    max_seq = max_seq.max(num);
                }
            }
        }
        max_seq
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StoredNodeState {
    Active,
    Deleted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ObservedNode {
    logical_id: String,
    source_id: String,
    state: StoredNodeState,
    body_seq: Option<u64>,
}

#[derive(Clone, Debug)]
struct RunArgs {
    db: PathBuf,
    sync_mode: SyncMode,
    scenario: Scenario,
    oracle_device: String,
    count: Option<u64>,
    scope: String,
    step_delay_ms: u64,
    wait_signal_file: Option<PathBuf>,
    wait_timeout_secs: u64,
    hold_after_ms: u64,
    keep_lock_on_exit: bool,
}

#[derive(Clone, Debug)]
struct VerifyArgs {
    db: PathBuf,
    sync_mode: SyncMode,
    scenario: Scenario,
    oracle_log: PathBuf,
    prev_oracle_log: Option<PathBuf>,
    scope: String,
    allow_drop_writes_rollback: bool,
    skip_post_write: bool,
}

#[derive(Debug, Serialize)]
struct VerifySummary {
    ok: bool,
    scenario: String,
    sync_mode: String,
    matched_prefix_index: usize,
    last_acked_index: usize,
    max_inflight_index: usize,
    active_nodes: usize,
    deleted_nodes: usize,
    stale_lock_present_before_open: bool,
    pending_erasures_before_recovery: usize,
    pending_erasures_after_recovery: usize,
    integrity_check: String,
    data_plane_findings: usize,
    post_recovery_write_ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Eq, PartialEq)]
struct PrefixMatch {
    matched_prefix_index: usize,
    last_acked_index: usize,
    max_inflight_index: usize,
}

enum OracleSink {
    Stdout,
    File { file: File, sync_data: bool },
}

struct OracleWriter {
    sink: OracleSink,
}

impl OracleWriter {
    fn open(target: &str) -> Result<Self, String> {
        if target == "stdout" || target == "-" {
            return Ok(Self { sink: OracleSink::Stdout });
        }
        let path = Path::new(target);
        let is_non_regular = fs::metadata(path).map(|meta| !meta.is_file()).unwrap_or(false);
        let file = if is_non_regular {
            OpenOptions::new()
                .write(true)
                .open(path)
                .map_err(|e| format!("open oracle device {}: {e}", path.display()))?
        } else {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("create oracle dir {}: {e}", parent.display()))?;
                }
            }
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| format!("open oracle file {}: {e}", path.display()))?
        };
        Ok(Self { sink: OracleSink::File { file, sync_data: !is_non_regular } })
    }

    fn emit(&mut self, line: &str) -> Result<(), String> {
        let mut payload = Vec::with_capacity(line.len() + 1);
        payload.extend_from_slice(line.as_bytes());
        payload.push(b'\n');
        match &mut self.sink {
            OracleSink::Stdout => {
                let mut stdout = io::stdout().lock();
                stdout.write_all(&payload).map_err(|e| format!("oracle stdout write: {e}"))?;
                stdout.flush().map_err(|e| format!("oracle stdout flush: {e}"))?;
            }
            OracleSink::File { file, sync_data } => {
                file.write_all(&payload).map_err(|e| format!("oracle write: {e}"))?;
                file.flush().map_err(|e| format!("oracle flush: {e}"))?;
                if *sync_data {
                    let _ = file.sync_data();
                }
            }
        }
        Ok(())
    }
}

fn lock_file_path(db: &Path) -> PathBuf {
    let mut os = db.as_os_str().to_os_string();
    os.push(LOCK_SUFFIX);
    PathBuf::from(os)
}

fn wal_file_path(db: &Path) -> PathBuf {
    let mut os = db.as_os_str().to_os_string();
    os.push("-wal");
    PathBuf::from(os)
}

fn base_id_for_record(record_num: u64) -> String {
    format!("dmf-{record_num:06}")
}

fn source_id_for_record(record_num: u64) -> String {
    format!("src-dmf-{record_num:06}")
}

fn parse_dmf_base_seq(base_id: &str) -> Option<u64> {
    base_id.strip_prefix("dmf-")?.parse::<u64>().ok()
}

fn parse_dmf_node_seq(logical_id: &str) -> Option<(u64, char)> {
    let rest = logical_id.strip_prefix("dmf-")?;
    let (num_str, part) = rest.split_once('-')?;
    let seq = num_str.parse::<u64>().ok()?;
    let ch = match part {
        "a" => 'a',
        "b" => 'b',
        _ => return None,
    };
    Some((seq, ch))
}

fn sanitize_single_line(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | ':' | '.' | '/') {
                c
            } else {
                '_'
            }
        })
        .take(120)
        .collect()
}

fn build_node_body(record_num: u64, part: char, scenario: Scenario, pad_bytes: usize) -> String {
    let pad = "x".repeat(pad_bytes);
    serde_json::json!({
        "seq": record_num,
        "part": part.to_string(),
        "scenario": scenario.as_str(),
        "pad": pad,
    })
    .to_string()
}

fn build_write_pair(
    record_num: u64,
    scenario: Scenario,
    pad_bytes: usize,
) -> Result<[PreparedWrite; 2], String> {
    let base = base_id_for_record(record_num);
    let src = SourceId::new(source_id_for_record(record_num))
        .map_err(|e| format!("invalid source id: {e}"))?;
    let node_a = PreparedWrite::Node {
        kind: NODE_KIND.to_string(),
        body: build_node_body(record_num, 'a', scenario, pad_bytes),
        source_id: src.clone(),
        logical_id: Some(format!("{base}-a")),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
    };
    let node_b = PreparedWrite::Node {
        kind: NODE_KIND.to_string(),
        body: build_node_body(record_num, 'b', scenario, pad_bytes),
        source_id: src,
        logical_id: Some(format!("{base}-b")),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
    };
    Ok([node_a, node_b])
}

fn build_actuate_pair(
    record_num: u64,
    scenario: Scenario,
    pad_bytes: usize,
) -> Result<ActuationBatchV1, String> {
    let base = base_id_for_record(record_num);
    let src = SourceId::new(source_id_for_record(record_num))
        .map_err(|e| format!("invalid source id: {e}"))?;
    let make_node = |part: char| -> Result<ActuationOperationV1, String> {
        let rev = ArtifactRevisionId::new(format!("rev-{base}-{part}"))
            .map_err(|e| format!("invalid revision id: {e}"))?;
        let ver = SourceVersionId::new(format!("ver-{base}-{part}"))
            .map_err(|e| format!("invalid version id: {e}"))?;
        Ok(ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
            kind: NODE_KIND.to_string(),
            body: build_node_body(record_num, part, scenario, pad_bytes),
            source_id: src.clone(),
            logical_id: Some(format!("{base}-{part}")),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: WriteProvenanceV1::canonical(rev, ver),
            properties: None,
        }))
    };
    ActuationBatchV1::new(format!("op-{base}"), vec![make_node('a')?, make_node('b')?])
        .map_err(|e| format!("build actuation batch: {e}"))
}

fn open_engine(
    db: &Path,
    sync_mode: SyncMode,
    scope: &str,
    create_if_missing: bool,
) -> Result<Engine, String> {
    let config = EngineConfig::default()
        .with_sync_mode(sync_mode)
        .with_scope_policy(ScopePolicy::single(scope));
    Engine::open_with_choice_config_and_create(db, EmbedderChoice::None, config, create_if_missing)
        .map(|opened| opened.engine)
        .map_err(|e| format!("Engine::open({}) failed: {e}", db.display()))
}

fn wait_for_signal_file(path: Option<&Path>, timeout_secs: u64) -> Result<(), String> {
    let Some(signal) = path else {
        return Ok(());
    };
    let deadline = Instant::now() + Duration::from_secs(timeout_secs.max(1));
    while Instant::now() < deadline {
        if signal.exists() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(25));
    }
    Err(format!("timed out waiting {}s for signal file {}", timeout_secs, signal.display()))
}

fn maybe_pace(step_delay_ms: u64) {
    if step_delay_ms > 0 {
        thread::sleep(Duration::from_millis(step_delay_ms));
    }
}

fn emit_write_or_actuate(
    engine: &Engine,
    oracle: &mut OracleWriter,
    event_seq: &mut u64,
    record_num: u64,
    scenario: Scenario,
    pad_bytes: usize,
    use_actuate: bool,
) -> Result<bool, String> {
    *event_seq += 1;
    let seq = *event_seq;
    let base = base_id_for_record(record_num);
    let op = if use_actuate { OpKind::Actuate } else { OpKind::Write };
    oracle.emit(&format!("INFLIGHT {seq} {} {base}", op.as_str()))?;
    let outcome = if use_actuate {
        let batch = build_actuate_pair(record_num, scenario, pad_bytes)?;
        engine
            .actuate(batch)
            .map(|r| r.resulting_write_boundary.unwrap_or(0))
            .map_err(|e| format!("{e}"))
    } else {
        let pair = build_write_pair(record_num, scenario, pad_bytes)?;
        engine.write(&pair).map(|r| r.cursor).map_err(|e| format!("{e}"))
    };
    match outcome {
        Ok(cursor) => {
            oracle.emit(&format!("ACK {seq} {cursor} {} {base}", op.as_str()))?;
            Ok(true)
        }
        Err(err) => {
            oracle.emit(&format!(
                "ERR {seq} {} {base} {}",
                op.as_str(),
                sanitize_single_line(&err)
            ))?;
            Ok(false)
        }
    }
}

fn emit_transition_delete(
    engine: &Engine,
    oracle: &mut OracleWriter,
    event_seq: &mut u64,
    logical_id: &str,
) -> Result<bool, String> {
    *event_seq += 1;
    let seq = *event_seq;
    oracle.emit(&format!("INFLIGHT {seq} {} {logical_id}", OpKind::TransitionDelete.as_str()))?;
    match engine.transition(
        logical_id,
        LifecycleState::Deleted,
        Some("dmflakey_soft_delete".to_string()),
    ) {
        Ok(()) => {
            oracle
                .emit(&format!("ACK {seq} 0 {} {logical_id}", OpKind::TransitionDelete.as_str()))?;
            Ok(true)
        }
        Err(err) => {
            oracle.emit(&format!(
                "ERR {seq} {} {logical_id} {}",
                OpKind::TransitionDelete.as_str(),
                sanitize_single_line(&format!("{err}"))
            ))?;
            Ok(false)
        }
    }
}

fn emit_purge(
    engine: &Engine,
    oracle: &mut OracleWriter,
    event_seq: &mut u64,
    logical_id: &str,
) -> Result<bool, String> {
    *event_seq += 1;
    let seq = *event_seq;
    oracle.emit(&format!("INFLIGHT {seq} {} {logical_id}", OpKind::Purge.as_str()))?;
    match engine.purge(logical_id) {
        Ok(()) => {
            oracle.emit(&format!("ACK {seq} 0 {} {logical_id}", OpKind::Purge.as_str()))?;
            Ok(true)
        }
        Err(err) => {
            oracle.emit(&format!(
                "ERR {seq} {} {logical_id} {}",
                OpKind::Purge.as_str(),
                sanitize_single_line(&format!("{err}"))
            ))?;
            Ok(false)
        }
    }
}

fn emit_erase_source(
    engine: &Engine,
    oracle: &mut OracleWriter,
    event_seq: &mut u64,
    source_id: &str,
) -> Result<bool, String> {
    *event_seq += 1;
    let seq = *event_seq;
    oracle.emit(&format!("INFLIGHT {seq} {} {source_id}", OpKind::EraseSource.as_str()))?;
    match engine.erase_source(source_id) {
        Ok(_) => {
            oracle.emit(&format!("ACK {seq} 0 {} {source_id}", OpKind::EraseSource.as_str()))?;
            Ok(true)
        }
        Err(err) => {
            oracle.emit(&format!(
                "ERR {seq} {} {source_id} {}",
                OpKind::EraseSource.as_str(),
                sanitize_single_line(&format!("{err}"))
            ))?;
            Ok(false)
        }
    }
}

fn emit_checkpoint(
    engine: &Engine,
    oracle: &mut OracleWriter,
    event_seq: &mut u64,
) -> Result<bool, String> {
    *event_seq += 1;
    let seq = *event_seq;
    oracle.emit(&format!("INFLIGHT {seq} {} wal", OpKind::Checkpoint.as_str()))?;
    let _ = engine.drain(10_000);
    match engine.truncate_wal() {
        Ok(_) => {
            oracle.emit(&format!("ACK {seq} 0 {} wal", OpKind::Checkpoint.as_str()))?;
            Ok(true)
        }
        Err(err) => {
            oracle.emit(&format!(
                "ERR {seq} {} wal {}",
                OpKind::Checkpoint.as_str(),
                sanitize_single_line(&format!("{err}"))
            ))?;
            Ok(false)
        }
    }
}

fn read_observed_nodes(db: &Path, scope: &str) -> Result<BTreeMap<String, ObservedNode>, String> {
    let conn = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("open read-only sqlite {}: {e}", db.display()))?;
    let mut stmt = conn
        .prepare(
            "SELECT logical_id, source_id, state, body \
             FROM canonical_nodes \
             WHERE scope = ?1 AND superseded_at IS NULL AND logical_id LIKE 'dmf-%' \
             ORDER BY logical_id ASC",
        )
        .map_err(|e| format!("prepare canonical_nodes scan: {e}"))?;
    let mut out = BTreeMap::new();
    let rows = stmt
        .query_map([scope], |row| {
            let logical_id: String = row.get(0)?;
            let source_id: String = row.get(1)?;
            let state_raw: String = row.get(2)?;
            let body: String = row.get(3)?;
            Ok((logical_id, source_id, state_raw, body))
        })
        .map_err(|e| format!("query canonical_nodes: {e}"))?;
    for row in rows {
        let (logical_id, source_id, state_raw, body) =
            row.map_err(|e| format!("read canonical_nodes row: {e}"))?;
        let state = match state_raw.as_str() {
            "active" => StoredNodeState::Active,
            "deleted" => StoredNodeState::Deleted,
            other => {
                return Err(format!("unexpected canonical_nodes state '{other}' for {logical_id}"));
            }
        };
        let body_seq = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v.get("seq").and_then(serde_json::Value::as_u64));
        out.insert(logical_id.clone(), ObservedNode { logical_id, source_id, state, body_seq });
    }
    Ok(out)
}

fn detect_contiguous_baseline_seq(
    observed: &BTreeMap<String, ObservedNode>,
) -> Result<u64, String> {
    let mut seen_a = BTreeMap::new();
    let mut seen_b = BTreeMap::new();
    for (id, node) in observed {
        let Some((seq, part)) = parse_dmf_node_seq(id) else {
            return Err(format!("unexpected logical_id in baseline DB: {id}"));
        };
        if node.state != StoredNodeState::Active {
            return Err(format!("unexpected non-active baseline node: {id}"));
        }
        match part {
            'a' => {
                seen_a.insert(seq, node.body_seq);
            }
            'b' => {
                seen_b.insert(seq, node.body_seq);
            }
            _ => unreachable!(),
        }
    }
    if seen_a.len() != seen_b.len() {
        return Err(format!(
            "baseline has mismatched -a ({}) vs -b ({}) count",
            seen_a.len(),
            seen_b.len()
        ));
    }
    let count = seen_a.len() as u64;
    for seq in 1..=count {
        if seen_a.get(&seq) != Some(&Some(seq)) || seen_b.get(&seq) != Some(&Some(seq)) {
            return Err(format!("baseline gap or body mismatch at seq {seq}"));
        }
    }
    Ok(count)
}

fn run_scenario(args: &RunArgs) -> Result<(), String> {
    let mut oracle = OracleWriter::open(&args.oracle_device)?;
    oracle.emit(&format!(
        "SESSION_START {} {} {}",
        args.scenario.as_str(),
        sync_mode_str(args.sync_mode),
        args.scope
    ))?;

    let create_if_missing = args.scenario != Scenario::ReopenReplay;
    let engine = open_engine(&args.db, args.sync_mode, &args.scope, create_if_missing)?;
    let count = args.count.unwrap_or_else(|| args.scenario.default_count());
    let mut event_seq: u64 = 0;

    match args.scenario {
        Scenario::ActiveCommit => {
            let total = count.max(4);
            let ready_at = (total / 4).max(2);
            for record_num in 1..=total {
                let use_actuate = record_num % 5 == 0;
                let ok = emit_write_or_actuate(
                    &engine,
                    &mut oracle,
                    &mut event_seq,
                    record_num,
                    args.scenario,
                    256,
                    use_actuate,
                )?;
                if !ok {
                    return Err(format!("active_commit write failed at seq {record_num}"));
                }
                if record_num == ready_at {
                    oracle.emit(&format!("READY_FOR_CUT active_commit {event_seq}"))?;
                    wait_for_signal_file(args.wait_signal_file.as_deref(), args.wait_timeout_secs)?;
                }
                if record_num >= ready_at {
                    maybe_pace(args.step_delay_ms);
                }
            }
        }
        Scenario::WalAccum => {
            let total = count.max(4);
            let split = (total / 2).max(2);
            for record_num in 1..=split {
                let ok = emit_write_or_actuate(
                    &engine,
                    &mut oracle,
                    &mut event_seq,
                    record_num,
                    args.scenario,
                    2048,
                    false,
                )?;
                if !ok {
                    return Err(format!("wal_accum baseline write failed at seq {record_num}"));
                }
            }
            let wal_bytes = fs::metadata(wal_file_path(&args.db)).map(|m| m.len()).unwrap_or(0);
            if wal_bytes == 0 {
                return Err("wal_accum expected non-zero -wal file before cut".to_string());
            }
            oracle.emit(&format!("WAL_BYTES {wal_bytes}"))?;
            oracle.emit(&format!("READY_FOR_CUT wal_accum {event_seq}"))?;
            wait_for_signal_file(args.wait_signal_file.as_deref(), args.wait_timeout_secs)?;
            for record_num in (split + 1)..=total {
                let ok = emit_write_or_actuate(
                    &engine,
                    &mut oracle,
                    &mut event_seq,
                    record_num,
                    args.scenario,
                    2048,
                    false,
                )?;
                if !ok {
                    return Err(format!("wal_accum phase2 write failed at seq {record_num}"));
                }
                maybe_pace(args.step_delay_ms);
            }
        }
        Scenario::CheckpointErasure => {
            let total_seed = count.max(8);
            let keep_end = total_seed / 2;
            let rem = total_seed - keep_end;
            let del_count = (rem / 3).max(1);
            let purge_count = (rem / 3).max(1);
            let erase_count = rem.saturating_sub(del_count + purge_count).max(1);
            let actual_seed = keep_end + del_count + purge_count + erase_count;

            for record_num in 1..=actual_seed {
                let ok = emit_write_or_actuate(
                    &engine,
                    &mut oracle,
                    &mut event_seq,
                    record_num,
                    args.scenario,
                    1024,
                    false,
                )?;
                if !ok {
                    return Err(format!("checkpoint_erasure seed write failed at {record_num}"));
                }
            }
            oracle.emit(&format!("READY_FOR_CUT checkpoint_erasure {event_seq}"))?;
            wait_for_signal_file(args.wait_signal_file.as_deref(), args.wait_timeout_secs)?;

            let del_start = keep_end + 1;
            let del_end = keep_end + del_count;
            for r in del_start..=del_end {
                let base = base_id_for_record(r);
                if !emit_transition_delete(
                    &engine,
                    &mut oracle,
                    &mut event_seq,
                    &format!("{base}-a"),
                )? {
                    return Err(format!("checkpoint_erasure transition_delete failed on {base}-a"));
                }
                maybe_pace(args.step_delay_ms);
                if !emit_transition_delete(
                    &engine,
                    &mut oracle,
                    &mut event_seq,
                    &format!("{base}-b"),
                )? {
                    return Err(format!("checkpoint_erasure transition_delete failed on {base}-b"));
                }
                maybe_pace(args.step_delay_ms);
            }

            let purge_start = del_end + 1;
            let purge_end = del_end + purge_count;
            for r in purge_start..=purge_end {
                let base = base_id_for_record(r);
                let id_a = format!("{base}-a");
                let id_b = format!("{base}-b");
                if !emit_transition_delete(&engine, &mut oracle, &mut event_seq, &id_a)? {
                    return Err(format!("checkpoint_erasure pre-purge delete failed on {id_a}"));
                }
                maybe_pace(args.step_delay_ms);
                if !emit_purge(&engine, &mut oracle, &mut event_seq, &id_a)? {
                    return Err(format!("checkpoint_erasure purge failed on {id_a}"));
                }
                maybe_pace(args.step_delay_ms);
                if !emit_transition_delete(&engine, &mut oracle, &mut event_seq, &id_b)? {
                    return Err(format!("checkpoint_erasure pre-purge delete failed on {id_b}"));
                }
                maybe_pace(args.step_delay_ms);
                if !emit_purge(&engine, &mut oracle, &mut event_seq, &id_b)? {
                    return Err(format!("checkpoint_erasure purge failed on {id_b}"));
                }
                maybe_pace(args.step_delay_ms);
            }

            let erase_start = purge_end + 1;
            let erase_end = purge_end + erase_count;
            for r in erase_start..=erase_end {
                let src = source_id_for_record(r);
                if !emit_erase_source(&engine, &mut oracle, &mut event_seq, &src)? {
                    return Err(format!("checkpoint_erasure erase_source failed on {src}"));
                }
                maybe_pace(args.step_delay_ms);
            }

            if !emit_checkpoint(&engine, &mut oracle, &mut event_seq)? {
                return Err("checkpoint_erasure first checkpoint failed".to_string());
            }
            maybe_pace(args.step_delay_ms);

            for r in (actual_seed + 1)..=(actual_seed + 2) {
                if !emit_write_or_actuate(
                    &engine,
                    &mut oracle,
                    &mut event_seq,
                    r,
                    args.scenario,
                    512,
                    false,
                )? {
                    return Err(format!("checkpoint_erasure post-checkpoint write failed at {r}"));
                }
                maybe_pace(args.step_delay_ms);
            }
            if !emit_checkpoint(&engine, &mut oracle, &mut event_seq)? {
                return Err("checkpoint_erasure second checkpoint failed".to_string());
            }
        }
        Scenario::ReopenReplay => {
            let observed = read_observed_nodes(&args.db, &args.scope)?;
            let baseline_seq = detect_contiguous_baseline_seq(&observed)?;
            oracle.emit(&format!("BASELINE {baseline_seq}"))?;
            let total = count.max(4);
            let ready_at = (total / 4).max(2);
            for idx in 1..=total {
                let record_num = baseline_seq + idx;
                let ok = emit_write_or_actuate(
                    &engine,
                    &mut oracle,
                    &mut event_seq,
                    record_num,
                    args.scenario,
                    512,
                    idx % 4 == 0,
                )?;
                if !ok {
                    return Err(format!("reopen_replay write failed at record {record_num}"));
                }
                if idx == ready_at {
                    oracle.emit(&format!("READY_FOR_CUT reopen_replay {event_seq}"))?;
                    wait_for_signal_file(args.wait_signal_file.as_deref(), args.wait_timeout_secs)?;
                }
                if idx >= ready_at {
                    maybe_pace(args.step_delay_ms);
                }
            }
        }
        Scenario::ErasureLockFault => {
            let total_seed = count.max(8);
            let keep_end = total_seed / 2;
            let rem = total_seed - keep_end;
            let del_count = (rem / 3).max(1);
            let purge_count = (rem / 3).max(2);
            let erase_count = rem.saturating_sub(del_count + purge_count).max(2);
            let actual_seed = keep_end + del_count + purge_count + erase_count;

            for r in 1..=actual_seed {
                let ok = emit_write_or_actuate(
                    &engine,
                    &mut oracle,
                    &mut event_seq,
                    r,
                    args.scenario,
                    1024,
                    false,
                )?;
                if !ok {
                    return Err(format!("erasure_lock_fault seed write failed at {r}"));
                }
            }

            let del_start = keep_end + 1;
            let del_end = keep_end + del_count;
            for r in del_start..=del_end {
                let base = base_id_for_record(r);
                emit_transition_delete(&engine, &mut oracle, &mut event_seq, &format!("{base}-a"))?;
                emit_transition_delete(&engine, &mut oracle, &mut event_seq, &format!("{base}-b"))?;
            }

            let purge_start = del_end + 1;
            let purge_mid = purge_start + (purge_count / 2);
            let purge_end = del_end + purge_count;
            for r in purge_start..purge_mid {
                let base = base_id_for_record(r);
                let id_a = format!("{base}-a");
                let id_b = format!("{base}-b");
                emit_transition_delete(&engine, &mut oracle, &mut event_seq, &id_a)?;
                emit_purge(&engine, &mut oracle, &mut event_seq, &id_a)?;
                emit_transition_delete(&engine, &mut oracle, &mut event_seq, &id_b)?;
                emit_purge(&engine, &mut oracle, &mut event_seq, &id_b)?;
            }

            let erase_start = purge_end + 1;
            let erase_mid = erase_start + (erase_count / 2);
            let erase_end = purge_end + erase_count;
            for r in erase_start..erase_mid {
                let src = source_id_for_record(r);
                emit_erase_source(&engine, &mut oracle, &mut event_seq, &src)?;
            }

            oracle.emit(&format!("READY_FOR_CUT erasure_lock_fault {event_seq}"))?;
            wait_for_signal_file(args.wait_signal_file.as_deref(), args.wait_timeout_secs)?;

            for r in purge_mid..=purge_end {
                let base = base_id_for_record(r);
                let id_a = format!("{base}-a");
                let id_b = format!("{base}-b");
                emit_transition_delete(&engine, &mut oracle, &mut event_seq, &id_a)?;
                maybe_pace(args.step_delay_ms);
                emit_purge(&engine, &mut oracle, &mut event_seq, &id_a)?;
                maybe_pace(args.step_delay_ms);
                emit_transition_delete(&engine, &mut oracle, &mut event_seq, &id_b)?;
                maybe_pace(args.step_delay_ms);
                emit_purge(&engine, &mut oracle, &mut event_seq, &id_b)?;
                maybe_pace(args.step_delay_ms);
            }
            for r in erase_mid..=erase_end {
                let src = source_id_for_record(r);
                emit_erase_source(&engine, &mut oracle, &mut event_seq, &src)?;
                maybe_pace(args.step_delay_ms);
            }
            emit_checkpoint(&engine, &mut oracle, &mut event_seq)?;
        }
        Scenario::ErrorWritesFull => {
            let baseline = count.max(4);
            for r in 1..=baseline {
                let ok = emit_write_or_actuate(
                    &engine,
                    &mut oracle,
                    &mut event_seq,
                    r,
                    args.scenario,
                    512,
                    false,
                )?;
                if !ok {
                    return Err(format!("error_writes_full baseline write failed at {r}"));
                }
            }
            oracle.emit(&format!("READY_FOR_ERROR_WRITES {event_seq}"))?;
            wait_for_signal_file(args.wait_signal_file.as_deref(), args.wait_timeout_secs)?;

            let mut err_count: u64 = 0;
            let mut ok_count: u64 = 0;
            for r in (baseline + 1)..=(baseline + 4) {
                let ok = emit_write_or_actuate(
                    &engine,
                    &mut oracle,
                    &mut event_seq,
                    r,
                    args.scenario,
                    1024,
                    false,
                )?;
                if ok {
                    ok_count += 1;
                } else {
                    err_count += 1;
                }
                maybe_pace(args.step_delay_ms);
            }
            oracle.emit(&format!("ERROR_WRITES_OBSERVED {err_count} {ok_count}"))?;
            if args.wait_signal_file.is_some() && (err_count == 0 || ok_count > 0) {
                return Err(format!(
                    "error_writes_full expected all post-fault writes to fail closed, observed err={err_count} ok={ok_count}"
                ));
            }
        }
    }

    if args.hold_after_ms > 0 {
        thread::sleep(Duration::from_millis(args.hold_after_ms));
    }
    oracle.emit(&format!("SCENARIO_COMPLETE {} {event_seq}", args.scenario.as_str()))?;
    let _ = engine.close();
    if args.keep_lock_on_exit {
        // Leave an unlocked stale `.lock` sidecar on disk to mirror hard power-off
        // in single-process unit tests without retaining an active in-process flock.
        let lock_path = lock_file_path(&args.db);
        fs::write(&lock_path, "999999\n")
            .map_err(|e| format!("write simulated stale lock {}: {e}", lock_path.display()))?;
    }
    Ok(())
}

fn parse_oracle_log(raw: &str) -> Result<OracleTimeline, String> {
    let cleaned = raw.replace('\0', "");
    let raw = cleaned.as_str();
    let mut timeline = OracleTimeline::default();
    let mut by_seq: BTreeMap<u64, usize> = BTreeMap::new();
    let lines: Vec<&str> = raw.split('\n').collect();
    let complete_line_count = if raw.ends_with('\n') {
        lines.len().saturating_sub(1)
    } else {
        // If the host killed QEMU mid-line without a trailing newline, parse complete
        // lines and tolerate a single truncated final fragment only if it fails parse.
        lines.len()
    };

    for (idx, line) in lines.iter().take(complete_line_count).enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let is_unterminated_last = !raw.ends_with('\n') && idx + 1 == complete_line_count;
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        match parts.as_slice() {
            ["SESSION_START", _, _, _]
            | ["READY_FOR_CUT", _, _]
            | ["SCENARIO_COMPLETE", _, _]
            | ["WAL_BYTES", _] => {}
            ["BASELINE", raw_seq] => {
                let b = raw_seq
                    .parse::<u64>()
                    .map_err(|e| format!("invalid BASELINE seq '{raw_seq}': {e}"))?;
                timeline.baseline_seq = Some(b);
            }
            ["READY_FOR_ERROR_WRITES", raw_seq] => {
                let s = raw_seq
                    .parse::<u64>()
                    .map_err(|e| format!("invalid READY_FOR_ERROR_WRITES '{raw_seq}': {e}"))?;
                timeline.ready_for_error_writes = Some(s);
            }
            ["ERROR_WRITES_OBSERVED", raw_errs, raw_oks] => {
                let errs = raw_errs
                    .parse::<u64>()
                    .map_err(|e| format!("invalid error count '{raw_errs}': {e}"))?;
                let oks = raw_oks
                    .parse::<u64>()
                    .map_err(|e| format!("invalid ok count '{raw_oks}': {e}"))?;
                timeline.error_writes_observed = Some((errs, oks));
            }
            ["INFLIGHT", raw_seq, raw_op, target_id] => {
                let seq = raw_seq
                    .parse::<u64>()
                    .map_err(|e| format!("invalid INFLIGHT seq '{raw_seq}': {e}"))?;
                let op_kind = OpKind::parse(raw_op)?;
                let pos = timeline.events.len();
                by_seq.insert(seq, pos);
                timeline.events.push(OracleEvent {
                    seq,
                    op_kind,
                    target_id: (*target_id).to_string(),
                    status: EventStatus::InflightOnly,
                    cursor: None,
                });
            }
            ["ACK", raw_seq, raw_cursor, raw_op, target_id] => {
                let seq = raw_seq
                    .parse::<u64>()
                    .map_err(|e| format!("invalid ACK seq '{raw_seq}': {e}"))?;
                let cursor = raw_cursor
                    .parse::<u64>()
                    .map_err(|e| format!("invalid ACK cursor '{raw_cursor}': {e}"))?;
                let op_kind = OpKind::parse(raw_op)?;
                let Some(&pos) = by_seq.get(&seq) else {
                    return Err(format!("ACK for seq {seq} without preceding INFLIGHT"));
                };
                let ev = &mut timeline.events[pos];
                if ev.op_kind != op_kind || ev.target_id != *target_id {
                    return Err(format!(
                        "ACK mismatch for seq {seq}: expected ({:?}, {}), got ({op_kind:?}, {target_id})",
                        ev.op_kind, ev.target_id
                    ));
                }
                ev.status = EventStatus::Acked;
                ev.cursor = Some(cursor);
            }
            ["ERR", raw_seq, raw_op, target_id, ..] => {
                let seq = raw_seq
                    .parse::<u64>()
                    .map_err(|e| format!("invalid ERR seq '{raw_seq}': {e}"))?;
                let op_kind = OpKind::parse(raw_op)?;
                let Some(&pos) = by_seq.get(&seq) else {
                    return Err(format!("ERR for seq {seq} without preceding INFLIGHT"));
                };
                let ev = &mut timeline.events[pos];
                if ev.op_kind != op_kind || ev.target_id != *target_id {
                    return Err(format!("ERR mismatch for seq {seq}"));
                }
                ev.status = EventStatus::Errored;
            }
            _ if is_unterminated_last => {
                // Tolerate a single truncated final line if QEMU died mid-write.
            }
            _ => {
                return Err(format!("unrecognized oracle line {}: '{trimmed}'", idx + 1));
            }
        }
    }
    Ok(timeline)
}

fn apply_event(state: &mut BTreeMap<String, (String, StoredNodeState, u64)>, ev: &OracleEvent) {
    if ev.status == EventStatus::Errored {
        return;
    }
    match ev.op_kind {
        OpKind::Write | OpKind::Actuate => {
            if let Some(seq_num) = parse_dmf_base_seq(&ev.target_id) {
                let src = source_id_for_record(seq_num);
                state.insert(
                    format!("{}-a", ev.target_id),
                    (src.clone(), StoredNodeState::Active, seq_num),
                );
                state
                    .insert(format!("{}-b", ev.target_id), (src, StoredNodeState::Active, seq_num));
            }
        }
        OpKind::TransitionDelete => {
            if let Some(entry) = state.get_mut(&ev.target_id) {
                entry.1 = StoredNodeState::Deleted;
            }
        }
        OpKind::Purge => {
            state.remove(&ev.target_id);
        }
        OpKind::EraseSource => {
            state.retain(|_, (src, _, _)| src != &ev.target_id);
        }
        OpKind::Checkpoint => {}
    }
}

fn states_match(
    expected: &BTreeMap<String, (String, StoredNodeState, u64)>,
    observed: &BTreeMap<String, ObservedNode>,
) -> bool {
    if expected.len() != observed.len() {
        return false;
    }
    for (id, (exp_src, exp_state, exp_seq)) in expected {
        let Some(obs) = observed.get(id) else {
            return false;
        };
        if obs.source_id != *exp_src || obs.state != *exp_state || obs.body_seq != Some(*exp_seq) {
            return false;
        }
    }
    true
}

fn verify_state_against_oracle(
    sync_mode: SyncMode,
    scenario: Scenario,
    timeline: &OracleTimeline,
    prev_timeline: Option<&OracleTimeline>,
    observed: &BTreeMap<String, ObservedNode>,
    allow_drop_writes_rollback: bool,
) -> Result<PrefixMatch, String> {
    // Special case: reopen_replay crashed during Engine::open before emitting BASELINE.
    if scenario == Scenario::ReopenReplay
        && timeline.baseline_seq.is_none()
        && timeline.events.is_empty()
    {
        if let Some(prev) = prev_timeline {
            return verify_state_against_oracle(
                sync_mode,
                Scenario::WalAccum,
                prev,
                None,
                observed,
                allow_drop_writes_rollback,
            );
        }
    }

    // Validate phase-1 baseline against prev_timeline when provided for reopen_replay.
    if let (Some(baseline), Some(prev)) = (timeline.baseline_seq, prev_timeline) {
        let prev_max_inflight =
            prev.events.iter().filter_map(|e| parse_dmf_base_seq(&e.target_id)).max().unwrap_or(0);
        if baseline > prev_max_inflight {
            return Err(format!(
                "reopen_replay baseline {baseline} exceeds phase-1 max inflight {prev_max_inflight}"
            ));
        }
        if sync_mode == SyncMode::Full {
            let prev_acked = prev.last_acked_write_record_seq();
            if baseline < prev_acked {
                return Err(format!(
                    "reopen_replay baseline {baseline} lost phase-1 SyncMode::Full ACK {prev_acked}"
                ));
            }
        }
    }

    // For error_writes_full, verify post-fault writes failed closed.
    if scenario == Scenario::ErrorWritesFull {
        if let Some(ready_seq) = timeline.ready_for_error_writes {
            for ev in &timeline.events {
                if ev.seq > ready_seq && ev.status == EventStatus::Acked {
                    return Err(format!(
                        "error_writes_full saw unexpected ACK at seq {} after READY_FOR_ERROR_WRITES {}",
                        ev.seq, ready_seq
                    ));
                }
            }
        }
        if let Some((errs, oks)) = timeline.error_writes_observed {
            if errs == 0 || oks > 0 {
                return Err(format!(
                    "error_writes_full expected err>0 and ok==0, observed err={errs} ok={oks}"
                ));
            }
        }
    }

    // Build State(0) from BASELINE (if present) and simulate k = 0..=events.len().
    let mut state: BTreeMap<String, (String, StoredNodeState, u64)> = BTreeMap::new();
    if let Some(baseline) = timeline.baseline_seq {
        for r in 1..=baseline {
            let base = base_id_for_record(r);
            let src = source_id_for_record(r);
            state.insert(format!("{base}-a"), (src.clone(), StoredNodeState::Active, r));
            state.insert(format!("{base}-b"), (src, StoredNodeState::Active, r));
        }
    }

    let last_acked_index = timeline.last_acked_index();
    let max_inflight_index = timeline.events.len();
    let mut matching_indices = Vec::new();

    if states_match(&state, observed) {
        matching_indices.push(0_usize);
    }
    for (idx, ev) in timeline.events.iter().enumerate() {
        apply_event(&mut state, ev);
        if states_match(&state, observed) {
            matching_indices.push(idx + 1);
        }
    }

    // Choose the highest matching prefix index (handles no-op events like checkpoint/ERR cleanly).
    let Some(&matched_prefix_index) = matching_indices.last() else {
        return Err(format!(
            "recovered DB state ({} nodes) does not match ANY contiguous prefix 0..={max_inflight_index} of external oracle (gap, partial batch, resurrection, or corruption)",
            observed.len()
        ));
    };

    if sync_mode == SyncMode::Full && matched_prefix_index < last_acked_index {
        return Err(format!(
            "SyncMode::Full LOST ACK-recorded commit(s): matched prefix {matched_prefix_index} < last_acked_index {last_acked_index}"
        ));
    }

    // AC32-DMFLAKEY-04: unless drop_writes explicitly rolled back a Normal run,
    // no ACK'd purge or erase_source may ever resurrect (both execute
    // wal_checkpoint(TRUNCATE) + fsync before ACK under both Normal and Full).
    if !allow_drop_writes_rollback
        && matches!(scenario, Scenario::ErasureLockFault | Scenario::CheckpointErasure)
    {
        for ev in &timeline.events {
            if ev.status != EventStatus::Acked {
                continue;
            }
            match ev.op_kind {
                OpKind::Purge if observed.contains_key(&ev.target_id) => {
                    return Err(format!("purged node {} resurrected after reboot", ev.target_id));
                }
                OpKind::EraseSource if observed.values().any(|n| n.source_id == ev.target_id) => {
                    return Err(format!("erased source {} resurrected after reboot", ev.target_id));
                }
                OpKind::TransitionDelete
                    if sync_mode == SyncMode::Full
                        && matches!(
                            observed.get(&ev.target_id).map(|n| n.state),
                            Some(StoredNodeState::Active)
                        ) =>
                {
                    return Err(format!(
                        "soft-deleted node {} resurrected as active",
                        ev.target_id
                    ));
                }
                _ => {}
            }
        }
    }

    Ok(PrefixMatch { matched_prefix_index, last_acked_index, max_inflight_index })
}

fn verify_database(args: &VerifyArgs) -> Result<VerifySummary, String> {
    let stale_lock_present_before_open = lock_file_path(&args.db).exists();
    let raw_oracle = fs::read_to_string(&args.oracle_log)
        .map_err(|e| format!("read oracle log {}: {e}", args.oracle_log.display()))?;
    let timeline = parse_oracle_log(&raw_oracle)?;
    let prev_timeline = match &args.prev_oracle_log {
        Some(path) => {
            let raw_prev = fs::read_to_string(path)
                .map_err(|e| format!("read prev oracle log {}: {e}", path.display()))?;
            Some(parse_oracle_log(&raw_prev)?)
        }
        None => None,
    };

    let mut engine = open_engine(&args.db, args.sync_mode, &args.scope, false)?;

    let pending_before = engine
        .pending_erasures()
        .map_err(|e| format!("pending_erasures before recovery failed: {e}"))?
        .len();
    if pending_before > 0 {
        engine.close().map_err(|e| format!("close before recover_complete_erasures: {e}"))?;
        recover_complete_erasures(&args.db)
            .map_err(|e| format!("recover_complete_erasures failed: {e}"))?;
        engine = open_engine(&args.db, args.sync_mode, &args.scope, false)?;
    }
    let pending_after = engine
        .pending_erasures()
        .map_err(|e| format!("pending_erasures after recovery failed: {e}"))?
        .len();
    if pending_after > 0 {
        return Err(format!("pending_erasures remained non-zero ({pending_after}) after recovery"));
    }

    let integrity = engine
        .check_integrity(CheckIntegrityOpts { quick: false, full: true, round_trip: false })
        .map_err(|e| format!("check_integrity(--full) failed: {e}"))?;
    if integrity.physical != Section::Clean
        || integrity.logical != Section::Clean
        || integrity.semantic != Section::Clean
    {
        return Err(format!(
            "check_integrity(--full) reported findings: physical={:?} logical={:?} semantic={:?}",
            integrity.physical, integrity.logical, integrity.semantic
        ));
    }

    let observed = read_observed_nodes(&args.db, &args.scope)?;

    // Cross-check Engine::read_get agrees with observed active/deleted state.
    let view = ReadView::default();
    for (id, node) in &observed {
        let fetched = engine
            .read_get(id, &view)
            .map_err(|e| format!("Engine::read_get({id}) failed: {e}"))?;
        match node.state {
            StoredNodeState::Active => {
                if fetched.is_none() {
                    return Err(format!("active node {id} missing from Engine::read_get"));
                }
            }
            StoredNodeState::Deleted => {
                if fetched.is_some() {
                    return Err(format!("deleted node {id} resurrected in Engine::read_get"));
                }
            }
        }
    }

    let prefix_match = verify_state_against_oracle(
        args.sync_mode,
        args.scenario,
        &timeline,
        prev_timeline.as_ref(),
        &observed,
        args.allow_drop_writes_rollback,
    )?;

    let active_nodes = observed.values().filter(|n| n.state == StoredNodeState::Active).count();
    let deleted_nodes = observed.values().filter(|n| n.state == StoredNodeState::Deleted).count();

    let mut post_recovery_write_ok = true;
    if !args.skip_post_write {
        let probe_id = "dmf-post-verify-probe";
        let probe_src = SourceId::new("src-dmf-post-verify-probe")
            .map_err(|e| format!("probe source_id: {e}"))?;
        engine
            .write(&[PreparedWrite::Node {
                kind: NODE_KIND.to_string(),
                body: "{\"probe\":\"post_recovery\"}".to_string(),
                source_id: probe_src,
                logical_id: Some(probe_id.to_string()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            }])
            .map_err(|e| format!("post-recovery write failed: {e}"))?;
        if engine
            .read_get(probe_id, &view)
            .map_err(|e| format!("post-recovery read_get failed: {e}"))?
            .is_none()
        {
            return Err("post-recovery probe node not readable after write".to_string());
        }
        engine
            .transition(probe_id, LifecycleState::Deleted, Some("post_verify_cleanup".to_string()))
            .map_err(|e| format!("post-recovery transition(Deleted) failed: {e}"))?;
        engine.purge(probe_id).map_err(|e| format!("post-recovery purge failed: {e}"))?;
        if engine
            .read_get(probe_id, &view)
            .map_err(|e| format!("post-recovery read_get after purge failed: {e}"))?
            .is_some()
        {
            return Err("post-recovery probe node still readable after purge".to_string());
        }
        engine.drain(10_000).map_err(|e| format!("post-recovery drain failed: {e}"))?;
        post_recovery_write_ok = true;
    }

    engine.close().map_err(|e| format!("Engine::close failed: {e}"))?;

    let dpi_req = DataPlaneIntegrityRequestV1::all(10_000, 100)
        .map_err(|e| format!("build data-plane-integrity request: {e}"))?;
    let dpi = inspect_data_plane_integrity(&args.db, dpi_req)
        .map_err(|e| format!("inspect_data_plane_integrity failed: {e}"))?;
    if !dpi.findings.is_empty() {
        return Err(format!(
            "data-plane-integrity reported {} finding(s): {:?}",
            dpi.findings.len(),
            dpi.findings
        ));
    }

    Ok(VerifySummary {
        ok: true,
        scenario: args.scenario.as_str().to_string(),
        sync_mode: sync_mode_str(args.sync_mode).to_string(),
        matched_prefix_index: prefix_match.matched_prefix_index,
        last_acked_index: prefix_match.last_acked_index,
        max_inflight_index: prefix_match.max_inflight_index,
        active_nodes,
        deleted_nodes,
        stale_lock_present_before_open,
        pending_erasures_before_recovery: pending_before,
        pending_erasures_after_recovery: pending_after,
        integrity_check: "ok".to_string(),
        data_plane_findings: dpi.findings.len(),
        post_recovery_write_ok,
        error: None,
    })
}

fn next_arg(args: &[String], idx: &mut usize, flag: &str) -> Result<String, String> {
    *idx += 1;
    args.get(*idx).cloned().ok_or_else(|| format!("missing value for {flag}"))
}

fn parse_run_args(args: &[String]) -> Result<RunArgs, String> {
    let mut db: Option<PathBuf> = None;
    let mut sync_mode: Option<SyncMode> = None;
    let mut scenario: Option<Scenario> = None;
    let mut oracle_device: Option<String> = None;
    let mut count: Option<u64> = None;
    let mut scope = DEFAULT_SCOPE.to_string();
    let mut step_delay_ms: u64 = 0;
    let mut wait_signal_file: Option<PathBuf> = None;
    let mut wait_timeout_secs: u64 = 60;
    let mut hold_after_ms: u64 = 0;
    let mut keep_lock_on_exit = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--db" => db = Some(PathBuf::from(next_arg(args, &mut i, "--db")?)),
            "--sync-mode" => {
                sync_mode = Some(parse_sync_mode(&next_arg(args, &mut i, "--sync-mode")?)?)
            }
            "--scenario" => {
                scenario = Some(Scenario::parse(&next_arg(args, &mut i, "--scenario")?)?)
            }
            "--oracle-device" => oracle_device = Some(next_arg(args, &mut i, "--oracle-device")?),
            "--count" => {
                let raw = next_arg(args, &mut i, "--count")?;
                count =
                    Some(raw.parse::<u64>().map_err(|e| format!("invalid --count '{raw}': {e}"))?);
            }
            "--scope" => scope = next_arg(args, &mut i, "--scope")?,
            "--step-delay-ms" => {
                let raw = next_arg(args, &mut i, "--step-delay-ms")?;
                step_delay_ms = raw
                    .parse::<u64>()
                    .map_err(|e| format!("invalid --step-delay-ms '{raw}': {e}"))?;
            }
            "--wait-signal-file" => {
                wait_signal_file =
                    Some(PathBuf::from(next_arg(args, &mut i, "--wait-signal-file")?));
            }
            "--wait-timeout-secs" => {
                let raw = next_arg(args, &mut i, "--wait-timeout-secs")?;
                wait_timeout_secs = raw
                    .parse::<u64>()
                    .map_err(|e| format!("invalid --wait-timeout-secs '{raw}': {e}"))?;
            }
            "--hold-after-ms" => {
                let raw = next_arg(args, &mut i, "--hold-after-ms")?;
                hold_after_ms = raw
                    .parse::<u64>()
                    .map_err(|e| format!("invalid --hold-after-ms '{raw}': {e}"))?;
            }
            "--keep-lock-on-exit" => keep_lock_on_exit = true,
            other => return Err(format!("unknown run flag: {other}")),
        }
        i += 1;
    }

    Ok(RunArgs {
        db: db.ok_or("missing required --db")?,
        sync_mode: sync_mode.ok_or("missing required --sync-mode")?,
        scenario: scenario.ok_or("missing required --scenario")?,
        oracle_device: oracle_device.ok_or("missing required --oracle-device")?,
        count,
        scope,
        step_delay_ms,
        wait_signal_file,
        wait_timeout_secs,
        hold_after_ms,
        keep_lock_on_exit,
    })
}

fn parse_verify_args(args: &[String]) -> Result<VerifyArgs, String> {
    let mut db: Option<PathBuf> = None;
    let mut sync_mode: Option<SyncMode> = None;
    let mut scenario: Option<Scenario> = None;
    let mut oracle_log: Option<PathBuf> = None;
    let mut prev_oracle_log: Option<PathBuf> = None;
    let mut scope = DEFAULT_SCOPE.to_string();
    let mut allow_drop_writes_rollback = false;
    let mut skip_post_write = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--db" => db = Some(PathBuf::from(next_arg(args, &mut i, "--db")?)),
            "--sync-mode" => {
                sync_mode = Some(parse_sync_mode(&next_arg(args, &mut i, "--sync-mode")?)?)
            }
            "--scenario" => {
                scenario = Some(Scenario::parse(&next_arg(args, &mut i, "--scenario")?)?)
            }
            "--oracle-log" => {
                oracle_log = Some(PathBuf::from(next_arg(args, &mut i, "--oracle-log")?))
            }
            "--prev-oracle-log" => {
                prev_oracle_log = Some(PathBuf::from(next_arg(args, &mut i, "--prev-oracle-log")?));
            }
            "--scope" => scope = next_arg(args, &mut i, "--scope")?,
            "--allow-drop-writes-rollback" => allow_drop_writes_rollback = true,
            "--skip-post-write" => skip_post_write = true,
            other => return Err(format!("unknown verify flag: {other}")),
        }
        i += 1;
    }

    Ok(VerifyArgs {
        db: db.ok_or("missing required --db")?,
        sync_mode: sync_mode.ok_or("missing required --sync-mode")?,
        scenario: scenario.ok_or("missing required --scenario")?,
        oracle_log: oracle_log.ok_or("missing required --oracle-log")?,
        prev_oracle_log,
        scope,
        allow_drop_writes_rollback,
        skip_post_write,
    })
}

fn usage() -> &'static str {
    "Usage:\n  \
     fathomdb-dmflakey-harness run --db <path> --sync-mode <normal|full> --scenario <active_commit|wal_accum|checkpoint_erasure|reopen_replay|erasure_lock_fault|error_writes_full> --oracle-device <path|stdout> [--count N] [--scope NAME] [--step-delay-ms MS] [--wait-signal-file PATH] [--wait-timeout-secs SECS] [--hold-after-ms MS] [--keep-lock-on-exit]\n  \
     fathomdb-dmflakey-harness verify --db <path> --sync-mode <normal|full> --scenario <name> --oracle-log <path> [--prev-oracle-log PATH] [--scope NAME] [--allow-drop-writes-rollback] [--skip-post-write]"
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(subcmd) = args.first().map(String::as_str) else {
        eprintln!("{}", usage());
        return ExitCode::from(2);
    };
    match subcmd {
        "-h" | "--help" => {
            println!("{}", usage());
            ExitCode::SUCCESS
        }
        "run" => match parse_run_args(&args[1..]).and_then(|a| run_scenario(&a)) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("ERROR(run): {err}");
                ExitCode::FAILURE
            }
        },
        "verify" => match parse_verify_args(&args[1..]) {
            Err(err) => {
                eprintln!("ERROR(verify-args): {err}");
                ExitCode::from(2)
            }
            Ok(vargs) => match verify_database(&vargs) {
                Ok(summary) => {
                    let json = serde_json::to_string(&summary)
                        .unwrap_or_else(|_| "{\"ok\":true}".to_string());
                    println!("VERIFY_RESULT {json}");
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    let failed = VerifySummary {
                        ok: false,
                        scenario: vargs.scenario.as_str().to_string(),
                        sync_mode: sync_mode_str(vargs.sync_mode).to_string(),
                        matched_prefix_index: 0,
                        last_acked_index: 0,
                        max_inflight_index: 0,
                        active_nodes: 0,
                        deleted_nodes: 0,
                        stale_lock_present_before_open: lock_file_path(&vargs.db).exists(),
                        pending_erasures_before_recovery: 0,
                        pending_erasures_after_recovery: 0,
                        integrity_check: "failed".to_string(),
                        data_plane_findings: 0,
                        post_recovery_write_ok: false,
                        error: Some(err.clone()),
                    };
                    let json = serde_json::to_string(&failed)
                        .unwrap_or_else(|_| "{\"ok\":false}".to_string());
                    println!("VERIFY_RESULT {json}");
                    eprintln!("ERROR(verify): {err}");
                    ExitCode::FAILURE
                }
            },
        },
        other => {
            eprintln!("unknown subcommand: {other}\n{}", usage());
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn pair_nodes(seq: u64, state: StoredNodeState) -> Vec<(String, ObservedNode)> {
        let base = format!("dmf-{seq:06}");
        let src = format!("src-{base}");
        vec![
            (
                format!("{base}-a"),
                ObservedNode {
                    logical_id: format!("{base}-a"),
                    source_id: src.clone(),
                    state,
                    body_seq: Some(seq),
                },
            ),
            (
                format!("{base}-b"),
                ObservedNode {
                    logical_id: format!("{base}-b"),
                    source_id: src,
                    state,
                    body_seq: Some(seq),
                },
            ),
        ]
    }

    #[test]
    fn normal_accepts_exact_contiguous_prefix_and_rejects_gaps_and_partial_commits() {
        let log = "\
SESSION_START active_commit normal tenant-flakey
INFLIGHT 1 write dmf-000001
ACK 1 2 write dmf-000001
INFLIGHT 2 write dmf-000002
ACK 2 4 write dmf-000002
INFLIGHT 3 write dmf-000003
ACK 3 6 write dmf-000003
INFLIGHT 4 write dmf-000004
";
        let timeline = parse_oracle_log(log).expect("parse oracle log");
        assert_eq!(timeline.events.len(), 4);

        // Prefix 1..=2 under Normal is valid (commits 3 and 4 rolled back atomically).
        let mut observed = BTreeMap::new();
        for seq in 1..=2 {
            for (k, v) in pair_nodes(seq, StoredNodeState::Active) {
                observed.insert(k, v);
            }
        }
        let matched = verify_state_against_oracle(
            SyncMode::Normal,
            Scenario::ActiveCommit,
            &timeline,
            None,
            &observed,
            false,
        )
        .expect("normal prefix 1..=2 must pass");
        assert_eq!(matched.matched_prefix_index, 2);

        // Out-of-order gap (1 and 3 present, 2 missing) MUST fail.
        let mut gapped = BTreeMap::new();
        for seq in [1_u64, 3_u64] {
            for (k, v) in pair_nodes(seq, StoredNodeState::Active) {
                gapped.insert(k, v);
            }
        }
        assert!(
            verify_state_against_oracle(
                SyncMode::Normal,
                Scenario::ActiveCommit,
                &timeline,
                None,
                &gapped,
                false,
            )
            .is_err(),
            "out-of-order gap must fail"
        );

        // Partial multi-op commit (-a present without -b) MUST fail.
        let mut partial = observed.clone();
        partial.insert(
            "dmf-000003-a".to_string(),
            ObservedNode {
                logical_id: "dmf-000003-a".to_string(),
                source_id: "src-dmf-000003".to_string(),
                state: StoredNodeState::Active,
                body_seq: Some(3),
            },
        );
        assert!(
            verify_state_against_oracle(
                SyncMode::Normal,
                Scenario::ActiveCommit,
                &timeline,
                None,
                &partial,
                false,
            )
            .is_err(),
            "partial batch commit must fail"
        );
    }

    #[test]
    fn full_rejects_lost_ack_and_allows_atomic_inflight_commit_or_rollback() {
        let log = "\
SESSION_START active_commit full tenant-flakey
INFLIGHT 1 write dmf-000001
ACK 1 2 write dmf-000001
INFLIGHT 2 write dmf-000002
ACK 2 4 write dmf-000002
INFLIGHT 3 write dmf-000003
";
        let timeline = parse_oracle_log(log).expect("parse oracle log");

        // Lost ACK #2 (only seq 1 survived) MUST FAIL under SyncMode::Full.
        let mut lost_ack = BTreeMap::new();
        for (k, v) in pair_nodes(1, StoredNodeState::Active) {
            lost_ack.insert(k, v);
        }
        assert!(
            verify_state_against_oracle(
                SyncMode::Full,
                Scenario::ActiveCommit,
                &timeline,
                None,
                &lost_ack,
                false,
            )
            .is_err(),
            "lost ACK under SyncMode::Full must fail"
        );

        // Exact last_acked (1..=2) passes under SyncMode::Full.
        let mut exact_acked = lost_ack.clone();
        for (k, v) in pair_nodes(2, StoredNodeState::Active) {
            exact_acked.insert(k, v);
        }
        let m2 = verify_state_against_oracle(
            SyncMode::Full,
            Scenario::ActiveCommit,
            &timeline,
            None,
            &exact_acked,
            false,
        )
        .expect("exact acked prefix must pass under Full");
        assert_eq!(m2.matched_prefix_index, 2);

        // Atomic commit of the single trailing INFLIGHT #3 (1..=3) also passes.
        let mut with_inflight = exact_acked;
        for (k, v) in pair_nodes(3, StoredNodeState::Active) {
            with_inflight.insert(k, v);
        }
        let m3 = verify_state_against_oracle(
            SyncMode::Full,
            Scenario::ActiveCommit,
            &timeline,
            None,
            &with_inflight,
            false,
        )
        .expect("atomic trailing inflight commit must pass under Full");
        assert_eq!(m3.matched_prefix_index, 3);
    }

    #[test]
    fn erasure_resurrection_is_rejected() {
        let log = "\
SESSION_START erasure_lock_fault full tenant-flakey
INFLIGHT 1 write dmf-000001
ACK 1 2 write dmf-000001
INFLIGHT 2 write dmf-000002
ACK 2 4 write dmf-000002
INFLIGHT 3 purge dmf-000002-a
ACK 3 0 purge dmf-000002-a
INFLIGHT 4 erase_source src-dmf-000002
ACK 4 0 erase_source src-dmf-000002
";
        let timeline = parse_oracle_log(log).expect("parse oracle log");

        // Resurrecting dmf-000002-a/b after ACK'd purge/erase_source MUST fail.
        let mut resurrected = BTreeMap::new();
        for seq in 1..=2 {
            for (k, v) in pair_nodes(seq, StoredNodeState::Active) {
                resurrected.insert(k, v);
            }
        }
        // Resurrecting dmf-000002-a/b after ACK'd purge/erase_source MUST fail
        // under both SyncMode::Full and SyncMode::Normal (without drop_writes).
        for mode in [SyncMode::Full, SyncMode::Normal] {
            assert!(
                verify_state_against_oracle(
                    mode,
                    Scenario::ErasureLockFault,
                    &timeline,
                    None,
                    &resurrected,
                    false,
                )
                .is_err(),
                "resurrected erased record must fail under {mode:?}"
            );
        }

        // Clean state with dmf-000002 removed passes.
        let mut clean = BTreeMap::new();
        for (k, v) in pair_nodes(1, StoredNodeState::Active) {
            clean.insert(k, v);
        }
        verify_state_against_oracle(
            SyncMode::Full,
            Scenario::ErasureLockFault,
            &timeline,
            None,
            &clean,
            false,
        )
        .expect("erased record absent must pass");
    }

    #[test]
    fn error_writes_full_rejects_silent_ok_and_accepts_fail_closed_err() {
        let log_ok = "\
SESSION_START error_writes_full full tenant-flakey
INFLIGHT 1 write dmf-000001
ACK 1 2 write dmf-000001
INFLIGHT 2 write dmf-000002
ACK 2 4 write dmf-000002
READY_FOR_ERROR_WRITES 2
INFLIGHT 3 write dmf-000003
ERR 3 write dmf-000003 storage_io_error
ERROR_WRITES_OBSERVED 1 0
";
        let tl_ok = parse_oracle_log(log_ok).expect("parse ok error_writes log");
        let mut observed = BTreeMap::new();
        for seq in 1..=2 {
            for (k, v) in pair_nodes(seq, StoredNodeState::Active) {
                observed.insert(k, v);
            }
        }
        verify_state_against_oracle(
            SyncMode::Full,
            Scenario::ErrorWritesFull,
            &tl_ok,
            None,
            &observed,
            false,
        )
        .expect("fail-closed ERR with intact baseline ACKs must pass");

        let log_silent_ok = "\
SESSION_START error_writes_full full tenant-flakey
INFLIGHT 1 write dmf-000001
ACK 1 2 write dmf-000001
READY_FOR_ERROR_WRITES 1
INFLIGHT 2 write dmf-000002
ACK 2 4 write dmf-000002
ERROR_WRITES_OBSERVED 0 1
";
        let tl_bad = parse_oracle_log(log_silent_ok).expect("parse bad error_writes log");
        assert!(
            verify_state_against_oracle(
                SyncMode::Full,
                Scenario::ErrorWritesFull,
                &tl_bad,
                None,
                &observed,
                false,
            )
            .is_err(),
            "silent Ok after READY_FOR_ERROR_WRITES must fail"
        );
    }

    #[test]
    fn end_to_end_local_scenarios_run_and_verify_for_normal_and_full() {
        for sync_mode in [SyncMode::Normal, SyncMode::Full] {
            for scenario in [
                Scenario::ActiveCommit,
                Scenario::WalAccum,
                Scenario::CheckpointErasure,
                Scenario::ErasureLockFault,
            ] {
                let dir = TempDir::new().expect("tempdir");
                let db = dir.path().join("fathom.db");
                let oracle = dir.path().join("oracle.log");
                let run_args = RunArgs {
                    db: db.clone(),
                    sync_mode,
                    scenario,
                    oracle_device: oracle.to_string_lossy().into_owned(),
                    count: Some(12),
                    scope: DEFAULT_SCOPE.to_string(),
                    step_delay_ms: 0,
                    wait_signal_file: None,
                    wait_timeout_secs: 5,
                    hold_after_ms: 0,
                    keep_lock_on_exit: scenario == Scenario::ErasureLockFault,
                };
                run_scenario(&run_args).unwrap_or_else(|e| {
                    panic!("run {scenario:?} ({sync_mode:?}) failed: {e}");
                });

                let verify_args = VerifyArgs {
                    db: db.clone(),
                    sync_mode,
                    scenario,
                    oracle_log: oracle.clone(),
                    prev_oracle_log: None,
                    scope: DEFAULT_SCOPE.to_string(),
                    allow_drop_writes_rollback: false,
                    skip_post_write: false,
                };
                let summary = verify_database(&verify_args).unwrap_or_else(|e| {
                    panic!("verify {scenario:?} ({sync_mode:?}) failed: {e}");
                });
                assert!(summary.ok, "summary must be ok: {summary:?}");
                assert_eq!(summary.integrity_check, "ok");
                assert_eq!(summary.data_plane_findings, 0);
                assert!(summary.post_recovery_write_ok);
                if scenario == Scenario::ErasureLockFault {
                    assert!(
                        summary.stale_lock_present_before_open,
                        "erasure_lock_fault must exercise stale .lock recovery"
                    );
                }
            }
        }
    }

    #[test]
    fn end_to_end_reopen_replay_two_phase_verify_passes() {
        let dir = TempDir::new().expect("tempdir");
        let db = dir.path().join("fathom.db");
        let oracle1 = dir.path().join("oracle1.log");
        let oracle2 = dir.path().join("oracle2.log");

        run_scenario(&RunArgs {
            db: db.clone(),
            sync_mode: SyncMode::Full,
            scenario: Scenario::WalAccum,
            oracle_device: oracle1.to_string_lossy().into_owned(),
            count: Some(10),
            scope: DEFAULT_SCOPE.to_string(),
            step_delay_ms: 0,
            wait_signal_file: None,
            wait_timeout_secs: 5,
            hold_after_ms: 0,
            keep_lock_on_exit: true,
        })
        .expect("phase 1 wal_accum");

        run_scenario(&RunArgs {
            db: db.clone(),
            sync_mode: SyncMode::Full,
            scenario: Scenario::ReopenReplay,
            oracle_device: oracle2.to_string_lossy().into_owned(),
            count: Some(8),
            scope: DEFAULT_SCOPE.to_string(),
            step_delay_ms: 0,
            wait_signal_file: None,
            wait_timeout_secs: 5,
            hold_after_ms: 0,
            keep_lock_on_exit: true,
        })
        .expect("phase 2 reopen_replay");

        let summary = verify_database(&VerifyArgs {
            db,
            sync_mode: SyncMode::Full,
            scenario: Scenario::ReopenReplay,
            oracle_log: oracle2,
            prev_oracle_log: Some(oracle1),
            scope: DEFAULT_SCOPE.to_string(),
            allow_drop_writes_rollback: false,
            skip_post_write: false,
        })
        .expect("verify reopen_replay");
        assert!(summary.ok);
        assert_eq!(summary.active_nodes, 36); // (10 + 8) * 2
    }
}
