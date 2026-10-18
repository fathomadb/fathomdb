//! Hints: FathomDB-authored suggestions for the next call, derived from facts
//! a search already holds (ADR-0.8.29-agent-hints-sidecar).
//!
//! This is the engine subset shared by every SDK: `no_results`, `degraded`
//! and `more_results`. It is pure — no SQL, I/O, clock or locks — so the MCP
//! adapter calls it with its own page facts and the rule cannot drift between
//! layers. Every hint value is a closed-enum word or a number; a hint never
//! carries record content.

/// The largest `limit` a `more_results` hint suggests: the ranked-result cap
/// of the MCP adapter and the bindings.
pub const HINT_LIMIT_CAP: usize = 100;

/// What a hint suggests doing.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum HintAction {
    /// Run a search (again, or with the hint's arguments).
    Search,
    /// Nothing to do; the hint explains the result.
    None,
}

impl HintAction {
    /// The wire word (`search`, `none`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::None => "none",
        }
    }
}

/// Why a hint was emitted.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum HintReason {
    /// The search returned zero hits.
    NoResults,
    /// A degradation entry is present: an arm did not run, or ran degraded.
    Degraded,
    /// The page reached its limit, so more hits may exist.
    MoreResults,
}

impl HintReason {
    /// The wire word (`no_results`, `degraded`, `more_results`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoResults => "no_results",
            Self::Degraded => "degraded",
            Self::MoreResults => "more_results",
        }
    }
}

/// A search path, as a fact and as a suggestion.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum HintMode {
    /// Lexical only.
    Text,
    /// Lexical plus the vector arm.
    Hybrid,
}

impl HintMode {
    /// The wire word (`text`, `hybrid`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Hybrid => "hybrid",
        }
    }
}

/// The arm a `degraded` hint names.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum HintArm {
    /// The vector arm of hybrid search.
    Vector,
    /// The text arm.
    Text,
    /// The edge-text arm.
    TextEdge,
    /// The graph arm of hybrid search.
    GraphArm,
    /// Graph expansion.
    Graph,
}

impl HintArm {
    /// The wire word (`vector`, `text`, `text_edge`, `graph_arm`, `graph`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Vector => "vector",
            Self::Text => "text",
            Self::TextEdge => "text_edge",
            Self::GraphArm => "graph_arm",
            Self::Graph => "graph",
        }
    }
}

/// Why an arm degraded: every cause word a `degraded` hint may carry.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum HintCause {
    /// Hybrid search fell back to one branch (`soft_fallback`).
    SoftFallback,
    /// The dense arm is disabled on this engine (`dense_disabled`).
    DenseDisabled,
    /// No usable embedder (`embedder_unavailable`).
    EmbedderUnavailable,
    /// The query embedding was refused by a full embed queue
    /// (`embedder_overloaded`).
    EmbedderOverloaded,
    /// Graph: the query seed fell back to text (`query_seed_text_fallback`).
    QuerySeedTextFallback,
    /// Graph: the projection is legacy and unverified
    /// (`projection_legacy_unverified`).
    ProjectionLegacyUnverified,
    /// Graph: the projection is still processing (`projection_processing`).
    ProjectionProcessing,
    /// Graph: the projection is blocked (`projection_blocked`).
    ProjectionBlocked,
    /// Graph: the projection is deferred (`projection_deferred`).
    ProjectionDeferred,
    /// Graph: the projection is degraded (`projection_degraded`).
    ProjectionDegraded,
}

impl HintCause {
    /// The wire word, as listed on each variant.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SoftFallback => "soft_fallback",
            Self::DenseDisabled => "dense_disabled",
            Self::EmbedderUnavailable => "embedder_unavailable",
            Self::EmbedderOverloaded => "embedder_overloaded",
            Self::QuerySeedTextFallback => "query_seed_text_fallback",
            Self::ProjectionLegacyUnverified => "projection_legacy_unverified",
            Self::ProjectionProcessing => "projection_processing",
            Self::ProjectionBlocked => "projection_blocked",
            Self::ProjectionDeferred => "projection_deferred",
            Self::ProjectionDegraded => "projection_degraded",
        }
    }
}

/// One suggested next action. Which optional fields are set depends on
/// `reason`:
///
/// - `NoResults` (`action: Search`): `mode` is `Some(Hybrid)` only when the
///   search ran in text mode and hybrid is available; otherwise `None`
///   (rephrase).
/// - `Degraded` (`action: None`): `arm` and `cause` are set.
/// - `MoreResults` (`action: Search`): `limit` is the suggested limit.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct Hint {
    /// What to do.
    pub action: HintAction,
    /// Why.
    pub reason: HintReason,
    /// Suggested search mode (`NoResults`).
    pub mode: Option<HintMode>,
    /// The degraded arm (`Degraded`).
    pub arm: Option<HintArm>,
    /// Why it degraded (`Degraded`).
    pub cause: Option<HintCause>,
    /// Suggested limit (`MoreResults`).
    pub limit: Option<usize>,
}

/// The facts of one returned search page that the engine subset reads.
/// Build with [`SearchFacts::new`]; the defaults are no degradation and no
/// hybrid route.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct SearchFacts<'a> {
    /// Hits returned on this page.
    pub returned: usize,
    /// The page's limit.
    pub limit: usize,
    /// The mode the search ran in.
    pub mode: HintMode,
    /// Degradation entries in the caller's canonical order; only the first
    /// is named in a hint.
    pub degradations: &'a [(HintArm, HintCause)],
    /// Whether a hybrid run of the same query could use the vector arm: the
    /// store has a usable embedder, the configuration permits hybrid, and the
    /// dense arm is not disabled. Read only for text-mode searches.
    pub hybrid_available: bool,
}

impl<'a> SearchFacts<'a> {
    /// Facts for a page of `returned` hits under `limit` in `mode`.
    #[must_use]
    pub const fn new(returned: usize, limit: usize, mode: HintMode) -> Self {
        Self { returned, limit, mode, degradations: &[], hybrid_available: false }
    }

    /// Set the degradation entries.
    #[must_use]
    pub const fn with_degradations(mut self, degradations: &'a [(HintArm, HintCause)]) -> Self {
        self.degradations = degradations;
        self
    }

    /// Set whether hybrid is available (see the field).
    #[must_use]
    pub const fn with_hybrid_available(mut self, hybrid_available: bool) -> Self {
        self.hybrid_available = hybrid_available;
        self
    }
}

/// Append the engine-subset hints for one search page to `out`, at most one
/// per reason, in the order `degraded`, `no_results`, `more_results`.
///
/// - `degraded` when `facts.degradations` is non-empty, naming its first
///   entry.
/// - `no_results` when `facts.returned` is zero, with `mode: Hybrid` only for
///   a text-mode search with `hybrid_available`.
/// - `more_results` when the page reached its limit and the limit is below
///   [`HINT_LIMIT_CAP`], suggesting the smaller of twice the limit and the
///   cap.
///
/// Never reads anything but `facts` and never panics. Allocates only when it
/// pushes onto an `out` without spare capacity.
pub fn search_hints(facts: &SearchFacts<'_>, out: &mut Vec<Hint>) {
    let empty = Hint {
        action: HintAction::Search,
        reason: HintReason::NoResults,
        mode: None,
        arm: None,
        cause: None,
        limit: None,
    };
    if let Some(&(arm, cause)) = facts.degradations.first() {
        out.push(Hint {
            action: HintAction::None,
            reason: HintReason::Degraded,
            arm: Some(arm),
            cause: Some(cause),
            ..empty
        });
    }
    if facts.returned == 0 {
        let suggest_hybrid = facts.mode == HintMode::Text && facts.hybrid_available;
        out.push(Hint { mode: suggest_hybrid.then_some(HintMode::Hybrid), ..empty });
    } else if facts.returned >= facts.limit && facts.limit < HINT_LIMIT_CAP {
        out.push(Hint {
            reason: HintReason::MoreResults,
            limit: Some(facts.limit.saturating_mul(2).min(HINT_LIMIT_CAP)),
            ..empty
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hints(facts: &SearchFacts<'_>) -> Vec<Hint> {
        let mut out = Vec::new();
        search_hints(facts, &mut out);
        out
    }

    fn no_results(mode: Option<HintMode>) -> Hint {
        Hint {
            action: HintAction::Search,
            reason: HintReason::NoResults,
            mode,
            arm: None,
            cause: None,
            limit: None,
        }
    }

    #[test]
    fn empty_text_search_with_hybrid_available_suggests_hybrid() {
        let facts = SearchFacts::new(0, 10, HintMode::Text).with_hybrid_available(true);
        assert_eq!(hints(&facts), vec![no_results(Some(HintMode::Hybrid))]);
    }

    /// Condition 4 of hints-sidecar § 3.3 (`dense_disabled`), and conditions
    /// 2 and 3, reach the engine subset as `hybrid_available = false`.
    #[test]
    fn empty_text_search_without_hybrid_suggests_rephrasing_without_mode() {
        let facts = SearchFacts::new(0, 10, HintMode::Text).with_hybrid_available(false);
        assert_eq!(hints(&facts), vec![no_results(None)]);
    }

    #[test]
    fn empty_hybrid_search_never_suggests_a_mode() {
        let facts = SearchFacts::new(0, 10, HintMode::Hybrid).with_hybrid_available(true);
        assert_eq!(hints(&facts), vec![no_results(None)]);
    }

    #[test]
    fn degradation_names_the_first_entry_before_no_results() {
        let degradations = [
            (HintArm::Vector, HintCause::DenseDisabled),
            (HintArm::Vector, HintCause::EmbedderUnavailable),
        ];
        let facts = SearchFacts::new(0, 10, HintMode::Hybrid).with_degradations(&degradations);
        let degraded = Hint {
            action: HintAction::None,
            reason: HintReason::Degraded,
            mode: None,
            arm: Some(HintArm::Vector),
            cause: Some(HintCause::DenseDisabled),
            limit: None,
        };
        assert_eq!(hints(&facts), vec![degraded, no_results(None)]);
    }

    #[test]
    fn a_full_page_suggests_twice_the_limit_up_to_the_cap() {
        let more = |limit| Hint {
            action: HintAction::Search,
            reason: HintReason::MoreResults,
            mode: None,
            arm: None,
            cause: None,
            limit: Some(limit),
        };
        assert_eq!(hints(&SearchFacts::new(10, 10, HintMode::Text)), vec![more(20)]);
        assert_eq!(hints(&SearchFacts::new(60, 60, HintMode::Text)), vec![more(100)]);
        assert_eq!(hints(&SearchFacts::new(100, 100, HintMode::Text)), vec![], "at the cap");
    }

    #[test]
    fn a_short_undegraded_page_has_no_hints() {
        assert_eq!(hints(&SearchFacts::new(3, 10, HintMode::Text)), vec![]);
    }

    #[test]
    fn hints_are_appended_in_reason_order_after_existing_entries() {
        let degradations = [(HintArm::Vector, HintCause::EmbedderOverloaded)];
        let facts = SearchFacts::new(10, 10, HintMode::Hybrid).with_degradations(&degradations);
        let mut out = vec![no_results(None)];
        search_hints(&facts, &mut out);
        let reasons: Vec<HintReason> = out.iter().map(|hint| hint.reason).collect();
        assert_eq!(
            reasons,
            vec![HintReason::NoResults, HintReason::Degraded, HintReason::MoreResults]
        );
    }

    #[test]
    fn every_cause_and_arm_has_its_wire_word() {
        let causes = [
            (HintCause::SoftFallback, "soft_fallback"),
            (HintCause::DenseDisabled, "dense_disabled"),
            (HintCause::EmbedderUnavailable, "embedder_unavailable"),
            (HintCause::EmbedderOverloaded, "embedder_overloaded"),
            (HintCause::QuerySeedTextFallback, "query_seed_text_fallback"),
            (HintCause::ProjectionLegacyUnverified, "projection_legacy_unverified"),
            (HintCause::ProjectionProcessing, "projection_processing"),
            (HintCause::ProjectionBlocked, "projection_blocked"),
            (HintCause::ProjectionDeferred, "projection_deferred"),
            (HintCause::ProjectionDegraded, "projection_degraded"),
        ];
        for (cause, word) in causes {
            assert_eq!(cause.as_str(), word);
        }
        let arms = [
            (HintArm::Vector, "vector"),
            (HintArm::Text, "text"),
            (HintArm::TextEdge, "text_edge"),
            (HintArm::GraphArm, "graph_arm"),
            (HintArm::Graph, "graph"),
        ];
        for (arm, word) in arms {
            assert_eq!(arm.as_str(), word);
        }
    }
}
