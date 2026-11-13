use fathomdb_engine::{EmbedderChoice, Engine};
use proptest::prelude::*;
use proptest::test_runner::RngSeed;
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn digest(path: &Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn inspect(store: &Path, records: &[Value]) -> Vec<Value> {
    let connection = Connection::open_with_flags(store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let canonical: Vec<(String,String,u64,String)> = connection.prepare("SELECT logical_id,kind,write_cursor,body FROM canonical_nodes WHERE superseded_at IS NULL ORDER BY write_cursor").unwrap().query_map([], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).unwrap().collect::<Result<_,_>>().unwrap();
    assert_eq!(canonical.len(), records.len());
    let mut actual = Vec::new();
    for ((id, kind, cursor, body), expected) in canonical.iter().zip(records) {
        assert_eq!(
            (id.as_str(), kind.as_str(), body.as_str()),
            (
                expected["id"].as_str().unwrap(),
                expected["kind"].as_str().unwrap(),
                expected["body"].as_str().unwrap()
            )
        );
        let (rowid, bytes, window, start, end, body_bytes): (u64,usize,u64,u64,Option<u64>,usize) = connection.query_row(
            "SELECT v.rowid,length(d.embedding),s.window_number,s.byte_start,s.byte_end,s.body_length FROM _fathomdb_vector_rows v JOIN vector_default d ON d.rowid=v.rowid JOIN _fathomdb_vector_spans s ON s.vector_rowid=v.rowid AND s.write_cursor=v.write_cursor WHERE v.kind=?1 AND v.write_cursor=?2", rusqlite::params![kind,cursor], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?))).unwrap();
        assert!(rowid > 0);
        assert_eq!((bytes, window, start, end, body_bytes), (3072, 0, 0, None, body.len()));
        actual.push(json!({"id":id,"kind":kind,"write_cursor":cursor,"body_bytes":body.len()}));
    }
    for table in
        ["canonical_nodes", "vector_default", "_fathomdb_vector_rows", "_fathomdb_vector_spans"]
    {
        assert_eq!(
            connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row
                    .get::<_, usize>(0))
                .unwrap(),
            records.len()
        );
    }
    let profile: (String,String,u32) = connection.query_row("SELECT name,revision,dimension FROM _fathomdb_embedder_profiles WHERE profile='default'", [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
    assert_eq!(profile, ("varying".into(), "perf-gates-dense".into(), 768));
    actual
}

fn verify(records: Vec<Value>, cohort: usize, mode: &str) {
    verify_case(records, cohort, mode, false);
}

fn verify_case(records: Vec<Value>, cohort: usize, mode: &str, with_queries: bool) {
    verify_observation_case(records, cohort, mode, with_queries, false);
}

fn verify_observation_case(
    records: Vec<Value>,
    cohort: usize,
    mode: &str,
    with_queries: bool,
    sample_readiness: bool,
) {
    verify_adapter_case(records, cohort, mode, with_queries, sample_readiness, None);
}

fn verify_adapter_case(
    records: Vec<Value>,
    cohort: usize,
    mode: &str,
    with_queries: bool,
    sample_readiness: bool,
    observer: Option<bool>,
) {
    let root = tempfile::Builder::new().prefix("s30-held-").tempdir().unwrap().keep();
    println!("held proof: {} rows={} cohort={cohort} mode={mode}", root.display(), records.len());
    Engine::open_with_choice(root.join("registration.sqlite"), EmbedderChoice::None)
        .unwrap()
        .engine
        .close()
        .unwrap();
    let config = root.join("cell.json");
    let queries = with_queries.then(|| vec![json!({"id":"held-query","query":"heldwitness"})]);
    std::fs::write(&config, serde_json::to_vec(&json!({"family":"D_HELD_FIXTURE","records":records,"cohort":cohort,"release_mode":mode,"queries":queries,"warmups":2,"samples":3,"readiness":sample_readiness,"observer":observer.unwrap_or(false)})).unwrap()).unwrap();
    let out = root.join("out");
    let binary = PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").unwrap())
        .join("release/examples/slice30_gd_diagnostic");
    println!("binary before runtime SHA256 {}", digest(&binary));
    let started = Instant::now();
    let mut child = Command::new(binary)
        .arg(&config)
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
            panic!("owned held fixture work deadline exceeded; NOT_RED; proof {}", root.display());
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    assert!(
        status.success(),
        "held API/control fixture failed NOT_RED: {}",
        std::fs::read_to_string(root.join("stderr.log")).unwrap()
    );
    let proof: Value =
        serde_json::from_slice(&std::fs::read(out.join("controls.json")).unwrap()).unwrap();
    assert_eq!(proof["api_complete"], true);
    assert_eq!(
        (
            proof["configured_projection_workers"].as_u64(),
            proof["configured_provider_workers"].as_u64()
        ),
        (Some(2), Some(5))
    );
    assert_eq!(proof["release_mode"], mode);
    assert_eq!(proof["release_requests"], 1);
    let empty = &proof["empty"];
    let idle = &proof["idle"];
    for phase in [empty, idle] {
        assert_eq!(phase["readiness"]["pending_count"], 0);
        assert_eq!(phase["readiness"]["usable_embedder"], true);
        for field in ["canonical_rows", "physical_vectors", "mapped_vectors", "span_rows"] {
            assert_eq!(phase["sql"][field], 0);
        }
        assert_eq!(
            phase["sql"]["profile"],
            json!({"name":"varying","revision":"perf-gates-dense","dimension":768})
        );
    }
    let count = records.len();
    let held = &proof["held"];
    let released = &proof["released"];
    assert_eq!(held["readiness"]["pending_count"], count);
    assert_eq!(held["readiness"]["state"], "processing");
    assert_eq!(held["readiness"]["usable_embedder"], true);
    assert!(held["readiness"]["blocked"].is_null());
    assert_eq!(released["readiness"]["pending_count"], 0);
    assert_eq!(released["readiness"]["state"], "ready");
    for phase in [held, released] {
        assert_eq!(phase["sql"]["canonical_rows"], count);
    }
    for key in ["physical_vectors", "mapped_vectors", "span_rows"] {
        assert_eq!(held["sql"][key], 0);
        assert_eq!(released["sql"][key], count);
    }
    let writes = proof["writes"].as_array().unwrap();
    assert_eq!(writes.len(), count.div_ceil(cohort));
    let mut receipts = Vec::new();
    let mut done = 0;
    for write in writes {
        let expected = cohort.min(count - done);
        assert_eq!(write["rows"], expected);
        let rows = write["row_cursors"].as_array().unwrap();
        assert_eq!(rows.len(), expected);
        receipts.extend(rows.iter().cloned());
        assert_eq!(write["cursor"], *rows.last().unwrap());
        done += expected;
    }
    assert_eq!(done, count);
    let store = out.join("store.sqlite");
    let expected = inspect(&store, &records);
    assert_eq!(released["sql"]["records"], Value::Array(expected.clone()));
    assert_eq!(held["sql"]["records"], Value::Array(expected.clone()));
    assert_eq!(
        receipts,
        expected.iter().map(|row| row["write_cursor"].clone()).collect::<Vec<_>>()
    );
    assert_eq!(proof["coverage"]["verified"], true);
    assert_eq!(proof["coverage"]["rows"].as_array().unwrap().len(), count);
    let mut order = vec![
        &proof["open"],
        &proof["configure"],
        empty,
        &proof["freeze_request"],
        &proof["empty_idle_drain"],
        idle,
    ];
    order.extend(writes.iter().map(|row| &row["clock"]));
    order.extend([
        held,
        &proof["release_request"],
        &proof["released_drain"],
        released,
        &proof["coverage_time"],
        &proof["close"],
    ]);
    let mut previous = 0;
    for clock in order {
        let start = clock["start_ns"].as_u64().unwrap();
        let end = clock["end_ns"].as_u64().unwrap();
        assert!(previous <= start && start < end, "actual shared attempt ordering failed NOT_RED");
        previous = end;
    }
    let wal = PathBuf::from(format!("{}-wal", store.display()));
    assert!(std::fs::metadata(wal).map_or(true, |metadata| metadata.len() == 0));
    let hash = digest(&store);
    assert_eq!(proof["closed_main_sha256"], hash);
    let directory = root.join("fresh-copy");
    std::fs::create_dir(&directory).unwrap();
    let copy = directory.join("store.sqlite");
    std::fs::copy(&store, &copy).unwrap();
    assert_eq!(digest(&copy), hash);
    let reopened = Engine::open_with_choice(&copy, EmbedderChoice::None).unwrap().engine;
    assert_eq!(reopened.read_embedding_readiness().unwrap().pending_count, 0);
    assert_eq!(inspect(&copy, &records), expected);
    reopened.close().unwrap();
    assert_eq!(digest(&store), hash);
    println!(
        "ordinary empty/freeze/idle/write/held/release/drain/SQL/span/close/copy controls passed"
    );
    let report_path = out.join("phases.json");
    let report: Value = if report_path.exists() {
        serde_json::from_slice(&std::fs::read(&report_path).unwrap()).unwrap()
    } else {
        Value::Null
    };
    let phases = report.as_array().cloned().unwrap_or_default();
    assert_eq!(
        phases.len(),
        2,
        "missing new held/released phase reports after successful ordinary controls"
    );
    for (row, (phase, observation, pending, vectors)) in
        phases.iter().zip([("held", held, count, 0usize), ("released", released, 0usize, count)])
    {
        assert_eq!(row["phase"], phase);
        assert_eq!(row["start_ns"], observation["start_ns"]);
        assert_eq!(row["end_ns"], observation["end_ns"]);
        assert_eq!(row["pending_count"], pending);
        assert_eq!(row["physical_vectors"], vectors);
        assert_eq!(row["canonical_rows"], count);
        assert_eq!(row["mapped_vectors"], vectors);
        assert_eq!(row["span_rows"], vectors);
        assert_eq!(row["profile"], observation["sql"]["profile"]);
    }
    assert!(
        phases[0]["end_ns"].as_u64().unwrap()
            <= proof["release_request"]["start_ns"].as_u64().unwrap()
    );
    assert!(
        proof["released_drain"]["end_ns"].as_u64().unwrap()
            <= phases[1]["start_ns"].as_u64().unwrap()
    );

    if with_queries {
        let path = out.join("queries.jsonl");
        let calls: Vec<Value> = if path.exists() {
            std::fs::read_to_string(path)
                .unwrap()
                .lines()
                .map(|s| serde_json::from_str(s).unwrap())
                .collect()
        } else {
            Vec::new()
        };
        assert_eq!(calls.len(), 5, "missing actual held-state warmup/timed queries before release");
        let expected: std::collections::BTreeSet<_> =
            records.iter().map(|r| format!("l:{}", r["id"].as_str().unwrap())).collect();
        let mut previous = held["end_ns"].as_u64().unwrap();
        for (index, call) in calls.iter().enumerate() {
            assert_eq!(call["ok"], true);
            assert_eq!(call["query_id"], "held-query");
            assert_eq!(call["query"], "heldwitness");
            assert_eq!(call["phase"], if index < 2 { "warmup" } else { "timed" });
            assert_eq!(call["sample_index"], if index < 2 { index } else { index - 2 });
            let ids: std::collections::BTreeSet<_> = call["ids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect();
            assert_eq!(ids, expected, "actual held text visibility, no drained-vector claim");
            let start = call["start_ns"].as_u64().unwrap();
            let end = call["end_ns"].as_u64().unwrap();
            assert!(previous <= start && start < end && call["elapsed_ns"].as_u64().unwrap() > 0);
            previous = end;
        }
        let after = &proof["after_queries"];
        assert!(
            previous <= after["start_ns"].as_u64().unwrap()
                && after["end_ns"].as_u64().unwrap()
                    <= proof["release_request"]["start_ns"].as_u64().unwrap()
        );
        assert_eq!(after["readiness"]["pending_count"], records.len());
        assert_eq!(after["sql"]["physical_vectors"], 0);
        if sample_readiness {
            let path = out.join("readiness.json");
            let reads: Vec<Value> = if path.exists() {
                serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
            } else {
                Vec::new()
            };
            assert!(!reads.is_empty(), "missing actual held readiness sampling load");
            let mut previous_end = calls[1]["end_ns"].as_u64().unwrap();
            let mut previous_start = None;
            for read in reads {
                assert_eq!(read["ok"], true);
                assert_eq!(read["pending_count"], records.len());
                let start = read["start_ns"].as_u64().unwrap();
                let end = read["end_ns"].as_u64().unwrap();
                assert!(
                    previous_end <= start
                        && start < end
                        && end <= after["start_ns"].as_u64().unwrap()
                );
                if let Some(prior) = previous_start {
                    assert!(start - prior >= 100_000_000);
                }
                previous_start = Some(start);
                previous_end = end;
            }
        }
    }
    if let Some(enabled) = observer {
        let path = out.join("worker-observer.json");
        let report: Value = if path.exists() {
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
        } else {
            Value::Null
        };
        if !cfg!(feature = "slice30-measurement") || !enabled {
            assert_eq!(report["available"], false, "missing held unavailable worker attribution");
            assert_eq!(
                report["reason"],
                if cfg!(feature = "slice30-measurement") {
                    "observer_disabled"
                } else {
                    "feature_absent"
                }
            );
            for key in
                ["records", "dropped", "query_records", "query_dropped", "attribution_complete"]
            {
                assert!(report[key].is_null());
            }
        } else {
            let workers = report["records"].as_array().cloned().unwrap_or_default();
            assert_eq!(
                workers.iter().filter_map(|w| w["job_count"].as_u64()).sum::<u64>(),
                records.len() as u64,
                "missing actual held-release worker adapter records"
            );
            assert_eq!(report["available"], true);
            assert_eq!(report["attribution_complete"], true);
            assert_eq!(report["dropped"], 0);
            assert_eq!(report["query_dropped"], 0);
            let queries = report["query_records"].as_array().unwrap();
            assert_eq!(queries.len(), 5);
            assert!(queries.iter().all(|q| q["event"] == "QueryComplete"));
            let mut actual = Vec::new();
            for worker in workers {
                assert_eq!(worker["event"], "ProjectionBatchEnd");
                assert_eq!(worker["engine_id"], queries[0]["engine_id"]);
                assert!(worker["clock_issue"].is_null());
                assert_eq!(worker["cleanup"], "confirmed");
                assert_eq!(worker["commit_outcome"], "ok");
                assert!(
                    worker["start_ns"].as_u64().unwrap()
                        >= proof["release_request"]["start_ns"].as_u64().unwrap()
                );
                assert!(
                    worker["end_ns"].as_u64().unwrap()
                        <= report["collected_at_ns"].as_u64().unwrap()
                );
                actual.extend(
                    worker["cursors"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|c| c["write_cursor"].as_u64().unwrap()),
                );
            }
            let mut expected: Vec<_> = receipts.iter().map(|c| c.as_u64().unwrap()).collect();
            actual.sort_unstable();
            expected.sort_unstable();
            assert_eq!(actual, expected);
        }
    }
}

fn literal() -> Vec<Value> {
    vec![
        json!({"id":"alpha","kind":"doc","body":"alpha é witness"}),
        json!({"id":"beta","kind":"note","body":"beta β evidence"}),
        json!({"id":"gamma","kind":"doc","body":"gamma 🐟 record"}),
    ]
}

#[test]
fn literal_explicit_release_reports_real_held_then_drained_phases() {
    verify(literal(), 3, "explicit");
}

#[test]
fn literal_drop_release_reports_real_held_then_drained_phases() {
    verify(literal(), 2, "drop");
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 4, max_shrink_iters: 0, failure_persistence: None, rng_seed: RngSeed::Fixed(0x303012), .. ProptestConfig::default() })]
    #[test]
    fn bounded_unicode_cohorts_preserve_completed_tail_and_phase_order(rows in prop::collection::vec((any::<bool>(),1usize..5),1..13), cohort in 1usize..9, drop_release in any::<bool>()) {
        let records = rows.iter().enumerate().map(|(index,(note,repeats))|json!({"id":format!("row-{index}"),"kind":if *note {"note"} else {"doc"},"body":format!("row {index} {}", "évidence β ".repeat(*repeats))})).collect();
        verify(records,cohort,if drop_release {"drop"}else{"explicit"});
    }
}

#[test]
fn held_queries_run_before_explicit_release_with_real_text_visibility() {
    verify_case(
        vec![
            json!({"id":"a","kind":"doc","body":"heldwitness alpha"}),
            json!({"id":"b","kind":"note","body":"heldwitness beta é"}),
        ],
        2,
        "explicit",
        true,
    );
}
proptest! {
    #![proptest_config(ProptestConfig{cases:4,max_shrink_iters:0,failure_persistence:None,rng_seed:RngSeed::Fixed(0x301308),..ProptestConfig::default()})]
    #[test]
    fn held_query_population_preserves_pending_then_drained_order(rows in 1usize..9,cohort in 1usize..5,drop_release in any::<bool>()){
        let records=(0..rows).map(|i|json!({"id":format!("held-{i}"),"kind":if i%2==0{"doc"}else{"note"},"body":format!("heldwitness {i} évidence 🐟")})).collect();
        verify_case(records,cohort,if drop_release{"drop"}else{"explicit"},true);
    }
}

#[test]
fn held_readiness_load_uses_actual_counts_and_shared_query_clock() {
    verify_observation_case(
        vec![json!({"id":"held","kind":"doc","body":"heldwitness é"})],
        1,
        "drop",
        true,
        true,
    );
}
proptest! {
    #![proptest_config(ProptestConfig{cases:4,max_shrink_iters:0,failure_persistence:None,rng_seed:RngSeed::Fixed(0x301309),..ProptestConfig::default()})]
    #[test]
    fn held_readiness_preserves_full_serial_invocations_before_release(rows in 1usize..9,cohort in 1usize..5,drop_release in any::<bool>()){
        let records=(0..rows).map(|i|json!({"id":format!("ready-{i}"),"kind":if i%2==0{"doc"}else{"note"},"body":format!("heldwitness {i} é")})).collect();
        verify_observation_case(records,cohort,if drop_release{"drop"}else{"explicit"},true,true);
    }
}

fn held_adapter_fixture(rows: usize, enabled: bool) {
    let records=(0..rows).map(|i|json!({"id":format!("observe-{i}"),"kind":if i%2==0{"doc"}else{"note"},"body":format!("heldwitness {i} é")})).collect();
    verify_adapter_case(records, 3, "drop", true, true, Some(enabled));
}
#[test]
fn held_worker_feature_control_reports_unavailable() {
    held_adapter_fixture(4, false);
}
#[cfg(feature = "slice30-measurement")]
#[test]
fn held_worker_adapter_preserves_post_release_jobs_and_held_queries() {
    held_adapter_fixture(7, true);
}
#[cfg(feature = "slice30-measurement")]
proptest! {
    #![proptest_config(ProptestConfig{cases:4,max_shrink_iters:0,failure_persistence:None,rng_seed:RngSeed::Fixed(0x301310),..ProptestConfig::default()})]
    #[test]
    fn held_worker_adapter_conserves_actual_cursors_after_release(rows in 1usize..9){held_adapter_fixture(rows,true);}
}
