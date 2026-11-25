use crate::errors::EngineError;
use crate::frozen_read;
#[cfg(feature = "test-hooks")]
use rusqlite::Connection;

use super::codec::{
    decode_token, encode_payload, encode_token, keyed, protect_generation, random_nonce, Payload,
};
use super::graph::{
    decode_graph_evidence_reference, frame_graph_selector, graph_evidence_authority,
    graph_request_commitment, protect_graph_selector, GraphEvidenceAuthority, GRAPH_MAC_DOMAIN,
    GRAPH_SELECTOR_BYTES, GRAPH_TOKEN_BYTES, GRAPH_TOKEN_PREFIX,
};
use super::*;
use proptest::prelude::*;

#[test]
fn graph_expansion_hashes_each_shared_source_once() {
    use crate::{
        GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, InitialState, PreparedWrite,
        ProvenancedEdgeV1, ProvenancedNodeV1, SourceId, SourceVersionId, WriteProvenanceV1,
    };
    let directory = tempfile::TempDir::new().unwrap();
    let engine = crate::Engine::open(directory.path().join("hash-count.fdb")).unwrap().engine;
    let body = "shared canonical source bytes";
    let derived = |revision: &str| {
        WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new("v1").unwrap(),
            SourceRevisionId::new("source-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(crate::canonical_body_hash(body)).unwrap(),
        )
    };
    let mut writes = vec![PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        logical_id: Some("root".into()),
        kind: "document".into(),
        body: body.into(),
        source_id: SourceId::new("owner").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new("source-r1").unwrap(),
            SourceVersionId::new("v1").unwrap(),
        ),
    })];
    for index in 0..2 {
        writes.push(PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            logical_id: Some(format!("target-{index}")),
            kind: "claim".into(),
            body: format!("claim {index}"),
            source_id: SourceId::new("owner").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
            provenance: derived(&format!("target-r{index}")),
        }));
        writes.push(PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
            logical_id: Some(format!("edge-{index}")),
            kind: "supports".into(),
            from: "root".into(),
            to: format!("target-{index}"),
            source_id: SourceId::new("owner").unwrap(),
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: Some(0.9),
            extractor_model_id: None,
            temporal_fallback: None,
            properties: None,
            provenance: derived(&format!("edge-r{index}")),
        }));
    }
    engine.write(&writes).unwrap();
    engine.drain(30_000).unwrap();
    let frozen = engine
        .freeze_read_context(
            &crate::ReadContextV1::new(crate::ReadView::default(), crate::SearchFilter::default())
                .unwrap(),
        )
        .unwrap();
    let request = GraphExpandRequestV1::new(
        GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![crate::IdSpace::logical("root")],
        },
        crate::TraversalDirection::Outgoing,
        GraphReadContextV1::Frozen { schema_version: 1, context: frozen.clone() },
        1,
        50,
        10_000,
    )
    .with_edge_kinds(vec!["supports".into()])
    .with_target_kinds(vec!["claim".into()])
    .with_evidence(true);
    let mut guard = engine.connection.lock().unwrap();
    let connection = guard.as_mut().unwrap();
    let binding = frozen_read::authenticate(connection, &frozen).unwrap();
    let authority = graph_evidence_authority(connection, &frozen).unwrap();
    // Run the real reader transaction on this thread to isolate the hash observer
    // from background workers and parallel tests without a process-global counter.
    crate::write_commit::CANONICAL_BODY_HASH_CALLS.with(|calls| calls.set(0));
    let result = crate::graph_expand::read_graph_expand_in_tx(
        connection,
        &request,
        Some(&binding),
        crate::ProjectionRuntimeStateV1::Absent,
        Some(&authority),
        #[cfg(feature = "test-hooks")]
        &crate::graph_expand::GraphExpandReaderControlsForTest::default(),
        &std::sync::Arc::new(crate::WalAttributionCollector::new()),
        0,
    )
    .unwrap();
    let calls = crate::write_commit::CANONICAL_BODY_HASH_CALLS.with(|calls| calls.get());
    assert_eq!(result.targets.len(), 2);
    assert_eq!(result.evidence.unwrap().entries.len(), 2);
    assert_eq!(calls, 1, "four selected artifacts share one canonical source");
}
#[cfg(feature = "test-hooks")]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "test-hooks")]
static NONCE_SQL_STATEMENTS: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "test-hooks")]
fn count_nonce_sql(event: rusqlite::trace::TraceEvent<'_>) {
    if matches!(event, rusqlite::trace::TraceEvent::Stmt(_, _)) {
        NONCE_SQL_STATEMENTS.fetch_add(1, Ordering::SeqCst);
    }
}

proptest! {
    #[test]
    fn canonical_token_round_trips_and_single_character_tamper_fails(
        write_cursor in any::<u64>(),
        effective_valid_at in any::<i64>(),
        rank in proptest::option::of(any::<u32>()),
        fused_score in -1_000_000_f64..1_000_000_f64,
        blended_score in -1_000_000_f64..1_000_000_f64,
        seed in any::<[u8; 32]>(),
    ) {
        let key = [0x5a; 32];
        let payload = Payload {
            artifact_class: EvidenceArtifactClassV1::Node,
            write_cursor,
            effective_valid_at,
            database_commitment: seed,
            context_commitment: keyed(&key, b"test-context", &seed),
            artifact_commitment: keyed(&key, b"test-artifact", &seed),
            source_commitment: keyed(&key, b"test-source", &seed),
            locator_commitment: keyed(&key, b"test-locator", &seed),
            hash_commitment: keyed(&key, b"test-hash", &seed),
            generation_nonce: seed[..16].try_into().unwrap(),
            generation_ciphertext: protect_generation(
                &key,
                &seed[..16].try_into().unwrap(),
                b"pgen1:00000000000000000000000000000000",
            ),
            arm: EvidenceArmV1::Text,
            contribution: EvidenceContributionV1 {
                schema_version: SCHEMA_VERSION,
                vector_rank: rank,
                text_rank: rank,
                graph_rank: None,
                fused_score,
                ce_score: None,
                blended_score,
                importance: None,
                confidence: None,
            },
            graph_origin: None,
            graph_edge_commitment: None,
        };

        let token = encode_token(&key, &payload).unwrap();
        prop_assert!(token.len() <= TOKEN_MAX_BYTES);
        let decoded = decode_token(&key, &token).unwrap();
        prop_assert_eq!(encode_payload(&decoded), encode_payload(&payload));

        let mut tampered = token.into_bytes();
        let index = tampered.len() - 1;
        tampered[index] = if tampered[index] == b'0' { b'1' } else { b'0' };
        let tampered = String::from_utf8(tampered).unwrap();
        let is_unavailable = matches!(
            decode_token(&key, &tampered),
            Err(EngineError::Evidence(EvidenceErrorV1 {
                reason: EvidenceErrorReasonV1::EvidenceUnavailable,
                ..
            }))
        );
        prop_assert!(is_unavailable);
    }

    #[test]
    fn graph_selector_round_trips_and_single_character_tamper_fails(
        nonce in any::<[u8; 16]>(),
        seed in any::<[u8; 32]>(),
        encoded_index in 0_usize..696,
    ) {
        let key = [0x6b; 32];
        let mut selector = [0_u8; GRAPH_SELECTOR_BYTES];
        selector[0..4].copy_from_slice(&1_u32.to_be_bytes());
        selector[4] = 0;
        selector[5] = EvidenceArtifactClassV1::Node.tag();
        selector[6] = 0;
        selector[8..12].copy_from_slice(&1_u32.to_be_bytes());
        selector[12..20].copy_from_slice(&7_u64.to_be_bytes());
        selector[20..28].copy_from_slice(&7_u64.to_be_bytes());
        selector[28..36].copy_from_slice(&9_u64.to_be_bytes());
        selector[36..44].copy_from_slice(&11_i64.to_be_bytes());
        for chunk in selector[44..].chunks_mut(32) {
            chunk.copy_from_slice(&seed[..chunk.len()]);
        }
        let token = frame_graph_selector(&key, &nonce, &selector);
        let reference = GraphEvidenceRefV1::new(token.clone()).unwrap();
        let decoded = decode_graph_evidence_reference(&reference, &key).unwrap();
        prop_assert_eq!(decoded.artifact_cursor, 7);

        let mut tampered = token.into_bytes();
        let index = GRAPH_TOKEN_PREFIX.len() + encoded_index;
        tampered[index] = if tampered[index] == b'0' { b'1' } else { b'0' };
        let reference =
            GraphEvidenceRefV1::new(String::from_utf8(tampered).unwrap()).unwrap();
        let unavailable = matches!(
            decode_graph_evidence_reference(&reference, &key),
            Err(EngineError::Evidence(EvidenceErrorV1 {
                reason: EvidenceErrorReasonV1::EvidenceUnavailable,
                ..
            }))
        );
        prop_assert!(unavailable);
    }
}

#[test]
fn generation_selector_resists_cross_token_known_plaintext_reuse() {
    let key = [0x31; 32];
    let generation = b"pgen1:00000000000000000000000000000000";
    let nonce_a = [0x41; 16];
    let nonce_b = [0x42; 16];
    let cipher_a = protect_generation(&key, &nonce_a, generation);
    let cipher_b = protect_generation(&key, &nonce_b, generation);
    assert_ne!(cipher_a, cipher_b);
    assert_eq!(protect_generation(&key, &nonce_a, &cipher_a), generation);

    let recovered_mask = cipher_a
        .iter()
        .zip(generation.iter())
        .map(|(cipher, plain)| cipher ^ plain)
        .collect::<Vec<_>>();
    let cross_token_guess =
        cipher_b.iter().zip(recovered_mask).map(|(cipher, mask)| cipher ^ mask).collect::<Vec<_>>();
    assert_ne!(cross_token_guess, generation);
}

#[test]
fn graph_selector_uses_ten_blocks_and_exact_fixed_nonce_framing() {
    let key = [0x31; 32];
    let nonce = [0x42; 16];
    let mut selector = [0_u8; GRAPH_SELECTOR_BYTES];
    selector[0..4].copy_from_slice(&1_u32.to_be_bytes());
    selector[4] = 0;
    selector[5] = EvidenceArtifactClassV1::Node.tag();
    selector[6] = 0;
    selector[8..12].copy_from_slice(&7_u32.to_be_bytes());
    selector[12..20].copy_from_slice(&11_u64.to_be_bytes());
    selector[20..28].copy_from_slice(&11_u64.to_be_bytes());
    selector[28..36].copy_from_slice(&19_u64.to_be_bytes());
    selector[36..44].copy_from_slice(&23_i64.to_be_bytes());
    for (index, byte) in selector[44..].iter_mut().enumerate() {
        *byte = u8::try_from(index % 251).unwrap();
    }

    let ciphertext = protect_graph_selector(&key, &nonce, &selector);
    assert_eq!(ciphertext.len(), GRAPH_SELECTOR_BYTES);
    assert_eq!(protect_graph_selector(&key, &nonce, &ciphertext), selector);
    assert_ne!(&ciphertext[64..96], &selector[64..96]);
    assert_ne!(&ciphertext[288..300], &selector[288..300]);

    let token = frame_graph_selector(&key, &nonce, &selector);
    assert_eq!(token.len(), GRAPH_TOKEN_BYTES);
    assert!(token.starts_with(GRAPH_TOKEN_PREFIX));
    assert!(token[GRAPH_TOKEN_PREFIX.len()..]
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
    let reference = GraphEvidenceRefV1::new(token).unwrap();
    let decoded = decode_graph_evidence_reference(&reference, &key).unwrap();
    assert_eq!(decoded.target_index, 7);
    assert_eq!(decoded.artifact_cursor, 11);
    assert_eq!(decoded.terminal_edge_cursor, 19);
}

#[test]
fn graph_selector_domains_are_not_interchangeable() {
    let key = [0x5a; 32];
    let nonce = [0x24; 16];
    let payload = [0x17; GRAPH_SELECTOR_BYTES];
    let graph_ciphertext = protect_graph_selector(&key, &nonce, &payload);
    let mut input = Vec::from(nonce);
    input.extend_from_slice(&0_u32.to_be_bytes());
    let wrong_domain_mask = frozen_read::hmac_sha256(&key, GRAPH_MAC_DOMAIN, &input);
    let wrong_first_block = payload[..32]
        .iter()
        .zip(wrong_domain_mask)
        .map(|(byte, mask)| byte ^ mask)
        .collect::<Vec<_>>();
    assert_ne!(&graph_ciphertext[..32], wrong_first_block.as_slice());
}

#[cfg(feature = "test-hooks")]
#[test]
fn graph_nonce_generation_executes_no_sql_statement() {
    // Raw connections must share Engine's configure-before-open ordering.
    crate::configure_runtime_for_open().unwrap();
    let mut connection = Connection::open_in_memory().unwrap();
    NONCE_SQL_STATEMENTS.store(0, Ordering::SeqCst);
    connection
        .trace_v2(rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT, Some(count_nonce_sql))
        .expect("install the nonce SQL statement counter");
    let nonce = random_nonce().unwrap();
    connection
        .trace_v2(
            rusqlite::trace::TraceEventCodes::empty(),
            None::<fn(rusqlite::trace::TraceEvent<'_>)>,
        )
        .expect("clear the nonce SQL statement counter");
    assert_ne!(nonce, [0; 16]);
    assert_eq!(NONCE_SQL_STATEMENTS.load(Ordering::SeqCst), 0);
}

#[test]
fn graph_selector_historical_flag_accepts_0_and_1_and_rejects_invalid_values() {
    let key = [0x47; 32];
    let nonce = [0x29; 16];
    let build = |flag: u8| {
        let mut selector = [0_u8; GRAPH_SELECTOR_BYTES];
        selector[0..4].copy_from_slice(&1_u32.to_be_bytes());
        selector[4] = 0;
        selector[5] = EvidenceArtifactClassV1::Node.tag();
        selector[6] = 0;
        selector[7] = flag;
        selector[8..12].copy_from_slice(&0_u32.to_be_bytes());
        selector[12..20].copy_from_slice(&11_u64.to_be_bytes());
        selector[20..28].copy_from_slice(&11_u64.to_be_bytes());
        selector[28..36].copy_from_slice(&12_u64.to_be_bytes());
        selector[36..44].copy_from_slice(&100_i64.to_be_bytes());
        let token = frame_graph_selector(&key, &nonce, &selector);
        assert_eq!(token.len(), GRAPH_TOKEN_BYTES);
        GraphEvidenceRefV1::new(token).unwrap()
    };

    let cur = decode_graph_evidence_reference(&build(0), &key).unwrap();
    assert!(!cur.historical);
    let hist = decode_graph_evidence_reference(&build(1), &key).unwrap();
    assert!(hist.historical);

    for invalid in [2_u8, 3, 127, 255] {
        let result = decode_graph_evidence_reference(&build(invalid), &key);
        assert!(
            matches!(
                result,
                Err(EngineError::Evidence(EvidenceErrorV1 {
                    reason: EvidenceErrorReasonV1::EvidenceUnavailable,
                    ..
                }))
            ),
            "payload[7]={invalid} must fail closed with EvidenceUnavailable"
        );
    }

    // Flipping the ciphertext hex nibble corresponding to payload[7] fails MAC check.
    let mut tampered = build(1).as_str().as_bytes().to_vec();
    let idx = GRAPH_TOKEN_PREFIX.len() + 32 + 14;
    tampered[idx] = if tampered[idx] == b'0' { b'1' } else { b'0' };
    let tampered_ref = GraphEvidenceRefV1::new(String::from_utf8(tampered).unwrap()).unwrap();
    let result = decode_graph_evidence_reference(&tampered_ref, &key);
    assert!(matches!(
        result,
        Err(EngineError::Evidence(EvidenceErrorV1 {
            reason: EvidenceErrorReasonV1::EvidenceUnavailable,
            ..
        }))
    ));
}

#[test]
fn historical_request_commitment_binds_as_of_cursor_and_epoch_distinctly() {
    let key = [0x88; 32];
    let authority = GraphEvidenceAuthority {
        database_commitment: [1; 32],
        context_commitment: [2; 32],
        key: key.to_vec(),
    };
    let c_none = graph_request_commitment(&authority, b"req-base");
    let c_cur_1 =
        graph_request_commitment(&authority, b"req-base\x02\x00\x00\x00\x00\x00\x00\x00\x01");
    let c_cur_2 =
        graph_request_commitment(&authority, b"req-base\x02\x00\x00\x00\x00\x00\x00\x00\x02");
    let c_epoch =
        graph_request_commitment(&authority, b"req-base\x03\x00\x00\x00\x00\x00\x00\x00\x01");
    assert_ne!(c_none, c_cur_1);
    assert_ne!(c_cur_1, c_cur_2);
    assert_ne!(c_cur_1, c_epoch);
    assert_ne!(c_none, c_epoch);
}
