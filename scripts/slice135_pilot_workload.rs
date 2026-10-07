//! Small real-engine workload for the baseline-only Slice 135 noise pilot.

use std::io::{self, Write};
use std::time::Instant;

use fathomdb_engine::{Engine, InitialState, PreparedWrite, SourceId};
use serde_json::{json, Value};
use tempfile::TempDir;

fn emit(cell: &str, attempt: Value) {
    let mut record = attempt.as_object().expect("attempt object").clone();
    record.insert("cell".into(), json!(cell));
    println!("SLICE135_ATTEMPT {}", Value::Object(record));
    io::stdout().flush().expect("flush attempt before next operation");
}

fn seed(engine: &Engine) -> Result<(), String> {
    let writes = (0..32)
        .map(|index| PreparedWrite::Node {
            kind: "doc".into(),
            body: format!("needle memory document {index} for bounded search"),
            source_id: SourceId::new("slice135:pilot").unwrap(),
            logical_id: Some(format!("doc-{index}")),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect::<Vec<_>>();
    engine.write(&writes).map_err(|error| error.to_string())?;
    engine.drain(30_000).map_err(|error| error.to_string())?;
    let count = engine
        .query_i64_col_for_test("SELECT count(*) FROM canonical_nodes")
        .map_err(|error| error.to_string())?;
    if count != [32] {
        return Err(format!("seed has {count:?} canonical rows"));
    }
    Ok(())
}

fn query(engine: &Engine) -> Value {
    let start = Instant::now();
    match engine.search_text_only("needle") {
        Ok(result) => {
            let latency_ns = start.elapsed().as_nanos().max(1) as u64;
            let count = result.results.len();
            let all_bodies_match = result.results.iter().all(|hit| hit.body.contains("needle"));
            let observed = json!({"record_count": count, "all_bodies_match": all_bodies_match});
            let semantic_ok = count == 10 && all_bodies_match;
            if semantic_ok {
                json!({"valid":true,"latency_ns":latency_ns,"semantic_ok":true,
                       "observed_checks":observed})
            } else {
                json!({"valid":false,"reason":"unexpected text search result",
                       "latency_ns":latency_ns,"semantic_ok":false,"observed_checks":observed})
            }
        }
        Err(error) => json!({"valid":false,"reason":format!("search: {error}")}),
    }
}

fn close_fresh() -> Value {
    let dir = TempDir::new().expect("temporary database directory");
    let path = dir.path().join("pilot.fdb");
    let engine = match Engine::open_without_embedder_for_test(&path) {
        Ok(opened) => opened.engine,
        Err(error) => return json!({"valid":false,"reason":format!("open: {error}")}),
    };
    let start = Instant::now();
    let close = engine.close();
    let latency_ns = start.elapsed().as_nanos().max(1) as u64;
    if let Err(error) = close {
        return json!({"valid":false,"reason":format!("close: {error}"),
                      "latency_ns":latency_ns});
    }
    let reopened = Engine::open_without_embedder_for_test(&path);
    match reopened {
        Ok(opened) => {
            let count =
                opened.engine.query_i64_col_for_test("SELECT count(*) FROM canonical_nodes");
            let close_again = opened.engine.close();
            let semantic_ok = count.as_ref().is_ok_and(|rows| rows == &[0]) && close_again.is_ok();
            let observed = json!({"reopen_ok":true,"canonical_rows":count.ok(),
                                  "second_close_ok":close_again.is_ok()});
            if semantic_ok {
                json!({"valid":true,"latency_ns":latency_ns,"semantic_ok":true,
                       "observed_checks":observed})
            } else {
                json!({"valid":false,"reason":"reopen state mismatch",
                       "latency_ns":latency_ns,"semantic_ok":false,"observed_checks":observed})
            }
        }
        Err(error) => json!({"valid":false,"reason":format!("reopen: {error}"),
                             "latency_ns":latency_ns,
                             "semantic_ok":false,"observed_checks":{"reopen_ok":false}}),
    }
}

#[test]
#[ignore = "explicit baseline pilot only"]
fn slice135_pilot() {
    let count: usize = std::env::var("SLICE135_COUNT").unwrap().parse().unwrap();
    assert!(count > 0);
    let dir = TempDir::new().unwrap();
    let engine =
        Engine::open_without_embedder_for_test(dir.path().join("query.fdb")).unwrap().engine;
    seed(&engine).unwrap();
    let _ = query(&engine);
    for _ in 0..count {
        emit("text", query(&engine));
    }
    engine.close().unwrap();
    let _ = close_fresh();
    for _ in 0..count {
        emit("close_fresh", close_fresh());
    }
}
