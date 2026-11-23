//! Snapshot-local record identity preserves equal-body records and their owners.

use fathomdb_engine::{
    EmbedderChoice, Engine, IdSpace, InitialState, PreparedWrite, ReadView, SearchResult,
    SnippetAnchor, SnippetOptions, SoftFallbackBranch, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

fn node(id: Option<&str>, source: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new(source).unwrap(),
        logical_id: id.map(str::to_owned),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
    }
}

fn search(engine: &Engine, query: &str, view: &ReadView) -> SearchResult {
    let mut snippet = SnippetOptions::default();
    snippet.max_bytes = 40;
    snippet.highlights = true;
    engine
        .search_reranked_view_with_snippets(
            query,
            None,
            0,
            false,
            0.3,
            0,
            true,
            view,
            10,
            Some(&snippet),
        )
        .unwrap()
}

fn assert_records(result: &SearchResult, cursors: &[u64], sources: &[&str], body: &str) {
    assert_eq!(
        result.results.iter().map(|hit| hit.write_cursor).collect::<Vec<_>>(),
        cursors,
        "eligible stored records remain distinct even when their bodies are equal"
    );
    assert_eq!(
        result.results.iter().map(|hit| hit.source_id.as_deref()).collect::<Vec<_>>(),
        sources.iter().copied().map(Some).collect::<Vec<_>>()
    );
    for (rank, hit) in result.results.iter().enumerate() {
        assert_eq!(hit.body, body);
        assert_eq!(hit.score.to_bits(), (3.0_f64 / (30.0 + rank as f64 + 1.0)).to_bits());
    }
    let explanation = result.explanation.as_ref().unwrap();
    for (rank, (hit, detail)) in result.results.iter().zip(&explanation.per_hit).enumerate() {
        assert_eq!(detail.id, hit.write_cursor);
        assert_eq!(detail.text_rank, Some(rank as u32));
        assert_eq!(detail.fused_score.to_bits(), hit.score.to_bits());
    }
}

#[test]
fn equal_body_governed_records_keep_their_own_ids_and_sources() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open_with_choice(
        dir.path().join(format!("governed{SQLITE_SUFFIX}")),
        EmbedderChoice::None,
    )
    .unwrap();
    let body = "peculiar wombat telemetry";
    let receipt = opened
        .engine
        .write(&[node(Some("note:a"), "src:a", body), node(Some("note:b"), "src:b", body)])
        .unwrap();
    let result = search(&opened.engine, "wombat", &ReadView::default());
    assert_records(&result, &receipt.row_cursors, &["src:a", "src:b"], body);
    assert_eq!(result.results[0].id, IdSpace::logical("note:a"));
    assert_eq!(result.results[1].id, IdSpace::logical("note:b"));
    opened.engine.close().unwrap();
}

#[test]
fn equal_body_anonymous_and_governed_records_preserve_both_id_spaces() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open_with_choice(
        dir.path().join(format!("mixed{SQLITE_SUFFIX}")),
        EmbedderChoice::None,
    )
    .unwrap();
    let body = "peculiar wombat telemetry";
    let receipt = opened
        .engine
        .write(&[node(None, "src:a", body), node(Some("note:b"), "src:b", body)])
        .unwrap();
    let result = search(&opened.engine, "wombat", &ReadView::default());
    assert_records(&result, &receipt.row_cursors, &["src:a", "src:b"], body);
    assert_eq!(result.results[0].id.space, fathomdb_engine::IdSpaceKind::Content);
    assert_eq!(result.results[1].id, IdSpace::logical("note:b"));
    opened.engine.close().unwrap();
}

#[test]
fn same_body_historical_versions_keep_their_record_and_source() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open_with_choice(
        dir.path().join(format!("history{SQLITE_SUFFIX}")),
        EmbedderChoice::None,
    )
    .unwrap();
    let body = "peculiar wombat telemetry";
    let first = opened.engine.write(&[node(Some("note:a"), "src:old", body)]).unwrap();
    let second = opened.engine.write(&[node(Some("note:a"), "src:new", body)]).unwrap();
    let view = ReadView { include_superseded: true, ..ReadView::default() };
    let result = search(&opened.engine, "wombat", &view);
    assert_records(
        &result,
        &[first.row_cursors[0], second.row_cursors[0]],
        &["src:old", "src:new"],
        body,
    );
    assert!(result.results.iter().all(|hit| hit.id == IdSpace::logical("note:a")));
    let current = search(&opened.engine, "wombat", &ReadView::default());
    assert_records(&current, &second.row_cursors, &["src:new"], body);
    opened.engine.close().unwrap();
}

#[test]
fn equal_body_node_and_edge_keep_their_namespaces_and_text_positions() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open_with_choice(
        dir.path().join(format!("node_edge{SQLITE_SUFFIX}")),
        EmbedderChoice::None,
    )
    .unwrap();
    let body = "peculiar wombat telemetry";
    opened.engine.write(&[node(Some("missing"), "test:endpoint-seed", "endpoint")]).unwrap();
    let receipt = opened
        .engine
        .write(&[
            node(Some("shared"), "src:node", body),
            PreparedWrite::Edge {
                kind: "link".into(),
                from: "shared".into(),
                to: "missing".into(),
                body: Some(body.into()),
                source_id: SourceId::new("src:edge").unwrap(),
                logical_id: Some("shared".into()),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
                properties: None,
            },
        ])
        .unwrap();
    let result = search(&opened.engine, "wombat", &ReadView::default());
    assert_records(&result, &receipt.row_cursors, &["src:node", "src:edge"], body);
    assert!(result.results.iter().all(|hit| hit.id == IdSpace::logical("shared")));
    assert_eq!(result.results[0].branch, SoftFallbackBranch::Text);
    assert_eq!(result.results[1].branch, SoftFallbackBranch::TextEdge);
    opened.engine.close().unwrap();
}

#[test]
fn identical_stored_role_tagged_twins_keep_record_coordinates_and_public_hash() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open_with_choice(
        dir.path().join(format!("tagged{SQLITE_SUFFIX}")),
        EmbedderChoice::None,
    )
    .unwrap();
    let body = "\u{001e}\u{0011}café pear";
    assert_eq!(
        body.as_bytes(),
        &[0x1e, 0x11, b'c', b'a', b'f', 0xc3, 0xa9, b' ', b'p', b'e', b'a', b'r']
    );
    let receipt =
        opened.engine.write(&[node(None, "src:a", body), node(None, "src:b", body)]).unwrap();
    let result = search(&opened.engine, "cafe", &ReadView::default());
    assert_records(&result, &receipt.row_cursors, &["src:a", "src:b"], body);
    assert_eq!(result.results[0].id, result.results[1].id);
    assert_eq!(result.results[0].id.space, fathomdb_engine::IdSpaceKind::Content);
    let snippets = result.snippets.as_ref().unwrap();
    assert_eq!(snippets.len(), 2);
    for snippet in snippets {
        assert_eq!(snippet.text.as_bytes(), body.as_bytes());
        assert_eq!(snippet.anchor, SnippetAnchor::Whole);
        assert_eq!(snippet.windows, [(0, 12)]);
        assert_eq!(snippet.spans, [(2, 7)]);
        assert_eq!(snippet.highlights, [(2, 7)]);
        assert!(!snippet.cut);
    }
    opened.engine.close().unwrap();
}

#[derive(Debug)]
struct FixedEmbedder;

impl fathomdb_embedder_api::Embedder for FixedEmbedder {
    fn identity(&self) -> fathomdb_embedder_api::EmbedderIdentity {
        fathomdb_embedder_api::EmbedderIdentity::new("record-identity", "v1", 8)
    }

    fn embed(
        &self,
        _text: &str,
    ) -> Result<fathomdb_embedder_api::Vector, fathomdb_embedder_api::EmbedderError> {
        Ok(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    }
}

#[test]
fn graph_reached_record_is_not_excluded_by_an_unrelated_vector_body_twin() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open_with_embedder_for_test(
        dir.path().join(format!("graph_twin{SQLITE_SUFFIX}")),
        std::sync::Arc::new(FixedEmbedder),
    )
    .unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    let mut seed = node(Some("seed"), "src:seed", "needle seed");
    let mut target = node(Some("target"), "src:target", "shared payload");
    for write in [&mut seed, &mut target] {
        if let PreparedWrite::Node { kind, .. } = write {
            *kind = "entity".into();
        }
    }
    let receipt = opened
        .engine
        .write(&[
            node(Some("unrelated"), "src:unrelated", "shared payload"),
            seed,
            target,
            PreparedWrite::Edge {
                kind: "link".into(),
                from: "seed".into(),
                to: "target".into(),
                body: None,
                source_id: SourceId::new("src:edge").unwrap(),
                logical_id: Some("edge".into()),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
                properties: None,
            },
        ])
        .unwrap();
    opened.engine.drain(10_000).unwrap();
    let without = opened
        .engine
        .search_reranked_view_with_limit(
            "needle",
            None,
            0,
            false,
            0.3,
            0,
            true,
            &ReadView::default(),
            10,
        )
        .unwrap();
    assert_eq!(
        without.results.iter().map(|hit| hit.id.value.as_str()).collect::<Vec<_>>(),
        ["seed", "unrelated"]
    );
    let with = opened
        .engine
        .search_reranked_view_with_limit(
            "needle",
            None,
            0,
            true,
            0.3,
            0,
            true,
            &ReadView::default(),
            10,
        )
        .unwrap();
    assert_eq!(
        with.results.iter().map(|hit| hit.id.value.as_str()).collect::<Vec<_>>(),
        ["seed", "target", "unrelated"],
        "a different equal-body record cannot suppress the reached node"
    );
    let target = &with.results[1];
    assert_eq!(target.write_cursor, receipt.row_cursors[2]);
    assert_eq!(target.body, "shared payload");
    assert_eq!(target.branch, SoftFallbackBranch::GraphArm);
    assert_eq!(target.source_id.as_deref(), Some("src:edge"));
    assert_eq!(target.score.to_bits(), (1.0_f64 / 31.0).to_bits());
    assert_eq!(with.results[0].score.to_bits(), (1.0_f64 / 31.0).to_bits());
    assert_eq!(with.results[2].score.to_bits(), (1.0_f64 / 32.0).to_bits());
    let detail = &with.explanation.as_ref().unwrap().per_hit[1];
    assert_eq!(detail.id, receipt.row_cursors[2]);
    assert_eq!(detail.vector_rank, None);
    assert_eq!(detail.text_rank, None);
    assert_eq!(detail.graph_rank, Some(0));
    assert_eq!(detail.fused_score.to_bits(), target.score.to_bits());
    opened.engine.close().unwrap();
}

fn ownerless_fixture(
    name: &str,
    bodies: &[&str],
    edge_body: &str,
    confidence: Option<f64>,
) -> (TempDir, fathomdb_engine::OpenedEngine, u64) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("{name}{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    opened.engine.write(&[node(Some("missing"), "test:endpoint-seed", "endpoint")]).unwrap();
    let receipt = opened
        .engine
        .write(&[
            node(Some("keeper"), "src:keeper", "unmatched keeper"),
            PreparedWrite::Edge {
                kind: "link".into(),
                from: "keeper".into(),
                to: "missing".into(),
                body: Some(edge_body.into()),
                source_id: SourceId::new("src:edge").unwrap(),
                logical_id: Some("edge".into()),
                t_valid: None,
                t_invalid: None,
                confidence,
                extractor_model_id: None,
                temporal_fallback: None,
                properties: None,
            },
        ])
        .unwrap();
    let cursor = receipt.row_cursors[1];
    opened.engine.close().unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    for (index, body) in bodies.iter().enumerate() {
        connection
            .execute(
                "INSERT INTO search_index(rowid,body,kind,write_cursor) VALUES(?1,?2,'doc',?3)",
                rusqlite::params![10_001 + index as i64, body, cursor],
            )
            .unwrap();
    }
    connection.close().unwrap();
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    (dir, opened, cursor)
}

#[test]
fn ownerless_equal_bodies_use_real_fts_rowids_even_when_their_cursor_names_an_edge() {
    let (_dir, opened, cursor) =
        ownerless_fixture("ownerless_twins", &["cafe shared", "cafe shared"], "cafe shared", None);
    let result = opened
        .engine
        .search_reranked_view_with_limit(
            "cafe",
            None,
            0,
            false,
            0.3,
            0,
            false,
            &ReadView::default(),
            10,
        )
        .unwrap();
    assert_eq!(
        result.results.len(),
        3,
        "two explicit ownerless rowids and a canonical edge are three distinct records"
    );
    for (rank, hit) in result.results.iter().enumerate() {
        assert_eq!(hit.write_cursor, cursor);
        assert_eq!(hit.body, "cafe shared");
        assert_eq!(hit.score.to_bits(), (3.0_f64 / (31.0 + rank as f64)).to_bits());
    }
    assert_eq!(result.results[0].id, result.results[1].id);
    assert_eq!(result.results[0].id.space, fathomdb_engine::IdSpaceKind::Content);
    assert_eq!(result.results[0].source_id, None);
    assert_eq!(result.results[1].source_id, None);
    assert_eq!(result.results[2].id, IdSpace::logical("edge"));
    assert_eq!(result.results[2].source_id.as_deref(), Some("src:edge"));
    opened.engine.close().unwrap();
}

#[test]
fn snippets_use_each_ownerless_fts_locator_when_all_raw_cursors_collide() {
    let one = format!("{}café first", "pad ".repeat(30));
    let two = format!("{}café second", "pad ".repeat(60));
    let (_dir, opened, cursor) =
        ownerless_fixture("ownerless_snippets", &[&one, &two], "café edgefact", None);
    let mut options = SnippetOptions::default();
    options.max_bytes = 40;
    options.highlights = true;
    let result = opened
        .engine
        .search_reranked_view_with_snippets(
            "cafe",
            None,
            0,
            false,
            0.3,
            0,
            false,
            &ReadView::default(),
            10,
            Some(&options),
        )
        .unwrap();
    assert_eq!(result.results.len(), 3);
    let snippets = result.snippets.as_ref().unwrap();
    assert_eq!(snippets.len(), result.results.len());
    for (body, start) in [(&one, 120_u32), (&two, 240_u32)] {
        let index = result.results.iter().position(|hit| &hit.body == body).unwrap();
        let hit = &result.results[index];
        let snippet = &snippets[index];
        assert_eq!(hit.write_cursor, cursor);
        assert_eq!(hit.source_id, None);
        assert_eq!(snippet.anchor, SnippetAnchor::Match);
        assert_eq!(snippet.spans, [(start, start + 5)]);
        assert!(snippet.text.contains("café"));
        assert_eq!(snippet.highlights.len(), 1);
        let (begin, end) = snippet.highlights[0];
        assert_eq!(&snippet.text[begin as usize..end as usize], "café");
        for &(begin, end) in &snippet.windows {
            assert!(body.is_char_boundary(begin as usize) && body.is_char_boundary(end as usize));
            assert!(begin <= start && start + 5 <= end);
        }
    }
    let edge = result.results.iter().position(|hit| hit.id == IdSpace::logical("edge")).unwrap();
    assert_eq!(snippets[edge].anchor, SnippetAnchor::Whole);
    assert_eq!(snippets[edge].spans, [(0, 5)]);
    opened.engine.close().unwrap();
}

#[test]
fn ownerless_rows_do_not_borrow_a_colliding_edge_confidence() {
    let (_dir, opened, cursor) = ownerless_fixture(
        "ownerless_weight",
        &["cafe orphanone", "cafe orphantwo"],
        "cafe edgefact",
        Some(0.2),
    );
    opened.engine.set_importance_reweight_enabled_for_test(true);
    let result = opened
        .engine
        .search_reranked_view_with_limit(
            "cafe",
            None,
            0,
            false,
            0.3,
            0,
            false,
            &ReadView::default(),
            10,
        )
        .unwrap();
    assert_eq!(result.results.len(), 3);
    assert_eq!(result.results[0].body, "cafe orphanone");
    assert_eq!(result.results[1].body, "cafe orphantwo");
    assert_eq!(result.results[0].score.to_bits(), (3.0_f64 / 31.0).to_bits());
    assert_eq!(result.results[1].score.to_bits(), (3.0_f64 / 32.0).to_bits());
    assert_eq!(result.results[2].write_cursor, cursor);
    assert_eq!(result.results[2].id, IdSpace::logical("edge"));
    assert_eq!(result.results[2].score.to_bits(), ((3.0_f64 / 33.0) * 0.2).to_bits());
    opened.engine.close().unwrap();
}

#[test]
fn conflicting_fts_owner_bodies_and_stored_kinds_refuse_instead_of_returning_forged_hits() {
    for table in ["search_index", "search_index_edges"] {
        for (column, value) in [("body", "cafe forged"), ("kind", "forged-kind")] {
            let dir = TempDir::new().unwrap();
            let path = dir.path().join(format!("conflict{SQLITE_SUFFIX}"));
            let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
            let write = if table == "search_index" {
                node(Some("record"), "src:record", "cafe original")
            } else {
                opened
                    .engine
                    .write(&[
                        node(Some("missing-a"), "test:endpoint-seed", "endpoint"),
                        node(Some("missing-b"), "test:endpoint-seed", "endpoint"),
                    ])
                    .unwrap();
                PreparedWrite::Edge {
                    kind: "link".into(),
                    from: "missing-a".into(),
                    to: "missing-b".into(),
                    body: Some("cafe original".into()),
                    source_id: SourceId::new("src:record").unwrap(),
                    logical_id: Some("record".into()),
                    t_valid: None,
                    t_invalid: None,
                    confidence: None,
                    extractor_model_id: None,
                    temporal_fallback: None,
                    properties: None,
                }
            };
            let receipt = opened.engine.write(&[write]).unwrap();
            opened.engine.close().unwrap();
            let connection = rusqlite::Connection::open(&path).unwrap();
            connection
                .execute(
                    &format!("UPDATE {table} SET {column}=?1 WHERE write_cursor=?2"),
                    rusqlite::params![value, receipt.row_cursors[0]],
                )
                .unwrap();
            connection.close().unwrap();
            let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
            let result = opened.engine.search_reranked_view_with_limit(
                "cafe",
                None,
                0,
                false,
                0.3,
                0,
                false,
                &ReadView::default(),
                10,
            );
            assert!(matches!(result, Err(fathomdb_engine::EngineError::Storage)),
                "{table}.{column} conflict must follow the existing storage-error route: {result:?}");
            opened.engine.close().unwrap();
        }
    }
}

#[test]
fn ownerless_explanation_cannot_borrow_a_colliding_edges_lifecycle() {
    let (_dir, opened, _cursor) =
        ownerless_fixture("ownerless_lifecycle", &["cafe orphanone"], "cafe edgefact", None);
    let result = opened.engine.search_reranked_view_with_limit(
        "cafe",
        None,
        0,
        false,
        0.3,
        0,
        true,
        &ReadView::default(),
        10,
    );
    assert!(matches!(result, Err(fathomdb_engine::EngineError::Storage)),
        "missing canonical node lifecycle retains the existing error instead of borrowing EdgeValid: {result:?}");
    opened.engine.close().unwrap();
}

#[test]
fn same_record_vector_edge_keeps_its_representative_kind_and_source() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open_with_embedder_for_test(
        dir.path().join(format!("vector_edge{SQLITE_SUFFIX}")),
        std::sync::Arc::new(FixedEmbedder),
    )
    .unwrap();
    opened
        .engine
        .write(&[
            node(Some("missing-a"), "test:endpoint-seed", "endpoint"),
            node(Some("missing-b"), "test:endpoint-seed", "endpoint"),
        ])
        .unwrap();
    let receipt = opened
        .engine
        .write(&[PreparedWrite::Edge {
            kind: "link".into(),
            from: "missing-a".into(),
            to: "missing-b".into(),
            body: Some("cafe edgefact".into()),
            source_id: SourceId::new("src:edge").unwrap(),
            logical_id: Some("edge".into()),
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
            properties: None,
        }])
        .unwrap();
    opened.engine.drain(10_000).unwrap();
    let result = opened
        .engine
        .search_reranked_view_with_limit(
            "cafe",
            None,
            0,
            false,
            0.3,
            0,
            true,
            &ReadView::default(),
            10,
        )
        .unwrap();
    assert_eq!(result.results.len(), 1);
    let hit = &result.results[0];
    assert_eq!(hit.write_cursor, receipt.row_cursors[0]);
    assert_eq!(hit.id, IdSpace::logical("edge"));
    assert_eq!(hit.kind, "edge_fact");
    assert_eq!(hit.branch, SoftFallbackBranch::TextEdge);
    assert_eq!(hit.source_id.as_deref(), Some("src:edge"));
    assert_eq!(hit.score.to_bits(), (1.0_f64 / 31.0 + 3.0_f64 / 31.0).to_bits());
    let detail = &result.explanation.as_ref().unwrap().per_hit[0];
    assert_eq!(detail.vector_rank, Some(0));
    assert_eq!(detail.text_rank, Some(0));
    opened.engine.close().unwrap();
}

#[test]
fn equal_body_records_keep_own_importance_and_explanation_after_reordering() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open_with_choice(
        dir.path().join(format!("record_importance{SQLITE_SUFFIX}")),
        EmbedderChoice::None,
    )
    .unwrap();
    let receipt = opened
        .engine
        .write(&[
            node(Some("low"), "src:low", "cafe same"),
            node(Some("high"), "src:high", "cafe same"),
        ])
        .unwrap();
    opened.engine.write_node_importance(receipt.row_cursors[0], 0.1).unwrap();
    opened.engine.write_node_importance(receipt.row_cursors[1], 0.9).unwrap();
    opened.engine.set_importance_reweight_enabled_for_test(true);
    let result = opened
        .engine
        .search_reranked_view_with_limit(
            "cafe",
            None,
            0,
            false,
            0.3,
            0,
            true,
            &ReadView::default(),
            10,
        )
        .unwrap();
    assert_eq!(
        result.results.iter().map(|hit| hit.write_cursor).collect::<Vec<_>>(),
        [receipt.row_cursors[1], receipt.row_cursors[0]]
    );
    let details = &result.explanation.as_ref().unwrap().per_hit;
    assert_eq!(details.len(), result.results.len());
    for (index, rank, importance, source) in
        [(0, 1_u32, 0.9_f64, "src:high"), (1, 0, 0.1, "src:low")]
    {
        let hit = &result.results[index];
        let detail = &details[index];
        assert_eq!(hit.source_id.as_deref(), Some(source));
        assert_eq!(detail.id, hit.write_cursor);
        assert_eq!(detail.text_rank, Some(rank));
        assert_eq!(detail.importance, Some(importance));
        let expected = (3.0_f64 / (31.0 + f64::from(rank))) * importance;
        assert_eq!(hit.score.to_bits(), expected.to_bits());
        assert_eq!(detail.fused_score.to_bits(), expected.to_bits());
    }
    opened.engine.close().unwrap();
}

#[test]
fn equal_body_evidence_sidecars_resolve_the_exact_selected_record_source() {
    use fathomdb_engine::{
        ArtifactRevisionId, EvidenceResolveRequestV1, EvidenceSearchRequestV1, ProvenancedNodeV1,
        ReadContextV1, SearchFilter, SourceVersionId, WriteProvenanceV1,
    };
    let dir = TempDir::new().unwrap();
    let opened = Engine::open_with_choice(
        dir.path().join(format!("record_evidence{SQLITE_SUFFIX}")),
        EmbedderChoice::None,
    )
    .unwrap();
    let writes: Vec<_> = ["a", "b"]
        .iter()
        .map(|label| {
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "document".into(),
                body: "cafe same".into(),
                source_id: SourceId::new(format!("src:{label}")).unwrap(),
                logical_id: Some(format!("record:{label}")),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new(format!("revision:{label}")).unwrap(),
                    SourceVersionId::new(format!("version:{label}")).unwrap(),
                ),
                properties: None,
            })
        })
        .collect();
    let receipt = opened.engine.write(&writes).unwrap();
    opened.engine.drain(10_000).unwrap();
    let context = ReadContextV1::new(
        ReadView { valid_as_of: Some(1_800_000_000), ..ReadView::default() },
        SearchFilter::default(),
    )
    .unwrap();
    let frozen = opened.engine.freeze_read_context(&context).unwrap();
    let result = opened
        .engine
        .search_with_evidence(
            &EvidenceSearchRequestV1::new("cafe".into(), frozen.clone()).with_explain(true),
        )
        .unwrap();
    assert_eq!(
        result.search_result.results.iter().map(|hit| hit.write_cursor).collect::<Vec<_>>(),
        receipt.row_cursors
    );
    assert_eq!(result.evidence.len(), result.search_result.results.len());
    assert_eq!(
        result.search_result.explanation.as_ref().unwrap().per_hit.len(),
        result.evidence.len()
    );
    for (index, label) in ["a", "b"].iter().enumerate() {
        let entry = &result.evidence[index];
        assert_eq!(entry.result_index, index as u32);
        assert_eq!(entry.artifact_revision_id, format!("revision:{label}"));
        let resolved = opened
            .engine
            .resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: entry.evidence_ref.clone(),
                context: frozen.clone(),
            })
            .unwrap();
        assert_eq!(resolved.logical_id, Some(format!("record:{label}")));
        assert_eq!(resolved.artifact_revision_id, format!("revision:{label}"));
        assert_eq!(resolved.source_id, format!("src:{label}"));
        assert_eq!(resolved.source_revision_id, format!("revision:{label}"));
        assert_eq!(resolved.source_version_id, format!("version:{label}"));
        assert_eq!(resolved.canonical_source_body, "cafe same");
        assert_eq!(resolved.evidence_text, "cafe same");
    }
    opened.engine.close().unwrap();
}
