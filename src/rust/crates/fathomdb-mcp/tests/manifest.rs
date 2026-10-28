//! Manifest lint (MR-070.2) and rmcp confinement (design §3): the adapter
//! reaches FathomDB only through `fathomdb-sdk`, never `rusqlite`, and only
//! the protocol edge and the binary name rmcp.

use std::fs;
use std::path::{Path, PathBuf};

fn crate_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn manifest() -> toml::Table {
    let text = fs::read_to_string(crate_dir().join("Cargo.toml")).unwrap();
    toml::from_str(&text).unwrap()
}

fn dependency_tables(manifest: &toml::Table) -> Vec<(String, toml::Table)> {
    let mut tables = Vec::new();
    for key in ["dependencies", "build-dependencies"] {
        if let Some(table) = manifest.get(key).and_then(toml::Value::as_table) {
            tables.push((key.to_string(), table.clone()));
        }
    }
    if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
        for (target, spec) in targets {
            for key in ["dependencies", "build-dependencies"] {
                if let Some(table) = spec.get(key).and_then(toml::Value::as_table) {
                    tables.push((format!("target.{target}.{key}"), table.clone()));
                }
            }
        }
    }
    tables
}

fn package_name(name: &str, spec: &toml::Value) -> String {
    spec.get("package").and_then(toml::Value::as_str).unwrap_or(name).to_string()
}

#[test]
fn fathomdb_sdk_is_the_only_fathomdb_dependency_and_rusqlite_is_absent() {
    let manifest = manifest();
    let mut saw_sdk = false;
    for (section, table) in dependency_tables(&manifest) {
        for (name, spec) in &table {
            let package = package_name(name, spec);
            if package == "fathomdb-sdk" {
                saw_sdk = true;
                continue;
            }
            assert!(
                !package.starts_with("fathomdb"),
                "[{section}] depends on {package}; only fathomdb-sdk is allowed (MR-070.2)"
            );
            assert!(
                !package.starts_with("rusqlite") && !package.starts_with("libsqlite3"),
                "[{section}] depends on {package}; the adapter must not reach SQLite"
            );
        }
    }
    assert!(saw_sdk, "fathomdb-sdk must be a dependency");
}

#[test]
fn features_only_forward_to_fathomdb_sdk() {
    let manifest = manifest();
    let features = manifest.get("features").and_then(toml::Value::as_table).unwrap();
    for (feature, members) in features {
        for member in members.as_array().unwrap() {
            let member = member.as_str().unwrap();
            if let Some((dependency, _)) = member.split_once('/') {
                let dependency = dependency.trim_start_matches("dep:").trim_end_matches('?');
                assert!(
                    !dependency.starts_with("fathomdb") || dependency == "fathomdb-sdk",
                    "feature {feature} reaches {dependency}"
                );
            }
        }
    }
}

#[test]
fn rmcp_is_pinned_exactly() {
    let manifest = manifest();
    let rmcp = &manifest["dependencies"]["rmcp"];
    let version = rmcp.get("version").and_then(toml::Value::as_str).unwrap();
    assert!(version.starts_with('='), "rmcp must be pinned with `=`, got {version}");
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn only_the_edge_and_main_name_rmcp() {
    let src = crate_dir().join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    let allowed = [src.join("edge.rs"), src.join("main.rs")];
    let mut edge_names_rmcp = false;
    for file in files {
        let text = fs::read_to_string(&file).unwrap();
        let names_rmcp = text.contains("rmcp::") || text.contains("use rmcp");
        if allowed.contains(&file) {
            edge_names_rmcp |= file.ends_with("edge.rs") && names_rmcp;
            continue;
        }
        assert!(!names_rmcp, "{} names rmcp; only src/edge.rs and src/main.rs may", file.display());
    }
    assert!(edge_names_rmcp, "src/edge.rs must be the protocol edge built on rmcp");
}
