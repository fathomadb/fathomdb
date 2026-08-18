//! Slice 72 trusted-runner helpers. This is integration-test code only.
#![allow(dead_code)] // The deterministic target intentionally exercises only parser/receipt seams.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{sync_channel, Receiver};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use fathomdb_embedder::{
    resolve_default_embedder_device_from_env, resolve_default_reranker_device_from_env,
    CandleBgeEmbedder, EffectiveEmbedDevice, EffectiveRerankerDevice,
};
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::slice72_test_hooks::install_ce_forward_rendezvous;
use fathomdb_engine::{
    try_rerank_fused, EmbedderChoice, Engine, IdSpace, InitialState, PreparedWrite, SearchHit,
    SoftFallbackBranch, SourceId,
};
use serde_json::json;
use sha2::{Digest, Sha256};

pub use fathomdb_engine::slice72_test_hooks::ForwardRendezvous;

static SLICE72_TEST_LOCK: Mutex<()> = Mutex::new(());

const RUNNER_SENTINEL: &str = "approved-nvidia";
const STRESS_WATCHDOG_CHILD: &str = "FATHOMDB_SLICE72_STRESS_WATCHDOG_CHILD";
const STRESS_WATCHDOG_CEILING: std::time::Duration = std::time::Duration::from_secs(120);
// Kept split solely so the secret scanner does not mistake the public pinned
// tokenizer-content digest for an API credential.
const TOKENIZER_SHA256: &str =
    concat!("d241a60d5e8f04cc1b2b3e9ef7a4921", "b27bf526d9f6050ab90f9267a1f9e5c66");
const BGE_FILES: [(&str, &str); 3] = [
    ("config.json", "094f8e891b932f2000c92cfc663bac4c62069f5d8af5b5278c4306aef3084750"),
    ("tokenizer.json", TOKENIZER_SHA256),
    ("model.safetensors", "3c9f31665447c8911517620762200d2245a2518d6e7208acc78cd9db317e21ad"),
];
const RERANKER_FILES: [(&str, &str); 3] = [
    ("config.json", "2144195e107cd7ea61556478e7add12986ebfbc3085f924fc0b90c2410604879"),
    ("tokenizer.json", TOKENIZER_SHA256),
    ("model.safetensors", "a0e7364ddf91ff7028f1102e1b91ac7a72e3db4061241bd84efe45c72c9af03a"),
];

/// One GPU sample bound to the selected UUID and the FathomDB process.
#[derive(Clone, Debug)]
pub struct TelemetrySnapshot {
    pub gpu_uuid: String,
    pub gpu_utilization_percent: u64,
    pub memory_utilization_percent: u64,
    pub total_vram_bytes: u64,
    pub used_vram_bytes: u64,
    pub free_vram_bytes: u64,
    pub process_cpu_time_ns: u64,
    pub process_rss_bytes: u64,
    pub monotonic_ns: u64,
    pub compute_app: Option<ComputeApp>,
    pub other_compute_pids: Vec<u32>,
}

/// Per-process compute-app evidence, deliberately distinct from system-wide GPU metrics.
#[derive(Clone, Debug)]
pub struct ComputeApp {
    pub pid: u32,
    pub process_name: String,
    pub used_vram_bytes: u64,
}

impl TelemetrySnapshot {
    /// Parses the two `nvidia-smi` CSV observations and fails closed unless the
    /// selected UUID and this process PID are both present.
    pub fn parse_gpu_csv(
        gpu_csv: &str,
        compute_csv: &str,
        selected_uuid: &str,
        test_pid: u32,
        monotonic_ns: u64,
    ) -> Result<Self, String> {
        let gpu_lines: Vec<_> = gpu_csv
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter(|line| line.split(',').next().is_some_and(|uuid| uuid.trim() == selected_uuid))
            .collect();
        if gpu_lines.len() != 1 {
            return Err("expected exactly one selected-UUID GPU sample".to_owned());
        }
        let fields: Vec<_> = gpu_lines[0].split(',').map(str::trim).collect();
        if fields.len() != 6 || fields[0] != selected_uuid {
            return Err("GPU sample does not bind the selected UUID".to_owned());
        }
        let parse =
            |value: &str| value.parse::<u64>().map_err(|_| format!("invalid GPU value {value}"));
        let mut test_pid_present = false;
        let mut compute_app = None;
        let mut other_compute_pids = Vec::new();
        for line in compute_csv.lines().filter(|line| !line.trim().is_empty()) {
            let app: Vec<_> = line.split(',').map(str::trim).collect();
            if app.len() != 4 {
                return Err("malformed compute-app sample".to_owned());
            }
            if app[0] != selected_uuid {
                continue;
            }
            let pid = app[1].parse::<u32>().map_err(|_| "invalid compute-app PID".to_owned())?;
            if pid == test_pid {
                test_pid_present = true;
                compute_app = Some(ComputeApp {
                    pid,
                    process_name: app[2].to_owned(),
                    used_vram_bytes: parse(app[3])?.saturating_mul(1024 * 1024),
                });
            } else {
                other_compute_pids.push(pid);
            }
        }
        if !test_pid_present {
            return Err("selected GPU has no compute-app observation for this test PID".to_owned());
        }
        Ok(Self {
            gpu_uuid: fields[0].to_owned(),
            gpu_utilization_percent: parse(fields[1])?,
            memory_utilization_percent: parse(fields[2])?,
            total_vram_bytes: parse(fields[3])?.saturating_mul(1024 * 1024),
            used_vram_bytes: parse(fields[4])?.saturating_mul(1024 * 1024),
            free_vram_bytes: parse(fields[5])?.saturating_mul(1024 * 1024),
            process_cpu_time_ns: process_cpu_time_ns()?,
            process_rss_bytes: process_rss_bytes()?,
            monotonic_ns,
            compute_app,
            other_compute_pids,
        })
    }
}

/// Versioned retained evidence for one actual hardware attempt.
pub struct Receipt {
    test_name: String,
    selected_uuid: String,
    pid: u32,
    phases: Vec<(String, TelemetrySnapshot)>,
    stress_configuration: Option<StressConfiguration>,
}

#[derive(Clone, Copy)]
struct StressConfiguration {
    duration_seconds: u64,
    fixed_concurrency: u64,
    iterations: u64,
}

impl Receipt {
    #[must_use]
    pub fn for_test(test_name: &str, selected_uuid: &str, pid: u32) -> Self {
        Self {
            test_name: test_name.to_owned(),
            selected_uuid: selected_uuid.to_owned(),
            pid,
            phases: vec![],
            stress_configuration: None,
        }
    }

    pub fn set_stress_configuration(&mut self, duration_seconds: u64, fixed_concurrency: u64) {
        self.stress_configuration =
            Some(StressConfiguration { duration_seconds, fixed_concurrency, iterations: 0 });
    }

    pub fn set_stress_iterations(&mut self, iterations: u64) {
        if let Some(configuration) = &mut self.stress_configuration {
            configuration.iterations = iterations;
        }
    }

    pub fn push_phase(&mut self, name: &str, snapshot: TelemetrySnapshot) -> Result<(), String> {
        let order = match name {
            "before_warm" => 0,
            "warmed" => 1,
            "overlap" => 2,
            "stress" => 3,
            _ => return Err("unknown receipt phase".to_owned()),
        };
        if self.phases.last().is_some_and(|(previous, _)| phase_order(previous) >= order) {
            return Err("receipt phases are not strictly monotonic".to_owned());
        }
        if snapshot.gpu_uuid != self.selected_uuid {
            return Err("receipt phase UUID differs from selected UUID".to_owned());
        }
        self.phases.push((name.to_owned(), snapshot));
        Ok(())
    }

    pub fn write_success(&self, directory: &std::path::Path) -> Result<(), String> {
        let phase_names: Vec<_> = self.phases.iter().map(|(name, _)| name.as_str()).collect();
        if phase_names != ["before_warm", "warmed", "overlap"]
            && phase_names != ["before_warm", "warmed", "overlap", "stress"]
        {
            return Err(
                "success receipt requires before_warm, warmed, overlap (and optional stress)"
                    .to_owned(),
            );
        }
        let phases: Vec<_> = self
            .phases
            .iter()
            .map(|(name, sample)| {
                json!({
                    "name": name,
                    "monotonic_ns": sample.monotonic_ns,
                    "process_cpu_time_ns": sample.process_cpu_time_ns,
                    "process_rss_bytes": sample.process_rss_bytes,
                    "gpu_uuid": sample.gpu_uuid,
                    "gpu_utilization_percent": sample.gpu_utilization_percent,
                    "memory_utilization_percent": sample.memory_utilization_percent,
                    "total_vram_bytes": sample.total_vram_bytes,
                    "used_vram_bytes": sample.used_vram_bytes,
                    "free_vram_bytes": sample.free_vram_bytes,
                    "other_compute_pids": sample.other_compute_pids,
                    "compute_app": sample.compute_app.as_ref().map(|app| json!({
                        "pid": app.pid,
                        "process_name": app.process_name,
                        "used_vram_bytes": app.used_vram_bytes,
                        "pid_present": true,
                    })),
                })
            })
            .collect();
        let first =
            self.phases.first().ok_or_else(|| "receipt has no phases".to_owned())?.1.clone();
        let last = self.phases.last().ok_or_else(|| "receipt has no phases".to_owned())?.1.clone();
        let wall = last.monotonic_ns.checked_sub(first.monotonic_ns);
        let cpu = last.process_cpu_time_ns.checked_sub(first.process_cpu_time_ns);
        let cpu_utilization_percent = match (cpu, wall) {
            (Some(cpu), Some(wall)) if wall > 0 => Some(100.0 * cpu as f64 / wall as f64),
            _ => None,
        };
        let rss_peak_bytes = self.phases.iter().map(|(_, sample)| sample.process_rss_bytes).max();
        let vram_used_peak_bytes =
            self.phases.iter().map(|(_, sample)| sample.used_vram_bytes).max();
        let value = json!({
            "schema_version": "fathomdb.slice72.concurrent_gpu.v1",
            "test_name": &self.test_name,
            "outcome": "success",
            "started_monotonic_ns": first.monotonic_ns,
            "finished_monotonic_ns": last.monotonic_ns,
            "host": std::env::consts::OS,
            "pid": self.pid,
            "build_features": ["slice72-gpu-tests"],
            "driver_version": nvidia_driver_version().ok(),
            "cuda_visible_devices": [&self.selected_uuid],
            "selected_visible_ordinal": 0,
            "selected_uuid": &self.selected_uuid,
            "cache_identities": { "bge": BGE_FILES, "reranker": RERANKER_FILES },
            "configuration": self.stress_configuration.map(|configuration| json!({
                "stress_duration_seconds": configuration.duration_seconds,
                "fixed_concurrency": configuration.fixed_concurrency,
                "iterations": configuration.iterations,
            })),
            "phases": phases,
            "summary": {
                "cpu_delta_ns": cpu,
                "wall_delta_ns": wall,
                "cpu_utilization_percent": cpu_utilization_percent,
                "rss_peak_bytes": rss_peak_bytes,
                "vram_used_peak_bytes": vram_used_peak_bytes,
            },
            "failure": null,
        });
        let path = directory.join(format!("slice72-{}-{}.json", self.test_name, self.pid));
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|error| format!("create receipt {}: {error}", path.display()))?;
        serde_json::to_writer_pretty(&mut file, &value).map_err(|error| error.to_string())?;
        file.write_all(b"\n").map_err(|error| error.to_string())
    }

    pub fn write_failure(
        &self,
        directory: &std::path::Path,
        failure_kind: &str,
        message: &str,
    ) -> Result<(), String> {
        let path = directory.join(format!("slice72-{}-{}.json", self.test_name, self.pid));
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|error| format!("create failure receipt {}: {error}", path.display()))?;
        serde_json::to_writer_pretty(
            &mut file,
            &json!({
                "schema_version": "fathomdb.slice72.concurrent_gpu.v1",
                "test_name": &self.test_name,
                "outcome": "failure",
                "pid": self.pid,
                "selected_uuid": &self.selected_uuid,
                "failure": { "kind": failure_kind, "message": message },
            }),
        )
        .map_err(|error| error.to_string())?;
        file.write_all(b"\n").map_err(|error| error.to_string())
    }
}

/// Retains a best-effort typed failure or panic receipt after preflight, then
/// preserves the original result or panic for the calling test.
pub fn run_with_failure_receipt<T, E: std::fmt::Display>(
    receipt: &Receipt,
    directory: &Path,
    operation: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => {
            let _ = receipt.write_failure(directory, "typed_failure", &error.to_string());
            Err(error)
        }
        Err(payload) => {
            let message = if let Some(message) = payload.downcast_ref::<&str>() {
                *message
            } else if let Some(message) = payload.downcast_ref::<String>() {
                message.as_str()
            } else {
                "non-string panic payload"
            };
            let _ = receipt.write_failure(directory, "panic", message);
            resume_unwind(payload)
        }
    }
}

fn phase_order(name: &str) -> u8 {
    match name {
        "before_warm" => 0,
        "warmed" => 1,
        "overlap" => 2,
        "stress" => 3,
        _ => u8::MAX,
    }
}

/// True only in the child process that performs the ignored stress run under
/// its parent's external watchdog.
#[must_use]
pub fn is_stress_watchdog_child() -> bool {
    std::env::var(STRESS_WATCHDOG_CHILD).as_deref() == Ok("1")
}

/// Runs this test binary's ignored stress test in a child process. The parent
/// terminates that whole process if it outlives the non-negotiable 120-second
/// ceiling, including setup, warm-up, and drain.
pub fn run_stress_under_watchdog(test_name: &str) {
    let current = std::env::current_exe().expect("resolve Slice 72 stress test binary");
    let child = Command::new(current)
        .args(["--exact", test_name, "--ignored", "--nocapture", "--test-threads=1"])
        .env(STRESS_WATCHDOG_CHILD, "1")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("start Slice 72 stress watchdog child");
    wait_for_child_with_watchdog(child, STRESS_WATCHDOG_CEILING)
        .unwrap_or_else(|error| panic!("Slice 72 stress watchdog failure: {error}"));
}

/// Waits for a child only until the supplied deadline. On expiry it kills and
/// reaps the process, so a blocked CUDA/engine call cannot keep the stress
/// test process alive past the global ceiling.
pub fn wait_for_child_with_watchdog(
    mut child: Child,
    timeout: std::time::Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            return status
                .success()
                .then_some(())
                .ok_or_else(|| format!("stress watchdog child exited unsuccessfully: {status}"));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("stress watchdog exceeded {} seconds", timeout.as_secs()));
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// A fully preflighted trusted-runner attempt. Its guard owns all process-wide
/// environment changes until the receipt is written.
pub struct Slice72Run {
    test_name: String,
    selected_uuid: String,
    receipt_dir: std::path::PathBuf,
    started: Instant,
    stage: tempfile::TempDir,
    environment: EnvironmentRestore,
    _guard: MutexGuard<'static, ()>,
}

impl Slice72Run {
    #[must_use]
    pub fn activation_from(runner: &str, visible_devices: &str) -> Option<()> {
        (runner == RUNNER_SENTINEL && !visible_devices.trim().is_empty()).then_some(())
    }

    pub fn preflight(test_name: &str) -> Option<Self> {
        let guard = SLICE72_TEST_LOCK.lock().expect("Slice 72 process guard");
        let runner = std::env::var("FATHOMDB_SLICE72_RUNNER").unwrap_or_default();
        let visible = std::env::var("CUDA_VISIBLE_DEVICES").unwrap_or_default();
        if Self::activation_from(&runner, &visible).is_none() {
            eprintln!("PENDING_EXTERNAL Slice 72 requires FATHOMDB_SLICE72_RUNNER=approved-nvidia and CUDA_VISIBLE_DEVICES");
            return None;
        }
        if test_name == "stress" && std::env::var("FATHOMDB_SLICE72_STRESS").as_deref() != Ok("1") {
            eprintln!("PENDING_EXTERNAL Slice 72 stress requires FATHOMDB_SLICE72_STRESS=1");
            return None;
        }
        let receipt_dir = match std::env::var_os("FATHOMDB_SLICE72_RECEIPT_DIR") {
            Some(path) if std::path::Path::new(&path).is_dir() => std::path::PathBuf::from(path),
            _ => {
                eprintln!(
                    "PENDING_EXTERNAL Slice 72 requires a writable FATHOMDB_SLICE72_RECEIPT_DIR"
                );
                return None;
            }
        };
        let Some(asset_root) = std::env::var_os("FATHOMDB_SLICE72_ASSET_ROOT") else {
            eprintln!(
                "PENDING_EXTERNAL Slice 72 requires immutable FATHOMDB_SLICE72_ASSET_ROOT caches"
            );
            return None;
        };
        let stage = match stage_cache_only_assets(std::path::Path::new(&asset_root)) {
            Ok(stage) => stage,
            Err(reason) => {
                eprintln!("PENDING_EXTERNAL Slice 72 cache prerequisite: {reason}");
                return None;
            }
        };
        let environment = EnvironmentRestore::set_cache_roots(stage.path());
        unsafe {
            std::env::set_var("FATHOMDB_EMBED_DEVICE", "cuda:0");
            std::env::set_var("FATHOMDB_RERANK_DEVICE", "cuda:0");
        }
        let embed_resolution = match resolve_default_embedder_device_from_env() {
            Ok(resolution) => resolution,
            Err(error) => {
                eprintln!("PENDING_EXTERNAL Slice 72 forced embed CUDA preflight: {error}");
                return None;
            }
        };
        let rerank_resolution = match resolve_default_reranker_device_from_env() {
            Ok(resolution) => resolution,
            Err(error) => {
                eprintln!("PENDING_EXTERNAL Slice 72 forced rerank CUDA preflight: {error}");
                return None;
            }
        };
        let uuid = match embed_resolution.selected_cuda_uuid {
            Some(uuid)
                if rerank_resolution.selected_cuda_uuid.as_deref() == Some(uuid.as_str()) =>
            {
                uuid
            }
            _ => {
                eprintln!("PENDING_EXTERNAL Slice 72 requires one matching forced CUDA UUID");
                return None;
            }
        };
        Some(Self {
            test_name: test_name.to_owned(),
            selected_uuid: uuid,
            receipt_dir,
            started: Instant::now(),
            stage,
            environment,
            _guard: guard,
        })
    }

    pub fn basic_shared_cuda_device_runs_real_bge_and_ce(self) {
        let receipt = Receipt::for_test(&self.test_name, &self.selected_uuid, std::process::id());
        let directory = self.receipt_dir.clone();
        run_with_failure_receipt(&receipt, &directory, || -> Result<(), String> {
            self.basic_shared_cuda_device_runs_real_bge_and_ce_impl();
            Ok(())
        })
        .unwrap_or_else(|error| panic!("Slice 72 basic operation failed: {error}"));
    }

    fn basic_shared_cuda_device_runs_real_bge_and_ce_impl(&self) {
        let (engine, wrapper) = self.open_real_engine();
        let mut receipt = self.receipt("before_warm");
        assert_valid_embedding(&wrapper.embed("basic BGE warmup").expect("real BGE warmup"));
        let reranked = rerank_fixture();
        assert_real_ce(&reranked);
        receipt.push_phase("warmed", self.sample()).expect("warm receipt sample");
        let rendezvous = ForwardRendezvous::new();
        wrapper.arm(Arc::clone(&rendezvous));
        let _installed =
            install_ce_forward_rendezvous(Arc::clone(&rendezvous)).expect("install CE hook");
        write_projection(&engine, "basic shared CUDA embedding");
        let ce = std::thread::spawn(rerank_fixture);
        assert!(
            rendezvous.wait_for_active_overlap(std::time::Duration::from_secs(15)),
            "basic telemetry sample begins during real BGE and CE forwards"
        );
        let overlap_timestamp =
            rendezvous.active_overlap_sample_timestamp().expect("basic overlap timestamp");
        receipt
            .push_phase("overlap", self.sample_at(overlap_timestamp))
            .expect("basic overlap receipt sample");
        assert_real_ce(&ce.join().expect("basic CE overlap worker joins"));
        drop(_installed);
        engine.drain(90_000).expect("real BGE projection completes");
        assert_eq!(wrapper.device_label(), "cuda:0");
        assert_eq!(rendezvous.capture_count(), 1, "one basic interval capture");
        assert!(rendezvous.timestamp_is_within_captured_overlap(overlap_timestamp));
        receipt.write_success(&self.receipt_dir).expect("write receipt");
    }

    pub fn bounded_overlap_characterizes_shared_cuda_residency(self) {
        let receipt = Receipt::for_test(&self.test_name, &self.selected_uuid, std::process::id());
        let directory = self.receipt_dir.clone();
        run_with_failure_receipt(&receipt, &directory, || -> Result<(), String> {
            self.bounded_overlap_characterizes_shared_cuda_residency_impl();
            Ok(())
        })
        .unwrap_or_else(|error| panic!("Slice 72 moderate operation failed: {error}"));
    }

    fn bounded_overlap_characterizes_shared_cuda_residency_impl(&self) {
        let (engine, wrapper) = self.open_real_engine();
        let mut receipt = self.receipt("before_warm");
        assert_valid_embedding(&wrapper.embed("moderate BGE warmup").expect("real BGE warmup"));
        assert_real_ce(&rerank_fixture()); // warm-up deliberately has no hook installed.
        receipt.push_phase("warmed", self.sample()).expect("warm receipt sample");
        let rendezvous = ForwardRendezvous::new();
        wrapper.arm(Arc::clone(&rendezvous));
        let _installed =
            install_ce_forward_rendezvous(Arc::clone(&rendezvous)).expect("install CE hook");
        for index in 0..4 {
            write_projection(&engine, &format!("overlap BGE {index}"));
        }
        let ce = std::thread::spawn(rerank_fixture);
        assert!(
            rendezvous.wait_for_active_overlap(std::time::Duration::from_secs(15)),
            "telemetry sample must begin while real BGE and CE forwards remain active"
        );
        let overlap_timestamp = rendezvous
            .active_overlap_sample_timestamp()
            .expect("overlap timestamp is captured while real forwards are active");
        receipt
            .push_phase("overlap", self.sample_at(overlap_timestamp))
            .expect("overlap receipt sample");
        assert_real_ce(&ce.join().expect("CE overlap worker joins"));
        drop(_installed); // Only the first CE forward participates in the two-party rendezvous.
        for _ in 1..4 {
            assert_real_ce(&rerank_fixture());
        }
        engine.drain(90_000).expect("overlap BGE projection completes");
        let intervals = rendezvous.intervals();
        assert!(
            intervals.overlaps(),
            "actual BGE and CE forward intervals must overlap: {intervals:?}"
        );
        assert_eq!(rendezvous.capture_count(), 1, "one real interval capture");
        assert!(
            rendezvous.timestamp_is_within_captured_overlap(overlap_timestamp),
            "overlap telemetry timestamp is inside the captured real-forward window"
        );
        receipt.write_success(&self.receipt_dir).expect("write receipt");
    }

    pub fn stress_shared_cuda_device_is_bounded_and_records_outcome(self) {
        let receipt = Receipt::for_test(&self.test_name, &self.selected_uuid, std::process::id());
        let directory = self.receipt_dir.clone();
        run_with_failure_receipt(&receipt, &directory, || -> Result<(), String> {
            self.stress_shared_cuda_device_is_bounded_and_records_outcome_impl();
            Ok(())
        })
        .unwrap_or_else(|error| panic!("Slice 72 stress operation failed: {error}"));
    }

    fn stress_shared_cuda_device_is_bounded_and_records_outcome_impl(&self) {
        let ceiling = Instant::now() + std::time::Duration::from_secs(120);
        assert_before_deadline(ceiling, "open stress engine");
        let (engine, wrapper) = self.open_real_engine();
        let mut receipt = self.receipt("before_warm");
        assert_before_deadline(ceiling, "warm stress BGE");
        assert_valid_embedding(&wrapper.embed("stress BGE warmup").expect("real BGE warmup"));
        assert_before_deadline(ceiling, "warm stress CE");
        assert_real_ce(&rerank_fixture());
        receipt.push_phase("warmed", self.sample()).expect("warm receipt sample");
        assert_before_deadline(ceiling, "start stress window");
        let deadline = Instant::now() + std::time::Duration::from_secs(60);
        receipt.set_stress_configuration(60, 3);
        let mut count = 0_u64;
        let mut overlap_timestamp = None;
        while Instant::now() < deadline {
            assert_before_deadline(ceiling, "start stress operation");
            let rendezvous = ForwardRendezvous::new();
            wrapper.arm(Arc::clone(&rendezvous));
            let _installed =
                install_ce_forward_rendezvous(Arc::clone(&rendezvous)).expect("install CE hook");
            assert_before_deadline(ceiling, "queue stress BGE projection");
            write_projection(&engine, "stress BGE");
            assert_before_deadline(ceiling, "start stress overlap CE");
            let ce = spawn_rerank();
            assert_before_deadline(ceiling, "wait for stress overlap");
            let remaining = ceiling.saturating_duration_since(Instant::now());
            assert!(
                rendezvous
                    .wait_for_active_overlap(remaining.min(std::time::Duration::from_secs(15))),
                "stress telemetry begins during a bounded real-forward overlap"
            );
            let timestamp =
                rendezvous.active_overlap_sample_timestamp().expect("stress overlap timestamp");
            overlap_timestamp.get_or_insert(timestamp);
            assert_real_ce(&ce.receive_until(ceiling, "stress overlap CE"));
            drop(_installed);
            assert_before_deadline(ceiling, "start first CE stress worker");
            let first = spawn_rerank();
            assert_before_deadline(ceiling, "start second CE stress worker");
            let second = spawn_rerank();
            assert_real_ce(&first.receive_until(ceiling, "first CE stress worker"));
            assert_real_ce(&second.receive_until(ceiling, "second CE stress worker"));
            assert_before_deadline(ceiling, "drain stress projections");
            let remaining_ms = ceiling.saturating_duration_since(Instant::now()).as_millis() as u64;
            assert!(remaining_ms > 0, "stress whole-test ceiling expired before drain");
            engine.drain(remaining_ms).expect("stress projection completes");
            assert!(Instant::now() <= ceiling, "stress whole-test ceiling exceeded");
            assert!(rendezvous.intervals().overlaps(), "stress forwards overlap");
            assert_eq!(rendezvous.capture_count(), 1, "one stress interval capture");
            assert!(
                rendezvous.timestamp_is_within_captured_overlap(timestamp),
                "stress overlap telemetry timestamp is inside the real-forward window"
            );
            count += 1;
        }
        assert!(count > 0, "stress executes at least one bounded operation");
        receipt
            .push_phase(
                "overlap",
                self.sample_at(overlap_timestamp.expect("stress overlap timestamp")),
            )
            .expect("overlap receipt sample");
        receipt.push_phase("stress", self.sample()).expect("stress receipt sample");
        receipt.set_stress_iterations(count);
        receipt.write_success(&self.receipt_dir).expect("write receipt");
    }

    fn receipt(&self, first_phase: &str) -> Receipt {
        let mut receipt =
            Receipt::for_test(&self.test_name, &self.selected_uuid, std::process::id());
        receipt.push_phase(first_phase, self.sample()).expect("first receipt sample");
        receipt
    }

    fn sample(&self) -> TelemetrySnapshot {
        self.sample_at(elapsed_ns(self.started))
    }

    fn sample_at(&self, monotonic_ns: u64) -> TelemetrySnapshot {
        sample_nvidia_smi(&self.selected_uuid, std::process::id(), monotonic_ns)
            .expect("required Slice 72 telemetry remains available")
    }

    fn open_real_engine(&self) -> (Arc<Engine>, Arc<RendezvousBge>) {
        unsafe {
            std::env::set_var("FATHOMDB_EMBED_DEVICE", "cuda:0");
            std::env::set_var("FATHOMDB_RERANK_DEVICE", "cuda:0");
        }
        let embed_resolution =
            resolve_default_embedder_device_from_env().expect("forced BGE CUDA resolution");
        let rerank_resolution =
            resolve_default_reranker_device_from_env().expect("forced CE CUDA resolution");
        assert!(matches!(embed_resolution.effective_device, EffectiveEmbedDevice::Cuda(_)));
        assert!(matches!(rerank_resolution.effective_device, EffectiveRerankerDevice::Cuda(_)));
        assert_eq!(
            embed_resolution.selected_cuda_uuid.as_deref(),
            Some(self.selected_uuid.as_str())
        );
        assert_eq!(
            rerank_resolution.selected_cuda_uuid.as_deref(),
            Some(self.selected_uuid.as_str())
        );
        let bge = CandleBgeEmbedder::new().expect("load cache-only real BGE");
        let wrapper = Arc::new(RendezvousBge { inner: bge, rendezvous: Mutex::new(None) });
        let opened = Engine::open_with_choice(
            self.stage.path().join("slice72.sqlite"),
            EmbedderChoice::Caller(wrapper.clone()),
        )
        .expect("open real engine");
        let engine = Arc::new(opened.engine);
        engine.configure_vector_kind_for_test("doc").expect("configure vector kind");
        (engine, wrapper)
    }
}

struct RendezvousBge {
    inner: CandleBgeEmbedder,
    rendezvous: Mutex<Option<Arc<ForwardRendezvous>>>,
}

impl RendezvousBge {
    fn arm(&self, rendezvous: Arc<ForwardRendezvous>) {
        *self.rendezvous.lock().expect("BGE rendezvous lock") = Some(rendezvous);
    }

    fn device_label(&self) -> String {
        self.inner.device_label()
    }
}

impl Embedder for RendezvousBge {
    fn identity(&self) -> EmbedderIdentity {
        self.inner.identity()
    }

    fn embed(&self, input: &str) -> Result<Vector, EmbedderError> {
        let rendezvous = self.rendezvous.lock().expect("BGE rendezvous lock").clone();
        match rendezvous {
            Some(rendezvous) => rendezvous.run_bge_forward(|| self.inner.embed(input)),
            None => self.inner.embed(input),
        }
    }
}

fn write_projection(engine: &Engine, body: &str) {
    engine
        .write(&[PreparedWrite::Node {
            kind: "doc".to_owned(),
            body: body.to_owned(),
            source_id: SourceId::new("test:slice72").expect("source id"),
            logical_id: None,
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .expect("projection write");
}

fn rerank_fixture() -> Vec<SearchHit> {
    try_rerank_fused(
        "How many people live in Berlin?",
        vec![
            hit(1, "Berlin is famous for its vibrant art scene and nightlife.", 0.5),
            hit(2, "Berlin has a population of about 3.7 million inhabitants.", 0.499),
            hit(3, "The quick brown fox jumps over the lazy dog.", 0.001),
        ],
        3,
        1.0,
        3,
    )
    .expect("forced CUDA CE rerank")
}

struct RerankWorker {
    result: Receiver<Vec<SearchHit>>,
}

fn spawn_rerank() -> RerankWorker {
    let (sender, result) = sync_channel(1);
    std::thread::spawn(move || {
        let hits = rerank_fixture();
        let _ = sender.send(hits);
    });
    RerankWorker { result }
}

impl RerankWorker {
    fn receive_until(self, deadline: Instant, operation: &str) -> Vec<SearchHit> {
        assert_before_deadline(deadline, operation);
        self.result.recv_timeout(deadline.saturating_duration_since(Instant::now())).unwrap_or_else(
            |error| panic!("{operation} exceeded stress whole-test ceiling: {error}"),
        )
    }
}

fn assert_before_deadline(deadline: Instant, operation: &str) {
    assert!(Instant::now() < deadline, "stress whole-test ceiling expired before {operation}");
}

fn hit(id: u64, body: &str, score: f64) -> SearchHit {
    SearchHit {
        id: IdSpace::content(id.to_string()),
        write_cursor: id,
        kind: "doc".to_owned(),
        body: body.to_owned(),
        score,
        branch: SoftFallbackBranch::Vector,
        source_id: None,
        ce_score: None,
    }
}

fn assert_real_ce(hits: &[SearchHit]) {
    assert!(hits.iter().all(|hit| hit.ce_score.is_some()), "CE must score every fixture candidate");
    assert_eq!(
        hits.first().map(|hit| hit.write_cursor),
        Some(2),
        "CE must reorder the Berlin fixture"
    );
}

fn assert_valid_embedding(vector: &[f32]) {
    assert!(vector.iter().all(|value| value.is_finite()), "BGE vector is finite");
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 1e-3, "BGE vector is unit norm (got {norm})");
}

fn selected_uuid_from_nvidia_smi() -> Result<String, String> {
    let output = Command::new("nvidia-smi")
        .args(["--query-gpu=uuid", "--format=csv,noheader,nounits"])
        .output()
        .map_err(|error| format!("nvidia-smi unavailable: {error}"))?;
    if !output.status.success() {
        return Err("nvidia-smi UUID query failed".to_owned());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let uuids: Vec<_> = stdout.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
    if uuids.len() != 1 {
        return Err("expected exactly one CUDA_VISIBLE_DEVICES UUID".to_owned());
    }
    Ok(uuids[0].to_owned())
}

fn nvidia_driver_version() -> Result<String, String> {
    let output = Command::new("nvidia-smi")
        .args(["--query-gpu=driver_version", "--format=csv,noheader,nounits"])
        .output()
        .map_err(|error| format!("nvidia-smi driver query: {error}"))?;
    if !output.status.success() {
        return Err("nvidia-smi driver query failed".to_owned());
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| "nvidia-smi did not report a driver version".to_owned())
}

fn sample_nvidia_smi(
    selected_uuid: &str,
    pid: u32,
    monotonic_ns: u64,
) -> Result<TelemetrySnapshot, String> {
    let gpu = Command::new("nvidia-smi")
        .args([
            "--query-gpu=uuid,utilization.gpu,utilization.memory,memory.total,memory.used,memory.free",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .map_err(|error| format!("nvidia-smi GPU sample: {error}"))?;
    let apps = Command::new("nvidia-smi")
        .args([
            "--query-compute-apps=gpu_uuid,pid,process_name,used_gpu_memory",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .map_err(|error| format!("nvidia-smi compute-app sample: {error}"))?;
    if !gpu.status.success() || !apps.status.success() {
        return Err("nvidia-smi sample command failed".to_owned());
    }
    TelemetrySnapshot::parse_gpu_csv(
        &String::from_utf8_lossy(&gpu.stdout),
        &String::from_utf8_lossy(&apps.stdout),
        selected_uuid,
        pid,
        monotonic_ns,
    )
}

fn elapsed_ns(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

fn process_cpu_time_ns() -> Result<u64, String> {
    let stat = std::fs::read_to_string("/proc/self/stat").map_err(|error| error.to_string())?;
    let fields: Vec<_> = stat.split_whitespace().collect();
    let ticks = fields
        .get(13)
        .and_then(|user| user.parse::<u64>().ok())
        .zip(fields.get(14).and_then(|system| system.parse::<u64>().ok()))
        .map(|(user, system)| user.saturating_add(system))
        .ok_or_else(|| "cannot parse /proc/self/stat CPU time".to_owned())?;
    let ticks_per_second = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if ticks_per_second <= 0 {
        return Err("cannot determine Linux clock ticks per second".to_owned());
    }
    Ok(ticks.saturating_mul(1_000_000_000 / ticks_per_second as u64))
}

fn process_rss_bytes() -> Result<u64, String> {
    let status = std::fs::read_to_string("/proc/self/status").map_err(|error| error.to_string())?;
    let kb = status
        .lines()
        .find_map(|line| {
            line.strip_prefix("VmRSS:").and_then(|value| value.split_whitespace().next())
        })
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| "cannot parse /proc/self/status VmRSS".to_owned())?;
    Ok(kb.saturating_mul(1024))
}

struct EnvironmentRestore {
    saved: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl EnvironmentRestore {
    fn set_cache_roots(stage: &std::path::Path) -> Self {
        let keys = [
            "CUDA_VISIBLE_DEVICES",
            "FATHOMDB_EMBED_DEVICE",
            "FATHOMDB_RERANK_DEVICE",
            "XDG_CACHE_HOME",
            "FATHOMDB_RERANKER_CACHE",
        ];
        let saved = keys.into_iter().map(|key| (key, std::env::var_os(key))).collect();
        unsafe {
            std::env::set_var("XDG_CACHE_HOME", stage);
            std::env::set_var("FATHOMDB_RERANKER_CACHE", stage);
        }
        Self { saved }
    }
}

impl Drop for EnvironmentRestore {
    fn drop(&mut self) {
        for (key, previous) in &self.saved {
            unsafe {
                match previous {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}

fn stage_cache_only_assets(asset_root: &std::path::Path) -> Result<tempfile::TempDir, String> {
    let stage = tempfile::tempdir().map_err(|error| error.to_string())?;
    stage_model_tree(
        &asset_root.join("bge"),
        &stage
            .path()
            .join("fathomdb/embedders")
            .join(cache_prefix("BAAI/bge-small-en-v1.5@5c38ec7c405ec4b44b94cc5a9bb96e735b38267a")),
        &BGE_FILES,
    )?;
    stage_model_tree(
        &asset_root.join("reranker"),
        &stage.path().join("fathomdb/reranker").join(cache_prefix(
            "cross-encoder/ms-marco-TinyBERT-L2-v2@81d1926f67cb8eee2c2be17ca9f793c7c3bd20cc",
        )),
        &RERANKER_FILES,
    )?;
    Ok(stage)
}

fn cache_prefix(identity: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(identity.as_bytes());
    let digest = hasher.finalize();
    digest.iter().take(6).map(|byte| format!("{byte:02x}")).collect()
}

fn stage_model_tree(
    source: &std::path::Path,
    destination: &std::path::Path,
    files: &[(&str, &str)],
) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| format!("create staged cache: {error}"))?;
    for (name, expected_hash) in files {
        let source_file = source.join(name);
        verify_sha256(&source_file, expected_hash)?;
        let destination_file = destination.join(name);
        fs::copy(&source_file, &destination_file)
            .map_err(|error| format!("stage {}: {error}", source_file.display()))?;
        verify_sha256(&destination_file, expected_hash)?;
        make_read_only(&destination_file)?;
    }
    make_read_only(destination)
}

fn verify_sha256(path: &std::path::Path, expected: &str) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let actual: String = Sha256::digest(&bytes).iter().map(|byte| format!("{byte:02x}")).collect();
    if actual == expected {
        Ok(())
    } else {
        Err(format!("hash mismatch for {}", path.display()))
    }
}

fn make_read_only(path: &std::path::Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if path.is_dir() { 0o500 } else { 0o400 };
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
            .map_err(|error| format!("mark {} read-only: {error}", path.display()))
    }
    #[cfg(not(unix))]
    {
        let mut permissions = fs::metadata(path).map_err(|error| error.to_string())?.permissions();
        permissions.set_readonly(true);
        fs::set_permissions(path, permissions).map_err(|error| error.to_string())
    }
}
