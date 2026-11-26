//! 0.8.32 Slice 41 (`WP-41C`, `#269` Option A) — Artifact-scoped `SearchFilter.property_predicates`
//! on evidence `source_filter` (`AC32-EVIDENCE-PROP-01..03`).

use std::collections::{BTreeMap, BTreeSet};

use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, DependencyTraceDirectionV1, DependencyTraceErrorReasonV1,
    DependencyTraceRequestV1, EmbedderChoice, Engine, EngineConfig, EngineError,
    EvidenceErrorReasonV1, EvidenceGraphOriginV1, EvidenceResolveManyItemV1,
    EvidenceResolveManyRequestV1, EvidenceResolveRequestV1, EvidenceSearchRequestV1,
    GraphEvidenceResolveRequestV1, GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, IdSpace,
    InitialState, PreparedWrite, ProjectionRole, ProjectionSpec, PropertyComparisonOp,
    PropertyEntityClass, PropertyMap, PropertyPredicate, PropertyValue, ProvenancedEdgeV1,
    ProvenancedNodeV1, ReadContextV1, ReadView, ScopePolicy, SearchFilter, SoftFallbackBranch,
    SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
    TraversalDirection, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn sha256_hex(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn configure_acl_and_confidence(engine: &Engine) {
    engine
        .configure_projections(
            &[
                ProjectionSpec {
                    name: "acl".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
                ProjectionSpec {
                    name: "confidence".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
            ],
            &[],
        )
        .unwrap();
}

fn canonical_node(
    logical_id: &str,
    kind: &str,
    revision_id: &str,
    source_version_id: &str,
    source_id: &str,
    body: &str,
    properties: Option<PropertyMap>,
) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        logical_id: Some(logical_id.into()),
        kind: kind.into(),
        body: body.into(),
        source_id: SourceId::new(source_id).unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(revision_id).unwrap(),
            SourceVersionId::new(source_version_id).unwrap(),
        ),
        properties,
    })
}

#[allow(clippy::too_many_arguments)]
fn derived_node(
    logical_id: &str,
    kind: &str,
    revision_id: &str,
    source_version_id: &str,
    source_revision_id: &str,
    source_id: &str,
    source_body: &str,
    body: &str,
    properties: Option<PropertyMap>,
) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        logical_id: Some(logical_id.into()),
        kind: kind.into(),
        body: body.into(),
        source_id: SourceId::new(source_id).unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision_id).unwrap(),
            SourceVersionId::new(source_version_id).unwrap(),
            SourceRevisionId::new(source_revision_id).unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(sha256_hex(source_body)).unwrap(),
        ),
        properties,
    })
}

#[allow(clippy::too_many_arguments)]
fn derived_edge(
    logical_id: &str,
    kind: &str,
    from: &str,
    to: &str,
    revision_id: &str,
    source_version_id: &str,
    source_revision_id: &str,
    source_id: &str,
    source_body: &str,
) -> PreparedWrite {
    PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
        logical_id: Some(logical_id.into()),
        kind: kind.into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new(source_id).unwrap(),
        body: None,
        t_valid: None,
        t_invalid: None,
        confidence: Some(0.95),
        extractor_model_id: None,
        temporal_fallback: Some(false),
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision_id).unwrap(),
            SourceVersionId::new(source_version_id).unwrap(),
            SourceRevisionId::new(source_revision_id).unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(sha256_hex(source_body)).unwrap(),
        ),
        properties: None,
    })
}

fn assert_evidence_unavailable_at(error: EngineError, expected_path: &str) {
    assert!(
        matches!(
            error,
            EngineError::Evidence(ref ev)
                if ev.reason == EvidenceErrorReasonV1::EvidenceUnavailable
                    && ev.field_path == expected_path
        ),
        "expected EvidenceUnavailable at {expected_path}, got {error:?}"
    );
}

fn assert_evidence_unavailable(error: EngineError) {
    assert_evidence_unavailable_at(error, "/evidenceRef");
}

#[test]
fn ac32_evidence_prop_01_property_predicates_artifact_scoped_across_all_evidence_surfaces() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("prop-artifact-scoped{SQLITE_SUFFIX}"));
    let engine = Engine::open(&path).unwrap().engine;
    configure_acl_and_confidence(&engine);

    // Canonical SourceDoc carries access-bearing acl="team-a" and NO `confidence` property.
    let source_body = "canonical team-a raw source transcript without extracted confidence";
    let source_props = BTreeMap::from([("acl".into(), PropertyValue::String("team-a".into()))]);
    let high_props = BTreeMap::from([
        ("acl".into(), PropertyValue::String("team-a".into())),
        ("confidence".into(), PropertyValue::Float(0.95)),
    ]);
    let low_props = BTreeMap::from([
        ("acl".into(), PropertyValue::String("team-a".into())),
        ("confidence".into(), PropertyValue::Float(0.25)),
    ]);
    let root_props = BTreeMap::from([
        ("acl".into(), PropertyValue::String("team-a".into())),
        ("confidence".into(), PropertyValue::Float(0.99)),
    ]);

    engine
        .write(&[
            canonical_node(
                "src-team-a",
                "SourceDoc",
                "src-team-a-r1",
                "src-team-a-v1",
                "team-a-source-bucket",
                source_body,
                Some(source_props),
            ),
            derived_node(
                "doc-root",
                "Doc",
                "doc-root-r1",
                "src-team-a-v1",
                "src-team-a-r1",
                "team-a-source-bucket",
                source_body,
                "root anchor document",
                Some(root_props),
            ),
            // Write doc-low first with shorter body so if property_predicates did NOT filter before
            // truncation (limit=1), doc-low would win BM25 over doc-high.
            derived_node(
                "doc-low",
                "Doc",
                "doc-low-r1",
                "src-team-a-v1",
                "src-team-a-r1",
                "team-a-source-bucket",
                source_body,
                "helios_needle",
                Some(low_props),
            ),
            derived_node(
                "doc-high",
                "Doc",
                "doc-high-r1",
                "src-team-a-v1",
                "src-team-a-r1",
                "team-a-source-bucket",
                source_body,
                "helios_needle high confidence extracted claim with extra context words",
                Some(high_props),
            ),
            derived_edge(
                "edge-high",
                "supports",
                "doc-root",
                "doc-high",
                "edge-high-r1",
                "src-team-a-v1",
                "src-team-a-r1",
                "team-a-source-bucket",
                source_body,
            ),
            derived_edge(
                "edge-low",
                "weakly_supports",
                "doc-root",
                "doc-low",
                "edge-low-r1",
                "src-team-a-v1",
                "src-team-a-r1",
                "team-a-source-bucket",
                source_body,
            ),
        ])
        .unwrap();
    engine.drain(10_000).unwrap();

    let mut eligibility = SearchFilter::default();
    eligibility.kind = Some("Doc".into());
    eligibility.attributes.push(("acl".into(), "team-a".into()));
    eligibility.property_predicates.push(PropertyPredicate {
        key: "confidence".into(),
        op: PropertyComparisonOp::Gte,
        value: PropertyValue::Float(0.8),
    });
    let frozen = engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), eligibility).unwrap())
        .unwrap();

    // 1. search_with_evidence (with limit=1 to prove doc-low is excluded before truncation)
    let mut search_req = EvidenceSearchRequestV1::new("helios_needle".into(), frozen.clone());
    search_req.limit = 1;
    let search_res = engine.search_with_evidence(&search_req).unwrap();
    assert_eq!(search_res.search_result.results.len(), 1);
    assert_eq!(search_res.search_result.results[0].id.value, "doc-high");
    assert_eq!(search_res.evidence.len(), 1);
    assert_eq!(search_res.evidence[0].artifact_revision_id, "doc-high-r1");

    // 2. resolve_evidence
    let resolved = engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: search_res.evidence[0].evidence_ref.clone(),
            context: frozen.clone(),
        })
        .unwrap();
    assert_eq!(resolved.artifact_revision_id, "doc-high-r1");
    assert_eq!(resolved.source_revision_id, "src-team-a-r1");
    assert_eq!(resolved.canonical_source_body, source_body);
    assert_eq!(resolved.evidence_text, source_body);

    // 3. resolve_evidence_many
    let resolved_many = engine
        .resolve_evidence_many(&EvidenceResolveManyRequestV1 {
            schema_version: 1,
            evidence_refs: vec![search_res.evidence[0].evidence_ref.clone()],
            context: frozen.clone(),
        })
        .unwrap();
    assert_eq!(resolved_many.len(), 1);
    match &resolved_many[0] {
        EvidenceResolveManyItemV1::Resolved(ev) => {
            assert_eq!(ev.artifact_revision_id, "doc-high-r1");
            assert_eq!(ev.source_revision_id, "src-team-a-r1");
            assert_eq!(ev.canonical_source_body, source_body);
        }
        other => panic!("expected Resolved item, got {other:?}"),
    }

    // 4. graph_expand(include_evidence=true) + resolve_graph_evidence
    let expand_req = GraphExpandRequestV1::new(
        GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("doc-root")],
        },
        TraversalDirection::Outgoing,
        GraphReadContextV1::Frozen { schema_version: 1, context: frozen.clone() },
        1,
        10,
        10_000,
    )
    .with_edge_kinds(vec!["supports".into(), "weakly_supports".into()])
    .with_evidence(true);
    let expanded = engine.graph_expand(&expand_req).unwrap();
    assert_eq!(expanded.targets.len(), 1);
    assert_eq!(expanded.targets[0].logical_id, "doc-high");
    let graph_entry = &expanded.evidence.unwrap().entries[0];
    let resolved_target = engine
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: graph_entry.target_evidence_ref.clone(),
            context: frozen.clone(),
        })
        .unwrap();
    assert_eq!(resolved_target.artifact_revision_id.as_str(), "doc-high-r1");
    assert_eq!(resolved_target.source_revision_id.as_str(), "src-team-a-r1");
    assert_eq!(resolved_target.canonical_source_body, source_body);

    let resolved_edge = engine
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: graph_entry.terminal_edge_evidence_ref.clone(),
            context: frozen,
        })
        .unwrap();
    assert_eq!(resolved_edge.artifact_revision_id.as_str(), "edge-high-r1");
    assert_eq!(resolved_edge.source_revision_id.as_str(), "src-team-a-r1");

    engine.close().unwrap();
}

#[test]
fn ac32_evidence_prop_02_source_attributes_and_scope_policy_remain_fail_closed() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("prop-source-acl{SQLITE_SUFFIX}"));
    let engine = Engine::open(&path).unwrap().engine;
    configure_acl_and_confidence(&engine);

    let src_a_body = "team-a canonical bytes";
    let src_b_body = "team-b restricted canonical bytes";
    let src_a_props = BTreeMap::from([("acl".into(), PropertyValue::String("team-a".into()))]);
    let src_b_props = BTreeMap::from([("acl".into(), PropertyValue::String("team-b".into()))]);
    let matching_doc_props = BTreeMap::from([
        ("acl".into(), PropertyValue::String("team-a".into())),
        ("confidence".into(), PropertyValue::Float(0.95)),
    ]);

    engine
        .write(&[
            canonical_node(
                "src-a",
                "SourceDoc",
                "src-a-r1",
                "src-a-v1",
                "bucket-a",
                src_a_body,
                Some(src_a_props),
            ),
            canonical_node(
                "src-b",
                "SourceDoc",
                "src-b-r1",
                "src-b-v1",
                "bucket-b",
                src_b_body,
                Some(src_b_props),
            ),
            derived_node(
                "doc-root",
                "Doc",
                "doc-root-r1",
                "src-a-v1",
                "src-a-r1",
                "bucket-a",
                src_a_body,
                "root anchor",
                Some(matching_doc_props.clone()),
            ),
            // Target artifact matches acl="team-a" and confidence>=0.8, but its canonical source
            // carries acl="team-b" -> MUST fail closed with non-disclosing EvidenceUnavailable.
            derived_node(
                "doc-from-b",
                "Doc",
                "doc-from-b-r1",
                "src-b-v1",
                "src-b-r1",
                "bucket-b",
                src_b_body,
                "restricted_source_needle",
                Some(matching_doc_props),
            ),
            derived_edge(
                "edge-to-b",
                "supports",
                "doc-root",
                "doc-from-b",
                "edge-to-b-r1",
                "src-a-v1",
                "src-a-r1",
                "bucket-a",
                src_a_body,
            ),
        ])
        .unwrap();
    engine.drain(10_000).unwrap();

    let mut eligibility = SearchFilter::default();
    eligibility.kind = Some("Doc".into());
    eligibility.attributes.push(("acl".into(), "team-a".into()));
    eligibility.property_predicates.push(PropertyPredicate {
        key: "confidence".into(),
        op: PropertyComparisonOp::Gte,
        value: PropertyValue::Float(0.8),
    });
    let frozen = engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), eligibility).unwrap())
        .unwrap();

    // Ranked evidence path rejects source acl mismatch with non-disclosing EvidenceUnavailable.
    assert_evidence_unavailable(
        engine
            .search_with_evidence(&EvidenceSearchRequestV1::new(
                "restricted_source_needle".into(),
                frozen.clone(),
            ))
            .unwrap_err(),
    );

    // Graph evidence path rejects source acl mismatch with non-disclosing EvidenceUnavailable.
    let expand_req = GraphExpandRequestV1::new(
        GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("doc-root")],
        },
        TraversalDirection::Outgoing,
        GraphReadContextV1::Frozen { schema_version: 1, context: frozen },
        1,
        10,
        10_000,
    )
    .with_edge_kinds(vec!["supports".into()])
    .with_evidence(true);
    assert_evidence_unavailable_at(engine.graph_expand(&expand_req).unwrap_err(), "/evidence");
    engine.close().unwrap();

    // Schema 39 ScopePolicy cross-scope isolation remains fail-closed under Option A.
    let scoped_path = dir.path().join(format!("prop-scope-isolation{SQLITE_SUFFIX}"));
    let eng_a = Engine::open_with_choice_and_config(
        &scoped_path,
        EmbedderChoice::None,
        EngineConfig::default().with_scope_policy(ScopePolicy::single("tenant-a")),
    )
    .unwrap()
    .engine;
    configure_acl_and_confidence(&eng_a);
    let tenant_a_body = "tenant-a canonical source body";
    eng_a
        .write(&[
            canonical_node(
                "src-t",
                "SourceDoc",
                "src-ta-r1",
                "src-ta-v1",
                "bucket-ta",
                tenant_a_body,
                Some(BTreeMap::from([("acl".into(), PropertyValue::String("team-a".into()))])),
            ),
            derived_node(
                "doc-t",
                "Doc",
                "doc-ta-r1",
                "src-ta-v1",
                "src-ta-r1",
                "bucket-ta",
                tenant_a_body,
                "scoped_option_a_needle",
                Some(BTreeMap::from([
                    ("acl".into(), PropertyValue::String("team-a".into())),
                    ("confidence".into(), PropertyValue::Float(0.95)),
                ])),
            ),
        ])
        .unwrap();
    eng_a.drain(10_000).unwrap();

    let mut scoped_elig = SearchFilter::default();
    scoped_elig.kind = Some("Doc".into());
    scoped_elig.attributes.push(("acl".into(), "team-a".into()));
    scoped_elig.property_predicates.push(PropertyPredicate {
        key: "confidence".into(),
        op: PropertyComparisonOp::Gte,
        value: PropertyValue::Float(0.8),
    });
    let frozen_a = eng_a
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), scoped_elig.clone()).unwrap())
        .unwrap();
    let res_a = eng_a
        .search_with_evidence(&EvidenceSearchRequestV1::new(
            "scoped_option_a_needle".into(),
            frozen_a.clone(),
        ))
        .unwrap();
    assert_eq!(res_a.evidence.len(), 1);
    let ref_a = res_a.evidence[0].evidence_ref.clone();
    let resolved_a = eng_a
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: ref_a.clone(),
            context: frozen_a.clone(),
        })
        .unwrap();
    assert_eq!(resolved_a.canonical_source_body, tenant_a_body);
    eng_a.close().unwrap();

    let eng_b = Engine::open_with_choice_and_config(
        &scoped_path,
        EmbedderChoice::None,
        EngineConfig::default().with_scope_policy(ScopePolicy::single("tenant-b")),
    )
    .unwrap()
    .engine;
    let frozen_b = eng_b
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), scoped_elig).unwrap())
        .unwrap();
    let res_b = eng_b
        .search_with_evidence(&EvidenceSearchRequestV1::new(
            "scoped_option_a_needle".into(),
            frozen_b.clone(),
        ))
        .unwrap();
    assert!(res_b.search_result.results.is_empty());
    // Presenting tenant-a's EvidenceRefV1 to tenant-b fails closed with EvidenceUnavailable.
    assert_evidence_unavailable(
        eng_b
            .resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: ref_a,
                context: frozen_b,
            })
            .unwrap_err(),
    );
    eng_b.close().unwrap();
}

#[test]
fn ac32_evidence_prop_03_trace_dependency_preserves_symmetric_property_predicates() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("prop-trace-symmetry{SQLITE_SUFFIX}"));
    let engine = Engine::open(&path).unwrap().engine;
    configure_acl_and_confidence(&engine);

    let asym_src_body = "asymmetric canonical source lacking confidence property";
    let sym_src_body = "symmetric canonical source carrying confidence property";
    let acl_only = BTreeMap::from([("acl".into(), PropertyValue::String("team-a".into()))]);
    let acl_and_conf = BTreeMap::from([
        ("acl".into(), PropertyValue::String("team-a".into())),
        ("confidence".into(), PropertyValue::Float(0.95)),
    ]);

    engine
        .write(&[
            canonical_node(
                "src-asym",
                "SourceDoc",
                "src-asym-r1",
                "src-asym-v1",
                "bucket-asym",
                asym_src_body,
                Some(acl_only),
            ),
            derived_node(
                "doc-asym",
                "Doc",
                "doc-asym-r1",
                "src-asym-v1",
                "src-asym-r1",
                "bucket-asym",
                asym_src_body,
                "asymmetrictraceneedle",
                Some(acl_and_conf.clone()),
            ),
            canonical_node(
                "src-sym",
                "SourceDoc",
                "src-sym-r1",
                "src-sym-v1",
                "bucket-sym",
                sym_src_body,
                Some(acl_and_conf.clone()),
            ),
            derived_node(
                "doc-sym",
                "Doc",
                "doc-sym-r1",
                "src-sym-v1",
                "src-sym-r1",
                "bucket-sym",
                sym_src_body,
                "symmetrictraceneedle",
                Some(acl_and_conf),
            ),
        ])
        .unwrap();
    engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-asym", "src-asym-r1", "doc-asym-r1").unwrap(),
        )
        .unwrap();
    engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-sym", "src-sym-r1", "doc-sym-r1").unwrap(),
        )
        .unwrap();
    engine.drain(10_000).unwrap();

    // Kind is left None so symmetric endpoint visibility is tested strictly on property_predicates.
    let mut eligibility = SearchFilter::default();
    eligibility.attributes.push(("acl".into(), "team-a".into()));
    eligibility.property_predicates.push(PropertyPredicate {
        key: "confidence".into(),
        op: PropertyComparisonOp::Gte,
        value: PropertyValue::Float(0.8),
    });
    let frozen = engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), eligibility).unwrap())
        .unwrap();

    // Option A: evidence resolution for doc-asym SUCCEEDS even though src-asym omits `confidence`.
    let ev_search = engine
        .search_with_evidence(&EvidenceSearchRequestV1::new(
            "asymmetrictraceneedle".into(),
            frozen.clone(),
        ))
        .unwrap();
    assert_eq!(ev_search.evidence.len(), 1);
    let ev_resolved = engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: ev_search.evidence[0].evidence_ref.clone(),
            context: frozen.clone(),
        })
        .unwrap();
    assert_eq!(ev_resolved.source_revision_id, "src-asym-r1");

    // ADR-0.8.25 #49 / ADR-0.8.32 #103 §2.1: trace_dependency remains symmetric — src-asym fails
    // `confidence >= 0.8` in Rust `visible_artifact`, so ToSource returns only the visible root.
    let asym_trace = engine
        .trace_dependency(
            DependencyTraceRequestV1::new(
                "doc-asym-r1",
                DependencyTraceDirectionV1::ToSource,
                frozen.clone(),
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(asym_trace.nodes.len(), 1);
    assert_eq!(asym_trace.nodes[0].artifact_revision_id, "doc-asym-r1");
    assert!(asym_trace.dependency_edges.is_empty());

    // Tracing from the ineligible canonical source fails closed with TraceUnavailable.
    let err = engine
        .trace_dependency(
            DependencyTraceRequestV1::new(
                "src-asym-r1",
                DependencyTraceDirectionV1::ToDependents,
                frozen.clone(),
            )
            .unwrap(),
        )
        .unwrap_err();
    assert!(
        matches!(
            err,
            EngineError::DependencyTrace(ref tr)
                if tr.reason == DependencyTraceErrorReasonV1::TraceUnavailable
        ),
        "expected TraceUnavailable for ineligible root, got {err:?}"
    );

    // When both endpoints match `confidence >= 0.8`, trace_dependency returns the full relation.
    let sym_trace = engine
        .trace_dependency(
            DependencyTraceRequestV1::new(
                "doc-sym-r1",
                DependencyTraceDirectionV1::ToSource,
                frozen,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(sym_trace.nodes.len(), 2);
    assert_eq!(sym_trace.dependency_edges.len(), 1);
    assert_eq!(sym_trace.dependency_edges[0].dependency_id, "dep-sym");
    assert_eq!(sym_trace.dependency_edges[0].source_revision_id, "src-sym-r1");

    engine.close().unwrap();
}

#[test]
fn ac32_evidence_prop_03_graph_arm_search_hit_source_id_vs_resolved_evidence_source_id() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("prop-graph-arm-source-id{SQLITE_SUFFIX}"));
    let engine = Engine::open(&path).unwrap().engine;
    configure_acl_and_confidence(&engine);

    let node_src_body = "canonical node provenance body without confidence property";
    let edge_src_body = "canonical edge provenance body without confidence property";
    let acl_only = BTreeMap::from([("acl".into(), PropertyValue::String("team-a".into()))]);
    let doc_props = BTreeMap::from([
        ("acl".into(), PropertyValue::String("team-a".into())),
        ("confidence".into(), PropertyValue::Float(0.95)),
    ]);

    engine
        .write(&[
            canonical_node(
                "node-src",
                "SourceDoc",
                "node-src-r1",
                "node-src-v1",
                "node-source-bucket",
                node_src_body,
                Some(acl_only.clone()),
            ),
            canonical_node(
                "edge-src",
                "SourceDoc",
                "edge-src-r1",
                "edge-src-v1",
                "edge-source-bucket",
                edge_src_body,
                Some(acl_only),
            ),
            derived_node(
                "ent-seed",
                "Doc",
                "ent-seed-r1",
                "node-src-v1",
                "node-src-r1",
                "node-source-bucket",
                node_src_body,
                "quasar_graph_seed_token",
                Some(doc_props.clone()),
            ),
            derived_node(
                "ent-neighbor",
                "Doc",
                "ent-neighbor-r1",
                "node-src-v1",
                "node-src-r1",
                "node-source-bucket",
                node_src_body,
                "unrelated neighbor payload",
                Some(doc_props),
            ),
            derived_edge(
                "ent-link",
                "links",
                "ent-seed",
                "ent-neighbor",
                "ent-link-r1",
                "edge-src-v1",
                "edge-src-r1",
                "edge-source-bucket",
                edge_src_body,
            ),
        ])
        .unwrap();
    engine.drain(10_000).unwrap();

    let mut eligibility = SearchFilter::default();
    eligibility.kind = Some("Doc".into());
    eligibility.attributes.push(("acl".into(), "team-a".into()));
    eligibility.property_predicates.push(PropertyPredicate {
        key: "confidence".into(),
        op: PropertyComparisonOp::Gte,
        value: PropertyValue::Float(0.8),
    });
    let frozen = engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), eligibility).unwrap())
        .unwrap();

    let mut req = EvidenceSearchRequestV1::new("quasar_graph_seed_token".into(), frozen.clone());
    req.use_graph_arm = true;
    let res = engine.search_with_evidence(&req).unwrap();

    let (neighbor_idx, neighbor_hit) = res
        .search_result
        .results
        .iter()
        .enumerate()
        .find(|(_, hit)| {
            hit.id.value == "ent-neighbor" && hit.branch == SoftFallbackBranch::GraphArm
        })
        .unwrap_or_else(|| {
            panic!("expected GraphArm hit for ent-neighbor, got {:?}", res.search_result.results)
        });

    // ADR-0.8.32 #103 §2.2: on GraphArm hits, SearchHit.source_id carries the traversed edge's
    // legacy source_id while ResolvedEvidenceV1.source_id resolves the returned node artifact's
    // own canonical provenance source.
    assert_eq!(neighbor_hit.source_id.as_deref(), Some("edge-source-bucket"));

    let resolved = engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: res.evidence[neighbor_idx].evidence_ref.clone(),
            context: frozen,
        })
        .unwrap();
    assert_eq!(resolved.artifact_revision_id, "ent-neighbor-r1");
    assert_eq!(resolved.source_id, "node-source-bucket");
    assert_ne!(resolved.source_id.as_str(), neighbor_hit.source_id.as_deref().unwrap());
    assert_eq!(resolved.canonical_source_body, node_src_body);
    assert!(matches!(
        resolved.projection_origin.graph_origin,
        Some(EvidenceGraphOriginV1::Traversal {
            ref edge_artifact_revision_id,
            hop_count: 1,
            ..
        }) if edge_artifact_revision_id == "ent-link-r1"
    ));

    engine.close().unwrap();
}
