#![no_main]

use std::sync::{Mutex, OnceLock};

use arbitrary::Unstructured;
use fathomdb_engine::{
    EmbedderChoice, Engine, EngineConfig, EngineOpenError, InitialState, PreparedWrite,
    ReadContextV1, ReadView, ScopePolicy, SearchFilter, SourceId, SyncMode, MAX_READ_SCOPES,
    MAX_SCOPE_TOKEN_BYTES,
};
use libfuzzer_sys::fuzz_target;
use tempfile::TempDir;

struct ScopeFixture {
    dir: TempDir,
    missing_path: std::path::PathBuf,
    live_path: std::path::PathBuf,
}

fn fixture() -> &'static Mutex<ScopeFixture> {
    static FIXTURE: OnceLock<Mutex<ScopeFixture>> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let dir = TempDir::new().expect("tempdir for fuzz_scope_policy");
        let missing_path = dir.path().join("never_created.sqlite");
        let live_path = dir.path().join("scoped_live.sqlite");
        // Pre-initialize live_path once so scoped opens don't re-run migrations.
        let engine = Engine::open_with_choice(&live_path, EmbedderChoice::None)
            .expect("bootstrap scoped db")
            .engine;
        engine.close().expect("close bootstrap scoped db");
        Mutex::new(ScopeFixture { dir, missing_path, live_path })
    })
}

fn arb_scope_token(u: &mut Unstructured<'_>) -> String {
    match u.int_in_range(0..=15).unwrap_or(0) {
        0 => String::new(),
        1 => "*".to_string(),
        2 => "tenant\x1ea".to_string(),
        3 => "tenant\0a".to_string(),
        4 => " leading".to_string(),
        5 => "trailing ".to_string(),
        6 => "\ttab\n".to_string(),
        7 => "\u{00a0}nbsp".to_string(),
        8 => "a".repeat(MAX_SCOPE_TOKEN_BYTES),
        9 => "a".repeat(MAX_SCOPE_TOKEN_BYTES + 1),
        10 => "../etc/passwd".to_string(),
        11 => "scope:alpha".to_string(),
        12 => "scope:beta".to_string(),
        13 => "scope\x7fdel".to_string(),
        14 => "scope\x01ctrl".to_string(),
        _ => {
            let s: String = u.arbitrary().unwrap_or_default();
            s.chars().take(160).collect()
        }
    }
}

fn is_valid_token(token: &str) -> bool {
    !token.is_empty()
        && token != "*"
        && token.len() <= MAX_SCOPE_TOKEN_BYTES
        && !token.bytes().any(|b| b == 0x1e)
        && !token.chars().any(char::is_control)
        && token.trim() == token
}

fn expected_valid_policy(policy: &ScopePolicy) -> bool {
    match policy {
        ScopePolicy::Unscoped => true,
        ScopePolicy::Scoped { read_scopes, write_scope } => {
            if read_scopes.is_empty() || read_scopes.len() > MAX_READ_SCOPES {
                return false;
            }
            if !read_scopes.iter().all(|s| is_valid_token(s)) {
                return false;
            }
            if let Some(ws) = write_scope {
                if !is_valid_token(ws) || !read_scopes.iter().any(|r| r == ws) {
                    return false;
                }
            }
            true
        }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    let mut u = Unstructured::new(data);
    let mode = u.int_in_range(0..=7).unwrap_or(0);

    let policy = match u.int_in_range(0..=3).unwrap_or(0) {
        0 => ScopePolicy::Unscoped,
        1 => ScopePolicy::single(arb_scope_token(&mut u)),
        2 => {
            let count = u.int_in_range(0..=40).unwrap_or(0);
            let reads: Vec<String> = (0..count).map(|_| arb_scope_token(&mut u)).collect();
            ScopePolicy::read_only(reads)
        }
        _ => {
            let count = u.int_in_range(0..=40).unwrap_or(0);
            let reads: Vec<String> = (0..count).map(|_| arb_scope_token(&mut u)).collect();
            let ws = if u.arbitrary().unwrap_or(true) {
                if !reads.is_empty() && u.arbitrary().unwrap_or(true) {
                    let idx = u.int_in_range(0..=(reads.len() - 1)).unwrap_or(0);
                    Some(reads[idx].clone())
                } else {
                    Some(arb_scope_token(&mut u))
                }
            } else {
                None
            };
            ScopePolicy::Scoped { read_scopes: reads, write_scope: ws }
        }
    };

    let expected_valid = expected_valid_policy(&policy);
    let sync_mode = if u.arbitrary().unwrap_or(false) { SyncMode::Full } else { SyncMode::Normal };
    let config =
        EngineConfig::default().with_scope_policy(policy.clone()).with_sync_mode(sync_mode);

    let guard = fixture().lock().unwrap_or_else(|p| p.into_inner());

    // Fast path: open with create_if_missing = false on nonexistent path.
    // Invalid ScopePolicy MUST fail with EngineConfiguration before touching disk.
    let res = Engine::open_with_choice_config_and_create(
        &guard.missing_path,
        EmbedderChoice::None,
        config.clone(),
        false,
    );
    match res {
        Ok(opened) => {
            let _ = opened.engine.close();
            panic!("nonexistent path must never open with create_if_missing=false");
        }
        Err(EngineOpenError::EngineConfiguration(_)) => {
            assert!(
                !expected_valid,
                "valid ScopePolicy was rejected as EngineConfiguration: {policy:?}"
            );
        }
        Err(EngineOpenError::DatabaseNotFound { .. }) => {
            assert!(
                expected_valid,
                "invalid ScopePolicy bypassed open-time validation: {policy:?}"
            );
        }
        Err(_) => {}
    }
    assert!(
        !guard.missing_path.exists(),
        "open_with_choice_config_and_create(false) must never create db file"
    );

    // Also exercise MCP config scope parser grammar & conflict checks purely in memory.
    let scope_json = match &policy {
        ScopePolicy::Unscoped => serde_json::json!(null),
        ScopePolicy::Scoped { read_scopes, write_scope } => serde_json::json!({
            "read": read_scopes,
            "write": write_scope,
        }),
    };
    let mcp_cfg = serde_json::json!({
        "allow_writes": u.arbitrary().unwrap_or(false),
        "stores": {
            "s1": {
                "path": guard.live_path.to_string_lossy(),
                "allow_writes": u.arbitrary().unwrap_or(false),
                "scope": scope_json,
                "sync_mode": sync_mode.as_str(),
            }
        }
    });
    let _ = fathomdb_mcp::config::Config::from_json_str(&mcp_cfg.to_string(), guard.dir.path());

    // Bounded real SQL lowering path: exercise ScopePredicate::sql_clause (single = vs IN (...))
    // on the live Schema 39 store when mode == 7 and policy is valid.
    if mode == 7 && expected_valid {
        if let Ok(opened) =
            Engine::open_with_choice_and_config(&guard.live_path, EmbedderChoice::None, config)
        {
            let engine = opened.engine;
            let _ = engine.read_list("note", &[], 5, &ReadView::default());
            let _ = engine.search_text_only("alpha");
            if let Ok(rc) = ReadContextV1::new(ReadView::default(), SearchFilter::default()) {
                let _ = engine.freeze_read_context(&rc);
            }
            if engine.scope_policy().write_scope().is_none() && !engine.scope_policy().is_unscoped()
            {
                if let Ok(src) = SourceId::new("ro-check") {
                    let write_res = engine.write(&[PreparedWrite::Node {
                        kind: "note".to_string(),
                        body: "denied".to_string(),
                        source_id: src,
                        logical_id: None,
                        state: InitialState::Active,
                        reason: None,
                        valid_from: None,
                        valid_until: None,
                        properties: None,
                    }]);
                    assert!(write_res.is_err(), "read-only ScopePolicy must refuse writes");
                }
            }
            let _ = engine.close();
        }
    }
});
