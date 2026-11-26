//! SDK-vs-MCP exact frozen-evidence equivalence, live `EP-01` `Envelope.state`,
//! and per-class projection namespace in `orient` (Slice 40 WP-40B).

mod common;

use std::path::Path;

use common::{Fixture, Harness};
use fathomdb_sdk::{
    graph, ActuationBatchV1, ActuationOperationV1, ArtifactRevisionId, CanonicalHash, Engine,
    EvidenceResolveManyItemV1, EvidenceResolveManyRequestV1, EvidenceSearchRequestV1,
    GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, IdSpace, InitialState, OpenOptions,
    ProjectionRole, ProjectionSpec, PropertyEntityClass, ProvenancedEdgeV1, ProvenancedNodeV1,
    ReadContextV1, ReadView, ScopePolicy, SearchFilter, SourceId, SourceLocator, SourceRevisionId,
    SourceVersionId, TraversalDirection, WriteProvenanceV1,
};
use serde_json::json;

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn seed_provenanced_store(path: &Path) {
    let body_a = "exact UTF-8 evidence body: river crossing é😀\nline two";
    let hash_a = CanonicalHash::sha256(sha256_hex(body_a.as_bytes())).unwrap();
    let body_b = "linked target node body: river valley café";

    // Seed scope "alpha"
    {
        let eng = Engine::open(
            path,
            OpenOptions::default().with_scope_policy(ScopePolicy::single("alpha")),
        )
        .unwrap();
        let batch = ActuationBatchV1::new(
            "seed-alpha-1",
            vec![
                ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
                    kind: "note".into(),
                    body: body_a.into(),
                    source_id: SourceId::new("src:alpha").unwrap(),
                    logical_id: Some("alpha:src".into()),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    properties: None,
                    provenance: WriteProvenanceV1::canonical(
                        ArtifactRevisionId::new("rev-alpha-src").unwrap(),
                        SourceVersionId::new("sv-alpha-1").unwrap(),
                    ),
                }),
                ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
                    kind: "note".into(),
                    body: body_b.into(),
                    source_id: SourceId::new("src:alpha").unwrap(),
                    logical_id: Some("alpha:dst".into()),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    properties: None,
                    provenance: WriteProvenanceV1::canonical(
                        ArtifactRevisionId::new("rev-alpha-dst").unwrap(),
                        SourceVersionId::new("sv-alpha-2").unwrap(),
                    ),
                }),
                ActuationOperationV1::PutDerivedEdge(ProvenancedEdgeV1 {
                    kind: "links".into(),
                    from: "alpha:src".into(),
                    to: "alpha:dst".into(),
                    source_id: SourceId::new("src:alpha").unwrap(),
                    logical_id: Some("alpha:edge:1".into()),
                    body: Some("edge evidence text".into()),
                    t_valid: None,
                    t_invalid: None,
                    confidence: Some(0.95),
                    extractor_model_id: None,
                    temporal_fallback: None,
                    properties: None,
                    provenance: WriteProvenanceV1::derived(
                        ArtifactRevisionId::new("rev-alpha-edge").unwrap(),
                        SourceVersionId::new("sv-alpha-1").unwrap(),
                        SourceRevisionId::new("rev-alpha-src").unwrap(),
                        SourceLocator::whole_body(),
                        hash_a,
                    ),
                }),
            ],
        )
        .unwrap();
        eng.actuate(batch).unwrap();
        eng.close().unwrap();
    }

    // Seed scope "beta" with overlapping query terms so caller-scope filtering is verified
    {
        let eng = Engine::open(
            path,
            OpenOptions::default().with_scope_policy(ScopePolicy::single("beta")),
        )
        .unwrap();
        let batch = ActuationBatchV1::new(
            "seed-beta-1",
            vec![ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
                kind: "note".into(),
                body: "beta secret river crossing evidence".into(),
                source_id: SourceId::new("src:beta").unwrap(),
                logical_id: Some("beta:src".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("rev-beta-src").unwrap(),
                    SourceVersionId::new("sv-beta-1").unwrap(),
                ),
            })],
        )
        .unwrap();
        eng.actuate(batch).unwrap();
        eng.close().unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn sdk_and_mcp_frozen_evidence_match_identically_on_bytes_revisions_scope_and_outcomes() {
    let fixture = Fixture::new();
    let db = fixture.path("ev.sqlite");
    seed_provenanced_store(&db);

    let pinned_as_of = 1_900_000_000_i64;

    // 1. Run reference workflow directly through `fathomdb-sdk` under scope "alpha"
    let (sdk_search_body, sdk_search_rev, sdk_search_hash, sdk_graph_body, sdk_graph_rev) = {
        let eng = Engine::open(
            &db,
            OpenOptions::default().with_scope_policy(ScopePolicy::single("alpha")),
        )
        .unwrap();
        let view = ReadView { valid_as_of: Some(pinned_as_of), ..ReadView::default() };
        let frozen = eng
            .freeze_read_context(&ReadContextV1::new(view, SearchFilter::default()).unwrap())
            .unwrap();
        let req = EvidenceSearchRequestV1::new("river".into(), frozen.clone()).with_limit(10);
        let res = eng.search_with_evidence(&req).unwrap();
        assert_eq!(res.evidence.len(), 2, "only alpha's 2 nodes visible");
        let src_idx = res
            .search_result
            .results
            .iter()
            .position(|h| h.id == IdSpace::logical("alpha:src"))
            .unwrap();
        let ev_ref = res.evidence[src_idx].evidence_ref.clone();
        let resolved = eng
            .resolve_evidence_many(&EvidenceResolveManyRequestV1 {
                schema_version: 1,
                evidence_refs: vec![ev_ref],
                context: frozen.clone(),
            })
            .unwrap();
        let EvidenceResolveManyItemV1::Resolved(ev) = &resolved[0] else {
            panic!("expected resolved evidence");
        };

        // Graph expand under same frozen context with evidence sidecar
        let g_req = GraphExpandRequestV1::new(
            GraphSeedV1::Explicit {
                schema_version: 1,
                logical_ids: vec![IdSpace::logical("alpha:src")],
            },
            TraversalDirection::Outgoing,
            GraphReadContextV1::Frozen { schema_version: 1, context: frozen.clone() },
            1,
            10,
            10_000,
        )
        .with_evidence(true);
        let g_res = graph::expand(&eng, &g_req).unwrap();
        let g_ev_sidecar = g_res.evidence.as_ref().unwrap();
        assert_eq!(g_ev_sidecar.entries.len(), 1);
        let g_resolved = eng
            .resolve_graph_evidence(&fathomdb_sdk::GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: g_ev_sidecar.entries[0].target_evidence_ref.clone(),
                context: frozen,
            })
            .unwrap();
        eng.close().unwrap();
        (
            ev.canonical_source_body.clone(),
            ev.artifact_revision_id.clone(),
            ev.canonical_source_hash.digest_hex().to_string(),
            g_resolved.canonical_source_body.clone(),
            g_resolved.artifact_revision_id.as_str().to_string(),
        )
    };

    // 2. Run identical workflow through `fathomdb-mcp`
    let harness = Harness::start(fixture.config(json!({
        "stores": {"ev": {"path": "ev.sqlite", "allow_writes": true, "scope": "alpha"}}
    })))
    .await;

    let search = harness
        .ok(
            "search",
            json!({
                "store": "ev",
                "query": "river",
                "as_of": pinned_as_of,
                "limit": 10,
                "freeze": true
            }),
        )
        .await;
    let frozen_handle =
        search["frozen_context"].as_str().expect("frozen_context handle").to_string();
    let hits = search["results"].as_array().unwrap();
    assert_eq!(hits.len(), 2, "alpha scope must exclude beta");
    let src_hit = hits.iter().find(|h| h["ref"] == "l:alpha:src").unwrap();
    let mcp_ev_ref = src_hit["evidence_ref"].as_str().expect("evidence_ref").to_string();
    assert_eq!(src_hit["artifact_revision_id"], sdk_search_rev);

    // Continuation guard rejects `freeze` and `frozen_context` on continuation calls
    let paged = harness
        .ok("search", json!({"store": "ev", "query": "river", "limit": 1, "as_of": pinned_as_of}))
        .await;
    let cont = paged["continuation"].as_str().unwrap();
    for bad in [
        json!({"store": "ev", "continuation": cont, "freeze": true}),
        json!({"store": "ev", "continuation": cont, "frozen_context": &frozen_handle}),
    ] {
        let err = harness.err("search", bad).await;
        assert_eq!(err["class"], "invalid_request");
        assert_eq!(err["code"], "bad_argument");
    }

    // Fetch exact search evidence via `fetch` + `frozen_context` + `evidence_refs`
    let fetched = harness
        .ok(
            "fetch",
            json!({
                "store": "ev",
                "frozen_context": &frozen_handle,
                "evidence_refs": [&mcp_ev_ref]
            }),
        )
        .await;
    let rec = &fetched["items"][0]["record"];
    assert_eq!(rec["body"].as_str().unwrap(), sdk_search_body, "exact UTF-8 byte equality");
    assert_eq!(rec["evidence_text"].as_str().unwrap(), sdk_search_body);
    assert_eq!(rec["artifact_revision_id"].as_str().unwrap(), sdk_search_rev);
    assert_eq!(rec["canonical_source_hash"].as_str().unwrap(), sdk_search_hash);
    assert_eq!(rec["body_cut"], false);

    // Expand under the same `frozen_context` with `include_evidence_refs: true`
    let expanded = harness
        .ok(
            "expand",
            json!({
                "store": "ev",
                "refs": ["l:alpha:src"],
                "frozen_context": &frozen_handle,
                "include_evidence_refs": true
            }),
        )
        .await;
    let target = &expanded["targets"][0];
    let graph_ev_ref =
        target["graph_evidence_ref"].as_str().expect("graph_evidence_ref").to_string();
    assert_eq!(target["artifact_revision_id"].as_str().unwrap(), sdk_graph_rev);

    // Resolve graph evidence via `fetch` + `frozen_context` + `graph_evidence_refs`
    let g_fetched = harness
        .ok(
            "fetch",
            json!({
                "store": "ev",
                "frozen_context": &frozen_handle,
                "graph_evidence_refs": [&graph_ev_ref]
            }),
        )
        .await;
    let g_rec = &g_fetched["items"][0]["record"];
    assert_eq!(g_rec["body"].as_str().unwrap(), sdk_graph_body);
    assert_eq!(g_rec["artifact_revision_id"].as_str().unwrap(), sdk_graph_rev);

    // Truncation outcome under tight `max_tokens` sets `body_cut: true` and `budget.cut: true`
    let long_body = "exact long utf8 evidence sentence ".repeat(60);
    harness
        .ok(
            "actuate",
            json!({
                "store": "ev",
                "operation_id": "op-long-ev",
                "operations": [{
                    "op": "put_canonical_node",
                    "kind": "note",
                    "body": long_body,
                    "source_id": "src:alpha",
                    "logical_id": "alpha:long",
                    "artifact_revision_id": "rev-alpha-long",
                    "source_version_id": "sv-alpha-long"
                }]
            }),
        )
        .await;

    // 3. State drift: using the OLD `frozen_handle` after the write fails with conflict/state_drift!
    let drift_err = harness
        .err(
            "fetch",
            json!({
                "store": "ev",
                "frozen_context": &frozen_handle,
                "evidence_refs": [&mcp_ev_ref]
            }),
        )
        .await;
    assert_eq!(drift_err["class"], "conflict");
    assert_eq!(drift_err["code"], "state_drift");

    // Mint a new frozen context and verify truncation on the long record
    let s2 = harness
        .ok(
            "search",
            json!({"store": "ev", "query": "sentence", "freeze": true, "as_of": pinned_as_of}),
        )
        .await;
    let fh2 = s2["frozen_context"].as_str().unwrap();
    let long_ref = s2["results"][0]["evidence_ref"].as_str().unwrap();
    let truncated = harness
        .ok(
            "fetch",
            json!({
                "store": "ev",
                "frozen_context": fh2,
                "evidence_refs": [long_ref],
                "max_tokens": 900
            }),
        )
        .await;
    assert_eq!(truncated["items"][0]["record"]["body_cut"], true);
    assert_eq!(truncated["budget"]["cut"], true);

    // Invalid/forged evidence_ref under valid frozen context returns per-item nondisclosing not_found
    let bad_ev = harness
        .ok(
            "fetch",
            json!({
                "store": "ev",
                "frozen_context": fh2,
                "evidence_refs": ["forged-opaque-ref"]
            }),
        )
        .await;
    assert_eq!(bad_ev["items"][0]["error"]["class"], "not_found");
    assert_eq!(bad_ev["items"][0]["error"]["code"], "not_found");

    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn orient_exposes_live_ep01_state_and_per_class_projection_namespace() {
    let fixture = Fixture::new();
    let db = fixture.path("proj.sqlite");
    {
        let eng = Engine::open(
            &db,
            OpenOptions::default().with_scope_policy(ScopePolicy::single("alpha")),
        )
        .unwrap();
        let node_status = ProjectionSpec {
            name: "status".into(),
            roles: [ProjectionRole::Filterable].into_iter().collect(),
            fts: None,
            vector: None,
            source: None,
            entity_class: PropertyEntityClass::Node,
        };
        let edge_status = ProjectionSpec {
            name: "status".into(),
            roles: [ProjectionRole::Filterable].into_iter().collect(),
            fts: None,
            vector: None,
            source: None,
            entity_class: PropertyEntityClass::Edge,
        };
        eng.configure_projections(&[node_status, edge_status], &[]).unwrap();
        eng.close().unwrap();
    }

    let harness = Harness::start(fixture.config(json!({
        "stores": {"p": {"path": "proj.sqlite", "allow_writes": true, "scope": "alpha"}}
    })))
    .await;

    let o1 = harness.ok("orient", json!({"store": "p"})).await;
    // Live EP-01 Envelope.state
    let st1 = &o1["state"];
    assert_eq!(st1["status"], "available");
    assert_eq!(st1["database_id"].as_str().unwrap().len(), 32);
    assert_eq!(st1["nonce"].as_str().unwrap().len(), 64);
    assert_eq!(st1["drift"], "unchanged");
    assert!(st1["canonical_write_boundary"].is_u64());
    assert!(st1["projection_cursor"].is_u64());
    assert_eq!(o1["not_in_this_release"], json!(["verdict", "kinds", "counts"]));

    // Same-name projections on node and edge coexist cleanly without colliding!
    let projs = o1["projections"].as_array().unwrap();
    let pairs: Vec<(&str, &str)> = projs
        .iter()
        .map(|p| (p["entity_class"].as_str().unwrap(), p["name"].as_str().unwrap()))
        .collect();
    assert!(pairs.contains(&("node", "status")), "missing (node, status): {pairs:?}");
    assert!(pairs.contains(&("edge", "status")), "missing (edge, status): {pairs:?}");

    let status_projs = o1["projection_status"]["projections"].as_array().unwrap();
    let status_pairs: Vec<(&str, &str)> = status_projs
        .iter()
        .map(|p| (p["entity_class"].as_str().unwrap(), p["name"].as_str().unwrap()))
        .collect();
    assert!(
        status_pairs.contains(&("node", "status")),
        "missing status (node, status): {status_pairs:?}"
    );
    assert!(
        status_pairs.contains(&("edge", "status")),
        "missing status (edge, status): {status_pairs:?}"
    );

    // Open a second connection before writing so it observes the pre-write boundary
    let client2 = common::connect(&harness.adapter).await;
    let c2_before = common::ok(&client2, "orient", json!({"store": "p"})).await;
    assert_eq!(c2_before["state"]["drift"], "unchanged");

    // Perform a write on connection 1 -> actuate reports drift == "canonical" and higher boundary!
    let b1 = st1["canonical_write_boundary"].as_u64().unwrap();
    let act = harness
        .ok(
            "actuate",
            json!({
                "store": "p",
                "operation_id": "op-drift-1",
                "operations": [{
                    "op": "put_canonical_node",
                    "kind": "note",
                    "body": "advances write boundary",
                    "source_id": "src:p",
                    "logical_id": "p:1",
                    "artifact_revision_id": "rev-p-1",
                    "source_version_id": "sv-p-1"
                }]
            }),
        )
        .await;
    assert_eq!(act["state"]["drift"], "canonical");
    assert!(act["state"]["canonical_write_boundary"].as_u64().unwrap() > b1);

    // Connection 2's next orient observes drift == "canonical"!
    let c2_after = common::ok(&client2, "orient", json!({"store": "p"})).await;
    assert_eq!(c2_after["state"]["drift"], "canonical");
    assert!(c2_after["state"]["canonical_write_boundary"].as_u64().unwrap() > b1);
    let _ = client2.cancel().await;

    // Unchanged subsequent orient on connection 1 reports drift == "unchanged"
    let o3 = harness.ok("orient", json!({"store": "p"})).await;
    assert_eq!(o3["state"]["drift"], "unchanged");

    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn ac32_frozen_stab_03_mcp_disjoint_kind_op_log_cross_scope_reopen_and_graph_drift() {
    let fixture = Fixture::new();
    let db = fixture.path("stab.sqlite");
    seed_provenanced_store(&db);

    // Register an AppendOnlyLog collection for operational_mutations stability check.
    {
        let eng = Engine::open(
            &db,
            OpenOptions::default().with_scope_policy(ScopePolicy::single("alpha")),
        )
        .unwrap();
        eng.write(&[fathomdb_sdk::PreparedWrite::AdminSchema {
            name: "mcp_audit".into(),
            kind: "append_only_log".into(),
            schema_json: r#"{"type":"object"}"#.into(),
            retention_json: "{}".into(),
        }])
        .unwrap();
        eng.close().unwrap();
    }

    let pinned_as_of = 1_900_000_000_i64;
    let harness = Harness::start(fixture.config(json!({
        "stores": {
            "alpha_store": {"path": "stab.sqlite", "allow_writes": true, "scope": "alpha"}
        },
        "idle_timeout_secs": 1
    })))
    .await;

    // 1. Mint a kind="note" frozen context in alpha_store via MCP search(freeze=true).
    let s1 = harness
        .ok(
            "search",
            json!({
                "store": "alpha_store",
                "query": "river",
                "kind": "note",
                "as_of": pinned_as_of,
                "limit": 10,
                "freeze": true
            }),
        )
        .await;
    let epoch_before = s1["epoch"].as_u64().expect("epoch");
    let frozen_handle = s1["frozen_context"].as_str().expect("frozen_context").to_string();
    let src_hit =
        s1["results"].as_array().unwrap().iter().find(|h| h["ref"] == "l:alpha:src").unwrap();
    let ev_ref = src_hit["evidence_ref"].as_str().unwrap().to_string();

    // Expand under same frozen_handle to capture a graph_evidence_ref before mutations.
    let exp1 = harness
        .ok(
            "expand",
            json!({
                "store": "alpha_store",
                "refs": ["l:alpha:src"],
                "frozen_context": &frozen_handle,
                "include_evidence_refs": true
            }),
        )
        .await;
    let graph_ev_ref = exp1["targets"][0]["graph_evidence_ref"].as_str().unwrap().to_string();

    // 2. Disjoint-kind actuation ("AppTurn") inside alpha_store MUST NOT drift kind="note"!
    harness
        .ok(
            "actuate",
            json!({
                "store": "alpha_store",
                "operation_id": "op-mcp-disjoint-turn",
                "operations": [{
                    "op": "put_canonical_node",
                    "kind": "AppTurn",
                    "body": "disjoint turn written via MCP",
                    "source_id": "src:alpha-turns",
                    "logical_id": "alpha:turn:1",
                    "artifact_revision_id": "rev-alpha-turn-1",
                    "source_version_id": "sv-alpha-turn-1"
                }]
            }),
        )
        .await;

    // Reusing frozen_handle across search, fetch, and expand MUST succeed with 0 drift!
    let s_after_disjoint = harness
        .ok(
            "search",
            json!({
                "store": "alpha_store",
                "query": "river",
                "kind": "note",
                "frozen_context": &frozen_handle,
                "limit": 10
            }),
        )
        .await;
    assert_eq!(s_after_disjoint["results"].as_array().unwrap().len(), 2);

    let f_after_disjoint = harness
        .ok(
            "fetch",
            json!({
                "store": "alpha_store",
                "frozen_context": &frozen_handle,
                "evidence_refs": [&ev_ref],
                "graph_evidence_refs": [&graph_ev_ref]
            }),
        )
        .await;
    assert!(f_after_disjoint["items"][0]["record"].is_object());
    assert!(f_after_disjoint["items"][1]["record"].is_object());

    // 3. Let alpha_store idle-expire & close (1s idle timeout, 1s sweep interval), then run:
    //    - operational_mutations AppendOnlyLog append in alpha,
    //    - cross-scope "beta" additive write, transition, purge, and erase_source!
    let eng_a = {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            match Engine::open(
                &db,
                OpenOptions::default().with_scope_policy(ScopePolicy::single("alpha")),
            ) {
                Ok(eng) => break eng,
                Err(_) if std::time::Instant::now() < deadline => {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                Err(err) => panic!("expected alpha_store idle close within 5s, got {err:?}"),
            }
        }
    };
    {
        eng_a
            .write(&[fathomdb_sdk::PreparedWrite::OpStore {
                collection: "mcp_audit".into(),
                record_key: "evt-1".into(),
                schema_id: None,
                body: r#"{"ok":true}"#.into(),
            }])
            .unwrap();
        eng_a.close().unwrap();
    }
    {
        let eng_b = Engine::open(
            &db,
            OpenOptions::default().with_scope_policy(ScopePolicy::single("beta")),
        )
        .unwrap();
        eng_b
            .write(&[
                fathomdb_sdk::PreparedWrite::Node {
                    kind: "note".into(),
                    body: "beta additive note".into(),
                    source_id: SourceId::new("src:beta-del").unwrap(),
                    logical_id: Some("beta:del".into()),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    properties: None,
                },
                fathomdb_sdk::PreparedWrite::Node {
                    kind: "note".into(),
                    body: "beta erasable note".into(),
                    source_id: SourceId::new("src:beta-erase").unwrap(),
                    logical_id: Some("beta:erase".into()),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    properties: None,
                },
            ])
            .unwrap();
        eng_b
            .transition("beta:del", fathomdb_sdk::LifecycleState::Deleted, Some("cleanup"))
            .unwrap();
        eng_b.purge("beta:del").unwrap();
        eng_b.erase_source("src:beta-erase").unwrap();
        eng_b.close().unwrap();
    }

    // 4. Reopen alpha_store via MCP (epoch incremented!): reused frozen_handle MUST survive
    //    across idle close/reopen AND cross-scope destructive ops AND op_log appends!
    let f_after_reopen = harness
        .ok(
            "fetch",
            json!({
                "store": "alpha_store",
                "frozen_context": &frozen_handle,
                "evidence_refs": [&ev_ref],
                "graph_evidence_refs": [&graph_ev_ref]
            }),
        )
        .await;
    assert!(
        f_after_reopen["epoch"].as_u64().unwrap() > epoch_before,
        "expected store reopen epoch bump after idle timeout"
    );
    assert!(f_after_reopen["items"][0]["record"].is_object());
    assert!(f_after_reopen["items"][1]["record"].is_object());

    // 5. Same-scope disjoint-kind edge write in alpha_store:
    //    - node-only `fetch(evidence_refs)` with kind="note" STILL succeeds!
    //    - `fetch(graph_evidence_refs)` MUST escalate to whole-call `conflict/state_drift`!
    //    - `expand(frozen_context)` MUST return `conflict/state_drift`!
    harness
        .ok(
            "actuate",
            json!({
                "store": "alpha_store",
                "operation_id": "op-mcp-alpha-edge-drift",
                "operations": [{
                    "op": "put_derived_edge",
                    "kind": "supports",
                    "from": "alpha:src",
                    "to": "alpha:dst",
                    "source_id": "src:alpha",
                    "logical_id": "alpha:edge:2",
                    "artifact_revision_id": "rev-alpha-edge-2",
                    "source_version_id": "sv-alpha-1",
                    "source_revision_id": "rev-alpha-src",
                    "canonical_source_hash": sha256_hex("exact UTF-8 evidence body: river crossing é😀\nline two".as_bytes())
                }]
            }),
        )
        .await;

    let f_node_still_ok = harness
        .ok(
            "fetch",
            json!({
                "store": "alpha_store",
                "frozen_context": &frozen_handle,
                "evidence_refs": [&ev_ref]
            }),
        )
        .await;
    assert!(f_node_still_ok["items"][0]["record"].is_object());

    let g_drift = harness
        .err(
            "fetch",
            json!({
                "store": "alpha_store",
                "frozen_context": &frozen_handle,
                "graph_evidence_refs": [&graph_ev_ref]
            }),
        )
        .await;
    assert_eq!(g_drift["class"], "conflict");
    assert_eq!(g_drift["code"], "state_drift");

    let exp_drift = harness
        .err(
            "expand",
            json!({
                "store": "alpha_store",
                "refs": ["l:alpha:src"],
                "frozen_context": &frozen_handle
            }),
        )
        .await;
    assert_eq!(exp_drift["class"], "conflict");
    assert_eq!(exp_drift["code"], "state_drift");

    harness.shutdown().await;
}
