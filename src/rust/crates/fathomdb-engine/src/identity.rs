use super::*;

/// C-2 (0.8.19 / OPP-12 record-lifecycle Phase-1, TC-8) — the **id-space** of a
/// [`SearchHit::id`]. A closed, typed enum (NOT a magic-prefixed string) — the
/// C-2 binding ratified in the OPP-12 protocol:
/// - [`Logical`](IdSpaceKind::Logical) — `"l:"`, a governed/canonical node keyed
///   by its `logical_id` (the only lifecycle-addressable space).
/// - [`Content`](IdSpaceKind::Content) — `"h:"`, a doc-seeded/anonymous node
///   keyed by a content hash of its body (the dominant corpus hit class).
/// - [`Passage`](IdSpaceKind::Passage) — `"p:"`, a synthetic `rerank_passages`
///   hit keyed by the caller-supplied passage ordinal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IdSpaceKind {
    /// `"l:"` — governed/canonical node (its `logical_id`).
    Logical,
    /// `"h:"` — doc-seeded/anonymous node (content hash of the body).
    Content,
    /// `"p:"` — synthetic rerank passage (caller-supplied ordinal).
    Passage,
}

impl IdSpaceKind {
    /// The two-char id-space prefix (`"l:"` / `"h:"` / `"p:"`) used in the
    /// prefixed string form. Byte-identical to the pre-swap `derive_stable_id`
    /// tags so real-gold keying stays a no-op.
    #[must_use]
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Logical => "l:",
            Self::Content => "h:",
            Self::Passage => "p:",
        }
    }

    /// The lowercase discriminant (`"logical"` / `"content"` / `"passage"`)
    /// surfaced through the SDK bindings as the `IdSpace.space` field (mirrors
    /// how `SoftFallbackBranch` is surfaced as a `branch` string).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Logical => "logical",
            Self::Content => "content",
            Self::Passage => "passage",
        }
    }
}

/// C-2 (0.8.19 / OPP-12 Phase-1, TC-8) — the typed, non-null, id-space-**total**
/// carrier for [`SearchHit::id`]. Subsumes the interim `write_cursor` id AND the
/// additive Cause-A `stable_id` field of prior releases: the `value` is the BARE
/// id (prefix stripped), and [`to_prefixed`](IdSpace::to_prefixed) reproduces the
/// pre-swap `stable_id` string byte-for-byte (`l:`/`h:` unchanged) so
/// cross-session real-gold keying continues on `id` as a true no-op.
///
/// Lifecycle-addressability is a type check consumed downstream by the
/// `transition`/`purge` verbs: only [`Logical`](IdSpaceKind::Logical) is
/// lifecycle-addressable; `Content`/`Passage` are total-but-not-addressable.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IdSpace {
    /// The typed id-space (`Logical`/`Content`/`Passage`).
    pub space: IdSpaceKind,
    /// The bare id value (id-space prefix stripped).
    pub value: String,
}

impl IdSpace {
    /// A `Logical` (`"l:"`) id carrying `value` (a `logical_id`).
    pub fn logical(value: impl Into<String>) -> Self {
        Self { space: IdSpaceKind::Logical, value: value.into() }
    }

    /// A `Content` (`"h:"`) id carrying `value` (a content hash).
    pub fn content(value: impl Into<String>) -> Self {
        Self { space: IdSpaceKind::Content, value: value.into() }
    }

    /// A `Passage` (`"p:"`) id carrying `value` (a caller-supplied ordinal).
    pub fn passage(value: impl Into<String>) -> Self {
        Self { space: IdSpaceKind::Passage, value: value.into() }
    }

    /// The prefixed string form (`{prefix}{value}`) — byte-identical to the
    /// pre-swap `derive_stable_id` output for `l:`/`h:`.
    #[must_use]
    pub fn to_prefixed(&self) -> String {
        format!("{}{}", self.space.prefix(), self.value)
    }

    /// Parse the prefixed string form back into a typed `IdSpace`. Round-trip
    /// stable: `IdSpace::parse(&x.to_prefixed()) == Some(x)`. Only the FIRST
    /// two-char id-space prefix is stripped, so a value that itself contains
    /// `":"` round-trips unchanged. Returns `None` for an untagged string.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        if let Some(v) = s.strip_prefix("l:") {
            Some(Self::logical(v))
        } else if let Some(v) = s.strip_prefix("h:") {
            Some(Self::content(v))
        } else {
            s.strip_prefix("p:").map(Self::passage)
        }
    }
}

impl std::fmt::Display for IdSpace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.space.prefix(), self.value)
    }
}

/// 0.8.20 Slice 5c (R-20-E3) — the provenance of a canonical row: which source
/// document it is attributable to, and therefore what `excise_source` must erase
/// when that source is withdrawn.
///
/// **Why a newtype and not `Option<String>`.** Erasure runs through provenance:
/// a row whose `source_id` is NULL is reachable by NO `excise_source` call and
/// is therefore **un-erasable**. Before 0.8.20 the public `PreparedWrite`
/// carried `source_id: Option<String>`, so a caller could express "no
/// provenance" and silently create such a row. A *runtime* rejection would not
/// have closed this: the facade crate re-exports `PreparedWrite` and
/// `Engine::write` is `pub`, so a caller can build the value directly and skip
/// any validation the engine performs. Replacing the field's type is what makes
/// the absence of provenance **inexpressible** rather than merely rejected —
/// the guarantee is enforced by `rustc`, not by a branch. `tests/ui/` in the
/// facade crate holds the compile-fail witness.
///
/// **This is a BREAKING change**, shipped ON by default as part of the 0.8.20
/// coordinated breaking-pair release. There is deliberately no compatibility
/// shim and no deprecation window: a shim would re-open the hole it closes.
///
/// **Reserved namespace.** Ids beginning with `_` belong to the engine and are
/// rejected by [`SourceId::new`]. Two are currently minted internally:
///
/// * [`SourceId::ENGINE_PREFIX`] (`_engine:`) — rows the engine derives for
///   itself (EXP-S coverage/graph substrate rows), which never pass through
///   `PreparedWrite` (design §4 item 6).
/// * [`SourceId::LEGACY_PRE_0_8_20`] (`_legacy:pre-0.8.20`) — stamped by schema
///   migration step 21 onto pre-0.8.20 rows that were stored with NULL
///   provenance, so they become erasable (R-20-E8). **Gated to UNGOVERNED rows
///   only** (`logical_id IS NULL`); a governed row keeps NULL `source_id` and
///   stays `purge`-addressable by its `logical_id` (TC-11 pin).
///
/// **`source_id` must not be PII.** It survives the erasure it authorises: the
/// `excise_source` audit row in `operational_mutations` records it verbatim, and
/// while 0.8.20 makes that audit row durable (design §2 defect D-A) the rule was
/// always that the handle you erase BY must not itself be the thing needing
/// erasure. Use an opaque document id, not an email address.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceId(pub(crate) String);

impl SourceId {
    /// Reserved prefix for engine-derived rows (design §4 item 6).
    pub const ENGINE_PREFIX: &'static str = "_engine:";

    /// Reserved provenance stamped by schema migration step 21 onto pre-0.8.20
    /// UNGOVERNED rows that were stored with NULL provenance (R-20-E8).
    pub const LEGACY_PRE_0_8_20: &'static str = "_legacy:pre-0.8.20";

    /// The single public constructor. Rejects the two ways a caller could
    /// express "effectively no provenance":
    ///
    /// * an empty or whitespace-only id — it names no source, and
    ///   `excise_source` already refuses the empty string, so such a row would
    ///   be un-erasable in practice;
    /// * an id in the engine's reserved `_`-prefixed namespace — a caller who
    ///   could mint `_legacy:pre-0.8.20` could hide rows among the migration's
    ///   back-filled ones, or mint `_engine:` rows that read as engine
    ///   substrate.
    ///
    /// # Errors
    ///
    /// [`EngineError::WriteValidation`] for either rejection above.
    pub fn new(id: impl Into<String>) -> Result<Self, EngineError> {
        let id = id.into();
        if id.trim().is_empty() || id.starts_with('_') {
            return Err(EngineError::WriteValidation);
        }
        Ok(Self(id))
    }

    /// Mint a reserved `_engine:*` provenance for an engine-derived row. Crate
    /// -internal by construction: the reserved namespace is exactly what
    /// [`SourceId::new`] refuses, so a caller cannot reach this spelling.
    pub(crate) fn engine_derived(role: &str) -> Self {
        Self(format!("{}{role}", Self::ENGINE_PREFIX))
    }

    /// The on-disk `source_id` text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume into the owned on-disk text.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for SourceId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for SourceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for SourceId {
    type Error = EngineError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for SourceId {
    type Error = EngineError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

pub(crate) fn valid_caller_identity(value: &str) -> bool {
    let bytes = value.as_bytes();
    !value.starts_with("_fdb:")
        && (1..=128).contains(&bytes.len())
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

macro_rules! caller_identity_newtype {
    ($name:ident, $reason:ident, $path:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub(crate) String);

        impl $name {
            /// Construct a caller-authored identifier in the closed v1 grammar.
            ///
            /// # Errors
            ///
            /// Returns a typed provenance error when the value is empty, too
            /// long, reserved, or contains a character outside the grammar.
            pub fn new(value: impl Into<String>) -> Result<Self, EngineError> {
                let value = value.into();
                if !valid_caller_identity(&value) {
                    return Err(ProvenanceError::new(ProvenanceErrorReason::$reason, $path).into());
                }
                Ok(Self(value))
            }

            /// Return the exact stored identifier.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

caller_identity_newtype!(
    ArtifactRevisionId,
    RevisionIdInvalid,
    "/provenance/artifactRevisionId",
    "Immutable identity of one canonical or derived artifact revision."
);

/// Caller-authored immutable identity of one registered source dependency.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DependencyId(pub(crate) String);

impl DependencyId {
    /// Construct a dependency identifier in the closed caller-ID grammar.
    ///
    /// # Errors
    ///
    /// Returns `dependency_id_invalid` when the value is outside the grammar.
    pub fn new(value: impl Into<String>) -> Result<Self, DependencyError> {
        let value = value.into();
        if !valid_caller_identity(&value) {
            return Err(DependencyError::new(
                DependencyErrorReason::DependencyIdInvalid,
                "/dependencyId",
            ));
        }
        Ok(Self(value))
    }

    /// Return the exact stored identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

caller_identity_newtype!(
    SourceRevisionId,
    RevisionIdInvalid,
    "/provenance/sourceRevisionId",
    "Immutable identity of the canonical source revision for a derived artifact."
);
caller_identity_newtype!(
    SourceVersionId,
    SourceVersionInvalid,
    "/provenance/sourceVersionId",
    "Caller-authored version identity scoped to one source id."
);

/// SHA-256 digest of the entire canonical source revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalHash {
    digest_hex: String,
}

impl CanonicalHash {
    /// Construct a canonical SHA-256 hash from exactly 64 lowercase hex digits.
    ///
    /// # Errors
    ///
    /// Returns `hash_invalid` for any other spelling.
    pub fn sha256(digest_hex: impl Into<String>) -> Result<Self, EngineError> {
        let digest_hex = digest_hex.into();
        if digest_hex.len() != 64
            || !digest_hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(ProvenanceError::new(
                ProvenanceErrorReason::HashInvalid,
                "/provenance/canonicalSourceHash/digestHex",
            )
            .into());
        }
        Ok(Self { digest_hex })
    }

    /// Return the lowercase SHA-256 digest.
    #[must_use]
    pub fn digest_hex(&self) -> &str {
        &self.digest_hex
    }
}

/// G11 (Slice 15) — derive a stable hex-encoded sha256 logical_id from a
/// `(kind, name)` pair. Both inputs are lowercased before hashing so that
/// entity identity is case-insensitive (`"Alice"` == `"alice"`). The
/// canonical form is `sha256("<kind>:<name>")` — identical to the
/// ADR-0.8.1-byo-llm derivation rule.
///
/// fix-34 [P1]: because `:` is the delimiter, a `:` in `kind` would let the
/// split point move and collide two distinct `(kind, name)` pairs onto one
/// identity (e.g. `("a:b","c")` and `("a","b:c")` both hash `"a:b:c"`),
/// silently dropping one entity via batch dedup / G0 supersession. An empty
/// `name` collapses every name-less entity of a kind onto `sha256("<kind>:")`.
/// We reject both at the boundary; this preserves the ADR derivation rule
/// (a colon-free `kind` makes the first `:` an unambiguous delimiter, so a `:`
/// in `name` stays safe — edge keys deliberately rely on that).
pub(super) fn derive_logical_id(kind: &str, name: &str) -> Result<String, EngineError> {
    if kind.contains(':') || name.is_empty() {
        return Err(EngineError::Extractor);
    }
    let input = format!("{}:{}", kind.to_lowercase(), name.to_lowercase());
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    // digest 0.11 returns `hybrid_array::Array`, which (unlike the old
    // `GenericArray`) does not implement `LowerHex`. Format the bytes
    // explicitly — byte-identical lowercase, zero-padded hex to the prior
    // `{:x}` rendering, preserving the load-bearing logical-id derivation.
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Cause-A (0.8.11.2) / C-2 (0.8.19, TC-8) — derive the typed **stable hit-id**
/// ([`IdSpace`]) carried on [`SearchHit::id`] for cross-session real-gold keying.
///
/// The stable id is the active canonical node's `logical_id` — the post-G0
/// supersession-stable identity, preserved across re-projection/re-ingest by the
/// tombstone-then-insert contract (whereas the engine-internal `write_cursor` is
/// reassigned on every re-ingest). When `logical_id` is NULL — the doc-seeded
/// node case, the *dominant* corpus hit type today — we fall back to a content
/// hash of the body so doc hits still carry a re-ingest-survivable key.
///
/// The result is a typed [`IdSpace`]; its `to_prefixed()` reproduces the pre-C-2
/// `stable_id` string byte-for-byte so real-gold keying is a no-op:
/// - [`IdSpace::logical`] (`"l:<logical_id>"`) — entities + edges (graph-arm,
///   vector-node, and edge hits when `logical_id` is present);
/// - [`IdSpace::content`] (`"h:<sha256(body)>"`) — doc nodes with NULL
///   `logical_id`, and any branch that cannot cheaply resolve a `logical_id`.
///
/// Behaviour-neutral: the value never participates in ranking/scoring (same
/// additive posture as `source_id` / `ce_score`).
pub(super) fn derive_stable_id(logical_id: Option<&str>, body: &str) -> IdSpace {
    match logical_id {
        Some(lid) if !lid.is_empty() => IdSpace::logical(lid),
        _ => {
            let mut hasher = Sha256::new();
            hasher.update(body.as_bytes());
            IdSpace::content(
                hasher.finalize().iter().map(|b| format!("{b:02x}")).collect::<String>(),
            )
        }
    }
}
