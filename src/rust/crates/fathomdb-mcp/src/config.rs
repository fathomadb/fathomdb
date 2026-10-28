//! Launch configuration (`dev/interfaces/mcp.md` §2;
//! ADR-0.6.0-operator-config-json-only). Parsing is strict: unknown keys,
//! wrong types and out-of-range values are all invalid, and each error names
//! the offending key.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

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

/// One explicit `stores` entry. `path` is absolute after parsing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoreEntry {
    /// The database file.
    pub path: PathBuf,
    /// Whether the adapter may create the file when it is missing.
    pub create: bool,
    /// Whether the store opens at launch.
    pub eager: bool,
}

/// One `roots` entry: every regular file directly in `path` ending in
/// `suffix` is addressable by its name without the suffix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Root {
    /// The directory scanned (not recursively).
    pub path: PathBuf,
    /// File-name suffix that makes a file addressable; default `.sqlite`.
    pub suffix: String,
}

/// Where an addressable name points.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolution {
    /// The database file.
    pub path: PathBuf,
    /// Whether the adapter may create the file (explicit entries only).
    pub create: bool,
    /// Whether the store opens at launch (explicit entries only).
    pub eager: bool,
}

/// A validated launch configuration. Field meanings and defaults are those of
/// `dev/interfaces/mcp.md` §2. `Config::default()` equals parsing `{}`, which
/// is also the starting point for embedded use (MR-004).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    /// Explicit stores by name; these win over root names.
    pub stores: BTreeMap<String, StoreEntry>,
    /// Directories whose database files are addressable by name.
    pub roots: Vec<Root>,
    /// Session default store when a call omits `store`; must be addressable.
    pub default_store: Option<String>,
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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            stores: BTreeMap::new(),
            roots: Vec::new(),
            default_store: None,
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
    fn new(message: impl Into<String>) -> Self {
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
    "write",
    "scope",
];

impl Config {
    /// Parse and validate `text`; relative paths resolve against `base_dir`.
    ///
    /// # Errors
    /// [`ConfigError`] naming the key for malformed JSON, unknown keys, wrong
    /// types, out-of-range values, values fixed in this release
    /// (`max_open_stores`, `write`, `scope`), bad store names, a root that is
    /// not a directory, a non-addressable `default_store`, and
    /// `search_mode: "hybrid"` without `use_default_embedder`.
    pub fn from_json_str(text: &str, base_dir: &Path) -> Result<Config, ConfigError> {
        let value: Value = serde_json::from_str(text)
            .map_err(|error| ConfigError::new(format!("config is not valid JSON: {error}")))?;
        let Value::Object(object) = value else {
            return Err(ConfigError::new("config must be a JSON object"));
        };
        reject_unknown(&object, TOP_LEVEL_KEYS, "")?;
        let defaults = Config::default();
        let call_deadline_ms = integer(&object, "call_deadline_ms", defaults.call_deadline_ms)?;
        let config = Config {
            stores: parse_stores(object.get("stores"), base_dir)?,
            roots: parse_roots(object.get("roots"), base_dir)?,
            default_store: optional_string(&object, "default_store")?,
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
        };
        if boolean(&object, "write", false)? {
            return Err(ConfigError::new("write: must be false in this release (read-only)"));
        }
        match object.get("scope") {
            None | Some(Value::Null) => {}
            Some(_) => {
                return Err(ConfigError::new(
                    "scope: must be null in this release; the engine does not yet apply scope \
                     to every read path (MR-014.4)",
                ))
            }
        }
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

    /// The record head each search snippet keeps, `min(48, snippet_bytes /
    /// 2)` bytes: 48 holds a LoCoMo-style header such as `(unknown date)
    /// [8 May 2023] Caroline:` (37 bytes) with room for a longer date or
    /// speaker, and never exceeds the selector's `max_bytes / 2`.
    #[must_use]
    pub fn snippet_head_bytes(&self) -> u32 {
        snippet_head_bytes(self.snippet_bytes)
    }

    /// Check the value rules of interface §2 on an already-built config
    /// (fields are public, so a hand-built config is checked again here).
    ///
    /// # Errors
    /// [`ConfigError`] naming the first key at fault.
    pub fn validate(&self) -> Result<(), ConfigError> {
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
        if self.search_mode == SearchMode::Hybrid && !self.use_default_embedder {
            return Err(ConfigError::new(
                "search_mode: \"hybrid\" requires use_default_embedder: true",
            ));
        }
        for name in self.stores.keys() {
            if validate_store_name(name).is_err() {
                return Err(ConfigError::new(format!(
                    "stores: {name:?} is not a valid store name"
                )));
            }
        }
        if let Some(name) = &self.default_store {
            if self.resolve(name).is_err() {
                return Err(ConfigError::new(format!(
                    "default_store: {name:?} is not an addressable store"
                )));
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
            });
        }
        for root in &self.roots {
            let candidate = root.path.join(format!("{name}{}", root.suffix));
            if is_regular_file(&candidate) {
                return Ok(Resolution { path: candidate, create: false, eager: false });
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

fn parse_search_mode(value: Option<&Value>) -> Result<SearchMode, ConfigError> {
    match value {
        None => Ok(SearchMode::Text),
        Some(Value::String(mode)) if mode == "text" => Ok(SearchMode::Text),
        Some(Value::String(mode)) if mode == "hybrid" => Ok(SearchMode::Hybrid),
        Some(_) => Err(ConfigError::new("search_mode: must be \"text\" or \"hybrid\"")),
    }
}

fn parse_stores(
    value: Option<&Value>,
    base_dir: &Path,
) -> Result<BTreeMap<String, StoreEntry>, ConfigError> {
    let Some(value) = value else { return Ok(BTreeMap::new()) };
    let Value::Object(entries) = value else {
        return Err(ConfigError::new(
            "stores: must be an object of name -> {path, create?, eager?}",
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
        reject_unknown(entry, &["path", "create", "eager"], &format!("stores.{name}."))?;
        let Some(Value::String(path)) = entry.get("path") else {
            return Err(ConfigError::new(format!("stores.{name}.path: required string")));
        };
        stores.insert(
            name.clone(),
            StoreEntry {
                path: absolutize(base_dir, path),
                create: boolean(entry, "create", false)
                    .map_err(|error| ConfigError::new(format!("stores.{name}.{error}")))?,
                eager: boolean(entry, "eager", false)
                    .map_err(|error| ConfigError::new(format!("stores.{name}.{error}")))?,
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
