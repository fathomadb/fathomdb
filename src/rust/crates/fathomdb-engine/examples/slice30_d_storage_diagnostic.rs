//! Private, unqualified SQLite-error diagnostic; ordinary D modules/native remain separate.
#[path = "slice30_d_measure/capture.rs"]
mod capture;
#[path = "slice30_d_measure/entry.rs"]
mod entry;
#[path = "slice30_d_measure/provider.rs"]
mod provider;
#[path = "slice30_d_measure/schedule.rs"]
mod schedule;
#[path = "slice30_d_measure/workload.rs"]
mod workload;
use serde_json::{json, Value};
use std::os::raw::c_int;
use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

const CORPUS_SEED: u64 = 0x0AC0_13D0_13D0;
const QUERY_SEED: u64 = 0x0AC0_130D_EC0D_E000;
const QUERY_COUNT: usize = 500;
const NATURAL_CALLS: usize = 1000;

const MAX_RECORDS: usize = 64;
const MAX_MESSAGE_BYTES: usize = 1024;

struct Capture {
    origin: Instant,
    records: Vec<Value>,
    dropped: u64,
    truncated: u64,
}

static CAPTURE: OnceLock<Mutex<Capture>> = OnceLock::new();
static CAPTURE_ERRORS: AtomicU64 = AtomicU64::new(0);

fn record_sqlite_log(code: c_int, message: &str) {
    let Some(capture) = CAPTURE.get() else {
        CAPTURE_ERRORS.fetch_add(1, Ordering::Relaxed);
        return;
    };
    let Ok(mut capture) = capture.lock() else {
        CAPTURE_ERRORS.fetch_add(1, Ordering::Relaxed);
        return;
    };
    if capture.records.len() == MAX_RECORDS {
        capture.dropped = capture.dropped.saturating_add(1);
        return;
    }
    let mut end = message.len().min(MAX_MESSAGE_BYTES);
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    if end < message.len() {
        capture.truncated = capture.truncated.saturating_add(1);
    }
    let elapsed = u64::try_from(capture.origin.elapsed().as_nanos())
        .expect("bounded diagnostic elapsed time fits u64");
    capture.records.push(json!({"code":code,"message":&message[..end],
        "thread":format!("{:?}",std::thread::current().id()),"elapsed_ns":elapsed}));
}

fn retained_log() -> Value {
    match CAPTURE.get().and_then(|capture| capture.lock().ok()) {
        Some(capture) => json!({"records":capture.records,"dropped":capture.dropped,
            "truncated":capture.truncated,"callback_errors":CAPTURE_ERRORS.load(Ordering::Relaxed),
            "capture_complete":capture.dropped == 0 && capture.truncated == 0
                && CAPTURE_ERRORS.load(Ordering::Relaxed) == 0}),
        None => json!({"records":[],"dropped":null,"truncated":null,
            "capture_complete":false,"issues":["diagnostic capture unavailable"]}),
    }
}

fn run(args: &[std::ffi::OsString]) -> Result<Value, String> {
    let origin = Instant::now();
    CAPTURE
        .set(Mutex::new(Capture {
            origin,
            records: Vec::with_capacity(MAX_RECORDS),
            dropped: 0,
            truncated: 0,
        }))
        .map_err(|_| "one private diagnostic per process is required")?;
    // SAFETY: this standalone process registers once before opening SQLite or
    // starting workers. The callback is thread-safe and never calls SQLite.
    unsafe { rusqlite::trace::config_log(Some(record_sqlite_log)) }
        .map_err(|error| format!("SQLite diagnostic registration: {error:?}"))?;
    let mut report = json!({"scope":"private D SQLite-log diagnostic",
        "complete":false,"qualifies_natural":false});
    let mut output = None;
    match args {
        [mode, path] if mode == "--sqlite-error-control" => {
            let connection = rusqlite::Connection::open(Path::new(path))
                .map_err(|error| format!("control open: {error:?}"))?;
            let error = connection
                .execute_batch("SELECT * FROM slice30_d_diagnostic_missing")
                .expect_err("source control deliberately references an absent table");
            report["control_error_code"] =
                json!(error.sqlite_error().map(|error| error.extended_code));
            report["control_error"] = json!(error.to_string());
            connection.close().map_err(|(_, error)| format!("control close: {error:?}"))?;
        }
        [mode] if mode == "--bounds-control" => {
            // SQLite's own formatter truncates long log messages before the
            // callback; exercise this callback's separate bound with the literal.
            record_sqlite_log(1, &"雪".repeat(342));
            for _ in 0..64 {
                rusqlite::trace::log(1, "literal bounded diagnostic control");
            }
        }
        [config, directory] => {
            let config = Path::new(config);
            let value: Value = serde_json::from_slice(
                &std::fs::read(config)
                    .map_err(|error| format!("read diagnostic cell: {error:?}"))?,
            )
            .map_err(|error| format!("decode diagnostic cell: {error:?}"))?;
            let (identity, _) = entry::select_config(&value)?;
            report["cell"] = identity.clone();
            if std::env::var("AGENT_LONG").as_deref() != Ok("1") {
                report["workload_error"] =
                    json!("private diagnostic requires selected long-run entry");
            } else if identity["n"] != 100000
                || identity["layer"] != "T"
                || identity["mode"] != "overlap"
            {
                report["workload_error"] =
                    json!("private diagnostic accepts only fixed100k T overlap");
            } else {
                match entry::run_file(config, Path::new(directory), origin) {
                    Ok(value) => {
                        report["workload_complete"] = json!(value["complete"] == true);
                        report["workload"] = value;
                    }
                    Err(error) => report["workload_error"] = json!(error),
                }
            }
            output = Some(Path::new(directory).join("result.json"));
        }
        _ => return Err("diagnostic entry is not selected".into()),
    }
    report["sqlite_log"] = retained_log();
    if let Some(output) = output {
        let encoded = serde_json::to_vec_pretty(&report)
            .map_err(|error| format!("encode diagnostic result: {error:?}"))?;
        std::fs::write(output, encoded)
            .map_err(|error| format!("write diagnostic result: {error:?}"))?;
    }
    Ok(report)
}

fn main() -> ExitCode {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    match run(&args) {
        Ok(report) => {
            if matches!(
                args.first().and_then(|value| value.to_str()),
                Some("--sqlite-error-control" | "--bounds-control")
            ) {
                println!("{report}");
                ExitCode::SUCCESS
            } else if report["workload_complete"] == true
                && report["sqlite_log"]["capture_complete"] == true
            {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
