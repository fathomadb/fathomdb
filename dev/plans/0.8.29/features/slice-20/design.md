# Slice 20 continuation design

## Reconciled product architecture

Actual657edeb owns sidecar/resolver hooks per Engine; graph hooks were already
scoped. Preserve those accepted transaction positions, evidence-only carriers,
ordinary-search layout and free export paths. Actual60 leaves Registry admission
unchanged but adds route counters and format-bound schema validation; actual70
adds bounded fallback causes and Closing propagation. Use current structured
configuration and explicit text search for race fixtures. Do not overwrite those
changes or resurrect old oracle expectations. No production architecture change,
new product fields, queue policy, migration or model/GPU operation is selected.

Owner2026-11-13 seq-369 explicitly approves verification-only A3/A5 proofs and
negative controls without manufactured failing RED. This narrowly supersedes the
passing-coverage restriction in tdd-red-must-fail-per-finding.md for A3/A5 only.
Actual defects still require genuine committed semantic RED then separate GREEN.

## Approved A3 proof and seam architecture

Registry::acquire captures Next::Admit under its mutex, then unlocks before admit.
Free permits bypass QueueTicket; hold_arrivals runs after QueueTicket::join.
After a permit, admit already rejects shutdown, non-Ready phase or changed epoch
before constructing StoreGuard. Load survives reopen; idle/LRU require busy0.
Thus force-draining a queued waiter is invalid and current source suggests missing
proof rather than a demonstrated product defect.

Under seq-369, add one adapter-owned test-seams observation,
consumed once in Next::Admit after registry unlock and before admit/permit/queue.
Arm returns an owned handle tied to exact store plus unique attempt token; refuse
an outstanding duplicate instead of overwrite. Reached carries captured epoch and
Load counters. Async bounded release waits must not park a Tokio thread or retain
registry/ownership mutexes. Guard Drop clears reached/held state after abort,
unwind, timeout or release_all; old handles cannot clear a newer arm. Only armed
attempts collect bounded stale-rejection/bridge-entry events. Pass captured epoch
through Hooks::before_call; no product result/ordinary feature state is added.
An observer must not extend original call wait_until or acquire a permit/ticket.

Use one real A store seeded with independently expected l:note:a. Configure
structured output, explicit text search, idle1s and wait10s. Open epoch1, arm,
spawn search, await reached(epoch1,queued0,in_flight0), observe closed with existing
readiness helper and can_open lock proof. Close that probe before resuming.
Block existing reopen seam, resume old attempt, observe stale rejection and
reopening with no old-epoch bridge entry. Release reopen and assert exact ref,
epoch2 only, zero accounting, later call and real idle/LRU health. Bound every
actor with cleanup installed before arming; existing Harness release_all remains
available. Idle proves the stale recheck; do not duplicate LRU unless a distinct
branch obligation appears. The timer is std::time::Instant, so paused Tokio time
alone cannot prove this transition.

Separate controls keep reopen held through the original absolute deadline and
require exact not_ready/reopening; abort/drop/panic releases the held seam; and
shutdown at the gap refuses not_ready/draining without bridge entry. Keep existing
queued/running cancellation semantics: no new pre-permit client-cancel guarantee.

Negative controls use disposable parent-created isolated checkouts with identical
new tests. First replace stale rejection's `return Ok(None)` with typed
not_ready/draining: the unchanged primary test must fail its wait/reopen/success
oracle. Second bypass the stale phase/epoch predicate: current closed-state
fixture may panic at the absent Engine; retain this as sensitivity failure,
without claiming it proves actual stale engine entry. The healthy proof itself
requires a reached stale-rejection event plus zero old-epoch bridge entries, not
only final success. Record each exact patch/assertion; never mutate the good
candidate to obtain RED. These are coverage sensitivity controls only.

## Approved A5 live tripwire

Add engine fusion unit guard for actual RRF_K/RRF_WEIGHT_TEXT/RRF_WEIGHT_VECTOR
and candle_bge module guard for DEFAULT_EMBEDDER_NAME/private MAX_SEQUENCE_TOKENS.
Accepted expectations are30.0/3.0/1.0/fathomdb-bge-small-en-v1.5/512; each assertion
names dev/design/overflow-chunking-reevaluation.md. Run default-embedder feature
explicitly without constructing any model. No new production export, text parser
or source-regex test. Five separate disposable mutations change one live constant
at a time (31/4/2/different name/513); corresponding unchanged assertion must fail.
These are sensitivity controls, not five fake product RED/GREEN pairs. T1 remains
unmeasured and T3 unscheduled; no answer campaign or chunking decision follows.

Do not alter product predicates or constants to manufacture RED. Any actual
new defect gets a separately committed unchanged failing regression before its
repair. The healthy candidate and sensitivity mutations remain distinct.

## A6 current Windows completion

The [current plan](plan.md#ordered-windows-completion) replaces stale live-disk/
readiness assumptions. November14 VM is shut off; original inactive disks are
intact, same uniquely owned image remains and previous live-only attachment is
absent. Refresh guest facts during parent-scheduled bounded VM window; restore
original off state through normal shutdown. No old endpoint/disk number/RAW/
C free-space sample establishes current readiness.

A narrow reviewed reattachment selects only existing exact image, matching
VM UUID3a928939-57ed-4c62-999b-b610f3e44d42, sdg/USB0:10/serialFDB29S20A6 and
current QEMU UID. Require unused target/port/serial, unchanged original disks
and inactive XML. No image creation, ACL, format or persistent branch. Existing
create-once capacity tool correctly refuses an occupied image and is not reused.

Existing format-only source at Git396a023e02580a1bb82ddcb7f16817874ad0046c has
qualified26 in-memory guard/three transport-refusal controls. Reuse original
results where inputs are equivalent; preparation is not live storage proof.
Actual owner formatting answer remains required for exact64GiB/nonboot/nonsystem/
RAW serialFDB29S20A6 only: GPT, new maximum R: partition, quick NTFS. Original
disk identities are denied and exact target/partition correspondence rechecked
before each mutation. Started synchronous storage may finish after transport
loss; UNKNOWN stops later host requests pending read-only resolution, no retry/
rollback. Seq-370 is capacity-only, never broader formatting authority.

Retained frozen5122 operator sources supply parse/probe/compile/private Job
Object containment, source transfer/extraction and four A6 selectors. Adapt
only current owned source/task/image/accounting and discovered endpoint into
tracked experiment tools. Actual PowerShell parser, existing compiler/reference
hashes and Reflection.Emit null-handle ABI check precede supervised csc. Child
creation enters private no-breakaway kill-on-close Job Object with restricted
stdio inheritance, original absolute lease/deadline and authoritative active-zero.
Independent actual binary/hash review precedes child/death/lease/transport
controls, then exact source-bound A6. No alternate compiler or tool installation.

Host start/stop root32/20GiB, taskfs250/50GiB, MemAvailable16/8GiB and200GiB
owned cap include the existing64GiB image reservation. Guest C8/5GiB, owned
workspace32/20GiB and RAM16/8GiB remain workload floors, with no silent waiver or
cleanup. Actual November14 C free6.37GiB and post-start host memory13.63GiB
fail their respective start floors; they are actual workload holds. One parent-coordinated heavy phase. Source archive must be clean/exact,
hash verified on both ends and per-file extracted on owned NTFS. Build jobs2/
1200s; each of four exact selectors90s/nonzero count; preserve original raw
outputs/exits and private-job-empty receipts. Missing termination outcome is
UNKNOWN. No Windows F64/CUDA input is inferred.

Caller-path/canonical-path mismatch and opener's unbounded Barrier remain
hypotheses. First run existing semantic oracles inside outer bounded process.
An actual current failure gets committed semantic RED before minimal repair,
retaining exact provider/SQLite/profile/admission/error/lock/schema37 assertions.
New helper defects retain observed failure before correction. No fabricated
compile/import/flag RED, backdating or unnecessary passing-check replay.
