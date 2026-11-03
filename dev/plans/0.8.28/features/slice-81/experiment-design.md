---
title: FathomDB 0.8.28 Slice 81 — experiment design
status: PLANNED
target_release: 0.8.28
---

# Slice 81 experiment design

Normative obligations: [requirements](requirements.md). Mechanisms and seams:
[draft design](design.md). Authorities/previous evidence: S81-3/4/5/9 in the
[reference index](README.md#references). No experiment has run under this protocol.

## Hypotheses and assumptions

| Hypothesis | Expected outcome | Falsifier |
| --- | --- | --- |
| H81-1 | Eager secure cleanup and completed incremental merging remove legacy and fresh target traces in qualified modes. | A positive physical marker survives claimed completion, or unrelated content is lost. |
| H81-2 | Committed incremental units reduce long writer occupancy while preserving unrelated service. | No progress between units, repeated whole-operation rollback/restart, or no consistent blocking reduction versus A. |
| H81-3 | Accepted cleanup survives crash/reopen and completes without later user traffic under a finite reserved-service policy. | Lost intent/deadline, idle starvation, unsafe replay or scope resurrection. |
| H81-4 | Reader leases/admission plus a measured finalization window can bound completion under an explicit operating envelope. | Unbounded pinning/finalization, or completion requires a pause outside the proposed envelope. |
| H81-5 | Correct denial/membership can coexist with pending physical bytes and safe unrelated writes. | Authorized score/cap/graph/evidence contamination, unsafe fence bypass, or actual Engine still globally blocks. |
| H81-6 | Ordinary expiry/supersession can be evaluated separately from source erasure, avoiding unnecessary eager physical work where permitted. | Lower cost depends on deleting authorized history or weakening later erasure completion. |

Assumptions to test, not claim: production SQLite/profile is identical in every
child; merge work commits and converges under admitted ingest; a page/row budget
is a useful control rather than a time guarantee; closure expansion is bounded
at admission; copied/staged data is fully inventoried; all blockers can be retired
within the supported envelope. Storage failure, absent process or unavailable
original sink cannot be silently excluded from accounting after acceptance.

## Stages and candidate order

1. **G0 inventory/diagnostics:** reproduce unchanged shipping ghost and native global
   fence; inventory actual registered modes. Freeze seam/coverage matrix.
2. **G1 functional gate:** implement A eager reference, then B incremental merge,
   with physical, durability and safe-admission tests. Reject unsafe families;
   do not time them as successful candidates. Test idleness and explicit failures.
3. **G2 core paired measurement:** run the matrix below on frozen code/data. No
   build or test campaign overlaps timing. Run engine-owned availability witnesses
   separately from raw SQLite/mechanism scouting.
4. **G3 conditional C:** try staged replacement only if B fails physical convergence,
   durable progress, inter-unit service, or the comparative rule below. Freeze its
   delta/cutover/retirement design and obtain Astra review before implementation.
   Use the remaining fixed C budget; preserve B's failure and all original cells.
5. **G4 synthesis:** separate safety, mechanism, actual-Engine availability and SLA
   verdicts. Recommend/reject with scope, costs and successor requirements.

A/B rejection can be conclusive without C if C cannot meet reviewed safety/scope
or requires a production redesign. Record the reason; do not silently drop C.
An unavailable Engine composition seam is an unqualified integration result, not
a license to substitute a standalone adapter and call Engine availability solved.

## Fixtures and inventory

Use fresh disposable copies of Engine-created current-format fixtures. Three
separate family fixtures, each with 10,000 governed owners and four retained prior
body revisions where that family actually retains them:

| Fixture | Explicit base and target |
| --- | --- |
| Node | 10,000 logical document owners, 50,000 canonical body revisions; target 1,000 logical owners and all their versions. |
| Edge | 10,000 logical body-bearing edge owners, 50,000 retained edge revisions, plus 10,001 stable endpoint nodes outside this family; target 1,000 edge owners without erasing endpoint sources. |
| Property | 10,000 logical owners with two searchable scalar names; 20,000 current property documents. Historical canonical revisions and FTS ghosts are counted separately, not advertised as maintained property rows. Target 1,000 owners and both values. |

Actual post-bootstrap/post-supersession counts must match the manifest or refuse
the cell. Prove source/dependency ownership, endpoint and family membership before
use. Inventory `search_index_v2` separately: populate legitimately if supported;
otherwise label dormant/unsupported. No empty-family physical-removal pass.

Primary indexed payload is 1,024 UTF-8 bytes using a fixed seeded varied vocabulary,
one common term and a unique nonstemming ASCII witness. Put the witness in the
indexed head and prove presence before deletion. Keep candidate setup identical.
Declare the seed and generator source in the run manifest. Hand-authored correctness
fixtures include zero-token/Unicode/phrase/stemming cases; a 1-MiB body, high-frequency
postings, duplicate values and high dependency fanout stress unit bounds outside
the primary timing matrix. Never extrapolate repetitive-text RSS to arbitrary entropy.

Two physical workloads: **fresh** deletion of live target rows, and **legacy**
whole-family sanitation after ordinary deletion has demonstrably left ghosts.
Legacy sanitation must start with witnesses still present. Do not apply prospective
secure-delete before creating the legacy failure. Target source IDs are disjoint
from unrelated foreground writes. Initial policy refuses same-source reingestion
until physical completion; generation-aware admission is not built in this POC.

## Core matrix and controls

Core = A/B x node/edge/property x fresh/legacy x idle/loaded: **24 cells**.
Each cell uses three fresh paired replicas. Predeclare alternating A/B order by
replica; do not choose order from observed performance. Reuse fixture generation,
query sequence and mutation plan, not mutable candidate databases.

Loaded replica: 15 seconds of open-loop offers, 100 reads/s across eight clients
and 100 unrelated writes/s through one client; predeclare offsets and seeds.
Each client has one in-flight slot; an occupied slot is a recorded offered-request
refusal, not a dropped sample or later replacement. Every issued call includes
queue/admission and service time. Unfinished calls retain IDs and elapsed time.
Each replica's 30-second experiment clock starts before its disposable-file
copy/setup. Loaded replicas run the full 15-second offer window after admission;
remaining replica time is available for completion, physical inspection and drain,
with no more than 15 seconds after the offer window. If setup leaves insufficient
time for the full offer window, record setup-incomplete; never shorten the load
window and call it complete. Idle replicas have no offer window: after admission
the worker has the remaining 30-second replica budget to clean, prove and drain
without any application traffic. These are experiment stops, not deletion SLAs.

Before erasure cells run no-erasure service controls at the same offered rates;
record their cost in the total budget. Public Engine calls and candidate-routed
calls have separate labels. Changing write commit order can change valid results;
exact ranking oracles run at controlled matching cuts, not across live asynchronous
histories. A candidate cannot claim success by refusing all foreground work.

Supplement: two A/B single-replica 10,000-current-record expiry/refresh cells,
reusing P2 scope. Record duration/occupancy and preserved history, not qualified
request percentiles or integrated public acceptance. Do not include these as
source-erasure physical-removal successes.

Initial campaign ceilings: **45 minutes core/control/supplement**, **15 minutes
conditional C**, **60 minutes total**, including fixture preparation and drain;
maximum **90 seconds per core cell**, including setup/copies, all three replicas,
physical inspection and drain, **8 GiB sampled process RSS**, **12 GiB new scratch
use**. Build separately before the timing window. C may run at most six
matched cells selected by the predeclared failed condition, with the same controls;
only measured/qualified families can support its recommendation. Poll resources
at 100 ms and report overshoot; these are stop controls, not hard allocation caps.

Preflight actual free resources; fail before starting if the reserved budget
cannot fit. No automatic extension, replacement run or threshold widening.
Setup failures/campaign exhaustion leave cells incomplete/not run. Any revised
controls require a new reviewed pre-run manifest, preserving the old campaign.
Run each cell in an isolated child. On a stop request controlled termination and
join/reap it; record forced termination as interrupted/incomplete, never completed
cleanup. Record shutdown/resource-monitor overshoot and charge it to campaign
elapsed time, leaving subsequent cells not run if the budget is exhausted. This
process control is not a hard Engine cancellation or shutdown-time guarantee.
These experimental ceilings do not approve a product timeout, queue depth or SLA.

## Measurements and interpretation

Record attempt/success/refusal/error/unfinished counts and durations separately;
publish qualified p50/p99 only for a fully completed replica with at least 1,000
successful observations per measured read/write stream, all scheduled offers
accounted for, zero refusal/error/unfinished outcomes in that stream, and completed
cleanup/physical proof under its protocol. Idle streams have no request quantiles.
Interrupted, failed-service or cleanup-incomplete replicas retain null qualified
percentiles even if more than 1,000 successes preceded the stop; retain raw durations
and all outcome counts, not replacement samples. Use nearest-rank quantiles
and test indices. Report individual paired replicas, not an invented pooled tail
confidence claim. A cell lacking counts or success behavior has no qualified
latency distribution. No-erasure controls failing at the offered rate make the
comparison inconclusive at that load; do not silently lower it.

Capture monotonic accepted, denial-commit, first/last-progress, finalization and
completion timestamps; wall time separately. Measure writer queue/wait and actual
lock-held intervals, worker service/gaps, maximum foreground stall, cleanup bytes,
progress commits, CPU/I/O, disk/WAL/RSS, old-reader age, telemetry and checkpoint
outcomes. Distinguish actual lock instrumentation from a call-duration upper bound.
Physical inspection is outside foreground timing but inside completion proof time.
Record instrumentation overhead with the same controls in both arms.

B supports H81-2 only if safe, services unrelated work between units, and its
median of three per-replica maximum writer-held intervals is at least 25% below
A, with the direction consistent in all three paired replicas for that family/
workload. This is a mechanism screening rule, not a product latency requirement.
Report read/write outcomes and paired p99 changes even if this rule passes;
no selected policy may hide worse refusal or completion behavior behind a smaller
successful-request percentile. A missing baseline distribution cannot be replaced
by inference. Remaining H81 verdicts use their explicit deterministic oracles.

A sampled maximum or finite run cannot prove a universal SLA. Output a proposed
service envelope with capacity/backlog, reader/finalization bounds and clock/
failure assumptions, separating measured margins from unproved worst-case bounds.
If no candidate supports that argument, reject or name the precise next evidence;
do not claim guaranteed deletion from a retry loop or experiment timeout.

## Executable sanitation protocol clarification

Reviewed before execution: the experiment compares sanitation after ordinary
native erasure, without changing native acceptance or fencing. Fresh A enables
FTS secure deletion before population; fresh B uses default FTS and follows native
erasure with committed merging. The full loaded offer window starts before native
erasure in both arms. Record native call duration separately; it is not writer
lock instrumentation. The 25% writer-occupancy comparison applies only to legacy
rebuild-versus-merge units measured through the same writer hook. Fresh writer
occupancy and native deferred-erasure availability remain unqualified.

Legacy paired replicas start from identical default fixtures after successful
native erasure and positive ghost inspection; that preparation cost is charged
to the campaign. Native incomplete outcomes fail a replica, without automatic
retry or sanitation admission. Stop and join foreground clients before physical
proof so inspection has an exclusive fixture. Loaded foreground writes use only
predeclared unrelated sources. No same-source reingestion or scoped admission
claim follows from this restricted workload. Candidate acceptance/completion
clocks are sanitation clocks; the earlier native-erasure interval remains visible.

A dependency-free source's incomplete WAL checkpoint does not necessarily create
a dependency closure, and unrelated writes may proceed. A registered derived
dependency does create a closure and globally fences unrelated writes until exact
retry. Both cases are baseline tests; neither is silently generalized to the other.
The original 24 cells, three replicas, 30/90-second stops and 45-minute campaign
budget remain unchanged. SLA scheduling and ordinary expiry remain separate
successor evidence, not fabricated by the sanitation controller.

## Reviewed continuation after the original campaign

The original campaign stopped after 2700.1545 seconds: ten completed replicas,
13 incomplete replicas and 49 never started. Master preparation consumed much
more time than the draft anticipated: native erasure required 266.16 seconds for
node, 145.04 seconds for edge and 775.21 seconds for property fixtures; property
master generation also required 439.97/510.91 seconds. The original manifest,
results and failures remain immutable under `/tmp/fathomdb-slice81-campaign`.

Under the owner's instruction to adjust the drafted plan and complete this slice,
Astra reviewed a **separate, at-most-25-minute active continuation**. This explicitly
supersedes the original 60-minute combined ceiling for this continuation; it is
neither an automatic extension of the original 45-minute campaign nor use of the
conditional-C allowance. Report original duration, continuation active duration
and total wall span including intervening review/setup separately.

The continuation selects only the original never-started IDs: cell 7 replica 2,
and replicas 0–2 of cells 8–23. It preserves their original alternating order,
30-second replica limit, original accumulated time against each 90-second cell
limit, load, fixture, page budget, physical-proof and percentile requirements.
Never rerun or replace an incomplete or completed original attempt. Reuse only
hash-verified original masters, the unchanged release binary and original service
controls. Start the new active clock before input validation and setup; stop on
its deadline or original resource limits, retaining every remaining disposition.
Combined scratch accounting includes the original retained masters and new copies.

The exact continuation manifest, controller and selection/budget tests receive
Astra review and independent verification before running. No staged replacement,
production remediation, scheduler or SLA scope is added. This adjustment completes
missing observations; it cannot turn the original timeout results into passes.

The continuation's resource observer stopped after 908.8956364342012 active
seconds on a `/proc` permission error near child exit. Its 13 completed and 25
incomplete outcomes remain immutable, including property legacy-idle A replica 0:
the child emitted a physical-completion record, but the parent classified the
attempt incomplete. Do not promote or repeat it.

A scoped observer fix tolerates a permission error only for that child's `/proc`
path after an independent process poll proves it has exited. Permission errors
for live children or other files still fail closed and reap the child. After
RED/GREEN, Astra review and independent verification, a separate recovery segment
may execute only the 11 still-never-started IDs, in original order. Its exact active
allowance is **1500 minus 908.8956364342012 seconds**, not a reset or extension.
Charge verification/setup to that allowance; carry all earlier cell time including
the monitor-failed attempt. Verify original and prior-continuation hashes, reuse
the unchanged probe/masters, and retain separate recovery manifest/results. All
original workload, proof, resource and percentile rules remain unchanged.
