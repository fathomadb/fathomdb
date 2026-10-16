//! Context pack: render ranked caller items into prompt context within a
//! character budget, with full and compact tiers, and an opt-in role-tagged
//! body convention that keeps user and assistant text separable.
//!
//! Everything here is pure: no SQLite, no engine handle. Every count and cut is
//! in Unicode scalar values. The SDK bindings convert values only, so the
//! three SDKs return byte-identical output.

use std::fmt;

/// One role-tagged piece of an item. `role` is free text for
/// [`pack_context`]; [`encode_round_body`] accepts only the roles in
/// [`ROUND_BODY_ROLES`] plus `None`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPart {
    pub role: Option<String>,
    pub text: String,
}

/// One caller item. `group` and `position` drive group expansion;
/// `order_key` drives chronological order; `header` is rendered verbatim on
/// its own line above the item text and is not charged to the budget.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ContextItem {
    pub id: String,
    pub parts: Vec<ContextPart>,
    pub group: Option<String>,
    pub position: Option<u64>,
    pub order_key: Option<String>,
    pub header: Option<String>,
}

/// Output order of the rendered items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ContextOrder {
    /// Selection order: hits in input order, then expansion items.
    #[default]
    Rank,
    /// Sorted by `(order_key, group, position)`, missing values last for each
    /// key, selection index as the final tie-break.
    Chronological,
}

/// Budget and tier settings. [`Default`] is the shipped product default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPolicy {
    /// Total item-text characters; must be at least 1.
    pub budget_chars: usize,
    /// Per-item cap; must be at least 1.
    pub item_chars: usize,
    /// Items at selection index below this render every part in full.
    pub full_items: usize,
    /// Shared allowance for non-kept parts of a compact item.
    pub compact_chars: usize,
    /// Roles rendered in full in the compact tier (role `None` always is).
    pub keep_roles: Vec<String>,
    /// Append the `context` items of each hit group after the hits.
    pub expand_groups: bool,
    pub order: ContextOrder,
}

/// Product defaults: top 3 full, 300-character excerpt, keep user text.
pub const DEFAULT_BUDGET_CHARS: usize = 30_000;
pub const DEFAULT_ITEM_CHARS: usize = 4_000;
pub const DEFAULT_FULL_ITEMS: usize = 3;
pub const DEFAULT_COMPACT_CHARS: usize = 300;

impl Default for ContextPolicy {
    fn default() -> Self {
        Self {
            budget_chars: DEFAULT_BUDGET_CHARS,
            item_chars: DEFAULT_ITEM_CHARS,
            full_items: DEFAULT_FULL_ITEMS,
            compact_chars: DEFAULT_COMPACT_CHARS,
            keep_roles: vec!["user".to_string()],
            expand_groups: true,
            order: ContextOrder::Rank,
        }
    }
}

/// Whether a provenance entry came from `hits` or `context`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextSource {
    Hit,
    Context,
}

/// The rule an item was rendered under; `Omitted` means not rendered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextTier {
    Full,
    Compact,
    Omitted,
}

/// Rendered characters of one part out of its full length.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPartEntry {
    pub role: Option<String>,
    pub chars: usize,
    pub of_chars: usize,
}

/// Provenance of one input item. `chars` counts the item text including the
/// `"\n"` joins between parts; `truncated` is true when a rendered item shows
/// fewer characters of some part than it has, and false for omitted items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPackEntry {
    pub id: String,
    pub source: ContextSource,
    pub tier: ContextTier,
    pub chars: usize,
    pub truncated: bool,
    pub rendered_index: Option<usize>,
    pub parts: Vec<ContextPartEntry>,
}

/// Result of [`pack_context`]. `chars_used` is the sum of item `chars`;
/// `text_chars` is the scalar length of `text`, headers and separators
/// included. `items` has one entry per input item, hits then context, in
/// input order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPack {
    pub text: String,
    pub chars_used: usize,
    pub text_chars: usize,
    pub items: Vec<ContextPackEntry>,
}

/// Invalid input. The message names the offending field, for example
/// `policy.budget_chars` or `context[3].position`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPackError {
    message: String,
}

impl ContextPackError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    /// The human-readable message, naming the offending field.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ContextPackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ContextPackError {}

/// Marker that opens every part of a tagged round body (U+001E).
pub const ROUND_BODY_MARK: char = '\u{1e}';

/// Role codes that follow [`ROUND_BODY_MARK`]: role `None` is U+0010, then
/// `user`, `assistant`, `system`, `tool` are U+0011 to U+0014.
pub const ROUND_BODY_ROLES: [(Option<&str>, char); 5] = [
    (None, '\u{10}'),
    (Some("user"), '\u{11}'),
    (Some("assistant"), '\u{12}'),
    (Some("system"), '\u{13}'),
    (Some("tool"), '\u{14}'),
];

/// Render `hits` (and, with group expansion, `context`) into one string.
///
/// # Errors
/// [`ContextPackError`] for a zero `budget_chars` or `item_chars`, an empty or
/// repeated id, or a grouped `context` item without `position` while
/// `expand_groups` is on.
pub fn pack_context(
    hits: &[ContextItem],
    policy: &ContextPolicy,
    context: &[ContextItem],
) -> Result<ContextPack, ContextPackError> {
    let _ = (hits, policy, context);
    todo!("Slice 41 W4: pack_context")
}

/// Encode parts as a tagged round body.
///
/// # Errors
/// [`ContextPackError`] for an empty part list, a role outside
/// [`ROUND_BODY_ROLES`], or a part text containing U+001E or a role code.
pub fn encode_round_body(parts: &[ContextPart]) -> Result<String, ContextPackError> {
    let _ = parts;
    todo!("Slice 41 W4: encode_round_body")
}

/// Decode a body into parts. A body that does not start with U+001E is
/// untagged and decodes to one part with role `None`.
///
/// # Errors
/// [`ContextPackError`] for a body that starts with U+001E but does not decode.
pub fn decode_round_body(body: &str) -> Result<Vec<ContextPart>, ContextPackError> {
    let _ = body;
    todo!("Slice 41 W4: decode_round_body")
}

/// JSON form of [`pack_context`] for the language bindings. The request is
/// `{"hits": [...], "policy": {...}, "context": [...]}`; missing policy fields
/// take the defaults and unknown fields are refused.
///
/// # Errors
/// [`ContextPackError`] naming the field for malformed JSON, a wrong type, a
/// negative or non-integer count, an unknown field or `order`, and every
/// [`pack_context`] refusal.
pub fn pack_context_json(request: &str) -> Result<String, ContextPackError> {
    let _ = request;
    todo!("Slice 41 W4: pack_context_json")
}

/// JSON form of [`encode_round_body`]: `parts` is a JSON array of
/// `{"role": string|null, "text": string}`; returns the body string.
///
/// # Errors
/// As [`encode_round_body`], plus malformed JSON.
pub fn encode_round_body_json(parts: &str) -> Result<String, ContextPackError> {
    let _ = parts;
    todo!("Slice 41 W4: encode_round_body_json")
}

/// JSON form of [`decode_round_body`]: `body` is the body as a JSON string
/// literal (so a lone surrogate arrives as an escape and is refused); returns
/// the parts as a JSON array.
///
/// # Errors
/// As [`decode_round_body`], plus malformed JSON.
pub fn decode_round_body_json(body: &str) -> Result<String, ContextPackError> {
    let _ = body;
    todo!("Slice 41 W4: decode_round_body_json")
}
