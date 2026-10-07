//! Slice 132 AC27-132D — SDK behaviour on a real database.
//!
//! Expected values are the documented Python/TypeScript SDK behaviour
//! (`dev/interfaces/{python,typescript,rust-sdk}.md`), not engine internals.

use std::sync::{Arc, Mutex};

use fathomdb_sdk::{
    admin, graph, read, rerank, Engine, EngineConfig, EngineError, Error, ErrorKind,
    FrozenSearchOptions, InitialState, LifecycleState, ListOptions, NeighborsOptions, OpenOptions,
    Predicate, PreparedWrite, ProjectedTextSearchOptions, ReadContextV1, ReadView, RerankOptions,
    RerankPassage, SearchExpandOptions, SearchFilter, SearchOptions, SourceId, Subscriber,
    SubscriberEvent, TextSearchOptions, TraversalDirection,
};

fn open_engine() -> (tempfile::TempDir, Engine) {
    let dir = tempfile::tempdir().expect("tempdir");
    let engine = Engine::open(dir.path().join("sdk.sqlite"), OpenOptions::default()).expect("open");
    (dir, engine)
}

fn node(logical_id: Option<&str>, kind: &str, body: &str, source: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: kind.to_string(),
        body: body.to_string(),
        source_id: SourceId::new(source).expect("source id"),
        logical_id: logical_id.map(str::to_string),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn edge(from: &str, to: &str) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "links".to_string(),
        from: from.to_string(),
        to: to.to_string(),
        source_id: SourceId::new("src-edges").expect("source id"),
        logical_id: None,
        body: None,
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}

fn seed(engine: &Engine) {
    engine
        .write(&[
            node(Some("note:1"), "note", "alpha river crossing", "src-a"),
            node(Some("note:2"), "note", "beta mountain pass", "src-a"),
            node(Some("note:3"), "note", "gamma alpha delta", "src-b"),
            edge("note:1", "note:2"),
        ])
        .expect("seed write");
    engine.drain(10_000).expect("drain");
}

fn kind_of<T: std::fmt::Debug>(result: fathomdb_sdk::Result<T>) -> ErrorKind {
    result.expect_err("expected an error").kind()
}

#[test]
fn open_keeps_report_and_requested_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = EngineConfig { slow_threshold_ms: Some(250), ..EngineConfig::default() };
    let engine = Engine::open(
        dir.path().join("sdk.sqlite"),
        OpenOptions { config: config.clone(), use_default_embedder: false },
    )
    .expect("open");
    assert_eq!(engine.config(), &config);
    assert!(
        engine.open_report().schema_version_after >= engine.open_report().schema_version_before
    );
    engine.close().expect("close");
}

#[test]
fn invalid_engine_config_is_invalid_argument() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = EngineConfig { embedder_pool_size: Some(0), ..EngineConfig::default() };
    let result = Engine::open(
        dir.path().join("sdk.sqlite"),
        OpenOptions { config, use_default_embedder: false },
    );
    assert_eq!(kind_of(result), ErrorKind::InvalidArgument);
}

#[cfg(not(feature = "default-embedder"))]
#[test]
fn default_embedder_without_feature_is_a_typed_open_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let result = Engine::open(
        dir.path().join("sdk.sqlite"),
        OpenOptions { config: EngineConfig::default(), use_default_embedder: true },
    );
    assert_eq!(kind_of(result), ErrorKind::Embedder);
}

#[test]
fn close_is_idempotent_and_later_calls_report_closing() {
    let (_dir, engine) = open_engine();
    engine.close().expect("first close");
    engine.close().expect("second close");
    assert_eq!(kind_of(engine.write(&[node(None, "note", "late", "src-a")])), ErrorKind::Closing);

    struct Quiet;
    impl Subscriber for Quiet {
        fn on_event(&self, _event: &SubscriberEvent) {}
    }
    assert_eq!(kind_of(engine.attach_subscriber(Arc::new(Quiet))), ErrorKind::Closing);
}

#[test]
fn attached_subscriber_receives_events_until_replaced() {
    struct Counting(Arc<Mutex<usize>>);
    impl Subscriber for Counting {
        fn on_event(&self, _event: &SubscriberEvent) {
            *self.0.lock().unwrap() += 1;
        }
    }
    let (_dir, engine) = open_engine();
    let first = Arc::new(Mutex::new(0));
    engine.attach_subscriber(Arc::new(Counting(Arc::clone(&first)))).expect("attach");
    let second = Arc::new(Mutex::new(0));
    engine.attach_subscriber(Arc::new(Counting(Arc::clone(&second)))).expect("replace");
    let before = *first.lock().unwrap();
    seed(&engine);
    assert_eq!(*first.lock().unwrap(), before, "replaced subscriber must be detached");
    assert!(*second.lock().unwrap() > 0, "current subscriber receives events");
    engine.close().expect("close");
}

#[test]
fn read_namespace_round_trips_written_nodes() {
    let (_dir, engine) = open_engine();
    seed(&engine);

    let got = read::get(&engine, "note:1", None).expect("get").expect("present");
    assert_eq!((got.kind.as_str(), got.body.as_str()), ("note", "alpha river crossing"));
    assert!(read::get(&engine, "note:missing", None).expect("get").is_none());

    let many = read::get_many(
        &engine,
        &["note:2".to_string(), "note:missing".to_string()],
        Some(&ReadView::default()),
    )
    .expect("get_many");
    assert_eq!(many.len(), 2);
    assert_eq!(many[0].as_ref().map(|n| n.logical_id.as_str()), Some("note:2"));
    assert!(many[1].is_none());

    let listed = read::list(&engine, "note", ListOptions::default()).expect("list");
    assert_eq!(listed.len(), 3);
    let limited = read::list(&engine, "note", ListOptions { limit: 1, ..ListOptions::default() })
        .expect("list limit");
    assert_eq!(limited.len(), 1);

    let crossings = read::crossed_boundary_since(&engine, 0, None).expect("crossings");
    let _ = crossings;
    engine.close().expect("close");
}

#[test]
fn read_list_rejects_predicates_with_filter() {
    let (_dir, engine) = open_engine();
    let options = ListOptions {
        predicates: vec![Predicate::JsonPathEq {
            path: "$.x".to_string(),
            value: fathomdb_sdk::ScalarValue::Integer(1),
        }],
        filter: Some(fathomdb_sdk::Filter::default()),
        ..ListOptions::default()
    };
    assert_eq!(kind_of(read::list(&engine, "note", options)), ErrorKind::InvalidArgument);
    engine.close().expect("close");
}

#[test]
fn search_defaults_and_argument_validation() {
    let (_dir, engine) = open_engine();
    seed(&engine);

    let result = engine.search("alpha", SearchOptions::default()).expect("search");
    assert!(!result.results.is_empty());
    assert!(result.results.len() <= 10);

    for limit in [0, 101] {
        let options = SearchOptions { limit, ..SearchOptions::default() };
        assert_eq!(kind_of(engine.search("alpha", options)), ErrorKind::InvalidArgument);
    }
    let options = SearchOptions { alpha: Some(f64::NAN), ..SearchOptions::default() };
    assert_eq!(kind_of(engine.search("alpha", options)), ErrorKind::InvalidArgument);

    let text = engine.search_text_only("alpha", TextSearchOptions::default()).expect("text");
    assert!(!text.results.is_empty());
    let options = TextSearchOptions { limit: 0, ..TextSearchOptions::default() };
    assert_eq!(kind_of(engine.search_text_only("alpha", options)), ErrorKind::InvalidArgument);
    engine.close().expect("close");
}

#[test]
fn search_accepts_the_unified_filter() {
    let (_dir, engine) = open_engine();
    seed(&engine);
    let filter = fathomdb_sdk::Filter::default();
    let options = SearchOptions { filter: Some(filter.into()), ..SearchOptions::default() };
    assert!(!engine.search("alpha", options).expect("search").results.is_empty());
    engine.close().expect("close");
}

#[test]
fn projected_text_search_on_unknown_projection_is_an_engine_error() {
    let (_dir, engine) = open_engine();
    let result = engine.search_projected_text(
        "alpha",
        "no_such_projection",
        ProjectedTextSearchOptions::default(),
    );
    // The core refuses it; the SDK adds no check of its own here.
    assert!(matches!(result, Err(Error::Engine(_))), "{result:?}");
    engine.close().expect("close");
}

#[test]
fn frozen_search_uses_shared_defaults() {
    let (_dir, engine) = open_engine();
    seed(&engine);
    let context = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("context"),
        )
        .expect("freeze");
    let frozen = engine
        .search_frozen("alpha", &context, FrozenSearchOptions::default())
        .expect("frozen search");
    let live = engine.search("alpha", SearchOptions::default()).expect("search");
    let ids =
        |r: &fathomdb_sdk::SearchResult| r.results.iter().map(|h| h.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(&frozen), ids(&live));

    let options = FrozenSearchOptions { limit: 0, ..FrozenSearchOptions::default() };
    assert_eq!(
        kind_of(engine.search_frozen("alpha", &context, options)),
        ErrorKind::InvalidArgument
    );

    let expanded = engine
        .search_expand_frozen("alpha", &context, 1, SearchExpandOptions::default())
        .expect("expand frozen");
    assert!(!expanded.search_hits.is_empty());
    assert!(expanded.expanded.iter().any(|(node, _)| node.logical_id == "note:2"));
    engine.close().expect("close");
}

#[test]
fn graph_namespace_traverses_edges() {
    let (_dir, engine) = open_engine();
    seed(&engine);

    let both = graph::neighbors(&engine, "note:1", 1, NeighborsOptions::default()).expect("both");
    assert!(both.iter().any(|n| n.logical_id == "note:2"));
    let incoming = graph::neighbors(
        &engine,
        "note:1",
        1,
        NeighborsOptions { direction: TraversalDirection::Incoming, view: None },
    )
    .expect("incoming");
    assert!(incoming.iter().all(|n| n.logical_id != "note:2"));

    let expanded = graph::search_expand(&engine, "alpha", 1, None, SearchExpandOptions::default())
        .expect("expand");
    let _ = expanded;

    let mut filter = SearchFilter::default();
    filter.attributes.push(("topic".to_string(), "x".to_string()));
    assert_eq!(
        kind_of(graph::search_expand(
            &engine,
            "alpha",
            1,
            Some(filter),
            SearchExpandOptions::default()
        )),
        ErrorKind::InvalidArgument
    );
    engine.close().expect("close");
}

#[test]
fn admin_configure_writes_an_admin_schema() {
    let (_dir, engine) = open_engine();
    let receipt = admin::configure(&engine, "settings", r#"{"type":"object"}"#).expect("configure");
    assert!(receipt.cursor > 0);
    engine.close().expect("close");
}

#[test]
fn lifecycle_and_erasure_commands() {
    let (_dir, engine) = open_engine();
    seed(&engine);
    engine.transition("note:2", LifecycleState::Deleted, Some("test")).expect("transition");
    assert!(read::get(&engine, "note:2", None).expect("get").is_none());
    engine.purge("note:2").expect("purge");

    let report = engine.erase_source("src-b").expect("erase");
    let _ = report;
    assert!(read::get(&engine, "note:3", None).expect("get").is_none());
    assert!(read::get(&engine, "note:1", None).expect("get").is_some());

    let error =
        engine.transition("h:not-lifecycle", LifecycleState::Deleted, None).expect_err("refused");
    assert!(matches!(
        error.kind(),
        ErrorKind::NotLifecycleAddressable | ErrorKind::InvalidArgument
    ));
    engine.close().expect("close");
}

#[test]
fn embedded_nul_strings_are_write_validation_errors() {
    let (_dir, engine) = open_engine();
    let bad = node(Some("note:nul"), "note", "a\0b", "src-a");
    assert_eq!(kind_of(engine.write(&[bad])), ErrorKind::WriteValidation);
    assert!(read::get(&engine, "note:nul", None).expect("get").is_none(), "no row written");

    assert_eq!(
        kind_of(engine.search("a\0b", SearchOptions::default())),
        ErrorKind::WriteValidation
    );
    assert_eq!(kind_of(read::get(&engine, "note\0x", None)), ErrorKind::WriteValidation);
    assert_eq!(kind_of(admin::configure(&engine, "a\0b", "{}")), ErrorKind::WriteValidation);
    engine.close().expect("close");
}

#[test]
fn embedded_nul_in_source_id_is_preserved() {
    let (_dir, engine) = open_engine();
    engine.write(&[node(Some("note:src"), "note", "kept", "tenant\0a")]).expect("write");
    assert!(read::get(&engine, "note:src", None).expect("get").is_some());
    engine.close().expect("close");
}

#[test]
fn rerank_identity_validation_and_errors() {
    let passages = vec![
        RerankPassage { id: 1, body: "first".to_string(), score: 0.9 },
        RerankPassage { id: 2, body: "second".to_string(), score: 0.5 },
    ];
    let identity = rerank("query", &passages, 0, RerankOptions::default()).expect("identity");
    assert_eq!(identity.iter().map(|r| r.id).collect::<Vec<_>>(), vec![1, 2]);
    assert!(rerank("query", &[], 5, RerankOptions::default()).expect("empty").is_empty());

    let options = RerankOptions { alpha: Some(f64::INFINITY), pool_n: None };
    assert_eq!(kind_of(rerank("query", &passages, 1, options)), ErrorKind::InvalidArgument);

    let bad = vec![RerankPassage { id: 1, body: "x".to_string(), score: f64::NAN }];
    assert_eq!(
        kind_of(rerank("query", &bad, 1, RerankOptions::default())),
        ErrorKind::WriteValidation
    );
    assert_eq!(
        kind_of(rerank("q\0", &passages, 0, RerankOptions::default())),
        ErrorKind::WriteValidation
    );
}

#[cfg(not(feature = "default-embedder"))]
#[test]
fn cls_embedding_without_feature_is_not_configured() {
    assert_eq!(kind_of(fathomdb_sdk::embed_batch_cls(&[])), ErrorKind::EmbedderNotConfigured);
    assert_eq!(kind_of(fathomdb_sdk::embed_batch_cls(&["text"])), ErrorKind::EmbedderNotConfigured);
}

#[test]
fn engine_embed_without_embedder_is_typed() {
    let (_dir, engine) = open_engine();
    let error = engine.embed("text").expect_err("no embedder");
    assert!(matches!(error.kind(), ErrorKind::EmbedderNotConfigured | ErrorKind::Embedder));
    engine.close().expect("close");
}

/// Every non-operator core variant maps to the Python/TypeScript class of the
/// same name; the binding mappings are in `fathomdb-py/src/errors.rs`.
#[test]
fn core_error_variants_map_to_shared_kinds() {
    let cases: Vec<(EngineError, ErrorKind)> = vec![
        (EngineError::Storage, ErrorKind::Storage),
        (EngineError::Projection, ErrorKind::Projection),
        (EngineError::Vector, ErrorKind::Vector),
        (EngineError::Embedder, ErrorKind::Embedder),
        (EngineError::EmbedderNotConfigured, ErrorKind::EmbedderNotConfigured),
        (EngineError::KindNotVectorIndexed, ErrorKind::KindNotVectorIndexed),
        (
            EngineError::EmbedderDimensionMismatch { expected: 1, actual: 2 },
            ErrorKind::EmbedderDimensionMismatch,
        ),
        (EngineError::Scheduler, ErrorKind::Scheduler),
        (EngineError::OpStore, ErrorKind::OpStore),
        (EngineError::WriteValidation, ErrorKind::WriteValidation),
        (EngineError::SchemaValidation, ErrorKind::SchemaValidation),
        (EngineError::Overloaded, ErrorKind::Overloaded),
        (EngineError::Closing, ErrorKind::Closing),
        (EngineError::Extractor, ErrorKind::Extractor),
        (EngineError::Consolidator, ErrorKind::Consolidator),
        (EngineError::InvalidFilter { reason: "r".into() }, ErrorKind::InvalidFilter),
        (EngineError::InvalidArgument { msg: "m".into() }, ErrorKind::InvalidArgument),
        (
            EngineError::VectorEquivalenceMismatch { reason: "r".into() },
            ErrorKind::VectorEquivalenceMismatch,
        ),
        (
            EngineError::IllegalTransition {
                from_state: LifecycleState::Active,
                to_state: LifecycleState::Pending,
                legal: vec![],
            },
            ErrorKind::IllegalTransition,
        ),
        (
            EngineError::NotLifecycleAddressable { id_space: fathomdb_sdk::IdSpaceKind::Content },
            ErrorKind::NotLifecycleAddressable,
        ),
        (
            EngineError::ErasureIncomplete { stage: "s".into(), detail: "d".into() },
            ErrorKind::ErasureIncomplete,
        ),
        (
            EngineError::ProjectionDestructive { name: "n".into(), delta: "d".into() },
            ErrorKind::ProjectionDestructive,
        ),
    ];
    for (core, kind) in cases {
        let error: Error = core.clone().into();
        assert_eq!(error.kind(), kind, "{core:?}");
    }
}

#[test]
fn wrapped_core_errors_map_through_real_operations() {
    let (_dir, engine) = open_engine();
    let trace_error = fathomdb_sdk::DependencyTraceRequestV1::new(
        "",
        fathomdb_sdk::DependencyTraceDirectionV1::ToSource,
        engine
            .freeze_read_context(
                &ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("context"),
            )
            .expect("freeze"),
    )
    .expect_err("invalid root");
    let error: Error = EngineError::from(trace_error).into();
    assert_eq!(error.kind(), ErrorKind::DependencyTrace);
    engine.close().expect("close");
}

#[test]
fn embedded_nul_in_predicates_filters_and_cursors_is_write_validation() {
    let (_dir, engine) = open_engine();
    seed(&engine);
    let text_predicate = Predicate::JsonPathEq {
        path: "$.title".to_string(),
        value: fathomdb_sdk::ScalarValue::Text("a\0b".to_string()),
    };
    let options =
        ListOptions { predicates: vec![text_predicate.clone()], ..ListOptions::default() };
    assert_eq!(kind_of(read::list(&engine, "note", options)), ErrorKind::WriteValidation);

    let path_predicate = Predicate::JsonPathEq {
        path: "$.ti\0tle".to_string(),
        value: fathomdb_sdk::ScalarValue::Integer(1),
    };
    let options = ListOptions { predicates: vec![path_predicate], ..ListOptions::default() };
    assert_eq!(kind_of(read::list(&engine, "note", options)), ErrorKind::WriteValidation);

    let filter =
        fathomdb_sdk::Filter { terms: vec![fathomdb_sdk::FilterTerm::Json(text_predicate)] };
    let options = ListOptions { filter: Some(filter), ..ListOptions::default() };
    assert_eq!(kind_of(read::list(&engine, "note", options)), ErrorKind::WriteValidation);

    let context = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("context"),
        )
        .expect("freeze");
    let page = fathomdb_sdk::PageRequestV1 {
        schema_version: 1,
        limit: 10,
        cursor: Some(fathomdb_sdk::PageCursor("c\0".to_string())),
    };
    assert_eq!(
        kind_of(read::canonical_page(&engine, "note", &context, &page)),
        ErrorKind::WriteValidation
    );
    assert_eq!(
        kind_of(read::operational_state_page(&engine, "settings", &context, &page)),
        ErrorKind::WriteValidation
    );
    engine.close().expect("close");
}

/// Python and TypeScript check `search_limit` before the string guard.
#[test]
fn graph_search_expand_checks_search_limit_first() {
    let (_dir, engine) = open_engine();
    let options = SearchExpandOptions { search_limit: 0 };
    assert_eq!(
        kind_of(graph::search_expand(&engine, "a\0b", 1, None, options)),
        ErrorKind::InvalidArgument
    );
    engine.close().expect("close");
}
