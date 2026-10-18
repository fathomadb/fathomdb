//! Match-centred snippets (EP-17): the pure window selector.
//!
//! [`snippet_for`] turns a record body and its match spans into a snippet of
//! at most `max_bytes` UTF-8 bytes. It is pure (no SQL, I/O, clock, floating
//! point or locks), so the engine calls it after fetching spans and the MCP
//! adapter re-runs it under a smaller budget with the same result rules.
//! Specification: `dev/design/agent-access/match-centred-snippets.md` §§ 6,
//! 7.2.

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
    let _ = (body, spans, options);
    SnippetV1 {
        text: String::new(),
        anchor: SnippetAnchor::Start,
        windows: Vec::new(),
        spans: Vec::new(),
        highlights: Vec::new(),
        cut: false,
    }
}
