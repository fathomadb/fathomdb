//! Shared private GG measured caller; explicit query/view, same API and observer clocks.

use super::{g_observer, CoreObserver, Recorder};
use fathomdb_engine::ReadView;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Instant;

pub(super) fn drain(engine: &fathomdb_engine::Engine, config: &Value) -> Result<Value, String> {
    super::g_budget::drain(config, |timeout| {
        engine.drain(timeout).map_err(|error| format!("{error:?}"))
    })
}

fn measured_call(
    engine: &fathomdb_engine::Engine,
    config: &Value,
    out: &Path,
    phase: &str,
    as_of: i64,
    query: &str,
) -> Result<Value, ()> {
    super::budget_clock::phase(config, out, phase)?;
    let started = Instant::now();
    let view = ReadView { valid_as_of: Some(as_of), ..ReadView::default() };
    let result = engine
        .search_reranked_view_with_limit(query, None, 0, false, 0.3, 0, false, &view, 10)
        .unwrap();
    let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
    let ids: Vec<_> = result.results.iter().map(|hit| hit.id.to_prefixed()).collect();
    Ok(json!({"adapter":"engine","mode":"hybrid","ok":true,"elapsed_ns":elapsed_ns,"ids":ids,
        "returned_hits":result.results.len(),"soft_fallback":format!("{:?}",result.soft_fallback),
        "degradations":format!("{:?}",result.degradations)}))
}

pub(super) fn measure_stage(
    engine: &fathomdb_engine::Engine,
    config: &Value,
    directory: &Path,
    stage: (&str, i64, &str),
    observer: CoreObserver,
) -> bool {
    let (stage, as_of, query) = stage;
    let out = directory.join(stage);
    std::fs::create_dir(&out).unwrap();
    let warmups = usize::try_from(config["warmups"].as_u64().unwrap()).unwrap();
    let samples = usize::try_from(config["samples"].as_u64().unwrap()).unwrap();
    assert!(samples > 0);
    let mut observation = g_observer::Observation::new(engine, observer, true);
    let mut recorder = Recorder::new(&out);
    let loop_started = Instant::now();
    let mut calls = 0;
    for (stream, count) in [("warmups", warmups), ("samples", samples)] {
        for sample in 0..count {
            let fingerprint: String =
                Sha256::digest(query.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect();
            let phase = if stream == "warmups" { "warmup" } else { "timed" };
            let Ok(mut row) = measured_call(engine, config, &out, phase, as_of, query) else {
                recorder.flush_all();
                observer.0(engine, false).unwrap();
                return false;
            };
            row["stage"] = json!(stage);
            row["valid_as_of"] = json!(as_of);
            row["query_id"] = json!(0);
            row["query_fingerprint"] = json!(fingerprint);
            row["sample_index"] = json!(sample);
            row["call_index"] = json!(calls);
            row["observer"] = observation.complete(true);
            recorder.emit(stream, &row);
            calls += 1;
        }
        recorder.flush_all();
    }
    let loop_ns = u64::try_from(loop_started.elapsed().as_nanos()).unwrap();
    let disabling = Instant::now();
    observer.0(engine, false).unwrap();
    let disable_ns = u64::try_from(disabling.elapsed().as_nanos()).unwrap();
    let proof = json!({"stage":stage,"valid_as_of":as_of,"api_calls":calls,"warmup_calls":warmups,"timed_calls":samples,
        "loop_ns":loop_ns,"observer_setup_ns":observation.setup_ns,"observer_disable_ns":disable_ns,
        "raw_persistence":recorder.statistics_since([0;5]),
        "throughput_scope":"stage warmup and timed APIs plus validation/observer collection/recording; literal controls/write/drain/open/close and observer toggles separate"});
    std::fs::write(out.join("api-proof.json"), serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
    true
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use fathomdb_engine::{EmbedderChoice, Engine, InitialState, PreparedWrite, SourceId};
    use proptest::prelude::*;

    fn actual_drain(milliseconds: u64) {
        let root = tempfile::Builder::new().prefix("s30-gg-drain-").tempdir().unwrap().keep();
        let store = root.join("store.sqlite");
        let engine = Engine::open_with_choice(&store, EmbedderChoice::None).unwrap().engine;
        engine
            .write(&[PreparedWrite::Node {
                kind: "doc".into(),
                body: "ggdrain épreuve".into(),
                logical_id: Some("gg-drain".into()),
                source_id: SourceId::new("test:s30-gg-drain").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .unwrap();
        let deadline =
            super::super::budget_clock::monotonic_ns().unwrap() + milliseconds * 1_000_000;
        let config = json!({"work_deadline_ns":deadline,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX});
        let result = drain(&engine, &config);
        assert_eq!(
            engine
                .search_text_only_filtered_with_snippets(
                    "ggdrain",
                    None,
                    &ReadView::default(),
                    10,
                    None
                )
                .unwrap()
                .results
                .len(),
            1
        );
        engine.close().unwrap();
        let db = rusqlite::Connection::open_with_flags(
            &store,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM canonical_nodes", [], |row| row.get::<_, usize>(0))
                .unwrap(),
            1
        );
        println!(
            "actual GG drain {} milliseconds={milliseconds} result={result:?}",
            root.display()
        );
        if milliseconds == 0 {
            assert!(result.is_err(), "expired GG drain invoked the actual API");
        } else {
            let report = result.unwrap();
            let expected = (deadline - report["start_ns"].as_u64().unwrap()) / 1_000_000;
            assert_eq!(
                report["timeout_ms"], expected,
                "GG drain renewed a fixed5000ms instead of original remaining budget"
            );
            assert_eq!(report["drain_calls"], 1);
        }
    }

    #[test]
    fn gg_drain_forwards_actual_original_remaining_timeout() {
        actual_drain(1500);
    }
    #[test]
    fn expired_gg_drain_does_not_invoke_api() {
        actual_drain(0);
    }
    proptest! {
        #![proptest_config(ProptestConfig { cases: 4, max_shrink_iters: 0, failure_persistence: None, .. ProptestConfig::default() })]
        #[test]
        fn gg_drains_never_renew_the_original_work_window(milliseconds in 50u64..2000) { actual_drain(milliseconds); }
    }

    fn actual_call(config: &Value, query: &str, as_of: i64, allowed: bool) {
        let root = tempfile::Builder::new().prefix("s30-gg-call-").tempdir().unwrap().keep();
        let store = root.join("store.sqlite");
        let engine = Engine::open_with_choice(&store, EmbedderChoice::None).unwrap().engine;
        engine
            .write(&[PreparedWrite::Node {
                kind: "doc".into(),
                body: "ggquery épreuve".into(),
                logical_id: Some("gg-query".into()),
                source_id: SourceId::new("test:s30-gg-call").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .unwrap();
        assert_eq!(
            engine
                .search_text_only_filtered_with_snippets(
                    "ggquery",
                    None,
                    &ReadView::default(),
                    10,
                    None
                )
                .unwrap()
                .results
                .len(),
            1
        );
        let healthy = measured_call(&engine, &json!({}), &root, "timed", as_of, query).unwrap();
        assert_eq!(healthy["ok"], true);
        assert!(healthy["elapsed_ns"].as_u64().unwrap() > 0);
        assert!(healthy["ids"].as_array().unwrap().iter().all(|id| id == "l:gg-query"));
        let result = measured_call(&engine, config, &root, "timed", as_of, query);
        engine.close().unwrap();
        let db = rusqlite::Connection::open_with_flags(
            &store,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM canonical_nodes", [], |row| row.get::<_, usize>(0))
                .unwrap(),
            1
        );
        println!("actual GG call {} allowed={allowed} result={result:?}", root.display());
        assert_eq!(
            result.is_ok(),
            allowed,
            "GG query admitted beyond original work/hard/shared endpoint"
        );
    }

    fn expired_call(key: &str, query: &str, as_of: i64) {
        let mut config = json!({"work_deadline_ns":u64::MAX,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX});
        config[key] = json!(0);
        actual_call(&config, query, as_of, false);
    }

    #[test]
    fn work_endpoint_refuses_next_gg_query() {
        expired_call("work_deadline_ns", "ggquery", 100);
    }
    #[test]
    fn hard_endpoint_refuses_next_gg_query() {
        expired_call("hard_deadline_ns", "ggquery", 80);
    }
    #[test]
    fn shared_endpoint_refuses_next_gg_query() {
        expired_call("shared_deadline_ns", "nomatch", 120);
    }
    proptest! {
        #![proptest_config(ProptestConfig { cases: 4, max_shrink_iters: 0, failure_persistence: None, .. ProptestConfig::default() })]
        #[test]
        fn gg_query_admission_does_not_depend_on_query_or_view(query in prop::sample::select(vec!["ggquery","nomatch"]), as_of in 80i64..121, key in prop::sample::select(vec!["work_deadline_ns","hard_deadline_ns","shared_deadline_ns"])) {
            expired_call(key, query, as_of);
        }
    }
}
