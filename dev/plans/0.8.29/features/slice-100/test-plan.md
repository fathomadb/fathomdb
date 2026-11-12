# Slice 100 test map and evidence limits

This candidate authorizes read-only inspection and documentation only. S1–S9
are existing selectors and assertion mappings, not requested new runs, tests or
source changes. [Design](design.md) explains selective current input equivalence
and the retained historical actual20 execution inference. Its bounded Astra PASS
is preserved; fresh v1 design PASS covers the bounded census. Exact v2 semantic
design PASS covers the proposal. Exact v3 design PASS verified documentary fixes;
parent scoped checks and a different fresh final documentary/output PASS cover
the imported stable parent candidate, as bound in [review log](review-log.md).

Existing selector form is `cargo test --locked -p fathomdb-engine --features
operator,test-hooks` plus the row below and serial test threads. Feature-gated
targets can be omitted without these features. Ignored child entrypoints count
as reached only through a parent whose assertions require their named exit;
directly selecting all ignored tests is not an equivalent proof. No command in
this map was executed by this owner.

| Selector | Exact target/filter / AC / meaningful independent oracle |
| --- | --- |
| S1 | `--test slice85_crash -- actual_process_exits_preserve_atomic_intent_removal_redaction_and_receipt --exact --test-threads=1`; A1/A2. Requires eight reached exit73 statuses,5s child watchdog, target/survivor and ticket logical state/retry assertions unchanged. |
| S2 | `--test slice82_recovery`; A1/A3/A4. Actual pre-delete/pre-sanitation COMMIT exits, positive continuation geometry, original-sink file/durable replay and malformed-state main/WAL preservation. Run selected parent(s) under an external owned watchdog because these existing child `.status()` waits themselves are unbounded; a timeout remains failed/unqualified and is reaped, never PASS. |
| S3 | `--test slice83_availability`; A2. No-follow-up autonomous reopen has independent observer with5s eventual-progress bound; this is a harness bound, not a product SLA. Preserve actual worker and unsupported/envelope/affected-write controls. Existing child wait limit as S2. |
| S4 | `--test slice85_physical`; A2/A3/A4. Real external reader pin requires Finalizing/Removed/WalCheckpoint, global fences through release/reopen, same activated offline receipt and Queued not activated, authenticated multi-original sink redaction, corrupt authority refusal. |
| S5 | `--test slice85_scope`; A3/A4. Positive four-family/dependency/non-head-vector fixtures, independent survivor-only score/order and generated queued-withdraw-write-reopen histories. Does not become abrupt-crash span proof by passing. |
| S6 | `--test slice82_physical_erasure`; A3/A4. Exact survivor/history rowID snapshot, independent survivor-only FTS scores/order, positive four-family raw tables/main/live-WAL target absence. Keep old admission/body/Unicode/sanitation constraints unchanged. |
| S7 | `--test slice83_routes`; A2/A3/A4. Independent survivor-only current/frozen/graph/evidence routes and actual projection drain before delete; explicit target span0 after queued work. No disabled worker passed off as autonomy. |
| S8 | `--test erasure_completeness`; A3/A4. AC080 raw-file positive secret/control before close, typed BUSY refusal and original-sink selective redaction/retry/reopen; preserve existing platform-specific Windows distinct-child diagnostic. Linux execution does not qualify that Windows selector. Record actual inventory, not stale “ten” count by narration. |
| S9 | `--lib physical_erasure::tests::`, then `--lib visibility_admission_tests::`, `--lib visibility_coalescing_tests::`, `--lib projection_registry::span_erasure_tests::`, plus `--test slice95_visibility_coalescing`; A1/A3/A6. Actual deferred-COMMIT rollback, mapped non-head replay residue refusal,24-case retry/survivor property, armed-state admission, mapping/span cleanup while coalesced, direct orphan-span erasure and current37 constraints. Record actual nonzero selections/features and property failures/seeds. |

## Original obligation and remaining evidence

S1/S2 establish actual transaction-interior interruption, addressing D28-09's
“inside a transaction or WAL checkpoint” fault-position minimum. S1 has eight
exit73 witnesses with5s watchdog and target/survivor/ticket logical-state/retry
assertions. It checks exact Complete phase pre-retry only after_completion_commit.
S2 has broader canonical/debt/continuation/sink assertions with unbounded child
waits. Neither establishes every rich dimension after every abrupt exit.

S3–S9 retain distinct autonomous, explicit, offline, positive physical, survivor,
history, span and refusal oracles. S8 raw physical proof is before close; S3's
post-crash main-file scan is after close. S7's route result evidence is historical;
current70 changes query degradation reporting, although its erasure mechanisms
and named test are unchanged. No automatic current-native/artifact qualification
is inferred from selective source equivalence.

G100-1 active-native-checkpoint interruption is NOT QUALIFIED and optional under
the original OR. Three retained predicates remain unmet: interrupted survivor
history (G100-2H), direct post-crash projection/span state (G100-2P) and independent
main/live-WAL absence after post-crash completion before close (G100-2W). These
are actual evidence gaps, distinct from optional all-in-one/every-family/every-hook
proof. [Design](design.md#obligation-and-disposition-map) retains their authority.

## Minimal coverage-only proposal

**Proposal only: no implementation, execution or exception granted.** One proposed
new normal parent in existing `tests/slice85_crash.rs`, named
`interrupted_history_projection_spans_and_live_wal`, addresses H/P/W. It uses two
existing faults and one healthy control, on disposable real format37 databases
with persisted ExplicitMaintenance. No debugger, new runtime hook, production
change, native checkpoint fault, sink expansion or broader campaign is proposed.

### Exact reuse and selected cells

Source symbols below are verified at1099978, within `fathomdb-engine`:
`tests/slice85_crash.rs` owns ignored `crash_child` and normal parent
`actual_process_exits_preserve_atomic_intent_removal_redaction_and_receipt`,
which owns eight-phase exit73/watchdog/reopen/retry assertions. Preserve that
original default seed, loop and assertions. Add only a test-local prepared-fixture
mode to the same child for the new parent; it opens the prepared DB with the same
deterministic provider/persisted policy and skips the default two-row seed.
It still queues the existing source selector and completes through the same
maintenance API, arming only existing before_delete_commit or after_delete_commit.
The selected source-erasure hooks bracket COMMIT after real writes in
`src/erasure.rs::Engine::excise_source_inner`, at `before_delete_commit` and
`after_delete_commit`; no runtime hook is added or moved.

Reuse `Fixed` and `rich_fixture` in `tests/slice85_scope.rs`: deterministic
8-dimensional provider, searchable text projection and existing doc vector kind. A local test helper
needs only `target` (`target-source`) and `survivor` (`kept-source`),
each with three deterministic body versions containing distinct target/control byte markers; drain each revision
so lexical/vector/span populations are positive. No dependency tree, non-head
synthetic vector or four-family cross-product is required for these three gaps.
Reuse `snapshot`,
`complete_erasure_removes_positive_terms_from_every_family_and_preserves_history`
and `survivor_only_database_agrees_on_fts_order_and_score_bits` in
`tests/slice82_physical_erasure.rs`, plus `erasure_wal_bytes_absent` in
`tests/erasure_completeness.rs` for raw byte/control semantics. These private
helpers are in separate integration targets; copy only their small seed/oracle
recipe locally rather than changing production or extracting shared infrastructure.

| Proposed cell / mapped acceptance | Reached boundary and independent assertions |
| --- | --- |
| H / A100-4 / G100-2H | For both selected crashes, independently read all three survivor canonical versions after reopen and again after retry. Compare every captured canonical row/column, cursor/rowID and body to the validated pre-crash snapshot, not just presence/count. Parent first checks that snapshot contains the three literal intended survivor bodies. Compare final survivor-only lexical ID/order/score bits against a separately seeded survivor-only store. |
| P / A100-3 / G100-2P | Before fault, capture positive target/survivor cursors, lexical rows, mapped native vector IDs and span tuples. After before_delete_commit exit73/reopen, target canonical/lexical/vector/span records equal their pre-crash snapshots (rollback). After after_delete_commit exit73/reopen, target canonical rows, selected lexical rows, captured native vector IDs/mappings and target-cursor spans are directly zero. Survivor history, lexical rows, vector IDs/mappings and spans remain as captured in both cases. Check before any retry through independent raw SQLite reads, not product proof validators or logical read absence. |
| W / A100-3 / G100-2W | Retry each crashed ticket to Complete; healthy control completes without a hook. Before any final Engine/observer close or subsequent write, independently scan main and live-WAL for absence of the positive target marker and presence of the survivor control marker in at least one artifact. Record each artifact's existence/length; successful TRUNCATE may leave WAL empty/absent, but unexpected I/O errors fail. Repeat direct target structural-zero and survivor-history checks. Complete alone is insufficient. |

### Minimal protocol and independent controls

Prepare each fixture and the separate survivor-only reference from literal
version bodies. Independently capture raw canonical/lexical/vector/span rows in
stable key order; assert the three intended historical bodies and positive target/
survivor lexical, native-vector and span populations. Require actual target/control
bytes with the seeded Engine live. Close preparation ownership before child open;
never regenerate expected snapshots from post-crash results.

Execute before_delete_commit and after_delete_commit once each through the
prepared child with exact73/reached phase and existing5s watchdog/kill-and-wait.
Hook miss/fall-through99 or zero selection fails. Reopen ExplicitMaintenance with
the deterministic provider, independently check H/P before retry, and end the
observer transaction before TRUNCATE. Retry the same ticket, then check H/P/W
with Engine and observer still live and no ordinary post-completion writes.
One healthy no-fault completion uses the same fixture and terminal H/P/W checks;
positive populations plus survivor-only comparison prevent vacuous absence or
whole-store deletion from passing. Original eight controls remain unchanged.

Use a60s external bound per fixture sequence and cleanup only owned children on
failure; bounds are test limits, not product deadlines. Retain all three fixture outcomes'
source/features/executable/command/count/exit, reached phase, raw pre/post snapshot
and artifact results, failures/skips. Timeout is unqualified. No mutation of
correct product code or missing-hook compile failure supplies RED.

### Required narrow policy exception and future qualification

These assertions will likely pass on the already-correct baseline. Strict TDD
cannot truthfully supply a product RED for absent assertions. The needed exception
would authorize **only this test-local H/P/W coverage parent, prepared-child mode,
small seed/oracle helpers and healthy control**, as honestly labeled coverage-only
commits with observed baseline results; no manufactured RED, new hook, production
change or further campaign. That exception is not granted, not inherited from20
and not asked of the user here. If absent, leave these three evidence gaps open.
A real accepted-invariant failure instead becomes its own observed RED commit/body
with exact assertion evidence, then separate minimal GREEN with unchanged tests.

Any later execution requires explicit scope/exception, reviewed exact source
entry and owned preflight/tools/artifacts. Native work starts only with aggregate
MemAvailable>=16GiB, data free>=250GiB and root free>=32GiB; light documentation
uses8GiB. Use one native phase, serial tests/jobs1 and task-only data temp/cache/
logs; borrow no slice resources. Current roughly13.6GiB memory permits docs only.
No paid/GPU/model/VM/network fault venue is required. Watchdog/resource failure
stops the owned task, preserves receipts and authorizes no threshold reduction.

Focused proposed selectors are the new parent and unchanged original parent in
`--test slice85_crash`, with operator,test-hooks and nonzero counts. If authorized
test/executable changes are implemented, run the required full source gate once
on the stable final code phase, keeping original failures and affected resumptions;
do not substitute focused coverage for it. Parent then obtains fresh independent
implementation/results review before any predicate or whole-slice closure claim.
Current documentary reviews/checks provide none of that runtime acceptance.

## Review and verification of this candidate

Owner performed link/source/hash/scope/whitespace consistency checks and froze
the exact seven-file patch/manifest. No repository Markdown or full source gate
ran in that owner stage. Parent subsequently passed the scoped Markdown/state/
diff checks on unchanged11-file diffc0e80622 against1099978, then obtained the
different fresh final DOC/OUTPUT PASS. [Review log](review-log.md) records exact
subjects, receipt and limits. These are documentary results, not product-test
passes, H/P/W qualification or a verdict on this record-only delta.

A later demonstrated defect needs observed RED SHA/assertion/selector evidence,
separate GREEN and unchanged oracle. Any future affected source gate is run once
on a stable code phase with original failures and scoped resumptions retained;
this map creates no execution or all-release rerun authority.160 retains affected
final combined qualification. Slice20's unresolved exception grants none to100.
