#![allow(dead_code)]
#[path = "../src/admission.rs"]
#[cfg(target_os = "linux")]
mod admission;
#[path = "../src/http.rs"]
#[cfg(target_os = "linux")]
mod http;
#[path = "../src/policy.rs"]
#[cfg(target_os = "linux")]
mod policy;
#[path = "../src/process.rs"]
#[cfg(target_os = "linux")]
mod process;
#[path = "../src/server.rs"]
#[cfg(target_os = "linux")]
mod server;
#[path = "../src/session.rs"]
#[cfg(target_os = "linux")]
mod session;
#[path = "../src/storage.rs"]
#[cfg(target_os = "linux")]
mod storage;
#[path = "../src/tree.rs"]
#[cfg(target_os = "linux")]
mod tree;
#[cfg(target_os = "linux")]
fn main() {
    assert!(std::env::args().skip(1).eq(["--session-worker"]));
    std::thread::sleep(std::time::Duration::from_secs(60));
    if worker::run().is_err() {
        std::process::exit(1);
    }
}
#[path = "../src/worker.rs"]
#[cfg(target_os = "linux")]
mod worker;

#[path = "../src/export.rs"]
#[cfg(target_os = "linux")]
mod export;

#[path = "../src/failure.rs"]
#[cfg(target_os = "linux")]
mod failure;

#[cfg(test)]
#[path = "../tests/fixtures/mod.rs"]
#[cfg(target_os = "linux")]
mod fixtures;

#[path = "../src/window.rs"]
#[cfg(target_os = "linux")]
mod window;

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("unsupported_platform: this Inspector build is qualified for Linux only");
    std::process::exit(1);
}
