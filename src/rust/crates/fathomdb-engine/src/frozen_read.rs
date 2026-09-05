use std::fmt::{Display, Formatter};
#[cfg(feature = "test-hooks")]
use std::time::Instant;

use rusqlite::{Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::{
    current_epoch_seconds, load_dependency_generation, load_next_cursor, load_projection_cursor,
    EngineError, ReadView, SearchFilter,
};

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
const DATABASE_ID_KEY: &str = "_fathomdb_database_id";
const READ_CONTEXT_KEY: &str = "_fathomdb_read_context_key";
const VISIBILITY_TRIGGER_TABLES: [(&str, &str); 18] = [
    ("cn", "canonical_nodes"),
    ("ce", "canonical_edges"),
    ("ar", "_fathomdb_artifact_revisions"),
    ("sv", "_fathomdb_source_versions"),
    ("sl", "_fathomdb_source_links"),
    ("sd", "_fathomdb_source_dependencies"),
    ("dc", "_fathomdb_dependency_closures"),
    ("pr", "_fathomdb_projection_registry"),
    ("ca", "canonical_attributes"),
    ("ps", "_fathomdb_projection_state"),
    ("pt", "_fathomdb_projection_terminal"),
    ("vk", "_fathomdb_vector_kinds"),
    ("vr", "_fathomdb_vector_rows"),
    ("ep", "_fathomdb_embedder_profiles"),
    ("pg", "_fathomdb_projection_generations"),
    ("pc", "_fathomdb_projection_generation_current"),
    ("oc", "operational_collections"),
    ("os", "operational_state"),
];

/// Versioned search eligibility and validity requested by a caller.
#[derive(Clone, Debug, Eq, PartialEq)]
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
#[derive(Clone, Debug, Eq, PartialEq)]
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
    let bytes = encode_context(context);
    if bytes.len() > CONTEXT_MAX_BYTES {
        return Err(FrozenReadError::new(FrozenReadErrorReason::ContextInvalid, "/eligibility"));
    }
    Ok(bytes)
}

pub(crate) fn mint(
    connection: &mut Connection,
    context: &ReadContextV1,
) -> Result<(FrozenReadContextV1, u64), EngineError> {
    mint_inner(connection, context).map(|(frozen, generation, _)| (frozen, generation))
}

#[cfg(feature = "test-hooks")]
pub(crate) fn mint_measured(
    connection: &mut Connection,
    context: &ReadContextV1,
) -> Result<(FrozenReadContextV1, u64, MintStageTiming), EngineError> {
    let (frozen, generation, timing) = mint_inner(connection, context)?;
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
    tx.query_row("SELECT COUNT(*) FROM canonical_nodes", [], |row| row.get::<_, i64>(0))
        .map_err(|_| EngineError::Storage)?;
    super::validate_filter_attributes_on_snapshot(&tx, &resolved.eligibility).map_err(|error| {
        match error {
            super::SearchReaderError::InvalidFilter(reason) => {
                EngineError::InvalidFilter { reason }
            }
            _ => EngineError::Storage,
        }
    })?;
    #[cfg(feature = "test-hooks")]
    let snapshot_validation_ns = snapshot_started.elapsed().as_nanos();
    #[cfg(not(feature = "test-hooks"))]
    let snapshot_validation_ns = 0;
    #[cfg(feature = "test-hooks")]
    let binding_started = Instant::now();
    let mut binding =
        binding_for_snapshot(&tx, effective_valid_at, digest(CONTEXT_DOMAIN, &context_bytes))?;
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
        binding.read_visibility_generation,
        [context_validation_ns, snapshot_validation_ns, binding_ns, token_codec_ns],
    ))
}

pub(crate) fn authenticate(
    connection: &Connection,
    frozen: &FrozenReadContextV1,
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
    expected: &FrozenReadBinding,
) -> Result<u64, FrozenReadError> {
    let actual =
        binding_for_snapshot(connection, expected.effective_valid_at, expected.context_digest)
            .map_err(|_| FrozenReadError::new(FrozenReadErrorReason::StateUnavailable, "/token"))?;
    if actual != *expected {
        return Err(FrozenReadError::new(FrozenReadErrorReason::StateDrifted, "/token"));
    }
    Ok(actual.read_visibility_generation)
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

pub(crate) fn validate_on_open(connection: &Connection, schema_version: u32) -> Result<(), String> {
    if schema_version < 31 {
        return Ok(());
    }
    let database_id =
        read_open_state(connection, DATABASE_ID_KEY).map_err(|error| error.to_string())?;
    if !is_lower_hex(&database_id, 32) {
        return Err("invalid frozen-read database identity".to_string());
    }
    read_hex_open_state(connection, READ_CONTEXT_KEY, 32).map_err(|error| error.to_string())?;
    load_visibility_generation(connection).map_err(|error| error.to_string())?;
    let trigger_tables = if schema_version >= 33 {
        &VISIBILITY_TRIGGER_TABLES[..18]
    } else if schema_version >= 32 {
        &VISIBILITY_TRIGGER_TABLES[..16]
    } else {
        &VISIBILITY_TRIGGER_TABLES[..14]
    };
    for &(short, table) in trigger_tables {
        for (suffix, operation) in [("ai", "INSERT"), ("au", "UPDATE"), ("ad", "DELETE")] {
            let name = format!("_fathomdb_read_visibility_{short}_{suffix}");
            let sql = connection
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE type='trigger' AND name=?1",
                    [&name],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("missing frozen-read visibility trigger {name}"))?;
            let expected = format!(
                "CREATE TRIGGER {name} AFTER {operation} ON {table} BEGIN UPDATE \
                 _fathomdb_read_visibility_state SET generation=CASE WHEN \
                 generation=9223372036854775807 THEN RAISE(ABORT,'read visibility generation \
                 exhausted') ELSE generation+1 END WHERE singleton=1; END"
            );
            if normalize_sql(&sql) != normalize_sql(&expected) {
                return Err(format!("frozen-read visibility trigger {name} differs from manifest"));
            }
        }
    }
    let count = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND name LIKE '_fathomdb_read_visibility_%'",
        [],
        |row| row.get::<_, i64>(0),
    ).map_err(|error| error.to_string())?;
    if count != i64::try_from(trigger_tables.len() * 3).unwrap_or(i64::MAX) {
        return Err("unexpected frozen-read visibility trigger".to_string());
    }
    if schema_version >= 33 {
        validate_page_index(
            connection,
            "canonical_nodes_kind_cursor_page_idx",
            "canonical_nodes",
            "CREATE UNIQUE INDEX canonical_nodes_kind_cursor_page_idx ON canonical_nodes(kind, write_cursor) WHERE logical_id IS NOT NULL",
        )?;
        validate_page_index(
            connection,
            "operational_state_collection_cursor_page_idx",
            "operational_state",
            "CREATE UNIQUE INDEX operational_state_collection_cursor_page_idx ON operational_state(collection_name, write_cursor)",
        )?;
        validate_page_index(
            connection,
            "operational_state_write_cursor_idx",
            "operational_state",
            "CREATE INDEX operational_state_write_cursor_idx ON operational_state(write_cursor)",
        )?;
        validate_page_index(
            connection,
            "_fathomdb_artifact_revisions_write_cursor_idx",
            "_fathomdb_artifact_revisions",
            "CREATE INDEX _fathomdb_artifact_revisions_write_cursor_idx ON _fathomdb_artifact_revisions(write_cursor)",
        )?;
    }
    Ok(())
}

fn validate_page_index(
    connection: &Connection,
    name: &str,
    table: &str,
    expected: &str,
) -> Result<(), String> {
    let sql = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='index' AND name=?1 AND tbl_name=?2",
            [name, table],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("missing pagination index {name}"))?;
    if normalize_sql(&sql) != normalize_sql(expected) {
        return Err(format!("pagination index {name} differs from manifest"));
    }
    Ok(())
}

fn normalize_sql(sql: &str) -> String {
    sql.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn binding_for_snapshot(
    connection: &Connection,
    effective_valid_at: i64,
    context_digest: [u8; 32],
) -> Result<FrozenReadBinding, EngineError> {
    Ok(FrozenReadBinding {
        database_id: read_open_state(connection, DATABASE_ID_KEY)?,
        effective_valid_at,
        canonical_write_boundary: load_next_cursor(connection),
        read_visibility_generation: load_visibility_generation(connection)?,
        dependency_generation: load_dependency_generation(connection)?,
        projection_cursor: load_projection_cursor(connection).map_err(|_| EngineError::Storage)?,
        registry_digest: projection_registry_digest(connection)?,
        serving_digest: projection_serving_digest(connection)?,
        context_digest,
    })
}

fn projection_registry_digest(connection: &Connection) -> Result<[u8; 32], EngineError> {
    let mut bytes = Vec::from(REGISTRY_DOMAIN);
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
        let (name, roles, fts, embedder, vector, source) = row.map_err(|_| EngineError::Storage)?;
        encode_string(&mut bytes, &name);
        encode_string(&mut bytes, &roles);
        encode_optional_string(&mut bytes, fts.as_deref());
        encode_optional_string(&mut bytes, embedder.as_deref());
        encode_i64(&mut bytes, vector);
        encode_optional_string(&mut bytes, source.as_deref());
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
    let mut bytes = if schema_version >= 33 {
        Vec::from(SERVING_DOMAIN_V3)
    } else if schema_version >= 32 {
        Vec::from(SERVING_DOMAIN_V2)
    } else {
        Vec::from(SERVING_DOMAIN_V1)
    };
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
    // Schema 33 visibility triggers make the monotonic generation in the outer
    // binding authoritative for every terminal insert/update/delete. Retaining
    // the historical row-by-row digest would add O(terminal rows) work to every
    // frozen read while providing no additional drift signal. Schema 31/32
    // retain their exact encoding for pinned historical fixtures.
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
    let binding = FrozenReadBinding {
        database_id: cursor.string()?,
        effective_valid_at: cursor.i64()?,
        canonical_write_boundary: cursor.u64()?,
        read_visibility_generation: cursor.u64()?,
        dependency_generation: cursor.u64()?,
        projection_cursor: cursor.u64()?,
        registry_digest: cursor.array32()?,
        serving_digest: cursor.array32()?,
        context_digest: cursor.array32()?,
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

#[cfg(test)]
mod tests {
    use fathomdb_schema::{migrate, migrate_with_steps, MIGRATIONS};
    use proptest::prelude::*;
    use serde_json::Value;

    use super::*;

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
            };
            let encoded = encode_binding(&expected);
            let decoded = decode_binding(&encoded).unwrap();
            prop_assert_eq!(&decoded, &expected);
            prop_assert_eq!(encode_binding(&decoded), encoded);
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
        let (frozen, _) = mint(&mut connection, &context).unwrap();
        let payload_hex = frozen.token.split('.').nth(1).unwrap();
        assert_eq!(payload_hex, fixture["payload_hex"].as_str().unwrap());
        assert_eq!(frozen.token, fixture["token"].as_str().unwrap());
    }

    #[test]
    fn normative_v1_fixture_remains_pinned_at_schema_31() {
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
                    crate::DEFAULT_EMBEDDER_NAME,
                    crate::DEFAULT_EMBEDDER_REVISION,
                    crate::DEFAULT_EMBEDDER_DIMENSION
                ],
            )
            .unwrap();
        crate::ensure_vector_partition(&mut connection, crate::DEFAULT_EMBEDDER_DIMENSION).unwrap();
        crate::projection_generation::bootstrap(&mut connection, 32).unwrap();
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
        crate::register_sqlite_vec_extension();
        let mut connection = Connection::open_in_memory().unwrap();
        migrate_with_steps(&connection, &MIGRATIONS[..32]).unwrap();
        connection
            .execute(
                "INSERT INTO _fathomdb_embedder_profiles(\
                   profile,name,revision,dimension,mean_vec\
                 ) VALUES('default',?1,?2,?3,NULL)",
                rusqlite::params![
                    crate::DEFAULT_EMBEDDER_NAME,
                    crate::DEFAULT_EMBEDDER_REVISION,
                    crate::DEFAULT_EMBEDDER_DIMENSION
                ],
            )
            .unwrap();
        crate::ensure_vector_partition(&mut connection, crate::DEFAULT_EMBEDDER_DIMENSION).unwrap();
        crate::projection_generation::bootstrap(&mut connection, 32).unwrap();
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
        let (frozen, _) = mint(&mut connection, &context).unwrap();

        connection
            .execute(
                "UPDATE operational_state SET payload_json='{\"value\":2}' \
                 WHERE collection_name='state' AND record_key='key'",
                [],
            )
            .unwrap();
        migrate(&connection).unwrap();

        let binding = authenticate(&connection, &frozen).unwrap();
        assert!(matches!(
            validate_snapshot(&connection, &binding),
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
        let (left_frozen, _) = mint(&mut left, &context).unwrap();
        let left_binding = authenticate(&left, &left_frozen).unwrap();
        validate_snapshot(&left, &left_binding).unwrap();

        let copied_binding = authenticate(&right, &left_frozen).unwrap();
        assert!(matches!(
            validate_snapshot(&right, &copied_binding),
            Err(error) if error.reason == FrozenReadErrorReason::StateDrifted
        ));
    }
}
