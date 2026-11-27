use std::fmt::{Display, Formatter};
#[cfg(feature = "test-hooks")]
use std::sync::atomic::Ordering;
#[cfg(feature = "test-hooks")]
use std::time::Instant;

use rusqlite::{Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::dependency::load_dependency_generation;
use crate::errors::EngineError;
use crate::filter::SearchFilter;
#[cfg(feature = "test-hooks")]
use crate::pagination::{self, PageRequestV1};
use crate::projection_commit::load_projection_cursor;
use crate::temporal::{current_epoch_seconds, ReadView};
use crate::write_commit::load_next_cursor;
use crate::Engine;

pub(crate) const FROZEN_READ_SCHEMA_VERSION: u32 = 1;
const TOKEN_PREFIX: &str = "fdbfr1";
const TOKEN_MAX_BYTES: usize = 1024;
const CONTEXT_MAX_BYTES: usize = 64 * 1024;
const CONTEXT_MAX_ATTRIBUTES: usize = 64;
const CONTEXT_DOMAIN: &[u8] = b"fathomdb.read-context.v1\0";
const TOKEN_DOMAIN: &[u8] = b"fathomdb.frozen-read.v1\0";
pub(crate) const PAGE_CURSOR_DOMAIN: &[u8] = b"fathomdb.page-cursor.v1\0";
const REGISTRY_DOMAIN: &[u8] = b"fathomdb.projection-registry-binding.v1\0";
const SERVING_DOMAIN_V1: &[u8] = b"fathomdb.projection-serving-binding.v1\0";
const SERVING_DOMAIN_V2: &[u8] = b"fathomdb.projection-serving-binding.v2\0";
const SERVING_DOMAIN_V3: &[u8] = b"fathomdb.projection-serving-binding.v3\0";
const SERVING_DOMAIN_V4: &[u8] = b"fathomdb.projection-serving-binding.v4\0";
const WATERMARK_TAG_DOMAIN_V1: &[u8] = b"fathomdb.visibility-watermark-tag.v1\0";
const SURFACE_TAGS_MAGIC: [u8; 2] = [0x53, 0x39];
const DATABASE_ID_KEY: &str = "_fathomdb_database_id";
const READ_CONTEXT_KEY: &str = "_fathomdb_read_context_key";

/// Versioned search eligibility and validity requested by a caller.
#[derive(Clone, Debug, PartialEq)]
pub struct ReadContextV1 {
    /// Wire-schema version. Version 1 is the only accepted value.
    pub schema_version: u32,
    /// Lifecycle and temporal visibility constraints.
    pub view: ReadView,
    /// Indexed, allowlisted eligibility applied before candidate truncation.
    pub eligibility: SearchFilter,
}

impl ReadContextV1 {
    /// Construct and validate a version-1 read context.
    pub fn new(view: ReadView, eligibility: SearchFilter) -> Result<Self, EngineError> {
        let context = Self { schema_version: FROZEN_READ_SCHEMA_VERSION, view, eligibility };
        validate_context(&context)?;
        Ok(context)
    }
}

/// Engine-minted authenticated context bound to one database read state.
#[derive(Clone, Debug, PartialEq)]
pub struct FrozenReadContextV1 {
    /// Wire-schema version. Version 1 is the only accepted value.
    pub schema_version: u32,
    /// The single validity instant resolved when the context was minted.
    pub effective_valid_at: i64,
    /// Echoed caller context authenticated by `token`.
    pub context: ReadContextV1,
    /// Content-free database-local authenticated state binding.
    pub token: String,
}

/// Closed reason vocabulary for frozen-read refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrozenReadErrorReason {
    UnsupportedSchemaVersion,
    TokenMalformed,
    TokenTooLarge,
    TokenAuthenticationFailed,
    DatabaseMismatch,
    ContextInvalid,
    StateUnavailable,
    StateDrifted,
}

impl FrozenReadErrorReason {
    /// Stable snake-case wire value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedSchemaVersion => "unsupported_schema_version",
            Self::TokenMalformed => "token_malformed",
            Self::TokenTooLarge => "token_too_large",
            Self::TokenAuthenticationFailed => "token_authentication_failed",
            Self::DatabaseMismatch => "database_mismatch",
            Self::ContextInvalid => "context_invalid",
            Self::StateUnavailable => "state_unavailable",
            Self::StateDrifted => "state_drifted",
        }
    }
}

/// Typed frozen-read refusal without token or eligibility disclosure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenReadError {
    /// Closed refusal reason.
    pub reason: FrozenReadErrorReason,
    /// RFC 6901 field path identifying the invalid request component.
    pub field_path: String,
}

impl FrozenReadError {
    fn new(reason: FrozenReadErrorReason, field_path: impl Into<String>) -> Self {
        Self { reason, field_path: field_path.into() }
    }
}

impl Display for FrozenReadError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} at {}", self.reason.as_str(), self.field_path)
    }
}

impl std::error::Error for FrozenReadError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReadSurface<'a> {
    Preflight,
    SearchNoGraph,
    GraphOrEdge,
    EvidenceNode,
    EvidenceGraph,
    CanonicalPage { path_kind: &'a str },
    OperationalState,
    TraceDependency,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SurfaceTags {
    pub(crate) op_state_tag: u64,
    pub(crate) node_all_tag: u64,
    pub(crate) projection_readiness_tag: u64,
    pub(crate) edge_kind_tag: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrozenReadBinding {
    database_id: String,
    effective_valid_at: i64,
    canonical_write_boundary: u64,
    read_visibility_generation: u64,
    dependency_generation: u64,
    projection_cursor: u64,
    registry_digest: [u8; 32],
    serving_digest: [u8; 32],
    context_digest: [u8; 32],
    scope_digest: Option<[u8; 32]>,
    surface_tags: Option<SurfaceTags>,
}

#[cfg(feature = "test-hooks")]
pub(crate) struct MintStageTiming {
    pub context_validation_ns: u128,
    pub snapshot_validation_ns: u128,
    pub binding_ns: u128,
    pub token_codec_ns: u128,
}

pub(crate) fn validate_context(context: &ReadContextV1) -> Result<Vec<u8>, FrozenReadError> {
    if context.schema_version != FROZEN_READ_SCHEMA_VERSION {
        return Err(FrozenReadError::new(
            FrozenReadErrorReason::UnsupportedSchemaVersion,
            "/schemaVersion",
        ));
    }
    if context.eligibility.attributes.len() > CONTEXT_MAX_ATTRIBUTES {
        return Err(FrozenReadError::new(
            FrozenReadErrorReason::ContextInvalid,
            "/eligibility/attributes",
        ));
    }
    if context.eligibility.property_predicates.len() > CONTEXT_MAX_ATTRIBUTES {
        return Err(FrozenReadError::new(
            FrozenReadErrorReason::ContextInvalid,
            "/eligibility/propertyPredicates",
        ));
    }
    let bytes = encode_context(context);
    if bytes.len() > CONTEXT_MAX_BYTES {
        return Err(FrozenReadError::new(FrozenReadErrorReason::ContextInvalid, "/eligibility"));
    }
    Ok(bytes)
}

pub(crate) fn mint(
    connection: &mut Connection,
    context: &ReadContextV1,
    scope: &crate::scope::ScopePredicate,
) -> Result<(FrozenReadContextV1, u64), EngineError> {
    mint_inner(connection, context, scope).map(|(frozen, generation, _)| (frozen, generation))
}

#[cfg(feature = "test-hooks")]
pub(crate) fn mint_measured(
    connection: &mut Connection,
    context: &ReadContextV1,
    scope: &crate::scope::ScopePredicate,
) -> Result<(FrozenReadContextV1, u64, MintStageTiming), EngineError> {
    let (frozen, generation, timing) = mint_inner(connection, context, scope)?;
    Ok((
        frozen,
        generation,
        MintStageTiming {
            context_validation_ns: timing[0],
            snapshot_validation_ns: timing[1],
            binding_ns: timing[2],
            token_codec_ns: timing[3],
        },
    ))
}

fn mint_inner(
    connection: &mut Connection,
    context: &ReadContextV1,
    scope: &crate::scope::ScopePredicate,
) -> Result<(FrozenReadContextV1, u64, [u128; 4]), EngineError> {
    #[cfg(feature = "test-hooks")]
    let context_started = Instant::now();
    validate_context(context)?;
    let mut resolved = context.clone();
    let effective_valid_at = resolved.view.valid_as_of.unwrap_or_else(current_epoch_seconds);
    resolved.view.valid_as_of = Some(effective_valid_at);
    let context_bytes = validate_context(&resolved)?;
    #[cfg(feature = "test-hooks")]
    let context_validation_ns = context_started.elapsed().as_nanos();
    #[cfg(not(feature = "test-hooks"))]
    let context_validation_ns = 0;
    #[cfg(feature = "test-hooks")]
    let snapshot_started = Instant::now();
    let tx = connection.transaction().map_err(|_| EngineError::Storage)?;
    tx.prepare_cached("SELECT generation FROM _fathomdb_read_visibility_state WHERE singleton = 1")
        .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
        .map_err(|_| EngineError::Storage)?;
    crate::filter::validate_filter_attributes_on_snapshot(&tx, &resolved.eligibility).map_err(
        |error| match error {
            crate::filter::SnapshotFilterError::InvalidFilter(reason) => {
                EngineError::InvalidFilter { reason }
            }
            crate::filter::SnapshotFilterError::Sqlite(_) => EngineError::Storage,
        },
    )?;
    #[cfg(feature = "test-hooks")]
    let snapshot_validation_ns = snapshot_started.elapsed().as_nanos();
    #[cfg(not(feature = "test-hooks"))]
    let snapshot_validation_ns = 0;
    #[cfg(feature = "test-hooks")]
    let binding_started = Instant::now();
    let mut binding = binding_for_snapshot(
        &tx,
        &resolved,
        effective_valid_at,
        digest(CONTEXT_DOMAIN, &context_bytes),
        scope,
    )?;
    let monotonic_generation = load_visibility_generation(&tx)?;
    #[cfg(feature = "test-hooks")]
    let binding_ns = binding_started.elapsed().as_nanos();
    #[cfg(not(feature = "test-hooks"))]
    let binding_ns = 0;
    #[cfg(feature = "test-hooks")]
    let codec_started = Instant::now();
    let key = read_hex_open_state(&tx, READ_CONTEXT_KEY, 32)?;
    let payload = encode_binding(&binding);
    let mac = hmac_sha256(&key, TOKEN_DOMAIN, &payload);
    let token = format!("{TOKEN_PREFIX}.{}.{}", hex_encode(&payload), hex_encode(&mac));
    if token.len() > TOKEN_MAX_BYTES {
        return Err(FrozenReadError::new(FrozenReadErrorReason::TokenTooLarge, "/token").into());
    }
    #[cfg(feature = "test-hooks")]
    let token_codec_ns = codec_started.elapsed().as_nanos();
    #[cfg(not(feature = "test-hooks"))]
    let token_codec_ns = 0;
    tx.commit().map_err(|_| EngineError::Storage)?;
    binding.context_digest = digest(CONTEXT_DOMAIN, &context_bytes);
    Ok((
        FrozenReadContextV1 {
            schema_version: FROZEN_READ_SCHEMA_VERSION,
            effective_valid_at,
            context: resolved,
            token,
        },
        monotonic_generation,
        [context_validation_ns, snapshot_validation_ns, binding_ns, token_codec_ns],
    ))
}

pub(crate) fn authenticate(
    connection: &Connection,
    frozen: &FrozenReadContextV1,
    scope: &crate::scope::ScopePredicate,
) -> Result<FrozenReadBinding, EngineError> {
    if frozen.schema_version != FROZEN_READ_SCHEMA_VERSION {
        return Err(FrozenReadError::new(
            FrozenReadErrorReason::UnsupportedSchemaVersion,
            "/schemaVersion",
        )
        .into());
    }
    if frozen.token.len() > TOKEN_MAX_BYTES {
        return Err(FrozenReadError::new(FrozenReadErrorReason::TokenTooLarge, "/token").into());
    }
    let mut pieces = frozen.token.split('.');
    let (Some(prefix), Some(payload_hex), Some(mac_hex), None) =
        (pieces.next(), pieces.next(), pieces.next(), pieces.next())
    else {
        return Err(FrozenReadError::new(FrozenReadErrorReason::TokenMalformed, "/token").into());
    };
    if prefix != TOKEN_PREFIX {
        return Err(FrozenReadError::new(FrozenReadErrorReason::TokenMalformed, "/token").into());
    }
    let payload = hex_decode(payload_hex)
        .ok_or_else(|| FrozenReadError::new(FrozenReadErrorReason::TokenMalformed, "/token"))?;
    let supplied_mac = hex_decode(mac_hex)
        .ok_or_else(|| FrozenReadError::new(FrozenReadErrorReason::TokenMalformed, "/token"))?;
    let binding = decode_binding(&payload)?;
    let key = read_hex_open_state(connection, READ_CONTEXT_KEY, 32)?;
    let expected_mac = hmac_sha256(&key, TOKEN_DOMAIN, &payload);
    if !constant_time_eq(&supplied_mac, &expected_mac) {
        return Err(FrozenReadError::new(
            FrozenReadErrorReason::TokenAuthenticationFailed,
            "/token",
        )
        .into());
    }
    let database_id = read_open_state(connection, DATABASE_ID_KEY)?;
    if binding.database_id != database_id {
        return Err(FrozenReadError::new(FrozenReadErrorReason::DatabaseMismatch, "/token").into());
    }
    if binding.scope_digest != scope.frozen_digest() {
        return Err(FrozenReadError::new(
            FrozenReadErrorReason::TokenAuthenticationFailed,
            "/token",
        )
        .into());
    }
    let context_bytes = validate_context(&frozen.context)?;
    if frozen.effective_valid_at != binding.effective_valid_at
        || frozen.context.view.valid_as_of != Some(binding.effective_valid_at)
        || digest(CONTEXT_DOMAIN, &context_bytes) != binding.context_digest
    {
        return Err(FrozenReadError::new(FrozenReadErrorReason::ContextInvalid, "/context").into());
    }
    Ok(binding)
}

pub(crate) fn validate_snapshot(
    connection: &Connection,
    context: &ReadContextV1,
    expected: &FrozenReadBinding,
    scope: &crate::scope::ScopePredicate,
    surface: ReadSurface<'_>,
) -> Result<u64, FrozenReadError> {
    let schema_version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| FrozenReadError::new(FrozenReadErrorReason::StateUnavailable, "/token"))?;
    if schema_version < 39 {
        let actual = binding_for_snapshot(
            connection,
            context,
            expected.effective_valid_at,
            expected.context_digest,
            scope,
        )
        .map_err(|_| FrozenReadError::new(FrozenReadErrorReason::StateUnavailable, "/token"))?;
        if actual != *expected {
            return Err(FrozenReadError::new(FrozenReadErrorReason::StateDrifted, "/token"));
        }
        return load_visibility_generation(connection)
            .map_err(|_| FrozenReadError::new(FrozenReadErrorReason::StateUnavailable, "/token"));
    }

    require_clean_visibility(connection)
        .map_err(|_| FrozenReadError::new(FrozenReadErrorReason::StateUnavailable, "/token"))?;
    let Some(expected_tags) = expected.surface_tags else {
        return Err(FrozenReadError::new(FrozenReadErrorReason::StateDrifted, "/token"));
    };
    let read_scopes = sorted_read_scopes(scope);
    let actual_db_id = read_open_state(connection, DATABASE_ID_KEY)
        .map_err(|_| FrozenReadError::new(FrozenReadErrorReason::StateUnavailable, "/token"))?;
    if actual_db_id != expected.database_id || scope.frozen_digest() != expected.scope_digest {
        return Err(FrozenReadError::new(FrozenReadErrorReason::StateDrifted, "/token"));
    }
    let actual_registry = projection_registry_digest(connection)
        .map_err(|_| FrozenReadError::new(FrozenReadErrorReason::StateUnavailable, "/token"))?;
    if actual_registry != expected.registry_digest {
        return Err(FrozenReadError::new(FrozenReadErrorReason::StateDrifted, "/token"));
    }
    let actual_serving = serving_digest_v4(connection, &read_scopes)
        .map_err(|_| FrozenReadError::new(FrozenReadErrorReason::StateUnavailable, "/token"))?;
    if actual_serving != expected.serving_digest {
        return Err(FrozenReadError::new(FrozenReadErrorReason::StateDrifted, "/token"));
    }

    let kind_is_none = context.eligibility.kind.is_none();
    let drifted = || FrozenReadError::new(FrozenReadErrorReason::StateDrifted, "/token");
    let unavailable = |_| FrozenReadError::new(FrozenReadErrorReason::StateUnavailable, "/token");

    match surface {
        ReadSurface::OperationalState => {
            let op_state_tag =
                compute_partition_tag(connection, &read_scopes, "op_state", "op_state")
                    .map_err(unavailable)?;
            if op_state_tag != expected_tags.op_state_tag {
                return Err(drifted());
            }
        }
        ReadSurface::CanonicalPage { path_kind: _ } => {
            let (boundary, node_tag) = compute_node_partition(
                connection,
                &read_scopes,
                context.eligibility.kind.as_deref(),
            )
            .map_err(unavailable)?;
            if boundary != expected.canonical_write_boundary
                || node_tag != expected.read_visibility_generation
            {
                return Err(drifted());
            }
        }
        ReadSurface::Preflight | ReadSurface::SearchNoGraph | ReadSurface::EvidenceNode => {
            let (boundary, node_tag) = compute_node_partition(
                connection,
                &read_scopes,
                context.eligibility.kind.as_deref(),
            )
            .map_err(unavailable)?;
            if boundary != expected.canonical_write_boundary
                || node_tag != expected.read_visibility_generation
            {
                return Err(drifted());
            }
            let vec_pk = vector_ready_partition_kind(context.eligibility.kind.as_deref());
            let vec_tag = compute_partition_tag(connection, &read_scopes, &vec_pk, "vector_ready")
                .map_err(unavailable)?;
            if vec_tag != expected_tags.projection_readiness_tag {
                return Err(drifted());
            }
            let edge_pk = edge_kind_partition_kind(context.eligibility.kind.as_deref());
            let edge_tag = compute_partition_tag(connection, &read_scopes, &edge_pk, "edge_kind")
                .map_err(unavailable)?;
            if edge_tag != expected_tags.edge_kind_tag {
                return Err(drifted());
            }
            let dep_tag = compute_partition_tag(
                connection,
                &read_scopes,
                "scope_dependency",
                "scope_dependency",
            )
            .map_err(unavailable)?;
            if dep_tag != expected.dependency_generation {
                return Err(drifted());
            }
            if kind_is_none {
                let edge_any_tag =
                    compute_partition_tag(connection, &read_scopes, "edge_any", "edge_any")
                        .map_err(unavailable)?;
                if edge_any_tag != expected.projection_cursor {
                    return Err(drifted());
                }
                let node_all_tag =
                    compute_partition_tag(connection, &read_scopes, "node_all", "node_all")
                        .map_err(unavailable)?;
                if node_all_tag != expected_tags.node_all_tag {
                    return Err(drifted());
                }
            }
        }
        ReadSurface::GraphOrEdge | ReadSurface::EvidenceGraph => {
            let (boundary, node_tag) = compute_node_partition(
                connection,
                &read_scopes,
                context.eligibility.kind.as_deref(),
            )
            .map_err(unavailable)?;
            if boundary != expected.canonical_write_boundary
                || node_tag != expected.read_visibility_generation
            {
                return Err(drifted());
            }
            let vec_pk = vector_ready_partition_kind(context.eligibility.kind.as_deref());
            let vec_tag = compute_partition_tag(connection, &read_scopes, &vec_pk, "vector_ready")
                .map_err(unavailable)?;
            if vec_tag != expected_tags.projection_readiness_tag {
                return Err(drifted());
            }
            let edge_pk = edge_kind_partition_kind(context.eligibility.kind.as_deref());
            let edge_tag = compute_partition_tag(connection, &read_scopes, &edge_pk, "edge_kind")
                .map_err(unavailable)?;
            if edge_tag != expected_tags.edge_kind_tag {
                return Err(drifted());
            }
            let dep_tag = compute_partition_tag(
                connection,
                &read_scopes,
                "scope_dependency",
                "scope_dependency",
            )
            .map_err(unavailable)?;
            if dep_tag != expected.dependency_generation {
                return Err(drifted());
            }
            let edge_any_tag =
                compute_partition_tag(connection, &read_scopes, "edge_any", "edge_any")
                    .map_err(unavailable)?;
            if edge_any_tag != expected.projection_cursor {
                return Err(drifted());
            }
            if kind_is_none {
                let node_all_tag =
                    compute_partition_tag(connection, &read_scopes, "node_all", "node_all")
                        .map_err(unavailable)?;
                if node_all_tag != expected_tags.node_all_tag {
                    return Err(drifted());
                }
            }
        }
        ReadSurface::TraceDependency => {
            let node_all_tag =
                compute_partition_tag(connection, &read_scopes, "node_all", "node_all")
                    .map_err(unavailable)?;
            if node_all_tag != expected_tags.node_all_tag {
                return Err(drifted());
            }
            let edge_any_tag =
                compute_partition_tag(connection, &read_scopes, "edge_any", "edge_any")
                    .map_err(unavailable)?;
            if edge_any_tag != expected.projection_cursor {
                return Err(drifted());
            }
            let dep_tag = compute_partition_tag(
                connection,
                &read_scopes,
                "scope_dependency",
                "scope_dependency",
            )
            .map_err(unavailable)?;
            if dep_tag != expected.dependency_generation {
                return Err(drifted());
            }
        }
    }

    load_visibility_generation(connection).map_err(unavailable)
}

pub(crate) fn load_visibility_generation(connection: &Connection) -> Result<u64, EngineError> {
    let generation = connection
        .query_row(
            "SELECT generation FROM _fathomdb_read_visibility_state WHERE singleton=1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| EngineError::Storage)?;
    u64::try_from(generation).map_err(|_| EngineError::Storage)
}

fn load_visibility_nonce(connection: &Connection) -> Result<String, EngineError> {
    connection
        .query_row(
            "SELECT state_nonce FROM _fathomdb_read_visibility_state WHERE singleton=1",
            [],
            |row| row.get::<_, String>(0),
        )
        .map_err(|_| EngineError::Storage)
}

#[cfg(any(test, feature = "slice30-observer"))]
pub(crate) fn slice30_authority_for_test(
    connection: &Connection,
) -> Result<(u64, String, u64), EngineError> {
    Ok((
        load_visibility_generation(connection)?,
        load_visibility_nonce(connection)?,
        load_dependency_generation(connection)?,
    ))
}

pub(crate) fn begin_visibility_writer(
    connection: &Connection,
) -> rusqlite::Result<rusqlite::Transaction<'_>> {
    let tx =
        rusqlite::Transaction::new_unchecked(connection, rusqlite::TransactionBehavior::Immediate)?;
    require_clean_visibility(&tx).map_err(|_| rusqlite::Error::InvalidQuery)?;
    Ok(tx)
}

pub(crate) fn require_clean_visibility(connection: &Connection) -> Result<(), EngineError> {
    let flag = connection
        .prepare_cached("SELECT coalescing FROM _fathomdb_read_visibility_state WHERE singleton=1")
        .and_then(|mut statement| statement.query_row([], |row| row.get::<_, i64>(0)))
        .map_err(|_| EngineError::Storage)?;
    if flag != 0 {
        return Err(EngineError::Storage);
    }
    Ok(())
}

static EXPECTED_VISIBILITY_TRIGGERS_CURRENT: std::sync::LazyLock<Vec<(String, String)>> =
    std::sync::LazyLock::new(|| {
        let schema_version = fathomdb_schema::SCHEMA_VERSION;
        let mut triggers = Vec::with_capacity(fathomdb_schema::TABLE_MANIFEST.len() * 3);
        for entry in fathomdb_schema::TABLE_MANIFEST.iter().filter(|entry| {
            entry.visibility_trigger_prefix.is_some()
                && entry.visibility_trigger_min_version.is_some_and(|min| min <= schema_version)
        }) {
            let short = entry.visibility_trigger_prefix.expect("checked some");
            for (suffix, operation) in [("ai", "INSERT"), ("au", "UPDATE"), ("ad", "DELETE")] {
                let name = format!("_fathomdb_read_visibility_{short}_{suffix}");
                let expected = fathomdb_schema::expected_visibility_trigger_sql(
                    entry,
                    suffix,
                    operation,
                    schema_version,
                );
                triggers.push((name, expected));
            }
        }
        triggers
    });

fn matches_visibility_trigger_like(name: &str) -> bool {
    const PATTERN: &[u8] = b"_fathomdb_read_visibility_";
    let bytes = name.as_bytes();
    if bytes.len() < PATTERN.len() {
        return false;
    }
    bytes.iter().zip(PATTERN.iter()).all(|(&b, &p)| p == b'_' || b.eq_ignore_ascii_case(&p))
}

pub(crate) fn validate_on_open(connection: &Connection, schema_version: u32) -> Result<(), String> {
    if schema_version < 31 {
        return Ok(());
    }
    let mut master_tables: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut master_indexes: std::collections::HashMap<(String, String), String> =
        std::collections::HashMap::new();
    let mut master_triggers: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    {
        let mut stmt = connection
            .prepare_cached(
                "SELECT type, name, tbl_name, sql FROM sqlite_master \
                 WHERE type IN ('table','index','trigger') AND sql IS NOT NULL",
            )
            .map_err(|error| error.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        for row in rows {
            let (obj_type, name, tbl_name, sql) = row.map_err(|error| error.to_string())?;
            match obj_type.as_str() {
                "table" => {
                    master_tables.insert(name, sql);
                }
                "index" => {
                    master_indexes.insert((name, tbl_name), sql);
                }
                "trigger" => {
                    master_triggers.insert(name, sql);
                }
                _ => {}
            }
        }
    }

    if schema_version >= 37 {
        require_clean_visibility(connection).map_err(|error| error.to_string())?;
        let layout = master_tables
            .get("_fathomdb_read_visibility_state")
            .ok_or_else(|| "missing _fathomdb_read_visibility_state table".to_string())?;
        if !normalize_sql(layout).contains("coalescing INTEGER NOT NULL DEFAULT 0 CHECK(typeof(coalescing)='integer' AND coalescing IN(0,1))") {
            return Err("invalid read visibility coalescing layout".to_string());
        }
    }
    let database_id =
        read_open_state(connection, DATABASE_ID_KEY).map_err(|error| error.to_string())?;
    if !is_lower_hex(&database_id, 32) {
        return Err("invalid frozen-read database identity".to_string());
    }
    read_hex_open_state(connection, READ_CONTEXT_KEY, 32).map_err(|error| error.to_string())?;
    load_visibility_generation(connection).map_err(|error| error.to_string())?;
    if schema_version >= 33 {
        let nonce = load_visibility_nonce(connection).map_err(|error| error.to_string())?;
        if !is_lower_hex(&nonce, 64) {
            return Err("invalid frozen-read branch-sensitive state".to_string());
        }
    }
    if schema_version >= 39 {
        let (gs_gen, gs_nonce) =
            load_global_struct_state(connection).map_err(|error| error.to_string())?;
        let _ = gs_gen;
        if !is_lower_hex(&gs_nonce, 64) {
            return Err("invalid frozen-read global structural state".to_string());
        }
        if !master_tables.contains_key("_fathomdb_scope_visibility_watermarks") {
            return Err("missing _fathomdb_scope_visibility_watermarks table".to_string());
        }
        connection
            .prepare_cached(
                "SELECT scope,partition_kind,add_generation,max_cursor,destructive_generation,state_nonce \
                 FROM _fathomdb_scope_visibility_watermarks LIMIT 0",
            )
            .map_err(|error| format!("invalid _fathomdb_scope_visibility_watermarks layout: {error}"))?;
    }
    let expected_trigger_count: usize;
    if schema_version == fathomdb_schema::SCHEMA_VERSION {
        let expected_triggers = &*EXPECTED_VISIBILITY_TRIGGERS_CURRENT;
        expected_trigger_count = expected_triggers.len();
        for (name, expected) in expected_triggers {
            let sql = master_triggers
                .get(name.as_str())
                .ok_or_else(|| format!("missing frozen-read visibility trigger {name}"))?;
            if !eq_collapsed_ws(sql, expected) {
                return Err(format!("frozen-read visibility trigger {name} differs from manifest"));
            }
        }
    } else {
        let mut trigger_table_count = 0usize;
        for entry in fathomdb_schema::TABLE_MANIFEST.iter().filter(|entry| {
            entry.visibility_trigger_prefix.is_some()
                && entry.visibility_trigger_min_version.is_some_and(|min| min <= schema_version)
        }) {
            trigger_table_count += 1;
            let short = entry.visibility_trigger_prefix.expect("checked some");
            for (suffix, operation) in [("ai", "INSERT"), ("au", "UPDATE"), ("ad", "DELETE")] {
                let name = format!("_fathomdb_read_visibility_{short}_{suffix}");
                let sql = master_triggers
                    .get(name.as_str())
                    .ok_or_else(|| format!("missing frozen-read visibility trigger {name}"))?;
                let expected = fathomdb_schema::expected_visibility_trigger_sql(
                    entry,
                    suffix,
                    operation,
                    schema_version,
                );
                if !eq_collapsed_ws(sql, &expected) {
                    return Err(format!(
                        "frozen-read visibility trigger {name} differs from manifest"
                    ));
                }
            }
        }
        expected_trigger_count = trigger_table_count * 3;
    }
    let count = master_triggers.keys().filter(|name| matches_visibility_trigger_like(name)).count();
    if count != expected_trigger_count {
        return Err("unexpected frozen-read visibility trigger".to_string());
    }
    if schema_version >= 35
        && !crate::vector_spans::cleanup_trigger_sql_is_valid(
            master_triggers.get("_fathomdb_vector_spans_mapping_delete").map(String::as_str),
        )
    {
        return Err("missing or altered vector span cleanup trigger".to_string());
    }
    if schema_version >= 39 {
        validate_page_index(
            &master_indexes,
            "canonical_nodes_scope_kind_cursor_page_idx",
            "canonical_nodes",
            "CREATE UNIQUE INDEX canonical_nodes_scope_kind_cursor_page_idx ON canonical_nodes(scope, kind, write_cursor) WHERE logical_id IS NOT NULL",
        )?;
        validate_page_index(
            &master_indexes,
            "operational_state_scope_collection_cursor_page_idx",
            "operational_state",
            "CREATE UNIQUE INDEX operational_state_scope_collection_cursor_page_idx ON operational_state(scope, collection_name, write_cursor)",
        )?;
        validate_page_index(
            &master_indexes,
            "operational_state_write_cursor_idx",
            "operational_state",
            "CREATE INDEX operational_state_write_cursor_idx ON operational_state(write_cursor)",
        )?;
        validate_page_index(
            &master_indexes,
            "_fathomdb_artifact_revisions_write_cursor_idx",
            "_fathomdb_artifact_revisions",
            "CREATE INDEX _fathomdb_artifact_revisions_write_cursor_idx ON _fathomdb_artifact_revisions(write_cursor)",
        )?;
    } else if schema_version >= 33 {
        validate_page_index(
            &master_indexes,
            "canonical_nodes_kind_cursor_page_idx",
            "canonical_nodes",
            "CREATE UNIQUE INDEX canonical_nodes_kind_cursor_page_idx ON canonical_nodes(kind, write_cursor) WHERE logical_id IS NOT NULL",
        )?;
        validate_page_index(
            &master_indexes,
            "operational_state_collection_cursor_page_idx",
            "operational_state",
            "CREATE UNIQUE INDEX operational_state_collection_cursor_page_idx ON operational_state(collection_name, write_cursor)",
        )?;
        validate_page_index(
            &master_indexes,
            "operational_state_write_cursor_idx",
            "operational_state",
            "CREATE INDEX operational_state_write_cursor_idx ON operational_state(write_cursor)",
        )?;
        validate_page_index(
            &master_indexes,
            "_fathomdb_artifact_revisions_write_cursor_idx",
            "_fathomdb_artifact_revisions",
            "CREATE INDEX _fathomdb_artifact_revisions_write_cursor_idx ON _fathomdb_artifact_revisions(write_cursor)",
        )?;
    }
    Ok(())
}

fn validate_page_index(
    master_indexes: &std::collections::HashMap<(String, String), String>,
    name: &str,
    table: &str,
    expected: &str,
) -> Result<(), String> {
    let sql = master_indexes
        .get(&(name.to_string(), table.to_string()))
        .ok_or_else(|| format!("missing pagination index {name}"))?;
    if !eq_collapsed_ws(sql, expected) {
        return Err(format!("pagination index {name} differs from manifest"));
    }
    Ok(())
}

pub(crate) fn eq_collapsed_ws(a: &str, b: &str) -> bool {
    a.split_whitespace().eq(b.split_whitespace())
}

fn normalize_sql(sql: &str) -> String {
    sql.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn load_global_struct_state(connection: &Connection) -> Result<(u64, String), EngineError> {
    let (gen_i64, nonce): (i64, String) = connection
        .prepare_cached(
            "SELECT global_struct_gen, global_struct_nonce \
             FROM _fathomdb_read_visibility_state WHERE singleton=1",
        )
        .and_then(|mut stmt| stmt.query_row([], |row| Ok((row.get(0)?, row.get(1)?))))
        .map_err(|_| EngineError::Storage)?;
    let gen_u64 = u64::try_from(gen_i64).map_err(|_| EngineError::Storage)?;
    Ok((gen_u64, nonce))
}

fn sorted_read_scopes(scope: &crate::scope::ScopePredicate) -> Vec<&str> {
    let mut scopes: Vec<&str> = scope.read_scopes().iter().map(String::as_str).collect();
    scopes.sort_unstable();
    scopes.dedup();
    scopes
}

fn node_partition_kind(kind: Option<&str>) -> String {
    match kind {
        Some(k) => format!("node:{k}"),
        None => "node_all".to_string(),
    }
}

fn vector_ready_partition_kind(kind: Option<&str>) -> String {
    match kind {
        Some(k) => format!("vector_ready:{k}"),
        None => "vector_ready_all".to_string(),
    }
}

fn edge_kind_partition_kind(kind: Option<&str>) -> String {
    match kind {
        Some(k) => format!("edge:{k}"),
        None => "edge_any".to_string(),
    }
}

fn query_watermark(
    connection: &Connection,
    scope: &str,
    partition_kind: &str,
) -> Result<(u64, u64, u64, String), EngineError> {
    let row: Option<(i64, i64, i64, String)> = connection
        .prepare_cached(
            "SELECT add_generation, max_cursor, destructive_generation, state_nonce \
             FROM _fathomdb_scope_visibility_watermarks \
             WHERE scope = ?1 AND partition_kind = ?2",
        )
        .and_then(|mut stmt| {
            stmt.query_row(rusqlite::params![scope, partition_kind], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })
            .optional()
        })
        .map_err(|_| EngineError::Storage)?;
    match row {
        Some((add_g, max_c, dest_g, nonce)) => Ok((
            u64::try_from(add_g).map_err(|_| EngineError::Storage)?,
            u64::try_from(max_c).map_err(|_| EngineError::Storage)?,
            u64::try_from(dest_g).map_err(|_| EngineError::Storage)?,
            nonce,
        )),
        None => Ok((0, 0, 0, String::new())),
    }
}

struct ScopeWatermarkRow {
    scope: String,
    partition_kind: String,
    add_generation: u64,
    max_cursor: u64,
    destructive_generation: u64,
    state_nonce: String,
}

fn load_partition_rows(
    connection: &Connection,
    read_scopes: &[&str],
    partition_kind: &str,
) -> Result<Vec<ScopeWatermarkRow>, EngineError> {
    let mut out = Vec::with_capacity(read_scopes.len());
    for &s in read_scopes {
        let (add_generation, max_cursor, destructive_generation, state_nonce) =
            query_watermark(connection, s, partition_kind)?;
        out.push(ScopeWatermarkRow {
            scope: s.to_string(),
            partition_kind: partition_kind.to_string(),
            add_generation,
            max_cursor,
            destructive_generation,
            state_nonce,
        });
    }
    Ok(out)
}

fn fold_owned_partition_rows(domain_label: &str, rows: &[ScopeWatermarkRow]) -> u64 {
    let mut bytes = Vec::from(WATERMARK_TAG_DOMAIN_V1);
    encode_string(&mut bytes, domain_label);
    for row in rows {
        encode_string(&mut bytes, &row.scope);
        encode_string(&mut bytes, &row.partition_kind);
        encode_u64(&mut bytes, row.add_generation);
        encode_u64(&mut bytes, row.max_cursor);
        encode_u64(&mut bytes, row.destructive_generation);
        encode_string(&mut bytes, &row.state_nonce);
    }
    let hash: [u8; 32] = Sha256::digest(bytes).into();
    u64::from_be_bytes(hash[0..8].try_into().expect("sha256 is 32 bytes"))
}

fn compute_partition_tag(
    connection: &Connection,
    read_scopes: &[&str],
    partition_kind: &str,
    domain_label: &str,
) -> Result<u64, EngineError> {
    let rows = load_partition_rows(connection, read_scopes, partition_kind)?;
    Ok(fold_owned_partition_rows(domain_label, &rows))
}

fn compute_node_partition(
    connection: &Connection,
    read_scopes: &[&str],
    kind: Option<&str>,
) -> Result<(u64, u64), EngineError> {
    let pk = node_partition_kind(kind);
    let rows = load_partition_rows(connection, read_scopes, &pk)?;
    let max_cursor = rows.iter().map(|r| r.max_cursor).max().unwrap_or(0);
    let tag = fold_owned_partition_rows("node", &rows);
    Ok((max_cursor, tag))
}

fn serving_encoding_v4(
    connection: &Connection,
    read_scopes: &[&str],
) -> Result<Vec<u8>, EngineError> {
    let mut bytes = Vec::from(SERVING_DOMAIN_V4);
    let (gs_gen, gs_nonce) = load_global_struct_state(connection)?;
    encode_u64(&mut bytes, gs_gen);
    encode_string(&mut bytes, &gs_nonce);
    let generation = crate::projection_generation::current_generation(connection)?;
    encode_string(&mut bytes, generation.id.as_str());
    encode_string(&mut bytes, &generation.digest);
    encode_u64(&mut bytes, generation.boundary);
    encode_string(&mut bytes, "serving");
    encode_string(&mut bytes, generation.origin.as_str());
    for &s in read_scopes {
        let (add_g, max_c, dest_g, nonce) = query_watermark(connection, s, "scope_destructive")?;
        encode_string(&mut bytes, s);
        encode_string(&mut bytes, "scope_destructive");
        encode_u64(&mut bytes, add_g);
        encode_u64(&mut bytes, max_c);
        encode_u64(&mut bytes, dest_g);
        encode_string(&mut bytes, &nonce);
    }
    Ok(bytes)
}

fn serving_digest_v4(
    connection: &Connection,
    read_scopes: &[&str],
) -> Result<[u8; 32], EngineError> {
    Ok(Sha256::digest(serving_encoding_v4(connection, read_scopes)?).into())
}

fn binding_for_snapshot(
    connection: &Connection,
    context: &ReadContextV1,
    effective_valid_at: i64,
    context_digest: [u8; 32],
    scope: &crate::scope::ScopePredicate,
) -> Result<FrozenReadBinding, EngineError> {
    let schema_version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| EngineError::Storage)?;
    #[cfg(any(test, feature = "migration-test-hooks"))]
    {
        // Retained normative token fixtures predate coalescing. This private
        // fixture route is absent from ordinary builds and public admission.
        if !(31..37).contains(&schema_version) {
            require_clean_visibility(connection)?;
        }
    }
    #[cfg(not(any(test, feature = "migration-test-hooks")))]
    require_clean_visibility(connection)?;

    if schema_version < 39 {
        return Ok(FrozenReadBinding {
            database_id: read_open_state(connection, DATABASE_ID_KEY)?,
            effective_valid_at,
            canonical_write_boundary: load_next_cursor(connection),
            read_visibility_generation: load_visibility_generation(connection)?,
            dependency_generation: load_dependency_generation(connection)?,
            projection_cursor: load_projection_cursor(connection)
                .map_err(|_| EngineError::Storage)?,
            registry_digest: projection_registry_digest(connection)?,
            serving_digest: projection_serving_digest(connection)?,
            context_digest,
            scope_digest: scope.frozen_digest(),
            surface_tags: None,
        });
    }

    let read_scopes = sorted_read_scopes(scope);
    let kind = context.eligibility.kind.as_deref();
    let node_pk = node_partition_kind(kind);
    let node_rows = load_partition_rows(connection, &read_scopes, &node_pk)?;
    let canonical_write_boundary = node_rows.iter().map(|r| r.max_cursor).max().unwrap_or(0);
    let read_visibility_generation = fold_owned_partition_rows("node", &node_rows);
    let node_all_tag = if kind.is_none() {
        fold_owned_partition_rows("node_all", &node_rows)
    } else {
        compute_partition_tag(connection, &read_scopes, "node_all", "node_all")?
    };

    let dep_tag =
        compute_partition_tag(connection, &read_scopes, "scope_dependency", "scope_dependency")?;
    let edge_any_rows = load_partition_rows(connection, &read_scopes, "edge_any")?;
    let edge_any_tag = fold_owned_partition_rows("edge_any", &edge_any_rows);
    let edge_kind_tag = if kind.is_none() {
        fold_owned_partition_rows("edge_kind", &edge_any_rows)
    } else {
        let edge_pk = edge_kind_partition_kind(kind);
        compute_partition_tag(connection, &read_scopes, &edge_pk, "edge_kind")?
    };
    let op_state_tag = compute_partition_tag(connection, &read_scopes, "op_state", "op_state")?;
    let vec_pk = vector_ready_partition_kind(kind);
    let projection_readiness_tag =
        compute_partition_tag(connection, &read_scopes, &vec_pk, "vector_ready")?;

    Ok(FrozenReadBinding {
        database_id: read_open_state(connection, DATABASE_ID_KEY)?,
        effective_valid_at,
        canonical_write_boundary,
        read_visibility_generation,
        dependency_generation: dep_tag,
        projection_cursor: edge_any_tag,
        registry_digest: projection_registry_digest(connection)?,
        serving_digest: serving_digest_v4(connection, &read_scopes)?,
        context_digest,
        scope_digest: scope.frozen_digest(),
        surface_tags: Some(SurfaceTags {
            op_state_tag,
            node_all_tag,
            projection_readiness_tag,
            edge_kind_tag,
        }),
    })
}

fn encode_registry_base_columns(
    bytes: &mut Vec<u8>,
    name: &str,
    roles: &str,
    fts: Option<&str>,
    embedder: Option<&str>,
    vector: i64,
    source: Option<&str>,
) {
    encode_string(bytes, name);
    encode_string(bytes, roles);
    encode_optional_string(bytes, fts);
    encode_optional_string(bytes, embedder);
    encode_i64(bytes, vector);
    encode_optional_string(bytes, source);
}

fn projection_registry_digest(connection: &Connection) -> Result<[u8; 32], EngineError> {
    let schema_version: u32 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|_| EngineError::Storage)?;
    let mut bytes = Vec::from(REGISTRY_DOMAIN);
    if schema_version >= 38 {
        let mut statement = connection
            .prepare(
                "SELECT name,roles,fts_tokenizer,vector_embedder,vector_declared,source,entity_class,vector_kinds \
                 FROM _fathomdb_projection_registry ORDER BY name, entity_class",
            )
            .map_err(|_| EngineError::Storage)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(|_| EngineError::Storage)?;
        for row in rows {
            let (name, roles, fts, embedder, vector, source, entity_class, vector_kinds) =
                row.map_err(|_| EngineError::Storage)?;
            encode_registry_base_columns(
                &mut bytes,
                &name,
                &roles,
                fts.as_deref(),
                embedder.as_deref(),
                vector,
                source.as_deref(),
            );
            encode_string(&mut bytes, &entity_class);
            encode_optional_string(&mut bytes, vector_kinds.as_deref());
        }
    } else {
        let mut statement = connection
            .prepare(
                "SELECT name,roles,fts_tokenizer,vector_embedder,vector_declared,source \
                 FROM _fathomdb_projection_registry ORDER BY name",
            )
            .map_err(|_| EngineError::Storage)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })
            .map_err(|_| EngineError::Storage)?;
        for row in rows {
            let (name, roles, fts, embedder, vector, source) =
                row.map_err(|_| EngineError::Storage)?;
            encode_registry_base_columns(
                &mut bytes,
                &name,
                &roles,
                fts.as_deref(),
                embedder.as_deref(),
                vector,
                source.as_deref(),
            );
        }
    }
    Ok(Sha256::digest(bytes).into())
}

fn projection_serving_digest(connection: &Connection) -> Result<[u8; 32], EngineError> {
    Ok(Sha256::digest(projection_serving_encoding(connection)?).into())
}

fn projection_serving_encoding(connection: &Connection) -> Result<Vec<u8>, EngineError> {
    let schema_version: u32 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|_| EngineError::Storage)?;
    if schema_version >= 39 {
        return serving_encoding_v4(connection, &[""]);
    }
    let mut bytes = if schema_version >= 33 {
        Vec::from(SERVING_DOMAIN_V3)
    } else if schema_version >= 32 {
        Vec::from(SERVING_DOMAIN_V2)
    } else {
        Vec::from(SERVING_DOMAIN_V1)
    };
    if schema_version >= 33 {
        encode_string(&mut bytes, &load_visibility_nonce(connection)?);
    }
    if schema_version >= 32 {
        let generation = crate::projection_generation::current_generation(connection)?;
        encode_string(&mut bytes, generation.id.as_str());
        encode_string(&mut bytes, &generation.digest);
        encode_u64(&mut bytes, generation.boundary);
        encode_string(&mut bytes, "serving");
        encode_string(&mut bytes, generation.origin.as_str());
    }
    let mut states = connection
        .prepare("SELECT kind,last_enqueued_cursor,updated_at FROM _fathomdb_projection_state ORDER BY kind")
        .map_err(|_| EngineError::Storage)?;
    let rows = states
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))
        })
        .map_err(|_| EngineError::Storage)?;
    for row in rows {
        let (kind, cursor, updated_at) = row.map_err(|_| EngineError::Storage)?;
        encode_string(&mut bytes, &kind);
        encode_i64(&mut bytes, cursor);
        encode_i64(&mut bytes, updated_at);
    }
    // Schema 33 visibility triggers atomically advance the outer monotonic
    // generation and rotate the branch-sensitive nonce above for every terminal
    // insert/update/delete. Retaining the historical row-by-row digest would add
    // O(terminal rows) work without strengthening that drift signal. Schema
    // 31/32 retain their exact encoding for pinned historical fixtures.
    if schema_version < 33 {
        let mut terminals = connection
            .prepare(
                "SELECT write_cursor,state FROM _fathomdb_projection_terminal ORDER BY write_cursor",
            )
            .map_err(|_| EngineError::Storage)?;
        let rows = terminals
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))
            .map_err(|_| EngineError::Storage)?;
        for row in rows {
            let (cursor, state) = row.map_err(|_| EngineError::Storage)?;
            encode_i64(&mut bytes, cursor);
            encode_string(&mut bytes, &state);
        }
    }
    Ok(bytes)
}

fn encode_context(context: &ReadContextV1) -> Vec<u8> {
    let mut bytes = Vec::new();
    encode_u32(&mut bytes, context.schema_version);
    bytes.push(u8::from(context.view.include_superseded));
    bytes.push(u8::from(context.view.include_inactive));
    bytes.push(u8::from(context.view.include_out_of_window));
    encode_optional_i64(&mut bytes, context.view.valid_as_of);
    encode_optional_string(&mut bytes, context.eligibility.source_type.as_deref());
    encode_optional_string(&mut bytes, context.eligibility.kind.as_deref());
    encode_optional_i64(&mut bytes, context.eligibility.created_after);
    encode_optional_string(&mut bytes, context.eligibility.status.as_deref());
    let mut attributes = context.eligibility.attributes.clone();
    attributes.sort();
    encode_u32(&mut bytes, u32::try_from(attributes.len()).unwrap_or(u32::MAX));
    for (name, value) in attributes {
        encode_string(&mut bytes, &name);
        encode_string(&mut bytes, &value);
    }
    if !context.eligibility.property_predicates.is_empty() {
        bytes.push(0x50);
        bytes.push(0x01);
        let mut encoded_preds: Vec<(&str, u8, u8, Vec<u8>)> = context
            .eligibility
            .property_predicates
            .iter()
            .map(|pred| {
                let op_byte = match pred.op {
                    crate::properties::PropertyComparisonOp::Eq => 1,
                    crate::properties::PropertyComparisonOp::Ne => 2,
                    crate::properties::PropertyComparisonOp::Lt => 3,
                    crate::properties::PropertyComparisonOp::Lte => 4,
                    crate::properties::PropertyComparisonOp::Gt => 5,
                    crate::properties::PropertyComparisonOp::Gte => 6,
                };
                let (val_tag, val_bytes) = match &pred.value {
                    crate::properties::PropertyValue::String(s) => (1_u8, s.as_bytes().to_vec()),
                    crate::properties::PropertyValue::Int(i) => (2_u8, i.to_le_bytes().to_vec()),
                    crate::properties::PropertyValue::Float(f) => {
                        let norm = if *f == 0.0 { 0.0_f64 } else { *f };
                        (3_u8, norm.to_bits().to_le_bytes().to_vec())
                    }
                    crate::properties::PropertyValue::Bool(b) => (4_u8, vec![u8::from(*b)]),
                };
                (pred.key.as_str(), op_byte, val_tag, val_bytes)
            })
            .collect();
        encoded_preds.sort();
        encode_u32(&mut bytes, u32::try_from(encoded_preds.len()).unwrap_or(u32::MAX));
        for (key, op_byte, val_tag, val_bytes) in encoded_preds {
            encode_string(&mut bytes, key);
            bytes.push(op_byte);
            bytes.push(val_tag);
            if val_tag == 1 {
                encode_u32(&mut bytes, u32::try_from(val_bytes.len()).unwrap_or(u32::MAX));
            }
            bytes.extend_from_slice(&val_bytes);
        }
    }
    bytes
}

fn encode_binding(binding: &FrozenReadBinding) -> Vec<u8> {
    let mut bytes = Vec::new();
    encode_u32(&mut bytes, FROZEN_READ_SCHEMA_VERSION);
    encode_string(&mut bytes, &binding.database_id);
    encode_i64(&mut bytes, binding.effective_valid_at);
    encode_u64(&mut bytes, binding.canonical_write_boundary);
    encode_u64(&mut bytes, binding.read_visibility_generation);
    encode_u64(&mut bytes, binding.dependency_generation);
    encode_u64(&mut bytes, binding.projection_cursor);
    bytes.extend_from_slice(&binding.registry_digest);
    bytes.extend_from_slice(&binding.serving_digest);
    bytes.extend_from_slice(&binding.context_digest);
    if let Some(scope_digest) = binding.scope_digest {
        bytes.extend_from_slice(&scope_digest);
    }
    if let Some(tags) = binding.surface_tags {
        bytes.extend_from_slice(&SURFACE_TAGS_MAGIC);
        encode_u64(&mut bytes, tags.op_state_tag);
        encode_u64(&mut bytes, tags.node_all_tag);
        encode_u64(&mut bytes, tags.projection_readiness_tag);
        encode_u64(&mut bytes, tags.edge_kind_tag);
    }
    bytes
}

fn decode_binding(bytes: &[u8]) -> Result<FrozenReadBinding, EngineError> {
    let mut cursor = Cursor { bytes, offset: 0 };
    let schema_version = cursor.u32()?;
    if schema_version != FROZEN_READ_SCHEMA_VERSION {
        return Err(FrozenReadError::new(
            FrozenReadErrorReason::UnsupportedSchemaVersion,
            "/token",
        )
        .into());
    }
    let database_id = cursor.string()?;
    let effective_valid_at = cursor.i64()?;
    let canonical_write_boundary = cursor.u64()?;
    let read_visibility_generation = cursor.u64()?;
    let dependency_generation = cursor.u64()?;
    let projection_cursor = cursor.u64()?;
    let registry_digest = cursor.array32()?;
    let serving_digest = cursor.array32()?;
    let context_digest = cursor.array32()?;
    let rem = bytes.len().saturating_sub(cursor.offset);
    let (scope_digest, surface_tags) = match rem {
        0 => (None, None),
        32 => (Some(cursor.array32()?), None),
        34 => {
            if cursor.take(2)? != SURFACE_TAGS_MAGIC {
                return Err(token_malformed());
            }
            let tags = SurfaceTags {
                op_state_tag: cursor.u64()?,
                node_all_tag: cursor.u64()?,
                projection_readiness_tag: cursor.u64()?,
                edge_kind_tag: cursor.u64()?,
            };
            (None, Some(tags))
        }
        66 => {
            let sd = cursor.array32()?;
            if cursor.take(2)? != SURFACE_TAGS_MAGIC {
                return Err(token_malformed());
            }
            let tags = SurfaceTags {
                op_state_tag: cursor.u64()?,
                node_all_tag: cursor.u64()?,
                projection_readiness_tag: cursor.u64()?,
                edge_kind_tag: cursor.u64()?,
            };
            (Some(sd), Some(tags))
        }
        _ => return Err(token_malformed()),
    };
    let binding = FrozenReadBinding {
        database_id,
        effective_valid_at,
        canonical_write_boundary,
        read_visibility_generation,
        dependency_generation,
        projection_cursor,
        registry_digest,
        serving_digest,
        context_digest,
        scope_digest,
        surface_tags,
    };
    if cursor.offset != bytes.len() || encode_binding(&binding) != bytes {
        return Err(FrozenReadError::new(FrozenReadErrorReason::TokenMalformed, "/token").into());
    }
    Ok(binding)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl Cursor<'_> {
    fn take(&mut self, count: usize) -> Result<&[u8], EngineError> {
        let end = self.offset.checked_add(count).ok_or_else(token_malformed)?;
        let value = self.bytes.get(self.offset..end).ok_or_else(token_malformed)?;
        self.offset = end;
        Ok(value)
    }

    fn u32(&mut self) -> Result<u32, EngineError> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().map_err(|_| token_malformed())?))
    }

    fn u64(&mut self) -> Result<u64, EngineError> {
        Ok(u64::from_be_bytes(self.take(8)?.try_into().map_err(|_| token_malformed())?))
    }

    fn i64(&mut self) -> Result<i64, EngineError> {
        Ok(i64::from_be_bytes(self.take(8)?.try_into().map_err(|_| token_malformed())?))
    }

    fn string(&mut self) -> Result<String, EngineError> {
        let length = usize::try_from(self.u32()?).map_err(|_| token_malformed())?;
        String::from_utf8(self.take(length)?.to_vec()).map_err(|_| token_malformed())
    }

    fn array32(&mut self) -> Result<[u8; 32], EngineError> {
        self.take(32)?.try_into().map_err(|_| token_malformed())
    }
}

fn token_malformed() -> EngineError {
    FrozenReadError::new(FrozenReadErrorReason::TokenMalformed, "/token").into()
}

pub(crate) fn read_open_state(connection: &Connection, key: &str) -> Result<String, EngineError> {
    connection
        .query_row("SELECT value FROM _fathomdb_open_state WHERE key=?1", [key], |row| row.get(0))
        .optional()
        .map_err(|_| EngineError::Storage)?
        .ok_or(EngineError::Storage)
}

pub(crate) fn read_hex_open_state(
    connection: &Connection,
    key: &str,
    expected_bytes: usize,
) -> Result<Vec<u8>, EngineError> {
    let value = read_open_state(connection, key)?;
    if !is_lower_hex(&value, expected_bytes * 2) {
        return Err(EngineError::Storage);
    }
    hex_decode(&value).ok_or(EngineError::Storage)
}

fn is_lower_hex(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(crate) fn digest(domain: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(payload);
    hasher.finalize().into()
}

pub(crate) fn hmac_sha256(key: &[u8], domain: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut block = [0_u8; 64];
    block[..key.len()].copy_from_slice(key);
    let mut inner_pad = [0x36_u8; 64];
    let mut outer_pad = [0x5c_u8; 64];
    for index in 0..64 {
        inner_pad[index] ^= block[index];
        outer_pad[index] ^= block[index];
    }
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(domain);
    inner.update(payload);
    let inner_digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_digest);
    outer.finalize().into()
}

pub(crate) fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter().zip(right).fold(0_u8, |diff, (a, b)| diff | (a ^ b)) == 0
}

pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        value.push_str(&format!("{byte:02x}"));
    }
    value
}

pub(crate) fn hex_decode(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) || !is_lower_hex(value, value.len()) {
        return None;
    }
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}

pub(crate) fn encode_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

pub(crate) fn encode_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

pub(crate) fn encode_i64(bytes: &mut Vec<u8>, value: i64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

pub(crate) fn encode_string(bytes: &mut Vec<u8>, value: &str) {
    encode_u32(bytes, u32::try_from(value.len()).unwrap_or(u32::MAX));
    bytes.extend_from_slice(value.as_bytes());
}

fn encode_optional_string(bytes: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            bytes.push(1);
            encode_string(bytes, value);
        }
        None => bytes.push(0),
    }
}

fn encode_optional_i64(bytes: &mut Vec<u8>, value: Option<i64>) {
    match value {
        Some(value) => {
            bytes.push(1);
            encode_i64(bytes, value);
        }
        None => bytes.push(0),
    }
}

pub(crate) fn page_cursor_material(
    connection: &Connection,
) -> Result<(String, Vec<u8>), EngineError> {
    Ok((
        read_open_state(connection, DATABASE_ID_KEY)?,
        read_hex_open_state(connection, READ_CONTEXT_KEY, 32)?,
    ))
}

pub(crate) fn page_context_digest(frozen: &FrozenReadContextV1) -> Result<[u8; 32], EngineError> {
    let context = validate_context(&frozen.context)?;
    let mut bytes = Vec::new();
    encode_u32(&mut bytes, frozen.schema_version);
    encode_i64(&mut bytes, frozen.effective_valid_at);
    encode_u32(&mut bytes, u32::try_from(context.len()).unwrap_or(u32::MAX));
    bytes.extend_from_slice(&context);
    encode_string(&mut bytes, &frozen.token);
    Ok(digest(b"fathomdb.page-context.v1\0", &bytes))
}

/// Test-only Slice 45 attribution for authenticated frozen-page setup.
#[cfg(feature = "test-hooks")]
#[derive(Clone, Copy, Debug)]
#[doc(hidden)]
pub struct Slice45FrozenStageTiming {
    pub cursor_authentication_ns: u128,
    pub token_authentication_ns: u128,
    pub snapshot_binding_ns: u128,
}

/// Test-only Slice 45 attribution for frozen-context minting.
#[cfg(feature = "test-hooks")]
#[derive(Clone, Copy, Debug)]
#[doc(hidden)]
pub struct Slice45MintStageTiming {
    pub context_validation_ns: u128,
    pub snapshot_validation_ns: u128,
    pub binding_ns: u128,
    pub token_codec_ns: u128,
}

impl Engine {
    /// Test-only stage attribution for the Slice 45 performance receipt.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn measure_slice45_frozen_stages_for_test(
        &self,
        context: &FrozenReadContextV1,
        page: &PageRequestV1,
    ) -> Result<Slice45FrozenStageTiming, EngineError> {
        let _operation = self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let scope = self.scope_predicate();
        let started = Instant::now();
        pagination::authenticate_cursor(connection, page)?;
        let cursor_authentication_ns = started.elapsed().as_nanos();
        let started = Instant::now();
        let binding = authenticate(connection, context, &scope)?;
        let token_authentication_ns = started.elapsed().as_nanos();
        let started = Instant::now();
        validate_snapshot(connection, &context.context, &binding, &scope, ReadSurface::Preflight)?;
        let snapshot_binding_ns = started.elapsed().as_nanos();
        Ok(Slice45FrozenStageTiming {
            cursor_authentication_ns,
            token_authentication_ns,
            snapshot_binding_ns,
        })
    }

    /// Test-only stage attribution for one Slice 45 frozen-context mint.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn measure_slice45_mint_stages_for_test(
        &self,
        context: &ReadContextV1,
    ) -> Result<Slice45MintStageTiming, EngineError> {
        let _operation = self.ensure_open()?;
        let scope = self.scope_predicate();
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        let (_, generation, timing) = mint_measured(connection, context, &scope)?;
        self.read_visibility_generation.fetch_max(generation, Ordering::AcqRel);
        Ok(Slice45MintStageTiming {
            context_validation_ns: timing.context_validation_ns,
            snapshot_validation_ns: timing.snapshot_validation_ns,
            binding_ns: timing.binding_ns,
            token_codec_ns: timing.token_codec_ns,
        })
    }
}
#[cfg(test)]
mod tests {
    use fathomdb_schema::{migrate, migrate_with_steps, MIGRATIONS};
    use proptest::prelude::*;
    use rusqlite::Connection;
    use serde_json::Value;

    use super::{
        authenticate, decode_binding, digest, encode_binding, encode_context, hex_decode,
        hex_encode, load_visibility_generation, mint, projection_registry_digest,
        projection_serving_digest, projection_serving_encoding, validate_snapshot,
        FrozenReadBinding, FrozenReadErrorReason, ReadContextV1, ReadSurface, ReadView,
        SearchFilter, SurfaceTags, CONTEXT_DOMAIN,
    };

    proptest! {
        #[test]
        fn binding_codec_round_trips_canonically(
            database_id in "[0-9a-f]{32}",
            effective_valid_at in any::<i64>(),
            canonical_write_boundary in any::<u64>(),
            read_visibility_generation in any::<u64>(),
            dependency_generation in any::<u64>(),
            projection_cursor in any::<u64>(),
            registry_digest in any::<[u8; 32]>(),
            serving_digest in any::<[u8; 32]>(),
            context_digest in any::<[u8; 32]>(),
            scope_digest in any::<Option<[u8; 32]>>(),
            surface_tags in prop::option::of((any::<u64>(), any::<u64>(), any::<u64>(), any::<u64>()))
                .prop_map(|opt| opt.map(|(op_state_tag, node_all_tag, projection_readiness_tag, edge_kind_tag)| SurfaceTags {
                    op_state_tag,
                    node_all_tag,
                    projection_readiness_tag,
                    edge_kind_tag,
                })),
        ) {
            let expected = FrozenReadBinding {
                database_id,
                effective_valid_at,
                canonical_write_boundary,
                read_visibility_generation,
                dependency_generation,
                projection_cursor,
                registry_digest,
                serving_digest,
                context_digest,
                scope_digest,
                surface_tags,
            };
            let encoded = encode_binding(&expected);
            let decoded = decode_binding(&encoded).unwrap();
            prop_assert_eq!(&decoded, &expected);
            prop_assert_eq!(encode_binding(&decoded), encoded);
            // Corrupting the 0x53 0x39 magic bytes must fail closed with TokenMalformed.
            if surface_tags.is_some() {
                let mut corrupted = encode_binding(&expected);
                let magic_pos = corrupted.len() - 34;
                corrupted[magic_pos] ^= 0xFF;
                prop_assert!(matches!(
                    decode_binding(&corrupted),
                    Err(crate::EngineError::FrozenRead(fe))
                        if fe.reason == FrozenReadErrorReason::TokenMalformed
                ));
            }
        }

        #[test]
        fn lowercase_hex_round_trips(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
            let encoded = hex_encode(&bytes);
            prop_assert_eq!(hex_decode(&encoded), Some(bytes));
        }
    }

    #[test]
    fn context_encoding_is_independent_of_attribute_order() {
        let left_filter = SearchFilter {
            attributes: vec![
                ("owner".to_string(), "alice".to_string()),
                ("scope".to_string(), "personal".to_string()),
            ],
            ..SearchFilter::default()
        };
        let mut right_filter = left_filter.clone();
        right_filter.attributes.reverse();
        let left = ReadContextV1::new(ReadView::default(), left_filter).unwrap();
        let right = ReadContextV1::new(ReadView::default(), right_filter).unwrap();
        assert_eq!(encode_context(&left), encode_context(&right));
    }

    fn assert_normative_fixture(mut connection: Connection, fixture: &Value) {
        let context = ReadContextV1::new(
            ReadView { valid_as_of: Some(1_700_000_000), ..ReadView::default() },
            SearchFilter {
                source_type: Some("node_body".to_string()),
                kind: Some("doc".to_string()),
                ..SearchFilter::default()
            },
        )
        .unwrap();
        let context_bytes = encode_context(&context);
        assert_eq!(hex_encode(&context_bytes), fixture["context_encoding_hex"].as_str().unwrap());
        assert_eq!(
            hex_encode(&digest(CONTEXT_DOMAIN, &context_bytes)),
            fixture["context_digest"].as_str().unwrap()
        );

        connection
            .execute(
                "UPDATE _fathomdb_open_state SET value=?1 \
                 WHERE key='_fathomdb_database_id'",
                [fixture["database_id"].as_str().unwrap()],
            )
            .unwrap();
        connection
            .execute(
                "UPDATE _fathomdb_open_state SET value=?1 \
                 WHERE key='_fathomdb_read_context_key'",
                [fixture["read_context_key"].as_str().unwrap()],
            )
            .unwrap();
        connection
            .execute(
                "UPDATE _fathomdb_read_visibility_state SET generation=0 WHERE singleton=1",
                [],
            )
            .unwrap();
        assert_eq!(
            hex_encode(&projection_registry_digest(&connection).unwrap()),
            fixture["projection_registry_digest"].as_str().unwrap()
        );
        assert_eq!(
            hex_encode(&projection_serving_digest(&connection).unwrap()),
            fixture["projection_serving_digest"].as_str().unwrap()
        );
        if let Some(expected) = fixture["projection_serving_encoding_hex"].as_str() {
            assert_eq!(hex_encode(&projection_serving_encoding(&connection).unwrap()), expected);
        }
        let (frozen, _) =
            mint(&mut connection, &context, &crate::scope::ScopePredicate::unscoped()).unwrap();
        let payload_hex = frozen.token.split('.').nth(1).unwrap();
        assert_eq!(payload_hex, fixture["payload_hex"].as_str().unwrap());
        assert_eq!(frozen.token, fixture["token"].as_str().unwrap());
    }

    #[test]
    fn normative_v1_fixture_remains_pinned_at_schema_31() {
        crate::configure_runtime_for_open().unwrap();
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../../tests/fixtures/slice35_frozen_context_v1.json"
        ))
        .unwrap();
        let connection = Connection::open_in_memory().unwrap();
        migrate_with_steps(&connection, &MIGRATIONS[..31]).unwrap();
        assert_normative_fixture(connection, &fixture);
    }

    #[test]
    fn normative_v2_fixture_pins_projection_generation_binding() {
        crate::configure_runtime_for_open().unwrap();
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../../tests/fixtures/slice40_frozen_context_v2.json"
        ))
        .unwrap();
        crate::register_sqlite_vec_extension();
        let mut connection = Connection::open_in_memory().unwrap();
        migrate_with_steps(&connection, &MIGRATIONS[..32]).unwrap();
        connection
            .execute(
                "INSERT INTO _fathomdb_embedder_profiles(\
                   profile,name,revision,dimension,mean_vec\
                 ) VALUES('default',?1,?2,?3,NULL)",
                rusqlite::params![
                    crate::open::DEFAULT_EMBEDDER_NAME,
                    crate::open::DEFAULT_EMBEDDER_REVISION,
                    crate::open::DEFAULT_EMBEDDER_DIMENSION
                ],
            )
            .unwrap();
        crate::ensure_vector_partition(&connection, crate::open::DEFAULT_EMBEDDER_DIMENSION)
            .unwrap();
        crate::projection_generation::bootstrap(&mut connection, 32, false).unwrap();
        let (old_id, declaration): (String, String) = connection
            .query_row(
                "SELECT generation_id,declaration_sha256 \
                 FROM _fathomdb_projection_generations WHERE role='serving'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(declaration, fixture["declaration_sha256"].as_str().unwrap());
        connection
            .execute(
                "UPDATE _fathomdb_projection_generations \
                 SET role='retired',retired_boundary=0 WHERE generation_id=?1",
                [old_id],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO _fathomdb_projection_generations(\
                   schema_version,generation_id,declaration_sha256,transition_boundary,role,origin\
                 ) VALUES(1,?1,?2,0,'serving','fresh')",
                rusqlite::params![fixture["generation_id"].as_str().unwrap(), declaration],
            )
            .unwrap();
        connection
            .execute(
                "UPDATE _fathomdb_projection_generation_current SET generation_id=?1 \
                 WHERE singleton=1",
                [fixture["generation_id"].as_str().unwrap()],
            )
            .unwrap();
        assert_normative_fixture(connection, &fixture);
    }

    #[test]
    fn schema33_serving_binding_is_compact_and_branch_sensitive() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("compact-serving.fathom.sqlite3");
        let opened = crate::Engine::open(&path).unwrap();
        opened.engine.close().unwrap();
        let connection = Connection::open(path).unwrap();
        let mut prior_encoding = projection_serving_encoding(&connection).unwrap();
        let before_generation = load_visibility_generation(&connection).unwrap();

        for cursor in 1..=100 {
            connection
                .execute(
                    "INSERT INTO _fathomdb_projection_terminal(write_cursor,state) \
                     VALUES(?1,'up_to_date')",
                    [cursor],
                )
                .unwrap();
            let next_encoding = projection_serving_encoding(&connection).unwrap();
            assert_ne!(next_encoding, prior_encoding);
            prior_encoding = next_encoding;
        }

        assert_eq!(
            load_visibility_generation(&connection).unwrap(),
            before_generation + 100,
            "terminal drift remains authenticated by the monotonic visibility generation"
        );
    }

    #[test]
    fn schema33_cutover_rejects_a_schema32_token_after_untracked_state_mutation() {
        crate::configure_runtime_for_open().unwrap();
        crate::register_sqlite_vec_extension();
        let mut connection = Connection::open_in_memory().unwrap();
        migrate_with_steps(&connection, &MIGRATIONS[..32]).unwrap();
        connection
            .execute(
                "INSERT INTO _fathomdb_embedder_profiles(\
                   profile,name,revision,dimension,mean_vec\
                 ) VALUES('default',?1,?2,?3,NULL)",
                rusqlite::params![
                    crate::open::DEFAULT_EMBEDDER_NAME,
                    crate::open::DEFAULT_EMBEDDER_REVISION,
                    crate::open::DEFAULT_EMBEDDER_DIMENSION
                ],
            )
            .unwrap();
        crate::ensure_vector_partition(&connection, crate::open::DEFAULT_EMBEDDER_DIMENSION)
            .unwrap();
        crate::projection_generation::bootstrap(&mut connection, 32, false).unwrap();
        connection
            .execute(
                "INSERT INTO operational_collections(\
                   name,kind,schema_json,retention_json,format_version,created_at\
                 ) VALUES('state','latest_state','{}','{}',1,0)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO operational_state(\
                   collection_name,record_key,payload_json,schema_id,write_cursor\
                 ) VALUES('state','key','{\"value\":1}',NULL,1)",
                [],
            )
            .unwrap();
        let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
        let unscoped = crate::scope::ScopePredicate::unscoped();
        let (frozen, _) = mint(&mut connection, &context, &unscoped).unwrap();

        connection
            .execute(
                "UPDATE operational_state SET payload_json='{\"value\":2}' \
                 WHERE collection_name='state' AND record_key='key'",
                [],
            )
            .unwrap();
        migrate(&connection).unwrap();

        let binding = authenticate(&connection, &frozen, &unscoped).unwrap();
        assert!(matches!(
            validate_snapshot(&connection, &frozen.context, &binding, &unscoped, ReadSurface::Preflight),
            Err(error) if error.reason == FrozenReadErrorReason::StateDrifted
        ));
    }

    #[test]
    fn equal_count_terminal_mutations_on_copied_databases_do_not_share_authority() {
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("original.fathom.sqlite3");
        let copied = directory.path().join("copied.fathom.sqlite3");
        let opened = crate::Engine::open(&original).unwrap();
        opened
            .engine
            .execute_for_test(
                "INSERT INTO _fathomdb_projection_terminal(write_cursor,state) \
                 VALUES(1,'up_to_date'),(2,'up_to_date')",
            )
            .unwrap();
        opened.engine.close().unwrap();
        std::fs::copy(&original, &copied).unwrap();

        let mut left = Connection::open(&original).unwrap();
        let right = Connection::open(&copied).unwrap();
        left.execute(
            "UPDATE _fathomdb_projection_terminal SET state='failed' WHERE write_cursor=1",
            [],
        )
        .unwrap();
        right
            .execute(
                "UPDATE _fathomdb_projection_terminal SET state='failed' WHERE write_cursor=2",
                [],
            )
            .unwrap();
        assert_eq!(
            load_visibility_generation(&left).unwrap(),
            load_visibility_generation(&right).unwrap()
        );
        let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
        let unscoped = crate::scope::ScopePredicate::unscoped();
        let (left_frozen, _) = mint(&mut left, &context, &unscoped).unwrap();
        let left_binding = authenticate(&left, &left_frozen, &unscoped).unwrap();
        validate_snapshot(
            &left,
            &left_frozen.context,
            &left_binding,
            &unscoped,
            ReadSurface::Preflight,
        )
        .unwrap();

        let copied_binding = authenticate(&right, &left_frozen, &unscoped).unwrap();
        assert!(matches!(
            validate_snapshot(&right, &left_frozen.context, &copied_binding, &unscoped, ReadSurface::Preflight),
            Err(error) if error.reason == FrozenReadErrorReason::StateDrifted
        ));
    }

    #[test]
    fn validate_on_open_scans_sqlite_master_in_single_pass_and_catches_tamper() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("validate-single-pass.fathom.sqlite3");
        let opened = crate::Engine::open(&path).unwrap();
        opened.engine.close().unwrap();

        let mut conn = Connection::open(&path).unwrap();
        let master_queries = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = std::sync::Arc::clone(&master_queries);
        conn.trace_v2(
            rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
            Some(move |event: rusqlite::trace::TraceEvent<'_>| {
                if let rusqlite::trace::TraceEvent::Stmt(_, sql) = event {
                    if sql.contains("sqlite_master") {
                        counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                }
            }),
        )
        .unwrap();
        super::validate_on_open(&conn, 39).unwrap();
        conn.trace_v2(
            rusqlite::trace::TraceEventCodes::empty(),
            None::<fn(rusqlite::trace::TraceEvent<'_>)>,
        )
        .unwrap();

        let count = master_queries.load(std::sync::atomic::Ordering::SeqCst);
        assert!(
            count <= 2,
            "validate_on_open must scan sqlite_master in a single pass (<=2 queries), got {count}"
        );

        // Tampering a trigger must still be caught cleanly.
        conn.execute_batch("DROP TRIGGER _fathomdb_read_visibility_cn_ai").unwrap();
        assert!(super::validate_on_open(&conn, 39).is_err());
    }
}
