//! EP-17 match spans from the live FTS index, for the final hits of one
//! search (`dev/design/agent-access/match-centred-snippets.md` §§ 6.1, 7.2).
//!
//! Runs on the search's reader transaction after truncation and the identity
//! pass, with read-only point queries only: one `highlight()` per hit,
//! constrained to the hit's FTS rowid.

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension};

use crate::search_types::SearchHit;
use crate::snippet::{snippet_for, SnippetOptions, SnippetV1};

/// The FTS table a text hit came from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FtsTable {
    Nodes,
}

impl FtsTable {
    fn highlight_sql(self) -> &'static str {
        match self {
            Self::Nodes => {
                "SELECT highlight(search_index, 0, ?1, ?2) FROM search_index \
                 WHERE search_index MATCH ?3 AND rowid = ?4"
            }
        }
    }
}

/// The search-local map from a text hit's `write_cursor` to its FTS row.
pub(crate) type FtsRows = HashMap<u64, (FtsTable, i64)>;

/// Marker pairs in preference order (§ 6.1): private use, then the last
/// private-use pair, then noncharacters. Private-use glyphs occur in real
/// text (Nerd Fonts, Powerline), hence a list rather than one pair.
const MARKERS: [(char, char); 3] =
    [('\u{E000}', '\u{E001}'), ('\u{F8FE}', '\u{F8FF}'), ('\u{FDD0}', '\u{FDD1}')];

/// One snippet per hit, in order.
pub(crate) fn snippets_for_hits(
    tx: &Connection,
    hits: &[SearchHit],
    rows: &FtsRows,
    match_expression: &str,
    options: &SnippetOptions,
) -> rusqlite::Result<Vec<SnippetV1>> {
    // `max_point_queries` caps the hits that get a point query, in rank order;
    // a hit cap, not a time cap, keeps the output deterministic (§ 6.5).
    let mut budget = usize::from(options.max_point_queries);
    hits.iter()
        .map(|hit| {
            // Rule 0: spans are needed only to cut the body or to highlight.
            let needs_spans = hit.body.len() > options.max_bytes as usize || options.highlights;
            let row = rows.get(&hit.write_cursor).filter(|_| needs_spans && budget > 0);
            let spans = match row {
                Some(&(table, rowid)) => {
                    budget -= 1;
                    match_spans(tx, table, rowid, match_expression, &hit.body)?.unwrap_or_default()
                }
                None => Vec::new(),
            };
            Ok(snippet_for(&hit.body, &spans, options))
        })
        .collect()
}

/// The spans `match_expression` marks in the row's body, or `None` when the
/// marked text does not strip back to `body`.
fn match_spans(
    tx: &Connection,
    table: FtsTable,
    rowid: i64,
    match_expression: &str,
    body: &str,
) -> rusqlite::Result<Option<Vec<(u32, u32)>>> {
    // The first pair whose code points are both absent from the body; none
    // free means no highlighting, so the hit gets the record start.
    let Some((open, close)) =
        MARKERS.into_iter().find(|&(open, close)| !body.contains(open) && !body.contains(close))
    else {
        return Ok(None);
    };
    let marked: Option<String> = tx
        .prepare_cached(table.highlight_sql())?
        .query_row(
            rusqlite::params![open.to_string(), close.to_string(), match_expression, rowid],
            |row| row.get(0),
        )
        .optional()?;
    Ok(marked.and_then(|marked| parse_marked(body, &marked, open, close)))
}

/// Parses `marked` once, left to right: each marker's offset minus the bytes
/// of the markers before it is a span boundary in the original body.
fn parse_marked(body: &str, marked: &str, open: char, close: char) -> Option<Vec<(u32, u32)>> {
    let mut stripped = String::with_capacity(body.len());
    let mut spans = Vec::new();
    let mut start = None;
    for c in marked.chars() {
        if c == open {
            if start.replace(stripped.len()).is_some() {
                return None;
            }
        } else if c == close {
            let s = start.take()?;
            spans.push((u32::try_from(s).ok()?, u32::try_from(stripped.len()).ok()?));
        } else {
            stripped.push(c);
        }
    }
    (start.is_none() && stripped == body).then_some(spans)
}
