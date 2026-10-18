//! Config grammar, defaults, invalid values, store-name grammar, root
//! resolution and path rejection (design §9 order 1; MR-005.3, MR-014.4,
//! MR-005.7, MR-060).

use std::fs;
use std::path::Path;
use std::process::Command;

use fathomdb_mcp::config::NameError;
use fathomdb_mcp::config::{Config, Resolution, SearchMode};

fn parse(json: &str, base: &Path) -> Result<Config, String> {
    Config::from_json_str(json, base).map_err(|error| error.to_string())
}

fn invalid(json: &str, base: &Path) -> String {
    match parse(json, base) {
        Ok(_) => panic!("expected {json} to be invalid"),
        Err(message) => message,
    }
}

#[test]
fn empty_object_yields_documented_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let config = parse("{}", dir.path()).unwrap();
    assert!(config.stores.is_empty());
    assert!(config.roots.is_empty());
    assert_eq!(config.default_store, None);
    assert_eq!(config.idle_timeout_secs, 60);
    assert_eq!(config.max_open_stores, 1);
    assert_eq!(config.admission_cap, 4);
    assert_eq!(config.call_deadline_ms, 10_000);
    assert_eq!(config.default_max_tokens, 4_000);
    assert_eq!(config.snippet_chars, 300);
    assert_eq!(config.search_mode, SearchMode::Text);
    assert!(!config.use_default_embedder);
    assert_eq!(config.telemetry_dir, None);
}

#[test]
fn default_config_matches_empty_json() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(Config::default(), parse("{}", dir.path()).unwrap());
}

#[test]
fn relative_paths_resolve_against_the_config_directory() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("dbs")).unwrap();
    let json = r#"{
        "stores": {"notes": {"path": "notes.sqlite", "create": true, "eager": true},
                   "abs": {"path": "/var/tmp/abs.sqlite"}},
        "roots": [{"path": "dbs"}, {"path": "dbs", "suffix": ".db"}],
        "default_store": "notes",
        "idle_timeout_secs": 0,
        "admission_cap": 7,
        "call_deadline_ms": 100,
        "default_max_tokens": 256,
        "snippet_chars": 40,
        "search_mode": "hybrid",
        "use_default_embedder": true,
        "telemetry_dir": "telemetry",
        "write": false,
        "scope": null,
        "max_open_stores": 1
    }"#;
    let config = parse(json, dir.path()).unwrap();
    let notes = &config.stores["notes"];
    assert_eq!(notes.path, dir.path().join("notes.sqlite"));
    assert!(notes.create);
    assert!(notes.eager);
    let abs = &config.stores["abs"];
    assert_eq!(abs.path, Path::new("/var/tmp/abs.sqlite"));
    assert!(!abs.create);
    assert!(!abs.eager);
    assert_eq!(config.roots.len(), 2);
    assert_eq!(config.roots[0].path, dir.path().join("dbs"));
    assert_eq!(config.roots[0].suffix, ".sqlite");
    assert_eq!(config.roots[1].suffix, ".db");
    assert_eq!(config.default_store.as_deref(), Some("notes"));
    assert_eq!(config.idle_timeout_secs, 0);
    assert_eq!(config.admission_cap, 7);
    assert_eq!(config.call_deadline_ms, 100);
    assert_eq!(config.default_max_tokens, 256);
    assert_eq!(config.snippet_chars, 40);
    assert_eq!(config.search_mode, SearchMode::Hybrid);
    assert!(config.use_default_embedder);
    assert_eq!(config.telemetry_dir, Some(dir.path().join("telemetry")));
}

#[test]
fn release_fixed_values_are_enforced() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    // MR-005.7: one open store at a time.
    assert!(invalid(r#"{"max_open_stores": 2}"#, base).contains("max_open_stores"));
    assert!(invalid(r#"{"max_open_stores": 0}"#, base).contains("max_open_stores"));
    // MR-060: read-only.
    assert!(invalid(r#"{"write": true}"#, base).contains("write"));
    // MR-014.4: no scope binding.
    assert!(invalid(r#"{"scope": {"tenant": "a"}}"#, base).contains("scope"));
    assert!(invalid(r#"{"scope": "a"}"#, base).contains("scope"));
}

#[test]
fn numeric_ranges_are_enforced() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    assert!(invalid(r#"{"admission_cap": 0}"#, base).contains("admission_cap"));
    assert!(invalid(r#"{"admission_cap": 8}"#, base).contains("admission_cap"));
    assert!(invalid(r#"{"call_deadline_ms": 99}"#, base).contains("call_deadline_ms"));
    assert!(invalid(r#"{"default_max_tokens": 255}"#, base).contains("default_max_tokens"));
    assert!(invalid(r#"{"snippet_chars": 39}"#, base).contains("snippet_chars"));
    assert!(invalid(r#"{"snippet_chars": 2001}"#, base).contains("snippet_chars"));
    assert!(parse(r#"{"snippet_chars": 2000}"#, base).is_ok());
    assert!(invalid(r#"{"idle_timeout_secs": -1}"#, base).contains("idle_timeout_secs"));
}

#[test]
fn queue_limit_defaults_to_32_and_must_be_at_least_1() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    let defaults = parse("{}", base).unwrap();
    assert_eq!(parse(r#"{"queue_limit": 32}"#, base).unwrap(), defaults, "32 is the default");
    assert_ne!(parse(r#"{"queue_limit": 1}"#, base).unwrap(), defaults);
    let message = invalid(r#"{"queue_limit": 0}"#, base);
    assert!(message.contains("queue_limit: must be at least 1"), "{message}");
    let message = invalid(r#"{"queue_limit": "many"}"#, base);
    assert!(message.contains("queue_limit: must be a non-negative integer"), "{message}");
}

#[test]
fn run_limit_ms_defaults_to_60_s_and_is_never_below_the_call_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    let defaults = parse("{}", base).unwrap();
    assert_eq!(parse(r#"{"run_limit_ms": 60000}"#, base).unwrap(), defaults, "60 s is the default");
    assert_ne!(parse(r#"{"run_limit_ms": 10000}"#, base).unwrap(), defaults);
    let message = invalid(r#"{"call_deadline_ms": 2000, "run_limit_ms": 1999}"#, base);
    assert!(message.contains("run_limit_ms: must be at least call_deadline_ms"), "{message}");
    // A config that sets only a long call deadline stays valid: the
    // omitted run limit is never below it.
    assert_eq!(
        parse(r#"{"call_deadline_ms": 120000}"#, base).unwrap(),
        parse(r#"{"call_deadline_ms": 120000, "run_limit_ms": 120000}"#, base).unwrap()
    );
}

#[test]
fn unknown_keys_and_wrong_types_are_invalid() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    assert!(invalid(r#"{"stores": {}, "colour": "blue"}"#, base).contains("colour"));
    assert!(
        invalid(r#"{"stores": {"a": {"path": "a.sqlite", "mode": "x"}}}"#, base).contains("mode")
    );
    assert!(invalid(r#"{"roots": [{"path": ".", "recursive": true}]}"#, base).contains("recursive"));
    assert!(invalid(r#"{"search_mode": "vector"}"#, base).contains("search_mode"));
    invalid(r#"[]"#, base);
    invalid(r#"{"stores": {"a": {}}}"#, base);
}

#[test]
fn hybrid_default_requires_the_default_embedder() {
    let dir = tempfile::tempdir().unwrap();
    let message = invalid(r#"{"search_mode": "hybrid"}"#, dir.path());
    assert!(message.contains("search_mode"), "{message}");
}

#[test]
fn store_names_follow_the_grammar() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    for name in ["notes", "a", "A9", "a.b-c_d", &"x".repeat(64)] {
        // Well formed, so resolution gets past the grammar to the registry.
        assert_eq!(Config::default().resolve(name), Err(NameError::Unknown), "{name}");
    }
    for name in
        ["", ".", "..", "-a", "_a", ".a", "a/b", "a\\b", "/abs", "a b", "é", &"x".repeat(65)]
    {
        assert_eq!(Config::default().resolve(name), Err(NameError::BadName), "{name:?}");
    }
    let message = invalid(r#"{"stores": {"a/b": {"path": "x.sqlite"}}}"#, base);
    assert!(message.contains("a/b"), "{message}");
    invalid(r#"{"stores": {"..": {"path": "x.sqlite"}}}"#, base);
}

#[test]
fn default_store_must_be_addressable() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    assert!(invalid(r#"{"default_store": "missing"}"#, base).contains("default_store"));
    fs::write(base.join("rooted.sqlite"), b"").unwrap();
    let json = r#"{"roots": [{"path": "."}], "default_store": "rooted"}"#;
    assert_eq!(parse(json, base).unwrap().default_store.as_deref(), Some("rooted"));
}

#[test]
fn roots_must_be_directories() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    assert!(invalid(r#"{"roots": [{"path": "nope"}]}"#, base).contains("roots"));
    fs::write(base.join("file"), b"").unwrap();
    assert!(invalid(r#"{"roots": [{"path": "file"}]}"#, base).contains("roots"));
}

#[test]
fn root_files_are_addressable_by_name_without_suffix_and_without_recursion() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    fs::create_dir_all(root.join("sub")).unwrap();
    fs::write(root.join("alpha.sqlite"), b"").unwrap();
    fs::write(root.join("beta.sqlite"), b"").unwrap();
    fs::write(root.join("gamma.txt"), b"").unwrap();
    fs::write(root.join("bad name.sqlite"), b"").unwrap();
    fs::write(root.join("sub").join("delta.sqlite"), b"").unwrap();
    fs::create_dir(root.join("dir.sqlite")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("alpha.sqlite"), root.join("link.sqlite")).unwrap();
    let config = parse(r#"{"roots": [{"path": "root"}]}"#, dir.path()).unwrap();

    assert_eq!(config.addressable_names(), vec!["alpha".to_string(), "beta".to_string()]);
    assert_eq!(
        config.resolve("alpha"),
        Ok(Resolution { path: root.join("alpha.sqlite"), create: false, eager: false })
    );
    for name in ["gamma", "delta", "dir", "link", "sub"] {
        assert_eq!(config.resolve(name), Err(NameError::Unknown), "{name}");
    }
    assert_eq!(config.resolve("sub/delta"), Err(NameError::BadName));
}

#[test]
fn explicit_store_wins_over_a_root_name() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("alpha.sqlite"), b"").unwrap();
    let json = r#"{"roots": [{"path": "root"}],
                   "stores": {"alpha": {"path": "elsewhere.sqlite", "create": true}}}"#;
    let config = parse(json, dir.path()).unwrap();
    assert_eq!(
        config.resolve("alpha"),
        Ok(Resolution { path: dir.path().join("elsewhere.sqlite"), create: true, eager: false })
    );
    assert_eq!(config.addressable_names(), vec!["alpha".to_string()]);
}

#[test]
fn path_like_names_are_rejected_before_any_filesystem_access() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    fs::create_dir(&root).unwrap();
    fs::write(dir.path().join("outside.sqlite"), b"").unwrap();
    let config = parse(r#"{"roots": [{"path": "root"}]}"#, dir.path()).unwrap();
    let outside = dir.path().join("outside.sqlite");
    for name in ["../outside", "..", "/etc/passwd", outside.to_str().unwrap(), "a/b", "a\\b"] {
        assert_eq!(config.resolve(name), Err(NameError::BadName), "{name}");
    }
    let entries: Vec<_> = fs::read_dir(&root).unwrap().collect();
    assert!(entries.is_empty(), "resolution must not create files");
}

#[test]
fn load_reads_a_file_and_resolves_relative_to_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mcp.json");
    fs::write(&path, r#"{"stores": {"n": {"path": "n.sqlite", "create": true}}}"#).unwrap();
    let config = Config::load(&path).unwrap();
    assert_eq!(config.stores["n"].path, dir.path().join("n.sqlite"));
    assert!(Config::load(&dir.path().join("absent.json")).is_err());
}

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fathomdb-mcp"))
}

#[test]
fn check_config_reports_valid_on_stderr_and_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mcp.json");
    fs::write(&path, r#"{"stores": {"n": {"path": "n.sqlite", "create": true}}}"#).unwrap();
    let output = binary().arg("--config").arg(&path).arg("--check-config").output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty(), "stdout must stay empty");
    let report: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(report["valid"], true);
    assert_eq!(report["stores"], serde_json::json!(["n"]));
    assert!(!dir.path().join("n.sqlite").exists(), "check-config must not open stores");
}

#[test]
fn check_config_reports_invalid_and_exits_two() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mcp.json");
    fs::write(&path, r#"{"write": true}"#).unwrap();
    let output = binary().arg("--config").arg(&path).arg("--check-config").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let report: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(report["valid"], false);
    assert!(report["error"].as_str().unwrap().contains("write"));
}

#[test]
fn invalid_arguments_and_configs_exit_two_without_serving() {
    let dir = tempfile::tempdir().unwrap();
    let output = binary().output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let output = binary().arg("--bogus").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    let path = dir.path().join("bad.json");
    fs::write(&path, "{not json").unwrap();
    let output = binary().arg("--config").arg(&path).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}
