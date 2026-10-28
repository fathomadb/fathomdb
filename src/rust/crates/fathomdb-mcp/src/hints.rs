//! The hints sidecar (ADR-0.8.29; `dev/design/agent-access/hints-sidecar.md`):
//! plan the hints of a response once from facts the call already holds, emit
//! them per budget-loop iteration, and render their text lines from
//! compile-time templates. Pure: no I/O, no clock.

use fathomdb_sdk::{search_hints, HintArm, HintCause, HintMode, SearchFacts};

use crate::output::{
    CutAnchors, Degradation, Hint, HintAction, HintReason, Hit, ModeWord, PerHitOut,
};
use crate::shape;

/// Refs listed in one cut-ref hint (`MAX_FETCH_REFS`), so the hint is one
/// executable `fetch` call.
pub(crate) const MAX_HINT_REFS: usize = 50;

/// A ref longer than this is left out of hints, which bounds what a
/// writer-chosen id can carry inside an imperative-looking hint (§ 2.3).
pub(crate) const MAX_HINT_REF_BYTES: usize = 128;

/// The smallest `max_tokens` a call accepts.
const MIN_MAX_TOKENS: u64 = 256;

/// One search page hit: `(hit, write cursor, per-hit explanation)`.
pub(crate) type PageHit = (Hit, u64, Option<PerHitOut>);

/// The facts of a search page that its hints are planned from.
pub(crate) struct SearchPageFacts<'a> {
    pub(crate) page: &'a [PageHit],
    pub(crate) page_limit: usize,
    pub(crate) hybrid: bool,
    /// Canonical order; only the first entry is named.
    pub(crate) degradation: &'a [Degradation],
    pub(crate) hybrid_available: bool,
}

/// The hints of one search response, planned once before its budget loop;
/// each iteration emits from the plan's prefix below `kept`.
#[derive(Default)]
pub(crate) struct SearchPlan {
    enabled: bool,
    /// Engine-subset hints, in emission order.
    engine: Vec<fathomdb_sdk::Hint>,
    /// Page indices of the hits whose snippet was cut, in rank order.
    snippet_cut: Vec<usize>,
}

impl SearchPlan {
    /// Plan a page's hints; with hints off the plan is empty and nothing is
    /// allocated.
    pub(crate) fn new(enabled: bool, facts: &SearchPageFacts<'_>) -> Self {
        if !enabled {
            return Self::default();
        }
        let first = facts.degradation.first().and_then(engine_degradation);
        let degradations = first.as_slice();
        let mode = if facts.hybrid { HintMode::Hybrid } else { HintMode::Text };
        let engine_facts = SearchFacts::new(facts.page.len(), facts.page_limit, mode)
            .with_degradations(degradations)
            .with_hybrid_available(facts.hybrid_available);
        let mut engine = Vec::new();
        search_hints(&engine_facts, &mut engine);
        let cut_count = facts.page.iter().filter(|(hit, _, _)| hit.snippet_cut).count();
        let mut snippet_cut = Vec::new();
        if cut_count > 0 {
            snippet_cut.reserve_exact(cut_count);
            snippet_cut.extend(
                facts
                    .page
                    .iter()
                    .enumerate()
                    .filter(|(_, (hit, _, _))| hit.snippet_cut)
                    .map(|(index, _)| index),
            );
        }
        Self { enabled, engine, snippet_cut }
    }

    /// Whether hints are on for this response.
    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// The hints of the response that keeps the first `kept` hits of `page`
    /// and carries `continuation`; `cut` when the budget dropped hits.
    /// `more_results` becomes the adapter's continuation form, is dropped
    /// without a continuation, and yields to `cut_by_budget` on a cut.
    pub(crate) fn emit(
        &self,
        page: &[PageHit],
        kept: usize,
        continuation: Option<&str>,
        cut: bool,
        max_tokens: u32,
    ) -> Vec<Hint> {
        if !self.enabled {
            return Vec::new();
        }
        let mut hints = Vec::with_capacity(4);
        let mut more = false;
        for hint in &self.engine {
            if matches!(hint.reason, fathomdb_sdk::HintReason::MoreResults) {
                more = true;
                continue;
            }
            if let Ok(hint) = Hint::try_from(*hint) {
                hints.push(hint);
            }
        }
        let kept_cut = self.snippet_cut.iter().take_while(|index| **index < kept);
        if let Some(mut hint) = fetch_hint(
            HintReason::SnippetCut,
            kept_cut.clone().map(|index| page[*index].0.reference.as_str()),
        ) {
            hint.cut_anchors = kept_cut.fold(CutAnchors::None, |anchors, index| {
                anchors.with(&page[*index].0.snippet_anchor)
            });
            hints.push(hint);
        }
        if let Some(continuation) = continuation {
            if cut {
                let mut hint = Hint::new(HintAction::Search, HintReason::CutByBudget);
                hint.continuation = Some(continuation.to_string());
                hint.max_tokens = Some(u64::from(max_tokens));
                hints.push(hint);
            } else if more {
                let mut hint = Hint::new(HintAction::Search, HintReason::MoreResults);
                hint.continuation = Some(continuation.to_string());
                hints.push(hint);
            }
        }
        hints
    }
}

/// The `body_cut` hint of a `fetch` or `expand` response: the cut refs in
/// result order, and `max_tokens`, the whole-response estimate with every
/// body uncut, which is enough to fetch them whole.
pub(crate) fn body_cut<'a>(refs: impl Iterator<Item = &'a str>, max_tokens: u64) -> Option<Hint> {
    let mut hint = fetch_hint(HintReason::BodyCut, refs)?;
    hint.max_tokens = Some(max_tokens.max(MIN_MAX_TOKENS));
    Some(hint)
}

/// A `fetch` hint listing `refs` in order, up to [`MAX_HINT_REFS`] of at
/// most [`MAX_HINT_REF_BYTES`] each, with the rest counted; `None` when
/// nothing is listed.
fn fetch_hint<'a>(reason: HintReason, refs: impl Iterator<Item = &'a str>) -> Option<Hint> {
    let mut listed = Vec::new();
    let mut omitted = 0_u32;
    for reference in refs {
        if reference.len() <= MAX_HINT_REF_BYTES && listed.len() < MAX_HINT_REFS {
            listed.push(reference.to_string());
        } else {
            omitted += 1;
        }
    }
    if listed.is_empty() {
        return None;
    }
    let mut hint = Hint::new(HintAction::Fetch, reason);
    hint.refs = Some(listed);
    hint.refs_omitted = (omitted > 0).then_some(omitted);
    Some(hint)
}

fn engine_degradation(entry: &Degradation) -> Option<(HintArm, HintCause)> {
    Some((arm(&entry.arm)?, cause(&entry.reason)?))
}

fn arm(word: &str) -> Option<HintArm> {
    Some(match word {
        "vector" => HintArm::Vector,
        "text" => HintArm::Text,
        "text_edge" => HintArm::TextEdge,
        "graph_arm" => HintArm::GraphArm,
        "graph" => HintArm::Graph,
        _ => return None,
    })
}

fn cause(word: &str) -> Option<HintCause> {
    Some(match word {
        "soft_fallback" => HintCause::SoftFallback,
        "dense_disabled" => HintCause::DenseDisabled,
        "embedder_unavailable" => HintCause::EmbedderUnavailable,
        "embedder_overloaded" => HintCause::EmbedderOverloaded,
        "query_seed_text_fallback" => HintCause::QuerySeedTextFallback,
        "projection_legacy_unverified" => HintCause::ProjectionLegacyUnverified,
        "projection_processing" => HintCause::ProjectionProcessing,
        "projection_blocked" => HintCause::ProjectionBlocked,
        "projection_deferred" => HintCause::ProjectionDeferred,
        "projection_degraded" => HintCause::ProjectionDegraded,
        _ => return None,
    })
}

fn word<T: serde::Serialize>(value: T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(word)) => word,
        _ => String::new(),
    }
}

// One fixed sentence per (action, reason); no hint text is built from data.
fn template(hint: &Hint) -> &'static str {
    match hint.reason {
        HintReason::Degraded => "an arm did not run or ran degraded; nothing to retry",
        HintReason::NoResults if hint.mode.is_some() => {
            "try this mode, or rephrase with synonyms, names or related events"
        }
        HintReason::NoResults => "rephrase with synonyms, names or related events",
        HintReason::SnippetCut => match hint.cut_anchors {
            CutAnchors::Match => "snippets are centred on the match; fetch to read the full record",
            CutAnchors::Mixed => {
                "snippets are centred on the match or are the record start; fetch to read the \
                 full record"
            }
            CutAnchors::Start | CutAnchors::None => {
                "snippets are the record start; fetch to read the full record"
            }
        },
        HintReason::BodyCut => {
            "bodies were shortened; fetch with this max_tokens to read them whole"
        }
        // A continuation call refuses query, limit and filters.
        HintReason::CutByBudget => {
            "results were dropped to fit max_tokens; for the rest call search \
             {\"continuation\": \"<c>\", \"max_tokens\": <n>}: pass only the continuation \
             and max_tokens; do not repeat query or limit"
        }
        HintReason::MoreResults => {
            "the page reached its limit; for more call search {\"continuation\": \"<c>\"}: \
             pass only the continuation; do not repeat query or limit"
        }
        HintReason::DefaultStoreSet => "calls without store use this store",
    }
}

/// Append one text line per hint: `hint: <action> <fields> (<reason>:
/// <template>)`. Refs and store names are JSON-quoted.
pub(crate) fn render(hints: &[Hint], text: &mut String) {
    for hint in hints {
        text.push_str("\nhint: ");
        text.push_str(&word(hint.action));
        if let Some(refs) = &hint.refs {
            text.push(' ');
            text.push_str(&serde_json::to_string(refs).expect("strings serialize"));
        }
        if let Some(omitted) = hint.refs_omitted {
            text.push_str(&format!(" refs_omitted={omitted}"));
        }
        if let Some(mode) = hint.mode {
            text.push_str(match mode {
                ModeWord::Text => " mode=text",
                ModeWord::Hybrid => " mode=hybrid",
            });
        }
        if let Some(arm) = hint.arm {
            text.push_str(&format!(" arm={}", word(arm)));
        }
        if let Some(cause) = hint.cause {
            text.push_str(&format!(" cause={}", word(cause)));
        }
        if let Some(continuation) = &hint.continuation {
            text.push_str(&format!(" continuation={continuation}"));
        }
        if let Some(max_tokens) = hint.max_tokens {
            text.push_str(&format!(" max_tokens={max_tokens}"));
        }
        if let Some(store) = &hint.store {
            text.push_str(&format!(" store={}", shape::quote(store)));
        }
        text.push_str(&format!(" ({}: {})", word(hint.reason), template(hint)));
    }
}

/// Whether a call's result carries hints: its own argument, else the
/// configured key.
pub(crate) fn enabled(given: Option<bool>, configured: bool) -> bool {
    given.unwrap_or(configured)
}

/// The `list_stores` hint: the session has a default store, so store-less
/// calls work.
pub(crate) fn default_store_set(store: Option<&str>) -> Option<Hint> {
    let mut hint = Hint::new(HintAction::Search, HintReason::DefaultStoreSet);
    hint.store = Some(store?.to_string());
    Some(hint)
}

/// The `degraded` hint of a non-search response (`expand`): one hint naming
/// the first entry of the canonical degradation list.
pub(crate) fn degraded(degradation: &[Degradation]) -> Option<Hint> {
    let (arm, cause) = engine_degradation(degradation.first()?)?;
    let mut hint = Hint::new(HintAction::None, HintReason::Degraded);
    hint.arm = Some(crate::output::HintArm::from_engine(arm)?);
    hint.cause = Some(crate::output::HintCause::from_engine(cause)?);
    Some(hint)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(arm: &str, reason: &str) -> Degradation {
        Degradation { arm: arm.to_string(), reason: reason.to_string() }
    }

    /// Hints-sidecar § 3.3: `expand` gets `degraded` for its graph codes.
    /// The engine reports no graph degradation on an explicit-seed expansion
    /// over a ready store, so the rule is tested here, on canonical entries.
    #[test]
    fn expand_degradation_names_the_first_graph_code() {
        let entries = shape::canonical_degradation(vec![
            entry("graph", "projection_processing"),
            entry("graph", "projection_blocked"),
        ]);
        let hint = degraded(&entries).expect("a degraded hint");
        assert_eq!(
            serde_json::to_value(&hint).unwrap(),
            serde_json::json!({
                "action": "none",
                "reason": "degraded",
                "arm": "graph",
                "cause": "projection_blocked"
            })
        );
        assert!(degraded(&[]).is_none());
    }
}
