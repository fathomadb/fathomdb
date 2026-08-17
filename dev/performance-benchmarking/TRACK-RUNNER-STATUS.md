# Track Runner status

This is the coordinator-owned, live progress board for the
[performance benchmarking and experiments program](PROGRAM.md). It records
coordination state, not measurements: receipts and the append-only experiment
index remain the source of execution evidence.

**Last reconciled:** 2026-08-16

**Integration base:** `acc5fa9a` on
`experiments/performance-experiments-20260815`. This is the verified integrated
LOCOMO-01/SCALE-01 preparation set and its accepted cross-lane correction;
replace it with the verified integration SHA at each accepted lane close.

## Current lanes

`LOCOMO-01` handed off `6bfc1004` from
`/tmp/fathomdb-locomo-01-20260816` on
`experiments/performance-locomo-01-20260816`; independent review requested a
targeted correction before integration. The review accepted its red-first,
no-live scope and pin evidence, but found that treatment IDs could retain their
names while their semantic parameters drifted. Remediation handoff `65427d29`
adds red-first mutation coverage and fail-closed, type-checked validation for
all six frozen treatment mappings; follow-up independent review accepted
`65427d29`, integrated here as `4bbc1b4f`. Its full verifier passed Rust but
stopped at Python collection because this isolated worktree has no local native
binding; that environmental limitation is recorded in its handoff, not treated
as a green gate. The integrated catalog is plan-only; no LOCOMO execution is
authorized.

`SCALE-01` handed off `9997c2cd` from
`/tmp/fathomdb-scale-01-20260816` on
`experiments/performance-scale-01-20260816`; independent read-only review
accepted it and it is integrated here as `afbfeed2`. Its red-first, focused 10-test, full
80-experiment-test, and full `./scripts/agent-verify.sh` evidence passed
(`72/73` suites, one documented skip). The worktree-local native binding used
for that verifier is ignored local state, not a source change. No TC-5
execution is authorized.

The final cross-lane review accepted the integrated safety, configuration,
receipt, and no-live boundaries at `acc5fa9a`, after focused review accepted
the active-identifier correction from legacy `L0`/`T0` to
`LOCOMO-01`/`SCALE-01`. The unchanged combined full verifier passed
(`agent-test.sh: 72/73 suites passed`, one documented skip; AC-037 had its
expected environmental downgrade). No implementation or execution evidence was
challenged.

HITL authorization `seq-249` permits: the LOCOMO fixed-subset dry run and
Phase-B CPU/FTS grid; LOCOMO GPU/CE cells; the SCALE TC-5 smoke and long CPU
characterization; and CORPUS-01 corpus and human-gold work. These permissions
do not commission a writer or replace the required frozen run configuration,
worker handoff, review, safe receipt/index, or artifact boundaries. No paid
service, extractor, push, or downstream ANSWER-01/MEMORY-01/SCALE-02 execution
is authorized.

HITL decision `seq-250` approves `parent_child_turn_session_v1`: its exact
frozen amendment is linked from PARENT-01. Later empirical variants remain
permitted only through a new frozen amendment and normal review. The 0.90 TC-5
goal remains in force; a current-configuration ground-truth remediation is
authorized but must first receive a dated diagnostic contract. SCALE-02 claim
policy remains open pending an HITL proposal; no product policy is authorized
before initial measures. The coordinator alone edits this board and PROGRAM
state.

Two preparation-to-execution implementation lanes were commissioned from
campaign base `378a8214`: `LOCOMO-01` plus approved `PARENT-01` in
`/tmp/fathomdb-perf-locomo-parent-exec-20260816` on
`experiments/performance-locomo-parent-exec-20260816`, and `SCALE-01` in
`/tmp/fathomdb-perf-scale-exec-20260816` on
`experiments/performance-scale-exec-20260816`. The SCALE preparation lane was
independently accepted at worker SHA `61142179` and integrated here as
`ec9978d5`: its red-first TC-5 execution boundary, safe configuration,
content-free receipt projection, and dated ground-truth remediation contract
are now the campaign control. Neither lane may acquire a corpus or run a
smoke, grid, model, GPU, or external write until the coordinator releases the
corresponding frozen execution step.

The LOCOMO/PARENT preparation handoff `4e444d19` received an independent
`REQUEST-CHANGES` verdict. The reviewer accepted its initial red-first history
and immutable cell grid, but found four execution-contract gaps: fixed-subset
dispatch order drift, receipts that could claim completion without bound cell
coverage and required metrics, parent bundles without membership/ordinal proof,
and a receipt-verdict correction without a red test. The worker is remediating
these issues in its isolated worktree; no LOCOMO/PARENT execution is released.

The red-first LOCOMO/PARENT remediation `77a0c700` was independently accepted
and is integrated here as `d0a88779`. It fixes the reported execution-order,
complete-receipt, relation-proof, and test-first gaps; the integrated focused
Phase-A/Phase-B tests and safe validation/preview pass. This remains a safe
adapter boundary: a separate live executor must be reviewed and released before
the authorized fixed-subset dry run uses external inputs.

The next SCALE-01 lane is commissioned from campaign base `a79ab744` in
`/tmp/fathomdb-perf-scale-live-executor-20260816` on
`experiments/performance-scale-live-executor-20260816`. It owns only the
external-only live-executor implementation and tests. The executor remains
disabled and must receive independent review and coordinator integration before
the authorized smoke or characterization is invoked.

The LOCOMO/PARENT live-executor lane is commissioned from campaign base
`4c9e3216` in `/tmp/fathomdb-perf-locomo-parent-live-executor-20260816` on
`experiments/performance-locomo-parent-live-executor-20260816`. It owns only
the external-only runner/configuration and tests. It remains disabled pending
its independent review and coordinator integration; it may not run the
authorized dry subset or any full CPU/GPU cell during implementation.

The SCALE live-executor handoff `98b0b87f` received an independent
`REQUEST-CHANGES` verdict. Its red-first history and scope were sound, but the
review found a generated-output symlink escape, non-finite uncertainty
acceptance, missing release/config/runner digest evidence in durable receipts,
and omitted runtime provenance fields. The worker is remediating these issues
in its isolated worktree; no SCALE execution is released.

The first SCALE remediation handoff `49088175` received a second
`REQUEST-CHANGES` verdict on evidence, not a known remaining runtime defect:
the new tests reached a generic provenance rejection before proving their
specific containment, receipt, finite-uncertainty, and runtime-provenance
claims, and one release-binding case was already green before the fix. The
worker must provide an unmasked red checkpoint and correct its handoff claim;
no SCALE execution is released.

The LOCOMO/PARENT live-executor handoff `b70ca4e1` received an independent
`REQUEST-CHANGES` verdict. Its red-first history and general release boundary
were sound, but full TRACE validation, canonical parent-manifest binding,
same-release full-grid closure, actual GPU attestation, duplicate-key-safe
adapter parsing, and accepted-review-evidence binding were absent. The worker
is remediating all six points in its isolated worktree; no LOCOMO/PARENT
execution is released.

The SCALE final replay review found the hardened executor sound, but issued a
last `REQUEST-CHANGES` for one inaccurate evidence claim: pre-hardening code
already rejected negative-infinite uncertainty (with a weaker diagnostic), so
the replay proves seven behavioral defects rather than eight. The worker is
correcting that historical replay and handoff wording only; no SCALE execution
is released.

The final SCALE live-executor handoff `4c30072f` was independently accepted
and is integrated here as `e87be86d`. It carries the release-bound,
external-only executor, hardened containment and provenance checks, seven
unmasked historical defect replays, and accurate negative-infinity regression
coverage. The next gate is a coordinator-issued release record plus factual
external manifest/corpus/output preflight before the authorized smoke.

The LOCOMO/PARENT live-executor remediation `2520bd48` was independently
accepted and is integrated here as `655ec77c`. It enforces full TRACE
validation, canonical parent-manifest binding, exact same-release 52-cell
closure, actual CUDA/adapter attestation, duplicate-key-safe adapter parsing,
and review-evidence binding. The next gate is factual external provenance and
environment preflight plus a coordinator-issued release before the authorized
fixed-subset dry run.

`CORPUS-01` is commissioned from campaign base `d07c551e` in
`/tmp/fathomdb-perf-corpus-01-20260816` on
`experiments/performance-corpus-01-20260816`. The lane may prepare the
versioned corpus/license matrix and human-gold protocol under the existing
external-data boundary. It may not acquire a new payload or manufacture gold
answers during implementation.

Read-only execution preflight found no qualified release inputs yet. A
gitignored LOCOMO corpus root is present outside this worktree, but no exact
external manifest/provenance, accepted TRACE sidecar, parent-relation proof,
or released adapter has been qualified. TC-5 likewise lacks its qualified
all-real manifest, external driver, and output root. `nvidia-smi` is present
but cannot communicate with the NVIDIA driver, so GPU/CE cells remain an
environmental blocker despite the installed CUDA compiler. These are factual
prerequisites, not a new authorization decision.

The LOCOMO external-adapter preparation lane is commissioned from campaign
base `efc21558` in `/tmp/fathomdb-perf-locomo-external-adapter-20260816` on
`experiments/performance-locomo-external-adapter-20260816`. It may implement
and test the released adapter ABI against synthetic fixtures only. It may not
read the corpus payload, invoke a model/device, generate a release, or run an
authorized measurement.

The CORPUS-01 handoff `13d63a97` received an independent `REQUEST-CHANGES`
verdict. Its portfolio coverage and no-payload boundary were sound, but it did
not fail closed on unsupported corpus/category claims, lacked machine-traceable
license/preflight/claim/power provenance, and did not provide committed
red-first topology. The worker is remediating these issues; no corpus or human
gold work is released.

The LOCOMO external-adapter handoff `70284d48` received an independent
`REQUEST-CHANGES` verdict. It must enforce frozen action/cell admission,
preserve ranked results and PARENT rank, prove contiguous parent ordinals,
derive rather than fabricate parent metrics, and contain generated output at
the adapter layer. The worker is adding red-first corrections; no adapter is
integrated or execution released.

The CORPUS-01 remediation `aa52220a` received a second `REQUEST-CHANGES`
verdict. Its native/qualified-human-gold model and factual preflight bindings
are sound, but an `approval_ref` can be syntactically forged because it is not
bound to a trusted amendment approval and exact corpus/category pair. The
worker is adding a content-free trusted approval registry requirement; no
corpus or human-gold work is released.

The LOCOMO external-adapter remediation `e5b6f17d` was independently accepted
and is integrated here as `477ad51a`. It enforces exact frozen cell/action
admission, rank-preserving parent proof and measured parent metrics,
contiguous canonical ordinals, and output containment. The CPU fixed-subset
still needs qualified external corpus/provenance/TRACE/parent inputs and a
coordinator release; GPU cells additionally need a working NVIDIA driver.

The CORPUS-01 trusted-amendment handoff `1ca17718` received a third
`REQUEST-CHANGES` verdict on proof and verification evidence. Its registry
comparison is sound, but its red test used a new API keyword instead of
demonstrating legacy forged-sequence acceptance, did not cover mismatched
registry rows, and omitted recorded post-fix full-verifier evidence. The
worker is correcting those narrow gaps; no corpus or human-gold work is
released.

The CORPUS-01 corrected handoff `cb920dd2` passed its code and red-replay
review, but received one final documentation `REQUEST-CHANGES`: it did not
durably record the already-green post-fix full-verifier result and still used
future-tense rerun wording. The worker is making that docs-only evidence
correction; no corpus or human-gold work is released.

The TC-5 external-driver preparation lane is commissioned from campaign base
`8101f1db` in `/tmp/fathomdb-perf-tc5-external-driver-20260816` on
`experiments/performance-tc5-external-driver-20260816`. It may implement and
test the CPU driver ABI with synthetic fixtures only. It may not inspect an
all-real payload, load the pinned model, run a smoke, or write a campaign
artifact during preparation.

The TC-5 driver worker committed an unintended shared
`fathomdb-py` test-hook change in its first implementation. That commit is not
eligible for review or integration. The worker is preserving history and
creating a corrective commit that restores the shared binding exactly to its
campaign-base content and removes any dependency on that hook; no TC-5 action
is released.

The corrected TC-5 driver handoff `219de861` restored shared scope and passed
its ABI review, but received one `REQUEST-CHANGES` for direct-invocation
safety: it checked for an existing result sidecar only after corpus/runtime
callbacks. The worker is adding a red-first early-destination rejection before
those callbacks; no TC-5 action is released.

The TC-5 external-driver remediation `426be68e` was independently accepted
and is integrated here as `d1a6b1aa`. It has no net shared-binding change and
rejects an existing result sidecar before input/runtime callbacks. The next
gate is factual qualification of the all-real manifest, corpus/output roots,
CPU host/model/ground truth, and a coordinator release before the authorized
smoke.

Two factual-input qualification lanes are commissioned from campaign base
`febd1155`: LOCOMO/PARENT in
`/tmp/fathomdb-perf-locomo-input-qualification-20260816` on
`experiments/performance-locomo-input-qualification-20260816`, and TC-5 in
`/tmp/fathomdb-perf-tc5-input-qualification-20260816` on
`experiments/performance-tc5-input-qualification-20260816`. They may inspect
the authorized external corpus roots and create only content-free external
preflight manifests/proofs under declared external roots. They may not issue
releases, load models, select GPU, run an adapter/driver, or measure a cell.

The CORPUS-01 final handoff `c540a9d7` was independently accepted and is
integrated here as `c419399e`. The matrix/protocol now fails closed on
unsupported native claims, unqualified human-gold amendments, incomplete
factual preflight, and untrusted approval bindings. The next gate is a
coordinator-created trusted amendment registry only when a real, approved
human-gold batch needs it; no payload or human-review work has yet begun.

## Track status

| ID | Portfolio state | Runner state | Verified evidence / next gate |
| --- | --- | --- | --- |
| SAFETY-01 | Complete infrastructure | Closed; re-check on each new track | Safe receipt/index contract exists; retain as campaign control. |
| TRACE-01 | Complete canary | Closed and integrated | `ca5b656d` integrates the independently accepted `a4a7ed0b` history: three red-first fixes, 10 focused tests, and a full `agent-verify` pass. |
| LOCOMO-01 | Active | External adapter integrated; external-input qualification and release pending | `477ad51a` integrates independently accepted `e5b6f17d`; 25 integrated synthetic tests pass. `seq-249` authorizes the fixed-subset dry run, Phase-B CPU/FTS grid, and GPU/CE cells. Next gate: qualify external inputs, issue a coordinator-bound CPU fixed-subset release, then run it. |
| PARENT-01 | Active | Matched external adapter integrated; external-input qualification and release pending | `477ad51a` enforces exact ordinal/rank and measured parent metrics. `seq-250` approves `parent_child_turn_session_v1`. Next gate: qualify the same external inputs and issue a matched CPU release; GPU cells additionally wait for a working NVIDIA driver. |
| SCALE-01 | Active | Live executor and external driver integrated; all-real input preflight and release pending | `d1a6b1aa` integrates independently accepted `426be68e`; 54 integrated focused driver/executor tests pass. `seq-249` authorizes the TC-5 smoke and long CPU characterization; `seq-250` retains 0.90 as the goal and authorizes a separate remediation. Next gate: qualify manifest/corpus/output/CPU/model/ground-truth facts and issue a coordinator-bound smoke release. |
| CORPUS-01 | Active | Matrix/protocol integrated; factual qualification and any approved human-gold batch pending | `c419399e` integrates independently accepted `c540a9d7`; 21 integrated focused tests pass. `seq-249` authorizes corpus/license matrix and human-gold work. Next gate: qualify existing external corpus facts and, only if required, create a coordinator-bound trusted amendment registry before a human-review batch. |
| ANSWER-01 | Blocked | Waiting for selected retrieval survivor | Require LOCOMO/PARENT selection plus scorer and cost preflight. |
| MEMORY-01 | Blocked | Waiting for ANSWER-01 | Require selected profile, native prerequisites, and declared spend ceiling. |
| SCALE-02 | Blocked | Waiting for initial measures and selected profile | Claim policy remains open pending an HITL proposal. Freeze workload matrix and distinguish canonical from derived counts after the evidence exists. |
| LATENT-01 | Parked | Not commissioned | Requires diagnosed cross-window discourse failure. |
| GRAPH-01 | Planned | Not commissioned | Requires bounded graph design and supporting-evidence protocol. |
| GLOBAL-01 | Parked | Not commissioned | Requires GRAPH-01 relevance and native cost/reproduction preflight. |
| REASON-01 | Parked | Not commissioned | Requires GRAPH-01 relevance and external Python/credential/corpus prerequisites. |
| SEARCH-01 | Complete historical | Closed | Retained FTS descriptive baseline; no current execution. |

## Update protocol

Only the Track Runner coordinator edits this file. Update it in the same
integration change whenever any of these events occurs:

1. A lane is commissioned: record its base SHA, branch/worktree, owner role,
   owned paths, and the next gate in **Current lanes**.
2. A worker hands off: record the commit SHA, test/verification result,
   independent-review verdict, and any blocker or authorization request.
3. A lane is integrated, rejected, blocked, resumed, or closed: update the
   affected row, PROGRAM/charter state if it truly changed, and link any safe
   receipt or run ID.
4. A final cross-lane review closes: record its reviewed integration SHA and
   the resulting next authorized action.

Never copy raw metrics, corpus payloads, credentials, or model output here.
Never change a row from blocked or planned to complete based on narration: link
the reviewed commit and receipt, then apply the charter's exit rule.
