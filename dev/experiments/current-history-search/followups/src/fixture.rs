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
        let connection = rusqlite::Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let complete = connection
            .query_row("SELECT complete FROM poc_fixture_complete", [], |r| r.get::<_, bool>(0))
            .unwrap_or(false);
        return if complete {
            Ok(())
        } else {
            Err("incomplete fixture retained; refuse reuse".into())
        };
    }
    let engine = Engine::open_with_embedder_for_test(path, Arc::new(Fixed))?.engine;
    engine.execute_for_test("CREATE TABLE poc_versions(cursor INTEGER PRIMARY KEY,ordinal INTEGER NOT NULL,version INTEGER NOT NULL,commit_cut INTEGER NOT NULL); CREATE INDEX poc_versions_selection ON poc_versions(version,ordinal);")?;
    if family == "edge" {
        for start in (0..=n).step_by(1000) {
            let endpoints = (start..(start + 1000).min(n + 1))
                .map(|i| PreparedWrite::Node {
                    kind: "entity".into(),
                    body: format!("endpoint {i}"),
                    source_id: SourceId::new("fixture").unwrap(),
                    logical_id: Some(format!("endpoint-{i}")),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                })
                .collect::<Vec<_>>();
            engine.write(&endpoints)?;
        }
    }
    for version in 0..=history {
        for start in (0..n).step_by(1000) {
            let mut writes = Vec::new();
            for ordinal in start..(start + 1000).min(n) {
                let scalar =
                    payload(bytes, ordinal, if version == history { 0 } else { version + 1 });
                let source_id = SourceId::new("fixture")?;
                let logical_id = Some(format!("owner-{ordinal}"));
                writes.push(if family=="edge" {PreparedWrite::Edge{kind:"rel".into(),from:format!("endpoint-{ordinal}"),to:format!("endpoint-{}",ordinal+1),source_id,logical_id,body:Some(scalar),t_valid:None,t_invalid:None,confidence:None,extractor_model_id:None,temporal_fallback:None}} else {PreparedWrite::Node{kind:"doc".into(),body:if family=="property"{serde_json::json!({"text":"fixed owner","p1":scalar,"p2":payload(bytes,ordinal,if version==history{0}else{version+1})}).to_string()}else{scalar},source_id,logical_id,state:if ordinal>=n-n/100 {InitialState::Pending}else{InitialState::Active},reason:None,valid_from:None,valid_until:None}});
            }
            let receipt = engine.write(&writes)?;
            let values = receipt
                .row_cursors
                .iter()
                .enumerate()
                .map(|(i, c)| format!("({c},{},{version},{})", start + i, receipt.cursor))
                .collect::<Vec<_>>()
                .join(",");
            engine.execute_for_test(&format!("INSERT INTO poc_versions VALUES {values}"))?;
        }
        engine.drain(600_000)?;
    }
    // Import a completed fixture from real batch high-water receipts; this is not a production write protocol.
    engine.execute_for_test(&format!("CREATE TABLE poc_temporal_fragments(cursor INTEGER NOT NULL,ordinal INTEGER NOT NULL,v0 INTEGER NOT NULL,v1 INTEGER NOT NULL,k0 INTEGER NOT NULL,k1 INTEGER);
        INSERT INTO poc_temporal_fragments SELECT cursor,ordinal,CASE WHEN version={history} AND {history}>0 AND ordinal<10 THEN 20 ELSE 0 END,100,commit_cut,LEAD(commit_cut) OVER(PARTITION BY ordinal ORDER BY version) FROM poc_versions;
        INSERT INTO poc_temporal_fragments SELECT old.cursor,old.ordinal,0,20,new.commit_cut,NULL FROM poc_versions old JOIN poc_versions new ON old.ordinal=new.ordinal AND new.version={history} WHERE {history}>0 AND old.version={history}-1 AND old.ordinal<10;
        CREATE INDEX poc_temporal_selection ON poc_temporal_fragments(cursor,k0,k1,v0,v1);"))?;
    engine.execute_for_test("CREATE TABLE poc_fixture_complete(complete INTEGER NOT NULL); INSERT INTO poc_fixture_complete VALUES(1);")?;
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
    let temporal = |valid: i64, known: String| {
        format!("EXISTS(SELECT 1 FROM poc_temporal_fragments f WHERE f.cursor=n.write_cursor AND f.v0<={valid} AND {valid}<f.v1 AND f.k0<=({known}) AND (f.k1 IS NULL OR ({known})<f.k1))")
    };
    let latest = "SELECT MAX(commit_cut) FROM poc_versions".to_string();
    let chosen = match view {
        "current" => format!("{} AND {active}", temporal(30, latest)),
        "corrected" => format!("{} AND {active}", temporal(10, latest)),
        "prior" => format!(
            "{} AND {active}",
            temporal(
                10,
                format!("SELECT MAX(commit_cut) FROM poc_versions WHERE version={}", history / 2)
            )
        ),
        "inactive" => "n.superseded_at IS NULL".into(),
        "superseded" => active.into(),
        "archive" | "both" => "1".into(),
        _ => panic!("unknown view"),
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock before epoch")
        .as_secs();
    let authority = include_str!("source_authority.sql")
        .replace("f.revision", "n.write_cursor")
        .replace("?4", &now.to_string());
    let chosen = format!("({chosen}) AND ({authority})");
    if family == "property" {
        (format!("SELECT n.write_cursor*2+p.name,json_extract(n.body,CASE p.name WHEN 0 THEN '$.p1' ELSE '$.p2' END),p.name=0 FROM {table} n JOIN poc_versions v ON v.cursor=n.write_cursor CROSS JOIN (SELECT 0 name UNION ALL SELECT 1) p WHERE {chosen}"),format!("SELECT body FROM {table} WHERE write_cursor=(?1/2)"))
    } else {
        (format!("SELECT n.write_cursor,n.body,1 FROM {table} n JOIN poc_versions v ON v.cursor=n.write_cursor WHERE {chosen}"),format!("SELECT body FROM {table} WHERE write_cursor=?1"))
    }
}
