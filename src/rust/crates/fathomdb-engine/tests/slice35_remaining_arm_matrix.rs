use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    Engine, InitialState, PreparedWrite, ReadContextV1, ReadView, SearchFilter, SoftFallbackBranch,
    SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

#[derive(Clone, Debug)]
struct ConstantEmbedder;

impl Embedder for ConstantEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice35-arm-matrix", "v1", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    }
}

fn node(id: &str, kind: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        logical_id: Some(id.to_string()),
        kind: kind.to_string(),
        body: body.to_string(),
        source_id: SourceId::new("test:slice35-arm-matrix").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn edge(id: &str, kind: &str, from: &str, to: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Edge {
        logical_id: Some(id.to_string()),
        kind: kind.to_string(),
        from: from.to_string(),
        to: to.to_string(),
        body: Some(body.to_string()),
        source_id: SourceId::new("test:slice35-arm-matrix").unwrap(),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}

fn context(kind: &str) -> ReadContextV1 {
    let mut filter = SearchFilter::default();
    filter.kind = Some(kind.to_string());
    ReadContextV1::new(ReadView::default(), filter).unwrap()
}

#[test]
fn edge_fts_applies_eligibility_before_its_candidate_limit() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(dir.path().join(format!("edge{SQLITE_SUFFIX}"))).unwrap();
    let mut writes = vec![node("left", "entity", "left"), node("right", "entity", "right")];
    for index in 0..150 {
        writes.push(edge(
            &format!("excluded-{index:03}"),
            "excluded-relation",
            "left",
            "right",
            "common edge needle",
        ));
    }
    writes.push(edge("eligible", "target-relation", "left", "right", "common edge needle"));
    opened.engine.write(&writes).unwrap();
    let frozen = opened.engine.freeze_read_context(&context("target-relation")).unwrap();

    let result = opened
        .engine
        .search_frozen("common edge needle", &frozen, 0, false, 0.3, 0, false, 1)
        .unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].id.value, "eligible");
    assert_eq!(result.results[0].branch, SoftFallbackBranch::TextEdge);
}

#[test]
fn vector_knn_applies_native_eligibility_before_its_candidate_limit() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open_with_embedder_for_test(
        dir.path().join(format!("vector{SQLITE_SUFFIX}")),
        Arc::new(ConstantEmbedder),
    )
    .unwrap();
    let mut writes = (0..101)
        .map(|index| node(&format!("excluded-{index:03}"), "excluded", "vector decoy"))
        .collect::<Vec<_>>();
    writes.push(node("eligible", "target", "vector target"));
    opened.engine.write(&writes).unwrap();
    opened.engine.configure_vector_kind_for_test("excluded").unwrap();
    opened.engine.configure_vector_kind_for_test("target").unwrap();
    let vector_hex = format!("0000803f{}", "00000000".repeat(7));
    let mut vector_rows = String::new();
    for cursor in 1..=102 {
        let kind = if cursor == 102 { "target" } else { "excluded" };
        vector_rows.push_str(&format!(
            "INSERT INTO _fathomdb_vector_rows(rowid,kind,write_cursor) \
             VALUES({cursor},'{kind}',{cursor}); \
             INSERT INTO vector_default( \
               rowid,embedding,embedding_bin,source_type,kind,created_at,status \
             ) VALUES( \
               {cursor},X'{vector_hex}',vec_quantize_binary(X'{vector_hex}'), \
               'node_body','{kind}',0,'' \
             );"
        ));
    }
    opened.engine.execute_for_test(&vector_rows).unwrap();
    let frozen = opened.engine.freeze_read_context(&context("target")).unwrap();

    let result = opened
        .engine
        .search_frozen("no lexical overlap", &frozen, 0, false, 0.3, 0, false, 1)
        .unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].id.value, "eligible");
    assert_eq!(result.results[0].branch, SoftFallbackBranch::Vector);
}

#[test]
fn graph_seed_selection_does_not_count_ineligible_fts_candidates() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(dir.path().join(format!("graph-seed{SQLITE_SUFFIX}"))).unwrap();
    let mut writes = Vec::new();
    for index in 0..150 {
        writes.push(node(&format!("excluded-{index:03}"), "excluded", "common seed needle"));
    }
    writes.push(node("eligible-seed", "target", "common seed needle"));
    writes.push(node("eligible-neighbor", "target", "graph-only neighbor"));
    writes.push(edge("link", "link", "eligible-seed", "eligible-neighbor", "unrelated"));
    opened.engine.write(&writes).unwrap();
    let frozen = opened.engine.freeze_read_context(&context("target")).unwrap();

    let result = opened
        .engine
        .search_frozen("common seed needle", &frozen, 0, true, 0.3, 0, false, 10)
        .unwrap();

    assert!(result.results.iter().any(|hit| {
        hit.id.value == "eligible-neighbor" && hit.branch == SoftFallbackBranch::GraphArm
    }));
}
