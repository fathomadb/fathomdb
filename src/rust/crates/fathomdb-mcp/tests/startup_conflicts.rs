//! Fail-fast startup conflict matrix (`SC-01`..`SC-10`, ADR-0.8.32 #99).
//!
//! Every rule is verified identically across:
//! - `fathomdb-mcp --config <path.json> --check-config` (exit 2, empty stdout,
//!   structured JSON error on stderr, zero `.sqlite`/`.lock` files created),
//! - `Config::from_json_str` / `Config::load` / `Config::validate`,
//! - `Adapter::new`, and
//! - `Adapter::register_engine` / `Adapter::register_engine_with_policy`.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use fathomdb_mcp::config::{Config, StoreEntry};
use fathomdb_mcp::{Adapter, AdapterError, HostedStorePolicy};
use fathomdb_sdk::{Engine, OpenOptions, ScopePolicy};
use serde_json::{json, Value};

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fathomdb-mcp"))
}

/// Assert that `config_json` fails identically in `Config::from_json_str`,
/// `Config::load`, and `--check-config` (exit 2, empty stdout, JSON stderr,
/// zero `.sqlite`/`.lock` created).
fn assert_startup_conflict(dir: &Path, config_json: &Value, expected_tokens: &[&str]) {
    let text = config_json.to_string();
    let parsed = Config::from_json_str(&text, dir);
    let parse_err = parsed
        .expect_err(&format!("expected Config::from_json_str conflict for {text}"))
        .to_string();

    let cfg_path = dir.join("conflict.json");
    fs::write(&cfg_path, &text).unwrap();
    let loaded = Config::load(&cfg_path);
    let load_err =
        loaded.expect_err(&format!("expected Config::load conflict for {text}")).to_string();
    assert_eq!(parse_err, load_err, "Config::load and from_json_str must match");

    let before_files: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();

    let output = binary().arg("--config").arg(&cfg_path).arg("--check-config").output().unwrap();
    assert_eq!(output.status.code(), Some(2), "--check-config must exit 2 for {text}");
    assert!(output.stdout.is_empty(), "stdout must stay empty on --check-config error");
    let report: Value = serde_json::from_slice(&output.stderr).unwrap_or_else(|e| {
        panic!("stderr must be valid JSON: {e}; raw={:?}", String::from_utf8_lossy(&output.stderr))
    });
    assert_eq!(report["valid"], false);
    let cli_err = report["error"].as_str().expect("error string").to_string();
    assert_eq!(cli_err, parse_err, "--check-config error must match Config::from_json_str");

    for token in expected_tokens {
        assert!(parse_err.contains(token), "error {parse_err:?} must name {token:?} for {text}");
    }

    let after_files: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    for file in &after_files {
        if !before_files.contains(file) {
            assert!(
                !file.ends_with(".sqlite") && !file.ends_with(".lock"),
                "--check-config must never create db/lock files; created {file}"
            );
        }
    }
}

#[test]
fn sc01_allow_writes_requires_non_empty_bound_scope() {
    let dir = tempfile::tempdir().unwrap();
    // Omitted scope
    assert_startup_conflict(
        dir.path(),
        &json!({"stores": {"a": {"path": "a.sqlite", "allow_writes": true}}}),
        &["scope"],
    );
    // Explicit null scope
    assert_startup_conflict(
        dir.path(),
        &json!({"stores": {"a": {"path": "a.sqlite", "allow_writes": true, "scope": null}}}),
        &["scope"],
    );
    // Empty string scope
    assert_startup_conflict(
        dir.path(),
        &json!({"stores": {"a": {"path": "a.sqlite", "allow_writes": true, "scope": ""}}}),
        &["scope"],
    );
    // Top-level allow_writes: true without any writable store
    assert_startup_conflict(
        dir.path(),
        &json!({"allow_writes": true, "stores": {"a": {"path": "a.sqlite"}}}),
        &["allow_writes"],
    );
}

#[test]
fn sc02_allow_writes_conflicts_with_read_only_scope_grant() {
    let dir = tempfile::tempdir().unwrap();
    assert_startup_conflict(
        dir.path(),
        &json!({
            "stores": {
                "a": {
                    "path": "a.sqlite",
                    "allow_writes": true,
                    "scope": {"read": ["alpha"]}
                }
            }
        }),
        &["scope"],
    );
    assert_startup_conflict(
        dir.path(),
        &json!({
            "stores": {
                "a": {
                    "path": "a.sqlite",
                    "allow_writes": true,
                    "scope": {"read": ["alpha"], "write": null}
                }
            }
        }),
        &["scope"],
    );
}

#[test]
fn sc03_read_only_store_rejects_write_grants_allowed_kinds_and_create_true() {
    let dir = tempfile::tempdir().unwrap();
    // create: true on read-only store
    assert_startup_conflict(
        dir.path(),
        &json!({"stores": {"a": {"path": "a.sqlite", "create": true}}}),
        &["create"],
    );
    // write grant in scope on read-only store
    assert_startup_conflict(
        dir.path(),
        &json!({
            "stores": {
                "a": {
                    "path": "a.sqlite",
                    "allow_writes": false,
                    "scope": {"read": ["alpha"], "write": "alpha"}
                }
            }
        }),
        &["scope"],
    );
    // allowed_kinds on read-only store
    assert_startup_conflict(
        dir.path(),
        &json!({
            "stores": {
                "a": {
                    "path": "a.sqlite",
                    "allowed_kinds": ["note"]
                }
            }
        }),
        &["allowed_kinds"],
    );
    // allowed_node_kinds / allowed_edge_kinds / allowed_collections on read-only store
    assert_startup_conflict(
        dir.path(),
        &json!({"stores": {"a": {"path": "a.sqlite", "allowed_node_kinds": ["note"]}}}),
        &["allowed_node_kinds"],
    );
    assert_startup_conflict(
        dir.path(),
        &json!({"stores": {"a": {"path": "a.sqlite", "allowed_edge_kinds": ["rel"]}}}),
        &["allowed_edge_kinds"],
    );
    assert_startup_conflict(
        dir.path(),
        &json!({"stores": {"a": {"path": "a.sqlite", "allowed_collections": ["ckpt"]}}}),
        &["allowed_collections"],
    );
}

#[test]
fn sc04_legacy_write_key_is_rejected_naming_offending_keys() {
    let dir = tempfile::tempdir().unwrap();
    assert_startup_conflict(dir.path(), &json!({"write": true}), &["write"]);
    assert_startup_conflict(dir.path(), &json!({"write": false}), &["write"]);
    assert_startup_conflict(
        dir.path(),
        &json!({"write": false, "allow_writes": false}),
        &["write", "allow_writes"],
    );
    assert_startup_conflict(
        dir.path(),
        &json!({"stores": {"a": {"path": "a.sqlite", "write": true}}}),
        &["write"],
    );
}

#[test]
fn sc05_roots_auto_discovery_rejects_write_enablement() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    fs::create_dir(&root).unwrap();
    assert_startup_conflict(
        dir.path(),
        &json!({"allow_writes": true, "roots": [{"path": "root"}]}),
        &["roots"],
    );
    assert_startup_conflict(
        dir.path(),
        &json!({"roots": [{"path": "root", "allow_writes": true}]}),
        &["roots"],
    );
}

#[test]
fn sc06_duplicate_canonical_path_aliases_are_rejected_for_writes_or_incompatible_policy() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("shared.sqlite");
    fs::write(&db, b"").unwrap();
    let sub = dir.path().join("sub");
    fs::create_dir(&sub).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&db, sub.join("alias.sqlite")).unwrap();

    // Two writable aliases to same existing canonical file (via .. traversal)
    assert_startup_conflict(
        dir.path(),
        &json!({
            "stores": {
                "a": {"path": "shared.sqlite", "allow_writes": true, "scope": "alpha"},
                "b": {"path": "sub/../shared.sqlite", "allow_writes": true, "scope": "alpha"}
            }
        }),
        &["stores"],
    );

    // One writable, one read-only alias to same canonical file (cross-scope/write bypass)
    assert_startup_conflict(
        dir.path(),
        &json!({
            "stores": {
                "a": {"path": "shared.sqlite", "allow_writes": true, "scope": "alpha"},
                "b": {"path": "sub/../shared.sqlite", "allow_writes": false, "scope": "beta"}
            }
        }),
        &["stores"],
    );

    // Not-yet-created `create: true` targets resolving via parent canonicalization
    assert_startup_conflict(
        dir.path(),
        &json!({
            "stores": {
                "a": {"path": "new.sqlite", "allow_writes": true, "scope": "alpha", "create": true},
                "b": {"path": "sub/../new.sqlite", "allow_writes": true, "scope": "alpha", "create": true}
            }
        }),
        &["stores"],
    );

    // Two read-only aliases with incompatible scope policies (via symlink on unix)
    #[cfg(unix)]
    assert_startup_conflict(
        dir.path(),
        &json!({
            "stores": {
                "a": {"path": "shared.sqlite", "scope": "alpha"},
                "b": {"path": "sub/alias.sqlite", "scope": "beta"}
            }
        }),
        &["stores"],
    );

    // Two read-only aliases with incompatible eager settings
    assert_startup_conflict(
        dir.path(),
        &json!({
            "stores": {
                "a": {"path": "shared.sqlite", "eager": true},
                "b": {"path": "sub/../shared.sqlite", "eager": false}
            }
        }),
        &["stores"],
    );
}

#[test]
fn sc07_multiple_eager_stores_conflict_with_max_open_stores_1() {
    let dir = tempfile::tempdir().unwrap();
    assert_startup_conflict(
        dir.path(),
        &json!({
            "stores": {
                "a": {"path": "a.sqlite", "eager": true},
                "b": {"path": "b.sqlite", "eager": true}
            }
        }),
        &["eager"],
    );
}

#[test]
fn sc08_malformed_or_contradictory_scope_policy_grammar_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let cases = [
        // Wildcard on string shorthand
        json!({"stores": {"a": {"path": "a.sqlite", "scope": "*"}}}),
        // Wildcard on write
        json!({"stores": {"a": {"path": "a.sqlite", "allow_writes": true, "scope": {"read": ["*"], "write": "*"}}}}),
        // Empty read list
        json!({"stores": {"a": {"path": "a.sqlite", "scope": {"read": []}}}}),
        // Duplicate read tokens
        json!({"stores": {"a": {"path": "a.sqlite", "scope": {"read": ["alpha", "alpha"]}}}}),
        // Write scope not in read scopes
        json!({"stores": {"a": {"path": "a.sqlite", "allow_writes": true, "scope": {"read": ["alpha"], "write": "beta"}}}}),
        // Control character / NUL
        json!({"stores": {"a": {"path": "a.sqlite", "scope": "bad\u{0000}scope"}}}),
        json!({"stores": {"a": {"path": "a.sqlite", "scope": "bad\x1escope"}}}),
        // Leading/trailing whitespace
        json!({"stores": {"a": {"path": "a.sqlite", "scope": " alpha "}}}),
        // Overlong token (> 128 bytes)
        json!({"stores": {"a": {"path": "a.sqlite", "scope": "x".repeat(129)}}}),
        // Unknown key inside structured scope
        json!({"stores": {"a": {"path": "a.sqlite", "scope": {"read": ["alpha"], "deny": ["beta"]}}}}),
    ];
    for case in &cases {
        assert_startup_conflict(dir.path(), case, &["scope"]);
    }
}

#[test]
fn sc09_cross_field_invariants_fail_fast() {
    let dir = tempfile::tempdir().unwrap();
    assert_startup_conflict(dir.path(), &json!({"default_store": "missing"}), &["default_store"]);
    assert_startup_conflict(
        dir.path(),
        &json!({"search_mode": "hybrid", "use_default_embedder": false}),
        &["search_mode"],
    );
    assert_startup_conflict(
        dir.path(),
        &json!({"call_deadline_ms": 2000, "run_limit_ms": 1999}),
        &["run_limit_ms"],
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn sc10_embedded_register_engine_rejects_policy_conflicts_leaving_engine_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let db_ro = dir.path().join("ro.sqlite");
    let db_rw = dir.path().join("rw.sqlite");
    let db_unscoped = dir.path().join("unscoped.sqlite");

    let eng_ro = Arc::new(
        Engine::open(
            &db_ro,
            OpenOptions::default().with_scope_policy(ScopePolicy::read_only(["alpha"])),
        )
        .unwrap(),
    );
    let eng_rw = Arc::new(
        Engine::open(
            &db_rw,
            OpenOptions::default().with_scope_policy(ScopePolicy::single("alpha")),
        )
        .unwrap(),
    );
    let eng_unscoped = Arc::new(Engine::open(&db_unscoped, OpenOptions::default()).unwrap());

    let telemetry = dir.path().join("telemetry");
    let mut cfg = Config { telemetry_dir: Some(telemetry.clone()), ..Config::default() };
    cfg.stores.insert(
        "configured".to_string(),
        StoreEntry {
            path: dir.path().join("configured.sqlite"),
            create: false,
            eager: false,
            allow_writes: false,
            scope: None,
            allowed_kinds: None,
            allowed_node_kinds: None,
            allowed_edge_kinds: None,
            allowed_collections: None,
        },
    );
    let adapter = Adapter::new(cfg).unwrap();

    // Colliding with configured filesystem stores[name] -> PolicyConflict
    let err = adapter
        .register_engine_with_policy(
            "configured",
            eng_unscoped.clone(),
            HostedStorePolicy::default(),
        )
        .unwrap_err();
    assert!(matches!(err, AdapterError::PolicyConflict(_)), "expected PolicyConflict, got {err:?}");

    // Read-only host engine registered with allow_writes: true -> PolicyConflict
    let err = adapter
        .register_engine_with_policy(
            "bad_ro_rw",
            eng_ro.clone(),
            HostedStorePolicy {
                allow_writes: true,
                scope: Some(ScopePolicy::single("alpha")),
                ..HostedStorePolicy::default()
            },
        )
        .unwrap_err();
    assert!(matches!(err, AdapterError::PolicyConflict(_)), "got {err:?}");

    // Host engine bound to "alpha", policy asks for "beta" -> PolicyConflict
    let err = adapter
        .register_engine_with_policy(
            "bad_scope",
            eng_rw.clone(),
            HostedStorePolicy {
                allow_writes: true,
                scope: Some(ScopePolicy::single("beta")),
                ..HostedStorePolicy::default()
            },
        )
        .unwrap_err();
    assert!(matches!(err, AdapterError::PolicyConflict(_)), "got {err:?}");

    // Unscoped host engine registered with allow_writes: true -> PolicyConflict
    let err = adapter
        .register_engine_with_policy(
            "bad_unscoped",
            eng_unscoped.clone(),
            HostedStorePolicy {
                allow_writes: true,
                scope: Some(ScopePolicy::single("alpha")),
                ..HostedStorePolicy::default()
            },
        )
        .unwrap_err();
    assert!(matches!(err, AdapterError::PolicyConflict(_)), "got {err:?}");

    // Verify rejected registrations left engines completely untouched (no telemetry created)
    assert!(!telemetry.exists(), "rejected registrations must not touch hosted engines");

    // Matching write policy succeeds!
    adapter
        .register_engine_with_policy(
            "ok_rw",
            eng_rw.clone(),
            HostedStorePolicy {
                allow_writes: true,
                scope: Some(ScopePolicy::single("alpha")),
                allowed_kinds: Some(vec!["note".to_string()]),
                ..HostedStorePolicy::default()
            },
        )
        .unwrap();

    adapter.shutdown().await;
    eng_ro.close().unwrap();
    eng_rw.close().unwrap();
    eng_unscoped.close().unwrap();
}

#[test]
fn valid_read_only_and_read_write_scoped_configs_parse_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = Config::from_json_str(
        &json!({
            "stores": {
                "ro_unscoped": {"path": "ro1.sqlite"},
                "ro_scoped": {"path": "ro2.sqlite", "scope": "alpha"},
                "rw_short": {
                    "path": "rw1.sqlite",
                    "allow_writes": true,
                    "scope": "alpha",
                    "create": true,
                    "allowed_kinds": ["note", "rel"],
                    "allowed_collections": ["checkpoints"]
                },
                "rw_struct": {
                    "path": "rw2.sqlite",
                    "allow_writes": true,
                    "scope": {"read": ["alpha", "shared"], "write": "alpha"},
                    "allowed_node_kinds": ["note"],
                    "allowed_edge_kinds": ["links"]
                }
            }
        })
        .to_string(),
        dir.path(),
    )
    .unwrap();
    assert!(!cfg.stores["ro_unscoped"].allow_writes);
    assert_eq!(cfg.stores["ro_unscoped"].scope, None);
    assert_eq!(cfg.stores["ro_scoped"].scope, Some(ScopePolicy::read_only(["alpha"])));
    assert!(cfg.stores["rw_short"].allow_writes);
    assert_eq!(cfg.stores["rw_short"].scope, Some(ScopePolicy::read_write(["alpha"], "alpha")));
    assert_eq!(
        cfg.stores["rw_struct"].scope,
        Some(ScopePolicy::read_write(["alpha", "shared"], "alpha"))
    );
}
