# Requirement-to-oracle mapping and minimal G100-2 composition

These are reviewed coverage oracles under seq-371/372, not new canonical ACs,
observed defects or completed tests. Source/runtime technical holds remain. Keep accepted maintenance,
format37/no migration, retained audit/closure carve-outs and numerical gates.

| Existing requirement/local acceptance | Proposed concrete protocol/oracle |
| --- | --- |
| R100-1/A100-1: atomic intent/deletion/sanitation, truthful phases | G2 existing before-delete hook proves rollback to Queued/intact targets; after-delete proves committed removal/Executing debt. Q1 native stop occurs after redacted commit and before receipt, requiring Finalizing/Removed, never speculative Complete. |
| R100-2/A100-2: exact policy/fences/reopen | Persisted ExplicitMaintenance in every new case; raw pre-retry state plus reopened same-ticket status, survivor reads and unrelated-write refusal without effects. Existing C7/S3 autonomous observer evidence stays separate; this protocol never disables a worker and claims autonomy. |
| R100-3/A100-3: exact registered-copy and physical success boundary | G2 positive four-FTS/dependency/all-version/non-head-vector/span fixture, original sink replay cell and raw main/live-WAL target absence before close at Complete. Q1 minimal canonical/FTS fixture adds active-copy interruption only and does not claim unpopulated vector/sink coverage. |
| R100-4/A100-4: immutable scope/survivors/permitted replay | Pre-authored fixture values and exact pre-fault survivor row/cursor/history snapshot; independent survivor-only store for score/order. Original cut/scope remains unchanged until the specified identity scrub. Same-ticket retry/idempotence, one receipt and reopened no resurrection. Existing offline Queued/activated controls remain their own proof. |
| R100-5/A100-5: actual declared fault reached | Unique same-execution native witness with eligible positive frame, exact erasure callstack and PC/subject/PID identity before owned-child kill. Healthy release-through and bounded unqualified rejection controls; no timing, pre/post, BUSY or mutant substitute. |
| R100-6/A100-6: attributable, minimal, truthful qualification | Future own source-qualified binary/operator and source/lock/features/tool/host identity; raw stop/exit/observer/live-file outcomes and all failures. Existing eight controls keep reviewed command/source inference limits. Design then fresh final independent review; no closure from packet/review existence. |

Canonical AC-028a/b/c preserve authorized audit and non-target projections;
R-20-E1/E5→AC-080 supplies structural/raw-byte success proof. REQ-031b/AC-034
power/OS-crash obligations are not qualified by these process exits. N29-R /
R29-REL-R and R29-3P-16/O3-16 remain the release allocation; inclusion is settled.

## Exact phase/root/request expectations

Read raw JSON independently of product load/proof functions. `_fathomdb_open_state`
key `_fathomdb_physical_erasure` is root authority; `_fathomdb_fts_sanitation`
records sanitation. `_fathomdb_erasure_requests` payload stores request/status;
`_fathomdb_read_visibility_state.coalescing` must be integer0. Freeze field names
from then-current reviewed source before implementation; unknown/malformed state
is refusal/failure, not an omitted assertion.

| Point observed before retry | Expected committed authority |
| --- | --- |
| G2 before_delete_commit | Queued / UnchangedByThisRequest. No activated root from this request; target canonical/FTS/vector/span/history and survivor snapshot remain intact. Original selector/cut/scope/originals/key/ticket match accepted snapshot. |
| G2 after_delete_commit | Executing / Removed, root references same ticket and exact admitted cursors/dependency closure. Canonical/registered projection targets are zero; sanitation/raw bytes need not yet be clean. Original request selector/cut/scope remains intact. Global mutation fence persists. |
| G2 after_redacted_commit and Q1 active-copy crash | Finalizing / Removed. Request selector=None, cut=0, scope cursors/sources/identities empty, originals empty. Root redacted=true, retry empty, cursors/vectors/denied_sources/denied_identities empty. Opaque ticket/closure IDs, safe counters and allowed audit metadata remain. No Complete receipt; root debt and global fence remain. Do not demand boundary/generation zero or erase contract-authorized metadata. |
| G2 after_sink_redaction | Sanitizing / Removed after completed sanitation units; FTS sanitation finalizing, root redacted=false and request selector/cut/scope retained. Original sink file is already selectively redacted but the durable pending original-sink obligation remains; no Finalizing identity scrub or Complete yet. |
| Actual Complete after healthy/retry | Same one terminal request Complete, no active root/owed closure, sanitation clean, structural targets zero; target bytes absent from main and actual live WAL before Engine close. Target markers may remain before completion. Survivor values/history/rowIDs and independent score/order stay exact. |

## Four G2 cells, one rich fixture pattern

Add only three explicit crash phases before_delete_commit, after_delete_commit,
after_redacted_commit, and one original-sink after_sink_redaction replay cell.
Use existing hooks/exit73; preserve old85 parent/assertions unchanged. No family×
hook matrix, extra fault venue, performance claim or erasure feature campaign.

Adapt the audited slice85_scope rich-fixture pattern into new test-local support;
do not move/rewrite existing tests or import their expected result as an oracle.
Seed explicit authored target/survivor revision bodies, root/dependent provenance
and dependency registration, all four positive FTS families, mapped non-head
vector900000 plus a positive `_fathomdb_vector_spans` join. Use the existing fixed
8-D test embedder with in-memory deterministic values, not models/GPU. Capture
target write_cursor/vector-rowid/span associations BEFORE fault; zero checks
must use those captured IDs rather than a post-delete canonical join that is
vacuously empty. Snapshot exact survivor canonical/history and FTS rowID/body
values. The independent survivor-only fixture supplies rank/order/score-bit
expectations AFTER committed target removal. Before-delete rollback instead
requires the exact intact full-fixture pre-fault score/order snapshot, because
targets still contribute to corpus statistics; survivor-only equality there
would be a false oracle. At every terminal Complete, use survivor-only equality.
Same-store pre/post ranking alone is insufficient for the post-removal proof.

Use a fresh identical populated case per selected phase, with positive fixture
checks and a healthy rich success control once. After each real child exit73,
preserve quiescent main/WAL/SHM before any Engine/SQLite open. The new Rust parent
can initialize vector support by opening only the independent survivor-only store
first, as existing raw-Connection tests do after Engine initialization, then use
read-only/query-only connections on working crash copies. No Engine/proof helper
may normalize the crash copy before the independent raw assertions. Failure to
observe a required raw vector/span family remains unqualified, not skipped.

The sink cell registers one original JSONL file before activation and records its
identity/fingerprint and a target+survivor stable-ID event. At the existing
post-rename hook, abrupt exit73 must leave target stable-ID absent from that SAME
file, survivor stable-ID retained and durable original-sink obligation still owed.
On a disposable retry copy, offline completion must refuse owed sink debt; do not
use it to clear the obligation. Explicit reopen authenticates the recorded
original through handle.reattach_original_erasure_sink and retries the SAME ticket.
No replacement sink creation, new telemetry registration after deletion or
whole-file erase is a substitute. This augments composition only; existing
82/85 multiple-original/wrong-sink/offline controls are reused with their limits.

For all cells, assert the fixed phase-specific pre-retry oracle first; only then
retry and check Complete, structural/span zero, unchanged survivor history and
live raw bytes. A missing fixture population is a harness failure, not product RED.

## Healthy and negative controls

One Q1 release-through case must actually reach the native witness and finish
normally before the matched fault case; a no-stop normal run is not enough.
One G2 healthy rich completion is sufficient; do not multiply it per hook.

Future operator parser/state-machine controls use explicitly synthetic minimal
transcripts for missing/wrong nonce, PID/start time, source/binary, unresolved PC,
wrong callstack, zero/invalid frame, missing locals, native-return/Complete before
stop, missing READY/DONE and timeout. Every such case rejects the qualification
transition and records why. A fake/no-op child verifies bounded reap behavior
without product faults. These test the NEW operator's refusal/cleanup, not the
database or product behavior. No mutation of correct production code, regenerated
golden or missing-symbol compile failure is presented as baseline RED.

If a real existing contract fails, preserve/minimize that actual test result and
apply the unchanged real per-finding RED→separate GREEN rule. Coverage-only work
is limited to the seq-371 exception; these controls create no broader authority.
