use crate::{
    fixtures::{database, edge_with, node},
    policy::Policy,
    session::Session,
    storage::{Class, Filter, Reader},
};
use fathomdb_engine::{EmbedderChoice, Engine};
#[test]
fn exact_canonical_browse_keeps_revisions_classes_and_policy_separate() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .write(&[
            node("alpha", "second revision"),
            edge_with(
                "relates",
                "alpha",
                "beta",
                "fixture:source",
                Some("alpha"),
                Some("edge body".into()),
                Some(10),
                Some(20),
                None,
                None,
                None,
            ),
        ])
        .unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let page = r.page(Class::Node, 0, 200, Filter::default()).unwrap();
    let items = page["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(page["scope"]["ordering"], "write_cursor_ascending");
    let alpha: Vec<_> = items.iter().filter(|x| x["logical_id"] == "alpha").collect();
    assert_eq!(alpha.len(), 2);
    assert_ne!(alpha[0]["handle"], alpha[1]["handle"]);
    let first = r.detail(alpha[0]["handle"].as_str().unwrap()).unwrap();
    assert_eq!(first["content"]["text"], "original text");
    assert_eq!(first["record"]["artifact_role"], "legacy");
    assert_eq!(first["record"]["completeness"], "migrated_incomplete");
    let edges = r.page(Class::Edge, 0, 200, Filter::default()).unwrap();
    assert_eq!(edges["items"][0]["logical_id"], "alpha");
    let edge = r.detail(edges["items"][0]["handle"].as_str().unwrap()).unwrap();
    assert_eq!(edge["content"]["text"], "edge body");
    assert_eq!(edge["record"]["from_id"], "alpha");
    assert!(edge["record"].get("state").is_none());
    assert!(r.detail("guessed-handle").is_err());
    let foreign = alpha[0]["handle"].as_str().unwrap().to_string();
    r.session.close().unwrap();
    let mut hidden = Reader::new(
        Session::attach(&path, Policy { no_show_inactive: true, ..Policy::default() }).unwrap(),
    );
    assert!(hidden.detail(&foreign).is_err());
    let page = hidden.page(Class::Node, 0, 200, Filter::default()).unwrap();
    assert_eq!(page["items"].as_array().unwrap().len(), 2);
    let exact = hidden
        .page(Class::Node, 0, 200, Filter { identity: Some("beta".into()), ..Filter::default() })
        .unwrap();
    assert_eq!(exact["items"].as_array().unwrap().len(), 1);
    assert!(hidden
        .page(
            Class::Node,
            0,
            200,
            Filter { collection: Some("unrelated".into()), ..Filter::default() }
        )
        .is_err());
    hidden.session.close().unwrap();
}

#[test]
fn display_previews_use_small_admitted_stored_fields_without_loading_large_bodies() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    let large =
        serde_json::json!({"body":"x".repeat(1024*1024),"title":"late title must not be parsed"})
            .to_string();
    opened
        .engine
        .write(&[
            node("titled", r#"{"title":"Actual note title 🦀","identifier":9007199254740993}"#),
            node("named", r#"{"name":"Stored person name","title":null}"#),
            node("large", &large),
            node("wrong-type", r#"{"title":12,"name":false}"#),
            node("truncated-title", &serde_json::json!({"title":"é".repeat(140)}).to_string()),
        ])
        .unwrap();
    opened.engine.close().unwrap();
    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let page = reader.page(Class::Node, 0, 200, Filter::default()).unwrap();
    let find = |id: &str| {
        page["items"].as_array().unwrap().iter().find(|item| item["logical_id"] == id).unwrap()
    };
    assert_eq!(find("titled")["display_preview"]["text"], "Actual note title 🦀");
    assert_eq!(find("named")["display_preview"]["field"], "name");
    assert_eq!(find("large")["display_preview"]["state"], "oversize_not_inspected");
    assert_eq!(find("wrong-type")["display_preview"]["state"], "no_display_field");
    assert_eq!(find("truncated-title")["display_preview"]["complete"], false);
    let detail = reader.detail(find("titled")["handle"].as_str().unwrap()).unwrap();
    assert_eq!(detail["record"]["display_preview"]["text"], "Actual note title 🦀");
    reader.session.close().unwrap();
}

#[test]
fn bounded_stored_labels_follow_all_eight_disclosure_combinations() {
    for bits in 0..8 {
        let dir = tempfile::tempdir().unwrap();
        let path = database(dir.path());
        let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
        opened
            .engine
            .write(&[node("labelled-history", r#"{"title":"retained private title"}"#)])
            .unwrap();
        opened
            .engine
            .write(&[node("labelled-history", r#"{"title":"current visible title"}"#)])
            .unwrap();
        opened.engine.close().unwrap();
        drop(opened);
        let policy = Policy {
            no_show_inactive: bits & 1 != 0,
            show_erased: bits & 2 != 0,
            show_payloads: bits & 4 != 0,
        };
        let mut reader = Reader::new(Session::attach(&path, policy).unwrap());
        let page = reader
            .page(
                Class::Node,
                0,
                200,
                Filter { identity: Some("labelled-history".into()), ..Filter::default() },
            )
            .unwrap();
        let items = page["items"].as_array().unwrap();
        assert_eq!(items.len(), if bits & 1 != 0 { 1 } else { 2 });
        assert_eq!(page.to_string().contains("retained private title"), bits & 1 == 0);
        assert!(page.to_string().contains("current visible title"));
        for item in items {
            let detail = reader.detail(item["handle"].as_str().unwrap()).unwrap();
            assert_eq!(detail["record"]["display_preview"], item["display_preview"]);
        }
        reader.session.close().unwrap();
    }
}

#[test]
fn outside_body_properties_scope_display_preview_fallback_and_exact_lexemes() {
    use fathomdb_engine::{PreparedWrite, PropertyMap, PropertyValue, SourceId};
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    let mut node_props = PropertyMap::new();
    node_props.insert("title".into(), PropertyValue::String("Property Fallback Title".into()));
    node_props.insert("big_i64".into(), PropertyValue::Int(9007199254740993));
    node_props.insert("verified".into(), PropertyValue::Bool(true));
    node_props.insert("empty_str".into(), PropertyValue::String(String::new()));
    let mut edge_props = PropertyMap::new();
    edge_props.insert("name".into(), PropertyValue::String("Edge Fallback Name".into()));
    edge_props.insert("weight".into(), PropertyValue::Float(1.5));
    edge_props.insert("active".into(), PropertyValue::Bool(false));
    let long_plain_body = "plain text body ".repeat(188); // ~3008 bytes plain text (non-JSON)
    assert!(long_plain_body.len() > 2900 && long_plain_body.len() < 3500);
    let mut prop_pad = node_props.clone();
    prop_pad.insert("padding".into(), PropertyValue::String("p".repeat(1800))); // > 1900 bytes total properties
    opened
        .engine
        .write(&[
            PreparedWrite::Node {
                kind: "document".into(),
                body: long_plain_body.clone(),
                source_id: SourceId::new("fixture:props").unwrap(),
                logical_id: Some("props-node".into()),
                state: fathomdb_engine::InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: Some(prop_pad),
            },
            PreparedWrite::Edge {
                kind: "relates".into(),
                from: "props-node".into(),
                to: "alpha".into(),
                source_id: SourceId::new("fixture:props").unwrap(),
                logical_id: Some("props-edge".into()),
                body: Some("plain text edge body".into()),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
                properties: Some(edge_props),
            },
            PreparedWrite::AdminSchema {
                name: "op-scope".into(),
                kind: "latest_state".into(),
                schema_json: r#"{"type":"object"}"#.into(),
                retention_json: "{}".into(),
            },
            PreparedWrite::AdminSchema {
                name: "op-log".into(),
                kind: "append_only_log".into(),
                schema_json: r#"{"type":"object"}"#.into(),
                retention_json: "{}".into(),
            },
        ])
        .unwrap();
    opened
        .engine
        .write(&[
            PreparedWrite::OpStore {
                collection: "op-scope".into(),
                record_key: "k1".into(),
                schema_id: None,
                body: r#"{"ok":true}"#.into(),
            },
            PreparedWrite::OpStore {
                collection: "op-log".into(),
                record_key: "k2".into(),
                schema_id: None,
                body: r#"{"ok":true}"#.into(),
            },
        ])
        .unwrap();
    opened.engine.close().unwrap();
    drop(opened);

    // Also verify raw SQLite JSON lexemes (beyond i64 & -0 & NULL) in properties via tree_field.
    let raw_props =
        r#"{"huge":9007199254740993123456789,"neg_zero":-0,"flag":true,"empty":"","nil":null}"#;
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute("UPDATE canonical_nodes SET properties=?1 WHERE logical_id='alpha'", [raw_props])
        .unwrap();
    drop(c);

    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let nodes = reader
        .page(
            Class::Node,
            0,
            200,
            Filter { identity: Some("props-node".into()), ..Default::default() },
        )
        .unwrap();
    let item = &nodes["items"][0];
    assert_eq!(item["scope"], "");
    assert_eq!(item["properties_status"], "available");
    assert_eq!(item["properties_storage"], "text");
    assert!(item["properties_bytes"].as_u64().unwrap() > 1800);
    assert_eq!(item["display_preview"]["state"], "available");
    assert_eq!(item["display_preview"]["text"], "Property Fallback Title");
    assert_eq!(item["display_preview"]["field"], "title");

    let handle = item["handle"].as_str().unwrap();
    let detail = reader.detail(handle).unwrap();
    assert_eq!(detail["record"]["scope"], "");
    assert_eq!(detail["record"]["display_preview"]["state"], "available");
    assert_eq!(detail["record"]["display_preview"]["text"], "Property Fallback Title");
    let props_detail = reader.detail_field(handle, "properties").unwrap();
    assert_eq!(props_detail["selected_field"], "properties");
    assert!(props_detail["content"]["text"].as_str().unwrap().contains("Property Fallback Title"));
    let props_chunk = reader.chunk(handle, "properties", 0, 1024).unwrap();
    assert!(props_chunk["text"].as_str().unwrap().starts_with('{'));

    // Edge outside-body properties & fallback display preview
    let edges = reader
        .page(
            Class::Edge,
            0,
            200,
            Filter { identity: Some("props-edge".into()), ..Default::default() },
        )
        .unwrap();
    let edge_item = &edges["items"][0];
    assert_eq!(edge_item["scope"], "");
    assert_eq!(edge_item["properties_status"], "available");
    assert_eq!(edge_item["display_preview"]["text"], "Edge Fallback Name");
    assert_eq!(edge_item["display_preview"]["field"], "name");
    let edge_handle = edge_item["handle"].as_str().unwrap();
    let edge_props_detail = reader.detail_field(edge_handle, "properties").unwrap();
    assert!(edge_props_detail["content"]["text"].as_str().unwrap().contains("Edge Fallback Name"));
    let edge_tree = reader.tree_field(edge_handle, Some("properties"), 0, None).unwrap();
    assert_eq!(edge_tree["kind"], "object");
    assert!(edge_tree["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["key"] == "name" && i["lexeme"] == r#""Edge Fallback Name""#));

    // Exact lexemes on alpha's raw JSON properties (beyond 2^53, -0, bool, empty, null)
    let alpha_page = reader
        .page(Class::Node, 0, 10, Filter { identity: Some("alpha".into()), ..Default::default() })
        .unwrap();
    let alpha_handle = alpha_page["items"][0]["handle"].as_str().unwrap();
    let alpha_tree = reader.tree_field(alpha_handle, Some("properties"), 0, None).unwrap();
    let items = alpha_tree["items"].as_array().unwrap();
    assert_eq!(items[0]["key"], "huge");
    assert_eq!(items[0]["lexeme"], "9007199254740993123456789");
    assert_eq!(items[1]["key"], "neg_zero");
    assert_eq!(items[1]["lexeme"], "-0");
    assert_eq!(items[2]["key"], "flag");
    assert_eq!(items[2]["lexeme"], "true");
    assert_eq!(items[3]["key"], "empty");
    assert_eq!(items[3]["lexeme"], r#""""#);
    assert_eq!(items[4]["key"], "nil");
    assert_eq!(items[4]["kind"], "null");
    assert_eq!(items[4]["lexeme"], "null");
    assert_eq!(
        reader.tree_field(alpha_handle, Some("logical_id"), 0, None),
        Err("unsupported_field")
    );

    // Node with SQL NULL properties reports null cleanly
    let beta_page = reader
        .page(Class::Node, 0, 10, Filter { identity: Some("beta".into()), ..Default::default() })
        .unwrap();
    assert_eq!(beta_page["items"][0]["properties_status"], "available");
    assert_eq!(beta_page["items"][0]["properties_storage"], "null");
    assert!(beta_page["items"][0]["properties_bytes"].is_null());
    let beta_handle = beta_page["items"][0]["handle"].as_str().unwrap();
    assert_eq!(reader.tree_field(beta_handle, Some("properties"), 0, None), Err("null"));

    // Operational records expose scope and reject properties
    for class in [Class::Current, Class::Log] {
        let op_page = reader.page(class, 0, 10, Filter::default()).unwrap();
        let op_item = &op_page["items"][0];
        assert_eq!(op_item["scope"], "");
        let op_handle = op_item["handle"].as_str().unwrap();
        assert_eq!(reader.detail_field(op_handle, "properties"), Err("unsupported_field"));
        assert_eq!(
            reader.tree_field(op_handle, Some("properties"), 0, None),
            Err("unsupported_field")
        );
    }
    reader.session.close().unwrap();
}
