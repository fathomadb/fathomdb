use fathomdb_engine::{
    configure_runtime, EmbedderChoice, Engine, EngineConfig, InitialState, PreparedWrite,
    RuntimeSqliteMode, SourceId,
};
use std::path::{Path, PathBuf};

pub(crate) fn node(id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "document".into(),
        body: body.into(),
        source_id: SourceId::new("fixture:source").unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
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
        ArtifactRevisionId, CanonicalHash, ProjectionRole, ProjectionSpec, ProvenancedNodeV1,
        SourceLocator, SourceRevisionId, SourceVersionId, WriteProvenanceV1,
    };
    use sha2::{Digest, Sha256};
    let source = r#"{"title":"source-title","text":"source body"}"#;
    let digest: String =
        Sha256::digest(source.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    let make = |id: &str, body: &str, provenance| {
        PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            kind: "document".into(),
            body: body.into(),
            source_id: SourceId::new("fixture:canonical").unwrap(),
            logical_id: Some(id.into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance,
        })
    };
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .configure_projections(
            &[ProjectionSpec {
                name: "title".into(),
                roles: std::collections::BTreeSet::from([ProjectionRole::Filterable]),
                fts: None,
                vector: None,
                source: None,
            }],
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
            PreparedWrite::Node {
                kind: "document".into(),
                body: "pending retained body".into(),
                source_id: SourceId::new("fixture:pending").unwrap(),
                logical_id: Some("pending".into()),
                state: InitialState::Pending,
                reason: Some("fixture state".into()),
                valid_from: None,
                valid_until: None,
            },
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
            .map(|n| PreparedWrite::Edge {
                kind: "scale-incident".into(),
                from: "alpha".into(),
                to: format!("scale-{n}"),
                source_id: SourceId::new("fixture:scale").unwrap(),
                logical_id: Some(format!("scale-edge-{n}")),
                body: Some("incident edge".into()),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
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
    let writes = [("null-edge", None), ("empty-edge", Some(String::new()))].map(|(id, body)| {
        PreparedWrite::Edge {
            kind: "nullable".into(),
            from: "alpha".into(),
            to: "beta".into(),
            source_id: SourceId::new("fixture:nullable").unwrap(),
            logical_id: Some(id.into()),
            body,
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
        }
    });
    opened.engine.write(&writes).unwrap();
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_metadata(path: &Path) {
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .write(&[
            PreparedWrite::Node {
                kind: "document".into(),
                body: "metadata fixture body".into(),
                source_id: SourceId::new("fixture:metadata").unwrap(),
                logical_id: Some("metadata".into()),
                state: InitialState::Pending,
                reason: Some("retained reason é🦀".repeat(20000)),
                valid_from: Some(1),
                valid_until: None,
            },
            PreparedWrite::Edge {
                kind: "precision".into(),
                from: "alpha".into(),
                to: "beta".into(),
                source_id: SourceId::new("fixture:metadata").unwrap(),
                logical_id: Some("metadata-edge".into()),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.12345678901234566),
                extractor_model_id: Some(String::new()),
                temporal_fallback: Some(true),
            },
        ])
        .unwrap();
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_catalog_cap_cases(path: &Path) {
    use fathomdb_engine::{ProjectionRole, ProjectionSpec};
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .configure_projections(
            &[ProjectionSpec {
                name: "title".into(),
                roles: std::collections::BTreeSet::from([ProjectionRole::Searchable]),
                fts: None,
                vector: None,
                source: None,
            }],
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
        ArtifactRevisionId, CanonicalHash, ProvenancedNodeV1, SourceLocator, SourceRevisionId,
        SourceVersionId, WriteProvenanceV1,
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
        .write(&[PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            kind: "document".into(),
            body: r#"{"title":"new-derived-title","text":"second derived body"}"#.into(),
            source_id: SourceId::new("fixture:canonical").unwrap(),
            logical_id: Some("derived".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: WriteProvenanceV1::derived(
                ArtifactRevisionId::new("derived-r2").unwrap(),
                SourceVersionId::new("source-v1").unwrap(),
                SourceRevisionId::new("source-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest).unwrap(),
            ),
        })])
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
    use fathomdb_engine::{ProjectionFts, ProjectionRole, ProjectionSpec};
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .configure_projections(
            &[ProjectionSpec {
                name: "biography".into(),
                roles: std::collections::BTreeSet::from([
                    ProjectionRole::Searchable,
                    ProjectionRole::Filterable,
                ]),
                fts: Some(ProjectionFts { tokenizer: Some("unicode61".into()) }),
                vector: None,
                source: Some(vec!["profile".into(), "biography".into()]),
            }],
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
        .write(&[PreparedWrite::Edge {
            kind: "knows".into(),
            from: "lexical-a".into(),
            to: "lexical-b".into(),
            source_id: SourceId::new("fixture:lexical-edge").unwrap(),
            logical_id: Some("lexical-edge".into()),
            body: Some("café runner edge".into()),
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
        }])
        .unwrap();
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_lexical_envelope(path: &Path) -> serde_json::Value {
    use fathomdb_engine::{ProjectionRole, ProjectionSpec};
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    let names = (0..8).map(|n| format!("{}-{n}", "a".repeat(4090))).collect::<Vec<_>>();
    let value = "v".repeat(4096);
    opened
        .engine
        .configure_projections(
            &names
                .iter()
                .map(|name| ProjectionSpec {
                    name: name.clone(),
                    roles: std::collections::BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                })
                .collect::<Vec<_>>(),
            &[],
        )
        .unwrap();
    let prefix = "i".repeat(4080);
    let mut writes = vec![];
    for n in 0..100 {
        let mut body = serde_json::Map::new();
        for name in &names {
            body.insert(name.clone(), serde_json::json!(value));
        }
        body.insert("text".into(), serde_json::json!("needle"));
        body.insert("number".into(), serde_json::json!(n));
        writes.push(PreparedWrite::Node {
            kind: "document".into(),
            body: serde_json::Value::Object(body).to_string(),
            source_id: SourceId::new("s".repeat(4096)).unwrap(),
            logical_id: Some(format!("{prefix}-{n}")),
            state: InitialState::Active,
            reason: Some("🦀".repeat(4096)),
            valid_from: None,
            valid_until: None,
        });
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
        ArtifactRevisionId, CanonicalHash, ProvenancedEdgeV1, SourceDependencyRegistrationV1,
        SourceLocator, SourceRevisionId, SourceVersionId, WriteProvenanceV1,
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
        .write(&[PreparedWrite::ProvenancedNode(fathomdb_engine::ProvenancedNodeV1 {
            kind: "document".into(),
            body: "native graph alpha target".into(),
            source_id: SourceId::new("fixture:canonical").unwrap(),
            logical_id: Some("alpha".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: WriteProvenanceV1::derived(
                ArtifactRevisionId::new("alpha-r1").unwrap(),
                SourceVersionId::new("source-v1").unwrap(),
                SourceRevisionId::new("source-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest.clone()).unwrap(),
            ),
        })])
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
        writes.push(PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
            kind: kind.into(),
            from: from.into(),
            to: to.into(),
            source_id: SourceId::new("fixture:canonical").unwrap(),
            logical_id: Some(id.into()),
            body,
            t_valid: None,
            t_invalid: None,
            confidence: Some(0.75),
            extractor_model_id: Some("extractor-v1".into()),
            temporal_fallback: Some(false),
            provenance: WriteProvenanceV1::derived(
                ArtifactRevisionId::new(format!("{id}-r1")).unwrap(),
                SourceVersionId::new("source-v1").unwrap(),
                SourceRevisionId::new("source-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest.clone()).unwrap(),
            ),
        }));
    }
    opened.engine.write(&writes).unwrap();
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_graph_visual(path: &Path) {
    add_graph(path);
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    let writes = (0..10)
        .map(|i| PreparedWrite::Edge {
            kind: "visual".into(),
            from: "derived".into(),
            to: if i < 2 { "derived".into() } else { "alpha".into() },
            source_id: SourceId::new("fixture:visual").unwrap(),
            logical_id: Some(format!("visual-{i}")),
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
        })
        .collect::<Vec<_>>();
    opened.engine.write(&writes).unwrap();
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_graph_missing_prefix(path: &Path) {
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::Edge {
            kind: "missing-prefix".into(),
            from: "p".repeat(5000),
            to: format!("{}q", "p".repeat(5000)),
            source_id: SourceId::new("fixture:missing-prefix").unwrap(),
            logical_id: Some("prefix-edge".into()),
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
        }])
        .unwrap();
    opened.engine.close().unwrap();
}

#[allow(dead_code)]
pub(crate) fn add_graph_highdegree(path: &Path) {
    let opened = Engine::open_with_choice(path, EmbedderChoice::None).unwrap();
    let writes = (0..160)
        .map(|i| PreparedWrite::Edge {
            kind: "bounded-incident".into(),
            from: "alpha".into(),
            to: format!("missing-{i}"),
            source_id: SourceId::new("fixture:highdegree").unwrap(),
            logical_id: Some(format!("incident-{i}")),
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
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
            PreparedWrite::Node {
                kind: "document".into(),
                body: "pending retained body".into(),
                source_id: SourceId::new("fixture:source").unwrap(),
                logical_id: Some("pending".into()),
                state: InitialState::Pending,
                reason: Some("fixture state".into()),
                valid_from: None,
                valid_until: None,
            },
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
        ArtifactRevisionId, CanonicalHash, ProvenancedNodeV1, SourceLocator, SourceRevisionId,
        SourceVersionId, WriteProvenanceV1,
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
        .write(&[PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            kind: "document".into(),
            body: "alpha expires after unpin".into(),
            source_id: SourceId::new("fixture:canonical").unwrap(),
            logical_id: Some("alpha".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: Some(end),
            provenance: WriteProvenanceV1::derived(
                ArtifactRevisionId::new("alpha-expiring-r1").unwrap(),
                SourceVersionId::new("source-v1").unwrap(),
                SourceRevisionId::new("source-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest).unwrap(),
            ),
        })])
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
    let edge = |id: String, from: String, to: String| PreparedWrite::Edge {
        kind: "mentions_project".into(),
        from,
        to,
        logical_id: Some(id),
        source_id: SourceId::new("fixture:ux").unwrap(),
        body: Some("recorded project mention".into()),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    };
    writes.push(edge("ux-first".into(), "ux-seed".into(), "ux-hub".into()));
    for n in 0..210 {
        writes.push(edge(format!("ux-edge-{n}"), "ux-hub".into(), format!("ux-leaf-{n}")));
    }
    opened.engine.write(&writes).unwrap();
    opened.engine.close().unwrap();
}
