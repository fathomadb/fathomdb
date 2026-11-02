use super::*;

fn change(claim: &str, body: &str, family: Family, known: i64) -> Change {
    Change {
        claim: claim.into(), body: body.into(), family,
        properties: BTreeMap::from([("summary".into(), body.into()), ("notes".into(), "separate property baseline".into())]),
        known, valid_from: 0, valid_to: None, intent: Intent::Revision,
        source: "source".into(), endpoints: None,
    }
}

fn identity_scores(hits: &[Hit]) -> Vec<(String, f64)> {
    hits.iter().map(|h| (h.claim.clone(), h.score)).collect()
}

#[test]
fn h1_current_node_edge_and_property_scores_ignore_excluded_revisions() {
    for family in [Family::Node, Family::Edge] {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let mut clean = Store::open(&a.path().join("clean.sqlite"), 50).unwrap();
        let mut history = Store::open(&b.path().join("history.sqlite"), 50).unwrap();
        for (id, body, known) in [("a", "alpha", 100), ("b", "beta beta extra", 101), ("c", "gamma filler", 102)] {
            clean.put(change(id, body, family, known)).unwrap();
        }
        for known in 1..30 {
            history.put(change("a", &"alpha distant old content ".repeat(5), family, known)).unwrap();
        }
        for (id, body, known) in [("a", "alpha", 100), ("b", "beta beta extra", 101), ("c", "gamma filler", 102)] {
            history.put(change(id, body, family, known)).unwrap();
        }
        for (projection, name) in [(family, None), (Family::Property, Some("summary"))] {
            if family == Family::Edge && projection == Family::Property { continue; }
            let expected = clean.search("alpha OR beta", projection, name, 10).unwrap();
            assert_eq!(expected.len(), 2, "current matches must exist before invariance is meaningful");
            let actual = history.search("alpha OR beta", projection, name, 10).unwrap();
            assert_eq!(identity_scores(&actual), identity_scores(&expected));
            assert_eq!(history.search("distant", projection, name, 10).unwrap().len(), 0);
        }
    }
}
