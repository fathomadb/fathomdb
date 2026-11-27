//! Slice 50 (`WP-50B`, `AC32-BACKUP-REBUILD-04`) — `fathomdb-mcp` repeated
//! open / write (`actuate`) / search / idle-drain / close lifecycle leak and
//! state-machine checks (bounded FDs, sidecar lock release, clean epoch
//! transitions).

mod common;

use std::time::Duration;

use common::{can_open, readiness, wait_for_readiness, Fixture, Harness};
use fathomdb_sdk::{Engine, OpenOptions, PreparedWrite, ScopePolicy};
use serde_json::json;

#[cfg(target_os = "linux")]
fn count_open_fds() -> usize {
    std::fs::read_dir("/proc/self/fd").expect("read /proc/self/fd").filter_map(Result::ok).count()
}

#[cfg(target_os = "linux")]
fn count_threads() -> usize {
    std::fs::read_dir("/proc/self/task")
        .expect("read /proc/self/task")
        .filter_map(Result::ok)
        .count()
}

#[tokio::test(flavor = "multi_thread")]
async fn mcp_repeated_open_write_search_idle_drain_close_has_zero_fd_or_lock_leaks() {
    let fixture = Fixture::new();
    let db_path = fixture.path("lifecycle.sqlite");

    // Seed an empty scoped store.
    {
        let eng = Engine::open(
            &db_path,
            OpenOptions::default().with_scope_policy(ScopePolicy::single("alpha")),
        )
        .expect("seed scoped store");
        eng.write(&[PreparedWrite::AdminSchema {
            name: "state".into(),
            kind: "latest_state".into(),
            schema_json: r#"{"type":"object","properties":{"n":{"type":"integer"}},"required":["n"]}"#
                .into(),
            retention_json: "{}".into(),
        }])
        .expect("register collection");
        eng.close().expect("close seeded store");
    }

    let cfg = fixture.config(json!({
        "stores": {
            "rw": {
                "path": "lifecycle.sqlite",
                "allow_writes": true,
                "scope": "alpha"
            }
        },
        "idle_timeout_secs": 1
    }));

    let harness = Harness::start(cfg).await;
    assert_eq!(readiness(&harness.client, "rw").await, "closed");
    assert!(can_open(&db_path), "lazy store starts unlocked");

    // Warmup cycle.
    let warm = harness
        .ok(
            "actuate",
            json!({
                "store": "rw",
                "operation_id": "op-warm-0",
                "operations": [{
                    "op": "put_canonical_node",
                    "kind": "note",
                    "body": "warmup lifecycle entry 0",
                    "source_id": "mcp-lifecycle",
                    "logical_id": "lc:warm",
                    "artifact_revision_id": "rev-warm-0",
                    "source_version_id": "sv-warm-0"
                }]
            }),
        )
        .await;
    assert_eq!(warm["outcome"], "committed");
    assert!(wait_for_readiness(&harness.client, "rw", "closed", Duration::from_secs(5)).await);
    assert!(can_open(&db_path), "lock released after warmup idle-drain");

    #[cfg(target_os = "linux")]
    let fds_before = count_open_fds();
    #[cfg(target_os = "linux")]
    let threads_before = count_threads();

    let mut last_epoch = 1_u64;
    for cycle in 1..=4_u64 {
        let write_res = harness
            .ok(
                "actuate",
                json!({
                    "store": "rw",
                    "operation_id": format!("op-cycle-{cycle}"),
                    "operations": [{
                        "op": "put_canonical_node",
                        "kind": "note",
                        "body": format!("mcp lifecycle cycle-{cycle} token-{cycle}"),
                        "source_id": "mcp-lifecycle",
                        "logical_id": format!("lc:{cycle}"),
                        "artifact_revision_id": format!("rev-cycle-{cycle}"),
                        "source_version_id": format!("sv-cycle-{cycle}")
                    }]
                }),
            )
            .await;
        assert_eq!(write_res["outcome"], "committed");
        assert_eq!(readiness(&harness.client, "rw").await, "ready");
        assert!(!can_open(&db_path), "lock held while store is open in cycle {cycle}");

        let found =
            harness.ok("search", json!({"store": "rw", "query": format!("token-{cycle}")})).await;
        let epoch = found["epoch"].as_u64().expect("epoch");
        assert!(
            epoch > last_epoch,
            "epoch must advance across idle closes: {epoch} > {last_epoch}"
        );
        last_epoch = epoch;
        assert_eq!(found["results"][0]["ref"], format!("l:lc:{cycle}"));

        // Wait for idle drain -> closed transition.
        assert!(
            wait_for_readiness(&harness.client, "rw", "closed", Duration::from_secs(5)).await,
            "store did not idle-close in cycle {cycle}"
        );
        assert!(can_open(&db_path), "sidecar lock leaked after idle close in cycle {cycle}");
    }

    #[cfg(target_os = "linux")]
    {
        let fds_after = count_open_fds();
        let threads_after = count_threads();
        assert!(
            fds_after <= fds_before + 2,
            "open FD leak across MCP open/write/search/idle-close cycles: before={fds_before}, after={fds_after}"
        );
        assert!(
            threads_after <= threads_before + 2,
            "thread leak across MCP open/write/search/idle-close cycles: before={threads_before}, after={threads_after}"
        );
    }

    harness.shutdown().await;
    assert!(can_open(&db_path), "sidecar lock must be released after shutdown");
}
