//! Private, unqualified SQLite-error diagnostic; ordinary D modules/native remain separate.
use serde_json::{json, Value};
use std::os::raw::c_int;
use std::path::Path;
use std::process::ExitCode;

fn record_sqlite_log(_code: c_int, _message: &str) {}

fn capture() -> Value {
    json!({"records":[],"dropped":0,"truncated":0,"capture_complete":true})
}

fn run(args: &[std::ffi::OsString]) -> Result<Value, String> {
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
            rusqlite::trace::log(1, &"雪".repeat(342));
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
