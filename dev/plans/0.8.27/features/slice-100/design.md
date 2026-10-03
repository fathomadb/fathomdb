---
title: FathomDB 0.8.27 Slice 100 — PyO3 decomposition
status: IN_REVIEW
target_release: 0.8.27
---

# Slice 100 PyO3 decomposition

Slice 90 closed at `c149584bc`; this is the entry design for Slice 100. All obligations below close within Slice 100
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
| Root `lib.rs` | Crate attributes, module declarations, shared private Rust/PyO3 imports and the single `_fathomdb` initializer with its explicit class/function/exception/alias registrations. Keep registration statements here so the existing lib-only comparator remains complete. No second initializer or duplicate class registration. |
| `errors` | All exception declarations, exhaustive engine/open/runtime/device/graph error conversions, stable reason/code strings and structured exception attributes. Preserve exception inheritance, `__module__`, identity and panic distinction. |
| `ffi` | `call_engine`, FFI string/range primitives and common Rust-to-Python execution helpers. Preserve validation before dispatch, detached-region scope, panic translation and attachment boundaries. Domain-specific conversion stays with its domain. |
| `engine` | `PyEngine`, open/report/close/control methods and its one `#[pymethods]` implementation, including native method entrypoints. Preserve the Slice 90 config seam. Keep a single pymethods block under the existing feature set; do not enable `multiple-pymethods` solely for file aesthetics. Extract existing conversion/work helpers where cohesive, without copying registered wrappers or changing evaluation order. |
| `types` | Truly shared identity/view/context/device/open-report carriers and their existing conversion/getter implementations. Domain-only receipts/carriers move with their consumer domain; the exact inventory names each, so this is not a miscellaneous bucket. |
| `write` | Write input translation, provenance/actuation parsing, receipts, ingest/extractor and consolidation conversion/operations. Cohesive private submodules separate large families without changing validation/transaction order. |
| `read_search` | Native read/page/frozen/search functions, filters and result conversions, search/evidence entry conversion helpers. Graph/evidence-specific carriers and authority checks stay in the owner below. |
| `graph_evidence` | Graph expansion/traversal/boundary operations, graph/evidence request/result conversion and resolve helpers, associated receipts. Existing canonical JSON codecs remain engine-owned. |
| `projection` | Configuration/registry/status/readiness functions and projection carrier conversions. Engine configuration forwarding remains distinct from projection configuration. |
| `embedding` | Standalone rerank and CLS embedding entrypoints/helpers and singleton ownership. Preserve one singleton, empty-input behavior, error classes and build-feature refusal. |
| `admin` | Runtime/admin configuration entrypoints, lifecycle/erasure/dependency operations and their domain conversion helpers. |
| `logging_subscriber` | Bounded queue, worker, weak logger lifetime, replacement/close slot and callback reentry guard for the existing Python logging attachment. |
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

### Entry correction: bounded Python logging subscriber

This entry correction implements the useful existing subscriber method under
[`ADR-0.8.27-python-subscriber-delivery`](../../../../adr/ADR-0.8.27-python-subscriber-delivery.md),
the accepted successor to the unmet 0.6.0 Python heartbeat promise. The engine
emits structured events, profiles, slow statements and debug stress failures.
The native method currently drops its logger and interval. Subscriber delivery
is a separate behavior change before any mechanical extraction.

FathomDB is a local SQLite-backed database. Its writer, reader and projection
threads must never execute arbitrary Python logger code or wait for it to
finish. `Engine::subscribe` remains the collection seam. Its registry takes a
snapshot before invoking subscribers, but SQLite profile callbacks can run
while a connection or writer transaction is owned. The adapter `Subscriber`
methods copy owned payloads into a short-held bounded queue with `try_push`;
they never enter Python, block on a full queue, perform I/O or hold a registry
lock across host code. Capacity is 4096 records per attachment. Queue overflow
drops the new record and increments a saturating loss counter. When the
worker resumes, it reports the loss count through a warning LogRecord and
allows a queued record to progress before another loss warning. No exact
LogRecord count or response-cycle pairing is
promised under overload; the engine's typed event emission remains exact.

One delivery worker per attachment resolves a weak reference to the caller's
`logging.Logger` and invokes `logger.log(level, message,
extra={"fathomdb": payload})` using `Python::try_attach`. The caller retains
the logger while it wants records; the weak reference avoids a rooted
Engine/logger/handler cycle. The worker checks the weak reference on a bounded idle wake (at most
once per second) as well as before delivery. If the logger disappears or
Python is shutting down, it disables and detaches the adapter, then exits. INFO carries ordinary event
phases and profiles; WARNING carries Slow, Failed, slow statements, debug
stress failures and queue loss. `fathomdb` holds `phase`, `source`, `category`
and optional `code` for events; `profile_record={wall_clock_ms, step_count,
cache_delta}`, `slow_statement={statement, wall_clock_ms}` and
`stress_failure={thread_group_id, op_kind, last_error_chain,
projection_state}` preserve existing typed payload fields. The overload
record uses `dropped_records`. Logger filters and levels remain caller-owned.
Exceptions from `logger.log` are contained; they never change a database
operation's result, and later records are attempted. The worker holds no
engine, subscriber-slot or queue lock while calling Python.

The worker sets a thread-local callback guard around `logger.log`. Every
binding path that can enter database work through `call_engine` refuses
reentry on that thread before dispatch, including search, write and close.
Attachment also refuses reentry. This blocks a handler-initiated search whose
reader workers would otherwise create recursive profile callbacks. Direct
read-only value getters that do not enter SQLite may remain callable. The
error is a typed `InvalidArgumentError` and is contained by the outer logger
callback policy if the handler propagates it.

`PyEngine` owns one attachment slot. Validate `logger.log` callable and
weak-reference support before replacing an attachment. The successor method
signature is `attach_logging_subscriber(logger)`; remove the misleading
`heartbeat_interval_ms` argument from native, stub, SDK and interface in the
same behavior change. There is no synthetic heartbeat or binding-generated
public operation ID: neither can truthfully report SQLite/provider progress
from the present `Event` shape. A host may display elapsed time around its
own synchronous call. Future engine-owned operation progress needs its own
accepted design, not a timer attached to logger delivery.

The slot has Open/Closing/Closed states. Attachment and close linearize under
a short-held mutex; attach after Closing starts raises `ClosingError`.
Replacement disables the old adapter under the slot mutex before installing
the new one. `try_push` checks enabled and inserts under the same queue mutex
that `disable` uses to clear queued records; an already snapshotted callback
cannot enqueue after disable. A record already dequeued by the old worker may
finish delivery after the replacement point; queued records are cleared.
The old RAII handle is taken out and dropped after releasing the slot mutex.
The slot retains at most one retiring worker; a
further replacement while that worker remains alive
raises `OverloadedError` and leaves the current attachment intact. This
bounds repeated replacement to at most one retired and one active worker.
Close takes and disables the attachment before native close, releases the slot
mutex across native close, and preserves native close's result, including
`SchedulerError`. Repeated close does not resurrect the attachment. Drop
performs the same cleanup. Already snapshotted callbacks see the disabled
flag. Shutdown clears the queue and signals the worker; it never waits for a
Python logger callback on the SQLite/provider teardown path. A hung handler
may leave a daemon delivery worker until it returns; at most two workers may
remain alive per Engine even after close. Tests cover idle logger
collection, normal worker exit, replacement during an event, repeated
replacement with a blocked callback, concurrent attach/close, callback
failure, profile-enabled reentrant search, slow logger overflow and
child-process shutdown.

The common engine subscriber dispatch, particularly the `extern "C"` SQLite
profile trampoline, contains subscriber panics before they can unwind across
the C boundary. A panic in one Rust subscriber cannot abort the process or
turn a successful database operation into a failure. This is covered by a
real-SQLite panic subscriber test and leaves exact engine event emission
unchanged. No generalized callback framework, engine progress timer, new
public event field or SDK executor is introduced.

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

The existing subscriber no-op is a contract defect. The bounded adapter and
successor signature above close it within this slice; a no-op or a new deferral
does not. The successor must be accepted before Slice 100 closes. Tests first
fail on the no-op and then pass against an installed corrected wheel.

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
Slice 100 affected-route matrix from release/CI contracts at entry: abi3 floor,
default-embedder on/off, test-hooks, applicable Windows/macOS compilation,
and accelerator/reranker routes whose native code or cfg this slice changes.
Build and install fresh wheels for the consumer paths exercised here. Slice
150 owns the broader final-release platform matrix. Do not use incompatible
all-features builds or call an unavailable affected route a pass; required
missing evidence blocks closeout. Reuse prior receipts only when
candidate/source/artifact/feature identity genuinely matches.

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
| R27-100C | FFI execution and lifetime are safe. | AC27-100C: detached-blocking progress, ownership/close/failure, panic, hostile conversion and precedence witnesses pass against real installed native code; the subscriber no-op is replaced by the reviewed bounded delivery contract, with real SQLite callback and overload witnesses. |
| R27-100D | Artifact evidence is real. | AC27-100D: required candidate-bound wheel/platform/feature routes pass, imports resolve inside isolated installations, release hooks are absent and test hooks present, and configuration receipts remain effective. Source-only success or skipped required routes cannot close the slice. |
| R27-100E | Complete before NAPI decomposition. | AC27-100E: scanner and repository-required gates pass, independent code review and read-only verification bind the final candidate, and zero Slice 100 obligations remain before Slice 110. Python SDK decomposition alone remains assigned to 130. |
