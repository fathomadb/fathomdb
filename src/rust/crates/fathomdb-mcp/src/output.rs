//! Typed tool results. Each type is both what the tool serializes into
//! `structuredContent` and the source of its declared `outputSchema` in
//! structured mode (MR-071). Compact mode renders the same governed facts
//! according to the explicit presentation census in the MCP interface.

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

/// The budget actually applied (MR-041). In structured mode `used_tokens`
/// counts the payload without this budget plus its final JSON text copy;
/// fit counts both complete copies. Compact text displays its own final
/// UTF-8 byte count divided by four, rounded up.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct Budget {
    pub max_tokens: u32,
    pub used_tokens: u64,
    pub cut: bool,
}

/// The literal `"available"` for live `EP-01` visibility state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StateStatusWord {
    Available,
}

/// Content-free drift classification relative to the previous observation in
/// the same session (`EP-01`).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DriftLabel {
    Unchanged,
    Canonical,
    Index,
}

/// Live content-free store visibility state (`EP-01`).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, JsonSchema)]
pub struct StoreStateOut {
    pub status: StateStatusWord,
    pub database_id: String,
    pub generation: u64,
    pub nonce: String,
    pub canonical_write_boundary: u64,
    pub projection_cursor: u64,
    pub drift: DriftLabel,
}

impl StoreStateOut {
    pub(crate) fn from_visibility(
        state: &fathomdb_sdk::VisibilityStateV1,
        drift: DriftLabel,
    ) -> Self {
        Self {
            status: StateStatusWord::Available,
            database_id: state.database_id.clone(),
            generation: state.generation,
            nonce: state.nonce.clone(),
            canonical_write_boundary: state.canonical_write_boundary,
            projection_cursor: state.projection_cursor,
            drift,
        }
    }
}

/// Fields present on every store-bound result (interface §3).
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct Envelope {
    pub store: String,
    pub epoch: u64,
    /// Epoch seconds the read was answered at; the only time-derived field.
    pub validity_instant: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<StoreStateOut>,
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

/// Required nullable default store: a name or JSON null.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum DefaultStoreName {
    /// One configured session store name.
    Name(String),
    /// This session has no default.
    Unset(()),
}

impl From<Option<String>> for DefaultStoreName {
    fn from(value: Option<String>) -> Self {
        value.map_or(Self::Unset(()), Self::Name)
    }
}

/// `list_stores` result. Never includes paths (MR-005.2).
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ListStoresOutput {
    /// Session default used when a store-bound call omits store, including null.
    pub default_store: DefaultStoreName,
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
    pub entity_class: String,
    pub name: String,
    pub roles: Vec<String>,
    pub fts: bool,
    pub vector: bool,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ProjectionDenseRow {
    pub entity_class: String,
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

/// The only reported mapped-span population.
#[derive(Clone, Copy, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CoveragePopulationOut {
    /// Attributable stored mapped heads, independent of live eligibility.
    StoredMappedSpans,
}

/// Availability reasons are distinct from typed metadata failures.
#[derive(Clone, Copy, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CoverageUnavailableReasonOut {
    /// No committed seed exists in this engine.
    CacheCold,
    /// Current writer token differs from the committed seed.
    StateChanged,
}

/// Stored default profile identity; its dimension is numeric.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct CoverageProfileOut {
    pub key: String,
    pub name: String,
    pub revision: String,
    pub dimension: u32,
}

/// Exact unreduced fraction carried as canonical decimal strings.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct CoverageFractionOut {
    #[schemars(regex(pattern = "^(0|[1-9][0-9]*)$"))]
    pub numerator: String,
    #[schemars(regex(pattern = "^[1-9][0-9]*$"))]
    pub denominator: String,
}

/// Required nullable exact share.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum NullableCoverageFraction {
    /// Exact share with positive denominator.
    Value(CoverageFractionOut),
    /// No known denominator.
    Null(()),
}

impl From<&Option<fathomdb_sdk::StoredHeadCoverageFraction>> for NullableCoverageFraction {
    fn from(value: &Option<fathomdb_sdk::StoredHeadCoverageFraction>) -> Self {
        value.as_ref().map_or(Self::Null(()), |value| {
            Self::Value(CoverageFractionOut {
                numerator: value.numerator.to_string(),
                denominator: value.denominator.to_string(),
            })
        })
    }
}

/// Exact committed counts; decimal transport avoids JSON integer rounding.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct CoverageCountsOut {
    #[schemars(regex(pattern = "^(0|[1-9][0-9]*)$"))]
    pub records: String,
    #[schemars(regex(pattern = "^(0|[1-9][0-9]*)$"))]
    pub known_records: String,
    #[schemars(regex(pattern = "^(0|[1-9][0-9]*)$"))]
    pub unknown_records: String,
    #[schemars(regex(pattern = "^(0|[1-9][0-9]*)$"))]
    pub truncated_known_records: String,
    #[schemars(regex(pattern = "^(0|[1-9][0-9]*)$"))]
    pub known_body_bytes: String,
    #[schemars(regex(pattern = "^(0|[1-9][0-9]*)$"))]
    pub unrepresented_known_bytes: String,
    #[schemars(regex(pattern = "^(0|[1-9][0-9]*)$"))]
    pub unattributed_heads: String,
    pub truncated_known_share: NullableCoverageFraction,
    pub unrepresented_known_byte_share: NullableCoverageFraction,
}

/// Truthful stored mapped-span population and committed-cache availability.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CoverageOut {
    /// Matching committed writer capture.
    Available {
        population: CoveragePopulationOut,
        profile: CoverageProfileOut,
        #[serde(flatten)]
        counts: Box<CoverageCountsOut>,
    },
    /// No matching committed counts.
    Unavailable {
        population: CoveragePopulationOut,
        profile: CoverageProfileOut,
        reason: CoverageUnavailableReasonOut,
    },
}

impl From<&fathomdb_sdk::StoredHeadCoverageV1> for CoverageOut {
    fn from(value: &fathomdb_sdk::StoredHeadCoverageV1) -> Self {
        use fathomdb_sdk::{StoredHeadCoverageUnavailableReason as R, StoredHeadCoverageV1 as C};
        let profile = |p: &fathomdb_sdk::StoredHeadCoverageProfile| CoverageProfileOut {
            key: p.key.clone(),
            name: p.name.clone(),
            revision: p.revision.clone(),
            dimension: p.dimension,
        };
        match value {
            C::Available { profile: p, counts: c, .. } => Self::Available {
                population: CoveragePopulationOut::StoredMappedSpans,
                profile: profile(p),
                counts: Box::new(CoverageCountsOut {
                    records: c.records.to_string(),
                    known_records: c.known_records.to_string(),
                    unknown_records: c.unknown_records.to_string(),
                    truncated_known_records: c.truncated_known_records.to_string(),
                    known_body_bytes: c.known_body_bytes.to_string(),
                    unrepresented_known_bytes: c.unrepresented_known_bytes.to_string(),
                    unattributed_heads: c.unattributed_heads.to_string(),
                    truncated_known_share: (&c.truncated_known_share).into(),
                    unrepresented_known_byte_share: (&c.unrepresented_known_byte_share).into(),
                }),
            },
            C::Unavailable { profile: p, reason, .. } => Self::Unavailable {
                population: CoveragePopulationOut::StoredMappedSpans,
                profile: profile(p),
                reason: match reason {
                    R::CacheCold => CoverageUnavailableReasonOut::CacheCold,
                    R::StateChanged => CoverageUnavailableReasonOut::StateChanged,
                },
            },
        }
    }
}

/// `orient` result.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct OrientOutput {
    /// Stored mapped-head coverage, exactly once outside projection_status.
    pub vector_coverage: CoverageOut,
    #[serde(flatten)]
    pub envelope: Envelope,
    pub projections: Vec<ProjectionRow>,
    pub projection_status: ProjectionStatusOut,
    pub embedding_readiness: EmbeddingReadinessOut,
    /// Kind enumeration needs an engine addition (MR-022).
    pub kinds: Unavailable,
    pub counts: Unavailable,
    /// Capabilities omitted until they have meaningful supported values.
    pub not_in_this_release: [&'static str; 3],
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
    /// Known original stored head extent of this actual vector participant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scored_bytes: Option<u32>,
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
    /// Opaque search evidence reference (present when frozen search is requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_ref: Option<String>,
    /// Immutable artifact revision id (present when frozen search is requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_revision_id: Option<String>,
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
    /// This query had no configured numerical embedding runtime.
    EmbedderAbsent,
    /// The query started in its provider but exceeded its deadline.
    EmbedderTimeout,
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
            C::EmbedderAbsent => Self::EmbedderAbsent,
            C::EmbedderTimeout => Self::EmbedderTimeout,
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
    /// For `snippet_cut`: the anchors of the cut hits, which pick the text
    /// line's sentence. Not on the wire; `snippet_anchor` is per hit.
    #[serde(skip)]
    #[schemars(skip)]
    pub(crate) cut_anchors: CutAnchors,
}

/// Which `snippet_anchor` values the cut hits of a page carry.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum CutAnchors {
    #[default]
    None,
    Match,
    Start,
    Mixed,
}

impl CutAnchors {
    /// Folds one more cut hit's anchor word in.
    pub(crate) fn with(self, anchor: &str) -> Self {
        let next = if anchor == "match" { Self::Match } else { Self::Start };
        match self {
            Self::None => next,
            current if current == next => current,
            _ => Self::Mixed,
        }
    }
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
            cut_anchors: CutAnchors::None,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<UnavailableWord>,
    /// `c<epoch>-<hash>` or null.
    pub continuation: Option<String>,
    /// Present (false) in embedded mode, where the host may write between
    /// pages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation_exact: Option<bool>,
    /// Opaque session handle for the frozen read context (present when `freeze: true`
    /// or `frozen_context` is supplied).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frozen_context: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_revision_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_version_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_revision_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_source_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_lifecycle_state: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_revision_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_evidence_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_edge_artifact_revision_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_edge_evidence_ref: Option<String>,
}

/// `expand` result. `bound` names the cap that was reached, or is null.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ExpandOutput {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub targets: Vec<Target>,
    pub complete: bool,
    pub bound: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frozen_context: Option<String>,
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

/// `actuate` and `put_latest_state` result.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ActuateOutput {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub operation_id: String,
    pub request_sha256: String,
    pub outcome: String,
    pub affected_revision_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resulting_write_boundary: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resulting_dependency_generation: Option<u64>,
    pub pending_projection_write_cursors: Vec<u64>,
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
