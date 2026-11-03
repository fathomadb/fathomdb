use erasure_availability::*;

#[test]
fn eager_fresh_and_legacy_remove_positive_physical_witnesses_in_all_families() {
    for family in ["node", "edge", "property"] {
        for legacy in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let f = Fixture::build(&dir.path().join("db.sqlite"), family, !legacy, 4, 1).unwrap();
            let before = f.inventory().unwrap();
            assert!(
                before.iter().any(|r| r.matches > 0 && r.shadow_markers > 0),
                "{family}: {before:?}"
            );
            assert!(physical_markers(&f.path, MARKER).unwrap() > 0);
            f.erase().unwrap();
            let after_erase = f.inventory().unwrap();
            assert!(after_erase.iter().all(|r| r.matches == 0), "{after_erase:?}");
            if legacy {
                assert!(
                    after_erase.iter().any(|r| r.shadow_markers > 0),
                    "legacy requires retained positive marker"
                );
                rebuild_eager(&f.engine).unwrap();
            }
            assert!(checkpoint(&f.engine).unwrap());
            let after = f.inventory().unwrap();
            println!(
                "{}",
                serde_json::json!({"family":family,"legacy":legacy,"before":before,"after":after,"physical_after":physical_markers(&f.path,MARKER).unwrap()})
            );
            assert!(after.iter().all(|r| r.shadow_markers == 0), "{family}/{legacy}: {after:?}");
            assert_eq!(physical_markers(&f.path, MARKER).unwrap(), 0, "{family}/{legacy}");
            let survivors = inventory(&f.path, "survivorneedle").unwrap();
            assert!(survivors.iter().any(|r| r.matches > 0));
            let path = f.path.clone();
            f.engine.close().unwrap();
            let reopened = open(&path).unwrap();
            assert!(checkpoint(&reopened).unwrap());
            assert_eq!(physical_markers(&path, MARKER).unwrap(), 0);
            assert!(inventory(&path, "survivorneedle").unwrap().iter().any(|r| r.matches > 0));
            reopened.close().unwrap();
        }
    }
}
