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
