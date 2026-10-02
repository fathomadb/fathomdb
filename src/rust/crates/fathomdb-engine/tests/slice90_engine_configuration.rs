#[cfg(debug_assertions)]
use std::sync::{Arc, Mutex};
#[cfg(debug_assertions)]
use std::time::{Duration, Instant};

#[cfg(debug_assertions)]
use fathomdb_engine::lifecycle::{
    Event, EventCategory, EventSource, Phase, SlowStatement, Subscriber,
};
use fathomdb_engine::{EmbedderChoice, Engine, EngineConfig, EngineOpenError, PreparedWrite};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

fn database_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

#[test]
fn requested_configuration_is_retained_without_setter_mutation() {
    let dir = TempDir::new().unwrap();
    let requested = EngineConfig {
        scheduler_runtime_threads: Some(4),
        embedder_pool_size: Some(3),
        embedder_call_timeout_ms: Some(500),
        provenance_row_cap: Some(0),
        slow_threshold_ms: Some(0),
    };
    let opened = Engine::open_with_choice_and_config(
        database_path(&dir, "configured"),
        EmbedderChoice::None,
        requested.clone(),
    )
    .expect("configured open");
    assert_eq!(opened.engine.config(), &requested);
    opened.engine.set_slow_threshold_ms(8).unwrap();
    assert_eq!(opened.engine.config(), &requested);
    opened.engine.close().unwrap();
}

#[test]
fn legacy_open_keeps_omitted_requested_values() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(database_path(&dir, "default")).expect("default open");
    assert_eq!(opened.engine.config(), &EngineConfig::default());
    opened.engine.close().unwrap();
}

#[test]
fn invalid_configuration_precedes_path_and_lock_side_effects() {
    let dir = TempDir::new().unwrap();
    let path = database_path(&dir, "missing-parent");
    let path = path.parent().unwrap().join("absent").join(path.file_name().unwrap());
    let invalid = EngineConfig { scheduler_runtime_threads: Some(0), ..EngineConfig::default() };
    let error = Engine::open_with_choice_and_config(&path, EmbedderChoice::None, invalid)
        .expect_err("invalid configuration");
    assert!(matches!(error, EngineOpenError::EngineConfiguration(_)));
    assert!(!path.parent().unwrap().exists(), "validation created the database parent");

    let held_path = database_path(&dir, "held");
    let held = Engine::open(&held_path).expect("hold lock");
    let invalid = EngineConfig { embedder_call_timeout_ms: Some(0), ..EngineConfig::default() };
    let error = Engine::open_with_choice_and_config(&held_path, EmbedderChoice::Default, invalid)
        .expect_err("invalid configuration must precede lock and embedder policy");
    assert!(matches!(error, EngineOpenError::EngineConfiguration(_)));
    held.engine.close().unwrap();
}

#[test]
fn every_setting_rejects_only_values_outside_its_contract() {
    let dir = TempDir::new().unwrap();
    let invalid = [
        EngineConfig { scheduler_runtime_threads: Some(65), ..EngineConfig::default() },
        EngineConfig { embedder_pool_size: Some(0), ..EngineConfig::default() },
        EngineConfig { embedder_pool_size: Some(65), ..EngineConfig::default() },
        EngineConfig { embedder_call_timeout_ms: Some(0), ..EngineConfig::default() },
        EngineConfig {
            embedder_call_timeout_ms: Some(u64::from(u32::MAX) + 1),
            ..EngineConfig::default()
        },
        EngineConfig { provenance_row_cap: Some(1_u64 << 53), ..EngineConfig::default() },
        EngineConfig { slow_threshold_ms: Some(1_u64 << 53), ..EngineConfig::default() },
    ];
    for (index, config) in invalid.into_iter().enumerate() {
        let path = database_path(&dir, &format!("invalid-{index}"));
        let error = Engine::open_with_choice_and_config(&path, EmbedderChoice::None, config)
            .expect_err("invalid configuration");
        assert!(matches!(error, EngineOpenError::EngineConfiguration(_)));
        assert!(!path.exists(), "invalid open created {path:?}");
    }
}

fn register_audit(engine: &Engine) {
    engine
        .write(&[PreparedWrite::AdminSchema {
            name: "audit".to_owned(),
            kind: "append_only_log".to_owned(),
            schema_json: "{}".to_owned(),
            retention_json: "{}".to_owned(),
        }])
        .expect("register audit collection");
}

fn append_audit(engine: &Engine, key: &str) {
    engine
        .write(&[PreparedWrite::OpStore {
            collection: "audit".to_owned(),
            record_key: key.to_owned(),
            schema_id: None,
            body: "{}".to_owned(),
        }])
        .expect("append audit row");
}

#[test]
fn open_time_provenance_zero_and_one_have_independent_retention() {
    let dir = TempDir::new().unwrap();
    let unlimited = Engine::open_with_choice_and_config(
        database_path(&dir, "unlimited"),
        EmbedderChoice::None,
        EngineConfig { provenance_row_cap: Some(0), ..EngineConfig::default() },
    )
    .expect("open unlimited")
    .engine;
    let capped = Engine::open_with_choice_and_config(
        database_path(&dir, "capped-one"),
        EmbedderChoice::None,
        EngineConfig { provenance_row_cap: Some(1), ..EngineConfig::default() },
    )
    .expect("open cap one")
    .engine;
    register_audit(&unlimited);
    register_audit(&capped);
    assert_eq!(unlimited.provenance_row_count_for_test().unwrap(), 0);
    assert_eq!(capped.provenance_row_count_for_test().unwrap(), 0);

    for (index, capped_count) in [1, 2, 1, 2].into_iter().enumerate() {
        let key = format!("{index:02}");
        append_audit(&unlimited, &key);
        append_audit(&capped, &key);
        assert_eq!(unlimited.provenance_row_count_for_test().unwrap(), index as u64 + 1);
        assert_eq!(capped.provenance_row_count_for_test().unwrap(), capped_count);
    }
    assert_eq!(
        unlimited.oldest_provenance_record_key_for_test("audit").unwrap().as_deref(),
        Some("00")
    );
    assert_eq!(
        capped.oldest_provenance_record_key_for_test("audit").unwrap().as_deref(),
        Some("02")
    );
    unlimited.close().unwrap();
    capped.close().unwrap();
}

#[test]
fn open_time_provenance_nondefault_cap_sweeps_then_allows_hysteresis() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open_with_choice_and_config(
        database_path(&dir, "capped-four"),
        EmbedderChoice::None,
        EngineConfig { provenance_row_cap: Some(4), ..EngineConfig::default() },
    )
    .expect("open cap four")
    .engine;
    register_audit(&engine);
    assert_eq!(engine.provenance_row_count_for_test().unwrap(), 0);

    for (index, expected) in [1, 2, 3, 4, 5, 4, 5, 4].into_iter().enumerate() {
        append_audit(&engine, &format!("{index:02}"));
        assert_eq!(engine.provenance_row_count_for_test().unwrap(), expected, "after row {index}");
    }
    assert_eq!(
        engine.oldest_provenance_record_key_for_test("audit").unwrap().as_deref(),
        Some("04")
    );
    engine.close().unwrap();
}

#[cfg(debug_assertions)]
#[derive(Default)]
struct SlowCapture {
    events: Mutex<Vec<Event>>,
    statements: Mutex<Vec<SlowStatement>>,
}

#[cfg(debug_assertions)]
impl Subscriber for SlowCapture {
    fn on_event(&self, event: &Event) {
        if event.phase == Phase::Slow {
            self.events.lock().unwrap().push(event.clone());
        }
    }

    fn on_slow_statement(&self, statement: &SlowStatement) {
        self.statements.lock().unwrap().push(statement.clone());
    }
}

#[cfg(debug_assertions)]
fn run_slow_statement(engine: &Engine, n: u64) {
    engine
        .execute_for_test(&format!(
            "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM c WHERE x < {n}) SELECT count(*) FROM c"
        ))
        .expect("execute recursive statement");
}

#[cfg(debug_assertions)]
fn calibrated_slow_cte_n(engine: &Engine) -> u64 {
    const PROBE_N: u64 = 1_000_000;
    let mut best = Duration::MAX;
    for _ in 0..2 {
        let started = Instant::now();
        run_slow_statement(engine, PROBE_N);
        best = best.min(started.elapsed());
    }
    let n = (PROBE_N as f64 * 500.0 / (best.as_secs_f64() * 1000.0).max(0.1)) as u64;
    n.clamp(100_000, 50_000_000)
}

#[cfg(debug_assertions)]
#[test]
fn open_time_slow_threshold_controls_operation_and_sqlite_events_until_setter_changes_effective_state(
) {
    let dir = TempDir::new().unwrap();
    let zero_config = EngineConfig { slow_threshold_ms: Some(0), ..EngineConfig::default() };
    let high_config = EngineConfig { slow_threshold_ms: Some(5_000), ..EngineConfig::default() };
    let zero = Engine::open_with_choice_and_config(
        database_path(&dir, "slow-zero"),
        EmbedderChoice::None,
        zero_config.clone(),
    )
    .expect("open zero threshold")
    .engine;
    let high = Engine::open_with_choice_and_config(
        database_path(&dir, "slow-high"),
        EmbedderChoice::None,
        high_config.clone(),
    )
    .expect("open high threshold")
    .engine;
    let cte_n = calibrated_slow_cte_n(&high);
    let zero_capture = Arc::new(SlowCapture::default());
    let high_capture = Arc::new(SlowCapture::default());
    let _zero_subscription = zero.subscribe(zero_capture.clone());
    let _high_subscription = high.subscribe(high_capture.clone());

    run_slow_statement(&zero, cte_n);
    run_slow_statement(&high, cte_n);
    assert_eq!(zero_capture.events.lock().unwrap().len(), 1);
    assert_eq!(zero_capture.statements.lock().unwrap().len(), 1);
    assert!(high_capture.events.lock().unwrap().is_empty());
    assert!(high_capture.statements.lock().unwrap().is_empty());

    high.set_slow_threshold_ms(0).unwrap();
    run_slow_statement(&high, cte_n);
    assert_eq!(high_capture.events.lock().unwrap().len(), 1);
    assert_eq!(high_capture.statements.lock().unwrap().len(), 1);
    assert_eq!(high.config(), &high_config);

    zero.set_slow_threshold_ms(5_000).unwrap();
    run_slow_statement(&zero, cte_n);
    assert_eq!(zero_capture.events.lock().unwrap().len(), 1);
    assert_eq!(zero_capture.statements.lock().unwrap().len(), 1);
    assert_eq!(zero.config(), &zero_config);
    let events = high_capture.events.lock().unwrap();
    assert_eq!(events[0].category, EventCategory::Search);
    assert_eq!(events[0].source, EventSource::Engine);
    let statements = high_capture.statements.lock().unwrap();
    assert!(statements[0].statement.contains("RECURSIVE"));
    assert!(statements[0].wall_clock_ms > 100, "fixture must distinguish 5s from default 100ms");
    zero.close().unwrap();
    high.close().unwrap();
}
