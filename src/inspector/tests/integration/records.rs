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
