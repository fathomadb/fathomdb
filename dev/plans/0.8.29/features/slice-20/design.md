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

## A6 capacity-only preparation and later qualification

Owner2026-11-13 seq-370 approves creation/necessary narrow ACL/live attachment of
one uniquely owned64GiB scratch disk to the existing Windows VM. It does not
approve GPT initialization, partitions, formatting, drive assignment, compiler,
helper or test execution. [Capacity-only proposal](capacity-only.md) and its
capacity-only.py derivative stop after live attachment and read-only identity.
Frozen5122aa08 remains unchanged. Do not run its provision-host.py: that script
continues into provision-guest.ps1 initialization/format after attachment.

Current read-only libvirt refresh confirms VM UUID3a928939-57ed-4c62-999b-b610f3e44d42,
original sda OS image, USB0 qemu-xhci15ports, unused sdg/USB0:10/serialFDB29S20A6,
and DAC UID64055/GID993. This is topology observation, not disk readiness or A6.
Latest host sample: root80,429,350,912B, /mnt/data609,778,946,048B,
MemAvailable30,873,688KiB. Refresh immediately before work; current read-only guest
C:7,055,458,304B (6.57GiB), RAM30,404,480KiB, only Disk0/QM00001 and no native
processes. C: is below the later8GiB workload start floor. Capacity-only stages
nothing on C: and has no guest free-space consumption; that workload floor does
not apply to host create/attach. C:8/5GiB remains the later start/stop policy.
The script reads current guest disk identity,
C:/RAM/build-process inventory before mutation and stops on unavailable facts.

Later A6 retains the four exact selectors and all original typed-error/provider/
SQLite/profile/admission/lock/schema assertions in test-map.md. The inherited
operator package has static source review only. Actual PowerShell parsing,
Reflection.Emit ABI, existing compiler/reference hashes, supervised compilation,
independent binary review, private-job death/lease/transport controls and exact
source transfer/extraction precede native A6 execution. These remain technical
and authority gates; capacity-only approval does not silently open them. No
cross-build route or alternate compiler/install fallback is selected. Transfer
standing approval remains as in windows-vm-for-fathomdb-testing.md, limited to
exact-source disposable verification. Preserve source-bound/native identity.

If Windows reveals a real path-hook or opener-rendezvous defect, diagnose under
bounded execution, commit a deterministic current semantic RED, then repair with
unchanged intended assertions. Capacity preparation alone cannot close A6 or20.
