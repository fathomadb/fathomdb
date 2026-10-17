//! Launch configuration (`dev/interfaces/mcp.md` §2).

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::names::NameError;

/// Default search path.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SearchMode {
    #[default]
    Text,
    Hybrid,
}

/// One explicit `stores` entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoreEntry {
    pub path: PathBuf,
    pub create: bool,
    pub eager: bool,
}

/// One `roots` entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Root {
    pub path: PathBuf,
    pub suffix: String,
}

/// Where an addressable name points.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolution {
    pub path: PathBuf,
    pub create: bool,
    pub eager: bool,
}

/// A validated launch configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub stores: BTreeMap<String, StoreEntry>,
    pub roots: Vec<Root>,
    pub default_store: Option<String>,
    pub idle_timeout_secs: u64,
    pub max_open_stores: u32,
    pub admission_cap: u32,
    pub call_deadline_ms: u64,
    pub default_max_tokens: u32,
    pub snippet_chars: u32,
    pub search_mode: SearchMode,
    pub use_default_embedder: bool,
    pub telemetry_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        todo!()
    }
}

/// An invalid configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigError {
    pub message: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    /// Parse and validate `text`.
    pub fn from_json_str(_text: &str, _base_dir: &Path) -> Result<Config, ConfigError> {
        todo!()
    }

    /// Read and validate the file at `path`.
    pub fn load(_path: &Path) -> Result<Config, ConfigError> {
        todo!()
    }

    /// Resolve an addressable name.
    pub fn resolve(&self, _name: &str) -> Result<Resolution, NameError> {
        todo!()
    }

    /// Every addressable name, sorted.
    pub fn addressable_names(&self) -> Vec<String> {
        todo!()
    }
}
