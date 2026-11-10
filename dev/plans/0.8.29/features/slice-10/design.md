# Slice 10 preparation design

## Existing architecture and smallest change

State discovery is tracked-Git based. `preflight-release-state.py` requires
`active_ref=refs/heads/release/0.8.29`, real closed dependency SHAs ancestral to
that ref and target HEAD, and tracked state/board/plan. It does not accept prose
closure or a lone untracked JSON. Use [bootstrap proposal](bootstrap-proposal.md)
without changing these guards. Parent owns only serial metadata import; same10
owner writes source only in a fresh worktree cut from that verified commit.

Current bootstrap/bootstrap-heavy/agent-build use editable installs; build also
falls back to system Python and trusts `.cache/agent/python-installed` time.
Introduce one `scripts/setup-python-owned.sh` entry point, used by both bootstrap
variants and agent-build. Modes `--provision` and `--build` separate installing
owned developer tools from building an artifact. Provision creates/reuses only
this checkout's real `.venv`, installs the actual pyproject dev requirement list
with pip, and verifies Python/pip/Ruff/Pyright before native work. Build mode
requires that environment; no fallback/provisioning into another prefix.
Both build a fresh wheel with checkout Python's maturin, `--locked --release`,
explicit `pyo3/extension-module,default-embedder,default-reranker`, exact interpreter
and owned output directory, then install that exact wheel noneditable into the
same venv. Record source SHA plus tree/delta, lock/features, interpreter/ABI,
wheel/module SHA256 and resolved paths. Timestamp sentinel is removed; every
requested build rebuilds, so a stale receipt cannot authorize reuse.

Reject a .venv or module directory resolving outside its owning checkout,
foreign `VIRTUAL_ENV`, missing pip/wrong pins, and installer/build/install failures
before later workload or receipt success. Verify installed imports from outside
`src/python` with source PYTHONPATH cleared; expect venv site-packages. Root/TS
modules come from owned `npm ci`; read-only immutable binary distributions may
be selected by absolute path. New helper must support paths with spaces and
fail with a named actionable error. Resource preflight is executed before it,
not replaced by “build succeeded.”

Bootstrap callers invoke the common helper instead of `pip -e`; agent-build
validates the owned prerequisites before its Rust workload and delegates Python
build, without sentinel/system fallback. Preserve full/heavy CI division and
existing installer-failure/no-unrelated-tooling protections. Full bootstrap's
unrelated host installers are not run wholesale locally. Existing heavy test's
explicit editable-install expectation is intentionally superseded in the RED
commit by this reviewed noneditable contract; retain all its other assertions.
Update affected diagnostics/docs together, including agent-lint-md's obsolete
shared-node_modules symlink recommendation. No shared symlink is offered.

The existing explicit Python test-hooks gate already builds a noneditable exact
candidate wheel and atomically installs its native module into its own ignored
source package with ownership, clean-SHA, nonce and digest receipts. Preserve that
sanctioned gate unchanged; do not manually copy a native library or invoke
maturin develop. Its source-package import path is distinct from installed-wheel
smoke's site-packages path. Full agent-test authorizes the existing fresh gate;
manual scoped unit tests may use its pure helper tests without loading bindings.
TS release and test builds retain separate production/test-hook surfaces and the
hermetic native wrapper; both operate only on the owned checkout and Cargo target.

## Approved manifest and survey work

Use current-lock `cargo update -p rustls --precise 0.23.46`
and only necessary webpki0.103.16 closure. Verify actual resulting graph; stop
if more dependency families require review. Never copy the stale candidate lock.
Update the exact DOMPurify lock node/integrity under existing Mermaid range,
install its owned lock, inspect consumed sanitizer configuration and run real
renderer acceptance. No browser/Mermaid/zod churn. Existing no-sandbox wrapper
configuration is inspected as-is; no sandbox-setting change is approved.

Survey discovery gains only exact `export-requirements.txt`, shared with dispatch
classification to prevent name drift. Retain requirements*.txt and tracked-only
input. Parser extension accepts a requirement followed by one or more whitespace-
separated `--hash=sha256:<64hex>` tokens and existing comments/blank lines; the
standalone current extra-index-url option produces no dependency and is never
executed. Strip only valid hash tokens; unsupported algorithm/length/value or
trailing malformed hash syntax raises named ManifestParseError/CLI3. The plain
parser's unrelated permissiveness remains. Logical continuation/backslash input
is deliberately unsupported here and fails explicitly when hash syntax spans
lines; no current exporter declaration needs continuation support.

The real28 declarations retain exact names/versions/direct origin/dev-tooling
and their original bytes/hashes; index URL is not a component. Add explicit
eval-only prefixes `dev/experiments/`, `experiments/comp_drivers/`,
`experiments/sdk_drivers/`; exact benchmark manifest override; fixture exclusion
`scripts/tests/fixtures/`; historical-evidence exclusion
`dev/plans/0.8.27/features/slice-135/results/`. Retain existing rules and no
catch-all. Exclude before parse/registry calls. Reviewed1's path sets reconcile
119 current recognized+exporter1 to120:23 shipped,9 dev-tooling,11 eval-only,
44 fixture and33 historical exclusions. Counts are dated checks; exact path-set
comparison and loud new gaps are the invariant. Supplemental13 snapshots/renamed
locks stay inventory-only, preserved byte-for-byte. Survey stays informational,
isolated and absent from agent-test/CI, with no pytest settings table.

## Protected exact first-tag route

Extend bootstrap whitelist from exactv0.8.28 to exactv0.8.28 OR v0.8.29, still
requiring marker==requested tag. Add real.29 fixture routing to gh stub, not a
synthetic success that bypasses HTTP status/body checks. Confirmed absent remote
HTTP404 with exact JSON is mandatory; every current failure stays refusal.
Preserve selected restricted GPU group3/name/visibility, trusted main release
workflow, exact environment tag policy, actual nonempty User/Team reviewers and
HELD output. Normal marker-free path remains exact-tag checked. No broadv0.8.x
match, environment reviewer deletion, hook bypass or dispatch. Future authorized
publisher must verify remote peeled SHA, add only exact selected tag ref and pass
normal guard before GPU approval;10 performs no such external changes.

## Contracts, versions and fit

[Register](preparation-register.md) fixes document scope. Correct only present
facts: format37 applicability, actual active selector after bootstrap, current
12-member/53-endpoint profile, existing short content tokens, opt-in weight
fetch, owned setup and current engine-versus-MCP refusal rationale. Public
README release/install versions remain published.28 until160. Keep generated
public content under its generator. No new MCP default/filter support is claimed.

Use set-version --workspace0.8.29 once after approved current-tree lock edits;
retain E0.8.0, format37 and all qualified pins. Canonical same-version AC052
conflict goes through9's explicit allocation; passing file alignment is not
AC052/AC054 whole-release proof. State/master/board use declared JSON views only.

Quality-roadmap adoption is changed-seam failure controls, installed identity and
independent parser/schema expectations. Broader performance/fault/CUDA matrices
remain downstream; no new migration/coverage or threshold program is introduced.
Source/tests/scripts require per-behavior observed RED commits and final stable
full gate; mechanical version/docs-only changes use applicable scoped checks.
