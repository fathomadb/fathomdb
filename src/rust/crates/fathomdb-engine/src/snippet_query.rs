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

const OPEN: char = '\u{E000}';
const CLOSE: char = '\u{E001}';

/// One snippet per hit, in order.
pub(crate) fn snippets_for_hits(
    tx: &Connection,
    hits: &[SearchHit],
    rows: &FtsRows,
    match_expression: &str,
    options: &SnippetOptions,
) -> rusqlite::Result<Vec<SnippetV1>> {
    hits.iter()
        .map(|hit| {
            // Rule 0: spans are needed only to cut the body or to highlight.
            let needs_spans = hit.body.len() > options.max_bytes as usize || options.highlights;
            let spans = match rows.get(&hit.write_cursor).filter(|_| needs_spans) {
                Some(&(table, rowid)) => {
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
    let marked: Option<String> = tx
        .prepare_cached(table.highlight_sql())?
        .query_row(
            rusqlite::params![OPEN.to_string(), CLOSE.to_string(), match_expression, rowid],
            |row| row.get(0),
        )
        .optional()?;
    Ok(marked.and_then(|marked| parse_marked(body, &marked, OPEN, CLOSE)))
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
