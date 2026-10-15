//! 0.8.28 Slice 31 D-19 (design § 14.2, owner rulings Q30 and Q36): a plain
//! Rust process that opens through the SDK reaches the Tegra private CUDA
//! pool without calling any init function. The decision admits the `cuInit`
//! the SDK's own device probe made.
//!
//! Orin only: the private pool exists only on the Jetson AGX Orin 64 GB, so
//! each test is `#[ignore]` and runs as its own child process (the decision
//! is process-wide). The build names only `embed-cuda`, which implies
//! `tegra-pool` (design § 14.3):
//!
//! ```text
//! flock /tmp/fathomdb-gpu.lock env FATHOMDB_REQUIRE_LIVE=1 cargo test --locked -p fathomdb-sdk \
//!   --features embed-cuda --test slice31_sdk_probe_init_cuda -- --ignored --nocapture --test-threads=1
//! ```
#![cfg(all(target_os = "linux", feature = "embed-cuda"))]

use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Output};

use fathomdb_sdk::{
    CudaAllocatorPath, CudaPoolMode, EffectiveEmbedDevice, Engine, ErrorKind, InitialState,
    OpenOptions, PreparedWrite, ReleaseThreshold, SearchOptions, SourceId,
};

const CHILD_ENV: &str = "FATHOMDB_TEST_SLICE31_SDK_CHILD";

/// The child: open through the SDK with the default embedder on `cuda:0`,
/// print the allocator report, write, search, close and exit.
fn sdk_child(mode: &str) {
    let mut options = OpenOptions { use_default_embedder: true, ..OpenOptions::default() };
    match mode {
        "pool-off" => options.config.cuda_pool = Some(CudaPoolMode::Off),
        "pool-on" => options.config.cuda_pool = Some(CudaPoolMode::On),
        "max" => options.config.cuda_pool_release_threshold = Some(ReleaseThreshold::Max),
        "size-sdk" => options.config.cuda_pool_max_size = Some(1 << 30),
        "size-huge" => options.config.cuda_pool_max_size = Some(1 << 40),
        "cls-then-off" => {
            fathomdb_sdk::embed_batch_cls(&["a module-level embed first"]).expect("cls embed");
            options.config.cuda_pool = Some(CudaPoolMode::Off);
            let dir = tempfile::tempdir().expect("tempdir");
            let error = Engine::open(dir.path().join("conflict.sqlite"), options)
                .err()
                .expect("an open with a different mode after the first GPU use conflicts");
            assert_eq!(error.kind(), ErrorKind::InvalidArgument, "{error}");
            println!("sdk-child: conflict {error}");
            std::process::exit(0);
        }
        _ => {}
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let engine = Engine::open(dir.path().join("slice31.sqlite"), options).expect("open");
    let resolution =
        engine.open_report().embedder_device_resolution.clone().expect("device resolution");
    let EffectiveEmbedDevice::Cuda(info) = &resolution.effective_device else {
        panic!("expected a CUDA embedder, got {resolution:?}");
    };
    let report = info.cuda_allocator.clone().expect("allocator report");
    println!(
        "sdk-child: path={} reason={} module_load_init={} pool_mode={} pool_mode_source={}",
        report.path.map_or("unknown", CudaAllocatorPath::as_str),
        report.reason.as_str(),
        report.module_load_init.as_str(),
        report.pool_mode.map_or("null", CudaPoolMode::as_str),
        report.pool_mode_source.map_or("null", |source| source.as_str()),
    );
    println!(
        "sdk-child: pool_max_size_bytes={} pool_max_size_source={} release_threshold={} \
         pool_release_threshold_source={}",
        report.pool_max_size_bytes.map_or("null".to_owned(), |bytes| bytes.to_string()),
        report.pool_max_size_source.map_or("null", |source| source.as_str()),
        report.release_threshold.map_or("null", ReleaseThreshold::as_str),
        report.pool_release_threshold_source.map_or("null", |source| source.as_str()),
    );
    engine
        .write(&[PreparedWrite::Node {
            kind: "doc".to_owned(),
            body: "the SDK reaches the private pool".to_owned(),
            source_id: SourceId::new("test:slice31-sdk").expect("source id"),
            logical_id: None,
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .expect("write");
    engine.drain(60_000).expect("drain");
    let hits = engine.search("private pool", SearchOptions::default()).expect("search");
    println!("sdk-child: searched hits={}", hits.results.len());
    engine.close().expect("close");
    println!("sdk-child: closed");
    std::process::exit(0);
}

fn run_child(test_name: &str, mode: &str, env: &[(&str, &str)]) -> (Output, String) {
    let exe = std::env::current_exe().expect("test binary");
    let mut command = Command::new(exe);
    command
        .args(["--exact", test_name, "--ignored", "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, mode)
        .env("FATHOMDB_EMBED_DEVICE", "cuda:0")
        .env_remove("FATHOMDB_POOL_MODE")
        .env_remove("FATHOMDB_CUDA_EARLY_INIT");
    for (name, value) in env {
        command.env(name, value);
    }
    let output = command.output().expect("child process");
    let report = format!(
        "status={:?}\n--- stdout\n{}\n--- stderr\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.signal(), None, "the child must not die on a signal:\n{report}");
    assert_eq!(output.status.code(), Some(0), "the child must exit 0:\n{report}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    if mode != "cls-then-off" {
        assert!(stdout.contains("sdk-child: searched"), "{report}");
        assert!(stdout.contains("sdk-child: closed"), "{report}");
    }
    (output, report)
}

fn child_or(test_name: &str, mode: &str, env: &[(&str, &str)], expected: &str) {
    if let Ok(child_mode) = std::env::var(CHILD_ENV) {
        sdk_child(&child_mode);
        return;
    }
    let (output, report) = run_child(test_name, mode, env);
    assert!(String::from_utf8_lossy(&output.stdout).contains(expected), "{report}");
}

#[test]
#[ignore = "Orin only: the private CUDA pool needs a Jetson AGX Orin 64 GB"]
fn gpu_orin_64_sdk_process_reaches_the_private_pool_without_an_init_call() {
    child_or(
        "gpu_orin_64_sdk_process_reaches_the_private_pool_without_an_init_call",
        "default",
        &[],
        "sdk-child: path=private reason=private_pool module_load_init=not_at_load \
         pool_mode=auto pool_mode_source=default",
    );
}

#[test]
#[ignore = "Orin only: the private CUDA pool needs a Jetson AGX Orin 64 GB"]
fn gpu_orin_64_sdk_process_with_cuda_pool_off_takes_the_0827_path() {
    child_or(
        "gpu_orin_64_sdk_process_with_cuda_pool_off_takes_the_0827_path",
        "pool-off",
        &[("FATHOMDB_POOL_MODE", "on")],
        "reason=mode_off module_load_init=not_at_load pool_mode=off pool_mode_source=sdk",
    );
}

#[test]
#[ignore = "Orin only: the private CUDA pool needs a Jetson AGX Orin 64 GB"]
fn gpu_orin_64_sdk_process_ignores_cuda_early_init_off() {
    child_or(
        "gpu_orin_64_sdk_process_ignores_cuda_early_init_off",
        "default",
        &[("FATHOMDB_CUDA_EARLY_INIT", "off")],
        "sdk-child: path=private reason=private_pool module_load_init=not_at_load",
    );
}

// ---- § 15.8 and § 15.9 Orin parity rows (the Rust SDK column) -------------

#[test]
#[ignore = "Orin only: the private CUDA pool needs a Jetson AGX Orin 64 GB"]
fn gpu_orin_64_sdk_cuda_pool_on_beats_an_env_off() {
    child_or(
        "gpu_orin_64_sdk_cuda_pool_on_beats_an_env_off",
        "pool-on",
        &[("FATHOMDB_POOL_MODE", "off")],
        "sdk-child: path=private reason=private_pool module_load_init=not_at_load \
         pool_mode=on pool_mode_source=sdk",
    );
}

#[test]
#[ignore = "Orin only: the private CUDA pool needs a Jetson AGX Orin 64 GB"]
fn gpu_orin_64_sdk_release_threshold_max_from_the_sdk() {
    child_or(
        "gpu_orin_64_sdk_release_threshold_max_from_the_sdk",
        "max",
        &[("FATHOMDB_POOL_RELEASE_THRESHOLD", "0")],
        "release_threshold=max pool_release_threshold_source=sdk",
    );
}

#[test]
#[ignore = "Orin only: the private CUDA pool needs a Jetson AGX Orin 64 GB"]
fn gpu_orin_64_sdk_pool_size_from_the_sdk_beats_the_env() {
    child_or(
        "gpu_orin_64_sdk_pool_size_from_the_sdk_beats_the_env",
        "size-sdk",
        &[("FATHOMDB_POOL_MAXSIZE", "2G")],
        "sdk-child: pool_max_size_bytes=1073741824 pool_max_size_source=sdk",
    );
}

#[test]
#[ignore = "Orin only: the private CUDA pool needs a Jetson AGX Orin 64 GB"]
fn gpu_orin_64_sdk_pool_size_above_device_memory_is_invalid_setting() {
    child_or(
        "gpu_orin_64_sdk_pool_size_above_device_memory_is_invalid_setting",
        "size-huge",
        &[],
        "reason=invalid_setting",
    );
}

#[test]
#[ignore = "Orin only: the private CUDA pool needs a Jetson AGX Orin 64 GB"]
fn gpu_orin_64_sdk_module_level_embed_then_an_open_with_off_conflicts() {
    child_or(
        "gpu_orin_64_sdk_module_level_embed_then_an_open_with_off_conflicts",
        "cls-then-off",
        &[],
        "sdk-child: conflict",
    );
}
