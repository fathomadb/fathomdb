use super::*;
use crate::mean::MEAN_VEC_PIN_THRESHOLD;
use crate::projection_runtime::PROJECTION_CURSOR_KEY;

fn projection_batch_has_no_custom_triggers(connection: &Connection) -> rusqlite::Result<bool> {
    let unexpected: bool = connection
        .prepare_cached(
            "SELECT EXISTS(
             SELECT 1 FROM sqlite_master
             WHERE type='trigger'
               AND tbl_name IN (
                   '_fathomdb_vector_rows','_fathomdb_projection_terminal',
                   '_fathomdb_embedder_profiles','operational_mutations',
                   '_fathomdb_open_state','_fathomdb_read_visibility_state',
                   'vector_default'
               )
               AND name NOT LIKE '_fathomdb_read_visibility_%'
             UNION ALL
             SELECT 1 FROM sqlite_temp_master
             WHERE type='trigger'
               AND tbl_name IN (
                   '_fathomdb_vector_rows','_fathomdb_projection_terminal',
                   '_fathomdb_embedder_profiles','operational_mutations',
                   '_fathomdb_open_state','_fathomdb_read_visibility_state',
                   'vector_default'
               )
               AND name NOT LIKE '_fathomdb_read_visibility_%'
         )",
        )?
        .query_row([], |row| row.get(0))?;
    Ok(!unexpected)
}

pub(crate) fn load_projection_cursor(connection: &Connection) -> rusqlite::Result<u64> {
    connection
        .query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key = ?1",
            [PROJECTION_CURSOR_KEY],
            |row| row.get::<_, String>(0),
        )
        .map(|value| value.parse::<u64>().unwrap_or(0))
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(0),
            _ => Err(err),
        })
}

pub(crate) fn store_projection_cursor(
    connection: &Connection,
    cursor: u64,
) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO _fathomdb_open_state(key, value) VALUES(?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![PROJECTION_CURSOR_KEY, cursor.to_string()],
    )?;
    Ok(())
}

pub(crate) fn record_projection_terminal(
    connection: &Connection,
    cursor: u64,
    state: &str,
) -> rusqlite::Result<()> {
    connection
        .prepare_cached(
            "INSERT OR IGNORE INTO _fathomdb_projection_terminal(write_cursor, state) VALUES(?1, ?2)",
        )?
        .execute(params![cursor, state])?;
    Ok(())
}

pub(crate) fn terminal_state_for_cursor(
    connection: &Connection,
    cursor: u64,
) -> rusqlite::Result<Option<String>> {
    connection
        .prepare_cached("SELECT state FROM _fathomdb_projection_terminal WHERE write_cursor = ?1")?
        .query_row([cursor], |row| row.get::<_, String>(0))
        .map(Some)
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            _ => Err(err),
        })
}

fn projection_physical_tuple_is_empty(
    connection: &Connection,
    cursor: u64,
) -> rusqlite::Result<bool> {
    let sidecar_count: u64 = connection
        .prepare_cached(
            "SELECT COUNT(*) FROM _fathomdb_vector_rows \
             WHERE rowid=?1 OR write_cursor=?1",
        )?
        .query_row([cursor], |row| row.get(0))?;
    let vector_count: u64 = connection
        .prepare_cached("SELECT COUNT(*) FROM vector_default WHERE rowid=?1")?
        .query_row([cursor], |row| row.get(0))?;
    Ok(sidecar_count == 0 && vector_count == 0)
}

fn projection_tuple_corruption() -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CORRUPT),
        Some("projection physical tuple is partial or mismatched".to_string()),
    )
}

pub(crate) fn advance_projection_cursor(connection: &Connection) -> rusqlite::Result<u64> {
    let mut cursor = load_projection_cursor(connection)?;
    loop {
        let next = cursor.saturating_add(1);
        if terminal_state_for_cursor(connection, next)?.is_some() {
            cursor = next;
        } else {
            break;
        }
    }
    store_projection_cursor(connection, cursor)?;
    Ok(cursor)
}

pub(crate) fn commit_projection_outcomes(
    connection: &mut Connection,
    outcomes: &[ProjectionOutcome],
    shared: &ProjectionRuntimeShared,
    worker_idx: usize,
) -> rusqlite::Result<()> {
    let embedder_identity = &shared.embedder_identity;
    let mc = identity_requires_mean_centering(embedder_identity);
    // EU-5f — serialize the whole commit across workers so the at-pin
    // re-quantize sees a totally-ordered history (see `commit_gate`).
    let _gate = shared.commit_gate.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    #[cfg(feature = "test-hooks")]
    if let Some((waiting, release)) = shared
        .projection_worker_before_write_lock_pause
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take()
    {
        waiting.wait();
        release.wait();
    }
    // Take the WAL write lock before the reads below. A deferred transaction
    // would need a read-to-write promotion while a concurrent Engine::write
    // holds its own immediate transaction, which SQLite rejects without
    // invoking the busy handler and forces the worker to recompute the batch.
    let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    if dependency_closure::guard_no_pending_physical(&tx).is_err() {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            Some("dependency closure fences projection publication".to_string()),
        ));
    }
    let coalesce_visibility = projection_batch_has_no_custom_triggers(&tx)?;
    let mut visibility_changed = false;
    let _activity = shared.wal_attribution.enabled.then(|| {
        WalAttributionActivity::begin(
            Arc::clone(&shared.wal_attribution),
            WalAttributionRole::ProjectionWorker,
            worker_idx,
            "transaction_opened",
        )
    });
    #[cfg(debug_assertions)]
    if let Some((transaction_ready, release)) = shared
        .projection_worker_transaction_pause
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take()
    {
        // `transaction_with_behavior(Immediate)` above has already acquired the
        // WAL write lock. The rendezvous must therefore report that SQLite fact
        // directly; WAL attribution is optional in integration builds and is
        // neither necessary nor sufficient to establish transaction ownership.
        if transaction_ready.send(()).is_ok() {
            let _ = release.recv_timeout(PROJECTION_TRANSACTION_TEST_PAUSE_RELEASE_TIMEOUT);
        }
    }
    // The accumulator is mutable process state coupled to this transaction.
    // Keep the shared value untouched while building a candidate so rollback
    // cannot count a vector or consume the pin threshold prematurely.
    let mut shared_accumulator =
        shared.mean_accumulator.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut candidate_accumulator = shared_accumulator.clone();
    // EU-5a2/EU-5f — the live pinned mean. Read once at the top; may pin
    // mid-batch (set to `Some` after a threshold-crossing row below).
    let mut current_mean: Option<Vec<f32>> = if mc {
        tx.query_row(
            "SELECT mean_vec FROM _fathomdb_embedder_profiles WHERE profile = 'default'",
            [],
            |row| row.get::<_, Option<Vec<u8>>>(0),
        )
        .ok()
        .flatten()
        .map(|bytes| decode_vector_blob(&bytes))
    } else {
        None
    };
    let mut staged_events: Vec<EmbedderEvent> = Vec::new();
    let mut trigger_guard = if coalesce_visibility {
        Some(TriggerStateGuard::disable(&tx).map_err(|_| rusqlite::Error::InvalidQuery)?)
    } else {
        None
    };
    // Generation and eligibility time cannot change while this IMMEDIATE
    // transaction owns the writer lock. Read them once for the whole batch.
    let current_generation_id = projection_generation::current_generation_id(&tx)
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    let effective_at = current_epoch_seconds();
    for outcome in outcomes {
        match outcome {
            ProjectionOutcome::Success { cursor, kind, blob, bin_blob, generation_id } => {
                if current_generation_id != *generation_id {
                    continue;
                }
                if projection_generation::dense_member_kind_at(&tx, *cursor, effective_at)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?
                    .as_deref()
                    != Some(kind.as_str())
                {
                    continue;
                }
                if terminal_state_for_cursor(&tx, *cursor)?.is_some() {
                    continue;
                }
                if !projection_physical_tuple_is_empty(&tx, *cursor)? {
                    return Err(projection_tuple_corruption());
                }
                // Build the threshold decision in the transaction-local
                // candidate. The shared accumulator changes only after commit.
                let pin_mean: Option<Vec<f32>> = if mc && current_mean.is_none() {
                    match candidate_accumulator.as_mut() {
                        Some(a) => {
                            a.add(&decode_vector_blob(bin_blob));
                            if a.count() >= MEAN_VEC_PIN_THRESHOLD {
                                let mean = a.materialize();
                                candidate_accumulator = None;
                                Some(mean)
                            } else {
                                None
                            }
                        }
                        None => None,
                    }
                } else {
                    None
                };

                let source_type = resolve_source_type(kind).map_err(|_| {
                    rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
                        Some(format!("unknown kind for source_type mapping: {kind}")),
                    )
                })?;
                let now_unix =
                    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
                        as i64;
                tx.execute(
                    "INSERT INTO _fathomdb_vector_rows(rowid, kind, write_cursor) \
                     VALUES(?1, ?2, ?3)",
                    params![cursor, kind, cursor],
                )?;
                // EU-5a2/EU-5f — sign-quant input is the mean-subtracted
                // vector iff a mean is live (`current_mean`); otherwise the
                // un-centered `bin_blob`. A row inserted just before the
                // crossing is centered retroactively by the re-quantize
                // pass below.
                let centered_blob: Vec<u8> = match &current_mean {
                    Some(mean) if mean.len() * 4 == bin_blob.len() => {
                        encode_vector_blob(&subtract_mean(&decode_vector_blob(bin_blob), mean))
                    }
                    _ => bin_blob.clone(),
                };
                // Slice 10 / G10 — `status` ships the empty-string sentinel (vec0
                // TEXT metadata is NOT NULL-able); no real population source yet
                // (reserved-gap candidate 13).
                //
                // 0.8.20 Slice 15e — when the live `vector_default` carries
                // `filterable` `attr_<hex>` columns, EVERY column must be bound
                // (vec0 rejects a partial-column INSERT). Bind each from the node
                // body's scalar extraction (or the `''` sentinel). The body is not
                // carried on the embed job, so it is read from `canonical_nodes` by
                // `write_cursor` (== rowid) — but ONLY when attr columns exist, so
                // the common no-filterable hot path stays byte-identical and does
                // NO extra lookup.
                if actual_vector_attr_columns(&tx)?.is_empty() {
                    tx.execute(
                        "INSERT INTO vector_default(
                            rowid, embedding, embedding_bin, source_type, kind, created_at, status
                         ) VALUES(?1, ?2, vec_quantize_binary(?3), ?4, ?5, ?6, '')",
                        params![cursor, blob, centered_blob, source_type, kind, now_unix],
                    )?;
                } else {
                    let body: String = tx
                        .query_row(
                            "SELECT body FROM canonical_nodes WHERE write_cursor = ?1 LIMIT 1",
                            [*cursor as i64],
                            |row| row.get(0),
                        )
                        .optional()?
                        .unwrap_or_default();
                    let (cols_sql, ph_sql, attr_vals) =
                        vector_attr_insert_fragments(&tx, &body, 7)?;
                    let sql = format!(
                        "INSERT INTO vector_default(
                            rowid, embedding, embedding_bin, source_type, kind, created_at, status{cols_sql}
                         ) VALUES(?1, ?2, vec_quantize_binary(?3), ?4, ?5, ?6, ''{ph_sql})"
                    );
                    let mut pv: Vec<rusqlite::types::Value> = vec![
                        rusqlite::types::Value::Integer(*cursor as i64),
                        rusqlite::types::Value::Blob(blob.clone()),
                        rusqlite::types::Value::Blob(centered_blob.clone()),
                        rusqlite::types::Value::Text(source_type.to_string()),
                        rusqlite::types::Value::Text(kind.to_string()),
                        rusqlite::types::Value::Integer(now_unix),
                    ];
                    pv.extend(attr_vals);
                    tx.execute(&sql, rusqlite::params_from_iter(pv.iter()))?;
                }
                record_projection_terminal(&tx, *cursor, "up_to_date")?;
                visibility_changed = true;

                // EU-5f — this row crossed the threshold: pin the mean and
                // re-quantize every row written so far (incl. earlier rows
                // in this same tx, which are visible to the SELECT) within
                // the same transaction so the pin is atomic.
                if let Some(mean) = pin_mean {
                    tx.execute(
                        "UPDATE _fathomdb_embedder_profiles SET mean_vec = ?1 WHERE profile = 'default'",
                        params![encode_vector_blob(&mean)],
                    )?;
                    let rows: Vec<(i64, Vec<u8>)> = {
                        let mut statement = tx.prepare(
                            "SELECT rowid, embedding FROM vector_default ORDER BY rowid",
                        )?;
                        let mapped = statement.query_map([], |row| {
                            Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
                        })?;
                        let mut out = Vec::new();
                        for r in mapped {
                            out.push(r?);
                        }
                        out
                    };
                    let (doc_count, _) =
                        run_pin_and_requantize_pass(&tx, &rows, &mean).map_err(|_| {
                            rusqlite::Error::SqliteFailure(
                                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
                                Some("mean-centering re-quantize pass failed".to_string()),
                            )
                        })?;
                    staged_events.push(EmbedderEvent::MeanVecPinned {
                        dim: u32::try_from(mean.len()).unwrap_or(u32::MAX),
                        doc_count,
                    });
                    current_mean = Some(mean);
                }
            }
            ProjectionOutcome::Failure { cursor, failure_code, generation_id } => {
                if current_generation_id != *generation_id {
                    continue;
                }
                if projection_generation::dense_member_kind_at(&tx, *cursor, effective_at)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?
                    .is_none()
                {
                    continue;
                }
                if terminal_state_for_cursor(&tx, *cursor)?.is_some() {
                    continue;
                }
                if !projection_physical_tuple_is_empty(&tx, *cursor)? {
                    return Err(projection_tuple_corruption());
                }
                let existing: u64 = tx.query_row(
                    "SELECT COUNT(*) FROM operational_mutations
                     WHERE collection_name = 'projection_failures'
                       AND json_extract(payload_json, '$.write_cursor') = ?1",
                    [cursor],
                    |row| row.get(0),
                )?;
                if existing == 0 {
                    let payload = format!(
                        r#"{{"write_cursor":{cursor},"failure_code":"{failure_code}","recorded_at":0}}"#
                    );
                    tx.execute(
                        "INSERT INTO operational_mutations(
                            collection_name, record_key, op_kind, payload_json, schema_id, write_cursor
                         ) VALUES('projection_failures', ?1, 'append', ?2, NULL, ?3)",
                        params![cursor.to_string(), payload, cursor],
                    )?;
                }
                record_projection_terminal(&tx, *cursor, "failed")?;
                visibility_changed = true;
            }
            // 0.8.20 Slice 20c fix-4 (codex §9 round 3 [P1]) — record NOTHING.
            //
            // No `projection_failures` audit row (an ABSENT embedder is an
            // environment fact, not an embed failure) and, decisively, no
            // terminal: the row keeps `terminal IS NULL`, so
            // `advance_projection_cursor` below cannot step over it, the shared
            // `connection_has_pending_projection_work` predicate still reports it
            // outstanding, and `derive_dense_readiness` therefore reads
            // `embedding`. That is the ONLY torn state
            // `dev/design/record-lifecycle-protocol/projection-registry-and-async-embed.md`
            // §4.1 invariant 1 tolerates; the alternative — the enqueue-side gate
            // — puts an `'up_to_date'` terminal on an ENROLLED row with no
            // vector, which is the torn `ready` that invariant calls FORBIDDEN.
            //
            // Q6a graceful-absent governs ROLE DECLARATION ("you declared a
            // projection I cannot build yet" -> defer + graft), i.e. the
            // NOT-yet-enrolled case fix-1/fix-2 handle. Once a kind IS enrolled,
            // §4.1 invariant 1 governs. (HITL ruling, 0.8.20 Slice 20c fix-4.)
            //
            // Consumer-visible consequence, accepted deliberately and pinned by
            // `slice20c_flush_barrier`: for the REST of that no-embedder session
            // `dense_readiness` stays `embedding` and `drain` burns its timeout
            // into `EngineError::Scheduler`. Loud and recoverable, rather than
            // silent and lost.
            ProjectionOutcome::Deferred => {}
        }
    }
    // 0.7.2 PR-2bc S2 — the AUTOMATIC in-ingest drift detector (EWMA recent
    // mean + cos-threshold + debounce + 200k cap + `MeanRecomputeDeferred`)
    // was CARVED OUT and DEFERRED to 0.8.x; its recall premise was refuted
    // (the mean is a non-lever) and the benefit is unmeasured. The mean is
    // refreshed only on demand via `Engine::recompute_mean` (the
    // `doctor recompute-mean` verb). See `dev/design/embedder.md` §0.3 and
    // `dev/plans/prompts/0.8.x-auto-mean-drift-DEFERRED.md`. Nothing here
    // mutates `mean_vec` after the initial pin.

    advance_projection_cursor(&tx)?;
    #[cfg(debug_assertions)]
    match shared.force_projection_commit_failure.swap(0, Ordering::SeqCst) {
        1 => {
            return Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                Some("forced projection commit failure".to_string()),
            ));
        }
        2 => return Err(rusqlite::Error::InvalidQuery),
        _ => {}
    }
    if coalesce_visibility && visibility_changed {
        advance_read_visibility(&tx)?;
    }
    if let Some(trigger_guard) = trigger_guard.as_mut() {
        trigger_guard.restore().map_err(|_| rusqlite::Error::InvalidQuery)?;
    }
    drop(trigger_guard);
    tx.commit()?;
    // Commit and the accumulator transition become visible together. On every
    // earlier error the transaction and the local candidate drop, leaving the
    // runtime state exactly as it was before this attempt.
    *shared_accumulator = candidate_accumulator;
    // EU-5f — publish MeanVecPinned only after the pin tx is durable, so a
    // rolled-back pin never emits a spurious event.
    if !staged_events.is_empty() {
        if let Ok(mut events) = shared.pending_events.lock() {
            events.extend(staged_events);
        }
    }
    Ok(())
}
