use erasure_availability::*;
use proptest::prelude::*;
use std::process::Command;

#[test]
#[ignore = "child process used by the crash/reopen driver"]
fn crash_child() {
    let path = std::path::PathBuf::from(std::env::var("S81_CRASH_PATH").unwrap());
    let engine = open(&path).unwrap();
    match std::env::var("S81_CRASH_PHASE").unwrap().as_str() {
        "uncommitted_admission" => {
            let _:Result<()>=engine.maintenance_transaction_for_test::<()>(|tx| {
                tx.execute_batch("CREATE TABLE poc_uncommitted_intent(value INTEGER); INSERT INTO poc_uncommitted_intent VALUES(1)")?;
                std::process::exit(73);
            }).map(|_|()).map_err(Into::into);
        }
        "committed_progress" => {
            step(&engine, 1, 102, false).unwrap();
            std::process::exit(73);
        }
        "uncommitted_progress" => {
            let _: Result<()> = engine
                .maintenance_transaction_for_test::<()>(|tx| {
                    tx.execute(
                        "INSERT INTO search_index(search_index,rank) VALUES('merge',-1)",
                        [],
                    )?;
                    tx.execute("UPDATE poc_sanitation SET steps=steps+1", [])?;
                    std::process::exit(73);
                })
                .map(|_| ())
                .map_err(Into::into);
        }
        _ => panic!("unknown phase"),
    }
}
fn crash(path: &std::path::Path, phase: &str) {
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "crash_child", "--ignored"])
        .env("S81_CRASH_PATH", path)
        .env("S81_CRASH_PHASE", phase)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(73));
}
#[test]
fn process_exit_rolls_back_uncommitted_work_and_preserves_committed_progress() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("crash.sqlite");
    let f = Fixture::build(&path, "node", false, 4, 1).unwrap();
    f.erase().unwrap();
    f.engine.close().unwrap();
    crash(&path, "uncommitted_admission");
    let engine = open(&path).unwrap();
    assert_eq!(
        engine
            .query_i64_col_for_test(
                "SELECT count(*) FROM sqlite_master WHERE name='poc_uncommitted_intent'"
            )
            .unwrap(),
        vec![0]
    );
    admit(&engine, 100, 130).unwrap();
    let initial = job(&engine).unwrap();
    engine.close().unwrap();
    crash(&path, "uncommitted_progress");
    let engine = open(&path).unwrap();
    assert_eq!(job(&engine).unwrap(), initial);
    engine.close().unwrap();
    crash(&path, "committed_progress");
    let engine = open(&path).unwrap();
    assert_eq!(job(&engine).unwrap().steps, 1);
    assert_eq!(job(&engine).unwrap().deadline, 130);
    for _ in 0..256 {
        if job(&engine).unwrap().phase != "cleaning" {
            break;
        }
        step(&engine, 1, 103, false).unwrap();
    }
    assert!(finalize_at(&engine, &path, MARKER, 104).unwrap());
    engine.close().unwrap();
}

fn rows(path: &std::path::Path) -> Vec<Vec<(i64, String)>> {
    let c = reader(path).unwrap();
    TABLES
        .iter()
        .map(|table| {
            let column = if *table == "property_search_index" { "attr_value" } else { "body" };
            c.prepare(&format!("SELECT rowid,{column} FROM {table} ORDER BY rowid"))
                .unwrap()
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
        })
        .collect()
}
#[test]
fn sanitation_preserves_exact_surviving_ids_and_content() {
    for family in ["node", "edge", "property"] {
        for eager in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            let f = Fixture::build(&dir.path().join("db.sqlite"), family, false, 4, 1).unwrap();
            f.erase().unwrap();
            let before = rows(&f.path);
            if eager {
                rebuild_eager(&f.engine).unwrap();
            } else {
                admit(&f.engine, 100, 130).unwrap();
                for _ in 0..256 {
                    if job(&f.engine).unwrap().phase != "cleaning" {
                        break;
                    }
                    step(&f.engine, 1, 101, false).unwrap();
                }
                assert!(finalize_at(&f.engine, &f.path, MARKER, 102).unwrap());
            }
            assert_eq!(before, rows(&f.path));
            f.engine.close().unwrap();
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(8))]
    #[test]
    fn rollback_sequences_preserve_immutable_deadline(flags in prop::collection::vec(any::<bool>(),1..12)) {
        let dir=tempfile::tempdir().unwrap();
        let f=Fixture::build(&dir.path().join("db.sqlite"),"node",false,4,0).unwrap();
        f.erase().unwrap(); admit(&f.engine,100,130).unwrap();
        for fail in flags {
            if job(&f.engine).unwrap().phase!="cleaning" {break;}
            let before=job(&f.engine).unwrap();
            let result=step(&f.engine,1,101,fail);
            if fail {prop_assert!(result.is_err()); prop_assert_eq!(job(&f.engine).unwrap(),before);} else {prop_assert!(result.is_ok());}
            prop_assert_eq!(job(&f.engine).unwrap().deadline,130);
        }
        f.engine.close().unwrap();
    }
}
