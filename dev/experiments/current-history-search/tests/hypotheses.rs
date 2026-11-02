use super::*;

fn change(claim: &str, body: &str, family: Family, known: i64) -> Change {
    Change {
        claim: claim.into(),
        body: body.into(),
        family,
        properties: BTreeMap::from([
            ("summary".into(), body.into()),
            ("notes".into(), "separate property baseline".into()),
        ]),
        known,
        valid_from: 0,
        valid_to: None,
        intent: Intent::Revision,
        source: "source".into(),
        endpoints: None,
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
        for (id, body, known) in
            [("a", "alpha", 100), ("b", "beta beta extra", 101), ("c", "gamma filler", 102)]
        {
            clean.put(change(id, body, family, known)).unwrap();
        }
        for known in 1..30 {
            history
                .put(change("a", &"alpha distant old content ".repeat(5), family, known))
                .unwrap();
        }
        for (id, body, known) in
            [("a", "alpha", 100), ("b", "beta beta extra", 101), ("c", "gamma filler", 102)]
        {
            history.put(change(id, body, family, known)).unwrap();
        }
        for (projection, name) in [(family, None), (Family::Property, Some("summary"))] {
            if family == Family::Edge && projection == Family::Property {
                continue;
            }
            let expected = clean.search("alpha OR beta", projection, name, 10).unwrap();
            assert_eq!(
                expected.len(),
                2,
                "current matches must exist before invariance is meaningful"
            );
            let actual = history.search("alpha OR beta", projection, name, 10).unwrap();
            assert_eq!(identity_scores(&actual), identity_scores(&expected));
            assert_eq!(history.search("distant", projection, name, 10).unwrap().len(), 0);
        }
    }
}

fn at(valid: i64, known: Option<i64>) -> View {
    View::Snapshot { valid, known }
}
fn bodies(store: &Store, valid: i64, known: Option<i64>) -> Vec<String> {
    store
        .search_view("Paris OR London OR Lyon", Family::Node, None, 20, at(valid, known))
        .unwrap()
        .into_iter()
        .map(|h| h.body)
        .collect()
}

#[test]
fn h2_late_world_change_preserves_previous_truth_and_prior_belief() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("world.sqlite"), 80).unwrap();
    let mut original = change("address", "Paris", Family::Node, 10);
    original.intent = Intent::Initial;
    store.put(original).unwrap();
    let mut moved = change("address", "London", Family::Node, 70);
    moved.valid_from = 60;
    moved.intent = Intent::WorldChange;
    store.put(moved).unwrap();
    assert_eq!(bodies(&store, 59, None), ["Paris"]);
    assert_eq!(bodies(&store, 60, None), ["London"]);
    assert_eq!(
        bodies(&store, 65, Some(69)),
        ["Paris"],
        "late knowledge is not the world change time"
    );
    assert_eq!(bodies(&store, 65, Some(70)), ["London"]);
}

#[test]
fn h2_partial_correction_and_nested_retraction_do_not_resurrect_errors() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("correction.sqlite"), 40).unwrap();
    let mut original = change("address", "Paris", Family::Node, 10);
    original.valid_to = Some(100);
    original.intent = Intent::Initial;
    store.put(original).unwrap();
    let mut corrected = change("address", "Lyon", Family::Node, 20);
    corrected.valid_from = 30;
    corrected.valid_to = Some(60);
    corrected.intent = Intent::Correction;
    store.put(corrected).unwrap();
    for (valid, expected) in
        [(0, "Paris"), (29, "Paris"), (30, "Lyon"), (59, "Lyon"), (60, "Paris"), (99, "Paris")]
    {
        assert_eq!(bodies(&store, valid, None), [expected]);
    }
    assert!(bodies(&store, 100, None).is_empty());
    assert_eq!(bodies(&store, 40, Some(19)), ["Paris"]);
    assert_eq!(bodies(&store, 40, Some(20)), ["Lyon"]);
    let mut retracted = change("address", "Lyon", Family::Node, 30);
    retracted.valid_from = 35;
    retracted.valid_to = Some(45);
    retracted.intent = Intent::Retraction;
    store.put(retracted).unwrap();
    assert!(
        bodies(&store, 40, None).is_empty(),
        "withdrawal must not fall through to original Paris"
    );
    assert_eq!(bodies(&store, 34, None), ["Lyon"]);
    assert_eq!(bodies(&store, 45, None), ["Lyon"]);
    assert_eq!(bodies(&store, 40, Some(29)), ["Lyon"]);
    let archive = store
        .search_view("Paris", Family::Node, None, 20, View::Archive { include_retracted: true })
        .unwrap();
    assert!(!archive.is_empty(), "audit can retrieve the original incorrect assertion");
}
