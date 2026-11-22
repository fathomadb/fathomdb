use sha2::{Digest, Sha256};
use std::path::Path;
fn collect(path: &Path, hash: &mut Sha256) {
    if path.is_dir() {
        let mut children =
            std::fs::read_dir(path).unwrap().map(|e| e.unwrap().path()).collect::<Vec<_>>();
        children.sort();
        for child in children {
            collect(&child, hash);
        }
    } else {
        println!("cargo:rerun-if-changed={}", path.display());
        hash.update(path.to_string_lossy().as_bytes());
        hash.update(std::fs::read(path).unwrap());
    }
}
fn constant(source: &str, name: &str) -> String {
    let start = source.find(&format!("const {name}:")).expect("selected constant must exist");
    let value = source[start..].split_once('=').unwrap().1.split_once(';').unwrap().0.trim();
    value.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(value).to_string()
}
fn main() {
    let mut hash = Sha256::new();
    for path in [
        "src",
        "assets",
        "Cargo.toml",
        "build.rs",
        "../../Cargo.lock",
        "../rust/crates/fathomdb-engine/src",
        "../rust/crates/fathomdb-schema/src",
        "../rust/crates/fathomdb-query/src",
        "../rust/crates/fathomdb-embedder/src",
        "../rust/crates/fathomdb-embedder-api/src",
    ] {
        let p = Path::new(path);
        if p.exists() {
            collect(p, &mut hash);
        }
    }
    let target = std::env::var("TARGET").unwrap();
    let profile = std::env::var("PROFILE").unwrap();
    let cached_cpu = std::env::var_os("CARGO_FEATURE_CACHED_CPU").is_some();
    let fixture_control = std::env::var_os("CARGO_FEATURE_FIXTURE_CONTROL").is_some();
    let output = std::process::Command::new(std::env::var_os("RUSTC").unwrap())
        .arg("--version")
        .arg("--verbose")
        .output()
        .unwrap();
    assert!(output.status.success());
    let rustc = String::from_utf8(output.stdout).unwrap();
    for key in [
        "TARGET",
        "PROFILE",
        "CARGO_CFG_TARGET_FEATURE",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_FEATURE_FIXTURE_CONTROL",
        "CARGO_FEATURE_CACHED_CPU",
        "OPT_LEVEL",
        "DEBUG",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
        hash.update(key);
        hash.update(std::env::var(key).unwrap_or_default());
    }
    hash.update(rustc.as_bytes());
    hash.update(b"protocol1/cleared-env-v2/sealed-cache-cpu-v1");
    let digest: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    println!("cargo:rustc-env=INSPECTOR_BUILD={digest}");
    let engine = std::fs::read_to_string("../rust/crates/fathomdb-engine/src/open.rs").unwrap();
    let loader = std::fs::read_to_string("../rust/crates/fathomdb-embedder/src/loader.rs").unwrap();
    for (key, name) in [
        ("INSPECTOR_DEFAULT_NAME", "DEFAULT_EMBEDDER_NAME"),
        ("INSPECTOR_DEFAULT_REVISION", "DEFAULT_EMBEDDER_REVISION"),
        ("INSPECTOR_DEFAULT_DIMENSION", "DEFAULT_EMBEDDER_DIMENSION"),
        ("INSPECTOR_MEAN_NAME", "BGE_SMALL_EMBEDDER_NAME"),
    ] {
        let value = constant(&engine, name);
        assert!(!value.contains(['\n', '\r']));
        println!("cargo:rustc-env={key}={value}");
    }
    assert_eq!(
        constant(&engine, "DEFAULT_EMBEDDER_REVISION"),
        constant(&loader, "HF_REVISION"),
        "Inspector pinned CPU asset/Engine revision drift"
    );
    let schema = std::fs::read_to_string("../rust/crates/fathomdb-schema/src/lib.rs").unwrap();
    let mut description = serde_json::json!({"version":env!("CARGO_PKG_VERSION"),"build":digest,"target":target,"profile":profile,"rustc":rustc,"fixture_control":fixture_control,"protocol":1,"environment_policy":"cleared-env-v2","cached_environment_policy":"sealed-cache-cpu-v1","cached_default_cpu":cached_cpu,"compiled_schema":constant(&schema,"SCHEMA_VERSION").parse::<u32>().unwrap(),"engine_default_identity":{"name":constant(&engine,"DEFAULT_EMBEDDER_NAME"),"revision":constant(&engine,"DEFAULT_EMBEDDER_REVISION"),"dimension":constant(&engine,"DEFAULT_EMBEDDER_DIMENSION").parse::<u32>().unwrap()},"model_revision":constant(&loader,"HF_REVISION"),"model_repository":constant(&loader,"HF_REPO"),"assets":{}});
    for (file, key) in [
        ("config.json", "CONFIG_JSON_SHA256"),
        ("tokenizer.json", "TOKENIZER_JSON_SHA256"),
        ("model.safetensors", "MODEL_SAFETENSORS_SHA256"),
    ] {
        let hash = constant(&loader, key);
        assert!(hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()));
        description["assets"][file] = serde_json::Value::String(hash);
    }
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(out.join("build-description.json"), serde_json::to_vec(&description).unwrap())
        .unwrap();
}
