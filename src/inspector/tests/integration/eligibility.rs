use super::{Class, Filter, Reader};
use crate::fixtures;
use crate::{policy::Policy, session::Session};

#[test]
fn raw_classification_uses_integer_boundaries_and_withholds_unknown_lifecycle() {
    use fathomdb_engine::{EmbedderChoice, Engine, InitialState};
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    opened
        .engine
        .write(&[
            fixtures::node_with(
                "timed",
                Some("timed-node"),
                "timed body",
                "fixture:timed",
                InitialState::Active,
                None,
                Some(10),
                Some(20),
            ),
            fixtures::edge_with(
                "timed",
                "alpha",
                "beta",
                "fixture:timed",
                Some("timed-edge"),
                None,
                Some(30),
                Some(40),
                None,
                None,
                None,
            ),
        ])
        .unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    for (class, id, cases) in [
        (
            Class::Node,
            "timed-node",
            vec![(9, true), (10, false), (19, false), (20, true), (21, true)],
        ),
        (
            Class::Edge,
            "timed-edge",
            vec![(9, false), (30, false), (39, false), (40, true), (41, true)],
        ),
    ] {
        let p = r
            .page(class, 0, 10, Filter { identity: Some(id.into()), ..Filter::default() })
            .unwrap();
        let locator = r.handles.get(p["items"][0]["handle"].as_str().unwrap()).unwrap().clone();
        for (at, inactive) in cases {
            let mut c = r.session.connection().unwrap();
            c.at_epoch_s = at;
            let (record, classification) = Reader::summary(&c, &locator).unwrap();
            assert_eq!(classification.inactive, inactive, "{class:?} at integer{at}");
            assert!(classification.known);
            if class == Class::Edge {
                assert!(record.get("state").is_none());
            }
        }
    }
    r.session.close().unwrap();
    let c = rusqlite::Connection::open(&path).unwrap();
    c.pragma_update(None, "ignore_check_constraints", true).unwrap();
    c.execute("UPDATE canonical_nodes SET state='future-state' WHERE logical_id='alpha'", [])
        .unwrap();
    drop(c);
    let session = match Session::attach(&path, Policy::default()) {
        Ok(session) => session,
        Err(_) => return,
    };
    let mut r = Reader::new(session);
    let p = r
        .page(Class::Node, 0, 200, Filter { identity: Some("alpha".into()), ..Filter::default() })
        .unwrap();
    assert!(
        p["items"].as_array().unwrap().is_empty(),
        "unknown lifecycle must not authorize content"
    );
    assert_eq!(p["status"], "partial");
    assert_eq!(p["scope"]["unknown_classification"], 1);
    assert!(p["scope"]["continuation"].is_null());
    let catalog = r.catalog().unwrap();
    assert_eq!(catalog["counts"]["node"]["exact"], false);
    assert_eq!(catalog["counts"]["node"]["unknown_classification"], 1);
    r.session.close().unwrap();
}
