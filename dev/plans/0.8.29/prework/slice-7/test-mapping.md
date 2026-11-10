# Slice 7 proposed verification mapping

No behavioral tests or product gates were run for this documentation proposal.
The complete-read fixtures below were inspected at product6edc1155f; they are
existing source contracts, not new executed passes. Future10 executable changes
need one observed failing RED per behavior before GREEN, visible staged/committed
test and exact failure evidence. Do not edit established tests during fix-to-spec
to erase their intent. Mechanical version metadata retains its exception.

## Direct fixtures read in full

| Requirement / fixture | Existing assertion and practical limit | Proposed delta / downstream acceptance |
| --- | --- | --- |
| P7-R1: scripts/tests/test_dev_environment_tools.sh | Checks required shellcheck/actionlint/Ruff/Pyright and root js-yaml; Gitleaks version is advisory. It does not establish wheel/native identity or Node pin equivalence. | Retain pin failures.10 adds owned setup evidence/control rather than treating this tool inventory as native qualification. |
| P7-R1: scripts/tests/test_bootstrap_heavy.sh | Fake-tool behavioral fixture checks owned venv, exact editable dev-extra invocation, imports, locked root/TS installs, cold/restored setup and installer failure before native build; static CI ownership via js-yaml. | RED new linked-worktree noneditable requirement against current editable behavior; GREEN supported owned route. Preserve developer extras/imports and installed-artifact identity, not merely removal of `-e`. Update this changed contract at RED, never quietly weaken it after implementation. |
| P7-R2: scripts/tests/test_agent_verify_markdown_scope.sh | Missing owned Python/pinned tools fail before gates; default environment→lint→typecheck→security→test; Markdown-only delegation/skip wording/failure and invalid scope/tier errors. | Retain. This is the direct verifier fixture; test_verify_release_gates covers release metadata instead. Additional pip/identity cases belong only to reviewed changed behavior. |
| P7-R2: scripts/tests/test_ci_verify_observability.sh | Real collect-all failure/summary, invocation-owned redacted spill logs, symlink/empty-log refusal, actual missing-venv heavy-preflight diagnostic and exact73 long-output exit. actionlint and declared js-yaml are prerequisites. | Retain real exits/full diagnostics and owned log collection. No empty artifact or swallowed logging failure becomes pass. |
| P7-R3: scripts/tests/test_ci_long_job_efficiency.sh | Checks pip/npm keys, jobs2/debug0/incremental0/no target-cache report, resource snapshots, serial Node files; executes BGE warm success/failure and summary-write failure. | Retain skipped-live warning/state and advisory parallel scope. Source constants are not current capacity/timing measurements. |
| P7-R6: scripts/tests/test_tag_cuda_route.sh | Direct offline normal exact-tag path plus .28 absent-tag bootstrap/held wording; rejects absent marker, mismatch, existing tag,403/network/malformed/successful404-looking responses, missing main workflow/restrictions/name/visibility/tag policy and missing/empty/malformed reviewers. | RED exactv0.8.29 positive under identical strict fixtures fails current guard for hardcoded .28. GREEN narrow amendment retains .28 and every negative; add unapproved future marker/tag rejection and post-create marker-free .29 normal success/missing exact ref refusal. |
| P7-R4/R6: scripts/tests/test_cuda_release_contract.sh | Static/mutation contracts for protected main-owned control plane, selected CUDA jobs/features/toolkits, artifact-only uploads/no host npm-cache archive, driverless proofs, source/network isolation and Tegra boundaries. | Retain unchanged. This is not the direct bootstrap behavior fixture or live route/driver/package proof. Any affected workflow change still needs actionlint. |
| P7-R4/R6: scripts/tests/test_release_workflow_scope.sh | Static target matrix, candidate/control-plane checkout exceptions, tier aggregate/order, polling, npm next→latest dependencies, bounded upload wiring and main-only three-minute identity probe. It uses PyYAML for structure; this does not replace actionlint workflow validation. | Retain. Static workflow shape does not establish delivery, package execution, OIDC exchange or registry consumer success. |
| P7-R4: scripts/tests/test_verify_release_gates.sh | Version/changelog/metadata, main-reach, immutable dry-run candidate, typed publish confirmation and canonical tag-checkout cases. Fixture snapshots/mutates version surfaces and creates temporary tags in checkout. | Retain only in future isolated execution; do not run against read-only/shared planning checkout. Not evidence for agent-verify ordering. |
| P7-R5: scripts/tests/test_gitleaks_guards.sh | Real scanner fixtures reject synthetic staged/current credentials and missing scanner/log enumeration, validate narrow policy mutations/redaction, retain always-on advisory CI and separate dispatch/release history advisory. | Retain policies; no new suppression or scanner changes selected. This fixture scans current repository when executed;7 did not run it. |
| P7-R5: scripts/tests/test_gitleaks_history_environment.sh | Real temporary-history scan ignores inherited config/TOML suppression and redacts constructed fixture value. | Retain; requires real pinned scanner. Nonzero fixture is expected detection, not a current repository leak. |
| P7-R5: scripts/tests/test_gitleaks_history_classifier.py | Exact100-record/51-fingerprint multiset fixture; unknown/missing/extra/malformed/inconsistent reports fail; mutations protect current prerequisite, isolated mirror, empty ignore/config neutralization/no broad bypass. | Retain. Fixture count is classifier policy, not current history scan count or proof all observed findings are credentials. |

## Prospective RED/GREEN controls for10

These are specifications, not observed failures or written tests:

1. Owned linked-checkout setup: controlled entry point may not call editable
   install, manual copy, system Python fallback or another checkout's environment.
   Current editable path supplies the expected RED. GREEN produces and installs
   the owned noneditable wheel with required developer tools; test path with spaces,
   correct import/native origin and exact source/features/ABI/digest receipt.
2. Native reuse: seeded timestamp sentinel plus mismatched source/features/native
   receipt must fail or rebuild through the supported owned path, never skip as
   ready. Match permits bounded reuse. Missing/wrong pins and installer failure
   stop before native workload; cold and restored dependencies preserve identity.
3. Exact .29 route: direct fixture above adds real expected positive RED before
   guard change. Retain negative controls with exact .29 responses, preserve old
   normal routing, and assert held wording differs from ordinary readiness.
   Post-push peeled-SHA/live policy checks remain an authorized execution duty;
   offline fixture alone cannot establish them.

10 chooses the smallest entry-point implementation after independent design
review; no unselected global bootstrap refactor follows from these controls.

## Catalogued downstream checks and unmeasured execution

Located controls below require complete reread by their executing owner before
claiming detailed assertions; they were not run or fully audited by7:
`test_set_version.sh`, `test_release_version_surfaces.sh`,
`test_runtime_release_alignment.sh`, `test_verify_embedder_api_no_drift.sh`,
`test_cargo_publish_if_new.sh`, `test_wait_for_crate_version.sh`,
`test_verify_release_python_wheel.sh`, `test_linux_aarch64_release_artifacts.sh`
and `test_ci_proportional_routing.py` under scripts/tests. Release/registry smoke
scripts and actual native receipt collectors stay with160. Canonical AC-054's
pending finalizer fixture cannot be counted as covered by this catalog.

Capacity/latency evidence is future execution metadata: exact phase/path,
before/peak/after bytes and memory, numeric reservation/stop threshold, Cargo jobs,
duration, source/config/artifact/input identity and real result. Current capacity,
corpus/model accessibility, external API privilege and runner status are unknown.
Retain existing preflight10GiB floor and one native build/gate initial concurrency;
neither supplies a measured phase budget or speedup guarantee.

Candidate document checks cover five complete files, P7-A1..A6, intended links,
whitespace and correct authority/provenance. Parent later runs scoped Markdown/
planning validators; missing owned linter is a failure, external version-mismatched
lint is limited. Future source/script phase runs required final owned full verifier;
heavy alone is partial. Preserve strict ptrace/capable-executor handling, full
workspace clippy/check and actual skipped/failed/assembled history. No broad build,
full-history scan, GPU work or publication is needed for this proposal review.
