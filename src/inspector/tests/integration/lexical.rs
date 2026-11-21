use super::dispatch;
use crate::{
    fixtures::{database, node},
    policy::Policy,
    session::Session,
    storage::Reader,
};
use fathomdb_engine::{EmbedderChoice, Engine, ProjectionFts, ProjectionRole, ProjectionSpec};
use serde_json::{json, Value};

fn corpus() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .configure_projections(
            &[ProjectionSpec {
                name: "biography".into(),
                roles: std::collections::BTreeSet::from([
                    ProjectionRole::Searchable,
                    ProjectionRole::Filterable,
                ]),
                fts: Some(ProjectionFts { tokenizer: Some("unicode61".into()) }),
                vector: None,
                source: Some(vec!["profile".into(), "biography".into()]),
            }],
            &[],
        )
        .unwrap();
    for (id, text, biography) in [
        ("lexical-a", "Café runners <b>run</b>", "orchid orchid"),
        ("lexical-b", "runner blue", "orchid"),
        ("lexical-twin", "runner blue", "orchid"),
        ("lexical-c", "stone", "stone"),
        ("lexical-d", "pebble", "pebble"),
    ] {
        opened
            .engine
            .write(&[node(
                id,
                &json!({"text":text,"profile":{"biography":biography},"status":"open"}).to_string(),
            )])
            .unwrap();
    }
    opened.engine.close().unwrap();
    (dir, path)
}
fn handle(reader: &mut Reader, identity: &str) -> String {
    dispatch(
        reader,
        json!({"op":"page","class":"node","after":"0","limit":10,"filter":{"identity":identity}}),
    )
    .unwrap()["items"][0]["handle"]
        .as_str()
        .unwrap()
        .into()
}
fn query(route: &str, text: &str) -> Value {
    json!({"op":"lexical","route":route,"query":text,"property":null,"filter":{},"view":{},"limit":10})
}

#[test]
fn indexed_text_is_actual_nested_materialization_with_exact_owner_and_effective_tokenizer() {
    let (_dir, path) = corpus();
    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let selected = handle(&mut reader, "lexical-a");
    let result =
        dispatch(&mut reader, json!({"op":"indexed","handle":selected,"offset":"0","limit":4096}));
    assert!(result.is_ok(), "indexed text operation must succeed: {result:?}");
    let data = result.unwrap();
    let items = data["items"].as_array().unwrap();
    let body = items.iter().find(|v| v["key"] == "body").unwrap();
    let property = items.iter().find(|v| v["key"] == "property:biography").unwrap();
    assert_eq!(property["text"], "orchid orchid");
    assert_ne!(body["text"], property["text"]);
    assert_eq!(property["configured_tokenizer"], "unicode61");
    assert!(property["effective_ddl"]
        .as_str()
        .unwrap()
        .contains("porter unicode61 remove_diacritics 2"));
    assert_eq!(property["total_bytes"], "13");
    assert_eq!(property["complete"], true);
    assert_eq!(property["origin"]["handle"], selected);
    assert_eq!(property["origin"]["record"]["logical_id"], "lexical-a");
    assert_eq!(data["context"]["snapshot"], "this_request_only");
    assert!(items.iter().any(|v| v["key"] == "bm25f:status" && v["text"] == "open"));
    let later = dispatch(
        &mut reader,
        json!({"op":"indexed","handle":selected,"key":"property:biography","offset":"7","limit":6}),
    )
    .unwrap();
    assert_eq!(later["items"][0]["text"], "orchid");
    assert_ne!(later["context"]["operation"], data["context"]["operation"]);
    reader.session.close().unwrap();
}

#[test]
fn native_body_and_property_queries_keep_score_families_real_snippets_and_exact_results() {
    let (_dir, path) = corpus();
    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let outcome = dispatch(&mut reader, query("body", "runner"));
    assert!(outcome.is_ok(), "native lexical request must succeed: {outcome:?}");
    let body = outcome.unwrap();
    assert_eq!(body["scope"]["score_family"], "rrf");
    let hits = body["items"].as_array().unwrap();
    assert_eq!(hits.len(), 3, "0.8.30 record-keyed fusion preserves equal-body records separately");
    let mut ids = std::collections::BTreeSet::new();
    for (rank, hit) in hits.iter().enumerate() {
        let id = hit["native_id"]["value"].as_str().unwrap();
        ids.insert(id);
        let score: f64 = hit["score"].as_str().unwrap().parse().unwrap();
        // Shorter equal bodies occupy text-arm ranks 1 and 2 (ordered by write_cursor);
        // the longer Café body is rank 3. Each stored record keeps its own rank.
        let arm_rank = match id {
            "lexical-b" => 1.0,
            "lexical-twin" => 2.0,
            "lexical-a" => 3.0,
            other => panic!("unexpected lexical hit {other}"),
        };
        assert!((score - 3.0 / (30.0 + arm_rank)).abs() < 1e-12);
        assert_eq!(hit["rank"], (rank + 1).to_string());
        assert_eq!(hit["origin"]["state"], "exact");
        assert_eq!(hit["origin"]["all_equal_body_contributors"], "record_keyed_not_merged");
        assert_eq!(hit["origin"]["targets"][0]["record"]["write_cursor"], hit["write_cursor"]);
        assert_eq!(hit["snippet"]["scope"], "engine_query_reader");
        let payload = hit["payload"]["text"].as_str().unwrap();
        for span in hit["snippet"]["spans"].as_array().unwrap() {
            let (a, b) = (span[0].as_u64().unwrap() as usize, span[1].as_u64().unwrap() as usize);
            assert!(payload.get(a..b).is_some(), "native spans must use UTF8 byte boundaries");
        }
    }
    assert_eq!(ids, std::collections::BTreeSet::from(["lexical-a", "lexical-b", "lexical-twin"]));
    let accent = dispatch(&mut reader, query("body", "cafe")).unwrap();
    assert_eq!(accent["items"].as_array().unwrap().len(), 1);
    let snippet = &accent["items"][0]["snippet"];
    assert!(!snippet["highlights"].as_array().unwrap().is_empty());
    let first = &snippet["highlights"][0];
    let highlighted = &snippet["text"].as_str().unwrap()
        [first[0].as_u64().unwrap() as usize..first[1].as_u64().unwrap() as usize];
    assert_eq!(highlighted, "Café");
    let mut projected = query("property", "orchid");
    projected["property"] = json!("biography");
    let property = dispatch(&mut reader, projected.clone()).unwrap();
    assert_eq!(property["scope"]["score_family"], "negated_sqlite_bm25");
    let hits = property["items"].as_array().unwrap();
    assert_eq!(hits.len(), 3, "property route does not dedup equal canonical bodies");
    assert_eq!(hits[0]["native_id"]["value"], "lexical-a");
    // Five independently authored values, lengths 2,1,1,1,1; df=3 clamps IDF to 1e-6.
    let expected = 1e-6 * 2.0 * 2.2 / (2.0 + 1.2 * (0.25 + 0.75 * 2.0 / 1.2));
    assert!((hits[0]["score"].as_str().unwrap().parse::<f64>().unwrap() - expected).abs() < 1e-14);
    assert_eq!(hits[0]["snippet"]["state"], "unavailable");
    assert_ne!(property["query_context"]["snapshot"], property["context"]["snapshot"]);
    projected["filter"] = json!({"attributes":[["biography","orchid"]]});
    projected["limit"] = json!(1);
    let filtered = dispatch(&mut reader, projected).unwrap();
    assert_eq!(filtered["items"].as_array().unwrap().len(), 1);
    assert_ne!(filtered["items"][0]["native_id"]["value"], "lexical-a");
    let mut invalid = query("body", " ");
    assert_eq!(dispatch(&mut reader, invalid.clone()), Err("invalid_range"));
    invalid["query"] = json!("runner");
    invalid["limit"] = json!(101);
    assert_eq!(dispatch(&mut reader, invalid), Err("invalid_range"));
    assert!(dispatch(&mut reader, query("body", "unfindableword")).unwrap()["items"]
        .as_array()
        .unwrap()
        .is_empty());
    let mut invalid = query("property", "orchid");
    invalid["property"] = json!("undeclared");
    assert_eq!(dispatch(&mut reader, invalid), Err("unsupported_constraint"));
    reader.session.close().unwrap();
}

#[test]
fn bm25f_is_a_real_bounded_distinct_substrate_with_native_validity_semantics() {
    let dir = tempfile::tempdir().unwrap();
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Diagnostics).unwrap();
    let path = dir.path().join("bm25f.sqlite");
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    let mut late = node("outside-window", "orchid");
    if let fathomdb_engine::PreparedWrite::Node { valid_from, .. } = &mut late {
        *valid_from = Some(4102444800);
    }
    opened
        .engine
        .write(&[node("twice", "orchid orchid"), late, node("nonmatch", "stone")])
        .unwrap();
    opened.engine.close().unwrap();
    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let mut request = query("bm25f", "orchid");
    request["limit"] = json!(1);
    let outcome = dispatch(&mut reader, request.clone());
    assert!(outcome.is_ok(), "native BM25F must be available: {outcome:?}");
    let result = outcome.unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 1);
    assert_eq!(result["scope"]["score_family"], "native_bm25f");
    assert_eq!(result["scope"]["native_limit"], Value::Null);
    assert_eq!(result["scope"]["native_returned"], 2);
    assert_eq!(result["scope"]["own_node_validity"], "not_filtered_by_native_bm25f");
    let hit = &result["items"][0];
    assert_eq!(hit["origin"]["targets"][0]["record"]["logical_id"], "twice");
    assert_eq!(hit["payload"]["scope"], "later_exact_cursor_hydration");
    // Three active/current documents, body lengths2/1/1, df2; native BM25F IDF is ln(1+1.5/2.5).
    let weighted_tf = 2.0 / (0.25 + 0.75 * 2.0 / (4.0 / 3.0));
    let expected = (1.0_f64 + 1.5 / 2.5).ln() * weighted_tf * 2.2 / (1.2 + weighted_tf);
    assert!((hit["score"].as_str().unwrap().parse::<f64>().unwrap() - expected).abs() < 1e-12);
    request["limit"] = json!(10);
    let result = dispatch(&mut reader, request.clone()).unwrap();
    assert!(result["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|h| h["origin"]["targets"][0]["record"]["logical_id"] == "outside-window"));
    request["filter"] = json!({"kind":"document"});
    assert_eq!(dispatch(&mut reader, request.clone()), Err("unsupported_constraint"));
    request["filter"] = json!({});
    request["view"] = json!({"include_inactive":true});
    assert_eq!(dispatch(&mut reader, request), Err("unsupported_constraint"));
    reader.session.close().unwrap();
}

#[test]
fn native_resource_admission_accounts_for_historical_property_alias_expansion() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    let specs = (0..24)
        .map(|n| ProjectionSpec {
            name: format!("alias-{n}"),
            roles: std::collections::BTreeSet::from([ProjectionRole::Searchable]),
            fts: Some(ProjectionFts::default()),
            vector: None,
            source: Some(vec!["text".into()]),
        })
        .collect::<Vec<_>>();
    opened.engine.configure_projections(&specs, &[]).unwrap();
    let mut retained = node(
        "retained-large",
        &json!({"text":format!("orchid {}","x ".repeat(1_500_000))}).to_string(),
    );
    if let fathomdb_engine::PreparedWrite::Node { state, .. } = &mut retained {
        *state = fathomdb_engine::InitialState::Pending;
    }
    opened.engine.write(&[retained]).unwrap();
    opened.engine.close().unwrap();
    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let mut request = query("property", "orchid");
    request["property"] = json!("alias-0");
    request["view"] = json!({"include_inactive":true});
    assert_eq!(
        dispatch(&mut reader, request),
        Err("budget_exhausted"),
        "native scratch aliases must be charged before invocation"
    );
    reader.session.close().unwrap();
}

#[test]
fn lexical_disclosure_composes_all_eight_real_queue_flags_and_rechecks_each_transaction() {
    use fathomdb_engine::{
        EngineConfig, PhysicalErasurePolicy, PhysicalErasureSelector, QueueResult,
    };
    let dir = tempfile::tempdir().unwrap();
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Diagnostics).unwrap();
    let path = dir.path().join("queued-lexical.sqlite");
    let engine = std::sync::Arc::new(
        Engine::open_with_choice_and_config(
            &path,
            EmbedderChoice::None,
            EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
        )
        .unwrap()
        .engine,
    );
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "biography".into(),
                roles: std::collections::BTreeSet::from([ProjectionRole::Searchable]),
                fts: Some(ProjectionFts::default()),
                vector: None,
                source: None,
            }],
            &[],
        )
        .unwrap();
    let queued = node("queued-lexical", r#"{"text":"orchid queued","biography":"orchid"}"#);
    let mut pending = node("pending-lexical", r#"{"text":"orchid pending","biography":"orchid"}"#);
    if let fathomdb_engine::PreparedWrite::Node { state, .. } = &mut pending {
        *state = fathomdb_engine::InitialState::Pending;
    }
    let mut clear = node("clear-lexical", r#"{"text":"orchid clear","biography":"orchid"}"#);
    if let fathomdb_engine::PreparedWrite::Node { source_id, .. } = &mut clear {
        *source_id = fathomdb_engine::SourceId::new("fixture:unrelated").unwrap();
    }
    engine.write(&[queued, pending, clear]).unwrap();
    let mut maintenance = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = match maintenance
        .queue_physical_erasure(
            PhysicalErasureSelector::SourceAllAtCut { source_id: "fixture:source".into() },
            "123456789abcdef0123456789abcdef0",
        )
        .unwrap()
    {
        QueueResult::Accepted { ticket, .. } => ticket,
        other => panic!("legitimate queue expected: {other:?}"),
    };
    maintenance.close().unwrap();
    drop(maintenance);
    engine.close().unwrap();
    drop(engine);
    for n in [false, true] {
        for e in [false, true] {
            for t in [false, true] {
                let mut reader = Reader::new(
                    Session::attach(
                        &path,
                        Policy { no_show_inactive: n, show_erased: e, show_payloads: t },
                    )
                    .unwrap(),
                );
                for route in ["body", "property", "bm25f"] {
                    let mut request = query(route, "orchid");
                    if route == "property" {
                        request["property"] = json!("biography");
                    }
                    if route != "bm25f" {
                        request["view"] = json!({"include_inactive":true});
                    }
                    let result = dispatch(&mut reader, request).unwrap_or_else(|error| {
                        panic!(
                            "route={route}, inactive_hidden={n}, erased={e}, payloads={t}: {error}"
                        )
                    });
                    let items = result["items"].as_array().unwrap();
                    let ids = items
                        .iter()
                        .map(|v| {
                            v["origin"]["targets"][0]["record"]["logical_id"].as_str().unwrap()
                        })
                        .collect::<std::collections::BTreeSet<_>>();
                    assert!(ids.contains("clear-lexical"));
                    assert_eq!(ids.contains("queued-lexical"), e);
                    assert_eq!(ids.contains("pending-lexical"), route != "bm25f" && e && !n);
                    if !e {
                        assert!(!serde_json::to_string(&result)
                            .unwrap()
                            .contains("queued-lexical"));
                        let clear = items
                            .iter()
                            .find(|v| {
                                v["origin"]["targets"][0]["record"]["logical_id"] == "clear-lexical"
                            })
                            .unwrap();
                        assert!(
                            clear["rank"].as_str().unwrap().parse::<usize>().unwrap() > 1,
                            "post-policy disclosure preserves original native rank gaps"
                        );
                    }
                    for hit in items {
                        let owner = hit["origin"]["targets"][0]["handle"].as_str().unwrap();
                        assert_eq!(
                            dispatch(
                                &mut reader,
                                json!({"op":"indexed","handle":owner,"offset":"0","limit":4096})
                            )
                            .unwrap()["status"],
                            "complete"
                        );
                    }
                }
                reader.session.close().unwrap();
            }
        }
    }
    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let first = dispatch(&mut reader, query("body", "queued")).unwrap();
    assert!(first["items"].as_array().unwrap().is_empty());
    reader.session.engine.withdraw_queued_erasure(&ticket).unwrap();
    let next = dispatch(&mut reader, query("body", "queued")).unwrap();
    assert_eq!(
        next["items"].as_array().unwrap().len(),
        1,
        "fresh read must observe actual queue withdrawal"
    );
    reader.session.close().unwrap();
}

#[test]
fn lexical_large_payloads_are_bounded_original_windows_and_native_edge_origins_are_class_exact() {
    let (_dir, path) = corpus();
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    let body = format!("orchid {}", "é🦀 ".repeat(1_500_000));
    assert!(body.len() > 10 * 1024 * 1024);
    opened
        .engine
        .write(&[
            node("large-lexical", &body),
            fathomdb_engine::PreparedWrite::Edge {
                kind: "knows".into(),
                from: "lexical-a".into(),
                to: "lexical-b".into(),
                source_id: fathomdb_engine::SourceId::new("fixture:edge").unwrap(),
                logical_id: Some("lexical-a".into()),
                body: Some("café edge-only".into()),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .unwrap();
    opened.engine.close().unwrap();
    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let result = dispatch(&mut reader, query("body", "orchid")).unwrap();
    let hit = result["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["origin"]["targets"][0]["record"]["logical_id"] == "large-lexical")
        .unwrap();
    assert_eq!(hit["payload"]["total_bytes"], body.len().to_string());
    assert_eq!(hit["payload"]["complete"], false);
    assert!(hit["payload"]["text"].as_str().unwrap().len() <= 4096);
    assert!(serde_json::to_vec(&result).unwrap().len() <= 2 * 1024 * 1024);
    let selected = handle(&mut reader, "large-lexical");
    let window = dispatch(
        &mut reader,
        json!({"op":"indexed","handle":selected,"key":"body","offset":"0","limit":65536}),
    )
    .unwrap();
    assert_eq!(
        window["items"][0]["text"].as_str().unwrap(),
        &body[..window["items"][0]["text"].as_str().unwrap().len()]
    );
    assert!(window["items"][0]["continuation"].is_string());
    let edge = dispatch(&mut reader, query("body", "edge-only")).unwrap();
    assert_eq!(edge["items"].as_array().unwrap().len(), 1);
    assert_eq!(edge["items"][0]["class"], "edge");
    assert_eq!(edge["items"][0]["origin"]["targets"][0]["record"]["class"], "edge");
    assert_eq!(edge["items"][0]["origin"]["targets"][0]["record"]["logical_id"], "lexical-a");
    let owner = edge["items"][0]["origin"]["targets"][0]["handle"].as_str().unwrap();
    assert_eq!(
        dispatch(&mut reader, json!({"op":"detail","handle":owner})).unwrap()["content"]["text"],
        "café edge-only"
    );
    reader.session.close().unwrap();
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(12))]
    #[test]
    fn lexical_request_roundtrip_retains_exact_values_and_native_bounds(text in "[a-zé🦀]{1,40}",limit in 1_usize..=100) {
        let original=query("body",&text);
        let mut request=original.clone();request["limit"]=json!(limit);
        let typed:crate::storage::lexical::Query=serde_json::from_value({let mut v=request.clone();v.as_object_mut().unwrap().remove("op");v}).unwrap();
        let roundtrip=serde_json::to_value(&typed).unwrap();
        proptest::prop_assert_eq!(roundtrip["query"].as_str().unwrap(),text.as_str());
        proptest::prop_assert_eq!(roundtrip["limit"].as_u64().unwrap(),limit as u64);
    }
}

#[test]
fn relaxed_property_result_opens_the_exact_retained_revision_without_current_substitution() {
    let (_dir, path) = corpus();
    let original = r#"{"text":"Café runners <b>run</b>","profile":{"biography":"orchid orchid"},"status":"open"}"#;
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .write(&[node(
            "lexical-a",
            r#"{"text":"replacement","profile":{"biography":"tulip"},"status":"open"}"#,
        )])
        .unwrap();
    opened.engine.close().unwrap();
    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let mut request = query("property", "orchid");
    request["property"] = json!("biography");
    request["view"] = json!({"include_superseded":true});
    let result = dispatch(&mut reader, request).unwrap();
    let retained = result["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["native_id"]["value"] == "lexical-a")
        .unwrap();
    let owner = retained["origin"]["targets"][0]["handle"].as_str().unwrap();
    let exact = dispatch(&mut reader, json!({"op":"detail","handle":owner})).unwrap();
    let expected: Value = serde_json::from_str(original).unwrap();
    let actual: Value = serde_json::from_str(exact["content"]["text"].as_str().unwrap()).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(exact["record"]["write_cursor"], retained["write_cursor"]);
    assert_eq!(exact["record"]["inactive_or_history"], true);
    assert_ne!(exact["context"]["operation"], result["context"]["operation"]);
    reader.session.close().unwrap();
}

#[test]
fn record_keyed_fusion_preserves_equal_body_node_edge_pairs_repeated_public_ids_and_retained_revisions(
) {
    use fathomdb_engine::{PreparedWrite, SourceId};
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let shared = "shared phoenix body";
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .write(&[
            node("phoenix-1", shared),
            node("phoenix-2", shared),
            node("phoenix-reused", shared),
            PreparedWrite::Edge {
                kind: "relates".into(),
                from: "phoenix-1".into(),
                to: "phoenix-2".into(),
                source_id: SourceId::new("fixture:edge").unwrap(),
                logical_id: Some("phoenix-edge".into()),
                body: Some(shared.into()),
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.9),
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .unwrap();
    // Write a second revision under the same logical_id with the exact same body.
    opened.engine.write(&[node("phoenix-reused", shared)]).unwrap();
    opened.engine.close().unwrap();

    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let mut request = query("body", "phoenix");
    request["view"] = json!({"include_superseded": true});
    let result = dispatch(&mut reader, request).unwrap();
    let hits = result["items"].as_array().unwrap();
    assert_eq!(
        hits.len(),
        5,
        "equal-body nodes, edge, and repeated logical_id revisions must remain distinct: {hits:?}"
    );
    let mut cursors = std::collections::BTreeSet::new();
    let mut edge_count = 0usize;
    let mut reused_count = 0usize;
    for (rank, hit) in hits.iter().enumerate() {
        assert_eq!(hit["rank"], (rank + 1).to_string());
        assert_eq!(hit["origin"]["state"], "exact");
        assert_eq!(hit["origin"]["all_equal_body_contributors"], "record_keyed_not_merged");
        let cursor = hit["write_cursor"].as_str().unwrap().to_string();
        assert!(cursors.insert(cursor.clone()), "each stored record must keep a unique cursor");
        assert_eq!(hit["origin"]["targets"][0]["record"]["write_cursor"], cursor);
        if hit["class"] == "edge" {
            edge_count += 1;
            assert_eq!(hit["representative_branch"], "text_edge");
            assert_eq!(hit["native_id"]["value"], "phoenix-edge");
        } else if hit["native_id"]["value"] == "phoenix-reused" {
            reused_count += 1;
        }
    }
    assert_eq!(edge_count, 1, "equal-body edge must remain separate from equal-body nodes");
    assert_eq!(reused_count, 2, "both superseded and current revisions of repeated ID must appear");
    reader.session.close().unwrap();
}

#[test]
fn diverged_fts_body_or_kind_refuses_and_ownerless_node_fts_is_withheld_truthfully() {
    use fathomdb_engine::{PreparedWrite, SourceId};
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .write(&[
            node("consistent-node", "cobalt signal body"),
            node("kind-node", "topaz kind signal body"),
            PreparedWrite::Edge {
                kind: "supports".into(),
                from: "consistent-node".into(),
                to: "alpha".into(),
                source_id: SourceId::new("fixture:edge").unwrap(),
                logical_id: Some("consistent-edge".into()),
                body: Some("amber edge signal".into()),
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.8),
                extractor_model_id: None,
                temporal_fallback: None,
            },
            PreparedWrite::Edge {
                kind: "supports".into(),
                from: "kind-node".into(),
                to: "alpha".into(),
                source_id: SourceId::new("fixture:edge").unwrap(),
                logical_id: Some("kind-edge".into()),
                body: Some("emerald kind edge signal".into()),
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.8),
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .unwrap();
    opened.engine.close().unwrap();

    // 1. Genuine ownerless FTS row: Engine returns it, Inspector withholds cleanly.
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute(
        "INSERT INTO search_index(body, kind, write_cursor) VALUES ('ownerlessquasar', 'note', 999999)",
        [],
    )
    .unwrap();
    drop(c);

    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let ownerless = dispatch(&mut reader, query("body", "ownerlessquasar")).unwrap();
    assert_eq!(ownerless["scope"]["native_returned"], 1);
    assert_eq!(ownerless["scope"]["withheld"], 1);
    assert!(ownerless["items"].as_array().unwrap().is_empty());
    reader.session.close().unwrap();

    // 2. Diverged node FTS body relative to canonical_nodes: 0.8.30 Engine refuses.
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute(
        "UPDATE search_index SET body = 'tampered cobalt signal body' WHERE write_cursor = (SELECT write_cursor FROM canonical_nodes WHERE logical_id = 'consistent-node')",
        [],
    )
    .unwrap();
    drop(c);

    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    assert_eq!(dispatch(&mut reader, query("body", "cobalt")), Err("storage"));
    reader.session.close().unwrap();

    // 3. Diverged edge FTS body relative to canonical_edges: 0.8.30 Engine refuses.
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute(
        "UPDATE search_index_edges SET body = 'tampered amber edge signal' WHERE write_cursor = (SELECT write_cursor FROM canonical_edges WHERE logical_id = 'consistent-edge')",
        [],
    )
    .unwrap();
    drop(c);

    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    assert_eq!(dispatch(&mut reader, query("body", "amber")), Err("storage"));
    reader.session.close().unwrap();

    // 4. Diverged node FTS kind relative to canonical_nodes: 0.8.30 Engine refuses.
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute(
        "UPDATE search_index SET kind = 'tampered_node_kind' WHERE write_cursor = (SELECT write_cursor FROM canonical_nodes WHERE logical_id = 'kind-node')",
        [],
    )
    .unwrap();
    drop(c);

    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    assert_eq!(dispatch(&mut reader, query("body", "topaz")), Err("storage"));
    reader.session.close().unwrap();

    // 5. Diverged edge FTS kind relative to canonical_edges: 0.8.30 Engine refuses.
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute(
        "UPDATE search_index_edges SET kind = 'tampered_edge_kind' WHERE write_cursor = (SELECT write_cursor FROM canonical_edges WHERE logical_id = 'kind-edge')",
        [],
    )
    .unwrap();
    drop(c);

    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    assert_eq!(dispatch(&mut reader, query("body", "emerald")), Err("storage"));
    reader.session.close().unwrap();
}
