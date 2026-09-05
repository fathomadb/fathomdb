use std::fmt::{Display, Formatter};

use rusqlite::Connection;

use crate::{frozen_read, EngineError, FrozenReadContextV1};

pub(crate) const PAGE_SCHEMA_VERSION: u32 = 1;
pub(crate) const PAGE_LIMIT_MAX: usize = 250;
const CURSOR_PREFIX: &str = "fdbpg1";
const CURSOR_MAX_BYTES: usize = 2 * 1024;
const SELECTOR_DOMAIN: &[u8] = b"fathomdb.page-selector.v1\0";

/// Opaque authenticated continuation token for one governed page walk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageCursor(pub String);

/// Bounded versioned page request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageRequestV1 {
    /// Wire schema version. Version 1 is the only accepted value.
    pub schema_version: u32,
    /// Number of records to return, in the inclusive range `1..=250`.
    pub limit: usize,
    /// Exclusive continuation returned by the preceding page.
    pub cursor: Option<PageCursor>,
}

/// One stable keyset page and its optional continuation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageV1<T> {
    /// Wire schema version. Always 1 in this release.
    pub schema_version: u32,
    /// Records in deterministic persisted-cursor order.
    pub items: Vec<T>,
    /// Continuation for the next page, or `None` at exhaustion.
    pub next_cursor: Option<PageCursor>,
}

/// Closed machine-readable pagination refusal reason.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageErrorReason {
    UnsupportedSchemaVersion,
    InvalidPageLimit,
    CursorMalformed,
    CursorTooLarge,
    CursorAuthenticationFailed,
    DatabaseMismatch,
    CursorMismatch,
    ContextNotApplicable,
    CollectionNotFound,
    CollectionKindMismatch,
    CollectionFormatUnsupported,
}

impl PageErrorReason {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedSchemaVersion => "unsupported_schema_version",
            Self::InvalidPageLimit => "invalid_page_limit",
            Self::CursorMalformed => "cursor_malformed",
            Self::CursorTooLarge => "cursor_too_large",
            Self::CursorAuthenticationFailed => "cursor_authentication_failed",
            Self::DatabaseMismatch => "database_mismatch",
            Self::CursorMismatch => "cursor_mismatch",
            Self::ContextNotApplicable => "context_not_applicable",
            Self::CollectionNotFound => "collection_not_found",
            Self::CollectionKindMismatch => "collection_kind_mismatch",
            Self::CollectionFormatUnsupported => "collection_format_unsupported",
        }
    }
}

/// Typed pagination refusal without selector, cursor, or record disclosure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageError {
    /// Closed refusal reason.
    pub reason: PageErrorReason,
    /// RFC 6901 field path over canonical camel-case wire names.
    pub field_path: String,
}

impl PageError {
    pub(crate) fn new(reason: PageErrorReason, field_path: impl Into<String>) -> Self {
        Self { reason, field_path: field_path.into() }
    }
}

impl Display for PageError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} at {}", self.reason.as_str(), self.field_path)
    }
}

impl std::error::Error for PageError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PageOperation {
    CanonicalNode,
    OperationalState,
}

impl PageOperation {
    fn as_str(self) -> &'static str {
        match self {
            Self::CanonicalNode => "canonical_node_page",
            Self::OperationalState => "operational_state_page",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AuthenticatedCursor {
    database_id: String,
    operation: String,
    selector_digest: [u8; 32],
    context_digest: [u8; 32],
    ordering_version: u32,
    limit: u64,
    last_write_cursor: u64,
}

pub(crate) fn validate_request(request: &PageRequestV1) -> Result<(), EngineError> {
    if request.schema_version != PAGE_SCHEMA_VERSION {
        return Err(
            PageError::new(PageErrorReason::UnsupportedSchemaVersion, "/schemaVersion").into()
        );
    }
    if request.limit == 0 || request.limit > PAGE_LIMIT_MAX {
        return Err(PageError::new(PageErrorReason::InvalidPageLimit, "/limit").into());
    }
    Ok(())
}

pub(crate) fn continuation(
    connection: &Connection,
    operation: PageOperation,
    selector: &str,
    frozen: &FrozenReadContextV1,
    limit: usize,
    last_write_cursor: u64,
) -> Result<PageCursor, EngineError> {
    let (database_id, key) = frozen_read::page_cursor_material(connection)?;
    let payload = AuthenticatedCursor {
        database_id,
        operation: operation.as_str().to_string(),
        selector_digest: frozen_read::digest(SELECTOR_DOMAIN, selector.as_bytes()),
        context_digest: frozen_read::page_context_digest(frozen)?,
        ordering_version: 1,
        limit: u64::try_from(limit).map_err(|_| EngineError::Storage)?,
        last_write_cursor,
    };
    let encoded = encode_payload(&payload);
    let mac = frozen_read::hmac_sha256(&key, frozen_read::PAGE_CURSOR_DOMAIN, &encoded);
    Ok(PageCursor(format!(
        "{CURSOR_PREFIX}.{}.{}",
        frozen_read::hex_encode(&encoded),
        frozen_read::hex_encode(&mac)
    )))
}

pub(crate) fn authenticate_cursor(
    connection: &Connection,
    request: &PageRequestV1,
) -> Result<Option<AuthenticatedCursor>, EngineError> {
    let Some(cursor) = &request.cursor else {
        return Ok(None);
    };
    if cursor.0.len() > CURSOR_MAX_BYTES {
        return Err(PageError::new(PageErrorReason::CursorTooLarge, "/cursor").into());
    }
    let mut pieces = cursor.0.split('.');
    let (Some(prefix), Some(payload_hex), Some(mac_hex), None) =
        (pieces.next(), pieces.next(), pieces.next(), pieces.next())
    else {
        return Err(PageError::new(PageErrorReason::CursorMalformed, "/cursor").into());
    };
    if prefix != CURSOR_PREFIX {
        return Err(PageError::new(PageErrorReason::CursorMalformed, "/cursor").into());
    }
    let payload_bytes = frozen_read::hex_decode(payload_hex)
        .ok_or_else(|| PageError::new(PageErrorReason::CursorMalformed, "/cursor"))?;
    let supplied_mac = frozen_read::hex_decode(mac_hex)
        .ok_or_else(|| PageError::new(PageErrorReason::CursorAuthenticationFailed, "/cursor"))?;
    let (database_id, key) = frozen_read::page_cursor_material(connection)?;
    let expected_mac =
        frozen_read::hmac_sha256(&key, frozen_read::PAGE_CURSOR_DOMAIN, &payload_bytes);
    if !frozen_read::constant_time_eq(&supplied_mac, &expected_mac) {
        return Err(PageError::new(PageErrorReason::CursorAuthenticationFailed, "/cursor").into());
    }
    let payload = decode_payload(&payload_bytes)?;
    if payload.database_id != database_id {
        return Err(PageError::new(PageErrorReason::DatabaseMismatch, "/cursor").into());
    }
    Ok(Some(payload))
}

pub(crate) fn resume_after(
    cursor: Option<&AuthenticatedCursor>,
    operation: PageOperation,
    selector: &str,
    frozen: &FrozenReadContextV1,
    request: &PageRequestV1,
) -> Result<u64, EngineError> {
    let Some(payload) = cursor else {
        return Ok(0);
    };
    let context_digest = frozen_read::page_context_digest(frozen)?;
    let limit = u64::try_from(request.limit).map_err(|_| EngineError::Storage)?;
    if payload.operation != operation.as_str()
        || payload.selector_digest != frozen_read::digest(SELECTOR_DOMAIN, selector.as_bytes())
        || payload.context_digest != context_digest
        || payload.ordering_version != 1
        || payload.limit != limit
    {
        return Err(PageError::new(PageErrorReason::CursorMismatch, "/cursor").into());
    }
    Ok(payload.last_write_cursor)
}

fn encode_payload(payload: &AuthenticatedCursor) -> Vec<u8> {
    let mut bytes = Vec::new();
    frozen_read::encode_u32(&mut bytes, PAGE_SCHEMA_VERSION);
    frozen_read::encode_string(&mut bytes, &payload.database_id);
    frozen_read::encode_string(&mut bytes, &payload.operation);
    bytes.extend_from_slice(&payload.selector_digest);
    bytes.extend_from_slice(&payload.context_digest);
    frozen_read::encode_u32(&mut bytes, payload.ordering_version);
    frozen_read::encode_u64(&mut bytes, payload.limit);
    frozen_read::encode_u64(&mut bytes, payload.last_write_cursor);
    bytes
}

fn decode_payload(bytes: &[u8]) -> Result<AuthenticatedCursor, EngineError> {
    let mut cursor = Cursor { bytes, offset: 0 };
    if cursor.u32()? != PAGE_SCHEMA_VERSION {
        return Err(PageError::new(PageErrorReason::CursorMalformed, "/cursor").into());
    }
    let payload = AuthenticatedCursor {
        database_id: cursor.string()?,
        operation: cursor.string()?,
        selector_digest: cursor.array32()?,
        context_digest: cursor.array32()?,
        ordering_version: cursor.u32()?,
        limit: cursor.u64()?,
        last_write_cursor: cursor.u64()?,
    };
    if cursor.offset != bytes.len() || encode_payload(&payload) != bytes {
        return Err(PageError::new(PageErrorReason::CursorMalformed, "/cursor").into());
    }
    Ok(payload)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl Cursor<'_> {
    fn take(&mut self, length: usize) -> Result<&[u8], EngineError> {
        let end = self.offset.checked_add(length).ok_or_else(cursor_malformed)?;
        let value = self.bytes.get(self.offset..end).ok_or_else(cursor_malformed)?;
        self.offset = end;
        Ok(value)
    }

    fn u32(&mut self) -> Result<u32, EngineError> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().map_err(|_| cursor_malformed())?))
    }

    fn u64(&mut self) -> Result<u64, EngineError> {
        Ok(u64::from_be_bytes(self.take(8)?.try_into().map_err(|_| cursor_malformed())?))
    }

    fn string(&mut self) -> Result<String, EngineError> {
        let length = usize::try_from(self.u32()?).map_err(|_| cursor_malformed())?;
        String::from_utf8(self.take(length)?.to_vec()).map_err(|_| cursor_malformed())
    }

    fn array32(&mut self) -> Result<[u8; 32], EngineError> {
        self.take(32)?.try_into().map_err(|_| cursor_malformed())
    }
}

fn cursor_malformed() -> EngineError {
    PageError::new(PageErrorReason::CursorMalformed, "/cursor").into()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn cursor_payload_codec_round_trips_canonically(
            database_id in "[0-9a-f]{32}",
            operation in "[a-z_]{1,32}",
            selector_digest in any::<[u8; 32]>(),
            context_digest in any::<[u8; 32]>(),
            ordering_version in any::<u32>(),
            limit in any::<u64>(),
            last_write_cursor in any::<u64>(),
        ) {
            let expected = AuthenticatedCursor {
                database_id,
                operation,
                selector_digest,
                context_digest,
                ordering_version,
                limit,
                last_write_cursor,
            };
            let encoded = encode_payload(&expected);
            let decoded = decode_payload(&encoded).unwrap();
            prop_assert_eq!(&decoded, &expected);
            prop_assert_eq!(encode_payload(&decoded), encoded);
        }

        #[test]
        fn every_truncated_cursor_payload_is_refused(
            database_id in "[0-9a-f]{32}",
            selector_digest in any::<[u8; 32]>(),
            context_digest in any::<[u8; 32]>(),
            last_write_cursor in any::<u64>(),
        ) {
            let payload = AuthenticatedCursor {
                database_id,
                operation: "canonical_node_page".to_string(),
                selector_digest,
                context_digest,
                ordering_version: 1,
                limit: 100,
                last_write_cursor,
            };
            let encoded = encode_payload(&payload);
            for length in 0..encoded.len() {
                let refused = matches!(
                    decode_payload(&encoded[..length]),
                    Err(EngineError::Page(PageError {
                        reason: PageErrorReason::CursorMalformed,
                        ..
                    }))
                );
                prop_assert!(refused);
            }
        }
    }
}
