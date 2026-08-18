//! Offline, benchmark-only TC-5 vector-stage executor.
//!
//! This binary deliberately accepts only a pinned specification and result path.
//! It emits no corpus content, paths, raw IDs, or predictions.

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use fathomdb_embedder::{CandleBgeEmbedder, ExplicitCandleDevice};
use fathomdb_embedder_api::{Embedder, EmbedderIdentity, Vector};
use fathomdb_engine::{
    tc5_benchmark::{VectorStageRequest, VectorStageScope},
    EmbedderChoice, Engine,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const RESULT_VERSION: u32 = 1;
const WORKLOAD: &str = "vector_stage_v1";
const ALGORITHM: &str = "bit_knn_f32_rerank_v1";
const RERANK: &str = "exact_f32";

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Spec {
    version: u32,
    workload: String,
    algorithm: String,
    rerank: String,
    candidate_k: usize,
    top_k: usize,
    warmups: u32,
    repetitions: u32,
    single_process: bool,
    manifest: String,
    settings_digest: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    model_asset_directory: String,
    model_asset_digest: String,
    database: String,
    scope_kind: String,
    selection_digest: String,
    query: String,
    expected_vector_rows: usize,
    allowed_candidate_k: Vec<usize>,
    allowed_top_k: Vec<usize>,
    fixture_digest: String,
    index_digest: String,
    query_digest: String,
    seed_digest: String,
    cuda_uuid: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DeviceInfo {
    uuid: String,
    pci_bus_id: String,
    name: String,
    driver: String,
    cuda_ordinal: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SelectedDevice {
    Cpu,
    Cuda(DeviceInfo),
}

impl SelectedDevice {
    fn logical_label(&self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Cuda(_) => "cuda:0",
        }
    }
}

/// Receives the preflight-selected device explicitly. It never consults an
/// ambient device variable and must never call the ordinary downloading loader.
trait CacheOnlyEmbedderFactory {
    fn local_asset_identity(&self, asset_dir: &Path) -> Result<String, FactoryFailure>;
    fn create(
        &self,
        device: &SelectedDevice,
        asset_dir: &Path,
    ) -> Result<Box<dyn CacheOnlyQueryEmbedder>, FactoryFailure>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum FactoryFailure {
    AssetUnavailable,
    DeviceUnavailable,
}

trait CacheOnlyQueryEmbedder: Send + Sync {
    fn device_label(&self) -> String;
    fn identity(&self) -> EmbedderIdentity;
    fn embed_query(&self, query: &str) -> Result<Vector, String>;
}

struct CandleCacheOnlyFactory;

impl CacheOnlyQueryEmbedder for CandleBgeEmbedder {
    fn device_label(&self) -> String {
        self.device_label()
    }

    fn identity(&self) -> EmbedderIdentity {
        Embedder::identity(self)
    }

    fn embed_query(&self, query: &str) -> Result<Vector, String> {
        self.embed(query).map_err(|error| format!("{error:?}"))
    }
}

impl CacheOnlyEmbedderFactory for CandleCacheOnlyFactory {
    fn local_asset_identity(&self, asset_dir: &Path) -> Result<String, FactoryFailure> {
        asset_directory_identity(asset_dir).map_err(|_| FactoryFailure::AssetUnavailable)
    }

    fn create(
        &self,
        device: &SelectedDevice,
        asset_dir: &Path,
    ) -> Result<Box<dyn CacheOnlyQueryEmbedder>, FactoryFailure> {
        let device = match device {
            SelectedDevice::Cpu => ExplicitCandleDevice::Cpu,
            SelectedDevice::Cuda(info) => ExplicitCandleDevice::Cuda(info.cuda_ordinal as usize),
        };
        CandleBgeEmbedder::new_from_local_asset_on_device(asset_dir, device)
            .map(|embedder| Box::new(embedder) as Box<dyn CacheOnlyQueryEmbedder>)
            .map_err(|error| match error {
                fathomdb_embedder::loader::EmbedderLoadError::DeviceUnavailable { .. } => {
                    FactoryFailure::DeviceUnavailable
                }
                _ => FactoryFailure::AssetUnavailable,
            })
    }
}

struct BoxedEmbedder {
    inner: Box<dyn CacheOnlyQueryEmbedder>,
}

impl Embedder for BoxedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        self.inner.identity()
    }

    fn embed(&self, input: &str) -> Result<Vector, fathomdb_embedder_api::EmbedderError> {
        self.inner
            .embed_query(input)
            .map_err(|message| fathomdb_embedder_api::EmbedderError::Failed { message })
    }
}

#[derive(Serialize)]
struct MeasurementResult<'a> {
    version: u32,
    status: &'static str,
    resolved_settings_digest: &'a str,
    algorithm: &'static str,
    rerank: &'static str,
    candidate_k: usize,
    top_k: usize,
    fixture_digest: &'a str,
    index_digest: &'a str,
    query_digest: &'a str,
    seed_digest: &'a str,
    model_asset_digest: &'a str,
    selection_digest: &'a str,
    embedding_device: &'a str,
    candidate_execution: &'a str,
    rerank_execution: &'a str,
    selected_vector_rows: usize,
    candidate_count: usize,
    rerank_count: usize,
    ground_truth_count: usize,
    candidate_ids_digest: String,
    rerank_ids_digest: String,
    ground_truth_ids_digest: String,
    candidate_elapsed_ns: u128,
    rerank_elapsed_ns: u128,
    ground_truth_elapsed_ns: u128,
    vector_stage_route_count: u8,
    search_route_count: u8,
    fts_route_count: u8,
    fusion_route_count: u8,
    graph_route_count: u8,
    cross_encoder_route_count: u8,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("fathomdb-tc5-benchmark: {message}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    reject_fathomdb_controls()?;
    let (spec_path, result_path) = parse_args(env::args().skip(1))?;
    reject_result_destination(&result_path)?;
    let spec = parse_json::<Spec>(&spec_path)?;
    validate_spec_shape(&spec)?;
    let manifest = parse_json::<Manifest>(Path::new(&spec.manifest))?;
    validate_manifest(&manifest, &spec)?;
    validate_resolved_settings_digest(&spec, &manifest)?;
    let visible = match discover_visible_cuda() {
        Ok(visible) => visible,
        Err(_) => {
            return write_nonmeasurement(
                &result_path,
                "device_unavailable",
                "device_preflight_failed",
                &spec,
            )
        }
    };
    execute_with_factory(&result_path, &spec, &manifest, &visible, &CandleCacheOnlyFactory)
}

fn execute_with_factory(
    result_path: &Path,
    spec: &Spec,
    manifest: &Manifest,
    visible: &[DeviceInfo],
    factory: &dyn CacheOnlyEmbedderFactory,
) -> Result<(), String> {
    let asset_dir = Path::new(&manifest.model_asset_directory);
    let observed_asset_identity = match factory.local_asset_identity(asset_dir) {
        Ok(identity) => identity,
        Err(FactoryFailure::AssetUnavailable) => {
            return write_nonmeasurement(
                result_path,
                "asset_unavailable",
                "cache_asset_missing",
                spec,
            )
        }
        Err(FactoryFailure::DeviceUnavailable) => {
            return write_nonmeasurement(
                result_path,
                "device_unavailable",
                "device_preflight_failed",
                spec,
            )
        }
    };
    if observed_asset_identity != manifest.model_asset_digest {
        return Err("local model asset identity does not match manifest pin".into());
    }
    let selected = match select_device(visible, manifest.cuda_uuid.as_deref()) {
        Ok(selected) => selected,
        Err(_) => {
            return write_nonmeasurement(
                result_path,
                "device_unavailable",
                "device_preflight_failed",
                spec,
            )
        }
    };
    if matches!(selected, SelectedDevice::Cuda(_)) && !cfg!(feature = "tc5-benchmark-cuda") {
        return write_nonmeasurement(
            result_path,
            "device_unavailable",
            "cuda_binary_unavailable",
            spec,
        );
    }
    let embedder = match factory.create(&selected, asset_dir) {
        Ok(embedder) => embedder,
        Err(FactoryFailure::AssetUnavailable) => {
            return write_nonmeasurement(
                result_path,
                "asset_unavailable",
                "cache_asset_invalid",
                spec,
            )
        }
        Err(FactoryFailure::DeviceUnavailable) => {
            return write_nonmeasurement(
                result_path,
                "device_unavailable",
                "device_factory_failed",
                spec,
            )
        }
    };
    if embedder.device_label() != selected.logical_label() {
        return write_nonmeasurement(
            result_path,
            "device_unavailable",
            "effective_device_mismatch",
            spec,
        );
    }
    let query_vector = match embedder.embed_query(&manifest.query) {
        Ok(vector) => vector,
        Err(_) => {
            return write_nonmeasurement(
                result_path,
                "device_unavailable",
                "device_probe_failed",
                spec,
            )
        }
    };
    if digest_text(&manifest.query) != manifest.query_digest {
        return Err("qualified query identity does not match manifest pin".into());
    }
    if sha256_file(Path::new(&manifest.database))? != manifest.index_digest {
        return Err("qualified index identity does not match manifest pin".into());
    }
    if VectorStageScope::kind(&manifest.scope_kind).selection_digest() != manifest.selection_digest
    {
        return Err("manifest scope selection digest mismatch".into());
    }
    let selected_scope = VectorStageScope::kind(&manifest.scope_kind);
    let embedding_device = embedder.device_label();
    let embedder: Arc<dyn Embedder> = Arc::new(BoxedEmbedder { inner: embedder });
    let opened = Engine::open_with_choice(&manifest.database, EmbedderChoice::Caller(embedder))
        .map_err(|_| "cannot open qualified benchmark database".to_string())?;
    let stage = opened
        .engine
        .tc5_vector_stage(VectorStageRequest {
            query_vector,
            candidate_k: spec.candidate_k,
            top_k: spec.top_k,
            scope: selected_scope,
            expected_vector_rows: manifest.expected_vector_rows,
        })
        .map_err(|_| "benchmark vector-stage execution failed".to_string())?;
    write_measurement(result_path, spec, manifest, &embedding_device, &stage)
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<(PathBuf, PathBuf), String> {
    let values: Vec<String> = args.collect();
    if values.len() != 4 || values[0] != "--spec" || values[2] != "--result" {
        return Err("usage is exactly: --spec <path> --result <path>".into());
    }
    if values[1].is_empty() || values[3].is_empty() {
        return Err("paths must be nonempty".into());
    }
    Ok((PathBuf::from(&values[1]), PathBuf::from(&values[3])))
}

fn reject_fathomdb_controls() -> Result<(), String> {
    if let Some((name, _)) =
        env::vars_os().find(|(name, _)| name.to_string_lossy().starts_with("FATHOMDB_"))
    {
        return Err(format!("ambient FathomDB control {name:?} is forbidden"));
    }
    Ok(())
}

fn parse_json<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<T, String> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| "cannot read input".to_string())?
        .read_to_end(&mut bytes)
        .map_err(|_| "cannot read input".to_string())?;
    serde_json::from_slice(&bytes).map_err(|_| "invalid strict JSON input".to_string())
}

fn validate_spec_shape(spec: &Spec) -> Result<(), String> {
    if spec.version != 1
        || spec.workload != WORKLOAD
        || spec.algorithm != ALGORITHM
        || spec.rerank != RERANK
    {
        return Err("unsupported benchmark specification".into());
    }
    if spec.candidate_k == 0
        || spec.top_k == 0
        || spec.top_k > spec.candidate_k
        || spec.warmups > 100
        || !(1..=1000).contains(&spec.repetitions)
        || !spec.single_process
    {
        return Err("invalid benchmark execution bounds".into());
    }
    Ok(())
}

fn validate_resolved_settings_digest(spec: &Spec, manifest: &Manifest) -> Result<(), String> {
    let manifest_identity = canonical_digest(&[
        ("model_asset_digest", &manifest.model_asset_digest),
        ("expected_vector_rows", &manifest.expected_vector_rows.to_string()),
        ("allowed_candidate_k", &canonical_usize_list(&manifest.allowed_candidate_k)),
        ("allowed_top_k", &canonical_usize_list(&manifest.allowed_top_k)),
        ("fixture_digest", &manifest.fixture_digest),
        ("index_digest", &manifest.index_digest),
        ("query_digest", &manifest.query_digest),
        ("seed_digest", &manifest.seed_digest),
        ("scope_kind", &manifest.scope_kind),
        ("selection_digest", &manifest.selection_digest),
        ("cuda_uuid", manifest.cuda_uuid.as_deref().unwrap_or("")),
    ]);
    let expected = canonical_digest(&[
        ("version", &spec.version.to_string()),
        ("workload", &spec.workload),
        ("algorithm", &spec.algorithm),
        ("rerank", &spec.rerank),
        ("candidate_k", &spec.candidate_k.to_string()),
        ("top_k", &spec.top_k.to_string()),
        ("warmups", &spec.warmups.to_string()),
        ("repetitions", &spec.repetitions.to_string()),
        ("single_process", &spec.single_process.to_string()),
        ("manifest_identity", &manifest_identity),
    ]);
    if spec.settings_digest != expected {
        return Err("settings digest mismatch".into());
    }
    Ok(())
}

fn validate_manifest(manifest: &Manifest, spec: &Spec) -> Result<(), String> {
    if manifest.version != 1
        || manifest.expected_vector_rows == 0
        || spec.candidate_k > manifest.expected_vector_rows
        || !manifest.allowed_candidate_k.contains(&spec.candidate_k)
        || !manifest.allowed_top_k.contains(&spec.top_k)
        || [
            manifest.model_asset_digest.as_str(),
            manifest.fixture_digest.as_str(),
            manifest.index_digest.as_str(),
            manifest.query_digest.as_str(),
            manifest.seed_digest.as_str(),
        ]
        .iter()
        .any(|pin| !is_digest(pin))
    {
        return Err("qualified manifest pin or range mismatch".into());
    }
    Ok(())
}

fn select_device(
    devices: &[DeviceInfo],
    pinned_uuid: Option<&str>,
) -> Result<SelectedDevice, String> {
    if devices.is_empty() {
        return Ok(SelectedDevice::Cpu);
    }
    let pinned_uuid = pinned_uuid.ok_or("visible CUDA requires a manifest UUID pin")?;
    let matching: Vec<&DeviceInfo> =
        devices.iter().filter(|device| device.uuid == pinned_uuid).collect();
    if matching.len() != 1 || matching[0].cuda_ordinal != 0 {
        return Err("visible CUDA mapping is ambiguous or does not match the pin".into());
    }
    Ok(SelectedDevice::Cuda(matching[0].clone()))
}

fn canonical_usize_list(values: &[usize]) -> String {
    values.iter().map(usize::to_string).collect::<Vec<_>>().join(",")
}

fn discover_visible_cuda() -> Result<Vec<DeviceInfo>, String> {
    let output = match std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=uuid,pci.bus_id,name,driver_version", "--format=csv,noheader"])
        .output()
    {
        Ok(output) if output.status.success() => output,
        Ok(_) => return Err("nvidia-smi reported an unusable CUDA environment".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err("cannot preflight CUDA visibility".into()),
    };
    output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .enumerate()
        .map(|(ordinal, line)| {
            let text = std::str::from_utf8(line)
                .map_err(|_| "invalid CUDA preflight output".to_string())?;
            let fields: Vec<&str> = text.split(',').map(str::trim).collect();
            if fields.len() != 4 || fields.iter().any(|field| field.is_empty()) {
                return Err("ambiguous CUDA preflight output".to_string());
            }
            Ok(DeviceInfo {
                uuid: fields[0].into(),
                pci_bus_id: fields[1].into(),
                name: fields[2].into(),
                driver: fields[3].into(),
                cuda_ordinal: ordinal as u32,
            })
        })
        .collect()
}

fn is_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn write_nonmeasurement(
    result: &Path,
    status: &str,
    code: &str,
    spec: &Spec,
) -> Result<(), String> {
    let body = format!("{{\"version\":{RESULT_VERSION},\"status\":\"{status}\",\"error_code\":\"{code}\",\"resolved_settings_digest\":\"{}\"}}", spec.settings_digest);
    install_new(result, body.as_bytes())
}

fn write_measurement(
    result: &Path,
    spec: &Spec,
    manifest: &Manifest,
    embedding_device: &str,
    stage: &fathomdb_engine::tc5_benchmark::VectorStageResult,
) -> Result<(), String> {
    if stage.routes.search != 0
        || stage.routes.fts != 0
        || stage.routes.fusion != 0
        || stage.routes.graph != 0
        || stage.routes.cross_encoder != 0
        || stage.routes.vector_stage != 1
    {
        return Err("prohibited route observation".into());
    }
    let body = serde_json::to_vec(&MeasurementResult {
        version: RESULT_VERSION,
        status: "measurement_complete",
        resolved_settings_digest: &spec.settings_digest,
        algorithm: ALGORITHM,
        rerank: RERANK,
        candidate_k: spec.candidate_k,
        top_k: spec.top_k,
        fixture_digest: &manifest.fixture_digest,
        index_digest: &manifest.index_digest,
        query_digest: &manifest.query_digest,
        seed_digest: &manifest.seed_digest,
        model_asset_digest: &manifest.model_asset_digest,
        selection_digest: &stage.selection_digest,
        embedding_device,
        candidate_execution: stage.candidate_execution,
        rerank_execution: stage.rerank_execution,
        selected_vector_rows: stage.selected_vector_rows,
        candidate_count: stage.candidate_count,
        rerank_count: stage.rerank.len(),
        ground_truth_count: stage.ground_truth.len(),
        candidate_ids_digest: digest_row_keys(&stage.candidates),
        rerank_ids_digest: digest_row_keys(&stage.rerank),
        ground_truth_ids_digest: digest_row_keys(&stage.ground_truth),
        candidate_elapsed_ns: stage.candidate_elapsed_ns,
        rerank_elapsed_ns: stage.rerank_elapsed_ns,
        ground_truth_elapsed_ns: stage.ground_truth_elapsed_ns,
        vector_stage_route_count: stage.routes.vector_stage,
        search_route_count: stage.routes.search,
        fts_route_count: stage.routes.fts,
        fusion_route_count: stage.routes.fusion,
        graph_route_count: stage.routes.graph,
        cross_encoder_route_count: stage.routes.cross_encoder,
    })
    .map_err(|_| "cannot serialize measurement result".to_string())?;
    install_new(result, &body)
}

fn asset_directory_identity(asset_dir: &Path) -> Result<String, String> {
    let fields = ["config.json", "tokenizer.json", "model.safetensors"]
        .into_iter()
        .map(|name| {
            let hash = sha256_file(&asset_dir.join(name))?;
            Ok((name, hash))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(canonical_digest(
        &fields.iter().map(|(name, hash)| (*name, hash.as_str())).collect::<Vec<_>>(),
    ))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|_| "cannot read qualified local asset".to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count =
            file.read(&mut buffer).map_err(|_| "cannot read qualified local asset".to_string())?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}

fn digest_text(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}

fn digest_row_keys(values: &[u64]) -> String {
    let encoded = values.iter().map(u64::to_string).collect::<Vec<_>>();
    let fields = encoded.iter().map(|value| ("row_key", value.as_str())).collect::<Vec<_>>();
    canonical_digest(&fields)
}

fn reject_result_destination(path: &Path) -> Result<(), String> {
    if path.exists() || fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink()) {
        return Err("result destination already exists or is a symlink".into());
    }
    Ok(())
}

fn install_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("result needs a parent directory")?;
    let temporary = parent.join(format!(".tc5-result-{}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|_| "cannot create result temporary".to_string())?;
    file.write_all(bytes).map_err(|_| "cannot write result".to_string())?;
    file.sync_all().map_err(|_| "cannot sync result".to_string())?;
    // `hard_link` fails if `path` appeared after validation, unlike rename's
    // replacement semantics. Both names are in one directory/filesystem.
    if fs::hard_link(&temporary, path).is_err() {
        let _ = fs::remove_file(&temporary);
        return Err("result destination raced or cannot install".into());
    }
    fs::remove_file(temporary).map_err(|_| "cannot finalize result".to_string())
}

fn canonical_digest(fields: &[(&str, &str)]) -> String {
    let mut hasher = Sha256::new();
    for (name, value) in fields {
        for part in [name.as_bytes(), value.as_bytes()] {
            hasher.update((part.len() as u64).to_be_bytes());
            hasher.update(part);
        }
    }
    hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    #[derive(Clone)]
    struct FakeEmbedder {
        effective_device: String,
        probe_result: Result<Vector, String>,
        calls: Arc<Mutex<Vec<&'static str>>>,
    }

    impl CacheOnlyQueryEmbedder for FakeEmbedder {
        fn device_label(&self) -> String {
            self.effective_device.clone()
        }

        fn identity(&self) -> EmbedderIdentity {
            EmbedderIdentity::new("tc5-fake", "1", 384)
        }

        fn embed_query(&self, _query: &str) -> Result<Vector, String> {
            self.calls.lock().unwrap().push("embed");
            self.probe_result.clone()
        }
    }

    struct FakeFactory {
        identity: Result<String, FactoryFailure>,
        created: Result<FakeEmbedder, FactoryFailure>,
        calls: Arc<Mutex<Vec<&'static str>>>,
    }

    impl CacheOnlyEmbedderFactory for FakeFactory {
        fn local_asset_identity(&self, _asset_dir: &Path) -> Result<String, FactoryFailure> {
            self.calls.lock().unwrap().push("asset");
            self.identity.clone()
        }

        fn create(
            &self,
            _device: &SelectedDevice,
            _asset_dir: &Path,
        ) -> Result<Box<dyn CacheOnlyQueryEmbedder>, FactoryFailure> {
            self.calls.lock().unwrap().push("create");
            self.created
                .clone()
                .map(|embedder| Box::new(embedder) as Box<dyn CacheOnlyQueryEmbedder>)
        }
    }

    fn valid_spec() -> Spec {
        Spec {
            version: 1,
            workload: WORKLOAD.into(),
            algorithm: ALGORITHM.into(),
            rerank: RERANK.into(),
            candidate_k: 192,
            top_k: 10,
            warmups: 0,
            repetitions: 1,
            single_process: true,
            manifest: "unused".into(),
            settings_digest: "a".repeat(64),
        }
    }

    fn valid_manifest(asset_directory: &str, asset_digest: &str) -> Manifest {
        Manifest {
            version: 1,
            model_asset_directory: asset_directory.into(),
            model_asset_digest: asset_digest.into(),
            database: "/private/qualified-index.db".into(),
            scope_kind: "doc".into(),
            selection_digest: VectorStageScope::kind("doc").selection_digest().into(),
            query: "qualified query".into(),
            expected_vector_rows: 192,
            allowed_candidate_k: vec![192],
            allowed_top_k: vec![10],
            fixture_digest: "0".repeat(64),
            index_digest: "1".repeat(64),
            query_digest: digest_text("qualified query"),
            seed_digest: "3".repeat(64),
            cuda_uuid: None,
        }
    }

    #[test]
    fn canonical_digest_is_length_prefixed() {
        assert_ne!(canonical_digest(&[("a", "bc")]), canonical_digest(&[("ab", "c")]));
    }

    #[test]
    fn strict_cli_rejects_extra_or_reordered_arguments() {
        assert!(
            parse_args(["--result", "x", "--spec", "y"].map(str::to_owned).into_iter()).is_err()
        );
        assert!(parse_args(
            ["--spec", "x", "--result", "y", "extra"].map(str::to_owned).into_iter()
        )
        .is_err());
    }

    #[test]
    fn finalization_never_replaces_an_existing_result() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("result.json");
        fs::write(&path, b"original").unwrap();
        assert!(install_new(&path, b"replacement").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original");
    }

    #[test]
    fn manifest_requires_allowed_values_and_sha_pins() {
        let spec = Spec {
            version: 1,
            workload: WORKLOAD.into(),
            algorithm: ALGORITHM.into(),
            rerank: RERANK.into(),
            candidate_k: 192,
            top_k: 10,
            warmups: 0,
            repetitions: 1,
            single_process: true,
            manifest: "unused".into(),
            settings_digest: String::new(),
        };
        let manifest = Manifest {
            version: 1,
            model_asset_directory: "unused".into(),
            model_asset_digest: "0".repeat(64),
            database: "unused".into(),
            scope_kind: "doc".into(),
            selection_digest: VectorStageScope::kind("doc").selection_digest().into(),
            query: "qualified query".into(),
            expected_vector_rows: 192,
            allowed_candidate_k: vec![192],
            allowed_top_k: vec![10],
            fixture_digest: "0".repeat(64),
            index_digest: "0".repeat(64),
            query_digest: "0".repeat(64),
            seed_digest: "invalid".into(),
            cuda_uuid: None,
        };
        assert!(validate_manifest(&manifest, &spec).is_err());
    }

    #[test]
    fn visible_cuda_requires_the_manifest_pinned_uuid_and_never_falls_back() {
        let devices = vec![DeviceInfo {
            uuid: "GPU-pinned".into(),
            pci_bus_id: "0000:01:00.0".into(),
            name: "test GPU".into(),
            driver: "test-driver".into(),
            cuda_ordinal: 0,
        }];
        let selected = select_device(&devices, Some("GPU-pinned")).unwrap();
        assert_eq!(selected.logical_label(), "cuda:0");
        assert!(select_device(&devices, Some("GPU-other")).is_err());
        assert!(select_device(&devices, None).is_err());
        assert_eq!(select_device(&[], Some("GPU-pinned")).unwrap().logical_label(), "cpu");
    }

    #[test]
    fn verified_directory_identity_and_real_embed_probe_precede_measurement() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let factory = FakeFactory {
            identity: Ok("a".repeat(64)),
            created: Ok(FakeEmbedder {
                effective_device: "cpu".into(),
                probe_result: Ok(vec![0.0; 384]),
                calls: calls.clone(),
            }),
            calls: calls.clone(),
        };
        let dir = tempdir().unwrap();
        let result = dir.path().join("result.json");
        let outcome = execute_with_factory(
            &result,
            &valid_spec(),
            &valid_manifest("/private/model-cache", &"a".repeat(64)),
            &[],
            &factory,
        );

        assert_eq!(outcome.unwrap_err(), "cannot read qualified local asset");
        assert_eq!(&*calls.lock().unwrap(), &["asset", "create", "embed"]);
        assert!(!result.exists(), "a failed measurement must not create a result");
    }

    #[test]
    fn effective_device_drift_is_a_typed_nonmeasurement_before_embedding() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let factory = FakeFactory {
            identity: Ok("a".repeat(64)),
            created: Ok(FakeEmbedder {
                effective_device: "cuda:0".into(),
                probe_result: Ok(vec![0.0; 384]),
                calls: calls.clone(),
            }),
            calls: calls.clone(),
        };
        let dir = tempdir().unwrap();
        let result = dir.path().join("result.json");
        let mut manifest = valid_manifest("/private/model-cache", &"a".repeat(64));
        manifest.cuda_uuid = None;

        execute_with_factory(&result, &valid_spec(), &manifest, &[], &factory).unwrap();

        assert_eq!(&*calls.lock().unwrap(), &["asset", "create"]);
        let output = std::fs::read_to_string(result).unwrap();
        assert!(output.contains("device_unavailable"));
        assert!(!output.contains("/private/model-cache"));
    }

    #[test]
    fn asset_failure_is_typed_and_never_leaks_the_local_directory() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let factory = FakeFactory {
            identity: Err(FactoryFailure::AssetUnavailable),
            created: Err(FactoryFailure::AssetUnavailable),
            calls: calls.clone(),
        };
        let dir = tempdir().unwrap();
        let result = dir.path().join("result.json");

        execute_with_factory(
            &result,
            &valid_spec(),
            &valid_manifest("/private/model-cache", &"a".repeat(64)),
            &[],
            &factory,
        )
        .unwrap();

        assert_eq!(&*calls.lock().unwrap(), &["asset"]);
        let output = std::fs::read_to_string(result).unwrap();
        assert!(output.contains("asset_unavailable"));
        assert!(output.contains("\"workload\":\"vector_stage_v1\""));
        assert!(!output.contains("/private/model-cache"));
    }

    #[test]
    fn measurement_result_contains_digests_not_internal_row_keys_or_paths() {
        let dir = tempdir().unwrap();
        let result = dir.path().join("result.json");
        let spec = valid_spec();
        let manifest = valid_manifest("/private/model-cache", &"a".repeat(64));
        let stage = fathomdb_engine::tc5_benchmark::VectorStageResult {
            candidates: vec![101, 202],
            rerank: vec![202],
            ground_truth: vec![101],
            candidate_count: 2,
            selected_vector_rows: 2,
            selection_digest: manifest.selection_digest.clone(),
            candidate_elapsed_ns: 0,
            rerank_elapsed_ns: 0,
            ground_truth_elapsed_ns: 0,
            candidate_execution: "cpu/sqlite-vec",
            rerank_execution: "cpu/sqlite-vec",
            routes: fathomdb_engine::tc5_benchmark::RouteAttestation {
                vector_stage: 1,
                ..Default::default()
            },
        };

        write_measurement(&result, &spec, &manifest, "cpu", &stage).unwrap();
        let output = fs::read_to_string(result).unwrap();
        assert!(output.contains("measurement_complete"));
        assert!(output.contains("\"workload\":\"vector_stage_v1\""));
        assert!(output.contains("index_construction_device"));
        assert!(output.contains("runtime_identity"));
        assert!(output.contains("build_identity"));
        assert!(output.contains("candidate_ids_digest"));
        assert!(!output.contains("[101"));
        assert!(!output.contains(",202"));
        assert!(!output.contains("/private/model-cache"));
    }

    proptest! {
        #[test]
        fn strict_spec_codec_round_trips_allowlisted_controls(
            candidate_k in 1_usize..=192,
            top_k in 1_usize..=192,
            warmups in 0_u32..=100,
            repetitions in 1_u32..=1_000,
        ) {
            prop_assume!(top_k <= candidate_k);
            let spec = Spec {
                version: 1,
                workload: WORKLOAD.into(),
                algorithm: ALGORITHM.into(),
                rerank: RERANK.into(),
                candidate_k,
                top_k,
                warmups,
                repetitions,
                single_process: true,
                manifest: "qualified-manifest.json".into(),
                settings_digest: "0".repeat(64),
            };
            let encoded = serde_json::to_vec(&spec).unwrap();
            let decoded: Spec = serde_json::from_slice(&encoded).unwrap();
            prop_assert_eq!(decoded, spec);
        }
    }
}
