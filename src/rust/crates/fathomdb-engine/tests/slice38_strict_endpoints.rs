//! 0.8.31 Slice 30 (`WP-30A`) — strict write-time endpoint integrity
//! (`AC31-30-01`, `AC31-30-02`).
//!
//! Verifies that `Engine::write` and `Engine::ingest_with_extractor` enforce
//! active, non-superseded endpoint existence (`superseded_at IS NULL AND state = 'active'`)
//! atomically across every edge in a batch (including in-batch superseded edges
//! under both G0 `logical_id` and G11 `(from_id, to_id, kind)` rules), while
//! preserving `Engine::actuate` deferred simulation refusal semantics
//! (`ActuationOutcomeV1::Refused { reason: ReferenceUnavailable }`), same-batch
//! forward references, active self-loops, and validation error precedence.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ActuationOutcomeV1, ActuationRefusalReasonV1,
    ArtifactRevisionId, CanonicalHash, Engine, EngineError, ExtractDocument, InitialState,
    LifecycleActuationV1, LifecycleState, PreparedWrite, ProjectionRole, ProjectionSpec,
    PropertyEntityClass, PropertyValue, ProvenanceErrorReason, ProvenancedEdgeV1,
    ProvenancedNodeV1, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
    WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn db_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn node(kind: &str, body: &str, logical_id: Option<&str>, state: InitialState) -> PreparedWrite {
    PreparedWrite::Node {
        kind: kind.to_string(),
        body: body.to_string(),
        source_id: SourceId::new("test:strict-endpoints").unwrap(),
        logical_id: logical_id.map(str::to_string),
        state,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
    }
}

fn node_with_props(
    kind: &str,
    body: &str,
    logical_id: Option<&str>,
    state: InitialState,
    tag: &str,
) -> PreparedWrite {
    PreparedWrite::Node {
        kind: kind.to_string(),
        body: body.to_string(),
        source_id: SourceId::new("test:strict-endpoints").unwrap(),
        logical_id: logical_id.map(str::to_string),
        state,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: Some(BTreeMap::from([("tag".to_string(), PropertyValue::String(tag.into()))])),
    }
}

fn edge(
    kind: &str,
    from: &str,
    to: &str,
    logical_id: Option<&str>,
    body: Option<&str>,
) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: kind.to_string(),
        from: from.to_string(),
        to: to.to_string(),
        source_id: SourceId::new("test:strict-endpoints").unwrap(),
        logical_id: logical_id.map(str::to_string),
        body: body.map(str::to_string),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        properties: Some(BTreeMap::from([(
            "edge_tag".to_string(),
            PropertyValue::String("e".into()),
        )])),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct TableCounts {
    canonical_nodes: i64,
    canonical_edges: i64,
    artifact_revisions: i64,
    property_values: i64,
    search_index: i64,
    search_index_edges: i64,
    projection_terminal: i64,
    projection_state_rows: i64,
}

fn snapshot_counts(path: &Path) -> TableCounts {
    let conn = Connection::open(path).expect("open sqlite for count snapshot");
    let count = |sql: &str| -> i64 { conn.query_row(sql, [], |r| r.get(0)).expect(sql) };
    TableCounts {
        canonical_nodes: count("SELECT COUNT(*) FROM canonical_nodes"),
        canonical_edges: count("SELECT COUNT(*) FROM canonical_edges"),
        artifact_revisions: count("SELECT COUNT(*) FROM _fathomdb_artifact_revisions"),
        property_values: count("SELECT COUNT(*) FROM canonical_property_values"),
        search_index: count("SELECT COUNT(*) FROM search_index"),
        search_index_edges: count("SELECT COUNT(*) FROM search_index_edges"),
        projection_terminal: count("SELECT COUNT(*) FROM _fathomdb_projection_terminal"),
        projection_state_rows: count("SELECT COUNT(*) FROM _fathomdb_projection_state"),
    }
}

fn zero_counts() -> TableCounts {
    TableCounts {
        canonical_nodes: 0,
        canonical_edges: 0,
        artifact_revisions: 0,
        property_values: 0,
        search_index: 0,
        search_index_edges: 0,
        projection_terminal: 0,
        projection_state_rows: 0,
    }
}

fn configure_tag_projections(engine: &Engine) {
    engine
        .configure_projections(
            &[
                ProjectionSpec {
                    name: "tag".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
                ProjectionSpec {
                    name: "edge_tag".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Edge,
                },
            ],
            &[],
        )
        .expect("configure projections");
}

fn assert_atomic_rejection_on_empty_store(name: &str, batch: Vec<PreparedWrite>) {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, name);
    {
        let opened = Engine::open(&path).expect("open");
        configure_tag_projections(&opened.engine);
        let err = opened
            .engine
            .write(&batch)
            .expect_err("strict write must reject batch with dangling/inactive edge endpoint");
        assert!(
            matches!(err, EngineError::WriteValidation),
            "case `{name}` expected EngineError::WriteValidation, got {err:?}"
        );
        opened.engine.close().expect("close");
    }
    assert_eq!(
        snapshot_counts(&path),
        zero_counts(),
        "case `{name}` must roll back all canonical, revision, property, FTS, and projection rows"
    );
    // Reopen to prove clean WAL/recovery state with zero persisted rows.
    {
        let reopened = Engine::open(&path).expect("reopen");
        assert_eq!(reopened.engine.search_text_only("alpha").unwrap().results.len(), 0);
        reopened.engine.close().expect("close reopened");
    }
    assert_eq!(
        snapshot_counts(&path),
        zero_counts(),
        "case `{name}` must still have zero rows after reopen"
    );
}

#[test]
fn strict_write_rejects_missing_from_and_to_with_atomic_rollback() {
    // Missing `to` with an accompanying active node (with body + Filterable property).
    assert_atomic_rejection_on_empty_store(
        "missing_to",
        vec![
            node_with_props("doc", "alpha node body", Some("A"), InitialState::Active, "t1"),
            edge("rel", "A", "MISSING_TO", Some("E1"), Some("alpha edge body")),
        ],
    );

    // Missing `from` with an accompanying active node.
    assert_atomic_rejection_on_empty_store(
        "missing_from",
        vec![
            node_with_props("doc", "alpha node body", Some("B"), InitialState::Active, "t2"),
            edge("rel", "MISSING_FROM", "B", Some("E2"), Some("alpha edge body")),
        ],
    );

    // Both missing.
    assert_atomic_rejection_on_empty_store(
        "missing_both",
        vec![edge("rel", "NO_A", "NO_B", None, Some("alpha edge body"))],
    );
}

#[test]
fn strict_write_rejects_pending_initial_state_and_in_batch_active_then_pending() {
    // Endpoint written with InitialState::Pending.
    assert_atomic_rejection_on_empty_store(
        "initial_pending_to",
        vec![
            node_with_props("doc", "alpha active", Some("A"), InitialState::Active, "t1"),
            node_with_props("doc", "alpha pending", Some("P"), InitialState::Pending, "t2"),
            edge("rel", "A", "P", Some("E_PEND"), Some("alpha edge body")),
        ],
    );

    // Endpoint written Active first, edge references it, then same-batch node
    // supersedes it into InitialState::Pending.
    assert_atomic_rejection_on_empty_store(
        "in_batch_active_then_pending",
        vec![
            node_with_props("doc", "alpha active", Some("A"), InitialState::Active, "t1"),
            node_with_props("doc", "alpha active b", Some("B"), InitialState::Active, "t2"),
            edge("rel", "A", "B", Some("E_FLIP"), Some("alpha edge body")),
            node_with_props("doc", "alpha now pending", Some("B"), InitialState::Pending, "t3"),
        ],
    );
}

#[test]
fn strict_write_rejects_existing_pending_and_transitioned_deleted_and_superseded_only_endpoints() {
    // Case 1: pre-existing Pending node, Case 2: Active -> Deleted, Case 3: Pending -> Deleted.
    for (case_name, initial_state, do_delete) in [
        ("existing_pending", InitialState::Pending, false),
        ("transitioned_active_to_deleted", InitialState::Active, true),
        ("transitioned_pending_to_deleted", InitialState::Pending, true),
    ] {
        let dir = TempDir::new().unwrap();
        let path = db_path(&dir, case_name);
        let baseline = {
            let opened = Engine::open(&path).expect("open");
            configure_tag_projections(&opened.engine);
            opened
                .engine
                .write(&[
                    node_with_props("doc", "seed a", Some("A"), InitialState::Active, "a"),
                    node_with_props("doc", "seed b", Some("B"), initial_state, "b"),
                ])
                .expect("seed nodes");
            if do_delete {
                opened
                    .engine
                    .transition("B", LifecycleState::Deleted, Some("lifecycle test".to_string()))
                    .expect("transition to deleted");
            }
            opened.engine.close().expect("close after setup");
            snapshot_counts(&path)
        };

        {
            let opened = Engine::open(&path).expect("reopen for failing batch");
            let err = opened
                .engine
                .write(&[
                    node_with_props(
                        "doc",
                        "rollback witness",
                        Some("W"),
                        InitialState::Active,
                        "w",
                    ),
                    edge("rel", "A", "B", Some("E_INACTIVE"), Some("edge rollback witness")),
                ])
                .expect_err("must reject edge to non-active endpoint");
            assert!(matches!(err, EngineError::WriteValidation), "case `{case_name}` got {err:?}");
            opened.engine.close().expect("close after rejected write");
        }

        assert_eq!(
            snapshot_counts(&path),
            baseline,
            "failed batch in `{case_name}` must leave store counts unchanged"
        );
    }

    // Superseded-only node (tombstoned via raw SQL, zero active rows remain).
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "superseded_only");
    {
        let opened = Engine::open(&path).expect("open");
        configure_tag_projections(&opened.engine);
        opened
            .engine
            .write(&[
                node_with_props("doc", "live a", Some("A"), InitialState::Active, "a"),
                node_with_props("doc", "superseded s", Some("S"), InitialState::Active, "s"),
            ])
            .expect("seed");
        opened.engine.close().expect("close");
    }
    {
        let conn = Connection::open(&path).expect("open raw sqlite");
        let changed = conn
            .execute(
                "UPDATE canonical_nodes SET superseded_at = 999
                 WHERE logical_id = 'S' AND superseded_at IS NULL",
                [],
            )
            .expect("tombstone S");
        assert_eq!(changed, 1);
    }
    // Reopen and close once so startup reconciliation settles after the raw SQL tombstone
    // before capturing the pre-write baseline.
    {
        let opened = Engine::open(&path).expect("reopen to settle");
        opened.engine.close().expect("close settled");
    }
    let baseline = snapshot_counts(&path);
    {
        let opened = Engine::open(&path).expect("reopen");
        let err = opened
            .engine
            .write(&[
                node_with_props("doc", "rollback witness", Some("W"), InitialState::Active, "w"),
                edge("rel", "A", "S", Some("E_SUP"), Some("edge witness")),
            ])
            .expect_err("must reject edge to superseded-only endpoint");
        assert!(matches!(err, EngineError::WriteValidation), "got {err:?}");
        opened.engine.close().expect("close");
    }
    assert_eq!(snapshot_counts(&path), baseline);
}

#[test]
fn strict_write_rejects_in_batch_g0_and_g11_superseded_dangling_edges() {
    // G0 logical_id supersession: earlier edge E points at GHOST, later edge E points at A -> A.
    // Strict mode checks EVERY edge in the batch and rejects the dangling earlier edge.
    assert_atomic_rejection_on_empty_store(
        "g0_superseded_dangling",
        vec![
            node_with_props("doc", "alpha a", Some("A"), InitialState::Active, "a"),
            edge("rel", "A", "GHOST", Some("E"), Some("alpha v1")),
            edge("rel", "A", "A", Some("E"), Some("alpha v2")),
        ],
    );

    // G11 (from_id, to_id, kind) supersession: earlier edge on (A, GHOST, rel) superseded
    // by a later fact-edge on the same triple (or vice-versa). Both are checked and rejected.
    assert_atomic_rejection_on_empty_store(
        "g11_superseded_dangling",
        vec![
            node_with_props("doc", "alpha a", Some("A"), InitialState::Active, "a"),
            edge("rel", "A", "GHOST", Some("E1"), None),
            edge("rel", "A", "GHOST", Some("E2"), Some("alpha g11 superseding")),
        ],
    );
}

#[test]
fn strict_write_allows_same_batch_forward_references_and_active_self_loops() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "valid_batches");
    let opened = Engine::open(&path).expect("open");

    // Same-batch [Edge, Node(from), Node(to)] + active self-loop.
    let receipt = opened
        .engine
        .write(&[
            edge("rel", "N1", "N2", Some("E_FWD"), Some("forward edge")),
            edge("self_rel", "N1", "N1", Some("E_SELF"), None),
            node("doc", "n1", Some("N1"), InitialState::Active),
            node("doc", "n2", Some("N2"), InitialState::Active),
        ])
        .expect("same-batch forward refs and active self-loop must commit");
    assert_eq!(receipt.dangling_edge_endpoints, 0);
    opened.engine.close().expect("close");
}

#[test]
fn strict_write_preserves_syntactic_and_provenance_validation_precedence() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(db_path(&dir, "precedence")).expect("open");

    // 1. Syntactic failure (empty kind) with dangling endpoints still yields WriteValidation.
    let err = opened
        .engine
        .write(&[edge("", "MISSING_A", "MISSING_B", None, None)])
        .expect_err("empty kind must fail");
    assert!(matches!(err, EngineError::WriteValidation));

    // 2. Provenance failure (SourceRevisionMissing) on a ProvenancedEdge with dangling endpoints
    // must still fail with EngineError::Provenance, NOT WriteValidation.
    let prov_edge = PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
        kind: "supports".into(),
        from: "MISSING_A".into(),
        to: "MISSING_B".into(),
        source_id: SourceId::new("source-bucket").unwrap(),
        logical_id: Some("prov-edge-1".into()),
        body: Some("edge body".into()),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new("edge-r1").unwrap(),
            SourceVersionId::new("v1").unwrap(),
            SourceRevisionId::new("nonexistent-source-rev").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest("anything")).unwrap(),
        ),
        properties: None,
    });
    let err = opened.engine.write(&[prov_edge]).expect_err("missing source revision must fail");
    assert!(
        matches!(
            err,
            EngineError::Provenance(ref p) if p.reason == ProvenanceErrorReason::SourceRevisionMissing
        ),
        "expected Provenance(SourceRevisionMissing), got {err:?}"
    );
    opened.engine.close().expect("close");
}

fn canonical_node(revision: &str, logical: &str, body: &str) -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("source-bucket").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new(format!("version-{revision}")).unwrap(),
        ),
        properties: None,
    }
}

fn derived_node(
    revision: &str,
    logical: &str,
    body: &str,
    source_rev: &str,
    source_body: &str,
) -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "fact".into(),
        body: body.into(),
        source_id: SourceId::new("source-bucket").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new(format!("version-{source_rev}")).unwrap(),
            SourceRevisionId::new(source_rev).unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest(source_body)).unwrap(),
        ),
        properties: None,
    }
}

fn derived_edge(
    revision: &str,
    logical: &str,
    from: &str,
    to: &str,
    source_rev: &str,
    source_body: &str,
) -> ProvenancedEdgeV1 {
    ProvenancedEdgeV1 {
        kind: "supports".into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new("source-bucket").unwrap(),
        logical_id: Some(logical.into()),
        body: Some("derived edge".into()),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new(format!("version-{source_rev}")).unwrap(),
            SourceRevisionId::new(source_rev).unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest(source_body)).unwrap(),
        ),
        properties: None,
    }
}

#[test]
fn actuate_preserves_deferred_simulation_for_edge_before_node_and_refused_dangling() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "actuate_seam");
    let opened = Engine::open(&path).expect("open");

    // Seed canonical source node first so derived artifacts can reference it.
    let seed = ActuationBatchV1::new(
        "op-seed",
        vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
            "src-r1",
            "source",
            "source body",
        ))],
    )
    .unwrap();
    assert_eq!(opened.engine.actuate(seed).unwrap().outcome, ActuationOutcomeV1::Committed);

    // 1. Same-batch [PutDerivedEdge, PutDerivedNode] MUST succeed (DeferredToActuationSimulation).
    let edge_before_node = ActuationBatchV1::new(
        "op-edge-before-node",
        vec![
            ActuationOperationV1::PutDerivedEdge(derived_edge(
                "edge-r1",
                "e-fwd",
                "source",
                "target",
                "src-r1",
                "source body",
            )),
            ActuationOperationV1::PutDerivedNode(derived_node(
                "target-r1",
                "target",
                "target body",
                "src-r1",
                "source body",
            )),
        ],
    )
    .unwrap();
    let ok_receipt = opened.engine.actuate(edge_before_node).expect("actuate edge-before-node");
    assert_eq!(ok_receipt.outcome, ActuationOutcomeV1::Committed);

    // 2. Dangling [PutDerivedEdge] returns ActuationOutcomeV1::Refused { ReferenceUnavailable }
    // with exact index and path (NOT Err(EngineError::WriteValidation)), and idempotent replay.
    let dangling = ActuationBatchV1::new(
        "op-dangling",
        vec![ActuationOperationV1::PutDerivedEdge(derived_edge(
            "edge-r2",
            "e-dangling",
            "source",
            "does-not-exist",
            "src-r1",
            "source body",
        ))],
    )
    .unwrap();
    let refused =
        opened.engine.actuate(dangling.clone()).expect("dangling actuate returns Ok(Refused)");
    assert_eq!(refused.outcome, ActuationOutcomeV1::Refused);
    assert_eq!(refused.reason_codes, vec![ActuationRefusalReasonV1::ReferenceUnavailable]);
    assert_eq!(refused.refused_operation_index, Some(0));
    assert_eq!(refused.refused_field_path.as_deref(), Some("/operations/0/record/to"));
    // Exact replay returns identical receipt.
    assert_eq!(opened.engine.actuate(dangling).unwrap(), refused);

    // 3. In-batch active-then-deleted endpoint in actuate is refused with ReferenceUnavailable.
    let active_then_deleted = ActuationBatchV1::new(
        "op-active-then-deleted",
        vec![
            ActuationOperationV1::PutDerivedNode(derived_node(
                "ephemeral-r1",
                "ephemeral",
                "ephemeral body",
                "src-r1",
                "source body",
            )),
            ActuationOperationV1::PutDerivedEdge(derived_edge(
                "edge-r3",
                "e-ephemeral",
                "source",
                "ephemeral",
                "src-r1",
                "source body",
            )),
            ActuationOperationV1::TransitionLifecycle(
                LifecycleActuationV1::new(
                    "ephemeral",
                    ArtifactRevisionId::new("ephemeral-r1").unwrap(),
                    LifecycleState::Deleted,
                    Some("deleted in same batch".into()),
                )
                .unwrap(),
            ),
        ],
    )
    .unwrap();
    let del_refused = opened
        .engine
        .actuate(active_then_deleted)
        .expect("in-batch deleted endpoint returns Ok(Refused)");
    assert_eq!(del_refused.outcome, ActuationOutcomeV1::Refused);
    assert_eq!(del_refused.reason_codes, vec![ActuationRefusalReasonV1::ReferenceUnavailable]);
    assert_eq!(del_refused.refused_operation_index, Some(1));
    assert_eq!(del_refused.refused_field_path.as_deref(), Some("/operations/1/record/to"));

    opened.engine.close().expect("close");
}

fn write_stub_extractor_script(dir: &TempDir, name: &str, response_json: &str) -> Vec<String> {
    let script_path = dir.path().join(name);
    let script = format!(
        r#"import sys, json
payload = json.loads({response_json:?})
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    req = json.loads(line)
    msg_type = req.get('type')
    if msg_type == 'hello':
        resp = {{'protocol': 'fathomdb.extract.v1', 'type': 'ready', 'schema_version': 1, 'provider': 'stub', 'model': 'stub-v1', 'supports': {{}}, 'max_docs_per_request': 8}}
    elif msg_type == 'extract':
        resp = {{'protocol': 'fathomdb.extract.v1', 'type': 'result', 'request_id': req.get('request_id', 'r1'), 'entities': payload.get('entities', []), 'edges': payload.get('edges', []), 'warnings': []}}
    else:
        resp = {{'protocol': 'fathomdb.extract.v1', 'type': 'error', 'error_code': 'invalid_request', 'detail': str(msg_type)}}
    print(json.dumps(resp), flush=True)
"#
    );
    std::fs::write(&script_path, script).expect("write stub extractor");
    vec!["python3".to_string(), script_path.to_string_lossy().into_owned()]
}

#[test]
fn ingest_with_extractor_rejects_unlisted_and_deactivated_endpoints_atomically() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "extractor_strict");
    let opened = Engine::open(&path).expect("open");

    // 1. Valid extraction with both endpoints listed succeeds.
    let valid_cmd = write_stub_extractor_script(
        &dir,
        "valid.py",
        r#"{
            "entities": [
                {"name": "Alice", "type": "entity", "source_doc_id": "doc:1"},
                {"name": "Bob", "type": "entity", "source_doc_id": "doc:1"}
            ],
            "edges": [
                {"from_entity": "Alice", "to_entity": "Bob", "relation": "knows", "source_doc_id": "doc:1"}
            ]
        }"#,
    );
    let valid_refs: Vec<&str> = valid_cmd.iter().map(String::as_str).collect();
    let docs =
        vec![ExtractDocument { source_doc_id: "doc:1".into(), body: "Alice knows Bob".into() }];
    let ok = opened
        .engine
        .ingest_with_extractor(&valid_refs, &docs)
        .expect("valid extraction must succeed");
    assert_eq!(ok.nodes_written, 2);
    assert_eq!(ok.edges_written, 1);

    // 2. Unlisted endpoint in extractor output fails tx2 atomically with WriteValidation.
    let unlisted_cmd = write_stub_extractor_script(
        &dir,
        "unlisted.py",
        r#"{
            "entities": [
                {"name": "Charlie", "type": "entity", "source_doc_id": "doc:2"}
            ],
            "edges": [
                {"from_entity": "Charlie", "to_entity": "GhostEntity", "relation": "knows", "source_doc_id": "doc:2"}
            ]
        }"#,
    );
    let unlisted_refs: Vec<&str> = unlisted_cmd.iter().map(String::as_str).collect();
    let docs2 = vec![ExtractDocument {
        source_doc_id: "doc:2".into(),
        body: "Charlie knows GhostEntity".into(),
    }];
    let err = opened
        .engine
        .ingest_with_extractor(&unlisted_refs, &docs2)
        .expect_err("unlisted endpoint must fail tx2 with WriteValidation");
    assert!(matches!(err, EngineError::WriteValidation), "got {err:?}");

    // 3. Deactivated endpoint (Bob transitioned to Deleted before doc:3 references Alice -> Bob
    // without re-creating Bob) fails tx2 atomically with WriteValidation.
    let bob_lid: String = Connection::open(&path)
        .unwrap()
        .query_row(
            "SELECT logical_id FROM canonical_nodes WHERE body = 'Bob' AND superseded_at IS NULL",
            [],
            |r| r.get(0),
        )
        .expect("find Bob logical_id");
    opened
        .engine
        .transition(&bob_lid, LifecycleState::Deleted, Some("deactivated".to_string()))
        .unwrap();

    let deactivated_cmd = write_stub_extractor_script(
        &dir,
        "deactivated.py",
        r#"{
            "entities": [],
            "edges": [
                {"from_entity": "Alice", "to_entity": "Bob", "relation": "mentors", "source_doc_id": "doc:3"}
            ]
        }"#,
    );
    let deactivated_refs: Vec<&str> = deactivated_cmd.iter().map(String::as_str).collect();
    let docs3 =
        vec![ExtractDocument { source_doc_id: "doc:3".into(), body: "Alice mentors Bob".into() }];
    let err = opened
        .engine
        .ingest_with_extractor(&deactivated_refs, &docs3)
        .expect_err("deactivated endpoint must fail tx2 with WriteValidation");
    assert!(matches!(err, EngineError::WriteValidation), "got {err:?}");

    opened.engine.close().expect("close");

    // Verify only the single valid edge from doc:1 is persisted.
    let conn = Connection::open(&path).expect("open sqlite");
    let edge_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM canonical_edges", [], |r| r.get(0)).unwrap();
    assert_eq!(edge_count, 1, "neither failed tx2 extraction may persist any edge row");
}
