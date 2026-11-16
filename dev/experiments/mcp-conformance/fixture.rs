use fathomdb_sdk::{
    Engine, InitialState, OpenOptions, PreparedWrite, ProjectionRole, ProjectionSpec, SourceId,
};

fn node(id: &str, body: &str, state: InitialState) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "note".into(),
        body: body.into(),
        source_id: SourceId::new("conformance").unwrap(),
        logical_id: Some(id.into()),
        state,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("database path");
    assert!(!std::path::Path::new(&path).exists());
    let engine = Engine::open(path, OpenOptions::default()).unwrap();
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "group".into(),
                roles: [ProjectionRole::Filterable].into(),
                fts: None,
                vector: None,
                source: None,
            }],
            &[],
        )
        .unwrap();
    engine
        .write(&[
            node(
                "note:1",
                "{\"text\":\"alpha first café 🧭\",\"group\":\"wanted\"}",
                InitialState::Active,
            ),
            node(
                "note:2",
                "{\"text\":\"alpha second\",\"group\":\"wanted\"}",
                InitialState::Active,
            ),
            node("note:3", "{\"text\":\"alpha third\",\"group\":\"other\"}", InitialState::Active),
            node("version", "alpha old edition", InitialState::Active),
            node("pending", "alpha pending edition", InitialState::Pending),
            PreparedWrite::Edge {
                kind: "mentions".into(),
                from: "note:1".into(),
                to: "note:2".into(),
                source_id: SourceId::new("conformance").unwrap(),
                logical_id: Some("edge:1".into()),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .unwrap();
    engine.write(&[node("version", "alpha current edition", InitialState::Active)]).unwrap();
    engine.close().unwrap();
}
