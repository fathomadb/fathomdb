//! Private G measurement host; the separate TC5 executable and protocol are unchanged.

#[path = "../../fathomdb-sdk/examples/slice30/g_fixture.rs"]
mod g_fixture;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "usage: slice30-g-measurement cell.json owned-output");
    let config = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    g_fixture::run(&config, std::path::Path::new(&args[2]));
}
