use super::reader_pool::READER_POOL_SIZE;
use super::{
    acquire_lock_without_metadata_mutation, install_post_probe_startup_fault_for_test,
    install_post_probe_visibility_fault_for_test, Engine, EngineConfig, EngineOpenError,
    InitialState, PreparedWrite, ReadView, SourceId, TestEmbedderChoice,
};
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use std::fs::File;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use tempfile::TempDir;

const CHILD_CASE: &str = "FATHOMDB_S20_OPEN_HOOK_CHILD_CASE";

struct OwnedChild(Child);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

// Hook slots belong to the process, so only the exact child may install them.
fn isolated_case(selector: &str, body: fn()) {
    if let Some(child_case) = std::env::var_os(CHILD_CASE) {
        assert_eq!(
            child_case.to_str(),
            Some(selector),
            "child dispatch must select its own semantic case"
        );
        println!("S20_OPEN_HOOK_CASE {selector} pid={}", std::process::id());
        body();
        return;
    }

    let logs = TempDir::new().expect("isolated case output");
    let stdout_path = logs.path().join("stdout.log");
    let stderr_path = logs.path().join("stderr.log");
    let deadline = Instant::now() + Duration::from_secs(90);
    let mut child = OwnedChild(
        Command::new(std::env::current_exe().expect("current test executable"))
            .args(["--exact", selector, "--nocapture", "--test-threads=1"])
            .env(CHILD_CASE, selector)
            .stdin(Stdio::null())
            .stdout(File::create(&stdout_path).expect("child stdout"))
            .stderr(File::create(&stderr_path).expect("child stderr"))
            .spawn()
            .expect("spawn isolated real-engine case"),
    );
    let pid = child.0.id();
    let (status, timed_out) = loop {
        if let Some(status) = child.0.try_wait().expect("observe owned child") {
            break (status, false);
        }
        if Instant::now() >= deadline {
            child.0.kill().expect("terminate exact owned timed-out child");
            break (child.0.wait().expect("reap exact owned timed-out child"), true);
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let stdout = std::fs::read(&stdout_path).expect("read original child stdout");
    let stderr = std::fs::read(&stderr_path).expect("read original child stderr");
    std::io::stdout().write_all(&stdout).expect("forward original child stdout");
    std::io::stderr().write_all(&stderr).expect("forward original child stderr");
    assert!(!timed_out, "isolated real-engine case exceeded its absolute90s deadline");
    let output = String::from_utf8_lossy(&stdout);
    assert!(
        output.contains(&format!("S20_OPEN_HOOK_CASE {selector} pid={pid}")),
        "the exact child must reach its real-engine semantic case"
    );
    assert!(status.success(), "isolated semantic case {selector} exited {status}");
    assert!(
        output
            .lines()
            .any(|line| { line.starts_with("test result: ok. 1 passed; 0 failed; 0 ignored;") }),
        "the exact child must execute one nonignored semantic case"
    );
}

struct FixedProvider;

impl Embedder for FixedProvider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice20-open-hook-alias", "r1", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0; 8])
    }
}

#[test]
fn startup_fault_matches_canonical_database_alias() {
    isolated_case(
        "slice20_open_hook_tests::startup_fault_matches_canonical_database_alias",
        startup_fault_alias_body,
    );
}

fn startup_fault_alias_body() {
    let dir = TempDir::new().expect("real database directory");
    std::fs::create_dir(dir.path().join("nested")).expect("existing alias parent");
    let path = dir.path().join("nested").join("..").join("startup-alias.sqlite");
    let initial = Engine::open_with_test_choice_and_config_for_test(
        &path,
        TestEmbedderChoice::Caller(Arc::new(FixedProvider)),
        EngineConfig::default(),
    )
    .expect("first real open");
    initial.engine.configure_vector_kind_for_test("doc").expect("enrol vector kind");
    initial
        .engine
        .write(&[named_record("target", "target committed aé猫")])
        .expect("commit target state");
    initial.engine.close().expect("close first session");
    let sibling_path = dir.path().join("unrelated.sqlite");
    let sibling = Engine::open_with_test_choice_for_test(
        &sibling_path,
        TestEmbedderChoice::Caller(Arc::new(FixedProvider)),
    )
    .expect("initial sibling")
    .engine;
    sibling.configure_vector_kind_for_test("doc").expect("sibling vector kind");
    sibling.write(&[named_record("sibling", "unrelated committed aé猫")]).expect("commit sibling");
    sibling.close().expect("close initial sibling");

    let (observed_tx, observed_rx) = mpsc::channel();
    install_post_probe_startup_fault_for_test(path.clone(), observed_tx);
    let sibling = Engine::open_with_test_choice_for_test(
        &sibling_path,
        TestEmbedderChoice::Caller(Arc::new(FixedProvider)),
    )
    .expect("targeted alias hook must not consume sibling startup")
    .engine;
    exact_record(&sibling, "sibling", "unrelated committed aé猫");
    let result = Engine::open_with_test_choice_and_config_for_test(
        &path,
        TestEmbedderChoice::Caller(Arc::new(FixedProvider)),
        EngineConfig { embedder_call_timeout_ms: Some(50), ..EngineConfig::default() },
    );
    let error = match result {
        Ok(opened) => {
            opened.engine.close().expect("close unexpected open");
            panic!("injected post-probe startup failure was ignored for canonical database alias")
        }
        Err(error) => error,
    };
    let observation = observed_rx.recv_timeout(Duration::from_secs(2)).expect("fault observation");
    let retained_at_return = observation.accounting.snapshot().live_workers;
    let owners_at_return = observation.registry.live.lock().expect("registry").len();
    let releases = observation.profile_releases.releases.lock().expect("profile facts");
    let released_profiles = releases.len();
    let profiles_safe = releases.iter().all(|fact| {
        fact.callback_uninstalled && fact.live_connections.is_empty() && fact.live_workers == 0
    });
    drop(releases);
    let admission = acquire_lock_without_metadata_mutation(&path);
    let admission_released = admission.is_ok();
    drop(admission);

    assert!(
        matches!(error, EngineOpenError::Io { message } if message == "injected post-probe startup failure")
    );
    assert_eq!(observation.profiles_at_fault, READER_POOL_SIZE + 1);
    assert_eq!(released_profiles, READER_POOL_SIZE + 1);
    assert!(profiles_safe, "profile contexts released before SQLite callback owners");
    assert_eq!(owners_at_return, 0, "all managed SQLite owners must exit");
    assert_eq!(retained_at_return, 0, "accepted provider workers leave on failed startup");
    assert!(admission_released, "failed open must release admission lock");
    exact_record(&sibling, "sibling", "unrelated committed aé猫");
    sibling
        .write(&[named_record("after-fault", "sibling still writes")])
        .expect("sibling work after target fault");
    exact_record(&sibling, "after-fault", "sibling still writes");
    sibling.close().expect("sibling clean close");
    let recovered = Engine::open_with_test_choice_for_test(
        &path,
        TestEmbedderChoice::Caller(Arc::new(FixedProvider)),
    )
    .expect("target ownership recovers")
    .engine;
    exact_record(&recovered, "target", "target committed aé猫");
    recovered.close().expect("recovered target close");
}

fn named_record(id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "note".to_owned(),
        body: body.to_owned(),
        source_id: SourceId::new("test:alias-isolation").unwrap(),
        logical_id: Some(id.to_owned()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
    }
}

fn exact_record(engine: &Engine, id: &str, body: &str) {
    let row =
        engine.read_get(id, &ReadView::default()).expect("read committed row").expect("present");
    assert_eq!(row.logical_id, id);
    assert_eq!(row.body, body);
}

#[test]
fn visibility_fault_matches_canonical_database_alias() {
    isolated_case(
        "slice20_open_hook_tests::visibility_fault_matches_canonical_database_alias",
        visibility_fault_alias_body,
    );
}

fn visibility_fault_alias_body() {
    let dir = TempDir::new().expect("real database directory");
    std::fs::create_dir(dir.path().join("nested")).expect("existing alias parent");
    let path = dir.path().join("nested").join("..").join("visibility-alias.sqlite");
    let initial = Engine::open_with_test_choice_for_test(
        &path,
        TestEmbedderChoice::Caller(Arc::new(FixedProvider)),
    )
    .expect("first real open");
    initial.engine.configure_vector_kind_for_test("doc").expect("enrol vector kind");
    initial.engine.close().expect("close first session");

    let (observed_tx, observed_rx) = mpsc::channel();
    install_post_probe_visibility_fault_for_test(path.clone(), observed_tx);
    let result = Engine::open_with_test_choice_for_test(
        &path,
        TestEmbedderChoice::Caller(Arc::new(FixedProvider)),
    );
    let error = match result {
        Ok(opened) => {
            opened.engine.close().expect("close unexpected open");
            panic!(
                "accepted post-probe visibility failure was ignored for canonical database alias"
            )
        }
        Err(error) => error,
    };
    assert!(
        matches!(error, EngineOpenError::Io { message } if message == "could not load frozen-read visibility generation")
    );
    let (accounting, registry) =
        observed_rx.recv_timeout(Duration::from_secs(2)).expect("fault observation");
    assert_eq!(accounting.snapshot().live_workers, 0, "accepted provider workers exit");
    assert_eq!(registry.live.lock().expect("registry").len(), 0, "all SQLite owners exit");
    assert!(acquire_lock_without_metadata_mutation(&path).is_ok(), "admission lock released");
}
