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
valid receipt is a baseline W0.8.28 setup witness; finalW0.8.29/pin/tree receipts
must replace it before final qualification. Test-hooks gate remains unchanged:
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
