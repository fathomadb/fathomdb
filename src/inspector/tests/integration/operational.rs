use crate::{
    fixtures::database,
    policy::Policy,
    session::Session,
    storage::{Class, Filter, Reader},
};
use fathomdb_engine::{EmbedderChoice, Engine, PreparedWrite};
#[test]
fn catalog_counts_and_operational_modes_do_not_invent_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    for (name, kind) in
        [("events", "append_only_log"), ("state", "latest_state"), ("unrelated", "latest_state")]
    {
        opened
            .engine
            .write(&[PreparedWrite::AdminSchema {
                name: name.into(),
                kind: kind.into(),
                schema_json: r#"{"type":"object"}"#.into(),
                retention_json: "{}".into(),
            }])
            .unwrap();
    }
    for (collection, body) in [
        ("events", r#"{"n":1}"#),
        ("events", r#"{"n":2}"#),
        ("state", r#"{"n":1}"#),
        ("state", r#"{"n":2}"#),
        ("unrelated", r#"{"n":7}"#),
    ] {
        opened
            .engine
            .write(&[PreparedWrite::OpStore {
                collection: collection.into(),
                record_key: "same-key".into(),
                schema_id: None,
                body: body.into(),
            }])
            .unwrap();
    }
    opened.engine.close().unwrap();
    drop(opened);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let catalog = r.catalog().unwrap();
    assert_eq!(catalog["counts"]["node"]["revisions"], "2");
    assert_eq!(catalog["counts"]["node"]["logical_identities"], "2");
    assert_eq!(catalog["counts"]["current"]["values"], "2");
    assert_eq!(catalog["counts"]["log"]["entries"], "2");
    assert_eq!(catalog["scope"]["exact"], true);
    assert!(catalog["collections"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["name"] == "events" && c["mode"] == "append_only_log"));
    let log = r
        .page(Class::Log, 0, 200, Filter { collection: Some("events".into()), ..Filter::default() })
        .unwrap();
    assert_eq!(log["items"].as_array().unwrap().len(), 2);
    assert_ne!(log["items"][0]["handle"], log["items"][1]["handle"]);
    let state = r
        .page(
            Class::Current,
            0,
            200,
            Filter { collection: Some("state".into()), ..Filter::default() },
        )
        .unwrap();
    assert_eq!(state["items"].as_array().unwrap().len(), 1);
    let value = r.detail(state["items"][0]["handle"].as_str().unwrap()).unwrap();
    assert_eq!(value["content"]["text"], r#"{"n":2}"#);
    assert_eq!(value["record"]["mode"], "latest_state");
    let missing = r
        .page(
            Class::Current,
            0,
            200,
            Filter { identity: Some("missing".into()), ..Filter::default() },
        )
        .unwrap();
    assert!(missing["items"].as_array().unwrap().is_empty());
    r.session.close().unwrap();
}

#[test]
fn log_pages_keep_exact_entry_ids_and_use_retained_entry_order() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::AdminSchema {
            name: "events".into(),
            kind: "append_only_log".into(),
            schema_json: r#"{"type":"object"}"#.into(),
            retention_json: "{}".into(),
        }])
        .unwrap();
    for n in 0..3 {
        opened
            .engine
            .write(&[PreparedWrite::OpStore {
                collection: "events".into(),
                record_key: "repeated".into(),
                schema_id: None,
                body: format!("{{\"n\":{n}}}"),
            }])
            .unwrap();
    }
    opened.engine.close().unwrap();
    drop(opened);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let mut after = 0;
    let mut ids = vec![];
    loop {
        let p = r
            .page(
                Class::Log,
                after,
                1,
                Filter { collection: Some("events".into()), ..Filter::default() },
            )
            .unwrap();
        assert_eq!(p["scope"]["ordering"], "entry_id_ascending");
        for item in p["items"].as_array().unwrap() {
            let id = item["entry_id"].as_str().unwrap().parse::<i64>().unwrap();
            assert!(id > after);
            ids.push(id);
            let detail = r.detail(item["handle"].as_str().unwrap()).unwrap();
            assert_eq!(detail["record"]["entry_id"], item["entry_id"]);
        }
        match p["scope"]["continuation"].as_str() {
            Some(v) => {
                after = v.parse().unwrap();
                assert_eq!(Some(&after), ids.last());
            }
            None => break,
        }
    }
    assert_eq!(ids.len(), 3);
    assert!(ids.windows(2).all(|w| w[0] < w[1]));
    r.session.close().unwrap();
}

#[test]
fn catalog_unicode_declarations_share_one_response_byte_budget() {
    use fathomdb_engine::ProjectionRole;
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    let declarations = (0..200)
        .map(|n| {
            crate::fixtures::projection_spec(
                &format!("{n}-{}", "🦀".repeat(4096)),
                &[ProjectionRole::Searchable],
                None,
                None,
                None,
            )
        })
        .collect::<Vec<_>>();
    opened.engine.configure_projections(&declarations, &[]).unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let catalog = r.catalog().unwrap();
    assert!(serde_json::to_vec(&catalog).unwrap().len()<=crate::storage::RESPONSE,"catalog metadata must share the transport byte cap even when every individual preview/item is bounded");
    assert_eq!(catalog["status"], "partial");
    assert_eq!(catalog["scope"]["projections_complete"], false);
    assert_eq!(catalog["scope"]["metadata_or_group_truncated"], true);
    assert_eq!(catalog["counts"]["node"]["revisions"], "2");
    assert_eq!(catalog["counts"]["node"]["exact"], true);
    r.session.close().unwrap();
}

#[test]
fn catalog_collections_declarations_and_groups_share_one_item_limit() {
    use fathomdb_engine::ProjectionRole;
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    let declarations = (0..200)
        .map(|n| {
            crate::fixtures::projection_spec(
                &format!("field-{n}"),
                &[ProjectionRole::Searchable],
                None,
                None,
                None,
            )
        })
        .collect::<Vec<_>>();
    opened.engine.configure_projections(&declarations, &[]).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::AdminSchema {
            name: "registered-state".into(),
            kind: "latest_state".into(),
            schema_json: r#"{"type":"object"}"#.into(),
            retention_json: "{}".into(),
        }])
        .unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let catalog = r.catalog().unwrap();
    let returned = catalog["collections"].as_array().unwrap().len()
        + catalog["projections"].as_array().unwrap().len()
        + catalog["groups"].as_object().unwrap().len();
    assert!(
        returned <= 200,
        "all catalog metadata categories must share one response item limit, actual={returned}"
    );
    assert_eq!(catalog["scope"]["returned_metadata_and_groups"], returned);
    assert_eq!(catalog["scope"]["groups_complete"], false);
    assert_eq!(catalog["status"], "partial");
    assert_eq!(catalog["counts"]["node"]["revisions"], "2");
    assert_eq!(catalog["counts"]["node"]["exact"], true);
    r.session.close().unwrap();
}
