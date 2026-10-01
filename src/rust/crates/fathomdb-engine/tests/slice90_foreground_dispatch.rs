//! Foreground provider calls share the engine's bounded inference deadline.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    EmbedderChoice, Engine, EngineConfig, EngineError, InitialState, PreparedWrite, ReadContextV1,
    ReadView, SearchFilter, SourceId,
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
    }
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
                .recv_timeout(Duration::from_secs(2))
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
    let opened = Engine::open_with_choice_and_config(
        database,
        EmbedderChoice::Caller(provider),
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
    let engine = Engine::open_with_choice_and_config(
        database,
        EmbedderChoice::Caller(Arc::new(FallibleEmbedder)),
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
        Engine::open_with_choice_and_config(
            database,
            EmbedderChoice::Caller(provider.clone()),
            EngineConfig { embedder_call_timeout_ms: Some(120), ..EngineConfig::default() },
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
        Engine::open_with_choice_and_config(
            database,
            EmbedderChoice::Caller(provider),
            EngineConfig { embedder_call_timeout_ms: Some(5_000), ..EngineConfig::default() },
        )
        .expect("open")
        .engine,
    );
    let pending_engine = Arc::clone(&engine);
    let pending = thread::spawn(move || pending_engine.embed_text("started before close"));
    entered_rx.recv_timeout(Duration::from_secs(1)).expect("provider entered");
    let started = Instant::now();
    engine.close().expect("current close cancels waiter");
    assert!(matches!(pending.join().expect("join pending"), Err(EngineError::Closing)));
    assert!(started.elapsed() < Duration::from_secs(1), "close must wake direct waiter");
    release.send(()).expect("release provider");
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
    let engine = Engine::open_with_choice_and_config(
        database,
        EmbedderChoice::Caller(provider),
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
fn ordinary_and_frozen_search_keep_sparse_fallback_on_provider_error_or_panic() {
    let directory = tempfile::tempdir().expect("test directory");
    let database = directory.path().join("search-failure.sqlite");
    let engine = Engine::open_with_choice_and_config(
        database,
        EmbedderChoice::Caller(Arc::new(FallibleEmbedder)),
        EngineConfig::default(),
    )
    .expect("open")
    .engine;
    engine.configure_vector_kind_for_test("note").expect("vector kind");
    let errored =
        engine.write(&[document("provider error appears here")]).expect("write error row");
    let panicked =
        engine.write(&[document("provider panic appears here")]).expect("write panic row");
    engine.drain(2_000).expect("project rows");
    let context =
        ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("context");
    let frozen = engine.freeze_read_context(&context).expect("freeze");

    for (query, expected) in
        [("provider error", errored.cursor), ("provider panic", panicked.cursor)]
    {
        let ordinary = engine.search(query).expect("ordinary sparse fallback");
        assert!(ordinary.results.iter().any(|hit| hit.write_cursor == expected));
        let reader = engine
            .search_frozen(query, &frozen, 0, false, 0.3, 0, false, 10)
            .expect("frozen sparse fallback");
        assert!(reader.results.iter().any(|hit| hit.write_cursor == expected));
    }
    assert_eq!(
        engine.embed_text("healthy after search failure").expect("worker reused"),
        vec![1.0; 8]
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
        Engine::open_with_choice_and_config(
            database,
            EmbedderChoice::Caller(provider),
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
