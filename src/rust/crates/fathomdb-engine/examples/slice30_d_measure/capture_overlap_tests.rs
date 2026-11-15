//! Tiny C-O capture control; it never requires or transfers an overlap witness.
use super::capture::run_characterized;
use super::provider::VaryingEmbedder;
use super::schedule::{Mode, OrdinaryCell};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn lines(path: &Value) -> Vec<Value> {
    BufReader::new(File::open(path.as_str().unwrap()).unwrap())
        .lines()
        .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
        .collect()
}

#[test]
fn characterized_overlap_streams_current_cohorts_and_groups_without_claiming_a_witness() {
    let directory = tempfile::tempdir().unwrap();
    let store = directory.path().join("overlap.sqlite");
    let output = directory.path().join("capture");
    let queries = (0..500).map(|index| format!("requestneedle{index}")).collect::<Vec<_>>();
    let bodies = (0..1001).map(|index| format!("corpus needle row{index}"));
    let identity = json!({"cell_id":"tiny-c-overlap-n1001-r0","attempt_id":"one",
        "n":1001,"repeat":0,"layer":"C","mode":"overlap"});
    let origin = Instant::now();
    let report = run_characterized(
        OrdinaryCell {
            path: &store,
            mode: Mode::Overlap,
            queries: &queries,
            origin,
            deadline: origin + Duration::from_secs(60),
        },
        bodies,
        Arc::new(VaryingEmbedder),
        &identity,
        &output,
    )
    .expect("C-O must join/close before assertions");
    let retained = directory.keep();
    eprintln!("C-O diagnostic retained streams={}", retained.display());
    if report["complete"] != true {
        eprintln!(
            "C-O incomplete: dropped_records={} issues={} take_count={} engine_id={} session_id={}",
            report["observation"]["dropped_records"],
            report["observation"]["issues"],
            report["observation"]["take_count"],
            report["observation"]["engine_id"],
            report["observation"]["session_id"]
        );
    }
    let connection = Connection::open_with_flags(&store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let counts: (u64, u64, u64, u64) = connection.query_row(
        "SELECT (SELECT COUNT(*) FROM canonical_nodes WHERE kind='doc'), (SELECT COUNT(*) FROM vector_default), (SELECT COUNT(*) FROM _fathomdb_vector_spans), (SELECT COUNT(*) FROM _fathomdb_projection_terminal WHERE state='up_to_date')",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).unwrap();
    assert_eq!(counts, (1001, 1001, 1001, 1001));
    assert_eq!(
        report["collection_enabled"], true,
        "actual C-O capture never armed before its writer"
    );
    assert_eq!(report["complete"], true);
    assert_eq!(report["close_succeeded"], true);
    assert_eq!(report["cell"], identity);
    assert!(report["held"].is_null());
    assert!(report["release_ns"].is_null());
    assert!(
        report["actual_overlap"].is_null(),
        "native capture alone cannot qualify a scientific overlap witness"
    );
    assert_eq!(report["batches"][0]["rows"], 1000);
    assert_eq!(report["batches"][1]["rows"], 1);
    let observation = &report["observation"];
    assert_eq!(observation["dropped_records"], 0);
    assert_eq!(observation["issues"], json!([]));
    assert_eq!(observation["final_take_after_close"], true);
    assert!(
        observation["helpers_joined_ns"].as_u64().unwrap() <= report["closed_ns"].as_u64().unwrap()
    );
    let owner = (&observation["engine_id"], &observation["session_id"]);
    let mut statement = connection
        .prepare("SELECT write_cursor FROM canonical_nodes WHERE kind='doc' ORDER BY write_cursor")
        .unwrap();
    let expected = statement
        .query_map([], |row| row.get::<_, u64>(0))
        .unwrap()
        .collect::<rusqlite::Result<BTreeSet<_>>>()
        .unwrap();
    let mut operations = BTreeSet::new();
    let mut committed = BTreeSet::new();
    let mut worker_groups = BTreeMap::new();
    let mut phases: BTreeMap<u64, BTreeSet<String>> = BTreeMap::new();
    for frame in lines(&observation["records_path"]) {
        assert_eq!(frame["cell"], identity);
        let record = &frame["record"];
        assert_eq!((&record["engine_id"], &record["session_id"]), owner);
        let operation = record["operation_id"].as_u64().unwrap();
        let phase = record["phase"].as_str().unwrap();
        if phase == "api_hybrid" {
            assert_eq!(record["status"], "ok");
            operations.insert(operation);
        }
        if phase.starts_with("projection_") {
            assert_eq!(record["status"], "ok");
            phases.entry(operation).or_default().insert(phase.to_owned());
        }
        if phase == "projection_processing" {
            let cursors = record["facts"]["cursors"]
                .as_array()
                .unwrap()
                .iter()
                .map(|cursor| cursor.as_u64().unwrap())
                .collect::<BTreeSet<_>>();
            assert!(!cursors.is_empty() && cursors.len() <= 64);
            worker_groups.insert(operation, cursors);
        }
        if phase == "projection_commit_outcome" {
            assert_eq!(record["facts"]["publication_state"], "committed");
            assert_eq!(record["facts"]["generation_id"], report["final"]["generation_id"]);
            let cursors = record["facts"]["cursors"].as_array().unwrap();
            assert_eq!(record["facts"]["committed_success"], cursors.len());
            assert_eq!(record["facts"]["committed_failure"], 0);
            committed.extend(cursors.iter().map(|cursor| cursor.as_u64().unwrap()));
        }
    }
    assert_eq!(operations, (1..=1000).collect::<BTreeSet<_>>());
    assert_eq!(committed, expected);
    assert!(!worker_groups.is_empty());
    for (operation, _) in worker_groups {
        for phase in [
            "projection_dequeue",
            "projection_embed_dispatch_wall",
            "projection_commit_gate_wait",
            "projection_commit_begin",
            "projection_commit_prepare",
            "projection_commit_publication",
            "projection_commit_sqlite",
            "projection_commit_completion",
            "projection_commit_outcome",
        ] {
            assert!(
                phases[&operation].contains(phase),
                "current group lacks required phase: {phase}"
            );
        }
    }
    let first_seed = report["batches"][0]["started_ns"].as_u64().unwrap();
    for path in ["runtime_path", "pending_path", "wal_path"] {
        let samples = lines(&observation[path]);
        assert!(!samples.is_empty());
        assert!(samples[0]["ended_ns"].as_u64().unwrap() <= first_seed);
        for sample in &samples {
            assert_eq!(sample["cell"], identity);
            assert_eq!((&sample["engine_id"], &sample["session_id"]), owner);
        }
        if path == "runtime_path" {
            assert_eq!(samples[0]["state"]["frozen"], false);
            assert_eq!(samples[0]["state"]["pending_scan"], false);
        }
    }
}
