use super::*;

/// 0.8.18 Slice 5 (#5 vector-equivalence probe) — parse the committed 45-probe
/// fixture into an ordered `Vec<&str>` (one probe per non-empty, non-`#`-comment
/// line). Order is stable so `probe_ordinal` is deterministic across opens.
fn vector_equivalence_probes() -> Vec<&'static str> {
    VECTOR_EQUIVALENCE_PROBE_FIXTURE
        .lines()
        .map(str::trim_end)
        .filter(|line| {
            let t = line.trim_start();
            !t.is_empty() && !t.starts_with('#')
        })
        .collect()
}

/// 0.8.18 Slice 5 — outcome of the open-time #5 self-check.
pub(crate) struct VectorEquivalenceOutcome {
    pub(crate) dense_disabled: bool,
    pub(crate) reason: Option<String>,
}

/// A dense runtime can schedule, repair, and report readiness only when an
/// embedder is attached and the open-time equivalence guard accepted it.
pub(crate) fn usable_dense_runtime(embedder: Option<&dyn Embedder>, dense_disabled: bool) -> bool {
    embedder.is_some() && !dense_disabled
}

/// 0.8.18 Slice 5 — embed one probe through the engine's bounded dispatcher.
/// The probe runs at open time on the writer connection before projection workers
/// spawn. A caller-supplied embedder that panics, errors, times out, or returns
/// a wrong-dimension vector yields `None`; the callers then fail-safe — a `None` at population
/// or check time means the vector arm cannot be established/verified, so dense is
/// REFUSED (`dense_disabled=true`), never silently served. `Engine::open` still
/// succeeds (no wedge; ADR-0.6.0 Invariant-5 posture, mirrored open-side).
fn probe_embed(embedder: &EmbedDispatcher, text: &str, dimension: usize) -> Option<Vec<f32>> {
    match embedder.submit_text(text.to_owned()).and_then(EmbedReply::wait) {
        Ok(EmbedOutput::One(vector)) if vector.len() == dimension => Some(vector),
        _ => None,
    }
}

/// 0.8.18 Slice 5 (#5 vector-equivalence probe KEYSTONE) — the open-time
/// self-check. Per `dev/design/0.8.18-slice-0-vector-equivalence-publish-design.md`
/// §U1 + `dev/adr/ADR-0.8.18-vector-equivalence-self-check.md`.
///
/// Runs AFTER open-time mean-recovery/requantize + `ensure_vector_partition`
/// (U1-b) so it reads the FINAL live `mean_vec`. Two paths:
///
///  - **First accepted vector arm** (probe table empty): re-embed the 45
///    committed probes with the LIVE embedder and verify the in-memory
///    **UN-centered f32 reference vectors** + embedder identity (R-VEQ-1)
///    before persisting them. Store f32 ONLY — the P1 bits are NEVER persisted
///    (U1-d). A rejected prospective arm writes neither the baseline nor cache.
///  - **Subsequent open** (probe table populated): re-embed the 45 probes and
///    assert BOTH dense-pipeline representations against the stored references:
///    **(P1)** the Phase-1 mean-centered `embedding_bin` sign-flip count via the
///    SAME `vec_quantize_binary(sign(x − mean_vec))` path as
///    `build_vector_phase1_sql` (floor = 0, exact); **(P2)** the un-centered
///    Phase-2 L2 (`vec_distance_l2` semantics) within `VECTOR_EQUIVALENCE_L2_EPSILON`.
///    Divergence beyond EITHER floor ⇒ `dense_disabled=true` (R-VEQ-2/3).
///
/// Mean-centering is gated by `identity_requires_mean_centering(identity)` ∧
/// `mean_pinned`, applied symmetrically to reference + reembed (un-centered
/// fallback otherwise; NoopEmbedder no-op) — R-VEQ-3c.
///
/// Fail-SAFE, never fail-open (0.8.18 Slice 5 fix-1, DEFECT #1): any inability to
/// RUN or VERIFY the probe — a probe embed that panics/errors/returns wrong-dim, a
/// malformed/missing reference row, an unreadable pinned mean, or a
/// `vec_quantize_binary`/L2 SQL failure — yields `dense_disabled=true` with a clear
/// reason (refuse the un-verifiable dense/fused arm; the text-only/FTS path still
/// serves). `Engine::open` still SUCCEEDS (never wedges on a panicking caller
/// embedder). The distinct-identity cross-vendor refusal (`check_embedder_profile`)
/// remains the PRIMARY gate; this probe is ADDITIVE-ONLY (R-VEQ-5), but on the
/// vector arm it fails CLOSED, not open — same-identity backend drift on an
/// un-verifiable arm is exactly what #5 must catch (R-VEQ-4 "loud typed refuse,
/// never silent").
pub(crate) fn run_vector_equivalence_probe(
    connection: &Connection,
    embedder: Option<&EmbedDispatcher>,
    identity: &EmbedderIdentity,
    mean_pinned: bool,
    prospective_dense_arm: bool,
) -> VectorEquivalenceOutcome {
    let not_disabled = VectorEquivalenceOutcome { dense_disabled: false, reason: None };

    // No runtime embedder means no dense arm to guard (EmbedderChoice::None).
    // The probe is inert; dense writes/queries already fail with
    // EmbedderNotConfigured.
    let Some(embedder) = embedder else { return not_disabled };

    // Gate: the probe engages once the workspace has either a REGISTERED vector
    // kind (`_fathomdb_vector_kinds` non-empty) or a durable prospective vector
    // declaration that the safe boot graft would otherwise enrol. A workspace
    // with neither has no dense arm to guard, so the probe does ZERO embed work
    // at that open — this keeps `Engine::open` free of the 45-probe re-embed on
    // empty/vector-less workspaces (and inert for the pathological
    // single-session hang/panic embedder tests, which register their kind AFTER
    // open and never reopen).
    //
    // fix-1 DEFECT #4 — the baseline is established at OPEN, at the first open
    // where a vector kind already exists (population path below). This covers BOTH:
    //   (b) the v18→v19 UPGRADE with pre-existing vector kinds: the baseline is
    //       captured here, at the first v19 open, from the identity-matched
    //       embedder (identity is already gated by `check_embedder_profile`, so the
    //       baseline is the same *claimed* embedder; future backend drift is caught);
    //   (a) a vector kind registered POST-OPEN in a prior session: the baseline is
    //       captured at the NEXT open (this gate + population), again identity-gated.
    // It is deliberately NOT captured in the registering session's write path: a
    // write must NEVER block on the embedder (the async-projection invariant —
    // `ac_029_canonical_writes_complete_under_projection_stall` and the PR-9 embed
    // watchdog/thread-leak bounds), and 45 synchronous probe embeds there would
    // violate it and hang/degrade under a stalling embedder. Serving vector queries
    // in the registering session is SAFE regardless: the serving backend IS the
    // backend that built those vectors, so there is nothing to diverge from. The
    // residual — a same-*identity* backend that drifted between the registering
    // session and the next open is not retroactively caught — is IDENTICAL to the
    // accepted upgrade residual (R-VEQ-5 additive-only; U3 same-identity candle
    // CPU↔CUDA = 0/17280). See `dev/design/0.8.18-slice-5-vector-equivalence-probe.md`.
    let vector_kind_registered: bool = connection
        .query_row("SELECT EXISTS(SELECT 1 FROM _fathomdb_vector_kinds)", [], |r| r.get(0))
        .unwrap_or(false);
    if !vector_kind_registered && !prospective_dense_arm {
        return not_disabled;
    }

    // A cold declared arm must earn its durable enrolment only after a successful
    // probe. Its preflight is read-only until acceptance: a refusal must leave no
    // reference baseline or verdict-cache mutation behind for a later open.
    let prospective_preflight = prospective_dense_arm && !vector_kind_registered;
    match probe_populate_or_check(
        connection,
        embedder,
        identity,
        mean_pinned,
        prospective_preflight,
    ) {
        Ok(()) => not_disabled,
        Err(reason) => VectorEquivalenceOutcome { dense_disabled: true, reason: Some(reason) },
    }
}

/// 0.8.18 Slice 5 — either PERSIST the baseline (probe table empty) or CHECK
/// against it (probe table populated). `Err(reason)` ⇒ refuse the dense arm
/// (`dense_disabled=true`); `Ok(())` ⇒ dense served. Fail-SAFE throughout.
fn probe_populate_or_check(
    connection: &Connection,
    embedder: &EmbedDispatcher,
    identity: &EmbedderIdentity,
    mean_pinned: bool,
    prospective_preflight: bool,
) -> Result<(), String> {
    let probes = vector_equivalence_probes();
    if probes.is_empty() {
        // Fail-SAFE: the compiled-in probe fixture is empty ⇒ nothing to verify
        // the vector arm against. (Defensive; the fixture is drift-guarded
        // non-empty at 45 probes.)
        return Err(
            "vector-equivalence probe fixture is empty; cannot verify the dense arm".to_string()
        );
    }

    let existing: i64 = connection
        .query_row("SELECT COUNT(*) FROM _fathomdb_embed_probe", [], |r| r.get(0))
        .map_err(|e| format!("could not read the probe reference table: {e}; cannot verify"))?;

    if existing == 0 {
        let pending_baseline = collect_probe_baseline(embedder, identity, &probes)?;
        if prospective_preflight {
            // A declaration alone is not an enrolled arm. Verify the in-memory
            // reference set first; on every refusal path this has performed reads
            // and embed calls only. Persist both the baseline and its cache marker
            // only after the verdict has been accepted.
            let cache_update = probe_check_stored_baseline(
                connection,
                embedder,
                identity,
                mean_pinned,
                &probes,
                &pending_baseline,
            )?;
            persist_probe_baseline(connection, &pending_baseline)?;
            if let Some(fingerprint) = cache_update {
                record_probe_verification(connection, &fingerprint);
            }
            Ok(())
        } else {
            // A registered arm preserves the established Slice 5 behavior:
            // persist its first baseline, then confirm that durable baseline
            // before serving dense.
            persist_probe_baseline(connection, &pending_baseline)?;
            probe_check_against_baseline(connection, embedder, identity, mean_pinned, &probes, true)
        }
    } else {
        // A prospective refusal must preserve even a stale prior marker: the
        // proposed arm was never accepted, so it cannot mutate durable state.
        probe_check_against_baseline(
            connection,
            embedder,
            identity,
            mean_pinned,
            &probes,
            !prospective_preflight,
        )
    }
}

/// Capture the 45 un-centered f32 reference vectors as rows not yet made durable.
/// This collection is deliberately side-effect free so a prospective dense arm can
/// be refused without changing the workspace it was merely considering joining.
fn collect_probe_baseline(
    embedder: &EmbedDispatcher,
    identity: &EmbedderIdentity,
    probes: &[&str],
) -> Result<Vec<StoredProbeRow>, String> {
    let dimension = identity.dimension as usize;
    let mut rows = Vec::with_capacity(probes.len());
    for (ordinal, probe) in probes.iter().enumerate() {
        match probe_embed(embedder, probe, dimension) {
            Some(vector) => rows.push((
                ordinal as i64,
                (*probe).to_string(),
                encode_vector_blob(&vector),
                identity.name.clone(),
                identity.revision.clone(),
                identity.dimension as i64,
            )),
            None => {
                return Err(format!(
                    "embedder failed to produce a reference vector for probe {ordinal}; \
                     cannot establish a vector-equivalence baseline (dense arm refused)"
                ));
            }
        }
    }
    Ok(rows)
}

/// Persist a complete accepted baseline atomically. A failed transaction leaves no
/// partial reference set for a future open to trust.
fn persist_probe_baseline(connection: &Connection, rows: &[StoredProbeRow]) -> Result<(), String> {
    let tx = connection
        .unchecked_transaction()
        .map_err(|e| format!("could not open the probe-baseline transaction: {e}"))?;
    for (ordinal, probe, blob, name, revision, dim) in rows {
        tx.execute(
            "INSERT OR REPLACE INTO _fathomdb_embed_probe(
                 probe_ordinal, probe_text, reference_vec,
                 embedder_name, embedder_revision, dim
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![ordinal, probe, blob, name, revision, dim],
        )
        .map_err(|e| format!("could not persist the probe baseline: {e}"))?;
    }
    tx.commit().map_err(|e| format!("could not commit the probe baseline: {e}"))?;
    Ok(())
}

/// 0.8.20 Slice 22 (TC-68) — one row of the stored probe baseline:
/// `(probe_ordinal, probe_text, reference_vec, embedder_name, embedder_revision, dim)`.
type StoredProbeRow = (i64, String, Vec<u8>, String, String, i64);

/// 0.8.20 Slice 22 (TC-68) — length-prefixed field feed for the verdict
/// fingerprint. The `u64` length prefix makes the concatenation UNAMBIGUOUS: two
/// different input tuples can never produce the same byte stream by sliding a
/// delimiter (e.g. name `"ab"` + revision `"c"` vs name `"a"` + revision `"bc"`).
fn hash_fingerprint_field(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

/// 0.8.20 Slice 22 (TC-68) — the **embedder-identity fingerprint** the cached
/// equivalence verdict is keyed on: a SHA-256 over EVERY input the probe's verdict
/// depends on. Two opens sharing a fingerprint would, by construction, compute the
/// same P1/P2 answer, so the second may reuse the first's.
///
/// The inputs, and why each is load-bearing:
///
/// - **the recipe tag** ([`VECTOR_EQUIVALENCE_FINGERPRINT_RECIPE`]) — bumping it
///   invalidates every cached verdict in the field at once;
/// - **`identity.{name, revision, dimension}`** — the nominal embedder. This is
///   *defence in depth only*: `check_embedder_profile` already REFUSES the open
///   with `EmbedderIdentityMismatch`/`EmbedderDimensionMismatch` before the probe
///   is reached, so an identity change is never observed here in practice;
/// - **the live pinned `mean_vec`** (and whether centering is applied at all) —
///   this one is NOT optional. P1 quantizes through
///   `vec_quantize_binary(sign(x − mean_vec))`, so rewriting the pinned mean
///   changes the verdict *for the same embedder and the same baseline*. A
///   fingerprint over the identity triple alone would be stale by construction,
///   because open-time mean-recovery/requantize and the operator `recompute_mean`
///   verb both rewrite it;
/// - **the committed probe fixture** — a cached verdict computed over a different
///   probe set means nothing. (`vector_equivalence_probe_fixture_drift.rs` does
///   NOT make this redundant: it pins the engine's copy equal to the embedder
///   crate's copy — it guards COPY drift between two committed files, not the
///   fixture's content across releases. The stored-baseline completeness check
///   below does fail closed first on a fixture edit, so this input is belt-and-
///   braces rather than the only guard; it is one hash of a ~3 KB constant.);
/// - **both D4 floors** — a verdict that passed under a loose ε must not be
///   inherited by a build that tightened it;
/// - **the STORED baseline rows, reference blobs included** — the 0.8.18 fix-2
///   completeness check pins each row's shape (count, ordinal, text, blob LENGTH,
///   identity) but not the blob CONTENT, which the re-embed comparison used to
///   catch. Hashing the blobs keeps that external-tamper closure intact at
///   negligible cost (~69 KB of SHA-256 against 45 model invocations).
fn probe_verification_fingerprint(
    identity: &EmbedderIdentity,
    mean_vec: Option<&[f32]>,
    stored: &[StoredProbeRow],
) -> String {
    let mut hasher = Sha256::new();
    hash_fingerprint_field(&mut hasher, VECTOR_EQUIVALENCE_FINGERPRINT_RECIPE.as_bytes());
    hash_fingerprint_field(&mut hasher, identity.name.as_bytes());
    hash_fingerprint_field(&mut hasher, identity.revision.as_bytes());
    hash_fingerprint_field(&mut hasher, &identity.dimension.to_le_bytes());
    match mean_vec {
        Some(mean) => {
            hash_fingerprint_field(&mut hasher, b"mean-centered");
            hash_fingerprint_field(&mut hasher, &encode_vector_blob(mean));
        }
        None => hash_fingerprint_field(&mut hasher, b"un-centered"),
    }
    hash_fingerprint_field(&mut hasher, VECTOR_EQUIVALENCE_PROBE_FIXTURE.as_bytes());
    hash_fingerprint_field(&mut hasher, &VECTOR_EQUIVALENCE_P1_FLIP_FLOOR.to_le_bytes());
    hash_fingerprint_field(&mut hasher, &VECTOR_EQUIVALENCE_L2_EPSILON.to_le_bytes());
    hash_fingerprint_field(&mut hasher, &(stored.len() as u64).to_le_bytes());
    for (ordinal, probe_text, reference_vec, name, revision, dim) in stored {
        hash_fingerprint_field(&mut hasher, &ordinal.to_le_bytes());
        hash_fingerprint_field(&mut hasher, probe_text.as_bytes());
        hash_fingerprint_field(&mut hasher, reference_vec);
        hash_fingerprint_field(&mut hasher, name.as_bytes());
        hash_fingerprint_field(&mut hasher, revision.as_bytes());
        hash_fingerprint_field(&mut hasher, &dim.to_le_bytes());
    }
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// 0.8.20 Slice 22 (TC-68) — is `fingerprint` the fingerprint under which the
/// probe last RAN and PASSED on this workspace?
///
/// Fail-SAFE against ACCIDENT (R-VEQ-4): **every** failure mode answers `false`,
/// which means "run the probe". A missing `_fathomdb_open_state` table, an absent
/// row, a non-TEXT value, a truncated or garbled value, a stale fingerprint, any
/// SQL error — none of them can be mistaken for a pass.
///
/// # What a `true` does and does not mean (fix-1, codex §9 round 2 [P1])
///
/// `true` means: **the fingerprint inputs are unchanged since *some* engine
/// recorded a pass.** It does NOT mean "this engine verified this backend", and it
/// cannot: the fingerprint is a SHA-256 over deterministic, publicly derivable DB
/// and build inputs, so an actor with write access to the file can compute the
/// current digest and write it here, skipping the 45-probe verification. This
/// marker is not — and cannot be — an authenticated attestation; an embedded
/// local-first engine holds no secret with which to authenticate one, and a salt
/// would be readable by the same actor.
///
/// The same actor also defeats the same arm through the **pre-slice** path, by
/// re-baselining `_fathomdb_embed_probe`'s `reference_vec` blobs to their drifted
/// backend's own output — the probe then runs in full and verifies the drifted
/// backend against itself. Measured by
/// `tests/tc68_probe_fingerprint_cache.rs::a_forged_stored_baseline_defeats_the_probe_even_when_it_fully_runs`
/// (marker deleted, all 45 embeds performed, dense still enabled), with the
/// un-forged control caught.
///
/// **That is the same actor, NOT the same cost, and fix-2 struck the claim that it
/// was.** Forging this marker needs only a publicly computable digest — usually the
/// value already sitting in the row. Re-baselining additionally needs the target
/// backend's 45 exact embeddings, encoded into every row. **So the cache IS a
/// cheaper bypass** for a writer of the database file.
///
/// What bounds it is the ruled residual, not this marker. A same-identity backend
/// drift moves no fingerprint input, so a marker recorded by an **honest** earlier
/// open already skips the probe and already serves the drifted backend, with no
/// forgery anywhere
/// (`residual_same_identity_backend_drift_is_not_caught_on_a_cached_open`). Forgery
/// adds capability only on an open where no valid marker exists for the *current*
/// fingerprint — and a digest is valid only for the state it was computed over, so
/// it stops working at the next change to any fingerprint input.
///
/// The equivalence probe is a **correctness self-check against backend drift, not
/// tamper evidence**; `dense_disabled` is not a tamper signal. Threat model, with
/// the concession and the bound: §8.4/§8.5 of
/// `dev/design/0.8.20-tc68-equivalence-probe-fingerprint-cache.md`.
fn probe_verification_is_cached(connection: &Connection, fingerprint: &str) -> bool {
    connection
        .query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key = ?1",
            [VECTOR_EQUIVALENCE_VERDICT_CACHE_KEY],
            |row| row.get::<_, String>(0),
        )
        .map(|cached| cached == fingerprint)
        .unwrap_or(false)
}

/// 0.8.20 Slice 22 (TC-68) — record that the probe RAN and PASSED under
/// `fingerprint`.
///
/// A write failure is deliberately SWALLOWED rather than turned into a verdict
/// failure. The arm has just been verified, so refusing dense because a marker
/// could not be persisted (read-only file, disk full) would be a false refusal;
/// and the consequence of the missing marker is simply that the next open re-runs
/// the probe — more work, never less. That is the fail-safe direction.
fn record_probe_verification(connection: &Connection, fingerprint: &str) {
    let _ = connection.execute(
        "INSERT INTO _fathomdb_open_state(key, value) VALUES(?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![VECTOR_EQUIVALENCE_VERDICT_CACHE_KEY, fingerprint],
    );
}

/// 0.8.20 Slice 22 (TC-68) — drop any cached verdict.
///
/// Called on every failure path of an already registered arm, so that arm cannot
/// carry a marker a later registered open might match. A prospective arm has not
/// joined durable state yet and deliberately preserves its snapshot on refusal.
fn clear_probe_verification(connection: &Connection) {
    let _ = connection.execute(
        "DELETE FROM _fathomdb_open_state WHERE key = ?1",
        [VECTOR_EQUIVALENCE_VERDICT_CACHE_KEY],
    );
}

/// 0.8.18 Slice 5 — SUBSEQUENT open: re-embed the 45 probes and assert BOTH
/// dense-pipeline representations against the stored references — **(P1)** the
/// mean-centered `embedding_bin` sign-flip count (floor 0, exact) and **(P2)** the
/// un-centered Phase-2 L2 (within `VECTOR_EQUIVALENCE_L2_EPSILON`).
///
/// 0.8.20 Slice 22 (TC-68) — this wrapper adds the failure half of the verdict
/// cache for an already registered arm: ANY `Err` drops the cached marker, so a
/// workspace that could not be verified never leaves a stale "verified" marker
/// behind for a later registered open to match. A prospective preflight passes
/// `false` for `clear_cache_on_error`: it has not joined the durable arm yet, so
/// rejection must remain globally mutation-free.
fn probe_check_against_baseline(
    connection: &Connection,
    embedder: &EmbedDispatcher,
    identity: &EmbedderIdentity,
    mean_pinned: bool,
    probes: &[&str],
    clear_cache_on_error: bool,
) -> Result<(), String> {
    let outcome = load_stored_probe_baseline(connection).and_then(|stored| {
        probe_check_stored_baseline(connection, embedder, identity, mean_pinned, probes, &stored)
    });
    match outcome {
        Ok(Some(fingerprint)) => {
            record_probe_verification(connection, &fingerprint);
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(reason) => {
            if clear_cache_on_error {
                clear_probe_verification(connection);
            }
            Err(reason)
        }
    }
}

fn load_stored_probe_baseline(connection: &Connection) -> Result<Vec<StoredProbeRow>, String> {
    let mut stmt = connection
        .prepare(
            "SELECT probe_ordinal, probe_text, reference_vec, embedder_name, embedder_revision, dim \
             FROM _fathomdb_embed_probe ORDER BY probe_ordinal",
        )
        .map_err(|e| format!("could not read the stored probe references: {e}; cannot verify"))?;
    stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Vec<u8>>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, i64>(5)?,
        ))
    })
    .and_then(|rows| rows.collect::<rusqlite::Result<Vec<_>>>())
    .map_err(|e| format!("could not read the stored probe references: {e}; cannot verify"))
}

/// 0.8.18 Slice 5 — the check proper. Fail-SAFE (fix-1 DEFECT #1): a probe embed
/// that panics/errors/returns wrong-dim, a malformed/missing reference row, an
/// unreadable pinned mean, or a `vec_quantize_binary`/L2 SQL failure each ⇒ `Err`
/// (cannot verify ⇒ refuse dense), never a silent skip-and-serve.
///
/// fix-2 (DEFECT #1 residual): BEFORE the divergence check, the STORED baseline is
/// validated to be EXACTLY the committed probe set — the expected row count, a
/// contiguous 0-based `probe_ordinal` per committed probe, each `probe_text` equal
/// to the committed fixture text at that ordinal, each `reference_vec` a well-formed
/// `4 * dim` f32 blob, and the stored embedder identity/dim matching the current
/// one. This closes the partial-baseline / external-tamper fail-open (a 44-of-45
/// table, or a re-attributed/mangled row, previously verified only the rows present
/// or re-embedded a tampered `probe_text` against itself). Any mismatch ⇒ `Err`.
///
/// 0.8.20 Slice 22 (TC-68) — the 45 re-embeds are CACHED against
/// [`probe_verification_fingerprint`]. Note WHERE the cache check sits: after the
/// mean resolution and after the fix-2 completeness validation, before the
/// re-embed loop. That split is deliberate — everything cheap keeps running on
/// EVERY open (so a short, re-attributed, mangled or fixture-mismatched baseline
/// still fails closed immediately), and only the expensive part, the 45 model
/// invocations, is skipped. The residual this buys is recorded in
/// `dev/design/0.8.20-tc68-equivalence-probe-fingerprint-cache.md`.
fn probe_check_stored_baseline(
    connection: &Connection,
    embedder: &EmbedDispatcher,
    identity: &EmbedderIdentity,
    mean_pinned: bool,
    probes: &[&str],
    stored: &[StoredProbeRow],
) -> Result<Option<String>, String> {
    let dimension = identity.dimension as usize;

    // Resolve the live mean. Fail-SAFE: if centering is required + pinned but the
    // mean cannot be read, we cannot reproduce `embedding_bin` ⇒ refuse (P1
    // un-verifiable). NoopEmbedder / no-pin ⇒ un-centered on BOTH sides (R-VEQ-3c).
    let mean_vec = if identity_requires_mean_centering(identity) && mean_pinned {
        match read_pinned_mean_vec(connection, identity.dimension) {
            Ok(Some(mean)) => Some(mean),
            Ok(None) => {
                return Err("mean-centering is required and pinned but mean_vec is absent; \
                     cannot verify P1 (dense arm refused)"
                    .to_string());
            }
            Err(_) => {
                return Err(
                    "could not read the pinned mean_vec; cannot verify P1 (dense arm refused)"
                        .to_string(),
                );
            }
        }
    } else {
        None
    };

    // fix-2 (DEFECT #1 residual) — COMPLETENESS validation of the STORED baseline.
    // `COUNT(*) > 0` is NOT proof of a complete, trustworthy baseline: a partially
    // populated or externally-tampered probe table (44 of 45 rows, a gap/dupe in the
    // ordinals, a mangled reference blob, a mismatched probe_text, or a foreign
    // embedder identity) is UNVERIFIABLE stored state. The prior code re-embedded
    // the STORED probe_text and compared it to its OWN reference, so a tampered
    // probe_text verified against itself and a short table verified only the rows
    // present — both fail-OPEN. Atomic population stops the ENGINE from writing a
    // partial set; this closes external corruption, a manual edit, and a future
    // migration bug the engine did not author. Any mismatch ⇒ fail CLOSED (dense
    // refused); the text-only/FTS path still serves. The stored baseline must be
    // EXACTLY the committed probe set, in order, under the current identity.
    if stored.len() != probes.len() {
        return Err(format!(
            "the probe reference table has {} rows but the committed fixture defines {}; \
             the stored baseline is incomplete or corrupt — cannot verify the dense arm (refused)",
            stored.len(),
            probes.len()
        ));
    }
    for (idx, (ordinal, probe_text, ref_blob, name, revision, dim)) in stored.iter().enumerate() {
        // Contiguous 0-based ordinals, one per committed probe (no gaps/dupes).
        if *ordinal != idx as i64 {
            return Err(format!(
                "probe reference ordinals are non-contiguous (row {idx} carries ordinal {ordinal}); \
                 the stored baseline is corrupt — cannot verify the dense arm (refused)"
            ));
        }
        // The stored text MUST be the committed fixture text at this ordinal —
        // otherwise a tampered probe_text re-embeds and verifies against ITSELF,
        // masking drift (the exact fail-open this fix closes).
        if probe_text != probes[idx] {
            return Err(format!(
                "probe reference {ordinal} text does not match the committed fixture; \
                 the stored baseline is tampered or corrupt — cannot verify the dense arm (refused)"
            ));
        }
        // Well-formed f32[dim] reference (4*dim little-endian bytes).
        if ref_blob.len() != dimension * 4 {
            return Err(format!(
                "probe reference {ordinal} is malformed (len {} != {}); \
                 cannot verify the dense arm (refused)",
                ref_blob.len(),
                dimension * 4
            ));
        }
        // The stored embedder identity/dim must match the CURRENT expected identity
        // (defence-in-depth beyond `check_embedder_profile`: catches a baseline row
        // re-attributed to a foreign embedder by external edit/migration).
        if *dim != identity.dimension as i64
            || name != &identity.name
            || revision != &identity.revision
        {
            return Err(format!(
                "probe reference {ordinal} was captured under embedder {name}/{revision}/dim={dim} \
                 but the current embedder is {}/{}/dim={}; the stored baseline does not match — \
                 cannot verify the dense arm (refused)",
                identity.name, identity.revision, identity.dimension
            ));
        }
    }

    // 0.8.20 Slice 22 (TC-68) — the CACHE gate. Everything above this line ran on
    // this open and still fails closed; everything below it is the 45 model
    // invocations that made `Engine::open` cost a flat 45 embeds FOREVER (measured
    // at `94bb33ef`: 0 with no enrolled kind, 90 on the one-time population open,
    // 45 on every open thereafter — independent of the enrolled-kind count, since
    // the probe gate is an `EXISTS` and the body never iterates kinds).
    //
    // If the probe already RAN and PASSED under this exact fingerprint, re-running
    // it is a pure re-computation of a known answer, so the verdict is reused.
    // Fail-SAFE: `probe_verification_is_cached` answers `false` for every failure
    // mode — missing table, absent row, garbled value, SQL error — so an
    // unreadable cache RUNS the probe, it never short-circuits to trusting it.
    let fingerprint = probe_verification_fingerprint(identity, mean_vec.as_deref(), stored);
    if probe_verification_is_cached(connection, &fingerprint) {
        return Ok(None);
    }

    let mut total_flips: u64 = 0;
    let mut max_l2: f32 = 0.0;
    let mut worst_probe: Option<String> = None;

    for (ordinal, probe_text, ref_blob, _, _, _) in stored {
        let reference = decode_vector_blob(ref_blob);
        let reembed = probe_embed(embedder, probe_text, dimension).ok_or_else(|| {
            format!(
                "embedder failed/panicked re-embedding probe {ordinal}; \
                 cannot verify the dense arm (refused)"
            )
        })?;

        // (P2) un-centered L2 — `vec_distance_l2(embedding, vec_f32(query))`.
        let l2 = l2_distance(&reembed, &reference);
        if l2 > max_l2 {
            max_l2 = l2;
            worst_probe = Some(probe_text.to_string());
        }

        // (P1) mean-centered Phase-1 flip count — same
        // `vec_quantize_binary(sign(x − mean_vec))` path as build_vector_phase1_sql.
        let (ref_c, reembed_c) = match &mean_vec {
            Some(mean) => (subtract_mean(&reference, mean), subtract_mean(&reembed, mean)),
            None => (reference.clone(), reembed.clone()),
        };
        let ref_bits = quantize_binary_via_sql(connection, &ref_c).ok_or_else(|| {
            format!("vec_quantize_binary SQL failed for probe {ordinal}; cannot verify P1")
        })?;
        let reembed_bits = quantize_binary_via_sql(connection, &reembed_c).ok_or_else(|| {
            format!("vec_quantize_binary SQL failed for probe {ordinal}; cannot verify P1")
        })?;
        total_flips = total_flips.saturating_add(hamming_bytes(&ref_bits, &reembed_bits));
    }

    let p1_tripped = total_flips > VECTOR_EQUIVALENCE_P1_FLIP_FLOOR;
    let p2_tripped = max_l2 > VECTOR_EQUIVALENCE_L2_EPSILON;
    if p1_tripped || p2_tripped {
        let probe_hint = worst_probe.as_deref().unwrap_or("<unknown>");
        return Err(format!(
            "P1 mean-centered embedding_bin flips={total_flips} (floor={VECTOR_EQUIVALENCE_P1_FLIP_FLOOR}), \
             P2 max un-centered L2={max_l2:.3e} (epsilon={VECTOR_EQUIVALENCE_L2_EPSILON:.3e}); \
             worst probe {probe_hint:?}"
        ));
    }

    // The caller persists this accepted fingerprint only after its enclosing arm
    // has become durable. That ordering keeps a rejected prospective preflight
    // entirely read-only while preserving cache behavior for registered arms.
    Ok(Some(fingerprint))
}

/// 0.8.18 Slice 5 — un-centered Euclidean (L2) distance, matching the
/// `vec_distance_l2` semantics used by the Phase-2 rerank.
fn l2_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y) * (x - y)).sum::<f32>().sqrt()
}
