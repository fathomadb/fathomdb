use super::*;

/// Opaque, dev-only reader-snapshot rendezvous for the Slice 65 installed
/// binding control. It is compiled only with `test-hooks`, never shipped.
#[cfg(feature = "test-hooks")]
#[pyclass(module = "fathomdb._fathomdb", name = "_WalSnapshotPause")]
pub(super) struct PyWalSnapshotPause {
    pub(super) snapshot_ready: Arc<Barrier>,
    pub(super) release: Arc<Barrier>,
    pub(super) reader_autocommit: Option<Arc<AtomicBool>>,
    pub(super) reader_native_state: Option<Arc<Mutex<Option<String>>>>,
}

#[cfg(feature = "test-hooks")]
#[pymethods]
impl PyWalSnapshotPause {
    fn wait_snapshot_ready(&self, py: Python<'_>) {
        let snapshot_ready = Arc::clone(&self.snapshot_ready);
        py.detach(move || snapshot_ready.wait());
    }

    fn release(&self, py: Python<'_>) {
        let release = Arc::clone(&self.release);
        py.detach(move || release.wait());
    }

    fn reader_connection_autocommit_for_test(&self) -> bool {
        self.reader_autocommit.as_ref().is_some_and(|value| value.load(Ordering::Acquire))
    }

    fn reader_native_state_for_test(&self) -> PyResult<String> {
        self.reader_native_state
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("snapshot pause has no native state recorder"))?
            .lock()
            .map_err(|_| PyValueError::new_err("snapshot native state recorder is unavailable"))?
            .clone()
            .ok_or_else(|| PyValueError::new_err("snapshot native state was not recorded"))
    }
}

// ===== Test hooks =====================================================

/// AC-067 force-panic probe. Gated by `cfg(any(test, feature =
/// "test-hooks"))` so release wheels built with `--no-default-features`
/// do not expose it.
#[cfg(any(test, feature = "test-hooks"))]
#[pyfunction]
pub(super) fn force_panic_for_test() -> PyResult<()> {
    panic!("force_panic_for_test: AC-067 probe");
}

/// Take one private native/Rusqlite WAL checkpoint sample for Slice 65's
/// disposable fresh-child diagnostic. This hook is absent from shipped wheels.
#[cfg(feature = "test-hooks")]
#[pyfunction(name = "_native_raw_wal_checkpoint_for_test")]
pub(super) fn native_raw_wal_checkpoint_for_test(
    py: Python<'_>,
    path: String,
) -> PyResult<(bool, u32, u32)> {
    validate_ffi_string_py(&path)?;
    call_engine(py, move || RustEngine::native_raw_wal_checkpoint_for_test(&path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_config_from_py_preserves_each_requested_field_and_omission() {
        Python::initialize();
        Python::attach(|py| {
            assert_eq!(engine_config_from_py(None).unwrap(), RustEngineConfig::default());
            assert_eq!(
                engine_config_from_py(Some(&PyDict::new(py))).unwrap(),
                RustEngineConfig::default()
            );

            let requested = PyDict::new(py);
            requested.set_item("scheduler_runtime_threads", 4).unwrap();
            requested.set_item("embedder_pool_size", 3).unwrap();
            requested.set_item("embedder_call_timeout_ms", 2_001).unwrap();
            requested.set_item("provenance_row_cap", 0).unwrap();
            requested.set_item("slow_threshold_ms", 7).unwrap();
            assert_eq!(
                engine_config_from_py(Some(&requested)).unwrap(),
                RustEngineConfig {
                    scheduler_runtime_threads: Some(4),
                    embedder_pool_size: Some(3),
                    embedder_call_timeout_ms: Some(2_001),
                    provenance_row_cap: Some(0),
                    slow_threshold_ms: Some(7),
                }
            );
        });
    }

    fn rewrite_schema_header(path: &std::path::Path, version: u32) {
        let mut bytes = std::fs::read(path).unwrap();
        bytes[60..64].copy_from_slice(&version.to_be_bytes());
        std::fs::write(path, bytes).unwrap();
    }

    fn derived_edge_actuation_json() -> &'static str {
        r#"{
          "schema_version": 1,
          "operation_id": "slice35-py-edge",
          "operations": [{
            "type": "put_derived_edge",
            "record": {
              "kind": "supports", "from": "source", "to": "target",
              "source_id": "source\u0000bucket", "logical_id": "edge-1",
              "body": "edge λ", "t_valid": -7, "t_invalid": null,
              "provenance": {
                "schema_version": 1, "role": "derived",
                "artifact_revision_id": "edge-r1", "source_version_id": "source-v1",
                "source_revision_id": "source-r1",
                "source_locator": {"kind": "whole_body"},
                "canonical_source_hash": {
                  "algorithm": "sha256",
                  "digest_hex": "0000000000000000000000000000000000000000000000000000000000000000"
                }
              }
            }
          }]
        }"#
    }

    #[test]
    fn derived_edge_actuation_translation_preserves_current_v1_shape() {
        Python::initialize();
        Python::attach(|py| {
            let json = PyModule::import(py, "json").unwrap();
            let request = json.call_method1("loads", (derived_edge_actuation_json(),)).unwrap();
            let translated = translate_actuation_request(&request).unwrap();
            let ActuationOperationV1::PutDerivedEdge(edge) = &translated.operations[0] else {
                panic!("derived edge discriminator translated to a different variant")
            };
            assert_eq!(edge.source_id.as_str().as_bytes(), b"source\0bucket");
            assert_eq!(edge.body.as_deref(), Some("edge λ"));
            assert_eq!(edge.t_valid, Some(-7));
        });
    }

    #[test]
    fn malformed_edge_preserves_every_earlier_top_level_precedence_family() {
        Python::initialize();
        Python::attach(|py| {
            let json = PyModule::import(py, "json").unwrap();
            let mut malformed: serde_json::Value =
                serde_json::from_str(derived_edge_actuation_json()).unwrap();
            malformed["operations"][0]["record"]["z_unknown"] = serde_json::json!(true);
            let assert_error = |value: &serde_json::Value, reason: &str, path: &str| {
                let request = json.call_method1("loads", (value.to_string(),)).unwrap();
                let error = translate_actuation_request(&request).unwrap_err();
                assert_eq!(
                    error.value(py).getattr("reason").unwrap().extract::<String>().unwrap(),
                    reason
                );
                assert_eq!(
                    error.value(py).getattr("field_path").unwrap().extract::<String>().unwrap(),
                    path
                );
            };

            let mut request = malformed.clone();
            request["schema_version"] = serde_json::json!(2);
            assert_error(&request, "unsupported_schema_version", "/schemaVersion");
            let mut request = malformed.clone();
            request["a_unknown"] = serde_json::json!(true);
            assert_error(&request, "unknown_field", "/aUnknown");
            let mut request = malformed.clone();
            request.as_object_mut().unwrap().remove("operation_id");
            assert_error(&request, "field_missing", "/operationId");
            let mut request = malformed.clone();
            request["operation_id"] = serde_json::json!(true);
            assert_error(&request, "field_type_invalid", "/operationId");
            let mut request = malformed.clone();
            request["decision_policy_id"] = serde_json::json!(true);
            assert_error(&request, "field_type_invalid", "/decisionPolicyId");
            let mut request = malformed.clone();
            request["expected_write_boundary"] = serde_json::json!(true);
            assert_error(&request, "field_type_invalid", "/expectedWriteBoundary");
            let mut request = malformed.clone();
            request.as_object_mut().unwrap().remove("operations");
            assert_error(&request, "field_missing", "/operations");
            let mut request = malformed.clone();
            request["operations"] = serde_json::json!({});
            assert_error(&request, "field_type_invalid", "/operations");
            let mut request = malformed.clone();
            request["operation_id"] = serde_json::json!("bad id");
            assert_error(&request, "operation_id_invalid", "/operationId");
            let mut request = malformed.clone();
            request["decision_policy_id"] = serde_json::json!("bad id");
            assert_error(&request, "decision_policy_id_invalid", "/decisionPolicyId");
            let mut request = malformed.clone();
            let operation = request["operations"][0].clone();
            request["operations"] = serde_json::Value::Array(vec![operation; 129]);
            assert_error(&request, "operation_count_invalid", "/operations");
            assert_error(&malformed, "unknown_field", "/operations/0/record/zUnknown");
        });
    }

    #[test]
    fn mutation_projection_status_rejects_non_string_request_keys() {
        Python::initialize();
        Python::attach(|py| {
            let request = PyDict::new(py);
            request.set_item("schemaVersion", 1).unwrap();
            request.set_item(7, "ignored").unwrap();
            let error = strict_projection_generation_request_dict(&request).unwrap_err();
            assert!(error.is_instance_of::<ProjectionGenerationError>(py));
            assert_eq!(
                error.value(py).getattr("reason").unwrap().extract::<String>().unwrap(),
                "unknown_field"
            );
            assert_eq!(
                error.value(py).getattr("field_path").unwrap().extract::<String>().unwrap(),
                ""
            );
        });
    }

    #[test]
    fn mutation_projection_status_rejects_boolean_schema_version() {
        Python::initialize();
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join(format!("bool-schema{}", fathomdb_schema::SQLITE_SUFFIX));
        let opened = RustEngine::open(&path).unwrap();
        let engine = PyEngine {
            inner: Arc::new(opened.engine),
            open_report: Arc::new(opened.report),
            logging: logging_subscriber::LoggingSlot::new(),
        };
        Python::attach(|py| {
            let request = PyDict::new(py);
            request.set_item("schemaVersion", true).unwrap();
            request.set_item("operationId", "slice40-bool").unwrap();
            request.set_item("writeCursor", "1").unwrap();
            request
                .set_item("expectedGenerationId", "pgen1:000102030405060708090a0b0c0d0e0f")
                .unwrap();
            let error = match read_mutation_projection_status(py, &engine, &request) {
                Err(error) => error,
                Ok(_) => panic!("boolean schemaVersion must be rejected"),
            };
            assert!(error.is_instance_of::<ProjectionGenerationError>(py));
            assert_eq!(
                error.value(py).getattr("reason").unwrap().extract::<String>().unwrap(),
                "unsupported_schema_version"
            );
            assert_eq!(
                error.value(py).getattr("field_path").unwrap().extract::<String>().unwrap(),
                "/schemaVersion"
            );
        });
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
    fn validate_ffi_string_rejects_lone_surrogate() {
        // The surrogate codepoint U+D800 cannot appear in a Rust &str
        // (it is not valid UTF-8). The Rust-side helper exists for the
        // case where the Python layer feeds us the codepoint via an
        // alternate path; construct it through `char::from_u32`
        // unchecked... actually `char::from_u32` returns None for
        // surrogates. The exhaustive guard sits in Python; the Rust
        // helper documents the rule and remains a runtime check for
        // bytes-derived input.
        let valid_high_unicode = "\u{FFFD}";
        assert!(validate_ffi_string(valid_high_unicode).is_ok());
    }

    #[test]
    fn embed_device_policy_open_error_uses_a_typed_python_exception() {
        Python::initialize();
        Python::attach(|py| {
            let error = engine_open_error_to_py(EngineOpenError::EmbedDevicePolicy(
                fathomdb_embedder::EmbedDevicePolicyError::Resolution(
                    fathomdb_embedder::DeviceResolutionError::CudaNotCompiled { ordinal: 2 },
                ),
            ));

            assert!(error.is_instance_of::<EmbedDevicePolicyError>(py));
            let value = error.value(py);
            assert_eq!(
                value.getattr("kind").unwrap().extract::<String>().unwrap(),
                "cuda_not_compiled"
            );
            assert_eq!(value.getattr("ordinal").unwrap().extract::<usize>().unwrap(), 2);
        });
    }

    #[test]
    fn python_open_maps_schema_33_refusal_to_the_typed_exception() {
        Python::initialize();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("schema-33.sqlite");
        RustEngine::open(&path).unwrap().engine.close().unwrap();
        rewrite_schema_header(&path, 33);

        Python::attach(|py| {
            let error = match PyEngine::open(py, path.to_string_lossy().into_owned(), false, None) {
                Ok(_) => panic!("Python open must refuse schema 33"),
                Err(error) => error,
            };
            assert!(error.is_instance_of::<IncompatibleSchemaVersionError>(py));
            assert!(error.to_string().contains("schema version 33"));
            assert!(error.to_string().contains("supported version 34"));
        });
    }

    #[test]
    fn reranker_device_policy_open_error_uses_a_typed_python_exception() {
        Python::initialize();
        Python::attach(|py| {
            let error = engine_open_error_to_py(EngineOpenError::RerankerDevicePolicy(
                fathomdb_embedder::RerankerDevicePolicyError::Resolution(
                    fathomdb_embedder::RerankerDeviceResolutionError::CudaNotCompiled {
                        ordinal: 2,
                    },
                ),
            ));
            assert!(error.is_instance_of::<RerankerDevicePolicyError>(py));
            assert_eq!(
                error.value(py).getattr("kind").unwrap().extract::<String>().unwrap(),
                "cuda_not_compiled"
            );
        });
    }

    #[test]
    fn reranker_device_policy_query_error_uses_the_same_typed_python_exception() {
        Python::initialize();
        Python::attach(|py| {
            let error = engine_error_to_py(RustEngineError::RerankerDevicePolicy(
                fathomdb_embedder::RerankerDevicePolicyError::Resolution(
                    fathomdb_embedder::RerankerDeviceResolutionError::ForcedCudaUnavailable {
                        ordinal: 1,
                        reason: fathomdb_embedder::RerankerDeviceResolutionReason::CudaProbeFailed,
                    },
                ),
            ));
            assert!(error.is_instance_of::<RerankerDevicePolicyError>(py));
            assert_eq!(error.value(py).getattr("ordinal").unwrap().extract::<usize>().unwrap(), 1);
        });
    }

    #[test]
    fn open_report_preserves_caller_device_resolution() {
        Python::initialize();
        Python::attach(|py| {
            let directory = tempfile::tempdir().expect("temporary database directory");
            let resolution = fathomdb_embedder::DeviceResolution {
                requested_policy: fathomdb_embedder::EmbedDevicePolicy::Cuda(3),
                cuda_compiled: true,
                effective_device: fathomdb_embedder::EffectiveEmbedDevice::Cuda(
                    fathomdb_embedder::CudaDeviceInfo::new(
                        3,
                        Some("GPU-test".to_string()),
                        Some("test CUDA".to_string()),
                        Some("555.42".to_string()),
                        Some("8.6".to_string()),
                        Some("12.8".to_string()),
                    ),
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
                directory.path().join("python-device-resolution.sqlite"),
                EmbedderChoice::CallerWithDeviceResolution {
                    embedder: Arc::new(fathomdb_embedder::NoopEmbedder::default()),
                    device_resolution: resolution,
                },
            )
            .expect("caller resolution opens");

            let report = PyOpenReport::from_rust(py, &opened.report);
            let resolution = report
                .embedder_device_resolution
                .expect("caller resolution must reach the Python open report");
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
            // D-80.6-6 — a CUDA *policy outcome* is not a measurement. The
            // witness stays absent unless one was actually taken.
            assert!(report.embedder_gpu_allocation_witness.is_none());
        });
    }

    /// 0.8.23 Slice 80.6 (D-80.6-6, R80-13) — the witness crosses the PyO3
    /// boundary with every number the verdict used still present, so a Python
    /// consumer can re-derive the verdict instead of trusting it.
    #[test]
    fn gpu_allocation_witness_crosses_the_pyo3_boundary_intact() {
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

        let mapped = PyGpuAllocationWitness::from_rust(&witness);

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
        assert_eq!(
            i128::from(mapped.free_before_bytes) - i128::from(mapped.free_after_bytes),
            mapped.delta_bytes
        );
        assert!(mapped.delta_bytes >= i128::from(mapped.delta_floor_bytes));
        assert!(
            i128::from(mapped.control_free_before_bytes)
                - i128::from(mapped.control_free_after_bytes)
                >= i128::from(mapped.control_allocation_request_bytes)
        );
    }

    // fix-1 finding 2: the CLS-embedder singleton must NOT cache a failed load.
    // `cls_embedder_singleton` itself is `#[cfg(feature = "default-embedder")]`
    // and drives a real model load, so we test the caching contract through the
    // feature-agnostic helper it is built on.
    #[test]
    fn get_or_try_init_caches_only_on_success() {
        static CELL: std::sync::OnceLock<u32> = std::sync::OnceLock::new();

        // A failed init returns the real error and leaves the cell empty so the
        // next call retries (the exact regression finding 2 flags).
        let err = get_or_try_init::<u32, &str>(&CELL, || Err("transient")).unwrap_err();
        assert_eq!(err, "transient");
        assert!(CELL.get().is_none(), "a failed init must not be cached");

        // A subsequent success is stored and returned.
        let v = get_or_try_init::<u32, &str>(&CELL, || Ok(7)).unwrap();
        assert_eq!(*v, 7);

        // Once cached, the init closure is never run again.
        let v2 = get_or_try_init::<u32, &str>(&CELL, || panic!("must not re-init")).unwrap();
        assert_eq!(*v2, 7);
    }

    // 0.8.16 Slice 5 / F9 (codex §9 fix-1, FINDING 2) — the pyo3 `PerHitExplain`
    // mirror must copy the new `importance`/`confidence` fields from the engine
    // type, keeping Py symmetric with the N-API mirror; otherwise Python
    // `search_explained` callers cannot observe the F9 contribution (the 0.8.14
    // `embed_batch_cls` binding blind-spot). The engine `PerHitExplain` is
    // `#[non_exhaustive]` (no cross-crate literal), so the source value comes from
    // a REAL `search_explained` run (F9 reweight ON, graph arm ON). Runs under
    // `cargo test` (no maturin needed; `extension-module` is off for the workspace
    // test build).
    #[test]
    fn per_hit_explain_from_rust_copies_f9_importance_and_confidence() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join(format!("py_f9_explain{}", fathomdb_schema::SQLITE_SUFFIX));
        let opened = RustEngine::open(&path).expect("open");
        let engine = &opened.engine;
        let receipt = engine
            .write(&[
                PreparedWrite::Node {
                    kind: "doc".to_string(),
                    body: "zephyr anchor entity".to_string(),
                    source_id: SourceId::new("test:fixture").expect("test source id"),
                    logical_id: Some("zephyr".to_string()),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                },
                PreparedWrite::Node {
                    kind: "doc".to_string(),
                    body: "beta reachable payload node".to_string(),
                    source_id: SourceId::new("test:fixture").expect("test source id"),
                    logical_id: Some("beta".to_string()),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                },
                PreparedWrite::Edge {
                    kind: "link".to_string(),
                    from: "zephyr".to_string(),
                    to: "beta".to_string(),
                    source_id: SourceId::new("test:fixture").expect("test source id"),
                    logical_id: Some("e-zb".to_string()),
                    body: Some("collaboration record".to_string()),
                    t_valid: None,
                    t_invalid: None,
                    confidence: Some(0.90),
                    extractor_model_id: None,
                    temporal_fallback: None,
                },
            ])
            .expect("write");
        let beta_cursor = receipt.row_cursors[1];
        engine.write_node_importance(beta_cursor, 0.25).expect("set importance");
        engine.set_importance_reweight_enabled_for_test(true);

        let explained =
            engine.search_explained("zephyr", None, 0, true, 0.3, 0).expect("search_explained");
        let exp = explained.explanation.expect("explanation sidecar present");
        let entry = exp
            .per_hit
            .iter()
            .find(|p| p.id == beta_cursor)
            .expect("per_hit entry for the graph-reached beta node");
        assert_eq!(entry.importance, Some(0.25), "source explain carries node importance");
        assert_eq!(entry.confidence, Some(0.90), "source explain carries edge confidence");

        let mirror = PyPerHitExplain::from_rust(entry);
        assert_eq!(mirror.importance, Some(0.25), "importance must propagate to the pyo3 mirror");
        assert_eq!(mirror.confidence, Some(0.90), "confidence must propagate to the pyo3 mirror");

        opened.engine.close().unwrap();
    }

    // ----- 0.8.28 Slice 30 (R30-04): the three CUDA pool kinds ---------------

    fn attr<'py>(py: Python<'py>, error: &PyErr, name: &str) -> Bound<'py, PyAny> {
        error.value(py).getattr(name).unwrap()
    }

    fn assert_pool_exhausted(py: Python<'_>, error: &PyErr) {
        assert!(error.is_instance_of::<CudaPoolExhaustedError>(py), "{error}");
        assert!(error.is_instance_of::<EmbedderError>(py));
        assert_eq!(attr(py, error, "ordinal").extract::<usize>().unwrap(), 0);
        assert_eq!(attr(py, error, "max_size_bytes").extract::<u64>().unwrap(), 3 << 30);
        assert_eq!(attr(py, error, "message").extract::<String>().unwrap(), "oom");
    }

    fn assert_context_lost(py: Python<'_>, error: &PyErr) {
        assert!(error.is_instance_of::<CudaContextLostError>(py), "{error}");
        assert!(error.is_instance_of::<EmbedderError>(py));
        assert_eq!(attr(py, error, "recorded_context_id").extract::<u64>().unwrap(), u64::MAX);
        assert_eq!(attr(py, error, "current_context_id").extract::<Option<u64>>().unwrap(), None);
        assert_eq!(
            attr(py, error, "driver_error").extract::<String>().unwrap(),
            "CUDA_ERROR_CONTEXT_IS_DESTROYED"
        );
        assert_eq!(attr(py, error, "operation").extract::<String>().unwrap(), "forward");
    }

    fn assert_build_refused(py: Python<'_>, error: &PyErr) {
        assert!(error.is_instance_of::<CudaPrivateBuildRefusedError>(py), "{error}");
        assert!(error.is_instance_of::<EmbedderError>(py));
        assert_eq!(attr(py, error, "ordinal").extract::<usize>().unwrap(), 1);
        assert_eq!(attr(py, error, "message").extract::<String>().unwrap(), "refused");
    }

    fn api_exhausted() -> fathomdb_embedder_api::EmbedderError {
        fathomdb_embedder_api::EmbedderError::CudaPoolExhausted {
            ordinal: 0,
            max_size_bytes: 3 << 30,
            message: "oom".to_owned(),
        }
    }

    fn api_context_lost() -> fathomdb_embedder_api::EmbedderError {
        fathomdb_embedder_api::EmbedderError::CudaContextLost {
            recorded_context_id: u64::MAX,
            current_context_id: None,
            driver_error: "CUDA_ERROR_CONTEXT_IS_DESTROYED".to_owned(),
            operation: "forward".to_owned(),
        }
    }

    fn api_refused() -> fathomdb_embedder_api::EmbedderError {
        fathomdb_embedder_api::EmbedderError::CudaPrivateBuildRefused {
            ordinal: 1,
            message: "refused".to_owned(),
        }
    }

    fn policy_exhausted() -> fathomdb_embedder::RerankerDevicePolicyError {
        fathomdb_embedder::RerankerDevicePolicyError::CudaPoolExhausted {
            ordinal: 0,
            max_size_bytes: 3 << 30,
            message: "oom".to_owned(),
        }
    }

    fn policy_context_lost() -> fathomdb_embedder::RerankerDevicePolicyError {
        fathomdb_embedder::RerankerDevicePolicyError::CudaContextLost {
            recorded_context_id: u64::MAX,
            current_context_id: None,
            driver_error: "CUDA_ERROR_CONTEXT_IS_DESTROYED".to_owned(),
            operation: "forward".to_owned(),
        }
    }

    fn policy_refused() -> fathomdb_embedder::RerankerDevicePolicyError {
        fathomdb_embedder::RerankerDevicePolicyError::CudaPrivateBuildRefused {
            ordinal: 1,
            message: "refused".to_owned(),
        }
    }

    #[test]
    fn engine_cuda_kinds_raise_their_own_typed_exceptions() {
        Python::initialize();
        Python::attach(|py| {
            assert_pool_exhausted(
                py,
                &engine_error_to_py(RustEngineError::CudaPoolExhausted {
                    ordinal: 0,
                    max_size_bytes: 3 << 30,
                    message: "oom".to_owned(),
                }),
            );
            assert_context_lost(
                py,
                &engine_error_to_py(RustEngineError::CudaContextLost {
                    recorded_context_id: u64::MAX,
                    current_context_id: None,
                    driver_error: "CUDA_ERROR_CONTEXT_IS_DESTROYED".to_owned(),
                    operation: "forward".to_owned(),
                }),
            );
            assert_build_refused(
                py,
                &engine_error_to_py(RustEngineError::CudaPrivateBuildRefused {
                    ordinal: 1,
                    message: "refused".to_owned(),
                }),
            );
            let replaced = engine_error_to_py(RustEngineError::CudaContextLost {
                recorded_context_id: 1,
                current_context_id: Some(2),
                driver_error: "d".to_owned(),
                operation: "o".to_owned(),
            });
            assert_eq!(
                attr(py, &replaced, "current_context_id").extract::<Option<u64>>().unwrap(),
                Some(2)
            );
        });
    }

    #[test]
    fn engine_reranker_cuda_kinds_are_not_swallowed_by_the_policy_class() {
        Python::initialize();
        Python::attach(|py| {
            assert_pool_exhausted(
                py,
                &engine_error_to_py(RustEngineError::RerankerDevicePolicy(policy_exhausted())),
            );
            assert_context_lost(
                py,
                &engine_error_to_py(RustEngineError::RerankerDevicePolicy(policy_context_lost())),
            );
            assert_build_refused(
                py,
                &engine_error_to_py(RustEngineError::RerankerDevicePolicy(policy_refused())),
            );
        });
    }

    #[test]
    fn open_cuda_kinds_raise_their_own_typed_exceptions() {
        Python::initialize();
        Python::attach(|py| {
            assert_pool_exhausted(
                py,
                &engine_open_error_to_py(EngineOpenError::CudaPoolExhausted {
                    ordinal: 0,
                    max_size_bytes: 3 << 30,
                    message: "oom".to_owned(),
                }),
            );
            assert_context_lost(
                py,
                &engine_open_error_to_py(EngineOpenError::CudaContextLost {
                    recorded_context_id: u64::MAX,
                    current_context_id: None,
                    driver_error: "CUDA_ERROR_CONTEXT_IS_DESTROYED".to_owned(),
                    operation: "forward".to_owned(),
                }),
            );
            assert_build_refused(
                py,
                &engine_open_error_to_py(EngineOpenError::CudaPrivateBuildRefused {
                    ordinal: 1,
                    message: "refused".to_owned(),
                }),
            );
            assert_pool_exhausted(
                py,
                &engine_open_error_to_py(EngineOpenError::Embedder(api_exhausted())),
            );
            assert_context_lost(
                py,
                &engine_open_error_to_py(EngineOpenError::Embedder(api_context_lost())),
            );
            assert_build_refused(
                py,
                &engine_open_error_to_py(EngineOpenError::Embedder(api_refused())),
            );
            assert_pool_exhausted(
                py,
                &engine_open_error_to_py(EngineOpenError::RerankerDevicePolicy(policy_exhausted())),
            );
        });
    }

    #[test]
    fn embed_batch_cls_forward_cuda_kinds_are_typed() {
        Python::initialize();
        Python::attach(|py| {
            assert_pool_exhausted(py, &cls_forward_error_to_py(api_exhausted()));
            assert_context_lost(py, &cls_forward_error_to_py(api_context_lost()));
            assert_build_refused(py, &cls_forward_error_to_py(api_refused()));
            let other = cls_forward_error_to_py(fathomdb_embedder_api::EmbedderError::Failed {
                message: "boom".to_owned(),
            });
            assert!(other.is_instance_of::<EmbedderError>(py));
            assert!(!other.is_instance_of::<CudaPoolExhaustedError>(py));
        });
    }

    #[cfg(feature = "default-embedder")]
    #[test]
    fn embed_batch_cls_load_cuda_kinds_are_typed() {
        use fathomdb_embedder::loader::EmbedderLoadError;
        Python::initialize();
        Python::attach(|py| {
            assert_pool_exhausted(
                py,
                &cls_load_error_to_py(EmbedderLoadError::CudaPoolExhausted {
                    ordinal: 0,
                    max_size_bytes: 3 << 30,
                    message: "oom".to_owned(),
                }),
            );
            assert_context_lost(
                py,
                &cls_load_error_to_py(EmbedderLoadError::CudaContextLost {
                    recorded_context_id: u64::MAX,
                    current_context_id: None,
                    driver_error: "CUDA_ERROR_CONTEXT_IS_DESTROYED".to_owned(),
                    operation: "forward".to_owned(),
                }),
            );
            assert_build_refused(
                py,
                &cls_load_error_to_py(EmbedderLoadError::CudaPrivateBuildRefused {
                    ordinal: 1,
                    message: "refused".to_owned(),
                }),
            );
            let other = cls_load_error_to_py(EmbedderLoadError::DeviceInitialization {
                message: "no device".to_owned(),
            });
            assert!(other.is_instance_of::<EmbedderNotConfiguredError>(py));
        });
    }

    #[test]
    fn rerank_errors_keep_their_typed_classes() {
        use fathomdb_engine::RerankPassagesError;
        Python::initialize();
        Python::attach(|py| {
            assert_pool_exhausted(
                py,
                &rerank_passages_error_to_py(RerankPassagesError::Reranker(policy_exhausted())),
            );
            assert_context_lost(
                py,
                &rerank_passages_error_to_py(RerankPassagesError::Reranker(policy_context_lost())),
            );
            assert_build_refused(
                py,
                &rerank_passages_error_to_py(RerankPassagesError::Reranker(policy_refused())),
            );
            let policy = rerank_passages_error_to_py(RerankPassagesError::Reranker(
                fathomdb_embedder::RerankerDevicePolicyError::Resolution(
                    fathomdb_embedder::RerankerDeviceResolutionError::CudaNotCompiled {
                        ordinal: 0,
                    },
                ),
            ));
            assert!(policy.is_instance_of::<RerankerDevicePolicyError>(py));
            assert_eq!(attr(py, &policy, "kind").extract::<String>().unwrap(), "cuda_not_compiled");
            let invalid = rerank_passages_error_to_py(RerankPassagesError::WriteValidation {
                message: "non-finite".to_owned(),
            });
            assert!(invalid.is_instance_of::<WriteValidationError>(py));
        });
    }
}
