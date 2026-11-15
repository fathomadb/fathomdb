# Slice 100 selectors, independent oracles and execution holds

Selected100/D28-09 is complete locally. The seq386 pair on clean
`fb92d51ab92702c0cc7a5dc515aa8a69187008e2` qualified healthy release-through,
then its own witnessed active-checkpoint SIGKILL, untouched snapshot, independent
observer and same-ticket retry/reopen. Independent Astra-medium accepted the
results and found no remaining selected100 acceptance gap. Existing five G2
cells,20 operator controls and accepted assembled source qualification remain
valid within their recorded limits. All earlier unqualified attempts are retained.
This is bounded process-interruption proof, not power/OS-crash or exhaustive
fault qualification. Seq386 is spent; no further fault is authorized.

Existing selectors and source-history references below retain their original
limits. No passing coverage or full gate was replayed to close the Q1 gap.

## Existing proof to reuse before writing tests

Base form is `cargo test --locked -p fathomdb-engine --features operator,test-hooks`
plus selectors below and `-- --test-threads=1` where no exact filter is given.
These targets require both named features in Cargo.toml. Default workspace tests
without them can skip the target entirely. Ignored child entrypoints called by
their normal parent count as actual child execution only when exit/reachability
evidence proves the parent ran; do not directly run all ignored tests.

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

Select only affected obligations or reuse exactly attributable existing receipts;
these rows define the evidence set, not permission for a broad repeat campaign.
Retain old successes, failures, first/persistent fault cases and healthy controls.
No observed RED/GREEN or product pass is supplied by the selector table.

Actual20 reuse: accepted f803 canonical `agent-test.sh --tier=all` invokes
`test-rust-workspace.sh --serial`: unfiltered workspace Rust, quiet output,
no-fail-fast, jobs1/test-threads1. CLI operator and engine self-dev test-hooks
closure select these controls. The retained Rust log is65,528B/2,408lines,
SHA256 `1baf83f557ee5b959c91866a778bb3d8c3dcf2ef2e6e13e7bbf30a74f6d44b84`.
Lines1616–1634 match85 crash's2tests/eightchild launches/1pass/1ignored/0fail;
unchanged passing parent requires all eight exit73/state/retry assertions. This
is command/source inference, not named phase/PID/binary-hash output. Entire
src/lock match f803→03bdd; whole src/lock do not match the older2f691 evidence.
The named erasure/crash/checkpoint files and canonical Rust caller are unchanged.
The newer result therefore adds current integrated-runtime attribution without
reruns, but no new assertion or native-checkpoint fault position. Qualification
assembles216/216 suites after original module-policy failure and bounded
continuation; native identities remain separate from Rust crash execution.
20 A3/A5 and RAW capacity preparation are accepted; A6/whole20 remain open. No
new hard dependency or100 exception follows. Keep original failures/skips; no
success fills G100-1/G100-2 observations absent from the unchanged assertions.

## Approved new coverage and held execution

S10: [Q1 child/operator protocol](q1-protocol.md), R100-5/A100-5 and G100-1.
A new ignored normal-exit0 child uses the ordinary explicit maintenance API.
A test-only ready marker gates native breakpoint setup after fixture/queue work;
no production seam is added. One healthy case must reach the eligible native
copy frame and finish normally. Only then may one matched interruption case
obtain its OWN same-execution source/binary/PID/PC/erasure-stack/positive-frame
witness before SIGKILL. Missing symbols/locals/ptrace/stack/frame or timeout is
UNQUALIFIED. Bounded cleanup of only registered actors is not a fault result.

S11: [Four G2 composition cells](composed-oracles.md), R100-1–4/A100-1–4 and
G100-2. Existing before-delete, after-delete, after-redacted and one post-sink-
rename hook supply the same existing process-exit model. Capture positive target
cursor/vector/span IDs before deletion, exact survivor/history values and raw
main/WAL/SHM before reopen/retry. Preserve old85 assertions/files; no broad matrix.
Before-delete rollback compares intact full-fixture statistics; after removal
uses independent survivor-only score/order. New healthy/refusal controls and
all six requirement/oracle joins are specified in the linked oracle document.

[Implementation and resource map](implementation-map.md) names intended files,
held commands, opt-in Q1 and proposed operational bounds under the approved
120s/case and64GiB caps. Seq-371 permits honest coverage commits; it does not
turn compiler/missing-symbol/mutant failures into product RED. No implementation
or execution occurs until this recheck and source GO; source review, actual
runtime readiness and parent technical GO precede Q1 faults.

## Receipts and terminal disposition

Each executed cell records source SHA/tree/delta, Cargo.lock/config/features,
toolchain/OS/architecture, test executable digest, exact command/test count,
ignored/skip/exit status, child identity/reached fault and pre/post state, observer
and live-WAL timing, fault-free control and replay result. Native carriers, if
affected, additionally require own installed module/ABI/features/digest receipt
and real open/read/write/close/reopen/exit; existing10 artifacts are not100 proof.

For each confirmed defect: observed RED SHA/assertion/exact selector/log → separate
GREEN SHA → unchanged oracle and independent review. Audit RED from Git before
acceptance. Two same-issue corrections trigger the repository rethink rule.
No code/no new coverage after reviewed source reconciliation is a valid result;
record actual G100-1/G100-2 results and remaining limitations before full
D28-09 closure. Approved scope alone supplies no qualification. A full gate passing cannot fill a missing reached
fault or authorize a TDD exception. Later affected core changes invalidate the
named receipts and160 reruns the affected combined seams.

The seq-383 healthy/native witness passed on integrated453, but its interruption
has no confirmed exit, raw snapshot or recovery result. Only the minimal held
repair/RED intent in design.md is next; no implementation or new fault precedes
independent design review. Existing positive coverage and original failures stay.
