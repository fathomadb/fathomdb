//! Launch configuration (`dev/interfaces/mcp.md` §2; ADR-0.6.0-operator-config-json-only;
//! ADR-0.8.32 #99). Parsing is strict: unknown keys, legacy evaluation keys,
//! wrong types, out-of-range values, and startup conflicts (`SC-01`–`SC-10`) fail
//! closed before any database or lock file is touched, and every error names
//! the offending key.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use fathomdb_sdk::{ScopePolicy, SyncMode, MAX_READ_SCOPES, MAX_SCOPE_TOKEN_BYTES};
use serde_json::{Map, Value};

use crate::names::validate_store_name;
pub use crate::names::NameError;

/// Default search path (`search_mode`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SearchMode {
    /// `search_text_only`: lexical only, never embeds the query.
    #[default]
    Text,
    /// Hybrid `search`.
    Hybrid,
}

impl SearchMode {
    /// The wire spelling (`"text"` / `"hybrid"`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Hybrid => "hybrid",
        }
    }
}

/// Policy governing an embedded host-registered engine
/// ([`crate::Adapter::register_engine_with_policy`], `SC-10`).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HostedStorePolicy {
    /// Whether mutating tools (`actuate`, `put_latest_state`) are permitted on this store.
    pub allow_writes: bool,
    /// Optional caller-scope policy asserted against `engine.config().scope_policy`.
    pub scope: Option<ScopePolicy>,
    /// WAL durability sync mode asserted against `engine.config().sync_mode.unwrap_or_default()`.
    pub sync_mode: SyncMode,
    /// Optional write-only allowlist of canonical node & edge kinds.
    pub allowed_kinds: Option<Vec<String>>,
    /// Optional write-only allowlist of canonical node kinds.
    pub allowed_node_kinds: Option<Vec<String>>,
    /// Optional write-only allowlist of canonical edge kinds.
    pub allowed_edge_kinds: Option<Vec<String>>,
    /// Optional write-only allowlist of `latest_state` operational collections.
    pub allowed_collections: Option<Vec<String>>,
}

impl HostedStorePolicy {
    /// Builder helper setting [`Self::sync_mode`].
    #[must_use]
    pub fn with_sync_mode(mut self, sync_mode: SyncMode) -> Self {
        self.sync_mode = sync_mode;
        self
    }
}

/// One explicit `stores` entry. `path` is absolute after parsing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoreEntry {
    /// The database file.
    pub path: PathBuf,
    /// Whether the adapter may create the file when it is missing (`EP-07`).
    pub create: bool,
    /// Whether the store opens at launch.
    pub eager: bool,
    /// Whether mutating tools (`actuate`, `put_latest_state`) are permitted on this store.
    pub allow_writes: bool,
    /// Optional caller-scope isolation policy bound when opening this store.
    pub scope: Option<ScopePolicy>,
    /// Optional WAL durability sync mode (write-enabled stores only; `None` defaults to `Normal`).
    pub sync_mode: Option<SyncMode>,
    /// Optional write-only allowlist of canonical node & edge kinds.
    pub allowed_kinds: Option<Vec<String>>,
    /// Optional write-only allowlist of canonical node kinds.
    pub allowed_node_kinds: Option<Vec<String>>,
    /// Optional write-only allowlist of canonical edge kinds.
    pub allowed_edge_kinds: Option<Vec<String>>,
    /// Optional write-only allowlist of `latest_state` operational collections.
    pub allowed_collections: Option<Vec<String>>,
}

/// One `roots` entry: every regular file directly in `path` ending in
/// `suffix` is addressable by its name without the suffix. Auto-discovered
/// `roots` entries are always strictly read-only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Root {
    /// The directory scanned (not recursively).
    pub path: PathBuf,
    /// File-name suffix that makes a file addressable; default `.sqlite`.
    pub suffix: String,
}

/// Where an addressable name points and the policy it carries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolution {
    /// The database file.
    pub path: PathBuf,
    /// Whether the adapter may create the file (explicit writable entries only).
    pub create: bool,
    /// Whether the store opens at launch (explicit entries only).
    pub eager: bool,
    /// Whether mutating tools are permitted on this store.
    pub allow_writes: bool,
    /// Optional caller-scope isolation policy bound when opening this store.
    pub scope: Option<ScopePolicy>,
    /// Effective WAL durability sync mode (`SyncMode::Normal` when omitted).
    pub sync_mode: SyncMode,
    /// Optional write-only allowlist of canonical node & edge kinds.
    pub allowed_kinds: Option<Vec<String>>,
    /// Optional write-only allowlist of canonical node kinds.
    pub allowed_node_kinds: Option<Vec<String>>,
    /// Optional write-only allowlist of canonical edge kinds.
    pub allowed_edge_kinds: Option<Vec<String>>,
    /// Optional write-only allowlist of `latest_state` operational collections.
    pub allowed_collections: Option<Vec<String>>,
}

/// A validated launch configuration. Field meanings and defaults are those of
/// `dev/interfaces/mcp.md` §2. `Config::default()` equals parsing `{}`, which
/// is also the starting point for embedded use (MR-004).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    /// Explicit stores by name; these win over root names.
    pub stores: BTreeMap<String, StoreEntry>,
    /// Directories whose database files are addressable by name (always read-only).
    pub roots: Vec<Root>,
    /// Session default store when a call omits `store`; must be addressable.
    pub default_store: Option<String>,
    /// Global write flag (default `false`); cannot be paired with `roots` (`SC-05`)
    /// and still requires per-store `allow_writes: true` + non-empty bound writable `scope`.
    pub allow_writes: bool,
    /// Seconds a store may stay idle before it is closed; 0 never closes.
    pub idle_timeout_secs: u64,
    /// Always 1 in this release (MR-005.7).
    pub max_open_stores: u32,
    /// Engine calls in flight per store, including abandoned ones (MR-033.2).
    pub admission_cap: u32,
    /// Per tool call wait deadline, from arrival: a call starts if it gets a
    /// permit before it (MR-033).
    pub call_deadline_ms: u64,
    /// Bound on a started call's run time, from its admission permit; a call
    /// still running then returns `timeout/deadline` and is abandoned. Never
    /// below `call_deadline_ms`.
    pub run_limit_ms: u64,
    /// Calls that may wait for an admission permit per store; one more is
    /// refused as `busy/admission`, reason `queue_full`.
    pub queue_limit: u32,
    /// Budget when a call omits `max_tokens` (MR-041.3).
    pub default_max_tokens: u32,
    /// Snippet cap in UTF-8 bytes (MR-040.1).
    pub snippet_bytes: u32,
    /// Search path when a call omits `mode`.
    pub search_mode: SearchMode,
    /// Open every store with the pinned default embedder (needs the
    /// `default-embedder` feature); required for `search_mode: hybrid`.
    pub use_default_embedder: bool,
    /// When set, a fresh telemetry sink is enabled on every open (MR-024.7).
    pub telemetry_dir: Option<PathBuf>,
    /// Whether results carry the `hints` sidecar when a call does not say;
    /// a call's own `hints` argument wins.
    pub hints: bool,
    /// Tool result representation: `text` (default) or `structured`.
    pub result_format: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            stores: BTreeMap::new(),
            roots: Vec::new(),
            default_store: None,
            allow_writes: false,
            idle_timeout_secs: 60,
            max_open_stores: 1,
            admission_cap: 4,
            call_deadline_ms: 10_000,
            run_limit_ms: 30_000,
            queue_limit: 32,
            default_max_tokens: 4_000,
            snippet_bytes: 300,
            search_mode: SearchMode::Text,
            use_default_embedder: false,
            telemetry_dir: None,
            hints: true,
            result_format: "text".into(),
        }
    }
}

/// An invalid configuration. The message names the key at fault.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigError {
    /// One line naming the key at fault and the rule it breaks.
    pub message: String,
}

impl ConfigError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ConfigError {}

const TOP_LEVEL_KEYS: &[&str] = &[
    "stores",
    "roots",
    "default_store",
    "allow_writes",
    "idle_timeout_secs",
    "max_open_stores",
    "admission_cap",
    "call_deadline_ms",
    "run_limit_ms",
    "queue_limit",
    "default_max_tokens",
    "snippet_bytes",
    "search_mode",
    "use_default_embedder",
    "telemetry_dir",
    "hints",
    "result_format",
];

const STORE_ENTRY_KEYS: &[&str] = &[
    "path",
    "create",
    "eager",
    "allow_writes",
    "scope",
    "sync_mode",
    "allowed_kinds",
    "allowed_node_kinds",
    "allowed_edge_kinds",
    "allowed_collections",
];

impl Config {
    /// Parse and validate `text`; relative paths resolve against `base_dir`.
    ///
    /// # Errors
    /// [`ConfigError`] naming the key for malformed JSON, unknown keys, legacy
    /// keys (`SC-04`), wrong types, out-of-range values, or startup conflicts
    /// (`SC-01`–`SC-09`).
    pub fn from_json_str(text: &str, base_dir: &Path) -> Result<Config, ConfigError> {
        let value: Value = serde_json::from_str(text)
            .map_err(|error| ConfigError::new(format!("config is not valid JSON: {error}")))?;
        let Value::Object(object) = value else {
            return Err(ConfigError::new("config must be a JSON object"));
        };
        // SC-04: explicit legacy key conflict check before unknown-key check so
        // the diagnostic names both keys when combined.
        if object.contains_key("write") {
            return Err(if object.contains_key("allow_writes") {
                ConfigError::new(
                    "write / allow_writes: legacy key \"write\" conflicts with \"allow_writes\"; \
                     remove \"write\" and configure per-store \"allow_writes\"",
                )
            } else {
                ConfigError::new(
                    "write: legacy evaluation key \"write\" is removed; use per-store \
                     \"allow_writes\"",
                )
            });
        }
        if object.contains_key("scope") {
            return Err(ConfigError::new(
                "scope: top-level \"scope\" is removed; bind \"scope\" per store under stores.<name>.scope",
            ));
        }
        reject_unknown(&object, TOP_LEVEL_KEYS, "")?;
        let defaults = Config::default();
        let call_deadline_ms = integer(&object, "call_deadline_ms", defaults.call_deadline_ms)?;
        let config = Config {
            stores: parse_stores(object.get("stores"), base_dir)?,
            roots: parse_roots(object.get("roots"), base_dir)?,
            default_store: optional_string(&object, "default_store")?,
            allow_writes: boolean(&object, "allow_writes", false)?,
            idle_timeout_secs: integer(&object, "idle_timeout_secs", defaults.idle_timeout_secs)?,
            max_open_stores: narrow(
                &object,
                "max_open_stores",
                u64::from(defaults.max_open_stores),
            )?,
            admission_cap: narrow(&object, "admission_cap", u64::from(defaults.admission_cap))?,
            call_deadline_ms,
            run_limit_ms: integer(
                &object,
                "run_limit_ms",
                defaults.run_limit_ms.max(call_deadline_ms),
            )?,
            queue_limit: narrow(&object, "queue_limit", u64::from(defaults.queue_limit))?,
            default_max_tokens: narrow(
                &object,
                "default_max_tokens",
                u64::from(defaults.default_max_tokens),
            )?,
            snippet_bytes: narrow(&object, "snippet_bytes", u64::from(defaults.snippet_bytes))?,
            search_mode: parse_search_mode(object.get("search_mode"))?,
            use_default_embedder: boolean(&object, "use_default_embedder", false)?,
            telemetry_dir: optional_string(&object, "telemetry_dir")?
                .map(|path| absolutize(base_dir, &path)),
            hints: boolean(&object, "hints", defaults.hints)?,
            result_format: match object.get("result_format") {
                None => defaults.result_format,
                Some(Value::String(value)) if matches!(value.as_str(), "text" | "structured") => {
                    value.clone()
                }
                Some(_) => {
                    return Err(ConfigError::new("result_format: must be text or structured"))
                }
            },
        };
        config.validate()?;
        Ok(config)
    }

    /// Read and validate the file at `path`; relative paths resolve against
    /// its directory.
    ///
    /// # Errors
    /// [`ConfigError`] when the file cannot be read or is invalid.
    pub fn load(path: &Path) -> Result<Config, ConfigError> {
        let text = fs::read_to_string(path).map_err(|error| {
            ConfigError::new(format!("cannot read config {}: {error}", path.display()))
        })?;
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|error| ConfigError::new(format!("cannot resolve config path: {error}")))?
                .join(path)
        };
        let base = absolute.parent().map(Path::to_path_buf).unwrap_or_default();
        Self::from_json_str(&text, &base)
    }

    /// Whether any explicit store in this configuration enables writes.
    #[must_use]
    pub fn any_writes_enabled(&self) -> bool {
        self.stores.values().any(|entry| entry.allow_writes)
    }

    /// The record head each search snippet keeps, `min(48, snippet_bytes /
    /// 2)` bytes.
    #[must_use]
    pub fn snippet_head_bytes(&self) -> u32 {
        snippet_head_bytes(self.snippet_bytes)
    }

    /// Check the value rules of interface §2 and startup conflict rules
    /// `SC-01`–`SC-09` on an already-built config (fields are public, so a
    /// hand-built config is checked again here).
    ///
    /// # Errors
    /// [`ConfigError`] naming the first key at fault.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !matches!(self.result_format.as_str(), "text" | "structured") {
            return Err(ConfigError::new("result_format: must be text or structured"));
        }
        if self.max_open_stores != 1 {
            return Err(ConfigError::new("max_open_stores: must be 1 in this release (MR-005.7)"));
        }
        if !(1..8).contains(&self.admission_cap) {
            return Err(ConfigError::new(
                "admission_cap: must be in 1..=7 (below the reader pool)",
            ));
        }
        if self.call_deadline_ms < 100 {
            return Err(ConfigError::new("call_deadline_ms: must be at least 100"));
        }
        // SC-09: run_limit_ms < call_deadline_ms
        if self.run_limit_ms < self.call_deadline_ms {
            return Err(ConfigError::new("run_limit_ms: must be at least call_deadline_ms"));
        }
        if self.queue_limit < 1 {
            return Err(ConfigError::new("queue_limit: must be at least 1"));
        }
        if self.default_max_tokens < 256 {
            return Err(ConfigError::new("default_max_tokens: must be at least 256"));
        }
        if !(40..=2000).contains(&self.snippet_bytes) {
            return Err(ConfigError::new("snippet_bytes: must be in 40..=2000"));
        }
        // SC-09: hybrid requires default embedder
        if self.search_mode == SearchMode::Hybrid && !self.use_default_embedder {
            return Err(ConfigError::new(
                "search_mode: \"hybrid\" requires use_default_embedder: true",
            ));
        }
        // SC-05: top-level allow_writes: true cannot be paired with roots
        if self.allow_writes && !self.roots.is_empty() {
            return Err(ConfigError::new(
                "roots: auto-discovered roots cannot be paired with allow_writes: true; \
                 roots are strictly read-only",
            ));
        }
        // SC-01: top-level allow_writes: true requires at least one explicit writable store
        if self.allow_writes && !self.any_writes_enabled() {
            return Err(ConfigError::new(
                "allow_writes: top-level allow_writes: true requires at least one explicit store \
                 with allow_writes: true and a bound writable scope",
            ));
        }

        let mut eager_count = 0usize;
        for (name, entry) in &self.stores {
            if validate_store_name(name).is_err() {
                return Err(ConfigError::new(format!(
                    "stores: {name:?} is not a valid store name"
                )));
            }
            validate_store_policy(
                &format!("stores.{name}"),
                entry.allow_writes,
                entry.create,
                entry.scope.as_ref(),
                entry.sync_mode,
                entry.allowed_kinds.as_deref(),
                entry.allowed_node_kinds.as_deref(),
                entry.allowed_edge_kinds.as_deref(),
                entry.allowed_collections.as_deref(),
            )?;
            if entry.eager {
                eager_count += 1;
            }
        }

        // SC-07: multiple eager stores when max_open_stores == 1
        if eager_count > self.max_open_stores as usize {
            return Err(ConfigError::new(format!(
                "stores.*.eager: at most {} eager store(s) allowed when max_open_stores is {}",
                self.max_open_stores, self.max_open_stores
            )));
        }

        // SC-06: canonical-path alias conflicts (explicit stores + root-discovered files)
        self.validate_canonical_aliases()?;

        // SC-09: default_store must be addressable
        if let Some(name) = &self.default_store {
            if self.resolve(name).is_err() {
                return Err(ConfigError::new(format!(
                    "default_store: {name:?} is not an addressable store"
                )));
            }
        }
        Ok(())
    }

    fn validate_canonical_aliases(&self) -> Result<(), ConfigError> {
        #[derive(Clone, Eq, PartialEq)]
        struct AliasEntry {
            name: String,
            allow_writes: bool,
            create: bool,
            eager: bool,
            sync_mode: SyncMode,
            scope_key: String,
        }

        let mut by_canon: BTreeMap<PathBuf, AliasEntry> = BTreeMap::new();
        let mut check = |name: String,
                         path: &Path,
                         allow_writes: bool,
                         create: bool,
                         eager: bool,
                         sync_mode: SyncMode,
                         scope: Option<&ScopePolicy>|
         -> Result<(), ConfigError> {
            let canon = canonical_target_path(path);
            let current = AliasEntry {
                name,
                allow_writes,
                create,
                eager,
                sync_mode,
                scope_key: canonical_scope_key(scope),
            };
            if let Some(existing) = by_canon.get(&canon) {
                if existing.allow_writes || current.allow_writes {
                    return Err(ConfigError::new(format!(
                        "stores: {:?} and {:?} resolve to the same database file and at least one \
                         enables allow_writes: true (duplicate writable alias forbidden)",
                        existing.name, current.name
                    )));
                }
                if existing.scope_key != current.scope_key
                    || existing.eager != current.eager
                    || existing.create != current.create
                    || existing.sync_mode != current.sync_mode
                {
                    return Err(ConfigError::new(format!(
                        "stores: {:?} and {:?} resolve to the same database file with \
                         incompatible policy settings (scope/eager/create/sync_mode)",
                        existing.name, current.name
                    )));
                }
            } else {
                by_canon.insert(canon, current);
            }
            Ok(())
        };

        for (name, entry) in &self.stores {
            check(
                name.clone(),
                &entry.path,
                entry.allow_writes,
                entry.create,
                entry.eager,
                entry.sync_mode.unwrap_or_default(),
                entry.scope.as_ref(),
            )?;
        }

        for root in &self.roots {
            let Ok(entries) = fs::read_dir(&root.path) else { continue };
            for entry in entries.flatten() {
                let Some(file_name) = entry.file_name().to_str().map(str::to_string) else {
                    continue;
                };
                let Some(name) = file_name.strip_suffix(root.suffix.as_str()) else { continue };
                if self.stores.contains_key(name) {
                    continue;
                }
                if validate_store_name(name).is_ok() && is_regular_file(&entry.path()) {
                    check(
                        name.to_string(),
                        &entry.path(),
                        false,
                        false,
                        false,
                        SyncMode::Normal,
                        None,
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Resolve an addressable name. Explicit `stores` entries win over root
    /// files. The grammar is checked first, so a path-like name never reaches
    /// the filesystem.
    ///
    /// # Errors
    /// [`NameError::BadName`] for a name outside the grammar,
    /// [`NameError::Unknown`] for a well-formed name that is not addressable.
    pub fn resolve(&self, name: &str) -> Result<Resolution, NameError> {
        validate_store_name(name)?;
        if let Some(entry) = self.stores.get(name) {
            return Ok(Resolution {
                path: entry.path.clone(),
                create: entry.create,
                eager: entry.eager,
                allow_writes: entry.allow_writes,
                scope: entry.scope.clone(),
                sync_mode: entry.sync_mode.unwrap_or_default(),
                allowed_kinds: entry.allowed_kinds.clone(),
                allowed_node_kinds: entry.allowed_node_kinds.clone(),
                allowed_edge_kinds: entry.allowed_edge_kinds.clone(),
                allowed_collections: entry.allowed_collections.clone(),
            });
        }
        for root in &self.roots {
            let candidate = root.path.join(format!("{name}{}", root.suffix));
            if is_regular_file(&candidate) {
                return Ok(Resolution {
                    path: candidate,
                    create: false,
                    eager: false,
                    allow_writes: false,
                    scope: None,
                    sync_mode: SyncMode::Normal,
                    allowed_kinds: None,
                    allowed_node_kinds: None,
                    allowed_edge_kinds: None,
                    allowed_collections: None,
                });
            }
        }
        Err(NameError::Unknown)
    }

    /// Every addressable name, sorted and deduplicated. Root directories are
    /// scanned at call time, so files added later become addressable.
    #[must_use]
    pub fn addressable_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.stores.keys().cloned().collect();
        for root in &self.roots {
            let Ok(entries) = fs::read_dir(&root.path) else { continue };
            for entry in entries.flatten() {
                let Some(file_name) = entry.file_name().to_str().map(str::to_string) else {
                    continue;
                };
                let Some(name) = file_name.strip_suffix(root.suffix.as_str()) else { continue };
                if validate_store_name(name).is_ok() && is_regular_file(&entry.path()) {
                    names.push(name.to_string());
                }
            }
        }
        names.sort();
        names.dedup();
        names
    }
}

/// Validate a [`HostedStorePolicy`] against `SC-01`, `SC-02`, `SC-03`, `SC-08`, and
/// `SC-10` (comparing against `host_scope` from `engine.config().scope_policy` and
/// `host_sync_mode` from `engine.config().sync_mode.unwrap_or_default()`).
pub(crate) fn validate_hosted_policy(
    name: &str,
    policy: &HostedStorePolicy,
    host_scope: &ScopePolicy,
    host_sync_mode: SyncMode,
) -> Result<ScopePolicy, String> {
    let prefix = format!("hosted.{name}");
    let requested_sync = (policy.sync_mode != SyncMode::Normal).then_some(policy.sync_mode);
    validate_store_policy(
        &prefix,
        policy.allow_writes,
        false,
        policy.scope.as_ref(),
        requested_sync,
        policy.allowed_kinds.as_deref(),
        policy.allowed_node_kinds.as_deref(),
        policy.allowed_edge_kinds.as_deref(),
        policy.allowed_collections.as_deref(),
    )
    .map_err(|error| error.message)?;

    if policy.sync_mode != host_sync_mode {
        return Err(format!(
            "{prefix}.sync_mode: hosted store sync_mode ({:?}) does not match host engine's \
             effective SyncMode ({:?})",
            policy.sync_mode.as_str(),
            host_sync_mode.as_str()
        ));
    }

    let host_canon = canonicalize_scope_policy(host_scope, &format!("{prefix}.host_scope"))
        .map_err(|error| error.message)?;

    if policy.allow_writes {
        let expected = policy.scope.as_ref().ok_or_else(|| {
            format!("{prefix}.scope: allow_writes requires a bound writable scope")
        })?;
        let expected_canon = canonicalize_scope_policy(expected, &format!("{prefix}.scope"))
            .map_err(|error| error.message)?;
        if host_canon.is_unscoped() || host_canon.write_scope().is_none() {
            return Err(format!(
                "{prefix}.scope: allow_writes: true conflicts with host engine's non-writing \
                 or unscoped ScopePolicy"
            ));
        }
        if host_canon != expected_canon {
            return Err(format!(
                "{prefix}.scope: hosted store policy does not match host engine's ScopePolicy"
            ));
        }
        return Ok(host_canon);
    }

    if let Some(expected) = &policy.scope {
        let expected_canon = canonicalize_scope_policy(expected, &format!("{prefix}.scope"))
            .map_err(|error| error.message)?;
        let reads_match = match (&host_canon, &expected_canon) {
            (ScopePolicy::Unscoped, ScopePolicy::Unscoped) => true,
            (
                ScopePolicy::Scoped { read_scopes: a, .. },
                ScopePolicy::Scoped { read_scopes: b, .. },
            ) => a == b,
            _ => false,
        };
        if !reads_match {
            return Err(format!(
                "{prefix}.scope: hosted store read scope policy does not match host engine's ScopePolicy"
            ));
        }
    }

    Ok(host_canon)
}

#[allow(clippy::too_many_arguments)]
fn validate_store_policy(
    prefix: &str,
    allow_writes: bool,
    create: bool,
    scope: Option<&ScopePolicy>,
    sync_mode: Option<SyncMode>,
    allowed_kinds: Option<&[String]>,
    allowed_node_kinds: Option<&[String]>,
    allowed_edge_kinds: Option<&[String]>,
    allowed_collections: Option<&[String]>,
) -> Result<(), ConfigError> {
    if let Some(policy) = scope {
        canonicalize_scope_policy(policy, &format!("{prefix}.scope"))?;
    }
    for (field, list) in [
        ("allowed_kinds", allowed_kinds),
        ("allowed_node_kinds", allowed_node_kinds),
        ("allowed_edge_kinds", allowed_edge_kinds),
        ("allowed_collections", allowed_collections),
    ] {
        if let Some(items) = list {
            if !allow_writes {
                return Err(ConfigError::new(format!(
                    "{prefix}.{field}: write-only allowlist requires allow_writes: true"
                )));
            }
            for item in items {
                if item.trim().is_empty() || item.chars().any(char::is_control) {
                    return Err(ConfigError::new(format!(
                        "{prefix}.{field}: entries must be non-empty strings without control characters"
                    )));
                }
            }
        }
    }

    if allow_writes {
        // SC-01: allow_writes requires explicit non-empty bound scope
        let Some(policy) = scope else {
            return Err(ConfigError::new(format!(
                "{prefix}.scope: allow_writes: true requires an explicit non-empty bound scope policy"
            )));
        };
        if policy.is_unscoped() {
            return Err(ConfigError::new(format!(
                "{prefix}.scope: allow_writes: true cannot use an unscoped policy"
            )));
        }
        // SC-02: allow_writes conflicts with read-only scope grant
        if policy.write_scope().is_none() {
            return Err(ConfigError::new(format!(
                "{prefix}.scope: allow_writes: true conflicts with a read-only scope policy (write scope is missing)"
            )));
        }
    } else {
        // SC-03: allow_writes: false rejects create: true, explicit sync_mode, and write scope grants
        if create {
            return Err(ConfigError::new(format!(
                "{prefix}.create: create: true requires allow_writes: true"
            )));
        }
        if sync_mode.is_some() {
            return Err(ConfigError::new(format!(
                "{prefix}.sync_mode: sync_mode requires allow_writes: true"
            )));
        }
        if let Some(policy) = scope {
            if policy.write_scope().is_some() {
                return Err(ConfigError::new(format!(
                    "{prefix}.scope: a write scope grant requires allow_writes: true"
                )));
            }
        }
    }
    Ok(())
}

/// Validate a [`ScopePolicy`] fail-closed (`SC-08`) and return its canonical
/// sorted form.
pub(crate) fn canonicalize_scope_policy(
    policy: &ScopePolicy,
    field: &str,
) -> Result<ScopePolicy, ConfigError> {
    match policy {
        ScopePolicy::Unscoped => Ok(ScopePolicy::Unscoped),
        ScopePolicy::Scoped { read_scopes, write_scope } => {
            if read_scopes.is_empty() {
                return Err(ConfigError::new(format!(
                    "{field}: read scopes must contain 1..={MAX_READ_SCOPES} entries"
                )));
            }
            if read_scopes.len() > MAX_READ_SCOPES {
                return Err(ConfigError::new(format!(
                    "{field}: read scopes length {} exceeds maximum {MAX_READ_SCOPES}",
                    read_scopes.len()
                )));
            }
            let mut seen = BTreeSet::new();
            for token in read_scopes {
                validate_scope_token(token, field)?;
                if !seen.insert(token.clone()) {
                    return Err(ConfigError::new(format!(
                        "{field}: duplicate scope token {token:?} in read scopes"
                    )));
                }
            }
            let canonical_reads: Vec<String> = seen.into_iter().collect();
            let canonical_write = match write_scope {
                Some(token) => {
                    validate_scope_token(token, field)?;
                    if canonical_reads.binary_search(token).is_err() {
                        return Err(ConfigError::new(format!(
                            "{field}: write scope must be included in read scopes"
                        )));
                    }
                    Some(token.clone())
                }
                None => None,
            };
            Ok(ScopePolicy::Scoped { read_scopes: canonical_reads, write_scope: canonical_write })
        }
    }
}

fn validate_scope_token(token: &str, field: &str) -> Result<(), ConfigError> {
    if token.is_empty() {
        return Err(ConfigError::new(format!("{field}: scope token must not be empty")));
    }
    if token == "*" {
        return Err(ConfigError::new(format!("{field}: wildcard \"*\" is forbidden")));
    }
    if token.len() > MAX_SCOPE_TOKEN_BYTES {
        return Err(ConfigError::new(format!(
            "{field}: scope token length {} exceeds maximum {MAX_SCOPE_TOKEN_BYTES} UTF-8 bytes",
            token.len()
        )));
    }
    if token.bytes().any(|byte| byte == 0x1e) {
        return Err(ConfigError::new(format!(
            "{field}: scope token must not contain record separator 0x1e"
        )));
    }
    if token.chars().any(char::is_control) {
        return Err(ConfigError::new(format!(
            "{field}: scope token must not contain NUL or control characters"
        )));
    }
    if token.trim() != token {
        return Err(ConfigError::new(format!(
            "{field}: scope token must not contain leading or trailing whitespace"
        )));
    }
    Ok(())
}

/// Deterministic key identifying a [`ScopePolicy`] for alias checks and session
/// table isolation.
pub(crate) fn canonical_scope_key(scope: Option<&ScopePolicy>) -> String {
    match scope {
        None | Some(ScopePolicy::Unscoped) => "unscoped".to_string(),
        Some(ScopePolicy::Scoped { read_scopes, write_scope }) => {
            let mut reads = read_scopes.clone();
            reads.sort();
            reads.dedup();
            format!("scoped:r={}:w={}", reads.join("\x1f"), write_scope.as_deref().unwrap_or(""))
        }
    }
}

/// Canonicalize `path` (following symlinks where they exist, or canonicalizing
/// the deepest existing ancestor for not-yet-created `create: true` targets).
fn canonical_target_path(path: &Path) -> PathBuf {
    let normalized = lexical_normalize(path);
    if let Ok(canon) = fs::canonicalize(&normalized) {
        return canon;
    }
    // Walk up ancestors until one can be canonicalized, preserving the tail.
    let mut current = normalized.as_path();
    let mut tail = Vec::new();
    while let Some(name) = current.file_name() {
        tail.push(name.to_os_string());
        let Some(parent) = current.parent() else { break };
        if let Ok(canon_parent) = fs::canonicalize(parent) {
            let mut joined = canon_parent;
            for part in tail.into_iter().rev() {
                joined.push(part);
            }
            return joined;
        }
        current = parent;
    }
    normalized
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

// Symlinks are not followed: a root may only expose files that are physically
// inside it.
fn is_regular_file(path: &Path) -> bool {
    fs::symlink_metadata(path).map(|metadata| metadata.file_type().is_file()).unwrap_or(false)
}

fn absolutize(base_dir: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn reject_unknown(
    object: &Map<String, Value>,
    known: &[&str],
    prefix: &str,
) -> Result<(), ConfigError> {
    for key in object.keys() {
        if !known.contains(&key.as_str()) {
            return Err(ConfigError::new(format!("{prefix}{key}: unknown key")));
        }
    }
    Ok(())
}

fn integer(object: &Map<String, Value>, key: &str, default: u64) -> Result<u64, ConfigError> {
    match object.get(key) {
        None => Ok(default),
        Some(value) => value
            .as_u64()
            .ok_or_else(|| ConfigError::new(format!("{key}: must be a non-negative integer"))),
    }
}

fn narrow(object: &Map<String, Value>, key: &str, default: u64) -> Result<u32, ConfigError> {
    u32::try_from(integer(object, key, default)?)
        .map_err(|_| ConfigError::new(format!("{key}: value is too large")))
}

fn boolean(object: &Map<String, Value>, key: &str, default: bool) -> Result<bool, ConfigError> {
    match object.get(key) {
        None => Ok(default),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(ConfigError::new(format!("{key}: must be a boolean"))),
    }
}

fn optional_string(object: &Map<String, Value>, key: &str) -> Result<Option<String>, ConfigError> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(ConfigError::new(format!("{key}: must be a string or null"))),
    }
}

fn optional_string_vec(
    object: &Map<String, Value>,
    key: &str,
    prefix: &str,
) -> Result<Option<Vec<String>>, ConfigError> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                let Some(text) = item.as_str() else {
                    return Err(ConfigError::new(format!(
                        "{prefix}{key}: must be an array of strings"
                    )));
                };
                out.push(text.to_string());
            }
            Ok(Some(out))
        }
        Some(_) => Err(ConfigError::new(format!("{prefix}{key}: must be an array of strings"))),
    }
}

fn parse_search_mode(value: Option<&Value>) -> Result<SearchMode, ConfigError> {
    match value {
        None => Ok(SearchMode::Text),
        Some(Value::String(mode)) if mode == "text" => Ok(SearchMode::Text),
        Some(Value::String(mode)) if mode == "hybrid" => Ok(SearchMode::Hybrid),
        Some(_) => Err(ConfigError::new("search_mode: must be \"text\" or \"hybrid\"")),
    }
}

fn parse_store_scope(
    value: Option<&Value>,
    allow_writes: bool,
    field: &str,
) -> Result<Option<ScopePolicy>, ConfigError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(token)) => {
            validate_scope_token(token, field)?;
            let policy = if allow_writes {
                ScopePolicy::read_write([token.clone()], token.clone())
            } else {
                ScopePolicy::read_only([token.clone()])
            };
            Ok(Some(policy))
        }
        Some(Value::Object(obj)) => {
            reject_unknown(obj, &["read", "write"], &format!("{field}."))?;
            let Some(Value::Array(raw_reads)) = obj.get("read") else {
                return Err(ConfigError::new(format!(
                    "{field}.read: required array of non-empty scope strings"
                )));
            };
            let mut reads = Vec::with_capacity(raw_reads.len());
            for item in raw_reads {
                let Some(token) = item.as_str() else {
                    return Err(ConfigError::new(format!(
                        "{field}.read: every entry must be a string"
                    )));
                };
                reads.push(token.to_string());
            }
            let write = match obj.get("write") {
                None | Some(Value::Null) => None,
                Some(Value::String(token)) => Some(token.clone()),
                Some(_) => {
                    return Err(ConfigError::new(format!(
                        "{field}.write: must be a string or null"
                    )))
                }
            };
            let policy = ScopePolicy::Scoped { read_scopes: reads, write_scope: write };
            let canonical = canonicalize_scope_policy(&policy, field)?;
            Ok(Some(canonical))
        }
        Some(_) => Err(ConfigError::new(format!(
            "{field}: must be null, a scope string, or {{\"read\": [...], \"write\"?: ...}}"
        ))),
    }
}

fn parse_store_sync_mode(
    value: Option<&Value>,
    field: &str,
) -> Result<Option<SyncMode>, ConfigError> {
    match value {
        None => Ok(None),
        Some(Value::String(mode)) if mode == "normal" => Ok(Some(SyncMode::Normal)),
        Some(Value::String(mode)) if mode == "full" => Ok(Some(SyncMode::Full)),
        Some(_) => Err(ConfigError::new(format!("{field}: must be \"normal\" or \"full\""))),
    }
}

fn parse_stores(
    value: Option<&Value>,
    base_dir: &Path,
) -> Result<BTreeMap<String, StoreEntry>, ConfigError> {
    let Some(value) = value else { return Ok(BTreeMap::new()) };
    let Value::Object(entries) = value else {
        return Err(ConfigError::new(
            "stores: must be an object of name -> {path, create?, eager?, allow_writes?, scope?, sync_mode?, allowed_kinds?}",
        ));
    };
    let mut stores = BTreeMap::new();
    for (name, entry) in entries {
        if validate_store_name(name).is_err() {
            return Err(ConfigError::new(format!(
                "stores: {name:?} is not a valid store name (^[A-Za-z0-9][A-Za-z0-9._-]{{0,63}}$)"
            )));
        }
        let Value::Object(entry) = entry else {
            return Err(ConfigError::new(format!("stores.{name}: must be an object")));
        };
        if entry.contains_key("write") {
            return Err(if entry.contains_key("allow_writes") {
                ConfigError::new(format!(
                    "stores.{name}.write / stores.{name}.allow_writes: legacy key \"write\" \
                     conflicts with \"allow_writes\""
                ))
            } else {
                ConfigError::new(format!(
                    "stores.{name}.write: legacy key \"write\" is removed; use \"allow_writes\""
                ))
            });
        }
        let prefix = format!("stores.{name}.");
        reject_unknown(entry, STORE_ENTRY_KEYS, &prefix)?;
        let Some(Value::String(path)) = entry.get("path") else {
            return Err(ConfigError::new(format!("stores.{name}.path: required string")));
        };
        let create = boolean(entry, "create", false)
            .map_err(|error| ConfigError::new(format!("stores.{name}.{error}")))?;
        let eager = boolean(entry, "eager", false)
            .map_err(|error| ConfigError::new(format!("stores.{name}.{error}")))?;
        let allow_writes = boolean(entry, "allow_writes", false)
            .map_err(|error| ConfigError::new(format!("stores.{name}.{error}")))?;
        let scope =
            parse_store_scope(entry.get("scope"), allow_writes, &format!("stores.{name}.scope"))?;
        let sync_mode =
            parse_store_sync_mode(entry.get("sync_mode"), &format!("stores.{name}.sync_mode"))?;
        let allowed_kinds = optional_string_vec(entry, "allowed_kinds", &prefix)?;
        let allowed_node_kinds = optional_string_vec(entry, "allowed_node_kinds", &prefix)?;
        let allowed_edge_kinds = optional_string_vec(entry, "allowed_edge_kinds", &prefix)?;
        let allowed_collections = optional_string_vec(entry, "allowed_collections", &prefix)?;

        stores.insert(
            name.clone(),
            StoreEntry {
                path: absolutize(base_dir, path),
                create,
                eager,
                allow_writes,
                scope,
                sync_mode,
                allowed_kinds,
                allowed_node_kinds,
                allowed_edge_kinds,
                allowed_collections,
            },
        );
    }
    Ok(stores)
}

fn parse_roots(value: Option<&Value>, base_dir: &Path) -> Result<Vec<Root>, ConfigError> {
    let Some(value) = value else { return Ok(Vec::new()) };
    let Value::Array(entries) = value else {
        return Err(ConfigError::new("roots: must be an array of {path, suffix?}"));
    };
    let mut roots = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let Value::Object(entry) = entry else {
            return Err(ConfigError::new(format!("roots[{index}]: must be an object")));
        };
        // SC-05: explicit fail-fast message when write keys are placed on roots
        for forbidden in ["allow_writes", "write", "create", "scope", "sync_mode"] {
            if entry.contains_key(forbidden) {
                return Err(ConfigError::new(format!(
                    "roots[{index}].{forbidden}: auto-discovered roots are strictly read-only"
                )));
            }
        }
        reject_unknown(entry, &["path", "suffix"], &format!("roots[{index}]."))?;
        let Some(Value::String(path)) = entry.get("path") else {
            return Err(ConfigError::new(format!("roots[{index}].path: required string")));
        };
        let suffix = match entry.get("suffix") {
            None => ".sqlite".to_string(),
            Some(Value::String(suffix)) if !suffix.is_empty() => suffix.clone(),
            Some(_) => {
                return Err(ConfigError::new(format!(
                    "roots[{index}].suffix: must be a non-empty string"
                )))
            }
        };
        let path = absolutize(base_dir, path);
        if !path.is_dir() {
            return Err(ConfigError::new(format!(
                "roots[{index}].path: {} is not a directory",
                path.display()
            )));
        }
        roots.push(Root { path, suffix });
    }
    Ok(roots)
}

/// The record head for a snippet cap of `snippet_bytes`: `min(48,
/// snippet_bytes / 2)` (see [`Config::snippet_head_bytes`]).
#[must_use]
pub(crate) fn snippet_head_bytes(snippet_bytes: u32) -> u32 {
    48.min(snippet_bytes / 2)
}
