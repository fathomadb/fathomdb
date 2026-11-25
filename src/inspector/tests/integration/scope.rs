use crate::{
    admission::preflight,
    fixtures::{add_scoped, database, node},
    policy::Policy,
    session::Session,
    storage::{
        graph::Direction,
        lexical::{LexicalFilter, LexicalView, Query, Route},
        Class, Filter, Reader,
    },
};
use fathomdb_engine::{
    configure_runtime, EmbedderChoice, Engine, EngineConfig, PhysicalErasurePolicy,
    PhysicalErasureSelector, QueueResult, RuntimeSqliteMode, ScopePolicy,
};
use serde_json::Value;
use std::fs;
use std::sync::Arc;

#[test]
fn table_manifest_drives_schema_39_admission_for_every_manifest_table() {
    assert_eq!(fathomdb_schema::SCHEMA_VERSION, 39);
    assert_eq!(fathomdb_schema::TABLE_MANIFEST.len(), 35);
    let dir = tempfile::tempdir().unwrap();
    let base = database(dir.path());
    assert!(preflight(&base).is_ok());

    // Every single table declared in TABLE_MANIFEST must be required by admission.
    // Drop a representative set including regular, internal, FTS5, and vec0 tables.
    for table in [
        "canonical_nodes",
        "canonical_edges",
        "operational_collections",
        "_fathomdb_actuation_receipt_source_refs",
        "_fathomdb_embed_probe",
        "_fathomdb_migrations",
        "search_index_v2",
        "property_search_index",
        "vector_default",
    ] {
        assert!(
            fathomdb_schema::TABLE_MANIFEST.iter().any(|entry| entry.name == table),
            "test table must be present in TABLE_MANIFEST: {table}"
        );
        let target = dir.path().join(format!("drop-{table}.sqlite"));
        fs::copy(&base, &target).unwrap();
        fs::write(format!("{}.lock", target.display()), b"lock").unwrap();
        let c = rusqlite::Connection::open(&target).unwrap();
        // Drop triggers/tables cleanly where FK exists.
        if table == "_fathomdb_actuation_receipts" {
            c.execute_batch("DROP TABLE _fathomdb_actuation_receipt_source_refs;").unwrap();
        }
        c.execute_batch(&format!("DROP TABLE {table};")).unwrap();
        drop(c);
        assert_eq!(
            preflight(&target).unwrap_err(),
            "malformed_layout",
            "dropping TABLE_MANIFEST table {table} must fail admission closed"
        );
    }
}

#[test]
fn catalog_discovers_scopes_and_enforces_d01_nondisclosure_for_inactive_and_erased_scopes() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    add_scoped(&path);

    // 1. Default policy (inactive visible): "", "alpha", "beta", "hidden-inactive" all appear.
    let mut default_reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let full = default_reader.catalog().unwrap();
    let scope_names = |cat: &Value| -> Vec<String> {
        cat["scopes"]
            .as_array()
            .expect("catalog must return scopes array")
            .iter()
            .map(|entry| entry["scope"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(scope_names(&full), vec!["", "alpha", "beta", "hidden-inactive"]);

    // Scoped catalog request filters class counts to "alpha" while keeping full visible scopes list.
    let alpha_cat = default_reader.catalog_scoped(Some("alpha")).unwrap();
    assert_eq!(scope_names(&alpha_cat), vec!["", "alpha", "beta", "hidden-inactive"]);
    assert_eq!(alpha_cat["counts"]["node"]["revisions"], "4");
    assert_eq!(alpha_cat["counts"]["edge"]["revisions"], "1");
    assert_eq!(alpha_cat["counts"]["current"]["values"], "1");
    assert_eq!(alpha_cat["counts"]["log"]["entries"], "1");
    assert_eq!(alpha_cat["scope"]["active_scope"], "alpha");
    default_reader.session.close().unwrap();

    // 2. --no-show-inactive (D-01): "hidden-inactive" MUST NOT appear in catalog().scopes!
    let hidden_policy = Policy { no_show_inactive: true, ..Policy::default() };
    let mut hidden_reader = Reader::new(Session::attach(&path, hidden_policy).unwrap());
    let cat = hidden_reader.catalog().unwrap();
    assert_eq!(
        scope_names(&cat),
        vec!["", "alpha", "beta"],
        "D-01: inactive-only scope must not leak into catalog().scopes under --no-show-inactive"
    );
    assert!(
        !cat.to_string().contains("hidden-inactive"),
        "D-01: hidden-inactive scope name must not appear anywhere in catalog payload"
    );
    let probed = hidden_reader.catalog_scoped(Some("hidden-inactive")).unwrap();
    assert_eq!(probed["counts"]["node"]["revisions"], "0");
    assert_eq!(probed["counts"]["edge"]["revisions"], "0");
    assert_eq!(scope_names(&probed), vec!["", "alpha", "beta"]);
    hidden_reader.session.close().unwrap();

    // 3. Erased-only scope ("hidden-erased"): MUST NOT appear in catalog().scopes when !show_erased!
    configure_runtime(RuntimeSqliteMode::Diagnostics).unwrap();
    let erased_path = dir.path().join("erased-scope.sqlite");
    {
        let opened = Engine::open_with_choice_and_config(
            &erased_path,
            EmbedderChoice::None,
            EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..EngineConfig::default()
            },
        )
        .unwrap();
        opened.engine.write(&[node("public-node", "visible body")]).unwrap();
        opened.engine.close().unwrap();
    }
    {
        let engine = Arc::new(
            Engine::open_with_choice_and_config(
                &erased_path,
                EmbedderChoice::None,
                EngineConfig {
                    scope_policy: ScopePolicy::single("hidden-erased"),
                    physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                    ..EngineConfig::default()
                },
            )
            .unwrap()
            .engine,
        );
        engine.write(&[node("erased-only-node", "secret erased body")]).unwrap();
        let mut maintenance = engine.begin_erasure_maintenance(1000).unwrap();
        assert!(matches!(
            maintenance
                .queue_physical_erasure(
                    PhysicalErasureSelector::SourceAllAtCut { source_id: "fixture:source".into() },
                    "0123456789abcdef0123456789abcdef",
                )
                .unwrap(),
            QueueResult::Accepted { .. }
        ));
        maintenance.close().unwrap();
        drop(maintenance);
        engine.close().unwrap();
    }
    let mut default_erased = Reader::new(Session::attach(&erased_path, Policy::default()).unwrap());
    let cat_hidden = default_erased.catalog().unwrap();
    assert_eq!(
        scope_names(&cat_hidden),
        vec![""],
        "D-01: erased-only scope must not appear in catalog().scopes when !show_erased"
    );
    default_erased.session.close().unwrap();

    let mut show_erased = Reader::new(
        Session::attach(&erased_path, Policy { show_erased: true, ..Policy::default() }).unwrap(),
    );
    let cat_shown = show_erased.catalog().unwrap();
    assert_eq!(
        scope_names(&cat_shown),
        vec!["", "hidden-erased"],
        "erased-only scope becomes visible when --show-erased is enabled"
    );
    show_erased.session.close().unwrap();
}

#[test]
fn scoped_page_uses_schema_39_indexes_without_scan_or_temp_btree() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    add_scoped(&path);

    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let c = reader.session.connection().unwrap();

    let assert_indexed_plan = |label: &str, class: Class, filter: Filter| {
        let (sql, params) = crate::storage::browse_query_for_plan(class, 0, &filter);
        let mut stmt = c.prepare(&format!("EXPLAIN QUERY PLAN {sql}")).unwrap();
        let details: Vec<String> = stmt
            .query_map(rusqlite::params_from_iter(params.iter()), |r| r.get::<_, String>(3))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        let joined = details.join(" | ");
        assert!(
            joined.contains("SEARCH"),
            "{label}: expected SEARCH in EXPLAIN QUERY PLAN, got: {joined}"
        );
        assert!(
            !joined.contains("SCAN"),
            "{label}: unexpected table SCAN in EXPLAIN QUERY PLAN: {joined}"
        );
        assert!(
            !joined.contains("TEMP B-TREE"),
            "{label}: unexpected TEMP B-TREE in EXPLAIN QUERY PLAN: {joined}"
        );
    };

    assert_indexed_plan(
        "canonical_nodes scoped",
        Class::Node,
        Filter { scope: Some("alpha".into()), ..Filter::default() },
    );
    assert_indexed_plan(
        "canonical_edges scoped",
        Class::Edge,
        Filter { scope: Some("alpha".into()), ..Filter::default() },
    );
    assert_indexed_plan(
        "operational_state scoped+collection",
        Class::Current,
        Filter {
            scope: Some("alpha".into()),
            collection: Some("scoped-state".into()),
            ..Filter::default()
        },
    );
    assert_indexed_plan(
        "operational_state scoped without collection",
        Class::Current,
        Filter { scope: Some("alpha".into()), ..Filter::default() },
    );
    assert_indexed_plan(
        "operational_mutations scoped+collection",
        Class::Log,
        Filter {
            scope: Some("alpha".into()),
            collection: Some("scoped-log".into()),
            ..Filter::default()
        },
    );
    assert_indexed_plan(
        "operational_mutations scoped without collection",
        Class::Log,
        Filter { scope: Some("alpha".into()), ..Filter::default() },
    );
    drop(c);

    // Functional verification: colliding logical_id and record_key stay strictly isolated per scope.
    for (scope_name, expected_body_needle, expected_op_owner) in [
        ("", "Default Shared Title", r#""owner":"default""#),
        ("alpha", "Alpha Shared Title", r#""owner":"alpha""#),
        ("beta", "Beta Shared Title", r#""owner":"beta""#),
    ] {
        let nodes = reader
            .page(
                Class::Node,
                0,
                50,
                Filter {
                    scope: Some(scope_name.into()),
                    identity: Some("shared-node".into()),
                    ..Filter::default()
                },
            )
            .unwrap();
        let items = nodes["items"].as_array().unwrap();
        assert_eq!(items.len(), 1, "scope {scope_name:?} must return exact 1 shared-node");
        assert_eq!(items[0]["scope"], scope_name);
        let detail = reader.detail(items[0]["handle"].as_str().unwrap()).unwrap();
        assert!(detail["content"]["text"].as_str().unwrap().contains(expected_body_needle));

        for (class, col) in [(Class::Current, "scoped-state"), (Class::Log, "scoped-log")] {
            let with_col = reader
                .page(
                    class,
                    0,
                    50,
                    Filter {
                        scope: Some(scope_name.into()),
                        collection: Some(col.into()),
                        identity: Some("shared-key".into()),
                        ..Filter::default()
                    },
                )
                .unwrap();
            assert_eq!(with_col["items"].as_array().unwrap().len(), 1);
            assert_eq!(with_col["items"][0]["scope"], scope_name);
            let op_detail =
                reader.detail(with_col["items"][0]["handle"].as_str().unwrap()).unwrap();
            assert!(op_detail["content"]["text"].as_str().unwrap().contains(expected_op_owner));

            let without_col = reader
                .page(
                    class,
                    0,
                    50,
                    Filter {
                        scope: Some(scope_name.into()),
                        identity: Some("shared-key".into()),
                        ..Filter::default()
                    },
                )
                .unwrap();
            assert_eq!(without_col["items"].as_array().unwrap().len(), 1);
            assert_eq!(without_col["items"][0]["scope"], scope_name);
        }
    }
    reader.session.close().unwrap();
}

#[test]
fn multi_scope_associations_investigate_graph_lexical_serving_and_vector_have_zero_cross_scope_leakage(
) {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    add_scoped(&path);

    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());

    // Fetch colliding handles for "alpha" and "beta".
    let get_handle = |reader: &mut Reader, scope_name: &str, logical_id: &str| -> String {
        let page = reader
            .page(
                Class::Node,
                0,
                50,
                Filter {
                    scope: Some(scope_name.into()),
                    identity: Some(logical_id.into()),
                    ..Filter::default()
                },
            )
            .unwrap();
        let items = page["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        items[0]["handle"].as_str().unwrap().to_string()
    };

    let alpha_derived = get_handle(&mut reader, "alpha", "shared-derived");
    let beta_derived = get_handle(&mut reader, "beta", "shared-derived");
    let alpha_node = get_handle(&mut reader, "alpha", "shared-node");
    let beta_node = get_handle(&mut reader, "beta", "shared-node");

    // 1. Associations: colliding revision_id ("shared-src-r1" / "shared-der-r1") must stay within scope!
    for (handle, expected_scope) in [(&alpha_derived, "alpha"), (&beta_derived, "beta")] {
        let assoc = reader.associations(handle).unwrap();
        let source_link = assoc["links"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["kind"] == "recorded_source")
            .unwrap();
        let targets = source_link["targets"].as_array().unwrap();
        assert_eq!(targets.len(), 1, "scope {expected_scope} must resolve to exactly 1 source");
        assert_eq!(targets[0]["scope"], expected_scope);
        assert_eq!(targets[0]["logical_id"], "shared-source");
    }

    // 2. Investigate: colliding logical_id and revision_id must stay within scope!
    for (handle, expected_scope) in [(&alpha_derived, "alpha"), (&beta_derived, "beta")] {
        let inv = reader.investigate(handle).unwrap();
        let revisions = inv["revisions"].as_array().unwrap();
        assert_eq!(revisions.len(), 1);
        assert_eq!(revisions[0]["scope"], expected_scope);
        let links = inv["links"].as_array().unwrap();
        assert!(!links.is_empty());
        for link in links {
            if let Some(target) = link["target"].as_object() {
                assert_eq!(
                    target["scope"], expected_scope,
                    "investigate link target leaked across scopes"
                );
            }
        }
    }

    // 3. Stored graph: colliding endpoints and edges must stay within scope, and cross-scope expansion rejected!
    for (handle, expected_scope) in [(&alpha_node, "alpha"), (&beta_node, "beta")] {
        let graph =
            reader.graph(handle, Direction::Outgoing, None, 2, vec![handle.clone()]).unwrap();
        let nodes = graph["nodes"].as_array().unwrap();
        let edges = graph["edges"].as_array().unwrap();
        assert_eq!(nodes.len(), 2, "scope {expected_scope} should see shared-node + shared-peer");
        assert_eq!(edges.len(), 1, "scope {expected_scope} should see only its own shared-edge");
        assert!(nodes.iter().all(|n| n["scope"] == expected_scope));
        assert!(edges.iter().all(|e| e["scope"] == expected_scope));
    }
    assert_eq!(
        reader.graph(&alpha_node, Direction::Both, None, 1, vec![beta_node.clone()]),
        Err("unsupported_constraint"),
        "mixing expansion anchor from another scope must be rejected"
    );

    // 4. Native graph_serving: Session switches engine scope "" -> "alpha" -> "beta" cleanly!
    for (handle, expected_scope, expected_needle) in
        [(&alpha_node, "alpha", "Alpha Peer"), (&beta_node, "beta", "Beta Peer")]
    {
        let serving_graph = reader.graph_serving(handle, Direction::Outgoing, None, 1).unwrap();
        assert_eq!(serving_graph["query_context"]["engine_scope"], expected_scope);
        let items = serving_graph["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["target"]["scope"], expected_scope);
        assert_eq!(items[0]["terminal_edge"]["scope"], expected_scope);
        assert!(items[0]["payload"]["text"].as_str().unwrap().contains(expected_needle));
    }

    // 5. Lexical (Body, Property, BM25F): explicit scope switches engine scope; None uses Unscoped ("")!
    for route in [Route::Body, Route::Property, Route::Bm25f] {
        let property = if route == Route::Property { Some("summary".to_string()) } else { None };
        let alpha_lex = reader
            .lexical(Query {
                route,
                query: "orchid".into(),
                property: property.clone(),
                filter: LexicalFilter::default(),
                view: LexicalView::default(),
                scope: Some("alpha".into()),
                limit: 20,
            })
            .unwrap();
        assert_eq!(alpha_lex["query_context"]["engine_scope"], "alpha");
        let alpha_items = alpha_lex["items"].as_array().unwrap();
        assert!(!alpha_items.is_empty(), "alpha lexical {route:?} must find alpha orchid rows");
        for item in alpha_items {
            for target in item["origin"]["targets"].as_array().unwrap() {
                assert_eq!(target["record"]["scope"], "alpha");
            }
        }

        let beta_lex = reader
            .lexical(Query {
                route,
                query: "orchid".into(),
                property: property.clone(),
                filter: LexicalFilter::default(),
                view: LexicalView::default(),
                scope: Some("beta".into()),
                limit: 20,
            })
            .unwrap();
        assert_eq!(beta_lex["query_context"]["engine_scope"], "beta");
        let beta_items = beta_lex["items"].as_array().unwrap();
        assert!(!beta_items.is_empty(), "beta lexical {route:?} must find beta orchid rows");
        for item in beta_items {
            for target in item["origin"]["targets"].as_array().unwrap() {
                assert_eq!(target["record"]["scope"], "beta");
            }
        }

        // scope: None -> Unscoped ("") strictly D-01-oblivious to non-empty scopes!
        let default_lex = reader
            .lexical(Query {
                route,
                query: "orchid".into(),
                property,
                filter: LexicalFilter::default(),
                view: LexicalView::default(),
                scope: None,
                limit: 20,
            })
            .unwrap();
        assert_eq!(default_lex["query_context"]["engine_scope"], "");
        let default_items = default_lex["items"].as_array().unwrap();
        assert!(!default_items.is_empty());
        for item in default_items {
            for target in item["origin"]["targets"].as_array().unwrap() {
                assert_eq!(target["record"]["scope"], "");
            }
        }
    }

    // 6. Native serving: switches engine scope cleanly and respects None -> ""!
    reader.session.cached_cpu = true;
    let alpha_srv = reader.serving("orchid", 10, Some("alpha")).unwrap();
    assert_eq!(alpha_srv["query_context"]["engine_scope"], "alpha");
    for item in alpha_srv["items"].as_array().unwrap() {
        for target in item["origin"]["targets"].as_array().unwrap() {
            assert_eq!(target["record"]["scope"], "alpha");
        }
    }
    let default_srv = reader.serving("orchid", 10, None).unwrap();
    assert_eq!(default_srv["query_context"]["engine_scope"], "");
    for item in default_srv["items"].as_array().unwrap() {
        for target in item["origin"]["targets"].as_array().unwrap() {
            assert_eq!(target["record"]["scope"], "");
        }
    }

    // 7. Vector neighborhood: zero cross-scope leakage when vectors exist in both "alpha" and "beta"!
    let c = rusqlite::Connection::open(&path).unwrap();
    let alpha_cursor: i64 = c
        .query_row(
            "SELECT write_cursor FROM canonical_nodes WHERE scope='alpha' AND logical_id='shared-node'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let beta_cursor: i64 = c
        .query_row(
            "SELECT write_cursor FROM canonical_nodes WHERE scope='beta' AND logical_id='shared-node'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let vec_bytes = 0.5_f32.to_le_bytes().repeat(384);
    let bin_bytes = vec![0xAA_u8; 48];
    c.execute(
        "INSERT OR IGNORE INTO _fathomdb_vector_kinds(kind, profile, created_at) VALUES ('document', 'default', 0)",
        [],
    )
    .unwrap();
    for (rowid, cursor, sc) in [(901_i64, alpha_cursor, "alpha"), (902_i64, beta_cursor, "beta")] {
        c.execute(
            "INSERT INTO _fathomdb_vector_rows(rowid, kind, write_cursor) VALUES (?1, 'document', ?2)",
            rusqlite::params![rowid, cursor],
        )
        .unwrap();
        c.execute(
            "INSERT INTO vector_default(rowid, embedding, embedding_bin, source_type, kind, created_at, status, lifecycle, valid_from, valid_until, scope, attr_73756d6d617279) VALUES (?1, ?2, vec_bit(?3), 'canonical', 'document', 0, '', 0, -9223372036854775808, 9223372036854775807, ?4, '')",
            rusqlite::params![rowid, vec_bytes, bin_bytes, sc],
        )
        .unwrap();
    }
    drop(c);

    let alpha_nb = reader.neighborhood(&alpha_node, 901, 0, 10).unwrap();
    let nb_items = alpha_nb["items"].as_array().unwrap();
    assert_eq!(nb_items.len(), 1, "neighborhood for alpha seed must NOT include beta vector");
    assert_eq!(nb_items[0]["vector_rowid"], "901");
    assert_eq!(nb_items[0]["origin"]["targets"][0]["record"]["scope"], "alpha");

    reader.session.close().unwrap();
}
