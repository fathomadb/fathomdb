//! Private G measurement host; the separate TC5 executable and protocol are unchanged.

#[path = "../../fathomdb-sdk/examples/slice30/g_fixture.rs"]
mod g_fixture;

use g_fixture::budget_clock;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "usage: slice30-g-measurement cell.json owned-output");
    let config = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    if budget_clock::phase(&config, std::path::Path::new(&args[2]), "entry").is_err() {
        return;
    }
    #[cfg(feature = "slice30-g-observer")]
    let observer: Option<g_fixture::CoreObserver> = Some((
        fathomdb_engine::Engine::slice30_observe_queries_for_test,
        fathomdb_engine::Engine::take_slice30_measurements_for_test,
        fathomdb_engine::Engine::slice30_measurement_stats_for_test,
    ));
    #[cfg(not(feature = "slice30-g-observer"))]
    let observer: Option<g_fixture::CoreObserver> = None;
    if observer.is_some() {
        g_fixture::run_observed(&config, std::path::Path::new(&args[2]), observer);
    } else {
        g_fixture::run(&config, std::path::Path::new(&args[2]));
    }
}
