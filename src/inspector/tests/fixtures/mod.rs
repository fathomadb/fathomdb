use fathomdb_engine::{
    configure_runtime, EmbedderChoice, Engine, EngineConfig, InitialState, PreparedWrite,
    ProjectionFts, ProjectionRole, ProjectionSpec, ProjectionVector, PropertyEntityClass,
    RuntimeSqliteMode, SourceId, WriteProvenanceV1,
};
use std::path::{Path, PathBuf};

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub(crate) fn node_with(
    kind: &str,
    logical_id: Option<&str>,
    body: &str,
    source_id: &str,
    state: InitialState,
    reason: Option<&str>,
    valid_from: Option<i64>,
    valid_until: Option<i64>,
) -> PreparedWrite {
    PreparedWrite::Node {
        kind: kind.into(),
        body: body.into(),
        source_id: SourceId::new(source_id).unwrap(),
        logical_id: logical_id.map(str::to_string),
        state,
        reason: reason.map(str::to_string),
        valid_from,
        valid_until,
        properties: None,
    }
}

pub(crate) fn node(id: &str, body: &str) -> PreparedWrite {
    node_with("document", Some(id), body, "fixture:source", InitialState::Active, None, None, None)
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub(crate) fn edge_with(
    kind: &str,
    from: &str,
    to: &str,
    source_id: &str,
    logical_id: Option<&str>,
    body: Option<String>,
    t_valid: Option<i64>,
    t_invalid: Option<i64>,
    confidence: Option<f64>,
    extractor_model_id: Option<String>,
    temporal_fallback: Option<bool>,
) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: kind.into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new(source_id).unwrap(),
        logical_id: logical_id.map(str::to_string),
        body,
        t_valid,
        t_invalid,
        confidence,
        extractor_model_id,
        temporal_fallback,
        properties: None,
    }
}

#[allow(dead_code)]
pub(crate) fn edge(
    kind: &str,
    from: &str,
    to: &str,
    source_id: &str,
    logical_id: Option<&str>,
    body: Option<String>,
) -> PreparedWrite {
    edge_with(kind, from, to, source_id, logical_id, body, None, None, None, None, None)
}

#[allow(dead_code)]
pub(crate) fn provenanced_node(
    logical_id: Option<&str>,
    body: &str,
    source_id: &str,
    valid_until: Option<i64>,
    provenance: WriteProvenanceV1,
) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(fathomdb_engine::ProvenancedNodeV1 {
        kind: "document".into(),
        body: body.into(),
        source_id: SourceId::new(source_id).unwrap(),
        logical_id: logical_id.map(str::to_string),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until,
        properties: None,
        provenance,
    })
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub(crate) fn provenanced_edge(
    kind: &str,
    from: &str,
    to: &str,
    source_id: &str,
    logical_id: Option<&str>,
    body: Option<String>,
    confidence: Option<f64>,
    extractor_model_id: Option<String>,
    temporal_fallback: Option<bool>,
    provenance: WriteProvenanceV1,
) -> PreparedWrite {
    PreparedWrite::ProvenancedEdge(fathomdb_engine::ProvenancedEdgeV1 {
        kind: kind.into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new(source_id).unwrap(),
        logical_id: logical_id.map(str::to_string),
        body,
        t_valid: None,
        t_invalid: None,
        confidence,
        extractor_model_id,
        temporal_fallback,
        properties: None,
        provenance,
    })
}

#[allow(dead_code)]
pub(crate) fn projection_vector(embedder: Option<String>) -> ProjectionVector {
    ProjectionVector { embedder, dense_readiness: None, kinds: None }
}

#[allow(dead_code)]
pub(crate) fn projection_spec(
    name: &str,
    roles: &[ProjectionRole],
    fts: Option<ProjectionFts>,
    vector: Option<ProjectionVector>,
    source: Option<Vec<String>>,
) -> ProjectionSpec {
    ProjectionSpec {
        name: name.into(),
        roles: roles.iter().copied().collect(),
        fts,
        vector,
        source,
        entity_class: PropertyEntityClass::Node,
    }
}
pub(crate) fn database(dir: &Path) -> PathBuf {
    configure_runtime(RuntimeSqliteMode::Diagnostics).unwrap();
    let path = dir.join("fixture.sqlite");
    let opened =
        Engine::open_with_choice_and_config(&path, EmbedderChoice::None, EngineConfig::default())
            .unwrap();
    opened.engine.write(&[node("alpha", "original text"), node("beta", "original text")]).unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    path
}

pub(crate) fn add_related(path: &Path) {
    use fathomdb_engine::{
        ArtifactRevisionId, CanonicalHash, SourceLocator, SourceRevisionId, SourceVersionId,
    };
    use sha2::{Digest, Sha256};
    let source = r#"{"title":"source-title","text":"source body"}"#;
    let digest: String =
        Sha256::digest(source.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    let make = |id: &str, body: &str, provenance| {
        provenanced_node(Some(id), body, "fixture:canonical", None, provenance)
    };
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .configure_projections(
            &[projection_spec("title", &[ProjectionRole::Filterable], None, None, None)],
            &[],
        )
        .unwrap();
    opened
        .engine
        .write(&[
            make(
                "source",
                source,
                WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("source-r1").unwrap(),
                    SourceVersionId::new("source-v1").unwrap(),
                ),
            ),
            make(
                "derived",
                r#"{"title":"derived-title","text":"derived body"}"#,
                WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("derived-r1").unwrap(),
                    SourceVersionId::new("source-v1").unwrap(),
                    SourceRevisionId::new("source-r1").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(digest).unwrap(),
                ),
            ),
        ])
        .unwrap();
    opened.engine.close().unwrap();
}

pub(crate) fn add_erasure_cases(path: &Path) {
    add_related(path);
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .write(&[
            node("alpha", "new retained revision"),
            node_with(
                "document",
                Some("pending"),
                "pending retained body",
                "fixture:pending",
                InitialState::Pending,
                Some("fixture state"),
                None,
                None,
            ),
        ])
        .unwrap();
    opened
        .engine
        .register_source_dependency(
            fathomdb_engine::SourceDependencyRegistrationV1::new(
                "fixture-dependency",
                "source-r1",
                "derived-r1",
            )
            .unwrap(),
        )
        .unwrap();
    opened.engine.transition("source", fathomdb_engine::LifecycleState::Deleted, None).unwrap();
    let sink = path.parent().unwrap().join("unavailable-telemetry.jsonl");
    opened.engine.enable_telemetry(sink.to_str().unwrap()).unwrap();
    std::fs::remove_file(&sink).unwrap();
    std::fs::create_dir(&sink).unwrap();
    let result = opened.engine.purge("source");
    assert!(
        matches!(result,Err(fathomdb_engine::EngineError::ErasureIncomplete{ref stage,..}) if stage=="telemetry_redaction")
    );
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_scale(path: &Path) {
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::AdminSchema {
            name: "scale-events".into(),
            kind: "append_only_log".into(),
            schema_json: r#"{"type":"object"}"#.into(),
            retention_json: "{}".into(),
        }])
        .unwrap();
    for batch in 0..100 {
        let writes = (batch * 1000..(batch + 1) * 1000)
            .map(|n| {
                node(
                    &format!("scale-{n}"),
                    &format!(r#"{{"sequence":{n},"title":"scale title","optional":null}}"#),
                )
            })
            .collect::<Vec<_>>();
        opened.engine.write(&writes).unwrap();
        if batch % 10 == 9 {
            eprintln!("scale canonical rows: {}", (batch + 1) * 1000);
        }
    }
    for batch in 0..100 {
        let writes = (batch * 1000..(batch + 1) * 1000)
            .map(|n| PreparedWrite::OpStore {
                collection: "scale-events".into(),
                record_key: format!("repeated-{}", n % 3),
                schema_id: None,
                body: format!(r#"{{"sequence":{n}}}"#),
            })
            .collect::<Vec<_>>();
        opened.engine.write(&writes).unwrap();
        if batch % 10 == 9 {
            eprintln!("scale retained mutations: {}", (batch + 1) * 1000);
        }
    }
    for batch in 0..10 {
        let writes = (batch * 1000..(batch + 1) * 1000)
            .map(|n| {
                edge(
                    "scale-incident",
                    "alpha",
                    &format!("scale-{n}"),
                    "fixture:scale",
                    Some(&format!("scale-edge-{n}")),
                    Some("incident edge".into()),
                )
            })
            .collect::<Vec<_>>();
        opened.engine.write(&writes).unwrap();
    }
    let body =
        serde_json::json!({"large":"é🦀".repeat(1_800_000),"integer":9007199254740993123_u64})
            .to_string();
    assert!(body.len() >= 10 * 1024 * 1024);
    opened.engine.write(&[node("scale-large", &body)]).unwrap();
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_nullable(path: &Path) {
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    let writes = [("null-edge", None), ("empty-edge", Some(String::new()))]
        .map(|(id, body)| edge("nullable", "alpha", "beta", "fixture:nullable", Some(id), body));
    opened.engine.write(&writes).unwrap();
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_metadata(path: &Path) {
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    let reason = "retained reason é🦀".repeat(20000);
    opened
        .engine
        .write(&[
            node_with(
                "document",
                Some("metadata"),
                "metadata fixture body",
                "fixture:metadata",
                InitialState::Pending,
                Some(&reason),
                Some(1),
                None,
            ),
            edge_with(
                "precision",
                "alpha",
                "beta",
                "fixture:metadata",
                Some("metadata-edge"),
                None,
                None,
                None,
                Some(0.12345678901234566),
                Some(String::new()),
                Some(true),
            ),
        ])
        .unwrap();
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_catalog_cap_cases(path: &Path) {
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .configure_projections(
            &[projection_spec("title", &[ProjectionRole::Searchable], None, None, None)],
            &[],
        )
        .unwrap();
    let mut schemas = (0..200)
        .map(|n| PreparedWrite::AdminSchema {
            name: format!("catalog-state-{n:03}"),
            kind: "latest_state".into(),
            schema_json: r#"{"type":"object"}"#.into(),
            retention_json: "{}".into(),
        })
        .collect::<Vec<_>>();
    for (name, kind) in [
        (format!("0000-{}", "🦀".repeat(5000)), "latest_state"),
        ("zz-current".into(), "latest_state"),
        ("zz-events".into(), "append_only_log"),
    ] {
        schemas.push(PreparedWrite::AdminSchema {
            name,
            kind: kind.into(),
            schema_json: r#"{"type":"object"}"#.into(),
            retention_json: "{}".into(),
        });
    }
    opened.engine.write(&schemas).unwrap();
    for collection in ["zz-current", "zz-events"] {
        for n in [1, 2] {
            opened
                .engine
                .write(&[PreparedWrite::OpStore {
                    collection: collection.into(),
                    record_key: "shared-key".into(),
                    schema_id: None,
                    body: format!("{{\"n\":{n}}}"),
                }])
                .unwrap();
        }
    }
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_many_related(path: &Path) {
    use fathomdb_engine::{
        ArtifactRevisionId, CanonicalHash, SourceLocator, SourceRevisionId, SourceVersionId,
    };
    use sha2::{Digest, Sha256};
    add_related(path);
    let digest: String =
        Sha256::digest(r#"{"title":"source-title","text":"source body"}"#.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .write(&[provenanced_node(
            Some("derived"),
            r#"{"title":"new-derived-title","text":"second derived body"}"#,
            "fixture:canonical",
            None,
            WriteProvenanceV1::derived(
                ArtifactRevisionId::new("derived-r2").unwrap(),
                SourceVersionId::new("source-v1").unwrap(),
                SourceRevisionId::new("source-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest).unwrap(),
            ),
        )])
        .unwrap();
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_chunked(path: &Path) {
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    let body = format!("{}{}{}", "A".repeat(65536), "B".repeat(65536), "C".repeat(1200));
    opened.engine.write(&[node("chunked", &body)]).unwrap();
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_lexical(path: &Path) {
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .configure_projections(
            &[projection_spec(
                "biography",
                &[ProjectionRole::Searchable, ProjectionRole::Filterable],
                Some(ProjectionFts { tokenizer: Some("unicode61".into()) }),
                None,
                Some(vec!["profile".into(), "biography".into()]),
            )],
            &[],
        )
        .unwrap();
    for (id, text, biography) in [
        (
            "lexical-a",
            "Café runners <img src=https://evil.invalid/x onerror=alert(1)>",
            "orchid orchid",
        ),
        ("lexical-b", "runner blue", "orchid"),
        ("lexical-twin", "runner blue", "orchid"),
        ("lexical-c", "stone", "stone"),
        ("lexical-d", "pebble", "pebble"),
    ] {
        opened
            .engine
            .write(&[node(
                id,
                &serde_json::json!({"text":text,"profile":{"biography":biography},"status":"open"})
                    .to_string(),
            )])
            .unwrap();
    }
    opened
        .engine
        .write(&[edge(
            "knows",
            "lexical-a",
            "lexical-b",
            "fixture:lexical-edge",
            Some("lexical-edge"),
            Some("café runner edge".into()),
        )])
        .unwrap();
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_lexical_envelope(path: &Path) -> serde_json::Value {
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    let names = (0..8).map(|n| format!("{}-{n}", "a".repeat(4090))).collect::<Vec<_>>();
    let value = "v".repeat(4096);
    opened
        .engine
        .configure_projections(
            &names
                .iter()
                .map(|name| projection_spec(name, &[ProjectionRole::Filterable], None, None, None))
                .collect::<Vec<_>>(),
            &[],
        )
        .unwrap();
    let prefix = "i".repeat(4080);
    let source_id = "s".repeat(4096);
    let reason = "🦀".repeat(4096);
    let mut writes = vec![];
    for n in 0..100 {
        let mut body = serde_json::Map::new();
        for name in &names {
            body.insert(name.clone(), serde_json::json!(value));
        }
        body.insert("text".into(), serde_json::json!("needle"));
        body.insert("number".into(), serde_json::json!(n));
        let logical_id = format!("{prefix}-{n}");
        writes.push(node_with(
            "document",
            Some(&logical_id),
            &serde_json::Value::Object(body).to_string(),
            &source_id,
            InitialState::Active,
            Some(&reason),
            None,
            None,
        ));
    }
    opened.engine.write(&writes).unwrap();
    opened.engine.close().unwrap();
    serde_json::json!({"op":"lexical","route":"body","query":"needle","filter":{"attributes":names.iter().map(|name|(name.clone(),value.clone())).collect::<Vec<_>>()},"view":{},"limit":100})
}

#[allow(dead_code)]
pub(crate) fn add_lexical_native(path: &Path) {
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    let text = (0..800).map(|n| format!("q{n}")).collect::<Vec<_>>().join(" ");
    for batch in 0..35 {
        let writes = (batch * 100..(batch + 1) * 100)
            .map(|n| node(&format!("native-lexical-{n}"), &format!("{text} row{n}")))
            .collect::<Vec<_>>();
        opened.engine.write(&writes).unwrap();
    }
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_graph(path: &Path) {
    use fathomdb_engine::{
        ArtifactRevisionId, CanonicalHash, SourceDependencyRegistrationV1, SourceLocator,
        SourceRevisionId, SourceVersionId,
    };
    use sha2::{Digest, Sha256};
    add_many_related(path);
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("graph-dependency", "source-r1", "derived-r1")
                .unwrap(),
        )
        .unwrap();
    let source = r#"{"title":"source-title","text":"source body"}"#;
    let digest: String =
        Sha256::digest(source.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    opened
        .engine
        .write(&[provenanced_node(
            Some("alpha"),
            "native graph alpha target",
            "fixture:canonical",
            None,
            WriteProvenanceV1::derived(
                ArtifactRevisionId::new("alpha-r1").unwrap(),
                SourceVersionId::new("source-v1").unwrap(),
                SourceRevisionId::new("source-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest.clone()).unwrap(),
            ),
        )])
        .unwrap();
    let mut writes = vec![];
    for (id, from, to, kind, body) in [
        (
            "parallel-a",
            "derived",
            "alpha",
            "supports",
            Some("full edge é🦀<img src=https://evil.invalid/x>".repeat(5000)),
        ),
        ("parallel-b", "derived", "alpha", "supports", None),
        ("self", "derived", "derived", "reflexive", Some(String::new())),
        ("cycle", "alpha", "derived", "returns", Some("cycle body".into())),
        ("missing", "derived", "not-retained", "supports", Some("missing endpoint body".into())),
    ] {
        writes.push(provenanced_edge(
            kind,
            from,
            to,
            "fixture:canonical",
            Some(id),
            body,
            Some(0.75),
            Some("extractor-v1".into()),
            Some(false),
            WriteProvenanceV1::derived(
                ArtifactRevisionId::new(format!("{id}-r1")).unwrap(),
                SourceVersionId::new("source-v1").unwrap(),
                SourceRevisionId::new("source-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest.clone()).unwrap(),
            ),
        ));
    }
    opened.engine.write(&writes).unwrap();
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_graph_visual(path: &Path) {
    add_graph(path);
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    let writes = (0..10)
        .map(|i| {
            edge(
                "visual",
                "derived",
                if i < 2 { "derived" } else { "alpha" },
                "fixture:visual",
                Some(&format!("visual-{i}")),
                None,
            )
        })
        .collect::<Vec<_>>();
    opened.engine.write(&writes).unwrap();
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_graph_missing_prefix(path: &Path) {
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    let from = "p".repeat(5000);
    let to = format!("{}q", "p".repeat(5000));
    opened
        .engine
        .write(&[edge(
            "missing-prefix",
            &from,
            &to,
            "fixture:missing-prefix",
            Some("prefix-edge"),
            None,
        )])
        .unwrap();
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_graph_highdegree(path: &Path) {
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    let writes = (0..160)
        .map(|i| {
            edge(
                "bounded-incident",
                "alpha",
                &format!("missing-{i}"),
                "fixture:highdegree",
                Some(&format!("incident-{i}")),
                None,
            )
        })
        .collect::<Vec<_>>();
    opened.engine.write(&writes).unwrap();
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn comparison_policy(dir: &Path) -> PathBuf {
    use fathomdb_engine::{
        EngineConfig, PhysicalErasurePolicy, PhysicalErasureSelector, QueueResult,
    };
    use std::sync::Arc;
    let path = dir.join("comparison-policy.sqlite");
    let opened = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::None,
        EngineConfig {
            physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
            ..Default::default()
        },
    )
    .unwrap();
    opened
        .engine
        .write(&[node("alpha", "old comparison needle"), node("beta", "safe unrelated body")])
        .unwrap();
    opened
        .engine
        .write(&[
            node("alpha", "current comparison needle"),
            node_with(
                "document",
                Some("pending"),
                "pending retained body",
                "fixture:source",
                InitialState::Pending,
                Some("fixture state"),
                None,
                None,
            ),
            PreparedWrite::AdminSchema {
                name: "state".into(),
                kind: "latest_state".into(),
                schema_json: r#"{"type":"object"}"#.into(),
                retention_json: "{}".into(),
            },
        ])
        .unwrap();
    opened
        .engine
        .write(&[PreparedWrite::OpStore {
            collection: "state".into(),
            record_key: "9007199254740993-é🦀".into(),
            schema_id: None,
            body: r#"{"payload":"ordinary exact operational value"}"#.into(),
        }])
        .unwrap();
    opened.engine.drain(25000).unwrap();
    let engine = Arc::new(opened.engine);
    let mut maintenance = engine.begin_erasure_maintenance(1000).unwrap();
    assert!(matches!(
        maintenance
            .queue_physical_erasure(
                PhysicalErasureSelector::SourceAllAtCut { source_id: "fixture:source".into() },
                "60606060606060606060606060606060"
            )
            .unwrap(),
        QueueResult::Accepted { .. }
    ));
    maintenance.close().unwrap();
    drop(maintenance);
    engine.close().unwrap();
    path
}

#[allow(dead_code)]
pub(crate) fn add_comparison_expiry(path: &Path) {
    use fathomdb_engine::{
        ArtifactRevisionId, CanonicalHash, SourceLocator, SourceRevisionId, SourceVersionId,
    };
    use sha2::{Digest, Sha256};
    add_graph(path);
    let source = r#"{"title":"source-title","text":"source body"}"#;
    let digest: String =
        Sha256::digest(source.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    let end = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        as i64
        + 8;
    opened
        .engine
        .write(&[provenanced_node(
            Some("alpha"),
            "alpha expires after unpin",
            "fixture:canonical",
            Some(end),
            WriteProvenanceV1::derived(
                ArtifactRevisionId::new("alpha-expiring-r1").unwrap(),
                SourceVersionId::new("source-v1").unwrap(),
                SourceRevisionId::new("source-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest).unwrap(),
            ),
        )])
        .unwrap();
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_ux_hub(path: &Path) {
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    let mut writes = vec![
        node("ux-seed", r#"{"title":"Seed note","body":"linked project"}"#),
        node("ux-hub", r#"{"title":"Compliance project","body":"many linked notes"}"#),
    ];
    for n in 0..210 {
        writes.push(node(&format!("ux-leaf-{n}"), "ordinary linked note"));
    }
    let ux_edge = |id: String, from: String, to: String| {
        edge(
            "mentions_project",
            &from,
            &to,
            "fixture:ux",
            Some(&id),
            Some("recorded project mention".into()),
        )
    };
    writes.push(ux_edge("ux-first".into(), "ux-seed".into(), "ux-hub".into()));
    for n in 0..210 {
        writes.push(ux_edge(format!("ux-edge-{n}"), "ux-hub".into(), format!("ux-leaf-{n}")));
    }
    opened.engine.write(&writes).unwrap();
    opened.engine.close().unwrap();
}
