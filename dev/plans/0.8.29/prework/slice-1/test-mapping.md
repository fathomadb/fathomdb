# Slice 1 proposed checks and downstream TDD mapping

Documentation-only1 needs independent completeness/source/link review, not
invented RED tests or dependency/tool installs. No tests/sweeps ran here.
Parent Astra plan review precedes finishing the proposal; a fresh Astra output
review follows; parent import/scoped Markdown validation is a separate action.
The source paths/test names below identify inspected or proposed checks, not
claims of an observed failing run.10 must read existing tests in full again at
its actual baseline before asserting their coverage or adding new behavior.

## Proposal completeness (1)

- Compare unique path sets in [inventory](manifest-inventory.md) against current `git ls-files -z` and the explicit recognized-basename/requirements rule; reproduce119 recognized/79 untiered and14 supplemental rows. Check proposed totals23 shipped,8 dev-tooling,11 eval-only,44 fixture exclusions and33 historical exclusions (119); exporter would add dev-tooling1 for120 executable discovery inputs.
- Reconcile each of the nine full candidate refs/touched files/current delta and every exact pin/export/action group. Verify source pin values/hashes directly; count alone does not establish set completeness or current remote state.
- Resolve relative links at intended repository destinations with staged files taking precedence, then existing root files. Check headings/tables/frontmatter/whitespace using available read-only/scoped tools; report missing authoritative tooling without provisioning it in1.
- Recheck live parent HEAD/status and actual completed Slice0 proposal/review-log. Dated planning/product baselines stay separate; proposal closure does not establish environment readiness.

## Per-behavior10 RED→GREEN design

| Behavior / proposed test placement | Human-intent oracle and required RED signal | GREEN/preservation checks |
| --- | --- | --- |
| Exporter discovery in `tests/test_discovery.py` plus mirrored conftest/design table | A tracked exact exporter path must be returned as PyPI manifest; same untracked on-disk name and unrelated notes must stay absent. Baseline omits tracked exporter; record actual failed assertion before code. | Both injected discovery and survey boundary honor checkout Git input. Existing discovery/source-isolation assertions remain unchanged. No filesystem walk. |
| Exporter dispatch in new/narrow survey integration test | Given discovered exporter, dev-tooling rule, hash-pinned declaration and offline registry source, document must contain expected named/pinned component with direct exporter origin. Isolate dispatch failure from discovery by injected refs if needed; observe missing component before fix. | Same component/constraint/version/tier in emitted output; existing independent CycloneDX validation remains. Do not satisfy test by only changing discovery helper. |
| Inline hash parser in proposed requirements parser tests | Explicit fixture `certifi==2026.7.22 --hash=sha256:<known64hex>` yields that exact pair, two hashes still one declaration; comments/options not components. Existing parser silently returns no requirement. Tests encode explicit expected values, never regenerate golden output. | Exact pin preserved; plain requirements/marker/comment input remains supported. Selected malformed/truncated hash token must produce named parse error/exit3, not disappear. Observe its own failure before strict subset fix. Logical continuation handling gets separate RED only if reviewed into10 scope. |
| Real exporter completeness at survey boundary | Explicit expected set of28 package/version pairs from reviewed source register, no index URL component, direct origins and dev-tooling on all. This must fail on pre-fix behavior; stage/commit before fix. | All28 expected pairs survive; no silent partial parse. Separate readable fixture expectations from implementation's matcher; no implementation-produced oracle. |
| Tier data and exclusions | Existing `test_tier_map_covers_every_discovered_manifest` in `test_tiering.py` checks all discovered paths; newly added current-path expected tiers/exclusions distinguish real experiments from fixtures/history and benchmark from MCP. Baseline fails on untiered inputs; record actual failure. | No catch-all; longest-prefix/order independence, loud genuinely new untiered paths, excluded provenance present but fake/historical components absent before parse/lookup. Preserve historical/fixture source bytes. |
| Informational isolation/status docs | Existing CLI non-CI-wiring test and README/design inspection preserve informational intent, no new gate. This docs-only wording is not a separate behavioral RED. | Survey remains outside agent-test/verify/CI; do not add pytest settings or shared-environment install. |

Read full baseline `test_discovery.py` and implementation for this proposal;
`test_every_tracked_manifest_is_discovered` derives expected names from a mirrored
recognition set, so it alone cannot independently catch exporter omission.
`test_tier_map_covers_every_discovered_manifest` covers recognized inputs only,
so it cannot prove missing-name coverage either. New explicit exporter oracle
is necessary. Baseline helper/test names inspected in `test_tiering.py`,
`test_cli.py` and `test_cyclonedx.py` inform preservation, not an executed suite.
Do not weaken old tests or fabricate RED from missing third-party dependencies;
a behavior RED requires intended assertion failure after owned setup succeeds.
Per-behavior RED commit/stage and failure evidence precede GREEN; source/script
changes receive independent code review and required full final gate once the
phase stabilizes, with scoped checks during iteration.

## Selected and conditional dependency qualification

| Group / receiving owner | Falsifiable proof required after selection, with scope limits |
| --- | --- |
| Approved Rustls/current lock /10 | Minimal current lock closure preserves current workspace/MCP/E/SQLite/fork pins and cudarc lock-set guard; selected artifact features/provider/config documented. Existing downloader hash, cache/atomic publication, retries and opt-in egress behavior survive; TLS handshake/download smoke uses approved isolated inputs/venue, no arbitrary user query egress. New product behavior correction needs its own RED; a mechanical lock bump uses the exception without relabelling passing tests RED. |
| Approved DOMPurify/Mermaid /10 | Verify exact target/integrity and actual selected sanitizer configuration. Existing `dev/tools/mermaid/test-render-mermaid.sh` drives real SVG/PNG/PDF/Markdown and wrapper exits; supplement meaningful hostile-input sanitization checks if current suite does not grade advisory semantics. Do not execute malicious diagrams in1, remove sandbox protections or merge browser/zod churn. |
| Conditional Markdown /next tool owner | Guarded formatter AST neutrality/known corruption regressions and `scripts/agent-lint-md.sh`/local links under owned pins. Current read-only source/Markdown changes need scoped checks; any executable fixer behavior change receives actual RED then final full source gate. |
| Conditional SQLite/vector /selected owner | Real DB open/vector rows, typed unavailable path, deletion/eligibility invariants, compile options/page-cache, Windows verbatim-path WAL and installed Rust consumer tests. No mocks or migration campaign. Preserve accepted AC thresholds and report unsupported venues. |
| Conditional ABI/fork/provider /140 then110/120/130 or SDK owner | Fork-local qualified repair first, exact registry/source metadata and closure then installed SDK/native parity and declared artifact matrices. Driverless CPU, explicit forced-CUDA typed failure and exact digest/GPU witness as affected. Broader .30 work and unselected majors do not gate .29. |
| Deferred exporter /future exporter owner | Before any run assess PyTorch advisory/input serialization and actual snapshot bytes; local-only path is insufficient safety proof. If selected, independently verify official hashes/28-pin resolver compatibility in owned CPython3.12 Linux x64 env, cold/warm export and two-run byte stability against existing SHA; no autonomous model/golden replacement. Missing exact model/tool inputs block export proof. |
| Tool/action pins /7 and10 | Exact version/commit provenance and supported consuming-job checks, actionlint for workflow syntax, named wrong-tool negative controls if guards change. Dry-run-skipped publishing remains unmeasured; no publication to test a tool. |

10 must resolve checkout-owned tool preflight/capacity before any invocation;
never borrow Memex/another checkout's environment or run mutable sweeps to make
prework appear complete. Survey suite's isolated third-party dependencies and
independent schema validator are prerequisites; no skip-based green. No advisory,
license/yank or security-clean conclusion follows simply from passing tests.
