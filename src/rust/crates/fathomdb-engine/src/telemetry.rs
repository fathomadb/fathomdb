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

/// 0.8.8 Slice 15 — append one JSON value as a line to the telemetry sink
/// (append-only, local file; no network). Best-effort caller handles the error.
fn append_jsonl(path: &Path, value: &serde_json::Value) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
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
    /// 0.8.8 Slice 15 (OPP-9) — enable opt-in telemetry capture to a local JSONL
    /// `sink_path` (append-only). Off by default; once enabled, each `search`
    /// records a query→result event and `record_feedback` appends agent labels.
    /// Local file only — no network/egress. `query_id` + `ts_monotonic_ms` are
    /// reset deterministically on enable. Idempotent re-enable resets the seq.
    pub fn enable_telemetry(&self, sink_path: &str) -> Result<(), EngineError> {
        // Touch the sink (create + validate writable) before arming capture, so a
        // bad path fails loudly here rather than silently dropping events.
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(sink_path)
            .map_err(|_| EngineError::Storage)?;
        let mut guard = self.telemetry.lock().map_err(|_| EngineError::Storage)?;
        *guard = Some(TelemetrySink {
            path: PathBuf::from(sink_path),
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
    fn capture_telemetry(&self, query: &str, result: &SearchResult) {
        // Fast OFF path (codex §9 P2): a single atomic load when telemetry has
        // never been enabled — NO mutex acquisition, NO contention with the search
        // hot path.
        if !self.telemetry_enabled.load(Ordering::Acquire) {
            return;
        }
        let Ok(mut guard) = self.telemetry.lock() else { return };
        let Some(sink) = guard.as_mut() else { return };
        Self::capture_telemetry_with_sink(query, result, sink);
    }

    pub(crate) fn finalize_search_observability(&self, query: &str, result: &mut SearchResult) {
        if result.explanation.is_none() {
            self.capture_telemetry(query, result);
            return;
        }

        #[cfg(feature = "test-hooks")]
        explanation_finalization_hooks::fire_before();
        let correlation_id = if let Ok(mut guard) = self.telemetry.lock() {
            if let Some(sink) = guard.as_mut() {
                Self::capture_telemetry_with_sink(query, result, sink)
            } else {
                self.mint_explanation_correlation_id()
            }
        } else {
            self.mint_explanation_correlation_id()
        };
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
            "result_stable_ids": result
                .results
                .iter()
                .map(|h| h.id.to_prefixed())
                .collect::<Vec<String>>(),
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
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Instant;
