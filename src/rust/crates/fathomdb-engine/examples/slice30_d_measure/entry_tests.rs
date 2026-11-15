use super::entry::run_cell;
use rusqlite::{Connection, OpenFlags};
use serde_json::json;
use std::time::{Duration, Instant};

#[test]
fn entry_routes_ordinary_and_characterized_cells_through_same_canonical_workload() {
    for layer in ["T", "C"] {
        let directory = tempfile::tempdir().unwrap();
        let identity = json!({"cell_id":format!("tiny-{layer}-held-n2-r0"),
            "attempt_id":"entry-one","n":2,"mode":"held","layer":layer,"repeat":0});
        let origin = Instant::now();
        let report =
            run_cell(&identity, directory.path(), origin, origin + Duration::from_secs(60))
                .expect("entry joins and closes its real store");
        assert_eq!(
            report["calls"].as_array().unwrap().len(),
            1000,
            "native entry never routed the canonical500-query body twice"
        );
        assert_eq!(report["cell"], identity);
        assert_eq!(report["complete"], true);
        assert_eq!(report["close_succeeded"], true);
        assert_eq!(report["batches"].as_array().unwrap().len(), 1);
        assert_eq!(report["batches"][0]["rows"], 2);
        assert_eq!(report["runtime"]["scheduler_workers"], 2);
        assert_eq!(report["runtime"]["embed_workers"], 5);
        assert_eq!(report["runtime"]["admission_capacity"], 128);
        assert_eq!(report["final"]["physical_rows"], 2);
        for (index, call) in report["calls"].as_array().unwrap().iter().enumerate() {
            assert_eq!(call["sample_index"], index);
            assert_eq!(call["query_index"], index % 500);
            assert_eq!(call["route"], "engine_hybrid");
            assert_eq!(call["limit"], 10);
            assert_eq!(call["ok"], true);
        }
        assert_eq!(report["collection_enabled"], layer == "C");
        if layer == "C" {
            assert_eq!(report["observation"]["dropped_records"], 0);
            assert_eq!(report["observation"]["issues"], json!([]));
            assert_eq!(report["observation"]["final_take_after_close"], true);
        } else {
            assert_eq!(report["periodic_numeric_polls"], 0);
            assert_eq!(report["periodic_readiness_polls"], 0);
            assert!(report["observation"].is_null());
        }
        let connection = Connection::open_with_flags(
            directory.path().join("store.sqlite"),
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let counts: (u64, u64, u64, u64) = connection.query_row(
            "SELECT (SELECT COUNT(*) FROM canonical_nodes WHERE kind='doc'), (SELECT COUNT(*) FROM vector_default), (SELECT COUNT(*) FROM _fathomdb_vector_spans), (SELECT COUNT(*) FROM _fathomdb_projection_terminal WHERE state='up_to_date')",
            [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        ).unwrap();
        assert_eq!(counts, (2, 2, 2, 2));
    }
}
