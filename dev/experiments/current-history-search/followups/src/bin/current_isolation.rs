use current_history_followups::current::CurrentStore;
use rusqlite::{Connection, OpenFlags};
use std::{collections::BTreeMap, io::Write, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = std::env::args().collect::<Vec<_>>();
    let input = Path::new(args.get(1).expect("original measurement directory"));
    let output = Path::new(args.get(2).expect("new replay directory"));
    std::fs::create_dir_all(output)?;
    let vocab = (0..1024)
        .map(|i| {
            format!(
                "{}{}{}{i:04}",
                (b'a' + ((i / 26 / 26) % 26) as u8) as char,
                (b'a' + ((i / 26) % 26) as u8) as char,
                (b'a' + (i % 26) as u8) as char
            )
        })
        .collect::<Vec<_>>();
    let band = &vocab[102..512];
    let mut rng = 0x0AC0_120D_EC0D_E000u64.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1);
    let queries = (0..1000)
        .map(|_| {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            band[rng as usize % band.len()].clone()
        })
        .collect::<Vec<_>>();
    let mut expected_heads = None;
    let mut expected_hits = None;
    let mut logs = std::fs::File::create(output.join("ordered-scores.jsonl"))?;
    for h in [0, 4, 16] {
        let original = input.join(format!("current-H{h}.sqlite"));
        assert!(
            !original.with_extension("sqlite-wal").exists(),
            "copy requires closed checkpointed original"
        );
        let copy = output.join(format!("replay-H{h}.sqlite"));
        assert!(!copy.exists(), "use a fresh output directory");
        std::fs::copy(&original, &copy)?;
        let store = CurrentStore::open(&copy)?;
        store.with_writer(|e|e.execute_for_test("UPDATE canonical_nodes SET valid_until=NULL WHERE superseded_at IS NULL; UPDATE _fathomdb_read_visibility_state SET generation=generation+1 WHERE singleton=1; UPDATE poc_certificate SET published_at=-9223372036854775808,next_boundary=-9223372036854775808,visibility=-1;"))?;
        store.bootstrap(50)?;
        let read = Connection::open_with_flags(&copy, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let heads=read.prepare("SELECT logical_id,body,state,source_id FROM canonical_nodes WHERE superseded_at IS NULL ORDER BY logical_id")?.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        assert_eq!(heads.len(), 10000);
        if let Some(ref expected) = expected_heads {
            assert_eq!(&heads, expected, "canonical current tuples differ at H{h}");
        } else {
            expected_heads = Some(heads.clone());
        }
        let owners = read
            .prepare(
                "SELECT write_cursor,logical_id FROM canonical_nodes WHERE superseded_at IS NULL",
            )?
            .query_map([], |r| Ok((r.get::<_, u64>(0)?, r.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<BTreeMap<_, _>>>()?;
        let mut hits = Vec::new();
        for (index, query) in queries.iter().enumerate() {
            let rows = store
                .search(query, 50, 10)?
                .rows
                .into_iter()
                .map(|r| (owners[&r.cursor].clone(), r.score.to_bits()))
                .collect::<Vec<_>>();
            writeln!(
                logs,
                "{}",
                serde_json::json!({"history":h,"query_index":index,"query":query,"ordered_owner_score_bits":rows})
            )?;
            hits.push(rows);
        }
        if let Some(ref expected) = expected_hits {
            assert_eq!(&hits, expected, "exact ordered owner/score mismatch H{h}");
        } else {
            expected_hits = Some(hits.clone());
        }
        let mut lengths = BTreeMap::new();
        for (_, body, _, _) in &heads {
            *lengths.entry(body.len()).or_insert(0usize) += 1;
        }
        println!(
            "{}",
            serde_json::json!({"history":h,"queries":queries.len(),"nonempty":hits.iter().filter(|r|!r.is_empty()).count(),"head_tuples_equal":true,"ordered_owner_score_bits_equal":true,"body_length_counts":lengths})
        );
        store.close()?;
    }
    Ok(())
}
