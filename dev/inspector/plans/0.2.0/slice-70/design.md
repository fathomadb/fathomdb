---
title: Inspector 0.2.0 Slice 70 — installed qualification design
date: 2026-11-19
product: FathomDB Inspector
target_release: 0.2.0
status: ACTIVE
desc: Product boundary, aggregate runtime and real-user qualification
---

# Slice 70 — installed qualification design

Execution design at baseline `5627290e72e232645efdf5ac30714475fe37d275`.
The [plan reconciliation](plan.md#execution-reconciliation--2026-11-20) supersedes
historical draft assumptions.20–60 supply complete feature contracts;70 adds
package/install/resource/human qualification. A fresh direct Sol-high review
precedes design implementation.

## Selected implementation changes

- Build Inspector0.2.0 only, preserving Engine6edc1155f/format37 and all external
  versions/pins. Package explicitly selects cached-cpu and excludes fixture-control.
  Help and build description expose its own version; artifact/README agree.
- Task-local Ubuntu22.04/glibc2.35 compiler/runtime image, existing host7.0 kernel,
  exact Chromium156.0.8078.4. Use a separate container-owned compilation target;
  reuse only locked source caches, never host-compiled objects. Inspect final ELF
  interpreter/versioned symbols and execute under that userland. Verify LandlockABI>=3 before positive cached serving.
  Record exact userland/kernel/browser/compiler pair; no stock5.15 compatibility
  claim. Local package setup does not modify host kernel, drivers or global tools.
- Installed harness starts the extracted production executable. An owned test
  browser launcher may enable CDP while preserving actual graphical Chromium,
  private bootstrap/profile and same-executable worker ownership. CDP drives UI;
  actual native window close must end production supervisor, release lock and
  remove private state. A debug driver or SIGTERM cannot satisfy that result.
  Test controls never enter the package. Runtime has no Node/Python/server need;
  measurement tools may use Node outside the product.
- Existing real-DB fixtures and browser/security/resource probes are reused with
  explicit installed target paths only where supported. Extend minimal target
  selection where required; never pair a compiled development supervisor with a
  different worker. Preserve own query/read context, all8D01 and exact values.
- F06 aggregate measurement includes service/worker/model RSS and browser process
  tree RSS; canonical browser heap growth<=512MiB remains separately checked.
  Record sampling interval and observed peak only. Queue/cancel/termination tests
  signal only exact children created by the recorded test session.
- Freeze separate authored familiarization/unseen F07 ordinary-write fixtures,
  independent intended key, T01–05/High task cards and no-teaching assistance rules.
  Five actual eligible participants remain required; prepare runnable study pack
  now and retain honest unperformed outcomes until actual sessions.

## Actual source and remaining work

- [Manifest](../../../../../src/inspector/Cargo.toml) is unpublished0.2.0 with
  empty defaults and Engine defaults disabled; fixture-control is explicit.
  [Build description](../../../../../src/inspector/build.rs) hashes selected
  sources/lock/compiler/flags, records schema/model/assets and fixture-control.
  It includes Inspector version and hashes compiled CPU/fixture capability.
  Runtime qualification still uses the actual artifact, not its metadata alone.
- [Package script](../../../../../src/inspector/tools/package.sh) builds locked
  offline release/default-disabled Linux x86_64, stages binary/license/readme/
  description/checksums with0.2.0 names. Existing
  [package test](../../../../../src/inspector/tests/browser/package.mjs) checks
  extraction/help/description/default skeleton, not installed all-view operation.
  Net new: consistent Inspector-only 0.2 identity, compatible build and integrated
  explicit-CPU artifact qualification; no database package bump or publication.
- [HTTP](../../../../../src/inspector/src/http.rs), process/session/worker and
  policy/storage code provide bounded parsing, local pairing, cleared child
  environments, typed observations and real per-transaction queued membership.
  The working worker now supports all20–60 views; their status records qualify
  the four development native/resource cases and owned environment. Installed
  Ubuntu22 compatibility and actual native window-close remain70 work.

## Selected product and installed flow

One Ubuntu 22.04-compatible offline Linux x86_64 tar bundles a Rust service/UI
with embedded static assets, selected unchanged 0.8.29 Engine/current format 37,
and exact Chromium pairing. Node/Playwright/Python are test tools only; runtime
needs no developer server, model service or network. Query assets already exist
in a compatible CPU cache. No pin upgrade, GPU policy/driver change or broad OS claim.

Extract → inspect offline build/help → native launch/private browser bootstrap →
explicit Engine-effect disclosure/attach → records/text/vector/graph/provenance →
two actual pinned observations → return/cancel/detach/exit. All production reads
and UI assets belong to that same artifact; test-controls cannot enter its closure.
Default artifact verification must exercise cache-backed semantic/hybrid positives,
not infer them from a fixture build or metadata field. Production Default/None
admission and contained test Caller follow the accepted
[vector policy](../../../../adr/ADR-0.8.28-vector-compatibility-open-policy.md) and
[Rust contract](../../../../interfaces/rust.md); the old helper→Caller design is
not a supported route. Slice 40 supplies a source-qualified public route or a blocker.

## Integration contracts and independent evidence

Keep existing A/S/Q/P/O/R/U/K contracts. Each request retains exact typed identity,
revision and read context; later hydration/checks never imply the same snapshot.
Stored L2 population/distance, lexical scores and serving/hybrid values stay distinct.
All eight D-01 combinations apply to every delivery, membership, reference/copy/
export; legitimate ExplicitMaintenance queued scope remains physically retained
without implying logical removal. Autonomous absence never reconstructs. Ordinary
Engine lifecycle effects are allowed and independently attributed; preflight
refusals preserve files and initial nonempty WAL remains unsupported.

Budget/cancel/close measures span supervisor, native worker/helper and browser,
including pinned observations, graph expansion and queued work, using unchanged
[operating limits](../../../requirements.md#verification-and-operating-envelope-slice-15-2026-10-30).
Acknowledgement does not mean native completion; retain the execution slot until
quiescence or bounded owned-child termination/reap. Measure peaks/residency and
actual lock release; sampling cannot claim an unobserved instantaneous peak.
Repeat hostile-parent/header/candidate/cache/network/log and origin/Host/build/
credential checks installed. Property/boundary, adjacency/lexical/L2 and intended
identity oracles are independent of UI output; no database mocks/generated goldens.
Keyboard interactions expose equivalent labeled facts on every required view.
Five real unseen-task participants use ordinary docs, a prewritten answer key
and fixed assistance rubric; separate High provenance/outcome investigations
remain required. Four-of-five is diagnostic; missing sessions block qualification.

## Genuine challenges, research questions and reviewer prerequisites

During execution investigate only unresolved questions using actual source and,
when useful, primary GNU/glibc/Rust target documentation, Chromium/Playwright
documentation and W3C accessibility guidance: what build baseline actually avoids
glibc 2.39; can the chosen browser run there offline; how should installed paths
enter existing harnesses without mixing IPC builds; what model residency/late work
is owned by supported production routes; which graph focus behavior exposes the
same facts without adding a second UI framework? No research is performed now.
Before fresh direct Sol-high design review, supply reconciled feature contracts,
public/private/test-only route map, declared compatible artifact/browser baseline,
all-view feature-closure checks and intended fixture/human rubric. Resolve material
design findings before affected code, then behavior RED/GREEN, fresh Sol-high code
review and independent Terra verification. Missing Engine capability stays blocked,
not an unsupported-mode completion. Keep only short execution status and needed
committed measurement tools; preserve the commissioned release checkout at cleanup.

## Direct reviews and qualification implementation

Fresh direct Sol-high design review approved the selected implementation after
requiring isolated Ubuntu22 compiled target, canonical heap plus distinct RSS
metrics and actual native-window close. Fresh direct Sol-high code review found
four qualification gaps: throughout-model bound/total RSS, recorded descendant
cleanup, exact queue admission and F06 UI workload. The harness now bounds the
entire service tree<=2GiB as a conservative model upper bound, separately checks
1GiB attached-baseline growth and browser512MiB heap growth, records same-pass
total RSS samples, records PID/starttime identities including immediately reparented
private-path browser helpers, and verifies no recorded live descendants after
actual Alt+F4 within30s. It admits eight waiters and rejects exactly one extra.
F06 includes10.8MiB exact clipboard/Back/refresh, six graph collapse/expand cycles
and pin eviction before real native cancel/fault/detach. Focused Sol-high source recheck APPROVE; these are qualification changes, not
changes to Engine/product behavior.

The installed CPU trace attaches only the recorded production supervisor after
browser bootstrap and before Engine attach, then observes its actual newborn
worker/threads. Container-local ptrace capability is confined to that task's PID
namespace; no host configuration changes. Network-family attempts and source-cache
writes are asserted from raw syscall traces. Raw logs remain owned scratch.
