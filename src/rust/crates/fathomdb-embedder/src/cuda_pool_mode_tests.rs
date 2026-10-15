//! 0.8.28 Slice 31 § 15 (owner rulings Q33 and Q35): the pool settings on
//! the SDK surface. One configuration is in force per process: the first
//! open that passes any setting, or else the environment at the first device
//! build, fixes all three; a later open that sets one to a different value
//! conflicts. These tests drive the pure rule; none touches the process
//! static.

use super::*;

const MIB: u64 = 1 << 20;

fn env(mode: Option<&str>, max_size: Option<&str>, threshold: Option<&str>) -> RawPoolSettings {
    RawPoolSettings {
        mode: mode.map(str::to_owned),
        max_size: max_size.map(str::to_owned),
        release_threshold: threshold.map(str::to_owned),
    }
}

fn request(
    mode: Option<PoolMode>,
    max_size: Option<u64>,
    threshold: Option<ReleaseThreshold>,
) -> CudaPoolRequest {
    CudaPoolRequest { mode, max_size, release_threshold: threshold }
}

fn set<T>(value: T, source: PoolSettingSource) -> Field<T> {
    Field::Set { value, source }
}

fn first(request: &CudaPoolRequest, env: &RawPoolSettings) -> PoolConfigInForce {
    resolve_pool_config(None, request, env).expect("nothing in force never conflicts")
}

#[test]
fn the_mode_round_trips_its_three_strings() {
    for (mode, name) in [(PoolMode::Auto, "auto"), (PoolMode::On, "on"), (PoolMode::Off, "off")] {
        assert_eq!(mode.as_str(), name);
        assert_eq!(name.parse::<PoolMode>(), Ok(mode));
    }
    for bad in ["", "AUTO", " on", "true", "0"] {
        assert!(bad.parse::<PoolMode>().is_err(), "{bad:?}");
    }
    assert_eq!(PoolSettingSource::Sdk.as_str(), "sdk");
    assert_eq!(PoolSettingSource::Env.as_str(), "env");
    assert_eq!(PoolSettingSource::Default.as_str(), "default");
    assert_eq!(CUDA_POOL_MAX_SIZE_MIN, 32 * MIB);
}

#[test]
fn the_first_configuration_takes_each_field_from_the_sdk_then_the_env_then_the_default() {
    let config = first(&request(None, None, None), &env(None, None, None));
    assert_eq!(config.mode, set(PoolMode::Auto, PoolSettingSource::Default));
    assert_eq!(config.max_size, Field::Default);
    assert_eq!(config.release_threshold, set(ReleaseThreshold::Zero, PoolSettingSource::Default));

    let config = first(&request(None, None, None), &env(Some("on"), Some("1G"), Some("max")));
    assert_eq!(config.mode, set(PoolMode::On, PoolSettingSource::Env));
    assert_eq!(config.max_size, set(1 << 30, PoolSettingSource::Env));
    assert_eq!(config.release_threshold, set(ReleaseThreshold::Max, PoolSettingSource::Env));

    // Mixed sources: mode from the SDK, size from the env, threshold default.
    let config =
        first(&request(Some(PoolMode::Off), None, None), &env(Some("on"), Some("64M"), None));
    assert_eq!(config.mode, set(PoolMode::Off, PoolSettingSource::Sdk));
    assert_eq!(config.max_size, set(64 * MIB, PoolSettingSource::Env));
    assert_eq!(config.release_threshold, set(ReleaseThreshold::Zero, PoolSettingSource::Default));
}

#[test]
fn a_malformed_env_value_is_kept_as_invalid_unless_the_sdk_sets_that_field() {
    let config = first(&request(None, None, None), &env(Some("bogus"), Some("3X"), Some("1")));
    assert_eq!(config.mode, Field::InvalidEnv("bogus".to_owned()));
    assert_eq!(config.max_size, Field::InvalidEnv("3X".to_owned()));
    assert_eq!(config.release_threshold, Field::InvalidEnv("1".to_owned()));

    let config = first(
        &request(Some(PoolMode::On), Some(64 * MIB), Some(ReleaseThreshold::Max)),
        &env(Some("bogus"), Some("3X"), Some("1")),
    );
    assert_eq!(config.mode, set(PoolMode::On, PoolSettingSource::Sdk));
    assert_eq!(config.max_size, set(64 * MIB, PoolSettingSource::Sdk));
    assert_eq!(config.release_threshold, set(ReleaseThreshold::Max, PoolSettingSource::Sdk));
}

#[test]
fn a_later_open_passes_when_every_field_it_sets_matches_and_never_conflicts_by_omission() {
    let in_force = first(&request(Some(PoolMode::On), None, None), &env(None, Some("1G"), None));
    let none = request(None, None, None);
    assert_eq!(
        resolve_pool_config(Some(&in_force), &none, &env(Some("off"), None, None)),
        Ok(in_force.clone())
    );
    let same = request(Some(PoolMode::On), Some(1 << 30), Some(ReleaseThreshold::Zero));
    assert_eq!(
        resolve_pool_config(Some(&in_force), &same, &env(None, None, None)),
        Ok(in_force.clone())
    );
}

#[test]
fn defaults_that_are_constants_accept_an_explicit_request_for_the_same_value() {
    let in_force = first(&request(None, None, None), &env(None, None, None));
    let explicit_defaults = request(Some(PoolMode::Auto), None, Some(ReleaseThreshold::Zero));
    assert_eq!(
        resolve_pool_config(Some(&in_force), &explicit_defaults, &env(None, None, None)),
        Ok(in_force.clone())
    );
}

#[test]
fn each_different_field_conflicts_and_names_the_value_in_force() {
    let in_force =
        first(&request(Some(PoolMode::Auto), Some(64 * MIB), None), &env(None, None, Some("max")));
    let conflict = resolve_pool_config(
        Some(&in_force),
        &request(Some(PoolMode::Off), None, None),
        &env(None, None, None),
    )
    .unwrap_err();
    assert_eq!(
        conflict,
        CudaPoolSettingConflict {
            setting: "cuda_pool",
            requested: CudaPoolSettingValue::Mode(PoolMode::Off),
            in_force: Some(CudaPoolSettingValue::Mode(PoolMode::Auto)),
            source: Some(PoolSettingSource::Sdk),
        }
    );
    assert_eq!(
        conflict.to_string(),
        "cuda_pool=off conflicts with cuda_pool=auto already in force in this process \
         (from sdk); open with the same value or omit it"
    );

    let conflict = resolve_pool_config(
        Some(&in_force),
        &request(None, Some(128 * MIB), None),
        &env(None, None, None),
    )
    .unwrap_err();
    assert_eq!(conflict.setting, "cuda_pool_max_size");
    assert_eq!(conflict.in_force, Some(CudaPoolSettingValue::MaxSize(64 * MIB)));
    assert_eq!(
        conflict.to_string(),
        "cuda_pool_max_size=134217728 conflicts with cuda_pool_max_size=67108864 already in \
         force in this process (from sdk); open with the same value or omit it"
    );

    let conflict = resolve_pool_config(
        Some(&in_force),
        &request(None, None, Some(ReleaseThreshold::Zero)),
        &env(None, None, None),
    )
    .unwrap_err();
    assert_eq!(conflict.setting, "cuda_pool_release_threshold");
    assert_eq!(conflict.source, Some(PoolSettingSource::Env));
    assert_eq!(
        conflict.to_string(),
        "cuda_pool_release_threshold=0 conflicts with cuda_pool_release_threshold=max already \
         in force in this process (from env); open with the same value or omit it"
    );
}

#[test]
fn the_first_conflicting_setting_is_named() {
    let in_force =
        first(&request(Some(PoolMode::On), Some(64 * MIB), None), &env(None, None, None));
    let conflict = resolve_pool_config(
        Some(&in_force),
        &request(Some(PoolMode::Off), Some(128 * MIB), None),
        &env(None, None, None),
    )
    .unwrap_err();
    assert_eq!(conflict.setting, "cuda_pool");
}

#[test]
fn a_device_derived_size_conflicts_with_any_explicit_size() {
    let in_force = first(&request(None, None, None), &env(None, None, None));
    let conflict = resolve_pool_config(
        Some(&in_force),
        &request(None, Some(3 << 30), None),
        &env(None, None, None),
    )
    .unwrap_err();
    assert_eq!(conflict.in_force, None);
    assert_eq!(conflict.source, Some(PoolSettingSource::Default));
    assert_eq!(
        conflict.to_string(),
        "cuda_pool_max_size=3221225472 conflicts with the device-derived default \
         cuda_pool_max_size already in force; open with the same value or omit it"
    );
}

#[test]
fn an_invalid_env_value_in_force_conflicts_with_any_explicit_value() {
    let in_force = first(&request(None, None, None), &env(Some("bogus"), None, None));
    for requested in [PoolMode::Auto, PoolMode::On, PoolMode::Off] {
        let conflict = resolve_pool_config(
            Some(&in_force),
            &request(Some(requested), None, None),
            &env(None, None, None),
        )
        .unwrap_err();
        assert_eq!(conflict.in_force, None);
        assert_eq!(conflict.source, None);
        assert_eq!(
            conflict.to_string(),
            format!(
                "cuda_pool={} conflicts with the invalid FATHOMDB_POOL_MODE this process \
                 started with; open with the same value or omit it",
                requested.as_str()
            )
        );
    }
}

#[test]
fn every_pair_of_different_modes_conflicts() {
    let modes = [PoolMode::Auto, PoolMode::On, PoolMode::Off];
    for in_force_mode in modes {
        let in_force = first(&request(Some(in_force_mode), None, None), &env(None, None, None));
        for requested in modes {
            let result = resolve_pool_config(
                Some(&in_force),
                &request(Some(requested), None, None),
                &env(None, None, None),
            );
            assert_eq!(
                result.is_ok(),
                requested == in_force_mode,
                "{in_force_mode:?} {requested:?}"
            );
        }
    }
}

#[test]
fn the_configuration_renders_as_the_raw_settings_the_decision_parses() {
    let config = first(
        &request(Some(PoolMode::On), Some(64 * MIB), Some(ReleaseThreshold::Max)),
        &env(None, None, None),
    );
    assert_eq!(render_raw_settings(&config), env(Some("on"), Some("67108864"), Some("max")));
    let config = first(&request(None, None, None), &env(None, None, None));
    assert_eq!(render_raw_settings(&config), env(Some("auto"), None, Some("0")));
    let config = first(&request(None, None, None), &env(Some("bogus"), Some("3X"), Some("1")));
    assert_eq!(render_raw_settings(&config), env(Some("bogus"), Some("3X"), Some("1")));
    // `off` still wins over an invalid size in the frozen gates.
    let config = first(&request(Some(PoolMode::Off), None, None), &env(None, Some("3X"), None));
    assert_eq!(
        pre_device_gates(&render_raw_settings(&config), ModuleLoadInit::Ran).map(|_| ()),
        Err(CudaAllocatorReason::ModeOff)
    );
}

#[test]
fn the_report_fields_read_each_field_and_its_source() {
    let config = first(&request(Some(PoolMode::On), None, None), &env(None, Some("1G"), None));
    let fields = config.report_fields();
    assert_eq!(fields.mode, Some(PoolMode::On));
    assert_eq!(fields.mode_source, Some(PoolSettingSource::Sdk));
    assert_eq!(fields.max_size_source, Some(PoolSettingSource::Env));
    assert_eq!(fields.release_threshold_source, Some(PoolSettingSource::Default));
    let invalid = first(&request(None, None, None), &env(Some("bogus"), None, None));
    let fields = invalid.report_fields();
    assert_eq!(fields.mode, None);
    assert_eq!(fields.mode_source, Some(PoolSettingSource::Env));
    assert_eq!(fields.max_size_source, Some(PoolSettingSource::Default));
}
