use super::*;

/// EU-5a2 — streaming f64 accumulator for the mean-centering pipeline,
/// per `dev/design/embedder.md` §0.3 (f64 chosen to bound numerical
/// drift across `MEAN_VEC_PIN_THRESHOLD` adds). Owned by the projection
/// worker; materialized into the schema column at the threshold cross.
#[derive(Clone, Debug)]
pub(crate) struct MeanAccumulator {
    sum: Vec<f64>,
    count: u64,
}

impl MeanAccumulator {
    pub(crate) fn new(dim: usize) -> Self {
        Self { sum: vec![0.0; dim], count: 0 }
    }

    pub(crate) fn add(&mut self, v: &[f32]) {
        debug_assert_eq!(v.len(), self.sum.len(), "accumulator dim mismatch");
        for (slot, value) in self.sum.iter_mut().zip(v.iter()) {
            *slot += f64::from(*value);
        }
        self.count = self.count.saturating_add(1);
    }

    pub(crate) fn materialize(&self) -> Vec<f32> {
        if self.count == 0 {
            return vec![0.0; self.sum.len()];
        }
        let denom = self.count as f64;
        self.sum.iter().map(|s| (s / denom) as f32).collect()
    }

    pub(crate) fn count(&self) -> u64 {
        self.count
    }
}

/// 0.7.2 PR-2b — cosine similarity between two equal-length vectors.
/// Returns 1.0 for a pair with a zero-norm operand (treated as "no drift
/// signal"), so the detector never fires on a degenerate all-zero mean.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 1.0;
    }
    let mut dot = 0.0f64;
    let mut na = 0.0f64;
    let mut nb = 0.0f64;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += f64::from(*x) * f64::from(*y);
        na += f64::from(*x) * f64::from(*x);
        nb += f64::from(*y) * f64::from(*y);
    }
    if na == 0.0 || nb == 0.0 {
        return 1.0;
    }
    (dot / (na.sqrt() * nb.sqrt())) as f32
}

/// EU-5b — at-pin pin-and-requantize pass per `dev/design/embedder.md`
/// §0.5. Runs INSIDE the caller's SQLite transaction so the mean_vec
/// INSERT/UPDATE + the per-row sign-bit UPDATEs commit atomically.
///
/// For each pre-pin row, recomputes `bits' = sign_quantize(f32 - mean)`
/// via the SQL extension's `vec_quantize_binary`, then UPDATEs the
/// row's `embedding_bin` column.
pub(crate) fn run_pin_and_requantize_pass(
    tx: &rusqlite::Transaction<'_>,
    rows: &[(i64, Vec<u8>)],
    mean: &[f32],
) -> Result<(u64, Vec<EmbedderEvent>), EngineError> {
    let mut updated: u64 = 0;
    let dim = mean.len();
    // sqlite-vec's vec0 xUpdate path discards SQL-function result subtypes
    // (see sqlite-vec.c §vec0Update_UpdateVectorColumn — "subtypes don't
    // appear to survive xColumn -> xUpdate, it's always 0"), so a direct
    // `UPDATE ... SET embedding_bin = vec_quantize_binary(?)` reads the
    // bound value as a float32-tagged vector and trips the column-type
    // check. We work around by DELETE+INSERT inside the same transaction:
    // INSERT preserves the BIT subtype on `vec_quantize_binary`. The
    // surrounding pin-commit tx keeps the rewrite atomic.
    for (rowid, blob) in rows {
        if blob.len() != dim * 4 {
            return Err(EngineError::Storage);
        }
        let un_centered = decode_vector_blob(blob);
        let centered = subtract_mean(&un_centered, mean);
        let centered_blob = encode_vector_blob(&centered);

        let (source_type, kind, created_at): (String, String, i64) = tx
            .query_row(
                "SELECT source_type, kind, created_at FROM vector_default WHERE rowid = ?1",
                params![rowid],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|_| EngineError::Storage)?;

        // 0.8.20 Slice 15e — this DELETE+INSERT re-quantize (its BIT-subtype
        // workaround, condition #4's SEPARATE same-shape op) must PRESERVE every
        // `filterable` `attr_<hex>` value, not just re-`''` them: a projection may
        // have been declared before the pin. Read the live attr columns + this
        // row's values BEFORE the DELETE, then re-bind them. Empty ⇒ the INSERT is
        // byte-identical to the shipped statement.
        let attr_cols = actual_vector_attr_columns(tx).map_err(|_| EngineError::Storage)?;
        let attr_vals: Vec<String> = if attr_cols.is_empty() {
            Vec::new()
        } else {
            let select = attr_cols.join(", ");
            tx.query_row(
                &format!("SELECT {select} FROM vector_default WHERE rowid = ?1"),
                params![rowid],
                |row| {
                    let mut vals = Vec::with_capacity(attr_cols.len());
                    for i in 0..attr_cols.len() {
                        vals.push(row.get::<_, String>(i)?);
                    }
                    Ok(vals)
                },
            )
            .map_err(|_| EngineError::Storage)?
        };

        delete_vector_partition_row(tx, *rowid).map_err(|_| EngineError::Storage)?;

        // Slice 10 / G10 — `status` ships the empty-string sentinel (vec0 TEXT
        // metadata is NOT NULL-able).
        let mut cols_sql = String::new();
        let mut ph_sql = String::new();
        for (i, col) in attr_cols.iter().enumerate() {
            cols_sql.push_str(&format!(", {col}"));
            ph_sql.push_str(&format!(", ?{}", 7 + i));
        }
        let sql = format!(
            "INSERT INTO vector_default(
                rowid, embedding, embedding_bin, source_type, kind, created_at, status{cols_sql}
             ) VALUES(?1, ?2, vec_quantize_binary(?3), ?4, ?5, ?6, ''{ph_sql})"
        );
        let mut pv: Vec<rusqlite::types::Value> = vec![
            rusqlite::types::Value::Integer(*rowid),
            rusqlite::types::Value::Blob(blob.clone()),
            rusqlite::types::Value::Blob(centered_blob),
            rusqlite::types::Value::Text(source_type),
            rusqlite::types::Value::Text(kind),
            rusqlite::types::Value::Integer(created_at),
        ];
        for v in attr_vals {
            pv.push(rusqlite::types::Value::Text(v));
        }
        tx.execute(&sql, rusqlite::params_from_iter(pv.iter()))
            .map_err(|_| EngineError::Storage)?;

        updated = updated.saturating_add(1);
    }
    let events = vec![EmbedderEvent::MeanVecPinned {
        dim: u32::try_from(dim).unwrap_or(u32::MAX),
        doc_count: updated,
    }];
    Ok((updated, events))
}

/// EU-5a2 — back-compat test-only count+emit helper. Preserved so the
/// EU-5a2 machinery test stays green; the EU-5b production path uses
/// `run_pin_and_requantize_pass`.
pub(crate) fn run_requantize_pass(
    rows: &[(i64, Vec<u8>)],
    mean: &[f32],
) -> (u64, Vec<EmbedderEvent>) {
    let mut updated: u64 = 0;
    let dim = mean.len();
    for (_rowid, blob) in rows {
        if blob.len() != dim * 4 {
            continue;
        }
        updated = updated.saturating_add(1);
    }
    let events = vec![EmbedderEvent::MeanVecPinned {
        dim: u32::try_from(dim).unwrap_or(u32::MAX),
        doc_count: updated,
    }];
    (updated, events)
}

/// EU-5f — open-time recovery pin (`dev/design/embedder.md` §0.3, Hazard 4).
/// Derives the corpus mean from the existing un-centered `vector_default`
/// rows, pins it, and re-quantizes every row, all in one transaction on the
/// single-threaded open connection (no workers running yet, so no gate is
/// needed). Called only when MC is required, no mean is pinned, and the row
/// count already meets the threshold.
pub(crate) fn recover_mean_vec_pin(
    connection: &mut Connection,
    identity: &EmbedderIdentity,
) -> Result<(), EngineError> {
    let tx = connection.transaction().map_err(|_| EngineError::Storage)?;
    recompute_mean_in_tx(&tx, identity)?;
    tx.commit().map_err(|_| EngineError::Storage)?;
    Ok(())
}

/// 0.7.2 PR-2b — shared mean (re)compute core, run INSIDE the caller's
/// transaction. Derives the FULL-corpus mean from the un-centered
/// `vector_default.embedding` BLOBs, writes `mean_vec`, and re-quantizes
/// EVERY row via the existing [`run_pin_and_requantize_pass`] so no row is
/// left under a stale centering.
///
/// This generalizes the EU-5f open-time recovery pin: it has NO "no mean
/// pinned yet" guard, so it equally serves the FIRST pin (recovery) and a
/// REFRESH of an already-pinned mean (PR-2b drift / `doctor recompute-mean`).
/// The caller owns the transaction boundary, which is what makes a fault
/// between the `mean_vec` UPDATE and re-quantize completion roll back
/// wholesale (`dev/design/embedder.md` §0.5 atomicity). It does NOT publish
/// any event — that is the caller's job, strictly post-durable-commit.
fn recompute_mean_in_tx(
    tx: &rusqlite::Transaction<'_>,
    identity: &EmbedderIdentity,
) -> Result<MeanRecomputeReport, EngineError> {
    recompute_mean_in_tx_inner(tx, identity, false)
}

/// 0.7.2 PR-2b — recompute core with an optional fault-injection point. The
/// `fail_after_mean_update` flag (debug builds only, set via a test seam)
/// errors AFTER the `mean_vec` UPDATE but BEFORE the re-quantize completes,
/// so the caller's tx rolls back the partial recentering.
fn recompute_mean_in_tx_inner(
    tx: &rusqlite::Transaction<'_>,
    identity: &EmbedderIdentity,
    fail_after_mean_update: bool,
) -> Result<MeanRecomputeReport, EngineError> {
    let started = Instant::now();
    let dim = identity.dimension as usize;
    // The previously-pinned mean (if any) is read first so we can report
    // the pre-recompute drift cosine.
    let old_mean = read_pinned_mean_vec(tx, identity.dimension)?;
    let rows: Vec<(i64, Vec<u8>)> = {
        let mut statement = tx
            .prepare("SELECT rowid, embedding FROM vector_default ORDER BY rowid")
            .map_err(|_| EngineError::Storage)?;
        let mapped = statement
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?)))
            .map_err(|_| EngineError::Storage)?;
        let mut out = Vec::new();
        for r in mapped {
            out.push(r.map_err(|_| EngineError::Storage)?);
        }
        out
    };
    let mut accumulator = MeanAccumulator::new(dim);
    for (_rowid, blob) in &rows {
        if blob.len() != dim * 4 {
            return Err(EngineError::Storage);
        }
        accumulator.add(&decode_vector_blob(blob));
    }
    let old_doc_count = accumulator.count();
    let mean = accumulator.materialize();
    let drift_cos_before = match &old_mean {
        Some(old) => cosine_similarity(&mean, old),
        None => 1.0,
    };
    tx.execute(
        "UPDATE _fathomdb_embedder_profiles SET mean_vec = ?1 WHERE profile = 'default'",
        params![encode_vector_blob(&mean)],
    )
    .map_err(|_| EngineError::Storage)?;
    if fail_after_mean_update {
        // Injected fault: bail before re-quantizing so the caller's tx
        // rolls back the `mean_vec` UPDATE too (crash-atomicity proof).
        return Err(EngineError::Storage);
    }
    let (doc_count, _) = run_pin_and_requantize_pass(tx, &rows, &mean)?;
    Ok(MeanRecomputeReport {
        dim: u32::try_from(dim).unwrap_or(u32::MAX),
        old_doc_count,
        doc_count_requantized: doc_count,
        drift_cos_before,
        mean_was_pinned: old_mean.is_some(),
        elapsed_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

/// EU-5a2 — does the live embedder identity request mean-centering?
/// Identity-name compare per EU-5a1's BGE_SMALL_EMBEDDER_NAME constant
/// (`dev/design/embedder.md` §0.6). NoopEmbedder returns `false`.
pub(crate) fn identity_requires_mean_centering(identity: &EmbedderIdentity) -> bool {
    identity.name == BGE_SMALL_EMBEDDER_NAME
}

/// EU-5a2 — read the pinned mean vector from
/// `_fathomdb_embedder_profiles.mean_vec` for the default profile.
/// Returns `Ok(None)` when the column is NULL or the row is missing;
/// returns `Err(EngineError::Storage)` on dimension drift (the open-time
/// `check_embedder_profile` already fails closed for this, so a runtime
/// drift here would be an internal-inconsistency signal).
pub(crate) fn read_pinned_mean_vec(
    connection: &Connection,
    dimension: u32,
) -> Result<Option<Vec<f32>>, EngineError> {
    let bytes: Option<Vec<u8>> = connection
        .query_row(
            "SELECT mean_vec FROM _fathomdb_embedder_profiles WHERE profile = 'default'",
            [],
            |row| row.get::<_, Option<Vec<u8>>>(0),
        )
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })
        .map_err(|_| EngineError::Storage)?;
    let Some(bytes) = bytes else { return Ok(None) };
    let expected_len = (dimension as usize).saturating_mul(4);
    if bytes.len() != expected_len {
        return Err(EngineError::Storage);
    }
    let mut out = Vec::with_capacity(dimension as usize);
    for chunk in bytes.chunks_exact(4) {
        let arr = [chunk[0], chunk[1], chunk[2], chunk[3]];
        out.push(f32::from_le_bytes(arr));
    }
    Ok(Some(out))
}

/// EU-5a2 — pointwise `v - mean`. Length-checked debug-assert; caller
/// guarantees equal length via `read_pinned_mean_vec` + dimension check.
pub(crate) fn subtract_mean(v: &[f32], mean: &[f32]) -> Vec<f32> {
    debug_assert_eq!(v.len(), mean.len(), "subtract_mean dim mismatch");
    v.iter().zip(mean.iter()).map(|(a, b)| *a - *b).collect()
}

impl Engine {
    /// 0.7.2 PR-2b — explicit `doctor recompute-mean` path. Re-derives the
    /// pinned corpus mean from the current `vector_default` rows and
    /// re-quantizes every row, SYNCHRONOUSLY in one transaction. ALWAYS
    /// allowed at any corpus size — this is the ONLY mean-refresh path as of
    /// 0.7.2 (the automatic in-ingest drift detector was carved out / deferred
    /// to 0.8.x; see `dev/design/embedder.md` §0.3).
    ///
    /// Serializes against the projection workers via `commit_gate` so the
    /// re-quantize sees a totally-ordered history, exactly like the at-pin
    /// commit. Publishes a `MeanVecRecomputed { trigger: Manual }` event
    /// only after the transaction is durable. No-op-safe on a non-MC
    /// identity (returns `EmbedderNotConfigured` rather than corrupting an
    /// un-centered workspace).
    #[cfg(feature = "operator")]
    pub fn recompute_mean(&self) -> Result<MeanRecomputeReport, EngineError> {
        self.ensure_open()?;
        let identity = self.runtime_embedder_identity.clone();
        if !identity_requires_mean_centering(&identity) {
            return Err(EngineError::EmbedderNotConfigured);
        }
        let report = {
            // Hold the commit gate for the whole recompute so no projection
            // worker commit interleaves with the re-quantize.
            let _gate = self
                .projection_runtime
                .shared
                .commit_gate
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
            let connection = connection.as_mut().ok_or(EngineError::Closing)?;
            dependency_closure::maintain_before_writer(connection)?;
            let tx = connection
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|_| EngineError::Storage)?;
            dependency_closure::guard_no_pending_physical(&tx)?;
            #[cfg(debug_assertions)]
            let fail = self
                .projection_runtime
                .shared
                .force_recompute_failure
                .swap(false, Ordering::SeqCst);
            #[cfg(not(debug_assertions))]
            let fail = false;
            let report = recompute_mean_in_tx_inner(&tx, &identity, fail)?;
            tx.commit().map_err(|_| EngineError::Storage)?;
            report
        };
        // Post-durable-commit publish.
        if let Ok(mut events) = self.projection_runtime.shared.pending_events.lock() {
            events.push(EmbedderEvent::MeanVecRecomputed {
                dim: report.dim,
                doc_count: report.doc_count_requantized,
                trigger: MeanRecomputeTrigger::Manual,
            });
        }
        Ok(report)
    }
}
