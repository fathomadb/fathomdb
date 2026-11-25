//! Caller-scope isolation policy and SQL predicate binding (ADR-0.8.32).

use std::fmt::{self, Display, Formatter};

use rusqlite::types::Value;
use sha2::{Digest, Sha256};

use crate::errors::EngineError;
use crate::Engine;

/// Maximum number of distinct read scopes accepted on one [`ScopePolicy`].
pub const MAX_READ_SCOPES: usize = 32;

/// Maximum UTF-8 byte length of a single scope token.
pub const MAX_SCOPE_TOKEN_BYTES: usize = 128;

const SCOPE_DIGEST_DOMAIN_V1: &[u8] = b"fathomdb-scope-policy-v1\x00";

/// Rejection reason returned when an [`EngineConfig::scope_policy`] fails
/// open-time validation before any database, lock, or worker side effect.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScopePolicyError {
    /// A [`ScopePolicy::Scoped`] policy carried zero `read_scopes`.
    EmptyReadScopes,
    /// `read_scopes` exceeded [`MAX_READ_SCOPES`].
    TooManyReadScopes { count: usize, max: usize },
    /// A scope token was empty (`""` is reserved for [`ScopePolicy::Unscoped`]).
    EmptyToken { is_write: bool },
    /// A scope token exceeded [`MAX_SCOPE_TOKEN_BYTES`] UTF-8 bytes.
    TokenTooLong { is_write: bool, len: usize, max: usize },
    /// A scope token contained the internal `\x1e` delimiter byte.
    DelimiterForbidden { is_write: bool },
    /// A scope token contained a NUL or control character.
    ControlCharacter { is_write: bool },
    /// A scope token contained leading or trailing whitespace.
    LeadingOrTrailingWhitespace { is_write: bool },
    /// Wildcard `"*"` is forbidden on all read and write scope tokens.
    WildcardForbidden { is_write: bool },
    /// `write_scope` was not present in `read_scopes`.
    WriteScopeNotInReadScopes,
}

impl Display for ScopePolicyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyReadScopes => {
                write!(f, "scope_policy.read_scopes must contain 1..={MAX_READ_SCOPES} scopes")
            }
            Self::TooManyReadScopes { count, max } => {
                write!(f, "scope_policy.read_scopes length {count} exceeds maximum {max}")
            }
            Self::EmptyToken { is_write } => {
                let kind = if *is_write { "write_scope" } else { "read_scopes entry" };
                write!(f, "scope_policy {kind} must not be empty")
            }
            Self::TokenTooLong { is_write, len, max } => {
                let kind = if *is_write { "write_scope" } else { "read_scopes entry" };
                write!(f, "scope_policy {kind} length {len} exceeds maximum {max} UTF-8 bytes")
            }
            Self::DelimiterForbidden { is_write } => {
                let kind = if *is_write { "write_scope" } else { "read_scopes entry" };
                write!(f, "scope_policy {kind} must not contain record separator byte 0x1e")
            }
            Self::ControlCharacter { is_write } => {
                let kind = if *is_write { "write_scope" } else { "read_scopes entry" };
                write!(f, "scope_policy {kind} must not contain NUL or control characters")
            }
            Self::LeadingOrTrailingWhitespace { is_write } => {
                let kind = if *is_write { "write_scope" } else { "read_scopes entry" };
                write!(f, "scope_policy {kind} must not contain leading or trailing whitespace")
            }
            Self::WildcardForbidden { is_write } => {
                let kind = if *is_write { "write_scope" } else { "read_scopes entry" };
                write!(f, "scope_policy wildcard '*' is forbidden on {kind}")
            }
            Self::WriteScopeNotInReadScopes => {
                write!(f, "scope_policy.write_scope must be included in scope_policy.read_scopes")
            }
        }
    }
}

impl std::error::Error for ScopePolicyError {}

/// Caller-scope isolation policy bound for the lifetime of an [`Engine`] handle.
///
/// [`ScopePolicy::Unscoped`] preserves single-scope compatibility (binding
/// `scope = ""` for both reads and writes). [`ScopePolicy::Scoped`] restricts
/// the handle to a caller-authorized slice of scopes.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
pub enum ScopePolicy {
    /// Single-scope / single-agent mode binding `scope = ""` for reads and writes.
    #[default]
    Unscoped,
    /// Multi-scope caller isolation mode restricted to explicit scope grants.
    Scoped {
        /// Allowed caller read scopes (validated `1..=32` entries at open).
        read_scopes: Vec<String>,
        /// Optional write grant; must belong to `read_scopes` when `Some`.
        write_scope: Option<String>,
    },
}

impl ScopePolicy {
    /// Construct a single-scope read/write policy.
    #[must_use]
    pub fn single(scope: impl Into<String>) -> Self {
        let token = scope.into();
        Self::Scoped { read_scopes: vec![token.clone()], write_scope: Some(token) }
    }

    /// Construct a read-only multi-scope policy (`write_scope = None`).
    #[must_use]
    pub fn read_only<I, S>(read_scopes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::Scoped {
            read_scopes: read_scopes.into_iter().map(Into::into).collect(),
            write_scope: None,
        }
    }

    /// Construct a multi-scope read policy with an explicit write grant.
    #[must_use]
    pub fn read_write<I, S>(read_scopes: I, write_scope: impl Into<String>) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::Scoped {
            read_scopes: read_scopes.into_iter().map(Into::into).collect(),
            write_scope: Some(write_scope.into()),
        }
    }

    /// Returns `true` when this handle runs in [`ScopePolicy::Unscoped`] mode.
    #[must_use]
    pub fn is_unscoped(&self) -> bool {
        matches!(self, Self::Unscoped)
    }

    /// Returns the granted non-empty read scopes (`&[]` when [`Self::Unscoped`]).
    #[must_use]
    pub fn read_scopes(&self) -> &[String] {
        match self {
            Self::Unscoped => &[],
            Self::Scoped { read_scopes, .. } => read_scopes.as_slice(),
        }
    }

    /// Returns the granted write scope (`None` for read-only or [`Self::Unscoped`]).
    #[must_use]
    pub fn write_scope(&self) -> Option<&str> {
        match self {
            Self::Unscoped => None,
            Self::Scoped { write_scope, .. } => write_scope.as_deref(),
        }
    }

    /// Validate and canonicalize (deduplicate + sort `read_scopes`) this policy.
    pub(crate) fn validated(&self) -> Result<Self, ScopePolicyError> {
        match self {
            Self::Unscoped => Ok(Self::Unscoped),
            Self::Scoped { read_scopes, write_scope } => {
                if read_scopes.is_empty() {
                    return Err(ScopePolicyError::EmptyReadScopes);
                }
                if read_scopes.len() > MAX_READ_SCOPES {
                    return Err(ScopePolicyError::TooManyReadScopes {
                        count: read_scopes.len(),
                        max: MAX_READ_SCOPES,
                    });
                }
                for token in read_scopes {
                    validate_scope_token(token, false)?;
                }
                let mut canonical_reads = read_scopes.clone();
                canonical_reads.sort();
                canonical_reads.dedup();
                if canonical_reads.len() > MAX_READ_SCOPES {
                    return Err(ScopePolicyError::TooManyReadScopes {
                        count: canonical_reads.len(),
                        max: MAX_READ_SCOPES,
                    });
                }
                let canonical_write = match write_scope {
                    Some(token) => {
                        validate_scope_token(token, true)?;
                        if canonical_reads.binary_search(token).is_err() {
                            return Err(ScopePolicyError::WriteScopeNotInReadScopes);
                        }
                        Some(token.clone())
                    }
                    None => None,
                };
                Ok(Self::Scoped { read_scopes: canonical_reads, write_scope: canonical_write })
            }
        }
    }

    pub(crate) fn predicate(&self) -> ScopePredicate {
        ScopePredicate::from_policy(self)
    }
}

fn validate_scope_token(token: &str, is_write: bool) -> Result<(), ScopePolicyError> {
    if token.is_empty() {
        return Err(ScopePolicyError::EmptyToken { is_write });
    }
    if token == "*" {
        return Err(ScopePolicyError::WildcardForbidden { is_write });
    }
    if token.len() > MAX_SCOPE_TOKEN_BYTES {
        return Err(ScopePolicyError::TokenTooLong {
            is_write,
            len: token.len(),
            max: MAX_SCOPE_TOKEN_BYTES,
        });
    }
    if token.bytes().any(|byte| byte == 0x1e) {
        return Err(ScopePolicyError::DelimiterForbidden { is_write });
    }
    if token.chars().any(char::is_control) {
        return Err(ScopePolicyError::ControlCharacter { is_write });
    }
    if token.trim() != token {
        return Err(ScopePolicyError::LeadingOrTrailingWhitespace { is_write });
    }
    Ok(())
}

/// Typed SQL predicate and parameter builder derived from an open-validated [`ScopePolicy`].
///
/// For [`ScopePolicy::Unscoped`], `read_scopes()` yields `[""]` and `write_scope()`
/// yields `Some("")` so single-scope queries always bind `scope = ?` (`""`) and hit
/// Schema 39's leading `(scope, ...)` composite indexes uniformly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ScopePredicate {
    is_unscoped: bool,
    read_scopes: Vec<String>,
    write_scope: Option<String>,
    frozen_digest: Option<[u8; 32]>,
}

impl Default for ScopePredicate {
    fn default() -> Self {
        Self::unscoped()
    }
}

impl ScopePredicate {
    pub(crate) fn unscoped() -> Self {
        Self {
            is_unscoped: true,
            read_scopes: vec![String::new()],
            write_scope: Some(String::new()),
            frozen_digest: None,
        }
    }

    pub(crate) fn single_for_query(scope: &str) -> Self {
        Self {
            is_unscoped: scope.is_empty(),
            read_scopes: vec![scope.to_string()],
            write_scope: None,
            frozen_digest: None,
        }
    }

    pub(crate) fn from_policy(policy: &ScopePolicy) -> Self {
        match policy {
            ScopePolicy::Unscoped => Self::unscoped(),
            ScopePolicy::Scoped { read_scopes, write_scope } => {
                let mut sorted = read_scopes.clone();
                sorted.sort();
                sorted.dedup();
                let mut hasher = Sha256::new();
                hasher.update(SCOPE_DIGEST_DOMAIN_V1);
                let count = u32::try_from(sorted.len()).unwrap_or(u32::MAX);
                hasher.update(count.to_le_bytes());
                for scope in &sorted {
                    hasher.update(scope.as_bytes());
                    hasher.update([0x1e]);
                }
                let digest: [u8; 32] = hasher.finalize().into();
                Self {
                    is_unscoped: false,
                    read_scopes: sorted,
                    write_scope: write_scope.clone(),
                    frozen_digest: Some(digest),
                }
            }
        }
    }

    pub(crate) fn is_unscoped(&self) -> bool {
        self.is_unscoped
    }

    pub(crate) fn read_scopes(&self) -> &[String] {
        &self.read_scopes
    }

    pub(crate) fn single_read_scope(&self) -> Option<&str> {
        if self.read_scopes.len() == 1 {
            Some(self.read_scopes[0].as_str())
        } else {
            None
        }
    }

    pub(crate) fn write_scope(&self) -> Option<&str> {
        self.write_scope.as_deref()
    }

    pub(crate) fn require_write_scope(&self, operation: &str) -> Result<&str, EngineError> {
        self.write_scope.as_deref().ok_or_else(|| EngineError::ScopeViolation {
            detail: format!(
                "operation '{operation}' requires a write_scope, but this Engine was opened \
                 with a read-only ScopePolicy (write_scope is None)"
            ),
        })
    }

    pub(crate) fn contains_scope(&self, scope: &str) -> bool {
        self.read_scopes.iter().any(|candidate| candidate == scope)
    }

    pub(crate) fn frozen_digest(&self) -> Option<[u8; 32]> {
        self.frozen_digest
    }

    /// Return `(sql_fragment, next_param_index)` for `column` starting at 1-based `start_index`.
    pub(crate) fn sql_clause(&self, column: &str, start_index: usize) -> (String, usize) {
        if self.read_scopes.len() == 1 {
            (format!("{column} = ?{start_index}"), start_index + 1)
        } else {
            let end_index = start_index + self.read_scopes.len();
            let placeholders: Vec<String> =
                (start_index..end_index).map(|idx| format!("?{idx}")).collect();
            (format!("{column} IN ({})", placeholders.join(", ")), end_index)
        }
    }

    /// Append the typed scope predicate on `column` into `sql` and append matching SQL values.
    pub(crate) fn append_sql_values(
        &self,
        column: &str,
        sql: &mut String,
        params: &mut Vec<Value>,
    ) {
        let start_index = params.len() + 1;
        let (clause, _) = self.sql_clause(column, start_index);
        sql.push_str(&clause);
        self.push_values(params);
    }

    /// Push all bound scope values in the exact order consumed by [`Self::sql_clause`].
    pub(crate) fn push_values(&self, params: &mut Vec<Value>) {
        for scope in &self.read_scopes {
            params.push(Value::Text(scope.clone()));
        }
    }
}

impl Engine {
    /// Return the open-validated, canonicalized caller scope isolation policy
    /// bound to this engine handle.
    pub fn scope_policy(&self) -> &ScopePolicy {
        &self.resolved_config.scope_policy
    }

    pub(crate) fn scope_predicate(&self) -> ScopePredicate {
        self.resolved_config.scope_policy.predicate()
    }
}
