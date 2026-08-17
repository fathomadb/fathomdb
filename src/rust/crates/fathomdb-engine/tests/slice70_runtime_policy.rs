//! Slice 70 runtime-policy behavior at the public `Engine::open` seam.
//!
//! The test intentionally runs on a CPU-only default-embedder build. A forced
//! CUDA request must fail with its policy outcome before the loader can fetch
//! a model, and must never be converted into an implicit CPU open.

use std::{env, sync::Mutex};

use fathomdb_embedder::{DeviceResolutionError, EmbedDevicePolicyError};
use fathomdb_engine::{EmbedderChoice, Engine, EngineOpenError};
use tempfile::tempdir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn forced_cuda_cannot_open_on_cpu_and_returns_the_typed_policy_outcome() {
    let _guard = ENV_LOCK.lock().expect("environment lock");
    let previous = env::var_os("FATHOMDB_EMBED_DEVICE");
    // SAFETY: this test serializes mutations of this process-global variable,
    // and restores the previous setting before returning.
    unsafe { env::set_var("FATHOMDB_EMBED_DEVICE", "cuda:0") };

    let directory = tempdir().expect("temporary database directory");
    let result = Engine::open_with_choice(
        directory.path().join("forced-cuda.sqlite"),
        EmbedderChoice::Default,
    );

    // SAFETY: paired with the serialized mutation above.
    unsafe {
        if let Some(value) = previous {
            env::set_var("FATHOMDB_EMBED_DEVICE", value);
        } else {
            env::remove_var("FATHOMDB_EMBED_DEVICE");
        }
    }

    assert_eq!(
        result.expect_err("forced CUDA must not silently open on CPU"),
        EngineOpenError::EmbedDevicePolicy(EmbedDevicePolicyError::Resolution(
            DeviceResolutionError::CudaNotCompiled { ordinal: 0 },
        )),
    );
}
