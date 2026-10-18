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
