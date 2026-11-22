#![cfg(target_os = "linux")]
use std::process::Command;

#[test]
fn offline_build_description_is_available_without_gui_database_or_filesystem_effects() {
    let scratch = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_fathomdb-inspector"))
        .env_clear()
        .current_dir(scratch.path())
        .arg("--build-description")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "build description must work before GUI/database startup: {output:?}"
    );
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["protocol"], 1);
    assert_eq!(value["compiled_schema"], fathomdb_schema::SCHEMA_VERSION);
    assert_eq!(value["environment_policy"], "cleared-env-v2");
    assert_eq!(value["fixture_control"], cfg!(feature = "fixture-control"));
    assert_eq!(value["cached_default_cpu"], cfg!(feature = "cached-cpu"));
    assert_eq!(value["build"].as_str().unwrap().len(), 64);
    assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
}
