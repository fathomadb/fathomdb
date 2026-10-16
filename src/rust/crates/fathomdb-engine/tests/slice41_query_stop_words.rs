//! Slice 41 (0.8.28) phase 2 unit Q — configurable query stop words at the
//! engine (AC41-36, AC41-37, AC41-38, AC41-40).
//!
//! Real stores, no mocking. The corpus is hand-written so that `had`, `long`
//! and `suggest` decide whether one document matches: with the setting, the
//! document that holds only those words must leave every query-compiling
//! surface. The cross-SDK case loads `src/conformance/query-stop-words-fixtures.json`,
//! which the Python and TypeScript suites also load.

use std::collections::BTreeSet;
use std::path::PathBuf;

use fathomdb_engine::{
    ArtifactRevisionId, EmbedderChoice, Engine, EngineConfig, EngineConfigurationError,
    EngineOpenError, EvidenceSearchRequestV1, GraphExpandRequestV1, GraphReadContextV1,
    GraphSeedV1, InitialState, OpenedEngine, PreparedWrite, ProjectionFts, ProjectionRole,
    ProjectionSpec, ProvenancedNodeV1, QueryStopWords, ReadContextV1, ReadView, SearchFilter,
    SearchResult, SourceId, SourceVersionId, StopWordBase, StopWordRejection, TraversalDirection,
    WriteProvenanceV1, DEFAULT_STOP_WORDS_SHA256,
};
use fathomdb_schema::SQLITE_SUFFIX;
use serde_json::Value;
use tempfile::TempDir;

const FIXTURES: &str = include_str!("../../../../conformance/query-stop-words-fixtures.json");
const H1_SHA256: &str = "4e1a05394b998ef1234cf0ff24a1692b33832f675a6cdadf131cd32c17964d99";
const QUERY: &str = "how long had the dentist suggest flossing";
const FIELD: &str = "title";

fn fixtures() -> Value {
    serde_json::from_str(FIXTURES).expect("fixture JSON")
}

fn h1() -> QueryStopWords {
    QueryStopWords {
        base: StopWordBase::Fathomdb,
        extra: vec!["had".into(), "long".into(), "suggest".into()],
        keep: Vec::new(),
    }
}

fn config(stop_words: Option<QueryStopWords>) -> EngineConfig {
    EngineConfig { query_stop_words: stop_words, ..EngineConfig::default() }
}

fn db(dir: &TempDir, name: &str) -> PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn open(path: &PathBuf, stop_words: Option<QueryStopWords>) -> OpenedEngine {
    Engine::open_with_choice_and_config(path, EmbedderChoice::None, config(stop_words))
        .expect("open")
}

fn logical_ids(result: &SearchResult) -> Vec<String> {
    result.results.iter().map(|hit| hit.id.value.clone()).collect()
}

fn scores(result: &SearchResult) -> Vec<u64> {
    result.results.iter().map(|hit| hit.score.to_bits()).collect()
}

// --- the site corpus ------------------------------------------------------------

const SITE_CORPUS: &[(&str, &str)] = &[
    ("only-stop-words", "had long suggest"),
    ("content", "dentist flossing"),
    ("unrelated", "weather report"),
];

fn build_site_store(dir: &TempDir) -> PathBuf {
    let path = db(dir, "sites");
    let opened = open(&path, None);
    opened
        .engine
        .configure_projections(
            &[ProjectionSpec {
                name: FIELD.into(),
                roles: BTreeSet::from([ProjectionRole::Searchable]),
                fts: Some(ProjectionFts { tokenizer: None }),
                vector: None,
                source: None,
            }],
            &[],
        )
        .expect("configure projections");
    let writes: Vec<PreparedWrite> = SITE_CORPUS
        .iter()
        .map(|(id, title)| {
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "doc".into(),
                body: serde_json::json!({ FIELD: title }).to_string(),
                source_id: SourceId::new(format!("slice41q-{id}")).expect("source id"),
                logical_id: Some((*id).to_string()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new(format!("{id}-r1")).expect("revision"),
                    SourceVersionId::new(format!("{id}-v1")).expect("version"),
                ),
            })
        })
        .collect();
    opened.engine.write(&writes).expect("write corpus");
    opened.engine.drain(30_000).expect("drain");
    opened.engine.close().expect("close");
    path
}

/// The ids each query-compiling surface returns for [`QUERY`].
fn every_site(engine: &Engine) -> Vec<(&'static str, Vec<String>)> {
    let text_only = engine.search_text_only(QUERY).expect("text only");
    let search = engine.search_explained(QUERY, None, 0, false, 0.3, 0).expect("search");
    let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("ctx");
    let frozen = engine.freeze_read_context(&context).expect("freeze");
    let frozen_search =
        engine.search_frozen(QUERY, &frozen, 0, false, 0.3, 0, true, 10).expect("frozen");
    let evidence = engine
        .search_with_evidence(&EvidenceSearchRequestV1 {
            schema_version: 1,
            query: QUERY.into(),
            context: frozen.clone(),
            rerank_depth: 0,
            use_graph_arm: false,
            alpha: 0.3,
            pool_n: 0,
            include_explanation: true,
            limit: 10,
        })
        .expect("evidence");
    let projected =
        engine.search_projected_text(QUERY, FIELD, None, &ReadView::default()).expect("projected");
    let graph = engine
        .graph_expand(&GraphExpandRequestV1 {
            schema_version: 1,
            seed: GraphSeedV1::Query { schema_version: 1, text: QUERY.into(), ranked_limit: 10 },
            direction: TraversalDirection::Outgoing,
            edge_kinds: Vec::new(),
            target_kinds: Vec::new(),
            context: GraphReadContextV1::Current { schema_version: 1, context },
            max_depth: 0,
            result_limit: 10,
            max_work_units: 10,
            include_explanation: false,
            include_evidence: false,
        })
        .expect("graph expand");
    vec![
        ("search_text_only", logical_ids(&text_only)),
        ("search", logical_ids(&search)),
        ("search_frozen", logical_ids(&frozen_search)),
        ("search_with_evidence", logical_ids(&evidence.search_result)),
        ("search_projected_text", logical_ids(&projected)),
        ("graph_expand query seeds", graph.seeds.iter().map(|s| s.logical_id.clone()).collect()),
    ]
}

/// AC41-37: with the defaults every surface finds the stop-word-only document;
/// with `extra = [had, long, suggest]` none does, and the content document
/// stays.
#[test]
fn every_compile_site_honours_the_setting() {
    let dir = TempDir::new().expect("tempdir");
    let path = build_site_store(&dir);

    let default = open(&path, None);
    for (site, ids) in every_site(&default.engine) {
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(sorted, ["content", "only-stop-words"], "default, {site}");
    }
    default.engine.close().expect("close");

    let configured = open(&path, Some(h1()));
    for (site, ids) in every_site(&configured.engine) {
        assert_eq!(ids, ["content"], "configured, {site}");
    }
    configured.engine.close().expect("close");
}

/// AC41-38: the hash is on the explain trace of every explained search path
/// and on the open report: the default constant when unset or `base =
/// fathomdb` with empty lists, the effective list's hash with extras.
#[test]
fn trace_records_the_stop_word_hash() {
    let dir = TempDir::new().expect("tempdir");
    let path = build_site_store(&dir);
    let explicit_default = QueryStopWords::default();
    for (setting, expected) in [
        (None, DEFAULT_STOP_WORDS_SHA256),
        (Some(explicit_default), DEFAULT_STOP_WORDS_SHA256),
        (Some(h1()), H1_SHA256),
    ] {
        let opened = open(&path, setting.clone());
        assert_eq!(opened.report.stop_words_sha256, expected, "open report, {setting:?}");
        assert_eq!(opened.engine.config().query_stop_words, setting, "config echo");
        let engine = &opened.engine;
        let search = engine.search_explained(QUERY, None, 0, false, 0.3, 0).expect("search");
        let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
        let frozen = engine.freeze_read_context(&context).expect("freeze");
        let frozen_search =
            engine.search_frozen(QUERY, &frozen, 0, false, 0.3, 0, true, 10).expect("frozen");
        let evidence = engine
            .search_with_evidence(&EvidenceSearchRequestV1 {
                schema_version: 1,
                query: QUERY.into(),
                context: frozen,
                rerank_depth: 0,
                use_graph_arm: false,
                alpha: 0.3,
                pool_n: 0,
                include_explanation: true,
                limit: 10,
            })
            .expect("evidence");
        for (path_name, result) in [
            ("search_explained", &search),
            ("search_frozen", &frozen_search),
            ("search_with_evidence", &evidence.search_result),
        ] {
            let trace = &result.explanation.as_ref().expect("explanation").trace;
            assert_eq!(trace.stop_words_sha256, expected, "{path_name}, {setting:?}");
        }
        opened.engine.close().expect("close");
    }
}

/// AC41-36: a refused setting fails the open with a typed, `Copy` error that
/// names the list and index, before any store is created.
#[test]
fn invalid_settings_are_refused_at_open() {
    let dir = TempDir::new().expect("tempdir");
    let path = db(&dir, "refused");
    let bad =
        QueryStopWords { base: StopWordBase::None, extra: Vec::new(), keep: vec!["the".into()] };
    let error = Engine::open_with_choice_and_config(&path, EmbedderChoice::None, config(Some(bad)))
        .expect_err("refused");
    let EngineOpenError::EngineConfiguration(inner) = error else {
        panic!("expected an engine configuration error, got {error:?}");
    };
    let copied = inner;
    assert_eq!(
        copied,
        EngineConfigurationError::QueryStopWords {
            field: "query_stop_words.keep",
            index: Some(0),
            reason: StopWordRejection::KeepNotInBase,
        }
    );
    assert!(inner.to_string().contains("query_stop_words.keep[0]"), "{inner}");
    assert!(!path.exists(), "a refused setting must not create the store");

    for case in fixtures()["invalid_settings"].as_array().expect("invalid_settings") {
        let setting = setting_from(&case["setting"]);
        let error =
            Engine::open_with_choice_and_config(&path, EmbedderChoice::None, config(Some(setting)))
                .expect_err("refused");
        let needle = case["message_contains"].as_str().expect("needle");
        assert!(
            matches!(error, EngineOpenError::EngineConfiguration(_)),
            "{}: {error:?}",
            case["name"]
        );
        assert!(error.to_string().contains(needle), "{}: {error}", case["name"]);
    }
}

fn setting_from(v: &Value) -> QueryStopWords {
    let words = |key: &str| -> Vec<String> {
        v[key]
            .as_array()
            .map(|a| a.iter().map(|w| w.as_str().expect("word").to_string()).collect())
            .unwrap_or_default()
    };
    QueryStopWords {
        base: match v["base"].as_str() {
            None | Some("fathomdb") => StopWordBase::Fathomdb,
            Some("none") => StopWordBase::None,
            Some(other) => panic!("fixture base {other}"),
        },
        extra: words("extra"),
        keep: words("keep"),
    }
}

/// AC41-40 (Rust engine): the shared fixture's hand-derived ids, and score bits
/// equal to the default engine queried with the words stripped.
#[test]
fn shared_fixture_ranks_as_derived_and_as_the_harness_transform() {
    let f = fixtures();
    let dir = TempDir::new().expect("tempdir");
    let path = db(&dir, "fixture");
    let writes: Vec<PreparedWrite> = f["corpus"]
        .as_array()
        .expect("corpus")
        .iter()
        .map(|doc| PreparedWrite::Node {
            kind: "doc".into(),
            body: doc["body"].as_str().expect("body").into(),
            source_id: SourceId::new("slice41q-fixture").expect("source id"),
            logical_id: Some(doc["logical_id"].as_str().expect("id").into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect();
    let default = open(&path, None);
    default.engine.write(&writes).expect("write");
    default.engine.drain(30_000).expect("drain");
    assert_eq!(default.report.stop_words_sha256, f["default_sha256"]);
    let mut stripped = Vec::new();
    for q in f["queries"].as_array().expect("queries") {
        let ids =
            logical_ids(&default.engine.search_text_only(q["query"].as_str().unwrap()).unwrap());
        assert_eq!(
            ids,
            q["default_ids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect::<Vec<_>>()
        );
        stripped
            .push(default.engine.search_text_only(q["stripped_query"].as_str().unwrap()).unwrap());
    }
    default.engine.close().expect("close");

    let configured = open(&path, Some(setting_from(&f["setting"])));
    assert_eq!(configured.report.stop_words_sha256, f["setting_sha256"]);
    for (q, transform) in f["queries"].as_array().unwrap().iter().zip(&stripped) {
        let result = configured.engine.search_text_only(q["query"].as_str().unwrap()).unwrap();
        let expected: Vec<&str> =
            q["setting_ids"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(logical_ids(&result), expected, "{}", q["query"]);
        assert_eq!(logical_ids(transform), expected, "transform, {}", q["query"]);
        assert_eq!(scores(&result), scores(transform), "score bits, {}", q["query"]);
    }
    configured.engine.close().expect("close");
}
