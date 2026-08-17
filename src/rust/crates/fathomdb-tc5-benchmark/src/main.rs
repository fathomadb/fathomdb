//! Offline, benchmark-only TC-5 vector-stage executor.
//!
//! This binary deliberately accepts only a pinned specification and result path.
//! It emits no corpus content, paths, raw IDs, or predictions.

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Deserialize;
use sha2::{Digest, Sha256};

const RESULT_VERSION: u32 = 1;
const WORKLOAD: &str = "vector_stage_v1";
const ALGORITHM: &str = "bit_knn_f32_rerank_v1";
const RERANK: &str = "exact_f32";

#[derive(Deserialize)]
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    model_asset: String,
    expected_vector_rows: usize,
    allowed_candidate_k: Vec<usize>,
    allowed_top_k: Vec<usize>,
    fixture_digest: String,
    index_digest: String,
    query_digest: String,
    seed_digest: String,
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
    validate_spec(&spec)?;
    let manifest = parse_json::<Manifest>(Path::new(&spec.manifest))?;
    validate_manifest(&manifest, &spec)?;
    if !Path::new(&manifest.model_asset).is_file() {
        return write_nonmeasurement(
            &result_path,
            "asset_unavailable",
            "cache_asset_missing",
            &spec,
        );
    }
    // A real selected-device cache-only Candle factory is intentionally required
    // before any measurement. This executable never falls back to the ordinary
    // loader, ambient device parsing, or a downloader.
    Err("cache-only selected-device factory unavailable in this build; no measurement emitted"
        .into())
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

fn validate_spec(spec: &Spec) -> Result<(), String> {
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
    use tempfile::tempdir;

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
            model_asset: "unused".into(),
            expected_vector_rows: 192,
            allowed_candidate_k: vec![192],
            allowed_top_k: vec![10],
            fixture_digest: "0".repeat(64),
            index_digest: "0".repeat(64),
            query_digest: "0".repeat(64),
            seed_digest: "invalid".into(),
        };
        assert!(validate_manifest(&manifest, &spec).is_err());
    }
}
