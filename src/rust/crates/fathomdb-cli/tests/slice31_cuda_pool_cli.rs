//! 0.8.28 Slice 31 § 15: the CUDA pool settings on the CLI. The three doctor
//! verbs that build a CUDA device take `--cuda-pool`,
//! `--cuda-pool-max-size` and `--cuda-pool-release-threshold`; the flag wins
//! over the environment. `doctor cuda-libraries` and database verbs take
//! none. Any host: on a build without CUDA the allocator record returns
//! early, but it still names the configuration in force.

use std::process::{Command, Output};

fn fathomdb(args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fathomdb"));
    command
        .args(args)
        .env_remove("FATHOMDB_POOL_MODE")
        .env_remove("FATHOMDB_POOL_MAXSIZE")
        .env_remove("FATHOMDB_POOL_RELEASE_THRESHOLD")
        .env("FATHOMDB_EMBED_DEVICE", "cpu")
        .env("FATHOMDB_RERANK_DEVICE", "cpu");
    for (name, value) in env {
        command.env(name, value);
    }
    command.output().expect("run fathomdb")
}

fn json(output: &Output) -> serde_json::Value {
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("one JSON object")
}

#[test]
fn the_cuda_pool_flag_wins_over_the_environment_in_the_allocator_record() {
    let output = fathomdb(
        &["doctor", "cuda-allocator", "--json", "--cuda-pool", "off"],
        &[("FATHOMDB_POOL_MODE", "on"), ("FATHOMDB_POOL_RELEASE_THRESHOLD", "max")],
    );
    let record = json(&output);
    assert_eq!(record["pool_mode"], "off");
    assert_eq!(record["pool_mode_source"], "sdk");
    assert_eq!(record["pool_max_size_source"], "default");
    assert_eq!(record["pool_release_threshold_source"], "env");
    assert!(record.get("mode").is_none(), "the raw `mode` field is replaced: {record}");
    // serde_json's map sorts keys, so the order is read from the raw line.
    let raw = String::from_utf8_lossy(&output.stdout);
    let positions: Vec<usize> = [
        "schema_version",
        "built",
        "module_load_init",
        "pool_mode",
        "pool_mode_source",
        "path",
        "reason",
        "pool_max_size_bytes",
        "pool_max_size_source",
        "release_threshold",
        "pool_release_threshold_source",
        "cuda_context_state",
    ]
    .iter()
    .map(|key| raw.find(&format!("\"{key}\":")).unwrap_or_else(|| panic!("missing {key}: {raw}")))
    .collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "key order: {raw}");
    assert_eq!(record.as_object().expect("object").len(), positions.len(), "{raw}");
}

#[test]
fn without_a_flag_the_allocator_record_reads_the_environment_or_the_default() {
    let record = json(&fathomdb(&["doctor", "cuda-allocator", "--json"], &[]));
    assert_eq!(record["pool_mode"], "auto");
    assert_eq!(record["pool_mode_source"], "default");
    let record =
        json(&fathomdb(&["doctor", "cuda-allocator", "--json"], &[("FATHOMDB_POOL_MODE", "on")]));
    assert_eq!(record["pool_mode"], "on");
    assert_eq!(record["pool_mode_source"], "env");
    let record = json(&fathomdb(
        &["doctor", "cuda-allocator", "--json", "--cuda-pool-max-size", "64M"],
        &[],
    ));
    assert_eq!(record["pool_max_size_source"], "sdk");
}

#[test]
fn the_pool_flags_parse_on_the_three_device_doctors_only() {
    for verb in ["gpu", "reranker-gpu", "cuda-allocator"] {
        let output = fathomdb(&["doctor", verb, "--help"], &[]);
        let help = String::from_utf8_lossy(&output.stdout);
        for flag in ["--cuda-pool <", "--cuda-pool-max-size", "--cuda-pool-release-threshold"] {
            assert!(help.contains(flag), "doctor {verb} --help lacks {flag}:\n{help}");
        }
    }
    let output = fathomdb(&["doctor", "cuda-libraries", "--cuda-pool", "off"], &[]);
    assert_eq!(output.status.code(), Some(2), "cuda-libraries must not take --cuda-pool");
    let output = fathomdb(&["doctor", "check-integrity", "--cuda-pool", "off", "x.sqlite"], &[]);
    assert_eq!(output.status.code(), Some(2), "a database verb must not take --cuda-pool");
}

#[test]
fn an_invalid_pool_flag_is_a_usage_error() {
    for args in [
        ["doctor", "cuda-allocator", "--cuda-pool", "AUTO"],
        ["doctor", "cuda-allocator", "--cuda-pool-max-size", "3X"],
        ["doctor", "cuda-allocator", "--cuda-pool-max-size", "16M"],
        ["doctor", "cuda-allocator", "--cuda-pool-release-threshold", "1"],
    ] {
        let output = fathomdb(&args, &[]);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
    }
}
