//! Report-only bare-vec0 layout benchmark; see dev/experiments/slice60-vec0-scale/README.md.
#[path = "slice60_scale/support.rs"]
mod support;
use rusqlite::{params, Connection};
use serde_json::json;
use std::{error::Error, time::Instant};
use support::{equivalent_candidates, percentile, vector};
const K: usize = 192;
const TOTAL_SQL: &str = "WITH candidates AS (SELECT rowid FROM vector_default WHERE embedding_bin MATCH vec_quantize_binary(vec_f32(?1)) ORDER BY distance LIMIT 192) SELECT c.rowid, vec_distance_l2(v.embedding,vec_f32(?2)) AS l2 FROM candidates c JOIN vector_default v ON v.rowid=c.rowid ORDER BY l2 LIMIT 192";
fn blob(vector: &[f32]) -> Vec<u8> {
    vector.iter().flat_map(|v| v.to_le_bytes()).collect()
}
fn numbers(name: &str, default: &str) -> Result<Vec<usize>, Box<dyn Error>> {
    Ok(std::env::var(name)
        .unwrap_or_else(|_| default.into())
        .split(',')
        .map(str::parse)
        .collect::<Result<_, _>>()?)
}
fn ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.
}
fn stats(samples: &[f64]) -> serde_json::Value {
    json!({"p50":percentile(samples,0.5),"p95":percentile(samples,0.95)})
}
fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        let init: unsafe extern "C" fn(
            *mut rusqlite::ffi::sqlite3,
            *mut *mut std::os::raw::c_char,
            *const rusqlite::ffi::sqlite3_api_routines,
        ) -> std::os::raw::c_int = std::mem::transmute(sqlite_vec::sqlite3_vec_init as *const ());
        assert_eq!(rusqlite::ffi::sqlite3_auto_extension(Some(init)), 0);
    }
    let counts = numbers("S60_COUNTS", "10000,100000,300000,1000000,2000000")?;
    let dimensions = numbers("S60_DIMENSIONS", "384,768")?;
    let chunks = numbers("S60_CHUNKS", "64,128,256")?;
    let samples: usize = std::env::var("S60_SAMPLES").unwrap_or_else(|_| "100".into()).parse()?;
    assert!(samples > 0 && !chunks.is_empty());
    for dimension in dimensions {
        assert!(dimension > 0 && dimension % 8 == 0);
        let queries: Vec<_> = (0..samples)
            .map(|i| serde_json::to_string(&vector(&format!("query:{i}"), dimension)).unwrap())
            .collect();
        for &count in &counts {
            assert!(count >= K);
            let mut baseline: Option<Vec<Vec<(i64, i64)>>> = None;
            let mut baseline_hits: Option<Vec<Vec<(i64, f64)>>> = None;
            for &chunk in &chunks {
                // TempDir removes only this cell, after its connection closes. Peak disk is one layout.
                let dir = tempfile::Builder::new().prefix("slice60-vec0-").tempdir()?;
                let path = dir.path().join("vectors.sqlite");
                let mut db = Connection::open(&path)?;
                db.execute_batch("PRAGMA page_size=4096; PRAGMA cache_size=-2048; PRAGMA mmap_size=0; PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL;")?;
                db.execute_batch(&format!("CREATE VIRTUAL TABLE vector_default USING vec0(embedding float[{dimension}],embedding_bin bit[{dimension}],source_type TEXT partition key,kind TEXT,created_at INTEGER,status TEXT,chunk_size={chunk})"))?;
                let seed_start = Instant::now();
                for first in (0..count).step_by(10000) {
                    let tx = db.transaction()?;
                    {
                        let mut insert=tx.prepare("INSERT INTO vector_default(rowid,embedding,embedding_bin,source_type,kind,created_at,status) VALUES(?1,vec_f32(?2),vec_quantize_binary(vec_f32(?2)),'article','doc',0,'')")?;
                        for i in first..(first + 10000).min(count) {
                            let v = blob(&vector(&format!("corpus:{i}"), dimension));
                            insert.execute(params![i as i64 + 1, v])?;
                        }
                    }
                    tx.commit()?;
                }
                let seed_ms = ms(seed_start);
                let disk_bytes = std::fs::metadata(&path)?.len();
                let mut full = db.prepare(TOTAL_SQL)?;
                for query in &queries {
                    let _: Vec<(i64, f64)> = full
                        .query_map(params![query, query], |r| Ok((r.get(0)?, r.get(1)?)))?
                        .collect::<Result<_, _>>()?;
                }
                let mut totals = Vec::new();
                let mut bit_times = Vec::new();
                let mut rerank_times = Vec::new();
                let mut all_candidates = Vec::new();
                let mut all_hits = Vec::new();
                let mut bit=db.prepare("SELECT rowid,CAST(distance AS INTEGER) FROM vector_default WHERE embedding_bin MATCH vec_quantize_binary(vec_f32(?1)) ORDER BY distance LIMIT 192")?;
                let mut rerank=db.prepare("SELECT vec_distance_l2(embedding,vec_f32(?1)) FROM vector_default WHERE rowid=?2")?;
                for query in &queries {
                    let start = Instant::now();
                    let mut hits: Vec<(i64, f64)> = full
                        .query_map(params![query, query], |r| Ok((r.get(0)?, r.get(1)?)))?
                        .collect::<Result<_, _>>()?;
                    totals.push(ms(start));
                    assert_eq!(hits.len(), K);
                    hits.truncate(10);
                    let start = Instant::now();
                    let candidates: Vec<(i64, i64)> = bit
                        .query_map([query], |r| Ok((r.get(0)?, r.get(1)?)))?
                        .collect::<Result<_, _>>()?;
                    bit_times.push(ms(start));
                    assert_eq!(candidates.len(), K);
                    let start = Instant::now();
                    let mut ranked = Vec::with_capacity(K);
                    for &(id, _) in &candidates {
                        ranked.push((
                            id,
                            rerank.query_row(params![query, id], |r| r.get::<_, f64>(0))?,
                        ));
                    }
                    ranked.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
                    ranked.truncate(10);
                    rerank_times.push(ms(start));
                    // This compares sorted scores to allow exact L2 ties to reorder rows.
                    assert_eq!(
                        hits.iter().map(|(_, d)| *d).collect::<Vec<_>>(),
                        ranked.iter().map(|(_, d)| *d).collect::<Vec<_>>()
                    );
                    all_candidates.push(candidates);
                    all_hits.push(hits);
                }
                let mut membership_differences = 0;
                let mut hit_differences = 0;
                if let Some(reference) = &baseline {
                    for (old, new) in reference.iter().zip(&all_candidates) {
                        assert!(
                            equivalent_candidates(old, new),
                            "candidate difference below Hamming cutoff"
                        );
                        let ids = |rows: &Vec<(i64, i64)>| {
                            rows.iter().map(|r| r.0).collect::<std::collections::BTreeSet<_>>()
                        };
                        membership_differences += usize::from(ids(old) != ids(new));
                    }
                    for (old, new) in baseline_hits.as_ref().unwrap().iter().zip(&all_hits) {
                        hit_differences += usize::from(old != new);
                    }
                } else {
                    baseline = Some(all_candidates.clone());
                    baseline_hits = Some(all_hits.clone());
                }
                println!(
                    "{}",
                    json!({"rows":count,"dimension":dimension,"chunk_size":chunk,"samples":samples,"seed_ms":seed_ms,"disk_bytes":disk_bytes,"records_at_one_vector":count,"records_at_5_1_vectors":count as f64/5.1,"total_ms":stats(&totals),"bit_knn_ms":stats(&bit_times),"diagnostic_rerank_ms":stats(&rerank_times),"cutoff_tie_membership_differences":membership_differences,"top10_differences":hit_differences,"baseline_chunk":chunks[0],"sqlite":rusqlite::version(),"vec":db.query_row("SELECT vec_version()",[],|r|r.get::<_,String>(0))?,"candidate_sets":all_candidates,"top10":all_hits})
                );
            }
        }
    }
    Ok(())
}
