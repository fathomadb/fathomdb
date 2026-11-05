use erasure_availability::*;

#[test]
fn merge_intent_progress_rollback_and_reopen_are_durable() {
    let dir = tempfile::tempdir().unwrap();
    let f = Fixture::build(&dir.path().join("db.sqlite"), "node", false, 4, 1).unwrap();
    f.erase().unwrap();
    admit(&f.engine, 100, 130).expect("durable sanitation admission");
    let original = job(&f.engine).unwrap();
    assert_eq!((original.accepted, original.deadline, original.steps), (100, 130, 0));
    assert!(admit(&f.engine, 101, 131).is_err(), "one reserved job, no deadline replacement");
    assert!(step(&f.engine, 1, 101, true).is_err());
    assert_eq!(job(&f.engine).unwrap(), original, "work and progress must roll back together");
    f.engine
        .write(&[node("unrelated", "other", "unrelated progress")])
        .expect("actual writer after rollback");
    f.engine.drain(60_000).unwrap();
    step(&f.engine, 1, 102, false).unwrap();
    let committed = job(&f.engine).unwrap();
    assert_eq!(committed.steps, 1);
    assert_eq!(committed.deadline, 130);
    let path = f.path.clone();
    f.engine.close().unwrap();
    let reopened = open(&path).unwrap();
    assert_eq!(job(&reopened).unwrap(), committed);
    for _ in 0..256 {
        if job(&reopened).unwrap().phase != "cleaning" {
            break;
        }
        step(&reopened, 1, 131, false).unwrap();
    }
    let done = job(&reopened).unwrap();
    assert_eq!(done.phase, "proof_pending");
    assert!(done.overdue);
    assert_eq!(done.deadline, 130);
    let complete = finalize(&reopened, &path, MARKER).unwrap();
    assert_eq!(complete, physical_markers(&path, MARKER).unwrap() == 0);
    assert_eq!(job(&reopened).unwrap().phase, if complete { "complete" } else { "residual" });
    reopened.close().unwrap();
}

#[test]
fn merge_protocol_reports_physical_outcome_for_every_family_and_geometry() {
    for family in ["node", "edge", "property"] {
        for compact_before_erase in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let f = Fixture::build(&dir.path().join("db.sqlite"), family, false, 4, 1).unwrap();
            if compact_before_erase {
                f.engine
                    .maintenance_transaction_for_test(|tx| {
                        for table in TABLES {
                            tx.execute(
                                &format!("INSERT INTO {table}({table}) VALUES('optimize')"),
                                [],
                            )?;
                        }
                        Ok(())
                    })
                    .unwrap();
            }
            let before = f.inventory().unwrap();
            assert!(before.iter().any(|r| r.shadow_markers > 0 && r.matches > 0));
            f.erase().unwrap();
            let legacy = f.inventory().unwrap();
            assert!(legacy.iter().any(|r| r.shadow_markers > 0), "positive ghost control {family}");
            let survivor_before = inventory(&f.path, "survivorneedle")
                .unwrap()
                .iter()
                .map(|r| r.matches)
                .collect::<Vec<_>>();
            admit(&f.engine, 100, 130).unwrap();
            for _ in 0..256 {
                if job(&f.engine).unwrap().phase != "cleaning" {
                    break;
                }
                step(&f.engine, 1, 101, false).unwrap();
            }
            assert_eq!(job(&f.engine).unwrap().phase, "proof_pending");
            let complete = finalize(&f.engine, &f.path, MARKER).unwrap();
            let after = f.inventory().unwrap();
            let residual = physical_markers(&f.path, MARKER).unwrap();
            println!(
                "{}",
                serde_json::json!({"family":family,"compact_before_erase":compact_before_erase,"before":before,"legacy":legacy,"after":after,"complete":complete,"residual":residual,"job":job(&f.engine).unwrap()})
            );
            assert_eq!(complete, residual == 0, "no work is not physical proof");
            assert!(after.iter().all(|r| r.matches == 0));
            let survivor_after = inventory(&f.path, "survivorneedle")
                .unwrap()
                .iter()
                .map(|r| r.matches)
                .collect::<Vec<_>>();
            assert_eq!(survivor_before, survivor_after);
            f.engine.close().unwrap();
        }
    }
}
