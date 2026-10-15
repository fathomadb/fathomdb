//! 0.8.28 Slice 31 § 15 (owner rulings Q33 and Q35): the CUDA pool settings
//! are open settings, fixed once per process. A later open that sets one to
//! a different value fails with an invalid-argument conflict naming the
//! setting; an open that omits them never fails. The rule holds on every
//! host, with or without a GPU.
//!
//! The configuration in force is process-wide, so each case runs in its own
//! child process of this test binary.

use std::process::Command;

use fathomdb_embedder::{CudaPoolSettingConflict, CudaPoolSettingValue, PoolSettingSource};
use fathomdb_engine::{
    CudaPoolMode, EmbedderChoice, Engine, EngineConfig, EngineConfigurationError, EngineOpenError,
    OpenedEngine, ReleaseThreshold,
};
use tempfile::TempDir;

const CHILD_ENV: &str = "FATHOMDB_TEST_SLICE31_POOL_SETTINGS_CHILD";
const MIB: u64 = 1 << 20;

fn open(dir: &TempDir, name: &str, config: EngineConfig) -> Result<OpenedEngine, EngineOpenError> {
    Engine::open_with_choice_and_config(dir.path().join(name), EmbedderChoice::None, config)
}

fn pool(mode: Option<CudaPoolMode>) -> EngineConfig {
    EngineConfig { cuda_pool: mode, ..EngineConfig::default() }
}

fn conflict(error: EngineOpenError) -> CudaPoolSettingConflict {
    match error {
        EngineOpenError::EngineConfiguration(
            EngineConfigurationError::CudaPoolSettingConflict(conflict),
        ) => conflict,
        other => panic!("expected a pool setting conflict, got {other:?}"),
    }
}

/// Runs `body` in a fresh child process when this is the parent, and in
/// this process when this is that child.
fn in_child(test_name: &str, env: &[(&str, &str)], body: fn()) {
    if std::env::var_os(CHILD_ENV).is_some() {
        body();
        return;
    }
    let exe = std::env::current_exe().expect("test binary");
    let mut command = Command::new(exe);
    command
        .args(["--exact", test_name, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, "1")
        .env_remove("FATHOMDB_POOL_MODE")
        .env_remove("FATHOMDB_POOL_MAXSIZE")
        .env_remove("FATHOMDB_POOL_RELEASE_THRESHOLD");
    for (name, value) in env {
        command.env(name, value);
    }
    let output = command.output().expect("child process");
    assert!(
        output.status.success(),
        "child {test_name} failed: {:?}\n--- stdout\n{}\n--- stderr\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn a_second_open_with_a_different_cuda_pool_fails_and_the_first_engine_still_works() {
    in_child(
        "a_second_open_with_a_different_cuda_pool_fails_and_the_first_engine_still_works",
        &[],
        || {
            let dir = TempDir::new().unwrap();
            let first = open(&dir, "first.sqlite", pool(Some(CudaPoolMode::On))).expect("first");
            let error = open(&dir, "second.sqlite", pool(Some(CudaPoolMode::Off)))
                .expect_err("a different mode must conflict");
            assert_eq!(
                error.to_string(),
                "cuda_pool=off conflicts with cuda_pool=on already in force in this process \
                 (from sdk); open with the same value or omit it"
            );
            let conflict = conflict(error);
            assert_eq!(conflict.setting, "cuda_pool");
            assert_eq!(conflict.requested, CudaPoolSettingValue::Mode(CudaPoolMode::Off));
            assert_eq!(conflict.in_force, Some(CudaPoolSettingValue::Mode(CudaPoolMode::On)));
            assert_eq!(conflict.source, Some(PoolSettingSource::Sdk));
            assert!(
                !dir.path().join("second.sqlite").exists(),
                "the conflict must precede the database's creation"
            );
            first.engine.drain(10_000).expect("the first engine still works");
            first.engine.close().unwrap();
        },
    );
}

#[test]
fn the_same_settings_pass_and_omitting_them_never_fails() {
    in_child(
        "the_same_settings_pass_and_omitting_them_never_fails",
        &[("FATHOMDB_POOL_MODE", "off")],
        || {
            let dir = TempDir::new().unwrap();
            let config = EngineConfig {
                cuda_pool: Some(CudaPoolMode::On),
                cuda_pool_max_size: Some(64 * MIB),
                cuda_pool_release_threshold: Some(ReleaseThreshold::Max),
                ..EngineConfig::default()
            };
            let first = open(&dir, "first.sqlite", config.clone()).expect("first");
            assert_eq!(first.engine.config(), &config, "read-back of all three settings");
            let same = open(&dir, "same.sqlite", config).expect("the same settings pass");
            let omitted = open(&dir, "omitted.sqlite", EngineConfig::default())
                .expect("an open that omits the settings never conflicts");
            assert_eq!(omitted.engine.config().cuda_pool, None);
            for opened in [first, same, omitted] {
                opened.engine.close().unwrap();
            }
        },
    );
}

#[test]
fn a_different_size_or_threshold_conflicts_naming_the_setting() {
    in_child("a_different_size_or_threshold_conflicts_naming_the_setting", &[], || {
        let dir = TempDir::new().unwrap();
        let first = open(
            &dir,
            "first.sqlite",
            EngineConfig { cuda_pool_max_size: Some(64 * MIB), ..EngineConfig::default() },
        )
        .expect("first");
        let size = conflict(
            open(
                &dir,
                "size.sqlite",
                EngineConfig { cuda_pool_max_size: Some(128 * MIB), ..EngineConfig::default() },
            )
            .expect_err("a different size"),
        );
        assert_eq!(size.setting, "cuda_pool_max_size");
        assert_eq!(size.in_force, Some(CudaPoolSettingValue::MaxSize(64 * MIB)));
        let threshold = conflict(
            open(
                &dir,
                "threshold.sqlite",
                EngineConfig {
                    cuda_pool_release_threshold: Some(ReleaseThreshold::Max),
                    ..EngineConfig::default()
                },
            )
            .expect_err("a different threshold"),
        );
        assert_eq!(threshold.setting, "cuda_pool_release_threshold");
        assert_eq!(
            threshold.in_force,
            Some(CudaPoolSettingValue::ReleaseThreshold(ReleaseThreshold::Zero))
        );
        assert_eq!(threshold.source, Some(PoolSettingSource::Default));
        first.engine.close().unwrap();
    });
}

#[test]
fn a_pool_size_below_the_minimum_is_out_of_range_before_any_side_effect() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("absent").join("small.sqlite");
    let error = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::None,
        EngineConfig { cuda_pool_max_size: Some(32 * MIB - 1), ..EngineConfig::default() },
    )
    .expect_err("too small");
    assert!(
        matches!(
            error,
            EngineOpenError::EngineConfiguration(EngineConfigurationError::OutOfRange {
                field: "cuda_pool_max_size",
                min,
                max: u64::MAX,
                ..
            }) if min == 32 * MIB
        ),
        "{error:?}"
    );
    assert!(!path.parent().unwrap().exists());
}

#[test]
fn an_invalid_env_mode_fixed_at_the_first_device_build_conflicts_with_an_explicit_open() {
    in_child(
        "an_invalid_env_mode_fixed_at_the_first_device_build_conflicts_with_an_explicit_open",
        &[("FATHOMDB_POOL_MODE", "bogus")],
        || {
            let dir = TempDir::new().unwrap();
            // What the process's first CUDA device build does when no open
            // set the configuration.
            fathomdb_embedder::fix_pool_config_from_env();
            let error = open(&dir, "explicit.sqlite", pool(Some(CudaPoolMode::Auto)))
                .expect_err("an invalid env mode in force conflicts");
            assert_eq!(
                error.to_string(),
                "cuda_pool=auto conflicts with the invalid FATHOMDB_POOL_MODE this process \
                 started with; open with the same value or omit it"
            );
            open(&dir, "omitted.sqlite", EngineConfig::default())
                .expect("an open that omits it still opens")
                .engine
                .close()
                .unwrap();
        },
    );
}

#[test]
fn the_settings_are_in_force_before_the_device_policy_probe_runs() {
    in_child(
        "the_settings_are_in_force_before_the_device_policy_probe_runs",
        &[("FATHOMDB_EMBED_DEVICE", "not-a-device")],
        || {
            let dir = TempDir::new().unwrap();
            // The default-embedder open fails at (or, without the feature,
            // instead of) the device-policy probe; its setting stays in force.
            Engine::open_with_choice_and_config(
                dir.path().join("probe.sqlite"),
                EmbedderChoice::Default,
                pool(Some(CudaPoolMode::Off)),
            )
            .expect_err("the device policy or the missing default embedder fails the open");
            let conflict = conflict(
                open(&dir, "later.sqlite", pool(Some(CudaPoolMode::On)))
                    .expect_err("the failed open's mode is kept"),
            );
            assert_eq!(conflict.in_force, Some(CudaPoolSettingValue::Mode(CudaPoolMode::Off)));
            open(&dir, "same.sqlite", pool(Some(CudaPoolMode::Off)))
                .expect("the same mode passes")
                .engine
                .close()
                .unwrap();
        },
    );
}
