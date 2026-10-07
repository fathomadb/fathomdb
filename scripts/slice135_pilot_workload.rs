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

fn mixed_sequence(attribution: bool) -> Value {
    let dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(error) => return json!({"valid":false,"reason":format!("tempdir: {error}")}),
    };
    let path = dir.path().join("mixed.fdb");
    let whole = Instant::now();
    let mut stages = serde_json::Map::new();
    let mut observed = serde_json::Map::new();
    macro_rules! timed {
        ($name:expr, $expression:expr) => {{
            if attribution {
                let start = Instant::now();
                let result = $expression;
                stages.insert($name.into(), json!(start.elapsed().as_nanos().max(1) as u64));
                result
            } else {
                $expression
            }
        }};
    }
    let outcome = (|| -> Result<(), String> {
        let engine = timed!("open", Engine::open_without_embedder_for_test(&path))
            .map_err(|error| format!("open: {error}"))?
            .engine;
        let fresh = engine
            .query_i64_col_for_test("SELECT count(*) FROM canonical_nodes")
            .map_err(|error| format!("fresh count: {error}"))?;
        observed.insert("fresh_rows".into(), json!(fresh));
        let write = PreparedWrite::Node {
            kind: "doc".into(),
            body: "needle mixed sentinel".into(),
            source_id: SourceId::new("slice135:mixed").unwrap(),
            logical_id: Some("mixed-doc".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        };
        timed!("governed_write", engine.write(&[write]))
            .map_err(|error| format!("write: {error}"))?;
        let written = engine
            .query_i64_col_for_test("SELECT count(*) FROM canonical_nodes")
            .map_err(|error| format!("written count: {error}"))?;
        observed.insert("written_rows".into(), json!(written));
        timed!("projection_drain", engine.drain(30_000))
            .map_err(|error| format!("drain: {error}"))?;
        let before = timed!("text_before_erase", engine.search_text_only("needle"))
            .map_err(|error| format!("search before erasure: {error}"))?;
        observed.insert("pre_erase_hits".into(), json!(before.results.len()));
        observed.insert(
            "pre_erase_body_match".into(),
            json!(before.results.len() == 1 && before.results[0].body == "needle mixed sentinel"),
        );
        timed!("erasure", engine.erase_source("slice135:mixed"))
            .map_err(|error| format!("erasure: {error}"))?;
        let erased = engine
            .query_i64_col_for_test("SELECT count(*) FROM canonical_nodes")
            .map_err(|error| format!("erased count: {error}"))?;
        observed.insert("post_erase_rows".into(), json!(erased));
        timed!("close", engine.close()).map_err(|error| format!("close: {error}"))?;
        let reopened = timed!("reopen", Engine::open_without_embedder_for_test(&path))
            .map_err(|error| format!("reopen: {error}"))?
            .engine;
        let reopen_rows = reopened
            .query_i64_col_for_test("SELECT count(*) FROM canonical_nodes")
            .map_err(|error| format!("reopen count: {error}"))?;
        observed.insert("reopened_rows".into(), json!(reopen_rows));
        let after = timed!("text_after_erase", reopened.search_text_only("needle"))
            .map_err(|error| format!("search after reopen: {error}"))?;
        observed.insert("post_reopen_hits".into(), json!(after.results.len()));
        timed!("reclose", reopened.close()).map_err(|error| format!("reclose: {error}"))?;
        Ok(())
    })();
    let elapsed = whole.elapsed().as_nanos().max(1) as u64;
    let expected = json!({
        "fresh_rows":[0], "written_rows":[1], "pre_erase_hits":1,
        "pre_erase_body_match":true, "post_erase_rows":[0],
        "reopened_rows":[0], "post_reopen_hits":0,
    });
    let semantic_ok = outcome.is_ok() && Value::Object(observed.clone()) == expected;
    let mut attempt = if semantic_ok {
        json!({"valid":true,"latency_ns":elapsed,"semantic_ok":true,
               "observed_checks":observed})
    } else {
        json!({"valid":false,"latency_ns":elapsed,"semantic_ok":false,
               "reason":outcome.err().unwrap_or_else(|| "mixed sequence state mismatch".into()),
               "observed_checks":observed})
    };
    if attribution {
        attempt["stages_ns"] = Value::Object(stages);
    }
    attempt
}

#[test]
#[ignore = "explicit baseline pilot only"]
fn slice135_pilot() {
    let count: usize = std::env::var("SLICE135_COUNT").unwrap().parse().unwrap();
    let attribution = std::env::var("SLICE135_TIMING_MODE").unwrap() == "attribution";
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
    let _ = mixed_sequence(attribution);
    for _ in 0..count {
        emit("mixed_sequence", mixed_sequence(attribution));
    }
}

#[test]
#[ignore = "explicit profiler attachment only; never a latency receipt"]
fn slice135_profile_loop() {
    let seconds: u64 = std::env::var("SLICE135_PROFILE_SECONDS").unwrap().parse().unwrap();
    assert!((1..=60).contains(&seconds));
    let path = std::env::var("SLICE135_PROFILE_PATH").unwrap();
    assert!(matches!(path.as_str(), "text" | "mixed_sequence"));
    let dir = TempDir::new().unwrap();
    let engine = if path == "text" {
        let engine =
            Engine::open_without_embedder_for_test(dir.path().join("profile.fdb")).unwrap().engine;
        seed(&engine).unwrap();
        Some(engine)
    } else {
        None
    };
    eprintln!("PROFILE_READY {path}");
    let until = Instant::now() + std::time::Duration::from_secs(seconds);
    let mut count = 0;
    while Instant::now() < until {
        let attempt = match &engine {
            Some(engine) => query(engine),
            None => mixed_sequence(false),
        };
        assert_eq!(attempt["semantic_ok"], true);
        count += 1;
    }
    if let Some(engine) = engine {
        engine.close().unwrap();
    }
    eprintln!("PROFILE_DONE {path} operations={count}");
}
