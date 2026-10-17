//! `fathomdb-mcp` binary: `fathomdb-mcp --config <path.json> [--check-config]`.
//!
//! stdout carries MCP JSON-RPC and nothing else; every diagnostic goes to
//! stderr (MR-002). Exit codes: 0 normal shutdown, 2 invalid configuration or
//! arguments, 1 any other startup failure.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use fathomdb_mcp::config::Config;
use fathomdb_mcp::Adapter;
use rmcp::ServiceExt;

const USAGE: &str = "usage: fathomdb-mcp --config <path.json> [--check-config]";

struct Args {
    config: PathBuf,
    check_only: bool,
}

fn parse_args(raw: impl Iterator<Item = OsString>) -> Result<Args, String> {
    let mut config = None;
    let mut check_only = false;
    let mut raw = raw;
    while let Some(arg) = raw.next() {
        match arg.to_str() {
            Some("--config") => {
                let value = raw.next().ok_or("--config needs a path")?;
                config = Some(PathBuf::from(value));
            }
            Some("--check-config") => check_only = true,
            _ => return Err(format!("unexpected argument {arg:?}")),
        }
    }
    let config = config.ok_or("--config is required")?;
    Ok(Args { config, check_only })
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args_os().skip(1)) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("fathomdb-mcp: {message}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let config = match Config::load(&args.config) {
        Ok(config) => config,
        Err(error) => {
            if args.check_only {
                eprintln!("{}", serde_json::json!({"valid": false, "error": error.message}));
            } else {
                eprintln!("fathomdb-mcp: invalid configuration: {error}");
            }
            return ExitCode::from(2);
        }
    };
    if args.check_only {
        let report = serde_json::json!({"valid": true, "stores": config.addressable_names()});
        eprintln!("{report}");
        return ExitCode::SUCCESS;
    }
    init_tracing();
    serve(config)
}

// stderr only: stdout belongs to the protocol (MR-002.3). The level comes
// from FATHOMDB_MCP_LOG (error, warn, info, debug, trace; default warn).
fn init_tracing() {
    let level = match std::env::var("FATHOMDB_MCP_LOG").as_deref() {
        Ok("error") => tracing::Level::ERROR,
        Ok("info") => tracing::Level::INFO,
        Ok("debug") => tracing::Level::DEBUG,
        Ok("trace") => tracing::Level::TRACE,
        _ => tracing::Level::WARN,
    };
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .with_max_level(level)
        .init();
}

fn serve(config: Config) -> ExitCode {
    let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::error!(%error, "cannot start the async runtime");
            return ExitCode::from(1);
        }
    };
    let code = runtime.block_on(async {
        let adapter = match Adapter::new(config) {
            Ok(adapter) => adapter,
            Err(error) => {
                tracing::error!(%error, "cannot start the adapter");
                return ExitCode::from(1);
            }
        };
        let running = match adapter.connection().serve(rmcp::transport::stdio()).await {
            Ok(running) => running,
            Err(error) => {
                tracing::error!(%error, "MCP initialization failed");
                adapter.shutdown().await;
                return ExitCode::from(1);
            }
        };
        let _ = running.waiting().await;
        adapter.shutdown().await;
        ExitCode::SUCCESS
    });
    // Abandoned engine calls may still hold blocking threads; do not let them
    // delay exit indefinitely.
    runtime.shutdown_timeout(std::time::Duration::from_secs(5));
    code
}
