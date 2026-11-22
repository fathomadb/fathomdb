use super::*;
use rusqlite::OptionalExtension;
use serde::Serialize;

/// The stored default embedder identity to which mapped-head facts belong.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StoredHeadCoverageProfile {
    /// Stored profile key; currently always `default`.
    pub key: String,
    /// Stored embedder name, independent of current runtime availability.
    pub name: String,
    /// Stored embedder revision.
    pub revision: String,
    /// Stored vector dimension, retaining its numeric u32 contract.
    pub dimension: u32,
}

/// An exact unreduced share; denominator is strictly positive.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StoredHeadCoverageFraction {
    /// Exact nonnegative numerator.
    pub numerator: u128,
    /// Exact positive denominator.
    pub denominator: u128,
}

/// Exact facts about attributable committed mapped head spans, including history.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct StoredHeadCoverageCounts {
    /// Attributable mapped head count, known plus unknown.
    pub records: u64,
    /// Heads with a known endpoint, including known-empty heads.
    pub known_records: u64,
    /// Heads whose validated stored endpoint is NULL.
    pub unknown_records: u64,
    /// Known heads ending before their stored body length.
    pub truncated_known_records: u64,
    /// Sum of original stored body byte lengths over known heads only.
    pub known_body_bytes: u128,
    /// Sum of body length minus endpoint over known heads only.
    pub unrepresented_known_bytes: u128,
    /// Passing mapped heads without exactly one non-NULL canonical body owner.
    pub unattributed_heads: u64,
    /// Exact truncated-known / known share, absent when known count is zero.
    pub truncated_known_share: Option<StoredHeadCoverageFraction>,
    /// Exact unrepresented / known-body bytes, absent when known bytes is zero.
    pub unrepresented_known_byte_share: Option<StoredHeadCoverageFraction>,
}

/// Why committed mapped-span facts cannot be reused for this writer snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StoredHeadCoverageUnavailableReason {
    /// This engine has not retained a successful full writer validation.
    CacheCold,
    /// The current writer token differs from the retained committed capture.
    StateChanged,
}

/// Stored mapped-head coverage; unavailable variants contain no numeric facts.
///
/// This population is metadata, not current eligibility or physical-vector health.
/// Availability may remain unavailable until an existing full status read or reopen.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum StoredHeadCoverageV1 {
    /// Facts captured by an existing full validation and committed successfully.
    Available {
        /// Fixed population discriminator `stored_mapped_spans`.
        population: &'static str,
        /// Stored default profile on the captured writer snapshot.
        profile: StoredHeadCoverageProfile,
        /// Exact counts, bytes and nullable shares for that snapshot.
        #[serde(flatten)]
        counts: StoredHeadCoverageCounts,
    },
    /// No matching committed summary; numeric facts are intentionally absent.
    Unavailable {
        /// Fixed population discriminator `stored_mapped_spans`.
        population: &'static str,
        /// Current stored default profile.
        profile: StoredHeadCoverageProfile,
        /// Availability reason, never a substitute for metadata/SQL errors.
        reason: StoredHeadCoverageUnavailableReason,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CoverageToken {
    database_id: String,
    user_version: u32,
    schema_version: u32,
    pub(crate) profile: StoredHeadCoverageProfile,
    generation: u64,
    nonce: String,
    data_version: u64,
    total_changes: u64,
}

#[derive(Clone)]
pub(crate) struct CachedStoredHeadCoverage {
    pub(crate) token: CoverageToken,
    pub(crate) counts: StoredHeadCoverageCounts,
}

fn corrupt() -> EngineError {
    projection_generation::corruption_error()
}

pub(crate) fn token(connection: &Connection) -> Result<Option<CoverageToken>, EngineError> {
    let profile = connection.query_row(
        "SELECT name,revision,dimension FROM _fathomdb_embedder_profiles WHERE profile='default'",
        [], |row| Ok(StoredHeadCoverageProfile { key: "default".into(), name: row.get(0)?, revision: row.get(1)?, dimension: row.get(2)? }),
    ).optional().map_err(|_| corrupt())?;
    let Some(profile) = profile else {
        return Ok(None);
    };
    if profile.name.is_empty() || profile.revision.is_empty() || profile.dimension == 0 {
        return Err(corrupt());
    }
    let database_id = frozen_read::read_open_state(connection, "_fathomdb_database_id")?;
    let (generation, nonce, clean): (u64, String, i64) = connection.query_row(
        "SELECT generation,state_nonce,coalescing FROM _fathomdb_read_visibility_state WHERE singleton=1",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|_| corrupt())?;
    let hex = |value: &str, length| {
        value.len() == length
            && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    };
    if !hex(&database_id, 32) || !hex(&nonce, 64) || clean != 0 || generation > i64::MAX as u64 {
        return Err(corrupt());
    }
    let token = CoverageToken {
        database_id,
        profile,
        generation,
        nonce,
        user_version: connection
            .pragma_query_value(Some("main"), "user_version", |r| r.get(0))
            .map_err(|_| corrupt())?,
        schema_version: connection
            .pragma_query_value(Some("main"), "schema_version", |r| r.get(0))
            .map_err(|_| corrupt())?,
        data_version: connection
            .pragma_query_value(Some("main"), "data_version", |r| r.get(0))
            .map_err(|_| corrupt())?,
        total_changes: connection.total_changes(),
    };
    if token.user_version != fathomdb_schema::SCHEMA_VERSION || token.schema_version == 0 {
        return Err(corrupt());
    }
    Ok(Some(token))
}

pub(crate) fn capture_with_counts(
    connection: &Connection,
    counts: StoredHeadCoverageCounts,
) -> Result<Option<CachedStoredHeadCoverage>, EngineError> {
    Ok(token(connection)?.map(|token| CachedStoredHeadCoverage { token, counts }))
}

pub(crate) fn retain_matching_open_seed(
    connection: &Connection,
    seed: Option<CachedStoredHeadCoverage>,
) -> Result<Option<CachedStoredHeadCoverage>, EngineError> {
    let tx =
        rusqlite::Transaction::new_unchecked(connection, rusqlite::TransactionBehavior::Deferred)
            .map_err(|_| EngineError::Storage)?;
    let current = token(&tx)?;
    tx.commit().map_err(|_| EngineError::Storage)?;
    Ok(seed.filter(|seed| Some(&seed.token) == current.as_ref()))
}

impl Engine {
    pub(crate) fn publish_stored_head_coverage(
        &self,
        seed: Option<CachedStoredHeadCoverage>,
    ) -> Result<(), EngineError> {
        if let Some(seed) = seed {
            *self.stored_head_coverage.lock().map_err(|_| EngineError::Storage)? = Some(seed);
        }
        Ok(())
    }

    pub(crate) fn stored_head_coverage_in_snapshot(
        &self,
        connection: &Connection,
    ) -> Result<StoredHeadCoverageV1, EngineError> {
        let cached = self.stored_head_coverage.lock().map_err(|_| EngineError::Storage)?.clone();
        let token = token(connection)?.ok_or_else(corrupt)?;
        let population = "stored_mapped_spans";
        Ok(match cached {
            Some(cached) if cached.token == token => StoredHeadCoverageV1::Available {
                population,
                profile: token.profile,
                counts: cached.counts,
            },
            cached => StoredHeadCoverageV1::Unavailable {
                population,
                profile: token.profile,
                reason: if cached.is_some() {
                    StoredHeadCoverageUnavailableReason::StateChanged
                } else {
                    StoredHeadCoverageUnavailableReason::CacheCold
                },
            },
        })
    }
}
