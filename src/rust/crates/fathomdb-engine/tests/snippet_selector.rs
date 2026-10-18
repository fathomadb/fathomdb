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

/// Space-separated `x` words, exactly `len` bytes (at least 2), ending in a
/// space.
fn pad(len: usize) -> String {
    let mut s = String::new();
    while len - s.len() > 10 {
        s.push_str("xxxx ");
    }
    s.push_str(&"x".repeat(len - s.len() - 1));
    s.push(' ');
    s
}

/// `head`, then `a`, a gap, and `b`, with `b`'s span ending exactly
/// `width − 1` bytes after `a`'s span starts, so the centred window's end
/// target falls one byte past `b`'s span. Returns the body and both spans.
fn edge_case(
    a: &str,
    a_span: (usize, usize),
    b: &str,
    b_span: (usize, usize),
) -> (String, Vec<(u32, u32)>) {
    let width = 120 - 8;
    let head = pad(1000);
    let a_start = head.len() + a_span.0;
    let gap = width - 1 - (a.len() - a_span.0) - b_span.1 - 1;
    let body = format!("{head}{a} {}{b} {}", pad(gap), pad(1000));
    let b_start = head.len() + a.len() + 1 + gap;
    let spans = vec![
        (a_start as u32, (head.len() + a_span.1) as u32),
        ((b_start + b_span.0) as u32, (b_start + b_span.1) as u32),
    ];
    assert_eq!(spans[1].1 - spans[0].0, (width - 1) as u32);
    (body, spans)
}

// § 6.3 rule 6 (T-18): UAX #29 words and unicode61 tokens differ (`don't` is
// one UAX word, two tokens; `e-mails` is three UAX words), so snapping to a
// word boundary could cut a counted span. Every span counted in rule 4 is
// still inside the window after snapping.
#[test]
fn rule6_snapping_keeps_every_counted_span() {
    let cases = [
        // `don` ends inside the UAX word `don't` at the end edge.
        edge_case("pear", (0, 4), "don't", (0, 3)),
        // `t` starts inside the UAX word `don't` at the start edge.
        edge_case("don't", (4, 5), "pear", (0, 4)),
        // `e-mail` ends inside `e-mails` at the end edge.
        edge_case("pear", (0, 4), "e-mails", (0, 6)),
    ];
    for (body, spans) in cases {
        let s = snippet_for(&body, &spans, &opts(120));
        let shown: Vec<&str> = spans.iter().map(|&(a, b)| &body[a as usize..b as usize]).collect();
        assert!(spans.iter().all(|&sp| shows(&s, sp)), "{shown:?} not all shown: {s:?}");
        assert!(s.text.len() <= 120);
    }
}

// § 6.3 rules 4.3 and 6 (T-17, second half): a UAX #29 sentence start within
// 64 bytes before the first span earns the score bonus, and the window start
// moves to it when the budget still covers the last counted span.
#[test]
fn rule6_sentence_start_before_the_first_span_moves_the_window_start() {
    let body = format!("{}The story ends. Then the ripe pear fell. {}", pad(1500), pad(1500));
    let then = body.find("Then").unwrap();
    let pear = body.find("pear").unwrap();
    let spans = [(pear as u32, (pear + 4) as u32)];
    let s = snippet_for(&body, &spans, &opts(120));
    assert_eq!(s.windows[0].0 as usize, then, "{s:?}");
    assert!(s.text.starts_with("… Then the ripe pear"));

    // The bonus breaks a tie: the later `pear`, after a sentence start, wins.
    let body = format!("{}pear {}The end. A pear. {}", pad(300), pad(1000), pad(1000));
    let early = 300;
    let late = body.rfind("pear").unwrap();
    let spans = [(early, early + 4), (late as u32, (late + 4) as u32)];
    let s = snippet_for(&body, &spans, &opts(120));
    assert!(shows(&s, spans[1]) && !shows(&s, spans[0]), "{s:?}");

    // No move when the sentence start would leave the last span out of budget.
    let body = format!("{}Then {}pear {}fig {}", pad(1500), pad(55), pad(97), pad(1500));
    let then = body.find("Then").unwrap();
    let (pear, fig) = (body.find("pear").unwrap(), body.find("fig").unwrap());
    let spans = [(pear as u32, (pear + 4) as u32), (fig as u32, (fig + 3) as u32)];
    assert!(pear - then <= 64 && then + 112 < fig + 3);
    let s = snippet_for(&body, &spans, &opts(120));
    assert!(s.windows[0].0 as usize > then, "{s:?}");
    assert!(shows(&s, spans[0]) && shows(&s, spans[1]), "{s:?}");
}

fn grapheme_boundaries(body: &str) -> Vec<usize> {
    use unicode_segmentation::UnicodeSegmentation;
    let mut v: Vec<usize> = body.grapheme_indices(true).map(|(i, _)| i).collect();
    v.push(body.len());
    v
}

// T-5: emoji ZWJ sequences, flags and combining marks at every window edge;
// no edge splits a grapheme cluster.
#[test]
fn t5_no_window_edge_splits_a_grapheme_cluster() {
    let clusters = [
        "\u{1F469}\u{200D}\u{1F469}\u{200D}\u{1F467}", // family, ZWJ sequence
        "\u{1F1EB}\u{1F1F7}",                          // flag, regional-indicator pair
        "e\u{301}",                                    // combining acute
        "a\u{308}\u{301}",                             // two combining marks
        "\u{1F44D}\u{1F3FD}",                          // emoji + skin-tone modifier
    ];
    for cluster in clusters {
        for lead in 0..6 {
            let body =
                format!("{}{}pear{}", "x".repeat(lead), cluster.repeat(120), cluster.repeat(120));
            let pear = body.find("pear").unwrap();
            let bounds = grapheme_boundaries(&body);
            for max_bytes in (40..=400).step_by(9) {
                for spans in [vec![], vec![(pear as u32, pear as u32 + 4)]] {
                    let s = snippet_for(&body, &spans, &opts(max_bytes));
                    for &(a, b) in &s.windows {
                        assert!(
                            bounds.contains(&(a as usize)) && bounds.contains(&(b as usize)),
                            "{cluster:?} lead {lead} max {max_bytes}: window ({a}, {b}) splits a cluster"
                        );
                    }
                    assert!(s.text.len() <= max_bytes as usize);
                    if !spans.is_empty() {
                        assert!(shows(&s, spans[0]), "{cluster:?} {max_bytes}: {s:?}");
                    }
                }
            }
        }
    }
}
