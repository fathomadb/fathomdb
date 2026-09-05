#![cfg(feature = "test-hooks")]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use fathomdb_engine::{
    Engine, Filter, InitialState, PageRequestV1, PreparedWrite, ReadContextV1, ReadView,
    SearchFilter, SourceId,
};
use rusqlite::Connection;
use serde_json::json;

const KIND: &str = "slice45_doc";
const COLLECTION: &str = "slice45_state";

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|value| value.parse().ok()).unwrap_or(default)
}

fn database_path() -> PathBuf {
    std::env::var_os("FATHOM_SLICE45_DATABASE")
        .map(PathBuf::from)
        .expect("FATHOM_SLICE45_DATABASE is required")
}

fn fixed_json(index: usize) -> String {
    let prefix = format!(r#"{{"id":"{index:08}","text":""#);
    let suffix = r#""}"#;
    let fill = 256usize.checked_sub(prefix.len() + suffix.len()).expect("fixture prefix");
    let value = format!("{prefix}{}{suffix}", "x".repeat(fill));
    assert_eq!(value.len(), 256);
    value
}

fn seed(path: &Path, rows: usize) {
    if path.exists() {
        return;
    }
    let opened = Engine::open(path).expect("open seed database");
    opened
        .engine
        .write(&[PreparedWrite::AdminSchema {
            name: COLLECTION.to_string(),
            kind: "latest_state".to_string(),
            schema_json: "{}".to_string(),
            retention_json: "{}".to_string(),
        }])
        .expect("register state collection");
    for start in (0..rows).step_by(128) {
        let end = (start + 128).min(rows);
        let nodes = (start..end)
            .map(|index| PreparedWrite::Node {
                kind: KIND.to_string(),
                body: fixed_json(index),
                source_id: SourceId::new(format!("slice45:source:{index:08}")).unwrap(),
                logical_id: Some(format!("slice45:node:{index:08}")),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect::<Vec<_>>();
        opened.engine.write(&nodes).expect("seed canonical nodes");
        let state = (start..end)
            .map(|index| PreparedWrite::OpStore {
                collection: COLLECTION.to_string(),
                record_key: format!("slice45:key:{index:08}"),
                schema_id: None,
                body: fixed_json(index),
            })
            .collect::<Vec<_>>();
        opened.engine.write(&state).expect("seed operational state");
    }
    opened.engine.close().expect("close seeded database");
}

fn page(cursor: Option<fathomdb_engine::PageCursor>) -> PageRequestV1 {
    PageRequestV1 { schema_version: 1, limit: 100, cursor }
}

fn percentile_ms(values: &mut [u128], percentile: usize) -> f64 {
    values.sort_unstable();
    let index = (values.len() * percentile).div_ceil(100).saturating_sub(1);
    values[index] as f64 / 1_000_000.0
}

fn measure<F>(samples: usize, mut operation: F) -> (f64, f64, f64)
where
    F: FnMut(),
{
    for _ in 0..100.min(samples) {
        operation();
    }
    let started = Instant::now();
    let mut values = Vec::with_capacity(samples);
    for _ in 0..samples {
        let sample = Instant::now();
        operation();
        values.push(sample.elapsed().as_nanos());
    }
    let throughput = samples as f64 / started.elapsed().as_secs_f64();
    let p50 = percentile_ms(&mut values, 50);
    let p95 = percentile_ms(&mut values, 95);
    (p50, p95, throughput)
}

fn measure_stages(
    engine: &Engine,
    frozen: &fathomdb_engine::FrozenReadContextV1,
    request: &PageRequestV1,
    samples: usize,
) -> serde_json::Value {
    for _ in 0..100.min(samples) {
        engine.measure_slice45_frozen_stages_for_test(frozen, request).unwrap();
    }
    let mut cursor = Vec::with_capacity(samples);
    let mut token = Vec::with_capacity(samples);
    let mut binding = Vec::with_capacity(samples);
    for _ in 0..samples {
        let timing = engine.measure_slice45_frozen_stages_for_test(frozen, request).unwrap();
        cursor.push(timing.cursor_authentication_ns);
        token.push(timing.token_authentication_ns);
        binding.push(timing.snapshot_binding_ns);
    }
    json!({
        "cursor_authentication_p50_ms":percentile_ms(&mut cursor, 50),
        "cursor_authentication_p95_ms":percentile_ms(&mut cursor, 95),
        "token_authentication_p50_ms":percentile_ms(&mut token, 50),
        "token_authentication_p95_ms":percentile_ms(&mut token, 95),
        "snapshot_binding_p50_ms":percentile_ms(&mut binding, 50),
        "snapshot_binding_p95_ms":percentile_ms(&mut binding, 95),
    })
}

fn measure_mint_stages(
    engine: &Engine,
    context: &ReadContextV1,
    samples: usize,
) -> serde_json::Value {
    for _ in 0..100.min(samples) {
        engine.measure_slice45_mint_stages_for_test(context).unwrap();
    }
    let mut context_validation = Vec::with_capacity(samples);
    let mut snapshot_validation = Vec::with_capacity(samples);
    let mut binding = Vec::with_capacity(samples);
    let mut token_codec = Vec::with_capacity(samples);
    for _ in 0..samples {
        let timing = engine.measure_slice45_mint_stages_for_test(context).unwrap();
        context_validation.push(timing.context_validation_ns);
        snapshot_validation.push(timing.snapshot_validation_ns);
        binding.push(timing.binding_ns);
        token_codec.push(timing.token_codec_ns);
    }
    json!({
        "context_validation_p50_ms":percentile_ms(&mut context_validation, 50),
        "context_validation_p95_ms":percentile_ms(&mut context_validation, 95),
        "snapshot_validation_p50_ms":percentile_ms(&mut snapshot_validation, 50),
        "snapshot_validation_p95_ms":percentile_ms(&mut snapshot_validation, 95),
        "binding_p50_ms":percentile_ms(&mut binding, 50),
        "binding_p95_ms":percentile_ms(&mut binding, 95),
        "token_codec_p50_ms":percentile_ms(&mut token_codec, 50),
        "token_codec_p95_ms":percentile_ms(&mut token_codec, 95),
    })
}

fn full_page_walk(
    engine: &Engine,
    frozen: &fathomdb_engine::FrozenReadContextV1,
) -> serde_json::Value {
    let started = Instant::now();
    let mut cursor = None;
    let mut pages = 0usize;
    let mut items = 0usize;
    loop {
        let result = engine.read_canonical_page(KIND, frozen, &page(cursor)).unwrap();
        pages += 1;
        items += result.items.len();
        match result.next_cursor {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    let elapsed = started.elapsed();
    json!({
        "elapsed_ms": elapsed.as_secs_f64() * 1_000.0,
        "pages": pages,
        "items": items,
        "items_per_second": items as f64 / elapsed.as_secs_f64(),
    })
}

fn peak_rss_kib() -> u64 {
    fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix("VmHWM:")
                    .and_then(|rest| rest.split_whitespace().next())
                    .and_then(|value| value.parse().ok())
            })
        })
        .unwrap_or(0)
}

#[test]
#[ignore = "preregistered Slice 45 10k/50k latency and RSS measurement"]
fn measure_slice45_pagination_overhead() {
    let path = database_path();
    let rows = env_usize("FATHOM_SLICE45_ROWS", 10_000);
    let samples = env_usize("FATHOM_SLICE45_SAMPLES", 1_000);
    seed(&path, rows);
    if std::env::var_os("FATHOM_SLICE45_SEED_ONLY").is_some() {
        println!("{}", json!({"kind":"seed","rows":rows,"database":path}));
        return;
    }

    let mode = std::env::var("FATHOM_SLICE45_MODE").unwrap_or_else(|_| "latency".into());
    if mode == "cold" {
        let open_started = Instant::now();
        let opened = Engine::open(&path).expect("open cold measurement database");
        let open_ms = open_started.elapsed().as_secs_f64() * 1_000.0;
        let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
        let mint_started = Instant::now();
        let frozen = opened.engine.freeze_read_context(&context).expect("mint cold context");
        let mint_ms = mint_started.elapsed().as_secs_f64() * 1_000.0;
        let page_started = Instant::now();
        let first = opened.engine.read_canonical_page(KIND, &frozen, &page(None)).unwrap();
        let page_ms = page_started.elapsed().as_secs_f64() * 1_000.0;
        assert_eq!(first.items.len(), 100);
        let point_key = format!("slice45:key:{:08}", rows / 2);
        let state_started = Instant::now();
        assert!(opened
            .engine
            .read_operational_state(COLLECTION, &point_key, Some(&frozen))
            .unwrap()
            .is_some());
        let state_ms = state_started.elapsed().as_secs_f64() * 1_000.0;
        println!(
            "{}",
            json!({
                "schema_version":"slice45-pagination-performance.v1",
                "kind":"cold", "rows":rows, "open_ms":open_ms, "mint_ms":mint_ms,
                "first_page_ms":page_ms, "frozen_state_ms":state_ms,
                "peak_rss_kib":peak_rss_kib(),
            })
        );
        opened.engine.close().expect("close cold measurement database");
        return;
    }

    let opened = Engine::open(&path).expect("open measurement database");
    let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
    let frozen = opened.engine.freeze_read_context(&context).expect("mint context");
    let first = opened.engine.read_canonical_page(KIND, &frozen, &page(None)).unwrap();
    let continuation = first.next_cursor.clone().expect("continuation fixture");
    let sql = Connection::open(&path).expect("open exact SQL baseline");
    let point_key = format!("slice45:key:{:08}", rows / 2);

    let result = match mode.as_str() {
        "latency" => {
            let mut exact_operation = || {
                assert_eq!(
                    opened
                        .engine
                        .read_canonical_page_baseline_for_test(KIND, &frozen, 100)
                        .unwrap()
                        .len(),
                    100
                );
            };
            let mut frozen_page_operation = || {
                assert_eq!(
                    opened
                        .engine
                        .read_canonical_page(KIND, &frozen, &page(None))
                        .unwrap()
                        .items
                        .len(),
                    100
                );
            };
            let treatment_first = std::env::var_os("FATHOM_SLICE45_TREATMENT_FIRST").is_some();
            let (exact, frozen_page) = if treatment_first {
                let frozen_page = measure(samples, &mut frozen_page_operation);
                let exact = measure(samples, &mut exact_operation);
                (exact, frozen_page)
            } else {
                let exact = measure(samples, &mut exact_operation);
                let frozen_page = measure(samples, &mut frozen_page_operation);
                (exact, frozen_page)
            };
            let frozen_validation = measure(samples, || {
                opened.engine.validate_frozen_read_context_for_binding(&frozen).unwrap();
            });
            let mint_page = measure(samples, || {
                let fresh = opened.engine.freeze_read_context(&context).unwrap();
                assert_eq!(
                    opened
                        .engine
                        .read_canonical_page(KIND, &fresh, &page(None))
                        .unwrap()
                        .items
                        .len(),
                    100
                );
            });
            let continuation_page = measure(samples, || {
                assert_eq!(
                    opened
                        .engine
                        .read_canonical_page(KIND, &frozen, &page(Some(continuation.clone())))
                        .unwrap()
                        .items
                        .len(),
                    100
                );
            });
            let mut current_state_operation = || {
                assert!(opened
                    .engine
                    .read_operational_state(COLLECTION, &point_key, None)
                    .unwrap()
                    .is_some());
            };
            let mut frozen_state_operation = || {
                assert!(opened
                    .engine
                    .read_operational_state(COLLECTION, &point_key, Some(&frozen))
                    .unwrap()
                    .is_some());
            };
            let (current_state, frozen_state) = if treatment_first {
                let frozen_state = measure(samples, &mut frozen_state_operation);
                let current_state = measure(samples, &mut current_state_operation);
                (current_state, frozen_state)
            } else {
                let current_state = measure(samples, &mut current_state_operation);
                let frozen_state = measure(samples, &mut frozen_state_operation);
                (current_state, frozen_state)
            };
            let first_page_stages = measure_stages(&opened.engine, &frozen, &page(None), samples);
            let continuation_stages =
                measure_stages(&opened.engine, &frozen, &page(Some(continuation.clone())), samples);
            let mint_stages = measure_mint_stages(&opened.engine, &context, samples);
            let public_list = measure(samples, || {
                assert_eq!(
                    opened
                        .engine
                        .read_list_filter(KIND, &Filter::default(), 100, &ReadView::default())
                        .unwrap()
                        .len(),
                    100
                );
            });
            let full_walk = full_page_walk(&opened.engine, &frozen);
            json!({
                "kind":"latency", "exact_page":exact, "frozen_page":frozen_page,
                "frozen_validation":frozen_validation,
                "mint_plus_page":mint_page, "continuation_page":continuation_page,
                "current_state":current_state, "frozen_state":frozen_state,
                "public_list":public_list, "full_walk":full_walk,
                "mint_stages":mint_stages,
                "first_page_stages":first_page_stages,
                "continuation_stages":continuation_stages,
            })
        }
        arm => {
            let measurement = match arm {
                "exact_page" => measure(samples, || {
                    assert_eq!(
                        opened
                            .engine
                            .read_canonical_page_baseline_for_test(KIND, &frozen, 100)
                            .unwrap()
                            .len(),
                        100
                    );
                }),
                "frozen_page" => measure(samples, || {
                    opened.engine.read_canonical_page(KIND, &frozen, &page(None)).unwrap();
                }),
                "mint_plus_page" => measure(samples, || {
                    let fresh = opened.engine.freeze_read_context(&context).unwrap();
                    opened.engine.read_canonical_page(KIND, &fresh, &page(None)).unwrap();
                }),
                "continuation_page" => measure(samples, || {
                    opened
                        .engine
                        .read_canonical_page(KIND, &frozen, &page(Some(continuation.clone())))
                        .unwrap();
                }),
                "current_state" => measure(samples, || {
                    opened.engine.read_operational_state(COLLECTION, &point_key, None).unwrap();
                }),
                "frozen_state" => measure(samples, || {
                    opened
                        .engine
                        .read_operational_state(COLLECTION, &point_key, Some(&frozen))
                        .unwrap();
                }),
                _ => panic!("unknown FATHOM_SLICE45_MODE={arm}"),
            };
            json!({"kind":"rss", "arm":arm, "measurement":measurement})
        }
    };

    let terminal_rows: i64 = sql
        .query_row("SELECT COUNT(*) FROM _fathomdb_projection_terminal", [], |row| row.get(0))
        .unwrap();
    let database_bytes = fs::metadata(&path).unwrap().len();
    println!(
        "{}",
        json!({
            "schema_version":"slice45-pagination-performance.v1",
            "rows":rows,
            "samples":samples,
            "page_size":100,
            "payload_bytes":256,
            "sqlite_version":rusqlite::version(),
            "token_bytes":frozen.token.len(),
            "cursor_bytes":continuation.0.len(),
            "terminal_rows":terminal_rows,
            "database_bytes":database_bytes,
            "peak_rss_kib":peak_rss_kib(),
            "result":result,
        })
    );
    opened.engine.close().expect("close measurement database");
}
