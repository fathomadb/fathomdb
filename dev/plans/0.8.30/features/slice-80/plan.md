---
title: Slice 80 authorized delivery and registry consumers
status: COMPLETE
slice: 80
target_release: 0.8.30
---

# Slice 80 — Authorized delivery and registry consumers

Owner seq-414 directs normal release completion after qualification. Seq-415
renews3090 availability; seq-416 limits Orin transfers to this session. Broad
coverage-guided fuzzing and power-loss proof are allocated to0.8.31.
Direct Astra-medium passed this updated whole plan after one consolidated
FIX-1. Slice75 is accepted at `cd3a6caaa61fca8625f7468f4dae8341b958a241`;
parent commission `623f982798d2a2eddd396bbe104e81139a753741` is the clean,
main-reachable entry. Final reviewed immutable publication subject is
`e3d31f5cdc0df3e32dab9bad151c5ab5c321222c`: five crate README corrections
only, with no code, manifest, lock, runtime or producer change. Parent Codex
passed that metadata delta; the current whole-entry Astra-medium review passed
at623, and its unchanged executable applicability extends to this final binding.

Deliver one qualified, immutable 0.8.30 candidate through the normal release
workflow and verify the exact versions from public registries and Pages.
R30-REL covers delivery and finalization under the [release plan](../../plan.md),
[release design](../../../../design/release.md), and existing REQ-047–052 and
AC-052/054/056. The acceptance checks below refine those contracts locally.

## Entry and preparation

- Require a commissioned Slice80 and accepted Slice75 candidate. Under the
  owner's explicit seq-414 completion direction, bind the exact reviewed
  candidate and normal protected route before the irreversible tag push.
  Do not request the already-granted normal completion again. A new version,
  bad immutable package or changed product scope needs its own disposition.
  Preparation may proceed while75 finishes; publication cannot.
- At entry compare the release checkout and current `origin/main` with reviewed
  draft `f7fb1da9a81ee226c70ae347fa2fab3e64ac3c25`. Inventory relevant intervening
  changes to jobs, scripts, required checks, publisher identity, artifact scope
  and allocated drafts. Reconcile prework 1 dependencies, 3 acceptance, 6 docs
  and 7 delivery findings, Slice 10 preparation, and the actual Slice 20, 60
  70 and75 exits. Keep, adjust or reject claims using source and observed results.
- Trace the version axes, actual product and fork graph, and candidate-to-public
  artifacts through [preflight](../../../../../scripts/verify-release-gates.sh),
  [release jobs](../../../../../.github/workflows/release.yml) and
  [Pages jobs](../../../../../.github/workflows/docs-pages.yml). Slice 60 owns
  final fork qualification and registry-resolvable pins. Verify the exact external
  dependencies, feature closures and published Axis E API before tagging.
  Slice 20 owns any normal-path OIDC, visibility, route or smoke gaps.
- Recheck [0.8.29 closure](../../../0.8.29/features/slice-160/status.md#actual-publication-and-cleanup)
  and the [.29 Tegra delivery state](../../../release-state-0.8.29.json).
  Retain seq-398's separate .29 ownership and carry live retained URLs into
  the applicable manifest. The prior core publish, original failures and
  focused recoveries provide precedents with their original inputs.

## R30-REL acceptance

| Need | Release-local acceptance and maintained route |
| --- | --- |
| One trusted candidate | Resolve the reviewed, clean, main-reachable candidate by full commit SHA. Check lockstep Axis W 0.8.30, independent Axis E, changelog and crate metadata. Bind the prospective tag, workflow provenance, approvals, manifests and build inputs to that commit. Verify the peeled remote tag after the explicitly authorized tag push. Use applicable approved nonpublishing rehearsal evidence from75. |
| Protected production route | Verify the restricted group, exact `v0.8.30` workflow and environment route, identified reviewers, and real matching workers before approval. Check owned runner process, host, toolchain, workspace/cache, credential scope, exclusive device/lease and job bounds when applicable. Verify the trust facts for the actual active worker, including any old matching service. A missing route or reviewer holds the affected transition. [Tag-route check](../../../../../scripts/release/check-tag-cuda-route.sh). |
| Complete immutable packages | Require all builds before publication. Keep the Rust dependency order embedder-api, schema, query, embedder, engine, facade plus SDK, then CLI. Verify PyPI and npm normal-path publishers, including all five npm platform packages and the thin main package, and expected five-target wheel contents. Preserve existing published versions only after confirming exact version and applicable artifact identity. Join the shipping native members and wheels to Slice75 proof by digest or verify the affected changed bytes within the applicable authorization. [Release jobs](../../../../../.github/workflows/release.yml). |
| Exact registry visibility | Verify each required crates.io dependency before running dependent tiers and the required PyPI/npm versions before consumers. Record expected and observed values and use Slice 20's bounded poll. Distinguish transient visibility from missing or wrong bytes. [Crates wait](../../../../../scripts/release/wait-for-crate-version.sh), [package wait](../../../../../scripts/release/wait-for-registry-version.py). |
| Eight successful public consumer jobs | Require four Linux x64 jobs (crates CLI, crates SDK, PyPI, npm) and four paired Python/npm jobs on Linux ARM64, macOS x64, macOS ARM64 and Windows x64. Fresh registry installs prove exact package and native resolution. SDK/bindings write a fixture, assert its body and the missing-record outcome, close and exit, then reopen, read and close in a separate process. CLI checks exact version and integrity on the persisted fixture. Keep native path/version, process exits and platform bounds. [Registry smoke jobs](../../../../../.github/workflows/release.yml). |
| Normal finalization | After all publishers, eight consumers and sibling co-tagging succeed, use Slice 20's shared pinned OIDC route to promote only main `fathomdb@0.8.30` to npm `latest`. Platform packages remain staged and resolve by pinned optional dependencies. Require bounded observed visibility and the GitHub Release for the exact tag with its expected assets. [Co-tagging](../../../../../scripts/release/assert-co-tagging.sh), [recovery precedent](../../../../../.github/workflows/promote-npm-release.yml). |
| Public docs and downloads | Require actual authorized Pages build, deployment and public reads of the version page and advertised downloads. Verify retained Tegra URLs and expected manifests/digests. For a selected new Tegra delivery, use the applicable authorized Tegra route and public installed consumers for its own matching pair. Generic ARM CPU packages and retained older Tegra downloads cover their own configurations. [Pages route](../../../../../.github/workflows/docs-pages.yml), [live verifier](../../../../../scripts/release/verify-tegra-pages-live.sh). |

Slice20 implemented persisted-body, missing-record and separate-process reopen
in the maintained registry consumers. Reuse that reviewed implementation and
execute its actual registry-installed routes here. Compare the [SDK smoke](../../../../../scripts/release/smoke/smoke-crates-sdk.sh),
[Python smoke](../../../../../scripts/release/smoke/smoke-pypi-wheel.sh) and
[npm smoke](../../../../../scripts/release/smoke/smoke-npm-package.sh) with the
maintained qualification consumers. Slice20 owns implementation and offline
qualification of those flows and the exact-first-tag route guard. Slice75
qualifies the candidate consumer bytes;80 requires their actual authorized
registry processes and route. Archive/local installs cannot supply that proof.

## Entry reconciliation and concrete binding

Compare the earlier draft `f7fb1da9a81ee226c70ae347fa2fab3e64ac3c25`
with entry `623f982798d2a2eddd396bbe104e81139a753741`. Slices10/20 closed
selected dependency/tooling preparation, exact registry consumers, shared pinned
OIDC promotion, first-tag guard and useful diagnostics. Slices30/40/50 closed
correctness, measured latency and explicit module release;60 published all five
qualified fork crates;70 completed integrated source qualification;75 accepted
the final native/platform, source-archive, CUDA and Orin package scope. The
allocated prework1/3/6/7 drafts are reconciled through those owning exits; no
additional product feature, runtime migration or publication framework is needed.
Current README and42 active public documents already describe0.8.30 source;
actual published-doc receipt remains0.8.29 until this delivery succeeds.

Product/producer runtime subject remains
`53400f69d29cbc5029f0c759d58a5edb1f76e874`. The final tag may include the later
reviewed README, control and acceptance-record delta; those changes leave the
executable scope unchanged. Retain6d native/source proof and actual source75
scope, without replaying qualification for metadata. AxisW is0.8.30, AxisE
is0.8.0; exact Cudarc3509cd6b, Candle/kernel952edb7d and PTX0 remain unchanged.
All five fork packages are already public and verified; do not republish them.

Normal protected rehearsal44922568609 succeeded. Canonical production
`build-cuda-linux-x64-gnu` runs `cuda-preflight.sh --rerank-cuda`, so its applicable
shipping proof is the v3 embed+rerank cohort, not the base cohort. Accepted75
records v3 platform170228211cb33b89e27bb3c33a9a1550345a2b9348b03f9da0e2c1c12121dca2,
addonafe6eab14c2344fe82378b267985a342da3b90f8ec12cbd042f48cd7d0062403,
and wheel577a1ff5ba72b405567b9600bbd30f240edd56d3ca687743bf3e3ae9ad0e3b19.
Final docs-updated main a6aa14b451e4d9278179ebb8dd3668600c806adee41b3fc20b5b29a861f69943
differs from measured68482 main only in README;44 other payloads match.
Production builds the exact tag and joins actual shipping identities/applicability
to those proofs. Changed native bytes require affected-byte verification; a
README-only delta does not require native replay. Preserve75's original failed,
skipped, cancelled and timeout outcomes and scoped recoveries; no whole-hosted
CI green claim follows.

Parent owns tag creation/push, canonical JSON/board/ledgers, protected approvals,
GPU/Pages control-plane and both runner listeners.80 owns publication monitoring,
coordinated normal Tegra dispatch, independent actual-delivery verification and
final docs/status edits in its own checkout. Normal publication is authorized
byseq414; no fresh generic production approval is open. Parent binds the final
reviewed main-reachable SHA. After the tag exists, parent fresh-reads group3,
adds only exact `release.yml@refs/tags/v0.8.30`, preserves all restrictions and
passes the marker-free route check before GPU approval. Exact environment tag
policy and reviewers remain present. Parent's temporary group3 runner169 uses
GPU0 only, UUID `GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b`; GPU1 is foreign.
Parent removes only its temporary169 registration/credentials after delivery.

Matching0.8.30 Tegra publication is selected. Parent verified the exact Pages
branch policy `release/0.8.30` (policy67692074); main policy remains intact.
Existing Orin runner22 is live under parent's foreground listener ownership;
its registration, credentials, service and foreign Memex listener stay intact.
After final branch/main freeze and public npm0.8.30 visibility,80 dispatches
`jetson-tegra-cuda-evidence.yml` once with exact branch SHA and
`publish_to_pages=true`. Its wheel/addon must be built in that same run, not
substituted from local75 packages. DriverlessARM and public installed matching
pair consumers must pass. Parent holds main and release/0.8.30 fixed at e3d until both source-currency
checks and site build/deployment finish: the candidate must contain freshly
fetched current main.80 signals when that constraint is released.

After actual Tegra public installs pass,80 adds the delivered pair to the
existing retained manifest, preserving older artifacts. Reconcile published
support/install/CHANGELOG wording, add the accepted0.8.30 schema37 receipt to
`check-public-doc-truth.py` and mechanically advance
`dev/platform-capabilities.json` only at actual delivery. Parent owns the
canonical Pages authorization flag and integrates those final docs before the
normal docs Pages deployment/public readback. Authorization-only skipped docs
run44928516448 is not actual0.8.30 public-page proof. Use scoped Markdown,
public-truth, generated/state, glibc-floor and strict MkDocs checks as applicable.
No docs-only native/rehearsal or full-source regression is needed.

## Design, execution and recovery

1. Draft route and evidence decisions before commissioning. Resolve control-plane
   versus tag subject identity, first-tag protected-route readiness, installed
   native resolution and bounded registry propagation. The current first-tag
   bootstrap allowlist includes .30 after Slice20. Verify that route against
   actual group and environment policy. Where necessary, consult primary GitHub
   documentation on runner restrictions, approval and reruns, npm CLI/OIDC and
   dist-tag behavior, and registry immutability and digest metadata. Check fit
   against the pinned tools, actual workflow and resolved artifact paths.
2. Prepare any needed design/contract deltas and obtain direct Astra-medium
   design review. Submit the updated plan to direct Astra-medium, with at most
   one FIX-1 and re-review. Commit after PASS. Continued failure holds the
   affected transition. Keep the required owner decisions and prerequisites visible.
3. Once authorized, run the exact-tag delivery route and monitor actual jobs,
   dependencies, installed processes and public endpoints through completion.
   Keep dry-run/build, registry presence and fresh consumer proof distinct.
   Preserve original failure, cancellation and skipped states.
4. A partial publish stays open. Inspect the exact run, attempt, immutable tag,
   available package versions and bytes, and first failing diagnostic. Scope
   recovery to failed/unrun jobs and their dependent work on the same version.
   Use the existing owner-authorized recovery route and set a bounded attempt
   against evidence of a recoverable cause. A successful mutation with lagging
   reads calls for bounded verification. A bad runtime result or wrong immutable
   bytes needs an explicit disposition and, if necessary, a separately authorized
   corrected release. Never force-retag, bypass hooks, or blindly republish.
5. Return executable source or workflow defects to Slice 20 or their actual
   product owner. Require a committed behavioral RED and separate GREEN, direct
   Sol6.2-high review and direct Terra verification of affected actual executions
   for new repairs. Operational runs use their real outcomes. Repeat only the
   invalidated qualification if a correction changes release inputs.
6. Direct Terra independently verifies the actual authorized job outcomes,
   installed exact-version registry processes and native digests, persisted/missing
   and separate-process reopen results, finalization and public Pages/downloads
   even if no repair was needed. Use existing logs/artifacts and public readbacks;
   independent verification does not re-dispatch or repeat publication.
7. Close only once required publishers, consumers, co-tagging, finalization,
   live Pages and owned cleanup have attributable outcomes. Record the exact
   tag/commit, actual release attempts, final registry and Pages results and
   any remaining limits in the owning release state. Module release and
   post-reset claims retain Slice 50/60/70/75's actual bounds.
8. Reap only verified owned workers, containers and children, remove owned
   temporary registration and credentials, and confirm lease release and
   quiescence. Preserve useful raw logs and artifact evidence before cleanup.
   Merge any temporary documentation branch back into its owning branch and
   clean only inactive owned scratch/worktrees.

The actual delivery and the owner20-reviewed focused consumer recovery are
recorded in the [Slice80 status](status.md). The recovery retains exact tag and
package identities, original outcomes and existing OIDC finalization; it changes
no product/producer scope or passing native qualification.

## Entry and completion disposition

Slice75 qualification was accepted and the exact reviewed main-reachable
candidate was bound to the tag/group/environment sequence before GPU approval.
Seq414 authorized normal completion. Matching0.8.30 Tegra delivery used its
same-run payload and protected Pages route under session-only Orin authority.
Actual publication, focused consumer recovery, independent acceptance and public
readback are complete in the [Slice80 status](status.md). Original failures and
qualification limits remain visible; product/producer scope stayed unchanged.
