//! The hints sidecar (ADR-0.8.29; `dev/design/agent-access/hints-sidecar.md`):
//! plan the hints of a response once from facts the call already holds, emit
//! them per budget-loop iteration, and render their text lines from
//! compile-time templates. Pure: no I/O, no clock.

use fathomdb_sdk::{search_hints, HintArm, HintCause, HintMode, SearchFacts};

use crate::output::{Degradation, Hint, HintReason, ModeWord};
use crate::shape;

/// The engine-subset facts of a search page, as the adapter knows them.
pub(crate) struct SearchPageFacts<'a> {
    pub(crate) page_hits: usize,
    pub(crate) page_limit: usize,
    pub(crate) hybrid: bool,
    /// Canonical order; only the first entry is named.
    pub(crate) degradation: &'a [Degradation],
    pub(crate) hybrid_available: bool,
}

/// The hints of one search response, planned before its budget loop.
#[derive(Default)]
pub(crate) struct SearchPlan {
    /// Engine-subset hints, in emission order.
    engine: Vec<fathomdb_sdk::Hint>,
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
        let engine_facts = SearchFacts::new(facts.page_hits, facts.page_limit, mode)
            .with_degradations(degradations)
            .with_hybrid_available(facts.hybrid_available);
        let mut engine = Vec::new();
        search_hints(&engine_facts, &mut engine);
        Self { engine }
    }

    /// The hints of the response that keeps the first `kept` page hits.
    pub(crate) fn emit(&self) -> Vec<Hint> {
        let mut hints = Vec::new();
        for hint in &self.engine {
            if matches!(hint.reason, fathomdb_sdk::HintReason::MoreResults) {
                continue;
            }
            if let Ok(hint) = Hint::try_from(*hint) {
                hints.push(hint);
            }
        }
        hints
    }
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
        HintReason::MoreResults => "the page reached its limit; pass the continuation for more",
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
