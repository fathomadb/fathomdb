use super::*;

// ===== Test hooks =====================================================

/// EU-6 — test-hooks-gated vector write seam. Lets TS tests exercise
/// the 0.5/§7 mean-vec pin transition end-to-end through the binding
/// (the public TS surface does not yet expose typed vector writes; that
/// is its own multi-slice campaign). Compiled out of release npm builds
/// by the `test-hooks` cfg. Kept in a separate `#[napi] impl` block
/// because napi-derive's per-method `#[cfg]` gating inside the
/// production-surface impl block does not compose with the impl-level
/// `#[napi]` glue table.
#[cfg(any(test, feature = "test-hooks"))]
#[napi]
impl Engine {
    #[napi]
    pub fn requested_engine_config_for_test(&self) -> Result<EngineConfig> {
        let engine = Arc::clone(&self.inner);
        call_engine_sync(move || {
            let requested = engine.config();
            Ok(EngineConfig {
                embedder_pool_size: requested.embedder_pool_size.map(|value| value as f64),
                scheduler_runtime_threads: requested
                    .scheduler_runtime_threads
                    .map(|value| value as f64),
                provenance_row_cap: requested.provenance_row_cap.map(|value| value as f64),
                embedder_call_timeout_ms: requested
                    .embedder_call_timeout_ms
                    .map(|value| value as f64),
                slow_threshold_ms: requested.slow_threshold_ms.map(|value| value as f64),
            })
        })
    }

    #[napi]
    pub async fn configure_vector_kind_for_test(&self, kind: String) -> Result<()> {
        validate_ffi_string_napi(&kind)?;
        let engine = Arc::clone(&self.inner);
        call_engine(move || engine.configure_vector_kind_for_test(&kind)).await
    }

    #[napi]
    pub async fn write_vector_for_test(&self, kind: String, text: String) -> Result<()> {
        validate_ffi_string_napi(&kind)?;
        validate_ffi_string_napi(&text)?;
        let engine = Arc::clone(&self.inner);
        call_engine(move || engine.write_vector_for_test(&kind, &text).map(|_| ())).await
    }
}

#[cfg(feature = "test-hooks")]
#[napi]
impl Engine {
    /// Return private managed-connection facts for installed-binding tests.
    #[napi]
    pub async fn binding_connection_inventory_for_test(&self) -> Result<String> {
        let engine = Arc::clone(&self.inner);
        call_engine(move || engine.binding_connection_inventory_for_test()).await
    }
}

#[cfg(all(any(test, feature = "test-hooks"), any(debug_assertions, feature = "test-hooks")))]
#[napi]
impl Engine {
    #[napi]
    pub async fn set_legacy_projection_search_subobjects_for_test(
        &self,
        name: String,
    ) -> Result<()> {
        validate_ffi_string_napi(&name)?;
        let engine = Arc::clone(&self.inner);
        call_engine(move || engine.set_legacy_projection_search_subobjects_for_test(&name)).await
    }
}

/// AC-067 force-panic probe. Gated by `cfg(any(test, feature =
/// "test-hooks"))` so release npm builds without the feature flag do
/// not expose it.
#[cfg(any(test, feature = "test-hooks"))]
#[napi(js_name = "forcePanicForTest")]
pub fn force_panic_for_test() -> Result<()> {
    call_panicking_engine_for_test()
}

/// AC-067 sync-path probe: exercises [`call_engine_sync`] so the
/// TS-side test asserts that panics on a sync `#[napi]` accessor land
/// as `FathomDbPanicError` (code `FDB_PANIC`) too, not just the async
/// path covered by [`force_panic_for_test`].
#[cfg(any(test, feature = "test-hooks"))]
#[napi(js_name = "forcePanicInAccessorForTest")]
pub fn force_panic_in_accessor_for_test() -> Result<()> {
    call_engine_sync(|| -> Result<()> {
        panic!("force_panic_in_accessor_for_test: AC-067 sync probe");
    })
}

#[cfg(any(test, feature = "test-hooks"))]
pub(crate) fn call_panicking_engine_for_test() -> Result<()> {
    // Run the panic through the same catch_unwind path that real
    // engine calls use so the TS-side test exercises the production
    // panic-translation seam.
    let join_result = std::panic::catch_unwind(AssertUnwindSafe(
        || -> std::result::Result<(), RustEngineError> {
            panic!("force_panic_for_test: AC-067 probe");
        },
    ));
    match join_result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(err)) => Err(engine_error_to_napi(err)),
        Err(_) => Err(panic_error()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn rewrite_schema_header(path: &std::path::Path, version: u32) {
        let mut bytes = std::fs::read(path).unwrap();
        bytes[60..64].copy_from_slice(&version.to_be_bytes());
        std::fs::write(path, bytes).unwrap();
    }

    fn derived_edge_actuation_request() -> JsonValue {
        json!({
            "schemaVersion": 1,
            "operationId": "slice35-napi-edge",
            "operations": [{
                "type": "put_derived_edge",
                "record": {
                    "kind": "supports",
                    "from": "source",
                    "to": "target",
                    "sourceId": "source\u{0}bucket",
                    "logicalId": "edge-1",
                    "body": "edge λ",
                    "tValid": -7,
                    "tInvalid": null,
                    "provenance": {
                        "schemaVersion": 1,
                        "role": "derived",
                        "artifactRevisionId": "edge-r1",
                        "sourceVersionId": "source-v1",
                        "sourceRevisionId": "source-r1",
                        "sourceLocator": { "kind": "whole_body" },
                        "canonicalSourceHash": {
                            "algorithm": "sha256",
                            "digestHex": "0000000000000000000000000000000000000000000000000000000000000000"
                        }
                    }
                }
            }]
        })
    }

    #[test]
    fn derived_edge_actuation_translation_preserves_current_v1_shape() {
        let translated = translate_actuation_request(&derived_edge_actuation_request()).unwrap();
        let ActuationOperationV1::PutDerivedEdge(edge) = &translated.operations[0] else {
            panic!("derived edge discriminator translated to a different variant")
        };
        assert_eq!(edge.source_id.as_str().as_bytes(), b"source\0bucket");
        assert_eq!(edge.body.as_deref(), Some("edge λ"));
        assert_eq!(edge.t_valid, Some(-7));
    }

    fn assert_actuation_input_error(request: &JsonValue, reason: &str, path: &str) {
        let error = translate_actuation_request(request).unwrap_err();
        let envelope: JsonValue = serde_json::from_str(&error.reason).unwrap();
        assert_eq!(envelope["payload"]["reason"], reason);
        assert_eq!(envelope["payload"]["fieldPath"], path);
    }

    fn malformed_edge_actuation_request() -> JsonValue {
        let mut request = derived_edge_actuation_request();
        request["operations"][0]["record"]["zUnknown"] = json!(true);
        request
    }

    #[test]
    fn malformed_edge_preserves_every_earlier_top_level_precedence_family() {
        let mut request = malformed_edge_actuation_request();
        request["schemaVersion"] = json!(2);
        assert_actuation_input_error(&request, "unsupported_schema_version", "/schemaVersion");

        let mut request = malformed_edge_actuation_request();
        request["aUnknown"] = json!(true);
        assert_actuation_input_error(&request, "unknown_field", "/aUnknown");

        let mut request = malformed_edge_actuation_request();
        request.as_object_mut().unwrap().remove("operationId");
        assert_actuation_input_error(&request, "field_missing", "/operationId");

        let mut request = malformed_edge_actuation_request();
        request["operationId"] = json!(true);
        assert_actuation_input_error(&request, "field_type_invalid", "/operationId");

        let mut request = malformed_edge_actuation_request();
        request["decisionPolicyId"] = json!(true);
        assert_actuation_input_error(&request, "field_type_invalid", "/decisionPolicyId");

        let mut request = malformed_edge_actuation_request();
        request["expectedWriteBoundary"] = json!(true);
        assert_actuation_input_error(&request, "field_type_invalid", "/expectedWriteBoundary");

        let mut request = malformed_edge_actuation_request();
        request.as_object_mut().unwrap().remove("operations");
        assert_actuation_input_error(&request, "field_missing", "/operations");

        let mut request = malformed_edge_actuation_request();
        request["operations"] = json!({});
        assert_actuation_input_error(&request, "field_type_invalid", "/operations");

        let mut request = malformed_edge_actuation_request();
        request["operationId"] = json!("bad id");
        assert_actuation_input_error(&request, "operation_id_invalid", "/operationId");

        let mut request = malformed_edge_actuation_request();
        request["decisionPolicyId"] = json!("bad id");
        assert_actuation_input_error(&request, "decision_policy_id_invalid", "/decisionPolicyId");

        let mut request = malformed_edge_actuation_request();
        let operation = request["operations"][0].clone();
        request["operations"] = JsonValue::Array(vec![operation; 129]);
        assert_actuation_input_error(&request, "operation_count_invalid", "/operations");

        assert_actuation_input_error(
            &malformed_edge_actuation_request(),
            "unknown_field",
            "/operations/0/record/zUnknown",
        );
    }

    proptest! {
        #[test]
        fn mutation_projection_cursor_canonical_decimal_round_trips(write_cursor in 1_u64..) {
            let request = json!({
                "schemaVersion": 1,
                "operationId": "slice40-property",
                "writeCursor": write_cursor.to_string(),
                "expectedGenerationId": "pgen1:000102030405060708090a0b0c0d0e0f",
            });
            let translated = translate_mutation_projection_status_request(&request).unwrap();
            prop_assert_eq!(translated.write_cursor, write_cursor);
        }
    }

    #[test]
    fn mutation_projection_request_preserves_u64_max() {
        let request = json!({
            "schemaVersion": 1,
            "operationId": "slice40-max",
            "writeCursor": u64::MAX.to_string(),
            "expectedGenerationId": "pgen1:000102030405060708090a0b0c0d0e0f",
        });
        let translated = translate_mutation_projection_status_request(&request).unwrap();
        assert_eq!(translated.write_cursor, u64::MAX);
    }

    #[test]
    fn mutation_projection_request_rejects_noncanonical_cursor() {
        let request = json!({
            "schemaVersion": 1,
            "operationId": "slice40-zero",
            "writeCursor": "00",
            "expectedGenerationId": "pgen1:000102030405060708090a0b0c0d0e0f",
        });
        let error = translate_mutation_projection_status_request(&request).unwrap_err();
        let envelope: JsonValue = serde_json::from_str(&error.reason).unwrap();
        assert_eq!(envelope["payload"]["reason"], "invalid_write_cursor");
        assert_eq!(envelope["payload"]["fieldPath"], "/writeCursor");
    }

    #[test]
    fn mutation_projection_request_checks_schema_before_unknown_fields() {
        let request = json!({"schemaVersion": 2, "zzz": true});
        let error = translate_mutation_projection_status_request(&request).unwrap_err();
        let envelope: JsonValue = serde_json::from_str(&error.reason).unwrap();
        assert_eq!(envelope["payload"]["reason"], "unsupported_schema_version");
        assert_eq!(envelope["payload"]["fieldPath"], "/schemaVersion");
    }

    #[test]
    fn validate_ffi_string_accepts_plain_ascii() {
        assert!(validate_ffi_string("hello").is_ok());
    }

    #[test]
    fn validate_ffi_string_accepts_non_ascii_utf8() {
        assert!(validate_ffi_string("héllo 🦀 文字").is_ok());
    }

    #[test]
    fn validate_ffi_string_rejects_embedded_nul() {
        let err = validate_ffi_string("a\0b").unwrap_err();
        assert!(err.contains("NUL"), "expected NUL diagnostic, got {err:?}");
    }

    #[test]
    fn validate_ffi_string_accepts_replacement_codepoint() {
        // U+FFFD is the only "high unicode" codepoint a Rust &str can
        // hold near the surrogate range; the codepoints U+D800..U+DFFF
        // themselves cannot appear in valid UTF-8, so the runtime
        // guard is exercised when JS surrogates round-trip through
        // napi-rs string conversion (covered by ffi-safety.test.ts).
        assert!(validate_ffi_string("\u{FFFD}").is_ok());
    }

    #[test]
    fn embed_device_policy_open_error_uses_a_typed_napi_envelope() {
        let error = engine_open_error_to_napi(EngineOpenError::EmbedDevicePolicy(
            fathomdb_embedder::EmbedDevicePolicyError::Resolution(
                fathomdb_embedder::DeviceResolutionError::CudaNotCompiled { ordinal: 2 },
            ),
        ));
        let envelope: JsonValue = serde_json::from_str(&error.reason).expect("typed envelope");

        assert_eq!(envelope["code"], "FDB_EMBED_DEVICE_POLICY");
        assert_eq!(envelope["payload"]["kind"], "cuda_not_compiled");
        assert_eq!(envelope["payload"]["ordinal"], 2);
    }

    #[test]
    fn napi_open_maps_schema_33_refusal_to_the_typed_envelope() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("schema-33.sqlite");
        RustEngine::open(&path).unwrap().engine.close().unwrap();
        rewrite_schema_header(&path, 33);

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let error = match runtime.block_on(Engine::open(path.to_string_lossy().into_owned(), None))
        {
            Ok(_) => panic!("N-API open must refuse schema 33"),
            Err(error) => error,
        };
        let envelope: JsonValue = serde_json::from_str(&error.reason).unwrap();
        assert_eq!(envelope["code"], CODE_INCOMPATIBLE_SCHEMA_VERSION);
        assert_eq!(envelope["payload"]["seen"], 33);
        assert_eq!(envelope["payload"]["supported"], 34);
    }

    #[test]
    fn reranker_device_policy_open_error_uses_a_typed_napi_envelope() {
        let error = engine_open_error_to_napi(EngineOpenError::RerankerDevicePolicy(
            fathomdb_embedder::RerankerDevicePolicyError::Resolution(
                fathomdb_embedder::RerankerDeviceResolutionError::CudaNotCompiled { ordinal: 2 },
            ),
        ));
        let envelope: JsonValue = serde_json::from_str(&error.reason).expect("typed envelope");
        assert_eq!(envelope["code"], "FDB_RERANKER_DEVICE_POLICY");
        assert_eq!(envelope["payload"]["kind"], "cuda_not_compiled");
    }

    /// 0.8.28 pool study (ruling 15 as amended): a reranker forward that
    /// exhausts the private pool surfaces with the pool-exhaustion envelope,
    /// as the embedder's does.
    #[cfg(feature = "tegra-pool-experiment")]
    #[test]
    fn reranker_pool_exhaustion_uses_the_pool_exhausted_envelope() {
        let error = engine_error_to_napi(RustEngineError::RerankerDevicePolicy(
            fathomdb_embedder::RerankerDevicePolicyError::CudaPoolExhausted {
                ordinal: 0,
                max_size_bytes: 3 << 30,
            },
        ));
        let envelope: JsonValue = serde_json::from_str(&error.reason).expect("typed envelope");
        assert_eq!(envelope["code"], "FDB_CUDA_POOL_EXHAUSTED");
        assert_eq!(envelope["payload"]["kind"], "cuda_pool_exhausted");
        assert_eq!(envelope["payload"]["maxSizeBytes"], 3_u64 << 30);
    }

    #[test]
    fn reranker_device_policy_query_error_uses_the_same_typed_napi_envelope() {
        let error = engine_error_to_napi(RustEngineError::RerankerDevicePolicy(
            fathomdb_embedder::RerankerDevicePolicyError::Resolution(
                fathomdb_embedder::RerankerDeviceResolutionError::ForcedCudaUnavailable {
                    ordinal: 1,
                    reason: fathomdb_embedder::RerankerDeviceResolutionReason::CudaProbeFailed,
                },
            ),
        ));
        let envelope: JsonValue = serde_json::from_str(&error.reason).expect("typed envelope");
        assert_eq!(envelope["code"], "FDB_RERANKER_DEVICE_POLICY");
        assert_eq!(envelope["payload"]["ordinal"], 1);
    }

    #[test]
    fn open_report_preserves_caller_device_resolution() {
        let directory = tempfile::tempdir().expect("temporary database directory");
        let resolution = fathomdb_embedder::DeviceResolution {
            requested_policy: fathomdb_embedder::EmbedDevicePolicy::Cuda(3),
            cuda_compiled: true,
            effective_device: fathomdb_embedder::EffectiveEmbedDevice::Cuda(
                fathomdb_embedder::CudaDeviceInfo {
                    ordinal: 3,
                    uuid: Some("GPU-test".to_string()),
                    name: Some("test CUDA".to_string()),
                    driver_version: Some("555.42".to_string()),
                    compute_capability: Some("8.6".to_string()),
                    cuda_toolkit_version: Some("12.8".to_string()),
                },
            ),
            visible_cuda_devices: vec![fathomdb_embedder::CudaVisibleDevice {
                visible_ordinal: 3,
                uuid: "GPU-test".to_string(),
                name: "test CUDA".to_string(),
                compute_capability: Some("8.6".to_string()),
            }],
            selected_cuda_uuid: Some("GPU-test".to_string()),
            reason: None,
        };
        let opened = RustEngine::open_with_choice(
            directory.path().join("napi-device-resolution.sqlite"),
            EmbedderChoice::CallerWithDeviceResolution {
                embedder: Arc::new(fathomdb_embedder::NoopEmbedder::default()),
                device_resolution: resolution,
            },
        )
        .expect("caller resolution opens");

        let report = OpenReport::from_rust(&opened.report);
        let resolution = report
            .embedder_device_resolution
            .expect("caller resolution must reach the N-API open report");
        assert_eq!(resolution.requested_policy, "cuda:3");
        assert!(resolution.cuda_compiled);
        assert_eq!(resolution.effective_device.kind, "cuda");
        let cuda = resolution
            .effective_device
            .cuda_device
            .expect("CUDA selection must retain its safe provider facts");
        assert_eq!(cuda.ordinal, 3);
        assert_eq!(cuda.uuid.as_deref(), Some("GPU-test"));
        assert_eq!(cuda.name.as_deref(), Some("test CUDA"));
        assert_eq!(cuda.driver_version.as_deref(), Some("555.42"));
        assert_eq!(cuda.compute_capability.as_deref(), Some("8.6"));
        assert_eq!(cuda.cuda_toolkit_version.as_deref(), Some("12.8"));
        assert_eq!(resolution.visible_cuda_devices.len(), 1);
        assert_eq!(resolution.visible_cuda_devices[0].visible_ordinal, 3);
        assert_eq!(resolution.selected_cuda_uuid.as_deref(), Some("GPU-test"));
        assert_eq!(resolution.reason, None);
        // D-80.6-6 — a CUDA *policy outcome* is not a measurement. The witness
        // stays absent unless one was actually taken.
        assert!(report.embedder_gpu_allocation_witness.is_none());
    }

    /// 0.8.23 Slice 80.6 (D-80.6-6, R80-13) — the witness crosses the N-API
    /// boundary with every number the verdict used still present, so a JS
    /// consumer can re-derive the verdict instead of trusting it.
    #[test]
    fn gpu_allocation_witness_crosses_the_napi_boundary_intact() {
        let witness = RustGpuAllocationWitness {
            device_ordinal_requested: 0,
            device_ordinal_actual: 0,
            device_uuid: "GPU-11111111-2222-3333-4444-555555555555".to_string(),
            device_name: "Orin".to_string(),
            compute_capability: "8.7".to_string(),
            free_before_bytes: 40_000_000_000,
            free_after_bytes: 39_856_635_904,
            total_bytes: 65_000_000_000,
            delta_bytes: 143_364_096,
            delta_floor_bytes: 67_108_864,
            control_allocation_request_bytes: 1_073_741_824,
            control_block_count: 8,
            control_free_before_bytes: 42_000_000_000,
            control_free_after_bytes: 40_800_000_000,
            control_delta_bytes: 1_200_000_000,
            embedded_vector_dim: 384,
        };

        let mapped = GpuAllocationWitness::from_rust(&witness);

        assert_eq!(mapped.schema, "fathomdb.tegra-gpu-allocation-witness/v1");
        assert!(mapped.sole_gpu_consumer_precondition.contains("sole GPU consumer"));
        assert_eq!(mapped.device_ordinal_requested, 0);
        assert_eq!(mapped.device_ordinal_actual, 0);
        assert_eq!(mapped.device_uuid, "GPU-11111111-2222-3333-4444-555555555555");
        assert_eq!(mapped.device_name, "Orin");
        assert_eq!(mapped.compute_capability, "8.7");
        assert_eq!(mapped.free_before_bytes, 40_000_000_000);
        assert_eq!(mapped.free_after_bytes, 39_856_635_904);
        assert_eq!(mapped.total_bytes, 65_000_000_000);
        assert_eq!(mapped.delta_bytes, 143_364_096);
        assert_eq!(mapped.delta_floor_bytes, 67_108_864);
        assert_eq!(mapped.control_allocation_request_bytes, 1_073_741_824);
        assert_eq!(mapped.control_block_count, 8);
        assert_eq!(mapped.control_free_before_bytes, 42_000_000_000);
        assert_eq!(mapped.control_free_after_bytes, 40_800_000_000);
        assert_eq!(mapped.control_delta_bytes, 1_200_000_000);
        assert_eq!(mapped.embedded_vector_dim, 384);
        // The verdict is re-derivable from the mapped record alone (R80-13).
        assert_eq!(mapped.free_before_bytes - mapped.free_after_bytes, mapped.delta_bytes);
        assert!(mapped.delta_bytes >= mapped.delta_floor_bytes);
        assert!(
            mapped.control_free_before_bytes - mapped.control_free_after_bytes
                >= mapped.control_allocation_request_bytes
        );
    }
}
