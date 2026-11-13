//! G-only post-group actuation; public APIs, live current coverage and explicit phases.

use super::{GAdapter, Recorder};
use fathomdb_engine::{InitialState, PreparedWrite, SourceId};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Instant;

pub(super) struct Cohort {
    pub(super) rows: usize,
    pub(super) probes: usize,
}

fn ns(origin: Instant) -> u64 {
    u64::try_from(origin.elapsed().as_nanos()).unwrap()
}

pub(super) fn current_coverage(
    path: &Path,
    expected: &fathomdb_embedder_api::EmbedderIdentity,
) -> Value {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let profile: (String,String,u32) = connection.query_row("SELECT name,revision,dimension FROM _fathomdb_embedder_profiles WHERE profile='default'", [], |row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
    assert_eq!(
        (&profile.0, &profile.1, profile.2),
        (&expected.name, &expected.revision, expected.dimension)
    );
    let rows: Vec<(String,i64,String)> = connection.prepare("SELECT kind,write_cursor,body FROM canonical_nodes WHERE state='active' AND superseded_at IS NULL ORDER BY write_cursor").unwrap().query_map([],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap().collect::<Result<_,_>>().unwrap();
    let mut by_kind = BTreeMap::<String, usize>::new();
    let mut physical_ids = BTreeSet::new();
    for (kind, cursor, body) in &rows {
        let mappings: Vec<i64> = connection
            .prepare("SELECT rowid FROM _fathomdb_vector_rows WHERE kind=?1 AND write_cursor=?2")
            .unwrap()
            .query_map(rusqlite::params![kind, cursor], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(mappings.len(), 1);
        let rowid = mappings[0];
        assert!(physical_ids.insert(rowid));
        let (physical_kind, bytes): (String, usize) = connection
            .query_row(
                "SELECT kind,length(embedding) FROM vector_default WHERE rowid=?1",
                [rowid],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(&physical_kind, kind);
        assert_eq!(bytes, usize::try_from(expected.dimension).unwrap() * 4);
        let spans:Vec<(i64,i64,Option<usize>,usize)>=connection.prepare("SELECT window_number,byte_start,byte_end,body_length FROM _fathomdb_vector_spans WHERE vector_rowid=?1 AND write_cursor=?2").unwrap().query_map(rusqlite::params![rowid,cursor],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).unwrap().collect::<Result<_,_>>().unwrap();
        assert_eq!(spans.len(), 1);
        let (window, start, end, length) = spans[0];
        assert_eq!((window, start, length), (0, 0, body.len()));
        let end = end.expect("pinned BGE successful spans are known");
        assert!(end <= body.len() && body.is_char_boundary(end));
        *by_kind.entry(kind.clone()).or_default() += 1;
    }
    let total: usize =
        connection.query_row("SELECT count(*) FROM vector_default", [], |row| row.get(0)).unwrap();
    let historical:usize=connection.query_row("SELECT count(*) FROM _fathomdb_vector_rows v JOIN canonical_nodes c ON c.kind=v.kind AND c.write_cursor=v.write_cursor JOIN vector_default d ON d.rowid=v.rowid AND d.kind=v.kind WHERE c.superseded_at IS NOT NULL",[],|row|row.get(0)).unwrap();
    let mappings: usize = connection
        .query_row("SELECT count(*) FROM _fathomdb_vector_rows", [], |row| row.get(0))
        .unwrap();
    assert_eq!(total, mappings);
    assert_eq!(physical_ids.len() + historical, total);
    json!({"current_by_kind":by_kind,"current_vector_cursors":physical_ids.len(),"total_physical_vectors":total,"historical_physical_vectors":historical})
}

pub(super) fn admitted_read<T>(
    config: &Value,
    out: &Path,
    phase: &str,
    read: impl FnOnce() -> T,
) -> Result<T, ()> {
    super::budget_clock::phase(config, out, phase)?;
    Ok(read())
}

fn governed_write(
    engine: &fathomdb_sdk::Engine,
    writes: &[PreparedWrite],
    origin: Instant,
    config: &Value,
    out: &Path,
) -> Result<(fathomdb_engine::WriteReceipt, u64, u64), ()> {
    super::budget_clock::phase(config, out, "write")?;
    let start = ns(origin);
    let receipt = engine.write(writes).unwrap();
    let end = ns(origin);
    Ok((receipt, start, end))
}

pub(super) fn reset(
    adapter: &GAdapter,
    path: &Path,
    config: &Value,
    out: &Path,
) -> Result<Value, ()> {
    let GAdapter::Sdk(engine) = adapter else { panic!("G2 reset is SDK only") };
    let writes: Vec<_> = (0..10)
        .map(|slot| PreparedWrite::Node {
            kind: "note".into(),
            body: format!("s30markercurrent{slot} revisionreset"),
            source_id: SourceId::new("test:slice30-g-schedule").unwrap(),
            logical_id: Some(format!("s30marker-{slot}")),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect();
    let writing = Instant::now();
    let (_, start, end) = governed_write(engine, &writes, writing, config, out)?;
    let write_ns = end - start;
    let draining = Instant::now();
    let drain_budget = super::g_budget::drain(config, |timeout| {
        engine.drain(timeout).map_err(|error| format!("{error:?}"))
    })
    .unwrap();
    let drain_ns = ns(draining);
    let pending = admitted_read(config, out, "readiness", || {
        fathomdb_sdk::read::embedding_readiness(engine).unwrap().pending_count
    })?;
    assert_eq!(pending, 0);
    let coverage = admitted_read(config, out, "coverage", || {
        current_coverage(path, &engine.open_report().default_embedder)
    })?;
    assert_eq!(coverage["current_by_kind"]["note"], 10);
    let mut report = json!({"reset_write_calls":1,"reset_rows":writes.len(),"reset_write_ns":write_ns,
        "reset_drain_ns":drain_ns,"pending_after_drain":pending,"coverage":coverage});
    if config["work_deadline_ns"].is_u64() {
        report["drain_budget"] = drain_budget;
    }
    Ok(report)
}

/// Execute one SDK G2 cohort after a completed natural group; return actual rows/probes.
/// Returns Err on deadline refusal; panics on API/coverage/output failure.
/// Already completed phase flushes stay recorded.
pub(super) fn run(
    adapter: &GAdapter,
    path: &Path,
    calls: usize,
    recorder: &mut Recorder,
    origin: Instant,
    config: &Value,
    out: &Path,
) -> Result<Cohort, ()> {
    assert!(calls > 0 && calls.is_multiple_of(10));
    let GAdapter::Sdk(engine) = adapter else {
        panic!("G2 scheduled source population is SDK only")
    };
    let cohort = calls / 10 - 1;
    let writes: Vec<_> = (0..10)
        .map(|slot| PreparedWrite::Node {
            kind: "note".into(),
            body: format!("s30markercurrent{slot} revision{cohort}"),
            source_id: SourceId::new("test:slice30-g-schedule").unwrap(),
            logical_id: Some(format!("s30marker-{slot}")),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect();
    let (receipt, start, end) = governed_write(engine, &writes, origin, config, out)?;
    recorder.emit("batches",&json!({"event":"governed_write","cohort":cohort,"after_natural_calls":calls,
        "rows":writes.len(),"cursor":receipt.cursor,"start_ns":start,"end_ns":end,"write_ns":end-start}));
    recorder.flush_all();
    let start = ns(origin);
    let drain_budget = super::g_budget::drain(config, |timeout| {
        engine.drain(timeout).map_err(|error| format!("{error:?}"))
    })
    .unwrap();
    let end = ns(origin);
    let pending = admitted_read(config, out, "readiness", || {
        fathomdb_sdk::read::embedding_readiness(engine).unwrap().pending_count
    })?;
    assert_eq!(pending, 0);
    let mut drained = json!({"event":"projection_drained","cohort":cohort,"after_natural_calls":calls,
        "start_ns":start,"end_ns":end,"drain_ns":end-start,"pending_after_drain":pending});
    if config["work_deadline_ns"].is_u64() {
        drained["drain_budget"] = drain_budget;
    }
    recorder.emit("batches", &drained);
    recorder.flush_all();
    let mut probes = 0;
    for slot in 0..10 {
        let mut options = fathomdb_sdk::TextSearchOptions::default();
        let mut filter = fathomdb_engine::SearchFilter::default();
        filter.kind = Some("note".into());
        options.filter = Some(filter);
        let (result, start, end) = admitted_read(config, out, "probe", || {
            let start = ns(origin);
            let result =
                engine.search_text_only(&format!("s30markercurrent{slot}"), options).unwrap();
            let end = ns(origin);
            (result, start, end)
        })?;
        let ids: Vec<_> = result.results.iter().map(|hit| hit.id.to_prefixed()).collect();
        let bodies: Vec<_> = result.results.iter().map(|hit| hit.body.clone()).collect();
        let current = ids == [format!("l:s30marker-{slot}")]
            && bodies == [format!("s30markercurrent{slot} revision{cohort}")];
        recorder.emit(
            "marker-probes",
            &json!({"ok":true,"marker_current":current,"cohort":cohort,
            "after_natural_calls":calls,"slot":slot,"ids":ids,"bodies":bodies,
            "start_ns":start,"end_ns":end,"proof_ns":end-start}),
        );
        assert!(current, "literal marker probe failed");
        probes += 1;
    }
    recorder.flush_all();
    let start = ns(origin);
    let mut coverage = admitted_read(config, out, "coverage", || {
        current_coverage(path, &engine.open_report().default_embedder)
    })?;
    let end = ns(origin);
    coverage["event"] = json!("active_coverage");
    coverage["cohort"] = json!(cohort);
    coverage["after_natural_calls"] = json!(calls);
    coverage["start_ns"] = json!(start);
    coverage["end_ns"] = json!(end);
    recorder.emit("batches", &coverage);
    recorder.flush_all();
    Ok(Cohort { rows: writes.len(), probes })
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn actual_write(config: &Value, count: usize, allowed: bool) {
        let root = tempfile::Builder::new().prefix("s30-g2-write-").tempdir().unwrap().keep();
        let store = root.join("store.sqlite");
        let engine = fathomdb_sdk::Engine::open(
            &store,
            fathomdb_sdk::OpenOptions { use_default_embedder: false, ..Default::default() },
        )
        .unwrap();
        let writes: Vec<_> = (0..count)
            .map(|index| PreparedWrite::Node {
                kind: "note".into(),
                body: format!("budgetmarker évidence {index}"),
                logical_id: Some(format!("marker-{index}")),
                source_id: SourceId::new("test:s30-g2-admission").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect();
        let result = governed_write(&engine, &writes, Instant::now(), config, &root);
        let hits = engine.search_text_only("budgetmarker", Default::default()).unwrap();
        engine.close().unwrap();
        let db = Connection::open_with_flags(&store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let actual: usize =
            db.query_row("SELECT count(*) FROM canonical_nodes", [], |row| row.get(0)).unwrap();
        assert_eq!(hits.results.len(), actual, "real SDK/SQLite control mismatch");
        println!(
            "actual G2 write {} allowed={allowed} rows={actual} result={result:?}",
            root.display()
        );
        assert_eq!(
            actual,
            if allowed { count } else { 0 },
            "G2 write admitted after original work/hard/shared endpoint"
        );
        assert_eq!(result.is_ok(), allowed);
    }

    fn expired(key: &str, count: usize) {
        actual_write(&json!({}), count, true);
        let mut config = json!({"work_deadline_ns":u64::MAX,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX});
        config[key] = json!(0);
        actual_write(&config, count, false);
    }

    #[test]
    fn original_work_endpoint_refuses_governed_g2_write() {
        expired("work_deadline_ns", 2);
    }
    #[test]
    fn original_hard_endpoint_refuses_governed_g2_write() {
        expired("hard_deadline_ns", 3);
    }
    #[test]
    fn original_shared_endpoint_refuses_governed_g2_write() {
        expired("shared_deadline_ns", 1);
    }
    proptest! {
        #![proptest_config(ProptestConfig { cases: 4, max_shrink_iters: 0, failure_persistence: None, .. ProptestConfig::default() })]
        #[test]
        fn expired_governed_writes_never_publish_a_partial_cohort(count in 1usize..5, key in prop::sample::select(vec!["work_deadline_ns","hard_deadline_ns","shared_deadline_ns"])) {
            expired(key, count);
        }
    }

    fn actual_read(config: &Value, count: usize, text: bool, allowed: bool) {
        let root = tempfile::Builder::new().prefix("s30-g2-read-").tempdir().unwrap().keep();
        let store = root.join("store.sqlite");
        let engine = fathomdb_sdk::Engine::open(
            &store,
            fathomdb_sdk::OpenOptions { use_default_embedder: false, ..Default::default() },
        )
        .unwrap();
        let writes: Vec<_> = (0..count)
            .map(|index| PreparedWrite::Node {
                kind: "note".into(),
                body: format!("readmarker évidence {index}"),
                logical_id: Some(format!("read-{index}")),
                source_id: SourceId::new("test:s30-g2-read-admission").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect();
        engine.write(&writes).unwrap();
        let db = Connection::open_with_flags(&store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let before: usize =
            db.query_row("SELECT count(*) FROM canonical_nodes", [], |row| row.get(0)).unwrap();
        assert_eq!(before, count);
        assert_eq!(
            engine.search_text_only("readmarker", Default::default()).unwrap().results.len(),
            count
        );
        let mut calls = 0;
        let result = admitted_read(config, &root, if text { "probe" } else { "coverage" }, || {
            calls += 1;
            if text {
                engine.search_text_only("readmarker", Default::default()).unwrap().results.len()
            } else {
                db.query_row("SELECT count(*) FROM canonical_nodes", [], |row| {
                    row.get::<_, usize>(0)
                })
                .unwrap()
            }
        });
        engine.close().unwrap();
        println!(
            "actual G2 read {} text={text} allowed={allowed} calls={calls} result={result:?}",
            root.display()
        );
        assert_eq!(
            calls,
            usize::from(allowed),
            "read invoked after original work/hard/shared endpoint"
        );
        if allowed {
            assert_eq!(result.unwrap(), count);
        } else {
            assert!(result.is_err());
        }
    }

    fn expired_read(key: &str, text: bool, count: usize) {
        actual_read(&json!({}), count, text, true);
        let mut config = json!({"work_deadline_ns":u64::MAX,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX});
        config[key] = json!(0);
        actual_read(&config, count, text, false);
    }

    #[test]
    fn work_endpoint_refuses_actual_marker_probe() {
        expired_read("work_deadline_ns", true, 2);
    }
    #[test]
    fn hard_endpoint_refuses_actual_marker_probe() {
        expired_read("hard_deadline_ns", true, 1);
    }
    #[test]
    fn shared_endpoint_refuses_actual_marker_probe() {
        expired_read("shared_deadline_ns", true, 3);
    }
    #[test]
    fn work_endpoint_refuses_actual_coverage_read() {
        expired_read("work_deadline_ns", false, 3);
    }
    #[test]
    fn hard_endpoint_refuses_actual_coverage_read() {
        expired_read("hard_deadline_ns", false, 2);
    }
    #[test]
    fn shared_endpoint_refuses_actual_coverage_read() {
        expired_read("shared_deadline_ns", false, 1);
    }
    proptest! {
        #![proptest_config(ProptestConfig { cases: 4, max_shrink_iters: 0, failure_persistence: None, .. ProptestConfig::default() })]
        #[test]
        fn read_admission_never_renews_original_endpoints(count in 1usize..5, text in any::<bool>(), key in prop::sample::select(vec!["work_deadline_ns","hard_deadline_ns","shared_deadline_ns"])) {
            expired_read(key, text, count);
        }
    }
}
