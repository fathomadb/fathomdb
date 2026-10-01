use fathomdb_engine::{EmbedderChoice, Engine, EngineConfig, EngineOpenError};
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
