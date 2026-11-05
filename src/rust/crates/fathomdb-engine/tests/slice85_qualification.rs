//! Bounded empirical screen, run only after correctness/review with no builds.
//! Serial closed-loop workload; raw outcomes and latencies are retained. Limits
//! are checked at safe API boundaries, never advertised as active SQL deadlines.
use fathomdb_engine::*;
use std::{
    fs::File,
    io::Write,
    path::Path,
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};
struct Screen {
    file: File,
    start: Instant,
    cell: Instant,
    label: String,
    root: std::path::PathBuf,
}
fn scratch(path: &Path) -> u64 {
    std::fs::read_dir(path)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum()
}
fn rss() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmHWM:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u64>().ok())
        })
        .unwrap_or(0)
        * 1024
}
impl Screen {
    fn run<T>(
        &mut self,
        phase: &str,
        operation: impl FnOnce() -> Result<T, EngineError>,
    ) -> Result<T, EngineError> {
        self.run_with(phase, operation, |_| "ok".into())
    }
    fn run_debug<T: std::fmt::Debug>(
        &mut self,
        phase: &str,
        operation: impl FnOnce() -> Result<T, EngineError>,
    ) -> Result<T, EngineError> {
        self.run_with(phase, operation, |value| format!("{value:?}"))
    }
    fn run_with<T>(
        &mut self,
        phase: &str,
        operation: impl FnOnce() -> Result<T, EngineError>,
        describe: impl FnOnce(&T) -> String,
    ) -> Result<T, EngineError> {
        let start = Instant::now();
        let result = operation();
        let elapsed = start.elapsed();
        let memory = rss();
        let bytes = scratch(&self.root);
        let row = serde_json::json!({"cell":self.label,"phase":phase,"elapsed_ns":elapsed.as_nanos() as u64,"outcome":match &result{Ok(value)=>describe(value),Err(e)=>format!("error:{e}")},"rss_peak_bytes":memory,"scratch_bytes":bytes});
        writeln!(self.file, "{row}").unwrap();
        self.file.flush().unwrap();
        assert!(
            self.cell.elapsed() < Duration::from_secs(120),
            "cell ceiling; outcome already retained"
        );
        assert!(self.start.elapsed() < Duration::from_secs(900), "total screen ceiling");
        assert!(memory <= 4 << 30 && bytes <= 2 << 30, "resource ceiling");
        result
    }
}
fn node(id: &str, source: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: format!("measurement content {id}"),
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}
fn accepted(r: QueueResult) -> String {
    match r {
        QueueResult::Accepted { ticket, .. } => ticket,
        _ => panic!("heavy admission refused"),
    }
}
#[test]
#[ignore = "empirical screen; run serially after review, retain JSONL output"]
fn empirical_explicit_erasure_screen() {
    let output = std::env::var("S85_QUALIFICATION_OUTPUT").unwrap_or_else(|_| {
        format!("/tmp/fathomdb-slice85-qualification-{}.jsonl", std::process::id())
    });
    let file = File::options().write(true).create_new(true).open(&output).unwrap();
    let mut screen = Screen {
        file,
        start: Instant::now(),
        cell: Instant::now(),
        label: String::new(),
        root: std::path::PathBuf::new(),
    };
    writeln!(screen.file,"{}",serde_json::json!({"protocol":"slice85-serial-screen-v1","stores_per_cell":3,"ordinary_reads_and_writes":100,"queue_backlogs":[0,1,16],"matched_autonomous_corpora":true,"cell_ceiling_s":120,"total_ceiling_s":900,"rss_ceiling_bytes":4u64<<30,"scratch_ceiling_bytes":2u64<<30,"percentile_claim":null,"rss_supported":cfg!(target_os="linux")})).unwrap();
    for explicit in [false, true] {
        for backlog in [0usize, 1, 16] {
            for repeat in 0..3 {
                let dir = tempfile::tempdir().unwrap();
                screen.root = dir.path().into();
                screen.cell = Instant::now();
                screen.label = format!(
                    "{}-corpus{}-queued{}-r{repeat}",
                    if explicit { "explicit" } else { "autonomous" },
                    backlog.max(1),
                    if explicit { backlog } else { 0 }
                );
                let path = dir.path().join("screen.sqlite");
                let policy = if explicit {
                    PhysicalErasurePolicy::ExplicitMaintenance
                } else {
                    PhysicalErasurePolicy::Autonomous
                };
                let started = Instant::now();
                let mut engine = Arc::new(
                    Engine::open_with_choice_and_config(
                        &path,
                        EmbedderChoice::None,
                        EngineConfig {
                            physical_erasure_policy: Some(policy),
                            ..Default::default()
                        },
                    )
                    .unwrap()
                    .engine,
                );
                writeln!(screen.file,"{}",serde_json::json!({"cell":screen.label,"phase":"fresh_open","elapsed_ns":started.elapsed().as_nanos() as u64,"outcome":"ok"})).unwrap();
                engine.enable_physical_cleanup_metrics_for_test();
                let mut seed = vec![node("kept", "kept-source")];
                for i in 0..backlog.max(1) {
                    seed.push(node(&format!("target{i}"), &format!("source{i}")));
                }
                engine.write(&seed).unwrap();
                let mut tickets = Vec::new();
                if explicit && backlog > 0 {
                    let mut handle = screen
                        .run("acquire_for_queue", || engine.begin_erasure_maintenance(1000))
                        .unwrap();
                    for i in 0..backlog {
                        let selector = PhysicalErasureSelector::SourceAllAtCut {
                            source_id: format!("source{i}"),
                        };
                        tickets.push(accepted(
                            screen
                                .run_debug("heavy_queue_accepted", || {
                                    handle.queue_physical_erasure(selector, &format!("{i:032x}"))
                                })
                                .unwrap(),
                        ));
                    }
                    handle.close().unwrap();
                }
                if explicit {
                    assert_eq!(
                        screen
                            .run_debug("normal_queue_needs_maintenance_admission", || engine
                                .queue_physical_erasure(
                                    PhysicalErasureSelector::SourceAllAtCut {
                                        source_id: "source0".into()
                                    },
                                    "19857156ca874ce89706cc0529d354c8"
                                ))
                            .unwrap(),
                        QueueResult::NeedsMaintenanceAdmission
                    );
                }
                for i in 0..100 {
                    screen
                        .run_debug("ordinary_read", || {
                            engine.read_get("kept", &ReadView::default())
                        })
                        .unwrap();
                    screen
                        .run_debug("ordinary_write", || {
                            engine.write(&[node(&format!("kept-{i}"), "kept-source")])
                        })
                        .unwrap();
                    if explicit && backlog > 0 {
                        let result = screen
                            .run_debug("exact_key_retry", || {
                                engine.queue_physical_erasure(
                                    PhysicalErasureSelector::SourceAllAtCut {
                                        source_id: "source0".into(),
                                    },
                                    "00000000000000000000000000000000",
                                )
                            })
                            .unwrap();
                        assert!(matches!(result, QueueResult::Accepted { .. }));
                    }
                }
                engine.close().unwrap();
                let started = Instant::now();
                engine = Arc::new(Engine::open(&path).unwrap().engine);
                writeln!(screen.file,"{}",serde_json::json!({"cell":screen.label,"phase":"reopen_no_implicit_cleanup","elapsed_ns":started.elapsed().as_nanos() as u64,"outcome":"ok"})).unwrap();
                engine.enable_physical_cleanup_metrics_for_test();
                if explicit {
                    let (reached_tx, reached_rx) = mpsc::channel();
                    let (release_tx, release_rx) = mpsc::channel();
                    let read = {
                        let engine = engine.clone();
                        std::thread::spawn(move || {
                            arm_explanation_before_telemetry_lock_hook_for_test(Box::new(
                                move || {
                                    reached_tx.send(()).unwrap();
                                    release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                                },
                            ));
                            engine.search_explained("measurement", None, 0, false, 0.3, 0)
                        })
                    };
                    reached_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                    let busy = screen
                        .run("acquire_owned_read_refused", || engine.begin_erasure_maintenance(0));
                    release_tx.send(()).unwrap();
                    read.join().unwrap().unwrap();
                    assert!(matches!(
                        busy,
                        Err(EngineError::Maintenance(MaintenanceError::MaintenanceBusy))
                    ));
                    let window_started = Instant::now();
                    let mut handle = screen
                        .run("acquire_drained", || engine.begin_erasure_maintenance(1000))
                        .unwrap();
                    for _ in 0..100 {
                        assert!(matches!(
                            screen.run_debug("newcomer_refused", || engine
                                .read_get("kept", &ReadView::default())),
                            Err(EngineError::Maintenance(MaintenanceError::MaintenanceBusy))
                        ));
                    }
                    let ticket = if tickets.is_empty() {
                        accepted(
                            screen
                                .run_debug("heavy_queue_in_window", || {
                                    handle.queue_physical_erasure(
                                        PhysicalErasureSelector::SourceAllAtCut {
                                            source_id: "source0".into(),
                                        },
                                        "d7916204ed084ce29c4a6ef11f7a1de9",
                                    )
                                })
                                .unwrap(),
                        )
                    } else {
                        tickets[0].clone()
                    };
                    assert!(matches!(
                        screen
                            .run_debug("physical_completion", || handle
                                .complete_physical_erasure(&ticket))
                            .unwrap(),
                        CleanupResult::Complete { .. }
                    ));
                    screen.run_debug("release", || handle.close()).unwrap();
                    writeln!(screen.file,"{}",serde_json::json!({"cell":screen.label,"phase":"whole_exclusive_blackout","elapsed_ns":window_started.elapsed().as_nanos() as u64,"outcome":"complete","physical_metrics":engine.physical_cleanup_metrics_for_test()})).unwrap();
                    if backlog == 1 && repeat == 0 {
                        engine.write(&[node("failure-target", "failure-source")]).unwrap();
                        let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
                        let ticket = accepted(
                            handle
                                .queue_physical_erasure(
                                    PhysicalErasureSelector::SourceAllAtCut {
                                        source_id: "failure-source".into(),
                                    },
                                    "69a13e5c3d0f413e8b18cfc86f8f1c12",
                                )
                                .unwrap(),
                        );
                        let reader = rusqlite::Connection::open(&path).unwrap();
                        reader
                            .execute_batch("BEGIN;SELECT count(*) FROM canonical_nodes;")
                            .unwrap();
                        assert!(matches!(
                            screen
                                .run_debug("external_reader_incomplete", || handle
                                    .complete_physical_erasure(&ticket))
                                .unwrap(),
                            CleanupResult::Incomplete { .. }
                        ));
                        handle.close().unwrap();
                        reader.execute_batch("ROLLBACK").unwrap();
                        let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
                        assert!(matches!(
                            screen
                                .run_debug("external_reader_retry_complete", || handle
                                    .complete_physical_erasure(&ticket))
                                .unwrap(),
                            CleanupResult::Complete { .. }
                        ));
                        handle.close().unwrap();
                        engine.write(&[node("sink-target", "sink-source")]).unwrap();
                        let original = dir.path().join("original.jsonl");
                        let other = dir.path().join("other.jsonl");
                        engine.enable_telemetry(original.to_str().unwrap()).unwrap();
                        engine.search_explained("measurement", None, 0, false, 0.3, 0).unwrap();
                        engine.enable_telemetry(other.to_str().unwrap()).unwrap();
                        let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
                        let ticket = accepted(
                            handle
                                .queue_physical_erasure(
                                    PhysicalErasureSelector::SourceAllAtCut {
                                        source_id: "sink-source".into(),
                                    },
                                    "43532414ed6641419e7373360e492ba2",
                                )
                                .unwrap(),
                        );
                        assert!(matches!(
                            screen
                                .run_debug("original_sink_unavailable", || handle
                                    .complete_physical_erasure(&ticket))
                                .unwrap(),
                            CleanupResult::Incomplete { .. }
                        ));
                        handle
                            .reattach_original_erasure_sink(&ticket, original.to_str().unwrap())
                            .unwrap();
                        assert!(matches!(
                            screen
                                .run_debug("original_sink_retry_complete", || handle
                                    .complete_physical_erasure(&ticket))
                                .unwrap(),
                            CleanupResult::Complete { .. }
                        ));
                        handle.close().unwrap();
                    }
                } else {
                    screen
                        .run_debug("autonomous_physical_completion", || {
                            engine.erase_source("source0")
                        })
                        .unwrap();
                }
                engine.close().unwrap();
            }
        }
    }
    println!("Slice85 empirical raw observations: {output};18 matched cells; no percentile or deadline claim");
}
