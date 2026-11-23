use crate::{
    fixtures,
    policy::Policy,
    session::Session,
    storage::{graph::Direction, Class, Filter, Reader},
};
use fathomdb_engine::{EmbedderChoice, Engine};
fn handle(r: &mut Reader, class: Class, id: &str, last: bool) -> String {
    let page =
        r.page(class, 0, 200, Filter { identity: Some(id.into()), ..Default::default() }).unwrap();
    let items = page["items"].as_array().unwrap();
    items[if last { items.len() - 1 } else { 0 }]["handle"].as_str().unwrap().into()
}
#[test]
fn expanded_hidden_revision_is_reclassified_before_any_adjacency_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    fixtures::add_graph(&path);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let root = handle(&mut r, Class::Node, "derived", true);
    let hidden = handle(&mut r, Class::Node, "alpha", false);
    r.session.policy.no_show_inactive = true;
    let result = r.graph(&root, Direction::Both, None, 3, vec![root.clone(), hidden]);
    assert_eq!(result,Err("refused"),"an old exact expansion handle must be classified at each read, even when its logical identity has a current revision");
    r.session.close().unwrap();
}

#[test]
fn graph_and_history_obey_all_eight_disclosure_combinations_and_fresh_withdrawal() {
    use fathomdb_engine::{
        EngineConfig, PhysicalErasurePolicy, PhysicalErasureSelector, QueueResult,
    };
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.sqlite");
    let engine = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::None,
        EngineConfig {
            physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
            ..Default::default()
        },
    )
    .unwrap();
    engine.engine.close().unwrap();
    drop(engine);
    fixtures::database(dir.path());
    fixtures::add_graph(&path);
    let engine = Arc::new(Engine::open_with_choice(&path, EmbedderChoice::None).unwrap().engine);
    let mut maintenance = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = match maintenance
        .queue_physical_erasure(
            PhysicalErasureSelector::SourceAllAtCut { source_id: "fixture:canonical".into() },
            "0123456789abcdef0123456789abcdef",
        )
        .unwrap()
    {
        QueueResult::Accepted { ticket, .. } => ticket,
        other => panic!("{other:?}"),
    };
    maintenance.close().unwrap();
    drop(maintenance);
    engine.close().unwrap();
    drop(engine);
    for n in [false, true] {
        for e in [false, true] {
            for t in [false, true] {
                let mut r = Reader::new(
                    Session::attach(&path, Policy { show_erased: true, ..Default::default() })
                        .unwrap(),
                );
                let current = handle(&mut r, Class::Node, "derived", true);
                let old = handle(&mut r, Class::Node, "derived", false);
                r.session.policy = Policy { no_show_inactive: n, show_erased: e, show_payloads: t };
                for (h, expected) in [(&current, e), (&old, e && !n)] {
                    assert_eq!(
                        r.graph(h, Direction::Both, None, 3, vec![h.clone()]).is_ok(),
                        expected,
                        "graph D01 n={n} e={e} t={t}"
                    );
                    assert_eq!(r.investigate(h).is_ok(), expected, "history D01 n={n} e={e} t={t}");
                    assert_eq!(r.chunk(h, "body", 0, 1024).is_ok(), expected);
                    let output = dir.path().join(format!("retained-{n}-{e}-{t}-{}", h));
                    assert_eq!(
                        r.export_to(h, &output).is_ok(),
                        expected,
                        "history-owned narrow export uses independent gates"
                    );
                }
                if e {
                    let graph =
                        r.graph(&current, Direction::Both, None, 3, vec![current.clone()]).unwrap();
                    assert!(graph["nodes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|v| !n || v["inactive_or_history"] == false));
                    assert!(r
                        .graph_serving(&current, Direction::Outgoing, Some("supports".into()), 1)
                        .is_ok());
                } else {
                    assert_eq!(
                        r.graph_serving(&current, Direction::Outgoing, None, 1),
                        Err("refused")
                    );
                }
                r.session.close().unwrap();
            }
        }
    }
    let mut r = Reader::new(
        Session::attach(&path, Policy { show_erased: true, ..Default::default() }).unwrap(),
    );
    let current = handle(&mut r, Class::Node, "derived", true);
    r.session.policy = Policy::default();
    assert_eq!(r.investigate(&current), Err("refused"));
    r.session.engine.withdraw_queued_erasure(&ticket).unwrap();
    assert!(r.investigate(&current).is_ok());
    assert!(r.graph(&current, Direction::Outgoing, None, 1, vec![current.clone()]).is_ok());
    r.session.close().unwrap();
    // Current Engine refuses a dangling provenanced source. Erase the legitimate
    // complete source cut rather than manufacturing an orphan for the reader.
    let engine = Arc::new(Engine::open_with_choice(&path, EmbedderChoice::None).unwrap().engine);
    let mut maintenance = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = match maintenance
        .queue_physical_erasure(
            PhysicalErasureSelector::SourceAllAtCut { source_id: "fixture:canonical".into() },
            "1123456789abcdef0123456789abcdef",
        )
        .unwrap()
    {
        QueueResult::Accepted { ticket, .. } => ticket,
        other => panic!("{other:?}"),
    };
    assert!(matches!(
        maintenance.complete_physical_erasure(&ticket).unwrap(),
        fathomdb_engine::CleanupResult::Complete { .. }
    ));
    maintenance.close().unwrap();
    drop(maintenance);
    engine.close().unwrap();
    drop(engine);
    for n in [false, true] {
        for e in [false, true] {
            for t in [false, true] {
                let mut r = Reader::new(
                    Session::attach(
                        &path,
                        Policy { no_show_inactive: n, show_erased: e, show_payloads: t },
                    )
                    .unwrap(),
                );
                let page = r
                    .page(
                        Class::Node,
                        0,
                        20,
                        Filter { identity: Some("derived".into()), ..Default::default() },
                    )
                    .unwrap();
                assert!(
                    page["items"].as_array().unwrap().is_empty(),
                    "physically erased bytes are unavailable under every flag"
                );
                r.session.close().unwrap();
            }
        }
    }
}

#[test]
fn native_graph_admission_refuses_repeated_large_source_and_large_target_before_hydration() {
    use fathomdb_engine::{
        ArtifactRevisionId, CanonicalHash, SourceLocator, SourceRevisionId, SourceVersionId,
        WriteProvenanceV1,
    };
    use sha2::{Digest, Sha256};
    for source_large in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let path = fixtures::database(dir.path());
        let source = if source_large { "s".repeat(4 * 1024 * 1024) } else { "small source".into() };
        let digest: String =
            Sha256::digest(source.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
        let derived = |id: &str| {
            WriteProvenanceV1::derived(
                ArtifactRevisionId::new(id).unwrap(),
                SourceVersionId::new("large-v1").unwrap(),
                SourceRevisionId::new("large-source-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest.clone()).unwrap(),
            )
        };
        let node = |id: &str, body: String, provenance| {
            fixtures::provenanced_node(Some(id), &body, "fixture:large", None, provenance)
        };
        let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
        opened
            .engine
            .write(&[node(
                "large-source",
                source,
                WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("large-source-r1").unwrap(),
                    SourceVersionId::new("large-v1").unwrap(),
                ),
            )])
            .unwrap();
        opened
            .engine
            .write(&[
                node("large-root", "root".into(), derived("large-root-r1")),
                node(
                    "large-target",
                    if source_large { "small target".into() } else { "t".repeat(4 * 1024 * 1024) },
                    derived("large-target-r1"),
                ),
                fixtures::provenanced_edge(
                    "supports",
                    "large-root",
                    "large-target",
                    "fixture:large",
                    Some("large-edge"),
                    None,
                    None,
                    None,
                    None,
                    derived("large-edge-r1"),
                ),
            ])
            .unwrap();
        opened.engine.close().unwrap();
        drop(opened);
        let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
        let root = handle(&mut r, Class::Node, "large-root", true);
        assert_eq!(
            r.graph_serving(&root, Direction::Outgoing, None, 1),
            Err("budget_exhausted"),
            "source_large={source_large}"
        );
        assert_eq!(r.detail(&root).unwrap()["content"]["text"], "root");
        r.session.close().unwrap();
    }
}

proptest::proptest! {
 #![proptest_config(proptest::test_runner::Config::with_cases(12))]
 #[test]
 fn directed_identity_and_bounds_match_independently_authored_edges(edges in proptest::collection::vec((0usize..4,0usize..4,0usize..3),0..30), depth in 1usize..4){
    let dir=tempfile::tempdir().unwrap();let path=fixtures::database(dir.path());let opened=Engine::open_with_choice(&path,EmbedderChoice::None).unwrap();
    opened.engine.write(&(0..4).map(|i|fixtures::node(&format!("p{i}"),"equal label/body")).collect::<Vec<_>>()).unwrap();
    let writes = edges
        .iter()
        .enumerate()
        .map(|(i, (from, to, kind))| {
            fixtures::edge(
                &format!("k{kind}"),
                &format!("p{from}"),
                &format!("p{to}"),
                "fixture:property",
                Some(&format!("e{i}")),
                Some("edge body".into()),
            )
        })
        .collect::<Vec<_>>();
    if !writes.is_empty(){opened.engine.write(&writes).unwrap();}opened.engine.close().unwrap();drop(opened);
    let mut r=Reader::new(Session::attach(&path,Policy::default()).unwrap());let h=handle(&mut r,Class::Node,"p0",true);
    for (direction,incoming,outgoing) in [(Direction::Both,true,true),(Direction::Incoming,true,false),(Direction::Outgoing,false,true)]{
      let graph=r.graph(&h,direction,None,depth,vec![h.clone()]).unwrap();
      let actual=graph["edges"].as_array().unwrap().iter().map(|v|v["logical_id"].as_str().unwrap().to_string()).collect::<std::collections::BTreeSet<_>>();
      let expected=edges.iter().enumerate().filter(|(_, (from,to,_))|(incoming&&*to==0)||(outgoing&&*from==0)).map(|(i,_)|format!("e{i}")).collect::<std::collections::BTreeSet<_>>();
      proptest::prop_assert_eq!(actual,expected);proptest::prop_assert!(graph["scope"]["work_units"].as_u64().unwrap()<=1000);
      let keys=graph["edges"].as_array().unwrap().iter().map(|v|v["key"].as_str().unwrap()).collect::<std::collections::BTreeSet<_>>();
      proptest::prop_assert_eq!(keys.len(),graph["edges"].as_array().unwrap().len());
    }
    r.session.close().unwrap();
 }
}

#[test]
fn endpoint_resolution_matches_complete_identity_instead_of_display_preview() {
    for id in ["n".repeat(5000), "🦀".repeat(5000)] {
        let dir = tempfile::tempdir().unwrap();
        let path = fixtures::database(dir.path());
        let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
        opened
            .engine
            .write(&[
                fixtures::node(&id, "long exact ID"),
                fixtures::node(
                    &(id.clone() + "-same-prefix"),
                    "different record sharing the display prefix",
                ),
                fixtures::edge(
                    "long-id",
                    "alpha",
                    &id,
                    "fixture:long",
                    Some("long-endpoint-edge"),
                    None,
                ),
            ])
            .unwrap();
        opened.engine.close().unwrap();
        drop(opened);
        let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
        let h = handle(&mut r, Class::Node, "alpha", true);
        let graph = r.graph(&h, Direction::Outgoing, None, 1, vec![h.clone()]).unwrap();
        assert_eq!(
            graph["edges"][0]["to"]["resolution"], "one",
            "complete stored logical identity must not be compared to its display preview"
        );
        assert_eq!(graph["edges"][0]["to"]["targets"].as_array().unwrap().len(), 1);
        let target = graph["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["key"] == graph["edges"][0]["to"]["targets"][0])
            .unwrap();
        let exact = r.chunk(target["handle"].as_str().unwrap(), "logical_id", 0, 65536).unwrap();
        assert_eq!(exact["text"], id);
        r.session.close().unwrap();
    }
}

#[test]
fn distinct_missing_endpoint_identities_keep_distinct_response_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    fixtures::add_graph_missing_prefix(&path);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let h = handle(&mut r, Class::Edge, "prefix-edge", true);
    let graph = r.graph(&h, Direction::Both, None, 1, vec![]).unwrap();
    let edge = &graph["edges"][0];
    assert_eq!(edge["from"]["identity"], edge["to"]["identity"], "previews deliberately collide");
    assert_ne!(
        edge["from"]["key"], edge["to"]["key"],
        "different complete missing endpoint identities require distinct response-local keys"
    );
    assert_eq!(edge["from"]["resolution"], "zero");
    assert_eq!(edge["to"]["resolution"], "zero");
    r.session.close().unwrap();
}

#[test]
fn investigation_charges_selected_record_metadata_to_the_shared_response_budget() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    let large = "🦀".repeat(3800);
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    for n in 0..35 {
        let body = format!("revision {n}");
        opened
            .engine
            .write(&[fixtures::node_with(
                &large,
                Some(&large),
                &body,
                &large,
                fathomdb_engine::InitialState::Pending,
                Some(&large),
                None,
                None,
            )])
            .unwrap();
    }
    opened.engine.close().unwrap();
    drop(opened);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let h = handle(&mut r, Class::Node, &large, true);
    let result = r.investigate(&h).unwrap();
    let bytes = serde_json::to_vec(&result).unwrap().len();
    println!(
        "history bytes={bytes} root={} revisions={} sample={}",
        serde_json::to_vec(&result["record"]).unwrap().len(),
        result["revisions"].as_array().unwrap().len(),
        serde_json::to_vec(&result["revisions"][0]).unwrap().len()
    );
    assert!(
        bytes <= 2 * 1024 * 1024,
        "selected record metadata plus history must share2MiB, got {bytes}"
    );
    assert_eq!(result["status"], "partial");
    assert_eq!(result["scope"]["history_complete"], false);
    assert!(r.detail(&h).is_ok());
    r.session.close().unwrap();
}

#[test]
fn partial_hub_expansion_keeps_its_exact_target_and_can_collapse_back_to_seed() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    fixtures::add_ux_hub(&path);
    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let seed = handle(&mut reader, Class::Node, "ux-seed", true);
    let hub = handle(&mut reader, Class::Node, "ux-hub", true);
    let initial = reader.graph(&seed, Direction::Both, None, 3, vec![seed.clone()]).unwrap();
    assert!(initial["nodes"].as_array().unwrap().iter().any(|v| v["logical_id"] == "ux-hub"));
    let expanded = reader.graph(&seed, Direction::Both, None, 3, vec![seed.clone(), hub]).unwrap();
    assert_eq!(expanded["status"], "partial");
    assert!(
        expanded["nodes"].as_array().unwrap().iter().any(|v| v["logical_id"] == "ux-hub"),
        "partial expansion must retain its admitted exact owner, not silently fall back to seed"
    );
    assert!(
        expanded["nodes"].as_array().unwrap().len() + expanded["edges"].as_array().unwrap().len()
            <= 200
    );
    let collapsed = reader.graph(&seed, Direction::Both, None, 3, vec![seed.clone()]).unwrap();
    assert_eq!(collapsed["edges"].as_array().unwrap().len(), 1);
    reader.session.close().unwrap();
}

#[test]
fn collapsing_seed_preserves_edges_owned_by_an_independently_expanded_hub() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    fixtures::add_ux_hub(&path);
    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let seed = handle(&mut reader, Class::Node, "ux-seed", true);
    let hub = handle(&mut reader, Class::Node, "ux-hub", true);
    let _ = reader.graph(&seed, Direction::Both, None, 3, vec![seed.clone(), hub.clone()]).unwrap();
    let collapsed = reader.graph(&seed, Direction::Both, None, 3, vec![hub]).unwrap();
    assert!(
        !collapsed["edges"].as_array().unwrap().is_empty(),
        "collapse must retain the independently expanded hub's recorded edges"
    );
    assert!(
        collapsed["nodes"].as_array().unwrap().len() + collapsed["edges"].as_array().unwrap().len()
            <= 200
    );
    reader.session.close().unwrap();
}

#[test]
fn provenance_partial_utf8_spans_and_negative_boundaries_distinguish_raw_links_from_native_eligibility(
) {
    use fathomdb_engine::{
        ArtifactRevisionId, CanonicalHash, SourceDependencyRegistrationV1, SourceLocator,
        SourceRevisionId, SourceVersionId, WriteProvenanceV1,
    };
    use sha2::{Digest, Sha256};

    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    let source_body = "Café résumé 🦀 alpha excerpt";
    let whole_digest: String =
        Sha256::digest(source_body.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    let cafe_end = "Café".len() as u64; // 5 bytes ('é' is 2 bytes: offsets 3..5)

    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    let make_derived = |id: &str, rev: &str, body: &str, locator: SourceLocator| {
        fixtures::provenanced_node(
            Some(id),
            body,
            "fixture:prov",
            None,
            WriteProvenanceV1::derived(
                ArtifactRevisionId::new(rev).unwrap(),
                SourceVersionId::new("prov-v1").unwrap(),
                SourceRevisionId::new("prov-source-r1").unwrap(),
                locator,
                CanonicalHash::sha256(whole_digest.clone()).unwrap(),
            ),
        )
    };
    let make_edge = |kind: &str, edge_id: &str, target_id: &str| {
        fixtures::provenanced_edge(
            kind,
            "prov-hub",
            target_id,
            "fixture:prov",
            Some(edge_id),
            Some(format!("edge to {target_id}")),
            Some(0.9),
            Some("extractor-v1".into()),
            Some(false),
            WriteProvenanceV1::derived(
                ArtifactRevisionId::new(format!("{edge_id}-r1")).unwrap(),
                SourceVersionId::new("prov-v1").unwrap(),
                SourceRevisionId::new("prov-source-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(whole_digest.clone()).unwrap(),
            ),
        )
    };

    opened
        .engine
        .write(&[
            fixtures::provenanced_node(
                Some("prov-source"),
                source_body,
                "fixture:prov",
                None,
                WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("prov-source-r1").unwrap(),
                    SourceVersionId::new("prov-v1").unwrap(),
                ),
            ),
            fixtures::provenanced_node(
                Some("prov-hub"),
                "hub node",
                "fixture:prov",
                None,
                WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("prov-hub-r1").unwrap(),
                    SourceVersionId::new("prov-hub-v1").unwrap(),
                ),
            ),
            make_derived(
                "derived-whole",
                "derived-whole-r1",
                "provsentinel whole derivation",
                SourceLocator::whole_body(),
            ),
            make_derived(
                "derived-excerpt",
                "derived-excerpt-r1",
                "provsentinel partial utf8 derivation",
                SourceLocator::utf8_bytes(0, cafe_end),
            ),
            make_derived(
                "derived-empty",
                "derived-empty-r1",
                "provsentinel empty span derivation",
                SourceLocator::utf8_bytes(cafe_end, cafe_end),
            ),
            make_derived(
                "derived-bad-digest",
                "derived-bad-digest-r1",
                "provsentinel bad digest derivation",
                SourceLocator::utf8_bytes(0, cafe_end),
            ),
            make_derived(
                "derived-mid-utf8",
                "derived-mid-utf8-r1",
                "provsentinel mid utf8 boundary derivation",
                SourceLocator::utf8_bytes(0, cafe_end),
            ),
            make_edge("cites", "edge-whole", "derived-whole"),
            make_edge("cites", "edge-excerpt", "derived-excerpt"),
            make_edge("cites", "edge-empty", "derived-empty"),
            make_edge("cites_fault", "edge-bad-digest", "derived-bad-digest"),
            make_edge("cites_fault", "edge-mid-utf8", "derived-mid-utf8"),
        ])
        .unwrap();

    for (dep, rev) in [
        ("dep-whole", "derived-whole-r1"),
        ("dep-excerpt", "derived-excerpt-r1"),
        ("dep-empty", "derived-empty-r1"),
        ("dep-bad-digest", "derived-bad-digest-r1"),
        ("dep-mid-utf8", "derived-mid-utf8-r1"),
    ] {
        opened
            .engine
            .register_source_dependency(
                SourceDependencyRegistrationV1::new(dep, "prov-source-r1", rev).unwrap(),
            )
            .unwrap();
    }
    opened.engine.close().unwrap();

    // Introduce stored negative-boundary faults on the two fault-specific links.
    let c = rusqlite::Connection::open(&path).unwrap();
    let bad_digest = "0".repeat(64);
    c.execute(
        "UPDATE _fathomdb_source_links SET hash_digest = ?1 WHERE artifact_revision_id = 'derived-bad-digest-r1'",
        [&bad_digest],
    )
    .unwrap();
    // Byte offset 4 lands inside the 2-byte UTF-8 sequence for 'é' (bytes 3..5).
    c.execute(
        "UPDATE _fathomdb_source_links SET end_byte = 4 WHERE artifact_revision_id = 'derived-mid-utf8-r1'",
        [],
    )
    .unwrap();
    drop(c);

    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());

    // 1. Both recorded and registered-dependency directions work on partial UTF-8 and whole-body derivations.
    let whole_h = handle(&mut r, Class::Node, "derived-whole", true);
    let whole_inv = r.investigate(&whole_h).unwrap();
    let whole_links = whole_inv["links"].as_array().unwrap();
    let whole_src = whole_links.iter().find(|v| v["kind"] == "recorded_source").unwrap();
    assert_eq!(whole_src["target"]["revision_id"], "prov-source-r1");
    assert_eq!(whole_src["metadata"]["locator_kind"], "whole_body");
    assert_eq!(whole_src["metadata"]["start_byte"], serde_json::Value::Null);
    assert_eq!(whole_src["metadata"]["end_byte"], serde_json::Value::Null);
    assert!(whole_links.iter().any(|v| {
        v["kind"] == "registered_dependency_to_source"
            && v["metadata"]["dependency_id"] == "dep-whole"
            && v["metadata"]["locator_kind"] == "whole_body"
    }));

    let cafe_end_str = cafe_end.to_string();
    let excerpt_h = handle(&mut r, Class::Node, "derived-excerpt", true);
    let excerpt_inv = r.investigate(&excerpt_h).unwrap();
    let excerpt_links = excerpt_inv["links"].as_array().unwrap();
    let src_link = excerpt_links.iter().find(|v| v["kind"] == "recorded_source").unwrap();
    assert_eq!(src_link["target"]["revision_id"], "prov-source-r1");
    assert_eq!(src_link["metadata"]["locator_kind"], "utf8_bytes");
    assert_eq!(src_link["metadata"]["start_byte"], "0");
    assert_eq!(src_link["metadata"]["end_byte"], cafe_end_str);
    assert!(excerpt_links.iter().any(|v| {
        v["kind"] == "registered_dependency_to_source"
            && v["metadata"]["dependency_id"] == "dep-excerpt"
            && v["metadata"]["locator_kind"] == "utf8_bytes"
            && v["metadata"]["start_byte"] == "0"
            && v["metadata"]["end_byte"] == cafe_end_str.as_str()
    }));

    let source_h = handle(&mut r, Class::Node, "prov-source", true);
    let source_inv = r.investigate(&source_h).unwrap();
    let source_links = source_inv["links"].as_array().unwrap();
    for (dep, rev) in [
        ("dep-whole", "derived-whole-r1"),
        ("dep-excerpt", "derived-excerpt-r1"),
        ("dep-empty", "derived-empty-r1"),
        ("dep-bad-digest", "derived-bad-digest-r1"),
        ("dep-mid-utf8", "derived-mid-utf8-r1"),
    ] {
        assert!(
            source_links
                .iter()
                .any(|v| v["kind"] == "recorded_derived" && v["target"]["revision_id"] == rev),
            "raw recorded_derived link must remain visible for {rev}"
        );
        assert!(
            source_links.iter().any(|v| {
                v["kind"] == "registered_dependency_to_dependents"
                    && v["target"]["revision_id"] == rev
                    && v["metadata"]["dependency_id"] == dep
            }),
            "reverse registered_dependency_to_dependents link must remain visible for ({dep}, {rev})"
        );
    }
    let excerpt_dep_link = source_links
        .iter()
        .find(|v| {
            v["kind"] == "registered_dependency_to_dependents"
                && v["target"]["revision_id"] == "derived-excerpt-r1"
        })
        .unwrap();
    assert_eq!(excerpt_dep_link["metadata"]["locator_kind"], "utf8_bytes");
    assert_eq!(excerpt_dep_link["metadata"]["start_byte"], "0");
    assert_eq!(excerpt_dep_link["metadata"]["end_byte"], cafe_end.to_string());
    let whole_dep_link = source_links
        .iter()
        .find(|v| {
            v["kind"] == "registered_dependency_to_dependents"
                && v["target"]["revision_id"] == "derived-whole-r1"
        })
        .unwrap();
    assert_eq!(whole_dep_link["metadata"]["locator_kind"], "whole_body");

    // 2. Native structural dependency evidence via graph_serving (with evidence preflight)
    //    must mark whole_body and valid partial utf8_bytes Registered, reject empty spans
    //    as NotRegistered, and refuse corrupted digest/UTF-8 targets cleanly.
    let hub_h = handle(&mut r, Class::Node, "prov-hub", true);
    let graph_valid =
        r.graph_serving(&hub_h, Direction::Outgoing, Some("cites".into()), 1).unwrap();
    let graph_dep: std::collections::BTreeMap<String, String> = graph_valid["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            (
                item["target"]["logical_id"].as_str().unwrap().to_string(),
                item["evidence"]["dependency"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(graph_dep.get("derived-whole").map(String::as_str), Some("Registered"));
    assert_eq!(graph_dep.get("derived-excerpt").map(String::as_str), Some("Registered"));
    assert_eq!(graph_dep.get("derived-empty").map(String::as_str), Some("NotRegistered"));
    assert_eq!(
        r.graph_serving(&hub_h, Direction::Outgoing, Some("cites_fault".into()), 1),
        Err("native_query_refused"),
        "graph evidence preflight must refuse corrupted digest/UTF-8 links"
    );

    // 3. Native structural dependency evidence via serving (search_frozen explanation ->
    //    structural_state::structural_dependency_state -> source_link_valid) verifies all
    //    five targets side-by-side: whole & valid excerpt are Registered; empty, bad-digest,
    //    and mid-UTF-8 boundaries all evaluate to NotRegistered.
    r.session.cached_cpu = true;
    let search_prov = r.serving("provsentinel", 10).unwrap();
    let search_dep: std::collections::BTreeMap<String, String> = search_prov["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            (
                item["native_id"]["value"].as_str().unwrap().to_string(),
                item["evidence"]["structural"]["dependency_state"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(search_dep.get("derived-whole").map(String::as_str), Some("Registered"));
    assert_eq!(search_dep.get("derived-excerpt").map(String::as_str), Some("Registered"));
    assert_eq!(search_dep.get("derived-empty").map(String::as_str), Some("NotRegistered"));
    assert_eq!(search_dep.get("derived-bad-digest").map(String::as_str), Some("NotRegistered"));
    assert_eq!(search_dep.get("derived-mid-utf8").map(String::as_str), Some("NotRegistered"));
    r.session.close().unwrap();
}

#[test]
fn native_graph_admission_charges_inline_properties_on_nodes_and_edges() {
    for is_edge in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = fixtures::database(dir.path());
        fixtures::add_graph(&path);
        // 1 MiB inline properties * 50 target/edge multiplicity + 32 MiB base > 64 MiB cap.
        let large_prop = format!(r#"{{"blob":"{}"}}"#, "p".repeat(1024 * 1024));
        let c = rusqlite::Connection::open(&path).unwrap();
        if is_edge {
            c.execute(
                "UPDATE canonical_edges SET properties=?1 WHERE logical_id='parallel-a'",
                [&large_prop],
            )
            .unwrap();
        } else {
            c.execute(
                "UPDATE canonical_nodes SET properties=?1 WHERE logical_id='alpha'",
                [&large_prop],
            )
            .unwrap();
        }
        drop(c);

        let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
        let root = handle(&mut r, Class::Node, "derived", true);
        assert_eq!(
            r.graph_serving(&root, Direction::Outgoing, Some("supports".into()), 1),
            Err("budget_exhausted"),
            "graph_admission must charge inline properties (is_edge={is_edge})"
        );
        r.session.close().unwrap();
    }
}

#[test]
fn historical_as_of_graph_and_graph_serving_resolve_superseded_revisions_and_p4_13() {
    use crate::storage::graph::AsOf;
    use fathomdb_engine::{
        ArtifactRevisionId, CanonicalHash, PropertyMap, PropertyValue, SourceLocator,
        SourceRevisionId, SourceVersionId, WriteProvenanceV1,
    };
    use sha2::{Digest, Sha256};

    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    let src_body = "historical source body";
    let digest: String =
        Sha256::digest(src_body.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    let derived = |rev: &str| {
        WriteProvenanceV1::derived(
            ArtifactRevisionId::new(rev).unwrap(),
            SourceVersionId::new("hist-v1").unwrap(),
            SourceRevisionId::new("hist-src-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest.clone()).unwrap(),
        )
    };
    let mk_node = |id: &str, rev: &str, body: &str, stage: &str| {
        let mut props = PropertyMap::new();
        props.insert("stage".into(), PropertyValue::String(stage.into()));
        fathomdb_engine::PreparedWrite::ProvenancedNode(fathomdb_engine::ProvenancedNodeV1 {
            kind: "document".into(),
            body: body.into(),
            source_id: fathomdb_engine::SourceId::new("fixture:hist").unwrap(),
            logical_id: Some(id.into()),
            state: fathomdb_engine::InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: Some(props),
            provenance: derived(rev),
        })
    };
    let mk_edge = |id: &str, rev: &str, from: &str, to: &str, ver: &str| {
        let mut props = PropertyMap::new();
        props.insert("ver".into(), PropertyValue::String(ver.into()));
        fathomdb_engine::PreparedWrite::ProvenancedEdge(fathomdb_engine::ProvenancedEdgeV1 {
            kind: "links".into(),
            from: from.into(),
            to: to.into(),
            source_id: fathomdb_engine::SourceId::new("fixture:hist").unwrap(),
            logical_id: Some(id.into()),
            body: Some(format!("edge {ver}")),
            t_valid: None,
            t_invalid: None,
            confidence: Some(0.9),
            extractor_model_id: Some("extractor-v1".into()),
            temporal_fallback: Some(false),
            properties: Some(props),
            provenance: derived(rev),
        })
    };

    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    let r1 = opened
        .engine
        .write(&[
            fixtures::provenanced_node(
                Some("hist-src"),
                src_body,
                "fixture:hist",
                None,
                WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("hist-src-r1").unwrap(),
                    SourceVersionId::new("hist-v1").unwrap(),
                ),
            ),
            mk_node("hist-root", "hist-root-r1", "root v1", "v1"),
            mk_node("hist-mid", "hist-mid-r1", "mid v1", "v1"),
            mk_node("hist-leaf", "hist-leaf-r1", "leaf v1", "v1"),
            mk_edge("e-root-mid", "e-root-mid-r1", "hist-root", "hist-mid", "e1"),
            mk_edge("e-mid-leaf", "e-mid-leaf-r1", "hist-mid", "hist-leaf", "e1"),
        ])
        .unwrap();
    let c1 = r1.cursor;
    std::thread::sleep(std::time::Duration::from_millis(5));
    let r2 = opened
        .engine
        .write(&[
            mk_node("hist-root", "hist-root-r2", "root v2", "v2"),
            mk_node("hist-mid", "hist-mid-r2", "mid v2", "v2"),
            mk_node("hist-leaf", "hist-leaf-r2", "leaf v2", "v2"),
            mk_node("hist-future", "hist-future-r2", "future v2", "v2"),
            mk_edge("e-root-mid", "e-root-mid-r2", "hist-root", "hist-mid", "e2"),
            mk_edge("e-root-future", "e-root-future-r2", "hist-root", "hist-future", "e2"),
        ])
        .unwrap();
    let c2 = r2.cursor;
    assert!(c2 > c1);
    std::thread::sleep(std::time::Duration::from_millis(5));
    let r3 = opened
        .engine
        .write(&[
            mk_node("hist-root", "hist-root-r3", "root v3", "v3"),
            mk_node("hist-mid", "hist-mid-r3", "mid v3", "v3"),
            mk_node("hist-late", "hist-late-r3", "late v3", "v3"),
            mk_edge("e-root-late", "e-root-late-r3", "hist-root", "hist-late", "e3"),
        ])
        .unwrap();
    let c3 = r3.cursor;
    assert!(c3 > c2);
    opened.engine.close().unwrap();
    drop(opened);

    let c = rusqlite::Connection::open(&path).unwrap();
    let t1: i64 = c
        .query_row(
            "SELECT committed_at_ms FROM _fathomdb_commit_timestamps WHERE write_cursor=?1",
            [c1 as i64],
            |r| r.get(0),
        )
        .unwrap();
    let t0: i64 = c
        .query_row("SELECT min(committed_at_ms) FROM _fathomdb_commit_timestamps", [], |r| r.get(0))
        .unwrap();
    // Verify EXPLAIN QUERY PLAN for both historical UNION ALL lookups uses SEARCH ... USING INDEX.
    for (label, sql) in [
        (
            "historical_node",
            "EXPLAIN QUERY PLAN SELECT rowid, write_cursor FROM canonical_nodes WHERE logical_id='hist-root' AND superseded_at IS NULL AND write_cursor<=10 UNION ALL SELECT rowid, write_cursor FROM canonical_nodes WHERE logical_id='hist-root' AND superseded_at IS NOT NULL AND superseded_at>10 AND write_cursor<=10 ORDER BY write_cursor DESC LIMIT 1",
        ),
        (
            "historical_edge",
            "EXPLAIN QUERY PLAN SELECT rowid, write_cursor FROM canonical_edges INDEXED BY canonical_edges_from_active_idx WHERE from_id='hist-root' AND superseded_at IS NULL AND write_cursor<=10 UNION ALL SELECT rowid, write_cursor FROM canonical_edges INDEXED BY canonical_edges_from_sup_kind_idx WHERE from_id='hist-root' AND superseded_at IS NOT NULL AND superseded_at>10 AND write_cursor<=10 ORDER BY rowid LIMIT 100",
        ),
    ] {
        let mut stmt = c.prepare(sql).unwrap();
        let details: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(3))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert!(
            details.iter().any(|d| d.contains("SEARCH") && d.contains("INDEX")),
            "{label} must use indexed SEARCH: {details:?}"
        );
        assert!(
            !details.iter().any(|d| d.starts_with("SCAN ")),
            "{label} must never table-SCAN: {details:?}"
        );
    }
    drop(c);

    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let root_v3 = handle(&mut r, Class::Node, "hist-root", true);
    let future_v2 = handle(&mut r, Class::Node, "hist-future", true);

    // 1. Stored graph at c1 (and t1) resolves superseded seed hist-root-r1 and c1 neighbors/edges only.
    for (as_of, expected_mode) in
        [(AsOf::WriteCursor(c1), "write_cursor"), (AsOf::EpochMs(t1), "epoch_ms")]
    {
        let g = r
            .graph_as_of(
                &root_v3,
                Direction::Outgoing,
                Some("links".into()),
                2,
                vec![root_v3.clone()],
                as_of,
            )
            .unwrap();
        assert_eq!(g["scope"]["as_of"]["mode"], expected_mode);
        assert_eq!(g["query_context"]["as_of"]["mode"], expected_mode);
        assert_eq!(g["scope"]["as_of"]["resolved_as_of_write_cursor"], c1.to_string());
        let node_revs: std::collections::BTreeSet<&str> = g["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["revision_id"].as_str().unwrap())
            .collect();
        assert_eq!(
            node_revs,
            std::collections::BTreeSet::from(["hist-root-r1", "hist-mid-r1"]),
            "depth 1 expansion from hist-root alone yields hist-root-r1 and hist-mid-r1 (as_of={as_of:?})"
        );
        let edge_revs: std::collections::BTreeSet<&str> = g["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["revision_id"].as_str().unwrap())
            .collect();
        assert_eq!(edge_revs, std::collections::BTreeSet::from(["e-root-mid-r1"]));

        // 2. Native graph_serving at c1 (depth 2) resolves superseded hist-root-r1 & hist-mid-r1 origins (P4-13)!
        let gs = r
            .graph_serving_as_of(&root_v3, Direction::Outgoing, Some("links".into()), 2, as_of)
            .unwrap();
        assert_eq!(gs["request"]["as_of"]["mode"], expected_mode);
        assert_eq!(gs["query_context"]["as_of"]["resolved_as_of_write_cursor"], c1.to_string());
        assert_eq!(gs["scope"]["as_of"]["resolved_as_of_write_cursor"], c1.to_string());
        let items = gs["items"].as_array().unwrap();
        assert_eq!(
            items.len(),
            2,
            "expected hist-mid-r1 (hop 1) and hist-leaf-r1 (hop 2), got: {gs}"
        );
        let target_revs: std::collections::BTreeSet<&str> =
            items.iter().map(|item| item["target"]["revision_id"].as_str().unwrap()).collect();
        assert_eq!(target_revs, std::collections::BTreeSet::from(["hist-mid-r1", "hist-leaf-r1"]));
        let leaf = items.iter().find(|i| i["target"]["revision_id"] == "hist-leaf-r1").unwrap();
        assert_eq!(leaf["terminal_edge"]["revision_id"], "e-mid-leaf-r1");
        assert_eq!(leaf["payload"]["properties"]["stage"], "v1");
        assert_eq!(leaf["payload"]["terminal_edge_properties"]["ver"], "e1");
        let origin_revs: Vec<&str> = leaf["origin_records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o["revision_id"].as_str().unwrap())
            .collect();
        assert_eq!(
            origin_revs,
            vec!["hist-root-r1", "hist-mid-r1"],
            "P4-13: graph_serving must resolve superseded origin/predecessor revisions at as_of cursor"
        );
    }

    // 3. Pre-first-commit timestamp resolves to cursor 0 (empty, no fallback to current).
    let pre_graph = r
        .graph_as_of(
            &root_v3,
            Direction::Both,
            None,
            2,
            vec![root_v3.clone()],
            AsOf::EpochMs(t0 - 10_000),
        )
        .unwrap();
    assert_eq!(pre_graph["scope"]["as_of"]["resolved_as_of_write_cursor"], "0");
    assert!(pre_graph["nodes"].as_array().unwrap().is_empty());
    assert!(pre_graph["edges"].as_array().unwrap().is_empty());
    let pre_serving = r
        .graph_serving_as_of(&root_v3, Direction::Outgoing, None, 2, AsOf::EpochMs(t0 - 10_000))
        .unwrap();
    assert_eq!(pre_serving["scope"]["as_of"]["resolved_as_of_write_cursor"], "0");
    assert!(pre_serving["items"].as_array().unwrap().is_empty());

    // 4. Future seed hist-future did not exist at c1 -> empty graph, never inserts future revision.
    let future_at_c1 = r
        .graph_as_of(
            &future_v2,
            Direction::Both,
            None,
            1,
            vec![future_v2.clone()],
            AsOf::WriteCursor(c1),
        )
        .unwrap();
    assert!(future_at_c1["nodes"].as_array().unwrap().is_empty());
    assert!(future_at_c1["edges"].as_array().unwrap().is_empty());

    r.session.close().unwrap();
}

#[test]
fn historical_as_of_graph_obeys_all_eight_disclosure_flags_including_inline_properties() {
    use crate::storage::graph::AsOf;
    use fathomdb_engine::{
        ArtifactRevisionId, CanonicalHash, EngineConfig, PhysicalErasurePolicy,
        PhysicalErasureSelector, PropertyMap, PropertyValue, QueueResult, SourceLocator,
        SourceRevisionId, SourceVersionId, WriteProvenanceV1,
    };
    use sha2::{Digest, Sha256};
    use std::sync::Arc;

    let dir = tempfile::tempdir().unwrap();
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Diagnostics).unwrap();
    let path = dir.path().join("hist-d01.sqlite");
    let engine = Arc::new(
        Engine::open_with_choice_and_config(
            &path,
            EmbedderChoice::None,
            EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
        )
        .unwrap()
        .engine,
    );
    let src_body = "d01 historical source";
    let digest: String =
        Sha256::digest(src_body.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    let mk_node = |id: &str, rev: &str, source: &str, src_rev: &str, secret_prop: Option<&str>| {
        let props = secret_prop.map(|val| {
            let mut m = PropertyMap::new();
            m.insert("secret".into(), PropertyValue::String(val.into()));
            m
        });
        fathomdb_engine::PreparedWrite::ProvenancedNode(fathomdb_engine::ProvenancedNodeV1 {
            kind: "document".into(),
            body: format!("body {rev}"),
            source_id: fathomdb_engine::SourceId::new(source).unwrap(),
            logical_id: Some(id.into()),
            state: fathomdb_engine::InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: props,
            provenance: WriteProvenanceV1::derived(
                ArtifactRevisionId::new(rev).unwrap(),
                SourceVersionId::new("v1").unwrap(),
                SourceRevisionId::new(src_rev).unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest.clone()).unwrap(),
            ),
        })
    };
    let mk_edge = |id: &str,
                   rev: &str,
                   from: &str,
                   to: &str,
                   source: &str,
                   src_rev: &str,
                   secret_prop: Option<&str>| {
        let props = secret_prop.map(|val| {
            let mut m = PropertyMap::new();
            m.insert("secret".into(), PropertyValue::String(val.into()));
            m
        });
        fathomdb_engine::PreparedWrite::ProvenancedEdge(fathomdb_engine::ProvenancedEdgeV1 {
            kind: "links".into(),
            from: from.into(),
            to: to.into(),
            source_id: fathomdb_engine::SourceId::new(source).unwrap(),
            logical_id: Some(id.into()),
            body: Some(format!("edge {rev}")),
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
            properties: props,
            provenance: WriteProvenanceV1::derived(
                ArtifactRevisionId::new(rev).unwrap(),
                SourceVersionId::new("v1").unwrap(),
                SourceRevisionId::new(src_rev).unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest.clone()).unwrap(),
            ),
        })
    };
    let r1 = engine
        .write(&[
            fixtures::provenanced_node(
                Some("pub-src"),
                src_body,
                "fixture:public",
                None,
                WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("pub-src-r1").unwrap(),
                    SourceVersionId::new("v1").unwrap(),
                ),
            ),
            fixtures::provenanced_node(
                Some("sec-src"),
                src_body,
                "fixture:secret",
                None,
                WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("sec-src-r1").unwrap(),
                    SourceVersionId::new("v1").unwrap(),
                ),
            ),
            fixtures::provenanced_node(
                Some("purged-src"),
                src_body,
                "fixture:purged",
                None,
                WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("purged-src-r1").unwrap(),
                    SourceVersionId::new("v1").unwrap(),
                ),
            ),
            mk_node("pub-seed", "pub-seed-r1", "fixture:public", "pub-src-r1", None),
            mk_node(
                "sec-target",
                "sec-target-r1",
                "fixture:secret",
                "sec-src-r1",
                Some("HISTORICAL_SECRET_TARGET"),
            ),
            mk_node(
                "purged-target",
                "purged-target-r1",
                "fixture:purged",
                "purged-src-r1",
                Some("PURGED_SECRET"),
            ),
            mk_edge(
                "sec-edge",
                "sec-edge-r1",
                "pub-seed",
                "sec-target",
                "fixture:secret",
                "sec-src-r1",
                Some("HISTORICAL_SECRET_EDGE"),
            ),
            mk_edge(
                "purged-edge",
                "purged-edge-r1",
                "pub-seed",
                "purged-target",
                "fixture:purged",
                "purged-src-r1",
                Some("PURGED_SECRET_EDGE"),
            ),
        ])
        .unwrap();
    let c1 = r1.cursor;
    // Supersede sec-target and sec-edge so c1 revisions are BOTH inactive (superseded) and queued erasure-pending.
    engine
        .write(&[
            mk_node(
                "sec-target",
                "sec-target-r2",
                "fixture:secret",
                "sec-src-r1",
                Some("CURRENT_SECRET_TARGET"),
            ),
            mk_edge(
                "sec-edge",
                "sec-edge-r2",
                "pub-seed",
                "sec-target",
                "fixture:secret",
                "sec-src-r1",
                Some("CURRENT_SECRET_EDGE"),
            ),
        ])
        .unwrap();
    // Physically erase fixture:purged completely, and leave fixture:secret queued.
    let mut maintenance = engine.begin_erasure_maintenance(1000).unwrap();
    let purged_ticket = match maintenance
        .queue_physical_erasure(
            PhysicalErasureSelector::SourceAllAtCut { source_id: "fixture:purged".into() },
            "22222222222222222222222222222222",
        )
        .unwrap()
    {
        QueueResult::Accepted { ticket, .. } => ticket,
        other => panic!("{other:?}"),
    };
    assert!(matches!(
        maintenance.complete_physical_erasure(&purged_ticket).unwrap(),
        fathomdb_engine::CleanupResult::Complete { .. }
    ));
    assert!(matches!(
        maintenance
            .queue_physical_erasure(
                PhysicalErasureSelector::SourceAllAtCut { source_id: "fixture:secret".into() },
                "33333333333333333333333333333333",
            )
            .unwrap(),
        QueueResult::Accepted { .. }
    ));
    maintenance.close().unwrap();
    drop(maintenance);
    engine.close().unwrap();
    drop(engine);

    for n in [false, true] {
        for e in [false, true] {
            for t in [false, true] {
                let policy = Policy { no_show_inactive: n, show_erased: e, show_payloads: t };
                let mut r = Reader::new(Session::attach(&path, policy).unwrap());
                let seed = handle(&mut r, Class::Node, "pub-seed", true);
                let stored = r
                    .graph_as_of(
                        &seed,
                        Direction::Outgoing,
                        Some("links".into()),
                        1,
                        vec![seed.clone()],
                        AsOf::WriteCursor(c1),
                    )
                    .unwrap();
                let serving = r
                    .graph_serving_as_of(
                        &seed,
                        Direction::Outgoing,
                        Some("links".into()),
                        1,
                        AsOf::WriteCursor(c1),
                    )
                    .unwrap();
                let stored_s = stored.to_string();
                let serving_s = serving.to_string();
                // Physically erased rows never resurrect under any flag combination.
                assert!(
                    !stored_s.contains("PURGED_SECRET"),
                    "stored resurrected purged (n={n},e={e},t={t})"
                );
                assert!(
                    !serving_s.contains("PURGED_SECRET"),
                    "serving resurrected purged (n={n},e={e},t={t})"
                );
                let visible = e && !n;
                assert_eq!(
                    stored_s.contains("HISTORICAL_SECRET_TARGET"),
                    visible,
                    "stored target properties D01 mismatch (n={n},e={e},t={t})"
                );
                assert_eq!(
                    stored_s.contains("HISTORICAL_SECRET_EDGE"),
                    visible,
                    "stored edge properties D01 mismatch (n={n},e={e},t={t})"
                );
                assert_eq!(
                    serving_s.contains("HISTORICAL_SECRET_TARGET"),
                    visible,
                    "serving target properties D01 mismatch (n={n},e={e},t={t})"
                );
                assert_eq!(
                    serving_s.contains("HISTORICAL_SECRET_EDGE"),
                    visible,
                    "serving edge properties D01 mismatch (n={n},e={e},t={t})"
                );
                assert_eq!(
                    serving["items"].as_array().unwrap().len(),
                    usize::from(visible),
                    "serving items count D01 mismatch (n={n},e={e},t={t})"
                );
                if !visible {
                    assert_eq!(serving["scope"]["withheld"], 1);
                }
                r.session.close().unwrap();
            }
        }
    }
}

#[test]
fn native_graph_serving_preserves_i64_properties_bounds_keys_and_rejects_non_finite_floats() {
    use crate::storage::graph::AsOf;
    use fathomdb_engine::{PropertyMap, PropertyValue};

    // 1. Unit checks on property_map_json: exact i64 decimal string, non-finite float rejection, 4096-byte key bound.
    let mut non_finite_nan = PropertyMap::new();
    non_finite_nan.insert("bad".into(), PropertyValue::Float(f64::NAN));
    assert_eq!(
        super::property_map_json(Some(&non_finite_nan)),
        Err("unsupported_value_encoding"),
        "NaN property float must be refused instead of serializing as null"
    );
    let mut non_finite_inf = PropertyMap::new();
    non_finite_inf.insert("bad".into(), PropertyValue::Float(f64::INFINITY));
    assert_eq!(
        super::property_map_json(Some(&non_finite_inf)),
        Err("unsupported_value_encoding"),
        "Infinite property float must be refused"
    );
    let mut bounded = PropertyMap::new();
    let long_key = "🦀".repeat(1500); // 6000 bytes (> 4096)
    bounded.insert(long_key, PropertyValue::Int(9_007_199_254_740_993));
    let encoded = super::property_map_json(Some(&bounded)).unwrap();
    let obj = encoded.as_object().unwrap();
    assert_eq!(obj.len(), 1);
    let (encoded_key, encoded_val) = obj.iter().next().unwrap();
    assert!(
        encoded_key.len() <= 4096,
        "property key must be bounded to <= 4096 bytes, got {}",
        encoded_key.len()
    );
    assert_eq!(
        encoded_val,
        &serde_json::json!("9007199254740993"),
        "i64 > 2^53 must serialize as exact decimal string"
    );

    // 2. End-to-end graph_serving_as_of against a real Schema 38 store with 2^53 + 1 on node and edge.
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    fixtures::add_graph_history(&path);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let root = handle(&mut r, Class::Node, "hist-root", true);
    let gs = r
        .graph_serving_as_of(&root, Direction::Outgoing, Some("links".into()), 1, AsOf::Current)
        .unwrap();
    let items = gs["items"].as_array().unwrap();
    assert!(!items.is_empty(), "expected native graph targets");
    let early = items.iter().find(|i| i["target"]["logical_id"] == "hist-early").unwrap();
    assert_eq!(
        early["payload"]["properties"]["big_i64"],
        serde_json::json!("9007199254740993"),
        "node 64-bit integer property must survive graph_serving_as_of as exact decimal string"
    );
    assert_eq!(
        early["payload"]["terminal_edge_properties"]["edge_big_i64"],
        serde_json::json!("9007199254740993"),
        "edge 64-bit integer property must survive graph_serving_as_of as exact decimal string"
    );
    r.session.close().unwrap();
}
