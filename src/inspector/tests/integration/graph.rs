use crate::{
    fixtures,
    policy::Policy,
    session::Session,
    storage::{graph::Direction, Class, Filter, Reader},
};
use fathomdb_engine::{EmbedderChoice, Engine, PreparedWrite, SourceId};
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
        ArtifactRevisionId, CanonicalHash, InitialState, ProvenancedEdgeV1, ProvenancedNodeV1,
        SourceLocator, SourceRevisionId, SourceVersionId, WriteProvenanceV1,
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
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "document".into(),
                body,
                source_id: SourceId::new("fixture:large").unwrap(),
                logical_id: Some(id.into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance,
            })
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
                PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                    kind: "supports".into(),
                    from: "large-root".into(),
                    to: "large-target".into(),
                    source_id: SourceId::new("fixture:large").unwrap(),
                    logical_id: Some("large-edge".into()),
                    body: None,
                    t_valid: None,
                    t_invalid: None,
                    confidence: None,
                    extractor_model_id: None,
                    temporal_fallback: None,
                    provenance: derived("large-edge-r1"),
                }),
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
    let writes=edges.iter().enumerate().map(|(i,(from,to,kind))|PreparedWrite::Edge{kind:format!("k{kind}"),from:format!("p{from}"),to:format!("p{to}"),source_id:SourceId::new("fixture:property").unwrap(),logical_id:Some(format!("e{i}")),body:Some("edge body".into()),t_valid:None,t_invalid:None,confidence:None,extractor_model_id:None,temporal_fallback:None}).collect::<Vec<_>>();
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
                PreparedWrite::Edge {
                    kind: "long-id".into(),
                    from: "alpha".into(),
                    to: id.clone(),
                    source_id: SourceId::new("fixture:long").unwrap(),
                    logical_id: Some("long-endpoint-edge".into()),
                    body: None,
                    t_valid: None,
                    t_invalid: None,
                    confidence: None,
                    extractor_model_id: None,
                    temporal_fallback: None,
                },
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
        opened
            .engine
            .write(&[PreparedWrite::Node {
                kind: large.clone(),
                body: format!("revision {n}"),
                source_id: SourceId::new(large.clone()).unwrap(),
                logical_id: Some(large.clone()),
                state: fathomdb_engine::InitialState::Pending,
                reason: Some(large.clone()),
                valid_from: None,
                valid_until: None,
            }])
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
        ArtifactRevisionId, CanonicalHash, InitialState, ProvenancedEdgeV1, ProvenancedNodeV1,
        SourceDependencyRegistrationV1, SourceLocator, SourceRevisionId, SourceVersionId,
        WriteProvenanceV1,
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
        PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            kind: "document".into(),
            body: body.into(),
            source_id: SourceId::new("fixture:prov").unwrap(),
            logical_id: Some(id.into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: WriteProvenanceV1::derived(
                ArtifactRevisionId::new(rev).unwrap(),
                SourceVersionId::new("prov-v1").unwrap(),
                SourceRevisionId::new("prov-source-r1").unwrap(),
                locator,
                CanonicalHash::sha256(whole_digest.clone()).unwrap(),
            ),
        })
    };
    let make_edge = |kind: &str, edge_id: &str, target_id: &str| {
        PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
            kind: kind.into(),
            from: "prov-hub".into(),
            to: target_id.into(),
            source_id: SourceId::new("fixture:prov").unwrap(),
            logical_id: Some(edge_id.into()),
            body: Some(format!("edge to {target_id}")),
            t_valid: None,
            t_invalid: None,
            confidence: Some(0.9),
            extractor_model_id: Some("extractor-v1".into()),
            temporal_fallback: Some(false),
            provenance: WriteProvenanceV1::derived(
                ArtifactRevisionId::new(format!("{edge_id}-r1")).unwrap(),
                SourceVersionId::new("prov-v1").unwrap(),
                SourceRevisionId::new("prov-source-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(whole_digest.clone()).unwrap(),
            ),
        })
    };

    opened
        .engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "document".into(),
                body: source_body.into(),
                source_id: SourceId::new("fixture:prov").unwrap(),
                logical_id: Some("prov-source".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("prov-source-r1").unwrap(),
                    SourceVersionId::new("prov-v1").unwrap(),
                ),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "document".into(),
                body: "hub node".into(),
                source_id: SourceId::new("fixture:prov").unwrap(),
                logical_id: Some("prov-hub".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("prov-hub-r1").unwrap(),
                    SourceVersionId::new("prov-hub-v1").unwrap(),
                ),
            }),
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
