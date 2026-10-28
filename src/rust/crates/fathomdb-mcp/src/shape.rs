//! Response shaping: pure functions from governed results to agent-shaped
//! output — snippets, the budget estimator, completeness, deterministic
//! handles and text rendering. No I/O, no clock, no rmcp.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::config::SearchMode;
use crate::output::{Completeness, Degradation};

/// Cut `body` to at most `cap` characters on a character boundary. Returns
/// the text and whether anything was cut.
#[must_use]
pub fn snippet(body: &str, cap: usize) -> (String, bool) {
    match body.char_indices().nth(cap) {
        None => (body.to_string(), false),
        Some((end, _)) => (body[..end].to_string(), true),
    }
}

/// The budget estimator (MR-041): ceil(bytes / 4).
#[must_use]
pub fn estimate_tokens(bytes: usize) -> u64 {
    (bytes as u64).div_ceil(4)
}

/// Insert `budget` into `structured` and return it with the estimate of the
/// whole response (structured content including `budget`, plus `text`).
///
/// `used_tokens` is computed before `budget` is inserted, as the interface
/// defines it, so it is stable; the returned whole-response estimate is what
/// a caller compares with `max_tokens` (MR-041.1).
#[must_use]
pub fn with_budget(mut structured: Value, text: &str, max_tokens: u32, cut: bool) -> (Value, u64) {
    if let Value::Object(object) = &mut structured {
        object.remove("budget");
    }
    let without = serde_json::to_string(&structured).expect("serializable").len();
    let used = estimate_tokens(without + text.len());
    if let Value::Object(object) = &mut structured {
        object.insert(
            "budget".to_string(),
            serde_json::json!({"max_tokens": max_tokens, "used_tokens": used, "cut": cut}),
        );
    }
    let whole = serde_json::to_string(&structured).expect("serializable").len();
    (structured, estimate_tokens(whole + text.len()))
}

/// Search completeness (MR-030, MR-032.3), from the page actually returned.
#[must_use]
pub fn completeness(
    returned: usize,
    page_hits: usize,
    page_limit: usize,
    cut_by_budget: bool,
    degraded: bool,
) -> Completeness {
    if cut_by_budget {
        return Completeness::CutByBudget;
    }
    if returned == 0 {
        return if degraded { Completeness::EmptyNotReady } else { Completeness::Empty };
    }
    if page_hits >= page_limit {
        Completeness::LimitReached
    } else if degraded {
        Completeness::Partial
    } else {
        Completeness::Complete
    }
}

/// The engine call that serves the page starting at `offset`: returns
/// `(run_limit, page_limit)` — how many ranked hits to request (never more
/// than `ranked_cap`) and how many of them belong to this page.
#[must_use]
pub fn page_window(offset: usize, limit: usize, ranked_cap: usize) -> (usize, usize) {
    let run_limit = (offset + limit).min(ranked_cap);
    (run_limit, run_limit.saturating_sub(offset))
}

/// Where the next page starts, or `None` when there is no next page: the
/// page kept `kept` of its `page_hits` hits (fewer when the budget cut it)
/// out of a window of `page_limit`.
#[must_use]
pub fn next_offset(
    offset: usize,
    kept: usize,
    page_hits: usize,
    page_limit: usize,
    ranked_cap: usize,
) -> Option<usize> {
    // A page that kept nothing offers no continuation: repeating it under
    // the same budget would never advance.
    let more = kept < page_hits || page_hits >= page_limit;
    let next = offset + kept;
    (more && kept > 0 && next < ranked_cap).then_some(next)
}

/// Sort and deduplicate degradation entries so output order is canonical.
#[must_use]
pub fn canonical_degradation(mut entries: Vec<Degradation>) -> Vec<Degradation> {
    entries.sort();
    entries.dedup();
    entries
}

/// The canonical form of a search request: the arguments with defaults
/// resolved and the validity instant left out unless the caller pinned it,
/// so no handle is derived from the clock (MR-043.2).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CanonicalSearch {
    pub query: String,
    pub mode: &'static str,
    pub kind: Option<String>,
    pub source_type: Option<String>,
    pub status: Option<String>,
    pub created_after: Option<i64>,
    pub attributes: Vec<(String, String)>,
    pub as_of: Option<i64>,
    pub limit: usize,
    pub explain: bool,
}

impl CanonicalSearch {
    pub(crate) fn search_mode(&self) -> SearchMode {
        if self.mode == "hybrid" {
            SearchMode::Hybrid
        } else {
            SearchMode::Text
        }
    }
}

/// A deterministic handle `<prefix><epoch>-<16 hex>`: SHA-256 over the parts,
/// each length-prefixed so no two part lists collide.
#[must_use]
pub fn handle(prefix: char, epoch: u64, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    let digest = hasher.finalize();
    let hex: String = digest.iter().take(8).map(|byte| format!("{byte:02x}")).collect();
    format!("{prefix}{epoch}-{hex}")
}

/// Content is quoted as a JSON string in text renderings, so it reads as
/// data, never as instructions or structure.
#[must_use]
pub fn quote(text: &str) -> String {
    serde_json::to_string(text).expect("strings serialize")
}

/// Render degradation entries for text blocks.
#[must_use]
pub fn render_degradation(entries: &[Degradation]) -> String {
    if entries.is_empty() {
        return "none".to_string();
    }
    entries
        .iter()
        .map(|entry| format!("{}({})", entry.arm, entry.reason))
        .collect::<Vec<_>>()
        .join(",")
}

/// Shorten the longest bodies until `excess` (bytes over budget for the
/// current bodies) reaches zero or every body is empty. `bodies` holds
/// `(text, cut)` pairs. Each body appears in both the structured content and
/// the text block, so each removed character saves at least two bytes.
pub fn cut_bodies(
    bodies: &mut [(String, bool)],
    mut excess: impl FnMut(&[(String, bool)]) -> usize,
) {
    loop {
        let over = excess(bodies);
        if over == 0 {
            return;
        }
        let Some(longest) = bodies
            .iter()
            .enumerate()
            .filter(|(_, (body, _))| !body.is_empty())
            .max_by_key(|(index, (body, _))| (body.chars().count(), usize::MAX - index))
            .map(|(index, _)| index)
        else {
            return;
        };
        let (body, cut) = &mut bodies[longest];
        let chars = body.chars().count();
        let keep = chars.saturating_sub(over.div_ceil(2).max(1));
        let (shorter, _) = snippet(body, keep);
        *body = shorter;
        *cut = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn snippets_are_char_boundary_prefixes(body in "\\PC{0,400}", cap in 0_usize..500) {
            let (cut_text, cut) = snippet(&body, cap);
            prop_assert!(body.starts_with(&cut_text));
            let chars = body.chars().count();
            prop_assert_eq!(cut_text.chars().count(), chars.min(cap));
            prop_assert_eq!(cut, chars > cap);
        }

        #[test]
        fn the_estimator_is_ceil_bytes_over_four(bytes in 0_usize..1_000_000) {
            let tokens = estimate_tokens(bytes);
            prop_assert!(tokens * 4 >= bytes as u64);
            prop_assert!(tokens * 4 < bytes as u64 + 4);
        }

        #[test]
        fn used_tokens_exclude_the_budget_and_the_whole_estimate_includes_it(
            text in "\\PC{0,300}",
            payload in "\\PC{0,300}",
            max_tokens in 256_u32..100_000,
            cut in any::<bool>(),
        ) {
            let structured = serde_json::json!({"payload": payload, "budget": {"stale": true}});
            let (with, whole) = with_budget(structured, &text, max_tokens, cut);
            let mut without = with.clone();
            without.as_object_mut().unwrap().remove("budget");
            let used = estimate_tokens(serde_json::to_string(&without).unwrap().len() + text.len());
            prop_assert_eq!(&with["budget"]["used_tokens"], &serde_json::json!(used));
            prop_assert_eq!(&with["budget"]["max_tokens"], &serde_json::json!(max_tokens));
            prop_assert_eq!(&with["budget"]["cut"], &serde_json::json!(cut));
            prop_assert_eq!(
                whole,
                estimate_tokens(serde_json::to_string(&with).unwrap().len() + text.len())
            );
            prop_assert!(whole >= used);
        }

        #[test]
        fn cut_bodies_fits_or_empties_and_only_shortens(
            bodies in proptest::collection::vec("\\PC{0,200}", 0..12),
            budget in 0_usize..3_000,
        ) {
            let original = bodies.clone();
            let mut state: Vec<(String, bool)> = bodies.into_iter().map(|body| (body, false)).collect();
            let size = |state: &[(String, bool)]| -> usize {
                state.iter().map(|(body, _)| 2 * serde_json::to_string(body).unwrap().len()).sum()
            };
            cut_bodies(&mut state, |state| size(state).saturating_sub(budget));
            let fits = size(&state) <= budget;
            prop_assert!(fits || state.iter().all(|(body, _)| body.is_empty()));
            for ((body, cut), before) in state.iter().zip(&original) {
                prop_assert!(before.starts_with(body.as_str()));
                prop_assert_eq!(*cut, body.len() < before.len());
            }
        }

        #[test]
        fn completeness_never_claims_complete_while_degraded(
            returned in 0_usize..120,
            extra in 0_usize..20,
            page_limit in 1_usize..100,
            cut in any::<bool>(),
            degraded in any::<bool>(),
        ) {
            let page_hits = returned + extra;
            let value = completeness(returned, page_hits, page_limit, cut, degraded);
            if degraded {
                prop_assert_ne!(value, Completeness::Complete);
            }
            if cut {
                prop_assert_eq!(value, Completeness::CutByBudget);
            } else if returned == 0 {
                prop_assert!(matches!(value, Completeness::Empty | Completeness::EmptyNotReady));
            }
        }

        #[test]
        fn handles_are_deterministic_and_part_boundaries_matter(
            a in "\\PC{0,20}", b in "\\PC{0,20}", epoch in 0_u64..1000,
        ) {
            let one = handle('r', epoch, &[&a, &b]);
            prop_assert_eq!(&one, &handle('r', epoch, &[&a, &b]));
            let prefix = format!("r{epoch}-");
            prop_assert!(one.starts_with(&prefix));
            prop_assert_eq!(one.len(), prefix.len() + 16);
            if !b.is_empty() {
                let joined = format!("{a}{b}");
                prop_assert_ne!(one, handle('r', epoch, &[&joined, ""]));
            }
        }

        /// Following continuations over an unchanged ranked list returns
        /// every hit up to the ranked cap once, in order, whatever each page's
        /// budget keeps (MR-041.2).
        #[test]
        fn continuation_pages_have_no_gaps_or_duplicates(
            total in 0_usize..160,
            limit in 1_usize..=100,
            keeps in proptest::collection::vec(1_usize..=100, 1..200),
        ) {
            let cap = 100;
            let ranked: Vec<usize> = (0..total).collect();
            let mut seen = Vec::new();
            let mut offset = Some(0_usize);
            let mut pages = 0;
            while let Some(start) = offset {
                let (run_limit, page_limit) = page_window(start, limit, cap);
                prop_assert!(run_limit <= cap);
                let page: Vec<usize> = ranked.iter().copied().take(run_limit).skip(start).collect();
                prop_assert!(page.len() <= page_limit);
                let kept = page.len().min(keeps[pages % keeps.len()]);
                seen.extend_from_slice(&page[..kept]);
                offset = next_offset(start, kept, page.len(), page_limit, cap);
                pages += 1;
                prop_assert!(pages <= 200, "paging must terminate");
            }
            let expected: Vec<usize> = ranked.into_iter().take(cap).collect();
            prop_assert_eq!(seen, expected);
        }
    }

    #[test]
    fn length_prefixing_separates_parts() {
        assert_ne!(handle('c', 1, &["ab", "c"]), handle('c', 1, &["a", "bc"]));
    }
}
