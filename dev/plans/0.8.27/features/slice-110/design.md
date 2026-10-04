---
title: FathomDB 0.8.27 Slice 110 — NAPI decomposition
status: IN_PROGRESS
target_release: 0.8.27
---

# Slice 110 NAPI decomposition

Commissioned at clean `release/0.8.27` entry `9340a824` after completed
Slice 103, the Slice 100 binding handoff, and the preserved Slice 90
configuration handoff. The [entry reconciliation](plan.md) records changes
since this design's historical review baseline. All native Node obligations
close here before Slice 120. No Slice
111 is allocated: sequential contract,
move and artifact batches provide the necessary technical boundaries.

## Substrate, authority and scope

Historical reviewed baseline: `63091b6249e5805ea71b44de59f0ac6703b786a9`.
Actual execution baseline: `9340a8246e39127c255b55a3f97b674553eb6908`.
`fathomdb-napi/src/lib.rs` contains native objects, conversions, errors,
functions and the Engine class. Cargo uses napi-rs 2, N-API 8, async/Tokio and
serde-json. `call_engine` uses Tokio `spawn_blocking` plus `catch_unwind`;
`call_engine_sync` separately contains accessor panics. Engine/report Arcs are
cloned into native work. Native errors carry JSON envelopes consumed by the
existing TypeScript error layer. Inputs mix typed NAPI objects and JSON; some
integers use numbers and others canonical decimal strings. Preserve each
actual field's representation rather than normalizing all numbers to BigInt.

Authority: [bindings design](../../../../design/bindings.md),
[TypeScript interface](../../../../interfaces/typescript.md),
[async ADR](../../../../adr/ADR-0.6.0-async-surface.md),
[TS API-shape ADR](../../../../adr/ADR-0.6.0-typescript-api-shape.md),
[Slice 30 comparator](../slice-30/design.md), and current release native
artifact/platform contracts. Do not upgrade napi-rs, introduce a new SDK API,
or decompose `src/ts/src/index.ts` here. Necessary native declaration, loader,
packaging and thin-wrapper corrections are scoped and tested here; TypeScript
domain decomposition remains Slice 120.

## Ownership and exact inventory

At entry derive the complete source inventory across every crate module,
attribute-generated export, object field, impl/method, free function, error
constant, conversion, static/singleton, test hook and test. Join it with fresh
generated declarations and loaded native runtime exports/class prototypes.
Record JS names/casing, defaults/optionality, sync/async shape, cfg, registration
identity and numeric/string/buffer representation. Map every entry to one owner
below or a specifically justified retained facade/root item. Missing,
duplicate and stale entries fail; all family rows expand to actual symbols
before movement. Preserve one Engine class and one export per existing name.

| Final private owner | Items and boundary |
| --- | --- |
| Root `lib.rs` | Crate attributes, module declarations and the exact imports/re-exports needed for napi-rs registration and shared Rust visibility. Attribute-generated exports are reconciled against runtime/declarations, not assumed to follow a moved source file. |
| `errors` | `typed_error`, error-code constants, exhaustive engine/open/runtime/device/graph mappings, panic envelope and stable structured payload helpers. No duplicate error class/envelope implementation in Rust or TypeScript. |
| `shared` | Narrowly shared translation helpers consumed by multiple domain modules. Internal reuse belongs here rather than duplicating validation or making a second public facade. |
| `execution` | `call_engine`, `call_engine_sync`, blocking handoff/join and panic policy, and any approved callback/executor implementation from the disposition below. Engine-owned configuration and host handoff scheduling remain distinct. |
| `subscriber` | Rust-owned diagnostic queue, N-API wakeup, callback trampoline, attachment replacement and teardown. This host callback route is separate from ordinary engine-call execution. |
| `engine` | Engine Arc/report ownership, its single native class implementation, open/close/report/control and method entrypoints. Preserve macro-cohesive impl blocks where required by the current napi-rs build; no speculative macro/dependency change just to scatter methods. |
| `types` | Shared identity/view/context/device/open-report native objects and their conversions; domain-only objects belong with their domain and are individually named in the inventory. |
| `write` | Write/provenance/actuation translation, receipts, ingest and consolidation operations and input conversions. Split cohesive families into private submodules without changing validation order. |
| `read_search` | Read/page/frozen/search/filter conversion, functions and result objects; preserve authentication-before-conversion ordering. |
| `graph_evidence` | Graph/traversal/boundary/evidence operations, request translation, result/sidecar conversion and resolve helpers. Canonical codecs remain engine-owned. |
| `projection` | Projection config/registry/status/readiness objects, conversions and entrypoints. Keep runtime EngineConfig separate from projection specifications. |
| `embedding` | Standalone rerank/CLS embedding entrypoints, device helpers and singletons, with existing feature refusal and empty-input behavior. |
| `admin` | Runtime/admin configuration, lifecycle/erasure/dependency entrypoints and controls. Domain families have cohesive private modules and exact item entries; the Engine method for attachment remains with the single Engine class. |
| `test_support` | Existing cfg-gated panic and diagnostic hooks/helpers. Engine-registered hooks stay attached to that class. Exact gate/test identity is preserved; production exports must not gain hooks. |

The TypeScript `binding.ts`, `platform.ts`, `errors.ts`, generated native
`index.d.ts`, compiler-emitted `dist` declarations, package metadata and
platform packages are separately inventoried consumers/surfaces. They remain
in place except required scoped path/type/build corrections. Shared vocabulary
does not require matching Python file trees. Narrow internal accessors can
preserve Rust privacy; they must not become new JS exports. No copied wrapper
or second native entrypoint is used to bridge a move.

## Pre-move contract disposition

The accepted
[`ADR-0.8.27-engine-owned-runtime-topology`](../../../../adr/ADR-0.8.27-engine-owned-runtime-topology.md)
expressly replaces the earlier async ADR's exact ThreadsafeFunction and
binding-pool mechanisms. The current TypeScript interface and binding design
name NAPI's Tokio `spawn_blocking` handoff. Record that authority and prove
off-event-loop, error, ownership and close behavior; do not rebuild the old
pool merely to match superseded wording. Separately, `attach_subscriber`
currently discards callback/options and returns success. A callable method
alone does not prove event delivery. The accepted Python subscriber successor
explicitly leaves TypeScript's contract untouched, so review the TypeScript
callback and heartbeat contract on its own terms before changing it.

The accepted [TypeScript subscriber successor](../../../../adr/ADR-0.8.27-typescript-subscriber-delivery.md)
selects one replaceable attachment per Engine. An owned, nonblocking 4096-record
queue carries diagnostics; a coalesced raw N-API wakeup carries a null data
pointer and drains at most 64 records per JS turn. This avoids the pinned
napi-rs helper's rejected-payload leak and contains JavaScript exceptions and
Rust panics through a C callback trampoline. It reports
cumulative drops, permits lock-free reentry and removes the untruthful binding
heartbeat option. Keep its public signature/payload delta separate from
mechanical module movement.

The queue/wakeup state transition, TSFN-handle mutex protocol and JS-facing
synchronous close-entry ordering are normative in that successor. RED tests
must include a producer/empty-drain race and callback-triggered replacement
and close, not only serial delivery.

### As-built ownership and rationale

The implementation at `87670f61d48e6552edcb2a64f7ddb2dacf2863e1`
reconciles all 465 frozen source items. The private `shared` module holds
helpers needed across domain modules, and `subscriber` is its own module:
callback delivery has a different thread, lock and teardown contract from
ordinary `execution::call_engine`. Keeping it out of `execution` prevents the
Promise handoff from becoming a general callback executor. The single
`Engine` class and its methods remain together in `engine.rs`; domain modules
own related objects, conversions and free entrypoints. `lib.rs` composes
and re-exports those modules so napi-rs's attribute-generated registration
still produces one native identity. A Rust `pub use` in that root is an
internal registration/visibility tool, not an authorization for a new
JavaScript API. The production artifact preserves 17 runtime exports and 44
Engine prototype names; only the separately accepted subscriber signature
changes its generated declaration.

Blocking engine calls continue through Tokio `spawn_blocking` with an Arc
retained until completion. This keeps SQLite work off the event loop without
adding a second capacity control beside the engine-owned scheduler and embed
dispatcher. In contrast, a subscriber may receive an event from SQLite's C
profile callback while a connection is held. The Rust-owned bounded queue
therefore does not execute JavaScript or wait on that producer thread; a raw,
data-free N-API wakeup moves delivery to the JavaScript thread. Coalescing
and the 64-record drain cap bound wakeups and each event-loop turn. A
generation gate, callback-fault containment and synchronous detachment at
close entry protect replacement, reentry and shutdown. The accepted
[subscriber ADR](../../../../adr/ADR-0.8.27-typescript-subscriber-delivery.md)
owns exact failure and payload semantics. Its removed heartbeat option is a
contract correction: the current engine cannot supply operation-scoped
progress, so the old binding option could only create a false signal.

The checked native build wrapper now launches its installed local napi-rs CLI
through Node on Windows. Directly spawning `npm.cmd` under the pinned Node
25.9.0 runtime failed with `EINVAL`; using the local CLI preserves the locked
tool version and avoids shell quoting or command lookup differences. The
Linux/macOS path retains the existing npm invocation. This correction is in
the shipping path, not an SDK surface change.

Before moving those owners, inspect current accepted successors and record
which contract governs each. A finding closes only with evidence that the
current behavior is already permitted, an accepted successor implemented and
tested, or a separately reviewed genuine RED→GREEN fix-to-contract completed
within Slice 110. No blanket grandfathering by baseline, unaccepted proposal,
"later slice" comment or deferral to Slice 120 is a valid disposition. Keep
each correction separate from mechanical movement, with named interface/API
deltas if needed. Do not confuse the TS handoff pool with Slice 90's engine
scheduler/embedder knobs. Do not invent a general callback system outside the
accepted subscriber obligation.

## Falsifiable behavior and lifetime proof

Use the actual native artifact and thin public SDK for complementary tests:
raw native envelopes and NAPI argument conversion are not proved by wrapper
mocks. Characterize source entry behavior and accepted contracts, retain the
existing FFI safety, surface, frozen-read/graph/evidence and numeric-property
tests, and add only missing witnesses. Prove new characterization with a
temporary plausible mutant and exact restoration. Any behavioral correction
requires genuine RED tests before code changes.

- Inventory sync accessors versus Promise operations and test exact immediate
  throws versus rejections, error code/message/data and wrapper error identity.
  Cover native conversion failure before dispatch, engine refusal, worker
  panic, sync-accessor panic, join failure where injectable, and subsequent
  healthy work without aborting the host.
- Controlled native rendezvous plus independent event-loop work proves
  responsiveness while engine work is blocked; a timer delay alone is not the
  oracle. Trace the actual executor and verify its governing contract.
- Exercise close with in-flight work, rejected work and repeated close; retain
  Engine/report ownership until completion. Do not treat abandonment of a
  Promise as cancellation unless the contract says so. Prove shutdown and
  process exit do not leak work/handles. Where callbacks actually exist,
  prove JS-thread invocation, bounded lifetime, error/reentrancy policy,
  unsubscribe/close behavior and no use of JS handles off their legal thread.
- Derive conversion cases from each native signature: number safe-integer
  boundaries, signed/unsigned limits, fractional/NaN/infinite values, bigint
  rejection or acceptance as specified, canonical decimal-string u64 limits,
  bool/null/undefined/wrong object types, casing aliases and nested hostile
  NUL/lone-surrogate strings. Preserve valid Unicode bytes and exact field-path
  diagnostics. For any accepted buffer/typed-array path, prove copy/borrow
  lifetime through async completion; absence is recorded, not a new buffer API.
- Prove rejected write/actuation input causes no real database mutation and
  preserves authentication/validation precedence. Retain human-defined
  conversion/round-trip property oracles without regenerated golden output.

## Production generation and installed-package evidence

Compare the immutable Slice 30 baseline plus reviewed Slice 90/100 deltas and
the exact entry capture. Native runtime exports/prototypes, production NAPI
declarations, TypeScript compiler declarations, package exports and installed
behavior are separate oracles. Any necessary contract correction requires a
named reviewed delta; mechanical moves must compare equal. Do not refresh a
baseline from a failing candidate.

Run the canonical `npm run build:native` in `src/ts`: its checked Node wrapper
cleans the release NAPI crate and uses isolated type-generation temporary
directories. Use the locked Node/npm/napi toolchain (the current contract pins
Node 25.9.0) and recorded Cargo feature closure. Run a test-hooks debug build
followed by the production command and prove both panic hooks disappear from
declarations **and loaded runtime exports**. Generate TypeScript declarations
with the production build configuration into a clean owned output tree;
test-build `dist/src` or stale declarations are not production evidence.

Build and stage the thin main package and matching `src/ts/npm/<triple>`
platform package with the existing release recipe, including platform naming,
os/cpu/libc, version matching, `.node` placement and optional-dependency
injection. Use `npm pack` for both staged packages without publishing or
editing source manifests for local installation. Install both resulting local
tarballs in a fresh external consumer (`npm install` with the explicit tarball
paths); verify resolved native paths and hashes belong to that installation,
not a repository fallback or registry replacement. Reuse the candidate-native
receipt tooling. Test package-root import, type compilation, runtime exports,
open/config/write/read/close/reopen and typed errors. A direct `.node` load is
additional raw-native evidence, not a replacement for the packaged loader.
Check packed file lists, licenses, absence of test output and dev hooks, and
unsupported-platform refusal. No source-linked package counts as acceptance.

Freeze supported platform/feature rows from CI/release contracts at entry:
production default-embedder, no-default-embedder refusal, test-hooks,
applicable CUDA/reranker routes, supported Linux targets and Windows/macOS
routes. Do not invent NAPI Metal support absent from its Cargo features.
Record candidate, target/libc, exact features/toolchain, binary/tarball hashes,
commands/exits and test counts. Required unavailable routes block closure;
prior artifacts are reusable only with matching candidate/source/provenance.

## Batches and exit

1. Consume prior handoffs; derive and approve exact source/export/call-shape
   inventories, platform matrix and executor/subscriber dispositions.
2. Add missing characterization and independently reviewed contract fixes;
   capture the corrected entry point before structural work.
3. Move error/execution owners, shared objects, then Engine facade, preserving
   one native registration identity and cfg attributes.
4. Move write, read/search, graph/evidence, projection, embedding and admin
   families separately; validate generated and loaded surfaces per family.
5. Reconcile all declarations/loader/package/scanner entries and run clean
   generation, packed-consumer matrix and exact-candidate independent code
   review plus independent read-only verification. Close every obligation.

Target 300–600 non-mechanical lines; split above 800 unless a reviewed cohesion
reason is recorded. Mechanical batches normally move 300–1,200 lines and one
owner; macro cohesion outranks a line target. Per batch run formatting, lint,
feature typechecks, owning native/SDK tests, affected declaration/runtime
surface rows and source readers. Inventory source-scraping consumers before
moves, including error-mapping/panic/removal and feature-leak checks. Parser
retargets require RED negative fixtures, unchanged assertions and no empty
scan pass. Run repository-required verification at closeout and remove only
owned temporary consumers/build files after recording receipts.

| ID | Requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-110A | Complete semantic native ownership. | AC27-110A: entry/final source and export inventories reconcile with zero missing, duplicate, stale or unassigned items; each retained facade/root item has its specific macro/ownership reason. |
| R27-110B | Native and package surfaces are preserved. | AC27-110B: runtime exports/prototypes, production NAPI and emitted TS declarations, package entrypoints and error envelopes equal the approved contract; debug-to-production generation removes hooks in both runtime and declaration oracles. |
| R27-110C | Async and callback contracts are resolved and proven. | AC27-110C: the executor's accepted authority is recorded and its outcomes proven; the subscriber discrepancy has an accepted, implemented disposition; exact sync/Promise/error/panic, event-loop progress, in-flight ownership and cleanup tests pass on native artifacts. No callback or executor obligation remains for 120. |
| R27-110D | Conversions and precedence remain exact. | AC27-110D: signature-derived numeric/string/object/buffer cases and existing properties pass, with exact field diagnostics and no database mutation on refusal. No convenience coercion or number representation change is hidden in extraction. |
| R27-110E | Shipping-path evidence is complete. | AC27-110E: supported candidate-bound feature/platform builds and fresh thin-main/platform-pair installs pass; binary provenance, package contents, runtime imports and consumer typechecks are verified. Required skipped routes cannot close the slice. |
| R27-110F | Native closure unblocks SDK decomposition. | AC27-110F: source-scraper/repository gates, exact-candidate code review and independent verification pass; inventory has zero open Slice 110 entries. Slice 120 receives the final native signatures, registration/runtime/declaration captures and packaged artifact receipts, with no native work deferred. |

Slice 110 remains incomplete until every acceptance is met. A new ladder
slice requires a concrete reviewed technical dependency; size, ceremony or
desire to label partial work complete does not justify Slice 111.
