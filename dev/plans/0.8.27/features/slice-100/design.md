---
title: FathomDB 0.8.27 Slice 100 — PyO3 decomposition
status: PLANNED
target_release: 0.8.27
---

# Slice 100 PyO3 decomposition

Prospective design only; uncommissioned. Entry requires completed Slice 90
and its configuration handoff. All obligations below close within Slice 100
before Slice 110. Python SDK decomposition remains Slice 130. Source-layout
symmetry with NAPI and line-count targets are not acceptance criteria.

## Existing substrate and authority

At reviewed candidate `63091b6249e5805ea71b44de59f0ac6703b786a9`,
`fathomdb-py/src/lib.rs` owns the native classes, exceptions, conversions,
operations, test seams and explicit `_fathomdb` registrations. Cargo selects
PyO3 0.29 with `abi3-py310`, not `multiple-pymethods`; extension-module linkage
is selected for wheels rather than ordinary workspace test binaries. The
initializer is `#[pymodule(gil_used = true)]`. Blocking calls use `py.detach`
with panic containment; Python remains synchronous. Many pyclasses explicitly
pin `module = "fathomdb._fathomdb"`; others, including expanded-result
carriers, omit it. Capture actual runtime module identities for both rather
than normalizing them. Preserve names, frozen/getter behavior and conversion
attributes. `PyEngine` owns engine/report Arcs; test-only `PyWalSnapshotPause`
owns rendezvous state. Preserve those contracts and exact feature gates.

The [bindings design](../../../../design/bindings.md),
[Python interface](../../../../interfaces/python.md),
[async ADR](../../../../adr/ADR-0.6.0-async-surface.md), and
[Slice 30 comparator design](../slice-30/design.md) are authority. Read their
current owners at entry. No dependency upgrade, free-threaded Python support,
new async Python facade, schema change or SDK redesign is authorized here.

## Complete ownership and registration map

Before production movement, derive an exact entry inventory from all crate
source files, Cargo cfg/features, initializer registrations and package/stub
declarations. Include every function, constant/static, type, exception macro,
impl/method, conversion, pyclass/pyfunction attribute, registration/alias,
test hook and test. Each symbol gets one final owner below or an individually
reviewed retained-root reason. Expand every family into named symbols and
record Python-visible name, signature/defaults, cfg, module identity and
registration site. Missing/duplicate/stale entries fail; no remainder bucket.

| Final private owner | Items and boundary |
| --- | --- |
| Root `lib.rs` | Crate attributes, module declarations, narrow internal imports and the single `_fathomdb` initializer with its explicit class/function/exception/alias registrations. Keep registration statements here so the existing lib-only comparator remains complete. No second initializer or duplicate class registration. |
| `errors` | All exception declarations, exhaustive engine/open/runtime/device/graph error conversions, stable reason/code strings and structured exception attributes. Preserve exception inheritance, `__module__`, identity and panic distinction. |
| `ffi` | `call_engine`, FFI string/range primitives and common Rust-to-Python execution helpers. Preserve validation before dispatch, detached-region scope, panic translation and attachment boundaries. Domain-specific conversion stays with its domain. |
| `engine` | `PyEngine`, open/report/close/control methods and its one `#[pymethods]` implementation, including native method entrypoints. Preserve the Slice 90 config seam. Keep a single pymethods block under the existing feature set; do not enable `multiple-pymethods` solely for file aesthetics. Extract existing conversion/work helpers where cohesive, without copying registered wrappers or changing evaluation order. |
| `types` | Truly shared identity/view/context/device/open-report carriers and their existing conversion/getter implementations. Domain-only receipts/carriers move with their consumer domain; the exact inventory names each, so this is not a miscellaneous bucket. |
| `write` | Write input translation, provenance/actuation parsing, receipts, ingest/extractor and consolidation conversion/operations. Cohesive private submodules separate large families without changing validation/transaction order. |
| `read_search` | Native read/page/frozen/search functions, filters and result conversions, search/evidence entry conversion helpers. Graph/evidence-specific carriers and authority checks stay in the owner below. |
| `graph_evidence` | Graph expansion/traversal/boundary operations, graph/evidence request/result conversion and resolve helpers, associated receipts. Existing canonical JSON codecs remain engine-owned. |
| `projection` | Configuration/registry/status/readiness functions and projection carrier conversions. Engine configuration forwarding remains distinct from projection configuration. |
| `embedding` | Standalone rerank and CLS embedding entrypoints/helpers and singleton ownership. Preserve one singleton, empty-input behavior, error classes and build-feature refusal. |
| `admin` | Runtime/admin configuration entrypoints, lifecycle/erasure/dependency operations and their domain conversion helpers, logging-subscriber adapter and controls. Split into cohesive private submodules when needed; every item remains named in the inventory. |
| `test_support` | Already gated native test functions and rendezvous carriers; Engine-registered test methods remain in its pymethods block. Preserve cfg and qualified tests; no production leak or gate broadening. |

No new Python-visible helper or Rust-public seam is introduced for extraction.
Use narrow internal accessors or explicit arguments where Rust privacy needs
a seam, without registering those accessors. If a macro-generated operation
cannot move safely, its exact item remains in the appropriate facade with a
reviewed reason; its implementation is not duplicated elsewhere.

Final inventory must equal entry plus individually approved deltas. The
inventory is a structural audit, not a test asserting arbitrary filenames.
Existing Python package modules, `__all__`, `_coinstall` import ordering,
`_fathomdb.pyi`, `py.typed`, native loader identity and wheel metadata remain
accounted for. Stub edits are path/documentation-only unless a separately
approved contract correction requires a specific signature delta.

## Contract characterization and corrections

Establish exact entry results before moves. New characterization tests must
fail under a plausible temporary mutation and pass after exact restoration.
Use existing FFI safety, native-stub/surface/parity, frozen-precedence, WAL and
real-database routes; add only missing witnesses. Keep assertions fixed while
moving code. A behavior change gets a separate reviewed genuine RED→GREEN
correction before extraction, with applicable ADR/interface updates; it is
never hidden in a mechanical diff.

Trace every blocking entrypoint, including open, close, standalone embedding
and reranking. Controlled rendezvous must prove another Python thread makes
progress while native work blocks. Capture Rust-owned data before detach;
no borrowed Python object is accessed without attachment. Prove engine/report
and pause-handle lifetime across concurrent work, repeated close and failure,
without deadlock, orphaned threads or use-after-free. Preserve the current GIL
mode and Python caller-owned executor model.

Test typed error class identity and attributes, panic as `pyo3_runtime`'s
`PanicException` rather than `EngineError`, and healthy subsequent work in a
fresh child. Cover invalid NUL/surrogates, nested inputs, bool-vs-integer,
negative/fractional/overflow values, missing/None/wrong-type inputs, finite
float policy and buffer/sequence ownership wherever the actual entry inventory
accepts them. Do not invent a new buffer or callback API. Exercise validation
precedence (including frozen authentication before dynamic controls) and prove
invalid writes leave the real database unchanged.

At this baseline `attach_logging_subscriber` accepts and discards its logger
and heartbeat arguments. Do not claim callback delivery/lifetime coverage
from that no-op. Before its batch, trace the current accepted observability
contract and record an explicit disposition: prove an already accepted
non-delivering contract, or complete a separately reviewed fix-to-contract or
accepted successor and its implementation/tests within Slice 100. An old
"later slice" comment, new deferral, or merely proposed ADR cannot close this
entry. Any actual callback implementation needs ownership/reentrancy, logging
failure, detach/reattach and shutdown-lifetime tests. This is a scoped contract
reconciliation, not authorization for a general subscription redesign.

## Artifacts and surface oracles

Use Slice 30's immutable baseline plus only the approved Slice 90 deltas, and
capture the exact Slice 100 entry contract separately. Compare package exports,
resolved wrappers, all native registrations with cfg, stub declarations and
installed runtime identities/signatures independently. A source registration
row alone does not prove the class initialized correctly. Test default/release
hook absence and test-feature presence. Do not regenerate a baseline to erase
a difference. Any additional public behavior/surface change requires named
reviewed authority and tests before implementation.

Use `scripts/verify-release-python-wheel.sh --python PYTHON --wheel-dir DIR
--venv-dir DIR` with new owned paths for the standard installed-wheel route.
It uses maturin release build, then a fresh venv and `pip install --no-index
--no-deps` of that wheel. Run the owning binding suites and consumer import,
open/write/read/close/reopen/error/config tests against that installed artifact
from outside the checkout, with PYTHONPATH removed and import/native paths
proved inside the venv. Package tests explicitly so repository pytest path
settings cannot shadow the installed package. A wheel smoke alone is not the
full exit matrix. Build separate test-hooks and applicable accelerator wheels
with the exact recorded feature lists; never call `maturin develop` or install
editable from the release worktree. Do not copy an ad-hoc `.so` into a package.

Record candidate SHA, Rust/Python/maturin versions, target, features, wheel
hash, native module provenance, commands/exits and test counts. Freeze the
supported platform matrix from release/CI contracts at entry: abi3 floor and
supported interpreter endpoints, Linux plus applicable Windows/macOS builds,
default-embedder on/off, test-hooks, and supported CUDA/Metal/reranker routes
separately. Do not use incompatible all-features builds or call an unavailable
route a pass; required missing evidence blocks closeout. Reuse prior receipts
only when candidate/source/artifact/feature identity genuinely matches.

## Batches and exit

1. Consume Slice 90 receipts; approve symbol/registration/contract inventory,
   platform matrix and focused test map. Reconcile the subscriber entry.
2. Characterize missing FFI/GIL/lifetime/precedence routes and fix any approved
   contract discrepancy separately. Capture the corrected comparison point.
3. Move errors/FFI, then shared carriers, then engine facade, one cluster per
   batch. Preserve the explicit root initializer and one Engine identity.
4. Move write families, read/search, graph/evidence, projection, embedding and
   admin families as separate batches; retain macro-required facade methods.
5. Reconcile stubs, package/registration and source-scraper inventories; run
   final artifacts/matrix, independent code review and independent read-only
   verification at the exact candidate. Close every requirement before 110.

Target 300–600 non-mechanical changed lines; split above 800 absent a reviewed
cohesion reason. Mechanical batches normally move 300–1,200 lines and one
owner; no artificial split of a macro-cohesive block. Per batch run formatting,
lint, feature typechecks, focused real binding tests, affected surface rows
and source scrapers. Enumerate lib-file readers before moving: the surface
comparator, slice50 hook inventory, Windows WAL guard and embedding-doc gate
are known examples. Retargeted scanners require RED mutants retaining the same
oracle. Run repository-required verification at closeout; record cleanup of
only owned scratch/artifact paths without disturbing shared environments.

| ID | Requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-100A | Complete native ownership. | AC27-100A: source-derived entry/final item and registration inventories reconcile exactly; every item has one owner or named facade/root exception; no duplicate Engine, registration or unresolved remainder. |
| R27-100B | Python contracts remain exact. | AC27-100B: immutable-plus-approved-delta and entry comparisons pass for wrapper/native/stub/runtime/package surfaces, exception identity, cfg and imports; module/GIL/ABI identities and singleton counts are unchanged. |
| R27-100C | FFI execution and lifetime are safe. | AC27-100C: detached-blocking progress, ownership/close/failure, panic, hostile conversion and precedence witnesses pass against real installed native code; named contract discrepancies including the subscriber entry are closed with accepted authority and tests. |
| R27-100D | Artifact evidence is real. | AC27-100D: required candidate-bound wheel/platform/feature routes pass, imports resolve inside isolated installations, release hooks are absent and test hooks present, and configuration receipts remain effective. Source-only success or skipped required routes cannot close the slice. |
| R27-100E | Complete before NAPI decomposition. | AC27-100E: scanner and repository-required gates pass, independent code review and read-only verification bind the final candidate, and zero Slice 100 obligations remain before Slice 110. Python SDK decomposition alone remains assigned to 130. |
