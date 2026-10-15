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
    CudaAllocatorPath, CudaPoolMode, EffectiveEmbedDevice, Engine, InitialState, OpenOptions,
    PreparedWrite, SearchOptions, SourceId,
};

const CHILD_ENV: &str = "FATHOMDB_TEST_SLICE31_SDK_CHILD";

/// The child: open through the SDK with the default embedder on `cuda:0`,
/// print the allocator report, write, search, close and exit.
fn sdk_child(mode: &str) {
    let mut options = OpenOptions { use_default_embedder: true, ..OpenOptions::default() };
    if mode == "pool-off" {
        options.config.cuda_pool = Some(CudaPoolMode::Off);
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
    assert!(stdout.contains("sdk-child: searched"), "{report}");
    assert!(stdout.contains("sdk-child: closed"), "{report}");
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
