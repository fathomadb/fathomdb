//! Real-engine resource and consuming-effect checks for the runtime matrix.

#![cfg(all(feature = "test-hooks", target_os = "linux"))]

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    EmbedderChoice, Engine, EngineConfig, EngineError, InitialState, PreparedWrite, SourceId,
};
use tempfile::TempDir;

static MATRIX_LOCK: Mutex<()> = Mutex::new(());
const DIMENSION: u32 = 8;

#[derive(Debug)]
struct HeldEmbedder {
    entered: mpsc::Sender<String>,
    gate: Arc<(Mutex<bool>, Condvar)>,
    active: AtomicUsize,
    peak: AtomicUsize,
}

struct ActiveCall<'a>(&'a AtomicUsize);

impl Drop for ActiveCall<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Embedder for HeldEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice90-matrix", "r1", DIMENSION)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        let _active = ActiveCall(&self.active);
        self.peak.fetch_max(active, Ordering::SeqCst);
        self.entered.send(text.to_owned()).expect("report provider entry");
        let (lock, ready) = &*self.gate;
        let mut released = lock.lock().expect("gate");
        while !*released {
            released = ready.wait(released).expect("gate wait");
        }
        Ok(vec![1.0; DIMENSION as usize])
    }
}

fn provider() -> (Arc<HeldEmbedder>, mpsc::Receiver<String>) {
    let (entered, observed) = mpsc::channel();
    (
        Arc::new(HeldEmbedder {
            entered,
            gate: Arc::new((Mutex::new(false), Condvar::new())),
            active: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
        }),
        observed,
    )
}

fn release(provider: &HeldEmbedder) {
    let (lock, ready) = &*provider.gate;
    *lock.lock().expect("gate") = true;
    ready.notify_all();
}

fn engine_threads() -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::from([("embed", 0), ("projection", 0), ("reader", 0)]);
    for task in std::fs::read_dir("/proc/self/task").expect("task inventory") {
        let name = match std::fs::read_to_string(task.expect("task").path().join("comm")) {
            Ok(name) => name,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => panic!("thread name: {error}"),
        };
        let role = if name.starts_with("fathomdb-embed") {
            Some("embed")
        } else if name.starts_with("fathomdb-projec") {
            Some("projection")
        } else if name.starts_with("fathomdb-reader") {
            Some("reader")
        } else {
            None
        };
        if let Some(role) = role {
            *counts.get_mut(role).expect("known role") += 1;
        }
    }
    counts
}

fn await_threads(expected: &BTreeMap<&'static str, usize>) {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let actual = engine_threads();
        if &actual == expected {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "thread inventory: expected {expected:?}, got {actual:?}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn expected_threads(
    baseline: &BTreeMap<&'static str, usize>,
    scheduler: usize,
    embed: usize,
) -> BTreeMap<&'static str, usize> {
    BTreeMap::from([
        ("embed", baseline["embed"] + embed),
        ("projection", baseline["projection"] + scheduler + 1),
        ("reader", baseline["reader"] + 8),
    ])
}

fn connection_inventory(engine: &Engine) -> String {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Ok(inventory) = engine.binding_connection_inventory_for_test() {
            return inventory;
        }
        assert!(Instant::now() < deadline, "SQLite inventory did not become idle");
        thread::sleep(Duration::from_millis(10));
    }
}

fn assert_live_connections(engine: &Engine, scheduler: u64) {
    let expected = format!("live=writer:1,readers:8,dispatcher:1,workers:{scheduler},probes:0");
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Ok(actual) = engine.d27_connection_inventory_for_test() {
            assert_eq!(actual, expected);
            return;
        }
        assert!(Instant::now() < deadline, "live SQLite roles did not become complete");
        thread::sleep(Duration::from_millis(10));
    }
}

fn database_descriptors(path: &Path) -> usize {
    std::fs::read_dir("/proc/self/fd")
        .expect("descriptor inventory")
        .flatten()
        .filter_map(|entry| std::fs::read_link(entry.path()).ok())
        .filter(|target| target == path)
        .count()
}

fn await_database_descriptors(path: &Path, expected: usize) {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let actual = database_descriptors(path);
        if actual == expected {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "database descriptors: expected {expected}, got {actual}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn document() -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".to_owned(),
        body: "projection shares the configured provider pool".to_owned(),
        source_id: SourceId::new("test:runtime-matrix").expect("source"),
        logical_id: None,
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

#[test]
fn default_engine_has_exact_five_worker_resources_and_twenty_waiting_slots() {
    let _lock = MATRIX_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = TempDir::new().expect("directory");
    let path = dir.path().join("default-2-5.sqlite");
    let baseline = engine_threads();
    let (embedder, entered) = provider();
    let opened = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Caller(embedder.clone()),
        EngineConfig::default(),
    )
    .expect("default real-engine open");
    let engine = Arc::new(opened.engine);
    assert_eq!(engine.config(), &EngineConfig::default());
    await_threads(&expected_threads(&baseline, 2, 5));
    assert_live_connections(&engine, 2);

    engine.begin_d27_observation_for_test(Instant::now());
    let mut callers = Vec::new();
    for index in 0..5 {
        let foreground = Arc::clone(&engine);
        callers.push(thread::spawn(move || foreground.embed_text(&format!("active-{index}"))));
    }
    for _ in 0..5 {
        entered.recv_timeout(Duration::from_secs(2)).expect("default provider entered");
    }
    assert_eq!(embedder.peak.load(Ordering::SeqCst), 5);
    for index in 0..20 {
        let foreground = Arc::clone(&engine);
        callers.push(thread::spawn(move || foreground.embed_text(&format!("waiting-{index}"))));
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let observed = engine.d27_observation_for_test().expect("engine observation");
        let waiting = observed
            .embed_dispatch_events
            .iter()
            .filter(|event| event.queued && event.started_ns.is_none())
            .count();
        if waiting == 20 {
            assert_eq!(observed.configuration_observation.source, "engine");
            assert_eq!(observed.configuration_observation.scheduler_runtime_threads, 2);
            assert_eq!(observed.configuration_observation.embedder_pool_size, 5);
            break;
        }
        assert!(Instant::now() < deadline, "default waiting capacity: expected 20, got {waiting}");
        thread::sleep(Duration::from_millis(10));
    }
    assert!(matches!(engine.embed_text("overflow"), Err(EngineError::Overloaded)));
    release(&embedder);
    for caller in callers {
        assert!(
            matches!(caller.join().expect("caller"), Ok(vector) if vector == [1.0; DIMENSION as usize])
        );
    }
    engine.close().expect("default close");
    await_threads(&baseline);
    await_database_descriptors(&path, 0);
}

#[test]
fn explicit_two_one_engine_has_exact_managed_roles_and_cleanup() {
    let _lock = MATRIX_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = TempDir::new().expect("directory");
    let path = dir.path().join("explicit-2-1.sqlite");
    let baseline = engine_threads();
    let (embedder, entered) = provider();
    let config = EngineConfig::default();
    let opened = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Caller(embedder.clone()),
        config.clone(),
    )
    .expect("2/1 real-engine open");
    let engine = Arc::new(opened.engine);
    assert_eq!(engine.config(), &config);
    await_threads(&expected_threads(&baseline, 2, 1));
    assert_live_connections(&engine, 2);
    assert_eq!(
        connection_inventory(&engine),
        "roles=writer:0,readers:0-7,dispatcher:0,workers:0-1;writer=autocommit;readers=8-autocommit;dispatcher=autocommit;workers=2-autocommit;creation=writer:1,readers:8,dispatcher:1,workers:2,probes:0"
    );
    await_database_descriptors(&path, 12);
    let foreground = Arc::clone(&engine);
    let caller = thread::spawn(move || foreground.embed_text("explicit 2/1 provider call"));
    assert_eq!(
        entered.recv_timeout(Duration::from_secs(2)).expect("provider entered"),
        "explicit 2/1 provider call"
    );
    release(&embedder);
    assert!(
        matches!(caller.join().expect("caller"), Ok(vector) if vector == [1.0; DIMENSION as usize])
    );
    assert_eq!(embedder.peak.load(Ordering::SeqCst), 1);
    engine.close().expect("2/1 close");
    await_threads(&baseline);
    await_database_descriptors(&path, 0);
    drop(engine);
    let reopened =
        Engine::open_with_choice_and_config(&path, EmbedderChoice::Caller(embedder), config)
            .expect("2/1 reopen");
    await_threads(&expected_threads(&baseline, 2, 1));
    assert_live_connections(&reopened.engine, 2);
    await_database_descriptors(&path, 12);
    reopened.engine.close().expect("2/1 reopen close");
    await_threads(&baseline);
    await_database_descriptors(&path, 0);
}

#[test]
fn configured_engine_matrix_has_exact_live_resources_and_reopen_cleanup() {
    let _lock = MATRIX_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = TempDir::new().expect("directory");
    let baseline = engine_threads();
    for (scheduler, pool) in [(1, 1), (2, 2), (4, 4), (64, 64)] {
        let path = dir.path().join(format!("matrix-{scheduler}-{pool}.sqlite"));
        let config = EngineConfig {
            scheduler_runtime_threads: Some(scheduler),
            embedder_pool_size: Some(pool),
            ..EngineConfig::default()
        };
        let (embedder, entered) = provider();
        let opened = Engine::open_with_choice_and_config(
            &path,
            EmbedderChoice::Caller(embedder.clone()),
            config.clone(),
        )
        .expect("configured real-engine open");
        let engine = Arc::new(opened.engine);
        assert_eq!(engine.config(), &config);
        await_threads(&expected_threads(&baseline, scheduler as usize, pool as usize));
        assert_live_connections(&engine, scheduler);

        if pool <= 4 {
            let mut callers = Vec::new();
            if pool == 2 {
                engine.configure_vector_kind_for_test("doc").expect("vector kind");
                let receipt = engine.write(&[document()]).expect("durable write");
                let first = entered
                    .recv_timeout(Duration::from_secs(2))
                    .expect("projection provider entered");
                assert!(first.contains("projection shares"), "first provider input: {first}");
                let foreground = Arc::clone(&engine);
                callers.push(thread::spawn(move || foreground.embed_text("foreground")));
                let second = entered
                    .recv_timeout(Duration::from_secs(2))
                    .expect("foreground provider entered");
                assert_eq!(second, "foreground");
                assert_eq!(embedder.peak.load(Ordering::SeqCst), 2);
                release(&embedder);
                for caller in callers {
                    assert!(
                        matches!(caller.join().expect("caller"), Ok(vector) if vector == [1.0; DIMENSION as usize])
                    );
                }
                engine.drain(3_000).expect("projection drain");
                assert!(engine.has_vector_for_cursor_for_test(receipt.cursor).expect("vector"));
            } else {
                for _ in 0..pool {
                    let foreground = Arc::clone(&engine);
                    callers.push(thread::spawn(move || foreground.embed_text("foreground")));
                }
                for _ in 0..pool {
                    entered.recv_timeout(Duration::from_secs(2)).expect("provider entered");
                }
                let peak = embedder.peak.load(Ordering::SeqCst);
                release(&embedder);
                for caller in callers {
                    assert!(
                        matches!(caller.join().expect("caller"), Ok(vector) if vector == [1.0; DIMENSION as usize])
                    );
                }
                assert_eq!(peak, pool as usize, "configured provider calls must overlap");
            }
        } else {
            release(&embedder);
        }

        engine.close().expect("configured close");
        await_threads(&baseline);
        await_database_descriptors(&path, 0);
        drop(engine);
        let reopened =
            Engine::open_with_choice_and_config(&path, EmbedderChoice::Caller(embedder), config)
                .expect("configured reopen");
        await_threads(&expected_threads(&baseline, scheduler as usize, pool as usize));
        assert_live_connections(&reopened.engine, scheduler);
        reopened.engine.close().expect("reopen close");
        await_threads(&baseline);
        await_database_descriptors(&path, 0);
    }
}

#[test]
fn no_provider_allocates_no_embed_workers_and_reopens_cleanly() {
    let _lock = MATRIX_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = TempDir::new().expect("directory");
    let path = dir.path().join("no-provider.sqlite");
    let baseline = engine_threads();
    let config = EngineConfig {
        scheduler_runtime_threads: Some(2),
        embedder_pool_size: Some(2),
        ..EngineConfig::default()
    };
    for _ in 0..2 {
        let opened =
            Engine::open_with_choice_and_config(&path, EmbedderChoice::None, config.clone())
                .expect("no-provider open");
        await_threads(&expected_threads(&baseline, 2, 0));
        assert_live_connections(&opened.engine, 2);
        await_database_descriptors(&path, 1 + 8 + 1 + 2);
        assert!(matches!(
            opened.engine.embed_text("absent"),
            Err(EngineError::EmbedderNotConfigured)
        ));
        let inventory = connection_inventory(&opened.engine);
        assert!(inventory.contains("workers:2"), "{inventory}");
        opened.engine.close().expect("no-provider close");
        await_threads(&baseline);
        await_database_descriptors(&path, 0);
    }
}
