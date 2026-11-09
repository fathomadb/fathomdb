# Erasure

The direct erasure verbs below describe the default Autonomous policy.
For the opt-in ExplicitMaintenance profile, see
[Schedule an explicit maintenance window](#schedule-an-explicit-maintenance-window).

FathomDB exposes two erasure verbs. Which one applies depends on **how the row
was identified when it was written**, not on who is asking.

| Verb | Addresses | Reaches | Surface |
|---|---|---|---|
| `purge(logical_id)` | one **governed** node, by its `logical_id` | all versions of that node + its touching edges | SDK (Python, TypeScript, Rust) |
| `erase_source(source_id)` | every row carrying a **provenance** id | governed *and* anonymous rows from that source | SDK (Python, TypeScript, Rust) |
| `fathomdb recover --excise-source` | same as `erase_source` | additionally the engine's reserved `_`-prefixed namespace | CLI only |

`erase_source` is new in 0.8.20. Before it, the only provenance-addressed
erasure lived in the operator CLI, so an embedded consumer with no `fathomdb`
binary on `PATH` could not delete **anonymous content** — rows written without a
`logical_id` — at all. `purge` cannot reach those rows, because there is no
`logical_id` to key on.

## What `erase_source` guarantees

The argument follows the write-side `SourceId` grammar. Embedded NUL is valid
in a source identity and remains addressable by an exact matching erase call.

For every canonical row whose `source_id` matches, `erase_source` deletes:

1. the **canonical rows** themselves (`canonical_nodes`, `canonical_edges`) —
   *every version*, not only the latest;
2. every **row-owned projection** of those rows. This is registry-driven rather
   than a hand-maintained list, so a projection table added later is covered by
   construction: FTS5 (`search_index`, `search_index_v2`), the `vec0` shadow
   tables, and `_fathomdb_vector_rows`;
3. the erased ids from the **telemetry sink**, by selective redaction — the
   unrelated records in the same sink survive;
4. the erased bytes from the **write-ahead log**, via a truncating checkpoint.
   Without this the content stays `grep`-able in `<db>-wal` even though every
   table row is gone: the `DELETE` appends new frames rather than rewriting old
   ones.

Correction does not make a source bucket uneraseable. After one source has
been superseded by another, either bucket may be erased first. Completed
direct-dependent closure rows are removed with their source, while receipt
redaction is validated before those closure records disappear. The returned
counts describe only rows and projections owned by the requested source; the
operation does not recursively erase the replacement bucket. The same holds for
`purge`: a corrected logical id can be purged once deleted, and its completed
correction closures are removed only after receipt redaction.

The call **does not report success on a partial erasure.** If the WAL checkpoint
cannot complete — typically a concurrent reader pinning a snapshot — the verb
raises **`ErasureIncompleteError`** (Python `fathomdb.errors.ErasureIncompleteError`,
TypeScript `ErasureIncompleteError`) instead of returning a report. It carries
`stage` (which step did not complete — e.g. `"wal_checkpoint"` /
`"telemetry_redaction"`) and `detail`. Retry the verb; it is idempotent, so a
retry after a partial failure is safe and an already-erased source is a
zero-count success.

```python
from fathomdb.errors import ErasureIncompleteError

try:
    report = engine.erase_source("tenant-a")
except ErasureIncompleteError as e:
    log.warning("erasure incomplete at %s: %s — retry", e.stage, e.detail)
```

Erasure is durable: it survives a close and re-open, because it is a committed
transaction rather than a cache eviction.

## What `erase_source` does *not* guarantee

Read this list before making a deletion promise to anyone downstream.

- **It does not reach anything outside the live database file.** Enumerated,
  not implied — each of these has to be re-generated or destroyed separately
  after any erasure:
  - **`safe_export` archives** produced by `fathomdb doctor safe-export`.
    They are point-in-time copies and are never revisited by an erasure.
  - **Operator backups and snapshots**, including anything your host or
    volume manager took.
  - **Curated gold / evaluation files** derived from the corpus — fixtures,
    exported judgement sets, cached embeddings held outside the engine.
  - Anything else replicated or copied into your own store.
- **It does not erase filesystem-level remnants.** Completion proves the
  registered SQLite-copy sanitation boundary and WAL truncation. Filesystem
  snapshots, copy-on-write prior blocks, SSD wear-levelling and other storage
  copies remain outside that boundary; no universal storage-remanence claim
  follows from database completion.
- **It does not erase the audit record that the erasure happened** — that is
  deliberate; see below.
- **It does not reach the engine's reserved namespace.** `source_id` values
  beginning with `_` (`_engine:*` engine substrate, `_legacy:pre-0.8.20`) are
  rejected by the SDK verb. Only `fathomdb recover --excise-source` may address
  them. This is a safety boundary, not an oversight: `_legacy:pre-0.8.20` names
  *every* anonymous row that predates 0.8.20, so a single SDK-reachable call
  against it would wipe them all at once.
- **It does not retroactively make un-provenanced rows erasable.** Provenance is
  mandatory as of 0.8.20, and the 0.8.20 migration back-fills
  `_legacy:pre-0.8.20` onto pre-existing anonymous rows, so this should be
  vacuous. Verify it on a real database with
  `fathomdb doctor orphan-provenance` (below).

## The erasure audit record

Every erasure appends a row to a dedicated operational collection
(`excise_source_audit` / `excise_record_audit`). As of 0.8.20 these collections
are **exempt from the retention sweep**: accountability — demonstrating *that*
an erasure occurred — is a separate obligation from the erasure itself, and a
retention cap must not be able to silently destroy the proof.

For op-store record erasure the audit stores a **SHA-256 digest** of
`collection + record_key`, never the key itself. A `record_key` is arbitrary
caller-supplied text and may itself be the identifier being erased, so echoing
it into a durable audit row would defeat the erasure. The digest is enough to
prove a specific record was erased to someone who already knows the key, and
useless to anyone who does not.

## `source_id` must not contain personal data

**Rule: treat `source_id` as a public identifier. Do not put personal data in
it.** Use an opaque document id, a tenant id, or a hash — not an email address,
a username, or a filename containing a person's name.

The reason is that `source_id` outlives the rows it names:

- **An erasure-audit row may not have been swept yet.** The audit row records
  the `source_id` that was erased. Audit rows are now retention-exempt
  (above), so a `source_id` recorded there persists by design.
- **Corollary — a `source_id` you erase is still readable afterwards** by anyone
  who can read the op-store, until and unless you erase the audit row too, which
  defeats the point of having it.

> **Note.** Earlier drafts of this rule (design v4 §3.6) justified it by
> claiming the audit row "retains `source_id` permanently, by design". That was
> **verified false at the time it was written** — `enforce_provenance_retention`
> swept `operational_mutations` cap-first with no collection filter, so audit
> rows were destroyed like any other op-store row. The rule was correct but the
> stated basis was not. 0.8.20 both **corrects the rationale** (an unswept audit
> row is the real exposure) and **makes the premise true** by exempting the
> audit collections from the sweep. That section of v4 is marked SUPERSEDED IN
> PART.

Treat caller-supplied `logical_id` values as public identifiers too. Direct
canonical writes accept opaque IDs such as `note:1`; they are not universally
derived from a name. Extractor ingestion derives entity IDs internally as:

```text
logical_id = SHA256( lowercase(kind) + ":" + lowercase(name) )
```

For those extractor-derived IDs, inputs are **lowercased** before hashing. That is load-bearing for identity
collapse (so `Alice Smith` and `alice smith` are one entity), but it also
*shrinks the preimage space*: a dictionary attack over plausible names does not
have to guess capitalisation. Do not treat a `logical_id` hash as though it
conceals the name that produced it.

## Auditing a real database

```bash
fathomdb doctor orphan-provenance --json ./app.sqlite
```

A read-only per-`source_id` census. It reports each provenance bucket with its
row count, how many of those rows are also `logical_id`-addressable, and whether
the bucket is in the reserved namespace.

The load-bearing field is `unerasable_rows`: canonical rows carrying **neither**
a `source_id` nor a `logical_id`. Such a row is reachable by no erasure verb —
`purge` keys on `logical_id`, `erase_source` keys on `source_id` — so it can
never be deleted on request. It should always be `0`.

Exit codes follow the usual doctor convention: `0` when clean, `65`
(`DOCTOR_FOUND_ISSUES`) when `unerasable_rows > 0`. Rows under
`_legacy:pre-0.8.20` are **reported but are not an issue** — they are fully
erasable through the CLI recovery seam; they are merely not attributable to a
specific caller source.

## Schedule an explicit maintenance window

New format-37 stores can opt into persisted explicit maintenance:

```python
from secrets import token_hex
from fathomdb import Engine, EngineConfig

engine = Engine.open("fresh.sqlite", config=EngineConfig(
    physical_erasure_policy="explicit_maintenance",
))
selector = {"kind": "source_all_at_cut", "source_id": "tenant-a"}
opaque = token_hex(16)  # Save this for exact retry after a lost response.
with engine.begin_erasure_maintenance(1000) as window:
    accepted = window.queue_physical_erasure(selector, opaque)
    if accepted["outcome"] == "accepted":
        result = window.complete_physical_erasure(accepted["ticket"])
        if result["outcome"] == "incomplete":
            print(result["status"])  # Retained obligation; not physical success.
engine.close()
```

Queue acceptance leaves data readable under its existing eligibility. Ordinary
new queue admission returns `needs_maintenance_admission` when safe cheap proof
is unavailable; acquire a window for the heavy preflight as above. Exact retained
key retries return the same ticket. `physical_erasure_status(ticket)` retrieves
identity-free status; `withdraw_queued_erasure(ticket)` withdraws only unactivated
intent, never executing debt. Use random opaque keys, not subject-derived values.

The window excludes ordinary reads/writes and projection/admin work. Newcomers
refuse promptly; existing owned operations drain cooperatively. Acquisition wait
is not an active cleanup timeout. A Complete result requires exact registered-copy
sanitation, original telemetry sinks and WAL truncation. Unavailable original sinks
or external pinned readers can produce Incomplete and a prolonged write fence.
Authenticated original-sink reattachment is available through
`window.reattach_original_erasure_sink(ticket, path)`; it never registers a replacement.
Handle close/drop releases temporary ownership but cannot forgive durable debt.
Explicit reopen preserves policy/fences and performs no autonomous cleanup.

Autonomous remains the default. Direct online erase/purge under explicit policy
refuses `maintenance_required`; use the window's selected ticket. Fresh format 37
is required for both profiles: earlier stores are refused, with no automatic
migration or in-place opt-in. Finish/export old stores using their supported old
product procedures. This API promises no immediate revocation, completion deadline,
backup deletion or universal storage remanence. TypeScript uses camelCase methods
and fields with the same outcome values; Rust exposes typed matching enums.
