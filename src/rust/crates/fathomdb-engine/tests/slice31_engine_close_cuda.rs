//! 0.8.28 Slice 31 (TDN-03, R16; design § 6, R-DES ruling 7): an engine
//! that embeds on the Tegra private CUDA memory pool closes and its process
//! exits with status 0 and no signal. Engine-level close evidence that does
//! not depend on the Node runtime; no pool-counter clause (R-TEST OP-4).
//!
//! Orin only: the private pool exists only on the Jetson AGX Orin 64 GB with
//! a `tegra-pool` build, so the test is `#[ignore]` everywhere and runs in
//! the Orin windows (test-plan § 4.2, cmd `c-engine-close`):
//!
//! ```text
//! flock /tmp/fathomdb-gpu.lock env FATHOMDB_REQUIRE_LIVE=1 cargo test --locked -p fathomdb-engine \
//!   --features embed-cuda,tegra-pool --test slice31_engine_close_cuda -- --ignored \
//!   --exact gpu_orin_64_engine_close_with_the_private_pool_then_exit_is_clean --nocapture --test-threads=1
//! ```
#![cfg(all(target_os = "linux", feature = "embed-cuda"))]

use std::os::unix::process::ExitStatusExt;
use std::process::Command;

use fathomdb_embedder::{CudaAllocatorPath, EffectiveEmbedDevice};
use fathomdb_engine::{EmbedderChoice, Engine, InitialState, PreparedWrite, SourceId};

const CHILD_ENV: &str = "FATHOMDB_TEST_SLICE31_ENGINE_CLOSE_CHILD";
const TEST_NAME: &str = "gpu_orin_64_engine_close_with_the_private_pool_then_exit_is_clean";

/// The child: open with the default embedder on `cuda:0`, write, search,
/// close the engine, then exit the process directly.
fn engine_close_child() {
    let dir = tempfile::tempdir().expect("tempdir");
    let opened =
        Engine::open_with_choice(dir.path().join("slice31.sqlite"), EmbedderChoice::Default)
            .expect("open with the default embedder");
    let resolution = opened.report.embedder_device_resolution.clone().expect("device resolution");
    let path = match &resolution.effective_device {
        EffectiveEmbedDevice::Cuda(info) => info
            .cuda_allocator
            .as_ref()
            .and_then(|report| report.path)
            .map_or("unknown", CudaAllocatorPath::as_str),
        EffectiveEmbedDevice::Cpu => "cpu",
    };
    println!("engine-close-child: path={path}");
    let engine = opened.engine;
    let node = PreparedWrite::Node {
        kind: "doc".to_owned(),
        body: "the private pool closes cleanly".to_owned(),
        source_id: SourceId::new("test:slice31").expect("source id"),
        logical_id: None,
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    };
    engine.write(&[node]).expect("write");
    engine.drain(60_000).expect("drain");
    let hits = engine.search("private pool").expect("search");
    println!("engine-close-child: searched hits={}", hits.results.len());
    engine.close().expect("close");
    println!("engine-close-child: closed");
    std::process::exit(0);
}

#[test]
#[ignore = "Orin only: the private CUDA pool needs a Jetson AGX Orin 64 GB and a tegra-pool build"]
fn gpu_orin_64_engine_close_with_the_private_pool_then_exit_is_clean() {
    if std::env::var_os(CHILD_ENV).is_some() {
        engine_close_child();
        return;
    }
    let exe = std::env::current_exe().expect("test binary");
    let output = Command::new(exe)
        .args(["--exact", TEST_NAME, "--ignored", "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, "1")
        .env("FATHOMDB_EMBED_DEVICE", "cuda:0")
        .output()
        .expect("child process");
    let report = format!(
        "status={:?}\n--- stdout\n{}\n--- stderr\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.signal(), None, "the child must not die on a signal:\n{report}");
    assert_eq!(output.status.code(), Some(0), "the child must exit 0:\n{report}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("engine-close-child: path=private"), "{report}");
    assert!(stdout.contains("engine-close-child: searched"), "{report}");
    assert!(stdout.contains("engine-close-child: closed"), "{report}");
}
