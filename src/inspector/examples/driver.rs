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
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let scratch = tempfile::tempdir().unwrap();
    let s = server::Server::start_with_executable(
        scratch.path(),
        Some(args[0].clone()),
        policy::Policy::parse(&args[2..]).unwrap().0,
        std::path::Path::new(&args[1]),
    )
    .unwrap();
    println!(
        "{}",
        serde_json::json!({"address":s.address,"bootstrap":s.bootstrap,"build":process::BUILD})
    );
    #[cfg(feature = "fixture-control")]
    use std::io::BufRead;
    #[cfg(feature = "fixture-control")]
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        if line == "close" {
            break;
        }
        #[cfg(feature = "fixture-control")]
        {
            println!("{}", s.fixture(serde_json::from_str(&line).unwrap()));
        }
    }
    #[cfg(not(feature = "fixture-control"))]
    {
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
    }
    drop(s);
}

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
