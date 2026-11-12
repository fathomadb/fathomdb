//! Bounded Slice30 normal G API measurement adapter; see the indexed README.

#[path = "slice30/g_fixture.rs"]
mod g_fixture;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "usage: slice30_g_mean cell.json owned-output");
    let config = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    g_fixture::run(&config, std::path::Path::new(&args[2]));
}
