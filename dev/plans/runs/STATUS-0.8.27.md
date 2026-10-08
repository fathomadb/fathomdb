---
title: FathomDB 0.8.27 release status
status: ACTIVE
---

# FathomDB 0.8.27 release status

This board is a generated-view consumer of
`dev/plans/release-state-0.8.27.json`. Update machine-owned facts in that state
file and regenerate; keep evidence and qualification prose here.

## Current state

<!-- BEGIN GENERATED release-state:0.8.27:status-current-state -->**Next is Slice 135 (PERFORMANCE-BASELINE), PLANNED.** Completed on local `release/0.8.27` per release state: 0 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 1 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 2 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 3 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 4 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 5 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 6 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 7 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 8 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 9 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 10 (`3097d191511d81a221b038ccd2e14f074dcafa6d`) · 20 (`b455bb73fb2b04c91f50e6e5dbdc16752325453b`) · 30 (`6ba3be95cd043570da1deafbe4e2f78c878d8a87`) · 40 (`fdd7fb646b0fb922b9b8fea134ef7ce7e71a5aae`) · 50 (`1f5b8614813b5a363ec5f81fcb580d48da4a4e8f`) · 60 (`d5a5bd39b3ee8a04bd080df451204564c6849bd1`) · 70 (`36fc2352cf243e022315ea302368d9424096aebd`) · 80 (`b7403958a3839d371c1672335c517fa762a451cf`) · 85 (`7a2f9bf90783f545603516502bac0016d4b93a14`) · 90 (`1398c821dd26b7945bb2f6fbfa02b68cd4daa8af`) · 100 (`731130c22a40bfed3f50e9f205500d5080022cc7`) · 103 (`c2e80ff7683fe856a4cf372a088897c3450b0b9a`) · 110 (`a25d063cd3e1642ad08dcc6aed3691b94445fe20`) · 114 (`25115902db8b9b5648c5ceca1823d3cc66fec8e8`) · 115 (`012e132920147396ac195f14af74444dd698f48e`) · 120 (`e05bfd5330beb737b7704300e2752d2b977bb5b8`) · 130 (`58b0bd192f6bc7cc87ea6e8099bfe1c52e68893f`) · 132 (`2996417f0cb78b2dbf5262de53798ae6a263469a`) — state-owned, not an `origin/main` claim.<!-- END GENERATED release-state:0.8.27:status-current-state -->

Prework Slices 0-9, bounded preparation Slice 10, correction-safe erasure
Slice 20, real-surface comparator Slice 30, and engine-foundation Slice 40 are
complete on `release/0.8.27`. Slice 40 preserves runtime behavior, schema,
public API, feature gates, and publication state while moving shared engine
foundations to private modules.

Engine-domain Slices 50, 60, 70, and 80 are complete. Slice 80's product
extraction preserves the read, search, graph, evidence, public, hidden,
schema, SQL, wire, and feature-gated contracts while moving their
implementation into private semantic modules. Its independent three-cycle
rereview is bound at `b7403958`, with closeout `e8e603b7`. This is a review
binding, not a claim of post-fix full canonical or surface verification.

Slice 100 and Slice 103 are complete. Slice 103 carried forward Tegra build
support, fixed the reproduced Windows local-drive WAL defect, and added
operator discovery and offline completion of owed erasures. The integrated
code SHA is `c2e80ff7683fe856a4cf372a088897c3450b0b9a`. Installed Windows
and Jetson wheels, independent review, strict repository verification, and
the broader release check passed against that code. True network-share WAL
remains outside the qualified support claim. See the
[Slice 103 status](../0.8.27/features/slice-103/status.md) for artifact hashes,
evidence, and limits. No publication is authorized here.

Slice 110 is complete on `release/0.8.27`. Its final product source includes
the reviewed NAPI work and Tegra fix merged at `a25d063cd`. Installed Linux
x64 CUDA Node and Python packages passed forced-GPU, CPU and no-GPU checks;
Windows x64 MSVC, hosted Linux arm64 GNU and both macOS Node package rows
passed. The merged installed Orin addon passed 20 / 20 witness-enabled
forced-CUDA processes on a quiet host. The clean amd64 `agent-verify` gate
passed 184 / 184 suites with strict security 0/0/0. The
[Slice 110 status](../0.8.27/features/slice-110/status.md) records artifact
hashes, review, feature-complete and platform evidence, and measured Tegra
limits. Slices 114 and 115 completed earlier under HITL sequencing exceptions;
Slices 120 and 130 have since completed. No publication is authorized here.

Slice 132 completed after Slice 130. It added `fathomdb-sdk`, a dedicated Rust
SDK with the Python/TypeScript surface, under
`ADR-0.8.27-rust-sdk-parity.md`. The `fathomdb` facade is unchanged, and the
provider/plugin crate posture remains a separate unruled decision.

Slice 117 is planned. It will deliver a CUDA-capable Linux AArch64
(Jetson/Tegra) Node addon through the release process, because the published
`fathomdb-linux-arm64-gnu` npm package is CPU-only. On 2026-10-05 the owner
added it to the ladder, superseded the 0.8.23 Tegra npm exclusions for its
scope, ruled its channel as the existing Tegra Pages route (not the npm
registry) for 0.8.27, and ruled that Slice 110 ships early `cuInit` with the
aarch64-Linux allocation fallback. Its prerequisites are now met: Slice 110 is
complete on `release/0.8.27`, and the owner's Linux x86_64 hold was met at
Slice 110 candidate `8b76f6115` (recorded by `c761b9369`). Implementation now
waits only on the unruled `slice-117-delivery-shape` items, so Slice 135 stays
next and Slice 117 follows it in the remaining ladder. Publication remains
behind `release-0.8.27-publication`. The 0.8.27 allocator approach is the
synchronous fallback with early `cuInit`; no memory pool ships in 0.8.27, and
0.8.28 evaluates an explicit or lazily created pool.

### Off-ladder landings (recorded 2026-10-07)

Two fixes landed on `release/0.8.27` outside the slice ladder. No completed
slice verified them, so Slice 135 and Slice 150 must run on a candidate that
contains both and must cover them:

- **Engine close releases the embedder.** `c816b8653` (merged at
  `96796fe04`, with `2ff744b06` and `8247d91a4`) makes `Engine::close` release
  the engine-owned embedder after its workers drain, and drops it after the
  close lock so an embedder whose `Drop` re-enters `close` cannot deadlock.
  Before the fix a closed engine still held its model (about 159 MiB per
  closed engine in the 0.8.28 pool study). This changes lifecycle behaviour:
  memory now returns at `close()` rather than when the last engine handle
  drops. Module-level embedding and reranking models still live until process
  exit (0.8.30 backlog B30-01). Focused tests: `projection_runtime` and the
  Slice 90 close tests. The 0.8.28 study rerun on this fix is
  `llm/0.8.28-tegra-pool-study` at `901455d16`.
- **Tegra install route points at the published `0.8.26+tegra` wheel.**
  `5ceab8624` (RED) and `c23e2d23f` pin `docs-pages.yml` to the
  `0.8.26+tegra` wheel that run 37141909082 published to Pages on 2026-10-03
  (SHA-256 `728df862…`), and change the Python classic-Tegra warning,
  `fathomdb doctor gpu --help` and the public install docs (`172fc6599`) from
  the `0.8.24+tegra` command, which the live index no longer serves. The same
  fix for `main` is PR #254. Slice 150's installed-artifact and Tegra route
  smokes must use the 0.8.26 route, and the Tegra publication for 0.8.27
  updates the pin again.

### Slice 135 execution note (2026-10-08)

Slice 135 is in progress on `llm/0.8.27-slice-135`; the release-state-owned
ladder still lists it as next until its complete closeout. The current
comparison product source is `224e44c593c13d86ece648adabe445723db04070`,
after Slice 132 and both off-ladder landings. Audited exact-source paired
E01–E12 engine and installed Python S01/S02 diagnostics exist. The Python
whole-sequence pooled p50 changed +0.287% against exact 0.8.26; engine
vector, hybrid and populated-open p50 increases remain attribution leads.
The four-area Phase 1 checkpoint and full protocol freeze are still open.
The [active Slice 135 plan](../0.8.27/features/slice-135/plan.md#active-phase-1-execution-snapshot-2026-10-08)
and [direct-execution hand-off](../0.8.27/features/slice-135/handoff-phase1-2026-10-08.md)
hold the remaining work and evidence limits. Local raw archives are preserved
but not yet committed; their retention and the deferred Gitleaks decision
remain end-of-phase work. No publication or release qualification claim
follows from these diagnostics.

## Immediate next action

| | |
| --- | --- |
| **Immediate next action** | <!-- BEGIN GENERATED release-state:0.8.27:status-next-action -->**Commission Slice 135 (PERFORMANCE-BASELINE)** — 0.8.26 performance preservation and improvement qualification. **Remaining ladder:** 135 → 117 → 140 → 150.<!-- END GENERATED release-state:0.8.27:status-next-action --> |

## Open decisions

There is <!-- BEGIN GENERATED release-state:0.8.27:status-live-open-count -->THREE<!-- END GENERATED release-state:0.8.27:status-live-open-count --> live open decisions:

- authorize tagging/publication only after the complete ladder and release
  qualification pass.
- `slice-132-external-provider-disposition`: decide whether the published
  provider/plugin contract remains a supported extension boundary.
- `slice-117-delivery-shape`: settle the Jetson Node package name and npm
  name reservation, loader policy, Pages retention, aarch64 import-time
  `cuInit` contract and the other items in the
  [Slice 117 design](../0.8.27/features/slice-117/design.md) § 10. The channel
  is ruled (`slice-117-channel-tegra-pages`): the existing Tegra Pages route;
  the npm registry options are not chosen for 0.8.27.

`D27-runtime-topology` is ruled as Option B by HITL decision `seq-293`. That
ruling selected the architectural direction. `seq-295` accepted the reviewed
numeric, default, API, error, shutdown and supersession specifics in
`ADR-0.8.27-engine-owned-runtime-topology.md`. Slice 90 implemented and
verified that successor while retaining the synchronous projection/commit
ownership model. Its stage-2 runtime checkpoint is bound in release state
with candidate/binding SHAs and hashed performance, code-review and
read-only-verification receipts. The always-on checkpoint gate enforces that
binding and stage-3 ancestry. See the
[code-grounded resolution](../0.8.27/features/slice-90/independent-findings-resolution.md).

## Completed release-branch ladder

| Slice | Scope | Status and evidence |
| ---: | --- | --- |
| 0 | Environment and infrastructure | Complete at `a3e6cff6`; existing worktree/base and local/unavailable capabilities recorded. |
| 1 | Dependencies and pins | Complete at `a3e6cff6`; updates retained, postponed, or allocated with evidence limitations explicit. |
| 2 | Repository/documentation cruft | Complete at `a3e6cff6`; disposition inventory only, no deletion. |
| 3 | Needs, requirements, acceptance, allocation | Complete at `a3e6cff6`; release-local contracts, global acceptance unchanged. |
| 4 | Architecture/code alignment | Complete at `a3e6cff6`; code-grounded module and ownership map. |
| 5 | Verification adequacy | Complete at `a3e6cff6`; deep existing evidence and missing non-vacuous oracles allocated. |
| 6 | Stale-documentation evidence | Complete at `a3e6cff6`; exact current/historical correction set allocated. |
| 7 | Build and delivery evidence | Complete at `a3e6cff6`; open recurring causes separated from closed regressions. |
| 8 | Reserved | Complete at `a3e6cff6`; intentionally empty. |
| 9 | Proposal review and closeout | Planned at `a3e6cff6`, closed at `73c53ffd`; every proposal ruled, Slice 10 inputs design-reviewed, and verification passed. |
| 10 | Bounded repository preparation | Complete at `3097d191`; current truth, dependency security, Action comments, digest authority, and Slice 30 prerequisites pass review. |
| 20 | Correction-safe source erasure | Product complete at `b455bb73`; exact atomicity, at-rest, binding, and unchanged-fixture Memex evidence pass review. The shared Python artifact gate is corrected at `6ba3be95`. |
| 30 | Current inventory and comparison guardrails | Complete at `6ba3be95`; deterministic 13-row real-surface baseline, mutation-tested comparator, candidate-bound artifact gates, independent design/code review, and focused verification pass. |
| 40 | Engine foundation and test seams | Complete at `fdd7fb64`; private error, identity, temporal, and hook modules preserve root paths and cfg gates. Final surface comparison, 124-suite gate, strict security, workspace Clippy/check, and independent review pass. |
| 50 | Engine erasure, lifecycle, dependency, and provenance | Complete at `1f5b8614`; private semantic modules preserve root paths and close nonterminal soft-closure residue with reviewed RED/GREEN evidence. |
| 60 | Engine write, ingest, and consolidation | Complete at `d5a5bd39`; six private write-domain modules, full-state write-boundary characterization, and bounded abort-seam carry-over correction. Public/hidden structural surfaces and release probe equal; test inventory additive-only; 127/127 gate and live AC-037 pass. |
| 70 | Engine projection, embedding, and reranking | Complete at `36fc2352`; ten private modules preserve public paths and runtime shape. Canonical, security, feature-complete, workspace, and artifact gates pass. |
| 80 | Engine read, search, graph, and evidence | Complete at reviewed candidate `b7403958` and closeout `e8e603b7`. `e9631b97`/`bca0c99d` remain historical. Production changes ended at `0efa62c5`; no post-fix canonical PASS or official post-fix public/hidden capture is claimed. |
| 85 | Engine carrier ownership and dependency-boundary enforcement | Complete on `release/0.8.27` per release state at reviewed candidate `7a2f9bf9` and closeout `8cd3389d`, from commissioned baseline `4c75bfec`. Design review cycle 1 then failed the enforcement half (boundary gate), cycle 2 of the FIX-1 head found residual gate gaps (D-14..D-22), cycle 3 of the FIX-2 head found macro-hidden edges and smaller gaps (D-23..D-27), and cycle 4 of the FIX-3 head found a renamed or re-pathed `include!` and qualified-self serde paths (D-28, D-29); FIX-1 through FIX-4 are recorded in `features/slice-85/tdd-chronology.md`, and FIX-4 was confirmed by a green mutation-script run. A test review of the FIX-4 head then returned PASS-WITH-FIXES (T-1..T-8); test FIX-1 is recorded in the same chronology. Test review cycle 2 then returned FAIL (T-9..T-12); test FIX-2 is recorded in the same chronology and was confirmed by a green 218-assertion mutation run and the feature-gated engine runs. Test review cycle 3 then returned FAIL (T-13..T-15); test FIX-3 is recorded in the same chronology and was confirmed by a green 222-assertion mutation run. Test review cycle 4 then returned FAIL (T-16); test FIX-4, recorded in the same chronology and confirmed by a green 240-assertion mutation run, awaits candidate rebinding. |
| 90 | Engine open, configuration, runtime, operator, and facade closure | Complete on `release/0.8.27` at reviewed source candidate `1398c821d`. The stage-2 runtime checkpoint, final Sol code review, final independent Terra verification, 180/180 full gate, strict GPU gate, frozen D27 comparison, named performance gates, installed bindings, Windows MSVC routes, and official public/hidden comparisons pass. The AC-073 stress PASS and superseded AC-075 combined-selector failure remain distinct. See [Slice 90 status](../0.8.27/features/slice-90/status.md) and [final verification](../0.8.27/features/slice-90/final-review-verification.md). |
| 100 | PyO3 binding decomposition and subscriber correction | Complete at production candidate `731130c22` and reviewed documentation/ADR closeout `89c0a7c70`. The source/runtime inventory, installed wheels, focused subscriber tests, independent Sol code and design reviews, Terra verification, affected platform routes, and strict full gate pass. The full gate reports 178/180 suites with two environment skips and zero security findings. See [Slice 100 status](../0.8.27/features/slice-100/status.md) and [verification](../0.8.27/features/slice-100/review-verification.md). |
| 103 | Tegra build continuity, Windows WAL, and owed erasure recovery | Complete at integrated code `c2e80ff76`. The exact-code installed Windows and Jetson wheels, independent review, 182/182 strict suites, and the broader release check pass. See [Slice 103 status](../0.8.27/features/slice-103/status.md). |
| 110 | NAPI binding decomposition and subscriber correction | Complete at merged source `a25d063cd`. Reviewed native ownership and TypeScript subscriber, installed Node platform pairs, amd64 CUDA Node/Python, merged Orin witness 20/20, feature-complete 353 passes, and clean strict gate 184/184 pass. See [Slice 110 status](../0.8.27/features/slice-110/status.md). |
| 114 | Engine configuration and constants audit | Complete at `25115902d` under the HITL sequencing exception; settings and runtime behavior unchanged. See [Slice 114 status](../0.8.27/features/slice-114/status.md). |
| 115 | Engine performance characterization | Complete at `012e13292` under the HITL sequencing exception. Twelve real-engine cells have seven valid samples each, with binary-bound profiles, reviewed receipt, independent code review and Terra verification. The inherited Slice 90 inventory mismatch prevents a full-workspace green claim. See [Slice 115 status](../0.8.27/features/slice-115/status.md). |
| 120 | TypeScript SDK decomposition | Complete at `e05bfd533`; root export map, private concern modules, exact public-surface and installed consumer checks, independent review, and 184/184 strict gate pass. See [Slice 120 status](../0.8.27/features/slice-120/status.md). |
| 130 | Python SDK decomposition | Complete at `58b0bd192`; thin facade, seven private concern modules, exact 119/1,131 public-surface equality, independent Sol review and Terra verification. Strict gate's two shared-venv provenance failures passed checkout-owned focused reruns; no uninterrupted full-gate green is claimed. See [Slice 130 status](../0.8.27/features/slice-130/status.md). |
| 132 | Dedicated Rust SDK parity | Complete at `2996417f0`. `fathomdb-sdk` covers 44/44 governed operations through `Engine`, `read`, `graph`, and `admin`, with `rerank` and `embed_batch_cls`. It has the shared option defaults and an `ErrorKind` per error class. The checker refuses core leaks. Opus design review, Fable design review, Opus code review, and Sonnet verification ran. The strict gate passed lint, typecheck, security, and Rust; its two Python suites failed only on a shared-venv receipt. The first crates.io publish needs a HITL token bootstrap. See [Slice 132 status](../0.8.27/features/slice-132/status.md). |

## Verification boundary

The final whole-work review covered the complete Slice 20/30 plans, assigned
functions, implementation, tests, baseline, records, and generated artifacts.
Committed RED/GREEN rounds closed stale Python native shadowing, debug NAPI
leakage, multiline cfg ownership, distinct-filesystem capacity, scratch
ownership, and fixture-cleanup defects. Independent design review, code review,
and focused verification passed at clean `2967593c`; the final `6ba3be95`
change only made an older packaging assertion follow the canonical wrapper.

The reviewed baseline has 13 rows and SHA-256 `06212f66…`; a clean later
capture compared equal with no metadata or row diff. The canonical gate passed
lint, typecheck, strict security 0/0/0, Rust, TypeScript, executable NAPI, and
123 of 124 registered suites. Its sole failure was that obsolete packaging
assertion. After correction, the full Python suite passed 1,533 tests with 27
documented skips and the candidate receipt validated. Slice 150 retains fresh
installed-package and platform qualification.

Slice 40's final clean candidate `fdd7fb64` produced a fresh 13-row capture
that compared equal to the immutable baseline with empty metadata and row
diffs. The canonical gate passed all 124 registered suites with no failures,
skips, or exclusions; strict security was 0/0/0; the Python native receipt was
candidate-bound; and full-workspace Clippy/check passed. The locked Slice 20
erasure matrix remained unchanged and passed 5/5.

Slice 50's final clean candidate `1f5b8614` split the lifecycle, provenance,
dependency, and erasure domains into private modules and closed nonterminal
soft-closure residue without changing public paths. Independent design/code
review and verification passed. Public and hidden structural surfaces are
equal; hidden inventory has only reviewed additive tests; the canonical gate
passed 127/127 with strict security 0/0/0; workspace Clippy/check and the
candidate-bound Python receipt passed.

Slice 60's final clean candidate `d5a5bd39` moved the write facade,
validation, commit, provider transport, ingest, and consolidation into six
private modules. `actuation.rs` kept its existing domain, and every public path
is unchanged. Before the moves, a new write-boundary suite compared a full
`sqlite_master`-derived snapshot at ten refusal boundaries, plus a
validation-before-mutation property. It passed unmodified production and
failed under every recorded mutant.

Post-closeout adversarial review added real refusal coverage for both commit
exits. Owner follow-up then proved that the private abort marker could survive
validation and that its hook could survive an unrelated commit failure;
`81d723b1` is RED and `d5a5bd39` is GREEN. The final boundary suite passes
15/15 debug and 14/14 release with `test-hooks`.

The final 13-row public surface is equal with empty metadata and row diffs.
All eight hidden structural rows and the 41-item release probe are equal; test
inventory has zero removals or changes and reviewed additions only. Target
coverage is 261 targets with 53 feature-complete-only. The full canonical gate
passed 127/127 with no skips or exclusions. On the named 2026-09-26 unconfined
executor, the AC-037 live layer ran and passed; security was 0/0/0. Workspace
Clippy/check and the candidate-bound Python receipt passed.

Slice 70's final clean candidate `36fc2352` moved projection runtime, worker,
commit, registry, rebuild, vector storage and equivalence, mean, embedding,
and standalone reranking into ten private modules in ten verbatim batches.
`lib.rs` shrank from 25,899 to 19,001 lines, and every public path is
unchanged. A new residue suite proves that a failed projection commit leaves
no terminal, sidecar, vec0, or failure audit. The pre-existing red slice35
audit was repaired. The public surface is equal; the hidden surface is
additive only. The canonical gate passed after a path-only Windows WAL guard
retarget. Strict security was 0/0/0 with both AC-037 live layers, run
through a temporary per-binary AppArmor `userns` profile for
`/usr/bin/unshare`. The feature-complete gate passed 349/357
with 8 documented ignores on the RTX 3090s. Workspace Clippy/check and the
candidate-bound Python receipt passed.

Slice 80's historical structural candidate `8e449963` moved fusion, filter,
search types and execution, search APIs, telemetry, read verbs, the reader-pool
implementation, and graph types/codec/execution/traversal into private
modules. Three characterization tests killed their specified mutants before the
moves. Production is unchanged after `0efa62c5`; historical test candidate
`31e78529` added the initial result-codec properties, and `1d7f826c` adds the
exact JSON-value assertion. The former independent `gpt-5.6-sol` high-reasoning
rereview at `e9631b97` and closeout `bca0c99d` are historical only. The
strengthened follow-up and corrected Slice 85 contract were independently
rereviewed through `b7403958`.

At exact historical candidate `3e60cc5d`, the canonical gate passed 127/127,
workspace Clippy/check and the candidate-bound Python receipt passed, the
13-row public capture was exact, and the 33-row hidden capture was additive
only with 261 additions and no changes or removals. Those receipts do not
cover production changes through `0efa62c5`. At exact `e9631b97`, the public
capture was blocked by less than 100 GB free, the hidden capture by
`nvidia-smi` exit 9, and canonical verification by an advanced-ref versus
historical-ref fixture. There is no post-fix full canonical PASS or official
post-fix public/hidden capture.

At live `24813b8e`, whose `src/` matches `31e78529` and `e9631b97`, unconfined
security including AC-036 and both AC-037 layers passed 0/0/0 and `test-rust`
passed in 516,820 ms. Of 127 registered suites, 125 ran, 123 passed, 2 failed
only because the local native module and consequent native receipt were
missing, 2 skipped, and none were excluded. This is diagnostic, not a
canonical PASS or release-qualified AC-037 receipt.

The post-hoc adversarial design review of Slice 80 (2026-09-27) returned
PASS-WITH-FIXES. Fix-1 `b8af4d86` made two changes, with no behavior change:

- it restored four widened carrier fields to private by keeping
  `TelemetrySink` and `EvidenceCapture` at the root;
- it narrowed six over-visible items.

The records list four Slice 80 module cycles and add the Slice 140 test-seam
carry-over. Commissioned Slice 85 owns elimination of all four and non-root
semantic placement of every root-kept carrier; only the three inherited cycles
inside its bounded read/search/graph policy are initially allowlist-eligible.
Other crate cycles and broad import normalization are reported without being
absorbed into Slice 85. Slice 90 consumes the settled boundary. See
`features/slice-80/design-review.md`.

Follow-up `9700991f` adds generated coherent result-codec typed round-trip and
positional evidence-corruption properties. Lint-only follow-up `31e78529`
packages the fixture input. Follow-up `1d7f826c` strengthens the coherent
property to exact generated canonical JSON-value equality for its fixed
one-seed/one-target/zero-or-one-evidence shape; the body-loss mutant fails that
assertion and exact restoration passes. No generated golden oracle exists.
The former `e9631b97` review and `bca0c99d` closeout are historical. The
current follow-up is independently review-bound at `b7403958`; closeout is
`e8e603b7`, without expanding the verification evidence above.

The `66e27983` live AC-037 receipt and `24813b8e` diagnostic result do not
qualify later HEAD or the final candidate. By owner ruling, Slice 150 alone
must execute the HITL runbook against the exact final candidate with
grant/revert evidence.

## Boundaries

- This release starts at schema 34 and currently proposes no schema migration.
- Publication is unauthorized.
- Slices 30, 40, 50, 60, 70, 80, 85, and 90 are complete. Slice 85 was commissioned
  by `seq-294` after its independent design review passed and its exact-candidate
  AC27-85F entry receipts passed at `4c75bfec`; it completed at `7a2f9bf9` with
  closeout `8cd3389d`. Later slices require separate commission. The
  Slice 85 design was corrected on 2026-09-27 after a
  code-grounded review (handler-result ownership, `Engine`-method and
  field-alias edges, crate-root semantics, ordered batches), then aligned with
  the release's established bounded-decomposition method: narrower handler
  errors, item-level root nodes, complete inventory with scoped enforcement,
  and no whole-crate or line-count-driven ownership expansion.
- The [Slice 90 design](../0.8.27/features/slice-90/design.md) and
  [completion record](../0.8.27/features/slice-90/status.md) cover runtime,
  projector and operator handoffs, effective configuration behavior, and
  retained reader-connection and shared search-control ownership. The
  accepted Option B successor and stage-2 checkpoint are implemented and
  verified. There is no Slice 91 allocation. Slice 100 is complete on the
  release branch with the Python subscriber defect corrected and its accepted
  ADR reflected in active architecture and design documentation.
- [Slice 103](../0.8.27/features/slice-103/status.md) is complete on the
  release branch. Its exact-code Tegra and Windows wheels and erasure recovery
  proof are recorded; it does not authorize publication.
- [Slice 110](../0.8.27/features/slice-110/status.md) is complete. Native
  ownership and the subscriber contract pass independent review; installed
  Linux, Windows, macOS and CUDA package routes pass. The merged Orin addon
  passed 20 / 20 witness-enabled installed forced-CUDA processes on a quiet
  host. The clean amd64 full gate passed 184 / 184 suites. Slices 120 and 130
  subsequently completed.
- [Slice 114](../0.8.27/features/slice-114/status.md) completed its engine
  configuration audit under the HITL's 2026-10-04 sequencing exception. The
  152-declaration census, public-setting trace, post-open SQLite witness,
  corrected guidance, design/code reviews, and Terra focused verification are
  recorded without claiming Slice 110's Tegra row or a full-workspace gate.
- [Slice 115](../0.8.27/features/slice-115/status.md) completed its current
  engine characterization under the HITL's sequencing exception. The frozen
  protocol, twelve path cells, real default-model probe, raw receipt and
  profiles pass independent code review and Terra verification. A focused
  clean-checkout Python suite passed; an inherited Slice 90 inventory hash
  mismatch blocked Slice 115's earlier full-gate attempt. The later clean
  Slice 110 gate passed 184 / 184 suites.
- [Slice 132](../0.8.27/features/slice-132/status.md) is complete at
  `2996417f0`. `fathomdb-sdk` reproduces the Python/TypeScript surface in Rust,
  and the parity checker covers it (44/44). Before the `v0.8.27` tag, its
  first crates.io publish needs a one-time HITL token bootstrap (ledger
  seq 273). Slice 135 is next.
- [Slice 117](../0.8.27/features/slice-117/plan.md) depends on Slice 110,
  which is complete, and is planned. It plans an opt-in CUDA-capable Jetson
  Node addon built through the existing Jetson Tegra CUDA evidence workflow,
  with installed and post-publication Jetson smokes. D-80.7-3 and D-80.6-2 are
  superseded for its scope, and its channel is ruled as Tegra Pages. The
  owner's x86_64 hold is met; implementation waits only on the remaining
  `slice-117-delivery-shape` rulings. No other slice depends on it.
