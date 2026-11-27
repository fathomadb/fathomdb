//! Slice 50 (`WP-50C`, `AC32-FUZZ-01` & `AC32-FUZZ-03`) — Deterministic, zero-fuzz-dep
//! governance isolation proofs and minimized adversarial regressions covering the 7 fuzzing
//! surfaces (`fuzz_query_filter`, `fuzz_property_predicates`, `fuzz_scope_policy`,
//! `fuzz_auth_tokens`, `fuzz_wire_codecs`, `fuzz_mcp_jsonrpc` [paired with
//! `fathomdb-mcp/tests/slice50_fuzz_regressions.rs`], and `fuzz_export_and_open_recovery`).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use fathomdb_engine::{
    decode_dependency_trace_result_v1, decode_graph_expand_request_v1,
    decode_graph_expand_result_v1, decode_logical_export_v1, decode_round_body,
    encode_resolved_graph_evidence_v1, restore_logical_archive_v1, ActuationBatchV1,
    ActuationOperationV1, ArtifactRevisionId, CanonicalHash, ComparisonOp, DenseReadiness,
    EmbedderChoice, Engine, EngineConfig, EngineError, EngineOpenError,
    EvidenceArtifactLifecycleV1, EvidenceRefV1, EvidenceResolveManyRequestV1,
    EvidenceResolveRequestV1, EvidenceSearchRequestV1, Filter, FilterTerm, GraphEvidenceArtifactV1,
    GraphEvidenceRefV1, GraphEvidenceResolveRequestV1, GraphExpandRequestV1, GraphReadContextV1,
    GraphSeedV1, InitialState, LatestStateActuationV1, LifecycleActuationV1, LifecycleState,
    PageCursor, PageRequestV1, Predicate, PreparedWrite, ProjectionFts, ProjectionRole,
    ProjectionSpec, ProjectionVector, PropertyComparisonOp, PropertyEntityClass, PropertyMap,
    PropertyPredicate, PropertyValue, QueryStopWords, ReadContextV1, ReadView,
    ResolvedGraphEvidenceV1, ScalarValue, ScopePolicy, SearchFilter, SourceId, SourceLocator,
    SourceRevisionId, StopWordBase, SyncMode, TraversalDirection, LOGICAL_EXPORT_FORMAT_V1,
    MAX_READ_SCOPES, MAX_SCOPE_TOKEN_BYTES,
};
use fathomdb_query::{compile_text_query, compile_text_query_with};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const FROZEN_DOMAIN: &[u8] = b"fathomdb.frozen-read.v1\0";
const EVIDENCE_DOMAIN: &[u8] = b"fathomdb.evidence-ref.v1\0";
const GRAPH_STREAM_DOMAIN: &[u8] = b"fathomdb.graph-evidence.stream.v1\0";
const GRAPH_MAC_DOMAIN: &[u8] = b"fathomdb.graph-evidence.mac.v1\0";
const PAGE_CURSOR_DOMAIN: &[u8] = b"fathomdb.page-cursor.v1\0";

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(4)
        .expect("fathomdb-engine lives 4 levels below repo root")
        .to_path_buf()
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

fn hex_decode(s: &str) -> Vec<u8> {
    assert_eq!(s.len() % 2, 0);
    s.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let hi = (pair[0] as char).to_digit(16).expect("hex hi");
            let lo = (pair[1] as char).to_digit(16).expect("hex lo");
            ((hi << 4) | lo) as u8
        })
        .collect()
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

fn bootstrap_schema39_bytes(dir: &Path) -> Vec<u8> {
    let path = dir.join("schema39_template.sqlite");
    let engine =
        Engine::open_with_choice(&path, EmbedderChoice::None).expect("bootstrap schema39").engine;
    engine.close().expect("close bootstrap engine");
    {
        let conn = Connection::open(&path).expect("open template for checkpoint");
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    }
    fs::read(&path).expect("read template sqlite bytes")
}

// =========================================================================
// AC32-FUZZ-01: Workspace, toolchain, SBOM-tier, and dependency isolation
// =========================================================================

#[test]
fn ac32_fuzz_01_shipping_crates_have_zero_fuzz_dependencies() {
    let root = repo_root();

    // 1. Root Cargo.toml excludes fuzz/ and Cargo.lock contains zero fuzz crates.
    let root_toml = fs::read_to_string(root.join("Cargo.toml")).expect("read root Cargo.toml");
    assert!(root_toml.contains(r#"exclude = ["fuzz"]"#), "root Cargo.toml must exclude fuzz/");
    let root_lock = fs::read_to_string(root.join("Cargo.lock")).expect("read root Cargo.lock");
    assert!(!root_lock.contains("libfuzzer-sys"), "root Cargo.lock must not contain libfuzzer-sys");
    assert!(!root_lock.contains("\"arbitrary\""), "root Cargo.lock must not contain arbitrary");

    // 2. Root rust-toolchain.toml remains pinned at 1.95.0.
    let toolchain = fs::read_to_string(root.join("rust-toolchain.toml"))
        .expect("read root rust-toolchain.toml");
    assert!(toolchain.contains("1.95.0"), "root rust-toolchain.toml must stay pinned at 1.95.0");

    // 3. scripts/sbom-survey/tiers.toml classifies fuzz/ as dev-tooling.
    let tiers =
        fs::read_to_string(root.join("scripts/sbom-survey/tiers.toml")).expect("read tiers.toml");
    assert!(
        tiers.contains(r#"prefix = "fuzz/""#) && tiers.contains(r#"tier   = "dev-tooling""#),
        "tiers.toml must classify prefix = \"fuzz/\" as dev-tooling"
    );

    // 4. Standalone fuzz/Cargo.toml is an isolated workspace with zero patch/replace/git overrides.
    let fuzz_toml = fs::read_to_string(root.join("fuzz/Cargo.toml")).expect("read fuzz/Cargo.toml");
    assert!(fuzz_toml.contains("[workspace]"), "fuzz/Cargo.toml must declare [workspace]");
    assert!(fuzz_toml.contains("publish = false"), "fuzz/Cargo.toml must set publish = false");
    assert!(!fuzz_toml.contains("[patch"), "fuzz/Cargo.toml must have zero [patch] sections");
    assert!(!fuzz_toml.contains("[replace"), "fuzz/Cargo.toml must have zero [replace] sections");
    assert!(!fuzz_toml.contains("git ="), "fuzz/Cargo.toml must have zero git = dependencies");
    for target in [
        "fuzz_query_filter",
        "fuzz_property_predicates",
        "fuzz_scope_policy",
        "fuzz_auth_tokens",
        "fuzz_wire_codecs",
        "fuzz_mcp_jsonrpc",
        "fuzz_export_and_open_recovery",
    ] {
        assert!(
            root.join("fuzz/fuzz_targets").join(format!("{target}.rs")).is_file(),
            "missing fuzz target {target}.rs"
        );
    }

    // 5. Real `cargo tree --locked` across every shipping workspace crate proves zero fuzz deps.
    let shipping_crates = [
        "fathomdb",
        "fathomdb-sdk",
        "fathomdb-engine",
        "fathomdb-query",
        "fathomdb-schema",
        "fathomdb-cli",
        "fathomdb-embedder",
        "fathomdb-embedder-api",
        "fathomdb-mcp",
        "fathomdb-napi",
        "fathomdb-py",
    ];
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&root).args(["tree", "--locked", "--offline"]);
    for pkg in shipping_crates {
        cmd.args(["-p", pkg]);
    }
    let output = cmd.output().expect("execute cargo tree --locked");
    assert!(
        output.status.success(),
        "cargo tree --locked failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("libfuzzer-sys"),
        "shipping crate dependency graph leaked libfuzzer-sys"
    );
    assert!(!stdout.contains("arbitrary v"), "shipping crate dependency graph leaked arbitrary");
}

// =========================================================================
// Surface 1: fuzz_query_filter adversarial regressions
// =========================================================================

#[test]
fn fuzz_regression_query_filter_fts5_injection_and_stopword_bounds() {
    let adversarial_queries = [
        "",
        "   \t\r\n  ",
        "the and of to in a is it on at by for",
        r#"alpha OR "beta*" NEAR/3 (gamma NOT delta) ^10 :col"#,
        "\0\x01\x1ecafe\u{0301} '\"()^*:",
        r#""""" '''' (((( )))) **** ::::"#,
        &"token ".repeat(512),
    ];
    for q in adversarial_queries {
        let compiled = compile_text_query(q);
        for term in &compiled.terms {
            assert!(term.starts_with('"') && term.ends_with('"'));
        }
    }

    // StopWord validation boundaries.
    let too_many = QueryStopWords {
        base: StopWordBase::None,
        extra: (0..=256).map(|i| format!("w{i}")).collect(),
        keep: vec![],
    };
    assert!(too_many.validate().is_err());

    let too_long =
        QueryStopWords { base: StopWordBase::None, extra: vec!["a".repeat(65)], keep: vec![] };
    assert!(too_long.validate().is_err());

    let bad_case =
        QueryStopWords { base: StopWordBase::None, extra: vec!["Alpha".to_string()], keep: vec![] };
    assert!(bad_case.validate().is_err());

    let overlap = QueryStopWords {
        base: StopWordBase::Fathomdb,
        extra: vec!["custom".to_string()],
        keep: vec!["custom".to_string()],
    };
    assert!(overlap.validate().is_err());

    let keep_outside_none =
        QueryStopWords { base: StopWordBase::None, extra: vec![], keep: vec!["the".to_string()] };
    assert!(keep_outside_none.validate().is_err());

    let valid_custom = QueryStopWords {
        base: StopWordBase::Fathomdb,
        extra: vec!["custom".to_string()],
        keep: vec!["the".to_string()],
    }
    .validate()
    .expect("valid custom stopwords");
    let compiled = compile_text_query_with("the custom signal", &valid_custom);
    assert_eq!(compiled.match_expression, "\"the\" OR \"signal\"");

    // Predicate SQL-injection paths must fail closed at construction.
    for bad_path in [
        "",
        "$.status') OR 1=1--",
        "$.\0status",
        "$.unknown",
        "status",
        "$.status; DROP TABLE canonical_nodes;--",
    ] {
        assert!(Predicate::json_path_eq(bad_path, ScalarValue::Text("x".into())).is_err());
        assert!(Predicate::json_path_compare(bad_path, ComparisonOp::Gt, ScalarValue::Integer(1))
            .is_err());
    }

    // Live SQLite execution of adversarial queries and filters.
    let dir = TempDir::new().expect("tempdir");
    let engine = Engine::open_with_choice(dir.path().join("q.sqlite"), EmbedderChoice::None)
        .expect("open")
        .engine;
    for q in adversarial_queries {
        let _ = engine.search_text_only(q);
    }
    let pred = Predicate::json_path_eq("$.status", ScalarValue::Text("' OR 1=1 --".into()))
        .expect("allowlisted path");
    let filter = Filter {
        terms: vec![
            FilterTerm::Json(pred),
            FilterTerm::CreatedAfter(i64::MIN),
            FilterTerm::Status("open\0".into()),
        ],
    };
    let rows = engine
        .read_list_filter("note", &filter, 10, &ReadView::default())
        .expect("parameterized read_list_filter must not error");
    assert!(rows.is_empty());
    engine.close().expect("close");
}

// =========================================================================
// Surface 2: fuzz_property_predicates adversarial regressions
// =========================================================================

#[test]
fn fuzz_regression_property_predicates_numeric_and_projection_edges() {
    assert_eq!(PropertyComparisonOp::parse("EQ"), None);
    assert_eq!(PropertyEntityClass::parse("NODE"), None);
    assert_eq!(DenseReadiness::from_str_opt("pending"), None);

    let dir = TempDir::new().expect("tempdir");
    let engine = Engine::open_with_choice(dir.path().join("p.sqlite"), EmbedderChoice::None)
        .expect("open")
        .engine;

    // Declare valid Filterable projections.
    engine
        .configure_projections(
            &[
                ProjectionSpec {
                    name: "p_int".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
                ProjectionSpec {
                    name: "p_flt".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
            ],
            &[],
        )
        .expect("valid projections");

    // Invalid ProjectionSpecs must fail closed without panic.
    let invalid_specs = [
        ProjectionSpec {
            name: String::new(),
            roles: BTreeSet::from([ProjectionRole::Filterable]),
            fts: None,
            vector: None,
            source: None,
            entity_class: PropertyEntityClass::Node,
        },
        ProjectionSpec {
            name: "no_searchable_but_fts".into(),
            roles: BTreeSet::from([ProjectionRole::Filterable]),
            fts: Some(ProjectionFts { tokenizer: None }),
            vector: None,
            source: None,
            entity_class: PropertyEntityClass::Node,
        },
        ProjectionSpec {
            name: "no_searchable_but_vec".into(),
            roles: BTreeSet::from([ProjectionRole::Filterable]),
            fts: None,
            vector: Some(ProjectionVector::default()),
            source: None,
            entity_class: PropertyEntityClass::Node,
        },
        ProjectionSpec {
            name: "empty_roles".into(),
            roles: BTreeSet::new(),
            fts: None,
            vector: None,
            source: None,
            entity_class: PropertyEntityClass::Node,
        },
    ];
    for spec in invalid_specs {
        assert!(engine.configure_projections(&[spec], &[]).is_err());
    }

    // Invalid property writes (NaN, Inf, NUL, \x1e) must fail closed.
    let src = SourceId::new("prop-src").expect("src");
    let bad_maps = [
        PropertyMap::from([("p_flt".into(), PropertyValue::Float(f64::NAN))]),
        PropertyMap::from([("p_flt".into(), PropertyValue::Float(f64::INFINITY))]),
        PropertyMap::from([("p_flt".into(), PropertyValue::Float(f64::NEG_INFINITY))]),
        PropertyMap::from([("bad\0key".into(), PropertyValue::Int(1))]),
        PropertyMap::from([("bad\x1ekey".into(), PropertyValue::Int(1))]),
        PropertyMap::from([("p_str".into(), PropertyValue::String("a\0b".into()))]),
        PropertyMap::from([("p_str".into(), PropertyValue::String("a\x1eb".into()))]),
    ];
    for bad_props in bad_maps {
        let res = engine.write(&[PreparedWrite::Node {
            kind: "note".into(),
            body: "body".into(),
            source_id: src.clone(),
            logical_id: None,
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: Some(bad_props),
        }]);
        assert!(matches!(res, Err(EngineError::WriteValidation)));
    }

    // Valid extreme integer and float boundaries round-trip and compare accurately.
    let edge_props = PropertyMap::from([
        ("p_int".into(), PropertyValue::Int(i64::MAX)),
        ("p_flt".into(), PropertyValue::Float(-0.0)),
    ]);
    engine
        .write(&[PreparedWrite::Node {
            kind: "note".into(),
            body: "edge".into(),
            source_id: src,
            logical_id: Some("edge-1".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: Some(edge_props),
        }])
        .expect("valid edge properties");

    // Non-finite and bool-ordering predicates must fail closed on read_list_filter.
    let invalid_preds = [
        PropertyPredicate {
            key: "p_flt".into(),
            op: PropertyComparisonOp::Gt,
            value: PropertyValue::Float(f64::NAN),
        },
        PropertyPredicate {
            key: "p_flt".into(),
            op: PropertyComparisonOp::Lt,
            value: PropertyValue::Float(f64::INFINITY),
        },
        PropertyPredicate {
            key: "p_bool".into(),
            op: PropertyComparisonOp::Lt,
            value: PropertyValue::Bool(true),
        },
        PropertyPredicate {
            key: "bad\x1ekey".into(),
            op: PropertyComparisonOp::Eq,
            value: PropertyValue::Int(1),
        },
    ];
    for pred in invalid_preds {
        let filter = Filter { terms: vec![FilterTerm::Property(pred)] };
        assert!(engine.read_list_filter("note", &filter, 10, &ReadView::default()).is_err());
    }

    // Precision boundary: i64::MAX == 2^63 - 1 is strictly < 2^63 (9223372036854775808.0).
    let filter_lt_2p63 = Filter {
        terms: vec![FilterTerm::Property(PropertyPredicate {
            key: "p_int".into(),
            op: PropertyComparisonOp::Lt,
            value: PropertyValue::Float(9_223_372_036_854_775_808.0),
        })],
    };
    let hits = engine
        .read_list_filter("note", &filter_lt_2p63, 10, &ReadView::default())
        .expect("exact i64/f64 comparison");
    assert_eq!(hits.len(), 1);

    // Non-finite properties inside ResolvedGraphEvidenceV1 encoding fail closed.
    let bad_resolved = ResolvedGraphEvidenceV1 {
        schema_version: 1,
        artifact_revision_id: ArtifactRevisionId::new("ar-1").expect("ar"),
        artifact: GraphEvidenceArtifactV1::Node {
            logical_id: "n1".into(),
            kind: "note".into(),
            body: "b".into(),
            properties: Some(PropertyMap::from([("f".into(), PropertyValue::Float(f64::NAN))])),
        },
        source_id: "s1".into(),
        source_version_id: "sv1".into(),
        source_revision_id: SourceRevisionId::new("sr-1").expect("sr"),
        locator: SourceLocator::whole_body(),
        canonical_source_body: "b".into(),
        evidence_text: "b".into(),
        canonical_source_hash: CanonicalHash::sha256(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .expect("hash"),
        effective_valid_at: 1_700_000_000,
        artifact_lifecycle: EvidenceArtifactLifecycleV1::Node {
            state: LifecycleState::Active,
            superseded: false,
        },
        source_lifecycle_state: LifecycleState::Active,
        dependency: None,
    };
    assert!(encode_resolved_graph_evidence_v1(&bad_resolved).is_err());

    engine.close().expect("close");
}

// =========================================================================
// Surface 3: fuzz_scope_policy adversarial regressions
// =========================================================================

#[test]
fn fuzz_regression_scope_policy_grammar_and_sql_lowering() {
    let dir = TempDir::new().expect("tempdir");
    let missing = dir.path().join("must_not_exist.sqlite");

    let invalid_policies = vec![
        ScopePolicy::read_only(Vec::<String>::new()),
        ScopePolicy::read_only((0..=MAX_READ_SCOPES).map(|i| format!("s{i}"))),
        ScopePolicy::single(""),
        ScopePolicy::single("*"),
        ScopePolicy::single("tenant\x1ea"),
        ScopePolicy::single("tenant\0a"),
        ScopePolicy::single("tenant\x01ctrl"),
        ScopePolicy::single("tenant\x7fdel"),
        ScopePolicy::single(" leading"),
        ScopePolicy::single("trailing "),
        ScopePolicy::single("\ttab"),
        ScopePolicy::single("\u{00a0}nbsp"),
        ScopePolicy::single("a".repeat(MAX_SCOPE_TOKEN_BYTES + 1)),
        ScopePolicy::read_write(["alpha"], "beta"),
    ];

    for policy in invalid_policies {
        let cfg = EngineConfig::default().with_scope_policy(policy.clone());
        let err =
            Engine::open_with_choice_config_and_create(&missing, EmbedderChoice::None, cfg, false)
                .expect_err("invalid ScopePolicy must fail closed");
        assert!(
            matches!(err, EngineOpenError::EngineConfiguration(_)),
            "expected EngineConfiguration for {policy:?}, got {err:?}"
        );
        assert!(!missing.exists(), "invalid ScopePolicy must not touch disk");
    }

    // Valid 32-scope policy with max-length 128-byte token lowers SQL cleanly and enforces read-only.
    let live_path = dir.path().join("scoped_live.sqlite");
    let mut scopes: Vec<String> =
        (0..(MAX_READ_SCOPES - 1)).map(|i| format!("scope:{i}")).collect();
    scopes.push("k".repeat(MAX_SCOPE_TOKEN_BYTES));
    let ro_cfg = EngineConfig::default()
        .with_scope_policy(ScopePolicy::read_only(scopes))
        .with_sync_mode(SyncMode::Normal);
    let engine = Engine::open_with_choice_and_config(&live_path, EmbedderChoice::None, ro_cfg)
        .expect("open 32-scope read-only engine")
        .engine;
    let _ = engine.read_list("note", &[], 10, &ReadView::default()).expect("32-scope IN query");
    let _ = engine.search_text_only("signal").expect("32-scope FTS search");
    let rc = ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("rc");
    let _ = engine.freeze_read_context(&rc).expect("32-scope freeze");
    let write_res = engine.write(&[PreparedWrite::Node {
        kind: "note".into(),
        body: "denied".into(),
        source_id: SourceId::new("s").expect("src"),
        logical_id: None,
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
    }]);
    assert!(matches!(write_res, Err(EngineError::ScopeViolation { .. })));
    engine.close().expect("close");
}

// =========================================================================
// Surface 4: fuzz_auth_tokens adversarial regressions (including 0x53 0x39)
// =========================================================================

#[test]
fn fuzz_regression_auth_tokens_malformed_and_forged_s9_trailers() {
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("auth.sqlite");
    let scoped_path = dir.path().join("auth_scoped.sqlite");

    let engine = Engine::open_with_choice(&db_path, EmbedderChoice::None).expect("open").engine;
    let src = SourceId::new("auth-src").expect("src");
    engine
        .write(&[PreparedWrite::Node {
            kind: "note".into(),
            body: "auth token test doc".into(),
            source_id: src,
            logical_id: Some("n1".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        }])
        .expect("seed node");
    let rc = ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("rc");
    let valid_frozen = engine.freeze_read_context(&rc).expect("freeze");

    let raw_conn = Connection::open(&db_path).expect("raw conn");
    let database_id: String = raw_conn
        .query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key = '_fathomdb_database_id'",
            [],
            |r| r.get(0),
        )
        .expect("db id");
    let key_hex: String = raw_conn
        .query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key = '_fathomdb_read_context_key'",
            [],
            |r| r.get(0),
        )
        .expect("key");
    let hmac_key = hex_decode(&key_hex);
    drop(raw_conn);

    // 1. Raw malformed FrozenReadContextV1 tokens & oversized tokens.
    let page = PageRequestV1 { schema_version: 1, limit: 10, cursor: None };
    for bad in ["", "fdbfr1", "fdbfr1.zz.00", "wrong.00.00", &"a".repeat(2048)] {
        let mut ctx = valid_frozen.clone();
        ctx.token = bad.to_string();
        assert!(engine.read_canonical_page("note", &ctx, &page).is_err());
    }

    // 2. MAC-authenticated forged payloads exercising `decode_binding` trailing lengths
    //    (0, 32, 34 valid/invalid 0x53 0x39, 66 valid/invalid 0x53 0x39, and invalid 33/35).
    let base_binding = || {
        let mut buf = Vec::new();
        encode_u32(&mut buf, 1);
        encode_str(&mut buf, &database_id);
        buf.extend_from_slice(&0_i64.to_be_bytes());
        for _ in 0..4 {
            encode_u64(&mut buf, 0);
        }
        buf.extend_from_slice(&[0u8; 96]);
        buf
    };
    let trailer_cases: Vec<Vec<u8>> = vec![
        vec![],
        vec![0xAA; 32],
        [&[0x53, 0x39][..], &[0xBB; 32][..]].concat(),
        [&[0x53, 0x38][..], &[0xBB; 32][..]].concat(), // wrong 0x53 0x39 magic
        [&[0xAA; 32][..], &[0x53, 0x39][..], &[0xBB; 32][..]].concat(),
        [&[0xAA; 32][..], &[0x00, 0x00][..], &[0xBB; 32][..]].concat(),
        vec![0xCC; 33],
        vec![0xCC; 35],
    ];
    for trailer in trailer_cases {
        let mut payload = base_binding();
        payload.extend_from_slice(&trailer);
        let mac = hmac_sha256(&hmac_key, FROZEN_DOMAIN, &payload);
        let mut ctx = valid_frozen.clone();
        ctx.token = format!("fdbfr1.{}.{}", hex_encode(&payload), hex_encode(&mac));
        assert!(engine.read_canonical_page("note", &ctx, &page).is_err());
        let req = EvidenceSearchRequestV1::new("auth".into(), ctx);
        assert!(engine.search_with_evidence(&req).is_err());
    }

    // 3. Cross-scope / cross-DB replay must fail closed.
    let scoped_engine = Engine::open_with_choice_and_config(
        &scoped_path,
        EmbedderChoice::None,
        EngineConfig::default().with_scope_policy(ScopePolicy::single("tenant:a")),
    )
    .expect("open scoped")
    .engine;
    assert!(scoped_engine.read_canonical_page("note", &valid_frozen, &page).is_err());
    scoped_engine.close().expect("close scoped");

    // 4. MAC-authenticated forged EvidenceRefV1, GraphEvidenceRefV1, and PageCursor payloads.
    for payload in [&[][..], &[0, 0, 0, 2][..], &[0xFF; 128][..]] {
        let ev_mac = hmac_sha256(&hmac_key, EVIDENCE_DOMAIN, payload);
        let ev_ref =
            EvidenceRefV1::new(format!("fdbev1.{}.{}", hex_encode(payload), hex_encode(&ev_mac)))
                .expect("ev carrier");
        assert!(engine
            .resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: ev_ref.clone(),
                context: valid_frozen.clone(),
            })
            .is_err());
        assert!(engine
            .resolve_evidence_many(&EvidenceResolveManyRequestV1 {
                schema_version: 1,
                evidence_refs: vec![ev_ref],
                context: valid_frozen.clone(),
            })
            .is_ok()); // returns per-item unavailable outcome without panic

        let gr_tok = frame_graph_token(&hmac_key, &[0x11; 16], payload);
        if let Ok(gr_ref) = GraphEvidenceRefV1::new(gr_tok) {
            assert!(engine
                .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                    schema_version: 1,
                    evidence_ref: gr_ref,
                    context: valid_frozen.clone(),
                })
                .is_err());
        }

        let pg_mac = hmac_sha256(&hmac_key, PAGE_CURSOR_DOMAIN, payload);
        let bad_page = PageRequestV1 {
            schema_version: 1,
            limit: 10,
            cursor: Some(PageCursor(format!(
                "fdbpg1.{}.{}",
                hex_encode(payload),
                hex_encode(&pg_mac)
            ))),
        };
        assert!(engine.read_canonical_page("note", &valid_frozen, &bad_page).is_err());
    }

    // 5. Wire decoders refuse truncated/corrupt bytes cleanly.
    for raw in [&[][..], &[0xFF; 16][..], b"{not-json"] {
        assert!(decode_graph_expand_request_v1(raw).is_err());
        assert!(decode_graph_expand_result_v1(raw).is_err());
        assert!(decode_dependency_trace_result_v1(raw).is_err());
    }

    let expand_req = GraphExpandRequestV1::new(
        GraphSeedV1::Query { schema_version: 1, text: "auth".into(), ranked_limit: 5 },
        TraversalDirection::Both,
        GraphReadContextV1::Frozen { schema_version: 1, context: valid_frozen },
        1,
        10,
        1_000,
    );
    assert!(engine.graph_expand(&expand_req).is_ok());

    engine.close().expect("close");
}

// =========================================================================
// Surface 5: fuzz_wire_codecs adversarial regressions
// =========================================================================

#[test]
fn fuzz_regression_wire_codecs_archives_writes_and_actuation() {
    // 1. Malformed / truncated / unknown-field logical-export/v1 payloads.
    for raw in [
        &b""[..],
        &[0xFF, 0xFE, 0xFD][..],
        b"{",
        b"{}",
        br#"{"format_version":"fathomdb.logical-export/v1","schema_version":39,"extra_forbidden":1}"#,
    ] {
        assert!(decode_logical_export_v1(raw).is_err());
    }

    // Non-matching include_vectors / invalid policy / bad span bounds must fail closed.
    let mismatch = serde_json::json!({
        "format_version": LOGICAL_EXPORT_FORMAT_V1,
        "schema_version": 39,
        "include_audit": false,
        "include_vectors": true,
        "physical_erasure_policy": "autonomous",
        "dependency_generation": 0,
        "authority": {
            "canonical_nodes": [], "canonical_edges": [], "operational_collections": [],
            "operational_mutations": [], "operational_state": [], "artifact_revisions": [],
            "source_versions": [], "source_links": [], "source_dependencies": [],
            "projection_registry": [], "vector_kinds": [], "commit_timestamps": []
        },
        "audit": null,
        "vectors": null
    })
    .to_string();
    assert!(decode_logical_export_v1(mismatch.as_bytes()).is_err());

    // 2. ActuationBatchV1 & LatestStateActuationV1 & LifecycleActuationV1 boundaries.
    assert!(ActuationBatchV1::new("", vec![]).is_err());
    assert!(ActuationBatchV1::new("op-1", vec![]).is_err());
    assert!(LatestStateActuationV1::new("c", "k", "{not json").is_err());
    assert!(LatestStateActuationV1::new("c\0", "k", "{}").is_err());
    let rev = ArtifactRevisionId::new("ar-1").expect("rev");
    assert!(LifecycleActuationV1::new("h:abc", rev.clone(), LifecycleState::Deleted, None).is_err());
    assert!(LifecycleActuationV1::new("p:abc", rev.clone(), LifecycleState::Deleted, None).is_err());
    assert!(
        LifecycleActuationV1::new("a\x1eb", rev.clone(), LifecycleState::Deleted, None).is_err()
    );
    assert!(LifecycleActuationV1::new("n1", rev, LifecycleState::Purged, None).is_err());

    let valid_ls = LatestStateActuationV1::new("c", "k", r#"{"ok":true}"#).expect("ls");
    let too_many_ops = vec![ActuationOperationV1::PutLatestState(valid_ls); 129];
    assert!(ActuationBatchV1::new("op-too-many", too_many_ops).is_err());

    // 3. PreparedWrite invalid validity windows & edge confidence boundaries.
    let dir = TempDir::new().expect("tempdir");
    let engine = Engine::open_with_choice(dir.path().join("w.sqlite"), EmbedderChoice::None)
        .expect("open")
        .engine;
    let src = SourceId::new("w-src").expect("src");
    assert!(engine
        .write(&[PreparedWrite::Node {
            kind: "note".into(),
            body: "empty window".into(),
            source_id: src.clone(),
            logical_id: None,
            state: InitialState::Active,
            reason: None,
            valid_from: Some(100),
            valid_until: Some(100),
            properties: None,
        }])
        .is_err());
    for bad_conf in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        assert!(engine
            .write(&[PreparedWrite::Edge {
                kind: "rel".into(),
                from: "a".into(),
                to: "b".into(),
                source_id: src.clone(),
                logical_id: None,
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(bad_conf),
                extractor_model_id: None,
                temporal_fallback: None,
                properties: None,
            }])
            .is_err());
    }

    // 4. ContextPack malformed markers do not panic.
    let _ = decode_round_body("");
    let _ = decode_round_body("\u{001e}\0invalid");

    engine.close().expect("close");
}

// =========================================================================
// Surface 7: fuzz_export_and_open_recovery adversarial regressions
// =========================================================================

#[test]
fn fuzz_regression_export_restore_and_corrupt_open_recovery() {
    let dir = TempDir::new().expect("tempdir");
    let template = bootstrap_schema39_bytes(dir.path());

    // 1. Adversarial logical archive restore with invalid row_kind / invalid state / invalid properties JSON.
    let bad_restore_json = serde_json::json!({
        "format_version": LOGICAL_EXPORT_FORMAT_V1,
        "schema_version": 39,
        "include_audit": false,
        "include_vectors": false,
        "physical_erasure_policy": "autonomous",
        "dependency_generation": 0,
        "authority": {
            "canonical_nodes": [{
                "write_cursor": 1,
                "kind": "note",
                "body": "hello",
                "source_id": "s1",
                "logical_id": "n1",
                "superseded_at": null,
                "row_kind": "invalid_row_kind",
                "importance": null,
                "state": "active",
                "reason": null,
                "valid_from": null,
                "valid_until": null,
                "properties": "{not-valid-json",
                "scope": ""
            }],
            "canonical_edges": [], "operational_collections": [],
            "operational_mutations": [], "operational_state": [], "artifact_revisions": [],
            "source_versions": [], "source_links": [], "source_dependencies": [],
            "projection_registry": [], "vector_kinds": [], "commit_timestamps": []
        },
        "audit": null,
        "vectors": null
    })
    .to_string();
    let archive =
        decode_logical_export_v1(bad_restore_json.as_bytes()).expect("decodes structurally");
    let restore_path = dir.path().join("restore_bad.sqlite");
    fs::write(&restore_path, &template).expect("write template");
    let mut conn = Connection::open(&restore_path).expect("open conn");
    assert!(restore_logical_archive_v1(&mut conn, &archive).is_err());
    drop(conn);

    // 2. Corrupt / truncated SQLite files must fail closed with typed EngineOpenError (never panic).
    let corrupt_payloads: Vec<Vec<u8>> = vec![
        b"SQLite format 3\x00\x00\x00\xff\xff\x01\x01\x00".to_vec(),
        vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, 0x02],
        template[..template.len() / 2].to_vec(),
        template[..64].to_vec(),
    ];
    for (idx, bytes) in corrupt_payloads.into_iter().enumerate() {
        let p = dir.path().join(format!("corrupt_{idx}.sqlite"));
        fs::write(&p, bytes).expect("write corrupt");
        let res = Engine::open_with_choice(&p, EmbedderChoice::None);
        assert!(res.is_err(), "corrupt sqlite fixture {idx} must refuse open");
    }

    // 3. Targeted Schema 39 recovery-metadata corruption must fail closed cleanly.
    let mutations = [
        "UPDATE _fathomdb_open_state SET value = 'not-hex' WHERE key = '_fathomdb_database_id';",
        "UPDATE _fathomdb_open_state SET value = 'short' WHERE key = '_fathomdb_read_context_key';",
        "UPDATE _fathomdb_open_state SET value = 'bogus' WHERE key = '_fathomdb_physical_erasure_policy';",
        "UPDATE _fathomdb_read_visibility_state SET coalescing = 1 WHERE singleton = 1;",
        "UPDATE _fathomdb_read_visibility_state SET state_nonce = 'bad', global_struct_nonce = 'bad' WHERE singleton = 1;",
        "PRAGMA user_version = 999;",
        "DROP TABLE _fathomdb_scope_visibility_watermarks;",
    ];
    for (idx, sql) in mutations.into_iter().enumerate() {
        let p = dir.path().join(format!("meta_corrupt_{idx}.sqlite"));
        fs::write(&p, &template).expect("write template copy");
        {
            let conn = Connection::open(&p).expect("open raw");
            conn.execute_batch(sql).expect("apply metadata corruption");
        }
        let res = Engine::open_with_choice(&p, EmbedderChoice::None);
        assert!(res.is_err(), "corrupted metadata case {idx} ({sql}) must refuse open");
    }
}
