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
