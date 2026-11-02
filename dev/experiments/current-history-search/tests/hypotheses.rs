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

fn edge_change(
    id: &str,
    body: &str,
    from: &str,
    to: &str,
    known: i64,
    valid_from: i64,
    valid_to: Option<i64>,
) -> Change {
    let mut c = change(id, body, Family::Edge, known);
    c.endpoints = Some((from.into(), to.into()));
    c.valid_from = valid_from;
    c.valid_to = valid_to;
    c
}

#[test]
fn h3_graph_never_combines_noncoexisting_edges_and_resolves_both_clocks() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("graph.sqlite"), 15).unwrap();
    for (id, known) in [("A", 1), ("B", 2), ("C", 3)] {
        store.put(change(id, id, Family::Node, known)).unwrap();
    }
    store.put(edge_change("ab", "connect", "A", "B", 4, 0, Some(20))).unwrap();
    store.put(edge_change("bc", "connect", "B", "C", 5, 30, None)).unwrap();
    assert_eq!(store.graph("A", at(15, None), 3, 10).unwrap(), [("B".into(), "B".into(), 1)]);
    assert!(store.graph("A", at(35, None), 3, 10).unwrap().is_empty());
    store.put(edge_change("bc-window", "connect", "B", "C", 6, 10, Some(18))).unwrap();
    assert_eq!(
        store.graph("A", at(15, None), 3, 10).unwrap(),
        [("B".into(), "B".into(), 1), ("C".into(), "C".into(), 2)]
    );
    assert_eq!(store.graph("A", at(18, None), 3, 10).unwrap().len(), 1);
    let mut correction = change("B", "B corrected", Family::Node, 7);
    correction.intent = Intent::Correction;
    store.put(correction).unwrap();
    assert_eq!(store.graph("A", at(15, None), 3, 10).unwrap()[0].1, "B corrected");
    assert_eq!(store.graph("A", at(15, Some(6)), 3, 10).unwrap()[0].1, "B");
    assert!(store.graph("A", View::Archive { include_retracted: true }, 3, 10).is_err());
}

#[test]
fn h1_h3_revision_multiplicity_cannot_consume_default_seed_or_graph_slots() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("seeds.sqlite"), 50).unwrap();
    for (id, known) in [("A", 1), ("B", 2), ("C", 3)] {
        store.put(change(id, id, Family::Node, known)).unwrap();
    }
    for known in 4..30 {
        store.put(edge_change("ab", "alpha oldword oldword", "A", "B", known, 0, None)).unwrap();
    }
    store.put(edge_change("ab", "alpha", "A", "B", 30, 0, None)).unwrap();
    store.put(edge_change("bc", "beta", "B", "C", 31, 0, None)).unwrap();
    let before = store.lexical_seeds("alpha OR beta", at(50, None), 3).unwrap();
    assert_eq!(before.len(), 3, "unique endpoints are capped after grouping");
    let ranked = identity_scores(
        &store
            .search_backend(
                "alpha OR beta",
                Family::Edge,
                None,
                1,
                at(50, None),
                Backend::Partitioned,
            )
            .unwrap(),
    );
    for known in 32..55 {
        store.put(edge_change("ab", "alpha oldword oldword", "A", "B", known, 0, None)).unwrap();
    }
    store.put(edge_change("ab", "alpha", "A", "B", 55, 0, None)).unwrap();
    assert_eq!(store.lexical_seeds("alpha OR beta", at(50, None), 3).unwrap(), before);
    assert_eq!(
        identity_scores(
            &store
                .search_backend(
                    "alpha OR beta",
                    Family::Edge,
                    None,
                    1,
                    at(50, None),
                    Backend::Partitioned
                )
                .unwrap()
        ),
        ranked
    );
    assert_eq!(
        store
            .graph("A", at(50, None), 3, 2)
            .unwrap()
            .iter()
            .map(|t| t.0.as_str())
            .collect::<Vec<_>>(),
        ["B", "C"]
    );
}

#[test]
fn h6_fault_rolls_back_canonical_history_current_and_knowledge_together() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("fault.sqlite"), 50).unwrap();
    store.put(change("a", "alpha", Family::Node, 1)).unwrap();
    let before = store.search("alpha", Family::Node, None, 10).unwrap();
    assert!(store.put_fault(change("a", "beta", Family::Node, 2)).is_err());
    assert_eq!(store.search("alpha", Family::Node, None, 10).unwrap(), before);
    assert!(store
        .search_view("beta", Family::Node, None, 10, View::Archive { include_retracted: true })
        .unwrap()
        .is_empty());
    assert_eq!(
        store.conn.query_row("SELECT known FROM config", [], |r| r.get::<_, i64>(0)).unwrap(),
        1
    );
    assert_eq!(
        store
            .conn
            .query_row("SELECT count(*) FROM assertions", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn h6_source_authority_inactive_erasure_and_rebuild_cover_all_projection_families() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lifecycle.sqlite");
    let mut store = Store::open(&path, 50).unwrap();
    store.put(change("a", "alpha old", Family::Node, 1)).unwrap();
    store.put(change("a", "alpha corrected", Family::Node, 2)).unwrap();
    store.put(edge_change("ab", "alpha", "a", "a", 3, 0, None)).unwrap();
    store.set_active("a", false).unwrap();
    assert!(store.search("alpha", Family::Node, None, 20).unwrap().is_empty());
    store.set_active("a", true).unwrap();
    let before = store.search("alpha", Family::Node, None, 20).unwrap();
    for family in Family::all() {
        store.conn.execute(&format!("DELETE FROM all_{}", family.label()), []).unwrap();
        store.conn.execute(&format!("DELETE FROM current_{}", family.label()), []).unwrap();
    }
    store.rebuild().unwrap();
    assert_eq!(store.search("alpha", Family::Node, None, 20).unwrap(), before);
    for family in Family::all() {
        assert!(
            store
                .conn
                .query_row(&format!("SELECT count(*) FROM all_{}", family.label()), [], |r| r
                    .get::<_, i64>(0))
                .unwrap()
                > 0
        );
    }
    store.configure_source("source", false, true, 0, None).unwrap();
    for family in Family::all() {
        assert!(
            store.search_view("alpha", family, None, 20, at(50, Some(1))).unwrap().is_empty(),
            "past knowledge cannot bypass current authority"
        );
        assert!(store
            .search_view("alpha", family, None, 20, View::Archive { include_retracted: true })
            .unwrap()
            .is_empty());
    }
    store.configure_source("source", true, true, 0, None).unwrap();
    assert!(!store.search("alpha", Family::Node, None, 20).unwrap().is_empty());
    store.erase_source("source").unwrap();
    store.rebuild().unwrap();
    drop(store);
    let reopened = Store::open(&path, 999).unwrap();
    assert_eq!(
        reopened
            .conn
            .query_row("SELECT count(*) FROM assertions", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    for family in Family::all() {
        for prefix in ["all", "current"] {
            assert_eq!(
                reopened
                    .conn
                    .query_row(
                        &format!("SELECT count(*) FROM {prefix}_{}", family.label()),
                        [],
                        |r| r.get::<_, i64>(0)
                    )
                    .unwrap(),
                0
            );
        }
    }
}

#[test]
fn h6_explicit_identity_and_interval_assumptions_fail_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("assumptions.sqlite"), 50).unwrap();
    store.put(change("a", "alpha", Family::Node, 10)).unwrap();
    let mut inverted = change("a", "wrong", Family::Node, 11);
    inverted.valid_to = Some(0);
    let mut different_source = change("a", "wrong", Family::Node, 11);
    different_source.source = "another".into();
    for invalid in [
        change("a", "wrong", Family::Node, 10),
        inverted,
        different_source,
        change("a", "wrong", Family::Edge, 11),
    ] {
        assert!(store.put(invalid).is_err());
    }
    assert_eq!(store.search("alpha", Family::Node, None, 10).unwrap().len(), 1);
    assert_eq!(
        store
            .conn
            .query_row("SELECT count(*) FROM assertions", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn h6_independent_source_claims_do_not_auto_supersede_one_another() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("sources.sqlite"), 50).unwrap();
    store.configure_source("second", true, true, 0, None).unwrap();
    store.put(change("claim-source-a", "Paris", Family::Node, 1)).unwrap();
    let mut other = change("claim-source-b", "Lyon", Family::Node, 2);
    other.source = "second".into();
    store.put(other).unwrap();
    assert_eq!(
        bodies(&store, 50, None).len(),
        2,
        "temporal coherence does not decide conflicting source truth"
    );
}

#[test]
fn h8_wal_reader_keeps_one_snapshot_across_replacement_and_source_denial() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wal.sqlite");
    let mut writer = Store::open(&path, 50).unwrap();
    writer.put(change("a", "alpha", Family::Node, 1)).unwrap();
    let reader = Store::open(&path, 50).unwrap();
    let tx = reader.conn.unchecked_transaction().unwrap();
    let before =
        search_on(&tx, "alpha", Family::Node, None, 10, at(50, None), Backend::Partitioned)
            .unwrap();
    writer.put(change("a", "beta", Family::Node, 2)).unwrap();
    writer.configure_source("source", false, true, 0, None).unwrap();
    for backend in [Backend::Partitioned, Backend::Custom, Backend::Materialized] {
        assert_rank_equal(
            &search_on(&tx, "alpha", Family::Node, None, 10, at(50, None), backend).unwrap(),
            &before,
        );
    }
    tx.commit().unwrap();
    assert!(
        reader
            .search_view("alpha OR beta", Family::Node, None, 10, at(50, Some(1)))
            .unwrap()
            .is_empty(),
        "new request observes source denial"
    );
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(24))]
    #[test]
    fn h6_generated_interval_overlays_preserve_nonoverlap_rebuild_and_reopen(updates in proptest::collection::vec((0i64..90,1i64..15),1..8)) {
        let dir=tempfile::tempdir().unwrap();
        let path=dir.path().join("property.sqlite");
        let mut store=Store::open(&path,50).unwrap();
        store.put(change("a","alpha",Family::Node,1)).unwrap();
        for (offset,(start,length)) in updates.into_iter().enumerate() {
            let mut c=change("a","beta",Family::Node,offset as i64+2);
            c.valid_from=start; c.valid_to=Some(start+length); c.intent=Intent::Correction;
            store.put(c).unwrap();
        }
        let overlaps=store.conn.query_row("SELECT count(*) FROM assertions a JOIN assertions b ON a.claim=b.claim AND a.id<b.id
            WHERE a.kt IS NULL AND b.kt IS NULL AND (a.vt IS NULL OR b.vf<a.vt) AND (b.vt IS NULL OR a.vf<b.vt)",[],|r|r.get::<_,i64>(0)).unwrap();
        proptest::prop_assert_eq!(overlaps,0);
        let before=store.search("alpha OR beta",Family::Node,None,20).unwrap();
        store.rebuild().unwrap();
        proptest::prop_assert_eq!(store.search("alpha OR beta",Family::Node,None,20).unwrap(),before.clone());
        drop(store);
        let reopened=Store::open(&path,1000).unwrap();
        proptest::prop_assert_eq!(reopened.search("alpha OR beta",Family::Node,None,20).unwrap(),before);
    }
}

#[test]
fn review_current_claim_admission_consistently_gates_every_knowledge_view() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("admission.sqlite"), 50).unwrap();
    store.put(change("a", "alpha", Family::Node, 1)).unwrap();
    store.put(change("a", "beta", Family::Node, 2)).unwrap();
    store.set_active("a", false).unwrap();
    assert!(
        store.search_view("alpha", Family::Node, None, 10, at(50, Some(1))).unwrap().is_empty(),
        "current admission must gate superseded rows too"
    );
    assert!(store
        .search_view(
            "alpha OR beta",
            Family::Node,
            None,
            10,
            View::Archive { include_retracted: true }
        )
        .unwrap()
        .is_empty());
    store.put(change("a", "gamma", Family::Node, 3)).unwrap();
    assert!(
        store.search("gamma", Family::Node, None, 10).unwrap().is_empty(),
        "new versions do not reset current admission"
    );
    store.set_active("a", true).unwrap();
    assert_eq!(
        store.search_view("alpha", Family::Node, None, 10, at(50, Some(1))).unwrap().len(),
        1
    );
}

#[test]
fn review_initial_and_structural_revision_cannot_rewrite_validity_silently() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("intent.sqlite"), 50).unwrap();
    store.put(change("a", "alpha", Family::Node, 1)).unwrap();
    let mut duplicate_initial = change("a", "beta", Family::Node, 2);
    duplicate_initial.intent = Intent::Initial;
    assert!(store.put(duplicate_initial).is_err(), "initial cannot supersede an existing claim");
    let mut shifted_revision = change("a", "beta", Family::Node, 2);
    shifted_revision.valid_from = 20;
    assert!(store.put(shifted_revision).is_err(), "structural revision must preserve the interval");
    assert_eq!(store.search("alpha", Family::Node, None, 10).unwrap().len(), 1);
}

#[test]
fn review_ineligible_endpoint_cannot_contribute_lexical_graph_seeds() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("endpoint.sqlite"), 50).unwrap();
    store.put(change("A", "A", Family::Node, 1)).unwrap();
    let mut b = change("B", "B", Family::Node, 2);
    b.valid_to = Some(40);
    store.put(b).unwrap();
    store.put(edge_change("ab", "needle", "A", "B", 3, 0, None)).unwrap();
    assert!(
        store.lexical_seeds("needle", at(50, None), 10).unwrap().is_empty(),
        "an edge with no coherent endpoint pair cannot seed graph retrieval"
    );
    assert!(store.graph("A", at(50, None), 2, 10).unwrap().is_empty());
}

#[test]
fn review_archive_applies_source_validity_at_its_explicit_authority_instant() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("archive-authority.sqlite"), 50).unwrap();
    store.put(change("a", "alpha", Family::Node, 1)).unwrap();
    store.configure_source("source", true, true, 0, Some(50)).unwrap();
    assert!(
        store
            .search_view("alpha", Family::Node, None, 10, View::Archive { include_retracted: true })
            .unwrap()
            .is_empty(),
        "source's half-open authority interval ends at archive now"
    );
}
