use fathomdb_engine::*;
use serde_json::json;
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
#[path = "support/slice100_fixture.rs"]
mod fixture;
use fixture::Preimage;

const FIXTURE: &str = "q1-v1-target1-survivor3-none-explicit";

fn case() -> (PathBuf, String) {
    let path = PathBuf::from(std::env::var("S100_CASE").unwrap());
    assert!(path.is_dir() && path.is_absolute());
    let nonce = std::env::var("S100_NONCE").unwrap();
    assert!(nonce.len() == 32 && nonce.bytes().all(|c| c.is_ascii_hexdigit()));
    (path, nonce)
}

fn queue(engine: &Arc<Engine>, nonce: &str) -> String {
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = match handle
        .queue_physical_erasure(
            PhysicalErasureSelector::SourceAllAtCut { source_id: "target".into() },
            nonce,
        )
        .unwrap()
    {
        QueueResult::Accepted { ticket, logical_state } => {
            assert_eq!(logical_state, ErasureLogicalState::UnchangedByThisRequest);
            ticket
        }
        _ => panic!("heavy admission refused"),
    };
    handle.close().unwrap();
    ticket
}

fn save_preimage(dir: &Path, pre: &Preimage) {
    std::fs::write(dir.join("preimage.json"), serde_json::to_vec_pretty(pre).unwrap()).unwrap();
}

fn load_preimage(dir: &Path, nonce: &str) -> Preimage {
    let pre: Preimage =
        serde_json::from_slice(&std::fs::read(dir.join("preimage.json")).unwrap()).unwrap();
    assert_eq!(pre.nonce, nonce);
    pre
}

#[inline(never)]
fn q1_ready_after_queue() {
    std::hint::black_box(());
}

#[test]
#[ignore]
fn q1_child() {
    let (dir, nonce) = case();
    for (key, _) in std::env::vars().filter(|(k, _)| k.starts_with("S100_")) {
        assert!(matches!(key.as_str(), "S100_CASE" | "S100_NONCE"), "unknown Q1 parameter");
    }
    let path = dir.join("candidate.sqlite");
    assert!(!path.exists(), "Q1 must create its own fresh store");
    let engine = fixture::minimal_fixture(&path);
    let ticket = queue(&engine, &nonce);
    let pre = fixture::capture(&engine, nonce.clone(), ticket.clone(), false, Vec::new());
    save_preimage(&dir, &pre);
    println!(
        "\nQ1_READY {}",
        json!({"nonce":nonce, "fixture":FIXTURE, "pid":std::process::id(), "ticket":ticket, "executable":std::env::current_exe().unwrap()})
    );
    std::io::stdout().flush().unwrap();
    q1_ready_after_queue();
    fixture::complete(&engine, &pre, None, &dir.join("healthy-live"));
    engine.close().unwrap();
    drop(engine);
    let reopened = fixture::reopen(&path, pre.rich);
    assert_eq!(reopened.physical_erasure_policy(), PhysicalErasurePolicy::ExplicitMaintenance);
    let c = fixture::read_only(&path);
    fixture::assert_phase(&c, &pre, "complete");
    fixture::assert_survivors(&c, &pre);
    fixture::assert_target_partition(&c, &pre, false);
    drop(c);
    reopened.close().unwrap();
    drop(reopened);
    println!("\nQ1_DONE {}", json!({"nonce":nonce, "fixture":FIXTURE}));
    std::io::stdout().flush().unwrap();
}

#[test]
#[ignore]
fn q1_observer() {
    let (dir, nonce) = case();
    let pre = load_preimage(&dir, &nonce);
    let path = PathBuf::from(std::env::var("S100_DATABASE").unwrap());
    assert!(path.starts_with(&dir) && path.exists());
    let c = fixture::read_only(&path);
    fixture::assert_phase(&c, &pre, "finalizing");
    fixture::assert_survivors(&c, &pre);
    fixture::assert_target_partition(&c, &pre, false);
}

#[test]
#[ignore]
fn q1_retry() {
    let (dir, nonce) = case();
    let pre = load_preimage(&dir, &nonce);
    let path = PathBuf::from(std::env::var("S100_DATABASE").unwrap());
    assert!(path.starts_with(&dir) && path.exists());
    let engine = fixture::reopen(&path, pre.rich);
    assert_eq!(engine.physical_erasure_policy(), PhysicalErasurePolicy::ExplicitMaintenance);
    let c = fixture::read_only(&path);
    fixture::assert_phase(&c, &pre, "finalizing");
    fixture::assert_survivors(&c, &pre);
    fixture::assert_target_partition(&c, &pre, false);
    drop(c);
    assert!(engine.read_get("kept-history", &ReadView::default()).unwrap().is_some());
    assert!(engine
        .write(&[fixture::node("must-not-appear", "unrelated", "fenced".into())])
        .is_err());
    assert!(engine.read_get("must-not-appear", &ReadView::default()).unwrap().is_none());
    fixture::complete(&engine, &pre, None, &dir.join("retry-live"));
    engine.close().unwrap();
    drop(engine);
    let reopened = fixture::reopen(&path, pre.rich);
    let c = fixture::read_only(&path);
    fixture::assert_phase(&c, &pre, "complete");
    fixture::assert_survivors(&c, &pre);
    fixture::assert_target_partition(&c, &pre, false);
    drop(c);
    reopened.close().unwrap();
}

#[test]
#[ignore]
fn g2_child() {
    let (dir, nonce) = case();
    let phase = std::env::var("S100_PHASE").unwrap();
    assert!([
        "healthy",
        "before_delete_commit",
        "after_delete_commit",
        "after_redacted_commit",
        "after_sink_redaction"
    ]
    .contains(&phase.as_str()));
    let path = dir.join("candidate.sqlite");
    assert!(!path.exists());
    let baseline = fixture::rich(&dir.join("baseline.sqlite"), false);
    let expected_scores = fixture::scores(&baseline);
    baseline.close().unwrap();
    drop(baseline);
    let engine = fixture::rich(&path, true);
    if phase == "after_sink_redaction" {
        engine.enable_telemetry(dir.join("original.jsonl").to_str().unwrap()).unwrap();
        std::fs::write(
            dir.join("original.jsonl"),
            "{\"type\":\"event\",\"result_stable_ids\":[\"l:root\",\"l:kept-history\"]}\n",
        )
        .unwrap();
    }
    let ticket = queue(&engine, &nonce);
    let pre = fixture::capture(&engine, nonce, ticket.clone(), true, expected_scores);
    save_preimage(&dir, &pre);
    if phase == "healthy" {
        fixture::complete(&engine, &pre, None, &dir.join("healthy-live"));
        engine.close().unwrap();
        return;
    }
    let phase: &'static str = Box::leak(phase.into_boxed_str());
    arm_physical_erasure_hook_for_test(phase, Box::new(|| std::process::exit(73)));
    let handle = engine.begin_erasure_maintenance(1000).unwrap();
    handle.complete_physical_erasure(&ticket).unwrap();
    panic!("selected existing-hook fault did not fire");
}

fn run_child(dir: &Path, nonce: &str, phase: &str) {
    let log = std::fs::File::create(dir.join("child.log")).unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "g2_child", "--ignored", "--nocapture", "--test-threads=1"])
        .env("S100_CASE", dir)
        .env("S100_NONCE", nonce)
        .env("S100_PHASE", phase)
        .stdout(log.try_clone().unwrap())
        .stderr(log)
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(120);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= until {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("owned G2 child timeout at {phase}");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(
        status.code(),
        Some(if phase == "healthy" { 0 } else { 73 }),
        "exact child status {phase}; retained child.log"
    );
}

fn copy_raw(from: &Path, into: &Path) {
    std::fs::create_dir(into).unwrap();
    for suffix in ["", "-wal", "-shm"] {
        let path = from.join(format!("candidate.sqlite{suffix}"));
        if path.exists() {
            std::fs::copy(path, into.join(format!("candidate.sqlite{suffix}"))).unwrap();
        } else {
            assert!(!suffix.is_empty(), "main evidence missing");
        }
    }
}

#[test]
#[ignore]
fn g2_case() {
    let (dir, nonce) = case();
    let phase = std::env::var("S100_PHASE").unwrap();
    let phase = phase.as_str();
    run_child(&dir, &nonce, phase);
    if phase == "healthy" {
        return;
    }
    // Preserve raw bytes before even a read-only SQLite open can reconstruct SHM.
    let snapshot = dir.join("untouched");
    copy_raw(&dir, &snapshot);
    let observed = dir.join("observed");
    copy_raw(&snapshot, &observed);
    let pre = load_preimage(&dir, &nonce);
    // Initialize the vector extension only against an independent survivor store.
    let survivor = fixture::rich(&dir.join("observer-survivor.sqlite"), false);
    let c = fixture::read_only(&observed.join("candidate.sqlite"));
    let expected_phase = match phase {
        "before_delete_commit" => "queued",
        "after_delete_commit" => "executing",
        "after_redacted_commit" => "finalizing",
        _ => "sanitizing",
    };
    fixture::assert_phase(&c, &pre, expected_phase);
    fixture::assert_survivors(&c, &pre);
    fixture::assert_target_partition(&c, &pre, phase == "before_delete_commit");
    drop(c);
    survivor.close().unwrap();
    drop(survivor);
    let retry_dir = dir.join("retry");
    copy_raw(&snapshot, &retry_dir);
    let path = retry_dir.join("candidate.sqlite");
    let engine = fixture::reopen(&path, pre.rich);
    assert_eq!(engine.physical_erasure_policy(), PhysicalErasurePolicy::ExplicitMaintenance);
    if phase == "before_delete_commit" {
        assert_eq!(fixture::scores(&engine), pre.intact_scores);
    } else {
        assert_eq!(fixture::scores(&engine), pre.survivor_only_scores);
        assert!(engine
            .write(&[fixture::node("must-not-appear", "other", "fenced".into())])
            .is_err());
        assert!(engine.read_get("must-not-appear", &ReadView::default()).unwrap().is_none());
    }
    let sink = dir.join("original.jsonl");
    if phase == "after_sink_redaction" {
        let bytes = std::fs::read_to_string(&sink).unwrap();
        assert!(!bytes.contains("l:root") && bytes.contains("l:kept-history"));
        let offline = dir.join("offline");
        copy_raw(&snapshot, &offline);
        let c = fixture::read_only(&observed.join("candidate.sqlite"));
        assert!(fixture::count(&c, "SELECT count(*) FROM operational_mutations WHERE collection_name='erasure_pending_redaction'") > 0);
        drop(c);
        assert!(
            matches!(recover_complete_erasures(offline.join("candidate.sqlite")),
            Err(EngineOpenError::Io { message }) if message == "original telemetry sink is unavailable; erasure remains owed"),
            "durable original-sink obligation remains owed"
        );
    }
    fixture::complete(
        &engine,
        &pre,
        sink.exists().then_some(sink.as_path()),
        &dir.join("retry-live"),
    );
    engine.close().unwrap();
    drop(engine);
    let reopened = fixture::reopen(&path, pre.rich);
    let c = fixture::read_only(&path);
    fixture::assert_phase(&c, &pre, "complete");
    fixture::assert_survivors(&c, &pre);
    fixture::assert_target_partition(&c, &pre, false);
    drop(c);
    reopened.close().unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn g2_composed_existing_faults() {
    use std::os::unix::process::CommandExt;
    // Only this dedicated test process becomes the owner of orphaned case children.
    assert_eq!(unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) }, 0);
    let root = match std::env::var("S100_CASE_ROOT") {
        Ok(path) => {
            let p = PathBuf::from(path);
            std::fs::create_dir(&p).unwrap();
            p
        }
        Err(_) => tempfile::tempdir().unwrap().keep(),
    };
    eprintln!("Slice100 retained G2 evidence: {}", root.display());
    for (index, phase) in [
        "healthy",
        "before_delete_commit",
        "after_delete_commit",
        "after_redacted_commit",
        "after_sink_redaction",
    ]
    .into_iter()
    .enumerate()
    {
        let dir = root.join(phase);
        std::fs::create_dir(&dir).unwrap();
        let log = std::fs::File::create(dir.join("case.log")).unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "g2_case", "--ignored", "--nocapture", "--test-threads=1"])
            .env("S100_CASE", &dir)
            .env("S100_NONCE", format!("{index:032x}"))
            .env("S100_PHASE", phase)
            .process_group(0)
            .stdout(log.try_clone().unwrap())
            .stderr(log)
            .spawn()
            .unwrap();
        let group = i32::try_from(child.id()).unwrap();
        let until = Instant::now() + Duration::from_secs(120);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break Some(status);
            }
            if Instant::now() >= until {
                break None;
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let timed_out = status.is_none();
        if timed_out {
            // The group was created by this exact spawn; it contains only this case.
            unsafe {
                libc::killpg(group, libc::SIGKILL);
            }
        }
        let cleanup = Instant::now() + Duration::from_secs(5);
        let mut leader = status;
        loop {
            if leader.is_none() {
                leader = child.try_wait().unwrap();
            }
            if leader.is_none() {
                assert!(Instant::now() < cleanup, "owned leader not reaped: {phase}, {group}");
                std::thread::sleep(Duration::from_millis(20));
                continue;
            }
            // After the case leader is reaped, only verified adopted children may be killed.
            let mut children = std::collections::BTreeSet::new();
            for thread in std::fs::read_dir("/proc/self/task").unwrap() {
                if let Ok(text) = std::fs::read_to_string(thread.unwrap().path().join("children")) {
                    children.extend(text.split_whitespace().map(|pid| pid.parse::<i32>().unwrap()));
                }
            }
            for pid in children {
                if unsafe { libc::getpgid(pid) } == group {
                    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
                    let fields =
                        stat.rsplit_once(')').unwrap().1.split_whitespace().collect::<Vec<_>>();
                    assert_eq!(fields[1].parse::<u32>().unwrap(), std::process::id());
                    // This unreaped direct child cannot have its PID reused by another process.
                    unsafe {
                        libc::kill(pid, libc::SIGKILL);
                    }
                }
            }
            let mut raw = 0;
            let reaped = unsafe { libc::waitpid(-group, &mut raw, libc::WNOHANG) };
            if leader.is_some()
                && reaped == -1
                && std::io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD)
            {
                break;
            }
            assert!(Instant::now() < cleanup, "owned group not reaped: {phase}, {group}");
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(
            !timed_out && leader.unwrap().success(),
            "G2 case failed/timed out at {phase}; retained case.log"
        );
    }
}
