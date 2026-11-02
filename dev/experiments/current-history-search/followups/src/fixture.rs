use crate::ledger::Result;
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::*;
use std::{path::Path, sync::Arc};
struct Fixed;
impl Embedder for Fixed {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("poc", "v1", 8)
    }
    fn embed(&self, _: &str) -> std::result::Result<Vector, EmbedderError> {
        Ok(vec![1.; 8])
    }
}
/// Actual engine fixture; historical rows come from governed supersession writes.
pub fn build(path: &Path, family: &str, history: usize, bytes: usize, n: usize) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    let engine = Engine::open_with_embedder_for_test(path, Arc::new(Fixed))?.engine;
    engine.execute_for_test("CREATE TABLE poc_versions(cursor INTEGER PRIMARY KEY,ordinal INTEGER NOT NULL,version INTEGER NOT NULL); CREATE INDEX poc_versions_selection ON poc_versions(version,ordinal);")?;
    let _ = (family, history, bytes, n);
    engine.close()?;
    Ok(())
}
/// Fixed UTF-8 byte count; ordinal and version distinguish bodies without changing length.
pub fn payload(bytes: usize, ordinal: usize, version: usize) -> String {
    let mut text = format!("needle owner{ordinal:05} revision{version:02} café running alpha ");
    while text.len() + 7 <= bytes {
        text.push_str(" alpha ");
    }
    while text.len() < bytes {
        text.push('x');
    }
    assert_eq!(text.len(), bytes);
    text
}
/// Exact selected statistical family; diagnostic corrected/prior meaning is fixture-owned.
pub fn queries(family: &str, history: usize, view: &str) -> (String, String) {
    let table = if family == "edge" { "canonical_edges" } else { "canonical_nodes" };
    let active = if family == "edge" { "1" } else { "n.state='active'" };
    let chosen = match view {
        "current" => format!("v.version={history} AND {active}"),
        "corrected" => {
            format!("v.version=CASE WHEN v.ordinal<10 THEN 0 ELSE {history} END AND {active}")
        }
        "prior" => format!("v.version={} AND {active}", history / 2),
        "inactive" => format!("v.version={history}"),
        "superseded" => active.into(),
        "archive" | "both" => "1".into(),
        _ => panic!("unknown view"),
    };
    if family == "property" {
        (format!("SELECT n.write_cursor*2+p.name,json_extract(n.body,CASE p.name WHEN 0 THEN '$.p1' ELSE '$.p2' END),p.name=0 FROM {table} n JOIN poc_versions v ON v.cursor=n.write_cursor CROSS JOIN (SELECT 0 name UNION ALL SELECT 1) p WHERE {chosen}"),format!("SELECT body FROM {table} WHERE write_cursor=(?1/2)"))
    } else {
        (format!("SELECT n.write_cursor,n.body,1 FROM {table} n JOIN poc_versions v ON v.cursor=n.write_cursor WHERE {chosen}"),format!("SELECT body FROM {table} WHERE write_cursor=?1"))
    }
}
