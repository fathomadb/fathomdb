use super::*;

/// 0.8.20 Slice 10b (R-20-RV / R-20-NV) — the **read view**: the single knob
/// that decides which `canonical_nodes` rows a read verb may see.
///
/// Every field is a *relaxation*: `ReadView::default()` is the STRICT view and
/// compiles to exactly the predicates the five read verbs carried before this
/// slice (`superseded_at IS NULL AND state = 'active'`), so the default read
/// path is behaviourally unchanged. Flags compose INDEPENDENTLY — each one
/// drops exactly one conjunct and no other.
///
/// The view is applied UNIFORMLY by [`Engine::read_get`],
/// [`Engine::read_get_many`], [`Engine::read_list`],
/// [`Engine::read_list_filter`] and [`Engine::graph_neighbors`] — and, inside
/// `graph_neighbors`, at EVERY position of EVERY direction's recursive CTE
/// (anchor, recursive join, final projection), so a relaxation cannot silently
/// apply on one traversal position and not another.
///
/// # World-time only
///
/// `valid_as_of` selects along the **world-time** (validity) axis only.
/// Transaction-time / `history_as_of` is explicitly OUT OF SCOPE — this type
/// deliberately has no way to ask "what did the database believe at time T".
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReadView {
    /// Relax `superseded_at IS NULL` — include superseded (historical) versions
    /// of a row, not just the current one. `false` (default) keeps the shipped
    /// current-version-only behaviour.
    ///
    /// On the point-lookup verbs ([`Engine::read_get`] /
    /// [`Engine::read_get_many`]) a `logical_id` can now match several rows;
    /// the slot resolves DETERMINISTICALLY to the highest `write_cursor` (the
    /// most recent version). Use [`Engine::read_list`] to enumerate history.
    pub include_superseded: bool,

    /// Relax `state = 'active'` — include rows in a non-`active` lifecycle
    /// state (`pending` / `deleted` / `purged`). `false` (default) keeps the
    /// shipped active-only behaviour.
    pub include_inactive: bool,

    /// Relax the validity-window predicate ENTIRELY — return rows whatever
    /// their `[valid_from, valid_until)` window, ignoring `valid_as_of`.
    /// `false` (default) filters to rows valid at the selected instant.
    ///
    /// Note this is a NO-OP on any row with an unbounded (NULL/NULL) window,
    /// which is every row that predates schema step 22.
    pub include_out_of_window: bool,

    /// The instant (INTEGER epoch SECONDS, UTC) at which validity is evaluated.
    /// `None` (default) resolves to *now* at query time.
    ///
    /// This is the **`:now` seam**: whichever way it resolves, the instant is
    /// compiled as a BOUND PARAMETER, never a `datetime('now')` SQL literal —
    /// which is what makes node validity deterministically testable without
    /// clock games. (The shipped EDGE temporal filter still inlines
    /// `datetime('now')`; that path is untouched by this slice.)
    pub valid_as_of: Option<i64>,
}

impl ReadView {
    /// The instant to bind for the validity predicate, or `None` when the view
    /// relaxes validity entirely (in which case no `:now` parameter is emitted
    /// and none must be bound).
    pub(crate) fn now_param(&self) -> Option<i64> {
        if self.include_out_of_window {
            return None;
        }
        Some(self.valid_as_of.unwrap_or_else(current_epoch_seconds))
    }

    /// The existence conjunct for node-table `alias`. Each flag drops exactly
    /// one conjunct; the strict view reproduces the pre-slice predicate pair
    /// verbatim. Always begins with ` AND ` (or is empty), so every call site
    /// must already have a preceding `WHERE` predicate.
    pub(crate) fn existence_sql(&self, alias: &str) -> String {
        let mut sql = String::new();
        if !self.include_superseded {
            sql.push_str(&format!(" AND {alias}.superseded_at IS NULL"));
        }
        if !self.include_inactive {
            sql.push_str(&format!(" AND {alias}.state = 'active'"));
        }
        sql
    }

    /// The validity conjunct for node-table `alias`, bound to positional
    /// parameter `?{now_idx}`. Empty when validity is relaxed.
    ///
    /// Encodes the HALF-OPEN window `[valid_from, valid_until)` with NULL
    /// meaning unbounded on that side — so a NULL/NULL row is valid at every
    /// instant and this conjunct never changes its visibility.
    pub(crate) fn validity_sql(&self, alias: &str, now_idx: usize) -> String {
        if self.include_out_of_window {
            return String::new();
        }
        format!(
            " AND ({alias}.valid_from IS NULL OR {alias}.valid_from <= ?{now_idx}) \
             AND ({alias}.valid_until IS NULL OR {alias}.valid_until > ?{now_idx})"
        )
    }

    /// The full node predicate (existence + validity) for `alias`. This is the
    /// ONE function every read site calls, so no site can drift from another.
    pub(crate) fn node_sql(&self, alias: &str, now_idx: usize) -> String {
        format!(
            "{}{}{}",
            self.existence_sql(alias),
            self.validity_sql(alias, now_idx),
            dependency_closure::read_eligibility_sql(
                alias,
                self.include_superseded,
                self.include_inactive,
                self.include_out_of_window,
                now_idx,
            )
        )
    }

    /// 0.8.20 Slice 15b fix-3 (F2) — resolve this view's validity instant ONCE
    /// and hand back a [`FrozenView`] that carries the resolved value.
    ///
    /// This is the ONLY constructor of a `FrozenView`, and therefore the only
    /// point on the search path where the wall clock is read.
    pub(crate) fn freeze(self) -> FrozenView {
        // TC-33: resolve the instant ONCE, unconditionally, and derive both
        // axes from it. `valid_as_of.unwrap_or_else(current_epoch_seconds)` is
        // exactly what `now_param()` computes, so the clock is read the same
        // number of times as before on every path that reads it at all.
        let resolved = self.valid_as_of.unwrap_or_else(current_epoch_seconds);
        let now = if self.include_out_of_window { None } else { Some(resolved) };
        FrozenView { view: self, now, edge_now: resolved }
    }

    /// 0.8.20 Slice 15b fix-2 — the `search` path honours the VALIDITY axis of a
    /// `ReadView` and refuses the EXISTENCE axis. See [`Engine::search_view`] for
    /// why refusing beats silently ignoring.
    pub(crate) fn reject_existence_relaxation_on_search(&self) -> Result<(), EngineError> {
        let relaxed = match (self.include_superseded, self.include_inactive) {
            (true, true) => "include_superseded + include_inactive",
            (true, false) => "include_superseded",
            (false, true) => "include_inactive",
            (false, false) => return Ok(()),
        };
        Err(EngineError::InvalidArgument {
            msg: format!(
                "ReadView.{relaxed} is not supported on the search path; search hydrates from \
                 projection indexes that are not version-complete, so only the validity axis \
                 (valid_as_of / include_out_of_window) is honoured. Use read_list for history."
            ),
        })
    }
}

/// 0.8.20 Slice 10b (R-20-NV) — one node that crossed a validity boundary
/// inside the interrogated interval, as reported by
/// [`Engine::crossed_boundary_since`].
///
/// A node can cross BOTH boundaries in the same interval (a window that opened
/// and closed inside it), so the two fields are independent `Option`s rather
/// than one enum.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundaryCrossing {
    /// The node that crossed.
    pub node: NodeRecord,
    /// `Some(valid_from)` when the node BECAME VALID inside the interval.
    pub became_valid_at: Option<i64>,
    /// `Some(valid_until)` when the node BECAME INVALID inside the interval.
    pub became_invalid_at: Option<i64>,
}

/// 0.8.20 Slice 15b fix-3 (F2) — a [`ReadView`] whose validity instant has
/// ALREADY been resolved, produced only by [`ReadView::freeze`].
///
/// R-20-NV requires `:now` to bind ONCE PER QUERY — not per row, and not per
/// ARM. The multi-arm search path made that easy to violate: each arm held a
/// `ReadView` and could call `now_param()`, which for the default view
/// (`valid_as_of == None`) reads the wall clock. Two arms, two instants, and a
/// query straddling a validity boundary gets nondeterministic membership.
///
/// The fix is TYPE-LEVEL rather than a comment asking future arms to behave:
/// the instant is resolved once at the top of `read_search_in_tx` and every arm
/// receives a `FrozenView`, which stores the resolved value in `now` and has NO
/// path back to the clock. An arm cannot re-resolve the instant because it
/// never holds anything that could — the failure mode is unreachable, not
/// merely discouraged.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FrozenView {
    /// The underlying view — consulted for SQL SHAPE only (which conjuncts to
    /// emit), never to re-resolve the instant.
    pub(crate) view: ReadView,
    /// The instant resolved at freeze time. `None` ⇔ the view relaxes validity
    /// entirely, in which case no conjunct is emitted and nothing is bound.
    pub(crate) now: Option<i64>,
    /// TC-33 — the instant EDGE validity is evaluated at. Always present.
    ///
    /// The EXISTENCE-relaxation flag `include_out_of_window` belongs to the NODE
    /// validity axis and does NOT relax edge recency: an edge invalidated in the
    /// past stays excluded regardless. So this is the resolved instant even when
    /// `now` is `None`, and it is resolved from the SAME clock read.
    pub(crate) edge_now: i64,
}

impl FrozenView {
    /// The instant to bind, resolved at freeze time. Unlike
    /// [`ReadView::now_param`] this is a stored value: calling it a second time
    /// cannot yield a different answer, and it never touches the clock.
    pub(crate) fn now_param(&self) -> Option<i64> {
        self.now
    }

    /// TC-33 — the instant to bind for the EDGE-validity conjunct
    /// ([`edge_validity_sql`]). Frozen, like [`FrozenView::now_param`].
    ///
    /// Honouring `valid_as_of` here is what finally UNIFIES the node and edge
    /// temporal axes: step 22 recorded "the shipped EDGE path still inlines
    /// `datetime('now')`" as the reason they could not be unified. For the
    /// DEFAULT view (`valid_as_of == None`) this is the wall clock, i.e. exactly
    /// the pre-TC-33 behaviour.
    pub(crate) fn edge_now(&self) -> i64 {
        self.edge_now
    }

    /// The validity conjunct — delegated to the one generator every read site
    /// shares, so the search arms cannot drift from the five read verbs.
    pub(crate) fn validity_sql(&self, alias: &str, now_idx: usize) -> String {
        self.view.validity_sql(alias, now_idx)
    }

    pub(crate) fn node_sql(&self, alias: &str, now_idx: usize) -> String {
        format!(
            "{}{}{}",
            self.view.existence_sql(alias),
            self.view.validity_sql(alias, now_idx),
            dependency_closure::read_eligibility_sql(
                alias,
                self.view.include_superseded,
                self.view.include_inactive,
                self.view.include_out_of_window,
                now_idx,
            )
        )
    }
}

/// 0.8.20 Slice 15b fix-3 (F2) — how many times [`current_epoch_seconds`] has
/// been called in this process. Test-only observation; see
/// [`clock_reads_for_test`].
static CLOCK_READS: AtomicU64 = AtomicU64::new(0);

/// Test seam — the process-wide count of wall-clock reads on the validity path.
/// Kept OFF the governed surface (`#[doc(hidden)]`, `_for_test`), mirroring the
/// sanctioned `set_vector_stage_only_for_test` / `vector_phase1_sql_for_test`
/// pattern; it is never re-exported from the `fathomdb` facade.
///
/// The counter is PROCESS-WIDE, so a test asserting on a delta must hold a
/// lock that excludes every other clock-reading test in its binary (test
/// binaries are separate processes, so only intra-binary contention matters).
/// `slice15b_search_validity_recall.rs` does this with a file-local mutex.
#[doc(hidden)]
#[must_use]
pub fn clock_reads_for_test() -> u64 {
    CLOCK_READS.load(Ordering::Relaxed)
}

/// Wall-clock now as INTEGER epoch SECONDS (UTC), saturating at 0 before the
/// Unix epoch. The single place the node-validity path reads the clock — and it
/// is read in RUST, then BOUND, never inlined into SQL as `datetime('now')`.
pub(crate) fn current_epoch_seconds() -> i64 {
    // 0.8.20 Slice 15b fix-3 (F2) — meter every wall-clock read on the validity
    // path. R-20-NV requires `:now` to bind ONCE PER QUERY (not per row, not per
    // ARM): if two arms of one query each resolve *now*, a query that straddles
    // a validity boundary can have its arms disagree about which side they are
    // on. That is invisible to a result-shape assertion and unreachable by a
    // deterministic test — you cannot assert on a race. Counting the reads makes
    // the property testable WITHOUT racing the clock, and keeps failing for any
    // arm added later that re-reads it. `Relaxed` is sufficient: the counter is
    // an observation, never a synchronization point.
    CLOCK_READS.fetch_add(1, Ordering::Relaxed);
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// TC-33 — the edge-validity conjunct, bound to positional parameter
/// `?{now_idx}`. THE one generator for "is this edge valid at `:now`", so no
/// read site can drift from another (the same discipline
/// [`ReadView::validity_sql`] applies to node validity).
///
/// An edge is valid at `t` iff it has no invalid-time, or its invalid-time is
/// strictly in the future. `t_invalid` is INTEGER epoch seconds since step 23,
/// so this is a direct integer comparison — no `datetime()` conversion per row.
///
/// **`:now` is a BOUND PARAMETER, never `datetime('now')`.** Before TC-33 every
/// edge read site inlined `datetime('now')`, which made the predicate
/// non-deterministic, untestable, and re-evaluated per row; step 22's comment
/// flagged that as the reason node and edge validity could not be unified. They
/// are unified now.
///
/// Always begins with ` AND `, so every call site must already have a preceding
/// `WHERE` predicate.
pub(crate) fn edge_validity_sql(alias: &str, now_idx: usize) -> String {
    edge_validity_sql_for_view(alias, now_idx, &ReadView::default())
}

pub(crate) fn edge_validity_sql_for_view(alias: &str, now_idx: usize, view: &ReadView) -> String {
    format!(
        " AND ({alias}.t_invalid IS NULL OR {alias}.t_invalid > ?{now_idx}){}",
        dependency_closure::read_eligibility_sql(
            alias,
            view.include_superseded,
            view.include_inactive,
            view.include_out_of_window,
            now_idx,
        )
    )
}

/// TC-33 — parse one ISO-8601 timestamp to INTEGER epoch seconds using SQLite's
/// own date parser, via a BOUND parameter.
///
/// Returns `None` when SQLite cannot resolve the value — `strftime` yields SQL
/// NULL for junk (`'not a date'`, `''`, `'2020-13-45T99:99:99Z'`, a bare epoch
/// string, whitespace-padded input, non-ASCII digits) and a digit string for
/// anything it understands.
///
/// **Why SQLite and not a date crate:** there is no `chrono`/`time` dependency
/// anywhere in the workspace, and HITL directed this spelling rather than adding
/// one. The value is BOUND, never interpolated.
///
/// **This does not violate the inline-clock rule.** That rule forbids
/// `datetime('now')` / `strftime('%s','now')` — an inline CLOCK. Parsing a bound
/// user value is deterministic and reads no clock. The current instant still
/// comes from the bound `:now` seam ([`current_epoch_seconds`]).
///
/// The `CAST` matters: `strftime('%s', ...)` returns TEXT (a digit string), not
/// an integer, and the column is `typeof(...) = 'integer'`-checked.
///
/// # TC-33 fix-5 — strict ISO-8601 SHAPE gate before delegating to SQLite
///
/// `strftime('%s', ?)` alone is NOT an ISO-8601 validator: SQLite's date parser
/// is MORE lenient than the declared wire contract. A bare number is read as a
/// **Julian day** (`strftime('%s','2451545.0')` → `946728000`, i.e. year 2000)
/// and `strftime('%s','0')` resolves to a pre-year-0000 epoch — so non-ISO input
/// was ACCEPTED and stored as an unrelated instant despite the "hard-reject
/// ISO-8601" contract HITL ratified (2026-07-21). [`is_iso8601_shape`] runs
/// FIRST and returns `None` for anything that is not a strict ISO-8601
/// date/datetime shape, so the existing hard-reject path fires. The shape gate
/// does NOT replace SQLite's calendar math — a shape-valid but impossible date
/// (`2025-13-45T00:00:00Z`) still `None`s out via `strftime` and hard-rejects.
///
/// # TC-47 — the calendar-DATE ROUND-TRIP backstop (keystone terminal codex P2)
///
/// The shape gate checks FORMAT, not CALENDAR VALIDITY, and `strftime('%s', ?)`
/// does NOT fully validate the calendar: it **rolls over an impossible DAY**
/// rather than returning NULL — `strftime('%s','2025-02-30T00:00:00Z')` yields
/// the epoch for `2025-03-02`, and `2025-04-31` yields `2025-05-01`. So a
/// shape-valid Feb-30 would parse to a DIFFERENT instant than the provider
/// supplied, bypassing the hard-reject contract. (An impossible MONTH like
/// `2025-13-01`, and impossible TIMES like `25:00:00` / `:60` / `:61`, already
/// NULL out; only impossible DAYS roll over — that is the sole residue.)
///
/// The `WHERE` clause is the round-trip: the literal calendar DATE component of
/// the input (`substr(?1, 1, 10)` — the `YYYY-MM-DD` the shape gate guarantees is
/// present) must survive SQLite's own calendar math UNCHANGED. If it rolled over,
/// `strftime('%Y-%m-%d', substr(?1,1,10))` differs from the literal substring and
/// the `WHERE` yields zero rows => `query_row` -> `QueryReturnedNoRows` -> `None`
/// => the existing hard-reject fires. This is a superset of the shape gate: it
/// also rejects the TC-44 Julian string `2451545.0` (its `substr(1,10)` renders
/// to `2000-01-01`, not itself).
///
/// **Why the DATE component and not the raw string or the UTC-rendered instant:**
/// a raw-string or `unixepoch`-rendered comparison would FALSE-REJECT valid
/// equivalent forms. `Z` vs `+00:00`, a non-UTC offset like `+05:00`, date-only,
/// and fractional seconds are all valid and store the correct (offset-shifted)
/// epoch — but a UTC re-render shifts the wall clock, so its date can differ from
/// the input's literal date. Comparing ONLY the literal DATE field is
/// tz-INVARIANT (the offset never alters the input's own `YYYY-MM-DD` text) while
/// still catching every DAY rollover, because the rollover happens in the
/// calendar math BEFORE any offset is applied. **Pure SQL — no date crate.**
pub(crate) fn iso8601_to_epoch_seconds(connection: &Connection, raw: &str) -> Option<i64> {
    if !is_iso8601_shape(raw) {
        return None;
    }
    connection
        .query_row(
            "SELECT CAST(strftime('%s', ?1) AS INTEGER) \
             WHERE strftime('%Y-%m-%d', substr(?1, 1, 10)) IS substr(?1, 1, 10)",
            params![raw],
            |r| r.get::<_, Option<i64>>(0),
        )
        .ok()
        .flatten()
}

/// TC-33 fix-5 — strict ISO-8601 date/datetime SHAPE gate. Hand-rolled on ASCII
/// bytes (NO new dependency: the workspace has no `chrono`/`time`, and `regex`
/// is only a transitive dep of `jsonschema`, not a direct one — adding either as
/// a direct dep would violate the "no new dependency" constraint).
///
/// Accepts EXACTLY:
/// - `YYYY-MM-DD` (date only); optionally followed by
/// - a `T` **or** a single space separator, then `HH:MM:SS`; optionally followed
///   by `.fff` fractional seconds (one or more digits); optionally followed by
///   a zone: `Z`, or `±HH:MM`, or `±HHMM`.
///
/// Rejects bare numbers (`0`, `2451545.0`), partial junk, whitespace, non-ASCII
/// digits, and anything with trailing characters. All digit positions require
/// ASCII `0..=9` (`u8::is_ascii_digit`), so non-ASCII digit look-alikes cannot
/// slip through. This is a SHAPE check only — calendar validity (e.g. month 13,
/// day 45) is still enforced by SQLite's `strftime` after this gate passes.
pub(crate) fn is_iso8601_shape(s: &str) -> bool {
    let b = s.as_bytes();
    let d = |c: u8| c.is_ascii_digit();

    // Date: YYYY-MM-DD (exactly 10 bytes).
    if b.len() < 10 {
        return false;
    }
    if !(d(b[0])
        && d(b[1])
        && d(b[2])
        && d(b[3])
        && b[4] == b'-'
        && d(b[5])
        && d(b[6])
        && b[7] == b'-'
        && d(b[8])
        && d(b[9]))
    {
        return false;
    }
    if b.len() == 10 {
        return true; // date-only
    }

    // Separator (`T` or a single space) + time HH:MM:SS (indices 10..=18).
    if b[10] != b'T' && b[10] != b' ' {
        return false;
    }
    if b.len() < 19 {
        return false;
    }
    if !(d(b[11])
        && d(b[12])
        && b[13] == b':'
        && d(b[14])
        && d(b[15])
        && b[16] == b':'
        && d(b[17])
        && d(b[18]))
    {
        return false;
    }

    let mut i = 19;

    // Optional fractional seconds `.fff` (one or more digits).
    if i < b.len() && b[i] == b'.' {
        i += 1;
        let start = i;
        while i < b.len() && d(b[i]) {
            i += 1;
        }
        if i == start {
            return false; // `.` with no digits
        }
    }

    // Optional zone.
    if i == b.len() {
        return true; // no zone
    }
    match b[i] {
        b'Z' => i += 1,
        b'+' | b'-' => {
            i += 1;
            // `HH`
            if i + 2 > b.len() || !d(b[i]) || !d(b[i + 1]) {
                return false;
            }
            i += 2;
            // `:MM` or `MM`
            if i < b.len() && b[i] == b':' {
                i += 1;
            }
            if i + 2 > b.len() || !d(b[i]) || !d(b[i + 1]) {
                return false;
            }
            i += 2;
        }
        _ => return false,
    }

    i == b.len() // no trailing junk
}

/// TC-33 — render INTEGER epoch seconds back to an ISO-8601 UTC string for the
/// BYO-LLM wire. The exact inverse of [`iso8601_to_epoch_seconds`].
///
/// Storage and the governed SDK are epoch seconds, but the harness protocols
/// (`fathomdb.extract.v1` and the consolidation harness) carry ISO-8601 — LLMs
/// reason about dates as text, and pushing epoch integers onto them would make
/// the wire hostile to the very providers it exists to serve. So the boundary
/// converts in BOTH directions and the representation split stays a boundary
/// concern rather than leaking into the protocol.
pub(crate) fn epoch_seconds_to_iso8601(connection: &Connection, epoch: i64) -> Option<String> {
    connection
        .query_row("SELECT strftime('%Y-%m-%dT%H:%M:%SZ', ?1, 'unixepoch')", params![epoch], |r| {
            r.get::<_, Option<String>>(0)
        })
        .ok()
        .flatten()
}

/// TC-33 fix-1 — the inclusive epoch-seconds range SQLite's
/// `strftime(..., 'unixepoch')` can render back to ISO-8601. SQLite's date
/// functions cover years 0000..=9999 ONLY, so:
/// - `MIN` = `0000-01-01T00:00:00Z`
/// - `MAX` = `9999-12-31T23:59:59Z`
///
/// TC-33 fix-5 makes these the ACTUAL rejection predicate (a numeric
/// `[MIN, MAX]` bounds check), not just message text. Renderability was too
/// weak: `strftime(..., 'unixepoch')` renders a below-`MIN` value like
/// `-62_167_219_201` to `-001-12-31T23:59:59Z` (NON-NULL), so a pre-year-0000
/// epoch slipped the renderability guard even though it is outside the declared
/// years 0000..=9999. Both bounds are verified against SQLite to correspond
/// EXACTLY to the first/last renderable instant:
/// `strftime('%Y-%m-%dT%H:%M:%SZ', MIN, 'unixepoch') = 0000-01-01T00:00:00Z` and
/// `= 9999-12-31T23:59:59Z` for `MAX` (`MAX+1` and `MIN-1` are the first
/// out-of-range instants).
pub(crate) const MIN_RENDERABLE_EPOCH: i64 = -62_167_219_200; // 0000-01-01T00:00:00Z
pub(crate) const MAX_RENDERABLE_EPOCH: i64 = 253_402_300_799; // 9999-12-31T23:59:59Z

/// TC-33 fix-1 — reject an edge epoch that SQLite cannot render back to
/// ISO-8601, at the governed write boundary, so it is UNSTORABLE.
///
/// # Why this is the primary layer
///
/// Storage and `PreparedWrite::Edge` carry INTEGER epoch seconds and accept an
/// arbitrary `i64`. The consolidation path renders each candidate's
/// `t_valid`/`t_invalid` to ISO-8601 for the LLM via `strftime(..., 'unixepoch')`,
/// which only spans years 0000..=9999. An epoch outside that range renders to
/// NULL, and the render site would then send a silent `null` for a timestamp
/// that is actually stored NON-NULL — the OUTBOUND twin of the fail-open TC-33
/// removes. A `null` `t_invalid` reads as "still valid", and the consolidation
/// reference stub echoes a winner's `t_valid` straight back as the verdict's
/// `t_invalid`, so the `null` round-trips through the inbound normaliser as
/// "still valid": an invalidated edge silently resurrected.
///
/// Inbound ISO normalisation can never MINT such an epoch (a 4-digit-year ISO
/// string maxes at 9999), so the governed integer surface is the only ingress —
/// which is exactly where this guard sits. Like the inbound
/// [`normalize_extractor_timestamp`] hard-reject, it is a typed
/// [`EngineError::InvalidArgument`] naming the offending value and the bound,
/// never a silent coercion.
///
/// ⚠ It no longer mirrors the `validate_write` Node branch's
/// `valid_from >= valid_until` refusal: decision #18 (0.8.20 Slice 22) moved
/// THAT refusal onto the message-less [`EngineError::WriteValidation`] unit
/// variant, which carries no value at all. The two refusals are deliberately in
/// different families now — a malformed submitted write SHAPE is
/// `WriteValidation`; an out-of-domain scalar on this render path stays
/// `InvalidArgument`. Do not restate them as one pattern.
pub(crate) fn reject_unrenderable_edge_epoch(
    field: &str,
    value: Option<i64>,
) -> Result<(), EngineError> {
    // TC-33 fix-5 — an explicit numeric MIN/MAX bounds check, NOT a renderability
    // test. `strftime(..., 'unixepoch')` renders a below-`MIN` epoch (e.g.
    // `-62_167_219_201`, year -0001) to a NON-NULL string, so a renderability
    // guard would let pre-year-0000 values through even though they are outside
    // the declared years 0000..=9999. No `Connection` is needed now that the
    // predicate is pure integer arithmetic.
    if let Some(ts) = value {
        if !(MIN_RENDERABLE_EPOCH..=MAX_RENDERABLE_EPOCH).contains(&ts) {
            return Err(EngineError::InvalidArgument {
                msg: format!(
                    "edge field `{field}` = {ts} is outside the epoch-seconds range SQLite can \
                     render to ISO-8601 ([{MIN_RENDERABLE_EPOCH}, {MAX_RENDERABLE_EPOCH}], i.e. \
                     years 0000..=9999). REJECTED rather than stored: such an epoch renders to a \
                     silent NULL (or a nonsensical out-of-range instant) on the consolidation \
                     wire, and a NULL `t_invalid` reads as \"still valid\" — resurrecting an \
                     invalidated edge."
                ),
            });
        }
    }
    Ok(())
}

/// The JSON type name of `value`, for diagnosing a mistyped extractor field.
fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// TC-33 — normalise one timestamp arriving on the **BYO-LLM extractor
/// boundary** (`fathomdb.extract.v1`) into the INTEGER epoch seconds the storage
/// and governed-SDK layers use. **HARD-REJECTS** anything it cannot normalise.
///
/// This is the layering boundary HITL ratified on 2026-07-21:
/// - the **extractor wire format stays ISO-8601 strings** — LLMs emit text, and
///   this function is the one place that changes;
/// - **storage and the governed SDK surface are INTEGER epoch seconds.**
///
/// # Why rejection, not coercion — fail-open is the defect
///
/// A NULL `t_invalid` means **"still valid"**. So any path that turns an
/// unparseable timestamp into NULL silently **resurrects an invalidated edge**.
/// Two distinct fail-opens are closed here:
///
/// 1. **Malformed strings.** Previously NOTHING parsed or validated these; junk
///    went verbatim into the INSERT. Under the old TEXT column it then failed
///    CLOSED by accident (`datetime('junk')` → NULL ⇒ the read disjunct is
///    falsy ⇒ the row vanished). Under INTEGER that polarity would INVERT.
/// 2. **Non-string JSON — a fail-open that PREDATES TC-33.** The old site read
///    `edge.get("t_invalid").and_then(|v| v.as_str())`, and `as_str()` returns
///    `None` for a JSON number/bool/object. So `"t_invalid": 1710000000` — a
///    plausible mistake, and exactly the epoch form storage now uses — had its
///    invalidation SILENTLY DISCARDED and the edge stored as "still valid".
///
/// `None`/JSON `null`/absent is the ONLY sanctioned way to say "unknown"; it
/// maps to `Ok(None)` and keeps the NULL-means-still-valid semantic.
///
/// Refuses with a typed [`EngineError::InvalidArgument`] CARRYING the offending
/// value, so a caller can see what was rejected.
///
/// ⚠ This is NOT the same pattern as the `validate_write` `Node` branch's
/// `valid_from >= valid_until` check, which that comment used to cite: decision
/// #18 (0.8.20 Slice 22) moved that refusal onto the message-less
/// [`EngineError::WriteValidation`] unit variant, which carries **no value at
/// all**. Both the family AND the carry-the-value property differ.
pub(crate) fn normalize_extractor_timestamp(
    connection: &Connection,
    field: &str,
    raw: Option<&Value>,
) -> Result<Option<i64>, EngineError> {
    match raw {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => match iso8601_to_epoch_seconds(connection, text) {
            Some(epoch) => Ok(Some(epoch)),
            None => Err(EngineError::InvalidArgument {
                msg: format!(
                    "extractor edge field `{field}` must be a valid, calendar-real ISO-8601 \
                     timestamp; got {text:?}, which either `strftime('%s', ?)` resolves to NULL \
                     or fails the calendar round-trip (a shape-valid but impossible DAY like \
                     `2025-02-30` that SQLite would silently ROLL OVER to a different instant). \
                     REJECTED rather than stored: a NULL `t_invalid` reads as \"still valid\" and \
                     a rolled-over date stores the WRONG instant — both breach the hard-reject \
                     contract. Use JSON null for \"unknown\"."
                ),
            }),
        },
        Some(other) => Err(EngineError::InvalidArgument {
            msg: format!(
                "extractor edge field `{field}` must be an ISO-8601 string or JSON null; got a \
                 JSON {kind} ({other}). The `fathomdb.extract.v1` wire format carries ISO-8601 at \
                 this boundary — INTEGER epoch seconds are the STORAGE representation, not the \
                 wire one. REJECTED rather than coerced to NULL, which reads as \"still valid\".",
                kind = json_type_name(other)
            ),
        }),
    }
}
