use current_history_followups::fixture;
use fathomdb_engine::{configure_runtime, RuntimeSqliteMode};

#[test]
fn child_fixture_runtime_probe() {
    let Some(path) = std::env::var_os("POC_RUNTIME_PROBE_PATH") else {
        return;
    };
    fixture::build(std::path::Path::new(&path), "node", 0, 128, 2).unwrap();
    let configured = configure_runtime(RuntimeSqliteMode::Performance);
    assert!(
        configured.is_ok(),
        "fixture must configure Performance before any SQLite open: {configured:?}"
    );
}

#[test]
fn fresh_and_cached_fixture_processes_share_explicit_production_runtime() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.sqlite");
    for phase in ["fresh", "cached"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "child_fixture_runtime_probe", "--nocapture"])
            .env("POC_RUNTIME_PROBE_PATH", &path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{phase}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
