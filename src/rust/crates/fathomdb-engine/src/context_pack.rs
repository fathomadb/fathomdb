//! Context pack: render ranked caller items into prompt context within a
//! character budget, with full and compact tiers, and an opt-in role-tagged
//! body convention that keeps user and assistant text separable.
//!
//! Everything here is pure: no SQLite, no engine handle. Every count and cut is
//! in Unicode scalar values. The SDK bindings convert values only, so the
//! three SDKs return byte-identical output.

use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt;

use serde_json::{json, Map, Value};

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
    /// Item ids that render in the full tier wherever they are selected,
    /// without using a `full_items` slot. Every id must match an input item;
    /// an unselected match has no effect. Default empty.
    pub full_ids: Vec<String>,
    /// Append the `context` items of each hit group after the hits.
    pub expand_groups: bool,
    /// Output order of the rendered items; tiers and cuts are decided first.
    pub order: ContextOrder,
}

/// Default [`ContextPolicy::budget_chars`]: total item-text characters.
pub const DEFAULT_BUDGET_CHARS: usize = 30_000;
/// Default [`ContextPolicy::item_chars`]: per-item cap.
pub const DEFAULT_ITEM_CHARS: usize = 4_000;
/// Default [`ContextPolicy::full_items`]: the top 3 selected items render in full.
pub const DEFAULT_FULL_ITEMS: usize = 3;
/// Default [`ContextPolicy::compact_chars`]: 300-character shared excerpt.
pub const DEFAULT_COMPACT_CHARS: usize = 300;

impl Default for ContextPolicy {
    fn default() -> Self {
        Self {
            budget_chars: DEFAULT_BUDGET_CHARS,
            item_chars: DEFAULT_ITEM_CHARS,
            full_items: DEFAULT_FULL_ITEMS,
            compact_chars: DEFAULT_COMPACT_CHARS,
            keep_roles: vec!["user".to_string()],
            full_ids: Vec::new(),
            expand_groups: true,
            order: ContextOrder::Rank,
        }
    }
}

/// Whether a provenance entry came from `hits` or `context`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextSource {
    /// The item came from `hits`.
    Hit,
    /// The item came from `context`.
    Context,
}

/// The rule an item was rendered under; `Omitted` means not rendered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextTier {
    /// Every part rendered (selection index below `full_items`, or forced by
    /// `full_ids`).
    Full,
    /// Kept-role parts whole, the rest cut from the shared allowance.
    Compact,
    /// Not rendered: not reached, not selected, or whitespace-only.
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
    /// True when the id is in `full_ids` and the item rendered in the full
    /// tier only because of it; false when its selection index already made
    /// it full, and false for an item that was not rendered.
    pub forced: bool,
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
/// Selection is the hits in input order, then, when `expand_groups` is on, for
/// each hit group in order of its first hit the `context` items of that group
/// ordered by `position` (input order breaks ties). Selection index `i <
/// full_items`, and any selected item whose id is in `full_ids`, renders every
/// part; forced items take no `full_items` slot, get no reserved budget and
/// are never moved. Later items keep role-`None` and
/// `keep_roles` parts whole and cut the other parts, in order, from one shared
/// `compact_chars` allowance, dropping any that render nothing. Item text is
/// the parts joined by `"\n"`, cut to `item_chars` and then to the budget left;
/// a whitespace-only item is omitted but keeps its selection slot. Items render
/// as `header + "\n" + text` (or `text`) joined by `"\n\n"`.
///
/// # Errors
/// [`ContextPackError`] for a zero `budget_chars` or `item_chars`, an empty or
/// repeated id, an empty, repeated or unknown `full_ids` entry (checked after
/// the item ids), or a grouped `context` item without `position` while
/// `expand_groups` is on.
pub fn pack_context(
    hits: &[ContextItem],
    policy: &ContextPolicy,
    context: &[ContextItem],
) -> Result<ContextPack, ContextPackError> {
    validate(hits, policy, context)?;
    let items: Vec<(&ContextItem, ContextSource)> = hits
        .iter()
        .map(|item| (item, ContextSource::Hit))
        .chain(context.iter().map(|item| (item, ContextSource::Context)))
        .collect();
    let mut entries: Vec<ContextPackEntry> = items
        .iter()
        .map(|(item, source)| ContextPackEntry {
            id: item.id.clone(),
            source: *source,
            tier: ContextTier::Omitted,
            chars: 0,
            truncated: false,
            forced: false,
            rendered_index: None,
            parts: item
                .parts
                .iter()
                .map(|part| ContextPartEntry {
                    role: part.role.clone(),
                    chars: 0,
                    of_chars: part.text.chars().count(),
                })
                .collect(),
        })
        .collect();

    // Indices into `items` (hits first, then context at `hits.len() + j`).
    let mut selection: Vec<usize> = (0..hits.len()).collect();
    if policy.expand_groups {
        let mut groups: Vec<&str> = Vec::new();
        for hit in hits {
            if let Some(group) = hit.group.as_deref() {
                if !groups.contains(&group) {
                    groups.push(group);
                }
            }
        }
        for group in groups {
            let mut members: Vec<usize> = context
                .iter()
                .enumerate()
                .filter(|(_, item)| item.group.as_deref() == Some(group))
                .map(|(j, _)| j)
                .collect();
            // Stable sort: equal positions keep input order.
            members.sort_by_key(|&j| context[j].position);
            selection.extend(members.into_iter().map(|j| hits.len() + j));
        }
    }

    let forced_ids: HashSet<&str> = policy.full_ids.iter().map(String::as_str).collect();
    let mut remaining = policy.budget_chars;
    let mut rendered: Vec<(usize, usize, String)> = Vec::new();
    for (index, &global) in selection.iter().enumerate() {
        if remaining == 0 {
            break;
        }
        let item = items[global].0;
        let forced = index >= policy.full_items && forced_ids.contains(item.id.as_str());
        let full = index < policy.full_items || forced;
        let mut allowance = policy.compact_chars;
        let mut kept: Vec<(usize, &str)> = Vec::new();
        for (k, part) in item.parts.iter().enumerate() {
            let keep = full
                || part
                    .role
                    .as_deref()
                    .is_none_or(|role| policy.keep_roles.iter().any(|kept| kept == role));
            if keep {
                kept.push((k, part.text.as_str()));
            } else {
                let take = part.text.chars().count().min(allowance);
                allowance -= take;
                if take > 0 {
                    kept.push((k, prefix(&part.text, take)));
                }
            }
        }
        let text = kept.iter().map(|(_, text)| *text).collect::<Vec<_>>().join("\n");
        if text.chars().all(char::is_whitespace) {
            continue;
        }
        let chars = text.chars().count().min(policy.item_chars).min(remaining);
        let entry = &mut entries[global];
        let mut left = chars;
        for (j, (k, part_text)) in kept.iter().enumerate() {
            if j > 0 {
                if left == 0 {
                    break;
                }
                left -= 1;
            }
            let shown = part_text.chars().count().min(left);
            entry.parts[*k].chars = shown;
            left -= shown;
        }
        entry.tier = if full { ContextTier::Full } else { ContextTier::Compact };
        entry.forced = forced;
        entry.chars = chars;
        entry.truncated = entry.parts.iter().any(|part| part.chars < part.of_chars);
        remaining -= chars;
        rendered.push((index, global, prefix(&text, chars).to_string()));
    }

    if policy.order == ContextOrder::Chronological {
        rendered.sort_by(|a, b| {
            let (x, y) = (items[a.1].0, items[b.1].0);
            none_last(&x.order_key, &y.order_key)
                .then_with(|| none_last(&x.group, &y.group))
                .then_with(|| none_last(&x.position, &y.position))
                .then_with(|| a.0.cmp(&b.0))
        });
    }

    let mut blocks: Vec<String> = Vec::with_capacity(rendered.len());
    for (position, (_, global, text)) in rendered.into_iter().enumerate() {
        entries[global].rendered_index = Some(position);
        blocks.push(match &items[global].0.header {
            Some(header) => format!("{header}\n{text}"),
            None => text,
        });
    }
    let text = blocks.join("\n\n");
    Ok(ContextPack {
        chars_used: entries.iter().map(|entry| entry.chars).sum(),
        text_chars: text.chars().count(),
        text,
        items: entries,
    })
}

fn validate(
    hits: &[ContextItem],
    policy: &ContextPolicy,
    context: &[ContextItem],
) -> Result<(), ContextPackError> {
    if policy.budget_chars == 0 {
        return Err(ContextPackError::new("policy.budget_chars must be at least 1"));
    }
    if policy.item_chars == 0 {
        return Err(ContextPackError::new("policy.item_chars must be at least 1"));
    }
    let mut seen: HashSet<&str> = HashSet::new();
    for (name, list) in [("hits", hits), ("context", context)] {
        for (i, item) in list.iter().enumerate() {
            if item.id.is_empty() {
                return Err(ContextPackError::new(format!("{name}[{i}].id must not be empty")));
            }
            if !seen.insert(item.id.as_str()) {
                return Err(ContextPackError::new(format!(
                    "{name}[{i}].id {:?} repeats an earlier id",
                    item.id
                )));
            }
        }
    }
    if policy.expand_groups {
        for (i, item) in context.iter().enumerate() {
            if item.group.is_some() && item.position.is_none() {
                return Err(ContextPackError::new(format!(
                    "context[{i}].position is required for a grouped context item while \
                     policy.expand_groups is on"
                )));
            }
        }
    }
    let mut forced: HashSet<&str> = HashSet::new();
    for (i, id) in policy.full_ids.iter().enumerate() {
        if id.is_empty() {
            return Err(ContextPackError::new(format!("policy.full_ids[{i}] must not be empty")));
        }
        if !forced.insert(id.as_str()) {
            return Err(ContextPackError::new(format!(
                "policy.full_ids[{i}] {id:?} repeats an earlier id"
            )));
        }
        if !seen.contains(id.as_str()) {
            return Err(ContextPackError::new(format!(
                "policy.full_ids[{i}] {id:?} matches no item in hits or context"
            )));
        }
    }
    Ok(())
}

fn none_last<T: Ord>(a: &Option<T>, b: &Option<T>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// The first `n` Unicode scalar values of `text`.
fn prefix(text: &str, n: usize) -> &str {
    match text.char_indices().nth(n) {
        Some((byte, _)) => &text[..byte],
        None => text,
    }
}

fn role_code(role: Option<&str>) -> Option<char> {
    ROUND_BODY_ROLES.iter().find(|(name, _)| *name == role).map(|(_, code)| *code)
}

fn role_for_code(code: char) -> Option<Option<&'static str>> {
    ROUND_BODY_ROLES.iter().find(|(_, c)| *c == code).map(|(name, _)| *name)
}

fn is_marker_char(c: char) -> bool {
    c == ROUND_BODY_MARK || role_for_code(c).is_some()
}

/// Encode parts as a tagged round body: each part is U+001E, its role code and
/// its text; parts are joined by `"\n"`. Removing every U+001E and the code
/// after it gives the `"\n"`-joined texts.
///
/// # Errors
/// [`ContextPackError`] for an empty part list, a role outside
/// [`ROUND_BODY_ROLES`], or a part text containing U+001E or a role code.
pub fn encode_round_body(parts: &[ContextPart]) -> Result<String, ContextPackError> {
    if parts.is_empty() {
        return Err(ContextPackError::new("parts must not be empty"));
    }
    let mut encoded: Vec<String> = Vec::with_capacity(parts.len());
    for (i, part) in parts.iter().enumerate() {
        let code = role_code(part.role.as_deref()).ok_or_else(|| {
            ContextPackError::new(format!(
                "parts[{i}].role {:?} is not user, assistant, system, tool or null",
                part.role
            ))
        })?;
        if part.text.chars().any(is_marker_char) {
            return Err(ContextPackError::new(format!(
                "parts[{i}].text contains U+001E or a role code (U+0010..U+0014)"
            )));
        }
        encoded.push(format!("{ROUND_BODY_MARK}{code}{}", part.text));
    }
    Ok(encoded.join("\n"))
}

/// Decode a body into parts. A body that does not start with U+001E is
/// untagged and decodes to one part with role `None`.
///
/// # Errors
/// [`ContextPackError`] for a body that starts with U+001E but does not decode:
/// a missing or unknown role code, or a stray U+001E or role code in a part.
pub fn decode_round_body(body: &str) -> Result<Vec<ContextPart>, ContextPackError> {
    let Some(rest) = body.strip_prefix(ROUND_BODY_MARK) else {
        return Ok(vec![ContextPart { role: None, text: body.to_string() }]);
    };
    let separator = format!("\n{ROUND_BODY_MARK}");
    let mut parts = Vec::new();
    for (k, segment) in rest.split(separator.as_str()).enumerate() {
        let mut chars = segment.chars();
        let code = chars.next().ok_or_else(|| {
            ContextPackError::new(format!("body: tagged part {k} has no role code"))
        })?;
        let role = role_for_code(code).ok_or_else(|| {
            ContextPackError::new(format!(
                "body: tagged part {k} has unknown role code U+{:04X}",
                u32::from(code)
            ))
        })?;
        let text = chars.as_str();
        if text.chars().any(is_marker_char) {
            return Err(ContextPackError::new(format!(
                "body: tagged part {k} contains a stray U+001E or role code"
            )));
        }
        parts.push(ContextPart { role: role.map(str::to_string), text: text.to_string() });
    }
    Ok(parts)
}

// --- JSON wire for the bindings ------------------------------------------------

fn parse_json(input: &str, what: &str) -> Result<Value, ContextPackError> {
    serde_json::from_str(input)
        .map_err(|error| ContextPackError::new(format!("{what} is not valid JSON: {error}")))
}

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a Map<String, Value>, ContextPackError> {
    value.as_object().ok_or_else(|| ContextPackError::new(format!("{path} must be a JSON object")))
}

fn known_fields(
    map: &Map<String, Value>,
    path: &str,
    known: &[&str],
) -> Result<(), ContextPackError> {
    match map.keys().find(|key| !known.contains(&key.as_str())) {
        Some(key) => {
            let field = if path.is_empty() { key.clone() } else { format!("{path}.{key}") };
            Err(ContextPackError::new(format!("{field} is not a known field")))
        }
        None => Ok(()),
    }
}

fn count(value: &Value, path: &str) -> Result<u64, ContextPackError> {
    value.as_u64().ok_or_else(|| {
        ContextPackError::new(format!(
            "{path} must be a non-negative integer (at most {})",
            u64::MAX
        ))
    })
}

fn usize_count(value: &Value, path: &str) -> Result<usize, ContextPackError> {
    usize::try_from(count(value, path)?)
        .map_err(|_| ContextPackError::new(format!("{path} is too large")))
}

fn string(value: &Value, path: &str) -> Result<String, ContextPackError> {
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| ContextPackError::new(format!("{path} must be a string")))
}

fn optional_string(value: Option<&Value>, path: &str) -> Result<Option<String>, ContextPackError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(ContextPackError::new(format!("{path} must be a string or null"))),
    }
}

fn array<'a>(value: &'a Value, path: &str) -> Result<&'a Vec<Value>, ContextPackError> {
    value.as_array().ok_or_else(|| ContextPackError::new(format!("{path} must be a JSON array")))
}

fn parse_part(value: &Value, path: &str) -> Result<ContextPart, ContextPackError> {
    let map = object(value, path)?;
    known_fields(map, path, &["role", "text"])?;
    let text =
        map.get("text").ok_or_else(|| ContextPackError::new(format!("{path}.text is required")))?;
    Ok(ContextPart {
        role: optional_string(map.get("role"), &format!("{path}.role"))?,
        text: string(text, &format!("{path}.text"))?,
    })
}

fn parse_parts(value: &Value, path: &str) -> Result<Vec<ContextPart>, ContextPackError> {
    array(value, path)?
        .iter()
        .enumerate()
        .map(|(i, part)| parse_part(part, &format!("{path}[{i}]")))
        .collect()
}

fn parse_item(value: &Value, path: &str) -> Result<ContextItem, ContextPackError> {
    let map = object(value, path)?;
    known_fields(map, path, &["id", "parts", "group", "position", "order_key", "header"])?;
    let id =
        map.get("id").ok_or_else(|| ContextPackError::new(format!("{path}.id is required")))?;
    let parts = map
        .get("parts")
        .ok_or_else(|| ContextPackError::new(format!("{path}.parts is required")))?;
    let position = match map.get("position") {
        None | Some(Value::Null) => None,
        Some(value) => Some(count(value, &format!("{path}.position"))?),
    };
    Ok(ContextItem {
        id: string(id, &format!("{path}.id"))?,
        parts: parse_parts(parts, &format!("{path}.parts"))?,
        group: optional_string(map.get("group"), &format!("{path}.group"))?,
        position,
        order_key: optional_string(map.get("order_key"), &format!("{path}.order_key"))?,
        header: optional_string(map.get("header"), &format!("{path}.header"))?,
    })
}

fn parse_items(value: Option<&Value>, path: &str) -> Result<Vec<ContextItem>, ContextPackError> {
    match value {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(value) => array(value, path)?
            .iter()
            .enumerate()
            .map(|(i, item)| parse_item(item, &format!("{path}[{i}]")))
            .collect(),
    }
}

fn parse_policy(value: Option<&Value>) -> Result<ContextPolicy, ContextPackError> {
    let mut policy = ContextPolicy::default();
    let map = match value {
        None | Some(Value::Null) => return Ok(policy),
        Some(value) => object(value, "policy")?,
    };
    for (key, value) in map {
        let path = format!("policy.{key}");
        match key.as_str() {
            "budget_chars" => policy.budget_chars = usize_count(value, &path)?,
            "item_chars" => policy.item_chars = usize_count(value, &path)?,
            "full_items" => policy.full_items = usize_count(value, &path)?,
            "compact_chars" => policy.compact_chars = usize_count(value, &path)?,
            "full_ids" => {
                policy.full_ids = array(value, &path)?
                    .iter()
                    .enumerate()
                    .map(|(i, id)| string(id, &format!("{path}[{i}]")))
                    .collect::<Result<_, _>>()?;
            }
            "keep_roles" => {
                policy.keep_roles = array(value, &path)?
                    .iter()
                    .enumerate()
                    .map(|(i, role)| string(role, &format!("{path}[{i}]")))
                    .collect::<Result<_, _>>()?;
            }
            "expand_groups" => {
                policy.expand_groups = value
                    .as_bool()
                    .ok_or_else(|| ContextPackError::new(format!("{path} must be a boolean")))?;
            }
            "order" => {
                policy.order = match value.as_str() {
                    Some("rank") => ContextOrder::Rank,
                    Some("chronological") => ContextOrder::Chronological,
                    _ => {
                        return Err(ContextPackError::new(format!(
                            "{path} must be \"rank\" or \"chronological\""
                        )))
                    }
                };
            }
            _ => return Err(ContextPackError::new(format!("{path} is not a known field"))),
        }
    }
    Ok(policy)
}

fn source_name(source: ContextSource) -> &'static str {
    match source {
        ContextSource::Hit => "hit",
        ContextSource::Context => "context",
    }
}

fn tier_name(tier: ContextTier) -> &'static str {
    match tier {
        ContextTier::Full => "full",
        ContextTier::Compact => "compact",
        ContextTier::Omitted => "omitted",
    }
}

fn pack_value(pack: &ContextPack) -> Value {
    let items: Vec<Value> = pack
        .items
        .iter()
        .map(|entry| {
            let parts: Vec<Value> = entry
                .parts
                .iter()
                .map(|part| json!({"role": part.role, "chars": part.chars, "of_chars": part.of_chars}))
                .collect();
            json!({
                "id": entry.id,
                "source": source_name(entry.source),
                "tier": tier_name(entry.tier),
                "chars": entry.chars,
                "truncated": entry.truncated,
                "forced": entry.forced,
                "rendered_index": entry.rendered_index,
                "parts": parts,
            })
        })
        .collect();
    json!({
        "text": pack.text,
        "chars_used": pack.chars_used,
        "text_chars": pack.text_chars,
        "items": items,
    })
}

/// JSON form of [`pack_context`] for the language bindings. The request is
/// `{"hits": [...], "policy": {...}, "context": [...]}`; a missing `policy`
/// or policy field takes the default, a missing list is empty, and unknown
/// fields are refused. The response mirrors [`ContextPack`] in snake_case.
///
/// # Errors
/// [`ContextPackError`] naming the field for malformed JSON, a wrong type, a
/// negative or non-integer count, an unknown field or `order`, and every
/// [`pack_context`] refusal.
pub fn pack_context_json(request: &str) -> Result<String, ContextPackError> {
    let value = parse_json(request, "request")?;
    let map = object(&value, "request")?;
    known_fields(map, "", &["hits", "policy", "context"])?;
    let hits = parse_items(map.get("hits"), "hits")?;
    let policy = parse_policy(map.get("policy"))?;
    let context = parse_items(map.get("context"), "context")?;
    let pack = pack_context(&hits, &policy, &context)?;
    Ok(pack_value(&pack).to_string())
}

/// JSON form of [`encode_round_body`]: `parts` is a JSON array of
/// `{"role": string|null, "text": string}`; returns the body string.
///
/// # Errors
/// As [`encode_round_body`], plus malformed JSON.
pub fn encode_round_body_json(parts: &str) -> Result<String, ContextPackError> {
    let value = parse_json(parts, "parts")?;
    encode_round_body(&parse_parts(&value, "parts")?)
}

/// JSON form of [`decode_round_body`]: `body` is the body as a JSON string
/// literal (so a lone surrogate arrives as an escape and is refused); returns
/// the parts as a JSON array.
///
/// # Errors
/// As [`decode_round_body`], plus malformed JSON.
pub fn decode_round_body_json(body: &str) -> Result<String, ContextPackError> {
    let value = parse_json(body, "body")?;
    let body = value.as_str().ok_or_else(|| ContextPackError::new("body must be a JSON string"))?;
    let parts: Vec<Value> = decode_round_body(body)?
        .into_iter()
        .map(|part| json!({"role": part.role, "text": part.text}))
        .collect();
    Ok(Value::Array(parts).to_string())
}
