use crate::PhysicalErasurePolicy;
/// REQ-006a / AC-007a default slow-statement threshold. Mutated at runtime
/// via [`Engine::set_slow_threshold_ms`].
pub(crate) const DEFAULT_SLOW_THRESHOLD_MS: u64 = 100;
/// Snapshot of engine-internal counters returned by [`Engine::counters`].
///
/// Public key set is owned by `dev/design/lifecycle.md` § Public key set
/// and locked by AC-004a. Reading a snapshot is non-perturbing per
/// AC-004c. The 0.6.0 surface exposes exactly these seven fields.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CounterSnapshot {
    pub queries: u64,
    pub writes: u64,
    pub write_rows: u64,
    pub errors_by_code: BTreeMap<String, u64>,
    pub admin_ops: u64,
    pub cache_hit: u64,
    pub cache_miss: u64,
}

#[cfg(all(feature = "test-hooks", target_os = "linux"))]
pub(crate) fn process_current_rss_bytes() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix("VmRSS:")
                    .and_then(|value| value.split_whitespace().next())
                    .and_then(|value| value.parse::<u64>().ok())
            })
        })
        .unwrap_or(0)
        .saturating_mul(1024)
}

#[cfg(all(feature = "test-hooks", not(target_os = "linux")))]
pub(crate) fn process_current_rss_bytes() -> u64 {
    0
}

#[cfg(all(feature = "test-hooks", target_os = "linux"))]
pub(crate) fn process_peak_rss_bytes() -> u64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    // SAFETY: `getrusage` initializes the supplied `rusage` on a zero return.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } == 0 {
        // SAFETY: guarded by the successful `getrusage` return above.
        let kilobytes = unsafe { usage.assume_init() }.ru_maxrss;
        u64::try_from(kilobytes).unwrap_or(0).saturating_mul(1024)
    } else {
        0
    }
}

#[cfg(all(feature = "test-hooks", not(target_os = "linux")))]
pub(crate) fn process_peak_rss_bytes() -> u64 {
    0
}

/// 0.8.8 Slice 15 (OPP-9) — opt-in telemetry capture state (per `enable_telemetry`).
/// Records query→result→feedback events to a local JSONL sink. Query text and
/// `source_id` are never captured. `query_id = "q{nonce}-{seq}"` is fully
/// deterministic; `ts_monotonic_ms` is monotonic since enable.
pub(super) struct TelemetrySink {
    path: PathBuf,
    base: Instant,
    nonce: u64,
    seq: u64,
    last_query_id: Option<String>,
}

impl TelemetrySink {
    pub(super) fn path(&self) -> &Path {
        &self.path
    }
}

const SINK_PREFIX: &str = "_fathomdb_telemetry_sink:";

pub(crate) fn sink_fingerprint(path: &Path) -> Result<String, EngineError> {
    use sha2::Digest;
    let encoded = serde_json::to_vec(path).map_err(|_| EngineError::Storage)?;
    let mut hash = sha2::Sha256::new();
    hash.update(b"fathomdb.telemetry-sink.v1\0");
    hash.update(encoded);
    Ok(crate::identity::hex_encode(&hash.finalize()))
}

pub(crate) fn registered_sinks(c: &Connection) -> Result<Vec<String>, EngineError> {
    let mut statement = c
        .prepare("SELECT key FROM _fathomdb_open_state WHERE substr(key,1,?1)=?2 ORDER BY key")
        .map_err(|_| EngineError::Storage)?;
    let rows = statement
        .query_map(params![SINK_PREFIX.len(), SINK_PREFIX], |r| r.get::<_, String>(0))
        .map_err(|_| EngineError::Storage)?;
    rows.map(|row| {
        let key = row.map_err(|_| EngineError::Storage)?;
        let value = &key[SINK_PREFIX.len()..];
        if value.len() != 64
            || !value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(EngineError::Storage);
        }
        Ok(value.to_owned())
    })
    .collect()
}

/// 0.8.8 Slice 15 — append one JSON value as a line to the telemetry sink
/// (append-only, local file; no network). Best-effort caller handles the error.
fn append_jsonl(path: &Path, value: &serde_json::Value) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.create(true).append(true);
    let mut file = std::fs::OpenOptions::open(&options, path)?;
    writeln!(file, "{value}")?;
    Ok(())
}

/// 0.8.8 Slice 15 — the lowercase wire string for a retrieval arm (telemetry +
/// the same spelling `SearchHit.branch` crosses every binding).
fn branch_str(branch: SoftFallbackBranch) -> &'static str {
    match branch {
        SoftFallbackBranch::Vector => "vector",
        SoftFallbackBranch::Text => "text",
        SoftFallbackBranch::TextEdge => "text_edge",
        SoftFallbackBranch::GraphArm => "graph_arm",
    }
}

impl Engine {
    /// Snapshot of engine-internal counters.
    ///
    /// Field set owned by `dev/design/lifecycle.md`.
    #[must_use]
    pub fn counters(&self) -> CounterSnapshot {
        self.counters.snapshot()
    }

    /// Toggle response-cycle profiling.
    ///
    /// Per `dev/design/lifecycle.md` § Per-statement profiling, profiling
    /// is an opt-in surface that is independently toggleable on a running
    /// engine without restart. AC-005a locks runtime toggleability.
    pub fn set_profiling(&self, enabled: bool) -> Result<(), EngineError> {
        let _operation = self.ensure_open()?;
        self.guard_explicit_metadata_mutation()?;
        self.profiling_enabled.store(enabled, Ordering::Relaxed);
        Ok(())
    }

    /// Set the threshold above which an operation is reported as slow.
    ///
    /// Per `dev/design/lifecycle.md` § Slow and heartbeat policy, the
    /// threshold is runtime-configurable; mutating it changes detection
    /// behavior on subsequent statements without restart (AC-007b).
    pub fn set_slow_threshold_ms(&self, value: u64) -> Result<(), EngineError> {
        let _operation = self.ensure_open()?;
        self.guard_explicit_metadata_mutation()?;
        self.slow_threshold_ms.store(value, Ordering::Relaxed);
        Ok(())
    }

    /// 0.8.8 Slice 15 (OPP-9) — enable opt-in telemetry capture to a local JSONL
    /// `sink_path` (append-only). Off by default; once enabled, each `search`
    /// records a query→result event and `record_feedback` appends agent labels.
    /// Local file only — no network/egress. `query_id` + `ts_monotonic_ms` are
    /// reset deterministically on enable. Idempotent re-enable resets the seq.
    pub fn enable_telemetry(&self, sink_path: &str) -> Result<(), EngineError> {
        let _operation = self.ensure_open()?;
        if self.physical_policy == PhysicalErasurePolicy::ExplicitMaintenance
            && crate::maintenance_admission::in_callback()
        {
            return Err(EngineError::Maintenance(crate::MaintenanceError::MaintenanceBusy));
        }
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        if self.physical_policy == PhysicalErasurePolicy::ExplicitMaintenance {
            crate::dependency_closure::guard_no_pending_physical(connection)?;
        }
        // Touch the sink (create + validate writable) before arming capture, so a
        // bad path fails loudly here rather than silently dropping events.
        let mut options = std::fs::OpenOptions::new();
        options.create(true).append(true);
        std::fs::OpenOptions::open(&options, sink_path).map_err(|_| EngineError::Storage)?;
        let path = std::fs::canonicalize(sink_path).map_err(|_| EngineError::Storage)?;
        let fingerprint = sink_fingerprint(&path)?;
        let mut guard = self.telemetry.lock().map_err(|_| EngineError::Storage)?;
        connection
            .execute(
                "INSERT OR IGNORE INTO _fathomdb_open_state(key,value) VALUES(?1,'1')",
                [format!("{SINK_PREFIX}{fingerprint}")],
            )
            .map_err(|_| EngineError::Storage)?;
        *guard = Some(TelemetrySink {
            path,
            base: Instant::now(),
            nonce: 0,
            seq: 0,
            last_query_id: None,
        });
        // Arm the fast OFF-path guard LAST (after the sink is installed) so a
        // concurrent search either sees telemetry fully off or fully on.
        self.telemetry_enabled.store(true, Ordering::Release);
        Ok(())
    }

    /// 0.8.8 Slice 15 — the most-recent captured `query_id` (for `record_feedback`).
    /// `None` when telemetry is off or no query has been captured yet.
    pub fn last_telemetry_query_id(&self) -> Option<String> {
        self.telemetry.lock().ok()?.as_ref().and_then(|s| s.last_query_id.clone())
    }

    /// 0.8.8 Slice 15 — capture a query→result telemetry event. No-op (no alloc,
    /// no I/O) when telemetry is off (the default). Best-effort: a sink write error
    /// never fails the search. Captures ONLY ids, arms, and the query LENGTH —
    /// never the query text or `source_id` (privacy, ADR §C).
    ///
    /// ID-SPACES (Cause-A, 0.8.11.2 — honest record). `result_ids` is the interim
    /// `SearchHit.id` == `write_cursor`: within-session consistent but NOT
    /// cross-session-stable (reassigned on re-projection/re-ingest). `arm_of` is
    /// keyed by that same `write_cursor`. Cause-A adds a NEW PARALLEL field
    /// `result_stable_ids` carrying the cross-session-stable id
    /// ([`SearchHit::stable_id`], `logical_id` / content-hash) in the SAME order as
    /// `result_ids`; the existing `write_cursor` keys are RETAINED unchanged so
    /// pre-Cause-A gold and sink byte-output stay valid (the F-8a `id_space` flip
    /// is a separate, conscious step — see
    /// `dev/plans/runs/NOTE-0.8.8-to-steward-id-contract.md`).
    fn capture_telemetry(&self, query: &str, result: &SearchResult) -> Option<String> {
        if self.physical_policy == PhysicalErasurePolicy::ExplicitMaintenance
            && self.explicit_debt.load(Ordering::Acquire)
        {
            return None;
        }
        if !self.telemetry_enabled.load(Ordering::Acquire) {
            return None;
        }
        // Match admission's writer -> telemetry lock order. A materialized read
        // may outlive its rows; it must not create a fresh durable copy afterwards.
        let connection = self.connection.lock().ok()?;
        let connection = connection.as_ref()?;
        let mut guard = self.telemetry.lock().ok()?;
        let sink = guard.as_mut()?;
        let mut stable_ids = Vec::with_capacity(result.results.len());
        for hit in &result.results {
            let present: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM canonical_nodes WHERE write_cursor=?1) OR EXISTS(SELECT 1 FROM canonical_edges WHERE write_cursor=?1)",
                [hit.write_cursor], |r| r.get(0),
            ).ok()?;
            stable_ids.push(if present { hit.id.to_prefixed() } else { "[erased]".into() });
        }
        Some(Self::capture_telemetry_with_sink(query, result, sink, stable_ids))
    }

    pub(crate) fn finalize_search_observability(&self, query: &str, result: &mut SearchResult) {
        if result.explanation.is_none() {
            self.capture_telemetry(query, result);
            return;
        }
        #[cfg(feature = "test-hooks")]
        explanation_finalization_hooks::fire_before();
        let correlation_id = self
            .capture_telemetry(query, result)
            .unwrap_or_else(|| self.mint_explanation_correlation_id());
        #[cfg(feature = "test-hooks")]
        explanation_finalization_hooks::fire_after();
        if let Some(explanation) = result.explanation.as_mut() {
            explanation.correlation_id = correlation_id;
        }
    }

    fn mint_explanation_correlation_id(&self) -> String {
        let sequence = self.explanation_sequence.fetch_add(1, Ordering::Relaxed);
        format!("x{:032x}-{sequence}", self.explanation_open_nonce)
    }

    fn capture_telemetry_with_sink(
        query: &str,
        result: &SearchResult,
        sink: &mut TelemetrySink,
        stable_ids: Vec<String>,
    ) -> String {
        let query_id = format!("q{}-{}", sink.nonce, sink.seq);
        let ts_monotonic_ms = sink.base.elapsed().as_millis() as u64;
        let mut arm_of = serde_json::Map::new();
        for h in &result.results {
            // Keyed on the engine-internal positional cursor (the pre-C-2
            // `SearchHit.id` == `write_cursor`), byte-unchanged so `record_feedback`
            // + the gold pipeline keep keying on the same `result_ids` space.
            arm_of
                .insert(h.write_cursor.to_string(), serde_json::Value::from(branch_str(h.branch)));
        }
        let event = serde_json::json!({
            "type": "event",
            "schema_version": 1,
            "ts_monotonic_ms": ts_monotonic_ms,
            "query_id": query_id,
            "query_chars": query.chars().count() as u64,
            "result_ids": result.results.iter().map(|h| h.write_cursor).collect::<Vec<u64>>(),
            // Cause-A / C-2: parallel cross-session-stable ids, SAME order as
            // result_ids. Post-C-2 the stable id lives on `SearchHit.id` (its
            // prefixed form == the pre-swap `stable_id` value byte-for-byte), so
            // the emitted bytes are unchanged and the `write_cursor` result_ids
            // keys are retained unchanged (pre-Cause-A gold stays valid).
            "result_stable_ids": stable_ids,
            "arm_of": arm_of,
        });
        let _ = append_jsonl(&sink.path, &event);
        sink.seq += 1;
        sink.last_query_id = Some(query_id.clone());
        query_id
    }

    /// 0.8.8 Slice 15 — append an agent-supplied relevance-label record for a
    /// previously-captured `query_id`. `label_source` is the only exogenous string
    /// (caller-declared, e.g. `"agent:hermes"`).
    ///
    /// ID-SPACE (Cause-A, 0.8.11.2 — honest record). `relevant_ids` /
    /// `irrelevant_ids` are the interim `SearchHit.id` == `write_cursor` (the same
    /// space as the captured event's `result_ids`), NOT `logical_id`. The
    /// signature is left byte-stable: the gold pipeline maps these `write_cursor`
    /// keys to the cross-session-stable id via the capture event's parallel
    /// `result_ids` ↔ `result_stable_ids` arrays (`eval/gold_capture.py`), so no
    /// new feedback parameter — and no binding-signature churn — is required.
    /// Errors if telemetry is off.
    pub fn record_feedback(
        &self,
        query_id: &str,
        relevant_ids: &[u64],
        irrelevant_ids: &[u64],
        label_source: &str,
    ) -> Result<(), EngineError> {
        let _operation = self.ensure_open()?;
        self.guard_explicit_metadata_mutation()?;
        if self.physical_policy == PhysicalErasurePolicy::ExplicitMaintenance
            && crate::maintenance_admission::in_callback()
        {
            return Err(EngineError::Maintenance(crate::MaintenanceError::MaintenanceBusy));
        }
        let guard = self.telemetry.lock().map_err(|_| EngineError::Storage)?;
        let sink = guard
            .as_ref()
            .ok_or(EngineError::InvalidArgument { msg: "telemetry is not enabled".to_string() })?;
        // codex §9 [P1] (privacy): `query_id` is an exogenous caller string. Only a
        // deterministic id that `capture_telemetry` has ALREADY emitted may be
        // persisted — otherwise a caller could smuggle query text / a `source_id`
        // into the sink under the `query_id` key. Require the canonical
        // `q{nonce}-{seq}` form with `nonce == sink.nonce` AND `seq < sink.seq`
        // (a seq the capture path has issued). Reject (writing nothing) otherwise.
        let is_issued_id = query_id
            .strip_prefix('q')
            .and_then(|rest| rest.split_once('-'))
            .and_then(|(nonce, seq)| Some((nonce.parse::<u64>().ok()?, seq.parse::<u64>().ok()?)))
            .is_some_and(|(nonce, seq)| nonce == sink.nonce && seq < sink.seq);
        if !is_issued_id {
            return Err(EngineError::InvalidArgument { msg: "unknown query_id".to_string() });
        }
        let record = serde_json::json!({
            "type": "feedback",
            "schema_version": 1,
            "query_id": query_id,
            "relevant_ids": relevant_ids,
            "irrelevant_ids": irrelevant_ids,
            "label_source": label_source,
        });
        append_jsonl(&sink.path, &record).map_err(|_| EngineError::Storage)
    }
}
use crate::errors::EngineError;
use crate::search_types::{SearchResult, SoftFallbackBranch};
#[cfg(feature = "test-hooks")]
use crate::test_hooks::explanation_finalization_hooks;
use crate::Engine;
use rusqlite::{params, Connection};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Instant;

pub(crate) fn original_attachment(path: PathBuf) -> TelemetrySink {
    TelemetrySink { path, base: Instant::now(), nonce: 0, seq: 0, last_query_id: None }
}

impl Engine {
    fn guard_explicit_metadata_mutation(&self) -> Result<(), EngineError> {
        if self.physical_policy == PhysicalErasurePolicy::ExplicitMaintenance
            && self.explicit_debt.load(Ordering::Acquire)
        {
            return Err(EngineError::ErasureIncomplete {
                stage: "physical_admission".into(),
                detail: "explicit physical cleanup remains owed".into(),
            });
        }
        Ok(())
    }
}
