use fathomdb_engine::{EmbedderChoice, Engine};
use proptest::prelude::*;
use proptest::test_runner::RngSeed;
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn digest(path: &Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap()).iter().map(|b| format!("{b:02x}")).collect()
}

fn verify(count: usize, warmups: usize, samples: usize) {
    let root = tempfile::Builder::new().prefix("s30-d-query-").tempdir().unwrap().keep();
    println!("query fixture {} rows={count} warmups={warmups} samples={samples}", root.display());
    Engine::open_with_choice(root.join("register.sqlite"), EmbedderChoice::None)
        .unwrap()
        .engine
        .close()
        .unwrap();
    let records:Vec<_>=(0..count).map(|i|json!({"id":format!("row-{i}"),"kind":if i%2==0 {"doc"}else{"note"},"body":format!("record {i} évidence 🐟")})).collect();
    let config = root.join("cell.json");
    std::fs::write(&config,serde_json::to_vec(&json!({"family":"D_QUERY_FIXTURE","records":records,"warmups":warmups,"samples":samples})).unwrap()).unwrap();
    let out = root.join("out");
    let binary = PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").unwrap())
        .join("release/examples/slice30_gd_diagnostic");
    println!("binary before runtime SHA256 {}", digest(&binary));
    let started = Instant::now();
    let mut child = Command::new(binary)
        .arg(config)
        .arg(&out)
        .stdout(Stdio::from(std::fs::File::create(root.join("stdout.log")).unwrap()))
        .stderr(Stdio::from(std::fs::File::create(root.join("stderr.log")).unwrap()))
        .spawn()
        .unwrap();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() >= Duration::from_secs(150) {
            let _ = child.kill();
            while started.elapsed() < Duration::from_secs(180) {
                if child.try_wait().unwrap().is_some() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            panic!("owned fixture timeout NOT_RED {}", root.display());
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    assert!(
        status.success(),
        "API/setup failed NOT_RED: {}",
        std::fs::read_to_string(root.join("stderr.log")).unwrap()
    );
    let complete: Value =
        serde_json::from_slice(&std::fs::read(out.join("complete.json")).unwrap()).unwrap();
    assert_eq!(complete["complete"], true);
    let proof: Value =
        serde_json::from_slice(&std::fs::read(out.join("template/api-proof.json")).unwrap())
            .unwrap();
    assert_eq!(proof["api_complete"], true);
    assert_eq!(proof["pending"], 0);
    let metadata: Value =
        serde_json::from_slice(&std::fs::read(out.join("template/template.json")).unwrap())
            .unwrap();
    assert_eq!(metadata["coverage"]["verified"], true);
    let source = out.join("template/template.sqlite");
    assert_eq!(metadata["closed_main_sha256"], digest(&source));
    for store in [&source, &out.join("query.sqlite")] {
        let wal = PathBuf::from(format!("{}-wal", store.display()));
        assert!(std::fs::metadata(wal).map_or(true, |m| m.len() == 0));
        let db = Connection::open_with_flags(store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        for table in
            ["canonical_nodes", "vector_default", "_fathomdb_vector_rows", "_fathomdb_vector_spans"]
        {
            assert_eq!(
                db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, usize>(0))
                    .unwrap(),
                count
            );
        }
        let profile:(String,String,u32)=db.query_row("SELECT name,revision,dimension FROM _fathomdb_embedder_profiles WHERE profile='default'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        assert_eq!(profile, ("varying".into(), "perf-gates-dense".into(), 768));
        drop(db);
        let reopened = Engine::open_with_choice(store, EmbedderChoice::None).unwrap().engine;
        assert_eq!(reopened.read_embedding_readiness().unwrap().pending_count, 0);
        reopened.close().unwrap();
    }
    let rows: Vec<Value> = std::fs::read_to_string(out.join("queries.jsonl"))
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert!(!rows.is_empty(), "missing successful ordinary API control NOT_RED");
    let expected: BTreeSet<_> = (0..count).map(|i| format!("l:row-{i}")).collect();
    for row in &rows {
        assert_eq!(row["ok"], true);
        assert_eq!(row["query"], "record");
        let ids: BTreeSet<_> = row["ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap().to_owned())
            .collect();
        assert_eq!(ids, expected, "actual literal API results NOT_RED");
        assert!(row["elapsed_ns"].as_u64().unwrap() > 0);
    }
    println!("actual query and closed/copy/profile/vector controls passed");
    let actual_warmups = rows.iter().filter(|r| r["phase"] == "warmup").count();
    let actual_samples = rows.iter().filter(|r| r["phase"] == "timed").count();
    assert_eq!(
        (actual_warmups, actual_samples),
        (warmups, samples),
        "missing actual D warmup/timed query loop"
    );
    let mut previous = 0;
    for (index, row) in rows.iter().enumerate() {
        let (phase, sample) =
            if index < warmups { ("warmup", index) } else { ("timed", index - warmups) };
        assert_eq!(row["phase"], phase);
        assert_eq!(row["sample_index"], sample);
        let start = row["start_ns"].as_u64().unwrap();
        let end = row["end_ns"].as_u64().unwrap();
        assert!(previous <= start && start < end);
        assert!(row["elapsed_ns"].as_u64().unwrap() <= end - start);
        previous = end;
    }
}

#[test]
fn timed_queries_repeat_without_warmups() {
    verify(3, 0, 4);
}
#[test]
fn warmup_queries_precede_timed_queries() {
    verify(2, 2, 3);
}
proptest! {
    #![proptest_config(ProptestConfig{cases:4,max_shrink_iters:0,failure_persistence:None,rng_seed:RngSeed::Fixed(0x301301),..ProptestConfig::default()})]
    #[test]
    fn bounded_populations_preserve_query_counts_order_and_values(rows in 1usize..6,warmups in 0usize..4,samples in 2usize..7){verify(rows,warmups,samples);}
}

fn writer_fixture(appended: usize, cohort: usize, warmups: usize) {
    writer_case(appended, cohort, warmups, false);
}

fn writer_case(appended: usize, cohort: usize, warmups: usize, readiness: bool) -> PathBuf {
    writer_observation_case(appended, cohort, warmups, readiness, false)
}

fn writer_observation_case(
    appended: usize,
    cohort: usize,
    warmups: usize,
    readiness: bool,
    observer: bool,
) -> PathBuf {
    let root = tempfile::Builder::new().prefix("s30-d-writer-").tempdir().unwrap().keep();
    println!(
        "writer fixture {} appended={appended} cohort={cohort} warmups={warmups}",
        root.display()
    );
    Engine::open_with_choice(root.join("register.sqlite"), EmbedderChoice::None)
        .unwrap()
        .engine
        .close()
        .unwrap();
    let records = vec![
        json!({"id":"seed-a","kind":"doc","body":"record alpha"}),
        json!({"id":"seed-b","kind":"note","body":"record beta"}),
    ];
    let appended_records:Vec<_>=(0..appended).map(|i|json!({"id":format!("tail-{i}"),"kind":"note","body":format!("append evidence {i} é")})).collect();
    let config = root.join("cell.json");
    std::fs::write(&config,serde_json::to_vec(&json!({"family":"D_QUERY_FIXTURE","records":records,"write_records":appended_records,"cohort":cohort,"warmups":warmups,"samples":4,"readiness":readiness,"observer":observer})).unwrap()).unwrap();
    let binary = PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").unwrap())
        .join("release/examples/slice30_gd_diagnostic");
    println!("binary before runtime SHA256 {}", digest(&binary));
    let out = root.join("out");
    let started = Instant::now();
    let mut child = Command::new(binary)
        .arg(config)
        .arg(&out)
        .stdout(Stdio::from(std::fs::File::create(root.join("stdout.log")).unwrap()))
        .stderr(Stdio::from(std::fs::File::create(root.join("stderr.log")).unwrap()))
        .spawn()
        .unwrap();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() >= Duration::from_secs(150) {
            let _ = child.kill();
            while started.elapsed() < Duration::from_secs(180) {
                if child.try_wait().unwrap().is_some() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            panic!("owned writer fixture timeout NOT_RED {}", root.display());
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    assert!(
        status.success(),
        "API/setup failed NOT_RED: {}",
        std::fs::read_to_string(root.join("stderr.log")).unwrap()
    );
    let metadata: Value =
        serde_json::from_slice(&std::fs::read(out.join("template/template.json")).unwrap())
            .unwrap();
    assert_eq!(metadata["coverage"]["verified"], true);
    assert_eq!(metadata["closed_main_sha256"], digest(&out.join("template/template.sqlite")));
    let rows: Vec<Value> = std::fs::read_to_string(out.join("queries.jsonl"))
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(rows.len(), warmups + 4, "existing real query loops NOT_RED");
    let permitted: BTreeSet<_> = ["l:seed-a".to_owned(), "l:seed-b".to_owned()]
        .into_iter()
        .chain((0..appended).map(|i| format!("l:tail-{i}")))
        .collect();
    for row in &rows {
        assert_eq!(row["ok"], true);
        assert!(row["elapsed_ns"].as_u64().unwrap() > 0);
        let ids = row["ids"].as_array().unwrap();
        assert!(!ids.is_empty());
        assert!(ids.iter().all(|id| permitted.contains(id.as_str().unwrap())));
    }
    let path = out.join("writes.json");
    let writes: Vec<Value> = if path.exists() {
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    } else {
        Vec::new()
    };
    println!(
        "existing real query/template controls passed; actual governed cohorts {}",
        writes.len()
    );
    assert_eq!(
        writes.len(),
        appended.div_ceil(cohort),
        "missing actual concurrent D governed writer work"
    );
    let first_timed =
        rows.iter().find(|r| r["phase"] == "timed").unwrap()["start_ns"].as_u64().unwrap();
    let last_warm = rows
        .iter()
        .filter(|r| r["phase"] == "warmup")
        .map(|r| r["end_ns"].as_u64().unwrap())
        .max()
        .unwrap_or(0);
    let mut previous = last_warm;
    let mut done = 0;
    let mut cursors = Vec::new();
    for write in &writes {
        let start = write["start_ns"].as_u64().unwrap();
        let end = write["end_ns"].as_u64().unwrap();
        assert!(previous <= start && start < end);
        assert!(
            write["launch_ns"].as_u64().unwrap() <= first_timed
                && write["launch_ns"].as_u64().unwrap() <= start
        );
        assert_eq!(write["rows"], cohort.min(appended - done));
        let current = write["row_cursors"].as_array().unwrap();
        assert_eq!(current.len(), cohort.min(appended - done));
        cursors.extend(current.iter().map(|v| v.as_u64().unwrap()));
        assert_eq!(write["cursor"], *current.last().unwrap());
        done += current.len();
        previous = end;
    }
    assert_eq!(done, appended);
    let db =
        Connection::open_with_flags(out.join("query.sqlite"), OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let actual:Vec<(String,u64)>=db.prepare("SELECT logical_id,write_cursor FROM canonical_nodes WHERE logical_id LIKE 'tail-%' ORDER BY write_cursor").unwrap().query_map([],|r|Ok((r.get(0)?,r.get(1)?))).unwrap().collect::<Result<_,_>>().unwrap();
    assert_eq!(
        actual,
        (0..appended).map(|i| (format!("tail-{i}"), cursors[i])).collect::<Vec<_>>()
    );
    for table in
        ["canonical_nodes", "vector_default", "_fathomdb_vector_rows", "_fathomdb_vector_spans"]
    {
        assert_eq!(
            db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get::<_, usize>(0))
                .unwrap(),
            2 + appended
        );
    }
    let wal = PathBuf::from(format!("{}-wal", out.join("query.sqlite").display()));
    assert!(std::fs::metadata(wal).map_or(true, |m| m.len() == 0));
    out
}

#[test]
fn governed_writer_runs_beside_timed_queries_after_warmups() {
    writer_fixture(7, 3, 2);
}
#[test]
fn zero_warmup_writer_keeps_partial_final_cohort() {
    writer_fixture(4, 3, 0);
}
proptest! {
    #![proptest_config(ProptestConfig{cases:4,max_shrink_iters:0,failure_persistence:None,rng_seed:RngSeed::Fixed(0x301302),..ProptestConfig::default()})]
    #[test]
    fn bounded_writer_cohorts_preserve_receipts_and_completed_tail(rows in 1usize..9,cohort in 1usize..5,warmups in 0usize..3){writer_fixture(rows,cohort,warmups);}
}

fn verify_readiness(rows: usize, warmups: usize) {
    let out = writer_case(rows, 3, warmups, true);
    let path = out.join("readiness.json");
    let reads: Vec<Value> = if path.exists() {
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    } else {
        Vec::new()
    };
    assert!(!reads.is_empty(), "missing actual concurrent readiness API invocations");
    let queries: Vec<Value> = std::fs::read_to_string(out.join("queries.jsonl"))
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    let writes: Vec<Value> =
        serde_json::from_slice(&std::fs::read(out.join("writes.json")).unwrap()).unwrap();
    let launch = writes[0]["launch_ns"].as_u64().unwrap();
    let last_warm = queries
        .iter()
        .filter(|r| r["phase"] == "warmup")
        .map(|r| r["end_ns"].as_u64().unwrap())
        .max()
        .unwrap_or(0);
    let mut previous_end = last_warm;
    let mut previous_start = None;
    for read in reads {
        assert_eq!(read["ok"], true, "ordinary readiness API success");
        assert!(read["pending_count"].as_u64().unwrap() <= rows as u64);
        let start = read["start_ns"].as_u64().unwrap();
        let end = read["end_ns"].as_u64().unwrap();
        assert!(launch <= start && previous_end <= start && start < end);
        if let Some(prior) = previous_start {
            assert!(start - prior >= 100_000_000, "100ms minimum invocation-start cadence");
        }
        previous_start = Some(start);
        previous_end = end;
    }
}
#[test]
fn actual_readiness_call_is_measured_on_shared_writer_query_origin() {
    verify_readiness(7, 2);
}
proptest! {
    #![proptest_config(ProptestConfig{cases:4,max_shrink_iters:0,failure_persistence:None,rng_seed:RngSeed::Fixed(0x301303),..ProptestConfig::default()})]
    #[test]
    fn readiness_uses_real_counts_and_serial_full_invocation_intervals(rows in 1usize..9,warmups in 0usize..3){verify_readiness(rows,warmups);}
}

fn verify_worker_adapter(count: usize, cohort: usize, enabled: bool) {
    let out = writer_observation_case(count, cohort, 2, true, enabled);
    let path = out.join("worker-observer.json");
    let report: Value = if path.exists() {
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    } else {
        Value::Null
    };
    if !cfg!(feature = "slice30-measurement") || !enabled {
        assert_eq!(report["available"], false, "missing explicit unavailable worker attribution");
        assert_eq!(
            report["reason"],
            if cfg!(feature = "slice30-measurement") {
                "observer_disabled"
            } else {
                "feature_absent"
            }
        );
        for key in ["records", "dropped", "query_records", "query_dropped", "attribution_complete"]
        {
            assert!(
                report[key].is_null(),
                "unavailable attribution must not fabricate zeros {key}"
            );
        }
        return;
    }
    let records = report["records"].as_array().cloned().unwrap_or_default();
    assert_eq!(
        records.iter().filter_map(|r| r["job_count"].as_u64()).sum::<u64>(),
        count as u64,
        "missing actual D worker adapter records after successful workload controls"
    );
    assert_eq!(report["available"], true);
    assert_eq!(report["attribution_complete"], true);
    assert_eq!(report["dropped"], 0);
    assert_eq!(report["query_dropped"], 0);
    assert_eq!(report["capacity"], 32768);
    let queries = report["query_records"].as_array().unwrap();
    assert_eq!(queries.len(), 6);
    assert!(queries.iter().all(|q| q["event"] == "QueryComplete"));
    let engine_id = &queries[0]["engine_id"];
    let raw: Vec<Value> = std::fs::read_to_string(out.join("queries.jsonl"))
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    let end = raw.last().unwrap()["end_ns"].as_u64().unwrap();
    let mut batches = BTreeSet::new();
    let mut actual = Vec::new();
    for record in records {
        assert_eq!(&record["engine_id"], engine_id);
        assert_eq!(record["event"], "ProjectionBatchEnd");
        assert_eq!(record["disposition"], "finished");
        assert_eq!(record["cleanup"], "confirmed");
        assert_eq!(record["commit_outcome"], "ok");
        assert!(record["clock_issue"].is_null());
        assert!(record["worker_idx"].as_u64().unwrap() < 2);
        assert!(batches.insert(record["batch_id"].as_u64().unwrap()));
        let start = record["start_ns"].as_u64().unwrap();
        let stop = record["end_ns"].as_u64().unwrap();
        assert!(start < stop);
        assert!(start >= raw[1]["end_ns"].as_u64().unwrap());
        assert!(
            end <= report["collected_at_ns"].as_u64().unwrap()
                && stop <= report["collected_at_ns"].as_u64().unwrap()
        );
        for cursor in record["cursors"].as_array().unwrap() {
            assert_eq!(cursor["kind"], "note");
            actual.push(cursor["write_cursor"].as_u64().unwrap());
        }
    }
    let writes: Vec<Value> =
        serde_json::from_slice(&std::fs::read(out.join("writes.json")).unwrap()).unwrap();
    let mut expected: Vec<u64> = writes
        .iter()
        .flat_map(|w| w["row_cursors"].as_array().unwrap().iter().map(|c| c.as_u64().unwrap()))
        .collect();
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}
#[test]
fn worker_native_feature_control_reports_unavailability_honestly() {
    verify_worker_adapter(4, 3, false);
}
#[cfg(feature = "slice30-measurement")]
#[test]
fn enabled_worker_adapter_preserves_actual_separate_streams_and_origin() {
    verify_worker_adapter(7, 3, true);
}
#[cfg(feature = "slice30-measurement")]
proptest! {
    #![proptest_config(ProptestConfig{cases:4,max_shrink_iters:0,failure_persistence:None,rng_seed:RngSeed::Fixed(0x301304),..ProptestConfig::default()})]
    #[test]
    fn worker_adapter_conserves_actual_cursors_without_query_stream_mixing(rows in 1usize..9,cohort in 1usize..5){verify_worker_adapter(rows,cohort,true);}
}
