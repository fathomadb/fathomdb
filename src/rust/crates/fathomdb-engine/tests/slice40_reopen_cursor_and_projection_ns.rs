//! 0.8.32 Slice 40 (`WP-40A1`, `Gate1-D9`) — RED/GREEN regression suite for:
//! 1. Bug 1: Empty-DB reopen cursor monotonicity after `AdminSchema` / trailing
//!    `AdminSchema` / post-purge high-cursor writes, plus O(log N) boundary indexes.
//! 2. Bug 2 (`Option B2-strict`): Per-class `_fathomdb_projection_registry`
//!    `PRIMARY KEY(entity_class, name)`, strictly typed `ProjectionTarget`,
//!    isolated same-name `Node` + `Edge` projections, `ProjectionDelta`,
//!    `ProjectionRuntimeStatusEntry.entity_class`, and Schema 38 -> 39 upgrade.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use fathomdb_engine::{
    Engine, EngineError, InitialState, PreparedWrite, ProjectionFts, ProjectionRole,
    ProjectionSpec, ProjectionTarget, PropertyComparisonOp, PropertyEntityClass, PropertyMap,
    PropertyPredicate, PropertyValue, ReadView, SearchFilter, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use tempfile::TempDir;

fn db_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn admin_schema_write(name: &str, kind: &str) -> PreparedWrite {
    PreparedWrite::AdminSchema {
        name: name.to_string(),
        kind: kind.to_string(),
        schema_json: r#"{"type":"object"}"#.to_string(),
        retention_json: "{}".to_string(),
    }
}

fn node_write(logical_id: &str, body: &str, properties: Option<PropertyMap>) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "note".to_string(),
        body: body.to_string(),
        source_id: SourceId::new("slice40-test").expect("source id"),
        logical_id: Some(logical_id.to_string()),
        state: InitialState::default(),
        reason: None,
        valid_from: None,
        valid_until: None,
        properties,
    }
}

fn edge_write(
    logical_id: &str,
    from: &str,
    to: &str,
    properties: Option<PropertyMap>,
) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "relates_to".to_string(),
        from: from.to_string(),
        to: to.to_string(),
        source_id: SourceId::new("slice40-test").expect("source id"),
        logical_id: Some(logical_id.to_string()),
        body: None,
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        properties,
    }
}

fn props(entries: &[(&str, PropertyValue)]) -> PropertyMap {
    entries.iter().map(|(k, v)| ((*k).to_string(), v.clone())).collect::<BTreeMap<_, _>>()
}

// =========================================================================
// Bug 1: Empty-DB Reopen Cursor Monotonicity & Boundary Indexes
// =========================================================================

#[test]
fn bug1_empty_db_admin_configure_reopen_single_and_multi_row_writes_succeed() {
    let dir = TempDir::new().expect("tempdir");
    let path = db_path(&dir, "bug1_empty_admin");

    // 1. Open empty DB, register an operational collection (AdminSchema -> cursor 1), close.
    {
        let opened = Engine::open(&path).expect("open fresh");
        let receipt = opened
            .engine
            .write(&[admin_schema_write("checkpoints", "latest_state")])
            .expect("admin configure on empty db");
        assert_eq!(receipt.cursor, 1);
        opened.engine.close().expect("close");
    }

    // 2. Reopen and write a single node: must advance past cursor 1 (no PK collision on
    //    _fathomdb_commit_timestamps).
    {
        let opened = Engine::open(&path).expect("reopen 1");
        let receipt = opened
            .engine
            .write(&[node_write("n-1", "alpha first post-reopen node", None)])
            .expect("single-row write after reopen must succeed");
        assert!(receipt.cursor > 1, "expected cursor > 1 after reopen, got {}", receipt.cursor);
        opened.engine.drain(5_000).expect("drain");
        let hits = opened.engine.search_text_only("alpha").expect("search text after single write");
        assert_eq!(hits.results.len(), 1);
        opened.engine.close().expect("close");
    }

    // 3. Fresh DB: AdminSchema -> close -> reopen -> multi-row batch:
    //    row 0 must NOT reuse cursor 1 (which already has terminal='up_to_date') and must
    //    be projected and searchable!
    let multi_path = db_path(&dir, "bug1_empty_admin_multi");
    {
        let opened = Engine::open(&multi_path).expect("open fresh multi");
        let receipt = opened
            .engine
            .write(&[admin_schema_write("events", "append_only_log")])
            .expect("admin configure multi");
        assert_eq!(receipt.cursor, 1);
        opened.engine.close().expect("close");
    }
    {
        let opened = Engine::open(&multi_path).expect("reopen multi");
        let receipt = opened
            .engine
            .write(&[
                node_write("m-1", "zirconium unique first batch row", None),
                node_write("m-2", "titanium unique second batch row", None),
            ])
            .expect("multi-row write after reopen");
        assert!(receipt.cursor >= 3);
        opened.engine.drain(5_000).expect("drain");
        let first_hits = opened.engine.search_text_only("zirconium").expect("search first row");
        assert_eq!(
            first_hits.results.len(),
            1,
            "first row of post-reopen multi-row batch must be projected"
        );
        let second_hits = opened.engine.search_text_only("titanium").expect("search second row");
        assert_eq!(second_hits.results.len(), 1);
        opened.engine.close().expect("close");
    }
}

#[test]
fn bug1_non_empty_trailing_admin_configure_reopen_advances_cursor() {
    let dir = TempDir::new().expect("tempdir");
    let path = db_path(&dir, "bug1_trailing_admin");

    {
        let opened = Engine::open(&path).expect("open fresh");
        let r1 = opened
            .engine
            .write(&[node_write("base-1", "baseline node content", None)])
            .expect("write baseline node");
        assert_eq!(r1.cursor, 1);
        let r2 = opened
            .engine
            .write(&[admin_schema_write("trailing_coll", "latest_state")])
            .expect("trailing admin configure");
        assert_eq!(r2.cursor, 2);
        opened.engine.close().expect("close");
    }

    {
        let opened = Engine::open(&path).expect("reopen");
        let r3 = opened
            .engine
            .write(&[node_write("after-trailing", "cobalt after trailing admin", None)])
            .expect("write after trailing admin reopen");
        assert!(
            r3.cursor > 2,
            "expected cursor > 2 after trailing admin reopen, got {}",
            r3.cursor
        );
        opened.engine.drain(5_000).expect("drain");
        let hits = opened.engine.search_text_only("cobalt").expect("search after trailing admin");
        assert_eq!(hits.results.len(), 1);
        opened.engine.close().expect("close");
    }
}

#[test]
fn bug1_schema39_adds_boundary_indexes_on_mutations_and_closures() {
    let dir = TempDir::new().expect("tempdir");
    let path = db_path(&dir, "bug1_indexes");
    let opened = Engine::open(&path).expect("open fresh");
    opened.engine.close().expect("close");

    let conn = Connection::open(&path).expect("open sqlite");
    let has_mut_idx: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='index' AND name='operational_mutations_write_cursor_idx')",
            [],
            |r| r.get(0),
        )
        .expect("check operational_mutations_write_cursor_idx");
    let has_closure_idx: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='index' AND name='_fathomdb_dependency_closures_boundary_idx')",
            [],
            |r| r.get(0),
        )
        .expect("check _fathomdb_dependency_closures_boundary_idx");
    assert!(has_mut_idx, "expected operational_mutations_write_cursor_idx in Schema 39");
    assert!(has_closure_idx, "expected _fathomdb_dependency_closures_boundary_idx in Schema 39");
}

// =========================================================================
// Bug 2: Option B2-strict Per-Class Projection Namespace & ProjectionTarget
// =========================================================================

#[test]
fn bug2_same_name_node_and_edge_projections_coexist_and_drop_independently() {
    let dir = TempDir::new().expect("tempdir");
    let path = db_path(&dir, "bug2_same_name");
    let opened = Engine::open(&path).expect("open");
    let engine = &opened.engine;

    // Seed two nodes and one edge all sharing the `"status"` property name with distinct values.
    engine
        .write(&[
            node_write(
                "n-active",
                r#"{"status":"open","note":"node open status"}"#,
                Some(props(&[("status", PropertyValue::String("open".into()))])),
            ),
            node_write(
                "n-closed",
                r#"{"status":"closed","note":"node closed status"}"#,
                Some(props(&[("status", PropertyValue::String("closed".into()))])),
            ),
            edge_write(
                "e-1",
                "n-active",
                "n-closed",
                Some(props(&[("status", PropertyValue::String("verified".into()))])),
            ),
        ])
        .expect("seed nodes and edge");

    let node_spec = ProjectionSpec {
        name: "status".to_string(),
        roles: BTreeSet::from([ProjectionRole::Filterable, ProjectionRole::Searchable]),
        fts: Some(ProjectionFts { tokenizer: None }),
        vector: None,
        source: None,
        entity_class: PropertyEntityClass::Node,
    };
    let edge_spec = ProjectionSpec {
        name: "status".to_string(),
        roles: BTreeSet::from([ProjectionRole::Filterable]),
        fts: None,
        vector: None,
        source: None,
        entity_class: PropertyEntityClass::Edge,
    };

    // 1. Registering `"status"` on BOTH node and edge in ONE call succeeds!
    let delta = engine
        .configure_projections(&[node_spec.clone(), edge_spec.clone()], &[])
        .expect("register same-name node and edge projections in one call");
    assert!(!delta.unchanged);
    assert_eq!(
        delta.built,
        vec![ProjectionTarget::node("status"), ProjectionTarget::edge("status")]
    );
    assert!(delta.dropped.is_empty());

    // 2. Idempotent re-apply is a clean no-op.
    let noop = engine
        .configure_projections(&[node_spec.clone(), edge_spec.clone()], &[])
        .expect("idempotent re-apply");
    assert!(noop.unchanged);
    assert!(noop.built.is_empty());

    // 3. read_projections and read_projection_status carry distinct entity_class entries.
    let listed = engine.read_projections().expect("read_projections");
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].name, "status");
    assert_eq!(listed[0].entity_class, PropertyEntityClass::Node);
    assert_eq!(listed[1].name, "status");
    assert_eq!(listed[1].entity_class, PropertyEntityClass::Edge);

    let status = engine.read_projection_status().expect("read_projection_status");
    assert_eq!(status.projections.len(), 2);
    assert_eq!(status.projections[0].name, "status");
    assert_eq!(status.projections[0].entity_class, PropertyEntityClass::Node);
    assert_eq!(status.projections[1].name, "status");
    assert_eq!(status.projections[1].entity_class, PropertyEntityClass::Edge);

    // 4. SearchFilter (legacy attribute + typed predicate) and search_projected_text resolve (Node, "status").
    let mut filter_eq = SearchFilter::default();
    filter_eq.property_predicates = vec![PropertyPredicate {
        key: "status".to_string(),
        op: PropertyComparisonOp::Eq,
        value: PropertyValue::String("open".to_string()),
    }];
    let filtered = engine
        .search_text_only_filtered("node", Some(filter_eq.clone()), &ReadView::default(), 10)
        .expect("node property predicate search with same-name edge projection");
    assert_eq!(filtered.results.len(), 1);
    assert_eq!(filtered.results[0].id.value.as_str(), "n-active");

    let proj_text = engine
        .search_projected_text_with_limit("open", "status", None, &ReadView::default(), 10)
        .expect("search_projected_text resolves node status");
    assert_eq!(proj_text.results.len(), 1);
    assert_eq!(proj_text.results[0].id.value.as_str(), "n-active");

    // 5. Dropping ProjectionTarget::edge("status") removes ONLY edge CPV + registry row,
    //    leaving node `"status"` EAV / FTS / CPV / registry intact!
    let drop_delta = engine
        .configure_projections(&[], &[ProjectionTarget::edge("status")])
        .expect("drop only edge status projection");
    assert!(!drop_delta.unchanged);
    assert_eq!(drop_delta.dropped, vec![ProjectionTarget::edge("status")]);
    assert!(drop_delta.built.is_empty());

    let listed_after = engine.read_projections().expect("read_projections after edge drop");
    assert_eq!(listed_after.len(), 1);
    assert_eq!(listed_after[0].name, "status");
    assert_eq!(listed_after[0].entity_class, PropertyEntityClass::Node);

    // Verify node search_projected_text and SearchFilter still work after dropping edge `"status"`.
    let filtered_after = engine
        .search_text_only_filtered("node", Some(filter_eq), &ReadView::default(), 10)
        .expect("node filter still works after edge drop");
    assert_eq!(filtered_after.results.len(), 1);
    let proj_after = engine
        .search_projected_text_with_limit("open", "status", None, &ReadView::default(), 10)
        .expect("node projected text still works after edge drop");
    assert_eq!(proj_after.results.len(), 1);

    // Verify SQL rows directly: node CPV/EAV/FTS intact, edge CPV gone.
    opened.engine.close().expect("close");
    let conn = Connection::open(&path).expect("open sqlite");
    let node_cpv: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM canonical_property_values WHERE entity_class='node' AND attr_name='status'",
            [],
            |r| r.get(0),
        )
        .expect("node cpv count");
    let edge_cpv: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM canonical_property_values WHERE entity_class='edge' AND attr_name='status'",
            [],
            |r| r.get(0),
        )
        .expect("edge cpv count");
    let node_eav: i64 = conn
        .query_row("SELECT COUNT(*) FROM canonical_attributes WHERE attr_name='status'", [], |r| {
            r.get(0)
        })
        .expect("node eav count");
    assert_eq!(node_cpv, 2, "node CPV rows must survive edge drop");
    assert_eq!(node_eav, 2, "node EAV rows must survive edge drop");
    assert_eq!(edge_cpv, 0, "edge CPV rows must be deleted by edge drop");
}

#[test]
fn bug2_duplicate_same_class_rejected_but_sequential_cross_class_allowed() {
    let dir = TempDir::new().expect("tempdir");
    let path = db_path(&dir, "bug2_dedupe_and_seq");
    let opened = Engine::open(&path).expect("open");
    let engine = &opened.engine;

    let node_spec = ProjectionSpec {
        name: "priority".to_string(),
        roles: BTreeSet::from([ProjectionRole::Filterable]),
        fts: None,
        vector: None,
        source: None,
        entity_class: PropertyEntityClass::Node,
    };
    let edge_spec = ProjectionSpec {
        name: "priority".to_string(),
        roles: BTreeSet::from([ProjectionRole::Filterable]),
        fts: None,
        vector: None,
        source: None,
        entity_class: PropertyEntityClass::Edge,
    };

    // Duplicate same-class spec in one call is rejected.
    let err = engine
        .configure_projections(&[node_spec.clone(), node_spec.clone()], &[])
        .expect_err("duplicate (Node, priority) must fail");
    assert!(matches!(err, EngineError::InvalidArgument { .. }));

    // Duplicate same-class drop in one call is rejected.
    let drop_err = engine
        .configure_projections(
            &[],
            &[ProjectionTarget::node("priority"), ProjectionTarget::node("priority")],
        )
        .expect_err("duplicate drop target must fail");
    assert!(matches!(drop_err, EngineError::InvalidArgument { .. }));

    // Sequential registration: Node first, then Edge with the same name.
    let d1 = engine.configure_projections(&[node_spec], &[]).expect("node priority");
    assert_eq!(d1.built, vec![ProjectionTarget::node("priority")]);
    let d2 = engine.configure_projections(&[edge_spec], &[]).expect("edge priority");
    assert_eq!(d2.built, vec![ProjectionTarget::edge("priority")]);

    // Dropping both classes in one call succeeds cleanly.
    let d3 = engine
        .configure_projections(
            &[],
            &[ProjectionTarget::node("priority"), ProjectionTarget::edge("priority")],
        )
        .expect("drop both classes");
    assert_eq!(
        d3.dropped,
        vec![ProjectionTarget::node("priority"), ProjectionTarget::edge("priority")]
    );
    assert!(engine.read_projections().expect("read").is_empty());

    // Dropping an absent target is an idempotent no-op.
    let noop = engine
        .configure_projections(&[], &[ProjectionTarget::edge("priority")])
        .expect("drop absent edge target");
    assert!(noop.unchanged);

    opened.engine.close().expect("close");
}

#[test]
fn bug2_golden_schema38_migrates_projection_registry_to_composite_pk() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../dev/fixtures/golden-0.8.31-schema38.sqlite");
    assert!(fixture.is_file(), "missing golden fixture at {}", fixture.display());

    let dir = TempDir::new().expect("tempdir");
    let copy = db_path(&dir, "golden_s38_upgrade");
    std::fs::copy(&fixture, &copy).expect("copy golden fixture");

    // Seed a Schema-38 projection row before upgrading to Schema 39.
    {
        let conn = Connection::open(&copy).expect("open pre-migration sqlite");
        let version: u32 =
            conn.query_row("PRAGMA user_version", [], |r| r.get(0)).expect("user_version");
        assert_eq!(version, 38);
        conn.execute(
            "INSERT INTO _fathomdb_projection_registry(name, roles, entity_class) VALUES('legacy_tag', 'filterable', 'node')",
            [],
        )
        .expect("seed schema38 registry row");
    }

    // Migrate to Schema 39 directly via fathomdb_schema::migrate.
    {
        let conn = Connection::open(&copy).expect("open for migrate");
        let report = fathomdb_schema::migrate(&conn).expect("migrate 38 -> 39");
        assert_eq!(report.schema_version_after, 39);

        // Verify composite PRIMARY KEY(entity_class, name) via PRAGMA table_info.
        let mut stmt = conn.prepare("PRAGMA table_info(_fathomdb_projection_registry)").unwrap();
        let mut pk_cols: Vec<(i64, String)> = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(5)?, r.get::<_, String>(1)?)))
            .unwrap()
            .filter_map(|row| row.ok())
            .filter(|(pk, _)| *pk > 0)
            .collect();
        pk_cols.sort_by_key(|(pk, _)| *pk);
        assert_eq!(
            pk_cols,
            vec![(1, "entity_class".to_string()), (2, "name".to_string())],
            "Schema 39 _fathomdb_projection_registry must have PRIMARY KEY(entity_class, name)"
        );

        // Verify all 3 visibility triggers (pr_ai, pr_au, pr_ad) were restored.
        let trigger_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND tbl_name='_fathomdb_projection_registry'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(trigger_count, 3, "expected pr_ai/pr_au/pr_ad triggers");

        // Verify the migrated Schema-38 row survived and same-name edge row can coexist,
        // while duplicate (entity_class, name) is rejected by PRIMARY KEY(entity_class, name).
        let surviving: (String, String, String) = conn
            .query_row(
                "SELECT name, roles, entity_class FROM _fathomdb_projection_registry WHERE name='legacy_tag'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .expect("migrated legacy_tag row must survive");
        assert_eq!(
            surviving,
            ("legacy_tag".to_string(), "filterable".to_string(), "node".to_string())
        );

        conn.execute(
            "INSERT INTO _fathomdb_projection_registry(name, roles, entity_class) VALUES('legacy_tag', 'filterable', 'edge')",
            [],
        )
        .expect("same-name edge projection row must coexist in Schema 39");

        let dup_err = conn.execute(
            "INSERT INTO _fathomdb_projection_registry(name, roles, entity_class) VALUES('legacy_tag', 'filterable', 'node')",
            [],
        );
        assert!(dup_err.is_err(), "duplicate (node, legacy_tag) must violate composite PK");
    }
}
