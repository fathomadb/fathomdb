# Slice10 implementation evidence

Source owner: same Sol-high slice owner, branch `llm/0.8.29-s10-preparation`,
checkout `/home/coreyt/projects/fathomdb-worktrees/0.8.29-s10-preparation`.
Parent-created bootstrap/base `ad102de7630141762aee28123fac57cbef9f61d5`;
entry strict worktree/closed9/32GiB preflight PASS with zero warnings.
Parent owns integration and10 closure; state remains IN_PROGRESS, acceptance
NOT_RUN, release_readiness NOT_READY and publication HELD. Fresh independent
implementation review is still required. This is preparation evidence only.

## Observed RED and separate GREEN commits

Each RED commit body records the actual failing assertion; no missing-package,
missing-helper or permission failure counts as behavioral RED. Human-intent
oracles stay fixed after RED; the heavy-bootstrap editable expectation was
intentionally superseded in the reviewed initial RED, before source changes.

| Behavior | RED | GREEN | Actual intended failure |
| --- | --- | --- | --- |
| Noneditable bootstrap |40cc312da |2006c3165 | Existing caller completed editable pip install. |
| Timestamp-only reuse |e7d9cd0c8 |0f1825a8f | Existing timestamp caused candidate wheel build to be skipped. |
| Missing owned env ordering |e35c3ed80 |44216ece1 | Existing caller reached Cargo before refusing absent env. |
| Exact Pyright pin plus notice |af68d1d68 |a4841a083 | Exact1.1.410 with informational banner was incorrectly rejected. |
| Actual interpreter ownership |418869252 |2f8f684bd | Foreign sys.prefix launcher reached package installation. |
| Exporter discovery |47e802971 |b49b31065 | Explicit tracked exact exporter omitted. |
| Valid inline hashes |e45857999 |19ddfb60a | Exact certifi pair silently dropped. |
| Exporter dispatch/completeness |3640e68c8 |eb51f3f36 | Explicit28-pin boundary map empty. |
| Malformed hashes/CLI |a0c919d46 |659090140 | Seven unsupported inputs did not raise; CLI0 instead of3. |
| Purpose tiers/exclusions |744d32cdb |f029f0b25 | Current real experiment manifest untiered. |
| Plain marker compatibility |6c2336936 |671d43304 | Valid PEP508 marker string containing --hash incorrectly refused. |
| Exact protected.29 route |45b1e89a7 |13637c6f4 | Authorized absent.29 tag refused after.28 controls passed. |
| DOMPurify XHTML hardening |7dda4e6ba |c484a461e | Sanitized output retained SMIL href rewrite attribute. |
| TC37 repair guidance |e3695902a |14582439e | Existing diagnostic lost required supported bootstrap pointer. |
| Pytest child candidate |97b5a3385 |e5e43ae17 | Existing child cancellation test imported production native without private rendezvous. |
| Accepted runner inventory validator |a7d4cf2d6 |3cd44e6d4 | Accepted5 rejected; missing Windows falsely accepted by old4 literal. |
| Bootstrap venv ordering |92ff7247c |f802bd584 | Both callers mutated foreign venv and invoked installers before refusal. |
| Bootstrap module ordering |2082030ce |85b5e0b0c | Both callers invoked mutators before module refusal; heavy npm wrote foreign root modules. |

Parent independently reproduced five immutable REDs (bootstrap, timestamp,
missing env, Pyright notice, exact tag) in separate clean detached proof
worktrees. All exit1 at the intended assertion, no native build/borrowed env or
timeout. Raw parent audit results: `/tmp/fathomdb-0829-direct/red-audits/results.json`
and corresponding SHA logs. No duplicate owner audit was run. Remaining own
raw RED/GREEN logs are `/tmp/fathomdb-0829-direct/slice-10/`.

Fixture preconditions were corrected before implementation: initial sandbox
mktemp denial and absent TS fixture manifest were not behavioral RED; the
unpushed first RED commit was amended to the genuine assertion. Later fixture
pins were seeded explicitly instead of a pip-version spy mutating them.
Existing assertion outcomes were not weakened. The real Pyright notice and
plain-marker edge each received their own new observed RED→GREEN correction.

## Owned setup and resources

Task root `/mnt/data/tmp2/fathomdb-0829-01a1c5a5-s10`, mode700; source/real
.venv and root/TS/Mermaid modules are checkout-owned. No editable installs,
shared modules, borrowed interpreter/native binding or global tool mutation.
Command-local Node25.9.0 selects the existing immutable exact binary; npm11.12.1
is installed into this task's private prefix. Python3.12.3/pip24.0, Ruff0.15.17,
Pyright1.1.410 and declared developer dependencies are in this checkout's venv.
Root Markdown cli2.23.3 is now authoritative; earlier.23.0 was supplemental only.
Separate survey .venv contains a noneditable source package and its validator.

Resource monitor receipts/logs live under task `receipts/` and `logs/`:
one native/render/full phase, Cargo/CMake jobs2; root32GiB start/20GiB stop,
taskfs250GiB start/50GiB stop,200GiB growth cap; MemAvailable16GiB start/8GiB
stop, sustained swap rate control. No resource stop occurred in completed scoped
runs. Before provisioning18:49UTC: root116634025984B free, taskfs980659458048B,
MemAvailable38368129024B. Standing.28 GPU install, driver/primary toolkit and
other sessions' paths untouched; no CUDA workload, cleanup or venue action.

Initial manual maturin invocation from repository root selected wrong package
metadata (PyInit_fathomdb_py warning); that artifact was not installed or
qualified. Correct helper invokes maturin from src/python, installs the exact
wheel noneditable and validates site-packages native origin/digest. The first
valid receipt was a baseline W0.8.28 setup witness; the finalW0.8.29/pin/tree
receipts below replace it. Test-hooks gate remains unchanged:
its explicit candidate wheel, checkout ownership, nonce and source/native digest
controls own any ignored source-package native replacement.

## Scoped qualification and bounded dependency evidence

Actual loader integration with loader-test-hooks enabled:12 passed, zero failed/
ignored. Actual loader::hub_publish_tests:6 passed. The earlier guessed
materialization_tests filter selected0 and is not coverage evidence. Raw logs:
task logs/loader-qualification.log and loader-hub-publish.log. These use local
HTTP fixtures, not TLS-protocol exploit qualification or real model downloads.

Actual renderer acceptance:21 passed/0 failed across SVG/PNG/PDF/Markdown,
flags/space paths, missing dependencies and wrapper exits. Advisory-specific
sanitizer RED→GREEN blocks requests and never inserts/executes untrusted output.
Puppeteer launch configuration is unchanged. Exact DOMPurify3.4.17 registry
integrity verified by npm ci; no Mermaid/browser/zod version churn.

Fresh2026-11-10 primary review: [Rustls0.23.46 release](https://github.com/rustls/rustls/releases/tag/v/0.23.46),
[handshake advisory](https://github.com/rustls/rustls/security/advisories/GHSA-2mjx-qc3c-rqvc)
and [DOMPurify XHTML advisory](https://github.com/cure53/DOMPurify/security/advisories/GHSA-c6gr-qvv4-5v3f).
Approved Rustls0.23.46/webpki0.103.16 current-lock update changes only those
external nodes. Actual sparse-index records mark both not yanked; licenses are
Rustls Apache-2.0 OR ISC OR MIT and webpki ISC, unchanged families. Locked feature
receipt shows actual provider/config closure; no Brotli/custom session-store/ECH
use is claimed from lock presence. No graph-wide security/license certification,
exporter execution or model/golden replacement occurred.

W0.8.29/E0.8.0 file alignment, existing version fixtures and generated contributor
map are checked through owning scripts; format37/query scaffold unchanged.
Current-authority/package/setup/MCP-rationale corrections follow the exact
reviewed map. Three inherited historical review hrefs were normalized to relative
paths, explicitly annotated; wording/labels/verdicts remain intact. Authoritative
Markdown and shell checks, installed survey/native receipts and final full source
gate are recorded below when actually complete. No scoped pass is full green.

## Final scoped preparation checkpoint

Owned authoritative agent-lint-md (including structure/planning/state/public-doc/
link legs) and tracked shell ratchet checks exit0 after the annotated portable
href correction. Installed noneditable survey CLI exits0 offline with exactly
120 manifests:23 shipped,9 dev-tooling,11 eval-only,44 fixture and33 historical
exclusions. Final isolated suite:37 passed. Existing version/runtime fixtures,
set-version --check-files, generated-public-doc check and13 state views pass.
Actual raw outputs are under task logs/markdown-scoped.log, shell-scoped.log,
survey-final-cli.log, survey-final-tests.log and the version logs.

The source-phase full gate and final source/version/pin native receipts are still
pending at this checkpoint; no preparation closure or release green is claimed.

## Final artifact checkpoint

Clean assembled source1cadab2ea92eca61f1132526be10cb41deec74f3 built the
W0.8.29 production Python wheel and TypeScript/native package successfully with
owned tools. Monitored start/end HEAD/delta match; no resource stop. Installed
Python write/search/close/reopen/search/close/process-exit smoke passed on a real
fresh store, module in this checkout's site-packages, native SHA256
5e80e3eece0e643d384326be7ecbcd88b576d75cfb4dfef41ba3c447acda9e9f.
Receipts: task receipts/final-python-wheel.json, final-ts-build.json and
python-installed-smoke.json; exact wheel identity also in .cache/agent/python-native.json.
The subsequent actual-prefix guard changes only setup/test code and generated
command hashes; native Cargo/Python/TS inputs and versions/pins remain identical.
Final full verification follows the regenerated clean source; no source edits
occur during its run. Final readiness and independent review remain pending.

## Full-run metadata finding

The one strict full invocation at98fcbd546e73402adfcbd0b8fd916a91edccc5b4
exited1 during lint after owned preflight, shell/module lint, full workspace
clippy, rustfmt, migration/platform and release-contract checks passed.
public-doc-truth rejected bootstrap public_docs_target_release0.8.28: a present
active-state field means pending-release docs promotion and must equal that
state's own version. Removing the field uses the checker's existing explicit
latest-published fallback, preserving published.28 docs. No checker, source
contract, readiness or publication gate was weakened. Bootstrap prose is corrected.
Raw failed run and resource receipt remain task logs/full-agent-verify.log and
receipts/full-agent-verify.json. Remaining qualification resumes through affected
lint, typecheck, unchanged strict security and all tests; any resulting evidence
is assembled/resumed, never an uninterrupted full-gate pass.

## Resumed gate checkpoint

On clean8d20a8c0356651408eda68d22ca7cf6886006725, canonical agent-lint,
agent-typecheck and STRICT=1 AC037_LIVE_OPTIONAL=1 agent-security exited0,
as demonstrated by the serial set-e shell entering agent-test afterward.
Raw task logs/resumed-source-gate.log includes all lint/typecheck steps and
security0violations/0blockers/0downgrades, including real ptrace and live-netns
negative controls. Declared platform/ESLint skips remain explicit.

The operational monitor then failed when du raced a removed temporary security
fixture directory. Its original test child was stopped via verified own cwd,
command and process group30287; no gate exit was inferred from monitor failure.
This test attempt is incomplete. Its existing platform-truth assertion exposed
dev/platform-capabilities.json still naming0.8.27 despite published0.8.28.
Only that metadata is corrected; platform list/status and the human oracle stay
unchanged. The failed/incomplete test suite is rerun under repaired supervision;
no second full wrapper or uninterrupted full-green claim follows.

## Frozen collector and TC37 observed RED

Clean b3dfb51745aac79bb36bc1b52598d706fe4c8751 all-tier collector exited1:
registered211/ran211/passed207/failed4/skipped0/excluded0, no resource stop,
unchanged start/end HEAD and empty tracked delta. Full Rust workspace serial
exit0 (1468814ms), TypeScript exit0 (75767ms), hermetic executable N-API gate
exit0 (27078ms), Python candidate receipt exit0. Python suite:1745 passed,
36 skipped,1 failed. Other failures: Jetson accepted-label literal, current
Tegra runbook-version literal and TC37 missing-tool repair diagnostic.

Raw task logs/final-test-suite.log and receipts/final-test-suite.json retain
real exits/resources. Read-only retention saved111 transient suite diagnostics
under task logs/retained-test-spills, including complete Rust/Python/TS outputs;
earlier successful fixture detail was already removed by the canonical runner.
No interrupted or failed attempt becomes uninterrupted full green.

Unchanged existing scripts/tests/test_lint_md_hard_fail_on_missing_linter.sh
provides actual behavioral RED at b3df: source diagnostic still refuses absent
linter but lost its required supported bootstrap repair pointer. Existing test
is already tracked and stays unchanged. No source GREEN precedes this record.
Exact observed diagnostic:

```text
FAIL lint-md-structure: markdownlint-cli2 not found at node_modules/.bin/markdownlint-cli2.
  A missing structural markdown linter must never report a silent pass (TC-37).
  Fix: select the declared Node/npm pins and run npm ci in this checkout.
  Linked worktrees require their own node_modules; do not share a mutable install.
exit=1
FAIL  hard-fail message does not tell the operator how to fix it (scripts/bootstrap.sh)
```

## Pytest child candidate observed RED

Unchanged test_slice85_maintenance.py::test_cancelled_caller_keeps_native_ownership_until_durable_completion
failed at b3df with its actual child return-code assertion, not missing tooling.
Full raw Python output remains task logs/retained-test-spills/
fathomdb-agent-test-python-182330.log and the original PID182330 spill.
Existing test stays unchanged; this record precedes separate caller GREEN.
Full observed child traceback:

```text
E       AssertionError: Traceback (most recent call last):
E           File "<string>", line 35, in <module>
E           File "/usr/lib/python3.12/asyncio/runners.py", line 194, in run
E             return runner.run(main)
E                    ^^^^^^^^^^^^^^^^
E           File "/usr/lib/python3.12/asyncio/runners.py", line 118, in run
E             return self._loop.run_until_complete(task)
E                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
E           File "/usr/lib/python3.12/asyncio/base_events.py", line 687, in run_until_complete
E             return future.result()
E                    ^^^^^^^^^^^^^^^
E           File "<string>", line 11, in main
E         AttributeError: 'builtins.ErasureMaintenance' object has no attribute '_arm_erasure_pause_for_test'
E         
```

Read-only python-child-import-proof.json under the Slice10 /tmp evidence root
confirms default subprocess import resolves to this venv's installed production
module5e80e3ee..., which has no private hook. Scoped own src/python resolves to
the sanctioned source candidate db577917..., matching the passed parent receipt
(candidate_sha=b3df, nonce=agent-test-182330). Proposed caller correction scopes
absolute own-checkout PYTHONPATH only to pytest/children. It changes no test,
conftest, wheel/nonce/digest guard or installed production module; final identity
proof must compare child path/digest/nonce to the newly qualified parent receipt.

## Final source/component qualification

Code1cc5701697eb93ab91e80fe297484cccd482b596 passed the initial affected
qualification; independent review then found P1 ownership ordering. Focused fixes
end at85b5e0b0c2cc29c30ab339a0e662c58851188afe and retain original failures. There was one full
agent-verify invocation (98fc exit1), followed by scoped resumption; no
uninterrupted full-green or new all-suite214 collector is claimed. Original
211-suite collector exited1 with four named failures, all subsequently repaired
and qualified below; three new validator/ownership registrations ran separately. Final
registration is214 total/207fast/7heavy, with exact partition/mutation checks.

| Component / source | Actual exit and receipt |
| --- | --- |
| Full original wrapper98fc |1; task logs/full-agent-verify.log, receipts/full-agent-verify.json. |
| Resumed lint/typecheck/strict security8d20 |0 each before serial shell entered tests; logs/resumed-source-gate.log. Security0 violations/blockers/downgrades, actual ptrace/live-netns controls pass. Interrupted test attempt remains incomplete. |
| Original collector b3df |1; registered211/ran211/passed207/failed4/skipped0/excluded0; logs/final-test-suite.log and receipt. |
| Full Rust workspace b3df |0,1468814ms; complete retained-test-spills/fathomdb-agent-test-rust-182330.log. |
| TS and executable N-API b3df |0 each; TS reports625pass/6skip/0fail of631 under canonical network gating,75.767s suite; hermetic executable gate27.078s. Complete raw retained spills. |
| Python repaired caller e5e | Unchanged failed case1pass, canonical1746pass/36skip/0fail, each exit0. logs/python-child-targeted.log and canonical-python.log, matching monitored receipts. |
| TC37 code1458 |0 unchanged fixture; /tmp Slice10 green-tc37.log. |
| Jetson validator3cd4 |0 original93 controls and new meta-fixture with all5 missing-label negatives; green-jetson-validator.log and green-label-oracle.log. RED/raw at a7d4/red-label-oracle.log. |
| Tegra metadata1cc5 |0 complete existing operator fixture; green-tegra-operator.log. Only current-runbook.27→published.28 assertion changed; all historical.27 mock controls unchanged. |
| Final canonical lint/affected checks1cc5 |0; logs/final-affected-checks.log and receipt. Includes all current lint legs, actions, registration/caller/target controls, generated map, W/E,13 state views, discovery and strict dependency9 preflight. |

Final preflight passes with the sole expected warning that this source branch
has advanced past bootstrap release/ad102 with its own commits. Entry preflight
at ad102 had zero warnings. No ref/ancestry/guard was weakened or force-updated.
AGENT_LONG was actually unset; existing long/measurement/CUDA/platform/network
qualification is not inferred from ordinary tests. Canonical Python reports36
skips; TS reports6 plus explicit network-return notices. Full ignored Rust
reasons/raw output are retained. Lint explicitly skips AArch64 CUDA tegra-pool
and unconfigured ESLint. No owned-venv/native-receipt suite was skipped.

### Candidate versus production identity

Both targeted and canonical Python reruns use the unchanged fresh-wheel/clean
source/owned-venv gate. Child proofs in task receipts/
python-child-targeted-identity.json and python-child-canonical-identity.json
compare exact source module path, SHA256, nonce and candidatee5e43ae17 with the
passed parent receipt. Candidate native db577917860d7c81670976bbbf428cdd8cc2b53232ba200d4e8c8e98afea77b7
is this checkout's src/python/fathomdb/_fathomdb.abi3.so; canonical nonce is
s10-canonical-842718-1794343115355312645. No has-hook-only identity claim,
conftest edit, installed-production rebind, editable install or manual native copy.

Separate default installed-production write/search/close/reopen/search/close/exit
smoke passed after the caller fix (receipts/python-production-after-child-fix.json):
W0.8.29 module remains this .venv's site-packages, SHA256
5e80e3eece0e643d384326be7ecbcd88b576d75cfb4dfef41ba3c447acda9e9f.
TS production src/ts/fathomdb.linux-x64-gnu.node remains SHA256
93fa5c015aa7833c526394ce197020b154f393975f2944de4af6879d8630a08e,
matching the earlier production build/lifecycle receipt. Final tool/native facts
confirm both; no artifact rebuild was needed. Git diff of Cargo.toml/Cargo.lock
and all src/rust/src/python/src/ts inputs from1cad to final code exits0.
These are local Linux developer artifacts, not release-matrix/registry proof.

### Final resource and ownership facts

Task receipts/final-tool-native-facts.json records real nonsymlink checkout .venv, distinct
root/TS/Mermaid modules, separate survey venv and mode700 task root, all uid1000.
Immutable Node25.9.0 is /home/coreyt/.nvm/versions/node/v25.9.0/bin/node;
npm11.12.1 is task tools/npm/node_modules/.bin/npm. Python3.12.3/pip24.0,
Ruff0.15.17/Pyright1.1.410 and root cli2.23.3 remain owned and exact; Rust/cargo
1.95, actionlint1.7.12, shellcheck0.11, lychee0.24.2 and strace6.8 are verified.
Primary CUDA readlink remains /usr/local/cuda-12.6; no GPU/driver/ACL action.

Original completed collector:96 samples, sampled minimum free root108153647104B,
taskfs922751963136B, MemAvailable37800558592B; sampled task peak57674990155B,
growth49686653532B. No resource stop. Every completed phase stayed below total
200GiB task storage; final supervisor enforces the cumulative own-root cap.
Earlier du/temporary-directory race and incomplete attempt remain recorded,
not silently converted into a passed measurement. No other session path cleanup.

### Scope and remaining boundary

T10-1/2: actual owned resources, parent bootstrap/ad102 and strict9 ancestry.
T10-3: common noneditable helper, caller RED/GREEN and exact native identities.
T10-4: contained Rustls/webpki/DOMPurify nodes and affected qualification above.
T10-5: installed survey37pass, complete120 path disposition and exact28 pins.
T10-6/7/8: bounded current docs/validator maintenance, protected exact.29 route
with all negatives/HELD, W0.8.29/E0.8.0/format37 and query scaffold preserved.
T10-9: assembled receipts ready for fresh independent final review, parent
integration and actual closure; state10 remains IN_PROGRESS/acceptance NOT_RUN.
Readiness NOT_READY/publication HELD and genuine110/120/optional150 future gates
remain; those future choices did not block this approved preparation.

Final metadata-only scoped checks and exact final branch SHA are supplied in
task receipts/final-metadata-checks.json and the final /tmp review registers.
No authoritative final implementation verdict or parent closure is asserted here.

## Independent P1 ownership correction and recheck

Fresh Astra-medium review of1cc5 found P1: caller-side venv creation and heavy
npmci preceded ownership guards. Independent actual-caller spy proof remains
/tmp/s10-astra-final-review/bootstrap-ownership-result.json. Bounded correction
design/selector-assertion placement PASS is retained at
/tmp/fathomdb-0829-direct/slice-10/ownership-correction-astra-review.md.

Own venv RED92ff7247c observes both full/heavy foreign marker writes and prior
installer calls. GREENf802bd584 adds nonmutating --check-ownership before all
installers/hooks/npm; creation is solely common --provision. The existing
selector assertion moves only to the real common-helper call, preserving its
human intent and all cold/restored and other behavioral controls. Separate
module RED2082030ce after the venv-only fix observes both callers' premature
mutators and heavy npm's foreign marker. GREEN85b5e0b0c adds early module
symlink/escape checks. Both venv cases and all4 caller/root/TS module cases pass
without mutator calls or external writes; preserved heavy14/setup5 controls pass.
Raw red/green-bootstrap-order, heavy-final and setup-final logs are under the
Slice10 /tmp evidence root. Real owned-checkout --check-ownership also exits0.

The independent reviewer reproduced both exact immutable REDs and both final
PASS cases without environment borrowing; P1 is resolved. Exact audit results:
/tmp/s10-astra-final-review/correction-audit/results.json. This finding recheck
is not the overall final implementation verdict, which remains parent-gated.
Only affected shell/setup/registration/metadata checks rerun; no Rust/Python/TS
source, lock, native artifact, conftest or nonce gate changed, so completed native
qualification is retained without blanket repetition.

## Selector-placement scoped follow-up

First final scoped attempt at eb8f0bc6944dbf5b1cf9114bd901d288a1812f32
exited1 on an additional obsolete direct-call assertion in creation armC.
Successful preceding shell/setup checks remain valid; exact failed log/receipt:
task logs/first-final-metadata-checks.log and receipts/first-final-metadata-checks.json.
This was not a new interpreter/production defect. Retry discipline reread A2,
full fixture and real memory entries, then inventoried all selector references
under scripts/.github/src. Only heavy (already corrected) and creation armC
were placement assertions; the6 selector cases are behavior-only. Exact
selector-placement-inventory.txt remains under the /tmp evidence root.

Independent Astra approved requiring BOTH actual bootstrap→common --provision
delegation and canonical selection inside that helper. A/B/D and bare-python3
rejection stay unchanged; the complete4-arm creation fixture passes. A local
ShellCheck directive marks the intentionally literal source-text match; no
ratchet or policy changed. No synthetic RED, extra behavior pair or native rerun.
Exact design report: /tmp/fathomdb-0829-direct/slice-10/selector-placement-astra-review.md.
Resume only failed/remaining affected checks, preserving original exits.
