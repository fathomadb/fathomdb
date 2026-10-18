//! EP-17 pure snippet selector (`snippet_for`), specification
//! `dev/design/agent-access/match-centred-snippets.md` §§ 6, 7.2. No database:
//! every test calls the pure function directly.

use fathomdb_engine::{snippet_for, EngineError, SnippetAnchor, SnippetOptions, SnippetV1};

fn is_invalid_argument(result: Result<(), EngineError>) -> bool {
    matches!(result, Err(EngineError::InvalidArgument { .. }))
}

// § 7.2: defaults and option ranges.
#[test]
fn options_default_and_reject_out_of_range_values() {
    let d = SnippetOptions::default();
    assert_eq!(
        (d.max_bytes, d.max_windows, d.head_bytes, d.highlights, d.max_point_queries),
        (300, 1, 0, false, 20)
    );
    assert!(d.validate().is_ok());

    for ok in [
        SnippetOptions { max_bytes: 40, ..d },
        SnippetOptions { max_bytes: 4000, ..d },
        SnippetOptions { max_windows: 3, ..d },
        SnippetOptions { head_bytes: 150, ..d },
        SnippetOptions { max_point_queries: 0, ..d },
    ] {
        assert!(ok.validate().is_ok(), "{ok:?} must validate");
    }
    for bad in [
        SnippetOptions { max_bytes: 39, ..d },
        SnippetOptions { max_bytes: 4001, ..d },
        SnippetOptions { max_windows: 0, ..d },
        SnippetOptions { max_windows: 4, ..d },
        SnippetOptions { head_bytes: 151, ..d },
    ] {
        assert!(is_invalid_argument(bad.validate()), "{bad:?} must be InvalidArgument");
    }
    assert_eq!(
        [SnippetAnchor::Whole, SnippetAnchor::Match, SnippetAnchor::Start]
            .map(SnippetAnchor::as_str),
        ["whole", "match", "start"]
    );
    let _: fn(&str, &[(u32, u32)], &SnippetOptions) -> SnippetV1 = snippet_for;
}

fn opts(max_bytes: u32) -> SnippetOptions {
    SnippetOptions { max_bytes, ..SnippetOptions::default() }
}

// § 6.3 rule 1: a body that fits is returned whole, with every span.
#[test]
fn rule1_body_that_fits_is_returned_whole() {
    let body = "The quick brown fox jumps over the lazy dog.";
    assert_eq!(body.len(), 44);
    let spans = [(4, 9), (16, 19)];

    let s = snippet_for(body, &spans, &opts(44));
    assert_eq!(s.text, body);
    assert_eq!(s.anchor, SnippetAnchor::Whole);
    assert_eq!(s.windows, vec![(0, 44)]);
    assert_eq!(s.spans, spans.to_vec());
    assert!(s.highlights.is_empty(), "highlights are off by default");
    assert!(!s.cut);

    let lit = snippet_for(body, &spans, &SnippetOptions { highlights: true, ..opts(300) });
    assert_eq!(lit.text, body);
    assert_eq!(lit.highlights, spans.to_vec());

    let empty = snippet_for("", &[], &opts(300));
    assert_eq!(empty.text, "");
    assert_eq!(empty.anchor, SnippetAnchor::Whole);
    assert!(empty.windows.is_empty());
    assert!(!empty.cut);
}

// § 6.1: unicode61 ends a token before a combining mark, inside the grapheme
// cluster; the selector widens span ends (and starts) to grapheme boundaries.
#[test]
fn spans_are_widened_to_grapheme_boundaries() {
    let nfd = "Le cafe\u{301} est ouvert.";
    let lit = SnippetOptions { highlights: true, ..opts(300) };
    let s = snippet_for(nfd, &[(3, 7)], &lit);
    assert_eq!(s.spans, vec![(3, 9)], "the accent belongs to the span");
    assert_eq!(s.highlights, vec![(3, 9)]);
    assert_eq!(&s.text[3..9], "cafe\u{301}");

    // A span that starts on the combining mark widens back to the cluster.
    let s = snippet_for(nfd, &[(7, 9)], &lit);
    assert_eq!(s.spans, vec![(6, 9)]);

    // Widening can make neighbours overlap; they merge.
    let s = snippet_for(nfd, &[(3, 7), (7, 9)], &lit);
    assert_eq!(s.spans, vec![(3, 9)]);
}

// § 6.3 rule 2: no spans gives the prefix, cut back to a word boundary within
// `max_bytes − len(" …")`, then `" …"`; a grapheme boundary when no word
// boundary falls within 32 bytes.
#[test]
fn rule2_no_spans_gives_the_prefix_on_a_word_boundary() {
    let body =
        "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho";
    let s = snippet_for(body, &[], &opts(62));
    assert_eq!(s.text, "alpha beta gamma delta epsilon zeta eta theta iota kappa …");
    assert_eq!(s.anchor, SnippetAnchor::Start);
    assert_eq!(s.windows, vec![(0, 56)]);
    assert!(s.spans.is_empty());
    assert!(s.cut);
    assert!(s.text.len() <= 62);

    // Spans that are all invalid count as no spans.
    let s = snippet_for(body, &[(5, 5), (80, 200), (90, 95)], &opts(62));
    assert_eq!(s.anchor, SnippetAnchor::Start);
    assert_eq!(s.windows, vec![(0, 56)]);

    // One long UAX #29 word of NFD clusters (3 bytes each): no word boundary
    // within 32 bytes of 56, so the edge falls back to the grapheme boundary 54.
    let blob = "e\u{301}".repeat(40);
    let s = snippet_for(&blob, &[], &opts(60));
    assert_eq!(s.windows, vec![(0, 54)]);
    assert_eq!(s.text, format!("{} …", "e\u{301}".repeat(18)));
    assert!(s.cut);
}

const FILLER: [&str; 5] = ["lorem", "ipsum", "dolor", "sit", "amet"];

/// A body of space-separated filler words with each `(offset, term)` placed
/// at the first word start at or after `offset`, padded to at least `len`
/// bytes. Returns the body and the terms' spans in order.
fn body_with(terms: &[(usize, &str)], len: usize) -> (String, Vec<(u32, u32)>) {
    let mut body = String::new();
    let mut spans = Vec::new();
    let mut filler = FILLER.iter().cycle();
    let mut pending = terms.iter().peekable();
    while body.len() < len || pending.peek().is_some() {
        if !body.is_empty() {
            body.push(' ');
        }
        match pending.peek() {
            Some(&&(at, term)) if body.len() >= at => {
                let start = body.len();
                body.push_str(term);
                spans.push((start as u32, body.len() as u32));
                pending.next();
            }
            _ => body.push_str(filler.next().unwrap()),
        }
    }
    (body, spans)
}

fn shows(s: &SnippetV1, span: (u32, u32)) -> bool {
    s.windows.iter().any(|&(ws, we)| ws <= span.0 && span.1 <= we)
}

// § 6.3 rule 4 (T-17, first half): distinct identities outrank repeats;
// identity is the lowercased span text; ties go to the earlier start.
#[test]
fn rule4_window_scoring_prefers_distinct_terms() {
    let (body, spans) = body_with(
        &[(300, "apple"), (330, "apple"), (360, "apple"), (1500, "pear"), (1540, "plum")],
        3000,
    );
    let s = snippet_for(&body, &spans, &opts(120));
    assert_eq!(s.anchor, SnippetAnchor::Match);
    assert!(shows(&s, spans[3]) && shows(&s, spans[4]), "{s:?}");
    assert!(!shows(&s, spans[0]));

    // `Apple` and `APPLE` are one identity, so `pear plum` wins.
    let (body, spans) =
        body_with(&[(300, "Apple"), (340, "APPLE"), (1500, "pear"), (1540, "plum")], 3000);
    let s = snippet_for(&body, &spans, &opts(120));
    assert!(shows(&s, spans[2]) && shows(&s, spans[3]), "{s:?}");

    // Equal scores: the earlier window wins.
    let (body, spans) =
        body_with(&[(300, "pear"), (340, "plum"), (1500, "pear"), (1540, "plum")], 3000);
    let s = snippet_for(&body, &spans, &opts(120));
    assert!(shows(&s, spans[0]) && shows(&s, spans[1]), "{s:?}");
    assert!(!shows(&s, spans[2]));
}

/// The rule 9 join of `windows` over `body`.
fn joined(body: &str, windows: &[(u32, u32)]) -> String {
    let mut text = String::new();
    for (i, &(s, e)) in windows.iter().enumerate() {
        if i > 0 {
            text.push_str(" … ");
        } else if s > 0 {
            text.push_str("… ");
        }
        text.push_str(&body[s as usize..e as usize]);
    }
    if windows.last().is_some_and(|&(_, e)| (e as usize) < body.len()) {
        text.push_str(" …");
    }
    text
}

// § 6.3 rule 9 and § 6.4: the text is the ellipsis join of the windows, at
// most `max_bytes`; highlights are ranges into `text` of the spans shown in
// full, and only those.
#[test]
fn rule9_assembly_joins_windows_and_maps_highlights_into_text() {
    let (body, spans) = body_with(&[(1500, "pear"), (1540, "plum"), (1700, "fig")], 3000);
    let lit = SnippetOptions { highlights: true, ..opts(120) };
    let s = snippet_for(&body, &spans, &lit);
    assert!(s.windows[0].0 > 0);
    assert_eq!(s.text, joined(&body, &s.windows));
    assert!(s.text.starts_with("… ") && s.text.ends_with(" …"));
    assert!(s.text.len() <= 120);
    let lit_text: Vec<&str> =
        s.highlights.iter().map(|&(a, b)| &s.text[a as usize..b as usize]).collect();
    assert_eq!(lit_text, ["pear", "plum"], "fig lies outside the window");

    // A window at byte 0 has no leading ellipsis; the highlight is not shifted.
    let (body, spans) = body_with(&[(0, "pear")], 3000);
    let s = snippet_for(&body, &spans, &lit);
    assert_eq!(s.windows[0].0, 0);
    assert!(s.text.starts_with("pear "));
    assert_eq!(s.highlights, vec![(0, 4)]);

    // Highlights stay off unless asked for.
    let s = snippet_for(&body, &spans, &opts(120));
    assert!(s.highlights.is_empty());
}

// § 6.3 rule 5: the window is shifted so its spans sit in the middle,
// clamped to the body. Snapping (rule 6) may move each edge by up to 32 bytes.
#[test]
fn rule5_window_is_centred_on_its_spans() {
    let (body, spans) = body_with(&[(2000, "pear")], 4000);
    let s = snippet_for(&body, &spans, &opts(300));
    let (ws, we) = s.windows[0];
    let before = spans[0].0 - ws;
    let after = we - spans[0].1;
    assert!(before.abs_diff(after) <= 70, "before {before}, after {after}: {s:?}");
    assert!(we - ws >= 292 - 70, "the window uses its budget: {s:?}");

    // Near the end the window is clamped to the body and keeps its width.
    let (body, spans) = body_with(&[(2000, "pear")], 2010);
    let s = snippet_for(&body, &spans, &opts(300));
    let (ws, we) = s.windows[0];
    assert_eq!(we as usize, body.len());
    assert!(!s.text.ends_with('…'));
    assert!(we - ws >= 296 - 70, "{s:?}");
    assert!(s.text.len() <= 300);
}

// § 6.3 rule 6: window edges snap to UAX #29 word boundaries, so no window
// starts or ends inside a word, and none starts or ends with whitespace.
#[test]
fn rule6_window_edges_snap_to_word_boundaries() {
    for at in 2000..2040 {
        let (body, spans) = body_with(&[(at, "pear")], 4000);
        let s = snippet_for(&body, &spans, &opts(300));
        let (ws, we) = (s.windows[0].0 as usize, s.windows[0].1 as usize);
        let shown = &body[ws..we];
        assert!(body.as_bytes()[ws - 1] == b' ', "start inside a word: {shown:?}");
        assert!(body.as_bytes()[we] == b' ', "end inside a word: {shown:?}");
        assert_eq!(shown.trim(), shown, "no edge whitespace");
        assert!(shows(&s, spans[0]));
        assert!(s.text.len() <= 300);
    }
}
