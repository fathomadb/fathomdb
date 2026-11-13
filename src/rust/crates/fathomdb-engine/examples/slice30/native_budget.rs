//! Private native phase-budget control using the real None-provider database route.

use fathomdb_engine::{EmbedderChoice, Engine};
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn run_control(_config: &Value, out: &Path) {
    std::fs::create_dir_all(out).unwrap();
    let path = out.join("store.sqlite");
    let engine = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap().engine;
    engine.close().unwrap();
    std::fs::write(
        out.join("budget.json"),
        serde_json::to_vec_pretty(
            &json!({"open_calls":1,"sqlite_created":path.exists(),"disposition":"executed"}),
        )
        .unwrap(),
    )
    .unwrap();
}
