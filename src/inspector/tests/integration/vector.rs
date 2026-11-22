use super::dispatch;
use crate::{fixtures::database, policy::Policy, session::Session, storage::Reader};
use serde_json::json;

#[test]
fn vector_inspection_retains_exact_origin_even_without_materialization() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let page = dispatch(
        &mut reader,
        json!({"op":"page","class":"node","after":"0","limit":10,"filter":{"identity":"alpha"}}),
    )
    .unwrap();
    let handle = page["items"][0]["handle"].as_str().unwrap();
    let result = dispatch(&mut reader, json!({"op":"vectors","handle":handle}));
    assert!(result.is_ok(), "stored vector operation must exist: {result:?}");
    let result = result.unwrap();
    assert_eq!(result["items"], json!([]));
    assert_eq!(result["scope"]["origin"]["handle"], handle);
    assert_eq!(result["scope"]["origin"]["record"]["logical_id"], "alpha");
    assert_eq!(result["scope"]["materialization"], "not_observed");
    assert_eq!(result["scope"]["retrieval_eligibility"], "not_evaluated");
    assert_eq!(
        dispatch(&mut reader, json!({"op":"vectors","handle":"forged"})).unwrap_err(),
        "reference_expired"
    );
    reader.session.close().unwrap();
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(12))]
    #[test]
    fn neighborhood_invalid_limits_are_rejected_before_reference_lookup(limit in proptest::prop_oneof![proptest::strategy::Just(0usize),101usize..10000]) {
        let dir=tempfile::tempdir().unwrap();let path=database(dir.path());
        let mut reader=Reader::new(Session::attach(&path,Policy::default()).unwrap());
        let result=dispatch(&mut reader,json!({"op":"neighborhood","handle":"forged","vector_rowid":"1","after":"0","limit":limit}));
        proptest::prop_assert_eq!(result.unwrap_err(),"invalid_range");
        reader.session.close().unwrap();
    }
}

#[test]
#[ignore = "mandatory real stored-vector numeric/head-window positive; requires pinned cache and owned wheel"]
fn real_head_window_neighbors_match_independent_f64_oracle_and_exact_origins() {
    let dir = tempfile::tempdir().unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let output = std::process::Command::new(root.join(".venv/bin/python"))
        .arg(root.join("src/inspector/tools/vector-fixture.py"))
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let path = std::path::PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let page = dispatch(
        &mut reader,
        json!({"op":"page","class":"node","after":"0","limit":10,"filter":{}}),
    )
    .unwrap();
    let mut vectors = vec![];
    for item in page["items"].as_array().unwrap() {
        let handle = item["handle"].as_str().unwrap();
        let data = dispatch(&mut reader, json!({"op":"vectors","handle":handle})).unwrap();
        let vector = &data["items"][0];
        assert_eq!(vector["source_cursor"], item["write_cursor"]);
        assert_eq!(vector["vector_rowid"], vector["source_cursor"]);
        assert_eq!(vector["producer"]["dimension"], 384);
        assert_eq!(vector["window"]["number"], 0);
        assert_eq!(vector["window"]["byte_start"], "0");
        assert_eq!(vector["origin"]["targets"][0]["handle"], handle);
        vectors.push((handle.to_string(), vector.clone()));
    }
    let seed = &vectors[0];
    let result = dispatch(
        &mut reader,
        json!({"op":"neighborhood","handle":seed.0,"vector_rowid":seed.1["vector_rowid"],"after":"0","limit":10}),
    );
    assert!(result.is_ok(), "stored L2 operation must succeed: {result:?}");
    let result = result.unwrap();
    assert_eq!(result["scope"]["examined"], 2);
    assert_eq!(result["scope"]["visible"], 2);
    assert_eq!(result["scope"]["returned"], 2);
    assert_eq!(result["scope"]["metric"], "uncentered_f32_euclidean_l2");
    assert_eq!(result["scope"]["direction"], "lower_nearer");
    let values = |vector: &serde_json::Value| {
        vector["values"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect::<Vec<_>>()
    };
    let seed_values = values(&seed.1);
    let mut expected = vec![];
    for (_, v) in &vectors {
        let other = values(v);
        let distance =
            seed_values.iter().zip(other).map(|(a, b)| (a - b) * (a - b)).sum::<f64>().sqrt();
        expected.push((v["vector_rowid"].clone(), distance));
    }
    expected.sort_by(|a, b| a.1.total_cmp(&b.1));
    for (hit, (rowid, oracle)) in result["items"].as_array().unwrap().iter().zip(expected) {
        assert_eq!(hit["vector_rowid"], rowid);
        let actual: f64 = hit["distance"].as_str().unwrap().parse().unwrap();
        assert!(
            (actual - oracle).abs() <= 384.0 * f32::EPSILON as f64 * oracle.max(1.0),
            "sqlitevec {actual}, independent f64 {oracle}"
        );
        let origin = &hit["origin"]["targets"][0];
        let detail =
            dispatch(&mut reader, json!({"op":"detail","handle":origin["handle"]})).unwrap();
        assert_eq!(detail["record"]["write_cursor"], hit["source_cursor"]);
    }
    assert_eq!(result["items"][0]["distance"], "0");
    let reader_cell = std::cell::RefCell::new(&mut reader);
    let mut runner =
        proptest::test_runner::TestRunner::new(proptest::test_runner::Config::with_cases(24));
    runner.run(&(0usize..vectors.len(),1usize..=100,0i64..=3),|(seed_index,limit,after)|{
        let selected=&vectors[seed_index];let data=dispatch(&mut reader_cell.borrow_mut(),json!({"op":"neighborhood","handle":selected.0,"vector_rowid":selected.1["vector_rowid"],"after":after.to_string(),"limit":limit})).unwrap();
        let anchor=values(&selected.1);let mut oracle=vectors.iter().filter_map(|(_,v)|{
            let rowid:i64=v["vector_rowid"].as_str().unwrap().parse().unwrap();if rowid<=after{return None;}
            let distance=anchor.iter().zip(values(v)).map(|(a,b)|(a-b).powi(2)).sum::<f64>().sqrt();Some((rowid,distance))
        }).collect::<Vec<_>>();oracle.sort_by(|a,b|a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));oracle.truncate(limit);
        proptest::prop_assert_eq!(data["items"].as_array().unwrap().len(),oracle.len());
        proptest::prop_assert!(data["scope"]["work"].as_u64().unwrap()<=100000);
        proptest::prop_assert!(serde_json::to_vec(&data).unwrap().len()<=2*1024*1024);
        for(hit,(rowid,distance))in data["items"].as_array().unwrap().iter().zip(oracle){
            proptest::prop_assert_eq!(hit["vector_rowid"].as_str().unwrap(),rowid.to_string());
            let actual:f64=hit["distance"].as_str().unwrap().parse().unwrap();proptest::prop_assert!((actual-distance).abs()<=384.0*f32::EPSILON as f64*distance.max(1.0));
        }
        Ok(())
    }).unwrap();
    reader.session.close().unwrap();
}

#[test]
#[ignore = "mandatory all8 vector disclosure, sign-bit, unknown-coverage and numeric-overflow qualification"]
fn stored_vectors_obey_all_disclosure_combinations_and_fresh_queue_state() {
    let dir = tempfile::tempdir().unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let fixture = std::process::Command::new(root.join(".venv/bin/python"))
        .arg(root.join("src/inspector/tools/vector-fixture.py"))
        .arg(dir.path())
        .arg("history-queued")
        .output()
        .unwrap();
    assert!(fixture.status.success(), "{}", String::from_utf8_lossy(&fixture.stderr));
    let path = std::path::PathBuf::from(String::from_utf8(fixture.stdout).unwrap().trim());
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
                let page=dispatch(&mut reader,json!({"op":"page","class":"node","after":"0","limit":10,"filter":{"identity":"dog"}})).unwrap();
                assert_eq!(page["items"].as_array().unwrap().len(), if n { 1 } else { 2 });
                for item in page["items"].as_array().unwrap() {
                    let handle = item["handle"].as_str().unwrap();
                    let vectors = dispatch(&mut reader, json!({"op":"vectors","handle":handle}));
                    if !e {
                        assert_eq!(vectors.unwrap_err(), "refused");
                        continue;
                    }
                    let vectors = vectors.unwrap();
                    let vector = &vectors["items"][0];
                    assert_eq!(vector["source_cursor"], item["write_cursor"]);
                    assert_eq!(vector["window"]["number"], 0);
                    assert_eq!(vector["window"]["byte_start"], "0");
                    let c = reader.session.connection().unwrap();
                    let actual: Vec<u8> = c
                        .query_row(
                            "SELECT embedding_bin FROM vector_default WHERE rowid=?1",
                            [vector["vector_rowid"].as_str().unwrap().parse::<i64>().unwrap()],
                            |r| r.get(0),
                        )
                        .unwrap();
                    let mean:Option<Vec<u8>>=c.query_row("SELECT mean_vec FROM _fathomdb_embedder_profiles WHERE profile='default'",[],|r|r.get(0)).unwrap();
                    assert!(mean.is_none());
                    let mut expected = vec![0u8; 48];
                    for (index, value) in vector["values"].as_array().unwrap().iter().enumerate() {
                        if value.as_f64().unwrap() > 0.0 {
                            expected[index / 8] |= 1 << (index % 8);
                        }
                    }
                    assert_eq!(actual, expected, "independent unpinned sign-bit oracle");
                    drop(c);
                    let neighbors=dispatch(&mut reader,json!({"op":"neighborhood","handle":handle,"vector_rowid":vector["vector_rowid"],"after":"0","limit":10})).unwrap();
                    assert_eq!(neighbors["items"].as_array().unwrap().len(), if n { 2 } else { 3 });
                    for hit in neighbors["items"].as_array().unwrap() {
                        let origin = &hit["origin"]["targets"][0];
                        assert_eq!(origin["record"]["write_cursor"], hit["source_cursor"]);
                        assert_eq!(origin["record"]["retained_erasure_pending"], true);
                        if n {
                            assert_eq!(origin["record"]["inactive_or_history"], false);
                        }
                    }
                }
                reader.session.close().unwrap();
            }
        }
    }
    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let page = dispatch(
        &mut reader,
        json!({"op":"page","class":"node","after":"0","limit":10,"filter":{"identity":"dog"}}),
    )
    .unwrap();
    let handle = page["items"][0]["handle"].as_str().unwrap();
    assert_eq!(
        dispatch(&mut reader, json!({"op":"vectors","handle":handle})).unwrap_err(),
        "refused"
    );
    let c = reader.session.connection().unwrap();
    let ticket: String = c
        .query_row("SELECT ticket FROM _fathomdb_erasure_requests WHERE phase='queued'", [], |r| {
            r.get(0)
        })
        .unwrap();
    drop(c);
    reader.session.engine.withdraw_queued_erasure(&ticket).unwrap();
    let vectors = dispatch(&mut reader, json!({"op":"vectors","handle":handle})).unwrap();
    assert_eq!(
        vectors["items"][0]["origin"]["targets"][0]["record"]["retained_erasure_pending"],
        false
    );
    let rowid = vectors["items"][0]["vector_rowid"].as_str().unwrap().parse::<i64>().unwrap();
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute("UPDATE _fathomdb_vector_spans SET byte_end=NULL WHERE vector_rowid=?1", [rowid])
        .unwrap();
    drop(c);
    let unknown = dispatch(&mut reader, json!({"op":"vectors","handle":handle})).unwrap();
    assert_eq!(unknown["items"][0]["window"]["coverage"], "unknown");
    assert_eq!(unknown["items"][0]["window"]["text"], serde_json::Value::Null);
    let c = rusqlite::Connection::open(&path).unwrap();
    let bytes = f32::MAX.to_le_bytes().repeat(384);
    c.execute("UPDATE vector_default SET embedding=?1 WHERE rowid=2", [bytes]).unwrap();
    drop(c);
    assert_eq!(dispatch(&mut reader,json!({"op":"neighborhood","handle":handle,"vector_rowid":rowid.to_string(),"after":"0","limit":10})).unwrap_err(),"unsupported_value_encoding");
    reader.session.close().unwrap();
}

#[test]
fn serving_reports_embedder_absent_degradation_rejects_ownerless_fts_and_propagates_closing() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    // Force serving route against a real EmbedderChoice::None Engine to verify
    // 0.8.29/0.8.30 EmbedderAbsent degradation passthrough.
    reader.session.cached_cpu = true;
    let degraded =
        dispatch(&mut reader, json!({"op":"serving","query":"alpha","limit":10})).unwrap();
    assert_eq!(
        degraded["query_context"]["degradations"],
        json!([{"arm": "Vector", "reason": "embedder_absent"}]),
        "serving must preserve Engine EmbedderAbsent degradation wire word"
    );
    reader.session.close().unwrap();

    // Ownerless node FTS row must cause serving (include_explanation=true) to refuse
    // instead of fabricating structural evidence.
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute(
        "INSERT INTO search_index(body, kind, write_cursor) VALUES ('ownerlessservingterm', 'note', 999998)",
        [],
    )
    .unwrap();
    drop(c);

    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    reader.session.cached_cpu = true;
    assert_eq!(
        dispatch(&mut reader, json!({"op":"serving","query":"ownerlessservingterm","limit":10})),
        Err("native_query_refused")
    );

    // Closed Engine must propagate Closing cleanly without panicking or falling back:
    // search_frozen against an already-frozen context returns EngineError::Closing
    // (mapped by Reader::serving to "native_query_refused"), while Reader::serving
    // after close fails at freeze_read_context with exact Err("storage").
    let frozen = reader
        .session
        .engine
        .freeze_read_context(
            &fathomdb_engine::ReadContextV1::new(
                fathomdb_engine::ReadView::default(),
                fathomdb_engine::SearchFilter::default(),
            )
            .unwrap(),
        )
        .unwrap();
    reader.session.engine.close().unwrap();
    assert!(
        matches!(
            reader.session.engine.search_frozen("alpha", &frozen, 0, false, 0.3, 0, true, 10),
            Err(fathomdb_engine::EngineError::Closing)
        ),
        "search_frozen on closed engine must return EngineError::Closing"
    );
    assert_eq!(
        dispatch(&mut reader, json!({"op":"serving","query":"alpha","limit":10})),
        Err("storage"),
        "post-close Reader::serving must fail at freeze_read_context with Err(\"storage\")"
    );
}

#[cfg(feature = "cached-cpu")]
#[test]
fn cached_cpu_candle013_serving_preserves_equal_body_records_reports_timeout_and_validates_open_spans(
) {
    use fathomdb_engine::{
        EmbedderChoice, Engine, EngineConfig, PreparedWrite, ProjectionRole, ProjectionSpec,
        ProjectionVector, SourceId,
    };
    let cache_root = std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".cache")))
        .expect("XDG_CACHE_HOME or HOME must be set");
    let bge_dir = cache_root.join("fathomdb/embedders/0b2926f8a9b1");
    assert!(
        bge_dir.join("model.safetensors").is_file(),
        "pinned BGE cache prerequisite missing at {}",
        bge_dir.display()
    );

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cached-cpu-0830.sqlite");
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Diagnostics).unwrap();
    let shared = r#"{"summary":"orbital telemetry calibration beacon"}"#;
    let opened = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Default,
        EngineConfig {
            scheduler_runtime_threads: Some(1),
            embedder_pool_size: Some(1),
            embedder_call_timeout_ms: Some(25_000),
            ..EngineConfig::default()
        },
    )
    .unwrap();
    opened
        .engine
        .configure_projections(
            &[ProjectionSpec {
                name: "summary".into(),
                roles: std::collections::BTreeSet::from([ProjectionRole::Searchable]),
                fts: None,
                vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
                source: None,
            }],
            &[],
        )
        .unwrap();
    let make_doc = |id: &str| PreparedWrite::Node {
        kind: "doc".into(),
        body: shared.into(),
        source_id: SourceId::new("fixture:vector").unwrap(),
        logical_id: Some(id.into()),
        state: fathomdb_engine::InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    };
    opened
        .engine
        .write(&[
            make_doc("beacon-1"),
            make_doc("beacon-2"),
            PreparedWrite::Edge {
                kind: "tracks".into(),
                from: "beacon-1".into(),
                to: "beacon-2".into(),
                source_id: SourceId::new("fixture:edge").unwrap(),
                logical_id: Some("beacon-edge".into()),
                body: Some(shared.into()),
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.9),
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .unwrap();
    opened.engine.drain(30_000).unwrap();
    opened.engine.close().unwrap();

    // 1. Reopen via Inspector cached-cpu session (exercises deferred vector_spans::capture
    //    + retain_matching_open_seed + Candle 0.13 CPU serving).
    let mut reader = Reader::new(Session::attach_mode(&path, Policy::default(), true).unwrap());
    let served = dispatch(
        &mut reader,
        json!({"op":"serving","query":"orbital telemetry calibration beacon","limit":10}),
    )
    .unwrap();
    assert!(
        served["query_context"]["vector_hits"].as_u64().unwrap() >= 3,
        "cached-cpu hybrid must execute real dense vector arm across nodes and edge: {served:?}"
    );
    assert!(
        served["query_context"]["degradations"].as_array().unwrap().is_empty(),
        "healthy cached-cpu hybrid must report zero degradations: {served:?}"
    );
    let items = served["items"].as_array().unwrap();
    assert_eq!(items.len(), 3, "hybrid serving must preserve both equal-body nodes and edge");
    let cursors: std::collections::BTreeSet<_> =
        items.iter().map(|v| v["write_cursor"].as_str().unwrap()).collect();
    assert_eq!(cursors.len(), 3);
    assert!(
        items.iter().any(|v| {
            v["class"] == "edge"
                && v["representative_branch"] == "TextEdge"
                && v["native_id"]["value"] == "beacon-edge"
        }),
        "hybrid serving must include the equal-body edge item alongside equal-body nodes: {items:?}"
    );
    let node_ids: std::collections::BTreeSet<_> = items
        .iter()
        .filter(|v| v["class"] == "node")
        .map(|v| v["native_id"]["value"].as_str().unwrap())
        .collect();
    assert_eq!(node_ids, std::collections::BTreeSet::from(["beacon-1", "beacon-2"]));
    reader.session.close().unwrap();

    // 2. Model-free fallback still inspects stored vectors cleanly without loading BGE.
    let mut bare = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let page = dispatch(
        &mut bare,
        json!({"op":"page","class":"node","after":"0","limit":10,"filter":{"identity":"beacon-1"}}),
    )
    .unwrap();
    let handle = page["items"][0]["handle"].as_str().unwrap();
    let stored = dispatch(&mut bare, json!({"op":"vectors","handle":handle})).unwrap();
    assert_eq!(stored["items"].as_array().unwrap().len(), 1);
    assert_eq!(stored["items"][0]["window"]["coverage"], "recorded");
    assert_eq!(
        dispatch(&mut bare, json!({"op":"serving","query":"orbital telemetry","limit":10})),
        Err("model_unavailable")
    );
    bare.session.close().unwrap();

    // 3. Real EmbedderTimeout degradation routed through Inspector's Reader::serving
    //    (1ms dispatcher deadline against real Candle 0.13 CPU embedder).
    let mut timed_reader =
        Reader::new(Session::attach_mode(&path, Policy::default(), true).unwrap());
    timed_reader.session.engine.close().unwrap();
    timed_reader.session.engine = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Default,
        EngineConfig {
            scheduler_runtime_threads: Some(1),
            embedder_pool_size: Some(1),
            embedder_call_timeout_ms: Some(1),
            ..EngineConfig::default()
        },
    )
    .unwrap()
    .engine;
    timed_reader.session.lock_fingerprint =
        crate::admission::fingerprint(std::path::Path::new(&format!("{}.lock", path.display())))
            .unwrap();
    let timed = dispatch(
        &mut timed_reader,
        json!({"op":"serving","query":"orbital telemetry calibration beacon","limit":10}),
    )
    .unwrap();
    assert_eq!(
        timed["query_context"]["degradations"],
        json!([{"arm": "Vector", "reason": "embedder_timeout"}]),
        "Reader::serving must emit EmbedderTimeout wire word: {timed:?}"
    );
    assert_eq!(timed["query_context"]["vector_hits"], 0);
    timed_reader.session.close().unwrap();

    // 4. Corrupted _fathomdb_vector_spans (passes SQLite CHECK, violates canonical body length)
    //    must refuse safely in Engine startup via vector_spans::capture, not preflight.
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute("UPDATE _fathomdb_vector_spans SET body_length = body_length + 5", []).unwrap();
    drop(c);
    let failure = Session::attach(&path, Policy::default())
        .err()
        .expect("corrupted stored-head span must refuse safely on open");
    assert!(
        matches!(failure.phase, crate::failure::Phase::Startup)
            && failure.error == "startup_failed_effects_possible",
        "corrupted vector span must fail in Engine startup, not preflight: {failure:?}"
    );
}
