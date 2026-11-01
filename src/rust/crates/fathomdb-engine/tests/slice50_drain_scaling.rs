#![cfg(feature = "test-hooks")]

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{Engine, InitialState, PreparedWrite, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;

// Frozen AC-013 corpus and deterministic embedder from perf_gates.rs.
struct SeededRng {
    state: u64,
}

impl SeededRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1) }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.state
    }

    fn next_in(&mut self, bound: usize) -> usize {
        (self.next_u64() as usize) % bound
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }
}

/// Vocabulary of synthetic English-like tokens. Size ~1024 keeps the
/// FTS index dense; tokens are short, ASCII, lowercase, deterministic.
fn perf_vocab() -> Vec<String> {
    let mut out = Vec::with_capacity(1024);
    for i in 0..1024 {
        let a = (b'a' + ((i / 26 / 26) % 26) as u8) as char;
        let b = (b'a' + ((i / 26) % 26) as u8) as char;
        let c = (b'a' + (i % 26) as u8) as char;
        out.push(format!("{a}{b}{c}{i:04}"));
    }
    out
}

/// Sample a token index under a Zipfian-ish distribution with shape s=1.0,
/// using inverse-CDF on a precomputed cumulative weight table. Returns an
/// index into the vocabulary in [0, vocab_size).
fn zipf_index(rng: &mut SeededRng, cumulative: &[f64]) -> usize {
    let r = rng.next_f64() * cumulative[cumulative.len() - 1];
    match cumulative.binary_search_by(|w| w.partial_cmp(&r).unwrap_or(std::cmp::Ordering::Equal)) {
        Ok(idx) => idx,
        Err(idx) => idx.min(cumulative.len() - 1),
    }
}

fn zipf_cumulative(vocab_size: usize) -> Vec<f64> {
    let mut cumulative = Vec::with_capacity(vocab_size);
    let mut acc = 0.0_f64;
    for k in 1..=vocab_size {
        acc += 1.0_f64 / k as f64;
        cumulative.push(acc);
    }
    cumulative
}

/// Generate one synthetic chunk body of approximately `target_bytes`
/// (~500 B per ADR) using the Zipfian sampler. Tokens are space-joined.
fn synth_chunk_body(rng: &mut SeededRng, vocab: &[String], cumulative: &[f64]) -> String {
    // Tokens average ~7 chars + 1 separator, target ~500 B => ~63 tokens.
    let mut body = String::with_capacity(512);
    let token_count = 55 + rng.next_in(20);
    for i in 0..token_count {
        if i > 0 {
            body.push(' ');
        }
        body.push_str(&vocab[zipf_index(rng, cumulative)]);
    }
    body
}

#[derive(Clone, Debug)]
struct VaryingEmbedder {
    identity: EmbedderIdentity,
    dim: u32,
}

impl VaryingEmbedder {
    fn new(dim: u32) -> Self {
        Self { identity: EmbedderIdentity::new("varying", "perf-gates-dense", dim), dim }
    }

    fn vector_for(&self, text: &str) -> Vector {
        let dim = self.dim as usize;
        // FNV-1a 64-bit on the input — same hash as the prior sparse
        // version, so seed determinism is unchanged.
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in text.as_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        // Avoid the all-zero xorshift fixed point for empty input.
        if h == 0 {
            h = 0xdeadbeef_cafebabe;
        }
        let mut state = h;
        let mut v = Vec::with_capacity(dim);
        for _ in 0..dim {
            // xorshift64 — full-period, zero-pole-avoidance above.
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            // Map u64 → [-1, 1) via the i32 cast of the low 32 bits.
            let x = ((state as u32) as i32 as f32) / (i32::MAX as f32 + 1.0);
            v.push(x);
        }
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
        for x in &mut v {
            *x /= norm;
        }
        v
    }
}

impl Embedder for VaryingEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        self.identity.clone()
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        Ok(self.vector_for(text))
    }
}

fn digest(connection: &Connection) -> String {
    let mut hash = Sha256::new();
    // Join by write_cursor for a logical projected-state digest, excluding
    // created_at timestamps. Include every other vector column.
    for sql in [
        "SELECT r.rowid,r.write_cursor,r.kind,v.rowid,v.source_type,v.kind,v.status,v.embedding,v.embedding_bin FROM _fathomdb_vector_rows r JOIN vector_default v ON v.rowid=r.rowid ORDER BY r.write_cursor",
        "SELECT write_cursor,state FROM _fathomdb_projection_terminal ORDER BY write_cursor",
        "SELECT key,value FROM _fathomdb_open_state WHERE key='projection_cursor'",
    ] {
        hash.update(sql.as_bytes());
        let mut stmt = connection.prepare(sql).unwrap();
        let columns = stmt.column_count();
        let mut rows = stmt.query([]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            for i in 0..columns {
                let value = row.get_ref(i).unwrap();
                let bytes = match value {
                    rusqlite::types::ValueRef::Null => vec![0],
                    rusqlite::types::ValueRef::Integer(n) => {let mut v=vec![1];v.extend(n.to_le_bytes());v},
                    rusqlite::types::ValueRef::Real(n) => {let mut v=vec![2];v.extend(n.to_le_bytes());v},
                    rusqlite::types::ValueRef::Text(v) => {let mut out=vec![3];out.extend(v);out},
                    rusqlite::types::ValueRef::Blob(v) => {let mut out=vec![4];out.extend(v);out},
                };
                hash.update((bytes.len() as u64).to_le_bytes());
                hash.update(bytes);
            }
        }
    }
    hash.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
#[ignore = "paired release-mode drain qualification; see slice-50/tools/README.md"]
fn slice50_drain_scaling() {
    let n: usize = std::env::var("SLICE50_N").unwrap_or_else(|_| "10000".into()).parse().unwrap();
    let dim: u32 = std::env::var("SLICE50_DIM").unwrap_or_else(|_| "768".into()).parse().unwrap();
    let mode = std::env::var("SLICE50_MODE").unwrap_or_else(|_| "held".into());
    assert!(mode == "held" || mode == "overlap");
    let timeout: u64 = std::env::var("SLICE50_DRAIN_TIMEOUT_MS")
        .unwrap_or_else(|_| "3600000".into())
        .parse()
        .unwrap();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("scaling{SQLITE_SUFFIX}"));
    let engine = Engine::open_with_embedder_for_test(&path, Arc::new(VaryingEmbedder::new(dim)))
        .unwrap()
        .engine;
    engine.configure_vector_kind_for_test("doc").unwrap();
    engine.set_projection_scheduler_frozen_for_test(mode == "held");
    let vocab = perf_vocab();
    let cumulative = zipf_cumulative(vocab.len());
    let mut rng = SeededRng::new(0x0AC0_13D0_13D0);
    let start = Instant::now();
    for offset in (0..n).step_by(1024) {
        let writes = (offset..n.min(offset + 1024))
            .map(|_| PreparedWrite::Node {
                kind: "doc".into(),
                body: synth_chunk_body(&mut rng, &vocab, &cumulative),
                source_id: SourceId::new("test:fixture").unwrap(),
                logical_id: None,
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect::<Vec<_>>();
        engine.write(&writes).unwrap();
    }
    let write_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    engine.set_projection_scheduler_frozen_for_test(false);
    engine.drain(timeout).unwrap();
    let drain_ms = start.elapsed().as_secs_f64() * 1000.0;
    let raw = Connection::open(path).unwrap();
    let vector_rows: u64 =
        raw.query_row("SELECT COUNT(*) FROM _fathomdb_vector_rows", [], |r| r.get(0)).unwrap();
    assert_eq!(vector_rows, n as u64);
    let projection_cursor: u64 = raw
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM _fathomdb_open_state WHERE key='projection_cursor'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let sqlite_version: String =
        raw.query_row("SELECT sqlite_version()", [], |r| r.get(0)).unwrap();
    let digest = digest(&raw);
    println!(
        "SLICE50_DRAIN {}",
        serde_json::json!({"n":n,"dim":dim,"mode":mode,"write_ms":write_ms,"drain_ms":drain_ms,"us_per_vector":drain_ms*1000.0/n as f64,"sqlite_version":sqlite_version,"vector_rows":vector_rows,"projection_cursor":projection_cursor,"digest":digest})
    );
    engine.close().unwrap();
}

fn digest_fixture() -> (TempDir, Engine, Connection) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("digest{SQLITE_SUFFIX}"));
    let engine = Engine::open_with_embedder_for_test(&path, Arc::new(VaryingEmbedder::new(8)))
        .unwrap()
        .engine;
    engine.configure_vector_kind_for_test("doc").unwrap();
    let raw = Connection::open(path).unwrap();
    (dir, engine, raw)
}

#[test]
fn digest_detects_orphan_vector_row() {
    let (_dir, _engine, raw) = digest_fixture();
    let before = digest(&raw);
    let vector: Vec<u8> = [1.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect();
    raw.execute("INSERT INTO vector_default(rowid,embedding,embedding_bin,source_type,kind,created_at,status) VALUES(1000,?1,vec_quantize_binary(?1),'document','doc',0,'')",[vector]).unwrap();
    assert_ne!(digest(&raw), before, "unregistered vec0 row must affect whole-store digest");
}

#[test]
fn digest_detects_dangling_vector_mapping() {
    let (_dir, _engine, raw) = digest_fixture();
    let before = digest(&raw);
    raw.execute(
        "INSERT INTO _fathomdb_vector_rows(rowid,kind,write_cursor) VALUES(1000,'doc',1000)",
        [],
    )
    .unwrap();
    assert_ne!(digest(&raw), before, "dangling mapping must affect whole-store digest");
}
