//! Foreground provider calls share the engine's bounded inference deadline.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    Engine, EngineConfig, EngineError, EngineOpenError, InitialState, PreparedWrite, ReadContextV1,
    ReadView, SearchFilter, SourceId, TestEmbedderChoice,
};

#[derive(Debug)]
struct ParkedEmbedder {
    release: Mutex<mpsc::Receiver<()>>,
}

#[derive(Debug)]
struct FirstCallHeld {
    calls: AtomicUsize,
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}

#[derive(Debug)]
struct QueryHeld {
    query_calls: AtomicUsize,
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}

#[derive(Debug)]
struct FallibleEmbedder;

#[derive(Debug)]
struct ShortServiceEmbedder;

impl Embedder for ShortServiceEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("short-service", "rev-a", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        thread::sleep(Duration::from_micros(100));
        Ok(vec![1.0; 8])
    }
}

impl Embedder for FallibleEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("fallible-foreground", "rev-a", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        match text {
            "provider error" => Err(EmbedderError::Failed { message: "injected".to_owned() }),
            "provider panic" => panic!("injected provider panic"),
            _ => Ok(vec![1.0; 8]),
        }
    }
}

impl Embedder for QueryHeld {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("query-held", "rev-a", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        if text == "needle" && self.query_calls.fetch_add(1, Ordering::SeqCst) == 0 {
            self.entered.send(()).expect("report query embedding");
            self.release
                .lock()
                .expect("release lock")
                .recv_timeout(Duration::from_secs(2))
                .expect("release query provider");
        }
        Ok(vec![1.0; 8])
    }
}

fn document(body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "note".to_owned(),
        body: body.to_owned(),
        source_id: SourceId::new("test:foreground").expect("source id"),
        logical_id: None,
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
    }
}

#[test]
fn eight_reader_burst_keeps_vector_only_search_results() {
    let directory = tempfile::tempdir().expect("test directory");
    let database = directory.path().join("eight-reader-burst.sqlite");
    let engine = Engine::open_with_test_choice_and_config_for_test(
        database,
        TestEmbedderChoice::Caller(Arc::new(ShortServiceEmbedder)),
        EngineConfig::default(),
    )
    .expect("open")
    .engine;
    engine.configure_vector_kind_for_test("note").expect("vector kind");
    engine.write(&[document("opaque body")]).expect("write vector-only fixture");
    engine.drain(2_000).expect("drain projection");
    assert!(!engine.search("semantic-0").expect("sequential search").results.is_empty());

    let engine = Arc::new(engine);
    let barrier = Arc::new(Barrier::new(8));
    let readers: Vec<_> = (0..8)
        .map(|_| {
            let engine = Arc::clone(&engine);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let mut empty = 0;
                for _ in 0..8 {
                    barrier.wait();
                    empty += usize::from(
                        engine.search("semantic-0").expect("burst search").results.is_empty(),
                    );
                }
                empty
            })
        })
        .collect();
    let empty: usize = readers.into_iter().map(|reader| reader.join().expect("reader")).sum();
    assert_eq!(
        empty, 0,
        "bounded dispatch must preserve vector-only search under an eight-reader burst"
    );
}

impl Embedder for FirstCallHeld {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("first-call-held", "rev-a", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            self.entered.send(()).expect("report provider entry");
            self.release
                .lock()
                .expect("release lock")
                .recv_timeout(Duration::from_secs(10))
                .expect("release provider");
        }
        Ok(vec![1.0; 8])
    }
}

impl Embedder for ParkedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("foreground-parked", "rev-a", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        let _ = self.release.lock().expect("release lock").recv_timeout(Duration::from_millis(500));
        Ok(vec![1.0; 8])
    }
}

#[test]
fn direct_embed_started_timeout_uses_engine_dispatch_deadline() {
    let directory = tempfile::tempdir().expect("test directory");
    let database = directory.path().join("direct-timeout.sqlite");
    let (release, held) = mpsc::channel();
    let provider = Arc::new(ParkedEmbedder { release: Mutex::new(held) });
    let opened = Engine::open_with_test_choice_and_config_for_test(
        database,
        TestEmbedderChoice::Caller(provider),
        EngineConfig { embedder_call_timeout_ms: Some(50), ..EngineConfig::default() },
    )
    .expect("open");
    let started = Instant::now();
    let result = opened.engine.embed_text("deadline probe");
    let elapsed = started.elapsed();
    release.send(()).ok();
    assert!(matches!(result, Err(EngineError::Embedder)), "started timeout is an embedder error");
    assert!(elapsed < Duration::from_millis(250), "direct call must honor the 50ms deadline");
}

#[test]
fn direct_embed_started_error_and_panic_preserve_error_contract_and_worker_reuse() {
    let directory = tempfile::tempdir().expect("test directory");
    let database = directory.path().join("direct-failure.sqlite");
    let engine = Engine::open_with_test_choice_and_config_for_test(
        database,
        TestEmbedderChoice::Caller(Arc::new(FallibleEmbedder)),
        EngineConfig::default(),
    )
    .expect("open")
    .engine;
    assert!(matches!(engine.embed_text("provider error"), Err(EngineError::Embedder)));
    let panic = catch_unwind(AssertUnwindSafe(|| engine.embed_text("provider panic")));
    assert!(panic.is_err(), "direct provider panic remains observable to its caller");
    assert_eq!(engine.embed_text("healthy after panic").expect("worker reused"), vec![1.0; 8]);
    engine.close().expect("close");
}

#[test]
fn direct_embed_saturation_and_queued_expiry_are_overloaded() {
    let directory = tempfile::tempdir().expect("test directory");
    let database = directory.path().join("direct-overload.sqlite");
    let (entered, entered_rx) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let provider =
        Arc::new(FirstCallHeld { calls: AtomicUsize::new(0), entered, release: Mutex::new(held) });
    let engine = Arc::new(
        Engine::open_with_test_choice_and_config_for_test(
            database,
            TestEmbedderChoice::Caller(provider.clone()),
            EngineConfig {
                embedder_pool_size: Some(1),
                embedder_call_timeout_ms: Some(120),
                ..EngineConfig::default()
            },
        )
        .expect("open")
        .engine,
    );
    let first_engine = Arc::clone(&engine);
    let first = thread::spawn(move || first_engine.embed_text("first started"));
    entered_rx.recv_timeout(Duration::from_secs(1)).expect("provider entered");

    let launch = Arc::new(Barrier::new(6));
    let waiters: Vec<_> = (0..5)
        .map(|index| {
            let engine = Arc::clone(&engine);
            let launch = Arc::clone(&launch);
            thread::spawn(move || {
                launch.wait();
                engine.embed_text(&format!("queued {index}"))
            })
        })
        .collect();
    launch.wait();
    for waiter in waiters {
        assert!(matches!(waiter.join().expect("join waiter"), Err(EngineError::Overloaded)));
    }
    assert!(matches!(first.join().expect("join first"), Err(EngineError::Embedder)));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1, "no replacement call starts");
    release.send(()).expect("release provider");
    engine.close().expect("close");
}

#[test]
fn close_cancels_started_direct_embed_waiter() {
    let directory = tempfile::tempdir().expect("test directory");
    let database = directory.path().join("direct-close.sqlite");
    let (entered, entered_rx) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let provider =
        Arc::new(FirstCallHeld { calls: AtomicUsize::new(0), entered, release: Mutex::new(held) });
    let engine = Arc::new(
        Engine::open_with_test_choice_and_config_for_test(
            database,
            TestEmbedderChoice::Caller(provider),
            EngineConfig { embedder_call_timeout_ms: Some(5_000), ..EngineConfig::default() },
        )
        .expect("open")
        .engine,
    );
    let pending_engine = Arc::clone(&engine);
    let (pending_tx, pending_rx) = mpsc::channel();
    let pending = thread::spawn(move || {
        pending_tx.send(pending_engine.embed_text("started before close")).expect("report waiter");
    });
    entered_rx.recv_timeout(Duration::from_secs(1)).expect("provider entered");
    let (closed_tx, closed_rx) = mpsc::channel();
    let closing = Arc::clone(&engine);
    let close_thread = thread::spawn(move || {
        closed_tx.send(closing.close()).expect("report close");
    });
    let started = Instant::now();
    let close_started = loop {
        if matches!(engine.drain(0), Err(EngineError::Closing)) {
            break true;
        }
        if started.elapsed() >= Duration::from_secs(2) {
            break false;
        }
        thread::yield_now();
    };
    let waiter = pending_rx.recv_timeout(Duration::from_secs(1));
    let waiter_elapsed = started.elapsed();
    let close_while_held = closed_rx.recv_timeout(Duration::from_millis(100));
    release.send(()).expect("release provider");
    let close_result = closed_rx.recv_timeout(Duration::from_secs(2)).expect("close after release");
    pending.join().expect("join pending");
    close_thread.join().expect("join close");
    assert!(close_started, "close did not stop admission while provider was held");
    assert!(matches!(waiter, Ok(Err(EngineError::Closing))), "waiter={waiter:?}");
    assert!(waiter_elapsed < Duration::from_secs(1), "close must wake direct waiter");
    assert!(
        matches!(close_while_held, Err(mpsc::RecvTimeoutError::Timeout)),
        "close may not claim completion while provider is held: {close_while_held:?}"
    );
    assert!(close_result.is_ok(), "close after provider release: {close_result:?}");
}

#[test]
fn ordinary_search_uses_sparse_fallback_after_started_provider_timeout() {
    let directory = tempfile::tempdir().expect("test directory");
    let database = directory.path().join("ordinary-fallback.sqlite");
    let (entered, entered_rx) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let provider = Arc::new(QueryHeld {
        query_calls: AtomicUsize::new(0),
        entered,
        release: Mutex::new(held),
    });
    let engine = Engine::open_with_test_choice_and_config_for_test(
        database,
        TestEmbedderChoice::Caller(provider),
        EngineConfig { embedder_call_timeout_ms: Some(80), ..EngineConfig::default() },
    )
    .expect("open")
    .engine;
    engine.configure_vector_kind_for_test("note").expect("vector kind");
    let receipt = engine.write(&[document("needle in the original body")]).expect("write");
    engine.drain(2_000).expect("project original row");
    let started = Instant::now();
    let result = engine.search("needle").expect("sparse fallback search");
    let elapsed = started.elapsed();
    entered_rx.recv_timeout(Duration::from_secs(1)).expect("query embed entered");
    assert!(elapsed < Duration::from_millis(300), "search honors provider deadline");
    assert!(result.results.iter().any(|hit| hit.write_cursor == receipt.cursor));
    release.send(()).expect("release late provider output");
}

#[test]
fn ordinary_and_frozen_search_keep_sparse_fallback_on_provider_error() {
    let directory = tempfile::tempdir().expect("test directory");
    let database = directory.path().join("search-failure.sqlite");
    let engine = Engine::open_with_test_choice_and_config_for_test(
        database,
        TestEmbedderChoice::Caller(Arc::new(FallibleEmbedder)),
        EngineConfig::default(),
    )
    .expect("open")
    .engine;
    engine.configure_vector_kind_for_test("note").expect("vector kind");
    let errored =
        engine.write(&[document("provider error appears here")]).expect("write error row");
    engine.drain(2_000).expect("project rows");
    let context =
        ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("context");
    let frozen = engine.freeze_read_context(&context).expect("freeze");

    let ordinary = engine.search("provider error").expect("ordinary sparse fallback");
    assert!(ordinary.results.iter().any(|hit| hit.write_cursor == errored.cursor));
    let reader = engine
        .search_frozen("provider error", &frozen, 0, false, 0.3, 0, false, 10)
        .expect("frozen sparse fallback");
    assert!(reader.results.iter().any(|hit| hit.write_cursor == errored.cursor));
    assert_eq!(
        engine.embed_text("healthy after search failure").expect("worker reused"),
        vec![1.0; 8]
    );
    engine.close().expect("close");
}

#[test]
fn ordinary_search_provider_panic_reaches_caller_boundary() {
    let directory = tempfile::tempdir().expect("test directory");
    let database = directory.path().join("ordinary-panic.sqlite");
    let engine = Engine::open_with_test_choice_and_config_for_test(
        database,
        TestEmbedderChoice::Caller(Arc::new(FallibleEmbedder)),
        EngineConfig::default(),
    )
    .expect("open")
    .engine;
    engine.configure_vector_kind_for_test("note").expect("vector kind");
    engine.write(&[document("provider panic appears here")]).expect("write sparse row");
    engine.drain(2_000).expect("project row");
    let outcome = catch_unwind(AssertUnwindSafe(|| engine.search("provider panic")));
    assert!(outcome.is_err(), "timely panic must cross the ordinary caller boundary");
    engine.close().expect("close");
}

#[test]
fn frozen_search_provider_panic_unwinds_reader_owner_boundary() {
    let directory = tempfile::tempdir().expect("test directory");
    let database = directory.path().join("frozen-panic.sqlite");
    let engine = Engine::open_with_test_choice_and_config_for_test(
        database,
        TestEmbedderChoice::Caller(Arc::new(FallibleEmbedder)),
        EngineConfig::default(),
    )
    .expect("open")
    .engine;
    engine.configure_vector_kind_for_test("note").expect("vector kind");
    engine.write(&[document("provider panic appears here")]).expect("write sparse row");
    engine.drain(2_000).expect("project row");
    let context =
        ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("context");
    let frozen = engine.freeze_read_context(&context).expect("freeze");
    assert!(
        matches!(
            engine.search_frozen("provider panic", &frozen, 0, false, 0.3, 0, false, 10),
            Err(EngineError::Storage)
        ),
        "reader-owner panic must disconnect its response rather than return sparse results"
    );
    engine.close().expect("close");
}

#[test]
fn frozen_sparse_fallback_keeps_reader_snapshot_across_provider_wait() {
    let directory = tempfile::tempdir().expect("test directory");
    let database = directory.path().join("frozen-fallback.sqlite");
    let (entered, entered_rx) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let provider = Arc::new(QueryHeld {
        query_calls: AtomicUsize::new(0),
        entered,
        release: Mutex::new(held),
    });
    let engine = Arc::new(
        Engine::open_with_test_choice_and_config_for_test(
            database,
            TestEmbedderChoice::Caller(provider),
            EngineConfig { embedder_call_timeout_ms: Some(300), ..EngineConfig::default() },
        )
        .expect("open")
        .engine,
    );
    engine.configure_vector_kind_for_test("note").expect("vector kind");
    let before = engine.write(&[document("needle before snapshot")]).expect("write first");
    engine.drain(2_000).expect("project first");
    let context =
        ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("context");
    let frozen = engine.freeze_read_context(&context).expect("freeze");
    let search_engine = Arc::clone(&engine);
    let search = thread::spawn(move || {
        search_engine.search_frozen("needle", &frozen, 0, false, 0.3, 0, false, 10)
    });
    entered_rx.recv_timeout(Duration::from_secs(1)).expect("query entered after snapshot");
    let after = engine.write(&[document("needle after snapshot")]).expect("concurrent write");
    let result = search.join().expect("join frozen search").expect("sparse fallback");
    assert!(result.results.iter().any(|hit| hit.write_cursor == before.cursor));
    assert!(result.results.iter().all(|hit| hit.write_cursor != after.cursor));
    release.send(()).expect("release provider");
    engine.drain(2_000).expect("finish later projection");
}

fn named_document(id: &str, body: &str) -> PreparedWrite {
    let mut write = document(body);
    if let PreparedWrite::Node { logical_id, .. } = &mut write {
        *logical_id = Some(id.to_owned());
    }
    write
}

fn exact_record(engine: &Engine, id: &str, body: &str) {
    let record =
        engine.read_get(id, &ReadView::default()).expect("read exact record").expect("present");
    assert_eq!(record.logical_id, id);
    assert_eq!(record.body, body);
}

struct ReleaseProvider(mpsc::Sender<()>);

impl Drop for ReleaseProvider {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

const STORED_RECORDS: [(&str, &str, &str); 2] = [
    ("held.sqlite", "held-record", "held side committed aé猫"),
    ("sibling.sqlite", "sibling-record", "independent sibling committed aé猫"),
];

#[test]
fn two_engine_dispatch_and_alias_close_recover_exact_state_in_fresh_process() {
    if let Some(directory) = std::env::var_os("FATHOMDB_SLICE50_CASE_DIR") {
        let directory = std::path::Path::new(&directory);
        let phase = std::env::var("FATHOMDB_SLICE50_CASE_PHASE").expect("selected child phase");
        match phase.as_str() {
            "ownership" => two_engine_ownership_case(directory),
            "reopen" | "verify" => {
                for (file, id, body) in STORED_RECORDS {
                    let engine =
                        Engine::open(directory.join(file)).expect("fresh-process open").engine;
                    exact_record(&engine, id, body);
                    assert!(engine.read_get("absent", &ReadView::default()).unwrap().is_none());
                    let sentinel = format!("fresh-process {id}");
                    if phase == "reopen" {
                        engine
                            .write(&[named_document("restart-sentinel", &sentinel)])
                            .expect("write after reopen");
                    }
                    exact_record(&engine, "restart-sentinel", &sentinel);
                    engine.close().expect("fresh-process close");
                }
            }
            _ => panic!("unknown child phase {phase}"),
        }
        println!("SLICE50_CASE_COMPLETED phase={phase} pid={}", std::process::id());
        return;
    }
    let directory = tempfile::tempdir().expect("real stores");
    std::fs::create_dir(directory.path().join("alias-parent")).unwrap();
    for phase in ["ownership", "reopen", "verify"] {
        run_bounded_engine_case(directory.path(), phase);
    }
}

fn two_engine_ownership_case(directory: &std::path::Path) {
    let database = directory.join("held.sqlite");
    let alias = directory.join("alias-parent/../held.sqlite");
    let (entered, entered_rx) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let release_on_unwind = ReleaseProvider(release.clone());
    let provider =
        Arc::new(FirstCallHeld { calls: AtomicUsize::new(0), entered, release: Mutex::new(held) });
    let engine = Arc::new(
        Engine::open_with_test_choice_and_config_for_test(
            &alias,
            TestEmbedderChoice::Caller(provider.clone()),
            EngineConfig {
                embedder_pool_size: Some(1),
                embedder_call_timeout_ms: Some(200),
                ..EngineConfig::default()
            },
        )
        .expect("open aliased owner")
        .engine,
    );
    engine.write(&[named_document("held-record", "held side committed aé猫")]).unwrap();
    exact_record(&engine, "held-record", "held side committed aé猫");
    assert!(
        matches!(Engine::open(&database), Err(EngineOpenError::DatabaseLocked { .. })),
        "canonical path shares live ownership"
    );
    let first_engine = Arc::clone(&engine);
    let first = thread::spawn(move || first_engine.embed_text("held first call"));
    entered_rx.recv_timeout(Duration::from_secs(5)).expect("A provider entered");

    let sibling = Engine::open_with_test_choice_and_config_for_test(
        directory.join("sibling.sqlite"),
        TestEmbedderChoice::Caller(Arc::new(ShortServiceEmbedder)),
        EngineConfig::default(),
    )
    .expect("independent engine open")
    .engine;
    sibling
        .write(&[named_document("sibling-record", "independent sibling committed aé猫")])
        .unwrap();
    exact_record(&sibling, "sibling-record", "independent sibling committed aé猫");
    assert_eq!(sibling.embed_text("sibling has its own worker").unwrap(), vec![1.0; 8]);
    assert!(matches!(first.join().expect("started timeout waiter"), Err(EngineError::Embedder)));
    assert!(matches!(
        engine.embed_text("expires queued behind retained worker"),
        Err(EngineError::Overloaded)
    ));
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        1,
        "queued expiry cannot invoke provider or replace the live worker"
    );
    exact_record(&sibling, "sibling-record", "independent sibling committed aé猫");
    release.send(()).expect("release actual provider");
    assert_eq!(engine.embed_text("healthy after late return").unwrap(), vec![1.0; 8]);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    drop(release_on_unwind);

    let other_close = Arc::clone(&engine);
    let close = thread::spawn(move || other_close.close());
    engine.close().expect("concurrent close");
    close.join().expect("close actor").expect("other close");
    engine.close().expect("repeated close");
    assert!(matches!(
        engine.read_get("held-record", &ReadView::default()),
        Err(EngineError::Closing)
    ));
    sibling.close().expect("sibling close");
    // Explicit close, rather than Drop, must release both canonical locks.
    for (file, id, body) in STORED_RECORDS {
        let reopened = Engine::open(directory.join(file))
            .expect("lock released while closed handle is alive")
            .engine;
        exact_record(&reopened, id, body);
        reopened.close().expect("reacquired owner close");
    }
    drop(engine);
    drop(sibling);
}

struct OwnedRuntimeChild(std::process::Child);

impl Drop for OwnedRuntimeChild {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn run_bounded_engine_case(directory: &std::path::Path, phase: &str) {
    let stdout_path = directory.join(format!("{phase}.stdout"));
    let stderr_path = directory.join(format!("{phase}.stderr"));
    let mut child = OwnedRuntimeChild(
        std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "two_engine_dispatch_and_alias_close_recover_exact_state_in_fresh_process",
                "--nocapture",
            ])
            .env("FATHOMDB_SLICE50_CASE_DIR", directory)
            .env("FATHOMDB_SLICE50_CASE_PHASE", phase)
            .stdin(std::process::Stdio::null())
            .stdout(std::fs::File::create(&stdout_path).unwrap())
            .stderr(std::fs::File::create(&stderr_path).unwrap())
            .spawn()
            .expect("spawn exact runtime case"),
    );
    let deadline = Instant::now() + Duration::from_secs(60);
    let status = loop {
        if let Some(status) = child.0.try_wait().expect("observe child") {
            break status;
        }
        assert!(Instant::now() < deadline, "runtime case {phase} exceeded watchdog");
        thread::sleep(Duration::from_millis(10));
    };
    let stdout = std::fs::read_to_string(stdout_path).unwrap();
    let stderr = std::fs::read_to_string(stderr_path).unwrap();
    assert!(status.success(), "case={phase} status={status} stdout={stdout} stderr={stderr}");
    assert!(stdout.contains(&format!("SLICE50_CASE_COMPLETED phase={phase} pid={}", child.0.id())));
    assert!(stdout.contains("1 passed; 0 failed; 0 ignored"), "{stdout}");
    print!("{stdout}");
}
