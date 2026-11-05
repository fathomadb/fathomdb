use erasure_availability::*;
#[test]
fn late_finalization_records_breach_and_completion_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let f = Fixture::build(&dir.path().join("db.sqlite"), "node", false, 4, 0).unwrap();
    f.erase().unwrap();
    admit(&f.engine, 100, 130).unwrap();
    for _ in 0..256 {
        if job(&f.engine).unwrap().phase != "cleaning" {
            break;
        }
        step(&f.engine, 1, 110, false).unwrap();
        assert!(
            !f.engine.search("survivorneedle").unwrap().results.is_empty(),
            "actual reader between units"
        );
    }
    assert!(finalize_at(&f.engine, &f.path, MARKER, 131).unwrap());
    assert!(job(&f.engine).unwrap().overdue, "late proof remains a breach");
    assert_eq!(job(&f.engine).unwrap().deadline, 130);
    assert!(finalize_at(&f.engine, &f.path, MARKER, 90).unwrap());
    assert!(job(&f.engine).unwrap().overdue, "clock rollback never clears a breach");
    f.engine.close().unwrap();
}

#[test]
fn missing_main_file_cannot_pass_physical_inspection() {
    let dir = tempfile::tempdir().unwrap();
    assert!(physical_markers(&dir.path().join("missing.sqlite"), MARKER).is_err());
}

#[test]
fn completed_on_time_job_does_not_become_overdue_on_late_observation() {
    let dir = tempfile::tempdir().unwrap();
    let f = Fixture::build(&dir.path().join("db.sqlite"), "node", false, 4, 0).unwrap();
    f.erase().unwrap();
    admit(&f.engine, 100, 130).unwrap();
    for _ in 0..256 {
        if job(&f.engine).unwrap().phase != "cleaning" {
            break;
        }
        step(&f.engine, 1, 110, false).unwrap();
    }
    assert!(finalize_at(&f.engine, &f.path, MARKER, 120).unwrap());
    let terminal = job(&f.engine).unwrap();
    assert!(!terminal.overdue);
    assert!(finalize_at(&f.engine, &f.path, MARKER, 140).unwrap());
    assert_eq!(job(&f.engine).unwrap(), terminal);
    f.engine.close().unwrap();
}

#[test]
fn physical_proof_crossing_deadline_records_breach() {
    let dir = tempfile::tempdir().unwrap();
    let f = Fixture::build(&dir.path().join("db.sqlite"), "node", false, 4, 0).unwrap();
    f.erase().unwrap();
    admit(&f.engine, 100, 130).unwrap();
    for _ in 0..256 {
        if job(&f.engine).unwrap().phase != "cleaning" {
            break;
        }
        step(&f.engine, 1, 110, false).unwrap();
    }
    let mut clock = [120, 140].into_iter();
    assert!(finalize_with_clock(&f.engine, &f.path, MARKER, || clock.next().unwrap()).unwrap());
    assert!(job(&f.engine).unwrap().overdue, "physical proof crossed original deadline");
    f.engine.close().unwrap();
}
