//! **FathomDB query** — the JSON-filter AST and its validation surface.
//!
//! An internal leaf crate of the FathomDB workspace, kept separate so the
//! filter grammar can be validated without pulling in the engine. **Application
//! code should depend on the `fathomdb` facade crate instead**, which exposes
//! the supported filter types (`Predicate`, `ScalarValue`, `ComparisonOp`,
//! `SearchFilter`) as governed surface.
//!
//! The filter grammar is deliberately **closed** — a fixed set of comparison
//! operators over an allowlisted set of JSON paths — rather than an open DSL.
//! Values are always bound as parameterized SQL and never interpolated.

use std::collections::HashSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryAst {
    pub raw: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledQuery {
    pub match_expression: String,
}

/// Function words stripped from the content-OR query (they add false matches
/// under OR semantics without carrying topical signal). Mirrors the IR-C
/// `content-OR` experiment list (`dev/plans/runs/performance-output-and-compare.md`,
/// 2026-06-10b): the smallest list that lifted exploratory recall with no
/// exact_fact cost. The last twelve entries are the fragments that splitting
/// on the apostrophe leaves ('s, n't, 'll, 're, 've, 'm) and short function
/// words; short content tokens such as "d", "c" or "d3" are not stopwords.
const STOPWORDS: &[&str] = &[
    "the", "and", "for", "are", "was", "were", "what", "when", "where", "who", "whom", "which",
    "how", "why", "did", "does", "do", "is", "of", "to", "in", "on", "at", "by", "an", "a", "it",
    "its", "this", "that", "these", "those", "with", "from", "as", "be", "or", "if", "about",
    "into", "over", "than", "then", "they", "them", "their", "you", "your", "we", "our", "i", "s",
    "t", "ll", "re", "ve", "m", "my", "me", "us", "am", "so", "up",
];

/// Content tokens of a query: lowercased, split on non-alphanumeric, empty
/// fragments and stopwords removed, de-duplicated in first-seen order. Tokens
/// of every length are kept: short tokens such as "c", "d3" or "1" carry query
/// meaning. These are the OR terms
/// of the compiled MATCH expression. Splitting on non-alphanumeric drops every
/// FTS5 control character (`*`, `"`, `:`, `^`, `(`, `)`, `,`), so the emitted
/// tokens are pure literals — the injection-safety property (AC-038).
#[must_use]
fn content_tokens(raw: &str) -> Vec<String> {
    let stop: HashSet<&str> = STOPWORDS.iter().copied().collect();
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for token in raw.to_lowercase().split(|c: char| !c.is_alphanumeric()) {
        if token.is_empty() || stop.contains(token) {
            continue;
        }
        if seen.insert(token.to_string()) {
            out.push(token.to_string());
        }
    }
    out
}

/// Compile a raw query into an FTS5 MATCH expression.
///
/// IR-C (2026-06-10b/c, `performance-output-and-compare.md`): the production
/// arm was an **AND** of every whitespace token, which near-zeroes recall on
/// natural-language questions (every token must be present). The validated
/// recipe is **content-OR** — OR over the content tokens (stopwords stripped) —
/// which any-token-matches and lets `bm25()` rank by overlap, the way the
/// same-dataset BM25 baselines (EnronQA/QAConv) are run.
///
/// All-stopword / symbol-only queries (no content tokens) fall back
/// to an OR over the raw whitespace tokens as injection-safe quoted phrases, so
/// such a query still searches instead of returning nothing.
#[must_use]
pub fn compile_text_query(raw: impl Into<String>) -> CompiledQuery {
    let raw = raw.into();
    let content = content_tokens(&raw);
    let match_expression = if content.is_empty() {
        raw.split_whitespace()
            .filter(|token| !token.is_empty())
            .map(|token| format!("\"{}\"", token.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" OR ")
    } else {
        content.into_iter().map(|token| format!("\"{token}\"")).collect::<Vec<_>>().join(" OR ")
    };

    CompiledQuery { match_expression }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use proptest::prelude::*;

    use super::{compile_text_query, STOPWORDS};

    #[test]
    fn content_tokens_are_or_joined() {
        // IR-C content-OR: content tokens OR-joined (was AND).
        let compiled = compile_text_query("alpha   beta");
        assert_eq!(compiled.match_expression, "\"alpha\" OR \"beta\"");
    }

    #[test]
    fn stopwords_are_dropped() {
        // "of"/"the" are stopwords — only content survives.
        let compiled = compile_text_query("status of the alpha");
        assert_eq!(compiled.match_expression, "\"status\" OR \"alpha\"");
    }

    #[test]
    fn short_stopwords_are_still_dropped() {
        // "a" and "i" are one-character stopwords; keeping short tokens must not
        // let them through. "d" is a one-character content token and is kept.
        let compiled = compile_text_query("a study i ran of vitamin D");
        assert_eq!(compiled.match_expression, "\"study\" OR \"ran\" OR \"vitamin\" OR \"d\"");
    }

    #[test]
    fn one_and_two_char_content_tokens_are_kept() {
        // Owner ruling (0.8.28 Slice 40): dropping 1–2 character tokens deleted
        // query meaning ("C", "D3", "II", "1"); they are content.
        let compiled = compile_text_query("Vitamin C-Enriched Bacon");
        assert_eq!(compiled.match_expression, "\"vitamin\" OR \"c\" OR \"enriched\" OR \"bacon\"");
        let compiled = compile_text_query("IGF-1 and vitamin D3 in Physicians' Study II");
        assert_eq!(
            compiled.match_expression,
            "\"igf\" OR \"1\" OR \"vitamin\" OR \"d3\" OR \"physicians\" OR \"study\" OR \"ii\""
        );
    }

    /// The Unit D additions to the stop list: possessive and contraction
    /// fragments, then short function words.
    const UNIT_D_STOPWORDS: [&str; 12] =
        ["s", "t", "ll", "re", "ve", "m", "my", "me", "us", "am", "so", "up"];

    #[test]
    fn unit_d_words_are_in_the_stop_list_and_content_letters_are_not() {
        for word in UNIT_D_STOPWORDS {
            assert!(STOPWORDS.contains(&word), "{word:?} must be a stopword");
        }
        for word in ["d", "c", "d3", "ii", "1", "b12"] {
            assert!(!STOPWORDS.contains(&word), "{word:?} must stay content");
        }
    }

    #[test]
    fn each_unit_d_word_alone_takes_the_fallback() {
        // Upper case tells the paths apart: the content path lowercases, the
        // fallback emits the raw token.
        for word in UNIT_D_STOPWORDS {
            let upper = word.to_uppercase();
            assert_eq!(
                compile_text_query(upper.as_str()).match_expression,
                format!("\"{upper}\""),
                "{word:?} alone has no content token"
            );
        }
    }

    /// The Slice 40 Unit H1 additions, the stop-list verdict's words
    /// (`stoplist-verdict.v1.json`, harness `3f0789271`).
    const UNIT_H_STOPWORDS: [&str; 3] = ["had", "long", "suggest"];

    #[test]
    fn unit_h_words_are_in_the_stop_list_and_content_tokens_are_not() {
        for word in UNIT_H_STOPWORDS {
            assert!(STOPWORDS.contains(&word), "{word:?} must be a stopword");
        }
        for word in UNIT_D_STOPWORDS {
            assert!(STOPWORDS.contains(&word), "{word:?} must stay a stopword");
        }
        for word in ["d", "c", "d3", "ii", "1", "b12"] {
            assert!(!STOPWORDS.contains(&word), "{word:?} must stay content");
        }
        // 63 words at 5b09afd44 (the 51 original plus the twelve of Unit D),
        // plus the verdict's: a drift on either side of the harness copy fails here.
        assert_eq!(STOPWORDS.len(), 63 + UNIT_H_STOPWORDS.len());
        let distinct: HashSet<&str> = STOPWORDS.iter().copied().collect();
        assert_eq!(distinct.len(), STOPWORDS.len(), "no word is listed twice");
    }

    #[test]
    fn each_unit_h_word_alone_takes_the_fallback() {
        // Upper case tells the paths apart, as for Unit D: the content path
        // would emit the lowercased token, the fallback emits the raw one.
        for word in UNIT_H_STOPWORDS {
            let upper = word.to_uppercase();
            assert_eq!(
                compile_text_query(upper.as_str()).match_expression,
                format!("\"{upper}\""),
                "{word:?} alone has no content token"
            );
        }
    }

    #[test]
    fn unit_h_question_shapes_compile_to_content_terms() {
        // The analysis shapes keep their words: the verdict admitted none of
        // "many", "have", "can", "recommend" or "some".
        assert_eq!(
            compile_text_query("How many siblings do I have?").match_expression,
            "\"many\" OR \"siblings\" OR \"have\""
        );
        assert_eq!(
            compile_text_query("Can you recommend some").match_expression,
            "\"can\" OR \"recommend\" OR \"some\""
        );
        assert_eq!(
            compile_text_query("How long had I been running before the race?").match_expression,
            "\"been\" OR \"running\" OR \"before\" OR \"race\""
        );
        assert_eq!(
            compile_text_query("Can you suggest a recipe?").match_expression,
            "\"can\" OR \"recipe\""
        );
    }

    #[test]
    fn possessive_and_contraction_fragments_are_dropped() {
        assert_eq!(compile_text_query("Cow's Milk").match_expression, "\"cow\" OR \"milk\"");
        assert_eq!(compile_text_query("don't panic").match_expression, "\"don\" OR \"panic\"");
        assert_eq!(
            compile_text_query("I've said we'll meet if you're free, I'm told").match_expression,
            "\"said\" OR \"meet\" OR \"free\" OR \"told\""
        );
    }

    #[test]
    fn short_function_words_are_dropped() {
        assert_eq!(
            compile_text_query("my dog likes me so I take us up the hill as I am").match_expression,
            "\"dog\" OR \"likes\" OR \"take\" OR \"hill\""
        );
    }

    #[test]
    fn short_content_tokens_survive_the_unit_d_stop_list() {
        assert_eq!(
            compile_text_query("my vitamin D level").match_expression,
            "\"vitamin\" OR \"d\" OR \"level\""
        );
        assert_eq!(
            compile_text_query("so is IGF-1 up with D3, B12 or C in study II?").match_expression,
            "\"igf\" OR \"1\" OR \"d3\" OR \"b12\" OR \"c\" OR \"study\" OR \"ii\""
        );
    }

    #[test]
    fn short_token_only_query_takes_the_content_path() {
        // A query whose only token is short is content, not a fallback: the
        // token is lowercased and emitted as a pure literal.
        assert_eq!(compile_text_query("D3").match_expression, "\"d3\"");
        assert_eq!(compile_text_query("C").match_expression, "\"c\"");
    }

    #[test]
    fn empty_split_fragments_are_never_emitted() {
        // Runs of separators split into empty fragments; none may become an
        // empty `""` phrase.
        assert_eq!(compile_text_query("alpha--beta").match_expression, "\"alpha\" OR \"beta\"");
        assert_eq!(compile_text_query("  C  ..  D ").match_expression, "\"c\" OR \"d\"");
    }

    #[test]
    fn duplicate_content_tokens_collapse_in_order() {
        let compiled = compile_text_query("alpha beta alpha");
        assert_eq!(compiled.match_expression, "\"alpha\" OR \"beta\"");
    }

    #[test]
    fn control_characters_are_stripped_to_literals() {
        // FTS5 control syntax splits into literal content tokens — no operators
        // reach SQLite (AC-038).
        let compiled = compile_text_query("alpha* AND \"beta\" NEAR(gamma)");
        assert_eq!(compiled.match_expression, "\"alpha\" OR \"beta\" OR \"near\" OR \"gamma\"");
    }

    #[test]
    fn all_stopword_query_falls_back_to_raw_or() {
        // No content tokens: OR over the raw whitespace tokens (still searches).
        let compiled = compile_text_query("who are we");
        assert_eq!(compiled.match_expression, "\"who\" OR \"are\" OR \"we\"");
    }

    #[test]
    fn escapes_double_quotes_in_fallback_tokens() {
        // The fallback path keeps injection-safe quote-escaping.
        let compiled = compile_text_query("an \"of");
        assert_eq!(compiled.match_expression, "\"an\" OR \"\"\"of\"");
    }

    #[test]
    fn all_short_stopword_query_still_falls_back_to_raw_or() {
        // Only one-character stopwords: no content tokens, so the fallback path
        // is unchanged.
        let compiled = compile_text_query("a I");
        assert_eq!(compiled.match_expression, "\"a\" OR \"I\"");
    }

    /// The content tokens the compiler must emit, written from the contract:
    /// lowercase, split on non-alphanumeric, drop empty fragments and
    /// stopwords, de-duplicate in first-seen order. No length filter.
    fn expected_content_tokens(raw: &str) -> Vec<String> {
        let stop: HashSet<&str> = STOPWORDS.iter().copied().collect();
        let mut seen = HashSet::new();
        raw.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| !t.is_empty() && !stop.contains(t))
            .map(str::to_string)
            .filter(|t| seen.insert(t.clone()))
            .collect()
    }

    /// Splits a compiled expression into its quoted terms.
    fn terms(expression: &str) -> Vec<&str> {
        if expression.is_empty() {
            Vec::new()
        } else {
            expression.split(" OR ").collect()
        }
    }

    /// True iff `term` is one FTS5 string literal: wrapped in `"` with every
    /// interior `"` doubled.
    fn is_string_literal(term: &str) -> bool {
        let Some(inner) = term.strip_prefix('"').and_then(|t| t.strip_suffix('"')) else {
            return false;
        };
        term.len() >= 2 && inner.replace("\"\"", "").find('"').is_none()
    }

    proptest! {
        /// AC40-17: every non-stopword alphanumeric token of the query, of any
        /// length, is an OR term, in first-seen order; with none, the fallback
        /// path runs.
        #[test]
        fn prop_content_terms_are_every_non_stopword_token(
            raw in "[a-zA-Z0-9 *\"():^,.'_+-]{0,40}",
        ) {
            let expected = expected_content_tokens(&raw);
            let compiled = compile_text_query(raw.clone());
            if expected.is_empty() {
                let fallback: Vec<String> = raw
                    .split_whitespace()
                    .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
                    .collect();
                prop_assert_eq!(compiled.match_expression, fallback.join(" OR "));
            } else {
                let quoted: Vec<String> =
                    expected.iter().map(|t| format!("\"{t}\"")).collect();
                prop_assert_eq!(compiled.match_expression, quoted.join(" OR "));
            }
        }

        /// AC40-18 (AC-038): every emitted term is a single FTS5 string
        /// literal, so no FTS5 operator or column filter can reach SQLite.
        #[test]
        fn prop_every_term_is_one_fts5_string_literal(raw in "\\PC{0,40}") {
            let compiled = compile_text_query(raw);
            for term in terms(&compiled.match_expression) {
                prop_assert!(is_string_literal(term), "term {:?} is not one literal", term);
            }
        }

        /// AC40-18 (AC-038): when the query has a content token, every term is
        /// a non-empty alphanumeric literal (the content path never carries
        /// punctuation, quotes, or empty phrases).
        #[test]
        fn prop_content_path_terms_are_alphanumeric(raw in "\\PC{0,40}") {
            prop_assume!(!expected_content_tokens(&raw).is_empty());
            let compiled = compile_text_query(raw);
            for term in terms(&compiled.match_expression) {
                let inner = &term[1..term.len() - 1];
                prop_assert!(!inner.is_empty(), "empty content term");
                prop_assert!(
                    inner.chars().all(char::is_alphanumeric),
                    "content term {:?} is not alphanumeric",
                    term
                );
            }
        }
    }
}
