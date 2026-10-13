//! 0.8.28 Slice 31 D-18 (design § 9.1, owner rulings Q24-Q26): before a CUDA
//! device is built, the embedder refuses a cuBLAS or cuRAND it cannot use
//! with a typed `cuda_library_unavailable` error instead of a panic inside
//! Candle's device construction or at the first missing symbol.
//!
//! Each test runs its CUDA work in a child process (this test binary,
//! re-executed for the same test with `FATHOMDB_TEST_D18_CHILD` set), so a
//! decoy library on `LD_LIBRARY_PATH` never reaches another test and a panic
//! is observed, not fatal. The decoys are built at test time by `cc` from
//! `tests/fixtures/decoy-cublas/`. On windchill3 every process runs with
//! `CUDA_VISIBLE_DEVICES` set to one RTX 3090 by UUID (design § 0.1).
//!
//! ```text
//! flock /tmp/fathomdb-gpu.lock cargo test --locked -p fathomdb-embedder --features embed-cuda \
//!   --test slice31_cuda_library_major -- --test-threads=1 --nocapture
//! ```
#![cfg(all(target_os = "linux", feature = "embed-cuda"))]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use candle_core::{DType, Device, Tensor};
use fathomdb_embedder::{resolve_default_embedder_device_from_env, EffectiveEmbedDevice};

#[path = "support/live.rs"]
mod live;

const CHILD_ENV: &str = "FATHOMDB_TEST_D18_CHILD";

fn in_child() -> bool {
    std::env::var_os(CHILD_ENV).is_some()
}

/// Builds `fixture` into a fresh directory as `libcublas.so.12` and
/// `libcublas.so`, so a loader that tries either name opens the decoy.
fn build_decoy(fixture: &str) -> (tempfile::TempDir, PathBuf) {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/decoy-cublas").join(fixture);
    let dir = tempfile::tempdir().expect("decoy directory");
    let versioned = dir.path().join("libcublas.so.12");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-o"])
        .arg(&versioned)
        .arg(&source)
        .status()
        .expect("cc must be installed to build the decoy cuBLAS");
    assert!(status.success(), "cc failed to build {}", source.display());
    std::fs::copy(&versioned, dir.path().join("libcublas.so")).expect("unversioned decoy name");
    let canonical = versioned.canonicalize().expect("decoy path");
    (dir, canonical)
}

/// Re-runs `test` in a child with `FATHOMDB_EMBED_DEVICE=cuda:0` and, when
/// given, `library_dir` first on `LD_LIBRARY_PATH`.
fn run_child(test: &str, library_dir: Option<&Path>) -> Output {
    let exe = std::env::current_exe().expect("test binary");
    let mut command = Command::new(exe);
    command
        .args(["--exact", test, "--include-ignored", "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, "1")
        .env("FATHOMDB_EMBED_DEVICE", "cuda:0");
    if let Some(dir) = library_dir {
        let inherited = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
        let path = if inherited.is_empty() {
            dir.display().to_string()
        } else {
            format!("{}:{inherited}", dir.display())
        };
        command.env("LD_LIBRARY_PATH", path);
    }
    command.output().expect("child process")
}

fn report(output: &Output) -> String {
    format!(
        "status={:?}\n--- stdout\n{}\n--- stderr\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The child's forced `cuda:0` resolution, printed for the parent; with
/// `run_matmul`, an F32 batched matmul on any device it returns.
fn child_resolve(run_matmul: bool) {
    let result = resolve_default_embedder_device_from_env();
    println!("d18-child: {result:?}");
    if let Err(error) = &result {
        println!("d18-child-message: {error}");
    }
    if let Ok(resolution) = &result {
        if let EffectiveEmbedDevice::Cuda(info) = &resolution.effective_device {
            println!("d18-child: device built on cuda:{}", info.ordinal);
            if run_matmul {
                let device = Device::new_cuda(info.ordinal).expect("Candle CUDA device");
                let a = Tensor::ones((2, 2, 2), DType::F32, &device).expect("tensor");
                let product = a.matmul(&a).expect("batched matmul");
                let values = product.to_vec3::<f32>().expect("read back");
                println!("d18-child: matmul ran: {values:?}");
            }
        }
    }
}

/// Without a visible CUDA device there is nothing to refuse: skip, or fail
/// under `FATHOMDB_REQUIRE_LIVE=1`.
fn gpu_visible(text: &str) -> bool {
    if text.contains("NoVisibleCudaDevice") || text.contains("no_visible_cuda_device") {
        live::require_live_or_skip("slice31_cuda_library_major: no visible CUDA device; skipping");
        return false;
    }
    true
}

#[test]
fn a_libcublas_so_12_that_reports_major_13_is_refused_before_gpu_work() {
    if in_child() {
        child_resolve(false);
        return;
    }
    let (dir, decoy) = build_decoy("decoy_cublas.c");
    let output = run_child(
        "a_libcublas_so_12_that_reports_major_13_is_refused_before_gpu_work",
        Some(dir.path()),
    );
    let text = report(&output);
    if !gpu_visible(&text) {
        return;
    }
    assert!(
        output.status.success(),
        "the child must exit 0 with a typed refusal; a panic fails:\n{text}"
    );
    for needle in [
        "LibraryUnavailable",
        "reason: WrongMajor",
        decoy.to_str().expect("utf-8 path"),
        "13.1.1",
        "required_major: 12",
    ] {
        assert!(text.contains(needle), "{needle:?} missing from the child output:\n{text}");
    }
    assert!(!text.contains("device built"), "no device may be built:\n{text}");
}

#[test]
fn a_libcublas_so_12_missing_a_required_symbol_is_refused_before_gpu_work() {
    if in_child() {
        child_resolve(true);
        return;
    }
    let (dir, decoy) = build_decoy("decoy_cublas_missing_symbol.c");
    let output = run_child(
        "a_libcublas_so_12_missing_a_required_symbol_is_refused_before_gpu_work",
        Some(dir.path()),
    );
    let text = report(&output);
    if !gpu_visible(&text) {
        return;
    }
    assert!(
        output.status.success(),
        "the child must exit 0 with a typed refusal; a panic fails:\n{text}"
    );
    for needle in [
        "LibraryUnavailable",
        "reason: MissingSymbol",
        "missing_symbols: [\"cublasSgemmStridedBatched\"]",
        decoy.to_str().expect("utf-8 path"),
        "12.6.4",
    ] {
        assert!(text.contains(needle), "{needle:?} missing from the child output:\n{text}");
    }
    assert!(!text.contains("device built"), "no device may be built:\n{text}");
    assert!(!text.contains("matmul ran"), "no matmul may run:\n{text}");
}

/// T-CUDA-COMPAT C3 (design § 9.3): run only on the GCP VM with the driver
/// and no CUDA toolkit, where no CUDA 12 cuBLAS exists.
#[test]
#[ignore = "C3 GCP cell"]
fn missing_cuda_12_cublas_is_a_typed_refusal_with_the_remedy() {
    if in_child() {
        child_resolve(false);
        // `auto` in the same process: CPU with the refusal's reason, and no
        // second stderr line.
        std::env::set_var("FATHOMDB_EMBED_DEVICE", "auto");
        let auto = resolve_default_embedder_device_from_env();
        println!("d18-child-auto: {auto:?}");
        return;
    }
    let output = run_child("missing_cuda_12_cublas_is_a_typed_refusal_with_the_remedy", None);
    let text = report(&output);
    assert!(output.status.success(), "the child must exit 0:\n{text}");
    assert!(
        !text.contains("device built"),
        "precondition failed: a CUDA 12 cuBLAS is loadable on this host:\n{text}"
    );
    for needle in [
        "LibraryUnavailable",
        "reason: Missing",
        "cuda_library_unavailable",
        "pip install nvidia-cublas-cu12 nvidia-curand-cu12",
        "nvidia.cublas",
        "nvidia.curand",
        "LD_LIBRARY_PATH",
    ] {
        assert!(text.contains(needle), "{needle:?} missing from the child output:\n{text}");
    }
    let auto = text.lines().find(|line| line.starts_with("d18-child-auto: ")).expect("auto line");
    assert!(auto.contains("effective_device: Cpu"), "{auto}");
    assert!(auto.contains("reason: Some(CudaLibraryUnavailable)"), "{auto}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let lines = stderr.lines().filter(|line| line.starts_with("fathomdb-cuda-libraries ")).count();
    assert_eq!(lines, 1, "exactly one refusal line per process:\n{text}");
}

/// Every library in the device report was really loaded: its path is in
/// this process's `/proc/self/maps`. cuRAND reports its own major, 10.
#[test]
fn the_cuda_library_report_names_the_loaded_files() {
    if in_child() {
        return;
    }
    let resolution = resolve_default_embedder_device_from_env().expect("CUDA resolution");
    let EffectiveEmbedDevice::Cuda(info) = resolution.effective_device else {
        live::require_live_or_skip("slice31_cuda_library_major: no CUDA device selected; skipping");
        return;
    };
    let maps = std::fs::read_to_string("/proc/self/maps").expect("/proc/self/maps");
    assert!(!info.cuda_libraries.is_empty(), "the report lists the loaded libraries");
    for library in ["driver", "cublas", "curand"] {
        assert!(
            info.cuda_libraries.iter().any(|report| report.library == library),
            "{library} missing from {:?}",
            info.cuda_libraries
        );
    }
    for report in &info.cuda_libraries {
        let path = report.path.as_deref().expect("a loaded library has a path");
        assert!(maps.contains(path), "{path} is not mapped in this process: {report:?}");
        assert_eq!(report.built_major, 12, "{report:?}");
    }
    let curand =
        info.cuda_libraries.iter().find(|report| report.library == "curand").expect("curand");
    assert!(
        curand.version.as_deref().is_some_and(|version| version.starts_with("10.")),
        "{curand:?}"
    );
    println!("cuda_libraries: {:?}", info.cuda_libraries);
}

/// Owner ruling Q24: a refusal writes one `fathomdb-cuda-libraries {json}`
/// line per process, however often the probe refuses.
#[test]
fn a_refusal_writes_one_stderr_line() {
    if in_child() {
        child_resolve(false);
        child_resolve(false);
        return;
    }
    let (dir, _decoy) = build_decoy("decoy_cublas.c");
    let output = run_child("a_refusal_writes_one_stderr_line", Some(dir.path()));
    let text = report(&output);
    if !gpu_visible(&text) {
        return;
    }
    assert!(output.status.success(), "{text}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let lines: Vec<&str> =
        stderr.lines().filter(|line| line.starts_with("fathomdb-cuda-libraries ")).collect();
    assert_eq!(lines.len(), 1, "exactly one refusal line for two refusals:\n{text}");
    assert!(lines[0].contains("\"reason\":\"wrong_major\""), "{}", lines[0]);
}

/// Regression guard (not a RED): a successful load writes nothing.
#[test]
fn a_successful_load_writes_no_stderr_line() {
    if in_child() {
        child_resolve(false);
        return;
    }
    let output = run_child("a_successful_load_writes_no_stderr_line", None);
    let text = report(&output);
    if !gpu_visible(&text) {
        return;
    }
    assert!(output.status.success(), "{text}");
    assert!(text.contains("device built"), "the child must build its device:\n{text}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.lines().any(|line| line.starts_with("fathomdb-cuda-libraries ")), "{text}");
}
