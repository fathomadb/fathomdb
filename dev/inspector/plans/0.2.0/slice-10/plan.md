---
title: Inspector 0.2.0 Slice 10 — conditional approved-item implementation
date: 2026-11-19
status: APPROVED
execution: COMPLETE
desc: Execute only individually approved Slice 9 items within the adopted Inspector boundary
---

# Slice 10 — implement the approved set

Execution commissioned separately on 2026-11-19; [current status](status.md)
records completed owned work and the owner-approved external metadata correction. The ratified planning text below
records authority and original not-yet-executed conditions; it does not override
the later commission or imply a new ratification gate.

## Goal and authority

Deliver exactly the individual items the owner approves at Slice 9, with their
accepted bounds, tests, ownership and rollback. **USER final response 3 on 2026-11-19 ratified S10-DRAFT-2 at 2604013b736ad655ac5dfa0aa32c1b67208ec63b: INCLUDE 001/002/004–008/010; POSTPONE 003/011–014; 009 remains mandatory proof in existing 40, outside 10. Slice 10 was separately commissioned; [current execution](status.md) owns outcomes.**
The [actual final ruling](../slice-9/plan.md#final-human-ratification--response-3)
approves only that exact set and bounds. Implementation requires a separate
Slice 10 commission; do not re-ask the settled item/scope/final-ratification choices
or infer approval for other proposals from Slices 0–8.
The [adopted scope](../scope.md), [master plan](../plan.md),
[shared workflow](../slice-workflow.md) and actual Slice 9 ruling govern.
Features begin at 20; this slice cannot absorb their implementation by calling
it prerequisite cleanup. Source inspection outside Inspector supplies evidence,
not implementation authority over Engine, SDK, MCP, schema, ranking or pins.

Execution-draft source is release branch `inspector/dev/0.2.0` at
`2604013b736ad655ac5dfa0aa32c1b67208ec63b`, preserving planning HEAD
`aef5b2eac19dbcf5850713081ccd427896909eea` and Engine ancestry
`6edc1155f039f87cd0a37660cb27ae4373bbce71`. Original draft baseline 609dd85c41
is historical. Packages remain 0.8.28/format 37, Inspector unpublished 0.1.0.
Slice 0 checkout creation and final plan ratification are done; no Slice 10
implementation or installation has occurred.

## Ratified S10-DRAFT-2 — selected set, not executed

Actual owner responses 1/2 on 2026-11-19 selected S9-004–007 and 010; exact quotes/rationale
is in [Slice 9 decisions](../slice-9/plan.md#actual-human-responses). Existing
release checkout is branch inspector/dev/0.2.0 at source ancestry aef5b2eac,
with actual reviewed prework commits 7b40cff3e and 090719f80. No 10 execution is
commissioned by this plan update. Before execution recheck current HEAD and only
selected item changes, preserving unchanged Engine 6edc1155f ancestry.

| Selected item / exact allowed documents | Concrete change and acceptance | Checks / failure handling |
| --- | --- | --- |
| S9-004: dev/inspector/{needs,requirements,acceptance}.md; design/README.md; reviews/README.md; Inspector product-area rows of dev/DOC-INDEX.md | Apply Slice 3/Slice 6 exact current-pointer/owner wording: current 0.2.0 plan, comparison 60/final 70; preserve historical links/anchors/status/evidence and all criteria. Correct mislabeled v0.2.0 link to old plan. | Inspect each modified link/current label against master; diff criteria/history unchanged; Markdown and applicable traceability/lifecycle checks. Revert only this item's owned commit if wording loses an obligation. |
| S9-005: dev/inspector/design/high-level-design.md | Replace current missing-snippet/Caller/multiwindow capability assertions with source-qualified public-route/fetch/write/head-only facts and mandatory cache prerequisite and response 2 single-head-window boundary from Slice 4/9. Preserve architecture responsibility and positive requirements; no new model/query implementation. | Reopen actual public methods/cfg, loader, projection_commit/vector_spans; no source/API/criterion change. Review exact replacement diff; revert only affected prose if inaccurate. |
| S9-006: dev/inspector/plans/0.1/verification-plan.md owner/stage cells and current allocation prose | Match master's 40 unique accountable rows; preserve methods/fixtures/criteria/executed evidence verbatim. M28/M40 comparison 60; aggregate final 70; local contributions explicit. Historical owner labels remain in historical evidence, clearly qualified. | Read-only parse 40 rows/no missing/duplicates, compare original method/evidence cells unchanged, scoped document checks. Restore only cell edits if correspondence fails. |
| S9-007: same single verification-plan.md evidence-method notes | Distinguish synthetic pause from active native interruption, bootstrap/SIGTERM from installed all-view/native window close, sampled RSS from peak; retain real queued-positive/all-eight-flags/fresh-read regression and all required new-view cases. | Check whole relevant test files; existing recorded outcomes unchanged. No test/gate added or run merely for prose. Restore only method text if it implies unsupported pass. |

Order 004→005→006→007; 004 and 006 share numbering facts, 005 supplies route truth
for 007. Docs-only: no fake RED/GREEN or full runtime campaign. Fresh Sol-high
draft/design review and Terra correspondence verification passed at the pinned
reviewed snapshot before actual final human ratification. Implementation later
uses source/link/Markdown checks,
normal hooks and one concise result. No Engine/interface/pin edit is part of these items. Public roadmap changes
are only response 2 scope correspondence below; setup/CI additions remain
selected by explicit final batch ratification; earlier recommendation provenance
is retained in Slice 9.

## S9-010 exact scope and acceptance amendment draft

The owner's actual response 2 narrows 0.2.0 to supported single head windows and
defers within-source multiple-window positive. [Exact replacement wording and
method/roadmap consequences](change-plan.md) are an approved local replacement proposal, still unapplied; use them in 10
after a separate execution commission. Final ratification is complete. This is a real scope reduction, not
an unavailable-pass reinterpretation. Cached CPU semantic/hybrid positive and
all other criteria remain mandatory. No Engine writer/admission/schema change.

## Ratified routine setup — not executed

Final batch ratification selects 001. Use this checkout's own
`.venv`; install native bindings through non-editable PEP517 wheel packaging,
with pinned developer tools and root npm lock. Selected commands, **not run**:

```sh
python3.12 -m venv .venv
CARGO_TARGET_DIR="$PWD/target" .venv/bin/python -m pip install --config-settings='build-args=--locked --offline' './src/python[dev]'
npm exec --yes --package=npm@11.12.1 -- npm ci
.venv/bin/python -m pip --version
.venv/bin/python -c 'import sys,fathomdb._fathomdb; print(sys.prefix); print(fathomdb._fathomdb.__file__)'
scripts/tests/test_dev_environment_tools.sh
./scripts/agent-lint-md.sh
```

The locked/offline Cargo build arguments use the existing installed Maturin
PEP517 `build-args` mechanism; [official configuration](https://www.maturin.rs/config.html)
also documents locked/frozen Cargo controls. Verify the actual selected
`maturin>=1.9,<2` backend supports those arguments before installation. Retain
pyproject's declared shipped features for **setup/import smoke only**, without
test hooks. Package registries are
allowed for Python/npm tools; Rust cache/lock misses stop with an actionable
failure instead of resolving or updating pins. No editable install, maturin
develop, shared native copy, model initialization or sibling venv. Import paths
must belong to this checkout's own venv/site-packages for this setup/import
smoke; no pth rebinding. This is distinct from the unchanged required final
source verifier: `agent-test.sh` deliberately sets its sanctioned own-checkout
`FATHOMDB_TESTS_ALLOW_REBUILD=1` and nonce-bound receipt. Existing conftest then
builds a non-editable current-candidate test-hooks wheel in disposable scratch
and installs its extension into this checkout's ignored source package; pytest
prepends that source package under its existing config. Preserve those ownership/
receipt checks and the exact normal gate, including its isolated test-only
artifact/import mode. Do not disable that path, use editable/shared installs or
manual cargo-build/copy. Setup-smoke evidence and later fixture-gate evidence
remain separate. Production Inspector keeps test hooks/fixture controls excluded;
a test-only provider positive cannot establish supported model serving.

Ruff 0.15.17/Pyright 1.1.410 come from existing dev extras. Current actionlint
1.7.12/shellcheck 0.11.0 already match observed pin expectations; verify tracked
pins rather than download installers unnecessarily. Root npm ci supplies local
Markdown 0.23.3/js-yaml; it does not reuse primary 0.23.0 or install the TS SDK.
Node 26 is separate from that SDK's Node25 contract; no SDK declaration/runtime
change. Normal pinned-tool preflight and local Markdown wrapper must pass
before source gates. Record actual tool versions and cache/install effects.
Failure stops the affected gate; preserve diagnostics, do not borrow tools or
weaken pins. Restore only task-owned partial setup when authorized, never shared
caches. No installation occurs during 9.

Selected 002 is bounded local executor-route selection in 10 only; actual
preparation and qualification stay 70: inspect existing suitable local 22.04 builder/
container/VM availability, name exact pinned route and artifact test executor.
Docker/Podman command presence is insufficient image/sandbox qualification.
No paid provisioning, host/driver changes, image acquisition or environment
mutation follows without an explicit concrete approved route. Required ABI and
installed all-view offline close remain 70 obligations, not a reduced target.
No code/build campaign to manufacture evidence during 9.

## S9-008 ratified narrow Inspector CI integration

Allowed future files only .github/workflows/ci.yml and its existing
scripts/tests/test_ci_proportional_routing.py fixture (plus directly required
same-purpose documentation). Add an Inspector changed-path output/filter;
route normal-mode eligible Inspector Rust/assets/package changes to the
existing Ubuntu verify job,
in addition to existing verify-fast. Do not label them general rust changes and
thereby trigger unrelated cross-platform/CUDA/release jobs. Markdown-only
changes keep existing docs-only routing. Extend existing fixture expectations
for Inspector Rust, assets, package script, Inspector Markdown, Engine-only and
manual release-candidate cases. Explicitly retain and fixture-test trusted
`ci_mode=lite` suppression of normal verify, untrusted-marker behavior and the
existing manual/release-candidate or CI-workflow overrides. No new job framework, required-check aggregator,
merge queue, global SDK runtime update or publication.

Acceptance: a normal-mode eligible Inspector-only source/assets/package
change no longer misses the intended Rust/source verification signal. Existing
trusted lite suppression and manual/release overrides retain their current
behavior; do not promise verify for every event. Other selectors/jobs remain
unchanged. This does not claim browser/native/installed coverage from CI.
Write/run actual failing routing fixture first and commit RED, then minimal
GREEN workflow/fixture fix, actionlint and focused routing suite. Independent
Sol-high code review/Terra execution; final required source gate once on clean
code phase. A failure stops this item; restore only its owned diff/commit, no
shared state reset. The earlier actor was agent recommendation; USER final batch response 3 now
selects this exact narrow shared-CI scope. No earlier individual vote is invented.

Final batch ratification postpones 003/011–014; no Inspector10 remedy.
009 stays a bounded proof obligation in existing 40, mandatory positive unqualified,
no Engine commission.010's single-head-window amendment is explicitly selected;
there is no remaining multiwindow-positive blocker after its approved application. Retained source/feature blockers and normal Markdown-wrapper
limit remain explicit. These selections and limits are settled by actual final response 3; source
implementation remains unstarted.

## Inputs and prerequisites

- Read Slices 0–8 reports and Slice 9 consolidation, risks and actual HITL decision, including rejected, postponed and mandatory unresolved items. A recommendation or review verdict is not owner approval.
- Use [needs](../../../needs.md), [requirements](../../../requirements.md), [acceptance](../../../acceptance.md), [HLD](../../../design/high-level-design.md) and the master plan's current allocation. Historical Slice 20 ACTIVE and old integration-60 remain history; current feature owners are 20/30/40/50/60, final integrated qualification is 70.
- Re-read applicable `AGENTS.md` and actual memory entries: `dont-be-distracted-by-ceremony.md`, `verify-design-against-code-not-just-architecture.md`, `tdd-red-must-fail-per-finding.md`, `keep-measurement-tools-in-repo.md` and `full-gate-run-frequency.md` from the indexed external memory store.
- Source-backed seams checked: [worker requests](../../../../../src/inspector/src/worker.rs) implement browse/detail/chunk/tree/associations/export, not lexical/vector/graph operations; [Cargo manifest](../../../../../src/inspector/Cargo.toml) has default features off and package 0.1.0; [qualification tools](../../../../../src/inspector/tools/README.md) distinguish ordinary, ignored/native, scale and installed checks.
- Revalidate the HLD against [Engine admission](../../../../../src/rust/crates/fathomdb-engine/src/open.rs) and [accepted provider policy](../../../../adr/ADR-0.8.28-vector-compatibility-open-policy.md): production supports Default/None; Caller belongs to contained test/evaluation routes. A missing supported cache-only serving route remains a 9/40 prerequisite; neither impossibility nor support follows from the stale HLD helper proposal.
- Slice 0 owns formal environment and CPU-only CUDA-policy review; the [accepted CUDA policy](../../../../adr/ADR-0.8.28-cuda-version-support-policy.md) does not authorize driver/toolkit/support changes here. Carry its disposition forward.

## Existing release checkout

The [owner decision](../slice-0/checkout-decision.md), 2026-11-19, assigns initial
creation **and entry** of `inspector/dev/0.2.0` at
`/home/coreyt/projects/fathomdb-worktrees/inspector-0.2.0` to Slice 0; that checkout creation/entry is completed and recorded in its
actual status.
Reports for 0–9 use that release checkout. Slice 10 reuses it sequentially and
owns no initial bootstrap action, even when its approved item set is empty.
Verify actual branch, HEAD, working directory, cleanliness and ownership before
work; preserve approved final planning ancestry, Inspector port/scope/plans and
Engine base `6edc1155f`. Report a missing or divergent checkout rather than
silently creating, replacing or resetting one. Other environment/infra changes
are bounded by the actual Slice 9 rulings and completed final ratification.
No checkout or environment change is performed by recording this decision.

## Preparation and individual item contracts

Enumerate changes since this draft: plans/rulings, source and tool contracts,
feature allocation, environment, and each allocated proposal. Mark every item
keep, adjust, reject or already-done with concrete evidence. Already-done needs
proof against its accepted outcome, not a duplicate commit. Materially changed
scope or acceptance returns to the owner; a reviewer cannot expand authorization.

Instantiate the following fields **for each actually approved item** in this plan
or its existing Slice 9 item record. Do not create a parallel acceptance register.

| Required field | Concrete completion condition |
| --- | --- |
| Decision and bounds | Stable Slice 9 item reference, exact owner ruling/date, approved outcome, allowed files/commands, exclusions and any conditions. No inferred approval from time or a default choice. |
| Ownership/dependency | Named Inspector implementation owner; affected feature owner; predecessor items and external blockers. Work outside adopted scope is held for separately authorized work. |
| Acceptance | Existing INS/V01/X/fixture IDs and independently observable before/after behavior, or exact documentary/configuration result. Include required positive, negative and boundary cases; no unavailable substitute for a mandatory positive. |
| Design | Actual affected source/API/configuration, public versus private/test-only seam, blast radius, alternatives and accepted change. No predetermined refactor or dependency bump. |
| Verification | Exact targeted command/fixture/manual task, expected independent facts, environment and relevant final gate. Name what a pass does and does not establish. |
| Rollback | Previous owned file/configuration state or item commit boundary, dependent items needing reversal, verification after reversal and residual effects. No data migration or destructive shared-cache cleanup. |

For example, an **approved** documentation correction must replace a specific
stale claim and resolve its cited source/link; an approved Inspector behavioral
fix needs its own failing assertion then passing behavior; an approved local
setup action needs the exact permitted installation/configuration and restoration
boundary. These are verification patterns, not a selected bundle. Public dependency
pin upgrades and external architecture changes remain excluded by current scope.

## Local draft needs, requirements, acceptance and design delta

| Existing contract | Local draft treatment / observable check | Owner |
| --- | --- | --- |
| UN-09/10; INS-X01/X03/X07/X09/X10 | No new need justified by the unknown set. Any approved setup or behavioral item must preserve bounded work, disclosure, offline/local behavior and finite close, checked at affected seams. | 10 only for its approved item; features retain their cases |
| INS-01–09; V01-N01–N07/S01–S04; F/T obligations | Map each selected item to existing criteria and current master ownership; rejected/postponed proposals cannot erase mandatory feature requirements. | 9 decision; 10 implementation; 20–70 qualification |
| INS-05.7–05.10/M40 | No comparison implementation here; preserve Slice 60's actual-observation design/verification responsibility. | 60; 70 integration |
| HLD and consuming/contributor documentation | Conditional source-grounded amendments only for approved items. Draft any changed contract locally, obtain required approval, then update the owning Inspector document once. | 10 for approved amendments |

Do not edit the locked global acceptance register or invent canonical IDs.
The generic executor creates no additional need/criterion. The specific
owner-approved S9-010 direction **does amend F-07 and related scope/method
wording**, exactly as approved in change-plan.md. Final ratification is complete;
the canonical application still waits for separately commissioned Slice 10. No other acceptance reduction or
new criterion follows from creating this plan. The locked global register is
unchanged; unresolved external routes stay explicit dependencies.

## Execution, review and verification

1. Commission one direct Slice 10 agent as sole writer in the selected release checkout; no Steward or Orchestrator. Reuse it sequentially. Extra worktrees are only for necessary concurrent writers and are cut from the correct approved release base.
2. Freeze the approved item set and dependency order in the concise execution status. If an item is inadequately specified, progress independent approved items while the missing ruling remains pending; do not guess or silently substitute work.
3. For actual requirements/design changes, obtain a fresh direct `gpt-6.1-sol` high design review against source and acceptance before affected implementation. Resolve findings within approved bounds. Prose-only work receives content/traceability review; a draft recommendation is not executed to satisfy a generic prompt.
4. For each approved behavior change, write/run its focused failing RED test and commit with the actual failing assertion, then separate GREEN implementation and refactor. Tests do not change to mask wrong behavior; use real databases, no generated oracles/goldens. Mechanical version/rename and docs-only changes follow applicable exceptions, not artificial runtime RED/GREEN.
5. Apply approved docs/configuration/environment changes only at their named paths with their meaningful checks. Never borrow another checkout's Python environment. A missing owned `.venv` fails preflight; it does not justify proceeding under system Python. If setup was not approved, record the blocker rather than expanding scope.
6. Obtain fresh direct Sol-high code review for substantive code/executable changes, plus direct Terra verification of the approved outputs and affected behavior. Terra reports actual executed versus blocked/skipped checks and evaluates required positive cases. Human verification remains human where required; agent review cannot stand in for owner/HITL or unseen-user tasks.
7. Run scoped validators while correcting findings. Docs use `scripts/agent-lint-md.sh`, public docs also `scripts/agent-lint-docs.sh`; state changes use `scripts/check-release-state-views.sh`. Code/test/executable-script work receives the required final `scripts/agent-verify.sh` once on the final clean tree, with only failed/affected suites rerun for narrow fixes. Preserve compiler diagnostics unaltered. Do not run full runtime regressions for prose-only/no-op work or claim scoped checks are full green.
8. Commit only approved item changes and required tests/docs/tools. Reusable measurement tools belong in the repo, indexed with commands/parameters; raw logs may remain task-owned scratch. Integrate verified slice-owned extra worktrees/branches, then clean only those; preserve the release checkout and unrelated work. No bypass hooks, force push, tag or publication.

Use the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
for selected behavior: real DB independent numeric/adjacency/lexical facts,
property invariants for changed round-trip/projection layers, and targeted
fault/resource/cancel plus browser/keyboard checks in the blast radius. The
working format-37 browser and queued-membership regression are useful baseline
evidence, not justification for a new engine fuzzing/global MC/DC/benchmark
campaign. Those remain deferred without separate scope and authority.

## Closure, no-op and handoff

On execution create only a short `status.md`: actual ruling/set and existing
release-checkout baseline, assigned item outcomes/commits, checks and blockers, next feature owner.
No execution status is created while drafting. COMPLETE means all approved items
and their checks are complete; a failed approved item cannot be renamed deferred
without an owner ruling. A genuinely approved empty set is a real no-op for
proposed product/environment changes, with only concise
status/handoff. It needs no invented source commit, test run or review ceremony.
Mandatory feature prerequisites, missing positive routes, owned environment
preflight and final installed/Linux/browser/human qualification remain blockers
for their owning slices; empty-set closure does not waive them. Hand 20 a precise
list of accepted facts, unresolved dependencies and current allocation, not a
runtime-green release claim. Reverse a failing item through its bounded normal
revert/restoration, in reverse dependency order, then check its blast radius;
never reset shared work or silently reverse an owner's accepted scope.

## Future execution prompt

The following prompt is **conditional**: “implement” means only the individually
approved Slice 9 set within this plan's bounds, reusing the checkout created
and entered by Slice 0 under the owner ruling. With an empty set it means the real no-op above; with no actual
Slice 9 ruling it authorizes no proposed changes. No generic instruction below
supersedes these conditions, scope exclusions or required human gates.

```text
Work on and complete Slice 10 for Inspector inspector/dev/0.2.0 (and in a worktree).

Ensure the slice has planned to do any needed work regarding requirements+acceptance criteria, design, implementation, and testing; each should be slice-complete and correct. The slice might have slice-related "change plan" documentation for guidance. The slice's plan should:
- identify and enumerate changes since slice's plan was written (review drafted slice plan, review in-repo related work done, review assigned feature/functions, review allocated draft items).
- Evaluate the enumerated output, and if needed, update/approve/reject/adjust the drafted plan, ensure it is complete but not overbuilt.
- add/adjust needs, requirements, acceptance criteria.
- write/update design (design additions and changes should be reviewed by a design review subagent (gpt-6.1-sol-high).
- The plan should then describe implementation with TDD (RED/GREEN method) and code review subagent (gpt-6.1-sol-high).
- Use a subagent (terra) to test/verify.
- Write Slice status record. Clean-up after itself: While a wt or branch should not be necessary for a slice, if a new worktree or branch was used, merge back and clean-up the worktree.

Keep ceremony to a minimum. Stay focused on the work-at-hand. When resolving a specific finding, do not run full regressions unless the changes are comprehensive enough to warrant the overhead. Generally, solve for the work outlined in the slice; check for impacts in the blast radius; be pragmatic. (do not use Steward or Orchestrator.)

After the planning for requirements+acceptance criteria, design, and implementation is planned: implement the slice according to the plan.
```
