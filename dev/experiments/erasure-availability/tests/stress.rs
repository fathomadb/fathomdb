use erasure_availability::*;
use fathomdb_engine::{PreparedWrite, SourceId};
fn ranked(path: &std::path::Path, table: &str) -> Vec<(i64, u64)> {
    let c = reader(path).unwrap();
    let rows=c.prepare(&format!("SELECT rowid,bm25({table}) FROM {table} WHERE {table} MATCH 'common' ORDER BY bm25({table}),rowid")).unwrap().query_map([],|r|Ok((r.get(0)?,r.get::<_,f64>(1)?.to_bits()))).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
    rows
}
fn write(family: &str, id: &str, source: &str, body: &str) -> PreparedWrite {
    match family {
        "property" => node(id, source, &serde_json::json!({"p1":body,"p2":body}).to_string()),
        "edge" => PreparedWrite::Edge {
            kind: "rel".into(),
            from: "left".into(),
            to: "right".into(),
            source_id: SourceId::new(source).unwrap(),
            logical_id: Some(id.into()),
            body: Some(body.into()),
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
        },
        _ => node(id, source, body),
    }
}
#[test]
fn large_common_postings_unicode_and_empty_documents_preserve_same_cut_rank_bits() {
    for family in ["node", "edge", "property"] {
        for eager in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            let f = Fixture::build(&dir.path().join("db.sqlite"), family, false, 4, 0).unwrap();
            let mut body = format!("{MARKER} common ");
            while body.len() < 1_048_576 {
                body.push_str("common forest ");
            }
            body.truncate(1_048_576);
            f.engine
                .write(&[
                    write(family, "large", "target", &body),
                    write(family, "empty", "target", "!!!"),
                    write(family, "unicode", "survivor", "common café running 東京 phrase αβγ"),
                ])
                .unwrap();
            f.engine.drain(60_000).unwrap();
            assert!(physical_markers(&f.path, MARKER).unwrap() > 0);
            f.erase().unwrap();
            let table = match family {
                "edge" => "search_index_edges",
                "property" => "property_search_index",
                _ => "search_index",
            };
            let before = ranked(&f.path, table);
            assert!(!before.is_empty());
            if eager {
                rebuild_eager(&f.engine).unwrap();
                assert!(checkpoint(&f.engine).unwrap());
            } else {
                admit(&f.engine, 100, 130).unwrap();
                for _ in 0..1024 {
                    if job(&f.engine).unwrap().phase != "cleaning" {
                        break;
                    }
                    step(&f.engine, 1, 101, false).unwrap();
                }
                assert!(finalize_at(&f.engine, &f.path, MARKER, 102).unwrap());
            }
            assert_eq!(physical_markers(&f.path, MARKER).unwrap(), 0, "{family}/{eager}");
            assert_eq!(ranked(&f.path, table), before, "{family}/{eager}");
            f.engine.close().unwrap();
        }
    }
}
