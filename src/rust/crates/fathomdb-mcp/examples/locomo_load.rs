//! Load LoCoMo into one FathomDB store per conversation for the MCP
//! hybrid-versus-text evaluation (`dev/experiments/mcp-hybrid-vs-keyword/`).
//!
//! `locomo_load <locomo10.json> <out-dir>` writes `<out-dir>/conv<i>.sqlite`
//! for each conversation: one `note` node per dialog turn, logical id
//! `<i>:<dia_id>`, body `[<session date>] <speaker>: <text>` (plus the shared
//! image caption when the turn has one). Each store declares one searchable
//! FTS + vector projection, is opened with the default embedder (device from
//! `FATHOMDB_EMBED_DEVICE`), is drained, and is then checked for embedding
//! completeness: readiness `ready` with nothing pending, and a vector row for
//! every node (read-only SQLite count after close). Any shortfall exits 1.

use std::path::{Path, PathBuf};
use std::time::Instant;

use fathomdb_sdk::{
    read, EmbeddingReadinessState, Engine, InitialState, OpenOptions, PreparedWrite, ProjectionFts,
    ProjectionRole, ProjectionSpec, ProjectionVector, SourceId,
};
use serde_json::Value;

const KIND: &str = "note";

struct Turn {
    logical_id: String,
    source: String,
    body: String,
}

fn session_number(key: &str) -> Option<u32> {
    key.strip_prefix("session_")?.parse().ok()
}

fn turns(index: usize, conversation: &Value) -> Vec<Turn> {
    let object = conversation.as_object().expect("conversation object");
    let mut sessions: Vec<(u32, &Value)> = object
        .iter()
        .filter_map(|(key, value)| session_number(key).map(|number| (number, value)))
        .collect();
    sessions.sort_by_key(|(number, _)| *number);
    let mut out = Vec::new();
    for (number, session) in sessions {
        let date = object
            .get(&format!("session_{number}_date_time"))
            .and_then(Value::as_str)
            .unwrap_or("");
        for turn in session.as_array().expect("session array") {
            let dia_id = turn["dia_id"].as_str().expect("dia_id");
            let speaker = turn["speaker"].as_str().expect("speaker");
            let text = turn["text"].as_str().expect("text");
            let mut body = if date.is_empty() {
                format!("{speaker}: {text}")
            } else {
                format!("[{date}] {speaker}: {text}")
            };
            if let Some(caption) = turn.get("blip_caption").and_then(Value::as_str) {
                body.push_str(&format!(" [shared image: {caption}]"));
            }
            out.push(Turn {
                logical_id: format!("{index}:{dia_id}"),
                source: format!("locomo-conv{index}-session{number}"),
                body,
            });
        }
    }
    out
}

fn spec() -> ProjectionSpec {
    ProjectionSpec {
        name: "turn".into(),
        roles: [ProjectionRole::Searchable].into_iter().collect(),
        fts: Some(ProjectionFts { tokenizer: None }),
        vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
        source: None,
    }
}

fn vector_coverage(path: &Path) -> (u64, u64) {
    let connection =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("open read-only");
    let nodes: u64 = connection
        .query_row("SELECT COUNT(*) FROM canonical_nodes WHERE kind = ?1", [KIND], |row| row.get(0))
        .expect("count nodes");
    let embedded: u64 = connection
        .query_row(
            "SELECT COUNT(*) FROM canonical_nodes n JOIN _fathomdb_vector_rows v \
             ON v.write_cursor = n.write_cursor WHERE n.kind = ?1",
            [KIND],
            |row| row.get(0),
        )
        .expect("count vectors");
    (nodes, embedded)
}

fn load(index: usize, conversation: &Value, out_dir: &Path) -> Result<(), String> {
    let path: PathBuf = out_dir.join(format!("conv{index}.sqlite"));
    if path.exists() {
        return Err(format!("{} exists; use an empty out-dir", path.display()));
    }
    let turns = turns(index, conversation);
    let started = Instant::now();
    let engine =
        Engine::open(&path, OpenOptions { use_default_embedder: true, ..Default::default() })
            .map_err(|error| format!("open {}: {error}", path.display()))?;
    let delta = engine.configure_projections(&[spec()], &[]).map_err(|e| e.to_string())?;
    if !delta.vector_unsupported_kinds.is_empty() {
        return Err(format!(
            "kinds the vector arm cannot embed: {:?}",
            delta.vector_unsupported_kinds
        ));
    }
    let writes: Vec<PreparedWrite> = turns
        .iter()
        .map(|turn| PreparedWrite::Node {
            kind: KIND.into(),
            body: turn.body.clone(),
            source_id: SourceId::new(&turn.source).expect("source id"),
            logical_id: Some(turn.logical_id.clone()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect();
    for chunk in writes.chunks(256) {
        engine.write(chunk).map_err(|error| format!("write: {error}"))?;
    }
    engine.drain(600_000).map_err(|error| format!("drain: {error}"))?;
    let readiness = read::embedding_readiness(&engine).map_err(|e| e.to_string())?;
    let projections = read::projections(&engine).map_err(|e| e.to_string())?;
    let dense = projections
        .iter()
        .find_map(|spec| spec.vector.as_ref().and_then(|vector| vector.dense_readiness));
    engine.close().map_err(|error| format!("close: {error}"))?;
    let (nodes, embedded) = vector_coverage(&path);
    println!(
        "{}",
        serde_json::json!({
            "store": format!("conv{index}"),
            "turns": turns.len(),
            "nodes": nodes,
            "vector_rows": embedded,
            "readiness_state": format!("{:?}", readiness.state),
            "usable_embedder": readiness.usable_embedder,
            "pending": readiness.pending_count,
            "dense_readiness": format!("{dense:?}"),
            "seconds": started.elapsed().as_secs_f64(),
        })
    );
    let complete = readiness.state == EmbeddingReadinessState::Ready
        && readiness.usable_embedder
        && readiness.pending_count == 0
        && nodes == turns.len() as u64
        && embedded == nodes;
    if complete {
        Ok(())
    } else {
        Err(format!("conv{index}: embedding incomplete"))
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(input), Some(out_dir)) = (args.next(), args.next()) else {
        eprintln!("usage: locomo_load <locomo10.json> <out-dir>");
        std::process::exit(2);
    };
    let text = std::fs::read_to_string(&input).expect("read LoCoMo json");
    let data: Value = serde_json::from_str(&text).expect("parse LoCoMo json");
    let out_dir = PathBuf::from(out_dir);
    std::fs::create_dir_all(&out_dir).expect("create out-dir");
    for (index, sample) in data.as_array().expect("array of samples").iter().enumerate() {
        if let Err(message) = load(index, &sample["conversation"], &out_dir) {
            eprintln!("locomo_load: {message}");
            std::process::exit(1);
        }
    }
}
