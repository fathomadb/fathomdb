# Slice 1 dependency proposal and bounded survey design

## Current substrate and boundaries

Discovery already uses invoking checkout `git ls-files -z`; README wording
“tracked on main” is stale. Current119-file discovery has79 untiered rows and
omits exporter. The [inventory](manifest-inventory.md) accounts for them and
supplemental historical/renamed inputs without rewriting those artifacts.
Pin posture and current source ownership are in [pins](pin-register.md), with
selected advisory/configuration evidence in [evidence](evidence-register.md).
Format37 is unchanged; no DB migration, new provider/query egress, ABI/runtime
floor change or default selection is implied. Keep the query scaffold.

## Proposed10 source changes

1. In `scripts/sbom-survey/sbom_survey/discovery.py`, recognize exact basename `export-requirements.txt` as PyPI manifest in the explicit table, retaining `requirements*.txt` and tracked-only discovery. Update its mirrored `tests/conftest.py` recognition set and the design-of-record table together. Do not match all `*.txt` or infer input by content.
2. In `scripts/sbom-survey/sbom_survey/survey.py`, route that same recognized basename to `parse_requirements`; existing `name.startswith("requirements")` alone would still drop it. Share the recognized requirements classification where practical so discovery/dispatch cannot diverge. Existing unrelated parser branches stay intact.
3. In `parse/python.py`, support the current exporter subset: requirement plus one or more inline `--hash=sha256:<64 hex>` tokens (and logical continuation lines if added in the reviewed10 design), comments/blank lines and the standalone `--extra-index-url` option. Strip only valid supported hash tokens before normal requirement parsing; never execute options, fetch indexes or treat options as dependencies. Unsupported/malformed hash syntax in a requirement must raise named `ManifestParseError`/exit3 rather than silently discard the dependency. This extension is bounded to declarative inventory, not a general pip interpreter. Current plain requirements parsing remains compatible; broader parser strictness needs a separate reviewed behavior change.
4. Exact pins remain declared requirement + pinned version. Hashes attest future package installation, not a graph lock or model identity proof. Preserve source file hashes/values; survey reads them and records dependency identity without editing them. The28 exporter declarations must all appear with their exact version, direct origin and dev-tooling tier; the PyTorch index line must not become a component.
5. In `scripts/sbom-survey/tiers.toml`, add the exact proposed prefixes/exclusions and benchmark override in the inventory. Longest-prefix-wins and no catch-all remain. Exclude44 recognized fixture and33 historical paths before parse/registry lookup; preserve the8 existing fixture exclusions within that44.10 ordinary experiment/driver inputs plus benchmark1 become eval-only. Reconcile current source-derived totals rather than hardcode old counts into production.
6. Correct `scripts/sbom-survey/README.md` current-checkout discovery and current operation status, its bounded requirements syntax description and the mirrored design table in `dev/design/0.8.20-slice-31-sbom-survey-tool.md`. Historical24-test evidence stays dated, not a claim of today's suite count or new sweep completion. Correct directly invalidated comments only. Survey remains informational and unwired from CI/full gate; preserve its isolated environment and no pytest-settings-table contract.

Before implementation,10 reads existing tests in full and proposes narrow test
additions for each behavior, preserving original assertions and independent
schema validation. The [test mapping](test-mapping.md) specifies intended failure
signals and negative controls; nothing ran here. Extension behavior that needs
new source design beyond this subset returns through10 review.

## Approved contained updates in10

Rustls: adapt `Cargo.lock` from current .29 source with a minimal0.23.46 update
and necessary rustls-webpki0.103.16 closure. Confirm resulting graph/diff; do not
replace the stale candidate lock, downgrade forks/SQLite/E or drop MCP. Retain
ureq2/TLS feature choice, SHA verification, cache/download/retry semantics and
no new networking in ordinary query paths. Selected exact-build feature closure
and developer/artifact security assessment belong to10. Do not add a direct
Rustls pin or broad transport migration unless design/necessity justifies it.

DOMPurify: update only `dev/tools/mermaid/package-lock.json`'s transitive
DOMPurify3.4.11→3.4.17 and required integrity metadata under existing Mermaid
range. Use actual renderer configuration to assess advisory applicability;
wrapper forwards additional mmdc/config arguments, so repository defaults cannot
prove all possible invocations safe. Retain real render/CLI behavior. Browser,
Puppeteer/zod and Mermaid major churn are outside this approved contained group.
No malicious untrusted rendering or browser sandbox reconfiguration in1.

Exact edit sites for other candidates are in their register, but they remain
unselected. Do not import stale complete locks, regenerate goldens/model hashes,
remove exact exceptions, unpause Dependabot or close PRs autonomously.

## Tool/source ownership and downstream fit

The completed Slice0 proposal/review-log at850f2c15d specifies owned command-local Node25.9.0/npm11.12.1, Rust1.95,
Python package>=3.10/developer>=3.11, Ruff/Pyright pins and source-bound
noneditable wheel/native outputs.10 resolves current bootstrap/build editable
invocations before running them in a linked worktree. Survey uses its own
owned environment; do not install into or borrow another checkout's venv.
No heavy activity precedes measured10 capacity/path reservations. Pin updates
and tests must bind source SHA/dirty delta, lock/features, target/interpreter
and artifact digest as applicable. Source metadata is not live tool readiness.

Survey/dependency preparation has no need for CUDA probes, toolkit/driver
changes or fault injection. R580/primary12.6 and no K620 execution remain fixed.
140 owns only required repairs fork-first; any resulting qualified pins feed
affected110/120, then130. Broad rebase/marker/multiarch work is .30. Preserve
SQLite page-cache policy, Windows WAL/path behavior, MCP exact edge isolation,
version axes and driverless mapping across future upgrades. Public contract
changes need their accepted ADR/interface work, not “dependency cleanup.”

## Review and completion controls

PhaseA is full candidate documents in isolated /tmp only. No repository, Git,
ledger, test/source/environment mutation occurs here. Parent-controlled Astra
plan review precedes proposal completion, followed by fresh final-output review.
Parent imports reviewed documents and runs Markdown validation with owned tools.
Path-set/link/stdlib evidence checks do not assert product success or audit
coverage. Later code phases use the full gate once on a stable final tree and
scoped checks while iterating. Missing owned setup is a preflight failure.
