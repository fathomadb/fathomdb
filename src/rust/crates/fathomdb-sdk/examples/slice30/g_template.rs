//! Normal Default-BGE closed preparation; bounded source fixtures precede campaigns.

use fathomdb_embedder::EffectiveEmbedDevice;
use fathomdb_engine::{
    EmbedderChoice, Engine, InitialState, PreparedWrite, ProjectionRole, ProjectionSpec,
    ProjectionVector, SourceId,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{BufRead, Read};
use std::path::Path;

#[path = "../../../fathomdb-engine/examples/slice30/template_verify.rs"]
mod template_verify;

fn hash(path: &Path) -> String {
    let mut file = std::fs::File::open(path).unwrap();
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    digest.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}

fn commit_cohort(
    engine: &Engine,
    config: &Value,
    out: &Path,
    cohort: &mut Vec<PreparedWrite>,
    written: &mut usize,
    cohorts: &mut Vec<Value>,
    recorder: &mut super::Recorder,
) -> Result<(), ()> {
    super::budget_clock::phase(config, out, "write")?;
    let started = std::time::Instant::now();
    let receipt = engine.write(cohort).unwrap();
    cohorts.push(json!({"first_input_index":*written,"rows":cohort.len(),"cursor":receipt.cursor,
        "write_ns":u64::try_from(started.elapsed().as_nanos()).unwrap()}));
    recorder.emit("batches", cohorts.last().unwrap());
    *written += cohort.len();
    cohort.clear();
    Ok(())
}

fn template_drain(engine: &Engine, config: &Value, out: &Path) -> Result<Value, ()> {
    match super::g_budget::drain(config, |timeout| {
        engine.drain(timeout).map_err(|error| format!("{error:?}"))
    }) {
        Ok(report) => Ok(report),
        Err(error) => {
            std::fs::write(
                out.join("budget.json"),
                serde_json::to_vec_pretty(&json!({
                    "phase":"drain","disposition":"phase_failed","error":error
                }))
                .unwrap(),
            )
            .unwrap();
            Err(())
        }
    }
}

fn verification<T>(config: &Value, out: &Path, read: impl FnOnce() -> T) -> Result<T, ()> {
    super::g_write_schedule::admitted_read(config, out, "verification", read)
}

pub(super) fn run(config: &Value, out: &Path) -> bool {
    let templates = config["templates"].as_array().unwrap();
    assert!(!templates.is_empty());
    std::fs::create_dir_all(out).unwrap();
    for (index, input) in templates.iter().enumerate() {
        let directory = out.join(format!("case-{index}"));
        std::fs::create_dir(&directory).unwrap();
        let store = directory.join("template.sqlite");
        let opening = std::time::Instant::now();
        let opened = Engine::open_with_choice(&store, EmbedderChoice::Default).unwrap();
        let open_ns = u64::try_from(opening.elapsed().as_nanos()).unwrap();
        let engine = &opened.engine;
        let resolution = opened.report.embedder_device_resolution.as_ref().unwrap();
        let device = match &resolution.effective_device {
            EffectiveEmbedDevice::Cuda(device) => device,
            EffectiveEmbedDevice::Cpu => panic!("normal G template requires approved CUDA"),
        };
        assert_eq!(resolution.selected_cuda_uuid.as_deref(), config["gpu_uuid"].as_str());
        assert_eq!(device.ordinal, 0);
        assert_eq!(opened.report.default_embedder.dimension, 384);
        if super::budget_clock::phase(config, out, "configure").is_err() {
            return false;
        }
        let configuring = std::time::Instant::now();
        engine
            .configure_projections(
                &[ProjectionSpec {
                    name: "s30_dense".into(),
                    roles: [ProjectionRole::Searchable].into_iter().collect(),
                    fts: None,
                    vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
                    source: None,
                }],
                &[],
            )
            .unwrap();
        let configure_ns = u64::try_from(configuring.elapsed().as_nanos()).unwrap();
        let seeding = std::time::Instant::now();
        let mut cohort = Vec::new();
        let mut cohorts = Vec::new();
        let mut ids = Vec::new();
        let mut written = 0;
        let mut recorder = super::Recorder::new(&directory);
        let mut commit = |cohort: &mut Vec<PreparedWrite>| {
            commit_cohort(engine, config, out, cohort, &mut written, &mut cohorts, &mut recorder)
        };
        for line in std::io::BufReader::new(
            std::fs::File::open(input["records"].as_str().unwrap()).unwrap(),
        )
        .lines()
        {
            let record: Value = serde_json::from_str(&line.unwrap()).unwrap();
            let id = record["id"].as_str().unwrap();
            ids.push(id.to_owned());
            cohort.push(PreparedWrite::Node {
                kind: record["kind"].as_str().unwrap().into(),
                body: record["body"].as_str().unwrap().into(),
                logical_id: Some(id.into()),
                source_id: SourceId::new("test:slice30-g-template").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            });
            if cohort.len() == 64 && commit(&mut cohort).is_err() {
                recorder.flush_all();
                engine.close().unwrap();
                return false;
            }
        }
        if !cohort.is_empty() && commit(&mut cohort).is_err() {
            recorder.flush_all();
            engine.close().unwrap();
            return false;
        }
        assert!(!ids.is_empty());
        recorder.flush_all();
        let seed_ns = u64::try_from(seeding.elapsed().as_nanos()).unwrap();
        let draining = std::time::Instant::now();
        if template_drain(engine, config, out).is_err() {
            engine.close().unwrap();
            return false;
        }
        let drain_ns = u64::try_from(draining.elapsed().as_nanos()).unwrap();
        let Ok(pending) =
            verification(config, out, || engine.read_embedding_readiness().unwrap().pending_count)
        else {
            engine.close().unwrap();
            return false;
        };
        assert_eq!(pending, 0);
        let closing = std::time::Instant::now();
        engine.close().unwrap();
        let close_ns = u64::try_from(closing.elapsed().as_nanos()).unwrap();
        let Ok((coverage, verification_ns)) = verification(config, out, || {
            let verifying = std::time::Instant::now();
            let coverage = template_verify::coverage(&store, &opened.report.default_embedder);
            (coverage, u64::try_from(verifying.elapsed().as_nanos()).unwrap())
        }) else {
            return false;
        };
        assert!(
            coverage["rows"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["byte_end"].as_u64().is_some()),
            "pinned BGE successful spans must have known ends"
        );
        let Ok(closed_hash) = verification(config, out, || hash(&store)) else {
            return false;
        };
        let wal = std::path::PathBuf::from(format!("{}-wal", store.display()));
        assert!(std::fs::metadata(wal).map_or(true, |metadata| metadata.len() == 0));
        let proof = json!({"api_complete":true,"input_ids":ids,"write_rows":written,"write_calls":cohorts.len(),"cohorts":cohorts,
            "open_ns":open_ns,"configure_ns":configure_ns,"seed_ns":seed_ns,"drain_ns":drain_ns,"close_ns":close_ns,"verification_ns":verification_ns,
            "pending":pending,"coverage":coverage,"closed_main_sha256":closed_hash,"raw_persistence":recorder.statistics_since([0;5]),
            "selected_cuda_uuid":resolution.selected_cuda_uuid,"cuda_ordinal":device.ordinal,"cuda_compiled":resolution.cuda_compiled,
            "cuda_libraries":format!("{:?}",device.cuda_libraries),"pooling":"engine Default BGE Mean; governed projection preparation"});
        std::fs::write(
            directory.join("api-proof.json"),
            serde_json::to_vec_pretty(&proof).unwrap(),
        )
        .unwrap();
        let report = json!({"coverage":proof["coverage"],"closed_main_sha256":proof["closed_main_sha256"],"preparation":proof});
        std::fs::write(
            directory.join("template.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
    true
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn verification_admission(key: &str, count: usize) {
        use std::cell::Cell;
        let root =
            tempfile::Builder::new().prefix("s30-gt-verification-").tempdir().unwrap().keep();
        let store = root.join("store.sqlite");
        let engine = Engine::open_with_choice(&store, EmbedderChoice::None).unwrap().engine;
        engine.write(&writes(count)).unwrap();
        assert_eq!(engine.read_embedding_readiness().unwrap().pending_count, 0);
        engine.close().unwrap();
        let calls = Cell::new(0);
        let read = || {
            calls.set(calls.get() + 1);
            let db = rusqlite::Connection::open_with_flags(
                &store,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            let actual: usize =
                db.query_row("SELECT count(*) FROM canonical_nodes", [], |row| row.get(0)).unwrap();
            assert_eq!(actual, count);
            let digest = hash(&store);
            assert_eq!(digest.len(), 64);
            (actual, digest)
        };
        assert_eq!(verification(&json!({}), &root, read).unwrap().0, count);
        calls.set(0);
        let mut config = json!({"work_deadline_ns":u64::MAX,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX});
        config[key] = json!(0);
        let result = verification(&config, &root, read);
        println!(
            "actual G final verification {} rows={count} key={key} calls={} refused={}",
            root.display(),
            calls.get(),
            result.is_err()
        );
        assert_eq!(
            (calls.get(), result.is_err()),
            (0, true),
            "expired preparation still reads/hashes its real closed store"
        );
        let report: Value =
            serde_json::from_slice(&std::fs::read(root.join("budget.json")).unwrap()).unwrap();
        assert_eq!(report["phase"], "verification");
        assert_eq!(report["disposition"], "deadline_refused");
    }

    #[test]
    fn work_expiry_prevents_late_preparation_reads() {
        verification_admission("work_deadline_ns", 2);
    }
    #[test]
    fn hard_expiry_prevents_late_preparation_reads() {
        verification_admission("hard_deadline_ns", 3);
    }
    #[test]
    fn shared_expiry_prevents_late_preparation_reads() {
        verification_admission("shared_deadline_ns", 1);
    }
    proptest! {
        #![proptest_config(ProptestConfig{cases:3,max_shrink_iters:0,failure_persistence:None,..ProptestConfig::default()})]
        #[test]
        fn preparation_reads_share_original_bounds(key in prop::sample::select(vec!["work_deadline_ns","hard_deadline_ns","shared_deadline_ns"]),count in 1usize..5) { verification_admission(key,count); }
    }

    fn writes(count: usize) -> Vec<PreparedWrite> {
        (0..count)
            .map(|index| PreparedWrite::Node {
                kind: "doc".into(),
                body: format!("real preparation admission {index}"),
                logical_id: Some(format!("cohort-{index}")),
                source_id: SourceId::new("test:s30-template-admission").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect()
    }

    fn check_write(config: &Value, count: usize, allowed: bool) {
        let root = tempfile::Builder::new().prefix("s30-template-write-").tempdir().unwrap().keep();
        let store = root.join("store.sqlite");
        let engine = Engine::open_with_choice(&store, EmbedderChoice::None).unwrap().engine;
        let mut cohort = writes(count);
        let mut written = 0;
        let mut cohorts = Vec::new();
        let mut recorder = super::super::Recorder::new(&root);
        let result = commit_cohort(
            &engine,
            config,
            &root,
            &mut cohort,
            &mut written,
            &mut cohorts,
            &mut recorder,
        );
        engine.close().unwrap();
        let db = rusqlite::Connection::open_with_flags(
            &store,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let actual: usize =
            db.query_row("SELECT count(*) FROM canonical_nodes", [], |row| row.get(0)).unwrap();
        println!(
            "actual template write {} allowed={allowed} rows={actual} result={result:?}",
            root.display()
        );
        assert_eq!(
            actual,
            if allowed { count } else { 0 },
            "expired preparation cohort still committed real rows"
        );
        assert_eq!(result.is_ok(), allowed);
        assert_eq!(written, if allowed { count } else { 0 });
        assert_eq!(cohorts.len(), usize::from(allowed));
        assert_eq!(cohort.len(), if allowed { 0 } else { count });
        if allowed {
            assert_eq!(cohorts[0]["rows"], count);
            assert_eq!(cohorts[0]["first_input_index"], 0);
        } else {
            let report: Value =
                serde_json::from_slice(&std::fs::read(root.join("budget.json")).unwrap()).unwrap();
            assert_eq!(report["phase"], "write");
            assert_eq!(report["disposition"], "deadline_refused");
        }
    }

    fn expired(bound: usize, count: usize) {
        check_write(&json!({}), 1, true);
        let mut config = json!({"work_deadline_ns":u64::MAX,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX});
        config[["work_deadline_ns", "hard_deadline_ns", "shared_deadline_ns"][bound]] = json!(0);
        check_write(&config, count, false);
    }

    fn refused_preparation_drain(key: &str, count: usize) {
        let root = tempfile::Builder::new().prefix("s30-gt-drain-").tempdir().unwrap().keep();
        let store = root.join("store.sqlite");
        let engine = Engine::open_with_choice(&store, EmbedderChoice::None).unwrap().engine;
        engine.write(&writes(count)).unwrap();
        let healthy = template_drain(&engine, &json!({}), &root).unwrap();
        assert_eq!(healthy["drain_calls"], 1);
        let mut config = json!({"work_deadline_ns":u64::MAX,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX});
        config[key] = json!(0);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            template_drain(&engine, &config, &root)
        }));
        engine.close().unwrap();
        let db = rusqlite::Connection::open_with_flags(
            &store,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let actual: usize =
            db.query_row("SELECT count(*) FROM canonical_nodes", [], |row| row.get(0)).unwrap();
        assert_eq!(actual, count);
        println!(
            "actual GT drain {} key={key} canonical={actual} result={result:?}",
            root.display()
        );
        assert!(
            matches!(result, Ok(Err(()))),
            "preparation drain refusal panicked instead of returning to stop its caller"
        );
        let report: Value =
            serde_json::from_slice(&std::fs::read(root.join("budget.json")).unwrap()).unwrap();
        assert_eq!(report["phase"], "drain");
        assert_eq!(report["disposition"], "phase_failed");
        assert!(report["error"].as_str().unwrap().contains("deadline exhausted"));
    }

    #[test]
    fn work_drain_refusal_returns_without_later_preparation() {
        refused_preparation_drain("work_deadline_ns", 2);
    }
    #[test]
    fn hard_drain_refusal_returns_without_later_preparation() {
        refused_preparation_drain("hard_deadline_ns", 2);
    }
    #[test]
    fn shared_drain_refusal_returns_without_later_preparation() {
        refused_preparation_drain("shared_deadline_ns", 2);
    }
    proptest! {
        #![proptest_config(ProptestConfig{cases:3,max_shrink_iters:0,failure_persistence:None,..ProptestConfig::default()})]
        #[test]
        fn original_preparation_drain_refusal_preserves_rows(count in 1usize..5,key in prop::sample::select(vec!["work_deadline_ns","hard_deadline_ns","shared_deadline_ns"])) {
            refused_preparation_drain(key,count);
        }
    }

    fn retained_cohorts(tail: usize) {
        let root = tempfile::Builder::new().prefix("s30-gt-checkpoint-").tempdir().unwrap().keep();
        let store = root.join("store.sqlite");
        let engine = Engine::open_with_choice(&store, EmbedderChoice::None).unwrap().engine;
        let mut recorder = super::super::Recorder::new(&root);
        let mut written = 0;
        let mut cohorts = Vec::new();
        for index in 0..16 + tail {
            let mut cohort = writes(1);
            if let PreparedWrite::Node { logical_id, .. } = &mut cohort[0] {
                *logical_id = Some(format!("retained-{index}"));
            }
            commit_cohort(
                &engine,
                &json!({}),
                &root,
                &mut cohort,
                &mut written,
                &mut cohorts,
                &mut recorder,
            )
            .unwrap();
        }
        let mut refused = writes(1);
        let result = commit_cohort(
            &engine,
            &json!({"work_deadline_ns":0,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX}),
            &root,
            &mut refused,
            &mut written,
            &mut cohorts,
            &mut recorder,
        );
        assert!(result.is_err());
        engine.close().unwrap();
        let db = rusqlite::Connection::open_with_flags(
            &store,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let cursors: Vec<u64> = db
            .prepare("SELECT write_cursor FROM canonical_nodes ORDER BY write_cursor")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(cursors.len(), 16 + tail);
        assert_eq!(written, 16 + tail);
        assert_eq!(cohorts.len(), 16 + tail);
        let read = || {
            std::fs::read_to_string(root.join("batches.jsonl"))
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str::<Value>(line).unwrap())
                .collect::<Vec<_>>()
        };
        let prefix = read();
        println!(
            "actual GT checkpoint {} canonical={} retained={} tail={tail}",
            root.display(),
            cursors.len(),
            prefix.len()
        );
        assert_eq!(
            prefix.len(),
            16,
            "completed preparation checkpoint was not retained before later refusal"
        );
        for (index, row) in prefix.iter().enumerate() {
            assert_eq!(row["cursor"], cursors[index]);
            assert_eq!(row["first_input_index"], index);
            assert_eq!(row["rows"], 1);
        }
        let statistics = recorder.statistics_since([0; 5]);
        assert_eq!(statistics["records"], 16 + tail);
        assert_eq!(statistics["checkpoints"], 1);
        recorder.flush_all();
        let complete = read();
        assert_eq!(complete.len(), 16 + tail);
        assert_eq!(
            complete.iter().map(|row| row["cursor"].as_u64().unwrap()).collect::<Vec<_>>(),
            cursors
        );
    }

    #[test]
    fn preparation_checkpoint_survives_subsequent_refused_write() {
        retained_cohorts(0);
    }
    proptest! {
        #![proptest_config(ProptestConfig{cases:3,max_shrink_iters:0,failure_persistence:None,..ProptestConfig::default()})]
        #[test]
        fn preparation_prefix_survives_small_uncheckpointed_tail(tail in 1usize..4) {retained_cohorts(tail);}
    }

    #[test]
    fn expired_work_prevents_real_preparation_write() {
        expired(0, 2);
    }
    #[test]
    fn expired_hard_prevents_real_preparation_write() {
        expired(1, 2);
    }
    #[test]
    fn expired_shared_prevents_real_preparation_write() {
        expired(2, 2);
    }
    proptest! {
        #![proptest_config(ProptestConfig{cases:4,max_shrink_iters:0,failure_persistence:None,..ProptestConfig::default()})]
        #[test]
        fn expired_cohort_preserves_unwritten_input_and_counters(bound in 0usize..3,count in 1usize..5) {expired(bound,count);}
    }
}
