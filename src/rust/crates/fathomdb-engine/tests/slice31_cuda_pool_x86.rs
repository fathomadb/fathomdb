//! 0.8.28 Slice 31 § 15.8, the x86 row: on an x86_64 CUDA build (the
//! RTX 3090), `cuda_pool = on` opens and embeds on CUDA, the setting reads
//! back, no allocator report exists (the pool code is aarch64-only), and the
//! one-configuration-per-process rule holds as on the Orin.
//!
//! Live GPU test: it runs only with `FATHOMDB_REQUIRE_LIVE=1` and skips
//! otherwise. The configuration in force is process-wide, so the body runs
//! in a child process of this test binary. Select the GPU by UUID:
//!
//! ```text
//! flock /tmp/fathomdb-gpu.lock env FATHOMDB_REQUIRE_LIVE=1 CUDA_DEVICE_ORDER=PCI_BUS_ID \
//!   CUDA_VISIBLE_DEVICES=GPU-<uuid> cargo test --locked -p fathomdb-engine --features embed-cuda \
//!   --test slice31_cuda_pool_x86 -- --nocapture --test-threads=1
//! ```
#![cfg(all(target_os = "linux", target_arch = "x86_64", feature = "embed-cuda"))]

use std::process::Command;

use fathomdb_embedder::EffectiveEmbedDevice;
use fathomdb_engine::{
    CudaPoolMode, EmbedderChoice, Engine, EngineConfig, EngineConfigurationError, EngineOpenError,
};

const CHILD_ENV: &str = "FATHOMDB_TEST_SLICE31_X86_POOL_CHILD";
const TEST_NAME: &str = "x86_cuda_pool_on_opens_embeds_and_holds_one_configuration_per_process";

fn pool(mode: CudaPoolMode) -> EngineConfig {
    EngineConfig { cuda_pool: Some(mode), ..EngineConfig::default() }
}

fn assert_embeds(engine: &Engine, label: &str) {
    let vector = engine.embed_text("the private pool is aarch64-only").expect(label);
    assert!(!vector.is_empty(), "{label}: empty vector");
    assert!(vector.iter().all(|value| value.is_finite()), "{label}: non-finite value");
    println!("x86-pool-child: {label} dim={}", vector.len());
}

fn child() {
    let dir = tempfile::tempdir().expect("tempdir");
    let opened = Engine::open_with_choice_and_config(
        dir.path().join("on.sqlite"),
        EmbedderChoice::Default,
        pool(CudaPoolMode::On),
    )
    .expect("open with cuda_pool=on and the default embedder");
    let resolution = opened.report.embedder_device_resolution.clone().expect("device resolution");
    let EffectiveEmbedDevice::Cuda(info) = &resolution.effective_device else {
        panic!("expected the default embedder on CUDA, got {resolution:?}");
    };
    assert_eq!(info.cuda_allocator, None, "x86 builds have no allocator report");
    println!("x86-pool-child: cuda:{} {:?} cuda_allocator=None", info.ordinal, info.name);
    let engine = opened.engine;
    assert_eq!(engine.config().cuda_pool, Some(CudaPoolMode::On));
    assert_embeds(&engine, "first embed");

    let error = match Engine::open_with_choice_and_config(
        dir.path().join("off.sqlite"),
        EmbedderChoice::None,
        pool(CudaPoolMode::Off),
    ) {
        Ok(_) => panic!("a second open with cuda_pool=off must conflict"),
        Err(error) => error,
    };
    let EngineOpenError::EngineConfiguration(EngineConfigurationError::CudaPoolSettingConflict(
        conflict,
    )) = error
    else {
        panic!("expected CudaPoolSettingConflict, got {error:?}");
    };
    assert_eq!(conflict.setting, "cuda_pool");
    println!("x86-pool-child: conflict {conflict}");
    assert_embeds(&engine, "embed after the conflict");
    engine.close().expect("close");
    println!("x86-pool-child: closed");
}

#[test]
fn x86_cuda_pool_on_opens_embeds_and_holds_one_configuration_per_process() {
    if std::env::var_os(CHILD_ENV).is_some() {
        child();
        return;
    }
    if std::env::var_os("FATHOMDB_REQUIRE_LIVE").is_none_or(|value| value != "1") {
        eprintln!(
            "skipped: needs a CUDA GPU and the default embedder; set FATHOMDB_REQUIRE_LIVE=1"
        );
        return;
    }
    let exe = std::env::current_exe().expect("test binary");
    let output = Command::new(exe)
        .args(["--exact", TEST_NAME, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, "1")
        .env("FATHOMDB_EMBED_DEVICE", "cuda:0")
        .env_remove("FATHOMDB_POOL_MODE")
        .env_remove("FATHOMDB_POOL_MAXSIZE")
        .env_remove("FATHOMDB_POOL_RELEASE_THRESHOLD")
        .output()
        .expect("child process");
    let report = format!(
        "status={:?}\n--- stdout\n{}\n--- stderr\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "{report}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("x86-pool-child: conflict cuda_pool=off conflicts with cuda_pool=on"),
        "{report}"
    );
    assert!(stdout.contains("x86-pool-child: closed"), "{report}");
    println!("{stdout}");
}
