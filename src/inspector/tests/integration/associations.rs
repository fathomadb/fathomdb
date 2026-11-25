use crate::{
    fixtures::{add_related, database},
    policy::Policy,
    session::Session,
    storage::{Class, Filter, Reader},
};
#[test]
fn recorded_source_and_projection_associations_are_distinct_and_exact() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    add_related(&path);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let page = r
        .page(Class::Node, 0, 200, Filter { identity: Some("derived".into()), ..Filter::default() })
        .unwrap();
    let h = page["items"][0]["handle"].as_str().unwrap();
    let a = r.associations(h).unwrap();
    assert_eq!(a["links"][0]["kind"], "recorded_source");
    assert_eq!(a["links"][0]["cardinality"], 1);
    let source = r.detail(a["links"][0]["targets"][0]["handle"].as_str().unwrap()).unwrap();
    assert_eq!(source["record"]["revision_id"], "source-r1");
    assert_eq!(source["record"]["artifact_role"], "canonical_source");
    assert_eq!(a["attributes"][0]["name"], "title");
    assert_eq!(a["attributes"][0]["value"], "derived-title");
    assert_eq!(a["projections"][0]["declared"], true);
    assert_eq!(a["projections"][0]["materialized_attribute"], true);
    let catalog = r.catalog().unwrap();
    assert_eq!(catalog["counts"]["node"]["revisions"], "4");
    assert_eq!(catalog["counts"]["projection"]["attribute_entries"], "2");
    r.session.close().unwrap();
}

#[test]
fn reactivated_canonical_source_is_not_classified_from_its_historical_closure() {
    use fathomdb_engine::{LifecycleState, SourceDependencyRegistrationV1};
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    add_related(&path);
    let mut r = Reader::new(
        Session::attach(&path, Policy { no_show_inactive: true, ..Policy::default() }).unwrap(),
    );
    r.session
        .engine()
        .unwrap()
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep", "source-r1", "derived-r1").unwrap(),
        )
        .unwrap();
    r.session.engine().unwrap().transition("source", LifecycleState::Deleted, None).unwrap();
    r.session.engine().unwrap().transition("source", LifecycleState::Active, None).unwrap();
    let p = r
        .page(Class::Node, 0, 200, Filter { identity: Some("source".into()), ..Filter::default() })
        .unwrap();
    assert_eq!(
        p["items"].as_array().unwrap().len(),
        1,
        "reactivated source is active; old dependency closure does not own the canonical source"
    );
    let d = r.detail(p["items"][0]["handle"].as_str().unwrap()).unwrap();
    assert_eq!(d["record"]["inactive_or_history"], false);
    r.session.close().unwrap();
}

#[test]
fn materialized_attribute_has_its_own_exact_origin_and_full_chunk_access() {
    use fathomdb_engine::{EmbedderChoice, Engine};
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    add_related(&path);
    let title = "exact-attribute-é🦀".repeat(10000);
    let body = serde_json::json!({"title":title}).to_string();
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    opened.engine.write(&[crate::fixtures::node("large-attribute", &body)]).unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let p = r
        .page(
            Class::Node,
            0,
            200,
            Filter { identity: Some("large-attribute".into()), ..Filter::default() },
        )
        .unwrap();
    let origin = p["items"][0]["handle"].as_str().unwrap();
    let a = r.associations(origin).unwrap();
    let handle = a["attributes"][0]["handle"]
        .as_str()
        .expect("materialized attribute needs a typed exact handle");
    assert_ne!(handle, origin);
    let d = r.detail(handle).unwrap();
    assert_eq!(d["record"]["class"], "attribute");
    assert_eq!(d["record"]["origin"]["logical_id"], "large-attribute");
    let mut actual = String::new();
    let mut offset = 0;
    loop {
        let part = r.chunk(handle, "body", offset, crate::storage::CHUNK).unwrap();
        actual.push_str(part["text"].as_str().unwrap());
        match part["continuation"].as_str() {
            Some(n) => offset = n.parse().unwrap(),
            None => break,
        }
    }
    assert_eq!(actual, title);
    assert!(
        r.chunk(handle, "logical_id", 0, 10).is_err(),
        "attribute handle cannot be coerced to canonical record"
    );
    r.session
        .engine()
        .unwrap()
        .transition("large-attribute", fathomdb_engine::LifecycleState::Deleted, None)
        .unwrap();
    assert!(
        r.detail(handle).is_err(),
        "removed materialization must not substitute canonical content"
    );
    r.session.close().unwrap();
}

#[test]
fn association_categories_share_one_item_and_byte_budget_and_report_partial() {
    use fathomdb_engine::{EmbedderChoice, Engine, ProjectionRole};
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let declarations = (0..201)
        .map(|n| {
            crate::fixtures::projection_spec(
                &format!("p{n:03}"),
                &[ProjectionRole::Searchable],
                None,
                None,
                None,
            )
        })
        .collect::<Vec<_>>();
    let body: serde_json::Map<String, serde_json::Value> = (0..201)
        .map(|n| (format!("p{n:03}"), serde_json::Value::String("🦀".repeat(4096))))
        .collect();
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    opened.engine.configure_projections(&declarations, &[]).unwrap();
    opened
        .engine
        .write(&[crate::fixtures::node(
            "many-attributes",
            &serde_json::Value::Object(body).to_string(),
        )])
        .unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let page = r
        .page(
            Class::Node,
            0,
            10,
            Filter { identity: Some("many-attributes".into()), ..Filter::default() },
        )
        .unwrap();
    let a = r.associations(page["items"][0]["handle"].as_str().unwrap()).unwrap();
    assert_eq!(
        a["status"], "partial",
        "capped materializations/declarations cannot claim complete"
    );
    let returned = a["attributes"].as_array().unwrap().len()
        + a["projections"].as_array().unwrap().len()
        + a["links"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["targets"].as_array().unwrap().len())
            .sum::<usize>();
    assert!(returned <= 200);
    assert_eq!(a["scope"]["returned"], returned);
    assert!(serde_json::to_vec(&a).unwrap().len() <= 2 * 1024 * 1024);
    assert_eq!(a["attributes_complete"], false);
    assert_eq!(a["projections_complete"], false);
    r.session.close().unwrap();
}

#[test]
fn projection_presence_constraint_returns_only_exact_scalar_materialization_origins() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    add_related(&path);
    let opened =
        fathomdb_engine::Engine::open_with_choice(&path, fathomdb_engine::EmbedderChoice::None)
            .unwrap();
    opened
        .engine
        .write(&[
            crate::fixtures::node("absent-title", r#"{"other":"value"}"#),
            crate::fixtures::node("null-title", r#"{"title":null}"#),
            crate::fixtures::node("object-title", r#"{"title":{"nested":"value"}}"#),
        ])
        .unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    let filter = |name: &str| {
        serde_json::from_value::<Filter>(serde_json::json!({"attribute":name}))
            .expect("exact scalar attribute presence is a supported node constraint")
    };
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let page = r.page(Class::Node, 0, 200, filter("title")).unwrap();
    let ids = page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["logical_id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["source", "derived"]);
    assert_eq!(
        page["scope"]["attribute_constraint_meaning"],
        "stored_scalar_materialization_presence; no body-field absence or readiness proof"
    );
    assert!(page["scope"]["attribute_examined"].as_u64().unwrap() >= 2);
    assert!(page["scope"]["examined"].as_u64().unwrap() <= 100000);
    for name in ["tit", "missing-title"] {
        assert!(r.page(Class::Node, 0, 10, filter(name)).unwrap()["items"]
            .as_array()
            .unwrap()
            .is_empty());
    }
    for class in [Class::Edge, Class::Current, Class::Log] {
        assert_eq!(r.page(class, 0, 10, filter("title")), Err("unsupported_constraint"));
    }
    r.session.policy.no_show_inactive = true;
    r.session
        .engine()
        .unwrap()
        .transition("source", fathomdb_engine::LifecycleState::Deleted, None)
        .unwrap();
    let page = r.page(Class::Node, 0, 200, filter("title")).unwrap();
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert_eq!(page["items"][0]["logical_id"], "derived");
    r.session.close().unwrap();
}

#[test]
fn detail_separates_stored_inspection_readiness_from_unknown_retrieval_and_session_runtime() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    add_related(&path);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let page = r
        .page(Class::Node, 0, 10, Filter { identity: Some("derived".into()), ..Filter::default() })
        .unwrap();
    let handle = page["items"][0]["handle"].as_str().unwrap();
    let detail = r.detail(handle).unwrap();
    let facts = &detail["facts"];
    assert_eq!(
        facts["storage"]["representation"], "canonical_document",
        "document storage must be a fact separate from retrieval"
    );
    assert_eq!(facts["storage"]["state"], "present");
    assert_eq!(facts["storage"]["bytes"], detail["content"]["total_bytes"]);
    assert_eq!(facts["eligibility"]["inspection"], "allowed");
    assert_eq!(facts["eligibility"]["retrieval"], "unknown");
    assert_eq!(facts["readiness"]["representation_read"], "ready");
    assert_eq!(facts["readiness"]["retrieval"], "unknown");
    assert_eq!(facts["runtime"]["dense_query"], "unavailable");
    assert_eq!(facts["runtime"]["reason"], "session_opened_with_embedder_none");
    assert_eq!(facts["runtime"]["scope"], "inspector_session");
    for key in ["storage", "eligibility", "readiness"] {
        assert_eq!(facts[key]["source"], "inspector_current_layout");
        assert_eq!(facts[key]["observation"], "detail_context");
    }
    let associations = r.associations(handle).unwrap();
    let attribute = r.detail(associations["attributes"][0]["handle"].as_str().unwrap()).unwrap();
    assert_eq!(attribute["facts"]["storage"]["representation"], "scalar_attribute");
    assert_eq!(attribute["facts"]["readiness"]["representation_read"], "ready");
    assert_eq!(attribute["facts"]["readiness"]["retrieval"], "unknown");
    r.session.close().unwrap();
}

#[test]
fn materialization_origin_reference_navigates_to_the_exact_retained_document() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    add_related(&path);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let page = r
        .page(Class::Node, 0, 10, Filter { identity: Some("derived".into()), ..Filter::default() })
        .unwrap();
    let handle = page["items"][0]["handle"].as_str().unwrap();
    let original = r.detail(handle).unwrap();
    let associations = r.associations(handle).unwrap();
    let attribute = r.detail(associations["attributes"][0]["handle"].as_str().unwrap()).unwrap();
    let origin = attribute["origin_reference"]["handle"]
        .as_str()
        .expect("scalar detail must provide an actionable exact canonical origin");
    let restored = r.detail(origin).unwrap();
    assert_eq!(restored["record"]["class"], original["record"]["class"]);
    assert_eq!(restored["record"]["write_cursor"], original["record"]["write_cursor"]);
    assert_eq!(restored["record"]["revision_id"], "derived-r1");
    assert_eq!(restored["content"], original["content"]);
    assert_ne!(
        restored["context"]["operation"], original["context"]["operation"],
        "origin hydration is a separate read, not the original snapshot"
    );
    r.session.close().unwrap();
}

#[test]
fn node_and_edge_property_values_and_bounded_catalog_respect_entity_class_and_policy() {
    use fathomdb_engine::{
        EmbedderChoice, Engine, PreparedWrite, ProjectionRole, ProjectionSpec, PropertyEntityClass,
        PropertyMap, PropertyValue, SourceId,
    };
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .configure_projections(
            &[
                ProjectionSpec {
                    name: "category".into(),
                    roles: [ProjectionRole::Filterable].into_iter().collect(),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
                ProjectionSpec {
                    name: "relation_weight".into(),
                    roles: [ProjectionRole::Filterable].into_iter().collect(),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Edge,
                },
            ],
            &[],
        )
        .unwrap();
    let mut node_props_expired = PropertyMap::new();
    node_props_expired.insert("category".into(), PropertyValue::String("expired".into()));
    let mut node_props_current = PropertyMap::new();
    node_props_current.insert("category".into(), PropertyValue::String("current".into()));
    let mut edge_props = PropertyMap::new();
    edge_props.insert("relation_weight".into(), PropertyValue::Int(9007199254740993));
    opened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "document".into(),
            body: "expired window".into(),
            source_id: SourceId::new("fixture:cpv").unwrap(),
            logical_id: Some("cpv-expired".into()),
            state: fathomdb_engine::InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: Some(1),
            properties: Some(node_props_expired),
        }])
        .unwrap();
    opened
        .engine
        .write(&[
            PreparedWrite::Node {
                kind: "document".into(),
                body: "v2".into(),
                source_id: SourceId::new("fixture:cpv").unwrap(),
                logical_id: Some("cpv-node".into()),
                state: fathomdb_engine::InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: Some(node_props_current),
            },
            PreparedWrite::Edge {
                kind: "links".into(),
                from: "cpv-node".into(),
                to: "alpha".into(),
                source_id: SourceId::new("fixture:cpv").unwrap(),
                logical_id: Some("cpv-edge".into()),
                body: Some("edge".into()),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
                properties: Some(edge_props),
            },
        ])
        .unwrap();
    opened.engine.close().unwrap();
    drop(opened);

    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let catalog = r.catalog().unwrap();
    assert_eq!(catalog["counts"]["projection"]["property_value_entries"], "3");
    assert_eq!(catalog["counts"]["projection"]["property_value_exact"], true);
    assert!(
        catalog["counts"]["commit_timestamps"]["entries"].as_str().unwrap().parse::<u64>().unwrap()
            >= 2
    );
    let projections = catalog["projections"].as_array().unwrap();
    assert!(projections.iter().any(|p| p["name"] == "category" && p["entity_class"] == "node"));
    assert!(projections
        .iter()
        .any(|p| p["name"] == "relation_weight" && p["entity_class"] == "edge"));

    let node_page = r
        .page(
            Class::Node,
            0,
            20,
            Filter { identity: Some("cpv-node".into()), ..Default::default() },
        )
        .unwrap();
    let node_h = node_page["items"][0]["handle"].as_str().unwrap();
    let node_assoc = r.associations(node_h).unwrap();
    assert!(node_assoc["projections"]
        .as_array()
        .unwrap()
        .iter()
        .all(|p| p["entity_class"] == "node"));
    assert!(node_assoc["projections"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["name"] == "category" && p["materialized_property_value"] == true));
    assert!(node_assoc["property_values"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["name"] == "category"
            && v["value_type"] == "string"
            && v["value"] == "current"));

    let edge_page = r
        .page(
            Class::Edge,
            0,
            20,
            Filter { identity: Some("cpv-edge".into()), ..Default::default() },
        )
        .unwrap();
    let edge_h = edge_page["items"][0]["handle"].as_str().unwrap();
    let edge_assoc = r.associations(edge_h).unwrap();
    assert!(edge_assoc["projections"]
        .as_array()
        .unwrap()
        .iter()
        .all(|p| p["entity_class"] == "edge"));
    assert!(edge_assoc["projections"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["name"] == "relation_weight" && p["materialized_property_value"] == true));
    assert!(edge_assoc["property_values"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["name"] == "relation_weight"
            && v["value_type"] == "integer"
            && v["value"] == "9007199254740993"));
    r.session.close().unwrap();

    // With --no-show-inactive, superseded cpv-node v1 property value is excluded from catalog count.
    let mut r_active = Reader::new(
        Session::attach(&path, Policy { no_show_inactive: true, ..Default::default() }).unwrap(),
    );
    let catalog_active = r_active.catalog().unwrap();
    assert_eq!(catalog_active["counts"]["projection"]["property_value_entries"], "2");
    r_active.session.close().unwrap();
}
