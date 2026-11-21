use crate::{
    fixtures::{add_erasure_cases, database},
    policy::Policy,
    session::Session,
    storage::{Class, Filter, Reader},
};
#[test]
fn real_atomic_deletion_and_telemetry_policy_hold_under_all_eight_launches() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    add_erasure_cases(&path);
    let c = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM canonical_nodes WHERE logical_id IN ('source','derived')",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM canonical_attributes", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(c.query_row("SELECT count(*) FROM _fathomdb_dependency_closures WHERE cause='purged' AND proof_json IS NOT NULL",[],|r|r.get::<_,i64>(0)).unwrap()>0);
    assert_eq!(c.query_row("SELECT count(*) FROM operational_mutations WHERE collection_name='erasure_pending_redaction'",[],|r|r.get::<_,i64>(0)).unwrap(),1);
    drop(c);
    for n in [false, true] {
        for e in [false, true] {
            for t in [false, true] {
                let policy = Policy { no_show_inactive: n, show_erased: e, show_payloads: t };
                let mut r = Reader::new(Session::attach(&path, policy).unwrap());
                let active = r
                    .page(
                        Class::Node,
                        0,
                        200,
                        Filter { identity: Some("beta".into()), ..Filter::default() },
                    )
                    .unwrap();
                let destination = dir.path().join(format!("no-target-{n}-{e}-{t}"));
                assert_eq!(
                    r.export_to(active["items"][0]["handle"].as_str().unwrap(), &destination)
                        .unwrap_err(),
                    "unavailable"
                );
                assert!(!destination.exists());
                let catalog = r.catalog().unwrap();
                assert_eq!(catalog["counts"]["node"]["revisions"], if n { "2" } else { "4" });
                let collection = catalog["collections"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|c| c["name"] == "erasure_pending_redaction")
                    .expect("real engine bookkeeping must be discoverable");
                assert_eq!(collection["engine_owned"], true);
                assert_eq!(collection["payload_status"], if t { "available" } else { "refused" });
                for id in ["source", "derived"] {
                    let erased = r
                        .page(
                            Class::Node,
                            0,
                            200,
                            Filter { identity: Some(id.into()), ..Filter::default() },
                        )
                        .unwrap();
                    assert!(erased["items"].as_array().unwrap().is_empty());
                }
                let log = r
                    .page(
                        Class::Log,
                        0,
                        200,
                        Filter {
                            collection: Some("erasure_pending_redaction".into()),
                            ..Filter::default()
                        },
                    )
                    .unwrap();
                assert_eq!(log["items"].as_array().unwrap().len(), 1);
                let handle = log["items"][0]["handle"].as_str().unwrap();
                if t {
                    let detail = r.detail(handle).unwrap();
                    let payload = detail["content"]["text"].as_str().unwrap();
                    assert!(payload.contains("l:source"));
                    assert!(!payload.contains("source body"));
                    assert_eq!(detail["record"]["retained_erasure_pending"], false);
                    assert_eq!(detail["record"]["inactive_or_history"], false);
                } else {
                    assert_eq!(r.detail(handle).unwrap_err(), "refused");
                    assert_eq!(r.chunk(handle, "body", 0, 100).unwrap_err(), "refused");
                    assert_eq!(r.associations(handle).unwrap_err(), "refused");
                }
                r.session.close().unwrap();
            }
        }
    }
}

#[test]
fn telemetry_key_constraints_and_field_samples_obey_payload_gate_under_all_flags() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    add_erasure_cases(&path);
    let c = rusqlite::Connection::open(&path).unwrap();
    let key:String=c.query_row("SELECT record_key FROM operational_mutations WHERE collection_name='erasure_pending_redaction'",[],|r|r.get(0)).unwrap();
    drop(c);
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
                let targeted = r.page(
                    Class::Log,
                    0,
                    10,
                    Filter {
                        collection: Some("erasure_pending_redaction".into()),
                        identity: Some(key.clone()),
                        ..Filter::default()
                    },
                );
                if t {
                    assert_eq!(targeted.unwrap()["items"].as_array().unwrap().len(), 1);
                } else {
                    assert_eq!(
                        targeted.unwrap_err(),
                        "refused",
                        "withheld key must not be probed through exact lookup"
                    );
                }
                let broad = r
                    .page(
                        Class::Log,
                        0,
                        10,
                        Filter { identity: Some(key.clone()), ..Filter::default() },
                    )
                    .unwrap();
                assert_eq!(broad["items"].as_array().unwrap().len(), usize::from(t));
                if !t {
                    assert_eq!(
                        broad["scope"]["identity_lookup_excluded_namespaces"],
                        serde_json::json!(["erasure_pending_redaction"])
                    );
                }
                let sample = r
                    .sample(
                        Class::Log,
                        0,
                        Filter {
                            collection: Some("erasure_pending_redaction".into()),
                            ..Filter::default()
                        },
                    )
                    .unwrap();
                let observation = &sample["items"][0]["field_observation"];
                if t {
                    assert_eq!(observation["root_kind"], "object");
                    assert!(!observation["names"].as_array().unwrap().is_empty());
                } else {
                    assert_eq!(observation["state"], "refused");
                    assert!(observation["root_kind"].is_null());
                    assert!(observation["names"].as_array().unwrap().is_empty());
                }
                r.session.close().unwrap();
            }
        }
    }
}

#[test]
fn queued_erasure_content_obeys_all_eight_launches_without_changing_engine_policy() {
    use fathomdb_engine::{
        EmbedderChoice, Engine, EngineConfig, ErasurePhase, PhysicalErasurePolicy,
        PhysicalErasureSelector, QueueResult,
    };
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Diagnostics).unwrap();
    let path = dir.path().join("queued.sqlite");
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
    let body = "legitimate retained queued content 🦀";
    let mut unrelated = crate::fixtures::node("unrelated", "unrelated retained content");
    if let fathomdb_engine::PreparedWrite::Node { source_id, .. } = &mut unrelated {
        *source_id = fathomdb_engine::SourceId::new("fixture:unrelated").unwrap();
    }
    engine.write(&[crate::fixtures::node("queued-target", body), unrelated]).unwrap();
    let mut maintenance = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = match maintenance
        .queue_physical_erasure(
            PhysicalErasureSelector::SourceAllAtCut { source_id: "fixture:source".into() },
            "0123456789abcdef0123456789abcdef",
        )
        .unwrap()
    {
        QueueResult::Accepted { ticket, .. } => ticket,
        other => panic!("expected legitimate queued state: {other:?}"),
    };
    maintenance.close().unwrap();
    engine.close().unwrap();
    drop(maintenance);
    drop(engine);
    for n in [false, true] {
        for e in [false, true] {
            for t in [false, true] {
                let policy = Policy { no_show_inactive: n, show_erased: e, show_payloads: t };
                let mut reader = Reader::new(Session::attach(&path, policy).unwrap());
                assert_eq!(
                    reader.session.engine.physical_erasure_policy(),
                    PhysicalErasurePolicy::ExplicitMaintenance
                );
                assert_eq!(
                    reader.session.engine.physical_erasure_status(&ticket).unwrap().unwrap().phase,
                    ErasurePhase::Queued
                );
                let page = reader
                    .page(
                        Class::Node,
                        0,
                        200,
                        Filter { identity: Some("queued-target".into()), ..Default::default() },
                    )
                    .unwrap();
                let records = page["items"].as_array().unwrap();
                assert_eq!(
                    records.len(),
                    1,
                    "erasure gate withholds content, not safe record metadata"
                );
                let handle = records[0]["handle"].as_str().unwrap();
                assert_eq!(records[0]["payload_status"], if e { "available" } else { "refused" });
                if !e {
                    assert_eq!(reader.detail(handle).unwrap_err(), "refused");
                    assert_eq!(reader.chunk(handle, "body", 0, 100).unwrap_err(), "refused");
                    let destination = dir.path().join(format!("denied-{n}-{t}"));
                    assert_eq!(reader.export_to(handle, &destination).unwrap_err(), "refused");
                    assert!(!destination.exists());
                }
                if e {
                    let handle = records[0]["handle"].as_str().unwrap();
                    assert_eq!(records[0]["retained_erasure_pending"], true);
                    assert_eq!(reader.detail(handle).unwrap()["content"]["text"], body);
                    let destination = dir.path().join(format!("queued-export-{n}-{t}"));
                    assert_eq!(
                        reader.export_to(handle, &destination).unwrap()["status"],
                        "complete"
                    );
                    let exported = std::fs::read(&destination).unwrap();
                    assert!(exported.ends_with(body.as_bytes()));
                }
                let unrelated = reader
                    .page(
                        Class::Node,
                        0,
                        200,
                        Filter { identity: Some("unrelated".into()), ..Default::default() },
                    )
                    .unwrap();
                let kept = &unrelated["items"][0];
                assert_eq!(kept["retained_erasure_pending"], false);
                assert_eq!(kept["payload_status"], "available");
                assert_eq!(
                    reader.detail(kept["handle"].as_str().unwrap()).unwrap()["content"]["text"],
                    "unrelated retained content"
                );
                reader.session.close().unwrap();
            }
        }
    }
}

#[test]
fn substantial_real_queue_keeps_catalog_and_unrelated_page_usable_and_fresh() {
    use fathomdb_engine::{
        EmbedderChoice, Engine, EngineConfig, PhysicalErasurePolicy, PhysicalErasureSelector,
        QueueResult, SourceId, WithdrawResult,
    };
    use std::sync::Arc;
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Diagnostics).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("substantial-queue.sqlite");
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
    let mut writes = Vec::new();
    for index in 0..200 {
        let mut write = crate::fixtures::node(&format!("unrelated-{index}"), "unrelated body");
        if let fathomdb_engine::PreparedWrite::Node { kind, source_id, .. } = &mut write {
            *kind = "unrelated".into();
            *source_id = SourceId::new("fixture:unrelated").unwrap();
        }
        writes.push(write);
    }
    for index in 0..8192 {
        writes.push(crate::fixtures::node(&format!("queued-{index}"), "queued body"));
    }
    engine.write(&writes).unwrap();
    drop(writes);
    let mut maintenance = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = match maintenance
        .queue_physical_erasure(
            PhysicalErasureSelector::SourceAllAtCut { source_id: "fixture:source".into() },
            "fedcba9876543210fedcba9876543210",
        )
        .unwrap()
    {
        QueueResult::Accepted { ticket, .. } => ticket,
        other => panic!("real queued scope must be admitted: {other:?}"),
    };
    maintenance.close().unwrap();
    drop(maintenance);
    engine.close().unwrap();
    drop(engine);
    let mut reader = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let catalog =
        reader.catalog().expect("bounded catalog must remain usable with legitimate queued scope");
    assert_eq!(catalog["status"], "complete");
    assert_eq!(catalog["scope"]["exact"], true);
    assert_eq!(catalog["counts"]["node"]["revisions"], "8392");
    let page = reader
        .page(Class::Node, 0, 200, Filter { kind: Some("unrelated".into()), ..Default::default() })
        .unwrap();
    assert_eq!(page["status"], "complete");
    assert_eq!(page["items"].as_array().unwrap().len(), 200);
    assert!(page["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| item["payload_status"] == "available"
            && item["retained_erasure_pending"] == false));
    let page = reader
        .page(
            Class::Node,
            0,
            200,
            Filter { identity: Some("queued-0".into()), ..Default::default() },
        )
        .unwrap();
    let handle = page["items"][0]["handle"].as_str().unwrap().to_string();
    assert_eq!(reader.detail(&handle).unwrap_err(), "refused");
    assert_eq!(
        reader.session.engine.withdraw_queued_erasure(&ticket).unwrap(),
        WithdrawResult::WithdrawnBeforeRemoval
    );
    let current =
        reader.detail(&handle).expect("a fresh read must not reuse the prior queued scope");
    assert_eq!(current["record"]["retained_erasure_pending"], false);
    assert_eq!(current["content"]["text"], "queued body");
    assert_eq!(
        reader.session.engine.physical_erasure_policy(),
        PhysicalErasurePolicy::ExplicitMaintenance
    );
    reader.session.close().unwrap();
}
