//! Typed tool results. Each type is both what the tool serializes into
//! `structuredContent` and the source of its declared `outputSchema`
//! (MR-071), so the two cannot drift apart.

use schemars::JsonSchema;
use serde::Serialize;

use crate::error::ToolError;

/// The literal `"unavailable"`.
#[derive(Clone, Copy, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UnavailableWord {
    Unavailable,
}

/// `{ "status": "unavailable" }`: a signal that needs an engine addition.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct Unavailable {
    pub status: UnavailableWord,
}

impl Unavailable {
    pub(crate) fn new() -> Self {
        Self { status: UnavailableWord::Unavailable }
    }
}

/// One arm that did not run, or ran degraded, and why (MR-032).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, JsonSchema)]
pub struct Degradation {
    pub arm: String,
    pub reason: String,
}

/// The budget actually applied (MR-041). `used_tokens` is
/// ceil(bytes / 4) of the structured content without `budget` plus the text
/// block.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct Budget {
    pub max_tokens: u32,
    pub used_tokens: u64,
    pub cut: bool,
}

/// Fields present on every store-bound result (interface §3).
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct Envelope {
    pub store: String,
    pub epoch: u64,
    /// Epoch seconds the read was answered at; the only time-derived field.
    pub validity_instant: i64,
    pub state: Unavailable,
    pub degradation: Vec<Degradation>,
    pub budget: Budget,
}

/// The tool error envelope `{error: {...}}`.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ErrorOutput {
    pub error: ToolError,
}

/// Store readiness as listed by `list_stores`.
#[derive(Clone, Copy, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Readiness {
    Closed,
    Opening,
    Reopening,
    Ready,
    Draining,
    Failed,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct StoreRow {
    pub name: String,
    pub open: bool,
    pub readiness: Readiness,
    /// Whether this is the session's default store.
    pub default: bool,
}

/// `list_stores` result. Never includes paths (MR-005.2).
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ListStoresOutput {
    pub stores: Vec<StoreRow>,
}

/// `use_store` result.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct UseStoreOutput {
    pub store: String,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ProjectionRow {
    pub name: String,
    pub roles: Vec<String>,
    pub fts: bool,
    pub vector: bool,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ProjectionDenseRow {
    pub name: String,
    pub dense_readiness: String,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ProjectionStatusOut {
    pub runtime_embedder_available: bool,
    pub runtime_unavailability_reason: String,
    pub projections: Vec<ProjectionDenseRow>,
    pub vector_unsupported_kinds: Vec<String>,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct EmbeddingReadinessOut {
    pub state: String,
    pub usable_embedder: bool,
    pub pending_count: u64,
    pub affected_kinds: Vec<String>,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct DenseDisabled {
    pub disabled: bool,
    pub reason: Option<String>,
}

/// `orient` result.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct OrientOutput {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub projections: Vec<ProjectionRow>,
    pub projection_status: ProjectionStatusOut,
    pub embedding_readiness: EmbeddingReadinessOut,
    pub dense_disabled: DenseDisabled,
    /// Kind enumeration needs an engine addition (MR-022).
    pub kinds: Unavailable,
    pub counts: Unavailable,
}

/// Search completeness (MR-030). `partial`: fewer results than the limit
/// while an arm was degraded, so `complete` cannot be claimed (MR-032.3).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Completeness {
    Complete,
    LimitReached,
    CutByBudget,
    Empty,
    EmptyNotReady,
    Partial,
}

impl Completeness {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::LimitReached => "limit_reached",
            Self::CutByBudget => "cut_by_budget",
            Self::Empty => "empty",
            Self::EmptyNotReady => "empty_not_ready",
            Self::Partial => "partial",
        }
    }
}

/// One search hit.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct Hit {
    /// `IdSpace` prefixed form: `l:<logical_id>`, `h:<hash>` or `p:<n>`.
    #[serde(rename = "ref")]
    pub reference: String,
    pub kind: String,
    pub snippet: String,
    pub snippet_cut: bool,
    pub source_id: Option<String>,
    pub score: f64,
    pub arm: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ce_score: Option<f64>,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct TraceOut {
    pub query_chars: u32,
    pub k: u32,
    pub rerank_depth: u32,
    pub pool_n: u32,
    pub alpha: f64,
    pub use_graph_arm: bool,
    pub recency: bool,
    pub embedder_id: String,
    pub ce_active: bool,
    pub vector_hits: u32,
    pub text_hits: u32,
    pub graph_hits: u32,
    pub dropped_edge_hits: u32,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct PerHitOut {
    #[serde(rename = "ref")]
    pub reference: String,
    pub arm: String,
    pub vector_rank: Option<u32>,
    pub text_rank: Option<u32>,
    pub graph_rank: Option<u32>,
    pub fused_score: f64,
    pub ce_score: Option<f64>,
    pub blended: f64,
}

/// Per-hit explanation (hybrid), or why there is none (text mode). The
/// engine's correlation id is omitted: it varies run to run (MR-043).
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum ExplanationOut {
    Detail { trace: TraceOut, per_hit: Vec<PerHitOut> },
    Unavailable { status: UnavailableWord, reason: String },
}

/// The literal `"text"` or `"hybrid"`.
#[derive(Clone, Copy, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModeWord {
    Text,
    Hybrid,
}

/// `search` result.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct SearchOutput {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub mode: ModeWord,
    /// `r<epoch>-<hash>`, for `feedback`.
    pub response_id: String,
    pub results: Vec<Hit>,
    pub completeness: Completeness,
    /// Needs calibrated scores (MR-031).
    pub verdict: UnavailableWord,
    /// `c<epoch>-<hash>` or null.
    pub continuation: Option<String>,
    /// Present (false) in embedded mode, where the host may write between
    /// pages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation_exact: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<ExplanationOut>,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct RecordOut {
    pub kind: String,
    pub body: String,
    pub body_cut: bool,
}

/// One `fetch` item: a record or a per-item error (MR-052).
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum FetchItem {
    Record {
        #[serde(rename = "ref")]
        reference: String,
        record: RecordOut,
    },
    Error {
        #[serde(rename = "ref")]
        reference: String,
        error: ToolError,
    },
}

/// `fetch` result.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct FetchOutput {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub items: Vec<FetchItem>,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct Target {
    #[serde(rename = "ref")]
    pub reference: String,
    pub kind: String,
    pub hop_count: u32,
    pub terminal_edge_kind: String,
    pub body: String,
    pub body_cut: bool,
}

/// `expand` result. `bound` names the cap that was reached, or is null.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ExpandOutput {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub targets: Vec<Target>,
    pub complete: bool,
    pub bound: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackStatus {
    Recorded,
    NotRecorded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotRecordedReason {
    TelemetryDisabled,
    TextMode,
    StoreReopened,
    ResponseExpired,
}

/// `feedback` result.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct FeedbackOutput {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub status: FeedbackStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<NotRecordedReason>,
}

/// A tool's structured content: its result, or a tool error.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum ToolResult<T> {
    Ok(T),
    Err(ErrorOutput),
}
