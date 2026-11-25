use super::{
    EvidenceArmV1, EvidenceArtifactClassV1, EvidenceArtifactLifecycleV1, EvidenceContributionV1,
    EvidenceErrorReasonV1, EvidenceErrorV1, GraphEvidenceArtifactV1, ResolvedGraphEvidenceV1,
    SCHEMA_VERSION, TOKEN_MAX_BYTES,
};
use crate::errors::EngineError;
use crate::frozen_read;
use crate::properties::properties_to_json_value;
use crate::provenance::SourceLocator;
use crate::search_types::{CapturedGraphOrigin, GraphArmSeedSource};

const TOKEN_PREFIX: &str = "fdbev1";
const TOKEN_DOMAIN: &[u8] = b"fathomdb.evidence-ref.v1\0";
const GENERATION_DOMAIN: &[u8] = b"fathomdb.evidence.generation.v1\0";
const GENERATION_TAIL_DOMAIN: &[u8] = b"fathomdb.evidence.generation-tail.v1\0";

/// Encode resolved graph evidence into the canonical V1 JSON object.
pub fn encode_resolved_graph_evidence_v1(
    value: &ResolvedGraphEvidenceV1,
) -> Result<Vec<u8>, EvidenceErrorV1> {
    let artifact = match &value.artifact {
        GraphEvidenceArtifactV1::Node { logical_id, kind, body, properties } => {
            let mut obj = serde_json::Map::new();
            obj.insert("artifactClass".into(), serde_json::json!("node"));
            obj.insert("logicalId".into(), serde_json::json!(logical_id));
            obj.insert("kind".into(), serde_json::json!(kind));
            obj.insert("body".into(), serde_json::json!(body));
            if let Some(props) = properties.as_ref().filter(|m| !m.is_empty()) {
                obj.insert(
                    "properties".into(),
                    properties_to_json_value(props).map_err(|_| {
                        EvidenceErrorV1::new(
                            EvidenceErrorReasonV1::EvidenceCorrupt,
                            "/artifact/properties",
                        )
                    })?,
                );
            }
            serde_json::Value::Object(obj)
        }
        GraphEvidenceArtifactV1::Edge { logical_id, kind, body, properties, from, to } => {
            let mut obj = serde_json::Map::new();
            obj.insert("artifactClass".into(), serde_json::json!("edge"));
            obj.insert("logicalId".into(), serde_json::json!(logical_id));
            obj.insert("kind".into(), serde_json::json!(kind));
            obj.insert("body".into(), serde_json::json!(body));
            if let Some(props) = properties.as_ref().filter(|m| !m.is_empty()) {
                obj.insert(
                    "properties".into(),
                    properties_to_json_value(props).map_err(|_| {
                        EvidenceErrorV1::new(
                            EvidenceErrorReasonV1::EvidenceCorrupt,
                            "/artifact/properties",
                        )
                    })?,
                );
            }
            obj.insert("from".into(), serde_json::json!(from));
            obj.insert("to".into(), serde_json::json!(to));
            serde_json::Value::Object(obj)
        }
    };
    let locator = match &value.locator {
        SourceLocator::WholeBody => serde_json::json!({
            "kind": "whole_body", "startInclusive": null, "endExclusive": null
        }),
        SourceLocator::Utf8Bytes { start_inclusive, end_exclusive } => serde_json::json!({
            "kind": "utf8_bytes", "startInclusive": start_inclusive.to_string(),
            "endExclusive": end_exclusive.to_string()
        }),
    };
    let lifecycle = match &value.artifact_lifecycle {
        EvidenceArtifactLifecycleV1::Node { state, superseded } => serde_json::json!({
            "kind": "node", "state": state.as_str(), "superseded": superseded,
            "validAtEffective": null
        }),
        EvidenceArtifactLifecycleV1::Edge { superseded, valid_at_effective } => serde_json::json!({
            "kind": "edge", "state": null, "superseded": superseded,
            "validAtEffective": valid_at_effective
        }),
    };
    let dependency = value.dependency.as_ref().map(|item| {
        serde_json::json!({
            "schemaVersion": item.schema_version,
            "dependencyId": item.dependency_id.as_str(),
            "sourceRevisionId": item.source_revision_id.as_str(),
            "derivedRevisionId": item.derived_revision_id.as_str(),
            "registeredDependencyGeneration": item.registered_dependency_generation.to_string()
        })
    });
    serde_json::to_vec(&serde_json::json!({
        "schemaVersion": value.schema_version,
        "artifactRevisionId": value.artifact_revision_id.as_str(),
        "artifact": artifact,
        "sourceId": value.source_id,
        "sourceVersionId": value.source_version_id,
        "sourceRevisionId": value.source_revision_id.as_str(),
        "locator": locator,
        "canonicalSourceBody": value.canonical_source_body,
        "evidenceText": value.evidence_text,
        "canonicalSourceHash": {
            "algorithm": "sha256", "digestHex": value.canonical_source_hash.digest_hex()
        },
        "effectiveValidAt": value.effective_valid_at,
        "artifactLifecycle": lifecycle,
        "sourceLifecycleState": value.source_lifecycle_state.as_str(),
        "dependency": dependency
    }))
    .map_err(|_| EvidenceErrorV1::new(EvidenceErrorReasonV1::EvidenceCorrupt, ""))
}

#[derive(Clone, Debug)]
pub(super) struct Payload {
    pub(super) artifact_class: EvidenceArtifactClassV1,
    pub(super) write_cursor: u64,
    pub(super) effective_valid_at: i64,
    pub(super) database_commitment: [u8; 32],
    pub(super) context_commitment: [u8; 32],
    pub(super) artifact_commitment: [u8; 32],
    pub(super) source_commitment: [u8; 32],
    pub(super) locator_commitment: [u8; 32],
    pub(super) hash_commitment: [u8; 32],
    pub(super) generation_nonce: [u8; 16],
    pub(super) generation_ciphertext: Vec<u8>,
    pub(super) arm: EvidenceArmV1,
    pub(super) contribution: EvidenceContributionV1,
    pub(super) graph_origin: Option<CapturedGraphOrigin>,
    pub(super) graph_edge_commitment: Option<[u8; 32]>,
}

pub(super) fn protect_generation(key: &[u8], nonce: &[u8; 16], value: &[u8]) -> Vec<u8> {
    let first = frozen_read::hmac_sha256(key, GENERATION_DOMAIN, nonce);
    let second = frozen_read::hmac_sha256(key, GENERATION_TAIL_DOMAIN, nonce);
    value.iter().zip(first.iter().chain(second.iter())).map(|(value, mask)| value ^ mask).collect()
}

pub(super) fn random_nonce() -> Result<[u8; 16], EngineError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| EngineError::Storage)?;
    Ok(bytes)
}

pub(super) fn keyed(key: &[u8], domain: &[u8], value: &[u8]) -> [u8; 32] {
    frozen_read::hmac_sha256(key, domain, value)
}

pub(super) fn locator_bytes(locator: &SourceLocator) -> Vec<u8> {
    match locator {
        SourceLocator::WholeBody => vec![0],
        SourceLocator::Utf8Bytes { start_inclusive, end_exclusive } => {
            let mut bytes = vec![1];
            bytes.extend_from_slice(&start_inclusive.to_be_bytes());
            bytes.extend_from_slice(&end_exclusive.to_be_bytes());
            bytes
        }
    }
}

pub(super) fn slice(body: &str, locator: &SourceLocator) -> Result<String, EngineError> {
    match locator {
        SourceLocator::WholeBody => Ok(body.to_string()),
        SourceLocator::Utf8Bytes { start_inclusive, end_exclusive } => {
            let start = usize::try_from(*start_inclusive).map_err(|_| corrupt_locator())?;
            let end = usize::try_from(*end_exclusive).map_err(|_| corrupt_locator())?;
            body.get(start..end).map(ToString::to_string).ok_or_else(corrupt_locator)
        }
    }
}

fn corrupt_locator() -> EngineError {
    EvidenceErrorV1::new(EvidenceErrorReasonV1::EvidenceCorrupt, "/provenance/sourceLocator").into()
}

pub(super) fn encode_token(key: &[u8], payload: &Payload) -> Result<String, EngineError> {
    let bytes = encode_payload(payload);
    let mac = frozen_read::hmac_sha256(key, TOKEN_DOMAIN, &bytes);
    let token = format!(
        "{TOKEN_PREFIX}.{}.{}",
        frozen_read::hex_encode(&bytes),
        frozen_read::hex_encode(&mac)
    );
    if token.len() > TOKEN_MAX_BYTES {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    Ok(token)
}

pub(super) fn encode_payload(payload: &Payload) -> Vec<u8> {
    let mut bytes = Vec::new();
    frozen_read::encode_u32(&mut bytes, SCHEMA_VERSION);
    bytes.push(payload.artifact_class.tag());
    frozen_read::encode_u64(&mut bytes, payload.write_cursor);
    frozen_read::encode_i64(&mut bytes, payload.effective_valid_at);
    for commitment in [
        payload.database_commitment,
        payload.context_commitment,
        payload.artifact_commitment,
        payload.source_commitment,
        payload.locator_commitment,
        payload.hash_commitment,
    ] {
        bytes.extend_from_slice(&commitment);
    }
    bytes.extend_from_slice(&payload.generation_nonce);
    frozen_read::encode_u32(
        &mut bytes,
        u32::try_from(payload.generation_ciphertext.len())
            .expect("projection generation ciphertext is bounded"),
    );
    bytes.extend_from_slice(&payload.generation_ciphertext);
    bytes.push(payload.arm.tag());
    encode_optional_u32(&mut bytes, payload.contribution.vector_rank);
    encode_optional_u32(&mut bytes, payload.contribution.text_rank);
    encode_optional_u32(&mut bytes, payload.contribution.graph_rank);
    encode_f64(&mut bytes, payload.contribution.fused_score);
    encode_optional_f64(&mut bytes, payload.contribution.ce_score);
    encode_f64(&mut bytes, payload.contribution.blended_score);
    encode_optional_f64(&mut bytes, payload.contribution.importance);
    encode_optional_f64(&mut bytes, payload.contribution.confidence);
    match (&payload.graph_origin, payload.graph_edge_commitment) {
        (None, None) => bytes.push(0),
        (Some(CapturedGraphOrigin::EdgeSeed { edge_cursor }), Some(commitment)) => {
            bytes.push(2);
            bytes.push(GraphArmSeedSource::EdgeFts.tag());
            frozen_read::encode_u64(&mut bytes, *edge_cursor);
            bytes.extend_from_slice(&commitment);
        }
        (
            Some(CapturedGraphOrigin::Traversal { edge_cursor, hop_count, seed_source }),
            Some(commitment),
        ) => {
            bytes.push(3);
            bytes.push(seed_source.tag());
            frozen_read::encode_u64(&mut bytes, *edge_cursor);
            frozen_read::encode_u32(&mut bytes, *hop_count);
            bytes.extend_from_slice(&commitment);
        }
        _ => unreachable!("graph edge origin and commitment are constructed together"),
    }
    bytes
}

pub(super) fn decode_token(key: &[u8], token: &str) -> Result<Payload, EngineError> {
    if token.len() > TOKEN_MAX_BYTES {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let mut parts = token.split('.');
    let (Some(prefix), Some(payload_hex), Some(mac_hex), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(EvidenceErrorV1::unavailable().into());
    };
    if prefix != TOKEN_PREFIX {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let bytes = frozen_read::hex_decode(payload_hex).ok_or_else(EvidenceErrorV1::unavailable)?;
    let mac = frozen_read::hex_decode(mac_hex).ok_or_else(EvidenceErrorV1::unavailable)?;
    let expected = frozen_read::hmac_sha256(key, TOKEN_DOMAIN, &bytes);
    if !frozen_read::constant_time_eq(&mac, &expected) {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let payload = decode_payload(&bytes)?;
    if encode_payload(&payload) != bytes {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    Ok(payload)
}

fn decode_payload(bytes: &[u8]) -> Result<Payload, EngineError> {
    let mut cursor = PayloadCursor { bytes, offset: 0 };
    if cursor.u32()? != SCHEMA_VERSION {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let artifact_class =
        EvidenceArtifactClassV1::from_tag(cursor.u8()?).ok_or_else(EvidenceErrorV1::unavailable)?;
    let write_cursor = cursor.u64()?;
    let effective_valid_at = cursor.i64()?;
    let database_commitment = cursor.array32()?;
    let context_commitment = cursor.array32()?;
    let artifact_commitment = cursor.array32()?;
    let source_commitment = cursor.array32()?;
    let locator_commitment = cursor.array32()?;
    let hash_commitment = cursor.array32()?;
    let generation_nonce = cursor.array16()?;
    let generation_length =
        usize::try_from(cursor.u32()?).map_err(|_| EvidenceErrorV1::unavailable())?;
    if generation_length == 0 || generation_length > 64 {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let generation_ciphertext = cursor.take(generation_length)?.to_vec();
    let arm = EvidenceArmV1::from_tag(cursor.u8()?).ok_or_else(EvidenceErrorV1::unavailable)?;
    let contribution = EvidenceContributionV1 {
        schema_version: SCHEMA_VERSION,
        vector_rank: cursor.optional_u32()?,
        text_rank: cursor.optional_u32()?,
        graph_rank: cursor.optional_u32()?,
        fused_score: cursor.f64()?,
        ce_score: cursor.optional_f64()?,
        blended_score: cursor.f64()?,
        importance: cursor.optional_f64()?,
        confidence: cursor.optional_f64()?,
    };
    let (graph_origin, graph_edge_commitment) = match cursor.u8()? {
        0 => (None, None),
        2 => {
            let seed_source = GraphArmSeedSource::from_tag(cursor.u8()?)
                .ok_or_else(EvidenceErrorV1::unavailable)?;
            if seed_source != GraphArmSeedSource::EdgeFts {
                return Err(EvidenceErrorV1::unavailable().into());
            }
            (
                Some(CapturedGraphOrigin::EdgeSeed { edge_cursor: cursor.u64()? }),
                Some(cursor.array32()?),
            )
        }
        3 => {
            let seed_source = GraphArmSeedSource::from_tag(cursor.u8()?)
                .ok_or_else(EvidenceErrorV1::unavailable)?;
            (
                Some(CapturedGraphOrigin::Traversal {
                    edge_cursor: cursor.u64()?,
                    hop_count: cursor.u32()?,
                    seed_source,
                }),
                Some(cursor.array32()?),
            )
        }
        _ => return Err(EvidenceErrorV1::unavailable().into()),
    };
    if cursor.offset != bytes.len() {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    Ok(Payload {
        artifact_class,
        write_cursor,
        effective_valid_at,
        database_commitment,
        context_commitment,
        artifact_commitment,
        source_commitment,
        locator_commitment,
        hash_commitment,
        generation_nonce,
        generation_ciphertext,
        arm,
        contribution,
        graph_origin,
        graph_edge_commitment,
    })
}

fn encode_optional_u32(bytes: &mut Vec<u8>, value: Option<u32>) {
    match value {
        Some(value) => {
            bytes.push(1);
            frozen_read::encode_u32(bytes, value);
        }
        None => bytes.push(0),
    }
}

fn encode_f64(bytes: &mut Vec<u8>, value: f64) {
    bytes.extend_from_slice(&value.to_bits().to_be_bytes());
}

fn encode_optional_f64(bytes: &mut Vec<u8>, value: Option<f64>) {
    match value {
        Some(value) => {
            bytes.push(1);
            encode_f64(bytes, value);
        }
        None => bytes.push(0),
    }
}

struct PayloadCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl PayloadCursor<'_> {
    fn take(&mut self, count: usize) -> Result<&[u8], EngineError> {
        let end = self.offset.checked_add(count).ok_or_else(EvidenceErrorV1::unavailable)?;
        let value = self.bytes.get(self.offset..end).ok_or_else(EvidenceErrorV1::unavailable)?;
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, EngineError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, EngineError> {
        Ok(u32::from_be_bytes(
            self.take(4)?.try_into().map_err(|_| EvidenceErrorV1::unavailable())?,
        ))
    }

    fn u64(&mut self) -> Result<u64, EngineError> {
        Ok(u64::from_be_bytes(
            self.take(8)?.try_into().map_err(|_| EvidenceErrorV1::unavailable())?,
        ))
    }

    fn i64(&mut self) -> Result<i64, EngineError> {
        Ok(i64::from_be_bytes(
            self.take(8)?.try_into().map_err(|_| EvidenceErrorV1::unavailable())?,
        ))
    }

    fn array32(&mut self) -> Result<[u8; 32], EngineError> {
        self.take(32)?.try_into().map_err(|_| EvidenceErrorV1::unavailable().into())
    }

    fn array16(&mut self) -> Result<[u8; 16], EngineError> {
        self.take(16)?.try_into().map_err(|_| EvidenceErrorV1::unavailable().into())
    }

    fn optional_u32(&mut self) -> Result<Option<u32>, EngineError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.u32()?)),
            _ => Err(EvidenceErrorV1::unavailable().into()),
        }
    }

    fn f64(&mut self) -> Result<f64, EngineError> {
        let value = f64::from_bits(self.u64()?);
        if value.is_finite() {
            Ok(value)
        } else {
            Err(EvidenceErrorV1::unavailable().into())
        }
    }

    fn optional_f64(&mut self) -> Result<Option<f64>, EngineError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.f64()?)),
            _ => Err(EvidenceErrorV1::unavailable().into()),
        }
    }
}
