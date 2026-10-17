//! 0.8.28 Slice 41 unit Q — configurable query stop words (AC41-34, AC41-35,
//! AC41-36, AC41-38). Expected expressions are written by hand from
//! requirements REQ41-Q1..Q5; the hashes were computed independently with
//! Python `hashlib.sha256("\n".join(sorted(words)))`.

use std::collections::HashSet;

use proptest::prelude::*;
use sha2::{Digest, Sha256};

use super::{
    compile_text_query, compile_text_query_with, QueryStopWords, StopWordBase, StopWordError,
    StopWordRejection, StopWordSet, DEFAULT_STOP_WORDS_SHA256, MAX_STOP_WORDS_PER_LIST,
    MAX_STOP_WORD_CHARS, STOPWORDS,
};

const H1_SHA256: &str = "4e1a05394b998ef1234cf0ff24a1692b33832f675a6cdadf131cd32c17964d99";
const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|word| (*word).to_string()).collect()
}

fn setting(base: StopWordBase, extra: &[&str], keep: &[&str]) -> QueryStopWords {
    QueryStopWords { base, extra: words(extra), keep: words(keep) }
}

fn set(base: StopWordBase, extra: &[&str], keep: &[&str]) -> StopWordSet {
    setting(base, extra, keep).validate().expect("valid setting")
}

fn compile(raw: &str, stop_words: &StopWordSet) -> String {
    compile_text_query_with(raw, stop_words).match_expression
}

fn h1() -> StopWordSet {
    set(StopWordBase::Fathomdb, &["had", "long", "suggest"], &[])
}

/// Queries with hand-written expressions under the built-in list, including
/// the content-OR, stopword and fallback shapes the `tests` module pins.
const PINNED: &[(&str, &str)] = &[
    ("alpha   beta", "\"alpha\" OR \"beta\""),
    ("status of the alpha", "\"status\" OR \"alpha\""),
    ("the and of", "\"the\" OR \"and\" OR \"of\""),
    ("a I", "\"a\" OR \"I\""),
    ("say \"hi\"", "\"say\" OR \"hi\""),
    (
        "how long had the dentist suggest flossing",
        "\"long\" OR \"had\" OR \"dentist\" OR \"suggest\" OR \"flossing\"",
    ),
];

/// AC41-38: the constant is the hash of the built-in list, recomputed here.
#[test]
fn default_hash_constant_matches_the_list() {
    let mut sorted: Vec<&str> = STOPWORDS.to_vec();
    sorted.sort_unstable();
    let digest: String =
        Sha256::digest(sorted.join("\n").as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(digest, DEFAULT_STOP_WORDS_SHA256);
    assert_eq!(StopWordSet::default().sha256(), DEFAULT_STOP_WORDS_SHA256);
    assert_eq!(StopWordSet::default().words().count(), 63);
}

/// AC41-38: the hash is of the effective words in Unicode scalar order.
#[test]
fn effective_hash_is_sorted_words_joined_by_newline() {
    assert_eq!(set(StopWordBase::Fathomdb, &[], &[]).sha256(), DEFAULT_STOP_WORDS_SHA256);
    assert_eq!(h1().sha256(), H1_SHA256);
    assert_eq!(set(StopWordBase::None, &[], &[]).sha256(), EMPTY_SHA256);
    assert_eq!(
        set(StopWordBase::None, &["é", "z"], &[]).sha256(),
        "83de4d7323290ebdba64e03eb87ea54a0c802134c78e330f81ffd07252f7c8fd"
    );
}

/// AC41-34: the default set compiles every pinned query as today.
#[test]
fn default_set_compiles_like_today() {
    let default = StopWordSet::default();
    let explicit = set(StopWordBase::Fathomdb, &[], &[]);
    for (raw, expected) in PINNED {
        assert_eq!(compile(raw, &default), *expected, "{raw:?}");
        assert_eq!(compile(raw, &explicit), *expected, "{raw:?}");
        assert_eq!(compile_text_query(*raw).match_expression, *expected, "{raw:?}");
    }
}

/// AC41-35: extra words drop, keep words return, base none keeps every token.
#[test]
fn extra_keep_and_none_bases_shape_the_terms() {
    assert_eq!(
        compile("how long had the dentist suggest flossing", &h1()),
        "\"dentist\" OR \"flossing\""
    );
    assert_eq!(
        compile("what is fathomdb", &set(StopWordBase::Fathomdb, &[], &["what"])),
        "\"what\" OR \"fathomdb\""
    );
    let none = set(StopWordBase::None, &[], &[]);
    assert_eq!(compile("the cat and the hat", &none), "\"the\" OR \"cat\" OR \"and\" OR \"hat\"");
    assert_eq!(
        compile("the cat and the hat", &set(StopWordBase::None, &["cat"], &[])),
        "\"the\" OR \"and\" OR \"hat\""
    );
}

/// AC41-35: the all-stopword fallback follows the effective set.
#[test]
fn all_stopword_fallback_uses_the_effective_set() {
    let dentist = set(StopWordBase::Fathomdb, &["dentist"], &[]);
    assert_eq!(compile("the Dentist!", &dentist), "\"the\" OR \"Dentist!\"");
    assert_eq!(compile("The!", &StopWordSet::default()), "\"The!\"");
    assert_eq!(compile("The!", &set(StopWordBase::Fathomdb, &[], &["the"])), "\"the\"");
    assert_eq!(compile("long had", &h1()), "\"long\" OR \"had\"");
}

/// AC41-35 (REQ41-Q2): matching is exact on the compiler's tokens, before
/// stemming and without diacritic folding.
#[test]
fn matching_is_exact_token() {
    assert_eq!(
        compile("suggests suggest", &set(StopWordBase::Fathomdb, &["suggest"], &[])),
        "\"suggests\""
    );
    assert_eq!(compile("café cafe", &set(StopWordBase::Fathomdb, &["cafe"], &[])), "\"café\"");
}

fn refusal(config: &QueryStopWords) -> StopWordError {
    config.validate().expect_err("must be refused")
}

fn expect(config: QueryStopWords, field: &str, index: Option<usize>, reason: StopWordRejection) {
    let error = refusal(&config);
    assert_eq!((error.field, error.index, error.reason), (field, index, reason), "{config:?}");
    let shown = error.to_string();
    let named = match index {
        Some(i) => format!("{field}[{i}]"),
        None => field.to_string(),
    };
    assert!(shown.starts_with(&named), "{shown:?} must name {named}");
}

/// AC41-36: each REQ41-Q2 rule refuses, naming the list and index.
#[test]
fn invalid_settings_are_refused_naming_the_field() {
    use StopWordBase::{Fathomdb, None as NoBase};
    use StopWordRejection as R;
    const EXTRA: &str = "query_stop_words.extra";
    const KEEP: &str = "query_stop_words.keep";
    let long = "x".repeat(MAX_STOP_WORD_CHARS + 1);
    expect(setting(Fathomdb, &[""], &[]), EXTRA, Some(0), R::Empty);
    expect(setting(Fathomdb, &["ok", "a b"], &[]), EXTRA, Some(1), R::NotAlphanumeric);
    expect(setting(Fathomdb, &["don't"], &[]), EXTRA, Some(0), R::NotAlphanumeric);
    expect(setting(Fathomdb, &["Had"], &[]), EXTRA, Some(0), R::NotLowercase);
    expect(setting(Fathomdb, &[long.as_str()], &[]), EXTRA, Some(0), R::TooLong);
    expect(setting(Fathomdb, &["had", "had"], &[]), EXTRA, Some(1), R::Repeated);
    expect(setting(Fathomdb, &["the"], &[]), EXTRA, Some(0), R::ExtraAlreadyInBase);
    expect(setting(Fathomdb, &["had"], &["had"]), KEEP, Some(0), R::InExtraAndKeep);
    expect(setting(Fathomdb, &[], &["hadd"]), KEEP, Some(0), R::KeepNotInBase);
    expect(setting(Fathomdb, &[], &["the", "the"]), KEEP, Some(1), R::Repeated);
    expect(setting(Fathomdb, &[], &["The"]), KEEP, Some(0), R::NotLowercase);
    expect(setting(Fathomdb, &[], &[""]), KEEP, Some(0), R::Empty);
    expect(setting(NoBase, &[], &["the"]), KEEP, Some(0), R::KeepNotInBase);

    let many: Vec<String> = (0..=MAX_STOP_WORDS_PER_LIST).map(|i| format!("w{i}")).collect();
    let too_many = QueryStopWords { base: Fathomdb, extra: many.clone(), keep: Vec::new() };
    expect(too_many, EXTRA, None, R::TooMany);
    let keep_many = QueryStopWords { base: NoBase, extra: Vec::new(), keep: many };
    expect(keep_many, KEEP, None, R::TooMany);
}

/// The bounds themselves are accepted.
#[test]
fn list_and_word_bounds_are_inclusive() {
    let at_cap: Vec<String> = (0..MAX_STOP_WORDS_PER_LIST).map(|i| format!("w{i}")).collect();
    let config = QueryStopWords { base: StopWordBase::Fathomdb, extra: at_cap, keep: Vec::new() };
    assert_eq!(config.validate().expect("256 words").words().count(), 63 + 256);
    let longest = "x".repeat(MAX_STOP_WORD_CHARS);
    assert!(set(StopWordBase::None, &[longest.as_str()], &[]).contains(&longest));
    assert!(set(StopWordBase::None, &["café", "d3"], &[]).contains("café"));
}

/// The compiler's contract, written independently of it: lowercase, split on
/// non-alphanumeric, drop empty fragments and words in `stop`, de-duplicate in
/// first-seen order; with none left, OR the raw whitespace tokens as quoted
/// phrases.
fn reference(raw: &str, stop: &HashSet<String>) -> String {
    let mut seen = HashSet::new();
    let content: Vec<String> = raw
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty() && !stop.contains(*t))
        .map(str::to_string)
        .filter(|t| seen.insert(t.clone()))
        .collect();
    if content.is_empty() {
        raw.split_whitespace()
            .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" OR ")
    } else {
        content.iter().map(|t| format!("\"{t}\"")).collect::<Vec<_>>().join(" OR ")
    }
}

proptest! {
    /// AC41-34: with no setting, or `base = fathomdb` and empty lists, every
    /// query compiles exactly as `compile_text_query` and as the contract.
    #[test]
    fn prop_default_set_is_identity(raw in "\\PC{0,40}") {
        let stop: HashSet<String> = STOPWORDS.iter().map(|w| (*w).to_string()).collect();
        let expected = reference(&raw, &stop);
        prop_assert_eq!(compile_text_query(raw.clone()).match_expression, expected.clone());
        prop_assert_eq!(compile(&raw, &StopWordSet::default()), expected.clone());
        prop_assert_eq!(compile(&raw, &set(StopWordBase::Fathomdb, &[], &[])), expected);
    }

    /// AC41-35: any valid setting compiles as the contract over
    /// `(base ∪ extra) \ keep`.
    #[test]
    fn prop_effective_set_follows_the_contract(
        raw in "[a-z ]{0,30}",
        none in any::<bool>(),
        extra in proptest::collection::btree_set("[a-z]{1,3}", 0..6),
        keep in proptest::sample::subsequence(STOPWORDS.to_vec(), 0..4),
    ) {
        let base = if none { StopWordBase::None } else { StopWordBase::Fathomdb };
        let base_words: HashSet<String> = if none {
            HashSet::new()
        } else {
            STOPWORDS.iter().map(|w| (*w).to_string()).collect()
        };
        let extra: Vec<String> = extra.into_iter().filter(|w| !base_words.contains(w)).collect();
        let keep: Vec<String> = if none {
            Vec::new()
        } else {
            keep.iter().map(|w| (*w).to_string()).filter(|w| !extra.contains(w)).collect()
        };
        let mut effective: HashSet<String> = base_words.union(&extra.iter().cloned().collect()).cloned().collect();
        for word in &keep {
            effective.remove(word);
        }
        let config = QueryStopWords { base, extra, keep };
        let validated = config.validate().expect("valid by construction");
        prop_assert_eq!(compile(&raw, &validated), reference(&raw, &effective));
    }
}
