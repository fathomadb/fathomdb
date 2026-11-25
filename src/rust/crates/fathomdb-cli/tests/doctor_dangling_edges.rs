use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use fathomdb_cli::exit_code;
use fathomdb_engine::{
    EmbedderChoice, Engine, EngineConfig, InitialState, PreparedWrite, ScopePolicy, SourceId,
    WriteOptions,
};
use rusqlite::Connection;
use serde_json::Value;
use tempfile::TempDir;

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut value = OsString::from(path.as_os_str());
    value.push(suffix);
    PathBuf::from(value)
}

fn file_set(path: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    ["", ".lock", "-wal", "-shm", "-journal"]
        .into_iter()
        .map(|suffix| {
            let candidate = sidecar(path, suffix);
            (suffix.to_string(), fs::read(candidate).ok())
        })
        .collect()
}

fn run_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fathomdb")).args(args).output().expect("run fathomdb cli")
}

fn certify_fts_sanitation(path: &Path) {
    let output =
        run_cli(&["recover", "--accept-data-loss", "--complete-erasures", path.to_str().unwrap()]);
    assert!(
        output.status.code() == Some(exit_code::OK)
            || output.status.code() == Some(exit_code::RECOVERY_ACCEPTED_LOSS),
        "expected recover --complete-erasures to succeed: {output:#?}"
    );
}

fn parse_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "expected JSON stdout: {err}; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn open_scoped(path: &Path, policy: ScopePolicy) -> Engine {
    Engine::open_with_choice_and_config(
        path,
        EmbedderChoice::None,
        EngineConfig::default().with_scope_policy(policy),
    )
    .expect("open scoped engine")
    .engine
}

fn node(id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("test:doctor-edges").unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
    }
}

fn edge(logical_id: Option<&str>, from: &str, to: &str) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "rel".into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new("test:doctor-edges").unwrap(),
        logical_id: logical_id.map(str::to_owned),
        body: None,
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        properties: None,
    }
}

#[test]
fn clean_store_exits_zero_for_data_plane_and_check_integrity_without_mutation() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("clean-edges.sqlite");
    {
        let opened = Engine::open(&path).unwrap();
        opened
            .engine
            .write(&[node("a", "alpha body"), node("b", "beta body"), edge(Some("e1"), "a", "b")])
            .unwrap();
        opened.engine.close().unwrap();
    }
    certify_fts_sanitation(&path);

    let before = file_set(&path);
    let path_str = path.to_str().unwrap();

    // Default all 5 checks in canonical order.
    let dpi_all = run_cli(&["doctor", "data-plane-integrity", "--json", path_str]);
    assert_eq!(dpi_all.status.code(), Some(exit_code::OK), "{dpi_all:#?}");
    let dpi_all_json = parse_json(&dpi_all);
    assert_eq!(dpi_all_json["status"], "clean");
    let check_names: Vec<&str> = dpi_all_json["report"]["checkCounts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["check"].as_str().unwrap())
        .collect();
    assert_eq!(
        check_names,
        vec![
            "dependency_chain",
            "active_searchable_orphans",
            "projection_generation",
            "mutation_readiness",
            "edge_endpoint_integrity",
        ]
    );
    assert_eq!(dpi_all_json["report"]["findings"].as_array().unwrap().len(), 0);
    assert_eq!(file_set(&path), before, "data-plane-integrity must be read-only");

    // Explicit --check edge_endpoint_integrity.
    let dpi_edge = run_cli(&[
        "doctor",
        "data-plane-integrity",
        "--json",
        "--check",
        "edge_endpoint_integrity",
        path_str,
    ]);
    assert_eq!(dpi_edge.status.code(), Some(exit_code::OK), "{dpi_edge:#?}");
    let dpi_edge_json = parse_json(&dpi_edge);
    assert_eq!(dpi_edge_json["status"], "clean");
    assert_eq!(dpi_edge_json["report"]["findings"].as_array().unwrap().len(), 0);
    assert_eq!(file_set(&path), before);

    // check-integrity preserves exact 3-section report shape and exits 0.
    let ci = run_cli(&["doctor", "check-integrity", "--json", path_str]);
    assert_eq!(ci.status.code(), Some(exit_code::OK), "{ci:#?}");
    let ci_json = parse_json(&ci);
    assert_eq!(ci_json["physical"]["status"], "clean");
    assert_eq!(ci_json["logical"]["status"], "clean");
    assert_eq!(ci_json["semantic"]["status"], "clean");
}

#[test]
fn bulk_ingested_dangling_edges_exit_65_and_clear_after_missing_nodes_arrive() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("dangling-edges.sqlite");
    let secret_missing_id = "secret-missing-logical-id-999";
    let edge_cursor: u64;
    {
        let opened = Engine::open(&path).unwrap();
        opened.engine.write(&[node("n1", "present node payload")]).unwrap();
        // Write a dangling edge using BulkNonStrict (1 missing endpoint).
        let receipt = opened
            .engine
            .write_with_options(
                &[edge(Some("e-dangling"), "n1", secret_missing_id)],
                &WriteOptions::bulk_non_strict(),
            )
            .unwrap();
        assert_eq!(receipt.dangling_edge_endpoints, 1);
        edge_cursor = receipt.cursor;
        opened.engine.close().unwrap();
    }
    certify_fts_sanitation(&path);

    let before = file_set(&path);
    let path_str = path.to_str().unwrap();

    // 1. data-plane-integrity --json exits 65 with deterministic dangling_edge_endpoint finding.
    let dpi = run_cli(&[
        "doctor",
        "data-plane-integrity",
        "--json",
        "--check",
        "edge_endpoint_integrity",
        path_str,
    ]);
    assert_eq!(dpi.status.code(), Some(exit_code::DOCTOR_FOUND_ISSUES), "{dpi:#?}");
    let dpi_stdout = String::from_utf8_lossy(&dpi.stdout);
    assert!(
        !dpi_stdout.contains(secret_missing_id),
        "data-plane-integrity must not leak logical_id: {dpi_stdout}"
    );
    let dpi_json = parse_json(&dpi);
    assert_eq!(dpi_json["status"], "findings");
    let findings = dpi_json["report"]["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "expected exactly 1 dangling finding: {findings:?}");
    assert_eq!(findings[0]["code"], "dangling_edge_endpoint");
    assert_eq!(findings[0]["severity"], "error");
    assert_eq!(findings[0]["writeCursor"], edge_cursor.to_string());
    assert_eq!(findings[0]["artifactRevisionIds"], serde_json::json!([]));
    assert_eq!(file_set(&path), before, "data-plane-integrity must not mutate DB");

    // 2. check-integrity --json exits 65 with E_DANGLING_EDGE_ENDPOINT in semantic section.
    let ci = run_cli(&["doctor", "check-integrity", "--json", path_str]);
    assert_eq!(ci.status.code(), Some(exit_code::DOCTOR_FOUND_ISSUES), "{ci:#?}");
    let ci_stdout = String::from_utf8_lossy(&ci.stdout);
    assert!(
        !ci_stdout.contains(secret_missing_id),
        "check-integrity must not leak logical_id: {ci_stdout}"
    );
    let ci_json = parse_json(&ci);
    assert_eq!(ci_json["physical"]["status"], "clean");
    assert_eq!(ci_json["logical"]["status"], "clean");
    assert_eq!(ci_json["semantic"]["status"], "findings");
    let sem_findings = ci_json["semantic"]["findings"].as_array().unwrap();
    assert_eq!(sem_findings.len(), 1, "{sem_findings:?}");
    assert_eq!(sem_findings[0]["code"], "E_DANGLING_EDGE_ENDPOINT");
    assert_eq!(sem_findings[0]["doc_anchor"], "design/recovery.md#e-dangling-edge-endpoint");

    // 3. Ingest the missing node in a second batch; both doctor commands now exit 0!
    {
        let opened = Engine::open(&path).unwrap();
        opened.engine.write(&[node(secret_missing_id, "missing node arrived")]).unwrap();
        opened.engine.close().unwrap();
    }
    certify_fts_sanitation(&path);

    let dpi_after = run_cli(&["doctor", "data-plane-integrity", "--json", path_str]);
    assert_eq!(dpi_after.status.code(), Some(exit_code::OK), "{dpi_after:#?}");
    assert_eq!(parse_json(&dpi_after)["status"], "clean");

    let ci_after = run_cli(&["doctor", "check-integrity", "--json", path_str]);
    assert_eq!(ci_after.status.code(), Some(exit_code::OK), "{ci_after:#?}");
    let ci_after_json = parse_json(&ci_after);
    assert_eq!(ci_after_json["semantic"]["status"], "clean");
}

#[test]
fn raw_sql_cross_scope_edge_anomaly_exits_65_with_cross_scope_finding_and_no_leakage() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("cross-scope-anomaly.sqlite");
    let alpha_secret_node = "alpha-only-secret-node";
    let beta_secret_node = "beta-only-secret-node";
    {
        let alpha = open_scoped(&path, ScopePolicy::single("scope-alpha-secret"));
        alpha.write(&[node(alpha_secret_node, "alpha payload secret")]).unwrap();
        alpha.close().unwrap();
    }
    {
        let beta = open_scoped(&path, ScopePolicy::single("scope-beta-secret"));
        beta.write(&[node(beta_secret_node, "beta payload secret")]).unwrap();
        beta.close().unwrap();
    }
    certify_fts_sanitation(&path);

    // Inject a cross-scope active edge directly via raw SQL (bypassing Engine write guards).
    let injected_cursor: i64 = 999;
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute(
            "INSERT INTO canonical_edges(write_cursor, kind, from_id, to_id, source_id, scope)
             VALUES(?1, 'rel', ?2, ?3, 'raw:injected', 'scope-alpha-secret')",
            rusqlite::params![injected_cursor, alpha_secret_node, beta_secret_node],
        )
        .unwrap();
    }

    let path_str = path.to_str().unwrap();

    // 1. data-plane-integrity --json reports CrossScopeEdgeEndpoint (critical) without double-counting.
    let dpi = run_cli(&[
        "doctor",
        "data-plane-integrity",
        "--json",
        "--check",
        "edge_endpoint_integrity",
        path_str,
    ]);
    assert_eq!(dpi.status.code(), Some(exit_code::DOCTOR_FOUND_ISSUES), "{dpi:#?}");
    let dpi_stdout = String::from_utf8_lossy(&dpi.stdout);
    for forbidden in [
        alpha_secret_node,
        beta_secret_node,
        "scope-alpha-secret",
        "scope-beta-secret",
        "alpha payload secret",
        "beta payload secret",
    ] {
        assert!(
            !dpi_stdout.contains(forbidden),
            "data-plane-integrity leaked {forbidden:?}: {dpi_stdout}"
        );
    }
    let dpi_json = parse_json(&dpi);
    assert_eq!(dpi_json["status"], "findings");
    let findings = dpi_json["report"]["findings"].as_array().unwrap();
    assert_eq!(
        findings.len(),
        1,
        "must classify as cross-scope only, no double-count: {findings:?}"
    );
    assert_eq!(findings[0]["code"], "cross_scope_edge_endpoint");
    assert_eq!(findings[0]["severity"], "critical");
    assert_eq!(findings[0]["writeCursor"], injected_cursor.to_string());
    assert_eq!(findings[0]["artifactRevisionIds"], serde_json::json!([]));

    // 2. check-integrity --json reports E_CROSS_SCOPE_EDGE_ENDPOINT in semantic section without leakage.
    let ci = run_cli(&["doctor", "check-integrity", "--json", path_str]);
    assert_eq!(ci.status.code(), Some(exit_code::DOCTOR_FOUND_ISSUES), "{ci:#?}");
    let ci_stdout = String::from_utf8_lossy(&ci.stdout);
    for forbidden in [
        alpha_secret_node,
        beta_secret_node,
        "scope-alpha-secret",
        "scope-beta-secret",
        "alpha payload secret",
        "beta payload secret",
    ] {
        assert!(
            !ci_stdout.contains(forbidden),
            "check-integrity leaked {forbidden:?}: {ci_stdout}"
        );
    }
    let ci_json = parse_json(&ci);
    assert_eq!(ci_json["physical"]["status"], "clean");
    assert_eq!(ci_json["logical"]["status"], "clean");
    assert_eq!(ci_json["semantic"]["status"], "findings");
    let sem_findings = ci_json["semantic"]["findings"].as_array().unwrap();
    assert_eq!(
        sem_findings.len(),
        1,
        "must not double-count cross-scope as dangling: {sem_findings:?}"
    );
    assert_eq!(sem_findings[0]["code"], "E_CROSS_SCOPE_EDGE_ENDPOINT");
    assert_eq!(sem_findings[0]["doc_anchor"], "design/recovery.md#e-cross-scope-edge-endpoint");
}

#[test]
fn superseded_dangling_edge_is_ignored_by_edge_endpoint_checks() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("superseded-dangling.sqlite");
    {
        let opened = Engine::open(&path).unwrap();
        opened.engine.write(&[node("n1", "anchor node")]).unwrap();
        let receipt = opened
            .engine
            .write_with_options(
                &[
                    edge(Some("e-sup"), "ghost-sup-a", "ghost-sup-b"),
                    edge(Some("e-sup"), "n1", "n1"),
                ],
                &WriteOptions::bulk_non_strict(),
            )
            .unwrap();
        assert_eq!(receipt.dangling_edge_endpoints, 0);
        opened.engine.close().unwrap();
    }
    certify_fts_sanitation(&path);

    let path_str = path.to_str().unwrap();
    let dpi = run_cli(&[
        "doctor",
        "data-plane-integrity",
        "--json",
        "--check",
        "edge_endpoint_integrity",
        path_str,
    ]);
    assert_eq!(dpi.status.code(), Some(exit_code::OK), "{dpi:#?}");
    assert_eq!(parse_json(&dpi)["status"], "clean");

    let ci = run_cli(&["doctor", "check-integrity", "--json", path_str]);
    assert_eq!(ci.status.code(), Some(exit_code::OK), "{ci:#?}");
    let ci_json = parse_json(&ci);
    assert_eq!(ci_json["semantic"]["status"], "clean");
}

#[test]
fn check_integrity_admits_superseded_only_active_endpoint_but_flags_inactive_current_revisions() {
    let dir = TempDir::new().unwrap();

    // 1. Genuine superseded-only same-scope active node (no current `superseded_at IS NULL` row):
    //    `check-integrity` exits 0 (matching `fathomdb migrate`), while strict
    //    `data-plane-integrity --check edge_endpoint_integrity` exits 65 (`dangling_edge_endpoint`).
    let sup_only_path = dir.path().join("superseded-only.sqlite");
    {
        let opened = Engine::open(&sup_only_path).unwrap();
        opened
            .engine
            .write(&[
                node("n-src", "source"),
                node("n-hist", "historical"),
                edge(Some("e1"), "n-src", "n-hist"),
            ])
            .unwrap();
        opened.engine.close().unwrap();
    }
    {
        let conn = Connection::open(&sup_only_path).unwrap();
        conn.execute(
            "UPDATE canonical_nodes SET superseded_at = 99 \
             WHERE logical_id = 'n-hist' AND superseded_at IS NULL",
            [],
        )
        .unwrap();
    }
    certify_fts_sanitation(&sup_only_path);
    let sup_str = sup_only_path.to_str().unwrap();

    let ci_sup = run_cli(&["doctor", "check-integrity", "--json", sup_str]);
    assert_eq!(ci_sup.status.code(), Some(exit_code::OK), "{ci_sup:#?}");
    assert_eq!(parse_json(&ci_sup)["semantic"]["status"], "clean");

    let dpi_sup = run_cli(&[
        "doctor",
        "data-plane-integrity",
        "--json",
        "--check",
        "edge_endpoint_integrity",
        sup_str,
    ]);
    assert_eq!(dpi_sup.status.code(), Some(exit_code::DOCTOR_FOUND_ISSUES), "{dpi_sup:#?}");
    let dpi_findings = parse_json(&dpi_sup)["report"]["findings"].as_array().unwrap().clone();
    assert_eq!(dpi_findings.len(), 1, "{dpi_findings:?}");
    assert_eq!(dpi_findings[0]["code"], "dangling_edge_endpoint");

    // 2. Earlier `'active'` revision superseded by a current `pending` row AND another
    //    superseded by a soft-`deleted` row: `check-integrity` MUST flag both with exit 65
    //    (`E_DANGLING_EDGE_ENDPOINT`), never masked by the older superseded `'active'` revision.
    let pending_path = dir.path().join("superseded-by-pending.sqlite");
    {
        let opened = Engine::open(&pending_path).unwrap();
        opened
            .engine
            .write(&[
                node("n-src", "source"),
                node("n-pend", "v1 active"),
                edge(Some("e1"), "n-src", "n-pend"),
            ])
            .unwrap();
        opened
            .engine
            .write(&[PreparedWrite::Node {
                kind: "doc".into(),
                body: "v2 pending".into(),
                source_id: SourceId::new("test:doctor-edges").unwrap(),
                logical_id: Some("n-pend".into()),
                state: InitialState::Pending,
                reason: Some("quarantine".into()),
                valid_from: None,
                valid_until: None,
                properties: None,
            }])
            .unwrap();
        opened.engine.close().unwrap();
    }
    certify_fts_sanitation(&pending_path);
    let ci_pend = run_cli(&["doctor", "check-integrity", "--json", pending_path.to_str().unwrap()]);
    assert_eq!(ci_pend.status.code(), Some(exit_code::DOCTOR_FOUND_ISSUES), "{ci_pend:#?}");
    let pend_findings = parse_json(&ci_pend)["semantic"]["findings"].as_array().unwrap().clone();
    assert_eq!(pend_findings.len(), 1, "{pend_findings:?}");
    assert_eq!(pend_findings[0]["code"], "E_DANGLING_EDGE_ENDPOINT");

    let deleted_path = dir.path().join("superseded-by-deleted.sqlite");
    {
        let opened = Engine::open(&deleted_path).unwrap();
        opened
            .engine
            .write(&[
                node("n-src", "source"),
                node("n-del", "v1 active"),
                edge(Some("e1"), "n-src", "n-del"),
            ])
            .unwrap();
        opened.engine.write(&[node("n-del", "v2 active")]).unwrap();
        opened
            .engine
            .transition("n-del", fathomdb_engine::LifecycleState::Deleted, Some("retire".into()))
            .unwrap();
        opened.engine.close().unwrap();
    }
    certify_fts_sanitation(&deleted_path);
    let ci_del = run_cli(&["doctor", "check-integrity", "--json", deleted_path.to_str().unwrap()]);
    assert_eq!(ci_del.status.code(), Some(exit_code::DOCTOR_FOUND_ISSUES), "{ci_del:#?}");
    let del_findings = parse_json(&ci_del)["semantic"]["findings"].as_array().unwrap().clone();
    assert_eq!(del_findings.len(), 1, "{del_findings:?}");
    assert_eq!(del_findings[0]["code"], "E_DANGLING_EDGE_ENDPOINT");
}
