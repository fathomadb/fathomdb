//! Private native phase-budget control using the real None-provider database route.

use fathomdb_engine::{EmbedderChoice, Engine};
use serde_json::{json, Value};
use std::path::Path;

#[path = "budget_clock.rs"]
mod clock;

pub(super) fn monotonic_ns() -> Result<u64, String> {
    clock::monotonic_ns()
}

pub(super) fn bounds(config: &Value) -> Result<Option<(u64, u64)>, String> {
    clock::bounds(config)
}

pub(super) fn phase(config: &Value, out: &Path, phase: &str) -> Result<(), ()> {
    clock::phase(config, out, phase)
}

#[cfg(feature = "test-hooks")]
pub(super) fn completion<T>(config: &Value, out: &Path, work: impl FnOnce() -> T) -> Result<T, ()> {
    phase(config, out, "complete")?;
    Ok(work())
}

pub(super) fn query_open(config: &Value, out: &Path) -> Result<(), ()> {
    phase(config, out, "query_open")
}

pub(super) fn admit(config: &Value, out: &Path) -> bool {
    phase(config, out, "entry").is_ok()
}

pub(super) fn run_control(config: &Value, out: &Path) {
    if !admit(config, out) {
        return;
    }
    std::fs::create_dir_all(out).unwrap();
    let path = out.join("store.sqlite");
    let limit = ["work_deadline_ns", "hard_deadline_ns", "shared_deadline_ns"]
        .into_iter()
        .map(|key| config[key].as_u64().unwrap())
        .min()
        .unwrap();
    let report = match monotonic_ns() {
        Ok(now) if now < limit => {
            let engine = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap().engine;
            engine.close().unwrap();
            json!({"open_calls":1,"sqlite_created":path.exists(),"disposition":"executed","admitted_at_ns":now})
        }
        Ok(now) => {
            json!({"open_calls":0,"sqlite_created":false,"disposition":"deadline_refused","observed_at_ns":now})
        }
        Err(error) => {
            json!({"open_calls":0,"sqlite_created":false,"disposition":"clock_unavailable","error":error})
        }
    };
    std::fs::write(out.join("budget.json"), serde_json::to_vec_pretty(&report).unwrap()).unwrap();
}

pub(super) fn drain(engine: &Engine, config: &Value) -> Result<Value, String> {
    let bound = bounds(config)?;
    let started = bound.map(|(now, _)| now);
    let timeout_ms = if let Some((now, limit)) = bound {
        let remaining_ms = limit.saturating_sub(now) / 1_000_000;
        if remaining_ms == 0 {
            return Ok(
                json!({"drain_calls":0,"timeout_ms":0,"start_ns":now,"disposition":"deadline_refused"}),
            );
        }
        remaining_ms
    } else {
        5000
    };
    engine.drain(timeout_ms).map_err(|error| format!("{error:?}"))?;
    Ok(json!({"drain_calls":1,"timeout_ms":timeout_ms,"start_ns":started}))
}

pub(super) fn checked_drain(engine: &Engine, config: &Value, out: &Path) -> Result<Value, String> {
    let (mut report, error) = match drain(engine, config) {
        Ok(report) if report["drain_calls"] == 1 => return Ok(report),
        Ok(report) => {
            (report, "native drain was not invoked before the original deadline".to_owned())
        }
        Err(error) => (json!({"disposition":"phase_failed","error":error.clone()}), error),
    };
    report["phase"] = json!("drain");
    std::fs::create_dir_all(out).unwrap();
    std::fs::write(out.join("budget.json"), serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    Err(error)
}

#[cfg(all(test, target_os = "linux", feature = "test-hooks"))]
mod completion_tests {
    use super::*;
    use fathomdb_engine::{InitialState, PreparedWrite, SourceId};
    use proptest::prelude::*;
    use rusqlite::{Connection, OpenFlags};
    use sha2::{Digest, Sha256};
    use std::cell::Cell;

    fn admission(key: &str) {
        let root = tempfile::Builder::new().prefix("s30-final-read-").tempdir().unwrap().keep();
        let store = root.join("store.sqlite");
        let engine = Engine::open_with_choice(&store, EmbedderChoice::None).unwrap().engine;
        engine
            .write(&[PreparedWrite::Node {
                kind: "doc".into(),
                body: "actual final read é".into(),
                source_id: SourceId::new("test:s30-final-read").unwrap(),
                logical_id: Some("final-control".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .unwrap();
        engine.close().unwrap();
        let calls = Cell::new(0);
        let work = |target: &Path| {
            calls.set(calls.get() + 1);
            let db = Connection::open_with_flags(&store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            let count: usize =
                db.query_row("SELECT count(*) FROM canonical_nodes", [], |row| row.get(0)).unwrap();
            assert_eq!(count, 1);
            let hash: String = Sha256::digest(std::fs::read(&store).unwrap())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            let report = json!({"rows":count,"closed_sha256":hash});
            std::fs::write(target, serde_json::to_vec(&report).unwrap()).unwrap();
            report
        };
        let healthy = completion(&json!({}), &root, || work(&root.join("healthy.json"))).unwrap();
        assert_eq!(healthy["rows"], 1);
        assert_eq!(healthy["closed_sha256"].as_str().unwrap().len(), 64);
        calls.set(0);
        let mut config = json!({"work_deadline_ns":u64::MAX,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX});
        config[key] = json!(0);
        let target = root.join("expired.json");
        let result = completion(&config, &root, || work(&target));
        println!(
            "actual final read {} key={key} calls={} published={}",
            root.display(),
            calls.get(),
            target.exists()
        );
        assert_eq!(
            (calls.get(), target.exists(), result.is_err()),
            (0, false, true),
            "expired completion still reads/hashes/publishes actual store metadata"
        );
        let report: Value =
            serde_json::from_slice(&std::fs::read(root.join("budget.json")).unwrap()).unwrap();
        assert_eq!(report["phase"], "complete");
        assert_eq!(report["disposition"], "deadline_refused");
    }

    #[test]
    fn expired_work_refuses_final_native_reads() {
        admission("work_deadline_ns");
    }
    #[test]
    fn expired_hard_refuses_final_native_reads() {
        admission("hard_deadline_ns");
    }
    #[test]
    fn expired_shared_refuses_final_native_reads() {
        admission("shared_deadline_ns");
    }
    proptest! {
        #![proptest_config(ProptestConfig{cases:3,max_shrink_iters:0,failure_persistence:None,..ProptestConfig::default()})]
        #[test]
        fn final_reads_share_every_original_bound(key in prop::sample::select(vec!["work_deadline_ns","hard_deadline_ns","shared_deadline_ns"])) {admission(key);}
    }
}
