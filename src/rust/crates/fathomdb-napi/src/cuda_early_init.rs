//! CUDA driver initialisation when the Node addon is registered (aarch64
//! Linux CUDA builds only).
//!
//! On Jetson (measured on the AGX Orin 64 GB, L4T R36, CUDA 12.6) `cuInit`
//! reserves about 61 GiB of address space in one range inside
//! [8 GiB, 128 GiB) and returns `CUDA_ERROR_OUT_OF_MEMORY` when no hole of at
//! least 4 GiB is left there. V8 scatters heap pages through that window as
//! the JavaScript heap grows, so a process that waits for `Engine.open` to
//! initialise CUDA can find it impossible. Initialising when the addon is
//! loaded secures `cuInit` and the driver's initial reservation before an
//! application that imports fathomdb first builds its data. It does not
//! secure everything: the default memory pool can still need a new range of
//! address space later, and in most measured Node runs it was unavailable,
//! which is why the synchronous allocator fallback is usually taken.
//!
//! The hook runs from module registration (`#[module_exports]`, called by
//! napi-rs 2 inside `napi_register_module_v1`). Node calls that synchronously
//! on the loading thread right after `dlopen` returns, so it runs before any
//! code that uses the module but outside the dynamic loader's lock; libcuda's
//! helper threads can therefore load libraries while `cuInit` waits for them.
//! A shared-library constructor would run inside glibc's `_dl_init`, under
//! that lock, and also at the start of every test binary linking this crate.
//! Registration happens once per Node env (the main thread and every
//! `worker_threads` worker that loads the addon); a `Once` keeps the call to
//! the first. It never throws, prints or fails the load: the outcome is only
//! recorded, and a later forced-CUDA refusal reads it to name an
//! out-of-memory `cuInit` as the cause.
//!
//! Off aarch64 Linux, or without `embed-cuda`/`rerank-cuda`, none of this is
//! compiled except the identity [`device_policy_refusal_message`].

#[cfg(any(
    test,
    all(
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    )
))]
use fathomdb_embedder::{EmbedDevicePolicy, RerankerDevicePolicy};

#[cfg(any(
    test,
    all(
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    )
))]
const CUDA_ERROR_OUT_OF_MEMORY: u32 = 2;

/// The environment variable that turns the registration-time `cuInit` off
/// while keeping CUDA: open then initialises the driver as before.
#[cfg(all(
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
const ENV_CUDA_EARLY_INIT: &str = "FATHOMDB_CUDA_EARLY_INIT";

/// Whether loading the addon should initialise the CUDA driver.
///
/// Only an exact `cpu` policy rules CUDA out, using the same parsers as open:
/// unset means `auto`, and a malformed value is refused at open rather than
/// here. A component built without CUDA cannot use it whatever its policy.
#[cfg(any(
    test,
    all(
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    )
))]
fn early_cuda_init_wanted(
    embed_cuda_compiled: bool,
    rerank_cuda_compiled: bool,
    embed_policy: Option<&str>,
    rerank_policy: Option<&str>,
) -> bool {
    let embed_cpu = embed_policy
        .is_some_and(|raw| matches!(raw.parse::<EmbedDevicePolicy>(), Ok(EmbedDevicePolicy::Cpu)));
    let rerank_cpu = rerank_policy.is_some_and(|raw| {
        matches!(raw.parse::<RerankerDevicePolicy>(), Ok(RerankerDevicePolicy::Cpu))
    });
    (embed_cuda_compiled && !embed_cpu) || (rerank_cuda_compiled && !rerank_cpu)
}

/// Whether `FATHOMDB_CUDA_EARLY_INIT` opts out. Only the exact value `off`
/// does; registration cannot report a malformed value, so anything else
/// keeps the default.
#[cfg(any(
    test,
    all(
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    )
))]
fn early_cuda_init_opted_out(raw: Option<&str>) -> bool {
    raw == Some("off")
}

/// Runs `init` on the first call for `once` only.
#[cfg(any(
    test,
    all(
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    )
))]
fn register_once(once: &std::sync::Once, init: impl FnOnce()) {
    once.call_once(init);
}

/// The cause and remedy to add to a `cuda_probe_failed` refusal when the
/// refusing probe's `cuInit` (`last_cu_init`, a raw `CUresult`) ran out of
/// memory.
#[cfg(any(
    test,
    all(
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    )
))]
fn fragmented_address_space_hint(kind: &str, last_cu_init: Option<u32>) -> Option<&'static str> {
    (kind == "cuda_probe_failed" && last_cu_init == Some(CUDA_ERROR_OUT_OF_MEMORY)).then_some(
        "cuInit returned CUDA_ERROR_OUT_OF_MEMORY: the CUDA driver could not reserve \
         its range of the process address space, which is usually fragmented by a \
         large JavaScript heap; import fathomdb before building large in-memory \
         data (at the top of the entry module), or start Node with \
         --import fathomdb",
    )
}

#[cfg(any(
    test,
    all(
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    )
))]
fn refusal_message(kind: &str, message: String, last_cu_init: Option<u32>) -> String {
    match fragmented_address_space_hint(kind, last_cu_init) {
        Some(hint) => format!("{message}; {hint}"),
        None => message,
    }
}

/// The component whose device policy refused.
#[derive(Clone, Copy, Debug)]
pub(crate) enum RefusingComponent {
    Embedder,
    Reranker,
}

/// The message for a device-policy refusal of `kind` by `component`. On
/// aarch64 Linux CUDA builds it names an out-of-memory `cuInit` made by that
/// component's probe as the cause, read from the most recent outcome of that
/// probe kind (another caller's `cuInit` cannot change it, a later probe of
/// the same kind can); elsewhere it is `message` unchanged.
pub(crate) fn device_policy_refusal_message(
    component: RefusingComponent,
    kind: &str,
    message: String,
) -> String {
    #[cfg(all(
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    ))]
    {
        use fathomdb_embedder::CudaInitCaller;
        let caller = match component {
            RefusingComponent::Embedder => CudaInitCaller::EmbedderProbe,
            RefusingComponent::Reranker => CudaInitCaller::RerankerProbe,
        };
        let last_cu_init =
            fathomdb_embedder::cuda_driver_init_seen_by(caller).and_then(|init| init.cu_result());
        refusal_message(kind, message, last_cu_init)
    }
    #[cfg(not(all(
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    )))]
    {
        let _ = (component, kind);
        message
    }
}

#[cfg(all(
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
static EARLY_INIT: std::sync::Once = std::sync::Once::new();

#[cfg(all(
    target_os = "linux",
    target_arch = "aarch64",
    any(feature = "embed-cuda", feature = "rerank-cuda")
))]
#[napi_derive::module_exports]
fn initialize_cuda_driver_at_registration(_exports: napi::JsObject) -> napi::Result<()> {
    register_once(&EARLY_INIT, || {
        if early_cuda_init_opted_out(std::env::var(ENV_CUDA_EARLY_INIT).ok().as_deref()) {
            return;
        }
        let embed_policy = std::env::var("FATHOMDB_EMBED_DEVICE").ok();
        let rerank_policy = std::env::var(fathomdb_embedder::ENV_RERANK_DEVICE).ok();
        if early_cuda_init_wanted(
            cfg!(feature = "embed-cuda"),
            cfg!(feature = "rerank-cuda"),
            embed_policy.as_deref(),
            rerank_policy.as_deref(),
        ) {
            // A panic must not fail module registration; the embedder records
            // the outcome either way.
            let init = std::panic::catch_unwind(fathomdb_embedder::initialize_cuda_driver);
            // 0.8.28 pool study only: the A-load variants install their pool
            // here, after a successful cuInit.
            #[cfg(feature = "tegra-pool-experiment")]
            if matches!(init, Ok(fathomdb_embedder::CudaDriverInit::Initialized)) {
                let _ = std::panic::catch_unwind(fathomdb_embedder::install_cuda_pool_at_load);
            }
            let _ = init;
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const OUT_OF_MEMORY: u32 = 2;
    const NO_DEVICE: u32 = 100;

    #[test]
    fn early_init_runs_unless_every_cuda_capable_policy_is_cpu() {
        // (FATHOMDB_EMBED_DEVICE, FATHOMDB_RERANK_DEVICE, expected) with both
        // components built with CUDA. Unset means `auto`, as at open. A value
        // outside the exact grammar is refused at open; load still initialises
        // because only an exact `cpu` rules CUDA out.
        let cases = [
            (None, None, true),
            (Some("auto"), Some("auto"), true),
            (Some("cpu"), None, true),
            (None, Some("cpu"), true),
            (Some("cpu"), Some("auto"), true),
            (Some("cuda:0"), Some("cpu"), true),
            (Some("cpu"), Some("cuda:1"), true),
            (Some("cpu"), Some("cpu"), false),
            (Some("CPU"), Some("cpu"), true),
            (Some(" cpu"), Some("cpu"), true),
            (Some("cuda"), Some("cpu"), true),
            (Some(""), Some("cpu"), true),
        ];
        for (embed, rerank, expected) in cases {
            assert_eq!(
                early_cuda_init_wanted(true, true, embed, rerank),
                expected,
                "FATHOMDB_EMBED_DEVICE={embed:?} FATHOMDB_RERANK_DEVICE={rerank:?}"
            );
        }
    }

    #[test]
    fn a_component_built_without_cuda_never_asks_for_early_init() {
        // embed-cuda only: the reranker cannot use CUDA whatever its policy.
        assert!(!early_cuda_init_wanted(true, false, Some("cpu"), Some("cuda:0")));
        assert!(!early_cuda_init_wanted(true, false, Some("cpu"), None));
        assert!(early_cuda_init_wanted(true, false, None, Some("cpu")));
        // rerank-cuda only: the embedder cannot use CUDA whatever its policy.
        assert!(!early_cuda_init_wanted(false, true, Some("cuda:0"), Some("cpu")));
        assert!(early_cuda_init_wanted(false, true, Some("cpu"), None));
        // No CUDA at all.
        assert!(!early_cuda_init_wanted(false, false, None, None));
    }

    #[test]
    fn the_address_space_hint_needs_a_probe_failure_and_an_out_of_memory_cu_init() {
        assert!(fragmented_address_space_hint("cuda_probe_failed", Some(OUT_OF_MEMORY)).is_some());
        // cuInit was never attempted, succeeded, or failed for another reason.
        for last_cu_init in [None, Some(0), Some(NO_DEVICE), Some(1)] {
            assert_eq!(
                fragmented_address_space_hint("cuda_probe_failed", last_cu_init),
                None,
                "{last_cu_init:?}"
            );
        }
        // Every other refusal keeps its message whatever cuInit returned.
        for kind in [
            "no_visible_cuda_device",
            "cuda_incompatible",
            "cuda_not_compiled",
            "invalid_policy",
            "arm64_sbsa_unsupported",
        ] {
            assert_eq!(fragmented_address_space_hint(kind, Some(OUT_OF_MEMORY)), None, "{kind}");
        }
    }

    #[test]
    fn the_hint_names_the_cause_and_both_remedies() {
        let hint = fragmented_address_space_hint("cuda_probe_failed", Some(OUT_OF_MEMORY))
            .expect("hint for an out-of-memory cuInit");
        for needle in [
            "cuInit",
            "CUDA_ERROR_OUT_OF_MEMORY",
            "address space",
            "JavaScript heap",
            "import fathomdb before",
            "--import fathomdb",
        ] {
            assert!(hint.contains(needle), "{needle:?} missing from {hint:?}");
        }
    }

    #[test]
    fn the_refusal_message_keeps_its_prefix_and_appends_the_hint() {
        let base = "cuda:0 requested but unavailable: CudaProbeFailed";
        let message = refusal_message("cuda_probe_failed", base.to_owned(), Some(OUT_OF_MEMORY));
        let hint = fragmented_address_space_hint("cuda_probe_failed", Some(OUT_OF_MEMORY))
            .expect("hint for an out-of-memory cuInit");
        assert_eq!(message, format!("{base}; {hint}"));
        assert_eq!(refusal_message("cuda_probe_failed", base.to_owned(), Some(0)), base);
        assert_eq!(refusal_message("cuda_probe_failed", base.to_owned(), None), base);
    }

    #[test]
    fn registration_initialises_at_most_once_across_envs() {
        // Node runs module registration once per env: the main thread and
        // every worker_threads worker that loads the addon.
        let once = std::sync::Once::new();
        let mut calls = 0;
        for _env in 0..3 {
            register_once(&once, || calls += 1);
        }
        assert_eq!(calls, 1);
    }

    #[test]
    fn only_the_exact_value_off_opts_out_of_early_init() {
        assert!(early_cuda_init_opted_out(Some("off")));
        for raw in [None, Some("on"), Some(""), Some("OFF"), Some(" off"), Some("0"), Some("false")]
        {
            assert!(!early_cuda_init_opted_out(raw), "{raw:?}");
        }
    }

    // Loading the library must not touch CUDA: the driver is initialised from
    // module registration, which Node runs after dlopen returns and which a
    // Rust test binary never runs. A shared-library constructor would already
    // have called cuInit before this test started. (Only an environment that
    // fixes every policy to cpu, or opts out, would hide such a constructor.)
    // Only the module-load slot is checked: other tests in this binary run the
    // embedder and reranker probes, which call cuInit as other callers.
    #[cfg(all(
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    ))]
    #[test]
    fn loading_the_addon_library_does_not_initialise_cuda() {
        assert_eq!(
            fathomdb_embedder::cuda_driver_init_seen_by(
                fathomdb_embedder::CudaInitCaller::ModuleLoad
            ),
            None
        );
    }
}
