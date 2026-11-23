use fathomdb_sdk::*;
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed, TestRunner};
use serde_json::Value;
use std::time::{Duration, Instant};
fn strings(value: &Value) -> Vec<&str> {
    value.as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect()
}

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../../conformance/slice55-behavior.json")).unwrap()
}
fn seed(engine: &Engine) {
    let f = fixture();
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "group".into(),
                roles: [ProjectionRole::Filterable].into(),
                fts: None,
                vector: None,
                source: None,
                entity_class: PropertyEntityClass::Node,
            }],
            &[],
        )
        .unwrap();
    let writes = f["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| {
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "doc".into(),
                body: f["body"].as_str().unwrap().into(),
                source_id: SourceId::new(n["source"].as_str().unwrap()).unwrap(),
                logical_id: Some(n["id"].as_str().unwrap().into()),
                state: InitialState::Active,
                reason: None,
                valid_from: Some(1_000_000_000),
                valid_until: if n["id"] == "cohesion:a" { Some(2_000_000_001) } else { None },
                properties: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new(n["revision"].as_str().unwrap()).unwrap(),
                    SourceVersionId::new(n["version"].as_str().unwrap()).unwrap(),
                ),
            })
        })
        .collect::<Vec<_>>();
    engine.write(&writes).unwrap();
    engine.drain(10_000).unwrap();
}
fn engine() -> (tempfile::TempDir, Engine) {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().join("shared.sqlite"), OpenOptions::default()).unwrap();
    seed(&engine);
    (dir, engine)
}
fn frozen(engine: &Engine) -> FrozenReadContextV1 {
    engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap()
}
fn verify(engine: &Engine) {
    let f = fixture();
    let result = engine
        .search_text_only(f["query"].as_str().unwrap(), TextSearchOptions::default())
        .unwrap();
    assert_eq!(
        result.results.iter().map(|h| h.id.value.as_str()).collect::<Vec<_>>(),
        strings(&f["currentIds"])
    );
    for (hit, node) in result.results.iter().zip(f["nodes"].as_array().unwrap()) {
        assert_eq!(hit.body, f["body"]);
        assert_eq!(hit.source_id.as_deref(), node["source"].as_str());
    }
    let mut filter = SearchFilter::default();
    filter.attributes = vec![("group".into(), "kept".into())];
    let mut options = TextSearchOptions::default();
    options.filter = Some(filter);
    let result = engine.search_text_only("wombat", options).unwrap();
    assert_eq!(result.results.len(), 2);
    let mut negative = TextSearchOptions::default();
    let mut excluded = SearchFilter::default();
    excluded.attributes = vec![("group".into(), "absent".into())];
    negative.filter = Some(excluded);
    assert!(engine.search_text_only("wombat", negative).unwrap().results.is_empty());
    let mut expired = TextSearchOptions::default();
    expired.view.valid_as_of = Some(f["instant"].as_i64().unwrap() + 1);
    assert_eq!(
        engine
            .search_text_only("wombat", expired)
            .unwrap()
            .results
            .iter()
            .map(|h| h.id.value.as_str())
            .collect::<Vec<_>>(),
        strings(&f["expiredIds"])
    );
    let context = frozen(engine);
    let first = read::canonical_page(
        engine,
        "doc",
        &context,
        &PageRequestV1 { schema_version: 1, limit: 1, cursor: None },
    )
    .unwrap();
    assert_eq!(first.items[0].logical_id.as_str(), "cohesion:a");
    let cursor = first.next_cursor.expect("real continuation");
    let second = read::canonical_page(
        engine,
        "doc",
        &context,
        &PageRequestV1 { schema_version: 1, limit: 1, cursor: Some(cursor.clone()) },
    )
    .unwrap();
    assert_eq!(second.items[0].logical_id.as_str(), "cohesion:b");
    assert!(second.next_cursor.is_none());
    let mut tampered = cursor.0.into_bytes();
    let last = tampered.last_mut().unwrap();
    *last = if *last == b'a' { b'b' } else { b'a' };
    let error = read::canonical_page(
        engine,
        "doc",
        &context,
        &PageRequestV1 {
            schema_version: 1,
            limit: 1,
            cursor: Some(PageCursor(String::from_utf8(tampered).unwrap())),
        },
    )
    .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Page);
    match error {
        Error::Engine(EngineError::Page(error)) => {
            assert_eq!(error.reason, PageErrorReason::CursorAuthenticationFailed)
        }
        other => panic!("{other:?}"),
    };
    assert_eq!(
        engine
            .search_text_only(f["invalidQuery"].as_str().unwrap(), TextSearchOptions::default())
            .unwrap_err()
            .kind(),
        ErrorKind::WriteValidation
    );
    let result = engine
        .search_with_evidence(&EvidenceSearchRequestV1::new("wombat".into(), context.clone()))
        .unwrap();
    assert_eq!(
        result.evidence.iter().map(|e| e.artifact_revision_id.as_str()).collect::<Vec<_>>(),
        f["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["revision"].as_str().unwrap())
            .collect::<Vec<_>>()
    );
    for (e, expected) in result.evidence.into_iter().zip(f["nodes"].as_array().unwrap()) {
        let resolved = engine
            .resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: e.evidence_ref,
                context: context.clone(),
            })
            .unwrap();
        assert_eq!(resolved.canonical_source_body, f["body"]);
        assert_eq!(resolved.artifact_revision_id, expected["revision"]);
        assert_eq!(resolved.source_revision_id, expected["revision"]);
        assert_eq!(resolved.source_id, expected["source"]);
    }
}
#[test]
fn shared_fixture_has_exact_identity_provenance_cursor_errors_and_persistence() {
    let (dir, engine) = engine();
    verify(&engine);
    engine.close().unwrap();
    drop(engine);
    let reopened = Engine::open(dir.path().join("shared.sqlite"), OpenOptions::default()).unwrap();
    verify(&reopened);
    reopened.close().unwrap();
}
fn run_inputs(
    target: &str,
    body: impl Fn(String) -> std::result::Result<(), proptest::test_runner::TestCaseError>,
) {
    let started = Instant::now();
    let mut count = 0;
    for raw in ["", "wombat", "猫 café", "\0", "\" OR * ( )", "fdbpg1.invalid"] {
        body(raw.into()).unwrap();
        count += 1;
    }
    let mut runner = TestRunner::new(Config {
        cases: 128,
        rng_seed: RngSeed::Fixed(0x55_0830),
        failure_persistence: Some(Box::new(proptest::test_runner::FileFailurePersistence::Direct(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/slice55_shared_behavior.proptest-regressions"
            ),
        ))),
        source_file: Some(file!()),
        test_name: Some("slice55_product_input_smoke"),
        ..Config::default()
    });
    runner
        .run(&proptest::collection::vec(any::<char>(), 0..64), |chars| {
            let raw = chars.into_iter().collect::<String>();
            prop_assert!(raw.len() <= 256);
            body(raw)
        })
        .unwrap();
    count += 128;
    assert!(started.elapsed() < Duration::from_secs(60), "{target} watchdog");
    eprintln!("SLICE55_PRODUCT_FUZZ target={target} method=proptest seed=0x550830 inputs={count} arbitrary_max_bytes=256 authenticated_envelope_max_bytes=2048 elapsed_ms={}",started.elapsed().as_millis());
}
#[test]
fn bounded_product_query_admission_fuzz_has_typed_outcomes_and_no_state_change() {
    let (_dir, engine) = engine();
    let before = read::list(&engine, "doc", ListOptions::default()).unwrap();
    run_inputs("sdk-text-query", |query| {
        let result = engine.search_text_only(&query, TextSearchOptions::default());
        if query.trim().is_empty() || query.contains('\0') {
            prop_assert_eq!(result.unwrap_err().kind(), ErrorKind::WriteValidation);
        } else {
            let result = result.unwrap();
            for hit in &result.results {
                prop_assert!(strings(&fixture()["currentIds"]).contains(&hit.id.value.as_str()));
            }
            if query == "wombat" {
                prop_assert_eq!(
                    result.results.iter().map(|h| h.id.value.clone()).collect::<Vec<_>>(),
                    vec!["cohesion:a".to_owned(), "cohesion:b".to_owned()]
                );
            }
        }
        prop_assert_eq!(
            read::list(&engine, "doc", ListOptions::default()).unwrap(),
            before.clone()
        );
        Ok(())
    });
    engine.close().unwrap();
}
#[test]
fn bounded_product_cursor_admission_fuzz_has_positive_continuation_and_no_state_change() {
    let (_dir, engine) = engine();
    verify(&engine);
    let context = frozen(&engine);
    let before = read::list(&engine, "doc", ListOptions::default()).unwrap();
    let first = read::canonical_page(
        &engine,
        "doc",
        &context,
        &PageRequestV1 { schema_version: 1, limit: 1, cursor: None },
    )
    .unwrap();
    let minted = first.next_cursor.unwrap().0;
    assert!(minted.len() <= 2048);
    run_inputs("sdk-page-cursor", |cursor| {
        let result = read::canonical_page(
            &engine,
            "doc",
            &context,
            &PageRequestV1 {
                schema_version: 1,
                limit: 1,
                cursor: Some(PageCursor(cursor.clone())),
            },
        );
        let error = result.expect_err("arbitrary strings cannot authenticate as a minted cursor");
        if cursor.contains('\0') {
            prop_assert_eq!(error.kind(), ErrorKind::WriteValidation);
        } else {
            match error {
                Error::Engine(EngineError::Page(error)) => {
                    prop_assert_eq!(error.reason, PageErrorReason::CursorMalformed)
                }
                other => panic!("{other:?}"),
            };
        }
        let mut changed = minted.clone().into_bytes();
        let end = changed.iter().rposition(|b| *b == b'.').unwrap();
        let index =
            7 + cursor.bytes().fold(0_usize, |sum, b| sum.wrapping_add(b as usize)) % (end - 7);
        changed[index] = if changed[index] == b'a' { b'b' } else { b'a' };
        let error = read::canonical_page(
            &engine,
            "doc",
            &context,
            &PageRequestV1 {
                schema_version: 1,
                limit: 1,
                cursor: Some(PageCursor(String::from_utf8(changed).unwrap())),
            },
        )
        .unwrap_err();
        match error {
            Error::Engine(EngineError::Page(error)) => {
                prop_assert_eq!(error.reason, PageErrorReason::CursorAuthenticationFailed)
            }
            other => panic!("{other:?}"),
        };
        prop_assert_eq!(
            read::list(&engine, "doc", ListOptions::default()).unwrap(),
            before.clone()
        );
        Ok(())
    });
    engine.close().unwrap();
}
