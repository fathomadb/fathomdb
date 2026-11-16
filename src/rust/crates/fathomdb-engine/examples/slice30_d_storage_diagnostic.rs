//! Private, unqualified SQLite-error diagnostic; ordinary D modules/native remain separate.
use serde_json::{json, Value};
use std::os::raw::c_int;
use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

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

fn capture() -> Value {
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
    CAPTURE
        .set(Mutex::new(Capture {
            origin: Instant::now(),
            records: Vec::with_capacity(MAX_RECORDS),
            dropped: 0,
            truncated: 0,
        }))
        .map_err(|_| "one private diagnostic per process is required")?;
    // SAFETY: this standalone process registers once before opening SQLite or
    // starting workers. The callback is thread-safe and never calls SQLite.
    unsafe { rusqlite::trace::config_log(Some(record_sqlite_log)) }
        .map_err(|error| format!("SQLite diagnostic registration: {error:?}"))?;
    let mut report = json!({"scope":"private D SQLite-log diagnostic; no ordinary measurement credit",
        "complete":false,"qualifies_natural":false});
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
        _ => return Err("diagnostic entry is not selected".into()),
    }
    report["sqlite_log"] = capture();
    Ok(report)
}

fn main() -> ExitCode {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    match run(&args) {
        Ok(report) => {
            println!("{report}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
