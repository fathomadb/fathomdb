//! Write-safety contract (`AC32-MCP-SAFETY-01`..`09`, `AC32-LEAK-01`, ADR-0.8.32 #99).
//!
//! Tests real database files, real rmcp connections, and real subprocesses.

mod common;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use common::{node, Fixture, Harness, RawClient, Spawned};
use fathomdb_mcp::{Adapter, HostedStorePolicy};
use fathomdb_sdk::{read, Engine, OpenOptions, PreparedWrite, ScopePolicy};
use serde_json::{json, Value};

fn seed_scoped_with_collection(path: &Path, scope: &str, collection: &str) {
    let engine =
        Engine::open(path, OpenOptions::default().with_scope_policy(ScopePolicy::single(scope)))
            .unwrap();
    engine
        .write(&[PreparedWrite::AdminSchema {
            name: collection.to_string(),
            kind: "latest_state".to_string(),
            schema_json:
                r#"{"type":"object","properties":{"step":{"type":"integer"}},"required":["step"]}"#
                    .to_string(),
            retention_json: "{}".to_string(),
        }])
        .unwrap();
    engine.close().unwrap();
}

fn canonical_node_op(id: &str, kind: &str, body: &str, rev: &str, src_ver: &str) -> Value {
    json!({
        "op": "put_canonical_node",
        "kind": kind,
        "body": body,
        "source_id": "mcp-test-src",
        "logical_id": id,
        "artifact_revision_id": rev,
        "source_version_id": src_ver
    })
}

fn derived_edge_op(
    kind: &str,
    from: &str,
    to: &str,
    rev: &str,
    src_ver: &str,
    src_rev: &str,
    hash: &str,
) -> Value {
    json!({
        "op": "put_derived_edge",
        "kind": kind,
        "from": from,
        "to": to,
        "source_id": "mcp-test-src",
        "artifact_revision_id": rev,
        "source_version_id": src_ver,
        "source_revision_id": src_rev,
        "canonical_source_hash": hash
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// AC32-MCP-SAFETY-01: Read-only default omits write tools from `tools/list` and refuses
/// direct calls with `writes_disabled` and zero DB mutation; write-enabled exposes 9 tools
/// while keeping read-only peer and `roots` stores strictly read-only; concurrent writes
/// serialize cleanly without SQLite busy-lock races.
#[tokio::test(flavor = "multi_thread")]
async fn ac32_mcp_safety_01_read_only_default_and_per_store_write_serialization() {
    let fixture = Fixture::new();
    fixture.store("ro.sqlite", vec![node("n1", "note", "initial read-only note")]);

    // 1. Read-only default profile: exact 7 tools; direct write calls fail `writes_disabled`
    let ro_harness = Harness::start(fixture.config(json!({
        "stores": {"ro": {"path": "ro.sqlite"}}
    })))
    .await;
    let ro_tools: Vec<String> =
        ro_harness.tools().await.into_iter().map(|t| t.name.into_owned()).collect();
    assert_eq!(ro_tools.len(), 7, "read-only profile must list exact 7 tools: {ro_tools:?}");
    assert!(!ro_tools.contains(&"actuate".to_string()));
    assert!(!ro_tools.contains(&"put_latest_state".to_string()));

    let mut raw = RawClient::connect(&ro_harness.adapter).await;
    for (tool, args) in [
        (
            "actuate",
            json!({
                "store": "ro",
                "operation_id": "op-ro-1",
                "operations": [canonical_node_op("n2", "note", "should fail", "rev-ro-1", "sv-1")]
            }),
        ),
        (
            "put_latest_state",
            json!({
                "store": "ro",
                "operation_id": "op-ro-2",
                "collection": "ckpt",
                "record_key": "k1",
                "payload_json": r#"{"step":1}"#
            }),
        ),
    ] {
        let resp = raw.request("tools/call", json!({"name": tool, "arguments": args})).await;
        let res = &resp["result"];
        assert_eq!(res["isError"], true, "{tool} must fail on read-only profile: {resp}");
        assert_eq!(res["structuredContent"]["error"]["class"], "invalid_request");
        assert_eq!(res["structuredContent"]["error"]["code"], "writes_disabled");
    }
    ro_harness.shutdown().await;

    // 2. Write-enabled profile alongside a read-only store AND a read-only `roots` store
    let root_dir = fixture.path("roots_dir");
    std::fs::create_dir(&root_dir).unwrap();
    fixture.store("roots_dir/rooted.sqlite", vec![node("r1", "note", "rooted read-only")]);
    seed_scoped_with_collection(&fixture.path("rw.sqlite"), "alpha", "ckpt");

    let rw_harness = Harness::start(fixture.config(json!({
        "stores": {
            "ro": {"path": "ro.sqlite"},
            "rw": {"path": "rw.sqlite", "allow_writes": true, "scope": "alpha"}
        },
        "roots": [{"path": "roots_dir"}],
        "admission_cap": 4
    })))
    .await;
    let mut rw_tools: Vec<String> =
        rw_harness.tools().await.into_iter().map(|t| t.name.into_owned()).collect();
    rw_tools.sort();
    assert_eq!(
        rw_tools,
        vec![
            "actuate",
            "expand",
            "feedback",
            "fetch",
            "list_stores",
            "orient",
            "put_latest_state",
            "search",
            "use_store"
        ]
    );
    // Instructions on write-enabled profile must name every listed tool and stay <= 1200 bytes.
    let info = rw_harness.client.peer_info().unwrap();
    let instr = info.instructions.as_deref().unwrap();
    assert!(instr.len() <= 1200, "write instructions {} > 1200 bytes", instr.len());
    for t in &rw_tools {
        assert!(instr.contains(t), "write instructions must name {t}: {instr}");
    }

    // Read-only explicit store and root-discovered store still refuse writes with `writes_disabled`
    for target in ["ro", "rooted"] {
        let err = rw_harness
            .err(
                "actuate",
                json!({
                    "store": target,
                    "operation_id": format!("op-deny-{target}"),
                    "operations": [canonical_node_op("denied", "note", "no", "rev-deny", "sv-1")]
                }),
            )
            .await;
        assert_eq!(err["class"], "invalid_request");
        assert_eq!(err["code"], "writes_disabled");
    }

    // Concurrent MCP writes serialize deterministically without busy/lock failures
    let mut tasks = Vec::new();
    for idx in 0..8 {
        let peer = rw_harness.client.peer().clone();
        tasks.push(tokio::spawn(async move {
            let args = json!({
                "store": "rw",
                "operation_id": format!("op-concurrent-{idx}"),
                "collection": "ckpt",
                "record_key": format!("k-{idx}"),
                "payload_json": format!(r#"{{"step":{idx}}}"#)
            });
            let resp = peer
                .call_tool_once(
                    rmcp::model::CallToolRequestParams::new("put_latest_state")
                        .with_arguments(args.as_object().unwrap().clone()),
                )
                .await
                .unwrap();
            let rmcp::model::CallToolResponse::Complete(res) = resp else {
                panic!("expected complete result");
            };
            assert_ne!(res.is_error, Some(true), "concurrent write {idx} failed: {res:?}");
            res.structured_content.unwrap()
        }));
    }
    let mut boundaries = Vec::new();
    for task in tasks {
        let out = task.await.unwrap();
        assert_eq!(out["outcome"], "committed");
        boundaries.push(out["resulting_write_boundary"].as_u64().unwrap());
    }
    boundaries.sort();
    boundaries.dedup();
    assert_eq!(boundaries.len(), 8, "8 serialized writes must advance 8 distinct boundaries");

    rw_harness.shutdown().await;
}

/// AC32-MCP-SAFETY-02: Idempotent `(store, scope, operation_id)` replay across session,
/// idle-close/reopen, and restart; differing payload fails atomically with `operation_id_conflict`.
#[tokio::test(flavor = "multi_thread")]
async fn ac32_mcp_safety_02_idempotent_replay_and_conflict_refusal() {
    let fixture = Fixture::new();
    seed_scoped_with_collection(&fixture.path("rw.sqlite"), "alpha", "ckpt");

    let cfg = fixture.config(json!({
        "stores": {"rw": {"path": "rw.sqlite", "allow_writes": true, "scope": "alpha"}},
        "idle_timeout_secs": 1
    }));
    let harness = Harness::start(cfg.clone()).await;

    let payload = json!({
        "store": "rw",
        "operation_id": "op-idem-1",
        "operations": [canonical_node_op("item:1", "note", "first idempotent body", "rev-1", "sv-1")]
    });
    let first = harness.ok("actuate", payload.clone()).await;
    assert_eq!(first["outcome"], "committed");
    let first_boundary = first["resulting_write_boundary"].as_u64().unwrap();

    // Same session replay
    let replay1 = harness.ok("actuate", payload.clone()).await;
    assert_eq!(replay1["outcome"], "committed");
    assert_eq!(replay1["resulting_write_boundary"].as_u64().unwrap(), first_boundary);
    assert_eq!(replay1["request_sha256"], first["request_sha256"]);

    // Wait for idle close -> reopen -> replay
    assert!(
        common::wait_for_readiness(&harness.client, "rw", "closed", Duration::from_secs(5)).await
    );
    let replay2 = harness.ok("actuate", payload.clone()).await;
    assert_eq!(replay2["outcome"], "committed");
    assert_eq!(replay2["resulting_write_boundary"].as_u64().unwrap(), first_boundary);

    // Differing payload with same operation_id -> Conflict / operation_id_conflict, zero mutation
    let conflict = harness
        .err(
            "actuate",
            json!({
                "store": "rw",
                "operation_id": "op-idem-1",
                "operations": [canonical_node_op("item:1", "note", "MUTATED DIFFERENT BODY", "rev-2", "sv-2")]
            }),
        )
        .await;
    assert_eq!(conflict["class"], "conflict");
    assert_eq!(conflict["code"], "operation_id_conflict");

    // Verify stored body is untouched
    let fetched = harness.ok("fetch", json!({"store": "rw", "refs": ["l:item:1"]})).await;
    assert_eq!(fetched["items"][0]["record"]["body"], "first idempotent body");
    harness.shutdown().await;

    // Restart new Adapter instance -> replay still returns persisted receipt
    let harness2 = Harness::start(cfg).await;
    let replay3 = harness2.ok("actuate", payload).await;
    assert_eq!(replay3["outcome"], "committed");
    assert_eq!(replay3["resulting_write_boundary"].as_u64().unwrap(), first_boundary);
    harness2.shutdown().await;
}

/// AC32-MCP-SAFETY-03: Concurrent CAS `expected_write_boundary` race -> exactly one winner,
/// losers fail atomically with `conflict/stale_write_boundary`, retry succeeds.
#[tokio::test(flavor = "multi_thread")]
async fn ac32_mcp_safety_03_cas_expected_write_boundary_race() {
    let fixture = Fixture::new();
    seed_scoped_with_collection(&fixture.path("rw.sqlite"), "alpha", "ckpt");
    let harness = Harness::start(fixture.config(json!({
        "stores": {"rw": {"path": "rw.sqlite", "allow_writes": true, "scope": "alpha"}},
        "admission_cap": 4
    })))
    .await;

    let orient = harness.ok("orient", json!({"store": "rw"})).await;
    let start_boundary = orient["state"]["canonical_write_boundary"].as_u64().unwrap();

    let mut tasks = Vec::new();
    for idx in 0..4 {
        let peer = harness.client.peer().clone();
        tasks.push(tokio::spawn(async move {
            let args = json!({
                "store": "rw",
                "operation_id": format!("op-cas-{idx}"),
                "collection": "ckpt",
                "record_key": "shared-key",
                "payload_json": format!(r#"{{"step":{idx}}}"#),
                "expected_write_boundary": start_boundary
            });
            let resp = peer
                .call_tool_once(
                    rmcp::model::CallToolRequestParams::new("put_latest_state")
                        .with_arguments(args.as_object().unwrap().clone()),
                )
                .await
                .unwrap();
            let rmcp::model::CallToolResponse::Complete(res) = resp else {
                panic!("expected complete result");
            };
            (res.is_error == Some(true), res.structured_content.unwrap())
        }));
    }

    let mut winners = 0usize;
    let mut losers = 0usize;
    let mut observed = None;
    for task in tasks {
        let (is_err, payload) = task.await.unwrap();
        if !is_err {
            winners += 1;
            assert_eq!(payload["outcome"], "committed");
        } else {
            losers += 1;
            assert_eq!(payload["error"]["class"], "conflict");
            assert_eq!(payload["error"]["code"], "stale_write_boundary");
            observed = payload["error"]["observed_write_boundary"].as_u64();
        }
    }
    assert_eq!(winners, 1, "exactly one CAS writer must win");
    assert_eq!(losers, 3, "three CAS writers must lose");
    let new_boundary = observed.expect("stale_write_boundary should carry observed_write_boundary");
    assert!(new_boundary > start_boundary);

    // Retry against the newly observed boundary succeeds
    let retry = harness
        .ok(
            "put_latest_state",
            json!({
                "store": "rw",
                "operation_id": "op-cas-retry",
                "collection": "ckpt",
                "record_key": "shared-key",
                "payload_json": r#"{"step":99}"#,
                "expected_write_boundary": new_boundary
            }),
        )
        .await;
    assert_eq!(retry["outcome"], "committed");
    harness.shutdown().await;
}

/// AC32-MCP-SAFETY-04: Two-caller scope isolation, anti-spoof, and kind/collection allowlists.
#[tokio::test(flavor = "multi_thread")]
async fn ac32_mcp_safety_04_scope_isolation_anti_spoof_and_allowlists() {
    let fixture = Fixture::new();
    let db = fixture.path("multi.sqlite");

    // Seed data for BOTH "alpha" and "beta" in the same database file!
    {
        let eng_a = Engine::open(
            &db,
            OpenOptions::default().with_scope_policy(ScopePolicy::single("alpha")),
        )
        .unwrap();
        eng_a
            .write(&[PreparedWrite::AdminSchema {
                name: "ckpt".into(),
                kind: "latest_state".into(),
                schema_json: "{}".into(),
                retention_json: "{}".into(),
            }])
            .unwrap();
        eng_a
            .write(&[
                node("alpha:1", "note", "shared keyword alpha-only-secret-token"),
                node("alpha:2", "note", "shared keyword alpha-second"),
            ])
            .unwrap();
        eng_a.close().unwrap();

        let eng_b = Engine::open(
            &db,
            OpenOptions::default().with_scope_policy(ScopePolicy::single("beta")),
        )
        .unwrap();
        eng_b.write(&[node("beta:1", "note", "shared keyword beta-only-secret-token")]).unwrap();
        eng_b.close().unwrap();
    }

    // Open under scope "alpha" with strict allowlists
    let harness_a = Harness::start(fixture.config(json!({
        "stores": {
            "tenant": {
                "path": "multi.sqlite",
                "allow_writes": true,
                "scope": "alpha",
                "allowed_node_kinds": ["note"],
                "allowed_edge_kinds": ["links"],
                "allowed_collections": ["ckpt"]
            }
        }
    })))
    .await;

    // 1. Alpha search sees ONLY alpha records, never beta!
    let search_a = harness_a
        .ok("search", json!({"store": "tenant", "query": "shared keyword", "limit": 1}))
        .await;
    let refs_a = common::refs(&search_a);
    assert_eq!(refs_a.len(), 1);
    assert!(refs_a[0].starts_with("l:alpha:"), "must return only alpha scope: {refs_a:?}");
    let cont_a = search_a["continuation"].as_str().unwrap().to_string();

    // 2. Alpha fetch on beta:1 returns nondisclosing not_found
    let fetch_cross = harness_a.ok("fetch", json!({"store": "tenant", "refs": ["l:beta:1"]})).await;
    assert_eq!(fetch_cross["items"][0]["error"]["class"], "not_found");
    assert_eq!(fetch_cross["items"][0]["error"]["code"], "not_found");

    // 3. Anti-spoof: injecting "scope": "beta" in actuation op fails with scope_violation
    let spoof = harness_a
        .err(
            "actuate",
            json!({
                "store": "tenant",
                "operation_id": "op-spoof-1",
                "operations": [{
                    "op": "put_canonical_node",
                    "scope": "beta",
                    "kind": "note",
                    "body": "spoofed",
                    "source_id": "s1",
                    "logical_id": "spoof:1",
                    "artifact_revision_id": "rev-spoof",
                    "source_version_id": "sv-spoof"
                }]
            }),
        )
        .await;
    assert_eq!(spoof["class"], "invalid_request");
    assert_eq!(spoof["code"], "scope_violation");

    // 4. Kind/collection allowlist rejection -> kind_not_allowed with zero DB mutation
    let bad_kind = harness_a
        .err(
            "actuate",
            json!({
                "store": "tenant",
                "operation_id": "op-bad-kind",
                "operations": [canonical_node_op("bad:1", "forbidden_kind", "no", "rev-bk", "sv-bk")]
            }),
        )
        .await;
    assert_eq!(bad_kind["class"], "invalid_request");
    assert_eq!(bad_kind["code"], "kind_not_allowed");

    let bad_coll = harness_a
        .err(
            "put_latest_state",
            json!({
                "store": "tenant",
                "operation_id": "op-bad-coll",
                "collection": "other_collection",
                "record_key": "k1",
                "payload_json": "{}"
            }),
        )
        .await;
    assert_eq!(bad_coll["class"], "invalid_request");
    assert_eq!(bad_coll["code"], "kind_not_allowed");

    // Alpha writes a new node -> stamped with alpha, never visible to beta
    let ok_write = harness_a
        .ok(
            "actuate",
            json!({
                "store": "tenant",
                "operation_id": "op-alpha-new",
                "operations": [canonical_node_op("alpha:3", "note", "shared keyword alpha-new", "rev-a3", "sv-a3")]
            }),
        )
        .await;
    assert_eq!(ok_write["outcome"], "committed");
    harness_a.shutdown().await;

    // Now open same DB under scope "beta"
    let harness_b = Harness::start(fixture.config(json!({
        "stores": {
            "tenant": {
                "path": "multi.sqlite",
                "allow_writes": true,
                "scope": "beta"
            }
        }
    })))
    .await;
    let search_b =
        harness_b.ok("search", json!({"store": "tenant", "query": "shared keyword"})).await;
    assert_eq!(common::refs(&search_b), vec!["l:beta:1"]);
    // Using alpha's continuation token on beta returns nondisclosing not_found
    let cont_cross =
        harness_b.err("search", json!({"store": "tenant", "continuation": cont_a})).await;
    assert_eq!(cont_cross["class"], "not_found");
    assert_eq!(cont_cross["code"], "not_found");
    harness_b.shutdown().await;
}

/// AC32-MCP-SAFETY-05: Strict-only MCP writes & atomic batch rollback; bulk/non-strict refused.
#[tokio::test(flavor = "multi_thread")]
async fn ac32_mcp_safety_05_strict_only_and_atomic_rollback() {
    let fixture = Fixture::new();
    seed_scoped_with_collection(&fixture.path("rw.sqlite"), "alpha", "ckpt");
    let harness = Harness::start(fixture.config(json!({
        "stores": {"rw": {"path": "rw.sqlite", "allow_writes": true, "scope": "alpha"}}
    })))
    .await;

    // Seed a canonical source so we can reference its revision in a derived edge
    let src_body = "canonical source body for strict edge test";
    let src_hash = sha256_hex(src_body.as_bytes());
    let seed = harness
        .ok(
            "actuate",
            json!({
                "store": "rw",
                "operation_id": "op-seed-strict",
                "operations": [canonical_node_op("src:1", "note", src_body, "rev-src-1", "sv-1")]
            }),
        )
        .await;
    assert_eq!(seed["outcome"], "committed");

    // Batch with op[0] valid node AND op[1] edge pointing to MISSING endpoint -> whole batch rolls back!
    let err = harness
        .err(
            "actuate",
            json!({
                "store": "rw",
                "operation_id": "op-rollback-batch",
                "operations": [
                    canonical_node_op("should-not-exist", "note", "rolled back body", "rev-rb-1", "sv-rb-1"),
                    derived_edge_op("links", "src:1", "missing-endpoint", "rev-rb-edge", "sv-1", "rev-src-1", &src_hash)
                ]
            }),
        )
        .await;
    assert_eq!(err["class"], "not_found", "missing edge endpoint refusal: {err}");

    // Confirm op[0] was rolled back atomically!
    let fetched = harness.ok("fetch", json!({"store": "rw", "refs": ["l:should-not-exist"]})).await;
    assert_eq!(fetched["items"][0]["error"]["code"], "not_found");

    // Bulk / non-strict flags refused with forbidden_operation
    let bulk_err = harness
        .err(
            "actuate",
            json!({
                "store": "rw",
                "operation_id": "op-bulk-refuse",
                "operations": [{
                    "op": "put_derived_edge",
                    "bulk": true,
                    "kind": "links",
                    "from": "src:1",
                    "to": "missing",
                    "source_id": "mcp-test-src",
                    "artifact_revision_id": "rev-b",
                    "source_version_id": "sv-1",
                    "source_revision_id": "rev-src-1",
                    "canonical_source_hash": src_hash
                }]
            }),
        )
        .await;
    assert_eq!(bulk_err["class"], "invalid_request");
    assert_eq!(bulk_err["code"], "forbidden_operation");

    harness.shutdown().await;
}

/// AC32-MCP-SAFETY-06 & AC32-LEAK-01: Blocking cancellation & shutdown drain fence for both
/// managed and hosted engines; permits & abandoned counters drain cleanly to zero.
#[tokio::test(flavor = "multi_thread")]
async fn ac32_mcp_safety_06_blocking_drain_fence_on_cancel_and_shutdown() {
    let fixture = Fixture::new();
    seed_scoped_with_collection(&fixture.path("rw.sqlite"), "alpha", "ckpt");

    let harness = Harness::start(fixture.config(json!({
        "stores": {"rw": {"path": "rw.sqlite", "allow_writes": true, "scope": "alpha"}},
        "call_deadline_ms": 500,
        "run_limit_ms": 500
    })))
    .await;
    // Open store first
    harness.ok("orient", json!({"store": "rw"})).await;

    // Block engine calls and start a write that hits run_limit (abandoned on blocking thread)
    harness.adapter.seams().block_calls("rw");
    let timed_out = harness
        .err(
            "put_latest_state",
            json!({
                "store": "rw",
                "operation_id": "op-drain-fence-1",
                "collection": "ckpt",
                "record_key": "k1",
                "payload_json": r#"{"step":42}"#
            }),
        )
        .await;
    assert_eq!(timed_out["class"], "timeout");
    assert_eq!(timed_out["code"], "deadline");

    // Start shutdown while the abandoned blocking write is still held!
    let adapter = harness.adapter.clone();
    let shutdown_task = tokio::spawn(async move {
        adapter.shutdown().await;
    });
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(
        !shutdown_task.is_finished(),
        "shutdown drain fence must wait while blocking write is still active"
    );

    // Release the blocking write; shutdown MUST now complete cleanly and quickly
    harness.adapter.seams().release_calls("rw");
    tokio::time::timeout(Duration::from_secs(5), shutdown_task)
        .await
        .expect("shutdown completes after blocking write drains")
        .unwrap();

    // DB lock must be released and the in-flight write committed cleanly!
    let verify = Engine::open(
        fixture.path("rw.sqlite"),
        OpenOptions::default().with_scope_policy(ScopePolicy::single("alpha")),
    )
    .unwrap();
    let state = read::operational_state(&verify, "ckpt", "k1", None).unwrap();
    assert!(state.is_some(), "in-flight write drained cleanly before close");
    verify.close().unwrap();

    // Also test hosted engine drain fence on Adapter::shutdown()
    let hosted_path = fixture.path("hosted.sqlite");
    seed_scoped_with_collection(&hosted_path, "alpha", "ckpt");
    let hosted_engine = Arc::new(
        Engine::open(
            &hosted_path,
            OpenOptions::default().with_scope_policy(ScopePolicy::single("alpha")),
        )
        .unwrap(),
    );
    let hosted_adapter = Adapter::new(common::structured_config()).unwrap();
    hosted_adapter
        .register_engine_with_policy(
            "h",
            hosted_engine.clone(),
            HostedStorePolicy {
                allow_writes: true,
                scope: Some(ScopePolicy::single("alpha")),
                ..HostedStorePolicy::default()
            },
        )
        .unwrap();
    let hosted_client = common::connect(&hosted_adapter).await;
    hosted_adapter.seams().block_calls("h");
    let peer = hosted_client.peer().clone();
    let in_flight = tokio::spawn(async move {
        peer.call_tool_once(
            rmcp::model::CallToolRequestParams::new("orient")
                .with_arguments(json!({"store": "h"}).as_object().unwrap().clone()),
        )
        .await
    });
    // Wait until seam holds the call
    for _ in 0..100 {
        if hosted_adapter.seams().held_calls("h") == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(hosted_adapter.seams().held_calls("h"), 1);

    let a2 = hosted_adapter.clone();
    let sd = tokio::spawn(async move { a2.shutdown().await });
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(!sd.is_finished(), "shutdown must wait for hosted blocking call to drain");
    hosted_adapter.seams().release_calls("h");
    tokio::time::timeout(Duration::from_secs(5), sd).await.unwrap().unwrap();
    let _ = in_flight.await;
    let _ = hosted_client.cancel().await;
    // Hosted engine stays open for its host!
    assert!(read::visibility_state(&hosted_engine).is_ok());
    hosted_engine.close().unwrap();
}

/// AC32-MCP-SAFETY-07: Real subprocess `SIGKILL` crash/reopen integrity and receipt replay.
#[cfg(unix)]
#[test]
fn ac32_mcp_safety_07_sigkill_crash_reopen_integrity_and_receipt_replay() {
    let fixture = Fixture::new();
    let db = fixture.path("crash.sqlite");
    seed_scoped_with_collection(&db, "alpha", "ckpt");
    let config = fixture.config_file(json!({
        "stores": {"rw": {"path": "crash.sqlite", "allow_writes": true, "scope": "alpha"}},
        "idle_timeout_secs": 0
    }));

    let mut server = Spawned::start(&config);
    server.initialize();
    let res = server.call(
        "put_latest_state",
        json!({
            "store": "rw",
            "operation_id": "op-before-sigkill",
            "collection": "ckpt",
            "record_key": "k1",
            "payload_json": r#"{"step":7}"#
        }),
    );
    assert_ne!(res["isError"], true, "{res}");
    let boundary = res["structuredContent"]["resulting_write_boundary"].as_u64().unwrap();

    // Hard-kill the subprocess mid-session (SIGKILL)
    server.child.kill().unwrap();
    let _ = server.child.wait().unwrap();

    // Verify PRAGMA integrity_check == ok
    let conn =
        rusqlite::Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let integrity: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0)).unwrap();
    assert_eq!(integrity, "ok");
    drop(conn);

    // Reopen fresh MCP subprocess: stale lock recovered, persisted receipt replays idempotently
    let mut server2 = Spawned::start(&config);
    server2.initialize();
    let replay = server2.call(
        "put_latest_state",
        json!({
            "store": "rw",
            "operation_id": "op-before-sigkill",
            "collection": "ckpt",
            "record_key": "k1",
            "payload_json": r#"{"step":7}"#
        }),
    );
    assert_ne!(replay["isError"], true, "{replay}");
    assert_eq!(replay["structuredContent"]["resulting_write_boundary"].as_u64().unwrap(), boundary);
    let (status, _, _) = server2.finish();
    assert_eq!(status.code(), Some(0));
}

/// AC32-MCP-SAFETY-08: Adversarial secret-scrubbed errors — canary secrets never leak in
/// any tool error or stderr diagnostic.
#[tokio::test(flavor = "multi_thread")]
async fn ac32_mcp_safety_08_adversarial_secret_scrubbed_errors() {
    let canary_body = "CANARY_BODY_SENTINEL_ALPHA";
    let canary_record_id = "CANARY_RECORD_ID_SENTINEL";
    let canary_sql = "SELECT * FROM canonical_nodes";

    let fixture = Fixture::new();
    let db = fixture.path("secret.sqlite");
    seed_scoped_with_collection(&db, "alpha", "ckpt");
    {
        let eng = Engine::open(
            &db,
            OpenOptions::default().with_scope_policy(ScopePolicy::single("beta")),
        )
        .unwrap();
        eng.write(&[node("beta:secret", "note", canary_body)]).unwrap();
        eng.close().unwrap();
    }

    let harness = Harness::start(fixture.config(json!({
        "stores": {"rw": {"path": "secret.sqlite", "allow_writes": true, "scope": "alpha"}}
    })))
    .await;

    let errors = vec![
        // Out-of-scope fetch
        harness.ok("fetch", json!({"store": "rw", "refs": ["l:beta:secret"]})).await["items"][0]
            ["error"]
            .clone(),
        // Invalid JSON payload containing canary
        harness
            .err(
                "put_latest_state",
                json!({
                    "store": "rw",
                    "operation_id": "op-sec-1",
                    "collection": "ckpt",
                    "record_key": canary_record_id,
                    "payload_json": format!("not-json-{canary_body}-{canary_sql}")
                }),
            )
            .await,
        // Schema violation payload containing canary
        harness
            .err(
                "put_latest_state",
                json!({
                    "store": "rw",
                    "operation_id": "op-sec-2",
                    "collection": "ckpt",
                    "record_key": canary_record_id,
                    "payload_json": format!(r#"{{"wrong":"{canary_body}"}}"#)
                }),
            )
            .await,
        // Invalid filter containing canary
        harness
            .err(
                "search",
                json!({
                    "store": "rw",
                    "query": "test",
                    "attributes": {canary_record_id: canary_body}
                }),
            )
            .await,
    ];

    for err in errors {
        let serialized = serde_json::to_string(&err).unwrap();
        assert!(!serialized.contains(canary_body), "body canary leaked: {serialized}");
        assert!(!serialized.contains(canary_record_id), "record_id canary leaked: {serialized}");
        assert!(!serialized.contains(canary_sql), "sql canary leaked: {serialized}");
    }
    harness.shutdown().await;
}

/// AC32-MCP-SAFETY-09: Forbidden operator escape hatches are never listed and always refused.
#[tokio::test(flavor = "multi_thread")]
async fn ac32_mcp_safety_09_forbidden_operator_escape_hatches_always_refused() {
    let fixture = Fixture::new();
    seed_scoped_with_collection(&fixture.path("rw.sqlite"), "alpha", "ckpt");
    let harness = Harness::start(fixture.config(json!({
        "stores": {"rw": {"path": "rw.sqlite", "allow_writes": true, "scope": "alpha"}}
    })))
    .await;

    let listed: Vec<String> =
        harness.tools().await.into_iter().map(|t| t.name.into_owned()).collect();
    let forbidden = [
        "recover",
        "restore",
        "repair",
        "fix",
        "rebuild",
        "purge",
        "erase_source",
        "migrate",
        "safe_export",
        "sql",
        "execute_sql",
        "bulk_write",
    ];
    for name in forbidden {
        assert!(!listed.contains(&name.to_string()), "forbidden tool {name} must not be listed");
    }

    let mut raw = RawClient::connect(&harness.adapter).await;
    for name in forbidden {
        let resp =
            raw.request("tools/call", json!({"name": name, "arguments": {"store": "rw"}})).await;
        let res = &resp["result"];
        assert_eq!(res["isError"], true, "forbidden tool {name} must fail: {resp}");
        assert_eq!(res["structuredContent"]["error"]["class"], "invalid_request");
        assert_eq!(res["structuredContent"]["error"]["code"], "forbidden_operation");

        // Also forbidden inside actuate.operations
        let act_err = harness
            .err(
                "actuate",
                json!({
                    "store": "rw",
                    "operation_id": format!("op-forbid-{name}"),
                    "operations": [{"op": name}]
                }),
            )
            .await;
        assert_eq!(act_err["class"], "invalid_request");
        assert_eq!(act_err["code"], "forbidden_operation");
    }
    harness.shutdown().await;
}
