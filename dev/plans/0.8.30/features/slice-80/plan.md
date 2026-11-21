---
title: Slice 80 authorized delivery and registry consumers
status: PROPOSED
slice: 80
target_release: 0.8.30
---

# Slice 80 — Authorized delivery and registry consumers

Owner seq-414 directs normal release completion after qualification. Seq-415
renews3090 availability; seq-416 limits Orin transfers to this session. Broad
coverage-guided fuzzing and power-loss proof are allocated to0.8.31.
Direct Astra-medium passed this updated whole plan after one consolidated
FIX-1; final75 acceptance remains the entry gate for actual delivery.

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

The prospective package-input candidate is
`53400f69d29cbc5029f0c759d58a5edb1f76e874`, already on remote main and
release/0.8.30.75 still owns its final artifact acceptance. Retain the6d native
matrix and source proof within their unchanged executable scope; the later
README/control/acceptance-record delta is explicit. Final package bytes and
public installations need their own exact identities. Do not replay passing
native/source tests for that metadata delta.

The normal protected rehearsal is44922568609. The obsolete6d rehearsal was
cancelled before GPU execution; automatic duplicate CI runs were cancelled.
Keep one current rehearsal and recover only failed/unrun jobs on a demonstrated
cause. The existing temporary user runner169 uses group3 and the selected3090;
stop it and remove its own registration after required delivery work. Preserve
the disabled service, unrelated listeners, driver and primary CUDA12.6.

The exactv0.8.30 CUDA environment tag policy is present; required reviewers
remain intact. After the tag exists, add only its exact selected workflow to
group3 and pass the marker-free route check before protected GPU approval.
Pages already permits main. Authorize and verify actual0.8.30 public docs only
after the corresponding delivery, retaining historical download evidence.
Converge provisional changelog/support wording in normal finalization without
retagging or repeating unchanged runtime qualification.

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

## Remaining entry conditions

Accept Slice75's actual qualification, bind its exact reviewed candidate, and
verify the specified tag/group/environment sequence before GPU approval.
Resolve any missing required smoke or route before publication. Seq-414 already
directs normal release completion; no fresh generic production approval is open.
Any selected matching0.8.30 Tegra delivery requires its own applicable75 package
proof and protected Pages route; it cannot inherit generic ARM CPU evidence.
Orin transfers remain limited to this session underseq-416. This plan itself
does not assert that qualification, publication or public readback has finished.
