//! Match-centred snippets (EP-17): the pure window selector.
//!
//! [`snippet_for`] turns a record body and its match spans into a snippet of
//! at most `max_bytes` UTF-8 bytes. It is pure (no SQL, I/O, clock, floating
//! point or locks), so the engine calls it after fetching spans and the MCP
//! adapter re-runs it under a smaller budget with the same result rules.
//! Specification: `dev/design/agent-access/match-centred-snippets.md` §§ 6,
//! 7.2.

use unicode_segmentation::UnicodeSegmentation;

use crate::EngineError;

/// Snippet settings for one search, and the inputs of [`snippet_for`].
///
/// Fields are public so callers can use struct-update syntax over
/// [`SnippetOptions::default`]. [`SnippetOptions::validate`] checks the
/// documented ranges; search entry points reject out-of-range values with
/// [`EngineError::InvalidArgument`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SnippetOptions {
    /// Output budget in UTF-8 bytes, ellipses included. `40..=4000`; default
    /// 300.
    pub max_bytes: u32,
    /// Most windows the snippet may show. `1..=3`; default 1.
    pub max_windows: u8,
    /// Bytes reserved for the start of the record, shown as its own window
    /// before the match windows. `0..=max_bytes / 2`; default 0 (no head).
    pub head_bytes: u32,
    /// Return highlight ranges into `text`. Default false.
    pub highlights: bool,
    /// Per-search cap on span point queries; hits past it get the record
    /// start. Not used by [`snippet_for`]. Default 20.
    pub max_point_queries: u8,
}

impl Default for SnippetOptions {
    fn default() -> Self {
        Self {
            max_bytes: 300,
            max_windows: 1,
            head_bytes: 0,
            highlights: false,
            max_point_queries: 20,
        }
    }
}

impl SnippetOptions {
    /// Smallest accepted `max_bytes`.
    pub const MIN_BYTES: u32 = 40;
    /// Largest accepted `max_bytes`.
    pub const MAX_BYTES: u32 = 4000;
    /// Largest accepted `max_windows`.
    pub const MAX_WINDOWS: u8 = 3;

    /// Checks every field against its documented range.
    ///
    /// # Errors
    ///
    /// [`EngineError::InvalidArgument`] naming the first field out of range.
    pub fn validate(&self) -> Result<(), EngineError> {
        let invalid = |msg: String| Err(EngineError::InvalidArgument { msg });
        if !(Self::MIN_BYTES..=Self::MAX_BYTES).contains(&self.max_bytes) {
            return invalid(format!(
                "snippet max_bytes must be in {}..={}, got {}",
                Self::MIN_BYTES,
                Self::MAX_BYTES,
                self.max_bytes
            ));
        }
        if !(1..=Self::MAX_WINDOWS).contains(&self.max_windows) {
            return invalid(format!(
                "snippet max_windows must be in 1..={}, got {}",
                Self::MAX_WINDOWS,
                self.max_windows
            ));
        }
        if self.head_bytes > self.max_bytes / 2 {
            return invalid(format!(
                "snippet head_bytes must be in 0..={} (max_bytes / 2), got {}",
                self.max_bytes / 2,
                self.head_bytes
            ));
        }
        Ok(())
    }
}

/// Where a snippet's text comes from.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum SnippetAnchor {
    /// The whole body fitted the budget.
    Whole,
    /// The window is centred on match spans.
    Match,
    /// The record start: no spans were available.
    Start,
}

impl SnippetAnchor {
    /// The wire word (`whole`, `match`, `start`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Whole => "whole",
            Self::Match => "match",
            Self::Start => "start",
        }
    }
}

/// One hit's snippet. All ranges are half-open UTF-8 byte ranges.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct SnippetV1 {
    /// The windows joined in body order with ellipses: `"… "` before a window
    /// that starts after byte 0, `" … "` between windows, `" …"` after a
    /// window that ends before the body does. At most `max_bytes` bytes.
    pub text: String,
    /// Where the text comes from.
    pub anchor: SnippetAnchor,
    /// The body ranges shown, sorted and disjoint. `text` is derived from
    /// them; they, not `text`, say where an ellipsis is structure.
    pub windows: Vec<(u32, u32)>,
    /// The match spans used, body-relative, sorted and disjoint, widened to
    /// grapheme boundaries. Empty when no spans were given.
    pub spans: Vec<(u32, u32)>,
    /// Ranges into `text` of the spans shown in full. Empty unless
    /// `SnippetOptions::highlights` is true.
    pub highlights: Vec<(u32, u32)>,
    /// True when any of the body is left out.
    pub cut: bool,
}

/// Selects the snippet for `body` from its match `spans` under `options`.
///
/// Spans are body byte ranges `[start, end)`. Empty spans, spans past the end
/// of the body and spans that do not fall on `char` boundaries are ignored;
/// the rest are sorted, widened to grapheme boundaries and merged where they
/// overlap. Options outside their documented ranges are clamped into them
/// ([`SnippetOptions::validate`] is the rejecting check).
///
/// The result is a pure function of the arguments and of the build's Unicode
/// tables; ties break toward the earlier byte offset.
#[must_use]
pub fn snippet_for(body: &str, spans: &[(u32, u32)], options: &SnippetOptions) -> SnippetV1 {
    let options = clamped(options);
    let graphemes = Boundaries::graphemes(body);
    let spans = normalised_spans(body, spans, &graphemes);
    let max_bytes = options.max_bytes as usize;
    if body.len() <= max_bytes {
        return whole(body, &spans, &options);
    }
    let words = Boundaries::words(body);
    let sentences = Boundaries::sentence_starts(body);
    let text = Text { body, graphemes: &graphemes, words: &words, sentences: &sentences };
    let ids = approximate_identities(body, &spans);
    let head = text.head(options.head_bytes as usize);
    let (windows, anchor) = match text.match_windows(&spans, &ids, head, &options) {
        Some(windows) => (windows, SnippetAnchor::Match),
        None => (vec![text.prefix(max_bytes - TRAILING.len())], SnippetAnchor::Start),
    };
    let (text_out, highlights) = assemble(body, &windows, &spans, options.highlights);
    debug_assert!(text_out.len() <= max_bytes);
    SnippetV1 {
        text: text_out,
        anchor,
        cut: windows != [(0, body.len())],
        windows: windows.iter().map(|&(s, e)| (to_u32(s), to_u32(e))).collect(),
        spans: to_ranges(&spans),
        highlights,
    }
}

const LEADING: &str = "\u{2026} ";
const BETWEEN: &str = " \u{2026} ";
const TRAILING: &str = " \u{2026}";

/// How far an edge may move to reach a word boundary before it falls back to
/// a grapheme boundary.
const WORD_REACH: usize = 32;

/// A body with its boundary tables.
struct Text<'a> {
    body: &'a str,
    graphemes: &'a Boundaries,
    words: &'a Boundaries,
    sentences: &'a Boundaries,
}

/// Rule 3, default: a span's identity is its text after Unicode lowercasing.
fn approximate_identities(body: &str, spans: &[Span]) -> Vec<u32> {
    let mut seen: Vec<String> = Vec::new();
    spans
        .iter()
        .map(|s| {
            let key = body[s.start..s.end].to_lowercase();
            let id = seen.iter().position(|k| *k == key).unwrap_or_else(|| {
                seen.push(key);
                seen.len() - 1
            });
            to_u32(id)
        })
        .collect()
}

/// A window chosen by rule 4: the spans of its pool from `spans[first]` to
/// `spans[last]` lie inside it.
#[derive(Clone, Copy, Debug)]
struct Choice {
    first: usize,
    last: usize,
    /// The rule 4.3 sentence start, when one lies within
    /// [`SENTENCE_REACH`] bytes before `spans[first]`.
    sentence: Option<usize>,
}

/// How far before the first span a sentence start earns the rule 4.3 bonus.
const SENTENCE_REACH: usize = 64;

/// Rule 4's lexicographic score: distinct identities, spans, sentence bonus.
type Score = (usize, usize, bool);

/// Fewest bytes rule 7 needs left before it adds another window.
const MIN_EXTRA_WINDOW: usize = 80;

impl Text<'_> {
    /// Rules 4–7: the head (if any) and the match windows, sorted. `None`
    /// when no window can hold a whole span.
    fn match_windows(
        &self,
        spans: &[Span],
        ids: &[u32],
        head: Option<(usize, usize)>,
        options: &SnippetOptions,
    ) -> Option<Vec<(usize, usize)>> {
        let max_bytes = options.max_bytes as usize;
        let max_windows = usize::from(options.max_windows);
        let all = (0, self.body.len());
        let full =
            max_bytes - TRAILING.len() - head.map_or(LEADING.len(), |(_, h)| h + BETWEEN.len());
        let pool: Vec<usize> = (0..spans.len()).collect();
        let (mut choice, _) = self.best_window(spans, ids, &pool, full, all)?;
        let mut width = full;
        let mut windows: Vec<(usize, usize)> = head.into_iter().collect();
        if max_windows > 1 {
            let first = self.place(spans, &choice, full, all);
            let with_first = sorted_with(&windows, first);
            if !self.uncovered(spans, ids, &with_first).is_empty() {
                // Leave room for the windows rule 7 will add.
                width = full / max_windows;
                choice = self.best_window(spans, ids, &pool, width, all)?.0;
            }
        }
        windows = sorted_with(&windows, self.place(spans, &choice, width, all));

        for left in (1..max_windows).rev() {
            let remaining = max_bytes.saturating_sub(assembled_len(self.body.len(), &windows));
            if remaining < MIN_EXTRA_WINDOW {
                break;
            }
            let eligible = self.uncovered(spans, ids, &windows);
            let width = (remaining - BETWEEN.len()) / left;
            let mut best: Option<(Choice, Score, (usize, usize))> = None;
            for gap in gaps(self.body.len(), &windows) {
                let pool: Vec<usize> = eligible
                    .iter()
                    .copied()
                    .filter(|&j| gap.0 <= spans[j].start && spans[j].end <= gap.1)
                    .collect();
                if let Some((c, score)) = self.best_window(spans, ids, &pool, width, gap) {
                    if best.is_none_or(|(_, b, _)| score > b) {
                        best = Some((c, score, gap));
                    }
                }
            }
            let Some((c, _, gap)) = best else { break };
            windows = sorted_with(&windows, self.place(spans, &c, width, gap));
        }
        Some(windows)
    }

    /// Rule 7's pool: spans outside every window whose identity no window
    /// shows.
    fn uncovered(&self, spans: &[Span], ids: &[u32], windows: &[(usize, usize)]) -> Vec<usize> {
        let inside = |s: &Span| windows.iter().any(|&(a, b)| a <= s.start && s.end <= b);
        let shown: Vec<u32> =
            (0..spans.len()).filter(|&j| inside(&spans[j])).map(|j| ids[j]).collect();
        (0..spans.len()).filter(|&j| !inside(&spans[j]) && !shown.contains(&ids[j])).collect()
    }

    /// Rule 4: for each span start in `pool` (indices into `spans`, sorted),
    /// scores the window of `width` bytes that begins there, within
    /// `region`, and returns the best. `None` when no window holds a whole
    /// span.
    fn best_window(
        &self,
        spans: &[Span],
        ids: &[u32],
        pool: &[usize],
        width: usize,
        region: (usize, usize),
    ) -> Option<(Choice, Score)> {
        let mut best: Option<(Choice, Score)> = None;
        for first in 0..pool.len() {
            let start = spans[pool[first]].start;
            let limit = (start + width).min(region.1);
            let mut last = None;
            let mut distinct: Vec<u32> = Vec::new();
            for (k, &j) in pool.iter().enumerate().skip(first) {
                if spans[j].end > limit {
                    break;
                }
                last = Some(k);
                if !distinct.contains(&ids[j]) {
                    distinct.push(ids[j]);
                }
            }
            let Some(last) = last else { continue };
            let sentence = Some(self.sentences.at_or_before(start))
                .filter(|&ss| ss >= region.0 && ss + SENTENCE_REACH >= start && ss <= start);
            let score = (distinct.len(), last - first + 1, sentence.is_some());
            // Strictly greater only: an equal score keeps the earlier start.
            if best.is_none_or(|(_, b)| score > b) {
                best = Some((Choice { first: pool[first], last: pool[last], sentence }, score));
            }
        }
        best
    }

    /// Rules 5 and 6: centres a window of `width` bytes on the chosen spans
    /// inside `region`, then snaps its edges.
    fn place(
        &self,
        spans: &[Span],
        choice: &Choice,
        width: usize,
        region: (usize, usize),
    ) -> (usize, usize) {
        let (lo, hi) = region;
        let (first_s, last_e) = (spans[choice.first].start, spans[choice.last].end);
        if let Some(ss) = choice.sentence.filter(|&ss| ss + width >= last_e) {
            let end = self.snap_end_back((ss + width).min(hi), last_e);
            return (ss, self.trim_end(last_e, end));
        }
        let slack = width - (last_e - first_s);
        let centred = first_s.saturating_sub(slack / 2).min(hi.saturating_sub(width)).max(lo);
        let start = self.snap_start(
            self.graphemes.at_or_after(centred).min(first_s),
            (lo, first_s),
            last_e,
            width,
        );
        let end = self.snap_end_back((start + width).min(hi), last_e);
        let start = self.trim_start(start, first_s);
        (start, self.trim_end(last_e, end))
    }

    /// Rule 6, start edge, for a window of `width` bytes that must keep
    /// `[first_s, last_e)`: the largest word boundary in
    /// `[target − WORD_REACH, target]`, not before `lo`, that still leaves
    /// `last_e` in budget; else the smallest word boundary in
    /// `[target, target + WORD_REACH]` not past `first_s`; else `target`
    /// itself (a grapheme boundary).
    fn snap_start(
        &self,
        target: usize,
        (lo, first_s): (usize, usize),
        last_e: usize,
        width: usize,
    ) -> usize {
        let back = self.words.at_or_before(target);
        if back >= lo && back + WORD_REACH >= target && back + width >= last_e {
            return back;
        }
        let forward = self.words.at_or_after(target);
        if forward <= first_s && forward <= target + WORD_REACH {
            return forward;
        }
        target
    }

    /// Drops whitespace clusters from the start of a window starting at
    /// `start`, never moving it past `ceiling`.
    fn trim_start(&self, mut start: usize, ceiling: usize) -> usize {
        while start < ceiling {
            let next = self.graphemes.at_or_after(start + 1);
            if next > ceiling || !self.body[start..next].chars().all(char::is_whitespace) {
                break;
            }
            start = next;
        }
        start
    }

    /// § 6.6: the record head, `[0, h)`, cut at the first newline or the
    /// last word boundary at or before `head_bytes`, whichever comes first.
    /// `None` when `head_bytes` is 0 or the cut leaves nothing.
    fn head(&self, head_bytes: usize) -> Option<(usize, usize)> {
        if head_bytes == 0 {
            return None;
        }
        let word = self.words.at_or_before(head_bytes.min(self.body.len()));
        // `\r` first: a CRLF pair is one grapheme cluster.
        let end = self.body[..word].find(['\n', '\r']).unwrap_or(word);
        let end = self.trim_end(0, end);
        (end > 0).then_some((0, end))
    }

    /// Rule 2: the prefix within `budget` bytes, on a word boundary.
    fn prefix(&self, budget: usize) -> (usize, usize) {
        let end = self.snap_end_back(budget.min(self.body.len()), 0);
        // Keep at least the first cluster, even when it is whitespace.
        let floor = self.graphemes.at_or_after(1).min(end);
        (0, self.trim_end(floor, end))
    }

    /// The largest word boundary in `[target − WORD_REACH, target]` and not
    /// before `floor`; else the largest grapheme boundary in `[floor, target]`.
    fn snap_end_back(&self, target: usize, floor: usize) -> usize {
        let word = self.words.at_or_before(target);
        if word >= floor && word + WORD_REACH >= target {
            return word;
        }
        self.graphemes.at_or_before(target).max(floor)
    }

    /// Drops whitespace clusters from the end of a window ending at `end`,
    /// never moving it before `floor`.
    fn trim_end(&self, floor: usize, mut end: usize) -> usize {
        while end > floor {
            let prev = self.graphemes.at_or_before(end - 1);
            if prev < floor || !self.body[prev..end].chars().all(char::is_whitespace) {
                break;
            }
            end = prev;
        }
        end
    }
}

/// `windows` (sorted, disjoint) with `window` inserted in order.
fn sorted_with(windows: &[(usize, usize)], window: (usize, usize)) -> Vec<(usize, usize)> {
    let mut out = windows.to_vec();
    let at = out.partition_point(|w| *w < window);
    out.insert(at, window);
    out
}

/// The body ranges between and around `windows` (sorted, disjoint).
fn gaps(len: usize, windows: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut out = Vec::with_capacity(windows.len() + 1);
    let mut lo = 0;
    for &(a, b) in windows {
        if a > lo {
            out.push((lo, a));
        }
        lo = lo.max(b);
    }
    if lo < len {
        out.push((lo, len));
    }
    out
}

/// The byte length [`assemble`] gives `windows`.
fn assembled_len(len: usize, windows: &[(usize, usize)]) -> usize {
    let content: usize = windows.iter().map(|&(a, b)| b - a).sum();
    let leading = windows.first().map_or(0, |&(a, _)| if a > 0 { LEADING.len() } else { 0 });
    let trailing = windows.last().map_or(0, |&(_, b)| if b < len { TRAILING.len() } else { 0 });
    content + leading + trailing + BETWEEN.len() * windows.len().saturating_sub(1)
}

/// Rule 9: joins the windows in body order with ellipses. With `highlights`,
/// also returns the ranges into the text of the spans shown in full.
fn assemble(
    body: &str,
    windows: &[(usize, usize)],
    spans: &[Span],
    highlights: bool,
) -> (String, Vec<(u32, u32)>) {
    let mut text = String::new();
    let mut lit = Vec::new();
    for (i, &(start, end)) in windows.iter().enumerate() {
        if i == 0 {
            if start > 0 {
                text.push_str(LEADING);
            }
        } else {
            text.push_str(BETWEEN);
        }
        let base = text.len();
        text.push_str(&body[start..end]);
        if highlights {
            lit.extend(
                spans
                    .iter()
                    .filter(|s| start <= s.start && s.end <= end)
                    .map(|s| (to_u32(base + s.start - start), to_u32(base + s.end - start))),
            );
        }
    }
    if windows.last().is_some_and(|&(_, end)| end < body.len()) {
        text.push_str(TRAILING);
    }
    (text, lit)
}

/// A match span in body bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Span {
    start: usize,
    end: usize,
}

fn clamped(options: &SnippetOptions) -> SnippetOptions {
    let max_bytes = options.max_bytes.clamp(SnippetOptions::MIN_BYTES, SnippetOptions::MAX_BYTES);
    SnippetOptions {
        max_bytes,
        max_windows: options.max_windows.clamp(1, SnippetOptions::MAX_WINDOWS),
        head_bytes: options.head_bytes.min(max_bytes / 2),
        ..*options
    }
}

/// Sorted byte offsets of one kind of boundary, including 0 and `body.len()`.
struct Boundaries(Vec<usize>);

impl Boundaries {
    /// UAX #29 sentence starts (0 included for a non-empty body).
    fn sentence_starts(body: &str) -> Self {
        Self(body.split_sentence_bound_indices().map(|(i, _)| i).collect())
    }

    fn words(body: &str) -> Self {
        let mut v: Vec<usize> = body.split_word_bound_indices().map(|(i, _)| i).collect();
        v.push(body.len());
        Self(v)
    }

    fn graphemes(body: &str) -> Self {
        let mut v: Vec<usize> = body.grapheme_indices(true).map(|(i, _)| i).collect();
        v.push(body.len());
        Self(v)
    }

    /// The largest boundary at or before `offset`.
    fn at_or_before(&self, offset: usize) -> usize {
        match self.0.binary_search(&offset) {
            Ok(i) => self.0[i],
            Err(i) => self.0[i.saturating_sub(1)],
        }
    }

    /// The smallest boundary at or after `offset`.
    fn at_or_after(&self, offset: usize) -> usize {
        match self.0.binary_search(&offset) {
            Ok(i) => self.0[i],
            Err(i) => self.0.get(i).copied().unwrap_or(offset),
        }
    }
}

fn normalised_spans(body: &str, spans: &[(u32, u32)], graphemes: &Boundaries) -> Vec<Span> {
    let mut out: Vec<Span> = spans
        .iter()
        .map(|&(s, e)| Span { start: s as usize, end: e as usize })
        .filter(|s| {
            s.start < s.end
                && s.end <= body.len()
                && body.is_char_boundary(s.start)
                && body.is_char_boundary(s.end)
        })
        .map(|s| Span { start: graphemes.at_or_before(s.start), end: graphemes.at_or_after(s.end) })
        .collect();
    out.sort_by_key(|s| (s.start, s.end));
    let mut merged: Vec<Span> = Vec::with_capacity(out.len());
    for span in out {
        match merged.last_mut() {
            Some(last) if span.start < last.end => last.end = last.end.max(span.end),
            _ => merged.push(span),
        }
    }
    merged
}

fn to_u32(offset: usize) -> u32 {
    u32::try_from(offset).unwrap_or(u32::MAX)
}

fn to_ranges(spans: &[Span]) -> Vec<(u32, u32)> {
    spans.iter().map(|s| (to_u32(s.start), to_u32(s.end))).collect()
}

fn whole(body: &str, spans: &[Span], options: &SnippetOptions) -> SnippetV1 {
    let ranges = to_ranges(spans);
    SnippetV1 {
        text: body.to_owned(),
        anchor: SnippetAnchor::Whole,
        windows: if body.is_empty() { Vec::new() } else { vec![(0, to_u32(body.len()))] },
        highlights: if options.highlights { ranges.clone() } else { Vec::new() },
        spans: ranges,
        cut: false,
    }
}
