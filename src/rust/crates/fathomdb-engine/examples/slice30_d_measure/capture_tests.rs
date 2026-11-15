//! Tiny native C-H stream custody; never a scaled D characterization claim.
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
    BufReader::new(File::open(path.as_str().expect("retained stream path")).unwrap())
        .lines()
        .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
        .collect()
}

#[test]
fn characterized_held_cell_streams_own_preseed_observation_through_closed_dense_rows() {
    let directory = tempfile::tempdir().unwrap();
    let store = directory.path().join("characterized.sqlite");
    let output = directory.path().join("capture");
    let queries = (0..500).map(|index| format!("requestneedle{index}")).collect::<Vec<_>>();
    let bodies = ["corpus needle alpha", "corpus needle beta"].map(str::to_owned);
    let identity = json!({"cell_id":"tiny-c-held-n2-r0","attempt_id":"one",
        "n":2,"repeat":0,"layer":"C","mode":"held"});
    let origin = Instant::now();
    let report = run_characterized(
        OrdinaryCell {
            path: &store,
            mode: Mode::Held,
            queries: &queries,
            origin,
            deadline: origin + Duration::from_secs(60),
        },
        bodies.into_iter(),
        Arc::new(VaryingEmbedder),
        &identity,
        &output,
    )
    .expect("characterized control must stop/join/close before assertions");
    assert_eq!(report["collection_enabled"], true, "native C capture never armed before seeding");
    assert_eq!(report["complete"], true);
    assert_eq!(report["cell"], identity);
    assert_eq!(report["close_succeeded"], true);
    assert_eq!(report["held"]["pending_count"], 2);
    assert_eq!(report["held"]["physical_rows"], 0);
    let observation = &report["observation"];
    assert_eq!(observation["dropped_records"], 0);
    assert_eq!(observation["issues"], json!([]));
    assert_eq!(observation["final_take_after_close"], true);
    assert!(
        observation["helpers_joined_ns"].as_u64().unwrap() <= report["closed_ns"].as_u64().unwrap()
    );
    let engine = observation["engine_id"].as_u64().unwrap();
    let session = observation["session_id"].as_u64().unwrap();
    assert!(engine > 0 && session > 0);
    let connection = Connection::open_with_flags(&store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut statement = connection
        .prepare("SELECT write_cursor FROM canonical_nodes WHERE kind='doc' ORDER BY write_cursor")
        .unwrap();
    let expected = statement
        .query_map([], |row| row.get::<_, u64>(0))
        .unwrap()
        .collect::<rusqlite::Result<BTreeSet<_>>>()
        .unwrap();
    assert_eq!(expected.len(), 2);
    let physical: (u64, u64, u64) = connection.query_row(
        "SELECT (SELECT COUNT(*) FROM vector_default), (SELECT COUNT(*) FROM _fathomdb_vector_spans), (SELECT COUNT(*) FROM _fathomdb_projection_terminal WHERE state='up_to_date')",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).unwrap();
    assert_eq!(physical, (2, 2, 2));
    let mut phases: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();
    let mut operations = BTreeSet::new();
    let mut committed = BTreeSet::new();
    for frame in lines(&observation["records_path"]) {
        assert_eq!(frame["cell"], identity);
        let record = &frame["record"];
        assert_eq!(record["engine_id"], engine);
        assert_eq!(record["session_id"], session);
        if record["phase"] == "api_hybrid" {
            assert_eq!(record["status"], "ok");
            operations.insert(record["operation_id"].as_u64().unwrap());
        }
        if let Some(cursors) = record["facts"]["cursors"].as_array() {
            phases
                .entry(record["phase"].as_str().unwrap().to_owned())
                .or_default()
                .extend(cursors.iter().map(|cursor| cursor.as_u64().unwrap()));
        }
        if record["phase"] == "projection_commit_outcome" {
            assert_eq!(record["facts"]["publication_state"], "committed");
            assert_eq!(record["facts"]["generation_id"], report["final"]["generation_id"]);
            committed.extend(
                record["facts"]["cursors"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|cursor| cursor.as_u64().unwrap()),
            );
        }
    }
    assert_eq!(operations, (1..=1000).collect());
    assert_eq!(committed, expected);
    for phase in [
        "projection_scan",
        "projection_dequeue",
        "projection_processing",
        "projection_embed_dispatch_wall",
        "projection_commit_gate_wait",
        "projection_commit_begin",
        "projection_commit_prepare",
        "projection_commit_publication",
        "projection_commit_sqlite",
        "projection_commit_completion",
    ] {
        assert_eq!(
            phases.get(phase),
            Some(&expected),
            "current worker phase coverage differs: {phase}"
        );
    }
    let first_seed = report["batches"][0]["started_ns"].as_u64().unwrap();
    for path in ["runtime_path", "pending_path", "wal_path"] {
        let samples = lines(&observation[path]);
        assert!(!samples.is_empty(), "required actual C sample stream is missing: {path}");
        assert!(samples[0]["ended_ns"].as_u64().unwrap() <= first_seed);
        for sample in &samples {
            assert_eq!(sample["cell"], identity);
            assert_eq!(sample["engine_id"], engine);
            assert_eq!(sample["session_id"], session);
            assert!(sample["ended_ns"].as_u64().unwrap() >= sample["started_ns"].as_u64().unwrap());
        }
        if path == "runtime_path" {
            for field in ["active_jobs", "active_scans", "queued_jobs", "in_flight_count"] {
                assert_eq!(samples[0]["state"][field], 0);
            }
            assert_eq!(samples[0]["state"]["frozen"], true);
        }
    }
    let probes = report["held_scan_probes"].as_array().unwrap();
    assert_eq!(probes.len(), 2);
    assert_eq!(probes[0]["scan"], "scheduler");
    assert_eq!(probes[1]["scan"], "candidate_page");
    assert_eq!(probes[0]["effective_at_epoch_s"], probes[1]["effective_at_epoch_s"]);
    for probe in probes {
        assert_eq!(probe["rows"].as_array().unwrap().len(), 2);
        assert!(probe["vm_steps"].as_i64().unwrap() > 0);
        assert!(!probe["plan"].as_array().unwrap().is_empty());
        assert!(probe["ended_ns"].as_u64().unwrap() <= report["release_ns"].as_u64().unwrap());
    }
}
