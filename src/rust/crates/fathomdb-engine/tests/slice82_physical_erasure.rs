use std::{collections::BTreeSet, path::Path};

use fathomdb_engine::{
    recover_complete_erasures, Engine, EngineError, InitialState, LifecycleState, PreparedWrite,
    ProjectionFts, ProjectionRole, ProjectionSpec, SourceId,
};
use rusqlite::Connection;
use tempfile::TempDir;

const MARKER: &str = "qzxvplmnbvcxzasdfghjklqwertx";
const TABLES: [&str; 4] =
    ["search_index", "search_index_v2", "search_index_edges", "property_search_index"];

fn node(id: &str, source: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn occurrences(path: &Path, marker: &str) -> usize {
    [path.to_path_buf(), std::path::PathBuf::from(format!("{}-wal", path.display()))]
        .iter()
        .filter_map(|p| std::fs::read(p).ok())
        .map(|bytes| bytes.windows(marker.len()).filter(|w| *w == marker.as_bytes()).count())
        .sum()
}

fn fixture() -> (TempDir, Engine) {
    fixture_with_targets(true)
}

fn fixture_with_targets(include_targets: bool) -> (TempDir, Engine) {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path().join("physical.sqlite")).unwrap().engine;
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "text".into(),
                roles: BTreeSet::from([ProjectionRole::Searchable]),
                fts: Some(ProjectionFts { tokenizer: None }),
                vector: None,
                source: None,
            }],
            &[],
        )
        .unwrap();
    engine
        .write(&[node("left", "endpoints", "left"), node("right", "endpoints", "right")])
        .unwrap();
    for revision in 0..3 {
        let mut writes = Vec::new();
        for owner in 0..8 {
            if !include_targets && owner % 2 == 0 {
                continue;
            }
            let (source, word) =
                if owner % 2 == 0 { ("target", MARKER) } else { ("survivor", "survivorneedle") };
            let content = format!(
                "{word} common owner{owner} revision{revision} {}",
                "alpha beta gamma ".repeat(40)
            );
            writes.push(node(
                &format!("owner{owner}"),
                source,
                &serde_json::json!({"text":content}).to_string(),
            ));
            writes.push(PreparedWrite::Edge {
                kind: "rel".into(),
                from: "left".into(),
                to: "right".into(),
                source_id: SourceId::new(source).unwrap(),
                logical_id: Some(format!("edge{owner}")),
                body: Some(content),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            });
        }
        engine.write(&writes).unwrap();
    }
    (dir, engine)
}

fn snapshot(c: &Connection) -> Vec<(String, Vec<(i64, String)>)> {
    TABLES.iter().map(|table| {
        let col = if *table == "property_search_index" { "attr_value" } else { "body" };
        let mut stmt = c.prepare(&format!("SELECT rowid,{col} FROM {table} WHERE {table} MATCH 'survivorneedle' ORDER BY rowid")).unwrap();
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
        (table.to_string(), rows)
    }).collect()
}

#[test]
fn complete_erasure_removes_positive_terms_from_every_family_and_preserves_history() {
    let (dir, engine) = fixture();
    let path = dir.path().join("physical.sqlite");
    let c = Connection::open(&path).unwrap();
    for table in TABLES {
        let matches: i64 = c
            .query_row(
                &format!("SELECT count(*) FROM {table} WHERE {table} MATCH ?1"),
                [MARKER],
                |r| r.get(0),
            )
            .unwrap();
        assert!(matches > 0, "positive population of {table}");
    }
    let survivors = snapshot(&c);
    assert!(occurrences(&path, MARKER) > 0);
    let report = engine.erase_source("target").unwrap();
    assert_eq!((report.nodes_excised, report.edges_excised), (12, 12));
    assert_eq!(snapshot(&c), survivors, "row IDs and authorized history survive in place");
    for table in TABLES {
        let matches: i64 = c
            .query_row(
                &format!("SELECT count(*) FROM {table} WHERE {table} MATCH ?1"),
                [MARKER],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(matches, 0);
        for suffix in ["data", "content"] {
            let mut stmt = c.prepare(&format!("SELECT * FROM {table}_{suffix}")).unwrap();
            let columns = stmt.column_count();
            let mut rows = stmt.query([]).unwrap();
            while let Some(row) = rows.next().unwrap() {
                for column in 0..columns {
                    let value = row.get_ref(column).unwrap();
                    if let Ok(bytes) = value.as_bytes() {
                        assert!(
                            !bytes.windows(MARKER.len()).any(|w| w == MARKER.as_bytes()),
                            "retained term in {table}_{suffix}"
                        );
                    }
                }
            }
        }
    }
    assert_eq!(occurrences(&path, MARKER), 0, "success includes main file and live WAL");
    engine.close().unwrap();
}

#[test]
fn root_only_busy_erasure_fences_mutation_then_exact_retry_finishes() {
    for purge in [false, true] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("root.sqlite");
        let engine = Engine::open(&path).unwrap().engine;
        engine.write(&[node("subject", "target", MARKER)]).unwrap();
        if purge {
            engine.transition("subject", LifecycleState::Deleted, None).unwrap();
        }
        let pin = Connection::open(&path).unwrap();
        pin.execute_batch("BEGIN; SELECT * FROM canonical_nodes;").unwrap();
        let error =
            if purge { engine.purge("subject") } else { engine.erase_source("target").map(|_| ()) }
                .unwrap_err();
        assert!(
            matches!(error, EngineError::ErasureIncomplete { ref stage, .. } if stage == "wal_checkpoint"),
            "{error:?}"
        );
        assert!(
            matches!(
                engine.write(&[node("other", "other", "unrelated")]),
                Err(EngineError::ErasureIncomplete { .. })
            ),
            "root-only physical debt must fence writers"
        );
        assert!(!engine.pending_erasures().unwrap().is_empty());
        pin.execute_batch("ROLLBACK").unwrap();
        if purge {
            engine.purge("subject").unwrap();
        } else {
            engine.erase_source("target").unwrap();
        }
        engine.write(&[node("other", "other", "unrelated")]).unwrap();
        engine.close().unwrap();
    }
}

#[test]
fn legacy_deleted_terms_are_owed_to_offline_completion_without_closure_ids() {
    let (dir, engine) = fixture();
    let path = dir.path().join("physical.sqlite");
    // Reproduce admitted format-35 old deletion policy, bypassing repaired verbs.
    engine.maintenance_transaction_for_test(|tx| {
        for table in TABLES {
            tx.execute(&format!("DELETE FROM {table} WHERE write_cursor IN (SELECT write_cursor FROM canonical_nodes WHERE source_id='target' UNION ALL SELECT write_cursor FROM canonical_edges WHERE source_id='target')"), [])?;
        }
        tx.execute("DELETE FROM canonical_attributes WHERE write_cursor IN (SELECT write_cursor FROM canonical_nodes WHERE source_id='target')", [])?;
        tx.execute("DELETE FROM canonical_nodes WHERE source_id='target'", [])?;
        tx.execute("DELETE FROM canonical_edges WHERE source_id='target'", [])?;
        tx.execute("DELETE FROM _fathomdb_open_state WHERE key='_fathomdb_fts_sanitation'", [])?;
        Ok(())
    }).unwrap();
    engine.close().unwrap();
    assert!(occurrences(&path, MARKER) > 0, "positive legacy ghost");
    let report = recover_complete_erasures(&path).unwrap();
    assert_eq!(report.status, fathomdb_engine::TruncateWalStatus::Done);
    assert_eq!(
        occurrences(&path, MARKER),
        0,
        "offline path owes legacy sanitation even with no closures"
    );
}

#[test]
fn survivor_only_database_agrees_on_fts_order_and_score_bits() {
    let (dir, engine) = fixture();
    let (expected_dir, expected) = fixture_with_targets(false);
    engine.erase_source("target").unwrap();
    let actual_c = Connection::open(dir.path().join("physical.sqlite")).unwrap();
    let expected_c = Connection::open(expected_dir.path().join("physical.sqlite")).unwrap();
    for table in TABLES {
        let column = if table == "property_search_index" { "attr_value" } else { "body" };
        let ranked = |c: &Connection| {
            let mut statement = c.prepare(&format!("SELECT {column},bm25({table}) FROM {table} WHERE {table} MATCH 'survivorneedle' ORDER BY bm25({table}),{column}")).unwrap();
            statement
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?.to_bits())))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
        };
        assert_eq!(
            ranked(&actual_c),
            ranked(&expected_c),
            "survivor-only ranking oracle for {table}"
        );
    }
    engine.close().unwrap();
    expected.close().unwrap();
}

#[test]
fn optimized_unicode_zero_token_and_large_inputs_preserve_sanitation_validity() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("stress.sqlite");
    let engine = Engine::open(&path).unwrap().engine;
    let large = format!("{MARKER} {}", "common ".repeat(150_000));
    engine
        .write(&[
            node("large", "target", &large),
            node("unicode", "target", "日本語 café naïve Straße"),
            node("punctuation", "target", "!!! … ---"),
            node("survivor", "other", "survivorneedle"),
        ])
        .unwrap();
    for table in TABLES {
        engine
            .execute_for_test(&format!("INSERT INTO {table}({table}) VALUES('optimize')"))
            .unwrap();
    }
    assert!(occurrences(&path, MARKER) > 0);
    engine.erase_source("target").unwrap();
    assert_eq!(occurrences(&path, MARKER), 0);
    assert!(engine.pending_erasures().unwrap().is_empty());
    engine.write(&[node("survivor", "other", "updated survivor")]).unwrap();
    assert!(
        engine.pending_erasures().unwrap().iter().any(|r| r.cause == "retained_terms"),
        "ordinary FTS mutation invalidates certificate"
    );
    engine.rebuild_projections().unwrap();
    assert!(engine.pending_erasures().unwrap().iter().any(|r| r.cause == "retained_terms"));
    engine.erase_source("target").unwrap();
    assert!(engine.pending_erasures().unwrap().is_empty());
    engine.close().unwrap();
}
