#[cfg(target_os = "linux")]
mod policy;

#[cfg(target_os = "linux")]
fn main() {
    if std::env::args().skip(1).eq(["--session-worker"]) {
        if worker::run().is_err() {
            std::process::exit(1);
        }
        return;
    }
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args == ["--build-description"] {
        println!("{}", include_str!(concat!(env!("OUT_DIR"), "/build-description.json")));
        return;
    }
    if args == ["--help"] {
        println!(concat!("fathomdb-inspector ", env!("CARGO_PKG_VERSION"), "\nfathomdb-inspector [--no-show-inactive] [--show-erased] [--show-payloads] [--] DATABASE_PATH\nUse --build-description for offline artifact identity.\nDefault: retained inactive/history visible; retained erasure content and telemetry payloads withheld. Qualified Engine startup/worker/shutdown effects may change files. All external writers must stop."));
        return;
    }
    let (policy, path) = match policy::Policy::parse(&args) {
        Ok(value) => value,
        Err(code) => {
            eprintln!("{code}");
            std::process::exit(2);
        }
    };
    if let Err(code) = launch::run(path, policy) {
        eprintln!("{code}");
        std::process::exit(1);
    }
}

#[cfg(test)]
#[path = "../tests/contracts/policy.rs"]
#[cfg(target_os = "linux")]
mod policy_tests;

#[cfg(target_os = "linux")]
mod process;
#[cfg(test)]
#[path = "../tests/contracts/process.rs"]
#[cfg(target_os = "linux")]
mod process_tests;

#[cfg(target_os = "linux")]
mod admission;
#[cfg(test)]
#[path = "../tests/integration/admission.rs"]
#[cfg(target_os = "linux")]
mod admission_tests;
#[cfg(test)]
#[path = "../tests/fixtures/mod.rs"]
#[cfg(target_os = "linux")]
mod fixtures;

#[cfg(target_os = "linux")]
mod session;
#[cfg(test)]
#[path = "../tests/integration/session.rs"]
#[cfg(target_os = "linux")]
mod session_tests;

#[cfg(test)]
#[path = "../tests/integration/records.rs"]
#[cfg(target_os = "linux")]
mod record_tests;
#[cfg(target_os = "linux")]
mod storage;

#[cfg(test)]
#[path = "../tests/integration/operational.rs"]
#[cfg(target_os = "linux")]
mod operational_tests;

#[cfg(test)]
#[path = "../tests/integration/chunks.rs"]
#[cfg(target_os = "linux")]
mod chunk_tests;

#[cfg(test)]
#[path = "../tests/integration/tree.rs"]
#[cfg(target_os = "linux")]
mod tree_tests;

#[cfg(target_os = "linux")]
mod tree;

#[cfg(target_os = "linux")]
mod worker;

#[cfg(target_os = "linux")]
mod http;
#[cfg(test)]
#[path = "../tests/contracts/http.rs"]
#[cfg(target_os = "linux")]
mod http_tests;

#[cfg(test)]
#[path = "../tests/integration/access.rs"]
#[cfg(target_os = "linux")]
mod access_tests;
#[cfg(target_os = "linux")]
mod server;

#[cfg(test)]
#[path = "../tests/integration/associations.rs"]
#[cfg(target_os = "linux")]
mod association_tests;

#[cfg(test)]
#[path = "../tests/contracts/browser_launch.rs"]
#[cfg(target_os = "linux")]
mod browser_launch_tests;

#[cfg(target_os = "linux")]
mod launch;

#[cfg(target_os = "linux")]
mod export;
#[cfg(test)]
#[path = "../tests/contracts/export.rs"]
#[cfg(target_os = "linux")]
mod export_tests;

#[cfg(test)]
#[path = "../tests/integration/disclosure.rs"]
#[cfg(target_os = "linux")]
mod disclosure_tests;

#[cfg(test)]
#[path = "../tests/integration/scope.rs"]
#[cfg(target_os = "linux")]
mod scope_tests;

#[cfg(test)]
#[path = "../tests/integration/preflight_guards.rs"]
#[cfg(target_os = "linux")]
mod preflight_guard_tests;

#[cfg(target_os = "linux")]
mod failure;

#[cfg(test)]
#[path = "../tests/contracts/build.rs"]
#[cfg(target_os = "linux")]
mod build_tests;

#[cfg(target_os = "linux")]
mod window;

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("unsupported_platform: this Inspector build is qualified for Linux only");
    std::process::exit(1);
}
