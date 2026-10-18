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
