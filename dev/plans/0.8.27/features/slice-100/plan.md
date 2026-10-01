---
title: FathomDB 0.8.27 Slice 100 - PyO3 execution plan
status: BLOCKED_PRE_ENTRY
target_release: 0.8.27
planning_baseline: 5f9433c98
---

# Slice 100 execution plan

## Entry and scope decision

This is a pre-entry reconciliation of the [draft design](design.md), not a
commission or completion receipt. The release-state ladder makes Slice 100
depend on Slice 90. At `5f9433c98`, Slice 90 is `PLANNED`, its stage-2 runtime
checkpoint is `PENDING`, and `scripts/preflight.sh --expect-closed 90 --plan
dev/plans/plan-0.8.27.md` fails. The Slice 90 worktree is a planning branch;
its plan was merged, but no Slice 90 production handoff or closeout exists.
Do not start native movement, characterize a supposedly final binding, or mark
Slice 100 complete until the dependency and checkpoint gates pass against the
actual release candidate. Planning may be reviewed ahead of that entry.

The outcome remains a behavior-preserving decomposition of the 5,943-line
`fathomdb-py/src/lib.rs` into private semantic owners. Slice 100 also owns a
separate, tested disposition of the native logging-subscriber contract. The
Python SDK remains Slice 130; NAPI remains Slice 110; final release-wide
platform/security qualification remains Slice 150. No schema, API, dependency,
async model, or wheel metadata change is part of the structural move.

## Reconciliation since the draft

The prospective design was written at `ab8f43be2` on 2026-09-27 and used
`63091b6249e5805ea71b44de59f0ac6703b786a9` as its reviewed source.
At planning baseline `5f9433c98`:

1. `git diff 63091b6..5f9433c98` has **no changes** to
   `fathomdb-py`, `src/python`, `dev/design/bindings.md`, or
   `dev/interfaces/python.md`. The 5,943-line native root and its no-op
   `attach_logging_subscriber` are still the entry *candidate*, subject to
   re-inventory after Slice 90.
2. Slice 85 recovery and status reconciliation landed. They constrain the
   engine boundary but assigned no new PyO3 work. The release branch also
   merged the Slice 90 planning branch; that did not execute Slice 90.
3. Slice 90's Option B runtime successor ADR is accepted. Its final design
   requires effective five-knob configuration, installed Python/Node evidence,
   an exact candidate-bound binding handoff, a stage-2 checkpoint, and no
   unresolved runtime/projector/operator item. Slice 100 consumes that result;
   it does not implement or requalify Slice 90's runtime correction.
4. The Slice 110 binding-closure review assigned the PyO3 native item and
   registration inventory, installed artifact joins, logging-subscriber
   disposition, macro/class identity, FFI/GIL/lifetime witnesses, and scanner
   retargets to Slice 100. These are retained. The NAPI subscriber and executor
   discrepancy stay with Slice 110.
5. The prework test approach assigns an actual installed PyO3 artifact,
   registration/signature/error/ABI comparison, deterministic blocking-thread
   tests, and stub-citation repair. Slice 30 owns the immutable surface
   baseline and comparator; Slice 90 may add only reviewed configuration
   deltas. Slice 150 owns exact final release qualification.
6. The current source has one `#[pymodule(gil_used = true)]` initializer,
   one `PyEngine` `#[pymethods]` block, `abi3-py310`, and explicit root
   registrations. `ExpandedNode` and `SearchExpandResult` omit
   `module = ...`; their installed runtime identity must be measured. The
   native subscriber still discards both
   arguments despite the delivery contract in the Python interface and
   bindings design. `test_surface.py` currently proves only that the method
   accepts a call, so it cannot close that discrepancy.
7. Source readers at entry include `dev/tools/surface_comparator.py`,
   `scripts/tests/test_windows_wal_attribution_ci_job.sh`,
   `scripts/tests/test_slice50_hook_inventory.py`, and
   `scripts/tests/test_slice70_embedding_docs_contract.sh`. The stub,
   `src/python/fathomdb/types.py`, and
   `src/python/tests/test_runtime_event_shape.py` also cite native source or
   line numbers and need accurate citations after movement.
   The full reader search must be repeated before each relevant move.

**Decision:** retain the draft's owner map and R27-100A–E provisionally,
subject to entry re-inventory and independent design approval. Keep
the subscriber correction explicit and separate from mechanical extraction.
Narrow qualification to the affected native feature/build routes and installed
consumer contracts at the exact Slice 100 candidate; leave the broad final
platform matrix to Slice 150. A supported route changed by this slice must
still pass here, and missing required evidence cannot be described as a pass.
The design records this proportional distinction. No new slice is needed.

## Assigned native families and final owners

| Entry family | Final private owner | Boundary |
| --- | --- | --- |
| Exception declarations and engine/open/domain error mapping | `errors` | Preserve class hierarchy, structured attributes and panic distinction. |
| String extraction, shared validation and detached execution | `ffi` | Keep validation order and Python attachment boundaries. |
| `PyEngine`, open/report/control, Engine methods and lifetime | `engine` | One class and one pymethods block; consume Slice 90 configuration. |
| Identity, view, context, device and open-report carriers | `types` | Only genuinely shared carriers. |
| Write, provenance, actuation, ingest and consolidation | `write` | No transaction or validation-order change. |
| Read/page/frozen/search and filters | `read_search` | Preserve precedence and frozen authority. |
| Graph traversal, expansion and evidence | `graph_evidence` | Engine-owned canonical codecs stay in the engine. |
| Projection configuration, registry, status and readiness | `projection` | Keep engine configuration separate. |
| Standalone rerank and CLS embed | `embedding` | Preserve feature refusal and singleton behavior. |
| Runtime/admin, lifecycle, erasure, dependency and logging adapter | `admin` | Subscriber contract corrected in its own RED/GREEN batch. |
| Gated native seams and rendezvous carriers | `test_support` | Preserve exact feature gates. |
| Initializer and explicit registration statements | root `lib.rs` | One module initializer and complete comparator source. |

Before extraction, generate a **named** entry ledger from every native item,
attribute, cfg, registration, alias, stub and package export. Assign each item
to exactly one owner above or record an individually justified root/facade
exception. Reconcile final versus entry plus reviewed deltas; family rows are
not a substitute for that ledger. After Slice 90, also inventory its exact
configuration symbols and receipts. Avoid moving the single Engine pymethods
block across multiple impls merely for visual symmetry.

## Requirements and acceptance

`design.md` owns R27-100A–E/AC27-100A–E. Entry adds an operational gate to
them: Slice 90's complete release-state record, PASS runtime checkpoint,
final binding handoff and committed source must be ancestors of the Slice 100
branch. The accepted logging contract is currently unmet. Delivery and payload
authority is `dev/interfaces/python.md` and `dev/design/bindings.md`;
in-flight heartbeat authority is `dev/design/lifecycle.md`. The Python
interval default and validation must be specified in the interface during the
correction. Before extraction,
either deliver the smallest logging adapter that emits the documented payload
and in-flight heartbeat, or obtain an accepted successor contract and
implement/test it within Slice 100. A no-op method or proposed document cannot
pass AC27-100C. Current engine events have no operation identity and no
heartbeat emission, so the adapter design must identify observed operations,
cadence/default/validation, concurrency, terminal ordering and shutdown before
coding. Test callback ownership, reentrancy and failures under that design. No
invented Python buffer or callback entrypoint is required.

AC27-100A/B require exact named inventory and installed runtime comparison,
including class `__module__`, signatures/defaults, exception identity,
registrations, stub declarations, package exports, cfg and ABI. AC27-100C
requires focused real-database and installed-native witnesses for changed
entrypoints, GIL progress, detached ownership, close/failure, hostile input,
panic recovery and precedence. AC27-100D requires candidate-bound wheels and
installed checks for affected feature/build routes; platform-only routes
that cannot be exercised locally need a named CI receipt for the same source
candidate. AC27-100E requires source-reader guards, repository-required
verification, independent code review and independent read-only verification
of the final candidate, with zero open Slice 100 items before Slice 110.

## RED/GREEN implementation order

1. Re-run preflight and read the Slice 90 handoff. Freeze exact source, package,
   runtime and feature inventories, candidate SHA, affected route matrix,
   focused existing-test map and scanner readers. Record actual installed
   entry behavior, not merely Rust declarations.
2. Add only missing fixed characterization tests. Demonstrate RED using a
   plausible temporary mutation, restore production, and keep the test fixed.
   For the subscriber discrepancy, first add RED tests for real `LogRecord`
   event/profile/slow delivery, in-flight heartbeat, overlapping calls,
   terminal/shutdown order, second-attach replacement, a raising logger,
   reentrant logging, and callback lifetime. Validate `None`, zero,
   negative and overflow intervals under the reviewed contract. The tests
   must fail on the present no-op before implementing the correction and
   getting GREEN.
   Capture the corrected comparison point separately.
3. Move errors/FFI, shared carriers and the Engine facade in reviewable
   batches. Then move write, read/search, graph/evidence, projection,
   embedding and admin families. Preserve API attributes, one initializer,
   one Engine class/impl identity and validation order. After each batch,
   run formatting, feature typechecks, focused binding tests, affected
   surface rows and source-reader guards. Refactor only after GREEN.
4. Reconcile registration/stub/package/source ledgers; run exact-candidate
   installed wheels, affected feature routes and supported-platform build
   receipts. Run `./scripts/agent-verify.sh` and additional blast-radius
   checks required by the actual changes. Keep final broader release gates
   with Slice 150 unless a Slice 100 change affects them directly.
5. Obtain independent code review of the actual diff and read-only test
   verification of the final candidate. Resolve findings with tests fixed;
   rebind both reviews after any production correction. Record status and
   evidence, integrate into the release branch, verify from Git, then remove
   this temporary worktree and branch when safe.

## Current disposition

The first independent design review found a concrete subscriber/heartbeat
design gap and a qualification-scope mismatch. The prospective design now
records both, but its subscriber mechanism still needs final code-grounded
selection after Slice 90. Design approval and implementation are **blocked at
entry**.
No RED/GREEN, native artifact, code review or test-verifier claim is made by
this planning record. The first dependent action is completion of Slice 90
and its exact binding handoff, followed by source re-inventory and design
review at the actual Slice 100 entry candidate.
