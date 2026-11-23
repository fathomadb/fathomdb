//! 0.8.31 Slice 20 (`PROPERTY-SCHEMA`) — Layer 3 RED/GREEN test suite for
//! inline scalar `properties`, O(1) read hydration, validation atomicity,
//! `canonical_property_values` lifecycle & `vec0` 8-slot budget, exact
//! pre-KNN/pre-selection recall, relaxed `ReadView` parity, frozen-context
//! binding, and monotonic `_fathomdb_commit_timestamps`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    ActuationBatchV1, ActuationErrorReason, ActuationOperationV1, ActuationOutcomeV1,
    ArtifactRevisionId, CanonicalHash, DataPlaneIntegrityCheckV1, DataPlaneIntegrityRequestV1,
    Engine, EngineError, EvidenceResolveRequestV1, EvidenceSearchRequestV1, Filter, FilterTerm,
    FrozenReadErrorReason, GraphEvidenceArtifactV1, GraphEvidenceResolveRequestV1,
    GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, IdSpace, InitialState,
    LifecycleActuationV1, LifecycleState, PageRequestV1, PreparedWrite, ProjectionFts,
    ProjectionRole, ProjectionSpec, ProjectionVector, PropertyComparisonOp, PropertyEntityClass,
    PropertyMap, PropertyPredicate, PropertyValue, ProvenancedEdgeV1, ProvenancedNodeV1,
    ReadContextV1, ReadView, SearchFilter, SourceDependencyRegistrationV1, SourceId, SourceLocator,
    SourceRevisionId, SourceVersionId, TraversalDirection, WriteProvenanceV1, TOP_K_BIT_CANDIDATES,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const FROZEN_NOW: i64 = 1_800_000_000;

#[derive(Clone, Debug)]
struct RecordingEmbedder {
    identity: EmbedderIdentity,
    seen_texts: Arc<Mutex<Vec<String>>>,
}

impl RecordingEmbedder {
    fn new() -> Self {
        Self {
            identity: EmbedderIdentity::new("slice38-recording", "v1", 8),
            seen_texts: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Embedder for RecordingEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        self.identity.clone()
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        self.seen_texts.lock().expect("lock").push(text.to_string());
        let mut vector = vec![0.0; 8];
        if text.contains("alpha_cluster") {
            vector[0] = 1.0;
        } else if text.contains("beta_cluster") {
            vector[1] = 1.0;
        } else {
            vector[2] = 1.0;
        }
        // Deterministic slight perturbation based on length so vectors are non-trivial.
        vector[7] = (text.len() % 17) as f32 * 0.01;
        Ok(vector)
    }
}

fn db_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn sha256_hex(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn filterable_spec(name: &str, entity_class: PropertyEntityClass) -> ProjectionSpec {
    ProjectionSpec {
        name: name.to_string(),
        roles: BTreeSet::from([ProjectionRole::Filterable]),
        fts: None,
        vector: None,
        source: None,
        entity_class,
    }
}

fn rich_properties() -> PropertyMap {
    let mut map = PropertyMap::new();
    map.insert("title_prop".into(), PropertyValue::String("alpha-omega".into()));
    map.insert("i64_min".into(), PropertyValue::Int(i64::MIN));
    map.insert("i64_max".into(), PropertyValue::Int(i64::MAX));
    map.insert("gt_2p53".into(), PropertyValue::Int((1_i64 << 53) + 1));
    map.insert("lt_neg_2p53".into(), PropertyValue::Int(-(1_i64 << 53) - 1));
    map.insert("ratio".into(), PropertyValue::Float(12.25));
    map.insert("neg_zero".into(), PropertyValue::Float(-0.0));
    map.insert("verified".into(), PropertyValue::Bool(true));
    map
}

fn normalized_expected_properties() -> PropertyMap {
    let mut expected = rich_properties();
    expected.insert("neg_zero".into(), PropertyValue::Float(0.0));
    expected
}

fn open_with_embedder(path: &Path) -> (Engine, Arc<Mutex<Vec<String>>>) {
    let embedder = Arc::new(RecordingEmbedder::new());
    let seen = Arc::clone(&embedder.seen_texts);
    let opened = Engine::open_with_embedder_for_test(path, embedder).expect("open");
    (opened.engine, seen)
}

#[test]
fn slice38_node_and_edge_property_round_trip_and_o1_hydration() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "roundtrip");
    let (engine, seen_embedder_texts) = open_with_embedder(&path);

    let props = rich_properties();
    let expected = normalized_expected_properties();

    // Verify PropertyValue::Float(-0.0) serialization normalization directly on expected values.
    let source_body = "plain_body_source alpha_cluster token";
    let derived_body = "plain_body_derived alpha_cluster token";
    let mut edge_props = PropertyMap::new();
    edge_props.insert("weight_exact".into(), PropertyValue::Int((1_i64 << 53) + 1));
    edge_props.insert("rel_label".into(), PropertyValue::String("direct-support".into()));
    edge_props.insert("active_flag".into(), PropertyValue::Bool(true));

    // Include an empty property map that must normalize to SQL NULL and read back as None.
    let empty_props = BTreeMap::new();

    // Include secret property strings NOT declared as searchable to confirm FTS & embedder exclusion.
    let mut secret_props = props.clone();
    secret_props.insert("zyxpropkeyalpha".into(), PropertyValue::String("zyxpropvalomega".into()));
    let mut expected_secret = expected.clone();
    expected_secret
        .insert("zyxpropkeyalpha".into(), PropertyValue::String("zyxpropvalomega".into()));

    engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "note".into(),
                body: source_body.into(),
                source_id: SourceId::new("src:rt").unwrap(),
                logical_id: Some("n-prov".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("rev-n-prov").unwrap(),
                    SourceVersionId::new("ver-rt-1").unwrap(),
                ),
                properties: Some(secret_props.clone()),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "note".into(),
                body: derived_body.into(),
                source_id: SourceId::new("src:rt").unwrap(),
                logical_id: Some("n-derived".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("rev-n-derived").unwrap(),
                    SourceVersionId::new("ver-rt-1").unwrap(),
                    SourceRevisionId::new("rev-n-prov").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(sha256_hex(source_body)).unwrap(),
                ),
                properties: Some(props.clone()),
            }),
            PreparedWrite::Node {
                kind: "note".into(),
                body: "empty_props_node alpha_cluster".into(),
                source_id: SourceId::new("src:rt").unwrap(),
                logical_id: Some("n-empty".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: Some(empty_props),
            },
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                kind: "supports".into(),
                from: "n-prov".into(),
                to: "n-derived".into(),
                source_id: SourceId::new("src:rt").unwrap(),
                logical_id: Some("e-prov".into()),
                body: Some("edge evidence body".into()),
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.9),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("rev-e-prov").unwrap(),
                    SourceVersionId::new("ver-rt-1").unwrap(),
                    SourceRevisionId::new("rev-n-prov").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(sha256_hex(source_body)).unwrap(),
                ),
                properties: Some(edge_props.clone()),
            }),
        ])
        .expect("write provenanced nodes/edge");

    // Also verify actuate round-trip + dependency for evidence resolution.
    let mut act_node_props = PropertyMap::new();
    act_node_props.insert("act_tag".into(), PropertyValue::String("via-actuate".into()));
    act_node_props.insert("act_num".into(), PropertyValue::Int(42));
    let mut act_edge_props = PropertyMap::new();
    act_edge_props.insert("act_edge".into(), PropertyValue::Bool(false));

    let actuate_batch = ActuationBatchV1::new(
        "op-rt-1",
        vec![
            ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
                kind: "note".into(),
                body: "actuate target alpha_cluster".into(),
                source_id: SourceId::new("src:rt").unwrap(),
                logical_id: Some("n-act".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("rev-n-act").unwrap(),
                    SourceVersionId::new("ver-rt-2").unwrap(),
                ),
                properties: Some(act_node_props.clone()),
            }),
            ActuationOperationV1::PutDerivedEdge(ProvenancedEdgeV1 {
                kind: "links".into(),
                from: "n-prov".into(),
                to: "n-act".into(),
                source_id: SourceId::new("src:rt").unwrap(),
                logical_id: Some("e-act".into()),
                body: Some("act edge body".into()),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("rev-e-act").unwrap(),
                    SourceVersionId::new("ver-rt-2").unwrap(),
                    SourceRevisionId::new("rev-n-act").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(sha256_hex("actuate target alpha_cluster")).unwrap(),
                ),
                properties: Some(act_edge_props.clone()),
            }),
            ActuationOperationV1::RegisterSourceDependency(
                SourceDependencyRegistrationV1::new("dep-n-derived", "rev-n-prov", "rev-n-derived")
                    .unwrap(),
            ),
            ActuationOperationV1::RegisterSourceDependency(
                SourceDependencyRegistrationV1::new("dep-e-prov", "rev-n-prov", "rev-e-prov")
                    .unwrap(),
            ),
        ],
    )
    .unwrap();
    engine.actuate(actuate_batch).expect("actuate roundtrip");

    engine.drain(15_000).expect("drain");

    // 1. NodeRecord via read_get, read_list, read_canonical_page
    let got_prov =
        engine.read_get("n-prov", &ReadView::default()).expect("read_get").expect("present");
    assert_eq!(got_prov.properties.as_ref(), Some(&expected_secret), "read_get must hydrate");
    // Verify exact float normalization (-0.0 -> +0.0, bit pattern sign bit clear)
    match got_prov.properties.as_ref().unwrap().get("neg_zero") {
        Some(PropertyValue::Float(v)) => {
            assert_eq!(*v, 0.0);
            assert!(v.is_sign_positive(), "-0.0 must normalize to +0.0");
        }
        other => panic!("expected Float(0.0), got {other:?}"),
    }

    let got_empty =
        engine.read_get("n-empty", &ReadView::default()).expect("read_get empty").expect("present");
    assert_eq!(got_empty.properties, None, "Some(empty_map) must read back as None");

    let list_rows = engine.read_list("note", &[], 20, &ReadView::default()).expect("read_list");
    let list_act = list_rows.iter().find(|r| r.logical_id == "n-act").expect("n-act in read_list");
    assert_eq!(list_act.properties.as_ref(), Some(&act_node_props));

    let context = ReadContextV1::new(
        ReadView { valid_as_of: Some(FROZEN_NOW), ..ReadView::default() },
        SearchFilter::default(),
    )
    .unwrap();
    let frozen = engine.freeze_read_context(&context).expect("freeze");
    let page = engine
        .read_canonical_page(
            "note",
            &frozen,
            &PageRequestV1 { schema_version: 1, limit: 20, cursor: None },
        )
        .expect("read_canonical_page");
    let page_derived = page.items.iter().find(|r| r.logical_id == "n-derived").expect("n-derived");
    assert_eq!(page_derived.properties.as_ref(), Some(&expected));

    // 2. SearchHit via search & search_text_only & search_frozen
    let hybrid_hits = engine.search("alpha_cluster").expect("search");
    let hit_prov =
        hybrid_hits.results.iter().find(|h| h.id == IdSpace::logical("n-prov")).expect("hit");
    assert_eq!(hit_prov.properties.as_ref(), Some(&expected_secret), "SearchHit must hydrate");

    let text_hits = engine.search_text_only("plain_body_derived").expect("search_text_only");
    let hit_derived =
        text_hits.results.iter().find(|h| h.id == IdSpace::logical("n-derived")).expect("hit");
    assert_eq!(hit_derived.properties.as_ref(), Some(&expected));

    // 3. FTS & embedding exclusion for non-searchable properties
    let secret_val_hits = engine.search_text_only("zyxpropvalomega").expect("secret val search");
    assert!(
        secret_val_hits.results.is_empty(),
        "body FTS must not contain property values, got {:?}",
        secret_val_hits.results
    );
    let secret_key_hits = engine.search_text_only("zyxpropkeyalpha").expect("secret key search");
    assert!(
        secret_key_hits.results.is_empty(),
        "body FTS must not contain property keys, got {:?}",
        secret_key_hits.results
    );
    {
        let texts = seen_embedder_texts.lock().unwrap();
        assert!(
            texts.iter().all(|t| !t.contains("zyxpropvalomega") && !t.contains("zyxpropkeyalpha")),
            "embedder must never receive undeclared/non-searchable property keys or values: {texts:?}"
        );
    }

    // 4. GraphTargetV1 & GraphOriginV1 via graph_expand + ResolvedGraphEvidenceV1
    let mut g_req = GraphExpandRequestV1::new(
        GraphSeedV1::Explicit { schema_version: 1, logical_ids: vec![IdSpace::logical("n-prov")] },
        TraversalDirection::Outgoing,
        GraphReadContextV1::Frozen { schema_version: 1, context: frozen.clone() },
        1,
        10,
        100,
    )
    .with_evidence(true);
    let g_res = engine.graph_expand(&g_req).expect("graph_expand");
    let g_target =
        g_res.targets.iter().find(|t| t.logical_id == "n-derived").expect("n-derived target");
    assert_eq!(g_target.properties.as_ref(), Some(&expected), "GraphTargetV1 must hydrate");
    assert_eq!(
        g_target.origin.terminal_edge_properties.as_ref(),
        Some(&edge_props),
        "GraphOriginV1.terminal_edge_properties must hydrate"
    );
    let g_act = g_res.targets.iter().find(|t| t.logical_id == "n-act").expect("n-act target");
    assert_eq!(g_act.properties.as_ref(), Some(&act_node_props));
    assert_eq!(g_act.origin.terminal_edge_properties.as_ref(), Some(&act_edge_props));

    let g_sidecar = g_res.evidence.expect("graph evidence sidecar");
    let entry_idx =
        g_res.targets.iter().position(|t| t.logical_id == "n-derived").expect("target idx");
    let entry = g_sidecar
        .entries
        .iter()
        .find(|e| e.target_index as usize == entry_idx)
        .expect("evidence entry");

    let resolved_target = engine
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: entry.target_evidence_ref.clone(),
            context: frozen.clone(),
        })
        .expect("resolve target graph evidence");
    assert_eq!(
        resolved_target.artifact,
        GraphEvidenceArtifactV1::Node {
            logical_id: "n-derived".into(),
            kind: "note".into(),
            body: derived_body.into(),
            properties: Some(expected.clone()),
        }
    );

    let resolved_edge = engine
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: entry.terminal_edge_evidence_ref.clone(),
            context: frozen.clone(),
        })
        .expect("resolve edge graph evidence");
    assert_eq!(
        resolved_edge.artifact,
        GraphEvidenceArtifactV1::Edge {
            logical_id: Some("e-prov".into()),
            kind: "supports".into(),
            body: Some("edge evidence body".into()),
            properties: Some(edge_props.clone()),
            from: "n-prov".into(),
            to: "n-derived".into(),
        }
    );

    // 5. ResolvedEvidenceV1 via search_with_evidence + resolve_evidence
    let ev_search = engine
        .search_with_evidence(&EvidenceSearchRequestV1 {
            schema_version: 1,
            query: "plain_body_derived".into(),
            context: frozen.clone(),
            rerank_depth: 0,
            use_graph_arm: false,
            alpha: 0.3,
            pool_n: 0,
            include_explanation: false,
            limit: 10,
        })
        .expect("search_with_evidence");
    let hit_idx = ev_search
        .search_result
        .results
        .iter()
        .position(|h| h.id == IdSpace::logical("n-derived"))
        .expect("hit in evidence search");
    let ev_entry = ev_search
        .evidence
        .iter()
        .find(|e| e.result_index as usize == hit_idx)
        .expect("evidence sidecar entry");
    let resolved_ev = engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: ev_entry.evidence_ref.clone(),
            context: frozen.clone(),
        })
        .expect("resolve_evidence");
    assert_eq!(resolved_ev.properties.as_ref(), Some(&expected), "ResolvedEvidenceV1 must hydrate");

    g_req.include_evidence = false;
    let _ = engine.graph_expand(&g_req).expect("graph_expand without evidence");
    engine.close().expect("close");

    // 6. Verify SQL NULL stored for empty property map & canonical properties persisted on tables
    let conn = Connection::open(&path).expect("sqlite open");
    let empty_is_null: bool = conn
        .query_row(
            "SELECT properties IS NULL FROM canonical_nodes WHERE logical_id = 'n-empty' AND superseded_at IS NULL",
            [],
            |r| r.get(0),
        )
        .expect("query empty properties IS NULL");
    assert!(empty_is_null, "Some(empty_map) must persist SQL NULL in canonical_nodes.properties");
}

#[test]
fn slice38_property_validation_rejects_malformed_atomically() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "validation");
    let opened = Engine::open(&path).expect("open");
    let engine = opened.engine;

    let valid_seed = || PreparedWrite::Node {
        kind: "note".into(),
        body: "valid companion row".into(),
        source_id: SourceId::new("src:val").unwrap(),
        logical_id: Some("valid-companion".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: Some(BTreeMap::from([("ok".into(), PropertyValue::Int(1))])),
    };

    let bad_maps: Vec<(&str, PropertyMap)> = vec![
        ("nan", BTreeMap::from([("k".into(), PropertyValue::Float(f64::NAN))])),
        ("inf", BTreeMap::from([("k".into(), PropertyValue::Float(f64::INFINITY))])),
        ("neg_inf", BTreeMap::from([("k".into(), PropertyValue::Float(f64::NEG_INFINITY))])),
        ("empty_key", BTreeMap::from([("".into(), PropertyValue::Int(1))])),
        ("nul_key", BTreeMap::from([("a\0b".into(), PropertyValue::Int(1))])),
        ("rs_key", BTreeMap::from([("a\u{1e}b".into(), PropertyValue::Int(1))])),
        ("nul_val", BTreeMap::from([("k".into(), PropertyValue::String("x\0y".into()))])),
        ("rs_val", BTreeMap::from([("k".into(), PropertyValue::String("x\u{1e}y".into()))])),
    ];

    // Node & Edge mixed batches with valid companion row must roll back atomically (0 rows committed).
    for (label, bad_map) in &bad_maps {
        let node_err = engine
            .write(&[
                valid_seed(),
                PreparedWrite::Node {
                    kind: "note".into(),
                    body: "bad node".into(),
                    source_id: SourceId::new("src:val").unwrap(),
                    logical_id: Some(format!("bad-node-{label}")),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    properties: Some(bad_map.clone()),
                },
            ])
            .expect_err(&format!("malformed node {label} must fail"));
        assert!(
            matches!(node_err, EngineError::WriteValidation),
            "expected WriteValidation for node {label}, got {node_err:?}"
        );
        assert!(
            engine.read_get("valid-companion", &ReadView::default()).unwrap().is_none(),
            "batch must roll back atomically on malformed node {label}"
        );

        let edge_err = engine
            .write(&[
                valid_seed(),
                PreparedWrite::Edge {
                    kind: "rel".into(),
                    from: "valid-companion".into(),
                    to: "valid-companion".into(),
                    source_id: SourceId::new("src:val").unwrap(),
                    logical_id: Some(format!("bad-edge-{label}")),
                    body: None,
                    t_valid: None,
                    t_invalid: None,
                    confidence: None,
                    extractor_model_id: None,
                    temporal_fallback: None,
                    properties: Some(bad_map.clone()),
                },
            ])
            .expect_err(&format!("malformed edge {label} must fail"));
        assert!(
            matches!(edge_err, EngineError::WriteValidation),
            "expected WriteValidation for edge {label}, got {edge_err:?}"
        );
        assert!(
            engine.read_get("valid-companion", &ReadView::default()).unwrap().is_none(),
            "batch must roll back atomically on malformed edge {label}"
        );
    }

    // Reusing the same actuate operation_id with different properties must return OperationIdConflict.
    let node_op = |props: Option<PropertyMap>| {
        ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
            kind: "note".into(),
            body: "act body".into(),
            source_id: SourceId::new("src:val").unwrap(),
            logical_id: Some("act-idem".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: WriteProvenanceV1::canonical(
                ArtifactRevisionId::new("rev-act-idem").unwrap(),
                SourceVersionId::new("ver-act-1").unwrap(),
            ),
            properties: props,
        })
    };

    let props_a = BTreeMap::from([("stage".into(), PropertyValue::Int(1))]);
    let props_b = BTreeMap::from([("stage".into(), PropertyValue::Int(2))]);
    let first = engine
        .actuate(
            ActuationBatchV1::new("op-prop-conflict", vec![node_op(Some(props_a.clone()))])
                .unwrap(),
        )
        .expect("initial actuate");
    let replay = engine
        .actuate(ActuationBatchV1::new("op-prop-conflict", vec![node_op(Some(props_a))]).unwrap())
        .expect("idempotent replay");
    assert_eq!(first, replay);

    let conflict_err = engine
        .actuate(ActuationBatchV1::new("op-prop-conflict", vec![node_op(Some(props_b))]).unwrap())
        .expect_err("different properties with same operation_id must conflict");
    assert!(
        matches!(
            conflict_err,
            EngineError::Actuation(ref err) if err.reason == ActuationErrorReason::OperationIdConflict
        ),
        "expected OperationIdConflict, got {conflict_err:?}"
    );
}

#[test]
fn slice38_declared_filterable_indexes_vec0_budget_and_lifecycle() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "indexes-lifecycle");
    let (engine, _) = open_with_embedder(&path);

    // 1. Refuse Searchable / Rankable on Edge specs with WriteValidation.
    for bad_role in [ProjectionRole::Searchable, ProjectionRole::Rankable] {
        let mut roles = BTreeSet::new();
        roles.insert(bad_role);
        let bad_spec = ProjectionSpec {
            name: "edge_bad".into(),
            roles,
            fts: if bad_role == ProjectionRole::Searchable {
                Some(fathomdb_engine::ProjectionFts { tokenizer: None })
            } else {
                None
            },
            vector: None,
            source: None,
            entity_class: PropertyEntityClass::Edge,
        };
        let err = engine.configure_projections(&[bad_spec], &[]).expect_err("edge non-filterable");
        assert!(
            matches!(err, EngineError::WriteValidation),
            "expected WriteValidation for edge role {bad_role:?}, got {err:?}"
        );
    }

    // 2. Declare 10 node Filterable properties + 2 edge Filterable properties + vector Searchable on body.
    // Exceeding MAX_VEC0_NODE_PROPERTY_SLOTS = 8 must NOT error in sqlite-vec!
    let mut specs = Vec::new();
    specs.push(ProjectionSpec {
        name: "title".into(),
        roles: BTreeSet::from([ProjectionRole::Searchable, ProjectionRole::Filterable]),
        fts: Some(fathomdb_engine::ProjectionFts { tokenizer: None }),
        vector: Some(ProjectionVector { embedder: None, dense_readiness: None, kinds: None }),
        source: None,
        entity_class: PropertyEntityClass::Node,
    });
    for i in 1..=10 {
        specs.push(filterable_spec(&format!("p{i:02}"), PropertyEntityClass::Node));
    }
    specs.push(filterable_spec("e_weight", PropertyEntityClass::Edge));
    specs.push(filterable_spec("e_kind_tag", PropertyEntityClass::Edge));
    engine.configure_projections(&specs, &[]).expect("configure 11 node + 2 edge filterable");

    // Confirm vec0 table never exceeds MAX_VEC0_NODE_PROPERTY_SLOTS (8) property columns
    // (16 total auxiliary/metadata columns with 7 built-ins + 8 node property slots + 1 reserved for scope).
    let round_trip_specs = engine.read_projections().expect("read_projections");
    let edge_spec =
        round_trip_specs.iter().find(|s| s.name == "e_weight").expect("e_weight round-trip");
    assert_eq!(edge_spec.entity_class, PropertyEntityClass::Edge);

    // 3. Property precedence over json_extract(body, ...), immutable revisions, and node/edge supersession.
    let r1_body = r#"{"p01":"from-body-should-lose","p02":1,"tag":"alpha_cluster"}"#;
    let r1_props = BTreeMap::from([
        ("p01".into(), PropertyValue::String("from-props-wins".into())),
        ("p02".into(), PropertyValue::Int(5)),
        ("p10".into(), PropertyValue::Int((1_i64 << 53) + 1)),
    ]);
    let target_r1 = engine
        .write(&[PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            kind: "note".into(),
            body: format!("{r1_body} alpha_cluster"),
            source_id: SourceId::new("src:idx").unwrap(),
            logical_id: Some("life-node".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: WriteProvenanceV1::canonical(
                ArtifactRevisionId::new("rev-life-1").unwrap(),
                SourceVersionId::new("ver-idx-1").unwrap(),
            ),
            properties: Some(r1_props),
        })])
        .expect("write life-node r1");

    // Confirm properties overrides json body in search_filtered & canonical_property_values.
    let mut f_body = SearchFilter::default();
    f_body.property_predicates = vec![PropertyPredicate {
        key: "p01".into(),
        op: PropertyComparisonOp::Eq,
        value: PropertyValue::String("from-body-should-lose".into()),
    }];
    engine.drain(15_000).expect("drain");
    assert!(
        engine.search_filtered("alpha_cluster", Some(f_body)).unwrap().results.is_empty(),
        "body value must lose to properties on declared key"
    );

    let mut f_props = SearchFilter::default();
    f_props.property_predicates = vec![PropertyPredicate {
        key: "p01".into(),
        op: PropertyComparisonOp::Eq,
        value: PropertyValue::String("from-props-wins".into()),
    }];
    assert_eq!(
        engine.search_filtered("alpha_cluster", Some(f_props.clone())).unwrap().results.len(),
        1
    );

    // Supersede life-node -> rev-life-2 with updated properties; confirm immutable revision + active-only cpv cleanup!
    let r2_props = BTreeMap::from([
        ("p01".into(), PropertyValue::String("rev2-props".into())),
        ("p02".into(), PropertyValue::Int(10)),
    ]);
    let target_r2 = engine
        .write(&[PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            kind: "note".into(),
            body: "alpha_cluster revised".into(),
            source_id: SourceId::new("src:idx").unwrap(),
            logical_id: Some("life-node".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: WriteProvenanceV1::canonical(
                ArtifactRevisionId::new("rev-life-2").unwrap(),
                SourceVersionId::new("ver-idx-2").unwrap(),
            ),
            properties: Some(r2_props),
        })])
        .expect("write life-node r2");
    assert!(target_r2.cursor > target_r1.cursor);

    // Check active-only cpv & immutable revisions.
    {
        let conn = Connection::open(&path).expect("open sqlite");
        let old_cpv: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM canonical_property_values WHERE entity_class = 'node' AND write_cursor = ?1",
                [target_r1.cursor as i64],
                |r| r.get(0),
            )
            .unwrap();
        let new_cpv: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM canonical_property_values WHERE entity_class = 'node' AND write_cursor = ?1",
                [target_r2.cursor as i64],
                |r| r.get(0),
            )
            .unwrap();
        let rev_count: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT write_cursor) FROM _fathomdb_artifact_revisions WHERE revision_id IN ('rev-life-1', 'rev-life-2')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(old_cpv, 0, "superseded node must have 0 active canonical_property_values rows");
        assert_eq!(new_cpv, 2, "active node must have 2 canonical_property_values rows");
        assert_eq!(rev_count, 2, "updates must create immutable revisions without overwriting");
    }

    // Check edge G0 (logical_id) & G11 ((from, to, kind)) active-only cpv cleanup.
    let e_g0_1 = engine
        .write(&[PreparedWrite::Edge {
            kind: "rel_g0".into(),
            from: "life-node".into(),
            to: "life-node".into(),
            source_id: SourceId::new("src:idx").unwrap(),
            logical_id: Some("edge-g0".into()),
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
            properties: Some(BTreeMap::from([("e_weight".into(), PropertyValue::Int(10))])),
        }])
        .unwrap();
    let e_g0_2 = engine
        .write(&[PreparedWrite::Edge {
            kind: "rel_g0".into(),
            from: "life-node".into(),
            to: "life-node".into(),
            source_id: SourceId::new("src:idx").unwrap(),
            logical_id: Some("edge-g0".into()),
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
            properties: Some(BTreeMap::from([("e_weight".into(), PropertyValue::Int(20))])),
        }])
        .unwrap();
    let e_g11_1 = engine
        .write(&[PreparedWrite::Edge {
            kind: "rel_g11".into(),
            from: "life-node".into(),
            to: "life-node".into(),
            source_id: SourceId::new("src:idx").unwrap(),
            logical_id: None,
            body: Some("fact edge g11 r1".into()),
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
            properties: Some(BTreeMap::from([("e_weight".into(), PropertyValue::Int(30))])),
        }])
        .unwrap();
    let e_g11_2 = engine
        .write(&[PreparedWrite::Edge {
            kind: "rel_g11".into(),
            from: "life-node".into(),
            to: "life-node".into(),
            source_id: SourceId::new("src:idx").unwrap(),
            logical_id: None,
            body: Some("fact edge g11 r2".into()),
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
            properties: Some(BTreeMap::from([("e_weight".into(), PropertyValue::Int(40))])),
        }])
        .unwrap();
    {
        let conn = Connection::open(&path).expect("open sqlite");
        for (old_c, new_c, label) in [
            (e_g0_1.cursor, e_g0_2.cursor, "edge G0"),
            (e_g11_1.cursor, e_g11_2.cursor, "edge G11"),
        ] {
            let old_cnt: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM canonical_property_values WHERE entity_class = 'edge' AND write_cursor = ?1",
                    [old_c as i64],
                    |r| r.get(0),
                )
                .unwrap();
            let new_cnt: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM canonical_property_values WHERE entity_class = 'edge' AND write_cursor = ?1",
                    [new_c as i64],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(old_cnt, 0, "{label}: superseded edge must purge cpv");
            assert_eq!(new_cnt, 1, "{label}: active edge must keep cpv");
        }
    }

    // 4. Lifecycle transition: purge cpv on Deleted/Pending, re-project from (body, properties) on Active.
    engine.transition("life-node", LifecycleState::Deleted, None).unwrap();
    {
        let conn = Connection::open(&path).expect("open sqlite");
        let cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM canonical_property_values WHERE entity_class = 'node' AND write_cursor = ?1",
                [target_r2.cursor as i64],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cnt, 0, "Deleted transition must purge cpv");
    }
    engine.transition("life-node", LifecycleState::Active, None).unwrap();
    let pending_r = engine
        .write(&[PreparedWrite::Node {
            kind: "note".into(),
            body: r#"{"p01":"pending-body","p02":7}"#.into(),
            source_id: SourceId::new("src:idx").unwrap(),
            logical_id: Some("pending-life-node".into()),
            state: InitialState::Pending,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: Some(BTreeMap::from([(
                "p01".into(),
                PropertyValue::String("pending-props".into()),
            )])),
        }])
        .unwrap();
    {
        let conn = Connection::open(&path).expect("open sqlite");
        let pending_cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM canonical_property_values WHERE entity_class = 'node' AND write_cursor = ?1",
                [pending_r.cursor as i64],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pending_cnt, 0, "Pending node must keep cpv purged");
        let cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM canonical_property_values WHERE entity_class = 'node' AND write_cursor = ?1",
                [target_r2.cursor as i64],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cnt, 2, "Active reactivation must re-project cpv from (body, properties)");
    }
    engine.transition("pending-life-node", LifecycleState::Active, None).unwrap();
    {
        let conn = Connection::open(&path).expect("open sqlite");
        let promoted_p01: String = conn
            .query_row(
                "SELECT text_val FROM canonical_property_values WHERE entity_class = 'node' AND write_cursor = ?1 AND attr_name = 'p01'",
                [pending_r.cursor as i64],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            promoted_p01, "pending-props",
            "Pending -> Active must project with properties precedence over body"
        );
    }

    // 5. InvalidFilter rejections (undeclared key, edge-only key in node search, Bool ordering).
    let invalid_cases = vec![
        PropertyPredicate {
            key: "undeclared_key".into(),
            op: PropertyComparisonOp::Eq,
            value: PropertyValue::Int(1),
        },
        PropertyPredicate {
            key: "e_weight".into(),
            op: PropertyComparisonOp::Eq,
            value: PropertyValue::Int(20),
        },
        PropertyPredicate {
            key: "p03".into(),
            op: PropertyComparisonOp::Lt,
            value: PropertyValue::Bool(true),
        },
        PropertyPredicate {
            key: "p03".into(),
            op: PropertyComparisonOp::Gte,
            value: PropertyValue::Bool(false),
        },
    ];
    for pred in invalid_cases {
        let mut sf = SearchFilter::default();
        sf.property_predicates = vec![pred.clone()];
        assert!(
            matches!(
                engine.search_filtered("alpha_cluster", Some(sf)),
                Err(EngineError::InvalidFilter { .. })
            ),
            "search_filtered must reject invalid predicate {pred:?}"
        );
        let lf = Filter { terms: vec![FilterTerm::Property(pred.clone())] };
        assert!(
            matches!(
                engine.read_list_filter("note", &lf, 10, &ReadView::default()),
                Err(EngineError::InvalidFilter { .. })
            ),
            "read_list_filter must reject invalid predicate {pred:?}"
        );
    }

    // 6. String vs Bool/Int non-collision on vec0 + 100% recall beyond TOP_K_BIT_CANDIDATES (192).
    // Seed 210 distractors closer to query vector than our distant targets!
    let mut batch = Vec::new();
    for idx in 0..(TOP_K_BIT_CANDIDATES + 25) {
        batch.push(PreparedWrite::Node {
            kind: "note".into(),
            body: "alpha_cluster distractor corpus".into(),
            source_id: SourceId::new("src:recall").unwrap(),
            logical_id: Some(format!("distractor-{idx}")),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: Some(BTreeMap::from([
                ("p01".into(), PropertyValue::String("true".into())), // String("true"), NOT Bool(true)
                ("p02".into(), PropertyValue::Int(100)),
                ("p03".into(), PropertyValue::Bool(false)),
                ("p09".into(), PropertyValue::String("miss".into())),
                ("p10".into(), PropertyValue::Int(1_i64 << 53)),
            ])),
        });
    }
    // Matching target placed on "beta_cluster" vector far in Hamming space from "alpha_cluster",
    // AND containing String("true") vs Bool(true) tests, Float(5.0)==Int(5), and exact (1<<53)+1.
    batch.push(PreparedWrite::Node {
        kind: "note".into(),
        body: "beta_cluster unique target vector".into(),
        source_id: SourceId::new("src:recall").unwrap(),
        logical_id: Some("recall-target".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: Some(BTreeMap::from([
            ("p01".into(), PropertyValue::String("vec0-slot1-hit".into())),
            ("p02".into(), PropertyValue::Int(5)),
            ("p03".into(), PropertyValue::Bool(true)),
            ("p09".into(), PropertyValue::String("slot9-exact".into())),
            ("p10".into(), PropertyValue::Int((1_i64 << 53) + 1)),
        ])),
    });
    engine.write(&batch).expect("write recall corpus");
    engine.drain(30_000).expect("drain recall corpus");

    // Non-collision on vec0: query p01 == Bool(true) must NOT match distractors with p01 == String("true")!
    let mut sf_bool_vs_str = SearchFilter::default();
    sf_bool_vs_str.property_predicates = vec![PropertyPredicate {
        key: "p01".into(),
        op: PropertyComparisonOp::Eq,
        value: PropertyValue::Bool(true),
    }];
    assert!(
        engine.search_filtered("alpha_cluster", Some(sf_bool_vs_str)).unwrap().results.is_empty(),
        "Bool(true) must not collide with String(\"true\") on vec0"
    );

    // 100% recall outside top-192 Hamming candidates on vector search for:
    // (a) slot 9+ predicate (p09 == "slot9-exact")
    // (b) Ne (p02 != 100)
    // (c) numeric Eq cross Int/Float (p02 == Float(5.0))
    // (d) exact i64 boundary ((1<<53)+1 != (1<<53))
    // (e) numeric range (p02 < 10)
    let recall_predicates = vec![
        (
            "slot9",
            PropertyPredicate {
                key: "p09".into(),
                op: PropertyComparisonOp::Eq,
                value: PropertyValue::String("slot9-exact".into()),
            },
        ),
        (
            "ne",
            PropertyPredicate {
                key: "p02".into(),
                op: PropertyComparisonOp::Ne,
                value: PropertyValue::Int(100),
            },
        ),
        (
            "num_eq_float",
            PropertyPredicate {
                key: "p02".into(),
                op: PropertyComparisonOp::Eq,
                value: PropertyValue::Float(5.0),
            },
        ),
        (
            "i64_exact",
            PropertyPredicate {
                key: "p10".into(),
                op: PropertyComparisonOp::Eq,
                value: PropertyValue::Int((1_i64 << 53) + 1),
            },
        ),
        (
            "range_lt",
            PropertyPredicate {
                key: "p02".into(),
                op: PropertyComparisonOp::Lt,
                value: PropertyValue::Int(10),
            },
        ),
    ];
    engine.set_vector_stage_only_for_test(true);
    for (label, pred) in recall_predicates {
        let mut sf = SearchFilter::default();
        sf.property_predicates = vec![pred];
        let res = engine.search_filtered("alpha_cluster", Some(sf)).unwrap();
        assert!(
            res.results.iter().any(|h| h.id == IdSpace::logical("recall-target")),
            "100% vector recall failed for {label} beyond TOP_K_BIT_CANDIDATES: hits={:?}",
            res.results.iter().map(|h| &h.id).collect::<Vec<_>>()
        );
    }
    engine.set_vector_stage_only_for_test(false);

    // 7. Relaxed ReadView parity (include_superseded || include_inactive) across search_frozen, read_list_filter, read_canonical_page.
    // Note: life-node rev-1 has p01="from-props-wins" and was SUPERSEDED, so it has ZERO cpv rows.
    // Delete a node that has p09="deleted-relaxed" -> also purged from cpv.
    engine
        .write(&[PreparedWrite::Node {
            kind: "note".into(),
            body: "alpha_cluster relaxed deleted node".into(),
            source_id: SourceId::new("src:relaxed").unwrap(),
            logical_id: Some("relaxed-deleted".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: Some(BTreeMap::from([(
                "p09".into(),
                PropertyValue::String("deleted-relaxed".into()),
            )])),
        }])
        .unwrap();
    engine.transition("relaxed-deleted", LifecycleState::Deleted, None).unwrap();
    engine.drain(15_000).unwrap();

    let relaxed_view =
        ReadView { include_superseded: true, include_inactive: true, ..ReadView::default() };
    let mut sf_superseded = SearchFilter::default();
    sf_superseded.property_predicates = vec![PropertyPredicate {
        key: "p01".into(),
        op: PropertyComparisonOp::Eq,
        value: PropertyValue::String("from-props-wins".into()),
    }];
    let frozen_relaxed = engine
        .freeze_read_context(&ReadContextV1::new(relaxed_view, sf_superseded.clone()).unwrap())
        .expect("freeze relaxed context");

    let search_relaxed = engine
        .search_frozen("alpha_cluster", &frozen_relaxed, 0, false, 0.3, 0, false, 10)
        .unwrap();
    assert!(
        search_relaxed.results.iter().any(|h| h.id == IdSpace::logical("life-node")),
        "relaxed search_frozen must evaluate inline properties on superseded row with 0 cpv rows"
    );

    let list_relaxed = engine
        .read_list_filter(
            "note",
            &Filter {
                terms: vec![FilterTerm::Property(sf_superseded.property_predicates[0].clone())],
            },
            20,
            &relaxed_view,
        )
        .unwrap();
    assert!(
        list_relaxed.iter().any(|n| n.logical_id == "life-node"),
        "relaxed read_list_filter must match superseded row via inline properties"
    );

    let page_relaxed = engine
        .read_canonical_page(
            "note",
            &frozen_relaxed,
            &PageRequestV1 { schema_version: 1, limit: 20, cursor: None },
        )
        .unwrap();
    assert!(
        page_relaxed.items.iter().any(|n| n.logical_id == "life-node"),
        "relaxed read_canonical_page must match superseded row via inline properties"
    );

    // Inactive check as well
    let mut sf_deleted = SearchFilter::default();
    sf_deleted.property_predicates = vec![PropertyPredicate {
        key: "p09".into(),
        op: PropertyComparisonOp::Eq,
        value: PropertyValue::String("deleted-relaxed".into()),
    }];
    let frozen_deleted =
        engine.freeze_read_context(&ReadContextV1::new(relaxed_view, sf_deleted).unwrap()).unwrap();
    let search_deleted = engine
        .search_frozen("alpha_cluster", &frozen_deleted, 0, false, 0.3, 0, false, 10)
        .unwrap();
    assert!(
        search_deleted.results.iter().any(|h| h.id == IdSpace::logical("relaxed-deleted")),
        "relaxed search_frozen must match deleted row with inline properties"
    );

    // 8. FrozenReadContextV1 binding changes digest when property_predicates changes.
    let mut alt_filter = SearchFilter::default();
    alt_filter.property_predicates = vec![PropertyPredicate {
        key: "p09".into(),
        op: PropertyComparisonOp::Eq,
        value: PropertyValue::String("slot9-exact".into()),
    }];
    let frozen_a = engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), f_props).unwrap())
        .unwrap();
    let frozen_b = engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), alt_filter).unwrap())
        .unwrap();
    assert_ne!(
        frozen_a.token, frozen_b.token,
        "different property_predicates must mint distinct frozen tokens"
    );
    let mut tampered = frozen_a.clone();
    tampered.context.eligibility.property_predicates =
        frozen_b.context.eligibility.property_predicates.clone();
    assert!(
        engine.search_frozen("alpha_cluster", &tampered, 0, false, 0.3, 0, false, 10).is_err(),
        "swapping property_predicates on frozen context must fail authentication"
    );
}

#[test]
fn slice38_commit_timestamps_monotonic_and_actuate_cursor_guard() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "timestamps");
    let opened = Engine::open(&path).expect("open");
    let engine = opened.engine;

    // 1. Single row per committed batch in `write`, keyed by high-water cursor.
    let receipt_w = engine
        .write(&[
            PreparedWrite::Node {
                kind: "note".into(),
                body: "b1".into(),
                source_id: SourceId::new("src:ts").unwrap(),
                logical_id: Some("ts-1".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
            PreparedWrite::Node {
                kind: "note".into(),
                body: "b2".into(),
                source_id: SourceId::new("src:ts").unwrap(),
                logical_id: Some("ts-2".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
            PreparedWrite::Node {
                kind: "note".into(),
                body: "b3".into(),
                source_id: SourceId::new("src:ts").unwrap(),
                logical_id: Some("ts-3".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
        ])
        .expect("write 3-node batch");

    let cursors = engine
        .query_i64_col_for_test(
            "SELECT write_cursor FROM _fathomdb_commit_timestamps ORDER BY write_cursor ASC",
        )
        .unwrap();
    assert_eq!(
        cursors,
        vec![receipt_w.cursor as i64],
        "expected exactly one commit timestamp row at high-water WriteReceipt.cursor"
    );

    // 2. Cursor-allocating actuate writes 1 timestamp row at its high-water cursor.
    engine
        .actuate(
            ActuationBatchV1::new(
                "op-ts-write",
                vec![
                    ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
                        kind: "note".into(),
                        body: "src act".into(),
                        source_id: SourceId::new("src:ts").unwrap(),
                        logical_id: Some("ts-act-src".into()),
                        state: InitialState::Active,
                        reason: None,
                        valid_from: None,
                        valid_until: None,
                        provenance: WriteProvenanceV1::canonical(
                            ArtifactRevisionId::new("rev-ts-src").unwrap(),
                            SourceVersionId::new("ver-ts-1").unwrap(),
                        ),
                        properties: None,
                    }),
                    ActuationOperationV1::PutDerivedNode(ProvenancedNodeV1 {
                        kind: "note".into(),
                        body: "der act".into(),
                        source_id: SourceId::new("src:ts").unwrap(),
                        logical_id: Some("ts-act-der".into()),
                        state: InitialState::Active,
                        reason: None,
                        valid_from: None,
                        valid_until: None,
                        provenance: WriteProvenanceV1::derived(
                            ArtifactRevisionId::new("rev-ts-der").unwrap(),
                            SourceVersionId::new("ver-ts-1").unwrap(),
                            SourceRevisionId::new("rev-ts-src").unwrap(),
                            SourceLocator::whole_body(),
                            CanonicalHash::sha256(sha256_hex("src act")).unwrap(),
                        ),
                        properties: None,
                    }),
                ],
            )
            .unwrap(),
        )
        .expect("cursor-allocating actuate");

    let cursors_after_act = engine
        .query_i64_col_for_test(
            "SELECT write_cursor FROM _fathomdb_commit_timestamps ORDER BY write_cursor ASC",
        )
        .unwrap();
    assert_eq!(cursors_after_act.len(), 2, "actuate must add 1 commit timestamp row");
    assert_eq!(
        cursors_after_act[1],
        receipt_w.cursor as i64 + 2,
        "actuate row must be keyed by high-water cursor"
    );

    // 3. Dependency-only and lifecycle-only actuations must NOT insert duplicate PK rows or panic.
    engine
        .actuate(
            ActuationBatchV1::new(
                "op-ts-dep-only",
                vec![ActuationOperationV1::RegisterSourceDependency(
                    SourceDependencyRegistrationV1::new("dep-ts", "rev-ts-src", "rev-ts-der")
                        .unwrap(),
                )],
            )
            .unwrap(),
        )
        .expect("dependency-only actuate");

    engine
        .actuate(
            ActuationBatchV1::new(
                "op-ts-life-only",
                vec![ActuationOperationV1::TransitionLifecycle(
                    LifecycleActuationV1::new(
                        "ts-act-der",
                        ArtifactRevisionId::new("rev-ts-der").unwrap(),
                        LifecycleState::Deleted,
                        Some("retire".into()),
                    )
                    .unwrap(),
                )],
            )
            .unwrap(),
        )
        .expect("lifecycle-only actuate");

    let cursors_after_non_cursor_ops = engine
        .query_i64_col_for_test(
            "SELECT write_cursor FROM _fathomdb_commit_timestamps ORDER BY write_cursor ASC",
        )
        .unwrap();
    assert_eq!(
        cursors_after_non_cursor_ops, cursors_after_act,
        "non-cursor-allocating actuations must emit zero commit timestamp rows"
    );

    // 4. Simulated clock regression: push the latest timestamp far into the future, then commit via
    // both `write` and `actuate`; committed_at_ms must clamp monotonically >= previous high-water ts.
    let simulated_future_ms = 9_999_999_999_999_i64;
    engine
        .execute_for_test(&format!(
            "UPDATE _fathomdb_commit_timestamps SET committed_at_ms = {simulated_future_ms} WHERE write_cursor = {}",
            cursors_after_act[1]
        ))
        .unwrap();

    let w_clamped = engine
        .write(&[PreparedWrite::Node {
            kind: "note".into(),
            body: "after clock jump".into(),
            source_id: SourceId::new("src:ts").unwrap(),
            logical_id: Some("ts-clamped".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        }])
        .expect("write after clock regression");

    let clamped_w_ts = engine
        .query_i64_col_for_test(&format!(
            "SELECT committed_at_ms FROM _fathomdb_commit_timestamps WHERE write_cursor = {}",
            w_clamped.cursor
        ))
        .unwrap();
    assert_eq!(clamped_w_ts, vec![simulated_future_ms], "write timestamp must clamp monotonically");

    engine
        .actuate(
            ActuationBatchV1::new(
                "op-ts-clamped",
                vec![ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
                    kind: "note".into(),
                    body: "act clamped".into(),
                    source_id: SourceId::new("src:ts").unwrap(),
                    logical_id: Some("ts-act-clamped".into()),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: WriteProvenanceV1::canonical(
                        ArtifactRevisionId::new("rev-ts-clamped").unwrap(),
                        SourceVersionId::new("ver-ts-2").unwrap(),
                    ),
                    properties: None,
                })],
            )
            .unwrap(),
        )
        .expect("actuate after clock regression");

    let latest_ts = engine
        .query_i64_col_for_test(
            "SELECT committed_at_ms FROM _fathomdb_commit_timestamps ORDER BY write_cursor DESC LIMIT 1",
        )
        .unwrap();
    assert_eq!(latest_ts, vec![simulated_future_ms], "actuate timestamp must clamp monotonically");
}

#[test]
fn slice38_neg_zero_eav_and_property_fts_doctor_integrity_clean() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "neg-zero-eav");
    {
        let opened = Engine::open(&path).expect("open");
        let engine = opened.engine;
        engine
            .configure_projections(
                &[ProjectionSpec {
                    name: "score".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable, ProjectionRole::Searchable]),
                    fts: Some(ProjectionFts { tokenizer: None }),
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                }],
                &[],
            )
            .expect("configure filterable+searchable score");

        engine
            .write(&[PreparedWrite::Node {
                kind: "note".into(),
                body: "negative zero score check".into(),
                source_id: SourceId::new("src:negzero").unwrap(),
                logical_id: Some("nz-1".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: Some(BTreeMap::from([("score".into(), PropertyValue::Float(-0.0))])),
            }])
            .expect("write -0.0");

        let eav_vals = engine
            .query_text_col_for_test(
                "SELECT attr_value FROM canonical_attributes WHERE attr_name = 'score'",
            )
            .unwrap();
        assert_eq!(
            eav_vals,
            vec!["0.0".to_string()],
            "canonical_attributes must canonicalize -0.0 -> 0.0"
        );

        let fts_vals = engine
            .query_text_col_for_test(
                "SELECT attr_value FROM property_search_index WHERE attr_name = 'score'",
            )
            .unwrap();
        assert_eq!(
            fts_vals,
            vec!["0.0".to_string()],
            "property_search_index must canonicalize -0.0 -> 0.0"
        );

        for check in [
            DataPlaneIntegrityCheckV1::DependencyChain,
            DataPlaneIntegrityCheckV1::ActiveSearchableOrphans,
            DataPlaneIntegrityCheckV1::ProjectionGeneration,
            DataPlaneIntegrityCheckV1::MutationReadiness,
        ] {
            let report = engine
                .check_data_plane_integrity(
                    DataPlaneIntegrityRequestV1::new(vec![check], 10_000, 100).unwrap(),
                )
                .expect("integrity check");
            assert!(
                report.findings.is_empty(),
                "expected 0 data-plane integrity findings for {check:?}, got: {:#?}",
                report.findings
            );
        }

        engine.close().expect("close");
    }

    // Reopen and verify clean re-derive + 0 integrity findings.
    let reopened = Engine::open(&path).expect("reopen");
    for check in [
        DataPlaneIntegrityCheckV1::DependencyChain,
        DataPlaneIntegrityCheckV1::ActiveSearchableOrphans,
        DataPlaneIntegrityCheckV1::ProjectionGeneration,
        DataPlaneIntegrityCheckV1::MutationReadiness,
    ] {
        let report = reopened
            .engine
            .check_data_plane_integrity(
                DataPlaneIntegrityRequestV1::new(vec![check], 10_000, 100).unwrap(),
            )
            .expect("integrity check after reopen");
        assert!(
            report.findings.is_empty(),
            "expected 0 integrity findings after reopen for {check:?}, got: {:#?}",
            report.findings
        );
    }
    reopened.engine.close().expect("close reopened");
}

#[test]
fn slice38_actuate_invalid_properties_returns_write_validation_without_consuming_op_id() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "actuate-invalid-props");
    let opened = Engine::open(&path).expect("open");
    let engine = opened.engine;

    let make_batch = |op_id: &str, props: PropertyMap| {
        ActuationBatchV1::new(
            op_id,
            vec![ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
                kind: "note".into(),
                body: "actuate invalid property validation".into(),
                source_id: SourceId::new("src:act-val").unwrap(),
                logical_id: Some("act-val-1".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("rev-act-val-1").unwrap(),
                    SourceVersionId::new("ver-act-val-1").unwrap(),
                ),
                properties: Some(props),
            })],
        )
        .unwrap()
    };

    for (label, bad_props) in [
        ("nan", BTreeMap::from([("k".into(), PropertyValue::Float(f64::NAN))])),
        ("inf", BTreeMap::from([("k".into(), PropertyValue::Float(f64::INFINITY))])),
        ("empty_key", BTreeMap::from([("".into(), PropertyValue::Int(1))])),
        ("nul_key", BTreeMap::from([("a\0b".into(), PropertyValue::Int(1))])),
        ("nul_val", BTreeMap::from([("k".into(), PropertyValue::String("x\0y".into()))])),
        ("rs_val", BTreeMap::from([("k".into(), PropertyValue::String("x\u{1e}y".into()))])),
    ] {
        let err = engine
            .actuate(make_batch("op-reusable-after-invalid", bad_props))
            .expect_err(&format!("invalid properties ({label}) must return Err(WriteValidation)"));
        assert!(
            matches!(err, EngineError::WriteValidation),
            "expected EngineError::WriteValidation for {label}, got {err:?}"
        );
    }

    // The same operation_id must remain unconsumed and succeed with valid properties.
    let receipt = engine
        .actuate(make_batch(
            "op-reusable-after-invalid",
            BTreeMap::from([("ok".into(), PropertyValue::Int(7))]),
        ))
        .expect("subsequent valid actuation with same operation_id must succeed");
    assert_eq!(receipt.outcome, ActuationOutcomeV1::Committed);
    engine.close().expect("close");
}

#[test]
fn slice38_frozen_read_rejects_excessive_property_predicates() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "frozen-pred-bound");
    let opened = Engine::open(&path).expect("open");
    let engine = opened.engine;

    let mut eligibility = SearchFilter::default();
    eligibility.property_predicates = (0..65)
        .map(|i| PropertyPredicate {
            key: format!("k{i}"),
            op: PropertyComparisonOp::Eq,
            value: PropertyValue::Int(i),
        })
        .collect();
    let err = engine
        .freeze_read_context(&ReadContextV1 {
            schema_version: 1,
            view: ReadView::default(),
            eligibility,
        })
        .expect_err("65 property_predicates must fail context validation");
    assert!(
        matches!(
            err,
            EngineError::FrozenRead(ref fe)
                if fe.reason == FrozenReadErrorReason::ContextInvalid
                    && fe.field_path == "/eligibility/propertyPredicates"
        ),
        "expected FrozenRead(ContextInvalid, /eligibility/propertyPredicates), got {err:?}"
    );
    engine.close().expect("close");
}
