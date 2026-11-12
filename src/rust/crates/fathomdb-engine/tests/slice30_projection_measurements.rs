use fathomdb_engine::{
    EmbedderChoice, Engine, InitialState, PreparedWrite, ProjectionRole, ProjectionSpec,
    ProjectionVector, ReadView, SourceId, TestEmbedderChoice,
};
use proptest::prelude::*;
use proptest::test_runner::RngSeed;
use rusqlite::{Connection, OpenFlags};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[path = "../examples/slice30/varying_embedder.rs"]
mod varying_embedder;
use varying_embedder::VaryingEmbedder;

fn digest(path: &Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn inspect(path: &Path, count: usize) -> Vec<(String, u64)> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut rows = connection.prepare("SELECT logical_id,kind,write_cursor,body FROM canonical_nodes WHERE superseded_at IS NULL ORDER BY write_cursor").unwrap();
    let canonical: Vec<(String, String, u64, String)> = rows
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(canonical.len(), count, "independent canonical count, NOT_RED");
    let mut cursors = Vec::new();
    for (index, (id, kind, cursor, body)) in canonical.iter().enumerate() {
        assert_eq!(id, &format!("item-{index}"));
        assert_eq!(kind, if index % 2 == 0 { "doc" } else { "note" });
        assert_eq!(body, &format!("workerwitness only{index} é β index{index}"));
        let physical: (u64, usize, u64, u64, Option<u64>, usize) = connection.query_row(
            "SELECT v.rowid,length(d.embedding),s.window_number,s.byte_start,s.byte_end,s.body_length FROM _fathomdb_vector_rows v JOIN vector_default d ON d.rowid=v.rowid JOIN _fathomdb_vector_spans s ON s.vector_rowid=v.rowid AND s.write_cursor=v.write_cursor WHERE v.kind=?1 AND v.write_cursor=?2",
            rusqlite::params![kind, cursor], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?))).unwrap();
        assert!(physical.0 > 0);
        assert_eq!(
            (physical.1, physical.2, physical.3, physical.4, physical.5),
            (3072, 0, 0, None, body.len())
        );
        cursors.push((kind.clone(), *cursor));
    }
    for table in ["vector_default", "_fathomdb_vector_rows", "_fathomdb_vector_spans"] {
        assert_eq!(
            connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row
                    .get::<_, usize>(0))
                .unwrap(),
            count
        );
    }
    let profile: (String, String, u32) = connection.query_row("SELECT name,revision,dimension FROM _fathomdb_embedder_profiles WHERE profile='default'", [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
    assert_eq!(profile, ("varying".into(), "perf-gates-dense".into(), 768));
    cursors
}

fn verify(count: usize, cohort: usize, enabled: bool) {
    let root = tempfile::Builder::new().prefix("s30-workers-").tempdir().unwrap().keep();
    println!("worker proof: {} count={count} cohort={cohort} enabled={enabled}", root.display());
    let store = root.join("store.sqlite");
    let engine = Engine::open_with_test_choice_for_test(
        &store,
        TestEmbedderChoice::Caller(Arc::new(VaryingEmbedder)),
    )
    .unwrap()
    .engine;
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "s30_dense".into(),
                roles: [ProjectionRole::Searchable].into_iter().collect(),
                fts: None,
                vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
                source: None,
            }],
            &[],
        )
        .unwrap();
    engine.begin_d27_observation_for_test(Instant::now());
    let configuration = engine.d27_observation_for_test().unwrap().configuration_observation;
    assert_eq!((configuration.scheduler_runtime_threads, configuration.embedder_pool_size), (2, 5));
    engine.slice30_observe_projection_workers_for_test(enabled).unwrap();
    let origin = Instant::now();
    let mut writes = Vec::new();
    for start in (0..count).step_by(cohort) {
        let batch: Vec<_> = (start..count.min(start + cohort))
            .map(|index| PreparedWrite::Node {
                kind: if index % 2 == 0 { "doc" } else { "note" }.into(),
                body: format!("workerwitness only{index} é β index{index}"),
                source_id: SourceId::new("test:s30-workers").unwrap(),
                logical_id: Some(format!("item-{index}")),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect();
        let started = origin.elapsed().as_nanos();
        let receipt = engine.write(&batch).unwrap();
        writes.push(json!({"start_ns":started,"end_ns":origin.elapsed().as_nanos(),"rows":batch.len(),"cursor":receipt.cursor}));
    }
    assert_eq!(writes.len(), count.div_ceil(cohort));
    assert_eq!(writes.iter().map(|row| row["rows"].as_u64().unwrap()).sum::<u64>(), count as u64);
    let draining = Instant::now();
    engine.drain(5000).unwrap();
    let drain_ns = draining.elapsed().as_nanos();
    assert_eq!(engine.read_embedding_readiness().unwrap().pending_count, 0);
    let expected = inspect(&store, count);
    let collecting = Instant::now();
    let deadline = collecting + Duration::from_secs(5);
    let mut observations = Vec::new();
    let mut dropped = 0;
    loop {
        let report = engine.take_slice30_projection_measurements_for_test(origin).unwrap();
        assert_eq!(report["capacity"], 32768);
        assert!(report["dropped"].as_u64().unwrap() >= dropped);
        dropped = report["dropped"].as_u64().unwrap();
        observations.extend(report["records"].as_array().unwrap().iter().cloned());
        let observed: usize =
            observations.iter().map(|row| row["job_count"].as_u64().unwrap() as usize).sum();
        if !enabled || observed >= count || Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let collected_at_ns = origin.elapsed().as_nanos();
    let collection_ns = collecting.elapsed().as_nanos();
    engine.slice30_observe_queries_for_test(true).unwrap();
    let result = engine
        .search_text_only_filtered_with_snippets("only0", None, &ReadView::default(), 10, None)
        .unwrap();
    assert_eq!(
        result.results.iter().map(|row| row.id.to_prefixed()).collect::<Vec<_>>(),
        ["l:item-0"]
    );
    let queries = engine.take_slice30_measurements_for_test().unwrap();
    assert_eq!(queries.len(), 1, "query-only stream isolation, NOT_RED");
    assert_eq!(queries[0]["event"], "QueryComplete");
    engine.slice30_observe_projection_workers_for_test(false).unwrap();
    let after = engine.take_slice30_projection_measurements_for_test(origin).unwrap();
    assert!(after["records"].as_array().unwrap().is_empty(), "query did not append worker records");
    assert_eq!(after["dropped"], dropped);
    engine.close().unwrap();
    let wal = std::path::PathBuf::from(format!("{}-wal", store.display()));
    assert!(std::fs::metadata(wal).map_or(true, |metadata| metadata.len() == 0));
    let hash = digest(&store);
    let directory = root.join("fresh-copy");
    std::fs::create_dir(&directory).unwrap();
    let copy = directory.join("store.sqlite");
    std::fs::copy(&store, &copy).unwrap();
    assert_eq!(digest(&copy), hash);
    let reopened = Engine::open_with_choice(&copy, EmbedderChoice::None).unwrap().engine;
    assert_eq!(reopened.read_embedding_readiness().unwrap().pending_count, 0);
    assert_eq!(inspect(&copy, count), expected);
    reopened.close().unwrap();
    assert_eq!(digest(&store), hash);
    let proof = json!({"count":count,"cohort":cohort,"enabled":enabled,"writes":writes,"drain_ns":drain_ns,
        "collection_ns":collection_ns,"collected_at_ns":collected_at_ns,"records":observations,"dropped":dropped,
        "query_records":queries,"closed_hash":hash,"configured_projection_workers":2,"configured_provider_workers":5});
    std::fs::write(root.join("proof.json"), serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
    println!("existing configure/write/drain/profile/cursor/span/closed-copy/query controls passed; observed_jobs={}", observations.iter().map(|row| row["job_count"].as_u64().unwrap()).sum::<u64>());
    if !enabled {
        assert!(observations.is_empty());
        return;
    }
    assert_eq!(
        observations.iter().map(|row| row["job_count"].as_u64().unwrap()).sum::<u64>(),
        count as u64,
        "missing actual projection worker observations after successful real API controls"
    );
    assert_eq!(dropped, 0);
    let mut actual = Vec::new();
    let mut ids = BTreeSet::new();
    let mut workers: BTreeMap<u64, Vec<(u64, u64)>> = BTreeMap::new();
    for row in &observations {
        assert_eq!(row["event"], "ProjectionBatchEnd");
        assert_eq!(row["engine_id"], queries[0]["engine_id"]);
        assert!(ids.insert(row["batch_id"].as_u64().unwrap()));
        let worker = row["worker_idx"].as_u64().unwrap();
        assert!(worker < 2);
        assert_eq!(row["disposition"], "finished");
        assert_eq!(row["cleanup"], "confirmed");
        assert_eq!(row["commit_outcome"], "ok");
        assert_eq!(row["caught_unwind"], false);
        assert!(row["clock_issue"].is_null());
        let start = row["start_ns"].as_u64().unwrap();
        let end = row["end_ns"].as_u64().unwrap();
        assert!(start <= end && u128::from(end) <= collected_at_ns);
        workers.entry(worker).or_default().push((start, end));
        let cursors = row["cursors"].as_array().unwrap();
        assert!((1..=64).contains(&cursors.len()));
        assert_eq!(row["job_count"], cursors.len());
        for cursor in cursors {
            actual.push((
                cursor["kind"].as_str().unwrap().to_owned(),
                cursor["write_cursor"].as_u64().unwrap(),
            ));
        }
        for key in ["start_state", "end_state"] {
            let state = &row[key];
            assert!(state.is_object());
            assert!(state["at_ns"].as_u64().is_some());
            for value in ["queued_jobs", "active_jobs", "in_flight_cursors"] {
                assert!(state[value].as_u64().is_some());
            }
        }
    }
    assert_eq!(
        ids.into_iter().collect::<Vec<_>>(),
        (1..=observations.len() as u64).collect::<Vec<_>>()
    );
    for values in workers.values_mut() {
        values.sort();
        for pair in values.windows(2) {
            assert!(pair[0].1 <= pair[1].0);
        }
    }
    actual.sort();
    let mut expected = expected;
    expected.sort();
    assert_eq!(actual, expected);
}

#[test]
fn literal_real65_jobs_preserve_worker_and_query_identity() {
    verify(65, 65, true);
}

#[test]
fn disabled_worker_observation_preserves_query_stream() {
    verify(3, 2, false);
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 4, max_shrink_iters: 0, failure_persistence: None, rng_seed: RngSeed::Fixed(0x303011), .. ProptestConfig::default() })]
    #[test]
    fn bounded_cohorts_preserve_exact_cursor_population_and_completed_tail(count in 1usize..130, cohort in 1usize..66) {
        verify(count, cohort, true);
    }
}
