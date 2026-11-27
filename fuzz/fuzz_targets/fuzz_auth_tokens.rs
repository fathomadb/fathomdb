#![no_main]

use std::sync::{Mutex, OnceLock};

use arbitrary::Unstructured;
use fathomdb_engine::{
    decode_dependency_trace_result_v1, decode_graph_expand_request_v1,
    decode_graph_expand_result_v1, EmbedderChoice, Engine, EngineConfig, EvidenceRefV1,
    EvidenceResolveManyRequestV1, EvidenceResolveRequestV1, EvidenceSearchRequestV1,
    FrozenReadContextV1, GraphEvidenceRefV1, GraphEvidenceResolveRequestV1, GraphExpandRequestV1,
    GraphReadContextV1, GraphSeedV1, InitialState, PageCursor, PageRequestV1, PreparedWrite,
    ReadContextV1, ReadView, ScopePolicy, SearchFilter, SourceId, TraversalDirection,
};
use libfuzzer_sys::fuzz_target;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const FROZEN_DOMAIN: &[u8] = b"fathomdb.frozen-read.v1\0";
const EVIDENCE_DOMAIN: &[u8] = b"fathomdb.evidence-ref.v1\0";
const GRAPH_STREAM_DOMAIN: &[u8] = b"fathomdb.graph-evidence.stream.v1\0";
const GRAPH_MAC_DOMAIN: &[u8] = b"fathomdb.graph-evidence.mac.v1\0";
const PAGE_CURSOR_DOMAIN: &[u8] = b"fathomdb.page-cursor.v1\0";

struct AuthFixture {
    _dir: TempDir,
    engine: Engine,
    scoped_engine: Engine,
    valid_frozen: FrozenReadContextV1,
    database_id: String,
    hmac_key: Vec<u8>,
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out.push(((hi << 4) | lo) as u8);
    }
    Some(out)
}

fn hmac_sha256(key: &[u8], domain: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut block = [0_u8; 64];
    let n = key.len().min(64);
    block[..n].copy_from_slice(&key[..n]);
    let mut inner_pad = [0x36_u8; 64];
    let mut outer_pad = [0x5c_u8; 64];
    for i in 0..64 {
        inner_pad[i] ^= block[i];
        outer_pad[i] ^= block[i];
    }
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(domain);
    inner.update(payload);
    let digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(digest);
    outer.finalize().into()
}

fn frame_graph_token(key: &[u8], nonce: &[u8; 16], payload: &[u8]) -> String {
    let mut ciphertext = Vec::with_capacity(payload.len());
    for (block_idx, chunk) in payload.chunks(32).enumerate() {
        let mut input = Vec::with_capacity(20);
        input.extend_from_slice(nonce);
        input.extend_from_slice(&(block_idx as u32).to_be_bytes());
        let mask = hmac_sha256(key, GRAPH_STREAM_DOMAIN, &input);
        ciphertext.extend(chunk.iter().zip(mask).map(|(b, m)| b ^ m));
    }
    let mut authenticated = Vec::with_capacity(8 + nonce.len() + ciphertext.len());
    authenticated.extend_from_slice(b"fdbgev1.");
    authenticated.extend_from_slice(nonce);
    authenticated.extend_from_slice(&ciphertext);
    let mac = hmac_sha256(key, GRAPH_MAC_DOMAIN, &authenticated);
    let mut framed = Vec::with_capacity(nonce.len() + ciphertext.len() + mac.len());
    framed.extend_from_slice(nonce);
    framed.extend_from_slice(&ciphertext);
    framed.extend_from_slice(&mac);
    format!("fdbgev1.{}", hex_encode(&framed))
}

fn fixture() -> &'static Mutex<AuthFixture> {
    static FIXTURE: OnceLock<Mutex<AuthFixture>> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let dir = TempDir::new().expect("tempdir for fuzz_auth_tokens");
        let path = dir.path().join("auth_fuzz.sqlite");
        let scoped_path = dir.path().join("auth_scoped.sqlite");

        let engine = Engine::open_with_choice(&path, EmbedderChoice::None)
            .expect("open auth fixture engine")
            .engine;
        let src = SourceId::new("auth-src").expect("valid source");
        let _ = engine.write(&[PreparedWrite::Node {
            kind: "note".to_string(),
            body: "auth token fuzz seed document".to_string(),
            source_id: src,
            logical_id: Some("n1".to_string()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        }]);
        let rc = ReadContextV1::new(ReadView::default(), SearchFilter::default())
            .expect("valid read context");
        let valid_frozen = engine.freeze_read_context(&rc).expect("mint frozen context");

        // Extract DB-local identity and HMAC key from _fathomdb_open_state so the
        // fuzz target can craft MAC-valid tokens that penetrate directly into binary
        // payload decoders (including 0x53 0x39 trailing-tag parsing).
        let raw_conn = Connection::open(&path).expect("open raw sqlite for key extraction");
        let database_id: String = raw_conn
            .query_row(
                "SELECT value FROM _fathomdb_open_state WHERE key = '_fathomdb_database_id'",
                [],
                |row| row.get(0),
            )
            .expect("read _fathomdb_database_id");
        let key_hex: String = raw_conn
            .query_row(
                "SELECT value FROM _fathomdb_open_state WHERE key = '_fathomdb_read_context_key'",
                [],
                |row| row.get(0),
            )
            .expect("read _fathomdb_read_context_key");
        let hmac_key = hex_decode(&key_hex).expect("decode hex key");
        drop(raw_conn);

        let scoped_engine = Engine::open_with_choice_and_config(
            &scoped_path,
            EmbedderChoice::None,
            EngineConfig::default().with_scope_policy(ScopePolicy::single("tenant:a")),
        )
        .expect("open scoped auth fixture")
        .engine;

        Mutex::new(AuthFixture {
            _dir: dir,
            engine,
            scoped_engine,
            valid_frozen,
            database_id,
            hmac_key,
        })
    })
}

fn encode_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_be_bytes());
}
fn encode_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_be_bytes());
}
fn encode_str(out: &mut Vec<u8>, s: &str) {
    encode_u32(out, s.len() as u32);
    out.extend_from_slice(s.as_bytes());
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // Exercise wire decoders on raw bytes directly.
    let _ = decode_graph_expand_request_v1(data);
    let _ = decode_graph_expand_result_v1(data);
    let _ = decode_dependency_trace_result_v1(data);

    let mut u = Unstructured::new(data);
    let mode = u.int_in_range(0..=5).unwrap_or(0);
    let guard = fixture().lock().unwrap_or_else(|p| p.into_inner());

    match mode {
        0 => {
            // Raw malformed FrozenReadContextV1 tokens (prefix, hex, MAC, oversize).
            let raw_token: String = u.arbitrary().unwrap_or_default();
            let mut ctx = guard.valid_frozen.clone();
            ctx.schema_version =
                if u.arbitrary().unwrap_or(true) { 1 } else { u.arbitrary().unwrap_or(0) };
            ctx.token = raw_token;
            let page = PageRequestV1 { schema_version: 1, limit: 10, cursor: None };
            let _ = guard.engine.read_canonical_page("note", &ctx, &page);
            let _ = guard.scoped_engine.read_canonical_page("note", &ctx, &page);
        }
        1 => {
            // MAC-authenticated arbitrary & structured payloads for FrozenReadContextV1,
            // specifically targeting `decode_binding` + `0x53 0x39` trailing-tag parsing
            // (`rem` in {0, 32, 34, 66, arbitrary}).
            let payload: Vec<u8> = if u.arbitrary().unwrap_or(false) {
                let mut buf = Vec::new();
                encode_u32(&mut buf, if u.arbitrary().unwrap_or(true) { 1 } else { 2 });
                if u.arbitrary().unwrap_or(true) {
                    encode_str(&mut buf, &guard.database_id);
                } else {
                    let fake_id: String = u.arbitrary().unwrap_or_default();
                    encode_str(&mut buf, &fake_id);
                }
                buf.extend_from_slice(&u.arbitrary::<i64>().unwrap_or(0).to_be_bytes());
                for _ in 0..4 {
                    encode_u64(&mut buf, u.arbitrary().unwrap_or(0));
                }
                // 3 x 32-byte digests (registry, serving, context)
                let digests: [u8; 96] = u.arbitrary().unwrap_or([0; 96]);
                buf.extend_from_slice(&digests);
                // Optional trailing scope_digest + 0x53 0x39 surface_tags
                match u.int_in_range(0..=5).unwrap_or(0) {
                    0 => {}
                    1 => {
                        let sd: [u8; 32] = u.arbitrary().unwrap_or([0; 32]);
                        buf.extend_from_slice(&sd);
                    }
                    2 => {
                        buf.extend_from_slice(&[0x53, 0x39]);
                        let tags: [u8; 32] = u.arbitrary().unwrap_or([0; 32]);
                        buf.extend_from_slice(&tags);
                    }
                    3 => {
                        let sd: [u8; 32] = u.arbitrary().unwrap_or([0; 32]);
                        buf.extend_from_slice(&sd);
                        buf.extend_from_slice(&[0x53, 0x39]);
                        let tags: [u8; 32] = u.arbitrary().unwrap_or([0; 32]);
                        buf.extend_from_slice(&tags);
                    }
                    4 => {
                        // Wrong magic at 34-byte trailer
                        let wrong: [u8; 34] = u.arbitrary().unwrap_or([0; 34]);
                        buf.extend_from_slice(&wrong);
                    }
                    _ => {
                        let extra: Vec<u8> = u.arbitrary().unwrap_or_default();
                        buf.extend_from_slice(&extra[..extra.len().min(128)]);
                    }
                }
                buf
            } else {
                let raw: Vec<u8> = u.arbitrary().unwrap_or_default();
                raw[..raw.len().min(480)].to_vec()
            };

            let mac = hmac_sha256(&guard.hmac_key, FROZEN_DOMAIN, &payload);
            let mut ctx = guard.valid_frozen.clone();
            ctx.token = format!("fdbfr1.{}.{}", hex_encode(&payload), hex_encode(&mac));

            let page = PageRequestV1 { schema_version: 1, limit: 10, cursor: None };
            let _ = guard.engine.read_canonical_page("note", &ctx, &page);
            let req = EvidenceSearchRequestV1::new("auth".to_string(), ctx.clone());
            let _ = guard.engine.search_with_evidence(&req);
            // Also test scope-binding mismatch against scoped_engine.
            let _ = guard.scoped_engine.read_canonical_page("note", &ctx, &page);
        }
        2 => {
            // EvidenceRefV1: both raw malformed strings and MAC-authenticated fuzzed payloads.
            let token = if u.arbitrary().unwrap_or(true) {
                let raw: Vec<u8> = u.arbitrary().unwrap_or_default();
                let payload = &raw[..raw.len().min(900)];
                let mac = hmac_sha256(&guard.hmac_key, EVIDENCE_DOMAIN, payload);
                format!("fdbev1.{}.{}", hex_encode(payload), hex_encode(&mac))
            } else {
                u.arbitrary().unwrap_or_default()
            };
            if let Ok(eref) = EvidenceRefV1::new(token) {
                let req = EvidenceResolveRequestV1 {
                    schema_version: 1,
                    evidence_ref: eref.clone(),
                    context: guard.valid_frozen.clone(),
                };
                let _ = guard.engine.resolve_evidence(&req);

                let many = EvidenceResolveManyRequestV1 {
                    schema_version: 1,
                    evidence_refs: vec![eref],
                    context: guard.valid_frozen.clone(),
                };
                let _ = guard.engine.resolve_evidence_many(&many);
            }
        }
        3 => {
            // GraphEvidenceRefV1: raw malformed strings and MAC-authenticated fuzzed selectors.
            let token = if u.arbitrary().unwrap_or(true) {
                let nonce: [u8; 16] = u.arbitrary().unwrap_or([0; 16]);
                let raw: Vec<u8> = u.arbitrary().unwrap_or_default();
                let payload = if u.arbitrary().unwrap_or(true) {
                    let mut p = vec![0u8; 300];
                    let n = raw.len().min(300);
                    p[..n].copy_from_slice(&raw[..n]);
                    p
                } else {
                    raw[..raw.len().min(512)].to_vec()
                };
                frame_graph_token(&guard.hmac_key, &nonce, &payload)
            } else {
                u.arbitrary().unwrap_or_default()
            };
            if let Ok(gref) = GraphEvidenceRefV1::new(token) {
                let req = GraphEvidenceResolveRequestV1 {
                    schema_version: 1,
                    evidence_ref: gref,
                    context: guard.valid_frozen.clone(),
                };
                let _ = guard.engine.resolve_graph_evidence(&req);
            }
        }
        4 => {
            // PageCursor: raw malformed strings and MAC-authenticated fuzzed payloads.
            let cursor_str = if u.arbitrary().unwrap_or(true) {
                let raw: Vec<u8> = u.arbitrary().unwrap_or_default();
                let payload = &raw[..raw.len().min(900)];
                let mac = hmac_sha256(&guard.hmac_key, PAGE_CURSOR_DOMAIN, payload);
                format!("fdbpg1.{}.{}", hex_encode(payload), hex_encode(&mac))
            } else {
                u.arbitrary().unwrap_or_default()
            };
            let page = PageRequestV1 {
                schema_version: if u.arbitrary().unwrap_or(true) { 1 } else { 2 },
                limit: u.int_in_range(0..=300).unwrap_or(10),
                cursor: Some(PageCursor(cursor_str)),
            };
            let _ = guard.engine.read_canonical_page("note", &guard.valid_frozen, &page);
            let _ = guard.engine.read_operational_state_page("ops", &guard.valid_frozen, &page);
        }
        _ => {
            // GraphExpandRequestV1 with fuzzed frozen context & seed.
            let mut ctx = guard.valid_frozen.clone();
            if u.arbitrary().unwrap_or(false) {
                ctx.token = u.arbitrary().unwrap_or_default();
            }
            let seed =
                GraphSeedV1::Query { schema_version: 1, text: "auth".to_string(), ranked_limit: 5 };
            let req = GraphExpandRequestV1::new(
                seed,
                TraversalDirection::Both,
                GraphReadContextV1::Frozen { schema_version: 1, context: ctx },
                1,
                10,
                1_000,
            );
            let _ = guard.engine.graph_expand(&req);
        }
    }
});
