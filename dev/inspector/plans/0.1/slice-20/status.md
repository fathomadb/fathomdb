---
title: Inspector v0.1 Slice 20 status
date: 2026-10-31
status: ACTIVE
desc: Implemented record foundations awaiting final independent gates
---

# Slice 20 status

**ACTIVE.** Product source through `4d85fda5a`; retained diagnostic `55bd993c4`.
Sole writer on `dev/0.8.30` in the original task checkout. Started at
`16693b72c`; S15/S16 and the N16-A01 correction remain complete. No extra
worktree, push, publish or main merge. Final fresh code review, Terra and source
gates remain pending; implementation is not an integrated v0.1 release.

## Scope and authority

- Reconciliation is in [the plan](plan.md); reviewed refinements feed the
  [canonical design](../../../design/high-level-design.md) and this
  [single matrix](../verification-plan.md#slice-20-executed-evidence).
- Inspector-only Rust supervisor/session, preflight, typed locators/context,
  bounded current-layout readers, local authentication, document/tree UI,
  full metadata/attribute/copy, source links, samples and finite ownership are
  implemented. Text/vector/native graph implementations remain 30/40/50.
- D-01 exact flags/defaults and independent overlapping gates apply throughout.
  B16-01 remains conditional: selected Engine atomically deletes content and
  projections with no E target. Typed supplied-content export tests and real
  deletion/absence/all eight flags are separate; no storage fabrication,
  retention, Engine change or reconstruction.
- Engine/query/schema source remains the selected unchanged dependency.
  Minimal external wiring is Cargo workspace membership/lockfile only; no
  public dependency pin, Engine/MCP/SDK/schema/agent-access contract changed.

## Executed TDD and feature evidence

Each behavior has an executed failing RED with failure evidence in Git, then
separate minimal GREEN. Representative critical pairs follow; the rest remain
in normal history and named tests, without regenerated oracles.

| Behavior | RED → GREEN |
| --- | --- |
| Closed worker composition / real hostile session | `8cc9fcb14` → `f29424a9b`; `d1594d7b8` → `083db37f7` |
| Preserving admission / owned lifecycle | `dcbc0dcc3` → `fce5c0170`; `0aef6dc68` → `806d5655a` |
| Exact canonical identity / actual document UI | `3b920757e` → `8cdc54b04`; `b82d39530` → `355c941cc` |
| Conditional export / real atomic deletion | `7ea29196a` → `8f0e1a0a5`; `ba823e5e4` → `2a2a96728` |
| Source/attribute links and exact materialization | `372d0bb2c` → `fb3532b87`; `efa220d9d` → `497d4e378` |
| Queue/cancel / parent death / early startup PID | `6492f5edf` → `f1bab5d7e`; `1f744c8af` → `5faff1b6e`; `2f861076d` → `1e6be3daa` |
| Lossless tree bytes / null versus empty | `992b504e7` → `48e34af95`; `52c412782` → `74ebbc971` |
| UTF8 header guard / incremental payload reads | `54f2744f4` → `4007bc2b2`; `e9250ecb3` → `fe4a9d66e` |
| Metadata allocation / full fields / native UI | `0eb9cfabd` → `7a76d9471`; `6c1e4701c` → `cf71547dd`; `e0b8401ef` → `b290fb71c` |
| Exact scalar materialization presence / UI | `180d7a3bc` → `e1ec88b0d`; `6df7dfb1e` → `ad935493d` |
| Offline build description / package skeleton | `b698a5217` → `88659bf0a`; `8a9d621a6` → `56626a261` |
| Real F06/native cooperative deadline | `77bd54b8d` → `0c673ab1c` |
| Combined catalog bytes / combined item count | `0eb3f7c0e` → `df9b62834`; `a96808694` → `aa7e0f9d7` |
| Partial metadata / current-log navigation | `4df703d98` → `125a27b99` |
| Exact operational clipboard / revision chooser | `428a789f6` → `45df18d1b`; `98ee8b809` → `c3137f81b` |
| Reaped startup ownership / chunk back-forward | `608a95c07` → `e1f4d6748`; `d75c9c95b` → `4d85fda5a` |

Actual Chromium flows cover private bootstrap, attach, every record class,
structured expansion, raw/decoded exact scalar copy, full Unicode metadata,
materializations and same-logical-ID retained revision choices, keyboard Enter,
Back, cache eviction and explicit refresh. Ordinary current data is only its
latest value; both retained log entries remain individually accessible.
Stored markup is inert. Filters use full identities; samples and partial
catalogs never prove absence. Source and derived roles remain distinct.

Scoped `cargo clippy -p fathomdb-inspector --all-targets --offline -- -D warnings`
and the ordinary Inspector suite passed before the latest presentation-only
changes: 32 unit tests plus CLI, noGUI and two worker tests. Focused new and
adjacent browser flows passed afterward. Native GUI and three resource tests
are explicitly ignored in ordinary runs, not counted as executed there.
The actual GUI oracle was run explicitly on the declared graphical executor.

## Actual resource and startup measurements

- Real 128 MiB body/identity REDs grew RSS about 136 MiB. Incremental read
  GREENs grew about 2.15/2.37 MiB for bounded windows/previews, respectively.
- Standalone F06 worker fixture: 100,003 nodes, 100,000 log entries,
  10,000 edges and a 10,800,042-byte body. The unchanged scale oracle passed with a truthful partial
  catalog in 25,020 ms, 84,640 examined and worker growth 9,297,920 bytes.
- Full native/browser fixture adds the structured baseline record (100,004
  nodes); qualification passed at `0c673ab1c`: cancel ACK 3.73 ms,
  reap/lock 32.70 ms; real worker fault 29.00 ms; detach 20,882 ms with preceding
  partial catalog. Exact large clipboard and keyboard Back/refresh passed.
  Observed 25 ms sampling: browser heap growth 31,610,366 bytes and combined
  supervisor/session RSS growth 34,770,944 bytes. Successful final milestone
  samples and cleanup precede receipt. This is not an instantaneous peak or
  installed-release measurement.
- Separate synthetic queue ownership proved one executing/eight waiting/four
  busy, no late payload; parent death and startup cancellation released locks.
  Synthetic pauses are not the native execution witness above.
- Hostile-parent startup probe calibrated actual socket denial and traced the
  real cleared worker plus ten threads: zero Internet/ambient-cache probes,
  partial cache entry/content snapshot unchanged (measurement atime excluded),
  exact environment/header/lock/close oracle passed. Two specified fixture
  canaries were absent from captured producer streams. This is model-free
  startup evidence, not general log or model-operation qualification.

All probes, prerequisites and parameters are committed in
[the tool index](../../../../../src/inspector/tools/README.md). Earlier clipboard
synchronization corrections and two invalid fixture assumptions were narrowly
reviewed by independent Sol with unchanged product intent: Playwright async
predicate timing, Engine false→NULL normalization, and ordinary writes already
having recorded legacy/incomplete revision evidence. No database mocks or
impossible erasure target were introduced.

## Reviews, limits and next gates

Fresh direct Sol-high DESIGN review approved the source-backed design; scoped
refinements were independently checked before affected implementation. The
fresh final Sol-high CODE source pass is complete: its confirmed chunk-navigation
finding has a separate executed RED/GREEN. Final recheck and direct Terra verification
remain pending. `node --test src/inspector/tests/browser/startup-failure.mjs`
passed after `e1f4d6748`: actual syscall tracing proves Cancel does not signal the
reaped refused worker. Scoped all-target clippy and paired build also passed.
`node --test src/inspector/tests/browser/chunk-navigation.mjs` passed after
`4d85fda5a`: final chunk → Previous → Next works in the real browser. The tests
were unchanged between each RED and GREEN.

Actual development executor is Ubuntu 24.04.5/glibc 2.39, Ryzen Threadripper PRO
5945WX, Rust 1.95.0, Node 26.8.2, Chromium 156.0.8078.4 and Playwright 1.62.1.
Tool hashes are emitted by the re-runnable resource probe. The tested release
candidate requires GLIBC 2.39 and cannot run on stock Ubuntu 22.04. The selected
22.04-compatible distribution and installed/human qualification remain 60;
no target change, publication or broad support claim is implied.

Remaining: final code review/fixes/recheck, Terra actual final-code and sampled
RED verification, final default artifact/native launch, scoped docs checks and
required repository source gate with this checkout's own environment. The scoped Markdown gate actually failed the unchanged historical
`release-state-0.8.27.json` slice90 AC-073 checkpoint: bundle directory must be
canonical/absolute and named selector AC-073 is missing. Prior structure, plan,
anchor and generated-view legs passed; separately run traceability, architecture
authority, lifecycle, public-doc lint and offline links passed. No full-workspace
green is claimed. No extra
checkout cleanup is needed; preserve this task checkout and committed tools.
