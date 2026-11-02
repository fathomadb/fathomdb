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

fn assert_rank_equal(left: &[Hit], right: &[Hit]) {
    assert_eq!(
        left.iter().map(|h| (&h.claim, h.version)).collect::<Vec<_>>(),
        right.iter().map(|h| (&h.claim, h.version)).collect::<Vec<_>>()
    );
    for (a, b) in left.iter().zip(right) {
        assert!((a.score - b.score).abs() < 1e-12, "score mismatch: {a:?} / {b:?}");
    }
}

#[test]
fn h4_rankers_share_selected_statistics_not_filtered_history_statistics() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("ranks.sqlite"), 50).unwrap();
    for known in 1..12 {
        store
            .put(change("a", &"alpha oldword ".repeat(known as usize), Family::Node, known))
            .unwrap();
    }
    for (id, body, known) in [("a", "alpha", 20), ("b", "beta beta extra", 21), ("c", "gamma", 22)]
    {
        store.put(change(id, body, Family::Node, known)).unwrap();
    }
    for view in [at(50, None), at(50, Some(5)), View::Archive { include_retracted: true }] {
        for (family, name) in [(Family::Node, None), (Family::Property, Some("summary"))] {
            for query in ["alpha", "alpha OR beta", "oldword"] {
                let expected = store
                    .search_backend(query, family, name, 10, view, Backend::Materialized)
                    .unwrap();
                for backend in [Backend::Partitioned, Backend::Custom] {
                    let actual =
                        store.search_backend(query, family, name, 10, view, backend).unwrap();
                    assert_rank_equal(&actual, &expected);
                }
            }
        }
    }
    let selected = documents(&store.conn, Family::Node, 50, None).unwrap();
    let leaky =
        native_search(&store.conn, "all_nodes", "alpha OR beta", &selected, None, 10).unwrap();
    let isolated = store
        .search_backend(
            "alpha OR beta",
            Family::Node,
            None,
            10,
            at(50, None),
            Backend::Materialized,
        )
        .unwrap();
    assert_ne!(
        identity_scores(&leaky),
        identity_scores(&isolated),
        "control must detect history's statistical influence"
    );
}

#[test]
fn h4_native_parity_covers_stems_empty_docs_idf_floor_ties_and_property_names() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("tokens.sqlite"), 50).unwrap();
    for (id, body, known) in [
        ("a", "alpha", 1),
        ("b", "alpha alpha beta", 2),
        ("c", "running café", 3),
        ("d", "!!!", 4),
        ("e", "tie", 5),
        ("f", "tie", 6),
    ] {
        let mut value = change(id, body, Family::Node, known);
        value.properties.insert(
            "notes".into(),
            "run cafe alpha alpha lots of nonmatching-name statistics".into(),
        );
        store.put(value).unwrap();
    }
    for query in ["alpha", "run", "running", "run OR running", "cafe", "tie", "alpha OR beta"] {
        for (family, name) in [(Family::Node, None), (Family::Property, Some("summary"))] {
            let expected = store
                .search_backend(query, family, name, 20, at(50, None), Backend::Materialized)
                .unwrap();
            assert!(!expected.is_empty());
            for backend in [Backend::Partitioned, Backend::Custom] {
                assert_rank_equal(
                    &store.search_backend(query, family, name, 20, at(50, None), backend).unwrap(),
                    &expected,
                );
            }
        }
    }
    let once = store
        .search_backend("run", Family::Node, None, 20, at(50, None), Backend::Materialized)
        .unwrap();
    let twice = store
        .search_backend("run OR running", Family::Node, None, 20, at(50, None), Backend::Custom)
        .unwrap();
    assert!((twice[0].score - 2.0 * once[0].score).abs() < 1e-12);
    let ties =
        store.search_backend("tie", Family::Node, None, 1, at(50, None), Backend::Custom).unwrap();
    assert_eq!(ties[0].claim, "e");
    for bad in ["alpha*", "alpha AND beta", "NEAR(alpha beta)", "café"] {
        assert!(store
            .search_backend(bad, Family::Node, None, 20, at(50, None), Backend::Custom)
            .is_err());
    }
}

#[test]
fn h4_hand_computed_common_term_score_counts_zero_token_documents() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("idf.sqlite"), 50).unwrap();
    for (id, body, known) in
        [("a", "alpha", 1), ("b", "alpha alpha beta", 2), ("c", "running café", 3), ("d", "!!!", 4)]
    {
        store.put(change(id, body, Family::Node, known)).unwrap();
    }
    let expected = -1e-6 * 2.2 / 1.9;
    for backend in [Backend::Partitioned, Backend::Custom, Backend::Materialized] {
        let hits =
            store.search_backend("alpha", Family::Node, None, 20, at(50, None), backend).unwrap();
        let actual = hits.iter().find(|h| h.claim == "a").unwrap().score;
        assert!(
            (actual - expected).abs() < 1e-15,
            "N=4, avgdl=1.5, IDF floor: {actual} vs {expected}"
        );
    }
}

#[test]
fn h5_clock_only_boundary_invalidates_native_partition_but_certified_fallback_is_correct() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("clock.sqlite"), 10).unwrap();
    let mut old = change("expires", "alpha alpha alpha", Family::Node, 1);
    old.valid_to = Some(20);
    store.put(old).unwrap();
    store.put(change("stays", "alpha", Family::Node, 2)).unwrap();
    let mut future = change("future", "beta", Family::Node, 3);
    future.valid_from = 20;
    store.put(future).unwrap();
    let docs = documents(&store.conn, Family::Node, 20, None).unwrap();
    let naive =
        native_search(&store.conn, "current_nodes", "alpha OR beta", &docs, None, 20).unwrap();
    let expected = store
        .search_backend(
            "alpha OR beta",
            Family::Node,
            None,
            20,
            at(20, None),
            Backend::Materialized,
        )
        .unwrap();
    assert_ne!(
        identity_scores(&naive),
        identity_scores(&expected),
        "negative control: current-only membership can go stale without a write"
    );
    let actual = store
        .search_backend("alpha OR beta", Family::Node, None, 20, at(20, None), Backend::Partitioned)
        .unwrap();
    assert_rank_equal(&actual, &expected);
    assert_eq!(
        actual.iter().map(|h| h.claim.as_str()).collect::<std::collections::BTreeSet<_>>(),
        ["future", "stays"].into_iter().collect()
    );
}
