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
