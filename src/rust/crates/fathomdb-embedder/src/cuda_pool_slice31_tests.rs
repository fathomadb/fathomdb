//! 0.8.28 Slice 31 Orin tests of the private CUDA pool (design §§ 4, 5, 6
//! and 8): the close release (N1), process exit with the pool alive (D-15),
//! the usable capacity of the 3 GiB pool (D-12) and the memory outside the
//! pool (D-14). Each test is `#[ignore]` and runs as its own process on the
//! Jetson AGX Orin 64 GB, because the allocator decision is process-wide
//! (test-plan § 3.13, cmd `c-pool-s31`):
//!
//! ```text
//! flock /tmp/fathomdb-gpu.lock env FATHOMDB_REQUIRE_LIVE=1 FATHOMDB_EMBED_DEVICE=cuda:0 \
//!   cargo test --locked -p fathomdb-embedder --features embed-cuda,tegra-pool --lib -- \
//!   --ignored --exact "cuda_pool_policy::slice31_tests::<t>" --nocapture --test-threads=1
//! ```
#![cfg(all(
    feature = "tegra-pool",
    feature = "embed-cuda",
    target_os = "linux",
    target_arch = "aarch64"
))]

use std::os::unix::process::ExitStatusExt;
use std::process::Output;

use fathomdb_embedder_api::Embedder;

use super::*;

const MIB: u64 = 1 << 20;
const GIB: u64 = 1 << 30;
const CHILD_ENV: &str = "FATHOMDB_TEST_SLICE31_CHILD";
const CHILD_TEST: &str = "cuda_pool_policy::slice31_tests::gpu_orin_64_slice31_child";

fn passages() -> Vec<String> {
    (0..64)
        .map(|i| {
            format!(
                "Passage {i}: the Jetson AGX Orin shares one LPDDR5 memory between its CPU and \
                 GPU, so a CUDA memory pool that never returns its reservation competes with \
                 every other process on the module. Record {i} of the batch repeats this \
                 sentence with its own number so that the tokenized lengths stay realistic."
            )
        })
        .collect()
}

fn early_init() {
    let init = crate::cuda_driver_init::run_module_load_early_init(true, true);
    assert_eq!(init, ModuleLoadInit::Ran);
}

/// Builds the default embedder on `cuda:0` and embeds four 64-passage
/// batches, so the pool holds model weights and activations.
fn loaded_embedder() -> crate::CandleBgeEmbedder {
    let embedder = crate::CandleBgeEmbedder::new().expect("embedder on cuda:0");
    let passages = passages();
    let inputs: Vec<&str> = passages.iter().map(String::as_str).collect();
    for _ in 0..4 {
        let vectors = embedder.embed_batch(&inputs).expect("embed batch");
        assert_eq!(vectors.len(), inputs.len());
    }
    embedder
}

fn spawn_child(mode: &str, extra_env: &[(&str, &str)]) -> (Output, String) {
    let exe = std::env::current_exe().expect("test binary");
    let mut command = std::process::Command::new(exe);
    command
        .args([CHILD_TEST, "--exact", "--ignored", "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, mode)
        .env("FATHOMDB_EMBED_DEVICE", "cuda:0");
    for (name, value) in extra_env {
        command.env(name, value);
    }
    let output = command.output().expect("child process");
    let report = format!(
        "status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output, report)
}

fn assert_clean_exit(output: &Output, report: &str) {
    assert_eq!(output.status.signal(), None, "the child must not die on a signal\n{report}");
    assert_eq!(output.status.code(), Some(0), "the child must exit 0\n{report}");
}

/// The drop-then-read body shared by R14's two tests: after the embedder is
/// dropped, the private pool's counters are read with no synchronization.
fn drop_and_read_unsynchronized(label: &str) -> PoolCounters {
    let embedder = loaded_embedder();
    let loaded =
        private_pool_counters_unsynchronized().expect("pool counters").expect("private decision");
    println!("{label} loaded: {loaded:?}");
    assert!(loaded.reserved_high > 0, "{loaded:?}");
    drop(embedder);
    let closed =
        private_pool_counters_unsynchronized().expect("pool counters").expect("private decision");
    println!("{label} dropped (no synchronize): {closed:?}");
    closed
}

// ---- R14: N1, the close release ------------------------------------------

/// N1 (design § 8): dropping a FathomDB-built private-pool embedder returns
/// the pool's unused memory without the caller synchronizing.
#[test]
#[ignore = "needs the Jetson AGX Orin 64 GB; run under flock /tmp/fathomdb-gpu.lock"]
fn gpu_orin_64_dropping_the_embedder_releases_pool_memory_without_a_caller_synchronize() {
    early_init();
    let closed = drop_and_read_unsynchronized("n1");
    assert_eq!(closed.used_current, 0, "{closed:?}");
    assert_eq!(closed.reserved_current, 0, "{closed:?}");
}

/// N1 with `FATHOMDB_POOL_RELEASE_THRESHOLD=max`: a synchronization alone
/// never releases, so only the close release's trim can.
#[test]
#[ignore = "needs the Jetson AGX Orin 64 GB; run under flock /tmp/fathomdb-gpu.lock"]
fn gpu_orin_64_close_release_also_trims_a_max_threshold_pool() {
    let (output, report) = spawn_child("max-threshold", &[(ENV_POOL_RELEASE_THRESHOLD, "max")]);
    assert_clean_exit(&output, &report);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("slice31-child: max-threshold reserved_current=0"), "{report}");
}

/// N1 under `max` with two models (design § 14.1, owner ruling Q34): the
/// pool keeps its reservation while any model is open and returns it when
/// the last one closes; the surviving model's output is unchanged.
#[test]
#[ignore = "needs the Jetson AGX Orin 64 GB; run under flock /tmp/fathomdb-gpu.lock"]
fn gpu_orin_64_max_pool_stays_reserved_until_the_last_model_closes() {
    let (output, report) = spawn_child("two-models", &[(ENV_POOL_RELEASE_THRESHOLD, "max")]);
    assert_clean_exit(&output, &report);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("slice31-child: two-models first-close kept"), "{report}");
    assert!(stdout.contains("slice31-child: two-models identical"), "{report}");
    assert!(stdout.contains("slice31-child: two-models last-close reserved_current=0"), "{report}");
}

// ---- R15: D-15, process exit with the pool alive (characterization) -------

/// D-15 item 2: a child builds the embedder, embeds and calls
/// `std::process::exit(0)` without dropping it.
#[test]
#[ignore = "needs the Jetson AGX Orin 64 GB; run under flock /tmp/fathomdb-gpu.lock"]
fn gpu_orin_64_process_exit_with_the_private_pool_alive_is_clean() {
    let (output, report) = spawn_child("exit-alive", &[]);
    assert_clean_exit(&output, &report);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("slice31-child: exit-alive embedded"), "{report}");
}

/// D-15: a child drops the embedder, sees the pool empty, and returns.
#[test]
#[ignore = "needs the Jetson AGX Orin 64 GB; run under flock /tmp/fathomdb-gpu.lock"]
fn gpu_orin_64_dropping_the_embedder_then_exiting_is_clean() {
    let (output, report) = spawn_child("drop-exit", &[]);
    assert_clean_exit(&output, &report);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("slice31-child: drop-exit reserved_current=0"), "{report}");
}

#[test]
#[ignore = "run by the Slice 31 child-process tests"]
fn gpu_orin_64_slice31_child() {
    let Ok(mode) = std::env::var(CHILD_ENV) else {
        return;
    };
    early_init();
    match mode.as_str() {
        "max-threshold" => {
            let device = new_cuda_device(0).expect("private device");
            let report = allocator_report(&device).expect("report");
            assert_eq!(report.release_threshold, Some(ReleaseThreshold::Max));
            drop(device);
            let closed = drop_and_read_unsynchronized("max-threshold");
            assert_eq!(closed.used_current, 0, "{closed:?}");
            println!("slice31-child: max-threshold reserved_current={}", closed.reserved_current);
        }
        "exit-alive" => {
            let embedder = loaded_embedder();
            embedder.embed("one more before exit").expect("embed");
            println!("slice31-child: exit-alive embedded");
            std::process::exit(0);
        }
        "drop-exit" => {
            let embedder = loaded_embedder();
            drop(embedder);
            let (closed, _) =
                private_pool_counters().expect("pool counters").expect("private decision");
            println!("slice31-child: drop-exit counters {closed:?}");
            assert_eq!(closed.used_current, 0, "{closed:?}");
            println!("slice31-child: drop-exit reserved_current={}", closed.reserved_current);
        }
        "two-models" => {
            let first = loaded_embedder();
            let second = loaded_embedder();
            let before = second.embed("the surviving model").expect("embed before");
            drop(first);
            let kept = private_pool_counters_unsynchronized()
                .expect("pool counters")
                .expect("private decision");
            println!("slice31-child: two-models after the first close {kept:?}");
            assert!(kept.reserved_current > 0, "{kept:?}");
            println!("slice31-child: two-models first-close kept {}", kept.reserved_current);
            let after = second.embed("the surviving model").expect("embed after");
            assert_eq!(before, after, "the surviving model's output must not change");
            println!("slice31-child: two-models identical");
            drop(second);
            let closed = private_pool_counters_unsynchronized()
                .expect("pool counters")
                .expect("private decision");
            println!("slice31-child: two-models after the last close {closed:?}");
            assert_eq!(closed.used_current, 0, "{closed:?}");
            println!(
                "slice31-child: two-models last-close reserved_current={}",
                closed.reserved_current
            );
        }
        other => panic!("unknown child mode {other}"),
    }
}

// ---- R17: D-12 and D-14 measurements --------------------------------------

/// D-12 (ORIN-13): allocate 32 MiB tensors on the private pool until the
/// pool is exhausted and print the usable bytes. A measurement; the typed
/// exhaustion is the only assertion.
#[test]
#[ignore = "needs the Jetson AGX Orin 64 GB; run under flock /tmp/fathomdb-gpu.lock"]
fn gpu_orin_64_records_the_usable_capacity_of_the_three_gib_pool() {
    early_init();
    let device = new_cuda_device(0).expect("private device");
    let report = allocator_report(&device).expect("report");
    assert_eq!(report.path, Some(CudaAllocatorPath::Private));
    let max_size = report.pool_max_size_bytes.expect("pool max size");
    assert_eq!(max_size, 3 * GIB);
    let chunk_elems = usize::try_from(32 * MIB / 4).expect("chunk");
    let mut held = Vec::new();
    let failure = loop {
        match candle_core::Tensor::zeros(chunk_elems, candle_core::DType::F32, &device) {
            Ok(tensor) => held.push(tensor),
            Err(error) => {
                break forward_failure(&error, &device, "capacity").unwrap_or_else(|| {
                    panic!("untyped failure after {} chunks: {error}", held.len())
                });
            }
        }
        assert!((held.len() as u64) * 32 * MIB <= max_size, "allocated past the cap");
    };
    let chunks = held.len() as u64;
    println!(
        "fathomdb-capacity {{\"max_size\":{max_size},\"usable_bytes\":{},\"chunks\":{chunks}}}",
        chunks * 32 * MIB
    );
    assert_eq!(failure.kind(), "cuda_pool_exhausted", "{failure}");
}

fn status_kib(status: &str, key: &str) -> u64 {
    status
        .lines()
        .find_map(|line| line.strip_prefix(key))
        .and_then(|rest| rest.trim().trim_end_matches("kB").trim().parse().ok())
        .unwrap_or(0)
}

fn d14_line(stage: &str, cuda: bool) {
    let status = std::fs::read_to_string("/proc/self/status").expect("/proc/self/status");
    let pool = if cuda {
        private_pool_counters().expect("pool counters").map(|(counters, _)| counters)
    } else {
        None
    };
    let pool_json = pool.map_or_else(
        || "null".to_owned(),
        |c| {
            format!(
                "{{\"reserved_current\":{},\"used_current\":{},\"reserved_high\":{}}}",
                c.reserved_current, c.used_current, c.reserved_high
            )
        },
    );
    let default_json = if cuda && stage != "S0" {
        let (reserved, used_high) = default_pool_counters(0).expect("default pool counters");
        assert_eq!((reserved, used_high), (0, 0), "{stage}: the default pool must stay empty");
        format!("{{\"reserved_current\":{reserved},\"used_high\":{used_high}}}")
    } else {
        "null".to_owned()
    };
    println!(
        "fathomdb-d14 {{\"stage\":\"{stage}\",\"device\":\"{}\",\"vm_rss\":{},\"rss_anon\":{},\"rss_file\":{},\"rss_shmem\":{},\"pool\":{pool_json},\"default_pool\":{default_json}}}",
        if cuda { "cuda" } else { "cpu" },
        status_kib(&status, "VmRSS:") * 1024,
        status_kib(&status, "RssAnon:") * 1024,
        status_kib(&status, "RssFile:") * 1024,
        status_kib(&status, "RssShmem:") * 1024,
    );
}

/// D-14 (ORIN-14): `VmRSS` and its split beside the private pool's counters
/// at stages S0-S5; the default pool stays empty at every stage (asserted).
/// With `FATHOMDB_EMBED_DEVICE=cpu` the same stages run on the CPU embedder
/// (S1 skipped) as the host-only control.
#[test]
#[ignore = "needs the Jetson AGX Orin 64 GB; run under flock /tmp/fathomdb-gpu.lock"]
fn gpu_orin_64_memory_outside_the_private_pool_is_recorded() {
    let cuda = std::env::var("FATHOMDB_EMBED_DEVICE").as_deref() != Ok("cpu");
    let init = crate::cuda_driver_init::run_module_load_early_init(true, true);
    if cuda {
        assert_eq!(init, ModuleLoadInit::Ran);
    }
    d14_line("S0", cuda);
    let device = if cuda {
        let device = new_cuda_device(0).expect("private device");
        let report = allocator_report(&device).expect("report");
        assert_eq!(report.path, Some(CudaAllocatorPath::Private));
        d14_line("S1", cuda);
        Some(device)
    } else {
        None
    };
    let embedder = crate::CandleBgeEmbedder::new().expect("embedder");
    d14_line("S2", cuda);
    for i in 0..5 {
        embedder.embed(&format!("warm-up {i} on the Orin")).expect("embed");
    }
    d14_line("S3", cuda);
    for i in 0..50 {
        embedder.embed(&format!("timed embed {i} on the Orin")).expect("embed");
    }
    d14_line("S4", cuda);
    drop(embedder);
    d14_line("S5", cuda);
    drop(device);
}
