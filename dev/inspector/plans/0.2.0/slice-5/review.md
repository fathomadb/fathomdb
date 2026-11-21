---
title: Inspector 0.2.0 verification adequacy assessment
date: 2026-11-19
status: REVIEW
---

# Requirements, tests and critical-path adequacy

Current disposition: owner S9-010, applied in [Slice 10](../slice-10/status.md),
qualifies supported single head windows in [0.2.0 scope](../scope.md) and defers
within-source multiple-window positive. Earlier findings/draft proposals below
retain their historical meaning; cache-only CPU Engine serving remains mandatory
and unqualified in 40.

Actual source `aef5b2eac19dbcf5850713081ccd427896909eea`. Read Slice 5 plan,
design/filled prompt, canonical criteria and every M01–M40 row, 0–4 outputs,
port evidence and selected real test bodies. Keep existing independent-oracle
strategy and single matrix. Adjust draft's “master/workflow not present” fact:
both now exist; adjust window feasibility using S4-WINDOW, not assumed
multiwindow production support. No test suite/campaign or source edit ran.

## Bidirectional coverage review

The single matrix covers every adopted INS-01–09/V01/X/F/T and High group;
no new user need or extra canonical AC is justified. M40 explicitly covers the
new comparison. Current owner cells remain legacy and need S3-MAP reconciliation.
The following grouped findings join to existing rows, not a replacement matrix.
Together groups account for M01–M40 once; cross-cutting contributions remain.

| Existing row group | Evidence class and meaningfulness | Smallest remaining check / delivery |
| --- | --- | --- |
| M01–07,09–13,15,17,18,24,29,32 | Record foundation assertions and browser flows exist; port records scoped execution, not final release. Real ordinary-write fixtures, identity/value/association tests and browser document/clipboard paths oppose wrong class, merged equal text, truncated values and guessed origin. | 20 revalidates carried gaps on selected build; 30/40/50 add their actual route/origin/state cases. Keep exact operational collection/key/log semantics. |
| M08,19,27,38 | Canonical edge/temporal storage tests do not establish directed graph interaction, dependency tracing or retained history. Worker lacks graph query route. High remains mandatory. | 50 prewritten directed adjacency/edge-body/revision/dependency manifest, self/parallel/cycle/disconnected/missing endpoint, real expand/collapse/keyboard. 70 separate High human session. |
| M16,21–23,25,26,30 | Native lexical/vector evidence and model positive remain PLANNED. S4 confirms public snippet routes, different BM25F path, and no current Inspector model integration. A negative unavailable result cannot detect incorrect actual ordering/distance/origin. | 30 tiny authored lexical/tokenizer/score corpus plus actual match/origin UI; 40 f64 reference L2 from stored f32, named population/stages/tolerance/ties, and real cache-only CPU Engine query. S4-CACHE/WINDOW prerequisites remain open. |
| M20,39 | `disclosure.rs` tests eight flag combinations, real ExplicitMaintenance queued content/export and unrelated source; real 8,192-target/200-unrelated case checks budget/freshness after withdrawal. `queued_scope.rs` progress-handler interruption asserts budget_exhausted. Port records RED/GREEN and focused pass. | Keep transaction-local membership regression. Every new snippet/vector/edge/pin/copy/export delivery needs independent I/E/T overlaps and withdrawal/late-read behavior; 20 common, 30–60 extensions. Atomic absence is correct only for genuinely absent targets. |
| M14,28,37,40 | Person native-view path, two actual pins and unseen transfer/human evidence absent; cache/response-order tests provide actual held-response and invalidation seams, not comparison qualification. | 60 overlap/disjoint typed IDs/revisions/origins, per-side request/filter/limit/context/metric, no rerun, caps/refusal/eviction/late results; 70 five real participants with ordinary docs/fixed rubric plus High. No invented participant or four-of-five release threshold. |
| M31,36 | Fully inspected resources.mjs drives real owned Engine child but pauses it synthetically; tests one executing/eight waiting/excess busy, <=250ms ack, no late counts, child reap <=30s, sampled RSS growth and lock release. It does not execute a native model/search during cancel. | Each feature tests its actual native operation and finite owned close; 70 aggregate resource/heap/scratch/handle checks. Retain “synthetic_owned_blockage_not_native_interruption” label and distinguish sampled RSS from observed peak. |
| M33–35 | Existing auth/build/Host/origin/inert-content and document keyboard tests cover foundation. package.mjs packages default release/help/build/checksum and no dev runtime; launch.rs native ignored case consumes browser bootstrap then SIGTERM, not installed all-view window close. | All new render sinks/links receive malicious markup/network/log tests; actual keyboard graph/vector/compare interactions; 70 supported installed default offline all-view/native-window-close and resource release. S0-ABI is prerequisite. |

## Critical path and test-strength conclusions

The product goal is not satisfied today: record → real lexical result → supported
vector representation/similar person → stored directed edge with body/history →
original observation → compare two retained observations cannot run because the
new routes/UI are absent. Requirements and AC are sufficiently explicit to fail
that incomplete implementation; the problem is implementation/executed evidence,
not a missing product criterion. Do not add labels-only tests to make it appear
complete or defer each view's local errors/accessibility to 70.

Preserve independent fixture intent: ordinary Engine writes declare exact IDs,
revisions, body/source links and directed adjacency before Inspector reads;
lexical facts use written token corpus and selected scoring family; vector
expectations use independent finite f64 L2/tolerance from stored f32. Actual
returned FTS snippets establish backend match evidence, not independent origin
identity. Never derive expected retrieval ranks from Inspector output or share
adapter predicates as an oracle. Future property tests target typed references,
UTF-8/chunk round trips and limit boundaries; database integration stays real.

Material proposals, deduplicated with other slices:

- S3-MAP: update only current owner/stage cells; preserve criteria/methods and
  historical execution. Include new comparison M40 under 60, final installed 70.
- S4-CACHE/WINDOW: resolve mandatory feasibility before prescribing a passing
  positive fixture. No synthetic multiwindow insert, private helper, Engine edit
  or unsupported/negative-mode waiver under Inspector commission.
- S5-METHOD: clarify in matrix-linked method text that native cancel/close,
  installed window interaction, measured peak versus sample and all-view keyboard
  remain outstanding. Explicitly retain existing real queued-positive coverage;
  no blanket absent-erasure waiver. Proposed 10 text-only if approved; actual
  tests stay with 20–70 feature owners.

Historical scope stays precise: final port ordinary run is 37 unit/real-DB,
one CLI, one no-GUI, two worker tests; four explicit native/scale/allocation
cases unrun there. Earlier 35-test entry is before remediation, not conflicting
new execution. Missing checkout-owned .venv stopped full gate. Independent
Astra rerun of compiled tests was not a new build. No current runtime green.

Quality roadmap: adopt targeted first/persistent ownership faults, cancel/late/
resource cases, properties and meaningful wrong-condition checks when an actual
risk warrants them. Defer global MC/DC, generic SQLite/VFS/power-loss fuzzing,
all-allocation and broad platform/ANN/relevance campaigns. Independent review/verification and scoped Markdown limits are in [the shared record](../slice-9/prework-verification.md).
