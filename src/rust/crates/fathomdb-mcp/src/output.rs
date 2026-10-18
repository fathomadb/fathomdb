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
    /// Suggested next calls; absent when there are none or hints are off.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<Hint>,
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
    /// True when any of the record is left out of `snippet`.
    pub snippet_cut: bool,
    /// Where `snippet` comes from: `whole` (the record fitted), `match` (a
    /// window centred on the matched words) or `start` (the record start:
    /// no lexical match, as for vector-only hits).
    pub snippet_anchor: String,
    /// `[start, end)` byte ranges into `snippet` of the matched words; only
    /// when the call passed `highlights: true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlights: Option<Vec<(u32, u32)>>,
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

/// What a hint suggests doing.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HintAction {
    Fetch,
    Search,
    None,
}

/// Why a hint was emitted (interface §3 vocabulary). Variants are in emission
/// order.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HintReason {
    Degraded,
    NoResults,
    SnippetCut,
    BodyCut,
    CutByBudget,
    MoreResults,
    DefaultStoreSet,
}

/// The arm a `degraded` hint names.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HintArm {
    Vector,
    Text,
    TextEdge,
    GraphArm,
    Graph,
}

/// Why the arm degraded: the adapter's degradation reasons and the graph
/// expansion codes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HintCause {
    SoftFallback,
    DenseDisabled,
    EmbedderUnavailable,
    EmbedderOverloaded,
    QuerySeedTextFallback,
    ProjectionLegacyUnverified,
    ProjectionProcessing,
    ProjectionBlocked,
    ProjectionDeferred,
    ProjectionDegraded,
}

impl HintArm {
    /// The wire word of an engine arm; `None` for one this adapter does not
    /// know yet.
    pub(crate) fn from_engine(arm: fathomdb_sdk::HintArm) -> Option<Self> {
        use fathomdb_sdk::HintArm as R;
        Some(match arm {
            R::Vector => Self::Vector,
            R::Text => Self::Text,
            R::TextEdge => Self::TextEdge,
            R::GraphArm => Self::GraphArm,
            R::Graph => Self::Graph,
            _ => return None,
        })
    }
}

impl HintCause {
    /// The wire word of an engine cause; `None` for one this adapter does
    /// not know yet.
    pub(crate) fn from_engine(cause: fathomdb_sdk::HintCause) -> Option<Self> {
        use fathomdb_sdk::HintCause as C;
        Some(match cause {
            C::SoftFallback => Self::SoftFallback,
            C::DenseDisabled => Self::DenseDisabled,
            C::EmbedderUnavailable => Self::EmbedderUnavailable,
            C::EmbedderOverloaded => Self::EmbedderOverloaded,
            C::QuerySeedTextFallback => Self::QuerySeedTextFallback,
            C::ProjectionLegacyUnverified => Self::ProjectionLegacyUnverified,
            C::ProjectionProcessing => Self::ProjectionProcessing,
            C::ProjectionBlocked => Self::ProjectionBlocked,
            C::ProjectionDeferred => Self::ProjectionDeferred,
            C::ProjectionDegraded => Self::ProjectionDegraded,
            _ => return None,
        })
    }
}

/// One suggested next call, authored by FathomDB (ADR-0.8.29). Every string
/// is a closed-enum word, an adapter handle, an operator store name or a ref
/// from the same response; never record content. Which optional fields are
/// present depends on `reason` (interface §3). Clients ignore a hint whose
/// `action` or `reason` they do not know.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct Hint {
    pub action: HintAction,
    pub reason: HintReason,
    /// Refs to pass to `fetch`, in result order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refs: Option<Vec<String>>,
    /// Cut refs left out: beyond 50, or longer than 128 bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refs_omitted: Option<u32>,
    /// Suggested search mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<ModeWord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm: Option<HintArm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<HintCause>,
    /// The continuation to pass to `search`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
    /// The `max_tokens` to pass with the suggested call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    /// The store the suggested call addresses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<String>,
}

impl Hint {
    pub(crate) fn new(action: HintAction, reason: HintReason) -> Self {
        Self {
            action,
            reason,
            refs: None,
            refs_omitted: None,
            mode: None,
            arm: None,
            cause: None,
            continuation: None,
            max_tokens: None,
            store: None,
        }
    }
}

/// The engine subset's words map one to one; an engine word this adapter does
/// not know yet is refused, so the hint is dropped (clients ignore unknown
/// words, and so does the adapter).
impl TryFrom<fathomdb_sdk::Hint> for Hint {
    type Error = fathomdb_sdk::Hint;

    fn try_from(hint: fathomdb_sdk::Hint) -> Result<Self, Self::Error> {
        use fathomdb_sdk::HintReason as W;
        use fathomdb_sdk::{HintAction as A, HintMode as M};
        let action = match hint.action {
            A::Search => HintAction::Search,
            A::None => HintAction::None,
            _ => return Err(hint),
        };
        let reason = match hint.reason {
            W::NoResults => HintReason::NoResults,
            W::Degraded => HintReason::Degraded,
            W::MoreResults => HintReason::MoreResults,
            _ => return Err(hint),
        };
        let mode = match hint.mode {
            None => None,
            Some(M::Text) => Some(ModeWord::Text),
            Some(M::Hybrid) => Some(ModeWord::Hybrid),
            Some(_) => return Err(hint),
        };
        let arm = match hint.arm {
            None => None,
            Some(arm) => match HintArm::from_engine(arm) {
                Some(arm) => Some(arm),
                None => return Err(hint),
            },
        };
        let cause = match hint.cause {
            None => None,
            Some(cause) => match HintCause::from_engine(cause) {
                Some(cause) => Some(cause),
                None => return Err(hint),
            },
        };
        // `more_results{limit}` becomes `more_results{continuation}` in the
        // adapter, which attaches its own handle; the limit is not carried.
        Ok(Self { mode, arm, cause, ..Self::new(action, reason) })
    }
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
    /// Suggested next calls; absent when there are none or hints are off.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<Hint>,
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
    /// Suggested next calls; absent when there are none or hints are off.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<Hint>,
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
    /// Suggested next calls; absent when there are none or hints are off.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<Hint>,
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

/// A tool's structured content: its result, or a tool error. Only its schema
/// is used (as the declared `outputSchema`); values are built per variant.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum ToolResult<T> {
    Ok(T),
    Err(ErrorOutput),
}
