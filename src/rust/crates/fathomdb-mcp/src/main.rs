//! `fathomdb-mcp` binary: `fathomdb-mcp --config <path.json> [--check-config]`.
//!
//! stdout carries MCP JSON-RPC and nothing else; every diagnostic goes to
//! stderr (MR-002). Exit codes: 0 normal shutdown, 2 invalid configuration or
//! arguments, 1 any other startup failure.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use fathomdb_mcp::config::Config;

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
    eprintln!("fathomdb-mcp: serving is not available in this build");
    ExitCode::from(1)
}
