#[cfg(feature = "test-hooks")]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering as AtomicOrdering;
use std::sync::mpsc;

#[cfg(feature = "test-hooks")]
use rusqlite::params;

#[cfg(feature = "test-hooks")]
use crate::dependency_closure::{self, ClosureCauseV1};
use crate::errors::EngineError;
use crate::frozen_read;
use crate::graph_expand::execution::{
    explain_graph_expand_incident_query, graph_error, graph_expand_incident_query,
    validate_semantics,
};
#[cfg(feature = "test-hooks")]
use crate::graph_expand::execution::{
    GraphExpandCurrentRssSampleForTest, GraphExpandIsolatedProcessRssSampleForTest,
    GraphExpandMeasurementForTest, GraphExpandProjectionGenerationForTest,
    GraphExpandProjectionStateForTest, GraphExpandReaderControlsForTest,
    GraphExpandRendezvousForTest, GraphExpandRetentionCountersForTest,
    GRAPH_EXPAND_RSS_SAMPLE_SEQUENCE, GRAPH_EXPAND_SQL_STATEMENTS,
};
use crate::graph_expand::{
    GraphExpandRequestV1, GraphExpandResultV1, GraphExpansionErrorReasonV1, GraphReadContextV1,
    TraversalDirection, SCHEMA_VERSION,
};
use crate::projection_generation::ProjectionRuntimeStateV1;
#[cfg(feature = "test-hooks")]
use crate::projection_generation::{ProjectionGenerationOriginV1, ProjectionReadinessV1};
use crate::read::NodeRecord;
use crate::reader_pool::ReaderRequest;
use crate::temporal::{BoundaryCrossing, ReadView};
use crate::Engine;

#[cfg(feature = "test-hooks")]
impl Engine {
    #[doc(hidden)]
    pub fn measure_graph_expand_for_test(&self) -> GraphExpandMeasurementForTest {
        GraphExpandMeasurementForTest {
            peak_rss_delta_bytes: self.graph_expand_rss_delta_bytes.load(AtomicOrdering::Relaxed),
        }
    }

    #[doc(hidden)]
    pub fn seed_graph_expand_dependency_closure_for_test(&self) -> Result<(), EngineError> {
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        let cursor = self.next_cursor.load(AtomicOrdering::SeqCst).saturating_add(1);
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        tx.execute(
            "INSERT INTO canonical_nodes(\
                write_cursor, kind, body, source_id, logical_id, row_kind, state, reason, valid_from, valid_until\
             ) VALUES(?1, 'fact', 'derived', 'source-owner', 'derived', 'leaf', 'active', NULL, NULL, NULL)",
            [i64::try_from(cursor).map_err(|_| EngineError::Storage)?],
        )
        .map_err(|_| EngineError::Storage)?;
        tx.execute(
            "INSERT INTO _fathomdb_artifact_revisions(\
                schema_version, revision_id, artifact_class, write_cursor, artifact_role, completeness\
             ) VALUES(1, 'derived-r1', 'node', ?1, 'derived_semantic', 'complete')",
            [i64::try_from(cursor).map_err(|_| EngineError::Storage)?],
        )
        .map_err(|_| EngineError::Storage)?;
        tx.execute(
            "INSERT INTO _fathomdb_source_links(\
                schema_version, artifact_revision_id, source_id, source_version_id, source_revision_id,\
                locator_kind, start_byte, end_byte, hash_algorithm, hash_digest\
             ) SELECT schema_version, 'derived-r1', 'source-owner', source_version_id, source_revision_id,\
                      locator_kind, start_byte, end_byte, hash_algorithm, hash_digest \
               FROM _fathomdb_source_links WHERE artifact_revision_id='source-r1'",
            [],
        )
        .map_err(|_| EngineError::Storage)?;
        tx.commit().map_err(|_| EngineError::Storage)?;
        self.next_cursor.store(cursor, AtomicOrdering::SeqCst);
        Ok(())
    }

    #[doc(hidden)]
    pub fn seed_graph_expand_nonterminal_dependency_closure_for_test(
        &self,
    ) -> Result<(), EngineError> {
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        dependency_closure::admit_soft_closure(
            &tx,
            "source-r1",
            ClosureCauseV1::SoftDeleted,
            self.next_cursor.load(AtomicOrdering::SeqCst),
            dependency_closure::SoftClosureMode::Proving,
        )?;
        tx.commit().map_err(|_| EngineError::Storage)
    }

    #[doc(hidden)]
    pub fn seed_graph_expand_erasure_for_test(&self) -> Result<(), EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        connection
            .query_row("SELECT COUNT(*) FROM canonical_nodes", [], |row| row.get::<_, i64>(0))
            .map(|_| ())
            .map_err(|_| EngineError::Storage)
    }

    #[doc(hidden)]
    pub fn seed_graph_expand_projection_state_for_test(&self) -> GraphExpandProjectionStateForTest {
        GraphExpandProjectionStateForTest::fresh_ready()
    }

    #[doc(hidden)]
    pub fn graph_expand_current_rss_samples_for_test(
        request: &GraphExpandRequestV1,
        work_units: &[u64],
        unrelated_nodes: &[u64],
    ) -> Result<Vec<GraphExpandCurrentRssSampleForTest>, EngineError> {
        if work_units.len() != unrelated_nodes.len() {
            return Err(EngineError::Storage);
        }
        let sequence = GRAPH_EXPAND_RSS_SAMPLE_SEQUENCE.fetch_add(1, AtomicOrdering::Relaxed);
        let root = std::env::temp_dir()
            .join(format!("fathomdb-slice60-rss-{}-{sequence}", std::process::id()));
        std::fs::create_dir_all(&root).map_err(|_| EngineError::Storage)?;
        let mut samples = Vec::with_capacity(work_units.len());
        for (index, (&work, &unrelated)) in work_units.iter().zip(unrelated_nodes).enumerate() {
            let path = root.join(format!("arm-{index}.sqlite"));
            let opened = Engine::open(&path).map_err(|_| EngineError::Storage)?;
            opened.engine.seed_graph_expand_rss_fixture_for_test(work, unrelated)?;
            let baseline = crate::telemetry::process_current_rss_bytes();
            let result = opened.engine.graph_expand(request)?;
            let observed = crate::telemetry::process_current_rss_bytes().saturating_sub(baseline);
            samples.push(GraphExpandCurrentRssSampleForTest {
                current_rss_delta_bytes: observed,
                work_units: result.work_units,
            });
            opened.engine.close()?;
        }
        let _ = std::fs::remove_dir_all(root);
        Ok(samples)
    }

    #[doc(hidden)]
    pub fn graph_expand_isolated_process_rss_samples_for_test(
        request: &GraphExpandRequestV1,
        work_units: &[u64],
        unrelated_nodes: &[u64],
    ) -> Result<Vec<GraphExpandIsolatedProcessRssSampleForTest>, EngineError> {
        const CHILD_ARM: &str = "FATHOMDB_SLICE60_FIX5_RSS_CHILD_ARM";
        const SAMPLE_PREFIX: &str = "FATHOMDB_SLICE60_FIX5_RSS_SAMPLE=";

        if work_units.len() != unrelated_nodes.len() {
            return Err(EngineError::Storage);
        }
        if let Ok(index) = std::env::var(CHILD_ARM) {
            let index = index.parse::<usize>().map_err(|_| EngineError::Storage)?;
            let work = *work_units.get(index).ok_or(EngineError::Storage)?;
            let unrelated = *unrelated_nodes.get(index).ok_or(EngineError::Storage)?;
            let sample = Self::measure_isolated_process_rss_arm_for_test(request, work, unrelated)?;
            println!(
                "{SAMPLE_PREFIX}{}:{}:{}:{}:{}:{}:{}",
                sample.process_id,
                sample.peak_rss_delta_bytes,
                sample.work_units,
                sample.retained_edge_batch_rows,
                sample.frontier_states,
                sample.visited_states,
                sample.candidate_targets,
            );
            std::process::exit(0);
        }

        let executable = std::env::current_exe().map_err(|_| EngineError::Storage)?;
        let mut samples = Vec::with_capacity(work_units.len());
        for index in 0..work_units.len() {
            let output = std::process::Command::new(&executable)
                .arg("--nocapture")
                .arg("--test-threads=1")
                .env(CHILD_ARM, index.to_string())
                .output()
                .map_err(|_| EngineError::Storage)?;
            if !output.status.success() {
                return Err(EngineError::Storage);
            }
            let stdout = String::from_utf8(output.stdout).map_err(|_| EngineError::Storage)?;
            let fields = stdout
                .lines()
                .find_map(|line| line.split_once(SAMPLE_PREFIX).map(|(_, sample)| sample))
                .ok_or(EngineError::Storage)?
                .split(':')
                .collect::<Vec<_>>();
            let [process_id, peak_rss_delta_bytes, work_units, retained_edge_batch_rows, frontier_states, visited_states, candidate_targets] =
                fields.as_slice()
            else {
                return Err(EngineError::Storage);
            };
            samples.push(GraphExpandIsolatedProcessRssSampleForTest {
                process_id: process_id.parse().map_err(|_| EngineError::Storage)?,
                peak_rss_delta_bytes: peak_rss_delta_bytes
                    .parse()
                    .map_err(|_| EngineError::Storage)?,
                work_units: work_units.parse().map_err(|_| EngineError::Storage)?,
                retained_edge_batch_rows: retained_edge_batch_rows
                    .parse()
                    .map_err(|_| EngineError::Storage)?,
                frontier_states: frontier_states.parse().map_err(|_| EngineError::Storage)?,
                visited_states: visited_states.parse().map_err(|_| EngineError::Storage)?,
                candidate_targets: candidate_targets.parse().map_err(|_| EngineError::Storage)?,
            });
        }
        Ok(samples)
    }

    fn measure_isolated_process_rss_arm_for_test(
        request: &GraphExpandRequestV1,
        work: u64,
        unrelated: u64,
    ) -> Result<GraphExpandIsolatedProcessRssSampleForTest, EngineError> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (request, work, unrelated);
            return Err(EngineError::Storage);
        }
        #[cfg(target_os = "linux")]
        {
            let sequence = GRAPH_EXPAND_RSS_SAMPLE_SEQUENCE.fetch_add(1, AtomicOrdering::Relaxed);
            let root = std::env::temp_dir()
                .join(format!("fathomdb-slice60-rss-child-{}-{sequence}", std::process::id()));
            std::fs::create_dir_all(&root).map_err(|_| EngineError::Storage)?;
            let path = root.join("arm.sqlite");
            let opened = Engine::open(&path).map_err(|_| EngineError::Storage)?;
            opened.engine.seed_graph_expand_rss_fixture_for_test(work, unrelated)?;
            let baseline = crate::telemetry::process_current_rss_bytes();
            if baseline == 0 {
                return Err(EngineError::Storage);
            }
            let rss_peak = std::sync::Arc::new(AtomicU64::new(baseline));
            let retention_counters =
                std::sync::Arc::new(GraphExpandRetentionCountersForTest::default());
            let result = opened.engine.graph_expand_inner(
                request,
                GraphExpandReaderControlsForTest {
                    rss_peak_bytes: Some(std::sync::Arc::clone(&rss_peak)),
                    retention_counters: Some(std::sync::Arc::clone(&retention_counters)),
                    ..GraphExpandReaderControlsForTest::default()
                },
            )?;
            let observed = crate::telemetry::process_current_rss_bytes();
            let mut peak = rss_peak.load(AtomicOrdering::Relaxed);
            while observed > peak {
                match rss_peak.compare_exchange_weak(
                    peak,
                    observed,
                    AtomicOrdering::Relaxed,
                    AtomicOrdering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(next) => peak = next,
                }
            }
            opened.engine.close()?;
            let _ = std::fs::remove_dir_all(root);
            let (retained_edge_batch_rows, frontier_states, visited_states, candidate_targets) =
                GraphExpandRetentionCountersForTest::load_relaxed(&retention_counters);
            Ok(GraphExpandIsolatedProcessRssSampleForTest {
                process_id: std::process::id(),
                peak_rss_delta_bytes: peak.saturating_sub(baseline),
                work_units: result.work_units,
                retained_edge_batch_rows,
                frontier_states,
                visited_states,
                candidate_targets,
            })
        }
    }

    fn seed_graph_expand_rss_fixture_for_test(
        &self,
        work: u64,
        unrelated: u64,
    ) -> Result<(), EngineError> {
        use crate::identity::SourceId;
        use crate::record_lifecycle::InitialState;
        use crate::write::PreparedWrite;

        {
            let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
            let connection = connection.as_ref().ok_or(EngineError::Closing)?;
            connection
                .execute_batch("PRAGMA default_cache_size=64")
                .map_err(|_| EngineError::Storage)?;
        }
        self.write(&[PreparedWrite::Node {
            logical_id: Some("root".into()),
            kind: "fact".into(),
            body: "root".into(),
            source_id: SourceId::new("slice60-rss").map_err(|_| EngineError::Storage)?,
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])?;
        let mut writes = Vec::new();
        for index in 0..work {
            writes.push(PreparedWrite::Node {
                logical_id: Some(format!("target-{index}")),
                kind: "fact".into(),
                body: "target".into(),
                source_id: SourceId::new("slice60-rss").map_err(|_| EngineError::Storage)?,
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            });
            writes.push(PreparedWrite::Edge {
                logical_id: Some(format!("edge-{index}")),
                kind: "link".into(),
                from: "root".into(),
                to: format!("target-{index}"),
                source_id: SourceId::new("slice60-rss").map_err(|_| EngineError::Storage)?,
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            });
        }
        if !writes.is_empty() {
            self.write(&writes)?;
        }
        if unrelated > 0 {
            let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
            let connection = connection.as_mut().ok_or(EngineError::Closing)?;
            let first = self.next_cursor.load(AtomicOrdering::SeqCst).saturating_add(1);
            let tx = connection
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|_| EngineError::Storage)?;
            let mut insert = tx
                .prepare("INSERT INTO canonical_nodes(\
                    write_cursor,kind,body,source_id,logical_id,row_kind,state,reason,valid_from,valid_until\
                 ) VALUES(?1,'fact','unrelated','slice60-rss',?2,'leaf','active',NULL,NULL,NULL)")
                .map_err(|_| EngineError::Storage)?;
            for index in 0..unrelated {
                let cursor = first.saturating_add(index);
                insert
                    .execute(params![
                        i64::try_from(cursor).map_err(|_| EngineError::Storage)?,
                        format!("unrelated-{index}")
                    ])
                    .map_err(|_| EngineError::Storage)?;
            }
            drop(insert);
            tx.commit().map_err(|_| EngineError::Storage)?;
            self.next_cursor
                .store(first.saturating_add(unrelated).saturating_sub(1), AtomicOrdering::SeqCst);
        }
        Ok(())
    }
}

impl Engine {
    /// Slice 20 test seam — run `EXPLAIN QUERY PLAN` on the BFS CTE SQL and
    /// return the plan detail lines. Used by `explain_plan_uses_indexes`.
    #[doc(hidden)]
    pub fn explain_graph_neighbors_for_test(
        &self,
        root_logical_id: &str,
        depth: u32,
        direction: TraversalDirection,
    ) -> Result<Vec<String>, EngineError> {
        self.ensure_open()?;
        let (response_tx, response_rx) = mpsc::sync_channel(1);
        let request = ReaderRequest::explain_graph_neighbors(
            root_logical_id.to_string(),
            depth,
            direction,
            response_tx,
        );
        if self.reader_pool.dispatch(request).is_err() {
            return Err(EngineError::Closing);
        }
        match response_rx.recv().map_err(|_| EngineError::Storage)? {
            Ok(plan) => Ok(plan),
            Err(err) => {
                self.emit_sqlite_internal_error(&err);
                Err(EngineError::Storage)
            }
        }
    }

    /// Expand caller-supplied or FTS-resolved logical seeds through one bounded graph walk.
    pub fn graph_expand(
        &self,
        request: &GraphExpandRequestV1,
    ) -> Result<GraphExpandResultV1, EngineError> {
        self.graph_expand_inner(
            request,
            #[cfg(feature = "test-hooks")]
            GraphExpandReaderControlsForTest::default(),
        )
    }

    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn graph_expand_with_rendezvous_for_test(
        &self,
        request: &GraphExpandRequestV1,
        rendezvous: GraphExpandRendezvousForTest,
    ) -> Result<GraphExpandResultV1, EngineError> {
        self.graph_expand_inner(
            request,
            GraphExpandReaderControlsForTest {
                rendezvous: Some(rendezvous),
                ..GraphExpandReaderControlsForTest::default()
            },
        )
    }

    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn graph_expand_with_projection_state_for_test(
        &self,
        request: &GraphExpandRequestV1,
        projection_state: GraphExpandProjectionStateForTest,
    ) -> Result<GraphExpandResultV1, EngineError> {
        self.graph_expand_inner(
            request,
            GraphExpandReaderControlsForTest {
                projection_state: Some(projection_state),
                ..GraphExpandReaderControlsForTest::default()
            },
        )
    }

    /// Execute one graph expansion while counting every SQLite statement on
    /// its reader connection.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn graph_expand_with_statement_count_for_test(
        &self,
        request: &GraphExpandRequestV1,
    ) -> Result<(GraphExpandResultV1, u64), EngineError> {
        GRAPH_EXPAND_SQL_STATEMENTS.store(0, AtomicOrdering::SeqCst);
        let result = self.graph_expand_inner(
            request,
            GraphExpandReaderControlsForTest {
                count_sql_statements: true,
                ..GraphExpandReaderControlsForTest::default()
            },
        )?;
        Ok((result, GRAPH_EXPAND_SQL_STATEMENTS.load(AtomicOrdering::SeqCst)))
    }

    #[doc(hidden)]
    #[cfg(feature = "test-hooks")]
    pub fn graph_expand_with_projection_generation_for_test(
        &self,
        request: &GraphExpandRequestV1,
        origin: ProjectionGenerationOriginV1,
        readiness: ProjectionReadinessV1,
    ) -> Result<GraphExpandResultV1, EngineError> {
        self.graph_expand_inner(
            request,
            GraphExpandReaderControlsForTest {
                projection_generation: Some(GraphExpandProjectionGenerationForTest::new(
                    origin, readiness,
                )),
                ..GraphExpandReaderControlsForTest::default()
            },
        )
    }

    pub(crate) fn graph_expand_inner(
        &self,
        request: &GraphExpandRequestV1,
        #[cfg(feature = "test-hooks")] test_controls: GraphExpandReaderControlsForTest,
    ) -> Result<GraphExpandResultV1, EngineError> {
        self.ensure_open()?;
        if request.include_evidence && matches!(request.context, GraphReadContextV1::Current { .. })
        {
            return Err(graph_error(GraphExpansionErrorReasonV1::GraphContextInvalid, "/context"));
        }
        let frozen_binding = match &request.context {
            GraphReadContextV1::Current { schema_version, context } => {
                if *schema_version != SCHEMA_VERSION {
                    return Err(graph_error(
                        GraphExpansionErrorReasonV1::UnsupportedSchemaVersion,
                        "/context/schemaVersion",
                    ));
                }
                frozen_read::validate_context(context).map_err(|_| {
                    graph_error(GraphExpansionErrorReasonV1::GraphContextInvalid, "/context")
                })?;
                None
            }
            GraphReadContextV1::Frozen { schema_version, context } => {
                if *schema_version != SCHEMA_VERSION {
                    return Err(graph_error(
                        GraphExpansionErrorReasonV1::UnsupportedSchemaVersion,
                        "/context/schemaVersion",
                    ));
                }
                let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
                let connection = connection.as_ref().ok_or(EngineError::Closing)?;
                Some(Box::new(frozen_read::authenticate(connection, context)?))
            }
        };
        let context = match &request.context {
            GraphReadContextV1::Current { context, .. } => context,
            GraphReadContextV1::Frozen { context, .. } => &context.context,
        };
        if request.include_evidence && context.view.include_out_of_window {
            return Err(graph_error(
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                "/context/context/view/includeOutOfWindow",
            ));
        }
        if context.view.include_superseded || context.view.include_inactive {
            return Err(graph_error(GraphExpansionErrorReasonV1::GraphContextInvalid, "/context"));
        }
        validate_semantics(request)?;
        let projection_runtime_state = if self.runtime_embedder.is_none() {
            ProjectionRuntimeStateV1::Absent
        } else if self.dense_disabled.load(AtomicOrdering::Acquire) {
            ProjectionRuntimeStateV1::Refused
        } else {
            ProjectionRuntimeStateV1::Usable
        };
        #[cfg(feature = "test-hooks")]
        self.graph_expand_rss_baseline_bytes
            .store(crate::telemetry::process_current_rss_bytes(), AtomicOrdering::Relaxed);
        let evidence_authority = match (&request.context, request.include_evidence) {
            (GraphReadContextV1::Frozen { context, .. }, true) => {
                let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
                let connection = connection.as_ref().ok_or(EngineError::Closing)?;
                Some(crate::evidence::graph_evidence_authority(connection, context)?)
            }
            _ => None,
        };
        let (respond, receive) = std::sync::mpsc::sync_channel(1);
        self.reader_pool
            .dispatch(ReaderRequest::graph_expand(
                request.clone(),
                frozen_binding,
                projection_runtime_state,
                evidence_authority,
                #[cfg(feature = "test-hooks")]
                test_controls,
                respond,
            ))
            .map_err(|_| EngineError::Closing)?;
        let received = receive.recv().map_err(|_| EngineError::Storage)?;
        let mut result = received?;
        #[cfg(feature = "test-hooks")]
        {
            let baseline = self.graph_expand_rss_baseline_bytes.load(AtomicOrdering::Relaxed);
            let observed = crate::telemetry::process_current_rss_bytes().saturating_sub(baseline);
            self.graph_expand_rss_delta_bytes.store(observed, AtomicOrdering::Relaxed);
        }
        if request.include_explanation {
            let sequence = self.explanation_sequence.fetch_add(1, AtomicOrdering::Relaxed);
            let correlation_id = format!("x{:032x}-{sequence}", self.explanation_open_nonce);
            if let Some(explanation) = result.explanation.as_mut() {
                explanation.correlation_id = correlation_id;
            }
        }
        Ok(result)
    }

    /// Return endpoint-index query plans used by graph expansion.
    #[doc(hidden)]
    pub fn explain_graph_expand_for_test(
        &self,
        direction: TraversalDirection,
    ) -> Result<Vec<String>, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let query = graph_expand_incident_query("fixture", direction, 1);
        explain_graph_expand_incident_query(connection, &query)
    }
}

impl Engine {
    /// Slice 20 (G5) — `read.neighbors`: bounded BFS from `root_logical_id`
    /// over `canonical_edges`. Returns nodes reachable within `depth` hops
    /// (`1..=3`) in the given `direction`, excluding the root itself.
    ///
    /// Hard cap: 50 results (engine-enforced `LIMIT 50`).
    /// Traversal filter: `superseded_at IS NULL AND (t_invalid IS NULL OR t_invalid > now)`.
    ///
    /// Returns `Err(EngineError::InvalidArgument)` for `depth > 3`.
    /// Returns `Ok(vec![])` for an unknown/superseded root.
    /// Reads ride the `ReaderWorkerPool` DEFERRED-tx path.
    pub fn graph_neighbors(
        &self,
        root_logical_id: &str,
        depth: u32,
        direction: TraversalDirection,
        view: &ReadView,
    ) -> Result<Vec<NodeRecord>, EngineError> {
        self.ensure_open()?;
        if depth == 0 || depth > 3 {
            return Err(EngineError::InvalidArgument {
                msg: format!("traversal depth {depth} is out of range; must be 1, 2, or 3"),
            });
        }
        let (response_tx, response_rx) = mpsc::sync_channel(1);
        let request = ReaderRequest::graph_neighbors(
            root_logical_id.to_string(),
            depth,
            direction,
            *view,
            response_tx,
        );
        if self.reader_pool.dispatch(request).is_err() {
            return Err(EngineError::Closing);
        }
        match response_rx.recv().map_err(|_| EngineError::Storage)? {
            Ok(nodes) => Ok(nodes),
            Err(err) => {
                self.emit_sqlite_internal_error(&err);
                Err(EngineError::Storage)
            }
        }
    }

    /// 0.8.20 Slice 10b (R-20-NV) — the **validity-boundary hook**: which nodes
    /// crossed a `[valid_from, valid_until)` boundary in the half-open interval
    /// `(since, as_of]`?
    ///
    /// `since` and the resolved upper bound are INTEGER epoch SECONDS. The upper
    /// bound is the view's own instant (`view.valid_as_of`, defaulting to now),
    /// so one instant governs both the boundary interval and the view — and, as
    /// everywhere else on this path, it is BOUND, never a `datetime('now')`
    /// literal, so the answer is deterministic for a fixed `(since, as_of)`.
    ///
    /// A node appears once, carrying whichever of the two boundaries it crossed;
    /// a window that both opened AND closed inside the interval reports both.
    /// Rows with an unbounded window on a side cannot cross that side, so a
    /// NULL/NULL row (every row predating schema step 22) never appears.
    ///
    /// The view's EXISTENCE flags still apply (so by default only current,
    /// active rows are considered), but its validity predicate does NOT: the
    /// question is about boundary crossings, not about being valid right now.
    ///
    /// When the view relaxes validity entirely (`include_out_of_window`), the
    /// interval is unbounded above.
    ///
    /// This is world-time only. There is deliberately no transaction-time
    /// (`history_as_of`) counterpart.
    pub fn crossed_boundary_since(
        &self,
        since: i64,
        view: &ReadView,
    ) -> Result<Vec<BoundaryCrossing>, EngineError> {
        self.ensure_open()?;
        let (response_tx, response_rx) = mpsc::sync_channel(1);
        let request = ReaderRequest::crossed_boundary_since(since, *view, response_tx);
        if self.reader_pool.dispatch(request).is_err() {
            return Err(EngineError::Closing);
        }
        match response_rx.recv().map_err(|_| EngineError::Storage)? {
            Ok(rows) => Ok(rows),
            Err(err) => {
                self.emit_sqlite_internal_error(&err);
                Err(EngineError::Storage)
            }
        }
    }
}
